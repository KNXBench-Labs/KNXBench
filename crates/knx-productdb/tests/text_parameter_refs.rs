//! `TextParameterRefId` is stored, queried and migrated (ADR-0126, Goal B14).
mod v20_rewind;
mod v22_rewind;

use std::io::{Cursor, Write};

use knx_productdb::query::{text_parameter_facts, text_parameter_refs, TextParameterFact};
use knx_productdb::{install_package, open_and_migrate, CURRENT_PRODUCTDB_VERSION};
use rusqlite::types::Value;
use rusqlite::Connection;
use zip::write::SimpleFileOptions;

const MASTER: &[u8] = br#"<KNX xmlns="http://knx.org/xml/project/11"><MasterData><Manufacturers><Manufacturer Id="M-0001" Name="Example"/></Manufacturers></MasterData></KNX>"#;
const HARDWARE: &[u8] = br#"<KNX xmlns="http://knx.org/xml/project/11"><ManufacturerData><Manufacturer RefId="M-0001"><Hardware><Hardware Id="H-1" Name="Example"/></Hardware></Manufacturer></ManufacturerData></KNX>"#;

/// All four hosts, a module channel with an argument inside its default, a
/// non-text parameter, a product default that a `ParameterRef` overrides, and
/// a `ParameterRef` carrying a reference (allowed, unattested, stays reported).
const PROGRAM: &[u8] = br#"<KNX xmlns="http://knx.org/xml/project/11"><ManufacturerData><Manufacturer RefId="M-0001"><ApplicationPrograms><ApplicationProgram Id="A-1" Name="Program">
<Static>
<ParameterTypes>
<ParameterType Id="PT-Text" Name="Text"><TypeText SizeInBit="240"/></ParameterType>
<ParameterType Id="PT-Num" Name="Num"><TypeNumber SizeInBit="8" Type="unsignedInt" minInclusive="0" maxInclusive="255"/></ParameterType>
</ParameterTypes>
<Parameters>
<Parameter Id="P-1" Name="ChannelName" ParameterType="PT-Text" Text="Name" Value=""/>
<Parameter Id="P-2" Name="PageName" ParameterType="PT-Text" Text="Page" Value="Living"/>
<Parameter Id="P-3" Name="Count" ParameterType="PT-Num" Text="Count" Value="1"/>
</Parameters>
<ParameterRefs>
<ParameterRef Id="P-1_R-1" RefId="P-1"/>
<ParameterRef Id="P-2_R-2" RefId="P-2" Value="Kitchen"/>
<ParameterRef Id="P-3_R-3" RefId="P-3" TextParameterRefId="P-1_R-1"/>
</ParameterRefs>
<ComObjectTable><ComObject Id="O-1" Number="1" Text="Switch {{0:Output}}"/></ComObjectTable>
<ComObjectRefs><ComObjectRef Id="O-1_R-1" RefId="O-1" TextParameterRefId="P-1_R-1"/><ComObjectRef Id="O-1_R-2" RefId="O-1" Text="Plain"/></ComObjectRefs>
</Static>
<Dynamic>
<Channel Id="CH-1" Name="A" Number="1" Text="{{0:Channel A}}" TextParameterRefId="P-1_R-1">
<ParameterBlock Id="PB-1" Name="b" Text="Page {{0:1}}" TextParameterRefId="P-2_R-2">
<ParameterSeparator Id="PS-1" Text="-- {{0}} --" TextParameterRefId="P-2_R-2"/>
<ParameterRefRef RefId="P-1_R-1"/>
<ComObjectRefRef RefId="O-1_R-1"/>
</ParameterBlock>
</Channel>
</Dynamic>
<ModuleDefs><ModuleDef Id="MD-1" Name="Module"><Dynamic><Channel Id="MD-1_CH-1" Name="m" Number="1" Text="{{0:Button {{ChNo}}}}" TextParameterRefId="P-1_R-1"/></Dynamic></ModuleDef></ModuleDefs>
</ApplicationProgram></ApplicationPrograms></Manufacturer></ManufacturerData></KNX>"#;

const DYNAMIC: &str =
    "/KNX/ManufacturerData/Manufacturer/ApplicationPrograms/ApplicationProgram/Dynamic";

fn package() -> Vec<u8> {
    let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for (path, bytes) in [
        ("knx_master.xml", MASTER),
        ("M-0001/Hardware.xml", HARDWARE),
        ("M-0001/A.xml", PROGRAM),
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

/// Everything v23 touches, in a stable order.
fn snapshot(conn: &Connection) -> Vec<Vec<Vec<String>>> {
    [
        "PRAGMA user_version",
        "SELECT * FROM dynamic_node ORDER BY program_id, module_def_id, node_id",
        "SELECT * FROM com_object_ref ORDER BY program_id, id",
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

fn reference(conn: &Connection, table_where: &str) -> Option<String> {
    conn.query_row(
        &format!("SELECT text_parameter_ref_id FROM {table_where}"),
        [],
        |r| r.get(0),
    )
    .unwrap()
}

fn unknown_xpaths(conn: &Connection) -> Vec<String> {
    conn.prepare(
        "SELECT DISTINCT xpath FROM ingest_unknown WHERE name = 'TextParameterRefId' ORDER BY 1",
    )
    .unwrap()
    .query_map([], |r| r.get(0))
    .unwrap()
    .collect::<Result<_, _>>()
    .unwrap()
}

#[test]
fn the_four_hosts_store_their_reference_and_report_nothing() {
    let (_dir, path) = installed();
    let conn = open_and_migrate(&path).unwrap();
    for (element, expected) in [
        ("CH-1", "P-1_R-1"),
        ("PB-1", "P-2_R-2"),
        ("PS-1", "P-2_R-2"),
        ("MD-1_CH-1", "P-1_R-1"),
    ] {
        assert_eq!(
            reference(
                &conn,
                &format!("dynamic_node WHERE element_id = '{element}'")
            ),
            Some(expected.to_string()),
            "{element}"
        );
    }
    // Every other node, and an object ref without one, stays NULL.
    let others: i64 = conn
        .query_row(
            "SELECT count(*) FROM dynamic_node WHERE text_parameter_ref_id IS NOT NULL
             AND kind NOT IN ('Channel', 'ParameterBlock', 'ParameterSeparator')",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(others, 0);
    assert_eq!(
        reference(&conn, "com_object_ref WHERE id = 'O-1_R-1'"),
        Some("P-1_R-1".into())
    );
    assert_eq!(
        reference(&conn, "com_object_ref WHERE id = 'O-1_R-2'"),
        None
    );
    // Not in `extra` any more, and reported only where it is still unmodelled.
    let extra: Option<String> = conn
        .query_row(
            "SELECT extra FROM dynamic_node WHERE element_id = 'PB-1'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(extra.as_deref(), Some("Name=b"));
    assert_eq!(
        unknown_xpaths(&conn),
        ["/KNX/ManufacturerData/Manufacturer/ApplicationPrograms/ApplicationProgram/Static/ParameterRefs/ParameterRef"]
    );
}

#[test]
fn the_queries_answer_per_program_with_product_defaults() {
    let (_dir, path) = installed();
    let conn = open_and_migrate(&path).unwrap();
    let refs = text_parameter_refs(&conn, "A-1").unwrap();
    let node = |element: &str| -> (String, i64) {
        conn.query_row(
            "SELECT module_def_id, node_id FROM dynamic_node WHERE element_id = ?1",
            [element],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap()
    };
    assert_eq!(refs.dynamic.len(), 4);
    assert_eq!(refs.dynamic[&node("CH-1")], "P-1_R-1");
    assert_eq!(refs.dynamic[&node("PS-1")], "P-2_R-2");
    assert_eq!(node("MD-1_CH-1").0, "MD-1");
    assert_eq!(refs.dynamic[&node("MD-1_CH-1")], "P-1_R-1");
    assert_eq!(refs.com_object_refs.len(), 1);
    assert_eq!(refs.com_object_refs["O-1_R-1"], "P-1_R-1");
    assert_eq!(
        text_parameter_refs(&conn, "A-404").unwrap(),
        Default::default()
    );

    let facts =
        text_parameter_facts(&conn, "A-1", &["P-1_R-1", "P-2_R-2", "P-3_R-3", "nope"]).unwrap();
    assert_eq!(
        facts["P-1_R-1"],
        TextParameterFact {
            is_text: true,
            default: Some(String::new())
        }
    );
    // The `ParameterRef`'s own value beats the `Parameter`'s.
    assert_eq!(
        facts["P-2_R-2"],
        TextParameterFact {
            is_text: true,
            default: Some("Kitchen".into())
        }
    );
    assert!(!facts["P-3_R-3"].is_text);
    assert!(!facts.contains_key("nope"));
}

#[test]
fn v22_to_v23_reproduces_a_fresh_install() {
    let (_fresh_dir, fresh_path) = installed();
    let fresh = snapshot(&open_and_migrate(&fresh_path).unwrap());

    let (_dir, path) = installed();
    {
        let conn = Connection::open(&path).unwrap();
        v22_rewind::rewind_to_v22(&conn);
        // The rewind is genuine: the attribute is back in the reports.
        let mut expected = vec![
            format!("{DYNAMIC}/Channel"),
            format!("{DYNAMIC}/ParameterBlock"),
            format!("{DYNAMIC}/ParameterSeparator"),
            "/KNX/ManufacturerData/Manufacturer/ApplicationPrograms/ApplicationProgram/ModuleDefs/ModuleDef/Dynamic/Channel".to_string(),
            "/KNX/ManufacturerData/Manufacturer/ApplicationPrograms/ApplicationProgram/Static/ComObjectRefs/ComObjectRef".to_string(),
            "/KNX/ManufacturerData/Manufacturer/ApplicationPrograms/ApplicationProgram/Static/ParameterRefs/ParameterRef".to_string(),
        ];
        expected.sort();
        assert_eq!(unknown_xpaths(&conn), expected);
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
fn v22_to_v23_names_a_damaged_blob_and_keeps_the_rest_openable() {
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
        v22_rewind::rewind_to_v22(&conn);
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
             WHERE source_sha256 = ?1 AND kind = 'TextParameterRefBackfillError'",
            [&program_sha],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(failures, 1);
    // Its rows were not retired, and the report that counts it says so once.
    assert!(unknown_xpaths(&conn).contains(&format!("{DYNAMIC}/Channel")));
    let status: String = conn
        .query_row("SELECT status FROM package_install_report", [], |r| {
            r.get(0)
        })
        .unwrap();
    assert_eq!(status, "unavailable");
    let report_failures: i64 = conn
        .query_row(
            "SELECT count(*) FROM ingest_unknown WHERE kind = 'InstallReportBackfillError'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(report_failures, 1);
}
