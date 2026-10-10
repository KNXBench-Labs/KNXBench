//! Rejects a native editor save when an unjournalled writer changed its reviewed target.

use knx_core::{
    CommandStack, CompletionStatus, Devices, Installation, InstallationId, Language, Project,
    Topology,
};
use knx_store::project_history::{self as history, NativeSnapshot};

fn snapshot(name: &str) -> NativeSnapshot {
    let mut project = Project::new(Language("en".into()));
    project.info.name = name.into();
    project.devices = Devices::new();
    project.installations.push(Installation {
        id: InstallationId(0),
        name: "Seed installation".into(),
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
    NativeSnapshot {
        project,
        opaque: vec![],
        manufacturer_refs: vec![],
    }
}

#[test]
fn unchanged_generation_does_not_authorize_overwriting_an_unreviewed_saved_root() {
    let conn = knx_store::open_and_migrate_in_memory().unwrap();
    let reviewed = snapshot("Reviewed root");
    knx_store::save_project_with_passthrough(
        &conn,
        &reviewed.project,
        &reviewed.opaque,
        &reviewed.manufacturer_refs,
    )
    .unwrap();
    let newer = snapshot("Concurrent root edit");
    knx_store::save_project_with_passthrough(
        &conn,
        &newer.project,
        &newer.opaque,
        &newer.manufacturer_refs,
    )
    .unwrap();
    let candidate = snapshot("Reviewed import result");
    let result = history::save_editor_if_unchanged(
        &conn,
        &candidate,
        &CommandStack::new(),
        0,
        &reviewed.semantic_hash().unwrap(),
        true,
    );
    assert!(
        result.is_err(),
        "generation zero alone must not authorize an unreviewed target"
    );
    assert_eq!(NativeSnapshot::read(&conn).unwrap(), newer);
}
