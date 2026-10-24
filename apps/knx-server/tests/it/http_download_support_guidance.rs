//! Verifies pre-plan support refusals without a connection or a report.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use knx_core::{DeviceId, DeviceInstance, Language, Project, SourceRef};
use serde_json::{json, Value};
use std::sync::Arc;
use tower::ServiceExt;

fn device(program: &str) -> DeviceInstance {
    DeviceInstance {
        id: DeviceId(1),
        source: SourceRef {
            path: "test".into(),
            ets_id: "D-1".into(),
        },
        name: "PRIVATE device".into(),
        description: None,
        address: Some("1.1.10".parse().unwrap()),
        product_ref: String::new(),
        program_ref: program.into(),
        commissioning: Default::default(),
        visibility_calculated: true,
        com_objects: vec![],
        binary_data: vec![],
    }
}

async fn post(app: axum::Router, uri: &str, body: Value) -> (StatusCode, Value) {
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(uri)
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
    (status, serde_json::from_slice(&bytes).unwrap())
}

#[tokio::test]
async fn configuration_and_missing_application_have_distinct_codes_before_any_session() {
    for (program, category) in [
        ("", "configuration"),
        ("M-0001_A-0001-01-0000", "product-data"),
    ] {
        let dir = tempfile::tempdir().unwrap();
        #[allow(clippy::field_reassign_with_default)]
        let state = {
            let mut state = knx_server::AppState::default();
            state.product_db = Some(
                knx_server::CatalogueOwner::adopt_opened(
                    knx_productdb::open_and_migrate(&dir.path().join("p.sqlite")).unwrap(),
                    std::path::PathBuf::new(),
                )
                .unwrap(),
            );
            let mut project = Project::new(Language("en".into()));
            project.devices.insert(device(program));
            *state.project.lock().unwrap() = Some(project);
            Arc::new(state)
        };
        let app = knx_server::app(state.clone(), None);
        let (status, body) = post(
            app.clone(),
            "/api/device-download/plan",
            json!({"address":"1.1.10"}),
        )
        .await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(body["kind"], "downloadPreparationRefused");
        assert_eq!(body["category"], category);
        assert!(body["error"]
            .as_str()
            .unwrap()
            .contains("no download to device 1.1.10 prepared"));
        let (status, _) = post(app, "/api/device-download/start", json!({"planId":0,"gateway":"192.0.2.10:3671","confirmation":"I confirm download to 1.1.10","acceptUntested":"I accept untested download to 1.1.10"})).await;
        assert!(!status.is_success());
    }
}

#[tokio::test]
async fn invalid_gateway_and_missing_plan_name_the_correction_without_a_tunnel() {
    let app = knx_server::app(Arc::new(knx_server::AppState::default()), None);
    for (gateway, status, kind) in [
        ("invalid", StatusCode::BAD_REQUEST, "downloadGatewayInvalid"),
        ("192.0.2.10:3671", StatusCode::CONFLICT, "downloadPlanStale"),
    ] {
        let (actual, body) = post(
            app.clone(),
            "/api/device-download/start",
            json!({"planId":0,"gateway":gateway,"confirmation":"I confirm download to 1.1.10"}),
        )
        .await;
        assert_eq!(actual, status);
        assert_eq!(body["kind"], kind);
    }
}

#[tokio::test]
async fn missing_database_is_not_an_unsupported_device() {
    let state = Arc::new(knx_server::AppState::default());
    let app = knx_server::app(state, None);
    let (status, body) = post(
        app,
        "/api/device-download/plan",
        json!({"address":"1.1.10"}),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(body["kind"], "downloadProductDataMissing");
}

async fn get(app: axum::Router, uri: &str) -> (StatusCode, Value) {
    let response = app
        .oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    (status, serde_json::from_slice(&bytes).unwrap())
}

/// The synthetic KL-006 witness of `knx_testsupport`, with or without its
/// `EtsDownloadPlugin` declaration, behind a server.
#[allow(clippy::field_reassign_with_default)]
fn plugin_app(dir: &std::path::Path, plugin: bool) -> axum::Router {
    use knx_testsupport::vendor_plugin as fixture;
    let products = knx_productdb::open_and_migrate(&dir.join("p.sqlite")).unwrap();
    let extension = if plugin {
        fixture::declaration()
    } else {
        String::new()
    };
    knx_productdb::ingest_file(
        &products,
        "M-FFF6/A.xml",
        fixture::program_xml(&extension).as_bytes(),
    )
    .unwrap();
    knx_productdb::ingest_file(
        &products,
        "M-FFF6/Hardware.xml",
        fixture::HARDWARE_XML.as_bytes(),
    )
    .unwrap();
    let mut state = knx_server::AppState::default();
    state.product_db = Some(
        knx_server::CatalogueOwner::adopt_opened(products, std::path::PathBuf::new()).unwrap(),
    );
    let mut project = Project::new(Language("en".into()));
    project.devices.insert(DeviceInstance {
        address: Some(fixture::TARGET.parse().unwrap()),
        product_ref: fixture::PRODUCT_REF.into(),
        program_ref: fixture::HARDWARE2PROGRAM_REF.into(),
        ..device("")
    });
    *state.project.lock().unwrap() = Some(project);
    knx_server::app(Arc::new(state), None)
}

/// KL-006: plan refuses every scope, the group-address-only partial
/// included, with the shared category; readiness agrees.
#[tokio::test]
async fn a_declared_download_plugin_is_refused_for_every_scope_and_readiness_agrees() {
    use knx_testsupport::vendor_plugin as fixture;
    let dir = tempfile::tempdir().unwrap();
    let app = plugin_app(dir.path(), true);
    for partial in [
        None,
        Some(json!({"parameters": true, "groupAddresses": false})),
        Some(json!({"parameters": false, "groupAddresses": true})),
        Some(json!({"parameters": true, "groupAddresses": true})),
    ] {
        let mut request = json!({"address": fixture::TARGET});
        if let Some(partial) = &partial {
            request["partial"] = partial.clone();
        }
        let (status, body) = post(app.clone(), "/api/device-download/plan", request).await;
        assert_eq!(
            status,
            StatusCode::UNPROCESSABLE_ENTITY,
            "{partial:?}: {body}"
        );
        assert_eq!(body["kind"], "downloadPreparationRefused", "{body}");
        assert_eq!(body["category"], "vendor-download-plugin", "{body}");
        let error = body["error"].as_str().unwrap();
        assert!(
            error.contains("KL-006") && error.contains(fixture::PLUGIN_GUID),
            "{error}"
        );
    }
    let (status, body) = get(app, "/api/device-readiness").await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let row = &body["devices"][0];
    assert_eq!(row["readiness"], "unsupported", "{body}");
    assert_eq!(row["category"], "vendor-download-plugin", "{body}");
    assert!(row["detail"].as_str().unwrap().contains("KL-006"), "{body}");

    // Without the declaration the same program plans, and grades, as before.
    let dir = tempfile::tempdir().unwrap();
    let app = plugin_app(dir.path(), false);
    let (status, body) = post(
        app.clone(),
        "/api/device-download/plan",
        json!({"address": fixture::TARGET, "partial": {"parameters": false, "groupAddresses": true}}),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["support"]["level"], "untested", "{body}");
    let (_, body) = get(app, "/api/device-readiness").await;
    assert_eq!(body["devices"][0]["readiness"], "untested", "{body}");
}
