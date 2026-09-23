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
    InstallationId, Language, Layer, Line, LineId, ModuleInstance, ModuleInstanceId, Override,
    ParameterInstance, Project, ProjectInfo, Resolved, ResolvedFlags, SourceRef, Text, Topology,
};

use crate::source::{
    RetainedAttribute, SourceBuildingPart, SourceComObjectInstance, SourceDevice, SourceDocument,
    SourceGroupRange, SourceInstallation,
};
use crate::values::{
    com_object_number, device_local_com_object_number, module_com_object_ref, parse_bool,
    parse_building_part_type, parse_completion_status, parse_timestamp, parse_u16, parse_u8,
    ValueError,
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
    pub module_instances: Count,
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
    if document.schema_version >= 21 {
        return map_v21(document, source_path);
    }

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

/// Schema-≥21 counterpart of [`map`], sharing its top-level shape exactly:
/// the same schema-agnostic `allocate_ids` pass, the same
/// `Project::new`/`MapOutput` assembly — only `map_project_info_v21` (for
/// `ProjectInfo::ets_schema_version`) and `map_installation_v21` (for
/// module-instance/`GroupObjectTree`-driven device mapping) differ.
fn map_v21(document: &SourceDocument, source_path: &str) -> MapOutput {
    let mut ids = IdAllocators::default();
    let tables = allocate_ids(document, &mut ids);
    // Schema ≥21's `ComObjectInstanceRef/@Links` names a group address by
    // its short id (`"GA-3"`), not the fully-qualified `@Id`
    // (`"P-03DE-0_GA-3"`) `tables.group_addresses` is keyed by — measured
    // against the KV reference project (RESEARCH §3.3/§3.4's "short id"
    // pattern, here on the group-address side rather than the com-object
    // side). Derived once from the already-built table, without touching
    // the schema-agnostic `allocate_ids` itself.
    let short_group_addresses = short_group_address_ids(&tables);

    let mut project = Project::new(Language("en".into()));

    let mut retained = Vec::new();
    let mut problems = Vec::new();
    let mut counts = EntityCounts::default();

    project.info = map_project_info_v21(document, &mut retained, &mut problems);

    for installation in &document.installations {
        let (mapped, installation_retained) = map_installation_v21(
            installation,
            source_path,
            &tables,
            &short_group_addresses,
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

/// The short form of every already-allocated group address id, e.g.
/// `"P-03DE-0_GA-3"` → `"GA-3"`. See [`map_v21`]'s call site for why this
/// exists. Silently skips any id that does not end in a `_GA-<n>` segment
/// rather than panicking — a document whose ids do not follow this pattern
/// just leaves schema-≥21 `Links` unresolved (reported by `push_link`'s
/// existing `DroppedLink`, same as any other dangling reference), not a
/// parse failure.
fn short_group_address_ids(tables: &IdTables) -> BTreeMap<String, GroupAddressId> {
    tables
        .group_addresses
        .iter()
        .filter_map(|(full, &id)| {
            full.rsplit_once('_')
                .filter(|(_, short)| short.starts_with("GA-"))
                .map(|(_, short)| (short.to_string(), id))
        })
        .collect()
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
        // documents go through `map_project_info_v21` below instead.
        ets_schema_version: 11,
    }
}

/// Schema-≥21 counterpart of [`map_project_info`]: a thin wrapper that
/// reuses every field conversion unchanged and only overrides
/// `ets_schema_version` with the document's real, detected version (21 or
/// 23). `document.info.project_traces_raw` (schema ≥21's `ProjectTraces`
/// audit log) is a [`crate::source::RetainedElement`], not a
/// `RetainedAttribute` — like `SourceLine/@BusAccess` (see the module doc
/// comment), it bypasses `MapOutput` entirely and reaches export straight
/// from `SourceDocument`, so there is nothing for this function to fold it
/// into.
fn map_project_info_v21(
    document: &SourceDocument,
    retained: &mut Vec<RetainedAttribute>,
    problems: &mut Vec<MapProblem>,
) -> ProjectInfo {
    let mut info = map_project_info(document, retained, problems);
    info.ets_schema_version = document.schema_version;
    info
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
                    &crate::xpath::device_v11(&device.id),
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
            retained.extend(keyed(&line.other, &line_xpath));
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
        retained.extend(keyed(&area.other, &area_xpath));
        counts.areas.bump();
    }

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
            &crate::xpath::unassigned_device(&device.id),
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
            "Buildings",
            "BuildingPart",
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
    xpath: &str,
    problems: &mut Vec<MapProblem>,
    counts: &mut EntityCounts,
) -> Vec<RetainedAttribute> {
    let xpath = xpath.to_string();
    let mut retained = keyed(&device.other, &xpath);

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
    for b in &device.binary_data {
        retained.extend(keyed(&b.other, &crate::xpath::binary_data(&xpath, &b.id)));
    }

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
        // Project intent, read from a file or a row. Nothing here is a
        // device fact, so the device side stays empty until a device
        // answers (§11.2).
        device_reported: Default::default(),
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
    let xpath = crate::xpath::com_object(device_xpath, &com.ref_id);
    let retained = keyed(&com.other, &xpath);

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
        // Deliberately absent. `ReadOnInitFlag` is measured 2533 times in
        // the local corpus, every one of them on an application program's
        // `ComObject` element and none on a `ComObjectInstanceRef` — not in
        // the ETS4 project, not in the ETS 6.3.0 one, not in the KV demo.
        // Guessing an instance-level attribute name would be worse than the
        // documented gap (KNOWN_LIMITATIONS §117), so the flag arrives from
        // the product database instead.
        read_on_init: Override::Absent,
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

/// Schema-≥21 counterpart of [`map_installation`]: the identical
/// area/line/unassigned-device loop shape and the identical, unchanged
/// `map_group_range`/`map_building_part` calls — `Segment` has already been
/// flattened away by the parser (Task 5), so `SourceLine::devices` already
/// holds exactly the right list either way. The only real difference is
/// mapping each device through [`map_device_v21`] instead of [`map_device`].
#[allow(clippy::too_many_arguments)]
fn map_installation_v21(
    installation: &SourceInstallation,
    source_path: &str,
    tables: &IdTables,
    short_group_addresses: &BTreeMap<String, GroupAddressId>,
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
        let area_xpath = crate::xpath::area(&area.id);
        let area_addr = required_u8(&area.address, "Area/@Address", &area_xpath, problems);

        let mut line_ids = Vec::new();
        for line in &area.lines {
            let line_id = *tables
                .lines
                .get(&line.id)
                .expect("every line is allocated in pass 1");
            let line_xpath = crate::xpath::line(&line.id);
            let segment_xpath = crate::xpath::segment(&line.id);
            let line_addr = required_u8(&line.address, "Line/@Address", &line_xpath, problems);

            let mut device_ids = Vec::new();
            for device in &line.devices {
                let device_id = *tables
                    .devices
                    .get(&device.id)
                    .expect("every device is allocated in pass 1");
                let device_retained = map_device_v21(
                    device,
                    device_id,
                    source_path,
                    Some(area_addr),
                    Some(line_addr),
                    short_group_addresses,
                    ids,
                    devices,
                    &mut parameters,
                    &crate::xpath::device_v21(&device.id),
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
            // `Segment`'s own attributes were folded into the enclosing
            // `Line` by the parser (`installation_v21.rs`'s `"Segment"`
            // arm), so they arrive here mixed in with the line's own and
            // are told apart by the path the parser gave them. They are
            // keyed by the owning line, not by the segment: `knx_core` has
            // no segment entity to key by, and the exporter synthesizes
            // exactly one segment per line. A line with two segments
            // therefore produces one key twice, which the export store
            // treats as ambiguous and refuses to write back — the right
            // answer, since there is no longer any way to tell which
            // segment a value came from.
            for attribute in &line.other {
                let owner = if attribute.xpath.ends_with("/Segment") {
                    &segment_xpath
                } else {
                    &line_xpath
                };
                retained.push(RetainedAttribute {
                    xpath: owner.clone(),
                    name: attribute.name.clone(),
                    value: attribute.value.clone(),
                });
            }
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
        retained.extend(keyed(&area.other, &area_xpath));
        counts.areas.bump();
    }

    for device in &installation.unassigned_devices {
        let device_id = *tables
            .devices
            .get(&device.id)
            .expect("every device is allocated in pass 1");
        let device_retained = map_device_v21(
            device,
            device_id,
            source_path,
            None,
            None,
            short_group_addresses,
            ids,
            devices,
            &mut parameters,
            &crate::xpath::unassigned_device(&device.id),
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
            "Locations",
            "Space",
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

/// Schema-≥21 counterpart of [`map_device`]. Parameters, binary data,
/// commissioning state and the final `DeviceInstance` are built exactly as
/// in `map_device` — `CompletionStatus`/`Broken` are absent at schema ≥21
/// (Global Constraints), but `required_completion`/`required_bool` already
/// treat `None` as the natural default with no spurious `MapProblem`, so no
/// special-casing is needed there.
///
/// Two things are genuinely new:
/// - Every `SourceModuleInstance` becomes a `knx_core::ModuleInstance`,
///   inserted into `devices` up front, so the com-object loop below can look
///   up each object's owning module instance by its `RefId` prefix.
/// - Communication objects are enumerated from `device.group_object_tree`
///   (ADR-0014's authoritative id list), not from `device.com_objects`
///   (which only holds the subset that carries an *instance-level override*
///   — text, DPT, links, …). An id with no override still produces a
///   `ComObjectInstance`, just one with every field at `Override::Absent`.
#[allow(clippy::too_many_arguments)]
fn map_device_v21(
    device: &SourceDevice,
    device_id: DeviceId,
    source_path: &str,
    area_address: Option<u8>,
    line_address: Option<u8>,
    short_group_addresses: &BTreeMap<String, GroupAddressId>,
    ids: &mut IdAllocators,
    devices: &mut Devices,
    parameters: &mut Vec<ParameterInstance>,
    xpath: &str,
    problems: &mut Vec<MapProblem>,
    counts: &mut EntityCounts,
) -> Vec<RetainedAttribute> {
    let xpath = xpath.to_string();
    let mut retained = keyed(&device.other, &xpath);

    let address = compose_individual_address(
        area_address,
        line_address,
        device.address.as_deref(),
        &xpath,
        problems,
    );

    // ModuleInstance construction first — the com-object mapping below needs
    // to know, for each GroupObjectTree id, whether it belongs to a module
    // in order to set `ComObjectInstance::module_instance`.
    let mut module_instance_ids: BTreeMap<String, ModuleInstanceId> = BTreeMap::new();
    for mi in &device.module_instances {
        let id = ids.next_module_instance_id();
        module_instance_ids.insert(mi.id.clone(), id);
        devices.insert_module_instance(ModuleInstance {
            id,
            device: device_id,
            source: SourceRef {
                path: source_path.to_string(),
                ets_id: mi.ref_id.clone(),
            },
            repeat_index: mi.repeat_index.clone().unwrap_or_default(),
            instance_ets_id: mi.id.clone(),
            arguments: mi
                .arguments
                .iter()
                .map(|a| {
                    (
                        SourceRef {
                            path: source_path.to_string(),
                            ets_id: a.ref_id.clone(),
                        },
                        a.value.clone().unwrap_or_default(),
                    )
                })
                .collect(),
        });
        counts.module_instances.bump();
    }

    // ComObjectInstanceRef overrides, keyed by RefId, for the lookup below.
    let overrides: BTreeMap<&str, &SourceComObjectInstance> = device
        .com_objects
        .iter()
        .map(|c| (c.ref_id.as_str(), c))
        .collect();

    // ADR-0014: GroupObjectTree is the authoritative id list. An id with no
    // override still produces a ComObjectInstance, Override::Absent.
    let mut com_object_ids = Vec::new();
    for ref_id in &device.group_object_tree {
        let com_id = ids.next_com_object_instance_id();
        let (mapped, com_retained) = map_com_object_v21(
            ref_id,
            overrides.get(ref_id.as_str()).copied(),
            com_id,
            device_id,
            source_path,
            &module_instance_ids,
            short_group_addresses,
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
    for b in &device.binary_data {
        retained.extend(keyed(&b.other, &crate::xpath::binary_data(&xpath, &b.id)));
    }

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
        // Project intent, read from a file or a row. Nothing here is a
        // device fact, so the device side stays empty until a device
        // answers (§11.2).
        device_reported: Default::default(),
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

/// Schema-≥21 counterpart of [`map_com_object`]. `ref_id` is the original,
/// unstripped `GroupObjectTree` id (e.g. `"MD-2_M-1_MI-1_O-2-0_R-4"`) — kept
/// verbatim in `ComObjectInstance::source::ets_id` rather than
/// `module_com_object_ref`'s short form, since later productdb-enrichment
/// tasks need the original to do their own transformation. Takes
/// `short_group_addresses` (see [`short_group_address_ids`]) rather than
/// `&IdTables` directly — `Links`'s targets are already short ids, and that
/// is the only cross-reference this function resolves.
#[allow(clippy::too_many_arguments)]
/// Re-keys the attributes one source element retained onto that element's
/// own instance xpath ([`crate::xpath`]).
///
/// The parser can only key a retained attribute by the *shape* of the
/// element it sat on (`".../DeviceInstance"`), because that is all a path
/// stack knows. Mapping is the first stage that holds the element's
/// identity, so it is where the key gains its `[@Id='…']` predicate. Every
/// device's `SerialNumber` used to collapse into one key here; now each one
/// keeps its own (`KNOWN_LIMITATIONS.md` §34).
fn keyed(attributes: &[RetainedAttribute], xpath: &str) -> Vec<RetainedAttribute> {
    attributes
        .iter()
        .map(|a| RetainedAttribute {
            xpath: xpath.to_string(),
            name: a.name.clone(),
            value: a.value.clone(),
        })
        .collect()
}

// Nine arguments, two of them added here: the device's own xpath, so the
// retained attributes can be keyed to this object rather than to every
// object in the project, and the problems it may report while doing so.
// A parameter struct would be a nicer signature and the same nine values.
#[allow(clippy::too_many_arguments)]
fn map_com_object_v21(
    ref_id: &str,
    over: Option<&SourceComObjectInstance>,
    com_id: ComObjectInstanceId,
    device_id: DeviceId,
    source_path: &str,
    module_instance_ids: &BTreeMap<String, ModuleInstanceId>,
    short_group_addresses: &BTreeMap<String, GroupAddressId>,
    device_xpath: &str,
    problems: &mut Vec<MapProblem>,
) -> (ComObjectInstance, Vec<RetainedAttribute>) {
    let xpath = crate::xpath::com_object(device_xpath, ref_id);
    let mut retained = match over {
        Some(c) => keyed(&c.other, &xpath),
        None => Vec::new(),
    };

    let (number, module_instance) = match module_com_object_ref(ref_id) {
        Ok((_short, number)) => {
            // "MD-<n>_M-<m>_MI-<k>_O-<a>-<b>_R-<c>" — `module_instance_ids`
            // is keyed by `SourceModuleInstance::id`, i.e. the
            // `ModuleInstance` element's own `@Id` (e.g. `"MD-2_M-1_MI-1"`,
            // *not* its `@RefId` `"MD-2_M-1"`, which only names the
            // `ModuleDef` it instantiates — a device can hold several
            // `ModuleInstance`s sharing one `@RefId`, distinguished by
            // `@Id`'s trailing `_MI-<k>`). That id is this `ref_id` with
            // just its own `_O-<a>-<b>_R-<c>` tail removed. Verified:
            // `"MD-2_M-1_MI-1_O-2-0_R-4".rsplitn(3, '_')` yields
            // `["R-4", "O-2-0", "MD-2_M-1_MI-1"]`, so `.nth(2)` is the
            // module-instance key — two segments dropped from the right
            // (`O-`, `R-`), not three; the brief's own prose description
            // ("keyed by the @RefId shape") named the wrong attribute, but
            // its `rsplitn(3, '_').nth(2)` arithmetic was actually right.
            let module_key = ref_id.rsplitn(3, '_').nth(2).unwrap_or(ref_id);
            (number, module_instance_ids.get(module_key).copied())
        }
        Err(_) => {
            // Not a module-based id. Three possibilities, in the order
            // they are tried:
            //
            // 1. Schema-11-shaped, fully qualified (`<program>_O-<n>_R-<m>`)
            //    — shouldn't occur under a schema-≥21 device, but handled.
            // 2. Device-local (`O-<n>_R-<m>`), which is what ETS 6 actually
            //    writes: all 867 `GroupObjectTree` ids in the ETS 6.3.0
            //    reference project take this form, and until this branch
            //    learned to read it, all 867 were reported as
            //    `MalformedRefId` and mapped to object number 0. They are
            //    not malformed — the application-program prefix is implied
            //    by the device rather than missing, and the object number
            //    is in the id (see `device_local_com_object_number`, whose
            //    doc comment carries the 867-of-867 measurement).
            // 3. Genuinely malformed — reported, never guessed at, and it
            //    must not abort the rest of the device.
            //
            // `module_instance` stays `None` in all three cases: only a
            // `MD-…` id names a module instance, and the ETS 6 reference
            // project spells no `ModuleInstance` element at all.
            match com_object_number(ref_id) {
                Ok(n) => (n, None),
                Err(e) => match device_local_com_object_number(ref_id) {
                    Ok(n) => (n, None),
                    Err(_) => {
                        problems.push(MapProblem {
                            xpath: xpath.clone(),
                            detail: MapProblemDetail::Value(e),
                        });
                        (0, None)
                    }
                },
            }
        }
    };

    let (text, description, dpt, flags, is_active, channel_id_retained) = match over {
        Some(c) => (
            override_text(&c.text),
            override_text(&c.description),
            override_dpt(&c.datapoint_type, &xpath, problems),
            // Schema ≥21 instance overrides *do* carry flags, contrary to
            // what this line asserted until now. ADR-0014's claim was
            // measured against `KV v2.5 - demo.knxproj` alone, where it
            // happens to hold: that project spells no flag on any of its
            // `ComObjectInstanceRef` elements. The ETS 6.3.0 reference
            // project (schema 23) carries 119 of them — `ReadFlag` 38,
            // `WriteFlag` 18, `TransmitFlag` 27, `UpdateFlag` 28,
            // `CommunicationFlag` 8 — spelled `"Enabled"`/`"Disabled"`,
            // exactly as schema 11 spells them. One sample proved the
            // wrong thing; the flags are resolved here now, the same way
            // `map_com_object` resolves them.
            ResolvedFlags {
                read: override_bool(&c.read_flag, &xpath, problems),
                write: override_bool(&c.write_flag, &xpath, problems),
                transmit: override_bool(&c.transmit_flag, &xpath, problems),
                update: override_bool(&c.update_flag, &xpath, problems),
                communication: override_bool(&c.communication_flag, &xpath, problems),
                // Absent for the same reason as at schema 11: no
                // `ReadOnInitFlag` occurs on a `ComObjectInstanceRef`
                // anywhere in the local corpus — not in the ETS4 project,
                // not in the ETS 6.3.0 one, not in the KV demo — so it
                // arrives from the product database instead
                // (KNOWN_LIMITATIONS §117).
                read_on_init: Override::Absent,
            },
            required_bool(&c.is_active, &xpath, problems),
            c.channel_id.clone().map(|v| RetainedAttribute {
                xpath: xpath.clone(),
                name: "ChannelId".into(),
                value: v,
            }),
        ),
        None => (
            Override::Absent,
            Override::Absent,
            Override::Absent,
            ResolvedFlags::none(),
            true,
            None,
        ),
    };
    retained.extend(channel_id_retained);

    let mut links = Vec::new();
    if let Some(c) = over {
        for target in &c.links {
            // Direction is not stated at schema ≥21 (Global Constraints
            // #2) — Send is a documented, flagged assumption, not an
            // invented fact; it does not affect export (Links is
            // regenerated from the GA id alone).
            //
            // `target` is `Links`'s short group-address id (`"GA-3"`), so
            // `push_link` — unchanged, reused as-is — is handed
            // `short_group_addresses` here rather than the fully-qualified
            // `tables.group_addresses` schema 11's `sends`/`receives` use.
            push_link(
                target,
                Direction::Send,
                ref_id,
                short_group_addresses,
                &xpath,
                &mut links,
                problems,
            );
        }
    }

    let instance = ComObjectInstance {
        id: com_id,
        source: SourceRef {
            path: source_path.to_string(),
            ets_id: ref_id.to_string(),
        },
        device: device_id,
        number,
        text,
        description,
        dpt,
        flags,
        // Never stated at instance level; filled from the application
        // program once the product database exists (Session 4), same as
        // schema 11.
        size: None,
        is_active,
        links,
        module_instance,
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
    let xpath = crate::xpath::group_range(&range.id);

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
        retained.extend(keyed(
            &address.other,
            &crate::xpath::group_address(&address.id),
        ));
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
    retained.extend(keyed(&range.other, &xpath));
    counts.group_ranges.bump();

    id
}

#[allow(clippy::too_many_arguments)]
fn map_building_part(
    part: &SourceBuildingPart,
    parent: Option<BuildingPartId>,
    source_path: &str,
    container: &str,
    element: &str,
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
    let xpath = crate::xpath::building_part(container, element, &part.id);

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
        // Includes documented Space types; genuinely unknown tokens still
        // surface as a mapping problem rather than silent normalization.
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
            container,
            element,
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
    retained.extend(keyed(&part.other, &xpath));
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
    use crate::testutil::{
        minimal_source_document, reference_kv_source_document, reference_source_document,
    };

    #[test]
    fn every_imported_value_carries_the_instance_layer() {
        if !crate::testutil::corpus_available() {
            eprintln!("skip: OriginalData/ corpus not present (gitignored, local-only)");
            return;
        }
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
        if !crate::testutil::corpus_available() {
            eprintln!("skip: OriginalData/ corpus not present (gitignored, local-only)");
            return;
        }
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
        if !crate::testutil::corpus_available() {
            eprintln!("skip: OriginalData/ corpus not present (gitignored, local-only)");
            return;
        }
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
        if !crate::testutil::corpus_available() {
            eprintln!("skip: OriginalData/ corpus not present (gitignored, local-only)");
            return;
        }
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
        if !crate::testutil::corpus_available() {
            eprintln!("skip: OriginalData/ corpus not present (gitignored, local-only)");
            return;
        }
        let out = map(&reference_source_document(), "P-0512/0.xml");
        assert!(out
            .retained
            .iter()
            .any(|a| a.name == "BCUKey" && a.value == "4294967295"));
        assert!(out.retained.iter().any(|a| a.name == "SplitType"));
    }

    #[test]
    fn links_keep_their_direction() {
        if !crate::testutil::corpus_available() {
            eprintln!("skip: OriginalData/ corpus not present (gitignored, local-only)");
            return;
        }
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

    #[test]
    fn a_module_based_device_maps_every_group_object_tree_id_even_without_an_override() {
        if !crate::testutil::corpus_available() {
            eprintln!("skip: OriginalData/ corpus not present (gitignored, local-only)");
            return;
        }
        let out = map(&reference_kv_source_document(), "P-03DE/0.xml");
        let device = out
            .project
            .devices
            .iter()
            .find(|d| !d.com_objects.is_empty())
            .unwrap();
        assert!(
            device.com_objects.len()
                > out
                    .project
                    .devices
                    .com_objects()
                    .filter(|c| c.device == device.id && c.text.is_present())
                    .count()
        );
    }

    #[test]
    fn a_module_based_com_object_carries_its_module_instance_id() {
        if !crate::testutil::corpus_available() {
            eprintln!("skip: OriginalData/ corpus not present (gitignored, local-only)");
            return;
        }
        let out = map(&reference_kv_source_document(), "P-03DE/0.xml");
        let com = out
            .project
            .devices
            .com_objects()
            .find(|c| c.module_instance.is_some())
            .unwrap();
        assert!(out
            .project
            .devices
            .module_instance(com.module_instance.unwrap())
            .is_some());
    }

    /// D38: the project's own `ModuleInstance/@Id` is retained as
    /// `instance_ets_id`, distinct from `source.ets_id` (`@RefId`) — this is
    /// the KV v2.5 demo's real switch actuator instance (design doc E1's
    /// table), not a synthesised value.
    #[test]
    fn a_module_instance_retains_its_id_distinct_from_its_ref_id() {
        if !crate::testutil::corpus_available() {
            eprintln!("skip: OriginalData/ corpus not present (gitignored, local-only)");
            return;
        }
        let out = map(&reference_kv_source_document(), "P-03DE/0.xml");
        let mi = out
            .project
            .devices
            .module_instances()
            .find(|m| m.source.ets_id == "MD-2_M-4")
            .unwrap();
        assert_eq!(mi.instance_ets_id, "MD-2_M-4_MI-1");
        assert_eq!(mi.source.ets_id, "MD-2_M-4");
        assert_ne!(mi.instance_ets_id, mi.source.ets_id);
    }

    #[test]
    fn schema_21_group_links_default_to_send_direction_documented_assumption() {
        if !crate::testutil::corpus_available() {
            eprintln!("skip: OriginalData/ corpus not present (gitignored, local-only)");
            return;
        }
        let out = map(&reference_kv_source_document(), "P-03DE/0.xml");
        let com = out
            .project
            .devices
            .com_objects()
            .find(|c| !c.links.is_empty())
            .unwrap();
        assert!(com.links.iter().all(|l| l.direction == Direction::Send));
    }

    #[test]
    fn ets_schema_version_is_recorded_on_the_project() {
        if !crate::testutil::corpus_available() {
            eprintln!("skip: OriginalData/ corpus not present (gitignored, local-only)");
            return;
        }
        let out = map(&reference_kv_source_document(), "P-03DE/0.xml");
        assert_eq!(out.project.info.ets_schema_version, 21);
    }

    /// Regression test for a review finding: a schema-≥21 `DeviceInstance`
    /// sits under `Line/Segment/`, not directly under `Line/` the way
    /// schema 11 does — a `MapProblem`'s xpath must reflect the real
    /// document structure, not schema 11's shallower one. The ancestor
    /// `Line` is no longer predicated by its own id ([`crate::xpath`]: the
    /// element's own id is what identifies it), so only the `/Segment/`
    /// step is asserted here.
    #[test]
    fn a_schema_21_map_problem_xpath_includes_the_segment_element() {
        if !crate::testutil::corpus_available() {
            eprintln!("skip: OriginalData/ corpus not present (gitignored, local-only)");
            return;
        }
        let mut doc = reference_kv_source_document();
        doc.installations[0]
            .areas
            .iter_mut()
            .flat_map(|a| a.lines.iter_mut())
            .find(|l| !l.devices.is_empty())
            .expect("the KV sample has at least one line with a device")
            .devices[0]
            .last_modified = Some("not a date".into());
        let out = map(&doc, "P-03DE/0.xml");
        let problem = out
            .problems
            .iter()
            .find(|p| matches!(p.detail, MapProblemDetail::Value(_)))
            .expect("the malformed LastModified value is reported");
        assert!(
            problem.xpath.contains("/Line/Segment/DeviceInstance["),
            "expected xpath to include .../Line/Segment/DeviceInstance[...], got: {}",
            problem.xpath
        );
    }

    /// Regression test for a re-opened review finding: the previous fix for
    /// the test above inserted `/Segment` *inside* `map_device_v21` itself,
    /// which also runs for `UnassignedDevices`-parented devices — a shape
    /// that has no `Segment` element (confirmed against `known.rs` and
    /// `export/schema21.rs`, both of which build
    /// `.../UnassignedDevices/DeviceInstance[...]` directly). An unassigned
    /// device's `MapProblem` xpath must not gain a `/Segment/` that doesn't
    /// exist in the real document.
    #[test]
    fn a_schema_21_unassigned_device_map_problem_xpath_has_no_segment_element() {
        if !crate::testutil::corpus_available() {
            eprintln!("skip: OriginalData/ corpus not present (gitignored, local-only)");
            return;
        }
        let mut doc = reference_kv_source_document();
        assert!(
            doc.installations[0].unassigned_devices.is_empty(),
            "KV sample has no unassigned devices; the hand-built device below is the only one"
        );
        let device = SourceDevice {
            id: "P-03DE-0_DI-unassigned-test".into(),
            last_modified: Some("not a date".into()),
            ..SourceDevice::default()
        };
        doc.installations[0].unassigned_devices.push(device);

        let out = map(&doc, "P-03DE/0.xml");
        let problem = out
            .problems
            .iter()
            .find(|p| matches!(p.detail, MapProblemDetail::Value(_)))
            .expect("the malformed LastModified value is reported");
        assert!(
            problem.xpath.contains("/UnassignedDevices/DeviceInstance["),
            "expected xpath to include .../UnassignedDevices/DeviceInstance[...], got: {}",
            problem.xpath
        );
        assert!(
            !problem.xpath.contains("/Segment/"),
            "unassigned devices have no Segment element in the real document, got: {}",
            problem.xpath
        );
    }
}
