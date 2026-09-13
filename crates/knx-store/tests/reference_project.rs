//! The design spec's headline acceptance test: import the real reference
//! `.knxproj`, `save_project` it, `load_project` it back, and assert
//! `Project == Project` (spec §Testing, "Round trip").
//!
//! It lives here rather than in `project.rs`'s unit tests because it needs
//! `knx-etsproj` — a dev-dependency only. `knx-app`'s `import_ets_project`
//! would do as well, but it adds nothing this test needs (opaque entries and
//! the manufacturer manifest, neither of which `save_project` touches) and
//! `knx-app` depends on `knx-store`, so reaching for the importer directly
//! keeps the dependency one-way.
//!
//! Only the ETS4 reference project is used: the ETS 6.3.0 one is schema 23,
//! which `knx-etsproj` deliberately refuses by name rather than misreading
//! (KNOWN_LIMITATIONS §1).

use std::path::PathBuf;

use knx_store::{load_project, open_and_migrate, save_project};

fn reference_ets4_path() -> PathBuf {
    knx_testsupport::reference_ets4_path()
}

#[test]
fn the_reference_project_round_trips_through_save_and_load() {
    if !reference_ets4_path().exists() {
        eprintln!("skip: OriginalData/ corpus not present (gitignored, local-only)");
        return;
    }
    let project = knx_etsproj::import_knxproj(&reference_ets4_path())
        .expect("the reference ETS4 project imports")
        .project;

    // Guard against a vacuous pass: the round trip below only means
    // something if the project actually carries every entity shape.
    assert!(project.devices.iter().count() > 0, "devices");
    assert!(project.installations[0].buildings.len() > 1, "buildings");
    assert!(
        project.installations[0].group_ranges.len() > 1,
        "group ranges"
    );
    assert!(
        !project.installations[0].group_addresses.is_empty(),
        "group addresses"
    );
    assert!(
        !project.installations[0].parameters.is_empty(),
        "parameters"
    );
    assert!(
        project.installations[0]
            .buildings
            .iter()
            .any(|b| b.parent.is_some()),
        "a nested building part"
    );

    let dir = tempfile::tempdir().unwrap();
    let conn = open_and_migrate(&dir.path().join("p.sqlite")).unwrap();
    save_project(&conn, &project).unwrap();
    let loaded = load_project(&conn).unwrap();

    // Compared piecewise first so a failure names the entity area that
    // broke, instead of dumping the whole multi-megabyte `Project` twice.
    assert_eq!(loaded.schema_version, project.schema_version);
    assert_eq!(loaded.info, project.info);
    assert_eq!(loaded.ids, project.ids);
    assert_eq!(loaded.strings, project.strings);
    assert_eq!(loaded.installations.len(), project.installations.len());
    for (a, b) in loaded.installations.iter().zip(&project.installations) {
        assert_eq!(a.id, b.id);
        assert_eq!(a.name, b.name);
        assert_eq!(a.default_line, b.default_line);
        assert_eq!(a.multicast_address, b.multicast_address);
        assert_eq!(a.completion, b.completion);
        assert_eq!(a.topology, b.topology);
        assert_eq!(a.buildings, b.buildings);
        assert_eq!(a.group_ranges, b.group_ranges);
        assert_eq!(a.group_addresses, b.group_addresses);
        assert_eq!(a.parameters, b.parameters);
    }
    for device in project.devices.iter() {
        assert_eq!(loaded.devices.get(device.id), Some(device));
        for com_id in &device.com_objects {
            assert_eq!(
                loaded.devices.com_object(*com_id),
                project.devices.com_object(*com_id)
            );
        }
    }

    // …and the whole graph, which also catches anything the piecewise
    // comparison above forgot to name.
    assert!(
        loaded == project,
        "the reference project did not round-trip losslessly"
    );
}

#[test]
fn saving_the_reference_project_twice_over_itself_is_idempotent() {
    if !reference_ets4_path().exists() {
        eprintln!("skip: OriginalData/ corpus not present (gitignored, local-only)");
        return;
    }
    let project = knx_etsproj::import_knxproj(&reference_ets4_path())
        .expect("the reference ETS4 project imports")
        .project;
    let dir = tempfile::tempdir().unwrap();
    let conn = open_and_migrate(&dir.path().join("p.sqlite")).unwrap();
    save_project(&conn, &project).unwrap();
    save_project(&conn, &project).unwrap();
    assert!(
        load_project(&conn).unwrap() == project,
        "a re-save over existing rows changed the project"
    );
}
