//! Versioned normalized instruction overrides; original text is stored elsewhere.
use crate::StoreError;
use knx_core::instruction::{
    InstructionDocument, InstructionRun, InstructionStyle, MAX_DOCUMENT_RUNS,
};
use knx_core::{DeviceId, Project};
use rusqlite::{params, Connection};
use std::collections::BTreeMap;

fn invalid(reason: &'static str) -> StoreError {
    StoreError::Sqlite(rusqlite::Error::FromSqlConversionFailure(
        0,
        rusqlite::types::Type::Text,
        Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, reason)),
    ))
}
pub(crate) fn check(project: &Project) -> Result<(), StoreError> {
    if project
        .instructions
        .keys()
        .any(|id| project.devices.get(*id).is_none())
    {
        return Err(invalid("instruction override references an absent device"));
    }
    Ok(())
}
pub(crate) fn write(conn: &Connection, project: &Project) -> Result<(), StoreError> {
    check(project)?;
    let mut doc = conn.prepare("INSERT INTO instruction_document (device_id) VALUES (?1)")?;
    let mut run = conn.prepare(
        "INSERT INTO instruction_run (device_id, position, text, style) VALUES (?1, ?2, ?3, ?4)",
    )?;
    for (id, value) in &project.instructions {
        doc.execute([id.0])?;
        for (position, part) in value.runs().iter().enumerate() {
            let style = i64::from(part.style.bold)
                | (i64::from(part.style.italic) << 1)
                | (i64::from(part.style.underline) << 2);
            run.execute(params![id.0, position as i64, part.text, style])?;
        }
    }
    Ok(())
}
pub(crate) fn load(
    conn: &Connection,
    project_devices: &knx_core::Devices,
) -> Result<BTreeMap<DeviceId, InstructionDocument>, StoreError> {
    let mut values = BTreeMap::new();
    let mut ids = conn.prepare("SELECT device_id FROM instruction_document ORDER BY device_id")?;
    let mut rows = ids.query([])?;
    while let Some(row) = rows.next()? {
        let raw: i64 = row.get(0)?;
        let id = DeviceId(
            u32::try_from(raw).map_err(|_| invalid("invalid instruction device identity"))?,
        );
        if project_devices.get(id).is_none() {
            return Err(invalid("instruction document has no device"));
        }
        let mut statement = conn.prepare("SELECT position, text, style FROM instruction_run WHERE device_id = ?1 ORDER BY position")?;
        let mut parts = statement.query([raw])?;
        let mut runs = Vec::new();
        while let Some(part) = parts.next()? {
            let position: i64 = part.get(0)?;
            if runs.len() >= MAX_DOCUMENT_RUNS || position != runs.len() as i64 {
                return Err(invalid(
                    "instruction positions are not contiguous or exceed limit",
                ));
            }
            let text: String = part.get(1)?;
            let style: i64 = part.get(2)?;
            if !(0..=7).contains(&style) {
                return Err(invalid("unknown instruction style"));
            }
            runs.push(InstructionRun {
                text,
                style: InstructionStyle {
                    bold: style & 1 != 0,
                    italic: style & 2 != 0,
                    underline: style & 4 != 0,
                },
            });
        }
        let document = InstructionDocument::new(runs)
            .map_err(|_| invalid("invalid or oversized instruction document"))?;
        if values.insert(id, document).is_some() {
            return Err(invalid("duplicate instruction document"));
        }
    }
    let dangling: bool = conn.query_row("SELECT EXISTS(SELECT 1 FROM instruction_run r LEFT JOIN instruction_document d ON r.device_id=d.device_id WHERE d.device_id IS NULL)", [], |row| row.get(0))?;
    if dangling {
        return Err(invalid("instruction run has no document"));
    }
    Ok(values)
}
