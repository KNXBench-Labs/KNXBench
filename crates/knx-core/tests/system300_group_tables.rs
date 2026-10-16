//! System-300 property images separate array lengths from actual table elements.

use knx_core::commissioning::group_tables::TableCapacity;
use knx_core::commissioning::system300_group_tables::build;
use knx_core::commissioning::systemb_group_tables::{AssociationFormat, Link, TableError};
use knx_core::GroupAddress;

fn link(object: u16, address: u16, sending: bool) -> Link {
    Link {
        object,
        group_address: GroupAddress::from_raw(address),
        sending,
    }
}

const ROOMY: TableCapacity = TableCapacity {
    group_addresses: 1024,
    associations: 2048,
};

#[test]
fn property_elements_have_no_memory_count_or_individual_address_prefix() {
    let tables = build(
        AssociationFormat::Format0,
        0x34,
        &[
            link(0x12, 0x1234, false),
            link(0x34, 0x2345, true),
            link(0x12, 0x3456, true),
        ],
        ROOMY,
    )
    .unwrap();
    assert_eq!(tables.address_table.element_count(), 3);
    assert_eq!(tables.address_table.length_value(), [0, 3]);
    assert_eq!(tables.address_table.element_width(), 2);
    assert_eq!(
        tables.address_table.elements(),
        &[0x12, 0x34, 0x23, 0x45, 0x34, 0x56]
    );
    assert_eq!(tables.association_table.element_count(), 3);
    assert_eq!(tables.association_table.length_value(), [0, 3]);
    assert_eq!(tables.association_table.element_width(), 2);
    // Object12 sends on TSAP3 although its receiver sorts first in the address table.
    assert_eq!(
        tables.association_table.elements(),
        &[3, 0x12, 1, 0x12, 2, 0x34]
    );
}

#[test]
fn format1_uses_word_indices_but_element_zero_is_still_a_two_octet_length() {
    let tables = build(
        AssociationFormat::Format1,
        0x3456,
        &[
            link(0x1234, 0x1234, false),
            link(0x3456, 0x2345, true),
            link(0x1234, 0x3456, true),
        ],
        ROOMY,
    )
    .unwrap();
    assert_eq!(tables.address_table.length_value(), [0, 3]);
    assert_eq!(tables.address_table.element_width(), 2);
    assert_eq!(
        tables.address_table.elements(),
        &[0x12, 0x34, 0x23, 0x45, 0x34, 0x56]
    );
    assert_eq!(tables.association_table.length_value(), [0, 3]);
    assert_eq!(tables.association_table.element_width(), 4);
    assert_eq!(
        tables.association_table.elements(),
        &[0, 3, 0x12, 0x34, 0, 1, 0x12, 0x34, 0, 2, 0x34, 0x56]
    );
}

#[test]
fn empty_tables_keep_zero_length_separate_from_the_empty_element_payload() {
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
        for table in [&tables.address_table, &tables.association_table] {
            assert_eq!(table.element_count(), 0);
            assert_eq!(table.length_value(), [0, 0]);
            assert!(table.elements().is_empty());
        }
        assert_eq!(tables.address_table.element_width(), 2);
        assert_eq!(
            tables.association_table.element_width(),
            if format == AssociationFormat::Format0 {
                2
            } else {
                4
            }
        );
    }
}

#[test]
fn counts_and_tsaps_beyond_an_octet_are_preserved_in_format1() {
    let links: Vec<_> = (1..=256)
        .map(|address| link(address, address, true))
        .collect();
    let tables = build(AssociationFormat::Format1, 256, &links, ROOMY).unwrap();
    assert_eq!(tables.address_table.element_count(), 256);
    assert_eq!(tables.address_table.length_value(), [1, 0]);
    assert_eq!(tables.association_table.element_count(), 256);
    assert_eq!(tables.association_table.length_value(), [1, 0]);
    assert_eq!(&tables.address_table.elements()[508..], &[0, 255, 1, 0]);
    assert_eq!(
        &tables.association_table.elements()[1016..],
        &[0, 255, 0, 255, 1, 0, 1, 0]
    );
    assert_eq!(tables.address_table.elements().len(), 512);
    assert_eq!(tables.association_table.elements().len(), 1024);
}

#[test]
fn format0_count_is_a_word_even_when_the_same_address_has_many_associations() {
    let links: Vec<_> = (1..=128)
        .flat_map(|object| [link(object, 0x1234, true), link(object, 0x5678, false)])
        .collect();
    let tables = build(AssociationFormat::Format0, 128, &links, ROOMY).unwrap();
    assert_eq!(tables.address_table.length_value(), [0, 2]);
    assert_eq!(tables.association_table.element_count(), 256);
    assert_eq!(tables.association_table.length_value(), [1, 0]);
    assert_eq!(tables.association_table.elements().len(), 512);
    assert_eq!(
        &tables.association_table.elements()[508..],
        &[1, 128, 2, 128]
    );
}

#[test]
fn input_permutation_never_changes_address_indices_or_the_sending_row() {
    let links = [
        link(1, 0x3456, true),
        link(1, 0x1234, false),
        link(2, 0x1234, true),
    ];
    for format in [AssociationFormat::Format0, AssociationFormat::Format1] {
        let expected = build(format, 2, &links, ROOMY).unwrap();
        let reversed: Vec<_> = links.iter().copied().rev().collect();
        assert_eq!(build(format, 2, &reversed, ROOMY).unwrap(), expected);
        assert_eq!(expected.address_table.element_count(), 2);
        assert_eq!(expected.association_table.element_count(), 3);
    }
}

#[test]
fn invalid_indices_and_unrepresentable_formats_refuse_instead_of_narrowing() {
    for (format, count, object, expected) in [
        (
            AssociationFormat::Format0,
            1,
            0,
            TableError::ObjectOutOfTable {
                number: 0,
                count: 1,
            },
        ),
        (
            AssociationFormat::Format1,
            1,
            2,
            TableError::ObjectOutOfTable {
                number: 2,
                count: 1,
            },
        ),
        (
            AssociationFormat::Format0,
            256,
            256,
            TableError::ObjectNotRepresentable { number: 256 },
        ),
    ] {
        assert_eq!(
            build(format, count, &[link(object, 0x1234, true)], ROOMY).unwrap_err(),
            expected
        );
    }
    let tables = build(
        AssociationFormat::Format1,
        u16::MAX,
        &[link(u16::MAX, 0x1234, true)],
        ROOMY,
    )
    .unwrap();
    assert_eq!(tables.association_table.elements(), &[0, 1, 255, 255]);
    let narrow = build(
        AssociationFormat::Format0,
        255,
        &[link(255, 0x1234, true)],
        ROOMY,
    )
    .unwrap();
    assert_eq!(narrow.association_table.elements(), &[1, 255]);
}

#[test]
fn duplicate_broadcast_and_missing_or_multiple_senders_remain_explicit_refusals() {
    for format in [AssociationFormat::Format0, AssociationFormat::Format1] {
        for (links, expected) in [
            (
                vec![link(1, 0, true)],
                TableError::BroadcastAddress { object: 1 },
            ),
            (
                vec![link(1, 0x1234, true), link(1, 0x1234, false)],
                TableError::DuplicateLink {
                    object: 1,
                    group_address: GroupAddress::from_raw(0x1234),
                },
            ),
            (
                vec![link(1, 0x1234, false)],
                TableError::NoSendingAddress { object: 1 },
            ),
            (
                vec![link(1, 0x1234, true), link(1, 0x5678, true)],
                TableError::SeveralSendingAddresses { object: 1 },
            ),
        ] {
            assert_eq!(build(format, 1, &links, ROOMY).unwrap_err(), expected);
        }
    }
}

#[test]
fn capacities_do_not_truncate_or_pad_property_elements() {
    let links = [link(1, 0x1234, true), link(1, 0x5678, false)];
    for format in [AssociationFormat::Format0, AssociationFormat::Format1] {
        assert_eq!(
            build(
                format,
                1,
                &links,
                TableCapacity {
                    group_addresses: 1,
                    associations: 2
                }
            )
            .unwrap_err(),
            TableError::TooManyGroupAddresses {
                count: 2,
                capacity: 1
            }
        );
        assert_eq!(
            build(
                format,
                1,
                &links,
                TableCapacity {
                    group_addresses: 2,
                    associations: 1
                }
            )
            .unwrap_err(),
            TableError::TooManyAssociations {
                count: 2,
                capacity: 1
            }
        );
        let exact = build(
            format,
            1,
            &links,
            TableCapacity {
                group_addresses: 2,
                associations: 2,
            },
        )
        .unwrap();
        let roomy = build(format, 1, &links, ROOMY).unwrap();
        assert_eq!(roomy, exact);
        assert_eq!(exact.address_table.elements().len(), 4);
        assert_eq!(
            exact.association_table.elements().len(),
            2 * exact.association_table.element_width()
        );
    }
}

#[test]
fn format0_tsap_width_is_not_extended_by_a_roomier_property_capacity() {
    let links: Vec<_> = (1..=256)
        .map(|address| link(1, address, address == 1))
        .collect();
    assert_eq!(
        build(AssociationFormat::Format0, 1, &links, ROOMY).unwrap_err(),
        TableError::TooManyGroupAddresses {
            count: 256,
            capacity: 255
        }
    );
    let exact = build(AssociationFormat::Format0, 1, &links[..255], ROOMY).unwrap();
    assert_eq!(exact.address_table.element_count(), 255);
    assert_eq!(&exact.association_table.elements()[508..], &[255, 1]);
}

#[test]
fn association_count_boundary_preserves_65535_rows_and_refuses_overflow() {
    let links: Vec<_> = (0..65536u32)
        .map(|n| link((n / 256 + 1) as u16, (n % 256 + 1) as u16, n % 256 == 0))
        .collect();
    let tables = build(
        AssociationFormat::Format1,
        256,
        &links[..65535],
        TableCapacity {
            group_addresses: usize::MAX,
            associations: usize::MAX,
        },
    )
    .unwrap();
    assert_eq!(tables.association_table.element_count(), 65535);
    assert_eq!(tables.association_table.length_value(), [255, 255]);
    assert_eq!(tables.association_table.elements().len(), 262140);
    assert_eq!(
        &tables.association_table.elements()[262136..],
        &[0, 255, 1, 0]
    );
    assert_eq!(
        build(
            AssociationFormat::Format1,
            256,
            &links,
            TableCapacity {
                group_addresses: usize::MAX,
                associations: usize::MAX
            }
        )
        .unwrap_err(),
        TableError::TooManyAssociations {
            count: 65536,
            capacity: 65535
        }
    );
}

#[test]
fn last_word_tsap_and_address_survive_the_selected_codec_representation() {
    let links: Vec<_> = (1..=u16::MAX)
        .map(|address| link(1, address, address == 1))
        .collect();
    let tables = build(
        AssociationFormat::Format1,
        1,
        &links,
        TableCapacity {
            group_addresses: usize::MAX,
            associations: usize::MAX,
        },
    )
    .unwrap();
    assert_eq!(tables.address_table.element_count(), u16::MAX);
    assert_eq!(tables.address_table.length_value(), [255, 255]);
    assert_eq!(tables.address_table.elements().len(), 131070);
    assert_eq!(
        &tables.address_table.elements()[131066..],
        &[255, 254, 255, 255]
    );
    assert_eq!(
        &tables.association_table.elements()[262136..],
        &[255, 255, 0, 1]
    );
}
