//! Reading and writing `.knxproj`: ZIP container, schema detection, tolerant
//! XML parsing, mapping to and from `knx-core`, and the import report.

pub mod compare;
pub mod container;
pub mod detect;
pub mod export;
pub mod infer;
pub mod known;
pub mod map;
pub mod opaque;
pub mod parse;
pub mod report;
pub mod source;
#[cfg(test)]
mod testutil;
pub mod validate;
pub mod values;

pub use container::{Container, ContainerError, EntryInfo};
pub use detect::{detect, DetectError, Detected, SchemaVersion};
pub use known::{known_schema, KnownElement, KnownSchema};
pub use parse::{
    parse_installation, parse_project_info, ParseError, ParseOutput, UnknownConstruct, UnknownKind,
};
pub use source::{
    RetainedAttribute, RetainedElement, SourceArea, SourceBinaryDataRef, SourceBuildingPart,
    SourceComObjectInstance, SourceDevice, SourceDocument, SourceGroupAddress, SourceGroupRange,
    SourceInstallation, SourceLine, SourceParameterInstance, SourceProjectInfo,
};

/// The result of a successful [`import_knxproj`]: the mapped project, every
/// opaque entry export needs to write back unchanged, and the human-facing
/// report summarizing all of it.
pub struct ImportOutcome {
    pub project: knx_core::Project,
    pub opaque: Vec<opaque::OpaqueEntry>,
    pub report: report::ImportReport,
}

// Hand-written rather than derived: `knx_core::Project` implements no
// `Debug` (it owns a `StringTable`/`IdAllocators`, neither meaningfully
// printable), so `ImportOutcome` cannot derive it either. This impl exists
// only so `Result<ImportOutcome, _>::unwrap_err()` compiles in tests —
// `opaque`'s entries carry raw bytes up to tens of megabytes each (Task 12),
// so this prints their count, never their content.
impl std::fmt::Debug for ImportOutcome {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ImportOutcome")
            .field("project", &"knx_core::Project { .. }")
            .field("opaque", &format!("{} entries", self.opaque.len()))
            .field("report", &self.report)
            .finish()
    }
}

/// Why an import could not even begin — a container that cannot be opened
/// at all, a schema version with no known-element table, or XML that
/// cannot be read. Everything else (a dangling reference, an unparsable
/// timestamp, an unknown attribute) is a report entry instead: a project
/// that is partly readable should open partly, not fail outright.
#[derive(Debug)]
pub enum ImportFailure {
    Io(std::io::Error),
    Container(ContainerError),
    Detect(DetectError),
    Parse(ParseError),
    NoKnownSchemaTable { version: u32 },
}

impl std::fmt::Display for ImportFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ImportFailure::Io(e) => write!(f, "{e}"),
            ImportFailure::Container(e) => write!(f, "{e}"),
            ImportFailure::Detect(e) => write!(f, "{e}"),
            ImportFailure::Parse(e) => write!(f, "{e}"),
            ImportFailure::NoKnownSchemaTable { version } => {
                write!(f, "no known-element table for schema version {version}")
            }
        }
    }
}

impl std::error::Error for ImportFailure {}

/// Reads and imports a `.knxproj` file from disk.
pub fn import_knxproj(path: &std::path::Path) -> Result<ImportOutcome, ImportFailure> {
    let bytes = std::fs::read(path).map_err(ImportFailure::Io)?;
    let file_name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or_default()
        .to_string();
    import_knxproj_bytes(bytes, &file_name)
}

/// Imports a `.knxproj` archive already in memory, running every stage in
/// order: detect the schema version, parse both `0.xml` and `Project.xml`
/// against its known-element table, validate the parsed document,
/// map it into `knx_core::Project`, infer group address datapoint types,
/// collect every container entry the exporter would otherwise have to
/// regenerate, and fold all of it into one [`report::ImportReport`].
pub fn import_knxproj_bytes(
    bytes: Vec<u8>,
    file_name: &str,
) -> Result<ImportOutcome, ImportFailure> {
    let file_size = bytes.len() as u64;
    let mut container = Container::open(bytes).map_err(ImportFailure::Container)?;
    // Checked here, first, and not left to surface however `detect` (which
    // also needs it internally, wrapped in its own `DetectError`) happens
    // to encounter it: a container with no project part at all is a
    // container-level problem, not a detection-level one, and reporting it
    // as `ImportFailure::Container` rather than `ImportFailure::Detect`
    // says so.
    let part = container
        .project_part()
        .map_err(ImportFailure::Container)?
        .to_string();
    let detected = detect(&mut container).map_err(ImportFailure::Detect)?;
    let schema = known_schema(detected.version.0).ok_or(ImportFailure::NoKnownSchemaTable {
        version: detected.version.0,
    })?;

    let topology_path = format!("{part}/0.xml");
    let info_path = format!("{part}/Project.xml");

    let topology_bytes = container
        .read(&topology_path)
        .map_err(ImportFailure::Container)?;
    let mut parsed = parse_installation(&topology_bytes, &topology_path, schema)
        .map_err(ImportFailure::Parse)?;

    let info_bytes = container
        .read(&info_path)
        .map_err(ImportFailure::Container)?;
    let (info, info_unknown) =
        parse_project_info(&info_bytes, &info_path, schema).map_err(ImportFailure::Parse)?;
    parsed.document.info = info;

    let mut unknown = parsed.unknown;
    unknown.extend(info_unknown);

    let validation = validate::validate(&parsed.document);
    let mapped = map::map(&parsed.document, &topology_path);
    let inference = infer::infer_group_address_dpts(&mapped.project);

    let mut opaque_entries = opaque::collect_container_entries(
        &mut container,
        &[topology_path.as_str(), info_path.as_str()],
    )
    .map_err(ImportFailure::Container)?;
    for attribute in &mapped.retained {
        opaque_entries.push(opaque::from_retained_attribute(&topology_path, attribute));
    }
    for element in &parsed.retained_elements {
        opaque_entries.push(opaque::from_retained_element(&topology_path, element));
    }
    // `SourceLine/@BusAccess` is known but deliberately not modeled (Task
    // 6), so it is not among the genuinely-unknown elements in
    // `retained_elements` above — it is captured per-`SourceLine` instead.
    // Export needs it back regardless of which of the two buckets it came
    // from, so both are walked into the same opaque entry list here.
    for installation in &parsed.document.installations {
        for area in &installation.areas {
            for line in &area.lines {
                if let Some(bus_access) = &line.bus_access {
                    opaque_entries.push(opaque::from_retained_element(&topology_path, bus_access));
                }
            }
        }
    }

    let import_report = report::build(
        file_name,
        file_size,
        &detected,
        &unknown,
        &validation,
        &mapped,
        &inference,
        &opaque_entries,
    );

    Ok(ImportOutcome {
        project: mapped.project,
        opaque: opaque_entries,
        report: import_report,
    })
}

#[cfg(test)]
mod import_tests {
    use super::*;
    use crate::testutil::{reference_ets4_path, reference_ets6_path};
    use knx_core::GroupAddressStyle;

    #[test]
    fn importing_the_reference_project_succeeds_with_a_clean_report() {
        let out = import_knxproj(&reference_ets4_path()).unwrap();
        assert_eq!(out.report.source.schema_version, 11);
        assert_eq!(out.report.errors, vec![]);
        assert_eq!(out.project.info.name, "Unser Zuhause");
        assert_eq!(
            out.project.info.group_address_style,
            GroupAddressStyle::ThreeLevel
        );
    }

    #[test]
    fn importing_the_ets6_project_fails_with_a_named_reason_not_wrong_data() {
        let err = import_knxproj(&reference_ets6_path()).unwrap_err();
        assert!(matches!(
            err,
            ImportFailure::NoKnownSchemaTable { version: 23 }
        ));
    }

    #[test]
    fn the_opaque_entries_cover_every_container_entry_we_do_not_regenerate() {
        let out = import_knxproj(&reference_ets4_path()).unwrap();
        assert_eq!(out.opaque.iter().filter(|e| e.xpath.is_empty()).count(), 36);
    }
}
