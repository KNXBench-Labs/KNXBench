//! Turns a [`ParsedCsv`] into a single [`Command`] plus a report, without
//! ever mutating the project.
//!
//! Design: `docs/superpowers/specs/2026-09-10-csv-group-address-exchange-design.md`
//! §4 (import semantics).

use std::collections::HashMap;

use knx_core::{
    Command, GroupAddress, GroupAddressEntry, GroupRange, GroupRangeId, Project, SourceRef,
};

use crate::read::{CsvProblem, CsvRow, IgnoredColumn, ParsedCsv, Severity};

/// The outcome of planning a CSV import: the single command that would
/// apply every create/update in one undo step, and the report describing
/// what was read and what would happen.
///
/// `command` is `None` when there is nothing to apply (every row is
/// `unchanged`) or when at least one row-level problem was found anywhere
/// in the file (design §4's all-or-nothing rule) — `report.problems` says
/// which, and every row is still classified and counted so a caller can
/// show what *would* have happened once the file is fixed.
#[derive(Debug, Clone, PartialEq)]
pub struct ImportPlan {
    pub command: Option<Command>,
    pub report: CsvImportReport,
}

/// Deliberately mirrors `knx_etsproj::ImportReport`'s shape (counts,
/// `CsvProblem`s with a location and severity) so callers can treat a CSV
/// import report the same way they already treat a `.knxproj` one (design
/// §6).
#[derive(Debug, Clone, PartialEq)]
pub struct CsvImportReport {
    pub separator: char,
    pub rows_read: usize,
    pub created: usize,
    pub updated: usize,
    pub unchanged: usize,
    pub ignored_columns: Vec<IgnoredColumn>,
    pub problems: Vec<CsvProblem>,
}

/// Plans applying `parsed` against `project`.
///
/// Each row is matched to an existing entry by its parsed **address**, never
/// by name (design §4). New ids are allocated in row order from a local
/// clone of `project.ids` — `plan_import` never mutates `project` itself, so
/// the ids it hands out are only real once a caller actually applies the
/// returned `Command`.
pub fn plan_import(project: &Project, parsed: &ParsedCsv) -> ImportPlan {
    let installation = project.installations.first();
    let existing_ranges: &[GroupRange] = installation
        .map(|i| i.group_ranges.as_slice())
        .unwrap_or(&[]);
    let existing_addresses: &[GroupAddressEntry] = installation
        .map(|i| i.group_addresses.as_slice())
        .unwrap_or(&[]);

    let mut problems = parsed.problems.clone();
    let mut commands = Vec::new();
    let mut created = 0usize;
    let mut updated = 0usize;
    let mut unchanged = 0usize;

    let mut ids = project.ids.clone();
    // Tracks the first line each address was seen on, so a repeat can be
    // reported as "also on line N" and excluded from the outcome counts —
    // reporting *an* outcome for a duplicated address would just be a
    // coin flip between its rows.
    let mut first_seen: HashMap<GroupAddress, usize> = HashMap::new();

    for row in &parsed.rows {
        if let Some(&first_line) = first_seen.get(&row.address) {
            problems.push(CsvProblem {
                row: Some(row.line),
                severity: Severity::Error,
                detail: format!(
                    "address {} is a duplicate of the one on line {first_line}",
                    row.address.format(project.info.group_address_style)
                ),
            });
            continue;
        }
        first_seen.insert(row.address, row.line);

        match existing_addresses.iter().find(|e| e.address == row.address) {
            None => {
                plan_create(row, existing_ranges, &mut ids, &mut commands, &mut problems);
                created += 1;
            }
            Some(existing) => {
                if plan_update(row, existing, &mut commands) {
                    updated += 1;
                } else {
                    unchanged += 1;
                }
            }
        }
    }

    let has_errors = problems.iter().any(|p| p.severity == Severity::Error);
    let command = if has_errors || commands.is_empty() {
        None
    } else {
        Some(Command::Batch(commands))
    };

    ImportPlan {
        command,
        report: CsvImportReport {
            separator: parsed.separator,
            rows_read: parsed.rows.len(),
            created,
            updated,
            unchanged,
            ignored_columns: parsed.ignored_columns.clone(),
            problems,
        },
    }
}

/// Plans a `Command::CreateGroupAddress` for `row`, whose address matched no
/// existing entry. Always succeeds — there is no row-level condition left
/// that can fail here, since the reader already rejected anything that
/// would have.
fn plan_create(
    row: &CsvRow,
    ranges: &[GroupRange],
    ids: &mut knx_core::IdAllocators,
    commands: &mut Vec<Command>,
    problems: &mut Vec<CsvProblem>,
) {
    let range = innermost_containing_range(ranges, row.address);
    if range.is_none() {
        problems.push(CsvProblem {
            row: Some(row.line),
            severity: Severity::Warning,
            detail: "no existing group range contains this address; created without one"
                .to_string(),
        });
    }

    let id = ids.next_group_address_id();
    let entry = GroupAddressEntry {
        id,
        // Matches `domain.rs`'s `create_group_address_impl`: a UI/CSV
        // created entry has no ETS origin to preserve, so it gets a
        // synthetic but stable source id instead of an empty,
        // export-breaking one.
        source: SourceRef {
            path: format!("KB-GA-{}", id.0),
            ets_id: format!("KB-GA-{}", id.0),
        },
        name: row.name.clone(),
        address: row.address,
        // Design §4/§3: an empty cell means "false" on create.
        central: row.central.unwrap_or(false),
        unfiltered: row.unfiltered.unwrap_or(false),
        range,
    };
    commands.push(Command::CreateGroupAddress { entry });
}

/// Compares `row` against `existing` and, if anything applied differs,
/// plans a `Command::UpdateGroupAddress`. Returns whether it did (`true` =
/// updated, `false` = unchanged). An empty `central`/`unfiltered` cell means
/// "leave unchanged" here (design §3), unlike `plan_create`'s "false".
fn plan_update(row: &CsvRow, existing: &GroupAddressEntry, commands: &mut Vec<Command>) -> bool {
    let central = row.central.unwrap_or(existing.central);
    let unfiltered = row.unfiltered.unwrap_or(existing.unfiltered);

    if existing.name == row.name && existing.central == central && existing.unfiltered == unfiltered
    {
        return false;
    }

    commands.push(Command::UpdateGroupAddress {
        id: existing.id,
        name: row.name.clone(),
        central,
        unfiltered,
    });
    true
}

/// The `GroupRange` with the narrowest `start..=end` that contains
/// `address` — a middle range wins over the main range enclosing it (design
/// §4).
fn innermost_containing_range(
    ranges: &[GroupRange],
    address: GroupAddress,
) -> Option<GroupRangeId> {
    ranges
        .iter()
        .filter(|r| r.contains(address))
        .min_by_key(|r| r.end.raw() - r.start.raw())
        .map(|r| r.id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::read::parse_group_addresses;
    use crate::testutil::{empty_project, entry, range};
    use knx_core::{GroupAddressId, GroupAddressStyle};

    fn parsed(text: &str) -> ParsedCsv {
        parse_group_addresses(text, GroupAddressStyle::Free)
    }

    #[test]
    fn an_unseen_address_plans_a_create_with_a_synthetic_stable_source() {
        let project = empty_project(GroupAddressStyle::Free);
        let plan = plan_import(&project, &parsed("Address,Name\n100,Kitchen Light\n"));

        assert_eq!(plan.report.created, 1);
        assert_eq!(plan.report.updated, 0);
        assert_eq!(plan.report.unchanged, 0);
        // No group range exists in this fixture project at all, so the
        // "created without one" warning is expected — this test is about
        // the `source` shape, not range placement (see the dedicated range
        // tests below).
        assert!(
            plan.report
                .problems
                .iter()
                .all(|p| p.severity == Severity::Warning),
            "{:?}",
            plan.report.problems
        );

        let Some(Command::Batch(cmds)) = plan.command else {
            panic!("expected a Batch command, got {:?}", plan.command);
        };
        assert_eq!(cmds.len(), 1);
        let Command::CreateGroupAddress { entry } = &cmds[0] else {
            panic!("expected CreateGroupAddress, got {:?}", cmds[0]);
        };
        assert_eq!(entry.name, "Kitchen Light");
        assert_eq!(entry.address, GroupAddress::from_raw(100));
        assert_eq!(entry.source.path, format!("KB-GA-{}", entry.id.0));
        assert_eq!(entry.source.ets_id, format!("KB-GA-{}", entry.id.0));
    }

    #[test]
    fn a_differing_name_plans_an_update() {
        let mut project = empty_project(GroupAddressStyle::Free);
        project.installations[0]
            .group_addresses
            .push(entry(1, 100, "Old Name"));

        let plan = plan_import(&project, &parsed("Address,Name\n100,New Name\n"));

        assert_eq!(plan.report.updated, 1);
        assert_eq!(plan.report.created, 0);
        assert_eq!(plan.report.unchanged, 0);

        let Some(Command::Batch(cmds)) = plan.command else {
            panic!("expected a Batch command, got {:?}", plan.command);
        };
        assert_eq!(
            cmds[0],
            Command::UpdateGroupAddress {
                id: GroupAddressId(1),
                name: "New Name".to_string(),
                central: false,
                unfiltered: false,
            }
        );
    }

    #[test]
    fn an_identical_row_counts_as_unchanged_and_plans_nothing() {
        let mut project = empty_project(GroupAddressStyle::Free);
        project.installations[0]
            .group_addresses
            .push(entry(1, 100, "Kitchen Light"));

        let plan = plan_import(&project, &parsed("Address,Name\n100,Kitchen Light\n"));

        assert_eq!(plan.report.unchanged, 1);
        assert_eq!(plan.report.created, 0);
        assert_eq!(plan.report.updated, 0);
        assert_eq!(plan.command, None);
    }

    #[test]
    fn a_file_where_every_row_is_unchanged_yields_no_command() {
        let mut project = empty_project(GroupAddressStyle::Free);
        project.installations[0]
            .group_addresses
            .push(entry(1, 100, "A"));
        project.installations[0]
            .group_addresses
            .push(entry(2, 200, "B"));

        let plan = plan_import(&project, &parsed("Address,Name\n100,A\n200,B\n"));

        assert_eq!(plan.report.unchanged, 2);
        assert_eq!(plan.command, None);
        assert!(
            plan.report.problems.is_empty(),
            "{:?}",
            plan.report.problems
        );
    }

    #[test]
    fn a_duplicate_address_inside_one_file_is_an_error() {
        let project = empty_project(GroupAddressStyle::Free);
        let plan = plan_import(&project, &parsed("Address,Name\n100,First\n100,Second\n"));

        assert!(plan
            .report
            .problems
            .iter()
            .any(|p| p.severity == Severity::Error && p.detail.contains("duplicate")));
        assert_eq!(plan.command, None);
    }

    #[test]
    fn any_row_level_error_yields_no_command_with_every_other_row_still_reported() {
        let mut project = empty_project(GroupAddressStyle::Free);
        project.installations[0]
            .group_addresses
            .push(entry(1, 300, "Unchanged"));

        // Row 1 (line 2) creates fine; row 2 (line 3) duplicates row 3
        // (line 4)'s address; row 4 (line 5) is unchanged. Only the
        // duplicate pair should produce an error, but the whole plan must
        // still refuse to commit anything.
        let text = "Address,Name\n100,New\n200,Dup A\n200,Dup B\n300,Unchanged\n";
        let plan = plan_import(&project, &parsed(text));

        assert_eq!(plan.command, None);
        // Row 1 (create) and row 2's *first* occurrence (also a create,
        // since it is not itself the repeat) are still classified; only
        // row 3, the repeat, is excluded and turned into an error.
        assert_eq!(plan.report.created, 2, "{:?}", plan.report);
        assert_eq!(plan.report.unchanged, 1, "{:?}", plan.report);
        assert_eq!(plan.report.updated, 0, "{:?}", plan.report);
        let error_rows: Vec<Option<usize>> = plan
            .report
            .problems
            .iter()
            .filter(|p| p.severity == Severity::Error)
            .map(|p| p.row)
            .collect();
        assert_eq!(error_rows, vec![Some(4)], "{:?}", plan.report.problems);
    }

    #[test]
    fn a_new_address_lands_in_the_innermost_containing_range() {
        let mut project = empty_project(GroupAddressStyle::Free);
        project.installations[0]
            .group_ranges
            .push(range(1, "Main", 0, 500, None));
        project.installations[0]
            .group_ranges
            .push(range(2, "Middle", 0, 100, Some(1)));

        let plan = plan_import(&project, &parsed("Address,Name\n50,Kitchen Light\n"));

        let Some(Command::Batch(cmds)) = plan.command else {
            panic!("expected a Batch command, got {:?}", plan.command);
        };
        let Command::CreateGroupAddress { entry } = &cmds[0] else {
            panic!("expected CreateGroupAddress, got {:?}", cmds[0]);
        };
        assert_eq!(entry.range, Some(GroupRangeId(2)));
        assert!(
            plan.report.problems.is_empty(),
            "{:?}",
            plan.report.problems
        );
    }

    #[test]
    fn range_none_with_a_warning_when_nothing_contains_the_address() {
        let project = empty_project(GroupAddressStyle::Free);
        let plan = plan_import(&project, &parsed("Address,Name\n50,Kitchen Light\n"));

        let Some(Command::Batch(cmds)) = &plan.command else {
            panic!("expected a Batch command, got {:?}", plan.command);
        };
        let Command::CreateGroupAddress { entry } = &cmds[0] else {
            panic!("expected CreateGroupAddress, got {:?}", cmds[0]);
        };
        assert_eq!(entry.range, None);
        assert_eq!(plan.report.problems.len(), 1);
        assert_eq!(plan.report.problems[0].severity, Severity::Warning);
    }

    #[test]
    fn counts_add_up_to_rows_read_when_the_file_is_clean() {
        let mut project = empty_project(GroupAddressStyle::Free);
        project.installations[0]
            .group_addresses
            .push(entry(1, 100, "Same"));
        project.installations[0]
            .group_addresses
            .push(entry(2, 200, "Old"));

        let text = "Address,Name\n100,Same\n200,New\n300,Brand New\n";
        let plan = plan_import(&project, &parsed(text));

        assert_eq!(plan.report.rows_read, 3);
        assert_eq!(
            plan.report.created + plan.report.updated + plan.report.unchanged,
            plan.report.rows_read
        );
        assert_eq!(plan.report.created, 1);
        assert_eq!(plan.report.updated, 1);
        assert_eq!(plan.report.unchanged, 1);
    }

    #[test]
    fn export_only_columns_are_carried_into_the_report_as_ignored() {
        let project = empty_project(GroupAddressStyle::Free);
        let text = "Address,Name,DatapointType\n100,Kitchen Light,DPST-1-1\n";
        let plan = plan_import(&project, &parsed(text));

        assert_eq!(plan.report.ignored_columns.len(), 1);
        assert_eq!(plan.report.ignored_columns[0].name, "DatapointType");
    }
}
