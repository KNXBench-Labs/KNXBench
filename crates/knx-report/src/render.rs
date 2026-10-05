//! Walks a [`crate::model::ReportModel`] and assembles it into one
//! self-contained HTML string via [`crate::html`]'s escaping helpers and
//! document shell (Task 3).
//!
//! `render` (called by [`crate::render_html`]) is total and pure: given a
//! `&Project` and a [`crate::ReportOptions`], it never panics, never reads
//! the clock, and never lets a `HashMap`'s iteration order reach the output
//! — every lookup map built in this file is read with `.get()` inside a
//! loop driven by the model's own `Vec`/`BTreeMap` order, never iterated
//! itself. The device section delegates every string-table and
//! provenance-layer resolution to `knx_projection::build_device_detail`
//! rather than re-deriving it here.

use std::collections::{BTreeMap, HashMap};
use std::fmt::Write as _;

use knx_core::{
    BuildingPart, BuildingPartId, ComObjectInstanceId, DeviceInstance, Devices, GroupAddressEntry,
    GroupAddressId, GroupAddressStyle, GroupRange, GroupRangeId, Project,
};
use knx_projection::{build_device_detail, ComObjectNode};

use crate::html::{document_head, document_tail, escape_text};
use crate::model::{self, Counts, InstallationModel, MalformedField, ReportModel};
use crate::{
    HtmlReport, ReportComObjectText, ReportField, ReportLanguage, ReportOptions, ReportSection,
    ReportWarning,
};

/// Assembles the whole document: [`crate::model::build`] derives the
/// structural walk once, then every section is appended in the order
/// `docs/superpowers/specs/2026-09-10-project-documentation-export-design.md`
/// §3 lists them, and the honesty section closes it out. Warnings found by
/// the model, plus the one kind of warning this file can itself discover
/// (a device `knx_projection::build_device_detail` refuses to resolve), are
/// both returned to the caller and rendered inline — CLAUDE.md: never
/// silently discard a structural oddity.
pub(crate) fn render(project: &Project, options: &ReportOptions) -> HtmlReport {
    let ReportModel {
        installations,
        orphan_com_objects,
        malformed_com_object_fields,
        warnings,
        counts,
    } = model::build(project);
    let mut warnings = warnings;

    let mut out = String::new();
    out.push_str(&document_head(
        &document_title(project, options.language),
        options.language.code(),
    ));

    render_header(&mut out, project, options);
    render_contents(&mut out, project, options);
    if options.sections.contains(&ReportSection::Summary) {
        render_summary(&mut out, &counts, options.language);
    }
    if options.sections.contains(&ReportSection::Topology) {
        render_topology(&mut out, project, options.language);
    }
    if options.sections.contains(&ReportSection::Buildings) {
        render_buildings(&mut out, project, &installations, options.language);
    }
    if options.sections.contains(&ReportSection::GroupAddresses) {
        render_group_addresses(&mut out, project, &installations, options.language);
    }
    if options.sections.contains(&ReportSection::Devices) {
        render_devices(
            &mut out,
            project,
            &orphan_com_objects,
            &malformed_com_object_fields,
            options,
            &mut warnings,
        );
    }
    render_limits(&mut out, &warnings, options.language);

    out.push_str(&document_tail());

    HtmlReport {
        html: out,
        warnings,
    }
}

fn document_title(project: &Project, language: ReportLanguage) -> String {
    let title = language.text(
        "KNXBench Project Documentation",
        "KNXBench-Projektdokumentation",
    );
    if project.info.name.is_empty() {
        title.to_string()
    } else {
        format!("{} \u{2014} {title}", project.info.name)
    }
}

fn format_optional_dt(dt: Option<chrono::DateTime<chrono::Utc>>) -> String {
    dt.map(|d| d.to_rfc3339())
        .unwrap_or_else(|| "\u{2014}".to_string())
}

fn render_header(out: &mut String, project: &Project, options: &ReportOptions) {
    let info = &project.info;
    out.push_str("<header id=\"header\">");
    write!(
        out,
        "<h1>{}</h1>",
        escape_text(&document_title(project, options.language))
    )
    .unwrap();
    write!(
        out,
        "<h2>{}</h2><table>",
        options.language.text("Header", "Kopf")
    )
    .unwrap();
    write!(
        out,
        "<tr><th>Project number</th><td>{}</td></tr>",
        info.project_number
            .as_deref()
            .map(escape_text)
            .unwrap_or_else(|| "\u{2014}".to_string())
    )
    .unwrap();
    write!(
        out,
        "<tr><th>Group address style</th><td>{:?}</td></tr>",
        info.group_address_style
    )
    .unwrap();
    write!(
        out,
        "<tr><th>Completion</th><td>{:?}</td></tr>",
        info.completion
    )
    .unwrap();
    write!(
        out,
        "<tr><th>Last modified</th><td>{}</td></tr>",
        format_optional_dt(info.last_modified)
    )
    .unwrap();
    write!(
        out,
        "<tr><th>Project start</th><td>{}</td></tr>",
        format_optional_dt(info.project_start)
    )
    .unwrap();
    write!(
        out,
        "<tr><th>ETS schema version</th><td>{}</td></tr>",
        info.ets_schema_version
    )
    .unwrap();
    write!(
        out,
        "<tr><th>Domain schema version</th><td>{}</td></tr>",
        project.schema_version
    )
    .unwrap();
    write!(
        out,
        "<tr><th>Generated at</th><td>{}</td></tr>",
        options.generated_at.to_rfc3339()
    )
    .unwrap();
    out.push_str("</table></header>");
}

fn render_contents(out: &mut String, project: &Project, options: &ReportOptions) {
    let language = options.language;
    write!(
        out,
        "<nav id=\"contents\"><h2>{}</h2><ul>",
        language.text("Contents", "Inhalt")
    )
    .unwrap();
    write!(
        out,
        "<li><a href=\"#header\">{}</a></li>",
        language.text("Header", "Kopf")
    )
    .unwrap();
    write!(
        out,
        "<li><a href=\"#contents\">{}</a></li>",
        language.text("Contents", "Inhalt")
    )
    .unwrap();
    if options.sections.contains(&ReportSection::Summary) {
        write!(
            out,
            "<li><a href=\"#summary\">{}</a></li>",
            language.text("Summary", "Zusammenfassung")
        )
        .unwrap();
    }
    if options.sections.contains(&ReportSection::Topology) {
        write!(
            out,
            "<li><a href=\"#topology\">{}</a><ul>",
            language.text("Topology", "Topologie")
        )
        .unwrap();
        for installation in &project.installations {
            write!(
                out,
                "<li><a href=\"#installation-{}\">{} {}: {}</a></li>",
                installation.id.0,
                language.text("Installation", "Installation"),
                installation.id.0,
                escape_text(&installation.name)
            )
            .unwrap();
        }
        out.push_str("</ul></li>");
    }
    if options.sections.contains(&ReportSection::Buildings) {
        write!(
            out,
            "<li><a href=\"#buildings\">{}</a></li>",
            language.text("Buildings", "Gebäude")
        )
        .unwrap();
    }
    if options.sections.contains(&ReportSection::GroupAddresses) {
        write!(
            out,
            "<li><a href=\"#group-addresses\">{}</a></li>",
            language.text("Group addresses", "Gruppenadressen")
        )
        .unwrap();
    }
    if options.sections.contains(&ReportSection::Devices) {
        write!(
            out,
            "<li><a href=\"#devices\">{}</a></li>",
            language.text("Devices", "Geräte")
        )
        .unwrap();
    }
    write!(
        out,
        "<li><a href=\"#limits\">{}</a></li>",
        language.text("Limits and warnings", "Grenzen und Warnungen")
    )
    .unwrap();
    out.push_str("</ul></nav>");
}

fn render_summary(out: &mut String, counts: &Counts, language: ReportLanguage) {
    write!(
        out,
        "<section id=\"summary\"><h2>{}</h2><table>",
        language.text("Summary", "Zusammenfassung")
    )
    .unwrap();
    let rows: [(&str, usize); 9] = [
        ("Installations", counts.installations),
        ("Areas", counts.areas),
        ("Lines", counts.lines),
        ("Devices", counts.devices),
        ("Communication objects", counts.com_objects),
        ("Group ranges", counts.group_ranges),
        ("Group addresses", counts.group_addresses),
        ("Building parts", counts.building_parts),
        ("Parameter values", counts.parameter_values),
    ];
    for (label, value) in rows {
        write!(out, "<tr><th>{label}</th><td>{value}</td></tr>").unwrap();
    }
    out.push_str("</table></section>");
}

/// Topology comes straight off `Installation::topology` — the model has
/// nothing derived to offer here, since a device with no line is already
/// valid state the raw data expresses directly (`topology.rs:1-4`). A line
/// or device id that does not resolve is dropped defensively, the same
/// precedent `knx_projection::build_topology` sets, rather than invented as
/// a new warning category outside this task's scope.
fn render_topology(out: &mut String, project: &Project, language: ReportLanguage) {
    write!(
        out,
        "<section id=\"topology\"><h2>{}</h2>",
        language.text("Topology", "Topologie")
    )
    .unwrap();
    for installation in &project.installations {
        write!(
            out,
            "<h3 id=\"installation-{}\">Installation {}: {}</h3>",
            installation.id.0,
            installation.id.0,
            escape_text(&installation.name)
        )
        .unwrap();

        let topo = &installation.topology;
        if topo.areas.is_empty() {
            out.push_str("<p>No areas.</p>");
        } else {
            out.push_str("<ul>");
            for area in &topo.areas {
                write!(
                    out,
                    "<li>Area {}: {}<ul>",
                    area.address,
                    escape_text(&area.name)
                )
                .unwrap();
                for line_id in &area.lines {
                    if let Some(line) = topo.line(*line_id) {
                        write!(
                            out,
                            "<li>Line {}: {} (medium {}",
                            line.address,
                            escape_text(&line.name),
                            escape_text(&line.medium_ref)
                        )
                        .unwrap();
                        if let Some(domain) = &line.domain_address {
                            write!(out, ", domain address {}", escape_text(domain)).unwrap();
                        }
                        if let Some(mc) = line.ip_routing_multicast_address {
                            write!(out, ", multicast {mc}").unwrap();
                        }
                        if let Some(ttl) = line.multicast_ttl {
                            write!(out, ", TTL {ttl}").unwrap();
                        }
                        out.push_str(")<ul>");
                        for device_id in &line.devices {
                            if let Some(device) = project.devices.get(*device_id) {
                                render_device_summary_li(out, device);
                            }
                        }
                        out.push_str("</ul></li>");
                    }
                }
                out.push_str("</ul></li>");
            }
            out.push_str("</ul>");
        }

        out.push_str("<h4>Devices with no line</h4>");
        if topo.unassigned.is_empty() {
            out.push_str("<p>None.</p>");
        } else {
            out.push_str("<ul>");
            for device_id in &topo.unassigned {
                match project.devices.get(*device_id) {
                    Some(device) => render_device_summary_li(out, device),
                    None => write!(out, "<li>device {} (not found)</li>", device_id.0).unwrap(),
                }
            }
            out.push_str("</ul>");
        }
    }
    out.push_str("</section>");
}

fn render_device_summary_li(out: &mut String, device: &DeviceInstance) {
    write!(out, "<li>{}", escape_text(&device.name)).unwrap();
    if let Some(addr) = device.address {
        write!(out, " ({addr})").unwrap();
    }
    if let Some(desc) = &device.description {
        write!(out, " \u{2014} {}", escape_text(desc)).unwrap();
    }
    out.push_str("</li>");
}

/// Buildings are stored flat (`BuildingPart::parent`/`children`); the model
/// already split each installation's parts into `building_roots` and
/// `orphan_building_parts`, so this file only needs to walk `children` from
/// each root and separately render the orphans — otherwise they are
/// reachable from no root and would silently vanish from the document.
fn render_buildings(
    out: &mut String,
    project: &Project,
    installations: &[InstallationModel],
    language: ReportLanguage,
) {
    write!(
        out,
        "<section id=\"buildings\"><h2>{}</h2>",
        language.text("Buildings", "Gebäude")
    )
    .unwrap();
    for (i, installation) in project.installations.iter().enumerate() {
        let inst_model = &installations[i];
        write!(
            out,
            "<h3 id=\"buildings-{}\">Installation {}: {}</h3>",
            installation.id.0,
            installation.id.0,
            escape_text(&installation.name)
        )
        .unwrap();

        let by_id: HashMap<BuildingPartId, &BuildingPart> =
            installation.buildings.iter().map(|p| (p.id, p)).collect();

        if inst_model.building_roots.is_empty() {
            out.push_str("<p>No building parts.</p>");
        } else {
            out.push_str("<ul>");
            for root in &inst_model.building_roots {
                render_building_part(out, *root, &by_id, &project.devices);
            }
            out.push_str("</ul>");
        }

        if !inst_model.orphan_building_parts.is_empty() {
            out.push_str("<h4>Building parts with an unresolved parent</h4><ul>");
            for id in &inst_model.orphan_building_parts {
                render_building_part(out, *id, &by_id, &project.devices);
            }
            out.push_str("</ul>");
        }
    }
    out.push_str("</section>");
}

fn render_building_part(
    out: &mut String,
    part_id: BuildingPartId,
    by_id: &HashMap<BuildingPartId, &BuildingPart>,
    devices: &Devices,
) {
    let Some(part) = by_id.get(&part_id) else {
        return;
    };
    write!(
        out,
        "<li id=\"building-{}\"><strong>{}</strong> ({:?}",
        part_id.0,
        escape_text(&part.name),
        part.kind
    )
    .unwrap();
    if let Some(number) = &part.number {
        write!(out, ", #{}", escape_text(number)).unwrap();
    }
    out.push(')');

    if !part.devices.is_empty() {
        out.push_str("<ul>");
        for device_id in &part.devices {
            if let Some(device) = devices.get(*device_id) {
                render_device_summary_li(out, device);
            }
        }
        out.push_str("</ul>");
    }

    if !part.children.is_empty() {
        out.push_str("<ul>");
        for child_id in &part.children {
            render_building_part(out, *child_id, by_id, devices);
        }
        out.push_str("</ul>");
    }

    out.push_str("</li>");
}

/// Everything one [`render_range_subtree`]/[`render_group_address`] call
/// needs, bundled so neither function grows past a handful of parameters
/// (`clippy::too_many_arguments`) for what is otherwise a plain read-only
/// walk of the model's own maps.
struct GroupAddressCtx<'a> {
    ranges_by_id: &'a HashMap<GroupRangeId, &'a GroupRange>,
    range_children: &'a BTreeMap<GroupRangeId, Vec<GroupRangeId>>,
    addresses_by_range: &'a BTreeMap<GroupRangeId, Vec<GroupAddressId>>,
    addresses_by_id: &'a HashMap<GroupAddressId, &'a GroupAddressEntry>,
    links_by_address: &'a BTreeMap<GroupAddressId, Vec<ComObjectInstanceId>>,
    devices: &'a Devices,
    style: GroupAddressStyle,
}

/// Group ranges nest main → middle via the model's own `range_roots`/
/// `range_children` — already presence-checked in both directions, so every
/// id this walks is guaranteed to resolve; the `by_id` lookups stay
/// defensive purely as belt-and-braces, not because a dangling id can reach
/// here. Each address is shown under its innermost containing range, with
/// every communication object [`crate::model`]'s inverse index links to it,
/// each with its own DPT rather than a computed consensus value (§3: a
/// group address has no datapoint type of its own).
fn render_group_addresses(
    out: &mut String,
    project: &Project,
    installations: &[InstallationModel],
    language: ReportLanguage,
) {
    let style = project.info.group_address_style;
    write!(
        out,
        "<section id=\"group-addresses\"><h2>{}</h2>",
        language.text("Group addresses", "Gruppenadressen")
    )
    .unwrap();
    for (i, installation) in project.installations.iter().enumerate() {
        let inst_model = &installations[i];
        write!(
            out,
            "<h3 id=\"addresses-{}\">Installation {}: {}</h3>",
            installation.id.0,
            installation.id.0,
            escape_text(&installation.name)
        )
        .unwrap();

        let ranges_by_id: HashMap<GroupRangeId, &GroupRange> = installation
            .group_ranges
            .iter()
            .map(|r| (r.id, r))
            .collect();
        let addresses_by_id: HashMap<GroupAddressId, &GroupAddressEntry> = installation
            .group_addresses
            .iter()
            .map(|e| (e.id, e))
            .collect();

        let ctx = GroupAddressCtx {
            ranges_by_id: &ranges_by_id,
            range_children: &inst_model.range_children,
            addresses_by_range: &inst_model.addresses_by_range,
            addresses_by_id: &addresses_by_id,
            links_by_address: &inst_model.links_by_address,
            devices: &project.devices,
            style,
        };

        if inst_model.range_roots.is_empty() {
            out.push_str("<p>No group ranges.</p>");
        } else {
            out.push_str("<ul>");
            for root in &inst_model.range_roots {
                render_range_subtree(out, *root, &ctx);
            }
            out.push_str("</ul>");
        }

        if !inst_model.addresses_without_range.is_empty() {
            out.push_str("<h4>Group addresses inside no range</h4><ul>");
            for id in &inst_model.addresses_without_range {
                render_group_address(out, *id, &ctx);
            }
            out.push_str("</ul>");
        }
    }
    out.push_str("</section>");
}

fn render_range_subtree(out: &mut String, range_id: GroupRangeId, ctx: &GroupAddressCtx) {
    let Some(range) = ctx.ranges_by_id.get(&range_id) else {
        return;
    };
    write!(
        out,
        "<li id=\"range-{}\"><strong>{}</strong> [{} \u{2013} {}]",
        range_id.0,
        escape_text(&range.name),
        range.start.format(ctx.style),
        range.end.format(ctx.style)
    )
    .unwrap();

    if let Some(addr_ids) = ctx.addresses_by_range.get(&range_id) {
        out.push_str("<ul>");
        for ga_id in addr_ids {
            render_group_address(out, *ga_id, ctx);
        }
        out.push_str("</ul>");
    }

    if let Some(children) = ctx.range_children.get(&range_id) {
        out.push_str("<ul>");
        for child_id in children {
            render_range_subtree(out, *child_id, ctx);
        }
        out.push_str("</ul>");
    }

    out.push_str("</li>");
}

fn render_group_address(out: &mut String, ga_id: GroupAddressId, ctx: &GroupAddressCtx) {
    let Some(entry) = ctx.addresses_by_id.get(&ga_id) else {
        return;
    };
    write!(
        out,
        "<li id=\"ga-{}\"><strong>{}</strong> \u{2014} {}",
        ga_id.0,
        escape_text(&entry.address.format(ctx.style)),
        escape_text(&entry.name)
    )
    .unwrap();
    if entry.central {
        out.push_str(" \u{b7} Central");
    }
    if entry.unfiltered {
        out.push_str(" \u{b7} Unfiltered");
    }
    match ctx.links_by_address.get(&ga_id) {
        None => out.push_str(" \u{2014} no linked communication objects."),
        Some(ids) => {
            out.push_str("<ul>");
            for com_id in ids {
                render_ga_link(out, *com_id, ga_id, ctx.devices);
            }
            out.push_str("</ul>");
        }
    }
    out.push_str("</li>");
}

fn render_ga_link(
    out: &mut String,
    com_id: ComObjectInstanceId,
    ga_id: GroupAddressId,
    devices: &Devices,
) {
    let Some(com) = devices.com_object(com_id) else {
        write!(out, "<li>communication object {} (missing)</li>", com_id.0).unwrap();
        return;
    };
    let device_label = match devices.get(com.device) {
        Some(device) => escape_text(&device.name),
        None => format!("device {} (not found)", com.device),
    };
    let dpt = com
        .dpt
        .value()
        .map(|resolved| escape_text(&resolved.value.to_string()))
        .unwrap_or_else(|| "\u{2014}".to_string());
    let direction = com
        .links
        .iter()
        .find(|link| link.ga == ga_id)
        .map(|link| format!("{:?}", link.direction))
        .unwrap_or_else(|| "\u{2014}".to_string());
    write!(
        out,
        "<li>{} \u{2014} object {} \u{b7} DPT {} \u{b7} {}</li>",
        device_label, com.number, dpt, direction
    )
    .unwrap();
}

/// `Devices` is the crate-wide device list (not per installation), and its
/// own `iter()`/`com_objects()` are `BTreeMap`-backed, so walking them
/// directly is already deterministic without a separate sort pass. Every
/// resolved communication-object field (name, description, DPT, layer,
/// links) comes from [`build_device_detail`] — never re-derived here, per
/// this task's delegation rule.
fn render_devices(
    out: &mut String,
    project: &Project,
    orphan_com_objects: &[ComObjectInstanceId],
    malformed_com_object_fields: &BTreeMap<ComObjectInstanceId, Vec<MalformedField>>,
    options: &ReportOptions,
    warnings: &mut Vec<ReportWarning>,
) {
    write!(
        out,
        "<section id=\"devices\"><h2>{}</h2>",
        options.language.text("Devices", "Geräte")
    )
    .unwrap();
    for device in project.devices.iter() {
        let resolved = options.device_data.get(&device.id);
        write!(out, "<article id=\"device-{}\">", device.id.0).unwrap();
        write!(
            out,
            "<h3>{} ({})</h3>",
            escape_text(&device.name),
            device.id.0
        )
        .unwrap();
        out.push_str("<table>");
        write!(
            out,
            "<tr><th>Individual address</th><td>{}</td></tr>",
            device
                .address
                .map(|a| a.to_string())
                .unwrap_or_else(|| "\u{2014}".to_string())
        )
        .unwrap();
        write!(
            out,
            "<tr><th>Description</th><td>{}</td></tr>",
            device
                .description
                .as_deref()
                .map(escape_text)
                .unwrap_or_else(|| "\u{2014}".to_string())
        )
        .unwrap();
        write!(
            out,
            "<tr><th>Commissioning</th><td>{:?}{}</td></tr>",
            device.commissioning.completion,
            if device.commissioning.broken {
                " (broken)"
            } else {
                ""
            }
        )
        .unwrap();
        write!(
            out,
            "<tr><th>Product ref</th><td>{}</td></tr>",
            escape_text(&device.product_ref)
        )
        .unwrap();
        write!(
            out,
            "<tr><th>Program ref</th><td>{}</td></tr>",
            escape_text(&device.program_ref)
        )
        .unwrap();
        let identity_fields = [
            (
                "Manufacturer",
                "manufacturer name",
                resolved.and_then(|data| data.manufacturer.as_deref()),
                resolved.and_then(|data| data.manufacturer_reference.as_deref()),
            ),
            (
                "Product",
                "product name",
                resolved.and_then(|data| data.product.as_deref()),
                Some(device.product_ref.as_str()),
            ),
            (
                "Application program",
                "application-program name",
                resolved.and_then(|data| data.application_program.as_deref()),
                Some(device.program_ref.as_str()),
            ),
        ];
        let mut identity_problems = Vec::new();
        for (label, description, value, raw_reference) in identity_fields {
            let value = value.filter(|value| !value.trim().is_empty());
            let fallback = raw_reference.filter(|value| !value.trim().is_empty());
            write!(
                out,
                "<tr><th>{label}</th><td>{}</td></tr>",
                value
                    .map(escape_text)
                    .or_else(|| fallback.map(escape_text))
                    .unwrap_or_else(|| "unresolved".to_string())
            )
            .unwrap();
            if resolved.is_some() && value.is_none() {
                identity_problems.push(format!(
                    "product database did not resolve {description}; {}",
                    fallback
                        .map(|reference| format!("raw reference {reference} shown"))
                        .unwrap_or_else(|| "no raw reference is available".to_string())
                ));
            }
        }
        out.push_str("</table>");

        let identity_row_resolved = resolved
            .and_then(|data| data.manufacturer_reference.as_ref())
            .is_some();
        if !identity_row_resolved {
            let detail = "product database did not resolve all manufacturer, product, and application-program names; raw references are shown";
            write!(out, "<p class=\"warning\">{}</p>", escape_text(detail)).unwrap();
            warnings.push(ReportWarning {
                location: format!("device {}", device.id),
                detail: detail.to_string(),
            });
        } else {
            for detail in identity_problems.into_iter().chain(
                resolved
                    .into_iter()
                    .flat_map(|data| data.problems.iter().cloned()),
            ) {
                write!(out, "<p class=\"warning\">{}</p>", escape_text(&detail)).unwrap();
                warnings.push(ReportWarning {
                    location: format!("device {}", device.id),
                    detail,
                });
            }
        }

        if let Some(data) = resolved {
            render_report_fields(out, "Parameters", &data.parameters, device.id, warnings);
            render_report_fields(
                out,
                "Module arguments",
                &data.module_arguments,
                device.id,
                warnings,
            );
        } else {
            let parameters: Vec<ReportField> = project
                .installations
                .iter()
                .flat_map(|installation| installation.parameters.iter())
                .filter(|parameter| parameter.device == device.id)
                .map(|parameter| ReportField {
                    reference: parameter.source.ets_id.clone(),
                    name: None,
                    raw_value: parameter.raw.clone(),
                    display_value: None,
                    problem: Some(
                        "parameter declaration could not be resolved; raw value shown".into(),
                    ),
                })
                .collect();
            let arguments: Vec<ReportField> = project
                .devices
                .module_instances()
                .filter(|module| module.device == device.id)
                .flat_map(|module| module.arguments.iter())
                .map(|(source, raw)| ReportField {
                    reference: source.ets_id.clone(),
                    name: None,
                    raw_value: raw.clone(),
                    display_value: None,
                    problem: Some(
                        "module argument declaration could not be resolved; raw value shown".into(),
                    ),
                })
                .collect();
            render_report_fields(out, "Parameters", &parameters, device.id, warnings);
            render_report_fields(out, "Module arguments", &arguments, device.id, warnings);
        }

        if !device.binary_data.is_empty() {
            out.push_str("<p>Binary data (referenced, not embedded): ");
            let items: Vec<String> = device
                .binary_data
                .iter()
                .map(|b| format!("{} ({})", escape_text(&b.name), escape_text(&b.id)))
                .collect();
            out.push_str(&items.join(", "));
            out.push_str("</p>");
        }

        match build_device_detail(project, device.id) {
            Some(detail) => render_com_objects_table(
                out,
                &detail.com_objects,
                malformed_com_object_fields,
                resolved.map(|data| &data.com_object_texts),
            ),
            None => {
                out.push_str(
                    "<p class=\"warning\">Communication object detail unavailable for this device.</p>",
                );
                warnings.push(ReportWarning {
                    location: format!("device {}", device.id),
                    detail: "build_device_detail returned no detail for this device".to_string(),
                });
            }
        }
        out.push_str("</article>");
    }

    out.push_str("<h3>Orphaned communication objects</h3>");
    if orphan_com_objects.is_empty() {
        out.push_str("<p>None.</p>");
    } else {
        out.push_str("<table><tr><th>Id</th><th>Device</th><th>Number</th></tr>");
        for id in orphan_com_objects {
            match project.devices.com_object(*id) {
                Some(com) => write!(
                    out,
                    "<tr><td id=\"com-{}\">{}</td><td>{}</td><td>{}</td></tr>",
                    id.0, id.0, com.device, com.number
                )
                .unwrap(),
                None => write!(
                    out,
                    "<tr><td id=\"com-{}\">{}</td><td colspan=\"2\">not found</td></tr>",
                    id.0, id.0
                )
                .unwrap(),
            }
        }
        out.push_str("</table>");
    }
    out.push_str("</section>");
}

fn render_report_fields(
    out: &mut String,
    heading: &str,
    fields: &[ReportField],
    device_id: knx_core::DeviceId,
    warnings: &mut Vec<ReportWarning>,
) {
    write!(out, "<h4>{}</h4>", escape_text(heading)).unwrap();
    if fields.is_empty() {
        out.push_str("<p>None.</p>");
        return;
    }
    out.push_str(
        "<table><tr><th>Reference</th><th>Name</th><th>Raw value</th><th>Display value</th></tr>",
    );
    for field in fields {
        let name = field.name.as_deref().filter(|name| !name.trim().is_empty());
        let mut problems = Vec::new();
        if name.is_none() {
            problems.push("field name could not be resolved; raw identifier shown".to_string());
        }
        if let Some(problem) = &field.problem {
            problems.push(problem.clone());
        }
        write!(
            out,
            "<tr{}><td>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>",
            if problems.is_empty() {
                ""
            } else {
                " class=\"warning\""
            },
            escape_text(&field.reference),
            name.map(escape_text)
                .unwrap_or_else(|| escape_text(&field.reference)),
            escape_text(&field.raw_value),
            field
                .display_value
                .as_deref()
                .map(escape_text)
                .unwrap_or_else(|| "—".into()),
        )
        .unwrap();
        for problem in problems {
            write!(
                out,
                "<tr class=\"warning\"><td colspan=\"4\">{}</td></tr>",
                escape_text(&problem)
            )
            .unwrap();
            warnings.push(ReportWarning {
                location: format!("device {device_id}, {}", field.reference),
                detail: problem,
            });
        }
    }
    out.push_str("</table>");
}

fn render_com_objects_table(
    out: &mut String,
    coms: &[ComObjectNode],
    malformed_com_object_fields: &BTreeMap<ComObjectInstanceId, Vec<MalformedField>>,
    translated: Option<&BTreeMap<ComObjectInstanceId, ReportComObjectText>>,
) {
    if coms.is_empty() {
        out.push_str("<p>No communication objects.</p>");
        return;
    }
    out.push_str(
        "<table><tr><th>Number</th><th>Name</th><th>Description</th><th>DPT</th><th>Active</th><th>R</th><th>W</th><th>T</th><th>U</th><th>C</th><th>I</th><th>Links</th></tr>",
    );
    for com in coms {
        let text = translated.and_then(|texts| texts.get(&ComObjectInstanceId(com.id)));
        let name = text
            .and_then(|text| text.name.as_deref())
            .or(com.name.as_deref());
        let description = text
            .and_then(|text| text.description.as_deref())
            .or(com.description.as_deref());
        write!(out, "<tr><td>{}</td>", com.number).unwrap();
        write!(
            out,
            "<td>{}</td>",
            name.map(escape_text)
                .unwrap_or_else(|| "\u{2014}".to_string())
        )
        .unwrap();
        write!(
            out,
            "<td>{}</td>",
            description
                .map(escape_text)
                .unwrap_or_else(|| "\u{2014}".to_string())
        )
        .unwrap();
        let dpt_cell = match (&com.dpt, &com.dpt_layer) {
            (Some(dpt), Some(layer)) => format!("{} ({})", escape_text(dpt), escape_text(layer)),
            (Some(dpt), None) => escape_text(dpt),
            _ => "\u{2014}".to_string(),
        };
        write!(out, "<td>{dpt_cell}</td>").unwrap();
        write!(out, "<td>{}</td>", if com.is_active { "yes" } else { "no" }).unwrap();
        for flag in [
            com.read,
            com.write,
            com.transmit,
            com.update,
            com.communication,
            com.read_on_init,
        ] {
            write!(out, "<td>{}</td>", if flag { "\u{2713}" } else { "" }).unwrap();
        }
        out.push_str("<td>");
        if com.links.is_empty() {
            out.push('\u{2014}');
        } else {
            let parts: Vec<String> =
                com.links
                    .iter()
                    .map(|link| {
                        let address =
                            link.address.as_deref().map(escape_text).unwrap_or_else(|| {
                                format!("group address {} (not found)", link.ga_id)
                            });
                        format!("{} {}", link.direction, address)
                    })
                    .collect();
            out.push_str(&parts.join(", "));
        }
        out.push_str("</td></tr>");

        if let Some(fields) = malformed_com_object_fields.get(&ComObjectInstanceId(com.id)) {
            for field in fields {
                write!(
                    out,
                    "<tr class=\"warning\"><td colspan=\"12\">Unparseable source value kept \
                     verbatim in field <strong>{}</strong>: {}</td></tr>",
                    escape_text(field.field),
                    escape_text(&field.raw)
                )
                .unwrap();
            }
        }
    }
    out.push_str("</table>");
}

/// The fixed closing section §5 requires every generated document to carry:
/// six honesty statements about what this generator deliberately does not
/// resolve or render, plus every anomaly the model walk found — rendered
/// here regardless of whether it was also surfaced naturally by an earlier
/// section, so this list alone is a complete account of every warning
/// `HtmlReport::warnings` carries (CLAUDE.md: never silently discard a
/// structural oddity).
fn render_limits(out: &mut String, warnings: &[ReportWarning], language: ReportLanguage) {
    write!(
        out,
        "<section id=\"limits\"><h2>{}</h2><ul>",
        language.text("Limits and warnings", "Grenzen und Warnungen")
    )
    .unwrap();
    write!(out, "<li>{}</li>", language.text(
        "Names and display values come from caller-supplied product data; unresolved values keep their raw identifiers and appear as warnings.",
        "Namen und Anzeigewerte stammen aus den vom Aufrufer gelieferten Produktdaten; nicht aufgelöste Werte behalten ihre Rohkennungen und erscheinen als Warnungen.",
    )).unwrap();
    write!(out, "<li>{}</li>", language.text(
        "Module arguments are shown as raw bindings; AllocatorRef and unknown module argument kinds, allocation metadata, and repeat semantics are not interpreted.",
        "Modulargumente werden als Rohbindungen gezeigt; AllocatorRef und unbekannte Modulargumenttypen, Allokationsmetadaten und Wiederholungssemantik werden nicht interpretiert.",
    )).unwrap();
    write!(
        out,
        "<li>{}</li>",
        language.text(
            "Binary data attached to devices is referenced by name and id only.",
            "Binärdaten an Geräten werden nur mit Name und Kennung referenziert.",
        )
    )
    .unwrap();
    write!(
        out,
        "<li>{}</li>",
        language.text(
            "This report is not an ETS report and has not been compared to one.",
            "Dieser Bericht ist kein ETS-Bericht und wurde mit keinem solchen verglichen.",
        )
    )
    .unwrap();
    out.push_str("</ul>");

    write!(
        out,
        "<h3>{}</h3>",
        language.text(
            "Anomalies found while generating this document",
            "Beim Erzeugen dieses Dokuments gefundene Auffälligkeiten",
        )
    )
    .unwrap();
    if warnings.is_empty() {
        out.push_str("<p>None.</p>");
    } else {
        out.push_str("<ul class=\"warning\">");
        for w in warnings {
            write!(
                out,
                "<li><strong>{}:</strong> {}</li>",
                escape_text(&w.location),
                escape_text(&w.detail)
            )
            .unwrap();
        }
        out.push_str("</ul>");
    }
    out.push_str("</section>");
}

#[cfg(test)]
mod tests {
    use chrono::{TimeZone, Utc};
    use knx_core::{
        Area, AreaId, BuildingPartType, CompletionStatus, DeviceId, GroupAddressStyle,
        IndividualAddress, Line, LineId, Project,
    };

    use crate::html::escape_text;
    use crate::testutil::{
        building_part, device, entry, linked_com_object, range, unlinked_com_object,
    };
    use crate::{
        render_html, ReportComObjectText, ReportDeviceData, ReportField, ReportLanguage,
        ReportOptions, ReportSection,
    };

    fn fixed_time() -> chrono::DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 9, 10, 12, 0, 0).unwrap()
    }

    fn options() -> ReportOptions {
        ReportOptions::new(fixed_time())
    }

    /// A hand-built project big enough to exercise every section at once:
    /// two devices (one on a line, one unassigned), a two-level building
    /// hierarchy, a two-level group-range hierarchy with one address inside
    /// it and one address inside no range, a linked communication object,
    /// and an orphaned one whose owning device does not exist.
    fn sample_project() -> Project {
        let mut project = crate::testutil::empty_project(GroupAddressStyle::Free);

        let area = Area {
            id: AreaId(1),
            source: crate::testutil::source("Area-1"),
            name: "Area 1".into(),
            address: 1,
            completion: CompletionStatus::FinishedDesign,
            lines: vec![LineId(1)],
        };
        let line = Line {
            id: LineId(1),
            source: crate::testutil::source("Line-1"),
            name: "Line 1".into(),
            address: 1,
            medium_ref: "TP".into(),
            domain_address: None,
            domain_address_is_checked: None,
            ip_routing_multicast_address: None,
            multicast_ttl: None,
            completion: CompletionStatus::FinishedDesign,
            devices: vec![DeviceId(1)],
        };
        project.installations[0].topology.areas = vec![area];
        project.installations[0].topology.lines = vec![line];
        project.installations[0].topology.unassigned = vec![DeviceId(2)];

        let building = building_part(1, "Main Building", BuildingPartType::Building, None, &[2]);
        let mut room = building_part(2, "Living Room", BuildingPartType::Room, Some(1), &[]);
        room.devices = vec![DeviceId(1)];
        project.installations[0].buildings = vec![building, room];

        let main = range(1, "Main Range", 0, 4095, None, &[2]);
        let middle = range(2, "Middle Range", 0, 2047, Some(1), &[]);
        project.installations[0].group_ranges = vec![main, middle];
        project.installations[0].group_addresses = vec![
            entry(1, 100, "Living Room Light"),
            entry(2, 5000, "Orphaned Address"),
        ];

        let mut device1 = device(1, &[1]);
        device1.name = "Living Room Switch".into();
        device1.address = Some(IndividualAddress::new(1, 1, 1).unwrap());
        device1.description = Some("Hallway switch".into());
        project.devices.insert(device1);
        project.devices.insert(device(2, &[]));
        project
            .devices
            .insert_com_object(linked_com_object(1, 1, 1));

        // Orphan: its owning device (99) does not exist.
        project
            .devices
            .insert_com_object(unlinked_com_object(3, 99));

        project
    }

    #[test]
    fn building_report_distinguishes_documented_space_kinds_from_building_part() {
        let mut project = sample_project();
        for (kind, token) in [
            (BuildingPartType::Stairway, "Stairway"),
            (BuildingPartType::RoomPart, "RoomPart"),
            (BuildingPartType::Area, "Area"),
            (BuildingPartType::Ground, "Ground"),
            (BuildingPartType::Segment, "Segment"),
        ] {
            project.installations[0].buildings = vec![building_part(1, "Test", kind, None, &[])];
            let report = render_html(&project, &ReportOptions::new(fixed_time()));
            assert!(report
                .html
                .contains(&format!("<strong>Test</strong> ({token})")));
            assert!(!report.html.contains("<strong>Test</strong> (BuildingPart)"));
        }
    }

    #[test]
    fn rendering_the_same_project_twice_with_the_same_timestamp_is_byte_identical() {
        let project = sample_project();
        let options = ReportOptions::new(fixed_time());
        let a = render_html(&project, &options);
        let b = render_html(&project, &options);
        assert_eq!(a.html, b.html);
    }

    #[test]
    fn device_group_address_and_building_part_names_are_escaped_everywhere() {
        let mut project = sample_project();
        project.devices.get_mut(DeviceId(1)).unwrap().name = "A & B <x> \"q\" 'r'".into();
        project.installations[0].group_addresses[0].name = "Licht & Steckdose".into();
        project.installations[0].buildings[1].name = "<Keller>".into();
        let report = render_html(&project, &ReportOptions::new(fixed_time()));

        assert!(!report.html.contains("A & B <x>"));
        assert!(report.html.contains("A &amp; B &lt;x&gt;"));
        assert!(report.html.contains("Licht &amp; Steckdose"));
        assert!(!report.html.contains("<Keller>"));
        assert!(report.html.contains("&lt;Keller&gt;"));
    }

    #[test]
    fn the_document_is_self_contained() {
        let project = sample_project();
        let report = render_html(&project, &ReportOptions::new(fixed_time()));
        assert!(!report.html.contains("<script"));
        assert!(!report.html.contains("http://"));
        assert!(!report.html.contains("https://"));
        assert!(!report.html.contains("transition:"));
        assert!(!report.html.contains("animation:"));
    }

    #[test]
    fn every_device_group_address_and_building_part_name_appears_and_orphans_are_listed() {
        let project = sample_project();
        let report = render_html(&project, &ReportOptions::new(fixed_time()));

        for device in project.devices.iter() {
            assert!(
                report.html.contains(&escape_text(&device.name)),
                "missing device name {:?}",
                device.name
            );
        }
        for entry in &project.installations[0].group_addresses {
            let formatted = entry.address.format(GroupAddressStyle::Free);
            assert!(
                report.html.contains(&formatted),
                "missing formatted group address {formatted}"
            );
        }
        for part in &project.installations[0].buildings {
            assert!(
                report.html.contains(&escape_text(&part.name)),
                "missing building part name {:?}",
                part.name
            );
        }

        // Device 2 has no line.
        assert!(report.html.contains("Devices with no line"));
        // Communication object 3 belongs to no existing device.
        assert!(report.html.contains("id=\"com-3\""));
    }

    #[test]
    fn all_required_section_headings_and_contents_anchors_are_present() {
        let project = sample_project();
        let report = render_html(&project, &ReportOptions::new(fixed_time()));

        for heading in [
            "Header",
            "Contents",
            "Summary",
            "Topology",
            "Buildings",
            "Group addresses",
            "Devices",
            "Limits and warnings",
        ] {
            assert!(report.html.contains(heading), "missing heading {heading}");
        }
        for anchor in [
            "href=\"#header\"",
            "href=\"#contents\"",
            "href=\"#summary\"",
            "href=\"#topology\"",
            "href=\"#buildings\"",
            "href=\"#group-addresses\"",
            "href=\"#devices\"",
            "href=\"#limits\"",
        ] {
            assert!(
                report.html.contains(anchor),
                "missing contents anchor {anchor}"
            );
        }
    }

    #[test]
    fn the_honesty_section_names_every_documented_limitation() {
        let project = sample_project();
        let report = render_html(&project, &ReportOptions::new(fixed_time()));

        assert!(report.html.contains("caller-supplied product data"));
        assert!(report.html.contains("raw identifiers"));
        assert!(report.html.contains("warnings"));
        assert!(report.html.contains("Binary data"));
        assert!(report.html.contains("not an ETS report"));
    }

    #[test]
    fn html_report_warnings_are_carried_and_each_is_visible_in_the_document_body() {
        let project = sample_project();
        let report = render_html(&project, &ReportOptions::new(fixed_time()));

        assert!(!report.warnings.is_empty());
        for warning in &report.warnings {
            assert!(
                report.html.contains(&warning.location),
                "warning location {:?} not visible in the document",
                warning.location
            );
        }
    }

    #[test]
    fn a_malformed_communication_object_field_and_a_dangling_building_child_both_reach_the_document(
    ) {
        use knx_core::Override;

        let mut project = sample_project();

        // Fix round 1, concern 2: an unparseable DPT is kept verbatim
        // (`Override::Malformed`) and must show up next to the
        // communication object it belongs to, not just in the warning
        // list.
        let mut malformed_com = linked_com_object(4, 1, 1);
        malformed_com.dpt = Override::Malformed("DPST-garbage".to_string());
        project.devices.insert_com_object(malformed_com);
        project
            .devices
            .get_mut(DeviceId(1))
            .unwrap()
            .com_objects
            .push(knx_core::ComObjectInstanceId(4));

        // Fix round 1, concern 1: a building part naming a child id that
        // does not exist must warn, not just be silently skipped.
        project.installations[0].buildings[0]
            .children
            .push(knx_core::BuildingPartId(404));

        let report = render_html(&project, &ReportOptions::new(fixed_time()));

        assert!(
            report.html.contains("DPST-garbage"),
            "malformed dpt raw text not visible in the document body"
        );
        assert!(report
            .warnings
            .iter()
            .any(|w| w.location.contains("communication object 4") && w.detail.contains("dpt")));

        assert!(report
            .warnings
            .iter()
            .any(|w| w.location.contains("building part 1") && w.detail.contains("404")));
        for warning in &report.warnings {
            assert!(
                report.html.contains(&warning.location),
                "warning location {:?} not visible in the document",
                warning.location
            );
        }
    }

    #[test]
    fn the_generated_at_timestamp_is_printed_and_changes_the_output() {
        let project = sample_project();
        let t1 = fixed_time();
        let t2 = Utc.with_ymd_and_hms(2030, 1, 1, 0, 0, 0).unwrap();
        let report1 = render_html(&project, &ReportOptions::new(t1));
        let report2 = render_html(&project, &ReportOptions::new(t2));

        assert!(report1.html.contains(&t1.to_rfc3339()));
        assert_ne!(report1.html, report2.html);
    }

    #[test]
    fn section_selection_changes_both_contents_and_rendered_body() {
        let project = sample_project();
        let mut options = options();
        options.sections = [ReportSection::Devices].into_iter().collect();

        let report = render_html(&project, &options);

        assert!(report.html.contains("href=\"#devices\""));
        assert!(report.html.contains("<section id=\"devices\""));
        assert!(!report.html.contains("href=\"#topology\""));
        assert!(!report.html.contains("<section id=\"topology\""));
        assert!(!report.html.contains("<section id=\"summary\""));
    }

    #[test]
    fn resolved_identity_values_and_module_arguments_are_rendered() {
        let project = sample_project();
        let mut options = options();
        options.device_data.insert(
            DeviceId(1),
            ReportDeviceData {
                manufacturer_reference: Some("M-acme".into()),
                manufacturer: Some("Acme Controls".into()),
                product: Some("Room Controller 8".into()),
                application_program: Some("Lighting 2.1".into()),
                problems: vec![],
                parameters: vec![ReportField {
                    reference: "P-1_R-1".into(),
                    name: Some("Operating mode".into()),
                    raw_value: "1".into(),
                    display_value: Some("Automatic".into()),
                    problem: None,
                }],
                module_arguments: vec![ReportField {
                    reference: "MD-1_A-1".into(),
                    name: Some("Channel".into()),
                    raw_value: "4".into(),
                    display_value: None,
                    problem: None,
                }],
                com_object_texts: std::collections::BTreeMap::new(),
            },
        );

        let report = render_html(&project, &options);

        for expected in [
            "Acme Controls",
            "Room Controller 8",
            "Lighting 2.1",
            "Operating mode",
            "Automatic",
            "Channel",
        ] {
            assert!(report.html.contains(expected), "missing {expected}");
        }
    }

    #[test]
    fn unresolved_identity_and_unrenderable_value_warn_and_remain_inline() {
        let mut project = sample_project();
        project.devices.remove(DeviceId(2));
        let mut options = options();
        options.device_data.insert(
            DeviceId(1),
            ReportDeviceData {
                parameters: vec![ReportField {
                    reference: "P-broken".into(),
                    name: None,
                    raw_value: "raw-17".into(),
                    display_value: None,
                    problem: None,
                }],
                ..ReportDeviceData::default()
            },
        );

        let report = render_html(&project, &options);

        assert!(report.html.contains("P-broken"));
        assert!(report.html.contains("raw-17"));
        assert!(report
            .warnings
            .iter()
            .any(|warning| warning.detail.contains("field name could not be resolved")));
        assert!(report
            .warnings
            .iter()
            .any(|warning| warning.detail.contains("product database")));
        for warning in &report.warnings {
            assert!(report.html.contains(&warning.detail));
        }
    }

    // AR10 (§37): the caller's translated communication-object text is what
    // the table shows; an object without an entry keeps the project's text.
    #[test]
    fn caller_translated_com_object_text_replaces_only_its_own_object() {
        let project = sample_project();
        let mut options = options();
        let mut com_object_texts = std::collections::BTreeMap::new();
        com_object_texts.insert(
            knx_core::ComObjectInstanceId(1),
            ReportComObjectText {
                name: Some("Schalten".into()),
                description: None,
            },
        );
        options.device_data.insert(
            DeviceId(1),
            ReportDeviceData {
                com_object_texts,
                ..ReportDeviceData::default()
            },
        );

        let report = render_html(&project, &options);

        assert!(report.html.contains("<td>Schalten</td>"));
        assert_eq!(report.html.matches("<td>Schalten</td>").count(), 1);
        let untranslated = render_html(&project, &ReportOptions::new(fixed_time()));
        assert!(!untranslated.html.contains("Schalten"));
    }

    #[test]
    fn german_report_localizes_chrome_and_sets_the_document_language() {
        let project = sample_project();
        let mut options = options();
        options.language = ReportLanguage::German;

        let report = render_html(&project, &options);

        assert!(report.html.contains("<html lang=\"de\">"));
        assert!(report.html.contains("Inhalt"));
        assert!(report.html.contains("Zusammenfassung"));
        assert!(report.html.contains("Geräte"));
        assert!(!report.html.contains(">Contents<"));
    }
}
