//! Placements and enumeration attributes reach typed columns; the rest stays reported (v25).
use crate::v24_rewind;
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
<Property ObjectIndex="1" PropertyId="12" Offset="3" BitOffset="0" Occurrence="1"/>
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
            // A `Property` that places no parameter stays unmodelled.
            row("", "Element", "Property", 1, None),
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
            row("/Property", "Attribute", "BitOffset", 1, Some("0")),
            row("/Property", "Attribute", "ObjectIndex", 1, Some("1")),
            row("/Property", "Attribute", "Occurrence", 1, Some("1")),
            row("/Property", "Attribute", "Offset", 1, Some("3")),
            row("/Property", "Attribute", "PropertyId", 1, Some("12")),
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

/// Earlier promotions must converge on fresh ingest, including evidence on
/// similarly named hosts outside the promoted context. Keep these synthetic
/// fixtures independent of the fixture-specific v22/v23/v24 rewind modules.
mod promotion_repair_regressions {
    use super::{rows, HARDWARE, MASTER};
    use crate::v20_rewind;
    use knx_productdb::{install_package, open_and_migrate, CURRENT_PRODUCTDB_VERSION};
    use rusqlite::{params, Connection};
    use std::io::{Cursor, Write};
    use std::path::{Path, PathBuf};
    use zip::write::SimpleFileOptions;

    const ROOT: &str = "/KNX/ManufacturerData/Manufacturer/ApplicationPrograms/ApplicationProgram";
    type Evidence = (String, String, String, i64, Option<String>);
    type Snapshot = Vec<Vec<Vec<String>>>;

    fn attribute(host: &str, name: &str, sample: &str) -> Evidence {
        (
            format!("{ROOT}{host}"),
            "Attribute".into(),
            name.into(),
            1,
            Some(sample.into()),
        )
    }

    fn package(programs: &str) -> Vec<u8> {
        let xml = format!(
            "<KNX xmlns=\"http://knx.org/xml/project/20\"><ManufacturerData>\
             <Manufacturer RefId=\"M-0001\"><ApplicationPrograms>{programs}\
             </ApplicationPrograms></Manufacturer></ManufacturerData></KNX>"
        );
        let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
        for (path, bytes) in [
            ("knx_master.xml", MASTER),
            ("M-0001/Hardware.xml", HARDWARE),
            ("M-0001/M-0001_A-1.xml", xml.as_bytes()),
        ] {
            writer
                .start_file(path, SimpleFileOptions::default())
                .unwrap();
            writer.write_all(bytes).unwrap();
        }
        writer.finish().unwrap().into_inner()
    }

    fn installed(bytes: &[u8]) -> (tempfile::TempDir, PathBuf, Connection) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("products.sqlite");
        let conn = open_and_migrate(&path).unwrap();
        install_package(&conn, "promotion-regression.knxprod", bytes).unwrap();
        (dir, path, conn)
    }

    fn evidence(conn: &Connection, table: &str) -> Vec<Evidence> {
        assert!(matches!(
            table,
            "ingest_unknown" | "package_install_unknown"
        ));
        conn.prepare(&format!(
            "SELECT xpath, kind, name, occurrences, sample FROM {table}
             WHERE kind IN ('Attribute', 'Element') ORDER BY xpath, kind, name"
        ))
        .unwrap()
        .query_map([], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?))
        })
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap()
    }

    fn named_evidence(conn: &Connection, name: &str) -> Vec<Evidence> {
        evidence(conn, "ingest_unknown")
            .into_iter()
            .filter(|r| r.2 == name)
            .collect()
    }

    /// Full affected normalized rows plus every persisted report and conflict
    /// fact. Exclude only ingest_unknown's surrogate id, not its ownership,
    /// sample or occurrence count. Source bytes and ownership are also pinned.
    fn snapshot(conn: &Connection) -> Snapshot {
        [
            "PRAGMA user_version",
            "SELECT * FROM application_program ORDER BY id",
            "SELECT * FROM com_object_ref ORDER BY program_id, id",
            "SELECT * FROM parameter ORDER BY program_id, id",
            "SELECT * FROM parameter_ref ORDER BY program_id, id",
            "SELECT * FROM parameter_type_enum ORDER BY program_id, id",
            "SELECT * FROM dynamic_node ORDER BY program_id, module_def_id, node_id",
            "SELECT source_sha256, program_id, xpath, kind, name, occurrences, sample
             FROM ingest_unknown ORDER BY 1, 2, 3, 4, 5, 6, 7",
            "SELECT * FROM package_install_report ORDER BY package_sha256",
            "SELECT * FROM package_install_unknown ORDER BY package_sha256, ordinal",
            "SELECT * FROM package_install_count ORDER BY package_sha256, ordinal",
            "SELECT * FROM package_install_diagnostic ORDER BY package_sha256, ordinal",
            "SELECT * FROM package_conflict ORDER BY package_sha256, ordinal",
            "SELECT sha256, unknown_count FROM package ORDER BY sha256",
            "SELECT * FROM source_file ORDER BY sha256",
            "SELECT * FROM package_member ORDER BY package_sha256, path",
        ]
        .iter()
        .map(|sql| rows(conn, sql))
        .collect()
    }

    fn assert_reports_consistent(conn: &Connection) {
        let report_rows = evidence(conn, "package_install_unknown");
        let distinct = report_rows.len() as i64;
        let occurrences: i64 = report_rows.iter().map(|r| r.3).sum();
        let report: (String, i64, i64) = conn
            .query_row(
                "SELECT status, unknown_distinct, unknown_occurrences FROM package_install_report",
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .unwrap();
        assert_eq!(report, ("measured".into(), distinct, occurrences));
        for (disposition, count) in [("read", occurrences), ("stored", distinct)] {
            let actual: i64 = conn
                .query_row(
                    "SELECT count FROM package_install_count
                     WHERE category = 'unknown_construct' AND disposition = ?1",
                    [disposition],
                    |r| r.get(0),
                )
                .unwrap();
            assert_eq!(actual, count, "unknown_construct/{disposition}");
        }
        // These packages each have one program member; the master and hardware
        // have no unknowns. IdConflict is a separate immutable encounter fact.
        assert_eq!(evidence(conn, "ingest_unknown"), report_rows);
        let unknown_count: i64 = conn
            .query_row("SELECT unknown_count FROM package", [], |r| r.get(0))
            .unwrap();
        assert_eq!(unknown_count, distinct);
    }

    /// Scoped historical reconstruction, not just a user_version change:
    /// remove every post-entry schema object and restore the exact unknown
    /// rows and measured-report delta of these fixtures' winning declaration.
    ///
    /// Boundary: no SuffixText, no promoted Dynamic caption/presentation
    /// values, no unmodelled Enumeration attributes, no unions and only one
    /// program blob/package. Thus older Dynamic extra and other entity counts
    /// are unchanged. Static ingest still reports unknown attributes from a
    /// losing declaration before deciding whether its normalized rows may be
    /// stored, so `old_only` includes both declarations' evidence.
    fn rewind(conn: &Connection, version: i64, old_only: &[Evidence]) {
        let promoted_dynamic: i64 = conn
            .query_row(
                "SELECT count(*) FROM dynamic_node WHERE text_parameter_ref_id IS NOT NULL
                 OR access IS NOT NULL OR ui_hint IS NOT NULL",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(promoted_dynamic, 0, "fixture exceeds scoped Dynamic rewind");
        let suffixes: i64 = conn
            .query_row(
                "SELECT (SELECT count(*) FROM parameter WHERE suffix_text IS NOT NULL)
                      + (SELECT count(*) FROM parameter_ref WHERE suffix_text IS NOT NULL)",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(suffixes, 0, "fixture exceeds scoped suffix rewind");
        assert_reports_consistent(conn);
        let mut historical = evidence(conn, "package_install_unknown");
        let blob: String = conn
            .query_row("SELECT source_sha256 FROM application_program", [], |r| {
                r.get(0)
            })
            .unwrap();
        for (xpath, kind, name, occurrences, sample) in old_only {
            assert!(!historical
                .iter()
                .any(|r| (&r.0, &r.1, &r.2) == (xpath, kind, name)));
            conn.execute(
                "INSERT INTO ingest_unknown
                 (source_sha256, program_id, xpath, kind, name, occurrences, sample)
                 VALUES (?1, NULL, ?2, ?3, ?4, ?5, ?6)",
                params![blob, xpath, kind, name, occurrences, sample],
            )
            .unwrap();
        }
        historical.extend_from_slice(old_only);
        historical.sort_by(|a, b| (&a.0, &a.1, &a.2).cmp(&(&b.0, &b.1, &b.2)));
        conn.execute("DELETE FROM package_install_unknown", [])
            .unwrap();
        let package: String = conn
            .query_row("SELECT sha256 FROM package", [], |r| r.get(0))
            .unwrap();
        for (ordinal, (xpath, kind, name, occurrences, sample)) in historical.iter().enumerate() {
            conn.execute(
                "INSERT INTO package_install_unknown
                 (package_sha256, ordinal, xpath, kind, name, occurrences, sample)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    package,
                    ordinal as i64,
                    xpath,
                    kind,
                    name,
                    occurrences,
                    sample
                ],
            )
            .unwrap();
        }
        let distinct = historical.len() as i64;
        let occurrences: i64 = historical.iter().map(|r| r.3).sum();
        conn.execute(
            "UPDATE package_install_report SET unknown_distinct = ?1, unknown_occurrences = ?2",
            params![distinct, occurrences],
        )
        .unwrap();
        for (disposition, count) in [("read", occurrences), ("stored", distinct)] {
            conn.execute(
                "UPDATE package_install_count SET count = ?2
                 WHERE category = 'unknown_construct' AND disposition = ?1",
                params![disposition, count],
            )
            .unwrap();
        }
        conn.execute("UPDATE package SET unknown_count = ?1", [distinct])
            .unwrap();
        match version {
            22 => v20_rewind::drop_v23_objects(conn),
            23 => v20_rewind::drop_v24_objects(conn),
            24 => v20_rewind::drop_v25_objects(conn),
            _ => panic!("unsupported scoped rewind version"),
        }
        conn.pragma_update(None, "user_version", version).unwrap();
        for (table, column, absent) in [
            ("com_object_ref", "text_parameter_ref_id", version < 23),
            ("dynamic_node", "text_parameter_ref_id", version < 23),
            ("dynamic_node", "access", version < 24),
            ("dynamic_node", "ui_hint", version < 24),
            ("parameter", "own_offset", true),
            ("parameter", "property_id", true),
            ("parameter_type_enum", "binary_value", true),
            ("parameter", "suffix_text", true),
            ("parameter_ref", "suffix_text", true),
        ] {
            let present: bool = conn
                .query_row(
                    "SELECT EXISTS(SELECT 1 FROM pragma_table_info(?1) WHERE name = ?2)",
                    [table, column],
                    |r| r.get(0),
                )
                .unwrap();
            assert_eq!(present, !absent, "v{version}: {table}.{column}");
        }
        assert_eq!(evidence(conn, "package_install_unknown"), historical);
        assert_reports_consistent(conn);
        assert_eq!(
            conn.pragma_query_value(None, "user_version", |r| r.get::<_, i64>(0))
                .unwrap(),
            version
        );
        assert_eq!(
            conn.query_row("PRAGMA integrity_check", [], |r| r.get::<_, String>(0))
                .unwrap(),
            "ok"
        );
        assert!(rows(conn, "PRAGMA foreign_key_check").is_empty());
    }

    fn assert_final(path: &Path, conn: Connection, fresh: &Snapshot) {
        assert_eq!(
            conn.pragma_query_value(None, "user_version", |r| r.get::<_, i64>(0))
                .unwrap(),
            CURRENT_PRODUCTDB_VERSION as i64
        );
        assert_reports_consistent(&conn);
        assert_eq!(
            &snapshot(&conn),
            fresh,
            "repaired semantic values/evidence must equal fresh ingest"
        );
        drop(conn);
        assert_eq!(
            &snapshot(&open_and_migrate(path).unwrap()),
            fresh,
            "reopen must be idempotent"
        );
    }

    fn object_program(caption: Option<&str>, name: &str) -> String {
        let caption = caption
            .map(|v| format!(" TextParameterRefId=\"{v}\""))
            .unwrap_or_default();
        format!(
            "<ApplicationProgram Id=\"A-1\" Name=\"{name}\"><Static>\
             <ComObjectTable><ComObject Id=\"O-1\" Number=\"0\"/></ComObjectTable>\
             <ComObjectRefs><ComObjectRef Id=\"O-R\" RefId=\"O-1\"{caption}/></ComObjectRefs>\
             </Static></ApplicationProgram>"
        )
    }

    fn object_caption(conn: &Connection) -> Option<String> {
        let values: Vec<Option<String>> = conn
            .prepare("SELECT text_parameter_ref_id FROM com_object_ref ORDER BY id")
            .unwrap()
            .query_map([], |r| r.get(0))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();
        assert_eq!(values.len(), 1);
        values[0].clone()
    }

    fn duplicate_object_case(winner: Option<&str>) {
        let bytes = package(&format!(
            "{}{}",
            object_program(winner, "first"),
            object_program(Some("loser-caption"), "second")
        ));
        let (_dir, path, conn) = installed(&bytes);
        assert_eq!(
            object_caption(&conn).as_deref(),
            winner,
            "fresh static ComObjectRef first winner"
        );
        assert!(named_evidence(&conn, "TextParameterRefId").is_empty());
        let fresh = snapshot(&conn);
        let mut old_caption = attribute(
            "/Static/ComObjectRefs/ComObjectRef",
            "TextParameterRefId",
            winner.unwrap_or("loser-caption"),
        );
        old_caption.3 = if winner.is_some() { 2 } else { 1 };
        let old_only = vec![old_caption];
        rewind(&conn, 22, &old_only);
        assert_eq!(named_evidence(&conn, "TextParameterRefId"), old_only);
        drop(conn);
        let migrated = open_and_migrate(&path).unwrap();
        assert_eq!(
            object_caption(&migrated).as_deref(),
            winner,
            "v22 repair must retain the first static ComObjectRef caption, including absence"
        );
        assert_final(&path, migrated, &fresh);
    }

    #[test]
    fn v22_duplicate_program_static_com_object_ref_first_winner() {
        duplicate_object_case(Some("first-caption"));
    }

    #[test]
    fn v22_duplicate_program_static_com_object_ref_winning_absence() {
        duplicate_object_case(None);
    }

    fn placement_program(declarations: bool, loser: bool) -> String {
        let (name, own, property, enumeration) = if !declarations {
            ("first", "", "", "")
        } else if loser {
            ("second", " Offset=\"12\" BitOffset=\"1\"", "<Property ObjectIndex=\"14\" ObjectType=\"15\" PropertyId=\"79\" Offset=\"16\" BitOffset=\"1\" Occurrence=\"18\"/>", " BinaryValue=\"Ag==\" Name=\"second-enum\" Icon=\"second.png\" PictureAlignment=\"Right\"")
        } else {
            ("first", " Offset=\"2\" BitOffset=\"3\"", "<Property ObjectIndex=\"4\" ObjectType=\"5\" PropertyId=\"78\" Offset=\"6\" BitOffset=\"7\" Occurrence=\"8\"/>", " BinaryValue=\"AQ==\" Name=\"first-enum\" Icon=\"first.png\" PictureAlignment=\"Left\"")
        };
        format!(
            "<ApplicationProgram Id=\"A-1\" Name=\"{name}\"><Static>\
             <ParameterTypes><ParameterType Id=\"PT-1\" Name=\"type\"><TypeRestriction Base=\"BinaryValue\" SizeInBit=\"8\">\
             <Enumeration Id=\"E-1\" Text=\"option\" Value=\"1\"{enumeration}/>\
             </TypeRestriction></ParameterType></ParameterTypes>\
             <Parameters><Parameter Id=\"P-1\" Name=\"parameter\" ParameterType=\"PT-1\" Value=\"1\"{own}>{property}</Parameter></Parameters>\
             </Static></ApplicationProgram>"
        )
    }

    fn placement_values(conn: &Connection) -> Vec<Vec<String>> {
        rows(
            conn,
            "SELECT own_offset, own_bit_offset, property_object_index, property_object_type,
                    property_id, property_offset, property_bit_offset, property_occurrence
                    FROM parameter ORDER BY id",
        )
    }

    fn enumeration_values(conn: &Connection) -> Vec<Vec<String>> {
        rows(conn, "SELECT binary_value, name, icon, picture_alignment FROM parameter_type_enum ORDER BY id")
    }

    fn duplicate_placement_case(declarations: bool) {
        let bytes = package(&format!(
            "{}{}",
            placement_program(declarations, false),
            placement_program(true, true)
        ));
        let (_dir, path, conn) = installed(&bytes);
        let expected_placement = if declarations {
            vec![vec![
                "Integer(2)",
                "Integer(3)",
                "Integer(4)",
                "Integer(5)",
                "Integer(78)",
                "Integer(6)",
                "Integer(7)",
                "Integer(8)",
            ]]
        } else {
            vec![vec!["Null"; 8]]
        };
        let expected_enum = if declarations {
            vec![vec![
                "Text(\"AQ==\")",
                "Text(\"first-enum\")",
                "Text(\"first.png\")",
                "Text(\"Left\")",
            ]]
        } else {
            vec![vec!["Null"; 4]]
        };
        assert_eq!(
            placement_values(&conn),
            expected_placement,
            "fresh placement first winner"
        );
        assert_eq!(
            enumeration_values(&conn),
            expected_enum,
            "fresh enumeration first winner"
        );
        assert!(evidence(&conn, "package_install_unknown").is_empty());
        let fresh = snapshot(&conn);
        let (own_offset, own_bit_offset, property_values) = if declarations {
            ("2", "3", ["4", "5", "78", "6", "7", "8"])
        } else {
            ("12", "1", ["14", "15", "79", "16", "1", "18"])
        };
        let occurrences = if declarations { 2 } else { 1 };
        let mut old_only = vec![
            attribute("/Static/Parameters/Parameter", "Offset", own_offset),
            attribute("/Static/Parameters/Parameter", "BitOffset", own_bit_offset),
            (
                format!("{ROOT}/Static/Parameters/Parameter"),
                "Element".into(),
                "Property".into(),
                occurrences,
                None,
            ),
        ];
        for (name, value) in [
            "ObjectIndex",
            "ObjectType",
            "PropertyId",
            "Offset",
            "BitOffset",
            "Occurrence",
        ]
        .into_iter()
        .zip(property_values)
        {
            old_only.push(attribute(
                "/Static/Parameters/Parameter/Property",
                name,
                value,
            ));
        }
        for row in &mut old_only {
            row.3 = occurrences;
        }
        old_only.sort_by(|a, b| (&a.0, &a.1, &a.2).cmp(&(&b.0, &b.1, &b.2)));
        rewind(&conn, 24, &old_only);
        assert_eq!(evidence(&conn, "package_install_unknown"), old_only);
        drop(conn);
        let migrated = open_and_migrate(&path).unwrap();
        // Assert together so the RED receipt includes both independent losers.
        assert_eq!(
            (placement_values(&migrated), enumeration_values(&migrated)),
            (
                placement_values_expected(expected_placement),
                placement_values_expected(expected_enum)
            ),
            "v24 repair must retain the first placement/enumeration, including absent values"
        );
        assert_final(&path, migrated, &fresh);
    }

    fn placement_values_expected(values: Vec<Vec<&str>>) -> Vec<Vec<String>> {
        values
            .into_iter()
            .map(|r| r.into_iter().map(str::to_string).collect())
            .collect()
    }

    #[test]
    fn v24_duplicate_program_parameter_placement_and_enumeration_first_winner() {
        duplicate_placement_case(true);
    }

    #[test]
    fn v24_duplicate_program_parameter_placement_and_enumeration_winning_absence() {
        duplicate_placement_case(false);
    }

    fn evidence_case(program: &str, version: i64, expected: Vec<Evidence>, name: &str) {
        let (_dir, path, conn) = installed(&package(program));
        assert_eq!(
            named_evidence(&conn, name),
            expected,
            "exact fresh unsupported evidence"
        );
        let fresh = snapshot(&conn);
        rewind(&conn, version, &[]);
        assert_eq!(
            named_evidence(&conn, name),
            expected,
            "unsupported evidence already existed at entry"
        );
        drop(conn);
        let migrated = open_and_migrate(&path).unwrap();
        assert_eq!(
            named_evidence(&migrated, name),
            expected,
            "v{version} repair must preserve unsupported host evidence"
        );
        assert_final(&path, migrated, &fresh);
    }

    #[test]
    fn v23_static_parameter_block_access_evidence_survives() {
        evidence_case(
            "<ApplicationProgram Id=\"A-1\" Name=\"first\"><Static><ParameterBlock Id=\"SB-1\" Access=\"Read\"/></Static></ApplicationProgram>",
            23,
            vec![attribute("/Static/ParameterBlock", "Access", "Read")],
            "Access",
        );
    }

    #[test]
    fn v22_static_parameter_block_text_parameter_ref_evidence_survives() {
        evidence_case(
            "<ApplicationProgram Id=\"A-1\" Name=\"first\"><Static><ParameterBlock Id=\"SB-1\" TextParameterRefId=\"unsupported-static-caption\"/></Static></ApplicationProgram>",
            22,
            vec![attribute("/Static/ParameterBlock", "TextParameterRefId", "unsupported-static-caption")],
            "TextParameterRefId",
        );
    }

    #[test]
    fn v22_dynamic_com_object_ref_evidence_survives() {
        evidence_case(
            "<ApplicationProgram Id=\"A-1\" Name=\"first\"><Static/><Dynamic><ComObjectRef Id=\"D-O-R\" TextParameterRefId=\"unsupported-dynamic-caption\"/></Dynamic></ApplicationProgram>",
            22,
            vec![attribute("/Dynamic/ComObjectRef", "TextParameterRefId", "unsupported-dynamic-caption")],
            "TextParameterRefId",
        );
    }

    #[test]
    fn v24_dynamic_parameter_offset_and_bit_offset_evidence_survives() {
        let (_dir, path, conn) = installed(&package(
            "<ApplicationProgram Id=\"A-1\" Name=\"first\"><Static/><Dynamic><Parameter Id=\"DP-1\" Offset=\"8\" BitOffset=\"2\"/></Dynamic></ApplicationProgram>"
        ));
        let expected = vec![
            attribute("/Dynamic/Parameter", "BitOffset", "2"),
            attribute("/Dynamic/Parameter", "Offset", "8"),
        ];
        let offsets = |c: &Connection| {
            evidence(c, "ingest_unknown")
                .into_iter()
                .filter(|r| matches!(r.2.as_str(), "Offset" | "BitOffset"))
                .collect::<Vec<_>>()
        };
        assert_eq!(
            offsets(&conn),
            expected,
            "exact fresh Dynamic offset evidence"
        );
        assert!(
            rows(&conn, "SELECT id FROM parameter").is_empty(),
            "Dynamic Parameter has no static placement row"
        );
        assert_eq!(
            rows(
                &conn,
                "SELECT extra FROM dynamic_node WHERE kind = 'Parameter'"
            ),
            vec![vec!["Text(\"BitOffset=2\\nId=DP-1\\nOffset=8\")"]]
        );
        let fresh = snapshot(&conn);
        rewind(&conn, 24, &[]);
        assert_eq!(offsets(&conn), expected);
        drop(conn);
        let migrated = open_and_migrate(&path).unwrap();
        assert_eq!(
            offsets(&migrated),
            expected,
            "v24 repair must retain both unmodelled Dynamic offsets"
        );
        assert_final(&path, migrated, &fresh);
    }
}
