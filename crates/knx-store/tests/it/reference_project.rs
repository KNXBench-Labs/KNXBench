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
//! All three corpus projects are covered: ETS4 (schema 11), the KV demo
//! (schema 21) and ETS 6.3.0 (schema 23). The ETS4-only scope dated from
//! when schema 23 was refused by name; that import has since been admitted,
//! so its project must survive the native round trip losslessly too.

use std::path::PathBuf;

use knx_store::{load_project, open_and_migrate, save_project};

fn reference_ets4_path() -> PathBuf {
    knx_testsupport::reference_ets4_path()
}

#[test]
#[ignore = "requires the gitignored OriginalData/ corpus; run with --ignored"]
fn the_reference_project_round_trips_through_save_and_load() {
    assert!(
        reference_ets4_path().exists(),
        "OriginalData/ corpus not present (gitignored, local-only); this test is #[ignore]d and must be run explicitly on a machine that has it"
    );
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
    assert_round_trips(&project, "ETS4");
}

/// The schema-21 KV demo and the schema-23 ETS 6.3.0 export of the same
/// house as the ETS4 file: both import, so both must reopen identical.
#[test]
#[ignore = "requires the gitignored OriginalData/ corpus; run with --ignored"]
fn the_schema_21_and_23_reference_projects_round_trip_through_save_and_load() {
    for (path, label) in [
        (
            knx_testsupport::reference_kv_schema21_path(),
            "KV schema 21",
        ),
        (
            knx_testsupport::reference_ets6_path(),
            "ETS 6.3.0 schema 23",
        ),
    ] {
        assert!(
            path.exists(),
            "OriginalData/ corpus not present (gitignored, local-only); this test is #[ignore]d and must be run explicitly on a machine that has it"
        );
        let project = knx_etsproj::import_knxproj(&path)
            .unwrap_or_else(|error| panic!("{label} imports: {error:?}"))
            .project;
        assert!(project.devices.iter().count() > 0, "{label}: devices");
        assert!(
            project
                .installations
                .iter()
                .any(|i| !i.group_addresses.is_empty()),
            "{label}: group addresses"
        );
        assert_round_trips(&project, label);
    }
}

fn assert_round_trips(project: &knx_core::Project, label: &str) {
    let dir = tempfile::tempdir().unwrap();
    let conn = open_and_migrate(&dir.path().join("p.sqlite")).unwrap();
    save_project(&conn, project).unwrap_or_else(|error| panic!("{label}: save: {error}"));
    let loaded = load_project(&conn).unwrap();

    // Compared piecewise first so a failure names the entity area that
    // broke, instead of dumping the whole multi-megabyte `Project` twice.
    assert_eq!(loaded.schema_version, project.schema_version, "{label}");
    assert_eq!(loaded.info, project.info, "{label}");
    assert_eq!(loaded.ids, project.ids, "{label}");
    assert_eq!(loaded.strings, project.strings, "{label}");
    assert_eq!(
        loaded.installations.len(),
        project.installations.len(),
        "{label}"
    );
    for (a, b) in loaded.installations.iter().zip(&project.installations) {
        assert_eq!(a.id, b.id, "{label}");
        assert_eq!(a.name, b.name, "{label}");
        assert_eq!(a.default_line, b.default_line, "{label}");
        assert_eq!(a.multicast_address, b.multicast_address, "{label}");
        assert_eq!(a.completion, b.completion, "{label}");
        assert_eq!(a.topology, b.topology, "{label}");
        assert_eq!(a.buildings, b.buildings, "{label}");
        assert_eq!(a.group_ranges, b.group_ranges, "{label}");
        assert_eq!(a.group_addresses, b.group_addresses, "{label}");
        assert_eq!(a.parameters, b.parameters, "{label}");
    }
    for device in project.devices.iter() {
        assert_eq!(loaded.devices.get(device.id), Some(device), "{label}");
        for com_id in &device.com_objects {
            assert_eq!(
                loaded.devices.com_object(*com_id),
                project.devices.com_object(*com_id),
                "{label}"
            );
        }
    }

    // …and the whole graph, which also catches anything the piecewise
    // comparison above forgot to name.
    assert!(
        &loaded == project,
        "{label}: the reference project did not round-trip losslessly"
    );
}

#[test]
#[ignore = "requires the gitignored OriginalData/ corpus; run with --ignored"]
fn saving_the_reference_project_twice_over_itself_is_idempotent() {
    assert!(
        reference_ets4_path().exists(),
        "OriginalData/ corpus not present (gitignored, local-only); this test is #[ignore]d and must be run explicitly on a machine that has it"
    );
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
