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

/// PDB-8: inside a *supported* master section the parser interprets only
/// part of the structure. Every element subtree it does not interpret is
/// reported once per occurrence at its canonical path, instead of vanishing
/// into the retained blob. Shapes taken from the private corpus:
/// `Manufacturer/PublicKeys` and `Manufacturer/OrderNumberFormattingScript`
/// under `Manufacturers`, `DatapointSubtype/Format` under `DatapointTypes`.
const MASTER_WITH_SUBTREES: &[u8] = br#"<KNX xmlns="http://knx.org/xml/project/11"><MasterData>
<Manufacturers>
<Manufacturer Id="M-0001" Name="Example"><PublicKeys><PublicKey><RSAKeyValue><Modulus>AQ==</Modulus><Exponent>AQAB</Exponent></RSAKeyValue></PublicKey></PublicKeys></Manufacturer>
<Manufacturer Id="M-0002" Name="Other"><OrderNumberFormattingScript>x</OrderNumberFormattingScript><PublicKeys><PublicKey/></PublicKeys></Manufacturer>
</Manufacturers>
<DatapointTypes><DatapointType Id="DPT-1" Number="1" Name="one" Text="one"><DatapointSubtypes>
<DatapointSubtype Id="DPST-1-1" Number="1" Name="switch" Text="switch"><Format><Bit Id="B-1" Cleared="off" Set="on"/></Format></DatapointSubtype>
<DatapointSubtype Id="DPST-1-2" Number="2" Name="bool" Text="bool"/>
</DatapointSubtypes></DatapointType></DatapointTypes>
<FunctionTypes><FunctionType Id="FT-1" Number="1" Text="f"><FunctionPoint Id="FP-1" Text="p"/></FunctionType></FunctionTypes>
<SpaceUsages><SpaceUsage Id="SU-1" Number="1" Text="room"/></SpaceUsages>
<Languages><Language Identifier="de-DE"><TranslationUnit RefId="M-0001"><TranslationElement RefId="M-0001"><Translation AttributeName="Name" Text="Beispiel"/></TranslationElement></TranslationUnit></Language></Languages>
<MaskVersions><MaskVersion Id="MV-1"><HawkConfigurationData/></MaskVersion></MaskVersions>
</MasterData></KNX>"#;

fn subtree_archive() -> Vec<u8> {
    archive(&[
        ("knx_master.xml", MASTER_WITH_SUBTREES),
        ("M-0001/Hardware.xml", HARDWARE),
        ("M-0001/Baggages.xml", BAGGAGES),
    ])
}

#[test]
fn uninterpreted_subtrees_inside_supported_master_sections_are_reported() {
    let dir = tempfile::tempdir().unwrap();
    let conn = open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
    let report = install_package(&conn, "subtrees.knxprod", &subtree_archive()).unwrap();
    let facts = report.facts.unwrap();

    let subtrees: Vec<(&str, u64)> = facts
        .diagnostics
        .iter()
        .filter(|row| row.kind() == InstallDiagnosticKind::UnsupportedMasterSubtree)
        .map(|row| (row.xml_path(), row.occurrences()))
        .collect();
    assert_eq!(
        subtrees,
        [
            (
                "/KNX/MasterData/DatapointTypes/DatapointType/DatapointSubtypes/DatapointSubtype/Format",
                1
            ),
            (
                "/KNX/MasterData/Manufacturers/Manufacturer/OrderNumberFormattingScript",
                1
            ),
            ("/KNX/MasterData/Manufacturers/Manufacturer/PublicKeys", 2),
        ],
        "one row per uninterpreted subtree root; descendants (PublicKey, \
         RSAKeyValue, Bit) are inside the root, not rows of their own; \
         interpreted structure (FunctionPoint, SpaceUsage, Translation) and \
         whole unsupported sections (MaskVersions) are not subtree rows"
    );
    assert!(facts
        .diagnostics
        .iter()
        .filter(|row| row.kind() == InstallDiagnosticKind::UnsupportedMasterSubtree)
        .all(|row| row.archive_path() == "knx_master.xml"));
    assert_eq!(
        fact_count(
            &facts,
            InstallCategory::MasterSubtree,
            InstallDisposition::Unsupported
        ),
        4
    );
    // The unsupported *section* keeps its own, unchanged diagnostic.
    assert_eq!(
        fact_count(
            &facts,
            InstallCategory::MasterSection,
            InstallDisposition::Unsupported
        ),
        1
    );
}

#[test]
fn a_master_file_without_uninterpreted_subtrees_measures_zero() {
    let dir = tempfile::tempdir().unwrap();
    let conn = open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
    let report = install_package(
        &conn,
        "plain.knxprod",
        &full_archive("M-0001/Baggages/vendor.bin", b"opaque"),
    )
    .unwrap();
    let facts = report.facts.unwrap();
    assert_eq!(
        fact_count(
            &facts,
            InstallCategory::MasterSubtree,
            InstallDisposition::Unsupported
        ),
        0
    );
    assert!(facts
        .diagnostics
        .iter()
        .all(|row| row.kind() != InstallDiagnosticKind::UnsupportedMasterSubtree));
}

/// Rewinds a v14 database to v13's report shape: no `master_subtree` count
/// row and no subtree diagnostics. The v14 CHECK lists stay, which v13 would
/// not have, but the migration rebuilds both tables anyway.
fn rewind_to_v13(conn: &Connection) {
    conn.execute_batch(
        "DELETE FROM package_install_count WHERE category = 'master_subtree';
         DELETE FROM package_install_diagnostic WHERE kind = 'unsupported-master-subtree';
         PRAGMA user_version = 13;",
    )
    .unwrap();
}

#[test]
fn v13_to_v14_backfills_subtrees_from_the_retained_master_blob() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("products.sqlite");
    let bytes = subtree_archive();
    let fresh = {
        let conn = open_and_migrate(&path).unwrap();
        let fresh = install_package(&conn, "subtrees.knxprod", &bytes)
            .unwrap()
            .facts
            .unwrap();
        rewind_to_v13(&conn);
        fresh
    };
    let conn = open_and_migrate(&path).unwrap();
    let version: i64 = conn
        .query_row("PRAGMA user_version", [], |row| row.get(0))
        .unwrap();
    assert_eq!(version, knx_productdb::CURRENT_PRODUCTDB_VERSION);
    let retried = install_package(&conn, "retry.knxprod", &bytes).unwrap();
    assert!(retried.skipped);
    assert_eq!(
        retried.facts.unwrap(),
        fresh,
        "the backfill measures exactly what a fresh install measures"
    );
}

#[test]
fn v13_to_v14_rolls_back_when_a_retained_master_no_longer_scans() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("products.sqlite");
    {
        let conn = open_and_migrate(&path).unwrap();
        install_package(&conn, "subtrees.knxprod", &subtree_archive()).unwrap();
        rewind_to_v13(&conn);
        conn.execute(
            "UPDATE source_file SET bytes = ?1 WHERE sha256 IN
                 (SELECT source_sha256 FROM package_member WHERE role = 'Master')",
            [b"<KNX><MasterData></KNX>".as_slice()],
        )
        .unwrap();
    }
    assert!(
        open_and_migrate(&path).is_err(),
        "a count that cannot be measured is not invented as zero"
    );
    let conn = Connection::open(&path).unwrap();
    let version: i64 = conn
        .query_row("PRAGMA user_version", [], |row| row.get(0))
        .unwrap();
    assert_eq!(version, 13, "the failed step is rolled back whole");
    let rows: i64 = conn
        .query_row("SELECT count(*) FROM package_install_count", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert!(rows > 0, "existing evidence survives the failed migration");
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
             ALTER TABLE application_program DROP COLUMN is_secure_enabled;
             ALTER TABLE application_program DROP COLUMN max_security_group_key_table_entries;
             ALTER TABLE application_program DROP COLUMN max_security_individual_address_entries;
             ALTER TABLE application_program DROP COLUMN max_security_p2p_key_table_entries;
             ALTER TABLE application_program DROP COLUMN max_tunneling_user_entries;
             ALTER TABLE application_program DROP COLUMN max_user_entries;
             ALTER TABLE application_program DROP COLUMN min_ets_version;
             ALTER TABLE application_program DROP COLUMN replaces_versions;
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
    assert_rejected_after(
        &corruptions,
        &full_archive("M-0001/Baggages/vendor.bin", b"opaque"),
    );
}

#[test]
fn malformed_persisted_subtree_diagnostics_are_rejected() {
    let corruptions = [
        "UPDATE package_install_count SET count = count + 1 WHERE category = 'master_subtree' AND disposition = 'unsupported'",
        "UPDATE package_install_diagnostic SET archive_path = 'M-0001/Hardware.xml' WHERE kind = 'unsupported-master-subtree'",
        "UPDATE package_install_diagnostic SET detail = detail || ' altered' WHERE kind = 'unsupported-master-subtree'",
        // A whole unsupported section is not a subtree of a supported one.
        "UPDATE package_install_diagnostic SET xml_path = '/KNX/MasterData/MaskVersions/MaskVersion', detail = 'master subtree MaskVersion is retained but not interpreted' WHERE kind = 'unsupported-master-subtree' AND xml_path LIKE '%/PublicKeys'",
        // Interpreted structure is not an uninterpreted subtree.
        "UPDATE package_install_diagnostic SET xml_path = '/KNX/MasterData/FunctionTypes/FunctionType/FunctionPoint', detail = 'master subtree FunctionPoint is retained but not interpreted' WHERE kind = 'unsupported-master-subtree' AND xml_path LIKE '%/PublicKeys'",
        // A descendant of an uninterpreted root is inside that root.
        "UPDATE package_install_diagnostic SET xml_path = '/KNX/MasterData/Manufacturers/Manufacturer/PublicKeys/PublicKey', detail = 'master subtree PublicKey is retained but not interpreted' WHERE kind = 'unsupported-master-subtree' AND xml_path LIKE '%/PublicKeys'",
    ];
    assert_rejected_after(&corruptions, &subtree_archive());
}

fn assert_rejected_after(corruptions: &[&str], bytes: &[u8]) {
    for (case, sql) in corruptions.iter().enumerate() {
        let dir = tempfile::tempdir().unwrap();
        let conn = open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
        install_package(&conn, "sample.knxprod", bytes).unwrap();
        conn.execute_batch("PRAGMA ignore_check_constraints = ON;")
            .unwrap();
        conn.execute_batch(sql).unwrap();
        conn.execute_batch("PRAGMA ignore_check_constraints = OFF;")
            .unwrap();
        assert!(
            install_package(&conn, "retry.knxprod", bytes).is_err(),
            "corruption case {case} ({sql}) was accepted"
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
