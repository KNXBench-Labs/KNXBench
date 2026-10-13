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
    routes::{package_refusal, CatalogInstallReportDto},
    SharedState,
};
use axum::{
    extract::{DefaultBodyLimit, Multipart, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use knx_productdb::{PackageError, PackageLimits, ProductDbError, UpdateError, UpdatePreview};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::sync::MutexGuard;

/// Same body limit as `/api/catalog/install`.
const MAX_PACKAGE_BYTES: usize = 256 * 1024 * 1024;
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

type Products<'a> = MutexGuard<'a, knx_productdb::Connection>;

/// The locked shared connection and its database file.
fn products(state: &SharedState) -> Result<(Products<'_>, PathBuf), ApiError> {
    let products = state
        .product_db
        .as_ref()
        .ok_or_else(|| {
            ApiError::with_status(
                StatusCode::SERVICE_UNAVAILABLE,
                "no product database configured",
            )
        })?
        .lock()
        .expect("state mutex poisoned");
    let path = products
        .path()
        .filter(|path| !path.is_empty())
        .map(PathBuf::from)
        .ok_or_else(|| ApiError::internal("the product database has no file"))?;
    Ok((products, path))
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
    let (_products, path) = products(&state)?;
    let preview = knx_productdb::preview_package_update(
        &path,
        &upload.name,
        &upload.bytes,
        PackageLimits::STANDARD,
    )
    .map_err(update_error)?;
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
    let (_products, path) = products(&state)?;
    let applied = knx_productdb::apply_package_update(
        &path,
        &upload.name,
        &upload.bytes,
        PackageLimits::STANDARD,
        &token,
    )
    .map_err(update_error)?;
    let backup =
        file_name(&applied.backup).ok_or_else(|| ApiError::internal("backup has no file name"))?;
    Ok(Json(AppliedView {
        applied: true,
        preview: applied.preview,
        backup,
        report: applied.report.into(),
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
fn file_identity(metadata: &std::fs::Metadata) -> String {
    let fingerprint = format!(
        "{}:{}:{}:{}.{}",
        metadata.dev(),
        metadata.ino(),
        metadata.len(),
        metadata.mtime(),
        metadata.mtime_nsec()
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
    let (_products, path) = products(&state)?;
    Ok(Json(BackupList {
        backups: list_backups(&path)?,
    }))
}

async fn restore(
    State(state): State<SharedState>,
    Json(request): Json<RestoreRequest>,
) -> Result<Json<Restored>, ApiError> {
    let (mut products, path) = products(&state)?;
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
    knx_productdb::restore_product_db_backup(&path, &backup).map_err(update_error)?;
    // A backup may carry an older supported schema: reopen and migrate, and
    // let every later request use the restored catalogue.
    *products = knx_productdb::open_and_migrate(&path).map_err(|e| {
        ApiError::internal(format!(
            "restored, but the product database could not be reopened: {e}"
        ))
    })?;
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
    // Serialised with apply and restore, which write and read backups.
    let (_products, path) = products(&state)?;
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
    // Checked again right before removal, without following a link.
    let still = std::fs::symlink_metadata(&backup).map_err(|_| backup_changed())?;
    if !still.file_type().is_file() || file_identity(&still) != request.identity {
        return Err(backup_changed());
    }
    std::fs::remove_file(&backup)
        .map_err(|error| ApiError::internal(format!("the backup could not be deleted: {error}")))?;
    Ok(Json(Deleted {
        deleted: entry.name,
    }))
}
