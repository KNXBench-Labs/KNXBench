//! `knx products ingest --allow-large-package` opts into the large package profile (KL-151).
//!
//! The fixture is synthetic: a 65 MiB member of zeros that compresses to a
//! few KiB. No bus, private corpus or manufacturer data is used.

use std::io::{Cursor, Write};
use std::path::Path;
use std::process::{Command, Output};

use zip::write::SimpleFileOptions;

const MASTER: &[u8] = br#"<KNX xmlns="http://knx.org/xml/project/11"><MasterData><Manufacturers><Manufacturer Id="M-0001" Name="Example"/></Manufacturers></MasterData></KNX>"#;
const HARDWARE: &[u8] = br#"<KNX xmlns="http://knx.org/xml/project/11"><ManufacturerData><Manufacturer RefId="M-0001"><Hardware><Hardware Id="M-0001_H-1"><Products><Product Id="M-0001_H-1_P-1" Text="Synthetic switch" OrderNumber="SYNTHETIC-1"/></Products></Hardware></Hardware></Manufacturer></ManufacturerData></KNX>"#;

fn package_with_a_65_mib_member(path: &Path) {
    let payload = vec![0_u8; 65 * 1024 * 1024];
    let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    for (name, bytes) in [
        ("knx_master.xml", MASTER),
        ("M-0001/Hardware.xml", HARDWARE),
        ("M-0001/Baggages/large.bin", payload.as_slice()),
    ] {
        writer.start_file(name, options).unwrap();
        writer.write_all(bytes).unwrap();
    }
    std::fs::write(path, writer.finish().unwrap().into_inner()).unwrap();
}

fn ingest(args: &[&std::ffi::OsStr], data_home: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_knx"))
        .args(["products", "ingest"])
        .args(args)
        .env("XDG_DATA_HOME", data_home)
        .output()
        .expect("run the real CLI against a synthetic input")
}

#[test]
fn the_standard_profile_refuses_and_names_the_opt_in() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("bundle.knxprod");
    let database = dir.path().join("products.sqlite");
    package_with_a_65_mib_member(&input);
    let output = ingest(
        &[
            input.as_os_str(),
            "--product-db".as_ref(),
            database.as_os_str(),
        ],
        &dir.path().join("data"),
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(1), "{stderr}");
    assert!(
        stderr.contains("product ZIP size limit exceeded: M-0001/Baggages/large.bin"),
        "{stderr}"
    );
    assert!(stderr.contains("--allow-large-package"), "{stderr}");
    assert!(output.stdout.is_empty());
}

#[test]
fn the_opt_in_installs_the_same_package() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("bundle.knxprod");
    let database = dir.path().join("products.sqlite");
    package_with_a_65_mib_member(&input);
    let output = ingest(
        &[
            input.as_os_str(),
            "--allow-large-package".as_ref(),
            "--product-db".as_ref(),
            database.as_os_str(),
        ],
        &dir.path().join("data"),
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        stdout.contains("package installed: scheme 11, 3 member(s)"),
        "{stdout}"
    );
}

#[test]
fn the_opt_in_is_refused_for_a_project_file() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("project.knxproj");
    std::fs::write(&input, b"not read").unwrap();
    let database = dir.path().join("products.sqlite");
    let output = ingest(
        &[
            input.as_os_str(),
            "--allow-large-package".as_ref(),
            "--product-db".as_ref(),
            database.as_os_str(),
        ],
        &dir.path().join("data"),
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(1), "{stderr}");
    assert!(
        stderr.contains("--allow-large-package applies to .knxprod product packages only"),
        "{stderr}"
    );
    assert!(!database.exists(), "nothing is opened for a refused option");
}
