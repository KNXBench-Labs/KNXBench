//! Runs a mask-`070nh` memory download plan against one device, step by step.
//!
//! **Direction.** *Download* here means one thing only: KNXBench writes to the
//! device, over the bus (the KNX sense: *"downloaded into the device"*, KNX
//! Architecture v03.00.02). It never means saving a file from the KNXBench
//! server to the user's computer; that is `/api/project/download`, an
//! unrelated HTTP route. See `docs/GLOSSARY.md`.
//!
//! The plan is `knx_core::commissioning::memory_download`'s and is decided
//! in full before this module sends anything. This module adds only what a
//! plan cannot know in advance: whether the device is the one the plan was
//! built for, and whether each step landed.
//!
//! - `[D]` *Configuration Procedures* (`03_05_03` v02.01.01) §3.9.2.2.2,
//!   pp. 67–68: connect, identify, then unload, load, write and complete.
//!   The identity checks all happen before the first write; a plan that asks
//!   for a check after a write is refused before anything is sent.
//! - `[D]` The load records and their state reads are MP §3.31.2's,
//!   [`ManagementSession::write_memory_load_record`].
//! - `[D]` Every data write is read back ([`ManagementSession::write_memory_region`]);
//!   MP §3.31.2 forbids Verify Mode on this mask, so the read-back is an
//!   explicit `A_Memory_Read`.
//! - `[A]` Each load record must end in the one state its event aims at:
//!   `Unloaded` after an unload, `Loading` after a load or a segment,
//!   `Loaded` after load completed. RES Table 94 permits more (an `Error`
//!   state is a legal answer), but a download that went anywhere else did
//!   not do what the plan says.
//!
//! A failure stops the run at once and nothing is undone. The machines are
//! left where the failure found them; the report says which step that was.
//!
//! One silence is not a failure: the closing Basic Restart's. `[D]` MP
//! §3.7.1.1.3, p. 80: *"The Application Layer of the Management Server
//! shall not confirm the A_Restart-service if a Basic Restart is called"*,
//! and §3.7.1.1.2, p. 78, lets the server between `t0` and `t1` *"not react
//! at all"*. §3.7.3, p. 89, adds that the reset of its communication system
//! may keep even the `T_Disconnect` off the bus. So a restart that went out
//! and heard nothing, after every machine reached `Loaded` and with nothing
//! but the disconnect left to do, is reported as
//! [`RestartOutcome::Unconfirmed`] inside an `Ok`. It is never retried here:
//! a second restart is the operator's decision (`goal-commission.md` K2).
//! `[V]` `1.1.67`, 2026-09-28, run 3 is that case.

use std::fmt;

use knx_core::commissioning::authorisation::{Authorisation, AuthorisationPlan};
use knx_core::commissioning::device_backup::{
    download_changes, written_regions, BackupRegion, DeviceBackup, OctetChange, RestoreError,
};
use knx_core::commissioning::load_control_memory::MemoryLoadStateMachine;
use knx_core::commissioning::load_state::{LoadEvent, LoadState, MaskVersion};
use knx_core::commissioning::memory::WriteLimit;
use knx_core::commissioning::memory_download::{
    machines, property_matches, MemoryDownloadPlan, MemoryDownloadStep,
};
use knx_core::commissioning::mutation::WriteScope;
use knx_core::commissioning::properties::ObjectIndex;

use knx_core::ContactableAddress;

use super::{ManagementSession, SessionError, SessionTiming};
use crate::management::ManagementTransport;

/// One step that ran, as the report tells it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StepDone {
    /// The step's index in the plan.
    pub index: usize,
    /// The step, as [`MemoryDownloadStep`]'s `Display` reads it.
    pub step: String,
    /// What the device answered, if the step reads an answer.
    pub observed: Option<String>,
}

/// One thing a running download tells its observer, in the order it
/// happens. Everything here travels from KNXBench to the device, except
/// what a step reports back from it.
///
/// Carried to the operator as it happens rather than in the final report,
/// because a download is long (`1.1.67`: three minutes) and a silent one is
/// indistinguishable from a hung one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Progress {
    /// The run is about to start.
    Started {
        /// The device being written to.
        target: knx_core::IndividualAddress,
        /// Steps in the plan.
        steps: usize,
        /// Segment octets the plan writes to the device.
        data_octets: usize,
    },
    /// A step is about to run.
    StepStarted {
        /// The step's index.
        index: usize,
        /// Steps in the plan.
        of: usize,
        /// The step, as its `Display` reads it.
        step: String,
    },
    /// One `A_Memory_Write` went to the device and was read back unchanged.
    DataWritten {
        /// The step it belongs to.
        index: usize,
        /// Where it went.
        address: u32,
        /// What went there.
        octets: Vec<u8>,
        /// Segment octets written so far, this chunk included.
        written: usize,
        /// Segment octets the plan writes in all.
        of: usize,
    },
    /// A step finished.
    StepDone(StepDone),
    /// The connection is open and the device's identity checked; this is
    /// the access the device granted, before the first write.
    Authorised {
        /// What authorisation obtained. `FreeLevelUnknown` when no key was
        /// sent (MP §3.5.1's guard skips the exchange).
        authorisation: Authorisation,
        /// Whether the level is the minimum a wrong key earns (AL §3.5.7).
        /// A suspicion, not a diagnosis.
        suspicious: bool,
    },
    /// Everything the plan is about to overwrite was read and kept
    /// ([`run_memory_download_with_backup`]); the first write follows.
    BackupTaken {
        /// Regions read.
        regions: usize,
        /// Octets read.
        octets: usize,
    },
}

/// What to tell the operator when a download failed in a way a locked
/// device also produces.
///
/// `[D]` AL §3.5.3/§3.5.4 answer an access-rights refusal with `number = 0`,
/// and §3.4.4.2 a property refusal with `nr_of_elem = 0`; on mask `070nh`,
/// where Verify Mode is forbidden (MP §3.31.2), a refused write shows up
/// only as a read-back that still holds the old octets, or as a load state
/// that did not move. None of those says *why*, so this is a hint and
/// never a diagnosis.
///
/// `[D]` AL §3.5.7: a wrong key selects the minimal level with no negative
/// response, so every hint asks for the key to be supplied or checked,
/// never for another one to be tried.
pub fn locked_device_hint(
    error: &MemoryDownloadError,
    authorisation: Option<Authorisation>,
    suspicious: bool,
) -> Option<String> {
    let refusal_shaped = matches!(
        error,
        MemoryDownloadError::UnexpectedState { .. }
            | MemoryDownloadError::Session {
                error: SessionError::MemoryRefused { .. }
                    | SessionError::PropertyRefused { .. }
                    | SessionError::ReadBackMismatch { .. }
                    | SessionError::MemoryLoadNotSettled { .. },
                ..
            }
    );
    if !refusal_shaped {
        return None;
    }
    Some(match authorisation? {
        Authorisation::FreeLevelUnknown => "no access key was sent, so the device granted \
             its free level; if it is locked, supply its key (the project's \
             Installation/@BCUKey, or a key file). A wrong key leaves less access than \
             none, so never guess one"
            .to_owned(),
        Authorisation::Granted { level } if suspicious => format!(
            "the key earned {level}, the minimum a wrong key earns (AL §3.5.7); check the \
             key rather than trying others"
        ),
        Authorisation::Granted { level } => format!(
            "the session held {level}; if the device is locked, that was not enough to \
             write: check the key rather than trying others"
        ),
    })
}

/// What a completed run did.
///
/// Not `Clone` or `PartialEq`: [`RestartOutcome::Unconfirmed`] carries the
/// [`SessionError`] itself, which is neither (see its own note).
#[derive(Debug)]
pub struct MemoryDownloadReport {
    /// Every step, in order.
    pub steps: Vec<StepDone>,
    /// The state each machine was last seen in.
    pub final_states: Vec<(MemoryLoadStateMachine, LoadState)>,
    /// Octets written as segment data, every one of them read back.
    pub data_octets: usize,
    /// What became of the plan's Basic Restart. Read this before telling
    /// anyone the device runs the new program.
    pub restart: RestartOutcome,
    /// What the device held before the first write, when the run took a
    /// backup ([`run_memory_download_with_backup`]).
    pub backup: Option<DeviceBackup>,
}

/// What became of the plan's Basic Restart.
#[derive(Debug)]
pub enum RestartOutcome {
    /// The plan has no restart step.
    NotInPlan,
    /// The device acknowledged the `A_Restart` at the Transport Layer.
    /// That is all a Basic Restart ever confirms (MP §3.7.1.1.3, p. 80):
    /// that the request arrived, not that the device restarted.
    Acknowledged {
        /// The restart's step.
        index: usize,
    },
    /// The `A_Restart` went out and nothing acknowledged it. Every machine
    /// was `Loaded` and only the disconnect was left. Whether the device
    /// restarted is unknown: power-cycle it or send a restart on the
    /// operator's say-so, never automatically.
    Unconfirmed {
        /// The restart's step.
        index: usize,
        /// The silence, as the session reported it.
        error: SessionError,
    },
}

impl RestartOutcome {
    /// Whether the device acknowledged the restart request.
    pub fn is_confirmed(&self) -> bool {
        matches!(self, RestartOutcome::Acknowledged { .. })
    }
}

impl fmt::Display for RestartOutcome {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RestartOutcome::NotInPlan => f.write_str("no restart in the plan"),
            RestartOutcome::Acknowledged { index } => {
                write!(f, "step {index}: restart acknowledged")
            }
            RestartOutcome::Unconfirmed { index, error } => write!(
                f,
                "step {index}: data loaded, restart unconfirmed ({error}); \
                 power-cycle the device or restart it on purpose"
            ),
        }
    }
}

/// Why a run stopped.
#[derive(Debug)]
pub enum MemoryDownloadError {
    /// The plan does not begin with [`MemoryDownloadStep::Connect`].
    DoesNotConnectFirst,
    /// The plan checks the device after it has already written to it.
    CheckAfterWrite {
        /// The late check.
        index: usize,
    },
    /// The device reports another mask.
    MaskMismatch {
        /// The plan's.
        expected: MaskVersion,
        /// The device's.
        found: MaskVersion,
    },
    /// The device reports another manufacturer.
    ManufacturerMismatch {
        /// The plan's.
        expected: u16,
        /// The device's.
        found: u16,
    },
    /// A property did not match the product's `InlineData`.
    PropertyMismatch {
        /// The step.
        index: usize,
        /// `ObjIdx`.
        object_index: u8,
        /// `PropId`.
        property_id: u8,
        /// What the device answered.
        found: Vec<u8>,
    },
    /// A partial download found a part that is not `Loaded`: it replaces
    /// parts of a loaded application and cannot stand in for a complete
    /// download.
    NotLoaded {
        /// The step.
        index: usize,
        /// The machine.
        machine: MemoryLoadStateMachine,
        /// Its state.
        found: LoadState,
    },
    /// A load record ended in another state than the one its event aims at.
    UnexpectedState {
        /// The step.
        index: usize,
        /// The machine.
        machine: MemoryLoadStateMachine,
        /// The event.
        event: LoadEvent,
        /// The state it should have reached.
        expected: LoadState,
        /// The state it reached.
        found: LoadState,
    },
    /// The session failed.
    Session {
        /// The step, or `None` before the first one.
        index: Option<usize>,
        /// The failure.
        error: SessionError,
    },
    /// The backup before the first write could not be read. Nothing was
    /// written.
    BackupRead {
        /// The step the backup preceded.
        index: usize,
        /// The failure.
        error: SessionError,
    },
    /// A backup read answered fewer octets than it asked for. Nothing was
    /// written.
    BackupShortRead {
        /// Where.
        address: u32,
        /// Asked for.
        asked: u8,
        /// Got.
        got: usize,
    },
    /// The caller could not keep the backup. Nothing was written.
    BackupNotKept(String),
}

impl std::error::Error for MemoryDownloadError {}

impl fmt::Display for MemoryDownloadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MemoryDownloadError::DoesNotConnectFirst => {
                f.write_str("the plan does not begin by connecting")
            }
            MemoryDownloadError::CheckAfterWrite { index } => write!(
                f,
                "step {index} checks the device after the plan has written to it"
            ),
            MemoryDownloadError::MaskMismatch { expected, found } => write!(
                f,
                "the device has mask {:04X}h, the plan is for {:04X}h",
                found.0, expected.0
            ),
            MemoryDownloadError::ManufacturerMismatch { expected, found } => write!(
                f,
                "the device is from manufacturer {found:04X}h, the plan is for {expected:04X}h"
            ),
            MemoryDownloadError::PropertyMismatch {
                index,
                object_index,
                property_id,
                found,
            } => write!(
                f,
                "step {index}: property {object_index}/{property_id} is {found:02X?}, which the product does not accept"
            ),
            MemoryDownloadError::NotLoaded {
                index,
                machine,
                found,
            } => write!(
                f,
                "step {index}: the {machine} is {found:?}, not Loaded; a partial download only \
                 replaces parts of a loaded application, so run the complete download"
            ),
            MemoryDownloadError::UnexpectedState {
                index,
                machine,
                event,
                expected,
                found,
            } => write!(
                f,
                "step {index}: the {machine} went to {found:?} on {event:?}, not {expected:?}"
            ),
            MemoryDownloadError::Session { index, error } => match index {
                Some(index) => write!(f, "step {index}: {error}"),
                None => write!(f, "{error}"),
            },
            MemoryDownloadError::BackupRead { index, error } => write!(
                f,
                "before step {index}: the backup of what the plan overwrites could not be \
                 read ({error}); nothing was written"
            ),
            MemoryDownloadError::BackupShortRead {
                address,
                asked,
                got,
            } => write!(
                f,
                "the backup read at {address:04X}h asked for {asked} octets and got {got}; \
                 nothing was written"
            ),
            MemoryDownloadError::BackupNotKept(why) => write!(
                f,
                "the backup could not be kept ({why}); nothing was written"
            ),
        }
    }
}

/// The state an event's record must leave its machine in.
fn aimed_at(event: LoadEvent) -> LoadState {
    match event {
        LoadEvent::Unload => LoadState::Unloaded,
        LoadEvent::LoadCompleted => LoadState::Loaded,
        LoadEvent::StartLoading | LoadEvent::AdditionalLoadControls | LoadEvent::NoOperation => {
            LoadState::Loading
        }
    }
}

/// Whether a restart that ended in `error` may have reached the device.
///
/// Only the silences qualify: the request was sent and nothing came back.
/// A refusal, a missing connection or an encoding failure means nothing
/// left this machine, and that is a failure like any other.
pub(super) fn restart_may_have_gone_out(error: &SessionError) -> bool {
    matches!(
        error,
        SessionError::ConnectionReleased { .. }
            | SessionError::ConnectionLost { .. }
            | SessionError::NoAnswer { .. }
            | SessionError::Lagged { .. }
    )
}

/// Whether an unacknowledged restart at `index` closes a loaded download:
/// every machine seen so far is `Loaded`, and nothing but a disconnect
/// follows it in the plan.
fn closes_a_loaded_download(
    plan: &MemoryDownloadPlan,
    index: usize,
    states: &[(MemoryLoadStateMachine, LoadState)],
) -> bool {
    !states.is_empty()
        && states.iter().all(|(_, state)| *state == LoadState::Loaded)
        && plan.steps[index + 1..]
            .iter()
            .all(|step| *step == MemoryDownloadStep::Disconnect)
}

/// Refuses a plan whose shape this module will not run.
fn check_shape(plan: &MemoryDownloadPlan) -> Result<(), MemoryDownloadError> {
    if plan.steps.first() != Some(&MemoryDownloadStep::Connect) {
        return Err(MemoryDownloadError::DoesNotConnectFirst);
    }
    let first_write = plan.steps.iter().position(|step| {
        matches!(
            step,
            MemoryDownloadStep::LoadRecord(_)
                | MemoryDownloadStep::WriteMemory { .. }
                | MemoryDownloadStep::Restart
        )
    });
    let late_check = plan.steps.iter().enumerate().position(|(index, step)| {
        matches!(
            step,
            MemoryDownloadStep::CompareProperty { .. } | MemoryDownloadStep::RequireLoaded(_)
        ) && first_write.is_some_and(|first| index > first)
    });
    match late_check {
        Some(index) => Err(MemoryDownloadError::CheckAfterWrite { index }),
        None => Ok(()),
    }
}

/// Runs `plan` through `session`, which must be authorised for
/// [`WriteScope::Download`] to the plan's device.
pub async fn run_memory_download<T: ManagementTransport>(
    session: &mut ManagementSession<'_, T>,
    plan: &MemoryDownloadPlan,
) -> Result<MemoryDownloadReport, MemoryDownloadError> {
    run_memory_download_observed(session, plan, |_| {}).await
}

/// [`run_memory_download`], telling `observe` each [`Progress`] as it
/// happens. The observer only watches: it cannot change, skip or stop a
/// step, so what is sent is the same with or without one.
/// A returned execution error closes any still-open management connection
/// best-effort; the original error is retained. This is not rollback or
/// cleanup when the future is dropped or the process is terminated.
pub async fn run_memory_download_observed<T: ManagementTransport>(
    session: &mut ManagementSession<'_, T>,
    plan: &MemoryDownloadPlan,
    observe: impl FnMut(Progress),
) -> Result<MemoryDownloadReport, MemoryDownloadError> {
    run(
        session,
        plan,
        observe,
        None::<fn(&DeviceBackup) -> Result<(), String>>,
    )
    .await
}

/// [`run_memory_download_observed`], with a backup first.
///
/// After the identity checks and before the first step that changes the
/// device, this reads every region the plan writes and the load state of
/// every machine it touches ([`DeviceBackup`]), in the same connection, and
/// hands it to `keep`. The first write happens only once `keep` returned
/// `Ok`: a backup that could not be read or kept stops the run with
/// nothing written. Reads only; the backup adds no write of its own.
///
/// `keep` is where the backup goes: `Ok` once it is kept (on disk, say),
/// `Err` with the reason when it could not be.
pub async fn run_memory_download_with_backup<T: ManagementTransport>(
    session: &mut ManagementSession<'_, T>,
    plan: &MemoryDownloadPlan,
    observe: impl FnMut(Progress),
    keep: impl FnMut(&DeviceBackup) -> Result<(), String>,
) -> Result<MemoryDownloadReport, MemoryDownloadError> {
    run(session, plan, observe, Some(keep)).await
}

/// Why [`read_what_the_plan_overwrites`] could not read.
#[derive(Debug)]
pub enum ReadBackError {
    /// The device reports another mask than the plan's.
    MaskMismatch {
        /// The plan's.
        expected: MaskVersion,
        /// The device's.
        found: MaskVersion,
    },
    /// The device reports another manufacturer than the plan's.
    ManufacturerMismatch {
        /// The plan's.
        expected: u16,
        /// The device's.
        found: u16,
    },
    /// The session failed.
    Session(SessionError),
    /// A read answered fewer octets than it asked for.
    ShortRead {
        /// Where.
        address: u32,
        /// Asked for.
        asked: u8,
        /// Got.
        got: usize,
    },
}

impl std::error::Error for ReadBackError {}

impl fmt::Display for ReadBackError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MaskMismatch { expected, found } => write!(
                f,
                "the device has mask {:04X}h, the plan is for {:04X}h; nothing was written",
                found.0, expected.0
            ),
            Self::ManufacturerMismatch { expected, found } => write!(
                f,
                "the device is from manufacturer {found:04X}h, the plan is for \
                 {expected:04X}h; nothing was written"
            ),
            Self::Session(error) => write!(f, "{error}; nothing was written"),
            Self::ShortRead {
                address,
                asked,
                got,
            } => write!(
                f,
                "the read at {address:04X}h asked for {asked} octets and got {got}; \
                 nothing was written"
            ),
        }
    }
}

/// Reads what a download of `plan` would overwrite, and nothing else: the
/// same [`DeviceBackup`] a download takes before its first write, without
/// the download.
///
/// The session should be [`ManagementSession::read_only`]: then no write
/// path exists at all, and a read-only session sends no Verify Mode write
/// on connect either. Connect, check mask and manufacturer against the plan
/// (as step 1 of a download does, `[D]` CP §3.9.2.2.2), negotiate the read
/// size, read the load states and every written region, disconnect. A
/// mismatch stops before the first memory read.
pub async fn read_what_the_plan_overwrites<T: ManagementTransport>(
    session: &mut ManagementSession<'_, T>,
    plan: &MemoryDownloadPlan,
) -> Result<DeviceBackup, ReadBackError> {
    session.adopt_mask(plan.mask);
    session.connect().await.map_err(ReadBackError::Session)?;
    let result = read_after_connect(session, plan).await;
    session.disconnect().await;
    result
}

/// What a device holds against what a download of the plan would write.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanComparison {
    /// Everything the plan would overwrite, as read, with the device's
    /// mask, manufacturer and load states.
    pub held: DeviceBackup,
    /// Every run of octets where the device and the plan differ, in
    /// address order. Empty when the device holds the plan.
    pub changes: Vec<OctetChange>,
}

impl PlanComparison {
    /// Whether the device holds every octet the plan would write.
    pub fn is_same(&self) -> bool {
        self.changes.is_empty()
    }

    /// Octets a download would change.
    pub fn differing_octets(&self) -> usize {
        self.changes.iter().map(|change| change.planned.len()).sum()
    }
}

/// Why [`compare_with_plan`] compared nothing. Every variant says, in its
/// `Display`, that nothing was written, because nothing can be.
#[derive(Debug)]
pub enum CompareError {
    /// No read-only session could be built.
    Session(SessionError),
    /// The read failed or was refused.
    Read(ReadBackError),
    /// What was read does not fit the plan's shape. The read follows the
    /// plan's own regions, so this is a defect, reported rather than
    /// trusted away.
    Shape(RestoreError),
}

impl std::error::Error for CompareError {}

impl fmt::Display for CompareError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Session(error) => write!(f, "{error}; nothing was written"),
            // `ReadBackError` says "nothing was written" itself.
            Self::Read(error) => write!(f, "{error}"),
            Self::Shape(error) => write!(f, "{error}; nothing was written"),
        }
    }
}

/// Reads what a download of `plan` would overwrite on `target` and lists
/// every octet it would change: `knx device compare`, and the server's
/// `/api/device-compare`.
///
/// The session is [`ManagementSession::read_only`], so no write path exists
/// and no Verify Mode write goes out on connect. No access key is sent
/// (`AuthorisationPlan::Skip`): a device that protects its memory against
/// reading at the free level refuses the read, and the error says so; no
/// key is ever guessed. The comparison is [`download_changes`], the same
/// plan a download would run.
pub async fn compare_with_plan<T: ManagementTransport>(
    transport: &T,
    target: ContactableAddress,
    timing: SessionTiming,
    plan: &MemoryDownloadPlan,
) -> Result<PlanComparison, CompareError> {
    let address = target.address();
    let mut session =
        ManagementSession::read_only(transport, address, AuthorisationPlan::Skip, timing)
            .map_err(CompareError::Session)?;
    let held = read_what_the_plan_overwrites(&mut session, plan)
        .await
        .map_err(CompareError::Read)?;
    let changes = download_changes(plan, address, &held).map_err(CompareError::Shape)?;
    Ok(PlanComparison { held, changes })
}

async fn read_after_connect<T: ManagementTransport>(
    session: &mut ManagementSession<'_, T>,
    plan: &MemoryDownloadPlan,
) -> Result<DeviceBackup, ReadBackError> {
    let found = session
        .read_mask_version()
        .await
        .map_err(ReadBackError::Session)?;
    if found != plan.mask {
        return Err(ReadBackError::MaskMismatch {
            expected: plan.mask,
            found,
        });
    }
    let manufacturer = session
        .read_manufacturer_id()
        .await
        .map_err(ReadBackError::Session)?;
    if manufacturer != plan.manufacturer {
        return Err(ReadBackError::ManufacturerMismatch {
            expected: plan.manufacturer,
            found: manufacturer,
        });
    }
    let limit = session
        .write_limit(None)
        .await
        .map_err(ReadBackError::Session)?;
    take_backup(session, plan, limit, 0)
        .await
        .map_err(|error| match error {
            MemoryDownloadError::BackupRead { error, .. } => ReadBackError::Session(error),
            MemoryDownloadError::BackupShortRead {
                address,
                asked,
                got,
            } => ReadBackError::ShortRead {
                address,
                asked,
                got,
            },
            other => unreachable!("take_backup returns only backup errors, not {other}"),
        })
}

/// Reads what `plan` is about to overwrite, `limit` octets at a time.
async fn take_backup<T: ManagementTransport>(
    session: &mut ManagementSession<'_, T>,
    plan: &MemoryDownloadPlan,
    limit: WriteLimit,
    index: usize,
) -> Result<DeviceBackup, MemoryDownloadError> {
    let at = |error| MemoryDownloadError::BackupRead { index, error };
    let mut load_states = Vec::new();
    for machine in machines(plan) {
        let state = session.read_memory_load_state(machine).await.map_err(at)?;
        load_states.push((machine, state));
    }
    let mut regions = Vec::new();
    for (address, length) in written_regions(plan) {
        let mut octets = Vec::with_capacity(length);
        while octets.len() < length {
            let from = u32::from(address) + octets.len() as u32;
            let asked = usize::from(limit.max_octets()).min(length - octets.len()) as u8;
            let got = session.read_memory(from, asked).await.map_err(at)?;
            if got.len() != usize::from(asked) {
                return Err(MemoryDownloadError::BackupShortRead {
                    address: from,
                    asked,
                    got: got.len(),
                });
            }
            octets.extend_from_slice(&got);
        }
        regions.push(BackupRegion { address, octets });
    }
    Ok(DeviceBackup {
        target: session.target(),
        mask: plan.mask,
        manufacturer: plan.manufacturer,
        load_states,
        regions,
    })
}

async fn run<T: ManagementTransport, K: FnMut(&DeviceBackup) -> Result<(), String>>(
    session: &mut ManagementSession<'_, T>,
    plan: &MemoryDownloadPlan,
    observe: impl FnMut(Progress),
    keep: Option<K>,
) -> Result<MemoryDownloadReport, MemoryDownloadError> {
    // An invalid offline plan must not produce a cleanup telegram either.
    check_shape(plan)?;
    let result = run_steps(session, plan, observe, keep).await;
    if result.is_err() && session.connection().is_some() {
        // Closing KNXnet/IP alone does not close the device's management
        // connection. Best effort only: preserve the original failure and
        // never retry a mutation or treat disconnect as recovery.
        session.disconnect().await;
    }
    result
}

async fn run_steps<T: ManagementTransport, K: FnMut(&DeviceBackup) -> Result<(), String>>(
    session: &mut ManagementSession<'_, T>,
    plan: &MemoryDownloadPlan,
    mut observe: impl FnMut(Progress),
    mut keep: Option<K>,
) -> Result<MemoryDownloadReport, MemoryDownloadError> {
    let of = plan.steps.len();
    let total_octets = plan.data_octets();
    let mut written_octets = 0usize;
    observe(Progress::Started {
        target: session.target(),
        steps: of,
        data_octets: total_octets,
    });
    // The session decides whether to set Verify Mode from the mask, and
    // must not on this one (MP §3.31.2). The device's own mask is read and
    // compared right after connecting, before anything is written.
    session.adopt_mask(plan.mask);
    let mut report = MemoryDownloadReport {
        steps: Vec::new(),
        final_states: Vec::new(),
        data_octets: 0,
        restart: RestartOutcome::NotInPlan,
        backup: None,
    };
    let mut limit = None;
    for (index, step) in plan.steps.iter().enumerate() {
        if step.changes_device() {
            if let Some(mut keep) = keep.take() {
                let limit = limit.expect("check_shape puts Connect first");
                let backup = match take_backup(session, plan, limit, index).await {
                    Ok(backup) => backup,
                    Err(error) => {
                        session.disconnect().await;
                        return Err(error);
                    }
                };
                if let Err(why) = keep(&backup) {
                    session.disconnect().await;
                    return Err(MemoryDownloadError::BackupNotKept(why));
                }
                observe(Progress::BackupTaken {
                    regions: backup.regions.len(),
                    octets: backup.octets(),
                });
                report.backup = Some(backup);
            }
        }
        let at = |error| MemoryDownloadError::Session {
            index: Some(index),
            error,
        };
        observe(Progress::StepStarted {
            index,
            of,
            step: step.to_string(),
        });
        let observed = match step {
            MemoryDownloadStep::Connect => {
                session.connect().await.map_err(at)?;
                let found = session.read_mask_version().await.map_err(at)?;
                if found != plan.mask {
                    session.disconnect().await;
                    return Err(MemoryDownloadError::MaskMismatch {
                        expected: plan.mask,
                        found,
                    });
                }
                let manufacturer = session.read_manufacturer_id().await.map_err(at)?;
                if manufacturer != plan.manufacturer {
                    session.disconnect().await;
                    return Err(MemoryDownloadError::ManufacturerMismatch {
                        expected: plan.manufacturer,
                        found: manufacturer,
                    });
                }
                let negotiated = session.write_limit(None).await.map_err(at)?;
                limit = Some(negotiated);
                if let Some(state) = session.connection() {
                    observe(Progress::Authorised {
                        authorisation: state.authorisation,
                        suspicious: session.authorisation_is_suspicious(),
                    });
                }
                Some(format!(
                    "mask {:04X}h, manufacturer {manufacturer:04X}h, {} octets per write",
                    found.0,
                    negotiated.max_octets()
                ))
            }
            MemoryDownloadStep::CompareProperty {
                object_index,
                property_id,
                inline_data,
            } => {
                let found = session
                    .read_property(ObjectIndex::new(*object_index), *property_id, 1, 1)
                    .await
                    .map_err(at)?;
                if !property_matches(&found, inline_data) {
                    session.disconnect().await;
                    return Err(MemoryDownloadError::PropertyMismatch {
                        index,
                        object_index: *object_index,
                        property_id: *property_id,
                        found,
                    });
                }
                Some(hex(&found))
            }
            MemoryDownloadStep::RequireLoaded(machine) => {
                let found = session.read_memory_load_state(*machine).await.map_err(at)?;
                if found != LoadState::Loaded {
                    session.disconnect().await;
                    return Err(MemoryDownloadError::NotLoaded {
                        index,
                        machine: *machine,
                        found,
                    });
                }
                Some(format!("{found:?}"))
            }
            MemoryDownloadStep::LoadRecord(record) => {
                let found = session
                    .write_memory_load_record(*record)
                    .await
                    .map_err(at)?;
                let expected = aimed_at(record.event());
                if found != expected {
                    return Err(MemoryDownloadError::UnexpectedState {
                        index,
                        machine: record.machine(),
                        event: record.event(),
                        expected,
                        found,
                    });
                }
                report
                    .final_states
                    .retain(|(machine, _)| *machine != record.machine());
                report.final_states.push((record.machine(), found));
                Some(format!("{found:?}"))
            }
            MemoryDownloadStep::WriteMemory { address, octets } => {
                let limit = limit.expect("check_shape puts Connect first");
                let chunks = session
                    .write_memory_region_observed(
                        u32::from(*address),
                        octets,
                        limit,
                        WriteScope::Download,
                        |address, chunk| {
                            written_octets += chunk.len();
                            observe(Progress::DataWritten {
                                index,
                                address,
                                octets: chunk.to_vec(),
                                written: written_octets,
                                of: total_octets,
                            });
                        },
                    )
                    .await
                    .map_err(at)?;
                report.data_octets += octets.len();
                Some(format!("{chunks} writes, each read back"))
            }
            MemoryDownloadStep::Restart => {
                // `restart_basic_as` disconnects on every path out (MP
                // §3.7.3 exception (5)), so an unconfirmed restart leaves
                // no connection behind for the disconnect step to close.
                match session.restart_basic_as(WriteScope::Download).await {
                    Ok(()) => {
                        report.restart = RestartOutcome::Acknowledged { index };
                        Some("acknowledged".to_owned())
                    }
                    Err(error)
                        if restart_may_have_gone_out(&error)
                            && closes_a_loaded_download(plan, index, &report.final_states) =>
                    {
                        let observed = format!("unconfirmed: {error}");
                        report.restart = RestartOutcome::Unconfirmed { index, error };
                        Some(observed)
                    }
                    Err(error) => return Err(at(error)),
                }
            }
            MemoryDownloadStep::Disconnect => {
                // A restart has already disconnected (MP §3.7.3 exception
                // (5)); a second `T_Disconnect` would go to a device that is
                // starting up.
                if session.connection().is_some() {
                    session.disconnect().await;
                }
                None
            }
        };
        let done = StepDone {
            index,
            step: step.to_string(),
            observed,
        };
        observe(Progress::StepDone(done.clone()));
        report.steps.push(done);
    }
    report.final_states.sort();
    Ok(report)
}

fn hex(octets: &[u8]) -> String {
    octets
        .iter()
        .map(|octet| format!("{octet:02X}"))
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use knx_core::commissioning::authorisation::AuthorisationPlan;
    use knx_core::commissioning::load_control_memory::{
        abs_data_segment, abs_task_segment, event_record, AbsoluteSegment, SegmentMemoryType,
        TaskSegment, MANAGEMENT_CONTROL_ADDRESS, MEMORY_LOAD_RECORD_OCTETS,
    };
    use knx_core::commissioning::mutation::WriteAuthorisation;
    use knx_core::commissioning::partial_memory_download::{
        derive_partial_plan, PartialDownloadParts,
    };
    use knx_core::commissioning::properties::{
        PID_HARDWARE_TYPE, PID_MANUFACTURER_ID, PID_PROGRAM_VERSION,
    };

    use super::*;
    use crate::commissioning::simulator::{Interruption, Seen, SimulatedDevice, SimulatorConfig};
    use crate::commissioning::SessionTiming;
    use knx_core::commissioning::device_backup::BackupRegion;

    const MANUFACTURER: u16 = 0x0083;
    const HARDWARE: [u8; 6] = [0, 0, 0, 0, 0x01, 0x27];

    fn fast() -> SessionTiming {
        SessionTiming {
            connection_timeout: Duration::from_millis(50),
            response_timeout: Duration::from_millis(50),
            poll_interval: Duration::from_millis(1),
            max_transition: Duration::from_millis(40),
            programming_delay: Duration::from_millis(0),
            restart_basic_t1: Duration::from_millis(1),
            restart_responsive_again: Duration::from_millis(5),
            post_restart_disconnect_wait: Duration::from_millis(5),
            programming_mode_broadcast_timeout: Duration::from_millis(20),
        }
    }

    fn device(config: SimulatorConfig) -> SimulatedDevice {
        let device = SimulatedDevice::with_config(SimulatorConfig {
            mask_version: 0x0701,
            ..config
        });
        device.preset_property(0, PID_MANUFACTURER_ID, &MANUFACTURER.to_be_bytes());
        device.preset_property(0, PID_HARDWARE_TYPE, &HARDWARE);
        device
    }

    fn session(
        device: &SimulatedDevice,
        scope: WriteScope,
    ) -> ManagementSession<'_, SimulatedDevice> {
        let authorisation =
            WriteAuthorisation::for_simulator(device.address(), scope).expect("not excluded");
        ManagementSession::authorised(device, AuthorisationPlan::Skip, fast(), authorisation)
            .expect("a simulator session")
    }

    fn record(machine: MemoryLoadStateMachine, event: LoadEvent) -> MemoryDownloadStep {
        MemoryDownloadStep::LoadRecord(event_record(machine, event).expect("an event record"))
    }

    /// The address table's download: the product's procedure, cut down to
    /// one machine and a 30-octet table.
    fn plan() -> MemoryDownloadPlan {
        let table = MemoryLoadStateMachine::AddressTable;
        let segment = AbsoluteSegment {
            start: 0x4000,
            length: 30,
            access: 0xFF,
            memory_type: SegmentMemoryType::Eeprom,
            checksum_control: true,
        };
        MemoryDownloadPlan {
            mask: MaskVersion(0x0701),
            manufacturer: MANUFACTURER,
            steps: vec![
                MemoryDownloadStep::Connect,
                MemoryDownloadStep::CompareProperty {
                    object_index: 0,
                    property_id: PID_HARDWARE_TYPE,
                    inline_data: [&HARDWARE[..], &[0, 0, 0, 0]].concat(),
                },
                record(table, LoadEvent::Unload),
                record(table, LoadEvent::StartLoading),
                MemoryDownloadStep::LoadRecord(
                    abs_data_segment(table, segment).expect("a segment record"),
                ),
                MemoryDownloadStep::WriteMemory {
                    address: 0x4000,
                    octets: vec![0x01],
                },
                MemoryDownloadStep::WriteMemory {
                    address: 0x4003,
                    octets: (0..27).collect(),
                },
                MemoryDownloadStep::LoadRecord(abs_task_segment(
                    table,
                    TaskSegment {
                        start: 0x4000,
                        pei_type: 0,
                        manufacturer: 0,
                        application: 0,
                        version: 0,
                    },
                )),
                record(table, LoadEvent::LoadCompleted),
                MemoryDownloadStep::Restart,
                MemoryDownloadStep::Disconnect,
            ],
        }
    }

    fn memory_writes(device: &SimulatedDevice) -> Vec<(u32, Vec<u8>)> {
        device
            .seen()
            .into_iter()
            .filter_map(|seen| match seen {
                Seen::MemoryWrite { address, data, .. } => Some((address, data)),
                _ => None,
            })
            .collect()
    }

    #[tokio::test]
    async fn a_plan_runs_to_loaded_and_leaves_the_masked_octets_alone() {
        let device = device(SimulatorConfig::default());
        // The individual address the plan must not touch.
        device.preset_memory(0x4001, &[0x11, 0x43]);
        let mut session = session(&device, WriteScope::Download);

        let report = run_memory_download(&mut session, &plan())
            .await
            .expect("the download runs");

        assert_eq!(
            report.final_states,
            vec![(MemoryLoadStateMachine::AddressTable, LoadState::Loaded)]
        );
        assert_eq!(report.data_octets, 28);
        assert_eq!(report.steps.len(), plan().steps.len());
        assert_eq!(device.load_state(ObjectIndex::new(1)), LoadState::Loaded);
        let memory: Vec<u8> = device
            .memory(0x4000, 30)
            .into_iter()
            .map(|octet| octet.expect("written or preset"))
            .collect();
        let mut expected = vec![0x01, 0x11, 0x43];
        expected.extend(0..27);
        assert_eq!(memory, expected);
        assert!(
            memory_writes(&device).iter().all(|(address, data)| {
                let end = address + data.len() as u32;
                end <= 0x4001 || *address >= 0x4003
            }),
            "nothing written to 4001h-4002h"
        );
        assert!(device.seen().iter().any(|seen| matches!(
            seen,
            Seen::Restart {
                restart_type: 0,
                ..
            }
        )));
    }

    #[tokio::test]
    async fn verify_mode_is_never_set() {
        let device = device(SimulatorConfig::default());
        let mut session = session(&device, WriteScope::Download);
        run_memory_download(&mut session, &plan())
            .await
            .expect("the download runs");
        assert!(!device.verify_mode());
        assert_eq!(device.device_control_reads(), 0);
    }

    #[tokio::test]
    async fn another_hardware_type_stops_the_run_before_any_write() {
        let device = device(SimulatorConfig::default());
        device.preset_property(0, PID_HARDWARE_TYPE, &[0, 0, 0, 0, 0x02, 0x39]);
        let mut session = session(&device, WriteScope::Download);

        let error = run_memory_download(&mut session, &plan())
            .await
            .expect_err("refused");

        assert!(
            matches!(
                error,
                MemoryDownloadError::PropertyMismatch { index: 1, ref found, .. }
                    if found == &[0, 0, 0, 0, 0x02, 0x39]
            ),
            "{error}"
        );
        assert!(!device.memory_was_written());
    }

    #[tokio::test]
    async fn another_manufacturer_stops_the_run_before_any_write() {
        let device = device(SimulatorConfig::default());
        device.preset_property(0, PID_MANUFACTURER_ID, &[0x00, 0x02]);
        let mut session = session(&device, WriteScope::Download);

        let error = run_memory_download(&mut session, &plan())
            .await
            .expect_err("refused");

        assert!(
            matches!(
                error,
                MemoryDownloadError::ManufacturerMismatch {
                    expected: MANUFACTURER,
                    found: 0x0002
                }
            ),
            "{error}"
        );
        assert!(!device.memory_was_written());
    }

    #[tokio::test]
    async fn another_mask_stops_the_run_before_any_write() {
        let device = SimulatedDevice::with_config(SimulatorConfig {
            mask_version: 0x0705,
            ..SimulatorConfig::default()
        });
        device.preset_property(0, PID_MANUFACTURER_ID, &MANUFACTURER.to_be_bytes());
        let mut session = session(&device, WriteScope::Download);

        let error = run_memory_download(&mut session, &plan())
            .await
            .expect_err("refused");

        assert!(
            matches!(
                error,
                MemoryDownloadError::MaskMismatch {
                    expected: MaskVersion(0x0701),
                    found: MaskVersion(0x0705)
                }
            ),
            "{error}"
        );
        assert!(!device.memory_was_written());
    }

    #[tokio::test]
    async fn a_write_the_device_does_not_keep_stops_the_run() {
        let device = device(SimulatorConfig {
            corrupt_memory_writes: true,
            ..SimulatorConfig::default()
        });
        let mut session = session(&device, WriteScope::Download);

        let error = run_memory_download(&mut session, &plan())
            .await
            .expect_err("the read-back notices");

        // The first corrupted write is a load record at 0104h, which is not
        // read back; the first data write is, at step 5.
        assert!(
            matches!(
                error,
                MemoryDownloadError::Session {
                    index: Some(_),
                    error: SessionError::ReadBackMismatch { .. }
                        | SessionError::MemoryLoadNotSettled { .. }
                        | SessionError::MemoryLoadIllegalTransition { .. }
                }
            ),
            "{error}"
        );
        assert_ne!(device.load_state(ObjectIndex::new(1)), LoadState::Loaded);
    }

    #[tokio::test]
    async fn a_failed_pre_write_property_read_closes_each_download_entry_point() {
        for entry in 0..3 {
            let device = device(SimulatorConfig::default());
            let mut session = session(&device, WriteScope::Download);
            let mut missing_property = plan();
            missing_property.steps[1] = MemoryDownloadStep::CompareProperty {
                object_index: 0,
                property_id: 0xFF,
                inline_data: vec![1],
            };
            let mut backup_called = false;
            let result = match entry {
                0 => run_memory_download(&mut session, &missing_property).await,
                1 => run_memory_download_observed(&mut session, &missing_property, |_| {}).await,
                _ => {
                    run_memory_download_with_backup(
                        &mut session,
                        &missing_property,
                        |_| {},
                        |_| {
                            backup_called = true;
                            Ok(())
                        },
                    )
                    .await
                }
            };
            assert!(matches!(
                result,
                Err(MemoryDownloadError::Session { index: Some(1), .. })
            ));
            assert!(!backup_called);
            assert!(memory_writes(&device).is_empty());
            assert!(
                session.connection().is_none(),
                "entry {entry} left a connection"
            );
            assert_eq!(
                device
                    .seen()
                    .iter()
                    .filter(|s| matches!(s, Seen::Disconnect))
                    .count(),
                1,
                "entry {entry} must attempt one transport-layer disconnect"
            );
        }
    }

    #[tokio::test]
    async fn a_failed_backed_up_mutation_disconnects_without_retrying_or_restoring() {
        let device = device(SimulatorConfig {
            corrupt_memory_writes: true,
            ..SimulatorConfig::default()
        });
        device.preset_load_state(ObjectIndex::new(1), LoadState::Loaded);
        let mut session = session(&device, WriteScope::Download);
        let mut kept = None;
        let result = run_memory_download_with_backup(
            &mut session,
            &plan(),
            |_| {},
            |backup| {
                kept = Some(backup.clone());
                Ok(())
            },
        )
        .await;
        assert!(matches!(result, Err(MemoryDownloadError::Session { .. })));
        assert!(kept.is_some(), "pre-write backup remains with the caller");
        assert!(
            !memory_writes(&device).is_empty(),
            "a mutation was attempted"
        );
        assert!(session.connection().is_none());
        let seen = device.seen();
        assert_eq!(
            seen.iter().filter(|s| matches!(s, Seen::Connect)).count(),
            1
        );
        assert_eq!(
            seen.iter()
                .filter(|s| matches!(s, Seen::Disconnect))
                .count(),
            1
        );
        let disconnect = seen
            .iter()
            .position(|s| matches!(s, Seen::Disconnect))
            .unwrap();
        assert!(!seen[disconnect + 1..].iter().any(|s| matches!(
            s,
            Seen::MemoryWrite { .. } | Seen::PropertyWrite { .. } | Seen::Connect
        )));
    }

    #[tokio::test]
    async fn a_protected_octet_in_a_write_stops_the_run() {
        let device = device(SimulatorConfig {
            protected_memory: Some((0x4003, 0x4004)),
            ..SimulatorConfig::default()
        });
        let mut session = session(&device, WriteScope::Download);

        let error = run_memory_download(&mut session, &plan())
            .await
            .expect_err("the read-back notices");

        assert!(
            matches!(error, MemoryDownloadError::Session { index: Some(6), .. }),
            "{error}"
        );
        assert_eq!(
            device.load_state(ObjectIndex::new(1)),
            LoadState::Loading,
            "left where the failure found it"
        );
    }

    #[tokio::test]
    async fn a_session_for_another_scope_writes_nothing() {
        let device = device(SimulatorConfig::default());
        let mut session = session(&device, WriteScope::Restart);

        let error = run_memory_download(&mut session, &plan())
            .await
            .expect_err("refused");

        assert!(
            matches!(
                error,
                MemoryDownloadError::Session {
                    index: Some(2),
                    error: SessionError::Refused(_)
                }
            ),
            "{error}"
        );
        assert!(!device.memory_was_written());
    }

    #[tokio::test]
    async fn a_read_only_session_writes_nothing() {
        let device = device(SimulatorConfig::default());
        let mut session = ManagementSession::read_only(
            &device,
            device.address(),
            AuthorisationPlan::Skip,
            fast(),
        )
        .expect("a session");

        let error = run_memory_download(&mut session, &plan())
            .await
            .expect_err("refused");

        assert!(
            matches!(
                error,
                MemoryDownloadError::Session {
                    index: Some(2),
                    error: SessionError::NoAuthorisation { .. }
                }
            ),
            "{error}"
        );
        assert!(!device.memory_was_written());
    }

    #[tokio::test]
    async fn a_plan_that_checks_after_writing_is_refused_before_connecting() {
        let device = device(SimulatorConfig::default());
        let mut session = session(&device, WriteScope::Download);
        let mut plan = plan();
        let check = plan.steps.remove(1);
        plan.steps.insert(3, check);

        let error = run_memory_download(&mut session, &plan)
            .await
            .expect_err("refused");

        assert!(
            matches!(error, MemoryDownloadError::CheckAfterWrite { index: 3 }),
            "{error}"
        );
        assert!(device.seen().is_empty(), "nothing sent at all");
    }

    #[tokio::test]
    async fn a_plan_that_does_not_connect_first_is_refused() {
        let device = device(SimulatorConfig::default());
        let mut session = session(&device, WriteScope::Download);
        let mut plan = plan();
        plan.steps.remove(0);

        let error = run_memory_download(&mut session, &plan)
            .await
            .expect_err("refused");

        assert!(matches!(error, MemoryDownloadError::DoesNotConnectFirst));
        assert!(device.seen().is_empty());
    }

    #[tokio::test]
    async fn an_invalid_plan_does_not_disconnect_a_callers_existing_connection() {
        let device = device(SimulatorConfig::default());
        let mut session = session(&device, WriteScope::Download);
        session.connect().await.unwrap();
        let before = device.seen();
        let mut invalid = plan();
        invalid.steps.remove(0);
        let result = run_memory_download(&mut session, &invalid).await;
        assert!(matches!(
            result,
            Err(MemoryDownloadError::DoesNotConnectFirst)
        ));
        assert!(session.connection().is_some());
        assert_eq!(device.seen(), before, "offline validation sends nothing");
        session.disconnect().await;
    }

    /// Recovery after an interrupted download is running the whole download
    /// again, and the plan's opening unload is what makes that work.
    ///
    /// `[D]` MP §3.1, p. 68: *"if an error is detected, the download shall be
    /// interrupted and an error-message shall be raised"*. There is no
    /// separate recovery procedure for it. RES Table 94, p. 296: a *Device
    /// Restart* in `Loading` leads to `Error` (recommended) or stays in
    /// `Loading` (optional), and `Unload` leads from every state, `Error`
    /// and `Loading` included, to `Unloaded`. CP §3.4.1.2.1 step 6, p. 38,
    /// opens the complete download with *"Unload all loadable Objects"*.
    /// So an interruption leaves `Loading` or `Error`, and the same plan run
    /// again ends `Loaded`.
    #[tokio::test]
    async fn the_same_plan_run_again_recovers_a_part_left_loading_or_in_error() {
        for left_in in [LoadState::Loading, LoadState::Error] {
            let device = device(SimulatorConfig::default());
            device.preset_load_state(ObjectIndex::new(1), left_in);
            let mut session = session(&device, WriteScope::Download);

            let report = run_memory_download(&mut session, &plan())
                .await
                .unwrap_or_else(|e| panic!("left in {left_in:?}: {e}"));

            assert_eq!(
                device.load_state(ObjectIndex::new(1)),
                LoadState::Loaded,
                "left in {left_in:?}: {report:?}"
            );
            assert_eq!(
                device.memory(0x4003, 27),
                (0..27).map(Some).collect::<Vec<_>>(),
                "left in {left_in:?}: the table is written again in full"
            );
        }
    }

    /// K7's optional interrupted-download recovery, exercised only in the
    /// simulator: the first run writes the first data byte, then loses the
    /// connection before the second region. No rollback happens; a retry
    /// must run the complete plan.
    #[tokio::test]
    async fn an_interrupted_run_can_be_repeated_on_the_same_simulated_device() {
        let device = device(SimulatorConfig {
            interrupt_at: Some(Interruption::MemoryWriteAt(0x4003)),
            ..SimulatorConfig::default()
        });
        let mut first = session(&device, WriteScope::Download);
        run_memory_download(&mut first, &plan())
            .await
            .expect_err("the first run loses its connection before the second region");
        assert_eq!(device.load_state(ObjectIndex::new(1)), LoadState::Loading);
        assert_eq!(device.memory(0x4000, 1), vec![Some(0x01)]);
        assert_eq!(device.memory(0x4003, 27), vec![None; 27]);
        let first_writes = memory_writes(&device).len();
        assert!(first_writes > 0, "the failed run did alter the device");

        let mut retry = session(&device, WriteScope::Download);
        let report = run_memory_download(&mut retry, &plan())
            .await
            .expect("the complete plan can recover after a one-shot interruption");
        assert_eq!(
            report.final_states,
            vec![(MemoryLoadStateMachine::AddressTable, LoadState::Loaded)]
        );
        assert_eq!(device.load_state(ObjectIndex::new(1)), LoadState::Loaded);
        assert!(
            memory_writes(&device).len() > first_writes,
            "the retry wrote data again"
        );
        assert_eq!(
            device.memory(0x4003, 27),
            (0..27).map(Some).collect::<Vec<_>>()
        );
    }

    #[tokio::test]
    async fn a_permitted_state_that_is_not_the_aim_stops_the_run() {
        // RES Table 94: `Start Loading` from `Error` stays in `Error`. The
        // session accepts that as the correct answer; the download must not.
        let device = device(SimulatorConfig::default());
        device.preset_load_state(ObjectIndex::new(1), LoadState::Error);
        let mut session = session(&device, WriteScope::Download);
        let mut plan = plan();
        plan.steps.remove(2); // no unload first

        let error = run_memory_download(&mut session, &plan)
            .await
            .expect_err("refused");

        assert!(
            matches!(
                error,
                MemoryDownloadError::UnexpectedState {
                    index: 2,
                    machine: MemoryLoadStateMachine::AddressTable,
                    event: LoadEvent::StartLoading,
                    expected: LoadState::Loading,
                    found: LoadState::Error,
                }
            ),
            "{error}"
        );
        assert!(
            memory_writes(&device)
                .iter()
                .all(|(address, _)| *address == 0x0104),
            "no segment data after a failed load record"
        );
        assert!(
            !device
                .seen()
                .iter()
                .any(|seen| matches!(seen, Seen::Restart { .. })),
            "no restart after a failed load"
        );
    }

    /// K2: `1.1.67`, 2026-09-28, run 3. Every data write read back, every
    /// load state `Loaded`, and the closing Basic Restart never got its
    /// `T_ACK`. MP §3.7.1.1.2, p. 78 lets the server *"not react at all"*
    /// and §3.7.1.1.3, p. 80 forbids it an AL confirmation, so the silence
    /// is not a failed download. It is not a confirmed restart either.
    #[tokio::test]
    async fn an_unacknowledged_closing_restart_is_loaded_but_unconfirmed() {
        let device = device(SimulatorConfig {
            restart_unanswered: true,
            ..SimulatorConfig::default()
        });
        let mut session = session(&device, WriteScope::Download);

        let report = run_memory_download(&mut session, &plan())
            .await
            .expect("the data is loaded, so this is not a failed download");

        assert_eq!(
            report.final_states,
            vec![(MemoryLoadStateMachine::AddressTable, LoadState::Loaded)]
        );
        assert_eq!(report.data_octets, 28);
        assert!(
            matches!(
                report.restart,
                RestartOutcome::Unconfirmed {
                    index: 9,
                    error: SessionError::ConnectionReleased { .. }
                }
            ),
            "{:?}",
            report.restart
        );
        assert!(!report.restart.is_confirmed());
        // `1.1.67`'s trace (2026-09-28) printed "no T_ACK for T_ACK for
        // A_Restart": the message must name the request once.
        let shown = report.restart.to_string();
        assert!(
            shown.contains("no T_ACK for A_Restart (Basic Restart)"),
            "{shown}"
        );
        assert!(!shown.contains("T_ACK for T_ACK"), "{shown}");
        let seqs = device.unanswered_restart_seqs();
        assert_eq!(
            seqs.len(),
            usize::from(crate::commissioning::MAX_TRANSMISSIONS)
        );
        assert!(
            seqs.iter().all(|seq| *seq == seqs[0]),
            "TL's repetitions of one request, never a second request: {seqs:?}"
        );
        assert_eq!(
            report.steps.len(),
            plan().steps.len(),
            "disconnect still ran"
        );
        assert!(session.connection().is_none());
    }

    #[tokio::test]
    async fn an_acknowledged_closing_restart_is_confirmed() {
        let device = device(SimulatorConfig::default());
        let mut session = session(&device, WriteScope::Download);
        let report = run_memory_download(&mut session, &plan())
            .await
            .expect("the download runs");
        assert!(
            matches!(report.restart, RestartOutcome::Acknowledged { index: 9 }),
            "{:?}",
            report.restart
        );
        assert!(report.restart.is_confirmed());
        assert!(device.unanswered_restart_seqs().is_empty());
    }

    /// The unconfirmed outcome is only for a restart after a complete load.
    /// A plan with anything but a disconnect after its restart has not
    /// finished, and the run stops as it always did.
    #[tokio::test]
    async fn an_unacknowledged_restart_before_the_end_still_fails() {
        let device = device(SimulatorConfig {
            restart_unanswered: true,
            ..SimulatorConfig::default()
        });
        let mut session = session(&device, WriteScope::Download);
        let mut plan = plan();
        plan.steps.insert(
            10,
            MemoryDownloadStep::WriteMemory {
                address: 0x4003,
                octets: vec![0],
            },
        );

        let error = run_memory_download(&mut session, &plan)
            .await
            .expect_err("a step after the restart is left undone");

        assert!(
            matches!(
                error,
                MemoryDownloadError::Session {
                    index: Some(9),
                    error: SessionError::ConnectionReleased { .. }
                }
            ),
            "{error}"
        );
    }

    /// Nor for a restart whose machines are not all `Loaded`.
    #[tokio::test]
    async fn an_unacknowledged_restart_without_loaded_machines_still_fails() {
        let device = device(SimulatorConfig {
            restart_unanswered: true,
            ..SimulatorConfig::default()
        });
        let mut session = session(&device, WriteScope::Download);
        let mut plan = plan();
        plan.steps.remove(8); // no LoadCompleted: the table stays Loading

        let error = run_memory_download(&mut session, &plan)
            .await
            .expect_err("a table left in Loading is not a loaded download");

        assert!(
            matches!(
                error,
                MemoryDownloadError::Session {
                    index: Some(8),
                    error: SessionError::ConnectionReleased { .. }
                }
            ),
            "{error}"
        );
    }

    /// A refused restart never left the machine: that is a failure, not an
    /// unconfirmed restart.
    #[test]
    fn a_restart_that_was_never_sent_is_not_unconfirmed() {
        assert!(!restart_may_have_gone_out(&SessionError::NotConnected));
        assert!(!restart_may_have_gone_out(&SessionError::Refused(
            knx_core::commissioning::mutation::AuthorisationRefused::WrongScope {
                authorised: WriteScope::Download,
                attempted: WriteScope::Restart,
            }
        )));
        assert!(restart_may_have_gone_out(
            &SessionError::ConnectionReleased {
                waiting_for: "x",
                each: Duration::from_millis(1),
                attempts: 4,
            }
        ));
        assert!(restart_may_have_gone_out(&SessionError::ConnectionLost {
            during: "x"
        }));
        assert!(restart_may_have_gone_out(&SessionError::Lagged {
            waiting_for: "x"
        }));
        assert!(restart_may_have_gone_out(&SessionError::NoAnswer {
            waiting_for: "x",
            each: Duration::from_millis(1),
            attempts: 1,
        }));
    }

    /// A device locked above its free level refuses the download that
    /// carries no key, and the operator learns why without a key being
    /// guessed.
    #[tokio::test]
    async fn a_locked_device_refuses_the_free_level_and_the_hint_says_why() {
        let device = device(SimulatorConfig {
            free_access_level: 3,
            key: Some(0x1234_5678),
            key_level: 1,
            write_requires_level: 2,
            ..SimulatorConfig::default()
        });
        let mut session = session(&device, WriteScope::Download);
        let mut granted = None;
        let error = run_memory_download_observed(&mut session, &plan(), |p| {
            if let Progress::Authorised {
                authorisation,
                suspicious,
            } = p
            {
                granted = Some((authorisation, suspicious));
            }
        })
        .await
        .expect_err("the free level may not write");
        let (authorisation, suspicious) = granted.expect("reported before the first write");
        assert_eq!(authorisation, Authorisation::FreeLevelUnknown);
        let hint = locked_device_hint(&error, Some(authorisation), suspicious)
            .expect("a refusal-shaped failure without a key earns the hint");
        assert!(hint.contains("never guess"), "{hint}");
        assert!(!hint.contains("305419896") && !hint.contains("12345678"));
        assert_ne!(device.load_state(ObjectIndex::new(1)), LoadState::Loaded);
    }

    /// The same locked device, with its key: MP §3.5.2 on a BIM M112 asks
    /// for the free level, finds it is not the highest, sends the key and
    /// keeps it, and the download completes.
    #[tokio::test]
    async fn the_right_key_unlocks_the_same_device_through_the_two_key_procedure() {
        let device = device(SimulatorConfig {
            free_access_level: 3,
            key: Some(0x1234_5678),
            key_level: 1,
            write_requires_level: 2,
            ..SimulatorConfig::default()
        });
        let authorisation =
            WriteAuthorisation::for_simulator(device.address(), WriteScope::Download)
                .expect("not excluded");
        let key = knx_core::commissioning::authorisation::AccessKey::new(0x1234_5678)
            .expect("not the sentinel");
        let mut session = ManagementSession::authorised(
            &device,
            AuthorisationPlan::WithKey(key),
            fast(),
            authorisation,
        )
        .expect("a simulator session")
        .with_two_key_extension();
        let mut granted = None;
        let report = run_memory_download_observed(&mut session, &plan(), |p| {
            if let Progress::Authorised { authorisation, .. } = p {
                granted = Some(authorisation);
            }
        })
        .await
        .expect("the key's level may write");
        assert_eq!(device.authorize_requests(), 2, "free key, then the key");
        assert_eq!(
            granted,
            Some(Authorisation::Granted {
                level: knx_core::commissioning::authorisation::AccessLevel::from_octet(1)
            })
        );
        assert_eq!(
            report.final_states,
            vec![(MemoryLoadStateMachine::AddressTable, LoadState::Loaded)]
        );
    }

    #[test]
    fn the_hint_only_answers_refusal_shaped_failures() {
        let refused = MemoryDownloadError::Session {
            index: Some(5),
            error: SessionError::MemoryRefused { address: 0x4000 },
        };
        let not_refused = MemoryDownloadError::MaskMismatch {
            expected: MaskVersion(0x0701),
            found: MaskVersion(0x07B0),
        };
        let free = Some(Authorisation::FreeLevelUnknown);
        assert!(locked_device_hint(&refused, free, false).is_some());
        assert!(locked_device_hint(&not_refused, free, false).is_none());
        assert!(
            locked_device_hint(&refused, None, false).is_none(),
            "never connected"
        );
        let minimum = Some(Authorisation::Granted {
            level: knx_core::commissioning::authorisation::AccessLevel::MINIMUM_OF_SIXTEEN,
        });
        let hint = locked_device_hint(&refused, minimum, true).expect("a hint");
        assert!(hint.contains("wrong key") && hint.contains("rather than trying others"));
    }

    /// The operator sees every step start and finish, and every octet that
    /// goes to the device, in order, while the run is still going.
    #[tokio::test]
    async fn progress_shows_every_step_and_every_octet_sent() {
        let device = device(SimulatorConfig::default());
        let mut session = session(&device, WriteScope::Download);
        let mut seen = Vec::new();

        let report = run_memory_download_observed(&mut session, &plan(), |p| seen.push(p))
            .await
            .expect("the download runs");

        let steps = plan().steps.len();
        assert_eq!(
            seen.first(),
            Some(&Progress::Started {
                target: device.address(),
                steps,
                data_octets: 28,
            })
        );
        let started: Vec<usize> = seen
            .iter()
            .filter_map(|p| match p {
                Progress::StepStarted { index, of, .. } => {
                    assert_eq!(*of, steps);
                    Some(*index)
                }
                _ => None,
            })
            .collect();
        assert_eq!(started, (0..steps).collect::<Vec<_>>());
        let done: Vec<&StepDone> = seen
            .iter()
            .filter_map(|p| match p {
                Progress::StepDone(done) => Some(done),
                _ => None,
            })
            .collect();
        assert_eq!(done, report.steps.iter().collect::<Vec<_>>());

        // Exactly the octets the device received, at the addresses it
        // received them, with a running total that ends at the plan's.
        let data: Vec<(u32, Vec<u8>, usize)> = seen
            .iter()
            .filter_map(|p| match p {
                Progress::DataWritten {
                    address,
                    octets,
                    written,
                    of,
                    ..
                } => {
                    assert_eq!(*of, 28);
                    Some((*address, octets.clone(), *written))
                }
                _ => None,
            })
            .collect();
        let shown: Vec<(u32, Vec<u8>)> = data.iter().map(|(a, o, _)| (*a, o.clone())).collect();
        let sent: Vec<(u32, Vec<u8>)> = memory_writes(&device)
            .into_iter()
            .filter(|(address, _)| *address != 0x0104)
            .collect();
        assert_eq!(shown, sent);
        assert_eq!(data.last().map(|(_, _, total)| *total), Some(28));
        assert!(data.windows(2).all(|w| w[0].2 < w[1].2));

        // Each data chunk is shown inside its own step, after it starts and
        // before it finishes.
        let position = |wanted: &dyn Fn(&Progress) -> bool| seen.iter().position(wanted).unwrap();
        let first_data = position(&|p| matches!(p, Progress::DataWritten { index: 5, .. }));
        assert!(position(&|p| matches!(p, Progress::StepStarted { index: 5, .. })) < first_data);
        assert!(first_data < position(&|p| matches!(p, Progress::StepDone(d) if d.index == 5)));
    }

    /// A failing run has told the operator everything up to the failure,
    /// and nothing after it.
    #[tokio::test]
    async fn progress_stops_where_the_run_stops() {
        let device = device(SimulatorConfig {
            protected_memory: Some((0x4003, 0x4004)),
            ..SimulatorConfig::default()
        });
        let mut session = session(&device, WriteScope::Download);
        let mut seen = Vec::new();

        run_memory_download_observed(&mut session, &plan(), |p| seen.push(p))
            .await
            .expect_err("the read-back notices");

        assert!(matches!(
            seen.last(),
            Some(Progress::StepStarted { index: 6, .. })
        ));
        assert!(!seen
            .iter()
            .any(|p| matches!(p, Progress::StepDone(d) if d.index >= 6)));
    }

    /// Watching changes nothing: the same frames go out with or without an
    /// observer.
    #[tokio::test]
    async fn an_observer_does_not_change_what_is_sent() {
        let quiet = device(SimulatorConfig::default());
        run_memory_download(&mut session(&quiet, WriteScope::Download), &plan())
            .await
            .expect("runs");
        let watched = device(SimulatorConfig::default());
        run_memory_download_observed(
            &mut session(&watched, WriteScope::Download),
            &plan(),
            |_| {},
        )
        .await
        .expect("runs");
        assert_eq!(quiet.seen(), watched.seen());
    }

    /// K15: a device already carrying `A-0027-15` with all three parts
    /// loaded, and the partial plan CP §3.9.2.4 derives from [`plan`] plus
    /// an application part.
    fn loaded_device() -> SimulatedDevice {
        let device = device(SimulatorConfig::default());
        device.preset_property(3, PID_PROGRAM_VERSION, &[0x00, 0x83, 0x00, 0x27, 0x15]);
        for machine in MemoryLoadStateMachine::ALL {
            device.preset_load_state(ObjectIndex::new(machine.type_number()), LoadState::Loaded);
        }
        device
    }

    fn complete_with_application() -> MemoryDownloadPlan {
        let application = MemoryLoadStateMachine::ApplicationProgram;
        let mut plan = plan();
        let tail = plan.steps.split_off(plan.steps.len() - 2);
        plan.steps.insert(3, record(application, LoadEvent::Unload));
        plan.steps.extend([
            record(application, LoadEvent::StartLoading),
            MemoryDownloadStep::LoadRecord(
                abs_data_segment(
                    application,
                    AbsoluteSegment {
                        start: 0x4400,
                        length: 8,
                        access: 0xFF,
                        memory_type: SegmentMemoryType::Eeprom,
                        checksum_control: true,
                    },
                )
                .expect("a segment record"),
            ),
            MemoryDownloadStep::WriteMemory {
                address: 0x4400,
                octets: vec![0xA5; 8],
            },
            MemoryDownloadStep::LoadRecord(abs_task_segment(
                application,
                TaskSegment {
                    start: 0x4400,
                    pei_type: 1,
                    manufacturer: MANUFACTURER,
                    application: 0x0027,
                    version: 0x15,
                },
            )),
            record(application, LoadEvent::LoadCompleted),
        ]);
        plan.steps.extend(tail);
        plan
    }

    fn partial(parts: PartialDownloadParts) -> MemoryDownloadPlan {
        derive_partial_plan(&complete_with_application(), parts)
            .expect("a partial plan")
            .plan
    }

    const PARAMETERS: PartialDownloadParts = PartialDownloadParts {
        parameters: true,
        group_addresses: false,
    };

    fn events_sent(device: &SimulatedDevice) -> Vec<u8> {
        memory_writes(device)
            .into_iter()
            .filter(|(address, data)| {
                *address == u32::from(MANAGEMENT_CONTROL_ADDRESS)
                    && data.len() == MEMORY_LOAD_RECORD_OCTETS
            })
            .map(|(_, data)| data[0])
            .collect()
    }

    #[tokio::test]
    async fn a_parameters_partial_download_rewrites_only_the_parameters() {
        let device = loaded_device();
        let mut session = session(&device, WriteScope::Download);
        let report = run_memory_download(&mut session, &partial(PARAMETERS))
            .await
            .expect("the partial download runs");
        // Start Loading (31h) and Load Completed (32h) of the application
        // only: no unload (34h), no allocation (33h), no table event.
        assert_eq!(events_sent(&device), vec![0x31, 0x32]);
        let data: Vec<_> = memory_writes(&device)
            .into_iter()
            .filter(|(address, _)| *address != u32::from(MANAGEMENT_CONTROL_ADDRESS))
            .collect();
        assert_eq!(data, vec![(0x4400, vec![0xA5; 8])]);
        assert_eq!(
            report.final_states,
            vec![(
                MemoryLoadStateMachine::ApplicationProgram,
                LoadState::Loaded
            )]
        );
        for machine in MemoryLoadStateMachine::ALL {
            assert_eq!(
                device.load_state(ObjectIndex::new(machine.type_number())),
                LoadState::Loaded,
                "{machine}"
            );
        }
    }

    #[tokio::test]
    async fn a_partial_download_to_another_application_writes_nothing() {
        let device = loaded_device();
        device.preset_property(3, PID_PROGRAM_VERSION, &[0x00, 0x83, 0x00, 0x27, 0x14]);
        let mut session = session(&device, WriteScope::Download);
        let error = run_memory_download(&mut session, &partial(PARAMETERS))
            .await
            .expect_err("another application version");
        assert!(
            matches!(
                error,
                MemoryDownloadError::PropertyMismatch {
                    object_index: 3,
                    property_id: PID_PROGRAM_VERSION,
                    ..
                }
            ),
            "{error}"
        );
        assert!(memory_writes(&device).is_empty());
    }

    #[tokio::test]
    async fn a_partial_download_over_an_unloaded_part_writes_nothing() {
        for state in [LoadState::Unloaded, LoadState::Loading, LoadState::Error] {
            let device = loaded_device();
            device.preset_load_state(ObjectIndex::new(1), state);
            let mut session = session(&device, WriteScope::Download);
            let error = run_memory_download(
                &mut session,
                &partial(PartialDownloadParts {
                    parameters: true,
                    group_addresses: true,
                }),
            )
            .await
            .expect_err("a part that is not loaded");
            assert!(
                matches!(
                    error,
                    MemoryDownloadError::NotLoaded {
                        machine: MemoryLoadStateMachine::AddressTable,
                        found,
                        ..
                    } if found == state
                ),
                "{state:?}: {error}"
            );
            assert!(memory_writes(&device).is_empty(), "{state:?}");
            assert!(error.to_string().contains("run the complete download"));
        }
    }

    #[tokio::test]
    async fn a_group_address_partial_download_leaves_the_application_loaded() {
        let device = loaded_device();
        let mut session = session(&device, WriteScope::Download);
        run_memory_download(
            &mut session,
            &partial(PartialDownloadParts {
                parameters: false,
                group_addresses: true,
            }),
        )
        .await
        .expect("the partial download runs");
        assert!(events_sent(&device).iter().all(|octet| octet >> 4 == 1));
        assert!(memory_writes(&device)
            .iter()
            .all(|(address, _)| *address < 0x4400));
        assert_eq!(device.load_state(ObjectIndex::new(3)), LoadState::Loaded);
    }

    #[tokio::test]
    async fn a_require_loaded_after_a_write_is_refused_before_connecting() {
        let device = loaded_device();
        let mut plan = partial(PARAMETERS);
        plan.steps.insert(
            plan.steps.len() - 2,
            MemoryDownloadStep::RequireLoaded(MemoryLoadStateMachine::AddressTable),
        );
        let mut session = session(&device, WriteScope::Download);
        let error = run_memory_download(&mut session, &plan)
            .await
            .expect_err("a late check");
        assert!(
            matches!(error, MemoryDownloadError::CheckAfterWrite { .. }),
            "{error}"
        );
        assert!(device.seen().is_empty());
    }

    #[test]
    fn every_event_aims_at_one_state() {
        assert_eq!(aimed_at(LoadEvent::Unload), LoadState::Unloaded);
        assert_eq!(aimed_at(LoadEvent::StartLoading), LoadState::Loading);
        assert_eq!(
            aimed_at(LoadEvent::AdditionalLoadControls),
            LoadState::Loading
        );
        assert_eq!(aimed_at(LoadEvent::LoadCompleted), LoadState::Loaded);
    }

    // ------------------------------------------------------------ backup

    /// The device before the download: an old address-table image and a
    /// loaded machine.
    fn old_device() -> SimulatedDevice {
        let device = device(SimulatorConfig::default());
        device.preset_memory(0x4000, &[0x07, 0x11, 0x43]);
        device.preset_memory(0x4003, &(100..127).collect::<Vec<u8>>());
        device.preset_load_state(ObjectIndex::new(1), LoadState::Loaded);
        device
    }

    fn memory_reads(device: &SimulatedDevice) -> Vec<(u32, u8)> {
        device
            .seen()
            .into_iter()
            .filter_map(|seen| match seen {
                Seen::MemoryRead {
                    address, number, ..
                } => Some((address, number)),
                _ => None,
            })
            .collect()
    }

    #[tokio::test]
    async fn the_backup_holds_the_old_octets_and_is_kept_before_the_first_write() {
        let device = old_device();
        let mut session = session(&device, WriteScope::Download);
        let mut kept = Vec::new();
        let mut seen = Vec::new();
        let writes_when_kept = std::cell::Cell::new(usize::MAX);
        let report = run_memory_download_with_backup(
            &mut session,
            &plan(),
            |p| seen.push(p),
            &mut |backup: &DeviceBackup| {
                writes_when_kept.set(memory_writes(&device).len());
                kept.push(backup.clone());
                Ok(())
            },
        )
        .await
        .expect("the download runs");

        assert_eq!(writes_when_kept.get(), 0, "kept before any write");
        assert_eq!(kept.len(), 1, "one backup per run");
        let backup = &kept[0];
        assert_eq!(report.backup.as_ref(), Some(backup));
        assert_eq!(backup.target, device.address());
        assert_eq!(
            backup.load_states,
            vec![(MemoryLoadStateMachine::AddressTable, LoadState::Loaded)]
        );
        assert_eq!(
            backup.regions,
            vec![
                BackupRegion {
                    address: 0x4000,
                    octets: vec![0x07],
                },
                BackupRegion {
                    address: 0x4003,
                    octets: (100..127).collect(),
                },
            ]
        );
        // 27 octets at 12 per read: 12 + 12 + 3, never past the region.
        let reads: Vec<_> = memory_reads(&device)
            .into_iter()
            .filter(|(address, _)| (0x4000..0x4100).contains(address))
            .take(4)
            .collect();
        assert_eq!(
            reads,
            vec![(0x4000, 1), (0x4003, 12), (0x400F, 12), (0x401B, 3)]
        );
        let taken = seen
            .iter()
            .position(|p| {
                matches!(
                    p,
                    Progress::BackupTaken {
                        regions: 2,
                        octets: 28
                    }
                )
            })
            .expect("the backup is reported");
        let first_write = seen
            .iter()
            .position(|p| matches!(p, Progress::StepStarted { index: 2, .. }))
            .expect("the unload starts");
        assert!(taken < first_write, "{seen:?}");
    }

    #[tokio::test]
    async fn restoring_the_backup_brings_the_old_octets_back() {
        use knx_core::commissioning::device_backup::restore_plan;
        let device = old_device();
        let before: Vec<Option<u8>> = device.memory(0x4000, 30);
        let mut kept = None;
        run_memory_download_with_backup(
            &mut session(&device, WriteScope::Download),
            &plan(),
            |_| {},
            &mut |backup: &DeviceBackup| {
                kept = Some(backup.clone());
                Ok(())
            },
        )
        .await
        .expect("the download runs");
        assert_ne!(device.memory(0x4000, 30), before, "the download changed it");

        let restore = restore_plan(&plan(), device.address(), &kept.unwrap()).unwrap();
        run_memory_download(&mut session(&device, WriteScope::Download), &restore)
            .await
            .expect("the restore runs");
        assert_eq!(device.memory(0x4000, 30), before);
        assert_eq!(device.load_state(ObjectIndex::new(1)), LoadState::Loaded);
    }

    #[tokio::test]
    async fn a_backup_that_cannot_be_kept_stops_the_run_with_nothing_written() {
        let device = old_device();
        let mut session = session(&device, WriteScope::Download);
        let error = run_memory_download_with_backup(
            &mut session,
            &plan(),
            |_| {},
            &mut |_: &DeviceBackup| Err("disk full".into()),
        )
        .await
        .expect_err("refused");
        assert!(
            matches!(&error, MemoryDownloadError::BackupNotKept(why) if why == "disk full"),
            "{error}"
        );
        assert!(error.to_string().contains("nothing was written"), "{error}");
        assert!(memory_writes(&device).is_empty(), "no write at all");
        assert_eq!(device.load_state(ObjectIndex::new(1)), LoadState::Loaded);
        assert!(session.connection().is_none());
        assert_eq!(
            device
                .seen()
                .iter()
                .filter(|s| matches!(s, Seen::Disconnect))
                .count(),
            1,
            "existing backup refusal must not receive a second cleanup disconnect"
        );
    }

    #[tokio::test]
    async fn a_backup_that_cannot_be_read_stops_the_run_with_nothing_written() {
        let device = device(SimulatorConfig {
            protected_memory: Some((0x4003, 0x4004)),
            ..SimulatorConfig::default()
        });
        device.preset_load_state(ObjectIndex::new(1), LoadState::Loaded);
        let mut called = false;
        let error = run_memory_download_with_backup(
            &mut session(&device, WriteScope::Download),
            &plan(),
            |_| {},
            &mut |_: &DeviceBackup| {
                called = true;
                Ok(())
            },
        )
        .await
        .expect_err("refused");
        assert!(
            matches!(
                error,
                MemoryDownloadError::BackupRead {
                    index: 2,
                    error: SessionError::MemoryRefused { address: 0x4003 }
                }
            ),
            "{error}"
        );
        assert!(!called, "nothing to keep");
        assert!(memory_writes(&device).is_empty(), "no write at all");
    }

    #[tokio::test]
    async fn without_a_backup_the_run_reads_nothing_extra() {
        let plain = old_device();
        let report = run_memory_download(&mut session(&plain, WriteScope::Download), &plan())
            .await
            .expect("the download runs");
        assert!(report.backup.is_none());
        let backed_up = old_device();
        run_memory_download_with_backup(
            &mut session(&backed_up, WriteScope::Download),
            &plan(),
            |_| {},
            &mut |_: &DeviceBackup| Ok(()),
        )
        .await
        .expect("the download runs");
        // The backup adds one load-state read and one read per chunk of
        // each region: 1 + (1 + 3). The read-backs are the same in both.
        assert_eq!(
            memory_reads(&backed_up).len(),
            memory_reads(&plain).len() + 5
        );
    }

    // ------------------------------------------------ read without writing

    fn read_only(device: &SimulatedDevice) -> ManagementSession<'_, SimulatedDevice> {
        ManagementSession::read_only(device, device.address(), AuthorisationPlan::Skip, fast())
            .expect("a read-only session")
    }

    fn writes_of_any_kind(device: &SimulatedDevice) -> Vec<Seen> {
        device
            .seen()
            .into_iter()
            .filter(|seen| {
                matches!(
                    seen,
                    Seen::MemoryWrite { .. } | Seen::PropertyWrite { .. } | Seen::Restart { .. }
                )
            })
            .collect()
    }

    #[tokio::test]
    async fn reading_what_a_plan_overwrites_returns_the_backup_and_writes_nothing() {
        let device = old_device();
        let before = device.memory(0x4000, 30);
        let held = read_what_the_plan_overwrites(&mut read_only(&device), &plan())
            .await
            .expect("read");
        assert_eq!(held.target, device.address());
        assert_eq!(
            held.load_states,
            vec![(MemoryLoadStateMachine::AddressTable, LoadState::Loaded)]
        );
        assert_eq!(
            held.regions,
            vec![
                BackupRegion {
                    address: 0x4000,
                    octets: vec![0x07],
                },
                BackupRegion {
                    address: 0x4003,
                    octets: (100..127).collect(),
                },
            ]
        );
        assert_eq!(writes_of_any_kind(&device), vec![], "read only");
        assert_eq!(device.memory(0x4000, 30), before);
        assert_eq!(device.seen().last(), Some(&Seen::Disconnect));
    }

    #[tokio::test]
    async fn another_mask_or_manufacturer_is_refused_before_any_memory_read() {
        let other_mask = SimulatedDevice::with_config(SimulatorConfig {
            mask_version: 0x0705,
            ..SimulatorConfig::default()
        });
        other_mask.preset_property(0, PID_MANUFACTURER_ID, &MANUFACTURER.to_be_bytes());
        let error = read_what_the_plan_overwrites(&mut read_only(&other_mask), &plan())
            .await
            .expect_err("refused");
        assert!(
            matches!(error, ReadBackError::MaskMismatch { .. }),
            "{error}"
        );
        assert!(memory_reads(&other_mask).is_empty());
        assert_eq!(other_mask.seen().last(), Some(&Seen::Disconnect));

        let other_maker = device(SimulatorConfig::default());
        other_maker.preset_property(0, PID_MANUFACTURER_ID, &0x0002u16.to_be_bytes());
        let error = read_what_the_plan_overwrites(&mut read_only(&other_maker), &plan())
            .await
            .expect_err("refused");
        assert!(
            matches!(
                error,
                ReadBackError::ManufacturerMismatch {
                    expected: MANUFACTURER,
                    found: 0x0002
                }
            ),
            "{error}"
        );
        assert!(memory_reads(&other_maker).is_empty());
    }

    #[tokio::test]
    async fn a_refused_read_is_an_error_not_a_shorter_backup() {
        let device = device(SimulatorConfig {
            protected_memory: Some((0x4003, 0x4004)),
            ..SimulatorConfig::default()
        });
        device.preset_load_state(ObjectIndex::new(1), LoadState::Loaded);
        let error = read_what_the_plan_overwrites(&mut read_only(&device), &plan())
            .await
            .expect_err("refused");
        assert!(
            matches!(
                error,
                ReadBackError::Session(SessionError::MemoryRefused { address: 0x4003 })
            ),
            "{error}"
        );
        assert!(error.to_string().contains("nothing was written"), "{error}");
        assert_eq!(writes_of_any_kind(&device), vec![]);
    }

    // ------------------------------------------------ compare with a plan

    fn contactable(device: &SimulatedDevice) -> ContactableAddress {
        ContactableAddress::new(device.address()).expect("not excluded")
    }

    #[tokio::test]
    async fn comparing_names_every_differing_run_and_writes_nothing() {
        let device = old_device();
        let before = device.memory(0x4000, 30);
        let compared = compare_with_plan(&device, contactable(&device), fast(), &plan())
            .await
            .expect("compared");
        assert!(!compared.is_same());
        assert_eq!(compared.held.target, device.address());
        assert_eq!(
            compared.changes,
            download_changes(&plan(), device.address(), &compared.held).unwrap()
        );
        assert_eq!(
            compared.differing_octets(),
            compared
                .changes
                .iter()
                .map(|c| c.planned.len())
                .sum::<usize>()
        );
        assert!(compared.differing_octets() > 0);
        assert_eq!(writes_of_any_kind(&device), vec![], "read only");
        assert_eq!(device.memory(0x4000, 30), before);
        assert_eq!(device.seen().last(), Some(&Seen::Disconnect));
    }

    #[tokio::test]
    async fn a_device_that_holds_the_plan_compares_the_same() {
        let device = old_device();
        run_memory_download(&mut session(&device, WriteScope::Download), &plan())
            .await
            .expect("the download runs");
        let writes = writes_of_any_kind(&device).len();
        let compared = compare_with_plan(&device, contactable(&device), fast(), &plan())
            .await
            .expect("compared");
        assert!(compared.is_same(), "{:?}", compared.changes);
        assert_eq!(compared.differing_octets(), 0);
        assert_eq!(
            writes_of_any_kind(&device).len(),
            writes,
            "the compare wrote nothing"
        );
    }

    #[tokio::test]
    async fn a_compare_that_cannot_read_says_why_and_that_nothing_was_written() {
        let other_mask = SimulatedDevice::with_config(SimulatorConfig {
            mask_version: 0x0705,
            ..SimulatorConfig::default()
        });
        other_mask.preset_property(0, PID_MANUFACTURER_ID, &MANUFACTURER.to_be_bytes());
        let error = compare_with_plan(&other_mask, contactable(&other_mask), fast(), &plan())
            .await
            .expect_err("refused");
        assert!(
            matches!(
                error,
                CompareError::Read(ReadBackError::MaskMismatch { .. })
            ),
            "{error}"
        );
        assert!(error.to_string().contains("nothing was written"), "{error}");
        assert!(memory_reads(&other_mask).is_empty());
    }
}
