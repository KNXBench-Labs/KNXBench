//! Synthetic exact-scheme-23 contracts; no manufacturer/runtime compatibility claim.
use std::io::{Cursor, Write};

use knx_productdb::report::UnknownKind;
use knx_productdb::{install_package, open_and_migrate, PackageError, ProductDbError};
use rusqlite::{types::Value, Connection};
use zip::write::SimpleFileOptions;

const MASTER: &[u8] = br#"<KNX xmlns="http://knx.org/xml/project/23"><MasterData><Manufacturers><Manufacturer Id="M-0023" Name="Example"/></Manufacturers><DatapointTypes><DatapointType Id="D-23" Number="23" VariableLength="true"/></DatapointTypes><Resources><Resource Id="R-23" Optional="opaque"/></Resources></MasterData></KNX>"#;
const HARDWARE: &[u8] = br#"<KNX xmlns="http://knx.org/xml/project/23"><ManufacturerData><Manufacturer RefId="M-0023"><Hardware><Hardware Id="H-23" Name="Example"><Products><Product Id="P-23" Text="Product"/></Products><Hardware2Programs><Hardware2Program Id="H2P-23" CouplerCapabilities="opaque"/></Hardware2Programs></Hardware></Hardware></Manufacturer></ManufacturerData></KNX>"#;
const PROGRAM: &[u8] = br#"<KNX xmlns="http://knx.org/xml/project/23"><ManufacturerData><Manufacturer RefId="M-0023"><ApplicationPrograms><ApplicationProgram Id="A-23" Name="Program" HardwareType="opaque"><Static><LoadProcedures><LoadProcedure><LdCtrlDeclarePropDesc ObjIdx="1"/></LoadProcedure></LoadProcedures></Static></ApplicationProgram></ApplicationPrograms></Manufacturer></ManufacturerData></KNX>"#;

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

fn package() -> Vec<u8> {
    archive(&[
        ("knx_master.xml", MASTER),
        ("M-0023/Hardware.xml", HARDWARE),
        ("M-0023/Application.xml", PROGRAM),
    ])
}

fn contents(connection: &Connection) -> Vec<(String, Vec<Vec<Value>>)> {
    let tables = connection
        .prepare("SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name")
        .unwrap()
        .query_map([], |row| row.get::<_, String>(0))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    tables
        .into_iter()
        .map(|table| {
            let query = format!("SELECT * FROM \"{}\"", table.replace('"', "\"\""));
            let statement = connection.prepare(&query).unwrap();
            let width = statement.column_count();
            let ordering = (1..=width)
                .map(|index| index.to_string())
                .collect::<Vec<_>>()
                .join(",");
            let mut statement = connection
                .prepare(&format!("{query} ORDER BY {ordering}"))
                .unwrap();
            let rows = statement
                .query_map([], |row| {
                    (0..width)
                        .map(|index| row.get::<_, Value>(index))
                        .collect::<Result<Vec<_>, _>>()
                })
                .unwrap()
                .collect::<Result<Vec<_>, _>>()
                .unwrap();
            (table, rows)
        })
        .collect()
}

fn seed(connection: &Connection) {
    let master = String::from_utf8(MASTER.to_vec())
        .unwrap()
        .replace("project/23", "project/21");
    let hardware = String::from_utf8(HARDWARE.to_vec())
        .unwrap()
        .replace("project/23", "project/21");
    let program = String::from_utf8(PROGRAM.to_vec())
        .unwrap()
        .replace("project/23", "project/21");
    let bytes = archive(&[
        ("knx_master.xml", master.as_bytes()),
        ("M-0023/Hardware.xml", hardware.as_bytes()),
        ("M-0023/Application.xml", program.as_bytes()),
    ]);
    let report = install_package(connection, "seed-21.knxprod", &bytes).unwrap();
    assert_eq!(report.scheme, 21);
    let retained: Vec<u8> = connection
        .query_row(
            "SELECT bytes FROM package WHERE sha256=?1",
            [&report.sha256],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(
        retained, bytes,
        "seed must be persisted before refusal snapshot"
    );
}

#[test]
fn exact_scheme23_installs_known_subset_with_queryable_rows() {
    let (_directory, connection) = db();
    let report = install_package(&connection, "synthetic-23.knxprod", &package())
        .expect("exact scheme23 known subset must install");
    assert_eq!(report.scheme, 23);
    for (table, id) in [("hardware", "H-23"), ("application_program", "A-23")] {
        let count: i64 = connection
            .query_row(
                &format!("SELECT count(*) FROM {table} WHERE id=?1"),
                [id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 1, "known subset must be queryable in {table}");
    }
}

#[test]
fn scheme23_retains_exact_archive_and_member_bytes_on_replay() {
    let (_directory, connection) = db();
    let bytes = package();
    let first = install_package(&connection, "first-23.knxprod", &bytes)
        .expect("scheme23 retained archive install");
    let stored: Vec<u8> = connection
        .query_row(
            "SELECT bytes FROM package WHERE sha256=?1",
            [&first.sha256],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(stored, bytes);
    for (path, original) in [
        ("knx_master.xml", MASTER),
        ("M-0023/Hardware.xml", HARDWARE),
        ("M-0023/Application.xml", PROGRAM),
    ] {
        let stored: Vec<u8> = connection
            .query_row(
                "SELECT bytes FROM source_file WHERE source_path=?1",
                [path],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(stored, original);
    }
    let before = contents(&connection);
    let replay = install_package(&connection, "first-23.knxprod", &bytes).unwrap();
    assert!(replay.skipped);
    assert_eq!(replay.scheme, 23);
    assert_eq!(first.facts, replay.facts);
    assert_eq!(first.unknown, replay.unknown);
    assert_eq!(contents(&connection), before);
}

#[test]
fn scheme23_keeps_existing_opaque_field_evidence_explicit() {
    let (_directory, connection) = db();
    let report = install_package(&connection, "opaque-23.knxprod", &package())
        .expect("scheme23 evidence install");
    let facts = report.facts.expect("persisted evidence");
    for (name, expected_xpath, sample) in [
        ("HardwareType", "/KNX/ManufacturerData/Manufacturer/ApplicationPrograms/ApplicationProgram", "opaque"),
        ("CouplerCapabilities", "/KNX/ManufacturerData/Manufacturer/Hardware/Hardware/Hardware2Programs/Hardware2Program", "opaque"),
        ("VariableLength", "/KNX/MasterData/DatapointTypes/DatapointType", "true"),
        ("Optional", "/KNX/MasterData/Resources/Resource", "opaque"),
        ("ObjIdx", "/KNX/ManufacturerData/Manufacturer/ApplicationPrograms/ApplicationProgram/Static/LoadProcedures/LoadProcedure/LdCtrlDeclarePropDesc", "1"),
    ] {
        let rows: Vec<_> = facts.unknown_constructs.iter().filter(|row| {
            row.kind == UnknownKind::Attribute && row.name == name && row.xpath == expected_xpath
        }).collect();
        assert_eq!(rows.len(), 1, "missing exact scheme23 opaque {name} evidence at {expected_xpath}");
        assert_eq!(rows[0].sample.as_deref(), Some(sample));
        assert_eq!(rows[0].occurrences, 1);
    }
    assert!(facts
        .unknown_constructs
        .iter()
        .any(|row| row.kind == UnknownKind::Element && row.name == "LdCtrlDeclarePropDesc"));
}

#[test]
fn scheme23_foreign_and_qualified_lookalikes_preserve_nonempty_database() {
    for (path, xml, cause) in [
        ("M-0023/Hardware.xml", br#"<KNX xmlns="http://knx.org/xml/project/23" xmlns:e="urn:extension"><ManufacturerData><Manufacturer RefId="M-0023"><Hardware><e:Hardware Id="H-23"/></Hardware></Manufacturer></ManufacturerData></KNX>"#.as_slice(), "scheme-23 XML contains a non-KNX element namespace"),
        ("M-0023/Application.xml", br#"<KNX xmlns="http://knx.org/xml/project/21"><ManufacturerData><Manufacturer RefId="M-0023"><ApplicationPrograms><ApplicationProgram Id="A-23"/></ApplicationPrograms></Manufacturer></ManufacturerData></KNX>"#, "scheme-23 XML contains a non-KNX element namespace"),
        ("M-0023/Hardware.xml", br#"<KNX xmlns="http://knx.org/xml/project/23" xmlns:e="urn:extension"><ManufacturerData><Manufacturer RefId="M-0023"><Hardware><Hardware Id="H-23" e:Name="foreign"/></Hardware></Manufacturer></ManufacturerData></KNX>"#, "scheme-23 XML contains a qualified attribute"),
        ("M-0023/Catalog.xml", br#"<KNX xmlns="http://knx.org/xml/project/23" xmlns:e="urn:extension"><ManufacturerData><Manufacturer RefId="M-0023"><Catalog><e:CatalogItem/></Catalog></Manufacturer></ManufacturerData></KNX>"#, "scheme-23 XML contains a non-KNX element namespace"),
        ("M-0023/Baggages.xml", br#"<KNX xmlns="http://knx.org/xml/project/23" xmlns:e="urn:extension"><ManufacturerData><Manufacturer RefId="M-0023"><Baggages><e:Baggage/></Baggages></Manufacturer></ManufacturerData></KNX>"#, "scheme-23 XML contains a non-KNX element namespace"),
    ] {
        let (_directory, connection) = db();
        seed(&connection);
        let before = contents(&connection);
        let bytes = archive(&[("knx_master.xml", MASTER), (path, xml)]);
        let error = install_package(&connection, "foreign-23.knxprod", &bytes).unwrap_err();
        assert!(matches!(error, PackageError::Database(ProductDbError::Xml { cause: ref actual, .. }) if actual.contains(cause)), "namespace boundary must reach member validation: {error}");
        assert!(error.to_string().contains(cause), "unexpected namespace refusal: {error}");
        assert_eq!(contents(&connection), before, "refusal must preserve every persisted seed row");
    }
}

#[test]
fn scheme23_late_evidence_failure_preserves_nonempty_database() {
    let (_directory, connection) = db();
    seed(&connection);
    let before = contents(&connection);
    let program = format!("<KNX xmlns=\"http://knx.org/xml/project/23\"><ManufacturerData><Manufacturer RefId=\"M-0023\"><ApplicationPrograms><ApplicationProgram Id=\"A-23\"><Static>{}<Property Occurrence=\"1\"/>{}</Static></ApplicationProgram></ApplicationPrograms></Manufacturer></ManufacturerData></KNX>", "<Wrapper>".repeat(1025), "</Wrapper>".repeat(1025));
    let bytes = archive(&[
        ("knx_master.xml", MASTER),
        ("M-0023/Hardware.xml", HARDWARE),
        ("M-0023/Deep.xml", program.as_bytes()),
    ]);
    let error = install_package(&connection, "deep-23.knxprod", &bytes).unwrap_err();
    assert!(
        matches!(error, PackageError::Database(ProductDbError::Xml { ref source_path, ref cause }) if source_path == "M-0023/Deep.xml" && cause.contains("XML nesting exceeds evidence limit 1024")),
        "expected late evidence refusal: {error}"
    );
    assert_eq!(contents(&connection), before);
}

#[test]
fn scheme23_admission_does_not_admit_unresearched_master_namespaces() {
    for namespace in [
        "http://knx.org/xml/project/10",
        "http://knx.org/xml/project/22",
        "http://knx.org/xml/project/24",
        "urn:extension",
    ] {
        let (_directory, connection) = db();
        seed(&connection);
        let before = contents(&connection);
        let master = format!("<KNX xmlns=\"{namespace}\"><MasterData/></KNX>");
        let bytes = archive(&[
            ("knx_master.xml", master.as_bytes()),
            ("M-0023/Hardware.xml", HARDWARE),
        ]);
        let error = install_package(&connection, "unresearched.knxprod", &bytes).unwrap_err();
        assert!(
            matches!(error, PackageError::UnsupportedNamespace { namespace: ref actual } if actual == namespace)
        );
        assert_eq!(contents(&connection), before);
    }
}
