//! Verifies previewed instruction edits and atomic native recovery persistence.
use knx_app::instruction_edit::*;
use knx_core::instruction::{InstructionDocument, InstructionRun};
use knx_core::{Command, CommandStack, DeviceId};
use knx_store::project_history::{self as history, NativeSnapshot};
use knx_testsupport::zip_with_entries;

fn source(value: &str) -> (NativeSnapshot, DeviceId) {
    let value = value
        .replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;");
    let xml = format!(
        r#"<KNX xmlns="http://knx.org/xml/project/23"><Project Id="P-0001"><Installations><Installation InstallationId="0"><Topology><UnassignedDevices><DeviceInstance Id="P-0001-0_DI-1" Name="Synthetic" InstallationHints="{value}"/></UnassignedDevices></Topology></Installation></Installations></Project></KNX>"#
    );
    let info = r#"<KNX xmlns="http://knx.org/xml/project/23"><Project Id="P-0001"><ProjectInformation Name="Instruction edits"/></Project></KNX>"#;
    let dir = tempfile::tempdir().unwrap();
    let conn = knx_store::open_and_migrate(&dir.path().join("import.knxdb")).unwrap();
    let bytes = zip_with_entries(&[
        ("P-0001.signature", b""),
        ("P-0001/0.xml", xml.as_bytes()),
        ("P-0001/Project.xml", info.as_bytes()),
    ]);
    let result =
        knx_app::import_ets_project_bytes(bytes, "synthetic.knxproj", &conn, Default::default())
            .unwrap();
    let id = result.project.devices.iter().next().unwrap().id;
    (
        NativeSnapshot {
            project: result.project,
            opaque: knx_store::load_opaque(&conn).unwrap(),
            manufacturer_refs: vec![],
        },
        id,
    )
}
fn revision() -> InstructionRevision {
    InstructionRevision {
        project_incarnation: "synthetic-load-1".into(),
        generation: 1,
    }
}
fn doc(text: &str) -> InstructionDocument {
    InstructionDocument::new(vec![InstructionRun {
        text: text.into(),
        style: Default::default(),
    }])
    .unwrap()
}
#[test]
fn supported_edit_is_one_undo_step_and_never_rewrites_originals() {
    let (mut snapshot, id) = source(r"{\rtf1 original}");
    let before = snapshot.clone();
    let rev = revision();
    let plan = preview_instruction_edit(&snapshot, &rev, id, Some(doc("edited Grüße"))).unwrap();
    assert!(!plan.conversion_required());
    assert!(!plan.is_no_op());
    assert_eq!(plan.device(), id);
    assert_eq!(plan.replacement().unwrap().plain_text(), "edited Grüße");
    let mut stack = CommandStack::new();
    let outcome =
        apply_instruction_edit(&mut snapshot, &mut stack, &rev, &plan, false, None).unwrap();
    assert!(outcome.changed);
    assert_eq!(stack.history_lengths(), (1, 0));
    assert_eq!(snapshot.opaque, before.opaque);
    assert_eq!(
        snapshot.project.instructions[&id].plain_text(),
        "edited Grüße"
    );
    stack.undo(&mut snapshot.project).unwrap();
    assert_eq!(snapshot, before);
    stack.redo(&mut snapshot.project).unwrap();
    assert_eq!(
        snapshot.project.instructions[&id].plain_text(),
        "edited Grüße"
    );
}
#[test]
fn unsupported_and_malformed_sources_require_explicit_conversion_consent() {
    for value in [r"{\rtf1 a{\*\unknown retained}b}", r"{\rtf1 unfinished"] {
        let (mut snapshot, id) = source(value);
        let before = snapshot.clone();
        let rev = revision();
        let mut stack = CommandStack::new();
        let plan =
            preview_instruction_edit(&snapshot, &rev, id, Some(doc("explicit subset"))).unwrap();
        assert!(plan.conversion_required());
        assert_eq!(
            apply_instruction_edit(&mut snapshot, &mut stack, &rev, &plan, false, None),
            Err(InstructionEditError::ConversionConsentRequired)
        );
        assert_eq!(snapshot, before);
        assert_eq!(stack.history_lengths(), (0, 0));
        assert!(
            apply_instruction_edit(&mut snapshot, &mut stack, &rev, &plan, true, None)
                .unwrap()
                .changed
        );
        assert_eq!(snapshot.opaque, before.opaque);
    }
}
#[test]
fn identical_supported_source_edit_preserves_absence_and_redo() {
    let (mut snapshot, id) = source("unchanged");
    let rev = revision();
    let mut stack = CommandStack::new();
    stack
        .do_command(
            &mut snapshot.project,
            Command::SetDeviceDescription {
                device: id,
                description: Some("temporary".into()),
            },
        )
        .unwrap();
    stack.undo(&mut snapshot.project).unwrap();
    let before = snapshot.clone();
    let lengths = stack.history_lengths();
    let plan = preview_instruction_edit(&snapshot, &rev, id, Some(doc("unchanged"))).unwrap();
    assert!(plan.is_no_op());
    assert!(
        !apply_instruction_edit(&mut snapshot, &mut stack, &rev, &plan, false, None)
            .unwrap()
            .changed
    );
    assert_eq!(snapshot, before);
    assert_eq!(stack.history_lengths(), lengths);
    assert!(stack.can_redo());
}
#[test]
fn model_source_and_project_incarnation_changes_refuse_stale_preview() {
    for kind in 0..4 {
        let (mut snapshot, id) = source("original");
        let mut rev = revision();
        let plan = preview_instruction_edit(&snapshot, &rev, id, Some(doc("edited"))).unwrap();
        match kind {
            0 => snapshot.project.info.name = "concurrent change".into(),
            1 => snapshot
                .opaque
                .iter_mut()
                .find(|e| e.source_path == "P-0001/0.xml" && e.xpath.is_empty())
                .unwrap()
                .bytes
                .push(b' '),
            2 => rev.project_incarnation = "other-load".into(),
            _ => rev.generation += 1,
        }
        let before = snapshot.clone();
        let mut stack = CommandStack::new();
        assert!(
            apply_instruction_edit(&mut snapshot, &mut stack, &rev, &plan, true, None).is_err()
        );
        assert_eq!(snapshot, before);
        assert_eq!(stack.history_lengths(), (0, 0));
    }
}
#[test]
fn invalid_revision_and_unknown_target_cannot_produce_a_preview() {
    let (snapshot, id) = source("original");
    let mut rev = revision();
    rev.project_incarnation.clear();
    assert!(preview_instruction_edit(&snapshot, &rev, id, Some(doc("edited"))).is_err());
    assert!(preview_instruction_edit(
        &snapshot,
        &revision(),
        DeviceId(u32::MAX),
        Some(doc("edited"))
    )
    .is_err());
}
#[test]
fn failed_native_persistence_leaves_live_project_and_stack_unchanged() {
    let (mut snapshot, id) = source("original");
    let before = snapshot.clone();
    let rev = revision();
    let plan = preview_instruction_edit(&snapshot, &rev, id, Some(doc("edited"))).unwrap();
    let mut stack = CommandStack::new();
    let dir = tempfile::tempdir().unwrap();
    let missing = dir.path().join("missing.knxdb");
    assert!(apply_instruction_edit(
        &mut snapshot,
        &mut stack,
        &rev,
        &plan,
        false,
        Some(InstructionNativeTarget {
            path: &missing,
            history_generation: 1
        })
    )
    .is_err());
    assert_eq!(snapshot, before);
    assert_eq!(stack.history_lengths(), (0, 0));
    assert!(!missing.exists());
}
#[test]
fn confirmed_native_edit_recovers_after_reopen_with_undo_and_original_bytes() {
    let (mut snapshot, id) = source("original");
    let before = snapshot.clone();
    let mut stack = CommandStack::new();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("native.knxdb");
    let generation = knx_app::project_history::save_as(&path, &snapshot, &stack).unwrap();
    let rev = InstructionRevision {
        project_incarnation: "synthetic-load".into(),
        generation: generation as u64,
    };
    let plan = preview_instruction_edit(&snapshot, &rev, id, Some(doc("edited"))).unwrap();
    let outcome = apply_instruction_edit(
        &mut snapshot,
        &mut stack,
        &rev,
        &plan,
        false,
        Some(InstructionNativeTarget {
            path: &path,
            history_generation: generation,
        }),
    )
    .unwrap();
    assert!(outcome.changed);
    assert!(outcome.native_generation.unwrap() > generation);
    let conn = knx_store::open_existing_and_migrate(&path).unwrap();
    let recovered = history::load_editor(&conn).unwrap().unwrap();
    assert_eq!(recovered.working, snapshot);
    assert_eq!(recovered.working.opaque, before.opaque);
    let mut restored = recovered.working;
    let mut stack = CommandStack::from_snapshot_states(recovered.undo, recovered.redo);
    stack.undo(&mut restored.project).unwrap();
    assert_eq!(restored, before);
}
#[test]
fn reset_restores_original_without_conversion_and_preview_debug_is_payload_free() {
    let (mut snapshot, id) = source(r"{\rtf1 a{\*\unknown retained-secret-marker}b}");
    snapshot
        .project
        .instructions
        .insert(id, doc("edited-secret-marker"));
    let rev = revision();
    let plan = preview_instruction_edit(&snapshot, &rev, id, None).unwrap();
    assert!(!plan.conversion_required());
    let debug = format!("{plan:?}");
    assert!(!debug.contains("secret-marker"));
    let mut stack = CommandStack::new();
    assert!(
        apply_instruction_edit(&mut snapshot, &mut stack, &rev, &plan, false, None)
            .unwrap()
            .changed
    );
    assert!(snapshot.project.instructions.is_empty());
    assert!(plan
        .original()
        .unwrap()
        .hints
        .as_ref()
        .unwrap()
        .original
        .contains("retained-secret-marker"));
}

#[test]
fn stale_native_generation_refuses_without_touching_either_live_state_or_the_other_writer() {
    let (mut snapshot, id) = source("original");
    let before = snapshot.clone();
    let mut stack = CommandStack::new();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("native.knxdb");
    let generation = knx_app::project_history::save_as(&path, &snapshot, &stack).unwrap();
    let rev = revision();
    let plan = preview_instruction_edit(&snapshot, &rev, id, Some(doc("candidate"))).unwrap();
    let mut other = snapshot.clone();
    other.project.info.name = "Other writer".into();
    let later =
        knx_app::project_history::persist_working(&path, &other, &stack, generation).unwrap();
    assert!(apply_instruction_edit(
        &mut snapshot,
        &mut stack,
        &rev,
        &plan,
        false,
        Some(InstructionNativeTarget {
            path: &path,
            history_generation: generation
        })
    )
    .is_err());
    assert_eq!(snapshot, before);
    assert_eq!(stack.history_lengths(), (0, 0));
    let conn = knx_store::open_existing_and_migrate(&path).unwrap();
    let recovered = history::load_editor(&conn).unwrap().unwrap();
    assert_eq!(recovered.working, other);
    assert_eq!(recovered.generation, later);
}
#[test]
fn already_approved_override_can_be_edited_without_reconverting_the_retained_original() {
    let (mut snapshot, id) = source(r"{\rtf1 bad{\*\unknown retained}source}");
    snapshot.project.instructions.insert(id, doc("approved"));
    let rev = revision();
    let mut stack = CommandStack::new();
    let plan = preview_instruction_edit(&snapshot, &rev, id, Some(doc("revised"))).unwrap();
    assert!(!plan.conversion_required());
    assert!(
        apply_instruction_edit(&mut snapshot, &mut stack, &rev, &plan, false, None)
            .unwrap()
            .changed
    );
    let plan = preview_instruction_edit(&snapshot, &rev, id, Some(doc("revised"))).unwrap();
    assert!(plan.is_no_op());
    assert!(
        !apply_instruction_edit(&mut snapshot, &mut stack, &rev, &plan, false, None)
            .unwrap()
            .changed
    );
    assert_eq!(stack.history_lengths(), (1, 0));
}

#[test]
fn an_original_replaced_consistently_between_preview_and_apply_is_a_conflict() {
    // Not a broken hash (that is a source error): another, internally consistent original.
    let (mut snapshot, id) = source("original");
    let (other, _) = source("another original");
    let rev = revision();
    let plan = preview_instruction_edit(&snapshot, &rev, id, Some(doc("edited"))).unwrap();
    let pick = |s: &NativeSnapshot| {
        s.opaque
            .iter()
            .position(|e| e.source_path == "P-0001/0.xml" && e.xpath.is_empty())
            .unwrap()
    };
    let (mine, theirs) = (pick(&snapshot), pick(&other));
    snapshot.opaque[mine] = other.opaque[theirs].clone();
    let before = snapshot.clone();
    let mut stack = CommandStack::new();
    let refused = apply_instruction_edit(&mut snapshot, &mut stack, &rev, &plan, true, None);
    assert!(
        matches!(refused, Err(InstructionEditError::Conflict)),
        "{refused:?}"
    );
    assert_eq!(snapshot, before);
    assert_eq!(stack.history_lengths(), (0, 0));
}
