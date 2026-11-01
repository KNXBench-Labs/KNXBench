//! Tests source candidates: exact TypeNumber comparisons and strict Union intervals, no repair.

use crate::distribution_inspection::*;
use crate::standalone_packages::wp16::{raw_zip, RawMember};
use knx_productdb::distribution::{
    inspect_distribution_with_limits, DistributionActivityInput, DistributionActivityReason,
    DistributionActivityState as Activity, DistributionArithmeticModel as Model,
    DistributionBitInterval, DistributionCandidateInitialization,
    DistributionCandidateKind as Kind, DistributionCandidateRows, DistributionCandidateSeverity,
    DistributionDecimalClass as Decimal, DistributionDependencyKind, DistributionDistinctInterval,
    DistributionEndpointRole, DistributionImpactScope, DistributionInspection,
    DistributionInspectionError as E, DistributionIntegerClass as Integer,
    DistributionIntervalComparison as Interval, DistributionLimits as L,
    DistributionPresence as Presence, DistributionReferenceState, DistributionResolutionBasis,
    DistributionSideComparison as Side, DistributionSourceCandidate, DistributionSourceRef,
    DistributionTypeShape, DistributionValueComparison as Cmp,
};
use std::collections::BTreeSet;

// Independently authored fixtures, byte-identical to the readiness packet; no vendor data.
const VALUES: &[u8] = include_bytes!("distribution_candidates/values.xml");
const UNION: &[u8] = include_bytes!("distribution_candidates/union-alternatives.xml");

/// Lists every checked occurrence, so exact arithmetic is asserted on `within` rows too.
fn inspect(bytes: &[u8]) -> DistributionInspection {
    inspect_distribution_with_limits("authored.xml", bytes, all_rows()).unwrap()
}
fn all_rows() -> L {
    L {
        candidate_rows: DistributionCandidateRows::All,
        ..L::default()
    }
}
fn rows(r: &DistributionInspection, kind: Kind) -> Vec<&DistributionSourceCandidate> {
    r.source_candidates
        .iter()
        .filter(|c| c.kind == kind)
        .collect()
}
fn original<'a>(r: &DistributionInspection, bytes: &'a [u8], s: DistributionSourceRef) -> &'a [u8] {
    let c = r.resolve_coordinate(s).unwrap();
    &bytes[c.start_byte as usize..c.end_byte as usize]
}
/// The `Id` of the element a row points at, read back from the original bytes.
fn id_of(r: &DistributionInspection, bytes: &[u8], s: DistributionSourceRef) -> String {
    let text = std::str::from_utf8(original(r, bytes, s)).unwrap();
    let start = text.find(" Id=\"").expect("Id") + 5;
    text[start..start + text[start..].find('"').unwrap()].to_string()
}
fn by_id<'a>(
    r: &'a DistributionInspection,
    bytes: &[u8],
    kind: Kind,
    id: &str,
) -> &'a DistributionSourceCandidate {
    let found = rows(r, kind)
        .into_iter()
        .filter(|c| id_of(r, bytes, c.source) == id)
        .collect::<Vec<_>>();
    assert_eq!(found.len(), 1, "{kind:?} {id}");
    found[0]
}
fn identity(r: &DistributionInspection, kind: &str, id: &str) -> Vec<DistributionSourceRef> {
    r.identities
        .iter()
        .filter(|i| i.kind == kind && i.declared_id == id)
        .map(|i| i.source)
        .collect()
}

/// Authored application with the given Static parts and Dynamic body.
fn app(types: &str, parameters: &str, refs: &str, dynamic: &str) -> Vec<u8> {
    format!(
        "<KNX xmlns=\"http://knx.org/xml/project/20\"><ManufacturerData><Manufacturer RefId=\"M-1234\">\
         <ApplicationPrograms><ApplicationProgram Id=\"APP-A\" Name=\"Authored\" ApplicationNumber=\"7\" \
         ApplicationVersion=\"1\" MaskVersion=\"MV-0701\"><Static><ParameterTypes>{types}</ParameterTypes>\
         <Parameters>{parameters}</Parameters><ParameterRefs>{refs}</ParameterRefs></Static>\
         <Dynamic>{dynamic}</Dynamic></ApplicationProgram></ApplicationPrograms></Manufacturer>\
         </ManufacturerData></KNX>"
    )
    .into_bytes()
}
fn attr(name: &str, value: Option<&str>) -> String {
    value.map_or(String::new(), |v| format!(" {name}=\"{v}\""))
}

/// (id, raw Value, minInclusive, maxInclusive); `None` omits the attribute.
type RangeCase<'a> = (&'a str, Option<&'a str>, Option<&'a str>, Option<&'a str>);

/// A range case plus its expected comparison.
type RangeVector = (
    &'static str,
    Option<&'static str>,
    Option<&'static str>,
    Option<&'static str>,
    Cmp,
);

/// Literal `vectors.json:ranges` rows (id, raw, min, max, expectation).
const RANGES: &[RangeVector] = &[
    ("absent", None, Some("1"), Some("64"), Cmp::ValueAbsent),
    ("empty", Some(""), Some("1"), Some("64"), Cmp::ValueEmpty),
    ("zero", Some("0"), Some("1"), Some("64"), Cmp::Below),
    ("low", Some("1"), Some("1"), Some("64"), Cmp::Within),
    ("high", Some("64"), Some("1"), Some("64"), Cmp::Within),
    ("above", Some("65"), Some("1"), Some("64"), Cmp::Above),
    ("lexical", Some("0002"), Some("1"), Some("64"), Cmp::Within),
    (
        "big",
        Some("9007199254740993"),
        Some("0"),
        Some("9007199254740992"),
        Cmp::Above,
    ),
    (
        "fraction",
        Some("1.25"),
        Some("1.00"),
        Some("1.20"),
        Cmp::Above,
    ),
    (
        "notfinite",
        Some("NaN"),
        Some("1"),
        Some("64"),
        Cmp::NotFiniteDecimal,
    ),
    (
        "badbound",
        Some("2"),
        Some("bad"),
        Some("64"),
        Cmp::BoundInvalid,
    ),
    (
        "reversed",
        Some("2"),
        Some("64"),
        Some("1"),
        Cmp::InvalidDeclaredRange,
    ),
    ("unbounded", Some("2"), None, None, Cmp::Unbounded),
];

/// One TypeNumber type and one Parameter per (id, value, min, max) row.
fn ranges_app(cases: &[RangeCase<'_>]) -> Vec<u8> {
    let mut types = String::new();
    let mut parameters = String::new();
    for (id, value, min, max) in cases {
        types.push_str(&format!(
            "<ParameterType Id=\"T-{id}\"><TypeNumber SizeInBit=\"64\" Type=\"signedInt\"{}{}/></ParameterType>",
            attr("minInclusive", *min),
            attr("maxInclusive", *max)
        ));
        parameters.push_str(&format!(
            "<Parameter Id=\"P-{id}\" ParameterType=\"T-{id}\"{}/>",
            attr("Value", *value)
        ));
    }
    app(&types, &parameters, "", "")
}

#[test]
fn numeric_candidate_presence_and_exact_closed_bounds() {
    // The authored fixture: absent, empty and zero stay distinct; endpoints are inside.
    let r = inspect(VALUES);
    let values = rows(&r, Kind::ParameterValue);
    assert_eq!(
        values
            .iter()
            .map(|c| (
                id_of(&r, VALUES, c.source),
                c.value_check.as_ref().unwrap().comparison
            ))
            .collect::<Vec<_>>(),
        [
            ("P-absent".to_string(), Cmp::ValueAbsent),
            ("P-empty".to_string(), Cmp::ValueEmpty),
            ("P-zero".to_string(), Cmp::Below),
            ("P-low".to_string(), Cmp::Within),
            ("P-high".to_string(), Cmp::Within),
            ("P-above".to_string(), Cmp::Above),
            ("P-lexical".to_string(), Cmp::Within),
        ]
    );
    let lexical = by_id(&r, VALUES, Kind::ParameterValue, "P-lexical");
    let check = lexical.value_check.as_ref().unwrap();
    assert_eq!(check.model, Model::FiniteDecimalComparisonV1);
    assert_eq!(check.value.value.as_deref(), Some("0002"), "raw text kept");
    assert_eq!(check.value.normalized.as_deref(), Some("2e0"));
    assert_eq!(check.value.class, Decimal::FiniteDecimal);
    assert_eq!(check.minimum.value.as_deref(), Some("1"));
    assert_eq!(check.maximum.value.as_deref(), Some("64"));
    assert_eq!(check.type_shape, DistributionTypeShape::TypeNumber);
    assert!(original(&r, VALUES, check.type_source.unwrap()).starts_with(b"<TypeNumber "));
    assert_eq!(
        check
            .type_attributes
            .iter()
            .map(|a| (a.name.as_str(), a.value.as_deref()))
            .collect::<Vec<_>>(),
        [("Type", Some("unsignedInt")), ("SizeInBit", Some("8"))]
    );
    let absent = by_id(&r, VALUES, Kind::ParameterValue, "P-absent");
    let empty = by_id(&r, VALUES, Kind::ParameterValue, "P-empty");
    let zero = by_id(&r, VALUES, Kind::ParameterValue, "P-zero");
    let field = |c: &DistributionSourceCandidate| {
        let v = &c.value_check.as_ref().unwrap().value;
        (v.presence, v.value.clone(), v.class, v.normalized.clone())
    };
    assert_eq!(
        field(absent),
        (Presence::Absent, None, Decimal::Absent, None)
    );
    assert_eq!(
        field(empty),
        (
            Presence::PresentEmpty,
            Some(String::new()),
            Decimal::Empty,
            None
        )
    );
    assert_eq!(
        field(zero),
        (
            Presence::PresentValue,
            Some("0".into()),
            Decimal::FiniteDecimal,
            Some("0".into())
        )
    );
    let zero_check = zero.value_check.as_ref().unwrap();
    assert_eq!(
        (zero_check.below_minimum, zero_check.above_maximum),
        (Side::Outside, Side::NotOutside)
    );

    // Literal vectors, compared exactly: no binary float and no 64-bit narrowing.
    let cases = RANGES
        .iter()
        .map(|&(id, v, min, max, _)| (id, v, min, max))
        .collect::<Vec<_>>();
    let bytes = ranges_app(&cases);
    let r = inspect(&bytes);
    for &(id, raw, min, max, expect) in RANGES {
        let c = by_id(&r, &bytes, Kind::ParameterValue, &format!("P-{id}"));
        let check = c.value_check.as_ref().unwrap();
        assert_eq!(check.comparison, expect, "{id}");
        assert_eq!(check.value.value.as_deref(), raw, "{id}");
        assert_eq!(check.minimum.value.as_deref(), min, "{id}");
        assert_eq!(check.maximum.value.as_deref(), max, "{id}");
    }
    let side = |id: &str| {
        let c = by_id(&r, &bytes, Kind::ParameterValue, &format!("P-{id}"));
        let check = c.value_check.as_ref().unwrap();
        (check.below_minimum, check.above_maximum)
    };
    assert_eq!(
        side("unbounded"),
        (Side::UnboundedOnThatSide, Side::UnboundedOnThatSide)
    );
    assert_eq!(side("badbound"), (Side::Unavailable, Side::NotOutside));
    assert_eq!(side("reversed"), (Side::Unavailable, Side::Unavailable));
    assert_eq!(side("notfinite"), (Side::Unavailable, Side::Unavailable));
    let bad = by_id(&r, &bytes, Kind::ParameterValue, "P-badbound");
    assert_eq!(
        bad.value_check.as_ref().unwrap().minimum.class,
        Decimal::NotFiniteDecimal,
        "an invalid bound is not an absent bound"
    );
    let big = by_id(&r, &bytes, Kind::ParameterValue, "P-big");
    assert_eq!(
        big.value_check
            .as_ref()
            .unwrap()
            .value
            .normalized
            .as_deref(),
        Some("9007199254740993e0")
    );
}

#[test]
fn one_sided_bounds_exponents_whitespace_and_lexical_caps_are_named_separately() {
    let long = format!("1{}", "0".repeat(127));
    let longer = format!("1{}", "0".repeat(128));
    let cases: Vec<RangeCase<'_>> = vec![
        ("no-min-above", Some("65"), None, Some("64")),
        ("no-min-inside", Some("-5"), None, Some("64")),
        ("no-max-below", Some("0"), Some("1"), None),
        ("bad-min-above", Some("65"), Some("x"), Some("64")),
        ("empty-max", Some("2"), Some("1"), Some("")),
        (
            "exponent",
            Some("5.000000000000000E+002"),
            Some("0"),
            Some("499"),
        ),
        ("equal-scale", Some("1.20"), Some("1.2"), Some("1.2")),
        ("negative-zero", Some("-0.0"), Some("0"), Some("0")),
        ("plus", Some("+7"), Some("7"), Some("7")),
        ("spaces", Some(" 5 "), Some("1"), Some("64")),
        ("inner-space", Some("1 0"), Some("1"), Some("64")),
        ("dot", Some("."), Some("1"), Some("64")),
        ("half", Some(".5"), Some("0"), Some("1.")),
        ("cap", Some(long.as_str()), Some("0"), None),
        ("over-cap", Some(longer.as_str()), Some("0"), None),
        ("exp-cap", Some("1e1000000"), Some("0"), None),
        ("over-exp", Some("1e1000001"), Some("0"), None),
        ("tiny", Some("1e-1000000"), Some("0"), Some("1e-999999")),
        ("tiny-above", Some("1e-1000000"), Some("0"), Some("0")),
    ];
    let bytes = ranges_app(&cases);
    let r = inspect(&bytes);
    let check = |id: &str| {
        by_id(&r, &bytes, Kind::ParameterValue, &format!("P-{id}"))
            .value_check
            .clone()
            .unwrap()
    };
    let summary = |id: &str| {
        let c = check(id);
        (c.comparison, c.below_minimum, c.above_maximum)
    };
    assert_eq!(
        summary("no-min-above"),
        (Cmp::Above, Side::UnboundedOnThatSide, Side::Outside)
    );
    assert_eq!(
        summary("no-min-inside"),
        (Cmp::Within, Side::UnboundedOnThatSide, Side::NotOutside)
    );
    assert_eq!(
        summary("no-max-below"),
        (Cmp::Below, Side::Outside, Side::UnboundedOnThatSide)
    );
    assert_eq!(
        summary("bad-min-above"),
        (Cmp::Above, Side::Unavailable, Side::Outside),
        "the valid side is still compared"
    );
    assert_eq!(
        summary("empty-max"),
        (Cmp::BoundInvalid, Side::NotOutside, Side::Unavailable),
        "an empty bound is invalid, not unbounded"
    );
    assert_eq!(check("empty-max").maximum.class, Decimal::Empty);
    assert_eq!(summary("exponent").0, Cmp::Above);
    assert_eq!(check("exponent").value.normalized.as_deref(), Some("5e2"));
    assert_eq!(summary("equal-scale").0, Cmp::Within);
    assert_eq!(
        check("equal-scale").value.normalized.as_deref(),
        Some("12e-1")
    );
    assert_eq!(summary("negative-zero").0, Cmp::Within);
    assert_eq!(
        check("negative-zero").value.normalized.as_deref(),
        Some("0")
    );
    assert_eq!(summary("plus").0, Cmp::Within);
    assert_eq!(
        summary("spaces").0,
        Cmp::Within,
        "xs:decimal whitespace collapse"
    );
    assert_eq!(check("spaces").value.value.as_deref(), Some(" 5 "));
    assert_eq!(summary("inner-space").0, Cmp::NotFiniteDecimal);
    assert_eq!(summary("dot").0, Cmp::NotFiniteDecimal);
    assert_eq!(summary("half").0, Cmp::Within);
    assert_eq!(check("half").value.normalized.as_deref(), Some("5e-1"));
    assert_eq!(check("cap").value.class, Decimal::FiniteDecimal);
    assert_eq!(check("cap").value.normalized.as_deref(), Some("1e127"));
    assert_eq!(check("over-cap").value.class, Decimal::ExceedsLexicalLimit);
    assert_eq!(summary("over-cap").0, Cmp::LexicalResourceLimit);
    assert_eq!(
        check("over-cap").value.value.as_deref(),
        Some(longer.as_str())
    );
    assert_eq!(
        check("exp-cap").value.normalized.as_deref(),
        Some("1e1000000")
    );
    assert_eq!(summary("over-exp").0, Cmp::LexicalResourceLimit);
    assert_eq!(summary("tiny").0, Cmp::Within);
    assert_eq!(summary("tiny-above").0, Cmp::Above, "no rounding to zero");
}

#[test]
fn only_typenumber_or_unknown_type_values_become_rows() {
    let bytes = app(
        "<ParameterType Id=\"T-N\"><TypeNumber SizeInBit=\"8\" minInclusive=\"0\" maxInclusive=\"9\"/></ParameterType>\
         <ParameterType Id=\"T-F\"><TypeFloat Encoding=\"IEEE-754 Single\" minInclusive=\"0\" maxInclusive=\"1\"/></ParameterType>\
         <ParameterType Id=\"T-T\"><TypeTime SizeInBit=\"16\" Unit=\"Seconds\" minInclusive=\"0\" maxInclusive=\"1\"/></ParameterType>\
         <ParameterType Id=\"T-none\"/>\
         <ParameterType Id=\"T-two\"><TypeNumber SizeInBit=\"8\"/><TypeNumber SizeInBit=\"8\"/></ParameterType>\
         <ParameterType Id=\"T-dup\"><TypeNumber SizeInBit=\"8\"/></ParameterType>\
         <ParameterType Id=\"T-dup\"><TypeNumber SizeInBit=\"8\"/></ParameterType>",
        "<Parameter Id=\"P-N\" ParameterType=\"T-N\" Value=\"10\"/>\
         <Parameter Id=\"P-F\" ParameterType=\"T-F\" Value=\"5\"/>\
         <Parameter Id=\"P-T\" ParameterType=\"T-T\" Value=\"5\"/>\
         <Parameter Id=\"P-none\" ParameterType=\"T-none\" Value=\"5\"/>\
         <Parameter Id=\"P-two\" ParameterType=\"T-two\" Value=\"5\"/>\
         <Parameter Id=\"P-dup\" ParameterType=\"T-dup\" Value=\"5\"/>\
         <Parameter Id=\"P-dangling\" ParameterType=\"T-missing\" Value=\"5\"/>\
         <Parameter Id=\"P-untyped\" Value=\"5\"/>",
        "<ParameterRef Id=\"R-F\" RefId=\"P-F\" Value=\"7\"/>\
         <ParameterRef Id=\"R-missing\" RefId=\"P-missing\" Value=\"7\"/>",
        "",
    );
    let r = inspect(&bytes);
    let shape = |c: &DistributionSourceCandidate| {
        let check = c.value_check.as_ref().unwrap();
        (
            id_of(&r, &bytes, c.source),
            check.type_shape,
            check.comparison,
        )
    };
    assert_eq!(
        rows(&r, Kind::ParameterValue)
            .into_iter()
            .map(shape)
            .collect::<Vec<_>>(),
        [
            ("P-N".into(), DistributionTypeShape::TypeNumber, Cmp::Above),
            (
                "P-none".into(),
                DistributionTypeShape::NoTypeElement,
                Cmp::TypeUnavailable
            ),
            (
                "P-two".into(),
                DistributionTypeShape::MultipleTypeElements,
                Cmp::TypeUnavailable
            ),
            (
                "P-dup".into(),
                DistributionTypeShape::TypeReferenceUnavailable,
                Cmp::TypeUnavailable
            ),
            (
                "P-dangling".into(),
                DistributionTypeShape::TypeReferenceUnavailable,
                Cmp::TypeUnavailable
            ),
            (
                "P-untyped".into(),
                DistributionTypeShape::TypeReferenceUnavailable,
                Cmp::TypeUnavailable
            ),
        ],
        "TypeFloat and TypeTime are not counted as TypeNumber"
    );
    let dup = by_id(&r, &bytes, Kind::ParameterValue, "P-dup");
    let target = dup.parameter_type.as_ref().unwrap();
    assert_eq!(
        target.reference_state,
        Some(DistributionReferenceState::Ambiguous)
    );
    assert_eq!(target.candidates, identity(&r, "ParameterType", "T-dup"));
    let untyped = by_id(&r, &bytes, Kind::ParameterValue, "P-untyped");
    let target = untyped.parameter_type.as_ref().unwrap();
    assert_eq!(
        (target.presence, target.reference_state),
        (Presence::Absent, None)
    );
    let refs = rows(&r, Kind::ParameterRefValue);
    assert_eq!(
        refs.len(),
        1,
        "R-F points at TypeFloat; R-missing has no Parameter"
    );
    let missing = refs[0];
    assert_eq!(id_of(&r, &bytes, missing.source), "R-missing");
    let check = missing.value_check.as_ref().unwrap();
    assert_eq!(
        check.type_shape,
        DistributionTypeShape::ParameterUnavailable
    );
    assert_eq!(check.comparison, Cmp::TypeUnavailable);
    assert_eq!(check.value.value.as_deref(), Some("7"));
    assert_eq!(
        missing.parameter.as_ref().unwrap().reference_state,
        Some(DistributionReferenceState::Dangling)
    );
    assert!(missing.parameter_type.is_none());
}

#[test]
fn declaration_counts_do_not_count_inherited_ref_as_override() {
    let r = inspect(VALUES);
    let refs = rows(&r, Kind::ParameterRefValue);
    assert_eq!(
        refs.iter()
            .map(|c| (
                id_of(&r, VALUES, c.source),
                c.value_check.as_ref().unwrap().comparison
            ))
            .collect::<Vec<_>>(),
        [
            ("R-empty".to_string(), Cmp::ValueEmpty),
            ("R-override".to_string(), Cmp::Within)
        ],
        "R-inherit declares no Value and is not an override occurrence"
    );
    let fallbacks = r.applications[0]
        .parameter_fallbacks
        .iter()
        .map(|f| (f.parameter_ref_id.as_str(), f.value.as_deref(), f.origin))
        .collect::<Vec<_>>();
    assert_eq!(
        fallbacks,
        [
            (
                "R-inherit",
                Some("0"),
                DistributionFallbackOrigin::Parameter
            ),
            (
                "R-empty",
                Some(""),
                DistributionFallbackOrigin::ParameterRef
            ),
            (
                "R-override",
                Some("64"),
                DistributionFallbackOrigin::ParameterRef
            ),
        ],
        "fallbacks stay as they were; no source default is repaired"
    );
    let zero = by_id(&r, VALUES, Kind::ParameterValue, "P-zero");
    let inherit = &r.applications[0].parameter_fallbacks[0];
    assert_ne!(
        zero.source, inherit.source,
        "the Value owner is the Parameter, not the inheriting ref"
    );
    assert!(original(&r, VALUES, zero.source).starts_with(b"<Parameter Id=\"P-zero\""));
    assert!(original(&r, VALUES, inherit.source).starts_with(b"<ParameterRef Id=\"R-inherit\""));
    for c in &refs {
        let parameter = c.parameter.as_ref().unwrap();
        assert_eq!(parameter.attribute, "RefId");
        assert_eq!(parameter.raw_ref.as_deref(), Some("P-zero"));
        assert_eq!(
            parameter.reference_state,
            Some(DistributionReferenceState::Resolved)
        );
        assert_eq!(parameter.candidates, [zero.source]);
        assert_eq!(
            c.parameter_type.as_ref().unwrap().candidates,
            identity(&r, "ParameterType", "T")
        );
        assert!(original(&r, VALUES, c.source).starts_with(b"<ParameterRef "));
    }
    let empty = by_id(&r, VALUES, Kind::ParameterRefValue, "R-empty");
    assert_eq!(
        empty.value_check.as_ref().unwrap().value.value.as_deref(),
        Some("")
    );
    assert_eq!(
        by_id(&r, VALUES, Kind::ParameterRefValue, "R-override")
            .value_check
            .as_ref()
            .unwrap()
            .value
            .normalized
            .as_deref(),
        Some("64e0")
    );
}

/// One Union per case: (id, offset, bitOffset, width, extent); widths are `TypeNumber/@SizeInBit`.
type IntervalCase<'a> = (
    &'a str,
    Option<&'a str>,
    Option<&'a str>,
    Option<&'a str>,
    Option<&'a str>,
);
fn interval_app(cases: &[IntervalCase<'_>]) -> Vec<u8> {
    let mut types = String::new();
    let mut parameters = String::new();
    for (id, offset, bit, width, extent) in cases {
        types.push_str(&format!(
            "<ParameterType Id=\"W-{id}\"><TypeNumber{}/></ParameterType>",
            attr("SizeInBit", *width)
        ));
        parameters.push_str(&format!(
            "<Union{}><Memory CodeSegment=\"SEG\" Offset=\"100\" BitOffset=\"2\"/>\
             <Parameter Id=\"U-{id}\" ParameterType=\"W-{id}\"{}{}/></Union>",
            attr("SizeInBit", *extent),
            attr("Offset", *offset),
            attr("BitOffset", *bit)
        ));
    }
    app(&types, &parameters, "", "")
}
fn interval(start_bit: i64, end_bit: i64) -> Option<DistributionBitInterval> {
    Some(DistributionBitInterval { start_bit, end_bit })
}

#[test]
fn source_union_interval_units_and_unknown_width() {
    let r = inspect(UNION);
    let members = rows(&r, Kind::UnionMemberInterval);
    let union = identity(&r, "Parameter", "U-A");
    assert_eq!(union.len(), 1);
    let result = |c: &&DistributionSourceCandidate| {
        let check = c.interval_check.as_ref().unwrap();
        (id_of(&r, UNION, c.source), check.interval, check.comparison)
    };
    assert_eq!(
        members.iter().map(result).collect::<Vec<_>>(),
        [
            ("U-A".to_string(), interval(3, 8), Interval::WithinExtent),
            ("U-B".to_string(), interval(3, 8), Interval::WithinExtent),
            ("U-out".to_string(), interval(8, 9), Interval::OutsideExtent),
            ("U-unknown".to_string(), None, Interval::OffsetAbsent),
        ],
        "[3,8) is a valid non-byte-aligned interval; [8,9) leaves extent 8"
    );
    let a = members[0].interval_check.as_ref().unwrap();
    assert_eq!(a.model, Model::UnionMemberRelativeBitsV1);
    assert!(original(&r, UNION, a.union_source).starts_with(b"<Union SizeInBit=\"8\">"));
    assert_eq!(a.union_placement.len(), 1);
    assert!(original(&r, UNION, a.union_placement[0]).starts_with(b"<Memory CodeSegment=\"SEG\""));
    assert_eq!(
        (
            a.extent.parsed,
            a.offset.parsed,
            a.bit_offset.parsed,
            a.width.parsed
        ),
        (Some(8), Some(0), Some(3), Some(5)),
        "Memory Offset 100 / BitOffset 2 are never added: no absolute address"
    );
    assert!(original(&r, UNION, a.type_source.unwrap()).starts_with(b"<TypeNumber SizeInBit=\"5\""));
    let unknown = members[3].interval_check.as_ref().unwrap();
    assert_eq!(
        (
            unknown.width.presence,
            unknown.width.class,
            unknown.width.parsed
        ),
        (Presence::Absent, Integer::Absent, None),
        "a missing width is unknown, never zero"
    );
    assert_eq!(unknown.offset.class, Integer::Absent);

    // Literal `vectors.json:intervals`; omitted offsets are strict, not zero (strict admission).
    let cases: &[IntervalCase<'_>] = &[
        ("fits-nonbyte", Some("0"), Some("3"), Some("5"), Some("8")),
        ("outside", Some("1"), Some("0"), Some("1"), Some("8")),
        ("zero-width", Some("1"), Some("0"), Some("0"), Some("8")),
        ("unknown-width", Some("0"), Some("0"), None, Some("8")),
        ("omitted-offset", None, Some("1"), Some("2"), Some("8")),
        (
            "explicit-zero-offset",
            Some("0"),
            Some("1"),
            Some("2"),
            Some("8"),
        ),
        ("omitted-bit", Some("0"), None, Some("2"), Some("8")),
        (
            "negative-start",
            Some("-1"),
            Some("0"),
            Some("1"),
            Some("8"),
        ),
        ("invalid-width", Some("0"), Some("0"), Some("-1"), Some("8")),
        (
            "large-boundary",
            Some("30"),
            Some("0"),
            Some("16"),
            Some("240"),
        ),
        ("no-extent", Some("0"), Some("0"), Some("4"), None),
        (
            "negative-extent",
            Some("0"),
            Some("0"),
            Some("4"),
            Some("-8"),
        ),
        (
            "fraction-offset",
            Some("1.5"),
            Some("0"),
            Some("4"),
            Some("8"),
        ),
        ("empty-bit", Some("0"), Some(""), Some("4"), Some("8")),
        (
            "huge-offset",
            Some("99999999999999999999"),
            Some("0"),
            Some("4"),
            Some("8"),
        ),
        (
            "overflow",
            Some("9223372036854775807"),
            Some("0"),
            Some("4"),
            Some("8"),
        ),
        ("nonbyte-bit", Some("0"), Some("9"), Some("4"), Some("16")),
    ];
    let bytes = interval_app(cases);
    let r = inspect(&bytes);
    let check = |id: &str| {
        by_id(&r, &bytes, Kind::UnionMemberInterval, &format!("U-{id}"))
            .interval_check
            .clone()
            .unwrap()
    };
    let got = |id: &str| {
        let c = check(id);
        (c.interval, c.comparison, c.zero_width_declared)
    };
    assert_eq!(
        got("fits-nonbyte"),
        (interval(3, 8), Interval::WithinExtent, false)
    );
    assert_eq!(
        got("outside"),
        (interval(8, 9), Interval::OutsideExtent, false)
    );
    assert_eq!(
        got("zero-width"),
        (interval(8, 8), Interval::WithinExtent, true),
        "explicit zero width is a known empty interval"
    );
    assert_eq!(got("unknown-width"), (None, Interval::WidthUnknown, false));
    assert_eq!(
        got("omitted-offset"),
        (None, Interval::OffsetAbsent, false),
        "strict-explicit: no zero is substituted for an omitted Offset"
    );
    assert_eq!(
        got("explicit-zero-offset"),
        (interval(1, 3), Interval::WithinExtent, false),
        "an explicit lexical zero participates"
    );
    assert_eq!(got("omitted-bit"), (None, Interval::BitOffsetAbsent, false));
    assert_eq!(
        got("negative-start"),
        (interval(-8, -7), Interval::OutsideExtent, false)
    );
    assert_eq!(got("invalid-width"), (None, Interval::WidthInvalid, false));
    assert_eq!(
        got("large-boundary"),
        (interval(240, 256), Interval::OutsideExtent, false)
    );
    assert_eq!(
        got("no-extent"),
        (interval(0, 4), Interval::ExtentUnknown, false)
    );
    assert_eq!(
        got("negative-extent"),
        (interval(0, 4), Interval::ExtentInvalid, false)
    );
    assert_eq!(
        got("fraction-offset"),
        (None, Interval::OffsetInvalid, false)
    );
    assert_eq!(got("empty-bit"), (None, Interval::BitOffsetInvalid, false));
    assert_eq!(check("empty-bit").bit_offset.class, Integer::Empty);
    assert_eq!(got("huge-offset"), (None, Interval::ArithmeticLimit, false));
    assert_eq!(check("huge-offset").offset.class, Integer::ExceedsLimit);
    assert_eq!(
        check("huge-offset").offset.value.as_deref(),
        Some("99999999999999999999")
    );
    assert_eq!(got("overflow"), (None, Interval::ArithmeticLimit, false));
    assert_eq!(check("overflow").offset.class, Integer::Integer);
    assert_eq!(
        got("nonbyte-bit"),
        (interval(9, 13), Interval::WithinExtent, false)
    );
}

#[test]
fn union_declarations_intervals_and_subset_alternatives_separate() {
    let r = inspect(UNION);
    let summaries = rows(&r, Kind::UnionIntervalSummary);
    assert_eq!(summaries.len(), 1);
    let s = summaries[0].union_summary.as_ref().unwrap();
    assert_eq!(s.model, Model::UnionMemberRelativeBitsV1);
    assert_eq!(s.member_declarations, 4);
    assert_eq!(s.known_intervals, 3);
    assert_eq!(s.unavailable_intervals, 1);
    assert_eq!(s.unknown_width_members, 1);
    assert_eq!(s.outside_members, 1);
    assert_eq!(
        s.distinct_intervals,
        [
            DistributionDistinctInterval {
                start_bit: 3,
                end_bit: 8,
                members: 2
            },
            DistributionDistinctInterval {
                start_bit: 8,
                end_bit: 9,
                members: 1
            }
        ]
    );
    assert_eq!(s.overlapping_distinct_pairs, 0);
    assert!(s.overlap_samples.is_empty() && !s.overlap_samples_truncated);
    assert_eq!(s.extent.parsed, Some(8));
    assert!(summaries[0].interval_check.is_none() && summaries[0].value_check.is_none());
    assert!(original(&r, UNION, summaries[0].source).starts_with(b"<Union "));
    assert_eq!(
        summaries[0].activity.reason,
        Some(DistributionActivityReason::SummaryRow)
    );

    // The shared evaluator ran on source fallbacks (R-A = 1): branch 1 is observed, nothing more.
    let app = &r.applications[0];
    assert_eq!(app.dynamic_state, DistributionPassState::PartiallyEvaluated);
    assert!(app.active_parameter_ref_ids.contains(&"R-U-A".to_string()));
    assert!(!app.active_parameter_ref_ids.contains(&"R-U-B".to_string()));
    let a = by_id(&r, UNION, Kind::UnionMemberInterval, "U-A");
    assert_eq!(a.activity.state, Activity::ObservedInEvaluatedSubset);
    assert_eq!(a.activity.observations.len(), 1);
    let observation = &a.activity.observations[0];
    assert_eq!(
        vec![observation.parameter_ref_source],
        identity(&r, "ParameterRef", "R-U-A")
    );
    assert!(observation.module_scope.is_empty());
    assert_eq!(
        observation.input,
        DistributionActivityInput::SourceFallbacks
    );
    let b = by_id(&r, UNION, Kind::UnionMemberInterval, "U-B");
    assert_eq!(
        (b.activity.state, b.activity.reason),
        (
            Activity::Unavailable,
            Some(DistributionActivityReason::NotObservedInEvaluatedSubset)
        ),
        "branch 2 was not exercised: absent from the subset is not inactive"
    );
    let out = by_id(&r, UNION, Kind::UnionMemberInterval, "U-out");
    assert_eq!(
        out.activity.reason,
        Some(DistributionActivityReason::NoUniqueParameterRef)
    );
    let json = serde_json::to_string(&r).unwrap();
    assert!(!json.contains("knownInactive") && !json.contains("simultaneous"));
    // The value rows of the same members are separate occurrences, checked separately.
    let value = by_id(&r, UNION, Kind::ParameterValue, "U-A");
    assert_eq!(
        value.value_check.as_ref().unwrap().comparison,
        Cmp::Unbounded
    );
    assert_eq!(
        app.parameter_fallbacks
            .iter()
            .map(|f| (f.parameter_ref_id.as_str(), f.value.as_deref()))
            .collect::<Vec<_>>(),
        [
            ("R-A", Some("1")),
            ("R-B", Some("2")),
            ("R-U-A", Some("1")),
            ("R-U-B", Some("2"))
        ],
        "no Union alternative is chosen and no value is written"
    );
}

#[test]
fn overlap_summary_counts_exactly_and_truncates_samples() {
    let mut parameters = String::from("<Union SizeInBit=\"32\">");
    for i in 0..20 {
        parameters.push_str(&format!(
            "<Parameter Id=\"U-{i}\" ParameterType=\"W\" Offset=\"0\" BitOffset=\"{i}\"/>"
        ));
    }
    parameters
        .push_str("<Parameter Id=\"U-empty\" ParameterType=\"Z\" Offset=\"0\" BitOffset=\"4\"/>");
    parameters.push_str("</Union>");
    let bytes = app(
        "<ParameterType Id=\"W\"><TypeNumber SizeInBit=\"8\"/></ParameterType>\
         <ParameterType Id=\"Z\"><TypeNumber SizeInBit=\"0\"/></ParameterType>",
        &parameters,
        "",
        "",
    );
    let r = inspect(&bytes);
    let summaries = rows(&r, Kind::UnionIntervalSummary);
    assert_eq!(summaries.len(), 1, "one summary row per Union");
    let s = summaries[0].union_summary.clone().unwrap();
    assert_eq!(s.member_declarations, 21);
    assert_eq!(s.distinct_intervals.len(), 21);
    assert_eq!(s.outside_members, 0);
    // Independent brute force over [i, i+8): pairs with |i - j| < 8; the empty [4,4) overlaps none.
    let expected = (0..20i64)
        .flat_map(|i| (i + 1..20).map(move |j| (i, j)))
        .filter(|(i, j)| j - i < 8)
        .count() as u64;
    assert_eq!(s.overlapping_distinct_pairs, expected);
    assert_eq!(s.overlap_samples.len(), 16);
    assert!(s.overlap_samples_truncated);
    for pair in &s.overlap_samples {
        let (a, b) = (
            s.distinct_intervals[pair.first as usize],
            s.distinct_intervals[pair.second as usize],
        );
        assert!(pair.first < pair.second);
        assert!(
            a.start_bit < b.end_bit && b.start_bit < a.end_bit,
            "{pair:?}"
        );
    }
}

#[test]
fn module_scoped_candidates_keep_one_row_and_separate_instance_observations() {
    let bytes = String::from(
        "<KNX xmlns=\"http://knx.org/xml/project/20\"><ManufacturerData><Manufacturer RefId=\"M\">\
         <ApplicationPrograms><ApplicationProgram Id=\"A\"><Static><ParameterTypes>\
         <ParameterType Id=\"T\"><TypeNumber SizeInBit=\"8\" minInclusive=\"1\" maxInclusive=\"9\"/></ParameterType>\
         </ParameterTypes><ModuleDefs><ModuleDef Id=\"MD\"><Static><Parameters>\
         <Parameter Id=\"P-M\" ParameterType=\"T\" Value=\"0\"/></Parameters><ParameterRefs>\
         <ParameterRef Id=\"R-M\" RefId=\"P-M\"/></ParameterRefs></Static><Dynamic>\
         <ParameterRefRef RefId=\"R-M\"/></Dynamic></ModuleDef></ModuleDefs></Static><Dynamic>\
         <Module Id=\"one\" RefId=\"MD\"/><Module Id=\"two\" RefId=\"MD\"/></Dynamic></ApplicationProgram>\
         </ApplicationPrograms></Manufacturer></ManufacturerData></KNX>"
    )
    .into_bytes();
    let r = inspect(&bytes);
    let values = rows(&r, Kind::ParameterValue);
    assert_eq!(
        values.len(),
        1,
        "a definition is one row, not one per instance"
    );
    let c = values[0];
    assert_eq!(c.value_check.as_ref().unwrap().comparison, Cmp::Below);
    assert!(original(&r, &bytes, c.module_definition_source.unwrap())
        .starts_with(b"<ModuleDef Id=\"MD\""));
    let target = c.parameter_type.as_ref().unwrap();
    assert_eq!(
        target.basis,
        Some(DistributionResolutionBasis::EnclosingApplicationScope)
    );
    assert_eq!(c.activity.state, Activity::ObservedInEvaluatedSubset);
    assert_eq!(
        c.activity
            .observations
            .iter()
            .map(|o| o.module_scope[0].module_id.clone())
            .collect::<Vec<_>>(),
        [Some("one".to_string()), Some("two".to_string())]
    );
    assert!(c
        .activity
        .observations
        .iter()
        .all(|o| o.module_scope.last().unwrap().module_def_id == "MD"));
}

#[test]
fn candidate_labels_and_dependency_context_without_calculated_values() {
    let plain = app(
        "<ParameterType Id=\"T\"><TypeNumber SizeInBit=\"8\" minInclusive=\"1\" maxInclusive=\"9\"/></ParameterType>",
        "<Parameter Id=\"P-A\" ParameterType=\"T\" Value=\"0\"/><Parameter Id=\"P-B\" ParameterType=\"T\" Value=\"1\"/>",
        "<ParameterRef Id=\"R-A\" RefId=\"P-A\" Value=\"10\"/><ParameterRef Id=\"R-B\" RefId=\"P-B\"/>",
        "<Assign TargetParamRefRef=\"R-A\" SourceParamRefRef=\"R-B\"/>",
    );
    let r = inspect(&plain);
    assert!(!r.source_candidates.is_empty());
    for c in &r.source_candidates {
        assert_eq!(c.severity, DistributionCandidateSeverity::SourceCandidate);
        assert_eq!(
            c.initialization,
            DistributionCandidateInitialization::NotPerformed
        );
        assert_eq!(c.application_source, r.applications[0].source);
        assert_eq!(
            c.dependency_context.impact_scope,
            DistributionImpactScope::DeclaredEndpointsOnly
        );
    }
    let assign = r.dependency_evidence[0].source;
    let override_row = by_id(&r, &plain, Kind::ParameterRefValue, "R-A");
    assert_eq!(
        override_row
            .dependency_context
            .declarations
            .iter()
            .map(|d| (d.kind, d.role, d.source))
            .collect::<Vec<_>>(),
        [(
            DistributionDependencyKind::Assign,
            DistributionEndpointRole::AssignTarget,
            assign
        )]
    );
    let a = by_id(&r, &plain, Kind::ParameterValue, "P-A");
    let b = by_id(&r, &plain, Kind::ParameterValue, "P-B");
    assert_eq!(
        a.dependency_context.declarations[0].parameter_ref_source,
        identity(&r, "ParameterRef", "R-A")[0]
    );
    assert_eq!(
        b.dependency_context
            .declarations
            .iter()
            .map(|d| d.role)
            .collect::<Vec<_>>(),
        [DistributionEndpointRole::AssignSource]
    );
    assert_eq!(
        override_row.value_check.as_ref().unwrap().comparison,
        Cmp::Above,
        "the declared override is compared; the Assign is not applied"
    );
    assert_eq!(
        a.activity.state,
        Activity::Unavailable,
        "an Assign-only reference is not observed by the subset"
    );

    let scripted = app(
        "<ParameterType Id=\"T\"><TypeNumber SizeInBit=\"8\"/></ParameterType>",
        "<Parameter Id=\"P-A\" ParameterType=\"T\" Value=\"0\"/>",
        "",
        "<Button Id=\"B\" EventHandler=\"authored\"/>",
    );
    let r = inspect(&scripted);
    assert_eq!(
        r.source_candidates[0].dependency_context.impact_scope,
        DistributionImpactScope::ApplicationImpactUnknown,
        "a declared handler may touch any parameter"
    );

    let ambiguous = app(
        "<ParameterType Id=\"T\"><TypeNumber SizeInBit=\"8\"/></ParameterType>",
        "<Parameter Id=\"P-A\" ParameterType=\"T\" Value=\"0\"/><Parameter Id=\"P-A\" ParameterType=\"T\"/>",
        "",
        "",
    );
    let r = inspect(&ambiguous);
    assert_eq!(
        r.applications[0].dynamic_state,
        DistributionPassState::NotAttempted
    );
    for c in &r.source_candidates {
        assert_eq!(
            (c.activity.state, c.activity.reason),
            (
                Activity::NotEvaluated,
                Some(DistributionActivityReason::DynamicNotAttempted)
            )
        );
    }
    assert_eq!(
        rows(&r, Kind::ParameterValue).len(),
        2,
        "duplicates stay separate"
    );
}

#[test]
fn additive_candidate_rows_are_bounded_deterministic_and_read_only() {
    let source = UNION.to_vec();
    let r = inspect(UNION);
    assert_eq!(source, UNION);
    let nested: usize = r
        .source_candidates
        .iter()
        .map(|c| {
            1 + c.parameter.as_ref().map_or(0, |t| t.candidates.len())
                + c.parameter_type.as_ref().map_or(0, |t| t.candidates.len())
                + c.value_check
                    .as_ref()
                    .map_or(0, |v| v.type_attributes.len())
                + c.interval_check
                    .as_ref()
                    .map_or(0, |i| i.union_placement.len())
                + c.union_summary
                    .as_ref()
                    .map_or(0, |s| s.distinct_intervals.len() + s.overlap_samples.len())
                + c.activity.observations.len()
                + c.activity
                    .observations
                    .iter()
                    .map(|o| o.module_scope.len())
                    .sum::<usize>()
                + c.dependency_context.declarations.len()
        })
        .sum();
    let count = r.output_record_count();
    let mut without = r.clone();
    without.source_candidates.clear();
    assert!(nested > r.source_candidates.len());
    assert_eq!(
        count - without.output_record_count(),
        nested,
        "every new row is charged"
    );
    let limited = |limits: L| inspect_distribution_with_limits("authored.xml", UNION, limits);
    for (cap, positive) in [(count - 1, false), (count, true)] {
        let result = limited(L {
            max_output_records: cap,
            ..all_rows()
        });
        assert_eq!(result.is_ok(), positive, "records cap={cap} count={count}");
        if !positive {
            assert!(matches!(result, Err(E::ResourceLimit(_))));
        }
    }
    let size = r.serialized_size().unwrap();
    for (cap, positive) in [(size - 1, false), (size, true)] {
        let result = limited(L {
            max_report_bytes: cap,
            ..all_rows()
        });
        assert_eq!(result.is_ok(), positive, "bytes cap={cap} size={size}");
    }
    let candidate_references: usize = r
        .source_candidates
        .iter()
        .flat_map(|c| c.parameter.iter().chain(c.parameter_type.iter()))
        .map(|t| 1 + t.candidates.len())
        .sum();
    assert!(candidate_references > 0);
    let references = r.references.len()
        + r.references
            .iter()
            .map(|x| x.candidates.len())
            .sum::<usize>()
        + r.dependency_evidence
            .iter()
            .flat_map(|e| &e.endpoints)
            .map(|p| 1 + p.candidates.len())
            .sum::<usize>()
        + candidate_references;
    for (cap, positive) in [(references - 1, false), (references, true)] {
        let result = limited(L {
            max_references: cap,
            ..all_rows()
        });
        assert_eq!(
            result.is_ok(),
            positive,
            "references cap={cap} total={references}"
        );
    }
    assert_eq!(
        serde_json::to_vec(&r).unwrap(),
        serde_json::to_vec(&inspect(UNION)).unwrap(),
        "repeat inspection is byte-equivalent"
    );
    let order = r
        .source_candidates
        .iter()
        .map(|c| r.resolve_coordinate(c.source).unwrap().start_byte)
        .collect::<Vec<_>>();
    let mut sorted = order.clone();
    sorted.sort();
    assert_eq!(order, sorted, "rows follow element order");
}

#[test]
fn zip_members_charge_candidate_rows_against_the_shared_budget() {
    let zip = raw_zip(&[
        RawMember::stored(b"a/values.xml", VALUES),
        RawMember::stored(b"b/union.xml", UNION),
    ]);
    let r = inspect(&zip);
    let members: BTreeSet<_> = r
        .source_candidates
        .iter()
        .map(|c| c.source.member_ordinal)
        .collect();
    assert_eq!(members, BTreeSet::from([Some(0), Some(1)]));
    assert_eq!(
        r.source_candidates.len(),
        inspect(VALUES).source_candidates.len() + inspect(UNION).source_candidates.len()
    );
    let count = r.output_record_count();
    let mut without = r.clone();
    without.source_candidates.clear();
    assert!(count > without.output_record_count());
    for (cap, positive) in [(count - 1, false), (count, true)] {
        let result = inspect_distribution_with_limits(
            "authored.zip",
            &zip,
            L {
                max_output_records: cap,
                ..all_rows()
            },
        );
        assert_eq!(result.is_ok(), positive, "records cap={cap} count={count}");
    }
}

#[test]
fn format_four_pins_candidates_and_refuses_a_format_three_report() {
    let r = inspect(VALUES);
    assert_eq!(r.format_version, 4);
    let mut json = serde_json::to_value(&r).unwrap();
    assert_eq!(json["formatVersion"], 4);
    assert_eq!(json["sourceCandidates"][0]["severity"], "sourceCandidate");
    assert_eq!(
        json["sourceCandidates"][0]["initialization"],
        "notPerformed"
    );
    assert_eq!(
        json["sourceCandidates"][0]["valueCheck"]["model"],
        "finite-decimal-comparison-v1"
    );
    let object = json.as_object_mut().unwrap();
    assert!(object.remove("sourceCandidates").is_some());
    object.insert("formatVersion".into(), 3.into());
    assert!(
        serde_json::from_value::<DistributionInspection>(json).is_err(),
        "a report without the pass must not read as a source without candidates"
    );
    let union = serde_json::to_value(inspect(UNION)).unwrap();
    let member = union["sourceCandidates"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["kind"] == "unionMemberInterval")
        .unwrap();
    assert_eq!(
        member["intervalCheck"]["model"],
        "union-member-relative-bits-v1"
    );
    assert_eq!(member["intervalCheck"]["interval"]["startBit"], 3);
}

#[test]
fn default_lists_findings_and_counts_every_checked_occurrence() {
    let full = inspect(VALUES);
    let default = inspect_distribution("authored.xml", VALUES).unwrap();
    let listed = |r: &DistributionInspection| {
        r.source_candidates
            .iter()
            .map(|c| (c.kind, c.value_check.as_ref().map(|v| v.comparison)))
            .collect::<Vec<_>>()
    };
    let findings = listed(&full)
        .into_iter()
        .filter(|(_, c)| *c != Some(Cmp::Within))
        .collect::<Vec<_>>();
    assert_eq!(
        listed(&default),
        findings,
        "rows outside `within` keep their order"
    );
    let full_rows = full
        .source_candidates
        .iter()
        .filter(|c| c.value_check.as_ref().unwrap().comparison != Cmp::Within)
        .cloned()
        .collect::<Vec<_>>();
    assert_eq!(
        default.source_candidates, full_rows,
        "finding rows are byte-identical"
    );
    assert_eq!(
        default
            .source_candidate_totals
            .iter()
            .map(|t| (t.kind, t.value_comparison, t.occurrences))
            .collect::<Vec<_>>(),
        full.source_candidate_totals
            .iter()
            .map(|t| (t.kind, t.value_comparison, t.occurrences))
            .collect::<Vec<_>>(),
        "totals do not depend on row detail"
    );
    let within = |r: &DistributionInspection, kind: Kind| {
        r.source_candidate_totals
            .iter()
            .find(|t| t.kind == kind && t.value_comparison == Some(Cmp::Within))
            .map(|t| (t.occurrences, t.rows))
    };
    assert_eq!(within(&default, Kind::ParameterValue), Some((3, 0)));
    assert_eq!(within(&full, Kind::ParameterValue), Some((3, 3)));
    assert_eq!(within(&default, Kind::ParameterRefValue), Some((1, 0)));
    for r in [&default, &full] {
        let occurrences: u64 = r
            .source_candidate_totals
            .iter()
            .map(|t| t.occurrences)
            .sum();
        assert_eq!(
            occurrences,
            full.source_candidates.len() as u64,
            "nothing is dropped"
        );
        let rows: u64 = r.source_candidate_totals.iter().map(|t| t.rows).sum();
        assert_eq!(
            rows,
            r.source_candidates.len() as u64,
            "rows count exactly the listed rows"
        );
        for t in &r.source_candidate_totals {
            assert_eq!(t.application_source, r.applications[0].source);
        }
    }

    let union_full = inspect(UNION);
    let union_default = inspect_distribution("authored.xml", UNION).unwrap();
    let members = |r: &DistributionInspection| {
        r.source_candidates
            .iter()
            .filter_map(|c| c.interval_check.as_ref().map(|i| i.comparison))
            .collect::<Vec<_>>()
    };
    assert!(members(&union_full).contains(&Interval::WithinExtent));
    assert!(!members(&union_default).contains(&Interval::WithinExtent));
    assert_eq!(
        members(&union_default),
        members(&union_full)
            .into_iter()
            .filter(|c| *c != Interval::WithinExtent)
            .collect::<Vec<_>>()
    );
    assert_eq!(
        rows(&union_default, Kind::UnionIntervalSummary),
        rows(&union_full, Kind::UnionIntervalSummary),
        "Union summaries are always listed"
    );
    let json = serde_json::to_value(&default).unwrap();
    assert!(json["sourceCandidateTotals"].is_array());
}

/// Totals as comparable tuples, without the application coordinate.
type TotalRow = (Kind, Option<Cmp>, Option<Interval>, u64, u64);
fn totals(r: &DistributionInspection) -> Vec<TotalRow> {
    r.source_candidate_totals
        .iter()
        .map(|t| {
            (
                t.kind,
                t.value_comparison,
                t.interval_comparison,
                t.occurrences,
                t.rows,
            )
        })
        .collect()
}
/// Totals key including the application, so ZIP members with several applications stay apart.
type TotalKey = ((Option<u32>, u32), Kind, Option<Cmp>, Option<Interval>, u64);
/// Occurrences per (application, kind, outcome) taken from an All-mode report's own rows.
fn occurrences_from_rows(r: &DistributionInspection) -> BTreeSet<TotalKey> {
    let mut counted = std::collections::BTreeMap::<_, u64>::new();
    for c in &r.source_candidates {
        let key = (
            (
                c.application_source.member_ordinal,
                c.application_source.location,
            ),
            c.kind,
            c.value_check.as_ref().map(|v| v.comparison),
            c.interval_check.as_ref().map(|i| i.comparison),
        );
        *counted.entry(key).or_default() += 1;
    }
    counted
        .into_iter()
        .map(|((app, k, v, i), n)| (app, k, v, i, n))
        .collect()
}
fn check_totals_against_rows(r_all: &DistributionInspection, r_default: &DistributionInspection) {
    let from_totals = |r: &DistributionInspection| {
        r.source_candidate_totals
            .iter()
            .map(|t| {
                (
                    (
                        t.application_source.member_ordinal,
                        t.application_source.location,
                    ),
                    t.kind,
                    t.value_comparison,
                    t.interval_comparison,
                    t.occurrences,
                )
            })
            .collect::<BTreeSet<_>>()
    };
    assert_eq!(
        from_totals(r_all),
        occurrences_from_rows(r_all),
        "every kind is counted"
    );
    assert_eq!(
        from_totals(r_default),
        from_totals(r_all),
        "totals ignore row detail"
    );
    for r in [r_all, r_default] {
        let rows: u64 = r.source_candidate_totals.iter().map(|t| t.rows).sum();
        assert_eq!(rows, r.source_candidates.len() as u64);
    }
}

#[test]
fn union_and_zip_totals_count_every_kind_exactly() {
    let all = inspect(UNION);
    let default = inspect_distribution("authored.xml", UNION).unwrap();
    check_totals_against_rows(&all, &default);
    let member = |r: &DistributionInspection, outcome: Interval| {
        totals(r)
            .into_iter()
            .find(|t| t.0 == Kind::UnionMemberInterval && t.2 == Some(outcome))
            .map(|t| (t.3, t.4))
    };
    assert_eq!(member(&default, Interval::WithinExtent), Some((2, 0)));
    assert_eq!(member(&default, Interval::OutsideExtent), Some((1, 1)));
    assert_eq!(member(&default, Interval::OffsetAbsent), Some((1, 1)));
    assert_eq!(member(&all, Interval::WithinExtent), Some((2, 2)));
    assert_eq!(
        totals(&default)
            .into_iter()
            .find(|t| t.0 == Kind::UnionIntervalSummary)
            .map(|t| (t.3, t.4)),
        Some((1, 1))
    );

    let cases: Vec<IntervalCase<'_>> = vec![
        ("in", Some("0"), Some("3"), Some("5"), Some("8")),
        ("out", Some("1"), Some("0"), Some("1"), Some("8")),
        ("no-offset", None, Some("1"), Some("2"), Some("8")),
        ("no-width", Some("0"), Some("0"), None, Some("8")),
    ];
    let bytes = interval_app(&cases);
    check_totals_against_rows(
        &inspect(&bytes),
        &inspect_distribution("authored.xml", &bytes).unwrap(),
    );

    let zip = raw_zip(&[
        RawMember::stored(b"a/values.xml", VALUES),
        RawMember::stored(b"b/union.xml", UNION),
    ]);
    let zip_default = inspect_distribution("authored.zip", &zip).unwrap();
    check_totals_against_rows(&inspect(&zip), &zip_default);
    let expected = [
        totals(&inspect_distribution("authored.xml", VALUES).unwrap()),
        totals(&default),
    ]
    .concat();
    assert_eq!(
        totals(&zip_default),
        expected,
        "ZIP members keep their own totals, in member order"
    );
    let members: Vec<_> = zip_default
        .source_candidate_totals
        .iter()
        .map(|t| t.application_source.member_ordinal)
        .collect();
    assert!(members.windows(2).all(|w| w[0] <= w[1]));
    assert_eq!(
        members.iter().collect::<BTreeSet<_>>(),
        BTreeSet::from([&Some(0), &Some(1)])
    );
}

#[test]
fn totals_are_charged_as_output_rows() {
    for bytes in [VALUES, UNION] {
        let r = inspect_distribution("authored.xml", bytes).unwrap();
        assert!(!r.source_candidate_totals.is_empty());
        let mut without = r.clone();
        without.source_candidate_totals.clear();
        assert_eq!(
            r.output_record_count(),
            without.output_record_count() + r.source_candidate_totals.len(),
            "each totals entry is one output row"
        );
    }
}

#[test]
fn each_union_keeps_its_own_placement() {
    let types = "<ParameterType Id=\"W\"><TypeNumber SizeInBit=\"4\"/></ParameterType>";
    let parameters = "<Union SizeInBit=\"8\"><Memory CodeSegment=\"SEG-1\" Offset=\"10\" BitOffset=\"0\"/>\
         <Parameter Id=\"U-1\" ParameterType=\"W\" Offset=\"0\" BitOffset=\"0\"/></Union>\
         <Union SizeInBit=\"8\"><Property ObjectIndex=\"2\" PropertyId=\"7\" Offset=\"20\" BitOffset=\"1\"/>\
         <Parameter Id=\"U-2\" ParameterType=\"W\" Offset=\"0\" BitOffset=\"4\"/></Union>";
    let bytes = app(types, parameters, "", "");
    let r = inspect(&bytes);
    let placement = |id: &str| {
        by_id(&r, &bytes, Kind::UnionMemberInterval, id)
            .interval_check
            .as_ref()
            .unwrap()
            .union_placement
            .iter()
            .map(|s| String::from_utf8(original(&r, &bytes, *s).to_vec()).unwrap())
            .collect::<Vec<_>>()
    };
    let first = placement("U-1");
    let second = placement("U-2");
    assert_eq!(first.len(), 1);
    assert_eq!(second.len(), 1);
    assert!(
        first[0].starts_with("<Memory CodeSegment=\"SEG-1\""),
        "{first:?}"
    );
    assert!(
        second[0].starts_with("<Property ObjectIndex=\"2\""),
        "{second:?}"
    );
}

#[test]
fn explicit_zero_width_inside_the_extent_is_still_listed() {
    let cases: Vec<IntervalCase<'_>> = vec![
        ("zero", Some("1"), Some("0"), Some("0"), Some("8")),
        ("plain", Some("0"), Some("0"), Some("8"), Some("8")),
    ];
    let bytes = interval_app(&cases);
    let r = inspect_distribution("authored.xml", &bytes).unwrap();
    let members = rows(&r, Kind::UnionMemberInterval);
    assert_eq!(members.len(), 1, "only the zero-width member is a finding");
    let check = members[0].interval_check.as_ref().unwrap();
    assert!(check.zero_width_declared);
    assert_eq!(check.comparison, Interval::WithinExtent);
    assert_eq!(check.interval, interval(8, 8));
    assert_eq!(id_of(&r, &bytes, members[0].source), "U-zero");
    assert_eq!(
        totals(&r)
            .into_iter()
            .find(|t| t.0 == Kind::UnionMemberInterval && t.2 == Some(Interval::WithinExtent))
            .map(|t| (t.3, t.4)),
        Some((2, 1)),
        "the listed zero-width member is counted as a row of its outcome"
    );
}
