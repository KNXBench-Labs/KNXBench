//! `knx products ingest` takes vendor ZIPs of product packages (ADR-0140).
//! Synthetic fixtures only: no corpus, manufacturer data or bus.

use std::io::{Cursor, Write};
use std::path::Path;
use std::process::{Command, Output};

use zip::write::SimpleFileOptions;

const MASTER: &[u8] = br#"<KNX xmlns="http://knx.org/xml/project/11"><MasterData><Manufacturers><Manufacturer Id="M-0001" Name="Example"/></Manufacturers></MasterData></KNX>"#;

fn hardware(n: u8) -> String {
    format!(
        r#"<KNX xmlns="http://knx.org/xml/project/11"><ManufacturerData><Manufacturer RefId="M-0001"><Hardware><Hardware Id="M-0001_H-{n}"><Products><Product Id="M-0001_H-{n}_P-1" Text="Synthetic switch {n}" OrderNumber="SYNTHETIC-{n}"/></Products></Hardware></Hardware></Manufacturer></ManufacturerData></KNX>"#
    )
}

fn zip_of(members: &[(&str, &[u8])]) -> Vec<u8> {
    let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for (name, bytes) in members {
        writer
            .start_file(*name, SimpleFileOptions::default())
            .unwrap();
        writer.write_all(bytes).unwrap();
    }
    writer.finish().unwrap().into_inner()
}

fn package(n: u8) -> Vec<u8> {
    let hardware = hardware(n);
    zip_of(&[
        ("knx_master.xml", MASTER),
        ("M-0001/Hardware.xml", hardware.as_bytes()),
    ])
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
fn a_vendor_zip_installs_both_inner_packages_and_names_the_manual() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("vendor.zip");
    std::fs::write(
        &input,
        zip_of(&[
            ("one.knxprod", &package(1)),
            ("Manual.pdf", b"%PDF-1.4 manual"),
            ("two/two.knxprod", &package(2)),
        ]),
    )
    .unwrap();
    let database = dir.path().join("products.sqlite");
    let output = ingest(
        &[
            input.as_os_str(),
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
        stdout.contains(
            "vendor ZIP: 2 product package(s) installed together, 1 other member(s) not installed"
        ),
        "{stdout}"
    );
    assert!(
        stdout.contains("inner package one.knxprod: package installed: scheme 11"),
        "{stdout}"
    );
    assert!(
        stdout.contains("inner package two/two.knxprod: package installed: scheme 11"),
        "{stdout}"
    );
    assert!(
        stdout.contains("not installed: Manual.pdf (15 bytes"),
        "{stdout}"
    );

    let conn = knx_productdb::open_and_migrate(&database).unwrap();
    let packages: i64 = conn
        .query_row("SELECT count(*) FROM package", [], |row| row.get(0))
        .unwrap();
    assert_eq!(packages, 2);
}

#[test]
fn a_plain_package_prints_the_line_it_always_did() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("switch.knxprod");
    std::fs::write(&input, package(1)).unwrap();
    let database = dir.path().join("products.sqlite");
    let output = ingest(
        &[
            input.as_os_str(),
            "--product-db".as_ref(),
            database.as_os_str(),
        ],
        &dir.path().join("data"),
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(output.status.code(), Some(0), "{stdout}");
    assert!(
        stdout.starts_with("package installed: scheme 11, 2 member(s)"),
        "{stdout}"
    );
    assert!(!stdout.contains("vendor ZIP"), "{stdout}");
}

#[test]
fn a_zip_holding_only_a_manual_names_the_absent_product_package() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("manual.zip");
    std::fs::write(&input, zip_of(&[("Manual.pdf", b"%PDF-1.4")])).unwrap();
    let database = dir.path().join("products.sqlite");
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
        stderr.contains("ZIP is not a standalone product package: missing knx_master.xml file"),
        "{stderr}"
    );
}

#[test]
fn macos_metadata_beside_a_package_is_named_and_not_installed() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("vendor.zip");
    let apple_double: &[u8] = b"\x00\x05\x16\x07\x00\x02\x00\x00Mac OS X        ";
    std::fs::write(
        &input,
        zip_of(&[
            ("one.knxprod", &package(1)),
            ("__MACOSX/._one.knxprod", apple_double),
        ]),
    )
    .unwrap();
    let database = dir.path().join("products.sqlite");
    let output = ingest(
        &[
            input.as_os_str(),
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
        stdout.contains(
            "vendor ZIP: 1 product package(s) installed together, 1 other member(s) not installed"
        ),
        "{stdout}"
    );
    assert!(
        stdout.contains("not installed: __MACOSX/._one.knxprod ("),
        "{stdout}"
    );
    assert!(stdout.contains(", macOS metadata)"), "{stdout}");
}
