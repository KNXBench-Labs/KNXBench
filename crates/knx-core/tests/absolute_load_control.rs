//! Absolute property-load records differ from BIM M112 memory-load records.

use knx_core::commissioning::absolute_load_control::{
    absolute_data, absolute_stack, task_segment, AbsolutePropertySegment,
};
use knx_core::commissioning::load_control_memory::SegmentMemoryType;

#[test]
fn property_allocation_is_ten_octets_without_a_memory_machine_or_segment_id() {
    let record = absolute_data(AbsolutePropertySegment {
        start: 0x1234,
        length: 0x5678,
        access: 0x9A,
        memory_type: SegmentMemoryType::Eeprom,
        checksum_disabled: true,
    })
    .unwrap();
    assert_eq!(
        record.octets(),
        &[3, 0, 0x12, 0x34, 0x56, 0x78, 0x9A, 3, 0, 0]
    );
}

#[test]
fn stack_and_task_records_have_their_own_subtype_and_exact_fields() {
    let segment = AbsolutePropertySegment {
        start: 0x1234,
        length: 0x5678,
        access: 0x9A,
        memory_type: SegmentMemoryType::Ram,
        checksum_disabled: false,
    };
    assert_eq!(
        absolute_stack(segment).unwrap().octets(),
        &[3, 1, 0x12, 0x34, 0x56, 0x78, 0x9A, 2, 0x80, 0]
    );
    let task = knx_core::commissioning::load_control_memory::TaskSegment {
        start: 0x1234,
        pei_type: 0x56,
        manufacturer: 0x789A,
        application: 0xBCDE,
        version: 0xF0,
    };
    assert_eq!(
        task_segment(task).octets(),
        &[3, 2, 0x12, 0x34, 0x56, 0x78, 0x9A, 0xBC, 0xDE, 0xF0]
    );
}

#[test]
fn empty_and_wrapping_allocations_are_refused_but_last_address_is_legal() {
    let mut segment = AbsolutePropertySegment {
        start: u16::MAX,
        length: 1,
        access: 0xFF,
        memory_type: SegmentMemoryType::ZeroPageRam,
        checksum_disabled: false,
    };
    assert_eq!(
        absolute_data(segment).unwrap().octets(),
        &[3, 0, 255, 255, 0, 1, 255, 1, 0x80, 0]
    );
    segment.length = 2;
    assert!(absolute_data(segment).is_err());
    assert!(absolute_stack(segment).is_err());
    segment.length = 0;
    assert!(absolute_data(segment).is_err());
    assert!(absolute_stack(segment).is_err());
    assert!(
        knx_core::commissioning::load_control::allocation_subtype_for(
            knx_core::commissioning::load_state::MaskVersion(0x0025)
        )
        .is_err()
    );
}

/// MP v02.01.02 §3.31.3.4 p140 explicitly defines bit7 = 0 as disabled.
#[test]
fn disabling_checksum_clears_property_memory_attribute_bit_seven() {
    for build in [absolute_data, absolute_stack] {
        let disabled = AbsolutePropertySegment {
            start: 0x3C00,
            length: 0x0100,
            access: 0x33,
            memory_type: SegmentMemoryType::Eeprom,
            checksum_disabled: true,
        };
        assert_eq!(
            build(disabled).unwrap().octets()[8],
            0,
            "MP p140: zero disables checksum control; no inverted polarity"
        );
        let not_disabled = AbsolutePropertySegment {
            checksum_disabled: false,
            ..disabled
        };
        assert_eq!(build(not_disabled).unwrap().octets()[8], 0x80);
    }
}
