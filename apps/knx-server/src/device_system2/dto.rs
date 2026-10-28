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
/// Status output version; the private `Metadata` file and `Preview` stay at version 1.
/// Version 2 added `failureDiagnostics`, version 3 `downloadResult`, version 4
/// `finalStepFailures`; older exact-version clients refuse it.
pub(super) const STATUS_SCHEMA_VERSION: u8 = 4;
/// The operation identity the status shows, without the private file's schema version, so
/// the flattened status carries exactly one `schemaVersion` key.
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct StatusIdentity {
    pub operation_id: String,
    pub binding: Binding,
    pub address: String,
    pub device_name: String,
}
impl From<&Metadata> for StatusIdentity {
    fn from(metadata: &Metadata) -> Self {
        Self {
            operation_id: metadata.operation_id.clone(),
            binding: metadata.binding.clone(),
            address: metadata.address.clone(),
            device_name: metadata.device_name.clone(),
        }
    }
}
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct Status {
    pub schema_version: u8,
    #[serde(flatten)]
    pub metadata: StatusIdentity,
    pub state: &'static str,
    pub completed_steps: usize,
    pub total_steps: Option<usize>,
    pub backup_available: bool,
    pub recovery_phrase: Option<String>,
    pub download_phrase: Option<String>,
    pub release_phrase: Option<String>,
    pub message: Option<&'static str>,
    /// Stable kind of this worker's refusal; not persisted. Reload and a new
    /// recovery attempt start at null rather than inheriting an old reason.
    pub refusal: Option<&'static str>,
    /// False only when the server confirms no write could have been sent,
    /// including refusals before connection or journal creation.
    pub mutation_may_have_started: bool,
    /// Captured prestate differed from the source shape or application:
    /// original-state recovery is unavailable for this operation.
    pub prestate_differs: bool,
    /// Declared allocations whose prior memory read was refused or short: no
    /// backup exists for them (their stored bytes are zero placeholders).
    pub prestate_unreadable_allocations: Vec<UnreadableAllocation>,
    pub pointer_observations: Vec<PointerObservation>,
    pub program_version_observation: Option<ProgramVersionObservation>,
    /// Typed, non-raw failure facts: at most one per run, download before restore.
    /// Never the private journal text; empty when none was recorded or it is unknown.
    pub failure_diagnostics: Vec<knx_app::property_validation::FailureDiagnostic>,
    /// Typed final-step failure per run (category and stage, no text): at most one per run,
    /// download before restore. Empty when none was recorded or it is unknown (older journal).
    pub final_step_failures: Vec<knx_app::property_validation::FinalStepDiagnostic>,
    /// What the download run provably achieved, derived from the validated journal and
    /// the plan. `verified` only after the final verification step finished; never a
    /// loaded or clean claim about the project.
    pub download_result: knx_app::property_validation::DownloadResult,
    /// A recovery was started: `state`, `completedSteps` and `totalSteps` are then the recovery's.
    pub restore_attempted: bool,
    /// The download's own frozen counters, unaffected by a later recovery.
    pub download_completed_steps: usize,
    pub download_total_steps: Option<usize>,
    /// What this process's terminal project settlement did; `notRecorded` after restart or reopen.
    pub project_settlement: super::settlement::Receipt,
}
/// Source-model pointer versus device read: lower-case hex (eight digits, or four for a
/// two-octet reply, never padded) or "refused".
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
/// Object, four lower-case hex digit start address and length of one allocation.
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct UnreadableAllocation {
    pub object: u8,
    pub address: String,
    pub length: usize,
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
