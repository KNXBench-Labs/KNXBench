//! Native exports bind the reviewed snapshot without changing legacy streamed downloads.

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use serde_json::Value;
use std::sync::Arc;
use tower::ServiceExt;

async fn get(app: &axum::Router, url: &str) -> axum::response::Response {
    app.clone()
        .oneshot(Request::builder().uri(url).body(Body::empty()).unwrap())
        .await
        .unwrap()
}
fn setup(dir: &std::path::Path) -> Arc<knx_server::AppState> {
    let state = Arc::new(knx_server::AppState::new(dir.into()));
    let mut project = knx_core::Project::new(knx_core::Language("en".into()));
    project.info.name = "Fictional current export".into();
    *state.project.lock().unwrap() = Some(project);
    let bytes = b"preserved fictional private attachment".to_vec();
    *state.opaque.lock().unwrap() = vec![knx_store::StoredOpaqueEntry {
        source_path: "P-FICTION/UserFiles/example.txt".into(),
        xpath: String::new(),
        kind: "ContainerEntry".into(),
        name: String::new(),
        sha256: knx_store::project_history::image_hash(&bytes),
        bytes,
    }];
    *state.manufacturer_refs.lock().unwrap() = vec![knx_store::ManufacturerRef {
        source_path: "M-FICTION/Hardware.xml".into(),
        kind: "ManufacturerData".into(),
        sha256: "c".repeat(64),
        len: 17,
    }];
    state
}
async fn binding(app: &axum::Router) -> String {
    let reply = get(app, "/api/project").await;
    assert_eq!(reply.status(), StatusCode::OK);
    let tree: Value = serde_json::from_slice(
        &axum::body::to_bytes(reply.into_body(), 1024 * 1024)
            .await
            .unwrap(),
    )
    .unwrap();
    format!(
        "serverIncarnation={}&projectIncarnation={}&snapshotRevision={}",
        tree["server_incarnation"].as_str().unwrap(),
        tree["project_incarnation"],
        tree["snapshot_revision"]
    )
}
#[tokio::test]
async fn reviewed_export_is_current_inert_native_and_preserves_source_state() {
    let dir = tempfile::tempdir().unwrap();
    let state = setup(dir.path());
    let app = knx_server::app(state.clone(), None);
    let before = state.project.lock().unwrap().clone().unwrap();
    let opaque = state.opaque.lock().unwrap().clone();
    let refs = state.manufacturer_refs.lock().unwrap().clone();
    let query = binding(&app).await;
    let reply = get(&app, &format!("/api/project/download?{query}")).await;
    assert_eq!(reply.status(), StatusCode::OK);
    assert_eq!(
        reply
            .headers()
            .get("cache-control")
            .and_then(|v| v.to_str().ok()),
        Some("no-store")
    );
    assert_eq!(
        reply
            .headers()
            .get("x-content-type-options")
            .and_then(|v| v.to_str().ok()),
        Some("nosniff")
    );
    assert_eq!(
        reply
            .headers()
            .get("content-disposition")
            .and_then(|v| v.to_str().ok()),
        Some("attachment; filename=\"project.knxdb\"")
    );
    let bytes = axum::body::to_bytes(reply.into_body(), 1024 * 1024)
        .await
        .unwrap();
    let path = dir.path().join("download.knxdb");
    std::fs::write(&path, bytes).unwrap();
    let conn = knx_store::open_existing_and_migrate(&path).unwrap();
    assert_eq!(knx_store::load_project(&conn).unwrap(), before);
    assert_eq!(knx_store::load_opaque(&conn).unwrap(), opaque);
    assert_eq!(knx_store::load_manufacturer_refs(&conn).unwrap(), refs);
    assert_eq!(state.project.lock().unwrap().as_ref().unwrap(), &before);
    assert_eq!(*state.opaque.lock().unwrap(), opaque);
    assert_eq!(*state.manufacturer_refs.lock().unwrap(), refs);
    assert!(state.store_path.lock().unwrap().is_none());
}
#[tokio::test]
async fn stale_partial_or_unknown_export_binding_refuses_without_mutating_anything() {
    let dir = tempfile::tempdir().unwrap();
    let state = setup(dir.path());
    let app = knx_server::app(state.clone(), None);
    let query = binding(&app).await;
    let before = state.project.lock().unwrap().clone();
    let opaque = state.opaque.lock().unwrap().clone();
    for wrong in [
        query.replace("snapshotRevision=0", "snapshotRevision=999"),
        query.replace("projectIncarnation=0", "projectIncarnation=999"),
        "serverIncarnation=wrong&projectIncarnation=0&snapshotRevision=0".to_string(),
    ] {
        assert_ne!(wrong, query);
        assert_eq!(
            get(&app, &format!("/api/project/download?{wrong}"))
                .await
                .status(),
            StatusCode::CONFLICT
        );
    }
    for partial in [
        "serverIncarnation=wrong",
        "projectIncarnation=0&snapshotRevision=0",
        "unexpected=true",
        "snapshotRevision=invalid",
    ] {
        assert_eq!(
            get(&app, &format!("/api/project/download?{partial}"))
                .await
                .status(),
            StatusCode::BAD_REQUEST
        );
    }
    assert_eq!(*state.project.lock().unwrap(), before);
    assert_eq!(*state.opaque.lock().unwrap(), opaque);
    assert!(state.store_path.lock().unwrap().is_none());
    assert_eq!(
        get(&app, "/api/project/download").await.status(),
        StatusCode::OK,
        "legacy authenticated download remains supported"
    );
}
