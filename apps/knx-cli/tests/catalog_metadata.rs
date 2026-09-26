//! `products show` prints catalogue source values without claiming security.
use std::process::Command;

#[test]
fn products_show_displays_source_catalogue_metadata_without_secure_claim() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("products.sqlite");
    let conn = knx_productdb::open_and_migrate(&db).unwrap();
    let xml = br#"<KNX xmlns="http://knx.org/xml/project/11"><ManufacturerData><Manufacturer RefId="M-0001"><ApplicationPrograms><ApplicationProgram Id="A-1" IsSecureEnabled="maybe" MinEtsVersion="" ReplacesVersions="1,2"/></ApplicationPrograms></Manufacturer></ManufacturerData></KNX>"#;
    knx_productdb::ingest_file(&conn, "M-0001/Program.xml", xml).unwrap();
    drop(conn);
    let output = Command::new(env!("CARGO_BIN_EXE_knx"))
        .args(["products", "show", "A-1", "--product-db"])
        .arg(&db)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(
        stdout.contains("source catalogue metadata (uninterpreted"),
        "{stdout}"
    );
    assert!(stdout.contains("IsSecureEnabled: maybe"), "{stdout}");
    assert!(stdout.contains("MinEtsVersion: "), "{stdout}");
    assert!(stdout.contains("ReplacesVersions: 1,2"), "{stdout}");
    assert!(!stdout.contains("MaxUserEntries:"), "{stdout}");
}
