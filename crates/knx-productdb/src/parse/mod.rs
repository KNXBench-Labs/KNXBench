//! The parsers, one module per manufacturer file kind. Each takes the
//! file's bytes and its content hash, writes rows, and returns the
//! constructs it did not recognize. None of them fails the ingest because
//! of one unknown element or attribute.

pub mod catalog;
pub mod comobject;
pub mod hardware;
pub mod master;
pub mod program;
pub mod translation;

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
            let sample = a.get(name).unwrap_or_default();
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
/// that gets to exist. If it is, and the existing row came from a
/// different file (`source_sha256` differs), that is recorded as an
/// `IdConflict` — but the existing row is kept regardless, so the answer
/// is always "did the caller's row win", never "is this now the winner".
///
/// Extracted from two byte-identical copies (`hardware.rs` and
/// `catalog.rs`; `program.rs` inlines the same idea for
/// `application_program` alone, differently enough — it also gates several
/// later match arms on the result — that folding it in here was not
/// attempted). Behaviour is unchanged on purpose: it still compares
/// `source_sha256` at the whole-file granularity it always has, same-file
/// duplicate ids included (KNOWN_LIMITATIONS.md §86). Making that
/// finer-grained is a separate, deliberate piece of work, not a side effect
/// of tidying up the copies.
pub(crate) fn first_winner(
    conn: &Connection,
    table: &str,
    id: Option<&str>,
    source_sha256: &str,
    conflicts: &mut Vec<IdConflict>,
) -> Result<bool, ProductDbError> {
    let id = id.unwrap_or_default();
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
            if kept != source_sha256 {
                conflicts.push(IdConflict {
                    table: table.to_string(),
                    id: id.to_string(),
                    kept_sha256: kept,
                    other_sha256: source_sha256.to_string(),
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
        // `first_winner`; this is the one they both now call.
        let (_dir, conn) = db();
        let mut conflicts = Vec::new();
        conn.execute(
            "INSERT INTO hardware (id, manufacturer_id, source_sha256) VALUES ('H-1', 'M-1', 'sha-a')",
            [],
        )
        .unwrap();

        // A fresh id always wins, no conflict recorded.
        assert!(first_winner(&conn, "hardware", Some("H-2"), "sha-a", &mut conflicts).unwrap());
        assert!(conflicts.is_empty());

        // The same id from the same file is not a conflict, just a loss.
        assert!(!first_winner(&conn, "hardware", Some("H-1"), "sha-a", &mut conflicts).unwrap());
        assert!(conflicts.is_empty());

        // The same id from a different file's hash is a recorded conflict,
        // and the first writer still keeps the row.
        assert!(!first_winner(&conn, "hardware", Some("H-1"), "sha-b", &mut conflicts).unwrap());
        assert_eq!(conflicts.len(), 1);
        assert_eq!(conflicts[0].table, "hardware");
        assert_eq!(conflicts[0].id, "H-1");
        assert_eq!(conflicts[0].kept_sha256, "sha-a");
        assert_eq!(conflicts[0].other_sha256, "sha-b");
    }
}
