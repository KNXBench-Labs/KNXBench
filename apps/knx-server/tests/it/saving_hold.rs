//! Holds the store path as a running Save does and checks a route refuses at once (WP-31).
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use serde_json::{json, Value};
use std::sync::{mpsc, Arc};
use std::time::Duration;
use tower::ServiceExt;

/// A thread holding `state.store_path` the way `save_editor_state` does for seconds.
struct SaveInProgress {
    release: Option<mpsc::Sender<()>>,
    thread: Option<std::thread::JoinHandle<()>>,
}

impl SaveInProgress {
    fn start(state: &Arc<knx_server::AppState>) -> Self {
        let state = Arc::clone(state);
        let (ready, acquired) = mpsc::channel();
        let (release, released) = mpsc::channel::<()>();
        let thread = std::thread::spawn(move || {
            let _guard = state.store_path.lock().unwrap();
            ready.send(()).unwrap();
            let _ = released.recv_timeout(Duration::from_secs(30));
        });
        acquired.recv_timeout(Duration::from_secs(5)).unwrap();
        Self {
            release: Some(release),
            thread: Some(thread),
        }
    }
}

impl Drop for SaveInProgress {
    fn drop(&mut self) {
        drop(self.release.take());
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

/// Sends the request while a Save holds the store path. Asserts the answer is `409
/// projectSaving`, comes promptly (a blocking route times out here) and leaves the project lock
/// free. Returns the body.
pub async fn refused_while_saving(
    state: &Arc<knx_server::AppState>,
    app: &axum::Router,
    method: &str,
    path: &str,
    payload: Value,
) -> Value {
    refused_while_saving_with(state, app, method, path, &[], payload).await
}

/// Sends the request while a Save holds the store path and returns its status and body: for a
/// read that must keep answering during the Save. Asserts it comes promptly and the project
/// lock stays free.
pub async fn answered_while_saving(
    state: &Arc<knx_server::AppState>,
    app: &axum::Router,
    method: &str,
    path: &str,
    payload: Value,
) -> (StatusCode, Value) {
    let save = SaveInProgress::start(state);
    let call = app.clone().oneshot(
        Request::builder()
            .method(method)
            .uri(path)
            .header("content-type", "application/json")
            .body(Body::from(payload.to_string()))
            .unwrap(),
    );
    let response = tokio::time::timeout(Duration::from_secs(5), call)
        .await
        .unwrap_or_else(|_| panic!("{method} {path} waited for the running save"))
        .unwrap();
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), 4 * 1024 * 1024)
        .await
        .unwrap();
    assert!(
        state.project.try_lock().is_ok(),
        "{path} kept the project lock during the save"
    );
    drop(save);
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

/// [`refused_while_saving`] for a route that needs extra request headers (a cookie).
pub async fn refused_while_saving_with(
    state: &Arc<knx_server::AppState>,
    app: &axum::Router,
    method: &str,
    path: &str,
    headers: &[(&str, &str)],
    payload: Value,
) -> Value {
    let save = SaveInProgress::start(state);
    let mut request = Request::builder()
        .method(method)
        .uri(path)
        .header("content-type", "application/json");
    for (name, value) in headers {
        request = request.header(*name, *value);
    }
    let call = app
        .clone()
        .oneshot(request.body(Body::from(payload.to_string())).unwrap());
    let response = tokio::time::timeout(Duration::from_secs(5), call)
        .await
        .unwrap_or_else(|_| panic!("{method} {path} waited for the running save"))
        .unwrap();
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), 4 * 1024 * 1024)
        .await
        .unwrap();
    let body: Value = serde_json::from_slice(&bytes).unwrap_or_else(|_| json!({}));
    assert_eq!(status, StatusCode::CONFLICT, "{path}: {body}");
    assert_eq!(body["kind"], "projectSaving", "{path}: {body}");
    assert_eq!(
        body["error"], "The project is being saved; try again in a moment",
        "{path}: {body}"
    );
    // The save is still writing: the refused route must not keep the project locked.
    assert!(
        state.project.try_lock().is_ok(),
        "{path} kept the project lock during the save"
    );
    drop(save);
    body
}
