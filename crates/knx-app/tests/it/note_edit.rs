//! Verifies private note previews, durable edits and unchanged refused operations.

use knx_app::note_edit::*;
use knx_core::note::ProjectNoteTarget;
use knx_core::{CommandStack, Language, Project, ProjectNoteId};
use knx_store::project_history::NativeSnapshot;

fn snapshot() -> NativeSnapshot {
    NativeSnapshot {
        project: Project::new(Language("en".into())),
        opaque: vec![],
        manufacturer_refs: vec![],
    }
}

fn revision() -> NoteRevision {
    NoteRevision {
        project_incarnation: "synthetic-load-1".into(),
        generation: 1,
    }
}

fn draft(title: &str) -> NoteDraft {
    NoteDraft {
        target: ProjectNoteTarget::Project,
        title: title.into(),
        body: "Literal Grüße🙂\r\n<img src=\"https://untrusted.invalid/never\">\n\t".into(),
        include_in_documentation: false,
    }
}

#[test]
fn create_preview_allocates_only_privately_and_apply_is_one_undo_step() {
    let mut state = snapshot();
    let before = state.clone();
    let rev = revision();
    let mut stack = CommandStack::new();
    let plan = preview_note_edit(
        &state,
        &rev,
        NoteAction::Create {
            draft: draft(" title "),
            position: 0,
        },
    )
    .expect("valid authored note must produce a preview");
    assert_eq!(state, before);
    assert_eq!(plan.created_id(), Some(ProjectNoteId(1)));
    assert!(!plan.is_no_op());
    let outcome = apply_note_edit(&mut state, &mut stack, &rev, &plan, None).unwrap();
    assert!(outcome.changed);
    assert_eq!(state.project.notes.len(), 1);
    let note = &state.project.notes[0];
    assert_eq!(note.id(), ProjectNoteId(1));
    assert_eq!(note.title(), " title ");
    assert_eq!(note.body(), draft("unused").body);
    assert!(!note.include_in_documentation());
    assert_eq!(stack.history_lengths(), (1, 0));
    stack.undo(&mut state.project).unwrap();
    assert!(state.project.notes.is_empty());
    assert_eq!(state.project.ids.peek_project_note(), 1);
    stack.redo(&mut state.project).unwrap();
    assert_eq!(state.project.notes.len(), 1);
    assert_eq!(state.project.notes[0].id(), ProjectNoteId(1));
}

#[test]
fn replace_move_remove_preview_exact_changes_and_restore_through_one_step_undo() {
    use knx_core::note::ProjectNote;
    let mut state = snapshot();
    for id in 1..=2 {
        knx_core::Command::InsertProjectNote {
            note: ProjectNote::new(
                ProjectNoteId(id),
                ProjectNoteTarget::Project,
                format!("note {id}"),
                format!("body {id}"),
            )
            .unwrap(),
            position: state.project.notes.len(),
        }
        .apply(&mut state.project)
        .unwrap();
    }
    let mut edited = draft(" edited Grüße ");
    edited.include_in_documentation = true;
    let actions = [
        NoteAction::Replace {
            id: ProjectNoteId(1),
            draft: edited.clone(),
        },
        NoteAction::Move {
            id: ProjectNoteId(1),
            position: 1,
        },
        NoteAction::Remove {
            id: ProjectNoteId(1),
        },
    ];
    let mut stack = CommandStack::new();
    let rev = revision();
    for (index, action) in actions.into_iter().enumerate() {
        let before = state.clone();
        let lengths = stack.history_lengths();
        let plan = preview_note_edit(&state, &rev, action)
            .expect("valid existing-note operation must produce a preview");
        assert_eq!(plan.created_id(), None);
        assert!(!plan.is_no_op());
        assert_eq!(state, before);
        assert!(
            apply_note_edit(&mut state, &mut stack, &rev, &plan, None)
                .unwrap()
                .changed
        );
        assert_eq!(stack.history_lengths(), (lengths.0 + 1, 0));
        match index {
            0 => {
                assert_eq!(state.project.notes[0].title(), edited.title);
                assert_eq!(state.project.notes[0].body(), edited.body);
                assert!(state.project.notes[0].include_in_documentation());
            }
            1 => assert_eq!(
                state
                    .project
                    .notes
                    .iter()
                    .map(|n| n.id())
                    .collect::<Vec<_>>(),
                [ProjectNoteId(2), ProjectNoteId(1)]
            ),
            _ => assert_eq!(
                state
                    .project
                    .notes
                    .iter()
                    .map(|n| n.id())
                    .collect::<Vec<_>>(),
                [ProjectNoteId(2)]
            ),
        }
        let after = state.clone();
        stack.undo(&mut state.project).unwrap();
        assert_eq!(state, before);
        stack.redo(&mut state.project).unwrap();
        assert_eq!(state, after);
        assert_eq!(state.project.ids.peek_project_note(), 2);
    }
}

#[test]
fn identical_replace_and_move_are_noops_that_preserve_redo_without_touching_native_path() {
    use knx_core::note::ProjectNote;
    let mut state = snapshot();
    let rev = revision();
    let mut stack = CommandStack::new();
    let current = draft("note");
    knx_core::Command::InsertProjectNote {
        note: ProjectNote::new(
            ProjectNoteId(1),
            current.target,
            current.title.clone(),
            current.body.clone(),
        )
        .unwrap(),
        position: 0,
    }
    .apply(&mut state.project)
    .unwrap();
    stack
        .do_command(
            &mut state.project,
            knx_core::Command::ReplaceProjectNote {
                note: ProjectNote::new(
                    ProjectNoteId(1),
                    current.target,
                    "temporary".into(),
                    current.body.clone(),
                )
                .unwrap(),
            },
        )
        .unwrap();
    stack.undo(&mut state.project).unwrap();
    let dir = tempfile::tempdir().unwrap();
    let missing = dir.path().join("must-not-be-created.knxdb");
    for action in [
        NoteAction::Replace {
            id: ProjectNoteId(1),
            draft: current.clone(),
        },
        NoteAction::Move {
            id: ProjectNoteId(1),
            position: 0,
        },
    ] {
        let before = state.clone();
        let lengths = stack.history_lengths();
        let plan = preview_note_edit(&state, &rev, action).unwrap();
        assert!(
            plan.is_no_op(),
            "identical authored note operation must be a no-op"
        );
        let outcome = apply_note_edit(
            &mut state,
            &mut stack,
            &rev,
            &plan,
            Some(NoteNativeTarget {
                path: &missing,
                history_generation: 0,
            }),
        )
        .unwrap();
        assert!(!outcome.changed);
        assert_eq!(outcome.native_generation, None);
        assert_eq!(state, before);
        assert_eq!(stack.history_lengths(), lengths);
        assert!(stack.can_redo());
        assert!(!missing.exists());
    }
}

#[test]
fn malformed_revision_cannot_create_a_plan_or_consume_an_id() {
    let state = snapshot();
    let before = state.clone();
    for rev in [
        NoteRevision {
            project_incarnation: String::new(),
            generation: 1,
        },
        NoteRevision {
            project_incarnation: "x".repeat(129),
            generation: 1,
        },
        NoteRevision {
            project_incarnation: "load".into(),
            generation: 9_007_199_254_740_992,
        },
    ] {
        let result = preview_note_edit(
            &state,
            &rev,
            NoteAction::Create {
                draft: draft("valid"),
                position: 0,
            },
        );
        assert!(
            matches!(result, Err(NoteEditError::InvalidRevision)),
            "malformed revision must refuse before allocating a candidate note"
        );
        assert_eq!(state, before);
        assert_eq!(state.project.ids.peek_project_note(), 0);
    }
}

fn retained_snapshot() -> NativeSnapshot {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("synthetic-predecessor.knxdb");
    std::fs::write(
        &path,
        include_bytes!("../../../knx-store/tests/fixtures/native13-before-notes-saved.sqlite"),
    )
    .unwrap();
    let opened = knx_store::open_existing_read_only(&path).unwrap();
    let mut state = NativeSnapshot::read(&opened.conn).unwrap();
    assert!(
        !state.opaque.is_empty(),
        "synthetic retained-context fixture must be nonempty"
    );
    state.manufacturer_refs.push(knx_store::ManufacturerRef {
        source_path: "M-0000/Synthetic.xml".into(),
        sha256: state.opaque[0].sha256.clone(),
        len: 0,
        kind: "SyntheticReference".into(),
    });
    state
}

#[test]
fn confirmed_native_edits_recover_notes_retained_context_and_both_history_stacks() {
    use knx_store::project_history as history;
    let mut state = retained_snapshot();
    let before = state.clone();
    let mut stack = CommandStack::new();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("notes.knxdb");
    let mut generation = knx_app::project_history::save_as(&path, &state, &stack).unwrap();
    let read = knx_store::open_existing_read_only(&path).unwrap();
    assert_eq!(NativeSnapshot::read(&read.conn).unwrap(), before);
    drop(read);
    let rev = revision();
    for action in [
        NoteAction::Create {
            draft: draft("one"),
            position: 0,
        },
        NoteAction::Replace {
            id: ProjectNoteId(1),
            draft: draft("two"),
        },
        NoteAction::Replace {
            id: ProjectNoteId(1),
            draft: draft("three"),
        },
    ] {
        let plan = preview_note_edit(&state, &rev, action).unwrap();
        let outcome = apply_note_edit(
            &mut state,
            &mut stack,
            &rev,
            &plan,
            Some(NoteNativeTarget {
                path: &path,
                history_generation: generation,
            }),
        )
        .expect("confirmed valid notes must persist to native working history");
        assert!(outcome.changed);
        assert!(outcome.native_generation.unwrap() > generation);
        generation = outcome.native_generation.unwrap();
        assert_eq!(state.opaque, before.opaque);
        assert_eq!(state.manufacturer_refs, before.manufacturer_refs);
        let read = knx_store::open_existing_read_only(&path).unwrap();
        assert_eq!(
            NativeSnapshot::read(&read.conn).unwrap(),
            before,
            "working edits must not silently overwrite the saved root"
        );
        assert_eq!(
            history::load_editor(&read.conn).unwrap().unwrap().working,
            state
        );
    }
    stack.undo(&mut state.project).unwrap();
    generation =
        knx_app::project_history::persist_working(&path, &state, &stack, generation).unwrap();
    let read = knx_store::open_existing_read_only(&path).unwrap();
    let editor = history::load_editor(&read.conn).unwrap().unwrap();
    assert_eq!(editor.generation, generation);
    assert_eq!(editor.working, state);
    assert_eq!(editor.undo.len(), 2);
    assert_eq!(editor.redo.len(), 1);
    let mut recovered = editor.working;
    let mut restored = CommandStack::from_snapshot_states(editor.undo, editor.redo);
    restored.redo(&mut recovered.project).unwrap();
    assert_eq!(recovered.project.notes[0].title(), "three");
    restored.undo(&mut recovered.project).unwrap();
    restored.undo(&mut recovered.project).unwrap();
    assert_eq!(recovered.project.notes[0].title(), "one");
    assert_eq!(recovered.project.ids.peek_project_note(), 1);
    assert_eq!(recovered.opaque, before.opaque);
    assert_eq!(recovered.manufacturer_refs, before.manufacturer_refs);
}

#[test]
fn invalid_note_actions_do_not_change_live_content_context_or_allocators() {
    use knx_core::{DeviceId, IdAllocators};
    let state = retained_snapshot();
    let mut invalid_target = draft("valid");
    invalid_target.target = ProjectNoteTarget::Device(DeviceId(u32::MAX));
    let mut blank = draft(" 	\n");
    blank.body = "valid".into();
    let mut control = draft("valid");
    control.body = "bad\0body".into();
    for action in [
        NoteAction::Create {
            draft: invalid_target,
            position: 0,
        },
        NoteAction::Create {
            draft: blank,
            position: 0,
        },
        NoteAction::Create {
            draft: control,
            position: 0,
        },
        NoteAction::Create {
            draft: draft(&"x".repeat(513)),
            position: 0,
        },
        NoteAction::Create {
            draft: draft("valid"),
            position: 1,
        },
        NoteAction::Replace {
            id: ProjectNoteId(0),
            draft: draft("valid"),
        },
        NoteAction::Replace {
            id: ProjectNoteId(99),
            draft: draft("valid"),
        },
        NoteAction::Remove {
            id: ProjectNoteId(99),
        },
        NoteAction::Move {
            id: ProjectNoteId(99),
            position: 0,
        },
    ] {
        let before = state.clone();
        assert!(preview_note_edit(&state, &revision(), action).is_err());
        assert_eq!(state, before);
    }
    let mut exhausted = state;
    exhausted.project.ids = IdAllocators::default().with_project_note_count(u32::MAX);
    let before = exhausted.clone();
    assert!(matches!(
        preview_note_edit(
            &exhausted,
            &revision(),
            NoteAction::Create {
                draft: draft("valid"),
                position: 0
            }
        ),
        Err(NoteEditError::Allocation(_))
    ));
    assert_eq!(exhausted, before);
}

#[test]
fn discarded_preview_does_not_reserve_or_consume_a_live_id() {
    let state = retained_snapshot();
    let before = state.clone();
    {
        let plan = preview_note_edit(
            &state,
            &revision(),
            NoteAction::Create {
                draft: draft("cancel"),
                position: 0,
            },
        )
        .unwrap();
        assert_eq!(plan.created_id(), Some(ProjectNoteId(1)));
    }
    assert_eq!(state, before);
    let next = preview_note_edit(
        &state,
        &revision(),
        NoteAction::Create {
            draft: draft("next"),
            position: 0,
        },
    )
    .unwrap();
    assert_eq!(next.created_id(), Some(ProjectNoteId(1)));
}

#[test]
fn model_opaque_manifest_and_incarnation_changes_refuse_stale_apply_without_touching_history() {
    for kind in 0..5 {
        let mut state = retained_snapshot();
        let mut rev = revision();
        let plan = preview_note_edit(
            &state,
            &rev,
            NoteAction::Create {
                draft: draft("pending"),
                position: 0,
            },
        )
        .unwrap();
        match kind {
            0 => state.project.info.name = "concurrent model".into(),
            1 => state.opaque[0].bytes.push(b' '),
            2 => state.manufacturer_refs[0].kind = "changed context".into(),
            3 => rev.project_incarnation = "other load".into(),
            _ => rev.generation += 1,
        }
        let before = state.clone();
        let mut stack = CommandStack::new();
        assert_eq!(
            apply_note_edit(&mut state, &mut stack, &rev, &plan, None),
            Err(NoteEditError::Conflict)
        );
        assert_eq!(state, before);
        assert_eq!(stack.history_lengths(), (0, 0));
    }
}

#[test]
fn failed_native_admission_keeps_model_ids_originals_and_both_stacks_unchanged() {
    use knx_core::{Command, DeviceId};
    let mut state = retained_snapshot();
    let rev = revision();
    let mut stack = CommandStack::new();
    stack
        .do_command(
            &mut state.project,
            Command::RenameDevice {
                device: DeviceId(1),
                name: "temporary name".into(),
            },
        )
        .unwrap();
    stack.undo(&mut state.project).unwrap();
    let plan = preview_note_edit(
        &state,
        &rev,
        NoteAction::Create {
            draft: draft("pending"),
            position: 0,
        },
    )
    .unwrap();
    let before = state.clone();
    let lengths = stack.history_lengths();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("missing.knxdb");
    assert!(matches!(
        apply_note_edit(
            &mut state,
            &mut stack,
            &rev,
            &plan,
            Some(NoteNativeTarget {
                path: &path,
                history_generation: 0
            })
        ),
        Err(NoteEditError::Persistence(_))
    ));
    assert_eq!(state, before);
    assert_eq!(stack.history_lengths(), lengths);
    assert!(!path.exists());
    stack.redo(&mut state.project).unwrap();
    assert_eq!(
        state.project.devices.get(DeviceId(1)).unwrap().name,
        "temporary name"
    );
}

#[test]
fn another_native_writer_conflicts_without_overwriting_its_working_state_or_file() {
    use knx_store::project_history as history;
    let mut state = retained_snapshot();
    let before = state.clone();
    let mut stack = CommandStack::new();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("shared.knxdb");
    let generation = knx_app::project_history::save_as(&path, &state, &stack).unwrap();
    let plan = preview_note_edit(
        &state,
        &revision(),
        NoteAction::Create {
            draft: draft("pending"),
            position: 0,
        },
    )
    .unwrap();
    let mut other = state.clone();
    other.project.info.name = "another writer".into();
    let newer =
        knx_app::project_history::persist_working(&path, &other, &stack, generation).unwrap();
    let bytes = std::fs::read(&path).unwrap();
    assert!(matches!(
        apply_note_edit(
            &mut state,
            &mut stack,
            &revision(),
            &plan,
            Some(NoteNativeTarget {
                path: &path,
                history_generation: generation
            })
        ),
        Err(NoteEditError::Persistence(_))
    ));
    assert_eq!(state, before);
    assert_eq!(stack.history_lengths(), (0, 0));
    assert_eq!(std::fs::read(&path).unwrap(), bytes);
    let opened = knx_store::open_existing_read_only(&path).unwrap();
    let editor = history::load_editor(&opened.conn).unwrap().unwrap();
    assert_eq!(editor.working, other);
    assert_eq!(editor.generation, newer);
}

#[test]
fn a_plain_save_that_cleared_history_conflicts_even_when_generation_is_zero() {
    let mut state = retained_snapshot();
    let before = state.clone();
    let mut stack = CommandStack::new();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("no-journal.knxdb");
    let conn = knx_store::open_and_migrate(&path).unwrap();
    knx_store::save_project_with_passthrough(
        &conn,
        &state.project,
        &state.opaque,
        &state.manufacturer_refs,
    )
    .unwrap();
    let plan = preview_note_edit(
        &state,
        &revision(),
        NoteAction::Create {
            draft: draft("pending"),
            position: 0,
        },
    )
    .unwrap();
    let mut other = state.clone();
    other.project.info.name = "plain writer changed root".into();
    knx_store::save_project_with_passthrough(
        &conn,
        &other.project,
        &other.opaque,
        &other.manufacturer_refs,
    )
    .unwrap();
    drop(conn);
    let bytes = std::fs::read(&path).unwrap();
    assert!(matches!(
        apply_note_edit(
            &mut state,
            &mut stack,
            &revision(),
            &plan,
            Some(NoteNativeTarget {
                path: &path,
                history_generation: 0
            })
        ),
        Err(NoteEditError::Persistence(_))
    ));
    assert_eq!(state, before);
    assert_eq!(stack.history_lengths(), (0, 0));
    assert_eq!(std::fs::read(&path).unwrap(), bytes);
    let opened = knx_store::open_existing_read_only(&path).unwrap();
    assert_eq!(NativeSnapshot::read(&opened.conn).unwrap(), other);
}

#[test]
fn history_budget_failure_keeps_existing_native_file_and_live_stack_unchanged() {
    let mut state = retained_snapshot();
    let before = state.clone();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("history-budget.knxdb");
    let initial = CommandStack::new();
    let generation = knx_app::project_history::save_as(&path, &state, &initial).unwrap();
    let mut stack = CommandStack::from_snapshot_states(
        vec![state.project.clone(); knx_store::project_history::MAX_STACK_STATES],
        vec![],
    );
    let bytes = std::fs::read(&path).unwrap();
    let plan = preview_note_edit(
        &state,
        &revision(),
        NoteAction::Create {
            draft: draft("pending"),
            position: 0,
        },
    )
    .unwrap();
    assert!(matches!(
        apply_note_edit(
            &mut state,
            &mut stack,
            &revision(),
            &plan,
            Some(NoteNativeTarget {
                path: &path,
                history_generation: generation
            })
        ),
        Err(NoteEditError::Persistence(_))
    ));
    assert_eq!(state, before);
    assert_eq!(
        stack.history_lengths(),
        (knx_store::project_history::MAX_STACK_STATES, 0)
    );
    assert_eq!(std::fs::read(&path).unwrap(), bytes);
    let opened = knx_store::open_existing_read_only(&path).unwrap();
    assert_eq!(NativeSnapshot::read(&opened.conn).unwrap(), before);
}

#[test]
fn exclusive_native_lock_refuses_without_consuming_live_id_or_history() {
    let mut state = retained_snapshot();
    let before = state.clone();
    let mut stack = CommandStack::new();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("locked.knxdb");
    let generation = knx_app::project_history::save_as(&path, &state, &stack).unwrap();
    let bytes = std::fs::read(&path).unwrap();
    let conn = knx_store::open_existing_and_migrate(&path).unwrap();
    conn.execute_batch("BEGIN EXCLUSIVE").unwrap();
    let plan = preview_note_edit(
        &state,
        &revision(),
        NoteAction::Create {
            draft: draft("pending"),
            position: 0,
        },
    )
    .unwrap();
    let result = apply_note_edit(
        &mut state,
        &mut stack,
        &revision(),
        &plan,
        Some(NoteNativeTarget {
            path: &path,
            history_generation: generation,
        }),
    );
    conn.execute_batch("ROLLBACK").unwrap();
    drop(conn);
    assert!(matches!(result, Err(NoteEditError::Persistence(_))));
    assert_eq!(state, before);
    assert_eq!(stack.history_lengths(), (0, 0));
    assert_eq!(std::fs::read(&path).unwrap(), bytes);
    let opened = knx_store::open_existing_read_only(&path).unwrap();
    assert_eq!(NativeSnapshot::read(&opened.conn).unwrap(), before);
}
