//! DPT-AS v02.02.01 numeric edge regressions from the full-document audit.
//!
//! Independent engineering bounds: §3.9.1 p37, §3.10 p39 and §3.14.1 p43.
//! Quantization must not manufacture an invalid sentinel or admit an input
//! outside the format's engineering range. No bus or private corpus access.

use knx_core::{decode, encode, encode_inferred_format, DptCodecError, DptInputFormat, DptRef};

fn dpt(main: u16, sub: u16) -> DptRef {
    DptRef {
        main,
        sub: Some(sub),
    }
}

fn refuses_both_encoders(dpt: DptRef, input: &str) {
    for result in [
        encode(dpt, input, DptInputFormat::Decimal),
        encode_inferred_format(dpt, input),
    ] {
        assert!(
            matches!(result, Err(DptCodecError::OutOfRange { .. })),
            "{dpt} accepted out-of-range/sentinel input {input}: {result:?}"
        );
    }
}

#[test]
fn percent_v16_rounding_cannot_generate_the_invalid_sentinel() {
    // Values below the printed arithmetic ceiling still round to 0x7FFF.
    for input in ["327.665", "327.666", "327.669", "327.67"] {
        refuses_both_encoders(dpt(8, 10), input);
    }
}

#[test]
fn f16_inputs_outside_the_printed_format_range_are_not_rounded_back_in() {
    for input in ["-671088.65", "-671100", "670433.29", "670434"] {
        refuses_both_encoders(dpt(9, 1), input);
    }
}

#[test]
fn flow_rate_inputs_outside_the_scaled_v32_range_are_not_rounded_back_in() {
    for input in ["-214748.36481", "214748.36471"] {
        refuses_both_encoders(dpt(13, 2), input);
    }
}

#[test]
fn runtime_restrictions_do_not_remove_parameter_or_diagnostic_codecs() {
    for (main, sub) in [
        (7, 3),
        (7, 4),
        (7, 6),
        (7, 13),
        (8, 3),
        (8, 4),
        (8, 6),
        (8, 12),
        (20, 22),
    ] {
        let dpt = dpt(main, sub);
        assert!(matches!(knx_core::validate_group_write_dpt(dpt),
            Err(DptCodecError::RuntimeRestricted { dpt: got, .. }) if got == dpt));
        let wire = encode(dpt, "1", DptInputFormat::Decimal).unwrap();
        assert!(decode(dpt, &wire).is_ok());
    }
    for (main, sub) in [
        (7, 2),
        (7, 5),
        (7, 7),
        (7, 12),
        (8, 2),
        (8, 5),
        (8, 7),
        (8, 11),
        (20, 102),
    ] {
        assert!(knx_core::validate_group_write_dpt(dpt(main, sub)).is_ok());
    }
    for main in [7, 8, 20] {
        assert!(knx_core::validate_group_write_dpt(DptRef { main, sub: None }).is_ok());
    }
}

#[test]
fn numeric_limits_and_in_range_rounding_remain_usable() {
    for (dpt, inputs) in [
        (dpt(8, 10), vec!["-327.68", "327.66", "327.664", "1.234"]),
        (dpt(9, 1), vec!["-671088.64", "670433.28", "21.5", "-0.01"]),
        (dpt(13, 2), vec!["-214748.3648", "214748.3647", "1.23456"]),
    ] {
        for input in inputs {
            let wire = encode(dpt, input, DptInputFormat::Decimal).unwrap();
            assert!(
                decode(dpt, &wire).is_ok(),
                "{dpt}: {input} encoded undecodable data"
            );
            assert_eq!(wire, encode_inferred_format(dpt, input).unwrap());
        }
    }
}
