//! Drives the product identity, family, order-number and retry CLI output (ADR-0043).
//
// Runs `knx products identity|family|order-number` and the retry line of
// `knx products ingest` against packages written here.

use std::io::{Cursor, Write};
use std::path::Path;
use std::process::{Command, Output};

fn run_cli(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_knx"))
        .args(args)
        .output()
        .expect("failed to run the knx binary")
}

const MASTER: &str = r#"<KNX xmlns="http://knx.org/xml/project/11"><MasterData><Manufacturers><Manufacturer Id="M-0001" Name="Example"/></Manufacturers></MasterData></KNX>"#;

fn manufacturer(body: &str) -> String {
    format!(
        r#"<KNX xmlns="http://knx.org/xml/project/11"><ManufacturerData><Manufacturer RefId="M-0001">{body}</Manufacturer></ManufacturerData></KNX>"#
    )
}

fn write_package(path: &Path, members: &[(&str, String)]) {
    let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let options = zip::write::SimpleFileOptions::default();
    writer.start_file("knx_master.xml", options).unwrap();
    writer.write_all(MASTER.as_bytes()).unwrap();
    for (name, xml) in members {
        writer.start_file(*name, options).unwrap();
        writer.write_all(xml.as_bytes()).unwrap();
    }
    std::fs::write(path, writer.finish().unwrap().into_inner()).unwrap();
}

fn hardware(text: &str) -> String {
    manufacturer(&format!(
        r#"<Hardware><Hardware Id="M-0001_H-1"><Products><Product Id="M-0001_H-1_P-1" Text="{text}" OrderNumber="ON 1"/></Products><Hardware2Programs><Hardware2Program Id="M-0001_H-1_HP-1"><ApplicationProgramRef RefId="M-0001_A-0001-02-0000"/></Hardware2Program></Hardware2Programs></Hardware></Hardware>"#
    ))
}

fn programs() -> String {
    manufacturer(
        r#"<ApplicationPrograms><ApplicationProgram Id="M-0001_A-0001-02-0000" Name="P" ApplicationNumber="1" ApplicationVersion="2" ReplacesVersions="1 3"/></ApplicationPrograms>"#,
    )
}

fn ok_stdout(out: Output) -> String {
    assert_eq!(
        out.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).unwrap()
}

#[test]
fn identity_family_and_order_number_commands_report_what_was_installed() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("products.sqlite");
    let db = db.to_str().unwrap();
    let first = dir.path().join("first.knxprod");
    let second = dir.path().join("second.knxprod");
    let copy = dir.path().join("copy.knxprod");
    write_package(
        &first,
        &[
            ("M-0001/Hardware.xml", hardware("one")),
            ("M-0001/M-0001_A-0001-02-0000.xml", programs()),
        ],
    );
    write_package(&second, &[("M-0001/Hardware.xml", hardware("two"))]);
    std::fs::copy(&first, &copy).unwrap();

    let installed = ok_stdout(run_cli(&[
        "products",
        "ingest",
        first.to_str().unwrap(),
        "--product-db",
        db,
    ]));
    assert!(!installed.contains("already known"), "{installed}");
    ok_stdout(run_cli(&[
        "products",
        "ingest",
        second.to_str().unwrap(),
        "--product-db",
        db,
    ]));
    let retried = ok_stdout(run_cli(&[
        "products",
        "ingest",
        copy.to_str().unwrap(),
        "--product-db",
        db,
    ]));
    assert!(
        retried.contains("(already known; 2 source name(s))"),
        "{retried}"
    );

    let identity = ok_stdout(run_cli(&[
        "products",
        "identity",
        "product",
        "M-0001_H-1_P-1",
        "--product-db",
        db,
    ]));
    assert!(
        identity.starts_with("product M-0001_H-1_P-1\n"),
        "{identity}"
    );
    assert!(identity.contains("  2 candidate(s)"), "{identity}");
    assert_eq!(identity.matches("  same  ").count(), 1, "{identity}");
    assert_eq!(identity.matches("  differs  ").count(), 1, "{identity}");
    assert_eq!(
        identity
            .matches("1 package(s)  M-0001/Hardware.xml")
            .count(),
        2,
        "{identity}"
    );

    let family = ok_stdout(run_cli(&[
        "products",
        "family",
        "M-0001_A-0001-02-0000",
        "--product-db",
        db,
    ]));
    assert!(
        family.contains("family M-0001 ApplicationNumber 1: 1 member(s)"),
        "{family}"
    );
    assert!(family.contains("ReplacesVersions \"1 3\""), "{family}");
    assert!(family.contains("      1: not installed"), "{family}");
    assert!(family.contains("      3: not installed"), "{family}");

    let order = ok_stdout(run_cli(&[
        "products",
        "order-number",
        "M-0001",
        "ON 1",
        "--product-db",
        db,
    ]));
    assert!(
        order.contains("1 product(s) with order number \"ON 1\""),
        "{order}"
    );
    assert!(order.contains("programs: M-0001_A-0001-02-0000"), "{order}");
    assert!(order.contains("package scheme(s): 11"), "{order}");
}

#[test]
fn identity_rejects_an_unknown_table_and_family_an_unknown_program() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("products.sqlite");
    let db = db.to_str().unwrap();
    let out = run_cli(&["products", "identity", "device", "X", "--product-db", db]);
    assert_eq!(out.status.code(), Some(1));
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert!(
        stderr.contains("unknown identity table \"device\""),
        "{stderr}"
    );
    assert!(stderr.contains("application_program"), "{stderr}");

    let out = run_cli(&["products", "family", "A-9", "--product-db", db]);
    assert_eq!(out.status.code(), Some(1));
    assert!(String::from_utf8(out.stderr)
        .unwrap()
        .contains("no such program: A-9"));

    let out = run_cli(&["products", "order-number", "M-0001", "--product-db", db]);
    assert_eq!(out.status.code(), Some(1));
    assert!(String::from_utf8(out.stderr)
        .unwrap()
        .contains("expected <manufacturer-id> <order-number>"));

    let empty = ok_stdout(run_cli(&[
        "products",
        "identity",
        "hardware",
        "H-9",
        "--product-db",
        db,
    ]));
    assert!(empty.contains("winner: (no stored row)"), "{empty}");
    assert!(empty.contains("0 candidate(s)"), "{empty}");
}
