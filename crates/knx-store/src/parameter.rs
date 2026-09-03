//! Persistence for `ParameterInstance` — retained but uninterpreted
//! (DATA_MODEL §10). `raw` is never parsed here or anywhere else yet.

use rusqlite::{params, Connection};

use knx_core::ids::{InstallationId, ParameterInstanceId, SourceRef};
use knx_core::parameter::ParameterInstance;

use crate::StoreError;

pub fn upsert_parameter_instance(
    conn: &Connection,
    position: i64,
    p: &ParameterInstance,
) -> Result<(), StoreError> {
    conn.execute(
        "INSERT INTO parameter_instance (id, device_id, position, source_path, source_ets_id, raw)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)
         ON CONFLICT(id) DO UPDATE SET
             device_id = excluded.device_id,
             position = excluded.position,
             source_path = excluded.source_path,
             source_ets_id = excluded.source_ets_id,
             raw = excluded.raw",
        params![p.id.0, p.device.0, position, p.source.path, p.source.ets_id, p.raw],
    )?;
    Ok(())
}

pub fn load_parameters_for_installation(
    conn: &Connection,
    installation_id: InstallationId,
) -> Result<Vec<ParameterInstance>, StoreError> {
    let mut stmt = conn.prepare(
        "SELECT p.id, p.device_id, p.source_path, p.source_ets_id, p.raw
         FROM parameter_instance p JOIN device d ON p.device_id = d.id
         WHERE d.installation_id = ?1 ORDER BY p.position",
    )?;
    let params_ = stmt
        .query_map(params![installation_id.0], |row| {
            Ok(ParameterInstance {
                id: ParameterInstanceId(row.get(0)?),
                device: knx_core::ids::DeviceId(row.get(1)?),
                source: SourceRef {
                    path: row.get(2)?,
                    ets_id: row.get(3)?,
                },
                raw: row.get(4)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(params_)
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
            topology: Topology { areas: vec![], lines: vec![], unassigned: vec![] },
            buildings: vec![],
            group_ranges: vec![],
            group_addresses: vec![],
            parameters: vec![],
        }
    }

    fn device() -> DeviceInstance {
        DeviceInstance {
            id: knx_core::ids::DeviceId(1),
            source: SourceRef { path: "t".into(), ets_id: "t".into() },
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
    fn parameters_round_trip_in_position_order() {
        let conn = open_and_migrate_in_memory().unwrap();
        upsert_installation_row(&conn, &installation()).unwrap();
        let d = device();
        upsert_device(&conn, InstallationId(0), 0, &d).unwrap();

        let p1 = ParameterInstance {
            id: ParameterInstanceId(1),
            device: d.id,
            source: SourceRef { path: "t".into(), ets_id: "M-1_P-1_R-1".into() },
            raw: "1".into(),
        };
        let p2 = ParameterInstance {
            id: ParameterInstanceId(2),
            device: d.id,
            source: SourceRef { path: "t".into(), ets_id: "M-1_UP-2_R-2".into() },
            raw: "42".into(),
        };
        upsert_parameter_instance(&conn, 0, &p1).unwrap();
        upsert_parameter_instance(&conn, 1, &p2).unwrap();
        assert_eq!(
            load_parameters_for_installation(&conn, InstallationId(0)).unwrap(),
            vec![p1, p2]
        );
    }
}
