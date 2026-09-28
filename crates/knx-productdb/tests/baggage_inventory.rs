//! PDB-10: the baggage inventory is typed, resolved, classified, and re-derived on upgrade.
//!
//! ADR-0042. Declarations resolve exactly, payloads are classified by
//! content, the inventory survives retry and tampering is rejected.

mod v16_rewind;

use std::io::{Cursor, Write};

use knx_productdb::{
    install_package, open_and_migrate, InstallCategory, InstallDiagnosticKind, InstallDisposition,
    MediaClass, NestedArchive, Resolution,
};
use rusqlite::Connection;
use zip::write::SimpleFileOptions;

const MASTER: &[u8] = br#"<KNX xmlns="http://knx.org/xml/project/20"><MasterData><Manufacturers><Manufacturer Id="M-0001" Name="Example"/></Manufacturers></MasterData></KNX>"#;
const HARDWARE: &[u8] = br#"<KNX xmlns="http://knx.org/xml/project/20"><ManufacturerData><Manufacturer RefId="M-0001"><Hardware><Hardware Id="H-1" Name="Example"/></Hardware></Manufacturer></ManufacturerData></KNX>"#;

/// Five declarations: two resolve (one of them twice to the same member),
/// one names a missing file, one climbs out with `..`, one has no `Name`.
/// Corpus-shaped attributes, including the numeric `InstallOnImport="0"`.
const INDEX: &[u8] = br#"<?xml version="1.0" encoding="utf-8"?>
<KNX xmlns="http://knx.org/xml/project/20"><ManufacturerData><Manufacturer RefId="M-0001"><Baggages>
<Baggage Id="M-0001_BG-logo" Name="logo.png" TargetPath="Icons" InstallOnImport="true"><FileInfo TimeInfo="2021-03-04T05:06:07.000Z"/></Baggage>
<Baggage Id="M-0001_BG-logo-again" Name="logo.png" TargetPath="Icons"><FileInfo TimeInfo="2021-03-04T05:06:07.000Z" Version="2"/></Baggage>
<Baggage Id="M-0001_BG-tools" Name="tools.zip" TargetPath="" InstallOnImport="0"><FileInfo TimeInfo="2022-01-01T00:00:00Z"/></Baggage>
<Baggage Id="M-0001_BG-gone" Name="gone.pdf" TargetPath="Docs"><FileInfo TimeInfo="2022-01-01T00:00:00Z"/></Baggage>
<Baggage Id="M-0001_BG-escape" Name="x.bin" TargetPath="../outside"><FileInfo TimeInfo="2022-01-01T00:00:00Z"/></Baggage>
<Baggage Id="M-0001_BG-nameless" TargetPath="Icons"><FileInfo TimeInfo="2022-01-01T00:00:00Z"/></Baggage>
</Baggages></Manufacturer></ManufacturerData></KNX>"#;

/// A BMP wearing a `.png` extension, as 35 corpus payloads do.
const BMP: &[u8] = b"BM\x3a\0\0\0\0\0\0\0\x36\0\0\0\x28\0\0\0\x01\0\0\0\x01\0\0\0\x01\0\x18\0\0\0\0\0\x04\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\xff\0";

fn nested_zip() -> Vec<u8> {
    let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let options = SimpleFileOptions::default();
    writer.start_file("readme.txt", options).unwrap();
    writer.write_all(&[b'x'; 300]).unwrap();
    writer.start_file("deeper.zip", options).unwrap();
    writer.write_all(b"PK\x05\x06").unwrap();
    writer.write_all(&[0; 18]).unwrap();
    writer.finish().unwrap().into_inner()
}

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
    let tools = nested_zip();
    archive(&[
        ("knx_master.xml", MASTER),
        ("M-0001/Hardware.xml", HARDWARE),
        ("M-0001/Baggages.xml", INDEX),
        ("M-0001/Baggages/Icons/logo.png", BMP),
        ("M-0001/Baggages/tools.zip", &tools),
        ("M-0001/Baggages/Extra/README", b"undeclared, extensionless"),
    ])
}

fn db() -> (tempfile::TempDir, Connection) {
    let dir = tempfile::tempdir().unwrap();
    let conn = open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
    (dir, conn)
}

#[test]
fn declarations_are_typed_resolved_exactly_and_payloads_classified_by_content() {
    let (_dir, conn) = db();
    let report = install_package(&conn, "baggage.knxprod", &package()).unwrap();
    let inventory = report
        .baggage
        .as_ref()
        .expect("fresh installs are measured");

    let rows: Vec<_> = inventory
        .declarations
        .iter()
        .map(|row| {
            (
                row.declaration.id.as_deref().unwrap(),
                row.resolution,
                row.member_path.as_deref(),
                row.detail.as_deref(),
            )
        })
        .collect();
    assert_eq!(
        rows,
        [
            (
                "M-0001_BG-logo",
                Resolution::Resolved,
                Some("M-0001/Baggages/Icons/logo.png"),
                None
            ),
            (
                "M-0001_BG-logo-again",
                Resolution::Resolved,
                Some("M-0001/Baggages/Icons/logo.png"),
                None
            ),
            (
                "M-0001_BG-tools",
                Resolution::Resolved,
                Some("M-0001/Baggages/tools.zip"),
                None
            ),
            (
                "M-0001_BG-gone",
                Resolution::Missing,
                None,
                Some("no package member at the declared path")
            ),
            (
                "M-0001_BG-escape",
                Resolution::Invalid,
                None,
                Some("TargetPath has an empty, dot or backslashed component")
            ),
            (
                "M-0001_BG-nameless",
                Resolution::Invalid,
                None,
                Some("declaration has no Name")
            ),
        ]
    );
    let first = &inventory.declarations[0].declaration;
    assert_eq!(first.install_on_import.as_deref(), Some("true"));
    assert_eq!(first.time_info.as_deref(), Some("2021-03-04T05:06:07.000Z"));
    assert_eq!(
        inventory.declarations[1]
            .declaration
            .file_version
            .as_deref(),
        Some("2")
    );
    assert_eq!(
        inventory.declarations[2]
            .declaration
            .install_on_import
            .as_deref(),
        Some("0"),
        "the raw lexeme is kept, not coerced to a boolean"
    );

    let payload = |path: &str| {
        inventory
            .payloads
            .iter()
            .find(|payload| payload.member_path == path)
            .unwrap()
    };
    let logo = payload("M-0001/Baggages/Icons/logo.png");
    assert_eq!(
        logo.media_class,
        MediaClass::Bmp,
        "content wins over the .png name"
    );
    assert_eq!(logo.extension_agrees(), Some(false));
    assert_eq!(logo.declarations, 2);
    assert_eq!(logo.size, BMP.len() as u64);
    let tools = payload("M-0001/Baggages/tools.zip");
    assert_eq!(tools.media_class, MediaClass::Zip);
    assert_eq!(
        tools.nested,
        NestedArchive::Read {
            entries: 2,
            declared_expanded_size: 322,
            encrypted_entries: 0,
            archive_named_entries: 1,
        }
    );
    let readme = payload("M-0001/Baggages/Extra/README");
    assert_eq!(readme.media_class, MediaClass::Unknown);
    assert_eq!(readme.declarations, 0);
    assert_eq!(readme.extension(), None);

    let facts = report.facts.as_ref().unwrap();
    let count = |category, disposition| {
        facts
            .counts
            .iter()
            .find(|row| row.category == category && row.disposition == disposition)
            .unwrap()
            .count
    };
    assert_eq!(
        count(InstallCategory::BaggageIndex, InstallDisposition::Read),
        6
    );
    assert_eq!(
        count(InstallCategory::BaggageIndex, InstallDisposition::Stored),
        6
    );
    let diagnostics: Vec<_> = facts
        .diagnostics
        .iter()
        .filter(|row| {
            matches!(
                row.kind(),
                InstallDiagnosticKind::UnresolvedBaggageDeclaration
                    | InstallDiagnosticKind::UndeclaredBaggagePayload
            )
        })
        .map(|row| {
            (
                row.kind(),
                row.archive_path(),
                row.detail(),
                row.occurrences(),
            )
        })
        .collect();
    assert_eq!(
        diagnostics,
        [
            (
                InstallDiagnosticKind::UnresolvedBaggageDeclaration,
                "M-0001/Baggages.xml",
                "baggage declaration does not resolve: TargetPath has an empty, dot or backslashed component",
                1
            ),
            (
                InstallDiagnosticKind::UnresolvedBaggageDeclaration,
                "M-0001/Baggages.xml",
                "baggage declaration does not resolve: declaration has no Name",
                1
            ),
            (
                InstallDiagnosticKind::UnresolvedBaggageDeclaration,
                "M-0001/Baggages.xml",
                "baggage declaration does not resolve: no package member at the declared path",
                1
            ),
            (
                InstallDiagnosticKind::UndeclaredBaggagePayload,
                "M-0001/Baggages/Extra/README",
                "baggage payload is retained but no Baggages.xml declaration names it",
                1
            ),
        ]
    );
    assert!(
        !dir_has_extracted_files(&_dir),
        "nothing is ever written next to the database"
    );
}

fn dir_has_extracted_files(dir: &tempfile::TempDir) -> bool {
    std::fs::read_dir(dir.path())
        .unwrap()
        .filter_map(Result::ok)
        .any(|entry| {
            !entry
                .file_name()
                .to_string_lossy()
                .starts_with("products.sqlite")
        })
}

#[test]
fn retry_returns_the_persisted_inventory_and_index_unknowns_are_reported() {
    let (_dir, conn) = db();
    let with_extra = String::from_utf8(INDEX.to_vec()).unwrap().replace(
        r#"TargetPath="Icons" InstallOnImport="true">"#,
        r#"TargetPath="Icons" InstallOnImport="true" Checksum="abc">"#,
    );
    let bytes = archive(&[
        ("knx_master.xml", MASTER),
        ("M-0001/Hardware.xml", HARDWARE),
        ("M-0001/Baggages.xml", with_extra.as_bytes()),
        ("M-0001/Baggages/Icons/logo.png", BMP),
    ]);
    let fresh = install_package(&conn, "a.knxprod", &bytes).unwrap();
    let retried = install_package(&conn, "b.knxprod", &bytes).unwrap();
    assert!(retried.skipped);
    assert_eq!(retried.baggage, fresh.baggage);
    assert_eq!(retried.facts, fresh.facts);
    let unknown = fresh
        .facts
        .unwrap()
        .unknown_constructs
        .into_iter()
        .find(|row| row.name == "Checksum")
        .expect("an unmodelled index attribute is reported, not dropped");
    assert_eq!(
        unknown.xpath,
        "/KNX/ManufacturerData/Manufacturer/Baggages/Baggage"
    );
    assert_eq!(unknown.sample.as_deref(), Some("abc"));
}

#[test]
fn a_tampered_inventory_is_rejected_on_reload() {
    let corruptions = [
        "UPDATE package_baggage_payload SET media_class = 'png' WHERE member_path LIKE '%logo.png'",
        "UPDATE package_baggage_payload SET declarations = 1 WHERE member_path LIKE '%logo.png'",
        "UPDATE package_baggage_payload SET sha256 = (SELECT sha256 FROM source_file LIMIT 1) WHERE member_path LIKE '%README'",
        "DELETE FROM package_baggage_payload WHERE member_path LIKE '%README'",
        "UPDATE package_baggage_declaration SET resolution = 'missing', member_path = NULL, detail = 'no package member at the declared path' WHERE baggage_id = 'M-0001_BG-tools'",
        "UPDATE package_baggage_declaration SET name = 'other.png' WHERE baggage_id = 'M-0001_BG-logo'",
        "DELETE FROM package_baggage_declaration WHERE baggage_id = 'M-0001_BG-gone'",
        "UPDATE package_baggage_declaration SET ordinal = 9 WHERE baggage_id = 'M-0001_BG-nameless'",
        "UPDATE package_baggage_inventory SET status = 'unavailable'",
        "DELETE FROM package_baggage_declaration; DELETE FROM package_baggage_payload; DELETE FROM package_baggage_inventory",
        "UPDATE package_baggage_payload SET nested_entries = nested_entries + 1 WHERE member_path LIKE '%tools.zip'",
        // Lexemes that do not change a resolution must still match the index.
        "UPDATE package_baggage_declaration SET baggage_id = 'forged' WHERE baggage_id = 'M-0001_BG-logo'",
        "UPDATE package_baggage_declaration SET install_on_import = 'false' WHERE baggage_id = 'M-0001_BG-tools'",
        "UPDATE package_baggage_declaration SET time_info = '1999-01-01T00:00:00Z' WHERE baggage_id = 'M-0001_BG-gone'",
        "UPDATE package_baggage_declaration SET name = 'other.pdf' WHERE baggage_id = 'M-0001_BG-gone'",
        // A payload row pointing at a different retained blob.
        "UPDATE package_baggage_payload SET sha256 = (SELECT source_sha256 FROM package_member WHERE path LIKE '%logo.png') WHERE member_path LIKE '%README'",
    ];
    for (case, sql) in corruptions.iter().enumerate() {
        let (_dir, conn) = db();
        install_package(&conn, "a.knxprod", &package()).unwrap();
        conn.execute_batch("PRAGMA ignore_check_constraints = ON; PRAGMA foreign_keys = OFF;")
            .unwrap();
        conn.execute_batch(sql).unwrap();
        conn.execute_batch("PRAGMA ignore_check_constraints = OFF; PRAGMA foreign_keys = ON;")
            .unwrap();
        let result = install_package(&conn, "b.knxprod", &package());
        assert!(
            result.is_err(),
            "corruption case {case} ({sql}) was accepted"
        );
    }
}

#[test]
fn v15_to_v16_upgrade_measures_exactly_what_a_fresh_install_measures() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("products.sqlite");
    let bytes = package();
    let fresh = {
        let conn = open_and_migrate(&path).unwrap();
        let fresh = install_package(&conn, "baggage.knxprod", &bytes).unwrap();
        v16_rewind::rewind_to_v15(&conn);
        let old: String = conn
            .query_row(
                "SELECT disposition FROM package_install_count WHERE category = 'baggage_index' AND disposition <> 'read'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(old, "unsupported", "rewind produced the v15 shape");
        fresh
    };
    let conn = open_and_migrate(&path).unwrap();
    let retried = install_package(&conn, "retry.knxprod", &bytes).unwrap();
    assert!(retried.skipped);
    assert_eq!(
        retried.baggage, fresh.baggage,
        "inventory re-derived from retained bytes"
    );
    assert_eq!(
        retried.facts, fresh.facts,
        "report rewritten into the fresh v16 shape"
    );
}

#[test]
fn v15_to_v16_upgrade_merges_index_unknowns_like_a_fresh_install() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("products.sqlite");
    let odd_index = String::from_utf8(INDEX.to_vec())
        .unwrap()
        .replace("<FileInfo TimeInfo=\"2022-01-01T00:00:00Z\"/></Baggage>\n<Baggage Id=\"M-0001_BG-gone\"", "<FileInfo TimeInfo=\"2022-01-01T00:00:00Z\"/><Signature/></Baggage>\n<Baggage Id=\"M-0001_BG-gone\"");
    assert_ne!(odd_index.as_bytes(), INDEX);
    let bytes = archive(&[
        ("knx_master.xml", MASTER),
        ("M-0001/Hardware.xml", HARDWARE),
        ("M-0001/Baggages.xml", odd_index.as_bytes()),
        ("M-0001/Baggages/Icons/logo.png", BMP),
    ]);
    let (fresh, fresh_ingest_unknown, fresh_unknown_count) = {
        let conn = open_and_migrate(&path).unwrap();
        let fresh = install_package(&conn, "odd.knxprod", &bytes).unwrap();
        let rows = ingest_unknown_rows(&conn);
        assert!(rows.iter().any(|row| row.2 == "Signature"));
        let count = package_unknown_count(&conn);
        assert_eq!(count, i64::try_from(fresh.unknown).unwrap());
        // v15 had no index parser unknowns: strip them before rewinding,
        // including the one they added to `package.unknown_count`.
        let index_rows = rows
            .iter()
            .filter(|row| {
                row.1
                    .starts_with("/KNX/ManufacturerData/Manufacturer/Baggages")
            })
            .count();
        assert_eq!(index_rows, 1);
        conn.execute(
            "UPDATE package SET unknown_count = unknown_count - ?1",
            [i64::try_from(index_rows).unwrap()],
        )
        .unwrap();
        conn.execute_batch(
            "DELETE FROM ingest_unknown WHERE xpath LIKE '/KNX/ManufacturerData/Manufacturer/Baggages%';
             DELETE FROM package_install_unknown WHERE xpath LIKE '/KNX/ManufacturerData/Manufacturer/Baggages%';
             UPDATE package_install_report SET
                unknown_distinct = (SELECT count(*) FROM package_install_unknown),
                unknown_occurrences = (SELECT coalesce(sum(occurrences), 0) FROM package_install_unknown);
             UPDATE package_install_count SET count = (SELECT coalesce(sum(occurrences), 0) FROM package_install_unknown)
                WHERE category = 'unknown_construct' AND disposition = 'read';
             UPDATE package_install_count SET count = (SELECT count(*) FROM package_install_unknown)
                WHERE category = 'unknown_construct' AND disposition = 'stored';",
        )
        .unwrap();
        v16_rewind::rewind_to_v15(&conn);
        (fresh, rows, count)
    };
    let conn = open_and_migrate(&path).unwrap();
    assert_eq!(ingest_unknown_rows(&conn), fresh_ingest_unknown);
    assert_eq!(
        package_unknown_count(&conn),
        fresh_unknown_count,
        "the upgrade restores the package's unknown count too"
    );
    let retried = install_package(&conn, "retry.knxprod", &bytes).unwrap();
    assert_eq!(retried.facts, fresh.facts);
    assert_eq!(retried.unknown, fresh.unknown);
}

fn package_unknown_count(conn: &Connection) -> i64 {
    conn.query_row("SELECT unknown_count FROM package", [], |r| r.get(0))
        .unwrap()
}

#[test]
fn a_prefixed_attribute_is_foreign_even_when_its_local_name_is_known() {
    let (_dir, conn) = db();
    let prefixed = String::from_utf8(INDEX.to_vec()).unwrap().replace(
        "<Baggage Id=\"M-0001_BG-gone\" Name=\"gone.pdf\"",
        "<Baggage xmlns:x=\"urn:example\" x:Name=\"shadow\" Id=\"M-0001_BG-gone\" Name=\"gone.pdf\"",
    );
    assert_ne!(prefixed.as_bytes(), INDEX);
    let bytes = archive(&[
        ("knx_master.xml", MASTER),
        ("M-0001/Hardware.xml", HARDWARE),
        ("M-0001/Baggages.xml", prefixed.as_bytes()),
    ]);
    let report = install_package(&conn, "prefixed.knxprod", &bytes).unwrap();
    let gone = report
        .baggage
        .as_ref()
        .unwrap()
        .declarations
        .iter()
        .find(|d| d.declaration.id.as_deref() == Some("M-0001_BG-gone"))
        .unwrap();
    assert_eq!(gone.declaration.name.as_deref(), Some("gone.pdf"));
    assert!(ingest_unknown_rows(&conn)
        .iter()
        .any(|row| row.2 == "x:Name" && row.1.ends_with("/Baggages/Baggage")));
}

fn ingest_unknown_rows(conn: &Connection) -> Vec<(String, String, String, i64)> {
    conn.prepare(
        "SELECT source_sha256, xpath, name, occurrences FROM ingest_unknown
         ORDER BY source_sha256, xpath, kind, name",
    )
    .unwrap()
    .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)))
    .unwrap()
    .collect::<Result<_, _>>()
    .unwrap()
}

#[test]
fn v15_to_v16_marks_an_unreadable_index_unavailable_and_keeps_opening() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("products.sqlite");
    let bytes = package();
    let healthy = archive(&[
        ("knx_master.xml", MASTER),
        ("M-0001/Hardware.xml", HARDWARE),
        ("M-0001/Baggages/solo.bin", b"lonely"),
    ]);
    {
        let conn = open_and_migrate(&path).unwrap();
        install_package(&conn, "baggage.knxprod", &bytes).unwrap();
        install_package(&conn, "healthy.knxprod", &healthy).unwrap();
        v16_rewind::rewind_to_v15(&conn);
        conn.execute(
            "UPDATE source_file SET bytes = ?1 WHERE sha256 = ?2",
            rusqlite::params![
                b"<KNX><Baggages></KNX>".as_slice(),
                knx_productdb::sha256_hex(INDEX)
            ],
        )
        .unwrap();
    }
    let conn = open_and_migrate(&path).expect("one bad blob does not lock the database");
    let rotten = install_package(&conn, "retry.knxprod", &bytes).unwrap();
    assert!(rotten.skipped);
    assert_eq!(
        rotten.baggage, None,
        "unmeasurable is unavailable, not an empty inventory"
    );
    assert_eq!(
        rotten.facts, None,
        "and its report cannot claim v16 counts either"
    );
    let recorded: i64 = conn
        .query_row(
            "SELECT count(*) FROM ingest_unknown
             WHERE kind = 'InstallReportBackfillError' AND name = 'baggage_inventory_backfill'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(recorded, 1);
    let other = install_package(&conn, "healthy-retry.knxprod", &healthy).unwrap();
    let inventory = other.baggage.expect("a healthy package stays measured");
    assert_eq!(inventory.undeclared().count(), 1);
    assert!(other.facts.is_some());
}

#[test]
fn v15_to_v16_downgrades_a_corrupt_report_instead_of_refusing_to_open() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("products.sqlite");
    let bytes = package();
    let healthy = archive(&[
        ("knx_master.xml", MASTER),
        ("M-0001/Hardware.xml", HARDWARE),
        ("M-0001/Baggages/solo.bin", b"lonely"),
    ]);
    {
        let conn = open_and_migrate(&path).unwrap();
        install_package(&conn, "baggage.knxprod", &bytes).unwrap();
        install_package(&conn, "healthy.knxprod", &healthy).unwrap();
        v16_rewind::rewind_to_v15(&conn);
        // The report's own member count no longer matches its members.
        conn.execute(
            "UPDATE package_install_count SET count = count + 1
             WHERE category = 'archive_member' AND disposition = 'read'
               AND package_sha256 = ?1",
            [knx_productdb::sha256_hex(&bytes)],
        )
        .unwrap();
    }
    let conn = open_and_migrate(&path).expect("one corrupt report does not lock the database");
    let rotten = install_package(&conn, "retry.knxprod", &bytes).unwrap();
    assert!(rotten.skipped);
    assert_eq!(rotten.facts, None, "the corrupt report is unavailable");
    assert_eq!(rotten.baggage, None);
    let recorded: i64 = conn
        .query_row(
            "SELECT count(*) FROM ingest_unknown
             WHERE kind = 'InstallReportBackfillError' AND name = 'baggage_inventory_backfill'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(recorded, 1);
    let other = install_package(&conn, "healthy-retry.knxprod", &healthy).unwrap();
    assert!(other.facts.is_some() && other.baggage.is_some());
}

#[test]
fn a_payload_row_swapped_to_a_same_shaped_blob_is_rejected_on_reload() {
    // Two payloads that measure identically (same size, same class): only
    // the member-to-blob binding (checked by the loader) tells them apart.
    let twins = archive(&[
        ("knx_master.xml", MASTER),
        ("M-0001/Hardware.xml", HARDWARE),
        ("M-0001/Baggages/a.txt", b"left twin"),
        ("M-0001/Baggages/b.txt", b"rightwin!"),
    ]);
    let (_dir, conn) = db();
    install_package(&conn, "twins.knxprod", &twins).unwrap();
    conn.execute_batch("PRAGMA foreign_keys = OFF;").unwrap();
    conn.execute(
        "UPDATE package_baggage_payload SET sha256 = ?1 WHERE member_path LIKE '%b.txt'",
        [knx_productdb::sha256_hex(b"left twin")],
    )
    .unwrap();
    conn.execute_batch("PRAGMA foreign_keys = ON;").unwrap();
    assert!(install_package(&conn, "again.knxprod", &twins).is_err());
}
