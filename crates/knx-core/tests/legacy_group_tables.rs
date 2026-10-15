//! Legacy association sending slots are indexed by ASAP, not sorted like Easy-3.

use knx_core::commissioning::group_tables::{TableCapacity, TableLink};
use knx_core::commissioning::legacy_group_tables::build;
use knx_core::{GroupAddress, IndividualAddress};

#[test]
fn sparse_object_numbers_keep_their_sending_index_and_append_receive_only_links() {
    let link = |object, raw, sending| TableLink {
        object,
        group_address: GroupAddress::from_raw(raw),
        sending,
    };
    let actual = build(
        IndividualAddress::from_raw(0x1234),
        4,
        &[
            link(3, 0x4567, true),
            link(3, 0x2345, false),
            link(1, 0x3456, false),
        ],
        TableCapacity {
            group_addresses: 20,
            associations: 20,
        },
    )
    .unwrap();
    assert_eq!(
        actual.address_table,
        [4, 0x12, 0x34, 0x23, 0x45, 0x34, 0x56, 0x45, 0x67]
    );
    // First four entries reserve ASAP0..3. FE marks unused slots, never a GA index.
    assert_eq!(
        actual.association_table,
        [6, 0xFE, 0, 0xFE, 1, 0xFE, 2, 3, 3, 2, 1, 1, 3]
    );
}

#[test]
fn indexed_unused_rows_count_against_capacity_and_fe_is_never_a_real_tsap() {
    let room = TableCapacity {
        group_addresses: 254,
        associations: 255,
    };
    let links: Vec<_> = (1..=254_u16)
        .map(|raw| TableLink {
            object: 0,
            group_address: GroupAddress::from_raw(raw),
            sending: false,
        })
        .collect();
    assert!(build(IndividualAddress::from_raw(0x1234), 1, &links, room).is_err());
    assert!(build(
        IndividualAddress::from_raw(0x1234),
        4,
        &[],
        TableCapacity {
            group_addresses: 20,
            associations: 3
        }
    )
    .is_err());
    assert_eq!(
        build(IndividualAddress::from_raw(0x1234), 0, &[], room)
            .unwrap()
            .association_table,
        [0]
    );
    let last = build(IndividualAddress::from_raw(0x1234), 255, &[], room).unwrap();
    assert_eq!(last.association_table.len(), 511);
    assert_eq!(&last.association_table[509..], &[0xFE, 254]);
}

#[test]
fn rejects_duplicates_broadcast_multiple_senders_and_objects_without_a_descriptor() {
    let room = TableCapacity {
        group_addresses: 20,
        associations: 20,
    };
    let a = TableLink {
        object: 1,
        group_address: GroupAddress::from_raw(0x2345),
        sending: true,
    };
    let b = TableLink {
        group_address: GroupAddress::from_raw(0x3456),
        ..a
    };
    for links in [
        vec![a, a],
        vec![a, b],
        vec![TableLink {
            group_address: GroupAddress::from_raw(0),
            ..a
        }],
    ] {
        assert!(build(IndividualAddress::from_raw(0x1234), 4, &links, room).is_err());
    }
    assert!(build(IndividualAddress::from_raw(0x1234), 1, &[a], room).is_err());
    let mut links = vec![
        TableLink {
            sending: false,
            ..a
        },
        TableLink {
            sending: false,
            ..b
        },
    ];
    let image = build(IndividualAddress::from_raw(0x1234), 4, &links, room).unwrap();
    links.reverse();
    assert_eq!(
        build(IndividualAddress::from_raw(0x1234), 4, &links, room).unwrap(),
        image
    );
    assert_eq!(
        &image.association_table[3..5],
        &[0xFE, 1],
        "no implicit sending address"
    );
}
