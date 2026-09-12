//! T32 Task 4: `GET /api/catalog/items?language=...` overlays the catalog
//! item's translated `Name`. Fixture setup idiom follows
//! `http_product_language.rs` (read there first); the `Catalog.xml` here
//! carries a `Languages` block scoped to the manufacturer, mirroring
//! `crates/knx-productdb/src/parse/translation.rs`'s own `CATALOG` fixture
//! for that shape.

use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use serde_json::Value;
use tower::ServiceExt;

const CATALOG: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<KNX xmlns="http://knx.org/xml/project/11">
  <ManufacturerData>
    <Manufacturer RefId="M-1">
      <Catalog>
        <CatalogSection Id="M-1_CG-1" Name="Actuators" Number="1" DefaultLanguage="de-DE">
          <CatalogItem Id="M-1_CI-1" Name="Schaltaktor" Number="ACT-1"
                       DefaultLanguage="de-DE"
                       ProductRefId="M-1_P-1"
                       Hardware2ProgramRefId="H-1_HP-1" />
        </CatalogSection>
      </Catalog>
      <Languages>
        <Language Identifier="de-DE">
          <TranslationUnit RefId="M-1_CI-1">
            <TranslationElement RefId="M-1_CI-1">
              <Translation AttributeName="Name" Text="Umschaltaktor" />
            </TranslationElement>
          </TranslationUnit>
        </Language>
      </Languages>
    </Manufacturer>
  </ManufacturerData>
</KNX>"#;

fn temp_product_db() -> (tempfile::TempDir, knx_productdb::Connection) {
    let dir = tempfile::tempdir().unwrap();
    let conn = knx_productdb::open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
    // `ingest_file`, not `parse::catalog::ingest_catalog` directly, so the
    // `Languages` second pass also runs (same reasoning
    // `http_product_language.rs`'s own `temp_product_db` gives).
    knx_productdb::ingest_file(&conn, "M-1/Catalog.xml", CATALOG.as_bytes()).unwrap();
    (dir, conn)
}

/// `AppState::default()` runs the real `default_path()` lookup — force
/// `product_db` explicitly rather than let a locally installed database
/// leak into this test (`http_product_language.rs`'s own
/// `state_without_product_db` carries this exact reasoning).
#[allow(clippy::field_reassign_with_default)]
fn state_with_products(products: knx_productdb::Connection) -> knx_server::AppState {
    let mut state = knx_server::AppState::default();
    state.product_db = Some(std::sync::Mutex::new(products));
    state
}

async fn body_json(response: axum::response::Response) -> Value {
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

async fn get(app: axum::Router, uri: &str) -> (StatusCode, Value) {
    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(uri)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    (status, body_json(response).await)
}

// Catches a route/handler that forgets to read `q.language` at all, or one
// that reads it but never threads it down to `query::catalog_items`.
#[tokio::test]
async fn catalog_items_with_a_language_returns_the_translated_name() {
    let (_dir, products) = temp_product_db();
    let state = Arc::new(state_with_products(products));
    let app = knx_server::app(state, None);

    let (status, body) = get(app, "/api/catalog/items?language=de-DE").await;
    assert_eq!(status, StatusCode::OK);
    let items = body.as_array().unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["name"], "Umschaltaktor");
}

// The companion of the previous test: an absent `language` query parameter
// must still behave exactly as before this task — the package's own name,
// untouched.
#[tokio::test]
async fn catalog_items_without_a_language_returns_the_stored_name() {
    let (_dir, products) = temp_product_db();
    let state = Arc::new(state_with_products(products));
    let app = knx_server::app(state, None);

    let (status, body) = get(app, "/api/catalog/items").await;
    assert_eq!(status, StatusCode::OK);
    let items = body.as_array().unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["name"], "Schaltaktor");
}
