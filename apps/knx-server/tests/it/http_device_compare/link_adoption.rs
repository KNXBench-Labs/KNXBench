//! Association rows → project links at the production mount, corpus-free.
//!
//! Synthetic `0701h` program (`knx_testsupport::LINK_ADOPTION_PROGRAM_XML`), a
//! saved native project and a simulated device whose association table differs.
//! Compare → preview → apply → re-compare → undo → re-compare, never a write.

use super::*;
use knx_core::{
    ComObjectInstance, ComObjectInstanceId, CompletionStatus, DeviceInstance, Direction,
    GroupAddress, GroupAddressEntry, GroupLink, Installation, InstallationId, Language, Override,
    Project, SourceRef, Topology,
};
use knx_store::project_history::NativeSnapshot;

const TARGET: &str = "1.1.7";
const ADDRESSES: [u8; 11] = [4, 0x11, 0x07, 0x08, 0x01, 0x08, 0x02, 0x08, 0x03, 0, 0];
/// Object 0 sends on 1/0/2; object 1 sends on 1/0/1 and receives 1/0/3.
const ASSOCIATIONS: [u8; 9] = [3, 2, 0, 1, 1, 3, 1, 0, 0];

fn project() -> Project {
    let mut project = Project::new(Language("en".into()));
    let device = project.ids.next_device_id().unwrap();
    project.installations.push(Installation {
        id: InstallationId(0),
        name: "I".into(),
        default_line: None,
        multicast_address: None,
        completion: CompletionStatus::FinishedDesign,
        topology: Topology {
            areas: vec![],
            lines: vec![],
            unassigned: vec![device],
        },
        buildings: vec![],
        group_ranges: vec![],
        group_addresses: vec![],
        parameters: vec![],
    });
    project.devices.insert(DeviceInstance {
        id: device,
        source: SourceRef {
            path: "synthetic".into(),
            ets_id: "D".into(),
        },
        name: "D".into(),
        description: None,
        address: Some(TARGET.parse().unwrap()),
        product_ref: "P".into(),
        program_ref: "HP".into(),
        commissioning: Default::default(),
        visibility_calculated: true,
        com_objects: vec![],
        binary_data: vec![],
    });
    let mut first = None;
    for raw in [0x0801, 0x0802, 0x0803] {
        let id = project.ids.next_group_address_id().unwrap();
        first.get_or_insert(id);
        project.installations[0]
            .group_addresses
            .push(GroupAddressEntry {
                id,
                source: SourceRef {
                    path: "synthetic".into(),
                    ets_id: format!("GA-{}", id.0),
                },
                name: format!("GA {raw:04X}"),
                address: GroupAddress::from_raw(raw),
                central: false,
                unfiltered: false,
                range: None,
                declared_dpt: Default::default(),
            });
    }
    for number in 0..3u16 {
        let id = project.ids.next_com_object_instance_id().unwrap();
        project.devices.insert_com_object(ComObjectInstance {
            id,
            source: SourceRef {
                path: "synthetic".into(),
                ets_id: format!("M-0001_A-0002-01-0000_O-{number}_R-{}", number + 1),
            },
            device,
            number,
            text: Override::Absent,
            description: Override::Absent,
            dpt: Override::Absent,
            flags: knx_core::ResolvedFlags::none(),
            size: None,
            is_active: true,
            links: if number == 0 {
                vec![GroupLink {
                    ga: first.unwrap(),
                    direction: Direction::Send,
                }]
            } else {
                vec![]
            },
            module_instance: None,
        });
        project
            .devices
            .get_mut(device)
            .unwrap()
            .com_objects
            .push(id);
    }
    project
}

async fn link_harness() -> Harness {
    let dir = tempfile::tempdir().unwrap();
    let products = knx_productdb::open_and_migrate(&dir.path().join("p.sqlite")).unwrap();
    knx_productdb::ingest_file(
        &products,
        "program.xml",
        knx_testsupport::LINK_ADOPTION_PROGRAM_XML.as_bytes(),
    )
    .unwrap();
    knx_productdb::ingest_file(
        &products,
        "Hardware.xml",
        knx_testsupport::LINK_ADOPTION_HARDWARE_XML.as_bytes(),
    )
    .unwrap();
    let project = project();
    let device = Arc::new(SimulatedDevice::with_config_at(
        TARGET.parse().unwrap(),
        SimulatorConfig {
            mask_version: 0x0701,
            ..Default::default()
        },
    ));
    device.preset_property(0, PID_MANUFACTURER_ID, &[0, 1]);
    device.preset_property(
        3,
        knx_core::commissioning::properties::PID_PROGRAM_VERSION,
        &[0, 1, 0, 2, 1],
    );
    // The object table and parameter as the project writes them; tables differ.
    let prepared = knx_app::device_download::prepare_device_download(
        &products,
        &project,
        TARGET.parse().unwrap(),
    )
    .unwrap();
    for step in &prepared.plan.steps {
        if let knx_core::commissioning::memory_download::MemoryDownloadStep::WriteMemory {
            address,
            octets,
        } = step
        {
            device.preset_memory(u32::from(*address), octets);
        }
    }
    device.preset_memory(0x4000, &ADDRESSES);
    device.preset_memory(0x4100, &ASSOCIATIONS);
    knx_app::project_history::save_as(
        &dir.path().join("links.knxdb"),
        &NativeSnapshot {
            project,
            opaque: vec![],
            manufacturer_refs: vec![],
        },
        &knx_core::CommandStack::new(),
    )
    .unwrap();
    let calls = Arc::new(AtomicUsize::new(0));
    let state = Arc::new(knx_server::AppState {
        product_db: Some(
            knx_server::CatalogueOwner::adopt_opened(products, std::path::PathBuf::new()).unwrap(),
        ),
        data_dir: dir.path().to_path_buf(),
        connector: Box::new(SimConnector {
            device: Arc::clone(&device),
            calls: Arc::clone(&calls),
        }),
        device_download_timing: fast(),
        ..knx_server::AppState::new(dir.path().to_path_buf())
    });
    let app = knx_server::app(Arc::clone(&state), None);
    let (status, body) = send(
        &app,
        post("/api/project/open", json!({ "path": "links.knxdb" })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    Harness {
        _dir: dir,
        app,
        state,
        device,
        calls,
    }
}

async fn compare_links(h: &Harness, cookie: Option<&str>) -> (String, Value) {
    let body = json!({ "address": TARGET, "gateway": GATEWAY });
    let request = match cookie {
        Some(cookie) => browser_post("/api/device-compare", body, cookie),
        None => post("/api/device-compare", body),
    };
    let response = h.app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let cookie = match cookie {
        Some(cookie) => cookie.to_owned(),
        None => response.headers()["set-cookie"]
            .to_str()
            .unwrap()
            .split(';')
            .next()
            .unwrap()
            .to_owned(),
    };
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    (cookie, serde_json::from_slice(&bytes).unwrap())
}

fn rows<'a>(compared: &'a Value, kind: &str) -> Vec<&'a Value> {
    compared["decoded"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|r| r["kind"] == kind)
        .collect()
}

fn indices(rows: &[&Value]) -> Vec<Value> {
    rows.iter().map(|r| r["index"].clone()).collect()
}

#[tokio::test]
async fn association_rows_adopt_links_at_the_mount_undo_and_never_write() {
    let h = link_harness().await;
    let (cookie, compared) = compare_links(&h, None).await;
    let associations = rows(&compared, "association");
    assert_eq!(associations.len(), 3);
    assert!(associations.iter().all(|r| r["adoptable"] == true));
    for kind in ["addressCount", "groupAddress", "associationCount", "raw"] {
        assert!(rows(&compared, kind)
            .iter()
            .all(|r| r["adoptable"] == false));
    }
    let token = compared["observationToken"].as_str().unwrap().to_owned();
    let (_, before) = send(&h.app, get("/api/project")).await;
    // A mixed selection with an address-table row refuses as a whole.
    let mut mixed = indices(&associations);
    mixed.push(rows(&compared, "groupAddress")[0]["index"].clone());
    let (status, body) = send(
        &h.app,
        browser_post(
            "/api/device-compare/preview",
            json!({ "observationToken": token, "selected": mixed }),
            &cookie,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{body}");
    assert_eq!(body["kind"], "deviceAdoptionRefused");
    // An incomplete object selection refuses with the missing row named.
    let (status, body) = send(
        &h.app,
        browser_post(
            "/api/device-compare/preview",
            json!({ "observationToken": token, "selected": [associations[1]["index"]] }),
            &cookie,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{body}");
    assert!(body["error"]
        .as_str()
        .unwrap()
        .contains("select every differing association row"));
    assert_eq!(send(&h.app, get("/api/project")).await.1, before);
    let (status, shown) = send(
        &h.app,
        browser_post(
            "/api/device-compare/preview",
            json!({ "observationToken": token, "selected": indices(&associations) }),
            &cookie,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{shown}");
    assert_eq!(shown["changed"], true);
    assert_eq!(shown["rows"], compared["decoded"]);
    let changes: Vec<_> = shown["linkChanges"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| {
            (
                c["add"].as_bool().unwrap(),
                c["object"].as_u64().unwrap(),
                c["groupAddress"].as_str().unwrap().to_owned(),
                c["sending"].as_bool().unwrap(),
            )
        })
        .collect();
    assert_eq!(
        changes,
        vec![
            (false, 0, "1/0/1".to_owned(), true),
            (true, 0, "1/0/2".to_owned(), true),
            (true, 1, "1/0/1".to_owned(), true),
            (true, 1, "1/0/3".to_owned(), false),
        ]
    );
    assert_eq!(send(&h.app, get("/api/project")).await.1, before);
    let lengths = h.state.command_stack.lock().unwrap().history_lengths();
    let (status, applied) = send(
        &h.app,
        browser_post(
            "/api/device-compare/apply",
            json!({ "previewToken": shown["previewToken"], "confirmation": "AdoptDeviceComparison" }),
            &cookie,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{applied}");
    assert_eq!(applied["changed"], true);
    assert_eq!(
        h.state.command_stack.lock().unwrap().history_lengths(),
        (lengths.0 + 1, 0)
    );
    let groups = |h: &Harness| {
        h.state
            .project
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .installations[0]
            .group_addresses
            .len()
    };
    assert_eq!(groups(&h), 3, "no group address created");
    let (_, adopted) = compare_links(&h, Some(&cookie)).await;
    for kind in [
        "addressCount",
        "groupAddress",
        "associationCount",
        "association",
    ] {
        for row in rows(&adopted, kind) {
            assert_eq!(row["project"], row["device"], "{kind} re-compares equal");
            assert!(!row["project"].is_null());
        }
    }
    let (status, body) = send(&h.app, post("/api/undo", json!({}))).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let object_one = h
        .state
        .project
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .devices
        .com_object(ComObjectInstanceId(2))
        .unwrap()
        .links
        .clone();
    assert!(object_one.is_empty(), "one undo restores the links");
    let (_, undone) = compare_links(&h, Some(&cookie)).await;
    assert_eq!(
        rows(&undone, "association")
            .iter()
            .map(|r| r["project"].clone())
            .collect::<Vec<_>>(),
        associations
            .iter()
            .map(|r| r["project"].clone())
            .collect::<Vec<_>>()
    );
    assert!(no_writes(&h.device));
    assert!(!h.device.memory_was_written());
}
