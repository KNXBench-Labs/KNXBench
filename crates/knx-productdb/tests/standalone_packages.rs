use std::io::{Cursor, Read, Write};
use std::path::PathBuf;

use knx_productdb::{install_package, open_and_migrate, PackageError};
use rusqlite::Connection;
use zip::write::SimpleFileOptions;

const MASTER: &[u8] = br#"<KNX xmlns="http://knx.org/xml/project/11"><MasterData><Manufacturers><Manufacturer Id="M-0001" Name="Example"/></Manufacturers></MasterData></KNX>"#;
const HARDWARE: &[u8] = br#"<KNX xmlns="http://knx.org/xml/project/11"><ManufacturerData><Manufacturer RefId="M-0001"><Hardware><Hardware Id="H-1" Name="Example"><Products><Product Id="P-1" Text="Example"/></Products></Hardware></Hardware></Manufacturer></ManufacturerData></KNX>"#;

fn db() -> (tempfile::TempDir, Connection) {
    let dir = tempfile::tempdir().unwrap();
    let conn = open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
    (dir, conn)
}

#[test]
fn known_single_file_ingest_still_skips_inside_a_callers_transaction() {
    let (_dir, conn) = db();
    knx_productdb::ingest_file(&conn, "M-0001/Hardware.xml", HARDWARE).unwrap();
    let tx = conn.unchecked_transaction().unwrap();
    assert!(matches!(
        knx_productdb::ingest_file(&tx, "M-0001/Hardware.xml", HARDWARE),
        Ok(knx_productdb::IngestOutcome::Skipped { .. })
    ));
}

#[test]
fn a_raw_member_cannot_poison_the_manufacturer_parse_cache() {
    let (_dir, conn) = db();
    let later = String::from_utf8(HARDWARE.to_vec())
        .unwrap()
        .replace("H-1", "H-2")
        .replace("P-1", "P-2");
    let bytes = archive(&[
        ("knx_master.xml", MASTER),
        ("notes.xml", later.as_bytes()),
        ("M-0001/Hardware.xml", HARDWARE),
    ]);
    install_package(&conn, "good.knxprod", &bytes).unwrap();
    knx_productdb::ingest_file(&conn, "M-0001/Hardware-later.xml", later.as_bytes()).unwrap();
    assert_eq!(
        conn.query_row("SELECT count(*) FROM hardware", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        2
    );
}

#[test]
fn retries_keep_conflicts_and_unknown_paths_cannot_supply_parsed_rows() {
    let (_dir, conn) = db();
    install_package(
        &conn,
        "first.knxprod",
        &archive(&[
            ("knx_master.xml", MASTER),
            ("M-0001/Hardware.xml", HARDWARE),
        ]),
    )
    .unwrap();
    let changed = String::from_utf8(HARDWARE.to_vec())
        .unwrap()
        .replace("Example", "Conflicting");
    let arbitrary = String::from_utf8(HARDWARE.to_vec())
        .unwrap()
        .replace("H-1", "H-2")
        .replace("P-1", "P-2");
    let bytes = archive(&[
        ("knx_master.xml", MASTER),
        ("M-0001/Hardware.xml", changed.as_bytes()),
        ("notes.xml", arbitrary.as_bytes()),
        ("M-0001/Baggages/data.xml", arbitrary.as_bytes()),
    ]);
    let first = install_package(&conn, "second.knxprod", &bytes).unwrap();
    assert!(!first.conflicts.is_empty());
    install_package(
        &conn,
        "third.knxprod",
        &archive(&[
            ("knx_master.xml", MASTER),
            ("M-0001/Hardware.xml", changed.as_bytes()),
        ]),
    )
    .unwrap();
    assert_eq!(
        install_package(&conn, "retry.knxprod", &bytes)
            .unwrap()
            .conflicts,
        first.conflicts
    );
    assert_eq!(
        conn.query_row("SELECT count(*) FROM hardware WHERE id = 'H-2'", [], |r| {
            r.get::<_, i64>(0)
        })
        .unwrap(),
        0
    );
}

#[test]
fn manufacturer_partition_and_ref_id_must_agree() {
    let (_dir, conn) = db();
    for path in ["M-anything/Hardware.xml", "M-0002/Hardware.xml"] {
        assert!(install_package(
            &conn,
            "bad.knxprod",
            &archive(&[("knx_master.xml", MASTER), (path, HARDWARE)])
        )
        .is_err());
        assert_eq!(counts(&conn), vec![0; 9]);
    }
    let fake_root = String::from_utf8(HARDWARE.to_vec())
        .unwrap()
        .replace("KNX", "UnknownRoot");
    assert!(install_package(
        &conn,
        "bad.knxprod",
        &archive(&[
            ("knx_master.xml", MASTER),
            ("M-0001/Hardware.xml", fake_root.as_bytes())
        ])
    )
    .is_err());
    assert_eq!(counts(&conn), vec![0; 9]);
}

#[test]
fn excessive_declared_entry_count_is_rejected_before_loading_zip_metadata() {
    let (_dir, conn) = db();
    let mut bytes = archive(&[
        ("knx_master.xml", MASTER),
        ("M-0001/Hardware.xml", HARDWARE),
    ]);
    let eocd = bytes.len() - 22;
    bytes[eocd + 8..eocd + 10].copy_from_slice(&4097_u16.to_le_bytes());
    bytes[eocd + 10..eocd + 12].copy_from_slice(&4097_u16.to_le_bytes());
    let result = install_package(&conn, "bad.knxprod", &bytes);
    assert!(
        matches!(result, Err(PackageError::SizeLimit { .. })),
        "{result:?}"
    );
    assert_eq!(counts(&conn), vec![0; 9]);
}

fn archive(members: &[(&str, &[u8])]) -> Vec<u8> {
    let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for (name, bytes) in members {
        zip.start_file(*name, SimpleFileOptions::default()).unwrap();
        zip.write_all(bytes).unwrap();
    }
    zip.finish().unwrap().into_inner()
}

fn counts(conn: &Connection) -> Vec<i64> {
    [
        "package",
        "package_member",
        "source_file",
        "manufacturer",
        "hardware",
        "product",
        "ingest_unknown",
        "catalog_item",
        "application_program",
    ]
    .iter()
    .map(|table| {
        conn.query_row(&format!("SELECT count(*) FROM {table}"), [], |r| r.get(0))
            .unwrap()
    })
    .collect()
}

#[test]
fn installs_the_readable_corpus() {
    let root = std::env::var_os("KNXBENCH_PRODUCT_CORPUS")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../OriginalData/ProductDatabases")
        });
    if !root.exists() {
        eprintln!("skip: OriginalData/ corpus not present (gitignored, local-only)");
        return;
    }
    for name in [
        "MDT_KP_AMI_AMS_03_Switch_Actuator_V31a.knxprod",
        "Dummy_Applikation_Secure.knxprod",
        "646704-04_ETS4_2012_47_DE_EN.knxprod",
        "Weinzierl_730_KNX_IP_Interface_ETS4.knxprod",
        "Weinzierl_730_KNX_IP_Interface_ETS4_v1.knxprod",
    ] {
        let bytes = std::fs::read(root.join(name)).unwrap_or_else(|e| panic!("corpus fixture {name} unavailable: {e}; set KNXBENCH_PRODUCT_CORPUS to OriginalData/ProductDatabases"));
        let (_dir, conn) = db();
        let report = install_package(&conn, name, &bytes).unwrap();
        assert!(!report.skipped);
        assert!([11, 20].contains(&report.scheme));
        let stored: Vec<u8> = conn
            .query_row(
                "SELECT bytes FROM package WHERE sha256 = ?1",
                [&report.sha256],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(stored, bytes);
        let mut zip = zip::ZipArchive::new(Cursor::new(&bytes)).unwrap();
        let mut member_count = 0;
        for i in 0..zip.len() {
            let mut member = zip.by_index(i).unwrap();
            if member.is_dir() {
                continue;
            }
            member_count += 1;
            let mut raw = Vec::new();
            member.read_to_end(&mut raw).unwrap();
            let (sha, size): (String, i64) = conn.query_row("SELECT source_sha256, size FROM package_member WHERE package_sha256 = ?1 AND path = ?2", [&report.sha256, member.name()], |r| Ok((r.get(0)?, r.get(1)?))).unwrap();
            assert_eq!(size as usize, raw.len());
            assert_eq!(
                knx_productdb::load_source_file(&conn, &sha)
                    .unwrap()
                    .unwrap(),
                raw
            );
        }
        assert_eq!(report.members.len(), member_count);
        assert!(knx_productdb::verify(&conn).unwrap().is_empty());
        assert!(!knx_productdb::query::catalog_items(&conn, None, None)
            .unwrap()
            .is_empty());
        let before = counts(&conn);
        assert!(install_package(&conn, name, &bytes).unwrap().skipped);
        assert_eq!(counts(&conn), before);
    }
}

#[test]
fn rejects_the_real_legacy_vd2_corpus_file_with_its_hash_and_size() {
    let root = std::env::var_os("KNXBENCH_PRODUCT_CORPUS")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../OriginalData/ProductDatabases")
        });
    if !root.exists() {
        eprintln!("skip: OriginalData/ corpus not present (gitignored, local-only)");
        return;
    }
    let name = "Weinzierl_730_KNX_IP_Interface_ETS2-3.vd2";
    let bytes = std::fs::read(root.join(name)).unwrap_or_else(|e| {
        panic!("corpus fixture {name} unavailable: {e}; set KNXBENCH_PRODUCT_CORPUS to OriginalData/ProductDatabases")
    });
    let (_dir, conn) = db();
    let err = install_package(&conn, name, &bytes).unwrap_err();
    let (sha256, len) = match &err {
        PackageError::LegacyVd2 { sha256, len } => (sha256.clone(), *len),
        other => panic!("expected PackageError::LegacyVd2, got {other:?}"),
    };
    assert_eq!(len, bytes.len());
    assert_eq!(sha256.len(), 64);
    assert!(sha256
        .chars()
        .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()));
    assert_eq!(sha256, knx_productdb::sha256_hex(&bytes));
    let rendered = err.to_string();
    assert!(rendered.contains(&sha256), "{rendered}");
    assert!(rendered.contains(&len.to_string()), "{rendered}");
    assert_eq!(counts(&conn), vec![0; 9]);
    eprintln!(
        "rejects_the_real_legacy_vd2_corpus_file_with_its_hash_and_size: sha256={sha256} len={len}"
    );
}

#[test]
fn a_small_vd2_still_reports_hash_and_length() {
    let (_dir, conn) = db();
    let bytes = vec![1u8, 2, 3];
    let err = install_package(&conn, "legacy.vd2", &bytes).unwrap_err();
    match err {
        PackageError::LegacyVd2 { sha256, len } => {
            assert_eq!(len, 3);
            assert_eq!(sha256, knx_productdb::sha256_hex(&bytes));
        }
        other => panic!("expected PackageError::LegacyVd2, got {other:?}"),
    }
}

#[test]
fn failures_preserve_an_existing_install_including_reports() {
    let (_dir, conn) = db();
    let valid = archive(&[
        ("knx_master.xml", MASTER),
        ("M-0001/Hardware.xml", HARDWARE),
    ]);
    let report = install_package(&conn, "valid.knxprod", &valid).unwrap();
    let before = counts(&conn);
    let changed = String::from_utf8(HARDWARE.to_vec())
        .unwrap()
        .replace("Example", "Conflicting");
    let malformed = archive(&[
        ("knx_master.xml", MASTER),
        ("M-0001/Hardware.xml", changed.as_bytes()),
        ("M-0001/new.xml", b"<KNX><Unknown>"),
    ]);
    assert!(install_package(&conn, "bad.knxprod", &malformed).is_err());
    assert_eq!(counts(&conn), before);
    assert_eq!(
        conn.query_row("SELECT name FROM hardware WHERE id = 'H-1'", [], |r| r
            .get::<_, String>(
            0
        ))
        .unwrap(),
        "Example"
    );
    assert_eq!(
        install_package(&conn, "renamed.knxprod", &valid)
            .unwrap()
            .members,
        report.members
    );
}

#[test]
fn master_namespace_must_be_exact_and_xml_complete() {
    let (_dir, conn) = db();
    for master in [
        br#"<KNX xmlns="http://knx.org/xml/project/110"/>"#.as_slice(),
        br#"<KNX xmlns="https://knx.org/xml/project/11"/>"#,
        br#"<KNX xmlns="http://knx.org/xml/project/11"><MasterData>"#,
        br#"<KNX xmlns="http://knx.org/xml/project/11"/><KNX/>"#,
    ] {
        assert!(install_package(
            &conn,
            "bad.knxprod",
            &archive(&[
                ("knx_master.xml", master),
                ("M-0001/Hardware.xml", HARDWARE)
            ])
        )
        .is_err());
        assert_eq!(counts(&conn), vec![0; 9]);
    }
    let prefixed = br#"<k:KNX xmlns:k="http://knx.org/xml/project/20"><k:MasterData/></k:KNX>"#;
    assert_eq!(
        install_package(
            &conn,
            "good.knxprod",
            &archive(&[
                ("knx_master.xml", prefixed),
                ("M-0001/Hardware.xml", HARDWARE)
            ])
        )
        .unwrap()
        .scheme,
        20
    );
}

#[test]
fn validates_unknown_xml_payloads_without_rejecting_valid_references() {
    let (_dir, conn) = db();
    for payload in [
        br#"<![CDATA[bad]]><R/>"#.as_slice(),
        br#"<R bad=>ok</R>"#,
        br#"<R>&undefined;</R>"#,
        br#"&amp;<R/>"#,
        br#"<R>&#0;</R>"#,
        br#"<R>&#+65;</R>"#,
    ] {
        let bytes = archive(&[
            ("knx_master.xml", MASTER),
            ("M-0001/Hardware.xml", HARDWARE),
            ("notes.xml", payload),
        ]);
        assert!(install_package(&conn, "malformed.knxprod", &bytes).is_err());
        assert_eq!(counts(&conn), vec![0; 9]);
    }
    let valid = archive(&[
        ("knx_master.xml", MASTER),
        ("M-0001/Hardware.xml", HARDWARE),
        ("notes.xml", br#"<R>&#65;&#x41;</R>"#),
    ]);
    assert!(install_package(&conn, "valid.knxprod", &valid).is_ok());
}

#[test]
fn malformed_and_unsupported_packages_leave_no_rows() {
    let (_dir, conn) = db();
    let cases = [
        ("legacy.vd2", vec![1, 2, 3], "legacy"),
        ("bad.knxprod", vec![1, 2, 3], "ZIP"),
        (
            "missing.knxprod",
            archive(&[("M-0001/Hardware.xml", HARDWARE)]),
            "master",
        ),
        (
            "unsafe.knxprod",
            archive(&[("knx_master.xml", MASTER), ("../escape", b"x")]),
            "unsafe",
        ),
        (
            "unknown.knxprod",
            archive(&[
                (
                    "knx_master.xml",
                    br#"<KNX xmlns="http://knx.org/xml/project/12"/>"#,
                ),
                ("M-0001/Hardware.xml", HARDWARE),
            ]),
            "namespace",
        ),
        (
            "project.knxprod",
            archive(&[("knx_master.xml", MASTER), ("P-0001/0.xml", b"<KNX/>")]),
            "project",
        ),
    ];
    for (name, bytes, message) in cases {
        let err = install_package(&conn, name, &bytes).unwrap_err();
        assert!(err.to_string().contains(message), "{name}: {err}");
        assert_eq!(counts(&conn), vec![0; 9]);
    }
}

#[test]
fn a_late_xml_failure_rolls_back_every_package_write() {
    let (_dir, conn) = db();
    let bytes = archive(&[
        ("knx_master.xml", MASTER),
        ("M-0001/Hardware.xml", HARDWARE),
        (
            "M-0001/Broken.xml",
            b"<KNX><ManufacturerData><ApplicationPrograms><ApplicationProgram Id=\"broken\">",
        ),
    ]);
    assert!(matches!(
        install_package(&conn, "bad.knxprod", &bytes),
        Err(PackageError::Database(_))
    ));
    assert_eq!(counts(&conn), vec![0; 9]);
}

#[test]
fn unknown_members_are_retained_and_reported() {
    let (_dir, conn) = db();
    let bytes = archive(&[
        ("knx_master.xml", MASTER),
        ("M-0001/Hardware.xml", HARDWARE),
        ("M-0001.signature", b"signature"),
        ("notes.txt", b"abc"),
    ]);
    let report = install_package(&conn, "example.knxprod", &bytes).unwrap();
    let member = report
        .members
        .iter()
        .find(|m| m.path == "notes.txt")
        .unwrap();
    assert_eq!(
        member.sha256,
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
    assert_eq!(member.role, "Unrecognized");
    assert_eq!(member.size, 3);
    assert!(report.unknown > 0);
}

#[test]
fn duplicate_encrypted_truncated_and_oversized_members_are_rejected() {
    let (_dir, conn) = db();
    let valid = archive(&[
        ("knx_master.xml", MASTER),
        ("M-0001/Hardware.xml", HARDWARE),
        ("one.txt", b"x"),
        ("two.txt", b"x"),
    ]);
    let mut duplicate = valid.clone();
    for i in 0..duplicate.len() - 7 {
        if &duplicate[i..i + 7] == b"two.txt" {
            duplicate[i..i + 7].copy_from_slice(b"one.txt");
        }
    }
    let result = install_package(&conn, "bad.knxprod", &duplicate);
    assert!(
        matches!(result, Err(PackageError::DuplicateMember { .. })),
        "{result:?}"
    );
    let mut encrypted = valid.clone();
    let mut oversized = valid.clone();
    for i in 0..valid.len() - 46 {
        if &valid[i..i + 4] == b"PK\x01\x02" {
            encrypted[i + 8] |= 1;
            oversized[i + 24..i + 28].copy_from_slice(&(64_u32 * 1024 * 1024 + 1).to_le_bytes());
        }
        if &valid[i..i + 4] == b"PK\x03\x04" {
            encrypted[i + 6] |= 1;
        }
    }
    assert!(matches!(
        install_package(&conn, "bad.knxprod", &encrypted),
        Err(PackageError::Encrypted { .. })
    ));
    assert!(matches!(
        install_package(&conn, "bad.knxprod", &oversized),
        Err(PackageError::SizeLimit { .. })
    ));
    assert!(install_package(&conn, "bad.knxprod", &valid[..valid.len() / 2]).is_err());
    assert_eq!(counts(&conn), vec![0; 9]);
}

#[test]
fn migrating_v1_preserves_existing_rows_and_blobs() {
    let (dir, conn) = db();
    knx_productdb::ingest_file(&conn, "M-0001/Hardware.xml", HARDWARE).unwrap();
    // `translation` is also rolled back to its pre-Task-1 (v0-v1) shape: `db()`
    // already ran the full chain up to v4, so `translation` already has
    // `scope`/`scope_id`, and rerunning `migrate_v3_to_v4`'s rebuild against a
    // table that is already in its own target shape would fail looking for
    // the `program_id` column it expects to migrate away from.
    conn.execute_batch(
        "DROP TABLE package_conflict; DROP TABLE package_member; DROP TABLE source_parse_evidence;
         DROP TABLE package; DROP TABLE dynamic_node;
         DROP INDEX translation_lookup;
         DROP TABLE translation;
         CREATE TABLE translation (
             program_id     TEXT NOT NULL,
             language       TEXT NOT NULL,
             ref_id         TEXT NOT NULL,
             attribute_name TEXT NOT NULL,
             text           TEXT,
             PRIMARY KEY (program_id, language, ref_id, attribute_name)
         ) STRICT;
         CREATE INDEX translation_lookup ON translation (program_id, language, ref_id);
         PRAGMA user_version = 1;",
    )
    .unwrap();
    drop(conn);
    let conn = open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
    assert_eq!(
        conn.query_row("PRAGMA user_version", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        4
    );
    assert_eq!(
        knx_productdb::load_source_file(&conn, &knx_productdb::sha256_hex(HARDWARE))
            .unwrap()
            .unwrap(),
        HARDWARE
    );
    install_package(
        &conn,
        "example.knxprod",
        &archive(&[
            ("knx_master.xml", MASTER),
            ("M-0001/Hardware.xml", HARDWARE),
        ]),
    )
    .unwrap();
    assert_eq!(
        conn.query_row("SELECT count(*) FROM hardware", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        1
    );
}

#[test]
fn a_failed_v1_to_v2_migration_rolls_back_its_ddl_and_version() {
    let (dir, conn) = db();
    // `translation` is also rolled back to its pre-Task-1 (v0-v1) shape, for
    // the same reason `migrating_v1_preserves_existing_rows_and_blobs` does:
    // `db()` already ran the full chain up to v4, so `migrate_v3_to_v4`'s
    // rebuild must find `program_id` still there to migrate away from.
    conn.execute_batch(
        "DROP TABLE package_conflict; DROP TABLE package_member; DROP TABLE source_parse_evidence;
         DROP TABLE package; DROP TABLE dynamic_node;
         DROP INDEX translation_lookup;
         DROP TABLE translation;
         CREATE TABLE translation (
             program_id     TEXT NOT NULL,
             language       TEXT NOT NULL,
             ref_id         TEXT NOT NULL,
             attribute_name TEXT NOT NULL,
             text           TEXT,
             PRIMARY KEY (program_id, language, ref_id, attribute_name)
         ) STRICT;
         CREATE INDEX translation_lookup ON translation (program_id, language, ref_id);
         CREATE TABLE package_conflict (marker INTEGER); PRAGMA user_version = 1;",
    )
    .unwrap();
    drop(conn);
    assert!(open_and_migrate(&dir.path().join("products.sqlite")).is_err());
    let conn = Connection::open(dir.path().join("products.sqlite")).unwrap();
    assert_eq!(
        conn.query_row("PRAGMA user_version", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        1
    );
    assert!(conn.prepare("SELECT * FROM package").is_err());
    conn.execute_batch("DROP TABLE package_conflict;").unwrap();
    drop(conn);
    assert_eq!(
        open_and_migrate(&dir.path().join("products.sqlite"))
            .unwrap()
            .query_row("PRAGMA user_version", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        4
    );
}
