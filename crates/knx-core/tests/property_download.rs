//! Property plans preserve ordered source positions and refuse unsafe normalized operations.

use knx_core::commissioning::absolute_load_control::{absolute_data, AbsolutePropertySegment};
use knx_core::commissioning::load_control::{
    additional_load_control, event_payload, task_control_2, LoadControlSubtype, TaskControl2,
};
use knx_core::commissioning::load_control_memory::SegmentMemoryType;
use knx_core::commissioning::load_state::{LoadEvent, MaskVersion};
use knx_core::commissioning::property_download::{
    MutationEffect, PropertyDownloadPlan, PropertyDownloadStep, PropertyOperation as Op,
};
use knx_core::IndividualAddress;

fn step(source_index: usize, operation: Op) -> PropertyDownloadStep {
    PropertyDownloadStep {
        source_index,
        operation,
    }
}

fn control(index: usize, event: LoadEvent) -> PropertyDownloadStep {
    step(
        index,
        Op::LoadControl {
            object: 3,
            payload: event_payload(event),
        },
    )
}

fn operations() -> Vec<PropertyDownloadStep> {
    vec![
        step(0, Op::Connect),
        step(
            1,
            Op::HardwareCheck {
                expected: [0x12, 0x34, 0x56, 0x78, 0x9A, 0xBC],
            },
        ),
        control(2, LoadEvent::Unload),
        control(3, LoadEvent::StartLoading),
        step(
            4,
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
            5,
            Op::ToolMetadata {
                table_address: 0x400,
            },
        ),
        control(6, LoadEvent::LoadCompleted),
        step(7, Op::Disconnect),
    ]
}

fn plan(steps: Vec<PropertyDownloadStep>) -> Result<PropertyDownloadPlan, impl std::fmt::Debug> {
    PropertyDownloadPlan::new(
        IndividualAddress::from_raw(0x1234),
        MaskVersion(0x0025),
        0x5678,
        steps,
    )
}

fn with_memory() -> Vec<PropertyDownloadStep> {
    let mut steps = operations();
    steps[4].operation = Op::LoadControl {
        object: 3,
        payload: absolute_data(AbsolutePropertySegment {
            start: 0x3C00,
            length: 4,
            access: 0xFF,
            memory_type: SegmentMemoryType::Eeprom,
            checksum_disabled: true,
        })
        .unwrap(),
    };
    steps.insert(
        5,
        step(
            4,
            Op::WriteMemory {
                object: 3,
                address: 0x3C00,
                data: vec![0x12, 0x34, 0x56, 0x78],
            },
        ),
    );
    steps
}

#[test]
fn a_complete_property_plan_preserves_source_positions_and_never_becomes_a_memory_plan() {
    let operations = operations();
    let plan = plan(operations.clone()).expect("complete simulator plan");
    assert_eq!(plan.steps(), operations);
    assert_eq!(plan.mask(), MaskVersion(0x0025));
    assert_eq!(plan.target(), IndividualAddress::from_raw(0x1234));
    assert_eq!(plan.manufacturer(), 0x5678);
    assert_eq!(plan.source_positions(), 8);
}

#[test]
fn allocation_and_its_write_share_one_preserved_source_position() {
    let steps = with_memory();
    let plan = plan(steps.clone()).unwrap();
    assert_eq!(plan.steps(), steps);
    assert_eq!(plan.source_positions(), 8);
    assert!(plan
        .effects()
        .contains(&MutationEffect::ConnectionAuthorization));
    assert!(plan.effects().contains(&MutationEffect::VerifyModeSetup));
    assert!(plan.effects().contains(&MutationEffect::MemoryWrite {
        source_index: 4,
        address: 0x3C00,
        length: 4
    }));
    assert!(plan.effects().contains(&MutationEffect::LoadControl {
        source_index: 4,
        object: 3,
        indirect_allocation: true
    }));
}

#[test]
fn gaps_duplicates_and_reordered_source_positions_refuse() {
    for (position, replacement) in [(0, 1), (1, 0), (2, 1), (4, 6), (7, usize::MAX)] {
        let mut steps = operations();
        steps[position].source_index = replacement;
        assert!(plan(steps).is_err(), "source position {position}");
    }
    let mut steps = operations();
    steps.insert(
        6,
        step(
            5,
            Op::ToolMetadata {
                table_address: 0x400,
            },
        ),
    );
    assert!(
        plan(steps).is_err(),
        "duplicate metadata is not a memory continuation"
    );
}

#[test]
fn missing_hardware_check_and_load_lifecycle_errors_refuse() {
    for index in [1, 2, 3, 6] {
        let mut steps = operations();
        steps[index].operation = Op::ToolMetadata {
            table_address: 0x400,
        };
        assert!(plan(steps).is_err(), "missing lifecycle at {index}");
    }
    let mut steps = operations();
    steps[5].operation = Op::HardwareCheck { expected: [0; 6] };
    assert!(plan(steps).is_err(), "late or repeated identity guard");
    for index in [0, 7] {
        let mut steps = operations();
        steps[index].operation = Op::ToolMetadata { table_address: 0 };
        assert!(plan(steps).is_err(), "connection boundary {index}");
    }
}

#[test]
fn unknown_resources_relative_controls_and_malformed_absolute_payloads_refuse() {
    for object in [0, 4, 5, 255] {
        let mut steps = operations();
        if let Op::LoadControl { object: field, .. } = &mut steps[4].operation {
            *field = object;
        }
        assert!(plan(steps).is_err(), "unresolved object {object}");
    }
    for (subtype, fields) in [
        (LoadControlSubtype::RelativeAllocation, [0; 8]),
        (LoadControlSubtype::DataRelativeAllocation, [0; 8]),
        (
            LoadControlSubtype::AllocAbsDataSeg,
            [0x3C, 0, 0, 0, 0xFF, 2, 0x80, 0],
        ),
        (
            LoadControlSubtype::AllocAbsDataSeg,
            [0xFF, 0xFF, 0, 2, 0xFF, 2, 0x80, 0],
        ),
        (
            LoadControlSubtype::AllocAbsDataSeg,
            [0x3C, 0, 0, 4, 0xFF, 4, 0x80, 0],
        ),
        (
            LoadControlSubtype::AllocAbsDataSeg,
            [0x3C, 0, 0, 4, 0xFF, 0, 0x80, 0],
        ),
        (
            LoadControlSubtype::AllocAbsDataSeg,
            [0x3C, 0, 0, 4, 0xFF, 2, 0x81, 0],
        ),
        (
            LoadControlSubtype::AllocAbsDataSeg,
            [0x3C, 0, 0, 4, 0xFF, 2, 0x80, 1],
        ),
    ] {
        let mut steps = operations();
        steps[4].operation = Op::LoadControl {
            object: 3,
            payload: additional_load_control(subtype, fields),
        };
        assert!(
            plan(steps).is_err(),
            "invalid additional control {subtype:?}/{fields:?}"
        );
    }
}

#[test]
fn unallocated_empty_out_of_bounds_and_cross_resource_writes_refuse() {
    for (object, address, data) in [
        (3, 0x3BFF, vec![1]),
        (3, 0x3C04, vec![1]),
        (3, 0x3C00, vec![]),
        (3, 0x3C00, vec![1; 5]),
        (2, 0x3C00, vec![1]),
    ] {
        let mut steps = with_memory();
        steps[5].operation = Op::WriteMemory {
            object,
            address,
            data,
        };
        assert!(plan(steps).is_err(), "write at {address:04X}");
    }
    let mut steps = with_memory();
    steps[4].operation = Op::ToolMetadata {
        table_address: 0x400,
    };
    assert!(plan(steps).is_err(), "no implicit allocation from metadata");
    let mut steps = with_memory();
    steps.insert(6, steps[5].clone());
    assert!(
        plan(steps).is_err(),
        "overwriting the same region is ambiguous"
    );
}

#[test]
fn only_the_explicit_mask_subset_is_represented_not_hardware_admission() {
    for mask in [0x0020, 0x0021, 0x0701, 0x07B0, 0xFFFF] {
        assert!(PropertyDownloadPlan::new(
            IndividualAddress::from_raw(0x1234),
            MaskVersion(mask),
            0,
            operations()
        )
        .is_err());
    }
    assert!(plan(vec![]).is_err());
    assert!(plan(vec![step(0, Op::Connect), step(1, Op::Disconnect)]).is_err());
}

#[test]
fn property_memory_type_attributes_and_boundary_remain_separate_fields() {
    for memory_type in 1..=3 {
        for attributes in [0, 0x80] {
            let mut steps = with_memory();
            steps[4].operation = Op::LoadControl {
                object: 3,
                payload: additional_load_control(
                    LoadControlSubtype::AllocAbsDataSeg,
                    [0xFF, 0xFC, 0, 4, 0xFF, memory_type, attributes, 0],
                ),
            };
            steps[5].operation = Op::WriteMemory {
                object: 3,
                address: 0xFFFC,
                data: vec![1, 2, 3, 4],
            };
            assert!(
                plan(steps).is_ok(),
                "memory type {memory_type}, attributes {attributes:02X}"
            );
        }
    }
    let mut steps = with_memory();
    steps[6].operation = steps[4].operation.clone();
    assert!(
        plan(steps).is_err(),
        "overlapping allocations at different source positions"
    );
}
