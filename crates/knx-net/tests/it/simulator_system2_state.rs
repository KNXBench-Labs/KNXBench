//! Independent decoded-service oracle, not hardware/firmware conformance evidence.

use knx_core::commissioning::load_state::LoadState;
use knx_net::cemi::{ApplicationService, Destination, Tpci};
use knx_net::commissioning::simulator::{
    SimulatedAbsoluteAllocation, SimulatedDevice, SimulatedSystem2Object,
    SimulatedSystem2ObjectState, SimulatedTaskControl2, SimulatedTaskSegment, SimulatorConfig,
};
use knx_net::management::ManagementTransport;

fn device() -> SimulatedDevice {
    SimulatedDevice::with_config(SimulatorConfig {
        mask_version: 0x0025,
        ..SimulatorConfig::default()
    })
}

async fn control(device: &SimulatedDevice, object: u8, data: [u8; 10]) {
    device
        .send_frame(
            Destination::Individual(device.address()),
            Tpci::UnnumberedData,
            ApplicationService::PropertyValueWrite {
                object_index: object,
                property_id: 5,
                nr_of_elem: 1,
                start_index: 1,
                data: data.to_vec(),
            },
        )
        .await
        .unwrap();
}

#[tokio::test]
async fn records_multiple_absolute_allocations_with_independent_raw_fields() {
    let device = device();
    control(&device, 3, [1, 0, 0, 0, 0, 0, 0, 0, 0, 0]).await;
    control(&device, 3, [3, 0, 0x31, 0x20, 0, 7, 0xA6, 3, 0x80, 0]).await;
    control(&device, 3, [3, 1, 0x00, 0xD1, 0, 3, 0x72, 1, 0, 0]).await;
    let snapshot = device.system2_snapshot();
    assert_eq!(
        snapshot.objects[2].allocations,
        vec![
            SimulatedAbsoluteAllocation {
                subtype: 0,
                start: 0x3120,
                length: 7,
                access: 0xA6,
                memory_type: 3,
                attributes: 0x80,
            },
            SimulatedAbsoluteAllocation {
                subtype: 1,
                start: 0x00D1,
                length: 3,
                access: 0x72,
                memory_type: 1,
                attributes: 0,
            },
        ],
        "both owned extents and every declared field must persist"
    );
    assert_eq!(snapshot.objects[2].table_reference, 0x00D1);
    assert_eq!(snapshot.objects[2].load_state, LoadState::Loading);
    assert_eq!(snapshot.objects[0], SimulatedSystem2ObjectState::default());
    // Typed names, not arbitrary object indices, bound fixture ownership.
    assert_eq!(SimulatedSystem2Object::Application.index(), 3);
}

fn old_object() -> SimulatedSystem2ObjectState {
    SimulatedSystem2ObjectState {
        load_state: LoadState::Loaded,
        table_reference: 0x2200,
        allocations: vec![SimulatedAbsoluteAllocation {
            subtype: 0,
            start: 0x2200,
            length: 4,
            access: 0x36,
            memory_type: 3,
            attributes: 0,
        }],
        task_segment: Some(SimulatedTaskSegment {
            start: 0x2200,
            pei_type: 0x19,
            manufacturer: 0x1234,
            application: 0x5678,
            version: 0x9A,
        }),
        task_control_2: Some(SimulatedTaskControl2 {
            callback: 0x1234,
            communication_objects: 0x2345,
            segment_1: 0x3456,
            segment_2: 0x4567,
        }),
    }
}

#[tokio::test]
async fn task_segment_and_control_words_are_persistent_for_each_bounded_owner() {
    let device = device();
    for object in 1..=3 {
        control(&device, object, [1, 0, 0, 0, 0, 0, 0, 0, 0, 0]).await;
        control(
            &device,
            object,
            [3, 2, 0x22, 0, 0x19, 0x12, 0x34, 0x56, 0x78, 0x9A],
        )
        .await;
        control(
            &device,
            object,
            [3, 5, 0x12, 0x34, 0x23, 0x45, 0x34, 0x56, 0x45, 0x67],
        )
        .await;
        let state = &device.system2_snapshot().objects[usize::from(object - 1)];
        assert_eq!(
            state.task_segment,
            old_object().task_segment,
            "subtype2 identity is not just a pointer"
        );
        assert_eq!(state.table_reference, 0x2200);
    }
}

#[tokio::test]
async fn task_control_2_persists_four_independent_words_without_allocating() {
    let device = device();
    for object in 1..=3 {
        control(&device, object, [1, 0, 0, 0, 0, 0, 0, 0, 0, 0]).await;
        control(
            &device,
            object,
            [3, 5, 0x12, 0x34, 0x23, 0x45, 0x34, 0x56, 0x45, 0x67],
        )
        .await;
        let state = &device.system2_snapshot().objects[usize::from(object - 1)];
        assert_eq!(
            state.task_control_2,
            old_object().task_control_2,
            "subtype5 must store all four independent words"
        );
        assert!(state.allocations.is_empty());
        assert_eq!(state.table_reference, 0);
    }
}

#[tokio::test]
async fn typed_old_fixture_unload_removes_only_its_owned_resource_metadata() {
    let device = device();
    for object in [
        SimulatedSystem2Object::AddressTable,
        SimulatedSystem2Object::AssociationTable,
        SimulatedSystem2Object::Application,
    ] {
        device.preset_system2_object(object, old_object());
    }
    device.preset_memory(0x2200, &[0xAA, 0xBB, 0xCC, 0xDD]);
    let before = device.system2_snapshot();
    assert!(before.objects.iter().all(|state| state == &old_object()));
    control(&device, 2, [4, 0, 0, 0, 0, 0, 0, 0, 0, 0]).await;
    let after = device.system2_snapshot();
    assert_eq!(
        after.objects[1],
        SimulatedSystem2ObjectState::default(),
        "Unload must erase allocation ownership and task metadata"
    );
    assert_eq!(after.objects[0], before.objects[0]);
    assert_eq!(after.objects[2], before.objects[2]);
    assert_eq!(
        after.memory, before.memory,
        "default compatibility fixture retains physical bytes, not ownership"
    );
}

async fn write_memory(device: &SimulatedDevice, address: u16, data: &[u8]) {
    device
        .send_frame(
            Destination::Individual(device.address()),
            Tpci::UnnumberedData,
            ApplicationService::MemoryWrite {
                address,
                data: data.to_vec(),
            },
        )
        .await
        .unwrap();
}

async fn reinstall_old(device: &SimulatedDevice, object: u8) {
    control(device, object, [4, 0, 0, 0, 0, 0, 0, 0, 0, 0]).await;
    control(device, object, [1, 0, 0, 0, 0, 0, 0, 0, 0, 0]).await;
    control(device, object, [3, 0, 0x22, 0, 0, 4, 0x36, 3, 0, 0]).await;
    control(
        device,
        object,
        [3, 2, 0x22, 0, 0x19, 0x12, 0x34, 0x56, 0x78, 0x9A],
    )
    .await;
    control(
        device,
        object,
        [3, 5, 0x12, 0x34, 0x23, 0x45, 0x34, 0x56, 0x45, 0x67],
    )
    .await;
    // The last byte is deliberately not rewritten: it represents a preserved mask.
    write_memory(device, 0x2200, &[0xAA, 0xBB, 0xCC]).await;
    control(device, object, [2, 0, 0, 0, 0, 0, 0, 0, 0, 0]).await;
}

#[tokio::test]
async fn destructive_unload_hypothesis_exposes_unrestored_preserved_byte() {
    let device = SimulatedDevice::with_config(SimulatorConfig {
        mask_version: 0x0025,
        experimental_destroy_allocation_memory_on_unload: true,
        ..SimulatorConfig::default()
    });
    device.preset_system2_object(SimulatedSystem2Object::Application, old_object());
    device.preset_memory(0x2200, &[0xAA, 0xBB, 0xCC, 0xDD]);
    device.preset_memory(0x219F, &[0xEA]);
    device.preset_memory(0x2204, &[0xEB]);
    let old = device.system2_snapshot();
    control(&device, 3, [4, 0, 0, 0, 0, 0, 0, 0, 0, 0]).await;
    assert_eq!(
        device.memory(0x2200, 4),
        vec![None; 4],
        "explicit destructive fixture must invalidate retained bytes"
    );
    assert_eq!(device.memory(0x219F, 1), vec![Some(0xEA)]);
    assert_eq!(device.memory(0x2204, 1), vec![Some(0xEB)]);
    reinstall_old(&device, 3).await;
    let incomplete = device.system2_snapshot();
    assert_eq!(incomplete.objects, old.objects);
    assert_ne!(
        incomplete, old,
        "resource/control replay alone does not restore preserved bytes"
    );
    assert_eq!(device.memory(0x2203, 1), vec![None]);
    write_memory(&device, 0x2203, &[0xDD]).await;
    assert_eq!(device.system2_snapshot(), old);
}

#[tokio::test]
async fn ordinary_unload_reinstall_can_restore_independently_seeded_old_state() {
    let device = device();
    device.preset_system2_object(SimulatedSystem2Object::AssociationTable, old_object());
    device.preset_memory(0x2200, &[0xAA, 0xBB, 0xCC, 0xDD]);
    let old = device.system2_snapshot();
    control(&device, 2, [4, 0, 0, 0, 0, 0, 0, 0, 0, 0]).await;
    write_memory(&device, 0x2200, &[1, 2, 3]).await;
    assert_ne!(device.system2_snapshot(), old);
    reinstall_old(&device, 2).await;
    assert_eq!(device.system2_snapshot(), old);
}

#[tokio::test]
async fn ignored_controls_cannot_change_persistent_reference_or_metadata() {
    let device = SimulatedDevice::with_config(SimulatorConfig {
        mask_version: 0x0025,
        drop_load_state_writes: true,
        ..SimulatorConfig::default()
    });
    let mut fixture = old_object();
    fixture.load_state = LoadState::Loading;
    device.preset_system2_object(SimulatedSystem2Object::Application, fixture);
    let before = device.system2_snapshot();
    control(&device, 3, [3, 0, 0x35, 0, 0, 2, 0xFF, 3, 0, 0]).await;
    control(&device, 3, [3, 2, 0x35, 0, 0, 0, 2, 0, 3, 4]).await;
    control(&device, 3, [3, 5, 0, 1, 0, 2, 0, 3, 0, 4]).await;
    control(&device, 3, [4, 0, 0, 0, 0, 0, 0, 0, 0, 0]).await;
    assert_eq!(
        device.system2_snapshot(),
        before,
        "ignored writes must not silently mutate the reference"
    );
}

#[tokio::test]
async fn failed_allocations_do_not_create_hidden_owned_extents_or_task_identity() {
    for task in [false, true] {
        let device = SimulatedDevice::with_config(SimulatorConfig {
            mask_version: 0x0025,
            allocation_fails_once_for: Some(3),
            ..SimulatorConfig::default()
        });
        control(&device, 3, [1, 0, 0, 0, 0, 0, 0, 0, 0, 0]).await;
        let before = device.system2_snapshot();
        let payload = if task {
            [3, 2, 0x22, 0, 0x19, 0x12, 0x34, 0x56, 0x78, 0x9A]
        } else {
            [3, 0, 0x22, 0, 0, 4, 0x36, 3, 0, 0]
        };
        control(&device, 3, payload).await;
        assert_eq!(
            device.system2_snapshot(),
            before,
            "failed allocation cannot leave hidden ownership"
        );
        control(&device, 3, payload).await;
        let after = device.system2_snapshot();
        assert_ne!(after, before);
        assert_eq!(after.objects[2].table_reference, 0x2200);
        assert_eq!(after.objects[2].allocations.len(), usize::from(!task));
        assert_eq!(after.objects[2].task_segment.is_some(), task);
    }
}

#[tokio::test]
async fn refused_unload_keeps_old_ownership_and_bytes_until_an_accepted_unload() {
    let device = SimulatedDevice::with_config(SimulatorConfig {
        mask_version: 0x0025,
        fail_on_event: Some(knx_core::commissioning::load_state::LoadEvent::Unload),
        experimental_destroy_allocation_memory_on_unload: true,
        ..SimulatorConfig::default()
    });
    device.preset_system2_object(SimulatedSystem2Object::Application, old_object());
    device.preset_memory(0x2200, &[9, 8, 7, 6]);
    let before = device.system2_snapshot();
    control(&device, 3, [4, 0, 0, 0, 0, 0, 0, 0, 0, 0]).await;
    let after = device.system2_snapshot();
    assert_eq!(after.objects[2].load_state, LoadState::Error);
    let mut expected = before;
    expected.objects[2].load_state = LoadState::Error;
    assert_eq!(after, expected);
}

#[tokio::test]
async fn malformed_records_and_controls_outside_loading_create_no_metadata() {
    let device = device();
    // Existing state machine enters Error, but no hidden metadata may appear.
    control(&device, 1, [3, 5, 0, 1, 0, 2, 0, 3, 0, 4]).await;
    assert!(device.system2_snapshot().objects[0]
        .task_control_2
        .is_none());
    control(&device, 1, [4, 0, 0, 0, 0, 0, 0, 0, 0, 0]).await;
    control(&device, 1, [1, 0, 0, 0, 0, 0, 0, 0, 0, 0]).await;
    let before = device.system2_snapshot();
    for payload in [
        [3, 0, 0x22, 0, 0, 0, 0x36, 3, 0, 0],    // zero length
        [3, 0, 0xFF, 0xFF, 0, 2, 0x36, 3, 0, 0], // overflow
        [3, 0, 0x22, 0, 0, 4, 0x36, 4, 0, 0],    // memory type
        [3, 0, 0x22, 0, 0, 4, 0x36, 3, 1, 0],    // attributes
        [3, 0, 0x22, 0, 0, 4, 0x36, 3, 0, 1],    // reserved
    ] {
        control(&device, 1, payload).await;
        assert_eq!(device.system2_snapshot(), before);
    }
    device
        .send_frame(
            Destination::Individual(device.address()),
            Tpci::UnnumberedData,
            ApplicationService::PropertyValueWrite {
                object_index: 1,
                property_id: 5,
                nr_of_elem: 1,
                start_index: 1,
                data: vec![3, 5, 0, 1],
            },
        )
        .await
        .unwrap();
    assert_eq!(device.system2_snapshot(), before);
}

#[tokio::test]
async fn snapshot_ignores_trace_connections_and_verify_mode_but_not_other_setup_bits() {
    let device = device();
    device.preset_property(0, 14, &[0xA0]);
    let before = device.system2_snapshot();
    for (transport, service) in [
        (Tpci::Connect, ApplicationService::NoApplicationPdu),
        (
            Tpci::UnnumberedData,
            ApplicationService::PropertyValueWrite {
                object_index: 0,
                property_id: 14,
                nr_of_elem: 1,
                start_index: 1,
                data: vec![0xA4],
            },
        ),
    ] {
        device
            .send_frame(
                Destination::Individual(device.address()),
                transport,
                service,
            )
            .await
            .unwrap();
    }
    assert!(device.verify_mode());
    assert_eq!(device.system2_snapshot(), before);
    device.break_connection();
    assert_eq!(device.system2_snapshot(), before);
    assert!(!device.seen().is_empty());
    device.preset_property(0, 14, &[0x80]);
    assert_ne!(
        device.system2_snapshot(),
        before,
        "non-Verify setup bits remain observable"
    );
}

async fn read_property(device: &SimulatedDevice, object: u8, property: u8) -> Vec<u8> {
    let mut events = device.subscribe();
    device
        .send_frame(
            Destination::Individual(device.address()),
            Tpci::UnnumberedData,
            ApplicationService::PropertyValueRead {
                object_index: object,
                property_id: property,
                nr_of_elem: 1,
                start_index: 1,
            },
        )
        .await
        .unwrap();
    while let Ok(knx_net::client::TunnelEvent::Telegram(frame)) = events.try_recv() {
        if let ApplicationService::PropertyValueResponse { data, .. } = frame.service {
            return data;
        }
    }
    panic!("expected decoded property response");
}

#[tokio::test]
async fn pid5_remains_status_and_references_keep_four_byte_default() {
    let device = device();
    control(&device, 1, [1, 0, 0, 0, 0, 0, 0, 0, 0, 0]).await;
    control(&device, 1, [3, 0, 0x22, 0, 0, 4, 0x36, 3, 0, 0]).await;
    assert_eq!(read_property(&device, 1, 5).await, vec![2]);
    assert_eq!(read_property(&device, 1, 7).await, vec![0, 0, 0x22, 0]);
    let generic = SimulatedDevice::new();
    control(&generic, 3, [1, 0, 0, 0, 0, 0, 0, 0, 0, 0]).await;
    control(&generic, 3, [3, 11, 0, 0, 0, 4, 0, 0, 0, 0]).await;
    assert_eq!(read_property(&generic, 3, 7).await, vec![0, 0, 0x70, 0]);
    assert!(generic.system2_snapshot().objects[2].allocations.is_empty());
}

#[tokio::test]
async fn zero_task_pointer_and_zero_control_words_are_declared_state_not_absence() {
    let device = device();
    control(&device, 3, [1, 0, 0, 0, 0, 0, 0, 0, 0, 0]).await;
    control(&device, 3, [3, 2, 0, 0, 0x19, 0x12, 0x34, 0x56, 0x78, 0x9A]).await;
    control(&device, 3, [3, 5, 0, 0, 0, 0, 0, 0, 0, 0]).await;
    let state = &device.system2_snapshot().objects[2];
    assert_eq!(state.table_reference, 0);
    let mut task = old_object().task_segment.unwrap();
    task.start = 0;
    assert_eq!(state.task_segment, Some(task));
    assert_eq!(
        state.task_control_2,
        Some(SimulatedTaskControl2 {
            callback: 0,
            communication_objects: 0,
            segment_1: 0,
            segment_2: 0,
        })
    );
}

#[tokio::test]
async fn memory_changes_only_when_an_object_completes_loading() {
    let device = SimulatedDevice::with_config(SimulatorConfig {
        mask_version: 0x0025,
        memory_after_load_completed: Some((0x3c00, vec![0xd7, 0x3e])),
        ..SimulatorConfig::default()
    });
    device.preset_memory(0x3c00, &[0xa5, 0xa5]);
    let event = |octet| {
        let mut data = [0u8; 10];
        data[0] = octet;
        data
    };
    let memory = |device: &SimulatedDevice| device.memory(0x3c00, 2);
    // Unload and StartLoading leave memory alone.
    control(&device, 3, event(4)).await;
    control(&device, 3, event(1)).await;
    assert_eq!(memory(&device), [Some(0xa5), Some(0xa5)]);
    assert_eq!(
        device.load_state(knx_core::commissioning::properties::ObjectIndex::new(3)),
        LoadState::Loading
    );
    // Loading → Loaded applies the change.
    control(&device, 3, event(2)).await;
    assert_eq!(memory(&device), [Some(0xd7), Some(0x3e)]);
    // A LoadCompleted outside Loading is an Error transition, not a completion.
    device.preset_memory(0x3c00, &[0xa5, 0xa5]);
    control(&device, 3, event(2)).await;
    assert_eq!(memory(&device), [Some(0xa5), Some(0xa5)]);
    // Unloaded + LoadCompleted (Error too) and Loading + AdditionalLoadControls
    // (stays Loading) do not apply it either; a second completion does.
    control(&device, 3, event(4)).await;
    control(&device, 3, event(2)).await;
    assert_eq!(memory(&device), [Some(0xa5), Some(0xa5)]);
    control(&device, 3, event(4)).await;
    control(&device, 3, event(1)).await;
    let mut additional = event(3);
    additional[1] = 0x0b;
    control(&device, 3, additional).await;
    assert_eq!(memory(&device), [Some(0xa5), Some(0xa5)]);
    control(&device, 3, event(2)).await;
    assert_eq!(memory(&device), [Some(0xd7), Some(0x3e)]);
}

#[tokio::test]
async fn the_memory_mapped_load_control_completes_the_same_way() {
    use knx_core::commissioning::load_control_memory::{
        MemoryLoadStateMachine, MANAGEMENT_CONTROL_ADDRESS, MEMORY_LOAD_RECORD_OCTETS,
    };
    let device = SimulatedDevice::with_config(SimulatorConfig {
        mask_version: 0x0701,
        memory_after_load_completed: Some((0x4000, vec![0xd7])),
        ..SimulatorConfig::default()
    });
    device.preset_memory(0x4000, &[0xa5]);
    let record = |event: u8| {
        let mut data = vec![0u8; MEMORY_LOAD_RECORD_OCTETS];
        data[0] = (MemoryLoadStateMachine::ApplicationProgram.type_number() << 4) | event;
        data
    };
    for event in [4, 1] {
        device
            .send_frame(
                Destination::Individual(device.address()),
                Tpci::UnnumberedData,
                ApplicationService::MemoryWrite {
                    address: MANAGEMENT_CONTROL_ADDRESS,
                    data: record(event),
                },
            )
            .await
            .unwrap();
    }
    assert_eq!(device.memory(0x4000, 1), [Some(0xa5)]);
    device
        .send_frame(
            Destination::Individual(device.address()),
            Tpci::UnnumberedData,
            ApplicationService::MemoryWrite {
                address: MANAGEMENT_CONTROL_ADDRESS,
                data: record(2),
            },
        )
        .await
        .unwrap();
    assert_eq!(device.memory(0x4000, 1), [Some(0xd7)]);
}
