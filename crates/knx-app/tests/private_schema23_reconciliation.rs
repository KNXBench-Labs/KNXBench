//! Explicit private reconciliation of reported unknowns to retained source bytes.
//!
//! Owner decision: reported, byte-exact retained-uninterpreted data may count as
//! reconciled for KL-1, but does not count as interpreted semantics. Public
//! controls use independent synthetic data; private assertions never print values.

use std::collections::BTreeMap;
use std::io::Read;

// A single opened regular-file handle supplies both metadata and bounded bytes.
fn read_bounded_regular_file(path: &std::path::Path, limit: u64) -> std::io::Result<Vec<u8>> {
    let file = std::fs::File::open(path)?;
    let metadata = file.metadata()?;
    if !metadata.is_file() || metadata.len() > limit {
        return Err(std::io::Error::other("source exceeds regular-file budget"));
    }
    let bound = limit
        .checked_add(1)
        .ok_or_else(|| std::io::Error::other("source budget overflow"))?;
    let mut bytes = Vec::new();
    file.take(bound).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > limit {
        return Err(std::io::Error::other("source exceeds actual read budget"));
    }
    Ok(bytes)
}

fn reported_sources_retained(
    report: &knx_etsproj::ImportReport,
    original: &BTreeMap<String, Vec<u8>>,
    retained: &BTreeMap<String, Vec<u8>>,
) -> bool {
    report.unknown.iter().all(|finding| {
        let key = finding.source_path.to_ascii_lowercase();
        finding.occurrences > 0
            && !finding.name.is_empty()
            && !finding.xpath.is_empty()
            && original
                .get(&key)
                .is_some_and(|bytes| retained.get(&key) == Some(bytes))
    })
}

#[test]
fn bounded_source_reader_rejects_oversized_missing_and_non_regular_inputs() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("synthetic-only.zip");
    std::fs::write(&path, b"synthetic").unwrap();
    assert!(read_bounded_regular_file(&path, 9).unwrap() == b"synthetic");
    assert!(read_bounded_regular_file(&path, 8).is_err());
    assert!(read_bounded_regular_file(dir.path(), 9).is_err());
    assert!(read_bounded_regular_file(&dir.path().join("absent.zip"), 9).is_err());
}

#[test]
fn source_reconciliation_rejects_missing_corrupt_and_wrong_owner_evidence() {
    let xml = br#"<KNX xmlns="http://knx.org/xml/project/23"><Project Id="P-TEST"><Installations><Installation InstallationId="0"><Topology/><GroupAddresses/></Installation></Installations></Project></KNX>"#;
    let info = br#"<KNX xmlns="http://knx.org/xml/project/23"><Project Id="P-TEST"><ProjectInformation Name="Synthetic reconciliation" FutureMarker="synthetic-only"/></Project></KNX>"#;
    let bytes = knx_testsupport::zip_with_entries(&[
        ("P-TEST.signature", b""),
        ("P-TEST/0.xml", xml),
        ("P-TEST/project.xml", info),
    ]);
    let dir = tempfile::tempdir().unwrap();
    let conn = knx_store::open_and_migrate(&dir.path().join("native.knxdb")).unwrap();
    let imported = knx_app::import_ets_project_bytes(
        bytes,
        "synthetic.knxproj",
        &conn,
        knx_app::ImportOptions::default(),
    )
    .unwrap();
    assert!(imported.report.error_count() == 0);
    assert!(!imported.report.unknown.is_empty());
    assert!(imported.report.has_losses());
    assert!(imported.report.unknown.iter().any(|finding| {
        finding.name == "FutureMarker"
            && finding
                .source_path
                .eq_ignore_ascii_case("P-TEST/project.xml")
    }));
    let original = BTreeMap::from([
        ("p-test/project.xml".to_owned(), info.to_vec()),
        ("p-test/0.xml".to_owned(), xml.to_vec()),
    ]);
    assert!(reported_sources_retained(
        &imported.report,
        &original,
        &original
    ));
    assert!(!reported_sources_retained(
        &imported.report,
        &original,
        &BTreeMap::new()
    ));
    let mut corrupt = original.clone();
    corrupt.insert("p-test/project.xml".to_owned(), b"changed".to_vec());
    assert!(!reported_sources_retained(
        &imported.report,
        &original,
        &corrupt
    ));
    let mut wrong_owner = original.clone();
    let moved = wrong_owner.remove("p-test/project.xml").unwrap();
    wrong_owner.insert("p-other/project.xml".to_owned(), moved);
    assert!(!reported_sources_retained(
        &imported.report,
        &original,
        &wrong_owner
    ));
}

#[test]
#[ignore = "requires KNXBENCH_PRIVATE_SCHEMA23_PROJECT; boolean-only source reconciliation"]
fn private_original_unknown_report_is_backed_by_exact_retained_sources() {
    let source = std::env::var_os("KNXBENCH_PRIVATE_SCHEMA23_PROJECT")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| panic!("refusal: explicit private source required"));
    assert!(
        source.is_file(),
        "refusal: configured source is unavailable"
    );
    const LIMIT: u64 = 256 * 1024 * 1024;
    let input = read_bounded_regular_file(&source, 2 * LIMIT)
        .unwrap_or_else(|_| panic!("refusal: private source cannot be read within budget"));
    let original_digest = knx_etsproj::opaque::sha256_hex(&input);
    let mut zip = zip::ZipArchive::new(std::io::Cursor::new(&input))
        .unwrap_or_else(|_| panic!("private source is not a readable archive"));
    let mut original = BTreeMap::new();
    let mut total = 0u64;
    for index in 0..zip.len() {
        let mut entry = zip
            .by_index(index)
            .unwrap_or_else(|_| panic!("private member cannot be read"));
        if !entry.name().to_ascii_lowercase().ends_with(".xml") {
            continue;
        }
        assert!(entry.size() <= LIMIT, "private XML exceeds member budget");
        assert!(
            total
                .checked_add(entry.size())
                .is_some_and(|n| n <= 2 * LIMIT),
            "private XML exceeds cumulative budget"
        );
        let name = entry.name().to_ascii_lowercase();
        let mut bytes = Vec::new();
        (&mut entry)
            .take(LIMIT + 1)
            .read_to_end(&mut bytes)
            .unwrap_or_else(|_| panic!("private XML read failed"));
        assert!(bytes.len() as u64 <= LIMIT, "private XML budget exceeded");
        total = total
            .checked_add(bytes.len() as u64)
            .unwrap_or_else(|| panic!("private XML budget overflow"));
        assert!(total <= 2 * LIMIT, "private XML exceeds cumulative budget");
        assert!(
            original.insert(name, bytes).is_none(),
            "private XML source identity is ambiguous"
        );
    }
    drop(zip);
    let evidence = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../.ai/logs/private-import-analysis");
    std::fs::create_dir_all(&evidence)
        .unwrap_or_else(|_| panic!("private reconciliation operation refused"));
    let dir = tempfile::Builder::new()
        .prefix("source-reconciliation-")
        .tempdir_in(evidence)
        .unwrap_or_else(|_| panic!("private reconciliation operation refused"));
    let native = dir.path().join("native.knxdb");
    let conn = knx_store::open_and_migrate(&native)
        .unwrap_or_else(|_| panic!("private reconciliation operation refused"));
    let products = knx_productdb::open_and_migrate(&dir.path().join("products.sqlite"))
        .unwrap_or_else(|_| panic!("private reconciliation operation refused"));
    let imported = knx_app::import_ets_project_bytes(
        input,
        "private-source.knxproj",
        &conn,
        knx_app::ImportOptions {
            product_db: Some(&products),
        },
    )
    .unwrap_or_else(|_| panic!("production project/product import refused"));
    assert!(imported.report.source.schema_version == 23);
    assert!(imported.report.error_count() == 0);
    assert!(imported.report.conflicts.is_empty());
    // Unknowns remain conservative semantic-loss diagnostics even when source
    // bytes reconcile under the owner-approved preservation-only contract.
    assert!(imported.report.has_losses());
    assert!(imported.report.source_observations.is_some());
    assert!(
        !imported.report.unknown.is_empty(),
        "no retained-unknown case exercised"
    );
    assert!(imported.enrichment.is_some());
    assert!(knx_productdb::verify(&products)
        .unwrap_or_else(|_| panic!("private reconciliation operation refused"))
        .is_empty());
    let opaque = knx_store::load_opaque(&conn)
        .unwrap_or_else(|_| panic!("private reconciliation operation refused"));
    let manifest = knx_store::load_manufacturer_refs(&conn)
        .unwrap_or_else(|_| panic!("private reconciliation operation refused"));
    assert!(!manifest.is_empty(), "manufacturer import not exercised");
    let mut retained = BTreeMap::new();
    for entry in &opaque {
        if entry.xpath.is_empty() && entry.source_path.to_ascii_lowercase().ends_with(".xml") {
            assert!(
                retained
                    .insert(entry.source_path.to_ascii_lowercase(), entry.bytes.clone())
                    .is_none(),
                "retained source identity is ambiguous"
            );
        }
    }
    for entry in &manifest {
        if entry.source_path.to_ascii_lowercase().ends_with(".xml") {
            let bytes = knx_productdb::load_source_file(&products, &entry.sha256)
                .unwrap_or_else(|_| panic!("private reconciliation operation refused"))
                .unwrap_or_else(|| panic!("reported manufacturer source is unavailable"));
            assert!(
                retained
                    .insert(entry.source_path.to_ascii_lowercase(), bytes)
                    .is_none(),
                "manufacturer source identity is ambiguous"
            );
        }
    }
    assert!(
        original
            .iter()
            .all(|(key, bytes)| retained.get(key) == Some(bytes)),
        "original XML source is missing or changed in project/product evidence"
    );
    assert!(
        reported_sources_retained(&imported.report, &original, &retained),
        "a reported unknown source is missing or changed in retained evidence"
    );
    knx_store::save_project(&conn, &imported.project)
        .unwrap_or_else(|_| panic!("private reconciliation operation refused"));
    drop(conn);
    let conn = knx_store::open_existing_and_migrate(&native)
        .unwrap_or_else(|_| panic!("private reconciliation operation refused"));
    assert!(
        knx_store::load_project(&conn)
            .unwrap_or_else(|_| panic!("private reconciliation operation refused"))
            == imported.project
    );
    assert!(
        knx_store::load_opaque(&conn)
            .unwrap_or_else(|_| panic!("private reconciliation operation refused"))
            == opaque
    );
    assert!(
        knx_store::load_manufacturer_refs(&conn)
            .unwrap_or_else(|_| panic!("private reconciliation operation refused"))
            == manifest
    );
    knx_store::save_project(&conn, &imported.project)
        .unwrap_or_else(|_| panic!("private reconciliation operation refused"));
    assert!(
        knx_store::load_opaque(&conn)
            .unwrap_or_else(|_| panic!("private reconciliation operation refused"))
            == opaque
    );
    assert!(
        knx_store::load_manufacturer_refs(&conn)
            .unwrap_or_else(|_| panic!("private reconciliation operation refused"))
            == manifest
    );
    assert!(
        knx_etsproj::opaque::sha256_hex(
            &read_bounded_regular_file(&source, 2 * LIMIT)
                .unwrap_or_else(|_| panic!("private reconciliation operation refused"))
        ) == original_digest,
        "private source changed during comparison"
    );
}
