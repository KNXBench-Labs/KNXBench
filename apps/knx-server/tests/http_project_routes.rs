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
#[ignore = "requires the gitignored OriginalData/ corpus; run with --ignored"]
async fn importing_the_reference_project_returns_the_golden_counts() {
    assert!(
        reference_ets4_path().exists(),
        "OriginalData/ corpus not present (gitignored, local-only); this test is #[ignore]d and must be run explicitly on a machine that has it"
    );
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
#[ignore = "requires the gitignored OriginalData/ corpus; run with --ignored"]
async fn importing_then_saving_as_then_reopening_round_trips() {
    assert!(
        reference_ets4_path().exists(),
        "OriginalData/ corpus not present (gitignored, local-only); this test is #[ignore]d and must be run explicitly on a machine that has it"
    );
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
async fn current_project_save_metadata_tracks_save_open_and_replacement() {
    let dir = tempfile::tempdir().unwrap();
    let state = Arc::new(knx_server::AppState::new(dir.path().to_path_buf()));
    let app = knx_server::app(state, None);

    // Before any save this session, there is no save record at all.
    assert_eq!(
        app.clone()
            .oneshot(post(
                "/api/project/new",
                json!({ "name": "First", "force": true }),
            ))
            .await
            .unwrap()
            .status(),
        StatusCode::OK
    );
    let fresh = body_json(app.clone().oneshot(get("/api/project")).await.unwrap()).await;
    assert_eq!(fresh["has_store_path"], false);
    assert_eq!(fresh["is_modified"], false);
    assert_eq!(
        fresh.get("last_saved_at"),
        None,
        "a session that has never saved reports no save time"
    );

    for (route, body, saved, save_time_expected) in [
        (
            "/api/project/save-as",
            json!({ "path": "first.knxdb" }),
            true,
            true,
        ),
        (
            "/api/project/new",
            json!({ "name": "Second", "force": true }),
            false,
            false,
        ),
        (
            "/api/project/open",
            json!({ "path": "first.knxdb" }),
            true,
            false,
        ),
        ("/api/project/save", json!({}), true, true),
    ] {
        let response = app.clone().oneshot(post(route, body)).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK, "{route}");
        let response = app.clone().oneshot(get("/api/project")).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let tree = body_json(response).await;
        assert_eq!(tree["has_store_path"], saved, "{route}");
        assert_eq!(tree["is_modified"], false, "{route}");
        // `last_saved_at` belongs to the project on screen. Replacing the
        // project (new, open) never saves it, so the status bar must say
        // "not yet saved" rather than show the previous project's time —
        // after "new" that time would describe a project nobody saved.
        assert_eq!(
            tree["last_saved_at"].as_str().is_some(),
            save_time_expected,
            "{route}: last_saved_at = {:?}",
            tree.get("last_saved_at")
        );
    }
}

#[tokio::test]
async fn modified_state_clears_only_after_successful_save_or_save_as() {
    let dir = tempfile::tempdir().unwrap();
    let state = Arc::new(knx_server::AppState::new(dir.path().to_path_buf()));
    let app = knx_server::app(state.clone(), None);
    let db_path = dir.path().join("project.knxdb");

    assert_eq!(
        app.clone()
            .oneshot(post("/api/project/new", json!({})))
            .await
            .unwrap()
            .status(),
        StatusCode::OK
    );
    let edited = app
        .clone()
        .oneshot(post(
            "/api/areas",
            json!({ "name": "Area A", "address": 1 }),
        ))
        .await
        .unwrap();
    assert_eq!(body_json(edited).await["is_modified"], true);
    assert_eq!(
        body_json(app.clone().oneshot(get("/api/project")).await.unwrap())
            .await
            .get("last_saved_at"),
        None,
        "a session that has never saved reports no save time"
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
    let after_save_as = body_json(app.clone().oneshot(get("/api/project")).await.unwrap()).await;
    assert_eq!(after_save_as["is_modified"], false);
    let first_saved_at = after_save_as["last_saved_at"]
        .as_str()
        .expect("save as stamps last_saved_at")
        .to_string();

    let edited_again = app
        .clone()
        .oneshot(post(
            "/api/areas",
            json!({ "name": "Area B", "address": 2 }),
        ))
        .await
        .unwrap();
    assert_eq!(body_json(edited_again).await["is_modified"], true);
    // Editing after a save must not touch the save record — only another
    // successful save may move it.
    assert_eq!(
        body_json(app.clone().oneshot(get("/api/project")).await.unwrap()).await["last_saved_at"],
        first_saved_at
    );

    *state.store_path.lock().unwrap() = Some(dir.path().join("missing/project.knxdb"));
    let failed = app
        .clone()
        .oneshot(post("/api/project/save", json!({})))
        .await
        .unwrap();
    assert_eq!(failed.status(), StatusCode::INTERNAL_SERVER_ERROR);
    let after_failed_save =
        body_json(app.clone().oneshot(get("/api/project")).await.unwrap()).await;
    assert_eq!(after_failed_save["is_modified"], true);
    // A failed save must never clear dirty state, and must never advance
    // the last-saved timestamp — this is the same save record a
    // successful save-as already stamped above.
    assert_eq!(
        after_failed_save["last_saved_at"], first_saved_at,
        "a failed save must not advance the last-saved timestamp"
    );

    *state.store_path.lock().unwrap() = Some(db_path);
    assert_eq!(
        app.clone()
            .oneshot(post("/api/project/save", json!({})))
            .await
            .unwrap()
            .status(),
        StatusCode::OK
    );
    let after_second_save =
        body_json(app.clone().oneshot(get("/api/project")).await.unwrap()).await;
    assert_ne!(
        after_second_save["last_saved_at"], first_saved_at,
        "a successful save must advance the last-saved timestamp"
    );
    assert_eq!(
        body_json(app.oneshot(get("/api/project")).await.unwrap()).await["is_modified"],
        false
    );
}

#[tokio::test]
async fn saved_edit_can_be_replaced_without_a_discard_confirmation() {
    let dir = tempfile::tempdir().unwrap();
    let app = knx_server::app(
        Arc::new(knx_server::AppState::new(dir.path().to_path_buf())),
        None,
    );
    assert_eq!(
        app.clone()
            .oneshot(post("/api/project/new", json!({})))
            .await
            .unwrap()
            .status(),
        StatusCode::OK
    );
    let edited = app
        .clone()
        .oneshot(post(
            "/api/areas",
            json!({ "name": "Unsaved area", "address": 1 }),
        ))
        .await
        .unwrap();
    assert_eq!(body_json(edited).await["is_modified"], true);
    let rejected = app
        .clone()
        .oneshot(post("/api/project/new", json!({})))
        .await
        .unwrap();
    assert_eq!(rejected.status(), StatusCode::CONFLICT);

    assert_eq!(
        app.clone()
            .oneshot(post(
                "/api/project/save-as",
                json!({ "path": dir.path().join("saved.knxdb").to_string_lossy() }),
            ))
            .await
            .unwrap()
            .status(),
        StatusCode::OK
    );
    let saved = body_json(app.clone().oneshot(get("/api/project")).await.unwrap()).await;
    assert_eq!(saved["is_modified"], false);
    let replaced = app
        .oneshot(post("/api/project/new", json!({})))
        .await
        .unwrap();
    assert_eq!(replaced.status(), StatusCode::OK);
    assert_eq!(body_json(replaced).await["is_modified"], false);
}

#[tokio::test]
async fn undo_to_saved_baseline_then_branch_tracks_the_new_unsaved_edit() {
    let dir = tempfile::tempdir().unwrap();
    let app = knx_server::app(
        Arc::new(knx_server::AppState::new(dir.path().to_path_buf())),
        None,
    );
    assert_eq!(
        app.clone()
            .oneshot(post("/api/project/new", json!({})))
            .await
            .unwrap()
            .status(),
        StatusCode::OK
    );
    assert_eq!(
        app.clone()
            .oneshot(post(
                "/api/project/save-as",
                json!({ "path": dir.path().join("saved.knxdb").to_string_lossy() }),
            ))
            .await
            .unwrap()
            .status(),
        StatusCode::OK
    );
    let saved = body_json(app.clone().oneshot(get("/api/project")).await.unwrap()).await;
    let saved_at = saved["last_saved_at"].as_str().unwrap();

    let first_edit = app
        .clone()
        .oneshot(post(
            "/api/areas",
            json!({ "name": "Discarded branch", "address": 1 }),
        ))
        .await
        .unwrap();
    assert_eq!(body_json(first_edit).await["is_modified"], true);
    let undone = app
        .clone()
        .oneshot(post("/api/undo", json!({})))
        .await
        .unwrap();
    let undone = body_json(undone).await;
    assert_eq!(undone["is_modified"], false);
    assert_eq!(undone["can_redo"], true);

    let branch = app
        .clone()
        .oneshot(post(
            "/api/areas",
            json!({ "name": "New branch", "address": 2 }),
        ))
        .await
        .unwrap();
    assert_eq!(branch.status(), StatusCode::OK);
    let branch = body_json(branch).await;
    assert_eq!(branch["is_modified"], true);
    assert_eq!(branch["can_redo"], false);
    assert_eq!(branch["last_saved_at"], saved_at);
    assert_eq!(
        branch["installations"][0]["topology"][0]["name"],
        "New branch"
    );
    assert_eq!(
        app.clone()
            .oneshot(post("/api/project/new", json!({})))
            .await
            .unwrap()
            .status(),
        StatusCode::CONFLICT
    );
    let back = app.oneshot(post("/api/undo", json!({}))).await.unwrap();
    let back = body_json(back).await;
    assert_eq!(back["is_modified"], false);
    assert_eq!(back["last_saved_at"], saved_at);
}

#[tokio::test]
async fn importing_replaces_a_dirty_project_with_a_clean_baseline() {
    let dir = tempfile::tempdir().unwrap();
    let path = knx_testsupport::write_minimal_knxproj(dir.path());
    let state = Arc::new(knx_server::AppState::new(dir.path().to_path_buf()));
    let app = knx_server::app(state, None);

    let created = app
        .clone()
        .oneshot(post("/api/project/new", json!({})))
        .await
        .unwrap();
    assert_eq!(created.status(), StatusCode::OK);
    let created = body_json(created).await;
    let edited = app
        .clone()
        .oneshot(post(
            "/api/areas",
            json!({ "name": "Unsaved area", "address": 2 }),
        ))
        .await
        .unwrap();
    assert_eq!(edited.status(), StatusCode::OK);
    let edited = body_json(edited).await;
    assert_eq!(edited["is_modified"], true);
    assert!(edited["snapshot_revision"].as_u64() > created["snapshot_revision"].as_u64());
    assert!(created["server_incarnation"]
        .as_str()
        .is_some_and(|value| !value.is_empty()));
    assert_eq!(edited["server_incarnation"], created["server_incarnation"]);

    // Since the AR18 review (F1) replacing unsaved edits needs the same
    // explicit consent as `/api/project/new`.
    let imported = app
        .clone()
        .oneshot(post(
            "/api/project/import",
            json!({ "path": path.to_string_lossy(), "discardChanges": true }),
        ))
        .await
        .unwrap();
    assert_eq!(imported.status(), StatusCode::OK);
    let imported = body_json(imported).await;
    assert_eq!(imported["is_modified"], false);
    assert!(imported["snapshot_revision"].as_u64() > edited["snapshot_revision"].as_u64());
    assert_eq!(
        imported["server_incarnation"],
        created["server_incarnation"]
    );

    // The import response is application-stamped; a fresh GET additionally
    // proves that the clean baseline remains the same published revision.
    let current = app.clone().oneshot(get("/api/project")).await.unwrap();
    assert_eq!(current.status(), StatusCode::OK);
    let tree = body_json(current).await;
    assert_eq!(tree["is_modified"], false);
    assert_eq!(tree["can_undo"], false);
    assert_eq!(tree["can_redo"], false);
    assert_eq!(tree["has_store_path"], false);
    assert_eq!(tree["snapshot_revision"], imported["snapshot_revision"]);
    assert_eq!(tree["server_incarnation"], imported["server_incarnation"]);

    let edited = app
        .clone()
        .oneshot(post(
            "/api/areas",
            json!({ "name": "Temporary area", "address": 2 }),
        ))
        .await
        .unwrap();
    assert_eq!(edited.status(), StatusCode::OK);
    let edited = body_json(edited).await;
    assert_eq!(edited["is_modified"], true);
    assert!(edited["snapshot_revision"].as_u64() > tree["snapshot_revision"].as_u64());
    let undone = app.oneshot(post("/api/undo", json!({}))).await.unwrap();
    assert_eq!(undone.status(), StatusCode::OK);
    let tree = body_json(undone).await;
    assert_eq!(tree["is_modified"], false);
    assert_eq!(tree["can_redo"], true);
    assert!(tree["snapshot_revision"].as_u64() > edited["snapshot_revision"].as_u64());
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
    let snapshot = body_json(response).await;
    assert_eq!(snapshot["has_store_path"], false);
    let tree = &snapshot;
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
        StatusCode::UNPROCESSABLE_ENTITY
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
#[ignore = "requires the gitignored OriginalData/ corpus; run with --ignored"]
async fn a_new_project_clears_the_path_the_previous_one_was_loaded_from() {
    assert!(
        reference_ets4_path().exists(),
        "OriginalData/ corpus not present (gitignored, local-only); this test is #[ignore]d and must be run explicitly on a machine that has it"
    );
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

/// AR18 independent review F1: Open and Import used to replace a project
/// with unsaved edits without asking, while New project refused with 409.
#[tokio::test]
async fn opening_or_importing_refuses_to_discard_unsaved_edits_unless_told_to() {
    let dir = tempfile::tempdir().unwrap();
    let source = knx_testsupport::write_minimal_knxproj(dir.path());
    let state = Arc::new(knx_server::AppState::new(dir.path().to_path_buf()));
    let app = knx_server::app(state, None);

    let send = |uri: &'static str, body: Value| {
        let app = app.clone();
        async move {
            let response = app.oneshot(post(uri, body)).await.unwrap();
            let status = response.status();
            let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
                .await
                .unwrap();
            let value = if bytes.is_empty() {
                Value::Null
            } else {
                serde_json::from_slice(&bytes).unwrap()
            };
            (status, value)
        }
    };
    let (status, _) = send("/api/project/new", json!({})).await;
    assert_eq!(status, StatusCode::OK);
    let (status, _) = send("/api/project/save-as", json!({ "path": "saved.knxdb" })).await;
    assert_eq!(status, StatusCode::OK);
    let (status, edited) = send(
        "/api/areas",
        json!({ "name": "Unsaved area", "address": 2 }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(edited["is_modified"], true);

    for (uri, body) in [
        (
            "/api/project/import",
            json!({ "path": source.to_string_lossy() }),
        ),
        ("/api/project/open", json!({ "path": "saved.knxdb" })),
        (
            "/api/project/import",
            json!({ "path": source.to_string_lossy(), "discardChanges": false }),
        ),
    ] {
        let (status, refusal) = send(uri, body).await;
        assert_eq!(status, StatusCode::CONFLICT, "{uri}: {refusal}");
        assert_eq!(refusal["kind"], "projectUnsavedChanges", "{uri}: {refusal}");
        let current = app.clone().oneshot(get("/api/project")).await.unwrap();
        let current = body_json(current).await;
        assert_eq!(
            current["is_modified"], true,
            "{uri} must leave the edit in place"
        );
        assert_eq!(
            current["snapshot_revision"], edited["snapshot_revision"],
            "{uri}"
        );
    }

    // The refusal comes before the file is read at all: a broken file that
    // would fail to load is answered with the same 409 and left untouched.
    std::fs::write(dir.path().join("broken.knxproj"), b"not a zip").unwrap();
    std::fs::write(dir.path().join("broken.knxdb"), b"not a database").unwrap();
    for (uri, path) in [
        ("/api/project/import", "broken.knxproj"),
        ("/api/project/open", "broken.knxdb"),
    ] {
        let (status, refusal) = send(uri, json!({ "path": path })).await;
        assert_eq!(status, StatusCode::CONFLICT, "{uri}: {refusal}");
        assert_eq!(refusal["kind"], "projectUnsavedChanges", "{uri}: {refusal}");
    }
    assert_eq!(
        std::fs::read(dir.path().join("broken.knxdb")).unwrap(),
        b"not a database",
        "a refused open must not migrate the file"
    );

    let (status, opened) = send(
        "/api/project/open",
        json!({ "path": "saved.knxdb", "discardChanges": true }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{opened}");
    assert_eq!(opened["is_modified"], false);
}
