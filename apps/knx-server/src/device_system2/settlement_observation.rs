//! Test-only counters observe the actual terminal worker's catalogue lease and proof duration.
use knx_core::commissioning::pending::ProgrammingParts;
use std::{
    sync::{mpsc, Arc, Mutex},
    time::{Duration, Instant},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Event {
    LeaseAttempt,
    LeaseAcquired,
    Settled,
    CommandProjectBlocked,
}
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(super) struct Counts {
    pub callbacks: usize,
    pub candidate: Option<usize>,
    pub compared_candidate: Option<usize>,
    pub command_blocks: usize,
    pub attempts: usize,
    pub leases: usize,
    pub proofs: usize,
    pub wait: Duration,
    pub proof: Duration,
    pub parts: Option<ProgrammingParts>,
}
#[derive(Default)]
struct State {
    counts: Counts,
    attempt: Option<Instant>,
    acquired: Option<Instant>,
    command: Option<(String, u64, u64, knx_core::Command)>,
    worker: Option<tokio::task::AbortHandle>,
}
// Per service, installed only by the owning mounted test. No global or thread-local state,
// database handles, production switches, waits, callbacks supplied by tests or extra timeouts.
pub(super) struct Observation {
    state: Mutex<State>,
    events: mpsc::Sender<Event>,
}
impl Observation {
    pub fn new() -> (Arc<Self>, mpsc::Receiver<Event>) {
        let (events, receiver) = mpsc::channel();
        (
            Arc::new(Self {
                state: Mutex::new(State::default()),
                events,
            }),
            receiver,
        )
    }
    pub fn callback(&self, project: &knx_core::Project) {
        let mut state = self.state.lock().unwrap();
        state.counts.callbacks += 1;
        state.counts.candidate = Some(std::ptr::from_ref(project) as usize);
    }
    pub fn command_blocked(
        &self,
        binding: &crate::domain::SnapshotBinding,
        command: &knx_core::Command,
    ) {
        let mut state = self.state.lock().unwrap();
        state.counts.command_blocks += 1;
        state.command = Some((
            binding.server_incarnation.clone(),
            binding.project_incarnation,
            binding.snapshot_revision,
            command.clone(),
        ));
        let _ = self.events.send(Event::CommandProjectBlocked);
    }
    pub fn blocked_command(&self) -> Option<(String, u64, u64, knx_core::Command)> {
        self.state.lock().unwrap().command.clone()
    }
    pub fn worker_started(&self, worker: tokio::task::AbortHandle) {
        self.state.lock().unwrap().worker = Some(worker);
    }
    pub fn worker_finished(&self) -> Option<bool> {
        self.state
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .worker
            .as_ref()
            .map(tokio::task::AbortHandle::is_finished)
    }
    pub fn attempt(&self) {
        let mut state = self.state.lock().unwrap();
        state.counts.attempts += 1;
        state.attempt = Some(Instant::now());
        // An unbounded channel send never waits for the receiver while project/stack are held.
        let _ = self.events.send(Event::LeaseAttempt);
    }
    pub fn acquired(&self, project: &knx_core::Project) {
        let mut state = self.state.lock().unwrap();
        let now = Instant::now();
        state.counts.leases += 1;
        state.counts.compared_candidate = Some(std::ptr::from_ref(project) as usize);
        let wait = now.duration_since(state.attempt.take().unwrap());
        state.counts.wait += wait;
        state.acquired = Some(now);
        let _ = self.events.send(Event::LeaseAcquired);
    }
    pub fn proof(&self, parts: Option<ProgrammingParts>) {
        let mut state = self.state.lock().unwrap();
        state.counts.proofs += 1;
        let elapsed = state.acquired.take().unwrap().elapsed();
        state.counts.proof += elapsed;
        state.counts.parts = parts;
    }
    pub fn settled(&self) {
        let _ = self.events.send(Event::Settled);
    }
    pub fn counts(&self) -> Counts {
        self.state.lock().unwrap().counts
    }
}
