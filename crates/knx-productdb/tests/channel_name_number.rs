//! Checks channel name and number import, projection, and the v18 migration.
//!
//! ADR-0052 (KNOWN_LIMITATIONS §146) stores both fields verbatim, retires
//! `Channel/@Number` from unknown evidence, and checks v17 migration against a
//! fresh install.
mod v18_rewind;

use std::collections::HashMap;
use std::io::{Cursor, Write};

use knx_productdb::dynamic::{evaluate, load_program_trees, load_tree};
use knx_productdb::{ingest_file, install_package, open_and_migrate, sha256_hex};
use rusqlite::Connection;
use zip::write::SimpleFileOptions;

const MASTER: &[u8] = br#"<KNX xmlns="http://knx.org/xml/project/11"><MasterData><Manufacturers><Manufacturer Id="M-0001" Name="Example"/></Manufacturers></MasterData></KNX>"#;
const HARDWARE: &[u8] = br#"<KNX xmlns="http://knx.org/xml/project/11"><ManufacturerData><Manufacturer RefId="M-0001"><Hardware><Hardware Id="H-1" Name="Example"/></Hardware></Manufacturer></ManufacturerData></KNX>"#;

/// Every shape the corpus shows, with invented values: a channel whose
/// `@Text` is empty, one whose `@Name` is empty, a non-numeric `@Number`,
/// a channel without either attribute, a `TextParameterRefId` that stays
/// unmodelled, and a `ModuleDef` channel with a placeholder in `@Text`.
const PROGRAM: &[u8] = br#"<KNX xmlns="http://knx.org/xml/project/11"><ManufacturerData><Manufacturer RefId="M-0001"><ApplicationPrograms><ApplicationProgram Id="A-1" Name="Program">
<Static><ComObjectTable><ComObject Id="O-1" Number="1"/><ComObject Id="O-2" Number="2"/></ComObjectTable><ComObjectRefs><ComObjectRef Id="OR-1" RefId="O-1"/><ComObjectRef Id="OR-2" RefId="O-2"/></ComObjectRefs></Static>
<Dynamic>
<Channel Id="CH-1" Name="Light A" Number="1" Text=""><ParameterBlock Id="PB-1" Name="block"><ComObjectRefRef RefId="OR-1"/></ParameterBlock></Channel>
<Channel Id="CH-2" Name="" Number="B" Text="Second" TextParameterRefId="P-1"><ComObjectRefRef RefId="OR-2"/></Channel>
<Channel Id="CH-3" Text="Bare"/>
</Dynamic>
<ModuleDefs><ModuleDef Id="MD-1" Name="Module"><Dynamic><Channel Id="MD-1_CH-1" Name="Module channel" Number="1/2/3" Text="{{argName}}"/></Dynamic></ModuleDef></ModuleDefs>
</ApplicationProgram></ApplicationPrograms></Manufacturer></ManufacturerData></KNX>"#;

const PROGRAM_CHANNEL: &str =
    "/KNX/ManufacturerData/Manufacturer/ApplicationPrograms/ApplicationProgram/Dynamic/Channel";

fn archive(entries: &[(&str, &[u8])]) -> Vec<u8> {
    let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for (path, bytes) in entries {
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
        ("M-0001/Hardware.xml", HARDWARE),
        ("M-0001/A.xml", PROGRAM),
    ])
}

/// `(name, number, extra)` of one channel.
type Stored = (Option<String>, Option<String>, Option<String>);

fn channel(conn: &Connection, element_id: &str) -> Stored {
    conn.query_row(
        "SELECT name, number, extra FROM dynamic_node WHERE kind = 'Channel' AND element_id = ?1",
        [element_id],
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
    )
    .unwrap()
}

fn unknown_names(conn: &Connection, xpath: &str) -> Vec<String> {
    conn.prepare("SELECT name FROM ingest_unknown WHERE xpath = ?1 ORDER BY name")
        .unwrap()
        .query_map([xpath], |r| r.get(0))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap()
}

fn s(value: &str) -> Option<String> {
    Some(value.to_string())
}

#[test]
fn channels_store_name_and_number_verbatim() {
    let dir = tempfile::tempdir().unwrap();
    let conn = open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
    ingest_file(&conn, "M-0001/A.xml", PROGRAM).unwrap();

    assert_eq!(channel(&conn, "CH-1"), (s("Light A"), s("1"), None));
    // An empty `@Name` is stored as the file states it, like `text`. The
    // unmodelled `TextParameterRefId` keeps its place in `extra`.
    assert_eq!(
        channel(&conn, "CH-2"),
        (s(""), s("B"), s("TextParameterRefId=P-1"))
    );
    assert_eq!(channel(&conn, "CH-3"), (None, None, None));
    assert_eq!(
        channel(&conn, "MD-1_CH-1"),
        (s("Module channel"), s("1/2/3"), None)
    );
    // `ParameterBlock/@Name` is still read by nothing and stays in `extra`.
    let block_extra: Option<String> = conn
        .query_row(
            "SELECT extra FROM dynamic_node WHERE element_id = 'PB-1'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(block_extra.as_deref(), Some("Name=block"));
    // Other kinds never get the columns.
    let others: i64 = conn
        .query_row(
            "SELECT count(*) FROM dynamic_node WHERE kind <> 'Channel' AND (name IS NOT NULL OR number IS NOT NULL)",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(others, 0);
}

#[test]
fn channel_number_is_no_longer_an_unknown_attribute() {
    let dir = tempfile::tempdir().unwrap();
    let conn = open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
    ingest_file(&conn, "M-0001/A.xml", PROGRAM).unwrap();
    // `TextParameterRefId` is still unmodelled, so still reported.
    assert_eq!(
        unknown_names(&conn, PROGRAM_CHANNEL),
        ["TextParameterRefId"]
    );
    let module_channel = "/KNX/ManufacturerData/Manufacturer/ApplicationPrograms/ApplicationProgram/ModuleDefs/ModuleDef/Dynamic/Channel";
    assert!(unknown_names(&conn, module_channel).is_empty());
}

#[test]
fn a_namespaced_number_is_not_mistaken_for_the_modelled_one() {
    // Only the unqualified attribute is modelled; `x:Number` is evidence of
    // something else and must stay reported.
    const PREFIXED: &[u8] = br#"<KNX xmlns="http://knx.org/xml/project/11" xmlns:x="urn:example"><ManufacturerData><Manufacturer RefId="M-0001"><ApplicationPrograms><ApplicationProgram Id="A-2" Name="Program">
<Dynamic><Channel Id="CH-P" Name="n" Number="1" x:Number="9" Text="t"/></Dynamic>
</ApplicationProgram></ApplicationPrograms></Manufacturer></ManufacturerData></KNX>"#;
    let dir = tempfile::tempdir().unwrap();
    let conn = open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
    ingest_file(&conn, "M-0001/A2.xml", PREFIXED).unwrap();
    let (name, number, extra) = channel(&conn, "CH-P");
    assert_eq!((name, number), (s("n"), s("1")));
    assert_eq!(extra.as_deref(), Some("x:Number=9"));
    assert_eq!(unknown_names(&conn, PROGRAM_CHANNEL), ["x:Number"]);
}

#[test]
fn the_evaluated_owner_carries_the_channel_name_and_number() {
    let dir = tempfile::tempdir().unwrap();
    let conn = open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
    ingest_file(&conn, "M-0001/A.xml", PROGRAM).unwrap();

    // Loads with the new columns (the evaluator reads the tree through it).
    load_tree(&conn, "A-1", "").unwrap();

    let trees = load_program_trees(&conn, "A-1").unwrap();
    let activation = evaluate(&trees, &HashMap::<String, String>::new().into());
    let owner = |ref_id: &str| {
        activation
            .com_object_refs
            .iter()
            .find(|r| r.ref_id == ref_id)
            .and_then(|r| r.channel.clone())
            .unwrap_or_else(|| panic!("{ref_id} has no channel"))
    };
    let first = owner("OR-1");
    assert_eq!(
        (first.element_id, first.name, first.number),
        (s("CH-1"), s("Light A"), s("1"))
    );
    let second = owner("OR-2");
    assert_eq!((second.name, second.number), (s(""), s("B")));
}

/// Everything v18 re-derives, in a stable order, without row ids.
#[derive(Debug, PartialEq)]
struct Snapshot {
    version: i64,
    nodes: Vec<Vec<Option<String>>>,
    arguments: Vec<Vec<Option<String>>>,
    unknowns: Vec<Vec<Option<String>>>,
    reports: Vec<Vec<Option<String>>>,
    counts: Vec<Vec<Option<String>>>,
    report_unknowns: Vec<Vec<Option<String>>>,
    diagnostics: Vec<Vec<Option<String>>>,
    unknown_count: Vec<Vec<Option<String>>>,
}

fn rows(conn: &Connection, sql: &str) -> Vec<Vec<Option<String>>> {
    let mut stmt = conn.prepare(sql).unwrap();
    let width = stmt.column_count();
    stmt.query_map([], |r| {
        (0..width)
            .map(|i| {
                let value: rusqlite::types::Value = r.get(i)?;
                Ok(match value {
                    rusqlite::types::Value::Null => None,
                    rusqlite::types::Value::Integer(v) => Some(v.to_string()),
                    rusqlite::types::Value::Real(v) => Some(v.to_string()),
                    rusqlite::types::Value::Text(v) => Some(v),
                    rusqlite::types::Value::Blob(v) => Some(sha256_hex(&v)),
                })
            })
            .collect()
    })
    .unwrap()
    .collect::<Result<_, _>>()
    .unwrap()
}

fn snapshot(conn: &Connection) -> Snapshot {
    Snapshot {
        version: conn
            .query_row("PRAGMA user_version", [], |r| r.get(0))
            .unwrap(),
        nodes: rows(
            conn,
            "SELECT * FROM dynamic_node ORDER BY program_id, module_def_id, node_id",
        ),
        arguments: rows(
            conn,
            "SELECT * FROM module_def_argument ORDER BY program_id, module_def_id, position",
        ),
        unknowns: rows(
            conn,
            "SELECT source_sha256, program_id, xpath, kind, name, occurrences, sample
             FROM ingest_unknown ORDER BY 1, 3, 4, 5, 6, 7",
        ),
        reports: rows(conn, "SELECT * FROM package_install_report ORDER BY 1"),
        counts: rows(conn, "SELECT * FROM package_install_count ORDER BY 1, 2"),
        report_unknowns: rows(conn, "SELECT * FROM package_install_unknown ORDER BY 1, 2"),
        diagnostics: rows(
            conn,
            "SELECT * FROM package_install_diagnostic ORDER BY 1, 2",
        ),
        unknown_count: rows(conn, "SELECT sha256, unknown_count FROM package ORDER BY 1"),
    }
}

fn installed() -> (tempfile::TempDir, std::path::PathBuf, Snapshot) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("products.sqlite");
    let conn = open_and_migrate(&path).unwrap();
    install_package(&conn, "sample.knxprod", &package()).unwrap();
    let fresh = snapshot(&conn);
    (dir, path, fresh)
}

#[test]
fn the_rewind_is_a_genuine_v17_database() {
    // Guards the fixture the migration tests rely on: the rewound report
    // must carry the `Number` rows v17 wrote, and validate.
    let (_dir, path, fresh) = installed();
    let conn = Connection::open(&path).unwrap();
    v18_rewind::rewind_to_v17(&conn);
    let v17 = snapshot(&conn);
    assert_eq!(v17.version, 17);
    assert_eq!(
        v17.unknowns.len(),
        fresh.unknowns.len() + 2,
        "one retired row per channel xpath"
    );
    assert_eq!(v17.report_unknowns.len(), fresh.report_unknowns.len() + 2);
    let extra: Option<String> = conn
        .query_row(
            "SELECT extra FROM dynamic_node WHERE element_id = 'CH-1'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(extra.as_deref(), Some("Name=Light A\nNumber=1"));
}

#[test]
fn v17_to_v18_reproduces_a_fresh_install() {
    let (_dir, path, fresh) = installed();
    {
        let conn = Connection::open(&path).unwrap();
        v18_rewind::rewind_to_v17(&conn);
    }
    let conn = open_and_migrate(&path).unwrap();
    assert_eq!(snapshot(&conn), fresh);
    // And the migrated report still loads as `measured`.
    let retry = install_package(&conn, "sample.knxprod", &package()).unwrap();
    assert!(retry.skipped);
    assert!(retry.facts.is_some());
}

#[test]
fn v17_to_v18_keeps_unknown_rows_it_does_not_retire() {
    // Rows under `…/Dynamic/…` that the dynamic pass would not report (as
    // scheme evidence reconciliation writes them) must survive: v18 does
    // not rebuild those rows from a parse, unlike v11.
    let (_dir, path, _fresh) = installed();
    let program_sha = sha256_hex(PROGRAM);
    {
        let conn = Connection::open(&path).unwrap();
        v18_rewind::rewind_to_v17(&conn);
        conn.execute(
            "INSERT INTO ingest_unknown (source_sha256, program_id, xpath, kind, name, occurrences, sample)
             VALUES (?1, NULL, ?2, 'Attribute', 'x:Evidence', 1, 'kept')",
            rusqlite::params![program_sha, PROGRAM_CHANNEL],
        )
        .unwrap();
    }
    let conn = open_and_migrate(&path).unwrap();
    assert_eq!(
        unknown_names(&conn, PROGRAM_CHANNEL),
        ["TextParameterRefId", "x:Evidence"]
    );
}

#[test]
fn v17_to_v18_leaves_a_damaged_blob_and_its_report_honest() {
    let (_dir, path, _fresh) = installed();
    let program_sha = sha256_hex(PROGRAM);
    {
        let conn = Connection::open(&path).unwrap();
        v18_rewind::rewind_to_v17(&conn);
        let mut damaged = PROGRAM.to_vec();
        let at = damaged.len() - 10;
        damaged[at] ^= 0x20;
        conn.execute(
            "UPDATE source_file SET bytes = ?1 WHERE sha256 = ?2",
            rusqlite::params![damaged, program_sha],
        )
        .unwrap();
    }
    let conn = open_and_migrate(&path).unwrap();

    // The blob keeps its v17 rows: no values, retired rows still there.
    assert_eq!(channel(&conn, "CH-1").0, None);
    assert!(unknown_names(&conn, PROGRAM_CHANNEL).contains(&"Number".to_string()));
    let failure: String = conn
        .query_row(
            "SELECT sample FROM ingest_unknown WHERE source_sha256 = ?1 AND kind = 'ChannelNameBackfillError'",
            [&program_sha],
            |r| r.get(0),
        )
        .unwrap();
    assert!(
        failure.contains("do not match their SHA-256 key"),
        "{failure}"
    );

    // Its package report is not carried forward, and says so.
    let status: String = conn
        .query_row("SELECT status FROM package_install_report", [], |r| {
            r.get(0)
        })
        .unwrap();
    assert_eq!(status, "unavailable");
    let recorded: i64 = conn
        .query_row(
            "SELECT count(*) FROM ingest_unknown WHERE kind = 'InstallReportBackfillError'
               AND name = 'channel_number_report_backfill'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(recorded, 1);
}

#[test]
fn v17_to_v18_records_a_damaged_program_that_no_longer_classifies() {
    let (_dir, path, _fresh) = installed();
    let program_sha = sha256_hex(PROGRAM);
    {
        let conn = Connection::open(&path).unwrap();
        v18_rewind::rewind_to_v17(&conn);
        conn.execute(
            "UPDATE source_file SET bytes = x'00' WHERE sha256 = ?1",
            [&program_sha],
        )
        .unwrap();
    }
    let conn = open_and_migrate(&path).unwrap();
    assert_eq!(channel(&conn, "CH-1").0, None);
    assert!(unknown_names(&conn, PROGRAM_CHANNEL).contains(&"Number".to_string()));
    let errors: i64 = conn
        .query_row(
            "SELECT count(*) FROM ingest_unknown WHERE source_sha256 = ?1
               AND kind = 'ChannelNameBackfillError'",
            [&program_sha],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(errors, 1, "a known program must not vanish silently");
}

#[test]
fn v17_to_v18_rejects_a_package_counter_that_would_underflow() {
    let (_dir, path, _fresh) = installed();
    {
        let conn = Connection::open(&path).unwrap();
        v18_rewind::rewind_to_v17(&conn);
        conn.execute("UPDATE package SET unknown_count = 0", [])
            .unwrap();
    }
    assert!(open_and_migrate(&path).is_err());
    let conn = Connection::open(&path).unwrap();
    let version: i64 = conn
        .query_row("PRAGMA user_version", [], |r| r.get(0))
        .unwrap();
    assert_eq!(version, 17, "failed migration rolls back atomically");
}

#[test]
fn v17_to_v18_downgrades_a_damaged_report_not_the_whole_database() {
    let (_dir, path, _fresh) = installed();
    {
        let conn = Connection::open(&path).unwrap();
        v18_rewind::rewind_to_v17(&conn);
        conn.execute(
            "UPDATE package_install_report SET unknown_occurrences = unknown_occurrences + 1",
            [],
        )
        .unwrap();
    }
    let conn = open_and_migrate(&path).unwrap();
    let status: String = conn
        .query_row("SELECT status FROM package_install_report", [], |r| {
            r.get(0)
        })
        .unwrap();
    assert_eq!(status, "unavailable");
    assert_eq!(channel(&conn, "CH-1").0.as_deref(), Some("Light A"));
    let errors: i64 = conn
        .query_row(
            "SELECT count(*) FROM ingest_unknown WHERE kind = 'InstallReportBackfillError'
              AND name = 'channel_number_report_backfill'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    let error_reason: String = conn.query_row(
        "SELECT sample FROM ingest_unknown WHERE kind = 'InstallReportBackfillError' AND name = 'channel_number_report_backfill'",
        [],
        |r| r.get(0),
    ).unwrap();
    assert!(
        error_reason.contains("header/detail mismatch"),
        "{error_reason}"
    );
    assert_eq!(errors, 1);
}

#[test]
fn v17_to_v18_does_not_invent_trees_for_a_blob_that_owns_none() {
    // A blob that lost its program id to another file stored no trees in
    // v17; the backfill must not give it any.
    const LOSER: &[u8] = br#"<KNX xmlns="http://knx.org/xml/project/11"><ManufacturerData><Manufacturer RefId="M-0001"><ApplicationPrograms><ApplicationProgram Id="A-1" Name="Other">
<Dynamic><Channel Id="CH-LOSER" Name="loser" Number="7" Text="t"/></Dynamic>
</ApplicationProgram></ApplicationPrograms></Manufacturer></ManufacturerData></KNX>"#;
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("products.sqlite");
    let fresh = {
        let conn = open_and_migrate(&path).unwrap();
        ingest_file(&conn, "M-0001/A.xml", PROGRAM).unwrap();
        ingest_file(&conn, "M-0001/A-other.xml", LOSER).unwrap();
        let fresh = snapshot(&conn);
        v18_rewind::rewind_to_v17(&conn);
        // The dynamic pass reported the loser's `@Number` although it
        // stored nothing; the rewind cannot see that, so add it as v17 did.
        conn.execute(
            "INSERT INTO ingest_unknown (source_sha256, program_id, xpath, kind, name, occurrences, sample)
             VALUES (?1, NULL, ?2, 'Attribute', 'Number', 1, '7')",
            rusqlite::params![sha256_hex(LOSER), PROGRAM_CHANNEL],
        )
        .unwrap();
        fresh
    };
    let conn = open_and_migrate(&path).unwrap();
    let loser: i64 = conn
        .query_row(
            "SELECT count(*) FROM dynamic_node WHERE element_id = 'CH-LOSER'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(loser, 0);
    assert_eq!(snapshot(&conn), fresh);
}
