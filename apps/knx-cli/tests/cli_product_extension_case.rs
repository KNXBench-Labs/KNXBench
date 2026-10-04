//! Exercises case-independent CLI package admission without private manufacturer data.
//!
//! All input bytes are synthetic; no bus, host catalogue or vendor code is used.

use std::path::Path;
use std::process::{Command, Output};

const MASTER: &[u8] = br#"<KNX xmlns="http://knx.org/xml/project/11"><MasterData><Manufacturers><Manufacturer Id="M-0001" Name="Example"/></Manufacturers></MasterData></KNX>"#;
const HARDWARE: &[u8] = br#"<KNX xmlns="http://knx.org/xml/project/11"><ManufacturerData><Manufacturer RefId="M-0001"><Hardware><Hardware Id="M-0001_H-1"><Products><Product Id="M-0001_H-1_P-1" Text="Synthetic switch" OrderNumber="SYNTHETIC-1"/></Products><Hardware2Programs><Hardware2Program Id="M-0001_H-1_HP-1"><ApplicationProgramRef RefId="M-0001_A-0001-02-0000"/></Hardware2Program></Hardware2Programs></Hardware></Hardware></Manufacturer></ManufacturerData></KNX>"#;
const PROGRAM: &[u8] = br#"<KNX xmlns="http://knx.org/xml/project/11"><ManufacturerData><Manufacturer RefId="M-0001"><ApplicationPrograms><ApplicationProgram Id="M-0001_A-0001-02-0000" Name="Synthetic" ApplicationNumber="1" ApplicationVersion="2"/></ApplicationPrograms></Manufacturer></ManufacturerData></KNX>"#;

fn synthetic_package_bytes() -> Vec<u8> {
    knx_testsupport::zip_with_entries(&[
        ("knx_master.xml", MASTER),
        ("M-0001/Hardware.xml", HARDWARE),
        ("M-0001/M-0001_A-0001-02-0000.xml", PROGRAM),
    ])
}

fn ingest(input: &Path, database: &Path, data_home: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_knx"))
        .args(["products", "ingest"])
        .arg(input)
        .arg("--product-db")
        .arg(database)
        .env("XDG_DATA_HOME", data_home)
        .output()
        .expect("run the real CLI against a synthetic input")
}

fn installed(output: Output) -> String {
    assert_eq!(
        output.status.code(),
        Some(0),
        "CLI package admission failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.starts_with("package installed: "), "{stdout}");
    stdout
}

fn scheme23_package_bytes() -> Vec<u8> {
    let master = String::from_utf8(MASTER.to_vec())
        .unwrap()
        .replace("project/11", "project/23")
        .replace("</Manufacturers>", "</Manufacturers><DatapointTypes><DatapointType Id=\"D-23\" Number=\"23\" VariableLength=\"true\"/></DatapointTypes>");
    let hardware = String::from_utf8(HARDWARE.to_vec())
        .unwrap()
        .replace("project/11", "project/23")
        .replace(
            "<Hardware2Program Id=",
            "<Hardware2Program CouplerCapabilities=\"opaque\" Id=",
        );
    let program = String::from_utf8(PROGRAM.to_vec())
        .unwrap()
        .replace("project/11", "project/23")
        .replace(
            " Name=\"Synthetic\"",
            " Name=\"Synthetic\" HardwareType=\"opaque\"",
        );
    knx_testsupport::zip_with_entries(&[
        ("knx_master.xml", master.as_bytes()),
        ("M-0001/Hardware.xml", hardware.as_bytes()),
        ("M-0001/M-0001_A-0001-02-0000.xml", program.as_bytes()),
    ])
}

#[test]
fn scheme23_cli_installs_reports_opaque_evidence_and_replays_retained_archive() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("synthetic-23.knxprod");
    let database = dir.path().join("products.sqlite");
    let data_home = dir.path().join("data");
    let bytes = scheme23_package_bytes();
    std::fs::write(&input, &bytes).unwrap();
    let first = installed(ingest(&input, &database, &data_home));
    assert!(first.contains("scheme 23"), "{first}");
    assert!(!first.contains("already known"), "{first}");
    let db = knx_productdb::open_and_migrate(&database).unwrap();
    for name in ["HardwareType", "CouplerCapabilities", "VariableLength"] {
        let count: i64 = db
            .query_row(
                "SELECT COUNT(*) FROM package_install_unknown WHERE name=?1 AND kind='Attribute'",
                [name],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 1, "CLI must retain explicit opaque {name} evidence");
    }
    let retained: Vec<u8> = db
        .query_row("SELECT bytes FROM package", [], |row| row.get(0))
        .unwrap();
    assert!(retained == bytes, "CLI retained archive bytes changed");
    let member_count: i64 = db
        .query_row("SELECT COUNT(*) FROM source_file", [], |row| row.get(0))
        .unwrap();
    assert_eq!(member_count, 3);
    let replay = installed(ingest(&input, &database, &data_home));
    assert!(
        replay.contains("(already known; 1 source name(s))"),
        "{replay}"
    );
    let facts = |stdout: &str| -> serde_json::Value {
        let encoded = stdout
            .lines()
            .find_map(|line| line.strip_prefix("install_facts_json "))
            .expect("CLI must expose measured JSON facts");
        serde_json::from_str(encoded).unwrap()
    };
    let first_facts = facts(&first);
    assert_eq!(first_facts["status"], "measured");
    for (name, suffix, sample) in [
        ("HardwareType", "/ApplicationProgram", "opaque"),
        ("CouplerCapabilities", "/Hardware2Program", "opaque"),
        ("VariableLength", "/DatapointType", "true"),
    ] {
        let matching: Vec<_> = first_facts["facts"]["unknownConstructs"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|row| row["name"] == name && row["kind"] == "Attribute")
            .collect();
        assert_eq!(
            matching.len(),
            1,
            "CLI must report exact opaque {name} evidence"
        );
        assert!(matching[0]["xpath"].as_str().unwrap().ends_with(suffix));
        assert_eq!(matching[0]["sample"], sample);
        assert_eq!(matching[0]["occurrences"], 1);
    }
    assert_eq!(
        facts(&replay),
        first_facts,
        "replay must preserve measured public evidence"
    );
    assert!(std::fs::read(&input).unwrap() == bytes);
}

#[test]
fn scheme23_cli_refuses_foreign_members_without_changing_seeded_database() {
    let dir = tempfile::tempdir().unwrap();
    let seed = dir.path().join("seed.knxprod");
    let input = dir.path().join("foreign-23.knxprod");
    let database = dir.path().join("products.sqlite");
    let data_home = dir.path().join("data");
    std::fs::write(&seed, synthetic_package_bytes()).unwrap();
    installed(ingest(&seed, &database, &data_home));
    let before = std::fs::read(&database).unwrap();
    let master = String::from_utf8(MASTER.to_vec())
        .unwrap()
        .replace("project/11", "project/23");
    let bytes = knx_testsupport::zip_with_entries(&[
        ("knx_master.xml", master.as_bytes()),
        ("M-0001/Hardware.xml", HARDWARE),
    ]);
    std::fs::write(&input, &bytes).unwrap();
    let output = ingest(&input, &database, &data_home);
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(
        stderr.contains("scheme-23 XML contains a non-KNX element namespace"),
        "{stderr}"
    );
    assert!(
        std::fs::read(&database).unwrap() == before,
        "caller refusal changed persisted seed database bytes"
    );
    assert!(std::fs::read(&input).unwrap() == bytes);
}

#[test]
fn uppercase_knxprod_installs_and_retries_the_same_retained_package() {
    let dir = tempfile::tempdir().unwrap();
    let database = dir.path().join("products.sqlite");
    let data_home = dir.path().join("isolated-data");
    let bytes = synthetic_package_bytes();
    let lower = dir.path().join("lower.knxprod");
    let upper = dir.path().join("upper.KNXPROD");
    std::fs::write(&lower, &bytes).unwrap();
    std::fs::write(&upper, &bytes).unwrap();

    let first = installed(ingest(&lower, &database, &data_home));
    assert!(!first.contains("already known"));
    let retried = installed(ingest(&upper, &database, &data_home));
    assert!(
        retried.contains("(already known; 2 source name(s))"),
        "{retried}"
    );

    let db = knx_productdb::open_and_migrate(&database).unwrap();
    let product_count: i64 = db
        .query_row(
            "SELECT COUNT(*) FROM product WHERE id = 'M-0001_H-1_P-1'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(product_count, 1);
    let package_count: i64 = db
        .query_row("SELECT COUNT(*) FROM package", [], |row| row.get(0))
        .unwrap();
    assert_eq!(package_count, 1);
    let retained: Vec<u8> = db
        .query_row("SELECT bytes FROM package", [], |row| row.get(0))
        .unwrap();
    assert!(
        retained == bytes,
        "retained synthetic archive bytes changed"
    );
    assert!(std::fs::read(&lower).unwrap() == bytes);
    assert!(std::fs::read(&upper).unwrap() == bytes);
}

#[test]
fn uppercase_and_mixed_case_knxprod_install_without_a_preexisting_package() {
    let bytes = synthetic_package_bytes();
    for extension in ["KNXPROD", "KnXpRoD"] {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join(format!("fresh.{extension}"));
        let database = dir.path().join("new-products.sqlite");
        std::fs::write(&input, &bytes).unwrap();
        let stdout = installed(ingest(&input, &database, &dir.path().join("data")));
        assert!(!stdout.contains("already known"));
        let db = knx_productdb::open_and_migrate(&database).unwrap();
        let count: i64 = db
            .query_row("SELECT COUNT(*) FROM package", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 1);
        let retained: Vec<u8> = db
            .query_row("SELECT bytes FROM package", [], |row| row.get(0))
            .unwrap();
        assert!(retained == bytes, "fresh package bytes changed");
        assert!(std::fs::read(&input).unwrap() == bytes);
    }
}

#[test]
fn uppercase_knxproj_still_uses_the_project_importer() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("project.KNXPROJ");
    let database = dir.path().join("products.sqlite");
    let bytes = knx_testsupport::minimal_knxproj_bytes();
    std::fs::write(&input, &bytes).unwrap();
    let output = ingest(&input, &database, &dir.path().join("data"));
    assert_eq!(
        output.status.code(),
        Some(0),
        "project control failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).unwrap();
    // The fixture includes one manufacturer XML and one retained baggage member;
    // this existing CLI summary counts both ingested files, not only parsed XML.
    assert!(
        stdout.contains("2 manufacturer file(s) ingested"),
        "{stdout}"
    );
    assert!(stdout.contains("facts: not applicable"), "{stdout}");
    assert!(!stdout.contains("package installed:"));
    let db = knx_productdb::open_and_migrate(&database).unwrap();
    let packages: i64 = db
        .query_row("SELECT COUNT(*) FROM package", [], |row| row.get(0))
        .unwrap();
    assert_eq!(packages, 0);
    let sources: i64 = db
        .query_row("SELECT COUNT(*) FROM source_file", [], |row| row.get(0))
        .unwrap();
    assert!(sources > 0);
    assert!(std::fs::read(&input).unwrap() == bytes);
}

#[test]
fn legacy_case_variants_stay_refused_before_database_creation() {
    let bytes = b"SYNTHETIC-NOT-A-LEGACY-CONTAINER";
    for extension in ["VD2", "vD2", "VD3", "vD4", "VD5", "PR3", "pR4", "Pr5"] {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join(format!("legacy.{extension}"));
        let database = dir.path().join("must-not-exist.sqlite");
        std::fs::write(&input, bytes).unwrap();
        let output = ingest(&input, &database, &dir.path().join("data"));
        assert_eq!(output.status.code(), Some(1));
        assert!(output.stdout.is_empty());
        let stderr = String::from_utf8(output.stderr).unwrap();
        assert!(
            stderr.contains(&format!("legacy ETS filename extension .{extension} is unsupported; legacy import is not implemented")),
            "{stderr}"
        );
        assert!(!stderr.contains("SYNTHETIC-NOT-A-LEGACY-CONTAINER"));
        assert!(!database.exists(), "legacy refusal created a database");
        assert!(std::fs::read(&input).unwrap() == bytes);
    }
}

#[test]
fn package_extension_lookalikes_do_not_acquire_package_semantics() {
    let bytes = synthetic_package_bytes();
    for suffix in ["KNXPROD.bak", "notknxprod", "KnXpRoD-extra"] {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join(format!("input.{suffix}"));
        let database = dir.path().join("must-not-exist.sqlite");
        std::fs::write(&input, &bytes).unwrap();
        let output = ingest(&input, &database, &dir.path().join("data"));
        assert_eq!(output.status.code(), Some(1));
        assert!(output.stdout.is_empty());
        assert!(String::from_utf8(output.stderr)
            .unwrap()
            .contains("no P-*.signature entry; cannot determine the project part"));
        assert!(
            !database.exists(),
            "unrecognized extension created a database"
        );
        assert!(std::fs::read(&input).unwrap() == bytes);
    }
}
