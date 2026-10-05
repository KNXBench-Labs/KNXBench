//! KNOWN_LIMITATIONS §156 / ADR-0081: `Parameter` and `ParameterRef`
//! report every attribute they do not store, at install and by the
//! v20 -> v21 backfill, which must reproduce a fresh install.
//!
//! Synthetic fixture: attribute *names* follow the AR07 corpus census, every
//! value is invented.
use std::io::{Cursor, Write};

use knx_productdb::{ingest_file, install_package, open_and_migrate, sha256_hex};
use rusqlite::{params, Connection};
use zip::write::SimpleFileOptions;

const MASTER: &[u8] = br#"<KNX xmlns="http://knx.org/xml/project/14"><MasterData><Manufacturers><Manufacturer Id="M-0001" Name="Example"/></Manufacturers></MasterData></KNX>"#;
const HARDWARE: &[u8] = br#"<KNX xmlns="http://knx.org/xml/project/14"><ManufacturerData><Manufacturer RefId="M-0001"><Hardware><Hardware Id="H-1" Name="Example"/></Hardware></Manufacturer></ManufacturerData></KNX>"#;

/// A plain parameter, a union member with its own placement, a prefixed
/// foreign attribute, refs with and without extra attributes, and a
/// `ModuleDef`'s static parameter.
const PROGRAM: &[u8] = br#"<KNX xmlns="http://knx.org/xml/project/14" xmlns:x="urn:example:extension"><ManufacturerData><Manufacturer RefId="M-0001"><ApplicationPrograms><ApplicationProgram Id="M-0001_A-1" Name="Program">
<Static>
<ParameterTypes><ParameterType Id="M-0001_A-1_PT-1" Name="n"><TypeNumber SizeInBit="8" Type="unsignedInt" minInclusive="0" maxInclusive="99"/></ParameterType></ParameterTypes>
<Parameters>
<Parameter Id="M-0001_A-1_P-1" Name="delay" ParameterType="M-0001_A-1_PT-1" Text="Delay" Value="5" SuffixText="s" InitialValue="1" LegacyPatchAlways="true" x:Note="foreign"/>
<Parameter Id="M-0001_A-1_P-2" Name="plain" ParameterType="M-0001_A-1_PT-1" Text="Plain" Value="0" Access="ReadWrite" SuffixText="min"/>
<Union SizeInBit="8"><Memory CodeSegment="M-0001_A-1_RS-1" Offset="0" BitOffset="0"/>
<Parameter Id="M-0001_A-1_P-3" Name="member" ParameterType="M-0001_A-1_PT-1" Text="Member" Value="0" Offset="0" BitOffset="4" DefaultUnionParameter="true"/>
</Union>
</Parameters>
<ParameterRefs>
<ParameterRef Id="M-0001_A-1_P-1_R-1" RefId="M-0001_A-1_P-1" Name="delayRef" SuffixText="sec" InitialValue="2" Access="Read"/>
<ParameterRef Id="M-0001_A-1_P-2_R-1" RefId="M-0001_A-1_P-2" Tag="t" DisplayOrder="1"/>
<ParameterRef Id="M-0001_A-1_P-3_R-1" RefId="M-0001_A-1_P-3" ForbidGrantingUseByCustomer="true"/>
</ParameterRefs>
</Static>
<ModuleDefs><ModuleDef Id="M-0001_A-1_MD-1" Name="Module"><Static><Parameters>
<Parameter Id="M-0001_A-1_MD-1_P-1" Name="inner" ParameterType="M-0001_A-1_PT-1" Text="Inner" Value="0" InternalDescription="note"/>
</Parameters></Static></ModuleDef></ModuleDefs>
</ApplicationProgram></ApplicationPrograms></Manufacturer></ManufacturerData></KNX>"#;

const STATIC: &str =
    "/KNX/ManufacturerData/Manufacturer/ApplicationPrograms/ApplicationProgram/Static";

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
        ("M-0001/M-0001_A-1.xml", PROGRAM),
    ])
}

/// `(xpath, name, occurrences, sample)` of every attribute row under a
/// `Parameter`/`ParameterRef` xpath.
type Row = (String, String, i64, Option<String>);

/// The rows §156 is about, in one place: the expected install output.
fn expected() -> Vec<Row> {
    let parameter = format!("{STATIC}/Parameters/Parameter");
    let member = format!("{STATIC}/Parameters/Union/Parameter");
    let module = "/KNX/ManufacturerData/Manufacturer/ApplicationPrograms/ApplicationProgram/ModuleDefs/ModuleDef/Static/Parameters/Parameter".to_string();
    let reference = format!("{STATIC}/ParameterRefs/ParameterRef");
    let row =
        |x: &str, n: &str, o: i64, s: &str| (x.to_string(), n.to_string(), o, Some(s.to_string()));
    let mut rows = vec![
        row(&module, "InternalDescription", 1, "note"),
        row(&parameter, "InitialValue", 1, "1"),
        row(&parameter, "LegacyPatchAlways", 1, "true"),
        row(&parameter, "SuffixText", 2, "s"),
        row(&parameter, "{urn:example:extension}Note", 1, "foreign"),
        row(&member, "BitOffset", 1, "4"),
        row(&member, "DefaultUnionParameter", 1, "true"),
        row(&member, "Offset", 1, "0"),
        row(&reference, "ForbidGrantingUseByCustomer", 1, "true"),
        row(&reference, "InitialValue", 1, "2"),
        row(&reference, "Name", 1, "delayRef"),
        row(&reference, "SuffixText", 1, "sec"),
    ];
    rows.sort();
    rows
}

const PARAMETER_XPATHS: &str =
    "kind = 'Attribute' AND (xpath LIKE '%/Parameter' OR xpath LIKE '%/ParameterRef')";

fn blob_rows(conn: &Connection) -> Vec<Row> {
    conn.prepare(&format!(
        "SELECT xpath, name, occurrences, sample FROM ingest_unknown
         WHERE {PARAMETER_XPATHS} ORDER BY 1, 2"
    ))
    .unwrap()
    .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)))
    .unwrap()
    .collect::<Result<_, _>>()
    .unwrap()
}

#[derive(Debug, PartialEq)]
struct Snapshot {
    version: i64,
    unknowns: Vec<Vec<Option<String>>>,
    reports: Vec<Vec<Option<String>>>,
    counts: Vec<Vec<Option<String>>>,
    report_unknowns: Vec<Vec<Option<String>>>,
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
        unknowns: rows(
            conn,
            "SELECT source_sha256, program_id, xpath, kind, name, occurrences, sample
             FROM ingest_unknown ORDER BY 1, 3, 4, 5, 6, 7",
        ),
        reports: rows(conn, "SELECT * FROM package_install_report ORDER BY 1"),
        counts: rows(conn, "SELECT * FROM package_install_count ORDER BY 1, 2"),
        report_unknowns: rows(conn, "SELECT * FROM package_install_unknown ORDER BY 1, 2"),
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

/// Rewinds to a genuine v20 database: the rows v21 adds are removed from
/// `ingest_unknown` and from every measured report (header, counts and
/// `package.unknown_count` included). Rows under expanded names were
/// already written by scheme evidence before v21 and stay.
fn rewind_to_v20(conn: &Connection) {
    let unqualified = "instr(name, ':') = 0 AND substr(name, 1, 1) <> '{'";
    let packages: Vec<String> = conn
        .prepare("SELECT sha256 FROM package")
        .unwrap()
        .query_map([], |r| r.get(0))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    for package in packages {
        let lost: i64 = conn
            .query_row(
                &format!(
                    "SELECT count(*) FROM (SELECT DISTINCT source_sha256, xpath, name
                     FROM ingest_unknown WHERE source_sha256 IN
                       (SELECT source_sha256 FROM package_member WHERE package_sha256 = ?1
                        AND role = 'ApplicationProgram')
                       AND {PARAMETER_XPATHS} AND {unqualified})"
                ),
                [&package],
                |r| r.get(0),
            )
            .unwrap();
        conn.execute(
            "UPDATE package SET unknown_count = unknown_count - ?2 WHERE sha256 = ?1",
            params![package, lost],
        )
        .unwrap();
        let kept: Vec<(String, String, String, i64, Option<String>)> = conn
            .prepare(&format!(
                "SELECT xpath, kind, name, occurrences, sample FROM package_install_unknown
                 WHERE package_sha256 = ?1 AND NOT ({PARAMETER_XPATHS} AND {unqualified})
                 ORDER BY ordinal"
            ))
            .unwrap()
            .query_map([&package], |r| {
                Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?))
            })
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();
        conn.execute(
            "DELETE FROM package_install_unknown WHERE package_sha256 = ?1",
            [&package],
        )
        .unwrap();
        for (ordinal, (xpath, kind, name, occurrences, sample)) in kept.iter().enumerate() {
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
        let distinct = kept.len() as i64;
        let total: i64 = kept.iter().map(|r| r.3).sum();
        conn.execute(
            "UPDATE package_install_report SET unknown_distinct = ?2, unknown_occurrences = ?3
             WHERE package_sha256 = ?1 AND status = 'measured'",
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
    }
    conn.execute(
        &format!("DELETE FROM ingest_unknown WHERE {PARAMETER_XPATHS} AND {unqualified}"),
        [],
    )
    .unwrap();
    conn.execute_batch("PRAGMA user_version = 20;").unwrap();
}

#[test]
fn install_reports_every_parameter_attribute_it_does_not_store() {
    let (_dir, path, _fresh) = installed();
    let conn = Connection::open(&path).unwrap();
    assert_eq!(blob_rows(&conn), expected());
    // The package report names the same rows; the raw prefixed spelling
    // never appears next to its expanded name.
    let report: Vec<Row> = conn
        .prepare(&format!(
            "SELECT xpath, name, occurrences, sample FROM package_install_unknown
             WHERE {PARAMETER_XPATHS} ORDER BY 1, 2"
        ))
        .unwrap()
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    assert_eq!(report, expected());
    // Stored attributes are not reported (`ParameterRef/@Name` is not a
    // `parameter_ref` column, so it is).
    for (element, stored) in [
        (
            "/Parameter",
            &["Id", "Name", "Text", "ParameterType", "Access", "Value"][..],
        ),
        (
            "/ParameterRef",
            &["Id", "RefId", "Tag", "DisplayOrder", "Access"][..],
        ),
    ] {
        assert!(!blob_rows(&conn)
            .iter()
            .any(|(x, n, _, _)| x.ends_with(element) && stored.contains(&n.as_str())));
    }
}

#[test]
fn a_standalone_program_reports_them_too() {
    let dir = tempfile::tempdir().unwrap();
    let conn = open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
    ingest_file(&conn, "M-0001/M-0001_A-1.xml", PROGRAM).unwrap();
    assert_eq!(blob_rows(&conn), expected());
}

#[test]
fn the_rewind_is_a_genuine_v20_database() {
    let (_dir, path, fresh) = installed();
    let conn = Connection::open(&path).unwrap();
    rewind_to_v20(&conn);
    let v20 = snapshot(&conn);
    assert_eq!(v20.version, 20);
    // Only the expanded-name evidence row survives, as v20 wrote it.
    assert_eq!(blob_rows(&conn).len(), 1);
    assert_eq!(v20.unknowns.len(), fresh.unknowns.len() - 11);
    assert_eq!(v20.report_unknowns.len(), fresh.report_unknowns.len() - 11);
    let count = |s: &Snapshot| {
        s.unknown_count[0][1]
            .clone()
            .unwrap()
            .parse::<i64>()
            .unwrap()
    };
    assert_eq!(count(&v20), count(&fresh) - 11);
}

#[test]
fn v20_to_v21_reproduces_a_fresh_install() {
    let (_dir, path, fresh) = installed();
    {
        let conn = Connection::open(&path).unwrap();
        rewind_to_v20(&conn);
    }
    let conn = open_and_migrate(&path).unwrap();
    assert_eq!(snapshot(&conn), fresh);
    let retry = install_package(&conn, "sample.knxprod", &package()).unwrap();
    assert!(retry.skipped);
    assert!(retry.facts.is_some());
}

#[test]
fn v20_to_v21_records_a_shared_program_once_per_package_that_parsed_it() {
    // Two packages carry the same program bytes; each install re-parses it,
    // so a fresh database holds its rows twice. The backfill must too.
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("products.sqlite");
    let second = archive(&[
        ("knx_master.xml", MASTER),
        ("M-0001/Hardware.xml", HARDWARE),
        ("M-0001/M-0001_A-1.xml", PROGRAM),
        ("M-0001/readme.txt", b"second package"),
    ]);
    let fresh = {
        let conn = open_and_migrate(&path).unwrap();
        install_package(&conn, "first.knxprod", &package()).unwrap();
        install_package(&conn, "second.knxprod", &second).unwrap();
        let rows: i64 = conn
            .query_row(
                &format!("SELECT count(*) FROM ingest_unknown WHERE {PARAMETER_XPATHS}"),
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(rows, 2 * expected().len() as i64);
        let fresh = snapshot(&conn);
        rewind_to_v20(&conn);
        fresh
    };
    let conn = open_and_migrate(&path).unwrap();
    assert_eq!(snapshot(&conn), fresh);
}

#[test]
fn v20_to_v21_changes_nothing_a_current_install_already_reports() {
    let (_dir, path, fresh) = installed();
    {
        let conn = Connection::open(&path).unwrap();
        conn.execute_batch("PRAGMA user_version = 20;").unwrap();
    }
    let conn = open_and_migrate(&path).unwrap();
    assert_eq!(snapshot(&conn), fresh);
}

#[test]
fn v20_to_v21_names_a_damaged_blob_and_downgrades_its_report() {
    let (_dir, path, _fresh) = installed();
    let program_sha = sha256_hex(PROGRAM);
    {
        let conn = Connection::open(&path).unwrap();
        rewind_to_v20(&conn);
        let mut damaged = PROGRAM.to_vec();
        let at = damaged.len() - 10;
        damaged[at] ^= 0x20;
        conn.execute(
            "UPDATE source_file SET bytes = ?1 WHERE sha256 = ?2",
            params![damaged, program_sha],
        )
        .unwrap();
    }
    let conn = open_and_migrate(&path).unwrap();
    let failures: i64 = conn
        .query_row(
            "SELECT count(*) FROM ingest_unknown
             WHERE source_sha256 = ?1 AND kind = 'ParameterAttributeBackfillError'",
            [&program_sha],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(failures, 1);
    assert_eq!(
        blob_rows(&conn).len(),
        1,
        "nothing invented for the damaged blob"
    );
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
fn the_current_schema_is_v21() {
    assert_eq!(knx_productdb::CURRENT_PRODUCTDB_VERSION, 21);
}
