//! A failed download's payload-free terminal diagnostic: cause, failing step and write uncertainty.
//!
//! DIAG-02 ([contract](../../../docs/contracts/download-failure-diagnostic.md)).
//! The executor's error text can name memory bytes, property values or a
//! gateway; this projection keeps only a closed cause vocabulary, the plan
//! step it stopped in (without its octets), what the plan says about the
//! closing restart and how many data octets were read back unchanged. It is
//! built once from the worker's own terminal result, so the server status,
//! the session log, the activity history and the tester's report all carry
//! the same value. A missing observation stays explicit (`unknown`, an
//! absent step), never a guess.

use knx_core::commissioning::load_state::LoadEvent;
use knx_core::commissioning::memory_download::{MemoryDownloadPlan, MemoryDownloadStep};
use knx_net::commissioning::memory_download::MemoryDownloadError;
use knx_net::commissioning::SessionError;
use serde::{Deserialize, Serialize};

/// Longest step summary accepted back from a persisted record.
pub const MAX_STEP_SUMMARY: usize = 160;
/// Largest count a reader accepts: JavaScript's safe-integer bound, so the
/// Rust and web readers admit the same stored values.
pub const MAX_COUNT: usize = (1 << 53) - 1;

/// Coarse class of a failure: what a user can act on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum FailureCategory {
    /// The plan, policy or authorisation refused before a device frame.
    PlanRefused,
    /// The device does not meet the operation's identity requirements.
    IdentityMismatch,
    /// The backup before the first write could not be taken or kept.
    BackupFailed,
    /// The device did not answer in time.
    Timeout,
    /// The connection to the device or gateway ended or was refused.
    ConnectionLost,
    /// The device refused a request (a write, or a read such as the exemplar serial).
    DeviceRefused,
    /// A written value read back differently.
    ReadBackMismatch,
    /// A load state machine did not reach the expected state.
    LoadState,
    /// The device answered something the protocol does not define.
    MalformedResponse,
    /// A KNXBench-side failure, not the device's.
    Internal,
    /// A stop requested by the user, accepted before the first change.
    Stopped,
    /// The worker ended without a terminal result.
    TaskEnded,
}

/// Stable cause code; its category follows from it ([`FailureCode::category`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum FailureCode {
    DoesNotConnectFirst,
    CheckAfterWrite,
    StopRefused,
    Excluded,
    SelfAddressed,
    NoAuthorisation,
    NotASimulator,
    AuthorisationRefused,
    LoadNotMemoryMapped,
    VerifyModeWithMemoryLoad,
    MaskMismatch,
    ManufacturerMismatch,
    PropertyMismatch,
    BackupRead,
    BackupShortRead,
    BackupNotKept,
    NoAnswer,
    TransitionTimedOut,
    Transport,
    NotConnected,
    ConnectionLost,
    ConnectRejected,
    ConnectionReleased,
    PropertyRefused,
    MemoryRefused,
    ReadBackMismatch,
    PropertyReadBackMismatch,
    NotLoaded,
    UnexpectedLoadState,
    UnknownLoadState,
    LoadErrorCode,
    LoadStateUnchanged,
    IllegalLoadTransition,
    AllocationFailed,
    MemoryLoadNotSettled,
    MemoryLoadIllegalTransition,
    MalformedProperty,
    MalformedDescriptor,
    MalformedRestartResponse,
    MalformedMemoryLoadState,
    Encode,
    Chunk,
    Lagged,
    Stopped,
    TaskEnded,
}

impl FailureCode {
    /// The executor's error, classified. Exhaustive on purpose: a new error
    /// variant does not compile until it has a code.
    pub fn of_error(error: &MemoryDownloadError) -> Self {
        use MemoryDownloadError as E;
        match error {
            E::DoesNotConnectFirst => Self::DoesNotConnectFirst,
            E::CheckAfterWrite { .. } => Self::CheckAfterWrite,
            E::MaskMismatch { .. } => Self::MaskMismatch,
            E::ManufacturerMismatch { .. } => Self::ManufacturerMismatch,
            E::PropertyMismatch { .. }
            | E::ExemplarMismatch { .. }
            | E::BackupExemplarMismatch { .. } => Self::PropertyMismatch,
            E::ExemplarElementCount { .. } => Self::MalformedProperty,
            E::NotLoaded { .. } => Self::NotLoaded,
            E::UnexpectedState { .. } => Self::UnexpectedLoadState,
            E::Session { error, .. } => Self::of_session(error),
            E::BackupRead { .. } => Self::BackupRead,
            E::BackupShortRead { .. } => Self::BackupShortRead,
            E::BackupNotKept(..) => Self::BackupNotKept,
            E::Stopped { .. } => Self::Stopped,
            E::StopRefused(..) => Self::StopRefused,
        }
    }

    /// A session error, classified; exhaustive like [`Self::of_error`].
    pub fn of_session(error: &SessionError) -> Self {
        use SessionError as S;
        match error {
            S::Transport(..) => Self::Transport,
            S::Encode(..) => Self::Encode,
            S::Excluded(..) => Self::Excluded,
            S::SelfAddressed(..) => Self::SelfAddressed,
            S::NoAuthorisation { .. } => Self::NoAuthorisation,
            S::NotASimulator { .. } => Self::NotASimulator,
            S::Refused(..) => Self::AuthorisationRefused,
            S::NotConnected => Self::NotConnected,
            S::ConnectionLost { .. } => Self::ConnectionLost,
            S::ConnectRejected { .. } => Self::ConnectRejected,
            S::NoAnswer { .. } => Self::NoAnswer,
            S::ConnectionReleased { .. } => Self::ConnectionReleased,
            S::Lagged { .. } => Self::Lagged,
            S::PropertyRefused { .. } => Self::PropertyRefused,
            S::MemoryRefused { .. } => Self::MemoryRefused,
            S::ReadBackMismatch { .. } => Self::ReadBackMismatch,
            S::PropertyReadBackMismatch { .. } => Self::PropertyReadBackMismatch,
            S::UnknownLoadState(..) => Self::UnknownLoadState,
            S::ErrorCode(..) => Self::LoadErrorCode,
            S::MalformedProperty { .. } => Self::MalformedProperty,
            S::MalformedDescriptor { .. } => Self::MalformedDescriptor,
            S::TransitionTimedOut { .. } => Self::TransitionTimedOut,
            S::StateUnchanged { .. } => Self::LoadStateUnchanged,
            S::IllegalTransition { .. } => Self::IllegalLoadTransition,
            S::Chunk(..) => Self::Chunk,
            S::AllocationFailed { .. } => Self::AllocationFailed,
            S::MalformedRestartResponse { .. } => Self::MalformedRestartResponse,
            S::LoadNotMemoryMapped { .. } => Self::LoadNotMemoryMapped,
            S::VerifyModeWithMemoryLoad => Self::VerifyModeWithMemoryLoad,
            S::MemoryLoadNotSettled { .. } => Self::MemoryLoadNotSettled,
            S::MalformedMemoryLoadState { .. } => Self::MalformedMemoryLoadState,
            S::MemoryLoadIllegalTransition { .. } => Self::MemoryLoadIllegalTransition,
        }
    }

    /// The class this code belongs to; the one place that decides it.
    pub fn category(self) -> FailureCategory {
        use FailureCategory as C;
        match self {
            Self::DoesNotConnectFirst
            | Self::CheckAfterWrite
            | Self::StopRefused
            | Self::Excluded
            | Self::SelfAddressed
            | Self::NoAuthorisation
            | Self::NotASimulator
            | Self::AuthorisationRefused
            | Self::LoadNotMemoryMapped
            | Self::VerifyModeWithMemoryLoad => C::PlanRefused,
            Self::MaskMismatch | Self::ManufacturerMismatch | Self::PropertyMismatch => {
                C::IdentityMismatch
            }
            Self::BackupRead | Self::BackupShortRead | Self::BackupNotKept => C::BackupFailed,
            Self::NoAnswer | Self::TransitionTimedOut => C::Timeout,
            Self::Transport
            | Self::NotConnected
            | Self::ConnectionLost
            | Self::ConnectRejected
            | Self::ConnectionReleased => C::ConnectionLost,
            Self::PropertyRefused | Self::MemoryRefused => C::DeviceRefused,
            Self::ReadBackMismatch | Self::PropertyReadBackMismatch => C::ReadBackMismatch,
            Self::NotLoaded
            | Self::UnexpectedLoadState
            | Self::UnknownLoadState
            | Self::LoadErrorCode
            | Self::LoadStateUnchanged
            | Self::IllegalLoadTransition
            | Self::AllocationFailed
            | Self::MemoryLoadNotSettled
            | Self::MemoryLoadIllegalTransition => C::LoadState,
            Self::MalformedProperty
            | Self::MalformedDescriptor
            | Self::MalformedRestartResponse
            | Self::MalformedMemoryLoadState => C::MalformedResponse,
            Self::Encode | Self::Chunk | Self::Lagged => C::Internal,
            Self::Stopped => C::Stopped,
            Self::TaskEnded => C::TaskEnded,
        }
    }
}

/// What kind of plan step a failure happened in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum StepKind {
    Connect,
    CompareProperty,
    LoadRecord,
    WriteMemory,
    RequireLoaded,
    Restart,
    Disconnect,
}

/// Where in the run a failure happened, relative to [`DownloadFailure::step`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum FailurePhase {
    /// Before the first plan step: the plan or session was refused, or the
    /// connection could not be set up. There is no step.
    BeforeSteps,
    /// While running `step`.
    InStep,
    /// While taking or keeping the backup that precedes `step`, which did
    /// not start.
    BackupBeforeStep,
    /// A stop landed before `step`, which did not start.
    StoppedBeforeStep,
}

/// The plan step a failure relates to, without the octets it carries.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FailedStep {
    /// 1-based, as shown.
    pub number: usize,
    /// The plan's step count.
    pub of: usize,
    pub kind: StepKind,
    /// A short, byte-free description (addresses and counts, never data).
    pub summary: String,
}

impl FailedStep {
    /// Step `number` (1-based) of `plan`, or `None` when it is out of range.
    pub fn of_plan(plan: &MemoryDownloadPlan, number: usize) -> Option<Self> {
        let step = plan.steps.get(number.checked_sub(1)?)?;
        let (kind, summary) = describe(step);
        Some(Self {
            number,
            of: plan.steps.len(),
            kind,
            summary,
        })
    }
}

/// What is known about the closing restart of a failed run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum FailureRestart {
    /// The plan has no restart step.
    NotInPlan,
    /// The run stopped before the restart step; this run sent no restart.
    NotSent,
    /// The run stopped in or after the restart step, or its end was not
    /// witnessed: whether the device restarted is unknown.
    Unknown,
}

/// One failed run's terminal diagnostic.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DownloadFailure {
    pub category: FailureCategory,
    pub code: FailureCode,
    /// How `step` relates to the failure.
    pub phase: FailurePhase,
    /// The step the failure relates to; always `None` before the first step.
    pub step: Option<FailedStep>,
    pub restart: FailureRestart,
    /// Data octets written and read back unchanged before the failure.
    /// Load records and the restart are not counted; a partial write
    /// beyond this count is possible.
    pub read_back_octets: usize,
}

impl DownloadFailure {
    /// The executor's terminal error. Its own step index wins; `last_started`
    /// (1-based) only locates errors that carry none.
    pub fn from_error(
        error: &MemoryDownloadError,
        plan: &MemoryDownloadPlan,
        last_started: Option<usize>,
        read_back_octets: usize,
    ) -> Self {
        let (phase, number) = locate(error, plan, last_started);
        Self::new(
            FailureCode::of_error(error),
            plan,
            (phase, number),
            read_back_octets,
        )
    }

    /// The session could not even be built: nothing was sent.
    pub fn from_session_refusal(error: &SessionError, plan: &MemoryDownloadPlan) -> Self {
        Self::new(
            FailureCode::of_session(error),
            plan,
            (FailurePhase::BeforeSteps, None),
            0,
        )
    }

    /// The worker ended without a terminal result after `last_started`.
    pub fn task_ended(
        plan: &MemoryDownloadPlan,
        last_started: Option<usize>,
        read_back_octets: usize,
    ) -> Self {
        let phase = match last_started {
            Some(_) => FailurePhase::InStep,
            None => FailurePhase::BeforeSteps,
        };
        Self::new(
            FailureCode::TaskEnded,
            plan,
            (phase, last_started),
            read_back_octets,
        )
    }

    fn new(
        code: FailureCode,
        plan: &MemoryDownloadPlan,
        (phase, number): (FailurePhase, Option<usize>),
        read_back_octets: usize,
    ) -> Self {
        let step = number
            .filter(|_| phase != FailurePhase::BeforeSteps)
            .and_then(|number| FailedStep::of_plan(plan, number));
        let restart_step = plan
            .steps
            .iter()
            .position(|step| matches!(step, MemoryDownloadStep::Restart))
            .map(|index| index + 1);
        // The executor runs its steps in order and ends at the first
        // failure, so a run that ended before the restart step never sent it.
        let restart = match (restart_step, code, phase, &step) {
            (None, ..) => FailureRestart::NotInPlan,
            // An unwitnessed end says nothing about frames still in flight.
            (Some(_), FailureCode::TaskEnded, ..) => FailureRestart::Unknown,
            (Some(_), _, FailurePhase::BeforeSteps, _) => FailureRestart::NotSent,
            (Some(restart), _, FailurePhase::InStep, Some(step)) if step.number < restart => {
                FailureRestart::NotSent
            }
            (
                Some(restart),
                _,
                FailurePhase::BackupBeforeStep | FailurePhase::StoppedBeforeStep,
                Some(step),
            ) if step.number <= restart => FailureRestart::NotSent,
            _ => FailureRestart::Unknown,
        };
        Self {
            category: code.category(),
            code,
            phase,
            step,
            restart,
            read_back_octets,
        }
    }

    /// Whether a value read back from storage is internally consistent:
    /// every combination a worker can produce, and nothing else.
    pub fn is_consistent(&self) -> bool {
        use FailureCategory as C;
        let phase_fits = match self.category {
            C::Stopped => self.phase == FailurePhase::StoppedBeforeStep,
            C::BackupFailed => self.phase == FailurePhase::BackupBeforeStep,
            _ => {
                matches!(self.phase, FailurePhase::BeforeSteps | FailurePhase::InStep)
                    // The restore's own backup observes the exemplar serial before any change.
                    || (self.phase == FailurePhase::BackupBeforeStep
                        && self.read_back_octets == 0
                        && matches!(
                            self.code,
                            FailureCode::PropertyMismatch | FailureCode::MalformedProperty
                        ))
            }
        };
        // A run that ended in or after its restart step never knows the restart was not sent.
        let restart_fits = !(self.restart == FailureRestart::NotSent
            && self.phase == FailurePhase::InStep
            && self
                .step
                .as_ref()
                .is_some_and(|step| step.kind == StepKind::Restart));
        self.category == self.code.category()
            && phase_fits
            && restart_fits
            && (self.phase != FailurePhase::BeforeSteps || self.step.is_none())
            && self.read_back_octets <= MAX_COUNT
            && self.step.as_ref().is_none_or(|step| {
                step.number >= 1
                    && step.number <= step.of
                    && step.of <= MAX_COUNT
                    && !step.summary.is_empty()
                    && step.summary.len() <= MAX_STEP_SUMMARY
                    // Built from addresses and counts: printable ASCII only.
                    && step.summary.bytes().all(|b| (0x20..0x7f).contains(&b))
            })
            && (self.code != FailureCode::Stopped || self.read_back_octets == 0)
    }

    /// One English line for the session log and the report summary.
    pub fn line(&self) -> String {
        let place = match (&self.step, self.phase) {
            (None, FailurePhase::BeforeSteps) => "before the first step".to_string(),
            (None, _) => "in an unknown step".to_string(),
            (Some(step), phase) => format!(
                "{} step {} of {} ({})",
                match phase {
                    FailurePhase::InStep | FailurePhase::BeforeSteps => "in",
                    FailurePhase::BackupBeforeStep => "in the backup before",
                    FailurePhase::StoppedBeforeStep => "before",
                },
                step.number,
                step.of,
                step.summary
            ),
        };
        let restart = match self.restart {
            FailureRestart::NotInPlan => "no restart in the plan",
            FailureRestart::NotSent => "restart not sent",
            FailureRestart::Unknown => "restart unknown",
        };
        format!(
            "{} {place}: {} [{}]; {} octets read back; {restart}",
            match self.category {
                FailureCategory::Stopped => "stopped",
                _ => "failed",
            },
            explanation(self.code),
            code_token(self.code),
            self.read_back_octets,
        )
    }
}

/// The code's wire token, as serialized.
pub fn code_token(code: FailureCode) -> String {
    serde_json::to_value(code)
        .ok()
        .and_then(|value| value.as_str().map(str::to_string))
        .unwrap_or_default()
}

/// A safe, readable explanation of `code`; never device data.
pub fn explanation(code: FailureCode) -> &'static str {
    use FailureCode as F;
    match code {
        F::DoesNotConnectFirst | F::CheckAfterWrite => "the download plan was refused",
        F::StopRefused => "the run was refused before it started",
        F::Excluded => "the device is on the project's exclusion list",
        F::SelfAddressed => "the target is the connection's own address",
        F::NoAuthorisation | F::AuthorisationRefused => "the write was not authorised",
        F::NotASimulator => "the target is not the authorised simulator",
        F::LoadNotMemoryMapped | F::VerifyModeWithMemoryLoad => {
            "the device's mask does not load this way"
        }
        F::MaskMismatch => "the device reports another mask version",
        F::ManufacturerMismatch => "the device reports another manufacturer",
        F::PropertyMismatch => {
            "a device property does not match the operation's identity requirements"
        }
        F::BackupRead | F::BackupShortRead => "the backup before the first write could not be read",
        F::BackupNotKept => "the backup before the first write could not be kept",
        F::NoAnswer => "the device did not answer in time",
        F::TransitionTimedOut => "a load state did not change in time",
        F::Transport => "the gateway connection failed",
        F::NotConnected => "the session was not connected",
        F::ConnectionLost | F::ConnectionReleased => "the connection to the device ended",
        F::ConnectRejected => "the connection to the device was rejected",
        F::PropertyRefused | F::MemoryRefused => "the device refused the request",
        F::ReadBackMismatch | F::PropertyReadBackMismatch => {
            "a written value read back differently"
        }
        F::NotLoaded => "a part of the application is not loaded",
        F::UnexpectedLoadState
        | F::UnknownLoadState
        | F::LoadErrorCode
        | F::LoadStateUnchanged
        | F::IllegalLoadTransition
        | F::MemoryLoadNotSettled
        | F::MemoryLoadIllegalTransition => "a load state did not reach the expected state",
        F::AllocationFailed => "the device refused a memory allocation",
        F::MalformedProperty
        | F::MalformedDescriptor
        | F::MalformedRestartResponse
        | F::MalformedMemoryLoadState => "the device answered something undefined",
        F::Encode | F::Chunk => "KNXBench could not build a frame (internal)",
        F::Lagged => "KNXBench missed bus frames; the answer may have been among them",
        F::Stopped => "stopped on request before the first change",
        F::TaskEnded => "the download task ended without a result",
    }
}

/// Where `error` happened: its own 0-based step index when it has one.
fn locate(
    error: &MemoryDownloadError,
    plan: &MemoryDownloadPlan,
    last_started: Option<usize>,
) -> (FailurePhase, Option<usize>) {
    use FailurePhase as P;
    use MemoryDownloadError as E;
    // The backup always precedes the first step that changes the device.
    let first_change = || {
        plan.steps
            .iter()
            .position(MemoryDownloadStep::changes_device)
            .map(|index| index + 1)
    };
    match error {
        // Refused by shape or admission before any frame.
        E::DoesNotConnectFirst | E::CheckAfterWrite { .. } | E::StopRefused(..) => {
            (P::BeforeSteps, None)
        }
        E::Session { index: None, .. } => (P::BeforeSteps, None),
        E::Session {
            index: Some(index), ..
        }
        | E::PropertyMismatch { index, .. }
        | E::ExemplarMismatch { index }
        | E::ExemplarElementCount {
            index,
            in_backup: false,
            ..
        }
        | E::NotLoaded { index, .. }
        | E::UnexpectedState { index, .. } => (P::InStep, Some(index + 1)),
        // Checked while connecting: the step that was running.
        E::MaskMismatch { .. } | E::ManufacturerMismatch { .. } => (P::InStep, last_started),
        E::BackupRead { index, .. }
        | E::BackupExemplarMismatch { index }
        | E::ExemplarElementCount {
            index,
            in_backup: true,
            ..
        } => (P::BackupBeforeStep, Some(index + 1)),
        E::BackupShortRead { .. } | E::BackupNotKept(..) => (P::BackupBeforeStep, first_change()),
        E::Stopped { before_step, .. } => (P::StoppedBeforeStep, Some(before_step + 1)),
    }
}

fn describe(step: &MemoryDownloadStep) -> (StepKind, String) {
    match step {
        MemoryDownloadStep::Connect => (StepKind::Connect, "connect; check identity".into()),
        MemoryDownloadStep::CompareProperty {
            object_index,
            property_id,
            ..
        } => (
            StepKind::CompareProperty,
            format!("compare property {object_index}/{property_id}"),
        ),
        // Closed vocabulary: the exemplar guard is a compare to the result shapes.
        MemoryDownloadStep::RequireExemplar(_) => (
            StepKind::CompareProperty,
            "check the device against the backed-up exemplar".into(),
        ),
        MemoryDownloadStep::LoadRecord(record) => (
            StepKind::LoadRecord,
            format!(
                "load control: {} ({})",
                event_name(record.event()),
                record.machine()
            ),
        ),
        MemoryDownloadStep::WriteMemory { address, octets } => (
            StepKind::WriteMemory,
            format!(
                "write memory {address:04X}h..{:04X}h, {} octets",
                usize::from(*address) + octets.len().saturating_sub(1),
                octets.len()
            ),
        ),
        MemoryDownloadStep::RequireLoaded(machine) => (
            StepKind::RequireLoaded,
            format!("check that the {machine} is loaded"),
        ),
        MemoryDownloadStep::Restart => (StepKind::Restart, "restart".into()),
        MemoryDownloadStep::Disconnect => (StepKind::Disconnect, "disconnect".into()),
    }
}

fn event_name(event: LoadEvent) -> &'static str {
    match event {
        LoadEvent::NoOperation => "no operation",
        LoadEvent::StartLoading => "load",
        LoadEvent::LoadCompleted => "load completed",
        LoadEvent::AdditionalLoadControls => "segment",
        LoadEvent::Unload => "unload",
    }
}

#[cfg(test)]
mod tests;
