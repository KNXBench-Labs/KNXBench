//! The "KNXBench group-address CSV v1" writer (see the crate-level docs for
//! why that name, and not "ETS CSV").
//!
//! Design: `docs/superpowers/specs/2026-09-10-csv-group-address-exchange-design.md`
//! §3 (the format).

use knx_core::{
    DptRef, GroupAddress, GroupAddressDpt, GroupAddressId, GroupAddressTypeOutcome, Installation,
    InstallationId, Project,
};

use crate::read::{CsvProblem, Severity};

/// The result of writing "KNXBench group-address CSV v1" text from a
/// project: the CSV text itself, and any warnings worth surfacing (today:
/// a `DatapointType` that could not be derived because the linked
/// communication objects disagree).
///
/// Reuses [`CsvProblem`] rather than a bespoke `CsvWarning` type — it
/// already carries exactly the `row`/`severity`/`detail` shape an export
/// warning needs, and CLAUDE.md forbids duplicating that.
#[derive(Debug, Clone, PartialEq)]
pub struct CsvExport {
    pub text: String,
    pub warnings: Vec<CsvProblem>,
}

/// Writes every group address in `project.installations.first()`, in their
/// existing order, as "KNXBench group-address CSV v1" text: UTF-8 with a
/// leading BOM, CRLF line endings, columns in the design §3 table's order.
/// Addresses are formatted exclusively through
/// [`GroupAddress::format`][knx_core::GroupAddress::format], in the
/// project's own [`GroupAddressStyle`][knx_core::GroupAddressStyle]. A
/// project with no installation writes just the header. Never mutates
/// `project`.
pub fn export_group_addresses(project: &Project) -> CsvExport {
    write_installation(project, project.installations.first())
}

/// An installation id that names no installation of the project.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnknownInstallation(pub InstallationId);

impl std::fmt::Display for UnknownInstallation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "installation {} does not exist", self.0)
    }
}

impl std::error::Error for UnknownInstallation {}

/// [`export_group_addresses`] for installation `target` (MODEL-01); `None`
/// keeps the first installation. Range names come from that installation.
pub fn export_group_addresses_from(
    project: &Project,
    target: Option<InstallationId>,
) -> Result<CsvExport, UnknownInstallation> {
    match target {
        None => Ok(export_group_addresses(project)),
        Some(id) => project
            .installations
            .iter()
            .find(|installation| installation.id == id)
            .map(|installation| write_installation(project, Some(installation)))
            .ok_or(UnknownInstallation(id)),
    }
}

fn write_installation(project: &Project, installation: Option<&Installation>) -> CsvExport {
    let style = project.info.group_address_style;
    let mut warnings = Vec::new();

    let mut writer = csv::WriterBuilder::new()
        .terminator(csv::Terminator::CRLF)
        .from_writer(Vec::new());
    writer
        .write_record([
            "Address",
            "Action",
            "NewAddress",
            "Name",
            "Central",
            "Unfiltered",
            "DatapointType (read-only)",
            "MainGroup (read-only)",
            "MiddleGroup (read-only)",
        ])
        .expect("writing to an in-memory Vec<u8> cannot fail");

    if let Some(installation) = installation {
        for (index, ga) in installation.group_addresses.iter().enumerate() {
            // 1-based, counting the header: matches what a spreadsheet
            // shows, and what `read.rs`'s `CsvRow::line`/`CsvProblem::row`
            // already mean.
            let line = index + 2;

            let (dpt, contest) = derive_dpt(project, ga.id);
            if let Some(contest) = contest {
                let why = match contest {
                    Contest::Linked => "linked communication objects disagree on datapoint type",
                    Contest::DeclaredWidth => {
                        "declared datapoint type and a linked communication object's differ in size"
                    }
                };
                warnings.push(CsvProblem {
                    row: Some(line),
                    severity: Severity::Warning,
                    detail: format!(
                        "address {}: {why}; DatapointType left empty",
                        ga.address.format(style)
                    ),
                });
            }
            let (main_group, middle_group) = containing_range_names(installation, ga.address);

            writer
                .write_record([
                    ga.address.format(style),
                    "upsert".to_string(),
                    String::new(),
                    ga.name.clone(),
                    ga.central.to_string(),
                    ga.unfiltered.to_string(),
                    dpt.map(|d| d.to_string()).unwrap_or_default(),
                    main_group.unwrap_or_default(),
                    middle_group.unwrap_or_default(),
                ])
                .expect("writing to an in-memory Vec<u8> cannot fail");
        }
    }

    let bytes = writer
        .into_inner()
        .expect("flushing an in-memory Vec<u8> cannot fail");
    let body = String::from_utf8(bytes)
        .expect("the csv writer only emits UTF-8 when fed UTF-8, which every field here is");

    CsvExport {
        text: format!("\u{FEFF}{body}"),
        warnings,
    }
}

/// The `DatapointType` column's value for `ga`: its effective type
/// (ADR-0078) — the group address's own declaration where it applies,
/// otherwise the unanimous DPT of every linked communication object. Empty
/// when nothing states one; empty plus a contest (a warning, at the call
/// site) when the linked objects disagree or the declaration and the linked
/// objects differ in width.
pub(crate) fn derive_dpt(
    project: &Project,
    ga: GroupAddressId,
) -> (Option<DptRef>, Option<Contest>) {
    let ty = knx_core::resolve_group_address_type(project, ga);
    match ty.effective() {
        GroupAddressDpt::None => (None, None),
        GroupAddressDpt::Single(dpt) => (Some(dpt), None),
        GroupAddressDpt::Conflict(_) if ty.outcome == GroupAddressTypeOutcome::SizeConflict => {
            (None, Some(Contest::DeclaredWidth))
        }
        GroupAddressDpt::Conflict(_) => (None, Some(Contest::Linked)),
    }
}

/// Why `derive_dpt` left the column empty although types were stated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Contest {
    /// The linked communication objects disagree.
    Linked,
    /// The address's declared type and a linked object's differ in width.
    DeclaredWidth,
}

/// The names of the main and/or middle `GroupRange`s containing `address`,
/// if any — `group.rs`'s own doc comment notes the model never nests more
/// than two levels deep, so at most one of each can match.
pub(crate) fn containing_range_names(
    installation: &Installation,
    address: GroupAddress,
) -> (Option<String>, Option<String>) {
    let mut main = None;
    let mut middle = None;
    for range in &installation.group_ranges {
        if !range.contains(address) {
            continue;
        }
        if range.parent.is_none() {
            main = Some(range.name.clone());
        } else {
            middle = Some(range.name.clone());
        }
    }
    (main, middle)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::{empty_project, entry, linked_com_object, range};
    use knx_core::{DptRef, GroupAddressStyle};

    #[test]
    fn writes_the_header_row_in_the_design_table_order() {
        let project = empty_project(GroupAddressStyle::Free);
        let export = export_group_addresses(&project);

        let header = export.text.lines().next().unwrap();
        assert_eq!(
            header.trim_start_matches('\u{FEFF}').trim_end_matches('\r'),
            "Address,Action,NewAddress,Name,Central,Unfiltered,DatapointType (read-only),MainGroup (read-only),MiddleGroup (read-only)"
        );
    }

    #[test]
    fn starts_with_a_bom_and_uses_crlf() {
        let mut project = empty_project(GroupAddressStyle::Free);
        project.installations[0]
            .group_addresses
            .push(entry(1, 100, "Kitchen Light"));

        let export = export_group_addresses(&project);

        assert!(export.text.starts_with('\u{FEFF}'), "{:?}", export.text);
        assert!(
            export.text.contains("\r\n"),
            "expected CRLF line endings, got {:?}",
            export.text
        );
        let body = export.text.trim_start_matches('\u{FEFF}');
        let newline_count = body.matches('\n').count();
        let crlf_count = body.matches("\r\n").count();
        assert_eq!(
            newline_count, crlf_count,
            "expected every line break to be CRLF, got {:?}",
            export.text
        );
    }

    #[test]
    fn formats_addresses_in_the_projects_own_style() {
        let mut project = empty_project(GroupAddressStyle::ThreeLevel);
        project.installations[0].group_addresses.push(entry(
            1,
            // main=1, middle=2, sub=3 in three-level packing.
            (1 << 11) | (2 << 8) | 3,
            "Living Room",
        ));

        let export = export_group_addresses(&project);

        let data_line = export.text.lines().nth(1).unwrap();
        assert!(
            data_line.starts_with("1/2/3,"),
            "expected the three-level address first, got {:?}",
            data_line
        );
    }

    #[test]
    fn a_name_with_separator_quote_and_newline_round_trips_through_quoting() {
        let mut project = empty_project(GroupAddressStyle::Free);
        let mut e = entry(1, 100, "K\u{fc}che, \"gro\u{df}\"\nAnnex");
        e.name = "K\u{fc}che, \"gro\u{df}\"\nAnnex".to_string();
        project.installations[0].group_addresses.push(e);

        let export = export_group_addresses(&project);
        let parsed = crate::read::parse_group_addresses(&export.text, GroupAddressStyle::Free);

        assert!(parsed.problems.is_empty(), "{:?}", parsed.problems);
        assert_eq!(parsed.rows.len(), 1);
        assert_eq!(parsed.rows[0].name, "K\u{fc}che, \"gro\u{df}\"\nAnnex");
    }

    #[test]
    fn datapoint_type_is_the_unanimous_dpt_of_linked_com_objects() {
        let mut project = empty_project(GroupAddressStyle::Free);
        project.installations[0]
            .group_addresses
            .push(entry(1, 100, "Kitchen Light"));
        let dpt = DptRef::parse("DPST-1-1").unwrap();
        project
            .devices
            .insert_com_object(linked_com_object(1, 1, 1, Some(dpt)));
        project
            .devices
            .insert_com_object(linked_com_object(2, 1, 1, Some(dpt)));

        let export = export_group_addresses(&project);

        let data_line = export.text.lines().nth(1).unwrap();
        assert!(
            data_line.contains(",DPST-1-1,"),
            "expected the unanimous DPT in the row, got {:?}",
            data_line
        );
        assert!(export.warnings.is_empty(), "{:?}", export.warnings);
    }

    #[test]
    fn datapoint_type_is_empty_when_no_com_object_is_linked() {
        let mut project = empty_project(GroupAddressStyle::Free);
        project.installations[0]
            .group_addresses
            .push(entry(1, 100, "Kitchen Light"));

        let export = export_group_addresses(&project);

        let data_line = export.text.lines().nth(1).unwrap();
        assert!(
            data_line.contains(",,,"),
            "expected an empty DatapointType column, got {:?}",
            data_line
        );
        assert!(export.warnings.is_empty(), "{:?}", export.warnings);
    }

    #[test]
    fn datapoint_type_is_empty_plus_a_warning_when_linked_com_objects_disagree() {
        let mut project = empty_project(GroupAddressStyle::Free);
        project.installations[0]
            .group_addresses
            .push(entry(1, 100, "Kitchen Light"));
        project.devices.insert_com_object(linked_com_object(
            1,
            1,
            1,
            Some(DptRef::parse("DPST-1-1").unwrap()),
        ));
        project.devices.insert_com_object(linked_com_object(
            2,
            1,
            1,
            Some(DptRef::parse("DPST-5-1").unwrap()),
        ));

        let export = export_group_addresses(&project);

        let data_line = export.text.lines().nth(1).unwrap();
        assert!(
            data_line.contains(",Kitchen Light,false,false,,"),
            "expected an empty DatapointType column, got {:?}",
            data_line
        );
        assert_eq!(export.warnings.len(), 1);
        assert_eq!(export.warnings[0].severity, Severity::Warning);
        assert!(export.warnings[0].detail.contains("disagree"));
    }

    #[test]
    fn datapoint_type_uses_the_declaration_and_warns_on_a_width_conflict() {
        // ADR-0078. Address 100 declares 9.001 over a linked 9.004 (same
        // width: the declaration is the column); 101 declares 1.001 over a
        // linked 5.001 (width conflict: empty plus a warning naming size).
        let mut project = empty_project(GroupAddressStyle::Free);
        for (id, raw, declared) in [(1, 100, "DPST-9-1"), (2, 101, "DPST-1-1")] {
            let mut ga = entry(id, raw, "GA");
            ga.declared_dpt = knx_core::Override::Value(knx_core::Resolved {
                value: DptRef::parse(declared).unwrap(),
                layer: knx_core::Layer::Instance,
            });
            project.installations[0].group_addresses.push(ga);
        }
        project.devices.insert_com_object(linked_com_object(
            1,
            1,
            1,
            Some(DptRef::parse("DPST-9-4").unwrap()),
        ));
        project.devices.insert_com_object(linked_com_object(
            2,
            1,
            2,
            Some(DptRef::parse("DPST-5-1").unwrap()),
        ));

        let export = export_group_addresses(&project);

        let lines: Vec<_> = export.text.lines().collect();
        assert!(
            lines[1].contains(",GA,false,false,DPST-9-1,"),
            "{:?}",
            lines[1]
        );
        assert!(lines[2].contains(",GA,false,false,,"), "{:?}", lines[2]);
        assert_eq!(export.warnings.len(), 1);
        assert_eq!(export.warnings[0].row, Some(3));
        assert!(export.warnings[0].detail.contains("differ in size"));
    }

    #[test]
    fn main_and_middle_group_carry_the_containing_ranges_names() {
        let mut project = empty_project(GroupAddressStyle::Free);
        project.installations[0]
            .group_ranges
            .push(range(1, "Lighting", 0, 200, None));
        project.installations[0]
            .group_ranges
            .push(range(2, "Ground Floor", 0, 100, Some(1)));
        project.installations[0]
            .group_addresses
            .push(entry(1, 50, "Kitchen Light"));

        let export = export_group_addresses(&project);

        let data_line = export.text.lines().nth(1).unwrap();
        assert!(
            data_line.ends_with(",Lighting,Ground Floor"),
            "expected MainGroup/MiddleGroup at the end, got {:?}",
            data_line
        );
    }

    #[test]
    fn main_and_middle_group_are_empty_when_nothing_contains_the_address() {
        let mut project = empty_project(GroupAddressStyle::Free);
        project.installations[0]
            .group_ranges
            .push(range(1, "Lighting", 0, 10, None));
        project.installations[0]
            .group_addresses
            .push(entry(1, 500, "Kitchen Light"));

        let export = export_group_addresses(&project);

        let data_line = export.text.lines().nth(1).unwrap();
        assert!(
            data_line.ends_with(",,"),
            "expected empty MainGroup/MiddleGroup, got {:?}",
            data_line
        );
    }

    #[test]
    fn iterates_group_addresses_in_their_existing_order() {
        let mut project = empty_project(GroupAddressStyle::Free);
        project.installations[0]
            .group_addresses
            .push(entry(2, 200, "Second"));
        project.installations[0]
            .group_addresses
            .push(entry(1, 100, "First"));

        let export = export_group_addresses(&project);

        let lines: Vec<&str> = export.text.lines().skip(1).collect();
        assert_eq!(lines.len(), 2);
        assert!(
            lines[0].starts_with("200,upsert,,Second,"),
            "{:?}",
            lines[0]
        );
        assert!(lines[1].starts_with("100,upsert,,First,"), "{:?}", lines[1]);
    }

    #[test]
    fn a_project_with_no_installation_writes_only_the_header() {
        let project = Project::new(knx_core::Language("en".into()));

        let export = export_group_addresses(&project);

        assert_eq!(export.text.lines().count(), 1);
        assert!(export.warnings.is_empty());
    }
}
