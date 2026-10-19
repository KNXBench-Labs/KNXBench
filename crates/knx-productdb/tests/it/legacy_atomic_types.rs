//! Legacy atomic types 3 and 5 map the way ETS4 converted them (KL-128).
//!
//! KL-128 (Goal A5): legacy atomic types 3 (`string`) and 5 (`long enum`)
//! map the way ETS4 converted the Siemens Nov 2016 database
//! (docs/research/legacy-vd-mapping.md, *Atomic types 3 and 5: the ETS4
//! oracle*). Synthetic EX-IM rows; the oracle itself is the ignored corpus
//! test in `crates/knx-app/tests/legacy_corpus.rs`.

use knx_productdb::legacy::{map_legacy_database, parse_exim, MappedProgram, MappingDiagnostic};

fn exim(tables: &str) -> Vec<u8> {
    format!(
        "EX-IM\r\nN x\r\nK ETS3\r\nK \r\nD 2026-10-13 09:00:00\r\nV 6.3\r\nH virtual_device\r\n{tables}XXX\r\n"
    )
    .into_bytes()
}

const SEP: &str = "-------------------------------------\r\n";

fn table(id: u32, name: &str, columns: &[&str], rows: &[&[&str]]) -> String {
    let mut out = format!("{SEP}T {id} {name}\r\n");
    for (i, column) in columns.iter().enumerate() {
        out.push_str(&format!("C{} T{id} 3 255 Y {column}\r\n", i + 1));
    }
    for (i, row) in rows.iter().enumerate() {
        out.push_str(&format!("R {} T {id} {name}\r\n", i + 1));
        for value in *row {
            out.push_str(value);
            out.push_str("\r\n");
        }
    }
    out
}

/// Type 20 is a 64-bit string, 21 a 16-bit long enum with consistent binary
/// values, 22 a long enum whose binary value does not fill its size, 23 an
/// atomic type no source describes.
fn mapping() -> (MappedProgram, Vec<MappingDiagnostic>) {
    let mut tables = table(3, "manufacturer", &["MANUFACTURER_ID"], &[&["4242"]]);
    tables += &table(
        11,
        "application_program",
        &["PROGRAM_ID", "MANUFACTURER_ID"],
        &[&["300", "4242"]],
    );
    tables += &table(
        15,
        "parameter_type",
        &[
            "PARAMETER_TYPE_ID",
            "ATOMIC_TYPE_NUMBER",
            "PROGRAM_ID",
            "PARAMETER_TYPE_NAME",
            "PARAMETER_TYPE_SIZE",
            "PARAMETER_MINIMUM_VALUE",
            "PARAMETER_MAXIMUM_VALUE",
        ],
        &[
            &["20", "3", "300", "Name", "64", "", ""],
            &["21", "5", "300", "Mode", "16", "", ""],
            &["22", "5", "300", "Broken", "16", "", ""],
            &["23", "9", "300", "Future", "8", "", ""],
        ],
    );
    tables += &table(
        16,
        "parameter_list_of_values",
        &[
            "PARAMETER_TYPE_ID",
            "REAL_VALUE",
            "DISPLAYED_VALUE",
            "DISPLAY_ORDER",
            "PARAMETER_VALUE_ID",
            "BINARY_VALUE",
            "BINARY_VALUE_LENGTH",
        ],
        &[
            &["21", "513", "Two-oh-one", "2", "1", "0201", "2"],
            &["21", "1", "One", "1", "2", "0001", "2"],
            &["22", "7", "Seven", "1", "3", "07", "1"],
        ],
    );
    tables += &table(
        17,
        "parameter",
        &[
            "PARAMETER_ID",
            "PROGRAM_ID",
            "PARAMETER_TYPE_ID",
            "PARAMETER_NUMBER",
            "PARAMETER_HIGH_ACCESS",
            "PAR_PARAMETER_ID",
            "PARAMETER_DEFAULT_LONG",
            "PARAMETER_DEFAULT_STRING",
        ],
        &[
            &["1", "300", "20", "1", "2", "", "", " Hello  "],
            &["2", "300", "21", "2", "1", "", "513", ""],
            &["3", "300", "22", "3", "0", "", "7", ""],
            &["4", "300", "23", "4", "2", "", "1", ""],
        ],
    );
    let document = parse_exim(&exim(&tables)).unwrap();
    let mapping = map_legacy_database(&document, &"ab".repeat(32)).unwrap();
    assert_eq!(mapping.programs.len(), 1);
    (
        mapping.programs.into_iter().next().unwrap(),
        mapping.diagnostics,
    )
}

fn type_shape(program: &MappedProgram, n: &str) -> Option<(String, Option<i64>, Option<String>)> {
    program
        .parameter_types
        .iter()
        .find(|t| t.id == format!("{}_PT-{n}", program.id))
        .map(|t| {
            assert_eq!((&t.min_inclusive, &t.max_inclusive), (&None, &None));
            assert_eq!(t.number_type, None);
            (t.kind.clone(), t.size_in_bit, t.base.clone())
        })
}

fn parameter<'a>(
    program: &'a MappedProgram,
    type_n: &str,
) -> &'a knx_productdb::legacy::MappedParameter {
    program
        .parameters
        .iter()
        .find(|p| p.parameter_type_id == format!("{}_PT-{type_n}", program.id))
        .unwrap_or_else(|| panic!("a parameter of type {type_n}"))
}

#[test]
fn a_string_type_becomes_text_of_its_size_with_the_verbatim_string_default() {
    let (program, _) = mapping();
    assert_eq!(
        type_shape(&program, "20"),
        Some(("Text".into(), Some(64), None))
    );
    let text = parameter(&program, "20");
    assert_eq!(text.value.as_deref(), Some(" Hello  "));
    assert_eq!(text.access.as_deref(), Some("ReadWrite"));
}

#[test]
fn a_long_enum_becomes_a_binary_value_restriction_keyed_like_any_enumeration() {
    let (program, _) = mapping();
    assert_eq!(
        type_shape(&program, "21"),
        Some(("Restriction".into(), Some(16), Some("BinaryValue".into())))
    );
    let values: Vec<_> = program
        .enumerations
        .iter()
        .filter(|e| e.parameter_type_id == format!("{}_PT-21", program.id))
        .map(|e| {
            (
                e.id.rsplit('_').next().unwrap().to_string(),
                e.value.clone(),
                e.text.clone(),
                e.display_order,
            )
        })
        .collect();
    assert_eq!(
        values,
        [
            ("EN-1".into(), "1".into(), Some("One".into()), 0),
            ("EN-513".into(), "513".into(), Some("Two-oh-one".into()), 1),
        ]
    );
    let long_enum = parameter(&program, "21");
    assert_eq!(long_enum.value.as_deref(), Some("513"));
    assert_eq!(long_enum.access.as_deref(), Some("Read"));
}

#[test]
fn binary_values_are_checked_and_reported_not_silently_dropped() {
    let (program, diagnostics) = mapping();
    let reported: Vec<_> = diagnostics
        .iter()
        .filter_map(|d| match d {
            MappingDiagnostic::BinaryValuesNotModeled {
                parameter_type,
                values,
                inconsistent,
            } => Some((parameter_type.as_str(), *values, *inconsistent)),
            _ => None,
        })
        .collect();
    assert_eq!(reported, [("21", 2, 0), ("22", 1, 1)]);
    assert!(diagnostics.iter().all(
        |d| d.kind() != "binary-values-not-modeled" || d.to_string().contains("stored payload")
    ));
    // The inconsistent type is still mapped: its value and text are what ETS
    // keys the enumeration by.
    assert_eq!(
        type_shape(&program, "22"),
        Some(("Restriction".into(), Some(16), Some("BinaryValue".into())))
    );
}

#[test]
fn an_undescribed_atomic_type_is_still_reported_and_not_mapped() {
    let (program, diagnostics) = mapping();
    assert_eq!(type_shape(&program, "23"), None);
    let unknown: Vec<_> = diagnostics
        .iter()
        .filter_map(|d| match d {
            MappingDiagnostic::UnknownAtomicType {
                parameter_type,
                atomic_type,
            } => Some((parameter_type.as_str(), atomic_type.as_str())),
            _ => None,
        })
        .collect();
    assert_eq!(unknown, [("23", "9")]);
}
