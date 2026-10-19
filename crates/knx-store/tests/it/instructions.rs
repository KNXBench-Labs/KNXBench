//! Native instruction overlays and authentic predecessor-history admission.
use knx_core::instruction::{InstructionDocument, InstructionRun, InstructionStyle};
use knx_store::project_history::{self as history, NativeSnapshot};
const PREDECESSOR: &[u8] =
    include_bytes!("../../fixtures/project_v12_instruction_history_device.sqlite");

fn copied_predecessor() -> (tempfile::TempDir, std::path::PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("predecessor.knxdb");
    std::fs::write(&path, PREDECESSOR).unwrap();
    (dir, path)
}
fn styled() -> InstructionDocument {
    InstructionDocument::new(vec![
        InstructionRun {
            text: "Grüße ".into(),
            style: Default::default(),
        },
        InstructionRun {
            text: "😀 bold".into(),
            style: InstructionStyle {
                bold: true,
                italic: true,
                underline: false,
            },
        },
    ])
    .unwrap()
}
#[test]
fn nonempty_instruction_overlay_survives_native_save_reopen_and_snapshot() {
    let (_dir, path) = copied_predecessor();
    let conn = knx_store::open_existing_and_migrate(&path).unwrap();
    let mut snapshot = NativeSnapshot::read(&conn).unwrap();
    let device = snapshot.project.devices.iter().next().unwrap().id;
    snapshot.project.instructions.insert(device, styled());
    // Avoid replacing the authentic recovered journal in this simple save
    // witness; history admission itself is covered separately below.
    let newdir = tempfile::tempdir().unwrap();
    let native = newdir.path().join("overlay.knxdb");
    let out = knx_store::open_and_migrate(&native).unwrap();
    knx_store::save_project_with_passthrough(
        &out,
        &snapshot.project,
        &snapshot.opaque,
        &snapshot.manufacturer_refs,
    )
    .unwrap();
    assert_eq!(
        NativeSnapshot::read(&out).unwrap(),
        snapshot,
        "native save must not discard instruction overlays"
    );
    drop(out);
    let out = knx_store::open_existing_and_migrate(&native).unwrap();
    assert_eq!(NativeSnapshot::read(&out).unwrap(), snapshot);
    let bytes = snapshot.encode().unwrap();
    assert_eq!(
        NativeSnapshot::decode(&bytes, &history::image_hash(&bytes)).unwrap(),
        snapshot
    );
}
#[test]
fn present_empty_document_and_absent_override_are_distinct_native_states() {
    let (_dir, path) = copied_predecessor();
    let conn = knx_store::open_existing_and_migrate(&path).unwrap();
    let mut snapshot = NativeSnapshot::read(&conn).unwrap();
    let original_hash = snapshot.semantic_hash().unwrap();
    let id = snapshot.project.devices.iter().next().unwrap().id;
    snapshot
        .project
        .instructions
        .insert(id, InstructionDocument::default());
    assert_ne!(
        snapshot.semantic_hash().unwrap(),
        original_hash,
        "explicit empty override changes user state"
    );
    let bytes = snapshot.encode().unwrap();
    assert_eq!(
        NativeSnapshot::decode(&bytes, &history::image_hash(&bytes))
            .unwrap()
            .project
            .instructions[&id],
        InstructionDocument::default()
    );
}
#[test]
fn authentic_v12_root_working_both_stacks_named_version_and_context_survive() {
    let (_dir, path) = copied_predecessor();
    let before = std::fs::read(&path).unwrap();
    // Read-only admission must upgrade only an isolated copy.
    let read = knx_store::open_existing_read_only(&path).unwrap();
    let state = history::load_editor(&read.conn).unwrap().unwrap();
    assert_eq!(state.baseline.project.info.name, "Saved predecessor");
    assert_eq!(state.working.project.info.name, "Working predecessor");
    assert_eq!(state.undo[0].info.name, "Saved predecessor");
    assert_eq!(state.redo[0].info.name, "Redo predecessor");
    assert_eq!(state.working.project.devices.iter().count(), 1);
    assert_eq!(
        state
            .working
            .project
            .devices
            .iter()
            .next()
            .unwrap()
            .description
            .as_deref(),
        Some("Grüße predecessor")
    );
    assert!(state.working.project.instructions.is_empty());
    assert_eq!(
        state.working.opaque[0].bytes,
        b"retained original synthetic evidence"
    );
    let versions = history::list_versions(&read.conn).unwrap();
    assert_eq!(versions.len(), 1);
    assert_eq!(versions[0].label, "Named predecessor");
    assert_eq!(
        history::read_version(&read.conn, versions[0].id)
            .unwrap()
            .project
            .info
            .name,
        "Working predecessor"
    );
    assert_eq!(
        std::fs::read(&path).unwrap(),
        before,
        "read-only admission must never migrate original file bytes"
    );
    drop(read);
    let migrated = knx_store::open_existing_and_migrate(&path).unwrap();
    let recovered = history::load_editor(&migrated).unwrap().unwrap();
    assert_eq!(recovered.working, state.working);
    assert_eq!(recovered.baseline, state.baseline);
    assert_eq!(recovered.undo, state.undo);
    assert_eq!(recovered.redo, state.redo);
    assert_eq!(
        history::list_versions(&migrated).unwrap()[0].label,
        versions[0].label
    );
    assert_eq!(
        migrated
            .query_row("PRAGMA user_version", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        knx_store::CURRENT_SCHEMA_VERSION
    );
}
#[test]
fn dangling_device_overlay_refuses_before_native_state_changes() {
    let (_dir, path) = copied_predecessor();
    let conn = knx_store::open_existing_and_migrate(&path).unwrap();
    let original = NativeSnapshot::read(&conn).unwrap();
    let mut invalid = original.clone();
    invalid
        .project
        .instructions
        .insert(knx_core::DeviceId(u32::MAX), styled());
    assert!(knx_store::save_project(&conn, &invalid.project).is_err());
    assert_eq!(NativeSnapshot::read(&conn).unwrap(), original);
}

#[test]
fn edited_working_state_and_both_history_stacks_survive_restart_and_undo_redo() {
    use knx_core::{Command, CommandStack};
    let (_dir, path) = copied_predecessor();
    let conn = knx_store::open_existing_and_migrate(&path).unwrap();
    let previous = history::load_editor(&conn).unwrap().unwrap();
    let mut working = previous.working.clone();
    let device = working.project.devices.iter().next().unwrap().id;
    let mut stack = CommandStack::from_snapshot_states(previous.undo, previous.redo);
    stack
        .do_command(
            &mut working.project,
            Command::SetDeviceInstructions {
                device,
                document: Some(styled()),
            },
        )
        .unwrap();
    let generation =
        history::save_editor(&conn, &working, &stack, Some(previous.generation), false).unwrap();
    drop(conn);
    let conn = knx_store::open_existing_and_migrate(&path).unwrap();
    let reopened = history::load_editor(&conn).unwrap().unwrap();
    assert_eq!(reopened.generation, generation);
    assert_eq!(reopened.working, working);
    assert_eq!(reopened.baseline.project.info.name, "Saved predecessor");
    let mut current = reopened.working.clone();
    let mut stack = CommandStack::from_snapshot_states(reopened.undo, reopened.redo);
    stack.undo(&mut current.project).unwrap();
    assert!(current.project.instructions.is_empty());
    stack.redo(&mut current.project).unwrap();
    assert_eq!(current, working);
    history::save_editor(&conn, &current, &stack, Some(generation), true).unwrap();
    assert_eq!(NativeSnapshot::read(&conn).unwrap(), working);
    assert_eq!(
        history::read_version(&conn, 1).unwrap().project.info.name,
        "Working predecessor"
    );
}
#[test]
fn malformed_native_runs_and_unknown_styles_refuse_without_guessing() {
    for mutation in [
        "UPDATE instruction_run SET position=9 WHERE position=0",
        "UPDATE instruction_run SET style=99 WHERE position=0",
        "UPDATE instruction_run SET text='' WHERE position=0",
        "UPDATE instruction_run SET style=0",
        "UPDATE instruction_run SET text=printf('%.*c',65537,'x') WHERE position=0",
    ] {
        let (_dir, path) = copied_predecessor();
        let conn = knx_store::open_existing_and_migrate(&path).unwrap();
        let mut snapshot = NativeSnapshot::read(&conn).unwrap();
        let id = snapshot.project.devices.iter().next().unwrap().id;
        snapshot.project.instructions.insert(id, styled());
        let out = knx_store::open_and_migrate_in_memory().unwrap();
        knx_store::save_project(&out, &snapshot.project).unwrap();
        out.execute_batch("PRAGMA ignore_check_constraints = ON")
            .unwrap();
        out.execute_batch(mutation).unwrap();
        assert!(
            knx_store::load_project(&out).is_err(),
            "corrupt native instruction rows must refuse"
        );
    }
}
#[test]
fn corrupt_or_future_predecessor_images_never_gain_a_derived_history() {
    for mutation in [
        "UPDATE project_history_context SET image_hash=printf('%064d',0)",
        "UPDATE project_history_state SET working_hash=printf('%064d',0)",
        "UPDATE project_history_state SET baseline_hash=printf('%064d',0)",
    ] {
        let (_dir, path) = copied_predecessor();
        let conn = knx_store::open_existing_and_migrate(&path).unwrap();
        conn.execute_batch(mutation).unwrap();
        assert!(history::load_editor(&conn).is_err());
    }
    let (_dir, path) = copied_predecessor();
    let conn = knx_store::Connection::open(&path).unwrap();
    conn.pragma_update(None, "user_version", 99).unwrap();
    drop(conn);
    let before = std::fs::read(&path).unwrap();
    assert!(knx_store::open_existing_read_only(&path).is_err());
    assert_eq!(std::fs::read(&path).unwrap(), before);
}
