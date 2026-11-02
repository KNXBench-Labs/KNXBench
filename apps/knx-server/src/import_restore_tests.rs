//! A reopened native project shows its stored import findings in the session log again.

use crate::session_log::Severity;
use crate::{domain, AppState};
use knx_etsproj::report::{
    EntityCount, EntityCounts, ImportError, Severity as ReportSeverity, SourceInfo,
    UnsupportedFeature,
};
use knx_etsproj::{ImportReport, UnknownConstruct, UnknownKind};

fn unknown(source_path: &str, name: &str) -> UnknownConstruct {
    UnknownConstruct {
        source_path: source_path.into(),
        xpath: "/KNX/Thing".into(),
        kind: UnknownKind::Attribute,
        name: name.into(),
        occurrences: 2,
        sample: None,
    }
}

/// One error, one warning-severity row, three unknowns (two share a manufacturer group),
/// one unsupported capability: the shared rule counts 1 error and 4 warnings.
fn report() -> ImportReport {
    ImportReport {
        source: SourceInfo {
            file_name: "synthetic.knxproj".into(),
            file_size: 1,
            schema_version: 20,
            namespace: "http://knx.org/xml/project/20".into(),
            created_by: None,
            tool_version: None,
            namespace_disagreement: None,
        },
        source_observations: None,
        counts: EntityCounts {
            rows: vec![EntityCount {
                entity: "device".into(),
                read: 1,
                mapped: 1,
            }],
        },
        unknown: vec![
            unknown("P-0001/0.xml", "Foo"),
            unknown("M-0001/a.xml", "Name"),
            unknown("M-0001/b.xml", "Name"),
        ],
        opaque: vec![],
        inferred: vec![],
        conflicts: vec![],
        unsupported: vec![UnsupportedFeature {
            what: "Scenes".into(),
            consequence: "not mapped".into(),
        }],
        errors: vec![
            ImportError {
                source_path: None,
                stage: "validate",
                severity: ReportSeverity::Error,
                xpath: "/a".into(),
                detail: "broken".into(),
            },
            ImportError {
                source_path: None,
                stage: "map",
                severity: ReportSeverity::Warning,
                xpath: "/b".into(),
                detail: "odd".into(),
            },
        ],
    }
}

fn reopened(with_report: bool) -> (crate::SharedState, knx_projection::ProjectTree) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("restored.knxdb");
    let mut opaque = Vec::new();
    if with_report {
        knx_app::import_evidence::capture_primary(&mut opaque, &report()).unwrap();
    }
    let project = knx_core::Project::new(knx_core::Language("en".into()));
    domain::save_project_as_impl(&path, &project, &opaque, &[]).unwrap();
    let state = std::sync::Arc::new(AppState::default());
    let progress = domain::detached_progress(crate::load_progress::LoadKind::Open, &path);
    let tree = domain::open_native_project(&state, &path, true, &progress).expect("reopens");
    (state, tree)
}

fn count(state: &AppState, severity: Severity) -> usize {
    let log = state.session_log.lock().unwrap();
    log.entries()
        .iter()
        .filter(|e| e.severity == severity && e.source.starts_with("import:"))
        .count()
}

#[test]
fn reopened_counters_equal_the_import_entries_in_the_log() {
    let (state, tree) = reopened(true);
    assert_eq!((tree.errors, tree.warnings), (1, 4));
    assert_eq!(count(&state, Severity::Error), tree.errors);
    assert_eq!(count(&state, Severity::Warning), tree.warnings);
}

#[test]
fn restored_entries_are_announced_once_and_never_claim_an_import_time() {
    let (state, _) = reopened(true);
    let log = state.session_log.lock().unwrap();
    let notes: Vec<_> = log
        .entries()
        .iter()
        .filter(|e| e.message.contains("stored import report"))
        .collect();
    assert_eq!(notes.len(), 1);
    assert_eq!(notes[0].source, "open");
    assert_eq!(notes[0].severity, Severity::Info);
    assert!(
        log.entries().iter().all(|e| e.source != "import"),
        "no fake import-time summary"
    );
}

#[test]
fn a_project_without_a_stored_report_gets_no_import_entries() {
    let (state, tree) = reopened(false);
    assert_eq!((tree.errors, tree.warnings), (0, 0));
    let log = state.session_log.lock().unwrap();
    assert!(log
        .entries()
        .iter()
        .all(|e| !e.source.starts_with("import") && !e.message.contains("stored import report")));
}

#[test]
fn the_stored_projection_matches_the_live_import_entries_apart_from_info_notes() {
    use crate::session_log::from_import_report;
    let shape = |e: crate::session_log::LogEntry| {
        (
            format!("{:?}", e.severity),
            e.source,
            e.message,
            e.location,
            e.detail,
        )
    };
    let live: Vec<_> = from_import_report(&report())
        .into_iter()
        .filter(|e| e.severity != Severity::Info)
        .map(shape)
        .collect();
    let stored: Vec<_> =
        crate::historical_import_log::project(&serde_json::to_value(report()).unwrap(), (1, 4))
            .unwrap()
            .into_iter()
            .map(shape)
            .collect();
    assert_eq!(stored, live);
}

// ---- Design-review repair: admitted-but-sparse historical rows ----

use serde_json::{json, Value};

const NOT_RECORDED: &str = "not recorded";

/// A minimal admitted report body; `rows` overrides or adds top-level arrays.
fn raw_report(rows: Value) -> Value {
    let mut report = json!({
        "source": {"file_name": "hist.knxproj", "file_size": 1, "schema_version": 20, "namespace": "ns"},
        "counts": {"rows": []},
        "unknown": [], "opaque": [], "inferred": [], "conflicts": [], "unsupported": [], "errors": []
    });
    for (key, value) in rows.as_object().unwrap() {
        report[key] = value.clone();
    }
    report
}

fn raw_evidence(report: &Value) -> knx_store::StoredOpaqueEntry {
    let envelope = json!({
        "format": "knxbench-native-import-evidence", "version": 1, "producer": "test",
        "report_json": report.to_string()
    });
    let bytes = serde_json::to_vec(&envelope).unwrap();
    knx_store::StoredOpaqueEntry {
        source_path: "knxbench-native/import-report.json".into(),
        xpath: String::new(),
        kind: "KnxBenchImportEvidence".into(),
        name: "import-report.json".into(),
        sha256: knx_store::project_history::image_hash(&bytes),
        bytes,
    }
}

struct Saved {
    _dir: tempfile::TempDir,
    path: std::path::PathBuf,
}

fn save_with(opaque: Vec<knx_store::StoredOpaqueEntry>) -> Saved {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("raw.knxdb");
    let project = knx_core::Project::new(knx_core::Language("en".into()));
    domain::save_project_as_impl(&path, &project, &opaque, &[]).unwrap();
    Saved { _dir: dir, path }
}

fn open_saved(state: &crate::SharedState, saved: &Saved) -> knx_projection::ProjectTree {
    let progress = domain::detached_progress(crate::load_progress::LoadKind::Open, &saved.path);
    domain::open_native_project(state, &saved.path, true, &progress).expect("reopens")
}

fn reopened_raw(report: &Value) -> (crate::SharedState, knx_projection::ProjectTree) {
    let saved = save_with(vec![raw_evidence(report)]);
    let state = std::sync::Arc::new(AppState::default());
    let tree = open_saved(&state, &saved);
    (state, tree)
}

fn import_entries(state: &AppState) -> Vec<crate::session_log::LogEntry> {
    let log = state.session_log.lock().unwrap();
    log.entries()
        .iter()
        .filter(|e| e.source.starts_with("import:"))
        .cloned()
        .collect()
}

fn counted(entries: &[crate::session_log::LogEntry]) -> (usize, usize) {
    (
        entries
            .iter()
            .filter(|e| e.severity == Severity::Error)
            .count(),
        entries
            .iter()
            .filter(|e| e.severity == Severity::Warning)
            .count(),
    )
}

#[test]
fn an_empty_unsupported_row_is_shown_as_not_recorded_with_its_raw_row() {
    let (state, tree) = reopened_raw(&raw_report(json!({"unsupported": [{}]})));
    assert_eq!((tree.errors, tree.warnings), (0, 1));
    let entries = import_entries(&state);
    assert_eq!(entries.len(), 1);
    assert!(
        entries[0].message.contains(NOT_RECORDED),
        "{}",
        entries[0].message
    );
    assert_eq!(entries[0].detail.as_deref(), Some("{}"));
}

#[test]
fn a_conflict_without_address_or_candidates_never_shows_null_or_zero() {
    let (state, _) = reopened_raw(&raw_report(
        json!({"conflicts": [{}, {"group_address": 7}]}),
    ));
    let entries = import_entries(&state);
    assert_eq!(entries.len(), 2);
    assert!(
        entries[0].message.matches(NOT_RECORDED).count() == 2,
        "{}",
        entries[0].message
    );
    assert!(
        entries[1].message.contains("7") && entries[1].message.contains(NOT_RECORDED),
        "{}",
        entries[1].message
    );
    for e in &entries {
        assert!(
            !e.message.contains("null") && !e.message.contains(" 0 "),
            "{}",
            e.message
        );
        assert!(e.detail.is_some(), "raw row kept");
    }
}

#[test]
fn an_unknown_construct_without_an_occurrence_count_is_not_a_zero() {
    let row = json!({"source_path": "P-0001/0.xml", "kind": "Attribute", "name": "X"});
    let (state, _) = reopened_raw(&raw_report(json!({"unknown": [row]})));
    let entries = import_entries(&state);
    assert_eq!(entries.len(), 1);
    assert!(
        entries[0].message.contains(NOT_RECORDED) && !entries[0].message.contains("0 time"),
        "{}",
        entries[0].message
    );
    assert!(entries[0]
        .detail
        .as_deref()
        .unwrap()
        .contains("P-0001/0.xml"));
}

#[test]
fn a_manufacturer_group_total_that_overflows_or_is_not_an_integer_is_withheld() {
    let max = u64::MAX;
    for second in [json!(max), json!(1.5), json!("2"), json!(-1), Value::Null] {
        let rows = json!([
            {"source_path": "M-0001/a.xml", "xpath": "/x", "kind": "Attribute", "name": "N", "occurrences": max},
            {"source_path": "M-0001/b.xml", "xpath": "/y", "kind": "Attribute", "name": "N", "occurrences": second},
        ]);
        let (state, tree) = reopened_raw(&raw_report(json!({"unknown": rows})));
        assert_eq!(tree.warnings, 1);
        let entries = import_entries(&state);
        assert_eq!(entries.len(), 1);
        assert!(
            entries[0].message.contains(NOT_RECORDED),
            "{second}: {}",
            entries[0].message
        );
        assert!(
            entries[0]
                .detail
                .as_deref()
                .unwrap()
                .contains("M-0001/b.xml"),
            "members still listed"
        );
    }
}

#[test]
fn a_manufacturer_group_with_integer_totals_still_sums_them() {
    let rows = json!([
        {"source_path": "M-0001/a.xml", "xpath": "/x", "kind": "Attribute", "name": "N", "occurrences": 2},
        {"source_path": "M-0001/b.xml", "xpath": "/y", "kind": "Attribute", "name": "N", "occurrences": 3},
    ]);
    let (state, _) = reopened_raw(&raw_report(json!({"unknown": rows})));
    assert!(import_entries(&state)[0]
        .message
        .contains("5 occurrence(s)"));
}

fn mixed_report() -> Value {
    let payload = |file: &str| json!({"what": file, "consequence": format!("{file}: opaque manufacturer payload; preserved, not interpreted or executed")});
    raw_report(json!({
        "errors": [
            {"stage": "validate", "severity": "error", "xpath": "/a", "detail": "d1"},
            {"stage": "map", "severity": "warning", "xpath": "", "detail": "d2", "source_path": "M-1/x.xml"},
        ],
        "unknown": [
            {"source_path": "P-0001/0.xml", "xpath": "/p", "kind": "Element", "name": "E", "occurrences": 1},
            {"source_path": "M-0001/a.xml", "xpath": "/x", "kind": "Attribute", "name": "N", "occurrences": 1},
            {"source_path": "M-0002/a.xml", "xpath": "/x", "kind": "Attribute", "name": "N", "occurrences": 1},
            {"source_path": "M-0003/a.xml", "xpath": "/x", "kind": "Attribute", "name": "Single", "occurrences": 1},
            {},
        ],
        "conflicts": [{"group_address": 3, "candidates": [["9.001", [1]], ["5.001", [2]]]}, {}],
        "unsupported": [payload("M-1/a.bin"), payload("M-1/b.bin"), payload("M-1/c.bin"), {"what": "Scenes", "consequence": "not mapped"}, {}],
    }))
}

#[test]
fn mixed_groups_and_both_severities_project_exactly_the_admitted_counts() {
    let (state, tree) = reopened_raw(&mixed_report());
    // errors 1; warnings: 1 + unknown (P, N-group, Single, {}) 4 + conflicts 2 + unsupported (group, Scenes, {}) 3
    assert_eq!((tree.errors, tree.warnings), (1, 10));
    let entries = import_entries(&state);
    assert_eq!(counted(&entries), (tree.errors, tree.warnings));
    assert!(
        entries
            .iter()
            .any(|e| e.message.contains("3 retained files")),
        "payload rows collapse to one entry"
    );
    assert!(
        entries
            .iter()
            .any(|e| e.message.contains("2 places in 2 files")),
        "manufacturer group"
    );
}

#[test]
fn the_projection_is_taken_before_the_log_cap_evicts_anything() {
    let errors: Vec<Value> = (0..16_100)
        .map(|i| json!({"stage": "validate", "severity": "error", "xpath": format!("/e{i}"), "detail": "d"}))
        .collect();
    let (state, tree) = reopened_raw(&raw_report(json!({"errors": errors})));
    assert_eq!(
        tree.errors, 16_100,
        "the counter is the admitted one, not the capped log"
    );
    let log = state.session_log.lock().unwrap();
    assert_eq!(log.entries().len(), crate::session_log::MAX_ENTRIES);
    assert!(log
        .entries()
        .iter()
        .any(|e| e.message.contains("log entries dropped")));
}

#[test]
fn reopening_twice_does_not_duplicate_the_restored_entries() {
    let saved = save_with(vec![raw_evidence(&mixed_report())]);
    let state = std::sync::Arc::new(AppState::default());
    let first = open_saved(&state, &saved);
    let again = open_saved(&state, &saved);
    assert_eq!(
        (first.errors, first.warnings),
        (again.errors, again.warnings)
    );
    assert_eq!(
        counted(&import_entries(&state)),
        (again.errors, again.warnings)
    );
    let log = state.session_log.lock().unwrap();
    assert_eq!(
        log.entries()
            .iter()
            .filter(|e| e.message.contains("stored import report"))
            .count(),
        1
    );
}

#[test]
fn an_unsaved_refusal_restores_nothing_and_keeps_the_open_project() {
    let saved = save_with(vec![raw_evidence(&mixed_report())]);
    let state = std::sync::Arc::new(AppState::default());
    domain::new_project_impl(&state, None, None, None, None, true).unwrap();
    domain::create_area_impl(&state, "Unsaved".into(), 1).unwrap();
    let progress = domain::detached_progress(crate::load_progress::LoadKind::Open, &saved.path);
    let refused = domain::open_native_project(&state, &saved.path, false, &progress).unwrap_err();
    assert_eq!(refused.kind, Some(domain::UNSAVED_CHANGES));
    assert!(import_entries(&state).is_empty());
    assert!(!state
        .session_log
        .lock()
        .unwrap()
        .entries()
        .iter()
        .any(|e| e.message.contains("stored import report")));
    assert!(state.has_unsaved_changes(), "old project untouched");
}

#[test]
fn a_fresh_import_logs_its_own_entries_and_no_restoration_line() {
    let dir = tempfile::tempdir().unwrap();
    let xml = br#"<KNX xmlns="http://knx.org/xml/project/21"><Project Id="P-0001"><Installations><Installation InstallationId="0" Name="Main" Odd="x"><Topology/></Installation></Installations></Project></KNX>"#;
    let info = br#"<KNX xmlns="http://knx.org/xml/project/21"><Project Id="P-0001"><ProjectInformation Name="Fx" GroupAddressStyle="ThreeLevel"/></Project></KNX>"#;
    let zip = knx_testsupport::zip_with_entries(&[
        ("P-0001.signature", b"s"),
        ("P-0001/0.xml", xml),
        ("P-0001/Project.xml", info),
    ]);
    let source = dir.path().join("fresh.knxproj");
    std::fs::write(&source, zip).unwrap();
    let state = std::sync::Arc::new(AppState::default());
    let progress = domain::detached_progress(crate::load_progress::LoadKind::Import, &source);
    let tree = domain::open_project(&state, &source, &progress).expect("imports");
    let entries = import_entries(&state);
    assert_eq!(counted(&entries), (tree.errors, tree.warnings));
    assert!(!entries.is_empty());
    let log = state.session_log.lock().unwrap();
    assert!(!log
        .entries()
        .iter()
        .any(|e| e.message.contains("stored import report")));
    assert!(
        log.entries().iter().any(|e| e.source == "import"),
        "the live summary line stays"
    );
}

#[test]
fn a_recovered_working_image_restores_its_own_report_not_the_root_one() {
    let root_report = raw_report(
        json!({"errors": [{"stage": "validate", "severity": "error", "xpath": "/root", "detail": "root error"}]}),
    );
    let working_report = raw_report(json!({"errors": [
        {"stage": "validate", "severity": "error", "xpath": "/w1", "detail": "working one"},
        {"stage": "validate", "severity": "error", "xpath": "/w2", "detail": "working two"}]}));
    let saved = save_with(vec![raw_evidence(&root_report)]);
    let conn = knx_store::open_and_migrate(&saved.path).unwrap();
    let working = knx_store::project_history::NativeSnapshot {
        project: knx_core::Project::new(knx_core::Language("en".into())),
        opaque: vec![raw_evidence(&working_report)],
        manufacturer_refs: vec![],
    };
    knx_store::project_history::save_editor(
        &conn,
        &working,
        &knx_core::CommandStack::new(),
        None,
        false,
    )
    .unwrap();
    drop(conn);
    let state = std::sync::Arc::new(AppState::default());
    let tree = open_saved(&state, &saved);
    assert_eq!(
        tree.errors, 2,
        "counter follows the recovered working image"
    );
    let messages: Vec<_> = import_entries(&state)
        .into_iter()
        .map(|e| e.message)
        .collect();
    assert_eq!(messages, ["working one", "working two"]);
}

#[test]
fn a_projection_that_disagrees_with_the_admitted_counts_lists_nothing() {
    let report = raw_report(json!({"unsupported": [{}]}));
    assert!(crate::historical_import_log::project(&report, (0, 1)).is_ok());
    let refusal = crate::historical_import_log::project(&report, (0, 2)).unwrap_err();
    assert!(refusal.contains("admitted counts"), "{refusal}");
}
