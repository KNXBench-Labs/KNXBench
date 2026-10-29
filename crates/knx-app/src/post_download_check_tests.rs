//! Sanitized post-download check records from real simulator observations and stored JSON.

use std::time::Duration;

use knx_core::commissioning::authorisation::AuthorisationPlan;
use knx_core::commissioning::load_control_memory::{
    abs_data_segment, abs_task_segment, event_record, AbsoluteSegment, MemoryLoadStateMachine,
    SegmentMemoryType, TaskSegment,
};
use knx_core::commissioning::load_state::{LoadEvent, LoadState, MaskVersion};
use knx_core::commissioning::memory_download::{MemoryDownloadPlan, MemoryDownloadStep};
use knx_core::commissioning::mutation::{WriteAuthorisation, WriteScope};
use knx_core::commissioning::partial_memory_download::{derive_partial_plan, PartialDownloadParts};
use knx_core::commissioning::properties::{
    ObjectIndex, PID_HARDWARE_TYPE, PID_MANUFACTURER_ID, PID_PROGRAM_VERSION,
};
use knx_core::ContactableAddress;
use knx_net::commissioning::memory_download::run_memory_download;
use knx_net::commissioning::post_download::observe_after_download;
use knx_net::commissioning::simulator::{SimulatedDevice, SimulatorConfig};
use knx_net::commissioning::{ManagementSession, SessionTiming};
use serde_json::{json, Value};

use super::*;
use MemoryLoadStateMachine::{AddressTable, ApplicationProgram, AssociationTable};

#[path = "post_download_check/exact0700_tests.rs"]
mod exact0700;
#[path = "post_download_check/run_state_tests.rs"]
mod run_state;

const HARDWARE: [u8; 6] = [0, 0, 0, 0, 0x01, 0x27];
const APPLICATION: [u8; 5] = [0x00, 0x83, 0x00, 0x27, 0x15];

fn fast() -> SessionTiming {
    SessionTiming {
        connection_timeout: Duration::from_millis(50),
        response_timeout: Duration::from_millis(50),
        answer_timeout: None,
        poll_interval: Duration::from_millis(1),
        max_transition: Duration::from_millis(40),
        programming_delay: Duration::from_millis(0),
        restart_basic_t1: Duration::from_millis(1),
        restart_responsive_again: Duration::from_millis(5),
        post_restart_disconnect_wait: Duration::from_millis(5),
        programming_mode_broadcast_timeout: Duration::from_millis(20),
    }
}

/// Every machine `Unloaded` before the transfer. Fixture assumption, stated:
/// the device already reports `application` as its program version; the
/// simulator does not derive it from the task segment the plan writes.
fn device(application: [u8; 5], config: SimulatorConfig) -> SimulatedDevice {
    let device = SimulatedDevice::with_config(SimulatorConfig {
        mask_version: 0x0701,
        ..config
    });
    device.preset_property(0, PID_MANUFACTURER_ID, &[0x00, 0x83]);
    device.preset_property(0, PID_HARDWARE_TYPE, &HARDWARE);
    device.preset_property(3, PID_PROGRAM_VERSION, &application);
    for machine in MemoryLoadStateMachine::ALL {
        device.preset_load_state(ObjectIndex::new(machine.type_number()), LoadState::Unloaded);
    }
    device
}

fn record(machine: MemoryLoadStateMachine, event: LoadEvent) -> MemoryDownloadStep {
    MemoryDownloadStep::LoadRecord(event_record(machine, event).expect("an event record"))
}

fn segment(start: u16, length: u16) -> AbsoluteSegment {
    AbsoluteSegment {
        start,
        length,
        access: 0xFF,
        memory_type: SegmentMemoryType::Eeprom,
        checksum_control: true,
    }
}

/// Address table and application, a hardware comparison and the
/// application identity in the task segment, closed by a restart.
fn plan() -> MemoryDownloadPlan {
    MemoryDownloadPlan {
        mask: MaskVersion(0x0701),
        manufacturer: 0x0083,
        steps: vec![
            MemoryDownloadStep::Connect,
            MemoryDownloadStep::CompareProperty {
                object_index: 0,
                property_id: PID_HARDWARE_TYPE,
                inline_data: [&HARDWARE[..], &[0, 0, 0, 0]].concat(),
            },
            record(AddressTable, LoadEvent::Unload),
            record(AssociationTable, LoadEvent::Unload),
            record(ApplicationProgram, LoadEvent::Unload),
            record(AddressTable, LoadEvent::StartLoading),
            MemoryDownloadStep::LoadRecord(
                abs_data_segment(AddressTable, segment(0x4000, 4)).expect("a segment"),
            ),
            MemoryDownloadStep::WriteMemory {
                address: 0x4000,
                octets: vec![0x01, 0x11, 0x43, 0x00],
            },
            record(AddressTable, LoadEvent::LoadCompleted),
            record(ApplicationProgram, LoadEvent::StartLoading),
            MemoryDownloadStep::LoadRecord(
                abs_data_segment(ApplicationProgram, segment(0x4400, 8)).expect("a segment"),
            ),
            MemoryDownloadStep::WriteMemory {
                address: 0x4400,
                octets: vec![0xA5; 8],
            },
            MemoryDownloadStep::LoadRecord(abs_task_segment(
                ApplicationProgram,
                TaskSegment {
                    start: 0x4400,
                    pei_type: 1,
                    manufacturer: 0x0083,
                    application: 0x0027,
                    version: 0x15,
                },
            )),
            record(ApplicationProgram, LoadEvent::LoadCompleted),
            MemoryDownloadStep::Restart,
            MemoryDownloadStep::Disconnect,
        ],
    }
}

async fn transfer_then_check(device: &SimulatedDevice, plan: &MemoryDownloadPlan) -> Value {
    let authorisation =
        WriteAuthorisation::for_simulator(device.address(), WriteScope::Download).unwrap();
    let mut session =
        ManagementSession::authorised(device, AuthorisationPlan::Skip, fast(), authorisation)
            .unwrap();
    run_memory_download(&mut session, plan)
        .await
        .expect("the transfer runs");
    let target = ContactableAddress::new(device.address()).unwrap();
    let observation = observe_after_download(device, target, fast(), plan).await;
    let check = PostDownloadCheck::from_observation(&observation);
    let value = serde_json::to_value(&check).unwrap();
    let back: PostDownloadCheck = serde_json::from_value(value.clone()).unwrap();
    assert_eq!(back, check, "a stored record reads back unchanged");
    value
}

fn machine(machine: &str, expected: Value, observed: Value) -> Value {
    json!({ "machine": machine, "expected": expected, "observed": observed })
}

fn loaded() -> Value {
    json!({ "result": "observed", "loadState": "loaded" })
}

fn unloaded() -> Value {
    json!({ "result": "observed", "loadState": "unloaded" })
}

fn matched(load_states: Vec<Value>, verdict: &str) -> Value {
    json!({
        "observation": { "status": "attempted", "reach": { "result": "reached",
            "identity": { "result": "match", "exemplar": "notVerified",
                "loadStates": load_states, "planLoadVerdict": verdict } } },
        "runtime": "unknown"
    })
}

fn matched_with_run_state(load_states: Vec<Value>, verdict: &str, run_state: Value) -> Value {
    let mut value = matched(load_states, verdict);
    value["observation"]["reach"]["identity"]["runState"] = run_state;
    value
}

/// [`matched`] as the producer writes it for a fixture device with no
/// object-type property: the continuation's first read is refused.
fn matched_without_object_type(load_states: Vec<Value>, verdict: &str) -> Value {
    let mut value = matched(load_states, verdict);
    value["observation"]["reach"]["identity"]["runState"] =
        json!({ "status": "unavailable", "step": "objectType", "cause": "refused" });
    value
}

fn reached(identity: Value) -> Value {
    json!({
        "observation": { "status": "attempted",
            "reach": { "result": "reached", "identity": identity } },
        "runtime": "unknown"
    })
}

#[tokio::test]
async fn a_complete_transfer_is_checked_with_expected_next_to_observed() {
    let device = device(APPLICATION, SimulatorConfig::default());
    let value = transfer_then_check(&device, &plan()).await;
    assert_eq!(
        value,
        matched_without_object_type(
            vec![
                machine("addressTable", json!("loaded"), loaded()),
                machine("associationTable", json!("unloaded"), unloaded()),
                machine("applicationProgram", json!("loaded"), loaded()),
            ],
            "matchesPlan"
        )
    );
}

#[tokio::test]
async fn a_partial_transfer_is_checked_against_the_machine_it_only_requires() {
    let device = device(APPLICATION, SimulatorConfig::default());
    // The complete transfer first, as a partial download needs it loaded.
    transfer_then_check(&device, &plan()).await;
    let partial = derive_partial_plan(
        &plan(),
        PartialDownloadParts {
            parameters: true,
            group_addresses: false,
        },
    )
    .unwrap()
    .plan;
    let value = transfer_then_check(&device, &partial).await;
    let identity = &value["observation"]["reach"]["identity"];
    assert_eq!(identity["result"], "match", "{value}");
    assert_eq!(identity["planLoadVerdict"], "matchesPlan", "{value}");
    let machines: Vec<&str> = identity["loadStates"]
        .as_array()
        .unwrap()
        .iter()
        .map(|reading| reading["machine"].as_str().unwrap())
        .collect();
    assert!(machines.contains(&"applicationProgram"), "{value}");
}

#[tokio::test]
async fn another_application_after_the_transfer_is_a_mismatch_without_its_octets() {
    let device = device([0x00, 0x83, 0x00, 0x27, 0x16], SimulatorConfig::default());
    let value = transfer_then_check(&device, &plan()).await;
    assert_eq!(
        value,
        reached(
            json!({ "result": "mismatch", "part": { "kind": "application" }, "kind": "differs" })
        )
    );
}

#[tokio::test]
async fn a_later_failed_read_keeps_the_earlier_reading() {
    let device = device(APPLICATION, SimulatorConfig::default());
    let authorisation =
        WriteAuthorisation::for_simulator(device.address(), WriteScope::Download).unwrap();
    let mut session =
        ManagementSession::authorised(&device, AuthorisationPlan::Skip, fast(), authorisation)
            .unwrap();
    run_memory_download(&mut session, &plan()).await.unwrap();
    let observation = observe_after_download(
        &FailSecondLoadRead::new(&device),
        ContactableAddress::new(device.address()).unwrap(),
        fast(),
        &plan(),
    )
    .await;
    let value = serde_json::to_value(PostDownloadCheck::from_observation(&observation)).unwrap();
    assert_eq!(
        value,
        matched_with_run_state(
            vec![
                machine("addressTable", json!("loaded"), loaded()),
                machine(
                    "associationTable",
                    json!("unloaded"),
                    json!({ "result": "unavailable" })
                ),
                machine(
                    "applicationProgram",
                    json!("loaded"),
                    json!({ "result": "notRead" })
                ),
            ],
            "unknown",
            json!({ "status": "notRead", "reason": "loadReadFailed" })
        )
    );
}

/// Passes frames to the device but refuses to send the second load-state
/// read: the first stays read, the second unavailable, the third unread.
struct FailSecondLoadRead<'a> {
    device: &'a SimulatedDevice,
    load_reads: std::sync::Mutex<u32>,
}

impl<'a> FailSecondLoadRead<'a> {
    fn new(device: &'a SimulatedDevice) -> Self {
        Self {
            device,
            load_reads: std::sync::Mutex::new(0),
        }
    }
}

impl knx_net::ManagementTransport for FailSecondLoadRead<'_> {
    fn assigned_address(&self) -> knx_core::IndividualAddress {
        self.device.assigned_address()
    }

    fn subscribe(&self) -> tokio::sync::broadcast::Receiver<knx_net::TunnelEvent> {
        self.device.subscribe()
    }

    async fn send_frame(
        &self,
        destination: knx_net::Destination,
        transport: knx_net::Tpci,
        service: knx_net::ApplicationService,
    ) -> Result<(), knx_net::BusError> {
        if let knx_net::ApplicationService::MemoryRead { address, .. } = &service {
            if (0xB6EA..=0xB6ED).contains(address) {
                let mut reads = self.load_reads.lock().unwrap();
                *reads += 1;
                if *reads == 2 {
                    return Err(knx_net::BusError::Timeout);
                }
            }
        }
        self.device
            .send_frame(destination, transport, service)
            .await
    }

    fn target_kind(&self) -> knx_core::commissioning::mutation::TargetKind {
        self.device.target_kind()
    }
}

#[tokio::test]
async fn a_silent_device_is_not_reached_and_an_unsupported_plan_sends_nothing() {
    let device = device(
        APPLICATION,
        SimulatorConfig {
            silent: true,
            ..SimulatorConfig::default()
        },
    );
    let target = ContactableAddress::new(device.address()).unwrap();
    let observation = observe_after_download(&device, target, fast(), &plan()).await;
    assert_eq!(
        serde_json::to_value(PostDownloadCheck::from_observation(&observation)).unwrap(),
        json!({ "observation": { "status": "attempted", "reach": { "result": "notReached" } },
            "runtime": "unknown" })
    );
    let mut unsupported = plan();
    unsupported.mask = MaskVersion(0x0805);
    let observation = observe_after_download(&device, target, fast(), &unsupported).await;
    assert_eq!(
        serde_json::to_value(PostDownloadCheck::from_observation(&observation)).unwrap(),
        json!({ "observation": { "status": "unsupported", "reason": "unsupportedMask" },
            "runtime": "unknown" })
    );
}

#[test]
fn absent_and_explicit_records_never_read_as_checked() {
    assert_eq!(
        serde_json::to_value(PostDownloadCheck::not_checked()).unwrap(),
        json!({ "observation": { "status": "notChecked" }, "runtime": "unknown" })
    );
    assert!(!PostDownloadCheck::not_checked().is_checked());
    assert!(PostDownloadCheck::interrupted().is_checked());
    assert_eq!(PostDownloadCheck::not_checked().line(), "not checked");
    assert!(PostDownloadCheck::interrupted()
        .lines()
        .contains(&"application running: unknown (not observed)".to_string()));
}

#[test]
fn stored_records_that_contradict_themselves_are_refused() {
    let refused = |value: Value| serde_json::from_value::<PostDownloadCheck>(value).is_err();
    // A match whose machine differs.
    assert!(refused(matched(
        vec![machine("addressTable", json!("loaded"), unloaded())],
        "matchesPlan"
    )));
    // A difference with nothing that differs.
    assert!(refused(matched(
        vec![machine("addressTable", json!("loaded"), loaded())],
        "differsFromPlan"
    )));
    // Unknown although every machine agrees.
    assert!(refused(matched(
        vec![machine("addressTable", json!("loaded"), loaded())],
        "unknown"
    )));
    // Unread without an earlier failure, and read after one.
    assert!(refused(matched(
        vec![machine(
            "addressTable",
            json!("loaded"),
            json!({ "result": "notRead" })
        )],
        "unknown"
    )));
    assert!(refused(matched(
        vec![
            machine(
                "addressTable",
                json!("loaded"),
                json!({ "result": "unavailable" })
            ),
            machine("applicationProgram", json!("loaded"), loaded()),
        ],
        "unknown"
    )));
    // Machines out of order.
    assert!(refused(matched(
        vec![
            machine("applicationProgram", json!("loaded"), loaded()),
            machine("addressTable", json!("loaded"), loaded()),
        ],
        "matchesPlan"
    )));
    // A runtime claim, a raw identity or an unknown status.
    let mut running = matched(vec![], "unknown");
    running["runtime"] = json!("running");
    assert!(refused(running));
    let mut raw = matched(vec![], "unknown");
    raw["observation"]["reach"]["identity"]["application"] = json!("0083002715");
    assert!(refused(raw));
    assert!(refused(
        json!({ "observation": { "status": "matched" }, "runtime": "unknown" })
    ));
    // A reach that is not reached cannot carry an identity.
    assert!(refused(json!({ "observation": { "status": "attempted",
        "reach": { "result": "notReached", "identity": { "result": "unavailable",
            "part": { "kind": "mask" } } } }, "runtime": "unknown" })));
    // What it accepts.
    assert!(!refused(matched(
        vec![
            machine("addressTable", json!("loaded"), loaded()),
            machine("applicationProgram", json!(null), loaded()),
        ],
        "unknown"
    )));
}

/// Each web fixture is accepted and re-serialized unchanged; that is all it proves
/// (the fixtures are written by hand, not captured from a run).
#[test]
fn the_web_fixtures_are_the_serializer_vocabulary() {
    let fixtures: Value = serde_json::from_str(include_str!(
        "../../../apps/knx-web/src/postDownloadCheck.fixtures.json"
    ))
    .unwrap();
    let fixtures = fixtures.as_object().unwrap();
    for (name, value) in fixtures {
        let check: PostDownloadCheck = serde_json::from_value(value.clone())
            .unwrap_or_else(|error| panic!("fixture {name}: {error}"));
        assert_eq!(
            &serde_json::to_value(&check).unwrap(),
            value,
            "fixture {name}"
        );
    }
    let names: Vec<&str> = fixtures.keys().map(String::as_str).collect();
    for required in [
        "notChecked",
        "unsupported",
        "notPerformed",
        "interrupted",
        "notReached",
        "reachUnknown",
        "identityUnavailable",
        "identityMismatch",
        "matched",
        "partialReading",
        "differsFromPlan",
    ] {
        assert!(names.contains(&required), "fixture {required} missing");
    }
}

/// One table case's `[pointer, value]` edits applied to its named fixture.
fn edited(fixtures: &Value, case: &Value) -> Value {
    let mut value = fixtures[case["fixture"].as_str().unwrap()].clone();
    assert!(value.is_object(), "fixture {}", case["fixture"]);
    for edit in case["set"].as_array().unwrap() {
        let pointer = edit[0].as_str().unwrap();
        let (parent, key) = pointer.rsplit_once('/').unwrap();
        match value.pointer_mut(parent) {
            Some(Value::Object(object)) => {
                object.insert(key.into(), edit[1].clone());
            }
            Some(Value::Array(array)) => array[key.parse::<usize>().unwrap()] = edit[1].clone(),
            _ => panic!("case {}: no parent for {pointer}", case["name"]),
        }
    }
    value
}

/// The table the web reader runs too: each case is a real fixture with one edit.
#[test]
fn the_shared_case_table_is_admitted_or_refused_exactly() {
    let fixtures: Value = serde_json::from_str(include_str!(
        "../../../apps/knx-web/src/postDownloadCheck.fixtures.json"
    ))
    .unwrap();
    let cases: Value = serde_json::from_str(include_str!(
        "../../../apps/knx-web/src/postDownloadCheck.cases.json"
    ))
    .unwrap();
    for case in cases["accepted"].as_array().unwrap() {
        let value = edited(&fixtures, case);
        let check: PostDownloadCheck = serde_json::from_value(value.clone())
            .unwrap_or_else(|error| panic!("case {}: {error}", case["name"]));
        assert_eq!(
            serde_json::to_value(&check).unwrap(),
            value,
            "case {}",
            case["name"]
        );
    }
    let refused = cases["refused"].as_array().unwrap();
    assert!(refused.len() >= 40, "the table stays broad");
    for case in refused {
        let value = edited(&fixtures, case);
        assert!(
            serde_json::from_value::<PostDownloadCheck>(value).is_err(),
            "case {} was admitted",
            case["name"]
        );
    }
}
