//! Authenticated source-scoped reports remain separate from primary project findings.

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use serde_json::Value;
use std::sync::Arc;
use tower::ServiceExt;

async fn get(app: &axum::Router, path: &str) -> (StatusCode, Value) {
    let reply = app
        .clone()
        .oneshot(Request::builder().uri(path).body(Body::empty()).unwrap())
        .await
        .unwrap();
    let status = reply.status();
    let bytes = axum::body::to_bytes(reply.into_body(), 20 * 1024 * 1024)
        .await
        .unwrap();
    (
        status,
        if bytes.is_empty() {
            Value::Null
        } else {
            serde_json::from_slice(&bytes).unwrap()
        },
    )
}

#[tokio::test]
async fn recorded_source_catalogue_and_exact_report_read_do_not_replace_primary_counts() {
    let dir = tempfile::tempdir().unwrap();
    let source = knx_app::selective_import::source_from_bytes(
        knx_testsupport::minimal_knxproj_bytes(),
        None,
    )
    .unwrap();
    let state = Arc::new(knx_server::AppState::new(dir.path().into()));
    *state.project.lock().unwrap() = Some(source.project.clone());
    *state.import_counts.lock().unwrap() = (0, 7);
    let exact = source.report.to_json();
    let bytes = exact.as_bytes().to_vec();
    *state.opaque.lock().unwrap() = vec![knx_store::StoredOpaqueEntry {
        source_path: format!("imports/{}/import-report.json", source.source_hash),
        xpath: String::new(),
        kind: "SelectiveImportReport".into(),
        name: String::new(),
        sha256: knx_store::project_history::image_hash(&bytes),
        bytes,
    }];
    let app = knx_server::app(state.clone(), None);
    let before = get(&app, "/api/project").await.1;
    let (status, catalogue) = get(&app, "/api/project/source-reports").await;
    assert_eq!(
        status,
        StatusCode::OK,
        "recorded source reports have no usable catalogue"
    );
    assert_eq!(catalogue["records"].as_array().unwrap().len(), 1);
    assert_eq!(catalogue["records"][0]["id"], source.source_hash);
    assert!(
        catalogue["records"][0].get("report_json").is_none(),
        "catalogue returned full private report body"
    );
    let (status, report) = get(
        &app,
        &format!("/api/project/source-reports/{}", source.source_hash),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(report["scope"], "selectiveSource");
    assert_eq!(report["report_json"], exact);
    assert_eq!(report["project_incarnation"], before["project_incarnation"]);
    assert_eq!(get(&app, "/api/project").await.1, before);
    assert_eq!(before["warnings"], 7);
    // A source report is not silently made a primary project report.
    assert_eq!(
        get(&app, "/api/project/import-report").await.1["available"],
        false
    );
    let (status, _) = get(&app, "/api/project/source-reports/not-a-source-hash").await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(get(&app, "/api/project").await.1, before);
}

#[tokio::test]
async fn source_report_catalogue_uses_existing_login_guard() {
    let dir = tempfile::tempdir().unwrap();
    let hash =
        knx_server::hash_password_with_iterations("fictional source-report guard fixture", 1000)
            .unwrap();
    let app = knx_server::app_with_auth(
        Arc::new(knx_server::AppState::new(dir.path().into())),
        None,
        knx_server::AuthConfig::from_password_hash(&hash).unwrap(),
    );
    let (status, _) = get(&app, "/api/project/source-reports").await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}
