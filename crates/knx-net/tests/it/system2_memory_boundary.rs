//! System 2 refuses an allocation reaching FFFFh before any frame; below it one service rules.

use super::property_recovery::{auth, timing};
use knx_core::commissioning::absolute_load_control::{absolute_data, AbsolutePropertySegment};
use knx_core::commissioning::authorisation::AuthorisationPlan;
use knx_core::commissioning::load_control::event_payload;
use knx_core::commissioning::load_control_memory::SegmentMemoryType;
use knx_core::commissioning::load_state::{LoadEvent, LoadState, MaskVersion};
use knx_core::commissioning::memory::MemoryService;
use knx_core::commissioning::properties::{ObjectIndex, PID_HARDWARE_TYPE, PID_PROGRAM_VERSION};
use knx_core::commissioning::property_download::{
    PropertyDownloadPlan, PropertyDownloadStep, PropertyOperation as Op,
};
use knx_net::commissioning::property_download::{
    run_experimental_property_plan, run_simulated_property_plan, PropertySimulationFailure,
};
use knx_net::commissioning::property_recovery::ExperimentalPropertyPreconditions;
use knx_net::commissioning::simulator::{Seen, SimulatedDevice, SimulatorConfig};

const SERIAL: [u8; 6] = [0, 2, 1, 2, 3, 4];
const VERSION: [u8; 5] = [0, 2, 0x12, 0x34, 1];

pub(super) fn system2_device() -> SimulatedDevice {
    let device = SimulatedDevice::with_config(SimulatorConfig {
        mask_version: 0x0025,
        serial_number: Some(SERIAL),
        ..SimulatorConfig::default()
    });
    device.preset_property(0, PID_HARDWARE_TYPE, &[1; 6]);
    device.preset_property(3, PID_PROGRAM_VERSION, &VERSION);
    device.preset_load_state(ObjectIndex::new(3), LoadState::Loaded);
    device
}

/// One absolute allocation of object 3 at `start`, one data write at `write_at` inside it.
pub(super) fn one_allocation_plan(
    device: &SimulatedDevice,
    start: u16,
    length: u16,
    write_at: u16,
    data: Vec<u8>,
) -> (PropertyDownloadPlan, ExperimentalPropertyPreconditions) {
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
                    start,
                    length,
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
                    address: write_at,
                    data,
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
    (plan, preconditions)
}

/// Every memory frame the simulator saw, as (read or write, service).
fn memory_frames(device: &SimulatedDevice) -> Vec<(&'static str, MemoryService)> {
    device
        .seen()
        .into_iter()
        .filter_map(|entry| match entry {
            Seen::MemoryRead { service, .. } => Some(("read", service)),
            Seen::MemoryWrite { service, .. } => Some(("write", service)),
            _ => None,
        })
        .collect()
}

#[tokio::test]
async fn an_allocation_ending_at_fffeh_writes_reads_and_verifies_with_a_memory_only() {
    let device = system2_device();
    // Base FFF0h plus 14 allocated octets is FFFEh; the plan writes four of them.
    let (plan, preconditions) = one_allocation_plan(&device, 0xFFF0, 14, 0xFFF4, vec![1, 2, 3, 4]);
    run_simulated_property_plan(
        &device,
        &plan,
        auth(&device),
        AuthorisationPlan::Skip,
        timing(),
    )
    .await
    .expect("the source shape is establishable below FFFFh");
    device.preset_memory(0xFFF0, &[0x5A; 14]);
    let before = device.seen().len();
    run_experimental_property_plan(
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
    .expect("prestate read, write and final verification all succeed");
    let frames = memory_frames(&device);
    let experimental: Vec<_> = device.seen()[before..]
        .iter()
        .filter(|entry| matches!(entry, Seen::MemoryRead { .. } | Seen::MemoryWrite { .. }))
        .cloned()
        .collect();
    assert!(
        experimental
            .iter()
            .any(|e| matches!(e, Seen::MemoryRead { .. })),
        "prestate and verification reads happened"
    );
    assert!(experimental
        .iter()
        .any(|e| matches!(e, Seen::MemoryWrite { .. })));
    assert!(
        frames
            .iter()
            .all(|(_, service)| *service == MemoryService::Memory),
        "{frames:?}"
    );
}

#[tokio::test]
async fn an_allocation_reaching_ffffh_or_beyond_is_refused_before_any_frame() {
    for (length, end) in [(15u16, 0xFFFFu32), (16, 0x1_0000)] {
        let device = system2_device();
        let (plan, preconditions) =
            one_allocation_plan(&device, 0xFFF0, length, 0xFFF0, vec![1, 2, 3, 4]);
        let error = run_experimental_property_plan(
            &device,
            &plan,
            auth(&device),
            AuthorisationPlan::Skip,
            timing(),
            &preconditions,
            &mut |event| panic!("no journal event before the refusal: {event:?}"),
            &|| false,
        )
        .await
        .expect_err("refused");
        assert!(
            matches!(
                error.failure,
                PropertySimulationFailure::MemoryServiceBoundary {
                    object: 3,
                    address: 0xFFF0,
                    length: l,
                } if l == usize::from(length)
            ),
            "end {end:#x}: {:?}",
            error.failure
        );
        assert_eq!(error.operation_index, None, "end {end:#x}");
        assert!(!error.mutation_may_have_started);
        assert!(error.completed.is_empty());
        assert_eq!(device.seen(), [], "end {end:#x}: not a single frame");
        // The plain simulator entry shares the executor and refuses the same way.
        let error = run_simulated_property_plan(
            &device,
            &plan,
            auth(&device),
            AuthorisationPlan::Skip,
            timing(),
        )
        .await
        .expect_err("refused");
        assert!(matches!(
            error.failure,
            PropertySimulationFailure::MemoryServiceBoundary { .. }
        ));
        assert_eq!(device.seen(), []);
    }
}
