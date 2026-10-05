//! Scheme-10 (ETS4-era) packages install through the strict exact-namespace path (ADR-0083).

use std::collections::BTreeMap;
use std::io::{Cursor, Write};

use knx_productdb::{install_package, open_and_migrate, PackageError, ProductDbError};
use rusqlite::Connection;
use zip::write::SimpleFileOptions;

const NS: &str = "http://knx.org/xml/project/10";
const MASTER_10: &[u8] = br#"<KNX xmlns="http://knx.org/xml/project/10"><MasterData><Manufacturers><Manufacturer Id="M-0010" Name="Example"/></Manufacturers><DatapointTypes><DatapointType Id="DPT-1" Number="1" Name="one" Text="one"><DatapointSubtypes><DatapointSubtype Id="DPST-1-1" Number="1" Name="switch" Text="switch"/></DatapointSubtypes></DatapointType></DatapointTypes><Languages><Language Identifier="de-DE"><TranslationUnit RefId="DPST-1-1"><TranslationElement RefId="DPST-1-1"><Translation AttributeName="Text" Text="Schalten"/></TranslationElement></TranslationUnit></Language></Languages></MasterData></KNX>"#;
const HARDWARE_10: &[u8] = br#"<KNX xmlns="http://knx.org/xml/project/10"><ManufacturerData><Manufacturer RefId="M-0010"><Hardware><Hardware Id="M-0010_H-1" Name="Example"><Products><Product Id="M-0010_H-1_P-1" Text="Example"/></Products><Hardware2Programs><Hardware2Program Id="M-0010_H-1_HP-1"><ApplicationProgramRef RefId="M-0010_A-1"/></Hardware2Program></Hardware2Programs></Hardware></Hardware></Manufacturer></ManufacturerData></KNX>"#;
const PROGRAM_10: &[u8] = br#"<KNX xmlns="http://knx.org/xml/project/10"><ManufacturerData><Manufacturer RefId="M-0010"><ApplicationPrograms><ApplicationProgram Id="M-0010_A-1" Name="Program" ApplicationNumber="1" ApplicationVersion="1" MaskVersion="MV-0701"><Static><Parameters><Parameter Id="M-0010_A-1_P-1" Name="P" Text="Delay"/></Parameters><ComObjectTable><ComObject Id="M-0010_A-1_O-1" Number="1" Text="Switch" ObjectSize="1 Bit"/></ComObjectTable><ComObjectRefs><ComObjectRef Id="M-0010_A-1_O-1_R-1" RefId="M-0010_A-1_O-1"/></ComObjectRefs><Mystery Marker="retained"/></Static><Dynamic><Channel Id="M-0010_A-1_CH-1"/></Dynamic></ApplicationProgram></ApplicationPrograms></Manufacturer></ManufacturerData></KNX>"#;

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
        ("knx_master.xml", MASTER_10),
        ("M-0010/Hardware.xml", HARDWARE_10),
        ("M-0010/M-0010_A-1.xml", PROGRAM_10),
    ])
}

fn database_counts(connection: &Connection) -> BTreeMap<String, i64> {
    let tables = connection
        .prepare("SELECT name FROM sqlite_schema WHERE type = 'table' AND name NOT LIKE 'sqlite_%' ORDER BY name")
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

fn count(connection: &Connection, sql: &str) -> i64 {
    connection.query_row(sql, [], |row| row.get(0)).unwrap()
}

#[test]
fn scheme_10_installs_queryable_rows_reports_unknowns_and_replays() {
    let (_directory, connection) = db();
    let bytes = package();
    let first = install_package(&connection, "scheme-10.knxprod", &bytes).unwrap();
    assert_eq!(first.scheme, 10);
    assert!(!first.skipped);
    assert!(
        first.unknown > 0,
        "the unmodelled Mystery element is reported"
    );
    for (table, id) in [
        ("manufacturer", "M-0010"),
        ("product", "M-0010_H-1_P-1"),
        ("hardware2program", "M-0010_H-1_HP-1"),
        ("application_program", "M-0010_A-1"),
        ("parameter", "M-0010_A-1_P-1"),
        ("com_object", "M-0010_A-1_O-1"),
        ("com_object_ref", "M-0010_A-1_O-1_R-1"),
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
        count(&connection, "SELECT count(*) FROM dynamic_node WHERE program_id = 'M-0010_A-1' AND kind = 'Channel'"),
        1
    );
    assert_eq!(
        count(&connection, "SELECT count(*) FROM translation WHERE scope = 'Master' AND ref_id = 'DPST-1-1' AND language = 'de-DE'"),
        1,
        "master-scope translations of a scheme-10 master are read"
    );
    assert_eq!(first.translations.master, 1);
    // The master-language evidence pass treats scheme 10 like 11: consumed
    // fields (Identifier, RefId, AttributeName, Text) stay quiet, the
    // unconsumed TranslationUnit@RefId is reported.
    let mut languages = connection
        .prepare("SELECT xpath, name FROM ingest_unknown WHERE xpath LIKE '/KNX/MasterData/Languages%' ORDER BY xpath, name")
        .unwrap()
        .query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    languages.dedup();
    assert_eq!(
        languages,
        vec![(
            "/KNX/MasterData/Languages/Language/TranslationUnit".to_string(),
            "RefId".to_string()
        )]
    );
    assert_eq!(
        knx_productdb::query::resolve_program(&connection, "M-0010_H-1_HP-1")
            .unwrap()
            .as_deref(),
        Some("M-0010_A-1")
    );
    let retry = install_package(&connection, "renamed.knxprod", &bytes).unwrap();
    assert!(retry.skipped);
    assert_eq!(retry.scheme, 10);
    assert_eq!(retry.facts, first.facts);
}

#[test]
fn scheme_10_foreign_and_qualified_content_is_refused_without_rows() {
    for (path, xml, cause) in [
        ("M-0010/Hardware.xml", format!(r#"<KNX xmlns="{NS}" xmlns:e="urn:extension"><ManufacturerData><Manufacturer RefId="M-0010"><Hardware><e:Hardware Id="H-1"/></Hardware></Manufacturer></ManufacturerData></KNX>"#), "scheme-10 XML contains a non-KNX element namespace"),
        ("M-0010/Hardware.xml", format!(r#"<KNX xmlns="{NS}" xmlns:e="urn:extension"><ManufacturerData><Manufacturer RefId="M-0010"><Hardware><Hardware Id="H-1" e:Name="x"/></Hardware></Manufacturer></ManufacturerData></KNX>"#), "scheme-10 XML contains a qualified attribute"),
        ("M-0010/M-0010_A-1.xml", r#"<KNX xmlns="http://knx.org/xml/project/11"><ManufacturerData><Manufacturer RefId="M-0010"><ApplicationPrograms><ApplicationProgram Id="M-0010_A-1"/></ApplicationPrograms></Manufacturer></ManufacturerData></KNX>"#.to_string(), "scheme-10 XML contains a non-KNX element namespace"),
    ] {
        let (_directory, connection) = db();
        install_package(&connection, "seed.knxprod", &package()).unwrap();
        let before = database_counts(&connection);
        let bytes = archive(&[("knx_master.xml", MASTER_10), (path, xml.as_bytes())]);
        let error = install_package(&connection, "foreign-10.knxprod", &bytes).unwrap_err();
        assert!(
            matches!(error, PackageError::Database(ProductDbError::Xml { cause: ref actual, .. }) if actual.contains(cause)),
            "{error}"
        );
        assert_eq!(database_counts(&connection), before, "refusal keeps every row");
    }
}

#[test]
fn late_scheme_10_failure_rolls_back_prior_member_writes() {
    let (_directory, connection) = db();
    connection
        .execute_batch(
            "CREATE TRIGGER force_late_scheme_10_failure BEFORE INSERT ON application_program
             WHEN NEW.id = 'M-0010_A-1' BEGIN SELECT RAISE(ABORT, 'forced late scheme-10 failure'); END;",
        )
        .unwrap();
    let before = database_counts(&connection);
    assert!(matches!(
        install_package(&connection, "late-10.knxprod", &package()),
        Err(PackageError::Database(_))
    ));
    assert_eq!(database_counts(&connection), before);
}

#[test]
fn only_the_exact_scheme_10_namespace_is_admitted() {
    for namespace in [
        "http://knx.org/xml/project/1",
        "http://knx.org/xml/project/100",
        "https://knx.org/xml/project/10",
        "http://knx.org/xml/project/10/",
    ] {
        let (_directory, connection) = db();
        let before = database_counts(&connection);
        let master = format!("<KNX xmlns=\"{namespace}\"><MasterData/></KNX>");
        let error = install_package(
            &connection,
            "lookalike.knxprod",
            &archive(&[
                ("knx_master.xml", master.as_bytes()),
                ("M-0010/Hardware.xml", HARDWARE_10),
            ]),
        )
        .unwrap_err();
        assert!(
            matches!(error, PackageError::UnsupportedNamespace { namespace: ref actual } if actual == namespace),
            "{error}"
        );
        assert_eq!(database_counts(&connection), before);
    }
}
