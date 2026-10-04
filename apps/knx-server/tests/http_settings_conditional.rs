//! Conditional preference patches: atomic key-scoped conflicts, no product DB or bus.
// SPDX-License-Identifier: AGPL-3.0-or-later
use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use serde_json::{json, Value};
use tower::ServiceExt;

// Do not call AppState::new/Default: they open/migrate the host product database
// before a later `product_db: None` override can prevent it.
fn fixture() -> (Arc<knx_server::AppState>, tempfile::TempDir) {
    let dir = tempfile::tempdir().unwrap();
    let state = knx_server::AppState {
        project: Default::default(),
        clean_project: Default::default(),
        last_saved_at: Default::default(),
        store_path: Default::default(),
        opaque: Default::default(),
        manufacturer_refs: Default::default(),
        command_stack: Default::default(),
        import_counts: Default::default(),
        server_incarnation: "synthetic-settings-test".into(),
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
        data_dir: dir.path().to_path_buf(),
        settings_lock: Default::default(),
        catalog_requests: Default::default(),
    };
    (Arc::new(state), dir)
}

async fn send(state: &Arc<knx_server::AppState>, method: &str, body: Value) -> (StatusCode, Value) {
    let response = knx_server::app(state.clone(), None)
        .oneshot(
            Request::builder()
                .method(method)
                .uri("/api/settings")
                .header("content-type", "application/json")
                .body(if method == "GET" {
                    Body::empty()
                } else {
                    Body::from(body.to_string())
                })
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    (status, serde_json::from_slice(&bytes).unwrap())
}

#[tokio::test]
async fn stale_conditional_theme_patch_refuses_before_any_file_write() {
    let (state, dir) = fixture();
    let file = dir.path().join(knx_server::SETTINGS_FILE_NAME);
    let original = json!({ "schemaVersion": 1, "settings": {
        "theme": "graphite", "uiThemePacks": { "user-other": { "opaqueFuture": [1, 2] } },
        "futurePreference": { "retained": true }
    }})
    .to_string();
    std::fs::write(&file, &original).unwrap();
    let (status, body) = send(
        &state,
        "PUT",
        json!({
            "settings": { "theme": "system", "uiThemePacks": {} },
            "expectedSettings": { "theme": "porcelain", "uiThemePacks": {} }
        }),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT, "{body}");
    assert_eq!(std::fs::read_to_string(&file).unwrap(), original);
    assert!(!dir.path().join("settings.json.tmp").exists());
    assert!(state.product_db.is_none());
    assert!(state.bus_session.lock().await.is_none());
}

#[tokio::test]
async fn server_advertises_conditional_capability_without_changing_file_schema() {
    let (state, _dir) = fixture();
    let (status, body) = send(&state, "GET", Value::Null).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["conditionalPatchVersion"], 1);
    assert_eq!(body["schemaVersion"], 1);
    assert_eq!(body["status"], "absent");
}

#[tokio::test]
async fn conditional_patch_does_not_replace_a_just_quarantined_record() {
    let (state, dir) = fixture();
    let file = dir.path().join(knx_server::SETTINGS_FILE_NAME);
    let damaged = "{ original damaged preferences";
    std::fs::write(&file, damaged).unwrap();
    let (status, _) = send(
        &state,
        "PUT",
        json!({
            "settings": { "theme": "system" }, "expectedSettings": { "theme": null }
        }),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert!(!file.exists());
    let files: Vec<_> = std::fs::read_dir(dir.path())
        .unwrap()
        .map(|p| p.unwrap().path())
        .collect();
    assert_eq!(files.len(), 1);
    assert_eq!(std::fs::read_to_string(&files[0]).unwrap(), damaged);
}

#[tokio::test]
async fn matching_snapshot_updates_coupled_keys_and_retains_unmentioned_preferences_after_restart()
{
    let (state, dir) = fixture();
    let initial = json!({ "theme": "user-test", "uiThemePacks": { "user-test": { "original": true } },
        "futurePreference": ["unchanged", { "nested": 0 }], "accent": "mint" });
    let file = dir.path().join(knx_server::SETTINGS_FILE_NAME);
    std::fs::write(
        &file,
        json!({ "schemaVersion": 1, "settings": initial }).to_string(),
    )
    .unwrap();
    let (status, body) = send(&state, "PUT", json!({
        "settings": { "theme": "system", "uiThemePacks": {} },
        "expectedSettings": { "theme": initial["theme"], "uiThemePacks": initial["uiThemePacks"] }
    })).await;
    assert_eq!(status, StatusCode::OK);
    let expected = json!({ "theme": "system", "uiThemePacks": {},
        "futurePreference": initial["futurePreference"], "accent": "mint" });
    assert_eq!(body["settings"], expected);
    let disk: Value = serde_json::from_slice(&std::fs::read(file).unwrap()).unwrap();
    assert_eq!(disk, json!({ "schemaVersion": 1, "settings": expected }));
    let (mut restarted, _other_dir) = fixture();
    Arc::get_mut(&mut restarted).unwrap().data_dir = dir.path().to_path_buf();
    assert_eq!(
        send(&restarted, "GET", Value::Null).await.1["settings"],
        expected
    );
}

#[tokio::test]
async fn two_clients_with_the_same_snapshot_cannot_both_overwrite_the_map() {
    let (state, dir) = fixture();
    let file = dir.path().join(knx_server::SETTINGS_FILE_NAME);
    std::fs::write(
        &file,
        json!({ "schemaVersion": 1, "settings": { "theme": "graphite" } }).to_string(),
    )
    .unwrap();
    let request = |id: &str| {
        json!({ "settings": { "theme": id, "uiThemePacks": { id: { "kept": id } } },
        "expectedSettings": { "theme": "graphite", "uiThemePacks": null } })
    };
    let (a, b) = tokio::join!(
        send(&state, "PUT", request("user-a")),
        send(&state, "PUT", request("user-b"))
    );
    let statuses = [a.0, b.0];
    assert_eq!(statuses.iter().filter(|s| **s == StatusCode::OK).count(), 1);
    assert_eq!(
        statuses
            .iter()
            .filter(|s| **s == StatusCode::CONFLICT)
            .count(),
        1
    );
    let winner = if a.0 == StatusCode::OK { a.1 } else { b.1 };
    assert_eq!(
        send(&state, "GET", Value::Null).await.1["settings"],
        winner["settings"]
    );
}

#[tokio::test]
async fn absence_precondition_distinguishes_existing_values_including_stored_null() {
    for value in [Value::Null, json!(false), json!(0), json!({})] {
        let (state, dir) = fixture();
        let file = dir.path().join(knx_server::SETTINGS_FILE_NAME);
        let original = json!({ "schemaVersion": 1, "settings": { "theme": value } }).to_string();
        std::fs::write(&file, &original).unwrap();
        let (status, _) = send(
            &state,
            "PUT",
            json!({ "settings": { "theme": "system" },
            "expectedSettings": { "theme": null } }),
        )
        .await;
        assert_eq!(status, StatusCode::CONFLICT);
        assert_eq!(std::fs::read_to_string(file).unwrap(), original);
    }
    let (state, _dir) = fixture();
    assert_eq!(
        send(
            &state,
            "PUT",
            json!({ "settings": { "theme": "system" },
        "expectedSettings": { "theme": null, "uiThemePacks": null } })
        )
        .await
        .0,
        StatusCode::OK
    );
}

#[tokio::test]
async fn conditional_write_failure_before_rename_keeps_the_old_record() {
    let (state, dir) = fixture();
    let file = dir.path().join(knx_server::SETTINGS_FILE_NAME);
    let original = json!({ "schemaVersion": 1, "settings": { "theme": "graphite" } }).to_string();
    std::fs::write(&file, &original).unwrap();
    std::fs::create_dir(dir.path().join("settings.json.tmp")).unwrap();
    assert_eq!(
        send(
            &state,
            "PUT",
            json!({ "settings": { "theme": "system" },
        "expectedSettings": { "theme": "graphite" } })
        )
        .await
        .0,
        StatusCode::INTERNAL_SERVER_ERROR
    );
    assert_eq!(std::fs::read_to_string(file).unwrap(), original);
}

#[test]
fn conditional_patch_waits_for_the_existing_settings_file_lease() {
    use std::sync::mpsc::{self, RecvTimeoutError};
    use std::time::Duration;

    let (state, dir) = fixture();
    let file = dir.path().join(knx_server::SETTINGS_FILE_NAME);
    let original = json!({ "schemaVersion": 1, "settings": {
        "theme": "graphite", "futurePreference": ["retained"]
    } })
    .to_string();
    std::fs::write(&file, &original).unwrap();

    let (started_tx, started_rx) = mpsc::channel();
    let (response_tx, response_rx) = mpsc::channel();
    let worker_state = state.clone();
    // Hold the real file lease outside the worker's runtime. No synchronous
    // mutex guard crosses an await, and the test never needs a real server.
    let lease = state.settings_lock.lock().unwrap();
    let worker = std::thread::spawn(move || {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        started_tx.send(()).unwrap();
        let response = runtime.block_on(send(
            &worker_state,
            "PUT",
            json!({ "settings": { "theme": "system" },
                "expectedSettings": { "theme": "graphite" } }),
        ));
        response_tx.send(response).unwrap();
    });
    started_rx.recv_timeout(Duration::from_secs(5)).unwrap();
    let early_response = response_rx.recv_timeout(Duration::from_millis(250));
    let waited = matches!(early_response, Err(RecvTimeoutError::Timeout));
    let preserved_while_held = std::fs::read_to_string(&file).unwrap() == original;
    // Always release before asserting or joining: a failing probe must not
    // strand the correctly blocked worker behind its own test's lease.
    drop(lease);
    let response = match early_response {
        Ok(response) => response,
        Err(RecvTimeoutError::Timeout) => response_rx.recv_timeout(Duration::from_secs(5)).unwrap(),
        Err(RecvTimeoutError::Disconnected) => panic!("conditional request worker disconnected"),
    };
    worker.join().unwrap();

    assert!(
        waited,
        "conditional writer bypassed the existing settings file lease"
    );
    assert!(
        preserved_while_held,
        "record changed while another writer held its lease"
    );
    assert_eq!(response.0, StatusCode::OK);
    let expected = json!({ "theme": "system", "futurePreference": ["retained"] });
    assert_eq!(response.1["settings"], expected);
    let stored: Value = serde_json::from_slice(&std::fs::read(&file).unwrap()).unwrap();
    assert_eq!(stored["settings"], expected);
}
