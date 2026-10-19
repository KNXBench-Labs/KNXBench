//! Independent review witnesses: role, admission and historical attribution.
use knx_productdb::{
    ingest_file, install_package, open_and_migrate, sha256_hex, source_diagnostics,
};
use rusqlite::{params, types::Value, Connection};
use std::io::{Cursor, Write};
use zip::write::SimpleFileOptions;

const MASTER: &str = r#"<KNX xmlns="http://knx.org/xml/project/20"><MasterData><Manufacturers><Manufacturer Id="M-0001" Name="Example"/></Manufacturers><Unmodelled/></MasterData></KNX>"#;
fn xml(id: &str, sample: &str, nonce: usize) -> String {
    format!(
        r#"<KNX xmlns="http://knx.org/xml/project/20"><!--{nonce}--><ManufacturerData><Manufacturer RefId="M-0001"><ApplicationPrograms><ApplicationProgram Id="{id}"><Static><ParameterBlock Access="{sample}"/></Static></ApplicationProgram></ApplicationPrograms></Manufacturer></ManufacturerData></KNX>"#
    )
}
fn package(master: &str, members: &[(&str, &str)]) -> Vec<u8> {
    let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for (path, bytes) in std::iter::once(("knx_master.xml", master)).chain(members.iter().copied())
    {
        zip.start_file(path, SimpleFileOptions::default()).unwrap();
        zip.write_all(bytes.as_bytes()).unwrap();
    }
    zip.finish().unwrap().into_inner()
}
fn rows(c: &Connection, sql: &str) -> Vec<Vec<Value>> {
    let mut s = c.prepare(sql).unwrap();
    let n = s.column_count();
    s.query_map([], |r| (0..n).map(|i| r.get(i)).collect())
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap()
}
fn stable(c: &Connection) -> Vec<Vec<Vec<Value>>> {
    ["SELECT * FROM source_file ORDER BY sha256", "SELECT * FROM package_member ORDER BY package_sha256,ordinal", "SELECT sha256,unknown_count FROM package ORDER BY sha256", "SELECT * FROM package_install_count WHERE category<>'unknown_construct' ORDER BY package_sha256,ordinal", "SELECT * FROM package_install_diagnostic ORDER BY package_sha256,ordinal"].iter().map(|s|rows(c,s)).collect()
}
#[test]
fn opaque_same_sha_does_not_inherit_program_evidence() {
    opaque_role_case(false);
}
#[test]
fn program_and_opaque_same_sha_count_program_exactly_once() {
    opaque_role_case(true);
}
fn opaque_role_case(program_member: bool) {
    {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("products.sqlite");
        let c = open_and_migrate(&path).unwrap();
        let program = xml("A", "Read", 0);
        ingest_file(&c, "standalone.xml", program.as_bytes()).unwrap();
        let dummy = xml("B", "Read", 0).replace("<ParameterBlock Access=\"Read\"/>", "");
        let mut members = vec![("M-0001/Baggages/copy.xml", program.as_str())];
        members.insert(
            0,
            (
                "M-0001/A.xml",
                if program_member {
                    program.as_str()
                } else {
                    dummy.as_str()
                },
            ),
        );
        let bytes = package(MASTER, &members);
        install_package(&c, "roles.knxprod", &bytes).unwrap();
        let before = stable(&c);
        let report = rows(
            &c,
            "SELECT * FROM package_install_unknown ORDER BY package_sha256,ordinal",
        );
        let access:i64 = c.query_row("SELECT coalesce(sum(occurrences),0) FROM package_install_unknown WHERE name='Access'",[],|r|r.get(0)).unwrap();
        assert_eq!(access, i64::from(program_member));
        c.pragma_update(None, "user_version", 27).unwrap();
        drop(c);
        let c = open_and_migrate(&path).unwrap();
        assert_eq!(
            rows(
                &c,
                "SELECT * FROM package_install_unknown ORDER BY package_sha256,ordinal"
            ),
            report,
            "opaque member never acquires source semantics"
        );
        assert_eq!(stable(&c), before);
        install_package(&c, "roles.knxprod", &bytes).unwrap();
        assert_eq!(
            rows(
                &c,
                "SELECT * FROM package_install_unknown ORDER BY package_sha256,ordinal"
            ),
            report
        );
    }
}
#[test]
fn nonprogram_missing_target_ledger_is_not_authenticated_by_retained_bytes() {
    nonprogram_ledger_case("missing");
}
#[test]
fn nonprogram_invented_target_ledger_is_not_authenticated_by_retained_bytes() {
    nonprogram_ledger_case("invented");
}
fn nonprogram_ledger_case(fault: &str) {
    {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("products.sqlite");
        let c = open_and_migrate(&path).unwrap();
        let master = MASTER.replace(
            "<Unmodelled/>",
            "<Languages><Parameter Offset=\"master-offset\"/></Languages>",
        );
        let program=xml("A","Read",0).replace("</KNX>","<MasterData><Languages><Parameter Offset=\"program-offset\"/></Languages></MasterData></KNX>");
        install_package(
            &c,
            "overlap.knxprod",
            &package(&master, &[("M-0001/A.xml", &program)]),
        )
        .unwrap();
        assert_eq!(
            rows(
                &c,
                "SELECT occurrences,sample FROM package_install_unknown WHERE name='Offset'"
            ),
            vec![vec![Value::Integer(2), Value::Text("master-offset".into())]]
        );
        let counts = rows(
            &c,
            "SELECT sha256,unknown_count FROM package ORDER BY sha256",
        );
        if fault == "missing" {
            c.execute(
                "DELETE FROM ingest_unknown WHERE source_sha256=?1 AND name='Offset'",
                [sha256_hex(master.as_bytes())],
            )
            .unwrap();
        } else {
            c.execute("UPDATE ingest_unknown SET occurrences=7,sample='invented' WHERE source_sha256=?1 AND name='Offset'",[sha256_hex(master.as_bytes())]).unwrap();
        }
        c.pragma_update(None, "user_version", 27).unwrap();
        drop(c);
        let c = open_and_migrate(&path).unwrap();
        assert_eq!(
            rows(&c, "SELECT status FROM package_install_report"),
            vec![vec![Value::Text("unavailable".into())]],
            "unverified nonprogram contribution must not become measured history: {fault}"
        );
        assert_eq!(
            rows(
                &c,
                "SELECT sha256,unknown_count FROM package ORDER BY sha256"
            ),
            counts
        );
        let reason: String = c
            .query_row(
                "SELECT sample FROM ingest_unknown WHERE xpath='promotion_report_backfill'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert!(reason.contains("non-program"));
    }
}
#[test]
fn failed_large_source_does_not_spend_retained_inventory_budget() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("products.sqlite");
    let c = open_and_migrate(&path).unwrap();
    let large = xml("Large", &"x".repeat(8_387_602), 0);
    let sha = sha256_hex(large.as_bytes());
    let small = (0..10000)
        .map(|n| xml("Small", "small", n))
        .find(|s| sha256_hex(s.as_bytes()) > sha)
        .unwrap();
    let mut small_package = String::new();
    for program in [&large, &small] {
        let bytes = package(MASTER, &[("M-0001/A.xml", program)]);
        if program == &small {
            small_package = sha256_hex(&bytes);
        }
        install_package(&c, "budget.knxprod", &bytes).unwrap();
    }
    c.execute_batch("INSERT INTO parameter(program_id,id,own_offset) VALUES('Large','not-in-source',99); PRAGMA user_version=27;").unwrap();
    drop(c);
    let c = open_and_migrate(&path).unwrap();
    let small_errors:i64=c.query_row("SELECT count(*) FROM ingest_unknown WHERE source_sha256=?1 AND kind='PromotionRepairError'",[sha256_hex(small.as_bytes())],|r|r.get(0)).unwrap();
    assert_eq!(
        small_errors, 0,
        "discarded source inventory must not charge a later source"
    );
    let status: String = c
        .query_row(
            "SELECT status FROM package_install_report WHERE package_sha256=?1",
            [small_package],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(status, "measured");
}
#[test]
fn oversized_evidence_is_refused_before_semantic_parser_or_full_document_copy() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("products.sqlite");
    let c = open_and_migrate(&path).unwrap();
    // A borrowed lexical admission can see this resource excess before the
    // semantic parser or complete-document validator encounters the bad suffix.
    // Their deterministic error is the oracle, not time or process RSS.
    let program = xml("A", &"x".repeat(9 * 1024 * 1024), 0) + "<broken";
    let sha = sha256_hex(program.as_bytes());
    c.execute(
        "INSERT INTO source_file(sha256,source_path,len,bytes) VALUES(?1,'large.xml',?2,?3)",
        params![sha, program.len() as i64, program.as_bytes()],
    )
    .unwrap();
    c.execute(
        "INSERT INTO application_program(id,manufacturer_id,source_sha256) VALUES('A','M-0001',?1)",
        [&sha],
    )
    .unwrap();
    c.pragma_update(None, "user_version", 27).unwrap();
    drop(c);
    let c = open_and_migrate(&path).unwrap();
    let reason:String=c.query_row("SELECT sample FROM ingest_unknown WHERE source_sha256=?1 AND kind='PromotionRepairError'",[sha],|r|r.get(0)).unwrap();
    assert!(
        reason.contains("preparse evidence admission"),
        "resource excess must refuse before parser work: {reason}"
    );
}
#[test]
fn missing_measurement_marker_surfaces_failure_without_inventing_measurement() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("products.sqlite");
    let c = open_and_migrate(&path).unwrap();
    let program = xml("A", "Read", 0);
    ingest_file(&c, "program.xml", program.as_bytes()).unwrap();
    let sha = sha256_hex(program.as_bytes());
    c.execute("DELETE FROM source_parse_evidence WHERE sha256=?1", [&sha])
        .unwrap();
    c.execute("UPDATE source_file SET len=len+1 WHERE sha256=?1", [&sha])
        .unwrap();
    c.pragma_update(None, "user_version", 27).unwrap();
    drop(c);
    let c = open_and_migrate(&path).unwrap();
    let result = source_diagnostics(&c, &sha);
    assert!(
        result.is_err(),
        "a repair failure must be visible without returning measured Some: {result:?}"
    );
    assert!(result.unwrap_err().to_string().contains("promotion repair"));
    let measured: i64 = c
        .query_row(
            "SELECT count(*) FROM source_parse_evidence WHERE sha256=?1",
            [&sha],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(measured, 0);
}
#[test]
fn partial_shared_aggregate_is_not_every_members_counter_history() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("products.sqlite");
    let c = open_and_migrate(&path).unwrap();
    let a = xml("A", "first", 0);
    let b = xml("B", "second", 0);
    install_package(
        &c,
        "partial.knxprod",
        &package(MASTER, &[("M-0001/A.xml", &a), ("M-0001/B.xml", &b)]),
    )
    .unwrap();
    c.execute(
        "DELETE FROM ingest_unknown WHERE source_sha256=?1 AND name='Access'",
        [sha256_hex(a.as_bytes())],
    )
    .unwrap();
    c.execute_batch("UPDATE package_install_unknown SET occurrences=1,sample='second' WHERE name='Access'; UPDATE package SET unknown_count=unknown_count-1; UPDATE package_install_report SET unknown_occurrences=unknown_occurrences-1; UPDATE package_install_count SET count=count-1 WHERE category='unknown_construct' AND disposition='read'; PRAGMA user_version=27;").unwrap();
    let counter = rows(&c, "SELECT unknown_count FROM package");
    drop(c);
    let c = open_and_migrate(&path).unwrap();
    assert_eq!(
        rows(&c, "SELECT status FROM package_install_report"),
        vec![vec![Value::Text("unavailable".into())]],
        "partial positive aggregate cannot identify retained member units"
    );
    assert_eq!(rows(&c, "SELECT unknown_count FROM package"), counter);
}

#[test]
fn complete_shared_aggregate_remains_measured_without_recounting_history() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("products.sqlite");
    let c = open_and_migrate(&path).unwrap();
    let a = xml("A", "first", 0);
    let b = xml("B", "second", 0);
    let bytes = package(MASTER, &[("M-0001/A.xml", &a), ("M-0001/B.xml", &b)]);
    install_package(&c, "complete.knxprod", &bytes).unwrap();
    let history = stable(&c);
    let report = rows(&c, "SELECT * FROM package_install_unknown ORDER BY ordinal");
    c.pragma_update(None, "user_version", 27).unwrap();
    drop(c);
    let c = open_and_migrate(&path).unwrap();
    assert_eq!(
        rows(&c, "SELECT status FROM package_install_report"),
        vec![vec![Value::Text("measured".into())]]
    );
    assert_eq!(
        rows(&c, "SELECT * FROM package_install_unknown ORDER BY ordinal"),
        report
    );
    assert_eq!(stable(&c), history);
    install_package(&c, "complete.knxprod", &bytes).unwrap();
}

#[test]
fn two_admitted_sources_keep_separate_reports_and_cumulative_inventory() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("products.sqlite");
    let c = open_and_migrate(&path).unwrap();
    for (id, ch) in [("A", 'x'), ("B", 'y')] {
        let program = xml(id, &ch.to_string().repeat(3 * 1024 * 1024), 0);
        install_package(
            &c,
            "admitted.knxprod",
            &package(MASTER, &[("M-0001/A.xml", &program)]),
        )
        .unwrap();
    }
    let counters = rows(
        &c,
        "SELECT sha256,unknown_count FROM package ORDER BY sha256",
    );
    let report = rows(
        &c,
        "SELECT * FROM package_install_unknown ORDER BY package_sha256,ordinal",
    );
    c.pragma_update(None, "user_version", 27).unwrap();
    drop(c);
    let c = open_and_migrate(&path).unwrap();
    assert_eq!(
        rows(
            &c,
            "SELECT status FROM package_install_report ORDER BY package_sha256"
        ),
        vec![vec![Value::Text("measured".into())]; 2]
    );
    assert_eq!(
        rows(
            &c,
            "SELECT * FROM package_install_unknown ORDER BY package_sha256,ordinal"
        ),
        report
    );
    assert_eq!(
        rows(
            &c,
            "SELECT sha256,unknown_count FROM package ORDER BY sha256"
        ),
        counters
    );
    assert!(rows(
        &c,
        "SELECT * FROM ingest_unknown WHERE kind='PromotionRepairError'"
    )
    .is_empty());
}

#[test]
fn mixed_member_retirement_refuses_ambiguous_history_and_preserves_counter() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("products.sqlite");
    let c = open_and_migrate(&path).unwrap();
    let a = xml("A", "first", 0);
    let b = xml("B", "second", 0);
    install_package(
        &c,
        "mixed.knxprod",
        &package(MASTER, &[("M-0001/A.xml", &a), ("M-0001/B.xml", &b)]),
    )
    .unwrap();
    // Released retire_promoted_from_reports subtracts only the source keys
    // actually retired, but retire_report_rows deletes the entire merged key.
    // This is its persisted mixed-history image: A retired, B still counted.
    c.execute(
        "DELETE FROM ingest_unknown WHERE source_sha256=?1 AND name='Access'",
        [sha256_hex(a.as_bytes())],
    )
    .unwrap();
    c.execute_batch("DELETE FROM package_install_unknown WHERE name='Access'; UPDATE package SET unknown_count=unknown_count-1; UPDATE package_install_report SET unknown_distinct=(SELECT count(*) FROM package_install_unknown),unknown_occurrences=(SELECT coalesce(sum(occurrences),0) FROM package_install_unknown); UPDATE package_install_count SET count=(SELECT CASE disposition WHEN 'read' THEN coalesce(sum(occurrences),0) ELSE count(*) END FROM package_install_unknown) WHERE category='unknown_construct'; PRAGMA user_version=27;").unwrap();
    let counts = rows(&c, "SELECT unknown_count FROM package");
    drop(c);
    let c = open_and_migrate(&path).unwrap();
    assert_eq!(
        rows(&c, "SELECT status FROM package_install_report"),
        vec![vec![Value::Text("unavailable".into())]],
        "missing shared aggregate does not establish every member's retired unit"
    );
    assert_eq!(
        rows(&c, "SELECT unknown_count FROM package"),
        counts,
        "never fabricate package counter history"
    );
    let reason: String = c
        .query_row(
            "SELECT sample FROM ingest_unknown WHERE xpath='promotion_report_backfill'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert!(reason.contains("ambiguous"));
}
