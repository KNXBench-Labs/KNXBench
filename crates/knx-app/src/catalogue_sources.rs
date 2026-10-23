//! Catalogue sources acquired for one revision-bound explicit native save.

use std::collections::BTreeSet;

use knx_store::{CarriedManufacturerFile, ManufacturerRef};

/// New manifest and file-owned payloads. Existing entries are preserved verbatim.
#[derive(Default)]
pub struct AcquiredSources {
    pub refs: Vec<ManufacturerRef>,
    pub files: Vec<CarriedManufacturerFile>,
    pub warnings: Vec<String>,
}

/// Only the source references needed by acquisition; do not clone project notes,
/// parameters or retained opaque data while the external database is read.
pub struct DeviceDependency {
    pub product_ref: String,
    pub hardware2program_ref: String,
}

pub fn device_dependencies(project: &knx_core::Project) -> Vec<DeviceDependency> {
    project
        .devices
        .iter()
        .filter(|d| !d.product_ref.is_empty())
        .map(|d| DeviceDependency {
            product_ref: d.product_ref.clone(),
            hardware2program_ref: d.program_ref.clone(),
        })
        .collect()
}

pub fn acquire_for_save(
    devices: &[DeviceDependency],
    existing: &[ManufacturerRef],
    products: Option<&knx_productdb::Connection>,
) -> Result<AcquiredSources, String> {
    let mut acquired = AcquiredSources {
        refs: existing.to_vec(),
        files: Vec::new(),
        warnings: Vec::new(),
    };
    if devices.is_empty() {
        return Ok(acquired);
    }
    let Some(products) = products else {
        if existing.is_empty() {
            return Err("catalogue source portability: required sources are unavailable; install matching originals before saving".into());
        }
        return Ok(acquired);
    };
    let read = products
        .unchecked_transaction()
        .map_err(|e| e.to_string())?;
    let mut covered: BTreeSet<String> = existing.iter().map(|r| r.sha256.clone()).collect();
    let mut charged = 0_u64;
    for device in devices {
        let owners = knx_productdb::catalogue_sources::device_source_owners(
            &read,
            &device.product_ref,
            &device.hardware2program_ref,
        )
        .map_err(|e| e.to_string())?;
        let Some(owners) = owners else {
            // Legacy manifests can describe missing sources that cannot be re-derived.
            // Preserve their existing warning/recovery contract, never guess membership.
            if !existing.is_empty() && !existing.iter().any(|r| r.kind == "CataloguePackage") {
                acquired.warnings.push(format!(
                    "catalogue source ownership is unavailable for product {} / program {}; the legacy manifest was preserved, but source portability is not established",
                    device.product_ref, device.hardware2program_ref,
                ));
                continue;
            }
            return Err(
                "catalogue source portability: required product/application mapping is unavailable"
                    .into(),
            );
        };
        if owners.is_subset(&covered) {
            continue;
        }
        let sources = knx_productdb::catalogue_sources::catalogue_dependency_sources(
            &read,
            &device.product_ref,
            &device.hardware2program_ref,
            knx_store::manufacturer_files::MAX_MANUFACTURER_FILE_BYTES,
            knx_core::limits::MAX_EXPANDED_ARCHIVE_BYTES.saturating_sub(charged),
        )
        .map_err(|e| e.to_string())?;
        for source in sources {
            if !covered.insert(source.sha256.clone()) {
                continue;
            }
            charged = charged
                .checked_add(source.bytes.len() as u64)
                .ok_or("catalogue source portability: source charge overflow")?;
            if charged > knx_core::limits::MAX_EXPANDED_ARCHIVE_BYTES {
                return Err(
                    "catalogue source portability: source closure exceeds expanded archive limit"
                        .into(),
                );
            }
            acquired.refs.push(ManufacturerRef {
                source_path: source.source_path.clone(),
                sha256: source.sha256,
                len: source.bytes.len() as i64,
                kind: source.kind,
            });
            acquired.files.push(CarriedManufacturerFile {
                source_path: source.source_path,
                bytes: source.bytes,
            });
        }
    }
    read.commit().map_err(|e| e.to_string())?;
    Ok(acquired)
}
