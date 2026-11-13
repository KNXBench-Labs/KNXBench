//! The device editor's reads and edits wait for the project off the runtime and past a Save (#36).

use std::sync::{mpsc, Arc};
use std::time::Duration;

use axum::http::StatusCode;
use knx_core::{DeviceId, DeviceInstance, SourceRef};
use tower::ServiceExt;

use crate::domain::{self, DURING_SAVE_WRITE};
use crate::AppState;

/// Generous: a passing run answers in milliseconds, a loaded gate host is slow.
const ANSWER: Duration = Duration::from_secs(10);

/// A new project with device 1 in it, as the editor would show it.
pub(super) fn project_with_device(state: &AppState) {
    domain::new_project_impl(state, None, None, None, None, true).unwrap();
    let mut project = state.project.lock().unwrap();
    let project = project.as_mut().unwrap();
    let id = project.ids.next_device_id().unwrap();
    assert_eq!(id, DeviceId(1));
    project.devices.insert(DeviceInstance {
        id,
        source: SourceRef {
            // Issued here, so its instructions are native (no imported original).
            path: "KB-DEV-1".into(),
            ets_id: "KB-DEV-1".into(),
        },
        name: "Edited".into(),
        description: None,
        address: None,
        product_ref: String::new(),
        program_ref: String::new(),
        commissioning: Default::default(),
        visibility_calculated: true,
        com_objects: vec![],
        binary_data: vec![],
    });
    project.installations[0].topology.unassigned.push(id);
}

fn router(state: &Arc<AppState>) -> axum::Router {
    super::project_routes()
        .merge(crate::note_routes::router(Arc::clone(state)))
        .merge(crate::instruction_routes::router(Arc::clone(state)))
        .with_state(Arc::clone(state))
}

pub(super) fn request(
    method: &str,
    uri: &str,
    body: &str,
) -> axum::http::Request<axum::body::Body> {
    axum::http::Request::builder()
        .method(method)
        .uri(uri)
        .header("content-type", "application/json")
        .body(axum::body::Body::from(body.to_owned()))
        .unwrap()
}

/// The device editor's reads and edits: each meets a held project lock. On
/// this single-threaded runtime one run inline would block the timer.
/// Exact statuses: an extractor refusal answers before the handler takes the
/// lock and would prove nothing. Without a product or communication object
/// the handler refuses under the lock (400) where the table says so.
#[tokio::test(flavor = "current_thread")]
async fn device_editor_routes_leave_the_runtime_free() {
    let routes: [(&str, &str, &str, StatusCode); 15] = [
        ("GET", "/api/project", "", StatusCode::OK),
        ("GET", "/api/project/status-metrics", "", StatusCode::OK),
        ("GET", "/api/project/notes", "", StatusCode::OK),
        ("GET", "/api/devices/1/instructions", "", StatusCode::OK),
        ("GET", "/api/device/1", "", StatusCode::OK),
        ("GET", "/api/device/1/parameters", "", StatusCode::OK),
        (
            "POST",
            "/api/device/1/parameters",
            r#"{"etsId":"P-1","raw":"1"}"#,
            StatusCode::BAD_REQUEST,
        ),
        (
            "POST",
            "/api/individual-address",
            r#"{"deviceId":1,"address":"1.1.7"}"#,
            StatusCode::OK,
        ),
        (
            "POST",
            "/api/device-description",
            r#"{"deviceId":1,"description":"Hall"}"#,
            StatusCode::OK,
        ),
        (
            "PATCH",
            "/api/devices/1/name",
            // Names another server incarnation: refused under the lock.
            r#"{"name":"Hall","expectedName":"Edited","serverIncarnation":"x","snapshotRevision":0,"projectIncarnation":0}"#,
            StatusCode::CONFLICT,
        ),
        (
            "POST",
            "/api/group-address-description",
            r#"{"groupAddressId":1,"description":"x"}"#,
            StatusCode::BAD_REQUEST,
        ),
        (
            "POST",
            "/api/com-object-dpt",
            r#"{"comObjectId":1,"dpt":"DPST-1-1"}"#,
            StatusCode::BAD_REQUEST,
        ),
        (
            "POST",
            "/api/com-object-description",
            r#"{"comObjectId":1,"description":"x"}"#,
            StatusCode::BAD_REQUEST,
        ),
        (
            "POST",
            "/api/com-object-flag",
            r#"{"comObjectId":1,"flag":"Read","value":true}"#,
            StatusCode::BAD_REQUEST,
        ),
        (
            "POST",
            "/api/com-object-priority",
            r#"{"comObjectId":1,"priority":"High"}"#,
            StatusCode::BAD_REQUEST,
        ),
    ];
    assert_routes_leave_the_runtime_free(&routes, router).await;
}

/// Sends each request to a fresh project with device 1 while another thread
/// holds the project lock. A handler that waits inline on this
/// single-threaded runtime keeps the timer from firing, so the holder times
/// out and the route is reported. A route that answers while the lock is
/// still held never reached it, so its row would prove nothing: reported too.
/// Each status is checked after the release.
pub(super) async fn assert_routes_leave_the_runtime_free(
    routes: &[(&str, &str, &str, StatusCode)],
    router: fn(&Arc<AppState>) -> axum::Router,
) {
    assert_routes_leave_the_runtime_free_with(routes, router, || Arc::new(AppState::default()))
        .await;
}

/// As above, with a caller-built state (for example one with products).
pub(super) async fn assert_routes_leave_the_runtime_free_with(
    routes: &[(&str, &str, &str, StatusCode)],
    router: fn(&Arc<AppState>) -> axum::Router,
    new_state: impl Fn() -> Arc<AppState>,
) {
    let mut blocked = Vec::new();
    let mut before_the_lock = Vec::new();
    for &(method, uri, body, expected) in routes {
        let state = new_state();
        project_with_device(&state);
        let router = router(&state);
        let (release, holder) = super::tests::hold_project_lock(&state);
        let request = request(method, uri, body);
        let call = tokio::spawn(async move { router.oneshot(request).await.unwrap().status() });
        tokio::task::yield_now().await;
        tokio::time::sleep(Duration::from_millis(20)).await;
        if call.is_finished() {
            before_the_lock.push(format!("{method} {uri}"));
        }
        let _ = release.send(());
        if !holder.join().unwrap() {
            blocked.push(format!("{method} {uri}"));
        }
        assert_eq!(call.await.unwrap(), expected, "{method} {uri}");
    }
    assert!(
        blocked.is_empty(),
        "these routes blocked the runtime: {blocked:?}"
    );
    assert!(
        before_the_lock.is_empty(),
        "these answered without waiting for the project: {before_the_lock:?}"
    );
}

/// While a Save writes its file (holding the store path, not the project),
/// the tree, the status metrics, notes, instructions and an edit all answer.
/// Each request runs on its own thread and runtime: a regression blocks it
/// until the save is done, which the deadline below reports.
#[test]
fn reads_and_an_edit_answer_while_a_save_writes() {
    let routes: [(&str, &str, &str, StatusCode); 6] = [
        ("GET", "/api/project", "", StatusCode::OK),
        ("GET", "/api/project/status-metrics", "", StatusCode::OK),
        ("GET", "/api/project/notes", "", StatusCode::OK),
        ("GET", "/api/devices/1/instructions", "", StatusCode::OK),
        ("GET", "/api/device/1", "", StatusCode::OK),
        (
            "POST",
            "/api/device-description",
            r#"{"deviceId":1,"description":"During save"}"#,
            StatusCode::OK,
        ),
    ];
    let dir = tempfile::tempdir().unwrap();
    let state = Arc::new(AppState::default());
    project_with_device(&state);
    domain::save_project_as(&state, &dir.path().join("project.knxdb")).unwrap();
    // A user's server journals edits in the background (ADR-0166).
    state.journal().run_in_background();

    let (seen_tx, seen_rx) = mpsc::channel();
    let hook_state = Arc::clone(&state);
    DURING_SAVE_WRITE.with(|hook| {
        *hook.borrow_mut() = Some(Box::new(move || {
            let mut seen = Vec::new();
            for (method, uri, body, expected) in routes {
                let (done_tx, done_rx) = mpsc::channel();
                let router = router(&hook_state);
                let request = request(method, uri, body);
                std::thread::spawn(move || {
                    let runtime = tokio::runtime::Builder::new_current_thread()
                        .enable_all()
                        .build()
                        .unwrap();
                    let status = runtime.block_on(router.oneshot(request)).unwrap().status();
                    let _ = done_tx.send(status);
                });
                seen.push((
                    format!("{method} {uri}"),
                    expected,
                    done_rx.recv_timeout(ANSWER).ok(),
                ));
            }
            seen_tx.send(seen).unwrap();
        }));
    });
    domain::save_project(&state).unwrap();

    let seen = seen_rx.try_recv().expect("the hook ran");
    let waited: Vec<_> = seen
        .iter()
        .filter(|(_, _, status)| status.is_none())
        .map(|(route, _, _)| route.as_str())
        .collect();
    assert!(
        waited.is_empty(),
        "these waited for the save write: {waited:?}"
    );
    for (route, expected, status) in &seen {
        assert_eq!(status.as_ref(), Some(expected), "{route}");
    }
    // The edit made during the write is journaled after it and stays unsaved.
    state.journal().flush();
    let current = domain::current_project_tree(&state).unwrap();
    assert!(current.has_store_path);
    assert!(current.tree.is_modified);
}
