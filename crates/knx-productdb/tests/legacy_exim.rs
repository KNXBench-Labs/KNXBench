//! The legacy EX-IM line grammar: what parses, what is refused and what is only reported.

use knx_productdb::legacy::{
    parse_exim, parse_exim_with_limits, ExImContent, ExImDiagnostic, ExImLimits, LegacyError,
};

fn fixture(path: &str) -> Vec<u8> {
    let full = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures/legacy")
        .join(path);
    std::fs::read(&full).unwrap_or_else(|e| panic!("fixture {full:?} unreadable: {e}"))
}

/// Builds an EX-IM payload from `\n`-separated lines, joined with CRLF and
/// terminated by one final CRLF, the way the observed files are.
fn payload(lines: &str) -> Vec<u8> {
    let mut out = lines.lines().collect::<Vec<_>>().join("\r\n").into_bytes();
    out.extend_from_slice(b"\r\n");
    out
}

const MINIMAL: &str = "EX-IM
N C:\\x\\ets.vd_
K ETS3
V 6.2
H virtual_device
-------------------------------------
T 3 manufacturer
C1 T3 1 4 N MANUFACTURER_ID
C2 T3 3 50 Y MANUFACTURER_NAME
R 1 T 3 manufacturer
4242
Marvin Test
XXX";

fn syntax_line(result: Result<impl std::fmt::Debug, LegacyError>) -> (usize, String) {
    match result {
        Err(LegacyError::Syntax { line, reason }) => (line, reason),
        other => panic!("expected LegacyError::Syntax, got {other:?}"),
    }
}

#[test]
fn the_synthetic_product_database_parses_completely() {
    let doc = parse_exim(&fixture("src-vd/MARVIN/ets.vd_")).unwrap();
    assert_eq!(doc.content(), ExImContent::ProductDatabase);
    assert_eq!(doc.format_version(), Some("6.2"));
    let names: Vec<&str> = doc.tables().iter().map(|t| t.name()).collect();
    assert_eq!(
        names,
        [
            "manufacturer",
            "mask",
            "hw_product",
            "catalog_entry",
            "application_program",
            "virtual_device",
            "parameter"
        ]
    );
    let catalog = doc.table("catalog_entry").unwrap();
    assert_eq!(catalog.id(), 10);
    assert_eq!(catalog.row_count(), 2);
    assert_eq!(catalog.columns().len(), 5);
    assert_eq!(catalog.columns()[3].name, "ORDER_NUMBER");
    assert_eq!(catalog.columns()[3].type_code, 3);
    assert_eq!(catalog.columns()[3].size, 20);
    assert!(catalog.columns()[3].nullable);
    assert!(!catalog.columns()[0].nullable);
    assert_eq!(
        catalog.text_by_name(1, "ORDER_NUMBER").as_deref(),
        Some("MT-42-B")
    );
    // 0xE9 decodes as Windows-1252 (and Latin-1) `é`.
    assert_eq!(
        catalog.text_by_name(1, "ENTRY_NAME").as_deref(),
        Some("Heart of Gold Sensor b\u{e9}ta")
    );
    assert!(doc.diagnostics().is_empty(), "{:?}", doc.diagnostics());
}

#[test]
fn continuation_lines_join_without_a_separator() {
    let doc = parse_exim(&fixture("src-vd/MARVIN/ets.vd_")).unwrap();
    let program = doc.table("application_program").unwrap();
    let eeprom = program.text_by_name(0, "EEPROM_DATA").unwrap();
    let expected = format!(
        "{}{}00FF",
        "0123456789ABCDEF".repeat(5),
        "FEDCBA9876543210".repeat(5)
    );
    assert_eq!(eeprom, expected);
    assert_eq!(doc.continuation_lines(), 2);
}

#[test]
fn dash_lines_inside_a_row_are_values_not_separators() {
    let doc = parse_exim(&fixture("src-vd/MARVIN/ets.vd_")).unwrap();
    let parameter = doc.table("parameter").unwrap();
    assert_eq!(
        parameter
            .text_by_name(0, "PARAMETER_DESCRIPTION")
            .as_deref(),
        Some("----")
    );
    assert_eq!(
        parameter
            .text_by_name(1, "PARAMETER_DESCRIPTION")
            .as_deref(),
        Some("-")
    );
}

#[test]
fn the_synthetic_project_export_is_recognised_and_may_hold_empty_tables() {
    let doc = parse_exim(&fixture("src-pr/MARVIN/ets.pr_")).unwrap();
    assert_eq!(doc.content(), ExImContent::ProjectExport);
    assert_eq!(doc.table("application_program").unwrap().row_count(), 0);
}

#[test]
fn an_unknown_content_header_is_kept_verbatim() {
    let doc = parse_exim(&payload(
        &MINIMAL.replace("H virtual_device", "H something"),
    ))
    .unwrap();
    assert_eq!(doc.content(), ExImContent::Other("something".into()));
}

#[test]
fn raw_bytes_survive_beside_the_decoded_text() {
    let text = MINIMAL.replace("Marvin Test", "Marvin T\u{e9}st");
    let mut bytes = payload(&text);
    // Re-encode the one non-ASCII character as its single Windows-1252 byte.
    let at = bytes
        .windows(2)
        .position(|w| w == "\u{e9}".as_bytes())
        .unwrap();
    bytes.splice(at..at + 2, [0xE9]);
    let doc = parse_exim(&bytes).unwrap();
    let table = doc.table("manufacturer").unwrap();
    assert_eq!(table.raw(0, 1), b"Marvin T\xe9st");
    assert_eq!(table.text(0, 1), "Marvin T\u{e9}st");
}

#[test]
fn bytes_in_the_windows_1252_only_range_are_reported_not_refused() {
    let mut bytes = payload(&MINIMAL.replace("Marvin Test", "Marvin XTest"));
    let at = bytes.windows(5).position(|w| w == b"XTest").unwrap();
    bytes[at] = 0x80;
    let doc = parse_exim(&bytes).unwrap();
    assert_eq!(
        doc.table("manufacturer").unwrap().text(0, 1),
        "Marvin \u{20ac}Test"
    );
    assert!(doc
        .diagnostics()
        .contains(&ExImDiagnostic::Windows1252OnlyBytes { count: 1 }));
}

#[test]
fn unknown_header_keys_type_codes_and_empty_required_values_are_reported() {
    let text = MINIMAL
        .replace("V 6.2", "V 6.2\nQ odd")
        .replace("C2 T3 3 50 Y", "C2 T3 7 50 Y")
        .replace("4242\nMarvin", "\nMarvin");
    let doc = parse_exim(&payload(&text)).unwrap();
    let diagnostics = doc.diagnostics();
    assert!(
        diagnostics.contains(&ExImDiagnostic::UnknownHeaderKey {
            line: 5,
            key: "Q".into()
        }),
        "{diagnostics:?}"
    );
    assert!(
        diagnostics.contains(&ExImDiagnostic::UnknownTypeCode {
            table: "manufacturer".into(),
            column: "MANUFACTURER_NAME".into(),
            type_code: 7
        }),
        "{diagnostics:?}"
    );
    assert!(
        diagnostics.contains(&ExImDiagnostic::EmptyRequiredValues {
            table: "manufacturer".into(),
            column: "MANUFACTURER_ID".into(),
            count: 1
        }),
        "{diagnostics:?}"
    );
}

#[test]
fn a_missing_magic_line_is_refused() {
    let (line, reason) = syntax_line(parse_exim(&payload(&MINIMAL.replace("EX-IM", "EX-OUT"))));
    assert_eq!(line, 1, "{reason}");
}

#[test]
fn bare_line_feeds_and_carriage_returns_are_refused() {
    let mut lf = payload(MINIMAL);
    let at = lf.windows(2).position(|w| w == b"\r\n").unwrap();
    lf.remove(at);
    let (line, reason) = syntax_line(parse_exim(&lf));
    assert_eq!(line, 1);
    assert!(reason.contains("bare LF"), "{reason}");

    let mut cr = payload(MINIMAL);
    let at = cr.iter().position(|&b| b == b'T').unwrap();
    cr.insert(at, b'\r');
    let (_, reason) = syntax_line(parse_exim(&cr));
    assert!(reason.contains("bare CR"), "{reason}");
}

#[test]
fn a_line_over_the_limit_is_refused() {
    let limits = ExImLimits {
        max_line_len: 20,
        ..ExImLimits::default()
    };
    // The 37-dash separator on line 6 is the first line longer than 20 bytes.
    let (line, reason) = syntax_line(parse_exim_with_limits(&payload(MINIMAL), limits));
    assert_eq!(line, 6, "{reason}");
}

#[test]
fn structural_mismatches_are_refused_with_their_line() {
    let cases = [
        (
            "C2 T3 3 50 Y MANUFACTURER_NAME",
            "C3 T3 3 50 Y MANUFACTURER_NAME",
            9,
        ),
        (
            "C2 T3 3 50 Y MANUFACTURER_NAME",
            "C2 T4 3 50 Y MANUFACTURER_NAME",
            9,
        ),
        (
            "C2 T3 3 50 Y MANUFACTURER_NAME",
            "C2 T3 3 50 Q MANUFACTURER_NAME",
            9,
        ),
        ("R 1 T 3 manufacturer", "R 2 T 3 manufacturer", 10),
        ("R 1 T 3 manufacturer", "R 1 T 4 manufacturer", 10),
        ("R 1 T 3 manufacturer", "R 1 T 3 hw_product", 10),
        ("T 3 manufacturer", "Z 3 manufacturer", 7),
        ("T 3 manufacturer", "T x manufacturer", 7),
        ("-------------------------------------", "+++", 6),
    ];
    for (from, to, expected) in cases {
        let text = MINIMAL.replace(from, to);
        let (line, reason) = syntax_line(parse_exim(&payload(&text)));
        assert_eq!(line, expected, "{to}: {reason}");
    }
}

#[test]
fn duplicate_tables_are_refused() {
    let second_id = "XXX".replace("XXX", "-----\nT 3 other\nC1 T3 1 4 N ID\nXXX");
    let text = MINIMAL.replace("XXX", &second_id);
    syntax_line(parse_exim(&payload(&text)));

    let second_name = MINIMAL.replace("XXX", "-----\nT 4 manufacturer\nC1 T4 1 4 N ID\nXXX");
    syntax_line(parse_exim(&payload(&second_name)));
}

#[test]
fn a_table_without_columns_is_refused() {
    let text = MINIMAL.replace("XXX", "-----\nT 4 empty\nXXX");
    syntax_line(parse_exim(&payload(&text)));
}

#[test]
fn a_truncated_row_is_refused() {
    let text = MINIMAL.replace("Marvin Test\nXXX", "");
    let bytes = payload(text.trim_end());
    syntax_line(parse_exim(&bytes));
}

#[test]
fn a_missing_end_marker_or_trailing_bytes_are_refused() {
    syntax_line(parse_exim(&payload(&MINIMAL.replace("\nXXX", ""))));
    let mut trailing = payload(MINIMAL);
    trailing.extend_from_slice(b"extra\r\n");
    syntax_line(parse_exim(&trailing));
    let mut unterminated = payload(MINIMAL);
    unterminated.truncate(unterminated.len() - 2);
    syntax_line(parse_exim(&unterminated));
}

#[test]
fn table_column_row_and_value_limits_are_enforced() {
    let two_tables = MINIMAL.replace("XXX", "-----\nT 4 other\nC1 T4 1 4 N ID\nXXX");
    let tables = ExImLimits {
        max_tables: 1,
        ..ExImLimits::default()
    };
    assert!(matches!(
        parse_exim_with_limits(&payload(&two_tables), tables),
        Err(LegacyError::SizeLimit { .. })
    ));
    let columns = ExImLimits {
        max_columns: 1,
        ..ExImLimits::default()
    };
    assert!(matches!(
        parse_exim_with_limits(&payload(MINIMAL), columns),
        Err(LegacyError::SizeLimit { .. })
    ));
    let rows = ExImLimits {
        max_rows: 0,
        ..ExImLimits::default()
    };
    assert!(matches!(
        parse_exim_with_limits(&payload(MINIMAL), rows),
        Err(LegacyError::SizeLimit { .. })
    ));
    let values = ExImLimits {
        max_values: 1,
        ..ExImLimits::default()
    };
    assert!(matches!(
        parse_exim_with_limits(&payload(MINIMAL), values),
        Err(LegacyError::SizeLimit { .. })
    ));
    let header = ExImLimits {
        max_header_lines: 2,
        ..ExImLimits::default()
    };
    assert!(matches!(
        parse_exim_with_limits(&payload(MINIMAL), header),
        Err(LegacyError::SizeLimit { .. })
    ));
}

#[test]
fn continuation_limits_are_enforced() {
    let text = MINIMAL.replace("Marvin Test", "Marvin\n\\\\Test\n\\\\s");
    let doc = parse_exim(&payload(&text)).unwrap();
    assert_eq!(doc.table("manufacturer").unwrap().text(0, 1), "MarvinTests");
    let continuations = ExImLimits {
        max_continuations_per_value: 1,
        ..ExImLimits::default()
    };
    assert!(matches!(
        parse_exim_with_limits(&payload(&text), continuations),
        Err(LegacyError::SizeLimit { .. })
    ));
    let value = ExImLimits {
        max_value_len: 8,
        ..ExImLimits::default()
    };
    assert!(matches!(
        parse_exim_with_limits(&payload(&text), value),
        Err(LegacyError::SizeLimit { .. })
    ));
}

/// The measured `.vd5` holds one value of 18,653,184 bytes over 233,164
/// continuation lines (table `Baggage`) and about 8.19 million values. The
/// default limits admit values of that shape; this one is 4 MB over 50,000
/// 80-byte lines, past the earlier 1 MiB / 16,384-line defaults.
#[test]
fn the_default_limits_admit_the_measured_vd5_shapes() {
    let defaults = ExImLimits::default();
    assert!(defaults.max_value_len >= 18_653_184 * 2, "{defaults:?}");
    assert!(
        defaults.max_continuations_per_value >= 233_164 * 2,
        "{defaults:?}"
    );
    assert!(defaults.max_values >= 8_186_041 * 2, "{defaults:?}");
    assert!(defaults.max_rows >= 872_166 * 2, "{defaults:?}");
    let chunk = "\\\\".to_string() + &"0123456789ABCDEF".repeat(5);
    let long = format!("Marvin\n{}", vec![chunk.as_str(); 50_000].join("\n"));
    let text = MINIMAL.replace("Marvin Test", &long);
    let doc = parse_exim(&payload(&text)).unwrap();
    assert_eq!(
        doc.table("manufacturer").unwrap().text(0, 1).len(),
        6 + 50_000 * 80
    );
}

#[test]
fn a_continuation_line_where_a_record_is_expected_is_refused() {
    // After a value, a `\\` line continues that value; after a table line it
    // has nothing to continue.
    let text = MINIMAL.replace("T 3 manufacturer\n", "T 3 manufacturer\n\\\\stray\n");
    let (line, _) = syntax_line(parse_exim(&payload(&text)));
    assert_eq!(line, 8);
}

#[test]
fn a_value_that_starts_with_the_continuation_prefix_is_refused_not_guessed() {
    // The first value of a row has no predecessor to continue, so a `\\`
    // there can only be the value's own text, which the observed encoding
    // cannot represent (design spec open question Q-3).
    let text = MINIMAL.replace("4242\nMarvin", "\\\\4242\nMarvin");
    let (line, reason) = syntax_line(parse_exim(&payload(&text)));
    assert_eq!(line, 11, "{reason}");
}

/// One manufacturer row whose name is `name` (raw EX-IM text, escapes
/// included), optionally followed by continuation lines.
fn with_name(name_lines: &str) -> Vec<u8> {
    payload(&MINIMAL.replace("Marvin Test", name_lines))
}

#[test]
fn value_escapes_are_decoded_and_the_raw_bytes_kept() {
    // Measured in real files: `\'`, `\r`, `\n` and `\\` (ETS's own
    // conversion shows `\'` as `'`).
    let doc = parse_exim(&with_name(r"l\'objet\r\nC:\\ETS3")).unwrap();
    let table = doc.table("manufacturer").unwrap();
    assert_eq!(table.text(0, 1), "l'objet\r\nC:\\ETS3");
    assert_eq!(table.raw(0, 1), br"l\'objet\r\nC:\\ETS3");
    assert!(doc.diagnostics().is_empty(), "{:?}", doc.diagnostics());
}

#[test]
fn an_escape_split_across_a_continuation_is_decoded_after_joining() {
    let doc = parse_exim(&with_name("first line ends in a backslash\\\n\\\\rsecond")).unwrap();
    assert_eq!(
        doc.table("manufacturer").unwrap().text(0, 1),
        "first line ends in a backslash\rsecond"
    );
}

#[test]
fn unknown_escapes_are_kept_verbatim_and_reported() {
    let doc = parse_exim(&with_name(r"tab\there\")).unwrap();
    assert_eq!(doc.table("manufacturer").unwrap().text(0, 1), r"tab\there\");
    assert_eq!(
        doc.diagnostics(),
        [ExImDiagnostic::UnknownEscapes { count: 2 }]
    );
}

#[test]
fn header_paths_keep_their_single_backslashes() {
    // `N` holds a raw Windows path, not an escaped value.
    let doc = parse_exim(&payload(
        &MINIMAL.replace("N C:\\x\\ets.vd_", r"N C:\Program Files\ets.vd_"),
    ))
    .unwrap();
    assert!(doc
        .header()
        .iter()
        .any(|(k, v)| k == "N" && v == r"C:\Program Files\ets.vd_"));
    assert!(doc.diagnostics().is_empty(), "{:?}", doc.diagnostics());
}
