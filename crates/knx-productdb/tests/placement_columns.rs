//! Placements and enumeration attributes reach typed columns; the rest stays reported (v25).
mod v20_rewind;
mod v24_rewind;

use std::io::{Cursor, Write};

use knx_productdb::{install_package, open_and_migrate, CURRENT_PRODUCTDB_VERSION};
use rusqlite::types::Value;
use rusqlite::Connection;
use zip::write::SimpleFileOptions;

const MASTER: &[u8] = br#"<KNX xmlns="http://knx.org/xml/project/20"><MasterData><Manufacturers><Manufacturer Id="M-0001" Name="Example"/></Manufacturers></MasterData></KNX>"#;
const HARDWARE: &[u8] = br#"<KNX xmlns="http://knx.org/xml/project/20"><ManufacturerData><Manufacturer RefId="M-0001"><Hardware><Hardware Id="H-1" Name="Example"/></Hardware></Manufacturer></ManufacturerData></KNX>"#;

/// A memory-placed parameter, a hidden fixed value on a device property (the
/// census-3 G10 shape), a property placed by object type whose `PropertyId`
/// is not a number, two union members with their own offsets, and an
/// enumeration with every attribute the collections carry plus one nobody
/// reads.
pub const PROGRAM: &[u8] = br#"<KNX xmlns="http://knx.org/xml/project/20"><ManufacturerData><Manufacturer RefId="M-0001"><ApplicationPrograms><ApplicationProgram Id="A-1" Name="Program">
<Static>
<ParameterTypes>
<ParameterType Id="PT-Num" Name="Num"><TypeNumber SizeInBit="8" Type="unsignedInt" minInclusive="0" maxInclusive="255"/></ParameterType>
<ParameterType Id="PT-Bin" Name="Bin"><TypeRestriction Base="BinaryValue" SizeInBit="48">
<Enumeration Id="PT-Bin_EN-1" Text="hw" Value="1" BinaryValue="AAEAAgAD" Name="hwname" Icon="ic.png" PictureAlignment="Left" Fancy="yes"/>
</TypeRestriction></ParameterType>
</ParameterTypes>
<Parameters>
<Parameter Id="P-1" Name="Plain" ParameterType="PT-Num" Text="One" Value="1"><Memory CodeSegment="AS-1" Offset="2" BitOffset="0"/></Parameter>
<Parameter Id="P-2" Name="Hw" ParameterType="PT-Bin" Text="Hw" Value="1" Access="None"><Property ObjectIndex="0" PropertyId="78" Offset="0" BitOffset="0" Occurrence="0"/></Parameter>
<Parameter Id="P-3" Name="ByType" ParameterType="PT-Num" Text="T" Value="2"><Property ObjectType="9" PropertyId="x51" Offset="1" BitOffset="0"/></Parameter>
<Union SizeInBit="16"><Memory CodeSegment="AS-1" Offset="4" BitOffset="0"/>
<Parameter Id="UP-1" Name="Wide" ParameterType="PT-Num" Text="W" Offset="0" BitOffset="0" Value="1"/>
<Parameter Id="UP-2" Name="Narrow" ParameterType="PT-Num" Text="N" Offset="1" BitOffset="3" Value="2"/>
</Union>
</Parameters>
<ParameterRefs><ParameterRef Id="P-1_R-1" RefId="P-1"/></ParameterRefs>
</Static>
</ApplicationProgram></ApplicationPrograms></Manufacturer></ManufacturerData></KNX>"#;

pub const STATIC: &str =
    "/KNX/ManufacturerData/Manufacturer/ApplicationPrograms/ApplicationProgram/Static";

fn package() -> Vec<u8> {
    let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for (path, bytes) in [
        ("knx_master.xml", MASTER),
        ("M-0001/Hardware.xml", HARDWARE),
        ("M-0001/M-0001_A-1.xml", PROGRAM),
    ] {
        writer
            .start_file(path, SimpleFileOptions::default())
            .unwrap();
        writer.write_all(bytes).unwrap();
    }
    writer.finish().unwrap().into_inner()
}

fn installed() -> (tempfile::TempDir, std::path::PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("products.sqlite");
    let conn = open_and_migrate(&path).unwrap();
    install_package(&conn, "synthetic.knxprod", &package()).unwrap();
    drop(conn);
    (dir, path)
}

fn rows(conn: &Connection, sql: &str) -> Vec<Vec<String>> {
    let mut stmt = conn.prepare(sql).unwrap();
    let width = stmt.column_count();
    stmt.query_map([], |r| {
        (0..width)
            .map(|i| r.get::<_, Value>(i).map(|v| format!("{v:?}")))
            .collect::<Result<Vec<_>, _>>()
    })
    .unwrap()
    .collect::<Result<_, _>>()
    .unwrap()
}

/// Everything v25 touches, in a stable order.
fn snapshot(conn: &Connection) -> Vec<Vec<Vec<String>>> {
    [
        "PRAGMA user_version",
        "SELECT * FROM parameter ORDER BY program_id, id",
        "SELECT * FROM parameter_type_enum ORDER BY program_id, id",
        "SELECT source_sha256, xpath, kind, name, occurrences, sample FROM ingest_unknown
         ORDER BY source_sha256, xpath, kind, name",
        "SELECT sha256, unknown_count FROM package ORDER BY sha256",
        "SELECT * FROM package_install_unknown ORDER BY package_sha256, ordinal",
        "SELECT package_sha256, status, unknown_distinct, unknown_occurrences
         FROM package_install_report ORDER BY package_sha256",
        "SELECT * FROM package_install_count WHERE category = 'unknown_construct'
         ORDER BY package_sha256, disposition",
    ]
    .iter()
    .map(|sql| rows(conn, sql))
    .collect()
}

/// `(xpath suffix after Static, kind, name, occurrences, sample)` of every
/// unknown row below `Static`.
fn static_unknowns(conn: &Connection) -> Vec<(String, String, String, i64, Option<String>)> {
    conn.prepare(
        "SELECT xpath, kind, name, occurrences, sample FROM ingest_unknown
         WHERE xpath LIKE ?1 ORDER BY xpath, kind, name",
    )
    .unwrap()
    .query_map([format!("{STATIC}%")], |r| {
        Ok((
            r.get::<_, String>(0)?[STATIC.len()..].to_string(),
            r.get(1)?,
            r.get(2)?,
            r.get(3)?,
            r.get(4)?,
        ))
    })
    .unwrap()
    .collect::<Result<_, _>>()
    .unwrap()
}

fn row(
    xpath: &str,
    kind: &str,
    name: &str,
    occurrences: i64,
    sample: Option<&str>,
) -> (String, String, String, i64, Option<String>) {
    (
        xpath.to_string(),
        kind.to_string(),
        name.to_string(),
        occurrences,
        sample.map(str::to_string),
    )
}

#[test]
fn only_what_no_column_stores_is_reported() {
    let (_dir, path) = installed();
    let conn = open_and_migrate(&path).unwrap();
    assert_eq!(
        static_unknowns(&conn),
        [
            row(
                "/ParameterTypes/ParameterType/TypeRestriction/Enumeration",
                "Attribute",
                "Fancy",
                1,
                Some("yes"),
            ),
            row(
                "/Parameters/Parameter/Property",
                "Attribute",
                "PropertyId",
                1,
                Some("x51"),
            ),
        ]
    );
}

#[test]
fn enumeration_attributes_are_stored() {
    let (_dir, path) = installed();
    let conn = open_and_migrate(&path).unwrap();
    let stored: (String, String, String, String) = conn
        .query_row(
            "SELECT binary_value, name, icon, picture_alignment FROM parameter_type_enum
             WHERE id = 'PT-Bin_EN-1'",
            [],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )
        .unwrap();
    assert_eq!(
        stored,
        (
            "AAEAAgAD".into(),
            "hwname".into(),
            "ic.png".into(),
            "Left".into()
        )
    );
}

type Placement = (
    Option<i64>,
    Option<i64>,
    Option<i64>,
    Option<i64>,
    Option<i64>,
    Option<i64>,
    Option<i64>,
    Option<i64>,
);

fn placement(conn: &Connection, id: &str) -> Placement {
    conn.query_row(
        "SELECT own_offset, own_bit_offset, property_object_index, property_object_type,
                property_id, property_offset, property_bit_offset, property_occurrence
         FROM parameter WHERE id = ?1",
        [id],
        |r| {
            Ok((
                r.get(0)?,
                r.get(1)?,
                r.get(2)?,
                r.get(3)?,
                r.get(4)?,
                r.get(5)?,
                r.get(6)?,
                r.get(7)?,
            ))
        },
    )
    .unwrap()
}

#[test]
fn union_member_offsets_and_property_placements_are_stored() {
    let (_dir, path) = installed();
    let conn = open_and_migrate(&path).unwrap();
    let none = (None, None, None, None, None, None, None, None);
    assert_eq!(placement(&conn, "P-1"), none);
    assert_eq!(
        placement(&conn, "UP-1"),
        (Some(0), Some(0), None, None, None, None, None, None)
    );
    assert_eq!(
        placement(&conn, "UP-2"),
        (Some(1), Some(3), None, None, None, None, None, None)
    );
    assert_eq!(
        placement(&conn, "P-2"),
        (
            None,
            None,
            Some(0),
            None,
            Some(78),
            Some(0),
            Some(0),
            Some(0)
        )
    );
    // `PropertyId="x51"` is not a number: no column value, and still reported.
    assert_eq!(
        placement(&conn, "P-3"),
        (None, None, None, Some(9), None, Some(1), Some(0), None)
    );
}

#[test]
fn v24_to_v25_reproduces_a_fresh_install() {
    let (_fresh_dir, fresh_path) = installed();
    let fresh = snapshot(&open_and_migrate(&fresh_path).unwrap());

    let (_dir, path) = installed();
    {
        let conn = Connection::open(&path).unwrap();
        v24_rewind::rewind_to_v24(&conn);
        // The rewind is genuine: the columns are gone and v24's rows are back.
        assert!(static_unknowns(&conn).contains(&row(
            "/Parameters/Parameter",
            "Element",
            "Property",
            2,
            None
        )));
        assert!(!static_unknowns(&conn)
            .iter()
            .any(|(_, _, name, _, _)| name == "Fancy"));
        assert_ne!(snapshot(&conn), fresh);
    }
    let migrated = snapshot(&open_and_migrate(&path).unwrap());
    assert_eq!(
        migrated[0],
        [[format!("Integer({CURRENT_PRODUCTDB_VERSION})")]]
    );
    assert_eq!(migrated, fresh);
}

#[test]
fn v24_to_v25_names_a_damaged_blob_and_keeps_the_rest_openable() {
    let (_dir, path) = installed();
    let program_sha: String;
    {
        let conn = Connection::open(&path).unwrap();
        program_sha = conn
            .query_row(
                "SELECT source_sha256 FROM application_program WHERE id = 'A-1'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        v24_rewind::rewind_to_v24(&conn);
        conn.execute(
            "UPDATE source_file SET bytes = substr(bytes, 1, 200) WHERE sha256 = ?1",
            [&program_sha],
        )
        .unwrap();
    }
    let conn = open_and_migrate(&path).unwrap();
    let failures: i64 = conn
        .query_row(
            "SELECT count(*) FROM ingest_unknown
             WHERE source_sha256 = ?1 AND kind = 'PlacementBackfillError'",
            [&program_sha],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(failures, 1);
    // Its rows were not retired, and the report that counts it says so.
    assert!(static_unknowns(&conn).contains(&row(
        "/Parameters/Parameter",
        "Element",
        "Property",
        2,
        None
    )));
    let status: String = conn
        .query_row("SELECT status FROM package_install_report", [], |r| {
            r.get(0)
        })
        .unwrap();
    assert_eq!(status, "unavailable");
}
