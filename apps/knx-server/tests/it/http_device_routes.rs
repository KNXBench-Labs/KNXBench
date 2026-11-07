//! Exercises mounted device routes and their persisted read-only object metadata.

use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use knx_core::{
    CommissioningState, CompletionStatus, DeviceId, DeviceInstance, Direction, GroupAddress,
    GroupAddressEntry, GroupAddressId, GroupLink, Installation, InstallationId, Language, Override,
    Project, ResolvedFlags, SourceRef, Topology,
};
use serde_json::{json, Value};
use tower::ServiceExt;

// The `product_db = None` line below is a deliberate, documented
// reassignment (see its own comment) — not an oversight clippy should
// fold into a `..Default::default()` struct literal. Same pattern
// `domain.rs`'s own `state_with_one_installation` test helper already
// carries this exact attribute for.
#[allow(clippy::field_reassign_with_default)]
fn state_with_one_installation() -> knx_server::AppState {
    let mut project = Project::new(Language("en".into()));
    project.installations.push(Installation {
        id: InstallationId(0),
        name: "I".into(),
        default_line: None,
        multicast_address: None,
        completion: CompletionStatus::FinishedDesign,
        topology: Topology {
            areas: vec![],
            lines: vec![],
            unassigned: vec![],
        },
        buildings: vec![],
        group_ranges: vec![],
        group_addresses: vec![],
        parameters: vec![],
    });
    let mut state = knx_server::AppState::default();
    // `AppState::default()` opens no product database (AR18 M8). Set
    // `None` explicitly anyway so `creating_a_device_without_a_product_
    // database_is_a_400` keeps its precondition if the default changes.
    state.product_db = None;
    *state.project.lock().unwrap() = Some(project);
    state
}

const HARDWARE: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<KNX xmlns="http://knx.org/xml/project/11"><ManufacturerData><Manufacturer RefId="M-1">
<Hardware><Hardware Id="H-1" Name="X" SerialNumber="S" VersionNumber="1">
<Products><Product Id="M-1_P-1" /></Products>
<Hardware2Programs><Hardware2Program Id="H-1_HP-1" MediumTypes="MT-0">
<ApplicationProgramRef RefId="A-1" /></Hardware2Program></Hardware2Programs>
</Hardware></Hardware></Manufacturer></ManufacturerData></KNX>"#;

const PROGRAM: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<KNX xmlns="http://knx.org/xml/project/11"><ManufacturerData><Manufacturer RefId="M-1">
<ApplicationPrograms><ApplicationProgram Id="A-1" Name="P" ApplicationVersion="1"
  MaskVersion="MV-0701"><Static>
<ComObjectTable>
  <ComObject Id="A-1_O-1" Number="1" Text="Schalten" ObjectSize="1 Bit"
             DatapointType="DPST-1-1" WriteFlag="Enabled" />
</ComObjectTable>
<ComObjectRefs>
  <ComObjectRef Id="A-1_O-1_R-1" RefId="A-1_O-1" />
</ComObjectRefs>
</Static></ApplicationProgram></ApplicationPrograms></Manufacturer></ManufacturerData></KNX>"#;

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
    </Manufacturer>
  </ManufacturerData>
</KNX>"#;

fn temp_product_db() -> (tempfile::TempDir, knx_productdb::Connection) {
    let dir = tempfile::tempdir().unwrap();
    let conn = knx_productdb::open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
    knx_productdb::parse::hardware::ingest_hardware(
        &conn,
        "sha-h",
        "M-1/Hardware.xml",
        HARDWARE.as_bytes(),
    )
    .unwrap();
    knx_productdb::parse::program::ingest_program(&conn, "sha-p", "M-1/A.xml", PROGRAM.as_bytes())
        .unwrap();
    knx_productdb::parse::catalog::ingest_catalog(
        &conn,
        "sha-c",
        "M-1/Catalog.xml",
        CATALOG.as_bytes(),
    )
    .unwrap();
    (dir, conn)
}

async fn body_json(response: axum::response::Response) -> Value {
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

#[tokio::test]
async fn creating_a_device_from_a_catalog_item_seeds_its_com_objects_then_deletes() {
    let mut state = state_with_one_installation();
    let (_dir, products) = temp_product_db();
    state.product_db = Some(
        knx_server::CatalogueOwner::adopt_opened(products, std::path::PathBuf::new()).unwrap(),
    );
    let state = Arc::new(state);
    let app = knx_server::app(Arc::clone(&state), None);

    let create = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/devices")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({ "catalogItemId": "M-1_CI-1", "name": "Actuator 1" }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(create.status(), StatusCode::OK);
    let response = body_json(create).await;
    let tree = &response["tree"];
    assert_eq!(
        response["diagnostics"],
        json!([{
            "kind": "dynamicOrModuleNotEvaluated",
            "programId": "A-1",
            "detail": "Dynamic and module activation was not evaluated for A-1; only static product data was seeded."
        }])
    );
    let devices = tree["installations"][0]["unassigned"].as_array().unwrap();
    assert_eq!(devices.len(), 1);
    let device_id = devices[0]["id"].as_u64().unwrap();

    let detail = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/device/{device_id}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(detail.status(), StatusCode::OK);
    let detail_json = body_json(detail).await;
    assert_eq!(detail_json["com_objects"].as_array().unwrap().len(), 1);

    let delete = app
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/devices/{device_id}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(delete.status(), StatusCode::OK);
    let tree = body_json(delete).await;
    assert!(tree["installations"][0]["unassigned"]
        .as_array()
        .unwrap()
        .is_empty());
    // `do_command` always pushes an inverse (see `CommandStack::do_command`),
    // so the create and the delete both remain on the undo stack here — the
    // stack still has two undoable entries, it is not "nothing happened".
    assert!(state.command_stack.lock().unwrap().can_undo());
}

#[tokio::test]
async fn dangling_com_object_ref_is_rejected_before_the_command_or_undo_state_changes() {
    let mut state = state_with_one_installation();
    let (_dir, products) = temp_product_db();
    products
        .execute("DELETE FROM com_object WHERE id = 'A-1_O-1'", [])
        .unwrap();
    state.product_db = Some(
        knx_server::CatalogueOwner::adopt_opened(products, std::path::PathBuf::new()).unwrap(),
    );
    let state = Arc::new(state);
    let app = knx_server::app(Arc::clone(&state), None);

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/devices")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({ "catalogItemId": "M-1_CI-1", "name": "Broken actuator" }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert!(body_json(response).await["error"]
        .as_str()
        .unwrap()
        .contains("communication-object reference is missing: A-1_O-1_R-1"));
    assert!(state
        .project
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .installations[0]
        .topology
        .unassigned
        .is_empty());
    assert!(!state.command_stack.lock().unwrap().can_undo());
}

#[tokio::test]
async fn missing_application_program_is_rejected_before_the_command_or_undo_state_changes() {
    let mut state = state_with_one_installation();
    let (_dir, products) = temp_product_db();
    products
        .execute("DELETE FROM application_program WHERE id = 'A-1'", [])
        .unwrap();
    state.product_db = Some(
        knx_server::CatalogueOwner::adopt_opened(products, std::path::PathBuf::new()).unwrap(),
    );
    let state = Arc::new(state);
    let app = knx_server::app(Arc::clone(&state), None);

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/devices")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({ "catalogItemId": "M-1_CI-1", "name": "Broken actuator" }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert!(body_json(response).await["error"]
        .as_str()
        .unwrap()
        .contains("no installed application program: A-1"));
    assert!(state
        .project
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .installations[0]
        .topology
        .unassigned
        .is_empty());
    assert!(!state.command_stack.lock().unwrap().can_undo());
}

#[tokio::test]
async fn creating_a_device_without_a_product_database_is_a_400() {
    let state = state_with_one_installation();
    let app = knx_server::app(Arc::new(state), None);

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/devices")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({ "catalogItemId": "anything", "name": "D" }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn deleting_a_device_with_a_linked_com_object_is_a_400() {
    let mut project = Project::new(Language("en".into()));
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
        group_addresses: vec![GroupAddressEntry {
            id: GroupAddressId(1),
            source: SourceRef {
                path: "t".into(),
                ets_id: "t".into(),
            },
            name: "GA".into(),
            address: GroupAddress::from_raw(1),
            central: false,
            unfiltered: false,
            range: None,
            declared_dpt: Default::default(),
        }],
        parameters: vec![],
    });
    project.devices.insert(DeviceInstance {
        id: DeviceId(1),
        source: SourceRef {
            path: "t".into(),
            ets_id: "t".into(),
        },
        name: "D".into(),
        description: None,
        address: None,
        product_ref: "P".into(),
        program_ref: "H".into(),
        commissioning: CommissioningState::default(),
        visibility_calculated: true,
        com_objects: vec![knx_core::ComObjectInstanceId(1)],
        binary_data: vec![],
    });
    project
        .devices
        .insert_com_object(knx_core::ComObjectInstance {
            priority_edit: None,
            saved_instance_priority: Default::default(),
            id: knx_core::ComObjectInstanceId(1),
            source: SourceRef {
                path: "t".into(),
                ets_id: "t".into(),
            },
            device: DeviceId(1),
            number: 0,
            text: Override::Absent,
            description: Override::Absent,
            dpt: Override::Absent,
            flags: ResolvedFlags::none(),
            size: None,
            is_active: true,
            links: vec![GroupLink {
                ga: GroupAddressId(1),
                direction: Direction::Send,
            }],
            module_instance: None,
        });
    let state = knx_server::AppState::default();
    *state.project.lock().unwrap() = Some(project);
    let app = knx_server::app(Arc::new(state), None);

    let response = app
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri("/api/devices/1")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn catalog_routes_list_the_manufacturer_and_item_just_ingested() {
    let mut state = state_with_one_installation();
    let (_dir, products) = temp_product_db();
    state.product_db = Some(
        knx_server::CatalogueOwner::adopt_opened(products, std::path::PathBuf::new()).unwrap(),
    );
    let app = knx_server::app(Arc::new(state), None);

    let manufacturers = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/catalog/manufacturers")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(manufacturers.status(), StatusCode::OK);
    let manufacturers = body_json(manufacturers).await;
    assert_eq!(manufacturers.as_array().unwrap().len(), 1);
    assert_eq!(manufacturers[0]["id"], "M-1");

    let items = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/catalog/items?manufacturer=M-1&search=schalt")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(items.status(), StatusCode::OK);
    let items = body_json(items).await;
    assert_eq!(items.as_array().unwrap().len(), 1);
    assert_eq!(items[0]["id"], "M-1_CI-1");
}

#[tokio::test]
async fn catalog_item_with_a_missing_hardware_to_program_relation_is_rejected_without_creating_a_device(
) {
    let mut state = state_with_one_installation();
    let (_dir, products) = temp_product_db();
    products
        .execute("DELETE FROM hardware2program WHERE id = 'H-1_HP-1'", [])
        .unwrap();
    state.product_db = Some(
        knx_server::CatalogueOwner::adopt_opened(products, std::path::PathBuf::new()).unwrap(),
    );
    let state = Arc::new(state);
    let app = knx_server::app(Arc::clone(&state), None);

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/devices")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({ "catalogItemId": "M-1_CI-1", "name": "Broken actuator" }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert!(body_json(response).await["error"]
        .as_str()
        .unwrap()
        .contains("hardware-to-program"));

    assert!(state
        .project
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .installations[0]
        .topology
        .unassigned
        .is_empty());
}

#[tokio::test]
async fn genuinely_programless_product_succeeds_with_a_visible_diagnostic() {
    let mut state = state_with_one_installation();
    let (_dir, products) = temp_product_db();
    products
        .execute_batch(
            "UPDATE catalog_item SET hardware2program_ref_id = NULL WHERE id = 'M-1_CI-1';
             UPDATE hardware SET has_application_program = 0 WHERE id = 'H-1'",
        )
        .unwrap();
    state.product_db = Some(
        knx_server::CatalogueOwner::adopt_opened(products, std::path::PathBuf::new()).unwrap(),
    );
    let app = knx_server::app(Arc::new(state), None);

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/devices")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({ "catalogItemId": "M-1_CI-1", "name": "Passive device" }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let response = body_json(response).await;
    assert_eq!(
        response["tree"]["installations"][0]["unassigned"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert!(response["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .any(|diagnostic| { diagnostic["kind"] == "programlessProduct" }));
    assert_eq!(response["diagnostics"][0]["catalogItemId"], "M-1_CI-1");
}

#[tokio::test]
async fn ambiguous_dpt_enrichment_succeeds_and_reports_the_alternatives() {
    let mut state = state_with_one_installation();
    let (_dir, products) = temp_product_db();
    products
        .execute(
            "UPDATE com_object SET dpt_list = 'DPST-1-1 DPST-3-7' WHERE id = 'A-1_O-1'",
            [],
        )
        .unwrap();
    state.product_db = Some(
        knx_server::CatalogueOwner::adopt_opened(products, std::path::PathBuf::new()).unwrap(),
    );
    let app = knx_server::app(Arc::new(state), None);

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/devices")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({ "catalogItemId": "M-1_CI-1", "name": "Ambiguous actuator" })
                        .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let response = body_json(response).await;
    assert!(response["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .any(|diagnostic| {
            diagnostic["kind"] == "ambiguousDpt"
                && diagnostic["refId"] == "A-1_O-1_R-1"
                && diagnostic["alternatives"] == json!(["DPST-1-1", "DPST-3-7"])
        }));
}

#[tokio::test]
async fn saved_instance_priority_mounted_get_preserves_all_required_states_after_working_reopen() {
    use knx_core::{
        ComObjectInstance, SavedInstancePriority as Fact, SavedPriorityAbsent,
        SavedPriorityDeclared, SavedPriorityOmission as Omission, SavedPrioritySource,
        SavedPriorityUnavailable as Reason,
    };
    use knx_store::project_history::{self as history, NativeSnapshot};
    let state = state_with_one_installation();
    let mut project = state.project.lock().unwrap().take().unwrap();
    let owner = project.ids.next_device_id().unwrap();
    let device_source = SourceRef {
        path: "P-SYNTHETIC/0.xml".into(),
        ets_id: "P-SYNTHETIC-0_DI-1".into(),
    };
    project.devices.insert(DeviceInstance {
        id: owner,
        source: device_source.clone(),
        name: "Saved root".into(),
        description: None,
        address: None,
        product_ref: "M-1_P-1".into(),
        program_ref: "H-1_HP-1".into(),
        commissioning: Default::default(),
        visibility_calculated: true,
        com_objects: vec![],
        binary_data: vec![],
    });
    project.installations[0].topology.unassigned.push(owner);
    let mut expected = Vec::new();
    for position in 0..15 {
        let id = project.ids.next_com_object_instance_id().unwrap();
        let object_source = SourceRef {
            path: device_source.path.clone(),
            ets_id: format!("O-{}_R-1", position + 1),
        };
        let source = SavedPrioritySource {
            source_schema: 23,
            device_source: device_source.clone(),
            object_source: object_source.clone(),
            owner_xpath: format!("/synthetic/owner/object-{position}"),
            topology_sha256: "a".repeat(64),
        };
        let fact = match position {
            0 => Fact::Declared(Box::new(SavedPriorityDeclared {
                raw: "High".into(),
                source,
            })),
            1 => Fact::Declared(Box::new(SavedPriorityDeclared {
                raw: String::new(),
                source,
            })),
            2 => Fact::Declared(Box::new(SavedPriorityDeclared {
                raw: " future Ω ".into(),
                source,
            })),
            3 | 4 => Fact::Absent(Box::new(SavedPriorityAbsent {
                omission: if position == 3 {
                    Omission::AttributeOmitted
                } else {
                    Omission::InstanceElementOmitted
                },
                source,
            })),
            5 => Fact::default(),
            6 => Fact::Unavailable {
                reason: Reason::UnsupportedSourceSchema,
            },
            7 => Fact::Unavailable {
                reason: Reason::ModuleScope,
            },
            8 => Fact::Unavailable {
                reason: Reason::AmbiguousOwner,
            },
            9 => Fact::Unavailable {
                reason: Reason::AmbiguousDeclaration,
            },
            10 => Fact::Unavailable {
                reason: Reason::SourceIntegrity,
            },
            11 => Fact::Unavailable {
                reason: Reason::ResourceLimit,
            },
            12 => Fact::Declared(Box::new(SavedPriorityDeclared {
                raw: "Low".into(),
                source,
            })),
            13 => Fact::Declared(Box::new(SavedPriorityDeclared {
                raw: "Alert".into(),
                source,
            })),
            14 => Fact::Declared(Box::new(SavedPriorityDeclared {
                raw: "WITHHELD-REBOUND".into(),
                source,
            })),
            _ => unreachable!(),
        };
        project.devices.insert_com_object(ComObjectInstance {
            id,
            device: owner,
            source: object_source,
            number: (position + 1) as u16,
            text: knx_core::Override::Absent,
            description: knx_core::Override::Absent,
            dpt: knx_core::Override::Absent,
            flags: ResolvedFlags::none(),
            size: None,
            is_active: true,
            links: vec![],
            module_instance: None,
            priority_edit: None,
            saved_instance_priority: fact,
        });
        project.devices.get_mut(owner).unwrap().com_objects.push(id);
        if position == 14 {
            project.devices.com_object_mut(id).unwrap().source.path = "scoped/0.xml".into();
        }
    }
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("working.knxdb");
    let conn = knx_store::open_and_migrate(&path).unwrap();
    let mut snapshot = NativeSnapshot {
        project,
        opaque: vec![],
        manufacturer_refs: vec![],
    };
    let mut stack = knx_core::CommandStack::new();
    let generation = history::save_editor(&conn, &snapshot, &stack, None, true).unwrap();
    stack
        .do_command(
            &mut snapshot.project,
            knx_core::Command::RenameDevice {
                device: owner,
                name: "Recovered working".into(),
            },
        )
        .unwrap();
    history::save_editor(&conn, &snapshot, &stack, Some(generation), false).unwrap();
    drop(conn);
    let conn = knx_store::open_existing_and_migrate(&path).unwrap();
    let recovered = history::load_editor(&conn).unwrap().unwrap();
    let view = knx_projection::build_device_detail(&recovered.working.project, owner).unwrap();
    for com in view.com_objects {
        expected.push(serde_json::to_value(com.saved_instance_priority).unwrap());
    }
    assert_eq!(expected[0]["status"], "declared");
    assert_eq!(expected[0]["raw"], "High");
    assert_eq!(expected[1]["token_status"], "Empty");
    assert_eq!(expected[2]["token_status"], "Unknown");
    assert_eq!(expected[3]["omission"], "AttributeOmitted");
    assert_eq!(expected[4]["omission"], "InstanceElementOmitted");
    assert_eq!(
        expected[14],
        json!({"status":"unavailable","reason":"SourceBindingMismatch"})
    );
    // Pin wire reasons independently of the production projection, including payload absence.
    let assert_unavailable = |facts: &[serde_json::Value]| {
        for (position, reason) in [
            (5, "NotCaptured"),
            (6, "UnsupportedSourceSchema"),
            (7, "ModuleScope"),
            (8, "AmbiguousOwner"),
            (9, "AmbiguousDeclaration"),
            (10, "SourceIntegrity"),
            (11, "ResourceLimit"),
        ] {
            assert_eq!(
                facts[position],
                json!({"status": "unavailable", "reason": reason}),
                "independent unavailable wire contract at position {position}"
            );
        }
    };
    *state.project.lock().unwrap() = Some(recovered.working.project.clone());
    *state.command_stack.lock().unwrap() =
        knx_core::CommandStack::from_snapshot_states(recovered.undo, recovered.redo);
    let state = Arc::new(state);
    let app = knx_server::app(Arc::clone(&state), None);
    let response = app
        .oneshot(
            Request::builder()
                .uri(format!("/api/device/{}", owner.0))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = body_json(response).await;
    assert_eq!(body["name"], "Recovered working");
    let got: Vec<_> = body["com_objects"]
        .as_array()
        .unwrap()
        .iter()
        .map(|o| o["saved_instance_priority"].clone())
        .collect();
    assert_eq!(got, expected);
    assert_unavailable(&got);
    assert!(!body.to_string().contains("WITHHELD-REBOUND"));
    // Actual catalogue/activation/text overlays must not alias or replace captured fields.
    let (_products_dir, products) = temp_product_db();
    products
        .execute("UPDATE com_object SET priority='Low'", [])
        .unwrap();
    let mut enriched = state_with_one_installation();
    enriched.product_db = Some(
        knx_server::CatalogueOwner::adopt_opened(products, std::path::PathBuf::new()).unwrap(),
    );
    *enriched.project.lock().unwrap() = Some(recovered.working.project);
    let response = knx_server::app(Arc::new(enriched), None)
        .oneshot(
            Request::builder()
                .uri(format!("/api/device/{}", owner.0))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = body_json(response).await;
    let got: Vec<_> = body["com_objects"]
        .as_array()
        .unwrap()
        .iter()
        .map(|o| o["saved_instance_priority"].clone())
        .collect();
    assert_eq!(got, expected);
    assert_unavailable(&got);
}

/// Product program for effective priority: each object pins one layer/literal case.
const PRIORITY_PROGRAM: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<KNX xmlns="http://knx.org/xml/project/11"><ManufacturerData><Manufacturer RefId="M-1">
<ApplicationPrograms><ApplicationProgram Id="A-1" Name="P" ApplicationVersion="1"
  MaskVersion="MV-0701"><Static>
<ComObjectTable>
  <ComObject Id="A-1_O-1" Number="1" Text="a" ObjectSize="1 Bit" Priority="Low" />
  <ComObject Id="A-1_O-2" Number="2" Text="b" ObjectSize="1 Bit" Priority="High" />
  <ComObject Id="A-1_O-3" Number="3" Text="c" ObjectSize="1 Bit" Priority="Low" />
  <ComObject Id="A-1_O-4" Number="4" Text="d" ObjectSize="1 Bit" />
  <ComObject Id="A-1_O-5" Number="5" Text="e" ObjectSize="1 Bit" Priority="Low" />
  <ComObject Id="A-1_O-6" Number="6" Text="f" ObjectSize="1 Bit" />
  <ComObject Id="A-1_O-7" Number="7" Text="g" ObjectSize="1 Bit" Priority="Alert" />
  <ComObject Id="A-1_O-8" Number="8" Text="h" ObjectSize="1 Bit" Priority="Alert" />
  <ComObject Id="A-1_O-9" Number="9" Text="i" ObjectSize="1 Bit" Priority="Alert" />
  <ComObject Id="A-1_O-10" Number="10" Text="j" ObjectSize="1 Bit" Priority="Alert" />
</ComObjectTable>
<ComObjectRefs>
  <ComObjectRef Id="A-1_O-1_R-1" RefId="A-1_O-1" Priority="Alert" />
  <ComObjectRef Id="A-1_O-2_R-1" RefId="A-1_O-2" />
  <ComObjectRef Id="A-1_O-3_R-1" RefId="A-1_O-3" Priority="Low" />
  <ComObjectRef Id="A-1_O-4_R-1" RefId="A-1_O-4" />
  <ComObjectRef Id="A-1_O-5_R-1" RefId="A-1_O-5" Priority="" />
  <ComObjectRef Id="A-1_O-6_R-1" RefId="A-1_O-6" Priority="low" />
  <ComObjectRef Id="A-1_O-7_R-1" RefId="A-1_O-7" />
  <ComObjectRef Id="A-1_O-8_R-1" RefId="A-1_O-8" />
  <ComObjectRef Id="A-1_O-9_R-1" RefId="A-1_O-9" />
  <ComObjectRef Id="A-1_O-10_R-1" RefId="A-1_O-10" />
</ComObjectRefs>
</Static></ApplicationProgram></ApplicationPrograms></Manufacturer></ManufacturerData></KNX>"#;

/// One device whose objects carry the given saved facts; `ref` is the object `RefId`.
fn effective_priority_project(
    program_ref: &str,
    objects: &[(&str, knx_core::SavedInstancePriority)],
) -> (Project, DeviceId) {
    use knx_core::{ComObjectInstance, SavedInstancePriority as Fact};
    let mut project = Project::new(Language("en".into()));
    project.installations = state_with_one_installation()
        .project
        .lock()
        .unwrap()
        .take()
        .unwrap()
        .installations;
    let owner = project.ids.next_device_id().unwrap();
    let device_source = SourceRef {
        path: "P-SYNTHETIC/0.xml".into(),
        ets_id: "P-SYNTHETIC-0_DI-1".into(),
    };
    project.devices.insert(DeviceInstance {
        id: owner,
        source: device_source.clone(),
        name: "Priorities".into(),
        description: None,
        address: None,
        product_ref: "M-1_P-1".into(),
        program_ref: program_ref.into(),
        commissioning: Default::default(),
        visibility_calculated: true,
        com_objects: vec![],
        binary_data: vec![],
    });
    project.installations[0].topology.unassigned.push(owner);
    for (position, (ref_id, fact)) in objects.iter().enumerate() {
        let id = project.ids.next_com_object_instance_id().unwrap();
        let object_source = SourceRef {
            path: device_source.path.clone(),
            ets_id: (*ref_id).into(),
        };
        let mut fact = fact.clone();
        let with_source = |fact: &mut Fact| {
            let source = knx_core::SavedPrioritySource {
                source_schema: 23,
                device_source: device_source.clone(),
                object_source: object_source.clone(),
                owner_xpath: format!("/synthetic/owner/object-{position}"),
                topology_sha256: "b".repeat(64),
            };
            match fact {
                Fact::Absent(value) => value.source = source,
                Fact::Declared(value) => value.source = source,
                Fact::Unavailable { .. } => {}
            }
        };
        with_source(&mut fact);
        project.devices.insert_com_object(ComObjectInstance {
            id,
            device: owner,
            source: object_source,
            number: (position + 1) as u16,
            text: Override::Absent,
            description: Override::Absent,
            dpt: Override::Absent,
            flags: ResolvedFlags::none(),
            size: None,
            is_active: true,
            links: vec![],
            module_instance: None,
            priority_edit: None,
            saved_instance_priority: fact,
        });
        project.devices.get_mut(owner).unwrap().com_objects.push(id);
    }
    (project, owner)
}

fn placeholder_source() -> knx_core::SavedPrioritySource {
    knx_core::SavedPrioritySource {
        source_schema: 23,
        device_source: SourceRef {
            path: String::new(),
            ets_id: String::new(),
        },
        object_source: SourceRef {
            path: String::new(),
            ets_id: String::new(),
        },
        owner_xpath: String::new(),
        topology_sha256: String::new(),
    }
}

fn absent_fact(omission: knx_core::SavedPriorityOmission) -> knx_core::SavedInstancePriority {
    knx_core::SavedInstancePriority::Absent(Box::new(knx_core::SavedPriorityAbsent {
        omission,
        source: placeholder_source(),
    }))
}

fn declared_fact(raw: &str) -> knx_core::SavedInstancePriority {
    knx_core::SavedInstancePriority::Declared(Box::new(knx_core::SavedPriorityDeclared {
        raw: raw.into(),
        source: placeholder_source(),
    }))
}

async fn get_objects(state: knx_server::AppState, device: DeviceId) -> Vec<Value> {
    let response = knx_server::app(Arc::new(state), None)
        .oneshot(
            Request::builder()
                .uri(format!("/api/device/{}", device.0))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    body_json(response).await["com_objects"]
        .as_array()
        .unwrap()
        .clone()
}

fn with_priority_products(project: Project) -> (tempfile::TempDir, knx_server::AppState) {
    let dir = tempfile::tempdir().unwrap();
    let conn = knx_productdb::open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
    knx_productdb::parse::hardware::ingest_hardware(&conn, "h", "M-1/H.xml", HARDWARE.as_bytes())
        .unwrap();
    knx_productdb::parse::program::ingest_program(
        &conn,
        "p",
        "M-1/A.xml",
        PRIORITY_PROGRAM.as_bytes(),
    )
    .unwrap();
    knx_productdb::parse::catalog::ingest_catalog(&conn, "c", "M-1/C.xml", CATALOG.as_bytes())
        .unwrap();
    let mut state = state_with_one_installation();
    state.product_db =
        Some(knx_server::CatalogueOwner::adopt_opened(conn, std::path::PathBuf::new()).unwrap());
    *state.project.lock().unwrap() = Some(project);
    (dir, state)
}

#[tokio::test]
async fn effective_priority_mounted_get_names_winning_source_and_never_inherits_bad_declarations() {
    use knx_core::{SavedInstancePriority as Fact, SavedPriorityOmission as Omission};
    let unresolved = |reason: &str| json!({"status": "unresolved", "reason": reason, "source": null, "raw": null});
    let product = |layer: &str, omission: &str| json!({"kind": "product", "layer": layer, "omission": omission});
    let cases: Vec<(&str, Fact, Value, Value)> = vec![
        // Reference literal wins by presence over the program's.
        (
            "O-1_R-1",
            absent_fact(Omission::AttributeOmitted),
            json!({"status": "resolved", "value": "Alert", "source": product("ProgramRef", "AttributeOmitted")}),
            unresolved("NoProductDatabase"),
        ),
        // §1.2.5.11 + §1.2.5.13: an unlisted instance also takes the product value.
        (
            "O-2_R-1",
            absent_fact(Omission::InstanceElementOmitted),
            json!({"status": "resolved", "value": "High", "source": product("Program", "InstanceElementOmitted")}),
            unresolved("NoProductDatabase"),
        ),
        // Equal literals at both layers still report the reference.
        (
            "O-3_R-1",
            absent_fact(Omission::AttributeOmitted),
            json!({"status": "resolved", "value": "Low", "source": product("ProgramRef", "AttributeOmitted")}),
            unresolved("NoProductDatabase"),
        ),
        (
            "O-4_R-1",
            absent_fact(Omission::AttributeOmitted),
            unresolved("ProductLiteralAbsent"),
            unresolved("NoProductDatabase"),
        ),
        (
            "O-5_R-1",
            absent_fact(Omission::AttributeOmitted),
            json!({"status": "unresolved", "reason": "ProductEmpty", "source": product("ProgramRef", "AttributeOmitted"), "raw": ""}),
            unresolved("NoProductDatabase"),
        ),
        (
            "O-6_R-1",
            absent_fact(Omission::InstanceElementOmitted),
            json!({"status": "unresolved", "reason": "ProductUnknown", "source": product("ProgramRef", "InstanceElementOmitted"), "raw": "low"}),
            unresolved("NoProductDatabase"),
        ),
        (
            "O-99_R-1",
            absent_fact(Omission::AttributeOmitted),
            unresolved("ProductObjectNotFound"),
            unresolved("NoProductDatabase"),
        ),
        // Counter-proofs: declarations never take the product's Alert.
        (
            "O-7_R-1",
            declared_fact("High"),
            json!({"status": "resolved", "value": "High", "source": {"kind": "instance"}}),
            json!({"status": "resolved", "value": "High", "source": {"kind": "instance"}}),
        ),
        (
            "O-8_R-1",
            declared_fact(""),
            json!({"status": "unresolved", "reason": "InstanceEmpty", "source": {"kind": "instance"}, "raw": ""}),
            json!({"status": "unresolved", "reason": "InstanceEmpty", "source": {"kind": "instance"}, "raw": ""}),
        ),
        (
            "O-9_R-1",
            declared_fact(" Alert"),
            json!({"status": "unresolved", "reason": "InstanceUnknown", "source": {"kind": "instance"}, "raw": " Alert"}),
            json!({"status": "unresolved", "reason": "InstanceUnknown", "source": {"kind": "instance"}, "raw": " Alert"}),
        ),
        (
            "O-10_R-1",
            Fact::default(),
            unresolved("InstanceUnavailable"),
            unresolved("InstanceUnavailable"),
        ),
    ];
    let facts: Vec<(&str, Fact)> = cases.iter().map(|(r, f, _, _)| (*r, f.clone())).collect();

    let (project, owner) = effective_priority_project("H-1_HP-1", &facts);
    let (_dir, state) = with_priority_products(project.clone());
    let with_db = get_objects(state, owner).await;
    let without_db_state = state_with_one_installation();
    *without_db_state.project.lock().unwrap() = Some(project);
    let without_db = get_objects(without_db_state, owner).await;
    for (index, (ref_id, _, expected, expected_without_db)) in cases.iter().enumerate() {
        assert_eq!(
            with_db[index]["effective_priority"], *expected,
            "{ref_id} #{index}"
        );
        assert_eq!(
            without_db[index]["effective_priority"], *expected_without_db,
            "{ref_id} #{index} without product database"
        );
        // The captured fact is untouched by the overlay.
        assert_eq!(
            with_db[index]["saved_instance_priority"],
            without_db[index]["saved_instance_priority"]
        );
    }
}

#[tokio::test]
async fn effective_priority_names_missing_program_and_withholds_rebound_facts() {
    use knx_core::SavedPriorityOmission as Omission;
    let unresolved = |reason: &str| json!({"status": "unresolved", "reason": reason, "source": null, "raw": null});
    let facts = [("O-1_R-1", absent_fact(Omission::AttributeOmitted))];
    for (program_ref, reason) in [
        ("", "NoProgramReference"),
        ("H-9_HP-9", "ProgramNotInDatabase"),
    ] {
        let (project, owner) = effective_priority_project(program_ref, &facts);
        let (_dir, state) = with_priority_products(project);
        let objects = get_objects(state, owner).await;
        assert_eq!(
            objects[0]["effective_priority"],
            unresolved(reason),
            "{program_ref:?}"
        );
    }
    // Without a program reference the missing database is irrelevant: that reason wins.
    let (project, owner) = effective_priority_project("", &facts);
    let no_db = state_with_one_installation();
    *no_db.project.lock().unwrap() = Some(project);
    let objects = get_objects(no_db, owner).await;
    assert_eq!(
        objects[0]["effective_priority"],
        unresolved("NoProgramReference")
    );
    // A rebound clone withholds the saved fact, so the product cannot fill it either.
    let (mut project, owner) = effective_priority_project("H-1_HP-1", &facts);
    let id = project.devices.get(owner).unwrap().com_objects[0];
    project.devices.com_object_mut(id).unwrap().source.path = "scoped/0.xml".into();
    let (_dir, state) = with_priority_products(project);
    let objects = get_objects(state, owner).await;
    assert_eq!(
        objects[0]["saved_instance_priority"],
        json!({"status": "unavailable", "reason": "SourceBindingMismatch"})
    );
    assert_eq!(
        objects[0]["effective_priority"],
        unresolved("InstanceUnavailable")
    );
}

async fn call(
    router: &axum::Router,
    method: &str,
    uri: &str,
    body: Option<Value>,
) -> (StatusCode, Value) {
    let request = Request::builder().method(method).uri(uri);
    let request = match body {
        Some(body) => request
            .header("content-type", "application/json")
            .body(Body::from(body.to_string())),
        None => request.body(Body::empty()),
    }
    .unwrap();
    let response = router.clone().oneshot(request).await.unwrap();
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
async fn a_priority_edit_wins_validates_its_token_undoes_and_leaves_the_captured_fact() {
    use knx_core::SavedPriorityOmission as Omission;
    let facts = [
        ("O-1_R-1", absent_fact(Omission::AttributeOmitted)),
        ("O-8_R-1", declared_fact("")),
        ("O-10_R-1", knx_core::SavedInstancePriority::default()),
    ];
    let (project, owner) = effective_priority_project("H-1_HP-1", &facts);
    let ids: Vec<u32> = project.devices.com_objects().map(|c| c.id.0).collect();
    let (_dir, state) = with_priority_products(project);
    let router = knx_server::app(Arc::new(state), None);
    let device = format!("/api/device/{}", owner.0);
    let (_, before) = call(&router, "GET", &device, None).await;
    let edit = |id: u32, priority: Value| json!({"comObjectId": id, "priority": priority});
    let user =
        |value: &str| json!({"status": "resolved", "value": value, "source": {"kind": "userEdit"}});

    for (id, token) in ids.iter().zip(["Low", "High", "Alert"]) {
        let (status, _) = call(
            &router,
            "POST",
            "/api/com-object-priority",
            Some(edit(*id, json!(token))),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{token}");
    }
    let (_, after) = call(&router, "GET", &device, None).await;
    for (index, token) in ["Low", "High", "Alert"].into_iter().enumerate() {
        let object = &after["com_objects"][index];
        // Over a product literal, an empty declaration and an unavailable capture alike.
        assert_eq!(object["effective_priority"], user(token), "#{index}");
        assert_eq!(
            object["saved_instance_priority"],
            before["com_objects"][index]["saved_instance_priority"],
            "the captured fact is untouched"
        );
    }

    // ids[1] holds High: each refusal below would otherwise be a real change,
    // so no no-op refusal can hide a token or shape check.
    for refused in [
        edit(ids[1], json!("low")),
        edit(ids[1], json!(" Low")),
        edit(ids[1], json!("Urgent")),
        edit(ids[1], json!("")),
        edit(ids[0], json!("Low")),
        edit(9_999, json!("Low")),
    ] {
        let (status, _) = call(
            &router,
            "POST",
            "/api/com-object-priority",
            Some(refused.clone()),
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{refused}");
    }
    for malformed in [
        json!({"comObjectId": ids[1]}),
        json!({"comObjectId": ids[1], "priority": "Low", "extra": 1}),
    ] {
        let (status, _) = call(
            &router,
            "POST",
            "/api/com-object-priority",
            Some(malformed.clone()),
        )
        .await;
        assert!(status.is_client_error(), "{malformed}: {status}");
    }
    let (_, unchanged) = call(&router, "GET", &device, None).await;
    assert_eq!(
        unchanged["com_objects"], after["com_objects"],
        "refusals change nothing"
    );

    let (status, _) = call(
        &router,
        "POST",
        "/api/com-object-priority",
        Some(edit(ids[2], Value::Null)),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let (_, cleared) = call(&router, "GET", &device, None).await;
    assert_eq!(
        cleared["com_objects"][2]["effective_priority"],
        before["com_objects"][2]["effective_priority"]
    );
    for _ in 0..4 {
        let (status, _) = call(&router, "POST", "/api/undo", None).await;
        assert_eq!(status, StatusCode::OK);
    }
    let (_, undone) = call(&router, "GET", &device, None).await;
    assert_eq!(
        undone["com_objects"], before["com_objects"],
        "undo restores every prior edit state"
    );
}
