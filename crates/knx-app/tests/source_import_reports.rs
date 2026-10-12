//! Source-report access reuses retained selective imports without replacing primary evidence.

use knx_app::{import_evidence, selective_import};
use knx_store::StoredOpaqueEntry;

fn source_report() -> (Vec<StoredOpaqueEntry>, String, String) {
    let source =
        selective_import::source_from_bytes(knx_testsupport::minimal_knxproj_bytes(), None)
            .unwrap();
    let id = source.source_hash;
    let exact = source.report.to_json();
    let bytes = exact.as_bytes().to_vec();
    let entries = vec![StoredOpaqueEntry {
        source_path: format!("imports/{id}/import-report.json"),
        xpath: String::new(),
        kind: "SelectiveImportReport".into(),
        name: String::new(),
        sha256: knx_store::project_history::image_hash(&bytes),
        bytes,
    }];
    (entries, id, exact)
}

#[test]
fn retained_source_reports_have_an_explicit_catalogue_and_exact_separate_read() {
    let (entries, id, exact) = source_report();
    let original = entries.clone();
    let catalogue = import_evidence::source_reports::catalogue(&entries).unwrap();
    assert_eq!(
        catalogue.len(),
        1,
        "preserved source report was not discoverable"
    );
    assert_eq!(catalogue[0].id, id);
    assert_eq!(catalogue[0].status, "recorded");
    let report = import_evidence::source_reports::read(&entries, &id).unwrap();
    assert_eq!(report.report_json, exact);
    assert!(
        import_evidence::read(&entries).unwrap().is_none(),
        "source report became a primary report"
    );
    assert_eq!(entries, original);
    let store = knx_store::open_and_migrate_in_memory().unwrap();
    knx_store::insert_opaque(&store, &entries).unwrap();
    let reopened = knx_store::load_opaque(&store).unwrap();
    assert_eq!(
        import_evidence::source_reports::read(&reopened, &id)
            .unwrap()
            .report_json,
        exact
    );
}

#[test]
fn invalid_second_source_report_refuses_the_whole_catalogue_without_partial_success() {
    let (mut entries, _, _) = source_report();
    let mut bad = entries[0].clone();
    bad.source_path = format!("imports/{}/import-report.json", "f".repeat(64));
    bad.bytes[0] ^= 1;
    entries.push(bad);
    let original = entries.clone();
    assert!(import_evidence::source_reports::catalogue(&entries).is_err());
    assert_eq!(entries, original);
}

#[test]
fn invalid_identity_duplicate_future_and_budget_records_never_yield_a_partial_catalogue() {
    let (base, id, _) = source_report();
    let mut duplicate = base.clone();
    duplicate.push(base[0].clone());
    assert!(import_evidence::source_reports::catalogue(&duplicate).is_err());
    for (path, kind) in [
        (
            format!("imports/{}/import-report.json", id.to_uppercase()),
            "SelectiveImportReport",
        ),
        (
            "imports/not-a-hash/import-report.json".into(),
            "SelectiveImportReport",
        ),
        (base[0].source_path.clone(), "SelectiveImportReportFuture"),
    ] {
        let mut bad = base[0].clone();
        bad.source_path = path;
        bad.kind = kind.into();
        assert!(
            import_evidence::source_reports::catalogue(&[bad]).is_err(),
            "unsupported source report identity was silently omitted"
        );
    }
    let mut oversized = base[0].clone();
    oversized.bytes = vec![0; import_evidence::MAX_EVIDENCE_BYTES + 1];
    assert!(import_evidence::source_reports::catalogue(&[oversized]).is_err());
    let mut many = Vec::new();
    for index in 0..257 {
        let mut row = base[0].clone();
        row.source_path = format!("imports/{index:064x}/import-report.json");
        many.push(row);
    }
    assert!(import_evidence::source_reports::catalogue(&many)
        .unwrap_err()
        .contains("256"));
    let mut expanded = Vec::new();
    for index in 0..5 {
        let mut row = base[0].clone();
        row.source_path = format!("imports/{index:064x}/import-report.json");
        row.bytes.resize(13 * 1024 * 1024, b' ');
        expanded.push(row);
    }
    assert!(import_evidence::source_reports::catalogue(&expanded)
        .unwrap_err()
        .contains("64 MiB"));
    assert!(import_evidence::source_reports::read(&base, "malformed selector").is_err());
}

#[test]
fn existing_selective_merge_and_native_history_keep_source_and_primary_reports_separate() {
    use knx_core::CommandStack;
    use knx_store::project_history::NativeSnapshot;
    let source =
        selective_import::source_from_bytes(knx_testsupport::minimal_knxproj_bytes(), None)
            .unwrap();
    let exact = source.report.to_json();
    let id = source.source_hash.clone();
    let mut project = source.project.clone();
    project.info.name = "Fictional destination".into();
    project.devices = knx_core::Devices::new();
    for installation in &mut project.installations {
        installation.parameters.clear();
        installation.buildings.clear();
        installation.group_addresses.clear();
        installation.group_ranges.clear();
        for line in &mut installation.topology.lines {
            line.devices.clear();
        }
        installation.topology.unassigned.clear();
    }
    let mut target = NativeSnapshot {
        project,
        opaque: vec![],
        manufacturer_refs: vec![],
    };
    import_evidence::capture_primary(&mut target.opaque, &source.report).unwrap();
    let primary = import_evidence::read(&target.opaque).unwrap().unwrap();
    let installation = source.project.installations[0].id.0;
    let selection = selective_import::Selection {
        source_installation: installation,
        target_installation: installation,
        devices: vec![source.project.devices.iter().next().unwrap().id.0],
        lines: vec![],
    };
    let plan = selective_import::plan(&target, &source, selection).unwrap();
    let mut stack = CommandStack::new();
    selective_import::apply(&mut target, &mut stack, plan).unwrap();
    assert_eq!(
        import_evidence::source_reports::read(&target.opaque, &id)
            .unwrap()
            .report_json,
        exact
    );
    assert_eq!(
        import_evidence::read(&target.opaque).unwrap().unwrap(),
        primary
    );
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("fictional.knxdb");
    knx_app::project_history::save_as(&path, &target, &stack).unwrap();
    let conn = knx_store::open_existing_read_only(&path).unwrap();
    let restored = knx_store::project_history::load_editor(&conn.conn)
        .unwrap()
        .unwrap();
    assert!(restored.working == target);
    assert_eq!(
        import_evidence::source_reports::read(&restored.working.opaque, &id)
            .unwrap()
            .report_json,
        exact
    );
    assert_eq!(
        import_evidence::read(&restored.working.opaque)
            .unwrap()
            .unwrap(),
        primary
    );
    stack.undo(&mut target.project).unwrap();
    stack.redo(&mut target.project).unwrap();
    assert_eq!(
        import_evidence::source_reports::read(&target.opaque, &id)
            .unwrap()
            .report_json,
        exact
    );
    assert_eq!(
        import_evidence::read(&target.opaque).unwrap().unwrap(),
        primary
    );
}

#[test]
fn a_retained_source_archive_without_report_is_explicitly_unavailable() {
    let id = "c".repeat(64);
    let entries = vec![StoredOpaqueEntry {
        source_path: format!("imports/{id}/source.knxproj"),
        xpath: String::new(),
        kind: "SelectiveImportArchive".into(),
        name: String::new(),
        bytes: b"fictional archive".to_vec(),
        sha256: id.clone(),
    }];
    let catalogue = import_evidence::source_reports::catalogue(&entries).unwrap();
    assert_eq!(catalogue.len(), 1);
    assert_eq!(catalogue[0].status, "unavailable");
    assert!(import_evidence::source_reports::read(&entries, &id).is_err());
}
