//! Verifies literal metadata completeness, safe relations and retained-master name fallback.
use super::*;

fn db() -> (tempfile::TempDir, Connection) {
    let dir = tempfile::tempdir().unwrap();
    let conn = crate::open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
    conn.execute_batch(include_str!("../../tests/fixtures/product_details.sql"))
        .unwrap();
    (dir, conn)
}

fn master(conn: &Connection, filename: &str, names: &str) {
    let xml = format!("<KNX><MasterData><Manufacturers>{names}</Manufacturers></MasterData></KNX>");
    crate::ingest_file(conn, filename, xml.as_bytes()).unwrap();
}

#[test]
fn names_resolve_existing_null_and_id_placeholders_without_changing_real_metadata() {
    let (_dir, conn) = db();
    master(
        &conn,
        "knx_master.xml",
        "<Manufacturer Id='M-0083' Name='Fictional Devices'/>",
    );
    for placeholder in [None, Some(""), Some("M-0083"), Some("0083")] {
        assert_eq!(
            manufacturer_name(&conn, "M-0083", placeholder)
                .unwrap()
                .as_deref(),
            Some("Fictional Devices")
        );
    }
    assert_eq!(
        manufacturer_name(&conn, "M-0083", Some("Installed Name"))
            .unwrap()
            .as_deref(),
        Some("Installed Name")
    );
    assert_eq!(manufacturer_name(&conn, "M-FFFF", None).unwrap(), None);
    assert_eq!(manufacturer_name(&conn, "M-83", None).unwrap(), None);
    let row = crate::query::device_product(&conn, "P-rich", "HP-rich", None)
        .unwrap()
        .unwrap();
    assert_eq!(row.manufacturer_name.as_deref(), Some("Fictional Devices"));
    let unchanged: String = conn
        .query_row("SELECT name FROM manufacturer WHERE id='M-0083'", [], |r| {
            r.get(0)
        })
        .unwrap();
    assert_eq!(unchanged, "M-0083");
}

#[test]
fn duplicate_names_agree_but_conflicting_or_malformed_master_evidence_does_not_guess() {
    let (_dir, conn) = db();
    master(
        &conn,
        "one/knx_master.xml",
        "<Manufacturer Id='M-0083' Name='One'/><Manufacturer Id='M-0083' Name='One'/>",
    );
    assert_eq!(
        manufacturer_name(&conn, "M-0083", None).unwrap().as_deref(),
        Some("One")
    );
    master(
        &conn,
        "two/knx_master.xml",
        "<Manufacturer Id='M-0083' Name='Two'/>",
    );
    assert_eq!(manufacturer_name(&conn, "M-0083", None).unwrap(), None);
    assert_eq!(
        manufacturer_name(&conn, "M-0083", Some("Real"))
            .unwrap()
            .as_deref(),
        Some("Real")
    );
    conn.execute(
        "UPDATE source_file SET bytes=X'3C' WHERE source_path LIKE '%knx_master.xml'",
        [],
    )
    .unwrap();
    assert_eq!(manufacturer_name(&conn, "M-0083", None).unwrap(), None);
}

#[test]
fn well_formed_but_changed_master_bytes_are_not_name_evidence() {
    let (_dir, conn) = db();
    master(
        &conn,
        "knx_master.xml",
        "<Manufacturer Id='M-0083' Name='One'/>",
    );
    conn.execute("UPDATE source_file SET bytes=CAST(replace(CAST(bytes AS TEXT),'One','Two') AS BLOB) WHERE source_path='knx_master.xml'", []).unwrap();
    assert_eq!(manufacturer_name(&conn, "M-0083", None).unwrap(), None);
    assert_eq!(
        manufacturer_name(&conn, "M-0083", Some("Real Name"))
            .unwrap()
            .as_deref(),
        Some("Real Name")
    );
}

#[test]
fn all_normalized_columns_and_every_placement_are_preserved_without_binary_or_local_paths() {
    let (_dir, conn) = db();
    let details = device_product_details(&conn, "P-rich", "HP-rich").unwrap();
    for (kind, table) in [
        ("product", "product"),
        ("hardware", "hardware"),
        ("programLink", "hardware2program"),
        ("application", "application_program"),
        ("manufacturer", "manufacturer"),
    ] {
        let section = details.iter().find(|s| s.kind == kind).unwrap();
        let stmt = conn.prepare(&format!("SELECT * FROM {table}")).unwrap();
        let keys: Vec<_> = stmt
            .column_names()
            .into_iter()
            .filter(|key| *key != "write_authority_recorded")
            .map(str::to_owned)
            .collect();
        assert_eq!(
            section
                .fields
                .iter()
                .map(|f| f.key.clone())
                .collect::<Vec<_>>(),
            keys,
            "{table} metadata must not silently shrink"
        );
    }
    assert_eq!(
        details
            .iter()
            .filter(|s| s.kind == "catalogItem")
            .map(|s| s.id.as_str())
            .collect::<Vec<_>>(),
        vec!["CI-a", "CI-b"]
    );
    assert_eq!(
        details
            .iter()
            .filter(|s| s.kind == "catalogSection")
            .count(),
        2
    );
    let source = details.iter().find(|s| s.kind == "source").unwrap();
    assert!(source
        .fields
        .iter()
        .any(|f| f.key == "source_name" && f.value == "fixture.xml"));
    assert!(details.iter().any(|s| s.kind == "additionalAttributes"
        && s.fields
            .iter()
            .any(|f| f.value == "FutureHardwareAttribute")));
    assert!(details
        .iter()
        .flat_map(|s| &s.fields)
        .all(|f| f.key != "bytes" && !f.value.contains("/home/alice")));
    let package = details.iter().find(|s| s.kind == "package").unwrap();
    assert_eq!(
        package
            .fields
            .iter()
            .map(|f| (f.key.as_str(), f.value.as_str()))
            .collect::<Vec<_>>(),
        vec![
            ("sha256", "fixture-package"),
            ("source_name", "fixture.knxprod"),
            ("scheme", "11"),
            ("size", "3")
        ]
    );
    assert!(source
        .fields
        .iter()
        .any(|f| f.key == "source_name" && f.value == "vendor.zip!/fixture.knxprod"));
    assert!(device_product_details(&conn, "missing", "HP-rich")
        .unwrap()
        .is_empty());
}

#[test]
fn missing_optional_values_cycles_and_mismatched_programs_stay_partial() {
    let (_dir, conn) = db();
    conn.execute_batch("UPDATE hardware SET bus_current=NULL; UPDATE product SET text=''; UPDATE catalog_section SET parent_id='CS-child' WHERE id='CS-root'; UPDATE hardware2program SET hardware_id='foreign' WHERE id='HP-rich';").unwrap();
    let details = device_product_details(&conn, "P-rich", "HP-rich").unwrap();
    assert!(!details.iter().any(|s| matches!(
        s.kind.as_str(),
        "application" | "programLink" | "catalogItem"
    )));
    assert!(!details
        .iter()
        .flat_map(|s| &s.fields)
        .any(|f| f.key == "bus_current"));
    assert!(details
        .iter()
        .flat_map(|s| &s.fields)
        .any(|f| f.key == "text" && f.value.is_empty()));
    conn.execute("UPDATE hardware2program SET hardware_id='H-rich'", [])
        .unwrap();
    assert_eq!(
        device_product_details(&conn, "P-rich", "HP-rich")
            .unwrap()
            .iter()
            .filter(|s| s.kind == "catalogSection")
            .count(),
        2
    );
}
