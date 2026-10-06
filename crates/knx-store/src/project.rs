//! Orchestrates every entity module into one `save_project`/`load_project`
//! pair — the only two functions outside this crate that see the whole
//! `knx_core::Project` graph at once (design doc:
//! docs/superpowers/specs/2026-09-03-knx-entity-persistence-design.md).

use std::collections::BTreeSet;

use rusqlite::{params, Connection, Transaction, TransactionBehavior};

use knx_core::address::GroupAddressStyle;
use knx_core::ids::{ComObjectInstanceId, DeviceId};
use knx_core::installation::Installation;
use knx_core::project::{IdAllocators, Project, ProjectInfo};
use knx_core::string_table::Language;

use crate::building::{load_buildings, upsert_building_part};
use crate::devices::{
    load_all_com_objects, load_all_device_ids, load_all_program_defaults, load_device,
    set_device_line, upsert_com_object_instance, upsert_com_object_program_defaults, upsert_device,
    upsert_group_links,
};
use crate::group::{
    load_group_addresses, load_group_ranges, upsert_group_address, upsert_group_range,
};
use crate::module_instance::{load_module_instances_for_installation, upsert_module_instance};
use crate::parameter::{load_parameters_for_installation, upsert_parameter_instance};
use crate::strings::{load_string_table, upsert_string_table};
use crate::topology::{
    completion_from_str, completion_to_str, load_installation_rows, load_topology, upsert_area,
    upsert_installation_row, upsert_line,
};
use crate::StoreError;

fn style_to_str(s: GroupAddressStyle) -> &'static str {
    match s {
        GroupAddressStyle::Free => "Free",
        GroupAddressStyle::TwoLevel => "TwoLevel",
        GroupAddressStyle::ThreeLevel => "ThreeLevel",
    }
}

/// Refuses an unrecognized string rather than falling back to
/// `ThreeLevel` — a persisted style that cannot be read back is data loss
/// (KNOWN_LIMITATIONS.md §84), and `POST /api/project/new` already refuses
/// an unknown style with a `400` rather than silently picking one, so
/// `load_project` refusing too is the same rule applied at the other end
/// of the round trip, not a new one.
fn style_from_str(s: &str) -> Result<GroupAddressStyle, StoreError> {
    match s {
        "Free" => Ok(GroupAddressStyle::Free),
        "TwoLevel" => Ok(GroupAddressStyle::TwoLevel),
        "ThreeLevel" => Ok(GroupAddressStyle::ThreeLevel),
        other => Err(StoreError::UnknownGroupAddressStyle(other.to_string())),
    }
}

/// Overwrites `project_info.group_address_style` in place — the one-column
/// counterpart to `save_project`'s full rewrite, for `command_sync`'s
/// `Command::SetGroupAddressStyle` arm, which has no other field to touch.
pub fn set_group_address_style(
    conn: &Connection,
    style: GroupAddressStyle,
) -> Result<(), StoreError> {
    conn.execute(
        "UPDATE project_info SET group_address_style = ?1 WHERE id = 0",
        params![style_to_str(style)],
    )?;
    Ok(())
}

const DELETE_ALL_TABLES: &[&str] = &[
    // Child-to-parent order — matches the reverse of the insert order below.
    "com_object_program_default",
    "com_object_override",
    "group_link",
    "com_object_instance",
    "binary_data_ref",
    "building_part_device",
    "parameter_instance",
    "module_instance_argument",
    "module_instance",
    "group_address",
    "group_range",
    "device",
    "building_part",
    "line",
    "area",
    "installation",
    "string_table_entry",
    "id_allocators",
    "project_info",
];

pub fn save_project(conn: &Connection, project: &Project) -> Result<(), StoreError> {
    let tx = conn.unchecked_transaction()?;
    save_project_transaction(tx, project)
}

/// Saves `project` together with its opaque passthrough entries and its
/// manufacturer manifest in **one** transaction (AR18 review M2). Saving
/// them one by one committed three times, so a crash in between left the
/// new project beside the previous file's opaque evidence. Either all three
/// tables change, or none does.
pub fn save_project_with_passthrough(
    conn: &Connection,
    project: &Project,
    opaque: &[crate::StoredOpaqueEntry],
    manufacturer_refs: &[crate::ManufacturerRef],
) -> Result<(), StoreError> {
    let tx = conn.unchecked_transaction()?;
    write_project(&tx, project)?;
    crate::opaque::write_opaque(&tx, opaque)?;
    crate::manifest::write_manufacturer_refs(&tx, manufacturer_refs)?;
    tx.commit()?;
    Ok(())
}

/// Saves `replacement` only if the persisted semantic project still equals
/// `expected`. `BEGIN IMMEDIATE` obtains SQLite's write lock before the
/// comparison and holds it through commit, closing the check/write race that
/// a caller-side reload would leave open.
pub fn save_project_if_unchanged(
    conn: &Connection,
    expected: &Project,
    replacement: &Project,
) -> Result<(), StoreError> {
    let tx = Transaction::new_unchecked(conn, TransactionBehavior::Immediate)?;
    // Repaired, like `expected` (every caller gets it from `load_project`):
    // the repair is deterministic, so comparing unrepaired stored counters
    // with repaired expected ones would refuse every save of a file whose
    // counters were stale — a false concurrency conflict (ADR-0039 D7).
    let current = load_project(&tx)?;
    if &current != expected {
        return Err(StoreError::ConcurrentModification);
    }
    save_project_transaction(tx, replacement)
}

fn save_project_transaction(tx: Transaction<'_>, project: &Project) -> Result<(), StoreError> {
    write_project(&tx, project)?;
    tx.commit()?;
    Ok(())
}

/// Writes `project` inside a transaction the caller owns and commits.
fn write_project(tx: &Transaction<'_>, project: &Project) -> Result<(), StoreError> {
    crate::representable::check_representable(project)?;
    // Defer every foreign-key check to `COMMIT`, for two reasons that both
    // come from `building_part`/`group_range` self-referencing via
    // `parent_id`:
    //
    //  * a bulk `DELETE FROM` on a self-referencing table would otherwise
    //    risk the constraint being checked against a row this same
    //    statement has not deleted yet, and
    //  * `Installation::buildings`/`::group_ranges` are flat `Vec`s in no
    //    guaranteed parent-before-child order — `knx-etsproj`'s importer
    //    builds them in *post*-order (every child precedes its parent), so
    //    inserting them in list order writes rows whose `parent_id` names a
    //    row that does not exist yet.
    //
    // Deferred does not mean disabled: SQLite still verifies every
    // constraint at `COMMIT`, so a genuinely dangling reference is still an
    // error — just a commit-time one. The pragma is scoped to this
    // transaction, SQLite resetting it to OFF as soon as the transaction
    // ends, so no other user of this connection is affected.
    tx.execute_batch("PRAGMA defer_foreign_keys = ON")?;

    for table in DELETE_ALL_TABLES {
        tx.execute(&format!("DELETE FROM {table}"), [])?;
    }

    tx.execute(
        "INSERT INTO project_info
             (id, project_id, name, project_number, group_address_style, completion,
              last_modified, project_start, default_language, ets_schema_version,
              unlifted_group_address_dpt_declarations)
         VALUES (0, ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        params![
            project.info.project_id,
            project.info.name,
            project.info.project_number,
            style_to_str(project.info.group_address_style),
            completion_to_str(project.info.completion),
            project.info.last_modified.map(|d| d.to_rfc3339()),
            project.info.project_start.map(|d| d.to_rfc3339()),
            project.strings.default_language().0,
            project.info.ets_schema_version,
            project.info.unlifted_group_address_dpt_declarations,
        ],
    )?;

    tx.execute(
        "INSERT INTO id_allocators
             (id, device, area, line, com_object_instance, group_range, group_address,
              building_part, parameter_instance, module_instance)
         VALUES (0, ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        params![
            project.ids.peek_device(),
            project.ids.peek_area(),
            project.ids.peek_line(),
            project.ids.peek_com_object_instance(),
            project.ids.peek_group_range(),
            project.ids.peek_group_address(),
            project.ids.peek_building_part(),
            project.ids.peek_parameter_instance(),
            project.ids.peek_module_instance(),
        ],
    )?;

    upsert_string_table(&tx, &project.strings)?;

    // `Devices` is an independent map: a `DeviceInstance` can live there
    // without any `Line::devices`/`Topology::unassigned` list naming it, and
    // a `ComObjectInstance` without its device's `com_objects` naming it.
    // The two walks below reach only what the topology names, so anything
    // they miss would vanish on save. Both sets are collected as the walks
    // run and checked before `COMMIT`.
    let mut written_devices: BTreeSet<DeviceId> = BTreeSet::new();

    for installation in &project.installations {
        upsert_installation_row(&tx, installation)?;

        for (i, area) in installation.topology.areas.iter().enumerate() {
            upsert_area(&tx, installation.id, i as i64, area)?;
            for (j, line_id) in area.lines.iter().enumerate() {
                let line = installation
                    .topology
                    .line(*line_id)
                    .expect("Area::lines only ever names lines that exist in this Topology");
                upsert_line(&tx, area.id, j as i64, line)?;
            }
        }

        // One pass per device placement — line-assigned devices, then
        // unassigned ones — writing the device row and its topology
        // placement together instead of in three separate passes over the
        // same ids.
        for line in &installation.topology.lines {
            for (i, device_id) in line.devices.iter().enumerate() {
                let device = project
                    .devices
                    .get(*device_id)
                    .expect("Line::devices only ever names devices that exist in Devices");
                upsert_device(&tx, installation.id, i as i64, device)?;
                set_device_line(&tx, *device_id, Some(line.id), i as i64)?;
                written_devices.insert(*device_id);
            }
        }
        for (i, device_id) in installation.topology.unassigned.iter().enumerate() {
            let device = project
                .devices
                .get(*device_id)
                .expect("Topology::unassigned only ever names devices that exist in Devices");
            upsert_device(&tx, installation.id, i as i64, device)?;
            set_device_line(&tx, *device_id, None, i as i64)?;
            written_devices.insert(*device_id);
        }

        for (i, flat_position, part) in flatten_buildings(&installation.buildings) {
            upsert_building_part(&tx, installation.id, i, flat_position, part)?;
        }

        for (i, flat_position, range) in flatten_ranges(&installation.group_ranges) {
            upsert_group_range(&tx, installation.id, i, flat_position, range)?;
        }

        for (i, entry) in installation.group_addresses.iter().enumerate() {
            upsert_group_address(&tx, installation.id, i as i64, entry)?;
        }

        for (i, p) in installation.parameters.iter().enumerate() {
            upsert_parameter_instance(&tx, i as i64, p)?;
        }
    }

    let unreachable_devices: Vec<DeviceId> = project
        .devices
        .iter()
        .map(|d| d.id)
        .filter(|id| !written_devices.contains(id))
        .collect();
    if !unreachable_devices.is_empty() {
        return Err(StoreError::UnreachableDevices(unreachable_devices));
    }

    // Com-object instances + group links: one pass over every device's
    // `com_objects`, independent of which installation the device belongs
    // to — `com_object_instance` has no `installation_id` column of its
    // own, only `device_id`, so this does not need to be nested inside the
    // installation loop above.
    let mut written_com_objects: BTreeSet<ComObjectInstanceId> = BTreeSet::new();
    for device in project.devices.iter() {
        for (j, com_id) in device.com_objects.iter().enumerate() {
            let com = project
                .devices
                .com_object(*com_id)
                .expect("DeviceInstance::com_objects only ever names existing com objects");
            upsert_com_object_instance(&tx, device.id, j as i64, com)?;
            upsert_group_links(&tx, com.id, &com.links)?;
            if let Some(defaults) = project.devices.program_defaults(com.id) {
                upsert_com_object_program_defaults(&tx, com.id, defaults)?;
            }
            written_com_objects.insert(com.id);
        }
    }

    let unreachable_com_objects: Vec<ComObjectInstanceId> = project
        .devices
        .com_objects()
        .map(|c| c.id)
        .filter(|id| !written_com_objects.contains(id))
        .collect();
    if !unreachable_com_objects.is_empty() {
        return Err(StoreError::UnreachableComObjects(unreachable_com_objects));
    }

    // Module instances: `Devices.module_instances` is a flat, global map
    // keyed by id, exactly like `com_objects` — every `ModuleInstance` is
    // reached directly by `ModuleInstance::device`, not via an owning
    // `Installation`/`DeviceInstance` list, so this mirrors the
    // com-object-instance loop above (one pass per device, `position` reset
    // per device), not the per-installation `parameters` loop.
    for device in project.devices.iter() {
        for (position, module_instance) in project
            .devices
            .module_instances()
            .filter(|m| m.device == device.id)
            .enumerate()
        {
            upsert_module_instance(&tx, position as i64, module_instance)?;
        }
    }

    Ok(())
}

/// `Installation::buildings`/`::group_ranges` are flat `Vec`s where each
/// element also carries `position` (sibling order under its own
/// `parent`/`parent_id`, used to rebuild `children`) — see the design
/// doc's "owned-list vs flat-list order" note.
fn flatten_buildings(
    buildings: &[knx_core::building::BuildingPart],
) -> Vec<(i64, i64, &knx_core::building::BuildingPart)> {
    flatten_hierarchy(
        buildings,
        |p| p.id.0,
        |p| p.parent.map(|x| x.0),
        |parent, child| parent.children.iter().position(|c| c.0 == child),
    )
}

fn flatten_ranges(
    ranges: &[knx_core::group::GroupRange],
) -> Vec<(i64, i64, &knx_core::group::GroupRange)> {
    flatten_hierarchy(
        ranges,
        |r| r.id.0,
        |r| r.parent.map(|x| x.0),
        |parent, child| parent.children.iter().position(|c| c.0 == child),
    )
}

/// `(position, flat_position, item)` for each element of a flat hierarchy
/// list.
///
/// `flat_position` is the element's index in the flat `Vec`. `position` is
/// its index inside its parent's own `children` `Vec` — the field that
/// actually defines sibling order — found via `sibling_index`, not
/// re-derived from where the element happens to sit in the flat list. The
/// two can disagree: `children` and the flat list are independent `pub`
/// fields, and reconstructing `children` from flat-list order on reload
/// would silently reorder it.
///
/// Two cases have no `children` list to read an order out of, and both fall
/// back to `flat_position`, which is always defined and always
/// deterministic:
///
///  * a root (`parent` is `None`) — nothing contains it. Nothing queries
///    root `position` as a group either: `load_buildings`/`load_group_ranges`
///    only ever read `position` through
///    `WHERE parent_id = ?1 ORDER BY position`, never for `parent_id IS NULL`.
///  * an element whose named parent is missing from the list, or whose
///    parent's `children` does not name it back — inconsistent input, where
///    any order is a guess; a deterministic one beats a panic.
fn flatten_hierarchy<T>(
    items: &[T],
    id: impl Fn(&T) -> u32,
    parent: impl Fn(&T) -> Option<u32>,
    sibling_index: impl Fn(&T, u32) -> Option<usize>,
) -> Vec<(i64, i64, &T)> {
    let by_id: std::collections::HashMap<u32, &T> =
        items.iter().map(|item| (id(item), item)).collect();
    items
        .iter()
        .enumerate()
        .map(|(flat, item)| {
            let position = parent(item)
                .and_then(|p| by_id.get(&p))
                .and_then(|p| sibling_index(p, id(item)))
                .map_or(flat as i64, |i| i as i64);
            (position, flat as i64, item)
        })
        .collect()
}

/// What `load_project_reporting` changed about the stored id allocator: a
/// counter below the largest id of its kind actually stored was raised
/// (ADR-0039 Decision 7). Only counters are touched; no user content.
#[derive(Debug, Clone, PartialEq)]
pub struct AllocatorRepair {
    pub stored: IdAllocators,
    pub repaired: IdAllocators,
}

/// `load_project`, plus a report of any allocator repair it had to make.
/// Callers that can surface diagnostics (the server's session log, the CLI)
/// should use this form so a repair is never silent.
pub fn load_project_reporting(
    conn: &Connection,
) -> Result<(Project, Option<AllocatorRepair>), StoreError> {
    let mut project = load_project_unrepaired(conn)?;
    let stored = project.ids.clone();
    let repair = if project.ids.raise_to(&highest_ids_in_use(&project)) {
        Some(AllocatorRepair {
            stored,
            repaired: project.ids.clone(),
        })
    } else {
        None
    };
    Ok((project, repair))
}

/// The largest id of every kind that `project` actually holds, as an
/// allocator: raising the stored counters to this makes the next allocation
/// of each kind strictly above every id already in use.
fn highest_ids_in_use(project: &Project) -> IdAllocators {
    fn max<I: Iterator<Item = u32>>(ids: I) -> u32 {
        ids.max().unwrap_or(0)
    }
    let installations = &project.installations;
    IdAllocators::from_counts(
        max(project.devices.iter().map(|d| d.id.0)),
        max(installations
            .iter()
            .flat_map(|i| &i.topology.areas)
            .map(|a| a.id.0)),
        max(installations
            .iter()
            .flat_map(|i| &i.topology.lines)
            .map(|l| l.id.0)),
        max(project.devices.com_objects().map(|c| c.id.0)),
        max(installations
            .iter()
            .flat_map(|i| &i.group_ranges)
            .map(|r| r.id.0)),
        max(installations
            .iter()
            .flat_map(|i| &i.group_addresses)
            .map(|g| g.id.0)),
        max(installations
            .iter()
            .flat_map(|i| &i.buildings)
            .map(|b| b.id.0)),
        max(installations
            .iter()
            .flat_map(|i| &i.parameters)
            .map(|p| p.id.0)),
        max(project.devices.module_instances().map(|m| m.id.0)),
    )
}

/// Raises `project`'s counters to cover every id it holds — what a
/// consistent in-memory project (one built by the importer or through
/// commands) already satisfies. For tests that hand-build fixtures.
#[cfg(test)]
pub(crate) fn cover_ids_in_use(project: &mut Project) {
    let highest = highest_ids_in_use(project);
    project.ids.raise_to(&highest);
}

/// Loads the saved project. A stored id counter below an id actually in use
/// is raised (ADR-0039 Decision 7) — use `load_project_reporting` to learn
/// whether that happened.
pub fn load_project(conn: &Connection) -> Result<Project, StoreError> {
    load_project_reporting(conn).map(|(project, _)| project)
}

fn load_project_unrepaired(conn: &Connection) -> Result<Project, StoreError> {
    let (
        project_id,
        name,
        project_number,
        style,
        completion,
        last_modified,
        project_start,
        default_language,
        ets_schema_version,
        unlifted_group_address_dpt_declarations,
    ) = conn
        .query_row(
            "SELECT project_id, name, project_number, group_address_style, completion,
                    last_modified, project_start, default_language, ets_schema_version,
                    unlifted_group_address_dpt_declarations
             FROM project_info WHERE id = 0",
            [],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, Option<String>>(5)?,
                    row.get::<_, Option<String>>(6)?,
                    row.get::<_, String>(7)?,
                    row.get::<_, u32>(8)?,
                    row.get::<_, u32>(9)?,
                ))
            },
        )
        .optional_not_saved()?;

    let strings = load_string_table(conn, Language(default_language))?;

    let ids = conn.query_row(
        "SELECT device, area, line, com_object_instance, group_range, group_address,
                building_part, parameter_instance, module_instance
         FROM id_allocators WHERE id = 0",
        [],
        |row| {
            Ok(IdAllocators::from_counts(
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
                row.get(5)?,
                row.get(6)?,
                row.get(7)?,
                row.get(8)?,
            ))
        },
    )?;

    let mut devices = knx_core::devices::Devices::new();
    let mut com_objects_by_device = load_all_com_objects(conn)?;
    let mut program_defaults = load_all_program_defaults(conn)?;
    for device_id in load_all_device_ids(conn)? {
        let mut device = load_device(conn, device_id)?;
        let com_objects = com_objects_by_device.remove(&device_id).unwrap_or_default();
        device.com_objects = com_objects.iter().map(|com| com.id).collect();
        for com in com_objects {
            if let Some(defaults) = program_defaults.remove(&com.id) {
                devices.set_program_defaults(com.id, defaults);
            }
            devices.insert_com_object(com);
        }
        devices.insert(device);
    }

    let mut installations = Vec::new();
    for row in load_installation_rows(conn)? {
        let topology = load_topology(conn, row.id)?;
        let buildings = load_buildings(conn, row.id)?;
        let group_ranges = load_group_ranges(conn, row.id)?;
        let group_addresses = load_group_addresses(conn, row.id)?;
        let parameters = load_parameters_for_installation(conn, row.id)?;
        for module_instance in load_module_instances_for_installation(conn, row.id)? {
            devices.insert_module_instance(module_instance);
        }
        installations.push(Installation {
            id: row.id,
            name: row.name,
            default_line: row.default_line,
            multicast_address: row.multicast_address,
            completion: row.completion,
            topology,
            buildings,
            group_ranges,
            group_addresses,
            parameters,
        });
    }

    Ok(Project {
        // Not stored, and deliberately so: the migration chain has already
        // brought this file to `CURRENT_SCHEMA_VERSION` before `load_project`
        // can run, so an older file's version is a fact about the file
        // before migration, not about the `Project` being reconstructed here.
        schema_version: knx_core::project::CURRENT_SCHEMA_VERSION,
        strings,
        info: ProjectInfo {
            project_id,
            name,
            project_number,
            group_address_style: style_from_str(&style)?,
            completion: completion_from_str(&completion),
            last_modified: last_modified.map(|s| {
                chrono::DateTime::parse_from_rfc3339(&s)
                    .expect("stored timestamp is always valid RFC3339")
                    .with_timezone(&chrono::Utc)
            }),
            project_start: project_start.map(|s| {
                chrono::DateTime::parse_from_rfc3339(&s)
                    .expect("stored timestamp is always valid RFC3339")
                    .with_timezone(&chrono::Utc)
            }),
            ets_schema_version,
            unlifted_group_address_dpt_declarations,
        },
        installations,
        devices,
        ids,
    })
}

/// Turns `query_row`'s `QueryReturnedNoRows` specifically into
/// `StoreError::NotSaved` — every other `rusqlite::Error` still becomes
/// `StoreError::Sqlite` via the existing `From` impl.
trait OptionalNotSaved<T> {
    fn optional_not_saved(self) -> Result<T, StoreError>;
}

impl<T> OptionalNotSaved<T> for Result<T, rusqlite::Error> {
    fn optional_not_saved(self) -> Result<T, StoreError> {
        match self {
            Err(rusqlite::Error::QueryReturnedNoRows) => Err(StoreError::NotSaved),
            other => other.map_err(StoreError::from),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::open_and_migrate_in_memory;
    use knx_core::address::GroupAddress;
    use knx_core::building::{BuildingPart, BuildingPartType};
    use knx_core::commissioning::{CommissioningState, CompletionStatus};
    use knx_core::device::DeviceInstance;
    use knx_core::group::{GroupAddressEntry, GroupRange};
    use knx_core::ids::{
        AreaId, BuildingPartId, DeviceId, GroupAddressId, GroupRangeId, InstallationId, LineId,
        SourceRef,
    };
    use knx_core::topology::{Area, Line, Topology};

    fn source() -> SourceRef {
        SourceRef {
            path: "t".into(),
            ets_id: "t".into(),
        }
    }

    #[test]
    fn building_documented_kinds_survive_native_load_and_resave() {
        let conn = open_and_migrate_in_memory().unwrap();
        let mut root = part(1, None, BuildingPartType::Building);
        root.children = (2..=6).map(BuildingPartId).collect();
        let mut parts = vec![root];
        for (index, token) in ["Stairway", "RoomPart", "Area", "Ground", "Segment"]
            .iter()
            .enumerate()
        {
            parts.push(part(
                index as u32 + 2,
                Some(1),
                knx_etsproj::values::parse_building_part_type(token).unwrap(),
            ));
        }
        let project = project_with_hierarchy(parts, vec![], vec![]);
        save_project(&conn, &project).unwrap();
        let loaded = load_project(&conn).unwrap();
        assert_eq!(loaded, project);
        let kinds: Vec<String> = conn
            .prepare("SELECT kind FROM building_part ORDER BY id")
            .unwrap()
            .query_map([], |row| row.get(0))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();
        assert_eq!(
            kinds,
            ["Building", "Stairway", "RoomPart", "Area", "Ground", "Segment"]
        );
        save_project(&conn, &loaded).unwrap();
        assert_eq!(load_project(&conn).unwrap(), project);
    }

    /// ADR-0038: a site above several buildings is a `Ground` root space.
    /// It must survive native save/load/re-save with both buildings under
    /// it, both devices on the one shared line, and each device referenced
    /// by exactly one building — the site adds no ownership of its own.
    #[test]
    fn a_ground_site_over_two_buildings_on_one_line_round_trips() {
        let conn = open_and_migrate_in_memory().unwrap();
        let mut site = part(1, None, BuildingPartType::Ground);
        site.children = vec![BuildingPartId(2), BuildingPartId(3)];
        let mut north = part(2, Some(1), BuildingPartType::Building);
        north.devices = vec![DeviceId(1)];
        let mut south = part(3, Some(1), BuildingPartType::Building);
        south.devices = vec![DeviceId(2)];
        let mut project = project_with_hierarchy(vec![site, north, south], vec![], vec![]);
        let installation = &mut project.installations[0];
        installation.topology.lines = vec![Line {
            id: LineId(1),
            source: source(),
            name: "Shared line".into(),
            address: 1,
            medium_ref: "TP".into(),
            domain_address: None,
            domain_address_is_checked: None,
            ip_routing_multicast_address: None,
            multicast_ttl: None,
            completion: CompletionStatus::FinishedDesign,
            devices: vec![DeviceId(1), DeviceId(2)],
        }];
        installation.topology.areas = vec![Area {
            id: AreaId(1),
            source: source(),
            name: "A1".into(),
            address: 1,
            completion: CompletionStatus::FinishedDesign,
            lines: vec![LineId(1)],
        }];
        project.devices.insert(device(1));
        project.devices.insert(device(2));

        cover_ids_in_use(&mut project);
        save_project(&conn, &project).unwrap();
        let loaded = load_project(&conn).unwrap();
        assert_eq!(loaded, project);
        let root_kinds: Vec<String> = conn
            .prepare("SELECT kind FROM building_part WHERE parent_id IS NULL")
            .unwrap()
            .query_map([], |row| row.get(0))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();
        assert_eq!(root_kinds, ["Ground"]);
        let device_refs: i64 = conn
            .query_row("SELECT count(*) FROM building_part_device", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(device_refs, 2);
        save_project(&conn, &loaded).unwrap();
        assert_eq!(load_project(&conn).unwrap(), project);
    }

    #[test]
    fn building_unknown_persisted_kind_is_refused() {
        let conn = open_and_migrate_in_memory().unwrap();
        let project = project_with_hierarchy(
            vec![part(1, None, BuildingPartType::Building)],
            vec![],
            vec![],
        );
        save_project(&conn, &project).unwrap();
        conn.execute("UPDATE building_part SET kind = 'FutureSpace'", [])
            .unwrap();
        let error = load_project(&conn).expect_err("unknown stored kind must not become Building");
        assert!(error.to_string().contains("FutureSpace"));
        assert!(
            matches!(error, StoreError::UnknownBuildingPartType(value) if value == "FutureSpace")
        );
    }

    #[test]
    fn an_empty_project_round_trips() {
        let conn = open_and_migrate_in_memory().unwrap();
        let project = Project::new(Language("en".into()));
        save_project(&conn, &project).unwrap();
        let loaded = load_project(&conn).unwrap();
        assert_eq!(loaded, project);
    }

    #[test]
    fn compare_and_save_rejects_a_project_changed_after_preview() {
        let conn = open_and_migrate_in_memory().unwrap();
        let mut expected = Project::new(Language("en".into()));
        expected.info.name = "previewed".into();
        save_project(&conn, &expected).unwrap();

        let mut concurrent = expected.clone();
        concurrent.info.name = "concurrent edit".into();
        save_project(&conn, &concurrent).unwrap();
        let mut replacement = expected.clone();
        replacement.info.name = "csv edit".into();

        let error = save_project_if_unchanged(&conn, &expected, &replacement)
            .expect_err("stale preview must not overwrite the newer project");

        assert!(matches!(error, StoreError::ConcurrentModification));
        assert_eq!(load_project(&conn).unwrap(), concurrent);
    }

    #[test]
    fn compare_and_save_commits_when_the_preview_still_matches() {
        let conn = open_and_migrate_in_memory().unwrap();
        let mut expected = Project::new(Language("en".into()));
        expected.info.name = "previewed".into();
        save_project(&conn, &expected).unwrap();
        let mut replacement = expected.clone();
        replacement.info.name = "csv edit".into();

        save_project_if_unchanged(&conn, &expected, &replacement).unwrap();

        assert_eq!(load_project(&conn).unwrap(), replacement);
    }

    /// `Project::new` defaults to `ThreeLevel` (see `ProjectInfo::default`),
    /// so this exercises the other two styles specifically — the acceptance
    /// list's store round-trip test for `style_from_str`'s now-fallible
    /// signature actually reading back what `style_to_str` wrote.
    #[test]
    fn a_non_default_group_address_style_round_trips() {
        let conn = open_and_migrate_in_memory().unwrap();
        let mut project = Project::new(Language("en".into()));
        project.info.group_address_style = GroupAddressStyle::Free;
        save_project(&conn, &project).unwrap();
        let loaded = load_project(&conn).unwrap();
        assert_eq!(loaded.info.group_address_style, GroupAddressStyle::Free);

        project.info.group_address_style = GroupAddressStyle::TwoLevel;
        save_project(&conn, &project).unwrap();
        let loaded = load_project(&conn).unwrap();
        assert_eq!(loaded.info.group_address_style, GroupAddressStyle::TwoLevel);
    }

    /// ADR-0078: a group address's declaration and the project's count of
    /// unattributed declarations survive a native save and load, and the
    /// saved project reloads equal.
    #[test]
    fn group_address_declarations_and_the_unlifted_count_round_trip() {
        use knx_core::address::GroupAddress;
        use knx_core::dpt::DptRef;
        use knx_core::group::GroupAddressEntry;
        use knx_core::ids::{GroupAddressId, SourceRef};
        use knx_core::provenance::{Layer, Override, Resolved};
        let conn = open_and_migrate_in_memory().unwrap();
        let mut project = project_with_one_installation();
        project.info.unlifted_group_address_dpt_declarations = 2;
        for (id, declared_dpt) in [
            (
                1,
                Override::Value(Resolved {
                    value: DptRef {
                        main: 9,
                        sub: Some(1),
                    },
                    layer: Layer::Instance,
                }),
            ),
            (2, Override::Malformed("DPST-1-1 DPST-1-2".into())),
            (3, Override::Empty),
        ] {
            project.installations[0]
                .group_addresses
                .push(GroupAddressEntry {
                    id: GroupAddressId(id),
                    source: SourceRef {
                        path: "P-1/0.xml".into(),
                        ets_id: format!("GA-{id}"),
                    },
                    name: "ga".into(),
                    address: GroupAddress::from_raw(2048 + id as u16),
                    central: false,
                    unfiltered: false,
                    range: None,
                    declared_dpt,
                });
        }
        crate::project::cover_ids_in_use(&mut project);
        save_project(&conn, &project).unwrap();
        assert_eq!(load_project(&conn).unwrap(), project);
    }

    /// A hand-edited or third-party-written `project_info.group_address_style`
    /// that names none of the three known styles must be refused, not quietly
    /// read back as `ThreeLevel` (KNOWN_LIMITATIONS.md §84) — the counterpart
    /// to `POST /api/project/new`'s `400` on an unknown style at the other
    /// end of the same round trip.
    #[test]
    fn an_unrecognized_persisted_style_is_refused_not_defaulted() {
        let conn = open_and_migrate_in_memory().unwrap();
        let project = Project::new(Language("en".into()));
        save_project(&conn, &project).unwrap();
        conn.execute(
            "UPDATE project_info SET group_address_style = 'Sideways' WHERE id = 0",
            [],
        )
        .unwrap();
        match load_project(&conn) {
            Err(StoreError::UnknownGroupAddressStyle(style)) => assert_eq!(style, "Sideways"),
            other => panic!("expected UnknownGroupAddressStyle, got {other:?}"),
        }
    }

    /// `set_group_address_style` is `command_sync`'s one-column counterpart
    /// to `save_project`'s full rewrite — this checks it in isolation rather
    /// than only through a `Command`.
    #[test]
    fn set_group_address_style_overwrites_the_one_column() {
        let conn = open_and_migrate_in_memory().unwrap();
        let project = Project::new(Language("en".into()));
        save_project(&conn, &project).unwrap();

        set_group_address_style(&conn, GroupAddressStyle::TwoLevel).unwrap();

        let loaded = load_project(&conn).unwrap();
        assert_eq!(loaded.info.group_address_style, GroupAddressStyle::TwoLevel);
    }

    #[test]
    fn loading_a_never_saved_database_is_not_saved_not_an_empty_project() {
        let conn = open_and_migrate_in_memory().unwrap();
        assert!(matches!(load_project(&conn), Err(StoreError::NotSaved)));
    }

    fn device(id: u32) -> DeviceInstance {
        DeviceInstance {
            id: DeviceId(id),
            source: source(),
            name: format!("D{id}"),
            description: None,
            address: None,
            product_ref: "P".into(),
            program_ref: "H".into(),
            commissioning: CommissioningState::default(),
            visibility_calculated: true,
            com_objects: vec![],
            binary_data: vec![],
        }
    }

    /// ADR-0039 Decision 7: a file saved with a counter below an id it
    /// stores (a pre-fix stale snapshot could write one) must load with the
    /// counter raised, or `IdInUse` would refuse every later create of that
    /// kind. The repair is reported, never silent.
    #[test]
    fn a_counter_below_a_stored_id_is_raised_on_load_and_reported() {
        let conn = open_and_migrate_in_memory().unwrap();
        let mut project = project_with_one_installation();
        project.installations[0].topology.unassigned = vec![DeviceId(4)];
        project.devices.insert(device(4));
        project.installations[0]
            .group_addresses
            .push(GroupAddressEntry {
                id: GroupAddressId(9),
                source: source(),
                name: "GA".into(),
                address: GroupAddress::from_raw(1),
                central: false,
                unfiltered: false,
                range: None,
                declared_dpt: Default::default(),
            });
        // Stored counters: device 2, group address 3 — both below the ids above.
        project.ids = IdAllocators::from_counts(2, 0, 0, 0, 0, 3, 0, 0, 0);
        save_project(&conn, &project).unwrap();

        let (loaded, repair) = load_project_reporting(&conn).unwrap();
        assert_eq!(loaded.ids.peek_device(), 4);
        assert_eq!(loaded.ids.peek_group_address(), 9);
        let repair = repair.expect("a raised counter must be reported");
        assert_eq!(repair.stored, project.ids);
        assert_eq!(repair.repaired, loaded.ids);
        assert!(load_project(&conn).unwrap().ids == loaded.ids);
    }

    /// The CLI's `ga-import` path: `expected` comes from `load_project`, so it
    /// carries the repaired counters. The concurrency check must compare like
    /// with like, or a stale-counter file could never be saved again.
    #[test]
    fn a_repaired_project_can_still_be_saved_if_unchanged() {
        let conn = open_and_migrate_in_memory().unwrap();
        let mut project = project_with_one_installation();
        project.installations[0].topology.unassigned = vec![DeviceId(4)];
        project.devices.insert(device(4));
        project.ids = IdAllocators::from_counts(1, 0, 0, 0, 0, 0, 0, 0, 0);
        save_project(&conn, &project).unwrap();

        let expected = load_project(&conn).unwrap();
        let mut replacement = expected.clone();
        replacement.installations[0].name = "Renamed".into();
        save_project_if_unchanged(&conn, &expected, &replacement).unwrap();
        assert_eq!(load_project(&conn).unwrap(), replacement);
    }

    #[test]
    fn counters_at_or_above_every_stored_id_load_unchanged_and_unreported() {
        let conn = open_and_migrate_in_memory().unwrap();
        let mut project = project_with_one_installation();
        project.installations[0].topology.unassigned = vec![DeviceId(4)];
        project.devices.insert(device(4));
        project.ids = IdAllocators::from_counts(7, 0, 0, 0, 0, 0, 0, 0, 0);
        save_project(&conn, &project).unwrap();

        let (loaded, repair) = load_project_reporting(&conn).unwrap();
        assert!(repair.is_none());
        assert_eq!(loaded, project);
    }

    fn project_with_one_installation() -> Project {
        let mut project = Project::new(Language("en".into()));
        project.installations.push(Installation {
            id: InstallationId(0),
            name: "I".into(),
            default_line: None,
            multicast_address: None,
            completion: CompletionStatus::FinishedDesign,
            topology: Topology {
                areas: vec![],
                lines: vec![],
                unassigned: vec![],
            },
            buildings: vec![],
            group_ranges: vec![],
            group_addresses: vec![],
            parameters: vec![],
        });
        project
    }

    /// `Devices` is an independent map, so a device can sit in it without
    /// any line or unassigned list naming it. `save_project` writes devices
    /// by walking the topology, so such a device would be silently dropped —
    /// it has to be refused instead (CLAUDE.md: never silently discard
    /// information).
    #[test]
    fn a_device_unreachable_from_the_topology_is_refused_not_dropped() {
        let conn = open_and_migrate_in_memory().unwrap();
        let mut project = project_with_one_installation();
        project.installations[0].topology.unassigned = vec![DeviceId(1)];
        project.devices.insert(device(1));
        project.devices.insert(device(2)); // named by nothing

        match save_project(&conn, &project) {
            Err(StoreError::UnreachableDevices(ids)) => assert_eq!(ids, vec![DeviceId(2)]),
            other => panic!("expected UnreachableDevices, got {other:?}"),
        }
        // Nothing was half-written either: the transaction never committed.
        assert!(matches!(load_project(&conn), Err(StoreError::NotSaved)));
    }

    /// The same one level down: a `ComObjectInstance` owned by `Devices` but
    /// named by no device's `com_objects` list.
    #[test]
    fn a_com_object_unreachable_from_its_device_is_refused_not_dropped() {
        use knx_core::device::ComObjectInstance;
        use knx_core::flags::ResolvedFlags;
        use knx_core::ids::ComObjectInstanceId;
        use knx_core::provenance::Override;

        let conn = open_and_migrate_in_memory().unwrap();
        let mut project = project_with_one_installation();
        project.installations[0].topology.unassigned = vec![DeviceId(1)];
        project.devices.insert(device(1));
        project.devices.insert_com_object(ComObjectInstance {
            id: ComObjectInstanceId(7),
            source: source(),
            device: DeviceId(1),
            number: 0,
            text: Override::Absent,
            description: Override::Absent,
            dpt: Override::Absent,
            flags: ResolvedFlags::none(),
            size: None,
            is_active: true,
            links: vec![],
            module_instance: None,
        });
        // …and deliberately never pushed onto device 1's `com_objects`.

        match save_project(&conn, &project) {
            Err(StoreError::UnreachableComObjects(ids)) => {
                assert_eq!(ids, vec![ComObjectInstanceId(7)])
            }
            other => panic!("expected UnreachableComObjects, got {other:?}"),
        }
        assert!(matches!(load_project(&conn), Err(StoreError::NotSaved)));
    }

    #[test]
    fn orphaned_com_object_graph_rows_are_ignored_during_project_load() {
        let conn = open_and_migrate_in_memory().unwrap();
        save_project(&conn, &Project::new(Language("en".into()))).unwrap();

        conn.execute_batch("PRAGMA foreign_keys = OFF").unwrap();
        conn.execute(
            "INSERT INTO com_object_instance
                 (id, device_id, position, source_path, source_ets_id, number,
                  size_kind, size_value, size_layer, is_active, module_instance_id)
             VALUES (77, 99, 0, 'orphan.xml', 'O-77', 0, 'bit', NULL, NULL, 1, NULL)",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO com_object_override
                 (com_object_instance_id, attr, state, value, text_kind, layer)
             VALUES (77, 'priority', 'value', 'low', NULL, 'Instance')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO group_link
                 (com_object_instance_id, group_address_id, direction, position)
             VALUES (77, 88, 'Send', 0)",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO com_object_override
                 (com_object_instance_id, attr, state, value, text_kind, layer)
             VALUES (78, 'priority', 'value', 'low', NULL, 'Instance')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO group_link
                 (com_object_instance_id, group_address_id, direction, position)
             VALUES (78, 88, 'Send', 0)",
            [],
        )
        .unwrap();

        let loaded = load_project(&conn).unwrap();
        assert_eq!(loaded.devices.iter().count(), 0);
        assert_eq!(loaded.devices.com_objects().count(), 0);
    }

    #[test]
    fn a_reachable_unknown_override_attribute_is_reported_by_project_load() {
        use knx_core::device::ComObjectInstance;
        use knx_core::flags::ResolvedFlags;
        use knx_core::provenance::Override;

        let conn = open_and_migrate_in_memory().unwrap();
        let mut project = project_with_one_installation();
        project.installations[0].topology.unassigned = vec![DeviceId(1)];
        project.devices.insert(device(1));
        project.devices.insert_com_object(ComObjectInstance {
            id: ComObjectInstanceId(1),
            source: source(),
            device: DeviceId(1),
            number: 0,
            text: Override::Absent,
            description: Override::Absent,
            dpt: Override::Absent,
            flags: ResolvedFlags::none(),
            size: None,
            is_active: true,
            links: vec![],
            module_instance: None,
        });
        project
            .devices
            .get_mut(DeviceId(1))
            .unwrap()
            .com_objects
            .push(ComObjectInstanceId(1));
        save_project(&conn, &project).unwrap();
        conn.execute(
            "INSERT INTO com_object_override
                 (com_object_instance_id, attr, state, value, text_kind, layer)
             VALUES (1, 'priority', 'value', 'low', NULL, 'Instance')",
            [],
        )
        .unwrap();

        match load_project(&conn) {
            Err(StoreError::UnknownOverrideAttr(attr)) => assert_eq!(attr, "priority"),
            other => panic!("expected UnknownOverrideAttr, got {other:?}"),
        }
    }

    #[test]
    fn a_project_with_one_device_and_an_unassigned_one_round_trips() {
        use knx_core::device::ComObjectInstance;
        use knx_core::flags::ResolvedFlags;
        use knx_core::provenance::Override;

        let conn = open_and_migrate_in_memory().unwrap();
        let mut project = Project::new(Language("de-DE".into()));
        let line = Line {
            id: LineId(1),
            source: source(),
            name: "HL".into(),
            address: 1,
            medium_ref: "TP".into(),
            domain_address: None,
            domain_address_is_checked: None,
            ip_routing_multicast_address: None,
            multicast_ttl: None,
            completion: CompletionStatus::FinishedDesign,
            devices: vec![DeviceId(1)],
        };
        let area = Area {
            id: AreaId(1),
            source: source(),
            name: "A1".into(),
            address: 1,
            completion: CompletionStatus::FinishedDesign,
            lines: vec![line.id],
        };
        let installation = Installation {
            id: InstallationId(0),
            name: "Haus".into(),
            default_line: Some(line.id),
            multicast_address: None,
            completion: CompletionStatus::FinishedDesign,
            topology: Topology {
                areas: vec![area],
                lines: vec![line],
                unassigned: vec![DeviceId(2)],
            },
            buildings: vec![],
            group_ranges: vec![],
            group_addresses: vec![],
            parameters: vec![],
        };
        project.installations.push(installation);
        for id in [1, 2] {
            project.devices.insert(DeviceInstance {
                id: DeviceId(id),
                source: source(),
                name: format!("D{id}"),
                description: None,
                address: None,
                product_ref: "P".into(),
                program_ref: "H".into(),
                commissioning: CommissioningState::default(),
                visibility_calculated: true,
                com_objects: vec![],
                binary_data: vec![],
            });
        }
        for (id, number) in [(9, 0), (3, 1)] {
            let id = ComObjectInstanceId(id);
            project.devices.insert_com_object(ComObjectInstance {
                id,
                source: source(),
                device: DeviceId(1),
                number,
                text: Override::Absent,
                description: Override::Absent,
                dpt: Override::Absent,
                flags: ResolvedFlags::none(),
                size: None,
                is_active: true,
                links: vec![],
                module_instance: None,
            });
            project
                .devices
                .get_mut(DeviceId(1))
                .unwrap()
                .com_objects
                .push(id);
        }
        // One of the two carries a lifted program default too (ADR-0012 gap
        // 2, ADR-0027) — `assert_eq!(loaded, project)` below already
        // compares all of `Devices`, program defaults included, so this is
        // the round trip proof for the new table, not a separate test.
        project.devices.set_program_defaults(
            ComObjectInstanceId(9),
            knx_core::device::ProgramDefaults {
                text: None,
                description: None,
                dpt: Some(knx_core::Resolved {
                    value: knx_core::DptRef {
                        main: 1,
                        sub: Some(1),
                    },
                    layer: knx_core::Layer::Program,
                }),
            },
        );

        cover_ids_in_use(&mut project);
        save_project(&conn, &project).unwrap();
        let loaded = load_project(&conn).unwrap();
        let loaded_device = loaded.devices.get(DeviceId(1)).unwrap();
        assert_eq!(
            loaded_device.com_objects,
            vec![ComObjectInstanceId(9), ComObjectInstanceId(3)]
        );
        assert!(loaded
            .devices
            .program_defaults(ComObjectInstanceId(9))
            .is_some());
        assert_eq!(loaded, project);
    }

    /// A two-level building hierarchy (`House` -> `Floor 1`) and a two-level
    /// group-range nesting (`Licht` -> `Licht - An/Aus`) plus one group
    /// address inside the inner range. Returned as separate values so each
    /// caller can put them into `Installation::buildings`/`::group_ranges`
    /// in its own order — the flat `Vec`'s order is exactly what the
    /// pre-order/post-order tests below differ on.
    #[allow(clippy::type_complexity)]
    fn hierarchy_nodes() -> (
        (BuildingPart, BuildingPart),
        (GroupRange, GroupRange),
        GroupAddressEntry,
    ) {
        let building = BuildingPart {
            id: BuildingPartId(1),
            source: source(),
            name: "House".into(),
            number: None,
            kind: BuildingPartType::Building,
            default_line: None,
            completion: CompletionStatus::FinishedDesign,
            children: vec![BuildingPartId(2)],
            devices: vec![],
            parent: None,
        };
        let floor = BuildingPart {
            id: BuildingPartId(2),
            source: source(),
            name: "Floor 1".into(),
            number: Some("1".into()),
            kind: BuildingPartType::Floor,
            default_line: None,
            completion: CompletionStatus::FinishedDesign,
            children: vec![],
            devices: vec![],
            parent: Some(BuildingPartId(1)),
        };
        let main_range = GroupRange {
            id: GroupRangeId(1),
            source: source(),
            name: "Licht".into(),
            start: GroupAddress::from_raw(0),
            end: GroupAddress::from_raw(2047),
            parent: None,
            children: vec![GroupRangeId(2)],
        };
        let mid_range = GroupRange {
            id: GroupRangeId(2),
            source: source(),
            name: "Licht - An/Aus".into(),
            start: GroupAddress::from_raw(0),
            end: GroupAddress::from_raw(255),
            parent: Some(GroupRangeId(1)),
            children: vec![],
        };
        let ga = GroupAddressEntry {
            id: GroupAddressId(1),
            source: source(),
            name: "EG Licht".into(),
            address: GroupAddress::from_raw(1),
            central: false,
            unfiltered: false,
            range: Some(GroupRangeId(2)),
            declared_dpt: Default::default(),
        };
        ((building, floor), (main_range, mid_range), ga)
    }

    fn project_with_hierarchy(
        buildings: Vec<BuildingPart>,
        group_ranges: Vec<GroupRange>,
        group_addresses: Vec<GroupAddressEntry>,
    ) -> Project {
        let mut project = Project::new(Language("en".into()));
        project.installations.push(Installation {
            id: InstallationId(0),
            name: "I".into(),
            default_line: None,
            multicast_address: None,
            completion: CompletionStatus::FinishedDesign,
            topology: Topology {
                areas: vec![],
                lines: vec![],
                unassigned: vec![],
            },
            buildings,
            group_ranges,
            group_addresses,
            parameters: vec![],
        });
        cover_ids_in_use(&mut project);
        project
    }

    #[test]
    fn a_project_with_nested_buildings_and_group_ranges_round_trips() {
        let conn = open_and_migrate_in_memory().unwrap();
        let ((building, floor), (main_range, mid_range), ga) = hierarchy_nodes();
        let project =
            project_with_hierarchy(vec![building, floor], vec![main_range, mid_range], vec![ga]);

        save_project(&conn, &project).unwrap();
        let loaded = load_project(&conn).unwrap();
        assert_eq!(loaded, project);

        // Re-save on top of existing self-referencing rows (`building_part`/
        // `group_range`'s `parent_id`) — the case a single-save round trip
        // above never exercises, and the one `PRAGMA foreign_keys = ON`
        // bulk-delete could violate if the two tables' `parent_id` columns
        // were still pointing at rows the same statement has not deleted yet.
        save_project(&conn, &project).unwrap();
        let loaded_again = load_project(&conn).unwrap();
        assert_eq!(loaded_again, project);
    }

    /// The order the real importer produces: `knx-etsproj`'s `map.rs`
    /// recurses into a node's children and pushes each of them onto the flat
    /// `Installation::buildings`/`::group_ranges` `Vec` *before* pushing the
    /// node itself, so every child precedes its parent. Writing a
    /// `building_part`/`group_range` row whose `parent_id` names a row that
    /// does not exist yet is a hard `FOREIGN KEY constraint failed` under
    /// `PRAGMA foreign_keys = ON` unless `save_project` defers foreign-key
    /// checking to `COMMIT` — which is exactly what this test pins down.
    #[test]
    fn a_post_order_hierarchy_round_trips_the_way_the_importer_builds_it() {
        let conn = open_and_migrate_in_memory().unwrap();
        let ((building, floor), (main_range, mid_range), ga) = hierarchy_nodes();
        let project = project_with_hierarchy(
            vec![floor, building],       // child before parent
            vec![mid_range, main_range], // child before parent
            vec![ga],
        );

        save_project(&conn, &project).unwrap();
        let loaded = load_project(&conn).unwrap();
        assert_eq!(loaded, project);

        // And again on top of the rows the first save left behind.
        save_project(&conn, &project).unwrap();
        assert_eq!(load_project(&conn).unwrap(), project);
    }

    /// Sibling order comes from the parent's own `children` field, not from
    /// the order the siblings happen to appear in the flat
    /// `Installation::buildings`/`::group_ranges` `Vec`. The two are
    /// independent `pub` fields; here they deliberately disagree (the flat
    /// list holds `[a, b]`, `children` says `[b, a]`, and the two are not
    /// even adjacent in the flat list), which counting flat-list occurrences
    /// per parent would silently "fix" by reordering `children` on reload.
    #[test]
    fn sibling_order_follows_the_children_field_not_the_flat_list_order() {
        let conn = open_and_migrate_in_memory().unwrap();

        let house = BuildingPart {
            children: vec![BuildingPartId(3), BuildingPartId(2)], // reversed
            ..part(1, None, BuildingPartType::Building)
        };
        let floor_a = part(2, Some(1), BuildingPartType::Floor);
        let floor_b = part(3, Some(1), BuildingPartType::Floor);
        let annex = part(4, None, BuildingPartType::Building); // separates the two

        let licht = GroupRange {
            children: vec![GroupRangeId(3), GroupRangeId(2)], // reversed
            ..range(1, None, 0, 2047)
        };
        let mid_a = range(2, Some(1), 0, 255);
        let mid_b = range(3, Some(1), 256, 511);
        let heizung = range(4, None, 2048, 4095); // separates the two

        let project = project_with_hierarchy(
            vec![floor_a, annex, floor_b, house],
            vec![mid_a, heizung, mid_b, licht],
            vec![],
        );

        save_project(&conn, &project).unwrap();
        let loaded = load_project(&conn).unwrap();

        let house_loaded = loaded.installations[0]
            .buildings
            .iter()
            .find(|p| p.id == BuildingPartId(1))
            .unwrap();
        assert_eq!(
            house_loaded.children,
            vec![BuildingPartId(3), BuildingPartId(2)],
            "children must come back in the order the field stated"
        );
        let licht_loaded = loaded.installations[0]
            .group_ranges
            .iter()
            .find(|r| r.id == GroupRangeId(1))
            .unwrap();
        assert_eq!(
            licht_loaded.children,
            vec![GroupRangeId(3), GroupRangeId(2)],
            "children must come back in the order the field stated"
        );
        assert_eq!(loaded, project);
    }

    fn part(id: u32, parent: Option<u32>, kind: BuildingPartType) -> BuildingPart {
        BuildingPart {
            id: BuildingPartId(id),
            source: source(),
            name: format!("Part {id}"),
            number: None,
            kind,
            default_line: None,
            completion: CompletionStatus::FinishedDesign,
            children: vec![],
            devices: vec![],
            parent: parent.map(BuildingPartId),
        }
    }

    fn range(id: u32, parent: Option<u32>, start: u16, end: u16) -> GroupRange {
        GroupRange {
            id: GroupRangeId(id),
            source: source(),
            name: format!("Range {id}"),
            start: GroupAddress::from_raw(start),
            end: GroupAddress::from_raw(end),
            parent: parent.map(GroupRangeId),
            children: vec![],
        }
    }

    /// Deferring foreign-key checks to `COMMIT` must not become "no foreign
    /// keys at all": a genuinely dangling reference is still rejected, just
    /// at commit time rather than at insert time.
    #[test]
    fn a_dangling_parent_reference_is_still_rejected_at_commit() {
        let conn = open_and_migrate_in_memory().unwrap();
        let ((_, mut floor), (main_range, mid_range), ga) = hierarchy_nodes();
        floor.parent = Some(BuildingPartId(99)); // no such building part
        let project = project_with_hierarchy(vec![floor], vec![main_range, mid_range], vec![ga]);

        let err = save_project(&conn, &project).unwrap_err();
        // ADR-0074: the representability check now names the problem before
        // any write; the deferred FK check is still pinned by
        // `tests/lossless_save.rs` (a dangling group-address range pointer).
        assert!(
            matches!(&err, StoreError::Unrepresentable(issues) if issues.iter().any(|issue|
            matches!(issue, crate::representable::RepresentationIssue::HierarchyMismatch {
                kind: crate::representable::EntityKind::BuildingPart, ..
            }))),
            "a dangling parent_id must still fail: {err}"
        );
    }

    #[test]
    fn allocator_state_survives_a_round_trip_and_next_id_does_not_collide() {
        let conn = open_and_migrate_in_memory().unwrap();
        let mut project = Project::new(Language("en".into()));
        let _ = project.ids.next_device_id().unwrap(); // DeviceId(1)
        let second = project.ids.next_device_id().unwrap(); // DeviceId(2)
        save_project(&conn, &project).unwrap();
        let mut loaded = load_project(&conn).unwrap();
        let next = loaded.ids.next_device_id().unwrap();
        assert_eq!(next, DeviceId(3));
        assert_ne!(next, second);
    }

    /// `ModuleInstance` lives on `Devices` as a flat, global map (no owning
    /// `Installation`/`DeviceInstance` list), unlike `ParameterInstance` — see
    /// this task's Controller correction. This exercises save->load for that
    /// storage shape end to end, not just `module_instance.rs`'s own
    /// unit-level round trip.
    #[test]
    fn module_instances_round_trip_through_save_and_load() {
        use knx_core::ids::ModuleInstanceId;
        use knx_core::module::ModuleInstance;

        let conn = open_and_migrate_in_memory().unwrap();
        let mut project = project_with_one_installation();
        project.installations[0].topology.unassigned = vec![DeviceId(1)];
        project.devices.insert(device(1));
        project.devices.insert_module_instance(ModuleInstance {
            id: ModuleInstanceId(1),
            device: DeviceId(1),
            source: source(),
            repeat_index: "6x1".into(),
            instance_ets_id: "t_MI-1".into(),
            arguments: vec![(source(), "1".into()), (source(), "0".into())],
        });

        cover_ids_in_use(&mut project);
        save_project(&conn, &project).unwrap();
        let loaded = load_project(&conn).unwrap();
        assert_eq!(loaded.devices.module_instances().count(), 1);
        assert_eq!(loaded, project);
    }
}
