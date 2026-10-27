//! Tests for the fixed group-address CSV import header profiles, with synthetic fixtures only.

use crate::testutil::{empty_project, entry_with_flags, linked_com_object, range};
use crate::{
    parse_group_addresses, plan_import, CsvProblem, IgnoredColumnReason, ParsedCsv, Severity,
};
use knx_core::{Command, DptRef, GroupAddress, GroupAddressId, GroupAddressStyle, GroupRangeId};

const TAIL: &str = "Central;Unfiltered;Description;DatapointType;Security";
const SHAPE1: &str = "Group name;Address;Central;Unfiltered;Description;DatapointType;Security";
const SHAPE2: &str =
    "Group name;Main;Middle;Sub;Central;Unfiltered;Description;DatapointType;Security";
const SHAPE3: &str =
    "Main;Middle;Sub;Address;Central;Unfiltered;Description;DatapointType;Security";
const SHAPE4: &str =
    "Main;Middle;Sub;Main;Middle;Sub;Central;Unfiltered;Description;DatapointType;Security";
const AUTO_WARNING: &str = "Security Auto is not imported; project security is unchanged";

fn three(text: &str) -> GroupAddress {
    GroupAddress::parse(text, GroupAddressStyle::ThreeLevel).unwrap()
}

fn parse(text: &str) -> ParsedCsv {
    parse_group_addresses(text, GroupAddressStyle::ThreeLevel)
}

fn file(header: &str, rows: &[&[&str]]) -> String {
    let mut text = format!("{header}\n");
    for row in rows {
        text.push_str(&row.join(";"));
        text.push('\n');
    }
    text
}

fn errors(parsed: &ParsedCsv) -> Vec<&CsvProblem> {
    parsed
        .problems
        .iter()
        .filter(|p| p.severity == Severity::Error)
        .collect()
}

fn warnings(parsed: &ParsedCsv) -> Vec<&CsvProblem> {
    parsed
        .problems
        .iter()
        .filter(|p| p.severity == Severity::Warning)
        .collect()
}

fn assert_clean(parsed: &ParsedCsv) {
    assert!(parsed.problems.is_empty(), "{:?}", parsed.problems);
}

fn assert_refused(text: &str) {
    let parsed = parse(text);
    assert!(parsed.rows.is_empty(), "{:?}", parsed.rows);
    let errs = errors(&parsed);
    assert_eq!(errs.len(), 1, "{:?}", parsed.problems);
    assert_eq!(errs[0].row, None, "{:?}", errs[0]);
    assert!(
        errs[0].detail.contains("not exactly one of the four"),
        "{:?}",
        errs[0]
    );
}

fn assert_single_row_error(text: &str, needle: &str) {
    let parsed = parse(text);
    assert!(parsed.rows.is_empty(), "{:?}", parsed.rows);
    let errs = errors(&parsed);
    assert_eq!(errs.len(), 1, "{:?}", parsed.problems);
    assert_eq!(errs[0].row, Some(2), "{:?}", errs[0]);
    assert!(errs[0].detail.contains(needle), "{:?}", errs[0]);
}

#[test]
fn shape_one_leaf_maps_name_address_and_flags() {
    let parsed = parse(&file(
        SHAPE1,
        &[&["Workshop", "1/2/3", "yes", "", "", "", ""]],
    ));
    assert_clean(&parsed);
    assert_eq!(parsed.rows.len(), 1);
    let row = &parsed.rows[0];
    assert_eq!((row.line, row.name.as_str()), (2, "Workshop"));
    assert_eq!(row.address, three("1/2/3"));
    assert_eq!((row.central, row.unfiltered), (Some(true), None));
    assert_eq!(row.datapoint_type, None);
    assert_eq!(
        (row.main_group.clone(), row.middle_group.clone()),
        (None, None)
    );
}

#[test]
fn shape_two_leaf_assembles_the_split_address() {
    let parsed = parse(&file(
        SHAPE2,
        &[&["Pump", "4", "5", "6", "", "no", "", "", ""]],
    ));
    assert_clean(&parsed);
    assert_eq!(parsed.rows[0].address, three("4/5/6"));
    assert_eq!(parsed.rows[0].name, "Pump");
    assert_eq!(
        (parsed.rows[0].central, parsed.rows[0].unfiltered),
        (None, Some(false))
    );
}

#[test]
fn shape_three_leaf_takes_the_sub_name_and_composite_address() {
    let parsed = parse(&file(
        SHAPE3,
        &[&["", "", "Upper floor lamp", "2/1/7", "", "", "", "", ""]],
    ));
    assert_clean(&parsed);
    assert_eq!(parsed.rows[0].address, three("2/1/7"));
    assert_eq!(parsed.rows[0].name, "Upper floor lamp");
}

#[test]
fn shape_four_leaf_uses_the_second_triplet_as_address() {
    let parsed = parse(&file(
        SHAPE4,
        &[&["", "", "Pump", "3", "0", "9", "", "", "", "", ""]],
    ));
    assert_clean(&parsed);
    assert_eq!(parsed.rows[0].address, three("3/0/9"));
    assert_eq!(parsed.rows[0].name, "Pump");
}

#[test]
fn headers_are_case_insensitive_and_trimmed() {
    let header = " GROUP NAME ;address; central ;UNFILTERED;description;datapointtype;SECURITY ";
    let parsed = parse(&file(header, &[&["Pump", "1/1/1", "", "", "", "", ""]]));
    assert_clean(&parsed);
    assert_eq!(parsed.rows.len(), 1);
}

#[test]
fn profile_dpt_description_and_security_columns_are_reported_as_not_applied() {
    let parsed = parse(&file(SHAPE1, &[]));
    assert_clean(&parsed);
    let names: Vec<_> = parsed
        .ignored_columns
        .iter()
        .map(|c| (c.name.as_str(), c.reason))
        .collect();
    assert_eq!(
        names,
        vec![
            ("Description", IgnoredColumnReason::ReadOnly),
            ("DatapointType", IgnoredColumnReason::ReadOnly),
            ("Security", IgnoredColumnReason::ReadOnly),
        ]
    );
}

#[test]
fn composite_ranges_are_skipped_with_an_explicit_warning() {
    let parsed = parse(&file(
        SHAPE1,
        &[
            &["Upper floor", "2/-/-", "", "", "", "", ""],
            &["Lighting", "2/1/-", "", "", "", "", ""],
        ],
    ));
    assert!(errors(&parsed).is_empty(), "{:?}", parsed.problems);
    assert!(parsed.rows.is_empty());
    let warns = warnings(&parsed);
    assert_eq!(warns.len(), 2);
    assert_eq!((warns[0].row, warns[1].row), (Some(2), Some(3)));
    for warn in warns {
        assert!(warn.detail.contains("not imported"), "{warn:?}");
    }
}

#[test]
fn split_ranges_are_skipped_with_an_explicit_warning() {
    let parsed = parse(&file(
        SHAPE2,
        &[
            &["Upper floor", "2", "", "", "", "", "", "", ""],
            &["Lighting", "2", "1", "", "", "", "", "", ""],
        ],
    ));
    assert!(errors(&parsed).is_empty(), "{:?}", parsed.problems);
    assert!(parsed.rows.is_empty());
    assert_eq!(warnings(&parsed).len(), 2);

    let named = parse(&file(
        SHAPE4,
        &[
            &["Upper floor", "", "", "2", "", "", "", "", "", "", ""],
            &["", "Lighting", "", "2", "1", "", "", "", "", "", ""],
        ],
    ));
    assert!(errors(&named).is_empty(), "{:?}", named.problems);
    assert!(named.rows.is_empty());
    assert_eq!(warnings(&named).len(), 2);
}

#[test]
fn range_rows_do_not_hide_later_leaf_rows() {
    let parsed = parse(&file(
        SHAPE1,
        &[
            &["Upper floor", "2/-/-", "", "", "", "", ""],
            &["Pump", "2/1/1", "", "", "", "", ""],
        ],
    ));
    assert_eq!(parsed.rows.len(), 1);
    assert_eq!(parsed.rows[0].line, 3);
}

#[test]
fn range_rows_never_plan_commands() {
    let project = empty_project(GroupAddressStyle::ThreeLevel);
    let parsed = parse(&file(
        SHAPE1,
        &[&["Upper floor", "2/-/-", "", "", "", "", ""]],
    ));
    let plan = plan_import(&project, &parsed);
    assert!(plan.command.is_none());
    assert_eq!(plan.report.created + plan.report.updated, 0);
    assert!(plan
        .report
        .problems
        .iter()
        .any(|p| p.detail.contains("not imported")));
}

#[test]
fn range_rows_leave_existing_ranges_untouched_while_leaves_apply() {
    let mut project = empty_project(GroupAddressStyle::ThreeLevel);
    let main_start = three("2/0/0").raw();
    let main_end = three("2/7/255").raw();
    let middle_start = three("2/1/0").raw();
    let middle_end = three("2/1/255").raw();
    let installation = &mut project.installations[0];
    installation
        .group_ranges
        .push(range(1, "Upstairs", main_start, main_end, None));
    installation
        .group_ranges
        .push(range(2, "Lamps", middle_start, middle_end, Some(1)));
    installation.group_ranges[0].children.push(GroupRangeId(2));
    let mut pump = entry_with_flags(1, three("2/1/1").raw(), "Pump", false, false);
    pump.range = Some(GroupRangeId(2));
    installation.group_addresses.push(pump);
    let original_ranges = installation.group_ranges.clone();
    assert_eq!(
        project.ids.next_group_address_id().unwrap(),
        GroupAddressId(1),
        "the seeded entry owns id 1"
    );

    let parsed = parse(&file(
        SHAPE1,
        &[
            &["Renamed floor", "2/-/-", "", "", "", "", ""],
            &["Renamed lamps", "2/1/-", "", "", "", "", ""],
            &["Pump 2", "2/1/1", "yes", "", "", "", ""],
            &["Fan", "2/1/2", "", "", "", "", ""],
        ],
    ));
    assert!(errors(&parsed).is_empty(), "{:?}", parsed.problems);
    assert_eq!(parsed.rows.len(), 2);
    let plan = plan_import(&project, &parsed);
    assert!(
        plan.report
            .problems
            .iter()
            .all(|p| p.severity == Severity::Warning),
        "{:?}",
        plan.report.problems
    );
    assert_eq!((plan.report.created, plan.report.updated), (1, 1));
    let command = plan.command.expect("leaf rows plan one batch");
    assert!(matches!(command, Command::Batch(_)));
    command.apply(&mut project).unwrap();

    let installation = &project.installations[0];
    assert_eq!(installation.group_ranges, original_ranges);
    let by_name = |name: &str| {
        installation
            .group_addresses
            .iter()
            .find(|e| e.name == name)
            .unwrap_or_else(|| panic!("missing {name}"))
    };
    assert_eq!(by_name("Pump 2").range, Some(GroupRangeId(2)));
    assert_eq!(by_name("Fan").range, Some(GroupRangeId(2)));
    assert_eq!(installation.group_addresses.len(), 2);
}

#[test]
fn line_numbers_survive_bom_crlf_and_multiline_names() {
    let text = format!(
        "\u{FEFF}{SHAPE1}\r\n\"Two\nlines\";1/1/1;;;;;\r\nPump;1/1/2;;;;;\r\nBad;1/1/x;;;;;\r\n"
    );
    let parsed = parse(&text);
    assert_eq!(parsed.rows.len(), 2);
    assert_eq!(parsed.rows[0].line, 2);
    assert_eq!(parsed.rows[0].name, "Two\nlines");
    assert_eq!(parsed.rows[1].line, 4);
    let errs = errors(&parsed);
    assert_eq!(errs.len(), 1);
    assert_eq!(errs[0].row, Some(5));
}

#[test]
fn names_keep_delimiters_quotes_and_non_ascii_exactly() {
    let text = format!("{SHAPE1}\n\"K\u{fc}che; \"\"gro\u{df}\"\" ,x\";1/1/1;;;;;\n");
    let parsed = parse(&text);
    assert_clean(&parsed);
    assert_eq!(parsed.rows[0].name, "K\u{fc}che; \"gro\u{df}\" ,x");
    let padded = parse(&file(SHAPE1, &[&["  Pump  ", "1/1/1", "", "", "", "", ""]]));
    assert_eq!(padded.rows[0].name, "  Pump  ");
}

#[test]
fn comma_separated_profile_files_parse() {
    let text = format!("{}\n\"Pump, main\",1/1/1,,,,,\n", SHAPE1.replace(';', ","));
    let parsed = parse(&text);
    assert_clean(&parsed);
    assert_eq!(parsed.separator, ',');
    assert_eq!(parsed.rows[0].name, "Pump, main");
}

#[test]
fn repeated_main_middle_sub_are_legal_only_in_the_complete_shape_four() {
    assert_clean(&parse(&file(SHAPE4, &[])));
    for header in [
        "Main;Middle;Sub;Main;Middle;Sub",
        "Main;Middle;Sub;Main;Middle;Sub;Central;Unfiltered;Description;DatapointType",
        "Main;Main;Middle;Sub;Middle;Sub;Central;Unfiltered;Description;DatapointType;Security",
        "Main;Middle;Sub;Sub;Middle;Main;Central;Unfiltered;Description;DatapointType;Security",
        "Group name;Main;Middle;Sub;Main;Central;Unfiltered;Description;DatapointType;Security",
        "Group name;Group name;Address;Central;Unfiltered;Description;DatapointType;Security",
    ] {
        assert_refused(&format!("{header}\n"));
    }
}

#[test]
fn native_duplicate_header_protection_remains() {
    let parsed = parse_group_addresses(
        "Address,Name,Address\n100,Kitchen,200\n",
        GroupAddressStyle::Free,
    );
    assert!(errors(&parsed)
        .iter()
        .any(|p| p.detail.contains("duplicate recognized column")));
}

#[test]
fn ambiguous_or_mixed_external_looking_headers_fail_closed() {
    for header in [
        "Group name;Address",
        "Name;Main;Middle;Sub",
        "Group name;Address;Action;NewAddress;Central;Unfiltered;Description;DatapointType;Security",
        "Address;Group name;Central;Unfiltered;Description;DatapointType;Security",
        "Group name;Address;Central;Unfiltered;DatapointType;Description;Security",
        "Group name;Address;Central;Unfiltered;Description;DatapointType",
        "Group name;Address;Central;Unfiltered;Description;DatapointType;Security;Extra",
    ] {
        assert_refused(&format!("{header}\n1;2;3;4;5;6;7;8;9\n"));
    }
}

#[test]
fn profiles_require_a_three_level_project() {
    for style in [GroupAddressStyle::TwoLevel, GroupAddressStyle::Free] {
        let parsed = parse_group_addresses(
            &file(SHAPE1, &[&["Pump", "1/1", "", "", "", "", ""]]),
            style,
        );
        assert!(parsed.rows.is_empty());
        let errs = errors(&parsed);
        assert_eq!(errs.len(), 1, "{:?}", parsed.problems);
        assert_eq!(errs[0].row, None);
        assert!(errs[0].detail.contains("ThreeLevel"), "{:?}", errs[0]);
    }
}

#[test]
fn malformed_composite_addresses_are_row_errors() {
    for address in [
        "", "1/2", "1/2/3/4", "1//3", "1/-/3", "-/-/-", "-/2/3", "1/2/-/-", "32/0/1", "1/8/1",
        "1/1/256", "0/0/0", "a/b/c", "1/-/x", "-1/2/3", "2/-", "1/2/3 x", "+1/2/3", "1/+2/3",
        "1/2/+3", "+1/-/-", "+1/2/-", "1/+2/-",
    ] {
        let parsed = parse(&file(SHAPE1, &[&["Pump", address, "", "", "", "", ""]]));
        assert!(parsed.rows.is_empty(), "{address:?}");
        let errs = errors(&parsed);
        assert_eq!(errs.len(), 1, "{address:?}: {:?}", parsed.problems);
        assert_eq!(errs[0].row, Some(2), "{address:?}");
    }
}

#[test]
fn composite_range_bounds_are_validated() {
    for address in ["32/-/-", "2/8/-", "99/-/-"] {
        assert_single_row_error(
            &file(SHAPE1, &[&["Upper floor", address, "", "", "", "", ""]]),
            "",
        );
    }
}

#[test]
fn malformed_split_addresses_are_row_errors() {
    let bad: &[[&str; 3]] = &[
        ["", "", ""],
        ["", "2", "3"],
        ["1", "", "3"],
        ["1", "-", ""],
        ["1", "-", "-"],
        ["1", "2", "-"],
        ["-", "", ""],
        ["x", "", ""],
        ["32", "", ""],
        ["1", "8", ""],
        ["1", "2", "256"],
        ["0", "0", "0"],
        ["1/2", "3", "4"],
        ["1", "", "-"],
        ["+1", "2", "3"],
        ["1", "+2", "3"],
        ["1", "2", "+3"],
        ["+1", "", ""],
        ["+1", "2", ""],
        ["1", "+2", ""],
    ];
    for [m, mi, s] in bad {
        let parsed = parse(&file(SHAPE2, &[&["Pump", m, mi, s, "", "", "", "", ""]]));
        assert!(parsed.rows.is_empty(), "{m:?} {mi:?} {s:?}");
        assert_eq!(
            errors(&parsed).len(),
            1,
            "{m:?} {mi:?} {s:?}: {:?}",
            parsed.problems
        );
        assert!(warnings(&parsed).is_empty(), "{m:?} {mi:?} {s:?}");
    }
}

#[test]
fn single_name_must_not_be_blank_even_for_ranges() {
    for address in ["1/1/1", "1/-/-", "1/1/-"] {
        for name in ["", "   "] {
            assert_single_row_error(
                &file(SHAPE1, &[&[name, address, "", "", "", "", ""]]),
                "name",
            );
        }
    }
}

#[test]
fn split_names_need_exactly_one_name_at_the_matching_depth() {
    let good = parse(&file(
        SHAPE3,
        &[
            &["Upper floor", "", "", "2/-/-", "", "", "", "", ""],
            &["", "Lighting", "", "2/1/-", "", "", "", "", ""],
            &["", "", "Pump", "2/1/1", "", "", "", "", ""],
        ],
    ));
    assert!(errors(&good).is_empty(), "{:?}", good.problems);
    assert_eq!(good.rows.len(), 1);
    assert_eq!(warnings(&good).len(), 2);

    let bad: &[[&str; 4]] = &[
        ["", "", "", "2/1/1"],
        ["Upper floor", "", "Pump", "2/1/1"],
        ["Upper floor", "Lighting", "Pump", "2/1/1"],
        ["Upper floor", "", "", "2/1/1"],
        ["", "Lighting", "", "2/1/1"],
        ["", "", "Pump", "2/-/-"],
        ["", "Lighting", "", "2/-/-"],
        ["", "", "Pump", "2/1/-"],
        ["Upper floor", "", "", "2/1/-"],
        ["  ", "", "", "2/-/-"],
    ];
    for [a, b, c, address] in bad {
        assert_single_row_error(
            &file(SHAPE3, &[&[a, b, c, address, "", "", "", "", ""]]),
            "name",
        );
    }
}

#[test]
fn split_triplet_names_keep_their_surrounding_spaces() {
    let parsed = parse(&file(
        SHAPE3,
        &[&["", "", "  Pump  ", "2/1/1", "", "", "", "", ""]],
    ));
    assert_clean(&parsed);
    assert_eq!(parsed.rows[0].name, "  Pump  ");

    let parsed = parse(&file(
        SHAPE4,
        &[&["", "", " Fan ", "2", "1", "2", "", "", "", "", ""]],
    ));
    assert_clean(&parsed);
    assert_eq!(parsed.rows[0].name, " Fan ");

    // Range rows are validated with the padded name but only leaves persist a name.
    let parsed = parse(&file(
        SHAPE3,
        &[
            &[" Upper floor ", "", "", "2/-/-", "", "", "", "", ""],
            &["", " Lighting ", "", "2/1/-", "", "", "", "", ""],
        ],
    ));
    assert!(errors(&parsed).is_empty(), "{:?}", parsed.problems);
    assert!(parsed.rows.is_empty());
    assert_eq!(warnings(&parsed).len(), 2);
}

#[test]
fn split_names_in_shape_four_follow_the_address_depth() {
    assert_single_row_error(
        &file(
            SHAPE4,
            &[&["Upper floor", "", "", "2", "1", "1", "", "", "", "", ""]],
        ),
        "name",
    );
    assert_single_row_error(
        &file(
            SHAPE4,
            &[&["", "", "Pump", "2", "", "", "", "", "", "", ""]],
        ),
        "name",
    );
}

#[test]
fn descriptions_are_refused_including_whitespace_only() {
    for description in ["Note", " ", "\t", "  x  "] {
        let leaf = file(SHAPE1, &[&["Pump", "1/1/1", "", "", description, "", ""]]);
        assert_single_row_error(&leaf, "Description");
        let range = file(SHAPE1, &[&["Floor", "1/-/-", "", "", description, "", ""]]);
        assert_single_row_error(&range, "Description");
    }
    assert_clean(&parse(&file(
        SHAPE1,
        &[&["Pump", "1/1/1", "", "", "", "", ""]],
    )));
}

#[test]
fn security_refuses_everything_but_empty_and_exact_auto() {
    for token in [
        "On", "Off", "true", "false", "Yes", "Secure", "auto", "AUTO", "Auto ", " Auto", " ", "1",
        "Unknown",
    ] {
        assert_single_row_error(
            &file(SHAPE1, &[&["Pump", "1/1/1", "", "", "", "", token]]),
            "Security",
        );
        assert_single_row_error(
            &file(SHAPE1, &[&["Floor", "1/-/-", "", "", "", "", token]]),
            "Security",
        );
    }
}

#[test]
fn security_auto_is_a_row_warning_on_leaves_and_ranges() {
    let parsed = parse(&file(
        SHAPE1,
        &[
            &["Pump", "1/1/1", "", "", "", "", "Auto"],
            &["Floor", "1/-/-", "", "", "", "", "Auto"],
        ],
    ));
    assert!(errors(&parsed).is_empty(), "{:?}", parsed.problems);
    assert_eq!(parsed.rows.len(), 1);
    let auto: Vec<_> = warnings(&parsed)
        .into_iter()
        .filter(|w| w.detail == AUTO_WARNING)
        .map(|w| w.row)
        .collect();
    assert_eq!(auto, vec![Some(2), Some(3)]);
}

#[test]
fn security_auto_warning_does_not_hide_other_errors() {
    let parsed = parse(&file(
        SHAPE1,
        &[&["Pump", "1/1/1", "maybe", "", "", "", "Auto"]],
    ));
    assert!(parsed.rows.is_empty());
    assert_eq!(errors(&parsed).len(), 1);
    assert_eq!(warnings(&parsed).len(), 1);
}

#[test]
fn leaf_flags_use_the_native_spellings_and_reject_others() {
    for (cell, expected) in [("TRUE", Some(true)), ("0", Some(false)), ("", None)] {
        let parsed = parse(&file(SHAPE1, &[&["Pump", "1/1/1", cell, cell, "", "", ""]]));
        assert_clean(&parsed);
        assert_eq!(parsed.rows[0].central, expected);
        assert_eq!(parsed.rows[0].unfiltered, expected);
    }
    assert_single_row_error(
        &file(SHAPE1, &[&["Pump", "1/1/1", "maybe", "", "", "", ""]]),
        "Central",
    );
    assert_single_row_error(
        &file(SHAPE1, &[&["Pump", "1/1/1", "", "on", "", "", ""]]),
        "Unfiltered",
    );
}

#[test]
fn any_nonblank_flag_on_a_range_row_is_an_error() {
    for cell in ["false", "true", "0", "maybe"] {
        for (central, unfiltered) in [(cell, ""), ("", cell)] {
            assert_single_row_error(
                &file(
                    SHAPE1,
                    &[&["Floor", "1/-/-", central, unfiltered, "", "", ""]],
                ),
                "range",
            );
        }
    }
}

#[test]
fn leaf_dpt_tokens_are_normalized_and_blank_is_absent() {
    for (cell, expected) in [
        ("DPST-1-1", Some("DPST-1-1")),
        ("DPT-01", Some("DPT-1")),
        ("DPST-009-001", Some("DPST-9-1")),
        (" DPT-5 ", Some("DPT-5")),
        ("", None),
        ("  ", None),
    ] {
        let parsed = parse(&file(SHAPE1, &[&["Pump", "1/1/1", "", "", "", cell, ""]]));
        assert_clean(&parsed);
        assert_eq!(
            parsed.rows[0].datapoint_type.as_deref(),
            expected,
            "{cell:?}"
        );
    }
}

#[test]
fn malformed_leaf_dpt_tokens_are_errors() {
    for cell in [
        "1.001",
        "DPT",
        "DPT-1-1",
        "DPST-1",
        "dpt-1",
        "DPT-x",
        "DPST-1-1 DPST-1-2",
        "DPT-+1",
        "DPST-+1-1",
        "DPST-1-+1",
        "DPT--1",
        "DPST-1--1",
        "DPT-",
        "DPST--",
    ] {
        assert_single_row_error(
            &file(SHAPE1, &[&["Pump", "1/1/1", "", "", "", cell, ""]]),
            "DatapointType",
        );
    }
}

#[test]
fn any_nonblank_dpt_on_a_range_row_is_an_error() {
    assert_single_row_error(
        &file(SHAPE1, &[&["Floor", "1/-/-", "", "", "", "DPST-1-1", ""]]),
        "DatapointType",
    );
    assert_single_row_error(
        &file(SHAPE1, &[&["Floor", "1/-/-", "", "", "", "junk", ""]]),
        "DatapointType",
    );
}

#[test]
fn a_dpt_matching_the_derived_type_is_accepted_and_never_applied() {
    let mut project = empty_project(GroupAddressStyle::ThreeLevel);
    let address = three("1/1/1");
    project.installations[0]
        .group_addresses
        .push(entry_with_flags(1, address.raw(), "Pump", false, false));
    project.devices.insert_com_object(linked_com_object(
        1,
        1,
        1,
        Some(DptRef::parse("DPST-1-1").unwrap()),
    ));

    let same = parse(&file(
        SHAPE1,
        &[&["Pump", "1/1/1", "", "", "", "DPST-1-1", ""]],
    ));
    let plan = plan_import(&project, &same);
    assert!(
        plan.report.problems.is_empty(),
        "{:?}",
        plan.report.problems
    );
    assert_eq!(plan.report.unchanged, 1);
    assert!(plan.command.is_none());

    let other = parse(&file(
        SHAPE1,
        &[&["Pump", "1/1/1", "", "", "", "DPST-5-1", ""]],
    ));
    let plan = plan_import(&project, &other);
    assert!(plan.command.is_none());
    assert!(plan
        .report
        .problems
        .iter()
        .any(|p| p.severity == Severity::Error && p.detail.contains("DatapointType")));

    let blank = parse(&file(SHAPE1, &[&["Pump", "1/1/1", "", "", "", "", ""]]));
    assert!(plan_import(&project, &blank).report.problems.is_empty());
}

#[test]
fn a_new_address_with_a_dpt_is_refused() {
    let project = empty_project(GroupAddressStyle::ThreeLevel);
    let parsed = parse(&file(
        SHAPE1,
        &[&["Pump", "1/1/1", "", "", "", "DPST-1-1", ""]],
    ));
    let plan = plan_import(&project, &parsed);
    assert!(plan.command.is_none());
    assert!(plan
        .report
        .problems
        .iter()
        .any(|p| p.severity == Severity::Error && p.detail.contains("DatapointType")));
}

#[test]
fn a_new_leaf_without_a_dpt_creates_with_native_defaults() {
    let project = empty_project(GroupAddressStyle::ThreeLevel);
    let parsed = parse(&file(SHAPE1, &[&["Pump", "1/1/1", "", "", "", "", ""]]));
    let plan = plan_import(&project, &parsed);
    assert_eq!(plan.report.created, 1);
    let Some(Command::Batch(commands)) = plan.command else {
        panic!("expected a batch");
    };
    let Command::CreateGroupAddress { entry, .. } = &commands[0] else {
        panic!("expected a create, got {:?}", commands[0]);
    };
    assert_eq!((entry.central, entry.unfiltered), (false, false));
    assert_eq!(entry.name, "Pump");
    assert_eq!(entry.address, three("1/1/1"));
}

#[test]
fn blank_flags_leave_an_existing_entry_unchanged_and_values_update() {
    let mut project = empty_project(GroupAddressStyle::ThreeLevel);
    project.installations[0]
        .group_addresses
        .push(entry_with_flags(
            1,
            three("1/1/1").raw(),
            "Pump",
            true,
            true,
        ));
    let blank = parse(&file(SHAPE1, &[&["Pump", "1/1/1", "", "", "", "", ""]]));
    let plan = plan_import(&project, &blank);
    assert_eq!(plan.report.unchanged, 1);

    let renamed = parse(&file(SHAPE1, &[&["Pump 2", "1/1/1", "no", "", "", "", ""]]));
    let plan = plan_import(&project, &renamed);
    assert_eq!(plan.report.updated, 1);
    let Some(Command::Batch(commands)) = plan.command else {
        panic!("expected a batch");
    };
    assert!(matches!(
        &commands[0],
        Command::UpdateGroupAddress { name, central: false, unfiltered: true, .. } if name == "Pump 2"
    ));
}

#[test]
fn any_error_suppresses_the_whole_command() {
    let project = empty_project(GroupAddressStyle::ThreeLevel);
    let parsed = parse(&file(
        SHAPE1,
        &[
            &["Pump", "1/1/1", "", "", "", "", ""],
            &["Floor", "1/-/-", "true", "", "", "", ""],
        ],
    ));
    let plan = plan_import(&project, &parsed);
    assert!(plan.command.is_none());
    assert!(plan
        .report
        .problems
        .iter()
        .any(|p| p.severity == Severity::Error));

    let duplicate = parse(&file(
        SHAPE1,
        &[
            &["Pump", "1/1/1", "", "", "", "", ""],
            &["Pump again", "1/1/1", "", "", "", "", ""],
        ],
    ));
    assert!(plan_import(&project, &duplicate).command.is_none());
}

#[test]
fn native_required_columns_keep_precedence_over_unknown_profile_named_columns() {
    for (extra, value) in [
        ("Main", "Workshop"),
        ("Middle", "Upper floor"),
        ("Sub", "Pump"),
        ("Group name", "Fan"),
    ] {
        let parsed = parse(&format!(
            "Address;Name;{extra}\n1/1/1;Native name;{value}\n"
        ));
        assert_clean(&parsed);
        assert_eq!(parsed.rows.len(), 1);
        assert_eq!(parsed.rows[0].name, "Native name");
        assert_eq!(parsed.ignored_columns.len(), 1);
        assert_eq!(parsed.ignored_columns[0].name, extra);
        assert_eq!(
            parsed.ignored_columns[0].reason,
            IgnoredColumnReason::Unknown
        );
    }
}

#[test]
fn native_files_keep_their_unknown_column_behavior() {
    let parsed = parse_group_addresses(
        "Address,Name,Description,Security\n100,Kitchen,x,y\n",
        GroupAddressStyle::Free,
    );
    assert!(parsed.problems.is_empty(), "{:?}", parsed.problems);
    assert_eq!(parsed.rows.len(), 1);
    assert_eq!(parsed.ignored_columns.len(), 2);
    assert!(parsed
        .ignored_columns
        .iter()
        .all(|c| c.reason == IgnoredColumnReason::Unknown));
}

#[test]
fn native_action_files_still_work_in_a_three_level_project() {
    let parsed = parse("Address;Name;Action;NewAddress\n1/1/1;Pump;upsert;\n");
    assert_clean(&parsed);
    assert_eq!(parsed.rows.len(), 1);
    assert_eq!(TAIL.split(';').count(), 5);
}
