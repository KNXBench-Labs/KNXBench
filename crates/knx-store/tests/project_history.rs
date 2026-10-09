//! Exercises native history recovery, version preservation and atomic refusals.

use knx_core::{Command, CommandStack, GroupAddressStyle, Language, Project};
use knx_store::project_history::{self as history, NativeSnapshot};

fn seed() -> NativeSnapshot {
    let mut project = Project::new(Language("en".into()));
    project.info.name = "History integrity witness".into();
    NativeSnapshot {
        project,
        opaque: vec![knx_store::StoredOpaqueEntry {
            source_path: "unknown.xml".into(),
            xpath: "/unknown".into(),
            kind: "element".into(),
            name: "FutureField".into(),
            bytes: b"opaque witness".to_vec(),
            sha256: "retained-source-hash".into(),
        }],
        manufacturer_refs: vec![knx_store::ManufacturerRef {
            source_path: "M-0001/Hardware.xml".into(),
            sha256: "manifest-source-hash".into(),
            len: 17,
            kind: "ManufacturerData".into(),
        }],
    }
}

fn opened() -> (
    tempfile::TempDir,
    knx_store::Connection,
    NativeSnapshot,
    i64,
) {
    let dir = tempfile::tempdir().unwrap();
    let conn = knx_store::open_and_migrate(&dir.path().join("history.knxdb")).unwrap();
    let seed = seed();
    let generation = history::save_editor(&conn, &seed, &CommandStack::new(), None, true).unwrap();
    assert_eq!(
        NativeSnapshot::read(&conn).unwrap(),
        seed,
        "fixture must really persist"
    );
    (dir, conn, seed, generation)
}

#[test]
fn native_snapshot_codec_is_exact_and_nonrecursive() {
    let snapshot = seed();
    let image = snapshot.encode().unwrap();
    assert_eq!(
        NativeSnapshot::decode(&image, &history::image_hash(&image)).unwrap(),
        snapshot
    );
    assert!(NativeSnapshot::decode(&image, &"0".repeat(64)).is_err());
    assert!(NativeSnapshot::decode(b"not sqlite", &history::image_hash(b"not sqlite")).is_err());
}

#[test]
fn journal_recovers_working_state_both_stacks_and_separate_baseline() {
    let (dir, conn, before, generation) = opened();
    let mut working = before.clone();
    let mut stack = CommandStack::new();
    stack
        .do_command(
            &mut working.project,
            Command::SetGroupAddressStyle {
                style: GroupAddressStyle::Free,
            },
        )
        .unwrap();
    let generation =
        history::save_editor(&conn, &working, &stack, Some(generation), false).unwrap();
    assert_eq!(NativeSnapshot::read(&conn).unwrap(), before);
    drop(conn);
    let conn = knx_store::open_existing_and_migrate(&dir.path().join("history.knxdb")).unwrap();
    let recovered = history::load_editor(&conn).unwrap().unwrap();
    assert_eq!(recovered.generation, generation);
    assert_eq!(recovered.baseline, before);
    assert_eq!(recovered.working, working);
    assert_eq!(recovered.undo.len(), 1);
    assert!(recovered.redo.is_empty());
    let mut stack = CommandStack::from_snapshot_states(recovered.undo, recovered.redo);
    stack.undo(&mut working.project).unwrap();
    let generation =
        history::save_editor(&conn, &working, &stack, Some(generation), false).unwrap();
    let recovered = history::load_editor(&conn).unwrap().unwrap();
    assert_eq!(recovered.generation, generation);
    assert!(recovered.undo.is_empty());
    assert_eq!(recovered.redo.len(), 1);
    assert_eq!(recovered.working, before);
}

#[test]
fn saves_and_named_versions_preserve_full_snapshots_and_restore_the_predecessor() {
    let (_dir, conn, before, generation) = opened();
    let named_generation =
        history::create_version(&conn, &before, "Before redesign", generation).unwrap();
    let mut working = before.clone();
    working.project.info.name = "Redesign".into();
    let generation = history::save_editor(
        &conn,
        &working,
        &CommandStack::new(),
        Some(named_generation),
        true,
    )
    .unwrap();
    let versions = history::list_versions(&conn).unwrap();
    assert_eq!(versions.len(), 2);
    let named = versions.iter().find(|v| v.reason == "named").unwrap();
    let restored = history::restore_version(&conn, named.id, &working, generation).unwrap();
    assert_eq!(restored.working, before);
    assert_eq!(NativeSnapshot::read(&conn).unwrap(), before);
    let versions = history::list_versions(&conn).unwrap();
    assert_eq!(versions.len(), 3);
    let safety = versions.iter().find(|v| v.reason == "pre_restore").unwrap();
    let restored_again =
        history::restore_version(&conn, safety.id, &before, restored.generation).unwrap();
    assert_eq!(restored_again.working, working);
}

#[test]
fn stale_editor_refusal_changes_neither_journal_nor_saved_root() {
    let (_dir, conn, before, generation) = opened();
    let mut working = before.clone();
    working.project.info.name = "First editor wins".into();
    history::save_editor(
        &conn,
        &working,
        &CommandStack::new(),
        Some(generation),
        false,
    )
    .unwrap();
    let admitted = history::load_editor(&conn).unwrap().unwrap();
    assert!(
        history::save_editor(&conn, &before, &CommandStack::new(), Some(generation), true).is_err()
    );
    assert_eq!(
        history::load_editor(&conn).unwrap().unwrap().working,
        admitted.working
    );
    assert_eq!(NativeSnapshot::read(&conn).unwrap(), before);
}

#[test]
fn late_journal_failure_rolls_back_saved_root_versions_and_stack() {
    let (_dir, conn, before, generation) = opened();
    let prior = history::load_editor(&conn).unwrap().unwrap();
    let mut working = before.clone();
    let mut stack = CommandStack::new();
    stack
        .do_command(
            &mut working.project,
            Command::SetGroupAddressStyle {
                style: GroupAddressStyle::Free,
            },
        )
        .unwrap();
    stack
        .do_command(
            &mut working.project,
            Command::SetGroupAddressStyle {
                style: GroupAddressStyle::TwoLevel,
            },
        )
        .unwrap();
    // A synthetic extra index has no payload to lose, but refuses the second
    // journal row after root/version writes. Keep proving late rollback rather
    // than the new earlier rejection of persistent foreign triggers.
    conn.execute_batch("CREATE UNIQUE INDEX refuse_history ON project_history_stack(side)")
        .unwrap();
    assert!(history::save_editor(&conn, &working, &stack, Some(generation), true).is_err());
    assert_eq!(NativeSnapshot::read(&conn).unwrap(), before);
    assert_eq!(
        history::load_editor(&conn).unwrap().unwrap().generation,
        prior.generation
    );
    assert!(history::list_versions(&conn).unwrap().is_empty());
    assert!(conn.is_autocommit());
}

#[test]
fn malformed_hash_or_stack_gap_refuses_the_entire_workspace_and_later_writes() {
    for corrupt in [
        "UPDATE project_history_state SET working_hash = printf('%064d', 0)",
        "UPDATE project_history_stack SET position = 4",
    ] {
        let (_dir, conn, before, generation) = opened();
        let mut working = before.clone();
        let mut stack = CommandStack::new();
        stack
            .do_command(
                &mut working.project,
                Command::SetGroupAddressStyle {
                    style: GroupAddressStyle::Free,
                },
            )
            .unwrap();
        let generation =
            history::save_editor(&conn, &working, &stack, Some(generation), false).unwrap();
        conn.execute_batch(corrupt).unwrap();
        assert!(history::load_editor(&conn).is_err());
        assert!(
            history::save_editor(&conn, &before, &CommandStack::new(), Some(generation), true)
                .is_err()
        );
        assert_eq!(NativeSnapshot::read(&conn).unwrap(), before);
    }
}

#[test]
fn corrupt_version_refuses_restore_and_listing_without_mutation() {
    let (_dir, conn, before, generation) = opened();
    let generation = history::create_version(&conn, &before, "Witness", generation).unwrap();
    let id = history::list_versions(&conn).unwrap()[0].id;
    conn.execute(
        "UPDATE project_history_version SET image_hash = ?1",
        ["0".repeat(64)],
    )
    .unwrap();
    assert!(history::list_versions(&conn).is_err());
    assert!(history::restore_version(&conn, id, &before, generation).is_err());
    assert_eq!(NativeSnapshot::read(&conn).unwrap(), before);
    assert_eq!(
        history::load_editor(&conn).unwrap().unwrap().generation,
        generation
    );
}

#[test]
fn plain_store_writer_preserves_recovered_unsaved_work_before_invalidating_undo() {
    let (_dir, conn, before, generation) = opened();
    let mut working = before.clone();
    working.project.info.name = "Unsaved recovered work".into();
    history::save_editor(
        &conn,
        &working,
        &CommandStack::new(),
        Some(generation),
        false,
    )
    .unwrap();
    let mut replacement = before.clone();
    replacement.project.info.name = "Non-editor writer".into();
    knx_store::save_project_with_passthrough(
        &conn,
        &replacement.project,
        &replacement.opaque,
        &replacement.manufacturer_refs,
    )
    .unwrap();
    let versions = history::list_versions(&conn).unwrap();
    let recovered = versions
        .iter()
        .find(|v| v.reason == "replaced_workspace")
        .unwrap();
    assert_eq!(history::read_version(&conn, recovered.id).unwrap(), working);
    assert_eq!(
        history::load_editor(&conn).unwrap().unwrap().working,
        replacement
    );
}

#[test]
fn duplicate_save_creates_no_duplicate_automatic_version() {
    let (_dir, conn, before, generation) = opened();
    history::save_editor(&conn, &before, &CommandStack::new(), Some(generation), true).unwrap();
    assert!(history::list_versions(&conn).unwrap().is_empty());
}

#[test]
fn explicit_delete_is_generation_bound_and_removes_only_the_selected_version() {
    let (_dir, conn, before, generation) = opened();
    let generation = history::create_version(&conn, &before, "First", generation).unwrap();
    let generation = history::create_version(&conn, &before, "Second", generation).unwrap();
    let versions = history::list_versions(&conn).unwrap();
    assert!(history::delete_version(&conn, versions[0].id, generation - 1).is_err());
    history::delete_version(&conn, versions[0].id, generation).unwrap();
    assert_eq!(history::list_versions(&conn).unwrap().len(), 1);
    assert_eq!(history::list_versions(&conn).unwrap()[0].id, versions[1].id);
}

#[test]
fn legacy_history_mutation_migrates_only_with_its_successful_commit() {
    let (dir, conn, before, _generation) = opened();
    conn.execute_batch("DROP TABLE project_history_stack; DROP TABLE project_history_state; DROP TABLE project_history_version; DROP TABLE project_history_context; ALTER TABLE line DROP COLUMN model_position; PRAGMA user_version = 10;").unwrap();
    drop(conn);
    let path = dir.path().join("history.knxdb");
    let original = std::fs::read(&path).unwrap();
    let conn = knx_store::Connection::open(&path).unwrap();
    let refusal = history::create_version(&conn, &before, "", 0).unwrap_err();
    assert!(
        matches!(refusal, history::HistoryError::Invalid(_)),
        "history admission must occur inside the migration transaction"
    );
    assert_eq!(
        conn.query_row("PRAGMA user_version", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        10
    );
    assert!(
        std::fs::read(&path).unwrap() == original,
        "refused migration must preserve file bytes"
    );
    let generation = history::create_version(&conn, &before, "First version", 0).unwrap();
    assert_eq!(generation, 1);
    assert_eq!(
        conn.query_row("PRAGMA user_version", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        knx_store::CURRENT_SCHEMA_VERSION
    );
    assert_eq!(NativeSnapshot::read(&conn).unwrap(), before);
}

#[test]
fn a_snapshot_that_would_repair_allocator_metadata_is_refused_before_encoding() {
    let mut snapshot = seed();
    snapshot.project.installations.push(knx_core::Installation {
        id: knx_core::InstallationId(1),
        name: "Synthetic installation".into(),
        default_line: None,
        multicast_address: None,
        completion: knx_core::CompletionStatus::FinishedDesign,
        topology: knx_core::Topology {
            areas: vec![],
            lines: vec![],
            unassigned: vec![knx_core::DeviceId(1)],
        },
        buildings: vec![],
        group_ranges: vec![],
        group_addresses: vec![],
        parameters: vec![],
    });
    snapshot.project.devices.insert(knx_core::DeviceInstance {
        id: knx_core::DeviceId(1),
        source: knx_core::SourceRef {
            path: "synthetic".into(),
            ets_id: "synthetic-device".into(),
        },
        name: "Witness".into(),
        description: None,
        address: None,
        product_ref: String::new(),
        program_ref: String::new(),
        commissioning: knx_core::CommissioningState::default(),
        visibility_calculated: true,
        com_objects: vec![],
        binary_data: vec![],
    });
    assert!(
        snapshot.encode().is_err(),
        "history snapshots must reopen exactly, not with a repaired allocator"
    );
}

#[test]
fn retained_context_is_stored_once_across_undo_and_named_versions() {
    let conn = knx_store::open_and_migrate_in_memory().unwrap();
    let mut working = seed();
    working.opaque[0].bytes = vec![b'x'; 8 * 1024 * 1024];
    let original = working.clone();
    let mut stack = CommandStack::new();
    let mut generation = history::save_editor(&conn, &working, &stack, None, true).unwrap();
    for style in [
        GroupAddressStyle::Free,
        GroupAddressStyle::TwoLevel,
        GroupAddressStyle::ThreeLevel,
    ] {
        stack
            .do_command(
                &mut working.project,
                Command::SetGroupAddressStyle { style },
            )
            .unwrap();
        generation =
            history::save_editor(&conn, &working, &stack, Some(generation), false).unwrap();
    }
    history::create_version(&conn, &working, "Checkpoint", generation).unwrap();
    assert!(
        history::stored_history_bytes(&conn).unwrap() < 16 * 1024 * 1024,
        "unchanged retained bytes must not be duplicated in every undo/version image"
    );
    let recovered = history::load_editor(&conn).unwrap().unwrap();
    assert!(
        recovered.working == original,
        "whole native context must survive deduplication"
    );
    let version = history::list_versions(&conn).unwrap().remove(0);
    assert!(history::read_version(&conn, version.id).unwrap() == original);
}

#[test]
fn history_reads_join_a_caller_owned_consistent_read_transaction() {
    let (_dir, conn, _working, generation) = opened();
    let tx = conn.unchecked_transaction().unwrap();
    assert_eq!(
        history::load_editor(&tx).unwrap().unwrap().generation,
        generation
    );
    assert!(history::list_versions(&tx).unwrap().is_empty());
    assert!(
        !conn.is_autocommit(),
        "a nested history reader must not commit its caller's transaction"
    );
    tx.commit().unwrap();
}

#[test]
fn changed_retained_context_remains_reachable_until_its_last_version_is_deleted() {
    let (_dir, conn, before, generation) = opened();
    let generation =
        history::create_version(&conn, &before, "Original context", generation).unwrap();
    let original = history::list_versions(&conn).unwrap()[0].id;
    let mut changed = before.clone();
    changed.opaque[0].bytes = b"changed retained context".to_vec();
    changed.manufacturer_refs[0].len += 1;
    let generation = history::save_editor(
        &conn,
        &changed,
        &CommandStack::new(),
        Some(generation),
        true,
    )
    .unwrap();
    let restored = history::restore_version(&conn, original, &changed, generation).unwrap();
    assert!(restored.working == before);
    let versions = history::list_versions(&conn).unwrap();
    let safety = versions
        .iter()
        .find(|version| version.reason == "pre_restore")
        .unwrap();
    assert!(history::read_version(&conn, safety.id).unwrap() == changed);
    let contexts: i64 = conn
        .query_row("SELECT count(*) FROM project_history_context", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(contexts, 2);
    history::delete_version(&conn, safety.id, restored.generation).unwrap();
    let contexts: i64 = conn
        .query_row("SELECT count(*) FROM project_history_context", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(contexts, 1);
    assert!(history::read_version(&conn, original).unwrap() == before);
}

#[test]
fn corrupt_or_future_retained_context_refuses_the_whole_journal_without_mutation() {
    for mutation in [
        "UPDATE project_history_context SET image_hash = printf('%064d', 0)",
        "PRAGMA ignore_check_constraints = ON; UPDATE project_history_context SET format_version = 2",
    ] {
        let (_dir, conn, before, generation) = opened();
        conn.execute_batch(mutation).unwrap();
        assert!(history::load_editor(&conn).is_err());
        assert!(history::save_editor(&conn, &before, &CommandStack::new(), Some(generation), true).is_err());
        assert!(NativeSnapshot::read(&conn).unwrap() == before);
        let remaining: i64 = conn.query_row("SELECT generation FROM project_history_state", [], |r| r.get(0)).unwrap();
        assert_eq!(remaining, generation);
    }
}

#[test]
fn a_five_thousand_device_project_recovers_undo_without_losing_entities() {
    let conn = knx_store::open_and_migrate_in_memory().unwrap();
    let mut before = seed();
    before.project.installations.push(knx_core::Installation {
        id: knx_core::InstallationId(1),
        name: "Synthetic large installation".into(),
        default_line: None,
        multicast_address: None,
        completion: knx_core::CompletionStatus::FinishedDesign,
        topology: knx_core::Topology {
            areas: vec![],
            lines: vec![],
            unassigned: (1..=5000).map(knx_core::DeviceId).collect(),
        },
        buildings: vec![],
        group_ranges: vec![],
        group_addresses: vec![],
        parameters: vec![],
    });
    for id in 1..=5000 {
        before.project.devices.insert(knx_core::DeviceInstance {
            id: knx_core::DeviceId(id),
            source: knx_core::SourceRef {
                path: "synthetic".into(),
                ets_id: format!("synthetic-{id}"),
            },
            name: format!("Synthetic device {id}"),
            description: None,
            address: None,
            product_ref: String::new(),
            program_ref: String::new(),
            commissioning: knx_core::CommissioningState::default(),
            visibility_calculated: true,
            com_objects: vec![],
            binary_data: vec![],
        });
    }
    before.project.ids = knx_core::IdAllocators::from_counts(5000, 0, 0, 0, 0, 0, 0, 0, 0);
    let mut working = before.clone();
    let mut stack = CommandStack::new();
    let generation = history::save_editor(&conn, &working, &stack, None, true).unwrap();
    stack
        .do_command(
            &mut working.project,
            Command::SetGroupAddressStyle {
                style: GroupAddressStyle::Free,
            },
        )
        .unwrap();
    history::save_editor(&conn, &working, &stack, Some(generation), false).unwrap();
    let recovered = history::load_editor(&conn).unwrap().unwrap();
    assert_eq!(recovered.working.project.devices.iter().count(), 5000);
    let mut stack = CommandStack::from_snapshot_states(recovered.undo, recovered.redo);
    stack.undo(&mut working.project).unwrap();
    assert!(
        working == before,
        "large-project undo must recover the complete synthetic model and retained context"
    );
}

#[test]
fn unknown_native_schema_extensions_are_refused_without_changing_the_saved_store() {
    for extension in [
        "CREATE TABLE future_payload(value BLOB); INSERT INTO future_payload VALUES (X'012345');",
        "CREATE TABLE sqlitex_payload(value BLOB); INSERT INTO sqlitex_payload VALUES (X'012345');",
        "ALTER TABLE project_info ADD COLUMN future_value TEXT; UPDATE project_info SET future_value = 'preserve me';",
        "CREATE TRIGGER future_writer AFTER UPDATE ON project_info BEGIN SELECT 1; END;",
        "CREATE VIEW future_view AS SELECT name FROM project_info;",
        "ALTER TABLE project_info ADD COLUMN future_generated TEXT GENERATED ALWAYS AS (name) VIRTUAL;",
    ] {
        let (dir, conn, snapshot, generation) = opened();
        conn.execute_batch(extension).unwrap();
        let bytes_before = conn.serialize(rusqlite::MAIN_DB).unwrap().to_vec();
        assert!(history::load_editor(&conn).is_err());
        assert!(history::open_editor_store(&dir.path().join("history.knxdb"), false).is_err());
        let result = history::save_editor(&conn, &snapshot, &CommandStack::new(), Some(generation), true);
        assert!(result.is_err(), "unknown native schema must be reported, not partly captured");
        assert_eq!(conn.serialize(rusqlite::MAIN_DB).unwrap().as_ref(), bytes_before);
    }
}

#[test]
fn snapshot_codec_refuses_unknown_native_schema_even_with_a_valid_image_hash() {
    let image = seed().encode().unwrap();
    let mut conn = knx_store::Connection::open_in_memory().unwrap();
    conn.deserialize_read_exact(rusqlite::MAIN_DB, image.as_slice(), image.len(), false)
        .unwrap();
    conn.execute_batch("CREATE TABLE future_payload (value TEXT); INSERT INTO future_payload VALUES ('preserve me');").unwrap();
    let extended = conn.serialize(rusqlite::MAIN_DB).unwrap().to_vec();
    assert!(
        NativeSnapshot::decode(&extended, &history::image_hash(&extended)).is_err(),
        "valid integrity metadata does not make unknown native data supported"
    );
}
