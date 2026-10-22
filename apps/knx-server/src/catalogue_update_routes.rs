//! Previewed, backed-up catalogue updates and restores for the web (ADR-0118).
//!
//! The server keeps no upload between preview and apply: apply receives the
//! file again with the preview's token, and `apply_package_update` re-runs the
//! preview and refuses a mismatch. Backups are addressed by name only, taken
//! from a fresh listing of the database directory; no client path is opened.
//! Deleting one (Goal B5) also needs the opaque identity that listing showed,
//! so a file replaced under the same name since then is kept.

use crate::{
    errors::ApiError,
    routes::{package_refusal, CatalogInstallReportDto, CatalogItemDto},
    SharedState,
};
use axum::{
    extract::{DefaultBodyLimit, Multipart, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use knx_productdb::managed_catalogue::{LegacyMutationKind, LegacySettlementHint};
use knx_productdb::{
    CleanupError, PackageError, PackageLimits, ProductDbError, UpdateError, UpdatePreview,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};

/// Same body limit as `/api/catalog/install`: the product database's package
/// input bound.
const MAX_PACKAGE_BYTES: usize = knx_productdb::MAX_PACKAGE_INPUT_BYTES as usize;
const BACKUP_MARKER: &str = ".backup-";
const BACKUP_SUFFIX: &str = ".sqlite";
const SHORT_HASH_LEN: usize = 12;

pub(crate) fn router() -> Router<SharedState> {
    Router::new()
        .route(
            "/api/catalog/update/preview",
            post(preview).layer(DefaultBodyLimit::max(MAX_PACKAGE_BYTES)),
        )
        .route(
            "/api/catalog/update/apply",
            post(apply).layer(DefaultBodyLimit::max(MAX_PACKAGE_BYTES)),
        )
        .route("/api/catalog/backups", get(backups))
        .route("/api/catalog/restore", post(restore))
        .route("/api/catalog/backups/delete", post(delete_backup))
        .route("/api/catalog/cleanup/preview", post(cleanup_preview))
        .route("/api/catalog/cleanup/apply", post(cleanup_apply))
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PreviewView {
    applied: bool,
    preview: UpdatePreview,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AppliedView {
    applied: bool,
    preview: UpdatePreview,
    /// File name only; never a path.
    backup: String,
    /// One entry per installed package: the file itself (`path` null) or
    /// every inner package of a vendor wrapper (ADR-0140).
    packages: Vec<AppliedPackageView>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AppliedPackageView {
    /// Member path inside the wrapper; null for a plain package.
    path: Option<String>,
    report: CatalogInstallReportDto,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct BackupEntry {
    name: String,
    bytes: u64,
    modified_unix_seconds: u64,
    /// Opaque: which file this name was when listed (device, inode, size,
    /// modification time). A deletion must present it unchanged.
    identity: String,
}

#[derive(Serialize)]
struct BackupList {
    backups: Vec<BackupEntry>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RestoreRequest {
    name: String,
}

#[derive(Serialize)]
struct Restored {
    restored: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DeleteBackupRequest {
    name: String,
    identity: String,
    #[serde(default)]
    confirmed: bool,
}

#[derive(Serialize)]
struct Deleted {
    deleted: String,
}

struct Upload {
    name: String,
    bytes: Vec<u8>,
    token: Option<String>,
}

fn multipart_error(error: axum::extract::multipart::MultipartError) -> ApiError {
    ApiError::with_status(error.status(), error.body_text())
}

/// The first file field and an optional `token` text field, in any order.
async fn read_upload(mut multipart: Multipart) -> Result<Upload, ApiError> {
    let mut file = None;
    let mut token = None;
    while let Some(field) = multipart.next_field().await.map_err(multipart_error)? {
        if let Some(name) = field.file_name().map(str::to_owned) {
            if file.is_none() {
                file = Some((name, field.bytes().await.map_err(multipart_error)?.to_vec()));
            }
        } else if field.name() == Some("token") {
            token = Some(field.text().await.map_err(multipart_error)?);
        }
    }
    let (name, bytes) =
        file.ok_or_else(|| ApiError::bad_request("no file field in catalogue update"))?;
    Ok(Upload { name, bytes, token })
}

/// The owner of the shared product catalogue, or a 503 refusal.
fn owner(state: &SharedState) -> Result<&crate::CatalogueOwner, ApiError> {
    state.product_db.as_ref().ok_or_else(|| {
        ApiError::with_status(StatusCode::SERVICE_UNAVAILABLE, state.product_db_missing())
    })
}

/// The database file path of a managed connection.
fn db_path(conn: &knx_productdb::Connection) -> Result<PathBuf, ApiError> {
    conn.path()
        .filter(|path| !path.is_empty())
        .map(PathBuf::from)
        .ok_or_else(|| ApiError::internal("the product database has no file"))
}

/// The settlement hint for a path-based update: only refusals proven to
/// precede any live open (semantic package, stale token, invalid backup)
/// settle as no-commit; database/io outcomes are uncertain and quarantine.
fn update_settlement<T>(result: &Result<T, UpdateError>) -> LegacySettlementHint {
    match result {
        Ok(_) => LegacySettlementHint::SuccessfulReturn,
        Err(UpdateError::Package(PackageError::Database(_))) => LegacySettlementHint::Uncertain,
        Err(UpdateError::Package(_)) => LegacySettlementHint::PreCommitSemanticRefusal,
        Err(UpdateError::StaleToken | UpdateError::InvalidBackup(_)) => {
            LegacySettlementHint::PreCommitSemanticRefusal
        }
        Err(UpdateError::Database(_) | UpdateError::Io(_)) => LegacySettlementHint::Uncertain,
    }
}

/// The settlement hint for a placement cleanup: selection/confirmation/token
/// refusals precede any live mutation; backup/database/not-file-backed
/// outcomes are uncertain.
fn cleanup_settlement<T>(result: &Result<T, CleanupError>) -> LegacySettlementHint {
    match result {
        Ok(_) => LegacySettlementHint::SuccessfulReturn,
        Err(
            CleanupError::InvalidSelection(_)
            | CleanupError::ConfirmationRequired
            | CleanupError::StaleToken,
        ) => LegacySettlementHint::PreCommitSemanticRefusal,
        Err(CleanupError::Backup(_) | CleanupError::Database(_) | CleanupError::NotFileBacked) => {
            LegacySettlementHint::Uncertain
        }
    }
}

fn update_error(error: UpdateError) -> ApiError {
    match error {
        UpdateError::Package(
            error @ PackageError::Database(
                ProductDbError::Sqlite(_) | ProductDbError::FutureVersion { .. },
            ),
        ) => ApiError::internal(error.to_string()),
        UpdateError::Package(error) => package_refusal(error),
        error @ UpdateError::StaleToken => {
            ApiError::conflict("catalogueUpdateStale", error.to_string())
        }
        error @ UpdateError::InvalidBackup(_) => {
            ApiError::refused("invalidBackup", error.to_string())
        }
        error @ (UpdateError::Database(_) | UpdateError::Io(_)) => {
            ApiError::internal(error.to_string())
        }
    }
}

async fn preview(
    State(state): State<SharedState>,
    multipart: Multipart,
) -> Result<Json<PreviewView>, ApiError> {
    let upload = read_upload(multipart).await?;
    let owner = owner(&state)?;
    let preview = owner
        .with_read(|conn| {
            let path = db_path(conn)?;
            knx_productdb::preview_package_update(
                &path,
                &upload.name,
                &upload.bytes,
                PackageLimits::STANDARD,
            )
            .map_err(update_error)
        })
        .map_err(crate::catalogue_owner::owner_error)??;
    Ok(Json(PreviewView {
        applied: false,
        preview,
    }))
}

async fn apply(
    State(state): State<SharedState>,
    multipart: Multipart,
) -> Result<Json<AppliedView>, ApiError> {
    let upload = read_upload(multipart).await?;
    let token = upload
        .token
        .filter(|token| !token.is_empty())
        .ok_or_else(|| ApiError::bad_request("a catalogue update needs the preview's token"))?;
    // Held for the whole update: no install or read on the shared
    // connection runs while the file is backed up and changed.
    let owner = owner(&state)?;
    let path = owner
        .with_read(db_path)
        .map_err(crate::catalogue_owner::owner_error)??;
    let applied = owner
        .with_legacy_mutation(
            LegacyMutationKind::PathUpdate,
            |_conn| {
                knx_productdb::apply_package_update(
                    &path,
                    &upload.name,
                    &upload.bytes,
                    PackageLimits::STANDARD,
                    &token,
                )
            },
            update_settlement,
        )
        .map_err(crate::catalogue_owner::owner_error)?
        .map_err(update_error)?;
    let backup =
        file_name(&applied.backup).ok_or_else(|| ApiError::internal("backup has no file name"))?;
    Ok(Json(AppliedView {
        applied: true,
        preview: applied.preview,
        backup,
        packages: applied
            .installed
            .packages
            .into_iter()
            .map(|package| AppliedPackageView {
                path: package.path,
                report: package.report.into(),
            })
            .collect(),
    }))
}

fn file_name(path: &Path) -> Option<String> {
    path.file_name()?.to_str().map(str::to_owned)
}

/// `<seconds>-<12 hex>[-<n>]` between the marker and the suffix: the names
/// `apply_package_update` writes. Returns the sort key (seconds, n).
fn backup_key(database: &str, name: &str) -> Option<(u64, u64)> {
    let middle = name
        .strip_prefix(database)?
        .strip_prefix(BACKUP_MARKER)?
        .strip_suffix(BACKUP_SUFFIX)?;
    let mut parts = middle.split('-');
    let seconds = digits(parts.next()?)?;
    let hash = parts.next()?;
    if hash.len() != SHORT_HASH_LEN
        || !hash
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return None;
    }
    let attempt = match parts.next() {
        None => 0,
        Some(n) => digits(n)?,
    };
    parts.next().is_none().then_some((seconds, attempt))
}

fn digits(text: &str) -> Option<u64> {
    if text.is_empty() || !text.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    text.parse().ok()
}

/// Which file a listed name is, without exposing device or inode numbers.
///
/// Device and inode alone do not tell a replaced file apart: ext4 hands a
/// deleted file's inode straight to the next file created in the directory,
/// and size and modification time can be copied. The change time cannot be
/// set by a user and moves on every rewrite or metadata change, so it is part
/// of the identity; a backup touched in any way since it was listed is kept.
fn file_identity(metadata: &std::fs::Metadata) -> String {
    let fingerprint = format!(
        "{}:{}:{}:{}.{}:{}.{}",
        metadata.dev(),
        metadata.ino(),
        metadata.len(),
        metadata.mtime(),
        metadata.mtime_nsec(),
        metadata.ctime(),
        metadata.ctime_nsec()
    );
    let digest = Sha256::digest(fingerprint.as_bytes());
    digest[..16]
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

/// Regular files (not links) next to the database with backup names, newest first.
fn list_backups(path: &Path) -> Result<Vec<BackupEntry>, ApiError> {
    let database = file_name(path)
        .ok_or_else(|| ApiError::internal("the product database has no file name"))?;
    let directory = path
        .parent()
        .ok_or_else(|| ApiError::internal("the product database has no directory"))?;
    let entries = std::fs::read_dir(directory).map_err(|e| ApiError::internal(e.to_string()))?;
    let mut found = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|e| ApiError::internal(e.to_string()))?;
        let Some(name) = entry.file_name().to_str().map(str::to_owned) else {
            continue;
        };
        let Some(key) = backup_key(&database, &name) else {
            continue;
        };
        let metadata = entry
            .metadata()
            .map_err(|e| ApiError::internal(e.to_string()))?;
        if !metadata.file_type().is_file() {
            continue;
        }
        let modified_unix_seconds = metadata
            .modified()
            .ok()
            .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
            .map_or(0, |elapsed| elapsed.as_secs());
        found.push((
            key,
            BackupEntry {
                name,
                bytes: metadata.len(),
                modified_unix_seconds,
                identity: file_identity(&metadata),
            },
        ));
    }
    found.sort_by(|(a, x), (b, y)| b.cmp(a).then_with(|| y.name.cmp(&x.name)));
    Ok(found.into_iter().map(|(_, entry)| entry).collect())
}

async fn backups(State(state): State<SharedState>) -> Result<Json<BackupList>, ApiError> {
    let owner = owner(&state)?;
    let path = owner
        .with_read(db_path)
        .map_err(crate::catalogue_owner::owner_error)??;
    Ok(Json(BackupList {
        backups: list_backups(&path)?,
    }))
}

async fn restore(
    State(state): State<SharedState>,
    Json(request): Json<RestoreRequest>,
) -> Result<Json<Restored>, ApiError> {
    let owner = owner(&state)?;
    let path = owner
        .with_read(db_path)
        .map_err(crate::catalogue_owner::owner_error)??;
    // Only a name from a fresh listing: never a client-chosen path.
    let listed = list_backups(&path)?
        .into_iter()
        .any(|entry| entry.name == request.name);
    if !listed {
        return Err(ApiError::with_kind(
            StatusCode::BAD_REQUEST,
            "unknownBackup",
            "not a product database backup listed by this server",
        ));
    }
    let backup = path.with_file_name(&request.name);
    owner
        .restore_legacy(&path, &backup)
        .map_err(crate::catalogue_owner::restore_error)?;
    Ok(Json(Restored {
        restored: request.name,
    }))
}

fn backup_changed() -> ApiError {
    ApiError::conflict(
        "catalogueBackupChanged",
        "this backup changed since it was listed; list the backups again",
    )
}

/// Deletes one backup the user confirmed, exactly the file that was listed.
async fn delete_backup(
    State(state): State<SharedState>,
    Json(request): Json<DeleteBackupRequest>,
) -> Result<Json<Deleted>, ApiError> {
    let owner = owner(&state)?;
    let path = owner
        .with_read(db_path)
        .map_err(crate::catalogue_owner::owner_error)??;
    if !request.confirmed {
        return Err(ApiError::with_kind(
            StatusCode::BAD_REQUEST,
            "confirmationRequired",
            "deleting a catalogue backup needs an explicit confirmation",
        ));
    }
    // Only a name from a fresh listing, which holds regular files only.
    let entry = list_backups(&path)?
        .into_iter()
        .find(|entry| entry.name == request.name)
        .ok_or_else(|| {
            ApiError::with_kind(
                StatusCode::BAD_REQUEST,
                "unknownBackup",
                "not a product database backup listed by this server",
            )
        })?;
    if entry.identity != request.identity {
        return Err(backup_changed());
    }
    let backup = path.with_file_name(&entry.name);
    // Serialised with apply and restore, which write and read backups.
    owner
        .with_legacy_mutation(
            LegacyMutationKind::BackupDelete,
            |_conn| {
                // Checked again right before removal, without following a link.
                let still = std::fs::symlink_metadata(&backup).map_err(|_| backup_changed())?;
                if !still.file_type().is_file() || file_identity(&still) != request.identity {
                    return Err(backup_changed());
                }
                std::fs::remove_file(&backup).map_err(|error| {
                    ApiError::internal(format!("the backup could not be deleted: {error}"))
                })?;
                Ok::<_, ApiError>(())
            },
            |result| match result {
                Ok(_) => LegacySettlementHint::SuccessfulReturn,
                // File removal never mutates the live catalogue: any failure
                // leaves W unchanged and healthy.
                Err(_) => LegacySettlementHint::PreCommitSemanticRefusal,
            },
        )
        .map_err(crate::catalogue_owner::owner_error)??;
    Ok(Json(Deleted {
        deleted: entry.name,
    }))
}

// --- Catalogue placement cleanup ---------------------------------------------
//
// Removes only selected `catalog_item` rows after a preview whose token binds
// the selection and the catalogue state; canonical products, programs and
// sources stay, so existing devices keep resolving. See the contract
// `docs/contracts/catalogue-update.md` (Placement cleanup).

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct CleanupPreviewRequest {
    item_ids: Vec<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct CleanupApplyRequest {
    item_ids: Vec<String>,
    token: String,
    #[serde(default)]
    confirmed: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CleanupImpact {
    /// No placement of this exact product + application pair is left.
    last_placement_for_binding: bool,
    /// Devices of the open project bound to this exact pair; saved projects
    /// that are not open are not inspected.
    open_project_devices: usize,
}

#[derive(Serialize)]
struct CleanupItemView {
    #[serde(flatten)]
    item: CatalogItemDto,
    impact: CleanupImpact,
}

#[derive(Serialize)]
struct CleanupPreviewView {
    token: String,
    items: Vec<CleanupItemView>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CleanupAppliedView {
    removed_item_ids: Vec<String>,
    /// File name only; never a path.
    backup: String,
}

fn cleanup_error(error: CleanupError) -> ApiError {
    match error {
        error @ CleanupError::InvalidSelection(_) => ApiError::with_kind(
            StatusCode::BAD_REQUEST,
            "invalidCatalogueSelection",
            error.to_string(),
        ),
        error @ CleanupError::ConfirmationRequired => ApiError::with_kind(
            StatusCode::BAD_REQUEST,
            "confirmationRequired",
            error.to_string(),
        ),
        error @ CleanupError::StaleToken => {
            ApiError::conflict("catalogueCleanupStale", error.to_string())
        }
        // The cause names local paths; it goes to the server log, not the client.
        CleanupError::Backup(error) => {
            eprintln!("knx-server: catalogue cleanup backup failed: {error}");
            ApiError::internal("the catalogue backup could not be written; nothing was removed")
        }
        error @ (CleanupError::Database(_) | CleanupError::NotFileBacked) => {
            ApiError::internal(error.to_string())
        }
    }
}

/// Cleanup reads the whole catalogue for its token: off the async workers,
/// like other heavy product-database work.
async fn blocking<T: Send + 'static>(
    work: impl FnOnce() -> Result<T, ApiError> + Send + 'static,
) -> Result<T, ApiError> {
    tokio::task::spawn_blocking(work)
        .await
        .map_err(|_| ApiError::internal("catalogue cleanup worker failed"))?
}

/// Devices per exact `(product_ref, program_ref)` pair in the open project.
/// The project lock is released before the product database is locked; the
/// two are never held together.
fn open_project_bindings(state: &SharedState) -> HashMap<(String, String), usize> {
    let project = state.project.lock().expect("state mutex poisoned");
    let mut counts = HashMap::new();
    if let Some(project) = project.as_ref() {
        for device in project.devices.iter() {
            *counts
                .entry((device.product_ref.clone(), device.program_ref.clone()))
                .or_default() += 1;
        }
    }
    counts
}

async fn cleanup_preview(
    State(state): State<SharedState>,
    Json(request): Json<CleanupPreviewRequest>,
) -> Result<Json<CleanupPreviewView>, ApiError> {
    let bindings = open_project_bindings(&state);
    let preview = blocking(move || {
        let owner = owner(&state)?;
        owner
            .with_read(|products| {
                knx_productdb::preview_catalog_cleanup(products, &request.item_ids)
                    .map_err(cleanup_error)
            })
            .map_err(crate::catalogue_owner::owner_error)?
    })
    .await?;
    let items = preview
        .items
        .into_iter()
        .map(|item| {
            let open_project_devices = match (&item.product_ref_id, &item.hardware2program_ref_id) {
                (Some(product), Some(binding)) => bindings
                    .get(&(product.clone(), binding.clone()))
                    .copied()
                    .unwrap_or(0),
                _ => 0,
            };
            let last_placement_for_binding = preview.bindings_without_placement.contains(&item.id);
            CleanupItemView {
                item: item.into(),
                impact: CleanupImpact {
                    last_placement_for_binding,
                    open_project_devices,
                },
            }
        })
        .collect();
    Ok(Json(CleanupPreviewView {
        token: preview.token,
        items,
    }))
}

async fn cleanup_apply(
    State(state): State<SharedState>,
    Json(request): Json<CleanupApplyRequest>,
) -> Result<Json<CleanupAppliedView>, ApiError> {
    // Held for the whole removal, like a package update: the backup and the
    // deletion see the same catalogue as the token check.
    let applied = blocking(move || {
        let owner = owner(&state)?;
        owner
            .with_legacy_mutation(
                LegacyMutationKind::PlacementCleanup,
                |conn| {
                    knx_productdb::apply_catalog_cleanup(
                        conn,
                        &request.item_ids,
                        &request.token,
                        request.confirmed,
                    )
                },
                cleanup_settlement,
            )
            .map_err(crate::catalogue_owner::owner_error)?
            .map_err(cleanup_error)
    })
    .await?;
    let backup =
        file_name(&applied.backup).ok_or_else(|| ApiError::internal("backup has no file name"))?;
    Ok(Json(CleanupAppliedView {
        removed_item_ids: applied.removed_item_ids,
        backup,
    }))
}
