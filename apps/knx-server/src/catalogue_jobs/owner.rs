//! The catalogue jobs owner: admission close and the drain lifecycle (P9).

use std::future::Future;
use std::sync::Mutex;
use std::time::Duration;

use tokio::task::JoinHandle;

use super::dto::{CatalogueDrain, DrainCleanup, DrainCommit, DrainEnd, SCHEMA_VERSION};
use super::strict::Incarnation;
use super::{AdmissionState, CatalogueJobManager};

/// The server-side catalogue jobs owner. The admission/replay manager lives
/// under one short lock; the worker/cleanup handles live under a separate
/// short lock, so no lock is held across an await.
pub struct CatalogueUpdateJobs {
    manager: Mutex<CatalogueJobManager>,
    handles: Mutex<Handles>,
}

#[derive(Default)]
struct Handles {
    /// The single tracked blocking worker/supervisor, once registered.
    worker: Option<JoinHandle<()>>,
}

impl CatalogueUpdateJobs {
    pub fn new(incarnation: Incarnation) -> Self {
        Self {
            manager: Mutex::new(CatalogueJobManager::new(incarnation)),
            handles: Mutex::new(Handles::default()),
        }
    }

    /// Closes fresh admission; idempotent, short lock only, no await.
    pub fn close_admission_and_request_stop(&self) {
        if let Ok(mut manager) = self.manager.lock() {
            manager.close_admission();
        }
    }

    /// The current admission state (for tests and the future routes).
    pub(crate) fn admission_state(&self) -> AdmissionState {
        self.manager
            .lock()
            .map(|m| m.admission().state)
            .unwrap_or(AdmissionState::Stopping)
    }

    /// Registers the worker handle; the worker increment calls this once.
    pub(crate) fn register_worker(&self, handle: JoinHandle<()>) {
        if let Ok(mut handles) = self.handles.lock() {
            handles.worker = Some(handle);
        }
    }

    /// Joins the registered worker/cleanup handles with a bounded wait racing
    /// the limit and `second_signal`. A timeout or second signal never claims
    /// the worker stopped; the still-live handle is retained for a later drain.
    pub async fn drain(
        &self,
        limit: Duration,
        second_signal: impl Future<Output = ()>,
    ) -> CatalogueDrain {
        let worker = self.handles.lock().ok().and_then(|mut h| h.worker.take());
        let Some(mut worker) = worker else {
            return nothing_running();
        };

        let end = tokio::select! {
            result = &mut worker => {
                let _ = result; // a join error is still a finished worker
                DrainEnd::Finished
            }
            _ = tokio::time::sleep(limit) => DrainEnd::TimedOut,
            _ = second_signal => DrainEnd::SecondSignal,
        };

        if matches!(end, DrainEnd::TimedOut | DrainEnd::SecondSignal) {
            // The worker is still live: return it so a later drain can await it.
            if let Ok(mut handles) = self.handles.lock() {
                handles.worker = Some(worker);
            }
        }

        CatalogueDrain {
            schema_version: SCHEMA_VERSION,
            end,
            work_outstanding: matches!(end, DrainEnd::TimedOut | DrainEnd::SecondSignal),
            cleanup: DrainCleanup::Complete,
            commit: DrainCommit::NotApplicable,
        }
    }
}

fn nothing_running() -> CatalogueDrain {
    CatalogueDrain {
        schema_version: SCHEMA_VERSION,
        end: DrainEnd::NothingRunning,
        work_outstanding: false,
        cleanup: DrainCleanup::Complete,
        commit: DrainCommit::NotApplicable,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use tokio::sync::Notify;

    fn owner() -> Arc<CatalogueUpdateJobs> {
        Arc::new(CatalogueUpdateJobs::new(Incarnation::from_bytes(
            [0x01; 16],
        )))
    }

    #[tokio::test]
    async fn close_is_idempotent_and_closes_admission() {
        let jobs = owner();
        assert_eq!(jobs.admission_state(), AdmissionState::Open);
        jobs.close_admission_and_request_stop();
        jobs.close_admission_and_request_stop();
        assert_eq!(jobs.admission_state(), AdmissionState::Stopping);
    }

    #[tokio::test]
    async fn drain_without_work_reports_nothing_running() {
        let jobs = owner();
        let drain = jobs
            .drain(Duration::from_secs(1), std::future::pending())
            .await;
        assert_eq!(drain.end, DrainEnd::NothingRunning);
        assert!(!drain.work_outstanding);
    }

    #[tokio::test]
    async fn drain_finishes_when_the_worker_ends() {
        let jobs = owner();
        jobs.register_worker(tokio::spawn(async {}));
        let drain = jobs
            .drain(Duration::from_secs(30), std::future::pending())
            .await;
        assert_eq!(drain.end, DrainEnd::Finished);
        assert!(!drain.work_outstanding);
    }

    #[tokio::test(start_paused = true)]
    async fn drain_times_out_with_a_stuck_worker_and_retains_it() {
        let jobs = owner();
        let release = Arc::new(Notify::new());
        let release2 = Arc::clone(&release);
        jobs.register_worker(tokio::spawn(async move {
            release2.notified().await;
        }));

        let drain = jobs
            .drain(Duration::from_secs(5), std::future::pending())
            .await;
        assert_eq!(drain.end, DrainEnd::TimedOut);
        assert!(drain.work_outstanding);

        // The still-live worker is retained: a second drain also times out
        // rather than reporting nothingRunning.
        let again = jobs
            .drain(Duration::from_secs(1), std::future::pending())
            .await;
        assert_eq!(again.end, DrainEnd::TimedOut);

        release.notify_one();
    }

    #[tokio::test(start_paused = true)]
    async fn drain_second_signal_does_not_claim_the_worker_stopped() {
        let jobs = owner();
        let release = Arc::new(Notify::new());
        let release2 = Arc::clone(&release);
        jobs.register_worker(tokio::spawn(async move {
            release2.notified().await;
        }));

        let drain = jobs
            .drain(
                Duration::from_secs(60),
                tokio::time::sleep(Duration::from_secs(1)),
            )
            .await;
        assert_eq!(drain.end, DrainEnd::SecondSignal);
        assert!(drain.work_outstanding);
        release.notify_one();
    }
}
