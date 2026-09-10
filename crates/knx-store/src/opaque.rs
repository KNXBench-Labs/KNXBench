//! The opaque passthrough table: persists everything the domain model does
//! not carry, as bytes plus a hash, so export can write it back unchanged.
//!
//! This crate does not depend on `knx-etsproj`: `StoredOpaqueEntry` is this
//! table's own row shape, not `knx-etsproj::opaque::OpaqueEntry` renamed.
//! `knx-app`, the only crate that knows both, is where the conversion
//! between the two happens — keeping that conversion out of `knx-store` is
//! what stops the storage schema from being defined by the import format.

use rusqlite::{params, Connection};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredOpaqueEntry {
    pub source_path: String,
    pub xpath: String,
    pub kind: String,
    pub name: String,
    pub bytes: Vec<u8>,
    pub sha256: String,
}

/// Replaces the whole table with `entries`, inside one transaction with a
/// prepared statement — a real project's opaque bytes can total tens of
/// megabytes across dozens of entries, and a partial insert would leave a
/// project file that cannot be exported.
///
/// Clears the table before inserting, mirroring `knx_store::project::
/// save_project`'s own DELETE-then-insert convention: this is meant to be
/// called on every save of a project (see `apps/knx-server`'s
/// `save_project_as_impl`), and without the clear, saving the same store
/// twice duplicates every row unboundedly.
pub fn insert_opaque(
    conn: &Connection,
    entries: &[StoredOpaqueEntry],
) -> Result<usize, rusqlite::Error> {
    // `unchecked_transaction` rather than `Connection::transaction`: the
    // interface this task is given takes `&Connection`, not `&mut
    // Connection`, and rusqlite's own unchecked variant exists precisely
    // for a caller that only has a shared reference. The transaction rolls
    // back on drop unless committed, so an error partway through any one
    // entry leaves the table exactly as it was before this call.
    let tx = conn.unchecked_transaction()?;
    tx.execute("DELETE FROM opaque_entry", [])?;
    let mut count = 0;
    {
        let mut stmt = tx.prepare(
            "INSERT INTO opaque_entry (source_path, xpath, kind, name, bytes, sha256)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        )?;
        for entry in entries {
            stmt.execute(params![
                entry.source_path,
                entry.xpath,
                entry.kind,
                entry.name,
                entry.bytes,
                entry.sha256,
            ])?;
            count += 1;
        }
    }
    tx.commit()?;
    Ok(count)
}

pub fn load_opaque(conn: &Connection) -> Result<Vec<StoredOpaqueEntry>, rusqlite::Error> {
    let mut stmt = conn.prepare(
        "SELECT source_path, xpath, kind, name, bytes, sha256 FROM opaque_entry ORDER BY id",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(StoredOpaqueEntry {
            source_path: row.get(0)?,
            xpath: row.get(1)?,
            kind: row.get(2)?,
            name: row.get(3)?,
            bytes: row.get(4)?,
            sha256: row.get(5)?,
        })
    })?;
    rows.collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::open_and_migrate;

    fn entries() -> Vec<StoredOpaqueEntry> {
        vec![StoredOpaqueEntry {
            source_path: "M-0083/Catalog.xml".into(),
            xpath: "/Project/Installation".into(),
            kind: "Element".into(),
            name: "Trade".into(),
            bytes: b"<Trade/>".to_vec(),
            sha256: "aa".into(),
        }]
    }

    #[test]
    fn saving_the_same_store_twice_does_not_duplicate_rows() {
        // Regression for the whole-branch review of the export-UI plan
        // (T10, round 2): `save_project_as_impl` calls `insert_opaque` on
        // every save, including a plain re-Save of an already-populated
        // `.knxdb`. Without the DELETE-first step, that duplicated every
        // row on every repeated save.
        let dir = tempfile::tempdir().unwrap();
        let conn = open_and_migrate(&dir.path().join("p.sqlite")).unwrap();
        assert_eq!(insert_opaque(&conn, &entries()).unwrap(), 1);
        assert_eq!(insert_opaque(&conn, &entries()).unwrap(), 1);
        assert_eq!(insert_opaque(&conn, &entries()).unwrap(), 1);
        assert_eq!(load_opaque(&conn).unwrap(), entries());
    }
}
