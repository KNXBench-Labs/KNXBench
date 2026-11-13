//! Exercise consent-gated System-2 execution and recovery through fresh public TCP CLI processes.

mod admission;
mod recovery;
mod recovery_refusal;

use super::{
    probe::{
        collect,
        fixture::tree,
        peer::{gateway, no_retry},
    },
    service_control::{connected, exit},
    stderr, stdout,
    system2_project::{inputs, Inputs},
};
use knx_app::{
    property_validation::{LocalOutcome, OperationStore},
    property_validation_bundle::PreparedBundle,
};
use knx_net::{ApplicationService as A, LDataFrame};
use std::{
    fs,
    path::PathBuf,
    process::{Command, Output},
    sync::Arc,
};

type PrivateFiles = Vec<(String, Vec<u8>)>;

struct Fixture {
    inputs: Inputs,
    bundle: PreparedBundle,
    first_mutation_evidence: Arc<std::sync::Mutex<Option<PrivateFiles>>>,
}
impl Fixture {
    /// The real observation CLI binds independently seeded OLD, without changing device state.
    fn new() -> Self {
        let inputs = inputs();
        inputs.device.preset_memory(0x7000, &[0xd1, 0xe2, 0xf3]);
        let old = inputs.device.system2_snapshot();
        let device = Arc::clone(&inputs.device);
        let (addr, worker, trap) = gateway(250, move |p| p.simulate(&device, false, |_| {}));
        fs::create_dir(inputs.dir.path().join("out")).unwrap();
        let out = collect(&mut inputs.command(&addr.to_string(), Some("tcp")));
        worker.join().unwrap();
        exit(&out, 0);
        no_retry(&trap);
        assert_eq!(inputs.device.system2_snapshot(), old);
        let bundle = knx_app::property_validation::read_bundle(&inputs.output()).unwrap();
        assert_eq!(bundle.format, 3);
        assert!(!bundle.binding.simulated);
        Self {
            inputs,
            bundle,
            first_mutation_evidence: Arc::default(),
        }
    }
    fn operation(&self) -> PathBuf {
        self.inputs.dir.path().join("operation")
    }
    fn history(&self) -> PathBuf {
        self.inputs.dir.path().join("activity.sqlite")
    }
    fn command(&self, addr: &str, transport: Option<&str>, recover: bool) -> Command {
        let mut cmd = self.inputs.bare();
        cmd.args(["device", "system2-validation", "--bundle"])
            .arg(self.inputs.output())
            .args(["--gateway", addr])
            .arg(if recover { "--recover" } else { "--operation" })
            .arg(self.operation())
            .arg(if recover {
                "--confirm-recovery"
            } else {
                "--confirm"
            })
            .arg(if recover {
                self.bundle.restore_phrase()
            } else {
                self.bundle.consent_phrase()
            })
            .arg("--accept-hypothesis")
            .arg(self.bundle.acceptance_phrase())
            .arg("--download-confirm")
            .arg(
                knx_core::commissioning::mutation::required_confirmation_phrase(
                    self.inputs.device.address(),
                    knx_core::WriteScope::Download,
                ),
            )
            .arg("--release-confirm")
            .arg(
                knx_net::commissioning::hardware_release::required_release_phrase(
                    self.inputs.device.address(),
                ),
            )
            .arg("--activity-history")
            .arg(self.history());
        if !recover {
            cmd.arg("--project")
                .arg(&self.inputs.project)
                .args(["--device-id", "1", "--product-db"])
                .arg(&self.inputs.products);
        }
        if let Some(t) = transport {
            cmd.args(["--tunnel-transport", t]);
        }
        cmd
    }
    fn store(&self) -> OperationStore {
        OperationStore::open(&self.operation(), &self.bundle.binding).unwrap()
    }
    fn rows(&self) -> Vec<serde_json::Value> {
        let log = knx_app::commissioning_activity::OneShotLog::for_run(self.history()).unwrap();
        let (rows, more) = log.history_page(0, 100).unwrap();
        assert!(!more);
        rows.iter()
            .map(|r| {
                let value = serde_json::to_value(r).unwrap();
                assert_eq!(value["kind"], "system2Validation");
                assert_eq!(value["address"], "1.2.52");
                assert_eq!(value["interrupted"], false);
                assert!(value["finishedAt"].is_string());
                value
            })
            .collect()
    }
    fn run(&self, recover: bool, skips: bool, fault: bool) -> (Output, Vec<LDataFrame>) {
        let device = Arc::clone(&self.inputs.device);
        let operation = self.operation();
        let first_mutation_evidence = Arc::clone(&self.first_mutation_evidence);
        let (addr, worker, trap) = gateway(250, move |p| {
            let mut faulted = false;
            p.simulate(&device, skips, |frame| {
                if is_mutation(&frame.service) {
                    backup_before_write(&operation);
                    let mut detached = first_mutation_evidence.lock().unwrap();
                    if detached.is_none() {
                        *detached = Some(
                            tree(&operation)
                                .into_iter()
                                .filter(|(name, _)| name != "HEAD")
                                .collect(),
                        );
                    }
                }
                // One deliberate readback mismatch AFTER literal NEW reached this same device.
                if fault
                    && !faulted
                    && matches!(frame.service, A::MemoryRead { address: 15360, .. })
                    && device.memory(15360, 2) == [Some(1), Some(2)]
                {
                    device.preset_memory(15360, &[0xee, 0xdd]);
                    faulted = true;
                }
            });
            assert_eq!(faulted, fault);
        });
        let output = collect(&mut self.command(&addr.to_string(), Some("tcp"), recover));
        let wire = worker.join().unwrap();
        connected(&output, &addr.to_string(), skips);
        no_retry(&trap);
        let frames = wire
            .into_iter()
            .filter_map(|(outgoing, bytes)| {
                (outgoing && bytes[2..4] == [4, 0x20])
                    .then(|| knx_net::cemi::decode_l_data(&bytes[10..]).unwrap())
            })
            .collect();
        (output, frames)
    }
}

fn is_mutation(service: &A) -> bool {
    matches!(
        service,
        A::PropertyValueWrite { .. } | A::MemoryWrite { .. }
    )
}
fn backup_before_write(path: &std::path::Path) {
    let records: Vec<serde_json::Value> = tree(path)
        .into_iter()
        .filter(|(name, _)| name.starts_with("checkpoint-") && name.ends_with(".json"))
        .map(|(_, data)| serde_json::from_slice(&data).unwrap())
        .collect();
    let backup = records
        .iter()
        .find(|v| v["checkpoint"]["event"] == "recovery_acquired")
        .expect("durable complete OLD backup BEFORE every mutation, including setup");
    let allocations = backup["checkpoint"]["recovery"]["allocations"]
        .as_array()
        .unwrap();
    assert_eq!(
        allocations
            .iter()
            .map(|a| a["data"].as_array().unwrap().len())
            .sum::<usize>(),
        48
    );
    assert!(allocations
        .iter()
        .all(|a| a["data"].as_array().unwrap().iter().all(|b| b == 0xa5)));
}
fn literal_new(frames: &[LDataFrame]) {
    let writes: Vec<_> = frames
        .iter()
        .filter_map(|frame| match &frame.service {
            A::MemoryWrite { address, data } => Some((*address, data.clone())),
            _ => None,
        })
        .collect();
    assert!(
        writes.contains(&(15360, vec![1, 2])),
        "literal configured parameter NEW: {writes:?}"
    );
    assert!(
        writes.contains(&(278, vec![3, 0x12, 0x34, 0x10, 0x35, 0x10, 0x36])),
        "literal table NEW: {writes:?}"
    );
    assert!(
        writes.contains(&(1024, vec![2, 0xa7, 0x12, 0x21, 0, 0x34, 0xf7, 7])),
        "literal flags NEW: {writes:?}"
    );
}

#[test]
fn tcp_execute_completes_literal_new_with_backup_and_optional_skips_without_recording() {
    for skips in [false, true] {
        let f = Fixture::new();
        let old = f.inputs.device.system2_snapshot();
        let (out, frames) = f.run(false, skips, false);
        exit(&out, 0);
        assert!(stdout(&out).contains("HARDWARE outcome: Finished"));
        literal_new(&frames);
        assert_ne!(f.inputs.device.system2_snapshot(), old);
        assert_eq!(
            f.inputs.device.memory(15360, 3),
            [Some(1), Some(2), Some(0xa5)]
        );
        assert_eq!(f.store().outcome(), LocalOutcome::Finished);
        assert!(f.store().recovery().is_some());
        assert_eq!(f.rows()[0]["state"], "verified");
        assert_eq!(
            f.rows()[0]["writeEvidence"],
            serde_json::json!({"backupRecorded": true, "sendPossible": true})
        );
    }
}

#[test]
fn tcp_skip_then_eof_discloses_actual_transport_and_fails_without_mutation_or_retry() {
    let f = Fixture::new();
    let old = f.inputs.device.system2_snapshot();
    let (addr, worker, trap) = gateway(250, |p| {
        p.target("1.2.52");
        p.begin(false);
        p.descriptor_request();
        p.skips();
        p.close_write();
    });
    let out = collect(&mut f.command(&addr.to_string(), Some("tcp"), false));
    worker.join().unwrap();
    exit(&out, 1);
    connected(&out, &addr.to_string(), true);
    no_retry(&trap);
    assert_eq!(f.inputs.device.system2_snapshot(), old);
    assert!(f.store().recovery().is_none());
    assert!(!f.store().mutation_possible());
    assert_eq!(f.rows()[0]["state"], "notSent");
    assert!(!stdout(&out).contains("HARDWARE outcome: Finished"));
}
