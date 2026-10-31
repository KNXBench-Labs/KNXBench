//! Independent synthetic OLD System-2 device state, shared by App and Server tests.
//!
//! Included by `#[path]` from Server tests too, so every path is absolute.
use knx_app::property_validation_bundle::PreparedBundle;
use knx_core::commissioning::{
    load_state::LoadState,
    properties::{PID_HARDWARE_TYPE, PID_MANUFACTURER_ID, PID_PROGRAM_VERSION},
};
use knx_core::IndividualAddress;
use knx_net::commissioning::simulator::{
    SimulatedAbsoluteAllocation, SimulatedDevice, SimulatedSystem2Object,
    SimulatedSystem2ObjectState, SimulatedTaskControl2, SimulatedTaskSegment, SimulatorConfig,
};

pub(crate) fn synthetic_old_device(
    bundle: &PreparedBundle,
    mut config: SimulatorConfig,
) -> SimulatedDevice {
    // Independent OLD, not produced by NEW execution; no real hardware history implied.
    config.mask_version = 0x0025;
    config.serial_number = Some(bundle.binding.serial);
    let device = SimulatedDevice::with_config_at(IndividualAddress::from_raw(0x1234), config);
    device.preset_property(
        0,
        knx_core::commissioning::properties::PID_MANUFACTURER_ID,
        &[0, 0],
    );
    device.preset_property(0, PID_HARDWARE_TYPE, &[240, 0, 0, 0, 0, 1]);
    device.preset_property(3, PID_PROGRAM_VERSION, &[0, 0, 0, 1, 1]);
    for (owner, extents, pointer, task_control) in [
        (
            SimulatedSystem2Object::AddressTable,
            vec![(278, 16)],
            278,
            false,
        ),
        (
            SimulatedSystem2Object::AssociationTable,
            vec![(608, 16)],
            608,
            false,
        ),
        (
            SimulatedSystem2Object::Application,
            vec![(1024, 8), (15360, 8)],
            0,
            true,
        ),
    ] {
        let allocations = extents
            .into_iter()
            .map(|(start, length)| {
                device.preset_memory(u32::from(start), &vec![0xa5; usize::from(length)]);
                SimulatedAbsoluteAllocation {
                    subtype: 0,
                    start,
                    length,
                    access: 0x33,
                    memory_type: 3,
                    attributes: 0,
                }
            })
            .collect();
        device.preset_system2_object(
            owner,
            SimulatedSystem2ObjectState {
                load_state: LoadState::Loaded,
                table_reference: pointer,
                allocations,
                task_segment: Some(SimulatedTaskSegment {
                    start: pointer as u16,
                    pei_type: 0,
                    manufacturer: 0,
                    application: 1,
                    version: 1,
                }),
                task_control_2: task_control.then_some(SimulatedTaskControl2 {
                    callback: 0,
                    communication_objects: 1024,
                    segment_1: 0,
                    segment_2: 0,
                }),
            },
        );
    }
    device
}

/// The configured fixture's identity (manufacturer 1, application 9205h v4) on that OLD state.
pub(crate) fn configure_old_device(device: &SimulatedDevice) {
    device.preset_property(0, PID_MANUFACTURER_ID, &[0, 1]);
    device.preset_property(0, PID_HARDWARE_TYPE, &[0, 1, 0, 0, 0, 91]);
    device.preset_property(3, PID_PROGRAM_VERSION, &[0, 1, 146, 5, 4]);
    for (object, mut state) in [
        SimulatedSystem2Object::AddressTable,
        SimulatedSystem2Object::AssociationTable,
        SimulatedSystem2Object::Application,
    ]
    .into_iter()
    .zip(device.system2_snapshot().objects)
    {
        if let Some(task) = state.task_segment.as_mut() {
            task.manufacturer = 1;
            task.application = 0x9205;
            task.version = 4;
        }
        device.preset_system2_object(object, state);
    }
}
