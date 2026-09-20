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
pub mod progress;
pub mod report;
pub mod source;
#[cfg(test)]
mod testutil;
pub mod validate;
pub mod values;
pub(crate) mod xpath;

pub use container::{Container, ContainerError, EncryptionScheme, EntryInfo};
pub use detect::{detect, DetectError, Detected, SchemaVersion};
pub use known::{known_schema, KnownElement, KnownSchema};
pub use parse::{
    parse_installation, parse_project_info, ParseError, ParseOutput, UnknownConstruct, UnknownKind,
};
pub use progress::{ImportObserver, ImportStage};
pub use report::ImportReport;
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
    /// Manufacturer files (`<M-xxxx>/*`, `Baggages/*` — everything except
    /// `.signature` entries) on their way to the product database
    /// (ADR-0005). `knx-app` is the only crate that ingests these.
    pub manufacturer: Vec<opaque::ManufacturerFile>,
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
            .field(
                "manufacturer",
                &format!("{} files", self.manufacturer.len()),
            )
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
    import_knxproj_observed(path, &())
}

/// [`import_knxproj`] with somebody watching: `observer` is told which
/// stage is running as it runs (ADR-0023). The unobserved form above is
/// this one with `&()`, so there is one import implementation, not two.
pub fn import_knxproj_observed(
    path: &std::path::Path,
    observer: &dyn ImportObserver,
) -> Result<ImportOutcome, ImportFailure> {
    let bytes = std::fs::read(path).map_err(ImportFailure::Io)?;
    let file_name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or_default()
        .to_string();
    import_knxproj_bytes_observed(bytes, &file_name, observer)
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
    import_knxproj_bytes_observed(bytes, file_name, &())
}

/// [`import_knxproj_bytes`] with somebody watching. The `observer.stage(..)`
/// calls below are the definition of the import's stage list — ADR-0023's
/// rule is that each is announced *before* the work it names, so a caller
/// showing the label shows what is running, not what has just finished.
pub fn import_knxproj_bytes_observed(
    bytes: Vec<u8>,
    file_name: &str,
    observer: &dyn ImportObserver,
) -> Result<ImportOutcome, ImportFailure> {
    let file_size = bytes.len() as u64;
    observer.stage(ImportStage::OpenContainer);
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
    let topology_path = format!("{part}/0.xml");
    let info_path = format!("{part}/Project.xml");

    // Read once, used for both detection and parsing — `Container::read`
    // has no cache, so reading `0.xml` (typically the container's largest
    // entry) a second time would re-decompress it for no reason.
    let topology_bytes = container
        .read(&topology_path)
        .map_err(ImportFailure::Container)?;
    observer.stage(ImportStage::DetectSchema);
    let detected = detect::detect_from_bytes(&topology_bytes, &topology_path, &mut container)
        .map_err(ImportFailure::Detect)?;
    let schema = known_schema(detected.version.0).ok_or(ImportFailure::NoKnownSchemaTable {
        version: detected.version.0,
    })?;

    // Schema ≥21 replaced `0.xml`'s shape enough (`Segment`, module
    // references, `GroupObjectTree`) that Task 6's tolerant schema-11 parser
    // cannot walk it meaningfully — `parse_installation_v21` is its own
    // tolerant walker over the schema-≥21 known-element table (Task 6/7),
    // not a variant of `parse_installation`. Schema 11 keeps its original
    // parser untouched, per the plan's Global Constraints.
    observer.stage(ImportStage::ParseTopology);
    let mut parsed = if detected.version.0 >= 21 {
        parse::parse_installation_v21(&topology_bytes, &topology_path, schema)
    } else {
        parse_installation(&topology_bytes, &topology_path, schema)
    }
    .map_err(ImportFailure::Parse)?;

    observer.stage(ImportStage::ParseProjectInfo);
    let info_bytes = container
        .read(&info_path)
        .map_err(ImportFailure::Container)?;
    let (info, info_unknown) =
        parse_project_info(&info_bytes, &info_path, schema).map_err(ImportFailure::Parse)?;
    parsed.document.info = info;

    let mut unknown = parsed.unknown;
    unknown.extend(info_unknown);

    observer.stage(ImportStage::Validate);
    let validation = validate::validate(&parsed.document);
    observer.stage(ImportStage::Map);
    let mapped = map::map(&parsed.document, &topology_path);
    observer.stage(ImportStage::InferDatapointTypes);
    let inference = infer::infer_group_address_dpts(&mapped.project);

    observer.stage(ImportStage::CollectContainerEntries);
    let collected = opaque::collect_container_entries_observed(
        &mut container,
        &[topology_path.as_str(), info_path.as_str()],
        observer,
    )
    .map_err(ImportFailure::Container)?;
    let mut opaque_entries = collected.opaque;
    let manufacturer = collected.manufacturer;
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
                    // Re-keyed onto this line, the way the device subtrees
                    // below are re-keyed onto their device: the parser's own
                    // path names the element's shape, not which line it came
                    // from, so a project with two lines used to collapse both
                    // `BusAccess` elements into one entry ([`xpath`]). The
                    // element's position within the line (directly under it at
                    // schema 11, inside `Segment` at schema ≥21) is preserved
                    // by keeping whatever followed `/Line` in the parser's path.
                    let mut rekeyed = bus_access.clone();
                    let suffix = bus_access
                        .xpath
                        .split_once("/Line")
                        .map(|(_, rest)| rest.to_string())
                        .unwrap_or_else(|| "/BusAccess".to_string());
                    rekeyed.xpath = format!("{}{suffix}", xpath::line(&line.id));
                    opaque_entries.push(opaque::from_retained_element(&topology_path, &rekeyed));
                }
            }
        }
    }
    // Schema ≥21's `ModuleInstances`/`GroupObjectTree`/`Security` are known
    // but deliberately not modeled beyond raw retention (the plan's Global
    // Constraints — neither has a `knx_core` field), exactly like
    // `BusAccess` above, except per-device rather than per-line: a project
    // xpath fixed regardless of which device it came from would be wrong
    // the moment a project has more than one device (the same failure mode
    // `BusAccess`'s own fixed xpath would have for more than one line), so
    // each device's own `@Id` is folded into the xpath here. Schema 11
    // devices never set any of the three fields, so this loop is a no-op
    // for schema-11 imports.
    for installation in &parsed.document.installations {
        for area in &installation.areas {
            for line in &area.lines {
                for device in &line.devices {
                    let device_xpath = xpath::device_v21(&device.id);
                    if let Some(raw) = &device.module_instances_raw {
                        let mut r = raw.clone();
                        r.xpath = format!("{device_xpath}/ModuleInstances");
                        opaque_entries.push(opaque::from_retained_element(&topology_path, &r));
                    }
                    if let Some(raw) = &device.group_object_tree_raw {
                        let mut r = raw.clone();
                        r.xpath = format!("{device_xpath}/GroupObjectTree");
                        opaque_entries.push(opaque::from_retained_element(&topology_path, &r));
                    }
                    if let Some(raw) = &device.security_raw {
                        let mut r = raw.clone();
                        r.xpath = format!("{device_xpath}/Security");
                        opaque_entries.push(opaque::from_retained_element(&topology_path, &r));
                    }
                }
            }
        }
        // `unassigned_devices`: same, if schema ≥21 ever has any — untested,
        // since neither reference project's known-element table models
        // `UnassignedDevices` at schema ≥21 yet (a schema-11-only path
        // today), but handled defensively regardless, on the same principle
        // as the loop above.
        for device in &installation.unassigned_devices {
            let device_xpath = xpath::unassigned_device(&device.id);
            if let Some(raw) = &device.module_instances_raw {
                let mut r = raw.clone();
                r.xpath = format!("{device_xpath}/ModuleInstances");
                opaque_entries.push(opaque::from_retained_element(&topology_path, &r));
            }
            if let Some(raw) = &device.group_object_tree_raw {
                let mut r = raw.clone();
                r.xpath = format!("{device_xpath}/GroupObjectTree");
                opaque_entries.push(opaque::from_retained_element(&topology_path, &r));
            }
            if let Some(raw) = &device.security_raw {
                let mut r = raw.clone();
                r.xpath = format!("{device_xpath}/Security");
                opaque_entries.push(opaque::from_retained_element(&topology_path, &r));
            }
        }
    }
    // `ProjectTraces` (schema ≥21's audit log) sits on `Project.xml`/
    // `project.xml`, not `0.xml` — bypasses `MapOutput` entirely, same as
    // the loop above, since `ProjectInfo` has no field for it either.
    if let Some(raw) = &parsed.document.info.project_traces_raw {
        opaque_entries.push(opaque::from_retained_element(&info_path, raw));
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
        &manufacturer,
    );

    Ok(ImportOutcome {
        project: mapped.project,
        opaque: opaque_entries,
        manufacturer,
        report: import_report,
    })
}

#[cfg(test)]
mod import_tests {
    use super::*;
    use crate::testutil::{reference_ets4_path, reference_ets6_path, reference_kv_schema21_path};
    use knx_core::GroupAddressStyle;

    #[test]
    fn importing_the_reference_project_succeeds_with_a_clean_report() {
        if !crate::testutil::corpus_available() {
            eprintln!("skip: OriginalData/ corpus not present (gitignored, local-only)");
            return;
        }
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
    fn importing_the_kv_schema_21_project_succeeds_with_zero_unknown_constructs() {
        if !crate::testutil::corpus_available() {
            eprintln!("skip: OriginalData/ corpus not present (gitignored, local-only)");
            return;
        }
        let out = import_knxproj(&reference_kv_schema21_path()).unwrap();
        assert_eq!(out.report.source.schema_version, 21);
        assert_eq!(
            out.report.unknown,
            vec![],
            "known.rs's SCHEMA_21 table is incomplete"
        );
        assert_eq!(out.project.devices.iter().count(), 4);
        assert!(out.project.devices.module_instances().count() > 0);
    }

    #[test]
    fn importing_the_ets6_schema_23_project_succeeds_but_carries_no_round_trip_claim() {
        if !crate::testutil::corpus_available() {
            eprintln!("skip: OriginalData/ corpus not present (gitignored, local-only)");
            return;
        }
        // Not a re-export of the KV reference project — a genuinely
        // different installation (Session 7 evidence). Schema 23 imports
        // successfully (Task 3/7's known-element table and mapper already
        // cover it) but is not claimed round-trip-clean the way schema 21
        // is: see `report::build`'s schema-23 `unsupported` entry.
        let out = import_knxproj(&reference_ets6_path()).unwrap();
        assert_eq!(out.report.source.schema_version, 23);
    }

    #[test]
    fn the_opaque_entries_cover_every_container_entry_we_do_not_regenerate() {
        if !crate::testutil::corpus_available() {
            eprintln!("skip: OriginalData/ corpus not present (gitignored, local-only)");
            return;
        }
        let out = import_knxproj(&reference_ets4_path()).unwrap();
        // 36 whole-file entries, split between the opaque store and the
        // manufacturer files now handed out separately (Task 12).
        let opaque_whole_files = out.opaque.iter().filter(|e| e.xpath.is_empty()).count();
        assert_eq!(opaque_whole_files + out.manufacturer.len(), 36);
    }
}
