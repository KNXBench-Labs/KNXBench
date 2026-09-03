//! Per-file ingest: hash, skip-or-parse, one transaction, classification.
//!
//! The content hash is the identity (ADR-0011). A file already in
//! `source_file` is skipped without being parsed at all — which is what
//! makes importing a second project that uses the same devices cheap
//! instead of costing another 22 MB of parsing (RESEARCH §4.1).

use rusqlite::Connection;

use crate::blob::{has_source_file, sha256_hex, store_source_file, SourceFile};
use crate::parse::{catalog, hardware, program};
use crate::report::{insert_conflicts, insert_unknown, IdConflict};
use crate::ProductDbError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileKind {
    Catalog,
    Hardware,
    ApplicationProgram,
    Baggages,
    Baggage,
    Unrecognized,
}

#[derive(Debug)]
pub enum IngestOutcome {
    Ingested {
        sha256: String,
        kind: FileKind,
        unknown: usize,
        conflicts: Vec<IdConflict>,
    },
    Skipped {
        sha256: String,
    },
}

/// Ingests one manufacturer file. Everything this function writes — the
/// blob, the parsed rows, the unknown-construct rows — happens in one
/// transaction, so a parse error partway through leaves the database
/// exactly as it was rather than storing a blob whose rows never landed.
pub fn ingest_file(
    conn: &Connection,
    source_path: &str,
    bytes: &[u8],
) -> Result<IngestOutcome, ProductDbError> {
    let sha256 = sha256_hex(bytes);
    if has_source_file(conn, &sha256)? {
        return Ok(IngestOutcome::Skipped { sha256 });
    }

    let tx = conn.unchecked_transaction()?;
    let manufacturer_id = source_path
        .split('/')
        .next()
        .filter(|top| top.starts_with("M-"))
        .map(str::to_string);
    store_source_file(
        &tx,
        &SourceFile {
            source_path: source_path.to_string(),
            manufacturer_id,
            bytes: bytes.to_vec(),
        },
    )?;

    let kind = classify(bytes);
    let (unknown, conflicts) = match kind {
        FileKind::Catalog => (
            catalog::ingest_catalog(&tx, &sha256, source_path, bytes)?,
            Vec::new(),
        ),
        FileKind::Hardware => (
            hardware::ingest_hardware(&tx, &sha256, source_path, bytes)?,
            Vec::new(),
        ),
        FileKind::ApplicationProgram => {
            let out = program::ingest_program(&tx, &sha256, source_path, bytes)?;
            (out.unknown, out.conflicts)
        }
        // Baggages.xml lists the blobs; the blobs themselves and anything
        // unrecognized are stored and not parsed.
        FileKind::Baggages | FileKind::Baggage | FileKind::Unrecognized => (Vec::new(), Vec::new()),
    };

    insert_unknown(&tx, &sha256, &unknown)?;
    insert_conflicts(&tx, &conflicts)?;
    tx.commit()?;

    Ok(IngestOutcome::Ingested {
        sha256,
        kind,
        unknown: unknown.len(),
        conflicts,
    })
}

/// Classifies by the first recognized element inside `ManufacturerData`,
/// not by file name: the name is a convention, the content is the fact.
/// A `Baggages/` blob is not XML at all, so it is recognized by its bytes.
fn classify(bytes: &[u8]) -> FileKind {
    let text = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(bytes);
    if text.iter().find(|b| !b.is_ascii_whitespace()) != Some(&b'<') {
        return FileKind::Baggage;
    }

    let mut reader = quick_xml::Reader::from_reader(text);
    let mut buf = Vec::new();
    loop {
        buf.clear();
        match reader.read_event_into(&mut buf) {
            Ok(quick_xml::events::Event::Start(e)) | Ok(quick_xml::events::Event::Empty(e)) => {
                match crate::xml::local_name(&e).as_str() {
                    "Catalog" => return FileKind::Catalog,
                    "Hardware" => return FileKind::Hardware,
                    "ApplicationPrograms" => return FileKind::ApplicationProgram,
                    "Baggages" => return FileKind::Baggages,
                    _ => {}
                }
            }
            Ok(quick_xml::events::Event::Eof) | Err(_) => return FileKind::Unrecognized,
            Ok(_) => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::open_and_migrate;

    const HARDWARE: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<KNX xmlns="http://knx.org/xml/project/11"><ManufacturerData><Manufacturer RefId="M-006A">
<Hardware><Hardware Id="H-1" Name="X" SerialNumber="S" VersionNumber="1"><Products>
<Product Id="H-1_P-1" Text="X" OrderNumber="N1" /></Products></Hardware></Hardware>
</Manufacturer></ManufacturerData></KNX>"#;

    fn db() -> (tempfile::TempDir, Connection) {
        let dir = tempfile::tempdir().unwrap();
        let conn = open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
        (dir, conn)
    }

    #[test]
    fn a_file_is_classified_by_its_content_not_its_name() {
        let (_dir, conn) = db();
        let out = ingest_file(&conn, "M-006A/Whatever.xml", HARDWARE.as_bytes()).unwrap();
        assert!(matches!(
            out,
            IngestOutcome::Ingested {
                kind: FileKind::Hardware,
                ..
            }
        ));
    }

    #[test]
    fn the_second_ingest_of_the_same_bytes_is_skipped_without_parsing() {
        let (_dir, conn) = db();
        assert!(matches!(
            ingest_file(&conn, "M-006A/Hardware.xml", HARDWARE.as_bytes()).unwrap(),
            IngestOutcome::Ingested { .. }
        ));
        assert!(matches!(
            ingest_file(&conn, "M-006A/Hardware.xml", HARDWARE.as_bytes()).unwrap(),
            IngestOutcome::Skipped { .. }
        ));
        let rows: i64 = conn
            .query_row("SELECT count(*) FROM hardware", [], |r| r.get(0))
            .unwrap();
        assert_eq!(rows, 1);
    }

    #[test]
    fn a_baggage_blob_is_stored_without_being_parsed() {
        let (_dir, conn) = db();
        let dll = b"MZ\x90\x00binary".to_vec();
        let out = ingest_file(&conn, "M-0008/Baggages/econEts3.dll", &dll).unwrap();
        let sha = match out {
            IngestOutcome::Ingested {
                sha256,
                kind: FileKind::Baggage,
                ..
            } => sha256,
            other => panic!("expected an ingested baggage blob, got {other:?}"),
        };
        assert_eq!(crate::load_source_file(&conn, &sha).unwrap(), Some(dll));
    }

    #[test]
    fn a_failing_parse_leaves_no_partial_rows_and_no_blob() {
        // One transaction per file: a truncated program must not leave the
        // database holding half of it, or the content-hash skip would then
        // consider that half complete for ever.
        let (_dir, conn) = db();
        let truncated = &HARDWARE.as_bytes()[..HARDWARE.len() / 2];
        assert!(ingest_file(&conn, "M-006A/Hardware.xml", truncated).is_err());
        let blobs: i64 = conn
            .query_row("SELECT count(*) FROM source_file", [], |r| r.get(0))
            .unwrap();
        let hardware: i64 = conn
            .query_row("SELECT count(*) FROM hardware", [], |r| r.get(0))
            .unwrap();
        assert_eq!((blobs, hardware), (0, 0));
    }

    #[test]
    fn an_unrecognized_xml_file_is_still_stored_as_a_blob() {
        let (_dir, conn) = db();
        let xml = br#"<?xml version="1.0"?><KNX><SomethingNew/></KNX>"#;
        let out = ingest_file(&conn, "M-006A/New.xml", xml).unwrap();
        assert!(matches!(
            out,
            IngestOutcome::Ingested {
                kind: FileKind::Unrecognized,
                ..
            }
        ));
        assert_eq!(
            crate::load_source_file(&conn, &crate::sha256_hex(xml)).unwrap(),
            Some(xml.to_vec())
        );
    }
}
