//! Manufacturer files a native project file carries itself (schema v16, ADR-0128).
//!
//! The manifest (`manifest.rs`) names every manufacturer file a project was
//! imported with: application programs, hardware, catalogue. Before v16 a
//! project imported with a product database kept only that manifest, so a
//! `.knxdb` opened next to an empty product database could not resolve a
//! single application program (GitHub #21). This table holds the files'
//! bytes, zlib-compressed and keyed by the SHA-256 the manifest records.
//!
//! The rows belong to the file, not to the editor history: a native
//! snapshot never contains them, so undo, redo and versions never copy
//! megabytes of XML. Every read re-checks length and hash.

use std::collections::BTreeSet;
use std::fmt;
use std::io::{Read, Write};

use flate2::{read::ZlibDecoder, write::ZlibEncoder, Compression};
use rusqlite::{params, Connection, OptionalExtension};
use sha2::{Digest, Sha256};

/// The largest file this table admits, decoded. The same bound the
/// `.knxproj` reader puts on one archive member, so no manifest entry an
/// import accepted can be refused here.
pub const MAX_MANUFACTURER_FILE_BYTES: usize = knx_core::limits::MAX_RETAINED_BYTES_USIZE;

/// Why a carried manufacturer file could not be stored or read back.
#[derive(Debug)]
pub enum ManufacturerFileError {
    Sqlite(rusqlite::Error),
    Io(std::io::Error),
    /// The file is larger than [`MAX_MANUFACTURER_FILE_BYTES`].
    TooLarge {
        sha256: String,
        len: usize,
    },
    /// The stored row does not decode to the bytes its key names.
    Corrupt {
        sha256: String,
        reason: &'static str,
    },
}

impl fmt::Display for ManufacturerFileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Sqlite(e) => write!(f, "{e}"),
            Self::Io(e) => write!(f, "{e}"),
            Self::TooLarge { sha256, len } => write!(
                f,
                "manufacturer file {sha256} has {len} bytes, more than the \
                 {MAX_MANUFACTURER_FILE_BYTES}-byte limit"
            ),
            Self::Corrupt { sha256, reason } => {
                write!(f, "carried manufacturer file {sha256} is damaged: {reason}")
            }
        }
    }
}

impl std::error::Error for ManufacturerFileError {}

impl From<rusqlite::Error> for ManufacturerFileError {
    fn from(e: rusqlite::Error) -> Self {
        Self::Sqlite(e)
    }
}

impl From<std::io::Error> for ManufacturerFileError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

/// One carried file, decoded and verified.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CarriedManufacturerFile {
    pub source_path: String,
    pub bytes: Vec<u8>,
}

/// Lower-case hexadecimal SHA-256, the spelling the manifest uses.
pub fn manufacturer_file_hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

/// The hashes this file can hand back: rows of this table plus whole-file
/// opaque entries (a project imported without a product database keeps its
/// manufacturer files there, with the same hash).
pub fn carried_manufacturer_hashes(conn: &Connection) -> Result<BTreeSet<String>, rusqlite::Error> {
    let mut stmt = conn.prepare(
        "SELECT sha256 FROM manufacturer_file
         UNION SELECT sha256 FROM opaque_entry WHERE xpath = ''",
    )?;
    let hashes = stmt.query_map([], |row| row.get(0))?;
    hashes.collect()
}

/// Stores `bytes` under its own hash unless that hash is already carried.
/// Returns whether a row was written.
pub fn store_manufacturer_file(
    conn: &Connection,
    source_path: &str,
    bytes: &[u8],
) -> Result<bool, ManufacturerFileError> {
    let sha256 = manufacturer_file_hash(bytes);
    if bytes.len() > MAX_MANUFACTURER_FILE_BYTES {
        return Err(ManufacturerFileError::TooLarge {
            sha256,
            len: bytes.len(),
        });
    }
    let present: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM manufacturer_file WHERE sha256 = ?1)",
        [&sha256],
        |row| row.get(0),
    )?;
    if present {
        return Ok(false);
    }
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(bytes)?;
    let compressed = encoder.finish()?;
    conn.execute(
        "INSERT INTO manufacturer_file (sha256, source_path, len, codec, bytes)
         VALUES (?1, ?2, ?3, 'zlib', ?4)",
        params![sha256, source_path, bytes.len() as i64, compressed],
    )?;
    Ok(true)
}

/// Reads the file carried under `sha256` from this table or from a
/// whole-file opaque entry. `None` when the file carries no such bytes.
/// Bytes that do not match their key are an error, never returned.
pub fn load_manufacturer_file(
    conn: &Connection,
    sha256: &str,
) -> Result<Option<CarriedManufacturerFile>, ManufacturerFileError> {
    let row: Option<(String, i64, Vec<u8>)> = conn
        .query_row(
            "SELECT source_path, len, bytes FROM manufacturer_file WHERE sha256 = ?1",
            [sha256],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .optional()?;
    let (source_path, bytes) = match row {
        Some((source_path, len, compressed)) => {
            let corrupt = |reason| ManufacturerFileError::Corrupt {
                sha256: sha256.to_string(),
                reason,
            };
            let len = usize::try_from(len)
                .ok()
                .filter(|len| *len <= MAX_MANUFACTURER_FILE_BYTES)
                .ok_or_else(|| corrupt("declared length out of range"))?;
            let mut bytes = Vec::with_capacity(len);
            ZlibDecoder::new(compressed.as_slice())
                .take(len as u64 + 1)
                .read_to_end(&mut bytes)
                .map_err(|_| corrupt("compressed stream does not decode"))?;
            if bytes.len() != len {
                return Err(corrupt("decoded length differs from the declared length"));
            }
            (source_path, bytes)
        }
        None => {
            let opaque: Option<(String, Vec<u8>)> = conn
                .query_row(
                    "SELECT source_path, bytes FROM opaque_entry
                     WHERE sha256 = ?1 AND xpath = '' ORDER BY id LIMIT 1",
                    [sha256],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )
                .optional()?;
            match opaque {
                Some(found) => found,
                None => return Ok(None),
            }
        }
    };
    if manufacturer_file_hash(&bytes) != sha256 {
        return Err(ManufacturerFileError::Corrupt {
            sha256: sha256.to_string(),
            reason: "bytes do not match their SHA-256",
        });
    }
    Ok(Some(CarriedManufacturerFile { source_path, bytes }))
}

/// Deletes every carried row whose hash `keep` does not name, so a file
/// holds exactly the files its saved manifest lists. Returns the count.
pub fn retain_manufacturer_files(
    conn: &Connection,
    keep: &BTreeSet<String>,
) -> Result<usize, rusqlite::Error> {
    let stored: Vec<String> = conn
        .prepare("SELECT sha256 FROM manufacturer_file")?
        .query_map([], |row| row.get(0))?
        .collect::<Result<_, _>>()?;
    let mut deleted = 0;
    for sha256 in stored.iter().filter(|sha256| !keep.contains(*sha256)) {
        deleted += conn.execute("DELETE FROM manufacturer_file WHERE sha256 = ?1", [sha256])?;
    }
    Ok(deleted)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::open_and_migrate_in_memory;

    const PROGRAM: &[u8] = b"<KNX><ManufacturerData /></KNX>";

    #[test]
    fn a_stored_file_reads_back_byte_for_byte_and_only_once() {
        let conn = open_and_migrate_in_memory().unwrap();
        assert!(store_manufacturer_file(&conn, "M-0001/M-0001_A-1.xml", PROGRAM).unwrap());
        assert!(!store_manufacturer_file(&conn, "M-0001/other.xml", PROGRAM).unwrap());
        let sha = manufacturer_file_hash(PROGRAM);
        let back = load_manufacturer_file(&conn, &sha).unwrap().unwrap();
        assert_eq!(back.bytes, PROGRAM);
        assert_eq!(back.source_path, "M-0001/M-0001_A-1.xml");
        assert!(carried_manufacturer_hashes(&conn).unwrap().contains(&sha));
    }

    #[test]
    fn rows_are_stored_compressed() {
        let conn = open_and_migrate_in_memory().unwrap();
        let big = PROGRAM.repeat(10_000);
        store_manufacturer_file(&conn, "M-0001/big.xml", &big).unwrap();
        let stored: i64 = conn
            .query_row("SELECT length(bytes) FROM manufacturer_file", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert!(
            (stored as usize) < big.len() / 10,
            "{stored} of {}",
            big.len()
        );
    }

    #[test]
    fn an_unknown_hash_is_none() {
        let conn = open_and_migrate_in_memory().unwrap();
        assert!(load_manufacturer_file(&conn, &"0".repeat(64))
            .unwrap()
            .is_none());
    }

    #[test]
    fn a_whole_file_opaque_entry_counts_as_carried() {
        let conn = open_and_migrate_in_memory().unwrap();
        let sha = manufacturer_file_hash(PROGRAM);
        crate::insert_opaque(
            &conn,
            &[crate::StoredOpaqueEntry {
                source_path: "M-0001/M-0001_A-1.xml".into(),
                xpath: String::new(),
                kind: "ManufacturerData".into(),
                name: String::new(),
                bytes: PROGRAM.to_vec(),
                sha256: sha.clone(),
            }],
        )
        .unwrap();
        assert!(carried_manufacturer_hashes(&conn).unwrap().contains(&sha));
        assert_eq!(
            load_manufacturer_file(&conn, &sha).unwrap().unwrap().bytes,
            PROGRAM
        );
    }

    #[test]
    fn damaged_rows_are_refused_not_returned() {
        let conn = open_and_migrate_in_memory().unwrap();
        store_manufacturer_file(&conn, "M-0001/a.xml", PROGRAM).unwrap();
        let sha = manufacturer_file_hash(PROGRAM);
        // A different, valid stream under the same key.
        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(b"<KNX>tampered</KNX>").unwrap();
        let other = encoder.finish().unwrap();
        conn.execute("UPDATE manufacturer_file SET bytes = ?1, len = 19", [other])
            .unwrap();
        assert!(matches!(
            load_manufacturer_file(&conn, &sha),
            Err(ManufacturerFileError::Corrupt { .. })
        ));
        // A length that does not match the stream.
        store_manufacturer_file(&conn, "M-0001/b.xml", b"second").unwrap();
        let second = manufacturer_file_hash(b"second");
        conn.execute(
            "UPDATE manufacturer_file SET len = 3 WHERE sha256 = ?1",
            [&second],
        )
        .unwrap();
        assert!(matches!(
            load_manufacturer_file(&conn, &second),
            Err(ManufacturerFileError::Corrupt { .. })
        ));
    }

    #[test]
    fn retain_keeps_exactly_the_named_hashes() {
        let conn = open_and_migrate_in_memory().unwrap();
        store_manufacturer_file(&conn, "M-0001/a.xml", b"a").unwrap();
        store_manufacturer_file(&conn, "M-0001/b.xml", b"b").unwrap();
        let keep = BTreeSet::from([manufacturer_file_hash(b"a")]);
        assert_eq!(retain_manufacturer_files(&conn, &keep).unwrap(), 1);
        assert_eq!(carried_manufacturer_hashes(&conn).unwrap(), keep);
    }

    #[test]
    fn a_v15_file_gains_an_empty_table_and_nothing_else_changes() {
        let conn = open_and_migrate_in_memory().unwrap();
        conn.execute_batch("DROP TABLE manufacturer_file; PRAGMA user_version = 15;")
            .unwrap();
        crate::migration::migrate(&conn).unwrap();
        let version: i64 = conn
            .query_row("PRAGMA user_version", [], |r| r.get(0))
            .unwrap();
        assert_eq!(version, crate::CURRENT_SCHEMA_VERSION);
        assert!(carried_manufacturer_hashes(&conn).unwrap().is_empty());
    }

    #[test]
    fn the_schema_refuses_a_malformed_key() {
        let conn = open_and_migrate_in_memory().unwrap();
        let refused = conn.execute(
            "INSERT INTO manufacturer_file (sha256, source_path, len, codec, bytes)
             VALUES ('ABC', 'x', 0, 'zlib', x'')",
            [],
        );
        assert!(refused.is_err());
    }
}
