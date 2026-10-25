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
use knx_net::commissioning::hardware_release::{required_release_phrase, HardwareRelease};
use knx_net::commissioning::property_download::{
    run_experimental_property_plan, run_experimental_property_plan_on_hardware,
    PropertySimulationFailure,
};
use knx_net::commissioning::property_recovery::{
    ExperimentalPropertyPreconditions, PropertyInProcedurePointerObservation, PropertyJournalEvent,
    PropertyPointerObservation, PropertyProgramVersionObservation, PropertySerialObservation,
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
            3 => snapshot.resources[0].pointer = Some([1, 0, 0x3C, 0]),
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
async fn device_type_or_short_memory_refuses_before_any_setup_write() {
    // Load state and pointer differences are observations now (issue #30); see
    // prestate_pointer_mismatch_is_observed_and_download_completes_without_original_recovery.
    // Serial is no admission check either (owner amendment 2026-10-25); see
    // serial_mismatch_is_recorded_and_the_download_proceeds.
    // Nor is the current program version (owner amendment 2); see
    // different_prior_program_version_is_recorded_and_the_download_proceeds.
    // A short prestate memory answer is an observation too (review fix 5); see
    // unreadable_prestate_memory_is_observed_and_leaves_no_recovery.
    for cause in [2, 3, 4] {
        let (mut device, original, _) = fixture();
        establish_shape(&device, &original).await;
        let plan = original.clone();
        match cause {
            2 => device.preset_property(0, PID_HARDWARE_TYPE, &[7; 6]),
            3 => device.preset_property(0, 12, &[0xFF; 2]),
            // Mask version (review fix 4): another device class answers.
            4 => {
                device = SimulatedDevice::with_config(SimulatorConfig {
                    mask_version: 0x0705,
                    serial_number: Some(SERIAL),
                    ..SimulatorConfig::default()
                });
                device.preset_property(0, PID_HARDWARE_TYPE, &[1; 6]);
                device.preset_property(3, PID_PROGRAM_VERSION, &VERSION);
            }
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
            short_read: false,
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
async fn silent_absolute_allocation_failure_is_an_in_procedure_observation() {
    // Amendment 3: the read-back after an allocation record is an observation.
    // The writes go to the plan's absolute addresses and the complete allocation
    // memory is verified at the end; a silently discarded allocation is recorded.
    let (device, plan, preconditions) = fixture();
    establish_shape(&device, &plan).await;
    let transport = Forward {
        device: &device,
        kind: TargetKind::Simulator,
        discard_allocation: true,
        short_read: false,
    };
    let mut journaled = Vec::new();
    let report = run_experimental_property_plan(
        &transport,
        &plan,
        auth(&device),
        AuthorisationPlan::Skip,
        timing(),
        &preconditions,
        &mut |event| {
            if let PropertyJournalEvent::InProcedurePointerObservation(observation) = event {
                journaled.push(observation.clone());
            }
            Ok(())
        },
        &|| false,
    )
    .await
    .unwrap();
    assert_eq!(report.in_procedure_pointer_observations, journaled);
    assert_eq!(journaled.len(), 1);
    assert_eq!(journaled[0].operation_index, 4);
    assert_eq!(journaled[0].observation.expected, [0, 0, 0x3C, 0]);
    assert_ne!(journaled[0].observation.observed, Some([0, 0, 0x3C, 0]));
    assert_eq!(
        device.memory(0x3C00, 4),
        [Some(1), Some(2), Some(3), Some(4)]
    );
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
async fn hardware_release_admits_a_best_effort_plan_with_unwritten_masked_bytes() {
    let (device, original, _) = fixture();
    // Same unwritten-half allocation as the refused case, but a valid release
    // admits the hardware-kind transport; the simulator preserves the unwritten
    // bytes so the full plan succeeds and verifies them.
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
    let transport = Forward {
        device: &device,
        kind: TargetKind::Hardware,
        discard_allocation: false,
        short_read: false,
    };
    let release =
        HardwareRelease::for_hardware(device.address(), &required_release_phrase(device.address()))
            .unwrap();
    let report = run_experimental_property_plan_on_hardware(
        &transport,
        &plan,
        hardware_auth(&device),
        AuthorisationPlan::Skip,
        timing(),
        &preconditions,
        &release,
        &mut |_| Ok(()),
        &|| false,
    )
    .await
    .unwrap();
    assert_eq!(report.execution.data_octets, 4);
    assert_eq!(
        device.memory(0x3C00, 8),
        [
            Some(1),
            Some(2),
            Some(3),
            Some(4),
            Some(5),
            Some(4),
            Some(3),
            Some(2)
        ]
    );
}

#[tokio::test]
async fn hardware_release_for_a_different_target_is_refused_before_connect() {
    let (device, original, _) = fixture();
    let preconditions = ExperimentalPropertyPreconditions::same_application_reinstallation(
        &original, SERIAL, VERSION,
    )
    .unwrap();
    let transport = Forward {
        device: &device,
        kind: TargetKind::Hardware,
        discard_allocation: false,
        short_read: false,
    };
    let other = IndividualAddress::from_raw(0x011A);
    let release = HardwareRelease::for_hardware(other, &required_release_phrase(other)).unwrap();
    let error = run_experimental_property_plan_on_hardware(
        &transport,
        &original,
        hardware_auth(&device),
        AuthorisationPlan::Skip,
        timing(),
        &preconditions,
        &release,
        &mut |_| Ok(()),
        &|| false,
    )
    .await
    .unwrap_err();
    assert!(
        device.seen().is_empty(),
        "a release for another device must refuse before connecting"
    );
    assert!(matches!(
        error.failure,
        PropertySimulationFailure::TargetMismatch { .. }
    ));
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
    assert_eq!(report.recovery.resources[2].pointer, Some([0; 4]));
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
                PropertyJournalEvent::FinalPointerObservations(observations) => {
                    panic!("matching shape produced observations {observations:?}")
                }
                PropertyJournalEvent::InProcedurePointerObservation(observation) => {
                    panic!("matching shape produced observation {observation:?}")
                }
            }
            Ok(())
        },
        &|| false,
    )
    .await
    .unwrap();
    assert_eq!(report.execution.completed.len(), plan.steps().len());
    // Mismatch-free path is unchanged: no observations, recovery available.
    assert!(report.recovery.prestate_observations().is_empty());
    assert!(report.final_pointer_observations.is_empty());
    assert!(report.recovery.recovery_plan(&plan).is_ok());
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

// Rewrites only the final PID_TABLE_REFERENCE answers after LoadCompleted, so a
// fully verified download meets a pointer the source model did not predict.
struct FinalReference<'a> {
    device: &'a SimulatedDevice,
    armed: std::sync::Arc<std::sync::atomic::AtomicBool>,
    property: u8,
    elements: u8,
    /// `None` answers with zero elements, the AL refusal.
    replacement: Option<Vec<u8>>,
    /// First PID 5 octet that arms the rewrite (2 LoadCompleted, 3 allocation).
    arm_on: u8,
}
impl ManagementTransport for FinalReference<'_> {
    fn assigned_address(&self) -> IndividualAddress {
        ManagementTransport::assigned_address(self.device)
    }
    fn subscribe(&self) -> broadcast::Receiver<TunnelEvent> {
        let mut upstream = ManagementTransport::subscribe(self.device);
        let (sender, receiver) = broadcast::channel(256);
        let armed = self.armed.clone();
        let (property, elements, replacement) =
            (self.property, self.elements, self.replacement.clone());
        tokio::spawn(async move {
            loop {
                match upstream.recv().await {
                    Ok(TunnelEvent::Telegram(mut frame)) => {
                        if armed.load(std::sync::atomic::Ordering::SeqCst) {
                            if let ApplicationService::PropertyValueResponse {
                                object_index: 3,
                                property_id,
                                nr_of_elem,
                                data,
                                ..
                            } = &mut frame.service
                            {
                                if *property_id == property {
                                    match &replacement {
                                        Some(bytes) => {
                                            *nr_of_elem = elements;
                                            *data = bytes.clone();
                                        }
                                        None => {
                                            *nr_of_elem = 0;
                                            data.clear();
                                        }
                                    }
                                }
                            }
                        }
                        let _ = sender.send(TunnelEvent::Telegram(frame));
                    }
                    Ok(TunnelEvent::Closed) => {
                        let _ = sender.send(TunnelEvent::Closed);
                        break;
                    }
                    Err(broadcast::error::RecvError::Lagged(_)) => continue,
                    Err(broadcast::error::RecvError::Closed) => break,
                }
            }
        });
        receiver
    }
    fn target_kind(&self) -> TargetKind {
        TargetKind::Simulator
    }
    async fn send_frame(
        &self,
        destination: Destination,
        transport: Tpci,
        service: ApplicationService,
    ) -> Result<(), BusError> {
        if let ApplicationService::PropertyValueWrite {
            property_id: 5,
            data,
            ..
        } = &service
        {
            if data.first() == Some(&self.arm_on) {
                self.armed.store(true, std::sync::atomic::Ordering::SeqCst);
            }
        }
        ManagementTransport::send_frame(self.device, destination, transport, service).await
    }
}

fn recovery_refusal(
    snapshot: &knx_net::commissioning::property_recovery::PropertyRecoverySnapshot,
    plan: &PropertyDownloadPlan,
) -> String {
    match snapshot.recovery_plan(plan) {
        Err(PropertySimulationFailure::Recovery(reason)) => reason,
        other => panic!("original-state recovery must be refused, got {other:?}"),
    }
}

#[tokio::test]
async fn prestate_pointer_mismatch_is_observed_and_download_completes_without_original_recovery() {
    // Issue #30: the device's PID 7 differs from the source model (fresh
    // simulator answers 0, the source record says 3C00h). Allowed by owner ruling.
    let (device, plan, preconditions) = fixture();
    let mut acquired = None;
    let report = run_experimental_property_plan(
        &device,
        &plan,
        auth(&device),
        AuthorisationPlan::Skip,
        timing(),
        &preconditions,
        &mut |event| {
            if let PropertyJournalEvent::RecoveryAcquired(snapshot) = event {
                acquired = Some(snapshot.clone());
            }
            Ok(())
        },
        &|| false,
    )
    .await
    .expect("a differing prestate pointer is an observation, not a refusal");
    let acquired = acquired.expect("durable snapshot precedes mutation");
    assert_eq!(acquired, report.recovery);
    assert_eq!(report.recovery.resources[0].object, 3);
    assert_eq!(report.recovery.resources[0].pointer, Some([0; 4]));
    assert_eq!(report.recovery.resources[0].state, LoadState::Loaded);
    assert_eq!(report.recovery.regions[0].data, [9, 8, 7, 6]);
    assert_eq!(
        report.recovery.prestate_observations(),
        [PropertyPointerObservation {
            object: 3,
            expected: [0, 0, 0x3C, 0],
            observed: Some([0; 4]),
            state: LoadState::Loaded,
        }]
    );
    assert!(report.final_pointer_observations.is_empty());
    let reason = recovery_refusal(&report.recovery, &plan);
    assert!(
        reason.contains("original-state recovery unavailable"),
        "{reason}"
    );
    assert!(
        reason.contains("object 3: expected PID_TABLE_REFERENCE 00003c00, observed 00000000"),
        "{reason}"
    );
    assert_eq!(
        device.memory(0x3C00, 4),
        [Some(1), Some(2), Some(3), Some(4)]
    );
    assert_eq!(device.load_state(ObjectIndex::new(3)), LoadState::Loaded);
    assert!(matches!(device.seen().last(), Some(Seen::Disconnect)));
}

#[tokio::test]
async fn non_loaded_prestate_is_observed_and_download_completes_without_original_recovery() {
    let (device, plan, preconditions) = fixture();
    establish_shape(&device, &plan).await;
    device.preset_load_state(ObjectIndex::new(3), LoadState::Error);
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
    .expect("a non-Loaded prestate is an observation, not a refusal");
    assert_eq!(report.recovery.resources[0].state, LoadState::Error);
    assert_eq!(report.recovery.resources[0].pointer, Some([0, 0, 0x3C, 0]));
    assert_eq!(
        report.recovery.prestate_observations()[0].state,
        LoadState::Error
    );
    let reason = recovery_refusal(&report.recovery, &plan);
    assert!(
        reason.contains("original-state recovery unavailable"),
        "{reason}"
    );
    assert_eq!(
        device.memory(0x3C00, 4),
        [Some(1), Some(2), Some(3), Some(4)]
    );
    assert_eq!(device.load_state(ObjectIndex::new(3)), LoadState::Loaded);
}

#[tokio::test]
async fn final_pointer_difference_after_verified_download_is_an_observation() {
    let (device, plan, preconditions) = fixture();
    establish_shape(&device, &plan).await;
    let transport = FinalReference {
        device: &device,
        armed: Default::default(),
        property: 7,
        elements: 1,
        replacement: Some(vec![0, 0, 0x3D, 0]),
        arm_on: 2,
    };
    let mut journaled = Vec::new();
    let report = run_experimental_property_plan(
        &transport,
        &plan,
        auth(&device),
        AuthorisationPlan::Skip,
        timing(),
        &preconditions,
        &mut |event| {
            if let PropertyJournalEvent::FinalPointerObservations(observations) = event {
                journaled.extend_from_slice(observations);
            }
            Ok(())
        },
        &|| false,
    )
    .await
    .expect("memory, Loaded and identity verified; PID 7 alone is no verdict");
    assert_eq!(report.execution.completed.len(), plan.steps().len());
    let expected = [PropertyPointerObservation {
        object: 3,
        expected: [0, 0, 0x3C, 0],
        observed: Some([0, 0, 0x3D, 0]),
        state: LoadState::Loaded,
    }];
    assert_eq!(report.final_pointer_observations, expected);
    assert_eq!(journaled, expected, "observation is journaled durably");
    assert!(report.recovery.prestate_observations().is_empty());
    // No prestate mismatch: original-state recovery stays available.
    assert!(report.recovery.recovery_plan(&plan).is_ok());
    assert_eq!(
        device.memory(0x3C00, 4),
        [Some(1), Some(2), Some(3), Some(4)]
    );
}

#[tokio::test]
async fn final_memory_verification_stays_hard_after_a_prestate_mismatch() {
    // Memory verification is proof; a pointer observation never softens it.
    let (device, plan, preconditions) = fixture();
    let transport = Forward {
        device: &device,
        kind: TargetKind::Simulator,
        discard_allocation: false,
        short_read: false,
    };
    let error = run_experimental_property_plan(
        &transport,
        &plan,
        auth(&device),
        AuthorisationPlan::Skip,
        timing(),
        &preconditions,
        &mut |event| {
            if let PropertyJournalEvent::OperationStarting {
                operation_index: 7, ..
            } = event
            {
                device.preset_memory(0x3C02, &[0xEE]);
            }
            Ok(())
        },
        &|| false,
    )
    .await
    .unwrap_err();
    assert!(error.mutation_may_have_started);
    match error.failure {
        PropertySimulationFailure::Recovery(reason) => {
            assert!(reason.contains("full allocation verification"), "{reason}")
        }
        other => panic!("unexpected failure {other:?}"),
    }
}

#[tokio::test]
async fn serial_mismatch_is_recorded_and_the_download_proceeds() {
    // Owner amendment: device type and class admit; the serial is an observation.
    let (device, plan, preconditions) = fixture();
    establish_shape(&device, &plan).await;
    device.preset_property(0, 11, &[0, 2, 9, 9, 9, 9]);
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
    .expect("a differing serial is recorded, not refused");
    assert_eq!(report.execution.completed.len(), plan.steps().len());
    assert_eq!(
        device.memory(0x3C00, 4),
        [Some(1), Some(2), Some(3), Some(4)]
    );
    assert_eq!(report.recovery.observed_serial, [0, 2, 9, 9, 9, 9]);
    assert_eq!(
        report.recovery.serial_observation(),
        Some(PropertySerialObservation {
            expected: SERIAL,
            observed: [0, 2, 9, 9, 9, 9],
        })
    );
    // Recovery stays available, bound to the serial actually read.
    assert!(report.recovery.recovery_plan(&plan).is_ok());
}

#[tokio::test]
async fn recovery_onto_a_different_serial_refuses_before_any_write() {
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
    // Another physical device of the same type now answers at the target.
    device.preset_property(0, 11, &[0, 2, 9, 9, 9, 9]);
    let before = device.seen().len();
    let error = restore_experimental_property_plan(
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
    .unwrap_err();
    assert!(!error.mutation_may_have_started);
    match error.failure {
        PropertySimulationFailure::Recovery(reason) => {
            assert!(reason.contains("serial number differs"), "{reason}")
        }
        other => panic!("unexpected failure {other:?}"),
    }
    assert!(!device.seen()[before..]
        .iter()
        .any(|seen| matches!(seen, Seen::MemoryWrite { .. } | Seen::PropertyWrite { .. })));
    assert_eq!(
        device.memory(0x3C00, 4),
        [Some(1), Some(2), Some(3), Some(4)]
    );
}

#[tokio::test]
async fn different_prior_program_version_is_recorded_and_the_download_proceeds() {
    // Owner amendment 2: the procedure unloads and writes the identity itself.
    let (device, plan, preconditions) = fixture();
    establish_shape(&device, &plan).await;
    device.preset_property(3, PID_PROGRAM_VERSION, &[0, 2, 0x12, 0x34, 9]);
    let report = run_experimental_property_plan(
        &device,
        &plan,
        auth(&device),
        AuthorisationPlan::Skip,
        timing(),
        &preconditions,
        &mut |event| {
            // The simulator does not derive PID 13 from a 03 02 task record, so
            // model the identity the real procedure writes before LoadCompleted.
            if let PropertyJournalEvent::OperationStarting {
                operation_index: 6, ..
            } = event
            {
                device.preset_property(3, PID_PROGRAM_VERSION, &VERSION);
            }
            Ok(())
        },
        &|| false,
    )
    .await
    .expect("a different prior application is recorded, not refused");
    assert_eq!(report.execution.completed.len(), plan.steps().len());
    assert_eq!(
        device.memory(0x3C00, 4),
        [Some(1), Some(2), Some(3), Some(4)]
    );
    let reason = recovery_refusal(&report.recovery, &plan);
    assert!(
        reason.contains("original-state recovery unavailable") && reason.contains("application"),
        "{reason}"
    );
    assert_eq!(
        report.recovery.program_version_observation(),
        Some(PropertyProgramVersionObservation {
            expected: VERSION,
            observed: vec![0, 2, 0x12, 0x34, 9],
        })
    );
    assert!(
        reason.contains("program version: expected 0002123401, observed 0002123409"),
        "{reason}"
    );
}

#[tokio::test]
async fn final_program_version_mismatch_still_fails_verification() {
    let (device, plan, preconditions) = fixture();
    establish_shape(&device, &plan).await;
    let transport = FinalReference {
        device: &device,
        armed: Default::default(),
        property: PID_PROGRAM_VERSION,
        elements: 1,
        replacement: Some(vec![0, 2, 0x12, 0x34, 9]),
        arm_on: 2,
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
    assert!(error.mutation_may_have_started);
    match error.failure {
        PropertySimulationFailure::Recovery(reason) => {
            assert!(
                reason.contains("final application identity differs"),
                "{reason}"
            )
        }
        other => panic!("unexpected failure {other:?}"),
    }
}

#[tokio::test]
async fn in_procedure_pointer_difference_after_allocation_is_an_observation() {
    // Amendment 3: ETS-programmed devices report a PID 7 the source model does
    // not predict. Memory writes use the plan's addresses and are verified.
    let (device, plan, preconditions) = fixture();
    establish_shape(&device, &plan).await;
    let transport = FinalReference {
        device: &device,
        armed: Default::default(),
        property: 7,
        elements: 1,
        replacement: Some(vec![0, 0, 0x3D, 0]),
        arm_on: 3,
    };
    let report = run_experimental_property_plan(
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
    .expect("in-procedure PID 7 read-back is an observation");
    assert_eq!(report.execution.completed.len(), plan.steps().len());
    assert_eq!(
        device.memory(0x3C00, 4),
        [Some(1), Some(2), Some(3), Some(4)]
    );
    assert!(report.recovery.recovery_plan(&plan).is_ok());
    assert_eq!(
        report.in_procedure_pointer_observations,
        [PropertyInProcedurePointerObservation {
            operation_index: 4,
            observation: PropertyPointerObservation {
                object: 3,
                expected: [0, 0, 0x3C, 0],
                observed: Some([0, 0, 0x3D, 0]),
                state: LoadState::Loading,
            },
        }]
    );
    assert_eq!(report.final_pointer_observations.len(), 1);
}

#[tokio::test]
async fn refused_pointer_reads_are_observations_in_every_phase() {
    let (device, plan, preconditions) = fixture();
    establish_shape(&device, &plan).await;
    let transport = FinalReference {
        device: &device,
        armed: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true)),
        property: 7,
        elements: 1,
        replacement: None,
        arm_on: 0xFF,
    };
    let report = run_experimental_property_plan(
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
    .expect("a refused PID 7 read is recorded, not a failure");
    assert_eq!(
        device.memory(0x3C00, 4),
        [Some(1), Some(2), Some(3), Some(4)]
    );
    let reason = recovery_refusal(&report.recovery, &plan);
    assert!(reason.contains("refused"), "{reason}");
    assert_eq!(report.recovery.resources[0].pointer, None);
    assert_eq!(report.recovery.prestate_observations()[0].observed, None);
    assert_eq!(
        report.in_procedure_pointer_observations[0]
            .observation
            .observed,
        None
    );
    assert_eq!(report.final_pointer_observations[0].observed, None);
    assert!(reason.contains("observed refused"), "{reason}");
}

#[tokio::test]
async fn pid7_answer_of_another_width_is_refused_before_any_write_with_the_raw_bytes() {
    // Issue #30: a real device answered PID 7 with a width other than four octets.
    // The refusal before the first write stays; it now carries what was observed.
    for (elements, answer) in [
        (1, vec![]),
        (1, vec![0x3C]),
        (1, vec![0x3C, 0x00]),
        (2, vec![0x3C, 0x00]),
        (1, vec![0x00, 0x3C, 0x00]),
        (1, vec![0x00, 0x00, 0x3C, 0x00, 0x01]),
    ] {
        let (device, plan, preconditions) = fixture();
        establish_shape(&device, &plan).await;
        let transport = FinalReference {
            device: &device,
            armed: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true)),
            property: 7,
            elements,
            replacement: Some(answer.clone()),
            arm_on: 0xFF,
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
        assert!(!error.mutation_may_have_started, "{answer:?}");
        let PropertySimulationFailure::Recovery(reason) = &error.failure else {
            panic!("expected a recovery refusal, got {:?}", error.failure);
        };
        let hex: String = answer.iter().map(|o| format!("{o:02x}")).collect();
        assert!(reason.contains("not four octets"), "{reason}");
        assert!(reason.contains("object 3"), "{reason}");
        assert!(reason.contains(&format!("{elements} element")), "{reason}");
        assert!(
            reason.contains(&format!("{} octet", answer.len())),
            "{reason}"
        );
        if answer.is_empty() {
            assert!(reason.ends_with("(none)"), "{reason}");
        } else {
            assert!(reason.contains(&hex), "{reason}");
        }
        assert!(device.seen()[before..]
            .iter()
            .all(|seen| !matches!(seen, Seen::MemoryWrite { .. } | Seen::PropertyWrite { .. })));
        assert!(matches!(device.seen().last(), Some(Seen::Disconnect)));
    }
}

#[tokio::test]
async fn placeholder_serials_download_but_leave_no_recovery() {
    for serial in [[0; 6], [0xFF; 6]] {
        let (device, plan, preconditions) = fixture();
        establish_shape(&device, &plan).await;
        device.preset_property(0, 11, &serial);
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
        .expect("a placeholder serial is no admission check");
        assert_eq!(report.recovery.observed_serial, serial);
        let reason = recovery_refusal(&report.recovery, &plan);
        assert!(
            reason.contains("original-state recovery unavailable") && reason.contains("serial"),
            "{reason}"
        );
    }
}

#[tokio::test]
async fn final_pointer_observations_are_durable_before_the_verify_mode_restore_write() {
    // Review fix 1: the journal barrier precedes the PID 14 restore, so a failed
    // barrier leaves the VerifyMode bit as the procedure set it.
    let (device, plan, preconditions) = fixture();
    establish_shape(&device, &plan).await;
    let transport = FinalReference {
        device: &device,
        armed: Default::default(),
        property: 7,
        elements: 1,
        replacement: Some(vec![0, 0, 0x3D, 0]),
        arm_on: 2,
    };
    let before = device.seen().len();
    let error = run_experimental_property_plan(
        &transport,
        &plan,
        auth(&device),
        AuthorisationPlan::Skip,
        timing(),
        &preconditions,
        &mut |event| match event {
            PropertyJournalEvent::OperationStarting {
                operation_index: 7, ..
            } => Ok(()),
            PropertyJournalEvent::OperationStarting { .. } => Ok(()),
            PropertyJournalEvent::OperationCompleted { .. } => Ok(()),
            PropertyJournalEvent::RecoveryAcquired(_) => Ok(()),
            PropertyJournalEvent::InProcedurePointerObservation(_) => Ok(()),
            PropertyJournalEvent::FinalPointerObservations(_) => {
                Err("durable store failed".to_string())
            }
        },
        &|| false,
    )
    .await
    .unwrap_err();
    assert!(matches!(
        error.failure,
        PropertySimulationFailure::Persistence(_)
    ));
    let controls: Vec<_> = device.seen()[before..]
        .iter()
        .filter(|seen| {
            matches!(
                seen,
                Seen::PropertyWrite {
                    object_index: 0,
                    property_id: 14,
                    ..
                }
            )
        })
        .cloned()
        .collect();
    assert_eq!(
        controls.len(),
        1,
        "only the VerifyMode setup write; no restore before the observation is durable"
    );
}

// Short-answers memory reads only before the first property write, so the
// prestate capture is incomplete while final verification reads normally.
struct PrestateShortRead<'a> {
    device: &'a SimulatedDevice,
    written: std::sync::atomic::AtomicBool,
}
impl ManagementTransport for PrestateShortRead<'_> {
    fn assigned_address(&self) -> IndividualAddress {
        ManagementTransport::assigned_address(self.device)
    }
    fn subscribe(&self) -> broadcast::Receiver<TunnelEvent> {
        ManagementTransport::subscribe(self.device)
    }
    fn target_kind(&self) -> TargetKind {
        TargetKind::Simulator
    }
    async fn send_frame(
        &self,
        destination: Destination,
        transport: Tpci,
        mut service: ApplicationService,
    ) -> Result<(), BusError> {
        use std::sync::atomic::Ordering::SeqCst;
        match &mut service {
            ApplicationService::PropertyValueWrite { .. } => self.written.store(true, SeqCst),
            ApplicationService::MemoryRead { number, .. }
            | ApplicationService::UserMemoryRead { number, .. }
                if *number > 1 && !self.written.load(SeqCst) =>
            {
                *number -= 1
            }
            _ => {}
        }
        ManagementTransport::send_frame(self.device, destination, transport, service).await
    }
}

#[tokio::test]
async fn unreadable_prestate_memory_is_observed_and_leaves_no_recovery() {
    let (device, plan, preconditions) = fixture();
    establish_shape(&device, &plan).await;
    let transport = PrestateShortRead {
        device: &device,
        written: Default::default(),
    };
    let report = run_experimental_property_plan(
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
    .expect("an unreadable prestate allocation is an observation");
    assert_eq!(
        device.memory(0x3C00, 4),
        [Some(1), Some(2), Some(3), Some(4)]
    );
    let reason = recovery_refusal(&report.recovery, &plan);
    assert!(reason.contains("prestate memory unreadable"), "{reason}");
    assert_eq!(report.recovery.unreadable_allocations, [0]);
    assert_eq!(
        report.recovery.allocations[0].data, [0; 4],
        "placeholder, not a backup"
    );
}

// Refuses (empty A_Memory_Response) or drops memory reads before the first
// property write; final verification afterwards reads normally.
struct PrestateMemory<'a> {
    device: &'a SimulatedDevice,
    drop_reads: bool,
    written: std::sync::Arc<std::sync::atomic::AtomicBool>,
}
impl ManagementTransport for PrestateMemory<'_> {
    fn assigned_address(&self) -> IndividualAddress {
        ManagementTransport::assigned_address(self.device)
    }
    fn subscribe(&self) -> broadcast::Receiver<TunnelEvent> {
        let mut upstream = ManagementTransport::subscribe(self.device);
        let (sender, receiver) = broadcast::channel(256);
        let written = self.written.clone();
        tokio::spawn(async move {
            loop {
                match upstream.recv().await {
                    Ok(TunnelEvent::Telegram(mut frame)) => {
                        if !written.load(std::sync::atomic::Ordering::SeqCst) {
                            if let ApplicationService::MemoryResponse { data, .. } =
                                &mut frame.service
                            {
                                data.clear();
                            }
                        }
                        let _ = sender.send(TunnelEvent::Telegram(frame));
                    }
                    Ok(TunnelEvent::Closed) => {
                        let _ = sender.send(TunnelEvent::Closed);
                        break;
                    }
                    Err(broadcast::error::RecvError::Lagged(_)) => continue,
                    Err(broadcast::error::RecvError::Closed) => break,
                }
            }
        });
        receiver
    }
    fn target_kind(&self) -> TargetKind {
        TargetKind::Simulator
    }
    async fn send_frame(
        &self,
        destination: Destination,
        transport: Tpci,
        service: ApplicationService,
    ) -> Result<(), BusError> {
        use std::sync::atomic::Ordering::SeqCst;
        match &service {
            ApplicationService::PropertyValueWrite { .. } => self.written.store(true, SeqCst),
            ApplicationService::MemoryRead { .. }
                if self.drop_reads && !self.written.load(SeqCst) =>
            {
                return Ok(());
            }
            _ => {}
        }
        ManagementTransport::send_frame(self.device, destination, transport, service).await
    }
}

#[tokio::test]
async fn refused_prestate_memory_read_is_observed_and_the_download_continues() {
    let (device, plan, preconditions) = fixture();
    establish_shape(&device, &plan).await;
    let transport = PrestateMemory {
        device: &device,
        drop_reads: false,
        written: Default::default(),
    };
    let report = run_experimental_property_plan(
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
    .expect("a refused prestate memory read is an observation");
    assert_eq!(report.recovery.unreadable_allocations, [0]);
    assert_eq!(
        device.memory(0x3C00, 4),
        [Some(1), Some(2), Some(3), Some(4)]
    );
    let reason = recovery_refusal(&report.recovery, &plan);
    assert!(reason.contains("prestate memory unreadable"), "{reason}");
}

#[tokio::test]
async fn prestate_memory_read_timeout_stays_an_error_before_any_write() {
    let (device, plan, preconditions) = fixture();
    establish_shape(&device, &plan).await;
    let transport = PrestateMemory {
        device: &device,
        drop_reads: true,
        written: Default::default(),
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
    assert!(!error.mutation_may_have_started);
    assert!(
        matches!(error.failure, PropertySimulationFailure::Session(_)),
        "{:?}",
        error.failure
    );
    assert!(!device.seen()[before..]
        .iter()
        .any(|seen| matches!(seen, Seen::MemoryWrite { .. } | Seen::PropertyWrite { .. })));
    assert_eq!(
        device.memory(0x3C00, 4),
        [Some(9), Some(8), Some(7), Some(6)]
    );
}
