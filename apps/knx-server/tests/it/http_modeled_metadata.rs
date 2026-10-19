//! Read-only modeled project metadata is authenticated and never a new hardware observation.

use axum::{
    body::Body,
    http::{Request, StatusCode},
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
fn state(dir: &std::path::Path) -> Arc<knx_server::AppState> {
    let state = Arc::new(knx_server::AppState::new(dir.into()));
    let mut project = knx_core::Project::new(knx_core::Language("en".into()));
    project.info.project_id = "P-FICTION".into();
    project.info.name = "  Fictional <name>  ".into();
    project.info.project_number = Some("".into());
    project.info.completion = knx_core::commissioning::CompletionStatus::Tested;
    project.info.last_modified = Some("2010-01-02T03:04:05Z".parse().unwrap());
    project.info.ets_schema_version = 23;
    *state.project.lock().unwrap() = Some(project);
    state
}
#[tokio::test]
async fn exact_modeled_metadata_and_current_binding_do_not_mutate_the_project() {
    let dir = tempfile::tempdir().unwrap();
    let state = state(dir.path());
    let app = knx_server::app(state.clone(), None);
    let before = state.project.lock().unwrap().clone();
    let tree = json(get(&app, "/api/project").await).await;
    let reply = get(&app, "/api/project/metadata").await;
    assert_eq!(
        reply.status(),
        StatusCode::OK,
        "modeled metadata unavailable"
    );
    assert_eq!(
        reply
            .headers()
            .get("cache-control")
            .and_then(|value| value.to_str().ok()),
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
    assert_eq!(view["metadata"]["view_version"], 1);
    assert_eq!(view["metadata"]["scope"], "normalizedProject");
    assert_eq!(view["metadata"]["project_id"], "P-FICTION");
    assert_eq!(view["metadata"]["name"], "  Fictional <name>  ");
    assert_eq!(view["metadata"]["project_number"], "");
    assert_eq!(view["metadata"]["completion"], "Tested");
    assert_eq!(
        view["metadata"]["last_modified"],
        "2010-01-02T03:04:05+00:00"
    );
    assert!(view["metadata"]["project_start"].is_null());
    assert!(view["metadata"].get("hardware_verified").is_none());
    assert!(view["metadata"].get("last_saved_at").is_none());
    assert_eq!(*state.project.lock().unwrap(), before);
    assert_eq!(json(get(&app, "/api/project").await).await, tree);
    state
        .project
        .lock()
        .unwrap()
        .as_mut()
        .unwrap()
        .info
        .project_number = None;
    let absent = json(get(&app, "/api/project/metadata").await).await;
    assert!(absent["metadata"]["project_number"].is_null());
}
#[tokio::test]
async fn missing_project_oversized_metadata_and_auth_refuse_explicitly() {
    let dir = tempfile::tempdir().unwrap();
    let empty = Arc::new(knx_server::AppState::new(dir.path().into()));
    assert_eq!(
        get(&knx_server::app(empty, None), "/api/project/metadata")
            .await
            .status(),
        StatusCode::BAD_REQUEST
    );
    let state = state(dir.path());
    state.project.lock().unwrap().as_mut().unwrap().info.name =
        "x".repeat(knx_app::modeled_metadata::MAX_TEXT_BYTES + 1);
    let before = state.project.lock().unwrap().clone();
    assert_eq!(
        get(
            &knx_server::app(state.clone(), None),
            "/api/project/metadata"
        )
        .await
        .status(),
        StatusCode::UNPROCESSABLE_ENTITY
    );
    assert_eq!(*state.project.lock().unwrap(), before);
    let hash = knx_server::hash_password_with_iterations("fictional metadata guard fixture", 1000)
        .unwrap();
    let guarded = knx_server::app_with_auth(
        state,
        None,
        knx_server::AuthConfig::from_password_hash(&hash).unwrap(),
    );
    assert_eq!(
        get(&guarded, "/api/project/metadata").await.status(),
        StatusCode::UNAUTHORIZED
    );
}
