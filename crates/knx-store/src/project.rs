//! Orchestrates every entity module into one `save_project`/`load_project`
//! pair — the only two functions outside this crate that see the whole
//! `knx_core::Project` graph at once (design doc:
//! docs/superpowers/specs/2026-09-03-knx-entity-persistence-design.md).

use std::collections::BTreeSet;

use rusqlite::{params, Connection};

use knx_core::address::GroupAddressStyle;
use knx_core::ids::{ComObjectInstanceId, DeviceId};
use knx_core::installation::Installation;
use knx_core::project::{IdAllocators, Project, ProjectInfo};
use knx_core::string_table::Language;

use crate::building::{load_buildings, upsert_building_part};
use crate::devices::{
    load_all_device_ids, load_com_object_ids_for_device, load_com_object_instance, load_device,
    load_group_links, set_device_line, upsert_com_object_instance, upsert_device,
    upsert_group_links,
};
use crate::group::{
    load_group_addresses, load_group_ranges, upsert_group_address, upsert_group_range,
};
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

fn style_from_str(s: &str) -> GroupAddressStyle {
    match s {
        "Free" => GroupAddressStyle::Free,
        "TwoLevel" => GroupAddressStyle::TwoLevel,
        _ => GroupAddressStyle::ThreeLevel,
    }
}

const DELETE_ALL_TABLES: &[&str] = &[
    // Child-to-parent order — matches the reverse of the insert order below.
    "com_object_override",
    "group_link",
    "com_object_instance",
    "binary_data_ref",
    "building_part_device",
    "parameter_instance",
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
              last_modified, project_start, default_language)
         VALUES (0, ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            project.info.project_id,
            project.info.name,
            project.info.project_number,
            style_to_str(project.info.group_address_style),
            completion_to_str(project.info.completion),
            project.info.last_modified.map(|d| d.to_rfc3339()),
            project.info.project_start.map(|d| d.to_rfc3339()),
            project.strings.default_language().0,
        ],
    )?;

    tx.execute(
        "INSERT INTO id_allocators
             (id, device, area, line, com_object_instance, group_range, group_address,
              building_part, parameter_instance)
         VALUES (0, ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            project.ids.peek_device(),
            project.ids.peek_area(),
            project.ids.peek_line(),
            project.ids.peek_com_object_instance(),
            project.ids.peek_group_range(),
            project.ids.peek_group_address(),
            project.ids.peek_building_part(),
            project.ids.peek_parameter_instance(),
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

    tx.commit()?;
    Ok(())
}

/// `Installation::buildings`/`::group_ranges` are flat `Vec`s where each
/// element also carries `position` (sibling order under its own
/// `parent`/`parent_id`, used to rebuild `children`) — see the design
/// doc's "owned-list vs flat-list order" note. `flat_position` is simply
/// the element's index in the flat `Vec`; `position` is recomputed here as
/// the 0-based rank among siblings sharing the same parent, in flat-list
/// order (which is the pre-order the importer itself produces — verified
/// by Task 6/8's own hierarchy tests).
fn flatten_buildings(
    buildings: &[knx_core::building::BuildingPart],
) -> Vec<(i64, i64, &knx_core::building::BuildingPart)> {
    sibling_positions(buildings, |p| p.parent.map(|x| x.0))
        .into_iter()
        .enumerate()
        .map(|(flat, (sib, part))| (sib, flat as i64, part))
        .collect()
}

fn flatten_ranges(
    ranges: &[knx_core::group::GroupRange],
) -> Vec<(i64, i64, &knx_core::group::GroupRange)> {
    sibling_positions(ranges, |r| r.parent.map(|x| x.0))
        .into_iter()
        .enumerate()
        .map(|(flat, (sib, range))| (sib, flat as i64, range))
        .collect()
}

/// For each element, its 0-based rank among the elements preceding it (in
/// slice order) that share its `parent_key`.
fn sibling_positions<T>(items: &[T], parent_key: impl Fn(&T) -> Option<u32>) -> Vec<(i64, &T)> {
    let mut counts: std::collections::HashMap<Option<u32>, i64> = std::collections::HashMap::new();
    items
        .iter()
        .map(|item| {
            let key = parent_key(item);
            let count = counts.entry(key).or_insert(0);
            let position = *count;
            *count += 1;
            (position, item)
        })
        .collect()
}

pub fn load_project(conn: &Connection) -> Result<Project, StoreError> {
    let (
        project_id,
        name,
        project_number,
        style,
        completion,
        last_modified,
        project_start,
        default_language,
    ) = conn
        .query_row(
            "SELECT project_id, name, project_number, group_address_style, completion,
                    last_modified, project_start, default_language
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
                ))
            },
        )
        .optional_not_saved()?;

    let strings = load_string_table(conn, Language(default_language))?;

    let ids = conn.query_row(
        "SELECT device, area, line, com_object_instance, group_range, group_address,
                building_part, parameter_instance
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
            ))
        },
    )?;

    let mut devices = knx_core::devices::Devices::new();
    for device_id in load_all_device_ids(conn)? {
        let mut device = load_device(conn, device_id)?;
        device.com_objects = load_com_object_ids_for_device(conn, device_id)?;
        for com_id in device.com_objects.clone() {
            let mut com = load_com_object_instance(conn, com_id)?;
            com.links = load_group_links(conn, com_id)?;
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
        schema_version: knx_core::project::CURRENT_SCHEMA_VERSION,
        strings,
        info: ProjectInfo {
            project_id,
            name,
            project_number,
            group_address_style: style_from_str(&style),
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
    fn an_empty_project_round_trips() {
        let conn = open_and_migrate_in_memory().unwrap();
        let project = Project::new(Language("en".into()));
        save_project(&conn, &project).unwrap();
        let loaded = load_project(&conn).unwrap();
        assert_eq!(loaded, project);
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
    fn a_project_with_one_device_and_an_unassigned_one_round_trips() {
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

        save_project(&conn, &project).unwrap();
        let loaded = load_project(&conn).unwrap();
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
        assert!(
            matches!(err, StoreError::Sqlite(_)),
            "a dangling parent_id must still fail: {err}"
        );
    }

    #[test]
    fn allocator_state_survives_a_round_trip_and_next_id_does_not_collide() {
        let conn = open_and_migrate_in_memory().unwrap();
        let mut project = Project::new(Language("en".into()));
        let _ = project.ids.next_device_id(); // DeviceId(1)
        let second = project.ids.next_device_id(); // DeviceId(2)
        save_project(&conn, &project).unwrap();
        let mut loaded = load_project(&conn).unwrap();
        let next = loaded.ids.next_device_id();
        assert_eq!(next, DeviceId(3));
        assert_ne!(next, second);
    }
}
