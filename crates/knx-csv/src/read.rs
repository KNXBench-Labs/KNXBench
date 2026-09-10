//! The "KNXBench group-address CSV v1" reader (see the crate-level docs for
//! why that name, and not "ETS CSV").
//!
//! Design: `docs/superpowers/specs/2026-09-10-csv-group-address-exchange-design.md`
//! §3 (the format) and §4 (import semantics this reader's *rows* feed).

use knx_core::{GroupAddress, GroupAddressStyle};
use serde::Serialize;

/// Mirrors `knx_etsproj::report::Severity`'s shape (lowercase on the wire).
/// Duplicated rather than shared because `knx-csv` must not depend on
/// `knx-etsproj` (see the layering rule in `xtask`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Error,
    Warning,
}

/// One problem found while reading a CSV file. `row` is the 1-based line
/// number counting the header row, matching what a spreadsheet displays;
/// `None` means the problem applies to the whole file (e.g. a missing
/// required column), not to any single row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CsvProblem {
    pub row: Option<usize>,
    pub severity: Severity,
    pub detail: String,
}

/// Why a header column was not used to build [`CsvRow`]s.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IgnoredColumnReason {
    /// A column this format defines (`DatapointType`, `MainGroup`,
    /// `MiddleGroup`) but that is derived on export and never applied on
    /// import — see design §3.
    ExportOnly,
    /// A column this reader does not recognize at all.
    Unknown,
}

/// A header column the reader did not turn into row data, kept so nothing
/// about the input file is silently dropped.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IgnoredColumn {
    pub name: String,
    pub reason: IgnoredColumnReason,
}

/// One successfully parsed data row. `central`/`unfiltered` are `None` when
/// the column was absent or the cell was empty ("leave unchanged"), never a
/// silent `Some(false)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CsvRow {
    pub line: usize,
    pub address: GroupAddress,
    pub name: String,
    pub central: Option<bool>,
    pub unfiltered: Option<bool>,
}

/// The result of reading a "KNXBench group-address CSV v1" file: every row
/// that parsed cleanly, every problem found (file- or row-level), and every
/// header column that was not turned into row data.
#[derive(Debug, Clone, PartialEq)]
pub struct ParsedCsv {
    pub separator: char,
    pub rows: Vec<CsvRow>,
    pub ignored_columns: Vec<IgnoredColumn>,
    pub problems: Vec<CsvProblem>,
}

/// Which known column, if any, a header cell was matched to.
#[derive(Debug, Default)]
struct HeaderMap {
    address: Option<usize>,
    name: Option<usize>,
    central: Option<usize>,
    unfiltered: Option<usize>,
}

/// Reads "KNXBench group-address CSV v1" `text` and produces typed rows.
///
/// `style` is the project's own [`GroupAddressStyle`]; addresses are parsed
/// through [`GroupAddress::parse`] exclusively, never hand-rolled.
///
/// Never panics on malformed input: every problem becomes a [`CsvProblem`].
/// A missing `Address` or `Name` column, or an empty file, produces a single
/// file-level problem (`row: None`) and no rows.
pub fn parse_group_addresses(text: &str, style: GroupAddressStyle) -> ParsedCsv {
    let text = strip_bom(text);

    if text.trim().is_empty() {
        return ParsedCsv {
            separator: ',',
            rows: Vec::new(),
            ignored_columns: Vec::new(),
            problems: vec![CsvProblem {
                row: None,
                severity: Severity::Error,
                detail: "file is empty".to_string(),
            }],
        };
    }

    let separator = detect_separator(text);

    let mut reader = csv::ReaderBuilder::new()
        .delimiter(separator as u8)
        .has_headers(true)
        .flexible(false)
        .from_reader(text.as_bytes());

    let headers = match reader.headers() {
        Ok(h) => h.clone(),
        Err(e) => {
            return ParsedCsv {
                separator,
                rows: Vec::new(),
                ignored_columns: Vec::new(),
                problems: vec![CsvProblem {
                    row: None,
                    severity: Severity::Error,
                    detail: format!("failed to read header row: {e}"),
                }],
            };
        }
    };

    let (columns, ignored_columns) = map_headers(&headers);

    let mut problems = Vec::new();
    if columns.address.is_none() {
        problems.push(CsvProblem {
            row: None,
            severity: Severity::Error,
            detail: "missing required column \"Address\"".to_string(),
        });
    }
    if columns.name.is_none() {
        problems.push(CsvProblem {
            row: None,
            severity: Severity::Error,
            detail: "missing required column \"Name\"".to_string(),
        });
    }
    if columns.address.is_none() || columns.name.is_none() {
        return ParsedCsv {
            separator,
            rows: Vec::new(),
            ignored_columns,
            problems,
        };
    }
    let address_col = columns.address.expect("checked above");
    let name_col = columns.name.expect("checked above");

    // The `csv` crate's `Position::line()` (verified empirically against
    // `csv` 1.4.0, undocumented) is exactly 1 too low for the *entire* file
    // when it uses CRLF terminators, and exactly correct for LF, even across
    // records with embedded newlines in quoted fields. Counting records
    // instead (as an earlier version of this function did) breaks down the
    // moment any earlier record spans more than one physical line, because
    // the record index no longer matches the line index at all.
    //
    // Fix: calibrate a single per-file additive offset once, right after the
    // header row is consumed. The header is always exactly one physical
    // line, so the first data row is always true physical line 2; comparing
    // that known-good value against what `position().line()` reports at
    // this point gives the file's constant (0 for LF, 1 for CRLF). Applying
    // that offset to every subsequent `position().line()` — including on
    // the malformed-row error path below — keeps line tracking correct for
    // multi-line records too, since `line()` itself (unlike the record
    // index) already accounts for embedded newlines correctly.
    let line_offset = 2u64.saturating_sub(reader.position().line());
    let line_of = |p: &csv::Position| (p.line() + line_offset) as usize;

    let mut rows = Vec::new();
    for result in reader.records() {
        let record = match result {
            Ok(r) => r,
            Err(e) => {
                problems.push(CsvProblem {
                    row: e.position().map(line_of),
                    severity: Severity::Error,
                    detail: format!("malformed row: {e}"),
                });
                continue;
            }
        };
        let line = record.position().map(line_of).unwrap_or(0);
        let mut row_ok = true;

        let address_raw = record.get(address_col).unwrap_or("").trim();
        let address = match GroupAddress::parse(address_raw, style) {
            Ok(a) if a.raw() == 0 => {
                problems.push(CsvProblem {
                    row: Some(line),
                    severity: Severity::Error,
                    detail: "group address 0 is reserved for broadcast and cannot be used"
                        .to_string(),
                });
                row_ok = false;
                None
            }
            Ok(a) => Some(a),
            Err(e) => {
                problems.push(CsvProblem {
                    row: Some(line),
                    severity: Severity::Error,
                    detail: e.to_string(),
                });
                row_ok = false;
                None
            }
        };

        let name_raw = record.get(name_col).unwrap_or("");
        if name_raw.trim().is_empty() {
            problems.push(CsvProblem {
                row: Some(line),
                severity: Severity::Error,
                detail: "name is blank".to_string(),
            });
            row_ok = false;
        }

        let central = read_bool_column(
            &record,
            columns.central,
            "Central",
            line,
            &mut problems,
            &mut row_ok,
        );
        let unfiltered = read_bool_column(
            &record,
            columns.unfiltered,
            "Unfiltered",
            line,
            &mut problems,
            &mut row_ok,
        );

        if !row_ok {
            continue;
        }

        rows.push(CsvRow {
            line,
            address: address.expect("row_ok implies address parsed and was nonzero"),
            name: name_raw.to_string(),
            central,
            unfiltered,
        });
    }

    ParsedCsv {
        separator,
        rows,
        ignored_columns,
        problems,
    }
}

/// Reads and validates one optional boolean column, pushing a row-level
/// problem and clearing `row_ok` on an unrecognized spelling. `column`
/// being absent from the header, or the cell being empty, both read as
/// `None` ("leave unchanged"), never a silent `Some(false)`.
fn read_bool_column(
    record: &csv::StringRecord,
    column: Option<usize>,
    label: &str,
    line: usize,
    problems: &mut Vec<CsvProblem>,
    row_ok: &mut bool,
) -> Option<bool> {
    let raw = column.and_then(|i| record.get(i))?;
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }
    match trimmed.to_ascii_lowercase().as_str() {
        "true" | "1" | "yes" => Some(true),
        "false" | "0" | "no" => Some(false),
        _ => {
            problems.push(CsvProblem {
                row: Some(line),
                severity: Severity::Error,
                detail: format!("column \"{label}\": unrecognized boolean value {trimmed:?}"),
            });
            *row_ok = false;
            None
        }
    }
}

/// Maps header cells to known columns, case-insensitively and trimmed of
/// surrounding whitespace, independent of column order. Everything else is
/// collected as an [`IgnoredColumn`] rather than silently dropped.
fn map_headers(header: &csv::StringRecord) -> (HeaderMap, Vec<IgnoredColumn>) {
    let mut columns = HeaderMap::default();
    let mut ignored = Vec::new();

    for (i, cell) in header.iter().enumerate() {
        let trimmed = cell.trim();
        match trimmed.to_ascii_lowercase().as_str() {
            "address" => columns.address = Some(i),
            "name" => columns.name = Some(i),
            "central" => columns.central = Some(i),
            "unfiltered" => columns.unfiltered = Some(i),
            "datapointtype" | "maingroup" | "middlegroup" => ignored.push(IgnoredColumn {
                name: trimmed.to_string(),
                reason: IgnoredColumnReason::ExportOnly,
            }),
            _ => ignored.push(IgnoredColumn {
                name: trimmed.to_string(),
                reason: IgnoredColumnReason::Unknown,
            }),
        }
    }

    (columns, ignored)
}

/// Strips a leading UTF-8 BOM, if present. Excel writes one so it opens the
/// file as UTF-8 instead of guessing the system code page; this reader
/// accepts files with or without it.
fn strip_bom(text: &str) -> &str {
    text.strip_prefix('\u{FEFF}').unwrap_or(text)
}

/// Counts `,` versus `;` outside quoted spans on the header line only,
/// defaulting to `,` on a tie. `text.lines()` strips a trailing `\r`, so
/// this works for both CRLF and LF input.
fn detect_separator(text: &str) -> char {
    let header_line = text.lines().next().unwrap_or("");

    let mut in_quotes = false;
    let mut commas = 0usize;
    let mut semicolons = 0usize;
    let mut chars = header_line.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '"' => {
                if in_quotes && chars.peek() == Some(&'"') {
                    chars.next();
                } else {
                    in_quotes = !in_quotes;
                }
            }
            ',' if !in_quotes => commas += 1,
            ';' if !in_quotes => semicolons += 1,
            _ => {}
        }
    }

    if semicolons > commas {
        ';'
    } else {
        ','
    }
}

#[cfg(test)]
mod tests {
    use super::super::*;
    use knx_core::GroupAddressStyle;

    #[test]
    fn parses_minimal_comma_file() {
        let text = "Address,Name\n100,Kitchen Light\n";
        let parsed = parse_group_addresses(text, GroupAddressStyle::Free);

        assert_eq!(parsed.separator, ',');
        assert!(parsed.problems.is_empty(), "{:?}", parsed.problems);
        assert!(parsed.ignored_columns.is_empty());
        assert_eq!(parsed.rows.len(), 1);
        let row = &parsed.rows[0];
        assert_eq!(row.line, 2);
        assert_eq!(row.address.raw(), 100);
        assert_eq!(row.name, "Kitchen Light");
        assert_eq!(row.central, None);
        assert_eq!(row.unfiltered, None);
    }

    #[test]
    fn parses_the_same_file_semicolon_separated() {
        let text = "Address;Name\n100;Kitchen Light\n";
        let parsed = parse_group_addresses(text, GroupAddressStyle::Free);

        assert_eq!(parsed.separator, ';');
        assert!(parsed.problems.is_empty(), "{:?}", parsed.problems);
        assert_eq!(parsed.rows.len(), 1);
        assert_eq!(parsed.rows[0].address.raw(), 100);
        assert_eq!(parsed.rows[0].name, "Kitchen Light");
    }

    #[test]
    fn strips_a_leading_utf8_bom() {
        let text = "\u{FEFF}Address,Name\n100,K\u{fc}che\n";
        let parsed = parse_group_addresses(text, GroupAddressStyle::Free);

        assert!(parsed.problems.is_empty(), "{:?}", parsed.problems);
        assert_eq!(parsed.rows.len(), 1);
        assert_eq!(parsed.rows[0].name, "K\u{fc}che");
    }

    #[test]
    fn accepts_crlf_line_endings() {
        let text = "Address,Name\r\n100,Kitchen Light\r\n200,Hallway\r\n";
        let parsed = parse_group_addresses(text, GroupAddressStyle::Free);

        assert!(parsed.problems.is_empty(), "{:?}", parsed.problems);
        assert_eq!(parsed.rows.len(), 2);
        assert_eq!(parsed.rows[0].line, 2);
        assert_eq!(parsed.rows[1].line, 3);
    }

    #[test]
    fn parses_a_quoted_name_with_separator_doubled_quote_and_umlaut() {
        let text = "Address,Name\n100,\"K\u{fc}che, \"\"gro\u{df}\"\"\"\n";
        let parsed = parse_group_addresses(text, GroupAddressStyle::Free);

        assert!(parsed.problems.is_empty(), "{:?}", parsed.problems);
        assert_eq!(parsed.rows.len(), 1);
        assert_eq!(parsed.rows[0].name, "K\u{fc}che, \"gro\u{df}\"");
    }

    #[test]
    fn parses_free_style_addresses() {
        let text = "Address,Name\n65535,Top\n";
        let parsed = parse_group_addresses(text, GroupAddressStyle::Free);

        assert!(parsed.problems.is_empty(), "{:?}", parsed.problems);
        assert_eq!(parsed.rows[0].address.raw(), 65535);
    }

    #[test]
    fn parses_two_level_style_addresses() {
        let text = "Address,Name\n5/100,Living Room\n";
        let parsed = parse_group_addresses(text, GroupAddressStyle::TwoLevel);

        assert!(parsed.problems.is_empty(), "{:?}", parsed.problems);
        assert_eq!(
            parsed.rows[0].address,
            knx_core::GroupAddress::parse("5/100", GroupAddressStyle::TwoLevel).unwrap()
        );
    }

    #[test]
    fn parses_three_level_style_addresses() {
        let text = "Address,Name\n5/3/100,Living Room\n";
        let parsed = parse_group_addresses(text, GroupAddressStyle::ThreeLevel);

        assert!(parsed.problems.is_empty(), "{:?}", parsed.problems);
        assert_eq!(
            parsed.rows[0].address,
            knx_core::GroupAddress::parse("5/3/100", GroupAddressStyle::ThreeLevel).unwrap()
        );
    }

    #[test]
    fn header_matching_is_case_insensitive_whitespace_tolerant_and_order_independent() {
        // Name before Address, mixed case, stray surrounding whitespace.
        let text = " name , ADDRESS \nKitchen Light,100\n";
        let parsed = parse_group_addresses(text, GroupAddressStyle::Free);

        assert!(parsed.problems.is_empty(), "{:?}", parsed.problems);
        assert!(parsed.ignored_columns.is_empty());
        assert_eq!(parsed.rows.len(), 1);
        assert_eq!(parsed.rows[0].address.raw(), 100);
        assert_eq!(parsed.rows[0].name, "Kitchen Light");
    }

    #[test]
    fn collects_an_unknown_column_by_name() {
        let text = "Address,Name,Foo\n100,Kitchen Light,bar\n";
        let parsed = parse_group_addresses(text, GroupAddressStyle::Free);

        assert!(parsed.problems.is_empty(), "{:?}", parsed.problems);
        assert_eq!(parsed.ignored_columns.len(), 1);
        assert_eq!(parsed.ignored_columns[0].name, "Foo");
        assert_eq!(
            parsed.ignored_columns[0].reason,
            IgnoredColumnReason::Unknown
        );
    }

    #[test]
    fn collects_the_export_only_columns_as_recognized_but_ignored() {
        let text = "Address,Name,DatapointType,MainGroup,MiddleGroup\n\
                     100,Kitchen Light,DPST-1-1,Lighting,Ground Floor\n";
        let parsed = parse_group_addresses(text, GroupAddressStyle::Free);

        assert!(parsed.problems.is_empty(), "{:?}", parsed.problems);
        assert_eq!(parsed.ignored_columns.len(), 3);
        for col in &parsed.ignored_columns {
            assert_eq!(col.reason, IgnoredColumnReason::ExportOnly);
        }
        let mut names: Vec<&str> = parsed
            .ignored_columns
            .iter()
            .map(|c| c.name.as_str())
            .collect();
        names.sort();
        assert_eq!(names, vec!["DatapointType", "MainGroup", "MiddleGroup"]);
        assert_eq!(parsed.rows.len(), 1);
    }

    #[test]
    fn missing_address_column_is_a_file_level_error() {
        let text = "Name\nKitchen Light\n";
        let parsed = parse_group_addresses(text, GroupAddressStyle::Free);

        assert!(parsed.rows.is_empty());
        assert_eq!(parsed.problems.len(), 1);
        assert_eq!(parsed.problems[0].row, None);
        assert_eq!(parsed.problems[0].severity, Severity::Error);
        assert!(parsed.problems[0].detail.to_lowercase().contains("address"));
    }

    #[test]
    fn missing_name_column_is_a_file_level_error() {
        let text = "Address\n100\n";
        let parsed = parse_group_addresses(text, GroupAddressStyle::Free);

        assert!(parsed.rows.is_empty());
        assert_eq!(parsed.problems.len(), 1);
        assert_eq!(parsed.problems[0].row, None);
        assert_eq!(parsed.problems[0].severity, Severity::Error);
        assert!(parsed.problems[0].detail.to_lowercase().contains("name"));
    }

    #[test]
    fn row_error_for_an_unparseable_address() {
        let text = "Address,Name\nnot-an-address,Kitchen Light\n";
        let parsed = parse_group_addresses(text, GroupAddressStyle::Free);

        assert!(parsed.rows.is_empty());
        assert_eq!(parsed.problems.len(), 1);
        assert_eq!(parsed.problems[0].row, Some(2));
        assert_eq!(parsed.problems[0].severity, Severity::Error);
    }

    #[test]
    fn row_error_for_an_out_of_range_address() {
        // TwoLevel main is 0..=31.
        let text = "Address,Name\n99/100,Kitchen Light\n";
        let parsed = parse_group_addresses(text, GroupAddressStyle::TwoLevel);

        assert!(parsed.rows.is_empty());
        assert_eq!(parsed.problems.len(), 1);
        assert_eq!(parsed.problems[0].row, Some(2));
        assert_eq!(parsed.problems[0].severity, Severity::Error);
    }

    #[test]
    fn row_error_for_address_zero() {
        let text = "Address,Name\n0,Kitchen Light\n";
        let parsed = parse_group_addresses(text, GroupAddressStyle::Free);

        assert!(parsed.rows.is_empty());
        assert_eq!(parsed.problems.len(), 1);
        assert_eq!(parsed.problems[0].row, Some(2));
        assert_eq!(parsed.problems[0].severity, Severity::Error);
    }

    #[test]
    fn row_error_for_a_blank_name() {
        let text = "Address,Name\n100,   \n";
        let parsed = parse_group_addresses(text, GroupAddressStyle::Free);

        assert!(parsed.rows.is_empty());
        assert_eq!(parsed.problems.len(), 1);
        assert_eq!(parsed.problems[0].row, Some(2));
        assert_eq!(parsed.problems[0].severity, Severity::Error);
    }

    #[test]
    fn row_error_for_an_unrecognized_boolean() {
        let text = "Address,Name,Central\n100,Kitchen Light,maybe\n";
        let parsed = parse_group_addresses(text, GroupAddressStyle::Free);

        assert!(parsed.rows.is_empty());
        assert_eq!(parsed.problems.len(), 1);
        assert_eq!(parsed.problems[0].row, Some(2));
        assert_eq!(parsed.problems[0].severity, Severity::Error);
    }

    #[test]
    fn empty_file_is_a_single_file_level_error() {
        let parsed = parse_group_addresses("", GroupAddressStyle::Free);

        assert_eq!(parsed.separator, ',');
        assert!(parsed.rows.is_empty());
        assert!(parsed.ignored_columns.is_empty());
        assert_eq!(parsed.problems.len(), 1);
        assert_eq!(parsed.problems[0].row, None);
        assert_eq!(parsed.problems[0].severity, Severity::Error);
        assert_eq!(parsed.problems[0].detail, "file is empty");
    }

    #[test]
    fn whitespace_only_file_is_a_single_file_level_error() {
        let parsed = parse_group_addresses("   \n\t\n  ", GroupAddressStyle::Free);

        assert!(parsed.rows.is_empty());
        assert_eq!(parsed.problems.len(), 1);
        assert_eq!(parsed.problems[0].row, None);
        assert_eq!(parsed.problems[0].severity, Severity::Error);
        assert_eq!(parsed.problems[0].detail, "file is empty");
    }

    #[test]
    fn malformed_row_under_strict_mode_reports_the_row_and_its_physical_line() {
        // `flexible(false)` rejects a ragged row (3 fields where every other
        // row has 2); the offending row itself never becomes a `CsvRow`.
        let text = "Address,Name\n100,Kitchen Light\n200,Extra,Field\n";
        let parsed = parse_group_addresses(text, GroupAddressStyle::Free);

        assert_eq!(parsed.rows.len(), 1, "{:?}", parsed.rows);
        assert_eq!(parsed.rows[0].name, "Kitchen Light");
        assert_eq!(parsed.problems.len(), 1);
        assert_eq!(parsed.problems[0].row, Some(3));
        assert_eq!(parsed.problems[0].severity, Severity::Error);
        assert!(
            parsed.problems[0].detail.contains("malformed row"),
            "{:?}",
            parsed.problems[0]
        );
    }

    // --- Finding 1 regression: line numbers after a multi-line quoted field ---
    //
    // Both cases put a name with an embedded newline in row 1 (a legal
    // quoted field per design §3/§8), then a blank-name error in row 2.
    // A spreadsheet opening either file shows the blank-name row on
    // physical line 4 (1: header, 2-3: row 1's two physical lines, 4: row
    // 2). Before this fix, `record()+1` reported line 3 instead, because it
    // counts *records*, not physical lines, and row 1 consumed two of them.
    //
    // Values observed against the real `csv` 1.4.0 dependency while writing
    // this fix (see the Task 1 fix report for the full table):
    //   LF:   row 1 `position().line()` = 2, row 2 = 4, line_offset = 0.
    //   CRLF: row 1 `position().line()` = 1, row 2 = 3, line_offset = 1.
    // Both resolve to the same correct physical line 4 for row 2.

    #[test]
    fn line_number_stays_correct_after_a_multiline_quoted_field_lf() {
        let text = "Address,Name\n100,\"Living Room\nAnnex\"\n200,\n";
        let parsed = parse_group_addresses(text, GroupAddressStyle::Free);

        assert_eq!(parsed.rows.len(), 1, "{:?}", parsed.rows);
        assert_eq!(parsed.rows[0].line, 2);
        assert_eq!(parsed.rows[0].name, "Living Room\nAnnex");
        assert_eq!(parsed.problems.len(), 1);
        assert_eq!(
            parsed.problems[0].row,
            Some(4),
            "expected the blank-name row to be reported as physical line 4, got {:?}",
            parsed.problems[0]
        );
    }

    #[test]
    fn line_number_stays_correct_after_a_multiline_quoted_field_crlf() {
        let text = "Address,Name\r\n100,\"Living Room\r\nAnnex\"\r\n200,\r\n";
        let parsed = parse_group_addresses(text, GroupAddressStyle::Free);

        assert_eq!(parsed.rows.len(), 1, "{:?}", parsed.rows);
        assert_eq!(parsed.rows[0].line, 2);
        assert_eq!(parsed.rows[0].name, "Living Room\r\nAnnex");
        assert_eq!(parsed.problems.len(), 1);
        assert_eq!(
            parsed.problems[0].row,
            Some(4),
            "expected the blank-name row to be reported as physical line 4, got {:?}",
            parsed.problems[0]
        );
    }

    #[test]
    fn central_and_unfiltered_accept_every_documented_boolean_spelling() {
        let text = "Address,Name,Central,Unfiltered\n\
                     100,A,true,0\n\
                     101,B,FALSE,yes\n\
                     102,C,1,No\n\
                     103,D,,\n";
        let parsed = parse_group_addresses(text, GroupAddressStyle::Free);

        assert!(parsed.problems.is_empty(), "{:?}", parsed.problems);
        assert_eq!(parsed.rows.len(), 4);
        assert_eq!(parsed.rows[0].central, Some(true));
        assert_eq!(parsed.rows[0].unfiltered, Some(false));
        assert_eq!(parsed.rows[1].central, Some(false));
        assert_eq!(parsed.rows[1].unfiltered, Some(true));
        assert_eq!(parsed.rows[2].central, Some(true));
        assert_eq!(parsed.rows[2].unfiltered, Some(false));
        // Empty cell means "unchanged" (None), never Some(false).
        assert_eq!(parsed.rows[3].central, None);
        assert_eq!(parsed.rows[3].unfiltered, None);
    }
}
