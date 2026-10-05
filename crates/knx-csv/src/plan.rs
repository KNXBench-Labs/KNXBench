//! Turns a [`ParsedCsv`] into a single [`Command`] plus a report, without
//! ever mutating the project.
//!
//! Design: `docs/superpowers/specs/2026-09-10-csv-group-address-exchange-design.md`
//! §4 (import semantics).

use std::collections::HashMap;

use knx_core::{
    ComObjectInstanceId, Command, Direction, GroupAddress, GroupAddressEntry, GroupAddressId,
    GroupRange, GroupRangeId, InstallationId, Project, SourceRef,
};

use crate::read::{CsvAction, CsvProblem, CsvRow, IgnoredColumn, ParsedCsv, Severity};
use crate::write::{containing_range_names, derive_dpt};

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
    pub readdressed: usize,
    pub deleted: usize,
    pub unchanged: usize,
    pub destructive_changes: Vec<CsvDestructiveChange>,
    pub ignored_columns: Vec<IgnoredColumn>,
    pub problems: Vec<CsvProblem>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CsvDestructiveAction {
    Readdress,
    Delete,
}

/// One explicitly requested destructive mutation and the stable-id links it
/// would affect. Blocked operations remain here for an honest preview.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CsvDestructiveChange {
    pub row: usize,
    pub action: CsvDestructiveAction,
    pub id: GroupAddressId,
    pub source_address: GroupAddress,
    pub target_address: Option<GroupAddress>,
    pub affected_links: Vec<CsvAffectedLink>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CsvAffectedLink {
    pub com_object: ComObjectInstanceId,
    pub direction: Direction,
}

/// Plans applying `parsed` against `project`.
///
/// Each row is matched to an existing entry by its parsed **address**, never
/// by name (design §4). New ids are allocated in row order from a local
/// clone of `project.ids` — `plan_import` never mutates `project` itself, so
/// the ids it hands out are only real once a caller actually applies the
/// returned `Command`.
pub fn plan_import(project: &Project, parsed: &ParsedCsv) -> ImportPlan {
    plan_import_into(project, parsed, None)
}

/// [`plan_import`] against installation `target_installation` (MODEL-01); `None` keeps
/// the first installation. Rows are matched, ranges looked up and new
/// addresses created only in that installation — the same address may
/// exist in another one, which is a separate infrastructure (ADR-0038). An
/// unknown installation is a file-level error: no command at all.
pub fn plan_import_into(
    project: &Project,
    parsed: &ParsedCsv,
    target_installation: Option<InstallationId>,
) -> ImportPlan {
    let installation = match target_installation {
        None => project.installations.first(),
        Some(id) => match project.installations.iter().find(|i| i.id == id) {
            Some(installation) => Some(installation),
            None => return unknown_installation_plan(parsed, id),
        },
    };
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
    let mut readdressed = 0usize;
    let mut deleted = 0usize;
    let mut unchanged = 0usize;
    let mut destructive_changes = Vec::new();

    let mut ids = project.ids.clone();
    // Tracks the first line each address was seen on, so a repeat can be
    // reported as "also on line N" and excluded from the outcome counts —
    // reporting *an* outcome for a duplicated address would just be a
    // coin flip between its rows.
    let mut first_seen: HashMap<GroupAddress, usize> = HashMap::new();
    let mut first_target: HashMap<GroupAddress, usize> = HashMap::new();

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

        let target = match row.action {
            CsvAction::Upsert => Some(row.address),
            CsvAction::Readdress => row.new_address,
            CsvAction::Delete => None,
        };
        if let Some(target) = target {
            if let Some(&first_line) = first_target.get(&target) {
                problems.push(CsvProblem {
                    row: Some(row.line),
                    severity: Severity::Error,
                    detail: format!(
                        "target address {} is already claimed by line {first_line}",
                        target.format(project.info.group_address_style)
                    ),
                });
                continue;
            }
            first_target.insert(target, row.line);
        }

        let existing = existing_addresses
            .iter()
            .find(|entry| entry.address == row.address);
        validate_read_only_cells(project, installation, row, existing, &mut problems);

        match (row.action, existing) {
            (CsvAction::Upsert, None) => {
                plan_create(
                    row,
                    existing_ranges,
                    target_installation,
                    &mut ids,
                    &mut commands,
                    &mut problems,
                );
                created += 1;
            }
            (CsvAction::Upsert, Some(existing)) => {
                if plan_update(row, existing, &mut commands) {
                    updated += 1;
                } else {
                    unchanged += 1;
                }
            }
            (CsvAction::Readdress, None) | (CsvAction::Delete, None) => {
                problems.push(CsvProblem {
                    row: Some(row.line),
                    severity: Severity::Error,
                    detail: "destructive action requires an existing source address".to_string(),
                });
            }
            (CsvAction::Readdress, Some(existing)) => {
                let target = row
                    .new_address
                    .expect("reader requires NewAddress for readdress");
                destructive_changes.push(CsvDestructiveChange {
                    row: row.line,
                    action: CsvDestructiveAction::Readdress,
                    id: existing.id,
                    source_address: row.address,
                    target_address: Some(target),
                    affected_links: affected_links(project, existing.id),
                });
                readdressed += 1;
                if existing_addresses
                    .iter()
                    .any(|candidate| candidate.address == target && candidate.id != existing.id)
                {
                    problems.push(CsvProblem {
                        row: Some(row.line),
                        severity: Severity::Error,
                        detail: format!(
                            "NewAddress {} already exists",
                            target.format(project.info.group_address_style)
                        ),
                    });
                    continue;
                }
                let range = innermost_containing_range(existing_ranges, target);
                if range.is_none() {
                    problems.push(CsvProblem {
                        row: Some(row.line),
                        severity: Severity::Warning,
                        detail: "no existing group range contains NewAddress; moved without one"
                            .to_string(),
                    });
                }
                commands.push(Command::ReaddressGroupAddress {
                    id: existing.id,
                    address: target,
                    range,
                });
                plan_update(row, existing, &mut commands);
            }
            (CsvAction::Delete, Some(existing)) => {
                let affected_links = affected_links(project, existing.id);
                destructive_changes.push(CsvDestructiveChange {
                    row: row.line,
                    action: CsvDestructiveAction::Delete,
                    id: existing.id,
                    source_address: row.address,
                    target_address: None,
                    affected_links: affected_links.clone(),
                });
                deleted += 1;
                if affected_links.is_empty() {
                    commands.push(Command::DeleteGroupAddress { id: existing.id });
                } else {
                    problems.push(CsvProblem {
                        row: Some(row.line),
                        severity: Severity::Error,
                        detail: format!(
                            "cannot delete: {} communication-object link(s) still reference this address",
                            affected_links.len()
                        ),
                    });
                }
            }
        }
    }

    if created > 0 {
        // Never-rewinding (ADR-0039 Decision 2): applied after a concurrent
        // edit it cannot lower that edit's counters, and undo leaves the
        // high-water mark in place so a stable `KB-GA-n` is never reissued.
        commands.push(Command::ReserveIds { through: ids });
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
            readdressed,
            deleted,
            unchanged,
            destructive_changes,
            ignored_columns: parsed.ignored_columns.clone(),
            problems,
        },
    }
}

fn unknown_installation_plan(parsed: &ParsedCsv, id: InstallationId) -> ImportPlan {
    let mut problems = parsed.problems.clone();
    problems.push(CsvProblem {
        row: None,
        severity: Severity::Error,
        detail: format!("installation {id} does not exist"),
    });
    ImportPlan {
        command: None,
        report: CsvImportReport {
            separator: parsed.separator,
            rows_read: parsed.rows.len(),
            created: 0,
            updated: 0,
            readdressed: 0,
            deleted: 0,
            unchanged: 0,
            destructive_changes: Vec::new(),
            ignored_columns: parsed.ignored_columns.clone(),
            problems,
        },
    }
}

fn affected_links(project: &Project, id: GroupAddressId) -> Vec<CsvAffectedLink> {
    project
        .devices
        .com_objects()
        .flat_map(|object| {
            object
                .links
                .iter()
                .filter(move |link| link.ga == id)
                .map(move |link| CsvAffectedLink {
                    com_object: object.id,
                    direction: link.direction,
                })
        })
        .collect()
}

fn validate_read_only_cells(
    project: &Project,
    installation: Option<&knx_core::Installation>,
    row: &CsvRow,
    existing: Option<&GroupAddressEntry>,
    problems: &mut Vec<CsvProblem>,
) {
    let expected_dpt = existing
        .and_then(|entry| derive_dpt(project, entry.id).0)
        .map(|dpt| dpt.to_string())
        .unwrap_or_default();
    let (expected_main, expected_middle) = installation
        .map(|installation| containing_range_names(installation, row.address))
        .unwrap_or_default();

    validate_read_only_cell(
        row,
        "DatapointType",
        row.datapoint_type.as_deref(),
        &expected_dpt,
        problems,
    );
    validate_read_only_cell(
        row,
        "MainGroup",
        row.main_group.as_deref(),
        expected_main.as_deref().unwrap_or(""),
        problems,
    );
    validate_read_only_cell(
        row,
        "MiddleGroup",
        row.middle_group.as_deref(),
        expected_middle.as_deref().unwrap_or(""),
        problems,
    );
}

fn validate_read_only_cell(
    row: &CsvRow,
    column: &str,
    supplied: Option<&str>,
    expected: &str,
    problems: &mut Vec<CsvProblem>,
) {
    let Some(supplied) = supplied else {
        return;
    };
    if supplied == expected {
        return;
    }
    problems.push(CsvProblem {
        row: Some(row.line),
        severity: Severity::Error,
        detail: format!(
            "column \"{column}\" is read-only: file has {supplied:?}, project derives {expected:?}"
        ),
    });
}

/// Plans a `Command::CreateGroupAddress` for `row`, whose address matched no
/// existing entry. An exhausted ID space is a row-level error, so the
/// all-or-nothing planner returns no command for the entire file.
fn plan_create(
    row: &CsvRow,
    ranges: &[GroupRange],
    installation: Option<InstallationId>,
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

    let id = match ids.next_group_address_id() {
        Ok(id) => id,
        Err(error) => {
            problems.push(CsvProblem {
                row: Some(row.line),
                severity: Severity::Error,
                detail: error.to_string(),
            });
            return;
        }
    };
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
        declared_dpt: Default::default(),
    };
    commands.push(Command::CreateGroupAddress {
        entry,
        installation,
    });
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
    use crate::testutil::{empty_project, entry, entry_with_flags, linked_com_object, range};
    use knx_core::{DptRef, GroupAddressId, GroupAddressStyle, GroupLink};

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
        assert_eq!(cmds.len(), 2);
        let Command::CreateGroupAddress { entry, .. } = &cmds[0] else {
            panic!("expected CreateGroupAddress, got {:?}", cmds[0]);
        };
        assert_eq!(entry.name, "Kitchen Light");
        assert_eq!(entry.address, GroupAddress::from_raw(100));
        assert_eq!(entry.source.path, format!("KB-GA-{}", entry.id.0));
        assert_eq!(entry.source.ets_id, format!("KB-GA-{}", entry.id.0));
        assert!(matches!(cmds[1], Command::ReserveIds { .. }));
    }

    #[test]
    fn successive_create_imports_advance_ids_and_never_reuse_a_stable_id() {
        let mut project = empty_project(GroupAddressStyle::Free);
        let first = plan_import(&project, &parsed("Address,Name\n100,A\n"));
        first.command.unwrap().apply(&mut project).unwrap();
        let second = plan_import(&project, &parsed("Address,Name\n200,B\n"));
        second.command.unwrap().apply(&mut project).unwrap();

        let ids: Vec<_> = project.installations[0]
            .group_addresses
            .iter()
            .map(|entry| entry.id)
            .collect();
        assert_eq!(ids, vec![GroupAddressId(1), GroupAddressId(2)]);
        assert_eq!(project.ids.peek_group_address(), 2);
    }

    /// ADR-0039 Decision 2: the plan reserves ids with the never-rewinding
    /// `ReserveIds`, so undo restores the user's content but not the
    /// counter, and a stable `KB-GA-n` never names two different addresses
    /// in one project's history.
    #[test]
    fn undoing_an_import_keeps_the_high_water_mark_so_a_stable_id_is_never_reissued() {
        let mut project = empty_project(GroupAddressStyle::Free);
        let mut stack = knx_core::CommandStack::new();
        let first = plan_import(&project, &parsed("Address,Name\n100,A\n"));
        let Some(Command::Batch(cmds)) = &first.command else {
            panic!("expected a batch, got {:?}", first.command);
        };
        assert!(
            matches!(cmds.last(), Some(Command::ReserveIds { .. })),
            "{cmds:?}"
        );
        stack
            .do_command(&mut project, first.command.unwrap())
            .unwrap();
        stack.undo(&mut project).unwrap();
        assert!(project.installations[0].group_addresses.is_empty());
        assert_eq!(project.ids.peek_group_address(), 1);

        let second = plan_import(&project, &parsed("Address,Name\n200,B\n"));
        stack
            .do_command(&mut project, second.command.unwrap())
            .unwrap();
        let only = &project.installations[0].group_addresses[0];
        assert_eq!(only.id, GroupAddressId(2));
        assert_eq!(only.source.ets_id, "KB-GA-2");
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
        let Command::CreateGroupAddress { entry, .. } = &cmds[0] else {
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
        let Command::CreateGroupAddress { entry, .. } = &cmds[0] else {
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
    fn an_empty_central_cell_on_update_leaves_an_existing_true_alone() {
        let mut project = empty_project(GroupAddressStyle::Free);
        project.installations[0]
            .group_addresses
            .push(entry_with_flags(1, 100, "Kitchen Light", true, true));

        // No Central/Unfiltered column at all, so both cells read as `None`
        // ("empty") for every row — this is the case `unwrap_or(false)` and
        // `unwrap_or(existing.central)` disagree on, so a name-only update
        // must still carry the *existing* `true` through untouched.
        let plan = plan_import(&project, &parsed("Address,Name\n100,New Name\n"));

        let Some(Command::Batch(cmds)) = plan.command else {
            panic!("expected a Batch command, got {:?}", plan.command);
        };
        assert_eq!(
            cmds[0],
            Command::UpdateGroupAddress {
                id: GroupAddressId(1),
                name: "New Name".to_string(),
                central: true,
                unfiltered: true,
            }
        );
    }

    #[test]
    fn an_explicit_false_cell_on_update_flips_an_existing_true() {
        let mut project = empty_project(GroupAddressStyle::Free);
        project.installations[0]
            .group_addresses
            .push(entry_with_flags(1, 100, "Kitchen Light", true, true));

        let text = "Address,Name,Central,Unfiltered\n100,Kitchen Light,false,true\n";
        let plan = plan_import(&project, &parsed(text));

        assert_eq!(plan.report.updated, 1, "{:?}", plan.report);
        let Some(Command::Batch(cmds)) = plan.command else {
            panic!("expected a Batch command, got {:?}", plan.command);
        };
        assert_eq!(
            cmds[0],
            Command::UpdateGroupAddress {
                id: GroupAddressId(1),
                name: "Kitchen Light".to_string(),
                central: false,
                unfiltered: true,
            }
        );
    }

    #[test]
    fn an_empty_central_cell_on_create_applies_false() {
        let project = empty_project(GroupAddressStyle::Free);
        // Unlike the update case above, a brand-new row has no existing
        // value to fall back to, so an empty cell must apply `false`, not
        // leave anything alone.
        let plan = plan_import(&project, &parsed("Address,Name\n100,Kitchen Light\n"));

        let Some(Command::Batch(cmds)) = plan.command else {
            panic!("expected a Batch command, got {:?}", plan.command);
        };
        let Command::CreateGroupAddress { entry, .. } = &cmds[0] else {
            panic!("expected CreateGroupAddress, got {:?}", cmds[0]);
        };
        assert!(!entry.central, "expected central to default to false");
        assert!(!entry.unfiltered, "expected unfiltered to default to false");
    }

    #[test]
    fn read_only_columns_are_carried_into_the_report_and_edits_are_rejected() {
        let project = empty_project(GroupAddressStyle::Free);
        let text = "Address,Name,DatapointType\n100,Kitchen Light,DPST-1-1\n";
        let plan = plan_import(&project, &parsed(text));

        assert_eq!(plan.report.ignored_columns.len(), 1);
        assert_eq!(plan.report.ignored_columns[0].name, "DatapointType");
        assert_eq!(plan.command, None);
        assert!(plan.report.problems.iter().any(|problem| {
            problem.severity == Severity::Error
                && problem.detail.contains("DatapointType")
                && problem.detail.contains("read-only")
        }));
    }

    #[test]
    fn exporting_then_importing_an_unchanged_project_is_a_no_op() {
        let mut project = empty_project(GroupAddressStyle::Free);
        project.installations[0]
            .group_ranges
            .push(range(1, "Lighting", 1, 200, None));
        project.installations[0]
            .group_ranges
            .push(range(2, "Ground Floor", 1, 100, Some(1)));
        project.installations[0]
            .group_addresses
            .push(entry(1, 50, "Kitchen Light"));
        project.devices.insert_com_object(linked_com_object(
            1,
            1,
            1,
            Some(DptRef::parse("DPST-1-1").unwrap()),
        ));

        let export = crate::export_group_addresses(&project);
        let parsed = parse_group_addresses(&export.text, GroupAddressStyle::Free);
        let plan = plan_import(&project, &parsed);

        assert!(
            plan.report.problems.is_empty(),
            "{:?}",
            plan.report.problems
        );
        assert_eq!(plan.report.unchanged, 1);
        assert_eq!(plan.command, None);
    }

    #[test]
    fn explicit_readdress_preserves_id_and_reports_affected_links() {
        let mut project = empty_project(GroupAddressStyle::Free);
        project.installations[0]
            .group_addresses
            .push(entry(1, 100, "Kitchen Light"));
        let mut object = linked_com_object(7, 1, 1, None);
        object.links.push(GroupLink {
            ga: GroupAddressId(1),
            direction: Direction::Receive,
        });
        project.devices.insert_com_object(object);
        let parsed = parsed("Address,Action,NewAddress,Name\n100,readdress,200,Kitchen Light\n");

        let plan = plan_import(&project, &parsed);

        assert_eq!(plan.report.readdressed, 1);
        assert_eq!(plan.report.destructive_changes.len(), 1);
        assert_eq!(
            plan.report.destructive_changes[0].affected_links,
            vec![
                CsvAffectedLink {
                    com_object: ComObjectInstanceId(7),
                    direction: Direction::Send,
                },
                CsvAffectedLink {
                    com_object: ComObjectInstanceId(7),
                    direction: Direction::Receive,
                },
            ]
        );
        let mut project_after = project.clone();
        plan.command
            .expect("readdress should be applicable")
            .apply(&mut project_after)
            .unwrap();
        assert_eq!(project_after.installations[0].group_addresses[0].id.0, 1);
        assert_eq!(
            project_after.installations[0].group_addresses[0]
                .address
                .raw(),
            200
        );
        assert_eq!(
            project_after
                .devices
                .com_object(ComObjectInstanceId(7))
                .unwrap()
                .links[0]
                .ga
                .0,
            1
        );
    }

    #[test]
    fn readdress_to_an_existing_target_is_blocked_before_mutation() {
        let mut project = empty_project(GroupAddressStyle::Free);
        project.installations[0]
            .group_addresses
            .push(entry(1, 100, "A"));
        project.installations[0]
            .group_addresses
            .push(entry(2, 200, "B"));

        let plan = plan_import(
            &project,
            &parsed("Address,Action,NewAddress,Name\n100,readdress,200,A\n"),
        );

        assert!(plan.command.is_none());
        assert!(plan
            .report
            .problems
            .iter()
            .any(|problem| problem.detail.contains("already exists")));
    }

    #[test]
    fn two_rows_cannot_claim_the_same_new_target() {
        let mut project = empty_project(GroupAddressStyle::Free);
        project.installations[0]
            .group_addresses
            .push(entry(1, 100, "A"));
        project.installations[0]
            .group_addresses
            .push(entry(2, 200, "B"));

        let plan = plan_import(
            &project,
            &parsed("Address,Action,NewAddress,Name\n100,readdress,300,A\n200,readdress,300,B\n"),
        );

        assert!(plan.command.is_none());
        assert!(plan
            .report
            .problems
            .iter()
            .any(|problem| problem.detail.contains("already claimed by line 2")));
    }

    #[test]
    fn delete_with_remaining_links_is_blocked_and_names_the_reference_count() {
        let mut project = empty_project(GroupAddressStyle::Free);
        project.installations[0]
            .group_addresses
            .push(entry(1, 100, "A"));
        project
            .devices
            .insert_com_object(linked_com_object(7, 1, 1, None));

        let plan = plan_import(
            &project,
            &parsed("Address,Action,NewAddress,Name\n100,delete,,A\n"),
        );

        assert!(plan.command.is_none());
        assert_eq!(plan.report.deleted, 1);
        assert!(plan
            .report
            .problems
            .iter()
            .any(|problem| problem.detail.contains("1 communication-object link")));
    }

    #[test]
    fn unreferenced_explicit_delete_plans_a_reversible_core_command() {
        let mut project = empty_project(GroupAddressStyle::Free);
        project.installations[0]
            .group_addresses
            .push(entry(1, 100, "A"));

        let plan = plan_import(
            &project,
            &parsed("Address,Action,NewAddress,Name\n100,delete,,A\n"),
        );

        assert_eq!(plan.report.deleted, 1);
        assert!(matches!(plan.command, Some(Command::Batch(_))));
    }
}
