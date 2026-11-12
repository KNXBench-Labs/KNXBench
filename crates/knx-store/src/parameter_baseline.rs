//! Native rows of per-device parameter baselines, kept apart from every model image (ADR-0173).
//!
//! Two tables: one header row per device (program id, settlement time) and
//! one value row per `ParameterRef`. They are file-level data like carried
//! manufacturer files: no `Project` save writes them, no history image or
//! version carries them and no foreign key binds them to `device`, because a
//! baseline may belong to a device the saved root does not hold yet (it was
//! created and downloaded after the last save).

use std::collections::{BTreeMap, BTreeSet};

use chrono::{DateTime, SecondsFormat, Utc};
use knx_core::commissioning::parameter_baseline::{
    BaselineEdit, ParameterBaseline, ParameterBaselines,
};
use knx_core::{DeviceId, Project};
use rusqlite::{params, Connection};

use crate::StoreError;

/// Native schema 20 DDL.
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

/// The two tables, value table first (the order rows must be deleted in).
pub(crate) const TABLES: [&str; 2] = [
    "device_parameter_baseline_value",
    "device_parameter_baseline",
];

fn integrity(reason: impl Into<String>) -> StoreError {
    StoreError::ParameterBaselineIntegrity(reason.into())
}

/// The settlement time as stored: RFC 3339 UTC with every fractional digit,
/// so it reads back as the identical instant.
fn encode_at(at: &DateTime<Utc>) -> String {
    at.to_rfc3339_opts(SecondsFormat::AutoSi, true)
}

fn decode_at(raw: &str) -> Result<DateTime<Utc>, StoreError> {
    DateTime::parse_from_rfc3339(raw)
        .map(|at| at.with_timezone(&Utc))
        .map_err(|_| integrity(format!("unreadable settlement time {raw:?}")))
}

/// Every stored baseline. A value row without its header is refused by the
/// foreign key; a header without values is a program without refs.
pub fn load_all(conn: &Connection) -> Result<ParameterBaselines, StoreError> {
    let mut headers: BTreeMap<DeviceId, ParameterBaseline> = BTreeMap::new();
    let mut stmt = conn.prepare(
        "SELECT device_id, program_id, at FROM device_parameter_baseline ORDER BY device_id",
    )?;
    let mut rows = stmt.query([])?;
    while let Some(row) = rows.next()? {
        let id: i64 = row.get(0)?;
        let device = u32::try_from(id)
            .map(DeviceId)
            .map_err(|_| integrity(format!("device id {id} out of range")))?;
        headers.insert(
            device,
            ParameterBaseline {
                program_id: row.get(1)?,
                values: BTreeMap::new(),
                at: decode_at(&row.get::<_, String>(2)?)?,
            },
        );
    }
    let mut stmt = conn.prepare(
        "SELECT device_id, ref_id, raw FROM device_parameter_baseline_value
         ORDER BY device_id, ref_id",
    )?;
    let mut rows = stmt.query([])?;
    while let Some(row) = rows.next()? {
        let id: i64 = row.get(0)?;
        let baseline = u32::try_from(id)
            .ok()
            .and_then(|id| headers.get_mut(&DeviceId(id)))
            .ok_or_else(|| integrity(format!("value row for device {id} without a baseline")))?;
        baseline.values.insert(row.get(1)?, row.get(2)?);
    }
    Ok(headers.into_iter().collect())
}

fn delete_device(conn: &Connection, device: DeviceId) -> Result<(), StoreError> {
    conn.execute(
        "DELETE FROM device_parameter_baseline_value WHERE device_id = ?1",
        [device.0],
    )?;
    conn.execute(
        "DELETE FROM device_parameter_baseline WHERE device_id = ?1",
        [device.0],
    )?;
    Ok(())
}

fn insert(
    conn: &Connection,
    device: DeviceId,
    baseline: &ParameterBaseline,
) -> Result<(), StoreError> {
    conn.execute(
        "INSERT INTO device_parameter_baseline (device_id, program_id, at) VALUES (?1, ?2, ?3)",
        params![device.0, baseline.program_id, encode_at(&baseline.at)],
    )?;
    let mut stmt = conn.prepare_cached(
        "INSERT INTO device_parameter_baseline_value (device_id, ref_id, raw) VALUES (?1, ?2, ?3)",
    )?;
    for (ref_id, raw) in &baseline.values {
        stmt.execute(params![device.0, ref_id, raw])?;
    }
    Ok(())
}

/// Applies `edits` in order inside the caller's transaction.
pub(crate) fn apply_edits(conn: &Connection, edits: &[BaselineEdit]) -> Result<(), StoreError> {
    for edit in edits {
        delete_device(conn, edit.device)?;
        if let Some(baseline) = &edit.baseline {
            insert(conn, edit.device, baseline)?;
        }
    }
    Ok(())
}

/// Replaces every stored baseline by those of `baselines` whose device
/// `project` contains, inside the caller's transaction (a native save).
pub(crate) fn replace_all(
    conn: &Connection,
    baselines: &ParameterBaselines,
    project: &Project,
) -> Result<(), StoreError> {
    for table in TABLES {
        conn.execute(&format!("DELETE FROM {table}"), [])?;
    }
    for (device, baseline) in baselines.iter() {
        if project.devices.get(device).is_some() {
            insert(conn, device, baseline)?;
        }
    }
    Ok(())
}

/// What a save that brings no baselines of its own does with the stored
/// ones: they stay for devices of `project`, and are dropped for every other
/// device, or entirely when the file held another project
/// (`previous_project_id`), whose device ids name different devices.
pub(crate) fn prune(
    conn: &Connection,
    previous_project_id: Option<&str>,
    project: &Project,
) -> Result<(), StoreError> {
    let stored: Vec<i64> = conn
        .prepare("SELECT device_id FROM device_parameter_baseline")?
        .query_map([], |row| row.get(0))?
        .collect::<Result<_, _>>()?;
    if stored.is_empty() {
        return Ok(());
    }
    let same_project = previous_project_id == Some(project.info.project_id.as_str());
    let present: BTreeSet<u32> = project.devices.iter().map(|d| d.id.0).collect();
    for id in stored {
        let keep = same_project && u32::try_from(id).is_ok_and(|id| present.contains(&id));
        if !keep {
            conn.execute(
                "DELETE FROM device_parameter_baseline_value WHERE device_id = ?1",
                [id],
            )?;
            conn.execute(
                "DELETE FROM device_parameter_baseline WHERE device_id = ?1",
                [id],
            )?;
        }
    }
    Ok(())
}
