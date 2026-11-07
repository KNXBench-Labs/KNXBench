//! Native rows for the user's object priority edits, apart from captured source facts.

use crate::StoreError;
use knx_core::{ComObjectInstanceId, ObjectPriority};
use rusqlite::{params, Connection};
use std::collections::BTreeMap;

pub(crate) const CREATE_TABLE: &str = "CREATE TABLE com_object_priority_edit (
    com_object_instance_id INTEGER PRIMARY KEY REFERENCES com_object_instance(id),
    priority TEXT NOT NULL CHECK(priority IN ('Low','High','Alert'))
) STRICT;";

/// Writes `Some` as one row and deletes the row for `None`.
pub(crate) fn write(
    conn: &Connection,
    id: ComObjectInstanceId,
    value: Option<ObjectPriority>,
) -> Result<(), StoreError> {
    match value {
        None => conn.execute(
            "DELETE FROM com_object_priority_edit WHERE com_object_instance_id=?1",
            [id.0],
        )?,
        Some(priority) => conn.execute(
            "INSERT INTO com_object_priority_edit (com_object_instance_id, priority)
             VALUES (?1, ?2)
             ON CONFLICT(com_object_instance_id) DO UPDATE SET priority=excluded.priority",
            params![id.0, priority.as_str()],
        )?,
    };
    Ok(())
}

fn decode(raw: &str) -> Result<ObjectPriority, StoreError> {
    ObjectPriority::parse(raw).ok_or(StoreError::PriorityEditIntegrity)
}

pub(crate) fn load(
    conn: &Connection,
    id: ComObjectInstanceId,
) -> Result<Option<ObjectPriority>, StoreError> {
    let mut stmt = conn
        .prepare("SELECT priority FROM com_object_priority_edit WHERE com_object_instance_id=?1")?;
    let mut rows = stmt.query([id.0])?;
    match rows.next()? {
        Some(row) => Ok(Some(decode(&row.get::<_, String>(0)?)?)),
        None => Ok(None),
    }
}

pub(crate) fn load_all(
    conn: &Connection,
) -> Result<BTreeMap<ComObjectInstanceId, ObjectPriority>, StoreError> {
    let mut stmt = conn.prepare(
        "SELECT com_object_instance_id, priority FROM com_object_priority_edit
         ORDER BY com_object_instance_id",
    )?;
    let mut rows = stmt.query([])?;
    let mut out = BTreeMap::new();
    while let Some(row) = rows.next()? {
        let id = ComObjectInstanceId(row.get(0)?);
        out.insert(id, decode(&row.get::<_, String>(1)?)?);
    }
    Ok(out)
}
