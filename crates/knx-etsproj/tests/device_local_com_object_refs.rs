//! Pins ETS 6's device-local `ComObjectInstanceRef/@RefId` against all three corpus projects.
//!
//! The 867 `MalformedRefId` losses that used to come out of the
//! schema-23 reference project are gone, and the ETS4 and schema-21
//! counts must not move because of the fix.
//!
//! Before `device_local_com_object_number` existed, the schema-23 project
//! reported 867 `MalformedRefId` map errors over 310 distinct ids and
//! numbered every one of its 867 communication objects `0`. Neither of
//! those is true any more, and this file is what notices if either comes
//! back.
//!
//! Gated by the standard `corpus_available()` pattern: `OriginalData/` is
//! the maintainer's own installation, gitignored, so CI and every
//! contributor without a copy skip rather than fail.

mod support;

use knx_etsproj::import_knxproj;
use knx_etsproj::report::Severity;

/// Every `MalformedRefId` in one import report, as its raw id string.
fn malformed_ref_ids(report: &knx_etsproj::ImportReport) -> Vec<String> {
    report
        .errors
        .iter()
        .filter_map(|e| {
            // `MapProblemDetail`/`ValueError` are matched through their
            // `Debug` rendering rather than by pattern: the report's error
            // detail is deliberately an opaque, display-oriented type, and
            // a regression test has no business widening its API.
            let rendered = format!("{:?}", e.detail);
            let start = rendered.find("MalformedRefId(")? + "MalformedRefId(".len();
            let rest = &rendered[start..];
            let end = rest.find(')')?;
            Some(rest[..end].to_string())
        })
        .collect()
}

/// The headline regression: the ETS 6.3.0 schema-23 reference project used
/// to lose 867 communication-object references over 310 distinct ids, and
/// nothing in `docs/` said so. Both numbers are now zero.
#[test]
fn the_ets6_project_reports_no_malformed_com_object_ref_ids() {
    if !knx_testsupport::corpus_available() {
        eprintln!("skip: OriginalData/ corpus not present (gitignored, local-only)");
        return;
    }
    let out = import_knxproj(&support::reference_ets6_path()).unwrap();
    assert_eq!(out.report.source.schema_version, 23);

    // Counted, not compared element-wise: the pre-fix failure printed all
    // 867 ids and was unreadable. The first few are enough to recognise
    // the shape that regressed.
    let malformed = malformed_ref_ids(&out.report);
    let distinct: std::collections::BTreeSet<&String> = malformed.iter().collect();
    assert_eq!(
        (malformed.len(), distinct.len()),
        (0, 0),
        "was (867, 310) before the device-local path existed; sample: {:?}",
        malformed.iter().take(5).collect::<Vec<_>>()
    );
    assert_eq!(
        out.report
            .errors
            .iter()
            .filter(|e| e.severity == Severity::Error)
            .count(),
        0,
        "the schema-23 project maps with no error-severity problems at all"
    );
}

/// Every one of the 867 communication objects now carries the object
/// number its own `RefId` states, instead of the `0` a failed split left
/// behind. Read off the mapped project, not off the file, so the assertion
/// is about what a caller actually receives.
#[test]
fn every_ets6_com_object_number_matches_its_own_ref_ids_o_digits() {
    if !knx_testsupport::corpus_available() {
        eprintln!("skip: OriginalData/ corpus not present (gitignored, local-only)");
        return;
    }
    let out = import_knxproj(&support::reference_ets6_path()).unwrap();
    let project = &out.project;

    let mut checked = 0usize;
    let mut distinct = std::collections::BTreeSet::new();
    let mut nonzero = 0usize;
    for device in project.devices.iter() {
        for &com_id in &device.com_objects {
            let com = project
                .devices
                .com_object(com_id)
                .expect("a device's own com object id resolves");
            let ets_id = &com.source.ets_id;
            distinct.insert(ets_id.clone());
            // The id is kept verbatim — the mapper reconstructs nothing
            // into `source.ets_id`, it only reads the number out of it.
            let (head, tail) = ets_id
                .split_once('_')
                .unwrap_or_else(|| panic!("{ets_id} is not a two-segment device-local id"));
            assert!(tail.starts_with("R-"), "{ets_id}");
            let expected: u16 = head
                .strip_prefix("O-")
                .and_then(|d| d.parse().ok())
                .unwrap_or_else(|| panic!("{ets_id} has no O-<n> head"));
            assert_eq!(com.number, expected, "{ets_id}");
            if com.number != 0 {
                nonzero += 1;
            }
            checked += 1;
        }
    }
    assert_eq!(checked, 867, "the schema-23 project's com object count");
    assert_eq!(distinct.len(), 310, "distinct device-local RefIds");
    assert_eq!(
        nonzero, 835,
        "before the fix every one of the 867 was numbered 0"
    );
}

/// The other two corpus projects are the regression risk: a fix that
/// resolves schema 23 and quietly changes what schema 11 or schema 21 map
/// to would be worse than the loss it removed. Both counts are the ones
/// measured on `fcf4563`, before the change.
#[test]
fn the_ets4_and_schema21_projects_are_untouched_by_the_device_local_path() {
    if !knx_testsupport::corpus_available() {
        eprintln!("skip: OriginalData/ corpus not present (gitignored, local-only)");
        return;
    }

    let ets4 = import_knxproj(&support::reference_ets4_path()).unwrap();
    assert_eq!(ets4.report.source.schema_version, 11);
    assert_eq!(ets4.report.errors, vec![], "schema 11 was already clean");
    assert_eq!(
        ets4.project
            .devices
            .iter()
            .map(|d| d.com_objects.len())
            .sum::<usize>(),
        907
    );

    let kv = import_knxproj(&knx_testsupport::reference_kv_schema21_path()).unwrap();
    assert_eq!(kv.report.source.schema_version, 21);
    assert_eq!(malformed_ref_ids(&kv.report).len(), 0);
    assert_eq!(
        kv.report.errors.len(),
        1,
        "one pre-existing UnresolvedReference, unrelated to RefId shape"
    );
    assert_eq!(
        kv.project
            .devices
            .iter()
            .map(|d| d.com_objects.len())
            .sum::<usize>(),
        75
    );
}
