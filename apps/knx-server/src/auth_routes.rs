//! The three unauthenticated endpoints a login screen needs: log in, log out, and ask which.

use std::sync::Arc;

use axum::extract::rejection::JsonRejection;
use axum::extract::State;
use axum::http::header::SET_COOKIE;
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};

use crate::auth::{clear_cookie, session_token, set_cookie, AuthState};
use crate::errors::ApiError;

/// `POST /api/auth/login`'s body. One field, because this server has one
/// password and no accounts — see ADR-0026 and KNOWN_LIMITATIONS.md §63.
#[derive(Deserialize)]
struct LoginRequest {
    password: String,
}

/// What `login` and `logout` answer with: the one fact the frontend acts
/// on. The token itself is never in a body — it is `HttpOnly`, so putting
/// it where a script can read it would undo the point of the cookie.
#[derive(Serialize)]
struct SessionDto {
    authenticated: bool,
}

/// `GET /api/auth/status`'s body. `required` is what decides whether a
/// login screen exists at all: the desktop shell runs with authentication
/// disabled and must never show one.
#[derive(Serialize)]
struct StatusDto {
    required: bool,
    authenticated: bool,
}

/// The routes that must stay reachable without a session, for the obvious
/// reason that they are how a session is obtained.
pub(crate) fn auth_routes() -> Router<Arc<AuthState>> {
    Router::new()
        .route("/api/auth/login", post(login))
        .route("/api/auth/logout", post(logout))
        .route("/api/auth/status", get(status))
}

async fn login(
    State(auth): State<Arc<AuthState>>,
    body: Result<Json<LoginRequest>, JsonRejection>,
) -> Result<Response, ApiError> {
    // Taken by hand instead of letting the extractor reject, so a
    // malformed body answers in the same `{"error": ...}` shape as
    // everything else in `errors.rs` rather than in axum's plain text.
    let Json(request) = body.map_err(|e| ApiError::bad_request(e.body_text()))?;

    let Some(credential) = auth.credential() else {
        // Not 401: nothing was refused. There is no password on this
        // server, so there is nothing to present one to, and a frontend
        // that got here ignored `GET /api/auth/status`.
        return Err(ApiError::bad_request(
            "authentication is not enabled on this server",
        ));
    };

    // One attempt at a time, for the whole of this attempt. Held across
    // both the derivation and the penalty sleep, so parallel guesses queue
    // instead of overlapping: without it the delay would be something each
    // request pays privately and none of them waits for, and a hundred
    // concurrent guesses would put a hundred PBKDF2 derivations on the
    // blocking pool at once. The permit is dropped when this function
    // returns, on every path.
    let _attempt = auth.begin_attempt().await;

    // PBKDF2 at the configured work factor is hundreds of milliseconds of
    // solid CPU. Run on the async worker it would stall every other
    // request on this runtime for that long — and the gate above would not
    // help, because the runtime it starves is not the one it guards.
    let password = request.password;
    let verified = tokio::task::spawn_blocking(move || credential.verify(&password))
        .await
        .map_err(|e| ApiError::internal(format!("password verification failed to run: {e}")))?;

    if !verified {
        let delay = auth.record_failure();
        eprintln!(
            "knx-server: failed login attempt, answering in {} ms",
            delay.as_millis()
        );
        tokio::time::sleep(delay).await;
        return Err(ApiError::with_status(
            StatusCode::UNAUTHORIZED,
            "invalid password",
        ));
    }

    auth.clear_failures();
    let token = auth
        .issue_token()
        .map_err(|e| ApiError::internal(format!("could not start a session: {e}")))?;
    Ok(with_cookie(
        Json(SessionDto {
            authenticated: true,
        }),
        &set_cookie(&token, auth.cookie_secure()),
    ))
}

/// Idempotent by design: logging out without a session, or with one that
/// expired an hour ago, is a success. The clearing cookie goes out either
/// way, so a browser holding a token this server has already forgotten
/// stops sending it.
async fn logout(State(auth): State<Arc<AuthState>>, headers: HeaderMap) -> Response {
    if let Some(token) = session_token(&headers) {
        auth.revoke(&token);
    }
    with_cookie(
        Json(SessionDto {
            authenticated: false,
        }),
        &clear_cookie(auth.cookie_secure()),
    )
}

/// Deliberately does not refresh the session it inspects — see
/// `AuthState::validate`. With authentication off, `authenticated` is
/// `true`: the caller does have full access, and reporting `false` would
/// invite a login screen onto the desktop shell.
async fn status(State(auth): State<Arc<AuthState>>, headers: HeaderMap) -> Json<StatusDto> {
    let required = auth.is_required();
    let authenticated = !required
        || session_token(&headers)
            .is_some_and(|token| auth.validate(&token, /* refresh */ false));
    Json(StatusDto {
        required,
        authenticated,
    })
}

/// Attaches a `Set-Cookie` header to an already-built response.
///
/// The value is assembled by this crate from a base64url token and a fixed
/// list of attributes, so it cannot contain a byte a header value forbids;
/// if that ever stops being true the request fails loudly rather than
/// answering with a session the browser was never told about.
fn with_cookie(body: impl IntoResponse, cookie: &str) -> Response {
    match HeaderValue::from_str(cookie) {
        Ok(value) => {
            let mut response = body.into_response();
            response.headers_mut().insert(SET_COOKIE, value);
            response
        }
        Err(e) => {
            ApiError::internal(format!("could not build the session cookie: {e}")).into_response()
        }
    }
}
