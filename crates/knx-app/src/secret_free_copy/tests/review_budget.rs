//! Pins residual-search, learned-value and cumulative inspection resource boundaries.

use super::*;

#[test]
fn secret_free_budget_matching_work_is_measured_and_sticky() {
    let values: BTreeSet<Vec<u8>> = (0..200)
        .map(|i| format!("AAAAAAB{i:06}").into_bytes())
        .collect();
    let needles = Needles::new(&values);
    assert!(!needles.any_in(&[b'A'; 64]));
    // All 59 six-byte windows select all 200 candidates; each upper-bound
    // comparison charges 13 bytes independently of the mismatching seventh byte.
    assert_eq!(needles.budget.matching_used(), 64 + 59 * 200 * 13);
    assert!(needles.budget.check().is_ok());
    needles.any_in(&vec![b'A'; 65536]);
    assert_eq!(
        needles.budget.check().unwrap_err(),
        "residual matching budget exceeded"
    );
    assert!(needles.budget.matching_used() <= budget::MATCH_WORK);
    assert!(
        needles.any_in(b"unrelated"),
        "exhaustion is not silently a no-match"
    );
    assert_eq!(
        needles.budget.check().unwrap_err(),
        "residual matching budget exceeded"
    );
}

#[test]
fn secret_free_budget_nested_children_share_exact_decoded_boundary() {
    let parent = Findings::default();
    let first = parent.child();
    let second = parent.child();
    first.budget.decoded(1024 * 1024 * 1024 - 1).unwrap();
    second.budget.decoded(1).unwrap();
    assert!(parent.ensure().is_ok());
    assert_eq!(
        second.budget.decoded(1).unwrap_err(),
        "decoded expansion budget exceeded"
    );
    assert!(parent.ensure().is_err());
}

#[test]
fn secret_free_budget_xml_fragments_share_exact_work_boundary() {
    let mut findings = Findings::default();
    // <a/> (4 bytes + event) and EOF consume six work units; seed through the real common meter.
    findings.budget.work(128 * 1024 * 1024 - 6).unwrap();
    let base = xml::Base {
        text: "a:",
        root_in_base: false,
    };
    assert!(xml::redact(b"<a/>", &base, &mut findings.child())
        .unwrap()
        .is_none());
    assert!(findings.ensure().is_ok());
    assert_eq!(
        xml::redact(b"<a/>", &base, &mut findings).unwrap_err(),
        "inspection work budget exceeded"
    );
}

#[test]
fn secret_free_review_r8_same_prefix_nonmatch_refuses_on_matching_budget() {
    let xml = format!(
        "<a>{}</a>",
        (0..200)
            .map(|i| format!("<b Password=\"AAAAAAB{i:06}\"/>"))
            .collect::<String>()
    );
    let binary = vec![b'A'; 65536];
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("input.knxproj");
    let bytes = knx_testsupport::zip_with_entries(&[
        ("secrets.xml", xml.as_bytes()),
        ("nonmatch.bin", &binary),
    ]);
    std::fs::write(&path, &bytes).unwrap();
    let result = import_source_without_secrets(&path);
    assert!(
        result.is_err(),
        "same-prefix nonmatch did not enforce matching-work budget"
    );
    let error = result.unwrap_err();
    assert_eq!(
        error.to_string(),
        "secret-free copy refused: residual matching budget exceeded"
    );
    assert_eq!(std::fs::read(path).unwrap(), bytes);
}

#[test]
fn secret_free_review_r8_learned_values_are_cumulatively_bounded() {
    let mut findings = Findings::default();
    for i in 0..16385 {
        findings.remember(format!("unique-secret-{i:06}").as_bytes());
    }
    assert!(
        findings.ensure().is_err(),
        "unbounded learned-value set admitted"
    );
}

#[test]
fn secret_free_review_r8_findings_are_cumulatively_bounded() {
    let mut findings = Findings::default();
    for i in 0..32769 {
        findings.redact(format!("row-{i}"), "Password".into());
    }
    assert!(findings.ensure().is_err(), "unbounded finding set admitted");
}

#[test]
fn secret_free_budget_archive_nested_expansion_is_cumulative_before_decode() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("input.knxproj");
    let inner = knx_testsupport::zip_with_entries(&[("safe.txt", b"safe")]);
    let bytes = knx_testsupport::zip_with_entries(&[("plugin.zip", inner.as_slice())]);
    std::fs::write(&path, &bytes).unwrap();
    let charge = 2 * (inner.len() + 4);
    let at = budget::Budget::default();
    at.decoded(1024 * 1024 * 1024 - charge).unwrap();
    let copy = archive::copy_with_budget(&path, || {}, at).unwrap();
    assert_eq!(copy.bytes, bytes);
    let over = budget::Budget::default();
    over.decoded(1024 * 1024 * 1024 - charge + 1).unwrap();
    let error = archive::copy_with_budget(&path, || {}, over).unwrap_err();
    assert_eq!(
        error.to_string(),
        "secret-free copy refused: decoded expansion budget exceeded"
    );
    assert_eq!(std::fs::read(path).unwrap(), bytes);
}

#[test]
fn secret_free_budget_source_reader_exact_limit_and_limit_plus_one() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("input.knxproj");
    std::fs::write(&path, b"12345678").unwrap();
    assert_eq!(
        archive::read_source_with_limit(&path, 8).unwrap(),
        b"12345678"
    );
    std::fs::write(&path, b"123456789").unwrap();
    assert!(matches!(
        archive::read_source_with_limit(&path, 8),
        Err(SecretFreeCopyError::TooLarge { bytes: 9, limit: 8 })
    ));
    let file = std::fs::File::create(&path).unwrap();
    file.set_len(budget::SOURCE_BYTES + 1).unwrap();
    assert!(matches!(
        import_source_without_secrets(&path),
        Err(SecretFreeCopyError::TooLarge { .. })
    ));
    file.set_len(budget::NATIVE_BYTES + 1).unwrap();
    assert!(matches!(
        native_project_without_secrets(&path),
        Err(SecretFreeCopyError::TooLarge { .. })
    ));
}
