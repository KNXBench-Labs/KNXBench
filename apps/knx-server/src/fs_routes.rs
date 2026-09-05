//! `/api/fs/*` and `/api/project/download` — the two file-handling
//! strategies for the web build (server-mount and upload/download; the
//! Tauri build skips these entirely in favor of native OS dialogs, see
//! `apps/knx-web/src/filePicker.ts`).
use std::path::{Path, PathBuf};

use axum::extract::{Multipart, Query, State};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};

use crate::domain;
use crate::errors::ApiError;
use crate::SharedState;

pub fn fs_routes() -> Router<SharedState> {
    Router::new()
        .route("/api/fs/list", get(list_dir))
        .route("/api/fs/upload", post(upload))
        .route("/api/project/download", get(download))
}

/// Resolves `relative` against `data_dir` and confirms the result still
/// lives under it — the only thing standing between `/api/fs/list` and a
/// `path=../../etc` escape out of the mounted volume. `canonicalize`
/// requires the path to exist, which also rejects a nonexistent `path`
/// with a clear "does not exist" instead of a confusing filesystem error
/// later.
fn resolve_in_data_dir(data_dir: &Path, relative: &str) -> Result<PathBuf, ApiError> {
    let candidate = data_dir.join(relative.trim_start_matches('/'));
    let canonical_root = data_dir
        .canonicalize()
        .map_err(|e| ApiError::internal(format!("data dir unreadable: {e}")))?;
    let canonical = candidate
        .canonicalize()
        .map_err(|_| ApiError::bad_request("path does not exist"))?;
    if !canonical.starts_with(&canonical_root) {
        return Err(ApiError::bad_request("path escapes the data directory"));
    }
    Ok(canonical)
}

#[derive(Deserialize)]
struct ListQuery {
    path: Option<String>,
}

#[derive(Serialize)]
struct DirEntryDto {
    name: String,
    is_dir: bool,
}

async fn list_dir(
    State(state): State<SharedState>,
    Query(q): Query<ListQuery>,
) -> Result<Json<Vec<DirEntryDto>>, ApiError> {
    let dir = resolve_in_data_dir(&state.data_dir, q.path.as_deref().unwrap_or(""))?;
    if !dir.is_dir() {
        return Err(ApiError::bad_request("not a directory"));
    }
    let mut entries = Vec::new();
    for entry in std::fs::read_dir(&dir).map_err(|e| ApiError::internal(e.to_string()))? {
        let entry = entry.map_err(|e| ApiError::internal(e.to_string()))?;
        let is_dir = entry
            .file_type()
            .map_err(|e| ApiError::internal(e.to_string()))?
            .is_dir();
        entries.push(DirEntryDto {
            name: entry.file_name().to_string_lossy().into_owned(),
            is_dir,
        });
    }
    entries.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(Json(entries))
}

#[derive(Serialize)]
struct UploadResponse {
    path: String,
}

async fn upload(
    State(state): State<SharedState>,
    mut multipart: Multipart,
) -> Result<Json<UploadResponse>, ApiError> {
    let uploads_dir = state.data_dir.join("uploads");
    std::fs::create_dir_all(&uploads_dir).map_err(|e| ApiError::internal(e.to_string()))?;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| ApiError::bad_request(e.to_string()))?
    {
        let Some(filename) = field.file_name().map(str::to_owned) else {
            continue;
        };
        // `Path::file_name` drops any directory components the client
        // sent (`../../etc/passwd` -> `passwd`) — the only sanitization
        // an upload's filename needs, since it never becomes a directory.
        let safe_name = Path::new(&filename)
            .file_name()
            .ok_or_else(|| ApiError::bad_request("empty filename"))?;
        let dest = uploads_dir.join(safe_name);
        let bytes = field
            .bytes()
            .await
            .map_err(|e| ApiError::bad_request(e.to_string()))?;
        std::fs::write(&dest, &bytes).map_err(|e| ApiError::internal(e.to_string()))?;
        let relative = dest.strip_prefix(&state.data_dir).unwrap_or(&dest);
        return Ok(Json(UploadResponse {
            path: relative.to_string_lossy().into_owned(),
        }));
    }
    Err(ApiError::bad_request("no file field in upload"))
}

/// Always serializes the current in-memory project fresh into a temp
/// `.knxdb` and streams that — regardless of whether it was ever saved to
/// `store_path` before, so "download" always reflects the latest edits.
async fn download(State(state): State<SharedState>) -> Result<Response, ApiError> {
    let project = state.project.lock().expect("state mutex poisoned");
    let project = project
        .as_ref()
        .ok_or_else(|| ApiError::bad_request("no project open"))?;

    let tmp = tempfile::NamedTempFile::new().map_err(|e| ApiError::internal(e.to_string()))?;
    domain::save_project_as_impl(tmp.path(), project).map_err(ApiError::internal)?;
    let bytes = std::fs::read(tmp.path()).map_err(|e| ApiError::internal(e.to_string()))?;

    Ok((
        [
            (axum::http::header::CONTENT_TYPE, "application/octet-stream"),
            (
                axum::http::header::CONTENT_DISPOSITION,
                "attachment; filename=\"project.knxdb\"",
            ),
        ],
        bytes,
    )
        .into_response())
}
