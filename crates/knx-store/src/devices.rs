//! Persistence for `DeviceInstance` and `ComObjectInstance`
//! (DATA_MODEL §4) — the two entities `knx_core::devices::Devices` owns.
//! This file grows across three plan tasks: device + binary data here,
//! `com_object_instance` + the `Override<T>` codec next, `group_link`
//! last (it needs `group_address` rows to exist first).

use rusqlite::{params, Connection};

use knx_core::address::IndividualAddress;
use knx_core::commissioning::CommissioningState;
use knx_core::device::{BinaryDataRef, DeviceInstance};
use knx_core::ids::{DeviceId, InstallationId, SourceRef};

use crate::topology::{completion_from_str, completion_to_str};
use crate::StoreError;

pub fn upsert_device(
    conn: &Connection,
    installation_id: InstallationId,
    topology_position: i64,
    device: &DeviceInstance,
) -> Result<(), StoreError> {
    let c = &device.commissioning;
    conn.execute(
        "INSERT INTO device
             (id, installation_id, line_id, topology_position, source_path, source_ets_id,
              name, description, address, product_ref, program_ref, completion,
              individual_address_loaded, application_program_loaded, parameters_loaded,
              communication_part_loaded, medium_config_loaded, last_modified, last_download,
              broken, visibility_calculated)
         VALUES (?1, ?2, NULL, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16,
                 ?17, ?18, ?19, ?20)
         ON CONFLICT(id) DO UPDATE SET
             installation_id = excluded.installation_id,
             source_path = excluded.source_path,
             source_ets_id = excluded.source_ets_id,
             name = excluded.name,
             description = excluded.description,
             address = excluded.address,
             product_ref = excluded.product_ref,
             program_ref = excluded.program_ref,
             completion = excluded.completion,
             individual_address_loaded = excluded.individual_address_loaded,
             application_program_loaded = excluded.application_program_loaded,
             parameters_loaded = excluded.parameters_loaded,
             communication_part_loaded = excluded.communication_part_loaded,
             medium_config_loaded = excluded.medium_config_loaded,
             last_modified = excluded.last_modified,
             last_download = excluded.last_download,
             broken = excluded.broken,
             visibility_calculated = excluded.visibility_calculated",
        params![
            device.id.0,
            installation_id.0,
            topology_position,
            device.source.path,
            device.source.ets_id,
            device.name,
            device.description,
            device.address.map(|a| a.raw()),
            device.product_ref,
            device.program_ref,
            completion_to_str(c.completion),
            c.individual_address_loaded,
            c.application_program_loaded,
            c.parameters_loaded,
            c.communication_part_loaded,
            c.medium_config_loaded,
            c.last_modified.map(|d| d.to_rfc3339()),
            c.last_download.map(|d| d.to_rfc3339()),
            c.broken,
            device.visibility_calculated,
        ],
    )?;
    // NOTE: the INSERT above always sets line_id = NULL — placement into a
    // line is Task 4/11's job (topology::upsert_line owns the device's
    // *membership*, not this row's own line_id column directly, since a
    // device can be reassigned between lines independent of its own
    // fields). A later task revisits this: see Task 11's orchestration
    // note on device placement, which sets `device.line_id` with a
    // separate targeted UPDATE once the owning line is known. Until then,
    // `topology::load_topology`'s "unassigned" query would misclassify
    // every device as unassigned — Task 11 must not skip that step.
    //
    // Note the ON CONFLICT clause deliberately omits both `line_id` and
    // `topology_position` from its SET list (unlike `topology::upsert_area`/
    // `upsert_line`, which do update their own `position` on conflict —
    // those columns are not co-owned by a second function the way a
    // device's placement is). Re-running upsert_device on an already-placed
    // device therefore leaves its existing line_id/topology_position
    // untouched; placement is set exclusively through `set_device_line`
    // below, which updates both columns together as one unit.
    conn.execute(
        "DELETE FROM binary_data_ref WHERE device_id = ?1",
        params![device.id.0],
    )?;
    let mut stmt = conn.prepare(
        "INSERT INTO binary_data_ref (device_id, position, blob_id, name) VALUES (?1, ?2, ?3, ?4)",
    )?;
    for (i, b) in device.binary_data.iter().enumerate() {
        stmt.execute(params![device.id.0, i as i64, b.id, b.name])?;
    }
    Ok(())
}

/// Sets a device's line placement directly — the one column `upsert_device`
/// deliberately leaves untouched. Called by Task 11's orchestration once
/// the owning `Line`'s id is known (or with `line_id = None` for an
/// unassigned device), and by Task 12's `sync_after_command` for
/// `SetIndividualAddress` (which does not move a device between lines, but
/// re-asserts the same placement is harmless and keeps that call site to
/// one function).
pub fn set_device_line(
    conn: &Connection,
    device_id: DeviceId,
    line_id: Option<knx_core::ids::LineId>,
    topology_position: i64,
) -> Result<(), StoreError> {
    conn.execute(
        "UPDATE device SET line_id = ?1, topology_position = ?2 WHERE id = ?3",
        params![line_id.map(|l| l.0), topology_position, device_id.0],
    )?;
    Ok(())
}

/// Loads the `device` row and its `binary_data_ref` rows. `com_objects` is
/// left empty — it is filled in by Task 11's orchestration, which loads
/// `com_object_instance` rows (Task 7) for this device and assigns them
/// into the returned value's `com_objects` field.
pub fn load_device(conn: &Connection, id: DeviceId) -> Result<DeviceInstance, StoreError> {
    let (
        source_path,
        source_ets_id,
        name,
        description,
        address,
        product_ref,
        program_ref,
        completion,
        individual_address_loaded,
        application_program_loaded,
        parameters_loaded,
        communication_part_loaded,
        medium_config_loaded,
        last_modified,
        last_download,
        broken,
        visibility_calculated,
    ) = conn.query_row(
        "SELECT source_path, source_ets_id, name, description, address, product_ref,
                program_ref, completion, individual_address_loaded, application_program_loaded,
                parameters_loaded, communication_part_loaded, medium_config_loaded,
                last_modified, last_download, broken, visibility_calculated
         FROM device WHERE id = ?1",
        params![id.0],
        |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, Option<String>>(3)?,
                row.get::<_, Option<u16>>(4)?,
                row.get::<_, String>(5)?,
                row.get::<_, String>(6)?,
                row.get::<_, String>(7)?,
                row.get::<_, bool>(8)?,
                row.get::<_, bool>(9)?,
                row.get::<_, bool>(10)?,
                row.get::<_, bool>(11)?,
                row.get::<_, bool>(12)?,
                row.get::<_, Option<String>>(13)?,
                row.get::<_, Option<String>>(14)?,
                row.get::<_, bool>(15)?,
                row.get::<_, bool>(16)?,
            ))
        },
    )?;

    let mut bd_stmt = conn.prepare(
        "SELECT blob_id, name FROM binary_data_ref WHERE device_id = ?1 ORDER BY position",
    )?;
    let binary_data = bd_stmt
        .query_map(params![id.0], |row| {
            Ok(BinaryDataRef {
                id: row.get(0)?,
                name: row.get(1)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(DeviceInstance {
        id,
        source: SourceRef {
            path: source_path,
            ets_id: source_ets_id,
        },
        name,
        description,
        address: address.map(IndividualAddress::from_raw),
        product_ref,
        program_ref,
        commissioning: CommissioningState {
            completion: completion_from_str(&completion),
            individual_address_loaded,
            application_program_loaded,
            parameters_loaded,
            communication_part_loaded,
            medium_config_loaded,
            last_modified: last_modified.map(|s| {
                chrono::DateTime::parse_from_rfc3339(&s)
                    .expect("stored timestamp is always valid RFC3339")
                    .with_timezone(&chrono::Utc)
            }),
            last_download: last_download.map(|s| {
                chrono::DateTime::parse_from_rfc3339(&s)
                    .expect("stored timestamp is always valid RFC3339")
                    .with_timezone(&chrono::Utc)
            }),
            broken,
        },
        visibility_calculated,
        com_objects: vec![],
        binary_data,
    })
}

pub fn load_all_device_ids(conn: &Connection) -> Result<Vec<DeviceId>, StoreError> {
    let mut stmt = conn.prepare("SELECT id FROM device ORDER BY id")?;
    let ids = stmt
        .query_map([], |row| Ok(DeviceId(row.get(0)?)))?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(ids)
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

    fn device() -> DeviceInstance {
        DeviceInstance {
            id: DeviceId(1),
            source: SourceRef {
                path: "0.xml".into(),
                ets_id: "M-1".into(),
            },
            name: "Aktor".into(),
            description: Some("Schaltaktor".into()),
            address: Some(IndividualAddress::new(1, 1, 5).unwrap()),
            product_ref: "P-1".into(),
            program_ref: "H-1".into(),
            commissioning: CommissioningState {
                completion: CompletionStatus::FinishedDesign,
                individual_address_loaded: true,
                application_program_loaded: true,
                parameters_loaded: false,
                communication_part_loaded: true,
                medium_config_loaded: false,
                last_modified: None,
                last_download: None,
                broken: false,
            },
            visibility_calculated: true,
            com_objects: vec![],
            binary_data: vec![
                BinaryDataRef {
                    id: "guid-1".into(),
                    name: "cert.dat".into(),
                },
                BinaryDataRef {
                    id: "guid-2".into(),
                    name: "key.dat".into(),
                },
            ],
        }
    }

    #[test]
    fn a_device_with_binary_data_round_trips() {
        let conn = open_and_migrate_in_memory().unwrap();
        upsert_installation_row(&conn, &installation()).unwrap();
        let d = device();
        upsert_device(&conn, InstallationId(0), 0, &d).unwrap();
        let loaded = load_device(&conn, d.id).unwrap();
        assert_eq!(loaded.name, d.name);
        assert_eq!(loaded.binary_data, d.binary_data);
        assert_eq!(loaded.commissioning, d.commissioning);
        assert_eq!(loaded.address, d.address);
    }

    #[test]
    fn a_device_without_an_address_or_description_round_trips() {
        let conn = open_and_migrate_in_memory().unwrap();
        upsert_installation_row(&conn, &installation()).unwrap();
        let mut d = device();
        d.address = None;
        d.description = None;
        d.binary_data = vec![];
        upsert_device(&conn, InstallationId(0), 0, &d).unwrap();
        let loaded = load_device(&conn, d.id).unwrap();
        assert_eq!(loaded.address, None);
        assert_eq!(loaded.description, None);
        assert_eq!(loaded.binary_data, vec![]);
    }

    #[test]
    fn set_device_line_updates_placement_without_touching_other_fields() {
        let conn = open_and_migrate_in_memory().unwrap();
        upsert_installation_row(&conn, &installation()).unwrap();
        let d = device();
        upsert_device(&conn, InstallationId(0), 0, &d).unwrap();
        // A line row must exist to satisfy the foreign key.
        conn.execute(
            "INSERT INTO area (id, installation_id, position, source_path, source_ets_id, name, address, completion)
             VALUES (1, 0, 0, 't', 't', 'A', 1, 'FinishedDesign')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO line (id, area_id, position, source_path, source_ets_id, name, address, medium_ref, completion)
             VALUES (1, 1, 0, 't', 't', 'L', 1, 'TP', 'FinishedDesign')",
            [],
        )
        .unwrap();
        set_device_line(&conn, d.id, Some(knx_core::ids::LineId(1)), 0).unwrap();
        let line_id: Option<i64> = conn
            .query_row("SELECT line_id FROM device WHERE id = 1", [], |r| r.get(0))
            .unwrap();
        assert_eq!(line_id, Some(1));
        let loaded = load_device(&conn, d.id).unwrap();
        assert_eq!(loaded.name, d.name); // untouched
    }
}
