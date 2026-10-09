//! Contribution analysis never installs into the user's state.
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use serde_json::Value;
use std::{
    io::{Cursor, Write},
    sync::Arc,
};
use tower::ServiceExt;
use zip::write::SimpleFileOptions;

fn package() -> Vec<u8> {
    let mut out = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for (name, text) in [
        (
            "knx_master.xml",
            r#"<KNX xmlns="http://knx.org/xml/project/11"><MasterData><Manufacturers><Manufacturer Id="M-0001" Name="Private manufacturer"/></Manufacturers></MasterData></KNX>"#,
        ),
        (
            "M-0001/Catalog.xml",
            r#"<KNX xmlns="http://knx.org/xml/project/11"><ManufacturerData><Manufacturer RefId="M-0001"><Catalog><CatalogSection Id="M-0001_CG-1" Name="Private room" Number="1" FutureFlag="private-value"><CatalogItem Id="M-0001_CI-1" Name="Private product" Number="1" ProductRefId="M-0001_P-1"/></CatalogSection></Catalog></Manufacturer></ManufacturerData></KNX>"#,
        ),
    ] {
        out.start_file(name, SimpleFileOptions::default()).unwrap();
        out.write_all(text.as_bytes()).unwrap();
    }
    out.finish().unwrap().into_inner()
}

fn upload(uri: &str, bytes: &[u8], options: Option<&str>) -> Request<Body> {
    let mut body = b"--evidence-boundary\r\nContent-Disposition: form-data; name=\"file\"; filename=\"private-source.knxprod\"\r\n\r\n".to_vec();
    body.extend_from_slice(bytes);
    body.extend_from_slice(b"\r\n");
    if let Some(options) = options {
        body.extend_from_slice(
            b"--evidence-boundary\r\nContent-Disposition: form-data; name=\"options\"\r\n\r\n",
        );
        body.extend_from_slice(options.as_bytes());
        body.extend_from_slice(b"\r\n");
    }
    body.extend_from_slice(b"--evidence-boundary--\r\n");
    Request::builder()
        .method("POST")
        .uri(uri)
        .header(
            "content-type",
            "multipart/form-data; boundary=evidence-boundary",
        )
        .body(Body::from(body))
        .unwrap()
}

fn ap1_package() -> Vec<u8> {
    knx_testsupport::zip_with_entries(&[
        ("knx_master.xml", br#"<KNX xmlns="http://knx.org/xml/project/20"><MasterData><Manufacturers><Manufacturer Id="M-0001" Name="Synthetic"/></Manufacturers><MaskVersions><MaskVersion Id="MV-07B0"><HawkConfigurationData><Procedures><Procedure ProcedureType="Load" ProcedureSubType="ap1"><LdCtrlConnect/><LdCtrlMerge MergeId="2"/><LdCtrlMerge MergeId="4"/><LdCtrlRestart/></Procedure></Procedures></HawkConfigurationData></MaskVersion></MaskVersions></MasterData></KNX>"#),
        ("M-0001/A.xml", br#"<KNX xmlns="http://knx.org/xml/project/20"><ManufacturerData><Manufacturer RefId="M-0001"><ApplicationPrograms><ApplicationProgram Id="M-0001_A-1" MaskVersion="MV-07B0" LoadProcedureStyle="MergedProcedure"><Static><LoadProcedures><LoadProcedure MergeId="2"><LdCtrlRelSegment LsmIdx="4" Size="16"/></LoadProcedure><LoadProcedure MergeId="4"><LdCtrlWriteRelMem ObjIdx="4" Size="16"/></LoadProcedure></LoadProcedures></Static></ApplicationProgram></ApplicationPrograms></Manufacturer></ManufacturerData></KNX>"#),
    ])
}

#[tokio::test]
async fn ap1_local_resolution_and_reduced_preview_keep_user_state_unchanged() {
    let dir = tempfile::tempdir().unwrap();
    let products = knx_productdb::open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
    knx_productdb::install_package(&products, "seed.knxprod", &package()).unwrap();
    let before = products.total_changes();
    let seed =
        knx_etsproj::import_knxproj_bytes(knx_testsupport::minimal_knxproj_bytes(), "seed.knxproj")
            .unwrap()
            .project;
    assert!(!seed.installations.is_empty());
    let state = Arc::new(knx_server::AppState {
        project: std::sync::Mutex::new(Some(seed.clone())),
        product_db: Some(std::sync::Mutex::new(products)),
        ..Default::default()
    });
    let app = knx_server::app(state.clone(), None);
    let response = app
        .clone()
        .oneshot(upload("/api/contributions/analyze", &ap1_package(), None))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let raw = axum::body::to_bytes(response.into_body(), 8 * 1024 * 1024)
        .await
        .unwrap();
    let local: Value = serde_json::from_slice(&raw).unwrap();
    assert_eq!(local["procedureResolutions"][0]["status"], "expanded");
    assert_eq!(local["procedureResolutions"][0]["executable"], false);
    let sha = local["procedureResolutions"][0]["sources"][0]["sha256"]
        .as_str()
        .unwrap();
    let response = app
        .oneshot(upload(
            "/api/contributions/preview",
            &ap1_package(),
            Some(r#"{"audience":"public","sampleIds":[],"consent":false}"#),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let raw = axum::body::to_bytes(response.into_body(), 8 * 1024 * 1024)
        .await
        .unwrap();
    let preview: Value = serde_json::from_slice(&raw).unwrap();
    let text = preview["files"]
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["path"] == "findings.json")
        .unwrap()["text"]
        .as_str()
        .unwrap();
    assert!(!text.contains("procedureResolutions"));
    assert!(!text.contains(sha));
    assert!(text.contains("offline-procedure-resolution"));
    assert_eq!(
        state
            .product_db
            .as_ref()
            .unwrap()
            .lock()
            .unwrap()
            .total_changes(),
        before
    );
    assert_eq!(*state.project.lock().unwrap(), Some(seed));
}

#[tokio::test]
async fn analysis_exposes_real_parser_gap_without_installing_the_package() {
    let dir = tempfile::tempdir().unwrap();
    let products = knx_productdb::open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
    knx_productdb::install_package(&products, "seed.knxprod", &package()).unwrap();
    let seed =
        knx_etsproj::import_knxproj_bytes(knx_testsupport::minimal_knxproj_bytes(), "seed.knxproj")
            .unwrap()
            .project;
    let before: i64 = products
        .query_row("SELECT COUNT(*) FROM source_file", [], |r| r.get(0))
        .unwrap();
    let state = Arc::new(knx_server::AppState {
        project: std::sync::Mutex::new(Some(seed.clone())),
        product_db: Some(std::sync::Mutex::new(products)),
        ..Default::default()
    });
    let response = knx_server::app(state.clone(), None)
        .oneshot(upload("/api/contributions/analyze", &package(), None))
        .await
        .unwrap();
    assert_eq!(
        response.status(),
        StatusCode::OK,
        "missing read-only analysis route"
    );
    let raw = axum::body::to_bytes(response.into_body(), 4 * 1024 * 1024)
        .await
        .unwrap();
    let result: Value = serde_json::from_slice(&raw).unwrap();
    assert_eq!(result["formatVersion"], 1);
    assert_eq!(result["kind"], "product");
    assert_eq!(result["scheme"], 11);
    let gap = result["findings"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["name"] == "FutureFlag")
        .unwrap();
    assert_eq!(gap["category"], "unknown");
    assert_eq!(gap["occurrences"], 1);
    assert_eq!(gap["sample"], "private-value");
    let products = state.product_db.as_ref().unwrap().lock().unwrap();
    let after: i64 = products
        .query_row("SELECT COUNT(*) FROM source_file", [], |r| r.get(0))
        .unwrap();
    assert_eq!(after, before);
    assert_eq!(*state.project.lock().unwrap(), Some(seed));
}

#[tokio::test]
async fn public_bundle_is_reduced_and_contains_no_source_values_or_original() {
    let app = knx_server::app(Arc::new(knx_server::AppState::default()), None);
    let response = app
        .oneshot(upload(
            "/api/contributions/export",
            &package(),
            Some(r#"{"audience":"public","sampleIds":[],"includeOriginal":false,"consent":true}"#),
        ))
        .await
        .unwrap();
    assert_eq!(
        response.status(),
        StatusCode::OK,
        "missing evidence export route"
    );
    assert_eq!(response.headers()["content-type"], "application/zip");
    let raw = axum::body::to_bytes(response.into_body(), 8 * 1024 * 1024)
        .await
        .unwrap();
    let mut archive = zip::ZipArchive::new(Cursor::new(raw)).unwrap();
    let names: Vec<_> = archive.file_names().map(str::to_owned).collect();
    assert_eq!(names, vec!["manifest.json", "findings.json", "README.md"]);
    let mut findings = String::new();
    std::io::Read::read_to_string(
        &mut archive.by_name("findings.json").unwrap(),
        &mut findings,
    )
    .unwrap();
    assert!(findings.contains("FutureFlag"));
    for private in [
        "private-source",
        "private-value",
        "Private room",
        "M-0001_CG-1",
        "M-0001/Catalog.xml",
    ] {
        assert!(
            !findings.contains(private),
            "public evidence leaked source data"
        );
    }
}

#[tokio::test]
async fn preview_is_exact_export_content_and_requires_no_external_submission() {
    let app = knx_server::app(Arc::new(knx_server::AppState::default()), None);
    let response = app
        .oneshot(upload(
            "/api/contributions/preview",
            &package(),
            Some(r#"{"audience":"public","sampleIds":["member-2"],"consent":false}"#),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), 8 * 1024 * 1024)
        .await
        .unwrap();
    let preview: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let files = preview["files"].as_array().unwrap();
    assert!(files.iter().any(|f| f["path"] == "samples/member-2.xml"
        && f["text"].as_str().unwrap().contains("private-value")));
    assert_eq!(preview["manifest"]["disclosure"], "context-samples");
}

#[tokio::test]
async fn all_contribution_routes_use_existing_session_guard() {
    let hash =
        knx_server::hash_password_with_iterations("synthetic test credential", 1000).unwrap();
    let auth = knx_server::AuthConfig::from_password_hash(&hash).unwrap();
    let app = knx_server::app_with_auth(Arc::new(knx_server::AppState::default()), None, auth);
    for operation in ["analyze", "preview", "export"] {
        let uri = format!("/api/contributions/{operation}");
        let response = app
            .clone()
            .oneshot(upload(&uri, &package(), None))
            .await
            .unwrap();
        assert_eq!(
            response.status(),
            StatusCode::UNAUTHORIZED,
            "unguarded {operation}"
        );
    }
}

#[tokio::test]
async fn analysis_refuses_malformed_and_oversized_input_with_client_errors() {
    let app = knx_server::app(Arc::new(knx_server::AppState::default()), None);
    let response = app
        .clone()
        .oneshot(upload("/api/contributions/analyze", b"not a ZIP", None))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_eq!(response.headers()["cache-control"], "no-store");
    let bytes = vec![0; knx_app::contribution::MAX_INPUT_BYTES + 1];
    let response = app
        .oneshot(upload("/api/contributions/analyze", &bytes, None))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
}

#[tokio::test]
async fn export_refuses_public_original_and_unconsented_report() {
    let app = knx_server::app(Arc::new(knx_server::AppState::default()), None);
    for options in [
        r#"{"audience":"public","includeOriginal":true,"originalConsent":true,"consent":true}"#,
        r#"{"audience":"public","consent":false}"#,
    ] {
        let response = app
            .clone()
            .oneshot(upload(
                "/api/contributions/export",
                &package(),
                Some(options),
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        assert_eq!(response.headers()["cache-control"], "no-store");
    }
}
