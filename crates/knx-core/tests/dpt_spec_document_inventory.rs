//! DPT-AS v02.02.01 document-wide support/format inventory, not subtype conformance.
//!
//! The checked-in CSV is a primary-source audit artifact (454 distinct IDs),
//! independent of the implementation dispatch. Every numbered DPT is exercised.
//! Subtype enums, units, narrowed ranges and FB use remain explicit limitations.

use knx_core::{
    decode, encode, encode_inferred_format, DptCodecError, DptInputFormat, DptRef, GroupValue,
};
use std::collections::BTreeSet;

const INVENTORY: &str = include_str!("../../../docs/spec-audits/2026-10-07-dpt-types.csv");

fn rows() -> Vec<(DptRef, Option<u32>)> {
    INVENTORY
        .lines()
        .skip(1)
        .map(|line| {
            let cells: Vec<_> = line.split(',').collect();
            assert_eq!(cells.len(), 12, "malformed inventory row");
            let main = cells[1].parse().unwrap();
            let sub = cells[2].parse().unwrap();
            assert_eq!(cells[0], format!("{main}.{sub:03}"));
            (
                DptRef {
                    main,
                    sub: Some(sub),
                },
                cells[4].parse().ok(),
            )
        })
        .collect()
}

fn specimen(dpt: DptRef, bits: u32) -> GroupValue {
    if bits != 0 && bits <= 6 {
        return GroupValue::Short(0);
    }
    let mut bytes = vec![0; if bits == 0 { 1 } else { (bits / 8) as usize }];
    match (dpt.main, dpt.sub) {
        (6, Some(20)) => bytes[0] = 1,
        (11, _) => {
            bytes[0] = 1;
            bytes[1] = 1;
        }
        (19, _) => {
            bytes[0] = 100;
            bytes[1] = 1;
            bytes[2] = 1;
        }
        _ => {}
    }
    GroupValue::Bytes(bytes)
}

#[test]
fn inventory_has_all_numbered_definition_ids_including_the_overview_omission() {
    let rows = rows();
    let unique: BTreeSet<_> = rows.iter().map(|(dpt, _)| *dpt).collect();
    assert_eq!(rows.len(), 454);
    assert_eq!(unique.len(), 454);
    assert_eq!(
        unique
            .iter()
            .map(|dpt| dpt.main)
            .collect::<BTreeSet<_>>()
            .len(),
        103
    );
    assert_eq!(rows.iter().filter(|(_, bits)| bits.is_some()).count(), 305);
    assert_eq!(rows.iter().filter(|(_, bits)| bits.is_none()).count(), 149);
    assert!(unique.contains(&DptRef {
        main: 249,
        sub: Some(600)
    }));
}

#[test]
fn every_in_scope_dpt_accepts_a_format_specimen_and_reencodes_it() {
    for (dpt, bits) in rows() {
        let Some(bits) = bits else {
            continue;
        };
        let wire = specimen(dpt, bits);
        let value = decode(dpt, &wire).unwrap_or_else(|e| panic!("{dpt}: {e}"));
        // Scene display is labelled (`scene 0`); the documented write
        // grammar is a decimal scene number, not inverse Display parsing.
        let input = if dpt.main == 17 {
            "0".to_owned()
        } else {
            value.to_string()
        };
        assert_eq!(
            encode_inferred_format(dpt, &input).unwrap_or_else(|e| panic!("{dpt}: {e}")),
            wire,
            "{dpt}"
        );
    }
}

#[test]
fn every_fixed_width_dpt_refuses_extra_or_missing_octets() {
    for (dpt, bits) in rows() {
        let Some(bits) = bits.filter(|&bits| bits != 0) else {
            continue;
        };
        let too_long = GroupValue::Bytes(vec![0; bits.div_ceil(8) as usize + 1]);
        assert!(
            matches!(decode(dpt, &too_long),
            Err(DptCodecError::WrongLength { expected_bits, .. }) if expected_bits == bits),
            "{dpt}"
        );
        if bits > 8 {
            assert!(
                matches!(decode(dpt, &GroupValue::Bytes(vec![0; (bits / 8 - 1) as usize])),
                Err(DptCodecError::WrongLength { expected_bits, .. }) if expected_bits == bits),
                "{dpt}"
            );
        }
    }
}

#[test]
fn every_out_of_scope_dpt_is_explicitly_unsupported_in_both_directions() {
    for (dpt, bits) in rows() {
        if bits.is_some() {
            continue;
        }
        assert_eq!(
            decode(dpt, &GroupValue::Bytes(vec![0])),
            Err(DptCodecError::UnsupportedDpt(dpt))
        );
        assert_eq!(
            encode_inferred_format(dpt, "0"),
            Err(DptCodecError::UnsupportedDpt(dpt))
        );
        assert_eq!(
            encode(dpt, "0", DptInputFormat::Decimal),
            Err(DptCodecError::UnsupportedDpt(dpt))
        );
    }
}
