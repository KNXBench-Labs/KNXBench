//! Proves incremental, trusted journal writes store exactly what a full stack rewrite stores.

use knx_core::{Command, CommandStack, GroupAddressStyle, Language, Project};
use knx_store::project_history::{self as history, JournalTrust, NativeSnapshot, StackRows};

fn style(step: usize) -> Command {
    let styles = [
        GroupAddressStyle::Free,
        GroupAddressStyle::TwoLevel,
        GroupAddressStyle::ThreeLevel,
    ];
    Command::SetGroupAddressStyle {
        style: styles[step % styles.len()],
    }
}

fn seed() -> NativeSnapshot {
    let mut project = Project::new(Language("en".into()));
    project.info.name = "Incremental history witness".into();
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
        manufacturer_refs: Vec::new(),
    }
}

/// One saved native file and the generation its journal is at.
struct File {
    _dir: tempfile::TempDir,
    conn: knx_store::Connection,
    generation: i64,
}

fn saved(seed: &NativeSnapshot) -> File {
    let dir = tempfile::tempdir().unwrap();
    let conn = knx_store::open_and_migrate(&dir.path().join("history.knxdb")).unwrap();
    let generation = history::save_editor(&conn, seed, &CommandStack::new(), None, true).unwrap();
    File {
        _dir: dir,
        conn,
        generation,
    }
}

/// The server's write path: trust what the last own write verified, rows only
/// below the prefix the core proves unchanged.
struct Incremental {
    file: File,
    earlier: Option<(Project, CommandStack, JournalTrust)>,
}

impl Incremental {
    fn write(&mut self, working: &NativeSnapshot, stack: &CommandStack) -> (usize, usize) {
        let trust = self.earlier.as_ref().map_or_else(
            JournalTrust::default,
            |(project, earlier, trust)| {
                let shared = stack.shared_snapshot_prefix(&working.project, earlier, project);
                trust.clone().with_rows(StackRows {
                    undo: trust.rows.undo[..shared.undo].to_vec(),
                    redo: trust.rows.redo[..shared.redo].to_vec(),
                })
            },
        );
        let (generation, next) = history::save_editor_trusting(
            &self.file.conn,
            working,
            stack,
            self.file.generation,
            &trust,
            &mut history::JournalWriteReport::default(),
        )
        .unwrap();
        assert_eq!(
            next.rows,
            stored_rows(&self.file.conn),
            "returned rows are the stored rows"
        );
        self.file.generation = generation;
        self.earlier = Some((working.project.clone(), stack.clone(), next));
        (trust.rows.undo.len(), trust.rows.redo.len())
    }
}

fn stored_rows(conn: &knx_store::Connection) -> StackRows {
    let mut rows = StackRows::default();
    let mut stmt = conn
        .prepare("SELECT side, image_hash FROM project_history_stack ORDER BY side, position")
        .unwrap();
    let mut query = stmt.query([]).unwrap();
    while let Some(row) = query.next().unwrap() {
        let side: String = row.get(0).unwrap();
        let hash: String = row.get(1).unwrap();
        match side.as_str() {
            "undo" => rows.undo.push(hash),
            "redo" => rows.redo.push(hash),
            other => panic!("unknown side {other}"),
        }
    }
    rows
}

fn full_write(file: &mut File, working: &NativeSnapshot, stack: &CommandStack) {
    file.generation =
        history::save_editor(&file.conn, working, stack, Some(file.generation), false).unwrap();
}

/// Both files must reopen to the same journal, row for row.
fn assert_same_journal(incremental: &Incremental, full: &File) {
    let a = history::load_editor(&incremental.file.conn)
        .unwrap()
        .unwrap();
    let b = history::load_editor(&full.conn).unwrap().unwrap();
    assert_eq!(a.working, b.working);
    assert_eq!(a.baseline, b.baseline);
    assert_eq!(a.undo, b.undo);
    assert_eq!(a.redo, b.redo);
    assert_eq!(stored_rows(&incremental.file.conn), stored_rows(&full.conn));
}

#[test]
fn incremental_journal_equals_a_full_rewrite_through_edits_undo_redo_and_outside_changes() {
    let seed = seed();
    let mut incremental = Incremental {
        file: saved(&seed),
        earlier: None,
    };
    let mut full = saved(&seed);
    let mut working = seed.clone();
    let mut stack = CommandStack::new();
    let mut kept = Vec::new();
    let mut step = |working: &NativeSnapshot, stack: &CommandStack, kept: &mut Vec<_>| {
        full_write(&mut full, working, stack);
        kept.push(incremental.write(working, stack));
        assert_same_journal(&incremental, &full);
    };

    for edit in 0..4 {
        stack.do_command(&mut working.project, style(edit)).unwrap();
        step(&working, &stack, &mut kept);
    }
    stack.undo(&mut working.project).unwrap();
    step(&working, &stack, &mut kept);
    stack.undo(&mut working.project).unwrap();
    step(&working, &stack, &mut kept);
    stack.redo(&mut working.project).unwrap();
    step(&working, &stack, &mut kept);
    stack.do_command(&mut working.project, style(5)).unwrap();
    step(&working, &stack, &mut kept);
    // A change outside the stack reaches every replayed state: nothing is kept.
    working.project.info.name = "Renamed outside the history".into();
    stack.do_command(&mut working.project, style(6)).unwrap();
    step(&working, &stack, &mut kept);
    // New retained context: kept rows follow it, as a full rewrite pairs them.
    working.opaque[0].bytes = b"replaced opaque witness".to_vec();
    stack.do_command(&mut working.project, style(7)).unwrap();
    step(&working, &stack, &mut kept);

    assert_eq!(
        kept,
        vec![
            (0, 0), // first write after the save: nothing of ours is stored yet
            (1, 0),
            (2, 0),
            (3, 0),
            (3, 0), // undo: the three entries below the undone one
            (2, 1), // second undo: the first redo state stays too
            (2, 1), // redo: unchanged bottom on both sides
            (3, 0), // a new edit after undo drops redo
            (0, 0), // outside change
            (5, 0), // context change keeps every model image
        ]
    );
}

#[test]
fn rows_another_writer_replaced_are_written_again() {
    let seed = seed();
    let mut incremental = Incremental {
        file: saved(&seed),
        earlier: None,
    };
    let mut full = saved(&seed);
    let mut working = seed.clone();
    let mut stack = CommandStack::new();
    for edit in 0..3 {
        stack.do_command(&mut working.project, style(edit)).unwrap();
        full_write(&mut full, &working, &stack);
        incremental.write(&working, &stack);
    }
    // Another writer stores a different stack of the same length.
    let mut other = seed.clone();
    let mut other_stack = CommandStack::new();
    for edit in [2, 0, 1] {
        other_stack
            .do_command(&mut other.project, style(edit))
            .unwrap();
    }
    full_write(&mut incremental.file, &other, &other_stack);
    stack.do_command(&mut working.project, style(3)).unwrap();
    full_write(&mut full, &working, &stack);
    let (undo, _) = incremental.write(&working, &stack);
    assert_eq!(
        undo, 3,
        "the stale claim is offered and must be refused by hash"
    );
    assert_same_journal(&incremental, &full);
}

#[test]
fn a_claim_beyond_the_stored_rows_keeps_only_what_matches() {
    let seed = seed();
    let mut file = saved(&seed);
    let mut working = seed.clone();
    let mut stack = CommandStack::new();
    stack.do_command(&mut working.project, style(0)).unwrap();
    full_write(&mut file, &working, &stack);
    let stored = stored_rows(&file.conn);
    stack.do_command(&mut working.project, style(1)).unwrap();
    let claim = StackRows {
        undo: vec![stored.undo[0].clone(), "f".repeat(64), "e".repeat(64)],
        redo: vec!["d".repeat(64)],
    };
    let (_, rows) =
        history::save_editor_reusing(&file.conn, &working, &stack, file.generation, &claim)
            .unwrap();
    assert_eq!(rows.undo.len(), 2);
    assert_eq!(rows.undo[0], stored.undo[0]);
    assert!(rows.redo.is_empty());
    let mut reference = saved(&seed);
    full_write(&mut reference, &working, &stack);
    assert_eq!(rows, stored_rows(&reference.conn));
    let loaded = history::load_editor(&file.conn).unwrap().unwrap();
    assert_eq!(
        loaded.undo,
        history::load_editor(&reference.conn).unwrap().unwrap().undo
    );
}

#[test]
fn a_trusted_row_whose_bytes_changed_is_refused() {
    let seed = seed();
    let mut file = saved(&seed);
    let mut working = seed.clone();
    let mut stack = CommandStack::new();
    stack.do_command(&mut working.project, style(0)).unwrap();
    full_write(&mut file, &working, &stack);
    let stored = stored_rows(&file.conn);
    // Damage the stored image but keep its recorded hash: trust is by bytes, not by the column.
    file.conn
        .execute(
            "UPDATE project_history_stack SET image = CAST(image || x'00' AS BLOB) WHERE side = 'undo' AND position = 0",
            [],
        )
        .unwrap();
    stack.do_command(&mut working.project, style(1)).unwrap();
    let refused =
        history::save_editor_reusing(&file.conn, &working, &stack, file.generation, &stored);
    assert!(refused.is_err(), "a damaged trusted row must not be kept");
}
