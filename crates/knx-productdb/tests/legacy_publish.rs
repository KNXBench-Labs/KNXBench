//! Publishing a legacy EX-IM database writes everything in one transaction, or nothing.

use std::collections::BTreeSet;

use knx_core::{GroupAddress, IndividualAddress};
use knx_productdb::code::{load_program_code, CodeError, MemoryPlacement, ParameterPlacement};
use knx_productdb::device_evaluation::evaluate_device;
use knx_productdb::download_plan::plan_memory_download;
use knx_productdb::image::{build_download_image, ImageRequest, Link};
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
        (12, 13, 3)
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
        "P-1012_R-1012",
        "P-1013_R-1013",
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

/// L4: the program's `s19_block` rows are its download code. The plan is
/// built the same way as an XML program's: identity from the program row,
/// machine 5's task segment left out, masked octets not written.
#[test]
fn a_legacy_program_plans_a_download_from_its_s19_blocks() {
    let (_dir, conn) = database();
    let bytes = fixture("marvin-program-plain.vd4");
    publish_legacy(&conn, "marvin.vd4", &bytes, &payload(&bytes)).unwrap();
    let id = program_id(&bytes);
    let code = load_program_code(&conn, &id).unwrap().expect("known");
    assert_eq!(code.mask_version.as_deref(), Some("MV-0701"));
    assert_eq!(
        code.load_procedure_style.as_deref(),
        Some("ProductProcedure")
    );
    assert_eq!(code.load_procedures[0].steps.len(), 20);
    // The union's members share one placement in the parameter segment.
    let segment = format!("{id}_AS-4100");
    let at = |offset, bit_offset| MemoryPlacement {
        code_segment: segment.clone(),
        offset,
        bit_offset,
    };
    assert_eq!(
        code.parameters[&format!("{id}_P-1001")],
        ParameterPlacement::Memory(at(0, 7))
    );
    for member in ["UP-1004", "UP-1005"] {
        assert_eq!(
            code.parameters[&format!("{id}_{member}")],
            ParameterPlacement::UnionMember {
                union: at(2, 0),
                offset: 0,
                bit_offset: 0
            },
            "{member}"
        );
    }

    let request = ImageRequest {
        program_id: id.clone(),
        individual_address: IndividualAddress::new(1, 1, 42).unwrap(),
        values: Default::default(),
        links: vec![Link {
            object: 1,
            group_address: GroupAddress::from_raw(0x0801),
            sending: true,
        }],
        flag_overrides: Default::default(),
    };
    let image = build_download_image(&conn, &request).unwrap();
    let plan = plan_memory_download(&image).unwrap();
    assert_eq!(plan.manufacturer, 0x1092);
    let steps: Vec<String> = plan.steps.iter().map(ToString::to_string).collect();
    assert_eq!(
        steps,
        [
            "connect; check mask and manufacturer",
            "compare property 0/78 with 00 00 00 00 00 07 00 00 00 00",
            "A_Memory_Write 0104h: 14 00 00 00 00 00 00 00 00 00 00 (unload address table)",
            "A_Memory_Write 0104h: 24 00 00 00 00 00 00 00 00 00 00 (unload association table)",
            "A_Memory_Write 0104h: 34 00 00 00 00 00 00 00 00 00 00 (unload application program)",
            "A_Memory_Write 0104h: 11 00 00 00 00 00 00 00 00 00 00 (load address table)",
            "A_Memory_Write 0104h: 13 00 00 40 00 00 09 FF 03 80 00 (segment address table)",
            // 4001h-4002h, the individual address, are masked out (01h in
            // the legacy mask is "written", 00h is not).
            "A_Memory_Write 4000h..4000h, 1 octets",
            "A_Memory_Write 4003h..4008h, 6 octets",
            "A_Memory_Write 0104h: 13 02 00 40 00 00 00 00 00 00 00 (segment address table)",
            "A_Memory_Write 0104h: 12 00 00 00 00 00 00 00 00 00 00 (load completed address table)",
            "A_Memory_Write 0104h: 21 00 00 00 00 00 00 00 00 00 00 (load association table)",
            "A_Memory_Write 0104h: 23 00 00 40 09 00 05 FF 03 80 00 (segment association table)",
            "A_Memory_Write 4009h..400Dh, 5 octets",
            "A_Memory_Write 0104h: 23 02 00 40 09 00 00 00 00 00 00 (segment association table)",
            "A_Memory_Write 0104h: 22 00 00 00 00 00 00 00 00 00 00 (load completed association table)",
            "A_Memory_Write 0104h: 31 00 00 00 00 00 00 00 00 00 00 (load application program)",
            "A_Memory_Write 0104h: 33 00 00 41 00 00 20 FF 03 80 00 (segment application program)",
            "A_Memory_Write 4100h..411Fh, 32 octets",
            // PEI 0, manufacturer 1092h, DEVICE_TYPE 7, PROGRAM_VERSION 22:
            // the program row, not the record's zeros.
            "A_Memory_Write 0104h: 33 02 00 41 00 00 10 92 00 07 16 (segment application program)",
            "A_Memory_Write 0104h: 32 00 00 00 00 00 00 00 00 00 00 (load completed application program)",
            "A_Restart (basic)",
            "disconnect",
        ]
    );
    let association = image.segment(&format!("{id}_AS-4009")).unwrap();
    // One association: group address entry 1 (the first after the device's
    // own address) with object 1.
    assert_eq!(association.octets, [1, 1, 1, 0, 0]);
    let parameters = &image.segment(&segment).unwrap().octets;
    // p_mode on at bit offset 7, the lowest bit (offsets count from the
    // most significant, as ETS's and as the import compares them), p_level
    // 42, the union's Reaction 2, p_governed 5, p_trim 0; then the group
    // object table's header.
    assert_eq!(&parameters[..5], [0x01, 42, 2, 5, 0]);
    assert_eq!(&parameters[8..11], [2, 0x41, 0x30]);
}

#[test]
fn a_payload_that_lost_its_procedure_is_refused_by_row() {
    let (_dir, conn) = database();
    let bytes = fixture("marvin-program-plain.vd4");
    publish_legacy(&conn, "marvin.vd4", &bytes, &payload(&bytes)).unwrap();
    let id = program_id(&bytes);
    let sha: String = conn
        .query_row(
            "SELECT payload_sha256 FROM legacy_program WHERE program_id = ?1",
            [&id],
            |r| r.get(0),
        )
        .unwrap();
    // Damage the stored payload's first segment record in place.
    let stored: Vec<u8> = conn
        .query_row(
            "SELECT bytes FROM source_file WHERE sha256 = ?1",
            [&sha],
            |r| r.get(0),
        )
        .unwrap();
    let record = b"13000040004008ff";
    let at = stored
        .windows(record.len())
        .position(|w| w == record)
        .expect("the record");
    let mut damaged = stored;
    damaged[at + 13] = b'9'; // end 4009h, one past the 9-octet segment
    conn.execute(
        "UPDATE source_file SET bytes = ?1 WHERE sha256 = ?2",
        (damaged, &sha),
    )
    .unwrap();
    match load_program_code(&conn, &id) {
        Err(CodeError::Legacy { program_id, cause }) => {
            assert_eq!(program_id, id);
            assert!(cause.contains("s19_block 907"), "{cause}");
        }
        other => panic!("expected CodeError::Legacy, got {other:?}"),
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
