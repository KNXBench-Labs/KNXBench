//! Restores a project's missing manufacturer files from its original `.knxproj` (ADR-0128).
//!
//! `POST /api/project/manufacturer-files/restore` with `{ "path": … }`
//! reads the archive's outer members and installs into the product
//! database exactly those whose SHA-256 the project's manifest records and
//! the database lacks. The project itself, its file and its undo history
//! stay untouched; the next Save stores the restored files in the file.

use std::path::PathBuf;

use crate::{
    errors::ApiError,
    paths::resolve_project_path,
    session_log::{self, LogEntry, Severity},
    SharedState,
};
use axum::{extract::State, routing::post, Json, Router};
use serde::{Deserialize, Serialize};

pub(crate) fn router() -> Router<SharedState> {
    Router::new().route("/api/project/manufacturer-files/restore", post(restore))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct RestoreRequest {
    path: String,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
struct FailedFileDto {
    source_path: String,
    reason: String,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
struct RestoreDto {
    installed: Vec<String>,
    already_installed: usize,
    mismatched: Vec<String>,
    not_in_archive: Vec<String>,
    failed: Vec<FailedFileDto>,
    /// Manifest entries the product database still lacks afterwards.
    still_missing: usize,
}

/// The archive is read whole, like an import; this is its ceiling.
const MAX_ARCHIVE_BYTES: u64 = 1024 * 1024 * 1024;

async fn restore(
    State(state): State<SharedState>,
    Json(body): Json<RestoreRequest>,
) -> Result<Json<RestoreDto>, ApiError> {
    let path = resolve_project_path(&state.data_dir, &body.path)?;
    tokio::task::spawn_blocking(move || restore_blocking(&state, path))
        .await
        .map_err(|e| ApiError::internal(e.to_string()))?
        .map(Json)
}

fn restore_blocking(state: &crate::AppState, path: PathBuf) -> Result<RestoreDto, ApiError> {
    if state
        .project
        .lock()
        .expect("state mutex poisoned")
        .is_none()
    {
        return Err(ApiError::bad_request("no project open"));
    }
    let refs = state
        .manufacturer_refs
        .lock()
        .expect("state mutex poisoned")
        .clone();
    let Some(products) = state.product_db.as_ref() else {
        return Err(ApiError::refused(
            "noProductDatabase",
            match &state.product_db_unavailable {
                Some(reason) => format!("manufacturer files cannot be installed: {reason}"),
                None => "this server has no product database to install manufacturer files into"
                    .to_string(),
            },
        ));
    };
    let len = std::fs::metadata(&path)
        .map_err(|e| ApiError::bad_request(format!("cannot read {}: {e}", path.display())))?
        .len();
    if len > MAX_ARCHIVE_BYTES {
        return Err(ApiError::refused(
            "archiveTooLarge",
            format!("the file has {len} bytes, more than the {MAX_ARCHIVE_BYTES}-byte limit"),
        ));
    }
    let bytes = std::fs::read(&path)
        .map_err(|e| ApiError::bad_request(format!("cannot read {}: {e}", path.display())))?;
    let products = products.lock().expect("state mutex poisoned");
    let report = knx_app::manufacturer_files::restore_from_knxproj(&products, &refs, bytes)
        .map_err(|e| match e {
            knx_app::manufacturer_files::ProductDataError::Archive(e) => {
                ApiError::refused("notAnArchive", e.to_string())
            }
            e => ApiError::internal(e.to_string()),
        })?;
    let still_missing = knx_app::manufacturer_files::missing_from_products(&products, &refs)
        .map_err(|e| ApiError::internal(e.to_string()))?
        .len();
    drop(products);
    log_restore(state, &path, &report, still_missing);
    Ok(RestoreDto {
        installed: report.installed,
        already_installed: report.already_installed,
        mismatched: report.mismatched,
        not_in_archive: report.not_in_archive,
        failed: report
            .failed
            .into_iter()
            .map(|(source_path, reason)| FailedFileDto {
                source_path,
                reason,
            })
            .collect(),
        still_missing,
    })
}

fn log_restore(
    state: &crate::AppState,
    path: &std::path::Path,
    report: &knx_app::manufacturer_files::RestoreReport,
    still_missing: usize,
) {
    let severity = if still_missing == 0 {
        Severity::Info
    } else {
        Severity::Warning
    };
    let mut detail = Vec::new();
    for (label, paths) in [
        ("installed", &report.installed),
        ("different bytes, not installed", &report.mismatched),
        ("not in the archive", &report.not_in_archive),
    ] {
        for source in paths {
            detail.push(format!("{label}: {source}"));
        }
    }
    for (source, reason) in &report.failed {
        detail.push(format!("failed: {source}: {reason}"));
    }
    state
        .session_log
        .lock()
        .expect("state mutex poisoned")
        .push(LogEntry {
            timestamp: session_log::now(),
            severity,
            source: "product data".to_string(),
            message: format!(
                "restored {} manufacturer file(s) from {}; {still_missing} still missing",
                report.installed.len(),
                path.display()
            ),
            location: None,
            diagnostic: None,
            detail: (!detail.is_empty()).then(|| detail.join("\n")),
        });
}
