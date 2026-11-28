//! HTTP integration tests for the WP-51 missing-name wizard backend routes.

use std::os::unix::fs::PermissionsExt;
use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use knx_core::function::{Function, FunctionDetails, FunctionId};
use knx_core::{
    Area, AreaId, BuildingPart, BuildingPartId, BuildingPartType, CommissioningState,
    CompletionStatus, DeviceId, DeviceInstance, GroupAddress, GroupAddressEntry, GroupAddressId,
    GroupRange, GroupRangeId, IndividualAddress, Installation, InstallationId, Language, Line,
    LineId, Override, Project, SourceRef, Topology,
};
use knx_testsupport::zip_with_entries;
use serde_json::{json, Value};
use tower::ServiceExt;

use crate::saving_hold;

async fn request(app: &axum::Router, method: &str, uri: &str, body: Value) -> (StatusCode, Value) {
    let builder = Request::builder().method(method).uri(uri);
    let req = if body.is_null() {
        builder.body(Body::empty()).unwrap()
    } else {
        builder
            .header("content-type", "application/json")
            .body(Body::from(body.to_string()))
            .unwrap()
    };
    let response = app.clone().oneshot(req).await.unwrap();
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or_else(|_| json!({"nonJsonError": true})),
    )
}

fn src(id: &str) -> SourceRef {
    SourceRef {
        ets_id: id.into(),
        path: format!("/KNX/Project/{id}"),
    }
}

fn full_q9_project() -> Project {
    let mut project = Project::new(Language("en-US".into()));
    project.info.name = String::new();
    project.installations = vec![Installation {
        id: InstallationId(0),
        name: "   ".into(),
        completion: CompletionStatus::Undefined,
        default_line: None,
        multicast_address: None,
        parameters: vec![],
        topology: Topology {
            areas: vec![Area {
                id: AreaId(1),
                source: src("A-1"),
                address: 1,
                name: "  ".into(),
                completion: CompletionStatus::Undefined,
                lines: vec![LineId(1)],
            }],
            lines: vec![Line {
                id: LineId(1),
                source: src("L-1"),
                address: 1,
                name: String::new(),
                medium_ref: "MT-0".into(),
                domain_address: None,
                domain_address_is_checked: None,
                ip_routing_multicast_address: None,
                multicast_ttl: None,
                completion: CompletionStatus::Undefined,
                devices: vec![DeviceId(1)],
            }],
            unassigned: vec![],
        },
        group_ranges: vec![GroupRange {
            id: GroupRangeId(1),
            source: src("GR-1"),
            name: String::new(),
            start: GroupAddress::from_raw(1),
            end: GroupAddress::from_raw(2047),
            parent: None,
            children: vec![],
        }],
        group_addresses: vec![
            GroupAddressEntry {
                id: GroupAddressId(1),
                source: src("GA-1"),
                name: " ".into(),
                address: GroupAddress::from_raw(1),
                central: false,
                unfiltered: false,
                range: Some(GroupRangeId(1)),
                declared_dpt: Override::Absent,
                description: None,
            },
            GroupAddressEntry {
                id: GroupAddressId(2),
                source: src("GA-2"),
                name: "Already Named".into(),
                address: GroupAddress::from_raw(2),
                central: false,
                unfiltered: false,
                range: Some(GroupRangeId(1)),
                declared_dpt: Override::Absent,
                description: None,
            },
        ],
        buildings: vec![BuildingPart {
            id: BuildingPartId(1),
            source: src("BP-1"),
            name: String::new(),
            kind: BuildingPartType::Room,
            parent: None,
            number: None,
            default_line: None,
            completion: CompletionStatus::Undefined,
            children: vec![],
            devices: vec![DeviceId(1)],
        }],
    }];
    project.devices.insert(DeviceInstance {
        id: DeviceId(1),
        source: src("D-1"),
        address: Some(IndividualAddress::new(1, 1, 1).unwrap()),
        name: "\t".into(),
        description: None,
        product_ref: String::new(),
        program_ref: String::new(),
        commissioning: CommissioningState::default(),
        visibility_calculated: true,
        com_objects: vec![],
        binary_data: vec![],
    });
    project.functions = vec![Function {
        id: FunctionId(1),
        source: src("F-1"),
        location: BuildingPartId(1),
        details: FunctionDetails::default(),
        members: vec![],
    }];
    // Native history requires allocator metadata to survive an exact store roundtrip.
    project.ids = knx_core::project::IdAllocators::from_counts(1, 1, 1, 0, 1, 2, 1, 0, 0)
        .with_function_counts(1, 0);
    project
}

fn state_with_project(
    project: Project,
) -> (tempfile::TempDir, Arc<knx_server::AppState>, axum::Router) {
    let dir = tempfile::tempdir().unwrap();
    let state = Arc::new(knx_server::AppState::new(dir.path().to_path_buf()));
    *state.project.lock().unwrap() = Some(project);
    let app = knx_server::app(state.clone(), None);
    (dir, state, app)
}

#[tokio::test]
async fn all_nine_q9_targets_scan_preview_apply_and_single_undo_restores_exact_blank_and_whitespace(
) {
    let (_dir, state, app) = state_with_project(full_q9_project());

    let (status, scan) = request(&app, "POST", "/api/missing-names/scan", json!({})).await;
    assert_eq!(status, StatusCode::OK, "scan: {scan}");
    assert_eq!(scan["persistence"], "session-only");
    assert_eq!(scan["discovery"]["completeness"], "complete");
    assert_eq!(scan["discovery"]["counts"]["total_candidates"], 9);
    let items = scan["items"].as_array().unwrap();
    assert_eq!(items.len(), 9);

    let entries = json!([
        {"target": {"kind": "project"}, "name": "Main Project"},
        {"target": {"kind": "installation", "id": 0}, "name": "Main Installation"},
        {"target": {"kind": "building_part", "id": 1}, "name": "Living Room"},
        {"target": {"kind": "area", "id": 1}, "name": "Area 1"},
        {"target": {"kind": "line", "id": 1}, "name": "Line 1.1"},
        {"target": {"kind": "device", "id": 1}, "name": "Switch Actuator"},
        {"target": {"kind": "group_range", "id": 1}, "name": "Lighting"},
        {"target": {"kind": "group_address", "id": 1}, "name": "Ceiling Light"},
        {"target": {"kind": "function", "id": 1}, "name": "Room Lighting"}]);

    // Non-blank target must be refused with 422 missingNameTargetNotBlank.
    let (status, refused) = request(
        &app,
        "POST",
        "/api/missing-names/preview",
        json!({
            "serverIncarnation": scan["serverIncarnation"],
            "projectIncarnation": scan["projectIncarnation"],
            "snapshotRevision": scan["snapshotRevision"],
            "entries": [{"target": {"kind": "group_address", "id": 2}, "name": "Overwrite"}]
        }),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{refused}");
    assert_eq!(refused["kind"], "missingNameTargetNotBlank");

    // Preview all 9 targets; live project must remain untouched.
    let before = state.project.lock().unwrap().clone();
    let (status, preview) = request(
        &app,
        "POST",
        "/api/missing-names/preview",
        json!({
            "serverIncarnation": scan["serverIncarnation"],
            "projectIncarnation": scan["projectIncarnation"],
            "snapshotRevision": scan["snapshotRevision"],
            "entries": entries
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "preview: {preview}");
    assert_eq!(preview["expiresInSeconds"], 300);
    assert_eq!(preview["persistence"], "session-only");
    assert_eq!(preview["preview"]["changed"], 9);
    assert_eq!(preview["preview"]["rows"].as_array().unwrap().len(), 9);
    assert_eq!(state.project.lock().unwrap().clone(), before);

    // Apply atomically.
    let (status, applied) = request(
        &app,
        "POST",
        "/api/missing-names/apply",
        json!({"token": preview["token"], "confirmed": true}),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "apply: {applied}");
    assert_eq!(applied["changed"], true);
    assert_eq!(applied["persistence"], "session-only");
    assert!(applied["nativeGeneration"].is_null());
    assert_eq!(
        state.command_stack.lock().unwrap().history_lengths(),
        (1, 0)
    );
    {
        let guard = state.project.lock().unwrap();
        let p = guard.as_ref().unwrap();
        assert_eq!(p.info.name, "Main Project");
        assert_eq!(p.installations[0].name, "Main Installation");
        assert_eq!(p.installations[0].buildings[0].name, "Living Room");
        assert_eq!(p.installations[0].topology.areas[0].name, "Area 1");
        assert_eq!(p.installations[0].topology.lines[0].name, "Line 1.1");
        assert_eq!(p.devices.get(DeviceId(1)).unwrap().name, "Switch Actuator");
        assert_eq!(p.installations[0].group_ranges[0].name, "Lighting");
        assert_eq!(p.installations[0].group_addresses[0].name, "Ceiling Light");
        assert_eq!(p.installations[0].group_addresses[1].name, "Already Named");
        assert_eq!(p.functions[0].details.name, "Room Lighting");
    }

    // One single Undo restores exact empty and whitespace strings.
    let (status, _) = request(&app, "POST", "/api/undo", Value::Null).await;
    assert_eq!(status, StatusCode::OK);
    {
        let guard = state.project.lock().unwrap();
        let p = guard.as_ref().unwrap();
        assert_eq!(p.info.name, "");
        assert_eq!(p.installations[0].name, "   ");
        assert_eq!(p.installations[0].buildings[0].name, "");
        assert_eq!(p.installations[0].topology.areas[0].name, "  ");
        assert_eq!(p.installations[0].topology.lines[0].name, "");
        assert_eq!(p.devices.get(DeviceId(1)).unwrap().name, "\t");
        assert_eq!(p.installations[0].group_ranges[0].name, "");
        assert_eq!(p.installations[0].group_addresses[0].name, " ");
        assert_eq!(p.installations[0].group_addresses[1].name, "Already Named");
        assert_eq!(p.functions[0].details.name, "");
    }
}

#[tokio::test]
async fn product_db_resolves_product_text_with_and_without_language_without_fallback() {
    let dir = tempfile::tempdir().unwrap();
    let products = knx_productdb::open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
    // Seed two products: P-rich has product_text + de-DE translation; P-blank has empty
    // product_text but non-empty hardware, catalog, application, and manufacturer names.
    let hardware_xml = r#"<?xml version="1.0" encoding="utf-8"?>
<KNX xmlns="http://knx.org/xml/project/11"><ManufacturerData><Manufacturer RefId="M-0083">
<Hardware>
  <Hardware Id="H-rich" Name="Hardware Trap Rich" SerialNumber="S1" VersionNumber="1">
    <Products><Product Id="P-rich" Text="Switch Actuator 4-fold" OrderNumber="ORD-1" /></Products>
    <Hardware2Programs><Hardware2Program Id="HP-rich" MediumTypes="MT-0"><ApplicationProgramRef RefId="A-rich" /></Hardware2Program></Hardware2Programs>
  </Hardware>
  <Hardware Id="H-blank" Name="Hardware Fallback Trap" SerialNumber="S2" VersionNumber="1">
    <Products><Product Id="P-blank" Text="   " OrderNumber="ORD-2" /></Products>
    <Hardware2Programs><Hardware2Program Id="HP-blank" MediumTypes="MT-0"><ApplicationProgramRef RefId="A-blank" /></Hardware2Program></Hardware2Programs>
  </Hardware>
</Hardware>
<Languages>
  <Language Identifier="de-DE">
    <TranslationUnit RefId="H-rich">
      <TranslationElement RefId="P-rich">
        <Translation AttributeName="Text" Text="Schaltaktor 4-fach" />
      </TranslationElement>
    </TranslationUnit>
  </Language>
</Languages>
</Manufacturer></ManufacturerData></KNX>"#;
    knx_productdb::ingest_file(&products, "M-0083/Hardware.xml", hardware_xml.as_bytes()).unwrap();
    knx_productdb::ingest_file(
        &products,
        "knx_master.xml",
        br#"<KNX><MasterData><Manufacturers><Manufacturer Id="M-0083" Name="Manufacturer Trap"/></Manufacturers></MasterData></KNX>"#,
    )
    .unwrap();
    products
        .execute_batch(
            "INSERT INTO catalog_section VALUES('CS-1','M-0083',NULL,'Catalog Section Trap','01','','en-US','src');
             INSERT INTO catalog_item VALUES('CI-1','M-0083','CS-1','Catalog Fallback Trap','CAT-2','','P-blank','HP-blank','en-US','src');
             INSERT INTO application_program(id,manufacturer_id,name,application_number,application_version,program_type,mask_version,pei_type,load_procedure_style,default_language,hash,linkable,original_manufacturer,source_sha256) VALUES('A-blank','M-0083','App Fallback Trap','1','1','ApplicationProgram','MV-0701','0','ProductSpecific','en-US','h',0,'M-0083','src');",
        )
        .unwrap();

    let mut project = Project::new(Language("en-US".into()));
    project.info.name = "Named Project".into();
    project.installations = vec![Installation {
        id: InstallationId(0),
        name: "Named Inst".into(),
        completion: CompletionStatus::Undefined,
        default_line: None,
        multicast_address: None,
        parameters: vec![],
        topology: Topology {
            areas: vec![],
            lines: vec![],
            unassigned: vec![],
        },
        group_ranges: vec![],
        group_addresses: vec![],
        buildings: vec![],
    }];
    for device in [
        DeviceInstance {
            id: DeviceId(1),
            source: src("D-1"),
            address: Some(IndividualAddress::new(1, 1, 1).unwrap()),
            name: String::new(),
            description: None,
            product_ref: "P-rich".into(),
            program_ref: "HP-rich".into(),
            commissioning: CommissioningState::default(),
            visibility_calculated: true,
            com_objects: vec![],
            binary_data: vec![],
        },
        DeviceInstance {
            id: DeviceId(2),
            source: src("D-2"),
            address: Some(IndividualAddress::new(1, 1, 2).unwrap()),
            name: String::new(),
            description: None,
            product_ref: "P-blank".into(),
            program_ref: "HP-blank".into(),
            commissioning: CommissioningState::default(),
            visibility_calculated: true,
            com_objects: vec![],
            binary_data: vec![],
        },
    ] {
        project.devices.insert(device);
    }

    let mut state = knx_server::AppState::new(dir.path().to_path_buf());
    state.product_db = Some(
        knx_server::CatalogueOwner::adopt_opened(products, dir.path().join("products.sqlite"))
            .unwrap(),
    );
    *state.project.lock().unwrap() = Some(project.clone());
    let app = knx_server::app(Arc::new(state), None);

    // Without language: resolves default product_text for Device 1, never substitutes traps for Device 2.
    let (status, scan_default) = request(&app, "POST", "/api/missing-names/scan", json!({})).await;
    assert_eq!(status, StatusCode::OK, "{scan_default}");
    let items = scan_default["items"].as_array().unwrap();
    let d1 = items
        .iter()
        .find(|i| i["candidate"]["target"] == json!({"kind": "device", "id": 1}))
        .unwrap();
    let d2 = items
        .iter()
        .find(|i| i["candidate"]["target"] == json!({"kind": "device", "id": 2}))
        .unwrap();
    assert!(
        d1["proposed_name"]
            .as_str()
            .unwrap()
            .contains("Switch Actuator 4-fold"),
        "expected product_text in proposal: {d1}"
    );
    let d2_text = d2["proposed_name"].as_str().unwrap_or("");
    for forbidden in [
        "Hardware Fallback Trap",
        "Catalog Fallback Trap",
        "App Fallback Trap",
        "Manufacturer Trap",
    ] {
        assert!(
            !d2_text.contains(forbidden),
            "must not substitute {forbidden} for product_text: {d2}"
        );
    }

    // With language=de-DE: resolves localized product_text.
    let (status, scan_de) = request(
        &app,
        "POST",
        "/api/missing-names/scan",
        json!({"language": "de-DE"}),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{scan_de}");
    let d1_de = scan_de["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|i| i["candidate"]["target"] == json!({"kind": "device", "id": 1}))
        .unwrap();
    assert!(
        d1_de["proposed_name"]
            .as_str()
            .unwrap()
            .contains("Schaltaktor 4-fach"),
        "expected de-DE product_text in proposal: {d1_de}"
    );

    // Invalid language tag rejected with 400.
    let (status, _) = request(
        &app,
        "POST",
        "/api/missing-names/scan",
        json!({"language": "not a valid tag!"}),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    // Graceful degradation when product_db is absent.
    let (_dir2, _state2, app_no_db) = state_with_project(project);
    let (status, scan_no_db) =
        request(&app_no_db, "POST", "/api/missing-names/scan", json!({})).await;
    assert_eq!(status, StatusCode::OK, "{scan_no_db}");
    assert_eq!(scan_no_db["items"].as_array().unwrap().len(), 2);
}

#[tokio::test]
async fn product_hardware_conflicts_stay_manual_in_scan_and_child_reevaluation() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("products.sqlite");
    let products = knx_productdb::open_and_migrate(&db_path).unwrap();
    let hardware_xml = br#"<KNX xmlns="http://knx.org/xml/project/11"><ManufacturerData><Manufacturer RefId="M-0083"><Hardware>
      <Hardware Id="H-rich" Name="Hardware Trap" SerialNumber="S1" VersionNumber="1">
        <Products><Product Id="P-rich" Text="Verified product" OrderNumber="O1" /></Products>
        <Hardware2Programs><Hardware2Program Id="HP-rich" MediumTypes="MT-0"><ApplicationProgramRef RefId="A-rich" /></Hardware2Program></Hardware2Programs>
      </Hardware>
      <Hardware Id="H-blank" Name="Other Hardware Trap" SerialNumber="S2" VersionNumber="1">
        <Products><Product Id="P-blank" Text="   " OrderNumber="O2" /></Products>
        <Hardware2Programs><Hardware2Program Id="HP-blank" MediumTypes="MT-0"><ApplicationProgramRef RefId="A-blank" /></Hardware2Program></Hardware2Programs>
      </Hardware>
    </Hardware></Manufacturer></ManufacturerData></KNX>"#;
    knx_productdb::ingest_file(&products, "M-0083/Hardware.xml", hardware_xml).unwrap();
    // Prove the fixture represents two real links and a known contradiction, not missing data.
    for (product, program, expected) in [
        (
            "P-rich",
            "HP-rich",
            knx_productdb::query::DeviceProgramRelation::Matched,
        ),
        (
            "P-rich",
            "HP-blank",
            knx_productdb::query::DeviceProgramRelation::HardwareMismatch,
        ),
        (
            "P-blank",
            "HP-rich",
            knx_productdb::query::DeviceProgramRelation::HardwareMismatch,
        ),
        (
            "P-rich",
            "HP-missing",
            knx_productdb::query::DeviceProgramRelation::Missing,
        ),
    ] {
        let row = knx_productdb::query::device_product(&products, product, program, None)
            .unwrap()
            .unwrap();
        assert_eq!(row.program_relation, expected);
    }
    let changes_before = products.total_changes();
    let mut project = Project::new(Language("en-US".into()));
    project.info.name = "Named project".into();
    let device_ids: Vec<_> = (1..=6).map(DeviceId).collect();
    project.installations = vec![Installation {
        id: InstallationId(0),
        name: "Named installation".into(),
        completion: CompletionStatus::Undefined,
        default_line: None,
        multicast_address: None,
        parameters: vec![],
        topology: Topology {
            areas: vec![],
            lines: vec![],
            unassigned: device_ids.clone(),
        },
        group_ranges: vec![],
        group_addresses: vec![],
        buildings: vec![BuildingPart {
            id: BuildingPartId(1),
            source: src("Room"),
            name: "Room".into(),
            kind: BuildingPartType::Room,
            parent: None,
            number: None,
            default_line: None,
            completion: CompletionStatus::Undefined,
            children: vec![],
            devices: device_ids,
        }],
    }];
    for (id, product, program, description) in [
        (1, "P-rich", "HP-rich", None),
        (
            2,
            "P-rich",
            "HP-blank",
            Some("Description must not erase conflict"),
        ),
        (
            3,
            "P-blank",
            "HP-rich",
            Some("Blank text must not erase conflict"),
        ),
        (4, "P-rich", "HP-missing", None),
        // Repeated pair exercises the resolver cache, without a description this time.
        (5, "P-rich", "HP-blank", None),
        (6, "P-blank", "HP-blank", Some("Valid description fallback")),
    ] {
        project.devices.insert(DeviceInstance {
            id: DeviceId(id),
            source: src(&format!("D-{id}")),
            address: None,
            name: String::new(),
            description: description.map(str::to_owned),
            product_ref: product.into(),
            program_ref: program.into(),
            commissioning: CommissioningState::default(),
            visibility_calculated: true,
            com_objects: vec![],
            binary_data: vec![],
        });
    }
    let mut state = knx_server::AppState::new(dir.path().to_path_buf());
    state.product_db =
        Some(knx_server::CatalogueOwner::adopt_opened(products, db_path.clone()).unwrap());
    *state.project.lock().unwrap() = Some(project.clone());
    let state = Arc::new(state);
    let app = knx_server::app(state.clone(), None);
    let db_before = std::fs::read(&db_path).unwrap();
    let (status, scan) = request(&app, "POST", "/api/missing-names/scan", json!({})).await;
    assert_eq!(status, StatusCode::OK, "{scan}");
    assert_eq!(state.project.lock().unwrap().as_ref(), Some(&project));
    assert_eq!(
        state.command_stack.lock().unwrap().history_lengths(),
        (0, 0)
    );

    // Drafts can only name blank targets: install a separate fixture and scan it afresh.
    let mut draft_project = project.clone();
    draft_project.installations[0].buildings[0].name.clear();
    *state.project.lock().unwrap() = Some(draft_project.clone());
    let (status, draft_scan) = request(&app, "POST", "/api/missing-names/scan", json!({})).await;
    assert_eq!(status, StatusCode::OK, "{draft_scan}");
    let (status, reeval) = request(
        &app,
        "POST",
        "/api/missing-names/reevaluate",
        json!({
            "serverIncarnation": draft_scan["serverIncarnation"],
            "projectIncarnation": draft_scan["projectIncarnation"],
            "snapshotRevision": draft_scan["snapshotRevision"],
            "changedParents": [{"kind": "building_part", "id": 1}],
            "draftParents": [{"target": {"kind": "building_part", "id": 1}, "name": "Draft Room"}]
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{reeval}");
    for (items, location) in [
        (&scan["items"], "Room"),
        (&reeval["proposals"], "Draft Room"),
    ] {
        let items = items.as_array().unwrap();
        assert_eq!(items.len(), 6);
        for id in 1..=6 {
            let item = items
                .iter()
                .find(|item| item["candidate"]["target"] == json!({"kind":"device", "id":id}))
                .unwrap();
            match id {
                2 | 3 | 5 => {
                    assert!(
                        item["proposed_name"].is_null(),
                        "conflict must remain manual: {item}"
                    );
                    assert_eq!(item["provenance"]["origin"], "manual_required");
                    assert!(
                        item["provenance"]["gaps"]
                            .as_array()
                            .unwrap()
                            .iter()
                            .any(|gap| gap["gap"] == "ambiguous"
                                && gap["kind"] == "product_designation"
                                && gap["detail"] == "unclear evidence association"),
                        "{item}"
                    );
                }
                1 | 4 => {
                    assert_eq!(
                        item["proposed_name"],
                        format!("{location} – Verified product")
                    );
                    assert_eq!(item["provenance"]["origin"], "automatic");
                }
                6 => assert_eq!(item["proposed_name"], "Valid description fallback"),
                _ => unreachable!(),
            }
        }
    }
    let invalidated = reeval["invalidated_targets"].as_array().unwrap();
    for id in 1..=6 {
        assert!(invalidated.contains(&json!({"kind":"device", "id":id})));
    }
    assert_eq!(state.project.lock().unwrap().as_ref(), Some(&draft_project));
    assert_eq!(
        state.command_stack.lock().unwrap().history_lengths(),
        (0, 0)
    );
    assert_eq!(std::fs::read(&db_path).unwrap(), db_before);
    let changes_after = state
        .product_db
        .as_ref()
        .unwrap()
        .with_read(|conn| Ok::<_, rusqlite::Error>(conn.total_changes()))
        .unwrap()
        .unwrap();
    assert_eq!(changes_after, changes_before);

    // No database means no known contradiction; descriptions remain usable in that case.
    let (_no_db_dir, no_db_state, no_db_app) = state_with_project(project.clone());
    let (status, no_db_scan) =
        request(&no_db_app, "POST", "/api/missing-names/scan", json!({})).await;
    assert_eq!(status, StatusCode::OK, "{no_db_scan}");
    for (id, expected) in [
        (2, "Description must not erase conflict"),
        (3, "Blank text must not erase conflict"),
        (6, "Valid description fallback"),
    ] {
        let item = no_db_scan["items"]
            .as_array()
            .unwrap()
            .iter()
            .find(|item| item["candidate"]["target"] == json!({"kind":"device", "id":id}))
            .unwrap();
        assert_eq!(item["proposed_name"], expected);
        assert_eq!(item["provenance"]["origin"], "automatic");
    }
    assert_eq!(no_db_state.project.lock().unwrap().as_ref(), Some(&project));
    assert_eq!(
        no_db_state.command_stack.lock().unwrap().history_lengths(),
        (0, 0)
    );
}

#[tokio::test]
async fn q17_pagination_and_incomplete_scan_suppresses_full_totals() {
    let mut project = Project::new(Language("en-US".into()));
    project.info.name = "Named".into();
    let gas: Vec<_> = (1..=10)
        .map(|i| GroupAddressEntry {
            id: GroupAddressId(i),
            source: src(&format!("GA-{i}")),
            name: String::new(),
            address: GroupAddress::from_raw(i as u16),
            central: false,
            unfiltered: false,
            range: None,
            declared_dpt: Override::Absent,
            description: None,
        })
        .collect();
    project.installations = vec![Installation {
        id: InstallationId(0),
        name: "Inst".into(),
        completion: CompletionStatus::Undefined,
        default_line: None,
        multicast_address: None,
        parameters: vec![],
        topology: Topology {
            areas: vec![],
            lines: vec![],
            unassigned: vec![],
        },
        group_ranges: vec![],
        group_addresses: gas,
        buildings: vec![],
    }];
    let (_dir, _state, app) = state_with_project(project);

    // Incomplete scan: total_candidates and total_by_kind MUST be null.
    let (status, inc) = request(
        &app,
        "POST",
        "/api/missing-names/scan",
        json!({"maxEntitiesScanned": 4, "offset": 0, "limit": 2}),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{inc}");
    assert_eq!(inc["discovery"]["completeness"], "incomplete");
    assert!(inc["discovery"]["counts"]["total_candidates"].is_null());
    assert!(inc["discovery"]["counts"]["total_by_kind"].is_null());
    assert_eq!(inc["items"].as_array().unwrap().len(), 2);

    // Complete paginated scan: full totals present and pages disjoint.
    let (status, page1) = request(
        &app,
        "POST",
        "/api/missing-names/scan",
        json!({"maxEntitiesScanned": null, "offset": 0, "limit": 4}),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(page1["discovery"]["completeness"], "complete");
    assert_eq!(page1["discovery"]["counts"]["total_candidates"], 10);
    assert_eq!(page1["discovery"]["has_more_in_scan"], true);
    assert_eq!(page1["items"].as_array().unwrap().len(), 4);

    let (status, page2) = request(
        &app,
        "POST",
        "/api/missing-names/scan",
        json!({"maxEntitiesScanned": null, "offset": 4, "limit": 4}),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(page2["items"].as_array().unwrap().len(), 4);
    let p1_ids: Vec<_> = page1["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|i| i["candidate"]["target"].clone())
        .collect();
    for item in page2["items"].as_array().unwrap() {
        assert!(!p1_ids.contains(&item["candidate"]["target"]));
    }
}

#[tokio::test]
async fn q16_reevaluate_recomputes_invalidated_children_without_mutating_project() {
    let mut project = Project::new(Language("en-US".into()));
    project.info.name = "Named".into();
    project.installations = vec![Installation {
        id: InstallationId(0),
        name: "Inst".into(),
        completion: CompletionStatus::Undefined,
        default_line: None,
        multicast_address: None,
        parameters: vec![],
        topology: Topology {
            areas: vec![],
            lines: vec![],
            unassigned: vec![],
        },
        group_ranges: vec![],
        group_addresses: vec![],
        buildings: vec![
            BuildingPart {
                id: BuildingPartId(1),
                source: src("BP-1"),
                name: String::new(),
                kind: BuildingPartType::Floor,
                parent: None,
                number: None,
                default_line: None,
                completion: CompletionStatus::Undefined,
                children: vec![BuildingPartId(2)],
                devices: vec![],
            },
            BuildingPart {
                id: BuildingPartId(2),
                source: src("BP-2"),
                name: String::new(),
                kind: BuildingPartType::Room,
                parent: Some(BuildingPartId(1)),
                number: None,
                default_line: None,
                completion: CompletionStatus::Undefined,
                children: vec![],
                devices: vec![DeviceId(1), DeviceId(2)],
            },
        ],
    }];
    for device in [
        DeviceInstance {
            id: DeviceId(1),
            source: src("D-1"),
            address: Some(IndividualAddress::new(1, 1, 1).unwrap()),
            name: String::new(),
            description: None,
            product_ref: String::new(),
            program_ref: String::new(),
            commissioning: CommissioningState::default(),
            visibility_calculated: true,
            com_objects: vec![],
            binary_data: vec![],
        },
        DeviceInstance {
            id: DeviceId(2),
            source: src("D-2"),
            address: Some(IndividualAddress::new(1, 1, 2).unwrap()),
            name: String::new(),
            description: None,
            product_ref: String::new(),
            program_ref: String::new(),
            commissioning: CommissioningState::default(),
            visibility_calculated: true,
            com_objects: vec![],
            binary_data: vec![],
        },
    ] {
        project.devices.insert(device);
    }
    let (_dir, state, app) = state_with_project(project);
    let before = state.project.lock().unwrap().clone();
    let (_, scan) = request(&app, "POST", "/api/missing-names/scan", json!({})).await;

    // Draft parent rename on BuildingPart(1) + manual child edit on Device(2) kept by caller.
    let (status, reeval) = request(
        &app,
        "POST",
        "/api/missing-names/reevaluate",
        json!({
            "serverIncarnation": scan["serverIncarnation"],
            "projectIncarnation": scan["projectIncarnation"],
            "snapshotRevision": scan["snapshotRevision"],
            "changedParents": [{"kind": "building_part", "id": 1}],
            "draftParents": [
                {"target": {"kind": "building_part", "id": 1}, "name": "Ground Floor"},
                {"target": {"kind": "device", "id": 2}, "name": "My Manual Device"}
            ]
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "reevaluate: {reeval}");
    let invalidated = reeval["invalidated_targets"].as_array().unwrap();
    assert!(invalidated.contains(&json!({"kind": "building_part", "id": 2})));
    assert!(invalidated.contains(&json!({"kind": "device", "id": 1})));
    // Live project must remain completely untouched.
    assert_eq!(state.project.lock().unwrap().clone(), before);
    // Device 2's manual edit is already non-blank in the cloned draft, so reevaluation does
    // not clobber it with a fresh automatic proposal.
    let proposal_targets: Vec<_> = reeval["proposals"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| p["candidate"]["target"].clone())
        .collect();
    assert!(!proposal_targets.contains(&json!({"kind": "device", "id": 2})));

    // Stale snapshotRevision rejected with 409 missingNameStale.
    let (status, stale) = request(
        &app,
        "POST",
        "/api/missing-names/reevaluate",
        json!({
            "serverIncarnation": scan["serverIncarnation"],
            "projectIncarnation": scan["projectIncarnation"],
            "snapshotRevision": scan["snapshotRevision"].as_u64().unwrap() + 1,
            "changedParents": [],
            "draftParents": []
        }),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(stale["kind"], "missingNameStale");
}

#[tokio::test]
async fn preview_immutability_cancel_unconfirmed_replaced_and_replay_rejection() {
    let (_dir, state, app) = state_with_project(full_q9_project());
    let before = state.project.lock().unwrap().clone();
    let (_, scan) = request(&app, "POST", "/api/missing-names/scan", json!({})).await;

    let make_preview = |name: &str| {
        json!({
            "serverIncarnation": scan["serverIncarnation"],
            "projectIncarnation": scan["projectIncarnation"],
            "snapshotRevision": scan["snapshotRevision"],
            "entries": [{"target": {"kind": "project"}, "name": name}]
        })
    };

    let (s1, p1) = request(
        &app,
        "POST",
        "/api/missing-names/preview",
        make_preview("First"),
    )
    .await;
    assert_eq!(s1, StatusCode::OK);

    // Unconfirmed apply refused with 422 missingNameConfirmationRequired.
    let (status, refused) = request(
        &app,
        "POST",
        "/api/missing-names/apply",
        json!({"token": p1["token"], "confirmed": false}),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(refused["kind"], "missingNameConfirmationRequired");
    assert_eq!(state.project.lock().unwrap().clone(), before);

    // Preview 2 replaces Preview 1.
    let (s2, p2) = request(
        &app,
        "POST",
        "/api/missing-names/preview",
        make_preview("Second"),
    )
    .await;
    assert_eq!(s2, StatusCode::OK);
    assert_ne!(p1["token"], p2["token"]);

    let (status, stale1) = request(
        &app,
        "POST",
        "/api/missing-names/apply",
        json!({"token": p1["token"], "confirmed": true}),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(stale1["kind"], "missingNameStale");

    // Cancel with old token does not cancel Preview 2.
    let (status, c1) = request(
        &app,
        "POST",
        "/api/missing-names/cancel",
        json!({"token": p1["token"]}),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(c1["cancelled"], false);

    // Cancel with Preview 2's token forgets it.
    let (status, c2) = request(
        &app,
        "POST",
        "/api/missing-names/cancel",
        json!({"token": p2["token"]}),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(c2["cancelled"], true);

    let (status, stale2) = request(
        &app,
        "POST",
        "/api/missing-names/apply",
        json!({"token": p2["token"], "confirmed": true}),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(stale2["kind"], "missingNameStale");

    // Preview 3 succeeds once, then replay fails with 409.
    let (s3, p3) = request(
        &app,
        "POST",
        "/api/missing-names/preview",
        make_preview("Third"),
    )
    .await;
    assert_eq!(s3, StatusCode::OK);
    let (status, ok3) = request(
        &app,
        "POST",
        "/api/missing-names/apply",
        json!({"token": p3["token"], "confirmed": true}),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{ok3}");
    let (status, replay) = request(
        &app,
        "POST",
        "/api/missing-names/apply",
        json!({"token": p3["token"], "confirmed": true}),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(replay["kind"], "missingNameStale");
}

#[tokio::test]
async fn selection_cap_256_passes_257_and_empty_noop_reject_with_zero_writes() {
    let mut project = Project::new(Language("en-US".into()));
    project.info.name = "Named".into();
    let gas: Vec<_> = (1..=257)
        .map(|i| GroupAddressEntry {
            id: GroupAddressId(i),
            source: src(&format!("GA-{i}")),
            name: String::new(),
            address: GroupAddress::from_raw(i as u16),
            central: false,
            unfiltered: false,
            range: None,
            declared_dpt: Override::Absent,
            description: None,
        })
        .collect();
    project.installations = vec![Installation {
        id: InstallationId(0),
        name: "Inst".into(),
        completion: CompletionStatus::Undefined,
        default_line: None,
        multicast_address: None,
        parameters: vec![],
        topology: Topology {
            areas: vec![],
            lines: vec![],
            unassigned: vec![],
        },
        group_ranges: vec![],
        group_addresses: gas,
        buildings: vec![],
    }];
    // Loading must not repair counters while deriving the preview's snapshot identity.
    project.ids = knx_core::project::IdAllocators::from_counts(0, 0, 0, 0, 0, 257, 0, 0, 0);
    let (_dir, state, app) = state_with_project(project);
    let before = state.project.lock().unwrap().clone();
    let (_, scan) = request(&app, "POST", "/api/missing-names/scan", json!({})).await;

    // Empty selection (no-op) refused with 422, 0 writes, 0 undo.
    let (status, empty_err) = request(
        &app,
        "POST",
        "/api/missing-names/preview",
        json!({
            "serverIncarnation": scan["serverIncarnation"],
            "projectIncarnation": scan["projectIncarnation"],
            "snapshotRevision": scan["snapshotRevision"],
            "entries": []
        }),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(empty_err["kind"], "missingNameSelection");

    // 257 entries refused with 422, 0 writes, 0 undo.
    let entries_257: Vec<Value> = (1..=257)
        .map(|i| json!({"target": {"kind": "group_address", "id": i}, "name": format!("GA {i}")}))
        .collect();
    let (status, over_err) = request(
        &app,
        "POST",
        "/api/missing-names/preview",
        json!({
            "serverIncarnation": scan["serverIncarnation"],
            "projectIncarnation": scan["projectIncarnation"],
            "snapshotRevision": scan["snapshotRevision"],
            "entries": entries_257
        }),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(over_err["kind"], "missingNameSelection");
    assert_eq!(state.project.lock().unwrap().clone(), before);
    assert_eq!(
        state.command_stack.lock().unwrap().history_lengths(),
        (0, 0)
    );

    // 256 entries pass preview and apply in one undo step.
    let entries_256: Vec<Value> = (1..=256)
        .map(|i| json!({"target": {"kind": "group_address", "id": i}, "name": format!("GA {i}")}))
        .collect();
    let (status, preview) = request(
        &app,
        "POST",
        "/api/missing-names/preview",
        json!({
            "serverIncarnation": scan["serverIncarnation"],
            "projectIncarnation": scan["projectIncarnation"],
            "snapshotRevision": scan["snapshotRevision"],
            "entries": entries_256
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{preview}");
    assert_eq!(preview["preview"]["changed"], 256);
    let (status, applied) = request(
        &app,
        "POST",
        "/api/missing-names/apply",
        json!({"token": preview["token"], "confirmed": true}),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{applied}");
    assert_eq!(
        state.command_stack.lock().unwrap().history_lengths(),
        (1, 0)
    );
}

#[tokio::test]
async fn native_knxdb_save_apply_undo_reopen_and_redo() {
    let (dir, state, app) = state_with_project(full_q9_project());
    let native = dir.path().join("missing-names.knxdb");
    let (status, saved) = request(
        &app,
        "POST",
        "/api/project/save-as",
        json!({"path": native}),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "save-as: {saved}");

    let (status, scan) = request(&app, "POST", "/api/missing-names/scan", json!({})).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(scan["persistence"], "native-working-history");

    let (status, preview) = request(
        &app,
        "POST",
        "/api/missing-names/preview",
        json!({
            "serverIncarnation": scan["serverIncarnation"],
            "projectIncarnation": scan["projectIncarnation"],
            "snapshotRevision": scan["snapshotRevision"],
            "entries": [
                {"target": {"kind": "project"}, "name": "Saved Project"},
                {"target": {"kind": "group_address", "id": 1}, "name": "Saved GA"}
            ]
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(preview["persistence"], "native-working-history");

    let (status, applied) = request(
        &app,
        "POST",
        "/api/missing-names/apply",
        json!({"token": preview["token"], "confirmed": true}),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "apply: {applied}");
    assert_eq!(applied["persistence"], "native-working-history");
    assert!(applied["nativeGeneration"].as_i64().is_some());

    // Undo persisted in working history.
    assert_eq!(
        request(&app, "POST", "/api/undo", Value::Null).await.0,
        StatusCode::OK
    );
    assert_eq!(
        state.project.lock().unwrap().as_ref().unwrap().info.name,
        ""
    );

    // Reopen native project in fresh AppState and verify Redo restores the batch.
    let reopened_state = Arc::new(knx_server::AppState::new(dir.path().to_path_buf()));
    let reopened_app = knx_server::app(reopened_state.clone(), None);
    let (status, opened) = request(
        &reopened_app,
        "POST",
        "/api/project/open",
        json!({"path": native}),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "reopen: {opened}");
    assert_eq!(
        reopened_state
            .project
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .info
            .name,
        ""
    );
    assert_eq!(
        request(&reopened_app, "POST", "/api/redo", Value::Null)
            .await
            .0,
        StatusCode::OK
    );
    let guard = reopened_state.project.lock().unwrap();
    let p = guard.as_ref().unwrap();
    assert_eq!(p.info.name, "Saved Project");
    assert_eq!(p.installations[0].group_addresses[0].name, "Saved GA");
}

#[tokio::test]
async fn external_knxdb_concurrent_conflict_and_readonly_failure_publish_nothing() {
    // Part A: external concurrent modification -> 409 missingNameStale, zero in-memory mutation.
    let (dir, state, app) = state_with_project(full_q9_project());
    let native = dir.path().join("concurrent.knxdb");
    let (status, _) = request(
        &app,
        "POST",
        "/api/project/save-as",
        json!({"path": native}),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (_, scan) = request(&app, "POST", "/api/missing-names/scan", json!({})).await;
    let (_, preview) = request(
        &app,
        "POST",
        "/api/missing-names/preview",
        json!({
            "serverIncarnation": scan["serverIncarnation"],
            "projectIncarnation": scan["projectIncarnation"],
            "snapshotRevision": scan["snapshotRevision"],
            "entries": [{"target": {"kind": "project"}, "name": "Will Conflict"}]
        }),
    )
    .await;

    // Simulate external writer advancing generation in SQLite.
    {
        let conn = knx_store::project_history::open_editor_store(&native, false).unwrap();
        conn.execute(
            "UPDATE project_history_state SET generation = generation + 1 WHERE id = 0",
            [],
        )
        .unwrap();
    }

    let before = state.project.lock().unwrap().clone();
    let (status, conflict) = request(
        &app,
        "POST",
        "/api/missing-names/apply",
        json!({"token": preview["token"], "confirmed": true}),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT, "conflict: {conflict}");
    assert_eq!(conflict["kind"], "missingNameStale");
    assert_eq!(state.project.lock().unwrap().clone(), before);
    assert_eq!(
        state.command_stack.lock().unwrap().history_lengths(),
        (0, 0)
    );

    // Part B: a .knxdb made read-only after the preview -> 409 missingNameReadOnly up front.
    let (dir2, state2, app2) = state_with_project(full_q9_project());
    let ro_native = dir2.path().join("readonly.knxdb");
    let (status, _) = request(
        &app2,
        "POST",
        "/api/project/save-as",
        json!({"path": ro_native}),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (_, scan2) = request(&app2, "POST", "/api/missing-names/scan", json!({})).await;
    assert_eq!(scan2["readOnly"], false);
    let (_, preview2) = request(
        &app2,
        "POST",
        "/api/missing-names/preview",
        json!({
            "serverIncarnation": scan2["serverIncarnation"],
            "projectIncarnation": scan2["projectIncarnation"],
            "snapshotRevision": scan2["snapshotRevision"],
            "entries": [{"target": {"kind": "project"}, "name": "Readonly Fail"}]
        }),
    )
    .await;
    assert_eq!(preview2["readOnly"], false);

    let generation_before = disk_generation(&ro_native);
    let bytes_before = std::fs::read(&ro_native).unwrap();
    std::fs::set_permissions(&ro_native, std::fs::Permissions::from_mode(0o444)).unwrap();
    let before2 = state2.project.lock().unwrap().clone();
    let (status, ro_err) = request(
        &app2,
        "POST",
        "/api/missing-names/apply",
        json!({"token": preview2["token"], "confirmed": true}),
    )
    .await;
    let bytes_after = std::fs::read(&ro_native).unwrap();
    // Restore writable permissions so tempdir cleanup always succeeds.
    let _ = std::fs::set_permissions(&ro_native, std::fs::Permissions::from_mode(0o644));
    assert_eq!(status, StatusCode::CONFLICT, "ro_err: {ro_err}");
    assert_eq!(ro_err["kind"], "missingNameReadOnly");
    assert_eq!(state2.project.lock().unwrap().clone(), before2);
    assert_eq!(
        state2.command_stack.lock().unwrap().history_lengths(),
        (0, 0)
    );
    assert_eq!(bytes_after, bytes_before);
    assert_eq!(disk_generation(&ro_native), generation_before);
}

fn disk_generation(native: &std::path::Path) -> i64 {
    let conn = knx_store::project_history::open_editor_store(native, false).unwrap();
    conn.query_row(
        "SELECT generation FROM project_history_state WHERE id = 0",
        [],
        |row| row.get(0),
    )
    .unwrap()
}

/// Opens `name` in a fresh state, saves it as `.knxdb` and makes the file read-only.
async fn read_only_native(
    name: &str,
) -> (
    tempfile::TempDir,
    Arc<knx_server::AppState>,
    axum::Router,
    std::path::PathBuf,
) {
    let (dir, state, app) = state_with_project(full_q9_project());
    let native = dir.path().join(name);
    let (status, _) = request(
        &app,
        "POST",
        "/api/project/save-as",
        json!({"path": native}),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    std::fs::set_permissions(&native, std::fs::Permissions::from_mode(0o444)).unwrap();
    (dir, state, app, native)
}

#[tokio::test]
async fn read_only_native_project_scans_but_refuses_preview_and_apply() {
    let (_dir, state, app, native) = read_only_native("locked.knxdb").await;
    let generation_before = {
        let _ = std::fs::set_permissions(&native, std::fs::Permissions::from_mode(0o644));
        let generation = disk_generation(&native);
        std::fs::set_permissions(&native, std::fs::Permissions::from_mode(0o444)).unwrap();
        generation
    };
    let bytes_before = std::fs::read(&native).unwrap();
    let before = state.project.lock().unwrap().clone();

    // Scan and review stay possible and report the read-only project.
    let (status, scan) = request(&app, "POST", "/api/missing-names/scan", json!({})).await;
    assert_eq!(status, StatusCode::OK, "scan: {scan}");
    assert_eq!(scan["readOnly"], true);
    assert_eq!(scan["persistence"], "native-working-history");
    assert!(!scan["items"].as_array().unwrap().is_empty());

    let (status, refused) = request(
        &app,
        "POST",
        "/api/missing-names/preview",
        json!({
            "serverIncarnation": scan["serverIncarnation"],
            "projectIncarnation": scan["projectIncarnation"],
            "snapshotRevision": scan["snapshotRevision"],
            "entries": [{"target": {"kind": "project"}, "name": "Never"}]
        }),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT, "preview: {refused}");
    assert_eq!(refused["kind"], "missingNameReadOnly");

    // No token was issued, so a forged apply is refused read-only, not stale.
    let (status, refused) = request(
        &app,
        "POST",
        "/api/missing-names/apply",
        json!({"token": "forged", "confirmed": true}),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT, "apply: {refused}");
    assert_eq!(refused["kind"], "missingNameReadOnly");

    assert_eq!(state.project.lock().unwrap().clone(), before);
    assert_eq!(
        state.command_stack.lock().unwrap().history_lengths(),
        (0, 0)
    );
    assert_eq!(std::fs::read(&native).unwrap(), bytes_before);
    let _ = std::fs::set_permissions(&native, std::fs::Permissions::from_mode(0o644));
    assert_eq!(disk_generation(&native), generation_before);
}

#[tokio::test]
async fn session_only_project_is_never_read_only() {
    let (_dir, _state, app) = state_with_project(full_q9_project());
    let (status, scan) = request(&app, "POST", "/api/missing-names/scan", json!({})).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(scan["persistence"], "session-only");
    assert_eq!(scan["readOnly"], false);
    let (status, preview) = request(
        &app,
        "POST",
        "/api/missing-names/preview",
        json!({
            "serverIncarnation": scan["serverIncarnation"],
            "projectIncarnation": scan["projectIncarnation"],
            "snapshotRevision": scan["snapshotRevision"],
            "entries": [{"target": {"kind": "project"}, "name": "Session"}]
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "preview: {preview}");
    assert_eq!(preview["readOnly"], false);
}

#[tokio::test]
async fn apply_during_save_returns_409_project_saving_and_preserves_token() {
    let (_dir, state, app) = state_with_project(full_q9_project());
    let (_, scan) = request(&app, "POST", "/api/missing-names/scan", json!({})).await;
    let (_, preview) = request(
        &app,
        "POST",
        "/api/missing-names/preview",
        json!({
            "serverIncarnation": scan["serverIncarnation"],
            "projectIncarnation": scan["projectIncarnation"],
            "snapshotRevision": scan["snapshotRevision"],
            "entries": [{"target": {"kind": "project"}, "name": "After Save"}]
        }),
    )
    .await;
    let body = json!({"token": preview["token"], "confirmed": true});
    saving_hold::refused_while_saving(
        &state,
        &app,
        "POST",
        "/api/missing-names/apply",
        body.clone(),
    )
    .await;
    assert_eq!(
        state.project.lock().unwrap().as_ref().unwrap().info.name,
        ""
    );

    let (status, applied) = request(&app, "POST", "/api/missing-names/apply", body).await;
    assert_eq!(status, StatusCode::OK, "apply after save: {applied}");
    assert_eq!(
        state.project.lock().unwrap().as_ref().unwrap().info.name,
        "After Save"
    );
}

#[tokio::test]
async fn session_only_knxproj_apply_and_undo_leave_knxproj_byte_identical() {
    let xml = r#"<KNX xmlns="http://knx.org/xml/project/23"><Project Id="P-0001"><Installations><Installation InstallationId="0"><GroupAddresses><GroupRanges><GroupRange Id="P-0001-0_GR-1" Name="" RangeStart="1" RangeEnd="2047"><GroupAddress Id="P-0001-0_GA-1" Address="1" Name=""/></GroupRange></GroupRanges></GroupAddresses></Installation></Installations></Project></KNX>"#;
    let info = r#"<KNX xmlns="http://knx.org/xml/project/23"><Project Id="P-0001"><ProjectInformation Name=""/></Project></KNX>"#;
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("synthetic.knxproj");
    let original_bytes = zip_with_entries(&[
        ("P-0001.signature", b""),
        ("P-0001/0.xml", xml.as_bytes()),
        ("P-0001/Project.xml", info.as_bytes()),
    ]);
    std::fs::write(&path, &original_bytes).unwrap();
    let state = Arc::new(knx_server::AppState::new(dir.path().to_path_buf()));
    let app = knx_server::app(state.clone(), None);
    let (status, imported) = request(
        &app,
        "POST",
        "/api/project/import",
        json!({"path": path, "clientToken": "missing-names-import"}),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "import: {imported}");

    let (status, scan) = request(&app, "POST", "/api/missing-names/scan", json!({})).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(scan["persistence"], "session-only");
    let (status, preview) = request(
        &app,
        "POST",
        "/api/missing-names/preview",
        json!({
            "serverIncarnation": scan["serverIncarnation"],
            "projectIncarnation": scan["projectIncarnation"],
            "snapshotRevision": scan["snapshotRevision"],
            "entries": [{"target": {"kind": "project"}, "name": "Session Only Name"}]
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let (status, applied) = request(
        &app,
        "POST",
        "/api/missing-names/apply",
        json!({"token": preview["token"], "confirmed": true}),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(applied["persistence"], "session-only");
    assert!(applied["nativeGeneration"].is_null());
    assert_eq!(
        request(&app, "POST", "/api/undo", Value::Null).await.0,
        StatusCode::OK
    );
    assert_eq!(std::fs::read(&path).unwrap(), original_bytes);
}

#[tokio::test]
async fn stale_snapshot_conflict_and_auth_guard_refuse_requests() {
    let (_dir, state, app) = state_with_project(full_q9_project());
    let (_, scan) = request(&app, "POST", "/api/missing-names/scan", json!({})).await;

    // Preview with stale snapshotRevision -> 409 missingNameStale.
    let (status, stale_prev) = request(
        &app,
        "POST",
        "/api/missing-names/preview",
        json!({
            "serverIncarnation": scan["serverIncarnation"],
            "projectIncarnation": scan["projectIncarnation"],
            "snapshotRevision": scan["snapshotRevision"].as_u64().unwrap() + 99,
            "entries": [{"target": {"kind": "project"}, "name": "Stale"}]
        }),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(stale_prev["kind"], "missingNameStale");

    // Edit after valid preview -> apply fails with 409 missingNameStale.
    let (_, preview) = request(
        &app,
        "POST",
        "/api/missing-names/preview",
        json!({
            "serverIncarnation": scan["serverIncarnation"],
            "projectIncarnation": scan["projectIncarnation"],
            "snapshotRevision": scan["snapshotRevision"],
            "entries": [{"target": {"kind": "project"}, "name": "Before Edit"}]
        }),
    )
    .await;
    state
        .project_revision
        .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let (status, stale_apply) = request(
        &app,
        "POST",
        "/api/missing-names/apply",
        json!({"token": preview["token"], "confirmed": true}),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(stale_apply["kind"], "missingNameStale");

    // Auth guard: all 5 routes require authentication when auth is enabled.
    let credential = "synthetic missing-name auth fixture";
    let hash = knx_server::hash_password_with_iterations(credential, 1000).unwrap();
    let auth = knx_server::AuthConfig::from_password_hash(&hash).unwrap();
    let guarded = knx_server::app_with_auth(state.clone(), None, auth);
    for path in [
        "/api/missing-names/scan",
        "/api/missing-names/reevaluate",
        "/api/missing-names/preview",
        "/api/missing-names/apply",
        "/api/missing-names/cancel",
    ] {
        let (status, _) = request(&guarded, "POST", path, json!({})).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED, "{path} must require auth");
    }
}
