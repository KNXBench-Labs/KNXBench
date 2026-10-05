//! Independent DPT-AS format-width audit of the codec's payload length rule (AR09).
//!
//! Each row is read from the `Format:` line of 03_07_02 Datapoint Types
//! v02.02.01 AS for that main type, not from the codec. A codec that accepted a
//! different width, or reported a different required width, would fail here.

use knx_core::dpt::codec::{decode, format_width_bits, DptCodecError};
use knx_core::dpt::{DptRef, GroupValue};

/// (main, sub, DPT-AS section, spec width in bits).
const FIXED_WIDTH: &[(u16, u16, &str, u32)] = &[
    (1, 1, "3.1 B1", 1),
    (2, 1, "3.2 B2", 2),
    (3, 7, "3.3 B1U3", 4),
    (4, 1, "3.4 A8", 8),
    (5, 1, "3.5 U8", 8),
    (6, 1, "3.6 V8", 8),
    (6, 20, "3.7 B5N3", 8),
    (7, 1, "3.8 U16", 16),
    (8, 1, "3.9 V16", 16),
    (9, 1, "3.10 F16", 16),
    (10, 1, "3.11 N3U5r2U6r2U6", 24),
    (11, 1, "3.12 r3U5r4U4r1U7", 24),
    (12, 1, "3.13 U32", 32),
    (13, 1, "3.14 V32", 32),
    (14, 0, "3.15 F32", 32),
    (15, 0, "3.16 U4U4U4U4U4U4B4N4", 32),
    (16, 0, "3.17 A112", 112),
    (17, 1, "3.18 r2U6", 8),
    (18, 1, "3.19 B1r1U6", 8),
    (19, 1, "3.20 U8[r4U4][r3U5][U3U5][r2U6][r2U6]B16", 64),
    (20, 1, "3.21 N8", 8),
    (21, 1, "3.22 B8", 8),
    (22, 100, "8.3 B16", 16),
    (23, 1, "3.23 N2", 2),
    (25, 1000, "8.4 U4U4", 8),
    (26, 1, "3.25 r1B1U6", 8),
    (27, 1, "3.26 B32", 32),
    (29, 10, "3.28 V64", 64),
    (30, 1010, "8.5 B24", 24),
];

fn dpt(main: u16, sub: u16) -> DptRef {
    DptRef {
        main,
        sub: Some(sub),
    }
}

/// A payload of exactly `bits` significant bits, all zero.
fn payload(bits: u32) -> GroupValue {
    if bits <= 6 {
        GroupValue::Short(0)
    } else {
        GroupValue::Bytes(vec![0; (bits / 8) as usize])
    }
}

#[test]
fn codec_accepts_exactly_the_dpt_as_format_width() {
    for &(main, sub, section, bits) in FIXED_WIDTH {
        let result = decode(dpt(main, sub), &payload(bits));
        assert!(
            !matches!(result, Err(DptCodecError::WrongLength { .. })),
            "{main}.{sub:03} (DPT-AS {section}): spec width {bits} bit refused: {result:?}"
        );
    }
}

#[test]
fn codec_refuses_one_octet_more_and_names_the_dpt_as_width() {
    for &(main, sub, section, bits) in FIXED_WIDTH {
        let wider = GroupValue::Bytes(vec![0; (bits.div_ceil(8) + 1) as usize]);
        match decode(dpt(main, sub), &wider) {
            Err(DptCodecError::WrongLength { expected_bits, .. }) => assert_eq!(
                expected_bits, bits,
                "{main}.{sub:03} (DPT-AS {section}): reported width differs from the spec"
            ),
            other => panic!(
                "{main}.{sub:03} (DPT-AS {section}): oversized payload not refused: {other:?}"
            ),
        }
    }
}

#[test]
fn codec_refuses_one_octet_less_for_multi_octet_formats() {
    for &(main, sub, section, bits) in FIXED_WIDTH.iter().filter(|row| row.3 >= 16) {
        let shorter = GroupValue::Bytes(vec![0; (bits / 8 - 1) as usize]);
        assert!(
            matches!(
                decode(dpt(main, sub), &shorter),
                Err(DptCodecError::WrongLength { .. })
            ),
            "{main}.{sub:03} (DPT-AS {section}): truncated payload not refused"
        );
    }
}

#[test]
fn the_public_width_table_matches_the_dpt_as_rows() {
    for &(main, sub, section, bits) in FIXED_WIDTH {
        assert_eq!(
            format_width_bits(main),
            Some(bits),
            "{main}.{sub:03} (DPT-AS {section})"
        );
    }
    for variable_or_unknown in [0, 24, 28, 31, 232] {
        assert_eq!(format_width_bits(variable_or_unknown), None);
    }
}
