//! Real route-to-service diagnostic transport with independent synthetic data.
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use serde_json::{json, Value};
use std::{
    path::Path,
    sync::{Arc, Mutex},
};
use tower::ServiceExt;

async fn post(app: &axum::Router, path: &str, body: Value) -> Vec<u8> {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(path)
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    axum::body::to_bytes(response.into_body(), 8 * 1024 * 1024)
        .await
        .unwrap()
        .to_vec()
}
#[tokio::test]
async fn import_transports_source_observations_and_manufacturer_findings_to_session_log() {
    let dir = tempfile::tempdir().unwrap();
    let mut input = knx_etsproj::Container::open(knx_testsupport::minimal_knxproj_bytes()).unwrap();
    let topology = input.read("P-0001/0.xml").unwrap();
    let info = input.read("P-0001/Project.xml").unwrap();
    let hardware=br#"<KNX xmlns="http://knx.org/xml/project/23"><ManufacturerData><Manufacturer RefId="M-0001"><Hardware><Hardware Id="M-0001_H-1" Name="Synthetic" Fancy="synthetic"/></Hardware></Manufacturer></ManufacturerData></KNX>"#;
    let source = dir.path().join("synthetic.knxproj");
    std::fs::write(
        &source,
        knx_testsupport::zip_with_entries(&[
            ("P-0001.signature", b""),
            ("P-0001/0.xml", &topology),
            ("P-0001/Project.xml", &info),
            ("M-0001/Hardware.xml", hardware),
        ]),
    )
    .unwrap();
    let state = Arc::new(knx_server::AppState {
        product_db: Some(Mutex::new(
            knx_productdb::open_and_migrate(&dir.path().join("products.sqlite")).unwrap(),
        )),
        data_dir: dir.path().to_path_buf(),
        ..Default::default()
    });
    let app = knx_server::app(state.clone(), None);
    for _ in 0..2 {
        post(&app, "/api/project/import", json!({"path":source})).await;
        let log = state.session_log.lock().unwrap();
        assert!(log
            .entries()
            .iter()
            .any(|e| e.source == "import:source-observations"
                && e.message.contains("not semantic acceptance")));
        assert!(log.entries().iter().any(|e| e.source == "import:unknown"
            && e.message.contains("Fancy")
            && e.message.contains("M-0001/Hardware.xml")));
    }
    let target = dir.path().join("native.knxdb");
    post(&app, "/api/project/save-as", json!({"path":target})).await;
    post(&app, "/api/project/open", json!({"path":target})).await;
    let conn = knx_store::open_existing_and_migrate(Path::new(&target)).unwrap();
    assert!(knx_store::load_opaque(&conn)
        .unwrap()
        .iter()
        .any(|e| e.source_path == "P-0001/0.xml" && e.bytes == topology));
}
