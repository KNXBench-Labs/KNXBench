//! Malformed and hostile manufacturer input. None of it may panic, and
//! none of it may leave the database half-written.

fn db() -> (tempfile::TempDir, knx_productdb::Connection) {
    let dir = tempfile::tempdir().unwrap();
    let conn = knx_productdb::open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
    (dir, conn)
}

const PROGRAM: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<KNX xmlns="http://knx.org/xml/project/11"><ManufacturerData><Manufacturer RefId="M-0001">
<ApplicationPrograms><ApplicationProgram Id="A-1" Name="P" ApplicationVersion="1"
 MaskVersion="MV-0010"><Static><ComObjectTable>
<ComObject Id="A-1_O-0" Number="0" Text="T" ObjectSize="1 Bit" />
</ComObjectTable></Static></ApplicationProgram></ApplicationPrograms>
</Manufacturer></ManufacturerData></KNX>"#;

#[test]
fn a_truncated_program_leaves_nothing_behind() {
    let (_dir, conn) = db();
    assert!(knx_productdb::ingest_file(
        &conn,
        "M-0001/A.xml",
        &PROGRAM.as_bytes()[..PROGRAM.len() / 2]
    )
    .is_err());
    let rows: i64 = conn
        .query_row("SELECT count(*) FROM source_file", [], |r| r.get(0))
        .unwrap();
    assert_eq!(rows, 0);
}

#[test]
fn an_empty_file_is_stored_and_classified_as_unrecognized() {
    let (_dir, conn) = db();
    let out = knx_productdb::ingest_file(&conn, "M-0001/Empty.xml", b"").unwrap();
    assert!(matches!(
        out,
        knx_productdb::IngestOutcome::Ingested {
            kind: knx_productdb::FileKind::Unrecognized,
            ..
        }
    ));
}

#[test]
fn deeply_nested_xml_does_not_blow_the_stack() {
    // The parsers are iterative, not recursive; 10,000 levels is the same
    // depth knx-etsproj's own malformed suite uses.
    let (_dir, conn) = db();
    let mut xml = String::from(r#"<KNX xmlns="http://knx.org/xml/project/11"><ManufacturerData>"#);
    for _ in 0..10_000 {
        xml.push_str("<Nested>");
    }
    for _ in 0..10_000 {
        xml.push_str("</Nested>");
    }
    xml.push_str("</ManufacturerData></KNX>");
    let _ = knx_productdb::ingest_file(&conn, "M-0001/Deep.xml", xml.as_bytes());
}

#[test]
fn the_same_program_id_from_two_different_files_keeps_the_first_and_records_the_conflict() {
    let (_dir, conn) = db();
    knx_productdb::ingest_file(&conn, "M-0001/A.xml", PROGRAM.as_bytes()).unwrap();
    let changed = PROGRAM.replace("Name=\"P\"", "Name=\"P2\"");
    knx_productdb::ingest_file(&conn, "M-0001/A2.xml", changed.as_bytes()).unwrap();
    let name: String = conn
        .query_row(
            "SELECT name FROM application_program WHERE id = 'A-1'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(name, "P");
    let conflicts: i64 = conn
        .query_row(
            "SELECT count(*) FROM ingest_unknown WHERE kind = 'IdConflict'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(conflicts, 1);
}
