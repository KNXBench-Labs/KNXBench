//! Parts a verified configured run may still credit at its end, against the current project.
use super::hardware::{admit_current_project_hardware, prepare_project_hardware_source};
use super::{invalid, RequestRecord};
use crate::programming_state::still_current;
use crate::property_validation_bundle::PreparedBundle;
use knx_core::commissioning::pending::ProgrammingParts;
use knx_core::{DeviceId, Project};
use knx_productdb::image_request::image_request_for_device;
use std::io;

/// The parts of `device` in the current `project` that still equal what the verified run
/// wrote. Proves the ORIGINAL retained project through strict admission first, or the
/// narrow caption-only settlement comparison after a typed changed-binding refusal.
/// Only then is the current request compared with the written
/// one, through the same dependency mask as a regular download. Product and program
/// references must not switch; equal resolved program ids do not hide a switch.
pub fn current_project_hardware_parts(
    conn: &knx_productdb::Connection,
    project: &Project,
    device: DeviceId,
    bundle: &PreparedBundle,
) -> io::Result<ProgrammingParts> {
    // Validate immutable O independently BEFORE interpreting any live preparation failure.
    // A helper proves consistency only; the server owns terminal store/admission pairing.
    bundle.plan()?;
    bundle.preconditions()?;
    let hardware = bundle
        .hardware_preparation
        .as_ref()
        .ok_or_else(|| invalid("separate configured hardware preparation required"))?;
    let source = bundle
        .project_preparation
        .as_ref()
        .ok_or_else(|| invalid("missing configured project source"))?;
    if source.device_id != device.0 {
        return Err(invalid("settled device is not the prepared device"));
    }
    // Source proof: the existing start admission, applied to the original project.
    let original = source.decoded_project()?;
    let comparison = match super::source_phase::outcome(admit_current_project_hardware(
        conn, &original, device, bundle,
    ))? {
        super::source_phase::StrictSource::Current => None,
        super::source_phase::StrictSource::ChangedBinding => Some(
            super::project::settlement::compare(conn, &original, device, bundle)?,
        ),
    };
    let (was, now) = original
        .devices
        .get(device)
        .zip(project.devices.get(device))
        .ok_or_else(|| invalid("settled device missing"))?;
    if was.product_ref != now.product_ref
        || was.program_ref != now.program_ref
        || was.source != now.source
    {
        return Err(invalid(
            "product or program reference of the device switched",
        ));
    }
    // The current request must itself pass the same configured preparation.
    let fresh = prepare_project_hardware_source(
        conn,
        project,
        device,
        comparison
            .as_ref()
            .map_or(source.profile_bytes.as_slice(), |c| c.profile.as_slice()),
        &hardware.operation,
        hardware.recovery_track,
    )?;
    let fresh = fresh
        .preview()
        .project_preparation
        .as_ref()
        .ok_or_else(|| invalid("missing fresh project source"))?;
    if comparison
        .as_ref()
        .is_some_and(|c| c.input != fresh.product_input)
    {
        return Err(invalid("current comparison source recollection differs"));
    }
    let written = image_request_for_device(conn, &original, device).map_err(io::Error::other)?;
    let current = image_request_for_device(conn, project, device).map_err(io::Error::other)?;
    if RequestRecord::from_request(&written) != source.request
        || RequestRecord::from_request(&current) != fresh.request
    {
        return Err(invalid("request differs from its configured preparation"));
    }
    Ok(still_current(&written, &current))
}
