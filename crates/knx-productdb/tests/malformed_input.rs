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

const HARDWARE_WITH_H2P: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<KNX xmlns="http://knx.org/xml/project/11"><ManufacturerData><Manufacturer RefId="M-0001">
<Hardware><Hardware Id="H-1" Name="First hardware"><Products>
<Product Id="P-1" Text="First product" OrderNumber="N-1" />
</Products><Hardware2Programs><Hardware2Program Id="HP-1" MediumTypes="MT-0">
<ApplicationProgramRef RefId="A" /><RegistrationInfo RegistrationStatus="First" />
</Hardware2Program></Hardware2Programs></Hardware></Hardware>
</Manufacturer></ManufacturerData></KNX>"#;

const CATALOG_WITH_ITEM: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<KNX xmlns="http://knx.org/xml/project/11"><ManufacturerData><Manufacturer RefId="M-0001">
<Catalog><CatalogSection Id="CS-1" Name="Section"><CatalogItem Id="CI-1" Name="First item"
ProductRefId="P-1" Hardware2ProgramRefId="HP-1" /></CatalogSection></Catalog>
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

#[test]
fn conflicting_hardware2program_keeps_first_winner_provenance_and_records_every_collision() {
    let (_dir, conn) = db();
    let first_hardware =
        knx_productdb::ingest_file(&conn, "M-0001/Hardware.xml", HARDWARE_WITH_H2P.as_bytes())
            .unwrap();
    let first_hardware_sha = match first_hardware {
        knx_productdb::IngestOutcome::Ingested { sha256, .. } => sha256,
        other => panic!("expected ingested hardware, got {other:?}"),
    };
    let changed_hardware = HARDWARE_WITH_H2P
        .replace("First hardware", "Second hardware")
        .replace("First product", "Second product")
        .replace("RefId=\"A\"", "RefId=\"B\"");
    let second_hardware =
        knx_productdb::ingest_file(&conn, "M-0001/Hardware-2.xml", changed_hardware.as_bytes())
            .unwrap();
    let second_hardware_sha = match second_hardware {
        knx_productdb::IngestOutcome::Ingested {
            sha256, conflicts, ..
        } => {
            assert!(conflicts.iter().any(|conflict| {
                conflict.table == "hardware"
                    && conflict.id == "H-1"
                    && conflict.kept_sha256 == first_hardware_sha
            }));
            assert!(conflicts.iter().any(|conflict| {
                conflict.table == "product"
                    && conflict.id == "P-1"
                    && conflict.kept_sha256 == first_hardware_sha
            }));
            assert!(conflicts.iter().any(|conflict| {
                conflict.table == "hardware2program"
                    && conflict.id == "HP-1"
                    && conflict.kept_sha256 == first_hardware_sha
            }));
            sha256
        }
        other => panic!("expected ingested hardware, got {other:?}"),
    };

    let (hardware_name, hardware_sha): (String, String) = conn
        .query_row(
            "SELECT name, source_sha256 FROM hardware WHERE id = 'H-1'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert_eq!(
        (hardware_name, hardware_sha),
        ("First hardware".into(), first_hardware_sha.clone())
    );
    let (product_text, product_sha): (String, String) = conn
        .query_row(
            "SELECT text, source_sha256 FROM product WHERE id = 'P-1'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert_eq!(
        (product_text, product_sha),
        ("First product".into(), first_hardware_sha.clone())
    );
    let (program_ref, h2p_sha): (String, String) = conn
        .query_row(
            "SELECT application_program_ref, source_sha256 FROM hardware2program WHERE id = 'HP-1'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert_eq!(
        (program_ref, h2p_sha),
        ("A".into(), first_hardware_sha.clone())
    );
    assert_eq!(
        knx_productdb::load_source_file(&conn, &first_hardware_sha).unwrap(),
        Some(HARDWARE_WITH_H2P.as_bytes().to_vec())
    );
    assert_eq!(
        knx_productdb::load_source_file(&conn, &second_hardware_sha).unwrap(),
        Some(changed_hardware.as_bytes().to_vec())
    );

    let first_catalog =
        knx_productdb::ingest_file(&conn, "M-0001/Catalog.xml", CATALOG_WITH_ITEM.as_bytes())
            .unwrap();
    let first_catalog_sha = match first_catalog {
        knx_productdb::IngestOutcome::Ingested { sha256, .. } => sha256,
        other => panic!("expected ingested catalog, got {other:?}"),
    };
    let changed_catalog = CATALOG_WITH_ITEM.replace("First item", "Second item");
    let second_catalog =
        knx_productdb::ingest_file(&conn, "M-0001/Catalog-2.xml", changed_catalog.as_bytes())
            .unwrap();
    let second_catalog_sha = match second_catalog {
        knx_productdb::IngestOutcome::Ingested {
            sha256, conflicts, ..
        } => {
            assert!(conflicts.iter().any(|conflict| {
                conflict.table == "catalog_section"
                    && conflict.id == "CS-1"
                    && conflict.kept_sha256 == first_catalog_sha
            }));
            assert!(conflicts.iter().any(|conflict| {
                conflict.table == "catalog_item"
                    && conflict.id == "CI-1"
                    && conflict.kept_sha256 == first_catalog_sha
            }));
            sha256
        }
        other => panic!("expected ingested catalog, got {other:?}"),
    };
    let (item_name, item_sha): (String, String) = conn
        .query_row(
            "SELECT name, source_sha256 FROM catalog_item WHERE id = 'CI-1'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert_eq!(
        (item_name, item_sha),
        ("First item".into(), first_catalog_sha.clone())
    );
    assert_eq!(
        knx_productdb::load_source_file(&conn, &first_catalog_sha).unwrap(),
        Some(CATALOG_WITH_ITEM.as_bytes().to_vec())
    );
    assert_eq!(
        knx_productdb::load_source_file(&conn, &second_catalog_sha).unwrap(),
        Some(changed_catalog.as_bytes().to_vec())
    );

    let conflicts: i64 = conn
        .query_row(
            "SELECT count(*) FROM ingest_unknown WHERE kind = 'IdConflict'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(conflicts, 5);
}
