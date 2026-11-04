//! Rehashed originals and ownership adversaries keep caption settlement fail-closed.
use super::*;
use knx_app::property_validation_bundle::{OperationRecord, PreparedBundle};
use knx_app::system2_validation_preparation::{ProjectProductInput, RequestRecord};
use knx_productdb::image_request::image_request_for_device;
use serde_json::{json, Value};

// Refresh outer digests deliberately: refusal must reach reconstruction/grammar, not a stale hash.
fn rehash(bundle: &mut PreparedBundle) {
    let source = bundle.project_preparation.as_mut().unwrap();
    source.profile_sha256 = knx_productdb::sha256_hex(&source.profile_bytes);
    source.plan_sha256 = bundle.normalized_plan.sha256().unwrap();
    bundle.binding.profile_sha256 = source.profile_sha256.clone();
    bundle.binding.plan_sha256 = source.plan_sha256.clone();
    let hardware = bundle.hardware_preparation.as_mut().unwrap();
    hardware.source_preparation_sha256 = source.sha256().unwrap();
    bundle.binding.project_preparation_sha256 = Some(hardware.sha256().unwrap());
}
fn refused(input: &f::Inputs, bad: &PreparedBundle, label: &str) {
    let before = input.project.clone();
    let frozen = serde_json::to_vec(bad).unwrap();
    let writes = input.conn.total_changes();
    assert!(
        current_project_hardware_parts(&input.conn, &input.project, f::DEVICE, bad).is_err(),
        "{label} must refuse even after refreshing outer hashes"
    );
    assert_eq!(input.project, before, "{label}: project remains immutable");
    assert_eq!(
        serde_json::to_vec(bad).unwrap(),
        frozen,
        "{label}: original remains immutable"
    );
    assert_eq!(
        input.conn.total_changes(),
        writes,
        "{label}: read proof never repairs catalogue"
    );
}

#[tokio::test]
async fn caption_rehashed_profile_fields_and_closed_json_cannot_be_repinned() {
    for retained in [false, true] {
        for namespace in [20, 23] {
            let input = f::inputs(namespace, retained);
            let original = observed(&input.conn, &input.project, &input.profile).await;
            let changed = f::caption(&input.package);
            let input = input.replace(&changed, false);
            assert_eq!(
                parts(&input.conn, &input.project, &original),
                Some(ProgrammingParts::ALL)
            );
            let profile: Value = serde_json::from_slice(&input.profile).unwrap();
            let other_namespace = if namespace == 20 { 23 } else { 20 };
            for (field, value) in [
                ("format", json!(1)),
                ("id", json!("other-original-profile")),
                ("version", json!(3)),
                ("classification", json!("hardware")),
                ("configuration", json!("defaults")),
                ("program_id", json!("other-program")),
                (
                    "schema_namespace",
                    json!(format!("http://knx.org/xml/project/{other_namespace}")),
                ),
                ("source_sha256", json!("0".repeat(64))),
                (
                    "product_input",
                    if retained {
                        json!({"kind":"standalone-package-v1","package_sha256":"0".repeat(64)})
                    } else {
                        json!({"kind":"retained-source-members-v1","closure_sha256":"0".repeat(64)})
                    },
                ),
                ("mask", json!(0x0701)),
                ("manufacturer", json!(2)),
                ("hardware", json!([0, 1, 0, 0, 0, 92])),
                ("application", json!([0, 1, 146, 5, 5])),
                ("numeric_order", json!("low-octet-first")),
                ("bit_union_offsets", json!("lsb0")),
                ("legacy_patch_always", json!("ignore")),
                ("invisible_parameters", json!("default-value")),
                ("binary_restrictions", json!("unrestricted")),
                ("reference_activation", json!("all-active")),
                (
                    "procedure_policy",
                    json!("declared-counts-zero-flags-full-table-identity-v1"),
                ),
                ("unknown", json!(true)),
            ] {
                let mut value_profile = profile.clone();
                value_profile[field] = value;
                let mut bad = original.clone();
                bad.project_preparation.as_mut().unwrap().profile_bytes =
                    serde_json::to_vec(&value_profile).unwrap();
                rehash(&mut bad);
                refused(&input, &bad, field);
            }
            let mut duplicate = input.profile.clone();
            assert_eq!(duplicate.pop(), Some(b'}'));
            duplicate.extend_from_slice(b",\"version\":2}");
            let mut bad = original.clone();
            bad.project_preparation.as_mut().unwrap().profile_bytes = duplicate;
            rehash(&mut bad);
            refused(&input, &bad, "duplicate profile key");
        }
    }
}

#[tokio::test]
async fn caption_rehashed_original_coverage_request_and_plan_must_reconstruct_exactly() {
    for retained in [false, true] {
        let input = f::inputs(23, retained);
        let original = observed(&input.conn, &input.project, &input.profile).await;
        let frozen = serde_json::to_vec(&original).unwrap();
        let changed = f::caption(&input.package);
        let input = input.replace(&changed, false);
        let coverage = &original.project_preparation.as_ref().unwrap().coverage;
        assert!(!coverage.is_empty());
        let first = serde_json::to_value(&coverage[0]).unwrap();
        for (field, replacement) in [
            ("parameter_id", json!("other-parameter")),
            ("references", json!(["other-reference"])),
            ("source_default", json!("9876")),
            ("segment", json!("other-segment")),
            ("field", json!([100, 1, 8])),
            (
                "legacy_patch_always",
                json!(!coverage[0].legacy_patch_always),
            ),
            (
                "default_union_parameter",
                json!(!coverage[0].default_union_parameter),
            ),
            ("active_reference", json!("other-active-reference")),
            ("disposition", json!("unreviewed-disposition")),
        ] {
            assert_ne!(first[field], replacement, "non-vacuous mutation {field}");
            let mut value = first.clone();
            value[field] = replacement;
            let mut bad = original.clone();
            bad.project_preparation.as_mut().unwrap().coverage[0] =
                serde_json::from_value(value).unwrap();
            rehash(&mut bad);
            assert_eq!(
                bad.normalized_plan, original.normalized_plan,
                "coverage adversary has equal plan"
            );
            refused(&input, &bad, field);
        }
        for attack in 0..15 {
            let mut bad = original.clone();
            match attack {
                0 => {
                    bad.project_preparation
                        .as_mut()
                        .unwrap()
                        .request
                        .values
                        .insert("unconsumed".into(), "1".into());
                }
                1 => {
                    bad.project_preparation
                        .as_mut()
                        .unwrap()
                        .request
                        .links
                        .reverse();
                }
                2 => bad
                    .project_preparation
                    .as_mut()
                    .unwrap()
                    .device_source_path
                    .push('!'),
                3 => {
                    let source = bad.project_preparation.as_mut().unwrap();
                    let mut project = source.decoded_project().unwrap();
                    parameter(&mut project, "772");
                    let snapshot = knx_store::project_history::NativeSnapshot {
                        project,
                        opaque: vec![],
                        manufacturer_refs: vec![],
                    };
                    source.project_snapshot = snapshot.encode().unwrap();
                    source.snapshot_sha256 = knx_productdb::sha256_hex(&source.project_snapshot);
                    source.project_semantic_sha256 = snapshot.semantic_hash().unwrap();
                }
                4 => bad.normalized_plan.steps[0].source_index += 1,
                5 => {
                    let data = bad
                        .normalized_plan
                        .steps
                        .iter_mut()
                        .find_map(|s| match &mut s.operation {
                            OperationRecord::WriteMemory { data, .. } => Some(data),
                            _ => None,
                        })
                        .unwrap();
                    data[0] ^= 1;
                }
                6 => {
                    let address = bad
                        .normalized_plan
                        .steps
                        .iter_mut()
                        .find_map(|s| match &mut s.operation {
                            OperationRecord::ToolMetadata { table_address } => Some(table_address),
                            _ => None,
                        })
                        .unwrap();
                    *address += 1;
                }
                7 => match &mut bad.project_preparation.as_mut().unwrap().product_input {
                    ProjectProductInput::StandalonePackage { package_bytes, .. } => {
                        package_bytes[0] ^= 1
                    }
                    ProjectProductInput::RetainedSourceMembers { member_bytes, .. } => {
                        member_bytes[1][0] ^= 1
                    }
                },
                8 | 9 => {
                    let operation = bad
                        .normalized_plan
                        .steps
                        .iter_mut()
                        .find_map(|s| match &mut s.operation {
                            OperationRecord::WriteMemory {
                                object, address, ..
                            } => Some((object, address)),
                            _ => None,
                        })
                        .unwrap();
                    if attack == 8 {
                        *operation.1 += 1;
                    } else {
                        *operation.0 ^= 1;
                    }
                }
                10 | 14 => {
                    let octets = bad
                        .normalized_plan
                        .steps
                        .iter_mut()
                        .find_map(|s| match &mut s.operation {
                            OperationRecord::LoadControl { octets, .. }
                                if octets[0] == 3
                                    && octets[1] == if attack == 10 { 2 } else { 0 } =>
                            {
                                Some(octets)
                            }
                            _ => None,
                        })
                        .unwrap();
                    octets[if attack == 10 { 5 } else { 4 }] ^= 1;
                }
                11 => {
                    let expected = bad
                        .normalized_plan
                        .steps
                        .iter_mut()
                        .find_map(|s| match &mut s.operation {
                            OperationRecord::HardwareCheck { expected } => Some(expected),
                            _ => None,
                        })
                        .unwrap();
                    expected[5] ^= 1;
                }
                12 => bad.normalized_plan.mask ^= 1,
                13 => bad.normalized_plan.manufacturer ^= 1,
                _ => unreachable!(),
            }
            assert_ne!(bad, original, "non-vacuous original mutation {attack}");
            rehash(&mut bad);
            refused(&input, &bad, &format!("original attack {attack}"));
        }
        assert_eq!(
            parts(&input.conn, &input.project, &original),
            Some(ProgrammingParts::ALL)
        );
        assert_eq!(serde_json::to_vec(&original).unwrap(), frozen);
    }
}

#[tokio::test]
async fn caption_changed_source_missing_winners_blobs_and_occurrences_never_repair() {
    for retained in [false, true] {
        for attack in [
            "DELETE FROM hardware2program",
            "DELETE FROM application_program",
            "DELETE FROM source_identity WHERE table_name='application_program'",
            "DELETE FROM source_file WHERE sha256=(SELECT source_sha256 FROM application_program LIMIT 1)",
            "UPDATE source_file SET bytes=x'00' WHERE sha256=(SELECT source_sha256 FROM application_program LIMIT 1)",
        ] {
            let input = f::inputs(23, retained);
            let bundle = observed(&input.conn, &input.project, &input.profile).await;
            let changed = f::caption(&input.package);
            let input = input.replace(&changed, false);
            assert_eq!(parts(&input.conn, &input.project, &bundle), Some(ProgrammingParts::ALL));
            input.conn.execute_batch("PRAGMA foreign_keys=OFF").unwrap();
            input.conn.execute_batch(attack).unwrap();
            assert!(input.conn.changes() > 0, "attack must affect real evidence: {attack}");
            refused(&input, &bundle, attack);
        }
    }
}

#[tokio::test]
async fn caption_standalone_missing_memberships_and_corrupt_measured_container_refuse() {
    for attack in 0..5 {
        let input = f::inputs(23, false);
        let bundle = observed(&input.conn, &input.project, &input.profile).await;
        let changed = f::caption(&input.package);
        let input = input.replace(&changed, false);
        assert_eq!(
            parts(&input.conn, &input.project, &bundle),
            Some(ProgrammingParts::ALL)
        );
        let source: String = input
            .conn
            .query_row(
                "SELECT source_sha256 FROM application_program WHERE id=?1",
                [f::PROGRAM],
                |r| r.get(0),
            )
            .unwrap();
        let package = knx_productdb::sha256_hex(&changed);
        input.conn.execute_batch("PRAGMA foreign_keys=OFF").unwrap();
        let affected = match attack {
            0 => input.conn.execute(
                "DELETE FROM package_member WHERE source_sha256=?1",
                [&source],
            ),
            1 => input.conn.execute(
                "UPDATE package_member SET path='different/Application.xml' WHERE source_sha256=?1",
                [&source],
            ),
            2 => input
                .conn
                .execute("UPDATE package SET bytes=x'00' WHERE sha256=?1", [&package]),
            3 => input.conn.execute("DELETE FROM package WHERE sha256=?1", [&package]),
            _ => input.conn.execute(
                "INSERT INTO package_member SELECT package_sha256, ordinal+100, 'alias/Application.xml', role, source_sha256, size FROM package_member WHERE source_sha256=?1",
                [&source],
            ),
        }
        .unwrap();
        assert!(affected > 0);
        refused(&input, &bundle, &format!("package attack {attack}"));
    }
}

#[tokio::test]
async fn caption_current_request_reads_unrelated_edits_and_rejects_unreadable_flag_overrides() {
    for retained in [false, true] {
        for namespace in [20, 23] {
            let input = f::inputs(namespace, retained);
            let bundle = observed(&input.conn, &input.project, &input.profile).await;
            let changed = f::caption(&input.package);
            let input = input.replace(&changed, false);
            let written = bundle.project_preparation.as_ref().unwrap().request.clone();
            let mut current = input.project.clone();
            let mut other = current.devices.get(f::DEVICE).unwrap().clone();
            other.id = current.ids.next_device_id().unwrap();
            other.address = Some("2.2.2".parse().unwrap());
            other.name = "Unrelated current device".into();
            other.source.ets_id = "KB-UNRELATED".into();
            other.com_objects.clear();
            current.installations[0].topology.unassigned.push(other.id);
            current.devices.insert(other);
            assert_eq!(
                RequestRecord::from_request(
                    &image_request_for_device(&input.conn, &current, f::DEVICE).unwrap()
                ),
                written
            );
            assert_eq!(
                parts(&input.conn, &current, &bundle),
                Some(ProgrammingParts::ALL)
            );
            parameter(&mut current, "772");
            let request = RequestRecord::from_request(
                &image_request_for_device(&input.conn, &current, f::DEVICE).unwrap(),
            );
            assert_ne!(request.values, written.values);
            assert_eq!(request.links, written.links);
            assert_eq!(request.flag_overrides, written.flag_overrides);
            assert_eq!(parts(&input.conn, &current, &bundle), Some(ADR_APP));
            for flag in [
                knx_core::provenance::Override::Empty,
                knx_core::provenance::Override::Malformed("Perhaps".into()),
            ] {
                let mut malformed = input.project.clone();
                malformed
                    .devices
                    .com_object_mut(ComObjectInstanceId(1))
                    .unwrap()
                    .flags
                    .write = flag;
                assert!(matches!(
                    image_request_for_device(&input.conn, &malformed, f::DEVICE),
                    Err(knx_productdb::image_request::DeviceRequestError::Project(
                        knx_productdb::image_request::ProjectRequestError::UnreadableFlag { .. }
                    ))
                ));
                assert_eq!(parts(&input.conn, &malformed, &bundle), None);
            }
        }
    }
}

// Explicit new pins are test-owned previews only. Never repin the observed original.
fn new_profile(input: &f::Inputs, package: &[u8]) -> Vec<u8> {
    use knx_app::system2_validation_preparation::{
        describe_project_source_closure, ProjectProductSelector, ProjectValidationProfile,
    };
    let mut profile = ProjectValidationProfile::from_json(&input.profile).unwrap();
    let closure = describe_project_source_closure(
        &input.conn,
        &input.project,
        f::DEVICE,
        &profile.schema_namespace,
    )
    .unwrap();
    profile.source_sha256 = closure.members[1].source_sha256.clone();
    profile.product_input = match profile.product_input {
        ProjectProductSelector::StandalonePackage { .. } => {
            ProjectProductSelector::StandalonePackage {
                package_sha256: knx_productdb::sha256_hex(package),
            }
        }
        ProjectProductSelector::RetainedSourceMembers { .. } => {
            ProjectProductSelector::RetainedSourceMembers {
                closure_sha256: closure.sha256().unwrap(),
            }
        }
    };
    serde_json::to_vec(&profile).unwrap()
}

#[tokio::test]
async fn caption_equal_plan_with_changed_coverage_does_not_establish_source_equivalence() {
    for retained in [false, true] {
        for namespace in [20, 23] {
            let input = f::inputs(namespace, retained);
            let original = observed(&input.conn, &input.project, &input.profile).await;
            let changed = f::changed(&input.package, |s| {
                assert!(s.contains("Value=\"4660\""));
                s.replace("Name=\"Synthetic\"", "Name=\"Revised caption\"")
                    .replace("Value=\"4660\"", "Value=\"4661\"")
            });
            let input = input.replace(&changed, false);
            let preview = prepare_project_hardware_source(
                &input.conn,
                &input.project,
                f::DEVICE,
                &new_profile(&input, &changed),
                "new-preview-only",
                RecoveryTrack::KnxBenchBestEffort,
            )
            .unwrap();
            assert_eq!(preview.preview().normalized_plan, original.normalized_plan);
            assert_eq!(
                serde_json::to_vec(&preview.preview().normalized_plan).unwrap(),
                serde_json::to_vec(&original.normalized_plan).unwrap()
            );
            assert_ne!(
                preview
                    .preview()
                    .project_preparation
                    .as_ref()
                    .unwrap()
                    .coverage,
                original.project_preparation.as_ref().unwrap().coverage,
                "active project override preserves plan while source default/coverage changes"
            );
            assert!(preview.preview().binding.simulated);
            refused(&input, &original, "equal plan with different coverage");
        }
    }
}

#[tokio::test]
async fn caption_new_exact_pins_do_not_remove_mandatory_unsupported_options_refusal() {
    for retained in [false, true] {
        for options in ["None", "DefaultValue", "unknown"] {
            let input = f::inputs(23, retained);
            let original = observed(&input.conn, &input.project, &input.profile).await;
            let changed = f::changed(&input.package, |s| {
                assert!(s.contains("DownloadInvisibleParameters=\"Background\""));
                s.replace("Name=\"Synthetic\"", "Name=\"Revised caption\"")
                    .replace(
                        "DownloadInvisibleParameters=\"Background\"",
                        &format!("DownloadInvisibleParameters=\"{options}\""),
                    )
            });
            let input = input.replace(&changed, false);
            let error = match prepare_project_hardware_source(
                &input.conn,
                &input.project,
                f::DEVICE,
                &new_profile(&input, &changed),
                "unsupported-preview",
                RecoveryTrack::KnxBenchBestEffort,
            ) {
                Err(error) => error,
                Ok(_) => panic!("fresh measured pins admitted unsupported Options: {options}"),
            };
            let expected = if options == "DefaultValue" {
                "source-template retention requires explicit DownloadInvisibleParameters=Background"
            } else {
                "unrecognized DownloadInvisibleParameters value"
            };
            assert!(
                error.to_string().contains(expected),
                "owning Options refusal: {error}"
            );
            refused(&input, &original, options);
        }
    }
}
