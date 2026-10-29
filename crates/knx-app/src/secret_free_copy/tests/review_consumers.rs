//! Pins historical consumer semantics and shared connection-parser work before allocation.

use super::*;
use serde_json::Value;

fn produced(dir: &Path, password: &str, selective: bool) -> (PathBuf, Value, (usize, usize)) {
    let xml = topology(false)
        .replace("/project/21", "/project/23")
        .replace(
            "<DeviceInstance ",
            &format!("<DeviceInstance Password=\"{password}\" DiagnosticWitness=\"safe sample\" "),
        );
    let info = project_info(false).replace("/project/21", "/project/23");
    let bytes = knx_testsupport::zip_with_entries(&[
        ("P-0001.signature", b"synthetic signature"),
        ("P-0001/0.xml", xml.as_bytes()),
        ("P-0001/Project.xml", info.as_bytes()),
        ("M-TEST/Baggages/a.bin", b"safe a"),
        ("M-TEST/Baggages/b.bin", b"safe b"),
    ]);
    let input = dir.join("source.knxproj");
    std::fs::write(&input, &bytes).unwrap();
    let (project, opaque, refs, report) = if selective {
        let source = crate::selective_import::source_from_bytes(bytes.clone(), None).unwrap();
        (
            source.project,
            source.opaque,
            source.manufacturer_refs,
            source.report,
        )
    } else {
        let conn = knx_store::open_and_migrate_in_memory().unwrap();
        let imported = crate::import_ets_project(&input, &conn).unwrap();
        let mut opaque = knx_store::load_opaque(&conn).unwrap();
        crate::import_evidence::capture_primary(&mut opaque, &imported.report).unwrap();
        (
            imported.project,
            opaque,
            knx_store::load_manufacturer_refs(&conn).unwrap(),
            imported.report,
        )
    };
    assert_eq!(std::fs::read(input).unwrap(), bytes);
    let baggage: Vec<_> = report
        .unsupported
        .iter()
        .filter(|r| r.what.starts_with("M-TEST/Baggages/"))
        .collect();
    assert_eq!(
        baggage.len(),
        2,
        "real producer must retain two distinct notices"
    );
    let labels: BTreeSet<_> = baggage
        .iter()
        .map(|r| crate::import_findings::retained_payload_label(&r.what, &r.consequence).unwrap())
        .collect();
    assert_eq!(labels.len(), 1, "the ordinary counter groups these notices");
    let counts = crate::import_findings::import_finding_counts(&report);
    let value = serde_json::to_value(&report).unwrap();
    assert!(value["unknown"]
        .as_array()
        .unwrap()
        .iter()
        .any(|r| r["name"] == "Password" && r["sample"] == password));
    let path = dir.join("project.knxdb");
    let conn = knx_store::open_and_migrate(&path).unwrap();
    knx_store::save_project_with_passthrough(&conn, &project, &opaque, &refs).unwrap();
    (path, value, counts)
}

fn historical(value: &Value) -> crate::import_evidence::HistoricalImportReport {
    // Reuse the actual admitted primary envelope identity and normal reader, not a counting replica.
    let conn = knx_store::open_and_migrate_in_memory().unwrap();
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("minimal.knxproj");
    std::fs::write(&input, knxproj(false)).unwrap();
    let imported = crate::import_ets_project(&input, &conn).unwrap();
    let mut entry = crate::import_evidence::encode(&imported.report).unwrap();
    let mut envelope: Value = serde_json::from_slice(&entry.bytes).unwrap();
    envelope["report_json"] = Value::String(serde_json::to_string(value).unwrap());
    entry.bytes = serde_json::to_vec(&envelope).unwrap();
    entry.sha256 = knx_store::project_history::image_hash(&entry.bytes);
    crate::import_evidence::read(&[entry]).unwrap().unwrap()
}

fn native_semantics(password: &str, selective: bool, unavailable: bool) {
    let dir = tempfile::tempdir().unwrap();
    let (path, report, counts) = produced(dir.path(), password, selective);
    let source = knx_store::open_existing_read_only(&path).unwrap();
    let original_rows = knx_store::load_opaque(&source.conn).unwrap();
    let catalogue = crate::import_evidence::source_reports::catalogue(&original_rows).unwrap();
    if selective {
        assert_eq!(
            (catalogue[0].error_rows, catalogue[0].warning_rows),
            (Some(counts.0), Some(counts.1))
        );
    } else {
        assert_eq!(
            crate::import_evidence::read(&original_rows)
                .unwrap()
                .unwrap()
                .counts(),
            counts
        );
    }
    drop(source);
    let before = std::fs::read(&path).unwrap();
    let copy = native_project_without_secrets(&path).unwrap();
    let out = reopened(&copy.bytes, dir.path());
    let rows = knx_store::load_opaque(&out.conn).unwrap();
    if unavailable {
        assert!(
            copy.withheld
                .iter()
                .any(|s| s.contains("report unavailable")),
            "withheld report must explicitly be unavailable"
        );
        if selective {
            let clean = crate::import_evidence::source_reports::catalogue(&rows).unwrap();
            assert_eq!(clean.len(), 1);
            assert_eq!(clean[0].status, "unavailable");
            assert_eq!((clean[0].error_rows, clean[0].warning_rows), (None, None));
            assert!(crate::import_evidence::source_reports::read(&rows, &catalogue[0].id).is_err());
        } else {
            assert!(crate::import_evidence::read(&rows).unwrap().is_none());
        }
    } else {
        let clean = if selective {
            let clean = crate::import_evidence::source_reports::catalogue(&rows).unwrap();
            assert_eq!(
                (clean[0].error_rows, clean[0].warning_rows),
                (Some(counts.0), Some(counts.1))
            );
            serde_json::from_str(
                &crate::import_evidence::source_reports::read(&rows, &catalogue[0].id)
                    .unwrap()
                    .report_json,
            )
            .unwrap()
        } else {
            let clean = crate::import_evidence::read(&rows).unwrap().unwrap();
            assert_eq!(clean.counts(), counts);
            clean.report
        };
        same_inventory(&report, &clean);
    }
    let source = knx_store::open_existing_read_only(&path).unwrap();
    assert_eq!(knx_store::load_opaque(&source.conn).unwrap(), original_rows);
    drop(source);
    assert_eq!(std::fs::read(path).unwrap(), before);
}

fn same_inventory(original: &Value, clean: &Value) {
    assert_eq!(clean["counts"], original["counts"]);
    assert_eq!(clean["unsupported"], original["unsupported"]);
    for key in ["unknown", "opaque", "errors", "conflicts", "inferred"] {
        let a = original[key].as_array().unwrap();
        let b = clean[key].as_array().unwrap();
        assert_eq!(a.len(), b.len());
        for (a, b) in a.iter().zip(b) {
            for field in ["source_path", "xpath", "kind", "name", "severity", "stage"] {
                assert_eq!(a[field], b[field]);
            }
        }
    }
    assert!(clean["unknown"]
        .as_array()
        .unwrap()
        .iter()
        .any(|r| r["name"] == "DiagnosticWitness" && r["sample"] == "safe sample"));
}

#[test]
fn secret_free_c1_real_grouped_report_preserves_derived_counts_or_refuses() {
    let dir = tempfile::tempdir().unwrap();
    let (_, report, counts) = produced(dir.path(), "payload", false);
    assert_eq!(historical(&report).counts(), counts);
    let before = report.clone();
    match import_report_without_secrets(&report) {
        Ok(copy) => {
            let clean = serde_json::from_slice(&copy.bytes).unwrap();
            assert_eq!(
                historical(&clean).counts(),
                counts,
                "successful sanitizer changed historical derived warning counts"
            );
            same_inventory(&report, &clean);
        }
        Err(error) => assert!(error.to_string().contains("report unavailable")),
    }
    assert_eq!(report, before);
}

#[test]
fn secret_free_c1_primary_grouping_identity_is_unavailable() {
    native_semantics("payload", false, true);
}
#[test]
fn secret_free_c1_selective_grouping_identity_is_unavailable() {
    native_semantics("payload", true, true);
}
#[test]
fn secret_free_c1_primary_safe_identity_still_reads() {
    native_semantics("unrelated-strong-password", false, false);
}
#[test]
fn secret_free_c1_selective_safe_identity_still_reads() {
    native_semantics("unrelated-strong-password", true, false);
}

#[test]
fn secret_free_c1_public_safe_report_keeps_exact_consumer_semantics() {
    let dir = tempfile::tempdir().unwrap();
    let (_, report, counts) = produced(dir.path(), "unrelated-strong-password", false);
    let before = report.clone();
    let copy = import_report_without_secrets(&report).unwrap();
    let clean = serde_json::from_slice(&copy.bytes).unwrap();
    assert_eq!(historical(&clean).counts(), counts);
    same_inventory(&report, &clean);
    assert_eq!(report, before);
}

fn interpreted_field(password: &str, field: &str) {
    let dir = tempfile::tempdir().unwrap();
    let (_, mut report, _) = produced(dir.path(), "unrelated-strong-password", false);
    for row in report["unknown"].as_array_mut().unwrap() {
        if row["name"] == "Password" {
            row["sample"] = Value::String(password.into());
        }
    }
    // Supplemental valid historical shapes isolate fields absent from this producer's errors.
    if field == "severity" {
        report["errors"].as_array_mut().unwrap().push(serde_json::json!({"stage":"validate", "severity":"warning", "xpath":"/safe", "detail":"safe warning"}));
    }
    if field == "entity" {
        report["counts"]["rows"][0]["entity"] = Value::String(password.into());
    }
    historical(&report);
    assert!(
        report.to_string().matches(password).count() > 1,
        "calibrate semantic collision {field}"
    );
    let before = report.clone();
    let error =
        import_report_without_secrets(&report).expect_err("interpreted field cannot be blanked");
    assert!(
        error.to_string().contains("report unavailable"),
        "{field}: {error}"
    );
    assert!(!error.to_string().contains(password));
    assert!(!format!("{error:?}").contains(password));
    assert_eq!(report, before);
}

#[test]
fn secret_free_c1_severity_is_not_blanked() {
    interpreted_field("warning", "severity");
}
#[test]
fn secret_free_c1_entity_unit_is_not_blanked() {
    interpreted_field("measurement-unit", "entity");
}
#[test]
fn secret_free_c1_unsupported_what_is_not_blanked() {
    interpreted_field("module-based", "what");
}

#[test]
fn secret_free_c2_connection_parser_precharges_shared_work() {
    let text = ";;;;;;;;";
    let charge = 64 * text.len() + 1;
    let findings = Findings::default();
    findings
        .budget
        .work(128 * 1024 * 1024 - charge + 1)
        .unwrap();
    let mut child = findings.child();
    assert!(xml::redact_connection_string(text, "x", &mut child).is_none());
    assert!(
        findings.ensure().is_err(),
        "connection parser did not reserve work before separator fan-out"
    );
    assert!(child.values.is_empty());
    assert!(child.redactions.is_empty());
}
