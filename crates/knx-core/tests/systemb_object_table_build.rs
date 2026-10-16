//! Complete System-B tables require explicit descriptors, not guessed inactive slots.

use knx_core::commissioning::group_object_table::{ObjectFlags, TransmissionPriority};
use knx_core::commissioning::systemb_object_table::{
    build_complete, ActiveObject, TableError, ValueType,
};

fn descriptor(number: u16) -> ActiveObject {
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
fn complete_table_is_built_without_a_source_template_in_asap_order() {
    let first = descriptor(1);
    let mut second = descriptor(2);
    second.flags = ObjectFlags {
        update: false,
        transmit: false,
        write: true,
        read: false,
        communication: false,
        priority: TransmissionPriority::Low,
    };
    second.read_on_initialisation = false;
    second.value_type = ValueType::from_code(255);
    let input = [second, first];
    let before = input;
    // Count2; ASAP1=update/transmit/initial-read/read/communication/normal;
    // ASAP2=write/low. No inactive-slot inference or trailing template bytes.
    assert_eq!(
        build_complete(&input, 2).unwrap(),
        [0, 2, 0xED, 22, 0x13, 255]
    );
    assert_eq!(input, before);
    assert_eq!(
        build_complete(&[first, second], 8).unwrap(),
        [0, 2, 0xED, 22, 0x13, 255]
    );
}

#[test]
fn missing_descriptors_are_not_invented_for_sparse_numbers() {
    assert_eq!(
        build_complete(&[descriptor(2)], 10),
        Err(TableError::ObjectOutOfTable { number: 2, size: 1 })
    );
    assert_eq!(
        build_complete(&[descriptor(1), descriptor(3)], 10),
        Err(TableError::ObjectOutOfTable { number: 3, size: 2 })
    );
}

#[test]
fn duplicate_and_zero_indices_are_explicit_refusals() {
    assert_eq!(
        build_complete(&[descriptor(1), descriptor(1)], 2),
        Err(TableError::DuplicateObject { number: 1 })
    );
    assert_eq!(
        build_complete(&[descriptor(0)], 1),
        Err(TableError::ObjectOutOfTable { number: 0, size: 1 })
    );
}

#[test]
fn caller_capacity_does_not_pad_or_silently_truncate_descriptors() {
    for maximum in [0, 1] {
        let error = TableError::TooManyObjects { count: 2, maximum };
        assert_eq!(
            build_complete(&[descriptor(1), descriptor(2)], maximum),
            Err(error.clone())
        );
        assert!(error.to_string().contains("2"));
    }
    assert_eq!(build_complete(&[], 0).unwrap(), [0, 0]);
    assert_eq!(build_complete(&[], u16::MAX).unwrap(), [0, 0]);
}

#[test]
fn complete_build_preserves_explicit_disabled_descriptor_fields() {
    let mut object = descriptor(1);
    object.flags.communication = false;
    // Disabling communication does not clear initial-read/update/type or
    // turn the descriptor into a guessed zero-filled hole.
    assert_eq!(build_complete(&[object], 1).unwrap(), [0, 1, 0xE9, 22]);
}

#[test]
fn full_word_count_and_indices_are_preserved_and_overflow_refuses() {
    let mut input: Vec<_> = (1..=u16::MAX).rev().map(descriptor).collect();
    let image = build_complete(&input, u16::MAX).unwrap();
    assert_eq!(image.len(), 131072);
    assert_eq!(&image[..4], &[255, 255, 0xED, 22]);
    assert_eq!(&image[512..514], &[0xED, 22]);
    assert_eq!(&image[131070..], &[0xED, 22]);
    input.push(descriptor(1));
    assert_eq!(
        build_complete(&input, u16::MAX),
        Err(TableError::TooManyObjects {
            count: 65536,
            maximum: 65535,
        })
    );
}
