//! Built command through the in-process simulator gateway: send boundary and bundle are exact.
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

#[path = "../../../../../crates/knx-app/src/sentinel_forms_tests.rs"]
mod sentinel_forms;

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
    gateway_counted(device).0
}

/// [`gateway`], also counting every datagram the gateway receives from a tool.
fn gateway_counted(device: SimulatedDevice) -> (String, Arc<std::sync::atomic::AtomicUsize>) {
    let datagrams = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let counter = Arc::clone(&datagrams);
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
            config.observer = Some(Arc::new(move |event| {
                if let knx_net::commissioning::simulator_gateway::GatewayEvent::Datagram {
                    from_tool: true,
                    ..
                } = event
                {
                    counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                }
            }));
            ready.send(()).unwrap();
            let _ = serve(Arc::new(device), socket, config).await;
        });
    });
    started.recv().unwrap();
    (address.to_string(), datagrams)
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
fn superseded_defaults_recovery_of_an_existing_operation_is_admitted_and_restores() {
    // v1 omits independent data, v2 has the superseded invisible reading; both still restore.
    for version in [1, 2] {
        superseded_recovery_restores(version);
    }
}

fn superseded_recovery_restores(version: u32) {
    use knx_app::property_validation::{execute_simulated, Checkpoint, FailureCode};
    let dir = tempfile::tempdir().unwrap();
    let (path, bundle) = bundle_version(dir.path(), version);
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
            local_diagnostic: format!("synthetic existing v{version} operation"),
            pointer_failure: None,
            final_step: None,
            allocation_evidence: None,
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
    assert_eq!(
        store(dir.path(), &bundle).outcome(),
        LocalOutcome::Restored,
        "profile v{version}"
    );
    let row = journal(dir.path());
    assert_eq!(row["state"], "verified", "profile v{version}: {row}");
}

/// What the device holds after LoadCompleted instead of the written `01 02 03 04`:
/// only the actual side of the final verification can carry it.
const LIE: [u8; 4] = [0xd7, 0x3e, 0x9c, 0x6b];

/// DIAG-03 Original14 C1: the built command against the loopback gateway fails its
/// final allocation verification, keeps the failure, and its diagnostic bundle holds
/// the typed failure and the closed projection but neither byte side and no recording.
#[test]
fn a_final_mismatch_is_bundled_with_neither_byte_side() {
    let dir = tempfile::tempdir().unwrap();
    let (path, bundle) = bundle(dir.path());
    let config = SimulatorConfig {
        memory_after_load_completed: Some((0x3c00, LIE.to_vec())),
        ..Default::default()
    };
    let gateway = gateway(device(&bundle, [1; 6], config));
    let zip = dir.path().join("bundle.zip");
    let out = download_with(
        dir.path(),
        &path,
        &gateway,
        &["--diagnostic-bundle", zip.to_str().unwrap()],
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_eq!(out.status.code(), Some(1), "{stderr}");
    assert!(stderr.contains("diagnostic bundle written to"), "{stderr}");

    // Preconditions: the failure is the final mismatch and the journal keeps both sides.
    let store = store(dir.path(), &bundle);
    assert_eq!(store.outcome(), LocalOutcome::Failed);
    let progress = serde_json::to_value(store.progress()).unwrap();
    assert_eq!(
        progress["final_step_failures"],
        json!([{"run": "download", "category": "verificationMismatch", "stage": "allocation"}])
    );
    let evidence = store.allocation_mismatch_evidence();
    assert_eq!(evidence.len(), 1);
    let evidence = serde_json::to_value(&evidence[0]).unwrap().to_string();
    for side in [
        sentinel_forms::hex(&LIE),
        sentinel_forms::hex(&[1, 2, 3, 4]),
    ] {
        assert!(
            evidence.contains(&side),
            "precondition: the journal holds {side}"
        );
    }
    assert_eq!(journal(dir.path())["state"], "effectUnverified");

    let mut archive = zip::ZipArchive::new(std::fs::File::open(&zip).unwrap()).unwrap();
    let mut names = Vec::new();
    for index in 0..archive.len() {
        let mut entry = archive.by_index(index).unwrap();
        let name = entry.name().to_owned();
        let mut content = Vec::new();
        std::io::Read::read_to_end(&mut entry, &mut content).unwrap();
        sentinel_forms::assert_no_sentinel_bytes(
            &format!("entry {name}"),
            &content,
            &[&LIE, &[1, 2, 3, 4]],
        );
        sentinel_forms::assert_no_sentinel_bytes(
            &format!("entry name {name}"),
            name.as_bytes(),
            &[&LIE, &[1, 2, 3, 4]],
        );
        assert!(!name.ends_with("recording.jsonl"), "{name}");
        if name == "operations/system2-private.json" {
            let private: Value = serde_json::from_slice(&content).unwrap();
            assert_eq!(
                private["runs"][0]["allocationMismatch"]["classification"], "recordedBytesWithheld",
                "{private}"
            );
        }
        if name == "reports/system2-report.json" {
            let report: Value = serde_json::from_slice(&content).unwrap();
            assert_eq!(report["finalStepFailures"], progress["final_step_failures"]);
        }
        names.push(name);
    }
    for required in [
        "operations/system2-private.json",
        "reports/system2-report.json",
    ] {
        assert!(names.iter().any(|name| name == required), "{names:?}");
    }
}

/// Every file below `root`, relative, sorted.
fn files_below(root: &Path) -> Vec<String> {
    let mut out = Vec::new();
    let mut pending = vec![root.to_path_buf()];
    while let Some(dir) = pending.pop() {
        for entry in std::fs::read_dir(&dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                pending.push(path);
            } else {
                out.push(path.strip_prefix(root).unwrap().display().to_string());
            }
        }
    }
    out.sort();
    out
}

/// DIAG-03 Original14 C2 (no flag): a real failed System-2 run without
/// `--diagnostic-bundle` writes no bundle and no recording anywhere.
#[test]
fn without_the_flag_a_failed_run_writes_no_bundle_and_no_recording() {
    let dir = tempfile::tempdir().unwrap();
    let (path, bundle) = bundle(dir.path());
    let config = SimulatorConfig {
        memory_after_load_completed: Some((0x3c00, LIE.to_vec())),
        ..Default::default()
    };
    let gateway = gateway(device(&bundle, [1; 6], config));
    let out = download(dir.path(), &path, &gateway);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_eq!(out.status.code(), Some(1), "{stderr}");
    assert!(!stderr.contains("diagnostic bundle"), "{stderr}");
    assert_eq!(store(dir.path(), &bundle).outcome(), LocalOutcome::Failed);
    let files = files_below(dir.path());
    assert!(
        files.iter().all(|file| !file.ends_with(".zip")
            && !file.ends_with(".jsonl")
            && !file.contains(".knxbench-recordings-")),
        "{files:#?}"
    );
}

/// DIAG-03 Original14 C2 (duplicate attachment): two bundle destinations, or the
/// same one twice, are refused before a single datagram reaches the gateway.
#[test]
fn a_doubled_bundle_flag_is_refused_before_any_datagram() {
    let dir = tempfile::tempdir().unwrap();
    let (path, bundle) = bundle(dir.path());
    let (gateway, datagrams) = gateway_counted(device(&bundle, [1; 6], SimulatorConfig::default()));
    let (one, two) = (dir.path().join("one.zip"), dir.path().join("two.zip"));
    for extra in [
        [
            "--diagnostic-bundle",
            one.to_str().unwrap(),
            "--diagnostic-bundle",
            two.to_str().unwrap(),
        ],
        [
            "--diagnostic-bundle",
            one.to_str().unwrap(),
            "--diagnostic-bundle",
            one.to_str().unwrap(),
        ],
    ] {
        let out = download_with(dir.path(), &path, &gateway, &extra);
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(!out.status.success(), "{stderr}");
        assert!(stderr.contains("given twice"), "{stderr}");
        assert!(!one.exists() && !two.exists());
        assert_eq!(datagrams.load(std::sync::atomic::Ordering::SeqCst), 0);
        assert!(!dir.path().join("operation").exists());
    }
    // Control: the same gateway is reachable, so zero above means refused, not unreachable.
    let out = download_with(
        dir.path(),
        &path,
        &gateway,
        &["--diagnostic-bundle", one.to_str().unwrap()],
    );
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(datagrams.load(std::sync::atomic::Ordering::SeqCst) > 0);
    assert!(one.exists());
}
