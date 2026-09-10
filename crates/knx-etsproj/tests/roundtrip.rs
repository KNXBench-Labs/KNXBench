//! The three roundtrip fidelity guarantees (Task 20): semantic equality
//! under the declared comparison relation, hash-identical opaque bytes,
//! and an export that is unsigned and says so — plus convergence: a
//! second roundtrip changes nothing further.

mod support;
use support::*;

use knx_etsproj::compare::{describe_difference, semantic_view};
use knx_etsproj::export::{export_knxproj, ExportWarning};
use knx_etsproj::opaque::OpaqueEntry;
use knx_etsproj::{import_knxproj, import_knxproj_bytes, Container};

#[test]
fn roundtrip_model_is_semantically_equal() {
    if !reference_ets4_path().exists() {
        eprintln!("skip: OriginalData/ corpus not present (gitignored, local-only)");
        return;
    }
    let first = import_knxproj(&reference_ets4_path()).unwrap();
    let exported = export_knxproj(
        &first.project,
        &all_entries(&first.opaque, &first.manufacturer),
    )
    .unwrap();
    let second = import_knxproj_bytes(exported.bytes, "roundtrip.knxproj").unwrap();

    let a = semantic_view(&first.project);
    let b = semantic_view(&second.project);
    if let Some(diff) = describe_difference(&a, &b) {
        panic!("roundtrip changed the model: {diff}");
    }
}

#[test]
fn roundtrip_opaque_bytes_are_hash_identical() {
    if !reference_ets4_path().exists() {
        eprintln!("skip: OriginalData/ corpus not present (gitignored, local-only)");
        return;
    }
    let first = import_knxproj(&reference_ets4_path()).unwrap();
    let exported = export_knxproj(
        &first.project,
        &all_entries(&first.opaque, &first.manufacturer),
    )
    .unwrap();
    let second = import_knxproj_bytes(exported.bytes, "roundtrip.knxproj").unwrap();

    let hashes = |entries: &[OpaqueEntry]| {
        let mut v: Vec<_> = entries
            .iter()
            .map(|e| {
                (
                    e.source_path.clone(),
                    e.xpath.clone(),
                    e.name.clone(),
                    e.sha256.clone(),
                )
            })
            .collect();
        v.sort();
        v
    };
    assert_eq!(hashes(&first.opaque), hashes(&second.opaque));
}

#[test]
fn export_is_unsigned_and_reports_it() {
    if !reference_ets4_path().exists() {
        eprintln!("skip: OriginalData/ corpus not present (gitignored, local-only)");
        return;
    }
    let first = import_knxproj(&reference_ets4_path()).unwrap();
    let exported = export_knxproj(
        &first.project,
        &all_entries(&first.opaque, &first.manufacturer),
    )
    .unwrap();
    assert!(exported
        .warnings
        .iter()
        .any(|w| matches!(w, ExportWarning::Unsigned { .. })));

    // And the file itself carries no signature we produced: the entries that
    // are there were copied, and are reported stale.
    let stale: Vec<_> = exported
        .warnings
        .iter()
        .filter_map(|w| match w {
            ExportWarning::StaleSignature { source_path } => Some(source_path.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(stale.len(), 5);
}

#[test]
fn a_second_roundtrip_changes_nothing_further() {
    // Convergence: if the first roundtrip normalizes something, the second
    // must not normalize it again. A pipeline that keeps changing the file is
    // not a roundtrip.
    if !reference_ets4_path().exists() {
        eprintln!("skip: OriginalData/ corpus not present (gitignored, local-only)");
        return;
    }
    let first = import_knxproj(&reference_ets4_path()).unwrap();
    let once = export_knxproj(
        &first.project,
        &all_entries(&first.opaque, &first.manufacturer),
    )
    .unwrap()
    .bytes;
    let mid = import_knxproj_bytes(once.clone(), "once.knxproj").unwrap();
    let twice = export_knxproj(&mid.project, &all_entries(&mid.opaque, &mid.manufacturer))
        .unwrap()
        .bytes;

    let a = Container::open(once).unwrap();
    let b = Container::open(twice).unwrap();
    let names = |c: &Container| {
        let mut v: Vec<_> = c.entries().iter().map(|e| e.path.clone()).collect();
        v.sort();
        v
    };
    assert_eq!(names(&a), names(&b));
    let mut a = a;
    let mut b = b;
    assert_eq!(
        a.read("P-0512/0.xml").unwrap(),
        b.read("P-0512/0.xml").unwrap()
    );
}
