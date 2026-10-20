//! v28 already-upgraded and failure-boundary fixtures, independent of rewind DDL.
use std::io::{Cursor, Write};
use std::path::PathBuf;

use knx_productdb::{install_package, open_and_migrate, sha256_hex, CURRENT_PRODUCTDB_VERSION};
use rusqlite::{params, types::Value, Connection};
use zip::write::SimpleFileOptions;

const MASTER: &str = r#"<KNX xmlns="http://knx.org/xml/project/20"><MasterData><Manufacturers><Manufacturer Id="M-0001" Name="Example"/></Manufacturers><DatapointTypes><DatapointType Number="1"/><DatapointType Id="D" Number="2"/></DatapointTypes><Unmodelled/></MasterData></KNX>"#;
const ROOT: &str = "/KNX/ManufacturerData/Manufacturer/ApplicationPrograms/ApplicationProgram";

fn xml(program: &str) -> String {
    format!(
        r#"<KNX xmlns="http://knx.org/xml/project/20"><ManufacturerData><Manufacturer RefId="M-0001"><ApplicationPrograms>{program}</ApplicationPrograms></Manufacturer></ManufacturerData></KNX>"#
    )
}

fn package(members: &[(&str, &str)]) -> Vec<u8> {
    package_with_master(MASTER, members)
}

fn package_with_master(master: &str, members: &[(&str, &str)]) -> Vec<u8> {
    let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for (path, bytes) in std::iter::once(("knx_master.xml", master)).chain(members.iter().copied())
    {
        zip.start_file(path, SimpleFileOptions::default()).unwrap();
        zip.write_all(bytes.as_bytes()).unwrap();
    }
    zip.finish().unwrap().into_inner()
}

fn installed(program: &str) -> (tempfile::TempDir, PathBuf, Connection, String) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("products.sqlite");
    let conn = open_and_migrate(&path).unwrap();
    let xml = xml(program);
    install_package(
        &conn,
        "synthetic.knxprod",
        &package(&[("M-0001/A.xml", &xml)]),
    )
    .unwrap();
    (dir, path, conn, sha256_hex(xml.as_bytes()))
}

fn rows(conn: &Connection, sql: &str) -> Vec<Vec<Value>> {
    let mut stmt = conn.prepare(sql).unwrap();
    let n = stmt.column_count();
    stmt.query_map([], |r| (0..n).map(|i| r.get(i)).collect())
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap()
}

fn preserved(conn: &Connection) -> Vec<Vec<Vec<Value>>> {
    ["SELECT * FROM master_source ORDER BY source_sha256",
     "SELECT * FROM master_declaration ORDER BY source_sha256,ordinal",
     "SELECT * FROM master_winner ORDER BY family,logical_id",
     "SELECT * FROM package_install_count WHERE category<>'unknown_construct' ORDER BY package_sha256,ordinal",
     "SELECT * FROM package_install_diagnostic ORDER BY package_sha256,ordinal",
     "SELECT * FROM package_member ORDER BY package_sha256,ordinal",
     "SELECT * FROM source_file ORDER BY sha256",
     "SELECT * FROM package_conflict ORDER BY package_sha256,ordinal",
     "SELECT * FROM parameter_ref ORDER BY program_id,id"].iter().map(|s|rows(conn,s)).collect()
}

fn caption(conn: &Connection) -> Option<String> {
    conn.query_row(
        "SELECT text_parameter_ref_id FROM com_object_ref",
        [],
        |r| r.get(0),
    )
    .unwrap()
}

const OBJECT: &str = r#"<ApplicationProgram Id="A"><Static><ComObjectRefs><ComObjectRef Id="R" RefId="O" TextParameterRefId="right"/></ComObjectRefs><Parameters><Parameter Id="P" Offset="2"/></Parameters><ParameterBlock Access="Read"/></Static></ApplicationProgram>"#;

#[test]
fn v27_failed_derivation_does_not_partially_correct_columns() {
    let (_dir, path, conn, _) = installed(OBJECT);
    conn.execute_batch(
        "UPDATE com_object_ref SET text_parameter_ref_id='contaminated';
        INSERT INTO parameter(program_id,id,own_offset) VALUES('A','not-in-source',99);
        PRAGMA user_version=27;",
    )
    .unwrap();
    let original = rows(&conn, "SELECT * FROM parameter ORDER BY id");
    let unknown = rows(&conn, "SELECT * FROM ingest_unknown ORDER BY id");
    drop(conn);
    let conn = open_and_migrate(&path).unwrap();
    assert_eq!(
        caption(&conn).as_deref(),
        Some("contaminated"),
        "source failure must undo earlier column repairs"
    );
    assert_eq!(rows(&conn, "SELECT * FROM parameter ORDER BY id"), original);
    let final_unknown = rows(&conn, "SELECT * FROM ingest_unknown ORDER BY id");
    assert!(final_unknown.starts_with(&unknown));
    assert_eq!(
        rows(&conn, "SELECT status FROM package_install_report"),
        vec![vec![Value::Text("unavailable".into())]]
    );
    assert_eq!(
        rows(
            &conn,
            "SELECT kind FROM ingest_unknown WHERE kind='PromotionRepairError'"
        )
        .len(),
        1
    );
}

#[test]
fn shared_source_reports_repair_proven_history_and_refuse_ambiguous_members() {
    let shared = xml(
        r#"<ApplicationProgram Id="A"><Static><ParameterBlock Access="shared"/></Static></ApplicationProgram>"#,
    );
    let other = xml(
        r#"<ApplicationProgram Id="B"><Static><ParameterBlock Access="second"/><ParameterBlock Access="third"/></Static></ApplicationProgram>"#,
    );
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("products.sqlite");
    let conn = open_and_migrate(&path).unwrap();
    for members in [
        vec![
            ("M-0001/first.xml", shared.as_str()),
            ("M-0001/second.xml", other.as_str()),
        ],
        vec![("M-0001/first.xml", shared.as_str())],
    ] {
        install_package(&conn, "synthetic.knxprod", &package(&members)).unwrap();
    }
    let expected = rows(
        &conn,
        "SELECT * FROM package_install_unknown ORDER BY package_sha256,ordinal",
    );
    let counts = rows(
        &conn,
        "SELECT sha256,unknown_count FROM package ORDER BY sha256",
    );
    let history = preserved(&conn);
    assert!(!history[0].is_empty());
    assert!(!history[4].is_empty());
    let ambiguous: String = conn.query_row(
        "SELECT package_sha256 FROM package_member WHERE role='ApplicationProgram' GROUP BY package_sha256 HAVING count(*)=2",
        [], |r| r.get(0),
    ).unwrap();
    // The global ledger is already present, but BOTH package reports lost the
    // targeted rows. A global insert delta is deliberately zero. The shared
    // aggregate does not prove which member counter units were retired; only
    // the one-program report can be reconstructed without guessing history.
    let hashes = rows(&conn, "SELECT sha256 FROM package ORDER BY sha256");
    for hash in hashes {
        let Value::Text(hash) = &hash[0] else {
            panic!()
        };
        conn.execute(
            "DELETE FROM package_install_unknown WHERE package_sha256=?1 AND name='Access'",
            [hash],
        )
        .unwrap();
        conn.execute("UPDATE package_install_report SET unknown_distinct=(SELECT count(*) FROM package_install_unknown WHERE package_sha256=?1),unknown_occurrences=(SELECT coalesce(sum(occurrences),0) FROM package_install_unknown WHERE package_sha256=?1) WHERE package_sha256=?1",[hash]).unwrap();
        conn.execute("UPDATE package_install_count SET count=(SELECT CASE ?2 WHEN 'read' THEN coalesce(sum(occurrences),0) ELSE count(*) END FROM package_install_unknown WHERE package_sha256=?1) WHERE package_sha256=?1 AND category='unknown_construct' AND disposition=?2",params![hash,"read"]).unwrap();
        conn.execute("UPDATE package_install_count SET count=(SELECT count(*) FROM package_install_unknown WHERE package_sha256=?1) WHERE package_sha256=?1 AND category='unknown_construct' AND disposition='stored'",[hash]).unwrap();
        conn.execute("UPDATE package SET unknown_count=unknown_count-(SELECT count(*) FROM package_member WHERE package_sha256=?1 AND role='ApplicationProgram') WHERE sha256=?1",[hash]).unwrap();
    }
    let retired_counts = rows(
        &conn,
        "SELECT sha256,unknown_count FROM package ORDER BY sha256",
    );
    let counts: Vec<_> = counts
        .into_iter()
        .zip(retired_counts)
        .map(|(original, retired)| {
            if original[0] == Value::Text(ambiguous.clone()) {
                retired
            } else {
                original
            }
        })
        .collect();
    let expected: Vec<_> = expected
        .into_iter()
        .filter(|r| r[0] != Value::Text(ambiguous.clone()))
        .collect();
    let mut history = history;
    // Established unavailable reports intentionally have no details. Retain
    // every unaffected report and all source/provenance/conflict rows exactly.
    for index in [3, 4] {
        history[index].retain(|r| r[0] != Value::Text(ambiguous.clone()));
    }
    conn.pragma_update(None, "user_version", 27).unwrap();
    drop(conn);
    let conn = open_and_migrate(&path).unwrap();
    assert_eq!(
        rows(
            &conn,
            "SELECT * FROM package_install_unknown ORDER BY package_sha256,ordinal"
        ),
        expected
    );
    assert_eq!(
        rows(
            &conn,
            "SELECT sha256,unknown_count FROM package ORDER BY sha256"
        ),
        counts
    );
    let status: String = conn
        .query_row(
            "SELECT status FROM package_install_report WHERE package_sha256=?1",
            [&ambiguous],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(status, "unavailable");
    let diagnostic: String = conn.query_row("SELECT sample FROM ingest_unknown WHERE source_sha256=?1 AND kind='PromotionRepairError'", [&ambiguous], |r| r.get(0)).unwrap();
    assert!(diagnostic.contains("ambiguous per-member promotion counter history"));
    assert_eq!(preserved(&conn), history);
    let access = rows(&conn,"SELECT occurrences,sample FROM package_install_unknown WHERE name='Access' ORDER BY occurrences");
    assert_eq!(
        access,
        vec![vec![Value::Integer(1), Value::Text("shared".into())]]
    );
    let report = rows(
        &conn,
        "SELECT * FROM package_install_report ORDER BY package_sha256",
    );
    let evidence = rows(&conn, "SELECT * FROM ingest_unknown ORDER BY id");
    drop(conn);
    let conn = open_and_migrate(&path).unwrap();
    assert_eq!(preserved(&conn), history);
    assert_eq!(
        rows(
            &conn,
            "SELECT * FROM package_install_report ORDER BY package_sha256"
        ),
        report
    );
    assert_eq!(
        rows(&conn, "SELECT * FROM ingest_unknown ORDER BY id"),
        evidence
    );
}

#[test]
fn corrupt_known_sources_preserve_rows_and_name_each_failure() {
    for fault in ["missing", "hash", "length", "classification", "xml"] {
        let (_dir, path, conn, mut sha) = installed(OBJECT);
        conn.execute(
            "UPDATE com_object_ref SET text_parameter_ref_id='contaminated'",
            [],
        )
        .unwrap();
        conn.execute_batch("PRAGMA foreign_keys=OFF").unwrap();
        match fault {
            "missing" => {
                conn.execute("DELETE FROM source_file WHERE sha256=?1", [&sha])
                    .unwrap();
            }
            "hash" => {
                let bytes: Vec<u8> = conn
                    .query_row(
                        "SELECT bytes FROM source_file WHERE sha256=?1",
                        [&sha],
                        |r| r.get(0),
                    )
                    .unwrap();
                let damaged = String::from_utf8(bytes)
                    .unwrap()
                    .replace("right", "wrong")
                    .into_bytes();
                conn.execute(
                    "UPDATE source_file SET bytes=?2 WHERE sha256=?1",
                    params![sha, damaged],
                )
                .unwrap();
            }
            "length" => {
                conn.execute("UPDATE source_file SET len=len+1 WHERE sha256=?1", [&sha])
                    .unwrap();
            }
            _ => {
                let bad = if fault == "classification" {
                    b"not XML".as_slice()
                } else {
                    b"<KNX><ManufacturerData><Manufacturer><ApplicationPrograms><ApplicationProgram Id=\"A\">".as_slice()
                };
                let new_sha = sha256_hex(bad);
                conn.execute("INSERT INTO source_file(sha256,source_path,len,bytes) VALUES(?1,'M-0001/bad.xml',?2,?3)",params![new_sha,bad.len() as i64,bad]).unwrap();
                conn.execute(
                    "UPDATE application_program SET source_sha256=?1 WHERE source_sha256=?2",
                    params![new_sha, sha],
                )
                .unwrap();
                conn.execute(
                    "UPDATE package_member SET source_sha256=?1 WHERE source_sha256=?2",
                    params![new_sha, sha],
                )
                .unwrap();
                conn.execute(
                    "UPDATE ingest_unknown SET source_sha256=?1 WHERE source_sha256=?2",
                    params![new_sha, sha],
                )
                .unwrap();
                conn.execute("DELETE FROM source_file WHERE sha256=?1", [&sha])
                    .unwrap();
                sha = new_sha;
            }
        }
        let data = rows(&conn, "SELECT * FROM parameter");
        let evidence = rows(&conn, "SELECT * FROM ingest_unknown ORDER BY id");
        let sources = rows(&conn, "SELECT * FROM source_file ORDER BY sha256");
        conn.pragma_update(None, "user_version", 27).unwrap();
        drop(conn);
        let conn = open_and_migrate(&path).unwrap();
        assert_eq!(caption(&conn).as_deref(), Some("contaminated"), "{fault}");
        assert_eq!(rows(&conn, "SELECT * FROM parameter"), data, "{fault}");
        assert!(
            rows(&conn, "SELECT * FROM ingest_unknown ORDER BY id").starts_with(&evidence),
            "{fault}"
        );
        assert_eq!(
            rows(&conn, "SELECT * FROM source_file ORDER BY sha256"),
            sources,
            "{fault}"
        );
        let diagnostic: String = conn.query_row("SELECT sample FROM ingest_unknown WHERE source_sha256=?1 AND kind='PromotionRepairError'",[sha],|r|r.get(0)).unwrap();
        let expected = match fault {
            "missing" => "missing retained",
            "hash" => "SHA-256",
            "length" => "length mismatch",
            "classification" => "no longer classifies",
            _ => "XML",
        };
        assert!(diagnostic.contains(expected), "{fault}: {diagnostic}");
        assert_eq!(
            rows(&conn, "SELECT status FROM package_install_report"),
            vec![vec![Value::Text("unavailable".into())]]
        );
        assert_eq!(
            conn.pragma_query_value(None, "user_version", |r| r.get::<_, i64>(0))
                .unwrap(),
            CURRENT_PRODUCTDB_VERSION
        );
    }
}

#[test]
fn missing_and_unavailable_reports_keep_their_availability() {
    for missing in [true, false] {
        let (_dir, path, conn, _) = installed(OBJECT);
        conn.execute_batch(
            "DELETE FROM package_install_unknown; DELETE FROM package_install_count;
            DELETE FROM package_install_diagnostic;",
        )
        .unwrap();
        if missing {
            conn.execute("DELETE FROM package_install_report", [])
                .unwrap();
        } else {
            conn.execute("UPDATE package_install_report SET status='unavailable',unknown_distinct=0,unknown_occurrences=0",[]).unwrap();
        }
        let before = rows(&conn, "SELECT * FROM package_install_report");
        conn.execute_batch("UPDATE com_object_ref SET text_parameter_ref_id='contaminated'; PRAGMA user_version=27;").unwrap();
        drop(conn);
        let conn = open_and_migrate(&path).unwrap();
        assert_eq!(caption(&conn).as_deref(), Some("right"));
        assert_eq!(rows(&conn, "SELECT * FROM package_install_report"), before);
        assert!(rows(&conn, "SELECT * FROM package_install_count").is_empty());
    }
}

#[test]
fn malformed_measured_history_is_not_silently_validated_by_rewrite() {
    let (_dir, path, conn, _) = installed(OBJECT);
    conn.execute_batch("UPDATE package_install_count SET count=count+1 WHERE category='application_program' AND disposition='read'; PRAGMA user_version=27;").unwrap();
    drop(conn);
    let conn = open_and_migrate(&path).unwrap();
    assert_eq!(
        rows(&conn, "SELECT status FROM package_install_report"),
        vec![vec![Value::Text("unavailable".into())]]
    );
    let reason: String = conn
        .query_row(
            "SELECT sample FROM ingest_unknown WHERE xpath='promotion_report_backfill'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert!(reason.contains("outcome mismatch"), "{reason}");
}

#[test]
fn late_sql_failure_rolls_back_columns_evidence_reports_and_version() {
    let (_dir, path, conn, _) = installed(OBJECT);
    conn.execute_batch("UPDATE com_object_ref SET text_parameter_ref_id='contaminated';
        DELETE FROM ingest_unknown WHERE name='Access';
        CREATE TRIGGER refuse_report BEFORE INSERT ON package_install_report BEGIN SELECT RAISE(ABORT,'injected v28 report failure'); END;
        PRAGMA user_version=27;").unwrap();
    let before = preserved(&conn);
    let evidence = rows(&conn, "SELECT * FROM ingest_unknown ORDER BY id");
    let reports = rows(&conn, "SELECT * FROM package_install_report");
    drop(conn);
    let error = open_and_migrate(&path).unwrap_err();
    assert!(
        error.to_string().contains("injected v28 report failure"),
        "{error}"
    );
    let conn = Connection::open(&path).unwrap();
    assert_eq!(caption(&conn).as_deref(), Some("contaminated"));
    assert_eq!(preserved(&conn), before);
    assert_eq!(
        rows(&conn, "SELECT * FROM ingest_unknown ORDER BY id"),
        evidence
    );
    assert_eq!(rows(&conn, "SELECT * FROM package_install_report"), reports);
    assert_eq!(
        conn.pragma_query_value(None, "user_version", |r| r.get::<_, i64>(0))
            .unwrap(),
        27
    );
}

#[test]
fn cross_file_loser_keeps_parser_evidence_without_gaining_normalized_ownership() {
    let winner = xml(
        r#"<ApplicationProgram Id="A"><Static><Parameters><Parameter Id="P" Offset="2"/></Parameters></Static><Dynamic><ComObjectRef TextParameterRefId="winner"/></Dynamic></ApplicationProgram>"#,
    );
    let loser = xml(
        r#"<ApplicationProgram Id="A"><Static><Parameters><Parameter Id="P" Offset="99"/></Parameters><ParameterBlock Access="losing-static"/></Static><Dynamic><ComObjectRef TextParameterRefId="losing-dynamic"/></Dynamic></ApplicationProgram>"#,
    );
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("products.sqlite");
    let conn = open_and_migrate(&path).unwrap();
    knx_productdb::ingest_file(&conn, "M-0001/winner.xml", winner.as_bytes()).unwrap();
    install_package(
        &conn,
        "loser.knxprod",
        &package(&[("M-0001/loser.xml", &loser)]),
    )
    .unwrap();
    let expected=rows(&conn,"SELECT source_sha256,program_id,xpath,kind,name,occurrences,sample FROM ingest_unknown ORDER BY 1,2,3,4,5");
    // The authoritative parser reports a losing tree's attrs before its
    // store=false branch; evidence is not the same as normalized ownership.
    assert_eq!(
        conn.query_row(
            "SELECT count(*) FROM ingest_unknown WHERE sample='losing-dynamic'",
            [],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        1
    );
    assert_eq!(
        conn.query_row(
            "SELECT count(*) FROM dynamic_node WHERE extra LIKE '%losing-dynamic%'",
            [],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        0
    );
    let history = preserved(&conn);
    conn.execute_batch("UPDATE parameter SET own_offset=99; DELETE FROM ingest_unknown WHERE name IN ('Access','TextParameterRefId'); PRAGMA user_version=27;").unwrap();
    drop(conn);
    let conn = open_and_migrate(&path).unwrap();
    assert_eq!(
        conn.query_row("SELECT own_offset FROM parameter", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        2
    );
    assert_eq!(rows(&conn,"SELECT source_sha256,program_id,xpath,kind,name,occurrences,sample FROM ingest_unknown ORDER BY 1,2,3,4,5"),expected);
    assert_eq!(preserved(&conn), history);
    assert_eq!(
        conn.query_row("SELECT source_sha256 FROM application_program", [], |r| {
            r.get::<_, String>(0)
        })
        .unwrap(),
        sha256_hex(winner.as_bytes())
    );
}

#[test]
fn module_paths_repeated_attrs_and_missing_parameter_id_follow_parser_defaults() {
    let (_dir, path, conn, _) = installed(
        r#"<ApplicationProgram Id="A"><Static><ModuleDefs><ModuleDef Id="M"><Static>
        <ParameterBlock Access="first"/><ParameterBlock Access="second"/>
        <Parameters><Union><Property ObjectIndex="4" PropertyId="78"/><Parameter Offset="3"/></Union></Parameters>
        </Static><Dynamic><ComObjectRef TextParameterRefId="module-caption"/><Parameter Offset="9" BitOffset="2"/></Dynamic>
        </ModuleDef></ModuleDefs></Static></ApplicationProgram>"#,
    );
    let evidence=rows(&conn,"SELECT source_sha256,program_id,xpath,kind,name,occurrences,sample FROM ingest_unknown ORDER BY 1,2,3,4,5");
    let expected = rows(&conn, "SELECT * FROM parameter");
    assert_eq!(
        conn.query_row("SELECT id FROM parameter", [], |r| r.get::<_, String>(0))
            .unwrap(),
        ""
    );
    let access: (String, i64, String) = conn
        .query_row(
            "SELECT xpath,occurrences,sample FROM ingest_unknown WHERE name='Access'",
            [],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .unwrap();
    assert_eq!(
        access,
        (
            format!("{ROOT}/Static/ModuleDefs/ModuleDef/Static/ParameterBlock"),
            2,
            "first".into()
        )
    );
    let offsets=rows(&conn,"SELECT xpath,name,sample FROM ingest_unknown WHERE name IN ('Offset','BitOffset') ORDER BY name");
    assert_eq!(
        offsets,
        vec![
            vec![
                Value::Text(format!("{ROOT}/ModuleDefs/ModuleDef/Dynamic/Parameter")),
                Value::Text("BitOffset".into()),
                Value::Text("2".into())
            ],
            vec![
                Value::Text(format!("{ROOT}/ModuleDefs/ModuleDef/Dynamic/Parameter")),
                Value::Text("Offset".into()),
                Value::Text("9".into())
            ]
        ]
    );
    conn.execute_batch("UPDATE parameter SET own_offset=99,property_id=99;
        DELETE FROM ingest_unknown WHERE name IN ('Access','TextParameterRefId','Offset','BitOffset'); PRAGMA user_version=27;").unwrap();
    drop(conn);
    let conn = open_and_migrate(&path).unwrap();
    assert_eq!(rows(&conn, "SELECT * FROM parameter"), expected);
    assert_eq!(rows(&conn,"SELECT source_sha256,program_id,xpath,kind,name,occurrences,sample FROM ingest_unknown ORDER BY 1,2,3,4,5"),evidence);
}

#[test]
fn qualified_attrs_and_scheme_reconciliation_remain_authoritative() {
    for scheme in [12, 21, 23] {
        let master = MASTER.replace("project/20", &format!("project/{scheme}"));
        let declarations = if scheme == 12 {
            r#"<ApplicationProgram Id="A"><Static><ParameterBlock Access="unqualified" xmlns:e="urn:test" e:Access="qualified"/></Static><Dynamic><Parameter Offset="5" xmlns:e="urn:test" e:Offset="qualified-offset"/></Dynamic></ApplicationProgram>"#
        } else {
            // Package admission refuses qualified attributes for schemes 21/23.
            r#"<ApplicationProgram Id="A"><Static><ParameterBlock Access="unqualified"/></Static><Dynamic><Parameter Offset="5"/></Dynamic></ApplicationProgram>"#
        };
        let program = xml(declarations).replace("project/20", &format!("project/{scheme}"));
        let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
        for (name, data) in [
            ("knx_master.xml", master.as_str()),
            ("M-0001/A.xml", program.as_str()),
        ] {
            zip.start_file(name, SimpleFileOptions::default()).unwrap();
            zip.write_all(data.as_bytes()).unwrap();
        }
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("products.sqlite");
        let conn = open_and_migrate(&path).unwrap();
        install_package(
            &conn,
            "qualified.knxprod",
            &zip.finish().unwrap().into_inner(),
        )
        .unwrap();
        let expected=rows(&conn,"SELECT source_sha256,program_id,xpath,kind,name,occurrences,sample FROM ingest_unknown ORDER BY 1,2,3,4,5");
        let reports = rows(
            &conn,
            "SELECT * FROM package_install_unknown ORDER BY ordinal",
        );
        assert_eq!(conn.query_row("SELECT count(*) FROM ingest_unknown WHERE sample='qualified' OR sample='qualified-offset'",[],|r|r.get::<_,i64>(0)).unwrap(),if scheme==12 {2} else {0});
        conn.execute_batch(
            "DELETE FROM ingest_unknown WHERE name IN ('Access','Offset'); PRAGMA user_version=27;",
        )
        .unwrap();
        drop(conn);
        let conn = open_and_migrate(&path).unwrap();
        assert_eq!(rows(&conn,"SELECT source_sha256,program_id,xpath,kind,name,occurrences,sample FROM ingest_unknown ORDER BY 1,2,3,4,5"),expected,"scheme {scheme}");
        assert_eq!(
            rows(
                &conn,
                "SELECT * FROM package_install_unknown ORDER BY ordinal"
            ),
            reports,
            "scheme {scheme}"
        );
    }
}

#[test]
fn legacy_and_opaque_payloads_are_not_xml_repair_inputs() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("products.sqlite");
    let conn = open_and_migrate(&path).unwrap();
    let bytes = include_bytes!("../../fixtures/legacy/marvin-program-plain.vd4");
    let payload = knx_productdb::legacy::read_legacy_member(bytes)
        .unwrap()
        .open_unencrypted()
        .unwrap();
    knx_productdb::legacy::publish_legacy(&conn, "legacy.vd4", bytes, &payload).unwrap();
    knx_productdb::ingest_file(
        &conn,
        "M-0001/Baggages/opaque.dll",
        b"MZ\x00opaque <ApplicationProgram",
    )
    .unwrap();
    let before = full_snapshot(&conn);
    conn.pragma_update(None, "user_version", 27).unwrap();
    drop(conn);
    let conn = open_and_migrate(&path).unwrap();
    assert_eq!(full_snapshot(&conn), before);
    assert_eq!(
        conn.query_row(
            "SELECT count(*) FROM ingest_unknown WHERE kind='PromotionRepairError'",
            [],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        0
    );
}

fn full_snapshot(conn: &Connection) -> Vec<(String, Vec<Vec<Value>>)> {
    let tables = conn
        .prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")
        .unwrap()
        .query_map([], |r| r.get::<_, String>(0))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    tables
        .into_iter()
        .map(|table| {
            let sql = format!("SELECT * FROM \"{table}\" ORDER BY rowid");
            let data = rows(conn, &sql);
            (table, data)
        })
        .collect()
}

#[test]
fn empty_v27_upgrade_is_clean_and_keeps_schema_objects() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("products.sqlite");
    let conn = open_and_migrate(&path).unwrap();
    let before = full_snapshot(&conn);
    let schema = rows(
        &conn,
        "SELECT type,name,tbl_name,sql FROM sqlite_master ORDER BY type,name",
    );
    conn.pragma_update(None, "user_version", 27).unwrap();
    drop(conn);
    let conn = open_and_migrate(&path).unwrap();
    assert_eq!(full_snapshot(&conn), before);
    assert_eq!(
        rows(
            &conn,
            "SELECT type,name,tbl_name,sql FROM sqlite_master ORDER BY type,name"
        ),
        schema
    );
}

#[test]
fn xml_looking_opaque_baggage_is_never_semantically_replayed() {
    let baggage = xml(
        r#"<ApplicationProgram Id="not-a-program"><Static><ParameterBlock Access="not-program-evidence"/></Static></ApplicationProgram>"#,
    );
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("products.sqlite");
    let conn = open_and_migrate(&path).unwrap();
    let program = xml(OBJECT);
    install_package(
        &conn,
        "opaque.knxprod",
        &package(&[
            ("M-0001/A.xml", &program),
            ("M-0001/Baggages/looks-like-program.xml", &baggage),
        ]),
    )
    .unwrap();
    assert_eq!(
        conn.query_row("SELECT count(*) FROM application_program", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        1
    );
    let before = full_snapshot(&conn);
    conn.pragma_update(None, "user_version", 27).unwrap();
    drop(conn);
    let conn = open_and_migrate(&path).unwrap();
    assert_eq!(
        full_snapshot(&conn),
        before,
        "retained opaque XML is not a semantic program source"
    );
}

#[test]
fn an_unowned_parse_source_integrity_failure_is_not_silently_disregarded() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("products.sqlite");
    let conn = open_and_migrate(&path).unwrap();
    let data = b"retained generic file";
    knx_productdb::ingest_file(&conn, "opaque.bin", data).unwrap();
    let sha = sha256_hex(data);
    conn.execute("UPDATE source_file SET len=len+1 WHERE sha256=?1", [&sha])
        .unwrap();
    conn.pragma_update(None, "user_version", 27).unwrap();
    drop(conn);
    let conn = open_and_migrate(&path).unwrap();
    let failures:i64=conn.query_row("SELECT count(*) FROM ingest_unknown WHERE source_sha256=?1 AND kind='PromotionRepairError'",[&sha],|r|r.get(0)).unwrap();
    assert_eq!(
        failures, 1,
        "failed source admission must have an explicit conservative disposition"
    );
}

#[test]
fn repairing_targets_does_not_recompute_unrelated_package_counter_from_global_ledger() {
    let program = OBJECT.replace(
        "Id=\"P\" Offset",
        "Id=\"P\" Fancy=\"historically reported\" Offset",
    );
    let (_dir, path, conn, _) = installed(&program);
    let original: i64 = conn
        .query_row("SELECT unknown_count FROM package", [], |r| r.get(0))
        .unwrap();
    assert_eq!(
        conn.query_row(
            "SELECT count(*) FROM package_install_unknown WHERE name='Fancy'",
            [],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        1
    );
    // An unrelated ledger gap does not authorize rewriting intact per-package
    // history. The target still needs repair independently.
    conn.execute_batch("DELETE FROM ingest_unknown WHERE name IN ('Fancy','Access');
        DELETE FROM package_install_unknown WHERE name='Access';
        UPDATE package_install_report SET unknown_distinct=unknown_distinct-1,unknown_occurrences=unknown_occurrences-1;
        UPDATE package_install_count SET count=count-1 WHERE category='unknown_construct';
        UPDATE package SET unknown_count=unknown_count-1; PRAGMA user_version=27;").unwrap();
    drop(conn);
    let conn = open_and_migrate(&path).unwrap();
    assert_eq!(
        conn.query_row("SELECT unknown_count FROM package", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        original,
        "only the exact target member delta may alter the package counter"
    );
    assert_eq!(
        conn.query_row(
            "SELECT count(*) FROM ingest_unknown WHERE name='Fancy'",
            [],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        0
    );
}

#[test]
fn unrelated_master_suffix_evidence_and_counter_are_preserved() {
    let program = xml(OBJECT);
    let master = MASTER.replace(
        "<Unmodelled/>",
        "<Languages><Parameter Offset=\"master-offset\"/></Languages>",
    );
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("products.sqlite");
    let conn = open_and_migrate(&path).unwrap();
    install_package(
        &conn,
        "master-witness.knxprod",
        &package_with_master(&master, &[("M-0001/A.xml", &program)]),
    )
    .unwrap();
    let witness = rows(
        &conn,
        "SELECT * FROM package_install_unknown WHERE sample='master-offset'",
    );
    assert_eq!(
        witness.len(),
        1,
        "actual master parser must produce the suffix witness"
    );
    let before = full_snapshot(&conn);
    conn.pragma_update(None, "user_version", 27).unwrap();
    drop(conn);
    let conn = open_and_migrate(&path).unwrap();
    assert_eq!(
        rows(
            &conn,
            "SELECT * FROM package_install_unknown WHERE sample='master-offset'"
        ),
        witness,
        "nonprogram suffix evidence must not be retired"
    );
    assert_eq!(full_snapshot(&conn), before);
}

#[test]
fn exact_key_master_and_program_overlap_keeps_member_units_and_first_sample() {
    let common = "/KNX/MasterData/Languages/Parameter";
    let master = MASTER.replace(
        "<Unmodelled/>",
        "<Languages><Parameter Offset=\"master-offset\"/></Languages>",
    );
    let program=xml(OBJECT).replace("</KNX>","<MasterData><Languages><Parameter Offset=\"program-offset\"/></Languages></MasterData></KNX>");
    let program_sha = sha256_hex(program.as_bytes());
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("products.sqlite");
    let conn = open_and_migrate(&path).unwrap();
    install_package(
        &conn,
        "intersecting.knxprod",
        &package_with_master(&master, &[("M-0001/A.xml", &program)]),
    )
    .unwrap();
    let expected = rows(
        &conn,
        "SELECT * FROM package_install_unknown ORDER BY ordinal",
    );
    let counter: i64 = conn
        .query_row("SELECT unknown_count FROM package", [], |r| r.get(0))
        .unwrap();
    let row:(i64,String)=conn.query_row("SELECT occurrences,sample FROM package_install_unknown WHERE xpath=?1 AND name='Offset'",[common],|r|Ok((r.get(0)?,r.get(1)?))).unwrap();
    assert_eq!(row, (2, "master-offset".into()));
    // Released retirement loses ONE program-member key, but deletes the
    // aggregate report key containing BOTH sources. The master ledger remains.
    conn.execute(
        "DELETE FROM ingest_unknown WHERE source_sha256=?1 AND xpath=?2 AND name='Offset'",
        params![program_sha, common],
    )
    .unwrap();
    conn.execute(
        "DELETE FROM package_install_unknown WHERE xpath=?1 AND name='Offset'",
        [common],
    )
    .unwrap();
    conn.execute_batch("UPDATE package SET unknown_count=unknown_count-1;
        UPDATE package_install_report SET unknown_distinct=unknown_distinct-1,unknown_occurrences=unknown_occurrences-2;
        UPDATE package_install_count SET count=count-CASE disposition WHEN 'read' THEN 2 ELSE 1 END WHERE category='unknown_construct';
        PRAGMA user_version=27;").unwrap();
    drop(conn);
    let conn = open_and_migrate(&path).unwrap();
    assert_eq!(
        rows(
            &conn,
            "SELECT * FROM package_install_unknown ORDER BY ordinal"
        ),
        expected
    );
    assert_eq!(
        conn.query_row("SELECT unknown_count FROM package", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        counter,
        "a nonprogram member never lost its distinct counter contribution"
    );
}

#[test]
fn promoted_program_attr_does_not_hide_a_retired_nonprogram_report_contribution() {
    let common = "/KNX/MasterData/Languages/Parameter";
    let master = MASTER.replace(
        "<Unmodelled/>",
        "<Languages><Parameter Offset=\"master-offset\"/></Languages>",
    );
    let program = xml(OBJECT).replace(
        "</KNX>",
        "<MasterData><Languages><Parameter Offset=\"5\"/></Languages></MasterData></KNX>",
    );
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("products.sqlite");
    let conn = open_and_migrate(&path).unwrap();
    install_package(
        &conn,
        "promoted-intersection.knxprod",
        &package_with_master(&master, &[("M-0001/A.xml", &program)]),
    )
    .unwrap();
    let expected = rows(
        &conn,
        "SELECT * FROM package_install_unknown ORDER BY ordinal",
    );
    let counter: i64 = conn
        .query_row("SELECT unknown_count FROM package", [], |r| r.get(0))
        .unwrap();
    conn.execute(
        "DELETE FROM package_install_unknown WHERE xpath=?1 AND name='Offset'",
        [common],
    )
    .unwrap();
    // The old program-member Offset was promoted and its counter decremented;
    // today's byte-derived program evidence therefore contributes zero here.
    // The retained master member's counter contribution was never decremented.
    conn.execute_batch("UPDATE package_install_report SET unknown_distinct=unknown_distinct-1,unknown_occurrences=unknown_occurrences-1;
        UPDATE package_install_count SET count=count-1 WHERE category='unknown_construct'; PRAGMA user_version=27;").unwrap();
    drop(conn);
    let conn = open_and_migrate(&path).unwrap();
    assert_eq!(
        rows(
            &conn,
            "SELECT * FROM package_install_unknown ORDER BY ordinal"
        ),
        expected
    );
    assert_eq!(
        conn.query_row("SELECT unknown_count FROM package", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        counter
    );
}

#[test]
fn source_diagnostic_reader_exposes_repair_failure_and_rejects_invalid_shape() {
    let (_dir, path, conn, sha) = installed(OBJECT);
    conn.execute("UPDATE source_file SET len=len+1 WHERE sha256=?1", [&sha])
        .unwrap();
    conn.pragma_update(None, "user_version", 27).unwrap();
    drop(conn);
    let conn = open_and_migrate(&path).unwrap();
    let diagnostics = knx_productdb::source_diagnostics(&conn, &sha)
        .unwrap()
        .unwrap();
    let repair = diagnostics
        .iter()
        .find(|row| row.kind == "PromotionRepairError")
        .unwrap();
    assert_eq!(repair.name, "promotion_repair");
    assert_eq!(repair.occurrences, 1);
    assert!(repair
        .sample
        .as_deref()
        .unwrap()
        .contains("length mismatch"));
    conn.execute(
        "UPDATE ingest_unknown SET name='invalid' WHERE kind='PromotionRepairError'",
        [],
    )
    .unwrap();
    assert!(knx_productdb::source_diagnostics(&conn, &sha).is_err());
}

#[test]
fn missing_and_empty_program_ids_follow_static_and_dynamic_parser_identity() {
    let (_dir, path, conn, _) = installed(
        r#"<ApplicationProgram><Static><Parameters><Parameter Offset="99"/></Parameters></Static><Dynamic><Parameter Offset="first-dynamic"/></Dynamic></ApplicationProgram>
        <ApplicationProgram Id=""><Static><Parameters><Parameter Offset="2"/></Parameters></Static><Dynamic><Parameter Offset="second-dynamic"/></Dynamic></ApplicationProgram>"#,
    );
    assert_eq!(
        conn.query_row("SELECT id,own_offset FROM parameter", [], |r| Ok((
            r.get::<_, String>(0)?,
            r.get::<_, i64>(1)?
        )))
        .unwrap(),
        ("".into(), 99)
    );
    let expected = full_snapshot(&conn);
    conn.execute_batch("UPDATE parameter SET own_offset=2; PRAGMA user_version=27;")
        .unwrap();
    drop(conn);
    let conn = open_and_migrate(&path).unwrap();
    assert_eq!(full_snapshot(&conn), expected);
}

#[test]
fn oversized_target_inventory_preserves_source_and_makes_report_unavailable() {
    let large = "x".repeat(16 * 1024 * 1024 + 1);
    let program = OBJECT.replace("Access=\"Read\"", &format!("Access=\"{large}\""));
    let (_dir, path, conn, sha) = installed(&program);
    conn.execute_batch(
        "UPDATE com_object_ref SET text_parameter_ref_id='contaminated'; PRAGMA user_version=27;",
    )
    .unwrap();
    let original_len: i64 = conn
        .query_row(
            "SELECT length(sample) FROM ingest_unknown WHERE name='Access'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    drop(conn);
    let conn = open_and_migrate(&path).unwrap();
    assert_eq!(
        caption(&conn).as_deref(),
        Some("contaminated"),
        "over-budget source must not be partially corrected"
    );
    assert_eq!(
        conn.query_row(
            "SELECT length(sample) FROM ingest_unknown WHERE name='Access'",
            [],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        original_len
    );
    assert_eq!(
        rows(&conn, "SELECT status FROM package_install_report"),
        vec![vec![Value::Text("unavailable".into())]]
    );
    let reason:String=conn.query_row("SELECT sample FROM ingest_unknown WHERE source_sha256=?1 AND kind='PromotionRepairError'",[sha],|r|r.get(0)).unwrap();
    assert!(reason.contains("allowance exceeded"), "{reason}");
}

#[test]
fn cumulative_target_inventory_refuses_later_authentic_source_without_undoing_prior_repair() {
    const ALLOWANCE: usize = 16 * 1024 * 1024;
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("products.sqlite");
    let conn = open_and_migrate(&path).unwrap();
    let mut sources = Vec::new();
    for (id, letter) in [("A0", 'x'), ("A1", 'y')] {
        // Neither source alone exhausts the allowance. Their retained target
        // samples do so cumulatively, even though the ordinary/extended modes
        // are identical and therefore share a single inventory.
        let sample = letter.to_string().repeat(4 * 1024 * 1024);
        let program = xml(&OBJECT
            .replace("Id=\"A\"", &format!("Id=\"{id}\""))
            .replace("Access=\"Read\"", &format!("Access=\"{sample}\"")));
        let archive = package(&[("M-0001/A.xml", &program)]);
        install_package(&conn, "cumulative.knxprod", &archive).unwrap();
        sources.push((
            sha256_hex(program.as_bytes()),
            id,
            sha256_hex(&archive),
            sample,
        ));
    }
    // The retained-source query's immutable SHA order, not installation order,
    // determines which repair is admitted first. The programs have disjoint ids.
    sources.sort_by(|a, b| a.0.cmp(&b.0));
    let mut cumulative = 0;
    for (index, (sha, _, _, sample)) in sources.iter().enumerate() {
        let targets = conn
            .prepare("SELECT xpath,kind,name,sample FROM ingest_unknown WHERE source_sha256=?1 AND name IN ('TextParameterRefId','Access','Offset','BitOffset','ObjectIndex','ObjectType','PropertyId','Occurrence','Property') ORDER BY id")
            .unwrap()
            .query_map([sha], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?, r.get::<_, String>(2)?, r.get::<_, String>(3)?)))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        assert_eq!(targets.len(), 1);
        let (xpath, kind, name, actual_sample) = &targets[0];
        assert_eq!(xpath, &format!("{ROOT}/Static/ParameterBlock"));
        assert_eq!((kind.as_str(), name.as_str()), ("Attribute", "Access"));
        assert_eq!(actual_sample, sample);
        // Independent accounting from authentic retained rows: source header,
        // two row/path/name/sample copies, and each original key's map charge.
        // Do not call the production meter to calculate its own expected result.
        let header = 256 + sha.len();
        let row_copies = 2 * (512 + xpath.len() + name.len() + sample.len());
        let original_key = 256 + xpath.len() + kind.len() + name.len();
        let admission = header + row_copies + original_key;
        assert!(admission < ALLOWANCE, "each individual source fits");
        cumulative += admission;
        assert_eq!(
            cumulative <= ALLOWANCE,
            index == 0,
            "exact cumulative admission"
        );
    }
    let mut history = preserved(&conn);
    // Established unavailable-report handling removes its detail rows. All
    // other history, including the admitted package's report facts, survives.
    for index in [3, 4] {
        history[index].retain(|row| row[0] != Value::Text(sources[1].2.clone()));
    }
    let package_counters = rows(
        &conn,
        "SELECT sha256,unknown_count FROM package ORDER BY sha256",
    );
    let successful_header = rows(
        &conn,
        &format!(
            "SELECT * FROM package_install_report WHERE package_sha256='{}'",
            sources[0].2
        ),
    );
    let successful_report = rows(
        &conn,
        &format!(
            "SELECT * FROM package_install_unknown WHERE package_sha256='{}' ORDER BY ordinal",
            sources[0].2
        ),
    );
    assert!(!successful_report.is_empty());
    conn.execute_batch(
        "UPDATE com_object_ref SET text_parameter_ref_id='contaminated';
        UPDATE parameter SET own_offset=99;
        UPDATE ingest_unknown SET occurrences=7 WHERE name='Access'; PRAGMA user_version=27;",
    )
    .unwrap();
    drop(conn);
    let conn = open_and_migrate(&path).unwrap();
    for (index, (sha, id, package_sha, sample)) in sources.iter().enumerate() {
        let admitted = index == 0;
        let values: (String, i64) = conn.query_row(
            "SELECT c.text_parameter_ref_id,p.own_offset FROM com_object_ref c JOIN parameter p ON p.program_id=c.program_id WHERE c.program_id=?1",
            [id], |r| Ok((r.get(0)?, r.get(1)?))).unwrap();
        assert_eq!(
            values,
            if admitted {
                ("right".into(), 2)
            } else {
                ("contaminated".into(), 99)
            },
            "prior successful source stays repaired; later source stays untouched"
        );
        let evidence: (String, i64, String) = conn.query_row(
            "SELECT xpath,occurrences,sample FROM ingest_unknown WHERE source_sha256=?1 AND name='Access'",
            [sha], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?))).unwrap();
        assert_eq!(evidence.0, format!("{ROOT}/Static/ParameterBlock"));
        assert_eq!(evidence.1, if admitted { 1 } else { 7 });
        assert_eq!(&evidence.2, sample, "no sample truncation or corruption");
        let status: String = conn
            .query_row(
                "SELECT status FROM package_install_report WHERE package_sha256=?1",
                [package_sha],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(status, if admitted { "measured" } else { "unavailable" });
        let failures = knx_productdb::source_diagnostics(&conn, sha)
            .unwrap()
            .unwrap()
            .into_iter()
            .filter(|row| row.kind == "PromotionRepairError")
            .collect::<Vec<_>>();
        if admitted {
            assert!(failures.is_empty());
            assert_eq!(rows(&conn, &format!("SELECT * FROM package_install_unknown WHERE package_sha256='{package_sha}' ORDER BY ordinal")), successful_report);
            let reported: String = conn.query_row("SELECT sample FROM package_install_unknown WHERE package_sha256=?1 AND name='Access'", [package_sha], |r| r.get(0)).unwrap();
            assert_eq!(&reported, sample);
        } else {
            assert_eq!(failures.len(), 1);
            assert_eq!(failures[0].name, "promotion_repair");
            assert_eq!(failures[0].occurrences, 1);
            assert_eq!(failures[0].xpath, "M-0001/A.xml");
            assert!(failures[0]
                .sample
                .as_deref()
                .unwrap()
                .contains("16 MiB evidence working-set allowance exceeded"));
        }
    }
    for (index, (actual, expected)) in preserved(&conn).iter().zip(&history).enumerate() {
        assert!(actual == expected, "preserved history surface {index}");
    }
    assert_eq!(
        rows(
            &conn,
            "SELECT sha256,unknown_count FROM package ORDER BY sha256"
        ),
        package_counters
    );
    assert_eq!(
        rows(
            &conn,
            &format!(
                "SELECT * FROM package_install_report WHERE package_sha256='{}'",
                sources[0].2
            )
        ),
        successful_header
    );
    for table in [
        "package_install_count",
        "package_install_unknown",
        "package_install_diagnostic",
    ] {
        assert!(
            rows(
                &conn,
                &format!(
                    "SELECT * FROM {table} WHERE package_sha256='{}'",
                    sources[1].2
                )
            )
            .is_empty(),
            "unavailable report details are not fabricated"
        );
    }
    assert_eq!(rows(&conn, &format!("SELECT unknown_distinct,unknown_occurrences FROM package_install_report WHERE package_sha256='{}'", sources[1].2)), vec![vec![Value::Integer(0), Value::Integer(0)]]);
    assert_eq!(
        rows(&conn, "PRAGMA integrity_check"),
        vec![vec![Value::Text("ok".into())]]
    );
    assert!(rows(&conn, "PRAGMA foreign_key_check").is_empty());
    let after = full_snapshot(&conn);
    drop(conn);
    assert_eq!(
        full_snapshot(&open_and_migrate(&path).unwrap()),
        after,
        "no-change reopen"
    );
}

fn repair_reader_fixture() -> (tempfile::TempDir, Connection, String) {
    let (dir, path, conn, sha) = installed(OBJECT);
    conn.execute("UPDATE source_file SET len=len+1 WHERE sha256=?1", [&sha])
        .unwrap();
    conn.pragma_update(None, "user_version", 27).unwrap();
    drop(conn);
    let conn = open_and_migrate(&path).unwrap();
    let diagnostic = knx_productdb::source_diagnostics(&conn, &sha)
        .unwrap()
        .unwrap()
        .into_iter()
        .filter(|row| row.kind == "PromotionRepairError")
        .collect::<Vec<_>>();
    assert_eq!(diagnostic.len(), 1);
    assert_eq!(diagnostic[0].name, "promotion_repair");
    assert_eq!(diagnostic[0].occurrences, 1);
    assert_eq!(diagnostic[0].xpath, "M-0001/A.xml");
    assert!(diagnostic[0]
        .sample
        .as_deref()
        .unwrap()
        .contains("length mismatch"));
    (dir, conn, sha)
}

fn assert_repair_reader_refuses_without_writing(conn: &Connection, sha: &str) {
    let before = full_snapshot(conn);
    let error = knx_productdb::source_diagnostics(conn, sha).unwrap_err();
    assert!(
        error
            .to_string()
            .contains("invalid persisted diagnostic kind or count"),
        "{error}"
    );
    assert_eq!(
        full_snapshot(conn),
        before,
        "reader refusal must not modify stored facts"
    );
}

#[test]
fn promotion_repair_reader_refuses_invalid_count_independently() {
    for count in [0_i64, 2, -1, i64::from(u32::MAX) + 1] {
        let (_dir, conn, sha) = repair_reader_fixture();
        let valid = knx_productdb::source_diagnostics(&conn, &sha).unwrap();
        conn.execute_batch("PRAGMA ignore_check_constraints=ON")
            .unwrap();
        conn.execute(
            "UPDATE ingest_unknown SET occurrences=?1 WHERE kind='PromotionRepairError'",
            [count],
        )
        .unwrap();
        assert_repair_reader_refuses_without_writing(&conn, &sha);
        conn.execute(
            "UPDATE ingest_unknown SET occurrences=1 WHERE kind='PromotionRepairError'",
            [],
        )
        .unwrap();
        assert_eq!(
            knx_productdb::source_diagnostics(&conn, &sha).unwrap(),
            valid
        );
    }
}

#[test]
fn promotion_repair_reader_refuses_empty_path_independently() {
    let (_dir, conn, sha) = repair_reader_fixture();
    let valid = knx_productdb::source_diagnostics(&conn, &sha).unwrap();
    conn.execute(
        "UPDATE ingest_unknown SET xpath='' WHERE kind='PromotionRepairError'",
        [],
    )
    .unwrap();
    assert_repair_reader_refuses_without_writing(&conn, &sha);
    conn.execute(
        "UPDATE ingest_unknown SET xpath='M-0001/A.xml' WHERE kind='PromotionRepairError'",
        [],
    )
    .unwrap();
    assert_eq!(
        knx_productdb::source_diagnostics(&conn, &sha).unwrap(),
        valid
    );
}

#[test]
fn promotion_repair_reader_refuses_missing_or_empty_sample_independently() {
    for sample in [None, Some("")] {
        let (_dir, conn, sha) = repair_reader_fixture();
        let valid = knx_productdb::source_diagnostics(&conn, &sha).unwrap();
        let original: String = conn
            .query_row(
                "SELECT sample FROM ingest_unknown WHERE kind='PromotionRepairError'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        conn.execute(
            "UPDATE ingest_unknown SET sample=?1 WHERE kind='PromotionRepairError'",
            [sample],
        )
        .unwrap();
        assert_repair_reader_refuses_without_writing(&conn, &sha);
        conn.execute(
            "UPDATE ingest_unknown SET sample=?1 WHERE kind='PromotionRepairError'",
            [original],
        )
        .unwrap();
        assert_eq!(
            knx_productdb::source_diagnostics(&conn, &sha).unwrap(),
            valid
        );
    }
}
