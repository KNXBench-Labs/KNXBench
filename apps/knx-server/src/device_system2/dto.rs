//! Strict bounded HTTP vocabulary for the configured System-2 adapter.
use crate::errors::ApiError;
use axum::http::StatusCode;
use knx_app::system2_validation_preparation::{
    HardwareConfirmations, HardwareRecoveryConfirmations, RecoveryTrack,
};
use serde::{Deserialize, Serialize};

pub(super) fn refuse(kind: &'static str) -> ApiError {
    ApiError::with_kind(StatusCode::CONFLICT, kind, kind)
}
pub(super) fn valid_id(id: &str) -> bool {
    id.len() == 32
        && id
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
pub(super) fn mint() -> Result<String, ApiError> {
    let mut bytes = [0; 16];
    getrandom::fill(&mut bytes).map_err(|_| refuse("system2Unavailable"))?;
    Ok(bytes.iter().map(|b| format!("{b:02x}")).collect())
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct Binding {
    pub server_incarnation: String,
    pub project_incarnation: String,
    pub project_revision: String,
    pub device_id: u32,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct Metadata {
    pub schema_version: u8,
    pub operation_id: String,
    pub binding: Binding,
    pub address: String,
    pub device_name: String,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct Preview {
    pub schema_version: u8,
    pub preparation_id: String,
    pub binding: Binding,
    pub device_name: String,
    #[serde(flatten)]
    pub summary: knx_app::system2_validation_preparation::ConfiguredHardwarePreview,
}
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct Status {
    #[serde(flatten)]
    pub metadata: Metadata,
    pub state: &'static str,
    pub completed_steps: usize,
    pub total_steps: Option<usize>,
    pub backup_available: bool,
    pub recovery_phrase: Option<String>,
    pub download_phrase: Option<String>,
    pub release_phrase: Option<String>,
    pub message: Option<&'static str>,
    /// False only when the durable journal proves no write could have been sent.
    pub mutation_may_have_started: bool,
    /// Captured prestate differed from the source shape or application:
    /// original-state recovery is unavailable for this operation.
    pub prestate_differs: bool,
    pub pointer_observations: Vec<PointerObservation>,
    pub program_version_observation: Option<ProgramVersionObservation>,
}
/// Source-model pointer versus device read, as lower-case hex or "refused".
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct PointerObservation {
    /// "download" or "restore": both runs of one operation are kept.
    pub run: &'static str,
    pub phase: &'static str,
    pub object: u8,
    /// Plan operation of an in-procedure read-back; null otherwise.
    pub operation_index: Option<usize>,
    pub expected: String,
    pub observed: String,
    pub load_state: String,
}
/// Plan application versus the device's prior program version (empty: refused).
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct ProgramVersionObservation {
    pub expected: String,
    pub observed: String,
}
pub(super) fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct Prepare {
    pub device_id: u32,
    pub source_mode: SourceMode,
}
pub(super) enum SourceMode {
    RetainedSources,
}
impl<'de> Deserialize<'de> for SourceMode {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        match String::deserialize(deserializer)?.as_str() {
            "retainedSources" => Ok(Self::RetainedSources),
            _ => Err(serde::de::Error::custom("unsupported System-2 source mode")),
        }
    }
}
pub(super) enum Track {
    Knxbench,
    Ets,
}
impl<'de> Deserialize<'de> for Track {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        match String::deserialize(deserializer)?.as_str() {
            "knxbenchBestEffort" => Ok(Self::Knxbench),
            "externalETS" => Ok(Self::Ets),
            _ => Err(serde::de::Error::custom(
                "unsupported System-2 recovery track",
            )),
        }
    }
}
impl Track {
    pub fn app(&self) -> RecoveryTrack {
        match self {
            Self::Knxbench => RecoveryTrack::KnxBenchBestEffort,
            Self::Ets => RecoveryTrack::ExternalEts,
        }
    }
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct Start {
    pub preparation_id: String,
    pub gateway: String,
    pub confirmation: String,
    pub hypothesis_confirmation: String,
    pub download_confirmation: String,
    pub release_confirmation: String,
    pub recovery_track: Track,
}
impl Start {
    pub fn confirmations(&self) -> HardwareConfirmations<'_> {
        HardwareConfirmations {
            operation: &self.confirmation,
            hypothesis: &self.hypothesis_confirmation,
            download: &self.download_confirmation,
            risk: &self.release_confirmation,
        }
    }
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct Recover {
    pub operation_id: String,
    pub gateway: String,
    pub recovery_confirmation: String,
    pub download_confirmation: String,
    pub release_confirmation: String,
}
impl Recover {
    pub fn confirmations(&self) -> HardwareRecoveryConfirmations<'_> {
        HardwareRecoveryConfirmations {
            recovery: &self.recovery_confirmation,
            download: &self.download_confirmation,
            risk: &self.release_confirmation,
        }
    }
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct Operation {
    pub operation_id: String,
}

/// These DTOs have no nested objects/maps. Serde's struct visitor rejects
/// duplicate fields; deny_unknown_fields closes the complete accepted grammar.
pub(super) fn decode<T: serde::de::DeserializeOwned>(bytes: &[u8]) -> Result<T, ApiError> {
    if bytes.len() > 16 * 1024 {
        return Err(ApiError::bad_request("system2InvalidRequest"));
    }
    serde_json::from_slice(bytes).map_err(|_| ApiError::bad_request("system2InvalidRequest"))
}
