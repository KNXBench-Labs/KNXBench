//! HTTP product-package install returns the measured install report over the wire.

use std::io::Cursor;
use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use serde_json::Value;
use tower::ServiceExt;
use zip::write::SimpleFileOptions;

const BOUNDARY: &str = "knx-product-install-boundary";
const MASTER: &[u8] = br#"<KNX xmlns="http://knx.org/xml/project/11"><MasterData><Manufacturers><Manufacturer Id="M-0001" Name="Example"/></Manufacturers></MasterData></KNX>"#;
const CATALOG: &[u8] = br#"<KNX xmlns="http://knx.org/xml/project/11"><ManufacturerData><Manufacturer RefId="M-0001"><Catalog><CatalogSection Id="M-0001_CG-1" Name="Actuators" Number="1"><CatalogItem Id="M-0001_CI-1" Name="Example actuator" Number="EX-1" ProductRefId="M-0001_P-1"/></CatalogSection></Catalog></Manufacturer></ManufacturerData></KNX>"#;

fn package() -> Vec<u8> {
    package_with_master(MASTER)
}

fn package_with_master(master: &[u8]) -> Vec<u8> {
    let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let options = SimpleFileOptions::default();
    for (path, bytes) in [("knx_master.xml", master), ("M-0001/Catalog.xml", CATALOG)] {
        writer.start_file(path, options).unwrap();
        std::io::Write::write_all(&mut writer, bytes).unwrap();
    }
    writer.finish().unwrap().into_inner()
}

fn multipart(filename: &str, bytes: &[u8]) -> Request<Body> {
    let mut body = Vec::new();
    body.extend_from_slice(format!("--{BOUNDARY}\r\n").as_bytes());
    body.extend_from_slice(
        format!("Content-Disposition: form-data; name=\"file\"; filename=\"{filename}\"\r\n\r\n")
            .as_bytes(),
    );
    body.extend_from_slice(bytes);
    body.extend_from_slice(format!("\r\n--{BOUNDARY}--\r\n").as_bytes());
    Request::builder()
        .method("POST")
        .uri("/api/catalog/install")
        .header(
            "content-type",
            format!("multipart/form-data; boundary={BOUNDARY}"),
        )
        .body(Body::from(body))
        .unwrap()
}

async fn json(response: axum::response::Response) -> Value {
    serde_json::from_slice(
        &axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap(),
    )
    .unwrap()
}

fn state() -> (tempfile::TempDir, knx_server::AppState) {
    let dir = tempfile::tempdir().unwrap();
    let state = knx_server::AppState {
        product_db: Some(std::sync::Mutex::new(
            knx_productdb::open_and_migrate(&dir.path().join("products.sqlite")).unwrap(),
        )),
        ..Default::default()
    };
    (dir, state)
}

fn scheme23_catalog_package(catalog: &str) -> Vec<u8> {
    let master = String::from_utf8(MASTER.to_vec())
        .unwrap()
        .replace("project/11", "project/23")
        .replace("</Manufacturers>", "</Manufacturers><DatapointTypes><DatapointType Id=\"D-23\" Number=\"23\" VariableLength=\"true\"/></DatapointTypes>");
    let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for (path, bytes) in [
        ("knx_master.xml", master.as_bytes()),
        ("M-0001/Catalog.xml", catalog.as_bytes()),
    ] {
        writer
            .start_file(path, SimpleFileOptions::default())
            .unwrap();
        std::io::Write::write_all(&mut writer, bytes).unwrap();
    }
    writer.finish().unwrap().into_inner()
}

#[tokio::test]
async fn scheme23_http_projects_measured_opaque_evidence_and_replays_exact_archive() {
    let (_dir, state) = state();
    let state = Arc::new(state);
    let app = knx_server::app(Arc::clone(&state), None);
    let catalog = String::from_utf8(CATALOG.to_vec())
        .unwrap()
        .replace("project/11", "project/23");
    let bytes = scheme23_catalog_package(&catalog);
    let first = app
        .clone()
        .oneshot(multipart("private-name-23.knxprod", &bytes))
        .await
        .unwrap();
    assert_eq!(first.status(), StatusCode::OK);
    let first = json(first).await;
    assert_eq!(first["scheme"], 23);
    assert_eq!(first["skipped"], false);
    let matching: Vec<_> = first["facts"]["unknownConstructs"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|row| row["name"] == "VariableLength" && row["kind"] == "Attribute")
        .collect();
    assert_eq!(matching.len(), 1);
    assert_eq!(matching[0]["sample"], "true");
    assert_eq!(matching[0]["occurrences"], 1);
    assert!(matching[0]["xpath"]
        .as_str()
        .unwrap()
        .ends_with("/DatapointType"));
    let encoded = first.to_string();
    assert!(!encoded.contains("private-name-23.knxprod"));
    assert!(!encoded.contains("sourceName"));
    assert!(!encoded.contains("source_name"));
    {
        let connection = state.product_db.as_ref().unwrap().lock().unwrap();
        let retained: Vec<u8> = connection
            .query_row("SELECT bytes FROM package", [], |row| row.get(0))
            .unwrap();
        assert!(retained == bytes, "HTTP retained archive bytes changed");
        let retained: Vec<u8> = connection
            .query_row(
                "SELECT bytes FROM source_file WHERE source_path='M-0001/Catalog.xml'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(retained, catalog.as_bytes());
    }
    let replay = app
        .clone()
        .oneshot(multipart("renamed-23.knxprod", &bytes))
        .await
        .unwrap();
    assert_eq!(replay.status(), StatusCode::OK);
    let replay = json(replay).await;
    assert_eq!(replay["scheme"], 23);
    assert_eq!(replay["skipped"], true);
    assert_eq!(replay["sha256"], first["sha256"]);
    assert_eq!(replay["facts"], first["facts"]);
    let found = app
        .oneshot(
            Request::builder()
                .uri("/api/catalog/items?manufacturer=M-0001")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(found.status(), StatusCode::OK);
    assert_eq!(json(found).await[0]["id"], "M-0001_CI-1");
}

#[tokio::test]
async fn scheme23_http_foreign_member_refusal_preserves_seeded_database() {
    let (dir, state) = state();
    let app = knx_server::app(Arc::new(state), None);
    let seed = app
        .clone()
        .oneshot(multipart("seed.knxprod", &package()))
        .await
        .unwrap();
    assert_eq!(seed.status(), StatusCode::OK);
    let database = dir.path().join("products.sqlite");
    let before = std::fs::read(&database).unwrap();
    let catalog = String::from_utf8(CATALOG.to_vec()).unwrap();
    let response = app
        .oneshot(multipart(
            "foreign-23.knxprod",
            &scheme23_catalog_package(&catalog),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let error = json(response).await;
    assert!(
        error["error"]
            .as_str()
            .unwrap()
            .contains("scheme-23 XML contains a non-KNX element namespace"),
        "{error}"
    );
    assert!(
        std::fs::read(&database).unwrap() == before,
        "HTTP refusal changed persisted seed database bytes"
    );
}

#[tokio::test]
async fn installing_a_package_returns_report_and_makes_catalog_item_discoverable() {
    let (_dir, state) = state();
    let app = knx_server::app(Arc::new(state), None);

    let response = app
        .clone()
        .oneshot(multipart("example.knxprod", &package()))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let report = json(response).await;
    eprintln!("install report: {report}");
    assert_eq!(report["scheme"], 11);
    assert_eq!(report["skipped"], false);
    assert_eq!(report["members"].as_array().unwrap().len(), 2);
    assert_eq!(report["facts"]["unknownOccurrences"], 0);
    assert_eq!(report["facts"]["unknownConstructs"], serde_json::json!([]));
    assert_eq!(report["facts"]["diagnostics"], serde_json::json!([]));
    assert_eq!(
        report["facts"]["counts"],
        serde_json::json!([
            {"category":"archive_member","count":2,"disposition":"read"},
            {"category":"archive_member","count":2,"disposition":"stored"},
            {"category":"archive_member","count":0,"disposition":"deduplicated"},
            {"category":"product","count":0,"disposition":"read"},
            {"category":"product","count":0,"disposition":"stored"},
            {"category":"product","count":0,"disposition":"deduplicated"},
            {"category":"application_program","count":0,"disposition":"read"},
            {"category":"application_program","count":0,"disposition":"stored"},
            {"category":"application_program","count":0,"disposition":"deduplicated"},
            {"category":"parameter","count":0,"disposition":"read"},
            {"category":"parameter","count":0,"disposition":"stored"},
            {"category":"communication_object","count":0,"disposition":"read"},
            {"category":"communication_object","count":0,"disposition":"stored"},
            {"category":"dynamic_node","count":0,"disposition":"read"},
            {"category":"dynamic_node","count":0,"disposition":"stored"},
            {"category":"module","count":0,"disposition":"read"},
            {"category":"baggage_index","count":0,"disposition":"read"},
            {"category":"baggage_index","count":0,"disposition":"stored"},
            {"category":"baggage","count":0,"disposition":"read"},
            {"category":"baggage","count":0,"disposition":"stored"},
            {"category":"baggage","count":0,"disposition":"deduplicated"},
            {"category":"baggage","count":0,"disposition":"retained-but-uninterpreted"},
            {"category":"unknown_construct","count":0,"disposition":"read"},
            {"category":"unknown_construct","count":0,"disposition":"stored"},
            {"category":"master_section","count":1,"disposition":"read"},
            {"category":"master_section","count":0,"disposition":"unsupported"},
            {"category":"master_subtree","count":0,"disposition":"unsupported"},
            {"category":"datapoint_type","count":0,"disposition":"read"},
            {"category":"datapoint_type","count":0,"disposition":"stored"},
            {"category":"datapoint_type","count":0,"disposition":"dropped"}
        ])
    );
    let encoded = report.to_string();
    assert!(!encoded.contains("sourceName"));
    assert!(!encoded.contains("source_name"));

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/catalog/items?manufacturer=M-0001")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(json(response).await[0]["id"], "M-0001_CI-1");
}

#[tokio::test]
async fn retry_projects_an_explicit_unavailable_marker_as_null_facts() {
    let (_dir, state) = state();
    let state = Arc::new(state);
    let app = knx_server::app(Arc::clone(&state), None);
    let bytes = package();

    let first = app
        .clone()
        .oneshot(multipart("example.knxprod", &bytes))
        .await
        .unwrap();
    assert_eq!(first.status(), StatusCode::OK);
    let first = json(first).await;
    let sha256 = first["sha256"].as_str().unwrap().to_owned();
    assert!(first["facts"].is_object());

    {
        let connection = state.product_db.as_ref().unwrap().lock().unwrap();
        for table in [
            "package_install_count",
            "package_install_unknown",
            "package_install_diagnostic",
        ] {
            connection
                .execute(
                    &format!("DELETE FROM {table} WHERE package_sha256 = ?1"),
                    [&sha256],
                )
                .unwrap();
        }
        connection
            .execute(
                "UPDATE package_install_report SET status = 'unavailable', unknown_distinct = 0, unknown_occurrences = 0 WHERE package_sha256 = ?1",
                [&sha256],
            )
            .unwrap();
    }

    let retry = app
        .oneshot(multipart("renamed.knxprod", &bytes))
        .await
        .unwrap();
    assert_eq!(retry.status(), StatusCode::OK);
    let retry = json(retry).await;
    assert_eq!(retry["skipped"], true);
    assert!(retry["facts"].is_null());
}

#[tokio::test]
async fn unsupported_diagnostics_expose_only_archive_relative_paths() {
    const MASTER_WITH_FUTURE_SECTION: &[u8] = br#"<KNX xmlns="http://knx.org/xml/project/11"><MasterData><Manufacturers><Manufacturer Id="M-0001" Name="Example"/></Manufacturers><FutureSection/></MasterData></KNX>"#;
    let (_dir, state) = state();
    let app = knx_server::app(Arc::new(state), None);

    let response = app
        .oneshot(multipart(
            "private-host-path.knxprod",
            &package_with_master(MASTER_WITH_FUTURE_SECTION),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let report = json(response).await;
    assert_eq!(
        report["facts"]["diagnostics"],
        serde_json::json!([{
            "kind": "unsupported-master-section",
            "archivePath": "knx_master.xml",
            "xmlPath": "/KNX/MasterData/FutureSection",
            "detail": "master section FutureSection is retained but not interpreted",
            "occurrences": 1
        }])
    );
    let encoded = report.to_string();
    assert!(!encoded.contains("private-host-path.knxprod"));
    assert!(!encoded.contains("sourceName"));
    assert!(!encoded.contains("source_name"));
}

#[tokio::test]
async fn malformed_and_legacy_product_uploads_are_typed_bad_requests() {
    let (_dir, state) = state();
    let app = knx_server::app(Arc::new(state), None);
    let legacy = package();
    let legacy_sha256 = knx_productdb::sha256_hex(&legacy);
    let legacy_len = legacy.len();
    let mut encrypted = package();
    for index in 0..encrypted.len() - 4 {
        if &encrypted[index..index + 4] == b"PK\x01\x02" {
            encrypted[index + 8] |= 1;
        }
        if &encrypted[index..index + 4] == b"PK\x03\x04" {
            encrypted[index + 6] |= 1;
        }
    }

    let malformed = app
        .clone()
        .oneshot(multipart("broken.knxprod", b"not a zip"))
        .await
        .unwrap();
    assert_eq!(malformed.status(), StatusCode::BAD_REQUEST);
    assert!(json(malformed).await["error"]
        .as_str()
        .unwrap()
        .contains("invalid product ZIP"));

    let legacy = app
        .clone()
        .oneshot(multipart("legacy.vd2", &legacy))
        .await
        .unwrap();
    assert_eq!(legacy.status(), StatusCode::BAD_REQUEST);
    let body = json(legacy).await;
    let error = body["error"].as_str().unwrap();
    assert!(error.contains("legacy .vd2 product data is unsupported"));
    assert!(error.contains(&legacy_sha256), "{error}");
    assert!(error.contains(&legacy_len.to_string()), "{error}");

    let encrypted = app
        .oneshot(multipart("encrypted.knxprod", &encrypted))
        .await
        .unwrap();
    assert_eq!(encrypted.status(), StatusCode::BAD_REQUEST);
    let body = json(encrypted).await;
    assert!(body["error"]
        .as_str()
        .unwrap()
        .contains("encrypted product ZIP member"));
}

#[tokio::test]
async fn installing_into_a_readonly_catalog_database_is_an_internal_error() {
    let (_dir, state) = state();
    state
        .product_db
        .as_ref()
        .unwrap()
        .lock()
        .unwrap()
        .execute_batch("PRAGMA query_only = ON")
        .unwrap();
    let app = knx_server::app(Arc::new(state), None);

    let response = app
        .oneshot(multipart("example.knxprod", &package()))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
}

/// KNOWN_LIMITATIONS.md §85: a `.signature` member's `role` reaches this
/// DTO qualified, not as the bare `"Signature"` `knx_productdb` stores —
/// nobody reading an install report over HTTP should mistake the row for
/// a passed check.
#[tokio::test]
async fn a_signature_members_role_is_qualified_as_unverified_in_the_install_report() {
    let (_dir, state) = state();
    let app = knx_server::app(Arc::new(state), None);

    let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let options = SimpleFileOptions::default();
    for (path, bytes) in [
        ("knx_master.xml", MASTER),
        ("M-0001/Catalog.xml", CATALOG),
        (
            "M-0001.signature",
            b"not a real signature, just bytes" as &[u8],
        ),
    ] {
        writer.start_file(path, options).unwrap();
        std::io::Write::write_all(&mut writer, bytes).unwrap();
    }
    let bytes = writer.finish().unwrap().into_inner();

    let response = app
        .oneshot(multipart("signed.knxprod", &bytes))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let report = json(response).await;
    let members = report["members"].as_array().unwrap();
    let signature_member = members
        .iter()
        .find(|m| m["path"] == "M-0001.signature")
        .unwrap();
    assert_eq!(signature_member["role"], "Signature (stored, not verified)");
}
