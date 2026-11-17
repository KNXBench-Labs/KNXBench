//! Persists per-device configuration profiles (ADR-0175 decision 5).
//!
//! One row per device, keyed by `device_id`; the write side refuses a second
//! profile for a device the same way the domain model does, and the read
//! side refuses duplicate or over-budget rows instead of guessing.

use knx_core::devconf::{
    DeviceConfigProfile, ProfileError, MAX_MODULE_ID_BYTES, MAX_MODULE_SHA256_BYTES,
    MAX_MODULE_VERSION_BYTES, MAX_PROFILE_DOCUMENT_BYTES,
};
use knx_core::ids::DeviceId;
use rusqlite::{params, Connection};

use crate::StoreError;

fn invalid(reason: impl Into<String>) -> StoreError {
    StoreError::ProfileIntegrity(reason.into())
}

pub(crate) fn check(project: &knx_core::Project) -> Result<(), StoreError> {
    for profile in project.devices.config_profiles() {
        DeviceConfigProfile::new(
            profile.device,
            &profile.module_id,
            &profile.module_version,
            &profile.module_sha256,
            profile.document.clone(),
        )
        .map_err(|e| invalid(e.to_string()))?;
        if project.devices.get(profile.device).is_none() {
            return Err(invalid(format!(
                "profile names device {} which does not exist",
                profile.device.0
            )));
        }
    }
    Ok(())
}

pub(crate) fn write(conn: &Connection, project: &knx_core::Project) -> Result<(), StoreError> {
    check(project)?;
    let mut stmt = conn.prepare(
        "INSERT INTO device_config_profile (device_id, module_id, module_version, module_sha256, document)
         VALUES (?1, ?2, ?3, ?4, ?5)",
    )?;
    for profile in project.devices.config_profiles() {
        stmt.execute(params![
            profile.device.0,
            profile.module_id,
            profile.module_version,
            profile.module_sha256,
            profile.document,
        ])?;
    }
    Ok(())
}

pub(crate) fn load(conn: &Connection, project: &mut knx_core::Project) -> Result<(), StoreError> {
    let mut stmt = conn.prepare(
        "SELECT device_id, module_id, module_version, module_sha256, document FROM device_config_profile ORDER BY device_id",
    )?;
    let mut rows = stmt.query([])?;
    while let Some(row) = rows.next()? {
        let device = DeviceId(row.get::<_, i64>(0)? as u32);
        let module_id: String = row.get(1)?;
        let module_version: String = row.get(2)?;
        let module_sha256: String = row.get(3)?;
        let document: Vec<u8> = row.get(4)?;
        if module_id.len() > MAX_MODULE_ID_BYTES
            || module_version.len() > MAX_MODULE_VERSION_BYTES
            || module_sha256.len() > MAX_MODULE_SHA256_BYTES
            || document.is_empty()
            || document.len() > MAX_PROFILE_DOCUMENT_BYTES
        {
            return Err(invalid("stored profile exceeds native load budget"));
        }
        let profile = DeviceConfigProfile::new(
            device,
            &module_id,
            &module_version,
            &module_sha256,
            document,
        )
        .map_err(|e: ProfileError| invalid(e.to_string()))?;
        if project.devices.get(device).is_none() {
            return Err(invalid(format!(
                "stored profile names device {} which does not exist",
                device.0
            )));
        }
        if project.devices.config_profile(device).is_some() {
            return Err(invalid("duplicate stored profile for one device"));
        }
        project.devices.set_config_profile(profile);
    }
    Ok(())
}
