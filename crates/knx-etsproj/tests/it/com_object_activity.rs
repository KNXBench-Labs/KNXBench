//! Pins every corpus project's communication-object activity counts (ISSUE-08).
//!
//! Project Schema23 §1.2.5.13 declares no `IsActive` on
//! `ComObjectInstanceRef_t`, and ADR-0014 makes `GroupObjectTree` membership
//! the activity statement for schema ≥ 21. Before this file existed the
//! schema-≥21 mapper still read the absent attribute as `false`, so the
//! ETS 6.3.0 reference project imported 691 of its 867 communication
//! objects as inactive (every one that also had an override) and the KV
//! schema-21 demo 26 of 75. Nothing reported it, because an absent
//! attribute is not a malformed one.
//!
//! The ETS4 (schema 11) project states `IsActive` on all 907 of its
//! overrides and keeps reading it verbatim; its count must not move.
//!
//! Gated by the standard `corpus_available()` pattern: `OriginalData/` is
//! the maintainer's own installation, gitignored, so CI and every
//! contributor without a copy skip rather than fail.

use knx_etsproj::import_knxproj;

/// `(active, total)` over every communication object in one project.
fn active_counts(path: &std::path::Path) -> (usize, usize) {
    let out = import_knxproj(path).unwrap();
    let project = &out.project;
    let mut active = 0;
    let mut total = 0;
    for device in project.devices.iter() {
        for &id in &device.com_objects {
            let com = project
                .devices
                .com_object(id)
                .expect("device lists an existing communication object");
            total += 1;
            if com.is_active {
                active += 1;
            }
        }
    }
    (active, total)
}

#[test]
#[ignore = "requires the gitignored OriginalData/ corpus; run with --ignored"]
fn every_group_object_tree_member_imports_as_active() {
    assert!(
        knx_testsupport::corpus_available(),
        "OriginalData/ corpus not present (gitignored, local-only); this test is #[ignore]d and must be run explicitly on a machine that has it"
    );
    // Schema 23: 867 `GroupObjectTree` ids, 691 of them with an override
    // that omits `IsActive`. All 867 are active.
    assert_eq!(
        active_counts(&knx_testsupport::reference_ets6_path()),
        (867, 867)
    );
    // Schema 21 (KV): 75 ids, 26 overrides, none with `IsActive`.
    assert_eq!(
        active_counts(&knx_testsupport::reference_kv_schema21_path()),
        (75, 75)
    );
    // Schema 11: `IsActive` is stated on all 907 overrides and is read as
    // stated. This number is the control: the schema-≥21 fix must not
    // reach it.
    assert_eq!(
        active_counts(&knx_testsupport::reference_ets4_path()),
        (907, 907)
    );
}
