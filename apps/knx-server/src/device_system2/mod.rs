//! Process-owned configured System-2 service; no request/component owns its task.
mod dto;
mod routes;
pub(crate) mod static_files;
mod storage;
#[cfg(test)]
mod test_observation;
#[cfg(test)]
mod tests;
mod worker;
pub(crate) use routes::routes;
pub(crate) use storage::ROOT_NAME;

use crate::{errors::ApiError, SharedState};
use dto::{refuse, Binding, Metadata, Status};
use knx_app::{
    property_validation::{
        LocalOutcome, OperationProgress, PointerObservationPhase, System2DrainOutcome,
        System2OperationControl, System2Settlement,
    },
    system2_validation_preparation::ProjectHardwareSource,
};
use knx_core::commissioning::load_state::LoadState;
use std::{
    path::Path,
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::{sync::watch, task::JoinHandle};

#[derive(Clone)]
struct Preparation {
    id: String,
    metadata: Metadata,
    source: ProjectHardwareSource,
}
struct Record {
    metadata: Metadata,
    progress: OperationProgress,
    updates: Option<watch::Receiver<OperationProgress>>,
    task: Option<JoinHandle<()>>,
    control_id: String,
    message: Option<&'static str>,
}
fn initial(outcome: LocalOutcome) -> OperationProgress {
    OperationProgress {
        outcome,
        completed_steps: 0,
        total_steps: None,
        backup_available: false,
        restore_available: false,
        mutation_possible: false,
        recovery_phrases: None,
        prestate_differs: false,
        pointer_observations: Vec::new(),
        program_version_observation: None,
    }
}
impl Record {
    fn view(&self) -> Status {
        let mut p = self
            .updates
            .as_ref()
            .map(|r| r.borrow().clone())
            .unwrap_or_else(|| self.progress.clone());
        let running = self.task.as_ref().is_some_and(|t| !t.is_finished());
        if self.task.as_ref().is_some_and(JoinHandle::is_finished)
            && matches!(p.outcome, LocalOutcome::Running | LocalOutcome::Restoring)
        {
            p.outcome = LocalOutcome::Interrupted;
        }
        // App may publish its durable result before IP teardown. Do not offer a
        // recovery or a new operation while that retained future still owns guards.
        if running || self.control_id.ends_with("-restore") {
            p.recovery_phrases = None;
        }
        if self.control_id.ends_with("-restore")
            && !matches!(
                p.outcome,
                LocalOutcome::Restoring | LocalOutcome::Restored | LocalOutcome::RestoreFailed
            )
        {
            // An accepted recovery that ended before App's RestoreStarted has
            // no restore outcome. Never project the OLD download's Finished as
            // a completed recovery; the durable command remains consumed.
            p.outcome = LocalOutcome::Interrupted;
        }
        let phrases = p.recovery_phrases;
        Status {
            metadata: self.metadata.clone(),
            state: if running
                && !matches!(p.outcome, LocalOutcome::Running | LocalOutcome::Restoring)
            {
                if self.control_id.ends_with("-restore") {
                    "restoring"
                } else {
                    "running"
                }
            } else {
                match p.outcome {
                    LocalOutcome::Running => "running",
                    LocalOutcome::Restoring => "restoring",
                    LocalOutcome::Finished => "finished",
                    LocalOutcome::Failed => "failed",
                    LocalOutcome::Cancelled => "cancelled",
                    LocalOutcome::Interrupted => "interrupted",
                    LocalOutcome::Restored => "restored",
                    LocalOutcome::RestoreFailed => "restoreFailed",
                }
            },
            completed_steps: p.completed_steps,
            total_steps: p.total_steps,
            backup_available: p.backup_available,
            recovery_phrase: phrases.as_ref().map(|p| p.recovery_phrase.clone()),
            download_phrase: phrases.as_ref().map(|p| p.download_phrase.clone()),
            release_phrase: phrases.map(|p| p.release_phrase),
            message: self.message,
            mutation_may_have_started: p.mutation_possible,
            prestate_differs: p.prestate_differs,
            pointer_observations: p
                .pointer_observations
                .iter()
                .map(|o| dto::PointerObservation {
                    phase: match o.phase {
                        PointerObservationPhase::Prestate => "prestate",
                        PointerObservationPhase::InProcedure => "inProcedure",
                        PointerObservationPhase::Final => "final",
                    },
                    object: o.record.object,
                    operation_index: o.operation_index,
                    expected: dto::hex(&o.record.expected),
                    observed: o
                        .record
                        .observed
                        .map_or_else(|| "refused".into(), |p| dto::hex(&p)),
                    load_state: LoadState::from_octet(o.record.state)
                        .map(|s| s.to_string())
                        .unwrap_or_else(|_| "Unknown".into()),
                })
                .collect(),
            program_version_observation: p.program_version_observation.as_ref().map(
                |(expected, observed)| dto::ProgramVersionObservation {
                    expected: dto::hex(expected),
                    observed: dto::hex(observed),
                },
            ),
        }
    }
}
struct Inner {
    preparation: Option<Preparation>,
    current: Option<Record>,
    selection_unavailable: bool,
}
/// The one actual task, local status and private durable evidence owner.
pub struct System2Service {
    #[cfg(test)]
    observation: Option<Arc<test_observation::Observation>>,
    pub control: System2OperationControl,
    command: tokio::sync::Mutex<()>,
    inner: Mutex<Inner>,
    storage: Mutex<Option<Arc<storage::Storage>>>,
    storage_unavailable: bool,
}
impl System2Service {
    /// Reattach the durable selected operation locally. Never opens a socket or
    /// resumes a download; an unfinished intact journal reopens as Interrupted.
    pub fn new(data_dir: &Path) -> Self {
        let opened = storage::Storage::open(data_dir, false);
        let storage_unavailable = opened.is_err();
        let storage = opened.ok().flatten().map(Arc::new);
        let mut inner = Inner {
            preparation: None,
            current: None,
            selection_unavailable: false,
        };
        if let Some(root) = &storage {
            let selected = root
                .selected()
                .and_then(|id| id.map(|id| load_record(root, &id)).transpose());
            match selected {
                Ok(record) => inner.current = record,
                Err(_) => inner.selection_unavailable = true,
            }
        }
        Self {
            #[cfg(test)]
            observation: None,
            control: System2OperationControl::new(),
            command: tokio::sync::Mutex::new(()),
            inner: Mutex::new(inner),
            storage: Mutex::new(storage),
            storage_unavailable,
        }
    }
    fn root(&self, state: &crate::AppState) -> Result<Arc<storage::Storage>, ApiError> {
        if self.storage_unavailable {
            return Err(refuse("system2EvidenceUnavailable"));
        }
        let mut root = self
            .storage
            .lock()
            .map_err(|_| refuse("system2Unavailable"))?;
        if root.is_none() {
            *root = storage::Storage::open(&state.data_dir, true)
                .map_err(|_| refuse("system2EvidenceUnavailable"))?
                .map(Arc::new);
        }
        root.clone()
            .ok_or_else(|| refuse("system2EvidenceUnavailable"))
    }
    fn idle(&self) -> Result<(), ApiError> {
        if !self.control.snapshot().accepting {
            return Err(refuse("system2AdmissionClosed"));
        }
        if matches!(
            self.control.snapshot().settlement,
            System2Settlement::Running { .. } | System2Settlement::Uncertain { .. }
        ) {
            return Err(refuse("system2Busy"));
        }
        let inner = self
            .inner
            .lock()
            .map_err(|_| refuse("system2Unavailable"))?;
        if inner
            .current
            .as_ref()
            .is_some_and(|r| r.task.as_ref().is_some_and(|t| !t.is_finished()))
        {
            return Err(refuse("system2Busy"));
        }
        Ok(())
    }
    fn status(&self) -> Result<Option<Status>, ApiError> {
        if self.storage_unavailable {
            return Err(refuse("system2EvidenceUnavailable"));
        }
        let inner = self
            .inner
            .lock()
            .map_err(|_| refuse("system2Unavailable"))?;
        if inner.selection_unavailable {
            return Err(refuse("system2SelectionRequired"));
        }
        Ok(inner.current.as_ref().map(Record::view))
    }
    fn publish_progress(&self, id: &str, receiver: watch::Receiver<OperationProgress>) {
        let mut inner = self.inner.lock().expect("System-2 status poisoned");
        if let Some(record) = inner
            .current
            .as_mut()
            .filter(|r| r.metadata.operation_id == id)
        {
            record.updates = Some(receiver);
        }
    }
    fn publish_end(&self, id: &str, fallback: LocalOutcome, message: Option<&'static str>) {
        let mut inner = self.inner.lock().expect("System-2 status poisoned");
        if let Some(record) = inner
            .current
            .as_mut()
            .filter(|r| r.metadata.operation_id == id)
        {
            if record.updates.is_none() {
                record.progress.outcome = fallback;
            }
            record.message = message;
        }
    }
    async fn wait_task(&self) {
        std::future::poll_fn(|cx| {
            let Ok(mut inner) = self.inner.lock() else {
                return std::task::Poll::Ready(());
            };
            let Some(record) = inner.current.as_mut() else {
                return std::task::Poll::Ready(());
            };
            let Some(task) = record.task.as_mut() else {
                return std::task::Poll::Ready(());
            };
            match std::future::Future::poll(std::pin::Pin::new(task), cx) {
                std::task::Poll::Pending => std::task::Poll::Pending,
                std::task::Poll::Ready(_) => {
                    record.task = None;
                    if let Some(updates) = record.updates.take() {
                        record.progress = updates.borrow().clone();
                    }
                    if matches!(
                        record.progress.outcome,
                        LocalOutcome::Running | LocalOutcome::Restoring
                    ) {
                        record.progress.outcome = LocalOutcome::Interrupted;
                        record.progress.recovery_phrases = None;
                    }
                    std::task::Poll::Ready(())
                }
            }
        })
        .await;
    }
    async fn join_finished(&self) {
        // Only take an already finished task. A timeout/waiter cancellation can
        // never detach a live future or lose its store/tunnel ownership.
        let task = {
            let mut inner = self.inner.lock().expect("System-2 status poisoned");
            inner.current.as_mut().and_then(|r| {
                if r.task.as_ref().is_some_and(JoinHandle::is_finished) {
                    if let Some(updates) = r.updates.take() {
                        r.progress = updates.borrow().clone();
                    }
                    if matches!(
                        r.progress.outcome,
                        LocalOutcome::Running | LocalOutcome::Restoring
                    ) {
                        r.progress.outcome = LocalOutcome::Interrupted;
                        r.progress.recovery_phrases = None;
                    }
                    r.task.take()
                } else {
                    None
                }
            })
        };
        if let Some(task) = task {
            let _ = task.await;
        }
    }
}
fn load_record(root: &storage::Storage, id: &str) -> std::io::Result<Record> {
    Ok(Record { metadata: root.metadata(id)?, progress: root.progress(id)?.unwrap_or_else(|| initial(LocalOutcome::Interrupted)), updates: None, task: None, control_id: id.into(), message: Some("Original retained operation; no automatic resume or retry. Recovery remains experimental best-effort.") })
}
fn binding(state: &crate::AppState, device: u32) -> Binding {
    use std::sync::atomic::Ordering;
    Binding {
        server_incarnation: state.server_incarnation.clone(),
        project_incarnation: state.project_incarnation.load(Ordering::SeqCst).to_string(),
        project_revision: state.project_revision.load(Ordering::SeqCst).to_string(),
        device_id: device,
    }
}
fn check_binding(state: &crate::AppState, expected: &Binding) -> Result<(), ApiError> {
    if &binding(state, expected.device_id) != expected {
        return Err(refuse("system2PreparationStale"));
    }
    Ok(())
}
async fn blocking<T: Send + 'static>(
    f: impl FnOnce() -> Result<T, ApiError> + Send + 'static,
) -> Result<T, ApiError> {
    tokio::task::spawn_blocking(f)
        .await
        .map_err(|_| refuse("system2Unavailable"))?
}

/// Independent lifecycle facts; no timeout result claims rollback or completion.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct System2Drain {
    pub outcome: System2DrainOutcome,
    pub work_outstanding: bool,
    pub second_signal: bool,
}
pub async fn drain_device_system2(
    state: &crate::AppState,
    limit: Duration,
    second_signal: impl std::future::Future<Output = ()>,
) -> System2Drain {
    state.device_system2.control.close_admission();
    let (outcome, second_signal) = tokio::select! {
        biased;
        outcome = tokio::time::timeout(limit, async {
            let outcome = state.device_system2.control.drain(limit).await;
            if matches!(&outcome, System2DrainOutcome::Settled(_)) { state.device_system2.wait_task().await; }
            outcome
        }) => (outcome.unwrap_or_else(|_| System2DrainOutcome::TimedOut(state.device_system2.control.snapshot())), false),
        () = second_signal => (System2DrainOutcome::TimedOut(state.device_system2.control.snapshot()), true),
    };
    let outstanding_task = state
        .device_system2
        .inner
        .lock()
        .map(|i| {
            i.current
                .as_ref()
                .is_some_and(|r| r.task.as_ref().is_some_and(|t| !t.is_finished()))
        })
        .unwrap_or(true);
    let work_outstanding = outstanding_task || !matches!(&outcome, System2DrainOutcome::Settled(_));
    if !work_outstanding {
        state.device_system2.join_finished().await;
    }
    System2Drain {
        outcome,
        work_outstanding,
        second_signal,
    }
}
