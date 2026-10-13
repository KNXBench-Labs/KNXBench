//! Shared counting and grouping rules for import findings (issue #7).
//!
//! The import report keeps one row per finding. Two kinds repeat so much
//! that a list of single rows hides everything else, so presentation (the
//! server's session log) groups them and the project status counts them once
//! per group:
//!
//! - per-file "retained payload" notices sharing one wording
//!   ([`retained_payload_label`]);
//! - unknown elements/attributes found in *manufacturer* data (application
//!   programs in `M-xxxx/…`), grouped by kind and name. Unknowns in the
//!   project itself stay individual: they describe the installation.
//!
//! Every count here equals the number of warning/error rows the session log
//! shows for the same report; the server's tests assert that.

use knx_etsproj::report::Severity;
use knx_etsproj::ImportReport;

pub use crate::import_payload_evidence::retained_payload_label;

/// The group of an unknown construct: `Some((kind, name))` for manufacturer
/// data, `None` for a construct that stays an individual finding. `kind` is
/// the report's serialized form (`"Element"`/`"Attribute"`).
pub fn manufacturer_unknown_group<'a>(
    source_path: &str,
    kind: &'a str,
    name: &'a str,
) -> Option<(&'a str, &'a str)> {
    knx_etsproj::opaque::is_manufacturer_entry(source_path).then_some((kind, name))
}

/// Serialized form of an unknown construct's kind, as the report stores it.
pub fn unknown_kind_name(kind: knx_etsproj::UnknownKind) -> &'static str {
    match kind {
        knx_etsproj::UnknownKind::Element => "Element",
        knx_etsproj::UnknownKind::Attribute => "Attribute",
    }
}

/// How many warnings an unsupported-capability list contributes: one per
/// ordinary row, one per distinct [`retained_payload_label`].
pub(crate) fn unsupported_warning_rows<'a>(
    rows: impl IntoIterator<Item = (&'a str, &'a str)>,
) -> usize {
    let mut labels = std::collections::BTreeSet::new();
    let mut other = 0usize;
    for (what, consequence) in rows {
        match retained_payload_label(what, consequence) {
            Some(label) => {
                labels.insert(label);
            }
            None => other += 1,
        }
    }
    other + labels.len()
}

/// How many warnings an unknown-construct list contributes: one per project
/// construct, one per distinct manufacturer `(kind, name)`.
pub(crate) fn unknown_warning_rows<'a>(
    rows: impl IntoIterator<Item = (&'a str, &'a str, &'a str)>,
) -> usize {
    let mut groups = std::collections::BTreeSet::new();
    let mut other = 0usize;
    for (source_path, kind, name) in rows {
        match manufacturer_unknown_group(source_path, kind, name) {
            Some(group) => {
                groups.insert(group);
            }
            None => other += 1,
        }
    }
    other + groups.len()
}

/// `(errors, warnings)` of an import report, the one counting rule for the
/// project status of a fresh import and of a reopened project's historical
/// report alike: genuine `Severity::Error` rows are errors; warning-severity
/// rows, unknown constructs, datapoint-type conflicts and unsupported
/// capabilities are warnings, with the two repetitive kinds above counted
/// once per group.
pub fn import_finding_counts(report: &ImportReport) -> (usize, usize) {
    let errors = report
        .errors
        .iter()
        .filter(|e| e.severity == Severity::Error)
        .count();
    let unsupported = unsupported_warning_rows(
        report
            .unsupported
            .iter()
            .map(|row| (row.what.as_str(), row.consequence.as_str())),
    );
    let unknown = unknown_warning_rows(report.unknown.iter().map(|row| {
        (
            row.source_path.as_str(),
            unknown_kind_name(row.kind),
            row.name.as_str(),
        )
    }));
    let warnings = report.errors.len() - errors + unknown + report.conflicts.len() + unsupported;
    (errors, warnings)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manufacturer_unknowns_group_by_kind_and_name_and_project_ones_stay_single() {
        let rows = [
            ("M-0083/M-0083_A-018C-21-716E.xml", "Attribute", "Name"),
            ("M-0001/M-0001_A-9205-04-9270.xml", "Attribute", "Name"),
            ("M-0083/M-0083_A-018C-21-716E.xml", "Attribute", "Id"),
            ("M-0083/M-0083_A-018C-21-716E.xml", "Element", "Name"),
            ("P-0512/project.xml", "Attribute", "ProjectId"),
            ("P-0512/project.xml", "Attribute", "ProjectId"),
            ("knx_master.xml", "Attribute", "Name"),
        ];
        // Name/Id/Element-Name groups (3) + two project rows + the master row.
        assert_eq!(unknown_warning_rows(rows), 6);
        assert_eq!(
            manufacturer_unknown_group("M-0001/x.xml", "Attribute", "Name"),
            Some(("Attribute", "Name"))
        );
        assert_eq!(
            manufacturer_unknown_group("P-0001/M-x.xml", "Attribute", "Name"),
            None
        );
    }
}
