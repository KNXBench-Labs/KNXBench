//! Strict historical handoff for retained typed diagnostic results.

use super::{
    AdmittedEvidence, AdmittedResultIdentity, BundleUnavailableCode, ProjectContext,
    RecordingSummary, SourceFailure,
};
use knx_app::device_diagnostics::{Export, MemoryEvidence, Report};
use knx_app::diagnostic_bundle::BundleFile;
use serde_json::Value;

const MAX_TYPED_RECORDING_BYTES: u64 = 8 * 1024 * 1024;

pub(super) fn diagnostic_recording(
    state: &crate::SharedState,
    operation: Option<&RecordingSummary>,
    token: &str,
) -> Result<AdmittedEvidence, SourceFailure> {
    #[cfg(test)]
    {
        diagnostic_recording_inner(state, operation, token, None)
    }
    #[cfg(not(test))]
    {
        diagnostic_recording_inner(state, operation, token)
    }
}

#[cfg(test)]
pub(super) fn diagnostic_recording_observed(
    state: &crate::SharedState,
    operation: Option<&RecordingSummary>,
    token: &str,
    observer: &mut dyn FnMut(usize),
) -> Result<AdmittedEvidence, SourceFailure> {
    diagnostic_recording_inner(state, operation, token, Some(observer))
}

fn diagnostic_recording_inner(
    state: &crate::SharedState,
    operation: Option<&RecordingSummary>,
    token: &str,
    #[cfg(test)] observer: Option<&mut dyn FnMut(usize)>,
) -> Result<AdmittedEvidence, SourceFailure> {
    let operation = operation.ok_or_else(SourceFailure::unavailable)?;
    if operation.format_version != Some(1) {
        return Err(SourceFailure::Unavailable(
            BundleUnavailableCode::RecordingVersion,
        ));
    }
    if operation.operation_kind.as_deref() != Some("device-diagnostics")
        || operation.related["diagnosticsToken"].as_str() != Some(token)
        || operation.active
        || operation.torn_tail
        || operation.outcome.as_deref() != Some("succeeded")
        || operation.size_bytes > MAX_TYPED_RECORDING_BYTES
    {
        return Err(SourceFailure::Unavailable(
            BundleUnavailableCode::RecordingResult,
        ));
    }
    let bytes = state
        .recordings
        .read_bounded(&operation.id, MAX_TYPED_RECORDING_BYTES)
        .map_err(|_| SourceFailure::Unavailable(BundleUnavailableCode::RecordingResult))?;
    #[cfg(test)]
    if let Some(observer) = observer {
        observer(bytes.len());
    }
    if bytes.len() as u64 != operation.size_bytes {
        return Err(SourceFailure::Unavailable(
            BundleUnavailableCode::RecordingResult,
        ));
    }
    if bytes.last() != Some(&b'\n') {
        return Err(SourceFailure::Unavailable(
            BundleUnavailableCode::RecordingResult,
        ));
    }
    let mut lines = bytes
        .split(|byte| *byte == b'\n')
        .filter(|line| !line.is_empty());
    let header = lines
        .next()
        .and_then(|line| serde_json::from_slice::<Value>(line).ok())
        .filter(|header| source_header_matches(header, operation, token))
        .ok_or(SourceFailure::Unavailable(
            BundleUnavailableCode::RecordingResult,
        ))?;
    let end = lines
        .next_back()
        .and_then(|line| serde_json::from_slice::<Value>(line).ok())
        .filter(|value| {
            value["type"] == "end"
                && value["outcome"] == "succeeded"
                && value["error_chain"].as_array().is_some_and(Vec::is_empty)
        })
        .ok_or(SourceFailure::Unavailable(
            BundleUnavailableCode::RecordingResult,
        ))?;
    let summary = end["summary"]
        .as_object()
        .ok_or(SourceFailure::Unavailable(
            BundleUnavailableCode::RecordingResult,
        ))?;
    if summary.len() != 2 || !summary.contains_key("report") || !summary.contains_key("memory") {
        return Err(SourceFailure::Unavailable(
            BundleUnavailableCode::RecordingResult,
        ));
    }
    let report: Report = serde_json::from_value(summary["report"].clone())
        .map_err(|_| SourceFailure::Unavailable(BundleUnavailableCode::RecordingResult))?;
    let memory: Option<MemoryEvidence> = serde_json::from_value(summary["memory"].clone())
        .map_err(|_| SourceFailure::Unavailable(BundleUnavailableCode::RecordingResult))?;
    let reference = report.project_reference.clone();
    let writer_version = header["writer"]["version"]
        .as_str()
        .ok_or(SourceFailure::Unavailable(
            BundleUnavailableCode::RecordingResult,
        ))?;
    let reports = knx_app::diagnostic_bundle::admit_retained_diagnostics(
        &Export { report, memory },
        writer_version,
    )
    .map_err(|_| SourceFailure::Unavailable(BundleUnavailableCode::RecordingResult))?
    .into_files();
    let identity = serde_json::json!({"diagnosticsReference": reference.clone()});
    let context = header["related"]["projectContext"]
        .as_str()
        .and_then(|value| serde_json::from_str::<ProjectContext>(value).ok());
    let device_id = header["target"]["deviceId"]
        .as_u64()
        .and_then(|value| u32::try_from(value).ok());
    Ok(AdmittedEvidence {
        identity: AdmittedResultIdentity::Diagnostics { reference },
        context,
        device_id,
        reports,
        operation_files: vec![BundleFile::json("diagnostic-result.json", &identity)],
    })
}

fn source_header_matches(header: &Value, operation: &RecordingSummary, token: &str) -> bool {
    let gateway = operation
        .gateway
        .as_ref()
        .map_or(Value::Null, |value| Value::String(value.clone()));
    header["type"] == "header"
        && header["format"] == knx_app::operation_recording::RECORDING_FORMAT
        && header["format_version"].as_u64() == operation.format_version
        && header["operation_id"].as_str() == Some(operation.id.as_str())
        && header["operation_kind"].as_str() == operation.operation_kind.as_deref()
        && header["started_at"].as_str() == operation.started_at.as_deref()
        && header["writer"]["program"] == "knx-server"
        && header["writer"]["version"].as_str() == operation.writer_version.as_deref()
        && matches!(
            header["writer"]["build_profile"].as_str(),
            Some("debug" | "release")
        )
        && header["target"] == operation.target
        && header["gateway"] == gateway
        && header["related"] == operation.related
        && header["related"]["diagnosticsToken"].as_str() == Some(token)
}
