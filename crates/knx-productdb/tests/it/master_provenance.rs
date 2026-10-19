//! Independent master identity provenance acceptance fixtures (ADR-0147).
use knx_productdb::{ingest_master_data, open_and_migrate, sha256_hex, Connection};

const FIRST: &str = r#"<KNX><MasterData>
<DatapointType Id="D" Number="7"><DatapointSubtype Id="S" Number="2"/><DatapointSubtype Id="D" Number="3"/></DatapointType>
<FunctionType Id="F" Number="1"><FunctionPoint Id="P" Role="first"/><FunctionPoint Id="P" Role="second"/></FunctionType>
<FunctionType Id="F" Number="2"/><SpaceUsage Id="U" Number="4"/><SpaceUsage Id="U" Number="5"/>
</MasterData></KNX>"#;
const SECOND: &str = r#"<KNX><MasterData><DatapointType Id="D" Number="9"><DatapointSubtype Id="S" Number="8"/></DatapointType><FunctionType Id="F" Number="9"><FunctionPoint Id="P" Role="other"/></FunctionType><SpaceUsage Id="U" Number="9"/></MasterData></KNX>"#;

fn db() -> (tempfile::TempDir, Connection) {
    let dir = tempfile::tempdir().unwrap();
    let conn = open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
    (dir, conn)
}
fn number(conn: &Connection, sql: &str) -> i64 {
    conn.query_row(sql, [], |r| r.get(0)).unwrap()
}

#[test]
fn all_families_same_and_cross_source_first_winners_have_exact_positions() {
    let (_dir, conn) = db();
    ingest_master_data(&conn, FIRST.as_bytes()).unwrap();
    ingest_master_data(&conn, SECOND.as_bytes()).unwrap();
    assert_eq!(
        number(&conn, "SELECT main FROM datapoint_type WHERE id='D'"),
        7
    );
    assert_eq!(
        number(&conn, "SELECT sub FROM datapoint_type WHERE id='S'"),
        2
    );
    assert_eq!(
        number(&conn, "SELECT number FROM function_type WHERE id='F'"),
        1
    );
    assert_eq!(
        number(&conn, "SELECT number FROM space_usage WHERE id='U'"),
        4
    );
    let role: String = conn
        .query_row("SELECT role FROM function_point WHERE id='P'", [], |r| {
            r.get(0)
        })
        .unwrap();
    assert_eq!(role, "first");
    let rows = conn.prepare("SELECT family,logical_id,ordinal,observed_write FROM master_declaration WHERE source_sha256=?1 ORDER BY ordinal").unwrap().query_map([sha256_hex(FIRST.as_bytes())],|r|Ok((r.get::<_,String>(0)?,r.get::<_,Option<String>>(1)?,r.get::<_,i64>(2)?,r.get::<_,Option<bool>>(3)?))).unwrap().collect::<Result<Vec<_>,_>>().unwrap();
    assert_eq!(
        rows,
        vec![
            ("datapoint_type".into(), Some("D".into()), 1, Some(true)),
            ("datapoint_type".into(), Some("S".into()), 2, Some(true)),
            ("datapoint_type".into(), Some("D".into()), 3, Some(false)),
            ("function_type".into(), Some("F".into()), 4, Some(true)),
            ("function_point".into(), Some("P".into()), 5, Some(true)),
            ("function_point".into(), Some("P".into()), 6, Some(false)),
            ("function_type".into(), Some("F".into()), 7, Some(false)),
            ("space_usage".into(), Some("U".into()), 8, Some(true)),
            ("space_usage".into(), Some("U".into()), 9, Some(false)),
        ]
    );
    let winners=conn.prepare("SELECT family,logical_id,source_sha256,ordinal FROM master_winner ORDER BY family,logical_id").unwrap().query_map([],|r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?,r.get::<_,String>(2)?,r.get::<_,i64>(3)?))).unwrap().collect::<Result<Vec<_>,_>>().unwrap();
    let sha = sha256_hex(FIRST.as_bytes());
    assert_eq!(
        winners,
        vec![
            ("datapoint_type".into(), "D".into(), sha.clone(), 1),
            ("datapoint_type".into(), "S".into(), sha.clone(), 2),
            ("function_point".into(), "P".into(), sha.clone(), 5),
            ("function_type".into(), "F".into(), sha.clone(), 4),
            ("space_usage".into(), "U".into(), sha.clone(), 8)
        ]
    );
    assert_eq!(
        number(
            &conn,
            "SELECT count(*) FROM master_declaration WHERE observed_write=0"
        ),
        9
    );
    let (start,end):(u32,u32)=conn.query_row("SELECT byte_start,byte_end FROM master_declaration WHERE source_sha256=?1 AND ordinal=3",[&sha],|r|Ok((r.get(0)?,r.get(1)?))).unwrap();
    assert_eq!(
        &FIRST[start as usize..end as usize],
        r#"<DatapointSubtype Id="D" Number="3"/>"#
    );
    assert_eq!(
        knx_productdb::load_source_file(&conn, &sha)
            .unwrap()
            .unwrap(),
        FIRST.as_bytes()
    );
}

#[test]
fn missing_id_orphan_and_empty_id_are_not_conflated() {
    let (_dir, conn) = db();
    let xml = r#"<KNX><MasterData><DatapointType><DatapointSubtype Id="child"/></DatapointType><DatapointSubtype/><FunctionType><FunctionPoint Id="orphan"/><FunctionPoint/></FunctionType><SpaceUsage/><SpaceUsage Id=""/><FunctionType Id=""><FunctionPoint Id=""/></FunctionType></MasterData></KNX>"#;
    ingest_master_data(&conn, xml.as_bytes()).unwrap();
    let flags = conn
        .prepare("SELECT ordinal,missing_id,orphan_parent FROM master_declaration ORDER BY ordinal")
        .unwrap()
        .query_map([], |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, bool>(1)?,
                r.get::<_, bool>(2)?,
            ))
        })
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert_eq!(
        flags,
        vec![
            (1, true, false),
            (2, false, false),
            (3, true, true),
            (4, true, false),
            (5, false, true),
            (6, true, true),
            (7, true, false),
            (8, false, false),
            (9, false, false),
            (10, false, false)
        ]
    );
    assert_eq!(
        number(&conn, "SELECT main FROM datapoint_type WHERE id='child'"),
        0
    );
    assert_eq!(
        number(&conn, "SELECT count(*) FROM space_usage WHERE id=''"),
        1
    );
    assert_eq!(
        number(
            &conn,
            "SELECT count(*) FROM function_point WHERE id='' AND function_type_id=''"
        ),
        1
    );
}

#[test]
fn replay_and_v26_migration_never_choose_historical_winners_or_rewrite_values() {
    use knx_productdb::master_provenance::{
        master_identity, master_source_report, rederive_master_provenance, MasterKind,
    };
    let (dir, conn) = db();
    ingest_master_data(&conn, FIRST.as_bytes()).unwrap();
    ingest_master_data(&conn, SECOND.as_bytes()).unwrap();
    // Frozen historical values need not agree with any candidate.
    conn.execute("UPDATE datapoint_type SET main=1234 WHERE id='D'", [])
        .unwrap();
    let before = table_values(
        &conn,
        &[
            "datapoint_type",
            "function_type",
            "function_point",
            "space_usage",
            "source_file",
            "package_install_report",
            "package_install_count",
            "package_install_diagnostic",
        ],
    );
    conn.execute_batch("DROP TABLE master_winner; DROP TABLE master_declaration; DROP TABLE master_source; PRAGMA user_version=26").unwrap();
    drop(conn);
    let conn = open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
    assert_eq!(number(&conn, "PRAGMA user_version"), 27);
    assert_eq!(
        table_values(
            &conn,
            &[
                "datapoint_type",
                "function_type",
                "function_point",
                "space_usage",
                "source_file",
                "package_install_report",
                "package_install_count",
                "package_install_diagnostic"
            ]
        ),
        before
    );
    for (family, id, n) in [
        (MasterKind::DatapointType, "D", 3),
        (MasterKind::DatapointType, "S", 2),
        (MasterKind::FunctionType, "F", 3),
        (MasterKind::FunctionPoint, "P", 3),
        (MasterKind::SpaceUsage, "U", 3),
    ] {
        let report = master_identity(&conn, family, id).unwrap();
        assert!(report.normalized_present);
        assert_eq!(report.winner, None);
        assert_eq!(report.candidates.len(), n);
        assert!(report.candidates.iter().all(|d| d.observed_write.is_none()));
    }
    assert_eq!(
        master_source_report(&conn, &sha256_hex(FIRST.as_bytes()))
            .unwrap()
            .unwrap()
            .history,
        "historical-unavailable"
    );
    let original = table_values(
        &conn,
        &["master_source", "master_declaration", "master_winner"],
    );
    let report = rederive_master_provenance(&conn).unwrap();
    assert_eq!(
        (
            report.master_sources,
            report.failed_sources,
            report.unexamined_sources,
            report.declarations
        ),
        (2, 0, 0, 14)
    );
    assert_eq!(
        table_values(
            &conn,
            &["master_source", "master_declaration", "master_winner"]
        ),
        original
    );
    assert_eq!(rederive_master_provenance(&conn).unwrap(), report);
}

fn table_values(conn: &Connection, tables: &[&str]) -> Vec<Vec<Vec<rusqlite::types::Value>>> {
    tables
        .iter()
        .map(|table| {
            let mut s = conn
                .prepare(&format!("SELECT * FROM {table} ORDER BY rowid"))
                .unwrap();
            let n = s.column_count();
            s.query_map([], |r| {
                (0..n)
                    .map(|i| r.get(i))
                    .collect::<Result<Vec<rusqlite::types::Value>, _>>()
            })
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap()
        })
        .collect()
}

#[test]
fn replay_keeps_fresh_observations_retry_reopen_and_measured_zero() {
    use knx_productdb::master_provenance::{
        master_identity, master_source_report, rederive_master_provenance, MasterKind,
    };
    let (dir, conn) = db();
    let first = ingest_master_data(&conn, FIRST.as_bytes()).unwrap();
    assert_eq!(first.diagnostics.len(), 4);
    let second = ingest_master_data(&conn, SECOND.as_bytes()).unwrap();
    assert_eq!(second.diagnostics.len(), 5);
    let empty = b"<KNX><MasterData/></KNX>";
    ingest_master_data(&conn, empty).unwrap();
    let sha = sha256_hex(empty);
    let zero = master_source_report(&conn, &sha).unwrap().unwrap();
    assert_eq!(
        (
            zero.history.as_str(),
            zero.status.as_str(),
            zero.declarations.len()
        ),
        ("observed", "complete", 0)
    );
    let before = table_values(
        &conn,
        &["master_source", "master_declaration", "master_winner"],
    );
    rederive_master_provenance(&conn).unwrap();
    assert_eq!(
        table_values(
            &conn,
            &["master_source", "master_declaration", "master_winner"]
        ),
        before
    );
    ingest_master_data(&conn, FIRST.as_bytes()).unwrap();
    assert_eq!(
        table_values(
            &conn,
            &["master_source", "master_declaration", "master_winner"]
        ),
        before
    );
    drop(conn);
    let conn = open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
    let winner = master_identity(&conn, MasterKind::DatapointType, "D")
        .unwrap()
        .winner
        .unwrap();
    assert_eq!(
        (winner.source_sha256, winner.ordinal),
        (sha256_hex(FIRST.as_bytes()), 1)
    );
    assert_eq!(
        table_values(
            &conn,
            &["master_source", "master_declaration", "master_winner"]
        ),
        before
    );
}

#[test]
fn replay_names_missing_bad_hash_and_unexamined_sources_and_repairs_owned_markers() {
    use knx_productdb::master_provenance::{master_source_report, rederive_master_provenance};
    let (_dir, conn) = db();
    ingest_master_data(&conn, FIRST.as_bytes()).unwrap();
    let sha = sha256_hex(FIRST.as_bytes());
    conn.execute("UPDATE source_file SET bytes=x'00' WHERE sha256=?1", [&sha])
        .unwrap();
    knx_productdb::store_source_file(
        &conn,
        &knx_productdb::SourceFile {
            source_path: "opaque.bin".into(),
            manufacturer_id: None,
            bytes: b"opaque".to_vec(),
        },
    )
    .unwrap();
    let opaque = sha256_hex(b"opaque");
    conn.execute("UPDATE source_file SET len=-1 WHERE sha256=?1", [&opaque])
        .unwrap();
    let bad = b"<KNX><MasterData><SpaceUsage Id='lost'>";
    let bad_sha = sha256_hex(bad);
    knx_productdb::store_source_file(
        &conn,
        &knx_productdb::SourceFile {
            source_path: "knx_master.xml".into(),
            manufacturer_id: None,
            bytes: bad.to_vec(),
        },
    )
    .unwrap();
    let r = rederive_master_provenance(&conn).unwrap();
    assert_eq!(
        (r.master_sources, r.failed_sources, r.unexamined_sources),
        (2, 2, 1)
    );
    assert_eq!(
        master_source_report(&conn, &sha).unwrap().unwrap().status,
        "failed"
    );
    assert_eq!(
        master_source_report(&conn, &bad_sha)
            .unwrap()
            .unwrap()
            .declarations
            .len(),
        0
    );
    assert_eq!(
        master_source_report(&conn, &opaque)
            .unwrap()
            .unwrap()
            .status,
        "unexamined"
    );
    assert_eq!(
        number(&conn, "SELECT count(*) FROM space_usage WHERE id='lost'"),
        0
    );
    conn.execute("DELETE FROM source_file WHERE sha256=?1", [&sha])
        .unwrap();
    rederive_master_provenance(&conn).unwrap();
    assert!(master_source_report(&conn, &sha)
        .unwrap()
        .unwrap()
        .reason
        .unwrap()
        .contains("missing"));
    conn.execute(
        "INSERT INTO source_file(sha256,source_path,len,bytes) VALUES(?1,'knx_master.xml',?2,?3)",
        rusqlite::params![sha, FIRST.len() as i64, FIRST.as_bytes()],
    )
    .unwrap();
    conn.execute("UPDATE source_file SET len=6 WHERE sha256=?1", [&opaque])
        .unwrap();
    rederive_master_provenance(&conn).unwrap();
    assert_eq!(
        master_source_report(&conn, &sha).unwrap().unwrap().status,
        "complete"
    );
    assert_eq!(master_source_report(&conn, &opaque).unwrap(), None);
}

#[test]
fn late_writes_and_outer_release_failures_rollback_owned_changes() {
    use knx_productdb::master_provenance::rederive_master_provenance;
    let (_dir, conn) = db();
    let before = table_values(
        &conn,
        &[
            "source_file",
            "datapoint_type",
            "function_type",
            "function_point",
            "space_usage",
            "master_source",
            "master_declaration",
            "master_winner",
        ],
    );
    conn.execute_batch("CREATE TRIGGER fail_late BEFORE INSERT ON master_winner WHEN NEW.family='space_usage' BEGIN SELECT RAISE(ABORT,'late failure'); END").unwrap();
    assert!(ingest_master_data(&conn, FIRST.as_bytes()).is_err());
    assert!(conn.is_autocommit());
    assert_eq!(
        table_values(
            &conn,
            &[
                "source_file",
                "datapoint_type",
                "function_type",
                "function_point",
                "space_usage",
                "master_source",
                "master_declaration",
                "master_winner"
            ]
        ),
        before
    );
    conn.execute_batch("DROP TRIGGER fail_late; CREATE TABLE deferred_parent(id INTEGER PRIMARY KEY); CREATE TABLE deferred_child(id INTEGER REFERENCES deferred_parent(id) DEFERRABLE INITIALLY DEFERRED); CREATE TRIGGER fail_commit AFTER INSERT ON master_source BEGIN INSERT INTO deferred_child VALUES(1); END").unwrap();
    assert!(ingest_master_data(&conn, FIRST.as_bytes()).is_err());
    assert!(conn.is_autocommit());
    assert_eq!(number(&conn, "SELECT count(*) FROM deferred_child"), 0);
    assert_eq!(
        table_values(
            &conn,
            &[
                "source_file",
                "datapoint_type",
                "function_type",
                "function_point",
                "space_usage",
                "master_source",
                "master_declaration",
                "master_winner"
            ]
        ),
        before
    );
    // Replay has its own commit boundary, exercised with retained raw bytes.
    knx_productdb::store_source_file(
        &conn,
        &knx_productdb::SourceFile {
            source_path: "knx_master.xml".into(),
            manufacturer_id: None,
            bytes: FIRST.as_bytes().to_vec(),
        },
    )
    .unwrap();
    assert!(rederive_master_provenance(&conn).is_err());
    assert!(conn.is_autocommit());
    assert_eq!(number(&conn, "SELECT count(*) FROM master_source"), 0);
    assert_eq!(number(&conn, "SELECT count(*) FROM deferred_child"), 0);
}

fn package(master: &[u8], marker: &str) -> Vec<u8> {
    use std::io::{Cursor, Write};
    let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for (name, bytes) in [
        ("knx_master.xml", master),
        ("M-0001/Hardware.xml", marker.as_bytes()),
    ] {
        writer
            .start_file(name, zip::write::SimpleFileOptions::default())
            .unwrap();
        writer.write_all(bytes).unwrap();
    }
    writer.finish().unwrap().into_inner()
}
const HARDWARE: &str = r#"<KNX xmlns="http://knx.org/xml/project/11"><ManufacturerData><Manufacturer RefId="M-0001"><Hardware/></Manufacturer></ManufacturerData></KNX>"#;

#[test]
fn package_diagnostics_are_per_declaration_and_per_install_not_global_source_history() {
    use knx_productdb::{install_package, InstallDiagnosticKind};
    let (dir, conn) = db();
    let master = FIRST
        .replace("<KNX>", r#"<KNX xmlns="http://knx.org/xml/project/11">"#)
        .replace(
            "</MasterData>",
            "<SpaceUsage/><FunctionPoint/></MasterData>",
        );
    let archive = package(master.as_bytes(), HARDWARE);
    let first = install_package(&conn, "first.knxprod", &archive).unwrap();
    let facts = first.facts.unwrap();
    let selected = facts
        .diagnostics
        .iter()
        .filter(|d| {
            matches!(
                d.kind(),
                InstallDiagnosticKind::MasterIdentityCollision
                    | InstallDiagnosticKind::MalformedMasterDeclaration
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(selected.len(), 6);
    assert_eq!(
        selected
            .iter()
            .filter(|d| d.kind() == InstallDiagnosticKind::MasterIdentityCollision)
            .count(),
        4
    );
    assert_eq!(
        selected
            .iter()
            .filter(|d| d.kind() == InstallDiagnosticKind::MalformedMasterDeclaration)
            .count(),
        2
    );
    let unique = selected
        .iter()
        .map(|d| (d.kind(), d.archive_path(), d.xml_path()))
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(unique.len(), 6);
    for d in &selected {
        assert_eq!(d.archive_path(), "knx_master.xml");
        assert_eq!(d.occurrences(), 1);
        assert!(d.detail().contains(&sha256_hex(master.as_bytes())));
        assert!(d.detail().contains("bytes ["));
    }
    let second = install_package(
        &conn,
        "second.knxprod",
        &package(
            master.as_bytes(),
            &HARDWARE.replace("<Hardware/>", "<Hardware></Hardware>"),
        ),
    )
    .unwrap()
    .facts
    .unwrap();
    assert_eq!(
        second
            .diagnostics
            .iter()
            .filter(|d| d.kind() == InstallDiagnosticKind::MasterIdentityCollision)
            .count(),
        9
    );
    assert_eq!(
        second
            .diagnostics
            .iter()
            .filter(|d| d.kind() == InstallDiagnosticKind::MalformedMasterDeclaration)
            .count(),
        2
    );
    // Saved snapshots survive replay and source inspection failure.
    let snapshots = table_values(
        &conn,
        &[
            "package",
            "package_install_report",
            "package_install_count",
            "package_install_diagnostic",
        ],
    );
    conn.execute(
        "UPDATE source_file SET len=-1 WHERE sha256=?1",
        [sha256_hex(master.as_bytes())],
    )
    .unwrap();
    knx_productdb::master_provenance::rederive_master_provenance(&conn).unwrap();
    assert_eq!(
        table_values(
            &conn,
            &[
                "package",
                "package_install_report",
                "package_install_count",
                "package_install_diagnostic"
            ]
        ),
        snapshots
    );
    drop(conn);
    let conn = open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
    // Exact retries read their original historical report, not current collisions.
    assert_eq!(
        install_package(&conn, "retry.knxprod", &archive)
            .unwrap()
            .facts,
        Some(facts)
    );
}

#[test]
fn unusual_parser_contexts_and_rejected_parents_keep_existing_normalized_policy() {
    use knx_productdb::master_provenance::{master_source_report, rederive_master_provenance};
    let (_dir, conn) = db();
    let xml = r#"<KNX><MasterData><Opaque><DatapointType Id="A" Number="7"><DatapointType Id="B" Number="8"></DatapointType><DatapointSubtype Id="orphan-dpt"/></DatapointType><DatapointType Id="A" Number="9"><DatapointSubtype Id="S" Number="2"/></DatapointType><FunctionType Id="F"><FunctionType Id="nested"></FunctionType><FunctionPoint Id="orphan-fp"/></FunctionType><FunctionType Id="F"><FunctionPoint Id="P"/></FunctionType></Opaque></MasterData></KNX>"#;
    let outcome = ingest_master_data(&conn, xml.as_bytes()).unwrap();
    assert_eq!(
        number(&conn, "SELECT main FROM datapoint_type WHERE id='S'"),
        9
    );
    assert_eq!(
        number(
            &conn,
            "SELECT count(*) FROM datapoint_type WHERE id='orphan-dpt'"
        ),
        0
    );
    assert_eq!(
        number(
            &conn,
            "SELECT count(*) FROM function_point WHERE id='orphan-fp'"
        ),
        0
    );
    let source = master_source_report(&conn, &outcome.source_sha256)
        .unwrap()
        .unwrap();
    assert_eq!(
        source
            .declarations
            .iter()
            .filter(|d| d.orphan_parent)
            .map(|d| d.ordinal)
            .collect::<Vec<_>>(),
        vec![3, 8]
    );
    assert_eq!(source.declarations[4].main_number, Some(9));
    assert_eq!(
        source.declarations[9].function_type_id.as_deref(),
        Some("F")
    );
    assert_eq!(
        source.declarations[9].xml_path,
        "/KNX/MasterData/Opaque/FunctionType/FunctionPoint"
    );
    rederive_master_provenance(&conn).unwrap();
    assert_eq!(
        master_source_report(&conn, &outcome.source_sha256)
            .unwrap()
            .unwrap(),
        source
    );
}

#[test]
fn winner_query_rejects_wrong_family_id_or_unobserved_declaration_pointer() {
    use knx_productdb::master_provenance::{master_identity, MasterKind};
    let (_dir, conn) = db();
    ingest_master_data(&conn, FIRST.as_bytes()).unwrap();
    assert_eq!(
        master_identity(&conn, MasterKind::DatapointType, "D")
            .unwrap()
            .winner
            .unwrap()
            .ordinal,
        1
    );
    for ordinal in [2, 3, 4] {
        conn.execute(
            "UPDATE master_winner SET ordinal=?1 WHERE family='datapoint_type' AND logical_id='D'",
            [ordinal],
        )
        .unwrap();
        assert!(
            master_identity(&conn, MasterKind::DatapointType, "D").is_err(),
            "wrong winner ordinal {ordinal} accepted"
        );
    }
}

#[test]
fn first_observed_ingest_after_declaration_only_replay_records_real_winners() {
    use knx_productdb::master_provenance::{
        master_identity, master_source_report, rederive_master_provenance, MasterKind,
    };
    let (_dir, conn) = db();
    knx_productdb::ingest_file(&conn, "knx_master.xml", FIRST.as_bytes()).unwrap();
    rederive_master_provenance(&conn).unwrap();
    let sha = sha256_hex(FIRST.as_bytes());
    assert_eq!(
        master_source_report(&conn, &sha).unwrap().unwrap().history,
        "historical-unavailable"
    );
    ingest_master_data(&conn, FIRST.as_bytes()).unwrap();
    assert!(master_identity(&conn, MasterKind::DatapointType, "D").is_ok());
    let source = master_source_report(&conn, &sha).unwrap().unwrap();
    assert_eq!(source.history, "observed");
    assert_eq!(
        source
            .declarations
            .iter()
            .map(|d| d.observed_write)
            .collect::<Vec<_>>(),
        vec![
            Some(true),
            Some(true),
            Some(false),
            Some(true),
            Some(true),
            Some(false),
            Some(false),
            Some(true),
            Some(false)
        ]
    );
    assert!(source.retained_source_available);
    conn.execute("UPDATE source_file SET len=-1 WHERE sha256=?1", [&sha])
        .unwrap();
    assert!(
        !master_source_report(&conn, &sha)
            .unwrap()
            .unwrap()
            .retained_source_available
    );
    let report = master_identity(&conn, MasterKind::DatapointType, "D").unwrap();
    assert!(!report.winner.unwrap().retained_source_available);
    assert_eq!(report.unavailable_sources, vec![sha.clone()]);
    rederive_master_provenance(&conn).unwrap();
    conn.execute(
        "UPDATE source_file SET len=?2 WHERE sha256=?1",
        rusqlite::params![sha, FIRST.len() as i64],
    )
    .unwrap();
    rederive_master_provenance(&conn).unwrap();
    assert_eq!(master_source_report(&conn, &sha).unwrap().unwrap(), source);
}

#[test]
fn public_readers_reject_corrupt_header_declaration_and_report_vocabulary() {
    use knx_productdb::master_provenance::master_source_report;
    let (_dir, conn) = db();
    let master = FIRST.replace("<KNX>", r#"<KNX xmlns="http://knx.org/xml/project/11">"#);
    let archive = package(master.as_bytes(), HARDWARE);
    knx_productdb::install_package(&conn, "corruption.knxprod", &archive).unwrap();
    let sha = sha256_hex(master.as_bytes());
    assert!(master_source_report(&conn, &sha).unwrap().is_some());
    // Savepoint controls keep the original good database independently readable.
    for sql in [
        "UPDATE master_source SET definition_version=2",
        "UPDATE master_source SET history='invented'",
        "UPDATE master_source SET status='invented'",
        "UPDATE master_source SET reason='contradiction'",
        "UPDATE master_declaration SET missing_id=2 WHERE ordinal=1",
        "UPDATE master_declaration SET orphan_parent=2 WHERE ordinal=1",
        "UPDATE master_declaration SET observed_write=2 WHERE ordinal=1",
        "UPDATE master_declaration SET family='function_type' WHERE ordinal=1",
        "UPDATE master_declaration SET xml_path='/../bad' WHERE ordinal=1",
        "UPDATE master_declaration SET byte_end=1000000001 WHERE ordinal=1",
        "UPDATE master_declaration SET byte_end=999999 WHERE ordinal=1",
        "UPDATE master_declaration SET byte_start=byte_start+1 WHERE ordinal=1",
        "UPDATE master_declaration SET observed_write=NULL WHERE ordinal=1",
        "UPDATE master_declaration SET ordinal=10000 WHERE ordinal=9",
    ] {
        conn.execute_batch("SAVEPOINT corrupt; PRAGMA ignore_check_constraints=ON")
            .unwrap();
        conn.execute(sql, []).unwrap();
        assert!(master_source_report(&conn, &sha).is_err(), "accepted {sql}");
        conn.execute_batch(
            "ROLLBACK TO corrupt; RELEASE corrupt; PRAGMA ignore_check_constraints=OFF",
        )
        .unwrap();
    }
    for sql in [
        "UPDATE package_install_diagnostic SET kind='master-identity-collison'",
        "UPDATE package_install_diagnostic SET occurrences=2 WHERE kind='master-identity-collision'",
        "UPDATE package_install_diagnostic SET detail=detail||' forged' WHERE kind='master-identity-collision'",
        "UPDATE package_install_diagnostic SET xml_path='/KNX/DatapointType[@master-ordinal=0]' WHERE kind='master-identity-collision'",
    ] {
        conn.execute_batch("SAVEPOINT corrupt; PRAGMA ignore_check_constraints=ON").unwrap();
        conn.execute(sql,[]).unwrap();
        assert!(knx_productdb::install_package(&conn,"retry.knxprod",&archive).is_err(),"accepted {sql}");
        conn.execute_batch("ROLLBACK TO corrupt; RELEASE corrupt; PRAGMA ignore_check_constraints=OFF").unwrap();
    }
}

#[test]
fn migration_preserves_nonempty_historical_reports_and_rejects_schema_collision_atomically() {
    let (dir, conn) = db();
    let master = FIRST.replace("<KNX>", r#"<KNX xmlns="http://knx.org/xml/project/11">"#);
    let archive = package(master.as_bytes(), HARDWARE);
    knx_productdb::install_package(&conn, "before.knxprod", &archive).unwrap();
    // A v26 snapshot cannot contain the new diagnostics; all other rows stay literal.
    conn.execute("DELETE FROM package_install_diagnostic WHERE kind IN ('master-identity-collision','malformed-master-declaration')",[]).unwrap();
    let tables = [
        "package",
        "datapoint_type",
        "function_type",
        "function_point",
        "space_usage",
        "package_install_report",
        "package_install_count",
        "package_install_diagnostic",
    ];
    let before = table_values(&conn, &tables);
    conn.execute_batch("DROP TABLE master_winner; DROP TABLE master_declaration; DROP TABLE master_source; PRAGMA user_version=26; CREATE TABLE master_source(collision TEXT)").unwrap();
    drop(conn);
    let path = dir.path().join("products.sqlite");
    assert!(open_and_migrate(&path).is_err());
    let conn = Connection::open(&path).unwrap();
    assert_eq!(number(&conn, "PRAGMA user_version"), 26);
    assert_eq!(table_values(&conn, &tables), before);
    conn.execute_batch("DROP TABLE master_source").unwrap();
    drop(conn);
    let conn = open_and_migrate(&path).unwrap();
    assert_eq!(table_values(&conn, &tables), before);
    let facts = knx_productdb::install_package(&conn, "retry.knxprod", &archive)
        .unwrap()
        .facts
        .unwrap();
    assert!(!facts.diagnostics.iter().any(|d| matches!(
        d.kind(),
        knx_productdb::InstallDiagnosticKind::MasterIdentityCollision
            | knx_productdb::InstallDiagnosticKind::MalformedMasterDeclaration
    )));
}

#[test]
fn bounded_depth_late_replay_write_failure_and_nested_ownership_are_explicit() {
    use knx_productdb::master_provenance::rederive_master_provenance;
    let (_dir, conn) = db();
    let deep = format!(
        "<KNX><MasterData>{}<SpaceUsage Id='deep'/>{}</MasterData></KNX>",
        "<X>".repeat(4096),
        "</X>".repeat(4096)
    );
    let before = table_values(&conn, &["source_file", "master_source", "space_usage"]);
    assert!(ingest_master_data(&conn, deep.as_bytes())
        .err()
        .unwrap()
        .to_string()
        .contains("depth budget"));
    assert_eq!(
        table_values(&conn, &["source_file", "master_source", "space_usage"]),
        before
    );
    knx_productdb::store_source_file(
        &conn,
        &knx_productdb::SourceFile {
            source_path: "knx_master.xml".into(),
            manufacturer_id: None,
            bytes: FIRST.as_bytes().to_vec(),
        },
    )
    .unwrap();
    conn.execute_batch("CREATE TRIGGER replay_late BEFORE INSERT ON master_declaration WHEN NEW.ordinal=9 BEGIN SELECT RAISE(ABORT,'late replay write'); END").unwrap();
    assert!(rederive_master_provenance(&conn).is_err());
    assert_eq!(number(&conn, "SELECT count(*) FROM master_source"), 0);
    assert_eq!(number(&conn, "SELECT count(*) FROM master_declaration"), 0);
    conn.execute_batch("DROP TRIGGER replay_late; BEGIN IMMEDIATE; INSERT INTO manufacturer(id,name) VALUES('caller','keep until rollback')").unwrap();
    rederive_master_provenance(&conn).unwrap();
    assert!(!conn.is_autocommit());
    assert_eq!(number(&conn, "SELECT count(*) FROM master_source"), 1);
    conn.execute_batch("ROLLBACK").unwrap();
    assert_eq!(number(&conn, "SELECT count(*) FROM master_source"), 0);
    assert_eq!(
        number(&conn, "SELECT count(*) FROM manufacturer WHERE id='caller'"),
        0
    );
    // Binary baggage and raw XML without a MasterData classification are not imported.
    let opaque = b"<KNX><SpaceUsage Id='must-stay-opaque'/></KNX>";
    knx_productdb::ingest_file(&conn, "unknown.xml", opaque).unwrap();
    rederive_master_provenance(&conn).unwrap();
    assert_eq!(number(&conn, "SELECT count(*) FROM space_usage"), 0);
    assert_eq!(
        number(
            &conn,
            "SELECT count(*) FROM master_declaration WHERE logical_id='must-stay-opaque'"
        ),
        0
    );
}
