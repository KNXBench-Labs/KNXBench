//! T16: `domain::device_detail`'s product/hardware resolution overlay.
//
// Fixture idiom follows `http_product_language.rs`'s own `temp_product_db`
// (read there first) — a `Hardware.xml` giving one `Product`/
// `Hardware2Program` pair, plus an `ApplicationProgram.xml` for the program
// side of the chain.

use std::sync::{Arc, Mutex};

use axum::body::Body;
use axum::http::{Request, StatusCode};
use knx_core::{CommissioningState, DeviceId, DeviceInstance, Language, Project, SourceRef};
use serde_json::Value;
use tower::ServiceExt;

const HARDWARE: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<KNX xmlns="http://knx.org/xml/project/11"><ManufacturerData><Manufacturer RefId="M-1">
<Hardware><Hardware Id="H-1" Name="X" SerialNumber="S" VersionNumber="1">
<Products><Product Id="M-1_P-1" Text="Switch Actuator" OrderNumber="ORD-1" /></Products>
<Hardware2Programs><Hardware2Program Id="H-1_HP-1" MediumTypes="MT-0">
<ApplicationProgramRef RefId="A-1" /></Hardware2Program></Hardware2Programs>
</Hardware></Hardware></Manufacturer></ManufacturerData></KNX>"#;

const PROGRAM: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<KNX xmlns="http://knx.org/xml/project/11"><ManufacturerData><Manufacturer RefId="M-1">
<ApplicationPrograms><ApplicationProgram Id="A-1" Name="P" ApplicationNumber="1" ApplicationVersion="1" MaskVersion="MV-0701">
<Static><ParameterTypes /><Parameters /><ParameterRefs /></Static>
<Dynamic /></ApplicationProgram></ApplicationPrograms></Manufacturer></ManufacturerData></KNX>"#;

fn temp_product_db() -> (tempfile::TempDir, knx_productdb::Connection) {
    let dir = tempfile::tempdir().unwrap();
    let conn = knx_productdb::open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
    knx_productdb::ingest_file(&conn, "M-1/Hardware.xml", HARDWARE.as_bytes()).unwrap();
    knx_productdb::ingest_file(&conn, "M-1/A.xml", PROGRAM.as_bytes()).unwrap();
    (dir, conn)
}

/// One device, `DeviceId(1)`, with the given `product_ref`/`program_ref`.
#[allow(clippy::field_reassign_with_default)]
fn state_with_device(
    products: Option<knx_productdb::Connection>,
    product_ref: &str,
    program_ref: &str,
) -> knx_server::AppState {
    let mut project = Project::new(Language("en".into()));
    project.devices.insert(DeviceInstance {
        id: DeviceId(1),
        source: SourceRef {
            path: "device-1".into(),
            ets_id: "device-1".into(),
        },
        name: "Device 1".into(),
        description: None,
        address: None,
        product_ref: product_ref.into(),
        program_ref: program_ref.into(),
        commissioning: CommissioningState::default(),
        visibility_calculated: true,
        com_objects: vec![],
        binary_data: vec![],
    });
    let mut state = knx_server::AppState::default();
    state.product_db = products.map(Mutex::new);
    *state.project.lock().unwrap() = Some(project);
    state
}

async fn body_json(response: axum::response::Response) -> Value {
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

/// Global Constraint (T16): the pure projection's `resolution` placeholder
/// (`"NoDatabase"`, see `knx_projection::DeviceProductNode`'s doc comment)
/// must never reach the wire once a real database is loaded and the refs
/// resolve — the server overlay is obliged to overwrite it.
#[tokio::test]
async fn device_product_resolution_always_overwrites_the_projections_placeholder() {
    let (_dir, products) = temp_product_db();
    let state = Arc::new(state_with_device(Some(products), "M-1_P-1", "H-1_HP-1"));
    let app = knx_server::app(state, None);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/device/1")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = body_json(response).await;
    let product = &body["product"];
    assert_eq!(product["resolution"], "Resolved");
    assert_ne!(product["resolution"], "NoDatabase");
    assert_eq!(product["catalog"]["product_text"], "Switch Actuator");
    assert_eq!(product["catalog"]["order_number"], "ORD-1");
    assert_eq!(product["catalog"]["application_name"], "P");
}

/// No product database configured at all: refs are present, so the pure
/// builder's `NoDatabase` placeholder is, this once, the honest answer —
/// but it must come from the server actually checking, not merely surviving
/// unchanged. `state_without_product_db` mirrors `http_product_language.rs`'s
/// own reasoning for forcing `None` explicitly rather than relying on
/// `AppState::default()`'s environment-dependent lookup.
#[tokio::test]
async fn device_product_with_no_database_loaded_is_no_database() {
    let state = Arc::new(state_with_device(None, "M-1_P-1", "H-1_HP-1"));
    let app = knx_server::app(state, None);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/device/1")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = body_json(response).await;
    let product = &body["product"];
    assert_eq!(product["resolution"], "NoDatabase");
    assert!(product["catalog"].is_null());
}

/// A database is loaded, but this device's refs are not in it — the
/// manufacturer's catalogue is simply not installed here.
#[tokio::test]
async fn device_product_with_unmatched_refs_is_not_in_database() {
    let (_dir, products) = temp_product_db();
    let state = Arc::new(state_with_device(
        Some(products),
        "M-1_P-does-not-exist",
        "H-1_HP-1",
    ));
    let app = knx_server::app(state, None);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/device/1")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = body_json(response).await;
    let product = &body["product"];
    assert_eq!(product["resolution"], "NotInDatabase");
    assert!(product["catalog"].is_null());
}

/// A device with neither ref stated needs no product-database lookup at
/// all — `NoReference` is final at the pure-projection layer already, and
/// the overlay must leave it alone even with a database loaded.
#[tokio::test]
async fn device_product_with_no_stated_ref_stays_no_reference_even_with_a_database_loaded() {
    let (_dir, products) = temp_product_db();
    let state = Arc::new(state_with_device(Some(products), "", ""));
    let app = knx_server::app(state, None);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/device/1")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = body_json(response).await;
    let product = &body["product"];
    assert_eq!(product["resolution"], "NoReference");
    assert!(product["catalog"].is_null());
    assert!(product["product_ref"].is_null());
    assert!(product["program_ref"].is_null());
}

/// Resolution must not depend on `?language=` — it's a database-membership
/// fact, not a translation. Same fixture as the first test, no query
/// parameter needed since `/api/device/{id}` already omits one, but this
/// pins that omission is exactly what makes the untranslated leg run too.
#[tokio::test]
async fn device_product_resolves_without_a_language_query_parameter() {
    let (_dir, products) = temp_product_db();
    let state = Arc::new(state_with_device(Some(products), "M-1_P-1", "H-1_HP-1"));
    let app = knx_server::app(state, None);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/device/1")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let body = body_json(response).await;
    assert_eq!(body["product"]["resolution"], "Resolved");
}
