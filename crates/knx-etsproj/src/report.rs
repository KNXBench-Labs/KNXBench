//! Stage 6: the import report — a deliverable, not a log.
//!
//! Every prior stage produces its own typed output for programmatic use
//! (`ValidationOutput::errors`, `MapOutput::problems`, and so on); this
//! module folds all of them into one structured, JSON-serializable summary
//! a human or another tool can read without knowing the internal stage
//! shapes. `counts` carries read-versus-mapped figures per entity type so a
//! discrepancy is a number, not a suspicion; `errors` flattens both
//! [`crate::validate::ValidationOutput`]'s and [`crate::map::MapOutput`]'s
//! typed problems into plain xpath/message pairs, tagged by stage and
//! severity, rather than exposing their internal enums to JSON — those stay
//! typed for code that wants to match on them, this report stays simple for
//! everything else.
//!
//! `has_losses()` answers "was anything actually lost or misunderstood",
//! not "does this project use a feature we can't yet edit". An unknown
//! construct or a validation/mapping error is a loss; an
//! [`UnsupportedFeature`] like vendor baggage is a documented capability
//! gap over data this application preserves byte-for-byte in the opaque
//! store — nothing about it is lost, so it does not count.

use serde::Serialize;

use crate::detect::Detected;
use crate::map::{Count, MapOutput};
use crate::opaque::OpaqueEntry;
use crate::parse::UnknownConstruct;
use crate::validate::ValidationOutput;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ImportReport {
    pub source: SourceInfo,
    pub counts: EntityCounts,
    pub unknown: Vec<UnknownConstruct>,
    pub opaque: Vec<OpaqueSummary>,
    pub inferred: Vec<crate::infer::InferredValue>,
    pub conflicts: Vec<crate::infer::Conflict>,
    pub unsupported: Vec<UnsupportedFeature>,
    pub errors: Vec<ImportError>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SourceInfo {
    pub file_name: String,
    pub file_size: u64,
    pub schema_version: u32,
    pub namespace: String,
    pub created_by: Option<String>,
    pub tool_version: Option<String>,
    pub namespace_disagreement: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct EntityCounts {
    pub rows: Vec<EntityCount>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct EntityCount {
    pub entity: String,
    pub read: u32,
    pub mapped: u32,
}

/// Bytes are not repeated in the report; the store holds them.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct OpaqueSummary {
    pub source_path: String,
    pub kind: String,
    pub size: u64,
    pub sha256: String,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct UnsupportedFeature {
    pub what: String,
    pub consequence: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Error,
    Warning,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ImportError {
    /// Which stage found this: `"validate"` or `"map"`.
    pub stage: &'static str,
    pub severity: Severity,
    pub xpath: String,
    /// The stage's own typed problem detail, `Debug`-formatted. Not pretty,
    /// but always available and always accurate — adding a `Display` impl
    /// to every `ProblemDetail`/`MapProblemDetail` variant purely to shave
    /// punctuation off a report string is not worth the upkeep yet.
    pub message: String,
}

impl ImportReport {
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).expect("ImportReport contains no non-serializable value")
    }

    /// Whether anything was actually lost or misunderstood — not whether a
    /// documented capability gap ([`Self::unsupported`]) exists.
    pub fn has_losses(&self) -> bool {
        !self.errors.is_empty() || !self.unknown.is_empty() || !self.conflicts.is_empty()
    }
}

/// Assembles the report from every prior stage's output. `file_name` and
/// `file_size` come from the caller, since nothing upstream of this module
/// reads the container's own file metadata.
#[allow(clippy::too_many_arguments)]
pub fn build(
    file_name: &str,
    file_size: u64,
    detected: &Detected,
    unknown: &[UnknownConstruct],
    validation: &ValidationOutput,
    map: &MapOutput,
    inference: &crate::infer::InferenceOutput,
    opaque: &[OpaqueEntry],
) -> ImportReport {
    let source = SourceInfo {
        file_name: file_name.to_string(),
        file_size,
        schema_version: detected.version.0,
        namespace: detected.namespace.clone(),
        created_by: detected.created_by.clone(),
        tool_version: detected.tool_version.clone(),
        namespace_disagreement: detected.namespace_disagreement.map(|master_version| {
            format!(
                "knx_master.xml declares schema {}, the project part declares schema {}",
                master_version.0, detected.version.0
            )
        }),
    };

    let counts = EntityCounts {
        rows: vec![
            entity_count("Area", map.counts.areas),
            entity_count("Line", map.counts.lines),
            entity_count("DeviceInstance", map.counts.devices),
            entity_count("ComObjectInstanceRef", map.counts.com_objects),
            entity_count("GroupRange", map.counts.group_ranges),
            entity_count("GroupAddress", map.counts.group_addresses),
            entity_count("BuildingPart", map.counts.building_parts),
            entity_count("ParameterInstanceRef", map.counts.parameters),
        ],
    };

    let opaque_summaries = opaque.iter().map(opaque_summary).collect();
    let unsupported = opaque
        .iter()
        .filter(|e| matches!(e.kind, crate::opaque::OpaqueKind::Baggage))
        .map(|e| UnsupportedFeature {
            what: e.source_path.clone(),
            consequence: format!(
                "{}: vendor-supplied plugin code; not executed by this application, so any \
                 device configuration behavior it implements is unavailable here",
                e.source_path
            ),
        })
        .collect();

    let mut errors: Vec<ImportError> = Vec::new();
    for p in &validation.errors {
        errors.push(ImportError {
            stage: "validate",
            severity: Severity::Error,
            xpath: p.xpath.clone(),
            message: format!("{:?}", p.detail),
        });
    }
    for p in &validation.warnings {
        errors.push(ImportError {
            stage: "validate",
            severity: Severity::Warning,
            xpath: p.xpath.clone(),
            message: format!("{:?}", p.detail),
        });
    }
    for p in &map.problems {
        errors.push(ImportError {
            stage: "map",
            severity: Severity::Error,
            xpath: p.xpath.clone(),
            message: format!("{:?}", p.detail),
        });
    }

    ImportReport {
        source,
        counts,
        unknown: unknown.to_vec(),
        opaque: opaque_summaries,
        inferred: inference.inferred.clone(),
        conflicts: inference.conflicts.clone(),
        unsupported,
        errors,
    }
}

fn entity_count(entity: &str, c: Count) -> EntityCount {
    EntityCount {
        entity: entity.to_string(),
        read: c.read as u32,
        mapped: c.mapped as u32,
    }
}

fn opaque_summary(e: &OpaqueEntry) -> OpaqueSummary {
    use crate::opaque::OpaqueKind;
    let reason = match e.kind {
        OpaqueKind::ContainerEntry => "container entry not regenerated by this build",
        OpaqueKind::ManufacturerData => {
            "manufacturer/application-program data; moves to the shared product database in Session 4"
        }
        OpaqueKind::Baggage => "vendor plugin binary; never executed",
        OpaqueKind::BinaryData => "opaque per-device binary blob, not interpreted",
        OpaqueKind::ExtraData => "ETS tool-internal data, not interpreted",
        OpaqueKind::Signature => "archive signature; cannot be regenerated",
        OpaqueKind::MasterData => "the DPT/product master catalogue",
        OpaqueKind::RetainedAttribute => "known or unknown attribute the model does not carry",
        OpaqueKind::RetainedElement => "element the model does not carry",
    };
    OpaqueSummary {
        source_path: e.source_path.clone(),
        kind: format!("{:?}", e.kind),
        size: e.bytes.len() as u64,
        sha256: e.sha256.clone(),
        reason: reason.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::container::Container;
    use crate::detect::detect;
    use crate::known::known_schema;
    use crate::map::map;
    use crate::opaque::collect_container_entries;
    use crate::parse::parse_installation;
    use crate::testutil::reference_ets4_bytes;
    use crate::validate::validate;

    fn reference_report() -> ImportReport {
        let bytes = reference_ets4_bytes();
        let file_size = bytes.len() as u64;
        let mut c = Container::open(bytes).unwrap();
        let detected = detect(&mut c).unwrap();
        let schema = known_schema(detected.version.0).unwrap();

        let part = c.project_part().unwrap().to_string();
        let topology_bytes = c.read(&format!("{part}/0.xml")).unwrap();
        let parsed = parse_installation(&topology_bytes, &format!("{part}/0.xml"), schema).unwrap();

        let validation = validate(&parsed.document);
        let mapped = map(&parsed.document, &format!("{part}/0.xml"));
        let inference = crate::infer::infer_group_address_dpts(&mapped.project);
        let opaque = collect_container_entries(
            &mut c,
            &[&format!("{part}/0.xml"), &format!("{part}/Project.xml")],
        )
        .unwrap();

        build(
            "Unser Zuhause ets4 - 2025-12-15.knxproj",
            file_size,
            &detected,
            &parsed.unknown,
            &validation,
            &mapped,
            &inference,
            &opaque,
        )
    }

    #[test]
    fn counts_carry_both_read_and_mapped_figures() {
        let r = reference_report();
        let devices = r
            .counts
            .rows
            .iter()
            .find(|c| c.entity == "DeviceInstance")
            .unwrap();
        assert_eq!(devices.read, 36);
        assert_eq!(devices.mapped, 36);
    }

    #[test]
    fn a_report_with_no_losses_says_so() {
        let r = reference_report();
        assert_eq!(r.errors, vec![]);
        assert_eq!(r.unknown, vec![]);
        assert!(!r.has_losses());
    }

    #[test]
    fn the_json_form_round_trips_and_names_every_section() {
        let json = reference_report().to_json();
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        for key in [
            "source",
            "counts",
            "unknown",
            "opaque",
            "inferred",
            "conflicts",
            "unsupported",
            "errors",
        ] {
            assert!(v.get(key).is_some(), "missing report section {key}");
        }
        assert_eq!(v["source"]["schema_version"], 11);
    }

    #[test]
    fn vendor_baggage_is_reported_as_unsupported() {
        let r = reference_report();
        let u = r
            .unsupported
            .iter()
            .find(|u| u.what.contains("econEts3.dll"))
            .unwrap();
        assert!(u.consequence.contains("not executed"));
    }

    #[test]
    fn the_opaque_summary_does_not_repeat_the_bytes() {
        let json = reference_report().to_json();
        // 22 MB of manufacturer data must not end up inside a JSON report.
        assert!(json.len() < 512 * 1024);
    }
}
