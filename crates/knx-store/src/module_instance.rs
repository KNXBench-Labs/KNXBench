//! Persistence for `ModuleInstance` (ADR-0013) — retained but
//! uninterpreted, same discipline as `parameter.rs`.

use rusqlite::{params, Connection};

use knx_core::ids::{DeviceId, InstallationId, ModuleInstanceId, SourceRef};
use knx_core::module::ModuleInstance;

use crate::StoreError;

pub fn upsert_module_instance(
    conn: &Connection,
    position: i64,
    m: &ModuleInstance,
) -> Result<(), StoreError> {
    conn.execute(
        "INSERT INTO module_instance (id, device_id, position, source_path, source_ets_id, repeat_index)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)
         ON CONFLICT(id) DO UPDATE SET
             device_id = excluded.device_id, position = excluded.position,
             source_path = excluded.source_path, source_ets_id = excluded.source_ets_id,
             repeat_index = excluded.repeat_index",
        params![m.id.0, m.device.0, position, m.source.path, m.source.ets_id, m.repeat_index],
    )?;
    conn.execute(
        "DELETE FROM module_instance_argument WHERE module_instance_id = ?1",
        params![m.id.0],
    )?;
    for (i, (source, value)) in m.arguments.iter().enumerate() {
        conn.execute(
            "INSERT INTO module_instance_argument (module_instance_id, position, source_path, source_ets_id, value)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![m.id.0, i as i64, source.path, source.ets_id, value],
        )?;
    }
    Ok(())
}

pub fn load_module_instances_for_installation(
    conn: &Connection,
    installation_id: InstallationId,
) -> Result<Vec<ModuleInstance>, StoreError> {
    let mut stmt = conn.prepare(
        "SELECT m.id, m.device_id, m.source_path, m.source_ets_id, m.repeat_index
         FROM module_instance m JOIN device d ON m.device_id = d.id
         WHERE d.installation_id = ?1 ORDER BY m.position",
    )?;
    let rows: Vec<(ModuleInstanceId, DeviceId, String, String, String)> = stmt
        .query_map(params![installation_id.0], |row| {
            Ok((
                ModuleInstanceId(row.get(0)?),
                DeviceId(row.get(1)?),
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;

    let mut out = Vec::with_capacity(rows.len());
    for (id, device, path, ets_id, repeat_index) in rows {
        let mut arg_stmt = conn.prepare(
            "SELECT source_path, source_ets_id, value FROM module_instance_argument
             WHERE module_instance_id = ?1 ORDER BY position",
        )?;
        let arguments = arg_stmt
            .query_map(params![id.0], |row| {
                Ok((
                    SourceRef {
                        path: row.get(0)?,
                        ets_id: row.get(1)?,
                    },
                    row.get(2)?,
                ))
            })?
            .collect::<Result<Vec<_>, _>>()?;
        out.push(ModuleInstance {
            id,
            device,
            source: SourceRef { path, ets_id },
            repeat_index,
            arguments,
        });
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::devices::upsert_device;
    use crate::open_and_migrate_in_memory;
    use crate::topology::upsert_installation_row;
    use knx_core::commissioning::{CommissioningState, CompletionStatus};
    use knx_core::device::DeviceInstance;
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
            id: knx_core::ids::DeviceId(1),
            source: SourceRef {
                path: "t".into(),
                ets_id: "t".into(),
            },
            name: "D".into(),
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

    #[test]
    fn module_instances_round_trip_in_position_order() {
        let conn = open_and_migrate_in_memory().unwrap();
        upsert_installation_row(&conn, &installation()).unwrap();
        let d = device();
        upsert_device(&conn, InstallationId(0), 0, &d).unwrap();

        let m1 = ModuleInstance {
            id: ModuleInstanceId(1),
            device: d.id,
            source: SourceRef {
                path: "t".into(),
                ets_id: "MD-1_M-1".into(),
            },
            repeat_index: "1x1".into(),
            arguments: vec![],
        };
        let m2 = ModuleInstance {
            id: ModuleInstanceId(2),
            device: d.id,
            source: SourceRef {
                path: "t".into(),
                ets_id: "MD-2_M-1".into(),
            },
            repeat_index: "6x1".into(),
            arguments: vec![
                (
                    SourceRef {
                        path: "t".into(),
                        ets_id: "MD-2_A-1".into(),
                    },
                    "1".into(),
                ),
                (
                    SourceRef {
                        path: "t".into(),
                        ets_id: "MD-2_A-2".into(),
                    },
                    "0".into(),
                ),
            ],
        };
        upsert_module_instance(&conn, 0, &m1).unwrap();
        upsert_module_instance(&conn, 1, &m2).unwrap();
        assert_eq!(
            load_module_instances_for_installation(&conn, InstallationId(0)).unwrap(),
            vec![m1, m2]
        );
    }
}
