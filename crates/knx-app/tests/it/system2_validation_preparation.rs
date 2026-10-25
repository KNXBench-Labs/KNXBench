//! Source-bound preparation keeps experimental defaults distinct from physical admission.

mod configured_hardware;
mod source_closure;

use knx_app::system2_validation_preparation::{prepare_simulated_candidate, ValidationProfile};
use knx_core::IndividualAddress;
use std::path::PathBuf;

mod fixture;
use fixture::*;

fn synthetic_inputs(
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
    knx_productdb::install_package(&conn, "synthetic.knxprod", &package).unwrap();
    let profile = synthetic_profile(namespace, &package, &source);
    (dir, conn, synthetic_project(), profile)
}
#[test]
fn project_offline_preparation_uses_real_project_mapping_for_both_namespaces() {
    for namespace in [20, 23] {
        let (_dir, conn, project, profile) = synthetic_inputs(namespace);
        let before = project.clone();
        let changes = conn.total_changes();
        let result = knx_app::system2_validation_preparation::prepare_project_simulated_candidate(
            &conn,
            &project,
            knx_core::DeviceId(1),
            &profile,
        );
        assert!(
            result.is_ok(),
            "synthetic project preparation must succeed: {}",
            result.err().map(|e| e.to_string()).unwrap_or_default()
        );
        assert_eq!(project, before);
        assert_eq!(conn.total_changes(), changes);
    }
}

fn project_candidate(
    conn: &knx_productdb::Connection,
    project: &knx_core::Project,
    profile: &[u8],
) -> knx_app::system2_validation_preparation::PreparedCandidate {
    knx_app::system2_validation_preparation::prepare_project_simulated_candidate(
        conn,
        project,
        knx_core::DeviceId(1),
        profile,
    )
    .unwrap()
}
fn project_bundle(namespace: u8) -> PreparedBundle {
    let (_dir, conn, project, profile) = synthetic_inputs(namespace);
    project_candidate(&conn, &project, &profile).bundle
}
fn write_bytes(bundle: &PreparedBundle, address: u16) -> Vec<u8> {
    use knx_app::property_validation_bundle::OperationRecord;
    bundle
        .normalized_plan
        .steps
        .iter()
        .find_map(|step| match &step.operation {
            OperationRecord::WriteMemory {
                address: a, data, ..
            } if *a == address => Some(data.clone()),
            _ => None,
        })
        .unwrap()
}
fn set_parameter(project: &mut knx_core::Project, id: &str, raw: &str) {
    project.installations[0]
        .parameters
        .iter_mut()
        .find(|p| p.source.ets_id == syn_ref(id))
        .unwrap()
        .raw = raw.into();
}
fn exported(value: bool) -> knx_core::provenance::Override<bool> {
    knx_core::provenance::Override::Value(knx_core::provenance::Resolved {
        value,
        layer: knx_core::provenance::Layer::UserEdit,
    })
}
#[test]
fn project_preparation_has_independent_new_bytes_and_retained_complete_request() {
    for namespace in [20, 23] {
        let (_dir, conn, project, profile) = synthetic_inputs(namespace);
        let candidate = project_candidate(&conn, &project, &profile);
        assert_eq!(
            (
                candidate.source_positions,
                candidate.allocation_octets,
                candidate.written_octets,
                candidate.preserved_octets
            ),
            (21, 48, 31, 17)
        );
        let bundle = candidate.bundle;
        // Declared table reservation is 15 bytes in each 16-byte allocation.
        // Its last byte is independent source data, not unused table capacity.
        assert_eq!(write_bytes(&bundle, 293), [0]);
        assert_eq!(write_bytes(&bundle, 623), [0]);
        assert_eq!(write_bytes(&bundle, 15360), [1, 2]);
        assert_eq!(
            write_bytes(&bundle, 278),
            [3, 0x12, 0x34, 0x10, 0x35, 0x10, 0x36]
        );
        assert_eq!(write_bytes(&bundle, 608), [3, 0xfe, 0, 1, 1, 2, 1]);
        assert_eq!(
            write_bytes(&bundle, 1024),
            [2, 0xa7, 0x12, 0x21, 0, 0x34, 0xf7, 7]
        );
        assert_eq!(write_bytes(&bundle, 15363), [0; 5]);
        assert!(bundle
            .normalized_plan
            .steps
            .iter()
            .any(|s| s.source_index == 16
                && matches!(
                    s.operation,
                    knx_app::property_validation_bundle::OperationRecord::ToolMetadata {
                        table_address: 1024
                    }
                )));
        let retained = bundle.project_preparation.as_ref().unwrap();
        assert_eq!(retained.decoded_project().unwrap(), project);
        assert_eq!(retained.request.values[&syn_ref("R")], "258");
        assert_eq!(retained.request.values[&syn_ref("RS")], "1");
        assert_eq!(
            retained
                .request
                .links
                .iter()
                .map(|l| (l.object, l.group_address, l.sending))
                .collect::<Vec<_>>(),
            [(1, 0x1035, true), (1, 0x1036, false)]
        );
        let flags = &retained.request.flag_overrides[&syn_ref("OR1")];
        assert_eq!(
            (
                flags.read,
                flags.write,
                flags.update,
                flags.communication,
                flags.transmit,
                flags.read_on_init
            ),
            (Some(false), Some(true), Some(true), None, None, None)
        );
        assert_eq!(retained.coverage.len(), 2);
        assert_eq!(retained.profile_bytes, profile);
        assert!(bundle.rebind_hardware([1; 6]).is_err());
        assert!(ValidationProfile::from_json(&profile).is_err());
        assert!(
            knx_app::system2_validation_preparation::ProjectValidationProfile::from_json(PROFILE)
                .is_err()
        );
    }
}
#[test]
fn project_artifact_survives_input_drop_and_replays_through_fresh_resolver() {
    for namespace in [20, 23] {
        let (dir, conn, project, profile) = synthetic_inputs(namespace);
        let saved = knx_store::open_and_migrate_in_memory().unwrap();
        knx_store::save_project(&saved, &project).unwrap();
        let reloaded = knx_store::load_project(&saved).unwrap();
        assert_eq!(reloaded, project);
        let first = project_candidate(&conn, &project, &profile).bundle;
        let second = project_candidate(&conn, &reloaded, &profile).bundle;
        assert_eq!(first.binding, second.binding);
        let mut bundle_path = dir.path().join("prepared.json");
        std::fs::write(&bundle_path, serde_json::to_vec(&first).unwrap()).unwrap();
        if let Some(output) = std::env::var_os("KNXBENCH_SYNTHETIC_ARTIFACT_DIR") {
            let output = std::path::PathBuf::from(output);
            std::fs::create_dir_all(&output).unwrap();
            let retained = output.join(format!("project-system2-namespace{namespace}.json"));
            std::fs::copy(&bundle_path, &retained).unwrap();
            assert_eq!(
                std::fs::read(&bundle_path).unwrap(),
                std::fs::read(&retained).unwrap()
            );
            bundle_path = retained;
        }
        drop(conn);
        drop(project);
        drop(reloaded);
        drop(profile);
        drop(saved);
        drop(first);
        let read = app::read_bundle(&bundle_path).unwrap();
        let replay =
            knx_app::system2_validation_preparation::replay_project_simulated_candidate(&read)
                .unwrap();
        assert_eq!(replay.bundle.binding, second.binding);
        assert_eq!(replay.bundle.normalized_plan, second.normalized_plan);
        assert_eq!(
            replay.bundle.project_preparation.as_ref().unwrap().request,
            read.project_preparation.as_ref().unwrap().request
        );
    }
}
#[test]
fn project_parameters_links_and_flags_change_only_the_desired_preparation() {
    use knx_core::{
        flags::Direction,
        provenance::{Layer, Override, Resolved},
        ComObjectInstanceId,
    };
    let (_dir, conn, project, profile) = synthetic_inputs(20);
    let base = project_candidate(&conn, &project, &profile).bundle;
    let mut changed = project.clone();
    set_parameter(&mut changed, "R", "772");
    let params = project_candidate(&conn, &changed, &profile).bundle;
    assert_eq!(write_bytes(&params, 15360), [3, 4]);
    assert_eq!(write_bytes(&params, 1024), write_bytes(&base, 1024));
    let mut changed = project.clone();
    changed
        .devices
        .com_object_mut(ComObjectInstanceId(1))
        .unwrap()
        .links[0]
        .direction = Direction::Receive;
    let links = project_candidate(&conn, &changed, &profile).bundle;
    assert_ne!(write_bytes(&links, 608), write_bytes(&base, 608));
    let mut changed = project.clone();
    changed.installations[0].group_addresses[1].address = knx_core::GroupAddress::from_raw(0x1037);
    assert_ne!(
        write_bytes(&project_candidate(&conn, &changed, &profile).bundle, 278),
        write_bytes(&base, 278)
    );
    let mut changed = project.clone();
    changed
        .devices
        .com_object_mut(ComObjectInstanceId(1))
        .unwrap()
        .flags
        .write = exported(false);
    assert_ne!(
        write_bytes(&project_candidate(&conn, &changed, &profile).bundle, 1024),
        write_bytes(&base, 1024)
    );
    let mut changed = project.clone();
    let flags = &mut changed
        .devices
        .com_object_mut(ComObjectInstanceId(1))
        .unwrap()
        .flags;
    flags.communication = Override::Value(Resolved {
        value: true,
        layer: Layer::Program,
    });
    flags.transmit = Override::Value(Resolved {
        value: true,
        layer: Layer::ProgramRef,
    });
    let inherited = project_candidate(&conn, &changed, &profile).bundle;
    assert_eq!(inherited.normalized_plan, base.normalized_plan);
    let f = &inherited
        .project_preparation
        .as_ref()
        .unwrap()
        .request
        .flag_overrides[&syn_ref("OR1")];
    assert_eq!((f.communication, f.transmit), (None, None));
}
#[test]
fn project_preparation_distinguishes_explicit_defaults_project_edits_and_device_ids() {
    let (_dir, conn, mut explicit, profile) = synthetic_inputs(20);
    set_parameter(&mut explicit, "R", "4660");
    let first = project_candidate(&conn, &explicit, &profile).bundle;
    let mut absent = explicit.clone();
    absent.installations[0]
        .parameters
        .retain(|p| p.source.ets_id != syn_ref("R"));
    let second = project_candidate(&conn, &absent, &profile).bundle;
    assert_eq!(first.normalized_plan, second.normalized_plan);
    assert_ne!(
        first.binding.project_preparation_sha256,
        second.binding.project_preparation_sha256
    );
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("operation");
    drop(OperationStore::create(&path, first.binding.clone()).unwrap());
    assert!(OperationStore::open(&path, &second.binding).is_err());
    let mut renamed = explicit.clone();
    renamed.info.name = "A project-only edit with reused ID".into();
    let third = project_candidate(&conn, &renamed, &profile).bundle;
    assert_eq!(first.normalized_plan, third.normalized_plan);
    assert_ne!(first.binding, third.binding);
    let mut another = explicit.clone();
    let mut instance = another.devices.remove(knx_core::DeviceId(1)).unwrap();
    instance.id = another.ids.next_device_id().unwrap();
    let id = instance.id;
    for object in instance.com_objects.clone() {
        another.devices.com_object_mut(object).unwrap().device = id;
    }
    for parameter in &mut another.installations[0].parameters {
        parameter.device = id;
    }
    another.installations[0].topology.unassigned = vec![id];
    another.devices.insert(instance);
    let fourth = knx_app::system2_validation_preparation::prepare_project_simulated_candidate(
        &conn, &another, id, &profile,
    )
    .unwrap()
    .bundle;
    assert_eq!(first.normalized_plan, fourth.normalized_plan);
    assert_ne!(first.binding, fourth.binding);
}
#[test]
fn project_configuration_refuses_inactive_aliases_duplicates_and_unreadable_flags() {
    use knx_core::{provenance::Override, ComObjectInstanceId};
    let (_dir, conn, project, profile) = synthetic_inputs(20);
    let check = |project: &knx_core::Project| {
        assert!(
            knx_app::system2_validation_preparation::prepare_project_simulated_candidate(
                &conn,
                project,
                knx_core::DeviceId(1),
                &profile
            )
            .is_err()
        )
    };
    let mut changed = project.clone();
    changed
        .devices
        .com_object_mut(ComObjectInstanceId(2))
        .unwrap()
        .flags
        .write = exported(true);
    check(&changed);
    let mut changed = project.clone();
    let links = changed
        .devices
        .com_object(ComObjectInstanceId(1))
        .unwrap()
        .links
        .clone();
    changed
        .devices
        .com_object_mut(ComObjectInstanceId(2))
        .unwrap()
        .links = links;
    check(&changed);
    let mut changed = project.clone();
    set_parameter(&mut changed, "RS", "0");
    check(&changed);
    for bad in [
        Override::Empty,
        Override::Malformed("not-a-boolean".into()),
        exported(true),
    ] {
        let mut changed = project.clone();
        let flags = &mut changed
            .devices
            .com_object_mut(ComObjectInstanceId(1))
            .unwrap()
            .flags;
        if bad == exported(true) {
            flags.read_on_init = bad;
        } else {
            flags.read = bad;
        }
        check(&changed);
    }
    let mut changed = project.clone();
    changed
        .devices
        .com_object_mut(ComObjectInstanceId(1))
        .unwrap()
        .source
        .ets_id = syn_ref("UNKNOWN");
    check(&changed);
    for reverse in [false, true] {
        let mut changed = project.clone();
        let mut duplicate = changed
            .devices
            .com_object(ComObjectInstanceId(1))
            .unwrap()
            .clone();
        duplicate.id = changed.ids.next_com_object_instance_id().unwrap();
        duplicate.links.clear();
        duplicate.flags.write = exported(false);
        changed
            .devices
            .get_mut(knx_core::DeviceId(1))
            .unwrap()
            .com_objects
            .push(duplicate.id);
        changed.devices.insert_com_object(duplicate);
        if reverse {
            changed
                .devices
                .get_mut(knx_core::DeviceId(1))
                .unwrap()
                .com_objects
                .reverse();
        }
        let error =
            match knx_app::system2_validation_preparation::prepare_project_simulated_candidate(
                &conn,
                &changed,
                knx_core::DeviceId(1),
                &profile,
            ) {
                Err(e) => e,
                Ok(_) => panic!("duplicate override must refuse before extraction"),
            };
        assert!(error
            .to_string()
            .contains("duplicate normalized flag-override"));
    }
    let mut changed = project.clone();
    changed
        .devices
        .get_mut(knx_core::DeviceId(1))
        .unwrap()
        .address = None;
    check(&changed);
    let mut changed = project.clone();
    changed
        .devices
        .get_mut(knx_core::DeviceId(1))
        .unwrap()
        .program_ref = "unknown".into();
    check(&changed);
    let mut changed = project.clone();
    changed.ids = Default::default();
    check(&changed);
}
#[test]
fn compact_project_retention_serializes_canonical_strings_and_preserves_decoded_identity() {
    use base64::{engine::general_purpose::STANDARD, Engine};
    let bundle = project_bundle(20);
    let record = bundle.project_preparation.as_ref().unwrap();
    let identity = record.sha256().unwrap();
    let plan_identity = bundle.normalized_plan.sha256().unwrap();
    let value = serde_json::to_value(&bundle).unwrap();
    let knx_app::system2_validation_preparation::ProjectProductInput::StandalonePackage {
        source_bytes,
        package_bytes,
        ..
    } = &record.product_input
    else {
        panic!("package fixture variant");
    };
    for (pointer, raw) in [
        (
            "/project_preparation/project_snapshot",
            &record.project_snapshot,
        ),
        ("/project_preparation/profile_bytes", &record.profile_bytes),
        (
            "/project_preparation/product_input/source_bytes",
            source_bytes,
        ),
        (
            "/project_preparation/product_input/package_bytes",
            package_bytes,
        ),
    ] {
        assert!(
            value.pointer(pointer).unwrap().as_str() == Some(STANDARD.encode(raw).as_str()),
            "canonical retained field {pointer}"
        );
    }
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("bundle.json");
    std::fs::write(&file, serde_json::to_vec_pretty(&bundle).unwrap()).unwrap();
    let reopened = app::read_bundle(&file).unwrap();
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
    assert_eq!(reopened.normalized_plan.sha256().unwrap(), plan_identity);
}

#[test]
fn compact_project_retention_refuses_numeric_array_fallback_in_every_scalar_field() {
    let baseline = serde_json::to_value(project_bundle(20)).unwrap();
    for pointer in [
        "/project_preparation/project_snapshot",
        "/project_preparation/profile_bytes",
        "/project_preparation/product_input/source_bytes",
        "/project_preparation/product_input/package_bytes",
    ] {
        for bad in [
            serde_json::json!([]),
            serde_json::json!([0, 1]),
            serde_json::Value::Null,
            serde_json::json!(1),
            serde_json::json!({"base64":""}),
        ] {
            let mut value = baseline.clone();
            *value.pointer_mut(pointer).unwrap() = bad;
            assert!(
                serde_json::from_value::<PreparedBundle>(value).is_err(),
                "non-string retained field accepted: {pointer}"
            );
        }
    }
}

#[test]
fn compact_project_retention_accepts_only_canonical_padded_standard_strings() {
    let baseline = serde_json::to_value(project_bundle(20)).unwrap();
    for pointer in [
        "/project_preparation/project_snapshot",
        "/project_preparation/profile_bytes",
        "/project_preparation/product_input/source_bytes",
        "/project_preparation/product_input/package_bytes",
    ] {
        let mut valid = baseline.clone();
        *valid.pointer_mut(pointer).unwrap() = serde_json::json!("AAH+/w==");
        assert!(
            serde_json::from_value::<PreparedBundle>(valid).is_ok(),
            "canonical string must decode before semantic validation: {pointer}"
        );
        for bad in [
            "A", "AA", "AAA", "A===", "====", "AA===", "AA=", "AA=A", "=AAA", "AA==AA==", "AA==\n",
            " AAA", "AA\r=", "AA\t=", "-w==", "_w==", "!!!!", "éAA", "AB==", "AAB=",
        ] {
            let mut value = baseline.clone();
            *value.pointer_mut(pointer).unwrap() = serde_json::json!(bad);
            assert!(
                serde_json::from_value::<PreparedBundle>(value).is_err(),
                "noncanonical retained field accepted: {pointer} {bad:?}"
            );
        }
    }
}

#[test]
fn project_bundle_read_refuses_drift_unknown_fields_and_format_mixing() {
    let bundle = project_bundle(20);
    let baseline = serde_json::to_value(&bundle).unwrap();
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("bundle.json");
    let mut variants = Vec::new();
    for pointer in [
        "/binding/project_preparation_sha256",
        "/binding/profile_sha256",
        "/binding/source_sha256",
        "/binding/plan_sha256",
        "/project_preparation/snapshot_sha256",
        "/project_preparation/project_semantic_sha256",
        "/project_preparation/source_sha256",
        "/project_preparation/product_input/package_sha256",
        "/project_preparation/profile_sha256",
        "/project_preparation/plan_sha256",
    ] {
        let mut value = baseline.clone();
        *value.pointer_mut(pointer).unwrap() = serde_json::Value::String("a".repeat(64));
        variants.push(value);
    }
    for (pointer, value) in [
        ("/format", serde_json::json!(1)),
        ("/project_preparation", serde_json::Value::Null),
        (
            "/binding/project_preparation_sha256",
            serde_json::Value::Null,
        ),
        ("/binding/simulated", serde_json::json!(false)),
        ("/project_preparation/device_id", serde_json::json!(2)),
        (
            "/project_preparation/schema_namespace",
            serde_json::json!("http://knx.org/xml/project/23"),
        ),
        (
            "/project_preparation/configuration_policy",
            serde_json::json!("guess"),
        ),
        (
            "/project_preparation/request/individual_address",
            serde_json::json!(0x1235),
        ),
        (
            "/normalized_plan/steps/0/source_index",
            serde_json::json!(1),
        ),
    ] {
        let mut v = baseline.clone();
        *v.pointer_mut(pointer).unwrap() = value;
        variants.push(v);
    }
    // Preserve digest drift as valid canonical encoding with one decoded-byte flip.
    for pointer in [
        "/project_preparation/project_snapshot",
        "/project_preparation/product_input/source_bytes",
        "/project_preparation/product_input/package_bytes",
        "/project_preparation/profile_bytes",
    ] {
        use base64::{engine::general_purpose::STANDARD, Engine};
        let mut value = baseline.clone();
        let mut bytes = STANDARD
            .decode(value.pointer(pointer).unwrap().as_str().unwrap())
            .unwrap();
        let original_len = bytes.len();
        bytes[0] ^= 1;
        assert_eq!(bytes.len(), original_len);
        *value.pointer_mut(pointer).unwrap() = STANDARD.encode(bytes).into();
        assert!(serde_json::from_value::<PreparedBundle>(value.clone()).is_ok());
        variants.push(value);
    }
    for pointer in [
        "",
        "/binding",
        "/project_preparation",
        "/project_preparation/request",
        "/project_preparation/request/flag_overrides/M-0000_A-0001-01-0000_OR1",
        "/project_preparation/coverage/0",
    ] {
        let mut value = baseline.clone();
        value
            .pointer_mut(pointer)
            .unwrap()
            .as_object_mut()
            .unwrap()
            .insert("unknown".into(), true.into());
        variants.push(value);
    }
    for value in variants {
        std::fs::write(&file, serde_json::to_vec(&value).unwrap()).unwrap();
        assert!(app::read_bundle(&file).is_err());
    }
    let mut over = bundle.clone();
    if let knx_app::system2_validation_preparation::ProjectProductInput::StandalonePackage {
        package_bytes,
        ..
    } = &mut over.project_preparation.as_mut().unwrap().product_input
    {
        *package_bytes = vec![255; 23 * 512 * 1024];
    } else {
        panic!("package fixture variant");
    }
    assert!(
        serde_json::to_vec(&over).unwrap().len() < 16 * 1024 * 1024,
        "compact evidence fits but nested pretty recovery must also fit"
    );
    let error = match over.plan() {
        Err(e) => e,
        Ok(_) => panic!("retained evidence must fit recovery envelope"),
    };
    assert_eq!(
        error.to_string(),
        "project bundle exceeds recovery envelope budget"
    );
    std::fs::write(&file, vec![b' '; 16 * 1024 * 1024 + 1]).unwrap();
    assert!(app::read_bundle(&file).is_err());
    let (_dir, conn, project, profile) = synthetic_inputs(20);
    let base: serde_json::Value = serde_json::from_slice(&profile).unwrap();
    for (key, value) in [
        (
            "schema_namespace",
            serde_json::json!("http://knx.org/xml/project/23"),
        ),
        (
            "schema_namespace",
            serde_json::json!("http://knx.org/xml/project/21"),
        ),
        ("program_id", serde_json::json!("M-0000_A-0002-01-0000")),
        ("source_sha256", serde_json::json!("a".repeat(64))),
        ("package_sha256", serde_json::json!("a".repeat(64))),
        ("hardware", serde_json::json!([0, 0, 0, 0, 0, 1])),
        ("application", serde_json::json!([0, 0, 0, 2, 1])),
        (
            "configuration",
            serde_json::json!("complete-defaults-no-links-v1"),
        ),
    ] {
        let mut changed = base.clone();
        changed[key] = value;
        assert!(
            knx_app::system2_validation_preparation::prepare_project_simulated_candidate(
                &conn,
                &project,
                knx_core::DeviceId(1),
                &serde_json::to_vec(&changed).unwrap()
            )
            .is_err()
        );
    }
}
fn synthetic_old_device(bundle: &PreparedBundle, mut config: SimulatorConfig) -> SimulatedDevice {
    // Independent OLD, not produced by NEW execution; no real hardware history implied.
    config.mask_version = 0x0025;
    config.serial_number = Some(bundle.binding.serial);
    let device = SimulatedDevice::with_config_at(IndividualAddress::from_raw(0x1234), config);
    device.preset_property(
        0,
        knx_core::commissioning::properties::PID_MANUFACTURER_ID,
        &[0, 0],
    );
    device.preset_property(0, PID_HARDWARE_TYPE, &[240, 0, 0, 0, 0, 1]);
    device.preset_property(3, PID_PROGRAM_VERSION, &[0, 0, 0, 1, 1]);
    for (owner, extents, pointer, task_control) in [
        (
            SimulatedSystem2Object::AddressTable,
            vec![(278, 16)],
            278,
            false,
        ),
        (
            SimulatedSystem2Object::AssociationTable,
            vec![(608, 16)],
            608,
            false,
        ),
        (
            SimulatedSystem2Object::Application,
            vec![(1024, 8), (15360, 8)],
            0,
            true,
        ),
    ] {
        let allocations = extents
            .into_iter()
            .map(|(start, length)| {
                device.preset_memory(u32::from(start), &vec![0xa5; usize::from(length)]);
                SimulatedAbsoluteAllocation {
                    subtype: 0,
                    start,
                    length,
                    access: 0x33,
                    memory_type: 3,
                    attributes: 0,
                }
            })
            .collect();
        device.preset_system2_object(
            owner,
            SimulatedSystem2ObjectState {
                load_state: LoadState::Loaded,
                table_reference: pointer,
                allocations,
                task_segment: Some(SimulatedTaskSegment {
                    start: pointer as u16,
                    pei_type: 0,
                    manufacturer: 0,
                    application: 1,
                    version: 1,
                }),
                task_control_2: task_control.then_some(SimulatedTaskControl2 {
                    callback: 0,
                    communication_objects: 1024,
                    segment_1: 0,
                    segment_2: 0,
                }),
            },
        );
    }
    device
}
#[tokio::test]
async fn project_candidate_executes_and_disk_reopened_restore_returns_full_independent_old_state() {
    for namespace in [20, 23] {
        let bundle = project_bundle(namespace);
        let device = synthetic_old_device(&bundle, SimulatorConfig::default());
        let old = device.system2_snapshot();
        let dir = tempfile::tempdir().unwrap();
        let root = std::env::var_os("KNXBENCH_SYNTHETIC_ARTIFACT_DIR")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| dir.path().to_path_buf());
        std::fs::create_dir_all(&root).unwrap();
        let file = root.join(format!("execution-bundle-namespace{namespace}.json"));
        std::fs::write(&file, serde_json::to_vec(&bundle).unwrap()).unwrap();
        let path = root.join(format!("operation-namespace{namespace}"));
        let mut store = OperationStore::create(&path, bundle.binding.clone()).unwrap();
        app::execute_simulated(
            &device,
            &bundle,
            &mut store,
            integrated_timing(),
            &AtomicBool::new(false),
        )
        .await
        .unwrap();
        assert_eq!(store.outcome(), LocalOutcome::Finished);
        assert_ne!(device.system2_snapshot().memory, old.memory);
        assert_eq!(device.system2_snapshot().objects, old.objects);
        assert_eq!(device.memory(15360, 3), [Some(1), Some(2), Some(0xa5)]);
        assert_eq!(device.memory(285, 8), vec![Some(0xa5); 8]);
        assert_eq!(device.memory(293, 1), [Some(0)]);
        assert_eq!(device.memory(615, 8), vec![Some(0xa5); 8]);
        assert_eq!(device.memory(623, 1), [Some(0)]);
        let backup = store.recovery().unwrap().clone();
        assert_eq!(
            backup
                .allocations
                .iter()
                .map(|a| a.data.len())
                .sum::<usize>(),
            48
        );
        assert!(backup
            .allocations
            .iter()
            .all(|a| a.data.iter().all(|b| *b == 0xa5)));
        drop(store);
        drop(bundle);
        let bundle = app::read_bundle(&file).unwrap();
        let mut fresh = OperationStore::open(&path, &bundle.binding).unwrap();
        let frames = device.seen().len();
        assert!(app::restore_simulated(
            &device,
            &bundle,
            &mut fresh,
            integrated_timing(),
            &AtomicBool::new(false),
            "wrong"
        )
        .await
        .is_err());
        assert_eq!(device.seen().len(), frames);
        app::restore_simulated(
            &device,
            &bundle,
            &mut fresh,
            integrated_timing(),
            &AtomicBool::new(false),
            &bundle.restore_phrase(),
        )
        .await
        .unwrap();
        assert_eq!(fresh.outcome(), LocalOutcome::Restored);
        assert_eq!(device.system2_snapshot(), old);
        assert_eq!(fresh.recovery().unwrap(), &backup);
        assert!(app::restore_simulated(
            &device,
            &bundle,
            &mut fresh,
            integrated_timing(),
            &AtomicBool::new(false),
            &bundle.restore_phrase()
        )
        .await
        .is_err());
        drop(fresh);
        assert_eq!(
            OperationStore::open(&path, &bundle.binding)
                .unwrap()
                .outcome(),
            LocalOutcome::Restored
        );
    }
}
#[tokio::test]
async fn project_partial_memory_failure_reopens_and_restores_without_overwriting_original_backup() {
    let bundle = project_bundle(20);
    let device = synthetic_old_device(
        &bundle,
        SimulatorConfig {
            interrupt_at: Some(Interruption::MemoryWriteAt(15360)),
            ..Default::default()
        },
    );
    let old = device.system2_snapshot();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("operation");
    let mut store = OperationStore::create(&path, bundle.binding.clone()).unwrap();
    assert!(app::execute_simulated(
        &device,
        &bundle,
        &mut store,
        integrated_timing(),
        &AtomicBool::new(false)
    )
    .await
    .is_err());
    assert_eq!(store.outcome(), LocalOutcome::Failed);
    assert!(store.mutation_possible());
    assert_ne!(device.system2_snapshot(), old);
    let backup = store.recovery().unwrap().clone();
    drop(store);
    let mut fresh = OperationStore::open(&path, &bundle.binding).unwrap();
    app::restore_simulated(
        &device,
        &bundle,
        &mut fresh,
        integrated_timing(),
        &AtomicBool::new(false),
        &bundle.restore_phrase(),
    )
    .await
    .unwrap();
    assert_eq!(device.system2_snapshot(), old);
    assert_eq!(fresh.recovery().unwrap(), &backup);
}
#[tokio::test]
async fn project_precancel_and_backup_persistence_refusal_precede_even_setup_mutations() {
    use knx_net::commissioning::simulator::Seen;
    let bundle = project_bundle(20);
    for checkpoint in [None, Some(3usize), Some(4usize)] {
        let device = synthetic_old_device(&bundle, SimulatorConfig::default());
        let old = device.system2_snapshot();
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("operation");
        let mut store = OperationStore::create(&path, bundle.binding.clone()).unwrap();
        if let Some(sequence) = checkpoint {
            std::fs::write(
                path.join(format!("checkpoint-{sequence:08}.json")),
                b"collision",
            )
            .unwrap();
        }
        assert!(app::execute_simulated(
            &device,
            &bundle,
            &mut store,
            integrated_timing(),
            &AtomicBool::new(checkpoint.is_none())
        )
        .await
        .is_err());
        assert_eq!(device.system2_snapshot(), old);
        assert!(!device
            .seen()
            .iter()
            .any(|s| matches!(s, Seen::MemoryWrite { .. } | Seen::PropertyWrite { .. })));
        if checkpoint.is_none() {
            assert!(device.seen().is_empty());
            assert_eq!(store.outcome(), LocalOutcome::Cancelled);
            assert!(store.recovery().is_none());
            assert!(!store.mutation_possible());
        }
    }
}
#[tokio::test]
async fn project_unloaded_device_downloads_without_original_recovery_and_final_identity_stays_hard()
{
    let bundle = project_bundle(20);
    let device = SimulatedDevice::with_config_at(
        IndividualAddress::from_raw(0x1234),
        SimulatorConfig {
            mask_version: 0x0025,
            serial_number: Some(bundle.binding.serial),
            ..Default::default()
        },
    );
    device.preset_property(
        0,
        knx_core::commissioning::properties::PID_MANUFACTURER_ID,
        &[0, 0],
    );
    device.preset_property(0, PID_HARDWARE_TYPE, &[240, 0, 0, 0, 0, 1]);
    let dir = tempfile::tempdir().unwrap();
    let mut store =
        OperationStore::create(&dir.path().join("operation"), bundle.binding.clone()).unwrap();
    // Owner rulings 2026-10-25: an unloaded device of the right type is
    // downloadable; its prestate is evidence only, never a recovery source.
    let error = app::execute_simulated(
        &device,
        &bundle,
        &mut store,
        integrated_timing(),
        &AtomicBool::new(false),
    )
    .await
    .unwrap_err();
    // This simulator derives no PID 13 from the 03 02 identity record, so the
    // HARD final program-version verification fails: it is not softened.
    assert!(error.to_string().contains("property_id: 13"), "{error}");
    let progress = store.progress();
    assert_eq!(progress.outcome, LocalOutcome::Failed);
    assert!(progress.mutation_possible);
    assert!(progress.backup_available);
    assert!(progress.prestate_differs);
    assert!(!progress.restore_available);
    assert!(progress.recovery_phrases.is_none());
    assert_eq!(
        progress.program_version_observation,
        Some((bundle.binding.application, Vec::new()))
    );
    assert!(progress.pointer_observations.iter().all(
        |o| o.record.state == knx_core::commissioning::load_state::LoadState::Unloaded.octet()
    ));
}
#[tokio::test]
async fn project_destructive_unload_and_cancelled_restore_are_durable_restore_failed() {
    for destructive in [true, false] {
        let bundle = project_bundle(20);
        let device = synthetic_old_device(
            &bundle,
            SimulatorConfig {
                experimental_destroy_allocation_memory_on_unload: destructive,
                ..Default::default()
            },
        );
        let old = device.system2_snapshot();
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("operation");
        let mut store = OperationStore::create(&path, bundle.binding.clone()).unwrap();
        let result = app::execute_simulated(
            &device,
            &bundle,
            &mut store,
            integrated_timing(),
            &AtomicBool::new(false),
        )
        .await;
        assert_eq!(result.is_err(), destructive);
        assert_eq!(
            store.outcome(),
            if destructive {
                LocalOutcome::Failed
            } else {
                LocalOutcome::Finished
            }
        );
        let backup = store.recovery().unwrap().clone();
        drop(store);
        let mut fresh = OperationStore::open(&path, &bundle.binding).unwrap();
        assert!(app::restore_simulated(
            &device,
            &bundle,
            &mut fresh,
            integrated_timing(),
            &AtomicBool::new(!destructive),
            &bundle.restore_phrase()
        )
        .await
        .is_err());
        assert_eq!(fresh.outcome(), LocalOutcome::RestoreFailed);
        assert_eq!(fresh.recovery().unwrap(), &backup);
        assert_ne!(device.system2_snapshot(), old);
        drop(fresh);
        let mut fresh = OperationStore::open(&path, &bundle.binding).unwrap();
        assert_eq!(fresh.outcome(), LocalOutcome::RestoreFailed);
        assert!(app::restore_simulated(
            &device,
            &bundle,
            &mut fresh,
            integrated_timing(),
            &AtomicBool::new(false),
            &bundle.restore_phrase()
        )
        .await
        .is_err());
        assert_eq!(fresh.recovery().unwrap(), &backup);
    }
}

#[tokio::test]
async fn project_cancellation_and_crash_checkpoint_boundaries_restore_from_disk() {
    use knx_app::property_validation::Checkpoint;
    use knx_app::property_validation_bundle::RecoveryRecord;
    use knx_core::commissioning::{
        authorisation::AuthorisationPlan,
        mutation::{WriteAuthorisation, WriteScope},
        property_download::PropertyOperation,
    };
    use knx_net::commissioning::{
        property_download::run_experimental_property_plan, property_recovery::PropertyJournalEvent,
    };
    use std::sync::atomic::Ordering;
    let bundle = project_bundle(20);
    for (cut_source, cancelled, first_run_only) in [
        (2, false, false),
        (6, false, false),
        (12, false, false),
        (13, true, false),
        (13, false, true),
    ] {
        let device = synthetic_old_device(&bundle, SimulatorConfig::default());
        let old = device.system2_snapshot();
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("operation");
        let mut store = OperationStore::create(&path, bundle.binding.clone()).unwrap();
        let cancel = AtomicBool::new(false);
        let plan = bundle.plan().unwrap();
        let result = run_experimental_property_plan(&device,&plan,
            WriteAuthorisation::for_simulator(plan.target(),WriteScope::Download).unwrap(),
            AuthorisationPlan::Skip,integrated_timing(),&bundle.preconditions().unwrap(),
            &mut |event| {
                match event {
                    PropertyJournalEvent::RecoveryAcquired(snapshot) => store.checkpoint(Checkpoint::RecoveryAcquired {recovery: RecoveryRecord::from_snapshot(snapshot,&bundle).map_err(|e|e.to_string())?}),
                    PropertyJournalEvent::OperationStarting {operation_index,mutation_may_have_started,..} => store.checkpoint(Checkpoint::StepStarted {index:operation_index,mutation:mutation_may_have_started || matches!(plan.steps()[operation_index].operation,PropertyOperation::LoadControl{..}|PropertyOperation::WriteMemory{..}|PropertyOperation::Disconnect)}),
                    PropertyJournalEvent::OperationCompleted {operation_index,source_index,..} => {
                        store.checkpoint(Checkpoint::StepFinished {index:operation_index}).map_err(|e|e.to_string())?;
                        if source_index == cut_source && (!first_run_only || matches!(plan.steps()[operation_index].operation, PropertyOperation::WriteMemory {address:15360,..})) {
                            if cancelled { cancel.store(true,Ordering::SeqCst); }
                            else {return Err("deterministic crash-boundary journal cut; caller terminal checkpoint intentionally absent".into());}
                        }
                        Ok(())
                    }
                    PropertyJournalEvent::FinalPointerObservations(_) => Err(std::io::Error::other("unexpected final pointer observation")),
                    PropertyJournalEvent::InProcedurePointerObservation(_) => Err(std::io::Error::other("unexpected in-procedure pointer observation")),
                }.map_err(|e|e.to_string())
            },&||cancel.load(Ordering::SeqCst)).await;
        assert!(result.is_err(), "source {cut_source} must stop");
        if cancelled {
            store.checkpoint(Checkpoint::Cancelled).unwrap();
        }
        drop(store);
        let mut reopened = OperationStore::open(&path, &bundle.binding).unwrap();
        assert_eq!(
            reopened.outcome(),
            if cancelled {
                LocalOutcome::Cancelled
            } else {
                LocalOutcome::Interrupted
            }
        );
        assert!(reopened.mutation_possible());
        app::restore_simulated(
            &device,
            &bundle,
            &mut reopened,
            integrated_timing(),
            &AtomicBool::new(false),
            &bundle.restore_phrase(),
        )
        .await
        .unwrap();
        assert_eq!(
            device.system2_snapshot(),
            old,
            "source {cut_source} old persistent state"
        );
        assert_eq!(reopened.outcome(), LocalOutcome::Restored);
    }
}

#[test]
fn project_fresh_catalogue_instance_needs_no_saved_project_or_previous_hardware_binding() {
    use knx_core::{Command, DeviceId};
    for namespace in [20, 23] {
        let (_dir, conn, template, profile) = synthetic_inputs(namespace);
        let item = knx_productdb::query::catalog_item(&conn, "M-0000_CI-1")
            .unwrap()
            .unwrap();
        let (product_ref, program_ref, program) =
            match knx_productdb::query::resolve_catalog_item_program(&conn, &item).unwrap() {
                knx_productdb::query::CatalogItemProgram::Program {
                    product_ref_id,
                    hardware2program_ref_id,
                    program_id,
                } => (product_ref_id, hardware2program_ref_id, program_id),
                _ => panic!("synthetic catalogue entry must resolve a real application"),
            };
        let mut fresh = template.clone();
        fresh.devices = knx_core::devices::Devices::new();
        fresh.installations[0].topology.unassigned.clear();
        let mut instance = template.devices.get(DeviceId(1)).unwrap().clone();
        instance.product_ref = product_ref;
        instance.program_ref = program_ref;
        instance.address = None;
        let mut objects = Vec::new();
        for (index, id) in knx_productdb::query::com_object_ref_ids(&conn, &program)
            .unwrap()
            .into_iter()
            .enumerate()
        {
            let view = knx_productdb::query::com_object_view(&conn, &program, &id, None)
                .unwrap()
                .unwrap();
            let mut object = template
                .devices
                .com_object(knx_core::ComObjectInstanceId(index as u32 + 1))
                .unwrap()
                .clone();
            object.source = syn_source(&id);
            object.number = view.number.unwrap() as u16;
            object.links.clear();
            object.flags = knx_core::ResolvedFlags::none();
            objects.push(object);
        }
        Command::CreateDevice {
            device: instance,
            com_objects: objects,
            program_defaults: vec![],
            installation: Some(knx_core::InstallationId(0)),
            line: None,
        }
        .apply(&mut fresh)
        .unwrap();
        // Address assignment is deliberate; preparation never discovers or guesses one.
        Command::SetIndividualAddress {
            device: DeviceId(1),
            address: Some(IndividualAddress::from_raw(0x1234)),
        }
        .apply(&mut fresh)
        .unwrap();
        for id in [
            knx_core::ComObjectInstanceId(1),
            knx_core::ComObjectInstanceId(2),
        ] {
            let desired = template.devices.com_object(id).unwrap();
            let object = fresh.devices.com_object_mut(id).unwrap();
            object.flags = desired.flags.clone();
            object.links = desired.links.clone();
        }
        assert_eq!(
            fresh.devices.get(DeviceId(1)).unwrap().commissioning,
            Default::default()
        );
        let before = fresh.clone();
        let prepared = project_candidate(&conn, &fresh, &profile).bundle;
        assert_eq!(fresh, before);
        assert_eq!(
            prepared.normalized_plan,
            project_candidate(&conn, &template, &profile)
                .bundle
                .normalized_plan
        );
        let mut reused = fresh.clone();
        reused.devices.get_mut(DeviceId(1)).unwrap().description =
            Some("Desired replacement configuration; previous physical state unknown".into());
        let replacement = project_candidate(&conn, &reused, &profile).bundle;
        assert_eq!(replacement.normalized_plan, prepared.normalized_plan);
        assert_ne!(
            replacement.binding.project_preparation_sha256,
            prepared.binding.project_preparation_sha256
        );
    }
}
#[tokio::test]
async fn project_restore_refuses_missing_or_changed_observed_application_and_keeps_original_backup()
{
    for version in [vec![], vec![0, 0, 0, 2, 1]] {
        let bundle = project_bundle(20);
        let device = synthetic_old_device(&bundle, SimulatorConfig::default());
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("operation");
        let mut store = OperationStore::create(&path, bundle.binding.clone()).unwrap();
        app::execute_simulated(
            &device,
            &bundle,
            &mut store,
            integrated_timing(),
            &AtomicBool::new(false),
        )
        .await
        .unwrap();
        let backup = store.recovery().unwrap().clone();
        drop(store);
        device.preset_property(3, PID_PROGRAM_VERSION, &version);
        let mut fresh = OperationStore::open(&path, &bundle.binding).unwrap();
        assert!(app::restore_simulated(
            &device,
            &bundle,
            &mut fresh,
            integrated_timing(),
            &AtomicBool::new(false),
            &bundle.restore_phrase()
        )
        .await
        .is_err());
        assert_eq!(fresh.outcome(), LocalOutcome::RestoreFailed);
        assert_eq!(fresh.recovery().unwrap(), &backup);
    }
}

#[test]
fn project_profile_cannot_enter_legacy_constructor_without_preparation_marker() {
    let project = project_bundle(20);
    assert_eq!(project.binding.profile_version, 2);
    assert!(project.rebind_hardware([1; 6]).is_err());
    let mut binding = project.binding.clone();
    binding.project_preparation_sha256 = None;
    assert!(
        PreparedBundle::new(binding, project.normalized_plan.clone()).is_err(),
        "unchanged project-v2 profile and plan must not enter the legacy constructor"
    );
}

#[test]
fn project_profile_in_direct_legacy_struct_refuses_plan_and_hardware_rebind() {
    let project = project_bundle(20);
    let mut binding = project.binding.clone();
    binding.project_preparation_sha256 = None;
    let direct = PreparedBundle {
        format: 1,
        binding,
        normalized_plan: project.normalized_plan,
        project_preparation: None,
        hardware_preparation: None,
    };
    assert!(
        direct.plan().is_err(),
        "legacy plan admission must reject v2"
    );
    assert!(direct.rebind_hardware([1; 6]).is_err());
}

#[test]
fn project_profile_in_deserialized_legacy_format_refuses_hardware_rebind() {
    let project = project_bundle(20);
    let mut value = serde_json::to_value(&project).unwrap();
    value["format"] = 1.into();
    value.as_object_mut().unwrap().remove("project_preparation");
    value["binding"]
        .as_object_mut()
        .unwrap()
        .remove("project_preparation_sha256");
    let reread: PreparedBundle = serde_json::from_value(value).unwrap();
    assert_eq!(
        reread.binding.profile_sha256,
        project.binding.profile_sha256
    );
    assert_eq!(reread.normalized_plan, project.normalized_plan);
    assert!(
        reread.rebind_hardware([1; 6]).is_err(),
        "deserialized legacy format must not rebind an unchanged v2 profile and plan"
    );
    assert!(reread.plan().is_err());
}

fn synthetic_legacy_bundle() -> PreparedBundle {
    let (_dir, conn, _project, project_profile) = synthetic_inputs(20);
    let mut value: serde_json::Value = serde_json::from_slice(&project_profile).unwrap();
    let legacy: serde_json::Value = serde_json::from_slice(PROFILE).unwrap();
    for field in [
        "format",
        "version",
        "configuration",
        "invisible_parameters",
        "procedure_policy",
    ] {
        value[field] = legacy[field].clone();
    }
    value["id"] = "synthetic-defaults-v1".into();
    value["package_sha256"] = value["product_input"]["package_sha256"].clone();
    value.as_object_mut().unwrap().remove("product_input");
    let bytes = serde_json::to_vec(&value).unwrap();
    let profile = ValidationProfile::from_json(&bytes).unwrap();
    let bundle = prepare_simulated_candidate(&conn, &bytes, IndividualAddress::from_raw(0x1234))
        .unwrap()
        .bundle;
    assert_eq!(bundle.binding.profile_version, profile.version);
    assert_eq!(
        bundle.binding.profile_sha256,
        knx_productdb::sha256_hex(&bytes)
    );
    bundle
}

#[test]
fn project_optional_evidence_does_not_change_legacy_format1_serialization() {
    let project = project_bundle(20);
    let legacy = synthetic_legacy_bundle();
    let bytes = serde_json::to_vec(&legacy).unwrap();
    let text = std::str::from_utf8(&bytes).unwrap();
    assert!(!text.contains("project_preparation"));
    let reread: PreparedBundle = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(reread.format, 1);
    assert_eq!(serde_json::to_vec(&reread).unwrap(), bytes);
    reread.plan().unwrap();
    assert_eq!(reread.binding.profile_version, 1);
    let rebuilt =
        PreparedBundle::new(legacy.binding.clone(), legacy.normalized_plan.clone()).unwrap();
    assert_eq!(rebuilt, legacy);
    let rebound = reread.rebind_hardware([1; 6]).unwrap();
    let mut expected = legacy.clone();
    expected.binding.serial = [1; 6];
    expected.binding.simulated = false;
    assert_eq!(rebound, expected);
    rebound.plan().unwrap();
    rebound.preconditions().unwrap();
    assert_eq!(rebound.consent_phrase(), legacy.consent_phrase());
    assert_eq!(rebound.acceptance_phrase(), legacy.acceptance_phrase());
    assert_eq!(rebound.restore_phrase(), legacy.restore_phrase());
    assert_eq!(
        legacy.consent_phrase(),
        format!(
            "I confirm property validation to {} plan {}",
            legacy.binding.target, legacy.binding.plan_sha256
        )
    );
    let mut mixed = legacy.clone();
    mixed.project_preparation = project.project_preparation;
    assert!(mixed.plan().is_err());
    assert!(mixed.rebind_hardware([1; 6]).is_err());
}

#[test]
fn project_retention_refuses_to_borrow_resolution_from_an_unretained_package() {
    use std::io::{Cursor, Read, Write};
    let (_dir, conn, project, original_profile) = synthetic_inputs(20);
    let (original, _) = synthetic_package(20);
    let mut reader = zip::ZipArchive::new(Cursor::new(original)).unwrap();
    let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for name in ["knx_master.xml", "M-0000/Application.xml"] {
        let mut bytes = Vec::new();
        reader
            .by_name(name)
            .unwrap()
            .read_to_end(&mut bytes)
            .unwrap();
        writer
            .start_file(name, zip::write::SimpleFileOptions::default())
            .unwrap();
        writer.write_all(&bytes).unwrap();
    }
    let partial = writer.finish().unwrap().into_inner();
    knx_productdb::install_package(&conn, "application-only.knxprod", &partial).unwrap();
    let mut profile: serde_json::Value = serde_json::from_slice(&original_profile).unwrap();
    profile["product_input"]["package_sha256"] = knx_productdb::sha256_hex(&partial).into();
    let before = conn.total_changes();
    assert!(
        knx_app::system2_validation_preparation::prepare_project_simulated_candidate(
            &conn,
            &project,
            knx_core::DeviceId(1),
            &serde_json::to_vec(&profile).unwrap()
        )
        .is_err()
    );
    assert_eq!(conn.total_changes(), before);
    project_candidate(&conn, &project, &original_profile);
}

const PROFILE: &[u8] =
    include_bytes!("../../../../data/system2-validation/up258d12-920504-v1.json");

#[test]
fn profile_requires_all_explicit_rules_and_rejects_unknown_or_unbounded_configuration() {
    let profile = ValidationProfile::from_json(PROFILE).unwrap();
    assert_eq!(profile.program_id, "M-0001_A-9205-04-9270");
    for (old, new) in [
        ("complete-defaults-no-links-v1", "any-project"),
        ("strong-hypothesis-simulator-validation-only", "verified"),
        ("declared-counts-zero-flags-full-table-identity-v1", "guess"),
    ] {
        let text = String::from_utf8(PROFILE.to_vec())
            .unwrap()
            .replace(old, new);
        assert!(ValidationProfile::from_json(text.as_bytes()).is_err());
    }
    let mut value: serde_json::Value = serde_json::from_slice(PROFILE).unwrap();
    value["unknown_rule"] = true.into();
    assert!(ValidationProfile::from_json(&serde_json::to_vec(&value).unwrap()).is_err());
}

#[test]
fn preparation_refuses_an_unknown_application_without_creating_or_mutating_products() {
    let dir = tempfile::tempdir().unwrap();
    let conn = knx_productdb::open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
    let before = conn.total_changes();
    let result = prepare_simulated_candidate(&conn, PROFILE, IndividualAddress::from_raw(0x0119));
    assert!(result.is_err());
    assert_eq!(conn.total_changes(), before);
}

#[test]
#[ignore = "requires pinned public UP258 package; set KNXBENCH_PUBLIC_UP258"]
fn exact_public_package_prepares_full_default_candidate_not_a_handcrafted_plan() {
    let path = std::env::var_os("KNXBENCH_PUBLIC_UP258")
        .map(PathBuf::from)
        .expect("set KNXBENCH_PUBLIC_UP258");
    let bytes = std::fs::read(path).unwrap();
    assert_eq!(
        knx_productdb::sha256_hex(&bytes),
        "520d3fcee5b63d079ac7d0bd5e7ce9467ff84cb5cde06ac341b77aba47e3ee0f"
    );
    let dir = tempfile::tempdir().unwrap();
    let conn = knx_productdb::open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
    knx_productdb::install_package_with_limits(
        &conn,
        "public.knxprod",
        &bytes,
        knx_productdb::PackageLimits::LARGE,
    )
    .unwrap();
    let before = conn.total_changes();
    let result =
        prepare_simulated_candidate(&conn, PROFILE, IndividualAddress::from_raw(0x0119)).unwrap();
    assert_eq!(result.source_positions, 21);
    assert_eq!(result.allocation_octets, 1514);
    assert_eq!(result.written_octets, 760);
    assert_eq!(result.preserved_octets, 754);
    assert!(result.bundle.binding.simulated);
    assert!(result.inferences.iter().all(|s| !s.is_empty()));
    assert_eq!(conn.total_changes(), before);
    assert_eq!(result.bundle.plan().unwrap().source_positions(), 21);
}

use knx_app::property_validation::{self as app, LocalOutcome, OperationStore};
use knx_app::property_validation_bundle::PreparedBundle;
use knx_core::commissioning::{
    load_state::LoadState,
    properties::{PID_HARDWARE_TYPE, PID_PROGRAM_VERSION},
};
use knx_net::commissioning::{
    simulator::{
        Interruption, SimulatedAbsoluteAllocation, SimulatedDevice, SimulatedSystem2Object,
        SimulatedSystem2ObjectState, SimulatedTaskControl2, SimulatedTaskSegment, SimulatorConfig,
    },
    SessionTiming,
};
use std::{sync::atomic::AtomicBool, time::Duration};

fn prepared_public() -> PreparedBundle {
    let bytes = std::fs::read(
        std::env::var_os("KNXBENCH_PUBLIC_UP258").expect("set KNXBENCH_PUBLIC_UP258"),
    )
    .unwrap();
    assert_eq!(
        knx_productdb::sha256_hex(&bytes),
        "520d3fcee5b63d079ac7d0bd5e7ce9467ff84cb5cde06ac341b77aba47e3ee0f"
    );
    let dir = tempfile::tempdir().unwrap();
    let db = knx_productdb::open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
    knx_productdb::install_package_with_limits(
        &db,
        "public.knxprod",
        &bytes,
        knx_productdb::PackageLimits::LARGE,
    )
    .unwrap();
    prepare_simulated_candidate(&db, PROFILE, IndividualAddress::from_raw(0x0119))
        .unwrap()
        .bundle
}

fn exact_old_device(bundle: &PreparedBundle, mut config: SimulatorConfig) -> SimulatedDevice {
    // Independent OLD fixture: source-declared extents and identity, NOT an execution
    // of the proposed NEW plan. Unknown real firmware history remains a hypothesis.
    config.mask_version = 0x0025;
    config.serial_number = Some(bundle.binding.serial);
    let device = SimulatedDevice::with_config_at(bundle.plan().unwrap().target(), config);
    device.preset_property(
        0,
        knx_core::commissioning::properties::PID_MANUFACTURER_ID,
        &[0, 1],
    );
    device.preset_property(0, PID_HARDWARE_TYPE, &[0, 1, 0, 0, 0, 91]);
    device.preset_property(3, PID_PROGRAM_VERSION, &[0, 1, 0x92, 5, 4]);
    for (owner, extents, pointer, has_task_control_2) in [
        (
            SimulatedSystem2Object::AddressTable,
            vec![(278, 330)],
            278,
            false,
        ),
        (
            SimulatedSystem2Object::AssociationTable,
            vec![(608, 416)],
            608,
            false,
        ),
        (
            SimulatedSystem2Object::Application,
            vec![(1024, 512), (15360, 256)],
            0,
            true,
        ),
    ] {
        let allocations = extents
            .into_iter()
            .map(|(start, length)| {
                device.preset_memory(u32::from(start), &vec![0xA5; usize::from(length)]);
                SimulatedAbsoluteAllocation {
                    subtype: 0,
                    start,
                    length,
                    access: 0x33,
                    memory_type: 3,
                    attributes: 0,
                }
            })
            .collect();
        device.preset_system2_object(
            owner,
            SimulatedSystem2ObjectState {
                load_state: LoadState::Loaded,
                table_reference: pointer,
                allocations,
                task_segment: Some(SimulatedTaskSegment {
                    start: pointer as u16,
                    pei_type: 0,
                    manufacturer: 1,
                    application: 0x9205,
                    version: 4,
                }),
                task_control_2: has_task_control_2.then_some(SimulatedTaskControl2 {
                    callback: 0,
                    communication_objects: 1024,
                    segment_1: 0,
                    segment_2: 0,
                }),
            },
        );
    }
    device
}

fn integrated_timing() -> SessionTiming {
    SessionTiming {
        connection_timeout: Duration::from_millis(30),
        response_timeout: Duration::from_millis(5),
        poll_interval: Duration::from_millis(1),
        max_transition: Duration::from_millis(30),
        programming_delay: Duration::ZERO,
        ..Default::default()
    }
}

#[tokio::test]
#[ignore = "requires pinned public UP258 package; set KNXBENCH_PUBLIC_UP258"]
async fn exact_public_full_download_and_reopened_recovery_compare_all_modeled_persistent_state() {
    let bundle = prepared_public();
    let device = exact_old_device(&bundle, SimulatorConfig::default());
    let old = device.system2_snapshot();
    let dir = tempfile::tempdir().unwrap();
    let bundle_path = dir.path().join("bundle.json");
    std::fs::write(&bundle_path, serde_json::to_vec(&bundle).unwrap()).unwrap();
    let path = dir.path().join("operation");
    let mut store = OperationStore::create(&path, bundle.binding.clone()).unwrap();
    app::execute_simulated(
        &device,
        &bundle,
        &mut store,
        integrated_timing(),
        &AtomicBool::new(false),
    )
    .await
    .unwrap();
    assert_eq!(store.outcome(), LocalOutcome::Finished);
    let new = device.system2_snapshot();
    assert_ne!(new.memory, old.memory);
    assert_eq!(
        new.objects, old.objects,
        "independent source-shape/task oracle"
    );
    let backup = store.recovery().unwrap();
    assert_eq!(
        backup
            .allocations
            .iter()
            .map(|a| a.data.len())
            .sum::<usize>(),
        1514
    );
    assert_eq!(
        backup.regions.iter().map(|a| a.data.len()).sum::<usize>(),
        760
    );
    drop(store);
    drop(bundle);
    let reopened_bundle: PreparedBundle =
        serde_json::from_slice(&std::fs::read(&bundle_path).unwrap()).unwrap();
    let mut reopened = OperationStore::open(&path, &reopened_bundle.binding).unwrap();
    app::restore_simulated(
        &device,
        &reopened_bundle,
        &mut reopened,
        integrated_timing(),
        &AtomicBool::new(false),
        &reopened_bundle.restore_phrase(),
    )
    .await
    .unwrap();
    assert_eq!(reopened.outcome(), LocalOutcome::Restored);
    assert_eq!(
        device.system2_snapshot(),
        old,
        "bytes, ownership, pointers, identity, task controls and properties must all return"
    );
}

#[tokio::test]
#[ignore = "requires pinned public UP258 package; set KNXBENCH_PUBLIC_UP258"]
async fn exact_public_partial_download_failure_reopens_and_explicitly_restores_old_state() {
    let bundle = prepared_public();
    let device = exact_old_device(
        &bundle,
        SimulatorConfig {
            interrupt_at: Some(Interruption::MemoryWriteAt(15360)),
            ..Default::default()
        },
    );
    let old = device.system2_snapshot();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("operation");
    let mut store = OperationStore::create(&path, bundle.binding.clone()).unwrap();
    assert!(app::execute_simulated(
        &device,
        &bundle,
        &mut store,
        integrated_timing(),
        &AtomicBool::new(false)
    )
    .await
    .is_err());
    assert_eq!(store.outcome(), LocalOutcome::Failed);
    assert!(store.mutation_possible());
    assert_ne!(device.system2_snapshot(), old);
    drop(store);
    let mut reopened = OperationStore::open(&path, &bundle.binding).unwrap();
    app::restore_simulated(
        &device,
        &bundle,
        &mut reopened,
        integrated_timing(),
        &AtomicBool::new(false),
        &bundle.restore_phrase(),
    )
    .await
    .unwrap();
    assert_eq!(reopened.outcome(), LocalOutcome::Restored);
    assert_eq!(device.system2_snapshot(), old);
}

#[tokio::test]
#[ignore = "requires pinned public UP258 package; set KNXBENCH_PUBLIC_UP258"]
async fn exact_public_cancellation_and_crash_checkpoint_boundaries_restore_from_disk() {
    use knx_app::property_validation::Checkpoint;
    use knx_app::property_validation_bundle::RecoveryRecord;
    use knx_core::commissioning::{
        authorisation::AuthorisationPlan,
        mutation::{WriteAuthorisation, WriteScope},
        property_download::PropertyOperation,
    };
    use knx_net::commissioning::{
        property_download::run_experimental_property_plan, property_recovery::PropertyJournalEvent,
    };
    use std::sync::atomic::Ordering;
    let bundle = prepared_public();
    for (cut_source, cancelled) in [(2, false), (6, false), (12, false), (13, true)] {
        let device = exact_old_device(&bundle, SimulatorConfig::default());
        let old = device.system2_snapshot();
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("operation");
        let mut store = OperationStore::create(&path, bundle.binding.clone()).unwrap();
        let cancel = AtomicBool::new(false);
        let plan = bundle.plan().unwrap();
        let result = run_experimental_property_plan(&device,&plan,
            WriteAuthorisation::for_simulator(plan.target(),WriteScope::Download).unwrap(),
            AuthorisationPlan::Skip,integrated_timing(),&bundle.preconditions().unwrap(),
            &mut |event| {
                match event {
                    PropertyJournalEvent::RecoveryAcquired(snapshot) => store.checkpoint(Checkpoint::RecoveryAcquired {recovery: RecoveryRecord::from_snapshot(snapshot,&bundle).map_err(|e|e.to_string())?}),
                    PropertyJournalEvent::OperationStarting {operation_index,mutation_may_have_started,..} => store.checkpoint(Checkpoint::StepStarted {index:operation_index,mutation:mutation_may_have_started || matches!(plan.steps()[operation_index].operation,PropertyOperation::LoadControl{..}|PropertyOperation::WriteMemory{..}|PropertyOperation::Disconnect)}),
                    PropertyJournalEvent::OperationCompleted {operation_index,source_index,..} => {
                        store.checkpoint(Checkpoint::StepFinished {index:operation_index}).map_err(|e|e.to_string())?;
                        if source_index == cut_source {
                            if cancelled { cancel.store(true,Ordering::SeqCst); }
                            else {return Err("deterministic crash-boundary journal cut; caller terminal checkpoint intentionally absent".into());}
                        }
                        Ok(())
                    }
                    PropertyJournalEvent::FinalPointerObservations(_) => Err(std::io::Error::other("unexpected final pointer observation")),
                    PropertyJournalEvent::InProcedurePointerObservation(_) => Err(std::io::Error::other("unexpected in-procedure pointer observation")),
                }.map_err(|e|e.to_string())
            },&||cancel.load(Ordering::SeqCst)).await;
        assert!(result.is_err(), "source {cut_source} must stop");
        if cancelled {
            store.checkpoint(Checkpoint::Cancelled).unwrap();
        }
        drop(store);
        let mut reopened = OperationStore::open(&path, &bundle.binding).unwrap();
        assert_eq!(
            reopened.outcome(),
            if cancelled {
                LocalOutcome::Cancelled
            } else {
                LocalOutcome::Interrupted
            }
        );
        assert!(reopened.mutation_possible());
        app::restore_simulated(
            &device,
            &bundle,
            &mut reopened,
            integrated_timing(),
            &AtomicBool::new(false),
            &bundle.restore_phrase(),
        )
        .await
        .unwrap();
        assert_eq!(
            device.system2_snapshot(),
            old,
            "source {cut_source} old persistent state"
        );
        assert_eq!(reopened.outcome(), LocalOutcome::Restored);
    }
}

#[tokio::test]
#[ignore = "requires pinned public UP258 package; set KNXBENCH_PUBLIC_UP258"]
async fn exact_public_incomplete_identity_or_cancelled_recovery_is_durably_failed_not_restored() {
    let bundle = prepared_public();
    for cancel_restore in [false, true] {
        let device = exact_old_device(&bundle, SimulatorConfig::default());
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("operation");
        let mut store = OperationStore::create(&path, bundle.binding.clone()).unwrap();
        app::execute_simulated(
            &device,
            &bundle,
            &mut store,
            integrated_timing(),
            &AtomicBool::new(false),
        )
        .await
        .unwrap();
        if !cancel_restore {
            device.preset_property(3, PID_PROGRAM_VERSION, &[]);
        }
        drop(store);
        let mut reopened = OperationStore::open(&path, &bundle.binding).unwrap();
        assert!(app::restore_simulated(
            &device,
            &bundle,
            &mut reopened,
            integrated_timing(),
            &AtomicBool::new(cancel_restore),
            &bundle.restore_phrase()
        )
        .await
        .is_err());
        assert_eq!(reopened.outcome(), LocalOutcome::RestoreFailed);
        drop(reopened);
        let mut fresh = OperationStore::open(&path, &bundle.binding).unwrap();
        assert_eq!(fresh.outcome(), LocalOutcome::RestoreFailed);
        assert!(
            app::restore_simulated(
                &device,
                &bundle,
                &mut fresh,
                integrated_timing(),
                &AtomicBool::new(false),
                &bundle.restore_phrase()
            )
            .await
            .is_err(),
            "failed restore must not automatically retry"
        );
    }
}

#[tokio::test]
#[ignore = "requires pinned public UP258 package; set KNXBENCH_PUBLIC_UP258"]
async fn exact_public_destructive_unload_counterhypothesis_is_not_reported_as_restored() {
    let bundle = prepared_public();
    let device = exact_old_device(
        &bundle,
        SimulatorConfig {
            experimental_destroy_allocation_memory_on_unload: true,
            ..Default::default()
        },
    );
    let old = device.system2_snapshot();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("operation");
    let mut store = OperationStore::create(&path, bundle.binding.clone()).unwrap();
    assert!(
        app::execute_simulated(
            &device,
            &bundle,
            &mut store,
            integrated_timing(),
            &AtomicBool::new(false),
        )
        .await
        .is_err(),
        "destructive Unload must fail the full-allocation verification already during NEW"
    );
    assert_eq!(store.outcome(), LocalOutcome::Failed);
    drop(store);
    let mut reopened = OperationStore::open(&path, &bundle.binding).unwrap();
    assert!(app::restore_simulated(
        &device,
        &bundle,
        &mut reopened,
        integrated_timing(),
        &AtomicBool::new(false),
        &bundle.restore_phrase()
    )
    .await
    .is_err());
    assert_eq!(reopened.outcome(), LocalOutcome::RestoreFailed);
    assert_ne!(
        device.system2_snapshot(),
        old,
        "masked bytes lost by destructive unload are an unresolved physical rule"
    );
}
