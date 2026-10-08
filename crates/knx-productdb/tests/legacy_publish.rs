//! Publishing a legacy EX-IM database writes everything in one transaction, or nothing.

use std::collections::BTreeSet;

use knx_productdb::device_evaluation::evaluate_device;
use knx_productdb::legacy::{
    publish_legacy, read_legacy_member, withhold_secret_values, LegacyPayload, LegacyPublishError,
};
use knx_productdb::{open_and_migrate, sha256_hex, Connection, CURRENT_PRODUCTDB_VERSION};

fn fixture(path: &str) -> Vec<u8> {
    let full = knx_testsupport::workspace_root()
        .join("crates/knx-productdb/fixtures/legacy")
        .join(path);
    std::fs::read(&full).unwrap_or_else(|e| panic!("fixture {full:?} unreadable: {e}"))
}

fn payload(bytes: &[u8]) -> LegacyPayload {
    read_legacy_member(bytes)
        .unwrap()
        .open_unencrypted()
        .unwrap()
}

fn database() -> (tempfile::TempDir, Connection) {
    let dir = tempfile::tempdir().unwrap();
    let conn = open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
    (dir, conn)
}

fn count(conn: &Connection, sql: &str) -> i64 {
    conn.query_row(sql, [], |r| r.get(0)).unwrap()
}

/// Every row of every table, for before/after comparisons.
fn snapshot(conn: &Connection) -> Vec<(String, i64)> {
    let tables: Vec<String> = conn
        .prepare("SELECT name FROM sqlite_master WHERE type = 'table' ORDER BY name")
        .unwrap()
        .query_map([], |r| r.get(0))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    tables
        .into_iter()
        .map(|t| {
            let n = count(conn, &format!("SELECT count(*) FROM \"{t}\""));
            (t, n)
        })
        .collect()
}

fn program_id(bytes: &[u8]) -> String {
    let sha = sha256_hex(
        &withhold_secret_values(payload(bytes).bytes())
            .unwrap()
            .bytes,
    );
    format!("M-1092_A-LX{}-300", sha[..8].to_ascii_uppercase())
}

#[test]
fn the_schema_is_v22_with_the_legacy_provenance_tables() {
    assert_eq!(CURRENT_PRODUCTDB_VERSION, 22);
    let (_dir, conn) = database();
    for table in [
        "legacy_source",
        "legacy_source_file",
        "legacy_program",
        "legacy_diagnostic",
    ] {
        assert_eq!(
            count(
                &conn,
                &format!("SELECT count(*) FROM sqlite_master WHERE name = '{table}'")
            ),
            1,
            "{table}"
        );
    }
}

#[test]
fn a_program_database_is_published_with_its_provenance() {
    let (_dir, conn) = database();
    let bytes = fixture("marvin-program-plain.vd4");
    let payload = payload(&bytes);
    let report = publish_legacy(&conn, "marvin.vd4", &bytes, &payload).unwrap();
    let id = program_id(&bytes);
    // The stored payload is the decrypted one without its secret values.
    let stored = withhold_secret_values(payload.bytes()).unwrap().bytes;
    assert_ne!(stored, payload.bytes());
    assert!(!report.skipped);
    assert_eq!(report.programs, std::slice::from_ref(&id));
    assert_eq!(report.payload_sha256, sha256_hex(&stored));
    assert_eq!(report.original_sha256, sha256_hex(&bytes));
    assert_eq!(
        (
            report.parameters,
            report.parameter_refs,
            report.com_object_refs
        ),
        (10, 11, 3)
    );
    assert_eq!(report.translations, 8);
    assert!(report
        .diagnostics
        .iter()
        .any(|(kind, detail)| kind == "unmapped-table" && detail.contains("s19_block")));

    // Rows point at the stored payload; the original is kept byte for byte,
    // the payload byte for byte except its withheld secret values.
    let source: String = conn
        .query_row(
            "SELECT source_sha256 FROM application_program WHERE id = ?1",
            [&id],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(source, report.payload_sha256);
    for (sha, expected) in [
        (&report.payload_sha256, &stored[..]),
        (&report.original_sha256, &bytes[..]),
    ] {
        assert_eq!(
            knx_productdb::load_source_file(&conn, sha)
                .unwrap()
                .as_deref(),
            Some(expected)
        );
    }
    assert_eq!(count(&conn, "SELECT count(*) FROM legacy_program"), 1);
    assert_eq!(count(&conn, "SELECT count(*) FROM catalog_item"), 1);
    assert_eq!(count(&conn, "SELECT count(*) FROM translation"), 8);
    let stored_diagnostics = count(&conn, "SELECT count(*) FROM legacy_diagnostic");
    assert_eq!(stored_diagnostics as usize, report.diagnostics.len());
}

fn active(conn: &Connection, program: &str, values: &[(&str, &str)]) -> BTreeSet<String> {
    let stored = values
        .iter()
        .map(|(r, v)| (format!("{program}_{r}"), v.to_string()))
        .collect();
    let evaluation = evaluate_device(conn, program, stored, &[]).unwrap();
    let activation = &evaluation.activation;
    activation
        .parameter_refs
        .iter()
        .chain(&activation.com_object_refs)
        .map(|r| {
            r.ref_id
                .strip_prefix(&format!("{program}_"))
                .unwrap()
                .to_string()
        })
        .collect()
}

#[test]
fn the_evaluator_shows_what_the_parent_values_select() {
    let (_dir, conn) = database();
    let bytes = fixture("marvin-program-plain.vd4");
    publish_legacy(&conn, "marvin.vd4", &bytes, &payload(&bytes)).unwrap();
    let id = program_id(&bytes);
    let set = |items: &[&str]| items.iter().map(|s| s.to_string()).collect::<BTreeSet<_>>();
    // Pages are labels to the evaluator, not active parameter refs.
    let always = [
        "O-0_R-10000",
        "P-1001_R-1001",
        "P-1010_R-1010",
        "P-2001_R-2001",
    ];
    // Defaults: mode 1, hidden control 1.
    let mut expected = set(&always);
    expected.extend(set(&[
        "P-1006_R-1006",
        "P-1002_R-1002",
        "UP-1004_R-1004",
        "O-1_R-10001",
        "P-1011_R-1011",
    ]));
    assert_eq!(active(&conn, &id, &[]), expected);
    // Mode 0 switches the branch; the hidden control 0 hides what it governs.
    let mut expected = set(&always);
    expected.extend(set(&["P-1002_R-1003", "UP-1005_R-1005", "O-1_R-10002"]));
    assert_eq!(
        active(
            &conn,
            &id,
            &[("P-1001_R-1001", "0"), ("P-1010_R-1010", "0")]
        ),
        expected
    );
}

#[test]
fn publishing_the_same_payload_again_only_records_the_name() {
    let (_dir, conn) = database();
    let bytes = fixture("marvin-program-plain.vd4");
    publish_legacy(&conn, "marvin.vd4", &bytes, &payload(&bytes)).unwrap();
    let before = snapshot(&conn);
    let again = publish_legacy(&conn, "renamed.vd4", &bytes, &payload(&bytes)).unwrap();
    assert!(again.skipped);
    assert_eq!(again.programs.len(), 1);
    let after = snapshot(&conn);
    let changed: Vec<_> = before
        .iter()
        .zip(&after)
        .filter(|(b, a)| b != a)
        .map(|(b, a)| (b.0.as_str(), b.1, a.1))
        .collect();
    assert_eq!(changed, [("legacy_source_file", 1, 2)]);
}

#[test]
fn a_failure_part_way_leaves_nothing_behind() {
    let (_dir, conn) = database();
    let bytes = fixture("marvin-program-plain.vd4");
    let sha = sha256_hex(
        &withhold_secret_values(payload(&bytes).bytes())
            .unwrap()
            .bytes,
    );
    // A row another source already owns under an id this file maps to:
    // the insert fails after the blobs and the provenance were written.
    conn.execute(
        "INSERT INTO catalog_section (id, manufacturer_id, source_sha256) VALUES (?1, 'M-1092', 'other')",
        [format!("M-1092_CS-LX{}-51", sha[..8].to_ascii_uppercase())],
    )
    .unwrap();
    let before = snapshot(&conn);
    let error = publish_legacy(&conn, "marvin.vd4", &bytes, &payload(&bytes)).unwrap_err();
    assert!(matches!(error, LegacyPublishError::Store(_)), "{error:?}");
    assert_eq!(snapshot(&conn), before);
}

#[test]
fn a_namespace_owned_by_another_payload_is_refused_by_name() {
    // Two payloads whose digests share the first eight hex digits would map
    // to the same ids. Unlikely (1 in 2^32), but never merged: refused.
    let (_dir, conn) = database();
    let bytes = fixture("marvin-program-plain.vd4");
    let sha = sha256_hex(
        &withhold_secret_values(payload(&bytes).bytes())
            .unwrap()
            .bytes,
    );
    let namespace = format!("LX{}", sha[..8].to_ascii_uppercase());
    conn.execute(
        "INSERT INTO legacy_source (payload_sha256, namespace, member_name, member_kind, charset)
         VALUES ('another-payload', ?1, 'ets.vd_', 'ProductDatabase', 'windows-1252')",
        [&namespace],
    )
    .unwrap();
    let before = snapshot(&conn);
    let error = publish_legacy(&conn, "marvin.vd4", &bytes, &payload(&bytes)).unwrap_err();
    let shown = error.to_string();
    assert!(matches!(error, LegacyPublishError::Legacy(_)), "{error:?}");
    assert!(
        shown.contains(&namespace) && shown.contains("another"),
        "{shown}"
    );
    assert_eq!(snapshot(&conn), before);
}

#[test]
fn a_payload_from_other_file_bytes_is_refused_before_any_write() {
    let (_dir, conn) = database();
    let bytes = fixture("marvin-program-plain.vd4");
    let mut other = bytes.clone();
    other.push(0);
    let before = snapshot(&conn);
    let error = publish_legacy(&conn, "marvin.vd4", &other, &payload(&bytes)).unwrap_err();
    assert!(matches!(error, LegacyPublishError::Legacy(_)), "{error:?}");
    assert_eq!(snapshot(&conn), before);
}

#[test]
fn the_download_path_refuses_a_legacy_program_by_name() {
    let (_dir, conn) = database();
    let bytes = fixture("marvin-program-plain.vd4");
    publish_legacy(&conn, "marvin.vd4", &bytes, &payload(&bytes)).unwrap();
    let id = program_id(&bytes);
    match knx_productdb::code::load_program_code(&conn, &id) {
        Err(knx_productdb::code::CodeError::LegacyProgram { program_id }) => {
            assert_eq!(program_id, id)
        }
        other => panic!("expected CodeError::LegacyProgram, got {other:?}"),
    }
}

#[test]
fn secret_values_are_withheld_from_the_stored_payload() {
    let (_dir, conn) = database();
    let bytes = fixture("marvin-program-plain.vd4");
    let report = publish_legacy(&conn, "marvin.vd4", &bytes, &payload(&bytes)).unwrap();
    let stored = knx_productdb::load_source_file(&conn, &report.payload_sha256)
        .unwrap()
        .expect("the stored payload is keyed by its own digest");
    assert_eq!(sha256_hex(&stored), report.payload_sha256);
    for secret in ["Zaphod42", "Beeblebrox"] {
        assert!(
            !stored.windows(secret.len()).any(|w| w == secret.as_bytes()),
            "{secret} kept in the stored payload"
        );
    }
    // Everything else stays: the stored copy differs only by the value.
    let original = payload(&bytes);
    assert_eq!(
        original.bytes().len() - stored.len(),
        "Zaphod42\r\n\\\\Beeblebrox".len()
    );
    let withheld: Vec<_> = report
        .diagnostics
        .iter()
        .filter(|(kind, _)| kind == "secret-withheld")
        .map(|(_, detail)| detail.as_str())
        .collect();
    assert_eq!(
        withheld,
        ["1 value of device.DEVICE_BCU_PASSWORD was withheld from the stored payload (secret-class column)"]
    );
}
