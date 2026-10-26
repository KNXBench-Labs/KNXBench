//! Built command: offline refusal and durable simulator exercise, never a gateway.
use knx_app::{
    property_validation::Binding,
    property_validation_bundle::{PlanRecord, PreparedBundle},
};
use knx_core::commissioning::{
    absolute_load_control::{absolute_data, AbsolutePropertySegment},
    load_control::event_payload,
    load_control_memory::SegmentMemoryType,
    load_state::{LoadEvent, MaskVersion},
    property_download::{PropertyDownloadPlan, PropertyDownloadStep, PropertyOperation as Op},
};
use std::{fs, net::UdpSocket, process::Command, time::Duration};

mod gateway;
mod journal;

fn bundle(dir: &std::path::Path) -> (std::path::PathBuf, PreparedBundle) {
    let step = |source_index, operation| PropertyDownloadStep {
        source_index,
        operation,
    };
    let ctrl = |i, payload| step(i, Op::LoadControl { object: 3, payload });
    let plan = PropertyDownloadPlan::new(
        "1.1.25".parse().unwrap(),
        MaskVersion(0x0025),
        2,
        vec![
            step(0, Op::Connect),
            step(1, Op::HardwareCheck { expected: [1; 6] }),
            ctrl(2, event_payload(LoadEvent::Unload)),
            ctrl(3, event_payload(LoadEvent::StartLoading)),
            ctrl(
                4,
                absolute_data(AbsolutePropertySegment {
                    start: 0x3c00,
                    length: 4,
                    access: 0xff,
                    memory_type: SegmentMemoryType::Eeprom,
                    checksum_disabled: true,
                })
                .unwrap(),
            ),
            step(
                4,
                Op::WriteMemory {
                    object: 3,
                    address: 0x3c00,
                    data: vec![1, 2, 3, 4],
                },
            ),
            ctrl(5, event_payload(LoadEvent::LoadCompleted)),
            step(6, Op::Disconnect),
        ],
    )
    .unwrap();
    let record = PlanRecord::from_plan(&plan);
    let binding = Binding {
        target: plan.target().to_string(),
        mask: 0x0025,
        manufacturer: 2,
        serial: [0, 2, 1, 2, 3, 4],
        hardware: [1; 6],
        application: [0, 2, 0x12, 0x34, 1],
        profile_id: "synthetic-v1".into(),
        profile_version: 1,
        profile_sha256: "a".repeat(64),
        source_sha256: "b".repeat(64),
        plan_sha256: record.sha256().unwrap(),
        simulated: true,
        project_preparation_sha256: None,
    };
    let bundle = PreparedBundle::new(binding, record).unwrap();
    let file = dir.join("bundle.json");
    fs::write(&file, serde_json::to_vec_pretty(&bundle).unwrap()).unwrap();
    (file, bundle)
}
fn cli() -> Command {
    let mut c = Command::new(env!("CARGO_BIN_EXE_knx"));
    c.args(["device", "system2-validation"]);
    c
}
#[test]
fn system2_validation_cli_preview_is_offline_and_creates_no_protocol() {
    let dir = tempfile::tempdir().unwrap();
    let (path, b) = bundle(dir.path());
    let out = cli()
        .args(["--bundle", path.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let text = String::from_utf8(out.stdout).unwrap();
    assert!(text.contains("Preview only; no connection or writes."));
    assert!(text.contains("STRONG HYPOTHESIS — synthetic simulator candidate"));
    assert!(text.contains(&format!("Confirmation: {}", b.consent_phrase())));
    assert!(text.contains(&format!("Hypothesis acceptance: {}", b.acceptance_phrase())));
    assert!(text.contains("NOT raw allocator rollback or guaranteed restoration"));
    assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 1);
}
#[test]
fn system2_validation_cli_gateway_requires_exact_download_and_release_confirmations() {
    let dir = tempfile::tempdir().unwrap();
    let (path, b) = bundle(dir.path());
    let socket = UdpSocket::bind("127.0.0.1:0").unwrap();
    socket
        .set_read_timeout(Some(Duration::from_millis(100)))
        .unwrap();
    let gateway = socket.local_addr().unwrap().to_string();
    let target: knx_core::IndividualAddress = "1.1.25".parse().unwrap();
    let download = knx_core::commissioning::mutation::required_confirmation_phrase(
        target,
        knx_core::commissioning::mutation::WriteScope::Download,
    );
    let operation = dir.path().join("operation");
    let history = dir.path().join("activity.jsonl");
    // A missing release confirmation must refuse before any frame.
    let out = cli()
        .args([
            "--bundle",
            path.to_str().unwrap(),
            "--gateway",
            &gateway,
            "--operation",
            operation.to_str().unwrap(),
            "--confirm",
            &b.consent_phrase(),
            "--accept-hypothesis",
            &b.acceptance_phrase(),
            "--download-confirm",
            &download,
            "--activity-history",
            history.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("--release-confirm"));
    assert!(!operation.exists());
    assert!(socket.recv(&mut [0; 1024]).is_err());
    // A wrong release phrase must refuse before any frame.
    let out = cli()
        .args([
            "--bundle",
            path.to_str().unwrap(),
            "--gateway",
            &gateway,
            "--operation",
            operation.to_str().unwrap(),
            "--confirm",
            &b.consent_phrase(),
            "--accept-hypothesis",
            &b.acceptance_phrase(),
            "--download-confirm",
            &download,
            "--release-confirm",
            "wrong release",
            "--activity-history",
            history.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("best-effort release confirmation"));
    assert!(!operation.exists());
    assert!(socket.recv(&mut [0; 1024]).is_err());
}
#[test]
fn system2_validation_cli_simulator_cannot_take_gateway_or_skip_hypothesis() {
    let dir = tempfile::tempdir().unwrap();
    let (path, b) = bundle(dir.path());
    for flags in [
        vec!["--simulate", "--gateway", "127.0.0.1:9"],
        vec!["--simulate", "--confirm", &b.consent_phrase()],
    ] {
        let out = cli()
            .args(["--bundle", path.to_str().unwrap()])
            .args(flags)
            .output()
            .unwrap();
        assert!(!out.status.success());
    }
    assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 1);
}
#[test]
fn system2_validation_cli_simulator_writes_private_protocol_then_fresh_process_restore() {
    let dir = tempfile::tempdir().unwrap();
    let (path, b) = bundle(dir.path());
    let operation = dir.path().join("operation");
    let report = dir.path().join("shareable.json");
    let out = cli()
        .args([
            "--bundle",
            path.to_str().unwrap(),
            "--simulate",
            "--operation",
            operation.to_str().unwrap(),
            "--confirm",
            &b.consent_phrase(),
            "--accept-hypothesis",
            &b.acceptance_phrase(),
            "--report",
            report.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let report: serde_json::Value = serde_json::from_slice(&fs::read(report).unwrap()).unwrap();
    assert_eq!(report["outcome"], "finished");
    assert_eq!(report["simulated"], true);
    assert!(report.get("target").is_none());
    assert!(report.get("parameters").is_none());
    let out = cli()
        .args([
            "--bundle",
            path.to_str().unwrap(),
            "--simulate",
            "--recover",
            operation.to_str().unwrap(),
            "--confirm-recovery",
            &b.restore_phrase(),
            "--accept-hypothesis",
            &b.acceptance_phrase(),
        ])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let store = knx_app::property_validation::OperationStore::open(&operation, &b.binding).unwrap();
    assert_eq!(
        store.outcome(),
        knx_app::property_validation::LocalOutcome::Restored
    );
    assert!(store.recovery().is_some());
}
