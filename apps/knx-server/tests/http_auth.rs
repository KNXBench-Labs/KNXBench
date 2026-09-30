//! The login, the guard over `/api/`, and the three things deliberately left open.

use std::sync::Arc;
use std::time::Duration;

use axum::body::Body;
use axum::http::header::{COOKIE, SET_COOKIE};
use axum::http::{Request, Response, StatusCode};
use axum::Router;
use tower::ServiceExt;

const PASSWORD: &str = "a perfectly ordinary password";

/// A thousand iterations, not six hundred thousand. Every case below
/// verifies a password at least once, and at the production work factor
/// this file alone would spend a minute proving nothing about work
/// factors — `auth_password.rs` owns that test and pays for it there.
const TEST_ITERATIONS: u32 = 1_000;

fn guarded_config() -> knx_server::AuthConfig {
    let stored = knx_server::hash_password_with_iterations(PASSWORD, TEST_ITERATIONS).unwrap();
    knx_server::AuthConfig::from_password_hash(&stored)
        .unwrap()
        // Nothing here is testing that `sleep` sleeps.
        .with_failure_delay(Duration::ZERO)
}

fn guarded_app() -> Router {
    knx_server::app_with_auth(
        Arc::new(knx_server::AppState::default()),
        None,
        guarded_config(),
    )
}

fn open_app() -> Router {
    knx_server::app(Arc::new(knx_server::AppState::default()), None)
}

async fn send(app: &Router, request: Request<Body>) -> Response<Body> {
    app.clone().oneshot(request).await.unwrap()
}

async fn get(app: &Router, uri: &str, cookie: Option<&str>) -> Response<Body> {
    let mut builder = Request::builder().uri(uri);
    if let Some(cookie) = cookie {
        builder = builder.header(COOKIE, cookie);
    }
    send(app, builder.body(Body::empty()).unwrap()).await
}

async fn post(app: &Router, uri: &str, cookie: Option<&str>) -> Response<Body> {
    let mut builder = Request::builder().method("POST").uri(uri);
    if let Some(cookie) = cookie {
        builder = builder.header(COOKIE, cookie);
    }
    send(app, builder.body(Body::empty()).unwrap()).await
}

async fn login(app: &Router, password: &str) -> Response<Body> {
    send(
        app,
        Request::builder()
            .method("POST")
            .uri("/api/auth/login")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::json!({ "password": password }).to_string(),
            ))
            .unwrap(),
    )
    .await
}

fn set_cookie_header(response: &Response<Body>) -> Option<String> {
    response
        .headers()
        .get(SET_COOKIE)
        .map(|v| v.to_str().unwrap().to_string())
}

/// The `name=value` pair a browser would send back from a `Set-Cookie`.
fn cookie_pair(response: &Response<Body>) -> String {
    let header = set_cookie_header(response).expect("expected a Set-Cookie header");
    header.split(';').next().unwrap().trim().to_string()
}

async fn json_body(response: Response<Body>) -> serde_json::Value {
    let bytes = axum::body::to_bytes(response.into_body(), 1024 * 1024)
        .await
        .unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

/// One representative route per guarded group, so adding a group without
/// guarding it fails here. `/api/version` is in the list on purpose: it is
/// the one route that answers with no project open, and it is still not an
/// exception.
const GUARDED_ROUTES: &[(&str, &str)] = &[
    ("GET", "/api/project"),
    ("GET", "/api/project/load-progress"),
    ("GET", "/api/fs/list"),
    ("GET", "/api/bus/monitor/telegrams"),
    ("GET", "/api/bus/activity"),
    ("POST", "/api/debug-report"),
    ("GET", "/api/version"),
];

async fn call(app: &Router, method: &str, uri: &str, cookie: Option<&str>) -> Response<Body> {
    match method {
        "GET" => get(app, uri, cookie).await,
        "POST" => post(app, uri, cookie).await,
        other => panic!("unexpected method {other}"),
    }
}

#[tokio::test]
async fn the_right_password_sets_a_session_cookie() {
    let app = guarded_app();
    let response = login(&app, PASSWORD).await;
    assert_eq!(response.status(), StatusCode::OK);

    let cookie = set_cookie_header(&response).expect("a successful login must set a cookie");
    assert!(cookie.starts_with(&format!("{}=", knx_server::SESSION_COOKIE)));
    assert!(cookie.contains("; HttpOnly"), "{cookie}");
    assert!(cookie.contains("; SameSite=Strict"), "{cookie}");
    assert!(cookie.contains("; Path=/"), "{cookie}");
    // Plain HTTP on a LAN is the documented deployment; a `Secure` cookie
    // would never be sent back at all there.
    assert!(!cookie.contains("Secure"), "{cookie}");
    // The token must not also come back where a script could read it.
    let body = json_body(response).await;
    assert_eq!(body, serde_json::json!({ "authenticated": true }));
}

#[tokio::test]
async fn the_cookie_is_marked_secure_when_the_deployment_says_so() {
    let app = knx_server::app_with_auth(
        Arc::new(knx_server::AppState::default()),
        None,
        guarded_config().with_cookie_secure(true),
    );
    let cookie = set_cookie_header(&login(&app, PASSWORD).await).unwrap();
    assert!(cookie.contains("; Secure"), "{cookie}");
}

#[tokio::test]
async fn the_wrong_password_is_refused_and_hands_out_nothing() {
    let app = guarded_app();
    let response = login(&app, "not the password").await;
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    assert!(
        set_cookie_header(&response).is_none(),
        "a failed login must not set a cookie"
    );
    let body = json_body(response).await;
    assert_eq!(body["error"], "invalid password");
}

#[tokio::test]
async fn a_malformed_login_body_is_a_bad_request_in_the_usual_shape() {
    let app = guarded_app();
    let response = send(
        &app,
        Request::builder()
            .method("POST")
            .uri("/api/auth/login")
            .header("content-type", "application/json")
            .body(Body::from("{\"password\":"))
            .unwrap(),
    )
    .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body = json_body(response).await;
    assert!(body["error"].is_string(), "got {body}");
}

#[tokio::test]
async fn logging_in_to_a_server_with_no_password_is_a_mistake_not_a_refusal() {
    let app = open_app();
    let response = login(&app, "anything at all").await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert!(set_cookie_header(&response).is_none());
}

#[tokio::test]
async fn every_guarded_group_refuses_a_caller_without_a_session() {
    let app = guarded_app();
    for (method, uri) in GUARDED_ROUTES {
        let response = call(&app, method, uri, None).await;
        assert_eq!(
            response.status(),
            StatusCode::UNAUTHORIZED,
            "{method} {uri} must refuse an unauthenticated caller"
        );
        let body = json_body(response).await;
        assert_eq!(body["error"], "authentication required", "{method} {uri}");
    }
}

#[tokio::test]
async fn a_forged_or_stale_cookie_is_no_better_than_none() {
    let app = guarded_app();
    for cookie in [
        "knx_session=",
        "knx_session=not-a-real-token",
        "theme=dark",
        "knx_session_backup=not-a-real-token",
    ] {
        let response = get(&app, "/api/version", Some(cookie)).await;
        assert_eq!(
            response.status(),
            StatusCode::UNAUTHORIZED,
            "`{cookie}` must not authenticate anyone"
        );
    }
}

#[tokio::test]
async fn the_probe_and_the_login_routes_stay_open() {
    let app = guarded_app();
    assert_eq!(
        get(&app, "/healthz", None).await.status(),
        StatusCode::OK,
        "a liveness probe that needs a password is not a liveness probe"
    );
    assert_eq!(
        get(&app, "/api/auth/status", None).await.status(),
        StatusCode::OK
    );
    assert_eq!(
        post(&app, "/api/auth/logout", None).await.status(),
        StatusCode::OK
    );
    // The login route's own refusal is a 401 about the password, not the
    // guard's 401 about the session — it reached the handler.
    let refusal = json_body(login(&app, "wrong").await).await;
    assert_eq!(refusal["error"], "invalid password");
}

#[tokio::test]
async fn a_session_cookie_reaches_the_handlers_behind_the_guard() {
    let app = guarded_app();
    let cookie = cookie_pair(&login(&app, PASSWORD).await);

    let response = get(&app, "/api/version", Some(&cookie)).await;
    assert_eq!(response.status(), StatusCode::OK);
    let body = json_body(response).await;
    assert_eq!(body["version"], knx_server::version_string());

    for (method, uri) in GUARDED_ROUTES {
        let status = call(&app, method, uri, Some(&cookie)).await.status();
        assert_ne!(
            status,
            StatusCode::UNAUTHORIZED,
            "{method} {uri} must let a logged-in caller through"
        );
    }
}

#[tokio::test]
async fn status_reports_what_the_frontend_needs_to_decide() {
    let app = guarded_app();
    let before = json_body(get(&app, "/api/auth/status", None).await).await;
    assert_eq!(
        before,
        serde_json::json!({ "required": true, "authenticated": false })
    );

    let cookie = cookie_pair(&login(&app, PASSWORD).await);
    let after = json_body(get(&app, "/api/auth/status", Some(&cookie)).await).await;
    assert_eq!(
        after,
        serde_json::json!({ "required": true, "authenticated": true })
    );
}

#[tokio::test]
async fn status_tells_the_desktop_shell_there_is_nothing_to_log_into() {
    let body = json_body(get(&open_app(), "/api/auth/status", None).await).await;
    assert_eq!(
        body,
        serde_json::json!({ "required": false, "authenticated": true })
    );
}

#[tokio::test]
async fn logging_out_invalidates_the_session_and_clears_the_cookie() {
    let app = guarded_app();
    let cookie = cookie_pair(&login(&app, PASSWORD).await);
    assert_eq!(
        get(&app, "/api/version", Some(&cookie)).await.status(),
        StatusCode::OK
    );

    let logout = post(&app, "/api/auth/logout", Some(&cookie)).await;
    assert_eq!(logout.status(), StatusCode::OK);
    let cleared = set_cookie_header(&logout).expect("logout must clear the cookie");
    assert!(cleared.contains("Max-Age=0"), "{cleared}");
    assert!(cleared.starts_with("knx_session=;"), "{cleared}");
    assert_eq!(
        json_body(logout).await,
        serde_json::json!({ "authenticated": false })
    );

    assert_eq!(
        get(&app, "/api/version", Some(&cookie)).await.status(),
        StatusCode::UNAUTHORIZED,
        "the token must stop working the moment it is revoked"
    );
}

#[tokio::test]
async fn an_expired_session_is_rejected() {
    let app = knx_server::app_with_auth(
        Arc::new(knx_server::AppState::default()),
        None,
        guarded_config().with_idle_timeout(Duration::ZERO),
    );
    let response = login(&app, PASSWORD).await;
    assert_eq!(
        response.status(),
        StatusCode::OK,
        "the login still succeeds"
    );
    let cookie = cookie_pair(&response);
    assert_eq!(
        get(&app, "/api/version", Some(&cookie)).await.status(),
        StatusCode::UNAUTHORIZED
    );
    let status = json_body(get(&app, "/api/auth/status", Some(&cookie)).await).await;
    assert_eq!(status["authenticated"], false);
}

#[tokio::test]
async fn an_unknown_api_path_is_still_a_404_not_a_401() {
    // `route_layer` guards matched routes only. A 401 here would be a
    // guard that answers for paths no handler owns, which turns every
    // typo into a security event and tells a caller that something exists.
    let response = get(&guarded_app(), "/api/no-such-route", None).await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn the_unauthenticated_router_behaves_exactly_as_it_did_before() {
    let app = open_app();
    for (method, uri) in GUARDED_ROUTES {
        let status = call(&app, method, uri, None).await.status();
        assert_ne!(
            status,
            StatusCode::UNAUTHORIZED,
            "{method} {uri} must stay open when no password is configured"
        );
    }
    assert_eq!(get(&app, "/healthz", None).await.status(), StatusCode::OK);
    assert_eq!(
        get(&app, "/api/version", None).await.status(),
        StatusCode::OK
    );
}

#[tokio::test]
async fn the_bind_address_follows_the_password_and_nothing_else() {
    // The guard that makes authentication optional at the type level and
    // mandatory in practice, tested without opening a socket.
    assert!(knx_server::bind_address(false).is_loopback());
    assert!(knx_server::bind_address(true).is_unspecified());
}

/// Two wrong passwords at once must cost what two wrong passwords cost in
/// sequence. Fired in parallel against an ungated login they would sleep
/// through the same penalty together — the delay would be a tax each
/// request pays privately, not a limit on how fast guesses can be made —
/// and both PBKDF2 derivations would land on the blocking pool at once.
///
/// Only a lower bound is asserted, because that is the direction the bug
/// lies in. With a 150 ms base delay the penalties are 150 ms and 300 ms:
/// serialised they sum to 450 ms, overlapped they finish in about 300 ms.
#[tokio::test]
async fn concurrent_failed_logins_are_serialised() {
    let delay = Duration::from_millis(150);
    let app = knx_server::app_with_auth(
        Arc::new(knx_server::AppState::default()),
        None,
        guarded_config().with_failure_delay(delay),
    );

    let started = std::time::Instant::now();
    let (first, second) = tokio::join!(
        login(&app, "not the password"),
        login(&app, "also not the password")
    );
    let elapsed = started.elapsed();

    assert_eq!(first.status(), StatusCode::UNAUTHORIZED);
    assert_eq!(second.status(), StatusCode::UNAUTHORIZED);
    assert!(
        elapsed >= delay * 2 + delay / 2,
        "two concurrent failed logins took {elapsed:?}, which is short enough \
         that they overlapped"
    );
}
