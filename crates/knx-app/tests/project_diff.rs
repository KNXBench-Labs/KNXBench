//! A corpus-gated proof of design spec §3.7's own correctness anchor:
//! `diff_projects(&p, &p)` — here, two *independent* imports of the same
//! `.knxproj`, the strongest form of that property — produces a
//! `ProjectDiff` in which every `added`/`removed`/`changed`/`ambiguous`
//! list is empty, at every level, for every entity type. A hand-built
//! fixture can assert the same property; it cannot exercise the natural-key
//! matching, `ets_id` matching, and field-resolution logic against
//! real, large, ETS-shaped data (36 devices, 907 communication objects,
//! 514 group addresses) the way this test does.
//!
//! Lives here, not in `knx-diff/tests/`, for the same reason
//! `csv_roundtrip.rs`/`documentation_export.rs` do: `check-layering` walks
//! dev-dependency edges too (`xtask/src/layering.rs:94-98`), so a
//! `knx-etsproj` dev-dependency on `knx-diff` — needed here to build two
//! real `Project`s from the reference `.knxproj` — would trip `knx-diff`'s
//! own layering rule (it may depend on `knx-core` only). `knx-app` is
//! deliberately the one crate that sees both sides, so its `tests/` is
//! where a test that needs both `knx-etsproj` and `knx-diff` belongs.
//!
//! `knx-app`'s tests cannot reach `knx-etsproj`'s `tests/support` module —
//! a crate's integration tests are private to it — so this uses
//! `knx-testsupport`, the shared dev-dependency every crate's tests can
//! reach, instead.
//!
//! Gated by the standard `corpus_available()` pattern: `OriginalData/` is
//! the maintainer's own real KNX installation, gitignored and local-only,
//! so CI (and any contributor without a copy) skips this test rather than
//! failing it.

use std::path::PathBuf;

fn reference_ets4_path() -> PathBuf {
    knx_testsupport::reference_ets4_path()
}

#[test]
fn rendering_diff_projects_between_two_independent_imports_of_the_reference_project_is_empty() {
    if !reference_ets4_path().exists() {
        eprintln!("skip: OriginalData/ corpus not present (gitignored, local-only)");
        return;
    }

    // Two independent imports, not one project cloned — the strongest form
    // of design spec §3.7's property, and the one that actually exercises
    // `ets_id` matching end to end (two separate mapper runs producing two
    // separate `Project` values that must still agree on every identity).
    let p1 = knx_etsproj::import_knxproj(&reference_ets4_path())
        .expect("the reference project is a known-good fixture")
        .project;
    let p2 = knx_etsproj::import_knxproj(&reference_ets4_path())
        .expect("the reference project is a known-good fixture")
        .project;

    // Proof the corpus path actually ran, not the skip path: a project
    // with real substance to diff.
    let device_count = p1.devices.iter().count();
    let com_object_count = p1.devices.com_objects().count();
    let group_address_count: usize = p1
        .installations
        .iter()
        .map(|i| i.group_addresses.len())
        .sum();
    assert!(
        device_count > 0 && com_object_count > 0 && group_address_count > 0,
        "expected the reference project to contain real devices, communication objects, \
         and group addresses; got {device_count} devices, {com_object_count} communication \
         objects, {group_address_count} group addresses — did the skip guard actually trigger?"
    );
    eprintln!(
        "project_diff corpus test: {device_count} devices, \
         {com_object_count} communication objects, {group_address_count} group addresses"
    );

    let diff = knx_diff::diff_projects(&p1, &p2);

    assert!(
        diff.info_changes.is_empty(),
        "expected no project-info changes between two independent imports of the same \
         project, got {:?}",
        diff.info_changes
    );

    assert_eq!(
        diff.installations.len(),
        p1.installations.len(),
        "expected one InstallationDiff per installation"
    );

    for installation in &diff.installations {
        assert_eq!(
            installation.status,
            knx_diff::EntityStatus::Matched,
            "expected installation {} to match by InstallationId on both sides",
            installation.id
        );
        assert!(
            installation.field_changes.is_empty(),
            "expected no installation-level field changes for installation {}, got {:?}",
            installation.id,
            installation.field_changes
        );

        assert_table_empty("areas", installation.id, &installation.areas);
        assert_table_empty("lines", installation.id, &installation.lines);
        assert_table_empty("group_ranges", installation.id, &installation.group_ranges);
        assert_table_empty(
            "group_addresses",
            installation.id,
            &installation.group_addresses,
        );
        assert_table_empty("buildings", installation.id, &installation.buildings);

        let devices = &installation.devices;
        assert!(
            devices.added.is_empty()
                && devices.removed.is_empty()
                && devices.changed.is_empty()
                && devices.ambiguous.is_empty(),
            "expected devices to be entirely empty for installation {}: \
             added={}, removed={}, changed={}, ambiguous={}",
            installation.id,
            devices.added.len(),
            devices.removed.len(),
            devices.changed.len(),
            devices.ambiguous.len()
        );
    }

    // A second run against the same two `Project`s is byte-for-byte the
    // same emptiness — cheap extra evidence there is no order-dependent
    // flake hiding behind "empty happens to look the same every time".
    let diff_again = knx_diff::diff_projects(&p1, &p2);
    assert_eq!(
        diff, diff_again,
        "expected diff_projects to be deterministic on the same two inputs"
    );
}

fn assert_table_empty<K: std::fmt::Debug, F>(
    label: &str,
    installation_id: u8,
    table: &knx_diff::EntityTable<K, F>,
) {
    assert!(
        table.added.is_empty()
            && table.removed.is_empty()
            && table.changed.is_empty()
            && table.ambiguous.is_empty(),
        "expected {label} to be entirely empty for installation {installation_id}: \
         added={}, removed={}, changed={}, ambiguous={}",
        table.added.len(),
        table.removed.len(),
        table.changed.len(),
        table.ambiguous.len()
    );
}
