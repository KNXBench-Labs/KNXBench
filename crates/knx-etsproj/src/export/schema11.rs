//! Writes `knx_core::Project` back out as schema-11 `0.xml`/`Project.xml`.
//!
//! Writing rules (mirrors the reading rules exactly, in reverse):
//!
//! - A modeled attribute value comes from the model. A known-but-not-
//!   modeled one (`Installation/@BCUKey`, `@SplitType`,
//!   `ProjectInformation`'s tool-state attributes) comes from the opaque
//!   store's `RetainedAttribute` entries, matched on `(xpath, name)` — the
//!   same schema-shaped path [`crate::known`]'s table itself uses, not a
//!   per-instance one, since that is the granularity Task 6 captured it at.
//! - `Override::Value` is written only when its `Layer::is_exported()` is
//!   true; `Override::Empty` is written as an empty attribute; nothing is
//!   written for `Override::Absent`. Nothing carrying `Layer::Inferred`,
//!   `Layer::Program` or `Layer::ProgramRef` can reach the file — there is
//!   no branch that writes an unexported layer's value.
//! - Element and attribute order follows schema 11's own order (per
//!   [`crate::known::SCHEMA_11`]), so a reader that depends on order is not
//!   given a reason to fail. Byte equality with the original file is
//!   neither attempted nor claimed.
//!
//! `KNX/@CreatedBy`/`@ToolVersion` are a deliberate exception to "value
//! comes from the model": `knx_core::Project` carries neither (Session 2's
//! domain model has no field for them, and Task 6's parser takes them
//! specially into `SourceDocument::created_by`/`tool_version`, which never
//! reaches `MapOutput` at all — genuinely unrecoverable from `Project`
//! alone). Rather than inventing knx_core's original values into
//! `RetainedAttribute`-shaped plumbing this task's own interface does not
//! ask for, this writer states its own honest authorship: a file this
//! application exports says a knx-etsproj tool wrote it, not that ETS did.

use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use quick_xml::events::{BytesDecl, BytesEnd, BytesStart, Event};
use quick_xml::Writer;

use knx_core::{
    Area, BuildingPart, BuildingPartType, ComObjectInstance, CompletionStatus, DeviceInstance,
    Direction, DptRef, GroupAddressEntry, GroupAddressStyle, GroupRange, Installation, Language,
    Line, Override, ParameterInstance, Project, StringTable, Text,
};

use crate::opaque::{OpaqueEntry, OpaqueKind};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExportError {
    Xml(String),
    MissingProjectId,
    UnsupportedSchemaVersion(u32),
}

impl std::fmt::Display for ExportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExportError::Xml(e) => write!(f, "{e}"),
            ExportError::MissingProjectId => {
                write!(f, "project has no Project/@Id; cannot export")
            }
            ExportError::UnsupportedSchemaVersion(v) => {
                write!(f, "no schema-{v} writer implemented")
            }
        }
    }
}

impl std::error::Error for ExportError {}

/// The tool identity this exporter writes into `KNX/@CreatedBy`/
/// `@ToolVersion` — see the module doc comment for why this is not the
/// original ETS tool identity.
const EXPORTER_NAME: &str = "knx-etsproj";
const EXPORTER_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Known-but-not-modeled attributes, keyed by the schema-shaped
/// `(xpath, name)` [`crate::known`] itself uses.
type RetainedAttrs = BTreeMap<(String, String), String>;

fn retained_attrs(opaque: &[OpaqueEntry]) -> RetainedAttrs {
    opaque
        .iter()
        .filter(|e| e.kind == OpaqueKind::RetainedAttribute)
        .map(|e| {
            (
                (e.xpath.clone(), e.name.clone()),
                String::from_utf8_lossy(&e.bytes).into_owned(),
            )
        })
        .collect()
}

/// Retained elements ([`crate::source::RetainedElement`], raw XML bytes),
/// keyed by their own schema-shaped xpath. Only `BusAccess` uses this path
/// today.
fn retained_elements(opaque: &[OpaqueEntry]) -> BTreeMap<String, Vec<u8>> {
    opaque
        .iter()
        .filter(|e| e.kind == OpaqueKind::RetainedElement)
        .map(|e| (e.xpath.clone(), e.bytes.clone()))
        .collect()
}

/// Accumulates one element's attributes in schema order, filling gaps from
/// the retained-attribute store last.
struct Attrs(Vec<(String, String)>);

impl Attrs {
    fn new() -> Self {
        Self(Vec::new())
    }

    fn push(&mut self, name: &str, value: impl Into<String>) -> &mut Self {
        self.0.push((name.to_string(), value.into()));
        self
    }

    fn opt(&mut self, name: &str, value: &Option<String>) -> &mut Self {
        if let Some(v) = value {
            self.push(name, v.clone());
        }
        self
    }

    fn opt_display(&mut self, name: &str, value: Option<impl std::fmt::Display>) -> &mut Self {
        if let Some(v) = value {
            self.push(name, v.to_string());
        }
        self
    }

    fn required_bool(&mut self, name: &str, value: bool) -> &mut Self {
        self.push(name, if value { "1" } else { "0" })
    }

    fn opt_bool(&mut self, name: &str, value: Option<bool>) -> &mut Self {
        if let Some(v) = value {
            self.required_bool(name, v);
        }
        self
    }

    fn has(&self, name: &str) -> bool {
        self.0.iter().any(|(n, _)| n == name)
    }

    /// Fills every attribute this element's schema-shaped `xpath` has in
    /// the retained store, skipping any name the model already wrote.
    fn fill_retained(&mut self, retained: &RetainedAttrs, xpath: &str) -> &mut Self {
        for ((x, name), value) in retained {
            if x == xpath && !self.has(name) {
                self.push(name, value.clone());
            }
        }
        self
    }
}

/// `Override::Value` writes only when `is_exported()`; `Override::Empty`
/// writes an empty attribute; `Override::Absent` writes nothing.
/// `ReadFlag`/`WriteFlag`/`TransmitFlag`/`UpdateFlag`/`CommunicationFlag`
/// spell their booleans `"Enabled"`/`"Disabled"` on schema 11 — measured,
/// RESEARCH §3.3's amendment — never `"1"`/`"0"`, which is every other
/// schema-11 boolean attribute's spelling.
fn push_override_flag(attrs: &mut Attrs, name: &str, o: &Override<bool>) {
    match o {
        Override::Absent => {}
        Override::Empty => {
            attrs.push(name, "");
        }
        Override::Value(r) if r.layer.is_exported() => {
            attrs.push(name, if r.value { "Enabled" } else { "Disabled" });
        }
        Override::Value(_) => {}
    }
}

fn push_override_dpt(attrs: &mut Attrs, name: &str, o: &Override<DptRef>) {
    match o {
        Override::Absent => {}
        Override::Empty => {
            attrs.push(name, "");
        }
        Override::Value(r) if r.layer.is_exported() => {
            attrs.push(name, r.value.to_string());
        }
        Override::Value(_) => {}
    }
}

/// `Text::Literal` (the only form this session's own import ever produces
/// — schema 11's instance-level `@Text`/`@Description` are literal, not
/// translated, per [`knx_core::string_table`]) resolves without the table.
/// A `Text::Localized` value can in principle only arrive here from a
/// future application-program import (Session 4) at `Layer::Program`/
/// `ProgramRef`, which `is_exported()` already excludes — the `strings`
/// lookup below is defensive completeness, not a path this session's data
/// exercises.
fn push_override_text(attrs: &mut Attrs, name: &str, o: &Override<Text>, strings: &StringTable) {
    match o {
        Override::Absent => {}
        Override::Empty => {
            attrs.push(name, "");
        }
        Override::Value(r) if r.layer.is_exported() => {
            let language = Language("en".to_string());
            let text = strings.text(&r.value, &language).unwrap_or_default();
            attrs.push(name, text);
        }
        Override::Value(_) => {}
    }
}

fn completion_status_str(c: CompletionStatus) -> &'static str {
    match c {
        CompletionStatus::Undefined => "Undefined",
        CompletionStatus::Editing => "Editing",
        CompletionStatus::FinishedDesign => "FinishedDesign",
        CompletionStatus::Accepted => "Accepted",
    }
}

fn group_address_style_str(s: GroupAddressStyle) -> &'static str {
    match s {
        GroupAddressStyle::Free => "Free",
        GroupAddressStyle::TwoLevel => "TwoLevel",
        GroupAddressStyle::ThreeLevel => "ThreeLevel",
    }
}

fn building_part_type_str(k: BuildingPartType) -> &'static str {
    match k {
        BuildingPartType::Building => "Building",
        BuildingPartType::Floor => "Floor",
        BuildingPartType::Room => "Room",
        BuildingPartType::Corridor => "Corridor",
        BuildingPartType::DistributionBoard => "DistributionBoard",
        BuildingPartType::BuildingPart => "BuildingPart",
    }
}

/// Schema 11's own naive-local timestamp form (`values::parse_timestamp`'s
/// counterpart): the internal `DateTime<Utc>` is treated as if its clock
/// reading were already local, per that function's documented assumption,
/// so formatting drops the offset rather than converting through it.
fn format_timestamp(dt: DateTime<Utc>) -> String {
    dt.format("%Y-%m-%dT%H:%M:%S").to_string()
}

// `Writer<Vec<u8>>::write_event` returns `std::io::Result`, not
// `quick_xml::Result`: `Vec<u8>` is a plain `std::io::Write`, so quick-xml
// reports write failures through `io::Error`, not its own `Error` type
// (which `write_raw_element`'s `Reader::read_event` still uses).
fn xml_err(e: std::io::Error) -> ExportError {
    ExportError::Xml(e.to_string())
}

fn open(writer: &mut Writer<Vec<u8>>, name: &str, attrs: &Attrs) -> Result<(), ExportError> {
    let mut elem = BytesStart::new(name);
    for (k, v) in &attrs.0 {
        elem.push_attribute((k.as_str(), v.as_str()));
    }
    writer.write_event(Event::Start(elem)).map_err(xml_err)
}

fn empty(writer: &mut Writer<Vec<u8>>, name: &str, attrs: &Attrs) -> Result<(), ExportError> {
    let mut elem = BytesStart::new(name);
    for (k, v) in &attrs.0 {
        elem.push_attribute((k.as_str(), v.as_str()));
    }
    writer.write_event(Event::Empty(elem)).map_err(xml_err)
}

fn close(writer: &mut Writer<Vec<u8>>, name: &str) -> Result<(), ExportError> {
    writer
        .write_event(Event::End(BytesEnd::new(name)))
        .map_err(xml_err)
}

/// Re-emits one retained element's raw bytes (a single, already
/// well-formed `Start`/`Empty` event, captured verbatim by Task 6) through
/// the same writer that is producing everything else, so it participates
/// in the same indentation instead of being spliced in as an untouched
/// byte run.
fn write_raw_element(writer: &mut Writer<Vec<u8>>, raw: &[u8]) -> Result<(), ExportError> {
    let mut reader = quick_xml::Reader::from_reader(raw);
    let event = reader
        .read_event()
        .map_err(|e| ExportError::Xml(e.to_string()))?;
    writer.write_event(event).map_err(xml_err)
}

pub fn write_installation_xml(
    project: &Project,
    opaque: &[OpaqueEntry],
) -> Result<Vec<u8>, ExportError> {
    if project.info.project_id.is_empty() {
        return Err(ExportError::MissingProjectId);
    }
    let retained = retained_attrs(opaque);
    let elements = retained_elements(opaque);

    let mut writer = Writer::new_with_indent(Vec::new(), b' ', 2);
    writer
        .write_event(Event::Decl(BytesDecl::new("1.0", Some("utf-8"), None)))
        .map_err(xml_err)?;

    let mut knx_attrs = Attrs::new();
    knx_attrs
        .push("xmlns:xsi", "http://www.w3.org/2001/XMLSchema-instance")
        .push("xmlns:xsd", "http://www.w3.org/2001/XMLSchema")
        .push("CreatedBy", EXPORTER_NAME)
        .push("ToolVersion", EXPORTER_VERSION)
        .push("xmlns", "http://knx.org/xml/project/11");
    open(&mut writer, "KNX", &knx_attrs)?;

    let mut project_attrs = Attrs::new();
    project_attrs.push("Id", project.info.project_id.clone());
    open(&mut writer, "Project", &project_attrs)?;

    open(&mut writer, "Installations", &Attrs::new())?;
    for installation in &project.installations {
        write_installation(&mut writer, project, installation, &retained, &elements)?;
    }
    close(&mut writer, "Installations")?;

    close(&mut writer, "Project")?;
    close(&mut writer, "KNX")?;

    Ok(writer.into_inner())
}

pub fn write_project_xml(
    project: &Project,
    opaque: &[OpaqueEntry],
) -> Result<Vec<u8>, ExportError> {
    if project.info.project_id.is_empty() {
        return Err(ExportError::MissingProjectId);
    }
    let retained = retained_attrs(opaque);

    let mut writer = Writer::new_with_indent(Vec::new(), b' ', 2);
    writer
        .write_event(Event::Decl(BytesDecl::new("1.0", Some("utf-8"), None)))
        .map_err(xml_err)?;

    let mut knx_attrs = Attrs::new();
    knx_attrs
        .push("xmlns:xsi", "http://www.w3.org/2001/XMLSchema-instance")
        .push("xmlns:xsd", "http://www.w3.org/2001/XMLSchema")
        .push("CreatedBy", EXPORTER_NAME)
        .push("ToolVersion", EXPORTER_VERSION)
        .push("xmlns", "http://knx.org/xml/project/11");
    open(&mut writer, "KNX", &knx_attrs)?;

    let mut project_attrs = Attrs::new();
    project_attrs.push("Id", project.info.project_id.clone());
    open(&mut writer, "Project", &project_attrs)?;

    let xpath = "/KNX/Project/ProjectInformation";
    let mut info_attrs = Attrs::new();
    info_attrs.push("Name", project.info.name.clone());
    info_attrs.opt_display(
        "LastModified",
        project.info.last_modified.map(format_timestamp),
    );
    info_attrs.opt_display(
        "ProjectStart",
        project.info.project_start.map(format_timestamp),
    );
    info_attrs.opt("ProjectId", &project.info.project_number);
    info_attrs.fill_retained(&retained, xpath);
    info_attrs.push(
        "GroupAddressStyle",
        group_address_style_str(project.info.group_address_style),
    );
    info_attrs.push(
        "CompletionStatus",
        completion_status_str(project.info.completion),
    );
    empty(&mut writer, "ProjectInformation", &info_attrs)?;

    close(&mut writer, "Project")?;
    close(&mut writer, "KNX")?;

    Ok(writer.into_inner())
}

fn write_installation(
    writer: &mut Writer<Vec<u8>>,
    project: &Project,
    installation: &Installation,
    retained: &RetainedAttrs,
    elements: &BTreeMap<String, Vec<u8>>,
) -> Result<(), ExportError> {
    let xpath = "/KNX/Project/Installations/Installation";
    let default_line_ets_id = installation
        .default_line
        .and_then(|id| installation.topology.line(id))
        .map(|l| l.source.ets_id.clone());

    let mut attrs = Attrs::new();
    attrs.push("InstallationId", installation.id.0.to_string());
    attrs.push("Name", installation.name.clone());
    attrs.fill_retained(retained, xpath);
    attrs.opt("DefaultLine", &default_line_ets_id);
    attrs.opt_display("IPRoutingMulticastAddress", installation.multicast_address);
    attrs.push(
        "CompletionStatus",
        completion_status_str(installation.completion),
    );
    open(writer, "Installation", &attrs)?;

    open(writer, "Topology", &Attrs::new())?;
    for area in &installation.topology.areas {
        write_area(writer, project, &installation.topology, area, elements)?;
    }
    if !installation.topology.unassigned.is_empty() {
        open(writer, "UnassignedDevices", &Attrs::new())?;
        for &device_id in &installation.topology.unassigned {
            if let Some(device) = project.devices.get(device_id) {
                write_device(writer, project, installation, device)?;
            }
        }
        close(writer, "UnassignedDevices")?;
    }
    close(writer, "Topology")?;

    if !installation.buildings.is_empty() {
        open(writer, "Buildings", &Attrs::new())?;
        let by_id: BTreeMap<_, _> = installation.buildings.iter().map(|b| (b.id, b)).collect();
        for part in installation.buildings.iter().filter(|b| b.parent.is_none()) {
            write_building_part(writer, project, installation, &by_id, part)?;
        }
        close(writer, "Buildings")?;
    }

    open(writer, "GroupAddresses", &Attrs::new())?;
    open(writer, "GroupRanges", &Attrs::new())?;
    let range_by_id: BTreeMap<_, _> = installation
        .group_ranges
        .iter()
        .map(|r| (r.id, r))
        .collect();
    for range in installation
        .group_ranges
        .iter()
        .filter(|r| r.parent.is_none())
    {
        write_group_range(writer, &range_by_id, &installation.group_addresses, range)?;
    }
    close(writer, "GroupRanges")?;
    close(writer, "GroupAddresses")?;

    close(writer, "Installation")?;
    Ok(())
}

fn write_area(
    writer: &mut Writer<Vec<u8>>,
    project: &Project,
    topology: &knx_core::Topology,
    area: &Area,
    elements: &BTreeMap<String, Vec<u8>>,
) -> Result<(), ExportError> {
    let mut attrs = Attrs::new();
    attrs.push("Id", area.source.ets_id.clone());
    attrs.push("Name", area.name.clone());
    attrs.push("Address", area.address.to_string());
    attrs.push("CompletionStatus", completion_status_str(area.completion));
    open(writer, "Area", &attrs)?;

    for &line_id in &area.lines {
        if let Some(line) = topology.line(line_id) {
            write_line(writer, project, line, elements)?;
        }
    }

    close(writer, "Area")?;
    Ok(())
}

fn write_line(
    writer: &mut Writer<Vec<u8>>,
    project: &Project,
    line: &Line,
    elements: &BTreeMap<String, Vec<u8>>,
) -> Result<(), ExportError> {
    let mut attrs = Attrs::new();
    attrs.push("Id", line.source.ets_id.clone());
    attrs.push("Name", line.name.clone());
    attrs.push("Address", line.address.to_string());
    attrs.push("MediumTypeRefId", line.medium_ref.clone());
    attrs.opt("DomainAddress", &line.domain_address);
    attrs.opt_bool("DomainAddressIsChecked", line.domain_address_is_checked);
    attrs.push("CompletionStatus", completion_status_str(line.completion));
    attrs.opt_display(
        "IPRoutingMulticastAddress",
        line.ip_routing_multicast_address,
    );
    attrs.opt_display("MulticastTTL", line.multicast_ttl);
    open(writer, "Line", &attrs)?;

    for &device_id in &line.devices {
        // `installation` here is only needed for its `.parameters` list;
        // devices under a `Line` need the same lookup as unassigned ones,
        // so this takes it via the caller passing `project` and resolving
        // parameters per-device instead (see `write_device`'s signature).
        if let Some(device) = project.devices.get(device_id) {
            write_device_with_params(writer, project, device)?;
        }
    }

    let bus_access_xpath = "/KNX/Project/Installations/Installation/Topology/Area/Line/BusAccess";
    if let Some(raw) = elements.get(bus_access_xpath) {
        write_raw_element(writer, raw)?;
    }

    close(writer, "Line")?;
    Ok(())
}

/// Devices are looked up by id from two call sites (`Line`, directly, and
/// `Installation` for unassigned ones) but both need every installation's
/// `parameters` to find the ones belonging to this device — a lookup that
/// is cheap enough to redo per device (a handful of installations, at most
/// a few thousand parameters) rather than threading a prebuilt map through
/// every caller for a one-installation reference project.
fn write_device(
    writer: &mut Writer<Vec<u8>>,
    project: &Project,
    installation: &Installation,
    device: &DeviceInstance,
) -> Result<(), ExportError> {
    write_device_inner(writer, project, &installation.parameters, device)
}

fn write_device_with_params(
    writer: &mut Writer<Vec<u8>>,
    project: &Project,
    device: &DeviceInstance,
) -> Result<(), ExportError> {
    // `Line`'s own caller (`write_area`) does not carry `Installation`, only
    // `Topology` — parameters live on `Installation`, so this path resolves
    // them by scanning every installation once. Only ever one installation
    // in the reference project; correct, if not maximally efficient, for
    // more than one.
    let parameters: Vec<&ParameterInstance> = project
        .installations
        .iter()
        .flat_map(|i| &i.parameters)
        .filter(|p| p.device == device.id)
        .collect();
    write_device_inner(writer, project, parameters, device)
}

fn write_device_inner<'a>(
    writer: &mut Writer<Vec<u8>>,
    project: &Project,
    all_parameters: impl IntoIterator<Item = &'a ParameterInstance>,
    device: &DeviceInstance,
) -> Result<(), ExportError> {
    let mut attrs = Attrs::new();
    attrs.push("Id", device.source.ets_id.clone());
    attrs.push("Name", device.name.clone());
    attrs.opt("Description", &device.description);
    if let Some(address) = device.address {
        attrs.push("Address", address.device().to_string());
    }
    attrs.push("ProductRefId", device.product_ref.clone());
    attrs.push("Hardware2ProgramRefId", device.program_ref.clone());
    attrs.opt_display(
        "LastModified",
        device.commissioning.last_modified.map(format_timestamp),
    );
    attrs.opt_display(
        "LastDownload",
        device.commissioning.last_download.map(format_timestamp),
    );
    attrs.push(
        "CompletionStatus",
        completion_status_str(device.commissioning.completion),
    );
    attrs.required_bool(
        "IndividualAddressLoaded",
        device.commissioning.individual_address_loaded,
    );
    attrs.required_bool(
        "ApplicationProgramLoaded",
        device.commissioning.application_program_loaded,
    );
    attrs.required_bool("ParametersLoaded", device.commissioning.parameters_loaded);
    attrs.required_bool(
        "CommunicationPartLoaded",
        device.commissioning.communication_part_loaded,
    );
    attrs.required_bool(
        "MediumConfigLoaded",
        device.commissioning.medium_config_loaded,
    );
    attrs.required_bool(
        "IsCommunicationObjectVisibilityCalculated",
        device.visibility_calculated,
    );
    attrs.required_bool("Broken", device.commissioning.broken);

    let params: Vec<&ParameterInstance> = all_parameters.into_iter().collect();
    let has_children =
        !params.is_empty() || !device.com_objects.is_empty() || !device.binary_data.is_empty();

    if !has_children {
        empty(writer, "DeviceInstance", &attrs)?;
        return Ok(());
    }
    open(writer, "DeviceInstance", &attrs)?;

    if !params.is_empty() {
        open(writer, "ParameterInstanceRefs", &Attrs::new())?;
        for param in params {
            let mut p = Attrs::new();
            p.push("RefId", param.source.ets_id.clone());
            p.push("Value", param.raw.clone());
            empty(writer, "ParameterInstanceRef", &p)?;
        }
        close(writer, "ParameterInstanceRefs")?;
    }

    if !device.com_objects.is_empty() {
        open(writer, "ComObjectInstanceRefs", &Attrs::new())?;
        for &com_id in &device.com_objects {
            if let Some(com) = project.devices.com_object(com_id) {
                write_com_object(writer, project, com)?;
            }
        }
        close(writer, "ComObjectInstanceRefs")?;
    }

    if !device.binary_data.is_empty() {
        open(writer, "BinaryData", &Attrs::new())?;
        for b in &device.binary_data {
            let mut a = Attrs::new();
            a.push("Id", b.id.clone());
            a.push("Name", b.name.clone());
            empty(writer, "BinaryData", &a)?;
        }
        close(writer, "BinaryData")?;
    }

    close(writer, "DeviceInstance")?;
    Ok(())
}

fn write_com_object(
    writer: &mut Writer<Vec<u8>>,
    project: &Project,
    com: &ComObjectInstance,
) -> Result<(), ExportError> {
    let mut attrs = Attrs::new();
    attrs.push("RefId", com.source.ets_id.clone());
    attrs.required_bool("IsActive", com.is_active);
    push_override_dpt(&mut attrs, "DatapointType", &com.dpt);
    push_override_text(&mut attrs, "Text", &com.text, &project.strings);
    push_override_text(
        &mut attrs,
        "Description",
        &com.description,
        &project.strings,
    );
    push_override_flag(&mut attrs, "ReadFlag", &com.flags.read);
    push_override_flag(&mut attrs, "WriteFlag", &com.flags.write);
    push_override_flag(&mut attrs, "TransmitFlag", &com.flags.transmit);
    push_override_flag(&mut attrs, "UpdateFlag", &com.flags.update);
    push_override_flag(&mut attrs, "CommunicationFlag", &com.flags.communication);

    if com.links.is_empty() {
        empty(writer, "ComObjectInstanceRef", &attrs)?;
        return Ok(());
    }
    open(writer, "ComObjectInstanceRef", &attrs)?;
    open(writer, "Connectors", &Attrs::new())?;
    for link in &com.links {
        let name = match link.direction {
            Direction::Send => "Send",
            Direction::Receive => "Receive",
        };
        let ga_ets_id = project
            .installations
            .iter()
            .flat_map(|i| &i.group_addresses)
            .find(|g| g.id == link.ga)
            .map(|g| g.source.ets_id.clone());
        if let Some(ets_id) = ga_ets_id {
            let mut a = Attrs::new();
            a.push("GroupAddressRefId", ets_id);
            empty(writer, name, &a)?;
        }
    }
    close(writer, "Connectors")?;
    close(writer, "ComObjectInstanceRef")?;
    Ok(())
}

fn write_building_part(
    writer: &mut Writer<Vec<u8>>,
    project: &Project,
    installation: &Installation,
    by_id: &BTreeMap<knx_core::BuildingPartId, &BuildingPart>,
    part: &BuildingPart,
) -> Result<(), ExportError> {
    let default_line_ets_id = part
        .default_line
        .and_then(|id| installation.topology.line(id))
        .map(|l| l.source.ets_id.clone());

    let mut attrs = Attrs::new();
    attrs.push("Id", part.source.ets_id.clone());
    attrs.push("Name", part.name.clone());
    attrs.opt("Number", &part.number);
    attrs.push("Type", building_part_type_str(part.kind));
    attrs.opt("DefaultLine", &default_line_ets_id);
    attrs.push("CompletionStatus", completion_status_str(part.completion));

    let has_children = !part.children.is_empty() || !part.devices.is_empty();
    if !has_children {
        empty(writer, "BuildingPart", &attrs)?;
        return Ok(());
    }
    open(writer, "BuildingPart", &attrs)?;
    for &child_id in &part.children {
        if let Some(child) = by_id.get(&child_id) {
            write_building_part(writer, project, installation, by_id, child)?;
        }
    }
    for &device_id in &part.devices {
        if let Some(device) = project.devices.get(device_id) {
            let mut a = Attrs::new();
            a.push("RefId", device.source.ets_id.clone());
            empty(writer, "DeviceInstanceRef", &a)?;
        }
    }
    close(writer, "BuildingPart")?;
    Ok(())
}

fn write_group_range(
    writer: &mut Writer<Vec<u8>>,
    by_id: &BTreeMap<knx_core::GroupRangeId, &GroupRange>,
    addresses: &[GroupAddressEntry],
    range: &GroupRange,
) -> Result<(), ExportError> {
    let mut attrs = Attrs::new();
    attrs.push("Id", range.source.ets_id.clone());
    attrs.push("Name", range.name.clone());
    attrs.push("RangeStart", range.start.raw().to_string());
    attrs.push("RangeEnd", range.end.raw().to_string());
    open(writer, "GroupRange", &attrs)?;

    for &child_id in &range.children {
        if let Some(child) = by_id.get(&child_id) {
            write_group_range(writer, by_id, addresses, child)?;
        }
    }
    for ga in addresses.iter().filter(|g| g.range == Some(range.id)) {
        let mut a = Attrs::new();
        a.push("Id", ga.source.ets_id.clone());
        a.push("Address", ga.address.raw().to_string());
        a.push("Name", ga.name.clone());
        a.required_bool("Central", ga.central);
        a.required_bool("Unfiltered", ga.unfiltered);
        empty(writer, "GroupAddress", &a)?;
    }

    close(writer, "GroupRange")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::known::known_schema;
    use crate::parse::parse_installation;
    use crate::testutil::reference_ets4_path;
    use knx_core::{provenance::Resolved, Layer};

    #[test]
    fn an_exported_installation_is_well_formed_and_carries_the_namespace() {
        let out = crate::import_knxproj(&reference_ets4_path()).unwrap();
        let xml = write_installation_xml(&out.project, &out.opaque).unwrap();
        let text = String::from_utf8(xml).unwrap();
        assert!(text.contains(r#"xmlns="http://knx.org/xml/project/11""#));
        assert!(text.contains(r#"<Project Id="P-0512">"#));
        // It parses back with our own parser, which is the only reader we control.
        parse_installation(text.as_bytes(), "P-0512/0.xml", known_schema(11).unwrap()).unwrap();
    }

    #[test]
    fn an_empty_override_is_written_as_an_empty_attribute() {
        let out = crate::import_knxproj(&reference_ets4_path()).unwrap();
        let xml =
            String::from_utf8(write_installation_xml(&out.project, &out.opaque).unwrap()).unwrap();
        assert_eq!(xml.matches(r#"DatapointType="""#).count(), 497);
    }

    #[test]
    fn retained_attributes_come_back_on_their_own_elements() {
        let out = crate::import_knxproj(&reference_ets4_path()).unwrap();
        let xml =
            String::from_utf8(write_installation_xml(&out.project, &out.opaque).unwrap()).unwrap();
        assert!(xml.contains(r#"BCUKey="4294967295""#));
        assert!(xml.contains(r#"Name='GATEWAY-NAME';IpAddr='192.0.2.1'"#));
    }

    #[test]
    fn an_inferred_value_is_never_written() {
        let mut out = crate::import_knxproj(&reference_ets4_path()).unwrap();
        // Force an inferred datapoint type onto an instance that had none.
        let id = *out
            .project
            .devices
            .iter()
            .next()
            .unwrap()
            .com_objects
            .first()
            .unwrap();
        out.project.devices.com_object_mut(id).unwrap().dpt = Override::Value(Resolved {
            value: DptRef {
                main: 99,
                sub: Some(99),
            },
            layer: Layer::Inferred,
        });
        let xml =
            String::from_utf8(write_installation_xml(&out.project, &out.opaque).unwrap()).unwrap();
        assert!(!xml.contains("DPST-99-99"));
    }

    #[test]
    fn the_project_xml_carries_the_group_address_style() {
        let out = crate::import_knxproj(&reference_ets4_path()).unwrap();
        let xml = String::from_utf8(write_project_xml(&out.project, &out.opaque).unwrap()).unwrap();
        assert!(xml.contains(r#"GroupAddressStyle="ThreeLevel""#));
        assert!(xml.contains(r#"Name="Unser Zuhause""#));
    }
}
