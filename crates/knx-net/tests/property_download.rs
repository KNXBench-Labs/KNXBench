//! Property execution exercises management sessions without accepting hardware transports.

use std::time::Duration;

use knx_core::commissioning::absolute_load_control::{absolute_data, AbsolutePropertySegment};
use knx_core::commissioning::authorisation::AuthorisationPlan;
use knx_core::commissioning::load_control::{event_payload, task_control_2, TaskControl2};
use knx_core::commissioning::load_control_memory::SegmentMemoryType;
use knx_core::commissioning::load_state::{LoadEvent, LoadState, MaskVersion};
use knx_core::commissioning::mutation::{WriteAuthorisation, WriteScope};
use knx_core::commissioning::properties::{
    ObjectIndex, PID_LOAD_STATE_CONTROL, PID_TABLE_REFERENCE,
};
use knx_core::commissioning::property_download::{
    PropertyDownloadPlan, PropertyDownloadStep, PropertyOperation as Op,
};
use knx_net::commissioning::property_download::run_simulated_property_plan;
use knx_net::commissioning::simulator::{Seen, SimulatedDevice, SimulatorConfig};
use knx_net::commissioning::{ManagementSession, SessionTiming};

const HARDWARE: [u8; 6] = [0x12, 0x34, 0x56, 0x78, 0x9A, 0xBC];

fn timing() -> SessionTiming {
    SessionTiming {
        connection_timeout: Duration::from_millis(20),
        response_timeout: Duration::from_millis(5),
        poll_interval: Duration::from_millis(1),
        max_transition: Duration::from_millis(30),
        programming_delay: Duration::ZERO,
        ..SessionTiming::default()
    }
}

fn fixture(config: SimulatorConfig) -> (SimulatedDevice, PropertyDownloadPlan) {
    let device = SimulatedDevice::with_config(config);
    device.preset_property(0, 78, &HARDWARE);
    let step = |source_index, operation| PropertyDownloadStep {
        source_index,
        operation,
    };
    let control = |source_index, event| {
        step(
            source_index,
            Op::LoadControl {
                object: 3,
                payload: event_payload(event),
            },
        )
    };
    let plan = PropertyDownloadPlan::new(
        device.address(),
        MaskVersion(0x0025),
        2,
        vec![
            step(0, Op::Connect),
            step(1, Op::HardwareCheck { expected: HARDWARE }),
            control(2, LoadEvent::Unload),
            control(3, LoadEvent::StartLoading),
            step(
                4,
                Op::LoadControl {
                    object: 3,
                    payload: absolute_data(AbsolutePropertySegment {
                        start: 0x3C00,
                        length: 4,
                        access: 0xFF,
                        memory_type: SegmentMemoryType::Eeprom,
                        checksum_disabled: true,
                    })
                    .unwrap(),
                },
            ),
            step(
                4,
                Op::WriteMemory {
                    object: 3,
                    address: 0x3C00,
                    data: vec![0x12, 0x34, 0x56, 0x78],
                },
            ),
            step(
                5,
                Op::LoadControl {
                    object: 3,
                    payload: task_control_2(TaskControl2 {
                        callback: 0x1234,
                        communication_objects: 0x400,
                        segment_1: 0,
                        segment_2: 0,
                    }),
                },
            ),
            step(
                6,
                Op::ToolMetadata {
                    table_address: 0x400,
                },
            ),
            control(7, LoadEvent::LoadCompleted),
            step(8, Op::Disconnect),
        ],
    )
    .unwrap();
    (device, plan)
}

fn config() -> SimulatorConfig {
    SimulatorConfig {
        mask_version: 0x0025,
        ..SimulatorConfig::default()
    }
}
fn auth(device: &SimulatedDevice) -> WriteAuthorisation {
    WriteAuthorisation::for_simulator(device.address(), WriteScope::Download).unwrap()
}
fn no_writes(device: &SimulatedDevice) {
    assert!(!device
        .seen()
        .iter()
        .any(|s| matches!(s, Seen::PropertyWrite { .. } | Seen::MemoryWrite { .. })));
}

#[tokio::test]
async fn normalized_plan_executes_exact_property_records_and_verified_data_without_an_object_five_write(
) {
    let (device, plan) = fixture(config());
    let report = run_simulated_property_plan(
        &device,
        &plan,
        auth(&device),
        AuthorisationPlan::Skip,
        timing(),
    )
    .await
    .unwrap();
    assert_eq!(report.completed.len(), 10);
    assert_eq!(
        report
            .completed
            .iter()
            .map(|s| s.source_index)
            .collect::<Vec<_>>(),
        [0, 1, 2, 3, 4, 4, 5, 6, 7, 8]
    );
    assert!(!report.completed[7].wire_operation);
    assert_eq!(report.data_octets, 4);
    assert_eq!(
        device.memory(0x3C00, 4),
        [Some(0x12), Some(0x34), Some(0x56), Some(0x78)]
    );
    assert_eq!(device.load_state(ObjectIndex::new(3)), LoadState::Loaded);
    let controls = device
        .seen()
        .into_iter()
        .filter_map(|s| match s {
            Seen::PropertyWrite {
                object_index,
                property_id: PID_LOAD_STATE_CONTROL,
                data,
            } => Some((object_index, data)),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(
        controls,
        vec![
            (3, vec![4, 0, 0, 0, 0, 0, 0, 0, 0, 0]),
            (3, vec![1, 0, 0, 0, 0, 0, 0, 0, 0, 0]),
            (3, vec![3, 0, 0x3C, 0, 0, 4, 0xFF, 3, 0x80, 0]),
            (3, vec![3, 5, 0x12, 0x34, 4, 0, 0, 0, 0, 0]),
            (3, vec![2, 0, 0, 0, 0, 0, 0, 0, 0, 0]),
        ]
    );
    assert!(matches!(device.seen().last(), Some(Seen::Disconnect)));
    let mut reader =
        ManagementSession::read_only(&device, device.address(), AuthorisationPlan::Skip, timing())
            .unwrap();
    reader.connect().await.unwrap();
    assert_eq!(
        reader
            .read_property(ObjectIndex::new(3), PID_TABLE_REFERENCE, 1, 1)
            .await
            .unwrap(),
        [0, 0, 0x3C, 0]
    );
    reader.disconnect().await;
}

#[tokio::test]
async fn mask_manufacturer_and_hardware_mismatch_stop_before_verify_mode_or_any_write() {
    for cause in 0..4 {
        let (device, plan) = fixture(if cause == 0 {
            SimulatorConfig {
                mask_version: 0x0701,
                ..config()
            }
        } else {
            config()
        });
        if cause == 1 {
            device.preset_property(0, 12, &[0xFF, 0xFF]);
        }
        if cause == 2 {
            device.preset_property(0, 78, &[0; 6]);
        }
        if cause == 3 {
            device.preset_property(0, 78, &[0; 5]);
        }
        let error = run_simulated_property_plan(
            &device,
            &plan,
            auth(&device),
            AuthorisationPlan::Skip,
            timing(),
        )
        .await
        .unwrap_err();
        assert_eq!(error.source_index, Some(if cause < 2 { 0 } else { 1 }));
        assert!(!error.mutation_may_have_started);
        no_writes(&device);
        assert!(matches!(device.seen().last(), Some(Seen::Disconnect)));
    }
}

#[tokio::test]
async fn wrong_scope_or_target_authorization_refuses_before_connecting() {
    for authorization in [
        WriteAuthorisation::for_simulator(
            knx_core::IndividualAddress::from_raw(0x1234),
            WriteScope::Download,
        )
        .unwrap(),
        WriteAuthorisation::for_simulator(
            knx_core::IndividualAddress::from_raw(0x1119),
            WriteScope::Restart,
        )
        .unwrap(),
    ] {
        let (device, plan) = fixture(config());
        let error = run_simulated_property_plan(
            &device,
            &plan,
            authorization,
            AuthorisationPlan::Skip,
            timing(),
        )
        .await
        .unwrap_err();
        assert_eq!(error.source_index, None);
        assert!(!error.mutation_may_have_started);
        assert!(device.seen().is_empty());
    }
}

#[tokio::test]
async fn ignored_load_control_reports_source_position_and_never_writes_data() {
    let (device, plan) = fixture(SimulatorConfig {
        drop_load_state_writes: true,
        ..config()
    });
    let error = run_simulated_property_plan(
        &device,
        &plan,
        auth(&device),
        AuthorisationPlan::Skip,
        timing(),
    )
    .await
    .unwrap_err();
    assert_eq!(error.source_index, Some(3));
    assert!(error.mutation_may_have_started);
    assert!(!device.memory_was_written());
    assert!(matches!(device.seen().last(), Some(Seen::Disconnect)));
}

#[tokio::test]
async fn corrupted_data_is_not_a_successful_download_or_an_automatic_restore() {
    let (device, plan) = fixture(SimulatorConfig {
        corrupt_memory_writes: true,
        ..config()
    });
    let error = run_simulated_property_plan(
        &device,
        &plan,
        auth(&device),
        AuthorisationPlan::Skip,
        timing(),
    )
    .await
    .unwrap_err();
    assert_eq!(error.source_index, Some(4));
    assert!(error.mutation_may_have_started);
    assert_eq!(device.load_state(ObjectIndex::new(3)), LoadState::Loading);
    assert!(matches!(device.seen().last(), Some(Seen::Disconnect)));
}

#[tokio::test]
async fn absent_verify_mode_uses_existing_explicit_memory_readback() {
    let (device, plan) = fixture(SimulatorConfig {
        verify_mode_supported: false,
        ..config()
    });
    let report = run_simulated_property_plan(
        &device,
        &plan,
        auth(&device),
        AuthorisationPlan::Skip,
        timing(),
    )
    .await
    .unwrap();
    assert_eq!(report.data_octets, 4);
    assert!(!device.verify_mode());
    assert_eq!(device.load_state(ObjectIndex::new(3)), LoadState::Loaded);
}

#[tokio::test]
async fn hardware_authorization_and_wrong_simulator_instance_never_connect() {
    use knx_core::commissioning::mutation::required_confirmation_phrase;
    let (device, plan) = fixture(config());
    let confirmation = required_confirmation_phrase(device.address(), WriteScope::Download);
    let hardware =
        WriteAuthorisation::for_hardware(device.address(), WriteScope::Download, &confirmation)
            .unwrap();
    assert!(run_simulated_property_plan(
        &device,
        &plan,
        hardware,
        AuthorisationPlan::Skip,
        timing()
    )
    .await
    .is_err());
    assert!(device.seen().is_empty());
    let other =
        SimulatedDevice::with_config_at(knx_core::IndividualAddress::from_raw(0x111A), config());
    let error = run_simulated_property_plan(
        &other,
        &plan,
        auth(&device),
        AuthorisationPlan::Skip,
        timing(),
    )
    .await
    .unwrap_err();
    assert_eq!(error.source_index, None);
    assert!(!error.mutation_may_have_started);
    assert!(other.seen().is_empty());
}
