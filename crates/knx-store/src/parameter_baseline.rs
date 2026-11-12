//! Persists independent parameter, object-flag and link evidence outside model history.

use crate::StoreError;
use chrono::{DateTime, SecondsFormat, Utc};
use knx_core::commissioning::parameter_baseline::{
    same_device, BaselineChange, BaselineEdit, BaselineLink, LinkBaseline, LinkBaselineChange,
    ParameterBaseline, ParameterBaselines, WrittenObjectFlags,
};
use knx_core::{DeviceId, GroupAddress, Project};
use rusqlite::{params, Connection};

/// Authentic schema20 DDL, used only by its original migration.
pub(crate) const CREATE_TABLES: &str = "CREATE TABLE device_parameter_baseline (
    device_id INTEGER PRIMARY KEY CHECK(device_id BETWEEN 0 AND 4294967295),
    program_id TEXT NOT NULL CHECK(length(program_id) > 0),
    at TEXT NOT NULL CHECK(length(at) > 0)
) STRICT;
CREATE TABLE device_parameter_baseline_value (
    device_id INTEGER NOT NULL REFERENCES device_parameter_baseline(device_id),
    ref_id TEXT NOT NULL,
    raw TEXT NOT NULL,
    PRIMARY KEY(device_id, ref_id)
) STRICT, WITHOUT ROWID;";

pub(crate) const EXTEND_TABLES: &str = "
ALTER TABLE device_parameter_baseline ADD COLUMN object_flags_known INTEGER NOT NULL DEFAULT 0 CHECK(object_flags_known IN (0,1));
CREATE TABLE device_object_flags_baseline (
    device_id INTEGER NOT NULL REFERENCES device_parameter_baseline(device_id),
    object_number INTEGER NOT NULL CHECK(object_number BETWEEN 0 AND 255),
    read INTEGER NOT NULL CHECK(read IN (0,1)),
    write INTEGER NOT NULL CHECK(write IN (0,1)),
    transmit INTEGER NOT NULL CHECK(transmit IN (0,1)),
    update_flag INTEGER NOT NULL CHECK(update_flag IN (0,1)),
    communication INTEGER NOT NULL CHECK(communication IN (0,1)),
    read_on_init INTEGER CHECK(read_on_init IN (0,1)),
    PRIMARY KEY(device_id, object_number)
) STRICT, WITHOUT ROWID;
CREATE TABLE device_link_baseline (
    device_id INTEGER PRIMARY KEY CHECK(device_id BETWEEN 0 AND 4294967295),
    program_id TEXT NOT NULL CHECK(length(program_id) > 0),
    at TEXT NOT NULL CHECK(length(at) > 0)
) STRICT;
CREATE TABLE device_link_baseline_link (
    device_id INTEGER NOT NULL REFERENCES device_link_baseline(device_id),
    object_number INTEGER NOT NULL CHECK(object_number BETWEEN 0 AND 255),
    group_address INTEGER NOT NULL CHECK(group_address BETWEEN 0 AND 65535),
    sending INTEGER NOT NULL CHECK(sending IN (0,1)),
    PRIMARY KEY(device_id, object_number, group_address)
) STRICT, WITHOUT ROWID;";

/// File-level tables, children before headers; no foreign key to the saved root device.
pub(crate) const TABLES: [&str; 5] = [
    "device_object_flags_baseline",
    "device_parameter_baseline_value",
    "device_parameter_baseline",
    "device_link_baseline_link",
    "device_link_baseline",
];
pub(crate) const TABLES_V20: [&str; 2] = [
    "device_parameter_baseline_value",
    "device_parameter_baseline",
];

fn integrity(reason: impl Into<String>) -> StoreError {
    StoreError::ParameterBaselineIntegrity(reason.into())
}
fn decode_at(raw: &str) -> Result<DateTime<Utc>, StoreError> {
    DateTime::parse_from_rfc3339(raw)
        .map(|at| at.with_timezone(&Utc))
        .map_err(|_| integrity(format!("unreadable settlement time {raw:?}")))
}
fn program(raw: String) -> Result<String, StoreError> {
    if raw.is_empty() {
        Err(integrity("empty program id"))
    } else {
        Ok(raw)
    }
}
fn device(raw: i64) -> Result<DeviceId, StoreError> {
    u32::try_from(raw)
        .map(DeviceId)
        .map_err(|_| integrity("device id out of range"))
}
fn boolean(raw: i64) -> Result<bool, StoreError> {
    match raw {
        0 => Ok(false),
        1 => Ok(true),
        _ => Err(integrity("invalid boolean")),
    }
}
fn number(raw: i64) -> Result<u8, StoreError> {
    u8::try_from(raw).map_err(|_| integrity("object number out of range"))
}

/// Loads and validates every section, including orphan and contradictory child rows.
pub fn load_all(conn: &Connection) -> Result<ParameterBaselines, StoreError> {
    let mut all = ParameterBaselines::new();
    let mut stmt = conn.prepare("SELECT device_id, program_id, at, object_flags_known FROM device_parameter_baseline ORDER BY device_id")?;
    let mut rows = stmt.query([])?;
    while let Some(row) = rows.next()? {
        all.apply(&BaselineEdit {
            device: device(row.get(0)?)?,
            parameter: BaselineChange::Record(ParameterBaseline {
                program_id: program(row.get(1)?)?,
                values: Default::default(),
                at: decode_at(&row.get::<_, String>(2)?)?,
                object_flags: boolean(row.get(3)?)?.then(Default::default),
            }),
            links: LinkBaselineChange::Keep,
        });
    }
    // Collect mutable parameter records without exposing mutation of baseline ownership.
    let mut parameters: std::collections::BTreeMap<_, _> =
        all.iter().map(|(id, b)| (id, b.clone())).collect();
    let mut stmt = conn.prepare("SELECT device_id, ref_id, raw FROM device_parameter_baseline_value ORDER BY device_id, ref_id")?;
    let mut rows = stmt.query([])?;
    while let Some(row) = rows.next()? {
        parameters
            .get_mut(&device(row.get(0)?)?)
            .ok_or_else(|| integrity("orphan parameter value"))?
            .values
            .insert(row.get(1)?, row.get(2)?);
    }
    let mut stmt = conn.prepare("SELECT device_id, object_number, read, write, transmit, update_flag, communication, read_on_init FROM device_object_flags_baseline ORDER BY device_id, object_number")?;
    let mut rows = stmt.query([])?;
    while let Some(row) = rows.next()? {
        let flags = parameters
            .get_mut(&device(row.get(0)?)?)
            .and_then(|b| b.object_flags.as_mut())
            .ok_or_else(|| integrity("flag row without known flags header"))?;
        flags.insert(
            number(row.get(1)?)?,
            WrittenObjectFlags {
                read: boolean(row.get(2)?)?,
                write: boolean(row.get(3)?)?,
                transmit: boolean(row.get(4)?)?,
                update: boolean(row.get(5)?)?,
                communication: boolean(row.get(6)?)?,
                read_on_init: row.get::<_, Option<i64>>(7)?.map(boolean).transpose()?,
            },
        );
    }
    all = parameters.into_iter().collect();
    let mut links: std::collections::BTreeMap<_, LinkBaseline> = Default::default();
    let mut stmt = conn
        .prepare("SELECT device_id, program_id, at FROM device_link_baseline ORDER BY device_id")?;
    let mut rows = stmt.query([])?;
    while let Some(row) = rows.next()? {
        links.insert(
            device(row.get(0)?)?,
            LinkBaseline {
                program_id: program(row.get(1)?)?,
                at: decode_at(&row.get::<_, String>(2)?)?,
                links: Vec::new(),
            },
        );
    }
    let mut stmt = conn.prepare("SELECT device_id, object_number, group_address, sending FROM device_link_baseline_link ORDER BY device_id, object_number, sending DESC, group_address")?;
    let mut rows = stmt.query([])?;
    while let Some(row) = rows.next()? {
        let baseline = links
            .get_mut(&device(row.get(0)?)?)
            .ok_or_else(|| integrity("orphan link row"))?;
        let address: i64 = row.get(2)?;
        baseline.links.push(BaselineLink {
            object: number(row.get(1)?)?,
            group_address: GroupAddress::from_raw(
                u16::try_from(address).map_err(|_| integrity("group address out of range"))?,
            ),
            sending: boolean(row.get(3)?)?,
        });
    }
    for (device, baseline) in links {
        all.apply(&BaselineEdit {
            device,
            parameter: BaselineChange::Keep,
            links: LinkBaselineChange::Record(baseline),
        });
    }
    Ok(all)
}

/// A single-device read retains the full loader's integrity checks.
pub fn load_one(
    conn: &Connection,
    device: DeviceId,
) -> Result<(Option<ParameterBaseline>, Option<LinkBaseline>), StoreError> {
    let all = load_all(conn)?;
    Ok((all.get(device).cloned(), all.get_links(device).cloned()))
}

fn delete_parameters(conn: &Connection, device: DeviceId) -> Result<(), StoreError> {
    for table in &TABLES[..3] {
        conn.execute(
            &format!("DELETE FROM {table} WHERE device_id = ?1"),
            [device.0],
        )?;
    }
    Ok(())
}
fn delete_links(conn: &Connection, device: DeviceId) -> Result<(), StoreError> {
    for table in &TABLES[3..] {
        conn.execute(
            &format!("DELETE FROM {table} WHERE device_id = ?1"),
            [device.0],
        )?;
    }
    Ok(())
}
fn insert_parameters(
    conn: &Connection,
    device: DeviceId,
    baseline: &ParameterBaseline,
) -> Result<(), StoreError> {
    conn.execute("INSERT INTO device_parameter_baseline (device_id, program_id, at, object_flags_known) VALUES (?1, ?2, ?3, ?4)",
        params![device.0, baseline.program_id, baseline.at.to_rfc3339_opts(SecondsFormat::AutoSi, true), baseline.object_flags.is_some()])?;
    for (reference, raw) in &baseline.values {
        conn.execute(
            "INSERT INTO device_parameter_baseline_value VALUES (?1, ?2, ?3)",
            params![device.0, reference, raw],
        )?;
    }
    if let Some(flags) = &baseline.object_flags {
        for (number, flags) in flags {
            conn.execute(
                "INSERT INTO device_object_flags_baseline VALUES (?1,?2,?3,?4,?5,?6,?7,?8)",
                params![
                    device.0,
                    number,
                    flags.read,
                    flags.write,
                    flags.transmit,
                    flags.update,
                    flags.communication,
                    flags.read_on_init
                ],
            )?;
        }
    }
    Ok(())
}
fn insert_links(
    conn: &Connection,
    device: DeviceId,
    baseline: &LinkBaseline,
) -> Result<(), StoreError> {
    conn.execute(
        "INSERT INTO device_link_baseline VALUES (?1,?2,?3)",
        params![
            device.0,
            baseline.program_id,
            baseline.at.to_rfc3339_opts(SecondsFormat::AutoSi, true)
        ],
    )?;
    for link in &baseline.links {
        conn.execute(
            "INSERT INTO device_link_baseline_link VALUES (?1,?2,?3,?4)",
            params![
                device.0,
                link.object,
                link.group_address.raw(),
                link.sending
            ],
        )?;
    }
    Ok(())
}

/// Applies independent edits inside the caller's transaction; Keep issues no row write.
pub(crate) fn apply_edits(conn: &Connection, edits: &[BaselineEdit]) -> Result<(), StoreError> {
    for edit in edits {
        match &edit.parameter {
            BaselineChange::Keep => {}
            BaselineChange::Remove => delete_parameters(conn, edit.device)?,
            BaselineChange::Record(baseline) => {
                delete_parameters(conn, edit.device)?;
                insert_parameters(conn, edit.device, baseline)?;
            }
        }
        match &edit.links {
            LinkBaselineChange::Keep => {}
            LinkBaselineChange::Remove => delete_links(conn, edit.device)?,
            LinkBaselineChange::Record(baseline) => {
                delete_links(conn, edit.device)?;
                insert_links(conn, edit.device, baseline)?;
            }
        }
    }
    Ok(())
}

/// Save As copies both sections, including link-only owners in a dirty working state.
pub(crate) fn replace_all(
    conn: &Connection,
    baselines: &ParameterBaselines,
    project: &Project,
) -> Result<(), StoreError> {
    for table in TABLES {
        conn.execute(&format!("DELETE FROM {table}"), [])?;
    }
    for id in baselines.device_ids() {
        if project.devices.get(id).is_none() {
            continue;
        }
        if let Some(b) = baselines.get(id) {
            insert_parameters(conn, id, b)?;
        }
        if let Some(b) = baselines.get_links(id) {
            insert_links(conn, id, b)?;
        }
    }
    Ok(())
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum PruneBasis<'a> {
    Continued {
        previous_project_id: Option<&'a str>,
    },
    Replaced {
        described: Option<&'a Project>,
    },
}

/// Prunes the union of section owners by continuation or exact device identity.
pub(crate) fn prune(
    conn: &Connection,
    basis: PruneBasis<'_>,
    project: &Project,
) -> Result<(), StoreError> {
    let stored = conn.prepare("SELECT device_id FROM device_parameter_baseline UNION SELECT device_id FROM device_link_baseline")?
        .query_map([], |row| row.get::<_, i64>(0))?.collect::<Result<Vec<_>, _>>()?;
    for raw in stored {
        let id = device(raw)?;
        let keep = project.devices.get(id).is_some_and(|device| match basis {
            PruneBasis::Continued {
                previous_project_id,
            } => previous_project_id == Some(project.info.project_id.as_str()),
            PruneBasis::Replaced { described } => described.is_some_and(|before| {
                before.info.project_id == project.info.project_id
                    && before
                        .devices
                        .get(id)
                        .is_some_and(|old| same_device(old, device))
            }),
        });
        if !keep {
            delete_parameters(conn, id)?;
            delete_links(conn, id)?;
        }
    }
    Ok(())
}
