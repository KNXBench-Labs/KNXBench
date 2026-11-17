//! Verifies the device-configuration-profile HTTP routes: read, preview with
//! incarnation binding, confirmation-required apply as one undo step, cancel,
//! and refusal without mutation (WP-42.2/WP-42.3, ADR-0175).
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use knx_testsupport::zip_with_entries;
use serde_json::{json, Value};
use std::sync::Arc;
use tower::ServiceExt;

async fn request(
    app: &axum::Router,
    method: &str,
    path: &str,
    payload: Value,
) -> (StatusCode, Value) {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(method)
                .uri(path)
                .header("content-type", "application/json")
                .body(Body::from(payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), 4 * 1024 * 1024)
        .await
        .unwrap();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or_else(|_| json!({"nonJsonError":true})),
    )
}

fn base64url(bytes: &[u8]) -> String {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let mut out = String::new();
    for chunk in bytes.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = *chunk.get(1).unwrap_or(&0) as u32;
        let b2 = *chunk.get(2).unwrap_or(&0) as u32;
        let n = (b0 << 16) | (b1 << 8) | b2;
        out.push(ALPHABET[(n >> 18) as usize & 63] as char);
        out.push(ALPHABET[(n >> 12) as usize & 63] as char);
        if chunk.len() > 1 {
            out.push(ALPHABET[(n >> 6) as usize & 63] as char);
        }
        if chunk.len() > 2 {
            out.push(ALPHABET[n as usize & 63] as char);
        }
    }
    out
}

async fn fixture() -> (tempfile::TempDir, Arc<knx_server::AppState>, axum::Router, u32) {
    let xml = r#"<KNX xmlns="http://knx.org/xml/project/23"><Project Id="P-0001"><Installations><Installation InstallationId="0"><Topology><UnassignedDevices><DeviceInstance Id="P-0001-0_DI-1" Name="Synthetic"/></UnassignedDevices></Topology></Installation></Installations></Project></KNX>"#;
    let info = r#"<KNX xmlns="http://knx.org/xml/project/23"><Project Id="P-0001"><ProjectInformation Name="Synthetic devconf"/></Project></KNX>"#;
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("synthetic.knxproj");
    std::fs::write(
        &path,
        zip_with_entries(&[
            ("P-0001.signature", b""),
            ("P-0001/0.xml", xml.as_bytes()),
            ("P-0001/Project.xml", info.as_bytes()),
        ]),
    )
    .unwrap();
    let state = Arc::new(knx_server::AppState::new(dir.path().to_path_buf()));
    let app = knx_server::app(state.clone(), None);
    let opened = request(
        &app,
        "POST",
        "/api/project/import",
        json!({"path":path,"clientToken":"synthetic-devconf-import"}),
    )
    .await;
    assert_eq!(
        opened.0,
        StatusCode::OK,
        "synthetic fixture setup: {}",
        opened.1
    );
    let id = state
        .project
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .devices
        .iter()
        .next()
        .unwrap()
        .id
        .0;
    (dir, state, app, id)
}

fn set_action(device: u32, document: &str) -> Value {
    json!({"kind":"set","draft":{
        "device":device,
        "moduleId":"example.demo-panel",
        "moduleVersion":"1.0.0",
        "moduleSha256":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        "documentB64":base64url(document.as_bytes())
    }})
}

fn preview_payload(view: &Value, action: Value) -> Value {
    json!({"serverIncarnation":view["serverIncarnation"],
        "projectIncarnation":view["projectIncarnation"],
        "snapshotRevision":view["snapshotRevision"],"action":action})
}

#[tokio::test]
async fn profiles_read_preview_apply_refuse_unconfirmed_and_cancel_is_one_undo_step() {
    let (_dir, state, app, device) = fixture().await;
    let before = state.project.lock().unwrap().clone();

    let (status, view) = request(&app, "GET", "/api/project/device-config-profiles", Value::Null).await;
    assert_eq!(status, StatusCode::OK, "read route: {view}");
    assert_eq!(view["profiles"], json!([]));

    let action = set_action(device, "{\"nodes\": []}");
    let (status, plan) = request(
        &app,
        "POST",
        "/api/project/device-config-profiles/preview",
        preview_payload(&view, action.clone()),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "valid preview: {plan}");
    assert_eq!(plan["action"], action);
    assert_eq!(plan["noOp"], false);
    let token = plan["token"].as_str().unwrap().to_string();

    // Apply without confirmation is refused and changes nothing.
    let (status, body) = request(
        &app,
        "POST",
        "/api/project/device-config-profiles/apply",
        json!({"token":token,"confirmed":false}),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{body}");
    assert_eq!(body["kind"], "devconfConfirmationRequired");
    assert_eq!(*state.project.lock().unwrap(), before, "refusal must not mutate");

    // Wrong token is a conflict.
    let (status, _) = request(
        &app,
        "POST",
        "/api/project/device-config-profiles/apply",
        json!({"token":"not-the-token","confirmed":true}),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);

    // Confirmed apply succeeds, is one undo step, and reports the change.
    let (status, applied) = request(
        &app,
        "POST",
        "/api/project/device-config-profiles/apply",
        json!({"token":token,"confirmed":true}),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "confirmed apply: {applied}");
    assert_eq!(applied["changed"], true);
    {
        let project = state.project.lock().unwrap();
        let project = project.as_ref().unwrap();
        let stored = project.devices.config_profile(knx_core::DeviceId(device)).unwrap();
        assert_eq!(stored.module_id(), "example.demo-panel");
        assert_eq!(stored.document(), b"{\"nodes\": []}".to_vec());
    }
    let after_set = state.project.lock().unwrap().clone();
    assert_ne!(after_set, before);

    // One undo step restores the previous (absent) profile.
    {
        let mut project = state.project.lock().unwrap();
        let project = project.as_mut().unwrap();
        let mut stack = state.command_stack.lock().unwrap();
        stack.undo(project).unwrap();
    }
    assert_eq!(*state.project.lock().unwrap(), before, "undo restores exactly");

    // A stale preview (wrong snapshot revision) conflicts.
    let (status, view2) = request(&app, "GET", "/api/project/device-config-profiles", Value::Null).await;
    assert_eq!(status, StatusCode::OK);
    let mut stale = preview_payload(&view2, set_action(device, "{\"nodes\": []}"));
    stale["snapshotRevision"] = json!(u64::MAX);
    let (status, body) = request(&app, "POST", "/api/project/device-config-profiles/preview", stale).await;
    assert_eq!(status, StatusCode::CONFLICT, "{body}");

    // Preview then cancel leaves the project untouched and clears the token.
    let (status, plan2) = request(
        &app,
        "POST",
        "/api/project/device-config-profiles/preview",
        preview_payload(&view2, set_action(device, "{\"nodes\": [1]}")),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{plan2}");
    let token2 = plan2["token"].as_str().unwrap().to_string();
    let (status, _) = request(
        &app,
        "POST",
        "/api/project/device-config-profiles/cancel",
        json!({"token":token2}),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let (status, _) = request(
        &app,
        "POST",
        "/api/project/device-config-profiles/apply",
        json!({"token":token2,"confirmed":true}),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT, "cancelled token must not apply");
    assert_eq!(*state.project.lock().unwrap(), before);

    // Unknown device is refused by preview with a structured error.
    let (status, body) = request(
        &app,
        "POST",
        "/api/project/device-config-profiles/preview",
        preview_payload(&view2, set_action(99999, "{}")),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{body}");
    assert_eq!(body["kind"], "devconfInvalid");
}

#[tokio::test]
async fn remove_action_requires_an_existing_profile_and_applies_as_one_undo_step() {
    let (_dir, state, app, device) = fixture().await;
    let (status, view) = request(&app, "GET", "/api/project/device-config-profiles", Value::Null).await;
    assert_eq!(status, StatusCode::OK);

    // Remove with nothing stored is refused.
    let (status, body) = request(
        &app,
        "POST",
        "/api/project/device-config-profiles/preview",
        preview_payload(&view, json!({"kind":"remove","device":device})),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{body}");

    // Set first.
    let (status, plan) = request(
        &app,
        "POST",
        "/api/project/device-config-profiles/preview",
        preview_payload(&view, set_action(device, "{\"nodes\": []}")),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{plan}");
    let token = plan["token"].as_str().unwrap().to_string();
    let (status, _) = request(&app, "POST", "/api/project/device-config-profiles/apply", json!({"token":token,"confirmed":true})).await;
    assert_eq!(status, StatusCode::OK);
    let with_profile = state.project.lock().unwrap().clone();

    // Read shows the stored profile.
    let (status, view2) = request(&app, "GET", "/api/project/device-config-profiles", Value::Null).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(view2["profiles"].as_array().unwrap().len(), 1);
    assert_eq!(view2["profiles"][0]["moduleId"], "example.demo-panel");

    // Remove via preview+apply.
    let (status, plan) = request(
        &app,
        "POST",
        "/api/project/device-config-profiles/preview",
        preview_payload(&view2, json!({"kind":"remove","device":device})),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{plan}");
    assert_eq!(plan["noOp"], false);
    let token = plan["token"].as_str().unwrap().to_string();
    let (status, applied) = request(&app, "POST", "/api/project/device-config-profiles/apply", json!({"token":token,"confirmed":true})).await;
    assert_eq!(status, StatusCode::OK, "{applied}");
    assert_eq!(applied["changed"], true);
    assert!(state
        .project
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .devices
        .config_profile(knx_core::DeviceId(device))
        .is_none());

    // Undo restores the exact profile.
    {
        let mut project = state.project.lock().unwrap();
        let project = project.as_mut().unwrap();
        let mut stack = state.command_stack.lock().unwrap();
        stack.undo(project).unwrap();
    }
    assert_eq!(*state.project.lock().unwrap(), with_profile);
}
