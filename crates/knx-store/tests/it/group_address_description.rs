//! Native 19 group-address descriptions: authentic v18 migration, persistence and identity.

use knx_core::{Command, CommandStack, Project};
use knx_store::project_history::{self as history, NativeSnapshot};
use rusqlite::{types::Value, Connection, OpenFlags};
use std::path::{Path, PathBuf};

/// Semantic identities the unchanged v18 writer recorded for its own images
/// (`native18-before-ga-description.provenance.json`, `semantic_sha256`).
const BASELINE_COMPLETE: &str = "2ffac29075f5997b2da9241a179a81f6df806a49c1c11dcc71695e2d74f5e83d";
const WORKING_COMPLETE: &str = "313335e7ff787e21a136140440d3566adb517db01529df698fbb950fa4f4880d";
const UNDO: [&str; 2] = [
    "dbdac75f95db6db303fd621c472f2fbf4bdb9945c800e4e7e884adbf433994b3",
    "ad66fb07a8b5680bcfb9b24bafa6e12b1023f5317984f07a19a32eb5ee1a8d37",
];
const REDO: &str = "6ea3435637bcb0b5e4ce97516b1fbfec666f2c755224bba5931dacd34beec839";
/// Fixture digest from the same provenance file.
const FIXTURE_SHA256: &str = "01713f37dc39caa0d2a9365cf7cf49bbcb4c982cc0dae3b72ba3c37ee8327fb4";

const GA_PREFIX: &str =
    "/KNX/Project/Installations/Installation/GroupAddresses/GroupRanges/GroupRange/GroupAddress";
const DESCRIBED: &str = "Beschreibung mit; Semikolon und \"Anf\u{fc}hrung\" \u{df}";

fn copy_fixture() -> (tempfile::TempDir, PathBuf, Vec<u8>) {
    let source = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/native18-before-ga-description.sqlite");
    let bytes = std::fs::read(source).unwrap();
    assert_eq!(history::image_hash(&bytes), FIXTURE_SHA256);
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("predecessor.knxdb");
    std::fs::write(&path, &bytes).unwrap();
    (dir, path, bytes)
}

fn user_version(conn: &Connection) -> i64 {
    conn.query_row("PRAGMA user_version", [], |r| r.get(0))
        .unwrap()
}

fn description_rows(conn: &Connection) -> i64 {
    conn.query_row("SELECT count(*) FROM group_address_description", [], |r| {
        r.get(0)
    })
    .unwrap()
}

fn history_rows(conn: &Connection) -> Vec<Vec<Vec<Value>>> {
    [
        "project_history_context",
        "project_history_state",
        "project_history_stack",
        "project_history_version",
    ]
    .into_iter()
    .map(|table| {
        let mut stmt = conn
            .prepare(&format!("SELECT * FROM {table} ORDER BY 1,2"))
            .unwrap();
        let columns = stmt.column_count();
        stmt.query_map([], |row| {
            (0..columns)
                .map(|i| row.get::<_, Value>(i))
                .collect::<Result<Vec<_>, _>>()
        })
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap()
    })
    .collect()
}

fn model(project: &Project) -> NativeSnapshot {
    NativeSnapshot {
        project: project.clone(),
        opaque: vec![],
        manufacturer_refs: vec![],
    }
}

/// The migration lifts nothing: every description stays unknown to the model.
fn no_mapped_descriptions(project: &Project) {
    for entry in &project.installations[0].group_addresses {
        assert_eq!(entry.description, None, "migration invented a description");
    }
}

/// The keyed retained `Description` rows the v18 importer wrote, unchanged.
fn retained_descriptions(opaque: &[knx_store::StoredOpaqueEntry]) -> Vec<(String, Vec<u8>)> {
    let mut rows: Vec<_> = opaque
        .iter()
        .filter(|e| e.kind == "RetainedAttribute" && e.name == "Description")
        .map(|e| (e.xpath.clone(), e.bytes.clone()))
        .collect();
    rows.sort();
    rows
}

fn expected_retained() -> Vec<(String, Vec<u8>)> {
    vec![
        (String::new(), b"Unkeyed-control".to_vec()),
        (
            format!("{GA_PREFIX}[@Id='P-0001-0_GA-1']"),
            DESCRIBED.as_bytes().to_vec(),
        ),
        (format!("{GA_PREFIX}[@Id='P-0001-0_GA-2']"), Vec::new()),
    ]
}

#[test]
fn native18_predecessor_migrates_without_lifting_and_keeps_every_identity() {
    let (_dir, path, bytes) = copy_fixture();
    let original = Connection::open_with_flags(&path, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    assert_eq!(user_version(&original), 18);
    let rows = history_rows(&original);
    drop(original);

    let store = knx_store::open_existing_read_only(&path).unwrap();
    assert_eq!(store.migrated_from, Some(18));
    assert_eq!(description_rows(&store.conn), 0);
    assert_eq!(history_rows(&store.conn), rows);
    let root = NativeSnapshot::read(&store.conn).unwrap();
    no_mapped_descriptions(&root.project);
    assert_eq!(retained_descriptions(&root.opaque), expected_retained());
    let recovered = history::load_editor(&store.conn).unwrap().unwrap();
    assert_eq!(
        recovered.baseline.semantic_hash().unwrap(),
        BASELINE_COMPLETE,
        "an unlifted root keeps the identity its history baseline recorded"
    );
    assert_eq!(recovered.working.semantic_hash().unwrap(), WORKING_COMPLETE);
    assert_eq!(recovered.undo.len(), 2);
    for (project, expected) in recovered.undo.iter().zip(UNDO) {
        no_mapped_descriptions(project);
        assert_eq!(model(project).semantic_hash().unwrap(), expected);
    }
    assert_eq!(model(&recovered.redo[0]).semantic_hash().unwrap(), REDO);
    let named = history::list_versions(&store.conn).unwrap();
    assert_eq!(named.len(), 1);
    no_mapped_descriptions(
        &history::read_version(&store.conn, named[0].id)
            .unwrap()
            .project,
    );
    drop(store);
    assert_eq!(std::fs::read(&path).unwrap(), bytes, "read-only open wrote");

    // An owned write migrates; a description edit then rides the old history.
    let conn = knx_store::open_existing_and_migrate(&path).unwrap();
    assert_eq!(user_version(&conn), knx_store::CURRENT_SCHEMA_VERSION);
    assert_eq!(history_rows(&conn), rows);
    assert_eq!(
        retained_descriptions(&knx_store::load_opaque(&conn).unwrap()),
        expected_retained(),
        "the opaque evidence survives the upgrade byte-exact"
    );
    let mut working = recovered.working.clone();
    let mut stack = CommandStack::from_snapshot_states(recovered.undo, recovered.redo);
    let target = working.project.installations[0].group_addresses[2].id;
    stack
        .do_command(
            &mut working.project,
            Command::SetGroupAddressDescription {
                id: target,
                description: Some("Neu \u{e4}".into()),
            },
        )
        .unwrap();
    assert_ne!(working.semantic_hash().unwrap(), WORKING_COMPLETE);
    let generation =
        history::save_editor(&conn, &working, &stack, Some(recovered.generation), false).unwrap();
    drop(conn);

    let store = knx_store::open_existing_read_only(&path).unwrap();
    assert_eq!(store.migrated_from, None);
    let reopened = history::load_editor(&store.conn).unwrap().unwrap();
    assert_eq!(reopened.generation, generation);
    let edited = reopened.working.project.installations[0]
        .group_addresses
        .iter()
        .find(|e| e.id == target)
        .unwrap();
    assert_eq!(edited.description.as_deref(), Some("Neu \u{e4}"));
    let mut project = reopened.working.project.clone();
    let mut stack = CommandStack::from_snapshot_states(reopened.undo, reopened.redo);
    stack.undo(&mut project).unwrap();
    no_mapped_descriptions(&project);
    assert_eq!(model(&project), model(&recovered.working.project));
}

#[test]
fn descriptions_save_reload_clear_and_change_the_semantic_identity() {
    let (_dir, path, _) = copy_fixture();
    let old = knx_store::open_existing_read_only(&path).unwrap();
    let mut project = knx_store::load_project(&old.conn).unwrap();
    drop(old);
    let conn = knx_store::open_and_migrate_in_memory().unwrap();
    let before = model(&project).semantic_hash().unwrap();
    let ids: Vec<_> = project.installations[0]
        .group_addresses
        .iter()
        .map(|e| e.id)
        .collect();
    for (id, text) in [(ids[0], DESCRIBED), (ids[1], "")] {
        Command::SetGroupAddressDescription {
            id,
            description: Some(text.into()),
        }
        .apply(&mut project)
        .unwrap();
    }
    knx_store::save_project(&conn, &project).unwrap();
    assert_eq!(description_rows(&conn), 2, "an empty description is a row");
    assert_eq!(knx_store::load_project(&conn).unwrap(), project);
    let edited = model(&project);
    assert_ne!(edited.semantic_hash().unwrap(), before);
    let image = edited.encode().unwrap();
    let decoded = NativeSnapshot::decode(&image, &history::image_hash(&image)).unwrap();
    assert_eq!(decoded, edited, "history images carry the description");

    for id in [ids[0], ids[1]] {
        Command::SetGroupAddressDescription {
            id,
            description: None,
        }
        .apply(&mut project)
        .unwrap();
    }
    knx_store::save_project(&conn, &project).unwrap();
    assert_eq!(description_rows(&conn), 0);
    assert_eq!(model(&project).semantic_hash().unwrap(), before);
}

#[test]
fn deleting_a_described_address_removes_its_description_row_on_the_next_save() {
    let (_dir, path, _) = copy_fixture();
    let old = knx_store::open_existing_read_only(&path).unwrap();
    let mut project = knx_store::load_project(&old.conn).unwrap();
    drop(old);
    let conn = knx_store::open_and_migrate_in_memory().unwrap();
    let target = project.installations[0].group_addresses[2].id;
    Command::SetGroupAddressDescription {
        id: target,
        description: Some("weg".into()),
    }
    .apply(&mut project)
    .unwrap();
    knx_store::save_project(&conn, &project).unwrap();
    assert_eq!(description_rows(&conn), 1);
    Command::DeleteGroupAddress { id: target }
        .apply(&mut project)
        .unwrap();
    knx_store::save_project(&conn, &project).unwrap();
    assert_eq!(description_rows(&conn), 0);
    assert_eq!(knx_store::load_project(&conn).unwrap(), project);
}

#[test]
fn a_future_store_is_still_refused() {
    let (_dir, path, _) = copy_fixture();
    {
        let conn = knx_store::open_existing_and_migrate(&path).unwrap();
        conn.pragma_update(None, "user_version", knx_store::CURRENT_SCHEMA_VERSION + 1)
            .unwrap();
    }
    assert!(knx_store::open_existing_and_migrate(&path).is_err());
    assert!(knx_store::open_existing_read_only(&path).is_err());
}
