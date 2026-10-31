//! Literal eight-byte oracles cross App execution, private journal and fresh-disk replay.
use super::*;
use knx_core::commissioning::mutation::{required_confirmation_phrase, TargetKind};
use knx_core::IndividualAddress;
use knx_net::{
    cemi::{ApplicationService, Destination, Tpci},
    client::{BusError, TunnelEvent},
    management::ManagementTransport,
};
use serde_json::{json, Value};
use std::sync::atomic::Ordering;
use tokio::sync::broadcast;

/// The Net evidence fixture, independently specified here: allocation 3C00h..3C08h,
/// old bytes 09 08 07 06 05 04 03 02, write 01 02 03 04 at 3C02h (offsets 2..6).
async fn eight_bytes() -> (SimulatedDevice, PreparedBundle) {
    let (device, base) = fixture();
    let plan = base.plan().unwrap();
    let mut steps = plan.steps().to_vec();
    steps[4].operation = Op::LoadControl {
        object: 3,
        payload: absolute_data(AbsolutePropertySegment {
            start: 0x3c00,
            length: 8,
            access: 0xff,
            memory_type: SegmentMemoryType::Eeprom,
            checksum_disabled: true,
        })
        .unwrap(),
    };
    steps[5].operation = Op::WriteMemory {
        object: 3,
        address: 0x3c02,
        data: vec![1, 2, 3, 4],
    };
    let plan =
        PropertyDownloadPlan::new(plan.target(), plan.mask(), plan.manufacturer(), steps).unwrap();
    let record = PlanRecord::from_plan(&plan);
    let mut binding = base.binding;
    binding.plan_sha256 = record.sha256().unwrap();
    binding.profile_id = "synthetic-evidence-defaults-v3".into();
    binding.profile_version = 3;
    let bundle = PreparedBundle::new(binding, record).unwrap();
    establish(&device, &bundle).await;
    device.preset_memory(0x3c00, &[9, 8, 7, 6, 5, 4, 3, 2]);
    (device, bundle.rebind_hardware(SERIAL).unwrap())
}

/// No sockets: hardware-kind App entry over the persistent simulator. Corrupt only
/// on a final read, after LoadCompleted, so all individual write readbacks succeed.
struct FinalRead<'a> {
    device: &'a SimulatedDevice,
    corrupt_offset: Option<u16>,
    short_prestate: bool,
    written: AtomicBool,
    final_phase: AtomicBool,
}
impl<'a> FinalRead<'a> {
    fn new(device: &'a SimulatedDevice, offset: Option<u16>, short_prestate: bool) -> Self {
        Self {
            device,
            corrupt_offset: offset,
            short_prestate,
            written: AtomicBool::new(false),
            final_phase: AtomicBool::new(false),
        }
    }
}
impl ManagementTransport for FinalRead<'_> {
    fn assigned_address(&self) -> IndividualAddress {
        ManagementTransport::assigned_address(self.device)
    }
    fn subscribe(&self) -> broadcast::Receiver<TunnelEvent> {
        ManagementTransport::subscribe(self.device)
    }
    fn target_kind(&self) -> TargetKind {
        TargetKind::Hardware
    }
    async fn send_frame(
        &self,
        destination: Destination,
        transport: Tpci,
        mut service: ApplicationService,
    ) -> Result<(), BusError> {
        match &mut service {
            ApplicationService::PropertyValueWrite {
                property_id, data, ..
            } => {
                self.written.store(true, Ordering::SeqCst);
                if *property_id == 5 && data.first() == Some(&2) {
                    self.final_phase.store(true, Ordering::SeqCst);
                }
            }
            ApplicationService::MemoryRead { number, .. } => {
                if self.short_prestate && !self.written.load(Ordering::SeqCst) && *number > 1 {
                    *number -= 1;
                }
                if self.final_phase.load(Ordering::SeqCst) {
                    if let Some(offset) = self.corrupt_offset {
                        self.device
                            .preset_memory(u32::from(0x3c00 + offset), &[0xee]);
                    }
                }
            }
            _ => {}
        }
        ManagementTransport::send_frame(self.device, destination, transport, service).await
    }
}

fn phrases(device: &SimulatedDevice) -> (String, String) {
    (
        required_confirmation_phrase(device.address(), WriteScope::Download),
        knx_net::commissioning::hardware_release::required_release_phrase(device.address()),
    )
}
fn last_checkpoint(path: &std::path::Path) -> Value {
    let mut files: Vec<_> = std::fs::read_dir(path)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| {
            p.file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with("checkpoint-")
        })
        .collect();
    files.sort();
    serde_json::from_slice(&std::fs::read(files.last().unwrap()).unwrap()).unwrap()
}
/// All expected data supplied here are literals from the fixture, never the returned view.
fn assert_literal(store: &OperationStore, path: &std::path::Path, expected_entries: Value) {
    assert_eq!(
        serde_json::to_value(store.allocation_mismatch_evidence()).unwrap(),
        expected_entries
    );
    let entry = expected_entries.as_array().unwrap().last().unwrap();
    let e = &entry["evidence"];
    let envelope = last_checkpoint(path);
    assert_eq!(envelope["format"], 4);
    assert_eq!(
        envelope["checkpoint"]["event"],
        if entry["run"] == "restore" {
            "restore_failed"
        } else {
            "failed"
        }
    );
    let differing: Vec<_> = e["differingRanges"].as_array().unwrap().iter().map(|r|
        json!({"offset": r["offset"], "length": r["length"], "classification": r["classification"]})).collect();
    assert_eq!(
        envelope["checkpoint"]["allocation_evidence"],
        json!({
            "allocation_index": 0, "object": 3, "address": 15360, "length": 8,
            "prestate_unreadable": e["prestateUnreadable"], "expected": e["expected"], "actual": e["actual"],
            "written": [{"offset": 2, "length": 4}], "differing": differing
        })
    );
}
fn literal(run: &str, unreadable: bool, expected: &str, actual: &str, ranges: Value) -> Value {
    json!({"run": run, "evidence": {"availability": "recorded", "allocationIndex": 0,
        "object": 3, "address": 15360, "length": 8, "prestateUnreadable": unreadable,
        "expected": expected, "actual": actual, "written": [{"offset": 2, "length": 4}],
        "differingRanges": ranges}})
}
async fn execute(
    transport: &FinalRead<'_>,
    bundle: &PreparedBundle,
    store: &mut OperationStore,
) -> std::io::Result<()> {
    let (download, release) = phrases(transport.device);
    app::execute_hardware(
        transport,
        bundle,
        store,
        timing(),
        &download,
        &release,
        &AtomicBool::new(false),
    )
    .await
}

#[tokio::test]
async fn literal_unwritten_bytes_survive_app_journal_and_reopen() {
    let (device, bundle) = eight_bytes().await;
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("op");
    let mut store = OperationStore::create(&path, bundle.binding.clone()).unwrap();
    execute(
        &FinalRead::new(&device, Some(6), false),
        &bundle,
        &mut store,
    )
    .await
    .unwrap_err();
    assert_eq!(store.outcome(), LocalOutcome::Failed);
    let expected = json!([literal(
        "download",
        false,
        "0908010203040302",
        "090801020304ee02",
        json!([
            {"offset": 6, "length": 1, "classification": "unwritten", "expected": "03", "actual": "ee"}
        ])
    )]);
    assert_literal(&store, &path, expected.clone());
    drop(store);
    let fresh = OperationStore::open(&path, &bundle.binding).unwrap();
    assert_eq!(fresh.outcome(), LocalOutcome::Failed);
    assert_literal(&fresh, &path, expected);
}

#[tokio::test]
async fn literal_unreadable_prestate_classes_survive_app_journal_and_reopen() {
    let (device, bundle) = eight_bytes().await;
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("op");
    let mut store = OperationStore::create(&path, bundle.binding.clone()).unwrap();
    execute(&FinalRead::new(&device, Some(3), true), &bundle, &mut store)
        .await
        .unwrap_err();
    assert_eq!(store.outcome(), LocalOutcome::Failed);
    assert_eq!(
        store.recovery().unwrap().prestate_unreadable_allocations,
        [0]
    );
    let expected = json!([literal(
        "download",
        true,
        "0000010203040000",
        "090801ee03040302",
        json!([
            {"offset": 0, "length": 2, "classification": "unwrittenUnpredicted", "expected": "0000", "actual": "0908"},
            {"offset": 3, "length": 1, "classification": "written", "expected": "02", "actual": "ee"},
            {"offset": 6, "length": 2, "classification": "unwrittenUnpredicted", "expected": "0000", "actual": "0302"}
        ])
    )]);
    assert_literal(&store, &path, expected.clone());
    drop(store);
    let fresh = OperationStore::open(&path, &bundle.binding).unwrap();
    assert_eq!(fresh.outcome(), LocalOutcome::Failed);
    assert_literal(&fresh, &path, expected);
}

#[tokio::test]
async fn literal_placeholder_only_differences_remain_success_after_reopen() {
    let (device, bundle) = eight_bytes().await;
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("op");
    let mut store = OperationStore::create(&path, bundle.binding.clone()).unwrap();
    execute(&FinalRead::new(&device, None, true), &bundle, &mut store)
        .await
        .unwrap();
    assert_eq!(store.outcome(), LocalOutcome::Finished);
    assert_eq!(
        store.recovery().unwrap().prestate_unreadable_allocations,
        [0]
    );
    assert_eq!(
        device.memory(0x3c00, 8),
        [
            Some(9),
            Some(8),
            Some(1),
            Some(2),
            Some(3),
            Some(4),
            Some(3),
            Some(2)
        ]
    );
    assert_eq!(store.allocation_mismatch_evidence(), []);
    assert_eq!(
        last_checkpoint(&path)["checkpoint"],
        json!({"event": "finished"})
    );
    drop(store);
    let fresh = OperationStore::open(&path, &bundle.binding).unwrap();
    assert_eq!(fresh.outcome(), LocalOutcome::Finished);
    assert_eq!(fresh.allocation_mismatch_evidence(), []);
    assert_eq!(
        fresh.recovery().unwrap().prestate_unreadable_allocations,
        [0]
    );
}

#[tokio::test]
async fn literal_restore_comparison_uses_original_captured_bytes_after_reopen() {
    let (device, bundle) = eight_bytes().await;
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("op");
    let mut store = OperationStore::create(&path, bundle.binding.clone()).unwrap();
    execute(
        &FinalRead::new(&device, Some(3), false),
        &bundle,
        &mut store,
    )
    .await
    .unwrap_err();
    assert_eq!(
        store.recovery().unwrap().allocations[0].data,
        [9, 8, 7, 6, 5, 4, 3, 2]
    );
    let download = literal(
        "download",
        false,
        "0908010203040302",
        "090801ee03040302",
        json!([
            {"offset": 3, "length": 1, "classification": "written", "expected": "02", "actual": "ee"}
        ]),
    );
    assert_literal(&store, &path, json!([download]));
    drop(store);
    let mut fresh = OperationStore::open(&path, &bundle.binding).unwrap();
    let (download_phrase, release) = phrases(&device);
    app::restore_hardware(
        &FinalRead::new(&device, Some(3), false),
        &bundle,
        &mut fresh,
        timing(),
        &bundle.restore_phrase(),
        &download_phrase,
        &release,
        &AtomicBool::new(false),
    )
    .await
    .unwrap_err();
    assert_eq!(fresh.outcome(), LocalOutcome::RestoreFailed);
    let restore = literal(
        "restore",
        false,
        "0908070605040302",
        "090807ee05040302",
        json!([
            {"offset": 3, "length": 1, "classification": "written", "expected": "06", "actual": "ee"}
        ]),
    );
    assert_literal(&fresh, &path, json!([download, restore]));
    drop(fresh);
    let reopened = OperationStore::open(&path, &bundle.binding).unwrap();
    assert_eq!(reopened.outcome(), LocalOutcome::RestoreFailed);
    assert_literal(&reopened, &path, json!([download, restore]));
}
