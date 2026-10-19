//! Synthetic app-import evidence separates opaque payload role and content.
//!
//! A role is an unverified name/location candidate, never ETS-App validation.

#[test]
fn project_and_manufacturer_payloads_report_content_without_certifying_roles() {
    let topology = br#"<KNX xmlns="http://knx.org/xml/project/23"><Project Id="P-TEST"><Installations><Installation InstallationId="0"><Topology/><GroupAddresses/></Installation></Installations></Project></KNX>"#;
    let info = br#"<KNX xmlns="http://knx.org/xml/project/23"><Project Id="P-TEST"><ProjectInformation Name="Synthetic payload evidence"/></Project></KNX>"#;
    let zip = knx_testsupport::zip_with_entries(&[("synthetic-only.txt", b"never extracted")]);
    let mut pe = vec![0u8; 132];
    pe[..2].copy_from_slice(b"MZ");
    pe[0x3c..0x40].copy_from_slice(&128u32.to_le_bytes());
    pe[128..].copy_from_slice(b"PE\0\0");
    let cases: Vec<(&str, &[u8], &str, Option<&str>)> = vec![
        (
            "P-TEST/UserFiles/opaque.etsapp",
            &zip,
            "zip",
            Some("ETS-app-named candidate (identity unverified)"),
        ),
        (
            "P-TEST/AddinData/picture.dat",
            b"\x89PNG\r\n\x1a\nsynthetic",
            "png",
            Some("add-in-state candidate (semantics unverified)"),
        ),
        (
            "P-TEST/AddInData/combined.ETSAPP",
            &zip,
            "zip",
            Some("ETS-app-named candidate (identity unverified)"),
        ),
        ("P-TEST/UserFiles/code.png", &pe, "pe-executable", None),
        (
            "P-TEST/UserFiles/pretend.dll",
            b"MZ is only text",
            "unknown",
            None,
        ),
        (
            "P-TEST/UserFiles/pretend.etsapp",
            b"not an archive",
            "unknown",
            Some("ETS-app-named candidate (identity unverified)"),
        ),
        (
            "M-TEST/Baggages/picture.dll",
            b"\x89PNG\r\n\x1a\nsynthetic",
            "png",
            None,
        ),
        (
            "M-TEST/Baggages/opaque.etsapp",
            &zip,
            "zip",
            Some("ETS-app-named candidate (identity unverified)"),
        ),
        (
            "M-TEST/Baggages/opaque.dat",
            b"synthetic unknown bytes",
            "unknown",
            None,
        ),
    ];
    for use_products in [false, true] {
        let mut entries = vec![
            ("P-TEST.signature", &b""[..]),
            ("P-TEST/0.xml", &topology[..]),
            ("P-TEST/project.xml", &info[..]),
        ];
        entries.extend(cases.iter().map(|(path, bytes, _, _)| (*path, *bytes)));
        let dir = tempfile::tempdir().unwrap();
        let native = dir.path().join("native.knxdb");
        let conn = knx_store::open_and_migrate(&native).unwrap();
        let products =
            knx_productdb::open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
        let imported = knx_app::import_ets_project_bytes(
            knx_testsupport::zip_with_entries(&entries),
            "synthetic.knxproj",
            &conn,
            knx_app::ImportOptions {
                product_db: use_products.then_some(&products),
            },
        )
        .unwrap();
        assert_eq!(imported.report.error_count(), 0);
        let opaque = knx_store::load_opaque(&conn).unwrap();
        for (path, bytes, class, role) in &cases {
            let summary = imported
                .report
                .opaque
                .iter()
                .find(|row| row.source_path == *path && row.xpath.is_empty())
                .unwrap();
            assert!(
                summary.reason.contains(&format!("payload ({class})")),
                "content must follow bytes, not extension: {path}"
            );
            if let Some(role) = role {
                assert!(
                    summary.reason.contains(role),
                    "candidate role must be marked unverified: {path}"
                );
            }
            if path.to_ascii_lowercase().contains("/addindata/") {
                assert!(summary
                    .reason
                    .contains("add-in-state candidate (semantics unverified)"));
            }
            assert!(summary.reason.contains("retained byte-exact"));
            assert!(summary.reason.contains("not rendered or executed"));
            assert!(summary.reason.contains("not unpacked"));
            assert!(!summary.reason.contains("vendor plugin binary"));
            let feature = imported
                .report
                .unsupported
                .iter()
                .find(|row| row.what == *path)
                .unwrap();
            assert_eq!(feature.consequence, summary.reason);
            if use_products && path.starts_with("M-") {
                let hash = knx_etsproj::opaque::sha256_hex(bytes);
                assert_eq!(
                    knx_productdb::load_source_file(&products, &hash)
                        .unwrap()
                        .as_deref(),
                    Some(*bytes)
                );
            } else {
                assert!(opaque
                    .iter()
                    .any(|row| row.source_path == *path && row.bytes == *bytes));
            }
        }
        knx_store::save_project(&conn, &imported.project).unwrap();
        drop(conn);
        let conn = knx_store::open_existing_and_migrate(&native).unwrap();
        assert_eq!(knx_store::load_opaque(&conn).unwrap(), opaque);
    }
}
