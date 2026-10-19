//! Device octet 0 is assignable only with manufacturer coupler evidence.
//!
//! MODEL-03: the KNX Association's offline project check accepts a device
//! address ending in 0 only for a device whose hardware is a coupler
//! (`Hardware/@IsCoupler`, RESEARCH §25). The server reads that flag from the
//! product database; without it the core keeps refusing a *new* `.0`.

use std::sync::Mutex;

use knx_core::{
    Area, AreaId, CommissioningState, CompletionStatus, DeviceId, DeviceInstance,
    IndividualAddress, Installation, InstallationId, Language, Line, LineId, Project, SourceRef,
    Topology,
};
use knx_server::AppState;

const COUPLER: &str = "M-0001_H-C_P-C";
const ORDINARY: &str = "M-0001_H-D_P-D";
const UNINSTALLED: &str = "M-0001_H-X_P-X";

fn source() -> SourceRef {
    SourceRef {
        path: "t".into(),
        ets_id: "t".into(),
    }
}

fn device(id: u32, product_ref: &str, address: Option<IndividualAddress>) -> DeviceInstance {
    DeviceInstance {
        id: DeviceId(id),
        source: source(),
        name: format!("D{id}"),
        description: None,
        address,
        product_ref: product_ref.into(),
        program_ref: String::new(),
        commissioning: CommissioningState::default(),
        visibility_calculated: true,
        com_objects: vec![],
        binary_data: vec![],
    }
}

fn line(id: u32, address: u8, devices: Vec<DeviceId>) -> Line {
    Line {
        id: LineId(id),
        source: source(),
        name: format!("L{address}"),
        address,
        medium_ref: "TP".into(),
        domain_address: None,
        domain_address_is_checked: None,
        ip_routing_multicast_address: None,
        multicast_ttl: None,
        completion: CompletionStatus::FinishedDesign,
        devices,
    }
}

/// Area 1 with lines 1.1 (devices 1–3 and 5) and 1.2 (device 4).
fn project() -> Project {
    let mut project = Project::new(Language("en".into()));
    project.installations.push(Installation {
        id: InstallationId(0),
        name: "I".into(),
        default_line: None,
        multicast_address: None,
        completion: CompletionStatus::FinishedDesign,
        topology: Topology {
            areas: vec![Area {
                id: AreaId(1),
                source: source(),
                name: "A".into(),
                address: 1,
                completion: CompletionStatus::FinishedDesign,
                lines: vec![LineId(1), LineId(2)],
            }],
            lines: vec![
                line(
                    1,
                    1,
                    vec![DeviceId(1), DeviceId(2), DeviceId(3), DeviceId(5)],
                ),
                line(2, 2, vec![DeviceId(4)]),
            ],
            unassigned: vec![],
        },
        buildings: vec![],
        group_ranges: vec![],
        group_addresses: vec![],
        parameters: vec![],
    });
    project.devices.insert(device(1, COUPLER, None));
    project.devices.insert(device(2, ORDINARY, None));
    project.devices.insert(device(3, UNINSTALLED, None));
    project.devices.insert(device(4, COUPLER, None));
    project.devices.insert(device(5, COUPLER, None));
    project
}

fn products(dir: &tempfile::TempDir) -> knx_productdb::Connection {
    let conn = knx_productdb::open_and_migrate(&dir.path().join("p.sqlite")).unwrap();
    conn.execute_batch(
        "INSERT INTO manufacturer (id, name) VALUES ('M-0001', 'M');
         INSERT INTO hardware (id, manufacturer_id, name, is_coupler, source_sha256)
              VALUES ('M-0001_H-C', 'M-0001', 'Line coupler', 1, 'x');
         INSERT INTO hardware (id, manufacturer_id, name, is_coupler, source_sha256)
              VALUES ('M-0001_H-D', 'M-0001', 'Actuator', NULL, 'x');
         INSERT INTO product (id, manufacturer_id, hardware_id, text, source_sha256)
              VALUES ('M-0001_H-C_P-C', 'M-0001', 'M-0001_H-C', 'Line coupler', 'x');
         INSERT INTO product (id, manufacturer_id, hardware_id, text, source_sha256)
              VALUES ('M-0001_H-D_P-D', 'M-0001', 'M-0001_H-D', 'Actuator', 'x');",
    )
    .unwrap();
    conn
}

fn state(with_products: bool) -> (tempfile::TempDir, AppState) {
    let dir = tempfile::tempdir().unwrap();
    let state = AppState {
        product_db: with_products.then(|| Mutex::new(products(&dir))),
        ..AppState::default()
    };
    *state.project.lock().unwrap() = Some(project());
    (dir, state)
}

fn address_of(state: &AppState, device: u32) -> Option<IndividualAddress> {
    state
        .project
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .devices
        .get(DeviceId(device))
        .unwrap()
        .address
}

fn ia(text: &str) -> IndividualAddress {
    text.parse().unwrap()
}

#[test]
fn a_coupler_product_takes_the_lines_zero_address_with_undo_and_redo() {
    let (_dir, state) = state(true);
    knx_server::set_individual_address_impl(&state, 1, Some("1.1.0".into())).unwrap();
    assert_eq!(address_of(&state, 1), Some(ia("1.1.0")));

    knx_server::undo_impl(&state).unwrap();
    assert_eq!(address_of(&state, 1), None);

    knx_server::redo_impl(&state).unwrap();
    assert_eq!(address_of(&state, 1), Some(ia("1.1.0")));
}

#[test]
fn an_ordinary_or_unknown_product_is_still_refused_the_zero_address() {
    let (_dir, state) = state(true);
    for device in [2, 3] {
        let error = knx_server::set_individual_address_impl(&state, device, Some("1.1.0".into()))
            .unwrap_err();
        assert!(error.contains("coupler"), "device {device}: {error}");
        assert_eq!(address_of(&state, device), None);
    }
}

#[test]
fn without_a_product_database_the_zero_address_is_refused() {
    let (_dir, state) = state(false);
    let error =
        knx_server::set_individual_address_impl(&state, 1, Some("1.1.0".into())).unwrap_err();
    assert!(error.contains("coupler"), "{error}");
    assert_eq!(address_of(&state, 1), None);
}

#[test]
fn coupler_evidence_does_not_relax_the_line_prefix_or_uniqueness() {
    let (_dir, state) = state(true);
    let error =
        knx_server::set_individual_address_impl(&state, 1, Some("1.2.0".into())).unwrap_err();
    assert!(error.contains("line 1.1"), "{error}");
    assert_eq!(address_of(&state, 1), None);

    knx_server::set_individual_address_impl(&state, 4, Some("1.2.0".into())).unwrap();
    knx_server::set_individual_address_impl(&state, 1, Some("1.1.0".into())).unwrap();
    // Device 1 now holds 1.1.0; a second coupler on the same line cannot.
    let error =
        knx_server::set_individual_address_impl(&state, 5, Some("1.1.0".into())).unwrap_err();
    assert!(error.contains("already used"), "{error}");
    assert_eq!(address_of(&state, 5), None);
}

#[test]
fn an_ordinary_nonzero_address_does_not_need_the_product_database() {
    let (_dir, state) = state(false);
    knx_server::set_individual_address_impl(&state, 2, Some("1.1.7".into())).unwrap();
    assert_eq!(address_of(&state, 2), Some(ia("1.1.7")));
}
