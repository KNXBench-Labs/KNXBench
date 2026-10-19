//! Stored size preflight must inspect actual blob length, not trust stale metadata.
#[test]
fn source_length_is_measured_without_loading_the_blob_or_trusting_len() {
    let dir = tempfile::tempdir().unwrap();
    let c = knx_productdb::open_and_migrate(&dir.path().join("p.sqlite")).unwrap();
    let file = knx_productdb::SourceFile {
        source_path: "PRIVATE".into(),
        manufacturer_id: None,
        bytes: vec![1, 2, 3],
    };
    knx_productdb::store_source_file(&c, &file).unwrap();
    let hash = knx_productdb::sha256_hex(&file.bytes);
    c.execute("UPDATE source_file SET len=1 WHERE sha256=?1", [&hash])
        .unwrap();
    assert_eq!(
        knx_productdb::blob::source_file_length(&c, &hash).unwrap(),
        Some(3)
    );
    assert_eq!(
        knx_productdb::blob::source_file_length(&c, "absent").unwrap(),
        None
    );
}
