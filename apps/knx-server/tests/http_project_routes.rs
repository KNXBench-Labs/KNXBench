use std::path::PathBuf;
use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use serde_json::{json, Value};
use tower::ServiceExt;

fn reference_ets4_path() -> PathBuf {
    knx_testsupport::reference_ets4_path()
}

async fn body_json(response: axum::response::Response) -> Value {
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

#[tokio::test]
async fn importing_the_reference_project_returns_the_golden_counts() {
    if !reference_ets4_path().exists() {
        eprintln!("skip: OriginalData/ corpus not present (gitignored, local-only)");
        return;
    }
    let state = Arc::new(knx_server::AppState::default());
    let app = knx_server::app(state, None);

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/project/import")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({ "path": reference_ets4_path().to_string_lossy() }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let tree = body_json(response).await;
    assert_eq!(tree["errors"], 0);
    assert_eq!(tree["warnings"], 2);
    assert_eq!(
        tree["installations"][0]["topology"][0]["lines"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
}

#[tokio::test]
async fn importing_a_missing_file_is_a_500() {
    let state = Arc::new(knx_server::AppState::default());
    let app = knx_server::app(state, None);

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/project/import")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({ "path": "/does/not/exist.knxproj" }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    let body = body_json(response).await;
    assert!(!body["error"].as_str().unwrap().is_empty());
}

#[tokio::test]
async fn saving_without_an_open_project_is_a_500() {
    let state = Arc::new(knx_server::AppState::default());
    let app = knx_server::app(state, None);

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/project/save")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
}

#[tokio::test]
async fn importing_then_saving_as_then_reopening_round_trips() {
    if !reference_ets4_path().exists() {
        eprintln!("skip: OriginalData/ corpus not present (gitignored, local-only)");
        return;
    }
    let state = Arc::new(knx_server::AppState::default());
    let app = knx_server::app(state, None);
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("roundtrip.knxdb");

    let import_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/project/import")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({ "path": reference_ets4_path().to_string_lossy() }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(import_response.status(), StatusCode::OK);

    let save_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/project/save-as")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({ "path": db_path.to_string_lossy() }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(save_response.status(), StatusCode::OK);

    let reopen_response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/project/open")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({ "path": db_path.to_string_lossy() }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(reopen_response.status(), StatusCode::OK);
    let tree = body_json(reopen_response).await;
    assert_eq!(tree["errors"], 0);
    assert_eq!(tree["warnings"], 0);
}

fn post(uri: &str, body: Value) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri(uri)
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .unwrap()
}

fn get(uri: &str) -> Request<Body> {
    Request::builder().uri(uri).body(Body::empty()).unwrap()
}

#[tokio::test]
async fn current_project_returns_the_live_tree_and_requires_an_open_project() {
    let state = Arc::new(knx_server::AppState::default());
    let app = knx_server::app(state.clone(), None);

    let missing = app.clone().oneshot(get("/api/project")).await.unwrap();
    assert_eq!(missing.status(), StatusCode::BAD_REQUEST);

    let created = app
        .clone()
        .oneshot(post("/api/project/new", json!({})))
        .await
        .unwrap();
    assert_eq!(created.status(), StatusCode::OK);

    for (name, address) in [("Area A", 1), ("Area B", 2)] {
        let response = app
            .clone()
            .oneshot(post(
                "/api/areas",
                json!({ "name": name, "address": address }),
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }
    let undone = app
        .clone()
        .oneshot(post("/api/undo", json!({})))
        .await
        .unwrap();
    assert_eq!(undone.status(), StatusCode::OK);
    *state.import_counts.lock().unwrap() = (2, 3);

    let response = app.oneshot(get("/api/project")).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let tree = body_json(response).await;
    assert_eq!(tree["errors"], 2);
    assert_eq!(tree["warnings"], 3);
    assert_eq!(tree["can_undo"], true);
    assert_eq!(tree["can_redo"], true);
    let topology = tree["installations"][0]["topology"].as_array().unwrap();
    assert_eq!(topology.len(), 1);
    assert_eq!(topology[0]["name"], "Area A");
}

#[tokio::test]
async fn a_new_project_is_seeded_with_exactly_one_empty_installation() {
    let app = knx_server::app(Arc::new(knx_server::AppState::default()), None);

    let response = app
        .oneshot(post(
            "/api/project/new",
            json!({ "name": "Scratch", "installationName": "Ground floor" }),
        ))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let tree = body_json(response).await;
    let installations = tree["installations"].as_array().unwrap();
    assert_eq!(installations.len(), 1);
    assert_eq!(installations[0]["name"], "Ground floor");
    // No area and no line: `Command::CreateDevice` takes `line: None` and
    // parks the device in `unassigned`, so none is invented here.
    assert!(installations[0]["topology"].as_array().unwrap().is_empty());
    assert!(installations[0]["buildings"].as_array().unwrap().is_empty());
    assert!(installations[0]["unassigned"]
        .as_array()
        .unwrap()
        .is_empty());
    assert!(installations[0]["group_addresses"]
        .as_array()
        .unwrap()
        .is_empty());
    assert_eq!(tree["errors"], 0);
    assert_eq!(tree["warnings"], 0);
}

#[tokio::test]
async fn a_new_project_keeps_the_group_address_style_it_was_created_with() {
    let app = knx_server::app(Arc::new(knx_server::AppState::default()), None);

    let response = app
        .clone()
        .oneshot(post(
            "/api/project/new",
            json!({ "name": "Two level", "groupAddressStyle": "TwoLevel" }),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // A new project has no group addresses, so the tree cannot show its
    // style directly — the only honest way to ask is to make the project
    // parse one. `4/612` is a valid two-level address and an invalid
    // three-level one — three-level wants three components and this has two,
    // so it is rejected on shape before any range is looked at — which means
    // this round trip fails outright if the style silently fell back.
    let created = app
        .clone()
        .oneshot(post(
            "/api/group-addresses",
            json!({ "name": "Kitchen light", "address": "4/612" }),
        ))
        .await
        .unwrap();
    assert_eq!(created.status(), StatusCode::OK);
    let tree = body_json(created).await;
    assert_eq!(
        tree["installations"][0]["group_addresses"][0]["address"],
        "4/612"
    );
}

#[tokio::test]
async fn a_new_project_without_a_stated_style_stays_three_level() {
    let app = knx_server::app(Arc::new(knx_server::AppState::default()), None);

    assert_eq!(
        app.clone()
            .oneshot(post("/api/project/new", json!({})))
            .await
            .unwrap()
            .status(),
        StatusCode::OK
    );

    // The mirror image of the test above: two-level input is rejected and
    // three-level input is accepted, which no other style does.
    assert_eq!(
        app.clone()
            .oneshot(post(
                "/api/group-addresses",
                json!({ "name": "Nope", "address": "4/612" }),
            ))
            .await
            .unwrap()
            .status(),
        StatusCode::BAD_REQUEST
    );
    let created = app
        .oneshot(post(
            "/api/group-addresses",
            json!({ "name": "Kitchen light", "address": "4/2/100" }),
        ))
        .await
        .unwrap();
    assert_eq!(created.status(), StatusCode::OK);
    let tree = body_json(created).await;
    assert_eq!(
        tree["installations"][0]["group_addresses"][0]["address"],
        "4/2/100"
    );
}

#[tokio::test]
async fn a_new_project_refuses_an_unknown_group_address_style_instead_of_guessing() {
    let app = knx_server::app(Arc::new(knx_server::AppState::default()), None);

    let refused = app
        .clone()
        .oneshot(post(
            "/api/project/new",
            json!({ "groupAddressStyle": "FourLevel" }),
        ))
        .await
        .unwrap();
    assert_eq!(refused.status(), StatusCode::BAD_REQUEST);
    let message = body_json(refused).await["error"]
        .as_str()
        .unwrap()
        .to_string();
    assert!(message.contains("FourLevel"), "{message}");
    assert!(message.contains("TwoLevel"), "{message}");

    // And no half-made project was left behind: nothing was created, so
    // there is nothing to add a group address to.
    let orphan = app
        .oneshot(post(
            "/api/group-addresses",
            json!({ "name": "Kitchen light", "address": "4/2/100" }),
        ))
        .await
        .unwrap();
    assert_eq!(orphan.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn a_new_project_refuses_to_discard_unsaved_edits_unless_told_to() {
    let app = knx_server::app(Arc::new(knx_server::AppState::default()), None);

    assert_eq!(
        app.clone()
            .oneshot(post("/api/project/new", json!({})))
            .await
            .unwrap()
            .status(),
        StatusCode::OK
    );
    // An unsaved edit. Nothing in `AppState` tracks dirtiness, so the
    // command stack having something to undo is the signal.
    assert_eq!(
        app.clone()
            .oneshot(post("/api/areas", json!({ "name": "A", "address": 1 })))
            .await
            .unwrap()
            .status(),
        StatusCode::OK
    );

    let refused = app
        .clone()
        .oneshot(post("/api/project/new", json!({})))
        .await
        .unwrap();
    assert_eq!(refused.status(), StatusCode::CONFLICT);
    assert!(body_json(refused).await["error"]
        .as_str()
        .unwrap()
        .contains("unsaved changes"));

    // The edit survived the refusal.
    let log = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/log")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert!(body_json(log)
        .await
        .as_array()
        .unwrap()
        .iter()
        .any(|e| e["source"] == "new" && e["severity"] == "warning"));

    let discarded = app
        .oneshot(post("/api/project/new", json!({ "discardChanges": true })))
        .await
        .unwrap();
    assert_eq!(discarded.status(), StatusCode::OK);
    let tree = body_json(discarded).await;
    assert!(tree["installations"][0]["topology"]
        .as_array()
        .unwrap()
        .is_empty());
}

#[tokio::test]
async fn a_new_project_clears_the_path_the_previous_one_was_loaded_from() {
    if !reference_ets4_path().exists() {
        eprintln!("skip: OriginalData/ corpus not present (gitignored, local-only)");
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("previous.knxdb");
    let app = knx_server::app(Arc::new(knx_server::AppState::default()), None);

    assert_eq!(
        app.clone()
            .oneshot(post(
                "/api/project/import",
                json!({ "path": reference_ets4_path().to_string_lossy() }),
            ))
            .await
            .unwrap()
            .status(),
        StatusCode::OK
    );
    assert_eq!(
        app.clone()
            .oneshot(post(
                "/api/project/save-as",
                json!({ "path": db_path.to_string_lossy() }),
            ))
            .await
            .unwrap()
            .status(),
        StatusCode::OK
    );
    let before = std::fs::metadata(&db_path).unwrap().len();

    assert_eq!(
        app.clone()
            .oneshot(post("/api/project/new", json!({})))
            .await
            .unwrap()
            .status(),
        StatusCode::OK
    );
    // Save must now refuse: an empty project silently overwriting the file
    // the previous one came from is exactly the data loss this guards.
    let save = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/project/save")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_ne!(save.status(), StatusCode::OK);
    assert_eq!(std::fs::metadata(&db_path).unwrap().len(), before);
}
