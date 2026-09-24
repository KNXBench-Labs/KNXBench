//! Scheme-13 package acceptance stays exact, atomic, and loss-accounted.

use std::collections::BTreeMap;
use std::io::{Cursor, Write};

use knx_productdb::{install_package, open_and_migrate, PackageError};
use rusqlite::Connection;
use zip::write::SimpleFileOptions;

const MASTER_13: &[u8] = br#"<KNX xmlns="http://knx.org/xml/project/13"><MasterData><Manufacturers><Manufacturer Id="M-0001" Name="Example"/></Manufacturers><DatapointTypes><DatapointType Id="DPT-1" Number="1" Name="one" Text="one"><DatapointSubtypes><DatapointSubtype Id="DPST-1-1" Number="1" Name="switch" Text="switch"/></DatapointSubtypes></DatapointType></DatapointTypes><Languages><Language Identifier="en-US"/></Languages><FutureSection Marker="retained"/></MasterData></KNX>"#;
const HARDWARE_13: &[u8] = br#"<KNX xmlns="http://knx.org/xml/project/13"><ManufacturerData><Manufacturer RefId="M-0001"><Hardware><Hardware Id="H-13" Name="Example"><Products><Product Id="P-13" Text="Example"/></Products></Hardware></Hardware></Manufacturer></ManufacturerData></KNX>"#;
const PROGRAM_13: &[u8] = br#"<KNX xmlns="http://knx.org/xml/project/13"><ManufacturerData><Manufacturer RefId="M-0001"><ApplicationPrograms><ApplicationProgram Id="A-13" Name="Program"><Static><Parameters><Parameter Id="PAR-13"/></Parameters><ComObjectTable><ComObject Id="O-13" Number="1"/></ComObjectTable><ComObjectRefs><ComObjectRef Id="OR-13" RefId="O-13"/></ComObjectRefs><Mystery Marker="retained"/></Static><Dynamic><Channel Id="CH-13"/></Dynamic></ApplicationProgram></ApplicationPrograms></Manufacturer></ManufacturerData></KNX>"#;

fn db() -> (tempfile::TempDir, Connection) {
    let directory = tempfile::tempdir().unwrap();
    let connection = open_and_migrate(&directory.path().join("products.sqlite")).unwrap();
    (directory, connection)
}

fn archive(members: &[(&str, &[u8])]) -> Vec<u8> {
    let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for (path, bytes) in members {
        writer
            .start_file(*path, SimpleFileOptions::default())
            .unwrap();
        writer.write_all(bytes).unwrap();
    }
    writer.finish().unwrap().into_inner()
}

fn database_counts(connection: &Connection) -> BTreeMap<String, i64> {
    let tables = connection
        .prepare(
            "SELECT name FROM sqlite_schema
             WHERE type = 'table' AND name NOT LIKE 'sqlite_%'
             ORDER BY name",
        )
        .unwrap()
        .query_map([], |row| row.get::<_, String>(0))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    tables
        .into_iter()
        .map(|table| {
            let quoted = table.replace('"', "\"\"");
            let count = connection
                .query_row(&format!("SELECT count(*) FROM \"{quoted}\""), [], |row| {
                    row.get(0)
                })
                .unwrap();
            (table, count)
        })
        .collect()
}

#[test]
fn scheme_13_installs_representative_rows_and_replays_persisted_evidence() {
    let (_directory, connection) = db();
    let bytes = archive(&[
        ("knx_master.xml", MASTER_13),
        ("M-0001/Hardware.xml", HARDWARE_13),
        ("M-0001/A.xml", PROGRAM_13),
        ("M-0001/Baggages/vendor.bin", b"opaque scheme 13 payload"),
    ]);

    let first = install_package(&connection, "scheme-13.knxprod", &bytes).unwrap();
    assert_eq!(first.scheme, 13);
    assert!(!first.skipped);
    assert!(first.unknown > 0);
    assert_eq!(
        connection
            .query_row(
                "SELECT count(*) FROM manufacturer WHERE id = 'M-0001'",
                [],
                |row| row.get::<_, i64>(0),
            )
            .unwrap(),
        1
    );
    for (table, id) in [
        ("product", "P-13"),
        ("application_program", "A-13"),
        ("parameter", "PAR-13"),
        ("com_object", "O-13"),
        ("com_object_ref", "OR-13"),
        ("datapoint_type", "DPT-1"),
        ("datapoint_type", "DPST-1-1"),
    ] {
        let actual: i64 = connection
            .query_row(
                &format!("SELECT count(*) FROM {table} WHERE id = ?1"),
                [id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(actual, 1, "{table}/{id}");
    }
    assert_eq!(
        connection
            .query_row(
                "SELECT count(*) FROM dynamic_node
                 WHERE program_id = 'A-13' AND kind = 'Channel' AND element_id = 'CH-13'",
                [],
                |row| row.get::<_, i64>(0),
            )
            .unwrap(),
        1
    );
    assert_eq!(
        connection
            .query_row(
                "SELECT bytes FROM source_file WHERE source_path = 'M-0001/Baggages/vendor.bin'",
                [],
                |row| row.get::<_, Vec<u8>>(0),
            )
            .unwrap(),
        b"opaque scheme 13 payload"
    );
    let first_facts = first.facts.clone().expect("measured scheme-13 evidence");
    assert!(!first_facts.unknown_constructs.is_empty());

    let retry = install_package(&connection, "renamed.knxprod", &bytes).unwrap();
    assert!(retry.skipped);
    assert_eq!(retry.scheme, 13);
    assert_eq!(retry.facts, Some(first_facts));
}

#[test]
fn malformed_scheme_13_package_publishes_no_rows() {
    let (_directory, connection) = db();
    let before = database_counts(&connection);
    let bytes = archive(&[
        ("knx_master.xml", MASTER_13),
        ("M-0001/Hardware.xml", HARDWARE_13),
        (
            "M-0001/Broken.xml",
            br#"<KNX xmlns="http://knx.org/xml/project/13"><ManufacturerData><ApplicationPrograms><ApplicationProgram Id="broken">"#,
        ),
    ]);

    assert!(matches!(
        install_package(&connection, "broken-13.knxprod", &bytes),
        Err(PackageError::Database(_))
    ));
    assert_eq!(database_counts(&connection), before);
}

#[test]
fn late_scheme_13_database_failure_rolls_back_prior_member_writes() {
    let (_directory, connection) = db();
    connection
        .execute_batch(
            "CREATE TRIGGER force_late_scheme_13_failure
             BEFORE INSERT ON application_program
             WHEN NEW.id = 'A-13'
             BEGIN
                 SELECT RAISE(ABORT, 'forced late scheme-13 failure');
             END;",
        )
        .unwrap();
    let before = database_counts(&connection);
    let bytes = archive(&[
        ("knx_master.xml", MASTER_13),
        ("M-0001/Hardware.xml", HARDWARE_13),
        ("M-0001/A.xml", PROGRAM_13),
    ]);

    assert!(matches!(
        install_package(&connection, "late-failure-13.knxprod", &bytes),
        Err(PackageError::Database(_))
    ));
    assert_eq!(database_counts(&connection), before);
}

#[test]
fn lookalike_and_unapproved_namespaces_remain_rejected_without_rows() {
    let (_directory, connection) = db();
    let before = database_counts(&connection);
    for namespace in [
        "http://knx.org/xml/project/130",
        "https://knx.org/xml/project/13",
        "http://knx.org/xml/project/14",
    ] {
        let master = format!("<KNX xmlns=\"{namespace}\"><MasterData/></KNX>");
        let result = install_package(
            &connection,
            "unsupported.knxprod",
            &archive(&[("knx_master.xml", master.as_bytes())]),
        );
        assert!(matches!(
            result,
            Err(PackageError::UnsupportedNamespace { namespace: actual }) if actual == namespace
        ));
        assert_eq!(database_counts(&connection), before);
    }
}
