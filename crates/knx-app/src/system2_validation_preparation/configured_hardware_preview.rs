//! Bounded operator-facing projection; never exports raw configuration or observed serials.
use super::{invalid, ProjectValidationProfile};
use crate::property_validation_bundle::{OperationRecord, PreparedBundle};
use knx_core::commissioning::mutation::{required_confirmation_phrase, WriteScope};
use knx_net::commissioning::hardware_release::required_release_phrase;
use serde::Serialize;
use std::io;

/// Shared App fields for thin UI/CLI adapters. Counts are not parameter values.
/// Adapter-owned preparation ID, project revision/incarnation and device name are separate.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfiguredHardwarePreview {
    pub address: String,
    pub program_id: String,
    pub profile_id: String,
    pub configuration_sha256: String,
    pub plan_sha256: String,
    pub parameter_values: usize,
    pub group_links: usize,
    pub flag_overrides: usize,
    pub steps: Vec<String>,
    pub data_octets: usize,
    pub allocation_octets: usize,
    pub not_written_octets: usize,
    pub warnings: Vec<String>,
    pub confirmation_phrase: String,
    pub hypothesis_phrase: String,
    pub download_phrase: String,
    pub release_phrase: String,
}
/// Shown for every source preview and every revision-2 observed record.
pub(crate) const PID7_WIDTH_WARNING: &str = "PID 7 width, experimental and limited to this exact UP258 profile: one element of two or four octets is accepted on objects 1, 2 and 3. A two-octet object 1 reply was observed on one device; objects 2 and 3 rely on the public package declaration only and are unverified. The standard defines four octets. This makes no claim about masks or firmware.";
/// Project-source preview is offline. Observed hardware records use the same source-bound
/// confirmation; observation and chosen recovery track gain their own journal binding.
pub(crate) fn project_hardware_preview(
    bundle: &PreparedBundle,
    operation: &str,
) -> io::Result<ConfiguredHardwarePreview> {
    let plan = bundle.plan()?;
    let source = bundle
        .project_preparation
        .as_ref()
        .ok_or_else(|| invalid("configured project source required"))?;
    let profile = ProjectValidationProfile::from_json(&source.profile_bytes)?;
    if bundle.normalized_plan.steps.len() > 2048 {
        return Err(invalid("configured plan exceeds public preview step bound"));
    }
    let mut data_octets = 0usize;
    let mut allocation_octets = 0usize;
    let mut steps = Vec::new();
    for (index, step) in bundle.normalized_plan.steps.iter().enumerate() {
        let detail = match &step.operation {
            OperationRecord::Connect => "connect and validate target identity".into(),
            OperationRecord::HardwareCheck { .. } => "check identity; acquire/persist/read back full affected prestate BEFORE VerifyMode setup".into(),
            OperationRecord::Disconnect => "verify complete allocation/control/application outcome; disconnect".into(),
            OperationRecord::WriteMemory { object, address, data } => {
                data_octets += data.len();
                format!("object {object} verified memory write at {address:#06x}, {} octets", data.len())
            }
            OperationRecord::LoadControl { object, octets } => {
                if octets[0] == 3 && octets[1] <= 1 { allocation_octets += usize::from(u16::from_be_bytes([octets[4],octets[5]])); }
                format!("object {object} load control event {} subtype {}", octets[0], octets[1])
            }
            OperationRecord::ToolMetadata { .. } => "tool-only metadata; no wire write".into(),
        };
        steps.push(format!("{index}: source {}: {detail}", step.source_index));
    }
    let configuration_sha256 = source.sha256()?;
    // Source previews precede any hardware record and must say what a fresh observation
    // will admit; an observed record carries the policy only from revision 2.
    let width_policy = bundle
        .hardware_preparation
        .as_ref()
        .is_none_or(|h| h.pointer_width_objects().is_some());
    let preview = ConfiguredHardwarePreview {
        address: bundle.binding.target.clone(), program_id: profile.program_id,
        profile_id: bundle.binding.profile_id.clone(),
        confirmation_phrase: format!("I confirm configured System-2 {operation} to {} preparation {configuration_sha256}", bundle.binding.target),
        configuration_sha256, plan_sha256: bundle.binding.plan_sha256.clone(),
        parameter_values: source.request.values.len(), group_links: source.request.links.len(), flag_overrides: source.request.flag_overrides.len(),
        steps, data_octets, allocation_octets,
        not_written_octets: allocation_octets.checked_sub(data_octets).ok_or_else(|| invalid("configured preview coverage overflow"))?,
        warnings: width_policy.then(|| PID7_WIDTH_WARNING.to_string()).into_iter().chain([
            "EXPERIMENTAL: software-tested, hardware compatibility and functional outcome unverified; not ETS equivalence.".into(),
            "Masked bytes and unused table capacity are NOT written. Unload/reallocation byte survival and indirect firmware effects are unresolved; recovery is best-effort, never guaranteed.".into(),
            "Requires the same installed application, Loaded resources and matching source-applicable pointers. Historical allocator/task shape is NOT observable or certified by those reads; it remains an explicit experimental premise.".into(),
            "The full affected actual prestate is acquired fresh and durably read back before setup or load/data writes. This is operation-specific evidence, NOT a whole-device backup.".into(),
            "Any failure, cancellation or lost response retains uncertainty and the original backup. No automatic retry, resume or restore. External ETS recovery is advisory, not certified; KNXBench restore is one explicit best-effort attempt.".into(),
            "Task starts and object mapping follow the retained source exactly. The historical observation does not define a general source-to-machine remapping rule.".into(),
        ]).collect(),
        hypothesis_phrase: bundle.acceptance_phrase(), download_phrase: required_confirmation_phrase(plan.target(), WriteScope::Download), release_phrase: required_release_phrase(plan.target()),
    };
    if serde_json::to_vec(&preview)
        .map_err(io::Error::other)?
        .len()
        > 64 * 1024
    {
        return Err(invalid("configured public preview exceeds response bound"));
    }
    Ok(preview)
}
/// Read-only summary of an already validated physical preparation.
pub fn configured_hardware_preview(
    bundle: &PreparedBundle,
) -> io::Result<ConfiguredHardwarePreview> {
    let hardware = bundle
        .hardware_preparation
        .as_ref()
        .ok_or_else(|| invalid("configured hardware preparation required"))?;
    project_hardware_preview(bundle, &hardware.operation)
}
