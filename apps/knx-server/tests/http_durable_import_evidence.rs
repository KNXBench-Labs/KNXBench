//! Full historical import evidence survives the productive native project lifecycle.

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use serde_json::{json, Value};
use std::{
    io::{Cursor, Write},
    sync::Arc,
};
use tower::ServiceExt;

fn fixture() -> Vec<u8> {
    let info = br#"<KNX xmlns="http://knx.org/xml/project/11"><Project Id="P-0001"><ProjectInformation Name="Fictional evidence" GroupAddressStyle="ThreeLevel" CompletionStatus="Undefined" /></Project></KNX>"#;
    let content = br#"<KNX xmlns="http://knx.org/xml/project/11"><Project Id="P-0001"><Installations><Installation InstallationId="0" Name="Fictional" CompletionStatus="Undefined"><Topology /><GroupAddresses><GroupRanges><GroupRange Id="P-0001-0_GR-1" Name="Synthetic" RangeStart="1" RangeEnd="255"><GroupAddress Id="P-0001-0_GA-1" Address="1" Name="Fictional GA" FutureExample="kept" /></GroupRange></GroupRanges></GroupAddresses></Installation></Installations></Project></KNX>"#;
    let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for (name, bytes) in [
        ("P-0001.signature", b"x".as_slice()),
        ("P-0001/Project.xml", info.as_slice()),
        ("P-0001/0.xml", content.as_slice()),
    ] {
        zip.start_file(name, zip::write::SimpleFileOptions::default())
            .unwrap();
        zip.write_all(bytes).unwrap();
    }
    zip.finish().unwrap().into_inner()
}

fn report_value(evidence: &Value) -> Value {
    serde_json::from_str(evidence["report_json"].as_str().unwrap()).unwrap()
}

async fn response(app: &axum::Router, path: &str, data: Option<Value>) -> (StatusCode, Value) {
    let request = Request::builder()
        .method(if data.is_some() { "POST" } else { "GET" })
        .uri(path)
        .header("content-type", "application/json")
        .body(data.map_or_else(Body::empty, |v| Body::from(v.to_string())))
        .unwrap();
    let response = app.clone().oneshot(request).await.unwrap();
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), 20 * 1024 * 1024)
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
async fn ok(app: &axum::Router, path: &str, data: Option<Value>) -> Value {
    let (status, body) = response(app, path, data).await;
    assert_eq!(
        status,
        StatusCode::OK,
        "synthetic lifecycle request refused: {path}"
    );
    body
}

#[tokio::test]
async fn full_import_report_and_historical_counts_survive_save_restart_edit_undo_redo() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("fictional.knxproj");
    let input = fixture();
    std::fs::write(&source, &input).unwrap();
    let native = dir.path().join("fictional.knxdb");
    let state = Arc::new(knx_server::AppState {
        product_db: None,
        ..knx_server::AppState::new(dir.path().into())
    });
    let app = knx_server::app(state.clone(), None);
    let imported = ok(&app, "/api/project/import", Some(json!({"path":source}))).await;
    assert!(imported["warnings"].as_u64().unwrap() > 0);
    let evidence = ok(&app, "/api/project/import-report", None).await;
    assert_eq!(evidence["available"], true);
    assert_eq!(evidence["historical"], false);
    assert!(report_value(&evidence)["unknown"]
        .as_array()
        .unwrap()
        .iter()
        .any(|v| v["name"] == "FutureExample"));
    let exact_report = evidence["report_json"].clone();
    let exact_opaque = state.opaque.lock().unwrap().clone();
    ok(&app, "/api/project/save-as", Some(json!({"path":native}))).await;
    drop(app);
    drop(state);
    let state = Arc::new(knx_server::AppState::new(dir.path().into()));
    let app = knx_server::app(state.clone(), None);
    let reopened = ok(&app, "/api/project/open", Some(json!({"path":native}))).await;
    assert_eq!(
        reopened["warnings"], imported["warnings"],
        "historical warnings became misleading zero"
    );
    assert_eq!(reopened["errors"], imported["errors"]);
    assert_eq!(reopened["import_report_available"], true);
    assert_eq!(reopened["import_report_historical"], true);
    assert_eq!(*state.opaque.lock().unwrap(), exact_opaque);
    for (path, body) in [
        (
            "/api/project/group-address-style",
            Some(json!({"groupAddressStyle":"TwoLevel"})),
        ),
        ("/api/undo", Some(json!({}))),
        ("/api/redo", Some(json!({}))),
        ("/api/project/save", Some(json!({}))),
    ] {
        ok(&app, path, body).await;
        let restored = ok(&app, "/api/project/import-report", None).await;
        assert_eq!(restored["historical"], true);
        assert_eq!(
            restored["report_json"], exact_report,
            "edit changed original historical report"
        );
        let current = ok(&app, "/api/project", None).await;
        assert_eq!(current["warnings"], imported["warnings"]);
    }
    assert_eq!(std::fs::read(&source).unwrap(), input);
    let saved = knx_store::open_existing_read_only(&native).unwrap();
    assert_eq!(knx_store::load_opaque(&saved.conn).unwrap(), exact_opaque);
}

#[tokio::test]
async fn corrupt_native_evidence_refuses_open_without_replacing_project_or_changing_file() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("fictional.knxproj");
    std::fs::write(&source, fixture()).unwrap();
    let state = Arc::new(knx_server::AppState {
        product_db: None,
        ..knx_server::AppState::new(dir.path().into())
    });
    let app = knx_server::app(state.clone(), None);
    ok(&app, "/api/project/import", Some(json!({"path":source}))).await;
    let project = state.project.lock().unwrap().as_ref().unwrap().clone();
    let mut opaque = state.opaque.lock().unwrap().clone();
    let metadata = opaque
        .iter_mut()
        .find(|e| e.kind == "KnxBenchImportEvidence")
        .unwrap();
    metadata.bytes[0] ^= 1;
    let corrupt = dir.path().join("corrupt.knxdb");
    let conn = knx_store::open_and_migrate(&corrupt).unwrap();
    knx_store::save_project_with_passthrough(&conn, &project, &opaque, &[]).unwrap();
    drop(conn);
    let before_file = std::fs::read(&corrupt).unwrap();
    let before_tree = ok(&app, "/api/project", None).await;
    let before_report = ok(&app, "/api/project/import-report", None).await;
    let (status, failure) =
        response(&app, "/api/project/open", Some(json!({"path":corrupt}))).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(failure["kind"], "projectNotOpenable");
    assert_eq!(ok(&app, "/api/project", None).await, before_tree);
    assert_eq!(
        ok(&app, "/api/project/import-report", None).await,
        before_report
    );
    assert_eq!(std::fs::read(&corrupt).unwrap(), before_file);
}

#[tokio::test]
async fn full_report_read_is_covered_by_the_existing_login_guard() {
    let dir = tempfile::tempdir().unwrap();
    let hash =
        knx_server::hash_password_with_iterations("fictional report guard fixture", 1000).unwrap();
    let config = knx_server::AuthConfig::from_password_hash(&hash).unwrap();
    let app = knx_server::app_with_auth(
        Arc::new(knx_server::AppState::new(dir.path().into())),
        None,
        config,
    );
    let (status, _) = response(&app, "/api/project/import-report", None).await;
    assert_eq!(
        status,
        StatusCode::UNAUTHORIZED,
        "report endpoint bypassed login"
    );
}

#[tokio::test]
async fn absent_legacy_report_stays_explicitly_unavailable() {
    let dir = tempfile::tempdir().unwrap();
    let native = dir.path().join("old.knxdb");
    let conn = knx_store::open_and_migrate(&native).unwrap();
    let project = knx_core::Project::new(knx_core::Language("en".into()));
    knx_store::save_project(&conn, &project).unwrap();
    drop(conn);
    let app = knx_server::app(Arc::new(knx_server::AppState::new(dir.path().into())), None);
    let tree = ok(&app, "/api/project/open", Some(json!({"path":native}))).await;
    assert_eq!(tree["import_report_available"], false);
    let evidence = ok(&app, "/api/project/import-report", None).await;
    assert_eq!(evidence["available"], false);
    assert!(evidence["report_json"].is_null());
}
