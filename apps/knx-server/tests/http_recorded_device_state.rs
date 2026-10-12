//! Stored device intent and current model-held reports remain separate in a read-only view.

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use knx_core::{
    commissioning::{load_state::LoadState, CompletionStatus, LoadPart},
    device::DeviceInstance,
    ids::{DeviceId, SourceRef},
    Language, Project,
};
use serde_json::Value;
use std::sync::Arc;
use tower::ServiceExt;

async fn get(app: &axum::Router, path: &str) -> axum::response::Response {
    app.clone()
        .oneshot(Request::builder().uri(path).body(Body::empty()).unwrap())
        .await
        .unwrap()
}
async fn json(reply: axum::response::Response) -> Value {
    serde_json::from_slice(
        &axum::body::to_bytes(reply.into_body(), 1024 * 1024)
            .await
            .unwrap(),
    )
    .unwrap()
}
fn fictional_project() -> Project {
    let mut project = Project::new(Language("en".into()));
    for _ in 0..42 {
        project.ids.next_device_id().unwrap();
    }
    let mut device = DeviceInstance {
        id: DeviceId(42),
        source: SourceRef {
            path: "synthetic-device".into(),
            ets_id: "D-FICTIONAL".into(),
        },
        name: "  Fictional <device>  ".into(),
        description: None,
        address: None,
        product_ref: String::new(),
        program_ref: String::new(),
        commissioning: Default::default(),
        visibility_calculated: false,
        com_objects: vec![],
        binary_data: vec![],
    };
    device.commissioning.completion = CompletionStatus::Tested;
    device.commissioning.individual_address_loaded = true;
    device.commissioning.parameters_loaded = true;
    device.commissioning.last_modified = Some("2010-01-02T03:04:05Z".parse().unwrap());
    device.commissioning.last_download = Some("2009-12-31T01:02:03Z".parse().unwrap());
    device
        .commissioning
        .device_reported
        .record(LoadPart::ApplicationProgram, LoadState::Loaded);
    device
        .commissioning
        .device_reported
        .record(LoadPart::Parameters, LoadState::Error);
    project.devices.insert(device);
    project
        .installations
        .push(knx_core::installation::Installation {
            id: knx_core::ids::InstallationId(0),
            name: "Fictional installation".into(),
            default_line: None,
            multicast_address: None,
            completion: CompletionStatus::Undefined,
            topology: knx_core::topology::Topology {
                areas: vec![],
                lines: vec![],
                unassigned: vec![DeviceId(42)],
            },
            buildings: vec![],
            group_ranges: vec![],
            group_addresses: vec![],
            parameters: vec![],
        });
    project
}
#[tokio::test]
async fn exact_project_belief_is_not_laundered_into_model_report_or_fresh_hardware_verification() {
    let dir = tempfile::tempdir().unwrap();
    let state = Arc::new(knx_server::AppState::new(dir.path().into()));
    *state.project.lock().unwrap() = Some(fictional_project());
    let app = knx_server::app(state.clone(), None);
    let before = state.project.lock().unwrap().clone();
    let tree = json(get(&app, "/api/project").await).await;
    let reply = get(&app, "/api/devices/42/recorded-state").await;
    assert_eq!(
        reply.status(),
        StatusCode::OK,
        "recorded state view is not available"
    );
    assert_eq!(
        reply
            .headers()
            .get("cache-control")
            .and_then(|v| v.to_str().ok()),
        Some("no-store")
    );
    let view = json(reply).await;
    for field in [
        "server_incarnation",
        "project_incarnation",
        "snapshot_revision",
    ] {
        assert_eq!(view[field], tree[field]);
    }
    let data = &view["state"];
    assert_eq!(data["view_version"], 1);
    assert_eq!(data["scope"], "recordedDeviceModel");
    assert_eq!(data["device_id"], 42);
    assert_eq!(data["name"], "  Fictional <device>  ");
    assert_eq!(data["completion"], "Tested");
    assert_eq!(data["broken"], false);
    assert_eq!(data["last_modified"], "2010-01-02T03:04:05+00:00");
    assert_eq!(data["last_download"], "2009-12-31T01:02:03+00:00");
    assert_eq!(
        data["parts"],
        serde_json::json!([
            {"part":"IndividualAddress","intended_loaded":true,"reported":null},
            {"part":"ApplicationProgram","intended_loaded":false,"reported":1},
            {"part":"Parameters","intended_loaded":true,"reported":3},
            {"part":"CommunicationPart","intended_loaded":false,"reported":null},
            {"part":"MediumConfig","intended_loaded":false,"reported":null}
        ])
    );
    assert!(data.get("hardware_verified").is_none());
    assert!(data.get("last_saved_at").is_none());
    assert_eq!(*state.project.lock().unwrap(), before);
    assert_eq!(json(get(&app, "/api/project").await).await, tree);
}

#[tokio::test]
async fn missing_target_invalid_id_budget_and_auth_are_explicit_without_mutation() {
    let dir = tempfile::tempdir().unwrap();
    let state = Arc::new(knx_server::AppState::new(dir.path().into()));
    let app = knx_server::app(state.clone(), None);
    assert_eq!(
        get(&app, "/api/devices/42/recorded-state").await.status(),
        StatusCode::BAD_REQUEST
    );
    *state.project.lock().unwrap() = Some(fictional_project());
    assert_eq!(
        get(&app, "/api/devices/43/recorded-state").await.status(),
        StatusCode::NOT_FOUND
    );
    for id in ["-1", "4294967296", "abc"] {
        assert_eq!(
            get(&app, &format!("/api/devices/{id}/recorded-state"))
                .await
                .status(),
            StatusCode::BAD_REQUEST
        );
    }
    state
        .project
        .lock()
        .unwrap()
        .as_mut()
        .unwrap()
        .devices
        .get_mut(DeviceId(42))
        .unwrap()
        .name = "é".repeat(32768);
    assert_eq!(
        get(&app, "/api/devices/42/recorded-state").await.status(),
        StatusCode::OK
    );
    state
        .project
        .lock()
        .unwrap()
        .as_mut()
        .unwrap()
        .devices
        .get_mut(DeviceId(42))
        .unwrap()
        .name
        .push('x');
    let before = state.project.lock().unwrap().clone();
    assert_eq!(
        get(&app, "/api/devices/42/recorded-state").await.status(),
        StatusCode::UNPROCESSABLE_ENTITY
    );
    assert_eq!(*state.project.lock().unwrap(), before);
    let hash = knx_server::hash_password_with_iterations("fictional-password", 1000).unwrap();
    let config = knx_server::AuthConfig::from_password_hash(&hash).unwrap();
    let auth = knx_server::app_with_auth(state, None, config);
    assert_eq!(
        get(&auth, "/api/devices/42/recorded-state").await.status(),
        StatusCode::UNAUTHORIZED
    );
}
#[tokio::test]
async fn every_known_retained_report_is_projected_without_altering_intent_or_dates() {
    let dir = tempfile::tempdir().unwrap();
    let state = Arc::new(knx_server::AppState::new(dir.path().into()));
    *state.project.lock().unwrap() = Some(fictional_project());
    let app = knx_server::app(state.clone(), None);
    for report in LoadState::ALL {
        {
            let mut lock = state.project.lock().unwrap();
            let d = lock
                .as_mut()
                .unwrap()
                .devices
                .get_mut(DeviceId(42))
                .unwrap();
            for part in LoadPart::ALL {
                d.commissioning.device_reported.record(part, report);
            }
        }
        let before = state.project.lock().unwrap().clone();
        let data = json(get(&app, "/api/devices/42/recorded-state").await).await;
        for part in data["state"]["parts"].as_array().unwrap() {
            assert_eq!(part["reported"], report.octet());
        }
        assert_eq!(data["state"]["parts"][0]["intended_loaded"], true);
        assert_eq!(data["state"]["parts"][1]["intended_loaded"], false);
        assert_eq!(*state.project.lock().unwrap(), before);
    }
}
#[tokio::test]
async fn native_save_refuses_unpersistable_reports_then_explicit_forget_allows_intent_roundtrip() {
    let dir = tempfile::tempdir().unwrap();
    let state = Arc::new(knx_server::AppState::new(dir.path().into()));
    *state.project.lock().unwrap() = Some(fictional_project());
    let app = knx_server::app(state.clone(), None);
    let before = json(get(&app, "/api/devices/42/recorded-state").await).await;
    // The existing exact-native journal refuses model-held reports rather than silently dropping them.
    let original_model = state.project.lock().unwrap().clone();
    let refused = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/project/save-as")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"path":"fictional.knxdb"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(refused.status(), StatusCode::INTERNAL_SERVER_ERROR);
    let reason = json(refused).await;
    assert!(reason["error"]
        .as_str()
        .unwrap()
        .contains("native snapshot would not reopen exactly"));
    assert_eq!(*state.project.lock().unwrap(), original_model);
    assert!(!dir.path().join("fictional.knxdb").exists());
    // Explicit test-only session reset, not a new save-time lossy normalizer.
    state
        .project
        .lock()
        .unwrap()
        .as_mut()
        .unwrap()
        .devices
        .get_mut(DeviceId(42))
        .unwrap()
        .commissioning
        .device_reported
        .forget();
    for (route, path) in [
        ("/api/project/save-as", "fictional.knxdb"),
        ("/api/project/open", "fictional.knxdb"),
    ] {
        let reply = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(route)
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::json!({"path":path}).to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        let status = reply.status();
        let body = axum::body::to_bytes(reply.into_body(), 1024 * 1024)
            .await
            .unwrap();
        assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
    }
    let bytes = std::fs::read(dir.path().join("fictional.knxdb")).unwrap();
    let after = json(get(&app, "/api/devices/42/recorded-state").await).await;
    for key in [
        "name",
        "completion",
        "broken",
        "last_modified",
        "last_download",
    ] {
        assert_eq!(before["state"][key], after["state"][key]);
    }
    for index in 0..5 {
        assert_eq!(
            before["state"]["parts"][index]["intended_loaded"],
            after["state"]["parts"][index]["intended_loaded"]
        );
        assert!(after["state"]["parts"][index]["reported"].is_null());
    }
    assert_eq!(
        std::fs::read(dir.path().join("fictional.knxdb")).unwrap(),
        bytes
    );
    assert_ne!(before["project_incarnation"], after["project_incarnation"]);
}
