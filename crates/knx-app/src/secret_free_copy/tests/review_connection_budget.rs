//! Checks streaming connection parsing, exact shared charges and caller publication refusals.

use super::*;

const WORK: usize = 128 * 1024 * 1024;
fn charge(text: &str) -> usize {
    64 * text.len() + 1
}
fn remaining(units: usize) -> budget::Budget {
    let budget = budget::Budget::default();
    budget.work(WORK - units).unwrap();
    budget
}
fn xml_input(text: &str) -> String {
    format!("<a><BusAccess Parameter=\"{text}\"/></a>")
}
fn resource(error: SecretFreeCopyError) {
    assert!(
        matches!(
            error,
            SecretFreeCopyError::BudgetExceeded {
                resource: "inspection work budget exceeded"
            }
        ),
        "{error}"
    );
}

#[test]
fn secret_free_c2_exact_streaming_charge_and_output_reservation() {
    for text in [
        ";;;;;;;;",
        ";Host=é;;Port=1;;;",
        "Host=é;UserPassword=秘密;;",
    ] {
        let budget = remaining(charge(text));
        let mut findings = Findings {
            budget: budget.clone(),
            ..Findings::default()
        };
        let clean = xml::redact_connection_string(text, "x", &mut findings);
        assert_eq!(budget.work_used(), WORK);
        findings.ensure().unwrap();
        assert_eq!(
            budget.output_used(),
            if clean.is_some() { text.len() } else { 0 }
        );
        assert_eq!(budget.work(1), Err("inspection work budget exceeded"));
        let budget = remaining(charge(text) - 1);
        let mut findings = Findings {
            budget: budget.clone(),
            ..Findings::default()
        };
        assert!(xml::redact_connection_string(text, "x", &mut findings).is_none());
        resource(findings.ensure().unwrap_err());
        assert_eq!(
            budget.output_used(),
            0,
            "refuse before allocating owned output"
        );
        assert!(findings.values.is_empty());
        assert!(findings.redactions.is_empty());
    }
    let mut findings = Findings::default();
    let owned = "Host=é;UserPassword=秘密;;";
    findings
        .budget
        .output(1024 * 1024 * 1024 - owned.len() + 1)
        .unwrap();
    assert!(xml::redact_connection_string(owned, "x", &mut findings).is_none());
    assert!(matches!(
        findings.ensure(),
        Err(SecretFreeCopyError::BudgetExceeded {
            resource: "inspection output budget exceeded"
        })
    ));
    assert!(
        findings.values.is_empty(),
        "reserve output before retaining credentials"
    );
    assert!(findings.redactions.is_empty());
}

#[test]
fn secret_free_c2_streaming_preserves_delimiters_unicode_and_mixed_syntax() {
    for (input, expected) in [
        ("", None),
        (";;;;;;;", None),
        (" ; ; ", None),
        ("Host=é;;Port=1;;;", None),
        ("password= ;", None),
        (
            " ;UserPASSWORD=秘密;; Host=é;TokenKey=x;;;",
            Some(" ;UserPASSWORD=;; Host=é;TokenKey=;;;"),
        ),
        ("UserPassword=abc;malformed", Some("")),
        ("opaque authenticationCode blob", Some("")),
        ("Host=é;malformed", None),
    ] {
        let mut findings = Findings::default();
        assert_eq!(
            xml::redact_connection_string(input, "x", &mut findings).as_deref(),
            expected,
            "{input}"
        );
        findings.ensure().unwrap();
        assert_eq!(findings.budget.work_used(), charge(input));
    }
    let (clean, _) =
        redact_text("<a><BusAccess Parameter=\"Host=é&amp;x;;UserPassword=秘密;Port=1;\"/></a>");
    assert_eq!(
        clean.unwrap(),
        "<a><BusAccess Parameter=\"Host=é&amp;x;;UserPassword=;Port=1;\"/></a>"
    );
}

#[test]
fn secret_free_c2_xml_and_archive_share_exact_connection_charge() {
    let text = ";;;;;;;;";
    let input = xml_input(text);
    // Three XML events plus EOF; no competing depth/path/duplicate-attribute charge.
    let units = input.len() + 4 + charge(text);
    let base = xml::Base {
        text: "x:",
        root_in_base: false,
    };
    let budget = remaining(units);
    let mut findings = Findings {
        budget: budget.clone(),
        ..Findings::default()
    };
    assert!(xml::redact(input.as_bytes(), &base, &mut findings)
        .unwrap()
        .is_none());
    assert_eq!(budget.work_used(), WORK);
    let mut findings = Findings {
        budget: remaining(units - 1),
        ..Findings::default()
    };
    assert_eq!(
        xml::redact(input.as_bytes(), &base, &mut findings).unwrap_err(),
        "inspection work budget exceeded"
    );

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("input.knxproj");
    let source = knx_testsupport::zip_with_entries(&[("x.xml", input.as_bytes())]);
    std::fs::write(&path, &source).unwrap();
    assert_eq!(import_source_without_secrets(&path).unwrap().bytes, source);
    assert_eq!(
        archive::copy_with_budget(&path, || {}, remaining(2 * units))
            .unwrap()
            .bytes,
        source
    );
    resource(archive::copy_with_budget(&path, || {}, remaining(2 * units - 1)).unwrap_err());
    // Fail specifically at first-pass connection reservation (not at EOF).
    resource(
        archive::copy_with_budget(
            &path,
            || {},
            remaining(3 + 1 + input.len() - 7 + charge(text) - 1),
        )
        .unwrap_err(),
    );
    assert_eq!(std::fs::read(path).unwrap(), source);
}

#[test]
fn secret_free_c2_report_fields_share_budget_and_preserve_input() {
    let text = ";;;;;;;;";
    let report = serde_json::json!({"unknown":[{"name":"Parameter", "xpath":"/a/BusAccess", "sample":text, "value":text, "raw_text_preview":text}]});
    let before = report.clone();
    let copy = import_report_without_secrets(&report).unwrap();
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&copy.bytes).unwrap(),
        before
    );
    let budget = remaining(3 * charge(text));
    report::copy_with_budget(&report, budget.clone()).unwrap();
    assert_eq!(budget.work_used(), WORK);
    resource(report::copy_with_budget(&report, remaining(3 * charge(text) - 1)).unwrap_err());
    resource(report::copy_with_budget(&report, remaining(2 * charge(text) - 1)).unwrap_err());
    assert_eq!(report, before);
}

#[test]
fn secret_free_c2_native_attributes_and_xml_propagate_shared_refusal() {
    for kind in [
        "RetainedAttribute",
        "SelectiveImportRetainedAttribute",
        "RetainedElement",
    ] {
        let dir = tempfile::tempdir().unwrap();
        let path = native_project(dir.path(), false);
        let conn = knx_store::open_existing_and_migrate(&path).unwrap();
        conn.execute_batch("DELETE FROM opaque_entry; DELETE FROM manufacturer_file;")
            .unwrap();
        let text = ";;;;;;;;";
        let input = xml_input(text);
        let (name, bytes, units) = if kind == "RetainedElement" {
            ("a", input.as_bytes(), input.len() + 4 + charge(text))
        } else {
            ("Parameter", text.as_bytes(), charge(text))
        };
        conn.execute("INSERT INTO opaque_entry(source_path,xpath,kind,name,bytes,sha256) VALUES(?1,?2,?3,?4,?5,?6)",
            ("safe.xml", "/a/BusAccess", kind, name, bytes, knx_store::project_history::image_hash(bytes))).unwrap();
        drop(conn);
        let before = std::fs::read(&path).unwrap();
        let public = native_project_without_secrets(&path).unwrap();
        let out = reopened(&public.bytes, dir.path());
        let rows = knx_store::load_opaque(&out.conn).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].bytes, bytes);
        drop(out);
        let budget = remaining(2 * units);
        native::copy_with_budget(&path, |_| {}, budget.clone()).unwrap();
        assert_eq!(budget.work_used(), WORK);
        let mut allocated = None;
        resource(
            native::copy_with_budget(
                &path,
                |p| allocated = Some(p.to_path_buf()),
                remaining(2 * units - 1),
            )
            .unwrap_err(),
        );
        assert!(!allocated.unwrap().exists());
        assert_eq!(std::fs::read(path).unwrap(), before);
    }
}
