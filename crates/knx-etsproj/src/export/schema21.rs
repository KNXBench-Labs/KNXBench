//! Writes `knx_core::Project` back out as schema-≥21 `0.xml`/`Project.xml`
//! (`ets_schema_version >= 21`) — a sibling of [`super::schema11`], not a
//! variant of it (ADR-0014's textually-separate-parser precedent). Reuses
//! [`super::schema11`]'s helpers (`Attrs`, `retained_attrs`/
//! `retained_elements`, `push_override_dpt`/`push_override_text`,
//! `open`/`empty`/`close`, `write_group_range`, …) rather than
//! reimplementing them: `GroupRanges`/`GroupRange`/`GroupAddress` are the
//! same element and attribute shapes on both schemas, so `write_group_range`
//! is called unmodified. `write_raw_element` is *not* reused: it re-emits
//! exactly one XML event, correct for `BusAccess` (always a single
//! self-closing tag in every sample measured) but wrong for
//! `ModuleInstances`/`GroupObjectTree`/`ProjectTraces`, which nest real
//! children — this module's own `write_raw_subtree` below re-emits every
//! event in the captured range instead.
//!
//! Three structural deltas from schema 11, mirroring the parser's own
//! (`installation_v21.rs`'s module doc):
//! - `Segment` is re-synthesized between `Line` and `DeviceInstance`,
//!   carrying the medium/domain-address attributes schema 11 puts directly
//!   on `Line` (the same fields, moved — see `SourceLine`'s own doc
//!   comment).
//! - `Locations`/`Space` replaces `Buildings`/`BuildingPart` — the same
//!   `knx_core::BuildingPart` tree, different element names.
//! - `ComObjectInstanceRef` is flat (`Links`/`ChannelId` attributes, no
//!   nested `Connectors`).
//!
//! `ModuleInstances`, `GroupObjectTree` and `Security` were never modeled
//! beyond raw retention (the plan's Global Constraints), so they are
//! spliced back in verbatim from `opaque`'s `RetainedElement` entries, at
//! the disambiguated per-device xpath `lib.rs`'s import loop gives them
//! (`.../DeviceInstance[@Id='<id>']/<ElementName>`) — a fixed,
//! non-disambiguated xpath (as `BusAccess` uses) would silently overwrite
//! one device's blob with another's the moment a project has more than one
//! device, which every real schema-≥21 sample does.
//!
//! **Known-but-unmapped attributes that are *not* reconstructed on export,
//! and why:** `crate::known`'s tables list several attributes with no
//! dedicated `SourceDevice`/`SourceLine` field (`DeviceInstance`'s
//! `Comment`/`SerialNumber`/`IsActivityCalculated`/`LastUsedAPDULength`/
//! `ReadMaxAPDULength`/`Puid`; `Segment`'s own `Id`/`Number`/`Puid`; `Puid`
//! generally, on every element that carries it). `map.rs` (Task 6) folds
//! all of these into one project-wide `Vec<RetainedAttribute>`, keyed only
//! by their schema-shaped xpath (e.g. every device's `Comment` collapses to
//! the single key `(".../DeviceInstance", "Comment")`) — the same
//! granularity `schema11.rs`'s own module doc already documents and
//! accepts for `Installation/@BCUKey`-style attributes. For an attribute
//! that only ever occurs once per project (`ProjectInformation`'s
//! `Comment`/`Guid`/`LastUsedPuid`/`ProjectType`; `Installation`'s
//! `BCUKey`/`IPRoutingLatencyTolerance`, assuming one installation) that
//! granularity loses nothing. For one that occurs once *per device* or
//! *per line* — confirmed against `KV v2.5 - demo.knxproj`: all 4 devices
//! carry a distinct `SerialNumber` and `Puid` — reconstructing it from that
//! single collapsed key would splice one device's real hardware serial
//! number onto every other device, a silent *corruption*, not a loss.
//! Between writing nothing and writing something actively wrong, this
//! writer always writes nothing for these; see `KNOWN_LIMITATIONS.md` for
//! the tracked gap and the fix it needs (per-instance xpaths in Task 6's
//! parser, out of this task's scope). `ComObjectInstanceRef/@ChannelId` is
//! the one known-but-unmapped attribute this writer *does* reconstruct,
//! because its retained xpath already embeds both the owning device's and
//! the object's own id (`map_com_object_v21`'s own xpath, unchanged here),
//! making it unambiguous even across many devices.
//!
//! Booleans: measured directly against `KV v2.5 - demo.knxproj`,
//! `DeviceInstance`'s loaded-state flags spell `"true"`/`"false"`, not
//! schema 11's `"1"`/`"0"`. `Segment/@DomainAddressIsChecked` and
//! `ComObjectInstanceRef/@IsActive` are not present anywhere in that sample
//! project to measure directly; this writer spells them `"true"`/`"false"`
//! too, for consistency with what *is* measured, not because both have
//! been independently confirmed.
//!
//! Timestamps: the sample spells them as RFC 3339 instants with fractional
//! seconds (`"2023-01-13T11:07:21.5596726Z"`), unlike schema 11's naive
//! local form — `values::parse_timestamp` already accepts both, so this
//! writer emits the schema-≥21 form via `chrono`'s own RFC 3339 writer.

use std::collections::BTreeMap;

use chrono::{DateTime, SecondsFormat, Utc};
use quick_xml::events::{BytesDecl, Event};
use quick_xml::Writer;

use knx_core::{
    Area, BuildingPart, ComObjectInstance, DeviceInstance, GroupAddressId, Installation, Line,
    ParameterInstance, Project, Topology,
};

use super::schema11::{
    building_part_type_str, close, empty, group_address_style_str, open, push_override_dpt,
    push_override_text, reject_unranged_group_addresses, retained_attrs, retained_elements,
    write_group_range, xml_err, Attrs, RetainedAttrs, EXPORTER_NAME, EXPORTER_VERSION,
};
use super::ExportError;
use crate::opaque::OpaqueEntry;

pub fn write_installation_xml_v21(
    project: &Project,
    opaque: &[OpaqueEntry],
) -> Result<Vec<u8>, ExportError> {
    if project.info.project_id.is_empty() {
        return Err(ExportError::MissingProjectId);
    }
    reject_unranged_group_addresses(project)?;
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
        .push(
            "xmlns",
            format!(
                "http://knx.org/xml/project/{}",
                project.info.ets_schema_version
            ),
        );
    open(&mut writer, "KNX", &knx_attrs)?;

    let mut project_attrs = Attrs::new();
    project_attrs.push("Id", project.info.project_id.clone());
    open(&mut writer, "Project", &project_attrs)?;

    open(&mut writer, "Installations", &Attrs::new())?;
    for installation in &project.installations {
        write_installation_v21(&mut writer, project, installation, &retained, &elements)?;
    }
    close(&mut writer, "Installations")?;

    close(&mut writer, "Project")?;
    close(&mut writer, "KNX")?;

    Ok(writer.into_inner())
}

pub fn write_project_xml_v21(
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
        .push(
            "xmlns",
            format!(
                "http://knx.org/xml/project/{}",
                project.info.ets_schema_version
            ),
        );
    open(&mut writer, "KNX", &knx_attrs)?;

    let mut project_attrs = Attrs::new();
    project_attrs.push("Id", project.info.project_id.clone());
    open(&mut writer, "Project", &project_attrs)?;

    let xpath = "/KNX/Project/ProjectInformation";
    let mut info_attrs = Attrs::new();
    info_attrs.push("Name", project.info.name.clone());
    info_attrs.push(
        "GroupAddressStyle",
        group_address_style_str(project.info.group_address_style),
    );
    info_attrs.opt_display(
        "LastModified",
        project.info.last_modified.map(format_timestamp_v21),
    );
    info_attrs.opt_display(
        "ProjectStart",
        project.info.project_start.map(format_timestamp_v21),
    );
    // `Comment`/`LastUsedPuid`/`Guid`/`ProjectType`: known but not modeled
    // in `knx_core::ProjectInfo`, only ever one `ProjectInformation` per
    // project, so the flat retained-attribute bucket loses nothing here
    // (see the module doc's granularity note).
    info_attrs.fill_retained(&retained, xpath);

    let traces_xpath = format!("{xpath}/ProjectTraces");
    match elements.get(&traces_xpath) {
        Some(raw) => {
            open(&mut writer, "ProjectInformation", &info_attrs)?;
            write_raw_subtree(&mut writer, raw)?;
            close(&mut writer, "ProjectInformation")?;
        }
        None => empty(&mut writer, "ProjectInformation", &info_attrs)?,
    }

    close(&mut writer, "Project")?;
    close(&mut writer, "KNX")?;

    Ok(writer.into_inner())
}

/// Schema ≥21's own RFC 3339 instant form — see the module doc's timestamp
/// note.
fn format_timestamp_v21(dt: DateTime<Utc>) -> String {
    dt.to_rfc3339_opts(SecondsFormat::AutoSi, true)
}

/// `"true"`/`"false"` — see the module doc's boolean note.
fn push_bool_tf(attrs: &mut Attrs, name: &str, value: bool) {
    attrs.push(name, if value { "true" } else { "false" });
}

/// Re-emits a retained subtree's raw bytes event by event, through the same
/// writer producing everything else — unlike `schema11::write_raw_element`
/// (one event only, correct for a leaf like `BusAccess`), this walks every
/// event in `raw` up to its own `Eof`, since `raw` here is always a whole,
/// self-contained, well-formed fragment (captured by
/// `bytes[pos_before..end]` at the matching close tag) that can itself hold
/// arbitrarily deep children — `ModuleInstances/ModuleInstance/Arguments/
/// Argument`, `GroupObjectTree/Nodes/Node`, `ProjectTraces/ProjectTrace`.
/// Re-emitting only the first event, as `write_raw_element` does, would
/// write the opening tag alone and silently drop everything nested inside
/// it — not tolerable data loss, an ill-formed document.
fn write_raw_subtree(writer: &mut Writer<Vec<u8>>, raw: &[u8]) -> Result<(), ExportError> {
    let mut reader = quick_xml::Reader::from_reader(raw);
    loop {
        let event = reader
            .read_event()
            .map_err(|e| ExportError::Xml(e.to_string()))?;
        if matches!(event, Event::Eof) {
            return Ok(());
        }
        writer.write_event(event).map_err(xml_err)?;
    }
}

/// The literal xpath `lib.rs`'s import-side splice loop gives a device's
/// `ModuleInstances`/`GroupObjectTree`/`Security` subtree — must match that
/// loop's own string exactly, since this is the only place either side's
/// formula is allowed to drift from the other's.
fn device_raw_xpath(device_ets_id: &str) -> String {
    format!(
        "/KNX/Project/Installations/Installation/Topology/Area/Line/Segment/DeviceInstance[@Id='{device_ets_id}']"
    )
}

/// Same as [`device_raw_xpath`], for a device with no line (untested at
/// schema ≥21 — see this module's own doc comment and `lib.rs`'s mirrored
/// splice loop).
fn unassigned_device_raw_xpath(device_ets_id: &str) -> String {
    format!(
        "/KNX/Project/Installations/Installation/Topology/UnassignedDevices/DeviceInstance[@Id='{device_ets_id}']"
    )
}

/// The short form of every group address's ETS id (`"GA-3"` from
/// `"P-03DE-0_GA-3"`), the inverse of `map.rs`'s own
/// `short_group_address_ids` — `ComObjectInstanceRef/@Links` names a group
/// address this way at schema ≥21, not by its fully-qualified `@Id`.
fn short_group_address_ids(installation: &Installation) -> BTreeMap<GroupAddressId, String> {
    installation
        .group_addresses
        .iter()
        .filter_map(|g| {
            g.source
                .ets_id
                .rsplit_once('_')
                .filter(|(_, short)| short.starts_with("GA-"))
                .map(|(_, short)| (g.id, short.to_string()))
        })
        .collect()
}

fn write_installation_v21(
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
    // No `InstallationId` at schema ≥21 (not in `crate::known::SCHEMA_21`'s
    // own attribute list — measured, `KV v2.5 - demo.knxproj` genuinely has
    // none): `installation.name` defensively written only if non-empty
    // (never populated by today's schema-≥21 parser either, for the same
    // reason), so information entered post-import through the app is not
    // silently dropped even though a genuine schema-≥21 import never sets
    // it.
    if !installation.name.is_empty() {
        attrs.push("Name", installation.name.clone());
    }
    attrs.opt("DefaultLine", &default_line_ets_id);
    // `BCUKey`/`IPRoutingLatencyTolerance`: known but not modeled; safe via
    // the flat bucket since there is only ever one `Installation` in every
    // sample measured so far (see the module doc's granularity note).
    attrs.fill_retained(retained, xpath);
    open(writer, "Installation", &attrs)?;

    let short_ga = short_group_address_ids(installation);

    open(writer, "Topology", &Attrs::new())?;
    for area in &installation.topology.areas {
        write_area_v21(
            writer,
            project,
            &installation.topology,
            area,
            retained,
            elements,
            &short_ga,
        )?;
    }
    // Untested at schema ≥21 (see the module doc and `lib.rs`'s mirrored
    // splice loop) — `crate::known::SCHEMA_21` has no `UnassignedDevices`
    // entry yet, so `installation.topology.unassigned` is always empty for
    // a genuine schema-≥21 import today. Kept for symmetry with
    // `schema11.rs` and forward compatibility, not because it is exercised.
    if !installation.topology.unassigned.is_empty() {
        open(writer, "UnassignedDevices", &Attrs::new())?;
        for &device_id in &installation.topology.unassigned {
            if let Some(device) = project.devices.get(device_id) {
                let parameters = device_parameters(project, device.id);
                let device_xpath = format!(
                    "{xpath}/Topology/UnassignedDevices/DeviceInstance[@Id='{}']",
                    device.source.ets_id
                );
                let raw_xpath = unassigned_device_raw_xpath(&device.source.ets_id);
                write_device_v21(
                    writer,
                    project,
                    &parameters,
                    device,
                    &device_xpath,
                    &raw_xpath,
                    retained,
                    elements,
                    &short_ga,
                )?;
            }
        }
        close(writer, "UnassignedDevices")?;
    }
    close(writer, "Topology")?;

    if !installation.buildings.is_empty() {
        open(writer, "Locations", &Attrs::new())?;
        let by_id: BTreeMap<_, _> = installation.buildings.iter().map(|b| (b.id, b)).collect();
        for part in installation.buildings.iter().filter(|b| b.parent.is_none()) {
            write_space(writer, project, &by_id, part)?;
        }
        close(writer, "Locations")?;
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
        // Reused verbatim from `schema11.rs`: `GroupRanges`/`GroupRange`/
        // `GroupAddress` are the same element and attribute shapes at
        // schema ≥21 (measured — `crate::known::SCHEMA_21`'s own table).
        write_group_range(writer, &range_by_id, &installation.group_addresses, range)?;
    }
    close(writer, "GroupRanges")?;
    close(writer, "GroupAddresses")?;

    close(writer, "Installation")?;
    Ok(())
}

fn write_area_v21(
    writer: &mut Writer<Vec<u8>>,
    project: &Project,
    topology: &Topology,
    area: &Area,
    retained: &RetainedAttrs,
    elements: &BTreeMap<String, Vec<u8>>,
    short_ga: &BTreeMap<GroupAddressId, String>,
) -> Result<(), ExportError> {
    let mut attrs = Attrs::new();
    attrs.push("Id", area.source.ets_id.clone());
    attrs.push("Address", area.address.to_string());
    // No `Name` at schema ≥21 either (see `write_installation_v21`'s own
    // note) — written only if non-empty.
    if !area.name.is_empty() {
        attrs.push("Name", area.name.clone());
    }
    open(writer, "Area", &attrs)?;

    for &line_id in &area.lines {
        if let Some(line) = topology.line(line_id) {
            write_line_v21(
                writer,
                project,
                &area.source.ets_id,
                line,
                retained,
                elements,
                short_ga,
            )?;
        }
    }

    close(writer, "Area")?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn write_line_v21(
    writer: &mut Writer<Vec<u8>>,
    project: &Project,
    area_ets_id: &str,
    line: &Line,
    retained: &RetainedAttrs,
    elements: &BTreeMap<String, Vec<u8>>,
    short_ga: &BTreeMap<GroupAddressId, String>,
) -> Result<(), ExportError> {
    let mut attrs = Attrs::new();
    attrs.push("Id", line.source.ets_id.clone());
    attrs.push("Address", line.address.to_string());
    if !line.name.is_empty() {
        attrs.push("Name", line.name.clone());
    }
    open(writer, "Line", &attrs)?;

    // `Segment` re-synthesizes the wrapper the parser transparently merged
    // away on import (module doc), carrying the medium/domain-address
    // attributes schema 11 puts directly on `Line`. Its own `Id`/`Number`
    // are not reconstructed — see the module doc's granularity note; there
    // is one `Segment` per `Line` in every sample measured so far, so this
    // loses a synthetic bookkeeping id, nothing a device or group address
    // ever refers back to.
    let mut seg_attrs = Attrs::new();
    seg_attrs.push("MediumTypeRefId", line.medium_ref.clone());
    seg_attrs.opt("DomainAddress", &line.domain_address);
    if let Some(checked) = line.domain_address_is_checked {
        push_bool_tf(&mut seg_attrs, "DomainAddressIsChecked", checked);
    }
    seg_attrs.opt_display(
        "IPRoutingMulticastAddress",
        line.ip_routing_multicast_address,
    );
    seg_attrs.opt_display("MulticastTTL", line.multicast_ttl);

    let line_xpath = format!(
        "/KNX/Project/Installations/Installation/Topology/Area[@Id='{area_ets_id}']/Line[@Id='{}']",
        line.source.ets_id
    );

    if line.devices.is_empty() {
        empty(writer, "Segment", &seg_attrs)?;
    } else {
        open(writer, "Segment", &seg_attrs)?;
        for &device_id in &line.devices {
            if let Some(device) = project.devices.get(device_id) {
                let parameters = device_parameters(project, device.id);
                let device_xpath = format!(
                    "{line_xpath}/DeviceInstance[@Id='{}']",
                    device.source.ets_id
                );
                let raw_xpath = device_raw_xpath(&device.source.ets_id);
                write_device_v21(
                    writer,
                    project,
                    &parameters,
                    device,
                    &device_xpath,
                    &raw_xpath,
                    retained,
                    elements,
                    short_ga,
                )?;
            }
        }
        close(writer, "Segment")?;
    }

    close(writer, "Line")?;
    Ok(())
}

/// `ParameterInstance`s are owned by `Installation`, not `DeviceInstance`
/// (DATA_MODEL), so every caller resolves them by scanning every
/// installation's own list — the same approach `schema11.rs`'s
/// `write_device_with_params` takes, for the same reason.
fn device_parameters(project: &Project, device: knx_core::DeviceId) -> Vec<&ParameterInstance> {
    project
        .installations
        .iter()
        .flat_map(|i| &i.parameters)
        .filter(|p| p.device == device)
        .collect()
}

#[allow(clippy::too_many_arguments)]
fn write_device_v21(
    writer: &mut Writer<Vec<u8>>,
    project: &Project,
    parameters: &[&ParameterInstance],
    device: &DeviceInstance,
    device_xpath: &str,
    raw_xpath: &str,
    retained: &RetainedAttrs,
    elements: &BTreeMap<String, Vec<u8>>,
    short_ga: &BTreeMap<GroupAddressId, String>,
) -> Result<(), ExportError> {
    let mut attrs = Attrs::new();
    attrs.push("Id", device.source.ets_id.clone());
    attrs.push("Name", device.name.clone());
    if let Some(address) = device.address {
        attrs.push("Address", address.device().to_string());
    }
    attrs.push("ProductRefId", device.product_ref.clone());
    attrs.push("Hardware2ProgramRefId", device.program_ref.clone());
    attrs.opt("Description", &device.description);
    // `Comment`/`SerialNumber`/`IsActivityCalculated`/`LastUsedAPDULength`/
    // `ReadMaxAPDULength`/`Puid` are deliberately not written — see the
    // module doc's granularity note. No `CompletionStatus`/`Broken`/
    // `IsCommunicationObjectVisibilityCalculated` either: genuinely absent
    // from `crate::known::SCHEMA_21`'s own attribute list, not merely
    // unmapped.
    push_bool_tf(
        &mut attrs,
        "ApplicationProgramLoaded",
        device.commissioning.application_program_loaded,
    );
    push_bool_tf(
        &mut attrs,
        "CommunicationPartLoaded",
        device.commissioning.communication_part_loaded,
    );
    push_bool_tf(
        &mut attrs,
        "IndividualAddressLoaded",
        device.commissioning.individual_address_loaded,
    );
    push_bool_tf(
        &mut attrs,
        "MediumConfigLoaded",
        device.commissioning.medium_config_loaded,
    );
    push_bool_tf(
        &mut attrs,
        "ParametersLoaded",
        device.commissioning.parameters_loaded,
    );
    attrs.opt_display(
        "LastModified",
        device.commissioning.last_modified.map(format_timestamp_v21),
    );
    attrs.opt_display(
        "LastDownload",
        device.commissioning.last_download.map(format_timestamp_v21),
    );

    let module_instances = elements.get(&format!("{raw_xpath}/ModuleInstances"));
    let group_object_tree = elements.get(&format!("{raw_xpath}/GroupObjectTree"));
    let security = elements.get(&format!("{raw_xpath}/Security"));

    let has_children = !parameters.is_empty()
        || !device.com_objects.is_empty()
        || module_instances.is_some()
        || group_object_tree.is_some()
        || security.is_some();
    if !has_children {
        empty(writer, "DeviceInstance", &attrs)?;
        return Ok(());
    }
    open(writer, "DeviceInstance", &attrs)?;

    if !parameters.is_empty() {
        open(writer, "ParameterInstanceRefs", &Attrs::new())?;
        for param in parameters {
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
                write_com_object_v21(writer, project, com, device_xpath, retained, short_ga)?;
            }
        }
        close(writer, "ComObjectInstanceRefs")?;
    }

    // Order matches the measured sample exactly: ParameterInstanceRefs,
    // ComObjectInstanceRefs, ModuleInstances, GroupObjectTree, Security.
    if let Some(raw) = module_instances {
        write_raw_subtree(writer, raw)?;
    }
    if let Some(raw) = group_object_tree {
        write_raw_subtree(writer, raw)?;
    }
    if let Some(raw) = security {
        write_raw_subtree(writer, raw)?;
    }

    close(writer, "DeviceInstance")?;
    Ok(())
}

/// Flat `ComObjectInstanceRef` — no nested `Connectors` at schema ≥21.
/// `RefId` is `com.source.ets_id` verbatim (the original, unstripped
/// `GroupObjectTree` id — `map_com_object_v21`'s own doc comment). Flags
/// are never written: schema ≥21 instance overrides never carry them
/// (`map_com_object_v21`'s own doc comment; `com.flags` is always
/// `ResolvedFlags::none()` here, so there is nothing to write even if this
/// function tried).
fn write_com_object_v21(
    writer: &mut Writer<Vec<u8>>,
    project: &Project,
    com: &ComObjectInstance,
    device_xpath: &str,
    retained: &RetainedAttrs,
    short_ga: &BTreeMap<GroupAddressId, String>,
) -> Result<(), ExportError> {
    let ref_id = &com.source.ets_id;
    let xpath = format!("{device_xpath}/GroupObjectTree[@Id='{ref_id}']");

    let mut attrs = Attrs::new();
    attrs.push("RefId", ref_id.clone());
    let channel_id = retained.get(&(xpath, "ChannelId".to_string())).cloned();
    attrs.opt("ChannelId", &channel_id);

    if !com.links.is_empty() {
        let links = com
            .links
            .iter()
            .filter_map(|l| short_ga.get(&l.ga))
            .cloned()
            .collect::<Vec<_>>()
            .join(" ");
        if !links.is_empty() {
            attrs.push("Links", links);
        }
    }
    if !com.is_active {
        push_bool_tf(&mut attrs, "IsActive", false);
    }
    push_override_dpt(&mut attrs, "DatapointType", &com.dpt);
    push_override_text(&mut attrs, "Text", &com.text, &project.strings);
    push_override_text(
        &mut attrs,
        "Description",
        &com.description,
        &project.strings,
    );

    empty(writer, "ComObjectInstanceRef", &attrs)
}

fn write_space(
    writer: &mut Writer<Vec<u8>>,
    project: &Project,
    by_id: &BTreeMap<knx_core::BuildingPartId, &BuildingPart>,
    part: &BuildingPart,
) -> Result<(), ExportError> {
    let default_line_ets_id = part.default_line.and_then(|id| {
        project
            .installations
            .iter()
            .find_map(|i| i.topology.line(id))
            .map(|l| l.source.ets_id.clone())
    });

    let mut attrs = Attrs::new();
    attrs.push("Id", part.source.ets_id.clone());
    attrs.push("Name", part.name.clone());
    attrs.opt("Number", &part.number);
    attrs.push("Type", building_part_type_str(part.kind));
    attrs.opt("DefaultLine", &default_line_ets_id);
    // No `CompletionStatus`: genuinely absent from `crate::known::SCHEMA_21`'s
    // `Locations/Space` entry.

    let has_children = !part.children.is_empty() || !part.devices.is_empty();
    if !has_children {
        empty(writer, "Space", &attrs)?;
        return Ok(());
    }
    open(writer, "Space", &attrs)?;
    for &child_id in &part.children {
        if let Some(child) = by_id.get(&child_id) {
            write_space(writer, project, by_id, child)?;
        }
    }
    for &device_id in &part.devices {
        if let Some(device) = project.devices.get(device_id) {
            let mut a = Attrs::new();
            a.push("RefId", device.source.ets_id.clone());
            empty(writer, "DeviceInstanceRef", &a)?;
        }
    }
    close(writer, "Space")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::{all_entries, reference_kv_schema21_path};

    #[test]
    fn schema_21_refuses_to_silently_drop_an_unranged_group_address() {
        let project = super::super::schema11::project_with_unranged_group_address(21);

        assert_eq!(
            write_installation_xml_v21(&project, &[]).unwrap_err(),
            ExportError::UnrangedGroupAddress {
                installation_id: knx_core::InstallationId(3),
                group_address_id: knx_core::GroupAddressId(7),
                address: knx_core::GroupAddress::from_raw(2305),
            }
        );
    }

    /// The schema-21 counterpart of `tests/roundtrip.rs`'s
    /// `roundtrip_model_is_semantically_equal`: goes through the real public
    /// `import_knxproj`/`export_knxproj`/`import_knxproj_bytes` pipeline
    /// (wired for schema ≥21 since Task 8's `c3ba51d`, not the schema-11-only
    /// parser/mapper shortcut this test used before that), and checks
    /// genuine deep semantic equality via `compare::{semantic_view,
    /// describe_difference}` — including `ModuleInstance` data, now that
    /// `compare.rs` covers it.
    #[test]
    fn a_schema_21_export_reimports_to_an_equal_domain_model() {
        if !crate::testutil::corpus_available() {
            eprintln!("skip: OriginalData/ corpus not present (gitignored, local-only)");
            return;
        }
        let first = crate::import_knxproj(&reference_kv_schema21_path()).unwrap();
        assert_eq!(first.project.info.ets_schema_version, 21);
        assert!(first.project.devices.module_instances().count() > 0);

        let exported = crate::export::export_knxproj(
            &first.project,
            &all_entries(&first.opaque, &first.manufacturer),
        )
        .unwrap();
        let second = crate::import_knxproj_bytes(exported.bytes, "KV v2.5 - demo.knxproj").unwrap();

        let a = crate::compare::semantic_view(&first.project);
        let b = crate::compare::semantic_view(&second.project);
        if let Some(diff) = crate::compare::describe_difference(&a, &b) {
            panic!("roundtrip changed the model: {diff}");
        }
    }

    #[test]
    fn the_exported_xml_carries_the_schema_21_namespace_and_segment_wrapper() {
        if !crate::testutil::corpus_available() {
            eprintln!("skip: OriginalData/ corpus not present (gitignored, local-only)");
            return;
        }
        let document = crate::testutil::reference_kv_source_document();
        let mapped = crate::map::map(&document, "P-03DE/0.xml");
        let xml =
            String::from_utf8(write_installation_xml_v21(&mapped.project, &[]).unwrap()).unwrap();
        assert!(xml.contains(r#"xmlns="http://knx.org/xml/project/21""#));
        assert!(xml.contains("<Segment"));
        assert!(!xml.contains("<Connectors"));
    }

    #[test]
    fn the_exported_project_xml_carries_no_completion_status_or_project_id() {
        if !crate::testutil::corpus_available() {
            eprintln!("skip: OriginalData/ corpus not present (gitignored, local-only)");
            return;
        }
        let document = crate::testutil::reference_kv_source_document();
        let mapped = crate::map::map(&document, "P-03DE/0.xml");
        let xml = String::from_utf8(write_project_xml_v21(&mapped.project, &[]).unwrap()).unwrap();
        assert!(!xml.contains("CompletionStatus"));
        assert!(!xml.contains("ProjectId="));
    }
}
