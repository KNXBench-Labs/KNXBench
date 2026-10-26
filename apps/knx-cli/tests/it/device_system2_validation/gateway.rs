//! Built command through the in-process simulator gateway: the journal send boundary is exact.
use std::{
    collections::BTreeMap,
    net::{SocketAddr, UdpSocket as StdSocket},
    path::Path,
    process::Output,
    sync::Arc,
};

use knx_app::{
    commissioning_activity::OneShotLog,
    property_validation::{LocalOutcome, OperationStore},
    property_validation_bundle::PreparedBundle,
};
use knx_core::commissioning::{
    load_state::LoadState,
    properties::{PID_HARDWARE_TYPE, PID_MANUFACTURER_ID, PID_PROGRAM_VERSION},
};
use knx_core::IndividualAddress;
use knx_net::commissioning::{
    simulator::{
        SimulatedAbsoluteAllocation, SimulatedDevice, SimulatedSystem2Object,
        SimulatedSystem2ObjectState, SimulatorConfig,
    },
    simulator_gateway::{serve, GatewayConfig},
};
use serde_json::{json, Value};

use super::{bundle, bundle_version, cli};

/// The synthetic bundle's device: 3C00h..3C03h allocated on the application object.
fn device(bundle: &PreparedBundle, hardware: [u8; 6], config: SimulatorConfig) -> SimulatedDevice {
    let b = &bundle.binding;
    let device = SimulatedDevice::with_config_at(
        b.target.parse().unwrap(),
        SimulatorConfig {
            mask_version: b.mask,
            serial_number: Some(b.serial),
            ..config
        },
    );
    device.preset_property(0, PID_MANUFACTURER_ID, &b.manufacturer.to_be_bytes());
    device.preset_property(0, PID_HARDWARE_TYPE, &hardware);
    device.preset_property(3, PID_PROGRAM_VERSION, &b.application);
    device.preset_system2_object(
        SimulatedSystem2Object::Application,
        SimulatedSystem2ObjectState {
            load_state: LoadState::Loaded,
            table_reference: 0x3c00,
            allocations: vec![SimulatedAbsoluteAllocation {
                subtype: 0,
                start: 0x3c00,
                length: 4,
                access: 0xff,
                memory_type: 2,
                attributes: 0,
            }],
            ..Default::default()
        },
    );
    device.preset_memory(0x3c00, &[0xa5; 4]);
    device
}

/// Serves `device` on a loopback port for the rest of the test process.
fn gateway(device: SimulatedDevice) -> String {
    let probe = StdSocket::bind("127.0.0.1:0").unwrap();
    let address = probe.local_addr().unwrap();
    let SocketAddr::V4(advertised) = address else {
        unreachable!("bound to IPv4")
    };
    let (ready, started) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        runtime.block_on(async move {
            probe.set_nonblocking(true).unwrap();
            let socket = tokio::net::UdpSocket::from_std(probe).unwrap();
            let mut config =
                GatewayConfig::new(advertised, IndividualAddress::new(1, 1, 0).unwrap());
            config.properties = BTreeMap::from([
                ((0, 1, 56), vec![0x00, 0xfe]),
                ((0, 1, 83), vec![0x09, 0x1a]),
            ]);
            ready.send(()).unwrap();
            let _ = serve(Arc::new(device), socket, config).await;
        });
    });
    started.recv().unwrap();
    address.to_string()
}

fn download(dir: &Path, path: &Path, gateway: &str) -> Output {
    download_with(dir, path, gateway, &[])
}

fn download_with(dir: &Path, path: &Path, gateway: &str, extra: &[&str]) -> Output {
    let preview = cli()
        .args(["--bundle", path.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(preview.status.success());
    let preview = String::from_utf8(preview.stdout).unwrap();
    let phrase = |label: &str| {
        preview
            .lines()
            .find_map(|line| line.strip_prefix(label))
            .unwrap()
            .to_owned()
    };
    cli()
        .env("XDG_RUNTIME_DIR", dir.join("runtime"))
        .env("XDG_DATA_HOME", dir.join("data"))
        .env("XDG_CONFIG_HOME", dir.join("config"))
        .args([
            "--bundle",
            path.to_str().unwrap(),
            "--gateway",
            gateway,
            "--operation",
            dir.join("operation").to_str().unwrap(),
            "--activity-history",
            dir.join("activity.sqlite").to_str().unwrap(),
            "--confirm",
            &phrase("Confirmation: "),
            "--accept-hypothesis",
            &phrase("Hypothesis acceptance: "),
            "--download-confirm",
            &phrase("Ordinary download: "),
            "--release-confirm",
            &phrase("Best-effort release: "),
        ])
        .args(extra)
        .output()
        .unwrap()
}

fn recover(dir: &Path, path: &Path, bundle: &PreparedBundle, gateway: &str) -> Output {
    let target: IndividualAddress = bundle.binding.target.parse().unwrap();
    cli()
        .env("XDG_RUNTIME_DIR", dir.join("runtime"))
        .env("XDG_DATA_HOME", dir.join("data"))
        .env("XDG_CONFIG_HOME", dir.join("config"))
        .args([
            "--bundle",
            path.to_str().unwrap(),
            "--gateway",
            gateway,
            "--recover",
            dir.join("operation").to_str().unwrap(),
            "--activity-history",
            dir.join("activity.sqlite").to_str().unwrap(),
            "--confirm-recovery",
            &bundle.restore_phrase(),
            "--accept-hypothesis",
            &bundle.acceptance_phrase(),
            "--download-confirm",
            &knx_core::commissioning::mutation::required_confirmation_phrase(
                target,
                knx_core::commissioning::mutation::WriteScope::Download,
            ),
            "--release-confirm",
            &knx_net::commissioning::hardware_release::required_release_phrase(target),
        ])
        .output()
        .unwrap()
}

/// Every journal row in start order, read back by a fresh strict reader.
fn journal_rows(dir: &Path) -> Vec<Value> {
    let log = Arc::new(OneShotLog::for_run(dir.join("activity.sqlite")).unwrap());
    let (rows, more) = log.history_page(0, 10).unwrap();
    assert!(!more);
    rows.iter()
        .map(|row| {
            let row = serde_json::to_value(row).unwrap();
            assert_eq!(row["kind"], "system2Validation");
            assert_eq!(row["address"], "1.1.25");
            assert_eq!(row["interrupted"], false);
            assert!(row["finishedAt"].is_string());
            row
        })
        .collect()
}

/// The single journal row.
fn journal(dir: &Path) -> Value {
    let mut rows = journal_rows(dir);
    assert_eq!(rows.len(), 1);
    rows.remove(0)
}

fn store(dir: &Path, bundle: &PreparedBundle) -> OperationStore {
    let mut binding = bundle.binding.clone();
    binding.simulated = false;
    OperationStore::open(&dir.join("operation"), &binding).unwrap()
}

#[test]
fn identity_refusal_before_recovery_is_not_sent() {
    let dir = tempfile::tempdir().unwrap();
    let (path, bundle) = bundle(dir.path());
    let gateway = gateway(device(&bundle, [9; 6], SimulatorConfig::default()));
    let out = download(dir.path(), &path, &gateway);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_eq!(out.status.code(), Some(1), "{stderr}");
    let store = store(dir.path(), &bundle);
    assert_eq!(store.outcome(), LocalOutcome::Failed);
    assert!(store.recovery().is_none());
    assert!(!store.mutation_possible());
    let row = journal(dir.path());
    assert_eq!(row["state"], "notSent", "{row}");
    assert_eq!(
        row["writeEvidence"],
        json!({"backupRecorded": false, "sendPossible": false})
    );
}

#[test]
fn completed_download_is_verified_after_its_recorded_boundary() {
    let dir = tempfile::tempdir().unwrap();
    let (path, bundle) = bundle(dir.path());
    let gateway = gateway(device(&bundle, [1; 6], SimulatorConfig::default()));
    let out = download(dir.path(), &path, &gateway);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let store = store(dir.path(), &bundle);
    assert_eq!(store.outcome(), LocalOutcome::Finished);
    assert!(store.recovery().is_some());
    let row = journal(dir.path());
    assert_eq!(row["state"], "verified", "{row}");
    assert_eq!(
        row["writeEvidence"],
        json!({"backupRecorded": true, "sendPossible": true})
    );
}

#[test]
fn failure_after_the_boundary_is_effect_unverified() {
    let dir = tempfile::tempdir().unwrap();
    let (path, bundle) = bundle(dir.path());
    let config = SimulatorConfig {
        drop_load_state_writes: true,
        ..Default::default()
    };
    let gateway = gateway(device(&bundle, [1; 6], config));
    let out = download(dir.path(), &path, &gateway);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_eq!(out.status.code(), Some(1), "{stderr}");
    let store = store(dir.path(), &bundle);
    assert_eq!(store.outcome(), LocalOutcome::Failed);
    assert!(store.recovery().is_some());
    assert!(store.mutation_possible());
    let row = journal(dir.path());
    assert_eq!(row["state"], "effectUnverified", "{row}");
    assert_eq!(
        row["writeEvidence"],
        json!({"backupRecorded": true, "sendPossible": true})
    );
}

#[test]
fn recovery_replay_records_its_own_boundary_and_is_verified() {
    let dir = tempfile::tempdir().unwrap();
    let (path, bundle) = bundle(dir.path());
    let gateway = gateway(device(&bundle, [1; 6], SimulatorConfig::default()));
    let out = download(dir.path(), &path, &gateway);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let out = recover(dir.path(), &path, &bundle, &gateway);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(store(dir.path(), &bundle).outcome(), LocalOutcome::Restored);
    let rows = journal_rows(dir.path());
    assert_eq!(rows.len(), 2);
    for row in rows {
        assert_eq!(row["state"], "verified", "{row}");
        assert_eq!(
            row["writeEvidence"],
            json!({"backupRecorded": true, "sendPossible": true})
        );
    }
}

#[test]
fn a_failed_report_write_keeps_the_witnessed_device_result() {
    let dir = tempfile::tempdir().unwrap();
    let (path, bundle) = bundle(dir.path());
    let gateway = gateway(device(&bundle, [1; 6], SimulatorConfig::default()));
    let report = dir.path().join("missing").join("report.json");
    let out = download_with(
        dir.path(),
        &path,
        &gateway,
        &["--report", report.to_str().unwrap()],
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_eq!(out.status.code(), Some(1), "{stderr}");
    assert!(!report.exists());
    assert_eq!(store(dir.path(), &bundle).outcome(), LocalOutcome::Finished);
    let row = journal(dir.path());
    assert_eq!(row["state"], "verified", "{row}");
}

#[test]
fn v1_defaults_recovery_of_an_existing_operation_is_admitted_and_restores() {
    use knx_app::property_validation::{execute_simulated, Checkpoint, FailureCode};
    let dir = tempfile::tempdir().unwrap();
    let (path, bundle) = bundle_version(dir.path(), 1);
    let device = device(&bundle, [1; 6], SimulatorConfig::default());
    // An existing v1 hardware operation: durable original recovery, then a failed download.
    let mut simulated =
        OperationStore::create(&dir.path().join("simulated"), bundle.binding.clone()).unwrap();
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(execute_simulated(
            &device,
            &bundle,
            &mut simulated,
            knx_net::commissioning::SessionTiming::default(),
            &std::sync::atomic::AtomicBool::new(false),
        ))
        .unwrap();
    let rebound = bundle.rebind_hardware(bundle.binding.serial).unwrap();
    let mut recovery = simulated.recovery().unwrap().clone();
    recovery.original_bundle = rebound.clone();
    let mut existing =
        OperationStore::create(&dir.path().join("operation"), rebound.binding.clone()).unwrap();
    existing
        .checkpoint(Checkpoint::RecoveryAcquired { recovery })
        .unwrap();
    existing
        .checkpoint(Checkpoint::Failed {
            code: FailureCode::Executor,
            local_diagnostic: "synthetic existing v1 operation".into(),
        })
        .unwrap();
    drop(existing);
    let gateway = gateway(device);
    let out = recover(dir.path(), &path, &bundle, &gateway);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(store(dir.path(), &bundle).outcome(), LocalOutcome::Restored);
    let row = journal(dir.path());
    assert_eq!(row["state"], "verified", "{row}");
}
