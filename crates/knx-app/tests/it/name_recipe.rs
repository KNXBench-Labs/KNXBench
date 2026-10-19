//! Verifies deterministic literal naming, atomic undo and source-bound admission.

#[test]
fn offline_facade_fingerprint_is_identical_to_the_previous_importer_hash_contract() {
    let snapshot = state();
    let revision = revision();
    let intent = intent();
    let exact_ids: Vec<u32> = intent.ids.iter().map(|id| id.0).collect();
    let plan: knx_edit::name_recipe::NameRecipePlan =
        preview_name_recipe(&snapshot, &revision, intent.clone()).unwrap();
    let authority = "fixed-synthetic-offline-authority";
    let legacy_transcript = serde_json::to_vec(&(
        "literal-ga-name-v1",
        authority,
        &revision,
        snapshot.semantic_hash().unwrap(),
        exact_ids,
        &intent.prefix,
        &intent.suffix,
        plan.preview(),
    ))
    .unwrap();
    assert_eq!(
        plan.confirmation_token(authority).unwrap(),
        knx_etsproj::opaque::sha256_hex(&legacy_transcript),
    );
    let facade_intent: knx_edit::name_recipe::NameRecipeIntent = intent;
    let facade_revision: knx_edit::name_recipe::NameRecipeRevision = revision;
    let direct =
        knx_edit::name_recipe::preview_name_recipe(&snapshot, &facade_revision, facade_intent)
            .unwrap();
    assert_eq!(direct.preview(), plan.preview());
    assert_eq!(
        direct.confirmation_token(authority).unwrap(),
        plan.confirmation_token(authority).unwrap()
    );
}

use knx_app::name_recipe::*;
use knx_core::{
    CommandStack, CompletionStatus, GroupAddress, GroupAddressEntry, GroupAddressId, Installation,
    InstallationId, Language, Project, SourceRef, Topology,
};
use knx_store::project_history::NativeSnapshot;

fn state() -> NativeSnapshot {
    let mut project = Project::new(Language("en".into()));
    project.installations.push(Installation {
        id: InstallationId(0),
        name: "unchanged".into(),
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
    for (address, name) in [("1/1/1", "  Küche 🛠 "), ("1/1/2", "Other")] {
        let id = project.ids.next_group_address_id().unwrap();
        project.installations[0]
            .group_addresses
            .push(GroupAddressEntry {
                id,
                source: SourceRef {
                    path: "synthetic.xml".into(),
                    ets_id: format!("GA-{}", id.0),
                },
                name: name.into(),
                address: GroupAddress::parse(address, project.info.group_address_style).unwrap(),
                central: true,
                unfiltered: true,
                range: None,
                declared_dpt: Default::default(),
            });
    }
    NativeSnapshot {
        project,
        opaque: vec![],
        manufacturer_refs: vec![],
    }
}
fn revision() -> NameRecipeRevision {
    NameRecipeRevision {
        project_incarnation: "synthetic-1".into(),
        generation: 1,
    }
}
fn intent() -> NameRecipeIntent {
    NameRecipeIntent {
        ids: vec![GroupAddressId(2), GroupAddressId(1)],
        prefix: "EG ".into(),
        suffix: " /literal".into(),
    }
}

#[test]
fn private_ordered_literal_preview_and_atomic_apply_preserve_every_non_name_field() {
    let mut snapshot = state();
    let before = snapshot.clone();
    let rev = revision();
    let mut stack = CommandStack::new();
    let plan = preview_name_recipe(&snapshot, &rev, intent()).unwrap();
    assert_eq!(snapshot, before);
    assert_eq!(plan.preview().rows[0].id, GroupAddressId(2));
    assert_eq!(plan.preview().rows[1].before, "  Küche 🛠 ");
    assert_eq!(plan.preview().rows[1].after, "EG   Küche 🛠  /literal");
    assert_eq!(plan.preview().changed, 2);
    let same = preview_name_recipe(&snapshot, &rev, intent()).unwrap();
    assert_eq!(
        plan.confirmation_token("synthetic-target").unwrap(),
        same.confirmation_token("synthetic-target").unwrap()
    );
    let result = apply_name_recipe(&mut snapshot, &mut stack, &rev, &plan, None).unwrap();
    assert!(result.changed);
    assert_eq!(stack.history_lengths(), (1, 0));
    let after = snapshot.clone();
    stack.undo(&mut snapshot.project).unwrap();
    assert_eq!(snapshot, before);
    stack.redo(&mut snapshot.project).unwrap();
    assert_eq!(snapshot, after);
    let mut restored = after;
    for (entry, original) in restored.project.installations[0]
        .group_addresses
        .iter_mut()
        .zip(&before.project.installations[0].group_addresses)
    {
        entry.name = original.name.clone();
    }
    assert_eq!(restored, before);
}
#[test]
fn invalid_selection_or_name_refuses_the_complete_plan_without_modifying_source() {
    let snapshot = state();
    let before = snapshot.clone();
    let rev = revision();
    for ids in [
        vec![],
        vec![GroupAddressId(1), GroupAddressId(1)],
        vec![GroupAddressId(1), GroupAddressId(u32::MAX)],
    ] {
        assert!(
            preview_name_recipe(&snapshot, &rev, NameRecipeIntent { ids, ..intent() }).is_err()
        );
    }
    for prefix in [
        "\n".into(),
        "\u{2028}".into(),
        "x".repeat(1024),
        "x".repeat(MAX_RECIPE_AFFIX_BYTES + 1),
    ] {
        assert!(
            preview_name_recipe(&snapshot, &rev, NameRecipeIntent { prefix, ..intent() }).is_err()
        );
    }
    let mut ambiguous = snapshot.clone();
    ambiguous
        .project
        .installations
        .push(ambiguous.project.installations[0].clone());
    assert!(preview_name_recipe(&ambiguous, &rev, intent()).is_err());
    assert_eq!(snapshot, before);
}

#[test]
fn no_op_keeps_redo_exceptional_imported_names_and_never_opens_native_target() {
    let mut snapshot = state();
    snapshot.project.installations[0].group_addresses[0].name = " \t".into();
    let mut stack = CommandStack::new();
    stack
        .do_command(
            &mut snapshot.project,
            knx_core::Command::RenameGroupAddress {
                id: GroupAddressId(2),
                name: "pending".into(),
            },
        )
        .unwrap();
    stack.undo(&mut snapshot.project).unwrap();
    let before = snapshot.clone();
    let dir = tempfile::tempdir().unwrap();
    let missing = dir.path().join("must-not-create.knxdb");
    let rev = revision();
    let plan = preview_name_recipe(
        &snapshot,
        &rev,
        NameRecipeIntent {
            prefix: String::new(),
            suffix: String::new(),
            ..intent()
        },
    )
    .unwrap();
    assert_eq!(plan.preview().changed, 0);
    assert!(!plan.preview().rows[1].changed);
    let outcome = apply_name_recipe(
        &mut snapshot,
        &mut stack,
        &rev,
        &plan,
        Some(NameRecipeNativeTarget {
            path: &missing,
            history_generation: 0,
        }),
    )
    .unwrap();
    assert!(!outcome.changed);
    assert_eq!(outcome.native_generation, None);
    assert_eq!(snapshot, before);
    assert_eq!(stack.history_lengths(), (0, 1));
    assert!(!missing.exists());
    let mut changed = rev.clone();
    changed.generation += 1;
    assert_eq!(
        apply_name_recipe(&mut snapshot, &mut stack, &changed, &plan, None),
        Err(NameRecipeError::Conflict)
    );
}
#[test]
fn naming_exceptional_imported_blank_name_restores_it_exactly_on_undo() {
    let mut snapshot = state();
    snapshot.project.installations[0].group_addresses[0].name = "  ".into();
    let before = snapshot.clone();
    let rev = revision();
    let mut stack = CommandStack::new();
    let plan = preview_name_recipe(&snapshot, &rev, intent()).unwrap();
    apply_name_recipe(&mut snapshot, &mut stack, &rev, &plan, None).unwrap();
    stack.undo(&mut snapshot.project).unwrap();
    assert_eq!(snapshot, before);
}
#[test]
fn explicit_budgets_and_malformed_revisions_do_not_create_a_plan() {
    let snapshot = state();
    for rev in [
        NameRecipeRevision {
            project_incarnation: String::new(),
            generation: 0,
        },
        NameRecipeRevision {
            project_incarnation: "x".repeat(129),
            generation: 0,
        },
        NameRecipeRevision {
            project_incarnation: "load".into(),
            generation: 9_007_199_254_740_992,
        },
    ] {
        assert!(matches!(
            preview_name_recipe(&snapshot, &rev, intent()),
            Err(NameRecipeError::InvalidRevision)
        ));
    }
    assert!(matches!(
        preview_name_recipe(
            &snapshot,
            &revision(),
            NameRecipeIntent {
                ids: vec![GroupAddressId(1); MAX_RECIPE_SELECTION + 1],
                ..intent()
            }
        ),
        Err(NameRecipeError::InvalidSelection)
    ));
    let mut long = snapshot.clone();
    long.project.installations[0].group_addresses[0].name =
        "x".repeat(MAX_RECIPE_PREVIEW_BYTES + 1);
    assert!(matches!(
        preview_name_recipe(
            &long,
            &revision(),
            NameRecipeIntent {
                prefix: String::new(),
                suffix: String::new(),
                ..intent()
            }
        ),
        Err(NameRecipeError::Budget)
    ));
}
#[test]
fn fingerprint_changes_with_exact_intent_order_context_or_retained_source() {
    let snapshot = state();
    let rev = revision();
    let first = preview_name_recipe(&snapshot, &rev, intent())
        .unwrap()
        .confirmation_token("target")
        .unwrap();
    let mut order = intent();
    order.ids.reverse();
    let mut suffix = intent();
    suffix.suffix.push(' ');
    for recipe in [order, suffix] {
        assert_ne!(
            preview_name_recipe(&snapshot, &rev, recipe)
                .unwrap()
                .confirmation_token("target")
                .unwrap(),
            first
        );
    }
    assert_ne!(
        preview_name_recipe(&snapshot, &rev, intent())
            .unwrap()
            .confirmation_token("other-target")
            .unwrap(),
        first
    );
    let mut changed = snapshot.clone();
    changed.project.info.name = "new identity".into();
    assert_ne!(
        preview_name_recipe(&changed, &rev, intent())
            .unwrap()
            .confirmation_token("target")
            .unwrap(),
        first
    );
}
fn retained_state() -> NativeSnapshot {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("synthetic-predecessor.knxdb");
    std::fs::write(
        &path,
        include_bytes!("../../../knx-store/tests/fixtures/native13-before-notes-saved.sqlite"),
    )
    .unwrap();
    let opened = knx_store::open_existing_read_only(&path).unwrap();
    let mut retained = NativeSnapshot::read(&opened.conn).unwrap();
    assert!(!retained.opaque.is_empty());
    retained.project = state().project;
    retained.manufacturer_refs.push(knx_store::ManufacturerRef {
        source_path: "M-0000/Synthetic.xml".into(),
        sha256: retained.opaque[0].sha256.clone(),
        len: 0,
        kind: "SyntheticReference".into(),
    });
    knx_core::Command::InsertProjectNote {
        note: knx_core::note::ProjectNote::new(
            knx_core::ProjectNoteId(1),
            knx_core::note::ProjectNoteTarget::Project,
            "unchanged note".into(),
            "literal body".into(),
        )
        .unwrap(),
        position: 0,
    }
    .apply(&mut retained.project)
    .unwrap();
    retained
}
#[test]
fn retained_context_changes_invalidate_a_pending_naming_plan() {
    for kind in 0..2 {
        let mut snapshot = retained_state();
        let rev = revision();
        let plan = preview_name_recipe(&snapshot, &rev, intent()).unwrap();
        if kind == 0 {
            snapshot.opaque[0].bytes.push(b' ');
        } else {
            snapshot.manufacturer_refs[0].kind = "changed context".into();
        }
        let before = snapshot.clone();
        let mut stack = CommandStack::new();
        assert_eq!(
            apply_name_recipe(&mut snapshot, &mut stack, &rev, &plan, None),
            Err(NameRecipeError::Conflict)
        );
        assert_eq!(snapshot, before);
        assert_eq!(stack.history_lengths(), (0, 0));
    }
}
#[test]
fn native_working_reopen_recovers_names_context_and_both_history_stacks() {
    use knx_store::project_history as history;
    let mut snapshot = retained_state();
    let root = snapshot.clone();
    let mut stack = CommandStack::new();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("native-recipe.knxdb");
    let mut generation = knx_app::project_history::save_as(&path, &snapshot, &stack).unwrap();
    let rev = revision();
    for prefix in ["EG ", "Floor "] {
        let plan = preview_name_recipe(
            &snapshot,
            &rev,
            NameRecipeIntent {
                prefix: prefix.into(),
                ..intent()
            },
        )
        .unwrap();
        generation = apply_name_recipe(
            &mut snapshot,
            &mut stack,
            &rev,
            &plan,
            Some(NameRecipeNativeTarget {
                path: &path,
                history_generation: generation,
            }),
        )
        .unwrap()
        .native_generation
        .unwrap();
    }
    stack.undo(&mut snapshot.project).unwrap();
    let conn = history::open_editor_store(&path, false).unwrap();
    generation = history::save_editor(&conn, &snapshot, &stack, Some(generation), false).unwrap();
    drop(conn);
    let opened = knx_store::open_existing_read_only(&path).unwrap();
    assert_eq!(NativeSnapshot::read(&opened.conn).unwrap(), root);
    let editor = history::load_editor(&opened.conn).unwrap().unwrap();
    assert_eq!(editor.generation, generation);
    assert_eq!(editor.working, snapshot);
    assert_eq!(editor.undo.len(), 1);
    assert_eq!(editor.redo.len(), 1);
    let mut restored = editor.working;
    let mut stack = CommandStack::from_snapshot_states(editor.undo, editor.redo);
    stack.redo(&mut restored.project).unwrap();
    stack.undo(&mut restored.project).unwrap();
    stack.undo(&mut restored.project).unwrap();
    assert_eq!(restored, root);
}
#[test]
fn failed_native_admission_preserves_live_model_both_stacks_and_existing_file() {
    use knx_store::project_history as history;
    for kind in 0..4 {
        let mut snapshot = retained_state();
        let mut stack = CommandStack::new();
        for name in ["first", "second"] {
            stack
                .do_command(
                    &mut snapshot.project,
                    knx_core::Command::RenameGroupAddress {
                        id: GroupAddressId(2),
                        name: name.into(),
                    },
                )
                .unwrap();
        }
        stack.undo(&mut snapshot.project).unwrap();
        assert_eq!(stack.history_lengths(), (1, 1));
        let before = snapshot.clone();
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("refusal.knxdb");
        let generation = knx_app::project_history::save_as(&path, &snapshot, &stack).unwrap();
        let rev = revision();
        let plan = preview_name_recipe(&snapshot, &rev, intent()).unwrap();
        let mut lock = None;
        match kind {
            0 => {
                let conn = history::open_editor_store(&path, false).unwrap();
                let mut other = snapshot.clone();
                other.project.info.name = "concurrent writer".into();
                history::save_editor(&conn, &other, &stack, Some(generation), false).unwrap();
            }
            1 => {
                let conn = knx_store::open_existing_and_migrate(&path).unwrap();
                let mut other = snapshot.clone();
                other.project.info.name = "plain writer".into();
                knx_store::save_project_with_passthrough(
                    &conn,
                    &other.project,
                    &other.opaque,
                    &other.manufacturer_refs,
                )
                .unwrap();
            }
            2 => {
                stack = CommandStack::from_snapshot_states(
                    vec![snapshot.project.clone(); history::MAX_STACK_STATES],
                    vec![snapshot.project.clone()],
                );
            }
            _ => {
                let conn = knx_store::open_existing_and_migrate(&path).unwrap();
                conn.execute_batch("BEGIN EXCLUSIVE").unwrap();
                lock = Some(conn);
            }
        }
        let lengths = stack.history_lengths();
        let mut expected_images = vec![];
        assert!(stack
            .visit_snapshot_states(&snapshot.project, |side, position, project| {
                expected_images.push((format!("{side:?}"), position, project.clone()));
                true
            })
            .unwrap());
        assert!(lengths.0 > 0 && lengths.1 > 0);
        let bytes = std::fs::read(&path).unwrap();
        let result = apply_name_recipe(
            &mut snapshot,
            &mut stack,
            &rev,
            &plan,
            Some(NameRecipeNativeTarget {
                path: &path,
                history_generation: generation,
            }),
        );
        if let Some(conn) = lock {
            conn.execute_batch("ROLLBACK").unwrap();
            drop(conn);
        }
        assert!(matches!(result, Err(NameRecipeError::Persistence(_))));
        assert_eq!(snapshot, before);
        assert_eq!(stack.history_lengths(), lengths);
        let mut actual_images = vec![];
        assert!(stack
            .visit_snapshot_states(&snapshot.project, |side, position, project| {
                actual_images.push((format!("{side:?}"), position, project.clone()));
                true
            })
            .unwrap());
        assert_eq!(actual_images, expected_images);
        assert_eq!(std::fs::read(&path).unwrap(), bytes);
    }
}

#[test]
fn plain_root_hash_conflict_at_zero_generation_preserves_native_bytes_and_live_state() {
    let mut snapshot = retained_state();
    let before = snapshot.clone();
    let mut stack = CommandStack::new();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("plain-root.knxdb");
    let conn = knx_store::open_and_migrate(&path).unwrap();
    knx_store::save_project_with_passthrough(
        &conn,
        &snapshot.project,
        &snapshot.opaque,
        &snapshot.manufacturer_refs,
    )
    .unwrap();
    let rev = revision();
    let plan = preview_name_recipe(&snapshot, &rev, intent()).unwrap();
    let mut other = snapshot.clone();
    other.project.info.name = "plain root writer changed context".into();
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
        apply_name_recipe(
            &mut snapshot,
            &mut stack,
            &rev,
            &plan,
            Some(NameRecipeNativeTarget {
                path: &path,
                history_generation: 0
            })
        ),
        Err(NameRecipeError::Persistence(_))
    ));
    assert_eq!(snapshot, before);
    assert_eq!(stack.history_lengths(), (0, 0));
    assert_eq!(std::fs::read(&path).unwrap(), bytes);
}

#[test]
fn missing_native_target_is_not_created_and_keeps_live_project_and_redo() {
    let mut snapshot = state();
    let before = snapshot.clone();
    let mut stack = CommandStack::new();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("absent.knxdb");
    let rev = revision();
    let plan = preview_name_recipe(&snapshot, &rev, intent()).unwrap();
    assert!(matches!(
        apply_name_recipe(
            &mut snapshot,
            &mut stack,
            &rev,
            &plan,
            Some(NameRecipeNativeTarget {
                path: &path,
                history_generation: 0
            })
        ),
        Err(NameRecipeError::Persistence(_))
    ));
    assert_eq!(snapshot, before);
    assert_eq!(stack.history_lengths(), (0, 0));
    assert!(!path.exists());
}

#[test]
fn changed_snapshot_or_revision_never_applies_a_reviewed_batch() {
    let mut snapshot = state();
    let before = snapshot.clone();
    let rev = revision();
    let plan = preview_name_recipe(&snapshot, &rev, intent()).unwrap();
    let mut stack = CommandStack::new();
    for changed in [
        NameRecipeRevision {
            generation: rev.generation + 1,
            ..rev.clone()
        },
        NameRecipeRevision {
            project_incarnation: "different-load".into(),
            ..rev.clone()
        },
    ] {
        assert_eq!(
            apply_name_recipe(&mut snapshot, &mut stack, &changed, &plan, None),
            Err(NameRecipeError::Conflict)
        );
        assert_eq!(snapshot, before);
        assert_eq!(stack.history_lengths(), (0, 0));
    }
    snapshot.project.info.name = "other context".into();
    let current = snapshot.clone();
    assert_eq!(
        apply_name_recipe(&mut snapshot, &mut stack, &rev, &plan, None),
        Err(NameRecipeError::Conflict)
    );
    assert_eq!(snapshot, current);
    assert_eq!(stack.history_lengths(), (0, 0));
}
