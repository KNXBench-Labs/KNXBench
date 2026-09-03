//! The `source_file` table: every ingested manufacturer file kept verbatim,
//! keyed by the SHA-256 of its bytes.
//!
//! This is the integrity anchor of the whole crate. The parsers below it
//! model what they understand; anything they do not understand — the
//! `Dynamic` subtree, a vendor's `Legacy*` option, a construct from a
//! manufacturer we have never seen — is still here, byte for byte, and the
//! exporter reads it back from here (IMPORT_EXPORT §10).

use rusqlite::{params, Connection, OptionalExtension};
use sha2::{Digest, Sha256};

use crate::ProductDbError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceFile {
    pub source_path: String,
    pub manufacturer_id: Option<String>,
    pub bytes: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlobMismatch {
    pub sha256: String,
    pub source_path: String,
    pub actual_sha256: String,
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hasher
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// Stores `file` unless its hash is already present. Returns whether a row
/// was written, which is what tells the ingest whether it still has to
/// parse the file.
pub fn store_source_file(conn: &Connection, file: &SourceFile) -> Result<bool, ProductDbError> {
    let sha = sha256_hex(&file.bytes);
    if has_source_file(conn, &sha)? {
        return Ok(false);
    }
    conn.execute(
        "INSERT INTO source_file (sha256, source_path, manufacturer_id, len, bytes)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![
            sha,
            file.source_path,
            file.manufacturer_id,
            file.bytes.len() as i64,
            file.bytes
        ],
    )?;
    Ok(true)
}

pub fn has_source_file(conn: &Connection, sha256: &str) -> Result<bool, ProductDbError> {
    let found: Option<i64> = conn
        .query_row(
            "SELECT 1 FROM source_file WHERE sha256 = ?1",
            [sha256],
            |r| r.get(0),
        )
        .optional()?;
    Ok(found.is_some())
}

pub fn load_source_file(
    conn: &Connection,
    sha256: &str,
) -> Result<Option<Vec<u8>>, ProductDbError> {
    let bytes: Option<Vec<u8>> = conn
        .query_row(
            "SELECT bytes FROM source_file WHERE sha256 = ?1",
            [sha256],
            |r| r.get(0),
        )
        .optional()?;
    Ok(bytes)
}

/// Re-hashes every blob and reports the rows whose bytes no longer match
/// their key. Backs `knx products verify`.
pub fn verify(conn: &Connection) -> Result<Vec<BlobMismatch>, ProductDbError> {
    let mut stmt = conn.prepare("SELECT sha256, source_path, bytes FROM source_file")?;
    let rows = stmt.query_map([], |r| {
        Ok((
            r.get::<_, String>(0)?,
            r.get::<_, String>(1)?,
            r.get::<_, Vec<u8>>(2)?,
        ))
    })?;
    let mut out = Vec::new();
    for row in rows {
        let (sha256, source_path, bytes) = row?;
        let actual = sha256_hex(&bytes);
        if actual != sha256 {
            out.push(BlobMismatch {
                sha256,
                source_path,
                actual_sha256: actual,
            });
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::open_and_migrate;

    fn db() -> (tempfile::TempDir, Connection) {
        let dir = tempfile::tempdir().unwrap();
        let conn = open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
        (dir, conn)
    }

    fn file(path: &str, bytes: &[u8]) -> SourceFile {
        SourceFile {
            source_path: path.to_string(),
            manufacturer_id: Some("M-0083".into()),
            bytes: bytes.to_vec(),
        }
    }

    #[test]
    fn a_blob_round_trips_through_sqlite_unchanged() {
        let (_dir, conn) = db();
        let f = file("M-0083/Catalog.xml", b"<KNX/>");
        let sha = sha256_hex(&f.bytes);
        assert!(store_source_file(&conn, &f).unwrap());
        assert_eq!(load_source_file(&conn, &sha).unwrap(), Some(b"<KNX/>".to_vec()));
    }

    #[test]
    fn storing_the_same_bytes_twice_stores_one_row() {
        let (_dir, conn) = db();
        let f = file("M-0083/Catalog.xml", b"<KNX/>");
        assert!(store_source_file(&conn, &f).unwrap());
        assert!(!store_source_file(&conn, &f).unwrap());
        let rows: i64 = conn
            .query_row("SELECT count(*) FROM source_file", [], |r| r.get(0))
            .unwrap();
        assert_eq!(rows, 1);
    }

    #[test]
    fn the_same_bytes_under_a_different_path_still_store_one_row() {
        // Two projects carry the identical application program at the same
        // path; a third could carry it renamed. Identity is the content
        // hash, never the path (ADR-0011).
        let (_dir, conn) = db();
        assert!(store_source_file(&conn, &file("M-0083/A.xml", b"same")).unwrap());
        assert!(!store_source_file(&conn, &file("M-0083/B.xml", b"same")).unwrap());
        let rows: i64 = conn
            .query_row("SELECT count(*) FROM source_file", [], |r| r.get(0))
            .unwrap();
        assert_eq!(rows, 1);
    }

    #[test]
    fn a_missing_hash_loads_as_none() {
        let (_dir, conn) = db();
        assert_eq!(load_source_file(&conn, "deadbeef").unwrap(), None);
        assert!(!has_source_file(&conn, "deadbeef").unwrap());
    }

    #[test]
    fn verify_reports_a_blob_whose_bytes_no_longer_match_its_key() {
        let (_dir, conn) = db();
        store_source_file(&conn, &file("M-0083/Catalog.xml", b"<KNX/>")).unwrap();
        assert_eq!(verify(&conn).unwrap(), vec![]);
        conn.execute("UPDATE source_file SET bytes = ?1", [&b"tampered"[..]])
            .unwrap();
        let bad = verify(&conn).unwrap();
        assert_eq!(bad.len(), 1);
        assert_eq!(bad[0].source_path, "M-0083/Catalog.xml");
        assert_eq!(bad[0].actual_sha256, sha256_hex(b"tampered"));
    }

    #[test]
    fn a_binary_blob_survives_byte_for_byte() {
        let (_dir, conn) = db();
        let bytes: Vec<u8> = (0u8..=255).cycle().take(4096).collect();
        let f = file("M-0008/Baggages/econEts3.dll", &bytes);
        let sha = sha256_hex(&bytes);
        store_source_file(&conn, &f).unwrap();
        assert_eq!(load_source_file(&conn, &sha).unwrap(), Some(bytes));
    }
}
