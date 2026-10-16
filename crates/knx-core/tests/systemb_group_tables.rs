//! System-B link tables use explicit widths and preserve the selected first-match sender.

use knx_core::commissioning::group_tables::TableCapacity;
use knx_core::commissioning::systemb_group_tables::{build, AssociationFormat, Link, TableError};
use knx_core::GroupAddress;

fn link(object: u16, address: u16, sending: bool) -> Link {
    Link {
        object,
        group_address: GroupAddress::from_raw(address),
        sending,
    }
}

fn capacity() -> TableCapacity {
    TableCapacity {
        group_addresses: usize::MAX,
        associations: usize::MAX,
    }
}

#[test]
fn format_zero_uses_word_lengths_no_individual_slot_and_sender_before_lower_address() {
    let input = [
        link(2, 0x0201, false),
        link(1, 0x0201, true),
        link(2, 0x1234, true),
    ];
    let tables = build(AssociationFormat::Format0, 2, &input, capacity()).unwrap();
    assert_eq!(tables.address_table, [0, 2, 0x02, 0x01, 0x12, 0x34]);
    assert_eq!(tables.association_table, [0, 3, 1, 1, 2, 2, 1, 2]);
    assert_eq!(input[0], link(2, 0x0201, false));
}

#[test]
fn format_one_uses_two_octet_tsap_and_asap_without_truncation() {
    let input: Vec<_> = (1..=256).map(|a| link(0x1234, a, a == 256)).collect();
    let tables = build(AssociationFormat::Format1, 0x1234, &input, capacity()).unwrap();
    assert_eq!(&tables.address_table[..6], &[1, 0, 0, 1, 0, 2]);
    assert_eq!(&tables.address_table[512..], &[1, 0]);
    assert_eq!(
        &tables.association_table[..10],
        &[1, 0, 1, 0, 0x12, 0x34, 0, 1, 0x12, 0x34]
    );
    assert_eq!(tables.association_table.len(), 2 + 256 * 4);
}

#[test]
fn both_explicit_formats_keep_sending_semantics_with_shuffled_input() {
    let input = [
        link(1, 0x200, false),
        link(1, 0x300, true),
        link(2, 0x100, true),
    ];
    for format in [AssociationFormat::Format0, AssociationFormat::Format1] {
        let mut reversed = input;
        reversed.reverse();
        assert_eq!(
            build(format, 2, &input, capacity()),
            build(format, 2, &reversed, capacity())
        );
        let table = build(format, 2, &input, capacity())
            .unwrap()
            .association_table;
        let row_size = if format == AssociationFormat::Format0 {
            2
        } else {
            4
        };
        let rows: Vec<(u16, u16)> = table[2..]
            .chunks_exact(row_size)
            .map(|row| {
                if row_size == 2 {
                    (u16::from(row[0]), u16::from(row[1]))
                } else {
                    (
                        u16::from_be_bytes([row[0], row[1]]),
                        u16::from_be_bytes([row[2], row[3]]),
                    )
                }
            })
            .collect();
        assert_eq!(rows, [(3, 1), (2, 1), (1, 2)]);
        assert_eq!(rows.iter().find(|r| r.1 == 1).unwrap().0, 3);
    }
}

#[test]
fn empty_tables_have_only_the_two_word_count_fields() {
    for format in [AssociationFormat::Format0, AssociationFormat::Format1] {
        let tables = build(
            format,
            0,
            &[],
            TableCapacity {
                group_addresses: 0,
                associations: 0,
            },
        )
        .unwrap();
        assert_eq!(tables.address_table, [0, 0]);
        assert_eq!(tables.association_table, [0, 0]);
    }
}

#[test]
fn one_based_asap_must_fit_explicit_source_object_count() {
    for (number, count) in [(0, 2), (3, 2), (1, 0)] {
        assert_eq!(
            build(
                AssociationFormat::Format1,
                count,
                &[link(number, 1, true)],
                capacity()
            ),
            Err(TableError::ObjectOutOfTable { number, count })
        );
    }
}

#[test]
fn broadcast_duplicate_pairs_and_multiple_senders_are_never_repaired() {
    assert_eq!(
        build(
            AssociationFormat::Format1,
            1,
            &[link(1, 0, true)],
            capacity()
        ),
        Err(TableError::BroadcastAddress { object: 1 })
    );
    assert_eq!(
        build(
            AssociationFormat::Format1,
            1,
            &[link(1, 1, true), link(1, 1, false)],
            capacity()
        ),
        Err(TableError::DuplicateLink {
            object: 1,
            group_address: GroupAddress::from_raw(1)
        })
    );
    assert_eq!(
        build(
            AssociationFormat::Format1,
            1,
            &[link(1, 1, true), link(1, 2, true)],
            capacity()
        ),
        Err(TableError::SeveralSendingAddresses { object: 1 })
    );
}

#[test]
fn no_sender_is_an_explicit_unrepresentable_intent_not_an_implicit_first_address() {
    assert_eq!(
        build(
            AssociationFormat::Format1,
            1,
            &[link(1, 1, false)],
            capacity()
        ),
        Err(TableError::NoSendingAddress { object: 1 })
    );
}

#[test]
fn format_zero_refuses_high_asap_and_the_256th_tsap() {
    let narrow: Vec<_> = (1..=255).map(|a| link(255, a, a == 255)).collect();
    let boundary = build(AssociationFormat::Format0, 255, &narrow, capacity()).unwrap();
    assert_eq!(&boundary.address_table[..4], &[0, 255, 0, 1]);
    assert_eq!(
        &boundary.association_table[..6],
        &[0, 255, 255, 255, 1, 255]
    );
    assert_eq!(boundary.association_table.len(), 512);
    assert_eq!(
        build(
            AssociationFormat::Format0,
            256,
            &[link(256, 1, true)],
            capacity()
        ),
        Err(TableError::ObjectNotRepresentable { number: 256 })
    );
    let links: Vec<_> = (1..=256).map(|a| link(1, a, a == 1)).collect();
    assert_eq!(
        build(AssociationFormat::Format0, 1, &links, capacity()),
        Err(TableError::TooManyGroupAddresses {
            count: 256,
            capacity: 255
        })
    );
}

#[test]
fn declared_capacity_is_not_silently_widened_to_the_format_limit() {
    let links = [link(1, 1, true), link(1, 2, false)];
    assert_eq!(
        build(
            AssociationFormat::Format1,
            1,
            &links,
            TableCapacity {
                group_addresses: 1,
                associations: 10
            }
        ),
        Err(TableError::TooManyGroupAddresses {
            count: 2,
            capacity: 1
        })
    );
    assert_eq!(
        build(
            AssociationFormat::Format1,
            1,
            &links,
            TableCapacity {
                group_addresses: 10,
                associations: 1
            }
        ),
        Err(TableError::TooManyAssociations {
            count: 2,
            capacity: 1
        })
    );
}

#[test]
fn full_word_count_and_last_tsap_and_asap_are_encoded_exactly() {
    let links: Vec<_> = (1..=u16::MAX)
        .map(|a| link(u16::MAX, a, a == u16::MAX))
        .collect();
    let tables = build(AssociationFormat::Format1, u16::MAX, &links, capacity()).unwrap();
    assert_eq!(&tables.address_table[..4], &[255, 255, 0, 1]);
    assert_eq!(
        &tables.address_table[tables.address_table.len() - 2..],
        &[255, 255]
    );
    assert_eq!(
        &tables.association_table[..10],
        &[255, 255, 255, 255, 255, 255, 0, 1, 255, 255]
    );
    assert_eq!(tables.address_table.len(), 2 + 65535 * 2);
    assert_eq!(tables.association_table.len(), 2 + 65535 * 4);
}

#[test]
fn association_word_overflow_is_refused_with_no_truncation() {
    let links: Vec<_> = (1..=256)
        .flat_map(|object| (1..=256).map(move |a| link(object, a, a == 1)))
        .collect();
    assert_eq!(
        build(AssociationFormat::Format1, 256, &links, capacity()),
        Err(TableError::TooManyAssociations {
            count: 65536,
            capacity: 65535
        })
    );
}

#[test]
fn one_shared_address_has_one_tsap_and_distinct_asaps() {
    let tables = build(
        AssociationFormat::Format1,
        0x0201,
        &[link(0x0201, 0x1234, true), link(1, 0x1234, true)],
        capacity(),
    )
    .unwrap();
    assert_eq!(tables.address_table, [0, 1, 0x12, 0x34]);
    assert_eq!(tables.association_table, [0, 2, 0, 1, 0, 1, 0, 1, 2, 1]);
}
