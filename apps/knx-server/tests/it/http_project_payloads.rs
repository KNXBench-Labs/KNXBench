//! Retained payload downloads use explicit current-project identity and inert attachment bytes.

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use serde_json::Value;
use std::sync::Arc;
use tower::ServiceExt;

fn state(dir: &std::path::Path) -> Arc<knx_server::AppState> {
    let state = Arc::new(knx_server::AppState::new(dir.into()));
    *state.project.lock().unwrap() = Some(knx_core::Project::new(knx_core::Language("en".into())));
    let bytes = b"<script>foreign code must not render</script>".to_vec();
    *state.opaque.lock().unwrap() = vec![knx_store::StoredOpaqueEntry {
        source_path: "P-FICTION/UserFiles/../../document.html".into(),
        xpath: String::new(),
        kind: "ContainerEntry".into(),
        name: String::new(),
        sha256: knx_store::project_history::image_hash(&bytes),
        bytes,
    }];
    state
}
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
#[tokio::test]
async fn current_bound_download_is_exact_inert_attachment_without_changing_source() {
    let dir = tempfile::tempdir().unwrap();
    let state = state(dir.path());
    let before = state.opaque.lock().unwrap().clone();
    let app = knx_server::app(state.clone(), None);
    let reply = get(&app, "/api/project/payloads").await;
    assert_eq!(
        reply.status(),
        StatusCode::OK,
        "retained payload catalogue unavailable"
    );
    assert_eq!(
        reply
            .headers()
            .get("cache-control")
            .and_then(|v| v.to_str().ok()),
        Some("no-store"),
        "private catalogue must not be cached"
    );
    let list = json(reply).await;
    assert_eq!(list["records"].as_array().unwrap().len(), 1);
    assert_eq!(list["records"][0]["source_path"], before[0].source_path);
    let path=format!("/api/project/payloads/0/download?serverIncarnation={}&projectIncarnation={}&snapshotRevision={}",list["server_incarnation"].as_str().unwrap(),list["project_incarnation"],list["snapshot_revision"]);
    let reply = get(&app, &path).await;
    assert_eq!(reply.status(), StatusCode::OK);
    assert_eq!(reply.headers()["content-type"], "application/octet-stream");
    assert_eq!(
        reply.headers()["content-disposition"],
        "attachment; filename=\"retained-payload.bin\""
    );
    assert_eq!(reply.headers()["x-content-type-options"], "nosniff");
    assert_eq!(reply.headers()["cache-control"], "no-store");
    let bytes = axum::body::to_bytes(reply.into_body(), 1024 * 1024)
        .await
        .unwrap();
    assert_eq!(bytes.as_ref(), before[0].bytes.as_slice());
    assert_eq!(*state.opaque.lock().unwrap(), before);
    let stale = path.replace("snapshotRevision=0", "snapshotRevision=999");
    assert_ne!(stale, path);
    assert_eq!(get(&app, &stale).await.status(), StatusCode::CONFLICT);
    assert_eq!(*state.opaque.lock().unwrap(), before);
}
#[tokio::test]
async fn selected_download_uses_bounded_body_frames_and_foreign_binding_refuses() {
    use http_body_util::BodyExt;
    let dir = tempfile::tempdir().unwrap();
    let state = state(dir.path());
    let bytes = vec![b'x'; 65536 * 3 + 17];
    {
        let mut rows = state.opaque.lock().unwrap();
        rows[0].bytes = bytes.clone();
        rows[0].sha256 = knx_store::project_history::image_hash(&bytes);
    }
    let app = knx_server::app(state.clone(), None);
    let list = json(get(&app, "/api/project/payloads").await).await;
    let url=format!("/api/project/payloads/0/download?serverIncarnation={}&projectIncarnation={}&snapshotRevision={}",list["server_incarnation"].as_str().unwrap(),list["project_incarnation"],list["snapshot_revision"]);
    let reply = get(&app, &url).await;
    assert_eq!(reply.status(), StatusCode::OK);
    let mut body = reply.into_body();
    let mut collected = Vec::new();
    let mut frames = 0;
    while let Some(frame) = body.frame().await {
        if let Ok(data) = frame.unwrap().into_data() {
            assert!(data.len() <= 65536);
            frames += 1;
            collected.extend_from_slice(&data);
        }
    }
    assert!(frames > 1);
    assert_eq!(collected, bytes);
    let wrong_server = url.replace(list["server_incarnation"].as_str().unwrap(), "wrong-server");
    assert_eq!(
        get(&app, &wrong_server).await.status(),
        StatusCode::CONFLICT
    );
    let wrong_load = url.replace("projectIncarnation=0", "projectIncarnation=999");
    assert_ne!(wrong_load, url);
    assert_eq!(get(&app, &wrong_load).await.status(), StatusCode::CONFLICT);
    assert_eq!(state.opaque.lock().unwrap()[0].bytes, bytes);
}

#[tokio::test]
async fn payload_catalogue_and_download_are_guarded_and_missing_binding_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let state = state(dir.path());
    let open = knx_server::app(state.clone(), None);
    assert_eq!(
        get(&open, "/api/project/payloads/0/download")
            .await
            .status(),
        StatusCode::BAD_REQUEST
    );
    let hash =
        knx_server::hash_password_with_iterations("fictional payload guard fixture", 1000).unwrap();
    let app = knx_server::app_with_auth(
        state,
        None,
        knx_server::AuthConfig::from_password_hash(&hash).unwrap(),
    );
    assert_eq!(
        get(&app, "/api/project/payloads").await.status(),
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        get(&app, "/api/project/payloads/0/download").await.status(),
        StatusCode::UNAUTHORIZED
    );
}
