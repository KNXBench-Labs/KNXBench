//! Strict serde wire DTOs for the catalogue job protocol (P2/P3 field/null/state contract).
//!
//! Private and unwired: no route, upload, spool or drain consumes these types yet.

use serde::de::{self, DeserializeSeed, MapAccess, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::HashSet;
use std::fmt;

use super::strict::{Incarnation, JobId, Sha256, U64};

/// The closed error kinds of the protocol (P3 `Failure.kind` / `Error.kind`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum FailureKind {
    InvalidRequest,
    InvalidHandle,
    UnsupportedMediaType,
    BodyLimit,
    Unauthenticated,
    OutcomeUnavailable,
    IncarnationChanged,
    StaleOffer,
    CommandConflict,
    JobBusy,
    InvalidPhase,
    UploadNotReplayable,
    ConsentExpired,
    CatalogueUpdateStale,
    UploadLengthMismatch,
    ServerStopping,
    CounterExhausted,
    ArtifactStoreUnavailable,
    CatalogueUnavailable,
    CatalogueAdmissionLimit,
    CatalogueGrowthLimit,
    ResultLimit,
    RecordLimit,
    PublicProjectionRefused,
    UnsupportedCatalogue,
    LegacyProductDatabase,
    PackageRefused,
    WriterTimeout,
    UploadAborted,
    IoFailure,
    DatabaseFailure,
    WorkerFailed,
    ReportMismatch,
    CleanupFailed,
    OutcomeUnknown,
    Cancelled,
    ReceivingIdleTimeout,
    ReceivingTimeout,
    WorkingTimeout,
    ServerStop,
    ReadyExpired,
}

/// The profile name admitted for a selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ProfileName {
    Standard,
    Large,
}

/// A progress stage token (P3 `Progress.stage`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ProgressStage {
    Receiving,
    Hashing,
    ArchiveScan,
    Decoding,
    XmlParsing,
    SnapshotCopy,
    Rehearsal,
    Binding,
    BackupCopy,
    Install,
    Reporting,
    Cleanup,
}

/// A progress unit token (P3 `Progress.unit`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ProgressUnit {
    Bytes,
    Entries,
    Events,
    Pages,
    Records,
}

/// A lease kind token (P3 `Lease.kind`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum LeaseKind {
    Receiving,
    Working,
    Ready,
    Retention,
}

/// A commit outcome token (P3/P7 `Commit.outcome`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CommitOutcome {
    Pending,
    Committed,
    NotCommitted,
    Unknown,
}

/// A commit evidence token (P3/P7 `Commit.evidence`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CommitEvidence {
    NotAttempted,
    CommitReturnedOk,
    NoLiveMutation,
    ExplicitRollback,
    AutomaticRollbackBeforeCommit,
    AmbiguousCommit,
    UnresolvedRollback,
    WorkerLost,
}

/// A cleanup disposition token (P3 `Cleanup.disposition`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CleanupDisposition {
    Pending,
    Complete,
    Quarantined,
}

/// A cleanup failure kind (P3 `Cleanup.failureKind`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CleanupFailureKind {
    CleanupFailed,
    IoFailure,
    WorkerFailed,
}

/// A backup state token (P3 `Backup.state`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum BackupState {
    None,
    Staging,
    Published,
    Retained,
    Quarantined,
}

/// A backup publication acknowledgement (P3 `Backup.publication`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum BackupPublication {
    None,
    FileAndDirectorySyncAcknowledged,
}

/// An artifact kind (P3 `Artifact.kind`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ArtifactKind {
    Preview,
    Result,
}

/// The public projection token (P3 `Artifact.projection`; const `redactedCompleteV1`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Projection {
    RedactedCompleteV1,
}

/// The apply availability token (P3 `Apply.state`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ApplyState {
    Unavailable,
    Available,
    Accepted,
    Spent,
}

/// The top-level job state (P3 `Status.state`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum JobState {
    Running,
    Succeeded,
    Cancelled,
    Stale,
    Expired,
    Failed,
}

/// A running phase (P3 `Status.phase`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Phase {
    Receiving,
    Previewing,
    Ready,
    WaitingForWriter,
    Validating,
    BackingUp,
    Installing,
    Committing,
    Settling,
}

/// The publication gate state (P3 `Status.gate`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Gate {
    Open,
    CancelAccepted,
    CommitClaimed,
}

/// A stop cause token (P3 `Status.stopCause`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum StopCause {
    UserCancel,
    ReadyExpired,
    ReceivingIdleTimeout,
    ReceivingTimeout,
    WorkingTimeout,
    ServerStop,
    CounterExhausted,
}

/// A cancel reply outcome token (P4 `CancelReply.outcome`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CancelOutcome {
    Requested,
    AlreadyRequested,
    AlreadyTerminal,
    TooLate,
}

/// The admission state token (P4 `Admission.admission`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AdmissionState {
    Open,
    Busy,
    Stopping,
    Exhausted,
    ArtifactStoreUnavailable,
    CatalogueUnavailable,
}

/// A drain end token (P9 `CatalogueDrain.end`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DrainEnd {
    NothingRunning,
    Finished,
    TimedOut,
    SecondSignal,
}

/// A drain cleanup token (P9 `CatalogueDrain.cleanup`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DrainCleanup {
    Complete,
    Quarantined,
    Pending,
}

/// A drain commit token (P9 `CatalogueDrain.commit`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DrainCommit {
    NotApplicable,
    Pending,
    Committed,
    NotCommitted,
    Unknown,
}

/// A dispose disposition token (P4 `DisposeReply.disposition`; const `disposed`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DisposeDisposition {
    Disposed,
}

// ---------------------------------------------------------------------------
// Protocol constants
// ---------------------------------------------------------------------------

/// `schemaVersion` const.
pub const SCHEMA_VERSION: u32 = 1;
/// `policyVersion` const.
pub const POLICY_VERSION: &str = "wp043-v1";
/// The source byte cap (policy `caps.sourceBytes`).
pub const MAX_SOURCE_BYTES: u64 = 1_000_000_000;
/// The member byte cap (policy `caps.memberBytes`).
pub const MAX_MEMBER_BYTES: u64 = 1_000_000_000;
/// The standard expansion cap (policy `caps.standardExpandedBytes`).
pub const STANDARD_EXPANDED_BYTES: u64 = 4_294_967_296;
/// The large expansion cap (policy `caps.largeExpandedBytes`).
pub const LARGE_EXPANDED_BYTES: u64 = 17_179_869_184;
/// The maximum accepted JSON nesting depth (policy `caps.maxJsonDepth`).
pub const MAX_JSON_DEPTH: usize = 8;
/// The maximum display source name in UTF-8 bytes (policy `caps.sourceNameUtf8Bytes`).
pub const MAX_SOURCE_NAME_BYTES: usize = 512;

// ---------------------------------------------------------------------------
// Strict JSON admission
// ---------------------------------------------------------------------------

/// A strict wire admission/validation failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WireError {
    /// A duplicate object member appeared at some nesting level.
    DuplicateKey,
    /// Malformed JSON, trailing input, a leading BOM, a non-finite number or
    /// nesting deeper than `MAX_JSON_DEPTH`.
    InvalidJson,
    /// An unknown field, wrong type/nullability, closed-enum token, or a bad
    /// `U64`/hex identity.
    Schema,
    /// A semantic state-rule violation.
    State,
    /// A const field did not carry its exact required value.
    Const,
    /// The display source name violated its grammar.
    SourceName,
    /// Profile limits or `allowLargePackage` were inconsistent.
    Profile,
    /// Source byte bounds were violated.
    SourceBytes,
}

impl fmt::Display for WireError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let text = match self {
            WireError::DuplicateKey => "duplicate object member",
            WireError::InvalidJson => "invalid JSON",
            WireError::Schema => "schema violation",
            WireError::State => "state-rule violation",
            WireError::Const => "const field violation",
            WireError::SourceName => "source name violation",
            WireError::Profile => "profile violation",
            WireError::SourceBytes => "source byte bound violation",
        };
        f.write_str(text)
    }
}

impl std::error::Error for WireError {}

/// Recursive duplicate/depth walk; returns `DuplicateKey`/`InvalidJson` on the
/// exact violation.
struct DepthCheck {
    depth: usize,
}

impl<'de> Visitor<'de> for DepthCheck {
    type Value = ();

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("any JSON value")
    }

    fn visit_map<A>(self, mut map: A) -> Result<(), A::Error>
    where
        A: MapAccess<'de>,
    {
        if self.depth >= MAX_JSON_DEPTH {
            return Err(de::Error::custom("nesting exceeds the maximum depth"));
        }
        let mut seen = HashSet::new();
        while let Some(key) = map.next_key::<String>()? {
            if !seen.insert(key) {
                return Err(de::Error::custom("duplicate object member"));
            }
            map.next_value_seed(DepthCheck {
                depth: self.depth + 1,
            })?;
        }
        Ok(())
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<(), A::Error>
    where
        A: SeqAccess<'de>,
    {
        if self.depth >= MAX_JSON_DEPTH {
            return Err(de::Error::custom("nesting exceeds the maximum depth"));
        }
        while let Some(()) = seq.next_element_seed(DepthCheck {
            depth: self.depth + 1,
        })? {}
        Ok(())
    }

    fn visit_some<D>(self, d: D) -> Result<(), D::Error>
    where
        D: Deserializer<'de>,
    {
        d.deserialize_any(self)
    }

    fn visit_bool<E>(self, _v: bool) -> Result<(), E> {
        Ok(())
    }
    fn visit_i64<E>(self, _v: i64) -> Result<(), E> {
        Ok(())
    }
    fn visit_u64<E>(self, _v: u64) -> Result<(), E> {
        Ok(())
    }
    fn visit_f64<E>(self, _v: f64) -> Result<(), E> {
        Ok(())
    }
    fn visit_str<E>(self, _v: &str) -> Result<(), E> {
        Ok(())
    }
    fn visit_borrowed_str<E>(self, _v: &'de str) -> Result<(), E> {
        Ok(())
    }
    fn visit_string<E>(self, _v: String) -> Result<(), E> {
        Ok(())
    }
    fn visit_bytes<E>(self, _v: &[u8]) -> Result<(), E> {
        Ok(())
    }
    fn visit_borrowed_bytes<E>(self, _v: &'de [u8]) -> Result<(), E> {
        Ok(())
    }
    fn visit_unit<E>(self) -> Result<(), E> {
        Ok(())
    }
    fn visit_none<E>(self) -> Result<(), E> {
        Ok(())
    }
}

impl<'de> DeserializeSeed<'de> for DepthCheck {
    type Value = ();

    fn deserialize<D>(self, d: D) -> Result<(), D::Error>
    where
        D: Deserializer<'de>,
    {
        d.deserialize_any(self)
    }
}

/// Rejects duplicate object members at every nesting level and any input
/// nested deeper than `MAX_JSON_DEPTH`.
fn reject_duplicate_keys(s: &str) -> Result<(), WireError> {
    let mut de = serde_json::Deserializer::from_str(s);
    de.deserialize_any(DepthCheck { depth: 0 })
        .map_err(|_| WireError::DuplicateKey)?;
    de.end().map_err(|_| WireError::InvalidJson)?;
    Ok(())
}

/// Parses `T` strictly: no BOM, no duplicate keys, no unknown fields, no
/// trailing input, no non-finite numbers.
pub fn from_str<T>(s: &str) -> Result<T, WireError>
where
    T: de::DeserializeOwned,
{
    if s.starts_with('\u{feff}') {
        return Err(WireError::InvalidJson);
    }
    reject_duplicate_keys(s)?;
    serde_json::from_str::<T>(s).map_err(|_| WireError::Schema)
}

/// Serializes `T` to a compact JSON string.
pub fn to_string<T>(value: &T) -> Result<String, WireError>
where
    T: Serialize,
{
    serde_json::to_string(value).map_err(|_| WireError::Schema)
}

// ---------------------------------------------------------------------------
// Serde for the strict wire primitives
// ---------------------------------------------------------------------------

impl Serialize for U64 {
    fn serialize<S>(&self, s: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        s.collect_str(self)
    }
}

impl<'de> Deserialize<'de> for U64 {
    fn deserialize<D>(d: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(d)?;
        s.parse::<U64>()
            .map_err(|_| de::Error::custom("invalid U64"))
    }
}

macro_rules! wire_hex_serde {
    ($t:ty) => {
        impl Serialize for $t {
            fn serialize<S>(&self, s: S) -> Result<S::Ok, S::Error>
            where
                S: Serializer,
            {
                s.collect_str(self)
            }
        }

        impl<'de> Deserialize<'de> for $t {
            fn deserialize<D>(d: D) -> Result<Self, D::Error>
            where
                D: Deserializer<'de>,
            {
                let s = String::deserialize(d)?;
                s.parse::<$t>()
                    .map_err(|_| de::Error::custom("invalid hex identity"))
            }
        }
    };
}

wire_hex_serde!(JobId);
wire_hex_serde!(Incarnation);
wire_hex_serde!(Sha256);

// ---------------------------------------------------------------------------
// Wire DTOs
// ---------------------------------------------------------------------------

/// Nullable does not mean optional: serde's ordinary `Option` default would
/// otherwise silently admit a missing key in this all-keys-required protocol.
fn required_nullable<'de, D, T>(d: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<T>::deserialize(d)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Handle {
    pub server_incarnation: Incarnation,
    pub admission_serial: U64,
    pub job_id: JobId,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Profile {
    pub name: ProfileName,
    pub max_member_bytes: U64,
    pub max_expanded_bytes: U64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Source {
    pub name: String,
    pub declared_bytes: U64,
    pub received_bytes: U64,
    #[serde(deserialize_with = "required_nullable")]
    pub sha256: Option<Sha256>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Progress {
    pub stage: ProgressStage,
    pub pass: U64,
    pub unit: ProgressUnit,
    pub completed: U64,
    #[serde(deserialize_with = "required_nullable")]
    pub total: Option<U64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Lease {
    pub kind: LeaseKind,
    pub deadline_tick_ms: U64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Failure {
    pub kind: FailureKind,
    /// Always null in v1; `Option<()>` enforces a required explicit `null`.
    #[serde(deserialize_with = "required_nullable")]
    pub context: Option<()>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Commit {
    pub outcome: CommitOutcome,
    pub evidence: CommitEvidence,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Cleanup {
    pub disposition: CleanupDisposition,
    #[serde(deserialize_with = "required_nullable")]
    pub failure_kind: Option<CleanupFailureKind>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Backup {
    pub state: BackupState,
    #[serde(deserialize_with = "required_nullable")]
    pub name: Option<String>,
    #[serde(deserialize_with = "required_nullable")]
    pub identity: Option<String>,
    pub publication: BackupPublication,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Artifact {
    pub artifact_id: JobId,
    pub kind: ArtifactKind,
    pub public_sha256: Sha256,
    pub public_bytes: U64,
    pub records: U64,
    pub withheld_fields: U64,
    pub withheld_records: U64,
    pub withheld_occurrences: U64,
    pub projection: Projection,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Preview {
    pub consent_id: JobId,
    pub artifact: Artifact,
    pub package_already_installed: bool,
    pub wrapper: bool,
    pub packages: U64,
    pub programs: U64,
    pub products_added: U64,
    pub hardware2_programs_added: U64,
    pub id_conflicts: U64,
    pub growth_cliff_possible: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct JobResult {
    pub artifact: Artifact,
    pub catalogue_page_bytes: U64,
    pub next_job_input_limit_exceeded: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Apply {
    pub state: ApplyState,
    #[serde(deserialize_with = "required_nullable")]
    pub accepted_command_id: Option<JobId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Status {
    pub schema_version: u32,
    pub policy_version: String,
    pub handle: Handle,
    pub revision: U64,
    pub source: Source,
    pub profile: Profile,
    pub state: JobState,
    #[serde(deserialize_with = "required_nullable")]
    pub phase: Option<Phase>,
    pub gate: Gate,
    #[serde(deserialize_with = "required_nullable")]
    pub stop_cause: Option<StopCause>,
    #[serde(deserialize_with = "required_nullable")]
    pub progress: Option<Progress>,
    pub lease: Lease,
    pub apply: Apply,
    #[serde(deserialize_with = "required_nullable")]
    pub preview: Option<Preview>,
    #[serde(deserialize_with = "required_nullable")]
    pub result: Option<JobResult>,
    #[serde(deserialize_with = "required_nullable")]
    pub failure: Option<Failure>,
    pub commit: Commit,
    pub cleanup: Cleanup,
    pub backup: Backup,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Admission {
    pub schema_version: u32,
    pub policy_version: String,
    pub server_incarnation: Incarnation,
    pub next_admission_serial: U64,
    pub server_tick_ms: U64,
    pub admission: AdmissionState,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Create {
    pub schema_version: u32,
    pub policy_version: String,
    pub server_incarnation: Incarnation,
    pub admission_serial: U64,
    pub source_name: String,
    pub declared_bytes: U64,
    pub profile: ProfileName,
    pub allow_large_package: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ApplyCommand {
    pub schema_version: u32,
    pub command_id: JobId,
    pub consent_id: JobId,
    pub preview_artifact_id: JobId,
    pub preview_public_sha256: Sha256,
    pub confirmed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CancelCommand {
    pub schema_version: u32,
    pub command_id: JobId,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CancelReply {
    pub schema_version: u32,
    pub command_id: JobId,
    pub outcome: CancelOutcome,
    pub job: Status,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DisposeReply {
    pub schema_version: u32,
    pub handle: Handle,
    pub disposition: DisposeDisposition,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Error {
    pub schema_version: u32,
    pub kind: FailureKind,
    #[serde(deserialize_with = "required_nullable")]
    pub context: Option<()>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CatalogueDrain {
    pub schema_version: u32,
    pub end: DrainEnd,
    pub work_outstanding: bool,
    pub cleanup: DrainCleanup,
    pub commit: DrainCommit,
}

// ---------------------------------------------------------------------------
// Semantic validation
// ---------------------------------------------------------------------------

fn check_schema_version(v: u32) -> Result<(), WireError> {
    if v == SCHEMA_VERSION {
        Ok(())
    } else {
        Err(WireError::Const)
    }
}

fn check_policy_version(v: &str) -> Result<(), WireError> {
    if v == POLICY_VERSION {
        Ok(())
    } else {
        Err(WireError::Const)
    }
}

/// Validates the P3 display-source-name grammar: 1..=512 UTF-8 bytes, basename
/// only, no path separators, drive colon, NUL, controls, `.`/`..`, or leading/
/// trailing ASCII whitespace.
pub(crate) fn is_forbidden_control(c: char) -> bool {
    c.is_control()
        || matches!(c, '\u{ad}' | '\u{600}'..='\u{605}' | '\u{61c}' | '\u{6dd}' | '\u{70f}' | '\u{890}'..='\u{891}' | '\u{8e2}' | '\u{180e}' | '\u{200b}'..='\u{200f}' | '\u{202a}'..='\u{202e}' | '\u{2060}'..='\u{2064}' | '\u{2066}'..='\u{206f}' | '\u{feff}' | '\u{fff9}'..='\u{fffb}' | '\u{110bd}' | '\u{110cd}' | '\u{13430}'..='\u{1343f}' | '\u{1bca0}'..='\u{1bca3}' | '\u{1d173}'..='\u{1d17a}' | '\u{e0001}' | '\u{e0020}'..='\u{e007f}')
}

fn check_source_name(name: &str) -> Result<(), WireError> {
    if name.is_empty() || name.len() > MAX_SOURCE_NAME_BYTES {
        return Err(WireError::SourceName);
    }
    if name == "." || name == ".." {
        return Err(WireError::SourceName);
    }
    if name.starts_with([' ', '\t', '\r', '\n']) || name.ends_with([' ', '\t', '\r', '\n']) {
        return Err(WireError::SourceName);
    }
    if name.contains(['/', '\\', ':', '\0']) {
        return Err(WireError::SourceName);
    }
    if name.chars().any(is_forbidden_control) {
        return Err(WireError::SourceName);
    }
    Ok(())
}

impl Create {
    /// Validates const fields, profile consistency, source-name grammar and
    /// the declared byte bound.
    pub fn validate(&self) -> Result<(), WireError> {
        check_schema_version(self.schema_version)?;
        check_policy_version(&self.policy_version)?;
        check_source_name(&self.source_name)?;
        if self.declared_bytes.get() > MAX_SOURCE_BYTES {
            return Err(WireError::SourceBytes);
        }
        let large = self.profile == ProfileName::Large;
        if self.allow_large_package != large {
            return Err(WireError::Profile);
        }
        Ok(())
    }
}

impl ApplyCommand {
    pub fn validate(&self) -> Result<(), WireError> {
        check_schema_version(self.schema_version)?;
        if !self.confirmed {
            return Err(WireError::Const);
        }
        Ok(())
    }
}

impl CancelCommand {
    pub fn validate(&self) -> Result<(), WireError> {
        check_schema_version(self.schema_version)
    }
}

impl Source {
    /// `0 <= received <= declared <= MAX_SOURCE_BYTES`, and a sealed digest
    /// implies the exact declared length was received.
    pub fn validate(&self) -> Result<(), WireError> {
        let declared = self.declared_bytes.get();
        let received = self.received_bytes.get();
        if received > declared || declared > MAX_SOURCE_BYTES {
            return Err(WireError::SourceBytes);
        }
        if self.sha256.is_some() && received != declared {
            return Err(WireError::SourceBytes);
        }
        Ok(())
    }
}

impl Profile {
    /// The limits must match the selected name exactly.
    pub fn validate(&self) -> Result<(), WireError> {
        if self.max_member_bytes.get() != MAX_MEMBER_BYTES {
            return Err(WireError::Profile);
        }
        let expanded = self.max_expanded_bytes.get();
        let ok = match self.name {
            ProfileName::Standard => expanded == STANDARD_EXPANDED_BYTES,
            ProfileName::Large => expanded == LARGE_EXPANDED_BYTES,
        };
        if ok {
            Ok(())
        } else {
            Err(WireError::Profile)
        }
    }
}

impl Commit {
    /// Enforces the P7 outcome/evidence allowed pairs.
    pub fn validate(&self) -> Result<(), WireError> {
        let ok = match self.outcome {
            CommitOutcome::Pending => self.evidence == CommitEvidence::NotAttempted,
            CommitOutcome::Committed => self.evidence == CommitEvidence::CommitReturnedOk,
            CommitOutcome::NotCommitted => matches!(
                self.evidence,
                CommitEvidence::NoLiveMutation
                    | CommitEvidence::ExplicitRollback
                    | CommitEvidence::AutomaticRollbackBeforeCommit
            ),
            CommitOutcome::Unknown => matches!(
                self.evidence,
                CommitEvidence::AmbiguousCommit
                    | CommitEvidence::UnresolvedRollback
                    | CommitEvidence::WorkerLost
            ),
        };
        if ok {
            Ok(())
        } else {
            Err(WireError::State)
        }
    }
}

impl Cleanup {
    /// `pending`/`complete` carry a null `failureKind`; `quarantined` carries a
    /// non-null one.
    pub fn validate(&self) -> Result<(), WireError> {
        let ok = match self.disposition {
            CleanupDisposition::Pending | CleanupDisposition::Complete => {
                self.failure_kind.is_none()
            }
            CleanupDisposition::Quarantined => self.failure_kind.is_some(),
        };
        if ok {
            Ok(())
        } else {
            Err(WireError::State)
        }
    }
}

impl Backup {
    /// `none`/`staging` have null public name+identity and `none` publication;
    /// `published`/`retained` have non-null name+identity and the sync
    /// acknowledgement.
    pub fn validate(&self) -> Result<(), WireError> {
        let ok = match self.state {
            BackupState::None | BackupState::Staging => {
                self.name.is_none()
                    && self.identity.is_none()
                    && self.publication == BackupPublication::None
            }
            BackupState::Published | BackupState::Retained => {
                self.name.is_some()
                    && self.identity.is_some()
                    && self.publication == BackupPublication::FileAndDirectorySyncAcknowledged
            }
            BackupState::Quarantined => {
                // Either fully null/none or a previously witnessed publication.
                let none = self.name.is_none()
                    && self.identity.is_none()
                    && self.publication == BackupPublication::None;
                let published = self.name.is_some()
                    && self.identity.is_some()
                    && self.publication == BackupPublication::FileAndDirectorySyncAcknowledged;
                none || published
            }
        };
        if ok {
            Ok(())
        } else {
            Err(WireError::State)
        }
    }
}

impl Artifact {
    pub fn validate(&self) -> Result<(), WireError> {
        if self.projection == Projection::RedactedCompleteV1 {
            Ok(())
        } else {
            Err(WireError::Const)
        }
    }
}

impl Preview {
    pub fn validate(&self) -> Result<(), WireError> {
        self.artifact.validate()?;
        if self.artifact.kind != ArtifactKind::Preview {
            return Err(WireError::State);
        }
        if !self.growth_cliff_possible {
            return Err(WireError::Const);
        }
        Ok(())
    }
}

impl JobResult {
    pub fn validate(&self) -> Result<(), WireError> {
        self.artifact.validate()?;
        if self.artifact.kind != ArtifactKind::Result {
            return Err(WireError::State);
        }
        Ok(())
    }
}

impl Apply {
    pub fn validate(&self) -> Result<(), WireError> {
        let ok = match self.state {
            ApplyState::Unavailable | ApplyState::Available => self.accepted_command_id.is_none(),
            ApplyState::Accepted => self.accepted_command_id.is_some(),
            ApplyState::Spent => {
                // Retained command stays when spent; null is also admissible
                // when apply was never admitted.
                true
            }
        };
        if ok {
            Ok(())
        } else {
            Err(WireError::State)
        }
    }
}

impl Admission {
    pub fn validate(&self) -> Result<(), WireError> {
        check_schema_version(self.schema_version)?;
        check_policy_version(&self.policy_version)
    }
}

impl Error {
    pub fn validate(&self) -> Result<(), WireError> {
        check_schema_version(self.schema_version)
    }
}

impl CatalogueDrain {
    pub fn validate(&self) -> Result<(), WireError> {
        check_schema_version(self.schema_version)
    }
}

impl Status {
    /// Validates the complete P3 field/null/state contract.
    pub fn validate(&self) -> Result<(), WireError> {
        check_schema_version(self.schema_version)?;
        check_policy_version(&self.policy_version)?;
        self.source.validate()?;
        self.profile.validate()?;
        self.commit.validate()?;
        self.cleanup.validate()?;
        self.backup.validate()?;
        self.apply.validate()?;
        if let Some(preview) = &self.preview {
            preview.validate()?;
        }
        if let Some(result) = &self.result {
            result.validate()?;
        }
        if self
            .failure
            .as_ref()
            .is_some_and(|failure| failure.context.is_some())
        {
            return Err(WireError::Const);
        }

        // stopCause non-null iff gate == cancelAccepted.
        match self.gate {
            Gate::CancelAccepted => {
                if self.stop_cause.is_none() {
                    return Err(WireError::State);
                }
            }
            Gate::Open | Gate::CommitClaimed => {
                if self.stop_cause.is_some() {
                    return Err(WireError::State);
                }
            }
        }

        // Schema-wide committed implications must precede phase/terminal
        // dispatch: settling and failed may also retain a witnessed commit.
        if self.commit.outcome == CommitOutcome::Committed
            && (self.gate != Gate::CommitClaimed
                || self.apply.accepted_command_id.is_none()
                || self.source.sha256.is_none()
                || self.preview.is_none()
                || self.result.is_none())
        {
            return Err(WireError::State);
        }

        match self.state {
            JobState::Running => self.validate_running(),
            JobState::Succeeded => self.validate_succeeded(),
            JobState::Cancelled => self.validate_cancelled(),
            JobState::Stale => self.validate_stale(),
            JobState::Expired => self.validate_expired(),
            JobState::Failed => self.validate_failed(),
        }
    }

    fn validate_running(&self) -> Result<(), WireError> {
        let phase = self.phase.ok_or(WireError::State)?;
        if self.cleanup.disposition != CleanupDisposition::Pending {
            return Err(WireError::State);
        }
        if phase == Phase::Settling {
            // Settlement may already have a failure or a witnessed commit;
            // neither means that worker/cleanup ownership has ended.
            if self.lease.kind != LeaseKind::Working
                || self.apply.state != ApplyState::Spent
                || (self.result.is_some() && self.commit.outcome != CommitOutcome::Committed)
            {
                return Err(WireError::State);
            }
            return Ok(());
        }
        if self.failure.is_some()
            || self.result.is_some()
            || self.commit.outcome != CommitOutcome::Pending
        {
            return Err(WireError::State);
        }
        let (sealed, preview, apply, lease, gate, backup_ok) = match phase {
            Phase::Receiving => (
                false,
                false,
                ApplyState::Unavailable,
                LeaseKind::Receiving,
                Gate::Open,
                self.backup.state == BackupState::None,
            ),
            Phase::Previewing => (
                true,
                false,
                ApplyState::Unavailable,
                LeaseKind::Working,
                Gate::Open,
                self.backup.state == BackupState::None,
            ),
            Phase::Ready => (
                true,
                true,
                ApplyState::Available,
                LeaseKind::Ready,
                Gate::Open,
                self.backup.state == BackupState::None,
            ),
            Phase::WaitingForWriter | Phase::Validating => (
                true,
                true,
                ApplyState::Accepted,
                LeaseKind::Working,
                Gate::Open,
                self.backup.state == BackupState::None,
            ),
            Phase::BackingUp => (
                true,
                true,
                ApplyState::Accepted,
                LeaseKind::Working,
                Gate::Open,
                matches!(
                    self.backup.state,
                    BackupState::None | BackupState::Staging | BackupState::Published
                ),
            ),
            Phase::Installing => (
                true,
                true,
                ApplyState::Accepted,
                LeaseKind::Working,
                Gate::Open,
                self.backup.state == BackupState::Published,
            ),
            Phase::Committing => (
                true,
                true,
                ApplyState::Accepted,
                LeaseKind::Working,
                Gate::CommitClaimed,
                self.backup.state == BackupState::Published,
            ),
            Phase::Settling => unreachable!(),
        };
        if self.source.sha256.is_some() != sealed
            || self.preview.is_some() != preview
            || self.apply.state != apply
            || self.lease.kind != lease
            || self.gate != gate
            || !backup_ok
        {
            return Err(WireError::State);
        }
        Ok(())
    }

    fn validate_terminal_common(&self) -> Result<(), WireError> {
        if self.phase.is_some() || self.progress.is_some() {
            return Err(WireError::State);
        }
        if self.lease.kind != LeaseKind::Retention || self.apply.state != ApplyState::Spent {
            return Err(WireError::State);
        }
        if self.cleanup.disposition == CleanupDisposition::Pending {
            return Err(WireError::State);
        }
        if self.commit.outcome == CommitOutcome::Pending {
            return Err(WireError::State);
        }
        Ok(())
    }

    fn validate_succeeded(&self) -> Result<(), WireError> {
        self.validate_terminal_common()?;
        if self.gate != Gate::CommitClaimed {
            return Err(WireError::State);
        }
        if self.failure.is_some()
            || self.result.is_none()
            || self.preview.is_none()
            || self.source.sha256.is_none()
            || self.apply.accepted_command_id.is_none()
        {
            return Err(WireError::State);
        }
        if self.commit.outcome != CommitOutcome::Committed {
            return Err(WireError::State);
        }
        if self.apply.state != ApplyState::Spent {
            return Err(WireError::State);
        }
        match self.backup.state {
            BackupState::Retained | BackupState::Quarantined => Ok(()),
            _ => Err(WireError::State),
        }
    }

    fn validate_cancelled(&self) -> Result<(), WireError> {
        self.validate_terminal_common()?;
        if self.gate != Gate::CancelAccepted {
            return Err(WireError::State);
        }
        if self.result.is_some() {
            return Err(WireError::State);
        }
        if self.commit.outcome != CommitOutcome::NotCommitted {
            return Err(WireError::State);
        }
        if self.cleanup.disposition != CleanupDisposition::Complete {
            return Err(WireError::State);
        }
        if self.backup.state != BackupState::None {
            return Err(WireError::State);
        }
        match (self.stop_cause, self.failure.as_ref().map(|f| f.kind)) {
            (Some(StopCause::UserCancel), Some(FailureKind::Cancelled))
            | (Some(StopCause::ServerStop), Some(FailureKind::ServerStop)) => Ok(()),
            _ => Err(WireError::State),
        }
    }

    fn validate_stale(&self) -> Result<(), WireError> {
        self.validate_terminal_common()?;
        if self.gate != Gate::Open || self.stop_cause.is_some() {
            return Err(WireError::State);
        }
        if self.result.is_some() || self.preview.is_none() || self.source.sha256.is_none() {
            return Err(WireError::State);
        }
        if self.commit.outcome != CommitOutcome::NotCommitted {
            return Err(WireError::State);
        }
        if self.cleanup.disposition != CleanupDisposition::Complete {
            return Err(WireError::State);
        }
        if self.backup.state != BackupState::None {
            return Err(WireError::State);
        }
        if self.failure.as_ref().map(|f| f.kind) != Some(FailureKind::CatalogueUpdateStale) {
            return Err(WireError::State);
        }
        Ok(())
    }

    fn validate_expired(&self) -> Result<(), WireError> {
        self.validate_terminal_common()?;
        if self.gate != Gate::CancelAccepted {
            return Err(WireError::State);
        }
        if self.result.is_some() {
            return Err(WireError::State);
        }
        if self.commit.outcome != CommitOutcome::NotCommitted {
            return Err(WireError::State);
        }
        if self.cleanup.disposition != CleanupDisposition::Complete {
            return Err(WireError::State);
        }
        if self.backup.state != BackupState::None {
            return Err(WireError::State);
        }
        match (self.stop_cause, self.failure.as_ref().map(|f| f.kind)) {
            (Some(StopCause::ReadyExpired), Some(FailureKind::ReadyExpired))
            | (Some(StopCause::ReceivingIdleTimeout), Some(FailureKind::ReceivingIdleTimeout))
            | (Some(StopCause::ReceivingTimeout), Some(FailureKind::ReceivingTimeout))
            | (Some(StopCause::WorkingTimeout), Some(FailureKind::WorkingTimeout)) => Ok(()),
            _ => Err(WireError::State),
        }
    }

    fn validate_failed(&self) -> Result<(), WireError> {
        self.validate_terminal_common()?;
        if self.failure.is_none() {
            return Err(WireError::State);
        }
        if self.apply.state != ApplyState::Spent {
            return Err(WireError::State);
        }
        match self.backup.state {
            BackupState::None | BackupState::Retained | BackupState::Quarantined => {}
            _ => return Err(WireError::State),
        }
        if self.result.is_some() && self.commit.outcome != CommitOutcome::Committed {
            return Err(WireError::State);
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Error mapping
// ---------------------------------------------------------------------------

impl FailureKind {
    /// The fixed HTTP status for this error kind (policy `errors` table).
    pub const fn http_status(self) -> u16 {
        use FailureKind::*;
        match self {
            InvalidRequest | InvalidHandle | UploadLengthMismatch | UploadAborted => 400,
            Unauthenticated => 401,
            ReceivingIdleTimeout | ReceivingTimeout | WorkingTimeout => 408,
            StaleOffer | CommandConflict | JobBusy | InvalidPhase | UploadNotReplayable
            | ConsentExpired | CatalogueUpdateStale | WriterTimeout | Cancelled | ReadyExpired => {
                409
            }
            OutcomeUnavailable | IncarnationChanged => 410,
            BodyLimit => 413,
            UnsupportedMediaType => 415,
            CatalogueAdmissionLimit
            | CatalogueGrowthLimit
            | ResultLimit
            | RecordLimit
            | PublicProjectionRefused
            | UnsupportedCatalogue
            | LegacyProductDatabase
            | PackageRefused => 422,
            IoFailure | DatabaseFailure | WorkerFailed | ReportMismatch | CleanupFailed
            | OutcomeUnknown => 500,
            ServerStopping
            | CounterExhausted
            | ArtifactStoreUnavailable
            | CatalogueUnavailable
            | ServerStop => 503,
        }
    }

    /// The fixed English message for this error kind (policy `errors` table).
    pub const fn message(self) -> &'static str {
        use FailureKind::*;
        match self {
            InvalidRequest => "The request is invalid.",
            InvalidHandle => "The job handle is invalid.",
            UnsupportedMediaType => "The request media type is not supported.",
            BodyLimit => "The request body exceeds the admitted limit.",
            Unauthenticated => "Authentication is required.",
            OutcomeUnavailable => {
                "The retained outcome is unavailable; do not retry installation automatically."
            }
            IncarnationChanged => {
                "The server lifetime changed; the previous outcome is unavailable."
            }
            StaleOffer => "The admission offer is no longer current.",
            CommandConflict => "The command conflicts with the retained command.",
            JobBusy => "Another catalogue job is active.",
            InvalidPhase => "The command is not allowed in this job phase.",
            UploadNotReplayable => {
                "The upload cannot be resumed or replaced; inspect the same job."
            }
            ConsentExpired => "The preview consent has expired.",
            CatalogueUpdateStale => {
                "The catalogue or bound source changed; installation was not admitted."
            }
            UploadLengthMismatch => "The upload length does not match the declared length.",
            ServerStopping => "The server is stopping; no new catalogue operation was admitted.",
            CounterExhausted => "A bounded operation counter is exhausted; admission is closed.",
            ArtifactStoreUnavailable => {
                "Private catalogue artifacts are unavailable; admission is closed."
            }
            CatalogueUnavailable => "The managed catalogue is unavailable.",
            CatalogueAdmissionLimit => "The catalogue exceeds the job input limit.",
            CatalogueGrowthLimit => {
                "The operation exceeds the catalogue main-database growth limit."
            }
            ResultLimit => "The complete result exceeds the job result policy.",
            RecordLimit => "A required complete evidence record exceeds the job record policy.",
            PublicProjectionRefused => "Required consent evidence cannot be safely projected.",
            UnsupportedCatalogue => {
                "This catalogue configuration is outside the supported job lane."
            }
            LegacyProductDatabase => "This source requires the separate legacy import workflow.",
            PackageRefused => "The product package was refused.",
            WriterTimeout => "The catalogue writer could not be acquired within the wait policy.",
            UploadAborted => "The upload ended before sealing.",
            IoFailure => "An artifact operation failed.",
            DatabaseFailure => "A catalogue operation failed.",
            WorkerFailed => "The catalogue worker ended unexpectedly.",
            ReportMismatch => "The live install evidence disagreed with the admitted rehearsal.",
            CleanupFailed => "Owned artifacts could not be completely settled.",
            OutcomeUnknown => "The commit outcome is unknown; automatic retry is forbidden.",
            Cancelled => "Cancellation was accepted.",
            ReceivingIdleTimeout => "The upload idle deadline elapsed.",
            ReceivingTimeout => "The upload overall deadline elapsed.",
            WorkingTimeout => "The working deadline elapsed.",
            ServerStop => "The server requested operation settlement.",
            ReadyExpired => "The preview consent deadline elapsed.",
        }
    }
}

#[cfg(test)]
mod tests;
