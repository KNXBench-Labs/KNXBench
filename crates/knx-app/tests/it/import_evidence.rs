//! Historical report evidence must survive native saves without inventing reassessment.

use knx_app::import_evidence;
use knx_etsproj::report::{EntityCounts, ImportError, Severity, SourceInfo, UnsupportedFeature};
use knx_etsproj::ImportReport;

fn report() -> ImportReport {
    ImportReport {
        source: SourceInfo {
            file_name: "fictional.knxproj".into(),
            file_size: 17,
            schema_version: 23,
            namespace: "http://knx.org/xml/project/23".into(),
            created_by: None,
            tool_version: None,
            namespace_disagreement: None,
        },
        source_observations: None,
        counts: EntityCounts { rows: vec![] },
        unknown: vec![],
        opaque: vec![],
        inferred: vec![],
        conflicts: vec![],
        unsupported: vec![UnsupportedFeature {
            what: "fictional feature".into(),
            consequence: "retained, not editable".into(),
        }],
        errors: vec![ImportError {
            source_path: None,
            stage: "map",
            severity: Severity::Warning,
            xpath: "/fictional".into(),
            detail: "fictional warning".into(),
        }],
    }
}

#[test]
fn native_evidence_roundtrips_full_report_and_counts_separately_from_current_findings() {
    let input = report();
    let entry = import_evidence::encode(&input).unwrap();
    let mut opaque = vec![entry];
    let restored = import_evidence::read(&opaque)
        .unwrap()
        .expect("historical report missing");
    assert_eq!(restored.report, serde_json::to_value(&input).unwrap());
    assert_eq!(restored.report_json, input.to_json());
    assert_eq!(restored.counts(), (0, 2));
    let conn = knx_store::open_and_migrate_in_memory().unwrap();
    knx_store::insert_opaque(&conn, &opaque).unwrap();
    opaque = knx_store::load_opaque(&conn).unwrap();
    assert_eq!(
        import_evidence::read(&opaque).unwrap().unwrap().report,
        restored.report
    );
}

fn replace_envelope(
    entry: &mut knx_store::StoredOpaqueEntry,
    change: impl FnOnce(&mut serde_json::Value),
) {
    let mut envelope: serde_json::Value = serde_json::from_slice(&entry.bytes).unwrap();
    change(&mut envelope);
    entry.bytes = serde_json::to_vec(&envelope).unwrap();
    entry.sha256 = knx_store::project_history::image_hash(&entry.bytes);
}

#[test]
fn corrupt_duplicate_future_and_malformed_records_are_refused_without_mutation() {
    let entry = import_evidence::encode(&report()).unwrap();
    let mut variants = Vec::new();
    let mut corrupt = entry.clone();
    corrupt.bytes[0] ^= 1;
    variants.push(vec![corrupt]);
    // A parseable envelope whose recorded hash no longer matches: only the
    // SHA-256 check can refuse it (flipping byte 0 already breaks the JSON).
    let mut stale_hash = entry.clone();
    stale_hash.sha256 = "0".repeat(64);
    variants.push(vec![stale_hash]);
    let mut bad_version = entry.clone();
    replace_envelope(&mut bad_version, |v| v["version"] = serde_json::json!(2));
    variants.push(vec![bad_version]);
    let mut bad_shape = entry.clone();
    replace_envelope(&mut bad_shape, |v| {
        v["report_json"] = serde_json::json!("{}")
    });
    variants.push(vec![bad_shape]);
    let mut bad_severity = entry.clone();
    replace_envelope(&mut bad_severity, |v| {
        let mut payload: serde_json::Value =
            serde_json::from_str(v["report_json"].as_str().unwrap()).unwrap();
        payload["errors"][0]["severity"] = serde_json::json!("invented");
        v["report_json"] = serde_json::json!(serde_json::to_string(&payload).unwrap());
    });
    variants.push(vec![bad_severity]);
    let mut bad_identity = entry.clone();
    bad_identity.name = "other.json".into();
    variants.push(vec![bad_identity]);
    variants.push(vec![entry.clone(), entry]);
    for entries in variants {
        let original = entries.clone();
        assert!(
            import_evidence::read(&entries).is_err(),
            "bad primary evidence admitted"
        );
        assert_eq!(entries, original);
    }
}

#[test]
fn ambiguous_counter_keys_are_refused_even_with_a_matching_payload_hash() {
    let mut entry = import_evidence::encode(&report()).unwrap();
    replace_envelope(&mut entry, |envelope| {
        let raw = envelope["report_json"].as_str().unwrap();
        envelope["report_json"] = serde_json::json!(raw.replace(
            "\"severity\":\"warning\"",
            "\"severity\":\"error\",\"severity\":\"warning\""
        ));
    });
    assert!(
        import_evidence::read(&[entry]).is_err(),
        "ambiguous historical severity accepted"
    );
}

#[test]
fn report_budget_is_enforced_at_encoding_and_before_decoding() {
    let mut input = report();
    input.source.file_name = "x".repeat(import_evidence::MAX_EVIDENCE_BYTES);
    assert!(
        import_evidence::encode(&input).is_err(),
        "oversized report encoded"
    );
    let mut entry = import_evidence::encode(&report()).unwrap();
    entry.bytes = vec![b'x'; import_evidence::MAX_EVIDENCE_BYTES + 1];
    assert!(
        import_evidence::read(&[entry]).is_err(),
        "oversized metadata decoded"
    );
}

#[test]
fn source_file_lookalikes_and_scoped_reports_are_preserved_without_primary_interpretation() {
    let mut lookalike = import_evidence::encode(&report()).unwrap();
    lookalike.kind = "File".into();
    let mut scoped = import_evidence::encode(&report()).unwrap();
    scoped.source_path = "selection/source/native/import-report.json".into();
    let entries = vec![lookalike, scoped];
    let original = entries.clone();
    assert!(import_evidence::read(&entries).unwrap().is_none());
    assert_eq!(entries, original);
}

#[test]
fn primary_capture_appends_one_record_and_refuses_duplicate_without_changing_entries() {
    let mut entries = vec![knx_store::StoredOpaqueEntry {
        source_path: "fictional.txt".into(),
        xpath: String::new(),
        kind: "File".into(),
        name: "fictional.txt".into(),
        bytes: b"exact original".to_vec(),
        sha256: "fictional identity".into(),
    }];
    let source = entries.clone();
    import_evidence::capture_primary(&mut entries, &report()).unwrap();
    assert_eq!(
        entries.len(),
        source.len() + 1,
        "primary evidence was not attached"
    );
    assert_eq!(&entries[..source.len()], source.as_slice());
    assert_eq!(
        import_evidence::read(&entries)
            .unwrap()
            .unwrap()
            .report_json,
        report().to_json()
    );
    let admitted = entries.clone();
    assert!(
        import_evidence::capture_primary(&mut entries, &report()).is_err(),
        "duplicate primary capture accepted"
    );
    assert_eq!(entries, admitted, "duplicate refusal changed evidence");
}

#[test]
fn refused_primary_capture_preserves_all_original_entries() {
    let mut entries = vec![];
    let mut oversized = report();
    oversized.source.file_name = "x".repeat(import_evidence::MAX_EVIDENCE_BYTES);
    assert!(import_evidence::capture_primary(&mut entries, &oversized).is_err());
    assert!(entries.is_empty());
}

#[test]
fn absent_legacy_evidence_is_unavailable_not_a_measured_zero() {
    assert!(import_evidence::read(&[]).unwrap().is_none());
}

const PNG_NOTICE: &str = "manufacturer payload (png); retained byte-exact, uninterpreted; not rendered or executed; contents not unpacked";

fn report_with_retained_files() -> ImportReport {
    let mut input = report();
    for name in ["a", "b", "c"] {
        input.unsupported.push(UnsupportedFeature {
            what: format!("M-0001/Baggages/{name}.png"),
            consequence: PNG_NOTICE.into(),
        });
    }
    input
}

#[test]
fn reopened_counts_follow_the_fresh_import_rule_for_retained_files() {
    // Issue #7: three retained PNGs share one notice and count once, both
    // right after the import and after the project was saved and reopened.
    let input = report_with_retained_files();
    assert_eq!(knx_app::import_finding_counts(&input), (0, 3));
    let entry = import_evidence::encode(&input).unwrap();
    let restored = import_evidence::read(&[entry]).unwrap().unwrap();
    assert_eq!(restored.counts(), knx_app::import_finding_counts(&input));
    assert_eq!(
        restored.report["unsupported"].as_array().unwrap().len(),
        4,
        "the stored report keeps one row per file"
    );
}

fn edit_report(entry: &mut knx_store::StoredOpaqueEntry, edit: impl FnOnce(&mut String)) {
    replace_envelope(entry, |envelope| {
        let mut raw = envelope["report_json"].as_str().unwrap().to_owned();
        edit(&mut raw);
        envelope["report_json"] = serde_json::json!(raw);
    });
}

#[test]
fn unsupported_rows_without_text_count_once_and_ambiguous_text_is_refused() {
    let mut entry = import_evidence::encode(&report_with_retained_files()).unwrap();
    let mut bare = entry.clone();
    edit_report(&mut bare, |raw| {
        *raw = raw.replace(
            r#"{"what":"fictional feature","consequence":"retained, not editable"}"#,
            "{}",
        );
    });
    assert_eq!(
        import_evidence::read(&[bare]).unwrap().unwrap().counts(),
        (0, 3)
    );
    edit_report(&mut entry, |raw| {
        *raw = raw.replacen(
            r#""consequence":"manufacturer"#,
            r#""consequence":"other","consequence":"manufacturer"#,
            1,
        );
    });
    assert!(
        import_evidence::read(&[entry]).is_err(),
        "ambiguous historical unsupported text accepted"
    );
}

#[test]
fn reopened_counts_group_manufacturer_unknowns_like_a_fresh_import() {
    let mut input = report();
    for (path, name) in [
        ("M-0083/M-0083_A-01.xml", "Name"),
        ("M-0001/M-0001_A-02.xml", "Name"),
        ("M-0083/M-0083_A-01.xml", "Id"),
        ("P-0512/project.xml", "ProjectId"),
    ] {
        input.unknown.push(knx_etsproj::UnknownConstruct {
            source_path: path.into(),
            xpath: "/x".into(),
            kind: knx_etsproj::UnknownKind::Attribute,
            name: name.into(),
            occurrences: 2,
            sample: None,
        });
    }
    // Name group + Id group + project row + the fixture's two other warnings.
    assert_eq!(knx_app::import_finding_counts(&input), (0, 5));
    let entry = import_evidence::encode(&input).unwrap();
    assert_eq!(
        import_evidence::read(&[entry]).unwrap().unwrap().counts(),
        (0, 5)
    );
}
