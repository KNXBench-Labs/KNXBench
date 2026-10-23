//! Admission and cooperative settlement signals for a caller-owned System-2 task.
use super::LocalOutcome;
use std::{
    fmt,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    time::Duration,
};
use tokio::sync::watch;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum System2Settlement {
    Idle,
    Running {
        operation: String,
    },
    /// Task returned and transport ownership settled; outcome is NOT a recovery guarantee.
    Settled {
        operation: String,
        outcome: LocalOutcome,
    },
    /// The task lease was dropped without explicit settlement. Never classify as finished.
    Uncertain {
        operation: String,
    },
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct System2ControlSnapshot {
    pub accepting: bool,
    pub cancellation_requested: bool,
    pub settlement: System2Settlement,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum System2DrainOutcome {
    Settled(System2ControlSnapshot),
    TimedOut(System2ControlSnapshot),
    Uncertain(System2ControlSnapshot),
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum System2ControlError {
    Closed,
    Busy,
    InvalidOperation,
    WrongOperation,
    InvalidSettlement,
}
impl fmt::Display for System2ControlError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "System-2 operation control: {self:?}")
    }
}
impl std::error::Error for System2ControlError {}
struct State {
    accepting: bool,
    settlement: System2Settlement,
    cancel: Option<Arc<AtomicBool>>,
}
impl State {
    fn snapshot(&self) -> System2ControlSnapshot {
        System2ControlSnapshot {
            accepting: self.accepting,
            cancellation_requested: self
                .cancel
                .as_ref()
                .is_some_and(|c| c.load(Ordering::SeqCst)),
            settlement: self.settlement.clone(),
        }
    }
}
struct Inner {
    state: Mutex<State>,
    updates: watch::Sender<System2ControlSnapshot>,
}
/// Shared admission/cancel/settlement coordinator. The adapter retains the REAL
/// JoinHandle, OperationStore and original bundle until task+disconnect settlement.
/// Drain timeout never aborts, detaches, drops or takes ownership of that task/backup.
#[derive(Clone)]
pub struct System2OperationControl {
    inner: Arc<Inner>,
}
/// Move into the actual owned task; settle only after its transport has disconnected.
/// Dropping this lease without settlement quarantines the control as Uncertain.
pub struct System2OperationLease {
    inner: Arc<Inner>,
    operation: String,
    cancel: Arc<AtomicBool>,
    settled: bool,
}
impl Default for System2OperationControl {
    fn default() -> Self {
        Self::new()
    }
}
impl System2OperationControl {
    pub fn new() -> Self {
        let state = State {
            accepting: true,
            settlement: System2Settlement::Idle,
            cancel: None,
        };
        let (updates, _) = watch::channel(state.snapshot());
        Self {
            inner: Arc::new(Inner {
                state: Mutex::new(state),
                updates,
            }),
        }
    }
    /// Admission and close use the SAME mutex: close cannot miss a newly admitted task.
    pub fn try_admit(&self, operation: &str) -> Result<System2OperationLease, System2ControlError> {
        if operation.is_empty()
            || operation.len() > 128
            || !operation
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"-_.".contains(&b))
        {
            return Err(System2ControlError::InvalidOperation);
        }
        let mut state = self.inner.state.lock().expect("System-2 control poisoned");
        if !state.accepting {
            return Err(System2ControlError::Closed);
        }
        match &state.settlement {
            System2Settlement::Running { .. } | System2Settlement::Uncertain { .. } => {
                return Err(System2ControlError::Busy)
            }
            System2Settlement::Settled {
                operation: previous,
                ..
            } if previous == operation => return Err(System2ControlError::InvalidOperation),
            _ => {}
        }
        let cancel = Arc::new(AtomicBool::new(false));
        state.cancel = Some(cancel.clone());
        state.settlement = System2Settlement::Running {
            operation: operation.into(),
        };
        self.inner.updates.send_replace(state.snapshot());
        Ok(System2OperationLease {
            inner: self.inner.clone(),
            operation: operation.into(),
            cancel,
            settled: false,
        })
    }
    pub fn snapshot(&self) -> System2ControlSnapshot {
        self.inner
            .state
            .lock()
            .expect("System-2 control poisoned")
            .snapshot()
    }
    pub fn subscribe(&self) -> watch::Receiver<System2ControlSnapshot> {
        self.inner.updates.subscribe()
    }
    /// Local idempotent close admission + cooperative cancellation. Never aborts a task.
    pub fn close_admission(&self) -> System2ControlSnapshot {
        let mut state = self.inner.state.lock().expect("System-2 control poisoned");
        state.accepting = false;
        if let Some(cancel) = &state.cancel {
            cancel.store(true, Ordering::SeqCst);
        }
        let snapshot = state.snapshot();
        self.inner.updates.send_replace(snapshot.clone());
        snapshot
    }
    pub fn request_cancel(
        &self,
        operation: &str,
    ) -> Result<System2ControlSnapshot, System2ControlError> {
        let state = self.inner.state.lock().expect("System-2 control poisoned");
        if !matches!(&state.settlement, System2Settlement::Running { operation: active } if active == operation)
        {
            return Err(System2ControlError::WrongOperation);
        }
        if let Some(cancel) = &state.cancel {
            cancel.store(true, Ordering::SeqCst);
        }
        let snapshot = state.snapshot();
        self.inner.updates.send_replace(snapshot.clone());
        Ok(snapshot)
    }
    /// Safe to join concurrently with other service drains. Timeout only drops a
    /// watch wait, never the caller-owned operation task or its immutable backup.
    pub async fn drain(&self, timeout: Duration) -> System2DrainOutcome {
        self.close_admission();
        let mut updates = self.subscribe();
        let waited = tokio::time::timeout(timeout, async {
            loop {
                let snapshot = updates.borrow_and_update().clone();
                match snapshot.settlement {
                    System2Settlement::Idle | System2Settlement::Settled { .. } => {
                        return System2DrainOutcome::Settled(snapshot)
                    }
                    System2Settlement::Uncertain { .. } => {
                        return System2DrainOutcome::Uncertain(snapshot)
                    }
                    System2Settlement::Running { .. } => {}
                }
                if updates.changed().await.is_err() {
                    return System2DrainOutcome::Uncertain(self.snapshot());
                }
            }
        })
        .await;
        waited.unwrap_or_else(|_| System2DrainOutcome::TimedOut(self.snapshot()))
    }
}
impl System2OperationLease {
    pub fn cancel_token(&self) -> Arc<AtomicBool> {
        self.cancel.clone()
    }
    /// Explicit task settlement, NOT rollback. Running/restoring are never terminal.
    /// Use the final durable store outcome; an abandoned/unknown task stays Uncertain.
    pub fn settle(
        mut self,
        outcome: LocalOutcome,
    ) -> Result<System2ControlSnapshot, System2ControlError> {
        if matches!(outcome, LocalOutcome::Running | LocalOutcome::Restoring) {
            return Err(System2ControlError::InvalidSettlement);
        }
        let mut state = self.inner.state.lock().expect("System-2 control poisoned");
        if !matches!(&state.settlement, System2Settlement::Running { operation } if operation == &self.operation)
        {
            return Err(System2ControlError::WrongOperation);
        }
        state.settlement = System2Settlement::Settled {
            operation: self.operation.clone(),
            outcome,
        };
        state.cancel = None;
        self.settled = true;
        let snapshot = state.snapshot();
        self.inner.updates.send_replace(snapshot.clone());
        Ok(snapshot)
    }
}
impl Drop for System2OperationLease {
    fn drop(&mut self) {
        if self.settled {
            return;
        }
        if let Ok(mut state) = self.inner.state.lock() {
            if matches!(&state.settlement, System2Settlement::Running { operation } if operation == &self.operation)
            {
                state.settlement = System2Settlement::Uncertain {
                    operation: self.operation.clone(),
                };
                state.accepting = false;
                self.inner.updates.send_replace(state.snapshot());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn close_admission_atomically_cancels_active_and_refuses_new_work() {
        let control = System2OperationControl::new();
        let active = control.try_admit("active").unwrap();
        let cancel = active.cancel_token();
        assert!(!cancel.load(Ordering::SeqCst));
        let snapshot = control.close_admission();
        assert!(
            !snapshot.accepting,
            "close must stop new System-2 admission"
        );
        assert!(
            cancel.load(Ordering::SeqCst),
            "close must request cooperative cancellation"
        );
        assert!(control.try_admit("next").is_err());
    }
    #[tokio::test]
    async fn drain_timeout_retains_active_and_abandonment_is_never_success() {
        let control = System2OperationControl::new();
        let active = control.try_admit("active").unwrap();
        let cancel = active.cancel_token();
        let (release, wait) = tokio::sync::oneshot::channel::<()>();
        let owned_task = tokio::spawn(async move {
            wait.await.unwrap();
            active.settle(LocalOutcome::Cancelled).unwrap();
        });
        assert!(matches!(
            control.drain(Duration::ZERO).await,
            System2DrainOutcome::TimedOut(_)
        ));
        assert!(matches!(
            control.snapshot().settlement,
            System2Settlement::Running { .. }
        ));
        assert!(cancel.load(Ordering::SeqCst));
        assert!(
            !owned_task.is_finished(),
            "timeout must retain the actual owned task"
        );
        release.send(()).unwrap();
        tokio::time::timeout(Duration::from_secs(1), owned_task)
            .await
            .unwrap()
            .unwrap();
        assert!(matches!(
            control.drain(Duration::ZERO).await,
            System2DrainOutcome::Settled(System2ControlSnapshot {
                settlement: System2Settlement::Settled {
                    outcome: LocalOutcome::Cancelled,
                    ..
                },
                ..
            })
        ));
        let abandoned = System2OperationControl::new();
        drop(abandoned.try_admit("abandoned").unwrap());
        assert!(matches!(
            abandoned.drain(Duration::ZERO).await,
            System2DrainOutcome::Uncertain(_)
        ));
    }
}
