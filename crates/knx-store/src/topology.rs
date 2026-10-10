//! Persistence for `Installation`'s scalar fields plus its `Topology`
//! (`Area`/`Line`, DATA_MODEL §5). Buildings, group ranges/addresses and
//! parameters — the rest of what `Installation` owns — are other modules'
//! tables, assembled together only in `project::save_project`/
//! `load_project`.

use std::net::Ipv4Addr;

use rusqlite::{params, Connection};

use knx_core::commissioning::CompletionStatus;
use knx_core::ids::{AreaId, DeviceId, InstallationId, LineId, SourceRef};
use knx_core::installation::Installation;
use knx_core::topology::{Area, Line, Topology};

use crate::StoreError;

pub fn completion_to_str(c: CompletionStatus) -> &'static str {
    match c {
        CompletionStatus::Undefined => "Undefined",
        CompletionStatus::Editing => "Editing",
        CompletionStatus::FinishedDesign => "FinishedDesign",
        CompletionStatus::FinishedCommissioning => "FinishedCommissioning",
        CompletionStatus::Tested => "Tested",
        CompletionStatus::Locked => "Locked",
        CompletionStatus::Accepted => "Accepted",
    }
}

pub fn completion_from_str(s: &str) -> rusqlite::Result<CompletionStatus> {
    Ok(match s {
        "Editing" => CompletionStatus::Editing,
        "FinishedDesign" => CompletionStatus::FinishedDesign,
        "FinishedCommissioning" => CompletionStatus::FinishedCommissioning,
        "Tested" => CompletionStatus::Tested,
        "Locked" => CompletionStatus::Locked,
        "Accepted" => CompletionStatus::Accepted,
        "Undefined" => CompletionStatus::Undefined,
        _ => {
            return Err(rusqlite::Error::FromSqlConversionFailure(
                0,
                rusqlite::types::Type::Text,
                Box::new(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "unknown native completion status",
                )),
            ))
        }
    })
}

pub fn upsert_installation_row(
    conn: &Connection,
    installation: &Installation,
) -> Result<(), StoreError> {
    conn.execute(
        "INSERT INTO installation (id, name, default_line_id, multicast_address, completion)
         VALUES (?1, ?2, ?3, ?4, ?5)
         ON CONFLICT(id) DO UPDATE SET
             name = excluded.name,
             default_line_id = excluded.default_line_id,
             multicast_address = excluded.multicast_address,
             completion = excluded.completion",
        params![
            installation.id.0,
            installation.name,
            installation.default_line.map(|l| l.0),
            installation.multicast_address.map(|a| a.to_string()),
            completion_to_str(installation.completion),
        ],
    )?;
    Ok(())
}

pub fn upsert_area(
    conn: &Connection,
    installation_id: InstallationId,
    position: i64,
    area: &Area,
) -> Result<(), StoreError> {
    conn.execute(
        "INSERT INTO area
             (id, installation_id, position, source_path, source_ets_id, name, address, completion)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
         ON CONFLICT(id) DO UPDATE SET
             installation_id = excluded.installation_id,
             position = excluded.position,
             source_path = excluded.source_path,
             source_ets_id = excluded.source_ets_id,
             name = excluded.name,
             address = excluded.address,
             completion = excluded.completion",
        params![
            area.id.0,
            installation_id.0,
            position,
            area.source.path,
            area.source.ets_id,
            area.name,
            area.address,
            completion_to_str(area.completion),
        ],
    )?;
    Ok(())
}

pub fn upsert_line(
    conn: &Connection,
    area_id: AreaId,
    position: i64,
    line: &Line,
) -> Result<(), StoreError> {
    conn.execute(
        "INSERT INTO line
             (id, area_id, position, source_path, source_ets_id, name, address, medium_ref,
              domain_address, domain_address_is_checked, ip_routing_multicast_address,
              multicast_ttl, completion)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
         ON CONFLICT(id) DO UPDATE SET
             area_id = excluded.area_id,
             position = excluded.position,
             source_path = excluded.source_path,
             source_ets_id = excluded.source_ets_id,
             name = excluded.name,
             address = excluded.address,
             medium_ref = excluded.medium_ref,
             domain_address = excluded.domain_address,
             domain_address_is_checked = excluded.domain_address_is_checked,
             ip_routing_multicast_address = excluded.ip_routing_multicast_address,
             multicast_ttl = excluded.multicast_ttl,
             completion = excluded.completion",
        params![
            line.id.0,
            area_id.0,
            position,
            line.source.path,
            line.source.ets_id,
            line.name,
            line.address,
            line.medium_ref,
            line.domain_address,
            line.domain_address_is_checked,
            line.ip_routing_multicast_address.map(|a| a.to_string()),
            line.multicast_ttl,
            completion_to_str(line.completion),
        ],
    )?;
    Ok(())
}

pub struct InstallationRow {
    pub id: InstallationId,
    pub name: String,
    pub default_line: Option<LineId>,
    pub multicast_address: Option<Ipv4Addr>,
    pub completion: CompletionStatus,
}

pub fn load_installation_rows(conn: &Connection) -> Result<Vec<InstallationRow>, StoreError> {
    let mut stmt = conn.prepare(
        "SELECT id, name, default_line_id, multicast_address, completion
         FROM installation ORDER BY id",
    )?;
    let rows = stmt.query_map([], |row| {
        let id: u8 = row.get(0)?;
        let name: String = row.get(1)?;
        let default_line: Option<u32> = row.get(2)?;
        let multicast_address: Option<String> = row.get(3)?;
        let completion: String = row.get(4)?;
        Ok((id, name, default_line, multicast_address, completion))
    })?;
    rows.map(|r| {
        let (id, name, default_line, multicast_address, completion) = r?;
        Ok(InstallationRow {
            id: InstallationId(id),
            name,
            default_line: default_line.map(LineId),
            multicast_address: multicast_address
                .map(|a| a.parse().expect("stored multicast address is always valid")),
            completion: completion_from_str(&completion)?,
        })
    })
    .collect()
}

pub fn load_topology(
    conn: &Connection,
    installation_id: InstallationId,
) -> Result<Topology, StoreError> {
    let mut area_stmt = conn.prepare(
        "SELECT id, source_path, source_ets_id, name, address, completion
         FROM area WHERE installation_id = ?1 ORDER BY position",
    )?;
    let areas = area_stmt
        .query_map(params![installation_id.0], |row| {
            let id: u32 = row.get(0)?;
            let source_path: String = row.get(1)?;
            let source_ets_id: String = row.get(2)?;
            let name: String = row.get(3)?;
            let address: u8 = row.get(4)?;
            let completion: String = row.get(5)?;
            Ok((id, source_path, source_ets_id, name, address, completion))
        })?
        .map(|r| {
            let (id, source_path, source_ets_id, name, address, completion) = r?;
            let area_id = AreaId(id);
            let mut line_stmt =
                conn.prepare("SELECT id FROM line WHERE area_id = ?1 ORDER BY position")?;
            let lines = line_stmt
                .query_map(params![area_id.0], |row| Ok(LineId(row.get(0)?)))?
                .collect::<Result<Vec<_>, _>>()?;
            Ok::<_, StoreError>(Area {
                id: area_id,
                source: SourceRef {
                    path: source_path,
                    ets_id: source_ets_id,
                },
                name,
                address,
                completion: completion_from_str(&completion)?,
                lines,
            })
        })
        .collect::<Result<Vec<_>, _>>()?;

    let mut line_stmt = conn.prepare(
        "SELECT l.id, l.source_path, l.source_ets_id, l.name, l.address, l.medium_ref,
                l.domain_address, l.domain_address_is_checked, l.ip_routing_multicast_address,
                l.multicast_ttl, l.completion
         FROM line l JOIN area a ON l.area_id = a.id
         WHERE a.installation_id = ?1 ORDER BY l.model_position, l.id",
    )?;
    let lines = line_stmt
        .query_map(params![installation_id.0], |row| {
            let id: u32 = row.get(0)?;
            let source_path: String = row.get(1)?;
            let source_ets_id: String = row.get(2)?;
            let name: String = row.get(3)?;
            let address: u8 = row.get(4)?;
            let medium_ref: String = row.get(5)?;
            let domain_address: Option<String> = row.get(6)?;
            let domain_address_is_checked: Option<bool> = row.get(7)?;
            let ip_routing_multicast_address: Option<String> = row.get(8)?;
            let multicast_ttl: Option<u8> = row.get(9)?;
            let completion: String = row.get(10)?;
            Ok((
                id,
                source_path,
                source_ets_id,
                name,
                address,
                medium_ref,
                domain_address,
                domain_address_is_checked,
                ip_routing_multicast_address,
                multicast_ttl,
                completion,
            ))
        })?
        .map(|r| {
            let (
                id,
                source_path,
                source_ets_id,
                name,
                address,
                medium_ref,
                domain_address,
                domain_address_is_checked,
                ip_routing_multicast_address,
                multicast_ttl,
                completion,
            ) = r?;
            let line_id = LineId(id);
            let mut dev_stmt = conn
                .prepare("SELECT id FROM device WHERE line_id = ?1 ORDER BY topology_position")?;
            let devices = dev_stmt
                .query_map(params![line_id.0], |row| Ok(DeviceId(row.get(0)?)))?
                .collect::<Result<Vec<_>, _>>()?;
            Ok::<_, StoreError>(Line {
                id: line_id,
                source: SourceRef {
                    path: source_path,
                    ets_id: source_ets_id,
                },
                name,
                address,
                medium_ref,
                domain_address,
                domain_address_is_checked,
                ip_routing_multicast_address: ip_routing_multicast_address
                    .map(|a| a.parse().expect("stored address is always valid")),
                multicast_ttl,
                completion: completion_from_str(&completion)?,
                devices,
            })
        })
        .collect::<Result<Vec<_>, _>>()?;

    let mut unassigned_stmt = conn.prepare(
        "SELECT id FROM device WHERE installation_id = ?1 AND line_id IS NULL
         ORDER BY topology_position",
    )?;
    let unassigned = unassigned_stmt
        .query_map(params![installation_id.0], |row| Ok(DeviceId(row.get(0)?)))?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(Topology {
        areas,
        lines,
        unassigned,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::open_and_migrate_in_memory;
    use knx_core::commissioning::CompletionStatus;

    fn source() -> SourceRef {
        SourceRef {
            path: "0.xml".into(),
            ets_id: "t".into(),
        }
    }

    fn installation() -> Installation {
        Installation {
            id: InstallationId(0),
            name: "Haus".into(),
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

    #[test]
    fn installation_row_round_trips() {
        let conn = open_and_migrate_in_memory().unwrap();
        let i = installation();
        upsert_installation_row(&conn, &i).unwrap();
        let rows = load_installation_rows(&conn).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].id, i.id);
        assert_eq!(rows[0].name, i.name);
        assert_eq!(rows[0].completion, i.completion);
    }

    #[test]
    fn area_and_line_round_trip_with_topology() {
        let conn = open_and_migrate_in_memory().unwrap();
        upsert_installation_row(&conn, &installation()).unwrap();
        let line = Line {
            id: LineId(1),
            source: source(),
            name: "HL1".into(),
            address: 1,
            medium_ref: "TP".into(),
            domain_address: None,
            domain_address_is_checked: None,
            ip_routing_multicast_address: None,
            multicast_ttl: None,
            completion: CompletionStatus::FinishedDesign,
            devices: vec![],
        };
        let area = Area {
            id: AreaId(1),
            source: source(),
            name: "A1".into(),
            address: 1,
            completion: CompletionStatus::FinishedDesign,
            lines: vec![line.id],
        };
        upsert_area(&conn, InstallationId(0), 0, &area).unwrap();
        upsert_line(&conn, area.id, 0, &line).unwrap();

        let topo = load_topology(&conn, InstallationId(0)).unwrap();
        assert_eq!(topo.areas, vec![area]);
        assert_eq!(topo.lines, vec![line]);
        assert_eq!(topo.unassigned, vec![]);
    }
}
