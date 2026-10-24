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
    /// Native catalogue creation records this exact source identity. Older or
    /// imported identities do not establish ownership and must stay unknown.
    pub catalogue_authored: bool,
    /// The same native-authored dependency is in the explicit saved baseline,
    /// not merely in an unsaved working journal recovered on reopen.
    pub saved_catalogue_dependency: bool,
}

pub fn device_dependencies(
    project: &knx_core::Project,
    saved: Option<&knx_core::Project>,
) -> Vec<DeviceDependency> {
    let saved_dependencies: BTreeSet<_> = saved
        .into_iter()
        .flat_map(|project| project.devices.iter())
        .filter(|device| is_catalogue_authored(&device.source))
        .map(|device| (device.product_ref.as_str(), device.program_ref.as_str()))
        .collect();
    project
        .devices
        .iter()
        .filter(|d| !d.product_ref.is_empty())
        .map(|d| DeviceDependency {
            product_ref: d.product_ref.clone(),
            hardware2program_ref: d.program_ref.clone(),
            catalogue_authored: is_catalogue_authored(&d.source),
            saved_catalogue_dependency: saved_dependencies
                .contains(&(d.product_ref.as_str(), d.program_ref.as_str())),
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
        // Only retain an already-saved native dependency offline. A working
        // journal or an unrelated existing manifest cannot acquire new sources.
        let has_saved_package = existing
            .iter()
            .any(|source| source.kind == "CataloguePackage");
        if devices.iter().any(|device| {
            device.catalogue_authored && (!has_saved_package || !device.saved_catalogue_dependency)
        }) {
            return Err("catalogue source portability: required sources are unavailable; install matching originals before saving".into());
        }
        for device in devices {
            acquired.warnings.push(unavailable_ownership(device));
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
            // Import can retain devices even when the archive has no manufacturer
            // declarations, hence an empty manifest is not proof of new authoring.
            // Conversely an unrelated legacy manifest cannot back a new device.
            if !device.catalogue_authored {
                acquired.warnings.push(unavailable_ownership(device));
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

fn unavailable_ownership(device: &DeviceDependency) -> String {
    format!(
        "catalogue source ownership is unavailable for product {} / program {}; the existing source manifest was preserved, but source portability is not established",
        device.product_ref, device.hardware2program_ref,
    )
}

fn is_catalogue_authored(source: &knx_core::SourceRef) -> bool {
    // The source ID survives copying/remapping of the project-local device ID.
    source.path == source.ets_id
        && source.path.strip_prefix("KB-DEV-").is_some_and(|id| {
            !id.starts_with('0')
                && id.bytes().all(|byte| byte.is_ascii_digit())
                && id.parse::<u32>().is_ok()
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_the_exact_native_catalogue_source_namespace_marks_new_authorship() {
        for (path, ets_id, expected) in [
            ("KB-DEV-1", "KB-DEV-1", true),
            ("KB-DEV-4294967295", "KB-DEV-4294967295", true),
            ("P-0001/0.xml", "KB-DEV-1", false),
            ("KB-DEV-1", "different", false),
            ("KB-DEV-0", "KB-DEV-0", false),
            ("KB-DEV-01", "KB-DEV-01", false),
            ("KB-DEV-4294967296", "KB-DEV-4294967296", false),
            ("fixture", "D-1", false),
        ] {
            assert_eq!(
                is_catalogue_authored(&knx_core::SourceRef {
                    path: path.into(),
                    ets_id: ets_id.into(),
                }),
                expected,
            );
        }
    }
}
