use std::{
    io::Cursor,
    sync::{Arc, Mutex},
};

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use serde_json::{json, Value};
use tower::ServiceExt;
use zip::write::SimpleFileOptions;

async fn get(app: &axum::Router, path: &str) -> Value {
    let response = app
        .clone()
        .oneshot(Request::builder().uri(path).body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    serde_json::from_slice(
        &axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap(),
    )
    .unwrap()
}

async fn post(app: &axum::Router, path: &str, body: Value) -> (StatusCode, Value) {
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
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

#[tokio::test]
async fn status_metrics_bind_saved_file_and_catalogue_to_their_own_resources() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("products.sqlite");
    let db = knx_productdb::open_and_migrate(&db_path).unwrap();
    db.execute_batch("INSERT INTO manufacturer (id, name) VALUES ('M-1','Test');
        INSERT INTO product (id, manufacturer_id, hardware_id, source_sha256) VALUES ('P-1','M-1','H-1','a');
        INSERT INTO application_program (id, manufacturer_id, source_sha256) VALUES ('A-1','M-1','a');").unwrap();
    let state = Arc::new(knx_server::AppState {
        product_db: Some(Mutex::new(db)),
        ..Default::default()
    });
    let app = knx_server::app(state, None);

    let (_, fresh) = post(&app, "/api/project/new", json!({"name":"Measured"})).await;
    let initial = get(&app, "/api/project/status-metrics").await;
    assert_eq!(initial["project"]["state"], "notSavedYet");
    assert_eq!(
        initial["project"]["projectIncarnation"],
        fresh["project_incarnation"]
    );
    assert_eq!(
        initial["project"]["snapshotRevision"],
        fresh["snapshot_revision"]
    );
    assert_eq!(initial["catalogue"]["state"], "available");
    assert_eq!(
        initial["catalogue"]["counts"],
        json!({"manufacturers":1,"products":1,"applicationPrograms":1})
    );
    assert_eq!(
        initial["catalogue"]["bytes"],
        std::fs::metadata(&db_path).unwrap().len()
    );
    assert!(!initial.to_string().contains(dir.path().to_str().unwrap()));

    let path = dir.path().join("saved.knxdb");
    let (save_status, _) = post(&app, "/api/project/save-as", json!({"path":path})).await;
    assert_eq!(save_status, StatusCode::OK);
    let saved = get(&app, "/api/project").await;
    let measured = get(&app, "/api/project/status-metrics").await;
    assert_eq!(measured["project"]["state"], "available");
    assert_eq!(
        measured["project"]["bytes"],
        std::fs::metadata(&path).unwrap().len()
    );
    assert_eq!(
        measured["project"]["snapshotRevision"],
        saved["snapshot_revision"]
    );
    assert!(!measured.to_string().contains(dir.path().to_str().unwrap()));

    let (_, modified) = post(
        &app,
        "/api/project/group-address-style",
        json!({"groupAddressStyle":"Free"}),
    )
    .await;
    assert_eq!(modified["is_modified"], true);
    let before_resave = get(&app, "/api/project/status-metrics").await;
    assert_eq!(
        before_resave["project"]["bytes"],
        std::fs::metadata(&path).unwrap().len()
    );
    assert_eq!(
        before_resave["project"]["snapshotRevision"],
        modified["snapshot_revision"]
    );
    assert_eq!(
        post(&app, "/api/project/save", json!({})).await.0,
        StatusCode::OK
    );
    let resaved = get(&app, "/api/project/status-metrics").await;
    assert_eq!(
        resaved["project"]["bytes"],
        std::fs::metadata(&path).unwrap().len()
    );
    assert_eq!(get(&app, "/api/project").await["is_modified"], false);

    let (failed, _) = post(&app, "/api/project/save-as", json!({"path":dir.path()})).await;
    assert_ne!(failed, StatusCode::OK);
    assert_eq!(
        get(&app, "/api/project/status-metrics").await["project"]["bytes"],
        resaved["project"]["bytes"]
    );
    let other_path = dir.path().join("other.knxdb");
    assert_eq!(
        post(&app, "/api/project/save-as", json!({"path":other_path}))
            .await
            .0,
        StatusCode::OK
    );
    assert_eq!(
        get(&app, "/api/project/status-metrics").await["project"]["bytes"],
        std::fs::metadata(&other_path).unwrap().len()
    );

    let source = knx_testsupport::write_minimal_knxproj(dir.path());
    let (import_status, imported) = post(&app, "/api/project/import", json!({"path":source})).await;
    assert_eq!(import_status, StatusCode::OK);
    let imported_metrics = get(&app, "/api/project/status-metrics").await;
    assert_eq!(imported_metrics["project"]["state"], "notSavedYet");
    assert_eq!(
        imported_metrics["project"]["projectIncarnation"],
        imported["project_incarnation"]
    );
    let (_, next) = post(&app, "/api/project/new", json!({"name":"Other"})).await;
    let switched = get(&app, "/api/project/status-metrics").await;
    assert_eq!(switched["project"]["state"], "notSavedYet");
    assert_eq!(
        switched["project"]["projectIncarnation"],
        next["project_incarnation"]
    );
}

#[tokio::test]
async fn installing_catalogue_changes_only_catalogue_metrics() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("products.sqlite");
    let db = knx_productdb::open_and_migrate(&db_path).unwrap();
    let state = Arc::new(knx_server::AppState {
        product_db: Some(Mutex::new(db)),
        ..Default::default()
    });
    let app = knx_server::app(state, None);
    let (_, fresh) = post(&app, "/api/project/new", json!({"name":"Unchanged"})).await;
    let before = get(&app, "/api/project/status-metrics").await;

    let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
    writer
        .start_file("knx_master.xml", SimpleFileOptions::default())
        .unwrap();
    std::io::Write::write_all(&mut writer, br#"<KNX xmlns="http://knx.org/xml/project/11"><MasterData><Manufacturers><Manufacturer Id="M-0001" Name="Fixture"/></Manufacturers></MasterData></KNX>"#).unwrap();
    writer
        .start_file("M-0001/Catalog.xml", SimpleFileOptions::default())
        .unwrap();
    std::io::Write::write_all(&mut writer, br#"<KNX xmlns="http://knx.org/xml/project/11"><ManufacturerData><Manufacturer RefId="M-0001"><Catalog><CatalogSection Id="M-0001_CG-1" Name="Examples" Number="1"><CatalogItem Id="M-0001_CI-1" Name="Example" Number="1" ProductRefId="M-0001_P-1"/></CatalogSection></Catalog></Manufacturer></ManufacturerData></KNX>"#).unwrap();
    let bytes = writer.finish().unwrap().into_inner();
    let boundary = "metrics-boundary";
    let mut body = format!("--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"synthetic.knxprod\"\r\n\r\n").into_bytes();
    body.extend_from_slice(&bytes);
    body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/catalog/install")
                .header(
                    "content-type",
                    format!("multipart/form-data; boundary={boundary}"),
                )
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let after = get(&app, "/api/project/status-metrics").await;
    assert_eq!(before["catalogue"]["counts"]["manufacturers"], 0);
    assert_eq!(after["catalogue"]["counts"]["manufacturers"], 1);
    assert_eq!(after["catalogue"]["counts"]["products"], 0);
    assert_eq!(
        after["catalogue"]["bytes"],
        std::fs::metadata(&db_path).unwrap().len()
    );
    assert_eq!(after["project"]["state"], "notSavedYet");
    assert_eq!(
        after["project"]["projectIncarnation"],
        fresh["project_incarnation"]
    );
}

#[tokio::test]
async fn status_metrics_distinguish_unavailable_database_from_empty_catalogue() {
    let dir = tempfile::tempdir().unwrap();
    let absent = knx_server::app(
        Arc::new(knx_server::AppState::new(dir.path().to_path_buf())),
        None,
    );
    assert_eq!(
        get(&absent, "/api/project/status-metrics").await["catalogue"]["state"],
        "unavailable"
    );

    let db_path = dir.path().join("empty.sqlite");
    let db = knx_productdb::open_and_migrate(&db_path).unwrap();
    let present = knx_server::app(
        Arc::new(knx_server::AppState {
            product_db: Some(Mutex::new(db)),
            ..Default::default()
        }),
        None,
    );
    let metrics = get(&present, "/api/project/status-metrics").await;
    assert_eq!(metrics["catalogue"]["state"], "available");
    assert_eq!(
        metrics["catalogue"]["counts"],
        json!({"manufacturers":0,"products":0,"applicationPrograms":0})
    );
}

#[tokio::test]
async fn status_metrics_report_missing_files_as_errors_without_paths() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("products.sqlite");
    let db = knx_productdb::open_and_migrate(&db_path).unwrap();
    let state = Arc::new(knx_server::AppState {
        product_db: Some(Mutex::new(db)),
        ..Default::default()
    });
    let app = knx_server::app(state, None);
    post(&app, "/api/project/new", json!({"name":"Saved"})).await;
    let path = dir.path().join("project.knxdb");
    assert_eq!(
        post(&app, "/api/project/save-as", json!({"path":path}))
            .await
            .0,
        StatusCode::OK
    );
    std::fs::remove_file(&path).unwrap();
    std::fs::remove_file(&db_path).unwrap();
    let metrics = get(&app, "/api/project/status-metrics").await;
    assert_eq!(metrics["project"]["state"], "error");
    assert_eq!(metrics["catalogue"]["state"], "error");
    assert!(metrics["project"]["bytes"].is_null());
    assert!(metrics["catalogue"]["bytes"].is_null());
    assert!(!metrics.to_string().contains(dir.path().to_str().unwrap()));
}
