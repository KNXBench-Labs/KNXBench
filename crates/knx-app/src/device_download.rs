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

use knx_core::commissioning::byte_order_evidence::{OctetOrder, Verdict};
use knx_core::commissioning::memory_download::{MemoryDownloadPlan, MemoryDownloadStep};
use knx_core::commissioning::partial_memory_download::{
    derive_partial_plan, PartialDownloadParts, PartialPlanError,
};
use knx_core::{DeviceId, IndividualAddress, Project};
use knx_productdb::byte_order_probe::{
    base_image_order, plan_probe, BaseImageOrder, ProbeError, ProbePlan,
};
use knx_productdb::code::load_program_code;
use knx_productdb::download_plan::{
    check_customer_adjustable, check_download_plugin, check_program_kind,
    plan_memory_download_with_inferences, DownloadPlanError,
};
use knx_productdb::image::{build_download_image, DownloadImage, ImageError, ImageRequest};
use knx_productdb::image_request::{image_request_for_device, DeviceRequestError};
use knx_productdb::inference::Inference;
use knx_productdb::Connection;

/// Everything a download to one device needs, built and checked offline.
#[derive(Debug)]
pub struct PreparedDownload {
    /// Versioned producer bound to this actual preparation.
    pub write_selection: crate::device_backup::WriteSelection,
    /// The project device.
    pub device: DeviceId,
    /// Its name in the project.
    pub device_name: String,
    /// Its individual address, the one the operator named.
    pub target: IndividualAddress,
    /// The configuration read from the project.
    pub request: ImageRequest,
    /// The literal source-based image; omitted bytes are not an expected post-write state.
    pub image: DownloadImage,
    /// The steps that put it there.
    pub plan: MemoryDownloadPlan,
    /// The inferences image and plan rest on (ADR-0086), for the user to
    /// see before acknowledging a write.
    pub inferences: Vec<Inference>,
    /// What the user should know before writing that does not stop the
    /// write (ADR-0120); empty when there is nothing.
    pub warnings: Vec<DownloadWarning>,
    /// `Some` once [`PreparedDownload::into_partial`] has replaced `plan`
    /// with a partial one: the parts, and the application writes CP
    /// §3.9.2.4 rule 3 ignores, as `(address, octets)`.
    pub partial: Option<(PartialDownloadParts, Vec<(u16, usize)>)>,
    /// The program's catalogue identity, read in the lease that built the
    /// request, image and plan. `None` when unreadable or not installed.
    pub program: Option<knx_productdb::query::ProgramIdentity>,
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

    /// Actual scoped WriteMemory payload octets per source segment.
    /// Source-protected and selection-omitted bytes are not counted; the image stays unchanged.
    pub fn octets_to_write(&self) -> Vec<(String, u32, usize, usize)> {
        self.image
            .segments
            .iter()
            .map(|segment| {
                let start = u64::from(segment.address);
                let end = start + segment.octets.len() as u64;
                let written = self
                    .plan
                    .steps
                    .iter()
                    .filter_map(|step| match step {
                        MemoryDownloadStep::WriteMemory { address, octets } => {
                            let from = u64::from(*address);
                            let to = from + octets.len() as u64;
                            Some(to.min(end).saturating_sub(from.max(start)) as usize)
                        }
                        _ => None,
                    })
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

/// Prepares the download to the one project device at `target`, to write
/// it: every download scope (complete or partial) and readiness start
/// here. A program that declares an ETS download plug-in is refused
/// (KL-006, [`check_download_plugin`]) right after the kind check, before
/// any image is built. The presence-only customer-adjustable refusal follows the plug-in check.
pub fn prepare_device_download(
    conn: &Connection,
    project: &Project,
    target: IndividualAddress,
) -> Result<PreparedDownload, PrepareError> {
    prepare(conn, project, target, Purpose::Download)
}

/// The same plan as [`prepare_device_download`], to *read* the device it
/// describes: `knx device compare` and comparison adoption. The byte-order
/// probe and device diagnostics have their own preparation services. Nothing
/// prepared here may be written: it skips the download refusals that do not
/// concern reading (KL-006), so a plug-in program's device can still be
/// compared.
pub fn prepare_device_download_for_reading(
    conn: &Connection,
    project: &Project,
    target: IndividualAddress,
) -> Result<PreparedDownload, PrepareError> {
    prepare(conn, project, target, Purpose::Reading)
}

/// What a preparation is for.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Purpose {
    Download,
    Reading,
}

fn prepare(
    conn: &Connection,
    project: &Project,
    target: IndividualAddress,
    purpose: Purpose,
) -> Result<PreparedDownload, PrepareError> {
    let device = device_at(project, target)?;
    let request =
        image_request_for_device(conn, project, device.id).map_err(PrepareError::Request)?;
    let (image, plan, inferences) = plan_request(conn, &request, purpose)?;
    let warnings = byte_order_warnings(conn, &request);
    let program = knx_productdb::query::program_identity(conn, &request.program_id)
        .ok()
        .flatten();
    Ok(PreparedDownload {
        write_selection: if purpose == Purpose::Download {
            crate::device_backup::WriteSelection::SourceDefaultInactiveWholeBytesV1
        } else {
            crate::device_backup::WriteSelection::SourceMaskOnlyV1
        },
        device: device.id,
        device_name: device.name.clone(),
        target,
        request,
        image,
        plan,
        inferences,
        warnings,
        partial: None,
        program,
    })
}

/// The image and plan `request` yields from the catalogue behind `conn`,
/// with every refusal a download of it meets. The one place that turns a
/// request into a plan, shared by preparing a download and by checking
/// later that its source still plans the same bytes.
fn plan_request(
    conn: &Connection,
    request: &ImageRequest,
    purpose: Purpose,
) -> Result<(DownloadImage, MemoryDownloadPlan, Vec<Inference>), PrepareError> {
    // A program no download translates is refused for that reason, not for
    // whatever its image build would trip over first. Precedence is fixed:
    // the kind (mask, then procedure style) is the more fundamental refusal
    // and comes first; then a declared plug-in (KL-006), then uninterpreted
    // customer-adjustable declarations refuse what remains, before image construction.
    if let Some(code) = load_program_code(conn, &request.program_id)
        .map_err(|e| PrepareError::Image(ImageError::Code(e)))?
    {
        check_program_kind(&code).map_err(PrepareError::Plan)?;
        if purpose == Purpose::Download {
            check_download_plugin(&code).map_err(PrepareError::Plan)?;
            check_customer_adjustable(&code).map_err(PrepareError::Plan)?;
        }
    }
    let image = build_download_image(conn, request).map_err(PrepareError::Image)?;
    let (plan, inferences) =
        plan_memory_download_with_inferences(&image).map_err(PrepareError::Plan)?;
    let plan = if purpose == Purpose::Download {
        knx_productdb::download_plan::select_inactive_whole_bytes(conn, request, &image, plan)
            .map_err(PrepareError::Plan)?
    } else {
        plan
    };
    Ok((image, plan, inferences))
}

/// Whether `written`, planned again from the catalogue behind `conn` now,
/// still yields exactly `ran` (scope `partial`): same mask and manufacturer,
/// same ordered load, task, check and write steps with the same addresses and
/// octets. `false` when the source no longer plans it, plans it differently
/// or is unreadable or refused: a download that ran an older plan then proves
/// nothing about the current source. A snapshot of this moment, not a watch
/// on later catalogue changes.
pub fn source_still_plans(
    conn: &Connection,
    written: &ImageRequest,
    partial: Option<PartialDownloadParts>,
    ran: &MemoryDownloadPlan,
) -> bool {
    let Ok((_, complete, _)) = plan_request(conn, written, Purpose::Download) else {
        return false;
    };
    match partial {
        None => complete == *ran,
        Some(parts) => derive_partial_plan(&complete, parts).is_ok_and(|now| now.plan == *ran),
    }
}

/// Something a download plan's user should know before writing that does not
/// stop the write (ADR-0120): a stable rule name, what it found here, and
/// where the analysis lives, like an [`Inference`]. Unlike an inference it is
/// not a rule the download applies.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DownloadWarning {
    /// Stable short name, the same for every program it applies to.
    pub rule: &'static str,
    /// What was found here.
    pub detail: String,
    /// Where the analysis is documented.
    pub reference: &'static str,
}

impl fmt::Display for DownloadWarning {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {} ({})", self.rule, self.detail, self.reference)
    }
}

/// [`DownloadWarning::rule`]: a big-endian program whose own base image
/// holds its multi-octet defaults low octet first.
pub const BASE_IMAGE_LOW_OCTET_FIRST: &str = "base-image-low-octet-first";

/// [`DownloadWarning::rule`]: the base image's octet order could not be
/// checked, so the warning above can neither be given nor ruled out.
pub const BASE_IMAGE_ORDER_UNCHECKED: &str = "base-image-order-unchecked";

/// Where the byte-order warnings are explained.
const BYTE_ORDER_REFERENCE: &str = "KNOWN_LIMITATIONS §170, ADR-0120";

/// The byte-order warnings of a download of `request` (KL-170, ADR-0120).
pub fn byte_order_warnings(conn: &Connection, request: &ImageRequest) -> Vec<DownloadWarning> {
    byte_order_warnings_for(base_image_order(conn, request))
}

/// The decision of [`byte_order_warnings`], from the base image's reading.
///
/// Warns only when every discriminating default is held low octet first in
/// a program that declares `BigEndian` or nothing: KNXBench writes what the
/// program declares, and the base image is product data, not a device, so
/// the mismatch is a reason to look, never a reason to write differently.
/// A program declaring another order is refused before any plan exists.
pub fn byte_order_warnings_for(order: Result<BaseImageOrder, ProbeError>) -> Vec<DownloadWarning> {
    let order = match order {
        Ok(order) => order,
        Err(error) => {
            return vec![DownloadWarning {
                rule: BASE_IMAGE_ORDER_UNCHECKED,
                detail: format!(
                    "the octet order of the program's base image could not be checked: {error}"
                ),
                reference: BYTE_ORDER_REFERENCE,
            }]
        }
    };
    let declaration = match order.declared_order.as_deref() {
        None => "declares no ParameterByteOrder (the schema default is BigEndian)",
        Some("BigEndian") => "declares ParameterByteOrder BigEndian",
        Some(_) => return Vec::new(),
    };
    if order.tally.verdict() != Verdict::Consistent(OctetOrder::LowFirst) {
        return Vec::new();
    }
    vec![DownloadWarning {
        rule: BASE_IMAGE_LOW_OCTET_FIRST,
        detail: format!(
            "the program {declaration}, so KNXBench writes multi-octet values high octet \
             first; its own base image holds {} multi-octet default(s) low octet first and \
             none high octet first. Whether the device reads them high octet first is not \
             known from the product data: check the device after the download",
            order.tally.low_first
        ),
        reference: BYTE_ORDER_REFERENCE,
    }]
}

/// The one project device at `target`; none and several are refused.
pub(crate) fn device_at(
    project: &Project,
    target: IndividualAddress,
) -> Result<&knx_core::DeviceInstance, PrepareError> {
    let mut matches: Vec<&knx_core::DeviceInstance> = project
        .devices
        .iter()
        .filter(|device| device.address == Some(target))
        .collect();
    matches.sort_by_key(|device| device.id.0);
    match matches.as_slice() {
        [] => Err(PrepareError::NoDeviceAt(target)),
        [one] => Ok(*one),
        several => Err(PrepareError::SeveralDevicesAt {
            address: target,
            devices: several.iter().map(|device| device.id.0).collect(),
        }),
    }
}

/// Why no byte-order probe was planned.
#[derive(Debug)]
pub enum ProbePrepareError {
    /// No single project device, or no request for it.
    Device(PrepareError),
    /// The program cannot be probed.
    Probe(ProbeError),
}

impl std::error::Error for ProbePrepareError {}

impl fmt::Display for ProbePrepareError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Device(error) => write!(f, "{error}"),
            Self::Probe(error) => write!(f, "{error}"),
        }
    }
}

/// Plans the byte-order probe of the one project device at `target`
/// (KL-170, ADR-0119): the device and request a download would take, read
/// rather than built into an image, so that a program declaring
/// `ParameterByteOrder="LittleEndian"` can be probed although it cannot be
/// downloaded.
pub fn prepare_byte_order_probe(
    conn: &Connection,
    project: &Project,
    target: IndividualAddress,
) -> Result<ProbePlan, ProbePrepareError> {
    let device = device_at(project, target).map_err(ProbePrepareError::Device)?;
    let request = image_request_for_device(conn, project, device.id)
        .map_err(|error| ProbePrepareError::Device(PrepareError::Request(error)))?;
    plan_probe(conn, &request).map_err(ProbePrepareError::Probe)
}

/// Prepares explicitly selected System-2 fields for offline source review only.
/// The distinct result type has no live-probe or download conversion.
pub fn prepare_system2_probe_preview(
    conn: &Connection,
    project: &Project,
    target: IndividualAddress,
    parameter_refs: &[String],
) -> Result<knx_productdb::byte_order_probe::SourceProbePreview, ProbePrepareError> {
    let device = device_at(project, target).map_err(ProbePrepareError::Device)?;
    let request = image_request_for_device(conn, project, device.id)
        .map_err(|error| ProbePrepareError::Device(PrepareError::Request(error)))?;
    knx_productdb::byte_order_probe::preview_system2_probe(conn, &request, parameter_refs)
        .map_err(ProbePrepareError::Probe)
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

    #[test]
    fn the_probe_finds_its_device_the_way_a_download_does() {
        let (_dir, conn) = products();
        let mut project = Project::new(Language("en".into()));
        project.devices.insert(device(7, Some("1.1.67")));
        project.devices.insert(device(3, Some("1.1.67")));
        project.devices.insert(device(4, Some("1.1.68")));
        assert!(matches!(
            prepare_byte_order_probe(&conn, &project, address("1.1.1")),
            Err(ProbePrepareError::Device(PrepareError::NoDeviceAt(_)))
        ));
        match prepare_byte_order_probe(&conn, &project, address("1.1.67")) {
            Err(ProbePrepareError::Device(PrepareError::SeveralDevicesAt { devices, .. })) => {
                assert_eq!(devices, vec![3, 7])
            }
            other => panic!("{other:?}"),
        }
        assert!(matches!(
            prepare_byte_order_probe(&conn, &project, address("1.1.68")),
            Err(ProbePrepareError::Device(PrepareError::Request(
                DeviceRequestError::Project(ProjectRequestError::NoProgram(DeviceId(4)))
            )))
        ));
    }

    fn order(declared: Option<&str>, low_first: usize, high_first: usize) -> BaseImageOrder {
        BaseImageOrder {
            declared_order: declared.map(str::to_string),
            tally: knx_core::commissioning::byte_order_evidence::Tally {
                low_first,
                high_first,
                ..Default::default()
            },
            without_data: 0,
        }
    }

    #[test]
    fn a_big_endian_program_with_a_low_first_base_image_is_warned_about() {
        for declared in [None, Some("BigEndian")] {
            let warnings = byte_order_warnings_for(Ok(order(declared, 3, 0)));
            assert_eq!(warnings.len(), 1, "{declared:?}");
            assert_eq!(warnings[0].rule, BASE_IMAGE_LOW_OCTET_FIRST);
            assert!(warnings[0]
                .detail
                .contains("3 multi-octet default(s) low octet first"));
            assert_eq!(warnings[0].reference, "KNOWN_LIMITATIONS §170, ADR-0120");
        }
        let shown = byte_order_warnings_for(Ok(order(None, 1, 0)))[0].to_string();
        assert!(shown.starts_with("base-image-low-octet-first: the program declares no"));
        assert!(shown.ends_with("(KNOWN_LIMITATIONS §170, ADR-0120)"));
    }

    #[test]
    fn high_first_mixed_or_silent_base_images_and_other_orders_are_not() {
        for (declared, low, high) in [
            (None, 0, 2),                 // agrees with what is written
            (Some("BigEndian"), 2, 1),    // contradictory: no consistent story
            (None, 0, 0),                 // nothing discriminating
            (Some("LittleEndian"), 3, 0), // refused before any plan anyway
        ] {
            assert_eq!(
                byte_order_warnings_for(Ok(order(declared, low, high))),
                vec![],
                "{declared:?} {low} {high}"
            );
        }
    }

    #[test]
    fn a_base_image_that_cannot_be_checked_is_named_not_passed_over() {
        let error = ProbeError::Program(DownloadPlanError::Address(0x1_0000));
        let warnings = byte_order_warnings_for(Err(error));
        assert_eq!(warnings.len(), 1);
        assert_eq!(warnings[0].rule, BASE_IMAGE_ORDER_UNCHECKED);
        assert!(warnings[0].detail.contains("could not be checked"));
    }
}
