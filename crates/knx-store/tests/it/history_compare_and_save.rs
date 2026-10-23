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

#[test]
fn acquired_sources_share_native_save_rollback_and_generation_admission() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("sources.knxdb");
    let conn = knx_store::open_and_migrate(&path).unwrap();
    let mut original = snapshot("Original");
    original.opaque.push(knx_store::StoredOpaqueEntry {
        source_path: "M-0001/Baggages/original.bin".into(),
        xpath: String::new(),
        kind: "Baggage".into(),
        name: String::new(),
        bytes: b"opaque baggage".to_vec(),
        sha256: knx_store::manufacturer_file_hash(b"opaque baggage"),
    });
    let stack = CommandStack::new();
    let generation = history::save_editor(&conn, &original, &stack, None, true).unwrap();
    let files = vec![
        knx_store::CarriedManufacturerFile {
            source_path: "catalogue-packages/first.knxprod".into(),
            bytes: b"first original".to_vec(),
        },
        knx_store::CarriedManufacturerFile {
            source_path: "M-0001/Hardware.xml".into(),
            bytes: b"second source".to_vec(),
        },
    ];
    let mut candidate = original.clone();
    candidate.project.info.name = "Candidate".into();
    for source in &files {
        candidate
            .manufacturer_refs
            .push(knx_store::ManufacturerRef {
                source_path: source.source_path.clone(),
                sha256: knx_store::manufacturer_file_hash(&source.bytes),
                len: source.bytes.len() as i64,
                kind: "ManufacturerData".into(),
            });
    }
    let versions = history::list_versions(&conn).unwrap().len();
    let stale =
        history::save_editor_with_sources(&conn, &candidate, &stack, Some(generation + 1), &files);
    assert!(matches!(stale, Err(history::HistoryError::Stale)));
    assert_eq!(NativeSnapshot::read(&conn).unwrap(), original);
    assert!(
        knx_store::load_manufacturer_file(&conn, &candidate.manufacturer_refs[0].sha256)
            .unwrap()
            .is_none()
    );
    let mut invalid = candidate.clone();
    invalid.manufacturer_refs[1].len += 1;
    let refusal =
        history::save_editor_with_sources(&conn, &invalid, &stack, Some(generation), &files);
    assert!(matches!(
        refusal,
        Err(history::HistoryError::Invalid(
            "acquired source is absent from the saved manifest"
        ))
    ));
    assert_eq!(NativeSnapshot::read(&conn).unwrap(), original);
    assert_eq!(
        history::load_editor(&conn).unwrap().unwrap().generation,
        generation
    );
    assert_eq!(history::list_versions(&conn).unwrap().len(), versions);
    assert!(
        knx_store::load_manufacturer_file(&conn, &candidate.manufacturer_refs[0].sha256)
            .unwrap()
            .is_none(),
        "first payload must roll back when second write fails"
    );
    history::save_editor_with_sources(&conn, &candidate, &stack, Some(generation), &files).unwrap();
    drop(conn);
    let reopened = knx_store::open_existing_read_only(&path).unwrap();
    assert_eq!(NativeSnapshot::read(&reopened.conn).unwrap(), candidate);
    for source in &files {
        assert_eq!(
            knx_store::load_manufacturer_file(
                &reopened.conn,
                &knx_store::manufacturer_file_hash(&source.bytes)
            )
            .unwrap()
            .unwrap()
            .bytes,
            source.bytes
        );
    }
    assert_eq!(
        knx_store::load_opaque(&reopened.conn).unwrap(),
        original.opaque
    );
}
