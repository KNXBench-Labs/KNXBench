//! Owned HTTP/server shutdown shared by desktop and API-only embedders.

use std::future::Future;
use std::io;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
#[cfg(test)]
use std::time::Duration;

use tokio::sync::watch;
use tokio::task::JoinHandle;

use crate::{AppState, BusRelease, CatalogueDrain, ServeEnd, WriteDrain};

/// Independent shutdown facts; a timeout is not worker completion or rollback.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmbeddedClose {
    pub http: Result<ServeEnd, String>,
    pub bus: BusRelease,
    pub writes: WriteDrain,
    pub catalogue: CatalogueDrain,
}

impl EmbeddedClose {
    /// Returning into runtime Drop is unsafe while owned blocking work remains.
    /// The process host, not this library, performs the explicit exit escape.
    pub fn requires_process_exit(&self) -> bool {
        self.http != Ok(ServeEnd::Drained)
            || matches!(
                self.writes.end,
                crate::WriteWaitEnd::TimedOut | crate::WriteWaitEnd::SecondSignal
            )
            || self.catalogue.work_outstanding
    }
}

/// Production HTTP lifetime. Retain this handle until shutdown has been drained;
/// dropping a waiter never drops the server or catalogue worker join handles.
#[derive(Clone)]
pub struct EmbeddedServerHandle {
    inner: Arc<Inner>,
}

struct Inner {
    state: Arc<AppState>,
    runtime: tokio::runtime::Handle,
    http_stop: watch::Sender<bool>,
    second_stop: watch::Sender<bool>,
    server: Mutex<Option<JoinHandle<io::Result<()>>>>,
    shutdown: Mutex<Option<JoinHandle<()>>>,
    shutdown_join: tokio::sync::Mutex<()>,
    result: watch::Sender<Option<Arc<EmbeddedClose>>>,
    #[cfg(test)]
    shutdown_runs: std::sync::atomic::AtomicUsize,
}

impl EmbeddedServerHandle {
    /// Start the actual router on an already-bound listener, inside a Tokio
    /// runtime. The embedder chooses its address/static assets; policies below
    /// are identical for desktop and API-only use, without environment reads.
    pub fn spawn(
        listener: tokio::net::TcpListener,
        state: Arc<AppState>,
        static_dir: Option<PathBuf>,
    ) -> Self {
        let (http_stop, mut stop_rx) = watch::channel(false);
        let (second_stop, _) = watch::channel(false);
        let (result, _) = watch::channel(None);
        let server_state = Arc::clone(&state);
        let inner = Arc::new(Inner {
            state,
            runtime: tokio::runtime::Handle::current(),
            http_stop,
            second_stop,
            server: Mutex::new(None),
            shutdown: Mutex::new(None),
            shutdown_join: tokio::sync::Mutex::new(()),
            result,
            #[cfg(test)]
            shutdown_runs: std::sync::atomic::AtomicUsize::new(0),
        });
        let weak = Arc::downgrade(&inner);
        // Register before a fast server termination can start the drain. No
        // await or worker work runs while this short ownership lock is held.
        let mut slot = inner.server.lock().unwrap_or_else(|p| p.into_inner());
        *slot = Some(tokio::spawn(async move {
            let served = axum::serve(listener, crate::app(Arc::clone(&server_state), static_dir))
                .with_graceful_shutdown(async move {
                    let _ = stop_rx.wait_for(|stop| *stop).await;
                })
                .await;
            crate::close_process_admission(&server_state);
            // Unexpected HTTP termination also starts the same retained drain;
            // the host still decides when/how to exit from the exact result.
            if let Some(inner) = weak.upgrade() {
                EmbeddedServerHandle { inner }.request_close();
            }
            served
        }));
        drop(slot);
        Self { inner }
    }

    /// Close both admission domains and start exactly one retained shutdown
    /// task. Safe from native callbacks outside the runtime's entered context.
    /// Repeated ordinary close requests do not mean force/second-stop.
    pub fn request_close(&self) {
        crate::close_process_admission(&self.inner.state);
        let mut shutdown = self
            .inner
            .shutdown
            .lock()
            .unwrap_or_else(|p| p.into_inner());
        if shutdown.is_some() || self.inner.result.borrow().is_some() {
            return;
        }
        self.inner.http_stop.send_replace(true);
        let inner = Arc::clone(&self.inner);
        #[cfg(test)]
        inner
            .shutdown_runs
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        *shutdown = Some(self.inner.runtime.spawn(async move {
            let result = run_close(&inner).await;
            inner.result.send_replace(Some(Arc::new(result)));
        }));
    }

    /// Await the one owned shutdown. Only the explicitly supplied second-stop
    /// future can cut waits short; cancellation of this waiter is not force.
    /// A timed-out result retains the unfinished handles for process exit or
    /// inspection; it never aborts blocking workers or invents terminal status.
    pub async fn drain_close(&self, second_signal: impl Future<Output = ()>) -> Arc<EmbeddedClose> {
        let mut result = self.inner.result.subscribe();
        self.request_close();
        let ready = async {
            result
                .wait_for(|result| result.is_some())
                .await
                .expect("shutdown result sender is retained")
                .as_ref()
                .unwrap()
                .clone()
        };
        tokio::pin!(ready);
        let outcome = tokio::select! {
            biased;
            result = &mut ready => result,
            () = second_signal => {
                self.inner.second_stop.send_replace(true);
                ready.await
            }
        };
        // Serialize actual JoinHandle polling while retaining it in the owner
        // across waiter cancellation. Do not replace it with a detached spawn.
        let _join = self.inner.shutdown_join.lock().await;
        std::future::poll_fn(|cx| {
            let mut task = self
                .inner
                .shutdown
                .lock()
                .unwrap_or_else(|p| p.into_inner());
            let Some(join) = task.as_mut() else {
                return std::task::Poll::Ready(());
            };
            match std::pin::Pin::new(join).poll(cx) {
                std::task::Poll::Pending => std::task::Poll::Pending,
                std::task::Poll::Ready(joined) => {
                    joined.expect("owned shutdown task failed");
                    *task = None;
                    std::task::Poll::Ready(())
                }
            }
        })
        .await;
        outcome
    }

    #[cfg(test)]
    fn shutdown_runs(&self) -> usize {
        self.inner
            .shutdown_runs
            .load(std::sync::atomic::Ordering::SeqCst)
    }
}

async fn second_stop(sender: &watch::Sender<bool>) {
    let mut second = sender.subscribe();
    let _ = second.wait_for(|second| *second).await;
}

async fn run_close(inner: &Inner) -> EmbeddedClose {
    let http = async {
        std::future::poll_fn(|cx| {
            let mut task = inner.server.lock().unwrap_or_else(|p| p.into_inner());
            let join = task.as_mut().expect("HTTP join has one shutdown owner");
            match std::pin::Pin::new(join).poll(cx) {
                std::task::Poll::Pending => std::task::Poll::Pending,
                std::task::Poll::Ready(joined) => {
                    *task = None;
                    std::task::Poll::Ready(match joined {
                        Ok(Ok(())) => Ok(ServeEnd::Drained),
                        Ok(Err(error)) => Err(error.to_string()),
                        Err(error) => Err(format!("HTTP task failed: {error}")),
                    })
                }
            }
        })
        .await
    };
    let http = tokio::select! {
        biased;
        result = http => result,
        () = second_stop(&inner.second_stop) => Ok(ServeEnd::Forced(crate::ForceReason::SecondSignal)),
        () = tokio::time::sleep(crate::STOP_GRACE) => Ok(ServeEnd::Forced(crate::ForceReason::GraceExpired)),
    };
    let bus = crate::release_bus(&inner.state, crate::BUS_RELEASE_TIMEOUT).await;
    let (writes, catalogue) = drain_owned_work(&inner.state, &inner.second_stop).await;
    EmbeddedClose {
        http,
        bus,
        writes,
        catalogue,
    }
}

async fn drain_owned_work(
    state: &AppState,
    second: &watch::Sender<bool>,
) -> (WriteDrain, CatalogueDrain) {
    tokio::join!(
        crate::drain_device_writes(state, crate::DEFAULT_WRITE_WAIT, second_stop(second)),
        crate::drain_catalogue_jobs(state, crate::CATALOGUE_DRAIN_TIMEOUT, second_stop(second)),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalogue_jobs::{AdmissionState, DrainEnd};
    use std::sync::Arc;
    use tokio::sync::Notify;

    async fn embedded() -> (
        EmbeddedServerHandle,
        Arc<crate::AppState>,
        tempfile::TempDir,
    ) {
        let directory = tempfile::tempdir().unwrap();
        let state = Arc::new(crate::AppState::new(directory.path().to_path_buf()));
        let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
            .await
            .unwrap();
        let address = listener.local_addr().unwrap();
        let handle = EmbeddedServerHandle::spawn(listener, Arc::clone(&state), None);
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let mut connection = tokio::net::TcpStream::connect(address).await.unwrap();
        connection
            .write_all(b"GET /healthz HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")
            .await
            .unwrap();
        let mut response = String::new();
        connection.read_to_string(&mut response).await.unwrap();
        assert!(
            response.starts_with("HTTP/1.1 200"),
            "actual embedded router must answer"
        );
        (handle, state, directory)
    }

    #[tokio::test]
    async fn close_uses_the_application_owner_and_is_idempotent() {
        let (handle, state, _directory) = embedded().await;
        assert_eq!(state.catalogue_jobs.admission_state(), AdmissionState::Open);
        handle.request_close();
        handle.request_close();
        assert!(state.stop_admission.is_closed());
        assert_eq!(
            state.catalogue_jobs.admission_state(),
            AdmissionState::Stopping
        );
        let result = handle.drain_close(std::future::pending()).await;
        assert_eq!(result.http, Ok(crate::ServeEnd::Drained));
        assert_eq!(result.catalogue.end, DrainEnd::NothingRunning);
        assert!(!result.requires_process_exit());
        assert_eq!(handle.shutdown_runs(), 1);
    }

    #[tokio::test]
    async fn a_cancelled_waiter_preserves_shutdown_and_worker_ownership() {
        let (handle, state, _directory) = embedded().await;
        let release = Arc::new(Notify::new());
        let worker_release = Arc::clone(&release);
        state
            .catalogue_jobs
            .register_worker(tokio::spawn(async move {
                worker_release.notified().await;
            }))
            .unwrap();
        let mut wait = Box::pin(handle.drain_close(std::future::pending()));
        std::future::poll_fn(|cx| {
            assert!(wait.as_mut().poll(cx).is_pending());
            std::task::Poll::Ready(())
        })
        .await;
        drop(wait);
        // Ordinary duplicate close is not a second stop.
        handle.request_close();
        release.notify_one();
        let result = handle.drain_close(std::future::pending()).await;
        assert_eq!(result.catalogue.end, DrainEnd::Finished);
        assert!(!result.catalogue.work_outstanding);
        assert_eq!(handle.shutdown_runs(), 1);
    }

    #[tokio::test]
    async fn explicit_second_stop_reports_outstanding_without_aborting_the_worker() {
        let (handle, state, _directory) = embedded().await;
        let (release, wait) = std::sync::mpsc::channel();
        let worker = tokio::task::spawn_blocking(move || {
            wait.recv().unwrap();
        });
        state.catalogue_jobs.register_worker(worker).unwrap();
        let result = handle.drain_close(async {}).await;
        let outstanding = result.catalogue.work_outstanding;
        let end = result.catalogue.end;
        // Clean up even when the production assertion fails.
        release.send(()).unwrap();
        let settled = state
            .catalogue_jobs
            .drain(Duration::from_secs(5), std::future::pending())
            .await;
        assert!(
            outstanding,
            "force must not claim the blocking worker ended"
        );
        assert_eq!(end, DrainEnd::SecondSignal);
        assert_eq!(settled.end, DrainEnd::Finished);
        assert!(result.requires_process_exit());
    }

    #[tokio::test(start_paused = true)]
    async fn catalogue_timeout_does_not_shorten_the_device_write_wait() {
        let state = Arc::new(crate::AppState::new(std::env::temp_dir()));
        let release_catalogue = Arc::new(Notify::new());
        let release_device = Arc::new(Notify::new());
        let catalogue_release = Arc::clone(&release_catalogue);
        state
            .catalogue_jobs
            .register_worker(tokio::spawn(async move {
                catalogue_release.notified().await;
            }))
            .unwrap();
        let device_release = Arc::clone(&release_device);
        let device = knx_core::IndividualAddress::new(1, 1, 67).unwrap();
        *state.device_download.lock().await =
            Some(crate::DeviceDownloadSession::synthetic_running(
                device,
                tokio::spawn(async move {
                    device_release.notified().await;
                }),
            ));
        let (force, _) = watch::channel(false);
        let start = tokio::time::Instant::now();
        let (writes, catalogue) = drain_owned_work(&state, &force).await;
        let elapsed = start.elapsed();
        release_device.notify_one();
        release_catalogue.notify_one();
        state
            .catalogue_jobs
            .drain(Duration::from_secs(1), std::future::pending())
            .await;
        assert_eq!(writes.end, crate::WriteWaitEnd::TimedOut);
        assert_eq!(catalogue.end, DrainEnd::TimedOut);
        assert_eq!(
            elapsed,
            crate::DEFAULT_WRITE_WAIT,
            "drains wait concurrently with independent bounds"
        );
    }
}
