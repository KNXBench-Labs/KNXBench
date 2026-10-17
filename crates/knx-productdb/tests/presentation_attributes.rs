//! Block/separator `Access` and separator `UIHint` are stored and migrated (ADR-0139).
mod v20_rewind;
mod v23_rewind;

use std::io::{Cursor, Write};

use knx_productdb::legacy::{publish_legacy, read_legacy_member};
use knx_productdb::{install_package, open_and_migrate, CURRENT_PRODUCTDB_VERSION};
use rusqlite::types::Value;
use rusqlite::Connection;
use zip::write::SimpleFileOptions;

const MASTER: &[u8] = br#"<KNX xmlns="http://knx.org/xml/project/20"><MasterData><Manufacturers><Manufacturer Id="M-0001" Name="Example"/></Manufacturers></MasterData></KNX>"#;
const HARDWARE: &[u8] = br#"<KNX xmlns="http://knx.org/xml/project/20"><ManufacturerData><Manufacturer RefId="M-0001"><Hardware><Hardware Id="H-1" Name="Example"/></Hardware></Manufacturer></ManufacturerData></KNX>"#;

/// Both hosts with and without `Access`, a hinted and an unhinted separator,
/// a restricted block inside a `ModuleDef`, and a `Channel/@Access` that is
/// not modelled and stays reported.
const PROGRAM: &[u8] = br#"<KNX xmlns="http://knx.org/xml/project/20"><ManufacturerData><Manufacturer RefId="M-0001"><ApplicationPrograms><ApplicationProgram Id="A-1" Name="Program">
<Static>
<ParameterTypes><ParameterType Id="PT-Num" Name="Num"><TypeNumber SizeInBit="8" Type="unsignedInt" minInclusive="0" maxInclusive="255"/></ParameterType></ParameterTypes>
<Parameters><Parameter Id="P-1" Name="One" ParameterType="PT-Num" Text="One" Value="1"/></Parameters>
<ParameterRefs><ParameterRef Id="P-1_R-1" RefId="P-1"/></ParameterRefs>
</Static>
<Dynamic>
<Channel Id="CH-1" Name="A" Number="1" Text="Channel" Access="None">
<ParameterBlock Id="PB-1" Name="open" Text="Open">
<ParameterSeparator Id="PS-1" Text="Heading" UIHint="Headline"/>
<ParameterRefRef RefId="P-1_R-1"/>
</ParameterBlock>
<ParameterBlock Id="PB-2" Name="ro" Text="Read only" Access="Read">
<ParameterSeparator Id="PS-2" Text="Careful" UIHint="Error" Access="None"/>
<ParameterSeparator Id="PS-3" Text="Plain"/>
</ParameterBlock>
</Channel>
</Dynamic>
<ModuleDefs><ModuleDef Id="MD-1" Name="Module"><Dynamic><ParameterBlock Id="MD-1_PB-1" Name="m" Text="Module page" Access="None"/></Dynamic></ModuleDef></ModuleDefs>
</ApplicationProgram></ApplicationPrograms></Manufacturer></ManufacturerData></KNX>"#;

const DYNAMIC: &str =
    "/KNX/ManufacturerData/Manufacturer/ApplicationPrograms/ApplicationProgram/Dynamic";
const MODULE_DYNAMIC: &str = "/KNX/ManufacturerData/Manufacturer/ApplicationPrograms/ApplicationProgram/ModuleDefs/ModuleDef/Dynamic";

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

/// Everything v24 touches, in a stable order.
fn snapshot(conn: &Connection) -> Vec<Vec<Vec<String>>> {
    [
        "PRAGMA user_version",
        "SELECT * FROM dynamic_node ORDER BY program_id, module_def_id, node_id",
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

/// `(access, ui_hint, extra)` of one element.
fn node(conn: &Connection, element: &str) -> (Option<String>, Option<String>, Option<String>) {
    conn.query_row(
        "SELECT access, ui_hint, extra FROM dynamic_node WHERE element_id = ?1",
        [element],
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
    )
    .unwrap()
}

fn access_unknown_xpaths(conn: &Connection) -> Vec<String> {
    conn.prepare(
        "SELECT DISTINCT xpath FROM ingest_unknown WHERE kind = 'Attribute' AND name = 'Access'
         AND xpath NOT LIKE '%/Static/%' ORDER BY 1",
    )
    .unwrap()
    .query_map([], |r| r.get(0))
    .unwrap()
    .collect::<Result<_, _>>()
    .unwrap()
}

fn some(value: &str) -> Option<String> {
    Some(value.to_string())
}

#[test]
fn both_hosts_store_access_and_the_separator_its_hint() {
    assert_eq!(CURRENT_PRODUCTDB_VERSION, 24);
    let (_dir, path) = installed();
    let conn = open_and_migrate(&path).unwrap();
    assert_eq!(node(&conn, "PB-1"), (None, None, some("Name=open")));
    assert_eq!(node(&conn, "PB-2"), (some("Read"), None, some("Name=ro")));
    assert_eq!(node(&conn, "PS-1"), (None, some("Headline"), None));
    assert_eq!(node(&conn, "PS-2"), (some("None"), some("Error"), None));
    assert_eq!(node(&conn, "PS-3"), (None, None, None));
    assert_eq!(
        node(&conn, "MD-1_PB-1"),
        (some("None"), None, some("Name=m"))
    );
    // `Channel/@Access` is not modelled: kept in `extra`, still reported.
    let (access, hint, extra) = node(&conn, "CH-1");
    assert_eq!((access, hint), (None, None));
    assert!(extra.unwrap().contains("Access=None"));
    assert_eq!(access_unknown_xpaths(&conn), [format!("{DYNAMIC}/Channel")]);
}

#[test]
fn v23_to_v24_reproduces_a_fresh_install() {
    let (_fresh_dir, fresh_path) = installed();
    let fresh = snapshot(&open_and_migrate(&fresh_path).unwrap());

    let (_dir, path) = installed();
    {
        let conn = Connection::open(&path).unwrap();
        v23_rewind::rewind_to_v23(&conn);
        // The rewind is genuine: the attribute is back in the reports and in
        // `extra`.
        assert_eq!(
            access_unknown_xpaths(&conn),
            [
                format!("{DYNAMIC}/Channel"),
                format!("{DYNAMIC}/ParameterBlock"),
                format!("{DYNAMIC}/ParameterSeparator"),
                format!("{MODULE_DYNAMIC}/ParameterBlock"),
            ]
        );
        let extra: String = conn
            .query_row(
                "SELECT extra FROM dynamic_node WHERE element_id = 'PS-2'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(extra, "Access=None\nUIHint=Error");
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
fn v23_to_v24_names_a_damaged_blob_and_keeps_the_rest_openable() {
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
        v23_rewind::rewind_to_v23(&conn);
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
             WHERE source_sha256 = ?1 AND kind = 'PresentationBackfillError'",
            [&program_sha],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(failures, 1);
    // Its rows were not retired, and the report that counts it says so once.
    assert!(access_unknown_xpaths(&conn).contains(&format!("{DYNAMIC}/ParameterBlock")));
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

#[test]
fn a_legacy_payload_is_not_a_backfill_failure() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("products.sqlite");
    {
        let conn = open_and_migrate(&path).unwrap();
        let bytes = std::fs::read(
            knx_testsupport::workspace_root()
                .join("crates/knx-productdb/fixtures/legacy/marvin-program-plain.vd4"),
        )
        .unwrap();
        let payload = read_legacy_member(&bytes)
            .unwrap()
            .open_unencrypted()
            .unwrap();
        publish_legacy(&conn, "marvin.vd4", &bytes, &payload).unwrap();
        v23_rewind::rewind_to_v23(&conn);
    }
    let conn = open_and_migrate(&path).unwrap();
    let failures: i64 = conn
        .query_row(
            "SELECT count(*) FROM ingest_unknown WHERE kind LIKE '%BackfillError'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(failures, 0);
    let legacy_nodes: i64 = conn
        .query_row(
            "SELECT count(*) FROM dynamic_node AS d JOIN legacy_program AS l
             ON l.program_id = d.program_id",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert!(legacy_nodes > 0, "the legacy trees stay where they were");
}
