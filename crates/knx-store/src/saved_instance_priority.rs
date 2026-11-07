//! Strict shared native codec for immutable captured saved-instance priority facts.

use crate::StoreError;
use knx_core::{
    ComObjectInstanceId, SavedInstancePriority, SavedPriorityAbsent, SavedPriorityDeclared,
    SavedPriorityOmission, SavedPrioritySource, SavedPriorityUnavailable, SourceRef,
};
use rusqlite::{params, Connection, Row};
use std::collections::BTreeMap;

pub(crate) const CREATE_TABLE: &str = "CREATE TABLE com_object_saved_priority (
    com_object_instance_id INTEGER PRIMARY KEY REFERENCES com_object_instance(id),
    state TEXT NOT NULL,
    raw TEXT,
    reason TEXT,
    omission TEXT,
    source_schema INTEGER,
    device_source_path TEXT,
    device_source_ets_id TEXT,
    object_source_path TEXT,
    object_source_ets_id TEXT,
    owner_xpath TEXT,
    topology_sha256 TEXT,
    CHECK ((
      (state = 'unavailable' AND reason IN ('UnsupportedSourceSchema','ModuleScope',
        'AmbiguousOwner','AmbiguousDeclaration','SourceIntegrity','ResourceLimit')
        AND raw IS NULL AND omission IS NULL AND source_schema IS NULL
        AND device_source_path IS NULL AND device_source_ets_id IS NULL
        AND object_source_path IS NULL AND object_source_ets_id IS NULL
        AND owner_xpath IS NULL AND topology_sha256 IS NULL)
      OR
      (state IN ('declared','absent') AND reason IS NULL
        AND ((state = 'declared' AND raw IS NOT NULL
              AND length(CAST(raw AS BLOB)) <= 4096 AND omission IS NULL)
          OR (state = 'absent' AND raw IS NULL
              AND omission IN ('AttributeOmitted','InstanceElementOmitted')))
        AND source_schema = 23 AND device_source_path = object_source_path
        AND length(CAST(device_source_path AS BLOB)) BETWEEN 1 AND 4096
        AND length(CAST(device_source_ets_id AS BLOB)) BETWEEN 1 AND 4096
        AND length(CAST(object_source_path AS BLOB)) BETWEEN 1 AND 4096
        AND length(CAST(object_source_ets_id AS BLOB)) BETWEEN 1 AND 4096
        AND length(CAST(owner_xpath AS BLOB)) BETWEEN 1 AND 4096
        AND length(CAST(topology_sha256 AS BLOB)) = 64 AND length(topology_sha256) = 64
        AND topology_sha256 NOT GLOB '*[^0-9a-f]*')
    ) IS TRUE)
) STRICT;";

const COLUMNS: &str = "com_object_instance_id,state,raw,reason,omission,source_schema,device_source_path,device_source_ets_id,object_source_path,object_source_ets_id,owner_xpath,topology_sha256";

fn invalid() -> StoreError {
    StoreError::SavedPriorityIntegrity
}

pub(crate) fn validate(value: &SavedInstancePriority) -> Result<(), StoreError> {
    if value.is_valid() {
        Ok(())
    } else {
        Err(invalid())
    }
}

pub(crate) fn check(project: &knx_core::Project) -> Result<(), StoreError> {
    for com in project.devices.com_objects() {
        validate(&com.saved_instance_priority)?;
    }
    Ok(())
}

fn reason_token(reason: SavedPriorityUnavailable) -> &'static str {
    match reason {
        SavedPriorityUnavailable::NotCaptured => "NotCaptured",
        SavedPriorityUnavailable::UnsupportedSourceSchema => "UnsupportedSourceSchema",
        SavedPriorityUnavailable::ModuleScope => "ModuleScope",
        SavedPriorityUnavailable::AmbiguousOwner => "AmbiguousOwner",
        SavedPriorityUnavailable::AmbiguousDeclaration => "AmbiguousDeclaration",
        SavedPriorityUnavailable::SourceIntegrity => "SourceIntegrity",
        SavedPriorityUnavailable::ResourceLimit => "ResourceLimit",
    }
}

fn omission_token(omission: SavedPriorityOmission) -> &'static str {
    match omission {
        SavedPriorityOmission::AttributeOmitted => "AttributeOmitted",
        SavedPriorityOmission::InstanceElementOmitted => "InstanceElementOmitted",
    }
}

pub(crate) fn write(
    conn: &Connection,
    id: ComObjectInstanceId,
    value: &SavedInstancePriority,
) -> Result<(), StoreError> {
    validate(value)?;
    if *value == SavedInstancePriority::default() {
        conn.execute(
            "DELETE FROM com_object_saved_priority WHERE com_object_instance_id=?1",
            [id.0],
        )?;
        return Ok(());
    }
    let (state, raw, reason, omission, source) = match value {
        SavedInstancePriority::Unavailable { reason } => {
            ("unavailable", None, Some(reason_token(*reason)), None, None)
        }
        SavedInstancePriority::Absent(value) => (
            "absent",
            None,
            None,
            Some(omission_token(value.omission)),
            Some(&value.source),
        ),
        SavedInstancePriority::Declared(value) => (
            "declared",
            Some(value.raw.as_str()),
            None,
            None,
            Some(&value.source),
        ),
    };
    conn.execute(&format!("INSERT INTO com_object_saved_priority ({COLUMNS})
        VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12)
        ON CONFLICT(com_object_instance_id) DO UPDATE SET state=excluded.state,raw=excluded.raw,
        reason=excluded.reason,omission=excluded.omission,source_schema=excluded.source_schema,
        device_source_path=excluded.device_source_path,device_source_ets_id=excluded.device_source_ets_id,
        object_source_path=excluded.object_source_path,object_source_ets_id=excluded.object_source_ets_id,
        owner_xpath=excluded.owner_xpath,topology_sha256=excluded.topology_sha256"), params![
        id.0,state,raw,reason,omission,source.map(|s|s.source_schema),
        source.map(|s|s.device_source.path.as_str()),source.map(|s|s.device_source.ets_id.as_str()),
        source.map(|s|s.object_source.path.as_str()),source.map(|s|s.object_source.ets_id.as_str()),
        source.map(|s|s.owner_xpath.as_str()),source.map(|s|s.topology_sha256.as_str())])?;
    Ok(())
}

fn decode(row: &Row<'_>) -> Result<(ComObjectInstanceId, SavedInstancePriority), StoreError> {
    let id = ComObjectInstanceId(row.get(0)?);
    let state: String = row.get(1)?;
    let raw: Option<String> = row.get(2)?;
    let reason: Option<String> = row.get(3)?;
    let omission: Option<String> = row.get(4)?;
    let schema: Option<u32> = row.get(5)?;
    let dp: Option<String> = row.get(6)?;
    let di: Option<String> = row.get(7)?;
    let op: Option<String> = row.get(8)?;
    let oi: Option<String> = row.get(9)?;
    let xpath: Option<String> = row.get(10)?;
    let hash: Option<String> = row.get(11)?;
    let value = if state == "unavailable" {
        if raw.is_some()
            || omission.is_some()
            || schema.is_some()
            || dp.is_some()
            || di.is_some()
            || op.is_some()
            || oi.is_some()
            || xpath.is_some()
            || hash.is_some()
        {
            return Err(invalid());
        }
        let reason = match reason.as_deref() {
            Some("UnsupportedSourceSchema") => SavedPriorityUnavailable::UnsupportedSourceSchema,
            Some("ModuleScope") => SavedPriorityUnavailable::ModuleScope,
            Some("AmbiguousOwner") => SavedPriorityUnavailable::AmbiguousOwner,
            Some("AmbiguousDeclaration") => SavedPriorityUnavailable::AmbiguousDeclaration,
            Some("SourceIntegrity") => SavedPriorityUnavailable::SourceIntegrity,
            Some("ResourceLimit") => SavedPriorityUnavailable::ResourceLimit,
            _ => return Err(invalid()),
        };
        SavedInstancePriority::Unavailable { reason }
    } else {
        if reason.is_some() {
            return Err(invalid());
        }
        let source = SavedPrioritySource {
            source_schema: schema.ok_or_else(invalid)?,
            device_source: SourceRef {
                path: dp.ok_or_else(invalid)?,
                ets_id: di.ok_or_else(invalid)?,
            },
            object_source: SourceRef {
                path: op.ok_or_else(invalid)?,
                ets_id: oi.ok_or_else(invalid)?,
            },
            owner_xpath: xpath.ok_or_else(invalid)?,
            topology_sha256: hash.ok_or_else(invalid)?,
        };
        match state.as_str() {
            "declared" if omission.is_none() => {
                SavedInstancePriority::Declared(Box::new(SavedPriorityDeclared {
                    raw: raw.ok_or_else(invalid)?,
                    source,
                }))
            }
            "absent" if raw.is_none() => {
                let omission = match omission.as_deref() {
                    Some("AttributeOmitted") => SavedPriorityOmission::AttributeOmitted,
                    Some("InstanceElementOmitted") => SavedPriorityOmission::InstanceElementOmitted,
                    _ => return Err(invalid()),
                };
                SavedInstancePriority::Absent(Box::new(SavedPriorityAbsent { omission, source }))
            }
            _ => return Err(invalid()),
        }
    };
    validate(&value)?;
    Ok((id, value))
}

pub(crate) fn load(
    conn: &Connection,
    id: ComObjectInstanceId,
) -> Result<SavedInstancePriority, StoreError> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {COLUMNS} FROM com_object_saved_priority WHERE com_object_instance_id=?1"
    ))?;
    let mut rows = stmt.query([id.0])?;
    rows.next()?
        .map(decode)
        .transpose()
        .map(|v| v.map(|(_, v)| v).unwrap_or_default())
}

pub(crate) fn load_all(
    conn: &Connection,
) -> Result<BTreeMap<ComObjectInstanceId, SavedInstancePriority>, StoreError> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {COLUMNS} FROM com_object_saved_priority ORDER BY com_object_instance_id"
    ))?;
    let mut rows = stmt.query([])?;
    let mut out = BTreeMap::new();
    while let Some(row) = rows.next()? {
        let (id, value) = decode(row)?;
        if out.insert(id, value).is_some() {
            return Err(invalid());
        }
    }
    Ok(out)
}
