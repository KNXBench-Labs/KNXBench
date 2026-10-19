//! Regression witnesses for WP17-R1-01..06; shared literal fixtures in parent.
use super::*;
use knx_productdb::master_provenance::{
    master_identity, master_source_report, rederive_master_provenance, MasterKind,
};

#[test]
fn tracked_document_roots_are_queryable_and_replayable() {
    let (_dir, conn) = db();
    for xml in [r#"<SpaceUsage Id="root"/>"#, "<SpaceUsage/>"] {
        let outcome = ingest_master_data(&conn, xml.as_bytes()).unwrap();
        let source = master_source_report(&conn, &outcome.source_sha256)
            .unwrap()
            .unwrap();
        assert_eq!(source.declarations.len(), 1);
        assert_eq!(source.declarations[0].xml_path, "/SpaceUsage");
        assert_eq!(source.declarations[0].missing_id, xml == "<SpaceUsage/>");
        rederive_master_provenance(&conn).unwrap();
        assert_eq!(
            master_source_report(&conn, &outcome.source_sha256)
                .unwrap()
                .unwrap(),
            source
        );
    }
    let identity = master_identity(&conn, MasterKind::SpaceUsage, "root").unwrap();
    assert_eq!(identity.winner.unwrap().ordinal, 1);
    assert_eq!(identity.candidates.len(), 1);
}

#[test]
fn winner_markers_distinguish_explicit_history_from_missing_or_broken_pairs() {
    let (_dir, conn) = db();
    ingest_master_data(&conn, FIRST.as_bytes()).unwrap();
    assert!(
        !master_identity(&conn, MasterKind::SpaceUsage, "absent")
            .unwrap()
            .normalized_present
    );
    for sql in [
        "DELETE FROM master_winner WHERE family='space_usage' AND logical_id='U'",
        "UPDATE master_winner SET source_sha256=NULL WHERE family='space_usage' AND logical_id='U'",
        "UPDATE master_winner SET ordinal=NULL WHERE family='space_usage' AND logical_id='U'",
    ] {
        conn.execute_batch("SAVEPOINT damage; PRAGMA ignore_check_constraints=ON")
            .unwrap();
        conn.execute(sql, []).unwrap();
        assert!(
            master_identity(&conn, MasterKind::SpaceUsage, "U").is_err(),
            "accepted {sql}"
        );
        assert!(
            ingest_master_data(&conn, FIRST.as_bytes()).is_err(),
            "ingest hid missing/broken marker for {sql}"
        );
        conn.execute_batch(
            "ROLLBACK TO damage; RELEASE damage; PRAGMA ignore_check_constraints=OFF",
        )
        .unwrap();
    }
    conn.execute("UPDATE master_winner SET source_sha256=NULL,ordinal=NULL WHERE family='space_usage' AND logical_id='U'", []).unwrap();
    let historical = master_identity(&conn, MasterKind::SpaceUsage, "U").unwrap();
    assert!(historical.normalized_present);
    assert_eq!(historical.winner, None);
}

#[test]
fn replay_refuses_missing_or_broken_winner_markers_without_inventing_history() {
    let (_dir, conn) = db();
    ingest_master_data(&conn, FIRST.as_bytes()).unwrap();
    // Normal replay and explicit paired-NULL history are both legitimate.
    rederive_master_provenance(&conn).unwrap();
    for (family, id) in [
        ("datapoint_type", "D"),
        ("datapoint_type", "S"),
        ("function_type", "F"),
        ("function_point", "P"),
        ("space_usage", "U"),
    ] {
        for change in [
            "DELETE FROM master_winner WHERE family=?1 AND logical_id=?2",
            "UPDATE master_winner SET source_sha256=NULL WHERE family=?1 AND logical_id=?2",
            "UPDATE master_winner SET ordinal=NULL WHERE family=?1 AND logical_id=?2",
        ] {
            conn.execute_batch("SAVEPOINT damage; PRAGMA ignore_check_constraints=ON")
                .unwrap();
            conn.execute(change, [family, id]).unwrap();
            let tables = [
                "master_source",
                "master_declaration",
                "master_winner",
                "datapoint_type",
                "function_type",
                "function_point",
                "space_usage",
            ];
            let before = table_values(&conn, &tables);
            assert!(
                rederive_master_provenance(&conn).is_err(),
                "replay concealed marker loss: {family}/{id}: {change}"
            );
            assert!(
                !conn.is_autocommit(),
                "replay committed the caller's transaction"
            );
            assert_eq!(table_values(&conn, &tables), before);
            conn.execute_batch(
                "ROLLBACK TO damage; RELEASE damage; PRAGMA ignore_check_constraints=OFF",
            )
            .unwrap();
        }
    }
    conn.execute(
        "UPDATE master_winner SET source_sha256=NULL,ordinal=NULL",
        [],
    )
    .unwrap();
    let before = table_values(&conn, &["master_winner"]);
    rederive_master_provenance(&conn).unwrap();
    assert_eq!(table_values(&conn, &["master_winner"]), before);
    assert!(conn.is_autocommit());
}

#[test]
fn replay_reconciles_derived_fields_and_unobserved_inventory() {
    let (_dir, conn) = db();
    ingest_master_data(&conn, FIRST.as_bytes()).unwrap();
    let sha = sha256_hex(FIRST.as_bytes());
    let expected = master_source_report(&conn, &sha).unwrap().unwrap();
    // Stable winner key: only byte-derived fields are damaged.
    conn.execute("UPDATE master_declaration SET byte_end=byte_end+1,xml_path='/wrong/DatapointType',main_number=888 WHERE ordinal=1", []).unwrap();
    rederive_master_provenance(&conn).unwrap();
    assert_eq!(
        master_source_report(&conn, &sha).unwrap().unwrap(),
        expected
    );
    assert_eq!(
        master_identity(&conn, MasterKind::DatapointType, "D")
            .unwrap()
            .winner
            .unwrap()
            .ordinal,
        1
    );
    // Historical declaration-only inventories have no outcomes to lose.
    conn.execute_batch("UPDATE master_winner SET source_sha256=NULL,ordinal=NULL; UPDATE master_source SET history='historical-unavailable'; UPDATE master_declaration SET observed_write=NULL; DELETE FROM master_declaration WHERE ordinal=2; UPDATE master_declaration SET logical_id='damaged' WHERE ordinal=1; INSERT INTO master_declaration SELECT source_sha256,10,family,element,logical_id,xml_path,byte_start,byte_end,main_number,function_type_id,missing_id,orphan_parent,NULL FROM master_declaration WHERE ordinal=9").unwrap();
    rederive_master_provenance(&conn).unwrap();
    let repaired = master_source_report(&conn, &sha).unwrap().unwrap();
    assert_eq!(repaired.declarations.len(), 9);
    assert_eq!(repaired.declarations[0].logical_id.as_deref(), Some("D"));
    assert!(repaired
        .declarations
        .iter()
        .all(|d| d.observed_write.is_none()));
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
}

#[test]
fn replay_atomically_refuses_lost_outcomes_or_ambiguous_winner_keys() {
    for sql in [
        "UPDATE master_declaration SET logical_id='wrong' WHERE ordinal=1",
        "DELETE FROM master_declaration WHERE ordinal=1",
        "DELETE FROM master_declaration WHERE ordinal=9",
        "UPDATE master_source SET definition_version=2",
        "UPDATE master_declaration SET observed_write=NULL WHERE ordinal=9",
        "UPDATE master_declaration SET ordinal=10 WHERE ordinal=9",
        "UPDATE master_winner SET ordinal=10 WHERE family='space_usage'; UPDATE master_declaration SET ordinal=10 WHERE ordinal=8",
        "INSERT INTO master_declaration SELECT source_sha256,10,family,element,logical_id,xml_path,byte_start,byte_end,main_number,function_type_id,missing_id,orphan_parent,NULL FROM master_declaration WHERE ordinal=8; UPDATE master_winner SET ordinal=10 WHERE family='space_usage'",
    ] {
        let (_dir, conn) = db();
        ingest_master_data(&conn, FIRST.as_bytes()).unwrap();
        conn.execute_batch("PRAGMA foreign_keys=OFF; PRAGMA ignore_check_constraints=ON").unwrap();
        conn.execute_batch(sql).unwrap();
        conn.execute_batch("PRAGMA foreign_keys=ON").unwrap();
        let tables = ["master_source", "master_declaration", "master_winner", "datapoint_type", "space_usage"];
        let damaged = table_values(&conn, &tables);
        assert!(rederive_master_provenance(&conn).is_err(), "replay invented history for {sql}");
        assert!(conn.is_autocommit());
        assert_eq!(table_values(&conn, &tables), damaged, "partial repair for {sql}");
    }
}

#[test]
fn persisted_reports_validate_kept_and_losing_inventories_on_reopen_retry() {
    for (source, sql) in [
        (true, "UPDATE master_declaration SET byte_end=byte_end+1 WHERE source_sha256=?1 AND ordinal=1"),
        (false, "UPDATE master_declaration SET main_number=888 WHERE source_sha256=?1 AND ordinal=2"),
        (true, "UPDATE master_source SET definition_version=2 WHERE source_sha256=?1"),
        (false, "UPDATE master_source SET history='invented' WHERE source_sha256=?1"),
    ] {
        let (dir, conn) = db();
        let a = FIRST.replace("<KNX>", r#"<KNX xmlns="http://knx.org/xml/project/11">"#);
        let b = SECOND.replace("<KNX>", r#"<KNX xmlns="http://knx.org/xml/project/11">"#);
        knx_productdb::install_package(&conn, "a.knxprod", &package(a.as_bytes(), HARDWARE)).unwrap();
        let archive = package(b.as_bytes(), HARDWARE);
        let facts = knx_productdb::install_package(&conn, "b.knxprod", &archive).unwrap().facts;
        assert_eq!(knx_productdb::install_package(&conn, "b.knxprod", &archive).unwrap().facts, facts);
        conn.execute_batch("PRAGMA ignore_check_constraints=ON").unwrap();
        conn.execute(sql, [sha256_hex(if source {a.as_bytes()} else {b.as_bytes()})]).unwrap();
        let snapshots = table_values(&conn, &["package_install_report", "package_install_count", "package_install_diagnostic"]);
        drop(conn);
        let conn = open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
        assert!(knx_productdb::install_package(&conn, "retry.knxprod", &archive).is_err(), "report accepted {sql}");
        assert_eq!(table_values(&conn, &["package_install_report", "package_install_count", "package_install_diagnostic"]), snapshots);
    }
}

#[test]
fn missing_or_hash_invalid_bytes_do_not_rewrite_historical_report_snapshots() {
    for source_a in [false, true] {
        for missing in [false, true] {
            let (dir, conn) = db();
            let a = FIRST.replace("<KNX>", r#"<KNX xmlns="http://knx.org/xml/project/11">"#);
            let b = SECOND.replace("<KNX>", r#"<KNX xmlns="http://knx.org/xml/project/11">"#);
            knx_productdb::install_package(&conn, "a.knxprod", &package(a.as_bytes(), HARDWARE))
                .unwrap();
            let archive = package(b.as_bytes(), HARDWARE);
            let facts = knx_productdb::install_package(&conn, "b.knxprod", &archive)
                .unwrap()
                .facts;
            let sha = sha256_hex(if source_a { a.as_bytes() } else { b.as_bytes() });
            conn.execute_batch("PRAGMA foreign_keys=OFF").unwrap();
            conn.execute(
                if missing {
                    "DELETE FROM source_file WHERE sha256=?1"
                } else {
                    "UPDATE source_file SET bytes=zeroblob(length(bytes)) WHERE sha256=?1"
                },
                [&sha],
            )
            .unwrap();
            conn.execute_batch("PRAGMA foreign_keys=ON").unwrap();
            let snapshots = table_values(
                &conn,
                &[
                    "package_install_report",
                    "package_install_count",
                    "package_install_diagnostic",
                ],
            );
            drop(conn);
            let conn = open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
            assert!(
                !master_source_report(&conn, &sha)
                    .unwrap()
                    .unwrap()
                    .retained_source_available
            );
            assert_eq!(
                knx_productdb::install_package(&conn, "retry.knxprod", &archive)
                    .unwrap()
                    .facts,
                facts
            );
            assert_eq!(
                table_values(
                    &conn,
                    &[
                        "package_install_report",
                        "package_install_count",
                        "package_install_diagnostic"
                    ]
                ),
                snapshots
            );
        }
    }
}

fn package_snapshots(conn: &Connection, sha: &str) -> Vec<Vec<Vec<rusqlite::types::Value>>> {
    [
        "package_install_report",
        "package_install_count",
        "package_install_diagnostic",
    ]
    .iter()
    .map(|table| {
        let mut s = conn
            .prepare(&format!(
                "SELECT * FROM {table} WHERE package_sha256=?1 ORDER BY rowid"
            ))
            .unwrap();
        let n = s.column_count();
        s.query_map([sha], |r| (0..n).map(|i| r.get(i)).collect())
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap()
    })
    .collect()
}

#[test]
fn historical_winners_stay_unavailable_after_first_real_observation_and_package_reuse() {
    let (dir, conn) = db();
    let master = FIRST.replace("<KNX>", r#"<KNX xmlns="http://knx.org/xml/project/11">"#);
    let old = package(master.as_bytes(), HARDWARE);
    let old_sha = knx_productdb::install_package(&conn, "old.knxprod", &old)
        .unwrap()
        .sha256;
    conn.execute_batch("DELETE FROM package_install_diagnostic WHERE kind IN ('master-identity-collision','malformed-master-declaration'); UPDATE datapoint_type SET main=1234,sub=CASE WHEN sub IS NULL THEN NULL ELSE 5678 END; UPDATE function_type SET number=2345; UPDATE function_point SET role='historical'; UPDATE space_usage SET number=3456").unwrap();
    let tables = [
        "datapoint_type",
        "function_type",
        "function_point",
        "space_usage",
    ];
    let normalized = table_values(&conn, &tables);
    let old_snapshots = package_snapshots(&conn, &old_sha);
    conn.execute_batch("DROP TABLE master_winner; DROP TABLE master_declaration; DROP TABLE master_source; PRAGMA user_version=26").unwrap();
    drop(conn);
    let conn = open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
    assert_eq!(table_values(&conn, &tables), normalized);
    assert_eq!(package_snapshots(&conn, &old_sha), old_snapshots);
    let source_sha = sha256_hex(master.as_bytes());
    assert!(master_source_report(&conn, &source_sha)
        .unwrap()
        .unwrap()
        .declarations
        .iter()
        .all(|d| d.observed_write.is_none()));
    let observed = ingest_master_data(&conn, master.as_bytes()).unwrap();
    assert_eq!(observed.diagnostics.len(), 9);
    assert!(observed
        .diagnostics
        .iter()
        .all(|d| d.detail.contains("kept winner historical-unavailable")));
    let new_archive = package(
        master.as_bytes(),
        &HARDWARE.replace("<Hardware/>", "<Hardware></Hardware>"),
    );
    let facts = knx_productdb::install_package(&conn, "reuse.knxprod", &new_archive)
        .unwrap()
        .facts
        .unwrap();
    let collisions = facts
        .diagnostics
        .iter()
        .filter(|d| d.kind() == knx_productdb::InstallDiagnosticKind::MasterIdentityCollision)
        .collect::<Vec<_>>();
    assert_eq!(collisions.len(), 9);
    assert!(collisions
        .iter()
        .all(|d| d.detail().contains("kept winner historical-unavailable")));
    for conn in [
        conn,
        open_and_migrate(&dir.path().join("products.sqlite")).unwrap(),
    ] {
        ingest_master_data(&conn, master.as_bytes()).unwrap();
        rederive_master_provenance(&conn).unwrap();
        assert_eq!(table_values(&conn, &tables), normalized);
        assert_eq!(package_snapshots(&conn, &old_sha), old_snapshots);
        assert_eq!(
            knx_productdb::install_package(&conn, "retry.knxprod", &new_archive)
                .unwrap()
                .facts,
            Some(facts.clone())
        );
        knx_productdb::install_package(&conn, "old-retry.knxprod", &old).unwrap();
        assert_eq!(package_snapshots(&conn, &old_sha), old_snapshots);
        assert_eq!(number(&conn, "SELECT count(*) FROM master_winner WHERE source_sha256 IS NULL AND ordinal IS NULL"), 5);
        for (kind, id) in [
            (MasterKind::DatapointType, "D"),
            (MasterKind::DatapointType, "S"),
            (MasterKind::FunctionType, "F"),
            (MasterKind::FunctionPoint, "P"),
            (MasterKind::SpaceUsage, "U"),
        ] {
            let identity = master_identity(&conn, kind, id).unwrap();
            assert!(identity.normalized_present);
            assert_eq!(identity.winner, None);
            assert!(identity
                .candidates
                .iter()
                .all(|d| d.observed_write == Some(false)));
        }
    }
}

fn amplified_master(wrapper_len: usize, id_len: usize, repeats: usize, points: bool) -> String {
    let wrapper = "X".repeat(wrapper_len);
    let id = "i".repeat(id_len);
    let declarations = if points {
        format!(
            "<FunctionType Id='{id}'>{}</FunctionType>",
            "<FunctionPoint Id='P'/>".repeat(repeats)
        )
    } else {
        format!("<SpaceUsage Id='{id}'/>").repeat(repeats)
    };
    format!("<KNX xmlns='http://knx.org/xml/project/11'><MasterData><{wrapper}>{declarations}</{wrapper}></MasterData></KNX>")
}

#[test]
fn evidence_byte_budget_admits_bounded_long_paths_and_refuses_amplification_atomically() {
    let (_dir, conn) = db();
    let positive = amplified_master(8192, 1, 8, false);
    let outcome = ingest_master_data(&conn, positive.as_bytes()).unwrap();
    assert_eq!(outcome.diagnostics.len(), 7);
    assert_eq!(
        master_source_report(&conn, &outcome.source_sha256)
            .unwrap()
            .unwrap()
            .declarations
            .len(),
        8
    );
    rederive_master_provenance(&conn).unwrap();
    let tables = [
        "source_file",
        "master_source",
        "master_declaration",
        "master_winner",
        "space_usage",
        "function_type",
        "function_point",
    ];
    let before = table_values(&conn, &tables);
    for xml in [
        amplified_master(8192, 1, 1024, false),
        amplified_master(1, 8192, 128, false),
        amplified_master(1, 8192, 2048, true),
    ] {
        let err = ingest_master_data(&conn, xml.as_bytes())
            .err()
            .expect("amplified evidence admitted");
        assert!(err.to_string().contains("evidence byte budget"), "{err}");
        assert!(conn.is_autocommit());
        assert_eq!(table_values(&conn, &tables), before);
    }
}

#[test]
fn persisted_source_and_report_readers_refuse_evidence_byte_exhaustion() {
    let (_dir, conn) = db();
    let xml = amplified_master(1, 1, 600, false);
    let archive = package(xml.as_bytes(), HARDWARE);
    knx_productdb::install_package(&conn, "budget.knxprod", &archive).unwrap();
    let sha = sha256_hex(xml.as_bytes());
    // Available bytes need not be materialized to reject oversized evidence.
    conn.execute_batch("SAVEPOINT damage; UPDATE master_declaration SET xml_path='/'||replace(hex(zeroblob(16384)),'0','X')||'/SpaceUsage'").unwrap();
    let err = master_source_report(&conn, &sha).unwrap_err();
    assert!(err.to_string().contains("evidence byte budget"), "{err}");
    conn.execute_batch("ROLLBACK TO damage; RELEASE damage; UPDATE package_install_diagnostic SET detail=detail||replace(hex(zeroblob(16384)),'0','X') WHERE kind='master-identity-collision'").unwrap();
    let err = knx_productdb::install_package(&conn, "retry.knxprod", &archive).unwrap_err();
    assert!(err.to_string().contains("evidence byte budget"), "{err}");
    conn.execute("UPDATE package_install_diagnostic SET detail=substr(detail,1,length(detail)-32768) WHERE kind='master-identity-collision'", []).unwrap();
    assert!(
        knx_productdb::install_package(&conn, "restored.knxprod", &archive)
            .unwrap()
            .facts
            .is_some()
    );
}

#[test]
fn available_bytes_still_validate_inventories_after_a_failed_inspection_marker() {
    let (_dir, conn) = db();
    let master = FIRST.replace("<KNX>", r#"<KNX xmlns="http://knx.org/xml/project/11">"#);
    let archive = package(master.as_bytes(), HARDWARE);
    knx_productdb::install_package(&conn, "a.knxprod", &archive).unwrap();
    conn.execute_batch("UPDATE master_source SET status='failed',reason='previous inspection failed'; UPDATE master_declaration SET byte_end=byte_end+1 WHERE ordinal=1").unwrap();
    assert!(
        knx_productdb::install_package(&conn, "retry.knxprod", &archive).is_err(),
        "failed marker hid contradictory available inventory"
    );
}

#[test]
fn replay_budget_exhaustion_rolls_back_all_owned_evidence_without_truncation() {
    let (_dir, conn) = db();
    ingest_master_data(&conn, FIRST.as_bytes()).unwrap();
    let xml = amplified_master(8192, 1, 2048, false);
    knx_productdb::store_source_file(
        &conn,
        &knx_productdb::SourceFile {
            source_path: "knx_master.xml".into(),
            manufacturer_id: None,
            bytes: xml.into_bytes(),
        },
    )
    .unwrap();
    let tables = [
        "source_file",
        "master_source",
        "master_declaration",
        "master_winner",
        "datapoint_type",
        "space_usage",
    ];
    let before = table_values(&conn, &tables);
    let err = rederive_master_provenance(&conn).unwrap_err();
    assert!(err.to_string().contains("evidence byte budget"), "{err}");
    assert!(conn.is_autocommit());
    assert_eq!(table_values(&conn, &tables), before);
}
