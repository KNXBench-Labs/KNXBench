//! ADR-0173 over HTTP: the parameter response's last-download comparison and the manual mark.

use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use knx_core::commissioning::parameter_baseline::{BaselineEdit, ParameterBaseline};
use knx_core::{
    CommissioningState, CompletionStatus, DeviceId, DeviceInstance, Installation, InstallationId,
    Language, ParameterInstance, ParameterInstanceId, Project, SourceRef, Topology,
};
use serde_json::{json, Value};
use tower::ServiceExt;

const HARDWARE: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<KNX xmlns="http://knx.org/xml/project/11"><ManufacturerData><Manufacturer RefId="M-1">
<Hardware><Hardware Id="H-1" Name="X" SerialNumber="S" VersionNumber="1">
<Products><Product Id="M-1_P-1" /></Products>
<Hardware2Programs><Hardware2Program Id="H-1_HP-1" MediumTypes="MT-0">
<ApplicationProgramRef RefId="A-1" /></Hardware2Program></Hardware2Programs>
</Hardware></Hardware></Manufacturer></ManufacturerData></KNX>"#;

/// `P-1_R-1` defaults to 5, `P-2_R-1` to 0.
const PROGRAM: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<KNX xmlns="http://knx.org/xml/project/11"><ManufacturerData><Manufacturer RefId="M-1">
<ApplicationPrograms><ApplicationProgram Id="A-1" Name="P" ApplicationVersion="1" MaskVersion="MV-0701">
<Static>
<ParameterTypes>
  <ParameterType Id="PT-Num" Name="num"><TypeNumber maxInclusive="255" minInclusive="0" SizeInBit="8" Type="unsignedInt" /></ParameterType>
</ParameterTypes>
<Parameters>
  <Parameter Id="P-1" Name="Delay" Text="Delay" ParameterType="PT-Num" Access="ReadWrite" Value="5" />
  <Parameter Id="P-2" Name="Mode" Text="Mode" ParameterType="PT-Num" Access="ReadWrite" Value="0" />
</Parameters>
<ParameterRefs>
  <ParameterRef Id="P-1_R-1" RefId="P-1" />
  <ParameterRef Id="P-2_R-1" RefId="P-2" />
</ParameterRefs>
</Static>
<Dynamic>
  <ParameterRefRef RefId="P-1_R-1" />
  <ParameterRefRef RefId="P-2_R-1" />
</Dynamic>
</ApplicationProgram></ApplicationPrograms></Manufacturer></ManufacturerData></KNX>"#;

fn products(dir: &std::path::Path) -> knx_server::CatalogueOwner {
    let path = dir.join("products.sqlite");
    let conn = knx_productdb::open_and_migrate(&path).unwrap();
    knx_productdb::ingest_file(&conn, "M-1/Hardware.xml", HARDWARE.as_bytes()).unwrap();
    knx_productdb::ingest_file(&conn, "M-1/A.xml", PROGRAM.as_bytes()).unwrap();
    knx_server::CatalogueOwner::adopt_opened(conn, path).unwrap()
}

fn project(parameters: &[(&str, &str)]) -> Project {
    let mut project = Project::new(Language("en".into()));
    project.info.project_id = "P-0001".into();
    project.devices.insert(DeviceInstance {
        id: DeviceId(1),
        source: SourceRef {
            path: "device-1".into(),
            ets_id: "device-1".into(),
        },
        name: "Device 1".into(),
        description: None,
        address: Some("1.1.1".parse().unwrap()),
        product_ref: "M-1_P-1".into(),
        program_ref: "H-1_HP-1".into(),
        commissioning: CommissioningState::default(),
        visibility_calculated: true,
        com_objects: vec![],
        binary_data: vec![],
    });
    project.installations.push(Installation {
        id: InstallationId(0),
        name: "I".into(),
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
        parameters: parameters
            .iter()
            .enumerate()
            .map(|(i, (ets_id, raw))| ParameterInstance {
                id: ParameterInstanceId(i as u32 + 1),
                device: DeviceId(1),
                source: SourceRef {
                    path: "device-1".into(),
                    ets_id: (*ets_id).into(),
                },
                raw: (*raw).into(),
            })
            .collect(),
    });
    project.ids = knx_core::project::IdAllocators::from_counts(
        1,
        0,
        0,
        0,
        0,
        0,
        0,
        parameters.len() as u32,
        0,
    );
    project
}

fn state(
    products: Option<knx_server::CatalogueOwner>,
    parameters: &[(&str, &str)],
) -> Arc<knx_server::AppState> {
    let state = knx_server::AppState {
        product_db: products,
        ..knx_server::AppState::default()
    };
    let project = project(parameters);
    *state.clean_project.lock().unwrap() = Some(project.clone());
    *state.project.lock().unwrap() = Some(project);
    Arc::new(state)
}

fn seed(state: &knx_server::AppState, program_id: &str, values: &[(&str, &str)]) {
    state
        .parameter_baselines
        .lock()
        .unwrap()
        .apply(&BaselineEdit {
            device: DeviceId(1),
            baseline: Some(ParameterBaseline {
                program_id: program_id.into(),
                values: values
                    .iter()
                    .map(|(k, v)| (k.to_string(), v.to_string()))
                    .collect(),
                at: chrono::DateTime::parse_from_rfc3339("2026-11-11T08:00:00.5Z")
                    .unwrap()
                    .with_timezone(&chrono::Utc),
            }),
        });
}

fn has_baseline(state: &knx_server::AppState) -> bool {
    state
        .parameter_baselines
        .lock()
        .unwrap()
        .get(DeviceId(1))
        .is_some()
}

async fn call(
    state: &Arc<knx_server::AppState>,
    method: &str,
    uri: &str,
    body: Option<Value>,
) -> (StatusCode, Value) {
    let mut request = Request::builder().method(method).uri(uri);
    let body = match body {
        Some(body) => {
            request = request.header("content-type", "application/json");
            Body::from(body.to_string())
        }
        None => Body::empty(),
    };
    let response = knx_server::app(state.clone(), None)
        .oneshot(request.body(body).unwrap())
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

async fn last_download(state: &Arc<knx_server::AppState>) -> Value {
    let (status, panel) = call(state, "GET", "/api/device/1/parameters", None).await;
    assert_eq!(status, StatusCode::OK, "{panel}");
    panel["lastDownload"].clone()
}

#[tokio::test]
async fn a_device_without_a_baseline_reports_absent_and_nothing_else() {
    let dir = tempfile::tempdir().unwrap();
    let state = state(Some(products(dir.path())), &[("P-1_R-1", "9")]);
    assert_eq!(last_download(&state).await, json!({ "state": "absent" }));
}

#[tokio::test]
async fn a_usable_baseline_names_every_differing_effective_value() {
    let dir = tempfile::tempdir().unwrap();
    // Current: P-1 chosen 9, P-2 chosen explicitly at its default 0, and a
    // stored key the program does not declare.
    let state = state(
        Some(products(dir.path())),
        &[("P-1_R-1", "9"), ("P-2_R-1", "0"), ("A-1_GHOST", "x")],
    );
    // Downloaded: P-1 at 5, P-2 at 0 and a key that is gone now.
    seed(
        &state,
        "A-1",
        &[("P-1_R-1", "5"), ("P-2_R-1", "0"), ("A-1_OLD", "1")],
    );
    assert_eq!(
        last_download(&state).await,
        json!({
            "state": "usable",
            "at": "2026-11-11T08:00:00.500Z",
            "differing": [
                { "refId": "A-1_GHOST", "downloaded": null, "current": "x" },
                { "refId": "A-1_OLD", "downloaded": "1", "current": null },
                { "refId": "P-1_R-1", "downloaded": "5", "current": "9" },
            ],
        })
    );
}

#[tokio::test]
async fn a_value_set_to_its_default_is_no_difference() {
    let dir = tempfile::tempdir().unwrap();
    let state = state(Some(products(dir.path())), &[("P-1_R-1", "5")]);
    seed(&state, "A-1", &[("P-1_R-1", "5"), ("P-2_R-1", "0")]);
    assert_eq!(
        last_download(&state).await,
        json!({ "state": "usable", "at": "2026-11-11T08:00:00.500Z", "differing": [] })
    );
}

#[tokio::test]
async fn another_program_missing_products_and_conflicting_values_are_not_compared() {
    let dir = tempfile::tempdir().unwrap();
    let other = state(Some(products(dir.path())), &[]);
    seed(&other, "A-2", &[("P-1_R-1", "5")]);
    assert_eq!(
        last_download(&other).await,
        json!({ "state": "otherProgram" })
    );

    let without = state(None, &[]);
    seed(&without, "A-1", &[("P-1_R-1", "5")]);
    assert_eq!(
        last_download(&without).await,
        json!({ "state": "productUnavailable" })
    );

    let dir = tempfile::tempdir().unwrap();
    let conflicting = state(
        Some(products(dir.path())),
        &[("P-1_R-1", "5"), ("P-1_R-1", "6")],
    );
    seed(&conflicting, "A-1", &[("P-1_R-1", "5")]);
    assert_eq!(
        last_download(&conflicting).await,
        json!({ "state": "currentUnreadable" })
    );
}

async fn tree(state: &Arc<knx_server::AppState>) -> Value {
    let (status, tree) = call(state, "GET", "/api/project", None).await;
    assert_eq!(status, StatusCode::OK, "{tree}");
    tree
}

fn mark(tree: &Value, programmed: bool) -> Value {
    json!({
        "programmed": programmed,
        "serverIncarnation": tree["server_incarnation"],
        "projectIncarnation": tree["project_incarnation"],
        "snapshotRevision": tree["snapshot_revision"],
    })
}

#[tokio::test]
async fn a_manual_mark_in_either_direction_forgets_the_baseline_durably_and_undo_keeps_it_gone() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("marks.knxdb");
    // No product database: the mark needs none, and a save without one
    // carries no catalogue sources.
    let state = state(None, &[]);
    let (status, saved) = call(
        &state,
        "POST",
        "/api/project/save-as",
        Some(json!({ "path": path })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{saved}");

    for programmed in [true, false] {
        seed(&state, "A-1", &[("P-1_R-1", "5")]);
        // A stale mark runs no command, so it forgets nothing either.
        let mut stale = mark(&tree(&state).await, programmed);
        stale["snapshotRevision"] = json!(9_999);
        let (status, _) = call(&state, "POST", "/api/device/1/programming", Some(stale)).await;
        assert_eq!(status, StatusCode::CONFLICT);
        assert!(has_baseline(&state), "a refused mark keeps the baseline");

        let shown = tree(&state).await;
        let (status, marked) = call(
            &state,
            "POST",
            "/api/device/1/programming",
            Some(mark(&shown, programmed)),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{marked}");
        assert!(!has_baseline(&state), "mark {programmed} forgot it");
        assert_eq!(last_download(&state).await, json!({ "state": "absent" }));

        // Undoing the mark restores the flags, never the baseline.
        let (status, undone) = call(&state, "POST", "/api/undo", None).await;
        assert_eq!(status, StatusCode::OK, "{undone}");
        assert!(!has_baseline(&state));
    }

    // Written with the mark's journal entry: a restarted server has none.
    seed(&state, "A-1", &[("P-1_R-1", "5")]);
    let (status, saved) = call(&state, "POST", "/api/project/save", Some(json!({}))).await;
    assert_eq!(status, StatusCode::OK, "{saved}");
    let restarted = Arc::new(knx_server::AppState::default());
    let (status, opened) = call(
        &restarted,
        "POST",
        "/api/project/open",
        Some(json!({ "path": path })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{opened}");
    assert!(has_baseline(&restarted), "a saved baseline reopens");
    let shown = tree(&restarted).await;
    let (status, marked) = call(
        &restarted,
        "POST",
        "/api/device/1/programming",
        Some(mark(&shown, true)),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{marked}");
    drop(restarted);
    let again = Arc::new(knx_server::AppState::default());
    let (status, opened) = call(
        &again,
        "POST",
        "/api/project/open",
        Some(json!({ "path": path })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{opened}");
    assert!(!has_baseline(&again), "the removal reached the file");
}
