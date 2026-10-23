//! Unwired catalogue admission kernel; no production construction or job execution.

use knx_productdb::operation::{
    CancelOutcome, CommitArbitration, CommitClaim, PublicationGate, PublicationState, Stop,
};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant};

pub(crate) trait Clock {
    fn sample(&self) -> Result<Duration, ()>;
}
pub(crate) struct MonotonicClock {
    pub(crate) origin: Instant,
}
impl Clock for MonotonicClock {
    fn sample(&self) -> Result<Duration, ()> {
        Instant::now().checked_duration_since(self.origin).ok_or(())
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Identity {
    pub(crate) incarnation: [u8; 16],
    pub(crate) serial: u64,
    pub(crate) job: [u8; 16],
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Phase {
    Receiving,
    Previewing,
    Ready,
    WaitingForWriter,
    Settling,
    Committing,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ReadyEvidence {
    pub(crate) consent: [u8; 16],
    pub(crate) artifact: [u8; 16],
    pub(crate) digest: [u8; 32],
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Ticket {
    pub(crate) identity: Identity,
    pub(crate) phase: Phase,
    pub(crate) generation: u64,
    pub(crate) kind: LeaseKind,
    pub(crate) deadline: Duration,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LeaseKind {
    Receiving,
    Working,
    Ready,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Lease {
    pub(crate) kind: LeaseKind,
    pub(crate) deadline: Duration,
    pub(crate) deadline_tick_ms: u64,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Consent {
    Unavailable,
    Available,
    Spent,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Command {
    pub(crate) command: [u8; 16],
    pub(crate) consent: [u8; 16],
    pub(crate) artifact: [u8; 16],
    pub(crate) digest: [u8; 32],
    pub(crate) confirmed: bool,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Cause {
    ReceivingTimeout,
    ReceivingIdleTimeout,
    UserCancel,
    ServerStop,
    ReadyExpired,
    WorkingTimeout,
    CounterExhausted,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Snapshot {
    pub(crate) identity: Identity,
    pub(crate) phase: Phase,
    pub(crate) revision: u64,
    pub(crate) generation: u64,
    pub(crate) lease: Lease,
    pub(crate) consent: Consent,
    pub(crate) accepted: Option<Command>,
    pub(crate) stop_cause: Option<Cause>,
    pub(crate) admission_closed: bool,
    pub(crate) gate: PublicationState,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Error {
    Unavailable,
    InvalidHandle,
    InvalidPhase,
    CommandConflict,
    ServerStopping,
    ConsentExpired,
    CounterExhausted,
    InvalidRequest,
    CatalogueUpdateStale,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Apply {
    AcceptedOnce(Snapshot),
    Replay(Snapshot),
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Expiry {
    Inert(Snapshot),
    Stop(CancelOutcome, Snapshot),
}
struct State {
    view: Snapshot,
    gate: Arc<PublicationGate>,
    last_sample: Duration,
    unavailable: bool,
    ready: Option<ReadyEvidence>,
}
// Immutable opaque evidence never enters the mutable mutex or a transition callback.
pub(crate) struct Control<B, C> {
    identity: Identity,
    binding: B,
    clock: C,
    state: Mutex<State>,
    // Same allocation as State::gate, never a second publication gate.
    gate: Arc<PublicationGate>,
}
impl<B, C: Clock> Control<B, C> {
    pub(crate) fn new_receiving(
        identity: Identity,
        binding: B,
        clock: C,
        deadline_tick_ms: u64,
    ) -> Result<Self, Error> {
        let control = Self::new(identity, binding, clock)?;
        {
            let mut s = control.lock()?;
            s.view.phase = Phase::Receiving;
            s.view.lease = Lease {
                kind: LeaseKind::Receiving,
                deadline: Duration::from_millis(deadline_tick_ms),
                deadline_tick_ms,
            };
        }
        Ok(control)
    }
    pub(crate) fn begin_preview(&self, deadline_tick_ms: u64) -> Result<Snapshot, Error> {
        let mut s = self.lock()?;
        let now = self.observe(&mut s)?;
        if s.view.phase != Phase::Receiving || s.view.admission_closed {
            return Err(Error::InvalidPhase);
        }
        if now >= s.view.lease.deadline {
            Self::stop(&mut s, Cause::ReceivingTimeout)?;
            return Err(Error::InvalidPhase);
        }
        let (revision, generation) = Self::ordinary_counters(&mut s)?;
        s.view.phase = Phase::Previewing;
        s.view.lease = Lease {
            kind: LeaseKind::Working,
            deadline: Duration::from_millis(deadline_tick_ms),
            deadline_tick_ms,
        };
        s.view.revision = revision;
        s.view.generation = generation;
        Ok(s.view)
    }
    pub(crate) fn new(identity: Identity, binding: B, clock: C) -> Result<Self, Error> {
        if identity.serial == 0 || identity.serial == u64::MAX {
            return Err(Error::InvalidHandle);
        }
        let now = clock.sample().map_err(|()| Error::Unavailable)?;
        let lease = Self::lease(now, LeaseKind::Working).ok_or(Error::Unavailable)?;
        let view = Snapshot {
            identity,
            phase: Phase::Previewing,
            revision: 1,
            generation: 1,
            lease,
            consent: Consent::Unavailable,
            accepted: None,
            stop_cause: None,
            admission_closed: false,
            gate: PublicationState::Open,
        };
        let gate = Arc::new(PublicationGate::new());
        Ok(Self {
            identity,
            binding,
            clock,
            state: Mutex::new(State {
                view,
                gate: Arc::clone(&gate),
                last_sample: now,
                unavailable: false,
                ready: None,
            }),
            gate,
        })
    }
    #[cfg(test)]
    fn new_test(identity: Identity, binding: B, clock: C) -> Result<Self, Error> {
        Self::new(identity, binding, clock)
    }
    pub(crate) fn binding(&self) -> &B {
        &self.binding
    }
    fn lock(&self) -> Result<MutexGuard<'_, State>, Error> {
        let state = self.state.lock().map_err(|_| Error::Unavailable)?;
        if state.unavailable {
            return Err(Error::Unavailable);
        }
        Ok(state)
    }
    // Deadline, stop and COMMIT arbitrate under this short mutex. No SQL.
    fn claim_live_commit(&self) -> Result<Option<CommitClaim<'_>>, Error> {
        let mut s = self.lock()?;
        if s.gate.state() != PublicationState::Open {
            return Ok(None);
        }
        if s.view.phase != Phase::WaitingForWriter || s.view.accepted.is_none() {
            return Err(Error::InvalidPhase);
        }
        let now = self.observe(&mut s)?;
        if now >= s.view.lease.deadline {
            Self::stop(&mut s, Cause::WorkingTimeout)?;
            return Ok(None);
        }
        let (revision, generation) = Self::ordinary_counters(&mut s)?;
        let Some(claim) = self.gate.claim() else {
            return Ok(None);
        };
        s.view.phase = Phase::Committing;
        s.view.gate = s.gate.state();
        s.view.revision = revision;
        s.view.generation = generation;
        Ok(Some(claim))
    }
    #[cfg(test)]
    fn claim_test(&self) -> Result<bool, Error> {
        self.claim_live_commit().map(|claim| claim.is_some())
    }
    pub(crate) fn request_stop(
        &self,
        identity: Identity,
        cause: Cause,
    ) -> Result<CancelOutcome, Error> {
        let mut s = self.lock()?;
        if identity != self.identity {
            return Err(Error::InvalidHandle);
        }
        Self::stop(&mut s, cause)
    }
    pub(crate) fn checkpoint(&self) -> Result<Result<(), Stop>, Error> {
        Ok(self.lock()?.gate.checkpoint())
    }
    fn lease(now: Duration, kind: LeaseKind) -> Option<Lease> {
        let seconds = match kind {
            LeaseKind::Receiving => 3600,
            LeaseKind::Ready => 900,
            LeaseKind::Working => 7200,
        };
        let deadline = now.checked_add(Duration::from_secs(seconds))?;
        Some(Lease {
            kind,
            deadline,
            deadline_tick_ms: u64::try_from(deadline.as_millis()).ok()?,
        })
    }
    fn replacement_lease(s: &mut State, now: Duration, kind: LeaseKind) -> Result<Lease, Error> {
        if let Some(lease) = Self::lease(now, kind) {
            return Ok(lease);
        }
        Self::stop(s, Cause::CounterExhausted)?;
        Err(Error::CounterExhausted)
    }
    fn observe(&self, s: &mut State) -> Result<Duration, Error> {
        if let Ok(now) = self.clock.sample() {
            if now >= s.last_sample && u64::try_from(now.as_millis()).is_ok() {
                s.last_sample = now;
                return Ok(now);
            }
        }
        Self::stop(s, Cause::CounterExhausted)?;
        Err(Error::CounterExhausted)
    }
    fn ordinary_counters(s: &mut State) -> Result<(u64, u64), Error> {
        if let Some((revision, generation)) = s
            .view
            .revision
            .checked_add(1)
            .zip(s.view.generation.checked_add(1))
        {
            if revision < u64::MAX - 64 && generation < u64::MAX - 64 {
                return Ok((revision, generation));
            }
        }
        Self::stop(s, Cause::CounterExhausted)?;
        Err(Error::CounterExhausted)
    }
    fn stop(s: &mut State, cause: Cause) -> Result<CancelOutcome, Error> {
        if s.view.admission_closed {
            return Ok(if s.gate.state() == PublicationState::CommitClaimed {
                CancelOutcome::TooLate
            } else {
                CancelOutcome::AlreadyAccepted
            });
        }
        let Some((revision, generation)) = s
            .view
            .revision
            .checked_add(1)
            .zip(s.view.generation.checked_add(1))
        else {
            s.unavailable = true;
            return Err(Error::Unavailable);
        };
        let result = s.gate.try_cancel();
        s.view.admission_closed = true;
        s.view.revision = revision;
        s.view.generation = generation;
        if result != CancelOutcome::TooLate {
            s.view.phase = Phase::Settling;
            s.view.consent = Consent::Spent;
            s.view.stop_cause = Some(cause);
        }
        s.view.gate = s.gate.state();
        Ok(result)
    }
    pub(crate) fn expire(&self, ticket: Ticket) -> Result<Expiry, Error> {
        let mut s = self.lock()?;
        if ticket != Self::state_ticket(&s) || s.view.admission_closed {
            return Ok(Expiry::Inert(s.view));
        }
        let now = self.observe(&mut s)?;
        if now < s.view.lease.deadline {
            return Ok(Expiry::Inert(s.view));
        }
        let cause = match s.view.lease.kind {
            LeaseKind::Receiving => Cause::ReceivingTimeout,
            LeaseKind::Ready => Cause::ReadyExpired,
            LeaseKind::Working => Cause::WorkingTimeout,
        };
        let result = Self::stop(&mut s, cause)?;
        Ok(Expiry::Stop(result, s.view))
    }
    pub(crate) fn apply(&self, identity: Identity, command: Command) -> Result<Apply, Error> {
        let mut s = self.lock()?;
        if identity != self.identity {
            return Err(Error::InvalidHandle);
        }
        if let Some(accepted) = s.view.accepted {
            return if accepted == command {
                Ok(Apply::Replay(s.view))
            } else {
                Err(Error::CommandConflict)
            };
        }
        if s.view.admission_closed {
            return Err(match s.view.stop_cause {
                Some(Cause::ServerStop) => Error::ServerStopping,
                Some(Cause::ReadyExpired) => Error::ConsentExpired,
                Some(Cause::CounterExhausted) => Error::CounterExhausted,
                _ => Error::InvalidPhase,
            });
        }
        if s.view.phase != Phase::Ready
            || s.view.consent != Consent::Available
            || s.view.lease.kind != LeaseKind::Ready
        {
            return Err(Error::InvalidPhase);
        }
        let now = self.observe(&mut s)?;
        if now >= s.view.lease.deadline {
            Self::stop(&mut s, Cause::ReadyExpired)?;
            return Err(Error::ConsentExpired);
        }
        let lease = Self::replacement_lease(&mut s, now, LeaseKind::Working)?;
        let (revision, generation) = Self::ordinary_counters(&mut s)?;
        if !command.confirmed {
            return Err(Error::InvalidRequest);
        }
        let ready = s.ready.ok_or(Error::InvalidPhase)?;
        if command.consent != ready.consent
            || command.artifact != ready.artifact
            || command.digest != ready.digest
        {
            return Err(Error::CatalogueUpdateStale);
        }
        s.view.lease = lease;
        s.view.phase = Phase::WaitingForWriter;
        s.view.revision = revision;
        s.view.generation = generation;
        s.view.consent = Consent::Spent;
        s.view.accepted = Some(command);
        Ok(Apply::AcceptedOnce(s.view))
    }
    pub(crate) fn snapshot(&self) -> Result<Snapshot, Error> {
        Ok(self.lock()?.view)
    }
    pub(crate) fn ticket(&self) -> Result<Ticket, Error> {
        let s = self.lock()?;
        Ok(Self::state_ticket(&s))
    }
    fn state_ticket(s: &State) -> Ticket {
        let v = s.view;
        Ticket {
            identity: v.identity,
            phase: v.phase,
            generation: v.generation,
            kind: v.lease.kind,
            deadline: v.lease.deadline,
        }
    }
    pub(crate) fn enter_ready(
        &self,
        expected: Ticket,
        evidence: ReadyEvidence,
    ) -> Result<Snapshot, Error> {
        let mut s = self.lock()?;
        if s.view.phase != Phase::Previewing
            || expected != Self::state_ticket(&s)
            || s.view.admission_closed
            || s.gate.state() != PublicationState::Open
        {
            return Err(Error::InvalidPhase);
        }
        let now = self.observe(&mut s)?;
        if now >= s.view.lease.deadline {
            Self::stop(&mut s, Cause::WorkingTimeout)?;
            return Err(Error::InvalidPhase);
        }
        let lease = Self::replacement_lease(&mut s, now, LeaseKind::Ready)?;
        let (revision, generation) = Self::ordinary_counters(&mut s)?;
        s.view.lease = lease;
        s.view.phase = Phase::Ready;
        s.view.revision = revision;
        s.view.generation = generation;
        s.view.consent = Consent::Available;
        s.ready = Some(evidence);
        Ok(s.view)
    }
}

impl<B, C: Clock> CommitArbitration for Control<B, C> {
    fn gate(&self) -> &PublicationGate {
        &self.gate
    }
    fn claim_commit(&self) -> Option<CommitClaim<'_>> {
        self.claim_live_commit().ok().flatten()
    }
}

#[path = "catalogue_jobs/controlled_apply.rs"]
pub(crate) mod controlled_apply;

#[cfg(test)]
#[path = "catalogue_job_control/tests.rs"]
mod tests;
