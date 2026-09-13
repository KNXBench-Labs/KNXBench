//! Per-file ingest: hash, skip-or-parse, one transaction, classification.
//!
//! The content hash is the identity (ADR-0011). A file already in
//! `source_file` is skipped without being parsed at all — which is what
//! makes importing a second project that uses the same devices cheap
//! instead of costing another 22 MB of parsing (RESEARCH §4.1).

use rusqlite::{Connection, OptionalExtension};

use crate::blob::{sha256_hex, store_source_file, SourceFile};
use crate::dynamic;
use crate::parse::translation::{ingest_translations, TranslationScope};
use crate::parse::{catalog, hardware, program};
use crate::report::{insert_conflicts, insert_unknown, IdConflict, TranslationCounts};
use crate::ProductDbError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileKind {
    Catalog,
    Hardware,
    ApplicationProgram,
    Baggages,
    Baggage,
    MasterData,
    Unrecognized,
}

#[derive(Debug)]
pub enum IngestOutcome {
    Ingested {
        sha256: String,
        kind: FileKind,
        unknown: usize,
        conflicts: Vec<IdConflict>,
        /// Translation rows this file's ingest pass actually wrote (R3).
        translations: TranslationCounts,
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
    let parsed: Option<i64> = conn
        .query_row(
            "SELECT 1 FROM source_parse_evidence WHERE sha256 = ?1",
            [&sha256],
            |r| r.get(0),
        )
        .optional()?;
    if parsed.is_some() {
        return Ok(IngestOutcome::Skipped { sha256 });
    }
    let tx = conn.unchecked_transaction()?;
    let outcome = ingest_file_in_transaction(&tx, source_path, bytes, false)?;
    tx.commit()?;
    Ok(outcome)
}

pub(crate) fn ingest_file_in_transaction(
    conn: &Connection,
    source_path: &str,
    bytes: &[u8],
    parse_existing: bool,
) -> Result<IngestOutcome, ProductDbError> {
    let sha256 = sha256_hex(bytes);
    let parsed: Option<i64> = conn
        .query_row(
            "SELECT 1 FROM source_parse_evidence WHERE sha256 = ?1",
            [&sha256],
            |r| r.get(0),
        )
        .optional()?;
    if !parse_existing && parsed.is_some() {
        return Ok(IngestOutcome::Skipped { sha256 });
    }

    let manufacturer_id = source_path
        .split('/')
        .next()
        .filter(|top| top.starts_with("M-"))
        .map(str::to_string);
    store_source_file(
        conn,
        &SourceFile {
            source_path: source_path.to_string(),
            manufacturer_id,
            bytes: bytes.to_vec(),
        },
    )?;

    let kind = classify(bytes);
    let (unknown, conflicts, translations) = match kind {
        FileKind::Catalog => {
            let out = catalog::ingest_catalog(conn, &sha256, source_path, bytes)?;
            // A second pass over the same bytes, in the same transaction:
            // `Catalog.xml`'s own `Languages` block is not read by
            // `ingest_catalog` at all.
            let catalog = ingest_translations(conn, TranslationScope::Catalog, source_path, bytes)?;
            (
                out.unknown,
                out.conflicts,
                TranslationCounts {
                    catalog,
                    ..Default::default()
                },
            )
        }
        FileKind::Hardware => {
            let out = hardware::ingest_hardware(conn, &sha256, source_path, bytes)?;
            // Same second pass as `Catalog` above, for `Hardware.xml`'s own
            // `Languages` block.
            let hardware =
                ingest_translations(conn, TranslationScope::Hardware, source_path, bytes)?;
            (
                out.unknown,
                out.conflicts,
                TranslationCounts {
                    hardware,
                    ..Default::default()
                },
            )
        }
        FileKind::ApplicationProgram => {
            let out = program::ingest_program(conn, &sha256, source_path, bytes)?;
            // A second pass over the same bytes, in the same transaction:
            // the `Static` pass above still skips `Dynamic` outright (its
            // own doc comment says so); this is what actually reads it.
            // Neither `Dynamic` nor `Languages` block live there, so it
            // contributes no translations of its own.
            let dyn_out = dynamic::parse::parse_dynamic_trees(conn, &sha256, source_path, bytes)?;
            let mut unknown = out.unknown;
            unknown.extend(dyn_out.unknown);
            (
                unknown,
                out.conflicts,
                TranslationCounts {
                    program: out.translations,
                    ..Default::default()
                },
            )
        }
        // Baggages.xml lists the blobs; the blobs themselves and anything
        // unrecognized are stored and not parsed. `knx_master.xml` is
        // ingested through `ingest_master_data` instead (its three call
        // sites — `package.rs`, `knx-app`'s importer, `knx-cli` — stay
        // unchanged), so a `MasterData` blob reaching this generic path is
        // stored, not parsed, exactly like `Unrecognized`.
        FileKind::Baggages | FileKind::Baggage | FileKind::MasterData | FileKind::Unrecognized => {
            (Vec::new(), Vec::new(), TranslationCounts::default())
        }
    };

    insert_unknown(conn, &sha256, &unknown)?;
    insert_conflicts(conn, &conflicts)?;
    conn.execute(
        "INSERT OR IGNORE INTO source_parse_evidence (sha256) VALUES (?1)",
        [&sha256],
    )?;

    Ok(IngestOutcome::Ingested {
        sha256,
        kind,
        unknown: unknown.len(),
        conflicts,
        translations,
    })
}

/// Classifies by the first recognized element inside `ManufacturerData` (or,
/// for `knx_master.xml`, `MasterData` itself), not by file name: the name is
/// a convention, the content is the fact. A `Baggages/` blob is not XML at
/// all, so it is recognized by its bytes.
pub(crate) fn classify(bytes: &[u8]) -> FileKind {
    if bytes.is_empty() {
        return FileKind::Unrecognized;
    }
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
                    "MasterData" => return FileKind::MasterData,
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

    const MASTER: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<KNX xmlns="http://knx.org/xml/project/11">
  <MasterData>
    <Manufacturers>
      <Manufacturer Id="M-0001" Name="Siemens" />
    </Manufacturers>
  </MasterData>
</KNX>"#;

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
    fn a_master_file_is_classified_by_its_content() {
        let (_dir, conn) = db();
        let out = ingest_file(&conn, "knx_master.xml", MASTER.as_bytes()).unwrap();
        assert!(matches!(
            out,
            IngestOutcome::Ingested {
                kind: FileKind::MasterData,
                ..
            }
        ));
        // `ingest_file` stores the blob but does not parse it — that is
        // `ingest_master_data`'s job, called separately by every one of its
        // three call sites. No `manufacturer` row appears from this path.
        let manufacturers: i64 = conn
            .query_row("SELECT count(*) FROM manufacturer", [], |r| r.get(0))
            .unwrap();
        assert_eq!(manufacturers, 0);
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
