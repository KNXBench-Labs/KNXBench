//! Experimental recovery acquisition uses the existing session, never another protocol.

use knx_core::commissioning::absolute_load_control::{absolute_data, AbsolutePropertySegment};
use knx_core::commissioning::authorisation::AuthorisationPlan;
use knx_core::commissioning::load_control::event_payload;
use knx_core::commissioning::load_control_memory::SegmentMemoryType;
use knx_core::commissioning::load_state::{LoadEvent, LoadState, MaskVersion};
use knx_core::commissioning::mutation::TargetKind;
use knx_core::commissioning::mutation::{WriteAuthorisation, WriteScope};
use knx_core::commissioning::properties::{ObjectIndex, PID_HARDWARE_TYPE, PID_PROGRAM_VERSION};
use knx_core::commissioning::property_download::{
    PropertyDownloadPlan, PropertyDownloadStep, PropertyOperation as Op,
};
use knx_core::IndividualAddress;
use knx_net::cemi::{ApplicationService, Destination, Tpci};
use knx_net::client::{BusError, TunnelEvent};
use knx_net::commissioning::property_download::{
    run_experimental_property_plan, PropertySimulationFailure,
};
use knx_net::commissioning::property_recovery::{
    ExperimentalPropertyPreconditions, PropertyJournalEvent,
};
use knx_net::commissioning::simulator::{Seen, SimulatedDevice, SimulatorConfig};
use knx_net::commissioning::SessionTiming;
use knx_net::management::ManagementTransport;
use std::time::Duration;
use tokio::sync::broadcast;

// A forwarding wrapper proves generic transport handling; it never reaches a bus.
struct Forward<'a> {
    device: &'a SimulatedDevice,
    kind: TargetKind,
    discard_allocation: bool,
    short_read: bool,
}
impl ManagementTransport for Forward<'_> {
    fn assigned_address(&self) -> IndividualAddress {
        ManagementTransport::assigned_address(self.device)
    }
    fn subscribe(&self) -> broadcast::Receiver<TunnelEvent> {
        ManagementTransport::subscribe(self.device)
    }
    fn target_kind(&self) -> TargetKind {
        self.kind
    }
    async fn send_frame(
        &self,
        destination: Destination,
        transport: Tpci,
        mut service: ApplicationService,
    ) -> Result<(), BusError> {
        if self.discard_allocation {
            if let ApplicationService::PropertyValueWrite {
                property_id: 5,
                data,
                ..
            } = &mut service
            {
                if data.first() == Some(&3) && data.get(1) == Some(&0) {
                    data[1] = 5;
                }
            }
        }
        if self.short_read {
            match &mut service {
                ApplicationService::MemoryRead { number, .. }
                | ApplicationService::UserMemoryRead { number, .. }
                    if *number > 1 =>
                {
                    *number -= 1
                }
                _ => {}
            }
        }
        ManagementTransport::send_frame(self.device, destination, transport, service).await
    }
}
fn hardware_auth(device: &SimulatedDevice) -> WriteAuthorisation {
    let phrase = knx_core::commissioning::mutation::required_confirmation_phrase(
        device.address(),
        WriteScope::Download,
    );
    WriteAuthorisation::for_hardware(device.address(), WriteScope::Download, &phrase).unwrap()
}

const SERIAL: [u8; 6] = [0, 2, 1, 2, 3, 4];
const VERSION: [u8; 5] = [0, 2, 0x12, 0x34, 1];
fn fixture() -> (
    SimulatedDevice,
    PropertyDownloadPlan,
    ExperimentalPropertyPreconditions,
) {
    let device = SimulatedDevice::with_config(SimulatorConfig {
        mask_version: 0x0025,
        serial_number: Some(SERIAL),
        ..SimulatorConfig::default()
    });
    device.preset_property(0, PID_HARDWARE_TYPE, &[1; 6]);
    device.preset_property(3, PID_PROGRAM_VERSION, &VERSION);
    device.preset_load_state(ObjectIndex::new(3), LoadState::Loaded);
    device.preset_memory(0x3C00, &[9, 8, 7, 6]);
    let op = |source_index, operation| PropertyDownloadStep {
        source_index,
        operation,
    };
    let control = |source_index, payload| op(source_index, Op::LoadControl { object: 3, payload });
    let plan = PropertyDownloadPlan::new(
        device.address(),
        MaskVersion(0x0025),
        2,
        vec![
            op(0, Op::Connect),
            op(1, Op::HardwareCheck { expected: [1; 6] }),
            control(2, event_payload(LoadEvent::Unload)),
            control(3, event_payload(LoadEvent::StartLoading)),
            control(
                4,
                absolute_data(AbsolutePropertySegment {
                    start: 0x3C00,
                    length: 4,
                    access: 0xFF,
                    memory_type: SegmentMemoryType::Eeprom,
                    checksum_disabled: true,
                })
                .unwrap(),
            ),
            op(
                4,
                Op::WriteMemory {
                    object: 3,
                    address: 0x3C00,
                    data: vec![1, 2, 3, 4],
                },
            ),
            control(5, event_payload(LoadEvent::LoadCompleted)),
            op(6, Op::Disconnect),
        ],
    )
    .unwrap();
    let preconditions =
        ExperimentalPropertyPreconditions::same_application_reinstallation(&plan, SERIAL, VERSION)
            .unwrap();
    // Establish exactly the source allocation shape before recovery acquisition.
    (device, plan, preconditions)
}
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
fn auth(device: &SimulatedDevice) -> WriteAuthorisation {
    WriteAuthorisation::for_simulator(device.address(), WriteScope::Download).unwrap()
}
async fn establish_shape(device: &SimulatedDevice, plan: &PropertyDownloadPlan) {
    knx_net::commissioning::property_download::run_simulated_property_plan(
        device,
        plan,
        auth(device),
        AuthorisationPlan::Skip,
        timing(),
    )
    .await
    .unwrap();
    device.preset_memory(0x3C00, &[9, 8, 7, 6]);
}
#[tokio::test]
async fn abort_after_durable_snapshot_is_read_only_and_never_enables_verify_mode() {
    use std::cell::Cell;
    let (device, plan, preconditions) = fixture();
    establish_shape(&device, &plan).await;
    let abort = Cell::new(false);
    let before = device.seen().len();
    let mut saved = false;
    let error = run_experimental_property_plan(
        &device,
        &plan,
        auth(&device),
        AuthorisationPlan::Skip,
        timing(),
        &preconditions,
        &mut |event| {
            if matches!(event, PropertyJournalEvent::RecoveryAcquired(_)) {
                saved = true;
                abort.set(true);
            }
            Ok(())
        },
        &|| abort.get(),
    )
    .await
    .unwrap_err();
    assert!(saved);
    assert!(matches!(
        error.failure,
        PropertySimulationFailure::Cancelled
    ));
    assert_eq!(error.operation_index, Some(1));
    assert_eq!(error.completed.len(), 1);
    assert!(!error.mutation_may_have_started);
    assert!(!device.seen()[before..]
        .iter()
        .any(|seen| matches!(seen, Seen::PropertyWrite { .. } | Seen::MemoryWrite { .. })));
    assert!(matches!(device.seen().last(), Some(Seen::Disconnect)));
}

#[tokio::test]
async fn abort_requested_by_durable_start_callback_stops_that_operation_before_send() {
    use std::cell::Cell;
    let (device, plan, preconditions) = fixture();
    establish_shape(&device, &plan).await;
    let abort = Cell::new(false);
    let before = device.seen().len();
    let error = run_experimental_property_plan(
        &device,
        &plan,
        auth(&device),
        AuthorisationPlan::Skip,
        timing(),
        &preconditions,
        &mut |event| {
            if matches!(
                event,
                PropertyJournalEvent::OperationStarting {
                    operation_index: 2,
                    ..
                }
            ) {
                abort.set(true);
            }
            Ok(())
        },
        &|| abort.get(),
    )
    .await
    .unwrap_err();
    assert!(matches!(
        error.failure,
        PropertySimulationFailure::Cancelled
    ));
    assert_eq!(
        error.operation_index,
        Some(2),
        "cancellation raised at intent barrier must prevent its send"
    );
    assert_eq!(error.completed.len(), 2);
    assert_eq!(device.load_state(ObjectIndex::new(3)), LoadState::Loaded);
    assert!(!device.seen()[before..].iter().any(|seen| matches!(
        seen,
        Seen::PropertyWrite { property_id: 5, .. } | Seen::MemoryWrite { .. }
    )));
}

#[tokio::test]
async fn exact_snapshot_shape_corruption_refuses_restore_before_connection() {
    use knx_net::commissioning::property_download::restore_experimental_property_plan;
    let (device, plan, preconditions) = fixture();
    establish_shape(&device, &plan).await;
    let report = run_experimental_property_plan(
        &device,
        &plan,
        auth(&device),
        AuthorisationPlan::Skip,
        timing(),
        &preconditions,
        &mut |_| Ok(()),
        &|| false,
    )
    .await
    .unwrap();
    for cause in 0..11 {
        let mut snapshot = report.recovery.clone();
        match cause {
            0 => snapshot.version = 2,
            1 => snapshot.resources.clear(),
            2 => snapshot.resources[0].state = LoadState::Loading,
            3 => snapshot.resources[0].pointer[0] = 1,
            4 => snapshot.allocations.clear(),
            5 => snapshot.allocations[0].data.pop().map(|_| ()).unwrap(),
            6 => snapshot.allocations[0].operation_index += 1,
            7 => snapshot.regions[0].data[0] ^= 1,
            8 => snapshot.regions[0].source_index += 1,
            9 => snapshot.regions.push(snapshot.regions[0].clone()),
            10 => snapshot.preconditions.source_pointers.clear(),
            _ => unreachable!(),
        }
        let before = device.seen().len();
        let error = restore_experimental_property_plan(
            &device,
            &plan,
            &snapshot,
            auth(&device),
            AuthorisationPlan::Skip,
            timing(),
            &mut |_| Ok(()),
            &|| false,
        )
        .await
        .unwrap_err();
        assert_eq!(
            device.seen().len(),
            before,
            "corrupt binding {cause} must refuse before connecting"
        );
        assert_eq!(error.operation_index, None);
        assert!(matches!(
            error.failure,
            PropertySimulationFailure::Recovery(_)
        ));
    }
}

#[tokio::test]
async fn durable_operation_barriers_fail_with_exact_completed_prefix_and_no_automatic_restore() {
    for at_completion in [false, true] {
        for position in 0..8 {
            let (device, plan, preconditions) = fixture();
            establish_shape(&device, &plan).await;
            let error = run_experimental_property_plan(
                &device,
                &plan,
                auth(&device),
                AuthorisationPlan::Skip,
                timing(),
                &preconditions,
                &mut |event| {
                    let fails = match event {
                        PropertyJournalEvent::OperationStarting {
                            operation_index, ..
                        } => !at_completion && operation_index == position,
                        PropertyJournalEvent::OperationCompleted {
                            operation_index, ..
                        } => at_completion && operation_index == position,
                        _ => false,
                    };
                    if fails {
                        Err("operation journal unavailable".into())
                    } else {
                        Ok(())
                    }
                },
                &|| false,
            )
            .await
            .unwrap_err();
            assert_eq!(error.operation_index, Some(position));
            assert_eq!(error.completed.len(), position + usize::from(at_completion));
            assert_eq!(
                error.mutation_may_have_started,
                position > 1 || (position == 1 && at_completion)
            );
            assert!(matches!(
                error.failure,
                PropertySimulationFailure::Persistence(_)
            ));
            assert!(matches!(device.seen().last(), Some(Seen::Disconnect)));
        }
    }
}

#[tokio::test]
async fn initial_identity_state_pointer_or_short_memory_refuses_before_any_setup_write() {
    for cause in 0..7 {
        let (device, original, _) = fixture();
        establish_shape(&device, &original).await;
        let mut plan = original.clone();
        match cause {
            0 => device.preset_property(0, 11, &[7; 6]),
            1 => device.preset_property(3, PID_PROGRAM_VERSION, &[7; 5]),
            2 => device.preset_property(0, PID_HARDWARE_TYPE, &[7; 6]),
            3 => device.preset_property(0, 12, &[0xFF; 2]),
            4 => device.preset_load_state(ObjectIndex::new(3), LoadState::Loading),
            5 => {
                let mut steps = plan.steps().to_vec();
                steps[4].operation = Op::LoadControl {
                    object: 3,
                    payload: absolute_data(AbsolutePropertySegment {
                        start: 0x3D00,
                        length: 4,
                        access: 0xFF,
                        memory_type: SegmentMemoryType::Eeprom,
                        checksum_disabled: true,
                    })
                    .unwrap(),
                };
                steps[5].operation = Op::WriteMemory {
                    object: 3,
                    address: 0x3D00,
                    data: vec![1, 2, 3, 4],
                };
                plan = PropertyDownloadPlan::new(
                    plan.target(),
                    plan.mask(),
                    plan.manufacturer(),
                    steps,
                )
                .unwrap();
            }
            6 => {}
            _ => unreachable!(),
        }
        let preconditions = ExperimentalPropertyPreconditions::same_application_reinstallation(
            &plan, SERIAL, VERSION,
        )
        .unwrap();
        let transport = Forward {
            device: &device,
            kind: TargetKind::Simulator,
            discard_allocation: false,
            short_read: cause == 6,
        };
        let before = device.seen().len();
        let error = run_experimental_property_plan(
            &transport,
            &plan,
            auth(&device),
            AuthorisationPlan::Skip,
            timing(),
            &preconditions,
            &mut |_| Ok(()),
            &|| false,
        )
        .await
        .unwrap_err();
        assert!(
            !error.mutation_may_have_started,
            "initial preflight cause {cause}"
        );
        assert!(!device.seen()[before..]
            .iter()
            .any(|seen| matches!(seen, Seen::MemoryWrite { .. } | Seen::PropertyWrite { .. })));
        assert!(matches!(device.seen().last(), Some(Seen::Disconnect)));
    }
}

#[tokio::test]
async fn generic_transport_requires_matching_target_kind_and_download_scope() {
    let (device, plan, preconditions) = fixture();
    establish_shape(&device, &plan).await;
    let transport = Forward {
        device: &device,
        kind: TargetKind::Hardware,
        discard_allocation: false,
        short_read: false,
    };
    for authorization in [
        auth(&device),
        WriteAuthorisation::for_simulator(device.address(), WriteScope::Restart).unwrap(),
        WriteAuthorisation::for_simulator(
            IndividualAddress::from_raw(0x111A),
            WriteScope::Download,
        )
        .unwrap(),
    ] {
        let before = device.seen().len();
        let error = run_experimental_property_plan(
            &transport,
            &plan,
            authorization,
            AuthorisationPlan::Skip,
            timing(),
            &preconditions,
            &mut |_| Ok(()),
            &|| false,
        )
        .await
        .unwrap_err();
        assert_eq!(device.seen().len(), before);
        assert!(!error.mutation_may_have_started);
    }
    // A matching hardware-kind authorization is still not source/recovery admission.
    // Forward terminates in memory; the regression opens no real transport.
    let before = device.seen().len();
    let error = run_experimental_property_plan(
        &transport,
        &plan,
        hardware_auth(&device),
        AuthorisationPlan::Skip,
        timing(),
        &preconditions,
        &mut |_| Ok(()),
        &|| false,
    )
    .await
    .expect_err("generic experimental API must not bypass physical source admission");
    assert_eq!(device.seen().len(), before);
    assert!(!error.mutation_may_have_started);
    assert!(matches!(
        error.failure,
        PropertySimulationFailure::Recovery(_)
    ));
}

#[tokio::test]
async fn silent_absolute_allocation_failure_stops_before_any_memory_write() {
    let (device, plan, preconditions) = fixture();
    establish_shape(&device, &plan).await;
    let before = device.seen().len();
    let transport = Forward {
        device: &device,
        kind: TargetKind::Simulator,
        discard_allocation: true,
        short_read: false,
    };
    let error = run_experimental_property_plan(
        &transport,
        &plan,
        auth(&device),
        AuthorisationPlan::Skip,
        timing(),
        &preconditions,
        &mut |_| Ok(()),
        &|| false,
    )
    .await
    .unwrap_err();
    assert_eq!(
        error.operation_index,
        Some(4),
        "allocation pointer must be checked before data"
    );
    assert!(!device.seen()[before..]
        .iter()
        .any(|seen| matches!(seen, Seen::MemoryWrite { .. })));
    assert!(error.mutation_may_have_started);
    assert_eq!(
        device.memory(0x3C00, 4),
        [Some(9), Some(8), Some(7), Some(6)]
    );
    assert!(matches!(device.seen().last(), Some(Seen::Disconnect)));
}

#[tokio::test]
async fn generic_hardware_kind_path_refuses_unwritten_allocation_bytes_before_connect() {
    let (device, original, _) = fixture();
    let mut steps = original.steps().to_vec();
    steps[4].operation = Op::LoadControl {
        object: 3,
        payload: absolute_data(AbsolutePropertySegment {
            start: 0x3C00,
            length: 8,
            access: 0xFF,
            memory_type: SegmentMemoryType::Eeprom,
            checksum_disabled: true,
        })
        .unwrap(),
    };
    let plan = PropertyDownloadPlan::new(
        original.target(),
        original.mask(),
        original.manufacturer(),
        steps,
    )
    .unwrap();
    let preconditions =
        ExperimentalPropertyPreconditions::same_application_reinstallation(&plan, SERIAL, VERSION)
            .unwrap();
    let transport = Forward {
        device: &device,
        kind: TargetKind::Hardware,
        discard_allocation: false,
        short_read: false,
    };
    let error = run_experimental_property_plan(
        &transport,
        &plan,
        hardware_auth(&device),
        AuthorisationPlan::Skip,
        timing(),
        &preconditions,
        &mut |_| Ok(()),
        &|| false,
    )
    .await
    .unwrap_err();
    assert!(
        device.seen().is_empty(),
        "unproven masked-byte recovery must refuse before connecting"
    );
    assert!(matches!(
        error.failure,
        PropertySimulationFailure::Recovery(_)
    ));
    assert!(!error.mutation_may_have_started);
    assert_eq!(error.operation_index, None);
}

#[tokio::test]
async fn masked_allocation_captures_full_interval_and_verifies_preserved_bytes() {
    use knx_net::commissioning::property_download::restore_experimental_property_plan;
    let (device, original, _) = fixture();
    let mut steps = original.steps().to_vec();
    steps[4].operation = Op::LoadControl {
        object: 3,
        payload: absolute_data(AbsolutePropertySegment {
            start: 0x3C00,
            length: 8,
            access: 0xFF,
            memory_type: SegmentMemoryType::Eeprom,
            checksum_disabled: true,
        })
        .unwrap(),
    };
    let plan = PropertyDownloadPlan::new(
        original.target(),
        original.mask(),
        original.manufacturer(),
        steps,
    )
    .unwrap();
    let preconditions =
        ExperimentalPropertyPreconditions::same_application_reinstallation(&plan, SERIAL, VERSION)
            .unwrap();
    establish_shape(&device, &plan).await;
    device.preset_memory(0x3C00, &[9, 8, 7, 6, 5, 4, 3, 2]);
    let report = run_experimental_property_plan(
        &device,
        &plan,
        auth(&device),
        AuthorisationPlan::Skip,
        timing(),
        &preconditions,
        &mut |_| Ok(()),
        &|| false,
    )
    .await
    .unwrap();
    assert_eq!(report.recovery.allocations.len(), 1);
    assert_eq!(report.recovery.allocations[0].operation_index, 4);
    assert_eq!(
        report.recovery.allocations[0].data,
        [9, 8, 7, 6, 5, 4, 3, 2]
    );
    assert_eq!(report.recovery.regions[0].data, [9, 8, 7, 6]);
    restore_experimental_property_plan(
        &device,
        &plan,
        &report.recovery,
        auth(&device),
        AuthorisationPlan::Skip,
        timing(),
        &mut |_| Ok(()),
        &|| false,
    )
    .await
    .unwrap();
    assert_eq!(
        device.memory(0x3C00, 8),
        [
            Some(9),
            Some(8),
            Some(7),
            Some(6),
            Some(5),
            Some(4),
            Some(3),
            Some(2)
        ]
    );
    let error = run_experimental_property_plan(
        &device,
        &plan,
        auth(&device),
        AuthorisationPlan::Skip,
        timing(),
        &preconditions,
        &mut |event| {
            if matches!(
                event,
                PropertyJournalEvent::OperationCompleted {
                    operation_index: 5,
                    ..
                }
            ) {
                device.preset_memory(0x3C04, &[0xEE]);
            }
            Ok(())
        },
        &|| false,
    )
    .await
    .unwrap_err();
    assert!(
        matches!(error.failure, PropertySimulationFailure::Recovery(_)),
        "full preserved interval must be verified"
    );
    assert_eq!(error.operation_index, Some(7));
    assert!(error.mutation_may_have_started);
}

#[tokio::test]
async fn interrupted_restore_revalidates_immutable_serial_without_requiring_intact_current_program_version(
) {
    use knx_net::commissioning::property_download::restore_experimental_property_plan;
    let (device, plan, preconditions) = fixture();
    establish_shape(&device, &plan).await;
    let original = run_experimental_property_plan(
        &device,
        &plan,
        auth(&device),
        AuthorisationPlan::Skip,
        timing(),
        &preconditions,
        &mut |_| Ok(()),
        &|| false,
    )
    .await
    .unwrap();
    device.preset_property(3, PID_PROGRAM_VERSION, &[]);
    device.preset_load_state(ObjectIndex::new(3), LoadState::Loading);
    let error = restore_experimental_property_plan(
        &device,
        &plan,
        &original.recovery,
        auth(&device),
        AuthorisationPlan::Skip,
        timing(),
        &mut |_| Ok(()),
        &|| false,
    )
    .await
    .expect_err("recovery must not claim success if the declared procedure did not restore program identity");
    assert!(matches!(
        error.failure,
        PropertySimulationFailure::Recovery(_)
    ));
    assert_eq!(error.source_index,Some(6),"initial interrupted identity must not prevent the recovery attempt; final verification must refuse");
    assert_eq!(
        device.memory(0x3C00, 4),
        [Some(9), Some(8), Some(7), Some(6)]
    );
}

#[tokio::test]
async fn three_resource_task_segment_source_pointers_include_explicit_zero_application_pointer() {
    use knx_core::commissioning::load_control::{additional_load_control, LoadControlSubtype};
    let (device, base, _) = fixture();
    let mut steps = base.steps()[..2].to_vec();
    let mut source = 2;
    for object in 1..=3 {
        for event in [LoadEvent::Unload, LoadEvent::StartLoading] {
            steps.push(PropertyDownloadStep {
                source_index: source,
                operation: Op::LoadControl {
                    object,
                    payload: event_payload(event),
                },
            });
            source += 1;
        }
        let pointer = if object == 3 {
            0u16
        } else {
            u16::from(object) * 0x100
        };
        let [hi, lo] = pointer.to_be_bytes();
        steps.push(PropertyDownloadStep {
            source_index: source,
            operation: Op::LoadControl {
                object,
                payload: additional_load_control(
                    LoadControlSubtype::AllocAbsTaskSeg,
                    [hi, lo, 0, 0, 0, 0, 0, 0],
                ),
            },
        });
        source += 1;
        steps.push(PropertyDownloadStep {
            source_index: source,
            operation: Op::LoadControl {
                object,
                payload: event_payload(LoadEvent::LoadCompleted),
            },
        });
        source += 1;
    }
    steps.push(PropertyDownloadStep {
        source_index: source,
        operation: Op::Disconnect,
    });
    let plan =
        PropertyDownloadPlan::new(base.target(), base.mask(), base.manufacturer(), steps).unwrap();
    let preconditions =
        ExperimentalPropertyPreconditions::same_application_reinstallation(&plan, SERIAL, VERSION)
            .expect("explicit source TaskSegment3 zero is not an absent table pointer");
    assert_eq!(
        preconditions
            .source_pointers
            .iter()
            .map(|p| (p.object, p.value))
            .collect::<Vec<_>>(),
        [(1, [0, 0, 1, 0]), (2, [0, 0, 2, 0]), (3, [0, 0, 0, 0])]
    );
    establish_shape(&device, &plan).await;
    let report = run_experimental_property_plan(
        &device,
        &plan,
        auth(&device),
        AuthorisationPlan::Skip,
        timing(),
        &preconditions,
        &mut |_| Ok(()),
        &|| false,
    )
    .await
    .unwrap();
    assert_eq!(report.recovery.resources.len(), 3);
    assert_eq!(report.recovery.resources[2].pointer, [0; 4]);
}

#[tokio::test]
async fn explicit_abort_leaves_partial_state_and_separate_restore_replays_complete_original_shape()
{
    use knx_net::commissioning::property_download::restore_experimental_property_plan;
    use std::cell::Cell;
    let (device, plan, preconditions) = fixture();
    establish_shape(&device, &plan).await;
    device.preset_property(0, 14, &[0x09]);
    let abort = Cell::new(false);
    let mut saved = None;
    let error = run_experimental_property_plan(
        &device,
        &plan,
        auth(&device),
        AuthorisationPlan::Skip,
        timing(),
        &preconditions,
        &mut |event| {
            match event {
                PropertyJournalEvent::RecoveryAcquired(snapshot) => saved = Some(snapshot.clone()),
                PropertyJournalEvent::OperationCompleted {
                    operation_index: 5, ..
                } => abort.set(true),
                _ => {}
            }
            Ok(())
        },
        &|| abort.get(),
    )
    .await
    .unwrap_err();
    assert!(matches!(
        error.failure,
        PropertySimulationFailure::Cancelled
    ));
    assert_eq!(
        (error.operation_index, error.source_index),
        (Some(6), Some(5))
    );
    assert_eq!(error.completed.len(), 6);
    assert!(error.mutation_may_have_started);
    assert_eq!(device.load_state(ObjectIndex::new(3)), LoadState::Loading);
    assert_eq!(
        device.memory(0x3C00, 4),
        [Some(1), Some(2), Some(3), Some(4)]
    );
    assert!(matches!(device.seen().last(), Some(Seen::Disconnect)));
    let snapshot = saved.unwrap();
    let before = device.seen().len();
    let restored = restore_experimental_property_plan(
        &device,
        &plan,
        &snapshot,
        auth(&device),
        AuthorisationPlan::Skip,
        timing(),
        &mut |_| Ok(()),
        &|| false,
    )
    .await
    .unwrap();
    assert_eq!(restored.execution.completed.len(), plan.steps().len());
    assert_eq!(
        device.memory(0x3C00, 4),
        [Some(9), Some(8), Some(7), Some(6)]
    );
    assert_eq!(device.load_state(ObjectIndex::new(3)), LoadState::Loaded);
    assert!(!device.verify_mode());
    let controls = device.seen()[before..]
        .iter()
        .filter_map(|seen| match seen {
            Seen::PropertyWrite {
                object_index: 3,
                property_id: 5,
                data,
            } => Some(data.clone()),
            _ => None,
        })
        .collect::<Vec<_>>();
    let expected = plan
        .steps()
        .iter()
        .filter_map(|step| match &step.operation {
            Op::LoadControl { payload, .. } => Some(payload.octets().to_vec()),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(
        controls, expected,
        "restore is full same-procedure reinstallation, never PID5 status reconstruction"
    );
    assert!(matches!(device.seen().last(), Some(Seen::Disconnect)));
}

#[tokio::test]
async fn successful_execution_restores_prior_verify_bit_and_journals_every_normalized_operation() {
    let (device, plan, preconditions) = fixture();
    establish_shape(&device, &plan).await;
    device.preset_property(0, 14, &[0x09]);
    let before = device.seen().len();
    let mut starts = Vec::new();
    let mut ends = Vec::new();
    let report = run_experimental_property_plan(
        &device,
        &plan,
        auth(&device),
        AuthorisationPlan::Skip,
        timing(),
        &preconditions,
        &mut |event| {
            match event {
                PropertyJournalEvent::RecoveryAcquired(snapshot) => {
                    assert_eq!(snapshot.old_device_control, 0x09)
                }
                PropertyJournalEvent::OperationStarting {
                    operation_index, ..
                } => starts.push(operation_index),
                PropertyJournalEvent::OperationCompleted {
                    operation_index, ..
                } => ends.push(operation_index),
            }
            Ok(())
        },
        &|| false,
    )
    .await
    .unwrap();
    assert_eq!(report.execution.completed.len(), plan.steps().len());
    assert_eq!(starts, (0..plan.steps().len()).collect::<Vec<_>>());
    assert_eq!(ends, starts);
    let controls = device.seen()[before..]
        .iter()
        .filter_map(|seen| match seen {
            Seen::PropertyWrite {
                object_index: 0,
                property_id: 14,
                data,
            } => Some(data.clone()),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(
        controls,
        [vec![0x0D], vec![0x09]],
        "VerifyMode lifecycle must restore the prior bit"
    );
    assert_eq!(
        device.memory(0x3C00, 4),
        [Some(1), Some(2), Some(3), Some(4)]
    );
    assert!(!device.verify_mode());
    assert!(matches!(device.seen().last(), Some(Seen::Disconnect)));
}

#[tokio::test]
async fn durable_snapshot_failure_prevents_first_verify_mode_mutation() {
    let (device, plan, preconditions) = fixture();
    establish_shape(&device, &plan).await;
    let before = device.seen().len();
    let mut acquired = false;
    let error = run_experimental_property_plan(
        &device,
        &plan,
        auth(&device),
        AuthorisationPlan::Skip,
        timing(),
        &preconditions,
        &mut |event| {
            if let PropertyJournalEvent::RecoveryAcquired(snapshot) = event {
                acquired = true;
                assert_eq!(snapshot.original_plan, plan);
                assert_eq!(snapshot.regions[0].data, [9, 8, 7, 6]);
                assert_eq!(snapshot.regions[0].operation_index, 5);
                return Err("durable store failed".to_string());
            }
            Ok(())
        },
        &|| false,
    )
    .await
    .unwrap_err();
    assert!(acquired, "read-only acquisition must precede persistence");
    assert!(matches!(
        error.failure,
        PropertySimulationFailure::Persistence(_)
    ));
    assert!(!error.mutation_may_have_started);
    assert_eq!(error.operation_index, Some(1));
    assert_eq!(error.completed.len(), 1);
    let seen = device.seen();
    assert!(!seen[before..]
        .iter()
        .any(|s| matches!(s, Seen::PropertyWrite { .. } | Seen::MemoryWrite { .. })));
    assert!(matches!(seen.last(), Some(Seen::Disconnect)));
}
