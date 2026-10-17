//! Tests for the shared read-only diagnostic service: privacy, loss, lease and archive.
use knx_app::device_diagnostics::{acquire, bundle, prepare, read_bundle, write_new_bundle};
use knx_core::{DeviceId, DeviceInstance, Language, Project, SourceRef};
use knx_net::commissioning::simulator::{SimulatedDevice, SimulatorConfig};
use std::sync::atomic::{AtomicBool, Ordering};

fn project(address: &str) -> Project {
    let mut p = Project::new(Language("en".into()));
    p.devices.insert(DeviceInstance {
        id: DeviceId(7),
        source: SourceRef {
            path: "/home/customer/SECRET".into(),
            ets_id: "SECRET_SOURCE".into(),
        },
        name: "SECRET_ROOM".into(),
        description: Some("SECRET_PASSWORD".into()),
        address: Some(address.parse().unwrap()),
        product_ref: "SECRET_PRODUCT".into(),
        program_ref: "SECRET_PROGRAM".into(),
        commissioning: Default::default(),
        visibility_calculated: true,
        com_objects: vec![],
        binary_data: vec![],
    });
    p
}
#[tokio::test]
async fn shared_service_exports_and_verifies_raw_identity_without_private_project_data() {
    let p = project("1.1.25");
    let before = p.clone();
    let plan = prepare(&p, DeviceId(7), None).unwrap();
    let d = SimulatedDevice::with_config(SimulatorConfig {
        mask_version: 0xabcd,
        ..Default::default()
    });
    d.preset_property(0, 12, &[0x12, 0x34]);
    let report = acquire(
        &d,
        &plan,
        Default::default(),
        "synthetic-test",
        &AtomicBool::new(false),
        |_| {},
    )
    .await;
    assert!(report.simulated);
    assert!(!report.written);
    assert_eq!(report.items[0].bytes, [0xab, 0xcd]);
    assert_eq!(report.items[1].bytes, [0x12, 0x34]);
    assert_eq!(report.memory_status, "not-requested");
    let bytes = bundle(&report).unwrap();
    assert_eq!(read_bundle(&bytes).unwrap(), report);
    let decoded = serde_json::to_string(&read_bundle(&bytes).unwrap()).unwrap();
    for secret in ["SECRET", "/home/", "1.1.25", "192.0.2."] {
        assert!(!decoded.contains(secret), "leaked {secret}");
    }
    assert_eq!(p, before);
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("diagnostic.zip");
    write_new_bundle(&path, &report).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            std::fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            0o600
        );
    }
    assert_eq!(read_bundle(&std::fs::read(&path).unwrap()).unwrap(), report);
    assert!(
        write_new_bundle(&path, &report).is_err(),
        "must never overwrite"
    );
    assert!(read_bundle(&bytes[..bytes.len() / 2]).is_err());
}
#[test]
fn diagnostic_processes_share_one_nonblocking_lease() {
    let dir = tempfile::tempdir().unwrap();
    let first = knx_app::device_diagnostics::lease_in(dir.path()).unwrap();
    assert!(knx_app::device_diagnostics::lease_in(dir.path()).is_err());
    drop(first);
    assert!(knx_app::device_diagnostics::lease_in(dir.path()).is_ok());
}
#[test]
fn excluded_missing_or_duplicate_target_fails_before_adapter_construction() {
    assert!(prepare(&project("1.1.25"), DeviceId(999), None).is_err());
    for address in knx_core::EXCLUDED_INDIVIDUAL_ADDRESSES {
        assert!(knx_core::is_project_excluded(*address));
        assert!(prepare(&project(&address.to_string()), DeviceId(7), None).is_err());
    }
    let mut p = project("1.1.25");
    let mut copy = p.devices.iter().next().unwrap().clone();
    copy.id = DeviceId(8);
    p.devices.insert(copy);
    assert!(prepare(&p, DeviceId(7), None).is_err());
}
#[tokio::test]
async fn cancelled_partial_is_still_a_valid_export() {
    let plan = prepare(&project("1.1.25"), DeviceId(7), None).unwrap();
    let d = SimulatedDevice::new();
    let cancel = AtomicBool::new(false);
    let report = acquire(
        &d,
        &plan,
        Default::default(),
        "synthetic-test",
        &cancel,
        |r| {
            if r.items[0].status == "read" {
                cancel.store(true, Ordering::SeqCst);
            }
        },
    )
    .await;
    assert_eq!(report.stop_reason.as_deref(), Some("cancelled"));
    assert_eq!(report.items[0].status, "read");
    assert_eq!(report.items[1].status, "not-read");
    assert_eq!(read_bundle(&bundle(&report).unwrap()).unwrap(), report);
}
#[tokio::test]
async fn archive_rejects_corrupt_members_extra_paths_and_unknown_format() {
    use std::io::{Cursor, Read, Write};
    let plan = prepare(&project("1.1.25"), DeviceId(7), None).unwrap();
    let report = acquire(
        &SimulatedDevice::new(),
        &plan,
        Default::default(),
        "synthetic-test",
        &AtomicBool::new(false),
        |_| {},
    )
    .await;
    let original = bundle(&report).unwrap();
    for mode in [
        "corrupt-report",
        "extra-path",
        "unknown-format",
        "missing-properties",
        "duplicate",
    ] {
        let mut input = zip::ZipArchive::new(Cursor::new(&original)).unwrap();
        let mut output = zip::ZipWriter::new(Cursor::new(vec![]));
        for i in 0..input.len() {
            let mut f = input.by_index(i).unwrap();
            let name = f.name().to_string();
            let mut data = vec![];
            f.read_to_end(&mut data).unwrap();
            if mode == "missing-properties" && name == "properties.json" {
                continue;
            }
            if mode == "corrupt-report" && name == "report.json" {
                data.push(b' ');
            }
            if mode == "unknown-format" && name == "manifest.json" {
                let mut v: serde_json::Value = serde_json::from_slice(&data).unwrap();
                v["format"] = 99.into();
                data = serde_json::to_vec(&v).unwrap();
            }
            output
                .start_file(&name, zip::write::SimpleFileOptions::default())
                .unwrap();
            output.write_all(&data).unwrap();
        }
        if mode == "extra-path" {
            output
                .start_file("../secret", zip::write::SimpleFileOptions::default())
                .unwrap();
        }
        if mode == "duplicate" {
            output
                .start_file(
                    "properties-copy.json",
                    zip::write::SimpleFileOptions::default(),
                )
                .unwrap();
        }
        let bytes = output.finish().unwrap().into_inner();
        assert!(read_bundle(&bytes).is_err(), "accepted {mode}");
    }
}

/// The durable activity history admits a closed set of kinds. An unknown kind
/// fails persistence, marks history unavailable and so blocks every later
/// write/download in that server session, and an older reader would refuse it.
#[test]
fn a_diagnostic_run_is_a_valid_durable_activity_and_keeps_writes_admissible() {
    use knx_app::commissioning_activity::OneShotLog;
    use std::sync::Arc;
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("activity.sqlite");
    let log = Arc::new(OneShotLog::persistent(path.clone(), "synthetic".into()));
    let failed = log.start("deviceDiagnostics", Some("1.1.25".into()));
    failed.finish("failed");
    let finished = log.start("deviceDiagnostics", Some("1.1.25".into()));
    finished.finish("finished");
    assert_eq!(log.history_state(), "configured");
    assert!(log.ensure_write_available().is_ok());
    // A fresh reader validates every stored row on load.
    let reopened = OneShotLog::persistent(path, "synthetic-2".into());
    let (entries, more) = reopened.history_page(0, 100).unwrap();
    assert!(!more);
    let states: Vec<_> = entries
        .iter()
        .map(|e| serde_json::to_value(e).unwrap())
        .map(|v| {
            (
                v["kind"].as_str().unwrap().to_string(),
                v["state"].as_str().unwrap().to_string(),
            )
        })
        .collect();
    assert_eq!(states.len(), 2);
    assert!(states.iter().all(|(k, _)| k == "deviceDiagnostics"));
    assert!(states.iter().any(|(_, s)| s == "failed"));
    assert!(states.iter().any(|(_, s)| s == "finished"));
}
/// Error and stop-reason fields are closed code tokens; free text (a path,
/// name or address) can neither be bundled nor survive a reimport.
#[test]
fn free_text_errors_never_enter_or_leave_an_archive() {
    let plan = knx_app::device_diagnostics::Prepared {
        target: knx_core::ContactableAddress::new("1.1.25".parse().unwrap()).unwrap(),
        expected: Default::default(),
        reference: "a".repeat(64),
    };
    let good = knx_app::device_diagnostics::pending(&plan, "synthetic", true);
    assert!(bundle(&good).is_ok());
    let mut item_error = good.clone();
    item_error.items[0].error = Some("/home/customer/SECRET".into());
    assert!(bundle(&item_error).is_err());
    let mut stop = good.clone();
    stop.stop_reason = Some("Connection to 192.0.2.10 lost".into());
    assert!(bundle(&stop).is_err());
}
