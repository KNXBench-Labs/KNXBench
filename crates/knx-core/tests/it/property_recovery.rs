//! Recovery-shape tests for the bounded normalized property procedure.

use knx_core::commissioning::load_control::{
    additional_load_control, event_payload, LoadControlSubtype,
};
use knx_core::commissioning::load_state::{LoadEvent, MaskVersion};
use knx_core::commissioning::property_download::{
    PropertyDownloadPlan, PropertyDownloadStep, PropertyOperation,
};
use knx_core::IndividualAddress;

#[test]
fn recovery_replaces_only_exact_memory_payloads_and_preserves_complete_procedure() {
    use knx_core::commissioning::absolute_load_control::{absolute_data, AbsolutePropertySegment};
    use knx_core::commissioning::load_control_memory::SegmentMemoryType;
    let op = |source_index, operation| PropertyDownloadStep {
        source_index,
        operation,
    };
    let control = |source_index, payload| {
        op(
            source_index,
            PropertyOperation::LoadControl { object: 3, payload },
        )
    };
    let plan = PropertyDownloadPlan::new(
        IndividualAddress::from_raw(0x1119),
        MaskVersion(0x0025),
        2,
        vec![
            op(0, PropertyOperation::Connect),
            op(1, PropertyOperation::HardwareCheck { expected: [1; 6] }),
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
                PropertyOperation::WriteMemory {
                    object: 3,
                    address: 0x3C00,
                    data: vec![1, 2, 3, 4],
                },
            ),
            control(5, event_payload(LoadEvent::LoadCompleted)),
            op(6, PropertyOperation::Disconnect),
        ],
    )
    .unwrap();
    let recovery = plan.with_memory_payloads(&[vec![8, 7, 6, 5]]).unwrap();
    let mut expected = plan.steps().to_vec();
    expected[5].operation = PropertyOperation::WriteMemory {
        object: 3,
        address: 0x3C00,
        data: vec![8, 7, 6, 5],
    };
    assert_eq!(recovery.steps(), expected);
    assert_eq!(recovery.target(), plan.target());
    assert_eq!(recovery.effects(), plan.effects());
    for payloads in [vec![], vec![vec![8]], vec![vec![8; 4], vec![9; 4]]] {
        assert!(
            plan.with_memory_payloads(&payloads).is_err(),
            "coverage must be exact"
        );
    }
}

#[test]
fn source_task_segment_pointer_is_preserved_for_each_resolved_resource() {
    for object in 1..=3 {
        let op = |source_index, operation| PropertyDownloadStep {
            source_index,
            operation,
        };
        let control = |source_index, payload| {
            op(
                source_index,
                PropertyOperation::LoadControl { object, payload },
            )
        };
        let pointer = additional_load_control(
            LoadControlSubtype::AllocAbsTaskSeg,
            [0x3C, object, 0, 0, 0, 0, 0, 0],
        );
        let steps = vec![
            op(0, PropertyOperation::Connect),
            op(1, PropertyOperation::HardwareCheck { expected: [1; 6] }),
            control(2, event_payload(LoadEvent::Unload)),
            control(3, event_payload(LoadEvent::StartLoading)),
            control(4, pointer),
            control(5, event_payload(LoadEvent::LoadCompleted)),
            op(6, PropertyOperation::Disconnect),
        ];
        let plan = PropertyDownloadPlan::new(
            IndividualAddress::from_raw(0x1119),
            MaskVersion(0x0025),
            2,
            steps.clone(),
        )
        .expect("source subtype2 must support resolved objects1/2 as well as3");
        assert_eq!(plan.steps(), steps);
    }
}
