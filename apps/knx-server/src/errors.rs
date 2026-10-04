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
/// comparison file), yet still maps its rejections to `bad_request`:
/// "no project open", "comparison file does not exist", an unsupported or
/// mismatched input kind — all the caller's to fix relative to a project
/// that may already be open, not an environment problem, so they follow
/// the second half of the split instead. Its one non-400 failure is a
/// `.knxproj` refused over error-level import diagnostics, a `422` that
/// carries the import report (see the route's own doc comment).
///
/// `bus_routes.rs` (T15 task 3) adds a third category the two constructors
/// above cannot express, so it reaches for [`ApiError::with_status`]
/// instead: `502` ("far-end failure"), for `BusSessionError::Transport` —
/// a real KNXnet/IP gateway (or, in tests, a `FakeConnector`/`FakeTunnel`
/// scripted to fail the same way) refused a connection, timed out, or
/// otherwise misbehaved. This is neither `bad_request` (the caller sent a
/// perfectly reasonable request; nothing about it needs fixing) nor
/// `internal` (this server is not the thing that broke — it only relayed
/// a failure from something on the far side of a socket it does not
/// control), which is exactly what HTTP's own `502 Bad Gateway` names:
/// this server, acting as a gateway to another system, got back a
/// failure from that system. Naming it here keeps this doc comment the
/// one place that answers "why did this route return that status", the
/// same reason the 400/500 split above lives here instead of being
/// rediscovered per-route; it does not weaken or replace that split —
/// `bus_routes.rs` never needs a `500` at all (nothing in it touches the
/// filesystem or `knx-store`), and its 400s/409s (a caller mistake, or a
/// session-state conflict the caller could have avoided) still follow
/// the rule above exactly.
#[derive(Debug)]
pub struct ApiError {
    status: StatusCode,
    message: String,
    validation: Option<ValidationHint>,
    /// A stable machine-readable reason the client acts on (see
    /// [`ApiError::refused`]); `None` for the plain `{ error }` body.
    kind: Option<&'static str>,
}

#[derive(Debug)]
struct ValidationHint {
    kind: &'static str,
    syntax: &'static str,
    example: &'static str,
}

impl ApiError {
    pub fn bad_request(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            message: message.into(),
            validation: None,
            kind: None,
        }
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            message: message.into(),
            validation: None,
            kind: None,
        }
    }

    /// Escape hatch for the cases the two constructors above cannot
    /// express: extractor-specific failures and state conflicts (for
    /// example, a 409 when unsaved project edits would be discarded).
    /// Parser-level 422 refusals use `validation` instead, keeping syntax
    /// hints separate from the legacy error text.
    pub fn with_status(status: StatusCode, message: impl Into<String>) -> Self {
        Self {
            status,
            message: message.into(),
            validation: None,
            kind: None,
        }
    }

    /// A `422` the client answers with a specific action rather than just
    /// showing the text, for example asking for a project password. The
    /// body is `{ error, kind }`; `kind` is a stable camelCase token.
    pub fn refused(kind: &'static str, message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::UNPROCESSABLE_ENTITY,
            message: message.into(),
            validation: None,
            kind: Some(kind),
        }
    }

    /// Parser-level rejection. Preserve the legacy `error` text while
    /// exposing a stable kind and actionable syntax separately.
    pub fn validation(
        kind: &'static str,
        detail: impl Into<String>,
        syntax: &'static str,
        example: &'static str,
    ) -> Self {
        Self {
            status: StatusCode::UNPROCESSABLE_ENTITY,
            message: detail.into(),
            validation: Some(ValidationHint {
                kind,
                syntax,
                example,
            }),
            kind: None,
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        match self.validation {
            Some(hint) => (
                self.status,
                Json(json!({
                    "error": self.message,
                    "detail": self.message,
                    "kind": hint.kind,
                    "syntax": hint.syntax,
                    "example": hint.example,
                })),
            )
                .into_response(),
            None => match self.kind {
                Some(kind) => (
                    self.status,
                    Json(json!({ "error": self.message, "kind": kind })),
                )
                    .into_response(),
                None => (self.status, Json(json!({ "error": self.message }))).into_response(),
            },
        }
    }
}
