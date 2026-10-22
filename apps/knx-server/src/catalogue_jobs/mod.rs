//! Unwired catalogue job admission/replay manager; no production route, spool or worker execution.

mod strict;

pub use strict::{Capability, Incarnation, JobId, U64};

mod dto;

use std::collections::VecDeque;

/// The broad admission state, mirroring the wire `admission` enum exactly.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdmissionState {
    Open,
    Busy,
    Stopping,
    Exhausted,
    ArtifactStoreUnavailable,
    CatalogueUnavailable,
}

/// The numeric profile limits admitted for a job selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Profile {
    Standard,
    Large,
}

/// An immutable admitted job handle: incarnation, serial and job id.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Handle {
    pub incarnation: Incarnation,
    pub serial: U64,
    pub job: JobId,
}

/// The exact create body; the selection capability travels in the request header.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateBody {
    pub incarnation: Incarnation,
    pub serial: U64,
    pub source_name: String,
    pub declared_bytes: U64,
    pub profile: Profile,
    pub allow_large_package: bool,
}

/// The read-only admission projection; allocates nothing and reserves no serial.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Admission {
    pub incarnation: Incarnation,
    pub next_serial: U64,
    pub state: AdmissionState,
}

/// The outcome of a create request; each variant maps to a distinct HTTP result.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CreateOutcome {
    /// A fresh job id was reserved (HTTP 201).
    Fresh(JobId),
    /// A retained exact replay returned the same job id (HTTP 200).
    Replay(JobId),
    /// Conflicting metadata for a retained handle (HTTP 409).
    CommandConflict,
    /// Unknown, capability-mismatch, consumed-forgotten or tombstoned (HTTP 410).
    OutcomeUnavailable,
    /// The offered serial is above the next one (HTTP 409).
    StaleOffer,
    /// The body names a previous incarnation (HTTP 410).
    IncarnationChanged,
    /// Admission refused without consuming the serial (HTTP 409).
    JobBusy,
    /// The serial high-water mark is exhausted (HTTP 409).
    Exhausted,
    /// Admission is closed by a stop (HTTP 409).
    Stopping,
}

/// The outcome of a dispose (DELETE) request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DisposeOutcome {
    /// Disposed: an exact tombstone was recorded (HTTP 200).
    Disposed,
    /// The job is still active or its cleanup is unfinished (HTTP 409).
    InvalidPhase,
    /// The artifact store is quarantined; no deletion amnesty (HTTP 503).
    ArtifactStoreUnavailable,
    /// Unknown or forgotten handle (HTTP 410).
    OutcomeUnavailable,
}

/// Bounded terminal-ring capacity; one active slot plus no queue (P4.2).
const MAX_TERMINALS: usize = 8;
/// Bounded tombstone-ring capacity (P4.2).
const MAX_TOMBSTONES: usize = 8;

/// Retained identity and metadata shared by active, terminal and tombstone records.
#[derive(Debug, Clone, PartialEq, Eq)]
struct RecordMeta {
    handle: Handle,
    capability: Capability,
    source_name: String,
    declared_bytes: U64,
    profile: Profile,
    allow_large_package: bool,
}

struct ActiveJob {
    meta: RecordMeta,
}

struct TerminalRecord {
    meta: RecordMeta,
    /// False while owned reclamation still needs filesystem I/O; eviction then
    /// refuses admission (`JobBusy`) instead of dropping the record.
    cleanup_complete: bool,
}

struct Tombstone {
    meta: RecordMeta,
    /// True when created by an explicit dispose; false when created by count
    /// eviction (an evicted outcome never pretends DELETE succeeded).
    disposed: bool,
}

struct State {
    incarnation: Incarnation,
    next_serial: U64,
    admission: AdmissionState,
    active: Option<ActiveJob>,
    terminals: VecDeque<TerminalRecord>,
    tombstones: VecDeque<Tombstone>,
}

/// The admission/replay manager behind the future job protocol.
///
/// All mutating methods take `&mut self` and touch only owned in-memory state:
/// no SQL, filesystem, await or parser work runs here, so the sole
/// writer -> short-control nesting rule is preserved by construction. The
/// server wraps this in its short ownership lock when routes are wired.
pub struct CatalogueJobManager {
    state: State,
}

impl CatalogueJobManager {
    pub fn new(incarnation: Incarnation) -> Self {
        Self {
            state: State {
                incarnation,
                next_serial: U64::ONE,
                admission: AdmissionState::Open,
                active: None,
                terminals: VecDeque::new(),
                tombstones: VecDeque::new(),
            },
        }
    }

    /// Read-only admission projection; never allocates or reserves a serial.
    pub fn admission(&self) -> Admission {
        Admission {
            incarnation: self.state.incarnation,
            next_serial: self.state.next_serial,
            state: self.state.admission,
        }
    }

    /// Admits or replays a create request, classifying in contract order.
    pub fn create(&mut self, capability: &Capability, body: CreateBody) -> CreateOutcome {
        if body.incarnation != self.state.incarnation {
            return CreateOutcome::IncarnationChanged;
        }

        if let Some(active) = &self.state.active {
            if active.meta.handle.serial == body.serial {
                return Self::replay_or_conflict(capability, &body, &active.meta);
            }
        }
        if let Some(found) = self
            .state
            .terminals
            .iter()
            .find(|t| t.meta.handle.serial == body.serial)
        {
            return Self::replay_or_conflict(capability, &body, &found.meta);
        }
        if self
            .state
            .tombstones
            .iter()
            .any(|t| t.meta.handle.serial == body.serial)
        {
            // A tombstoned serial is consumed-but-forgotten; no metadata oracle.
            return CreateOutcome::OutcomeUnavailable;
        }

        let next = self.state.next_serial;
        if body.serial < next {
            return CreateOutcome::OutcomeUnavailable;
        }
        if body.serial > next {
            return CreateOutcome::StaleOffer;
        }

        match self.state.admission {
            AdmissionState::Open => {}
            AdmissionState::Stopping => return CreateOutcome::Stopping,
            AdmissionState::Exhausted => return CreateOutcome::Exhausted,
            AdmissionState::Busy
            | AdmissionState::ArtifactStoreUnavailable
            | AdmissionState::CatalogueUnavailable => return CreateOutcome::JobBusy,
        }

        if self.state.active.is_some() {
            return CreateOutcome::JobBusy;
        }

        if self.state.terminals.len() >= MAX_TERMINALS {
            let incomplete = self
                .state
                .terminals
                .front()
                .is_some_and(|oldest| !oldest.cleanup_complete);
            if incomplete {
                return CreateOutcome::JobBusy;
            }
            let evicted = self
                .state
                .terminals
                .pop_front()
                .expect("terminal ring nonempty above");
            self.push_tombstone(evicted.meta, false);
        }

        if self.state.next_serial == U64::MAX {
            self.state.admission = AdmissionState::Exhausted;
            return CreateOutcome::Exhausted;
        }
        let serial = self.state.next_serial;
        self.state.next_serial = serial
            .checked_add(U64::ONE)
            .expect("next serial is below the exhausted sentinel");
        let job = JobId::mint();
        self.state.active = Some(ActiveJob {
            meta: RecordMeta {
                handle: Handle {
                    incarnation: self.state.incarnation,
                    serial,
                    job,
                },
                capability: capability.clone(),
                source_name: body.source_name,
                declared_bytes: body.declared_bytes,
                profile: body.profile,
                allow_large_package: body.allow_large_package,
            },
        });
        CreateOutcome::Fresh(job)
    }

    /// Disposes a terminal or tombstoned job; active jobs are never disposed.
    pub fn dispose(&mut self, handle: &Handle, capability: &Capability) -> DisposeOutcome {
        if self.state.admission == AdmissionState::ArtifactStoreUnavailable {
            return DisposeOutcome::ArtifactStoreUnavailable;
        }

        if let Some(active) = &self.state.active {
            if active.meta.handle == *handle {
                return if active.meta.capability.verify(capability) {
                    DisposeOutcome::InvalidPhase
                } else {
                    DisposeOutcome::OutcomeUnavailable
                };
            }
        }

        if let Some(pos) = self
            .state
            .terminals
            .iter()
            .position(|t| t.meta.handle == *handle)
        {
            {
                let found = &self.state.terminals[pos];
                if !found.meta.capability.verify(capability) {
                    return DisposeOutcome::OutcomeUnavailable;
                }
                if !found.cleanup_complete {
                    return DisposeOutcome::InvalidPhase;
                }
            }
            let removed = self.state.terminals.remove(pos).expect("position valid");
            self.push_tombstone(removed.meta, true);
            return DisposeOutcome::Disposed;
        }

        if let Some(found) = self
            .state
            .tombstones
            .iter()
            .find(|t| t.meta.handle == *handle)
        {
            if !found.meta.capability.verify(capability) {
                return DisposeOutcome::OutcomeUnavailable;
            }
            return if found.disposed {
                DisposeOutcome::Disposed
            } else {
                DisposeOutcome::OutcomeUnavailable
            };
        }

        DisposeOutcome::OutcomeUnavailable
    }

    /// Closes fresh admission; retained replay and dispose stay available.
    pub fn close_admission(&mut self) {
        self.state.admission = AdmissionState::Stopping;
    }

    /// Marks the serial high-water mark exhausted; never wraps.
    pub fn exhaust(&mut self) {
        self.state.admission = AdmissionState::Exhausted;
    }

    /// Marks the artifact store quarantined; disposes are refused, no amnesty.
    pub fn mark_artifact_store_unavailable(&mut self) {
        self.state.admission = AdmissionState::ArtifactStoreUnavailable;
    }

    fn push_tombstone(&mut self, meta: RecordMeta, disposed: bool) {
        if self.state.tombstones.len() >= MAX_TOMBSTONES {
            self.state.tombstones.pop_front();
        }
        self.state
            .tombstones
            .push_back(Tombstone { meta, disposed });
    }

    /// Classifies a retained create against exact capability then metadata.
    fn replay_or_conflict(
        capability: &Capability,
        body: &CreateBody,
        meta: &RecordMeta,
    ) -> CreateOutcome {
        if !meta.capability.verify(capability) {
            return CreateOutcome::OutcomeUnavailable;
        }
        let exact = meta.source_name == body.source_name
            && meta.declared_bytes == body.declared_bytes
            && meta.profile == body.profile
            && meta.allow_large_package == body.allow_large_package;
        if exact {
            CreateOutcome::Replay(meta.handle.job)
        } else {
            CreateOutcome::CommandConflict
        }
    }
}

#[cfg(test)]
impl CatalogueJobManager {
    /// Moves the active job to a terminal record with the chosen cleanup state.
    fn finish_active(&mut self, cleanup_complete: bool) -> Option<Handle> {
        let active = self.state.active.take()?;
        let handle = active.meta.handle;
        self.state.terminals.push_back(TerminalRecord {
            meta: active.meta,
            cleanup_complete,
        });
        Some(handle)
    }

    fn set_next_serial(&mut self, serial: U64) {
        self.state.next_serial = serial;
    }

    fn set_admission(&mut self, state: AdmissionState) {
        self.state.admission = state;
    }

    fn active_job(&self) -> Option<JobId> {
        self.state.active.as_ref().map(|a| a.meta.handle.job)
    }

    fn terminal_count(&self) -> usize {
        self.state.terminals.len()
    }

    fn tombstone_count(&self) -> usize {
        self.state.tombstones.len()
    }

    fn oldest_terminal_cleanup_complete(&self) -> Option<bool> {
        self.state.terminals.front().map(|t| t.cleanup_complete)
    }

    fn oldest_terminal_serial(&self) -> Option<U64> {
        self.state.terminals.front().map(|t| t.meta.handle.serial)
    }

    fn oldest_tombstone_disposed(&self) -> Option<bool> {
        self.state.tombstones.front().map(|t| t.disposed)
    }

    fn oldest_tombstone_serial(&self) -> Option<U64> {
        self.state.tombstones.front().map(|t| t.meta.handle.serial)
    }
}

#[cfg(test)]
mod tests;
