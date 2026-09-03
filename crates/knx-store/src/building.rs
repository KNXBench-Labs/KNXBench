//! Persistence for `BuildingPart` (DATA_MODEL §5) — a recursive hierarchy
//! stored flat (`flat_position` gives `Installation::buildings`'s order;
//! `parent_id` + `position` rebuilds each part's `children`).

use rusqlite::{params, Connection};

use knx_core::building::{BuildingPart, BuildingPartType};
use knx_core::ids::{BuildingPartId, DeviceId, InstallationId, LineId, SourceRef};

use crate::topology::{completion_from_str, completion_to_str};
use crate::StoreError;

fn kind_to_str(k: BuildingPartType) -> &'static str {
    match k {
        BuildingPartType::Building => "Building",
        BuildingPartType::Floor => "Floor",
        BuildingPartType::Room => "Room",
        BuildingPartType::Corridor => "Corridor",
        BuildingPartType::DistributionBoard => "DistributionBoard",
        BuildingPartType::BuildingPart => "BuildingPart",
    }
}

fn kind_from_str(s: &str) -> BuildingPartType {
    match s {
        "Floor" => BuildingPartType::Floor,
        "Room" => BuildingPartType::Room,
        "Corridor" => BuildingPartType::Corridor,
        "DistributionBoard" => BuildingPartType::DistributionBoard,
        "BuildingPart" => BuildingPartType::BuildingPart,
        _ => BuildingPartType::Building,
    }
}

pub fn upsert_building_part(
    conn: &Connection,
    installation_id: InstallationId,
    position: i64,
    flat_position: i64,
    part: &BuildingPart,
) -> Result<(), StoreError> {
    conn.execute(
        "INSERT INTO building_part
             (id, installation_id, parent_id, position, flat_position, source_path,
              source_ets_id, name, number, kind, default_line_id, completion)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
         ON CONFLICT(id) DO UPDATE SET
             installation_id = excluded.installation_id,
             parent_id = excluded.parent_id,
             position = excluded.position,
             flat_position = excluded.flat_position,
             source_path = excluded.source_path,
             source_ets_id = excluded.source_ets_id,
             name = excluded.name,
             number = excluded.number,
             kind = excluded.kind,
             default_line_id = excluded.default_line_id,
             completion = excluded.completion",
        params![
            part.id.0,
            installation_id.0,
            part.parent.map(|p| p.0),
            position,
            flat_position,
            part.source.path,
            part.source.ets_id,
            part.name,
            part.number,
            kind_to_str(part.kind),
            part.default_line.map(|l| l.0),
            completion_to_str(part.completion),
        ],
    )?;
    conn.execute(
        "DELETE FROM building_part_device WHERE building_part_id = ?1",
        params![part.id.0],
    )?;
    let mut stmt = conn.prepare(
        "INSERT INTO building_part_device (building_part_id, device_id, position)
         VALUES (?1, ?2, ?3)",
    )?;
    for (i, device_id) in part.devices.iter().enumerate() {
        stmt.execute(params![part.id.0, device_id.0, i as i64])?;
    }
    Ok(())
}

pub fn load_buildings(
    conn: &Connection,
    installation_id: InstallationId,
) -> Result<Vec<BuildingPart>, StoreError> {
    let mut stmt = conn.prepare(
        "SELECT id, parent_id, source_path, source_ets_id, name, number, kind,
                default_line_id, completion
         FROM building_part WHERE installation_id = ?1 ORDER BY flat_position",
    )?;
    let rows = stmt
        .query_map(params![installation_id.0], |row| {
            Ok((
                row.get::<_, u32>(0)?,
                row.get::<_, Option<u32>>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, Option<String>>(5)?,
                row.get::<_, String>(6)?,
                row.get::<_, Option<u32>>(7)?,
                row.get::<_, String>(8)?,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;

    rows.into_iter()
        .map(
            |(id, parent_id, source_path, source_ets_id, name, number, kind, default_line_id, completion)| {
                let building_id = BuildingPartId(id);
                let mut children_stmt = conn.prepare(
                    "SELECT id FROM building_part WHERE parent_id = ?1 ORDER BY position",
                )?;
                let children = children_stmt
                    .query_map(params![building_id.0], |row| Ok(BuildingPartId(row.get(0)?)))?
                    .collect::<Result<Vec<_>, _>>()?;
                let mut dev_stmt = conn.prepare(
                    "SELECT device_id FROM building_part_device WHERE building_part_id = ?1
                     ORDER BY position",
                )?;
                let devices = dev_stmt
                    .query_map(params![building_id.0], |row| Ok(DeviceId(row.get(0)?)))?
                    .collect::<Result<Vec<_>, _>>()?;
                Ok::<_, StoreError>(BuildingPart {
                    id: building_id,
                    source: SourceRef {
                        path: source_path,
                        ets_id: source_ets_id,
                    },
                    name,
                    number,
                    kind: kind_from_str(&kind),
                    default_line: default_line_id.map(LineId),
                    completion: completion_from_str(&completion),
                    children,
                    devices,
                    parent: parent_id.map(BuildingPartId),
                })
            },
        )
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::open_and_migrate_in_memory;
    use crate::topology::upsert_installation_row;
    use knx_core::commissioning::CompletionStatus;
    use knx_core::installation::Installation;
    use knx_core::topology::Topology;

    fn installation() -> Installation {
        Installation {
            id: InstallationId(0),
            name: "H".into(),
            default_line: None,
            multicast_address: None,
            completion: CompletionStatus::FinishedDesign,
            topology: Topology { areas: vec![], lines: vec![], unassigned: vec![] },
            buildings: vec![],
            group_ranges: vec![],
            group_addresses: vec![],
            parameters: vec![],
        }
    }

    fn part(id: u32, parent: Option<u32>, kind: BuildingPartType) -> BuildingPart {
        BuildingPart {
            id: BuildingPartId(id),
            source: SourceRef { path: "t".into(), ets_id: "t".into() },
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

    #[test]
    fn a_hierarchy_with_siblings_round_trips_with_correct_flat_order_and_sibling_order() {
        let conn = open_and_migrate_in_memory().unwrap();
        upsert_installation_row(&conn, &installation()).unwrap();
        let building = part(1, None, BuildingPartType::Building);
        let floor = part(2, Some(1), BuildingPartType::Floor);
        let mut room = part(3, Some(2), BuildingPartType::Room);
        room.devices = vec![DeviceId(9), DeviceId(7)]; // deliberately non-sorted by id
        let corridor = part(4, Some(2), BuildingPartType::Corridor); // room's sibling under floor

        // Insert device rows to satisfy building_part_device's FK. Minimal
        // direct SQL, matching the migration test's own style — devices.rs's
        // upsert_device is not yet wired to installation membership here.
        for id in [7, 9] {
            conn.execute(
                &format!(
                    "INSERT INTO device (id, installation_id, line_id, topology_position,
                        source_path, source_ets_id, name, product_ref, program_ref, completion,
                        individual_address_loaded, application_program_loaded, parameters_loaded,
                        communication_part_loaded, medium_config_loaded, broken,
                        visibility_calculated)
                     VALUES ({id}, 0, NULL, 0, 't', 't', 'D', 'P', 'H', 'FinishedDesign',
                             0, 0, 0, 0, 0, 0, 1)"
                ),
                [],
            )
            .unwrap();
        }

        // `position` (sibling rank under `parent_id`) and `flat_position`
        // (index in the flat `Installation::buildings` list) are deliberately
        // chosen so neither one coincides with the other, with id order, or
        // with insertion order (building, floor, room, corridor — in that
        // call sequence below). A swap of the two orderings, or a fallback to
        // id/rowid order for either, therefore produces a visibly different
        // (and test-failing) sequence rather than coincidentally matching:
        //
        //   id:            1(building) 2(floor)   3(room)    4(corridor)
        //   position:      3           2          1          0
        //   flat_position: 0           1          3          2
        //
        // Correct flat order (by flat_position): building, floor, corridor, room.
        // Sibling order under floor (by position, room=1 and corridor=0):
        // corridor before room — reversed from both insertion order (room
        // inserted before corridor) and id order (room=3 < corridor=4).
        upsert_building_part(&conn, InstallationId(0), 3, 0, &building).unwrap();
        upsert_building_part(&conn, InstallationId(0), 2, 1, &floor).unwrap();
        upsert_building_part(&conn, InstallationId(0), 1, 3, &room).unwrap();
        upsert_building_part(&conn, InstallationId(0), 0, 2, &corridor).unwrap();

        let loaded = load_buildings(&conn, InstallationId(0)).unwrap();
        assert_eq!(loaded.len(), 4);

        // Flat (top-level) order follows `flat_position`. Ordering by
        // `position` instead would yield [corridor, room, floor, building];
        // falling back to id/insertion order would yield
        // [building, floor, room, corridor]. Both differ from the expected
        // sequence below.
        let ids: Vec<u32> = loaded.iter().map(|p| p.id.0).collect();
        assert_eq!(ids, vec![1, 2, 4, 3]); // building, floor, corridor, room

        assert_eq!(loaded[0].id, BuildingPartId(1));
        assert_eq!(loaded[0].children, vec![BuildingPartId(2)]);

        // Floor's children are ordered by `position`: corridor (0) before
        // room (1) — reversed from both id order and insertion order, so
        // reconstructing by id/rowid instead of `position` would fail this.
        assert_eq!(loaded[1].id, BuildingPartId(2));
        assert_eq!(loaded[1].children, vec![BuildingPartId(4), BuildingPartId(3)]);

        let room_loaded = loaded.iter().find(|p| p.id == BuildingPartId(3)).unwrap();
        assert_eq!(room_loaded.devices, vec![DeviceId(9), DeviceId(7)]);
        assert_eq!(room_loaded.parent, Some(BuildingPartId(2)));
    }
}
