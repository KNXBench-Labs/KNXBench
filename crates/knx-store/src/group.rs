//! Persistence for `GroupRange` and `GroupAddressEntry` (DATA_MODEL §9).
//! `GroupRange` is stored flat with the same two-ordering pattern as
//! `BuildingPart` (`building.rs`): `flat_position` for `Installation::
//! group_ranges`, `position` (sibling order under `parent_id`) for
//! `GroupRange::children`.

use rusqlite::{params, Connection};

use knx_core::address::GroupAddress;
use knx_core::group::{GroupAddressEntry, GroupRange};
use knx_core::ids::{GroupAddressId, GroupRangeId, InstallationId, SourceRef};

use crate::StoreError;

pub fn upsert_group_range(
    conn: &Connection,
    installation_id: InstallationId,
    position: i64,
    flat_position: i64,
    range: &GroupRange,
) -> Result<(), StoreError> {
    conn.execute(
        "INSERT INTO group_range
             (id, installation_id, parent_id, position, flat_position, source_path,
              source_ets_id, name, range_start, range_end)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
         ON CONFLICT(id) DO UPDATE SET
             installation_id = excluded.installation_id,
             parent_id = excluded.parent_id,
             position = excluded.position,
             flat_position = excluded.flat_position,
             source_path = excluded.source_path,
             source_ets_id = excluded.source_ets_id,
             name = excluded.name,
             range_start = excluded.range_start,
             range_end = excluded.range_end",
        params![
            range.id.0,
            installation_id.0,
            range.parent.map(|p| p.0),
            position,
            flat_position,
            range.source.path,
            range.source.ets_id,
            range.name,
            range.start.raw(),
            range.end.raw(),
        ],
    )?;
    Ok(())
}

pub fn load_group_ranges(
    conn: &Connection,
    installation_id: InstallationId,
) -> Result<Vec<GroupRange>, StoreError> {
    let mut stmt = conn.prepare(
        "SELECT id, parent_id, source_path, source_ets_id, name, range_start, range_end
         FROM group_range WHERE installation_id = ?1 ORDER BY flat_position",
    )?;
    let rows = stmt
        .query_map(params![installation_id.0], |row| {
            Ok((
                row.get::<_, u32>(0)?,
                row.get::<_, Option<u32>>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, u16>(5)?,
                row.get::<_, u16>(6)?,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;

    rows.into_iter()
        .map(
            |(id, parent_id, source_path, source_ets_id, name, start, end)| {
                let range_id = GroupRangeId(id);
                let mut children_stmt = conn
                    .prepare("SELECT id FROM group_range WHERE parent_id = ?1 ORDER BY position")?;
                let children = children_stmt
                    .query_map(params![range_id.0], |row| Ok(GroupRangeId(row.get(0)?)))?
                    .collect::<Result<Vec<_>, _>>()?;
                Ok::<_, StoreError>(GroupRange {
                    id: range_id,
                    source: SourceRef {
                        path: source_path,
                        ets_id: source_ets_id,
                    },
                    name,
                    start: GroupAddress::from_raw(start),
                    end: GroupAddress::from_raw(end),
                    parent: parent_id.map(GroupRangeId),
                    children,
                })
            },
        )
        .collect()
}

pub fn upsert_group_address(
    conn: &Connection,
    installation_id: InstallationId,
    position: i64,
    entry: &GroupAddressEntry,
) -> Result<(), StoreError> {
    let dpt = crate::devices::encode_dpt(&entry.declared_dpt);
    conn.execute(
        "INSERT INTO group_address
             (id, installation_id, range_id, position, source_path, source_ets_id, name,
              address, central, unfiltered, dpt_state, dpt_value, dpt_layer)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
         ON CONFLICT(id) DO UPDATE SET
             installation_id = excluded.installation_id,
             range_id = excluded.range_id,
             position = excluded.position,
             source_path = excluded.source_path,
             source_ets_id = excluded.source_ets_id,
             name = excluded.name,
             address = excluded.address,
             central = excluded.central,
             unfiltered = excluded.unfiltered,
             dpt_state = excluded.dpt_state,
             dpt_value = excluded.dpt_value,
             dpt_layer = excluded.dpt_layer",
        params![
            entry.id.0,
            installation_id.0,
            entry.range.map(|r| r.0),
            position,
            entry.source.path,
            entry.source.ets_id,
            entry.name,
            entry.address.raw(),
            entry.central,
            entry.unfiltered,
            dpt.state,
            dpt.value,
            dpt.layer,
        ],
    )?;
    Ok(())
}

pub fn load_group_addresses(
    conn: &Connection,
    installation_id: InstallationId,
) -> Result<Vec<GroupAddressEntry>, StoreError> {
    let mut stmt = conn.prepare(
        "SELECT id, range_id, source_path, source_ets_id, name, address, central, unfiltered,
                dpt_state, dpt_value, dpt_layer
         FROM group_address WHERE installation_id = ?1 ORDER BY position",
    )?;
    let entries = stmt
        .query_map(params![installation_id.0], |row| {
            Ok(GroupAddressEntry {
                id: GroupAddressId(row.get(0)?),
                range: row.get::<_, Option<u32>>(1)?.map(GroupRangeId),
                source: SourceRef {
                    path: row.get(2)?,
                    ets_id: row.get(3)?,
                },
                name: row.get(4)?,
                address: GroupAddress::from_raw(row.get(5)?),
                central: row.get(6)?,
                unfiltered: row.get(7)?,
                declared_dpt: crate::devices::decode_dpt(
                    &row.get::<_, String>(8)?,
                    row.get(9)?,
                    row.get(10)?,
                ),
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(entries)
}

pub fn delete_group_address(conn: &Connection, id: GroupAddressId) -> Result<(), StoreError> {
    conn.execute(
        "DELETE FROM group_link WHERE group_address_id = ?1",
        params![id.0],
    )?;
    conn.execute("DELETE FROM group_address WHERE id = ?1", params![id.0])?;
    Ok(())
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
            topology: Topology {
                areas: vec![],
                lines: vec![],
                unassigned: vec![],
            },
            buildings: vec![],
            group_ranges: vec![],
            group_addresses: vec![],
            parameters: vec![],
        }
    }

    fn source() -> SourceRef {
        SourceRef {
            path: "t".into(),
            ets_id: "t".into(),
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

    #[test]
    fn a_range_with_two_children_round_trips_with_correct_flat_order_and_sibling_order() {
        let conn = open_and_migrate_in_memory().unwrap();
        upsert_installation_row(&conn, &installation()).unwrap();

        let main_a = range(1, None, 2048, 4095);
        let middle_1 = range(2, Some(1), 2048, 2303);
        let middle_2 = range(3, Some(1), 2304, 2559);

        // `position` (sibling rank under `parent_id`) and `flat_position`
        // (index in the flat `Installation::group_ranges` list) are chosen
        // so that neither coincides with the other, with id order, or with
        // insertion order (main_a, middle_1, middle_2 — the call sequence
        // below). A swap of the two orderings, or a fallback to id/rowid
        // order for either, therefore produces a visibly different (and
        // test-failing) sequence rather than coincidentally matching:
        //
        //   id:            1(main_a)  2(middle_1)  3(middle_2)
        //   position:      2          1            0
        //   flat_position: 1          0            2
        //
        // Correct flat order (by flat_position): middle_1, main_a, middle_2.
        // Sibling order under main_a (by position, middle_2=0, middle_1=1):
        // middle_2 before middle_1 — reversed from both insertion order
        // (middle_1 inserted before middle_2) and id order (middle_1=2 <
        // middle_2=3).
        //
        // Using `flat_position` to order middle_1/middle_2's siblings instead
        // (middle_1=0, middle_2=2) would give [middle_1, middle_2] — the
        // opposite of the correct [middle_2, middle_1] — so a swap of the two
        // fields is caught here, not just a swap at the top level.
        upsert_group_range(&conn, InstallationId(0), 2, 1, &main_a).unwrap();
        upsert_group_range(&conn, InstallationId(0), 1, 0, &middle_1).unwrap();
        upsert_group_range(&conn, InstallationId(0), 0, 2, &middle_2).unwrap();

        let loaded = load_group_ranges(&conn, InstallationId(0)).unwrap();

        let ids: Vec<u32> = loaded.iter().map(|r| r.id.0).collect();
        assert_eq!(ids, vec![2, 1, 3], "flat order must follow flat_position");

        let main_a_loaded = loaded.iter().find(|r| r.id == GroupRangeId(1)).unwrap();
        assert_eq!(
            main_a_loaded.children,
            vec![GroupRangeId(3), GroupRangeId(2)],
            "children must follow position, not id/insertion order"
        );
        assert_eq!(main_a_loaded.parent, None);

        let middle_1_loaded = loaded.iter().find(|r| r.id == GroupRangeId(2)).unwrap();
        assert_eq!(middle_1_loaded.parent, Some(GroupRangeId(1)));
        assert_eq!(middle_1_loaded.children, vec![]);

        let middle_2_loaded = loaded.iter().find(|r| r.id == GroupRangeId(3)).unwrap();
        assert_eq!(middle_2_loaded.parent, Some(GroupRangeId(1)));
        assert_eq!(middle_2_loaded.children, vec![]);

        // Full-struct equality as a final cross-check, including names,
        // source and address bounds round-tripping unchanged.
        let mut expected_main_a = main_a.clone();
        expected_main_a.children = vec![GroupRangeId(3), GroupRangeId(2)];
        assert_eq!(main_a_loaded, &expected_main_a);
        assert_eq!(middle_1_loaded, &middle_1);
        assert_eq!(middle_2_loaded, &middle_2);
    }

    #[test]
    fn a_group_address_round_trips_and_deletes_cleanly() {
        let conn = open_and_migrate_in_memory().unwrap();
        upsert_installation_row(&conn, &installation()).unwrap();
        let entry = GroupAddressEntry {
            id: GroupAddressId(1),
            source: source(),
            name: "Licht EG An/Aus".into(),
            address: GroupAddress::from_raw(2048),
            central: false,
            unfiltered: false,
            range: None,
            declared_dpt: Default::default(),
        };
        upsert_group_address(&conn, InstallationId(0), 0, &entry).unwrap();
        assert_eq!(
            load_group_addresses(&conn, InstallationId(0)).unwrap(),
            vec![entry.clone()]
        );
        delete_group_address(&conn, entry.id).unwrap();
        assert_eq!(
            load_group_addresses(&conn, InstallationId(0)).unwrap(),
            vec![]
        );
    }

    #[test]
    fn a_group_address_declaration_round_trips_in_all_four_states() {
        use knx_core::dpt::DptRef;
        use knx_core::provenance::{Layer, Override, Resolved};
        let conn = open_and_migrate_in_memory().unwrap();
        upsert_installation_row(&conn, &installation()).unwrap();
        let states = [
            Override::Absent,
            Override::Empty,
            Override::Malformed("DPST-1-1 DPST-1-2".into()),
            Override::Value(Resolved {
                value: DptRef {
                    main: 9,
                    sub: Some(1),
                },
                layer: Layer::Instance,
            }),
            Override::Value(Resolved {
                value: DptRef { main: 5, sub: None },
                layer: Layer::UserEdit,
            }),
        ];
        let entries: Vec<GroupAddressEntry> = states
            .into_iter()
            .enumerate()
            .map(|(i, declared_dpt)| GroupAddressEntry {
                id: GroupAddressId(i as u32 + 1),
                source: source(),
                name: format!("ga {i}"),
                address: GroupAddress::from_raw(2048 + i as u16),
                central: false,
                unfiltered: false,
                range: None,
                declared_dpt,
            })
            .collect();
        for (i, entry) in entries.iter().enumerate() {
            upsert_group_address(&conn, InstallationId(0), i as i64, entry).unwrap();
        }
        assert_eq!(
            load_group_addresses(&conn, InstallationId(0)).unwrap(),
            entries
        );
    }

    #[test]
    fn a_group_address_without_a_range_is_valid() {
        let conn = open_and_migrate_in_memory().unwrap();
        upsert_installation_row(&conn, &installation()).unwrap();
        let entry = GroupAddressEntry {
            id: GroupAddressId(1),
            source: source(),
            name: "Sonder".into(),
            address: GroupAddress::from_raw(1),
            central: true,
            unfiltered: true,
            range: None,
            declared_dpt: Default::default(),
        };
        upsert_group_address(&conn, InstallationId(0), 0, &entry).unwrap();
        let loaded = load_group_addresses(&conn, InstallationId(0)).unwrap();
        assert_eq!(loaded[0].range, None);
    }
}
