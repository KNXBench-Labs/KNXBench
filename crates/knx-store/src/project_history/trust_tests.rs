//! Proves trusted journal writes skip known work and still refuse what changed.

use super::snapshot::BUILT_STORES;
use super::*;
use knx_core::{Command, GroupAddressStyle, Language};

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
    project.info.name = "Trusted history witness".into();
    NativeSnapshot {
        project,
        opaque: vec![crate::StoredOpaqueEntry {
            source_path: "product.xml".into(),
            xpath: "/product".into(),
            kind: "element".into(),
            name: "EmbeddedProductData".into(),
            bytes: vec![0x5a; 64 * 1024],
            sha256: "retained-source-hash".into(),
        }],
        manufacturer_refs: Vec::new(),
    }
}

struct File {
    _dir: tempfile::TempDir,
    conn: Connection,
    generation: i64,
}

fn saved(seed: &NativeSnapshot) -> File {
    let dir = tempfile::tempdir().unwrap();
    let conn = crate::open_and_migrate(&dir.path().join("trust.knxdb")).unwrap();
    let generation = save_editor(&conn, seed, &CommandStack::new(), None, true).unwrap();
    File {
        _dir: dir,
        conn,
        generation,
    }
}

/// Runs `write` and counts the native stores it built: (with context, model only).
fn built<T>(write: impl FnOnce() -> T) -> (T, (usize, usize)) {
    BUILT_STORES.with(|built| built.set((0, 0)));
    let result = write();
    (result, BUILT_STORES.with(std::cell::Cell::get))
}

fn trusted(
    file: &mut File,
    working: &NativeSnapshot,
    stack: &CommandStack,
    trust: &JournalTrust,
) -> (JournalTrust, (usize, usize)) {
    let ((generation, next), built) =
        built(|| save_editor_trusting(&file.conn, working, stack, file.generation, trust).unwrap());
    file.generation = generation;
    (next, built)
}

/// Writes the same state fully into a fresh file and requires equal journals.
fn assert_matches_full_rewrite(
    file: &File,
    seed: &NativeSnapshot,
    working: &NativeSnapshot,
    stack: &CommandStack,
) {
    let reference = saved(seed);
    save_editor(
        &reference.conn,
        working,
        stack,
        Some(reference.generation),
        false,
    )
    .unwrap();
    let a = load_editor(&file.conn).unwrap().unwrap();
    let b = load_editor(&reference.conn).unwrap().unwrap();
    assert_eq!(a.working, b.working);
    assert_eq!(a.baseline, b.baseline);
    assert_eq!(a.undo, b.undo);
    assert_eq!(a.redo, b.redo);
    assert_eq!(a.rows, b.rows);
}

#[test]
fn a_trusted_write_derives_no_context_and_encodes_only_the_new_working_state() {
    let seed = seed();
    let mut file = saved(&seed);
    let mut working = seed.clone();
    let mut stack = CommandStack::new();
    stack.do_command(&mut working.project, style(0)).unwrap();
    let (trust, cold) = trusted(&mut file, &working, &stack, &JournalTrust::default());
    assert!(
        cold.0 >= 3,
        "an untrusted write derives the context: {cold:?}"
    );
    assert_eq!(trust.rows, load_editor(&file.conn).unwrap().unwrap().rows);

    stack.do_command(&mut working.project, style(1)).unwrap();
    let (trust, warm) = trusted(&mut file, &working, &stack, &trust);
    // The kept bottom row, the known context and baseline, and the previous
    // working bytes as the new top undo row: only the new working image is new.
    assert_eq!(warm, (0, 1));
    assert_matches_full_rewrite(&file, &seed, &working, &stack);

    stack.undo(&mut working.project).unwrap();
    let (_, undone) = trusted(&mut file, &working, &stack, &trust);
    assert_eq!(undone.0, 0, "undo derives no context either");
    assert_matches_full_rewrite(&file, &seed, &working, &stack);
}

#[test]
fn what_an_open_verified_serves_the_first_write() {
    let seed = seed();
    let mut file = saved(&seed);
    let mut working = seed.clone();
    let mut stack = CommandStack::new();
    stack.do_command(&mut working.project, style(0)).unwrap();
    file.generation =
        save_editor(&file.conn, &working, &stack, Some(file.generation), false).unwrap();
    let opened = load_editor(&file.conn).unwrap().unwrap().trust();
    stack.do_command(&mut working.project, style(1)).unwrap();
    let (_, first) = trusted(&mut file, &working, &stack, &opened);
    assert_eq!(first, (0, 1));
    assert_matches_full_rewrite(&file, &seed, &working, &stack);
}

#[test]
fn a_baseline_changed_by_another_writer_is_hashed_again_and_refused() {
    let seed = seed();
    let mut file = saved(&seed);
    let mut working = seed.clone();
    let mut stack = CommandStack::new();
    stack.do_command(&mut working.project, style(0)).unwrap();
    let (trust, _) = trusted(&mut file, &working, &stack, &JournalTrust::default());
    file.conn
        .execute("UPDATE project_info SET name = 'Saved elsewhere'", [])
        .unwrap();
    stack.do_command(&mut working.project, style(1)).unwrap();
    let refused = save_editor_trusting(&file.conn, &working, &stack, file.generation, &trust);
    assert!(matches!(refused, Err(HistoryError::Stale)), "{refused:?}");
}

#[test]
fn a_new_retained_context_is_encoded_again() {
    let seed = seed();
    let mut file = saved(&seed);
    let mut working = seed.clone();
    let mut stack = CommandStack::new();
    stack.do_command(&mut working.project, style(0)).unwrap();
    let (trust, _) = trusted(&mut file, &working, &stack, &JournalTrust::default());
    working.opaque[0].bytes[0] = 0xa5;
    stack.do_command(&mut working.project, style(1)).unwrap();
    let (trust, changed) = trusted(&mut file, &working, &stack, &trust);
    assert!(changed.0 > 0, "a changed context is encoded: {changed:?}");
    assert_matches_full_rewrite(&file, &seed, &working, &stack);
    // The new context is known from here on, and so is the saved baseline,
    // which still carries the old one.
    stack.do_command(&mut working.project, style(2)).unwrap();
    let (_, next) = trusted(&mut file, &working, &stack, &trust);
    assert_eq!(next.0, 0);
    assert_matches_full_rewrite(&file, &seed, &working, &stack);
}

#[test]
fn a_known_context_whose_bytes_changed_is_refused() {
    let seed = seed();
    let mut file = saved(&seed);
    let mut working = seed.clone();
    let mut stack = CommandStack::new();
    stack.do_command(&mut working.project, style(0)).unwrap();
    let (trust, _) = trusted(&mut file, &working, &stack, &JournalTrust::default());
    // Keep the recorded hash: trust is by bytes, not by the column.
    file.conn
        .execute(
            "UPDATE project_history_context SET image = CAST(image || x'00' AS BLOB)",
            [],
        )
        .unwrap();
    stack.do_command(&mut working.project, style(1)).unwrap();
    let refused = save_editor_trusting(&file.conn, &working, &stack, file.generation, &trust);
    assert!(
        refused.is_err(),
        "damaged context bytes must not be trusted"
    );
}

#[test]
fn a_working_state_another_writer_stored_is_not_copied() {
    let seed = seed();
    let mut file = saved(&seed);
    let mut working = seed.clone();
    let mut stack = CommandStack::new();
    stack.do_command(&mut working.project, style(0)).unwrap();
    let (trust, _) = trusted(&mut file, &working, &stack, &JournalTrust::default());
    // Another writer stores a different working state over the same stack.
    let mut other = working.clone();
    other.project.info.name = "Edited elsewhere".into();
    file.generation =
        save_editor(&file.conn, &other, &stack, Some(file.generation), false).unwrap();
    stack.do_command(&mut working.project, style(1)).unwrap();
    let (_, written) = trusted(&mut file, &working, &stack, &trust);
    // The rename outside the stack replays into the bottom row as well, so
    // both rows and the working state are encoded; the stored working bytes
    // are another state and must not become the top row.
    assert_eq!(
        written.1, 3,
        "the new top row is encoded, not copied: {written:?}"
    );
    assert_matches_full_rewrite(&file, &seed, &working, &stack);
}

fn name_version(file: &mut File, working: &NativeSnapshot, label: &str) {
    file.generation = change_history(
        &file.conn,
        working,
        file.generation,
        HistoryAction::Create(label),
    )
    .unwrap()
    .generation;
}

#[test]
fn saved_versions_decode_their_contexts_once_per_writer() {
    let seed = seed();
    let mut file = saved(&seed);
    let mut working = seed.clone();
    let mut stack = CommandStack::new();
    stack.do_command(&mut working.project, style(0)).unwrap();
    let (trust, _) = trusted(&mut file, &working, &stack, &JournalTrust::default());
    // Two versions on the working context and one on an older one.
    name_version(&mut file, &working, "First");
    name_version(&mut file, &working, "Second");
    let mut older = working.clone();
    older.opaque[0].bytes[0] = 0x11;
    name_version(&mut file, &older, "Older context");

    stack.do_command(&mut working.project, style(1)).unwrap();
    let (trust, first) = trusted(&mut file, &working, &stack, &trust);
    assert_eq!(first, (1, 1), "only the older version context is decoded");
    stack.do_command(&mut working.project, style(2)).unwrap();
    let (_, next) = trusted(&mut file, &working, &stack, &trust);
    assert_eq!(next, (0, 1), "every version context is known now");
    assert_versions_list_untrusted(&file);
}

/// An untrusted reader still admits and lists every version.
fn assert_versions_list_untrusted(file: &File) {
    let versions = list_versions(&file.conn).unwrap();
    assert_eq!(versions.len(), 3);
}

#[test]
fn a_known_version_context_whose_bytes_changed_is_refused() {
    let seed = seed();
    let mut file = saved(&seed);
    let mut working = seed.clone();
    let mut stack = CommandStack::new();
    stack.do_command(&mut working.project, style(0)).unwrap();
    let (trust, _) = trusted(&mut file, &working, &stack, &JournalTrust::default());
    let mut older = working.clone();
    older.opaque[0].bytes[0] = 0x11;
    name_version(&mut file, &older, "Older context");
    stack.do_command(&mut working.project, style(1)).unwrap();
    let (trust, _) = trusted(&mut file, &working, &stack, &trust);
    let older_context = older_context_hash(&file.conn, &working);
    file.conn
        .execute(
            "UPDATE project_history_context SET image = CAST(image || x'00' AS BLOB) WHERE content_hash = ?1",
            [&older_context],
        )
        .unwrap();
    stack.do_command(&mut working.project, style(2)).unwrap();
    let refused = save_editor_trusting(&file.conn, &working, &stack, file.generation, &trust);
    assert!(
        refused.is_err(),
        "damaged version context bytes must not be trusted"
    );
}

/// The retained context of the saved versions that is not the working one.
fn older_context_hash(conn: &Connection, working: &NativeSnapshot) -> String {
    let working_hash = load_editor(conn).unwrap().unwrap().trust();
    let known = working_hash.context().unwrap().content_hash.clone();
    assert!(working.opaque[0].bytes[0] != 0x11);
    conn.query_row(
        "SELECT context_hash FROM project_history_version WHERE context_hash != ?1",
        [&known],
        |r| r.get(0),
    )
    .unwrap()
}
