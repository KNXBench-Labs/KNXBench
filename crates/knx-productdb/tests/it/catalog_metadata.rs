//! Source-value catalogue metadata remains distinguishable from runtime security support.
use knx_productdb::{ingest_file, open_and_migrate, query};

fn program(attrs: &str) -> String {
    format!(
        "<KNX xmlns=\"http://knx.org/xml/project/11\"><ManufacturerData><Manufacturer RefId=\"M-0001\"><ApplicationPrograms><ApplicationProgram Id=\"A-1\" Name=\"Sample\" {attrs}/></ApplicationPrograms></Manufacturer></ManufacturerData></KNX>"
    )
}

#[test]
fn direct_ingest_rejects_malformed_document_before_catalogue_metadata() {
    let base = program("IsSecureEnabled=\"true\"");
    let malformed = [
        format!("{base}unexpected"),
        base.replace(
            "<ApplicationPrograms>",
            "<ApplicationPrograms><Other>&notDeclared;</Other>",
        ),
        base.replace(
            "<ApplicationPrograms>",
            "<ApplicationPrograms><Other bad=\"1\" bad=\"2\"/>",
        ),
        base.replace(
            "<ApplicationPrograms>",
            "<ApplicationPrograms><!-- invalid -- comment -->",
        ),
        format!("{base}<!DOCTYPE KNX>"),
    ];
    for bytes in malformed {
        let dir = tempfile::tempdir().unwrap();
        let conn = open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
        assert!(ingest_file(&conn, "M-0001/Program.xml", bytes.as_bytes()).is_err());
        assert!(query::programs(&conn, Some("M-0001")).unwrap().is_empty());
        let sources: i64 = conn
            .query_row("SELECT count(*) FROM source_file", [], |r| r.get(0))
            .unwrap();
        assert_eq!(sources, 0, "direct ingest retained an invalid source");
    }
}

#[test]
fn catalogue_exposes_source_lexemes_without_inventing_secure_capability() {
    let dir = tempfile::tempdir().unwrap();
    let conn = open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
    ingest_file(
        &conn,
        "M-0001/Program.xml",
        program(
            "IsSecureEnabled=\"maybe\" MaxSecurityGroupKeyTableEntries=\"0\" MaxSecurityIndividualAddressEntries=\"17\" MaxSecurityP2PKeyTableEntries=\"8\" MaxTunnelingUserEntries=\"4\" MaxUserEntries=\"6\" MinEtsVersion=\"\" ReplacesVersions=\"1, 2; x\"",
        )
        .as_bytes(),
    )
    .unwrap();
    let row = query::programs(&conn, Some("M-0001")).unwrap().remove(0);
    assert_eq!(row.is_secure_enabled.as_deref(), Some("maybe"));
    assert_eq!(
        row.max_security_group_key_table_entries.as_deref(),
        Some("0")
    );
    assert_eq!(
        row.max_security_individual_address_entries.as_deref(),
        Some("17")
    );
    assert_eq!(row.max_security_p2p_key_table_entries.as_deref(), Some("8"));
    assert_eq!(row.max_tunneling_user_entries.as_deref(), Some("4"));
    assert_eq!(row.max_user_entries.as_deref(), Some("6"));
    assert_eq!(row.min_ets_version.as_deref(), Some(""));
    assert_eq!(row.replaces_versions.as_deref(), Some("1, 2; x"));

    // Freshly persisted values cannot also be reported as unknown.
    let unknown: i64 = conn
        .query_row(
            "SELECT count(*) FROM ingest_unknown WHERE name IN ('IsSecureEnabled', 'MinEtsVersion', 'ReplacesVersions')",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(unknown, 0);
}

#[test]
fn missing_and_losing_duplicate_do_not_fabricate_or_overwrite_metadata() {
    let dir = tempfile::tempdir().unwrap();
    let conn = open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
    ingest_file(&conn, "M-0001/first.xml", program("").as_bytes()).unwrap();
    ingest_file(
        &conn,
        "M-0001/second.xml",
        program("IsSecureEnabled=\"true\" MinEtsVersion=\"6.0\"").as_bytes(),
    )
    .unwrap();
    let row = query::programs(&conn, Some("M-0001")).unwrap().remove(0);
    assert_eq!(row.is_secure_enabled, None);
    assert_eq!(row.min_ets_version, None);
}

#[test]
fn qualified_lookalikes_remain_unknown_and_cannot_supply_catalogue_values() {
    let dir = tempfile::tempdir().unwrap();
    let conn = open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
    let xml = program(
        "xmlns:fake=\"urn:foreign\" fake:IsSecureEnabled=\"true\" fake:MinEtsVersion=\"999\"",
    );
    ingest_file(&conn, "M-0001/Program.xml", xml.as_bytes()).unwrap();
    let row = query::programs(&conn, Some("M-0001")).unwrap().remove(0);
    assert_eq!(row.is_secure_enabled, None);
    assert_eq!(row.min_ets_version, None);
    let unknown: i64 = conn
        .query_row(
            "SELECT count(*) FROM ingest_unknown WHERE name IN ('fake:IsSecureEnabled', 'fake:MinEtsVersion')",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(unknown, 2);
}

#[test]
fn same_blob_duplicate_program_id_keeps_first_catalogue_value() {
    let dir = tempfile::tempdir().unwrap();
    let conn = open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
    let xml = br#"<KNX><ManufacturerData><Manufacturer RefId="M-0001"><ApplicationPrograms><ApplicationProgram Id="A-1" IsSecureEnabled="false"/><ApplicationProgram Id="A-1" IsSecureEnabled="true"/></ApplicationPrograms></Manufacturer></ManufacturerData></KNX>"#;
    ingest_file(&conn, "M-0001/Program.xml", xml).unwrap();
    let row = query::programs(&conn, Some("M-0001")).unwrap().remove(0);
    assert_eq!(row.is_secure_enabled.as_deref(), Some("false"));
}
