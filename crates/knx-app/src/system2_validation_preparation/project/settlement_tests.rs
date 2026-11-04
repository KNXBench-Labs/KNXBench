//! Owning unit controls separate record equality from caption classification and authority.
use super::*;
use crate::system2_fixture as fixture;
use serde_json::json;
use std::io::{Cursor, Read, Write};

// Reuse installed synthetic originals; no hardware observation or fabricated execution evidence.
fn pair(edit: impl Fn(&str) -> String) -> (PreparedBundle, PreparedBundle) {
    let (_dir, conn, project, profile) = fixture::configured_inputs();
    let old = prepare_project_simulated_candidate(&conn, &project, DeviceId(1), &profile)
        .unwrap()
        .bundle;
    let old_record = old.project_preparation.as_ref().unwrap();
    let ProjectProductInput::StandalonePackage { package_bytes, .. } = &old_record.product_input
    else {
        panic!("fixture must use the actual standalone original");
    };
    let mut archive = zip::ZipArchive::new(Cursor::new(package_bytes)).unwrap();
    let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let mut application = Vec::new();
    for i in 0..archive.len() {
        let mut member = archive.by_index(i).unwrap();
        let name = member.name().unwrap().into_owned();
        let mut text = String::new();
        member.read_to_string(&mut text).unwrap();
        if name.ends_with("Application.xml") {
            text = edit(&text);
            application = text.as_bytes().to_vec();
        }
        writer
            .start_file(name, zip::write::SimpleFileOptions::default())
            .unwrap();
        writer.write_all(text.as_bytes()).unwrap();
    }
    drop(archive);
    assert!(!application.is_empty());
    let package = writer.finish().unwrap().into_inner();
    let directory = tempfile::tempdir().unwrap();
    let current =
        knx_productdb::open_and_migrate(&directory.path().join("products.sqlite")).unwrap();
    knx_productdb::install_package(&current, "new-test-preview.knxprod", &package).unwrap();
    let mut profile = ProjectValidationProfile::from_json(&profile).unwrap();
    profile.source_sha256 = digest(&application);
    profile.product_input = ProjectProductSelector::StandalonePackage {
        package_sha256: digest(&package),
    };
    let new = prepare_project_simulated_candidate(
        &current,
        &project,
        DeviceId(1),
        &serde_json::to_vec(&profile).unwrap(),
    )
    .unwrap()
    .bundle;
    // Both are complete valid SOURCE preparations; neither is hardware or execution authority.
    old.plan().unwrap();
    new.plan().unwrap();
    assert!(old.binding.simulated && new.binding.simulated);
    (old, new)
}
fn check(old: &PreparedBundle, new: &PreparedBundle) -> io::Result<()> {
    compare_prepared_records(
        old.project_preparation.as_ref().unwrap(),
        new.project_preparation.as_ref().unwrap(),
        &old.normalized_plan,
        &new.normalized_plan,
    )
}
fn reason(result: io::Result<()>) {
    let error = result.expect_err("inner record guard must refuse independently of classifier");
    assert_eq!(error.kind(), io::ErrorKind::InvalidData);
    assert_eq!(
        error.to_string(),
        "original request/coverage/policy/plan differs"
    );
}

#[test]
fn record_guard_accepts_caption_provenance_only() {
    let (old, new) = pair(|text| {
        assert!(text.contains("Name=\"Synthetic\""));
        text.replacen("Name=\"Synthetic\"", "Name=\"Revised caption\"", 1)
    });
    assert_ne!(old.project_preparation, new.project_preparation);
    assert_eq!(old.normalized_plan, new.normalized_plan);
    check(&old, &new).unwrap();
}

#[test]
fn record_guard_refuses_valid_equal_plan_coverage_only_difference() {
    let (old, new) = pair(|text| {
        assert!(text.contains("Value=\"4660\""));
        text.replace("Value=\"4660\"", "Value=\"4661\"")
    });
    assert_eq!(old.normalized_plan, new.normalized_plan);
    assert_eq!(
        serde_json::to_vec(&old.normalized_plan).unwrap(),
        serde_json::to_vec(&new.normalized_plan).unwrap()
    );
    let a = old.project_preparation.as_ref().unwrap();
    let b = new.project_preparation.as_ref().unwrap();
    assert_ne!(a.coverage, b.coverage);
    // After the helper's four provenance allowances, ONLY coverage differs. This is a
    // direct defense-in-depth control, not a naturally admitted caption-class example.
    let comparable = ProjectPreparationRecord {
        source_sha256: a.source_sha256.clone(),
        product_input: a.product_input.clone(),
        profile_bytes: a.profile_bytes.clone(),
        profile_sha256: a.profile_sha256.clone(),
        ..b.as_ref().clone()
    };
    assert_eq!(
        comparable,
        ProjectPreparationRecord {
            coverage: b.coverage.clone(),
            ..a.as_ref().clone()
        }
    );
    reason(check(&old, &new));
    check(&old, &old).unwrap();
}

#[test]
fn record_guard_refuses_each_non_provenance_field() {
    let (old, _) = pair(str::to_owned);
    let a = old.project_preparation.as_ref().unwrap();
    for attack in 0..15 {
        let mut b = a.as_ref().clone();
        match attack {
            0 => b.revision += 1,
            1 => b.scope.push('!'),
            2 => b.device_id += 1,
            3 => b.device_source_path.push('!'),
            4 => b.device_source_id.push('!'),
            5 => b.project_snapshot.push(0),
            6 => b.snapshot_sha256 = "0".repeat(64),
            7 => b.project_semantic_sha256 = "0".repeat(64),
            8 => {
                b.request.values.insert("extra".into(), "1".into());
            }
            9 => b.schema_namespace.push('!'),
            10 => b.configuration_policy.push('!'),
            11 => b.plan_sha256 = "0".repeat(64),
            12 => b.request.links.reverse(),
            13 => b.request.individual_address ^= 1,
            14 => b.coverage.reverse(),
            _ => unreachable!(),
        }
        assert_ne!(b, **a, "non-vacuous field mutation {attack}");
        reason(compare_prepared_records(
            a,
            &b,
            &old.normalized_plan,
            &old.normalized_plan,
        ));
    }
    check(&old, &old).unwrap();
}

#[test]
fn record_guard_refuses_ordered_coverage_and_each_coverage_field() {
    let (old, _) = pair(str::to_owned);
    let a = old.project_preparation.as_ref().unwrap();
    assert!(a.coverage.len() >= 2);
    let first = &a.coverage[0];
    for (field, value) in [
        ("parameter_id", json!("other")),
        ("references", json!(["other"])),
        ("source_default", json!("9876")),
        ("segment", json!("other")),
        ("field", json!([100, 1, 8])),
        ("legacy_patch_always", json!(!first.legacy_patch_always)),
        (
            "default_union_parameter",
            json!(!first.default_union_parameter),
        ),
        ("active_reference", json!("other")),
        ("disposition", json!("other")),
    ] {
        let mut coverage = serde_json::to_value(first).unwrap();
        assert_ne!(coverage[field], value);
        coverage[field] = value;
        let mut b = a.as_ref().clone();
        b.coverage[0] = serde_json::from_value(coverage).unwrap();
        reason(compare_prepared_records(
            a,
            &b,
            &old.normalized_plan,
            &old.normalized_plan,
        ));
    }
    for duplicate in [false, true] {
        let mut b = a.as_ref().clone();
        if duplicate {
            b.coverage.push(first.clone());
        } else {
            b.coverage.reverse();
        }
        assert_ne!(b.coverage, a.coverage);
        reason(compare_prepared_records(
            a,
            &b,
            &old.normalized_plan,
            &old.normalized_plan,
        ));
    }
    check(&old, &old).unwrap();
}

#[test]
fn record_guard_refuses_changed_plan_values_and_order() {
    let (old, _) = pair(str::to_owned);
    let record = old.project_preparation.as_ref().unwrap();
    for attack in 0..6 {
        let mut plan = old.normalized_plan.clone();
        match attack {
            0 => plan.target = "1.1.26".into(),
            1 => plan.mask ^= 1,
            2 => plan.manufacturer ^= 1,
            3 => plan.steps.reverse(),
            4 => plan.steps[0].source_index += 1,
            5 => {
                let bytes = plan
                    .steps
                    .iter_mut()
                    .find_map(|step| match &mut step.operation {
                        crate::property_validation_bundle::OperationRecord::WriteMemory {
                            data,
                            ..
                        } => Some(data),
                        _ => None,
                    })
                    .unwrap();
                bytes[0] ^= 1;
            }
            _ => unreachable!(),
        }
        assert_ne!(plan, old.normalized_plan);
        reason(compare_prepared_records(
            record,
            record,
            &old.normalized_plan,
            &plan,
        ));
    }
    check(&old, &old).unwrap();
}
