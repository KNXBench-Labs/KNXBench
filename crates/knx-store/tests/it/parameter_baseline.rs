//! Native 20 parameter baselines: migration, refusal, settlement atomicity, images and pruning.

use std::path::{Path, PathBuf};
use std::time::Instant;

use chrono::{DateTime, Utc};
use knx_core::commissioning::parameter_baseline::{
    BaselineEdit, ParameterBaseline, ParameterBaselines,
};
use knx_core::commissioning::pending::{ProgrammingChange, ProgrammingParts};
use knx_core::project::IdAllocators;
use knx_core::{
    CommandStack, CommissioningState, CompletionStatus, DeviceId, DeviceInstance, Installation,
    InstallationId, Language, Project, SourceRef, Topology,
};
use knx_store::project_history::{
    self as history, JournalTrust, JournalWriteReport, NativeSnapshot,
};
use rusqlite::{types::Value, Connection, OpenFlags};

const PROGRAM: &str = "M-0001_A-0001-01-0000";

fn project(id: &str, devices: u32) -> Project {
    let mut project = Project::new(Language("en".into()));
    project.info.project_id = id.into();
    project.installations.push(Installation {
        id: InstallationId(0),
        name: "I".into(),
        default_line: None,
        multicast_address: None,
        completion: CompletionStatus::FinishedDesign,
        topology: Topology {
            areas: vec![],
            lines: vec![],
            unassigned: (1..=devices).map(DeviceId).collect(),
        },
        buildings: vec![],
        group_ranges: vec![],
        group_addresses: vec![],
        parameters: vec![],
    });
    for id in 1..=devices {
        project.devices.insert(DeviceInstance {
            id: DeviceId(id),
            source: SourceRef {
                path: "t".into(),
                ets_id: format!("D-{id}"),
            },
            name: format!("Device {id}"),
            description: None,
            address: None,
            product_ref: String::new(),
            program_ref: String::new(),
            commissioning: CommissioningState::default(),
            visibility_calculated: true,
            com_objects: vec![],
            binary_data: vec![],
        });
    }
    project.ids = IdAllocators::from_counts(devices, 0, 0, 0, 0, 0, 0, 0, 0);
    project
}

fn model(project: &Project) -> NativeSnapshot {
    NativeSnapshot {
        project: project.clone(),
        opaque: vec![],
        manufacturer_refs: vec![],
    }
}

fn at() -> DateTime<Utc> {
    DateTime::parse_from_rfc3339("2026-11-11T09:30:00.123456789Z")
        .unwrap()
        .with_timezone(&Utc)
}

fn baseline(refs: usize) -> ParameterBaseline {
    ParameterBaseline {
        program_id: PROGRAM.into(),
        values: (0..refs)
            .map(|i| (format!("{PROGRAM}_P-{i}_R-{i}"), format!("{}", i % 7)))
            .collect(),
        at: at(),
    }
}

fn record(device: u32, refs: usize) -> BaselineEdit {
    BaselineEdit {
        device: DeviceId(device),
        baseline: Some(baseline(refs)),
    }
}

/// A saved native file holding `project`, and its journal generation.
fn saved(dir: &Path, name: &str, project: &Project) -> (PathBuf, i64) {
    let path = dir.join(name);
    let conn = knx_store::open_and_migrate(&path).unwrap();
    let generation =
        history::save_editor(&conn, &model(project), &CommandStack::new(), None, true).unwrap();
    (path, generation)
}

/// A settlement-style journal write of `working` carrying `edits`.
fn settle(
    path: &Path,
    working: &Project,
    generation: i64,
    edits: &[BaselineEdit],
) -> Result<i64, history::HistoryError> {
    let conn = history::open_editor_store(path, false).unwrap();
    history::save_editor_settling(
        &conn,
        &model(working),
        &CommandStack::new(),
        generation,
        &JournalTrust::default(),
        &mut JournalWriteReport::default(),
        edits,
    )
    .map(|(generation, _)| generation)
}

fn stored(path: &Path) -> ParameterBaselines {
    let store = knx_store::open_existing_read_only(path).unwrap();
    history::load_parameter_baselines(&store.conn).unwrap()
}

fn programmed(project: &Project, device: u32) -> Project {
    let mut project = project.clone();
    let state = &mut project
        .devices
        .get_mut(DeviceId(device))
        .unwrap()
        .commissioning;
    ProgrammingChange::mark(ProgrammingParts::ALL, true).apply_to(state);
    project
}

fn user_version(conn: &Connection) -> i64 {
    conn.query_row("PRAGMA user_version", [], |r| r.get(0))
        .unwrap()
}

fn table_exists(conn: &Connection, name: &str) -> bool {
    conn.query_row(
        "SELECT count(*) FROM sqlite_schema WHERE type = 'table' AND name = ?1",
        [name],
        |r| r.get::<_, i64>(0),
    )
    .unwrap()
        == 1
}

fn rows(conn: &Connection, sql: &str) -> Vec<Vec<Value>> {
    let mut stmt = conn.prepare(sql).unwrap();
    let columns = stmt.column_count();
    stmt.query_map([], |row| {
        (0..columns)
            .map(|i| row.get::<_, Value>(i))
            .collect::<Result<Vec<_>, _>>()
    })
    .unwrap()
    .collect::<Result<Vec<_>, _>>()
    .unwrap()
}

const HISTORY: &str =
    "SELECT 'state', id, generation, working_hash, length(working) FROM project_history_state
     UNION ALL SELECT side, position, 0, image_hash, length(image) FROM project_history_stack
     UNION ALL SELECT 'version', id, 0, image_hash, length(image) FROM project_history_version
     ORDER BY 1, 2";

#[test]
fn a_native19_file_migrates_additively_and_every_baseline_starts_unknown() {
    let dir = tempfile::tempdir().unwrap();
    let base = project("P-1", 2);
    let (path, generation) = saved(dir.path(), "p.knxdb", &base);
    settle(&path, &programmed(&base, 1), generation, &[]).unwrap();
    // Back to the native19 shape: v20 only added the two tables.
    {
        let conn = Connection::open(&path).unwrap();
        conn.execute_batch(
            "DROP TABLE device_parameter_baseline_value; DROP TABLE device_parameter_baseline;
             PRAGMA user_version = 19;",
        )
        .unwrap();
    }
    let bytes = std::fs::read(&path).unwrap();
    let before = rows(
        &Connection::open_with_flags(&path, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap(),
        HISTORY,
    );

    let store = knx_store::open_existing_read_only(&path).unwrap();
    assert_eq!(store.migrated_from, Some(19));
    assert!(history::load_parameter_baselines(&store.conn)
        .unwrap()
        .is_empty());
    assert_eq!(rows(&store.conn, HISTORY), before);
    drop(store);
    assert_eq!(std::fs::read(&path).unwrap(), bytes, "read-only open wrote");

    let conn = knx_store::open_existing_and_migrate(&path).unwrap();
    assert_eq!(user_version(&conn), 20);
    assert_eq!(knx_store::CURRENT_SCHEMA_VERSION, 20);
    for table in [
        "device_parameter_baseline",
        "device_parameter_baseline_value",
    ] {
        assert!(table_exists(&conn, table), "{table}");
    }
    assert!(history::load_parameter_baselines(&conn).unwrap().is_empty());
    assert_eq!(
        rows(&conn, HISTORY),
        before,
        "history rows survive unchanged"
    );
    let recovered = history::load_editor(&conn).unwrap().unwrap();
    assert_eq!(recovered.working.project, programmed(&base, 1));
    drop(conn);

    // The migrated file takes a settlement like any native20 file.
    let next = settle(
        &path,
        &programmed(&base, 1),
        recovered.generation,
        &[record(1, 3)],
    )
    .unwrap();
    assert!(next > recovered.generation);
    assert_eq!(stored(&path).get(DeviceId(1)), Some(&baseline(3)));
}

#[test]
fn a_baseline_only_write_brings_a_native19_file_to_the_current_schema_first() {
    let dir = tempfile::tempdir().unwrap();
    let (path, _) = saved(dir.path(), "p.knxdb", &project("P-1", 1));
    {
        let conn = Connection::open(&path).unwrap();
        conn.execute_batch(
            "DROP TABLE device_parameter_baseline_value; DROP TABLE device_parameter_baseline;
             PRAGMA user_version = 19;",
        )
        .unwrap();
    }
    let conn = history::open_editor_store(&path, false).unwrap();
    history::write_parameter_baselines(&conn, &[record(1, 2)]).unwrap();
    assert_eq!(user_version(&conn), 20);
    drop(conn);
    assert_eq!(stored(&path).get(DeviceId(1)), Some(&baseline(2)));
}

#[test]
fn a_future_native_store_is_refused_like_every_schema_step() {
    let dir = tempfile::tempdir().unwrap();
    let (path, _) = saved(dir.path(), "p.knxdb", &project("P-1", 1));
    {
        let conn = Connection::open(&path).unwrap();
        conn.pragma_update(None, "user_version", knx_store::CURRENT_SCHEMA_VERSION + 1)
            .unwrap();
    }
    for refused in [
        knx_store::open_existing_and_migrate(&path).map(drop),
        knx_store::open_existing_read_only(&path).map(drop),
    ] {
        assert!(
            matches!(
                refused,
                Err(knx_store::MigrationError::FutureSchemaVersion {
                    found: 21,
                    supported: 20
                })
            ),
            "{refused:?}"
        );
    }
}

#[test]
fn settlement_edits_record_replace_and_remove_exactly() {
    let dir = tempfile::tempdir().unwrap();
    let base = project("P-1", 3);
    let (path, generation) = saved(dir.path(), "p.knxdb", &base);
    let generation = settle(&path, &base, generation, &[record(1, 5), record(2, 0)]).unwrap();
    let all = stored(&path);
    assert_eq!(all.len(), 2);
    assert_eq!(
        all.get(DeviceId(1)),
        Some(&baseline(5)),
        "nanoseconds survive"
    );
    assert_eq!(
        all.get(DeviceId(2)),
        Some(&baseline(0)),
        "a program without refs"
    );
    let mut changed = baseline(2);
    changed.values.insert("extra".into(), String::new());
    let replace = BaselineEdit {
        device: DeviceId(1),
        baseline: Some(changed.clone()),
    };
    let remove = BaselineEdit {
        device: DeviceId(2),
        baseline: None,
    };
    let absent = BaselineEdit {
        device: DeviceId(3),
        baseline: None,
    };
    settle(&path, &base, generation, &[replace, remove, absent]).unwrap();
    let all = stored(&path);
    assert_eq!(all.len(), 1);
    assert_eq!(
        all.get(DeviceId(1)),
        Some(&changed),
        "an empty value is a value"
    );
}

#[test]
fn a_failed_baseline_write_takes_the_settled_flags_down_with_it() {
    let dir = tempfile::tempdir().unwrap();
    let base = project("P-1", 2);
    let (path, generation) = saved(dir.path(), "p.knxdb", &base);
    let generation = settle(&path, &base, generation, &[record(2, 1)]).unwrap();
    let before = std::fs::read(&path).unwrap();
    // The header table refuses an empty program id: the fault happens after
    // the journal rows of the newer flags were written in the transaction.
    let mut invalid = baseline(1);
    invalid.program_id.clear();
    let refused = settle(
        &path,
        &programmed(&base, 1),
        generation,
        &[
            BaselineEdit {
                device: DeviceId(2),
                baseline: None,
            },
            BaselineEdit {
                device: DeviceId(1),
                baseline: Some(invalid),
            },
        ],
    );
    assert!(refused.is_err(), "{refused:?}");
    assert_eq!(
        std::fs::read(&path).unwrap(),
        before,
        "nothing was committed"
    );
    let store = knx_store::open_existing_read_only(&path).unwrap();
    let recovered = history::load_editor(&store.conn).unwrap().unwrap();
    assert_eq!(recovered.generation, generation);
    assert_eq!(
        recovered.working.project, base,
        "no newer flags without their baseline"
    );
    assert_eq!(
        history::load_parameter_baselines(&store.conn)
            .unwrap()
            .get(DeviceId(2)),
        Some(&baseline(1)),
        "the removal rolled back too"
    );
}

#[test]
fn history_images_and_versions_are_the_same_bytes_with_or_without_baselines() {
    let dir = tempfile::tempdir().unwrap();
    let base = project("P-1", 2);
    let working = programmed(&base, 1);
    let edits = [record(1, 200), record(2, 200)];
    let mut images = Vec::new();
    for (name, edits) in [("with.knxdb", &edits[..]), ("without.knxdb", &[][..])] {
        let (path, generation) = saved(dir.path(), name, &base);
        settle(&path, &working, generation, edits).unwrap();
        let conn = knx_store::open_existing_and_migrate(&path).unwrap();
        let generation = history::load_editor(&conn).unwrap().unwrap().generation;
        let all = history::load_parameter_baselines(&conn).unwrap();
        // A root save writes a "Previous saved state" version image too.
        history::save_editor_with_baselines(
            &conn,
            &model(&working),
            &CommandStack::new(),
            Some(generation),
            &[],
            &all,
        )
        .unwrap();
        assert_eq!(history::load_parameter_baselines(&conn).unwrap(), all);
        images.push((
            rows(
                &conn,
                "SELECT working, working_hash FROM project_history_state
                 UNION ALL SELECT image, image_hash FROM project_history_version ORDER BY 2",
            ),
            history::stored_history_bytes(&conn).unwrap(),
            all.len(),
        ));
    }
    assert_eq!((images[0].2, images[1].2), (2, 0));
    assert_eq!(images[0].0, images[1].0, "identical stored image bytes");
    assert_eq!(images[0].1, images[1].1, "identical history size");
}

#[test]
fn a_root_save_prunes_absent_devices_and_another_project_takes_none() {
    let dir = tempfile::tempdir().unwrap();
    let base = project("P-1", 3);
    let (path, generation) = saved(dir.path(), "p.knxdb", &base);
    let generation = settle(
        &path,
        &base,
        generation,
        &[record(1, 1), record(2, 1), record(3, 1)],
    )
    .unwrap();

    // A save without baselines of its own (the CLI's): device 3 is gone.
    let mut fewer = base.clone();
    fewer.installations[0]
        .topology
        .unassigned
        .retain(|d| d.0 != 3);
    fewer.devices.remove(DeviceId(3));
    let conn = history::open_editor_store(&path, false).unwrap();
    history::save_editor_with_sources(
        &conn,
        &model(&fewer),
        &CommandStack::new(),
        Some(generation),
        &[],
    )
    .unwrap();
    drop(conn);
    let kept: Vec<_> = stored(&path).iter().map(|(device, _)| device.0).collect();
    assert_eq!(kept, vec![1, 2]);

    // A server save of its complete set keeps exactly the saved devices.
    let conn = history::open_editor_store(&path, false).unwrap();
    let generation = history::load_editor(&conn).unwrap().unwrap().generation;
    let held: ParameterBaselines = [1, 2, 9]
        .into_iter()
        .map(|d| (DeviceId(d), baseline(d as usize)))
        .collect();
    history::save_editor_with_baselines(
        &conn,
        &model(&fewer),
        &CommandStack::new(),
        Some(generation),
        &[],
        &held,
    )
    .unwrap();
    drop(conn);
    let all = stored(&path);
    assert_eq!(
        all.iter()
            .map(|(d, b)| (d.0, b.clone()))
            .collect::<Vec<_>>(),
        vec![(1, baseline(1)), (2, baseline(2))]
    );

    // A plain save of another project's model into the file drops them all:
    // its device ids name other devices.
    let conn = knx_store::open_existing_and_migrate(&path).unwrap();
    knx_store::save_project_with_passthrough(&conn, &project("P-2", 2), &[], &[]).unwrap();
    drop(conn);
    assert!(stored(&path).is_empty());
}

#[test]
fn a_plain_save_of_the_same_project_keeps_present_devices() {
    let dir = tempfile::tempdir().unwrap();
    let base = project("P-1", 2);
    let (path, generation) = saved(dir.path(), "p.knxdb", &base);
    settle(&path, &base, generation, &[record(1, 2), record(2, 2)]).unwrap();
    let mut fewer = base.clone();
    fewer.installations[0]
        .topology
        .unassigned
        .retain(|d| d.0 != 2);
    fewer.devices.remove(DeviceId(2));
    let conn = knx_store::open_existing_and_migrate(&path).unwrap();
    knx_store::save_project_with_passthrough(&conn, &fewer, &[], &[]).unwrap();
    drop(conn);
    let kept: Vec<_> = stored(&path).iter().map(|(device, _)| device.0).collect();
    assert_eq!(kept, vec![1]);
}

/// 500 devices with 200 refs each: one native save and one full read stay
/// within a few seconds even under `nice` on a shared machine. The bound is
/// generous on purpose; the measured times are printed for the record.
#[test]
fn a_large_project_saves_and_reads_its_baselines_proportionately() {
    let dir = tempfile::tempdir().unwrap();
    let base = project("P-1", 500);
    let (path, generation) = saved(dir.path(), "large.knxdb", &base);
    let size_before = std::fs::metadata(&path).unwrap().len();
    let held: ParameterBaselines = (1..=500).map(|d| (DeviceId(d), baseline(200))).collect();
    let conn = history::open_editor_store(&path, false).unwrap();
    let started = Instant::now();
    history::save_editor_with_baselines(
        &conn,
        &model(&base),
        &CommandStack::new(),
        Some(generation),
        &[],
        &held,
    )
    .unwrap();
    let saved_in = started.elapsed();
    drop(conn);
    let started = Instant::now();
    let read = stored(&path);
    let read_in = started.elapsed();
    let size_after = std::fs::metadata(&path).unwrap().len();
    let values: usize = read.iter().map(|(_, b)| b.values.len()).sum();
    assert_eq!((read.len(), values), (500, 100_000));
    assert_eq!(read, held);
    let grown = size_after.saturating_sub(size_before);
    println!(
        "baselines 500x200: save {saved_in:?}, read {read_in:?}, file grew {:.1} MiB",
        grown as f64 / (1024.0 * 1024.0)
    );
    // About 60 bytes per value row with this fixture's ids; far below 50 MiB.
    assert!(grown < 50 * 1024 * 1024, "{grown} bytes");
    assert!(saved_in.as_secs() < 60 && read_in.as_secs() < 60);
}

#[test]
fn a_version_stored_by_an_older_schema_restores_and_leaves_the_baselines_alone() {
    // The authentic native18 fixture holds a named version written by the
    // unchanged v18 writer; v19 images are covered by the snapshot codec's
    // own unit test.
    let source = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/native18-before-ga-description.sqlite");
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("predecessor.knxdb");
    std::fs::copy(source, &path).unwrap();
    let conn = knx_store::open_existing_and_migrate(&path).unwrap();
    assert_eq!(user_version(&conn), 20);
    history::write_parameter_baselines(&conn, &[record(1, 2)]).unwrap();
    let editor = history::load_editor(&conn).unwrap().unwrap();
    let versions = history::list_versions(&conn).unwrap();
    assert_eq!(versions.len(), 1);
    let restored =
        history::restore_version(&conn, versions[0].id, &editor.working, editor.generation)
            .unwrap();
    assert!(restored.generation > editor.generation);
    assert_eq!(
        history::load_parameter_baselines(&conn)
            .unwrap()
            .get(DeviceId(1)),
        Some(&baseline(2)),
        "a restore never brings back or drops a baseline; only a save prunes"
    );
}
