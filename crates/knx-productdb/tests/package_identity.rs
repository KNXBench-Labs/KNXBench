//! Package identity, its parser agreement, backfill and version queries (ADR-0043).
//
// Covers source names, per-candidate element digests, the agreement check,
// producer facts, the v16 -> v17 backfill and the family, `ReplacesVersions`
// and order-number queries.

mod v17_rewind;
mod v20_rewind;

use std::collections::BTreeMap;
use std::io::{Cursor, Write};

use knx_productdb::{
    identity_candidates, identity_divergences, ingest_file, install_package, open_and_migrate,
    package_source_names, products_by_order_number, program_family, sha256_hex, FamilyKey,
    IdentityKind, ReplacesVersions,
};
use rusqlite::Connection;
use zip::write::SimpleFileOptions;

const NS11: &str = "http://knx.org/xml/project/11";
const NS21: &str = "http://knx.org/xml/project/21";

fn db() -> (tempfile::TempDir, Connection) {
    let dir = tempfile::tempdir().unwrap();
    let conn = open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
    (dir, conn)
}

fn master(ns: &str) -> String {
    format!(
        r#"<KNX xmlns="{ns}"><MasterData><Manufacturers><Manufacturer Id="M-0001" Name="Example"/></Manufacturers></MasterData></KNX>"#
    )
}

fn manufacturer_file(ns: &str, root_attrs: &str, body: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="utf-8"?><KNX xmlns="{ns}"{root_attrs}><ManufacturerData><Manufacturer RefId="M-0001">{body}</Manufacturer></ManufacturerData></KNX>"#
    )
}

fn program_file(root_attrs: &str, programs: &str) -> String {
    manufacturer_file(
        NS11,
        root_attrs,
        &format!("<ApplicationPrograms>{programs}</ApplicationPrograms>"),
    )
}

fn hardware_file(ns: &str, hardware: &str) -> String {
    manufacturer_file(ns, "", &format!("<Hardware>{hardware}</Hardware>"))
}

fn program(id: &str, dynamic: &str) -> String {
    format!(
        r#"<ApplicationProgram Id="{id}" Name="P" ApplicationNumber="1" ApplicationVersion="1" MaskVersion="MV-0701"><Static/><Dynamic>{dynamic}</Dynamic></ApplicationProgram>"#
    )
}

fn archive(members: &[(&str, &[u8])]) -> Vec<u8> {
    let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for (name, bytes) in members {
        zip.start_file(*name, SimpleFileOptions::default()).unwrap();
        zip.write_all(bytes).unwrap();
    }
    zip.finish().unwrap().into_inner()
}

fn package(ns: &str, members: &[(&str, &str)]) -> Vec<u8> {
    let master = master(ns);
    let mut all: Vec<(&str, &[u8])> = vec![("knx_master.xml", master.as_bytes())];
    all.extend(members.iter().map(|(path, xml)| (*path, xml.as_bytes())));
    archive(&all)
}

/// Row count of every table in the database, by name.
fn table_counts(conn: &Connection) -> BTreeMap<String, i64> {
    let tables: Vec<String> = conn
        .prepare("SELECT name FROM sqlite_master WHERE type = 'table' ORDER BY name")
        .unwrap()
        .query_map([], |r| r.get(0))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    tables
        .into_iter()
        .map(|table| {
            let count = conn
                .query_row(&format!("SELECT count(*) FROM \"{table}\""), [], |r| {
                    r.get(0)
                })
                .unwrap();
            (table, count)
        })
        .collect()
}

type IdentityRow = (String, String, String, i64, String);

fn identity_rows(conn: &Connection) -> Vec<IdentityRow> {
    conn.prepare(
        "SELECT source_sha256, table_name, logical_id, occurrence, digest FROM source_identity
         ORDER BY source_sha256, table_name, logical_id, occurrence",
    )
    .unwrap()
    .query_map([], |r| {
        Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?))
    })
    .unwrap()
    .collect::<Result<_, _>>()
    .unwrap()
}

fn rows<T: rusqlite::types::FromSql>(conn: &Connection, sql: &str) -> Vec<Vec<Option<T>>> {
    let mut statement = conn.prepare(sql).unwrap();
    let width = statement.column_count();
    statement
        .query_map([], |r| (0..width).map(|i| r.get(i)).collect())
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap()
}

fn winner(conn: &Connection, table: &str, id: &str) -> String {
    conn.query_row(
        &format!("SELECT source_sha256 FROM {table} WHERE id = ?1"),
        [id],
        |r| r.get(0),
    )
    .unwrap()
}

// ---------------------------------------------------------------------------
// 1. Source names
// ---------------------------------------------------------------------------

#[test]
fn a_byte_identical_package_under_a_second_name_records_only_the_name() {
    let (_dir, conn) = db();
    let bytes = package(
        NS11,
        &[(
            "M-0001/Hardware.xml",
            &hardware_file(
                NS11,
                r#"<Hardware Id="H-1"><Products><Product Id="P-1"/></Products></Hardware>"#,
            ),
        )],
    );
    let first = install_package(&conn, "b.knxprod", &bytes).unwrap();
    assert!(!first.skipped);
    assert_eq!(first.source_names, ["b.knxprod"]);
    let before = table_counts(&conn);

    let retry = install_package(&conn, "a-copy.knxprod", &bytes).unwrap();
    assert!(retry.skipped);
    assert_eq!(retry.source_names, ["a-copy.knxprod", "b.knxprod"]);
    assert_eq!(
        package_source_names(&conn, &first.sha256).unwrap(),
        ["a-copy.knxprod", "b.knxprod"]
    );
    let mut after = table_counts(&conn);
    assert_eq!(after.remove("package_source_name"), Some(2));
    let mut expected = before;
    assert_eq!(expected.remove("package_source_name"), Some(1));
    assert_eq!(after, expected, "the retry wrote nothing but the name");
    // `package.source_name` still holds the first name.
    let stored: String = conn
        .query_row("SELECT source_name FROM package", [], |r| r.get(0))
        .unwrap();
    assert_eq!(stored, "b.knxprod");

    // The same name again is not a second row.
    let again = install_package(&conn, "b.knxprod", &bytes).unwrap();
    assert_eq!(again.source_names, ["a-copy.knxprod", "b.knxprod"]);
    assert_eq!(table_counts(&conn)["package_source_name"], 2);
}

// ---------------------------------------------------------------------------
// 2. + 7. Candidates, winners and install order
// ---------------------------------------------------------------------------

struct Fixture {
    first: Vec<u8>,
    second: Vec<u8>,
    third: Vec<u8>,
}

/// `first` and `second` carry `A-1` differing only inside `Dynamic`;
/// `third` carries the same `A-1` element as `first` in a blob whose bytes
/// differ outside the element (a root attribute).
fn fixture() -> Fixture {
    let a = program_file(
        "",
        &program("M-0001_A-0001-01-0000", "<ChannelIndependentBlock/>"),
    );
    let b = program_file(
        "",
        &program(
            "M-0001_A-0001-01-0000",
            r#"<ChannelIndependentBlock><ParameterBlock Id="M-0001_A-0001-01-0000_PB-1" Name="B"/></ChannelIndependentBlock>"#,
        ),
    );
    let c = program_file(
        r#" CreatedBy="Tool" ToolVersion="1.0""#,
        &program("M-0001_A-0001-01-0000", "<ChannelIndependentBlock/>"),
    );
    Fixture {
        first: package(NS11, &[("M-0001/M-0001_A-0001-01-0000.xml", &a)]),
        second: package(NS11, &[("M-0001/M-0001_A-0001-01-0000.xml", &b)]),
        third: package(NS11, &[("M-0001/M-0001_A-0001-01-0000.xml", &c)]),
    }
}

const PROGRAM: &str = "M-0001_A-0001-01-0000";

#[test]
fn every_candidate_is_named_with_its_packages_and_whether_it_equals_the_winner() {
    let (_dir, conn) = db();
    let f = fixture();
    let first = install_package(&conn, "first.knxprod", &f.first).unwrap();
    let second = install_package(&conn, "second.knxprod", &f.second).unwrap();
    let third = install_package(&conn, "third.knxprod", &f.third).unwrap();
    let blob = |report: &knx_productdb::InstallReport| {
        report
            .members
            .iter()
            .find(|m| m.role == "ApplicationProgram")
            .unwrap()
            .sha256
            .clone()
    };
    let (a, b, c) = (blob(&first), blob(&second), blob(&third));

    let report = identity_candidates(&conn, IdentityKind::ApplicationProgram, PROGRAM).unwrap();
    assert_eq!(report.winner.as_deref(), Some(a.as_str()));
    assert!(report.unmeasured.is_empty());
    let mut expected = vec![
        (a.clone(), Some(true), vec![first.sha256.clone()]),
        (b.clone(), Some(false), vec![second.sha256.clone()]),
        (c.clone(), Some(true), vec![third.sha256.clone()]),
    ];
    expected.sort();
    let got: Vec<_> = report
        .candidates
        .iter()
        .map(|c| {
            (
                c.source_sha256.clone(),
                c.same_as_winner,
                c.packages.clone(),
            )
        })
        .collect();
    assert_eq!(got, expected);
    assert!(report
        .candidates
        .iter()
        .all(|c| c.occurrence == 1 && c.source_path == "M-0001/M-0001_A-0001-01-0000.xml"));
    let digest = |sha: &str| {
        report
            .candidates
            .iter()
            .find(|c| c.source_sha256 == sha)
            .unwrap()
            .digest
            .clone()
    };
    assert_ne!(
        digest(&a),
        digest(&b),
        "Dynamic content is part of the element"
    );
    assert_eq!(
        digest(&a),
        digest(&c),
        "bytes outside the element do not count"
    );

    let divergences = identity_divergences(&conn).unwrap();
    assert_eq!(divergences.len(), 1, "{divergences:?}");
    assert_eq!(divergences[0].kind, IdentityKind::ApplicationProgram);
    assert_eq!(divergences[0].logical_id, PROGRAM);
    assert_eq!(divergences[0].candidates, 3);
    assert_eq!(divergences[0].distinct_digests, 2);
}

#[test]
fn the_recorded_candidates_do_not_depend_on_install_order_but_the_winner_does() {
    let f = fixture();
    let (_d1, forward) = db();
    let (_d2, backward) = db();
    install_package(&forward, "first.knxprod", &f.first).unwrap();
    install_package(&forward, "second.knxprod", &f.second).unwrap();
    install_package(&backward, "second.knxprod", &f.second).unwrap();
    install_package(&backward, "first.knxprod", &f.first).unwrap();

    assert_eq!(identity_rows(&forward), identity_rows(&backward));
    assert_ne!(
        winner(&forward, "application_program", PROGRAM),
        winner(&backward, "application_program", PROGRAM)
    );
    for conn in [&forward, &backward] {
        let report = identity_candidates(conn, IdentityKind::ApplicationProgram, PROGRAM).unwrap();
        let packages: Vec<_> = report
            .candidates
            .iter()
            .flat_map(|c| c.packages.clone())
            .collect();
        assert_eq!(packages.len(), 2);
        let same: Vec<_> = report.candidates.iter().map(|c| c.same_as_winner).collect();
        assert!(
            same.contains(&Some(true)) && same.contains(&Some(false)),
            "{same:?}"
        );
    }
}

// ---------------------------------------------------------------------------
// 5. Mirror edge cases against the real parsers
// ---------------------------------------------------------------------------

fn candidates_of(conn: &Connection, sha: &str) -> Vec<(String, String, i64)> {
    conn.prepare(
        "SELECT table_name, logical_id, occurrence FROM source_identity WHERE source_sha256 = ?1
         ORDER BY table_name, logical_id, occurrence",
    )
    .unwrap()
    .query_map([sha], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
    .unwrap()
    .collect::<Result<_, _>>()
    .unwrap()
}

fn owned(rows: &[(&str, &str, i64)]) -> Vec<(String, String, i64)> {
    rows.iter()
        .map(|(t, id, o)| (t.to_string(), id.to_string(), *o))
        .collect()
}

#[test]
fn programs_the_parser_never_dispatches_are_not_candidates_and_the_check_agrees() {
    let (_dir, conn) = db();
    let xml = program_file(
        "",
        r#"<ApplicationProgram Id="M-0001_A-0001-01-0000" Name="P" ApplicationNumber="1" ApplicationVersion="1"><Static><ParameterTypes><ParameterType Id="M-0001_A-0001-01-0000_PT-1" Name="T"><ApplicationProgram Id="M-0001_A-0009-01-0000"/></ParameterType></ParameterTypes></Static><Dynamic><ApplicationProgram Id="M-0001_A-0008-01-0000"/></Dynamic></ApplicationProgram>"#,
    );
    ingest_file(&conn, "M-0001/M-0001_A-0001-01-0000.xml", xml.as_bytes()).unwrap();
    assert_eq!(
        candidates_of(&conn, &sha256_hex(xml.as_bytes())),
        owned(&[("application_program", "M-0001_A-0001-01-0000", 1)])
    );
    let programs: i64 = conn
        .query_row("SELECT count(*) FROM application_program", [], |r| r.get(0))
        .unwrap();
    assert_eq!(programs, 1);
}

#[test]
fn a_duplicate_id_in_one_file_is_two_occurrences_and_its_conflict_names_the_second() {
    let (_dir, conn) = db();
    let xml = hardware_file(
        NS11,
        r#"<Hardware Id="H-1"><Products><Product Id="P-1" Text="a"/><Product Id="P-1" Text="b"/></Products><Hardware2Programs><Hardware2Program MediumTypes="MT-0"/></Hardware2Programs></Hardware><Hardware><Products/></Hardware>"#,
    );
    let outcome = ingest_file(&conn, "M-0001/Hardware.xml", xml.as_bytes()).unwrap();
    let knx_productdb::IngestOutcome::Ingested { conflicts, .. } = outcome else {
        panic!("not ingested");
    };
    assert_eq!(conflicts.len(), 1);
    assert_eq!(
        (
            conflicts[0].table.as_str(),
            conflicts[0].id.as_str(),
            conflicts[0].occurrence
        ),
        ("product", "P-1", 2)
    );
    let sha = sha256_hex(xml.as_bytes());
    assert_eq!(
        candidates_of(&conn, &sha),
        owned(&[
            ("hardware", "H-1", 1),
            ("hardware2program", "", 1),
            ("product", "P-1", 1),
            ("product", "P-1", 2),
        ]),
        "the Id-less outer Hardware is no candidate; the Id-less Hardware2Program is"
    );
    let report = identity_candidates(&conn, IdentityKind::Product, "P-1").unwrap();
    let same: Vec<_> = report
        .candidates
        .iter()
        .map(|c| (c.occurrence, c.same_as_winner))
        .collect();
    assert_eq!(same, [(1, Some(true)), (2, Some(false))]);
}

#[test]
fn a_blob_the_scan_cannot_read_is_recorded_unavailable_and_still_ingests() {
    let (_dir, conn) = db();
    // The hardware parser ignores text; the scan must resolve every reference.
    let xml = hardware_file(NS11, r#"<Hardware Id="H-1"><Note>&nbsp;</Note></Hardware>"#);
    ingest_file(&conn, "M-0001/Hardware.xml", xml.as_bytes()).unwrap();
    let sha = sha256_hex(xml.as_bytes());
    let (status, reason): (String, Option<String>) = conn
        .query_row(
            "SELECT status, reason FROM source_identity_scan WHERE source_sha256 = ?1",
            [&sha],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap();
    assert_eq!(status, "unavailable");
    assert!(reason.unwrap().contains("nbsp"));
    assert!(candidates_of(&conn, &sha).is_empty());
    let report = identity_candidates(&conn, IdentityKind::Hardware, "H-1").unwrap();
    assert_eq!(report.winner.as_deref(), Some(sha.as_str()));
    assert!(report.candidates.is_empty());
    assert_eq!(report.unmeasured.len(), 1);
    assert_eq!(report.unmeasured[0].source_sha256, sha);
}

// ---------------------------------------------------------------------------
// 6. The agreement check bites and rolls back
// ---------------------------------------------------------------------------

#[test]
fn a_recorded_candidate_set_that_misses_a_parsed_row_fails_the_ingest_and_rolls_back() {
    let (_dir, conn) = db();
    let xml = hardware_file(
        NS11,
        r#"<Hardware Id="H-1"><Products><Product Id="P-1"/></Products></Hardware>"#,
    );
    let sha = sha256_hex(xml.as_bytes());
    // A stored, measured blob whose recorded candidates lack `P-1`: what a
    // scan that stopped mirroring the hardware parser would have written.
    conn.execute(
        "INSERT INTO source_file (sha256, source_path, manufacturer_id, len, bytes) VALUES (?1, 'M-0001/Hardware.xml', 'M-0001', ?2, ?3)",
        rusqlite::params![sha, xml.len() as i64, xml.as_bytes()],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO source_identity_scan (source_sha256, status, scanner) VALUES (?1, 'measured', 1)",
        [&sha],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO source_identity VALUES (?1, 'hardware', 'H-1', 1, ?2)",
        rusqlite::params![sha, "0".repeat(64)],
    )
    .unwrap();
    let before = table_counts(&conn);

    let error = ingest_file(&conn, "M-0001/Hardware.xml", xml.as_bytes()).unwrap_err();
    assert!(
        error
            .to_string()
            .contains("identity scan disagrees with parser: product row \"P-1\""),
        "{error}"
    );
    assert_eq!(
        table_counts(&conn),
        before,
        "the failed ingest left nothing behind"
    );
}

#[test]
fn a_first_parse_whose_scan_adds_a_candidate_fails_the_ingest() {
    // The exact rule of a first parse, wired into the ingest: a recorded
    // candidate whose id another blob holds passes rules a-c, but no row or
    // conflict of this parse produced it. Simulated by a stray candidate
    // row planted before the blob's first ingest (foreign keys off, as no
    // valid database holds such a row).
    let (_dir, conn) = db();
    let other = hardware_file(NS11, r#"<Hardware Id="H-OTHER"/>"#);
    ingest_file(&conn, "M-0001/Other.xml", other.as_bytes()).unwrap();
    let xml = hardware_file(NS11, r#"<Hardware Id="H-1"/>"#);
    let sha = sha256_hex(xml.as_bytes());
    conn.pragma_update(None, "foreign_keys", "OFF").unwrap();
    conn.execute(
        "INSERT INTO source_identity VALUES (?1, 'hardware', 'H-OTHER', 1, ?2)",
        rusqlite::params![sha, "0".repeat(64)],
    )
    .unwrap();
    let before = table_counts(&conn);
    let error = ingest_file(&conn, "M-0001/Hardware.xml", xml.as_bytes()).unwrap_err();
    assert!(
        error.to_string().contains(
            "candidate hardware \"H-OTHER\" occurrence 1 matches no row or conflict of this parse"
        ),
        "{error}"
    );
    assert_eq!(
        table_counts(&conn),
        before,
        "the failed ingest left nothing behind"
    );
}

// ---------------------------------------------------------------------------
// 8. v16 -> v17
// ---------------------------------------------------------------------------

fn identity_tables(conn: &Connection) -> Vec<Vec<Vec<Option<String>>>> {
    vec![
        rows(conn, "SELECT package_sha256, source_name FROM package_source_name ORDER BY 1, 2"),
        rows(
            conn,
            "SELECT source_sha256, status, reason, CAST(scanner AS TEXT) FROM source_identity_scan ORDER BY 1",
        ),
        rows(
            conn,
            "SELECT source_sha256, table_name, logical_id, CAST(occurrence AS TEXT), digest FROM source_identity ORDER BY 1, 2, 3, 4",
        ),
        rows(
            conn,
            "SELECT source_sha256, root_namespace, created_by, tool_version FROM source_producer ORDER BY 1",
        ),
    ]
}

#[test]
fn v16_to_v17_backfills_exactly_what_a_fresh_install_records() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("products.sqlite");
    let f = fixture();
    let fresh = {
        let conn = open_and_migrate(&path).unwrap();
        install_package(&conn, "first.knxprod", &f.first).unwrap();
        install_package(&conn, "second.knxprod", &f.second).unwrap();
        install_package(&conn, "third.knxprod", &f.third).unwrap();
        let loose = hardware_file(
            NS11,
            r#"<Hardware Id="H-1"><Products><Product Id="P-1"/></Products></Hardware>"#,
        );
        ingest_file(&conn, "M-0001/Hardware.xml", loose.as_bytes()).unwrap();
        let tables = identity_tables(&conn);
        assert!(tables.iter().all(|t| !t.is_empty()), "{tables:?}");
        v17_rewind::rewind_to_v16(&conn);
        tables
    };
    let conn = open_and_migrate(&path).unwrap();
    let version: i64 = conn
        .query_row("PRAGMA user_version", [], |r| r.get(0))
        .unwrap();
    assert_eq!(version, knx_productdb::CURRENT_PRODUCTDB_VERSION);
    assert_eq!(identity_tables(&conn), fresh);
}

#[test]
fn v16_to_v17_records_a_blob_whose_bytes_no_longer_match_as_unavailable() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("products.sqlite");
    let f = fixture();
    let damaged = {
        let conn = open_and_migrate(&path).unwrap();
        let report = install_package(&conn, "first.knxprod", &f.first).unwrap();
        let sha = report
            .members
            .iter()
            .find(|m| m.role == "ApplicationProgram")
            .unwrap()
            .sha256
            .clone();
        v17_rewind::rewind_to_v16(&conn);
        conn.execute(
            "UPDATE source_file SET bytes = X'3c4b4e582f3e' WHERE sha256 = ?1",
            [&sha],
        )
        .unwrap();
        sha
    };
    let conn = open_and_migrate(&path).unwrap();
    let (status, reason): (String, String) = conn
        .query_row(
            "SELECT status, reason FROM source_identity_scan WHERE source_sha256 = ?1",
            [&damaged],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap();
    assert_eq!(status, "unavailable");
    assert!(reason.contains("do not match"), "{reason}");
    assert!(candidates_of(&conn, &damaged).is_empty());
    let producer: i64 = conn
        .query_row(
            "SELECT count(*) FROM source_producer WHERE source_sha256 = ?1",
            [&damaged],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(producer, 0, "damaged bytes yield no producer facts");
    let report = identity_candidates(&conn, IdentityKind::ApplicationProgram, PROGRAM).unwrap();
    assert_eq!(report.winner.as_deref(), Some(damaged.as_str()));
    assert_eq!(report.unmeasured.len(), 1);
    // The master blob is intact and still gets its facts.
    let masters: i64 = conn
        .query_row("SELECT count(*) FROM source_producer", [], |r| r.get(0))
        .unwrap();
    assert_eq!(masters, 1);
}

#[test]
fn v16_to_v17_records_historical_rows_that_disagree_with_the_scan_as_unavailable() {
    // Review M-2: rows an earlier build wrote are checked during the
    // backfill; a disagreement degrades that blob to `unavailable` instead
    // of making every later re-parse of it fail.
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("products.sqlite");
    let xml = hardware_file(
        NS11,
        r#"<Hardware Id="H-1"><Products><Product Id="P-1"/></Products></Hardware>"#,
    );
    let sha = sha256_hex(xml.as_bytes());
    {
        let conn = open_and_migrate(&path).unwrap();
        ingest_file(&conn, "M-0001/Hardware.xml", xml.as_bytes()).unwrap();
        v17_rewind::rewind_to_v16(&conn);
        // A row the scan will not find, as an older parser might have kept.
        conn.execute(
            "INSERT INTO product (id, hardware_id, manufacturer_id, source_sha256) VALUES ('P-OLD', 'H-1', 'M-0001', ?1)",
            [&sha],
        )
        .unwrap();
    }
    let conn = open_and_migrate(&path).unwrap();
    let (status, reason): (String, String) = conn
        .query_row(
            "SELECT status, reason FROM source_identity_scan WHERE source_sha256 = ?1",
            [&sha],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap();
    assert_eq!(status, "unavailable");
    assert!(
        reason.contains("v17 backfill: stored rows disagree with the scan")
            && reason.contains("product row \"P-OLD\""),
        "{reason}"
    );
    assert!(candidates_of(&conn, &sha).is_empty());
    // Re-ingesting the blob does not trip over the historical row.
    ingest_file(&conn, "M-0001/Hardware.xml", xml.as_bytes()).unwrap();
    let report = identity_candidates(&conn, IdentityKind::Product, "P-OLD").unwrap();
    assert_eq!(report.unmeasured.len(), 1);
    assert_eq!(report.unmeasured[0].source_sha256, sha);
}

#[test]
fn v17_indexes_the_identity_tables_by_source() {
    let (_dir, conn) = db();
    for table in [
        "catalog_section",
        "catalog_item",
        "hardware",
        "product",
        "hardware2program",
        "application_program",
    ] {
        let plan: String = conn
            .query_row(
                &format!("EXPLAIN QUERY PLAN SELECT id FROM {table} WHERE source_sha256 = 'x'"),
                [],
                |r| r.get(3),
            )
            .unwrap();
        assert!(
            plan.contains(&format!("INDEX {table}_source")),
            "{table}: {plan}"
        );
    }
}

#[test]
fn a_measured_winner_without_a_first_candidate_is_named_as_unmeasured() {
    // Review M-6: otherwise every candidate reads `same_as_winner: None`
    // with no explanation.
    let (_dir, conn) = db();
    let xml = hardware_file(NS11, r#"<Hardware Id="H-1"/>"#);
    let sha = sha256_hex(xml.as_bytes());
    ingest_file(&conn, "M-0001/Hardware.xml", xml.as_bytes()).unwrap();
    conn.execute(
        "DELETE FROM source_identity WHERE source_sha256 = ?1",
        [&sha],
    )
    .unwrap();
    let report = identity_candidates(&conn, IdentityKind::Hardware, "H-1").unwrap();
    assert_eq!(report.winner.as_deref(), Some(sha.as_str()));
    assert!(report.candidates.is_empty());
    assert_eq!(report.unmeasured.len(), 1);
    assert!(
        report.unmeasured[0]
            .reason
            .contains("records no first occurrence"),
        "{:?}",
        report.unmeasured
    );
}

// ---------------------------------------------------------------------------
// 9. Families, ReplacesVersions, order numbers
// ---------------------------------------------------------------------------

fn family_program(id: &str, number: Option<&str>, version: &str, replaces: Option<&str>) -> String {
    let number = number
        .map(|n| format!(r#" ApplicationNumber="{n}""#))
        .unwrap_or_default();
    let replaces = replaces
        .map(|r| format!(r#" ReplacesVersions="{r}""#))
        .unwrap_or_default();
    format!(
        r#"<ApplicationProgram Id="{id}" Name="P"{number} ApplicationVersion="{version}"{replaces}/>"#
    )
}

#[test]
fn a_family_groups_winning_programs_by_manufacturer_and_parsed_number() {
    let (_dir, conn) = db();
    let files = [
        family_program("M-0001_A-0001-01-0000", Some("1"), "1", None),
        family_program("M-0001_A-0001-02-0000", Some(" +0001 "), "2", Some("1 2 7")),
        family_program("M-0001_A-0001-03-0000-O0001", Some("1"), "03", Some("")),
        family_program("M-0001_A-0002-01-0000", Some("2"), "1", Some("1, 2")),
        family_program("M-0001_A-0003-01-0000", Some("0x3"), "1", None),
        family_program("M-0001_A-0004-01-0000", Some("65536"), "1", None),
    ];
    for (i, body) in files.iter().enumerate() {
        let xml = program_file("", body);
        ingest_file(&conn, &format!("M-0001/P{i}.xml"), xml.as_bytes()).unwrap();
    }
    // Another manufacturer's program with the same number is no member.
    let other = program_file(
        "",
        &family_program("M-0002_A-0001-01-0000", Some("1"), "1", None),
    )
    .replace(r#"RefId="M-0001""#, r#"RefId="M-0002""#);
    ingest_file(&conn, "M-0002/P.xml", other.as_bytes()).unwrap();

    let family = program_family(&conn, "M-0001_A-0001-02-0000")
        .unwrap()
        .unwrap();
    assert_eq!(family.key, FamilyKey::Number(1));
    assert_eq!(family.manufacturer_id, "M-0001");
    let ids: Vec<_> = family
        .members
        .iter()
        .map(|m| m.program_id.as_str())
        .collect();
    assert_eq!(
        ids,
        [
            "M-0001_A-0001-01-0000",
            "M-0001_A-0001-02-0000",
            "M-0001_A-0001-03-0000-O0001",
        ]
    );
    assert_eq!(family.members[2].application_version.as_deref(), Some("03"));
    assert_eq!(family.members[2].parsed_version, Some(3));
    assert_eq!(family.members[0].replaces, ReplacesVersions::Absent);
    assert_eq!(
        family.members[1].replaces,
        ReplacesVersions::Parsed {
            raw: "1 2 7".into(),
            entries: vec![
                (1, vec!["M-0001_A-0001-01-0000".to_string()]),
                (2, vec!["M-0001_A-0001-02-0000".to_string()]),
                (7, vec![]),
            ],
        }
    );
    assert_eq!(
        family.members[2].replaces,
        ReplacesVersions::Parsed {
            raw: String::new(),
            entries: vec![],
        },
        "an empty list is a list, not an absent attribute"
    );

    let unparsed_list = program_family(&conn, "M-0001_A-0002-01-0000")
        .unwrap()
        .unwrap();
    assert_eq!(unparsed_list.key, FamilyKey::Number(2));
    assert!(matches!(
        &unparsed_list.members[0].replaces,
        ReplacesVersions::Unparsed { raw, .. } if raw == "1, 2"
    ));

    for (id, raw) in [
        ("M-0001_A-0003-01-0000", "0x3"),
        ("M-0001_A-0004-01-0000", "65536"),
    ] {
        let family = program_family(&conn, id).unwrap().unwrap();
        assert!(
            matches!(&family.key, FamilyKey::Unparsed { raw: Some(r), .. } if r == raw),
            "{:?}",
            family.key
        );
        let ids: Vec<_> = family
            .members
            .iter()
            .map(|m| m.program_id.as_str())
            .collect();
        assert_eq!(ids, [id], "an unparsed key has no family beyond itself");
    }
    assert_eq!(
        program_family(&conn, "M-0001_A-0099-01-0000").unwrap(),
        None
    );
}

#[test]
fn an_order_number_lists_every_winning_product_with_its_programs_and_schemes() {
    let (_dir, conn) = db();
    let hw = |ns: &str, hardware: &str, product: &str, program: &str| {
        hardware_file(
            ns,
            &format!(
                r#"<Hardware Id="{hardware}"><Products><Product Id="{product}" Text="T {product}" OrderNumber="ON-1"/></Products><Hardware2Programs><Hardware2Program Id="{hardware}_HP-1"><ApplicationProgramRef RefId="{program}"/></Hardware2Program></Hardware2Programs></Hardware>"#
            ),
        )
    };
    let eleven = hw(
        NS11,
        "M-0001_H-1",
        "M-0001_H-1_P-1",
        "M-0001_A-0001-01-0000",
    );
    let twenty_one = hw(
        NS21,
        "M-0001_H-2",
        "M-0001_H-2_P-1",
        "M-0001_A-0002-01-0000",
    );
    install_package(
        &conn,
        "11.knxprod",
        &package(NS11, &[("M-0001/Hardware.xml", &eleven)]),
    )
    .unwrap();
    install_package(
        &conn,
        "21.knxprod",
        &package(NS21, &[("M-0001/Hardware.xml", &twenty_one)]),
    )
    .unwrap();
    let other = hw(
        NS11,
        "M-0001_H-3",
        "M-0001_H-3_P-1",
        "M-0001_A-0003-01-0000",
    )
    .replace("ON-1", "ON-2");
    ingest_file(&conn, "M-0001/Other.xml", other.as_bytes()).unwrap();

    let found = products_by_order_number(&conn, "M-0001", "ON-1").unwrap();
    let got: Vec<_> = found
        .iter()
        .map(|p| {
            (
                p.product_id.as_str(),
                p.text.as_deref(),
                p.hardware_id.as_str(),
                p.programs.clone(),
                p.schemes.clone(),
            )
        })
        .collect();
    assert_eq!(
        got,
        [
            (
                "M-0001_H-1_P-1",
                Some("T M-0001_H-1_P-1"),
                "M-0001_H-1",
                vec!["M-0001_A-0001-01-0000".to_string()],
                vec![11],
            ),
            (
                "M-0001_H-2_P-1",
                Some("T M-0001_H-2_P-1"),
                "M-0001_H-2",
                vec!["M-0001_A-0002-01-0000".to_string()],
                vec![21],
            ),
        ]
    );
    assert_eq!(found[0].source_sha256, sha256_hex(eleven.as_bytes()));
    assert!(products_by_order_number(&conn, "M-0001", "on-1")
        .unwrap()
        .is_empty());
    assert!(products_by_order_number(&conn, "M-0002", "ON-1")
        .unwrap()
        .is_empty());
    let loose = products_by_order_number(&conn, "M-0001", "ON-2").unwrap();
    assert_eq!(loose.len(), 1);
    assert!(
        loose[0].schemes.is_empty(),
        "a loose ingest is in no package"
    );
}

// ---------------------------------------------------------------------------
// 10. Producer facts
// ---------------------------------------------------------------------------

#[test]
fn producer_facts_are_recorded_per_blob_from_the_unprefixed_root_attributes() {
    let (_dir, conn) = db();
    // Scheme 21 refuses qualified attributes in a package, so the
    // prefixed lookalikes ride in a scheme-11 package.
    let with_facts = manufacturer_file(
        NS11,
        r#" xmlns:x="urn:x" CreatedBy="Tool &amp; Co" ToolVersion="5.7.1" x:CreatedBy="lookalike""#,
        "<Hardware/>",
    );
    let lookalike_only =
        manufacturer_file(NS11, r#" xmlns:x="urn:x" x:ToolVersion="9""#, "<Catalog/>");
    let bytes = package(
        NS11,
        &[
            ("M-0001/Hardware.xml", &with_facts),
            ("M-0001/Catalog.xml", &lookalike_only),
            ("M-0001/Baggages/logo.bin", "\u{1}\u{2}not xml"),
        ],
    );
    install_package(&conn, "p.knxprod", &bytes).unwrap();
    let facts = |xml: &str| -> Option<(Option<String>, Option<String>, Option<String>)> {
        conn.query_row(
            "SELECT root_namespace, created_by, tool_version FROM source_producer WHERE source_sha256 = ?1",
            [sha256_hex(xml.as_bytes())],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .ok()
    };
    assert_eq!(
        facts(&with_facts),
        Some((
            Some(NS11.to_string()),
            Some("Tool & Co".to_string()),
            Some("5.7.1".to_string())
        ))
    );
    assert_eq!(
        facts(&lookalike_only),
        Some((Some(NS11.to_string()), None, None))
    );
    assert_eq!(
        facts(&master(NS11)),
        Some((Some(NS11.to_string()), None, None))
    );
    assert_eq!(facts("\u{1}\u{2}not xml"), None);
    let total: i64 = conn
        .query_row("SELECT count(*) FROM source_producer", [], |r| r.get(0))
        .unwrap();
    assert_eq!(total, 3);

    // A loose ingest records the facts too, with the namespace as written.
    let loose = manufacturer_file(NS21, r#" ToolVersion="6.0""#, "<Hardware/>");
    ingest_file(&conn, "M-0001/Loose.xml", loose.as_bytes()).unwrap();
    assert_eq!(
        facts(&loose),
        Some((Some(NS21.to_string()), None, Some("6.0".to_string())))
    );
    // A root that is not `KNX` has no facts.
    let foreign = r#"<Other CreatedBy="x"/>"#;
    ingest_file(&conn, "M-0001/Foreign.xml", foreign.as_bytes()).unwrap();
    assert_eq!(facts(foreign), None);
}

// ---------------------------------------------------------------------------
// 11. Persisted-row corruption fails closed
// ---------------------------------------------------------------------------

#[test]
fn a_malformed_persisted_identity_row_is_a_query_error() {
    let f = fixture();
    for corruption in [
        "UPDATE source_identity SET digest = upper(digest)",
        "UPDATE source_identity SET table_name = 'Hardware'",
        "UPDATE source_identity SET occurrence = 0",
    ] {
        let (_dir, conn) = db();
        install_package(&conn, "first.knxprod", &f.first).unwrap();
        conn.execute_batch(&format!(
            "PRAGMA ignore_check_constraints = ON; {corruption}; PRAGMA ignore_check_constraints = OFF;"
        ))
        .unwrap();
        assert!(identity_divergences(&conn).is_err(), "{corruption}");
        if !corruption.contains("table_name") {
            assert!(
                identity_candidates(&conn, IdentityKind::ApplicationProgram, PROGRAM).is_err(),
                "{corruption}"
            );
        }
    }
}
