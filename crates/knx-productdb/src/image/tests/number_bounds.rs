//! A declared TypeNumber bound the image builder cannot read refuses the build, never vanishes.

use super::*;

const SEGMENT: &str = "AS-4400";
const OCTET: usize = 12;
const UNSIGNED_BOUNDS: &str =
    r#"SizeInBit="8" Type="unsignedInt" minInclusive="0" maxInclusive="100""#;
const U64_OVERFLOW: &str = "18446744073709551616";
const I64_OVERFLOW: &str = "9223372036854775808";

/// `PROGRAM` whose `PT-N` is a one-octet `number_type` with the given bound attributes.
fn program(number_type: &str, min: Option<&str>, max: Option<&str>) -> String {
    let attr = |name: &str, value: Option<&str>| {
        value.map_or(String::new(), |v| format!(r#" {name}="{v}""#))
    };
    let xml = PROGRAM.replace(
        &format!("<TypeNumber {UNSIGNED_BOUNDS} />"),
        &format!(
            r#"<TypeNumber SizeInBit="8" Type="{number_type}"{}{} />"#,
            attr("minInclusive", min),
            attr("maxInclusive", max)
        ),
    );
    assert!(
        xml.contains(&format!(r#"Type="{number_type}""#)),
        "fixture must replace PT-N"
    );
    xml
}

/// Builds through the public entry after proving the database kept the bounds verbatim.
fn build_with_bounds(
    number_type: &str,
    min: Option<&str>,
    max: Option<&str>,
    values: &[(&str, &str)],
) -> Result<DownloadImage, ImageError> {
    let (_dir, conn) = db(&program(number_type, min, max));
    let stored: (Option<String>, Option<String>) = conn
        .query_row(
            "SELECT min_inclusive, max_inclusive FROM parameter_type
             WHERE program_id = ?1 AND id = 'PT-N'",
            [ID],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .expect("PT-N row");
    assert_eq!(
        stored,
        (min.map(String::from), max.map(String::from)),
        "the importer keeps the raw bound text"
    );
    build_download_image(&conn, &request(values, vec![]))
}

fn written(image: &DownloadImage) -> u8 {
    octets(image, SEGMENT)[OCTET]
}

/// Refusal for the bound itself: `P-2_R-2`, the bound's attribute, its raw text and the type.
fn refused_for(
    result: Result<DownloadImage, ImageError>,
    attribute: &str,
    raw: &str,
    number_type: &str,
) -> Result<(), String> {
    match result {
        Ok(image) => Err(format!(
            "{attribute}={raw:?} admitted, octet {OCTET} = {:02X}",
            written(&image)
        )),
        Err(ImageError::Value {
            parameter_ref,
            cause,
        }) if parameter_ref == "P-2_R-2"
            && cause.contains(attribute)
            && cause.contains(&format!("{raw:?}"))
            && cause.contains(number_type) =>
        {
            Ok(())
        }
        Err(other) => Err(format!("{attribute}={raw:?}: wrong refusal: {other}")),
    }
}

fn assert_all(failures: Vec<String>) {
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn unsigned_unreadable_max_refuses_a_requested_value_that_would_fit() {
    let failures = ["", "abc", "100.0", "1e2", U64_OVERFLOW, "-1"]
        .into_iter()
        .filter_map(|raw| {
            let result = build_with_bounds("unsignedInt", None, Some(raw), &[("P-2_R-2", "200")]);
            refused_for(result, "maxInclusive", raw, "unsignedInt").err()
        })
        .collect();
    assert_all(failures);
}

#[test]
fn unsigned_unreadable_min_refuses_a_requested_value_that_would_fit() {
    let failures = ["", "abc", "0.0", "5e0", U64_OVERFLOW]
        .into_iter()
        .filter_map(|raw| {
            let result = build_with_bounds("unsignedInt", Some(raw), None, &[("P-2_R-2", "50")]);
            refused_for(result, "minInclusive", raw, "unsignedInt").err()
        })
        .collect();
    assert_all(failures);
}

#[test]
fn signed_unreadable_max_refuses_a_requested_value_that_would_fit() {
    let failures = ["", "abc", "100.0", "1e2", I64_OVERFLOW]
        .into_iter()
        .filter_map(|raw| {
            let result = build_with_bounds("signedInt", None, Some(raw), &[("P-2_R-2", "120")]);
            refused_for(result, "maxInclusive", raw, "signedInt").err()
        })
        .collect();
    assert_all(failures);
}

#[test]
fn signed_unreadable_min_refuses_a_requested_value_that_would_fit() {
    let failures = [
        "",
        "abc",
        "0.0",
        "5e0",
        I64_OVERFLOW,
        "-9223372036854775809",
    ]
    .into_iter()
    .filter_map(|raw| {
        let result = build_with_bounds("signedInt", Some(raw), None, &[("P-2_R-2", "50")]);
        refused_for(result, "minInclusive", raw, "signedInt").err()
    })
    .collect();
    assert_all(failures);
}

#[test]
fn an_unreadable_bound_refuses_the_source_default_without_any_request_value() {
    let mut failures = Vec::new();
    for (number_type, min, max, attribute, raw) in [
        ("unsignedInt", None, Some("100.0"), "maxInclusive", "100.0"),
        ("unsignedInt", Some("abc"), None, "minInclusive", "abc"),
        ("signedInt", None, Some("1e2"), "maxInclusive", "1e2"),
        ("signedInt", Some(""), None, "minInclusive", ""),
    ] {
        let result = build_with_bounds(number_type, min, max, &[]);
        failures.extend(refused_for(result, attribute, raw, number_type).err());
    }
    assert_all(failures);
}

#[test]
fn both_bounds_are_read_before_the_range_so_a_range_miss_cannot_hide_one() {
    // 50 is below the valid minimum 60: the unreadable maximum is still the named reason.
    let result = build_with_bounds("unsignedInt", Some("60"), Some("abc"), &[("P-2_R-2", "50")]);
    assert_all(
        refused_for(result, "maxInclusive", "abc", "unsignedInt")
            .err()
            .into_iter()
            .collect(),
    );
    // 200 is above the valid maximum 100: the unreadable minimum is still the named reason.
    let result = build_with_bounds("signedInt", Some("x"), Some("100"), &[("P-2_R-2", "120")]);
    assert_all(
        refused_for(result, "minInclusive", "x", "signedInt")
            .err()
            .into_iter()
            .collect(),
    );
}

#[test]
fn two_unreadable_bounds_report_the_minimum_first() {
    let result = build_with_bounds("unsignedInt", Some("lo"), Some("hi"), &[("P-2_R-2", "50")]);
    assert_all(
        refused_for(result, "minInclusive", "lo", "unsignedInt")
            .err()
            .into_iter()
            .collect(),
    );
}

#[test]
fn two_unreadable_signed_bounds_report_the_minimum_first() {
    let result = build_with_bounds("signedInt", Some("lo"), Some("hi"), &[("P-2_R-2", "50")]);
    assert_all(
        refused_for(result, "minInclusive", "lo", "signedInt")
            .err()
            .into_iter()
            .collect(),
    );
}

#[test]
fn readable_unsigned_bounds_are_inclusive_and_one_beyond_is_refused() {
    for (value, expected) in [
        ("10", Some(10u8)),
        ("20", Some(20)),
        ("9", None),
        ("21", None),
    ] {
        let result =
            build_with_bounds("unsignedInt", Some("10"), Some("20"), &[("P-2_R-2", value)]);
        match (result, expected) {
            (Ok(image), Some(octet)) => assert_eq!(written(&image), octet, "{value}"),
            (
                Err(ImageError::Value {
                    parameter_ref,
                    cause,
                }),
                None,
            ) => {
                assert_eq!(parameter_ref, "P-2_R-2");
                assert!(cause.contains("is outside"), "{cause}");
            }
            (other, _) => panic!("{value}: {:?}", other.map(|i| written(&i))),
        }
    }
}

#[test]
fn readable_signed_bounds_are_inclusive_and_one_beyond_is_refused() {
    for (value, expected) in [
        ("10", Some(10u8)),
        ("20", Some(20)),
        ("9", None),
        ("21", None),
    ] {
        let result = build_with_bounds("signedInt", Some("10"), Some("20"), &[("P-2_R-2", value)]);
        match (result, expected) {
            (Ok(image), Some(octet)) => assert_eq!(written(&image), octet, "{value}"),
            (
                Err(ImageError::Value {
                    parameter_ref,
                    cause,
                }),
                None,
            ) => {
                assert_eq!(parameter_ref, "P-2_R-2");
                assert!(cause.contains("is outside"), "{cause}");
            }
            (other, _) => panic!("{value}: {:?}", other.map(|i| written(&i))),
        }
    }
}

#[test]
fn absent_bounds_and_whitespace_around_readable_bounds_keep_their_behaviour() {
    for number_type in ["unsignedInt", "signedInt"] {
        let image = build_with_bounds(number_type, None, None, &[("P-2_R-2", "127")]).unwrap();
        assert_eq!(written(&image), 127, "{number_type}: no bounds at all");
        let image = build_with_bounds(
            number_type,
            Some(" 10 "),
            Some("  20 "),
            &[("P-2_R-2", "20")],
        )
        .unwrap();
        assert_eq!(written(&image), 20, "{number_type}: padded bounds");
        let image =
            build_with_bounds(number_type, Some("10"), None, &[("P-2_R-2", "127")]).unwrap();
        assert_eq!(written(&image), 127, "{number_type}: only a minimum");
        let image = build_with_bounds(number_type, None, Some("60"), &[]).unwrap();
        assert_eq!(
            written(&image),
            50,
            "{number_type}: only a maximum, source default"
        );
    }
}

#[test]
fn the_default_value_is_still_checked_against_readable_bounds_only() {
    // Default 50 inside 0..=100 builds; the same default outside 60..=100 is a range refusal.
    let image = build_with_bounds("unsignedInt", Some("0"), Some("100"), &[]).unwrap();
    assert_eq!(written(&image), 50);
    let error = build_with_bounds("unsignedInt", Some("60"), Some("100"), &[]).unwrap_err();
    assert!(
        matches!(&error, ImageError::Value { cause, .. } if cause.contains("is outside")),
        "{error}"
    );
}

#[test]
fn a_u64_max_maximum_is_a_readable_bound_for_an_8_bit_field() {
    let image = build_with_bounds(
        "unsignedInt",
        Some("0"),
        Some("18446744073709551615"),
        &[("P-2_R-2", "255")],
    )
    .unwrap();
    assert_eq!(written(&image), 255);
}

#[test]
fn a_signed_bound_at_i64_max_is_readable_and_a_positive_value_still_encodes() {
    let image = build_with_bounds(
        "signedInt",
        Some("0"),
        Some("9223372036854775807"),
        &[("P-2_R-2", "127")],
    )
    .unwrap();
    assert_eq!(written(&image), 127);
}

#[test]
fn a_signed_bound_at_i64_min_is_readable_and_a_nonnegative_value_still_encodes() {
    let image = build_with_bounds(
        "signedInt",
        Some("-9223372036854775808"),
        Some("100"),
        &[("P-2_R-2", "127")],
    );
    // 127 is above the maximum 100: the minimum read fine and the range names the miss.
    assert!(
        matches!(&image, Err(ImageError::Value { cause, .. }) if cause.contains("is outside")),
        "{:?}",
        image.map(|i| written(&i))
    );
    let image = build_with_bounds(
        "signedInt",
        Some("-9223372036854775808"),
        Some("200"),
        &[("P-2_R-2", "127")],
    )
    .unwrap();
    assert_eq!(written(&image), 127);
}

/// Integer `FromStr` is what reads a bound today, so a sign and leading zeros are admitted.
/// This pins the implementation, not an XML lexical rule (see the contract).
#[test]
fn a_plus_sign_and_leading_zeros_read_as_the_integer_parse_reads_them() {
    for number_type in ["unsignedInt", "signedInt"] {
        let image = build_with_bounds(number_type, Some("+10"), Some("0020"), &[("P-2_R-2", "20")])
            .unwrap();
        assert_eq!(written(&image), 20, "{number_type}");
        let image =
            build_with_bounds(number_type, Some("00"), Some("+0"), &[("P-2_R-2", "0")]).unwrap();
        assert_eq!(written(&image), 0, "{number_type}");
    }
}

#[test]
fn an_unsigned_minimum_outside_u64_refuses_a_value_that_would_fit() {
    // No negative text is a u64: not "-1", not "-0", not below i64::MIN.
    let failures = ["-1", "-0", "-9223372036854775809"]
        .into_iter()
        .filter_map(|raw| {
            let result = build_with_bounds("unsignedInt", Some(raw), None, &[("P-2_R-2", "200")]);
            refused_for(result, "minInclusive", raw, "unsignedInt").err()
        })
        .collect();
    assert_all(failures);
}
