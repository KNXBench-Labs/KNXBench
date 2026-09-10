//! `diff_projects`: the crate's one public entry point. Ties `key.rs`'s
//! generic matching engine together with `semantic.rs`'s per-entity field
//! extraction, per installation, per entity type, and hands back a fully
//! sorted, `HashMap`-free `ProjectDiff` (design spec §4).
//!
//! Devices get their own `DeviceTable`/`DeviceChange` pair instead of the
//! generic `EntityTable<DeviceKey, DeviceFields>` — nesting a device's
//! communication objects and parameters inside a `Fields` type used on both
//! `left` and `right` of a generic `EntityChange` would duplicate the
//! nested diff meaninglessly on both sides (orchestrator ruling, task-3
//! brief).

use std::collections::{BTreeMap, BTreeSet};

use knx_core::{
    Area, BuildingPart, BuildingPartId, ComObjectInstance, DeviceId, DeviceInstance, Devices,
    GroupAddressEntry, GroupAddressId, GroupRange, GroupRangeId, Installation, Line,
    ModuleInstanceId, ParameterInstance, Project, ProjectInfo, StringTable,
};

use crate::key::{
    match_entities, AmbiguityNote, EntityChange, EntityStatus, EntityTable, MatchKind,
};
use crate::semantic::*;

/// One field that differs between two matched entities, formatted for
/// display. Never `Debug`-formatted for a `String`/`Option<String>` field —
/// that would smuggle quotes into CLI output and the web panel (orchestrator
/// ruling 5). `Debug` is reserved for enums and structs with no plain
/// textual form of their own.
#[derive(Debug, Clone, PartialEq)]
pub struct FieldChange {
    pub field: &'static str,
    pub left: String,
    pub right: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProjectDiff {
    /// Empty when `left`/`right` agree on every field in the "Project info"
    /// comparison basis (design spec §3.5). `project_id` is deliberately
    /// excluded: comparing two files that are not the same container is the
    /// entire point of a diff.
    pub info_changes: Vec<FieldChange>,
    pub installations: Vec<InstallationDiff>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct InstallationDiff {
    pub id: u8,
    pub status: EntityStatus,
    /// Only populated when `status == EntityStatus::Matched`.
    pub field_changes: Vec<FieldChange>,
    pub areas: EntityTable<AreaKey, AreaFields>,
    pub lines: EntityTable<LineKey, LineFields>,
    pub devices: DeviceTable,
    pub group_ranges: EntityTable<GroupRangeKey, GroupRangeFields>,
    pub group_addresses: EntityTable<GroupAddressKey, GroupAddressFields>,
    pub buildings: EntityTable<BuildingPartKey, BuildingPartFields>,
}

/// Devices' own `added`/`removed`/`changed`/`ambiguous` table. Shaped like
/// `EntityTable` everywhere except `changed`, which carries `DeviceChange`
/// (nested `com_objects`/`parameters`) instead of the generic
/// `EntityChange`.
#[derive(Debug, Clone, PartialEq)]
pub struct DeviceTable {
    pub added: Vec<(DeviceKey, DeviceFields)>,
    pub removed: Vec<(DeviceKey, DeviceFields)>,
    pub changed: Vec<DeviceChange>,
    pub ambiguous: Vec<AmbiguityNote<DeviceKey>>,
}

/// A matched device pair. Emitted when `changed_fields` is non-empty **or**
/// either nested table (`com_objects`, `parameters`) is non-empty — a
/// device whose own fields are unchanged but which has one changed
/// communication object must still appear here, with an empty
/// `changed_fields` (orchestrator ruling 2; see
/// `a_device_with_unchanged_own_fields_but_a_changed_com_object_dpt_still_appears_in_changed`).
#[derive(Debug, Clone, PartialEq)]
pub struct DeviceChange {
    pub key: DeviceKey,
    pub matched_by: MatchKind,
    pub left: DeviceFields,
    pub right: DeviceFields,
    pub changed_fields: Vec<&'static str>,
    pub com_objects: EntityTable<ComObjectKey, ComObjectFields>,
    pub parameters: EntityTable<ParameterKey, ParameterFields>,
}

/// Computes what changed between `left` and `right`. Pure: no clock, no
/// filesystem, no RNG — the same two `Project` values always produce
/// `PartialEq`-equal `ProjectDiff`s (design spec §4).
pub fn diff_projects(left: &Project, right: &Project) -> ProjectDiff {
    let info_changes = diff_project_info(&left.info, &right.info);
    let installations = diff_installations(left, right);
    ProjectDiff {
        info_changes,
        installations,
    }
}

// ---------------------------------------------------------------------
// Field-change formatting (orchestrator ruling 5): plain text for a
// string-shaped field, `Debug` only for an enum or a struct with no plain
// textual form.
// ---------------------------------------------------------------------

fn fmt_plain(value: &str) -> String {
    value.to_string()
}

fn fmt_plain_opt(value: &Option<String>) -> String {
    match value {
        Some(s) => s.clone(),
        None => "-".to_string(),
    }
}

fn fmt_display_opt<T: std::fmt::Display>(value: &Option<T>) -> String {
    match value {
        Some(v) => v.to_string(),
        None => "-".to_string(),
    }
}

fn fmt_debug<T: std::fmt::Debug>(value: &T) -> String {
    format!("{value:?}")
}

// ---------------------------------------------------------------------
// Project info / installation scalar fields
// ---------------------------------------------------------------------

fn diff_project_info(left: &ProjectInfo, right: &ProjectInfo) -> Vec<FieldChange> {
    let mut changes = Vec::new();
    if left.name != right.name {
        changes.push(FieldChange {
            field: "name",
            left: fmt_plain(&left.name),
            right: fmt_plain(&right.name),
        });
    }
    if left.project_number != right.project_number {
        changes.push(FieldChange {
            field: "project_number",
            left: fmt_plain_opt(&left.project_number),
            right: fmt_plain_opt(&right.project_number),
        });
    }
    if left.group_address_style != right.group_address_style {
        changes.push(FieldChange {
            field: "group_address_style",
            left: fmt_debug(&left.group_address_style),
            right: fmt_debug(&right.group_address_style),
        });
    }
    if left.completion != right.completion {
        changes.push(FieldChange {
            field: "completion",
            left: fmt_debug(&left.completion),
            right: fmt_debug(&right.completion),
        });
    }
    changes
}

fn diff_installation_fields(left: &Installation, right: &Installation) -> Vec<FieldChange> {
    let mut changes = Vec::new();
    if left.name != right.name {
        changes.push(FieldChange {
            field: "name",
            left: fmt_plain(&left.name),
            right: fmt_plain(&right.name),
        });
    }
    if left.multicast_address != right.multicast_address {
        changes.push(FieldChange {
            field: "multicast_address",
            left: fmt_display_opt(&left.multicast_address),
            right: fmt_display_opt(&right.multicast_address),
        });
    }
    if left.completion != right.completion {
        changes.push(FieldChange {
            field: "completion",
            left: fmt_debug(&left.completion),
            right: fmt_debug(&right.completion),
        });
    }

    let left_default_line = left
        .default_line
        .and_then(|id| left.topology.line(id))
        .and_then(|line| line_key(line, &left.topology));
    let right_default_line = right
        .default_line
        .and_then(|id| right.topology.line(id))
        .and_then(|line| line_key(line, &right.topology));
    if left_default_line != right_default_line {
        changes.push(FieldChange {
            field: "default_line",
            left: fmt_debug(&left_default_line),
            right: fmt_debug(&right_default_line),
        });
    }

    changes
}

// ---------------------------------------------------------------------
// Installations
// ---------------------------------------------------------------------

fn diff_installations(left: &Project, right: &Project) -> Vec<InstallationDiff> {
    let left_by_id: BTreeMap<u8, &Installation> =
        left.installations.iter().map(|i| (i.id.0, i)).collect();
    let right_by_id: BTreeMap<u8, &Installation> =
        right.installations.iter().map(|i| (i.id.0, i)).collect();

    let mut ids: BTreeSet<u8> = left_by_id.keys().copied().collect();
    ids.extend(right_by_id.keys().copied());

    ids.into_iter()
        .map(|id| match (left_by_id.get(&id), right_by_id.get(&id)) {
            // An installation present on only one side is Added/Removed —
            // its own add/remove already accounts for everything it
            // contains, the same "container's own add/remove accounts for
            // its contents" principle ruling 2 states for a device's
            // com_objects/parameters, applied one level up (this is this
            // plan's own extension of that ruling, not a literal spec
            // requirement — the spec never names the installation case
            // directly).
            (Some(_), None) => empty_installation_diff(id, EntityStatus::Removed),
            (None, Some(_)) => empty_installation_diff(id, EntityStatus::Added),
            (Some(l), Some(r)) => diff_matched_installation(left, l, right, r, id),
            (None, None) => unreachable!("id came from the union of both id sets"),
        })
        .collect()
}

fn empty_installation_diff(id: u8, status: EntityStatus) -> InstallationDiff {
    InstallationDiff {
        id,
        status,
        field_changes: Vec::new(),
        areas: empty_table(),
        lines: empty_table(),
        devices: empty_device_table(),
        group_ranges: empty_table(),
        group_addresses: empty_table(),
        buildings: empty_table(),
    }
}

fn empty_table<K, F>() -> EntityTable<K, F> {
    EntityTable {
        added: Vec::new(),
        removed: Vec::new(),
        changed: Vec::new(),
        ambiguous: Vec::new(),
    }
}

fn empty_device_table() -> DeviceTable {
    DeviceTable {
        added: Vec::new(),
        removed: Vec::new(),
        changed: Vec::new(),
        ambiguous: Vec::new(),
    }
}

fn diff_matched_installation(
    project_left: &Project,
    left: &Installation,
    project_right: &Project,
    right: &Installation,
    id: u8,
) -> InstallationDiff {
    let field_changes = diff_installation_fields(left, right);

    let areas = diff_prepared(
        &prepare_areas(left),
        &prepare_areas(right),
        area_changed_fields,
    );
    let lines = diff_prepared(
        &prepare_lines(left),
        &prepare_lines(right),
        line_changed_fields,
    );
    let group_ranges = diff_prepared(
        &prepare_group_ranges(left),
        &prepare_group_ranges(right),
        group_range_changed_fields,
    );
    let group_addresses = diff_prepared(
        &prepare_group_addresses(project_left, left),
        &prepare_group_addresses(project_right, right),
        group_address_changed_fields,
    );
    let buildings = diff_prepared(
        &prepare_building_parts(left),
        &prepare_building_parts(right),
        building_part_changed_fields,
    );
    let devices = diff_devices(project_left, left, project_right, right);

    InstallationDiff {
        id,
        status: EntityStatus::Matched,
        field_changes,
        areas,
        lines,
        devices,
        group_ranges,
        group_addresses,
        buildings,
    }
}

// ---------------------------------------------------------------------
// The generic matching+diff engine over a pre-resolved `(ets_id, natural
// key, display key, fields)` tuple.
//
// Several entities' `*_key`/`*_fields` functions need side-specific context
// (a `Topology`, a `GroupAddressStyle`, a `BTreeMap<BuildingPartId, &..>`)
// that differs between `left`'s project and `right`'s — an area on the left
// must resolve through `left`'s own topology, never `right`'s. Resolving
// every entity's key/fields/natural-key up front, once per side, before
// `match_entities` ever runs, means the matching+diffing code below never
// needs to know which side an entity came from: `Prepared` already carries
// everything it needs.
// ---------------------------------------------------------------------

struct Prepared<K, F, NK> {
    ets_id: String,
    natural_key: Option<NK>,
    key: K,
    fields: F,
}

fn diff_prepared<K, F, NK>(
    left: &[Prepared<K, F, NK>],
    right: &[Prepared<K, F, NK>],
    changed_fields_of: impl Fn(&F, &F) -> Vec<&'static str>,
) -> EntityTable<K, F>
where
    K: Ord + Clone,
    F: Clone,
    NK: Ord + Clone,
{
    let outcome = match_entities(
        left,
        right,
        |p: &Prepared<K, F, NK>| p.ets_id.as_str(),
        |p: &Prepared<K, F, NK>| p.natural_key.clone(),
    );

    let mut added: Vec<(K, F)> = outcome
        .right_leftover
        .iter()
        .map(|p| (p.key.clone(), p.fields.clone()))
        .collect();
    added.sort_by(|a, b| a.0.cmp(&b.0));

    let mut removed: Vec<(K, F)> = outcome
        .left_leftover
        .iter()
        .map(|p| (p.key.clone(), p.fields.clone()))
        .collect();
    removed.sort_by(|a, b| a.0.cmp(&b.0));

    // A matched pair with zero differing fields produces no output at all
    // (design spec §3.3 step 5) — it is not "unchanged", it is simply
    // absent, the same convention `git diff` uses.
    let mut changed: Vec<EntityChange<K, F>> = outcome
        .matched
        .iter()
        .filter_map(|(l, r, kind)| {
            let changed_fields = changed_fields_of(&l.fields, &r.fields);
            if changed_fields.is_empty() {
                None
            } else {
                Some(EntityChange {
                    key: l.key.clone(),
                    matched_by: kind.clone(),
                    left: l.fields.clone(),
                    right: r.fields.clone(),
                    changed_fields,
                })
            }
        })
        .collect();
    changed.sort_by(|a, b| a.key.cmp(&b.key));

    // Per design spec §3.3 step 3: an ambiguous natural key is reported as
    // *both* individual adds/removes (already covered above, since
    // `match_entities` never subtracts an ambiguous candidate from its
    // leftover lists) *and* one `AmbiguityNote` here.
    let mut ambiguous: Vec<AmbiguityNote<K>> = outcome
        .ambiguous
        .iter()
        .map(|group| {
            let representative = group
                .left
                .first()
                .or_else(|| group.right.first())
                .expect("an AmbiguityGroup always has at least one member on some side");
            AmbiguityNote {
                key: representative.key.clone(),
                left_candidates: group.left.len(),
                right_candidates: group.right.len(),
            }
        })
        .collect();
    ambiguous.sort_by(|a, b| a.key.cmp(&b.key));

    EntityTable {
        added,
        removed,
        changed,
        ambiguous,
    }
}

// ---------------------------------------------------------------------
// Per-entity preparation: resolves each entity's own `ets_id`, natural key,
// display key and fields against its own side's context.
// ---------------------------------------------------------------------

fn prepare_areas(inst: &Installation) -> Vec<Prepared<AreaKey, AreaFields, AreaKey>> {
    inst.topology
        .areas
        .iter()
        .map(|a: &Area| {
            let key = area_key(a);
            Prepared {
                ets_id: a.source.ets_id.clone(),
                natural_key: Some(key.clone()),
                key,
                fields: area_fields(a),
            }
        })
        .collect()
}

/// A line unreachable from any `Area` in its own installation's topology
/// has no derivable `LineKey` at all (`line_key`'s own doc comment: "should
/// not occur for well-formed data"). Such a line is excluded from the diff
/// entirely, rather than invented a key for it — there is no meaningful key
/// to sort or match it by, and `line_key` already treats this as an honest
/// `None` rather than a value to guess at.
fn prepare_lines(inst: &Installation) -> Vec<Prepared<LineKey, LineFields, LineKey>> {
    inst.topology
        .lines
        .iter()
        .filter_map(|l: &Line| {
            let key = line_key(l, &inst.topology)?;
            Some(Prepared {
                ets_id: l.source.ets_id.clone(),
                natural_key: Some(key.clone()),
                key,
                fields: line_fields(l, &inst.topology),
            })
        })
        .collect()
}

fn prepare_group_ranges(
    inst: &Installation,
) -> Vec<Prepared<GroupRangeKey, GroupRangeFields, GroupRangeKey>> {
    let ranges_by_id: BTreeMap<GroupRangeId, &GroupRange> =
        inst.group_ranges.iter().map(|r| (r.id, r)).collect();
    inst.group_ranges
        .iter()
        .map(|r| {
            let key = group_range_key(r);
            Prepared {
                ets_id: r.source.ets_id.clone(),
                natural_key: Some(key.clone()),
                key,
                fields: group_range_fields(r, &ranges_by_id),
            }
        })
        .collect()
}

/// Matches by the raw 16-bit address, never the style-formatted string
/// (design spec §6's "one address-formatting note") — two sides that
/// disagree on `GroupAddressStyle` must still match the same underlying
/// address.
fn prepare_group_addresses(
    project: &Project,
    inst: &Installation,
) -> Vec<Prepared<GroupAddressKey, GroupAddressFields, u16>> {
    let style = project.info.group_address_style;
    let ranges_by_id: BTreeMap<GroupRangeId, &GroupRange> =
        inst.group_ranges.iter().map(|r| (r.id, r)).collect();
    inst.group_addresses
        .iter()
        .map(|g: &GroupAddressEntry| Prepared {
            ets_id: g.source.ets_id.clone(),
            natural_key: Some(g.address.raw()),
            key: group_address_key(g, style),
            fields: group_address_fields(g, &ranges_by_id),
        })
        .collect()
}

fn prepare_building_parts(
    inst: &Installation,
) -> Vec<Prepared<BuildingPartKey, BuildingPartFields, BuildingPartKey>> {
    let by_id: BTreeMap<BuildingPartId, &BuildingPart> =
        inst.buildings.iter().map(|b| (b.id, b)).collect();
    inst.buildings
        .iter()
        .map(|b| {
            let key = building_part_key(b, &by_id);
            Prepared {
                ets_id: b.source.ets_id.clone(),
                natural_key: Some(key.clone()),
                key,
                fields: building_part_fields(b, &inst.topology),
            }
        })
        .collect()
}

fn prepare_com_objects(
    device: &DeviceInstance,
    devices_store: &Devices,
    strings: &StringTable,
    ga_keys: &BTreeMap<GroupAddressId, GroupAddressKey>,
    module_ets_ids: &BTreeMap<ModuleInstanceId, String>,
    device_key: DeviceKey,
) -> Vec<Prepared<ComObjectKey, ComObjectFields, u16>> {
    device
        .com_objects
        .iter()
        .filter_map(|&id| devices_store.com_object(id))
        .map(|c: &ComObjectInstance| Prepared {
            ets_id: c.source.ets_id.clone(),
            natural_key: Some(c.number),
            key: com_object_key(c, device_key.clone()),
            fields: com_object_fields(c, strings, ga_keys, module_ets_ids),
        })
        .collect()
}

/// Per design spec §3.4: a parameter instance has no natural-key fallback
/// at all — its `ets_id` *is* its identity, so an unmatched one is a
/// genuine add/remove, never a candidate for guessing.
fn prepare_parameters(
    params: &[ParameterInstance],
    device_key: DeviceKey,
) -> Vec<Prepared<ParameterKey, ParameterFields, ()>> {
    params
        .iter()
        .map(|p| Prepared {
            ets_id: p.source.ets_id.clone(),
            natural_key: None,
            key: parameter_key(p, device_key.clone()),
            fields: parameter_fields(p),
        })
        .collect()
}

// ---------------------------------------------------------------------
// Devices — bespoke rather than routed through `diff_prepared`, since a
// `DeviceChange` needs the two matched devices' own `com_objects`/
// `parameters` tables built *after* matching, from each side's own project
// (ruling 1/ruling 2, task-3 brief).
// ---------------------------------------------------------------------

/// A device "belongs" to an installation if its id appears in any of that
/// installation's `topology.lines[*].devices` or in
/// `topology.unassigned` — mirrors
/// `crates/knx-etsproj/src/compare.rs:186-192`'s own precedent exactly,
/// since `Devices`/`Installation::parameters` are not themselves scoped by
/// installation in the domain model.
fn installation_device_ids(inst: &Installation) -> Vec<DeviceId> {
    let mut ids: Vec<DeviceId> = inst
        .topology
        .lines
        .iter()
        .flat_map(|l| l.devices.iter().copied())
        .collect();
    ids.extend(inst.topology.unassigned.iter().copied());
    ids
}

fn building_of_device_map(inst: &Installation) -> BTreeMap<DeviceId, BuildingPartId> {
    let mut map = BTreeMap::new();
    for part in &inst.buildings {
        for &device in &part.devices {
            map.insert(device, part.id);
        }
    }
    map
}

/// Bucketed by `device` id once per installation, per the brief: parameters
/// live on `Installation::parameters`, never walked from the device struct
/// itself.
fn group_parameters_by_device(
    params: &[ParameterInstance],
) -> BTreeMap<DeviceId, Vec<ParameterInstance>> {
    let mut map: BTreeMap<DeviceId, Vec<ParameterInstance>> = BTreeMap::new();
    for p in params {
        map.entry(p.device).or_default().push(p.clone());
    }
    map
}

fn module_ets_ids_map(project: &Project) -> BTreeMap<ModuleInstanceId, String> {
    project
        .devices
        .module_instances()
        .map(|m| (m.id, m.source.ets_id.clone()))
        .collect()
}

fn ga_keys_map(
    project: &Project,
    inst: &Installation,
) -> BTreeMap<GroupAddressId, GroupAddressKey> {
    let style = project.info.group_address_style;
    inst.group_addresses
        .iter()
        .map(|g| (g.id, group_address_key(g, style)))
        .collect()
}

fn table_is_empty<K, F>(table: &EntityTable<K, F>) -> bool {
    table.added.is_empty()
        && table.removed.is_empty()
        && table.changed.is_empty()
        && table.ambiguous.is_empty()
}

fn diff_devices(
    project_left: &Project,
    inst_left: &Installation,
    project_right: &Project,
    inst_right: &Installation,
) -> DeviceTable {
    let left_ids = installation_device_ids(inst_left);
    let right_ids = installation_device_ids(inst_right);

    let left_devices: Vec<DeviceInstance> = left_ids
        .iter()
        .filter_map(|&id| project_left.devices.get(id))
        .cloned()
        .collect();
    let right_devices: Vec<DeviceInstance> = right_ids
        .iter()
        .filter_map(|&id| project_right.devices.get(id))
        .cloned()
        .collect();

    // Only when `Some` (design spec §3.4): a device with no individual
    // address has no natural key at all.
    let outcome = match_entities(
        &left_devices,
        &right_devices,
        |d: &DeviceInstance| d.source.ets_id.as_str(),
        |d: &DeviceInstance| d.address.map(|a| a.to_string()),
    );

    let left_building_of_device = building_of_device_map(inst_left);
    let right_building_of_device = building_of_device_map(inst_right);
    let left_building_by_id: BTreeMap<BuildingPartId, &BuildingPart> =
        inst_left.buildings.iter().map(|b| (b.id, b)).collect();
    let right_building_by_id: BTreeMap<BuildingPartId, &BuildingPart> =
        inst_right.buildings.iter().map(|b| (b.id, b)).collect();

    let left_params_by_device = group_parameters_by_device(&inst_left.parameters);
    let right_params_by_device = group_parameters_by_device(&inst_right.parameters);

    let left_ga_keys = ga_keys_map(project_left, inst_left);
    let right_ga_keys = ga_keys_map(project_right, inst_right);
    let left_module_ets_ids = module_ets_ids_map(project_left);
    let right_module_ets_ids = module_ets_ids_map(project_right);

    let device_fields_left = |d: &DeviceInstance| -> DeviceFields {
        let line = line_of_device(d.id, &inst_left.topology);
        let building = left_building_of_device
            .get(&d.id)
            .and_then(|bid| left_building_by_id.get(bid))
            .map(|b| building_part_key(b, &left_building_by_id));
        device_fields(d, line, building)
    };
    let device_fields_right = |d: &DeviceInstance| -> DeviceFields {
        let line = line_of_device(d.id, &inst_right.topology);
        let building = right_building_of_device
            .get(&d.id)
            .and_then(|bid| right_building_by_id.get(bid))
            .map(|b| building_part_key(b, &right_building_by_id));
        device_fields(d, line, building)
    };

    // Ruling 2: an added/removed device's `com_objects`/`parameters` are
    // not diffed at all — its own add/remove already accounts for them.
    let mut added: Vec<(DeviceKey, DeviceFields)> = outcome
        .right_leftover
        .iter()
        .map(|d| (device_key(d), device_fields_right(d)))
        .collect();
    added.sort_by(|a, b| a.0.cmp(&b.0));

    let mut removed: Vec<(DeviceKey, DeviceFields)> = outcome
        .left_leftover
        .iter()
        .map(|d| (device_key(d), device_fields_left(d)))
        .collect();
    removed.sort_by(|a, b| a.0.cmp(&b.0));

    let empty_params: Vec<ParameterInstance> = Vec::new();
    let mut changed: Vec<DeviceChange> = Vec::new();
    for (l, r, kind) in &outcome.matched {
        let left_fields = device_fields_left(l);
        let right_fields = device_fields_right(r);
        let changed_fields = device_changed_fields(&left_fields, &right_fields);
        let key = device_key(l);

        let left_params = left_params_by_device.get(&l.id).unwrap_or(&empty_params);
        let right_params = right_params_by_device.get(&r.id).unwrap_or(&empty_params);

        let com_objects = diff_prepared(
            &prepare_com_objects(
                l,
                &project_left.devices,
                &project_left.strings,
                &left_ga_keys,
                &left_module_ets_ids,
                key.clone(),
            ),
            &prepare_com_objects(
                r,
                &project_right.devices,
                &project_right.strings,
                &right_ga_keys,
                &right_module_ets_ids,
                key.clone(),
            ),
            com_object_changed_fields,
        );
        let parameters = diff_prepared(
            &prepare_parameters(left_params, key.clone()),
            &prepare_parameters(right_params, key.clone()),
            parameter_changed_fields,
        );

        // Ruling 1: emit a `DeviceChange` when `changed_fields` is
        // non-empty **or** either nested table is non-empty — a device
        // whose own fields are unchanged but which has one changed
        // communication object must still appear here.
        if changed_fields.is_empty() && table_is_empty(&com_objects) && table_is_empty(&parameters)
        {
            continue;
        }

        changed.push(DeviceChange {
            key,
            matched_by: kind.clone(),
            left: left_fields,
            right: right_fields,
            changed_fields,
            com_objects,
            parameters,
        });
    }
    changed.sort_by(|a, b| a.key.cmp(&b.key));

    let mut ambiguous: Vec<AmbiguityNote<DeviceKey>> = outcome
        .ambiguous
        .iter()
        .map(|group| {
            let representative = group
                .left
                .first()
                .or_else(|| group.right.first())
                .expect("an AmbiguityGroup always has at least one member on some side");
            AmbiguityNote {
                key: device_key(representative),
                left_candidates: group.left.len(),
                right_candidates: group.right.len(),
            }
        })
        .collect();
    ambiguous.sort_by(|a, b| a.key.cmp(&b.key));

    DeviceTable {
        added,
        removed,
        changed,
        ambiguous,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::*;
    use knx_core::{
        BuildingPartType, ComObjectInstanceId, Command, CompletionStatus, Direction, DptRef,
        InstallationId, Layer, ModuleInstance, Topology,
    };

    /// One of every entity kind this design covers: an area, a line, a
    /// device with an individual address, a device without one, a group
    /// range, a group address inside it, a building part, a schema-≥21
    /// module-based device (a com object with `module_instance` set) and a
    /// schema-11 monolithic one (a com object without).
    fn full_project() -> Project {
        let mut project = project();

        let area = add_area(&mut project, 1, 1);
        let line = add_line(&mut project, area, 1, 1);

        let device_with_addr = add_device(&mut project, 1, Some(line));
        project.devices.get_mut(device_with_addr).unwrap().address =
            Some(individual_address(1, 1, 5));
        let _device_without_addr = add_device(&mut project, 2, Some(line));

        let building = add_building_part(&mut project, 1, "Room", BuildingPartType::Room, None);
        if let Some(b) = project.installations[0]
            .buildings
            .iter_mut()
            .find(|b| b.id == building)
        {
            b.devices.push(device_with_addr);
        }

        let group_range = add_group_range(&mut project, 1, "Range", 0, 100, None);
        let group_address = add_group_address(&mut project, 1, 5, "GA", Some(group_range));

        add_parameter(&mut project, 1, device_with_addr, "P-1", "raw-value");

        // Monolithic (schema 11) communication object: no `module_instance`.
        let mut com1 = com_object(1, device_with_addr, 1);
        com1.dpt = dpt_value(1, Some(1), Layer::Instance);
        com1.links.push(link(group_address.0, Direction::Send));
        project.devices.insert_com_object(com1);
        project
            .devices
            .get_mut(device_with_addr)
            .unwrap()
            .com_objects
            .push(ComObjectInstanceId(1));

        // Module-based (schema >= 21) device and communication object.
        let device_module = add_device(&mut project, 3, Some(line));
        project.devices.insert_module_instance(ModuleInstance {
            id: module_id(1),
            device: device_module,
            source: source("MD-1"),
            repeat_index: "1x1".into(),
            arguments: vec![],
        });
        let mut com2 = com_object(2, device_module, 1);
        com2.module_instance = Some(module_id(1));
        project.devices.insert_com_object(com2);
        project
            .devices
            .get_mut(device_module)
            .unwrap()
            .com_objects
            .push(ComObjectInstanceId(2));

        project
    }

    fn assert_table_empty<K, F>(table: &EntityTable<K, F>) {
        assert!(table.added.is_empty(), "expected no added entries");
        assert!(table.removed.is_empty(), "expected no removed entries");
        assert!(table.changed.is_empty(), "expected no changed entries");
        assert!(table.ambiguous.is_empty(), "expected no ambiguous entries");
    }

    fn assert_device_table_empty(table: &DeviceTable) {
        assert!(table.added.is_empty(), "expected no added devices");
        assert!(table.removed.is_empty(), "expected no removed devices");
        assert!(table.changed.is_empty(), "expected no changed devices");
        assert!(table.ambiguous.is_empty(), "expected no ambiguous devices");
    }

    #[test]
    fn diff_of_a_project_against_itself_is_empty_at_every_level() {
        let project = full_project();

        let diff = diff_projects(&project, &project);

        assert!(diff.info_changes.is_empty());
        assert_eq!(diff.installations.len(), 1);
        let inst = &diff.installations[0];
        assert_eq!(inst.status, EntityStatus::Matched);
        assert!(inst.field_changes.is_empty());
        assert_table_empty(&inst.areas);
        assert_table_empty(&inst.lines);
        assert_device_table_empty(&inst.devices);
        assert_table_empty(&inst.group_ranges);
        assert_table_empty(&inst.group_addresses);
        assert_table_empty(&inst.buildings);
    }

    #[test]
    fn running_diff_projects_twice_on_the_same_inputs_produces_equal_results() {
        // `Project` does not derive `Clone` (its `StringTable` does not),
        // so `left`/`right` are two independently built, structurally
        // identical fixtures rather than one cloned into the other —
        // `full_project`'s ids are all explicit, so two calls produce the
        // same shape every time.
        let left = full_project();
        let mut right = full_project();
        // A non-trivial, but arbitrary, edit — the test cares only that
        // repeating the same comparison twice yields identical results, not
        // about what the edit is.
        right.installations[0].topology.areas[0].name = "A different area name".into();

        let first = diff_projects(&left, &right);
        let second = diff_projects(&left, &right);

        assert_eq!(first, second);
    }

    /// A project with one unassigned device carrying one communication
    /// object (`dpt: Override::Absent`). Both `left` and `right` in the
    /// tests below are built by calling this twice — see
    /// `running_diff_projects_twice_on_the_same_inputs_produces_equal_results`'s
    /// comment for why (`Project` has no `Clone`).
    fn device_with_one_com_object() -> Project {
        let mut project = project();
        let device = add_device(&mut project, 1, None);
        let com = com_object(1, device, 1); // dpt: Override::Absent
        project.devices.insert_com_object(com);
        project
            .devices
            .get_mut(device)
            .unwrap()
            .com_objects
            .push(ComObjectInstanceId(1));
        project
    }

    #[test]
    fn a_device_with_unchanged_own_fields_but_a_changed_com_object_dpt_still_appears_in_changed() {
        let left = device_with_one_com_object();
        let mut right = device_with_one_com_object();
        Command::SetComObjectDpt {
            com_object: ComObjectInstanceId(1),
            dpt: Some(DptRef {
                main: 1,
                sub: Some(1),
            }),
        }
        .apply(&mut right)
        .unwrap();

        let diff = diff_projects(&left, &right);
        let inst = &diff.installations[0];

        assert_eq!(inst.devices.changed.len(), 1);
        let device_change = &inst.devices.changed[0];
        assert!(
            device_change.changed_fields.is_empty(),
            "the device's own fields did not change: {:?}",
            device_change.changed_fields
        );
        assert_eq!(device_change.com_objects.changed.len(), 1);
        assert_eq!(
            device_change.com_objects.changed[0].changed_fields,
            vec!["dpt"]
        );
    }

    #[test]
    fn an_added_or_removed_devices_nested_tables_are_empty() {
        let left = project();
        let mut right = project();

        let device = add_device(&mut right, 1, None);
        let com = com_object(1, device, 1);
        right.devices.insert_com_object(com);
        right
            .devices
            .get_mut(device)
            .unwrap()
            .com_objects
            .push(ComObjectInstanceId(1));
        add_parameter(&mut right, 1, device, "P-1", "some-value");

        let diff = diff_projects(&left, &right);
        let inst = &diff.installations[0];

        // Ruling 2/3: `DeviceTable::added`/`removed` entries are plain
        // `(DeviceKey, DeviceFields)` pairs — the type itself carries no
        // `com_objects`/`parameters` field to inspect, which is a stronger
        // guarantee than checking those fields are empty: there is nowhere
        // for the added device's communication objects/parameters to have
        // been diffed into at all. This is witnessed here by there being no
        // `DeviceChange` (and hence no nested tables) for the added device.
        assert_eq!(inst.devices.added.len(), 1);
        assert!(inst.devices.removed.is_empty());
        assert!(
            inst.devices.changed.is_empty(),
            "an added device must never produce a DeviceChange (which is the \
             only place com_objects/parameters tables exist)"
        );
        assert!(inst.devices.ambiguous.is_empty());
    }

    #[test]
    fn an_ambiguous_natural_key_never_appears_in_changed() {
        let mut left = project();
        let left_a = add_device(&mut left, 1, None);
        left.devices.get_mut(left_a).unwrap().address = Some(individual_address(1, 1, 1));
        let left_b = add_device(&mut left, 2, None);
        left.devices.get_mut(left_b).unwrap().address = Some(individual_address(1, 1, 1));

        let mut right = project();
        let right_a = add_device(&mut right, 11, None);
        right.devices.get_mut(right_a).unwrap().address = Some(individual_address(1, 1, 1));
        let right_b = add_device(&mut right, 12, None);
        right.devices.get_mut(right_b).unwrap().address = Some(individual_address(1, 1, 1));

        let diff = diff_projects(&left, &right);
        let inst = &diff.installations[0];

        assert!(inst.devices.changed.is_empty());
        assert_eq!(inst.devices.added.len(), 2);
        assert_eq!(inst.devices.removed.len(), 2);
        assert_eq!(inst.devices.ambiguous.len(), 1);
        assert_eq!(inst.devices.ambiguous[0].left_candidates, 2);
        assert_eq!(inst.devices.ambiguous[0].right_candidates, 2);
    }

    #[test]
    fn installation_added_and_removed_carry_empty_nested_tables() {
        let left = project();
        let mut right = project();

        let device = DeviceId(100);
        right.devices.insert(DeviceInstance {
            id: device,
            source: source("D-100"),
            name: "Device 100".into(),
            description: None,
            address: None,
            product_ref: "P-0".into(),
            program_ref: "H-0".into(),
            commissioning: Default::default(),
            visibility_calculated: true,
            com_objects: vec![],
            binary_data: vec![],
        });
        right.installations.push(Installation {
            id: InstallationId(1),
            name: "Second installation".into(),
            default_line: None,
            multicast_address: None,
            completion: CompletionStatus::default(),
            topology: Topology {
                areas: vec![Area {
                    id: knx_core::AreaId(1),
                    source: source("A-1"),
                    name: "Area 1".into(),
                    address: 1,
                    completion: CompletionStatus::default(),
                    lines: vec![],
                }],
                lines: vec![],
                unassigned: vec![device],
            },
            buildings: vec![],
            group_ranges: vec![],
            group_addresses: vec![],
            parameters: vec![],
        });

        let diff = diff_projects(&left, &right);

        assert_eq!(diff.installations.len(), 2);
        let added = diff
            .installations
            .iter()
            .find(|i| i.id == 1)
            .expect("installation 1 present in the diff");

        // This plan's own extension of ruling 2, one level up (not a
        // literal spec requirement): an installation that only exists on
        // one side carries every nested table empty, even though its own
        // topology/devices/etc. are not — its own add/remove already
        // accounts for everything inside it.
        assert_eq!(added.status, EntityStatus::Added);
        assert!(added.field_changes.is_empty());
        assert_table_empty(&added.areas);
        assert_table_empty(&added.lines);
        assert_device_table_empty(&added.devices);
        assert_table_empty(&added.group_ranges);
        assert_table_empty(&added.group_addresses);
        assert_table_empty(&added.buildings);
    }

    /// One area, one line, two devices, one group range with one group
    /// address inside it, one building part, one parameter (on `device_a`)
    /// and one communication object (also on `device_a`) — every entity
    /// kind exactly once, with all-explicit ids so calling this twice
    /// produces two structurally identical `Project`s (`Project` has no
    /// `Clone` — see
    /// `running_diff_projects_twice_on_the_same_inputs_produces_equal_results`'s
    /// comment).
    #[allow(clippy::type_complexity)]
    fn build_big_fixture() -> (
        Project,
        knx_core::AreaId,
        knx_core::LineId,
        DeviceId,
        DeviceId,
        knx_core::GroupRangeId,
        knx_core::GroupAddressId,
        knx_core::BuildingPartId,
    ) {
        let mut project = project();
        let area = add_area(&mut project, 1, 1);
        let line = add_line(&mut project, area, 1, 1);

        // Two separate devices, one per device-field-kind change, so each
        // one's `changed_fields` names exactly one field.
        let device_a = add_device(&mut project, 1, Some(line));
        let device_b = add_device(&mut project, 2, Some(line));

        let group_range = add_group_range(&mut project, 1, "Range", 0, 100, None);
        let group_address = add_group_address(&mut project, 1, 5, "GA", Some(group_range));
        let building = add_building_part(&mut project, 1, "Room", BuildingPartType::Room, None);
        add_parameter(&mut project, 1, device_a, "P-1", "raw-before");

        let com = com_object(1, device_a, 1);
        project.devices.insert_com_object(com);
        project
            .devices
            .get_mut(device_a)
            .unwrap()
            .com_objects
            .push(ComObjectInstanceId(1));

        (
            project,
            area,
            line,
            device_a,
            device_b,
            group_range,
            group_address,
            building,
        )
    }

    #[test]
    fn every_entity_kinds_own_change_is_reported_exactly_once_in_the_right_table_with_the_right_changed_fields(
    ) {
        let (left, area, line, device_a, device_b, group_range, group_address, building) =
            build_big_fixture();
        let (mut right, _, _, _, _, _, _, _) = build_big_fixture();

        // 1. Device description.
        Command::SetDeviceDescription {
            device: device_a,
            description: Some("new description".into()),
        }
        .apply(&mut right)
        .unwrap();

        // 2. Device product_ref — no Command exists.
        right.devices.get_mut(device_b).unwrap().product_ref = "P-new".into();

        // 3. Communication object DPT.
        Command::SetComObjectDpt {
            com_object: ComObjectInstanceId(1),
            dpt: Some(DptRef {
                main: 1,
                sub: Some(1),
            }),
        }
        .apply(&mut right)
        .unwrap();

        // 4. Group address rename.
        let entry = right.installations[0]
            .group_addresses
            .iter()
            .find(|e| e.id == group_address)
            .unwrap()
            .clone();
        Command::UpdateGroupAddress {
            id: group_address,
            name: "renamed".into(),
            central: entry.central,
            unfiltered: entry.unfiltered,
        }
        .apply(&mut right)
        .unwrap();

        // 5. Group range rename.
        Command::RenameGroupRange {
            id: group_range,
            name: "renamed".into(),
        }
        .apply(&mut right)
        .unwrap();

        // 6. Building part rename.
        Command::RenameBuildingPart {
            id: building,
            name: "renamed".into(),
        }
        .apply(&mut right)
        .unwrap();

        // 7. Area rename — no Command exists.
        right.installations[0]
            .topology
            .areas
            .iter_mut()
            .find(|a| a.id == area)
            .unwrap()
            .name = "renamed area".into();

        // 8. Line rename — no Command exists.
        right.installations[0]
            .topology
            .lines
            .iter_mut()
            .find(|l| l.id == line)
            .unwrap()
            .name = "renamed line".into();

        // Parameter value — no Command exists. (Nested under device_a; not
        // one of the eight top-level entity-kind changes, but exercised
        // here too since it is otherwise untested by this fixture.)
        right.installations[0].parameters[0].raw = "raw-after".into();

        let diff = diff_projects(&left, &right);
        assert!(diff.info_changes.is_empty());
        assert_eq!(diff.installations.len(), 1);
        let inst = &diff.installations[0];
        assert!(inst.field_changes.is_empty());

        // Area
        assert!(inst.areas.added.is_empty());
        assert!(inst.areas.removed.is_empty());
        assert!(inst.areas.ambiguous.is_empty());
        assert_eq!(inst.areas.changed.len(), 1);
        assert_eq!(inst.areas.changed[0].changed_fields, vec!["name"]);

        // Line
        assert!(inst.lines.added.is_empty());
        assert!(inst.lines.removed.is_empty());
        assert!(inst.lines.ambiguous.is_empty());
        assert_eq!(inst.lines.changed.len(), 1);
        assert_eq!(inst.lines.changed[0].changed_fields, vec!["name"]);

        // Group range
        assert!(inst.group_ranges.added.is_empty());
        assert!(inst.group_ranges.removed.is_empty());
        assert!(inst.group_ranges.ambiguous.is_empty());
        assert_eq!(inst.group_ranges.changed.len(), 1);
        assert_eq!(inst.group_ranges.changed[0].changed_fields, vec!["name"]);

        // Group address
        assert!(inst.group_addresses.added.is_empty());
        assert!(inst.group_addresses.removed.is_empty());
        assert!(inst.group_addresses.ambiguous.is_empty());
        assert_eq!(inst.group_addresses.changed.len(), 1);
        assert_eq!(inst.group_addresses.changed[0].changed_fields, vec!["name"]);

        // Building part
        assert!(inst.buildings.added.is_empty());
        assert!(inst.buildings.removed.is_empty());
        assert!(inst.buildings.ambiguous.is_empty());
        assert_eq!(inst.buildings.changed.len(), 1);
        assert_eq!(inst.buildings.changed[0].changed_fields, vec!["name"]);

        // Devices
        assert!(inst.devices.added.is_empty());
        assert!(inst.devices.removed.is_empty());
        assert!(inst.devices.ambiguous.is_empty());
        assert_eq!(inst.devices.changed.len(), 2);

        let key_a = device_key(left.devices.get(device_a).unwrap());
        let key_b = device_key(left.devices.get(device_b).unwrap());

        let change_a = inst
            .devices
            .changed
            .iter()
            .find(|c| c.key == key_a)
            .expect("device_a present in devices.changed");
        assert_eq!(change_a.changed_fields, vec!["description"]);
        assert_eq!(change_a.com_objects.changed.len(), 1);
        assert_eq!(change_a.com_objects.changed[0].changed_fields, vec!["dpt"]);
        assert!(change_a.com_objects.added.is_empty());
        assert!(change_a.com_objects.removed.is_empty());
        assert!(change_a.com_objects.ambiguous.is_empty());
        assert_eq!(change_a.parameters.changed.len(), 1);
        assert_eq!(change_a.parameters.changed[0].changed_fields, vec!["raw"]);
        assert!(change_a.parameters.added.is_empty());
        assert!(change_a.parameters.removed.is_empty());
        assert!(change_a.parameters.ambiguous.is_empty());

        let change_b = inst
            .devices
            .changed
            .iter()
            .find(|c| c.key == key_b)
            .expect("device_b present in devices.changed");
        assert_eq!(change_b.changed_fields, vec!["product_ref"]);
        assert_table_empty(&change_b.com_objects);
        assert_table_empty(&change_b.parameters);
    }
}
