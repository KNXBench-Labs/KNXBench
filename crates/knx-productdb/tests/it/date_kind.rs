//! v28 -> v29 (WP-08): stored `TypeDate` types become kind `Date` exactly as a fresh install.
//!
//! Synthetic fixture: element and attribute names follow the two collection
//! programs that declare `TypeDate`; every id and value is invented.

use std::io::{Cursor, Write};

use knx_productdb::{install_package, open_and_migrate, sha256_hex};
use rusqlite::{params, Connection};
use zip::write::SimpleFileOptions;

mod modes;

const MASTER: &[u8] = br#"<KNX xmlns="http://knx.org/xml/project/14"><MasterData><Manufacturers><Manufacturer Id="M-0001" Name="Example"/></Manufacturers></MasterData></KNX>"#;

/// Two `TypeDate` types (one with a second, invented attribute) and one
/// number type, each used by a parameter.
const PROGRAM: &[u8] = br#"<KNX xmlns="http://knx.org/xml/project/14"><ManufacturerData><Manufacturer RefId="M-0001"><ApplicationPrograms><ApplicationProgram Id="M-0001_A-1" Name="Program">
<Static>
<ParameterTypes>
<ParameterType Id="M-0001_A-1_PT-D1" Name="d1"><TypeDate Encoding="DPT 11"/></ParameterType>
<ParameterType Id="M-0001_A-1_PT-D2" Name="d2"><TypeDate Encoding="DPT 11" UIHint="Calendar"/></ParameterType>
<ParameterType Id="M-0001_A-1_PT-N" Name="n"><TypeNumber SizeInBit="8" Type="unsignedInt" minInclusive="0" maxInclusive="99"/></ParameterType>
</ParameterTypes>
<Parameters>
<Parameter Id="M-0001_A-1_P-1" Name="first" ParameterType="M-0001_A-1_PT-D1" Text="First" Value="2022-09-15"/>
<Parameter Id="M-0001_A-1_P-2" Name="second" ParameterType="M-0001_A-1_PT-D2" Text="Second" Value="1964-06-17"/>
<Parameter Id="M-0001_A-1_P-3" Name="count" ParameterType="M-0001_A-1_PT-N" Text="Count" Value="5"/>
</Parameters>
<ParameterRefs>
<ParameterRef Id="M-0001_A-1_P-1_R-1" RefId="M-0001_A-1_P-1" DisplayOrder="1"/>
<ParameterRef Id="M-0001_A-1_P-2_R-1" RefId="M-0001_A-1_P-2" DisplayOrder="2"/>
<ParameterRef Id="M-0001_A-1_P-3_R-1" RefId="M-0001_A-1_P-3" DisplayOrder="3"/>
</ParameterRefs>
</Static>
</ApplicationProgram></ApplicationPrograms></Manufacturer></ManufacturerData></KNX>"#;

const TYPE_PATH: &str = "/KNX/ManufacturerData/Manufacturer/ApplicationPrograms/ApplicationProgram/Static/ParameterTypes/ParameterType";

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

/// A package around `PROGRAM`; `hardware` tells two packages apart while
/// they share the one program blob.
fn package(hardware: &str) -> Vec<u8> {
    package_with(MASTER, hardware)
}

fn package_with(master: &[u8], hardware: &str) -> Vec<u8> {
    let hardware = format!(
        r#"<KNX xmlns="http://knx.org/xml/project/14"><ManufacturerData><Manufacturer RefId="M-0001"><Hardware><Hardware Id="H-1" Name="{hardware}"/></Hardware></Manufacturer></ManufacturerData></KNX>"#
    );
    archive(&[
        ("knx_master.xml", master),
        ("M-0001/Hardware.xml", hardware.as_bytes()),
        ("M-0001/M-0001_A-1.xml", PROGRAM),
    ])
}

fn installed(packages: &[&str]) -> (tempfile::TempDir, std::path::PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("products.sqlite");
    let conn = open_and_migrate(&path).unwrap();
    for hardware in packages {
        install_package(&conn, "sample.knxprod", &package(hardware)).unwrap();
    }
    (dir, path)
}

type Row = (String, String, i64, Option<String>);

/// `(xpath, kind, name, occurrences, sample)`-shaped rows of one table that
/// sit at the type path or below it.
fn type_rows(conn: &Connection, table: &str) -> Vec<Row> {
    conn.prepare(&format!(
        "SELECT xpath, name, occurrences, sample FROM {table}
         WHERE xpath LIKE ?1 || '%' AND kind IN ('Element', 'Attribute') ORDER BY 1, 2, 3, 4"
    ))
    .unwrap()
    .query_map([TYPE_PATH], |r| {
        Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?))
    })
    .unwrap()
    .collect::<Result<_, _>>()
    .unwrap()
}

fn kinds(conn: &Connection) -> Vec<(String, String)> {
    conn.prepare("SELECT id, kind FROM parameter_type ORDER BY id")
        .unwrap()
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap()
}

/// Rewinds a v29 install to what a v28 parser left: `kind = 'Other'`, one
/// `TypeDate` element row (no sample, every occurrence counted) instead of
/// the child's attribute rows, in `ingest_unknown`, every measured report
/// and `package.unknown_count` alike. v29 adds no DDL.
fn rewind_to_v28(conn: &Connection) {
    let blob = sha256_hex(PROGRAM);
    let child_path = format!("{TYPE_PATH}/TypeDate");
    let dates: i64 = conn
        .query_row(
            "SELECT count(*) FROM parameter_type WHERE kind = 'Date'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    let attribute_rows = conn
        .execute(
            "DELETE FROM ingest_unknown WHERE source_sha256 = ?1 AND xpath = ?2",
            params![blob, child_path],
        )
        .unwrap() as i64;
    conn.execute(
        "INSERT INTO ingest_unknown (source_sha256, program_id, xpath, kind, name, occurrences, sample)
         VALUES (?1, NULL, ?2, 'Element', 'TypeDate', ?3, NULL)",
        params![blob, TYPE_PATH, dates],
    )
    .unwrap();
    let packages: Vec<String> = conn
        .prepare("SELECT sha256 FROM package ORDER BY 1")
        .unwrap()
        .query_map([], |r| r.get(0))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    for package in packages {
        let mut rows: Vec<(String, String, String, i64, Option<String>)> = conn
            .prepare(
                "SELECT xpath, kind, name, occurrences, sample FROM package_install_unknown
                 WHERE package_sha256 = ?1 AND xpath != ?2 ORDER BY ordinal",
            )
            .unwrap()
            .query_map(params![package, child_path], |r| {
                Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?))
            })
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();
        rows.push((
            TYPE_PATH.into(),
            "Element".into(),
            "TypeDate".into(),
            dates,
            None,
        ));
        rows.sort_by(|a, b| (&a.0, &a.1, &a.2).cmp(&(&b.0, &b.1, &b.2)));
        conn.execute(
            "DELETE FROM package_install_unknown WHERE package_sha256 = ?1",
            [&package],
        )
        .unwrap();
        for (ordinal, (xpath, kind, name, occurrences, sample)) in rows.iter().enumerate() {
            conn.execute(
                "INSERT INTO package_install_unknown VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
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
        let distinct = rows.len() as i64;
        let total: i64 = rows.iter().map(|r| r.3).sum();
        conn.execute(
            "UPDATE package_install_report SET unknown_distinct = ?2, unknown_occurrences = ?3
             WHERE package_sha256 = ?1",
            params![package, distinct, total],
        )
        .unwrap();
        for (disposition, count) in [("read", total), ("stored", distinct)] {
            conn.execute(
                "UPDATE package_install_count SET count = ?3
                 WHERE package_sha256 = ?1 AND category = 'unknown_construct' AND disposition = ?2",
                params![package, disposition, count],
            )
            .unwrap();
        }
        conn.execute(
            "UPDATE package SET unknown_count = unknown_count - ?2 + 1 WHERE sha256 = ?1",
            params![package, attribute_rows],
        )
        .unwrap();
    }
    conn.execute_batch(
        "UPDATE parameter_type SET kind = 'Other' WHERE kind = 'Date';
         PRAGMA user_version = 28;",
    )
    .unwrap();
}

type Table = Vec<Vec<Option<String>>>;

fn table(conn: &Connection, sql: &str) -> Table {
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

/// Everything v29 touches, as a fresh install leaves it.
fn snapshot(conn: &Connection) -> Vec<Table> {
    vec![
        table(conn, "PRAGMA user_version"),
        table(conn, "SELECT * FROM parameter_type ORDER BY program_id, id"),
        table(
            conn,
            "SELECT source_sha256, program_id, xpath, kind, name, occurrences, sample
             FROM ingest_unknown ORDER BY 1, 3, 4, 5, 6, 7",
        ),
        table(conn, "SELECT * FROM package_install_report ORDER BY 1"),
        table(conn, "SELECT * FROM package_install_count ORDER BY 1, 2"),
        table(conn, "SELECT * FROM package_install_unknown ORDER BY 1, 2"),
        table(conn, "SELECT sha256, unknown_count FROM package ORDER BY 1"),
    ]
}

#[test]
fn a_fresh_install_stores_date_and_reports_its_encoding() {
    let (_dir, path) = installed(&["Example"]);
    let conn = Connection::open(&path).unwrap();
    assert_eq!(
        kinds(&conn),
        vec![
            ("M-0001_A-1_PT-D1".into(), "Date".into()),
            ("M-0001_A-1_PT-D2".into(), "Date".into()),
            ("M-0001_A-1_PT-N".into(), "Number".into()),
        ]
    );
    let child = format!("{TYPE_PATH}/TypeDate");
    let expected = vec![
        (child.clone(), "Encoding".into(), 2, Some("DPT 11".into())),
        (child, "UIHint".into(), 1, Some("Calendar".into())),
    ];
    assert_eq!(type_rows(&conn, "ingest_unknown"), expected);
    assert_eq!(type_rows(&conn, "package_install_unknown"), expected);
    let views = knx_productdb::query::parameter_views(&conn, "M-0001_A-1", None).unwrap();
    let first = views.iter().find(|v| v.id == "M-0001_A-1_P-1_R-1").unwrap();
    assert_eq!(first.kind, "Date");
}

#[test]
fn the_v28_rewind_is_genuine() {
    let (_dir, path) = installed(&["Example"]);
    let conn = Connection::open(&path).unwrap();
    rewind_to_v28(&conn);
    assert!(kinds(&conn)
        .iter()
        .all(|(id, kind)| id.ends_with("PT-N") || kind == "Other"));
    let expected = vec![(TYPE_PATH.into(), "TypeDate".into(), 2, None)];
    assert_eq!(type_rows(&conn, "ingest_unknown"), expected);
    assert_eq!(type_rows(&conn, "package_install_unknown"), expected);
}

#[test]
fn v28_to_v29_reproduces_a_fresh_install() {
    let (_dir, path) = installed(&["Example"]);
    let fresh = snapshot(&Connection::open(&path).unwrap());
    rewind_to_v28(&Connection::open(&path).unwrap());
    let conn = open_and_migrate(&path).unwrap();
    assert_eq!(snapshot(&conn), fresh);
    let retry = install_package(&conn, "sample.knxprod", &package("Example")).unwrap();
    assert!(retry.skipped);
}

/// A BOM master whose declarations collide or are malformed quotes byte ranges in the
/// install report. The v29 backfill reloads that report with the fixed parser, so the
/// pre-v31 ranges must move first; otherwise the report would be downgraded.
#[test]
fn v28_to_v29_keeps_the_report_of_a_bom_master_whose_ranges_move() {
    bom_master_report_survives_from(28);
}

/// v27 additionally runs the v27 -> v28 promotion repair, which reloads the report first.
#[test]
fn v27_to_v28_keeps_the_report_of_a_bom_master_whose_ranges_move() {
    bom_master_report_survives_from(27);
}

fn bom_master_report_survives_from(version: i64) {
    let master = [
        b"\xef\xbb\xbf".as_slice(),
        br#"<KNX xmlns="http://knx.org/xml/project/14"><MasterData><Manufacturers><Manufacturer Id="M-0001" Name="Example"/></Manufacturers><DatapointType Id="D" Number="7"/><DatapointType Id="D" Number="8"/><SpaceUsage/></MasterData></KNX>"#,
    ]
    .concat();
    let archive = package_with(&master, "Example");
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("products.sqlite");
    let diagnostics = |conn: &Connection| -> Vec<(String, String)> {
        conn.prepare("SELECT kind, detail FROM package_install_diagnostic ORDER BY ordinal")
            .unwrap()
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap()
    };
    let (fresh, fresh_diagnostics, facts) = {
        let conn = open_and_migrate(&path).unwrap();
        let facts = install_package(&conn, "sample.knxprod", &archive)
            .unwrap()
            .facts
            .unwrap();
        let fresh_diagnostics = diagnostics(&conn);
        assert_eq!(
            fresh_diagnostics
                .iter()
                .filter(|(_, detail)| detail.contains(" bytes ["))
                .count(),
            2,
            "one collision and one malformed declaration quote ranges"
        );
        let fresh = snapshot(&conn);
        super::master_provenance::rewind_bom_ranges(&conn, &sha256_hex(&master));
        assert_ne!(diagnostics(&conn), fresh_diagnostics);
        rewind_to_v28(&conn);
        conn.pragma_update(None, "user_version", version).unwrap();
        (fresh, fresh_diagnostics, facts)
    };
    let conn = open_and_migrate(&path).unwrap();
    let status: String = conn
        .query_row("SELECT status FROM package_install_report", [], |r| {
            r.get(0)
        })
        .unwrap();
    assert_eq!(status, "measured");
    assert_eq!(diagnostics(&conn), fresh_diagnostics);
    assert_eq!(snapshot(&conn), fresh);
    let retry = install_package(&conn, "sample.knxprod", &archive).unwrap();
    assert!(retry.skipped);
    assert_eq!(retry.facts, Some(facts));
}

/// One program blob in two packages: the blob is re-derived once, and each
/// package's counter and report follow it.
#[test]
fn v28_to_v29_follows_a_blob_shared_by_two_packages() {
    let (_dir, path) = installed(&["Example", "Other example"]);
    let fresh = snapshot(&Connection::open(&path).unwrap());
    assert_eq!(fresh[6].len(), 2, "two packages");
    rewind_to_v28(&Connection::open(&path).unwrap());
    let conn = open_and_migrate(&path).unwrap();
    assert_eq!(snapshot(&conn), fresh);
}

#[test]
fn v28_to_v29_names_a_damaged_blob_keeps_its_rows_and_downgrades_its_report() {
    let (_dir, path) = installed(&["Example"]);
    let blob = sha256_hex(PROGRAM);
    {
        let conn = Connection::open(&path).unwrap();
        rewind_to_v28(&conn);
        let mut damaged = PROGRAM.to_vec();
        let at = damaged.len() - 10;
        damaged[at] ^= 0x20;
        conn.execute(
            "UPDATE source_file SET bytes = ?1 WHERE sha256 = ?2",
            params![damaged, blob],
        )
        .unwrap();
    }
    let conn = open_and_migrate(&path).unwrap();
    let failures: Vec<(String, String)> = conn
        .prepare(
            "SELECT kind, name FROM ingest_unknown
             WHERE kind LIKE '%BackfillError' ORDER BY 1",
        )
        .unwrap()
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    assert_eq!(
        failures,
        vec![
            (
                "InstallReportBackfillError".into(),
                "date_kind_report_backfill".into()
            ),
            (
                "ParameterKindBackfillError".into(),
                "backfill_date_kind".into()
            ),
        ]
    );
    // Nothing re-kinded, nothing retired, nothing invented.
    assert!(kinds(&conn)
        .iter()
        .all(|(id, kind)| id.ends_with("PT-N") || kind == "Other"));
    assert_eq!(
        type_rows(&conn, "ingest_unknown"),
        vec![(TYPE_PATH.into(), "TypeDate".into(), 2, None)]
    );
    let status: String = conn
        .query_row("SELECT status FROM package_install_report", [], |r| {
            r.get(0)
        })
        .unwrap();
    assert_eq!(status, "unavailable");
}

/// `PT-D1` with a second, non-deciding `TypeDate` (schema-invalid, but
/// possible): its attributes were reported by v28 too, so the surviving
/// `Encoding` row changes its sample (first occurrence in document order).
fn edge_program() -> Vec<u8> {
    String::from_utf8(PROGRAM.to_vec())
        .unwrap()
        .replace(
            r#"<TypeDate Encoding="DPT 11"/></ParameterType>"#,
            r#"<TypeDate Encoding="DPT 11"/><TypeDate Encoding="X"/></ParameterType>"#,
        )
        .into_bytes()
}

fn edge_package() -> Vec<u8> {
    let hardware = br#"<KNX xmlns="http://knx.org/xml/project/14"><ManufacturerData><Manufacturer RefId="M-0001"><Hardware><Hardware Id="H-1" Name="Example"/></Hardware></Manufacturer></ManufacturerData></KNX>"#;
    archive(&[
        ("knx_master.xml", MASTER),
        ("M-0001/Hardware.xml", hardware),
        ("M-0001/M-0001_A-1.xml", &edge_program()),
    ])
}

/// What a v28 parser wrote for `edge_program`, checked once against an
/// actual v28 install: the element row counts all three `TypeDate`s, only
/// the non-deciding one's `Encoding` is reported, and with its own sample.
fn rewind_edge_to_v28(conn: &Connection) {
    let child_path = format!("{TYPE_PATH}/TypeDate");
    for table in ["ingest_unknown", "package_install_unknown"] {
        conn.execute(
            &format!("UPDATE {table} SET occurrences = 3 WHERE xpath = ?1 AND name = 'TypeDate'"),
            [TYPE_PATH],
        )
        .unwrap();
        conn.execute(
            &format!("DELETE FROM {table} WHERE xpath = ?1 AND name = 'UIHint'"),
            [&child_path],
        )
        .unwrap();
        conn.execute(
            &format!(
                "UPDATE {table} SET occurrences = 1, sample = 'X' WHERE xpath = ?1 AND name = 'Encoding'"
            ),
            [&child_path],
        )
        .unwrap();
    }
    let package: String = conn
        .query_row("SELECT sha256 FROM package", [], |r| r.get(0))
        .unwrap();
    let rows: Vec<i64> = conn
        .prepare(
            "SELECT occurrences FROM package_install_unknown WHERE package_sha256 = ?1 ORDER BY 1",
        )
        .unwrap()
        .query_map([&package], |r| r.get(0))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    // Ordinals stay dense: renumber in report order.
    conn.execute_batch(
        "CREATE TEMP TABLE renumbered AS SELECT * FROM package_install_unknown ORDER BY xpath, kind, name;
         DELETE FROM package_install_unknown;
         INSERT INTO package_install_unknown
             SELECT package_sha256, row_number() OVER () - 1, xpath, kind, name, occurrences, sample
             FROM renumbered;
         DROP TABLE renumbered;",
    )
    .unwrap();
    let (distinct, total) = (rows.len() as i64, rows.iter().sum::<i64>());
    conn.execute(
        "UPDATE package_install_report SET unknown_distinct = ?2, unknown_occurrences = ?3
         WHERE package_sha256 = ?1",
        params![package, distinct, total],
    )
    .unwrap();
    for (disposition, count) in [("read", total), ("stored", distinct)] {
        conn.execute(
            "UPDATE package_install_count SET count = ?3
             WHERE package_sha256 = ?1 AND category = 'unknown_construct' AND disposition = ?2",
            params![package, disposition, count],
        )
        .unwrap();
    }
    conn.execute(
        "UPDATE package SET unknown_count = unknown_count - 1 WHERE sha256 = ?1",
        [&package],
    )
    .unwrap();
    conn.execute_batch(
        "UPDATE parameter_type SET kind = 'Other' WHERE kind = 'Date';
         PRAGMA user_version = 28;",
    )
    .unwrap();
}

/// The blob's own rows become exactly a fresh install's; the merged report
/// cannot tell whose sample it should keep, so it is downgraded and named.
#[test]
fn v28_to_v29_downgrades_a_report_whose_surviving_row_changes_its_sample() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("products.sqlite");
    install_package(
        &open_and_migrate(&path).unwrap(),
        "sample.knxprod",
        &edge_package(),
    )
    .unwrap();
    let fresh = snapshot(&Connection::open(&path).unwrap());
    rewind_edge_to_v28(&Connection::open(&path).unwrap());
    let conn = open_and_migrate(&path).unwrap();
    let migrated = snapshot(&conn);
    assert_eq!(migrated[..2], fresh[..2], "version and kinds");
    let without_errors: Vec<_> = migrated[2]
        .iter()
        .filter(|row| row[3].as_deref() != Some("InstallReportBackfillError"))
        .cloned()
        .collect();
    assert_eq!(without_errors, fresh[2], "the blob's own rows");
    assert_eq!(
        migrated[6], fresh[6],
        "package.unknown_count follows the blob"
    );
    let status: String = conn
        .query_row("SELECT status FROM package_install_report", [], |r| {
            r.get(0)
        })
        .unwrap();
    assert_eq!(status, "unavailable");
    let cause: String = conn
        .query_row(
            "SELECT sample FROM ingest_unknown WHERE kind = 'InstallReportBackfillError'
             AND name = 'date_kind_report_backfill'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert!(cause.contains("sample changed"), "{cause}");
}

/// A later duplicate program is evidence, not authority over the first
/// program's parameter types (ADR-0011).
#[test]
fn v28_to_v29_keeps_the_first_program_declaration() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("products.sqlite");
    let raw = std::str::from_utf8(PROGRAM).unwrap();
    let start = raw.find("<ApplicationProgram ").unwrap();
    let end = raw.find("</ApplicationProgram>").unwrap() + "</ApplicationProgram>".len();
    let first = raw[start..end].replace("<TypeDate", "<TypeHologram");
    let xml = format!(
        "{}{}{}{}",
        &raw[..start],
        first,
        &raw[start..end],
        &raw[end..]
    );
    let conn = open_and_migrate(&path).unwrap();
    knx_productdb::ingest_file(&conn, "M-0001/duplicate.xml", xml.as_bytes()).unwrap();
    let fresh = kinds(&conn);
    let fresh_unknown = type_rows(&conn, "ingest_unknown");
    assert_eq!(fresh[0].1, "Other");
    assert_eq!(fresh[1].1, "Other");
    conn.execute(
        "DELETE FROM ingest_unknown WHERE xpath = ?1",
        [format!("{TYPE_PATH}/TypeDate")],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO ingest_unknown (source_sha256, xpath, kind, name, occurrences)
         VALUES (?1, ?2, 'Element', 'TypeDate', 2)",
        params![sha256_hex(xml.as_bytes()), TYPE_PATH],
    )
    .unwrap();
    conn.execute_batch("PRAGMA user_version = 28;").unwrap();
    drop(conn);
    let conn = open_and_migrate(&path).unwrap();
    assert_eq!(
        kinds(&conn),
        fresh,
        "a losing program cannot promote the winner's Other types"
    );
    assert_eq!(
        type_rows(&conn, "ingest_unknown"),
        fresh_unknown,
        "losing declarations still contribute unknown evidence"
    );
}

/// A damaged historical report must be named and made unavailable rather
/// than silently made plausible by recomputing its counters.
#[test]
fn v28_to_v29_downgrades_an_inconsistent_measured_report() {
    let (_dir, path) = installed(&["Example"]);
    let conn = Connection::open(&path).unwrap();
    rewind_to_v28(&conn);
    conn.execute_batch(
        "UPDATE package_install_report SET unknown_occurrences = unknown_occurrences + 100;",
    )
    .unwrap();
    drop(conn);
    let conn = open_and_migrate(&path).unwrap();
    let status: String = conn
        .query_row("SELECT status FROM package_install_report", [], |r| {
            r.get(0)
        })
        .unwrap();
    assert_eq!(
        status, "unavailable",
        "corrupt measured history is not repaired"
    );
    let failures: i64 = conn.query_row("SELECT count(*) FROM ingest_unknown WHERE kind = 'InstallReportBackfillError' AND name = 'date_kind_report_backfill'", [], |r| r.get(0)).unwrap();
    assert_eq!(failures, 1);
}

/// A missing historical count is not evidence that the count was zero.
#[test]
fn v28_to_v29_downgrades_a_report_with_a_missing_count_row() {
    assert_corrupt_count_downgraded(
        "DELETE FROM package_install_count WHERE category = 'parameter' AND disposition = 'stored'",
    );
}

#[test]
fn v28_to_v29_downgrades_a_report_with_a_wrong_unknown_count() {
    assert_corrupt_count_downgraded(
        "UPDATE package_install_count SET count = count + 7 WHERE category = 'unknown_construct' AND disposition = 'read'",
    );
}

fn assert_corrupt_count_downgraded(sql: &str) {
    let (_dir, path) = installed(&["Example"]);
    let conn = Connection::open(&path).unwrap();
    rewind_to_v28(&conn);
    conn.execute_batch(sql).unwrap();
    drop(conn);
    let conn = open_and_migrate(&path).unwrap();
    let status: String = conn
        .query_row("SELECT status FROM package_install_report", [], |r| {
            r.get(0)
        })
        .unwrap();
    assert_eq!(
        status, "unavailable",
        "invalid historical count must not become measured zero"
    );
    let failures: i64 = conn.query_row("SELECT count(*) FROM ingest_unknown WHERE kind = 'InstallReportBackfillError' AND name = 'date_kind_report_backfill'", [], |r| r.get(0)).unwrap();
    assert_eq!(failures, 1);
}

/// An earlier member gains an attribute key that a later member already
/// contributed. A merged historical sample cannot be assigned to its member.
#[test]
fn v28_to_v29_downgrades_a_report_with_an_ambiguous_cross_member_sample() {
    let a = r#"<KNX xmlns="http://knx.org/xml/project/14"><ManufacturerData><Manufacturer RefId="M-0001"><ApplicationPrograms><ApplicationProgram Id="M-0001_A-1" Name="First"><Static><ParameterTypes><ParameterType Id="M-0001_A-1_PT-1"><TypeDate Encoding="DPT 11"/></ParameterType></ParameterTypes></Static></ApplicationProgram></ApplicationPrograms></Manufacturer></ManufacturerData></KNX>"#.to_string();
    let b = a.replace("M-0001_A-1", "M-0001_A-2").replace(
        r#"<TypeDate Encoding="DPT 11"/>"#,
        r#"<TypeNumber SizeInBit="8" Type="unsignedInt"/><TypeDate Encoding="X"/>"#,
    );
    let hw = br#"<KNX xmlns="http://knx.org/xml/project/14"><ManufacturerData><Manufacturer RefId="M-0001"><Hardware><Hardware Id="H-1" Name="Example"/></Hardware></Manufacturer></ManufacturerData></KNX>"#;
    let pkg = archive(&[
        ("knx_master.xml", MASTER),
        ("M-0001/Hardware.xml", hw),
        ("M-0001/M-0001_A-1.xml", a.as_bytes()),
        ("M-0001/M-0001_A-2.xml", b.as_bytes()),
    ]);
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("products.sqlite");
    let conn = open_and_migrate(&path).unwrap();
    install_package(&conn, "two-members.knxprod", &pkg).unwrap();
    let fresh_kinds = kinds(&conn);
    let fresh_unknown = type_rows(&conn, "ingest_unknown");
    let before_count: i64 = conn
        .query_row("SELECT unknown_count FROM package", [], |r| r.get(0))
        .unwrap();
    // Genuine v28 output: A reports an element, B an element plus its
    // non-deciding attribute. The merged Encoding row has B's sample only.
    conn.execute(
        "DELETE FROM ingest_unknown WHERE source_sha256 = ?1 AND xpath = ?2",
        params![sha256_hex(a.as_bytes()), format!("{TYPE_PATH}/TypeDate")],
    )
    .unwrap();
    conn.execute("INSERT INTO ingest_unknown (source_sha256, xpath, kind, name, occurrences) VALUES (?1, ?2, 'Element', 'TypeDate', 1)", params![sha256_hex(a.as_bytes()), TYPE_PATH]).unwrap();
    conn.execute(
        "UPDATE parameter_type SET kind = 'Other' WHERE kind = 'Date'",
        [],
    )
    .unwrap();
    conn.execute("UPDATE package_install_unknown SET occurrences = 2 WHERE xpath = ?1 AND kind = 'Element' AND name = 'TypeDate'", [TYPE_PATH]).unwrap();
    conn.execute("UPDATE package_install_unknown SET occurrences = 1, sample = 'X' WHERE xpath = ?1 AND name = 'Encoding'", [format!("{TYPE_PATH}/TypeDate")]).unwrap();
    // The two changes cancel in both distinct and occurrence counts.
    conn.execute_batch("PRAGMA user_version = 28;").unwrap();
    drop(conn);
    let conn = open_and_migrate(&path).unwrap();
    assert_eq!(kinds(&conn), fresh_kinds);
    assert_eq!(type_rows(&conn, "ingest_unknown"), fresh_unknown);
    let after_count: i64 = conn
        .query_row("SELECT unknown_count FROM package", [], |r| r.get(0))
        .unwrap();
    assert_eq!(after_count, before_count);
    let status: String = conn
        .query_row("SELECT status FROM package_install_report", [], |r| {
            r.get(0)
        })
        .unwrap();
    assert_eq!(
        status, "unavailable",
        "do not silently keep the later member's sample"
    );
}

/// A database without a `TypeDate` element row (here: one a v29 parser
/// already wrote) gains nothing but its version.
#[test]
fn v28_to_v29_leaves_a_database_without_date_element_rows_alone() {
    let (_dir, path) = installed(&["Example"]);
    Connection::open(&path)
        .unwrap()
        .execute_batch("PRAGMA user_version = 28;")
        .unwrap();
    let before = snapshot(&Connection::open(&path).unwrap());
    let conn = open_and_migrate(&path).unwrap();
    let after = snapshot(&conn);
    assert_eq!(
        after[0],
        vec![vec![Some(
            knx_productdb::CURRENT_PRODUCTDB_VERSION.to_string()
        )]]
    );
    assert_eq!(after[1..], before[1..]);
}
