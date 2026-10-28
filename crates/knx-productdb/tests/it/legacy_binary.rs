//! Legacy long-enum `BINARY_VALUE`s reach `binary_value`, fresh and through the v30 backfill.
//!
//! Synthetic EX-IM text only. Behaviour is asserted through the public API and SQL.

use std::collections::BTreeMap;

use knx_productdb::code::load_program_code;
use knx_productdb::legacy::{publish_legacy, read_legacy_member, LegacyPublishError};
use knx_productdb::{
    open_and_migrate, open_read_only, sha256_hex, Connection, CURRENT_PRODUCTDB_VERSION,
};
use rusqlite::types::Value;

const SEP: &str = "-------------------------------------\r\n";
const BACKFILL_NOTE: &str = "found by the v30 backfill";

type Row = [&'static str; 7];
type EnumRow = (String, String, String, i64, Option<String>);
type OldDiagnostics<'a> = &'a [(&'a str, usize, usize)];
type StoredEnum = (String, String, String, Option<String>, Option<String>, i64);

fn table(id: u32, name: &str, columns: &[&str], rows: &[Vec<String>]) -> String {
    let mut out = format!("{SEP}T {id} {name}\r\n");
    for (i, column) in columns.iter().enumerate() {
        out.push_str(&format!("C{} T{id} 3 255 Y {column}\r\n", i + 1));
    }
    for (i, row) in rows.iter().enumerate() {
        out.push_str(&format!("R {} T {id} {name}\r\n", i + 1));
        for value in row {
            out.push_str(value);
            out.push_str("\r\n");
        }
    }
    out
}

fn strings(row: &[&str]) -> Vec<String> {
    row.iter().map(|s| s.to_string()).collect()
}

/// `types`: (type id, atomic type, size). `list`: (type, real, shown, order, value id, hex, length).
fn exim(
    program: &str,
    types: &[(&str, &str, &str)],
    list: &[Row],
    binary_columns: bool,
) -> Vec<u8> {
    let mut tables = table(
        3,
        "manufacturer",
        &["MANUFACTURER_ID"],
        &[strings(&["4242"])],
    );
    tables += &table(
        11,
        "application_program",
        &["PROGRAM_ID", "MANUFACTURER_ID"],
        &[strings(&[program, "4242"])],
    );
    let type_rows: Vec<Vec<String>> = types
        .iter()
        .map(|(id, atomic, size)| strings(&[id, atomic, program, &format!("T{id}"), size, "", ""]))
        .collect();
    tables += &table(
        15,
        "parameter_type",
        &[
            "PARAMETER_TYPE_ID",
            "ATOMIC_TYPE_NUMBER",
            "PROGRAM_ID",
            "PARAMETER_TYPE_NAME",
            "PARAMETER_TYPE_SIZE",
            "PARAMETER_MINIMUM_VALUE",
            "PARAMETER_MAXIMUM_VALUE",
        ],
        &type_rows,
    );
    let mut columns = vec![
        "PARAMETER_TYPE_ID",
        "REAL_VALUE",
        "DISPLAYED_VALUE",
        "DISPLAY_ORDER",
        "PARAMETER_VALUE_ID",
    ];
    if binary_columns {
        columns.extend(["BINARY_VALUE", "BINARY_VALUE_LENGTH"]);
    }
    let list_rows: Vec<Vec<String>> = list
        .iter()
        .map(|r| strings(if binary_columns { &r[..] } else { &r[..5] }))
        .collect();
    tables += &table(16, "parameter_list_of_values", &columns, &list_rows);
    format!(
        "EX-IM\r\nN x\r\nK ETS3\r\nK \r\nD 2026-10-28 09:00:00\r\nV 6.3\r\nH virtual_device\r\n{tables}XXX\r\n"
    )
    .into_bytes()
}

fn stored_zip(name: &str, data: &[u8]) -> Vec<u8> {
    let crc = crc32fast::hash(data);
    let mut out = Vec::new();
    out.extend_from_slice(b"PK\x03\x04");
    out.extend_from_slice(&[20, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
    out.extend_from_slice(&crc.to_le_bytes());
    out.extend_from_slice(&(data.len() as u32).to_le_bytes());
    out.extend_from_slice(&(data.len() as u32).to_le_bytes());
    out.extend_from_slice(&(name.len() as u16).to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes());
    out.extend_from_slice(name.as_bytes());
    out.extend_from_slice(data);
    let mut central = Vec::new();
    central.extend_from_slice(b"PK\x01\x02");
    central.extend_from_slice(&[20, 0, 20, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
    central.extend_from_slice(&crc.to_le_bytes());
    central.extend_from_slice(&(data.len() as u32).to_le_bytes());
    central.extend_from_slice(&(data.len() as u32).to_le_bytes());
    central.extend_from_slice(&(name.len() as u16).to_le_bytes());
    central.extend_from_slice(&[0; 12]);
    central.extend_from_slice(&0u32.to_le_bytes());
    central.extend_from_slice(name.as_bytes());
    let start = out.len() as u32;
    out.extend_from_slice(&central);
    out.extend_from_slice(b"PK\x05\x06\0\0\0\0");
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&(central.len() as u32).to_le_bytes());
    out.extend_from_slice(&start.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes());
    out
}

struct Published {
    payload_sha: String,
    program_id: String,
    report: knx_productdb::legacy::LegacyPublishReport,
}

fn try_publish(
    conn: &Connection,
    name: &str,
    exim_bytes: &[u8],
    program: &str,
) -> Result<Published, LegacyPublishError> {
    let zip = stored_zip("MARVIN/ets.vd_", exim_bytes);
    let payload = read_legacy_member(&zip)
        .unwrap()
        .open_unencrypted()
        .unwrap();
    let report = publish_legacy(conn, name, &zip, &payload)?;
    let sha = sha256_hex(exim_bytes);
    Ok(Published {
        program_id: format!("M-1092_A-LX{}-{program}", sha[..8].to_ascii_uppercase()),
        payload_sha: sha,
        report,
    })
}

fn publish(conn: &Connection, exim_bytes: &[u8], program: &str) -> Published {
    try_publish(conn, "marvin.vd4", exim_bytes, program).unwrap()
}

fn database() -> (tempfile::TempDir, std::path::PathBuf, Connection) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("products.sqlite");
    let conn = open_and_migrate(&path).unwrap();
    (dir, path, conn)
}

fn dump(conn: &Connection, sql: &str) -> Vec<Vec<String>> {
    let mut stmt = conn.prepare(sql).unwrap();
    let width = stmt.column_count();
    let mut rows: Vec<Vec<String>> = stmt
        .query_map([], |r| {
            (0..width)
                .map(|i| r.get::<_, Value>(i).map(|v| format!("{v:?}")))
                .collect()
        })
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    rows.sort();
    rows
}

fn all_tables_except(conn: &Connection, skip: &[&str]) -> BTreeMap<String, Vec<Vec<String>>> {
    let names: Vec<String> = conn
        .prepare("SELECT name FROM sqlite_master WHERE type = 'table' ORDER BY name")
        .unwrap()
        .query_map([], |r| r.get(0))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    names
        .into_iter()
        .filter(|n| !skip.contains(&n.as_str()))
        .map(|n| {
            let rows = dump(conn, &format!("SELECT * FROM \"{n}\""));
            (n, rows)
        })
        .collect()
}

/// `(id, value, text, display_order, binary_value)` of one type's enumerations by display order.
fn enums(
    conn: &Connection,
    program_id: &str,
    type_n: &str,
) -> Vec<(String, String, String, i64, Option<String>)> {
    conn.prepare(
        "SELECT id, value, text, display_order, binary_value FROM parameter_type_enum
         WHERE program_id = ?1 AND parameter_type_id = ?2 ORDER BY display_order, id",
    )
    .unwrap()
    .query_map([program_id, &format!("{program_id}_PT-{type_n}")], |r| {
        Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?))
    })
    .unwrap()
    .collect::<Result<_, _>>()
    .unwrap()
}

fn diagnostics(conn: &Connection, sha: &str) -> Vec<(i64, String, String)> {
    conn.prepare(
        "SELECT ordinal, kind, detail FROM legacy_diagnostic WHERE payload_sha256 = ?1 ORDER BY ordinal",
    )
    .unwrap()
    .query_map([sha], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
    .unwrap()
    .collect::<Result<_, _>>()
    .unwrap()
}

fn kind_details(report: &knx_productdb::legacy::LegacyPublishReport, kind: &str) -> Vec<String> {
    report
        .diagnostics
        .iter()
        .filter(|(k, _)| k == kind)
        .map(|(_, d)| d.clone())
        .collect()
}

const BINARY_KINDS: [&str; 5] = [
    "binary-values-invalid",
    "atomic4-binary-fields-unmodeled",
    "binary-values-backfilled",
    "binary-values-backfill-error",
    "binary-values-not-modeled",
];

// ---------------------------------------------------------------- fixtures

/// Types 21 (16 bit), 24 (24 bit), 25 (8 bit); every row valid.
fn valid_types() -> Vec<(&'static str, &'static str, &'static str)> {
    vec![("21", "5", "16"), ("24", "5", "24"), ("25", "5", "8")]
}

/// REAL_VALUE deliberately unlike the bytes; two rows tie on order 1 with the duplicate
/// value 1 (suffix key), upper and lower case hex, leading zero bytes at two lengths.
fn valid_rows() -> Vec<Row> {
    vec![
        ["21", "513", "Two-oh-one", "2", "v1", "0001", "2"],
        ["21", "1", "One", "1", "v2", "0aFf", "2"],
        ["21", "1", "One again", "1", "v3", "00ff", "2"],
        ["21", "9", "Nine", "1", "v4", "FFFE", "2"],
        ["24", "5", "Five", "1", "v5", "000000", "3"],
        ["24", "6", "Six", "2", "v6", "000102", "03"],
        ["25", "1", "Single", "1", "v7", "00", "1"],
    ]
}

/// Type 26 holds one invalid row; type 30 is a plain enumeration with binary fields.
fn main_source(program: &str) -> Vec<u8> {
    let mut types = valid_types();
    types.push(("26", "5", "16"));
    types.push(("30", "4", "8"));
    let mut rows = valid_rows();
    rows.push(["26", "1", "Bad", "1", "v8", "00zz", "2"]);
    rows.push(["30", "1", "A", "1", "a1", "7e7e", "2"]);
    rows.push(["30", "2", "B", "2", "a2", "", "3"]);
    rows.push(["30", "3", "C", "3", "a3", "", ""]);
    exim(program, &types, &rows, true)
}

fn expected_valid(program_id: &str) -> Vec<(String, Vec<EnumRow>)> {
    let row = |type_n: &str, id: &str, value: &str, text: &str, order: i64, bin: &str| {
        (
            format!("{program_id}_PT-{type_n}_EN-{id}"),
            value.to_string(),
            text.to_string(),
            order,
            Some(bin.to_string()),
        )
    };
    vec![
        (
            "21".into(),
            vec![
                row("21", "1", "1", "One", 0, "Cv8="),
                row("21", "1-v3", "1", "One again", 1, "AP8="),
                row("21", "9", "9", "Nine", 2, "//4="),
                row("21", "513", "513", "Two-oh-one", 3, "AAE="),
            ],
        ),
        (
            "24".into(),
            vec![
                row("24", "5", "5", "Five", 0, "AAAA"),
                row("24", "6", "6", "Six", 1, "AAEC"),
            ],
        ),
        ("25".into(), vec![row("25", "1", "1", "Single", 0, "AA==")]),
    ]
}

fn assert_valid_enums(conn: &Connection, program_id: &str) {
    for (type_n, expected) in expected_valid(program_id) {
        assert_eq!(enums(conn, program_id, &type_n), expected, "type {type_n}");
    }
}

// ---------------------------------------------------------------- 1 fresh

#[test]
fn fresh_publish_stores_exact_base64_and_reopens_stable() {
    let (_dir, path, conn) = database();
    let source = exim("300", &valid_types(), &valid_rows(), true);
    let p = publish(&conn, &source, "300");
    assert_valid_enums(&conn, &p.program_id);
    // `value` stays the selector, never the bytes; no binary diagnostic at all.
    for kind in BINARY_KINDS {
        assert!(kind_details(&p.report, kind).is_empty(), "{kind}");
    }
    assert!(diagnostics(&conn, &p.payload_sha)
        .iter()
        .all(|(_, k, _)| !BINARY_KINDS.contains(&k.as_str())));
    let before = all_tables_except(&conn, &[]);
    drop(conn);
    let conn = open_and_migrate(&path).unwrap();
    assert_valid_enums(&conn, &p.program_id);
    assert_eq!(all_tables_except(&conn, &[]), before);
    // The same payload again is skipped and changes nothing but its own name.
    let again = try_publish(&conn, "other.vd4", &source, "300").unwrap();
    assert!(again.report.skipped);
    assert_valid_enums(&conn, &p.program_id);
}

// ---------------------------------------------------------------- 2 malformed

#[test]
fn malformed_binary_values_are_unset_diagnosed_and_retained() {
    let cases: &[(&str, &str)] = &[
        ("", "2"),
        ("000", "1"),
        ("00zz", "2"),
        ("0x0001", "3"),
        ("０００１", "2"),
        (" 0001", "2"),
        ("0001 ", "2"),
        ("00 01", "2"),
        (r"00\n01", "2"),
        (r"00\r01", "2"),
        ("0001", ""),
        ("0001", "+2"),
        ("0001", " 2"),
        ("0001", "3"),
        ("0001", "1"),
        ("0001", "99999999999999999999"),
        ("0001", "٢"),
        ("0001", "2.0"),
    ];
    let reals: Vec<String> = (0..cases.len()).map(|i| format!("{}", 100 + i)).collect();
    let ids: Vec<String> = (0..cases.len()).map(|i| format!("w{i}")).collect();
    let mut rows: Vec<Row> = Vec::new();
    for (i, (hex, len)) in cases.iter().enumerate() {
        let leak = |s: String| -> &'static str { Box::leak(s.into_boxed_str()) };
        rows.push([
            "21",
            leak(reals[i].clone()),
            leak(format!("text{i}")),
            leak(format!("{}", i + 1)),
            leak(ids[i].clone()),
            hex,
            len,
        ]);
    }
    rows.push(["21", "900", "Good", "99", "good", "0001", "02"]);
    rows.push(["26", "1", "x", "1", "s1", "0001", "2"]);
    rows.push(["27", "1", "x", "1", "s2", "0001", "2"]);
    rows.push(["28", "1", "x", "1", "s3", "0001", "2"]);
    rows.push(["29", "1", "x", "1", "s4", "0001", "2"]);
    let types = [
        ("21", "5", "16"),
        ("26", "5", ""),
        ("27", "5", "abc"),
        ("28", "5", "8"),
        ("29", "5", "24"),
    ];
    let (_dir, _path, conn) = database();
    let source = exim("300", &types, &rows, true);
    let p = publish(&conn, &source, "300");
    let stored = enums(&conn, &p.program_id, "21");
    assert_eq!(stored.len(), cases.len() + 1);
    for (i, (id, value, text, order, binary)) in stored.iter().take(cases.len()).enumerate() {
        assert_eq!(id, &format!("{}_PT-21_EN-{}", p.program_id, 100 + i));
        assert_eq!(
            (value.as_str(), text.as_str(), *order),
            (reals[i].as_str(), format!("text{i}").as_str(), i as i64),
            "case {i}"
        );
        assert_eq!(binary, &None, "case {i}: {:?}", cases[i]);
    }
    assert_eq!(stored[cases.len()].4.as_deref(), Some("AAE="));
    for n in ["26", "27", "28", "29"] {
        assert_eq!(enums(&conn, &p.program_id, n)[0].4, None, "type {n}");
    }
    let invalid = kind_details(&p.report, "binary-values-invalid");
    let expected = [
        format!("parameter type 21: {} of {} ", cases.len(), cases.len() + 1),
        "parameter type 26: 1 of 1 ".into(),
        "parameter type 27: 1 of 1 ".into(),
        "parameter type 28: 1 of 1 ".into(),
        "parameter type 29: 1 of 1 ".into(),
    ];
    assert_eq!(invalid.len(), expected.len(), "{invalid:?}");
    for (detail, prefix) in invalid.iter().zip(&expected) {
        assert!(detail.starts_with(prefix.as_str()), "{detail} vs {prefix}");
    }
    assert!(kind_details(&p.report, "binary-values-not-modeled").is_empty());
    // The raw lexemes stay in the retained payload.
    let bytes: Vec<u8> = conn
        .query_row(
            "SELECT bytes FROM source_file WHERE sha256 = ?1",
            [&p.payload_sha],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(bytes, source);
    for needle in ["00zz", "０００１", r"00\n01", "99999999999999999999"] {
        assert!(
            bytes.windows(needle.len()).any(|w| w == needle.as_bytes()),
            "{needle}"
        );
    }
}

#[test]
fn absent_binary_columns_leave_the_value_unset_and_are_diagnosed() {
    let (_dir, _path, conn) = database();
    let rows = [
        ["21", "1", "One", "1", "v1", "", ""],
        ["21", "2", "Two", "2", "v2", "", ""],
        ["40", "1", "Plain", "1", "v3", "", ""],
    ];
    let source = exim("300", &[("21", "5", "16"), ("40", "4", "8")], &rows, false);
    let p = publish(&conn, &source, "300");
    assert!(enums(&conn, &p.program_id, "21")
        .iter()
        .all(|e| e.4.is_none()));
    let invalid = kind_details(&p.report, "binary-values-invalid");
    assert_eq!(invalid.len(), 1);
    assert!(
        invalid[0].starts_with("parameter type 21: 2 of 2 "),
        "{invalid:?}"
    );
    assert!(kind_details(&p.report, "atomic4-binary-fields-unmodeled").is_empty());
}

// ---------------------------------------------------------------- 3 atomic 4

#[test]
fn atomic4_binary_fields_are_diagnosed_never_interpreted() {
    let (_dir, _path, conn) = database();
    let p = publish(&conn, &main_source("300"), "300");
    for (_, _, _, _, binary) in enums(&conn, &p.program_id, "30") {
        assert_eq!(binary, None);
    }
    assert_eq!(enums(&conn, &p.program_id, "30").len(), 3);
    let unknown = kind_details(&p.report, "atomic4-binary-fields-unmodeled");
    assert_eq!(unknown.len(), 1, "{unknown:?}");
    assert!(
        unknown[0].starts_with("parameter type 30: 2 enumeration rows "),
        "{unknown:?}"
    );
    let stored: Vec<_> = diagnostics(&conn, &p.payload_sha)
        .into_iter()
        .filter(|(_, k, _)| k == "atomic4-binary-fields-unmodeled")
        .collect();
    assert_eq!(stored.len(), 1);
    let bytes: Vec<u8> = conn
        .query_row(
            "SELECT bytes FROM source_file WHERE sha256 = ?1",
            [&p.payload_sha],
            |r| r.get(0),
        )
        .unwrap();
    assert!(bytes.windows(4).any(|w| w == b"7e7e"));
}

// ---------------------------------------------------------------- 4 upgrade

/// Old-style (v29) state: no binary values, the old not-modeled diagnostics only.
fn rewind_to_v29(conn: &Connection, published: &[(&Published, OldDiagnostics)]) {
    conn.execute("UPDATE parameter_type_enum SET binary_value = NULL", [])
        .unwrap();
    for (p, old) in published {
        conn.execute(
            "DELETE FROM legacy_diagnostic WHERE payload_sha256 = ?1 AND kind IN
             ('binary-values-invalid','atomic4-binary-fields-unmodeled','binary-values-not-modeled')",
            [&p.payload_sha],
        )
        .unwrap();
        for (type_n, values, inconsistent) in *old {
            let ordinal: i64 = conn
                .query_row(
                    "SELECT coalesce(max(ordinal), -1) + 1 FROM legacy_diagnostic WHERE payload_sha256 = ?1",
                    [&p.payload_sha],
                    |r| r.get(0),
                )
                .unwrap();
            conn.execute(
                "INSERT INTO legacy_diagnostic (payload_sha256, ordinal, kind, detail) VALUES (?1,?2,'binary-values-not-modeled',?3)",
                rusqlite::params![
                    p.payload_sha,
                    ordinal,
                    format!("parameter type {type_n}: {values} binary enumeration values ({inconsistent} inconsistent with their length or the type's size) are kept in the stored payload only; the product database does not model them"),
                ],
            )
            .unwrap();
        }
    }
    conn.pragma_update(None, "user_version", 29).unwrap();
}

const MAIN_OLD: [(&str, usize, usize); 4] =
    [("21", 4, 0), ("24", 2, 0), ("25", 1, 0), ("26", 1, 1)];

fn enum_columns(conn: &Connection) -> Vec<Vec<String>> {
    dump(conn, "SELECT program_id, parameter_type_id, id, value, text, display_order, binary_value FROM parameter_type_enum")
}

#[test]
fn a_v29_database_is_upgraded_to_exactly_the_fresh_values() {
    let (dir, path, conn) = database();
    let source = main_source("300");
    let p = publish(&conn, &source, "300");
    let fresh_db = database();
    let fresh = publish(&fresh_db.2, &source, "300");
    assert_eq!(fresh.program_id, p.program_id);
    let fresh_enums = enum_columns(&fresh_db.2);
    let fresh_types = dump(&fresh_db.2, "SELECT * FROM parameter_type");

    rewind_to_v29(&conn, &[(&p, &MAIN_OLD)]);
    // Unrelated unknown rows and an unrelated provenance fact must survive.
    conn.execute(
        "INSERT INTO ingest_unknown (source_sha256, program_id, xpath, kind, name, occurrences, sample)
         VALUES ('unrelated', NULL, '/x', 'Element', 'Y', 3, 's')",
        [],
    )
    .unwrap();
    let old_diags = diagnostics(&conn, &p.payload_sha);
    assert_eq!(
        old_diags
            .iter()
            .filter(|(_, k, _)| k == "binary-values-not-modeled")
            .count(),
        4
    );
    let others = all_tables_except(&conn, &["parameter_type_enum", "legacy_diagnostic"]);
    let before_enums = enum_columns(&conn);
    drop(conn);

    let conn = open_and_migrate(&path).unwrap();
    let version: i64 = conn
        .query_row("PRAGMA user_version", [], |r| r.get(0))
        .unwrap();
    assert_eq!(version, CURRENT_PRODUCTDB_VERSION);
    assert_eq!(CURRENT_PRODUCTDB_VERSION, 30);
    assert_eq!(enum_columns(&conn), fresh_enums);
    assert_eq!(dump(&conn, "SELECT * FROM parameter_type"), fresh_types);
    // Only binary_value differs from the pre-upgrade rows.
    let strip = |rows: &[Vec<String>]| -> Vec<Vec<String>> {
        rows.iter().map(|r| r[..6].to_vec()).collect()
    };
    assert_eq!(strip(&enum_columns(&conn)), strip(&before_enums));
    assert_eq!(
        all_tables_except(&conn, &["parameter_type_enum", "legacy_diagnostic"]),
        others
    );

    // Old diagnostics: identical prefix, same ordinals; v30 findings appended after them.
    let after = diagnostics(&conn, &p.payload_sha);
    assert_eq!(&after[..old_diags.len()], &old_diags[..]);
    let appended = &after[old_diags.len()..];
    assert!(appended
        .iter()
        .all(|(o, _, _)| *o > old_diags.last().unwrap().0));
    let kinds: Vec<&str> = appended.iter().map(|(_, k, _)| k.as_str()).collect();
    assert_eq!(
        kinds
            .iter()
            .filter(|k| **k == "binary-values-invalid")
            .count(),
        1,
        "{kinds:?}"
    );
    assert_eq!(
        kinds
            .iter()
            .filter(|k| **k == "atomic4-binary-fields-unmodeled")
            .count(),
        1
    );
    assert_eq!(
        kinds
            .iter()
            .filter(|k| **k == "binary-values-backfilled")
            .count(),
        1
    );
    assert!(!kinds.contains(&"binary-values-backfill-error"));
    for (_, k, d) in appended {
        if k == "binary-values-backfilled" {
            assert!(
                d.contains("7 binary values") && d.contains("1 programs"),
                "{d}"
            );
        } else {
            assert!(d.contains(BACKFILL_NOTE), "{d}");
        }
    }

    // Reopen and re-running the step are stable.
    let settled = all_tables_except(&conn, &[]);
    drop(conn);
    let conn = open_and_migrate(&path).unwrap();
    assert_eq!(all_tables_except(&conn, &[]), settled);
    conn.pragma_update(None, "user_version", 29).unwrap();
    drop(conn);
    let conn = open_and_migrate(&path).unwrap();
    assert_eq!(all_tables_except(&conn, &[]), settled);

    // Skipped re-ingest surfaces the persisted findings.
    let again = try_publish(&conn, "again.vd4", &source, "300").unwrap();
    assert!(again.report.skipped);
    assert_eq!(
        kind_details(&again.report, "binary-values-backfilled").len(),
        1
    );
    assert_eq!(
        kind_details(&again.report, "binary-values-invalid").len(),
        1
    );
    assert_eq!(enum_columns(&conn), fresh_enums);
    drop(dir);
}

// ---------------------------------------------------------------- 5 refusals

struct Ctx {
    a_sha: String,
    a: String,
    b_sha: String,
    b: String,
}

fn a_source() -> Vec<u8> {
    let types = [("21", "5", "16"), ("24", "5", "24"), ("26", "5", "16")];
    let rows: Vec<Row> = vec![
        ["21", "513", "Two-oh-one", "2", "v1", "0001", "2"],
        ["21", "1", "One", "1", "v2", "0aFf", "2"],
        ["24", "5", "Five", "1", "v5", "000000", "3"],
        ["24", "6", "Six", "2", "v6", "000102", "3"],
        ["26", "1", "Bad", "1", "v8", "00zz", "2"],
    ];
    exim("300", &types, &rows, true)
}

fn b_source() -> Vec<u8> {
    let rows: Vec<Row> = vec![
        ["21", "7", "Seven", "1", "w1", "0007", "2"],
        ["21", "8", "Eight", "2", "w2", "0108", "2"],
    ];
    exim("301", &[("21", "5", "16")], &rows, true)
}

const A_OLD: [(&str, usize, usize); 3] = [("21", 2, 0), ("24", 2, 0), ("26", 1, 1)];
const B_OLD: [(&str, usize, usize); 1] = [("21", 2, 0)];

/// Two payloads in a v29-state file; returns the file for tampering.
fn two_payloads() -> (tempfile::TempDir, std::path::PathBuf, Ctx) {
    let (dir, path, conn) = database();
    let a = publish(&conn, &a_source(), "300");
    let b = publish(&conn, &b_source(), "301");
    rewind_to_v29(&conn, &[(&a, &A_OLD), (&b, &B_OLD)]);
    let ctx = Ctx {
        a_sha: a.payload_sha,
        a: a.program_id,
        b_sha: b.payload_sha,
        b: b.program_id,
    };
    (dir, path, ctx)
}

fn fresh_b_enums(ctx: &Ctx) -> Vec<Vec<String>> {
    let db = database();
    let b = publish(&db.2, &b_source(), "301");
    assert_eq!(b.program_id, ctx.b);
    dump(
        &db.2,
        &format!(
            "SELECT * FROM parameter_type_enum WHERE program_id = '{}'",
            ctx.b
        ),
    )
}

fn tamper_exec(path: &std::path::Path, statements: &[String]) {
    let conn = Connection::open(path).unwrap();
    conn.pragma_update(None, "foreign_keys", "OFF").unwrap();
    for s in statements {
        conn.execute_batch(s).unwrap_or_else(|e| panic!("{s}: {e}"));
    }
}

fn rekey_statements(old: &str, new_bytes: &[u8]) -> Vec<String> {
    let new = sha256_hex(new_bytes);
    let hex: String = new_bytes.iter().map(|b| format!("{b:02x}")).collect();
    vec![
        format!("UPDATE source_file SET sha256 = '{new}', bytes = X'{hex}', len = {} WHERE sha256 = '{old}'", new_bytes.len()),
        format!("UPDATE legacy_source SET payload_sha256 = '{new}', namespace = 'LX{}' WHERE payload_sha256 = '{old}'", new[..8].to_ascii_uppercase()),
        format!("UPDATE legacy_source_file SET payload_sha256 = '{new}' WHERE payload_sha256 = '{old}'"),
        format!("UPDATE legacy_program SET payload_sha256 = '{new}' WHERE payload_sha256 = '{old}'"),
        format!("UPDATE application_program SET source_sha256 = '{new}' WHERE source_sha256 = '{old}'"),
        format!("UPDATE legacy_diagnostic SET payload_sha256 = '{new}' WHERE payload_sha256 = '{old}'"),
    ]
}

#[test]
fn a_payload_that_cannot_be_authenticated_is_refused_whole_and_another_succeeds() {
    type Tamper = fn(&Ctx) -> Vec<String>;
    let cases: Vec<(&str, &str, Tamper)> = vec![
        ("missing", "retained source is missing", |c| {
            vec![format!(
                "DELETE FROM source_file WHERE sha256 = '{}'",
                c.a_sha
            )]
        }),
        ("corrupt", "does not match its SHA-256", |c| {
            vec![format!(
                "UPDATE source_file SET bytes = substr(bytes, 1, length(bytes) - 1) WHERE sha256 = '{}'",
                c.a_sha
            )]
        }),
        ("unparseable", "cannot be parsed or mapped", |c| {
            rekey_statements(&c.a_sha, b"this is not EX-IM")
        }),
        ("namespace", "namespace does not match", |c| {
            vec![format!(
                "UPDATE legacy_source SET namespace = 'LXDEADBEEF' WHERE payload_sha256 = '{}'",
                c.a_sha
            )]
        }),
        ("exim id", "stored programs do not match", |c| {
            vec![format!(
                "UPDATE legacy_program SET exim_program_id = '999' WHERE program_id = '{}'",
                c.a
            )]
        }),
        ("extra program", "stored programs do not match", |c| {
            vec![format!("INSERT INTO legacy_program (program_id, payload_sha256, exim_program_id) VALUES ('M-1092_A-X-777','{}','777')", c.a_sha)]
        }),
        ("owner", "program owner does not match", |c| {
            vec![format!(
                "UPDATE application_program SET source_sha256 = '{}' WHERE id = '{}'",
                c.b_sha, c.a
            )]
        }),
        ("width", "parameter type does not match", |c| {
            vec![format!("UPDATE parameter_type SET size_in_bit = 32 WHERE program_id = '{0}' AND id = '{0}_PT-24'", c.a)]
        }),
        ("base", "parameter type does not match", |c| {
            vec![format!("UPDATE parameter_type SET base = 'Value' WHERE program_id = '{0}' AND id = '{0}_PT-24'", c.a)]
        }),
        ("missing enum", "enumerations do not match", |c| {
            vec![format!("DELETE FROM parameter_type_enum WHERE program_id = '{0}' AND id = '{0}_PT-24_EN-6'", c.a)]
        }),
        ("extra enum", "enumerations do not match", |c| {
            vec![format!("INSERT INTO parameter_type_enum (program_id, parameter_type_id, id, value, text, display_order) VALUES ('{0}','{0}_PT-24','{0}_PT-24_EN-77','77','x',5)", c.a)]
        }),
        ("text", "enumerations do not match", |c| {
            vec![format!("UPDATE parameter_type_enum SET text = 'changed' WHERE program_id = '{0}' AND id = '{0}_PT-24_EN-6'", c.a)]
        }),
        ("order", "enumerations do not match", |c| {
            vec![format!("UPDATE parameter_type_enum SET display_order = 9 WHERE program_id = '{0}' AND id = '{0}_PT-24_EN-6'", c.a)]
        }),
        ("selector", "enumerations do not match", |c| {
            vec![format!("UPDATE parameter_type_enum SET value = '999' WHERE program_id = '{0}' AND id = '{0}_PT-24_EN-6'", c.a)]
        }),
        (
            "conflicting value",
            "conflicts with the retained source",
            |c| {
                vec![format!("UPDATE parameter_type_enum SET binary_value = 'AAAA' WHERE program_id = '{0}' AND id = '{0}_PT-21_EN-513'", c.a)]
            },
        ),
        (
            "fresh none must not clear",
            "conflicts with the retained source",
            |c| {
                vec![format!("UPDATE parameter_type_enum SET binary_value = 'AAAA' WHERE program_id = '{0}' AND id = '{0}_PT-26_EN-1'", c.a)]
            },
        ),
    ];
    let fresh_b = {
        let (_d, _p, ctx) = two_payloads();
        fresh_b_enums(&ctx)
    };
    for (name, reason, tamper) in cases {
        let (_dir, path, ctx) = two_payloads();
        tamper_exec(&path, &tamper(&ctx));
        let (others_before, a_enums_before, b_diags_before) = {
            let conn = Connection::open(&path).unwrap();
            (
                all_tables_except(&conn, &["parameter_type_enum", "legacy_diagnostic"]),
                dump(
                    &conn,
                    &format!(
                        "SELECT * FROM parameter_type_enum WHERE program_id = '{}'",
                        ctx.a
                    ),
                ),
                diagnostics(&conn, &ctx.b_sha),
            )
        };
        let all_diags_before = {
            let conn = Connection::open(&path).unwrap();
            dump(&conn, "SELECT * FROM legacy_diagnostic")
        };
        let conn = open_and_migrate(&path).unwrap();
        assert_eq!(
            all_tables_except(&conn, &["parameter_type_enum", "legacy_diagnostic"]),
            others_before,
            "{name}"
        );
        assert_eq!(
            dump(
                &conn,
                &format!(
                    "SELECT * FROM parameter_type_enum WHERE program_id = '{}'",
                    ctx.a
                )
            ),
            a_enums_before,
            "{name}: payload A rows changed"
        );
        let errors: Vec<(String, String)> = dump(&conn, "SELECT kind, detail FROM legacy_diagnostic WHERE kind = 'binary-values-backfill-error'")
            .into_iter()
            .map(|r| (r[0].clone(), r[1].clone()))
            .collect();
        assert_eq!(errors.len(), 1, "{name}: {errors:?}");
        assert!(errors[0].1.contains(reason), "{name}: {}", errors[0].1);
        // Every old diagnostic row is still there, unchanged.
        let all_diags_after = dump(&conn, "SELECT * FROM legacy_diagnostic");
        for row in &all_diags_before {
            assert!(
                all_diags_after.contains(row),
                "{name}: old diagnostic lost {row:?}"
            );
        }
        // Payload B was upgraded regardless (the "owner" case still leaves B intact).
        assert_eq!(
            dump(
                &conn,
                &format!(
                    "SELECT * FROM parameter_type_enum WHERE program_id = '{}'",
                    ctx.b
                )
            ),
            fresh_b,
            "{name}: payload B"
        );
        let b_diags = diagnostics(&conn, &ctx.b_sha);
        assert_eq!(
            &b_diags[..b_diags_before.len()],
            &b_diags_before[..],
            "{name}"
        );
        assert!(
            b_diags
                .iter()
                .any(|(_, k, _)| k == "binary-values-backfilled"),
            "{name}"
        );
    }
}

#[test]
fn rows_are_paired_by_key_never_by_row_order_and_matching_values_are_idempotent() {
    let (_dir, path, ctx) = two_payloads();
    // Reinsert A's enumerations in reverse; keep one correct value already stored.
    let rows: Vec<StoredEnum> = {
        let conn = Connection::open(&path).unwrap();
        let mut stmt = conn
            .prepare(
                "SELECT parameter_type_id, id, value, text, binary_value, display_order FROM parameter_type_enum
                 WHERE program_id = ?1 ORDER BY rowid DESC",
            )
            .unwrap();
        let rows = stmt
            .query_map([&ctx.a], |r| {
                Ok((
                    r.get(0)?,
                    r.get(1)?,
                    r.get(2)?,
                    r.get(3)?,
                    r.get(4)?,
                    r.get(5)?,
                ))
            })
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();
        rows
    };
    {
        let conn = Connection::open(&path).unwrap();
        conn.execute(
            "DELETE FROM parameter_type_enum WHERE program_id = ?1",
            [&ctx.a],
        )
        .unwrap();
        for (t, id, value, text, _bin, order) in &rows {
            let bin = if id.ends_with("PT-21_EN-513") {
                Some("AAE=")
            } else {
                None
            };
            conn.execute(
                "INSERT INTO parameter_type_enum (program_id, parameter_type_id, id, value, text, display_order, binary_value)
                 VALUES (?1,?2,?3,?4,?5,?6,?7)",
                rusqlite::params![ctx.a, t, id, value, text, order, bin],
            )
            .unwrap();
        }
    }
    let conn = open_and_migrate(&path).unwrap();
    let fresh = database();
    publish(&fresh.2, &a_source(), "300");
    assert_eq!(
        dump(
            &conn,
            &format!(
                "SELECT * FROM parameter_type_enum WHERE program_id = '{}'",
                ctx.a
            )
        ),
        dump(
            &fresh.2,
            &format!(
                "SELECT * FROM parameter_type_enum WHERE program_id = '{}'",
                ctx.a
            )
        ),
    );
    assert!(dump(
        &conn,
        "SELECT kind FROM legacy_diagnostic WHERE kind = 'binary-values-backfill-error'"
    )
    .is_empty());
}

#[test]
fn a_type_with_an_unusable_width_stays_unset_while_valid_types_are_filled() {
    let types = [
        ("21", "5", "16"),
        ("27", "5", "abc"),
        ("26", "5", ""),
        ("29", "5", "24"),
    ];
    let rows: Vec<Row> = vec![
        ["21", "513", "Two-oh-one", "2", "v1", "0001", "2"],
        ["21", "1", "One", "1", "v2", "0aFf", "2"],
        ["27", "1", "x", "1", "s1", "0001", "2"],
        ["26", "1", "x", "1", "s2", "0001", "2"],
        ["29", "1", "x", "1", "s3", "0001", "2"],
    ];
    let source = exim("300", &types, &rows, true);
    let (_dir, path, conn) = database();
    let p = publish(&conn, &source, "300");
    let fresh = database();
    publish(&fresh.2, &source, "300");
    let fresh_enums = enum_columns(&fresh.2);
    let fresh_types = dump(&fresh.2, "SELECT * FROM parameter_type");
    let old = [("21", 2, 0), ("27", 1, 1), ("26", 1, 1), ("29", 1, 1)];
    rewind_to_v29(&conn, &[(&p, &old)]);
    let old_diags = diagnostics(&conn, &p.payload_sha);
    drop(conn);
    let conn = open_and_migrate(&path).unwrap();
    assert_eq!(enum_columns(&conn), fresh_enums);
    assert_eq!(dump(&conn, "SELECT * FROM parameter_type"), fresh_types);
    assert_eq!(
        enums(&conn, &p.program_id, "21")
            .iter()
            .map(|e| e.4.clone())
            .collect::<Vec<_>>(),
        [Some("Cv8=".to_string()), Some("AAE=".to_string())]
    );
    for n in ["27", "26", "29"] {
        assert_eq!(enums(&conn, &p.program_id, n)[0].4, None, "type {n}");
    }
    let after = diagnostics(&conn, &p.payload_sha);
    assert_eq!(&after[..old_diags.len()], &old_diags[..]);
    let appended = &after[old_diags.len()..];
    assert!(appended
        .iter()
        .all(|(_, k, _)| k != "binary-values-backfill-error"));
    assert_eq!(
        appended
            .iter()
            .filter(|(_, k, _)| k == "binary-values-invalid")
            .count(),
        3
    );
    assert!(appended
        .iter()
        .any(|(_, k, d)| k == "binary-values-backfilled" && d.contains("2 binary values")));
}

#[test]
fn a_skipped_reingest_reports_history_and_the_persisted_backfill_error() {
    for case in ["missing", "corrupt"] {
        let (_dir, path, ctx) = two_payloads();
        let statement = match case {
            "missing" => format!("DELETE FROM source_file WHERE sha256 = '{}'", ctx.a_sha),
            _ => format!(
                "UPDATE source_file SET bytes = substr(bytes, 1, length(bytes) - 1) WHERE sha256 = '{}'",
                ctx.a_sha
            ),
        };
        tamper_exec(&path, &[statement]);
        let old_a = {
            let conn = Connection::open(&path).unwrap();
            diagnostics(&conn, &ctx.a_sha)
        };
        let conn = open_and_migrate(&path).unwrap();
        let a_enums = dump(
            &conn,
            &format!(
                "SELECT * FROM parameter_type_enum WHERE program_id = '{}'",
                ctx.a
            ),
        );
        let again = try_publish(&conn, "again.vd4", &a_source(), "300").unwrap();
        assert!(again.report.skipped, "{case}");
        let reported = &again.report.diagnostics;
        for (_, kind, detail) in &old_a {
            assert!(
                reported.iter().any(|(k, d)| k == kind && d == detail),
                "{case}: history lost: {kind}"
            );
        }
        assert_eq!(
            kind_details(&again.report, "binary-values-backfill-error").len(),
            1,
            "{case}"
        );
        assert_eq!(
            kind_details(&again.report, "binary-values-backfilled").len(),
            0,
            "{case}"
        );
        assert!(
            enums(&conn, &ctx.a, "21").iter().all(|e| e.4.is_none()),
            "{case}"
        );
        assert_eq!(
            dump(
                &conn,
                &format!(
                    "SELECT * FROM parameter_type_enum WHERE program_id = '{}'",
                    ctx.a
                )
            ),
            a_enums,
            "{case}"
        );
        // Reopen and a second run of the step add no second failure finding.
        conn.pragma_update(None, "user_version", 29).unwrap();
        drop(conn);
        let conn = open_and_migrate(&path).unwrap();
        let errors = diagnostics(&conn, &ctx.a_sha)
            .into_iter()
            .filter(|(_, k, _)| k == "binary-values-backfill-error")
            .count();
        assert_eq!(errors, 1, "{case}");
    }
}

// ---------------------------------------------------------------- 6 duplicate keys

fn colliding_source() -> Vec<u8> {
    let rows: Vec<Row> = vec![
        ["21", "1", "a", "1", "", "0001", "2"],
        ["21", "1", "b", "2", "", "0002", "2"],
        ["21", "1", "c", "3", "", "0003", "2"],
    ];
    exim("300", &[("21", "5", "16")], &rows, true)
}

#[test]
fn a_colliding_duplicate_key_is_refused_fresh_and_by_the_backfill() {
    // Characterization: the third row maps to the key of the second; fresh publish refuses.
    let (_dir, _path, conn) = database();
    let before = all_tables_except(&conn, &[]);
    let error = try_publish(&conn, "x.vd4", &colliding_source(), "300")
        .err()
        .expect("refused");
    assert!(matches!(error, LegacyPublishError::Store(_)), "{error:?}");
    assert_eq!(all_tables_except(&conn, &[]), before);

    // A hand-built inventory (authentic hash, correct owners) for that source: the key is ambiguous.
    let source = colliding_source();
    let sha = sha256_hex(&source);
    let ns = format!("LX{}", sha[..8].to_ascii_uppercase());
    let program = format!("M-1092_A-{ns}-300");
    let hex: String = source.iter().map(|b| format!("{b:02x}")).collect();
    conn.execute_batch(&format!(
        "INSERT INTO manufacturer (id) VALUES ('M-1092');
         INSERT INTO source_file (sha256, source_path, manufacturer_id, len, bytes) VALUES ('{sha}','x!ets.vd_','M-1092',{},X'{hex}');
         INSERT INTO legacy_source (payload_sha256, namespace, member_name, member_kind, charset) VALUES ('{sha}','{ns}','ets.vd_','ProductDatabase','windows-1252');
         INSERT INTO application_program (id, manufacturer_id, source_sha256) VALUES ('{program}','M-1092','{sha}');
         INSERT INTO legacy_program (program_id, payload_sha256, exim_program_id) VALUES ('{program}','{sha}','300');
         INSERT INTO parameter_type (program_id, id, kind, size_in_bit, base) VALUES ('{program}','{program}_PT-21','Restriction',16,'BinaryValue');
         INSERT INTO parameter_type_enum (program_id, parameter_type_id, id, value, text, display_order) VALUES
           ('{program}','{program}_PT-21','{program}_PT-21_EN-1','1','a',0),
           ('{program}','{program}_PT-21','{program}_PT-21_EN-1-','1','b',1);",
        source.len()
    ))
    .unwrap();
    conn.pragma_update(None, "user_version", 29).unwrap();
    let stored = all_tables_except(&conn, &["legacy_diagnostic"]);
    drop(conn);
    let conn = open_and_migrate(&_path).unwrap();
    assert_eq!(all_tables_except(&conn, &["legacy_diagnostic"]), stored);
    let errors = dump(
        &conn,
        "SELECT detail FROM legacy_diagnostic WHERE kind = 'binary-values-backfill-error'",
    );
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert!(
        errors[0][0].contains("maps one enumeration key twice"),
        "{errors:?}"
    );
}

// ---------------------------------------------------------------- 7 eligibility

#[test]
fn download_eligibility_and_unrelated_columns_are_unchanged() {
    let fixture = std::fs::read(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures/legacy/marvin-program-plain.vd4"),
    )
    .unwrap();
    let (_dir, path, conn) = database();
    let payload = read_legacy_member(&fixture)
        .unwrap()
        .open_unencrypted()
        .unwrap();
    publish_legacy(&conn, "marvin.vd4", &fixture, &payload).unwrap();
    let program: String = conn
        .query_row("SELECT program_id FROM legacy_program", [], |r| r.get(0))
        .unwrap();
    let code_before = format!("{:?}", load_program_code(&conn, &program));
    conn.execute("UPDATE parameter_type_enum SET binary_value = NULL", [])
        .unwrap();
    conn.pragma_update(None, "user_version", 29).unwrap();
    let enums_before = enum_columns(&conn);
    let others = all_tables_except(&conn, &["parameter_type_enum", "legacy_diagnostic"]);
    drop(conn);
    let conn = open_and_migrate(&path).unwrap();
    assert_eq!(
        format!("{:?}", load_program_code(&conn, &program)),
        code_before
    );
    assert_eq!(
        enum_columns(&conn),
        enums_before,
        "no atomic-5 type in this fixture"
    );
    assert_eq!(
        all_tables_except(&conn, &["parameter_type_enum", "legacy_diagnostic"]),
        others
    );
    // Nothing the experimental image path keys on was added to the program.
    let (xml_sources,): (i64,) = conn
        .query_row(
            "SELECT count(*) FROM source_file WHERE source_path LIKE '%.xml'",
            [],
            |r| Ok((r.get(0)?,)),
        )
        .unwrap();
    assert_eq!(xml_sources, 0);
}

// ---------------------------------------------------------------- 8 read-only

#[test]
fn a_v29_file_is_upgraded_in_a_read_only_copy_only() {
    let (_dir, path, conn) = database();
    let p = publish(&conn, &main_source("300"), "300");
    rewind_to_v29(&conn, &[(&p, &MAIN_OLD)]);
    drop(conn);
    let before = std::fs::read(&path).unwrap();
    let db = open_read_only(&path).unwrap();
    assert_eq!(db.migrated_from, Some(29));
    assert_valid_enums(&db.conn, &p.program_id);
    drop(db);
    assert_eq!(std::fs::read(&path).unwrap(), before);
}
