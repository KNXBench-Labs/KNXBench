//! Legacy Type-1 and Type-2 table bytes are independent of mask download admission.

use knx_core::commissioning::group_object_table::{
    ActiveObject, ObjectFlags, TransmissionPriority, ValueType,
};
use knx_core::commissioning::legacy_object_table::{rewrite, LegacyObjectFormat};

fn active() -> ActiveObject {
    ActiveObject {
        number: 0,
        flags: ObjectFlags {
            update: true,
            transmit: true,
            write: false,
            read: true,
            communication: true,
            priority: TransmissionPriority::Normal,
        },
        value_type: ValueType::from_code(8).unwrap(),
    }
}

#[test]
fn type_two_uses_three_octet_descriptors_and_preserves_pointers_selector_and_tail() {
    let base = [2, 0xA7, 0x12, 0x20, 0, 0x34, 0xFF, 7, 0xDE, 0xAD];
    let before = base;
    let actual = rewrite(LegacyObjectFormat::Type2, &base, &[active()]).unwrap();
    // Current size, RAM pointer, data pointers and segment selector are not recomputed.
    // Active flags = 80|40|08|04|01, plus original selector20. Inactive clears only04.
    assert_eq!(actual, [2, 0xA7, 0x12, 0xED, 8, 0x34, 0xFB, 7, 0xDE, 0xAD]);
    assert_eq!(base, before);
}

#[test]
fn short_header_diagnostic_names_the_actual_two_octet_grammar() {
    let error = rewrite(LegacyObjectFormat::Type2, &[0], &[]).unwrap_err();
    assert!(error.to_string().contains("2-octet"), "{error}");
}

#[test]
fn type_one_keeps_its_mandatory_high_bit_and_refuses_update_enable() {
    let base = [1, 0xA7, 0x12, 0x20, 0];
    let mut object = active();
    object.flags.update = false;
    assert_eq!(
        rewrite(LegacyObjectFormat::Type1, &base, &[object]).unwrap(),
        [1, 0xA7, 0x12, 0xED, 8]
    );
    object.flags.update = true;
    assert!(rewrite(LegacyObjectFormat::Type1, &base, &[object]).is_err());
    object.flags.update = false;
    assert_eq!(
        rewrite(LegacyObjectFormat::Type2, &base, &[object]).unwrap(),
        [1, 0xA7, 0x12, 0x6D, 8]
    );
}

#[test]
fn truncated_duplicate_and_out_of_range_objects_are_rejected_without_modifying_source() {
    for base in [vec![], vec![1], vec![1, 0xA7], vec![2, 0, 1, 2, 3]] {
        assert!(rewrite(LegacyObjectFormat::Type2, &base, &[]).is_err());
    }
    let base = [1, 0xA7, 0x12, 0x24, 0];
    let object = active();
    assert!(rewrite(LegacyObjectFormat::Type2, &base, &[object, object]).is_err());
    let mut object = active();
    object.number = 1;
    assert!(rewrite(LegacyObjectFormat::Type2, &base, &[object]).is_err());
    assert_eq!(base, [1, 0xA7, 0x12, 0x24, 0]);
}

#[test]
fn empty_tables_and_maximum_count_never_truncate_the_source_tail() {
    assert_eq!(
        rewrite(LegacyObjectFormat::Type2, &[0, 0xA7, 0xDE], &[]).unwrap(),
        [0, 0xA7, 0xDE]
    );
    let mut base = vec![255, 0xA7];
    for number in 0..255_u8 {
        base.extend_from_slice(&[number, 0xFF, 0]);
    }
    base.extend_from_slice(&[0xDE, 0xAD]);
    let mut object = active();
    object.number = 254;
    let image = rewrite(LegacyObjectFormat::Type2, &base, &[object]).unwrap();
    assert_eq!(&image[2 + 254 * 3..2 + 255 * 3], &[254, 0xED, 8]);
    assert_eq!(&image[2..5], &[0, 0xFB, 0]);
    assert_eq!(&image[767..], &[0xDE, 0xAD]);
}
