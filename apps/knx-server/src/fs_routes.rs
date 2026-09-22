//! `/api/fs/*` and `/api/project/download` — the two file-handling
//! strategies for the web build (server-mount and upload/download; the
//! Tauri build skips these entirely in favor of native OS dialogs, see
//! `apps/knx-web/src/filePicker.ts`).
use std::path::Path;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use axum::body::{Body, Bytes, HttpBody};
use axum::extract::{DefaultBodyLimit, Multipart, Query, State};
use axum::http::{header, HeaderValue, Request};
use axum::response::Response;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use tower::ServiceExt;
use tower_http::services::ServeFile;

use crate::domain;
use crate::errors::ApiError;
use crate::paths::resolve_in_data_dir;
use crate::SharedState;

/// Upload ceiling for `/api/fs/upload`, replacing axum's 2 MB default —
/// which is well under the size of a real ETS export (this repository's
/// own reference `.knxproj` is 1.7 MB, and a whole-building project is a
/// multiple of that). 100 MB is chosen to be comfortably above any
/// `.knxproj`/`.knxdb` observed so far while still bounding how much one
/// request can make the server buffer, since `upload` reads the field
/// fully into memory before writing it.
const MAX_UPLOAD_BYTES: usize = 100 * 1024 * 1024;
const DOWNLOAD_CHUNK_BYTES: usize = 64 * 1024;

pub fn fs_routes() -> Router<SharedState> {
    Router::new()
        .route("/api/fs/list", get(list_dir))
        .route(
            "/api/fs/upload",
            post(upload).layer(DefaultBodyLimit::max(MAX_UPLOAD_BYTES)),
        )
        .route("/api/project/download", get(download))
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

/// Keeps axum's own status choice for a failed multipart read instead of
/// flattening everything to 400: a body that blew `MAX_UPLOAD_BYTES` is a
/// 413 with "Request payload is too large", not a 400 framed as a
/// malformed-encoding problem the client could fix by re-encoding.
fn multipart_error(e: axum::extract::multipart::MultipartError) -> ApiError {
    ApiError::with_status(e.status(), e.body_text())
}

async fn upload(
    State(state): State<SharedState>,
    mut multipart: Multipart,
) -> Result<Json<UploadResponse>, ApiError> {
    let uploads_dir = state.data_dir.join("uploads");
    std::fs::create_dir_all(&uploads_dir).map_err(|e| ApiError::internal(e.to_string()))?;

    while let Some(field) = multipart.next_field().await.map_err(multipart_error)? {
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
        let bytes = field.bytes().await.map_err(multipart_error)?;
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
    let tmp = {
        let project = state.project.lock().expect("state mutex poisoned");
        let project = project
            .as_ref()
            .ok_or_else(|| ApiError::bad_request("no project open"))?;
        let opaque = state.opaque.lock().expect("state mutex poisoned");
        let manufacturer_refs = state
            .manufacturer_refs
            .lock()
            .expect("state mutex poisoned");

        let tmp = tempfile::NamedTempFile::new().map_err(|e| ApiError::internal(e.to_string()))?;
        domain::save_project_as_impl(tmp.path(), project, &opaque, &manufacturer_refs)
            .map_err(ApiError::internal)?;
        tmp.into_temp_path()
    };
    stream_temp_file(tmp, DOWNLOAD_CHUNK_BYTES).await
}

/// Own the path in the body itself: HTTP may discard response extensions
/// before it finishes sending the body. Dropping an interrupted body also
/// closes the file and removes the temporary path.
struct TemporaryFileBody {
    body: Body,
    _temp_path: Arc<tempfile::TempPath>,
}

impl HttpBody for TemporaryFileBody {
    type Data = Bytes;
    type Error = axum::Error;

    fn poll_frame(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<http_body::Frame<Self::Data>, Self::Error>>> {
        Pin::new(&mut self.body).poll_frame(cx)
    }

    fn is_end_stream(&self) -> bool {
        self.body.is_end_stream()
    }

    fn size_hint(&self) -> http_body::SizeHint {
        self.body.size_hint()
    }
}

async fn stream_temp_file(
    tmp: tempfile::TempPath,
    chunk_bytes: usize,
) -> Result<Response, ApiError> {
    let temp_path = Arc::new(tmp);
    let response = ServeFile::new(&*temp_path)
        .with_buf_chunk_size(chunk_bytes)
        .oneshot(Request::new(Body::empty()))
        .await
        .map_err(|e| ApiError::internal(e.to_string()))?;
    let mut response = response.map(|body| {
        Body::new(TemporaryFileBody {
            body: Body::new(body),
            _temp_path: temp_path,
        })
    });
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/octet-stream"),
    );
    response.headers_mut().insert(
        header::CONTENT_DISPOSITION,
        HeaderValue::from_static("attachment; filename=\"project.knxdb\""),
    );
    Ok(response)
}

#[cfg(test)]
mod tests {
    use super::*;
    use http_body_util::BodyExt;

    #[tokio::test]
    async fn temporary_download_streams_bounded_frames_and_cleans_up_after_body_drop() {
        const CHUNK_BYTES: usize = 256;
        let expected: Vec<u8> = (0..CHUNK_BYTES * 3 + 17).map(|n| n as u8).collect();
        let tmp = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(tmp.path(), &expected).unwrap();
        let path = tmp.path().to_path_buf();

        let response = stream_temp_file(tmp.into_temp_path(), CHUNK_BYTES)
            .await
            .unwrap();
        let mut body = response.into_body();

        let mut actual = Vec::new();
        let mut data_frames = 0;
        while let Some(frame) = body.frame().await {
            if let Ok(data) = frame.unwrap().into_data() {
                if !data.is_empty() {
                    data_frames += 1;
                    assert!(
                        data.len() <= CHUNK_BYTES,
                        "oversized body frame: {}",
                        data.len()
                    );
                    actual.extend_from_slice(&data);
                }
            }
        }
        assert!(data_frames >= 2, "a whole-file buffer is not streaming");
        assert_eq!(actual, expected);
        assert!(path.exists(), "the body must own its temporary path");
        drop(body);
        assert!(
            !path.exists(),
            "completed downloads must remove their temporary file"
        );
    }

    #[tokio::test]
    async fn dropping_an_unconsumed_download_body_removes_its_temporary_file() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(tmp.path(), b"abandoned download").unwrap();
        let path = tmp.path().to_path_buf();
        let response = stream_temp_file(tmp.into_temp_path(), 256).await.unwrap();
        let body = response.into_body();
        assert!(
            path.exists(),
            "response extraction must not remove the temporary file"
        );
        drop(body);
        assert!(
            !path.exists(),
            "abandoned downloads must remove their temporary file"
        );
    }
}
