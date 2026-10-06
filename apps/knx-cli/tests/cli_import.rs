//! Drives the built `knx` binary with `std::process::Command` — this is
//! the CLI's own contract with a user, not something a library-level test
//! against `import_ets_project` directly can stand in for.

use std::io::{Cursor, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn reference_ets4_path() -> PathBuf {
    knx_testsupport::reference_ets4_path()
}

fn run_cli(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_knx"))
        .args(args)
        .output()
        .expect("failed to run the knx binary")
}

#[test]
#[ignore = "requires the gitignored OriginalData/ corpus; run with --ignored"]
fn the_cli_reports_counts_and_exits_zero() {
    assert!(
        reference_ets4_path().exists(),
        "OriginalData/ corpus not present (gitignored, local-only); this test is #[ignore]d and must be run explicitly on a machine that has it"
    );
    // `--no-product-db`: this test is about import mechanics, not product
    // data, and must not touch the real shared product database — every
    // plain `knx import` invocation across this suite otherwise races on
    // the same default-path file when tests run in parallel.
    let out = run_cli(&[
        "import",
        reference_ets4_path().to_str().unwrap(),
        "--no-product-db",
    ]);
    assert_eq!(out.status.code(), Some(0));
    let text = String::from_utf8(out.stdout).unwrap();
    assert!(text.contains("36 devices"));
    assert!(text.contains("514 group addresses"));
    assert!(text.contains("unsupported"));
}

#[test]
#[ignore = "requires the gitignored OriginalData/ corpus; run with --ignored"]
fn the_cli_writes_a_machine_readable_report() {
    assert!(
        reference_ets4_path().exists(),
        "OriginalData/ corpus not present (gitignored, local-only); this test is #[ignore]d and must be run explicitly on a machine that has it"
    );
    let dir = tempfile::tempdir().unwrap();
    let report = dir.path().join("report.json");
    let out = run_cli(&[
        "import",
        reference_ets4_path().to_str().unwrap(),
        "--report-json",
        report.to_str().unwrap(),
        "--no-product-db",
    ]);
    assert_eq!(out.status.code(), Some(0));
    let json: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&report).unwrap()).unwrap();
    assert_eq!(json["source"]["schema_version"], 11);
    assert!(!json["unsupported"].as_array().unwrap().is_empty());
}

#[test]
#[ignore = "requires the gitignored OriginalData/ corpus; run with --ignored"]
fn the_cli_exits_nonzero_on_a_file_it_cannot_read() {
    assert!(
        reference_ets4_path().exists(),
        "OriginalData/ corpus not present (gitignored, local-only); this test is #[ignore]d and must be run explicitly on a machine that has it"
    );
    let out = run_cli(&["import", "/nonexistent.knxproj", "--no-product-db"]);
    assert_eq!(out.status.code(), Some(1));
    assert!(String::from_utf8(out.stderr)
        .unwrap()
        .contains("nonexistent"));
}

#[test]
fn a_flag_missing_its_value_is_a_usage_error_not_a_swallowed_flag() {
    // `--store` must not silently consume `--report-json` as its own value.
    let out = run_cli(&[
        "import",
        "--store",
        "--report-json",
        "out.json",
        reference_ets4_path().to_str().unwrap(),
    ]);
    assert_eq!(out.status.code(), Some(1));
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert!(stderr.contains("--store"));
}

/// A structurally valid schema-11 project whose only defect is a duplicate
/// `GroupAddress/@Id` — validation reports it as an error, but the import
/// itself still produces a project. Written by hand rather than derived
/// from the reference project, so the CLI's exit-code contract is tested
/// against a known, minimal defect.
const DUPLICATE_ID_TOPOLOGY: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<KNX xmlns="http://knx.org/xml/project/11" CreatedBy="ETS4" ToolVersion="ETS 4.1.8">
  <Project Id="P-0001">
    <Installations>
      <Installation InstallationId="0" Name="" CompletionStatus="Undefined">
        <Topology />
        <GroupAddresses>
          <GroupRanges>
            <GroupRange Id="P-0001-0_GR-1" Name="Licht" RangeStart="1" RangeEnd="255">
              <GroupAddress Id="P-0001-0_GA-1" Address="1" Name="GA" />
              <GroupAddress Id="P-0001-0_GA-1" Address="2" Name="GA2" />
            </GroupRange>
          </GroupRanges>
        </GroupAddresses>
      </Installation>
    </Installations>
  </Project>
</KNX>"#;

const PROJECT_INFO: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<KNX xmlns="http://knx.org/xml/project/11">
  <Project Id="P-0001">
    <ProjectInformation Name="T" GroupAddressStyle="ThreeLevel" CompletionStatus="Undefined" />
  </Project>
</KNX>"#;

fn write_knxproj_with_duplicate_id(path: &Path) {
    write_knxproj_topology(path, DUPLICATE_ID_TOPOLOGY);
}

fn write_knxproj_topology(path: &Path, topology: &str) {
    let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let options =
        zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
    for (name, bytes) in [
        ("P-0001.signature", "x"),
        ("P-0001/0.xml", topology),
        ("P-0001/Project.xml", PROJECT_INFO),
    ] {
        writer.start_file(name, options).unwrap();
        writer.write_all(bytes.as_bytes()).unwrap();
    }
    std::fs::write(path, writer.finish().unwrap().into_inner()).unwrap();
}

#[test]
#[ignore = "requires the gitignored OriginalData/ corpus; run with --ignored"]
fn import_with_a_product_db_reports_what_it_ingested() {
    assert!(
        reference_ets4_path().exists(),
        "OriginalData/ corpus not present (gitignored, local-only); this test is #[ignore]d and must be run explicitly on a machine that has it"
    );
    let dir = tempfile::tempdir().unwrap();
    let products = dir.path().join("products.sqlite");
    let out = run_cli(&[
        "import",
        reference_ets4_path().to_str().unwrap(),
        "--product-db",
        products.to_str().unwrap(),
    ]);
    assert_eq!(out.status.code(), Some(0));
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("manufacturer file"), "{stdout}");
    assert!(products.exists());
}

#[test]
#[ignore = "requires the gitignored OriginalData/ corpus; run with --ignored"]
fn import_with_no_product_db_keeps_manufacturer_data_in_the_project() {
    assert!(
        reference_ets4_path().exists(),
        "OriginalData/ corpus not present (gitignored, local-only); this test is #[ignore]d and must be run explicitly on a machine that has it"
    );
    let dir = tempfile::tempdir().unwrap();
    let store = dir.path().join("p.knxdb");
    let out = run_cli(&[
        "import",
        reference_ets4_path().to_str().unwrap(),
        "--store",
        store.to_str().unwrap(),
        "--no-product-db",
    ]);
    assert_eq!(out.status.code(), Some(0));
    let conn = knx_store::open_and_migrate(&store).unwrap();
    assert!(knx_store::load_opaque(&conn)
        .unwrap()
        .iter()
        .any(|e| e.kind == "ManufacturerData"));
}

#[test]
fn product_db_and_no_product_db_together_are_a_usage_error() {
    let out = run_cli(&[
        "import",
        reference_ets4_path().to_str().unwrap(),
        "--product-db",
        "/tmp/x.sqlite",
        "--no-product-db",
    ]);
    assert_eq!(out.status.code(), Some(1));
    assert!(String::from_utf8(out.stderr)
        .unwrap()
        .contains("--no-product-db"));
}

#[test]
#[ignore = "requires the gitignored OriginalData/ corpus; run with --ignored"]
fn products_list_prints_what_was_ingested() {
    assert!(
        reference_ets4_path().exists(),
        "OriginalData/ corpus not present (gitignored, local-only); this test is #[ignore]d and must be run explicitly on a machine that has it"
    );
    let dir = tempfile::tempdir().unwrap();
    let products = dir.path().join("products.sqlite");
    run_cli(&[
        "products",
        "ingest",
        reference_ets4_path().to_str().unwrap(),
        "--product-db",
        products.to_str().unwrap(),
    ]);
    let out = run_cli(&[
        "products",
        "list",
        "--product-db",
        products.to_str().unwrap(),
    ]);
    assert_eq!(out.status.code(), Some(0));
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("M-0083"), "{stdout}");
}

fn write_standalone_product_package(path: &Path) {
    let master = br#"<KNX xmlns="http://knx.org/xml/project/11"><MasterData><Manufacturers><Manufacturer Id="M-0001" Name="Example"/></Manufacturers></MasterData></KNX>"#;
    let catalog = br#"<KNX xmlns="http://knx.org/xml/project/11"><ManufacturerData><Manufacturer RefId="M-0001"><Catalog><CatalogSection Id="M-0001_CG-1" Name="Actuators" Number="1"><CatalogItem Id="M-0001_CI-1" Name="Example actuator" Number="EX-1" ProductRefId="M-0001_P-1"/></CatalogSection></Catalog></Manufacturer></ManufacturerData></KNX>"#;
    let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let options = zip::write::SimpleFileOptions::default();
    for (name, bytes) in [
        ("knx_master.xml", master.as_slice()),
        ("M-0001/Catalog.xml", catalog.as_slice()),
    ] {
        writer.start_file(name, options).unwrap();
        std::io::Write::write_all(&mut writer, bytes).unwrap();
    }
    std::fs::write(path, writer.finish().unwrap().into_inner()).unwrap();
}

#[test]
#[ignore = "requires the gitignored OriginalData/ corpus; run with --ignored"]
fn products_ingest_installs_a_standalone_package() {
    assert!(
        reference_ets4_path().exists(),
        "OriginalData/ corpus not present (gitignored, local-only); this test is #[ignore]d and must be run explicitly on a machine that has it"
    );
    let dir = tempfile::tempdir().unwrap();
    let products = dir.path().join("products.sqlite");
    let package = dir.path().join("example.knxprod");
    write_standalone_product_package(&package);

    let out = run_cli(&[
        "products",
        "ingest",
        package.to_str().unwrap(),
        "--product-db",
        products.to_str().unwrap(),
    ]);

    assert_eq!(out.status.code(), Some(0));
    assert!(String::from_utf8(out.stdout)
        .unwrap()
        .contains("package installed"));
    let conn = knx_productdb::open_and_migrate(&products).unwrap();
    assert_eq!(
        knx_productdb::query::catalog_items(&conn, Some("M-0001"), None, None)
            .unwrap()
            .len(),
        1
    );
}

#[test]
fn legacy_cli_rejection_happens_before_destination_creation() {
    let dir = tempfile::tempdir().unwrap();
    let store = dir.path().join("new.knxdb");
    let products = dir.path().join("new-products.sqlite");
    let report = dir.path().join("new-report.json");
    for extension in [
        "vd2", "vd3", "vd4", "vd5", "pr3", "pr4", "pr5", "VD4", "Pr3",
    ] {
        let source = dir.path().join(format!("input.{extension}"));
        std::fs::write(&source, b"SYNTHETIC-SENSITIVE-PAYLOAD").unwrap();
        for args in [
            vec![
                "import",
                source.to_str().unwrap(),
                "--store",
                store.to_str().unwrap(),
                "--product-db",
                products.to_str().unwrap(),
                "--report-json",
                report.to_str().unwrap(),
            ],
            vec![
                "products",
                "ingest",
                source.to_str().unwrap(),
                "--product-db",
                products.to_str().unwrap(),
            ],
        ] {
            let out = run_cli(&args);
            assert_eq!(out.status.code(), Some(1));
            assert!(out.stdout.is_empty());
            assert!(!store.exists(), "refusal created a native destination");
            assert!(!products.exists(), "refusal created a product database");
            assert!(!report.exists(), "refusal emitted a success report");
            let stderr = String::from_utf8(out.stderr).unwrap();
            assert!(stderr.contains(&format!("legacy ETS filename extension .{extension} is unsupported; legacy import is not implemented")), "{stderr}");
            assert!(!stderr.contains("SYNTHETIC-SENSITIVE-PAYLOAD"));
        }
    }
}

#[test]
fn legacy_cli_rejection_preserves_native_projects_products_and_reports() {
    let dir = tempfile::tempdir().unwrap();
    let seed = dir.path().join("seed.knxproj");
    let topology = DUPLICATE_ID_TOPOLOGY.replace(
        r#"<GroupAddress Id="P-0001-0_GA-1" Address="2" Name="GA2" />"#,
        r#"<GroupAddress Id="P-0001-0_GA-2" Address="2" Name="GA2" />"#,
    );
    write_knxproj_topology(&seed, &topology);
    let store = dir.path().join("existing.knxdb");
    let conn = knx_store::open_and_migrate(&store).unwrap();
    let imported = knx_app::import_ets_project(&seed, &conn).unwrap();
    assert_eq!(imported.report.error_count(), 0);
    knx_store::save_project(&conn, &imported.project).unwrap();

    let opaque = knx_store::load_opaque(&conn).unwrap();
    assert!(!opaque.is_empty());
    drop(conn);
    let products = dir.path().join("existing-products.sqlite");
    let package = dir.path().join("seed.knxprod");
    write_standalone_product_package(&package);
    assert_eq!(
        run_cli(&[
            "products",
            "ingest",
            package.to_str().unwrap(),
            "--product-db",
            products.to_str().unwrap()
        ])
        .status
        .code(),
        Some(0)
    );
    let report = dir.path().join("existing-report.json");
    std::fs::write(&report, b"existing report must survive refusal").unwrap();
    let before = [&store, &products, &report].map(|path| std::fs::read(path).unwrap());
    for extension in [
        "vd2", "vd3", "vd4", "vd5", "pr3", "pr4", "pr5", "VD5", "Pr4",
    ] {
        let source = dir
            .path()
            .join(format!("readable-modern-project.{extension}"));
        std::fs::copy(&seed, &source).unwrap();
        for args in [
            vec![
                "import",
                source.to_str().unwrap(),
                "--store",
                store.to_str().unwrap(),
                "--product-db",
                products.to_str().unwrap(),
                "--report-json",
                report.to_str().unwrap(),
            ],
            vec![
                "products",
                "ingest",
                source.to_str().unwrap(),
                "--product-db",
                products.to_str().unwrap(),
            ],
        ] {
            let out = run_cli(&args);
            assert_eq!(out.status.code(), Some(1));
            assert!(out.stdout.is_empty());
            let stderr = String::from_utf8(out.stderr).unwrap();
            assert!(
                stderr.contains(&format!(
                    "legacy ETS filename extension .{extension} is unsupported"
                )),
                "{stderr}"
            );
            assert!(
                [&store, &products, &report].map(|path| std::fs::read(path).unwrap()) == before
            );
        }
    }
    let conn = knx_store::open_and_migrate(&store).unwrap();
    assert_eq!(knx_store::load_project(&conn).unwrap(), imported.project);
    assert!(knx_store::load_opaque(&conn).unwrap() == opaque);
    let conn = knx_productdb::open_and_migrate(&products).unwrap();
    assert_eq!(
        knx_productdb::query::catalog_items(&conn, None, None, None)
            .unwrap()
            .len(),
        1
    );
    assert!(knx_productdb::verify(&conn).unwrap().is_empty());
}

#[test]
fn products_ingest_refuses_unsupported_master_before_manufacturer_writes() {
    let dir = tempfile::tempdir().unwrap();
    let products = dir.path().join("products.sqlite");
    let seed = dir.path().join("seed.knxprod");
    write_standalone_product_package(&seed);
    assert_eq!(
        run_cli(&[
            "products",
            "ingest",
            seed.to_str().unwrap(),
            "--product-db",
            products.to_str().unwrap()
        ])
        .status
        .code(),
        Some(0)
    );
    let before = std::fs::read(&products).unwrap();
    for (root, namespace) in [
        ("KNX", "urn:SYNTHETIC-SENSITIVE-PAYLOAD/11"),
        ("Other", "http://knx.org/xml/project/11"),
    ] {
        let project = dir.path().join("unsupported-master.knxproj");
        let master = format!("<{root} xmlns=\"{namespace}\"><MasterData><DatapointTypes><DatapointType Id=\"DPT-9\" Number=\"9\" Name=\"Reject\"/></DatapointTypes></MasterData></{root}>");
        let hardware = br#"<KNX xmlns="http://knx.org/xml/project/11"><ManufacturerData><Manufacturer RefId="M-0001"><Hardware><Hardware Id="H-NEW" Name="New"/></Hardware></Manufacturer></ManufacturerData></KNX>"#;
        let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
        let options = zip::write::SimpleFileOptions::default();
        for (name, bytes) in [
            ("P-0001.signature", b"x".as_slice()),
            ("P-0001/0.xml", DUPLICATE_ID_TOPOLOGY.as_bytes()),
            ("P-0001/Project.xml", PROJECT_INFO.as_bytes()),
            ("knx_master.xml", master.as_bytes()),
            ("M-0001/Hardware.xml", hardware.as_slice()),
        ] {
            writer.start_file(name, options).unwrap();
            writer.write_all(bytes).unwrap();
        }
        std::fs::write(&project, writer.finish().unwrap().into_inner()).unwrap();
        let out = run_cli(&[
            "products",
            "ingest",
            project.to_str().unwrap(),
            "--product-db",
            products.to_str().unwrap(),
        ]);
        assert_eq!(out.status.code(), Some(1));
        assert!(out.stdout.is_empty());
        let stderr = String::from_utf8(out.stderr).unwrap();
        assert!(
            stderr
                .contains("project master root metadata is unsupported; no product data ingested"),
            "{stderr}"
        );
        assert!(!stderr.contains("SYNTHETIC-SENSITIVE-PAYLOAD"));
        assert_eq!(std::fs::read(&products).unwrap(), before);
        let unopened = dir.path().join("must-not-be-created.sqlite");
        let out = run_cli(&[
            "products",
            "ingest",
            project.to_str().unwrap(),
            "--product-db",
            unopened.to_str().unwrap(),
        ]);
        assert_eq!(out.status.code(), Some(1));
        assert!(out.stdout.is_empty());
        assert_eq!(
            String::from_utf8(out.stderr).unwrap().trim(),
            "project master root metadata is unsupported; no product data ingested"
        );
        assert!(
            !unopened.exists(),
            "master refusal created a product database"
        );
        let non_database = dir.path().join("must-not-be-opened.sqlite");
        let sentinel = b"synthetic destination must remain unopened";
        std::fs::write(&non_database, sentinel).unwrap();
        let out = run_cli(&[
            "products",
            "ingest",
            project.to_str().unwrap(),
            "--product-db",
            non_database.to_str().unwrap(),
        ]);
        assert_eq!(out.status.code(), Some(1));
        assert!(out.stdout.is_empty());
        assert_eq!(
            String::from_utf8(out.stderr).unwrap().trim(),
            "project master root metadata is unsupported; no product data ingested"
        );
        assert_eq!(std::fs::read(&non_database).unwrap(), sentinel);
        assert!(!non_database.with_extension("sqlite-wal").exists());
        assert!(!non_database.with_extension("sqlite-shm").exists());
        let conn = knx_productdb::open_and_migrate(&products).unwrap();
        assert_eq!(
            conn.query_row(
                "SELECT count(*) FROM hardware WHERE id = 'H-NEW'",
                [],
                |row| row.get::<_, i64>(0)
            )
            .unwrap(),
            0
        );
        assert_eq!(
            conn.query_row(
                "SELECT count(*) FROM datapoint_type WHERE id = 'DPT-9'",
                [],
                |row| row.get::<_, i64>(0)
            )
            .unwrap(),
            0
        );
    }
}

#[test]
fn standalone_package_cli_emits_one_structured_facts_line() {
    let dir = tempfile::tempdir().unwrap();
    let products = dir.path().join("products.sqlite");
    let package = dir.path().join("example.knxprod");
    write_standalone_product_package(&package);
    let out = run_cli(&[
        "products",
        "ingest",
        package.to_str().unwrap(),
        "--product-db",
        products.to_str().unwrap(),
    ]);
    assert_eq!(out.status.code(), Some(0));
    let stdout = String::from_utf8(out.stdout).unwrap();
    let line = stdout
        .lines()
        .find_map(|line| line.strip_prefix("install_facts_json "))
        .unwrap();
    let json: serde_json::Value = serde_json::from_str(line).unwrap();
    assert_eq!(json["status"], "measured");
    assert!(json["facts"]["counts"].is_array());
    assert!(json["facts"]["diagnostics"].is_array());
    assert_eq!(stdout.matches("install_facts_json ").count(), 1);
}

#[test]
fn single_file_project_ingest_does_not_claim_package_facts() {
    let dir = tempfile::tempdir().unwrap();
    let project = dir.path().join("sample.knxproj");
    let products = dir.path().join("products.sqlite");
    write_knxproj_with_duplicate_id(&project);
    let out = run_cli(&[
        "products",
        "ingest",
        project.to_str().unwrap(),
        "--product-db",
        products.to_str().unwrap(),
    ]);
    assert_eq!(out.status.code(), Some(0));
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(!stdout.contains("install_facts_json"), "{stdout}");
    assert!(stdout.contains("facts: not applicable"), "{stdout}");
}
#[test]
#[ignore = "requires the gitignored OriginalData/ corpus; run with --ignored"]
fn products_verify_is_clean_after_an_ingest() {
    assert!(
        reference_ets4_path().exists(),
        "OriginalData/ corpus not present (gitignored, local-only); this test is #[ignore]d and must be run explicitly on a machine that has it"
    );
    let dir = tempfile::tempdir().unwrap();
    let products = dir.path().join("products.sqlite");
    run_cli(&[
        "products",
        "ingest",
        reference_ets4_path().to_str().unwrap(),
        "--product-db",
        products.to_str().unwrap(),
    ]);
    let out = run_cli(&[
        "products",
        "verify",
        "--product-db",
        products.to_str().unwrap(),
    ]);
    assert_eq!(out.status.code(), Some(0));
    assert!(String::from_utf8(out.stdout)
        .unwrap()
        .contains("0 mismatch"));
}

#[test]
fn a_report_with_real_errors_exits_two_not_zero() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("broken.knxproj");
    write_knxproj_with_duplicate_id(&file);

    let out = run_cli(&["import", file.to_str().unwrap(), "--no-product-db"]);

    // 2, not 1: the project did import — a caller can tell "imported with
    // known holes" from "could not import at all".
    assert_eq!(out.status.code(), Some(2));
    let text = String::from_utf8(out.stdout).unwrap();
    assert!(text.contains("1 error(s)"), "stdout was: {text}");
}

/// Two valid one-installation projects that differ in their group addresses.
fn two_distinct_projects(dir: &Path) -> (PathBuf, PathBuf) {
    let two = DUPLICATE_ID_TOPOLOGY.replace(
        r#"<GroupAddress Id="P-0001-0_GA-1" Address="2" Name="GA2" />"#,
        r#"<GroupAddress Id="P-0001-0_GA-2" Address="2" Name="GA2" />"#,
    );
    let one = DUPLICATE_ID_TOPOLOGY.replace(
        r#"<GroupAddress Id="P-0001-0_GA-1" Address="2" Name="GA2" />"#,
        "",
    );
    let (a, b) = (dir.join("two.knxproj"), dir.join("one.knxproj"));
    write_knxproj_topology(&a, &two);
    write_knxproj_topology(&b, &one);
    (a, b)
}

fn stored_group_addresses(store: &Path) -> usize {
    let conn = knx_store::open_and_migrate(store).unwrap();
    knx_store::load_project(&conn).unwrap().installations[0]
        .group_addresses
        .len()
}

#[test]
fn an_occupied_store_is_refused_without_replace_and_left_untouched() {
    // AR18 independent review F4: a second import into the same --store used
    // to replace the project in it without a word.
    let dir = tempfile::tempdir().unwrap();
    let (two, one) = two_distinct_projects(dir.path());
    let store = dir.path().join("project.knxdb");
    let first = run_cli(&[
        "import",
        two.to_str().unwrap(),
        "--store",
        store.to_str().unwrap(),
        "--no-product-db",
    ]);
    assert_eq!(
        first.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&first.stderr)
    );
    let before = std::fs::read(&store).unwrap();

    let second = run_cli(&[
        "import",
        one.to_str().unwrap(),
        "--store",
        store.to_str().unwrap(),
        "--no-product-db",
    ]);
    assert_eq!(second.status.code(), Some(1));
    assert!(second.stdout.is_empty());
    let stderr = String::from_utf8(second.stderr).unwrap();
    assert!(
        stderr.contains("already exists") && stderr.contains("--replace"),
        "{stderr}"
    );
    assert!(
        std::fs::read(&store).unwrap() == before,
        "the store must be byte-identical"
    );
    assert_eq!(stored_group_addresses(&store), 2);
}

#[test]
fn replace_overwrites_an_occupied_store_on_request() {
    let dir = tempfile::tempdir().unwrap();
    let (two, one) = two_distinct_projects(dir.path());
    let store = dir.path().join("project.knxdb");
    let s = store.to_str().unwrap();
    assert_eq!(
        run_cli(&[
            "import",
            two.to_str().unwrap(),
            "--store",
            s,
            "--no-product-db"
        ])
        .status
        .code(),
        Some(0)
    );
    let out = run_cli(&[
        "import",
        one.to_str().unwrap(),
        "--store",
        s,
        "--no-product-db",
        "--replace",
    ]);
    assert_eq!(
        out.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(stored_group_addresses(&store), 1);
}

#[test]
fn replace_without_store_is_a_usage_error() {
    let dir = tempfile::tempdir().unwrap();
    let (two, _) = two_distinct_projects(dir.path());
    let out = run_cli(&[
        "import",
        two.to_str().unwrap(),
        "--replace",
        "--no-product-db",
    ]);
    assert_eq!(out.status.code(), Some(1));
    assert!(String::from_utf8(out.stderr)
        .unwrap()
        .contains("--replace needs --store"));
}

#[test]
fn a_failed_import_creates_no_store_file() {
    // AR18 review M1a: the store used to be created and migrated before
    // the import had a chance to fail.
    let dir = tempfile::tempdir().unwrap();
    let broken = dir.path().join("broken.knxproj");
    std::fs::write(&broken, b"not a zip").unwrap();
    let store = dir.path().join("never.knxdb");
    let out = run_cli(&[
        "import",
        broken.to_str().unwrap(),
        "--store",
        store.to_str().unwrap(),
        "--no-product-db",
    ]);
    assert_eq!(out.status.code(), Some(1));
    assert!(
        !store.exists(),
        "a failed import must not leave a store behind"
    );
}

#[test]
fn a_failed_replace_leaves_the_existing_store_byte_identical() {
    let dir = tempfile::tempdir().unwrap();
    let (two, _) = two_distinct_projects(dir.path());
    let store = dir.path().join("project.knxdb");
    let s = store.to_str().unwrap();
    assert_eq!(
        run_cli(&[
            "import",
            two.to_str().unwrap(),
            "--store",
            s,
            "--no-product-db"
        ])
        .status
        .code(),
        Some(0)
    );
    let before = std::fs::read(&store).unwrap();
    let broken = dir.path().join("broken.knxproj");
    std::fs::write(&broken, b"not a zip").unwrap();
    let out = run_cli(&[
        "import",
        broken.to_str().unwrap(),
        "--store",
        s,
        "--no-product-db",
        "--replace",
    ]);
    assert_eq!(out.status.code(), Some(1));
    assert!(std::fs::read(&store).unwrap() == before);
}

#[test]
fn readers_never_create_a_file_at_a_mistyped_path() {
    // AR18 review M1d.
    let dir = tempfile::tempdir().unwrap();
    let typo = dir.path().join("typo.knxdb");
    for args in [
        vec!["doc-export", typo.to_str().unwrap(), "out.html"],
        vec!["ga-export", typo.to_str().unwrap(), "out.csv"],
    ] {
        let out = run_cli(&args);
        assert_eq!(out.status.code(), Some(1), "{args:?}");
        assert!(!typo.exists(), "{args:?} created {}", typo.display());
    }
}

#[test]
fn readers_leave_a_foreign_sqlite_file_untouched() {
    // AR18 review M1b: a foreign database used to get 22 KNXBench tables.
    let dir = tempfile::tempdir().unwrap();
    let foreign = dir.path().join("notes.sqlite");
    knx_store::Connection::open(&foreign)
        .unwrap()
        .execute_batch("CREATE TABLE notes (body TEXT); INSERT INTO notes VALUES ('mine');")
        .unwrap();
    let before = std::fs::read(&foreign).unwrap();
    let out_html = dir.path().join("out.html");
    let out = run_cli(&[
        "doc-export",
        foreign.to_str().unwrap(),
        out_html.to_str().unwrap(),
    ]);
    assert_eq!(out.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&out.stderr).contains("not created by KNXBench"));
    assert!(std::fs::read(&foreign).unwrap() == before);
}
