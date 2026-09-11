use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::json;

/// Every function in `domain.rs` returns `Result<T, String>` (or
/// `Result<T, knx_app::AppError>` for the `_impl` variants, converted at
/// the route boundary) — there is no error taxonomy below this layer to
/// switch on. The 400/500 split here is drawn by *which* operation
/// failed, not by inspecting the message: `open_project`/
/// `open_native_project`/`save_project`/`save_project_as` touch the
/// filesystem and `knx-store`, so their failures (missing file, corrupt
/// database, disk full) are environment problems -> `internal` (500).
/// Every other route only ever touches already-loaded in-memory state, so
/// its failures (malformed input, stale id, "no project open") are always
/// the caller's to fix -> `bad_request` (400).
///
/// `diff_project` (`routes.rs`) is the one deliberate exception to the
/// first half of that rule: it also calls into `knx-store` (to load the
/// comparison file), yet still maps its whole result to `bad_request`.
/// Both of its failure modes — "no project open" and "comparison file
/// does not exist" — are the caller's to fix relative to a project that
/// may already be open, not an environment problem, so they follow the
/// second half of the split instead (see the route's own doc comment for
/// the full reasoning).
#[derive(Debug)]
pub struct ApiError {
    status: StatusCode,
    message: String,
}

impl ApiError {
    pub fn bad_request(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            message: message.into(),
        }
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            message: message.into(),
        }
    }

    /// Escape hatch for the one case where the status is not ours to
    /// choose: an axum extractor that already classified its own failure
    /// (`MultipartError` distinguishes 413 "too large" from 400
    /// "malformed"). Everything else goes through the two constructors
    /// above and the split documented on this type.
    pub fn with_status(status: StatusCode, message: impl Into<String>) -> Self {
        Self {
            status,
            message: message.into(),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.status, Json(json!({ "error": self.message }))).into_response()
    }
}
