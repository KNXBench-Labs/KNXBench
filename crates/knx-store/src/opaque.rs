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

/// Inserts every entry inside one transaction with a prepared statement — a
/// real project's opaque bytes can total tens of megabytes across dozens of
/// entries, and a partial insert would leave a project file that cannot be
/// exported.
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
