//! The catalogue jobs owner: admission close and the drain lifecycle (P9).

use std::future::Future;
use std::sync::Mutex;
use std::time::Duration;

use tokio::task::JoinHandle;

use super::dto::{CatalogueDrain, DrainCleanup, DrainCommit, DrainEnd, SCHEMA_VERSION};
use super::strict::Incarnation;
use super::{AdmissionState, CatalogueJobManager};

/// The server-side catalogue jobs owner. The admission/replay manager lives
/// under one short standard lock; the worker/cleanup handles live under a
/// separate short standard lock. An async permit serializes join waiters;
/// neither standard lock is held across an await.
pub struct CatalogueUpdateJobs {
    manager: Mutex<CatalogueJobManager>,
    handles: Mutex<Handles>,
    /// Only one waiter polls the join handle; waiting for this permit counts
    /// against each drain's own deadline and second-signal escape.
    draining: tokio::sync::Mutex<()>,
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
            draining: tokio::sync::Mutex::new(()),
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

    /// Registers the worker at the once-only handoff seam. Refusal returns
    /// the exact submitted handle: the caller still owns and must join it.
    pub(crate) fn register_worker(&self, handle: JoinHandle<()>) -> Result<(), JoinHandle<()>> {
        let Ok(mut handles) = self.handles.lock() else {
            return Err(handle);
        };
        if handles.worker.is_some() {
            return Err(handle);
        }
        handles.worker = Some(handle);
        Ok(())
    }

    /// Joins the registered worker/cleanup handles with a bounded wait racing
    /// the limit and `second_signal`. A timeout or second signal never claims
    /// the worker stopped; the still-live handle is retained for a later drain.
    pub async fn drain(
        &self,
        limit: Duration,
        second_signal: impl Future<Output = ()>,
    ) -> CatalogueDrain {
        let joined = async {
            let _draining = self.draining.lock().await;
            std::future::poll_fn(|cx| {
                // Keep authoritative ownership in the owner even while the
                // caller is suspended. Only this synchronous poll holds the
                // std lock; dropping the drain merely releases its permit.
                // Poison is a sticky ownership-health latch, not permission
                // to trust an empty slot. Recover only to join known handles;
                // this primitive cannot reconcile possibly lost ownership.
                let (mut handles, uncertain) = match self.handles.lock() {
                    Ok(handles) => (handles, false),
                    Err(poisoned) => (poisoned.into_inner(), true),
                };
                let Some(worker) = handles.worker.as_mut() else {
                    return if uncertain {
                        std::task::Poll::Pending
                    } else {
                        std::task::Poll::Ready(DrainEnd::NothingRunning)
                    };
                };
                match std::pin::Pin::new(worker).poll(cx) {
                    std::task::Poll::Pending => std::task::Poll::Pending,
                    std::task::Poll::Ready(_) => {
                        // A join error is still a finished worker. Never poll
                        // this completed handle again or abort blocking work.
                        handles.worker = None;
                        if uncertain {
                            std::task::Poll::Pending
                        } else {
                            std::task::Poll::Ready(DrainEnd::Finished)
                        }
                    }
                }
            })
            .await
        };
        let end = tokio::select! {
            // Already joined/no-work takes precedence over simultaneous escapes.
            biased;
            end = joined => end,
            _ = tokio::time::sleep(limit) => DrainEnd::TimedOut,
            _ = second_signal => DrainEnd::SecondSignal,
        };

        CatalogueDrain {
            schema_version: SCHEMA_VERSION,
            end,
            work_outstanding: matches!(end, DrainEnd::TimedOut | DrainEnd::SecondSignal),
            cleanup: if self.handles.is_poisoned() {
                DrainCleanup::Quarantined
            } else {
                DrainCleanup::Complete
            },
            commit: if self.handles.is_poisoned() {
                DrainCommit::Unknown
            } else {
                DrainCommit::NotApplicable
            },
        }
    }
}

#[cfg(test)]
mod lifecycle_tests;

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

    // Poll the real drain to Pending without assuming handle transfer.
    async fn poll_pending(future: std::pin::Pin<&mut impl Future<Output = CatalogueDrain>>) {
        let mut future = future;
        std::future::poll_fn(|cx| {
            assert!(future.as_mut().poll(cx).is_pending());
            std::task::Poll::Ready(())
        })
        .await;
    }

    #[tokio::test(start_paused = true)]
    async fn review_overlapping_drain_must_not_report_no_work() {
        let jobs = owner();
        let release = Arc::new(Notify::new());
        let notify = Arc::clone(&release);
        jobs.register_worker(tokio::spawn(async move { notify.notified().await }))
            .unwrap();
        let mut first = Box::pin(jobs.drain(Duration::from_secs(60), std::future::pending()));
        poll_pending(first.as_mut()).await;

        let observed = jobs.drain(Duration::ZERO, std::future::pending()).await;
        release.notify_one();
        let finished = first.await;
        assert_eq!(finished.end, DrainEnd::Finished);
        assert!(
            observed.work_outstanding,
            "overlapping drain must not claim no work"
        );
        assert_eq!(observed.end, DrainEnd::TimedOut);
    }

    #[tokio::test(start_paused = true)]
    async fn review_dropped_drain_must_retain_the_running_worker() {
        let jobs = owner();
        let release = Arc::new(Notify::new());
        let notify = Arc::clone(&release);
        // Independent completion witness permits fixture cleanup on original RED.
        let (done, ended) = tokio::sync::oneshot::channel();
        jobs.register_worker(tokio::spawn(async move {
            notify.notified().await;
            let _ = done.send(());
        }))
        .unwrap();
        let mut first = Box::pin(jobs.drain(Duration::from_secs(60), std::future::pending()));
        poll_pending(first.as_mut()).await;
        drop(first);

        let observed = jobs.drain(Duration::ZERO, std::future::pending()).await;
        release.notify_one();
        ended.await.unwrap();
        let final_drain = jobs
            .drain(Duration::from_secs(1), std::future::pending())
            .await;
        assert!(
            observed.work_outstanding,
            "dropping a drain must not detach its unfinished worker"
        );
        assert_eq!(observed.end, DrainEnd::TimedOut);
        assert_eq!(final_drain.end, DrainEnd::Finished);
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
        jobs.register_worker(tokio::spawn(async {})).unwrap();
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
        }))
        .unwrap();

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
        }))
        .unwrap();

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
