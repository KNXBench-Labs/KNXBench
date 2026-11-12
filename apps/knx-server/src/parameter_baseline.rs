//! Compares a device's current parameter values with those of its last download (ADR-0173 §4).
//!
//! The baseline is what a completed regular download built its image from
//! (`knx_core::commissioning::parameter_baseline`); the current side is the
//! same computation over the values the project chooses now. The per-part
//! `parameters` flag stays the authority for "pending": this only explains
//! which values changed, and says when it cannot.

use chrono::SecondsFormat;
use knx_core::commissioning::parameter_baseline::{differences, ParameterBaseline};
use knx_core::{DeviceId, Project};
use serde::Serialize;

use crate::AppState;

/// Whether the baseline can be compared, and the verdict when it can.
#[derive(Serialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LastDownloadDto {
    pub(crate) state: LastDownloadState,
    /// When the download's settlement recorded the baseline (RFC 3339,
    /// UTC). Only for `usable`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) at: Option<String>,
    /// Every ref whose value differs, by id. Only for `usable`; may be empty.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) differing: Option<Vec<DifferingParameterDto>>,
}

#[derive(Serialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) enum LastDownloadState {
    /// Compared: `differing` names every difference.
    Usable,
    /// No completed download recorded one (an imported or older project, a
    /// manual mark, a System-2 run, a changed source, an interrupted run).
    Absent,
    /// The device's current program is not the one the download wrote.
    OtherProgram,
    /// The product data the current side needs is not available.
    ProductUnavailable,
    /// The project states two different values for one ref of this device,
    /// so its current values are not known (a download refuses it too).
    CurrentUnreadable,
}

/// One ref whose value at the last download is not its current value; a
/// side without a value is `null`.
#[derive(Serialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DifferingParameterDto {
    pub(crate) ref_id: String,
    pub(crate) downloaded: Option<String>,
    pub(crate) current: Option<String>,
}

impl LastDownloadDto {
    fn unusable(state: LastDownloadState) -> Self {
        Self {
            state,
            at: None,
            differing: None,
        }
    }
}

/// The comparison for `device` of `project` against `baseline`. Takes only
/// the product database lease; the caller read `project` and `baseline`
/// together under the project lock and released it.
pub(crate) fn last_download(
    state: &AppState,
    project: &Project,
    device: DeviceId,
    baseline: Option<&ParameterBaseline>,
) -> LastDownloadDto {
    let Some(baseline) = baseline else {
        return LastDownloadDto::unusable(LastDownloadState::Absent);
    };
    let Some(instance) = project.devices.get(device) else {
        return LastDownloadDto::unusable(LastDownloadState::Absent);
    };
    let Ok(chosen) = knx_productdb::image_request::chosen_parameter_values(project, device) else {
        return LastDownloadDto::unusable(LastDownloadState::CurrentUnreadable);
    };
    let Some(owner) = state.product_db.as_ref() else {
        return LastDownloadDto::unusable(LastDownloadState::ProductUnavailable);
    };
    owner
        .with_read(|conn| {
            Ok::<_, std::convert::Infallible>(compare(
                conn,
                &instance.program_ref,
                &chosen,
                baseline,
            ))
        })
        .ok()
        .and_then(Result::ok)
        .unwrap_or_else(|| LastDownloadDto::unusable(LastDownloadState::ProductUnavailable))
}

/// The comparison in one catalogue lease: the program the device names now,
/// then its effective values against the baseline.
fn compare(
    conn: &knx_productdb::Connection,
    program_ref: &str,
    chosen: &std::collections::BTreeMap<String, String>,
    baseline: &ParameterBaseline,
) -> LastDownloadDto {
    let program = match knx_productdb::query::resolve_program(conn, program_ref) {
        Ok(Some(program)) => program,
        Ok(None) | Err(_) => {
            return LastDownloadDto::unusable(LastDownloadState::ProductUnavailable)
        }
    };
    if program != baseline.program_id {
        return LastDownloadDto::unusable(LastDownloadState::OtherProgram);
    }
    match knx_productdb::dynamic::resolve_unscoped_values(conn, &program, chosen) {
        Ok(current) => LastDownloadDto {
            state: LastDownloadState::Usable,
            at: Some(baseline.at.to_rfc3339_opts(SecondsFormat::Millis, true)),
            differing: Some(
                differences(&baseline.values, &current)
                    .into_iter()
                    .map(|difference| DifferingParameterDto {
                        ref_id: difference.ref_id,
                        downloaded: difference.downloaded,
                        current: difference.current,
                    })
                    .collect(),
            ),
        },
        Err(_) => LastDownloadDto::unusable(LastDownloadState::ProductUnavailable),
    }
}
