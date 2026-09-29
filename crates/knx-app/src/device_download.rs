//! A download to one device, prepared from a project: request, image and plan, nothing sent.
//!
//! "Download" means KNXBench → device over the bus (docs/GLOSSARY.md). This
//! module stops one step short of that: it finds the device the operator
//! named by its individual address, reads its configuration from the
//! project ([`knx_productdb::image_request::image_request_for_device`]),
//! builds the memory image and the product's own load procedure as a plan.
//! No socket, no session, no write. The CLI (`knx device download`) and a
//! later UI both run the plan through `knx_net`'s memory download.

use std::fmt;

use knx_core::commissioning::memory_download::{unmasked_runs, MemoryDownloadPlan};
use knx_core::commissioning::partial_memory_download::{
    derive_partial_plan, PartialDownloadParts, PartialPlanError,
};
use knx_core::{DeviceId, IndividualAddress, Project};
use knx_productdb::code::load_program_code;
use knx_productdb::download_plan::{check_program_kind, plan_memory_download, DownloadPlanError};
use knx_productdb::image::{build_download_image, DownloadImage, ImageError, ImageRequest};
use knx_productdb::image_request::{image_request_for_device, DeviceRequestError};
use knx_productdb::Connection;

/// Everything a download to one device needs, built and checked offline.
#[derive(Debug)]
pub struct PreparedDownload {
    /// The project device.
    pub device: DeviceId,
    /// Its name in the project.
    pub device_name: String,
    /// Its individual address, the one the operator named.
    pub target: IndividualAddress,
    /// The configuration read from the project.
    pub request: ImageRequest,
    /// The memory the device should hold afterwards.
    pub image: DownloadImage,
    /// The steps that put it there.
    pub plan: MemoryDownloadPlan,
    /// `Some` once [`PreparedDownload::into_partial`] has replaced `plan`
    /// with a partial one: the parts, and the application writes CP
    /// §3.9.2.4 rule 3 ignores, as `(address, octets)`.
    pub partial: Option<(PartialDownloadParts, Vec<(u16, usize)>)>,
}

impl PreparedDownload {
    /// Replaces the complete plan with the partial download of `parts`
    /// CP §3.9.2.4 derives from it
    /// ([`knx_core::commissioning::partial_memory_download`]).
    pub fn into_partial(mut self, parts: PartialDownloadParts) -> Result<Self, PartialPlanError> {
        let partial = derive_partial_plan(&self.plan, parts)?;
        self.plan = partial.plan;
        self.partial = Some((parts, partial.ignored_writes));
        Ok(self)
    }

    /// Octets the plan writes to the device, per segment: masked octets
    /// (the device keeps its own) are not counted.
    pub fn octets_to_write(&self) -> Vec<(String, u32, usize, usize)> {
        self.image
            .segments
            .iter()
            .map(|segment| {
                let written = unmasked_runs(&segment.octets, segment.mask.as_deref())
                    .iter()
                    .map(|(_, run)| run.len())
                    .sum();
                (
                    segment.id.clone(),
                    segment.address,
                    segment.octets.len(),
                    written,
                )
            })
            .collect()
    }
}

/// Why no download could be prepared.
#[derive(Debug)]
pub enum PrepareError {
    /// No project device has this address.
    NoDeviceAt(IndividualAddress),
    /// More than one project device has this address, so the operator's
    /// address does not say which configuration to send.
    SeveralDevicesAt {
        address: IndividualAddress,
        devices: Vec<u32>,
    },
    /// The project does not yield a request for the device.
    Request(DeviceRequestError),
    /// The request does not yield an image.
    Image(ImageError),
    /// The image does not yield a plan.
    Plan(DownloadPlanError),
}

impl std::error::Error for PrepareError {}

impl fmt::Display for PrepareError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoDeviceAt(address) => {
                write!(f, "no device in the project has address {address}")
            }
            Self::SeveralDevicesAt { address, devices } => write!(
                f,
                "{} devices in the project have address {address} (ids {devices:?}); \
                 give each its own address first",
                devices.len()
            ),
            Self::Request(e) => write!(f, "configuration: {e}"),
            Self::Image(e) => write!(f, "memory image: {e}"),
            Self::Plan(e) => write!(f, "load procedure: {e}"),
        }
    }
}

/// Prepares the download to the one project device at `target`.
pub fn prepare_device_download(
    conn: &Connection,
    project: &Project,
    target: IndividualAddress,
) -> Result<PreparedDownload, PrepareError> {
    let mut matches: Vec<&knx_core::DeviceInstance> = project
        .devices
        .iter()
        .filter(|device| device.address == Some(target))
        .collect();
    matches.sort_by_key(|device| device.id.0);
    let device = match matches.as_slice() {
        [] => return Err(PrepareError::NoDeviceAt(target)),
        [one] => *one,
        several => {
            return Err(PrepareError::SeveralDevicesAt {
                address: target,
                devices: several.iter().map(|device| device.id.0).collect(),
            })
        }
    };
    let request =
        image_request_for_device(conn, project, device.id).map_err(PrepareError::Request)?;
    // A program no download translates is refused for that reason, not for
    // whatever its image build would trip over first.
    if let Some(code) = load_program_code(conn, &request.program_id)
        .map_err(|e| PrepareError::Image(ImageError::Code(e)))?
    {
        check_program_kind(&code).map_err(PrepareError::Plan)?;
    }
    let image = build_download_image(conn, &request).map_err(PrepareError::Image)?;
    let plan = plan_memory_download(&image).map_err(PrepareError::Plan)?;
    Ok(PreparedDownload {
        device: device.id,
        device_name: device.name.clone(),
        target,
        request,
        image,
        plan,
        partial: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use knx_core::{DeviceInstance, Language, SourceRef};
    use knx_productdb::image_request::ProjectRequestError;

    fn device(id: u32, address: Option<&str>) -> DeviceInstance {
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
            program_ref: String::new(),
            commissioning: Default::default(),
            visibility_calculated: true,
            com_objects: vec![],
            binary_data: vec![],
        }
    }

    fn products() -> (tempfile::TempDir, Connection) {
        let dir = tempfile::tempdir().unwrap();
        let conn = knx_productdb::open_and_migrate(&dir.path().join("p.sqlite")).unwrap();
        (dir, conn)
    }

    fn address(text: &str) -> IndividualAddress {
        text.parse().unwrap()
    }

    #[test]
    fn an_address_no_device_has_is_refused() {
        let (_dir, conn) = products();
        let mut project = Project::new(Language("en".into()));
        project.devices.insert(device(1, Some("1.1.1")));
        project.devices.insert(device(2, None));
        assert!(matches!(
            prepare_device_download(&conn, &project, address("1.1.67")),
            Err(PrepareError::NoDeviceAt(a)) if a == address("1.1.67")
        ));
    }

    #[test]
    fn two_devices_with_the_address_are_refused_by_id() {
        let (_dir, conn) = products();
        let mut project = Project::new(Language("en".into()));
        project.devices.insert(device(7, Some("1.1.67")));
        project.devices.insert(device(3, Some("1.1.67")));
        match prepare_device_download(&conn, &project, address("1.1.67")) {
            Err(PrepareError::SeveralDevicesAt { devices, .. }) => assert_eq!(devices, vec![3, 7]),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn a_device_without_a_program_is_refused_before_any_image() {
        let (_dir, conn) = products();
        let mut project = Project::new(Language("en".into()));
        project.devices.insert(device(1, Some("1.1.67")));
        assert!(matches!(
            prepare_device_download(&conn, &project, address("1.1.67")),
            Err(PrepareError::Request(DeviceRequestError::Project(
                ProjectRequestError::NoProgram(DeviceId(1))
            )))
        ));
    }
}
