//! Verifies exact optional editor evidence through real HTTP and native parameter writes.

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use knx_core::{
    CommissioningState, CompletionStatus, DeviceId, DeviceInstance, Installation, InstallationId,
    Language, Project, SourceRef, Topology,
};
use serde_json::{json, Value};
use std::sync::Arc;
use tower::ServiceExt;

const HARDWARE: &str = r#"<KNX xmlns="http://knx.org/xml/project/20"><ManufacturerData><Manufacturer RefId="M-0001"><Hardware><Hardware Id="H-1" Name="Synthetic" SerialNumber="S" VersionNumber="1"><Products><Product Id="M-0001_P-1"/></Products><Hardware2Programs><Hardware2Program Id="HP-1" MediumTypes="MT-0"><ApplicationProgramRef RefId="A-1"/></Hardware2Program></Hardware2Programs></Hardware></Hardware></Manufacturer></ManufacturerData></KNX>"#;
const PROGRAM: &str = r#"<KNX xmlns="http://knx.org/xml/project/20"><ManufacturerData><Manufacturer RefId="M-0001"><ApplicationPrograms><ApplicationProgram Id="A-1" Name="Synthetic" ApplicationVersion="1" MaskVersion="MV-0701"><Static><ParameterTypes><ParameterType Id="PT" Name="Time"><TypeTime SizeInBit="16" minInclusive="0" maxInclusive="3600" Unit="Seconds"/></ParameterType></ParameterTypes><Parameters><Parameter Id="P" Name="Delay" Text="Delay" Access="ReadWrite" ParameterType="PT" Value="0010"/></Parameters><ParameterRefs><ParameterRef Id="PR" RefId="P"/></ParameterRefs></Static><Dynamic><ParameterRefRef RefId="PR"/></Dynamic></ApplicationProgram></ApplicationPrograms></Manufacturer></ManufacturerData></KNX>"#;
fn fixture() -> (tempfile::TempDir, Arc<knx_server::AppState>, axum::Router) {
    let dir = tempfile::tempdir().unwrap();
    let products = knx_productdb::open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
    super::native_source_fixture::install(
        &products,
        20,
        ("M-0001/Hardware.xml", HARDWARE.as_bytes()),
        ("M-0001/Application.xml", PROGRAM.as_bytes()),
    );
    let state = knx_server::AppState {
        product_db: Some(
            knx_server::CatalogueOwner::adopt_opened(products, std::path::PathBuf::new()).unwrap(),
        ),
        ..knx_server::AppState::new(dir.path().into())
    };
    let mut project = Project::new(Language("en".into()));
    project.devices.insert(DeviceInstance {
        id: DeviceId(1),
        source: SourceRef {
            path: "synthetic-device".into(),
            ets_id: "synthetic-device".into(),
        },
        name: "Synthetic".into(),
        description: None,
        address: None,
        product_ref: "M-0001_P-1".into(),
        program_ref: "HP-1".into(),
        commissioning: CommissioningState::default(),
        visibility_calculated: true,
        com_objects: vec![],
        binary_data: vec![],
    });
    project.installations.push(Installation {
        id: InstallationId(0),
        name: "Synthetic".into(),
        default_line: None,
        multicast_address: None,
        completion: CompletionStatus::FinishedDesign,
        topology: Topology {
            areas: vec![],
            lines: vec![],
            unassigned: vec![DeviceId(1)],
        },
        buildings: vec![],
        group_ranges: vec![],
        group_addresses: vec![],
        parameters: vec![],
    });
    project.ids = knx_core::project::IdAllocators::from_counts(1, 0, 0, 0, 0, 0, 0, 0, 0);
    *state.project.lock().unwrap() = Some(project);
    let state = Arc::new(state);
    let app = knx_server::app(state.clone(), None);
    (dir, state, app)
}
async fn request(
    app: &axum::Router,
    method: &str,
    path: &str,
    payload: Value,
) -> (StatusCode, Value) {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(method)
                .uri(path)
                .header("content-type", "application/json")
                .body(Body::from(payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), 4 * 1024 * 1024)
        .await
        .unwrap();
    (
        status,
        if bytes.is_empty() {
            Value::Null
        } else {
            serde_json::from_slice(&bytes).unwrap_or_else(|_| json!({"nonJsonBody":true}))
        },
    )
}
#[tokio::test]
async fn exact_device_program_source_is_read_only_and_not_a_write_permit() {
    let (_dir, state, app) = fixture();
    let before = state.project.lock().unwrap().clone();
    let changes = state
        .product_db
        .as_ref()
        .unwrap()
        .with_read(|conn| Ok::<_, std::convert::Infallible>(conn.total_changes()))
        .unwrap()
        .unwrap();
    let (status, view) = request(
        &app,
        "GET",
        "/api/device/1/parameter-editor-source?etsId=PR",
        Value::Null,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(view["device"], 1);
    assert_eq!(view["parameterRefId"], "PR");
    assert_eq!(view["evidence"]["status"], "declared");
    assert_eq!(view["evidence"]["source"]["programId"], "A-1");
    assert_eq!(view["evidence"]["source"]["attributes"]["Unit"], "Seconds");
    assert_eq!(
        view["evidence"]["source"]["sourceSha256"],
        knx_productdb::sha256_hex(PROGRAM.as_bytes())
    );
    assert!(view["serverIncarnation"]
        .as_str()
        .is_some_and(|s| !s.is_empty()));
    assert!(view["snapshotRevision"].is_number());
    assert!(view.get("editable").is_none());
    assert_eq!(*state.project.lock().unwrap(), before);
    assert_eq!(
        state
            .product_db
            .as_ref()
            .unwrap()
            .with_read(|conn| Ok::<_, std::convert::Infallible>(conn.total_changes()))
            .unwrap()
            .unwrap(),
        changes
    );
}

#[tokio::test]
async fn missing_metadata_is_named_and_database_failure_is_not_hidden() {
    let (_dir, state, app) = fixture();
    let (status, view) = request(
        &app,
        "GET",
        "/api/device/1/parameter-editor-source?etsId=missing",
        Value::Null,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        view["evidence"],
        json!({"status":"unavailable","reason":"missingReference"})
    );
    state
        .product_db
        .as_ref()
        .unwrap()
        .with_read(|conn| {
            conn.execute_batch("DROP TABLE parameter_ref").unwrap();
            Ok::<(), std::convert::Infallible>(())
        })
        .unwrap()
        .unwrap();
    assert_eq!(
        request(
            &app,
            "GET",
            "/api/device/1/parameter-editor-source?etsId=PR",
            Value::Null
        )
        .await
        .0,
        StatusCode::INTERNAL_SERVER_ERROR
    );
}
#[tokio::test]
async fn source_route_cannot_select_another_program_or_mutate_values() {
    let (_dir, state, app) = fixture();
    let before = state.project.lock().unwrap().clone();
    for path in [
        "/api/device/1/parameter-editor-source?etsId=PR&programId=other",
        "/api/device/1/parameter-editor-source?etsId=",
        "/api/device/99/parameter-editor-source?etsId=PR",
    ] {
        assert_eq!(
            request(&app, "GET", path, Value::Null).await.0,
            StatusCode::BAD_REQUEST
        );
    }
    assert_eq!(
        request(
            &app,
            "POST",
            "/api/device/1/parameter-editor-source?etsId=PR",
            json!({"raw":"20"})
        )
        .await
        .0,
        StatusCode::METHOD_NOT_ALLOWED
    );
    assert_eq!(*state.project.lock().unwrap(), before);
}
#[tokio::test]
async fn source_route_remains_inside_existing_authentication() {
    let (_dir, state, _app) = fixture();
    let auth = knx_server::AuthConfig::from_plaintext_password("synthetic-test-only").unwrap();
    let app = knx_server::app_with_auth(state, None, auth);
    assert_eq!(
        request(
            &app,
            "GET",
            "/api/device/1/parameter-editor-source?etsId=PR",
            Value::Null
        )
        .await
        .0,
        StatusCode::UNAUTHORIZED
    );
}
#[tokio::test]
async fn existing_parameter_validation_undo_and_native_reopen_preserve_exact_values() {
    let (dir, state, app) = fixture();
    let native = dir.path().join("synthetic.knxdb");
    assert_eq!(
        request(&app, "POST", "/api/project/save-as", json!({"path":native}))
            .await
            .0,
        StatusCode::OK
    );
    super::native_source_fixture::assert_carried(&state, &native);
    let source = request(
        &app,
        "GET",
        "/api/device/1/parameter-editor-source?etsId=PR",
        Value::Null,
    )
    .await
    .1;
    let before = state.project.lock().unwrap().clone();
    let bytes = std::fs::read(&native).unwrap();
    let history = request(&app, "GET", "/api/project/history", Value::Null)
        .await
        .1;
    assert_eq!(
        request(
            &app,
            "POST",
            "/api/device/1/parameters",
            json!({"etsId":"PR","raw":"3601"})
        )
        .await
        .0,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(*state.project.lock().unwrap(), before);
    assert_eq!(std::fs::read(&native).unwrap(), bytes);
    assert_eq!(
        request(&app, "GET", "/api/project/history", Value::Null)
            .await
            .1,
        history
    );
    let (status, panel) = request(
        &app,
        "POST",
        "/api/device/1/parameters",
        json!({"etsId":"PR","raw":"+0012"}),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(panel["sections"][0]["fields"][0]["value"], "+0012");
    assert!(panel["tree"].is_object());
    let history = request(&app, "GET", "/api/project/history", Value::Null)
        .await
        .1;
    assert_eq!(history["undoSteps"], 1);
    assert_eq!(
        request(&app, "POST", "/api/undo", json!({})).await.0,
        StatusCode::OK
    );
    assert_eq!(
        request(&app, "GET", "/api/device/1/parameters", Value::Null)
            .await
            .1["sections"][0]["fields"][0]["value"],
        "0010"
    );
    assert_eq!(
        request(&app, "POST", "/api/redo", json!({})).await.0,
        StatusCode::OK
    );
    assert_eq!(
        request(&app, "POST", "/api/project/save", json!({}))
            .await
            .0,
        StatusCode::OK
    );
    let saved = std::fs::read(&native).unwrap();
    assert_eq!(
        request(
            &app,
            "POST",
            "/api/project/open",
            json!({"path":native,"clientToken":"synthetic-reopen"})
        )
        .await
        .0,
        StatusCode::OK
    );
    assert_eq!(
        request(&app, "GET", "/api/device/1/parameters", Value::Null)
            .await
            .1["sections"][0]["fields"][0]["value"],
        "+0012"
    );
    assert_eq!(std::fs::read(&native).unwrap(), saved);
    let reopened = request(
        &app,
        "GET",
        "/api/device/1/parameter-editor-source?etsId=PR",
        Value::Null,
    )
    .await
    .1;
    assert_eq!(reopened["evidence"], source["evidence"]);
    assert_eq!(
        request(&app, "GET", "/api/project/history", Value::Null)
            .await
            .1["undoSteps"],
        history["undoSteps"]
    );
    state
        .product_db
        .as_ref()
        .unwrap()
        .with_read(|conn| {
            assert_eq!(
                knx_productdb::load_source_file(
                    conn,
                    &knx_productdb::sha256_hex(PROGRAM.as_bytes())
                )
                .unwrap()
                .unwrap(),
                PROGRAM.as_bytes()
            );
            Ok::<(), std::convert::Infallible>(())
        })
        .unwrap()
        .unwrap();
}
