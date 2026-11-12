//! Schema21 authentic migration, independent section writes and union-owner lifecycle regressions.

use super::*;
use knx_core::commissioning::parameter_baseline::{
    BaselineChange, BaselineLink, LinkBaseline, LinkBaselineChange, WrittenObjectFlags,
};

fn links(id: u32, empty: bool) -> BaselineEdit {
    BaselineEdit {
        device: DeviceId(id),
        parameter: BaselineChange::Keep,
        links: LinkBaselineChange::Record(LinkBaseline {
            program_id: PROGRAM.into(),
            at: at(),
            links: if empty {
                vec![]
            } else {
                vec![BaselineLink {
                    object: 255,
                    group_address: knx_core::GroupAddress::from_raw(65535),
                    sending: true,
                }]
            },
        }),
    }
}
fn flag() -> WrittenObjectFlags {
    WrittenObjectFlags {
        read: true,
        write: false,
        transmit: true,
        update: false,
        communication: true,
        read_on_init: None,
    }
}
fn flags(id: u32, known: bool, empty: bool) -> BaselineEdit {
    let mut b = baseline(1);
    b.object_flags = known.then(|| {
        if empty {
            Default::default()
        } else {
            [(255, flag())].into()
        }
    });
    BaselineEdit {
        device: DeviceId(id),
        parameter: BaselineChange::Record(b),
        links: LinkBaselineChange::Keep,
    }
}

#[test]
fn authentic_native20_migration_preserves_predecessor_oracles_and_unknown_new_sections() {
    let source = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/native20-before-link-and-object-flags.sqlite");
    // Literal predecessor-writer oracles, never recalculated by the schema21 writer.
    let oracle: std::collections::BTreeMap<&str, &str> = [
        (
            "named-complete",
            "02100447b1a23460661e14e16c48192b8d10067ed1f4acb300d41144ac86a8f9",
        ),
        (
            "named-model",
            "8c76168439c7fbe1830770e92bb0c2cd52dd38ee5e2a6ddf8ffe3a1a7e987099",
        ),
        (
            "redo-0-complete",
            "c106b278b006925bcdb94f7badafcb35d7af39b96b866c65cb1ebcae6bc47e8a",
        ),
        (
            "redo-0-model",
            "02b0387bd266668bd9d71d946e6d2e73d92e75e981a2006d4204b111dcd64dca",
        ),
        (
            "root-complete",
            "02100447b1a23460661e14e16c48192b8d10067ed1f4acb300d41144ac86a8f9",
        ),
        (
            "root-model",
            "8c76168439c7fbe1830770e92bb0c2cd52dd38ee5e2a6ddf8ffe3a1a7e987099",
        ),
        (
            "stored-context",
            "b675662adb821bffe5f20e74b30dedc1c031d2aefc3fe8af22443c404dcd7ed5",
        ),
        (
            "stored-named-1",
            "8c76168439c7fbe1830770e92bb0c2cd52dd38ee5e2a6ddf8ffe3a1a7e987099",
        ),
        (
            "stored-redo-0",
            "02b0387bd266668bd9d71d946e6d2e73d92e75e981a2006d4204b111dcd64dca",
        ),
        (
            "stored-undo-0",
            "8c76168439c7fbe1830770e92bb0c2cd52dd38ee5e2a6ddf8ffe3a1a7e987099",
        ),
        (
            "stored-undo-1",
            "b6f9165fc26103e55edc3b41db5398a85383ffd96ca60159e75af35babcf7826",
        ),
        (
            "stored-working",
            "20302901d1decf353175a47742e2811f882124319c4e5af24b81debf3a97bfa0",
        ),
        (
            "undo-0-complete",
            "02100447b1a23460661e14e16c48192b8d10067ed1f4acb300d41144ac86a8f9",
        ),
        (
            "undo-0-model",
            "8c76168439c7fbe1830770e92bb0c2cd52dd38ee5e2a6ddf8ffe3a1a7e987099",
        ),
        (
            "undo-1-complete",
            "ce56fdb6001e9ee906407ee042299dbc403aa469ee8d323e418043d9253a5b78",
        ),
        (
            "undo-1-model",
            "b6f9165fc26103e55edc3b41db5398a85383ffd96ca60159e75af35babcf7826",
        ),
        (
            "working-complete",
            "c47d898016337adf6100a2a2eb3bf4f248efbe07301a4984625df8fb5d92912c",
        ),
        (
            "working-model",
            "20302901d1decf353175a47742e2811f882124319c4e5af24b81debf3a97bfa0",
        ),
    ]
    .into();
    let original = std::fs::read(&source).unwrap();
    assert_eq!(
        history::image_hash(&original),
        "14ca5d6e41b845d5898016b2a4883f8e0e25a0f40926bf5961b42add95098f2a"
    );
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("authentic.knxdb");
    std::fs::write(&path, &original).unwrap();
    let before = rows(
        &Connection::open_with_flags(&path, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap(),
        HISTORY,
    );
    let readonly = knx_store::open_existing_read_only(&path).unwrap();
    assert_eq!(readonly.migrated_from, Some(20));
    assert_eq!(user_version(&readonly.conn), 21);
    let all = history::load_parameter_baselines(&readonly.conn).unwrap();
    assert_eq!(all.device_ids(), [DeviceId(1), DeviceId(2)].into());
    let one = all.get(DeviceId(1)).unwrap();
    assert_eq!(one.program_id, "M-00FA_A-0001-01-0000");
    assert_eq!(
        one.at.to_rfc3339_opts(chrono::SecondsFormat::AutoSi, true),
        "2026-01-02T03:04:05.123456789Z"
    );
    assert_eq!(
        one.values,
        [
            ("M-00FA_A-0001-01-0000_P-1_R-1".into(), "01".into()),
            ("M-00FA_A-0001-01-0000_P-2_R-2".into(), "".into()),
            (
                "M-00FA_A-0001-01-0000_P-3_R-3".into(),
                "Text; \"quoted\" ß\nnext line".into()
            ),
        ]
        .into()
    );
    assert!(
        one.object_flags.is_none(),
        "migrated flags are UNKNOWN, not recorded empty"
    );
    assert!(
        all.get(DeviceId(2)).unwrap().values.is_empty(),
        "recorded empty parameter baseline survives"
    );
    assert!(all.get(DeviceId(2)).unwrap().object_flags.is_none());
    assert!(all.get(DeviceId(3)).is_none());
    assert!(all.iter_links().next().is_none());
    let editor = history::load_editor(&readonly.conn).unwrap().unwrap();
    assert_eq!(
        editor.baseline.semantic_hash().unwrap(),
        oracle["root-complete"]
    );
    assert_eq!(
        editor.working.semantic_hash().unwrap(),
        oracle["working-complete"]
    );
    assert_eq!(
        history::read_version(&readonly.conn, 1)
            .unwrap()
            .semantic_hash()
            .unwrap(),
        oracle["named-complete"]
    );
    assert_eq!(
        (editor.undo.len(), editor.redo.len(), editor.generation),
        (2, 1, 4)
    );
    for (side, states) in [("undo", &editor.undo), ("redo", &editor.redo)] {
        for (index, project) in states.iter().enumerate() {
            let snapshot = NativeSnapshot {
                project: project.clone(),
                opaque: editor.working.opaque.clone(),
                manufacturer_refs: editor.working.manufacturer_refs.clone(),
            };
            assert_eq!(
                snapshot.semantic_hash().unwrap(),
                oracle[format!("{side}-{index}-complete").as_str()]
            );
        }
    }
    // All six physical history payloads decode under recorded old-writer hashes.
    for (sql, kind) in [
        (
            "SELECT image, image_hash FROM project_history_context",
            "stored-context",
        ),
        (
            "SELECT working, working_hash FROM project_history_state",
            "stored-working",
        ),
        (
            "SELECT image, image_hash FROM project_history_version",
            "stored-named-1",
        ),
        (
            "SELECT image, image_hash FROM project_history_stack WHERE side='undo' AND position=0",
            "stored-undo-0",
        ),
        (
            "SELECT image, image_hash FROM project_history_stack WHERE side='undo' AND position=1",
            "stored-undo-1",
        ),
        (
            "SELECT image, image_hash FROM project_history_stack WHERE side='redo'",
            "stored-redo-0",
        ),
    ] {
        let (image, hash): (Vec<u8>, String) = readonly
            .conn
            .query_row(sql, [], |r| Ok((r.get(0)?, r.get(1)?)))
            .unwrap();
        assert_eq!(
            NativeSnapshot::decode(&image, &hash)
                .unwrap()
                .semantic_hash()
                .unwrap(),
            oracle[kind]
        );
    }
    assert_eq!(rows(&readonly.conn, HISTORY), before);
    drop(readonly);
    assert_eq!(
        std::fs::read(&path).unwrap(),
        original,
        "read-only migration changed file bytes"
    );
    let writable = knx_store::open_existing_and_migrate(&path).unwrap();
    assert_eq!(history::load_parameter_baselines(&writable).unwrap(), all);
    assert_eq!(rows(&writable, HISTORY), before);
    assert_eq!(NativeSnapshot::read(&writable).unwrap(), editor.baseline);
}

#[test]
fn known_empty_and_unknown_sections_reopen_and_unselected_timestamps_stay_exact() {
    let dir = tempfile::tempdir().unwrap();
    let base = project("P", 3);
    let (path, generation) = saved(dir.path(), "sections.knxdb", &base);
    let generation = settle(
        &path,
        &base,
        generation,
        &[
            flags(1, false, true),
            flags(2, true, true),
            flags(3, true, false),
            links(2, true),
            links(3, false),
        ],
    )
    .unwrap();
    let first = stored(&path);
    assert_eq!(first.get(DeviceId(1)).unwrap().object_flags, None);
    assert_eq!(
        first.get(DeviceId(2)).unwrap().object_flags,
        Some(Default::default())
    );
    assert_eq!(
        first.get(DeviceId(3)).unwrap().object_flags,
        Some([(255, flag())].into())
    );
    assert!(first.get_links(DeviceId(1)).is_none());
    assert!(first.get_links(DeviceId(2)).unwrap().links.is_empty());
    let mut edit = links(3, true);
    if let LinkBaselineChange::Record(b) = &mut edit.links {
        b.at += chrono::Duration::seconds(10);
    }
    let generation = settle(&path, &base, generation, &[edit]).unwrap();
    assert_eq!(stored(&path).get(DeviceId(3)), first.get(DeviceId(3)));
    let link_after = stored(&path).get_links(DeviceId(3)).cloned();
    settle(&path, &base, generation, &[flags(3, true, true)]).unwrap();
    assert_eq!(stored(&path).get_links(DeviceId(3)).cloned(), link_after);
}

#[test]
fn every_new_section_refuses_malformed_or_orphan_rows_on_full_and_single_load() {
    let mutations = [
        "UPDATE device_parameter_baseline SET object_flags_known=2",
        "UPDATE device_parameter_baseline SET object_flags_known=0",
        "UPDATE device_parameter_baseline SET at='yesterday'",
        "UPDATE device_object_flags_baseline SET device_id=999",
        "UPDATE device_object_flags_baseline SET object_number=256",
        "UPDATE device_object_flags_baseline SET read=2",
        "UPDATE device_object_flags_baseline SET write=-1",
        "UPDATE device_object_flags_baseline SET transmit=3",
        "UPDATE device_object_flags_baseline SET update_flag=2",
        "UPDATE device_object_flags_baseline SET communication=2",
        "UPDATE device_object_flags_baseline SET read_on_init=2",
        "UPDATE device_link_baseline SET at='bad'",
        "UPDATE device_link_baseline SET program_id=''",
        "UPDATE device_link_baseline_link SET device_id=999",
        "UPDATE device_link_baseline_link SET object_number=-1",
        "UPDATE device_link_baseline_link SET group_address=65536",
        "UPDATE device_link_baseline_link SET sending=2",
    ];
    for mutation in mutations {
        let conn = knx_store::open_and_migrate_in_memory().unwrap();
        history::write_parameter_baselines(&conn, &[flags(1, true, false), links(1, false)])
            .unwrap();
        conn.execute_batch("PRAGMA foreign_keys=OFF; PRAGMA ignore_check_constraints=ON;")
            .unwrap();
        conn.execute_batch(mutation).unwrap();
        assert!(
            knx_store::parameter_baseline::load_all(&conn).is_err(),
            "{mutation}"
        );
        assert!(
            knx_store::parameter_baseline::load_one(&conn, DeviceId(1)).is_err(),
            "{mutation}"
        );
    }
}

#[test]
fn failed_link_write_rolls_back_both_sections_and_journal_generation() {
    let dir = tempfile::tempdir().unwrap();
    let base = project("P", 1);
    let (path, generation) = saved(dir.path(), "rollback.knxdb", &base);
    let generation = settle(
        &path,
        &base,
        generation,
        &[flags(1, true, false), links(1, false)],
    )
    .unwrap();
    let before = stored(&path);
    let mut bad = links(1, true);
    if let LinkBaselineChange::Record(b) = &mut bad.links {
        b.program_id.clear();
    }
    assert!(settle(
        &path,
        &programmed(&base, 1),
        generation,
        &[flags(1, true, true), bad]
    )
    .is_err());
    assert_eq!(stored(&path), before);
    let reopened = knx_store::open_existing_read_only(&path).unwrap();
    let editor = history::load_editor(&reopened.conn).unwrap().unwrap();
    assert_eq!(editor.working.project, base);
    assert_eq!(editor.generation, generation);
}

#[test]
fn link_only_owner_is_pruned_on_replacement_and_after_deletion_without_fk_failure() {
    let dir = tempfile::tempdir().unwrap();
    let base = project("P", 1);
    let (path, generation) = saved(dir.path(), "union.knxdb", &base);
    let _generation = settle(&path, &base, generation, &[links(1, false)]).unwrap();
    assert!(stored(&path).iter().next().is_none(), "link-only premise");
    let changed = replaced_device(&base, 1, |d| d.source.ets_id = "Other".into());
    let conn = history::open_editor_store(&path, false).unwrap();
    knx_store::save_project(&conn, &changed).unwrap();
    assert!(
        history::load_parameter_baselines(&conn).unwrap().is_empty(),
        "union-owner identity pruning"
    );
    drop(conn);
    // A continued dirty new device need not exist in saved root; later removal is safe.
    let (path, _) = saved(dir.path(), "continued.knxdb", &project("P", 0));
    let conn = history::open_editor_store(&path, false).unwrap();
    let generation = history::load_editor(&conn).unwrap().unwrap().generation;
    drop(conn);
    let generation = settle(&path, &base, generation, &[links(1, false)]).unwrap();
    assert!(stored(&path).get_links(DeviceId(1)).is_some());
    let conn = history::open_editor_store(&path, false).unwrap();
    history::save_editor(
        &conn,
        &model(&project("P", 0)),
        &CommandStack::new(),
        Some(generation),
        true,
    )
    .unwrap();
    assert!(history::load_parameter_baselines(&conn).unwrap().is_empty());
    knx_store::save_project(&conn, &project("P", 0)).unwrap();
}

#[test]
fn restore_returns_exact_union_owner_set_and_both_sections_follow_identity() {
    let dir = tempfile::tempdir().unwrap();
    let older = project("P", 3);
    let (path, generation) = saved(dir.path(), "restore-union.knxdb", &older);
    let conn = history::open_editor_store(&path, false).unwrap();
    let generation = history::create_version(&conn, &model(&older), "Before", generation).unwrap();
    let version = history::list_versions(&conn).unwrap()[0].id;
    drop(conn);
    let live = replaced_device(&older, 2, |d| d.product_ref = "Other".into());
    let generation = settle(
        &path,
        &live,
        generation,
        &[
            links(1, false),
            links(2, false),
            flags(3, true, false),
            links(3, true),
        ],
    )
    .unwrap();
    let conn = history::open_editor_store(&path, false).unwrap();
    let changed = history::change_history(
        &conn,
        &model(&live),
        generation,
        history::HistoryAction::Restore(version),
    )
    .unwrap();
    let kept = changed.restored_baselines.unwrap();
    assert_eq!(kept.device_ids(), [DeviceId(1), DeviceId(3)].into());
    assert_eq!(kept, history::load_parameter_baselines(&conn).unwrap());
}

#[test]
fn plain_save_writes_no_rows_in_any_of_the_five_baseline_tables() {
    let dir = tempfile::tempdir().unwrap();
    let base = project("P", 1);
    let (path, generation) = saved(dir.path(), "no-rewrite.knxdb", &base);
    let generation = settle(
        &path,
        &base,
        generation,
        &[flags(1, true, false), links(1, false)],
    )
    .unwrap();
    let conn = history::open_editor_store(&path, false).unwrap();
    count_baseline_writes(&conn);
    let generation = history::save_editor_with_sources(
        &conn,
        &model(&programmed(&base, 1)),
        &CommandStack::new(),
        Some(generation),
        &[],
    )
    .unwrap();
    assert_eq!(
        baseline_writes(&conn),
        0,
        "plain save must not rewrite file-level evidence"
    );
    let all = history::load_parameter_baselines(&conn).unwrap();
    history::save_editor_with_baselines(
        &conn,
        &model(&programmed(&base, 1)),
        &CommandStack::new(),
        Some(generation),
        &[],
        &all,
    )
    .unwrap();
    assert_eq!(
        baseline_writes(&conn),
        10,
        "calibration observes each of five nonempty tables deleted and inserted"
    );
}
