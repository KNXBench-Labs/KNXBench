//! Stage 5: maps a [`SourceDocument`] into a `knx_core::Project`.
//!
//! Two passes over the document. The first allocates a stable internal id
//! for every `Area`, `Line`, `DeviceInstance`, `GroupRange`, `GroupAddress`
//! and `BuildingPart`, and records each one in a `BTreeMap<String, XxxId>`
//! keyed by its ETS id string. The second builds the actual entities,
//! looking each cross-reference (`Connectors/Send/@GroupAddressRefId`,
//! `BuildingPart`'s `DeviceInstanceRef`/`@DefaultLine`,
//! `Installation/@DefaultLine`) up in those tables rather than allocating on
//! the fly — a `Send` can name a group address the document has not reached
//! yet, so the id has to already exist by the time the link is resolved.
//! `ComObjectInstance` and `ParameterInstance` ids are allocated directly in
//! the second pass: nothing in the source data ever refers to one by its
//! ETS id, so no table is needed for them.
//!
//! A [`ValueError`] never aborts an entity's mapping: the field is left at
//! its type's natural default, a [`MapProblem`] is recorded, and every
//! other field on that entity — and every other entity — still maps. The
//! one exception is a dangling cross-reference, which drops just that link
//! (`MapProblemDetail::DroppedLink`) or that single optional field
//! (`MapProblemDetail::UnresolvedReference`), never the entity that held it.
//!
//! A field modelled as [`Override`] does better than "natural default": an
//! unparsable present value becomes `Override::Malformed`, keeping the raw
//! text so export writes back exactly what the source said instead of
//! dropping the attribute. Fields modelled as a bare value or an `Option`
//! have nowhere to keep the raw text, so those still fall back to the
//! default with the problem recorded — the loss is reported, not silent.
//!
//! `retained` collects every [`RetainedAttribute`] this walk passes —
//! known-but-not-modelled attributes such as `Installation/@BCUKey` and
//! `@SplitType`, plus anything Stage 3 could not classify at all — for
//! export to write back. `SourceLine/@BusAccess` is deliberately *not*
//! among them: it is a [`crate::source::RetainedElement`] (raw XML bytes),
//! not a [`RetainedAttribute`] (a name/value pair), and the two cannot be
//! merged into one `Vec` without inventing a fake text value for binary
//! content. `RetainedElement`s stay in `ParseOutput` and reach export from
//! there directly, bypassing `MapOutput` — a correction to this task's
//! original interface sketch, which named `BusAccess` as flowing through
//! `retained` without accounting for the type mismatch.

use std::collections::BTreeMap;
use std::net::Ipv4Addr;

use chrono::{DateTime, Utc};

use knx_core::{
    Area, AreaId, BinaryDataRef, BuildingPart, BuildingPartId, BuildingPartType, ComObjectInstance,
    ComObjectInstanceId, CommissioningState, CompletionStatus, DeviceId, DeviceInstance, Devices,
    Direction, DptRef, GroupAddress, GroupAddressEntry, GroupAddressId, GroupAddressStyle,
    GroupLink, GroupRange, GroupRangeId, IdAllocators, IndividualAddress, Installation,
    InstallationId, Language, Layer, Line, LineId, Override, ParameterInstance, Project,
    ProjectInfo, Resolved, ResolvedFlags, SourceRef, Text, Topology,
};

use crate::source::{
    RetainedAttribute, SourceBuildingPart, SourceComObjectInstance, SourceDevice, SourceDocument,
    SourceGroupRange, SourceInstallation,
};
use crate::values::{
    com_object_number, parse_bool, parse_building_part_type, parse_completion_status,
    parse_timestamp, parse_u16, parse_u8, ValueError,
};

pub struct MapOutput {
    pub project: Project,
    /// Known-but-not-modelled attributes, for export. See the module doc
    /// comment for why `BusAccess` is not among them.
    pub retained: Vec<RetainedAttribute>,
    pub problems: Vec<MapProblem>,
    pub counts: EntityCounts,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MapProblem {
    pub xpath: String,
    pub detail: MapProblemDetail,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MapProblemDetail {
    Value(ValueError),
    UnresolvedReference {
        kind: &'static str,
        target: String,
    },
    DroppedLink {
        com_object: String,
        group_address: String,
    },
}

/// How many source elements of one kind were read, and how many were
/// mapped into a core entity. In this implementation the two always match —
/// a bad field defaults and is reported, but never drops the entity that
/// held it — so the distinction is currently only informational, kept for
/// a future case where an entity genuinely cannot be mapped at all.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Count {
    pub read: usize,
    pub mapped: usize,
}

impl Count {
    fn bump(&mut self) {
        self.read += 1;
        self.mapped += 1;
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EntityCounts {
    pub areas: Count,
    pub lines: Count,
    pub devices: Count,
    pub com_objects: Count,
    pub group_ranges: Count,
    pub group_addresses: Count,
    pub building_parts: Count,
    pub parameters: Count,
}

/// Pass 1's result: a stable internal id for every cross-referenceable
/// entity, keyed by its ETS id string.
#[derive(Default)]
struct IdTables {
    areas: BTreeMap<String, AreaId>,
    lines: BTreeMap<String, LineId>,
    devices: BTreeMap<String, DeviceId>,
    group_ranges: BTreeMap<String, GroupRangeId>,
    group_addresses: BTreeMap<String, GroupAddressId>,
    building_parts: BTreeMap<String, BuildingPartId>,
}

pub fn map(document: &SourceDocument, source_path: &str) -> MapOutput {
    let mut ids = IdAllocators::default();
    let tables = allocate_ids(document, &mut ids);

    // Session 3 imports no application program, so no `TranslationUnit`
    // ever populates the string table and every `Text` produced here is
    // `Text::Literal`. The project file itself carries no project-wide
    // language tag (RESEARCH §4.1 — `DefaultLanguage` belongs to an
    // application program's `RegistrationInfo`, not to `ProjectInformation`),
    // so this is a placeholder pending Session 4, not a measured fact.
    let mut project = Project::new(Language("en".into()));

    let mut retained = Vec::new();
    let mut problems = Vec::new();
    let mut counts = EntityCounts::default();

    project.info = map_project_info(document, &mut retained, &mut problems);

    for installation in &document.installations {
        let (mapped, installation_retained) = map_installation(
            installation,
            source_path,
            &tables,
            &mut ids,
            &mut project.devices,
            &mut problems,
            &mut counts,
        );
        retained.extend(installation_retained);
        project.installations.push(mapped);
    }

    project.ids = ids;

    MapOutput {
        project,
        retained,
        problems,
        counts,
    }
}

fn allocate_ids(document: &SourceDocument, ids: &mut IdAllocators) -> IdTables {
    let mut tables = IdTables::default();
    for installation in &document.installations {
        for area in &installation.areas {
            tables.areas.insert(area.id.clone(), ids.next_area_id());
            for line in &area.lines {
                tables.lines.insert(line.id.clone(), ids.next_line_id());
                for device in &line.devices {
                    tables
                        .devices
                        .insert(device.id.clone(), ids.next_device_id());
                }
            }
        }
        for device in &installation.unassigned_devices {
            tables
                .devices
                .insert(device.id.clone(), ids.next_device_id());
        }
        for range in &installation.group_ranges {
            allocate_group_range_ids(range, ids, &mut tables);
        }
        for part in &installation.buildings {
            allocate_building_part_ids(part, ids, &mut tables);
        }
    }
    tables
}

fn allocate_group_range_ids(
    range: &SourceGroupRange,
    ids: &mut IdAllocators,
    tables: &mut IdTables,
) {
    tables
        .group_ranges
        .insert(range.id.clone(), ids.next_group_range_id());
    for address in &range.addresses {
        tables
            .group_addresses
            .insert(address.id.clone(), ids.next_group_address_id());
    }
    for child in &range.children {
        allocate_group_range_ids(child, ids, tables);
    }
}

fn allocate_building_part_ids(
    part: &SourceBuildingPart,
    ids: &mut IdAllocators,
    tables: &mut IdTables,
) {
    tables
        .building_parts
        .insert(part.id.clone(), ids.next_building_part_id());
    for child in &part.children {
        allocate_building_part_ids(child, ids, tables);
    }
}

fn map_project_info(
    document: &SourceDocument,
    retained: &mut Vec<RetainedAttribute>,
    problems: &mut Vec<MapProblem>,
) -> ProjectInfo {
    let info = &document.info;
    retained.extend(info.other.iter().cloned());
    let xpath = "/KNX/Project/ProjectInformation";

    let group_address_style = match info.group_address_style.as_deref() {
        None => GroupAddressStyle::ThreeLevel,
        Some("Free") => GroupAddressStyle::Free,
        Some("TwoLevel") => GroupAddressStyle::TwoLevel,
        Some("ThreeLevel") => GroupAddressStyle::ThreeLevel,
        Some(other) => {
            problems.push(MapProblem {
                xpath: xpath.to_string(),
                detail: MapProblemDetail::Value(ValueError::UnknownEnumValue {
                    kind: "ProjectInformation/@GroupAddressStyle",
                    value: other.to_string(),
                }),
            });
            GroupAddressStyle::ThreeLevel
        }
    };

    ProjectInfo {
        project_id: document.project_id.clone(),
        name: info.name.clone().unwrap_or_default(),
        project_number: info.project_number.clone(),
        group_address_style,
        completion: required_completion(&info.completion_status, xpath, problems),
        last_modified: optional_timestamp(&info.last_modified, xpath, problems),
        project_start: optional_timestamp(&info.project_start, xpath, problems),
        // This mapper only ever handles schema-11 documents; schema ≥21/23
        // detection and wiring is Task 6's job.
        ets_schema_version: 11,
    }
}

#[allow(clippy::too_many_arguments)]
fn map_installation(
    installation: &SourceInstallation,
    source_path: &str,
    tables: &IdTables,
    ids: &mut IdAllocators,
    devices: &mut Devices,
    problems: &mut Vec<MapProblem>,
    counts: &mut EntityCounts,
) -> (Installation, Vec<RetainedAttribute>) {
    let mut retained = installation.other.clone();
    let xpath = "/KNX/Project/Installations/Installation";

    let id = match &installation.installation_id {
        Some(s) => s.parse::<u8>().map(InstallationId).unwrap_or_else(|_| {
            problems.push(MapProblem {
                xpath: xpath.to_string(),
                detail: MapProblemDetail::Value(ValueError::NotAnInteger {
                    field: "InstallationId",
                    value: s.clone(),
                }),
            });
            InstallationId(0)
        }),
        None => InstallationId(0),
    };

    let default_line = resolve_optional(
        &installation.default_line,
        &tables.lines,
        "Installation/@DefaultLine",
        xpath,
        problems,
    );
    let multicast_address =
        optional_ipv4(&installation.ip_routing_multicast_address, xpath, problems);
    let completion = required_completion(&installation.completion_status, xpath, problems);

    let mut topology = Topology {
        areas: Vec::new(),
        lines: Vec::new(),
        unassigned: Vec::new(),
    };
    let mut group_ranges = Vec::new();
    let mut group_addresses = Vec::new();
    let mut buildings = Vec::new();
    let mut parameters = Vec::new();

    for area in &installation.areas {
        let area_id = *tables
            .areas
            .get(&area.id)
            .expect("every area is allocated in pass 1");
        let area_xpath = format!("{xpath}/Topology/Area[@Id='{}']", area.id);
        let area_addr = required_u8(&area.address, "Area/@Address", &area_xpath, problems);

        let mut line_ids = Vec::new();
        for line in &area.lines {
            let line_id = *tables
                .lines
                .get(&line.id)
                .expect("every line is allocated in pass 1");
            let line_xpath = format!("{area_xpath}/Line[@Id='{}']", line.id);
            let line_addr = required_u8(&line.address, "Line/@Address", &line_xpath, problems);

            let mut device_ids = Vec::new();
            for device in &line.devices {
                let device_id = *tables
                    .devices
                    .get(&device.id)
                    .expect("every device is allocated in pass 1");
                let device_retained = map_device(
                    device,
                    device_id,
                    source_path,
                    Some(area_addr),
                    Some(line_addr),
                    tables,
                    ids,
                    devices,
                    &mut parameters,
                    &line_xpath,
                    problems,
                    counts,
                );
                retained.extend(device_retained);
                device_ids.push(device_id);
            }

            topology.lines.push(Line {
                id: line_id,
                source: SourceRef {
                    path: source_path.to_string(),
                    ets_id: line.id.clone(),
                },
                name: line.name.clone().unwrap_or_default(),
                address: line_addr,
                medium_ref: line.medium_type_ref_id.clone().unwrap_or_default(),
                domain_address: line.domain_address.clone(),
                domain_address_is_checked: optional_bool(
                    &line.domain_address_is_checked,
                    &line_xpath,
                    problems,
                ),
                ip_routing_multicast_address: optional_ipv4(
                    &line.ip_routing_multicast_address,
                    &line_xpath,
                    problems,
                ),
                multicast_ttl: optional_u8(
                    &line.multicast_ttl,
                    "MulticastTTL",
                    &line_xpath,
                    problems,
                ),
                completion: required_completion(&line.completion_status, &line_xpath, problems),
                devices: device_ids,
            });
            retained.extend(line.other.iter().cloned());
            counts.lines.bump();
            line_ids.push(line_id);
        }

        topology.areas.push(Area {
            id: area_id,
            source: SourceRef {
                path: source_path.to_string(),
                ets_id: area.id.clone(),
            },
            name: area.name.clone().unwrap_or_default(),
            address: area_addr,
            completion: required_completion(&area.completion_status, &area_xpath, problems),
            lines: line_ids,
        });
        retained.extend(area.other.iter().cloned());
        counts.areas.bump();
    }

    let unassigned_xpath = format!("{xpath}/Topology/UnassignedDevices");
    for device in &installation.unassigned_devices {
        let device_id = *tables
            .devices
            .get(&device.id)
            .expect("every device is allocated in pass 1");
        let device_retained = map_device(
            device,
            device_id,
            source_path,
            None,
            None,
            tables,
            ids,
            devices,
            &mut parameters,
            &unassigned_xpath,
            problems,
            counts,
        );
        retained.extend(device_retained);
        topology.unassigned.push(device_id);
    }

    for range in &installation.group_ranges {
        map_group_range(
            range,
            None,
            source_path,
            xpath,
            tables,
            &mut group_ranges,
            &mut group_addresses,
            &mut retained,
            problems,
            counts,
        );
    }

    for part in &installation.buildings {
        map_building_part(
            part,
            None,
            source_path,
            xpath,
            tables,
            &mut buildings,
            &mut retained,
            problems,
            counts,
        );
    }

    (
        Installation {
            id,
            name: installation.name.clone().unwrap_or_default(),
            default_line,
            multicast_address,
            completion,
            topology,
            buildings,
            group_ranges,
            group_addresses,
            parameters,
        },
        retained,
    )
}

/// Maps one device, its owned communication objects and its parameters,
/// inserting all of them directly into `devices`/`parameters` (the sole
/// owners, per DATA_MODEL §5/§10 — `knx_core::DeviceInstance` itself only
/// holds ids). Returns the device's and its com objects' retained
/// attributes for the caller to fold into the installation-wide list.
#[allow(clippy::too_many_arguments)]
fn map_device(
    device: &SourceDevice,
    device_id: DeviceId,
    source_path: &str,
    area_address: Option<u8>,
    line_address: Option<u8>,
    tables: &IdTables,
    ids: &mut IdAllocators,
    devices: &mut Devices,
    parameters: &mut Vec<ParameterInstance>,
    parent_xpath: &str,
    problems: &mut Vec<MapProblem>,
    counts: &mut EntityCounts,
) -> Vec<RetainedAttribute> {
    let xpath = format!("{parent_xpath}/DeviceInstance[@Id='{}']", device.id);
    let mut retained = device.other.clone();

    let address = compose_individual_address(
        area_address,
        line_address,
        device.address.as_deref(),
        &xpath,
        problems,
    );

    let mut com_object_ids = Vec::new();
    for com in &device.com_objects {
        let com_id = ids.next_com_object_instance_id();
        let (mapped, com_retained) = map_com_object(
            com,
            com_id,
            device_id,
            source_path,
            tables,
            &xpath,
            problems,
        );
        retained.extend(com_retained);
        devices.insert_com_object(mapped);
        com_object_ids.push(com_id);
        counts.com_objects.bump();
    }

    for param in &device.parameters {
        parameters.push(ParameterInstance {
            id: ids.next_parameter_instance_id(),
            device: device_id,
            source: SourceRef {
                path: source_path.to_string(),
                ets_id: param.ref_id.clone(),
            },
            raw: param.value.clone().unwrap_or_default(),
        });
        counts.parameters.bump();
    }

    let binary_data = device
        .binary_data
        .iter()
        .map(|b| BinaryDataRef {
            id: b.id.clone(),
            name: b.name.clone().unwrap_or_default(),
        })
        .collect();

    let commissioning = CommissioningState {
        completion: required_completion(&device.completion_status, &xpath, problems),
        individual_address_loaded: required_bool(
            &device.individual_address_loaded,
            &xpath,
            problems,
        ),
        application_program_loaded: required_bool(
            &device.application_program_loaded,
            &xpath,
            problems,
        ),
        parameters_loaded: required_bool(&device.parameters_loaded, &xpath, problems),
        communication_part_loaded: required_bool(
            &device.communication_part_loaded,
            &xpath,
            problems,
        ),
        medium_config_loaded: required_bool(&device.medium_config_loaded, &xpath, problems),
        last_modified: optional_timestamp(&device.last_modified, &xpath, problems),
        last_download: optional_timestamp(&device.last_download, &xpath, problems),
        broken: required_bool(&device.broken, &xpath, problems),
    };

    devices.insert(DeviceInstance {
        id: device_id,
        source: SourceRef {
            path: source_path.to_string(),
            ets_id: device.id.clone(),
        },
        name: device.name.clone().unwrap_or_default(),
        description: device.description.clone(),
        address,
        product_ref: device.product_ref_id.clone().unwrap_or_default(),
        program_ref: device.hardware2program_ref_id.clone().unwrap_or_default(),
        commissioning,
        visibility_calculated: required_bool(&device.visibility_calculated, &xpath, problems),
        com_objects: com_object_ids,
        binary_data,
    });
    counts.devices.bump();

    retained
}

fn map_com_object(
    com: &SourceComObjectInstance,
    com_id: ComObjectInstanceId,
    device_id: DeviceId,
    source_path: &str,
    tables: &IdTables,
    device_xpath: &str,
    problems: &mut Vec<MapProblem>,
) -> (ComObjectInstance, Vec<RetainedAttribute>) {
    let xpath = format!(
        "{device_xpath}/ComObjectInstanceRefs/ComObjectInstanceRef[@RefId='{}']",
        com.ref_id
    );
    let retained = com.other.clone();

    let number = match com_object_number(&com.ref_id) {
        Ok(n) => n,
        Err(e) => {
            problems.push(MapProblem {
                xpath: xpath.clone(),
                detail: MapProblemDetail::Value(e),
            });
            0
        }
    };

    let flags = ResolvedFlags {
        read: override_bool(&com.read_flag, &xpath, problems),
        write: override_bool(&com.write_flag, &xpath, problems),
        transmit: override_bool(&com.transmit_flag, &xpath, problems),
        update: override_bool(&com.update_flag, &xpath, problems),
        communication: override_bool(&com.communication_flag, &xpath, problems),
    };

    let mut links = Vec::new();
    for target in &com.sends {
        push_link(
            target,
            Direction::Send,
            &com.ref_id,
            &tables.group_addresses,
            &xpath,
            &mut links,
            problems,
        );
    }
    for target in &com.receives {
        push_link(
            target,
            Direction::Receive,
            &com.ref_id,
            &tables.group_addresses,
            &xpath,
            &mut links,
            problems,
        );
    }

    let instance = ComObjectInstance {
        id: com_id,
        source: SourceRef {
            path: source_path.to_string(),
            ets_id: com.ref_id.clone(),
        },
        device: device_id,
        number,
        text: override_text(&com.text),
        description: override_text(&com.description),
        dpt: override_dpt(&com.datapoint_type, &xpath, problems),
        flags,
        // Never stated at instance level in schema 11; filled from the
        // application program once the product database exists (Session 4).
        size: None,
        is_active: required_bool(&com.is_active, &xpath, problems),
        links,
        // Schema 11 has no `ModuleInstance` concept; module-based devices
        // (schema ≥21) are wired up starting with Task 6.
        module_instance: None,
    };

    (instance, retained)
}

fn push_link(
    target: &str,
    direction: Direction,
    com_ref_id: &str,
    table: &BTreeMap<String, GroupAddressId>,
    xpath: &str,
    links: &mut Vec<GroupLink>,
    problems: &mut Vec<MapProblem>,
) {
    match table.get(target) {
        Some(&ga) => links.push(GroupLink { ga, direction }),
        None => problems.push(MapProblem {
            xpath: xpath.to_string(),
            detail: MapProblemDetail::DroppedLink {
                com_object: com_ref_id.to_string(),
                group_address: target.to_string(),
            },
        }),
    }
}

#[allow(clippy::too_many_arguments)]
fn map_group_range(
    range: &SourceGroupRange,
    parent: Option<GroupRangeId>,
    source_path: &str,
    parent_xpath: &str,
    tables: &IdTables,
    group_ranges: &mut Vec<GroupRange>,
    group_addresses: &mut Vec<GroupAddressEntry>,
    retained: &mut Vec<RetainedAttribute>,
    problems: &mut Vec<MapProblem>,
    counts: &mut EntityCounts,
) -> GroupRangeId {
    let id = *tables
        .group_ranges
        .get(&range.id)
        .expect("every group range is allocated in pass 1");
    let xpath = format!("{parent_xpath}/GroupRanges/GroupRange[@Id='{}']", range.id);

    let start = required_ga(
        &range.range_start,
        "GroupRange/@RangeStart",
        &xpath,
        problems,
    );
    let end = required_ga(&range.range_end, "GroupRange/@RangeEnd", &xpath, problems);

    let mut child_ids = Vec::new();
    for child in &range.children {
        child_ids.push(map_group_range(
            child,
            Some(id),
            source_path,
            &xpath,
            tables,
            group_ranges,
            group_addresses,
            retained,
            problems,
            counts,
        ));
    }

    for address in &range.addresses {
        let ga_id = *tables
            .group_addresses
            .get(&address.id)
            .expect("every group address is allocated in pass 1");
        group_addresses.push(GroupAddressEntry {
            id: ga_id,
            source: SourceRef {
                path: source_path.to_string(),
                ets_id: address.id.clone(),
            },
            name: address.name.clone().unwrap_or_default(),
            address: required_ga(&address.address, "GroupAddress/@Address", &xpath, problems),
            central: required_bool(&address.central, &xpath, problems),
            unfiltered: required_bool(&address.unfiltered, &xpath, problems),
            range: Some(id),
        });
        retained.extend(address.other.iter().cloned());
        counts.group_addresses.bump();
    }

    group_ranges.push(GroupRange {
        id,
        source: SourceRef {
            path: source_path.to_string(),
            ets_id: range.id.clone(),
        },
        name: range.name.clone().unwrap_or_default(),
        start,
        end,
        parent,
        children: child_ids,
    });
    retained.extend(range.other.iter().cloned());
    counts.group_ranges.bump();

    id
}

#[allow(clippy::too_many_arguments)]
fn map_building_part(
    part: &SourceBuildingPart,
    parent: Option<BuildingPartId>,
    source_path: &str,
    parent_xpath: &str,
    tables: &IdTables,
    buildings: &mut Vec<BuildingPart>,
    retained: &mut Vec<RetainedAttribute>,
    problems: &mut Vec<MapProblem>,
    counts: &mut EntityCounts,
) -> BuildingPartId {
    let id = *tables
        .building_parts
        .get(&part.id)
        .expect("every building part is allocated in pass 1");
    let xpath = format!("{parent_xpath}/BuildingPart[@Id='{}']", part.id);

    let kind = match &part.kind {
        None => {
            problems.push(MapProblem {
                xpath: xpath.clone(),
                detail: MapProblemDetail::Value(ValueError::UnknownEnumValue {
                    kind: "BuildingPart/@Type",
                    value: String::new(),
                }),
            });
            BuildingPartType::BuildingPart
        }
        Some(s) => parse_building_part_type(s).unwrap_or_else(|e| {
            problems.push(MapProblem {
                xpath: xpath.clone(),
                detail: MapProblemDetail::Value(e),
            });
            BuildingPartType::BuildingPart
        }),
    };

    let default_line = resolve_optional(
        &part.default_line,
        &tables.lines,
        "BuildingPart/@DefaultLine",
        &xpath,
        problems,
    );
    let devices = resolve_many(
        &part.device_refs,
        &tables.devices,
        "DeviceInstanceRef",
        &xpath,
        problems,
    );

    let mut child_ids = Vec::new();
    for child in &part.children {
        child_ids.push(map_building_part(
            child,
            Some(id),
            source_path,
            &xpath,
            tables,
            buildings,
            retained,
            problems,
            counts,
        ));
    }

    buildings.push(BuildingPart {
        id,
        source: SourceRef {
            path: source_path.to_string(),
            ets_id: part.id.clone(),
        },
        name: part.name.clone().unwrap_or_default(),
        number: part.number.clone(),
        kind,
        default_line,
        completion: required_completion(&part.completion_status, &xpath, problems),
        children: child_ids,
        devices,
        parent,
    });
    retained.extend(part.other.iter().cloned());
    counts.building_parts.bump();

    id
}

// --- Small, single-purpose field converters ------------------------------
//
// Each follows the same shape: an absent `Option<String>` maps to the
// field's natural empty state (`false`, `None`, the type's `Default`) with
// no problem recorded — absence is normal, not an error; a present-but-
// unparseable value falls back to that same natural state *and* records a
// `MapProblem`, per the module doc comment's rule that a bad value never
// aborts the entity.

/// `parse_bool`'s error carries no field name (unlike `parse_u8`'s), so
/// this converter — unlike its numeric siblings — takes no `field` param.
fn required_bool(value: &Option<String>, xpath: &str, problems: &mut Vec<MapProblem>) -> bool {
    match value {
        None => false,
        Some(s) => parse_bool(s).unwrap_or_else(|e| {
            problems.push(MapProblem {
                xpath: xpath.to_string(),
                detail: MapProblemDetail::Value(e),
            });
            false
        }),
    }
}

fn optional_bool(
    value: &Option<String>,
    xpath: &str,
    problems: &mut Vec<MapProblem>,
) -> Option<bool> {
    let s = value.as_ref()?;
    match parse_bool(s) {
        Ok(b) => Some(b),
        Err(e) => {
            problems.push(MapProblem {
                xpath: xpath.to_string(),
                detail: MapProblemDetail::Value(e),
            });
            None
        }
    }
}

/// A required `u8` field with no natural absent-but-fine state (`Area`'s and
/// `Line`'s own `@Address`, which `knx_core` models as a bare `u8`, not an
/// `Option<u8>`): absence itself is recorded as a problem, since the field
/// has nowhere else to signal "not stated."
fn required_u8(
    value: &Option<String>,
    field: &'static str,
    xpath: &str,
    problems: &mut Vec<MapProblem>,
) -> u8 {
    match value {
        None => {
            problems.push(MapProblem {
                xpath: xpath.to_string(),
                detail: MapProblemDetail::Value(ValueError::NotAnInteger {
                    field,
                    value: String::new(),
                }),
            });
            0
        }
        Some(s) => parse_u8(s, field).unwrap_or_else(|e| {
            problems.push(MapProblem {
                xpath: xpath.to_string(),
                detail: MapProblemDetail::Value(e),
            });
            0
        }),
    }
}

fn optional_u8(
    value: &Option<String>,
    field: &'static str,
    xpath: &str,
    problems: &mut Vec<MapProblem>,
) -> Option<u8> {
    let s = value.as_ref()?;
    match parse_u8(s, field) {
        Ok(v) => Some(v),
        Err(e) => {
            problems.push(MapProblem {
                xpath: xpath.to_string(),
                detail: MapProblemDetail::Value(e),
            });
            None
        }
    }
}

fn optional_ipv4(
    value: &Option<String>,
    xpath: &str,
    problems: &mut Vec<MapProblem>,
) -> Option<Ipv4Addr> {
    let s = value.as_ref()?;
    match s.parse::<Ipv4Addr>() {
        Ok(v) => Some(v),
        Err(_) => {
            problems.push(MapProblem {
                xpath: xpath.to_string(),
                detail: MapProblemDetail::Value(ValueError::UnknownEnumValue {
                    kind: "IPRoutingMulticastAddress",
                    value: s.clone(),
                }),
            });
            None
        }
    }
}

fn optional_timestamp(
    value: &Option<String>,
    xpath: &str,
    problems: &mut Vec<MapProblem>,
) -> Option<DateTime<Utc>> {
    let s = value.as_ref()?;
    match parse_timestamp(s) {
        Ok(t) => Some(t),
        Err(e) => {
            problems.push(MapProblem {
                xpath: xpath.to_string(),
                detail: MapProblemDetail::Value(e),
            });
            None
        }
    }
}

fn required_completion(
    value: &Option<String>,
    xpath: &str,
    problems: &mut Vec<MapProblem>,
) -> CompletionStatus {
    match value {
        None => CompletionStatus::default(),
        Some(s) => parse_completion_status(s).unwrap_or_else(|e| {
            problems.push(MapProblem {
                xpath: xpath.to_string(),
                detail: MapProblemDetail::Value(e),
            });
            CompletionStatus::default()
        }),
    }
}

/// A required `GroupAddress` field (`GroupRange`'s bounds, a group
/// address's own value): same absence-is-a-problem shape as
/// [`required_u8`], since `knx_core` models these as bare values.
fn required_ga(
    value: &Option<String>,
    field: &'static str,
    xpath: &str,
    problems: &mut Vec<MapProblem>,
) -> GroupAddress {
    let raw = match value {
        None => {
            problems.push(MapProblem {
                xpath: xpath.to_string(),
                detail: MapProblemDetail::Value(ValueError::NotAnInteger {
                    field,
                    value: String::new(),
                }),
            });
            0
        }
        Some(s) => parse_u16(s, field).unwrap_or_else(|e| {
            problems.push(MapProblem {
                xpath: xpath.to_string(),
                detail: MapProblemDetail::Value(e),
            });
            0
        }),
    };
    GroupAddress::from_raw(raw)
}

/// `Text` never fails to convert — any string is a valid literal — so
/// unlike its siblings this converter takes no `xpath`/`problems`.
fn override_text(value: &Option<String>) -> Override<Text> {
    match value {
        None => Override::Absent,
        Some(s) if s.is_empty() => Override::Empty,
        Some(s) => Override::Value(Resolved {
            value: Text::Literal(s.clone()),
            layer: Layer::Instance,
        }),
    }
}

fn override_dpt(
    value: &Option<String>,
    xpath: &str,
    problems: &mut Vec<MapProblem>,
) -> Override<DptRef> {
    match value {
        None => Override::Absent,
        Some(s) if s.is_empty() => Override::Empty,
        Some(s) => match DptRef::parse(s) {
            Ok(dpt) => Override::Value(Resolved {
                value: dpt,
                layer: Layer::Instance,
            }),
            Err(_) => {
                problems.push(MapProblem {
                    xpath: xpath.to_string(),
                    detail: MapProblemDetail::Value(ValueError::UnknownEnumValue {
                        kind: "ComObjectInstanceRef/@DatapointType",
                        value: s.clone(),
                    }),
                });
                Override::Malformed(s.clone())
            }
        },
    }
}

fn override_bool(
    value: &Option<String>,
    xpath: &str,
    problems: &mut Vec<MapProblem>,
) -> Override<bool> {
    match value {
        None => Override::Absent,
        Some(s) if s.is_empty() => Override::Empty,
        Some(s) => match parse_bool(s) {
            Ok(b) => Override::Value(Resolved {
                value: b,
                layer: Layer::Instance,
            }),
            Err(e) => {
                problems.push(MapProblem {
                    xpath: xpath.to_string(),
                    detail: MapProblemDetail::Value(e),
                });
                Override::Malformed(s.clone())
            }
        },
    }
}

/// A device's individual address, composed from its enclosing `Area`'s and
/// `Line`'s already-resolved `@Address` and its own. Both `area`/`line`
/// being `None` (an unassigned device) and the device's own `@Address`
/// being absent are valid, unreported states — only a *present* value that
/// fails to parse or pack is a problem.
fn compose_individual_address(
    area: Option<u8>,
    line: Option<u8>,
    device_address: Option<&str>,
    xpath: &str,
    problems: &mut Vec<MapProblem>,
) -> Option<IndividualAddress> {
    let (area, line, device_address) = match (area, line, device_address) {
        (Some(a), Some(l), Some(d)) => (a, l, d),
        _ => return None,
    };
    let device = match parse_u8(device_address, "DeviceInstance/@Address") {
        Ok(d) => d,
        Err(e) => {
            problems.push(MapProblem {
                xpath: xpath.to_string(),
                detail: MapProblemDetail::Value(e),
            });
            return None;
        }
    };
    match IndividualAddress::new(area, line, device) {
        Ok(addr) => Some(addr),
        Err(_) => {
            problems.push(MapProblem {
                xpath: xpath.to_string(),
                detail: MapProblemDetail::Value(ValueError::NotAnInteger {
                    field: "IndividualAddress",
                    value: format!("{area}.{line}.{device}"),
                }),
            });
            None
        }
    }
}

fn resolve_optional<Id: Copy>(
    value: &Option<String>,
    table: &BTreeMap<String, Id>,
    kind: &'static str,
    xpath: &str,
    problems: &mut Vec<MapProblem>,
) -> Option<Id> {
    let s = value.as_ref()?;
    match table.get(s) {
        Some(&id) => Some(id),
        None => {
            problems.push(MapProblem {
                xpath: xpath.to_string(),
                detail: MapProblemDetail::UnresolvedReference {
                    kind,
                    target: s.clone(),
                },
            });
            None
        }
    }
}

fn resolve_many<Id: Copy>(
    values: &[String],
    table: &BTreeMap<String, Id>,
    kind: &'static str,
    xpath: &str,
    problems: &mut Vec<MapProblem>,
) -> Vec<Id> {
    values
        .iter()
        .filter_map(|s| match table.get(s) {
            Some(&id) => Some(id),
            None => {
                problems.push(MapProblem {
                    xpath: xpath.to_string(),
                    detail: MapProblemDetail::UnresolvedReference {
                        kind,
                        target: s.clone(),
                    },
                });
                None
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::{minimal_source_document, reference_source_document};

    #[test]
    fn every_imported_value_carries_the_instance_layer() {
        let out = map(&reference_source_document(), "P-0512/0.xml");
        let com = out
            .project
            .devices
            .iter()
            .flat_map(|d| d.com_objects.iter())
            .filter_map(|id| out.project.devices.com_object(*id))
            .find(|c| c.dpt.value().is_some())
            .unwrap();
        assert_eq!(com.dpt.layer(), Some(Layer::Instance));
        assert!(com.dpt.layer().unwrap().is_exported());
    }

    #[test]
    fn an_empty_datapoint_type_maps_to_empty_and_an_absent_one_to_absent() {
        let out = map(&reference_source_document(), "P-0512/0.xml");
        let mut empty = 0usize;
        let mut absent = 0usize;
        let mut valued = 0usize;
        let mut malformed = 0usize;
        for id in out
            .project
            .devices
            .iter()
            .flat_map(|d| d.com_objects.clone())
        {
            match out.project.devices.com_object(id).unwrap().dpt {
                Override::Empty => empty += 1,
                Override::Absent => absent += 1,
                Override::Value(_) => valued += 1,
                Override::Malformed(_) => malformed += 1,
            }
        }
        assert_eq!(empty, 497);
        assert_eq!(valued, 261);
        assert_eq!(absent, 149);
        assert_eq!(malformed, 0);
        assert_eq!(empty + valued + absent + malformed, 907);
    }

    #[test]
    fn a_device_address_is_composed_from_its_area_and_line() {
        let out = map(&reference_source_document(), "P-0512/0.xml");
        let d = out
            .project
            .devices
            .iter()
            .find(|d| d.name == "PM - Gardrobe")
            .unwrap();
        assert_eq!(d.address.unwrap().to_string(), "1.1.1");
    }

    #[test]
    fn the_unassigned_device_keeps_no_address_and_is_still_owned() {
        let out = map(&reference_source_document(), "P-0512/0.xml");
        let installation = &out.project.installations[0];
        assert_eq!(installation.topology.unassigned.len(), 1);
        let id = installation.topology.unassigned[0];
        assert!(out.project.devices.get(id).unwrap().address.is_none());
        assert_eq!(out.project.devices.iter().count(), 36);
    }

    #[test]
    fn a_bad_attribute_value_is_reported_and_the_rest_of_the_entity_still_maps() {
        let mut doc = minimal_source_document();
        doc.installations[0].areas[0].lines[0].devices[0].last_modified = Some("not a date".into());
        let out = map(&doc, "P-0001/0.xml");
        assert!(matches!(out.problems[0].detail, MapProblemDetail::Value(_)));
        let d = out.project.devices.iter().next().unwrap();
        assert_eq!(d.name, "D");
        assert!(d.commissioning.last_modified.is_none());
    }

    #[test]
    fn an_unparsable_override_keeps_its_raw_text_instead_of_disappearing() {
        let mut doc = minimal_source_document();
        let com = &mut doc.installations[0].areas[0].lines[0].devices[0].com_objects[0];
        com.datapoint_type = Some("DPST-nonsense".into());
        com.read_flag = Some("Perhaps".into());

        let out = map(&doc, "P-0001/0.xml");

        let mapped = out
            .project
            .devices
            .iter()
            .next()
            .and_then(|d| out.project.devices.com_object(d.com_objects[0]))
            .unwrap();
        assert_eq!(mapped.dpt, Override::Malformed("DPST-nonsense".into()));
        assert_eq!(mapped.flags.read, Override::Malformed("Perhaps".into()));
        // Reported, not silently kept: preserving the text is not the same
        // as pretending the value was understood.
        assert_eq!(
            out.problems
                .iter()
                .filter(|p| matches!(p.detail, MapProblemDetail::Value(_)))
                .count(),
            2
        );
    }

    #[test]
    fn known_but_unmodelled_attributes_are_retained_for_export() {
        let out = map(&reference_source_document(), "P-0512/0.xml");
        assert!(out
            .retained
            .iter()
            .any(|a| a.name == "BCUKey" && a.value == "4294967295"));
        assert!(out.retained.iter().any(|a| a.name == "SplitType"));
    }

    #[test]
    fn links_keep_their_direction() {
        let out = map(&reference_source_document(), "P-0512/0.xml");
        let (send, receive) = out
            .project
            .devices
            .iter()
            .flat_map(|d| d.com_objects.clone())
            .filter_map(|id| out.project.devices.com_object(id))
            .flat_map(|c| c.links.iter())
            .fold((0usize, 0usize), |(s, r), l| match l.direction {
                Direction::Send => (s + 1, r),
                Direction::Receive => (s, r + 1),
            });
        assert_eq!((send, receive), (569, 27));
    }
}
