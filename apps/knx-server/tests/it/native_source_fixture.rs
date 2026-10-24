//! Installs byte-exact synthetic originals for parameter/native-save fixtures.

/// Direct XML ingestion has no package ownership. Use the real installer so
/// saves exercise measured declarations, original bytes and package membership.
/// Source XML is supplied unchanged by each test, including its schema version.
pub fn install(
    products: &knx_productdb::Connection,
    schema: u32,
    hardware: (&str, &[u8]),
    program: (&str, &[u8]),
) {
    let master =
        format!(r#"<KNX xmlns="http://knx.org/xml/project/{schema}"><MasterData /></KNX>"#);
    let bytes = knx_testsupport::zip_with_entries(&[
        ("knx_master.xml", master.as_bytes()),
        hardware,
        program,
    ]);
    let report = knx_productdb::package::install_package(products, "parameters.knxprod", &bytes)
        .expect("the synthetic source package must install");
    assert!(!report.skipped);
    assert!(knx_productdb::catalogue_sources::has_catalogue_original(
        products,
        &knx_productdb::sha256_hex(&bytes),
    )
    .unwrap());
}

pub fn assert_carried(state: &knx_server::AppState, native: &std::path::Path) {
    let read = knx_store::open_existing_read_only(native).unwrap();
    let refs = knx_store::load_manufacturer_refs(&read.conn).unwrap();
    assert_eq!(refs.len(), 4, "original, master, hardware and application");
    assert_eq!(
        refs.iter().filter(|r| r.kind == "CataloguePackage").count(),
        1
    );
    state
        .product_db
        .as_ref()
        .unwrap()
        .with_read(|products| {
            for source in &refs {
                let expected = if source.kind == "CataloguePackage" {
                    knx_productdb::catalogue_sources::load_catalogue_original(
                        products,
                        &source.sha256,
                        knx_store::manufacturer_files::MAX_MANUFACTURER_FILE_BYTES,
                    )
                    .unwrap()
                    .unwrap()
                } else {
                    knx_productdb::load_source_file(products, &source.sha256)
                        .unwrap()
                        .unwrap()
                };
                let carried = knx_store::load_manufacturer_file(&read.conn, &source.sha256)
                    .unwrap()
                    .unwrap();
                assert_eq!(carried.bytes, expected);
            }
            Ok::<(), std::convert::Infallible>(())
        })
        .unwrap()
        .unwrap();
}
