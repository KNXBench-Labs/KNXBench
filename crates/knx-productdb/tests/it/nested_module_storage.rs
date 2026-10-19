//! Synthetic nested ModuleDef storage regressions; no manufacturer runtime semantics.

use std::io::{Cursor, Write};

use knx_productdb::{install_package, load_source_file, open_and_migrate, sha256_hex};
use rusqlite::Connection;
use zip::write::SimpleFileOptions;

const PROGRAM_ID: &str = "M-00FA_A-0001-01-0000";
const MASTER: &[u8] = br#"<KNX xmlns="http://knx.org/xml/project/20"><MasterData><Manufacturers><Manufacturer Id="M-00FA" Name="Synthetic"/></Manufacturers></MasterData></KNX>"#;

type NodeRow = (i64, Option<i64>, i64, String, Option<String>);
type DatabaseRows = Vec<(String, Vec<Vec<rusqlite::types::Value>>)>;

fn db() -> (tempfile::TempDir, Connection) {
    let dir = tempfile::tempdir().unwrap();
    let conn = open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
    (dir, conn)
}

fn program(body: &str) -> Vec<u8> {
    format!(
        r#"<KNX xmlns="http://knx.org/xml/project/20"><ManufacturerData><Manufacturer RefId="M-00FA"><ApplicationPrograms><ApplicationProgram Id="{PROGRAM_ID}" Name="Synthetic" ApplicationVersion="1" MaskVersion="MV-0701"><Static><ComObjectTable/><ComObjectRefs/></Static>{body}</ApplicationProgram></ApplicationPrograms></Manufacturer></ManufacturerData></KNX>"#
    )
    .into_bytes()
}

fn package(program: &[u8]) -> Vec<u8> {
    let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for (name, bytes) in [("knx_master.xml", MASTER), ("M-00FA/Program.xml", program)] {
        zip.start_file(name, SimpleFileOptions::default()).unwrap();
        zip.write_all(bytes).unwrap();
    }
    zip.finish().unwrap().into_inner()
}

fn scope_nodes(conn: &Connection, scope: &str) -> Vec<NodeRow> {
    conn.prepare(
        "SELECT node_id, parent_id, position, kind, element_id FROM dynamic_node
         WHERE program_id = ?1 AND module_def_id = ?2 ORDER BY node_id",
    )
    .unwrap()
    .query_map([PROGRAM_ID, scope], |row| {
        Ok((
            row.get(0)?,
            row.get(1)?,
            row.get(2)?,
            row.get(3)?,
            row.get(4)?,
        ))
    })
    .unwrap()
    .collect::<Result<_, _>>()
    .unwrap()
}

fn assert_channel_scope(conn: &Connection, scope: &str, channel: &str) {
    assert_eq!(
        scope_nodes(conn, scope),
        vec![
            (0, None, 0, "Dynamic".to_string(), None),
            (
                1,
                Some(0),
                0,
                "Channel".to_string(),
                Some(channel.to_string())
            ),
        ]
    );
}

#[test]
fn nested_module_definitions_install_with_separate_dynamic_keys() {
    let (_dir, conn) = db();
    let source = program(
        r#"<Dynamic><Channel Id="program-channel"/></Dynamic>
        <ModuleDefs><ModuleDef Id="outer"><ModuleDefs><ModuleDef Id="inner">
        <Dynamic><Channel Id="inner-channel"/></Dynamic>
        </ModuleDef></ModuleDefs><Dynamic><Channel Id="outer-channel"/></Dynamic>
        </ModuleDef></ModuleDefs>"#,
    );
    let bytes = package(&source);
    let report = install_package(&conn, "nested.knxprod", &bytes)
        .expect("nested definitions must not collide with the program's dynamic keys");
    assert!(!report.skipped);
    assert_channel_scope(&conn, "", "program-channel");
    assert_channel_scope(&conn, "outer", "outer-channel");
    assert_channel_scope(&conn, "inner", "inner-channel");
    assert_eq!(
        conn.query_row("SELECT count(*) FROM dynamic_node", [], |row| row
            .get::<_, i64>(0))
            .unwrap(),
        6
    );
    assert_eq!(
        load_source_file(&conn, &sha256_hex(&source)).unwrap(),
        Some(source)
    );
    assert!(
        install_package(&conn, "retry.KNXPROD", &bytes)
            .unwrap()
            .skipped
    );
    assert_channel_scope(&conn, "outer", "outer-channel");
}

#[test]
fn empty_nested_definitions_restore_the_enclosing_scope() {
    for empty in [
        r#"<ModuleDef Id="empty"/>"#,
        r#"<ModuleDef Id="empty"></ModuleDef>"#,
    ] {
        let (_dir, conn) = db();
        let source = program(&format!(
            r#"<ModuleDefs><ModuleDef Id="outer"><ModuleDefs>{empty}</ModuleDefs>
            <Dynamic><Channel Id="outer-channel"/></Dynamic></ModuleDef></ModuleDefs>
            <Dynamic><Channel Id="program-channel"/></Dynamic>"#
        ));
        install_package(&conn, "empty-nested.knxprod", &package(&source)).unwrap();
        assert_channel_scope(&conn, "outer", "outer-channel");
        assert_channel_scope(&conn, "", "program-channel");
        assert!(scope_nodes(&conn, "empty").is_empty());
    }
}

#[test]
fn deeper_and_sibling_definitions_keep_their_own_trees() {
    let (_dir, conn) = db();
    let source = program(
        r#"<ModuleDefs><ModuleDef Id="outer"><ModuleDefs>
        <ModuleDef Id="middle"><ModuleDefs><ModuleDef Id="inner">
        <Dynamic><Channel Id="inner-channel"/></Dynamic></ModuleDef></ModuleDefs>
        <Dynamic><Channel Id="middle-channel"/></Dynamic></ModuleDef>
        <ModuleDef Id="sibling"><Dynamic><Channel Id="sibling-channel"/></Dynamic></ModuleDef>
        </ModuleDefs><Dynamic><Channel Id="outer-channel"/></Dynamic></ModuleDef></ModuleDefs>
        <Dynamic><Channel Id="program-channel"/></Dynamic>"#,
    );
    install_package(&conn, "deeper.knxprod", &package(&source)).unwrap();
    for (scope, channel) in [
        ("", "program-channel"),
        ("outer", "outer-channel"),
        ("middle", "middle-channel"),
        ("inner", "inner-channel"),
        ("sibling", "sibling-channel"),
    ] {
        assert_channel_scope(&conn, scope, channel);
    }
    assert_eq!(
        conn.query_row("SELECT count(*) FROM dynamic_node", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        10
    );
}

#[test]
fn enclosing_argument_positions_resume_after_nested_definitions() {
    let (_dir, conn) = db();
    // Split declaration containers deliberately exercise lexical storage state,
    // not an assertion that a manufacturer XSD admits this synthetic layout.
    let source = program(
        r#"<ModuleDefs><ModuleDef Id="outer">
        <Arguments><Argument Id="outer-first" Name="First"/></Arguments>
        <ModuleDefs><ModuleDef Id="inner">
        <Arguments><Argument Id="inner-first" Name="Inner"/></Arguments>
        </ModuleDef><ModuleDef Id="empty"/></ModuleDefs>
        <Arguments><Argument Id="outer-second" Name="Second"/></Arguments>
        </ModuleDef></ModuleDefs>"#,
    );
    install_package(&conn, "arguments.knxprod", &package(&source)).unwrap();
    let rows: Vec<(String, String, i64)> = conn
        .prepare("SELECT module_def_id, id, position FROM module_def_argument ORDER BY module_def_id, position")
        .unwrap()
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    assert_eq!(
        rows,
        vec![
            ("inner".into(), "inner-first".into(), 0),
            ("outer".into(), "outer-first".into(), 0),
            ("outer".into(), "outer-second".into(), 1),
        ]
    );
}

fn database_rows(conn: &Connection) -> DatabaseRows {
    let tables: Vec<String> = conn
        .prepare("SELECT name FROM sqlite_schema WHERE type = 'table' ORDER BY name")
        .unwrap()
        .query_map([], |r| r.get(0))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    tables
        .into_iter()
        .map(|name| {
            let mut statement = conn
                .prepare(&format!("SELECT * FROM \"{}\"", name.replace('"', "\"\"")))
                .unwrap();
            let column_count = statement.column_count();
            let rows = statement
                .query_map([], |row| {
                    (0..column_count)
                        .map(|i| row.get(i))
                        .collect::<Result<Vec<rusqlite::types::Value>, _>>()
                })
                .unwrap()
                .collect::<Result<Vec<_>, _>>()
                .unwrap();
            (name, rows)
        })
        .collect()
}

#[test]
fn duplicate_nested_keys_and_malformed_xml_roll_back_the_whole_package() {
    let (_dir, conn) = db();
    let seed = program(r#"<Dynamic><Channel Id="seed-channel"/></Dynamic>"#);
    install_package(&conn, "seed.knxprod", &package(&seed)).unwrap();
    assert_channel_scope(&conn, "", "seed-channel");
    assert_eq!(
        load_source_file(&conn, &sha256_hex(&seed)).unwrap(),
        Some(seed.clone())
    );
    assert_eq!(
        conn.query_row("SELECT count(*) FROM package", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        1
    );
    let before = database_rows(&conn);
    for (index, body) in [
        r#"<ModuleDefs><ModuleDef Id="outer"><ModuleDefs>
        <ModuleDef Id="same"><Dynamic><Channel Id="first"/></Dynamic></ModuleDef>
        <ModuleDef Id="same"><Dynamic><Channel Id="second"/></Dynamic></ModuleDef>
        </ModuleDefs></ModuleDef></ModuleDefs>"#,
        r#"<ModuleDefs><ModuleDef Id="outer"><ModuleDefs><ModuleDef Id="broken"></ModuleDefs>"#,
    ]
    .into_iter()
    .enumerate()
    {
        let bad = String::from_utf8(program(body))
            .unwrap()
            .replace(PROGRAM_ID, "M-00FA_A-0002-01-0000");
        let error =
            install_package(&conn, "refused.knxprod", &package(bad.as_bytes())).unwrap_err();
        assert!(matches!(error, knx_productdb::PackageError::Database(_)));
        if index == 0 {
            assert!(error.to_string().contains("UNIQUE constraint failed: dynamic_node.program_id, dynamic_node.module_def_id, dynamic_node.node_id"));
        } else {
            assert!(matches!(
                error,
                knx_productdb::PackageError::Database(knx_productdb::ProductDbError::Xml { .. })
            ));
        }
        assert!(conn.is_autocommit());
        assert!(
            database_rows(&conn) == before,
            "refusal must preserve every seeded table row"
        );
        assert_eq!(
            load_source_file(&conn, &sha256_hex(&seed)).unwrap(),
            Some(seed.clone())
        );
    }
}

#[test]
fn migration_style_replay_rebuilds_the_same_scoped_rows() {
    let (_dir, conn) = db();
    let source = program(
        r#"<Dynamic><Channel Id="program-channel"/></Dynamic>
        <ModuleDefs><ModuleDef Id="outer"><ModuleDefs><ModuleDef Id="inner">
        <Dynamic><Channel Id="inner-channel"/></Dynamic></ModuleDef></ModuleDefs>
        <Dynamic><Channel Id="outer-channel"/></Dynamic></ModuleDef></ModuleDefs>"#,
    );
    install_package(&conn, "replay.knxprod", &package(&source)).unwrap();
    let before = database_rows(&conn);
    let transaction = conn.unchecked_transaction().unwrap();
    transaction
        .execute(
            "DELETE FROM dynamic_node WHERE program_id = ?1",
            [PROGRAM_ID],
        )
        .unwrap();
    knx_productdb::dynamic::parse::parse_dynamic_trees(
        &transaction,
        &sha256_hex(&source),
        "M-00FA/Program.xml",
        &source,
    )
    .unwrap();
    transaction.commit().unwrap();
    assert!(
        database_rows(&conn) == before,
        "replay must rebuild the same rows without changing retained evidence"
    );
}

#[test]
fn two_programs_keep_reused_module_ids_separate() {
    let (_dir, conn) = db();
    let body = r#"<ModuleDefs><ModuleDef Id="outer"><ModuleDefs><ModuleDef Id="inner">
        <Dynamic><Channel Id="inner-channel"/></Dynamic></ModuleDef></ModuleDefs>
        <Dynamic><Channel Id="outer-channel"/></Dynamic></ModuleDef></ModuleDefs>
        <Dynamic><Channel Id="program-channel"/></Dynamic>"#;
    let source = program(&format!(
        r#"{body}</ApplicationProgram><ApplicationProgram Id="second" Name="Second" ApplicationVersion="1" MaskVersion="MV-0701"><Static/>{body}"#
    ));
    install_package(&conn, "two-programs.knxprod", &package(&source)).unwrap();
    for program_id in [PROGRAM_ID, "second"] {
        for (scope, channel) in [
            ("", "program-channel"),
            ("inner", "inner-channel"),
            ("outer", "outer-channel"),
        ] {
            let identity: String = conn.query_row(
                "SELECT element_id FROM dynamic_node WHERE program_id = ?1 AND module_def_id = ?2 AND node_id = 1",
                [program_id, scope], |r| r.get(0),
            ).unwrap();
            assert_eq!(identity, channel);
            assert_eq!(conn.query_row(
                "SELECT count(*) FROM dynamic_node WHERE program_id = ?1 AND module_def_id = ?2",
                [program_id, scope], |r| r.get::<_, i64>(0),
            ).unwrap(), 2);
        }
    }
    assert_eq!(
        conn.query_row("SELECT count(*) FROM dynamic_node", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        12
    );
}
