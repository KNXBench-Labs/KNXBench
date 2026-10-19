//! Persisted diagnostic availability and corruption controls, synthetic only.
#[test]
fn absent_measurement_differs_from_measured_zero_and_corrupt_rows_fail() {
    let dir = tempfile::tempdir().unwrap();
    let conn = knx_productdb::open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
    assert_eq!(
        knx_productdb::source_diagnostics(&conn, "unavailable").unwrap(),
        None
    );
    let payload = b"opaque synthetic payload";
    let hash = knx_productdb::sha256_hex(payload);
    knx_productdb::ingest_file(&conn, "M-0001/Baggages/synthetic.dat", payload).unwrap();
    assert_eq!(
        knx_productdb::source_diagnostics(&conn, &hash).unwrap(),
        Some(vec![])
    );
    conn.execute("INSERT INTO ingest_unknown (source_sha256,program_id,xpath,kind,name,occurrences,sample) VALUES (?1,NULL,'/KNX','Attribute','Synthetic',1,'synthetic sample')", [&hash]).unwrap();
    assert_eq!(
        knx_productdb::source_diagnostics(&conn, &hash)
            .unwrap()
            .unwrap()[0]
            .occurrences,
        1
    );
    for (kind, count) in [
        ("FutureKind", 1),
        ("Attribute", -1),
        ("Attribute", 0),
        ("Attribute", i64::MAX),
    ] {
        // Parameters are supplied through the already public rusqlite re-export's
        // heterogeneous ToSql slice, without a new crate dependency.
        let args: &[&dyn rusqlite::ToSql] = &[&kind, &count, &hash];
        conn.execute(
            "UPDATE ingest_unknown SET kind=?1,occurrences=?2 WHERE source_sha256=?3",
            args,
        )
        .unwrap();
        assert!(knx_productdb::source_diagnostics(&conn, &hash).is_err());
    }
}
