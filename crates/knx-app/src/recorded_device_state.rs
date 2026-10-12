//! Pure read-only projection: persisted belief and model-held answers are separate.
use knx_core::{commissioning::LoadPart, device::DeviceInstance};
use serde::Serialize;
#[derive(Debug, Serialize)]
pub struct PartState {
    pub part: &'static str,
    pub intended_loaded: bool,
    pub reported: Option<u8>,
}
#[derive(Debug, Serialize)]
pub struct RecordedDeviceState {
    pub view_version: u32,
    pub scope: &'static str,
    pub device_id: u32,
    pub name: String,
    pub completion: &'static str,
    pub broken: bool,
    pub last_modified: Option<String>,
    pub last_download: Option<String>,
    pub parts: [PartState; 5],
}
pub fn device(device: &DeviceInstance) -> Result<RecordedDeviceState, String> {
    if device.name.len() > super::modeled_metadata::MAX_TEXT_BYTES {
        return Err(
            "recorded device model exceeds 64 KiB text budget; device remains unchanged".into(),
        );
    }
    let state = &device.commissioning;
    Ok(RecordedDeviceState {
        view_version: 1,
        scope: "recordedDeviceModel",
        device_id: device.id.0,
        name: device.name.clone(),
        completion: super::modeled_metadata::completion_name(state.completion),
        broken: state.broken,
        last_modified: state.last_modified.as_ref().map(|d| d.to_rfc3339()),
        last_download: state.last_download.as_ref().map(|d| d.to_rfc3339()),
        parts: LoadPart::ALL.map(|part| PartState {
            part: match part {
                LoadPart::IndividualAddress => "IndividualAddress",
                LoadPart::ApplicationProgram => "ApplicationProgram",
                LoadPart::Parameters => "Parameters",
                LoadPart::CommunicationPart => "CommunicationPart",
                LoadPart::MediumConfig => "MediumConfig",
            },
            intended_loaded: state.intends_loaded(part),
            reported: state.device_reported.reported(part).map(|r| r.octet()),
        }),
    })
}
