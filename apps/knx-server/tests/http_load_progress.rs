//! HTTP tests for `GET /api/project/load-progress` and the operation it reports (ADR-0023).
//!
//! The route is the whole of the progress transport, so these tests pin
//! the four things a browser depends on: nothing loaded yet is `null`, a
//! finished load leaves a `succeeded` snapshot, a failed one leaves a
//! `failed` snapshot *and* the project that was already open, and a second
//! load while one is running is refused with `409` rather than racing.
//!
//! The fixture is `knx_testsupport::write_minimal_knxproj` rather than the
//! reference ETS4 export: the corpus is gitignored, and nothing asserted
//! here is about project size.

use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use serde_json::{json, Value};
use tower::ServiceExt;

fn state() -> Arc<knx_server::AppState> {
    // Progress transport does not need a product database or a real connector.
    // Do not call AppState::new/Default and then override product_db: opening
    // and migrating the default database has already happened by that point.
    Arc::new(knx_server::AppState {
        project: Default::default(),
        clean_project: Default::default(),
        last_saved_at: Default::default(),
        store_path: Default::default(),
        opaque: Default::default(),
        manufacturer_refs: Default::default(),
        command_stack: Default::default(),
        import_counts: Default::default(),
        server_incarnation: "synthetic-progress-test".into(),
        project_revision: Default::default(),
        product_db: None,
        session_log: Default::default(),
        connector: Box::new(knx_server::fake::FakeConnector::discovering(Vec::new())),
        bus_session: Default::default(),
        group_address_style_publication: Default::default(),
        next_bus_session_id: Default::default(),
        line_scan_session: Default::default(),
        next_line_scan_session_id: Default::default(),
        device_download_plan: Default::default(),
        device_download: Default::default(),
        next_device_download_id: Default::default(),
        device_download_timing: Default::default(),
        address_programming: Default::default(),
        next_address_programming_id: Default::default(),
        one_shot_activity: Default::default(),
        address_programming_timing: Default::default(),
        address_programming_pause: Default::default(),
        load_operations: Default::default(),
        data_dir: std::env::temp_dir(),
        settings_lock: Default::default(),
        catalog_requests: Default::default(),
    })
}

#[test]
fn progress_fixture_does_not_open_an_unrelated_default_product_database() {
    let state = state();
    assert!(
        state.product_db.is_none(),
        "progress transport fixtures must not open or migrate the default product database"
    );
}

async fn body_json(response: axum::response::Response) -> Value {
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

fn get(uri: &str) -> Request<Body> {
    Request::builder()
        .method("GET")
        .uri(uri)
        .body(Body::empty())
        .unwrap()
}

fn post(uri: &str, body: Value) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri(uri)
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .unwrap()
}

async fn progress(state: &Arc<knx_server::AppState>) -> Value {
    let response = knx_server::app(state.clone(), None)
        .oneshot(get("/api/project/load-progress"))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    body_json(response).await
}

#[tokio::test]
async fn a_server_that_has_loaded_nothing_reports_null_rather_than_an_idle_operation() {
    let state = state();

    assert_eq!(
        progress(&state).await,
        Value::Null,
        "there is no operation zero: a client must be able to tell \
         'never started' from 'started and finished'"
    );
}

#[tokio::test]
async fn a_finished_import_leaves_a_succeeded_snapshot_naming_its_source() {
    let dir = tempfile::tempdir().unwrap();
    let path = knx_testsupport::write_minimal_knxproj(dir.path());
    let state = state();

    let response = knx_server::app(state.clone(), None)
        .oneshot(post(
            "/api/project/import",
            json!({ "path": path.to_string_lossy() }),
        ))
        .await
        .unwrap();
    let status = response.status();
    assert_eq!(
        status,
        StatusCode::OK,
        "unexpected synthetic import response: {}",
        body_json(response).await
    );

    let snapshot = progress(&state).await;
    assert_eq!(snapshot["operationId"], 1);
    assert_eq!(snapshot["kind"], "import");
    assert_eq!(snapshot["source"], "minimal.knxproj");
    assert_eq!(snapshot["status"], "succeeded");
    assert_eq!(snapshot["error"], Value::Null);
    assert_eq!(
        snapshot["phase"], "buildProjectTree",
        "the last phase announced is the last one the pipeline ran"
    );
}

// Fix round 3, F9: the client token replaces the id-and-source heuristic
// entirely, so its round trip through the actual HTTP body is the one
// thing that must work — everything `ownsOperation` does downstream rests
// on the server echoing back exactly what the client sent.
#[tokio::test]
async fn a_client_token_sent_on_import_is_echoed_in_every_snapshot() {
    let dir = tempfile::tempdir().unwrap();
    let path = knx_testsupport::write_minimal_knxproj(dir.path());
    let state = state();

    let response = knx_server::app(state.clone(), None)
        .oneshot(post(
            "/api/project/import",
            json!({ "path": path.to_string_lossy(), "clientToken": "11111111-1111-1111-1111-111111111111" }),
        ))
        .await
        .unwrap();
    let status = response.status();
    assert_eq!(
        status,
        StatusCode::OK,
        "unexpected synthetic import response: {}",
        body_json(response).await
    );

    let snapshot = progress(&state).await;
    assert_eq!(
        snapshot["clientToken"], "11111111-1111-1111-1111-111111111111",
        "the token this client sent must come back verbatim, not a server-invented id"
    );
}

// Fix round 4, F11: the same round trip for `/api/project/open`, which
// had none. Deleting `clientToken` from the open POST body left `tsc` and
// the whole vitest suite green while the banner went blind for every
// native `.knxdb` — the import path was tested, the open path was
// assumed, and that asymmetry is the exact shape the earlier rounds kept
// walking into. A `.knxdb` is built here through the routes the UI uses
// (new, then save-as) rather than a fixture, so nothing about the file
// format is being asserted by accident.
#[tokio::test]
async fn a_client_token_sent_on_the_native_open_is_echoed_in_every_snapshot() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("villa.knxdb");
    let state = state();

    let created = knx_server::app(state.clone(), None)
        .oneshot(post("/api/project/new", json!({})))
        .await
        .unwrap();
    assert_eq!(created.status(), StatusCode::OK);
    let saved = knx_server::app(state.clone(), None)
        .oneshot(post(
            "/api/project/save-as",
            json!({ "path": db_path.to_string_lossy() }),
        ))
        .await
        .unwrap();
    assert_eq!(saved.status(), StatusCode::OK);

    let response = knx_server::app(state.clone(), None)
        .oneshot(post(
            "/api/project/open",
            json!({ "path": db_path.to_string_lossy(), "clientToken": "22222222-2222-2222-2222-222222222222" }),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let snapshot = progress(&state).await;
    assert_eq!(snapshot["kind"], "open");
    assert_eq!(snapshot["source"], "villa.knxdb");
    assert_eq!(snapshot["status"], "succeeded");
    assert_eq!(
        snapshot["clientToken"], "22222222-2222-2222-2222-222222222222",
        "the open route must carry the token as faithfully as the import \
         route does, or the banner is blind for every .knxdb"
    );
}

// Fix round 4, F13: the failure path's token, at HTTP level. A client
// that lost the POST's response learns its load failed only from the
// snapshot — and only if it can tell the snapshot is *its own*, which is
// the one moment the token is doing real work.
#[tokio::test]
async fn a_failed_load_still_echoes_the_token_that_started_it() {
    let state = state();

    let response = knx_server::app(state.clone(), None)
        .oneshot(post(
            "/api/project/import",
            json!({ "path": "/does/not/exist.knxproj", "clientToken": "33333333-3333-3333-3333-333333333333" }),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);

    let snapshot = progress(&state).await;
    assert_eq!(snapshot["status"], "failed");
    assert_eq!(
        snapshot["clientToken"], "33333333-3333-3333-3333-333333333333",
        "a failed operation keeps its owner: the alternative is a client \
         that cannot claim the error it caused"
    );
}

#[tokio::test]
async fn a_load_started_with_no_client_token_names_its_owner_as_null() {
    let dir = tempfile::tempdir().unwrap();
    let path = knx_testsupport::write_minimal_knxproj(dir.path());
    let state = state();

    let response = knx_server::app(state.clone(), None)
        .oneshot(post(
            "/api/project/import",
            json!({ "path": path.to_string_lossy() }),
        ))
        .await
        .unwrap();
    let status = response.status();
    assert_eq!(
        status,
        StatusCode::OK,
        "unexpected synthetic import response: {}",
        body_json(response).await
    );

    let snapshot = progress(&state).await;
    assert_eq!(
        snapshot["clientToken"],
        Value::Null,
        "an operation nobody sent a token for belongs to nobody, not to a guess"
    );
}

#[tokio::test]
async fn a_failed_import_is_reported_as_failed_and_leaves_the_open_project_alone() {
    let state = state();
    let app = knx_server::app(state.clone(), None);
    let created = app
        .oneshot(post(
            "/api/project/new",
            json!({ "installationName": "Retained Villa" }),
        ))
        .await
        .unwrap();
    assert_eq!(created.status(), StatusCode::OK);

    let response = knx_server::app(state.clone(), None)
        .oneshot(post(
            "/api/project/import",
            json!({ "path": "/does/not/exist.knxproj" }),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    let reported = body_json(response).await["error"]
        .as_str()
        .unwrap()
        .to_string();

    let snapshot = progress(&state).await;
    assert_eq!(snapshot["status"], "failed");
    assert_eq!(
        snapshot["error"].as_str().unwrap(),
        reported,
        "a client that lost the POST's response learns the same thing from \
         the snapshot, which is the only place left to learn it"
    );

    // The project that was open before the failed import is still open,
    // still editable, and still the one the user made.
    let edited = knx_server::app(state.clone(), None)
        .oneshot(post("/api/areas", json!({ "name": "A", "address": 1 })))
        .await
        .unwrap();
    assert_eq!(edited.status(), StatusCode::OK);
    let tree = body_json(edited).await;
    assert_eq!(tree["installations"][0]["name"], "Retained Villa");
}

#[tokio::test]
async fn a_second_load_while_one_is_running_is_refused_with_409() {
    let dir = tempfile::tempdir().unwrap();
    let path = knx_testsupport::write_minimal_knxproj(dir.path());
    let state = state();
    // Claiming the slot directly is how a test holds an operation "in
    // flight" without a sleep: the handle lives until this scope ends, so
    // the route below meets exactly the state a slow import would leave.
    let held = state
        .load_operations
        .begin(
            knx_server::LoadKind::Import,
            "something-slow.knxproj".to_string(),
            None,
        )
        .expect("a fresh registry has no operation in flight");

    let response = knx_server::app(state.clone(), None)
        .oneshot(post(
            "/api/project/import",
            json!({ "path": path.to_string_lossy() }),
        ))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CONFLICT);
    let error = body_json(response).await;
    let message = error["error"].as_str().unwrap();
    assert!(
        message.contains("already running") && message.contains("operation 1"),
        "the refusal names the operation in the way, which the client can \
         look up in the snapshot: {message}"
    );
    let snapshot = progress(&state).await;
    assert_eq!(
        snapshot["source"], "something-slow.knxproj",
        "the refused load never became the current operation"
    );
    assert_eq!(snapshot["status"], "running");
    held.succeed();
}
