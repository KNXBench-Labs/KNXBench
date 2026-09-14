//! What an ingest has to say for itself: unknown constructs, id conflicts,
//! and the counts a caller prints.
//!
//! Same rule as `knx-etsproj`'s `ImportReport`: an unrecognized element or
//! attribute is counted and reported, never silently dropped and never
//! fatal on its own (CLAUDE.md's data-integrity rule).

use std::collections::BTreeMap;

use rusqlite::{params, Connection};

use crate::ProductDbError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnknownKind {
    Element,
    Attribute,
}

impl UnknownKind {
    fn as_str(self) -> &'static str {
        match self {
            UnknownKind::Element => "Element",
            UnknownKind::Attribute => "Attribute",
        }
    }
}

/// One construct an ingest met and does not model, counted by how often it
/// was met.
///
/// `xpath` is always **the path of the container of `name`**, never the path
/// of `name` itself — one rule, but it lands on a different node for each
/// `kind`, so spelling both out here saves the first renderer of these rows
/// from getting it wrong once:
///
/// * `Element` — `xpath` is the *parent* element's path, `name` is the
///   unmodelled element. `/KNX/…/ApplicationProgram/Static` + `AddressTable`
///   means `…/Static/AddressTable` was found.
/// * `Attribute` — `xpath` is the *owning* element's path, `name` is the
///   unmodelled attribute, and `sample` is one value it was seen with.
///   `/KNX/…/Static/AbsoluteSegment` + `Size` means
///   `…/Static/AbsoluteSegment/@Size` was found.
///
/// So an unmodelled element with unmodelled attributes produces rows at two
/// different `xpath`s, one segment apart, and joining them means appending
/// the `Element` row's `name` to its `xpath`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownConstruct {
    pub xpath: String,
    pub kind: UnknownKind,
    pub name: String,
    pub occurrences: u32,
    pub sample: Option<String>,
}

/// An id that lost against an already-present row of the same `table`. The
/// first ingest's row is kept; this records that a second one existed.
///
/// `occurrence` is `first_winner`'s own per-parse-call count of how many
/// times this exact `(table, id)` pair has been handed to it so far,
/// including this one, so it is always `>= 1`. `1` means what it always
/// meant before KNOWN_LIMITATIONS.md §86 was closed: `kept_sha256 !=
/// other_sha256`, a collision between this file and a different,
/// previously-ingested one. Anything greater means the file currently
/// being parsed declared this id more than once *by itself* — `kept_sha256
/// == other_sha256` in that case, both being this file's own hash, and
/// `occurrence` is the only field that still says two different elements
/// were competing rather than one being re-read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdConflict {
    pub table: String,
    pub id: String,
    pub kept_sha256: String,
    pub other_sha256: String,
    pub occurrence: u32,
}

/// How many `translation` rows an ingest pass actually wrote, by scope.
/// Counted the same way `UnknownConstruct::occurrences` is: from what an
/// `INSERT OR IGNORE` actually changed, never from how many `Translation`
/// elements the parser merely walked past — a row already present under the
/// same `(scope, scope_id, language, ref_id, attribute_name)` key is ignored
/// by SQLite and contributes nothing here, even though it was seen.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TranslationCounts {
    pub program: usize,
    pub catalog: usize,
    pub hardware: usize,
    pub master: usize,
}

impl TranslationCounts {
    pub fn total(&self) -> usize {
        self.program + self.catalog + self.hardware + self.master
    }

    pub fn add(&mut self, other: TranslationCounts) {
        self.program += other.program;
        self.catalog += other.catalog;
        self.hardware += other.hardware;
        self.master += other.master;
    }
}

#[derive(Debug, Clone, Default)]
pub struct UnknownCollector {
    seen: BTreeMap<(String, &'static str, String), UnknownConstruct>,
}

impl UnknownCollector {
    pub fn element(&mut self, xpath: &str, name: &str) {
        self.record(xpath, UnknownKind::Element, name, None);
    }

    pub fn attribute(&mut self, xpath: &str, name: &str, sample: &str) {
        self.record(xpath, UnknownKind::Attribute, name, Some(sample));
    }

    fn record(&mut self, xpath: &str, kind: UnknownKind, name: &str, sample: Option<&str>) {
        let key = (xpath.to_string(), kind.as_str(), name.to_string());
        self.seen
            .entry(key)
            .and_modify(|u| u.occurrences += 1)
            .or_insert_with(|| UnknownConstruct {
                xpath: xpath.to_string(),
                kind,
                name: name.to_string(),
                occurrences: 1,
                sample: sample.map(str::to_string),
            });
    }

    pub fn into_vec(self) -> Vec<UnknownConstruct> {
        self.seen.into_values().collect()
    }
}

pub fn insert_unknown(
    conn: &Connection,
    source_sha256: &str,
    unknown: &[UnknownConstruct],
) -> Result<(), ProductDbError> {
    let mut stmt = conn.prepare(
        "INSERT INTO ingest_unknown (source_sha256, program_id, xpath, kind, name, occurrences, sample)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
    )?;
    for u in unknown {
        stmt.execute(params![
            source_sha256,
            Option::<String>::None,
            u.xpath,
            u.kind.as_str(),
            u.name,
            u.occurrences as i64,
            u.sample,
        ])?;
    }
    Ok(())
}

/// Records an id collision as an `ingest_unknown` row with
/// `kind = 'IdConflict'`, so both hashes and the winning row stay on record
/// without a table of its own (spec §5). `occurrences` is repurposed to
/// carry `IdConflict::occurrence` — `1` for every conflict recorded before
/// KNOWN_LIMITATIONS.md §86 was closed, since that was the only value
/// `first_winner` ever produced; a same-file collision now lands here as
/// whatever repeat count it was, so this row alone says which kind it was
/// without a schema change to `ingest_unknown`.
pub fn insert_conflicts(conn: &Connection, conflicts: &[IdConflict]) -> Result<(), ProductDbError> {
    let mut stmt = conn.prepare(
        "INSERT INTO ingest_unknown (source_sha256, program_id, xpath, kind, name, occurrences, sample)
         VALUES (?1, NULL, ?2, 'IdConflict', ?3, ?5, ?4)",
    )?;
    for c in conflicts {
        stmt.execute(params![
            c.kept_sha256,
            c.table,
            c.id,
            c.other_sha256,
            c.occurrence
        ])?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::open_and_migrate;

    #[test]
    fn repeated_unknowns_collapse_into_one_counted_row() {
        let mut c = UnknownCollector::default();
        c.attribute("/KNX/ManufacturerData/Manufacturer/Hardware", "Fancy", "1");
        c.attribute("/KNX/ManufacturerData/Manufacturer/Hardware", "Fancy", "2");
        c.element("/KNX/ManufacturerData/Manufacturer", "Extension");
        let v = c.into_vec();
        assert_eq!(v.len(), 2);
        let fancy = v.iter().find(|u| u.name == "Fancy").unwrap();
        assert_eq!(fancy.occurrences, 2);
        assert_eq!(fancy.kind, UnknownKind::Attribute);
        // The first sample is kept, so the value is from the first sighting.
        assert_eq!(fancy.sample.as_deref(), Some("1"));
    }

    #[test]
    fn unknown_rows_persist_with_their_source_hash() {
        let dir = tempfile::tempdir().unwrap();
        let conn = open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
        let mut c = UnknownCollector::default();
        c.element("/KNX/ManufacturerData", "Whatsit");
        insert_unknown(&conn, "abc123", &c.into_vec()).unwrap();
        let (name, occurrences): (String, i64) = conn
            .query_row(
                "SELECT name, occurrences FROM ingest_unknown WHERE source_sha256 = 'abc123'",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert_eq!(name, "Whatsit");
        assert_eq!(occurrences, 1);
    }
}
