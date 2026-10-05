//! Real CLI product-package raw-size admission before reading or opening the product DB.
//!
//! Inputs are sparse synthetic files: their logical length crosses the existing
//! 256 MiB package bound without consuming that much disk space. No bus,
//! private corpus or manufacturer data is used.

use std::fs::File;
use std::path::Path;
use std::process::{Command, Output};

const PACKAGE_LIMIT: u64 = 256 * 1024 * 1024;
const MASTER: &[u8] = br#"<KNX xmlns="http://knx.org/xml/project/11"><MasterData><Manufacturers><Manufacturer Id="M-0001" Name="Example"/></Manufacturers></MasterData></KNX>"#;
const HARDWARE: &[u8] = br#"<KNX xmlns="http://knx.org/xml/project/11"><ManufacturerData><Manufacturer RefId="M-0001"><Hardware><Hardware Id="M-0001_H-1"><Products><Product Id="M-0001_H-1_P-1" Text="Synthetic switch" OrderNumber="SYNTHETIC-1"/></Products></Hardware></Hardware></Manufacturer></ManufacturerData></KNX>"#;

fn sparse_file(path: &Path, length: u64) {
    File::create(path).unwrap().set_len(length).unwrap();
    assert_eq!(std::fs::metadata(path).unwrap().len(), length);
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

fn refused_with_size_limit(output: &Output, input: &Path) {
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(1), "{stderr}");
    assert!(
        output.stdout.is_empty(),
        "refusal must not print an install"
    );
    assert!(
        stderr.contains(&format!(
            "product ZIP size limit exceeded: {}",
            input.display()
        )),
        "refusal must name the typed size limit and the caller file: {stderr}"
    );
}

#[test]
fn one_byte_over_package_limit_is_refused_before_any_product_db_is_created() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("over-limit.knxprod");
    let database = dir.path().join("products.sqlite");
    sparse_file(&input, PACKAGE_LIMIT + 1);
    let output = ingest(&input, &database, &dir.path().join("data"));
    refused_with_size_limit(&output, &input);
    assert!(
        !database.exists(),
        "an oversized input must be refused before the product DB is opened or created"
    );
}

#[test]
fn far_over_limit_sparse_input_is_refused_by_size_without_reading_it() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("huge.knxprod");
    let database = dir.path().join("products.sqlite");
    // 64 GiB logical length: reading it into memory would be a defect.
    sparse_file(&input, 64 * 1024 * 1024 * 1024);
    let output = ingest(&input, &database, &dir.path().join("data"));
    refused_with_size_limit(&output, &input);
    assert!(!database.exists());
}

#[test]
fn over_limit_input_leaves_an_existing_product_db_byte_identical() {
    let dir = tempfile::tempdir().unwrap();
    let database = dir.path().join("products.sqlite");
    let data_home = dir.path().join("data");
    let seed = dir.path().join("seed.knxprod");
    std::fs::write(
        &seed,
        knx_testsupport::zip_with_entries(&[
            ("knx_master.xml", MASTER),
            ("M-0001/Hardware.xml", HARDWARE),
        ]),
    )
    .unwrap();
    let seeded = ingest(&seed, &database, &data_home);
    assert_eq!(
        seeded.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&seeded.stderr)
    );
    let snapshot = |path: &Path| {
        let mut files = std::fs::read_dir(path.parent().unwrap())
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .filter(|entry| {
                entry
                    .file_name()
                    .unwrap()
                    .to_string_lossy()
                    .starts_with("products.sqlite")
            })
            .map(|entry| (entry.clone(), std::fs::read(entry).unwrap()))
            .collect::<Vec<_>>();
        files.sort();
        files
    };
    let before = snapshot(&database);
    assert!(!before.is_empty());
    let input = dir.path().join("over-limit.knxprod");
    sparse_file(&input, PACKAGE_LIMIT + 1);
    let output = ingest(&input, &database, &data_home);
    refused_with_size_limit(&output, &input);
    assert_eq!(
        snapshot(&database),
        before,
        "product DB files must stay byte-identical"
    );
}

#[test]
fn input_exactly_at_package_limit_is_read_and_validated_as_a_zip() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("at-limit.knxprod");
    sparse_file(&input, PACKAGE_LIMIT);
    let output = ingest(
        &input,
        &dir.path().join("products.sqlite"),
        &dir.path().join("data"),
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(1), "{stderr}");
    assert!(
        stderr.contains("missing complete end-of-directory record"),
        "the inclusive bound must reach real ZIP validation, not a size refusal: {stderr}"
    );
    assert!(!stderr.contains("size limit"), "{stderr}");
}
