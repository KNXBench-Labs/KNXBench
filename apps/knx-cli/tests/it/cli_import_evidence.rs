//! Actual CLI native imports retain their final original report as separate evidence.

use std::path::Path;
use std::process::{Command, Output};

fn import(
    source: &Path,
    native: &Path,
    report: &Path,
    products: Option<&Path>,
    replace: bool,
) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_knx"));
    command
        .arg("import")
        .arg(source)
        .arg("--store")
        .arg(native)
        .arg("--report-json")
        .arg(report);
    if let Some(products) = products {
        command.arg("--product-db").arg(products);
    } else {
        command.arg("--no-product-db");
    }
    if replace {
        command.arg("--replace");
    }
    command
        .output()
        .expect("synthetic CLI import could not start")
}

#[test]
fn cli_native_copy_keeps_exact_final_report_with_and_without_product_database() {
    for use_products in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("fictional.knxproj");
        let native = dir.path().join("fictional.knxdb");
        let report = dir.path().join("report.json");
        let products = dir.path().join("products.sqlite");
        let input = knx_testsupport::minimal_knxproj_bytes();
        std::fs::write(&source, &input).unwrap();
        let result = import(
            &source,
            &native,
            &report,
            use_products.then_some(products.as_path()),
            false,
        );
        assert_eq!(
            result.status.code(),
            Some(0),
            "synthetic CLI import refused"
        );
        let expected = std::fs::read_to_string(&report).unwrap();
        let conn = knx_store::open_existing_read_only(&native).unwrap();
        let opaque = knx_store::load_opaque(&conn.conn).unwrap();
        let historical = knx_app::import_evidence::read(&opaque)
            .unwrap()
            .expect("CLI native copy lost the original report");
        assert_eq!(
            historical.report_json, expected,
            "native report differs from CLI report output"
        );
        assert_eq!(
            opaque
                .iter()
                .filter(|e| e.kind == "KnxBenchImportEvidence")
                .count(),
            1
        );
        let source_opaque_count = opaque
            .iter()
            .filter(|e| e.kind != "KnxBenchImportEvidence")
            .count();
        let stdout = String::from_utf8(result.stdout).unwrap();
        assert!(
            stdout.contains(&format!("{source_opaque_count} opaque entries stored")),
            "source counter was silently inflated by native metadata"
        );
        assert_eq!(std::fs::read(&source).unwrap(), input);
    }
}

#[test]
fn refused_cli_replacement_keeps_original_native_report_and_source_untouched() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("fictional.knxproj");
    let native = dir.path().join("fictional.knxdb");
    let report = dir.path().join("report.json");
    let input = knx_testsupport::minimal_knxproj_bytes();
    std::fs::write(&source, &input).unwrap();
    assert_eq!(
        import(&source, &native, &report, None, false).status.code(),
        Some(0)
    );
    let before = std::fs::read(&native).unwrap();
    let report_before = std::fs::read(&report).unwrap();
    assert_eq!(
        import(&source, &native, &report, None, false).status.code(),
        Some(1)
    );
    assert_eq!(std::fs::read(&native).unwrap(), before);
    assert_eq!(std::fs::read(&report).unwrap(), report_before);
    assert_eq!(std::fs::read(&source).unwrap(), input);
}

#[test]
fn explicit_cli_replacement_stores_one_primary_report_not_a_duplicate() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("fictional.knxproj");
    let native = dir.path().join("fictional.knxdb");
    let report = dir.path().join("report.json");
    std::fs::write(&source, knx_testsupport::minimal_knxproj_bytes()).unwrap();
    assert_eq!(
        import(&source, &native, &report, None, false).status.code(),
        Some(0)
    );
    assert_eq!(
        import(&source, &native, &report, None, true).status.code(),
        Some(0)
    );
    let conn = knx_store::open_existing_read_only(&native).unwrap();
    let entries = knx_store::load_opaque(&conn.conn).unwrap();
    assert_eq!(
        entries
            .iter()
            .filter(|e| e.kind == "KnxBenchImportEvidence")
            .count(),
        1
    );
    assert_eq!(
        knx_app::import_evidence::read(&entries)
            .unwrap()
            .unwrap()
            .report_json,
        std::fs::read_to_string(&report).unwrap()
    );
}
