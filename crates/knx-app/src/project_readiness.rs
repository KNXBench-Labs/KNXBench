//! Offline download readiness for all project devices or one selected device.
//!
//! `knx products coverage` answers per *application program*, with product
//! defaults. The question an installer has is narrower: which of *my*
//! devices can KNXBench download, with the configuration this project gives
//! them. This module answers it per project device, by preparing exactly the
//! download `knx device download` would prepare
//! ([`prepare_device_download`]) and grading it as that command does
//! ([`download_level`]). Nothing is sent; there is no socket here.
//!
//! A device on the project exclusion list is not prepared at all: it is
//! reported as excluded, the same refusal a download gives before anything
//! else. A device without an individual address has no download.

use std::collections::BTreeMap;

use knx_core::{is_project_excluded, DeviceId, DeviceInstance, IndividualAddress, Project};
use knx_productdb::Connection;

use crate::device_download::{prepare_device_download, PrepareError};
use crate::download_support::{
    download_level, image_category, plan_category, SupportLevel, UnsupportedCategory,
    VerifiedEvidence,
};

/// Where one project device stands.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeviceReadiness {
    /// On the project exclusion list: never contacted, never prepared.
    Excluded,
    /// No individual address, so no download target.
    NoAddress,
    /// Prepared as `knx device download` prepares it, and graded.
    Graded(SupportLevel),
}

impl DeviceReadiness {
    /// A stable name: `verified`, `untested`, `unsupported`, `excluded` or
    /// `no-address`.
    pub fn code(&self) -> &'static str {
        match self {
            Self::Excluded => "excluded",
            Self::NoAddress => "no-address",
            Self::Graded(level) => level.code(),
        }
    }
}

/// One project device's row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceRow {
    /// Its individual address, if it has one.
    pub address: Option<IndividualAddress>,
    /// Its name in the project.
    pub name: String,
    /// Its `program_ref`, verbatim (empty when the device names none).
    pub program_ref: String,
    /// Where it stands.
    pub readiness: DeviceReadiness,
}

/// Every device of `project`, ordered by address (devices without one
/// last, by name).
pub fn project_readiness(
    conn: &Connection,
    project: &Project,
    evidence: &BTreeMap<String, VerifiedEvidence>,
) -> Vec<DeviceRow> {
    let mut rows: Vec<DeviceRow> = project
        .devices
        .iter()
        .map(|device| device_row(conn, project, device, evidence))
        .collect();
    rows.sort_by(|a, b| {
        (a.address.is_none(), a.address, &a.name).cmp(&(b.address.is_none(), b.address, &b.name))
    });
    rows
}

/// Only the selected device, using the complete project for preparation and safety checks.
/// An unknown id returns `None`; no other device is prepared.
pub fn device_readiness(
    conn: &Connection,
    project: &Project,
    device_id: DeviceId,
    evidence: &BTreeMap<String, VerifiedEvidence>,
) -> Option<DeviceRow> {
    project
        .devices
        .get(device_id)
        .map(|device| device_row(conn, project, device, evidence))
}

fn device_row(
    conn: &Connection,
    project: &Project,
    device: &DeviceInstance,
    evidence: &BTreeMap<String, VerifiedEvidence>,
) -> DeviceRow {
    DeviceRow {
        address: device.address,
        name: device.name.clone(),
        program_ref: device.program_ref.clone(),
        readiness: match device.address {
            None => DeviceReadiness::NoAddress,
            Some(address) if is_project_excluded(address) => DeviceReadiness::Excluded,
            Some(address) => DeviceReadiness::Graded(grade(conn, project, address, evidence)),
        },
    }
}

#[cfg(test)]
thread_local! {
    // Records real preparation entry, not returned rows; each test thread owns its trace.
    static PREPARED_ADDRESSES: std::cell::RefCell<Vec<IndividualAddress>> = const {
        std::cell::RefCell::new(Vec::new())
    };
}

fn grade(
    conn: &Connection,
    project: &Project,
    address: IndividualAddress,
    evidence: &BTreeMap<String, VerifiedEvidence>,
) -> SupportLevel {
    #[cfg(test)]
    PREPARED_ADDRESSES.with_borrow_mut(|addresses| addresses.push(address));
    match prepare_device_download(conn, project, address) {
        Ok(prepared) => download_level(&prepared, evidence),
        Err(error) => SupportLevel::Unsupported {
            category: prepare_category(&error),
            detail: error.to_string(),
        },
    }
}

/// The category of a refusal to prepare.
pub fn prepare_category(error: &PrepareError) -> UnsupportedCategory {
    match error {
        PrepareError::Request(knx_productdb::image_request::DeviceRequestError::Database(_))
        | PrepareError::Request(knx_productdb::image_request::DeviceRequestError::Project(
            knx_productdb::image_request::ProjectRequestError::UnresolvedProgram { .. },
        )) => UnsupportedCategory::ProductData,
        PrepareError::NoDeviceAt(_)
        | PrepareError::SeveralDevicesAt { .. }
        | PrepareError::Request(_) => UnsupportedCategory::Configuration,
        PrepareError::Image(error) => image_category(error),
        PrepareError::Plan(error) => plan_category(error),
    }
}

/// Counts by readiness code, and refusals by category.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ReadinessSummary {
    /// Devices in the project.
    pub devices: usize,
    /// Per [`DeviceReadiness::code`].
    pub by_code: BTreeMap<&'static str, usize>,
    /// Refused devices, per category.
    pub unsupported: BTreeMap<UnsupportedCategory, usize>,
}

impl ReadinessSummary {
    /// Summarises `rows`.
    pub fn of(rows: &[DeviceRow]) -> Self {
        let mut summary = Self {
            devices: rows.len(),
            ..Self::default()
        };
        for row in rows {
            *summary.by_code.entry(row.readiness.code()).or_default() += 1;
            if let DeviceReadiness::Graded(SupportLevel::Unsupported { category, .. }) =
                &row.readiness
            {
                *summary.unsupported.entry(*category).or_default() += 1;
            }
        }
        summary
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use knx_core::{DeviceId, DeviceInstance, Language, SourceRef};

    fn device(id: u32, address: Option<&str>, program: &str) -> DeviceInstance {
        DeviceInstance {
            id: DeviceId(id),
            source: SourceRef {
                path: "test".into(),
                ets_id: format!("DEV-{id}"),
            },
            name: format!("Device {id}"),
            description: None,
            address: address.map(|a| a.parse().unwrap()),
            product_ref: String::new(),
            program_ref: program.into(),
            commissioning: Default::default(),
            visibility_calculated: true,
            com_objects: vec![],
            binary_data: vec![],
        }
    }

    fn rows(devices: Vec<DeviceInstance>) -> Vec<DeviceRow> {
        let dir = tempfile::tempdir().unwrap();
        let conn = knx_productdb::open_and_migrate(&dir.path().join("p.sqlite")).unwrap();
        let mut project = Project::new(Language("en".into()));
        for device in devices {
            project.devices.insert(device);
        }
        project_readiness(&conn, &project, &BTreeMap::new())
    }

    #[test]
    fn an_excluded_device_is_not_prepared_and_says_so() {
        let excluded = knx_core::EXCLUDED_INDIVIDUAL_ADDRESSES[0].to_string();
        let rows = rows(vec![device(1, Some(&excluded), "")]);
        // Prepared, the empty program_ref would be a configuration refusal.
        assert_eq!(rows[0].readiness, DeviceReadiness::Excluded);
    }

    #[test]
    fn a_device_without_an_address_has_no_download() {
        let rows = rows(vec![device(1, None, "")]);
        assert_eq!(rows[0].readiness, DeviceReadiness::NoAddress);
    }

    #[test]
    fn a_configuration_refusal_is_graded_unsupported_with_the_download_s_words() {
        let rows = rows(vec![
            device(1, Some("1.1.1"), ""),
            device(2, Some("1.1.2"), "M-0083_A-FFFF-10-0000"),
            device(3, Some("1.1.3"), ""),
            device(4, Some("1.1.3"), ""),
        ]);
        for (index, row) in rows.iter().enumerate() {
            match &row.readiness {
                DeviceReadiness::Graded(SupportLevel::Unsupported { category, detail }) => {
                    let expected = if index == 1 {
                        UnsupportedCategory::ProductData
                    } else {
                        UnsupportedCategory::Configuration
                    };
                    assert_eq!(*category, expected, "{detail}");
                }
                other => panic!("{}: {other:?}", row.name),
            }
        }
        let detail = |row: &DeviceRow| match &row.readiness {
            DeviceReadiness::Graded(SupportLevel::Unsupported { detail, .. }) => detail.clone(),
            _ => unreachable!(),
        };
        assert!(
            detail(&rows[1]).contains("M-0083_A-FFFF-10-0000"),
            "{}",
            detail(&rows[1])
        );
        assert!(
            detail(&rows[2]).contains("2 devices"),
            "{}",
            detail(&rows[2])
        );
    }

    #[test]
    fn scoped_rows_match_full_rows_including_exclusions_and_duplicate_address_refusals() {
        let dir = tempfile::tempdir().unwrap();
        let conn = knx_productdb::open_and_migrate(&dir.path().join("p.sqlite")).unwrap();
        let excluded = knx_core::EXCLUDED_INDIVIDUAL_ADDRESSES[0].to_string();
        let mut project = Project::new(Language("en".into()));
        for device in [
            device(1, None, ""),
            device(2, Some(&excluded), ""),
            device(3, Some("1.1.3"), ""),
            device(4, Some("1.1.4"), "missing-program"),
            device(5, Some("1.1.5"), ""),
            device(6, Some("1.1.5"), ""),
        ] {
            project.devices.insert(device);
        }
        let evidence = BTreeMap::new();
        let full = project_readiness(&conn, &project, &evidence);
        for id in 1..=6 {
            let scoped = device_readiness(&conn, &project, DeviceId(id), &evidence).unwrap();
            assert_eq!(
                &scoped,
                full.iter().find(|row| row.name == scoped.name).unwrap()
            );
        }
        assert!(device_readiness(&conn, &project, DeviceId(99), &evidence).is_none());
        let duplicate = device_readiness(&conn, &project, DeviceId(5), &evidence).unwrap();
        assert!(matches!(duplicate.readiness,
            DeviceReadiness::Graded(SupportLevel::Unsupported { category: UnsupportedCategory::Configuration, ref detail })
            if detail.contains("2 devices")));
    }

    #[test]
    fn scoped_readiness_prepares_only_the_requested_device() {
        let dir = tempfile::tempdir().unwrap();
        let conn = knx_productdb::open_and_migrate(&dir.path().join("p.sqlite")).unwrap();
        let mut project = Project::new(Language("en".into()));
        let excluded = knx_core::EXCLUDED_INDIVIDUAL_ADDRESSES[0].to_string();
        for device in [
            device(1, Some("1.1.1"), ""),
            device(2, Some("1.1.2"), ""),
            device(3, Some(&excluded), ""),
            device(4, None, ""),
        ] {
            project.devices.insert(device);
        }
        let evidence = BTreeMap::new();
        PREPARED_ADDRESSES.with_borrow_mut(Vec::clear);
        device_readiness(&conn, &project, DeviceId(2), &evidence).unwrap();
        for id in [3, 4, 99] {
            let _ = device_readiness(&conn, &project, DeviceId(id), &evidence);
        }
        PREPARED_ADDRESSES.with_borrow(|addresses| {
            assert_eq!(addresses, &["1.1.2".parse::<IndividualAddress>().unwrap()]);
        });
        // Calibrate the observer: the unchanged full path really prepares both ordinary devices.
        PREPARED_ADDRESSES.with_borrow_mut(Vec::clear);
        project_readiness(&conn, &project, &evidence);
        PREPARED_ADDRESSES.with_borrow(|addresses| {
            assert_eq!(
                addresses,
                &[
                    "1.1.1".parse::<IndividualAddress>().unwrap(),
                    "1.1.2".parse().unwrap()
                ]
            );
        });
    }

    #[test]
    fn scoped_readiness_preserves_real_plan_grading_evidence_and_plan_refusal() {
        use knx_testsupport::vendor_plugin::{
            declaration, program_xml, HARDWARE2PROGRAM_REF, HARDWARE_XML, PRODUCT_REF, PROGRAM_ID,
            TARGET,
        };
        let mut project = Project::new(Language("en".into()));
        let mut target = device(1, Some(TARGET), HARDWARE2PROGRAM_REF);
        target.product_ref = PRODUCT_REF.into();
        project.devices.insert(target);
        project.installations.push(knx_core::Installation {
            id: knx_core::InstallationId(0),
            name: "Synthetic".into(),
            default_line: None,
            multicast_address: None,
            completion: knx_core::CompletionStatus::FinishedDesign,
            topology: knx_core::Topology {
                areas: vec![],
                lines: vec![],
                unassigned: vec![DeviceId(1)],
            },
            buildings: vec![],
            group_ranges: vec![],
            group_addresses: vec![],
            parameters: vec![],
        });
        for plugin in [false, true] {
            let dir = tempfile::tempdir().unwrap();
            let conn = knx_productdb::open_and_migrate(&dir.path().join("p.sqlite")).unwrap();
            let program = program_xml(&if plugin { declaration() } else { String::new() });
            knx_productdb::ingest_file(&conn, "M-FFF6/A.xml", program.as_bytes()).unwrap();
            knx_productdb::ingest_file(&conn, "M-FFF6/Hardware.xml", HARDWARE_XML.as_bytes())
                .unwrap();
            for verified in [false, true] {
                // Synthetic evidence tests grade selection only; it makes no hardware claim.
                let evidence = if verified {
                    BTreeMap::from([(
                        PROGRAM_ID.into(),
                        VerifiedEvidence {
                            program_id: PROGRAM_ID.into(),
                            scopes: vec!["complete".into()],
                            device: "Synthetic test only".into(),
                            date: "2026-01-01".into(),
                            reference: "synthetic".into(),
                        },
                    )])
                } else {
                    BTreeMap::new()
                };
                let scoped = device_readiness(&conn, &project, DeviceId(1), &evidence).unwrap();
                assert_eq!(
                    vec![scoped.clone()],
                    project_readiness(&conn, &project, &evidence)
                );
                assert_eq!(
                    scoped.readiness.code(),
                    if plugin {
                        "unsupported"
                    } else if verified {
                        "verified"
                    } else {
                        "untested"
                    }
                );
                if plugin {
                    assert!(matches!(
                        scoped.readiness,
                        DeviceReadiness::Graded(SupportLevel::Unsupported {
                            category: UnsupportedCategory::VendorDownloadPlugin,
                            ..
                        })
                    ));
                }
            }
        }
    }

    #[test]
    fn rows_are_ordered_by_address_and_counted_by_code() {
        let excluded = knx_core::EXCLUDED_INDIVIDUAL_ADDRESSES[0].to_string();
        let rows = rows(vec![
            device(1, None, ""),
            device(2, Some("1.1.10"), ""),
            device(3, Some(&excluded), ""),
            device(4, Some("1.1.2"), ""),
        ]);
        let order: Vec<String> = rows
            .iter()
            .map(|row| row.address.map_or("-".into(), |a| a.to_string()))
            .collect();
        assert_eq!(order, vec!["1.1.2", "1.1.10", excluded.as_str(), "-"]);
        let summary = ReadinessSummary::of(&rows);
        assert_eq!(summary.devices, 4);
        assert_eq!(summary.by_code.get("unsupported"), Some(&2));
        assert_eq!(summary.by_code.get("excluded"), Some(&1));
        assert_eq!(summary.by_code.get("no-address"), Some(&1));
        assert_eq!(
            summary.unsupported.get(&UnsupportedCategory::Configuration),
            Some(&2)
        );
    }
}
