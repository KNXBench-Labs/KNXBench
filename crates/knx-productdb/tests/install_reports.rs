//! PDB-3 install evidence is measured at encounter/write time and survives retry.
use std::io::{Cursor, Write};

use knx_productdb::{
    install_package, open_and_migrate, InstallCategory, InstallDiagnosticKind, InstallDisposition,
    InstallFacts,
};
use rusqlite::Connection;
use zip::write::SimpleFileOptions;

const MASTER: &[u8] = br#"<KNX xmlns="http://knx.org/xml/project/11"><MasterData><Manufacturers><Manufacturer Id="M-0001" Name="Example"/></Manufacturers></MasterData></KNX>"#;
const HARDWARE: &[u8] = br#"<KNX xmlns="http://knx.org/xml/project/11"><ManufacturerData><Manufacturer RefId="M-0001"><Hardware><Hardware Id="H-1" Name="Example"/></Hardware></Manufacturer></ManufacturerData></KNX>"#;
const BAGGAGES: &[u8] = br#"<KNX xmlns="http://knx.org/xml/project/11"><ManufacturerData><Manufacturer RefId="M-0001"><Baggages/></Manufacturer></ManufacturerData></KNX>"#;

const FULL_MASTER: &[u8] = br#"<KNX xmlns="http://knx.org/xml/project/11"><MasterData>
<Manufacturers><Manufacturer Id="M-0001" Name="Example"/></Manufacturers>
<DatapointTypes><DatapointType Id="DPT-1" Number="1" Name="one" Text="one"><DatapointSubtypes><DatapointSubtype Id="DPST-1-1" Number="1" Name="switch" Text="switch"/></DatapointSubtypes></DatapointType></DatapointTypes>
<Languages><Language Identifier="en-US"/></Languages>
<MediumTypes/><MaskVersions/><MaskVersions/>
</MasterData></KNX>"#;
const FULL_HARDWARE: &[u8] = br#"<KNX xmlns="http://knx.org/xml/project/11"><ManufacturerData><Manufacturer RefId="M-0001"><Hardware><Hardware Id="H-1" Name="Example"><Products><Product Id="P-1" Text="Product" OrderNumber="1"/></Products></Hardware></Hardware></Manufacturer></ManufacturerData></KNX>"#;
const FULL_PROGRAM: &[u8] = br#"<KNX xmlns="http://knx.org/xml/project/11"><ManufacturerData><Manufacturer RefId="M-0001"><ApplicationPrograms><ApplicationProgram Id="A-1" Name="Program">
<Static><Parameters><Parameter Id="PAR-1"/><Parameter Id="PAR-2"/></Parameters><ComObjectTable><ComObject Id="O-1" Number="1"/></ComObjectTable><ComObjectRefs><ComObjectRef Id="OR-1" RefId="O-1"/></ComObjectRefs><Mystery Foo="1"/><Mystery Foo="2"/></Static>
<Dynamic><Channel Id="CH-1"/></Dynamic>
<ModuleDefs><ModuleDef Id="MD-1" Name="Module"><Dynamic><ParameterBlock Id="PB-1"/></Dynamic></ModuleDef></ModuleDefs>
</ApplicationProgram></ApplicationPrograms></Manufacturer></ManufacturerData></KNX>"#;
const FULL_BAGGAGES: &[u8] = br#"<KNX xmlns="http://knx.org/xml/project/11"><ManufacturerData><Manufacturer RefId="M-0001"><Baggages><Baggage TargetPath="Baggages/a.bin"/><Baggage TargetPath="Baggages/b.bin"/></Baggages></Manufacturer></ManufacturerData></KNX>"#;

fn full_archive(payload_path: &str, payload: &[u8]) -> Vec<u8> {
    archive(&[
        ("knx_master.xml", FULL_MASTER),
        ("M-0001/Baggages.xml", FULL_BAGGAGES),
        (payload_path, payload),
        ("M-0001/Hardware.xml", FULL_HARDWARE),
        ("M-0001/A.xml", FULL_PROGRAM),
    ])
}

fn fact_count(
    facts: &InstallFacts,
    category: InstallCategory,
    disposition: InstallDisposition,
) -> u64 {
    facts
        .counts
        .iter()
        .find(|row| row.category == category && row.disposition == disposition)
        .unwrap_or_else(|| panic!("missing {category:?}/{disposition:?}"))
        .count
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

#[test]
fn install_report_is_sorted_and_retry_returns_the_persisted_facts() {
    let dir = tempfile::tempdir().unwrap();
    let conn = open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
    let bytes = archive(&[
        ("knx_master.xml", MASTER),
        ("M-0001/Baggages.xml", BAGGAGES),
        ("M-0001/Baggages/vendor.bin", b"opaque"),
        ("M-0001/Hardware.xml", HARDWARE),
    ]);
    let first = install_package(&conn, "sample.knxprod", &bytes).unwrap();
    let facts = first.facts.clone().unwrap();
    assert!(facts
        .counts
        .windows(2)
        .all(|w| (&w[0].category, &w[0].disposition) <= (&w[1].category, &w[1].disposition)));
    assert!(facts.counts.iter().any(|x| {
        x.category == knx_productdb::package::InstallCategory::Baggage
            && x.disposition == knx_productdb::package::InstallDisposition::RetainedButUninterpreted
    }));
    assert!(facts.counts.iter().any(|x| {
        x.category == knx_productdb::package::InstallCategory::Baggage
            && x.disposition == knx_productdb::package::InstallDisposition::Stored
    }));

    let retry = install_package(&conn, "renamed.knxprod", &bytes).unwrap();
    assert!(retry.skipped);
    assert_eq!(retry.facts, Some(facts));
}

#[test]
fn baggage_xml_and_baggage_members_are_not_misclassified() {
    let dir = tempfile::tempdir().unwrap();
    let conn = open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
    let report = install_package(
        &conn,
        "sample.knxprod",
        &archive(&[
            ("knx_master.xml", MASTER),
            ("M-0001/Baggages.xml", BAGGAGES),
            ("M-0001/Baggages/vendor.bin", b"opaque"),
            ("M-0001/Hardware.xml", HARDWARE),
        ]),
    )
    .unwrap();
    assert_eq!(
        report
            .members
            .iter()
            .find(|m| m.path.ends_with("Baggages.xml"))
            .unwrap()
            .role,
        "Baggages"
    );
    assert_eq!(
        report
            .members
            .iter()
            .find(|m| m.path.ends_with("vendor.bin"))
            .unwrap()
            .role,
        "Baggage"
    );
}

#[test]
fn persisted_report_corruption_is_rejected_without_mutating_the_database() {
    let dir = tempfile::tempdir().unwrap();
    let conn = open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
    let bytes = archive(&[
        ("knx_master.xml", MASTER),
        ("M-0001/Hardware.xml", HARDWARE),
    ]);
    let first = install_package(&conn, "sample.knxprod", &bytes).unwrap();
    conn.execute_batch("PRAGMA ignore_check_constraints = ON;")
        .unwrap();
    conn.execute(
        "UPDATE package_install_report SET report_version = 99 WHERE package_sha256 = ?1",
        [&first.sha256],
    )
    .unwrap();
    conn.execute_batch("PRAGMA ignore_check_constraints = OFF;")
        .unwrap();
    assert!(install_package(&conn, "retry.knxprod", &bytes).is_err());
    let version: i64 = conn
        .query_row("PRAGMA user_version", [], |r| r.get(0))
        .unwrap();
    assert_eq!(version, knx_productdb::CURRENT_PRODUCTDB_VERSION);
}

#[test]
fn v11_to_v12_collision_rolls_back_and_preserves_version_and_table() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("products.sqlite");
    drop(open_and_migrate(&path).unwrap());
    let collision_sql = {
        let conn = Connection::open(&path).unwrap();
        conn.pragma_update(None, "foreign_keys", "ON").unwrap();
        conn.pragma_update(None, "user_version", 11).unwrap();
        conn.query_row(
            "SELECT sql FROM sqlite_master WHERE type = 'table' AND name = 'package_install_report'",
            [],
            |row| row.get::<_, String>(0),
        )
        .unwrap()
    };

    assert!(open_and_migrate(&path).is_err());
    let conn = Connection::open(&path).unwrap();
    let version: i64 = conn
        .query_row("PRAGMA user_version", [], |row| row.get(0))
        .unwrap();
    assert_eq!(version, 11);
    let table_count: i64 = conn
        .query_row(
            "SELECT count(*) FROM sqlite_master WHERE type = 'table' AND name = 'package_install_report'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(table_count, 1);
    let after_sql: String = conn
        .query_row(
            "SELECT sql FROM sqlite_master WHERE type = 'table' AND name = 'package_install_report'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(after_sql, collision_sql);
}

#[test]
fn retained_alias_is_rejected_on_reload() {
    let dir = tempfile::tempdir().unwrap();
    let conn = open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
    let bytes = archive(&[
        ("knx_master.xml", MASTER),
        ("M-0001/Baggages.xml", BAGGAGES),
        ("M-0001/Baggages/vendor.bin", b"opaque"),
        ("M-0001/Hardware.xml", HARDWARE),
    ]);
    let first = install_package(&conn, "sample.knxprod", &bytes).unwrap();
    conn.execute_batch("PRAGMA ignore_check_constraints = ON;")
        .unwrap();
    conn.execute(
        "UPDATE package_install_count SET disposition = 'retained' \
         WHERE package_sha256 = ?1 AND disposition = 'retained-but-uninterpreted'",
        [&first.sha256],
    )
    .unwrap();
    conn.execute_batch("PRAGMA ignore_check_constraints = OFF;")
        .unwrap();

    assert!(install_package(&conn, "retry.knxprod", &bytes).is_err());
}

#[test]
fn measured_facts_match_parser_owned_rows_and_baggage_boundaries() {
    let dir = tempfile::tempdir().unwrap();
    let conn = open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
    let report = install_package(
        &conn,
        "synthetic.knxprod",
        &full_archive("M-0001/Baggages/vendor.bin", b"opaque\0payload"),
    )
    .unwrap();
    let facts = report.facts.as_ref().unwrap();

    let expected = [
        (InstallCategory::ArchiveMember, InstallDisposition::Read, 5),
        (
            InstallCategory::ArchiveMember,
            InstallDisposition::Stored,
            5,
        ),
        (InstallCategory::Product, InstallDisposition::Read, 1),
        (InstallCategory::Product, InstallDisposition::Stored, 1),
        (
            InstallCategory::ApplicationProgram,
            InstallDisposition::Read,
            1,
        ),
        (
            InstallCategory::ApplicationProgram,
            InstallDisposition::Stored,
            1,
        ),
        (InstallCategory::Parameter, InstallDisposition::Read, 2),
        (InstallCategory::Parameter, InstallDisposition::Stored, 2),
        (
            InstallCategory::CommunicationObject,
            InstallDisposition::Read,
            2,
        ),
        (
            InstallCategory::CommunicationObject,
            InstallDisposition::Stored,
            2,
        ),
        (InstallCategory::DynamicNode, InstallDisposition::Read, 4),
        (InstallCategory::DynamicNode, InstallDisposition::Stored, 4),
        (InstallCategory::Module, InstallDisposition::Read, 1),
        (InstallCategory::DatapointType, InstallDisposition::Read, 2),
        (
            InstallCategory::DatapointType,
            InstallDisposition::Stored,
            2,
        ),
        (InstallCategory::BaggageIndex, InstallDisposition::Read, 2),
        (
            InstallCategory::BaggageIndex,
            InstallDisposition::Unsupported,
            2,
        ),
        (InstallCategory::Baggage, InstallDisposition::Read, 1),
        (InstallCategory::Baggage, InstallDisposition::Stored, 1),
        (
            InstallCategory::Baggage,
            InstallDisposition::RetainedButUninterpreted,
            1,
        ),
        (InstallCategory::MasterSection, InstallDisposition::Read, 6),
        (
            InstallCategory::MasterSection,
            InstallDisposition::Unsupported,
            3,
        ),
    ];
    for (category, disposition, count) in expected {
        assert_eq!(fact_count(facts, category, disposition), count);
    }
    for (table, rows) in [
        ("product", 1),
        ("application_program", 1),
        ("parameter", 2),
        ("com_object", 1),
        ("com_object_ref", 1),
        ("dynamic_node", 4),
        ("datapoint_type", 2),
    ] {
        let actual: i64 = conn
            .query_row(&format!("SELECT count(*) FROM {table}"), [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(actual, rows, "{table}");
    }
    assert_eq!(
        conn.query_row(
            "SELECT bytes FROM source_file WHERE source_path LIKE '%vendor.bin'",
            [],
            |row| row.get::<_, Vec<u8>>(0)
        )
        .unwrap(),
        b"opaque\0payload"
    );
    assert!(facts.counts.iter().all(|row| {
        row.category != InstallCategory::Module || row.disposition == InstallDisposition::Read
    }));
}

#[test]
fn real_identity_dedup_does_not_label_parent_rejected_children_deduplicated() {
    let dir = tempfile::tempdir().unwrap();
    let conn = open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
    install_package(
        &conn,
        "first.knxprod",
        &full_archive("M-0001/Baggages/first.bin", b"first opaque"),
    )
    .unwrap();
    let second = install_package(
        &conn,
        "second.knxprod",
        &full_archive("M-0001/Baggages/second.bin", b"second opaque"),
    )
    .unwrap();
    let facts = second.facts.as_ref().unwrap();

    assert_eq!(
        fact_count(
            facts,
            InstallCategory::ArchiveMember,
            InstallDisposition::Stored
        ),
        1
    );
    assert_eq!(
        fact_count(
            facts,
            InstallCategory::ArchiveMember,
            InstallDisposition::Deduplicated
        ),
        4
    );
    assert_eq!(
        fact_count(
            facts,
            InstallCategory::Product,
            InstallDisposition::Deduplicated
        ),
        1
    );
    assert_eq!(
        fact_count(
            facts,
            InstallCategory::ApplicationProgram,
            InstallDisposition::Deduplicated
        ),
        1
    );
    for category in [
        InstallCategory::Parameter,
        InstallCategory::CommunicationObject,
        InstallCategory::DynamicNode,
    ] {
        assert!(facts.counts.iter().all(|row| !(row.category == category
            && row.disposition == InstallDisposition::Deduplicated)));
        assert_eq!(fact_count(facts, category, InstallDisposition::Stored), 0);
        assert!(fact_count(facts, category, InstallDisposition::Read) > 0);
    }
    assert_eq!(
        fact_count(
            facts,
            InstallCategory::DatapointType,
            InstallDisposition::Dropped
        ),
        2
    );
}

#[test]
fn same_file_duplicate_program_cannot_contribute_dynamic_rows_or_arguments() {
    const DUPLICATE_PROGRAM: &[u8] = br#"<KNX xmlns="http://knx.org/xml/project/11"><ManufacturerData><Manufacturer RefId="M-0001"><ApplicationPrograms>
<ApplicationProgram Id="A-DUP" Name="winner"><Static/></ApplicationProgram>
<ApplicationProgram Id="A-DUP" Name="loser"><ModuleDefs><ModuleDef Id="MD-LOSER" Name="loser module"><Arguments><Argument Id="ARG-LOSER" Name="loser argument"/></Arguments><Dynamic><ParameterBlock Id="PB-LOSER"/></Dynamic></ModuleDef></ModuleDefs><Dynamic><Channel Id="CH-LOSER"/></Dynamic></ApplicationProgram>
</ApplicationPrograms></Manufacturer></ManufacturerData></KNX>"#;

    let dir = tempfile::tempdir().unwrap();
    let conn = open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
    let report = install_package(
        &conn,
        "duplicate.knxprod",
        &archive(&[
            ("knx_master.xml", MASTER),
            ("M-0001/Hardware.xml", HARDWARE),
            ("M-0001/A.xml", DUPLICATE_PROGRAM),
        ]),
    )
    .unwrap();

    for table in ["dynamic_node", "module_def_argument"] {
        let rows: i64 = conn
            .query_row(&format!("SELECT count(*) FROM {table}"), [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(rows, 0, "duplicate declaration wrote {table} rows");
    }

    let facts = report.facts.unwrap();
    assert_eq!(
        fact_count(
            &facts,
            InstallCategory::ApplicationProgram,
            InstallDisposition::Read
        ),
        2
    );
    assert_eq!(
        fact_count(
            &facts,
            InstallCategory::ApplicationProgram,
            InstallDisposition::Stored
        ),
        1
    );
    assert_eq!(
        fact_count(
            &facts,
            InstallCategory::ApplicationProgram,
            InstallDisposition::Deduplicated
        ),
        1
    );
    assert_eq!(
        fact_count(
            &facts,
            InstallCategory::DynamicNode,
            InstallDisposition::Read
        ),
        4
    );
    assert_eq!(
        fact_count(
            &facts,
            InstallCategory::DynamicNode,
            InstallDisposition::Stored
        ),
        0
    );
}

#[test]
fn repeated_unknowns_are_counted_and_are_independent_of_prior_blob_state() {
    let dir = tempfile::tempdir().unwrap();
    let conn = open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
    let first = install_package(
        &conn,
        "first.knxprod",
        &full_archive("M-0001/Baggages/first.bin", b"one"),
    )
    .unwrap();
    let second = install_package(
        &conn,
        "second.knxprod",
        &full_archive("M-0001/Baggages/second.bin", b"two"),
    )
    .unwrap();
    let first = first.facts.unwrap();
    let second = second.facts.unwrap();
    assert_eq!(first.unknown_constructs, second.unknown_constructs);
    assert_eq!(first.unknown_occurrences, second.unknown_occurrences);

    let mystery = first
        .unknown_constructs
        .iter()
        .find(|row| row.name == "Mystery")
        .unwrap();
    let foo = first
        .unknown_constructs
        .iter()
        .find(|row| row.name == "Foo")
        .unwrap();
    assert_eq!(mystery.occurrences, 2);
    assert_eq!(foo.occurrences, 2);
    assert!(first.unknown_occurrences > first.unknown_constructs.len() as u64);
}

#[test]
fn unsupported_master_sections_aggregate_and_languages_remain_supported() {
    let dir = tempfile::tempdir().unwrap();
    let conn = open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
    let report = install_package(
        &conn,
        "/host/private/source.knxprod",
        &full_archive("M-0001/Baggages/vendor.bin", b"opaque"),
    )
    .unwrap();
    let diagnostics = &report.facts.unwrap().diagnostics;
    let mask = diagnostics
        .iter()
        .find(|row| row.xml_path().ends_with("/MaskVersions"))
        .unwrap();
    assert_eq!(mask.kind(), InstallDiagnosticKind::UnsupportedMasterSection);
    assert_eq!(mask.occurrences(), 2);
    assert!(diagnostics
        .iter()
        .all(|row| !row.xml_path().ends_with("/Languages")));
    assert!(diagnostics.iter().all(|row| {
        !row.archive_path().starts_with('/')
            && row.xml_path().starts_with('/')
            && !row.detail().contains("/host/private")
    }));
    let baggage = diagnostics
        .iter()
        .find(|row| row.kind() == InstallDiagnosticKind::UnsupportedBaggageIndex)
        .unwrap();
    assert_eq!(baggage.occurrences(), 2);
}

#[test]
fn migrated_packages_are_unavailable_while_fresh_zeroes_are_measured() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("products.sqlite");
    let old_bytes = archive(&[
        ("knx_master.xml", MASTER),
        ("M-0001/Hardware.xml", HARDWARE),
    ]);
    {
        let conn = open_and_migrate(&path).unwrap();
        install_package(&conn, "old.knxprod", &old_bytes).unwrap();
        let foreign_keys: i64 = conn
            .query_row("PRAGMA foreign_keys", [], |row| row.get(0))
            .unwrap();
        assert_eq!(foreign_keys, 1);
        // Simulate v11 in dependency order while FK enforcement stays on:
        // detail children first, then their report parent.
        conn.execute_batch(
            "DROP TABLE package_install_diagnostic;
             DROP TABLE package_install_unknown;
             DROP TABLE package_install_count;
             DROP TABLE package_install_report;
             PRAGMA user_version = 11;",
        )
        .unwrap();
    }
    let conn = open_and_migrate(&path).unwrap();
    let old = install_package(&conn, "old-retry.knxprod", &old_bytes).unwrap();
    assert!(old.skipped);
    assert_eq!(old.facts, None);
    let status: String = conn
        .query_row(
            "SELECT status FROM package_install_report WHERE package_sha256 = ?1",
            [&old.sha256],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(status, "unavailable");

    let fresh_bytes = archive(&[
        ("knx_master.xml", MASTER),
        ("M-0001/Baggages/fresh.bin", b"fresh"),
        ("M-0001/Hardware.xml", HARDWARE),
    ]);
    let fresh = install_package(&conn, "fresh.knxprod", &fresh_bytes).unwrap();
    let facts = fresh.facts.unwrap();
    assert_eq!(
        fact_count(&facts, InstallCategory::Product, InstallDisposition::Read),
        0
    );
    assert_eq!(
        fact_count(&facts, InstallCategory::Module, InstallDisposition::Read),
        0
    );
}

#[test]
fn malformed_persisted_report_shapes_are_rejected() {
    let corruptions = [
        "UPDATE package_install_count SET category = 'invented' WHERE ordinal = 0",
        "UPDATE package_install_count SET count = -1 WHERE ordinal = 0",
        "UPDATE package_install_report SET status = 'mystery'",
        "UPDATE package_install_report SET unknown_occurrences = unknown_occurrences + 1",
        "UPDATE package_install_diagnostic SET kind = 'free-form' WHERE ordinal = 0",
        "UPDATE package_install_diagnostic SET archive_path = '/absolute.xml' WHERE ordinal = 0",
        "UPDATE package_install_diagnostic SET xml_path = 'relative/path' WHERE ordinal = 0",
        "UPDATE package_install_diagnostic SET archive_path = 'M-0001/missing.xml' WHERE kind = 'unsupported-master-section'",
        "UPDATE package_install_diagnostic SET archive_path = 'M-0001/Hardware.xml' WHERE kind = 'unsupported-master-section'",
        "UPDATE package_install_diagnostic SET archive_path = 'M-0001/Baggages.xml' WHERE kind = 'unsupported-master-section'",
        "UPDATE package_install_diagnostic SET xml_path = '/KNX/MasterData/MaskVersions/Nested' WHERE kind = 'unsupported-master-section'",
        "UPDATE package_install_diagnostic SET detail = detail || ' altered' WHERE kind = 'unsupported-master-section'",
        "UPDATE package_install_diagnostic SET xml_path = '/KNX/MasterData/Languages', detail = 'master section Languages is retained but not interpreted' WHERE kind = 'unsupported-master-section' AND ordinal = (SELECT min(ordinal) FROM package_install_diagnostic WHERE kind = 'unsupported-master-section')",
        "UPDATE package_install_diagnostic SET archive_path = 'knx_master.xml' WHERE kind = 'unsupported-baggage-index'",
        "UPDATE package_install_diagnostic SET xml_path = '/KNX/ManufacturerData/Manufacturer/Baggages/Other' WHERE kind = 'unsupported-baggage-index'",
        "UPDATE package_install_diagnostic SET detail = detail || ' altered' WHERE kind = 'unsupported-baggage-index'",
        "DELETE FROM package_install_count WHERE category = 'module' AND disposition = 'read'",
        "UPDATE package_install_count SET count = count + 1 WHERE category = 'master_section' AND disposition = 'unsupported'",
    ];
    for (case, sql) in corruptions.into_iter().enumerate() {
        let dir = tempfile::tempdir().unwrap();
        let conn = open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
        let bytes = full_archive("M-0001/Baggages/vendor.bin", b"opaque");
        install_package(&conn, "sample.knxprod", &bytes).unwrap();
        conn.execute_batch("PRAGMA ignore_check_constraints = ON;")
            .unwrap();
        conn.execute_batch(sql).unwrap();
        conn.execute_batch("PRAGMA ignore_check_constraints = OFF;")
            .unwrap();
        assert!(
            install_package(&conn, "retry.knxprod", &bytes).is_err(),
            "corruption case {case} was accepted"
        );
    }
}

#[test]
fn diagnostic_identity_is_unique_in_the_schema() {
    let dir = tempfile::tempdir().unwrap();
    let conn = open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
    let bytes = full_archive("M-0001/Baggages/vendor.bin", b"opaque");
    install_package(&conn, "sample.knxprod", &bytes).unwrap();

    let duplicate = conn.execute_batch(
        "INSERT INTO package_install_diagnostic
             (package_sha256, ordinal, kind, archive_path, xml_path, detail, occurrences)
         SELECT package_sha256, 999, kind, archive_path, xml_path, detail, occurrences
         FROM package_install_diagnostic LIMIT 1;",
    );
    assert!(duplicate.is_err());
}

#[test]
fn duplicate_diagnostic_identity_is_rejected_on_reload_without_schema_help() {
    let dir = tempfile::tempdir().unwrap();
    let conn = open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
    let bytes = full_archive("M-0001/Baggages/vendor.bin", b"opaque");
    install_package(&conn, "sample.knxprod", &bytes).unwrap();

    conn.execute_batch(
        "ALTER TABLE package_install_diagnostic RENAME TO original_diagnostic;
         CREATE TABLE package_install_diagnostic (
             package_sha256 TEXT NOT NULL REFERENCES package_install_report(package_sha256),
             ordinal INTEGER NOT NULL CHECK (ordinal >= 0),
             kind TEXT NOT NULL CHECK (kind IN ('unsupported-master-section','unsupported-baggage-index')),
             archive_path TEXT NOT NULL,
             xml_path TEXT NOT NULL,
             detail TEXT NOT NULL,
             occurrences INTEGER NOT NULL CHECK (occurrences > 0),
             PRIMARY KEY (package_sha256, ordinal)
         ) STRICT;
         INSERT INTO package_install_diagnostic SELECT * FROM original_diagnostic;
         UPDATE package_install_diagnostic SET occurrences = 1
         WHERE kind = 'unsupported-baggage-index';
         INSERT INTO package_install_diagnostic
             (package_sha256, ordinal, kind, archive_path, xml_path, detail, occurrences)
         SELECT package_sha256, 999, kind, archive_path, xml_path, detail, 1
         FROM original_diagnostic WHERE kind = 'unsupported-baggage-index';
         DROP TABLE original_diagnostic;",
    )
    .unwrap();

    assert!(install_package(&conn, "retry.knxprod", &bytes).is_err());
}

#[test]
fn missing_measured_report_row_is_corruption() {
    let dir = tempfile::tempdir().unwrap();
    let conn = open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
    let bytes = full_archive("M-0001/Baggages/vendor.bin", b"opaque");
    let report = install_package(&conn, "sample.knxprod", &bytes).unwrap();
    conn.execute_batch("PRAGMA foreign_keys = OFF;").unwrap();
    conn.execute(
        "DELETE FROM package_install_report WHERE package_sha256 = ?1",
        [&report.sha256],
    )
    .unwrap();
    conn.execute_batch("PRAGMA foreign_keys = ON;").unwrap();
    assert!(install_package(&conn, "retry.knxprod", &bytes).is_err());
}

#[test]
fn late_parser_failure_rolls_back_package_blobs_and_domain_rows() {
    const BAD_PROGRAM: &[u8] = br#"<KNX xmlns="http://knx.org/xml/project/11"><ManufacturerData><Manufacturer RefId="M-0001"><ApplicationPrograms><ApplicationProgram Id="A-BAD"><Static><Parameters><Parameter Id="DUP"/><Parameter Id="DUP"/></Parameters></Static></ApplicationProgram></ApplicationPrograms></Manufacturer></ManufacturerData></KNX>"#;
    let dir = tempfile::tempdir().unwrap();
    let conn = open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
    let bytes = archive(&[
        ("knx_master.xml", FULL_MASTER),
        ("M-0001/Hardware.xml", FULL_HARDWARE),
        ("M-0001/A.xml", BAD_PROGRAM),
    ]);
    assert!(install_package(&conn, "bad.knxprod", &bytes).is_err());
    for table in [
        "package",
        "source_file",
        "product",
        "application_program",
        "parameter",
        "datapoint_type",
    ] {
        let rows: i64 = conn
            .query_row(&format!("SELECT count(*) FROM {table}"), [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(rows, 0, "{table} escaped transaction rollback");
    }
}
