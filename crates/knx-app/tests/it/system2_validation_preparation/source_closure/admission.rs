//! Synthetic admission and retained-original reproduction controls.
use super::*;
use knx_app::system2_validation_preparation::ProjectProductInput;

#[test]
fn rehashed_snapshot_program_ref_and_authentic_wrong_bridge_witness_cannot_bypass_reconstruction() {
    let original = source_bundle(20);
    let mut changed = original.clone();
    let record = changed.project_preparation.as_mut().unwrap();
    let mut project = record.decoded_project().unwrap();
    project
        .devices
        .get_mut(knx_core::DeviceId(1))
        .unwrap()
        .program_ref = "absent-bridge".into();
    let snapshot = knx_store::project_history::NativeSnapshot {
        project,
        opaque: vec![],
        manufacturer_refs: vec![],
    };
    record.project_snapshot = snapshot.encode().unwrap();
    record.snapshot_sha256 = knx_productdb::sha256_hex(&record.project_snapshot);
    record.project_semantic_sha256 = snapshot.semantic_hash().unwrap();
    rebind_record(&mut changed);
    assert!(changed.plan().is_err());
    let mut changed = original;
    let record = changed.project_preparation.as_mut().unwrap();
    if let ProjectProductInput::RetainedSourceMembers {
        descriptor,
        member_bytes,
        ..
    } = &mut record.product_input
    {
        let wrong = String::from_utf8(member_bytes[0].clone())
            .unwrap()
            .replace(SYN_ID, "absent-application")
            .into_bytes();
        let dir = tempfile::tempdir().unwrap();
        let conn = knx_productdb::open_and_migrate(&dir.path().join("witness.sqlite")).unwrap();
        knx_productdb::ingest_file(&conn, &descriptor.members[0].source_path, &wrong).unwrap();
        let sha = knx_productdb::sha256_hex(&wrong);
        let witness = knx_productdb::identity::owned_source_identity(
            &conn,
            knx_productdb::identity::IdentityKind::Hardware2Program,
            &descriptor.program_ref,
            &sha,
        )
        .unwrap();
        descriptor.members[0].source_sha256 = sha;
        descriptor.members[0].byte_len = wrong.len() as u64;
        descriptor.members[0].owner.element_digest = witness.digest;
        member_bytes[0] = wrong;
    }
    repin_closure(&mut changed);
    assert!(
        changed.plan().is_err(),
        "a fully hash-consistent actual declaration is not proof of the retained request's bridge"
    );
}

pub(super) fn source_inputs(
    namespace: u8,
) -> (
    tempfile::TempDir,
    knx_productdb::Connection,
    knx_core::Project,
    Vec<u8>,
) {
    let dir = tempfile::tempdir().unwrap();
    let conn = knx_productdb::open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
    let (package, source) = synthetic_package(namespace);
    let mut zip = zip::ZipArchive::new(Cursor::new(&package)).unwrap();
    for path in ["M-0000/Hardware.xml", "M-0000/Application.xml"] {
        let mut bytes = Vec::new();
        zip.by_name(path).unwrap().read_to_end(&mut bytes).unwrap();
        knx_productdb::ingest_file(&conn, path, &bytes).unwrap();
    }
    let project = synthetic_project();
    let profile = source_profile(
        &conn,
        &project,
        namespace,
        &synthetic_profile(namespace, &package, &source),
    );
    (dir, conn, project, profile)
}
pub(super) fn source_bundle(namespace: u8) -> PreparedBundle {
    let (_dir, conn, project, profile) = source_inputs(namespace);
    project_candidate(&conn, &project, &profile).bundle
}
pub(super) fn rebind_record(bundle: &mut PreparedBundle) {
    let record = bundle.project_preparation.as_mut().unwrap();
    record.profile_sha256 = knx_productdb::sha256_hex(&record.profile_bytes);
    bundle.binding.profile_sha256 = record.profile_sha256.clone();
    bundle.binding.source_sha256 = record.source_sha256.clone();
    record.plan_sha256 = bundle.normalized_plan.sha256().unwrap();
    bundle.binding.plan_sha256 = record.plan_sha256.clone();
    bundle.binding.project_preparation_sha256 = Some(record.sha256().unwrap());
}
pub(super) fn repin_closure(bundle: &mut PreparedBundle) {
    let record = bundle.project_preparation.as_mut().unwrap();
    if let ProjectProductInput::RetainedSourceMembers {
        descriptor,
        closure_sha256,
        ..
    } = &mut record.product_input
    {
        *closure_sha256 = descriptor.sha256().unwrap();
        let mut profile: serde_json::Value = serde_json::from_slice(&record.profile_bytes).unwrap();
        profile["product_input"]["closure_sha256"] = closure_sha256.clone().into();
        profile["source_sha256"] = record.source_sha256.clone().into();
        record.profile_bytes = serde_json::to_vec(&profile).unwrap();
    } else {
        panic!("source fixture variant");
    }
    rebind_record(bundle);
}
#[test]
fn imported_and_unsaved_source_closures_roundtrip_replay_without_inputs_and_match_package_plans() {
    for namespace in [20, 23] {
        for imported in [false, true] {
            let (dir, conn, project, profile) = if imported {
                let (dir, conn, project, old_profile) = imported_sources(namespace);
                let profile = source_profile(&conn, &project, namespace, &old_profile);
                (dir, conn, project, profile)
            } else {
                source_inputs(namespace)
            };
            let before = project.clone();
            let changes = conn.total_changes();
            let candidate = project_candidate(&conn, &project, &profile);
            let bundle = candidate.bundle;
            let again = project_candidate(&conn, &project, &profile).bundle;
            assert_eq!(bundle, again);
            assert_eq!(project, before);
            assert_eq!(conn.total_changes(), changes);
            assert!(bundle.rebind_hardware([7; 6]).is_err());
            let record = bundle.project_preparation.as_ref().unwrap();
            let ProjectProductInput::RetainedSourceMembers {
                descriptor,
                member_bytes,
                closure_sha256,
            } = &record.product_input
            else {
                panic!("source fixture variant")
            };
            assert_eq!(descriptor.closure_revision, 1);
            assert_eq!(descriptor.members.len(), 2);
            assert_eq!(descriptor.members[0].owner.logical_id, "M-0000_H-1_HP-1");
            assert_eq!(descriptor.members[1].owner.logical_id, SYN_ID);
            assert_eq!(descriptor.members[0].source_path, "M-0000/Hardware.xml");
            assert_eq!(descriptor.members[1].source_path, "M-0000/Application.xml");
            assert_ne!(
                descriptor.members[0].source_sha256,
                descriptor.members[1].source_sha256
            );
            assert_ne!(closure_sha256, &record.source_sha256);
            let (_package_dir, package_conn, _, package_profile) = synthetic_inputs(namespace);
            let package = project_candidate(&package_conn, &project, &package_profile).bundle;
            assert_eq!(package.normalized_plan, bundle.normalized_plan);
            assert_eq!(
                package.project_preparation.as_ref().unwrap().request,
                record.request
            );
            assert_eq!(
                package.project_preparation.as_ref().unwrap().coverage,
                record.coverage
            );
            assert_ne!(
                package.binding.project_preparation_sha256,
                bundle.binding.project_preparation_sha256
            );
            let replay_db =
                knx_productdb::open_and_migrate(&dir.path().join("reingest.sqlite")).unwrap();
            for (member, bytes) in descriptor.members.iter().zip(member_bytes) {
                assert_eq!(member.byte_len, bytes.len() as u64);
                assert_eq!(member.source_sha256, knx_productdb::sha256_hex(bytes));
                knx_productdb::ingest_file(&replay_db, &member.source_path, bytes).unwrap();
            }
            for table in ["package", "package_member"] {
                assert_eq!(
                    replay_db
                        .query_row(&format!("SELECT count(*) FROM {table}"), [], |r| r
                            .get::<_, i64>(0))
                        .unwrap(),
                    0
                );
            }
            let file = dir.path().join("bundle.json");
            std::fs::write(&file, serde_json::to_vec(&bundle).unwrap()).unwrap();
            if let Some(output) = std::env::var_os("KNXBENCH_SYNTHETIC_ARTIFACT_DIR") {
                std::fs::create_dir_all(&output).unwrap();
                std::fs::copy(
                    &file,
                    std::path::PathBuf::from(output).join(format!(
                        "source-closure-{namespace}-imported-{imported}.json"
                    )),
                )
                .unwrap();
            }
            drop(replay_db);
            drop(conn);
            drop(project);
            drop(profile);
            let read = app::read_bundle(&file).unwrap();
            assert_eq!(read, bundle);
            let replay =
                knx_app::system2_validation_preparation::replay_project_simulated_candidate(&read)
                    .unwrap();
            assert_eq!(replay.bundle, bundle);
        }
    }
}
#[test]
fn source_closure_refuses_missing_stale_duplicate_and_forged_owner_evidence_without_repairs() {
    for sql in [
        "DELETE FROM hardware2program",
        "DELETE FROM application_program",
        "UPDATE hardware2program SET application_program_ref='absent'",
        "DELETE FROM source_parse_evidence",
        "DELETE FROM source_identity_scan",
        "UPDATE source_identity_scan SET scanner=scanner+1",
        "UPDATE source_identity_scan SET status='unavailable',reason='synthetic unavailable'",
        "DELETE FROM source_identity WHERE table_name='hardware2program'",
        "DELETE FROM source_identity WHERE table_name='application_program'",
        "INSERT INTO source_identity SELECT source_sha256,table_name,logical_id,2,digest FROM source_identity WHERE table_name='hardware2program'",
        "INSERT INTO source_identity SELECT source_sha256,table_name,logical_id,2,digest FROM source_identity WHERE table_name='application_program'",
        "UPDATE source_identity SET digest='gggggggggggggggggggggggggggggggggggggggggggggggggggggggggggggggg' WHERE table_name='hardware2program'",
        "UPDATE hardware2program SET source_sha256=(SELECT source_sha256 FROM application_program LIMIT 1)",
        "UPDATE source_file SET source_path='../escape.xml'",
    ] {
        let (_dir,conn,project,profile)=source_inputs(20);
        conn.execute_batch("PRAGMA foreign_keys=OFF").unwrap();
        conn.execute_batch(sql).unwrap();
        let changes=conn.total_changes();
        assert!(knx_app::system2_validation_preparation::prepare_project_simulated_candidate(&conn,&project,knx_core::DeviceId(1),&profile).is_err(),"must refuse {sql}");
        assert_eq!(conn.total_changes(),changes);
    }
    // Forged row still has a valid witness for the logical ID, but its actual
    // winning bytes point at a different application. Fresh ingest must catch it.
    let (_dir, conn, project, profile) = source_inputs(20);
    conn.execute(
        "UPDATE hardware2program SET application_program_ref=?1",
        [SYN_ID],
    )
    .unwrap();
    let mut forged = project.clone();
    let instance = forged.devices.get_mut(knx_core::DeviceId(1)).unwrap();
    instance.program_ref = "forged-HP".into();
    conn.execute("INSERT INTO hardware2program(id,hardware_id,manufacturer_id,application_program_ref,source_sha256) SELECT 'forged-HP',hardware_id,manufacturer_id,application_program_ref,source_sha256 FROM hardware2program",[]).unwrap();
    conn.execute("INSERT INTO source_identity SELECT source_sha256,table_name,'forged-HP',1,digest FROM source_identity WHERE table_name='hardware2program'",[]).unwrap();
    let forged_profile = source_profile(&conn, &forged, 20, &profile);
    assert!(
        knx_app::system2_validation_preparation::prepare_project_simulated_candidate(
            &conn,
            &forged,
            knx_core::DeviceId(1),
            &forged_profile
        )
        .is_err()
    );
}
#[test]
fn source_record_rehashing_cannot_invent_mapping_owner_or_request_plan_coverage() {
    let original = source_bundle(20);
    for variant in 0..8 {
        let mut bundle = original.clone();
        let record = bundle.project_preparation.as_mut().unwrap();
        match variant {
            0 => {
                if let ProjectProductInput::RetainedSourceMembers { descriptor, .. } =
                    &mut record.product_input
                {
                    descriptor.members[0].owner.element_digest = "a".repeat(64)
                }
            }
            1 => {
                if let ProjectProductInput::RetainedSourceMembers { descriptor, .. } =
                    &mut record.product_input
                {
                    descriptor.members[0].owner.logical_id = "invented".into()
                }
            }
            2 => record.request.program_id = "invented".into(),
            3 => record.coverage.clear(),
            4 => {
                record.request.values.insert(syn_ref("R"), "772".into());
            }
            5 => {
                if let ProjectProductInput::RetainedSourceMembers { descriptor, .. } =
                    &mut record.product_input
                {
                    descriptor.program_ref = "invented".into();
                    descriptor.members[0].owner.logical_id = "invented".into()
                }
            }
            6 => {
                if let ProjectProductInput::RetainedSourceMembers { member_bytes, .. } =
                    &mut record.product_input
                {
                    member_bytes[0].clear()
                }
            }
            7 => {
                bundle.normalized_plan.steps.pop();
            }
            _ => unreachable!(),
        }
        repin_closure(&mut bundle);
        assert!(
            bundle.plan().is_err(),
            "rehash variant {variant} must fail authoritative reconstruction"
        );
    }
}
#[test]
fn compact_source_members_preserve_order_bytes_and_identity_and_refuse_numeric_pairs() {
    use base64::{engine::general_purpose::STANDARD, Engine};
    let bundle = source_bundle(20);
    let record = bundle.project_preparation.as_ref().unwrap();
    let identity = record.sha256().unwrap();
    let ProjectProductInput::RetainedSourceMembers { member_bytes, .. } = &record.product_input
    else {
        panic!("source fixture variant");
    };
    let value = serde_json::to_value(&bundle).unwrap();
    assert_eq!(
        value["project_preparation"]["product_input"]["member_bytes"],
        serde_json::json!([
            STANDARD.encode(&member_bytes[0]),
            STANDARD.encode(&member_bytes[1])
        ])
    );
    let reopened: PreparedBundle = serde_json::from_value(value.clone()).unwrap();
    assert_eq!(reopened, bundle);
    assert_eq!(
        reopened
            .project_preparation
            .as_ref()
            .unwrap()
            .sha256()
            .unwrap(),
        identity
    );
    let mut bad = value;
    bad["project_preparation"]["product_input"]["member_bytes"] = serde_json::json!([[], []]);
    assert!(
        serde_json::from_value::<PreparedBundle>(bad).is_err(),
        "two numeric arrays must not be a legacy fallback"
    );
}

#[test]
fn source_record_inventory_has_exact_two_ordered_roles_and_safe_distinct_paths() {
    let original = source_bundle(20);
    for variant in 0..9 {
        let mut bundle = original.clone();
        if let ProjectProductInput::RetainedSourceMembers {
            descriptor,
            member_bytes,
            ..
        } = &mut bundle.project_preparation.as_mut().unwrap().product_input
        {
            match variant {
                0 => descriptor.members.swap(0, 1),
                1 => descriptor.members[1].role = descriptor.members[0].role,
                2 => descriptor.members[1].source_path = descriptor.members[0].source_path.clone(),
                3 => descriptor.members[0].source_path = "/absolute.xml".into(),
                4 => descriptor.members[0].source_path = "M-0000/../Hardware.xml".into(),
                5 => descriptor.members[0].source_path = "M-0000\\Hardware.xml".into(),
                6 => descriptor.members[0].byte_len += 1,
                7 => {
                    descriptor.members[0].source_sha256 =
                        descriptor.members[1].source_sha256.clone();
                    member_bytes[0] = member_bytes[1].clone();
                }
                8 => descriptor.members[0].source_path = "x".repeat(1025),
                _ => unreachable!(),
            }
        }
        repin_closure(&mut bundle);
        assert!(bundle.plan().is_err(), "inventory variant {variant}");
    }
    let value = serde_json::to_value(&original).unwrap();
    for len in [0, 1, 3] {
        let mut bad = value.clone();
        let member =
            bad["project_preparation"]["product_input"]["descriptor"]["members"][0].clone();
        bad["project_preparation"]["product_input"]["descriptor"]["members"] =
            serde_json::Value::Array(vec![member; len]);
        assert!(serde_json::from_value::<PreparedBundle>(bad).is_err());
        let mut bad = value.clone();
        bad["project_preparation"]["product_input"]["member_bytes"] =
            serde_json::Value::Array(vec![serde_json::json!(""); len]);
        assert!(serde_json::from_value::<PreparedBundle>(bad).is_err());
    }
}
#[test]
fn source_selector_never_downgrades_package_mode_or_overloads_the_application_hash() {
    let (_dir, conn, project, profile) = source_inputs(20);
    let mut value: serde_json::Value = serde_json::from_slice(&profile).unwrap();
    let closure = value["product_input"]["closure_sha256"].clone();
    value["product_input"] =
        serde_json::json!({"kind":"standalone-package-v1","package_sha256":closure});
    assert!(
        knx_app::system2_validation_preparation::prepare_project_simulated_candidate(
            &conn,
            &project,
            knx_core::DeviceId(1),
            &serde_json::to_vec(&value).unwrap()
        )
        .is_err()
    );
    let mut value: serde_json::Value = serde_json::from_slice(&profile).unwrap();
    value["source_sha256"] = value["product_input"]["closure_sha256"].clone();
    assert!(
        knx_app::system2_validation_preparation::prepare_project_simulated_candidate(
            &conn,
            &project,
            knx_core::DeviceId(1),
            &serde_json::to_vec(&value).unwrap()
        )
        .is_err()
    );
    value.as_object_mut().unwrap().remove("product_input");
    assert!(
        knx_app::system2_validation_preparation::ProjectValidationProfile::from_json(
            &serde_json::to_vec(&value).unwrap()
        )
        .is_err()
    );
}
