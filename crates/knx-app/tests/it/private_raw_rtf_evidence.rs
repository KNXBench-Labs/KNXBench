//! Opt-in private gate: boolean-only raw candidate/native evidence comparisons.
//!
//! Original input, native output and logs stay in ignored private evidence.
use std::path::PathBuf;

#[test]
#[ignore = "requires explicit private schema-23 project; missing input refuses"]
fn private_raw_rtf_candidates_keep_full_native_evidence_and_only_literal_bounded_previews() {
    let path = PathBuf::from(
        std::env::var_os("KNXBENCH_PRIVATE_SCHEMA23_PROJECT")
            .unwrap_or_else(|| panic!("private input env missing; refusal")),
    );
    assert!(
        path.is_absolute() && path.is_file(),
        "private input unavailable; refusal"
    );
    let bytes = std::fs::read(path).unwrap_or_else(|_| panic!("private input read refused"));
    let dir = tempfile::tempdir().unwrap_or_else(|_| panic!("private temporary directory refused"));
    let native = dir.path().join("native.knxdb");
    let conn = knx_store::open_and_migrate(&native)
        .unwrap_or_else(|_| panic!("private native creation refused"));
    let imported = knx_app::import_ets_project_bytes(
        bytes,
        "private input",
        &conn,
        knx_app::ImportOptions::default(),
    )
    .unwrap_or_else(|_| panic!("private import refused"));
    assert!(
        imported.report.error_count() == 0,
        "private import contains errors"
    );
    let retained = knx_store::load_opaque(&conn)
        .unwrap_or_else(|_| panic!("private retained evidence read refused"));
    let candidates: Vec<_> = imported
        .report
        .opaque
        .iter()
        .filter_map(|row| row.raw_text_preview.as_ref().map(|preview| (row, preview)))
        .collect();
    assert!(
        !candidates.is_empty(),
        "private raw-header candidate evidence absent"
    );
    for (row, preview) in candidates {
        let entry = retained
            .iter()
            .find(|e| {
                e.source_path == row.source_path
                    && e.xpath == row.xpath
                    && e.name == row.name
                    && e.sha256 == row.sha256
            })
            .unwrap_or_else(|| panic!("private candidate owner/source mismatch"));
        let full = std::str::from_utf8(&entry.bytes)
            .unwrap_or_else(|_| panic!("private candidate encoding mismatch"));
        assert!(
            preview.format == "rtf-like",
            "private candidate label mismatch"
        );
        assert!(
            preview.value == full.chars().take(512).collect::<String>(),
            "private raw preview mismatch"
        );
        assert!(
            preview.truncated == (full.chars().count() > 512),
            "private preview truncation mismatch"
        );
        assert!(
            row.size == entry.bytes.len() as u64,
            "private full-source size mismatch"
        );
    }
    knx_store::save_project(&conn, &imported.project)
        .unwrap_or_else(|_| panic!("private project save refused"));
    drop(conn);
    let conn = knx_store::open_existing_and_migrate(&native)
        .unwrap_or_else(|_| panic!("private native reopen refused"));
    let loaded = knx_store::load_opaque(&conn)
        .unwrap_or_else(|_| panic!("private reopened evidence read refused"));
    assert!(
        loaded == retained,
        "private full retained source changed after reopen"
    );
}
