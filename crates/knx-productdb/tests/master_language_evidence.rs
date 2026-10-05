//! Tests consumed versus retained and uninterpreted master-language metadata.
//!
//! Synthetic offline packages prove
//! exact unknown rows, original-byte retention and immutable install-time
//! reports across reopen and retry; no manufacturer corpus or bus is needed.

mod v20_rewind;

use std::io::{Cursor, Write};

use knx_productdb::report::UnknownKind;
use knx_productdb::{ingest_master_data, install_package, open_and_migrate, sha256_hex};
use zip::write::SimpleFileOptions;

const MASTER: &[u8] = br#"<KNX xmlns="http://knx.org/xml/project/11"><MasterData>
<Manufacturers><Manufacturer Id="M-0001" Name="Synthetic"/></Manufacturers>
<Languages ExtraWrapper="wrapper">
 <Language Identifier="en-US" ExtraLanguage="language">
  <TranslationUnit RefId="unit-not-interpreted" Version="opaque-version">
   <TranslationElement RefId="M-0001" ExtraElement="element">
    <Translation AttributeName="Name" Text="Synthetic translated" ExtraTranslation="translation"/>
   </TranslationElement>
  </TranslationUnit>
 </Language>
</Languages>
</MasterData></KNX>"#;

const HARDWARE: &[u8] = br#"<KNX xmlns="http://knx.org/xml/project/11"><ManufacturerData>
<Manufacturer RefId="M-0001"><Hardware><Hardware Id="H-1" Name="Synthetic"/></Hardware></Manufacturer>
</ManufacturerData></KNX>"#;

fn package(master: &[u8]) -> Vec<u8> {
    let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for (path, bytes) in [
        ("knx_master.xml", master),
        ("M-0001/Hardware.xml", HARDWARE),
    ] {
        writer
            .start_file(path, SimpleFileOptions::default())
            .unwrap();
        writer.write_all(bytes).unwrap();
    }
    writer.finish().unwrap().into_inner()
}

fn expected_unknowns() -> Vec<(String, String, u32, Option<String>)> {
    let root = "/KNX/MasterData/Languages";
    let language = format!("{root}/Language");
    let unit = format!("{language}/TranslationUnit");
    let element = format!("{unit}/TranslationElement");
    let translation = format!("{element}/Translation");
    let mut expected = vec![
        (
            root.to_string(),
            "ExtraWrapper".to_string(),
            1,
            Some("wrapper".to_string()),
        ),
        (
            language,
            "ExtraLanguage".to_string(),
            1,
            Some("language".to_string()),
        ),
        (
            unit.clone(),
            "RefId".to_string(),
            1,
            Some("unit-not-interpreted".to_string()),
        ),
        (
            unit,
            "Version".to_string(),
            1,
            Some("opaque-version".to_string()),
        ),
        (
            element,
            "ExtraElement".to_string(),
            1,
            Some("element".to_string()),
        ),
        (
            translation,
            "ExtraTranslation".to_string(),
            1,
            Some("translation".to_string()),
        ),
    ];
    expected.sort();
    expected
}

#[test]
fn master_language_attributes_are_reported_unless_their_values_are_consumed() {
    let directory = tempfile::tempdir().unwrap();
    let conn = open_and_migrate(&directory.path().join("products.sqlite")).unwrap();
    let result = ingest_master_data(&conn, MASTER).unwrap();
    let mut actual = result
        .unknown
        .iter()
        .map(|item| {
            assert_eq!(item.kind, UnknownKind::Attribute);
            (
                item.xpath.clone(),
                item.name.clone(),
                item.occurrences,
                item.sample.clone(),
            )
        })
        .collect::<Vec<_>>();
    actual.sort();
    assert_eq!(actual, expected_unknowns());
    assert_eq!(result.translations, 1);
    let row = conn
        .query_row(
            "SELECT language, ref_id, attribute_name, text FROM translation WHERE scope='Master'",
            [],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                ))
            },
        )
        .unwrap();
    assert_eq!(
        row,
        (
            "en-US".to_string(),
            "M-0001".to_string(),
            "Name".to_string(),
            "Synthetic translated".to_string()
        )
    );
}

#[test]
fn master_language_unknowns_survive_package_reopen_and_retry_without_remeasurement() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("products.sqlite");
    let bytes = package(MASTER);
    let first = {
        let conn = open_and_migrate(&path).unwrap();
        let report = install_package(&conn, "synthetic.knxprod", &bytes).unwrap();
        let facts = report.facts.as_ref().unwrap();
        assert_eq!(facts.unknown_occurrences, 6);
        let mut actual = facts
            .unknown_constructs
            .iter()
            .map(|item| {
                assert_eq!(item.kind, UnknownKind::Attribute);
                (
                    item.xpath.clone(),
                    item.name.clone(),
                    item.occurrences,
                    item.sample.clone(),
                )
            })
            .collect::<Vec<_>>();
        actual.sort();
        assert_eq!(actual, expected_unknowns());
        let stored: Vec<u8> = conn
            .query_row(
                "SELECT bytes FROM source_file WHERE sha256=?1",
                [sha256_hex(MASTER)],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(stored, MASTER);
        let persisted: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM ingest_unknown WHERE source_sha256=?1",
                [sha256_hex(MASTER)],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(persisted, 6);
        report
    };
    let conn = open_and_migrate(&path).unwrap();
    let retry = install_package(&conn, "renamed-synthetic.knxprod", &bytes).unwrap();
    assert!(retry.skipped);
    assert_eq!(retry.facts, first.facts);
    assert_eq!(retry.translations, first.translations);
    let persisted: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM ingest_unknown WHERE source_sha256=?1",
            [sha256_hex(MASTER)],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(persisted, 6);
}

#[test]
fn repeated_master_language_unknowns_count_occurrences_at_each_exact_owner_path() {
    let master = br#"<KNX xmlns="http://knx.org/xml/project/11"><MasterData><Languages>
    <Language Identifier="en-US"><TranslationUnit Version="first"/><TranslationUnit Version="second"/></Language>
    <Language Identifier="de-DE"><TranslationUnit Version="third"/></Language>
    </Languages></MasterData></KNX>"#;
    let directory = tempfile::tempdir().unwrap();
    let conn = open_and_migrate(&directory.path().join("products.sqlite")).unwrap();
    let result = ingest_master_data(&conn, master).unwrap();
    assert_eq!(result.unknown.len(), 1);
    let item = &result.unknown[0];
    assert_eq!(item.kind, UnknownKind::Attribute);
    assert_eq!(
        item.xpath,
        "/KNX/MasterData/Languages/Language/TranslationUnit"
    );
    assert_eq!(item.name, "Version");
    assert_eq!(item.occurrences, 3);
    assert_eq!(item.sample.as_deref(), Some("first"));
}

#[test]
fn qualified_and_unqualified_metadata_keep_separate_namespace_keys_and_samples() {
    let master =
        br#"<k:KNX xmlns:k="http://knx.org/xml/project/11" xmlns:x="urn:one" xmlns:y="urn:two">
    <k:MasterData><k:Languages><k:Language Identifier="en-US">
    <k:TranslationUnit Version="plain" x:Version="first namespace" y:Version="second namespace"/>
    </k:Language></k:Languages></k:MasterData></k:KNX>"#;
    let directory = tempfile::tempdir().unwrap();
    let conn = open_and_migrate(&directory.path().join("products.sqlite")).unwrap();
    let result = ingest_master_data(&conn, master).unwrap();
    let actual = result
        .unknown
        .iter()
        .map(|item| {
            assert_eq!(item.kind, UnknownKind::Attribute);
            assert_eq!(
                item.xpath,
                "/KNX/MasterData/Languages/Language/TranslationUnit"
            );
            assert_eq!(item.occurrences, 1);
            (item.name.clone(), item.sample.clone())
        })
        .collect::<std::collections::BTreeMap<_, _>>();
    assert_eq!(
        actual,
        std::collections::BTreeMap::from([
            ("Version".into(), Some("plain".into())),
            ("{urn:one}Version".into(), Some("first namespace".into())),
            ("{urn:two}Version".into(), Some("second namespace".into())),
        ])
    );
}

#[test]
fn a_foreign_language_ancestor_does_not_make_its_attributes_consumed_fields() {
    let master = br#"<KNX xmlns="http://knx.org/xml/project/11" xmlns:x="urn:foreign"><MasterData>
    <x:Languages><x:Language Identifier="foreign"/></x:Languages>
    <Languages><Language Identifier="canonical"><TranslationUnit Version="canonical metadata"/></Language></Languages>
    </MasterData></KNX>"#;
    let directory = tempfile::tempdir().unwrap();
    let conn = open_and_migrate(&directory.path().join("products.sqlite")).unwrap();
    let result = ingest_master_data(&conn, master).unwrap();
    assert_eq!(result.unknown.len(), 2);
    let actual = result
        .unknown
        .iter()
        .map(|item| (item.xpath.clone(), item.name.clone(), item.sample.clone()))
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(
        actual,
        std::collections::BTreeSet::from([
            (
                "/KNX/MasterData/x:Languages/x:Language".into(),
                "Identifier".into(),
                Some("foreign".into())
            ),
            (
                "/KNX/MasterData/Languages/Language/TranslationUnit".into(),
                "Version".into(),
                Some("canonical metadata".into())
            ),
        ])
    );
}

fn table_counts(conn: &knx_productdb::Connection) -> std::collections::BTreeMap<String, i64> {
    let tables = conn.prepare("SELECT name FROM sqlite_schema WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name").unwrap()
        .query_map([],|row|row.get::<_,String>(0)).unwrap().collect::<Result<Vec<_>,_>>().unwrap();
    tables
        .into_iter()
        .map(|table| {
            let count = conn
                .query_row(
                    &format!("SELECT COUNT(*) FROM \"{}\"", table.replace('"', "\"\"")),
                    [],
                    |row| row.get(0),
                )
                .unwrap();
            (table, count)
        })
        .collect()
}

#[test]
fn master_language_depth_refusal_leaves_every_package_table_unchanged() {
    let directory = tempfile::tempdir().unwrap();
    let conn = open_and_migrate(&directory.path().join("products.sqlite")).unwrap();
    let before = table_counts(&conn);
    let mut master =
        String::from("<KNX xmlns=\"http://knx.org/xml/project/11\"><MasterData><Languages>");
    master.push_str(&"<Extra>".repeat(1024));
    master.push_str(&"</Extra>".repeat(1024));
    master.push_str("</Languages></MasterData></KNX>");
    let error =
        install_package(&conn, "synthetic-deep.knxprod", &package(master.as_bytes())).unwrap_err();
    assert!(error
        .to_string()
        .contains("master-language evidence depth/item budget exceeded"));
    assert_eq!(table_counts(&conn), before);
}

#[test]
fn a_valid_late_hardware_write_failure_rolls_back_master_language_evidence() {
    let directory = tempfile::tempdir().unwrap();
    let conn = open_and_migrate(&directory.path().join("products.sqlite")).unwrap();
    conn.execute_batch("CREATE TRIGGER refuse_synthetic_hardware BEFORE INSERT ON hardware BEGIN SELECT RAISE(ABORT,'synthetic late hardware failure'); END;").unwrap();
    let before = table_counts(&conn);
    let error = install_package(&conn, "synthetic-late.knxprod", &package(MASTER)).unwrap_err();
    assert!(error
        .to_string()
        .contains("synthetic late hardware failure"));
    assert_eq!(table_counts(&conn), before);
}

fn restore_v18_language_omission(conn: &knx_productdb::Connection) {
    // This fixture has only the formerly omitted Languages attributes:
    // reproduce a measured-zero snapshot, not an unavailable one.
    v20_rewind::drop_v20_objects(conn);
    conn.execute_batch(
        "DELETE FROM ingest_unknown;
         DELETE FROM package_install_unknown;
         UPDATE package_install_report SET unknown_distinct=0, unknown_occurrences=0;
         UPDATE package_install_count SET count=0 WHERE category='unknown_construct';
         UPDATE package SET unknown_count=0;
         PRAGMA user_version=18;",
    )
    .unwrap();
}

#[test]
fn v18_upgrade_rederives_source_fields_without_rewriting_the_measured_zero_install() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("products.sqlite");
    let bytes = package(MASTER);
    let historical = {
        let conn = open_and_migrate(&path).unwrap();
        install_package(&conn, "synthetic.knxprod", &bytes).unwrap();
        restore_v18_language_omission(&conn);
        conn.execute("INSERT INTO ingest_unknown (source_sha256, xpath, kind, name, occurrences, sample) VALUES (?1,'/Foreign/Languages/Language/TranslationUnit','Attribute','Version',9,'preserve me')",[sha256_hex(MASTER)]).unwrap();
        let report = install_package(&conn, "synthetic.knxprod", &bytes).unwrap();
        assert_eq!(report.facts.as_ref().unwrap().unknown_occurrences, 0);
        report
    };
    let conn = open_and_migrate(&path).unwrap();
    let count: i64 = conn.query_row("SELECT COUNT(*) FROM ingest_unknown WHERE source_sha256=?1 AND xpath LIKE '/KNX/MasterData/Languages%'",[sha256_hex(MASTER)],|row|row.get(0)).unwrap();
    assert_eq!(count, 6, "upgrade must add byte-derived source evidence");
    let sentinel: (i64,String) = conn.query_row("SELECT occurrences,sample FROM ingest_unknown WHERE xpath='/Foreign/Languages/Language/TranslationUnit'",[],|row|Ok((row.get(0)?,row.get(1)?))).unwrap();
    assert_eq!(sentinel, (9, "preserve me".into()));
    let retry = install_package(&conn, "synthetic.knxprod", &bytes).unwrap();
    assert_eq!(retry.facts, historical.facts);
    assert_eq!(retry.unknown, historical.unknown);
    assert_eq!(retry.translations, historical.translations);
    // The AR05 Languages rederivation is v19; v20 (ADR-0080) keeps it.
    assert_eq!(knx_productdb::CURRENT_PRODUCTDB_VERSION, 20);
}

#[test]
fn v18_upgrade_keeps_a_historical_unavailable_report_unavailable() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("products.sqlite");
    let bytes = package(MASTER);
    {
        let conn = open_and_migrate(&path).unwrap();
        install_package(&conn, "synthetic.knxprod", &bytes).unwrap();
        restore_v18_language_omission(&conn);
        conn.execute_batch("DELETE FROM package_install_count; DELETE FROM package_install_unknown; DELETE FROM package_install_diagnostic; UPDATE package_install_report SET status='unavailable';").unwrap();
    }
    let conn = open_and_migrate(&path).unwrap();
    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM ingest_unknown WHERE source_sha256=?1",
            [sha256_hex(MASTER)],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(count, 6);
    assert!(install_package(&conn, "synthetic.knxprod", &bytes)
        .unwrap()
        .facts
        .is_none());
}
