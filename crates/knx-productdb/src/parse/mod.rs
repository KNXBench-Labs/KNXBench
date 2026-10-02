//! The parsers, one module per manufacturer file kind. Each takes the
//! file's bytes and its content hash, writes rows, and returns the
//! constructs it did not recognize. None of them fails the ingest because
//! of one unknown element or attribute.

pub(crate) mod baggage;
pub mod catalog;
pub mod comobject;
pub mod hardware;
pub mod master;
pub(crate) mod master_language;
pub mod program;
pub(crate) mod scheme_evidence;
pub mod translation;

use std::collections::HashMap;

use rusqlite::{Connection, OptionalExtension};

use crate::report::{IdConflict, UnknownCollector};
use crate::xml::Attrs;
use crate::ProductDbError;

/// Reports every attribute on `a` that is not in `known`, so an unmodelled
/// manufacturer attribute is visible rather than lost.
pub(crate) fn report_unknown_attrs(
    collector: &mut UnknownCollector,
    xpath: &str,
    a: &Attrs,
    known: &[&str],
) {
    for name in a.names() {
        if !known.contains(&name) {
            let sample = a.evidence_value(name).unwrap_or_default();
            collector.attribute(xpath, name, sample);
        }
    }
}

/// `"1"`/`"0"` and `xs:boolean`'s other canonical spelling, `"true"`/`"false"`,
/// as SQLite integers; anything else stays `None` rather than being guessed
/// into a boolean. `"false"` earned its place here because schema 20/21
/// write `Linkable="false"`, and that is not unknown text, it is the
/// datatype's own lexical form — so `"True"`, `"yes"` and `"-1"` still stay
/// `None`, uppercase and synonyms included.
///
/// An attribute that is simply absent is not reported — there is nothing to
/// discard. One present under a spelling `xs:boolean` does not recognize is
/// reported through `collector` at `xpath` instead of vanishing into `None`
/// unremarked, so the next corpus's surprise ends up in an ingest report
/// rather than nowhere.
pub(crate) fn bool_flag(
    collector: &mut UnknownCollector,
    xpath: &str,
    a: &Attrs,
    name: &str,
) -> Option<i64> {
    match a.get(name) {
        Some("1") | Some("true") => Some(1),
        Some("0") | Some("false") => Some(0),
        Some(other) => {
            collector.attribute(xpath, name, other);
            None
        }
        None => None,
    }
}

/// First writer wins: if `id` is not already in `table`, this is the row
/// that gets to exist. If it is, and the existing row lost regardless —
/// either because it came from a different file (`source_sha256` differs)
/// or because it is not the first time *this parse call* has handed
/// `first_winner` this exact `(table, id)` pair — that is recorded as an
/// `IdConflict`. The existing row is kept regardless, so the answer is
/// always "did the caller's row win", never "is this now the winner".
///
/// Extracted from byte-identical hardware and catalog copies. The program
/// parser also uses it for `application_program`, while keeping its later
/// child-row gating local to that parser.
///
/// Comparing `source_sha256` alone (as this did until KNOWN_LIMITATIONS.md
/// §86 was closed) cannot see a same-file collision: one parse call passes
/// the *same* `source_sha256` for every element in the file it is reading,
/// so two elements sharing an `@Id` in that one file always compared
/// equal and the second was dropped without a trace. `seen_this_call`
/// fixes that by counting, so the comparison is now "has this exact
/// occurrence count been seen for this id, in this call" in addition to
/// the original file-hash check — a finer question than "which file",
/// answerable without touching the `source_sha256` column any row is
/// actually stored under (still the whole file's hash, still what
/// `program_should_be_skipped` and friends rely on elsewhere; only the
/// *comparison* got finer, not what gets persisted).
///
/// `seen_this_call` must be fresh (empty) at the start of one parse call
/// over one file and threaded through every `first_winner` call made
/// during it — a stale or shared map would count occurrences across files,
/// which is exactly the distinction this exists to preserve.
pub(crate) fn first_winner(
    conn: &Connection,
    table: &str,
    id: Option<&str>,
    source_sha256: &str,
    seen_this_call: &mut HashMap<(String, String), u32>,
    conflicts: &mut Vec<IdConflict>,
) -> Result<bool, ProductDbError> {
    let id = id.unwrap_or_default();
    let occurrence = {
        let count = seen_this_call
            .entry((table.to_string(), id.to_string()))
            .or_insert(0);
        *count += 1;
        *count
    };
    let existing: Option<String> = conn
        .query_row(
            &format!("SELECT source_sha256 FROM {table} WHERE id = ?1"),
            [id],
            |row| row.get(0),
        )
        .optional()?;
    match existing {
        None => Ok(true),
        Some(kept) => {
            if kept != source_sha256 || occurrence > 1 {
                conflicts.push(IdConflict {
                    table: table.to_string(),
                    id: id.to_string(),
                    kept_sha256: kept,
                    other_sha256: source_sha256.to_string(),
                    occurrence,
                });
            }
            Ok(false)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use quick_xml::events::Event;
    use quick_xml::Reader;

    fn attrs_from(xml: &str) -> Attrs {
        let mut r = Reader::from_reader(xml.as_bytes());
        let mut buf = Vec::new();
        let start = match r.read_event_into(&mut buf).unwrap() {
            Event::Empty(e) => e,
            other => panic!("expected an empty element, got {other:?}"),
        };
        crate::xml::attrs(&start, "t.xml").unwrap()
    }

    #[test]
    fn all_four_xs_boolean_spellings_are_recognised() {
        let mut unknown = UnknownCollector::default();
        assert_eq!(
            bool_flag(&mut unknown, "/E", &attrs_from(r#"<E F="1"/>"#), "F"),
            Some(1)
        );
        assert_eq!(
            bool_flag(&mut unknown, "/E", &attrs_from(r#"<E F="0"/>"#), "F"),
            Some(0)
        );
        assert_eq!(
            bool_flag(&mut unknown, "/E", &attrs_from(r#"<E F="true"/>"#), "F"),
            Some(1)
        );
        assert_eq!(
            bool_flag(&mut unknown, "/E", &attrs_from(r#"<E F="false"/>"#), "F"),
            Some(0)
        );
        assert!(unknown.into_vec().is_empty());
    }

    #[test]
    fn capitalised_or_absent_spellings_stay_none() {
        let mut unknown = UnknownCollector::default();
        assert_eq!(
            bool_flag(&mut unknown, "/E", &attrs_from(r#"<E F="True"/>"#), "F"),
            None
        );
        assert_eq!(
            bool_flag(&mut unknown, "/E", &attrs_from(r#"<E F=""/>"#), "F"),
            None
        );
        assert_eq!(
            bool_flag(&mut unknown, "/E", &attrs_from(r#"<E/>"#), "F"),
            None
        );
    }

    #[test]
    fn an_unrecognised_spelling_is_recorded_through_the_collector() {
        let mut unknown = UnknownCollector::default();
        let value = bool_flag(&mut unknown, "/E", &attrs_from(r#"<E F="True"/>"#), "F");
        assert_eq!(value, None);
        let recorded = unknown.into_vec();
        assert_eq!(recorded.len(), 1);
        assert_eq!(recorded[0].xpath, "/E");
        assert_eq!(recorded[0].name, "F");
        assert_eq!(recorded[0].sample.as_deref(), Some("True"));
    }

    #[test]
    fn an_absent_attribute_is_not_reported_as_unrecognised() {
        let mut unknown = UnknownCollector::default();
        let value = bool_flag(&mut unknown, "/E", &attrs_from(r#"<E/>"#), "F");
        assert_eq!(value, None);
        assert!(unknown.into_vec().is_empty());
    }

    fn db() -> (tempfile::TempDir, rusqlite::Connection) {
        let dir = tempfile::tempdir().unwrap();
        let conn = crate::open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
        (dir, conn)
    }

    #[test]
    fn first_winner_is_shared_by_every_caller_not_copy_pasted() {
        // hardware.rs and catalog.rs each carried a byte-identical private
        // `first_winner`; this is the one they both now call. Each
        // simulated "file" below gets its own fresh `seen` map, exactly as
        // a real `ingest_hardware`/`ingest_catalog` call would — reusing
        // one map across what are meant to be different files is the
        // misuse `first_winner`'s own doc comment warns about, and would
        // fabricate a same-file conflict between "H-1"/sha-a and
        // "H-1"/sha-b that never shared a file at all.
        let (_dir, conn) = db();
        let mut conflicts = Vec::new();
        conn.execute(
            "INSERT INTO hardware (id, manufacturer_id, source_sha256) VALUES ('H-1', 'M-1', 'sha-a')",
            [],
        )
        .unwrap();

        // A fresh id always wins, no conflict recorded. Modelled as part of
        // the same file ("sha-a") that "H-1" above came from.
        let mut seen_a = HashMap::new();
        assert!(first_winner(
            &conn,
            "hardware",
            Some("H-2"),
            "sha-a",
            &mut seen_a,
            &mut conflicts
        )
        .unwrap());
        assert!(conflicts.is_empty());

        // The same id from the same file (same map, same hash) is not a
        // conflict, just a loss — the idempotent re-ingest case.
        assert!(!first_winner(
            &conn,
            "hardware",
            Some("H-1"),
            "sha-a",
            &mut seen_a,
            &mut conflicts
        )
        .unwrap());
        assert!(conflicts.is_empty());

        // The same id from a different file's hash — its own fresh map, a
        // genuinely different parse call — is a recorded conflict, and the
        // first writer still keeps the row. `occurrence` is `1`: this file
        // only saw the id once, the conflict is against a different file.
        let mut seen_b = HashMap::new();
        assert!(!first_winner(
            &conn,
            "hardware",
            Some("H-1"),
            "sha-b",
            &mut seen_b,
            &mut conflicts
        )
        .unwrap());
        assert_eq!(conflicts.len(), 1);
        assert_eq!(conflicts[0].table, "hardware");
        assert_eq!(conflicts[0].id, "H-1");
        assert_eq!(conflicts[0].kept_sha256, "sha-a");
        assert_eq!(conflicts[0].other_sha256, "sha-b");
        assert_eq!(conflicts[0].occurrence, 1);
    }

    /// KNOWN_LIMITATIONS.md §86, now closed for every caller that uses this
    /// helper: two elements sharing an `@Id` *inside one file* — same
    /// `seen` map, same `source_sha256`, because a real caller only ever
    /// creates one of each per file — now produce a recorded conflict
    /// instead of a silent drop. Before this change, `first_winner` had no
    /// `seen_this_call` parameter at all, its only test was `kept !=
    /// source_sha256`, and that test is always false here (both calls carry
    /// the identical `"one-file-sha"`), so this exact scenario would have
    /// left `conflicts` empty — the fact this test asserts `conflicts.len()
    /// == 1` is what would have failed against that old signature/logic.
    #[test]
    fn two_calls_sharing_a_seen_map_and_hash_record_a_same_file_conflict() {
        let (_dir, conn) = db();
        let mut seen = HashMap::new();
        let mut conflicts = Vec::new();

        assert!(first_winner(
            &conn,
            "hardware",
            Some("H-DUP"),
            "one-file-sha",
            &mut seen,
            &mut conflicts
        )
        .unwrap());
        conn.execute(
            "INSERT INTO hardware (id, manufacturer_id, source_sha256) VALUES ('H-DUP', 'M-1', 'one-file-sha')",
            [],
        )
        .unwrap();
        assert!(
            conflicts.is_empty(),
            "the first occurrence is never a conflict"
        );

        assert!(!first_winner(
            &conn,
            "hardware",
            Some("H-DUP"),
            "one-file-sha",
            &mut seen,
            &mut conflicts
        )
        .unwrap());
        assert_eq!(
            conflicts.len(),
            1,
            "a second occurrence of the same id, in the same file, is now a recorded conflict"
        );
        assert_eq!(conflicts[0].table, "hardware");
        assert_eq!(conflicts[0].id, "H-DUP");
        assert_eq!(conflicts[0].kept_sha256, "one-file-sha");
        assert_eq!(conflicts[0].other_sha256, "one-file-sha");
        assert_eq!(
            conflicts[0].occurrence, 2,
            "this is the second time this call has seen H-DUP in hardware"
        );
    }
}
