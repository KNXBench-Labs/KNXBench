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
    let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let options = SimpleFileOptions::default();
    for (path, bytes) in [("knx_master.xml", MASTER), ("M-0001/Catalog.xml", CATALOG)] {
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
    let mut state = knx_server::AppState::default();
    state.product_db = Some(std::sync::Mutex::new(
        knx_productdb::open_and_migrate(&dir.path().join("products.sqlite")).unwrap(),
    ));
    (dir, state)
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
    assert_eq!(report["scheme"], 11);
    assert_eq!(report["skipped"], false);
    assert_eq!(report["members"].as_array().unwrap().len(), 2);

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
async fn malformed_and_legacy_product_uploads_are_typed_bad_requests() {
    let (_dir, state) = state();
    let app = knx_server::app(Arc::new(state), None);
    let legacy = package();
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
    assert!(body["error"]
        .as_str()
        .unwrap()
        .contains("legacy .vd2 product data is unsupported"));

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
