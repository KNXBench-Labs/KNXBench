//! The manufacturer manifest: what a project was imported with, so it can
//! still name the gap when the shared product database is absent (spec
//! §6, ADR-0005) rather than silently losing the information.
//!
//! Mirrors `opaque.rs`: this crate does not depend on `knx-etsproj` or
//! `knx-productdb`, so `ManufacturerRef` is this table's own row shape, not
//! either crate's type renamed. `knx-app` is where the conversion happens.

use rusqlite::{params, Connection};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManufacturerRef {
    pub source_path: String,
    pub sha256: String,
    pub len: i64,
    pub kind: String,
}

/// Replaces the whole table with `refs`, inside one transaction with a
/// prepared statement — the same guarantee `insert_opaque` gives: a failure
/// partway through leaves the table exactly as it was before this call, and
/// the table is cleared before inserting so saving the same store twice
/// does not duplicate rows (see `insert_opaque`'s doc comment).
pub fn insert_manufacturer_refs(
    conn: &Connection,
    refs: &[ManufacturerRef],
) -> Result<usize, rusqlite::Error> {
    let tx = conn.unchecked_transaction()?;
    let count = write_manufacturer_refs(&tx, refs)?;
    tx.commit()?;
    Ok(count)
}

/// The body of [`insert_manufacturer_refs`], inside a transaction the
/// caller owns (`save_project_with_passthrough`).
pub(crate) fn write_manufacturer_refs(
    conn: &Connection,
    refs: &[ManufacturerRef],
) -> Result<usize, rusqlite::Error> {
    conn.execute("DELETE FROM manufacturer_ref", [])?;
    let mut count = 0;
    let mut stmt = conn.prepare(
        "INSERT INTO manufacturer_ref (source_path, sha256, len, kind)
         VALUES (?1, ?2, ?3, ?4)",
    )?;
    for r in refs {
        stmt.execute(params![r.source_path, r.sha256, r.len, r.kind])?;
        count += 1;
    }
    Ok(count)
}

pub fn load_manufacturer_refs(conn: &Connection) -> Result<Vec<ManufacturerRef>, rusqlite::Error> {
    let mut stmt =
        conn.prepare("SELECT source_path, sha256, len, kind FROM manufacturer_ref ORDER BY id")?;
    let rows = stmt.query_map([], |row| {
        Ok(ManufacturerRef {
            source_path: row.get(0)?,
            sha256: row.get(1)?,
            len: row.get(2)?,
            kind: row.get(3)?,
        })
    })?;
    rows.collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::open_and_migrate;

    fn refs() -> Vec<ManufacturerRef> {
        vec![
            ManufacturerRef {
                source_path: "M-0083/Catalog.xml".into(),
                sha256: "aa".into(),
                len: 21155,
                kind: "ManufacturerData".into(),
            },
            ManufacturerRef {
                source_path: "M-0008/Baggages/econEts3.dll".into(),
                sha256: "bb".into(),
                len: 641536,
                kind: "Baggage".into(),
            },
        ]
    }

    #[test]
    fn the_manifest_round_trips_in_insertion_order() {
        let dir = tempfile::tempdir().unwrap();
        let conn = open_and_migrate(&dir.path().join("p.sqlite")).unwrap();
        assert_eq!(insert_manufacturer_refs(&conn, &refs()).unwrap(), 2);
        assert_eq!(load_manufacturer_refs(&conn).unwrap(), refs());
    }

    #[test]
    fn the_manifest_records_a_file_the_project_no_longer_carries() {
        // The point of the manifest: a project can still name what it was
        // imported with, even when the product database is gone (spec §6).
        let dir = tempfile::tempdir().unwrap();
        let conn = open_and_migrate(&dir.path().join("p.sqlite")).unwrap();
        insert_manufacturer_refs(&conn, &refs()).unwrap();
        let loaded = load_manufacturer_refs(&conn).unwrap();
        assert_eq!(loaded[1].len, 641536);
        assert_eq!(loaded[1].sha256, "bb");
    }

    #[test]
    fn saving_the_same_store_twice_does_not_duplicate_rows() {
        // Regression for the whole-branch review of the export-UI plan
        // (T10, round 2) — same bug class as `opaque.rs`'s test of the
        // same name.
        let dir = tempfile::tempdir().unwrap();
        let conn = open_and_migrate(&dir.path().join("p.sqlite")).unwrap();
        assert_eq!(insert_manufacturer_refs(&conn, &refs()).unwrap(), 2);
        assert_eq!(insert_manufacturer_refs(&conn, &refs()).unwrap(), 2);
        assert_eq!(insert_manufacturer_refs(&conn, &refs()).unwrap(), 2);
        assert_eq!(load_manufacturer_refs(&conn).unwrap(), refs());
    }
}
