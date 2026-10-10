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
//! Synthetic boundary cases run in ordinary CI. Private corpus cases are
//! explicitly ignored by default and fail when requested without their inputs;
//! `OriginalData/` is local-only and never committed.

mod support;

use knx_etsproj::import_knxproj;
use knx_etsproj::report::Severity;

/// Repeated RefIds must resolve within each owning device, never across devices.
#[test]
fn synthetic_device_local_refs_keep_each_devices_identity_overrides_and_links() {
    use knx_core::{Direction, GroupAddress, Layer, Override, Resolved, Text};

    for version in [21, 23] {
        let imported = knx_etsproj::import_knxproj_bytes(
            knx_testsupport::mapping_boundary_knxproj_bytes(version, None, None, "O-7_R-1"),
            "synthetic-objects.knxproj",
        )
        .unwrap();
        assert_eq!(imported.report.errors, vec![]);
        let project = &imported.project;
        let devices: Vec<_> = project.devices.iter().collect();
        assert_eq!(devices.len(), 2);
        assert_eq!(project.devices.com_objects().count(), 4);
        assert_ne!(devices[0].com_objects, devices[1].com_objects);
        for (index, device) in devices.iter().enumerate() {
            assert_eq!(device.com_objects.len(), 2);
            let object = project.devices.com_object(device.com_objects[0]).unwrap();
            assert_eq!(object.device, device.id);
            assert_eq!(object.source.ets_id, "O-7_R-1");
            assert_eq!(object.number, 7);
            assert_eq!(object.module_instance, None);
            assert_eq!(
                object.flags.read,
                Override::Value(Resolved {
                    value: index == 0,
                    layer: Layer::Instance,
                })
            );
            assert_eq!(
                object.text,
                Override::Value(Resolved {
                    value: Text::Literal(
                        if index == 0 {
                            "first device"
                        } else {
                            "second device"
                        }
                        .to_string(),
                    ),
                    layer: Layer::Instance
                })
            );
            assert_eq!(object.links.len(), 1);
            assert_eq!(object.links[0].direction, Direction::Send);
            let ga = project.installations[0]
                .group_addresses
                .iter()
                .find(|ga| ga.id == object.links[0].ga)
                .unwrap();
            assert_eq!(
                ga.address,
                GroupAddress::from_raw(if index == 0 { 1 } else { 2 })
            );
            let continuation = project.devices.com_object(device.com_objects[1]).unwrap();
            assert_eq!(continuation.device, device.id);
            assert_eq!(continuation.source.ets_id, "O-19_R-2");
            assert_eq!(continuation.number, 19);
        }
    }
}

#[test]
fn synthetic_malformed_local_refs_are_reported_without_losing_later_objects() {
    use knx_etsproj::map::MapProblemDetail;
    use knx_etsproj::values::ValueError;

    for version in [21, 23] {
        for ref_id in ["O-65536_R-1", "O-7_R-x", "O-7_R-1_extra"] {
            let imported = knx_etsproj::import_knxproj_bytes(
                knx_testsupport::mapping_boundary_knxproj_bytes(version, None, None, ref_id),
                "synthetic-malformed-objects.knxproj",
            )
            .unwrap();
            let expected = format!(
                "{:?}",
                MapProblemDetail::Value(ValueError::MalformedRefId(ref_id.to_string()),)
            );
            assert_eq!(imported.report.errors.len(), 2);
            for (index, device) in imported.project.devices.iter().enumerate() {
                let error = &imported.report.errors[index];
                assert_eq!(error.stage, "map");
                assert_eq!(error.severity, Severity::Error);
                assert_eq!(error.detail, expected);
                assert!(error.xpath.contains(&device.source.ets_id));
                assert!(error.xpath.ends_with(&format!("[@RefId='{ref_id}']")));
                assert_eq!(device.com_objects.len(), 2);
                let object = imported
                    .project
                    .devices
                    .com_object(device.com_objects[0])
                    .unwrap();
                assert_eq!(object.device, device.id);
                assert_eq!(object.source.ets_id, ref_id);
                // Existing explicit error placeholder, not a successful object-number claim.
                assert_eq!(object.number, 0);
                let continuation = imported
                    .project
                    .devices
                    .com_object(device.com_objects[1])
                    .unwrap();
                assert_eq!(continuation.device, device.id);
                assert_eq!(continuation.number, 19);
            }
            assert_eq!(imported.project.devices.com_objects().count(), 4);
        }
    }
}

#[test]
#[ignore = "requires the gitignored OriginalData/ corpus; run with --ignored"]
fn the_schema21_empty_default_line_is_one_exact_mapping_diagnostic() {
    use knx_etsproj::map::MapProblemDetail;
    use knx_etsproj::report::ImportError;

    assert!(
        knx_testsupport::corpus_available(),
        "explicit corpus scope is unavailable"
    );
    let imported = import_knxproj(&knx_testsupport::reference_kv_schema21_path()).unwrap();
    assert_eq!(imported.report.source.schema_version, 21);
    assert_eq!(
        imported.report.errors,
        vec![ImportError {
            source_path: None,
            stage: "map",
            severity: Severity::Error,
            xpath: "/KNX/Project/Installations/Installation".to_string(),
            detail: format!(
                "{:?}",
                MapProblemDetail::UnresolvedReference {
                    kind: "Installation/@DefaultLine",
                    target: String::new(),
                }
            ),
        }]
    );
    assert_eq!(imported.project.installations.len(), 1);
    let installation = &imported.project.installations[0];
    assert_eq!(installation.default_line, None);
    assert!(!installation.topology.lines.is_empty());
    // The existing real line must not be invented as the empty token's target.
}

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
#[ignore = "requires the gitignored OriginalData/ corpus; run with --ignored"]
fn the_ets6_project_reports_no_malformed_com_object_ref_ids() {
    assert!(
        knx_testsupport::corpus_available(),
        "OriginalData/ corpus not present (gitignored, local-only); this test is #[ignore]d and must be run explicitly on a machine that has it"
    );
    let out = import_knxproj(&support::reference_ets6_path()).unwrap();
    assert_eq!(out.report.source.schema_version, 23);

    // The RefId loss is gone, but nine unrelated, explicitly reported
    // attributes remain unsupported. Pin both the loss signal and the
    // constructs behind it so a future unknown cannot hide in the same
    // headline count. Every occurrence must also have reached the opaque
    // store summary; reporting without preservation would not be enough.
    assert!(out.report.has_losses());
    assert_eq!(out.report.unknown.len(), 9);
    let unknown_names = out
        .report
        .unknown
        .iter()
        .map(|unknown| unknown.name.as_str())
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(
        unknown_names,
        std::collections::BTreeSet::from([
            "CompletionStatus",
            "Hide16BitGroupsFromLegacyPlugins",
            "Name",
            "ProjectId",
            "ProjectTracingLevel",
        ])
    );
    for unknown in &out.report.unknown {
        assert_eq!(unknown.kind, knx_etsproj::parse::UnknownKind::Attribute);
        assert!(unknown.sample.is_some(), "{} has no sample", unknown.name);
    }
    let reported_by_name =
        out.report
            .unknown
            .iter()
            .fold(std::collections::BTreeMap::new(), |mut counts, unknown| {
                *counts.entry(unknown.name.as_str()).or_insert(0usize) +=
                    unknown.occurrences as usize;
                counts
            });
    let preserved_by_name = out
        .report
        .opaque
        .iter()
        .filter(|opaque| opaque.kind == "RetainedAttribute")
        .fold(std::collections::BTreeMap::new(), |mut counts, opaque| {
            *counts.entry(opaque.name.as_str()).or_insert(0usize) += 1;
            counts
        });
    for (name, reported) in reported_by_name {
        assert!(
            preserved_by_name
                .get(name)
                .is_some_and(|preserved| *preserved >= reported),
            "{name}: reported {reported}, preserved {:?}",
            preserved_by_name.get(name)
        );
    }

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
#[ignore = "requires the gitignored OriginalData/ corpus; run with --ignored"]
fn every_ets6_com_object_number_matches_its_own_ref_ids_o_digits() {
    assert!(
        knx_testsupport::corpus_available(),
        "OriginalData/ corpus not present (gitignored, local-only); this test is #[ignore]d and must be run explicitly on a machine that has it"
    );
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
#[ignore = "requires the gitignored OriginalData/ corpus; run with --ignored"]
fn the_ets4_and_schema21_projects_are_untouched_by_the_device_local_path() {
    assert!(
        knx_testsupport::corpus_available(),
        "OriginalData/ corpus not present (gitignored, local-only); this test is #[ignore]d and must be run explicitly on a machine that has it"
    );

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
