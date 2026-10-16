//! System-B object descriptors use their own grammar without granting download admission.

use knx_core::commissioning::group_object_table::{ObjectFlags, TransmissionPriority};
use knx_core::commissioning::systemb_object_table::{rewrite, ActiveObject, TableError, ValueType};

fn active(number: u16) -> ActiveObject {
    ActiveObject {
        number,
        flags: ObjectFlags {
            update: true,
            transmit: true,
            write: false,
            read: true,
            communication: true,
            priority: TransmissionPriority::Normal,
        },
        read_on_initialisation: true,
        value_type: ValueType::from_code(22),
    }
}

#[test]
fn source_header_and_tail_are_preserved_with_one_based_two_octet_descriptors() {
    let base = [0, 2, 0x12, 0x34, 0xFF, 0xA5, 0xDE, 0xAD, 0xBE];
    let before = base;
    // Active = update|transmit|initial-read|read|communication|normal = EDh.
    // Inactive retains all fields except communication bit 10.
    assert_eq!(
        rewrite(&base, &[active(1)]).unwrap(),
        [0, 2, 0xED, 22, 0xFB, 0xA5, 0xDE, 0xAD, 0xBE]
    );
    assert_eq!(base, before);
}

#[test]
fn every_flag_and_priority_has_the_original_table_87_bit_position() {
    let mut object = active(1);
    object.flags = ObjectFlags {
        update: false,
        transmit: false,
        write: false,
        read: false,
        communication: false,
        priority: TransmissionPriority::System,
    };
    object.read_on_initialisation = false;
    object.value_type = ValueType::from_code(0xA7);
    let base = [0, 1, 0xFF, 0xFF];
    assert_eq!(rewrite(&base, &[object]).unwrap(), [0, 1, 0, 0xA7]);
    for (bit, high) in [(0, 0x80), (1, 0x40), (2, 0x20), (3, 0x10), (4, 8), (5, 4)] {
        let mut changed = object;
        match bit {
            0 => changed.flags.update = true,
            1 => changed.flags.transmit = true,
            2 => changed.read_on_initialisation = true,
            3 => changed.flags.write = true,
            4 => changed.flags.read = true,
            _ => changed.flags.communication = true,
        }
        assert_eq!(rewrite(&base, &[changed]).unwrap(), [0, 1, high, 0xA7]);
    }
    for (priority, high) in [
        (TransmissionPriority::System, 0),
        (TransmissionPriority::Urgent, 2),
        (TransmissionPriority::Normal, 1),
        (TransmissionPriority::Low, 3),
    ] {
        object.flags.priority = priority;
        assert_eq!(rewrite(&base, &[object]).unwrap(), [0, 1, high, 0xA7]);
    }
}

#[test]
fn all_inactive_descriptor_bits_except_communication_are_retained() {
    for high in 0..=u8::MAX {
        for low in [0, 14, 21, 254, 255] {
            assert_eq!(
                rewrite(&[0, 1, high, low], &[]).unwrap(),
                [0, 1, high & !4, low]
            );
        }
    }
}

#[test]
fn empty_table_keeps_even_an_odd_tail_but_cannot_hold_an_active_object() {
    assert_eq!(rewrite(&[0, 0, 0xA7], &[]).unwrap(), [0, 0, 0xA7]);
    assert_eq!(
        rewrite(&[0, 0], &[active(1)]),
        Err(TableError::ObjectOutOfTable { number: 1, size: 0 })
    );
}

#[test]
fn malformed_header_and_truncation_are_explicit_and_never_modify_source() {
    assert_eq!(rewrite(&[], &[]), Err(TableError::NoHeader { octets: 0 }));
    assert_eq!(rewrite(&[1], &[]), Err(TableError::NoHeader { octets: 1 }));
    for base in [vec![0, 1], vec![0, 1, 0xAA], vec![1, 0, 0, 0]] {
        let size = u16::from_be_bytes([base[0], base[1]]);
        let before = base.clone();
        assert_eq!(
            rewrite(&base, &[]),
            Err(TableError::Truncated {
                needed: 2 + usize::from(size) * 2,
                octets: base.len(),
            })
        );
        assert_eq!(base, before);
    }
}

#[test]
fn zero_out_of_range_and_duplicate_objects_are_not_silently_ignored() {
    let base = [0, 2, 0, 0, 0, 0];
    for number in [0, 3, u16::MAX] {
        assert_eq!(
            rewrite(&base, &[active(number)]),
            Err(TableError::ObjectOutOfTable { number, size: 2 })
        );
    }
    assert_eq!(
        rewrite(&base, &[active(2), active(2)]),
        Err(TableError::DuplicateObject { number: 2 })
    );
    assert_eq!(base, [0, 2, 0, 0, 0, 0]);
}

#[test]
fn active_input_order_does_not_change_the_image() {
    let base = [0, 3, 0, 0, 0, 0, 0, 0];
    assert_eq!(
        rewrite(&base, &[active(1), active(3)]),
        rewrite(&base, &[active(3), active(1)])
    );
}

#[test]
fn full_u16_count_and_last_one_based_object_are_not_truncated_to_legacy_width() {
    let mut base = vec![0xFF; 2 + usize::from(u16::MAX) * 2];
    base.extend_from_slice(&[0xDE, 0xAD]);
    let image = rewrite(&base, &[active(256), active(u16::MAX)]).unwrap();
    assert_eq!(&image[..4], &[0xFF, 0xFF, 0xFB, 0xFF]);
    assert_eq!(&image[512..514], &[0xED, 22]);
    assert_eq!(&image[131070..], &[0xED, 22, 0xDE, 0xAD]);
}

#[test]
fn table_88_literal_value_types_include_nonlegacy_sizes() {
    for (bits, code) in [
        (1, 0),
        (2, 1),
        (3, 2),
        (4, 3),
        (5, 4),
        (6, 5),
        (7, 6),
        (8, 7),
        (16, 8),
        (24, 9),
        (32, 10),
        (48, 11),
        (64, 12),
        (80, 13),
        (112, 14),
        (40, 15),
        (56, 16),
        (72, 17),
        (88, 18),
        (96, 19),
        (104, 20),
        (120, 21),
        (128, 22),
        (136, 23),
        (1984, 254),
        (2016, 255),
    ] {
        assert_eq!(ValueType::for_bits(bits).map(ValueType::code), Some(code));
    }
    for code in 21..=254_u8 {
        assert_eq!(
            ValueType::for_bits(u32::from(code - 6) * 8).unwrap().code(),
            code
        );
    }
    for code in 0..=u8::MAX {
        assert_eq!(ValueType::from_code(code).code(), code);
    }
}

#[test]
fn unsupported_value_sizes_are_refused_rather_than_rounded() {
    for bits in [
        0,
        9,
        15,
        17,
        113,
        119,
        121,
        1985,
        1992,
        2000,
        2008,
        2017,
        u32::MAX,
    ] {
        assert_eq!(ValueType::for_bits(bits), None, "{bits}");
    }
}
