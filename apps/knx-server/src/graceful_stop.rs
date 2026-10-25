//! Leaving on request: SIGTERM and SIGINT end the server in an orderly way.
//!
//! In a container `knx-server` is PID 1, and the kernel gives PID 1 no
//! default action for SIGTERM or SIGINT: a signal it has no handler for is
//! simply discarded. Before this module existed `docker stop` therefore
//! waited out its 10 seconds and sent SIGKILL (exit code 137), and a bus
//! monitor's tunnel stayed occupied on the gateway until its heartbeat
//! timed out.
//!
//! The order on a signal:
//! 1. refuse every new bus job ([`StopAdmission`]: device download, address
//!    programming, monitor, line scan, compare, serial lookup, service
//!    control), stop accepting connections and let requests in flight finish
//!    ([`STOP_GRACE`] at most; a second signal ends the wait at once),
//! 2. release the bus: stop the monitor session and cancel a line scan, so
//!    the gateway receives its `DISCONNECT_REQUEST` ([`BUS_RELEASE_TIMEOUT`]),
//! 3. wait for a device download or address programming that is already
//!    writing, up to `KNX_STOP_WRITE_WAIT_SECS` ([`DEFAULT_WRITE_WAIT`]);
//!    an address programming still waiting for its button is asked to stop
//!    through its own defined stop, and a regular download is stopped only
//!    while it has not committed its first change (ADR-0153; the stop is
//!    requested at step 1 already, by [`StopAdmission::close`]). A download
//!    past that point is waited for, never cancelled. A second signal ends
//!    this wait too,
//!    In parallel, join the catalogue owner's cooperatively stopped work
//!    for at most [`CATALOGUE_DRAIN_TIMEOUT`]. Catalogue stop was requested
//!    with admission closure, not delayed until a device wait ends.
//! 4. return only after owned work drained; otherwise explicitly exit with
//!    truthful outstanding-work diagnostics rather than blocking-runtime Drop.
//!
//! Steps 1 and 2 stay below Docker's default 10-second stop timeout; step 3
//! only waits while a device write is actually running, and Docker's own
//! timeout must be raised to cover it (`docker stop -t`). Nothing here can
//! end a partly written device safely; see the KL-163 dossier.

use std::future::{Future, IntoFuture};
use std::io;
use std::time::Duration;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, PoisonError, Weak};

use knx_net::commissioning::memory_download::{DownloadStop, StopRequest};

use tokio::sync::{mpsc, watch};

use crate::errors::ApiError;
use crate::AppState;

/// How long requests in flight get to finish after the first signal.
pub const STOP_GRACE: Duration = Duration::from_secs(5);
/// How long releasing the bus may take after the HTTP side has stopped.
pub const BUS_RELEASE_TIMEOUT: Duration = Duration::from_secs(2);
/// Catalogue work has its own bound, independent of the device safety wait.
pub const CATALOGUE_DRAIN_TIMEOUT: Duration = Duration::from_secs(30);
/// How long a stop waits for a device write already running, unless
/// `KNX_STOP_WRITE_WAIT_SECS` says otherwise.
pub const DEFAULT_WRITE_WAIT: Duration = Duration::from_secs(120);
/// The largest accepted `KNX_STOP_WRITE_WAIT_SECS`.
pub const MAX_WRITE_WAIT: Duration = Duration::from_secs(3600);
/// How often the write wait looks at the running sessions.
const WRITE_POLL: Duration = Duration::from_millis(100);

/// `KNX_STOP_WRITE_WAIT_SECS`: whole seconds from 0 (do not wait) to
/// [`MAX_WRITE_WAIT`]; absent means [`DEFAULT_WRITE_WAIT`]. Anything else is
/// refused rather than silently replaced by the default.
pub fn write_wait_from_env(value: Option<&str>) -> Result<Duration, String> {
    let Some(value) = value else {
        return Ok(DEFAULT_WRITE_WAIT);
    };
    let seconds: u64 = value
        .trim()
        .parse()
        .map_err(|_| format!("KNX_STOP_WRITE_WAIT_SECS must be whole seconds, not {value:?}"))?;
    let wait = Duration::from_secs(seconds);
    if wait > MAX_WRITE_WAIT {
        return Err(format!(
            "KNX_STOP_WRITE_WAIT_SECS is {seconds}; at most {} is accepted",
            MAX_WRITE_WAIT.as_secs()
        ));
    }
    Ok(wait)
}

/// First-stop ordering shared by signals, HTTP termination and embedders.
/// Catalogue stop is cooperative; this does not cancel device writes.
pub fn close_process_admission(state: &AppState) {
    state.stop_admission.close();
    state.catalogue_jobs.close_admission_and_request_stop();
    state.device_system2.control.close_admission();
}

/// Borrow the application's actual worker owner. Timeout/second stop retains
/// its handles and independent commit/cleanup facts, not a cancellation claim.
pub async fn drain_catalogue_jobs(
    state: &AppState,
    limit: Duration,
    second_signal: impl Future<Output = ()>,
) -> crate::CatalogueDrain {
    state.catalogue_jobs.drain(limit, second_signal).await
}

/// Whether new bus jobs may still start. Closed by the first stop signal;
/// it never opens again in this process.
#[derive(Debug, Default)]
pub struct StopAdmission {
    closed: AtomicBool,
    /// The regular download registered last. Closing and registering take
    /// this one short synchronous lock (never held across an `await`), so
    /// every registration is either refused or seen by the close.
    download: Mutex<Weak<DownloadStop>>,
}

impl StopAdmission {
    /// Closes admission and, in the same critical section, asks the
    /// registered regular download (if any is still alive) to stop before
    /// its first change. See [`close_and_request_download_stop`](Self::close_and_request_download_stop).
    pub fn close(&self) {
        self.close_and_request_download_stop();
    }

    /// [`close`](Self::close), returning what the registered download's
    /// token answered. The request is made before this returns, without the
    /// async download reservation lock: `WillStop` means the run will send
    /// no memory write, load record, property write or restart (it may
    /// still fail with its own error first); `TooLate` means its first
    /// change was already committed and it goes on unchanged; `Ended` means
    /// its run is over. `None`: no download was registered.
    pub(crate) fn close_and_request_download_stop(&self) -> Option<StopRequest> {
        let slot = self.download.lock().unwrap_or_else(PoisonError::into_inner);
        self.closed.store(true, Ordering::SeqCst);
        slot.upgrade().map(|stop| stop.request())
    }

    /// Registers the stop token of a regular download that is about to
    /// start: `503 serverStopping` once closed (nothing is started), else a
    /// fresh token that a later [`close`](Self::close) will request. Called
    /// while the route holds the download reservation, after its busy
    /// check, so the previous registration belongs to a run that has ended.
    pub(crate) fn register_download(&self) -> Result<Arc<DownloadStop>, ApiError> {
        let mut slot = self.download.lock().unwrap_or_else(PoisonError::into_inner);
        self.admit()?;
        let stop = Arc::new(DownloadStop::new());
        *slot = Arc::downgrade(&stop);
        Ok(stop)
    }

    pub fn is_closed(&self) -> bool {
        self.closed.load(Ordering::SeqCst)
    }

    /// `Ok` while running; `503 serverStopping` once a stop has begun.
    /// Called first by every route that would open a tunnel.
    pub fn admit(&self) -> Result<(), ApiError> {
        if self.is_closed() {
            return Err(ApiError::with_kind(
                axum::http::StatusCode::SERVICE_UNAVAILABLE,
                "serverStopping",
                "the server is stopping; nothing was started and nothing was sent",
            ));
        }
        Ok(())
    }
}

/// How far stopping has progressed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Stage {
    Running,
    /// A signal arrived; requests in flight may still finish.
    Requested,
    /// Stop waiting for anything.
    Forced(ForceReason),
}

/// Why the server stopped waiting for requests in flight.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ForceReason {
    /// The grace period ran out.
    GraceExpired,
    /// A second signal arrived during the grace period.
    SecondSignal,
}

/// How serving ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServeEnd {
    /// Every connection finished within the grace period.
    Drained,
    /// The server stopped waiting; open connections are dropped on exit.
    Forced(ForceReason),
}

/// The shared view of a stop in progress. Cheap to clone; every clone
/// observes the same signals.
#[derive(Clone)]
pub struct GracefulStop {
    stage: watch::Receiver<Stage>,
    /// `true` once a second signal arrived, whenever that was.
    second: watch::Receiver<bool>,
}

impl GracefulStop {
    /// Installs SIGTERM and SIGINT handlers for this process.
    ///
    /// Must be called inside the Tokio runtime and before the server starts
    /// listening, so a signal that arrives during start-up is not lost.
    pub fn install() -> io::Result<Self> {
        use tokio::signal::unix::{signal, SignalKind};
        let mut term = signal(SignalKind::terminate())?;
        let mut int = signal(SignalKind::interrupt())?;
        let (tx, rx) = mpsc::unbounded_channel();
        tokio::spawn(async move {
            loop {
                let name = tokio::select! {
                    _ = term.recv() => "SIGTERM",
                    _ = int.recv() => "SIGINT",
                };
                eprintln!("knx-server: {name} received");
                if tx.send(()).is_err() {
                    return;
                }
            }
        });
        Ok(Self::from_signals(rx, STOP_GRACE))
    }

    /// The state machine behind [`install`](Self::install): every message on
    /// `signals` is one signal. Tests drive it without real signals.
    fn from_signals(mut signals: mpsc::UnboundedReceiver<()>, grace: Duration) -> Self {
        let (tx, rx) = watch::channel(Stage::Running);
        let (second_tx, second_rx) = watch::channel(false);
        tokio::spawn(async move {
            if signals.recv().await.is_none() || tx.send(Stage::Requested).is_err() {
                return;
            }
            eprintln!(
                "knx-server: stopping; open requests have {}s to finish \
                 (send the signal again to stop at once)",
                grace.as_secs()
            );
            let reason = tokio::select! {
                _ = tokio::time::sleep(grace) => ForceReason::GraceExpired,
                Some(()) = signals.recv() => ForceReason::SecondSignal,
            };
            let _ = tx.send(Stage::Forced(reason));
            // A second signal after the grace still ends the write wait.
            if reason == ForceReason::SecondSignal || signals.recv().await.is_some() {
                let _ = second_tx.send(true);
            }
        });
        GracefulStop {
            stage: rx,
            second: second_rx,
        }
    }

    /// Resolves once a second signal has arrived, during the grace period or
    /// after it. Never resolves otherwise.
    pub fn second_signal(&self) -> impl Future<Output = ()> + Send + 'static {
        let mut second = self.second.clone();
        async move {
            if second.wait_for(|arrived| *arrived).await.is_err() {
                std::future::pending::<()>().await;
            }
        }
    }

    /// Resolves once a stop has been requested. This is the future handed
    /// to `axum::serve(..).with_graceful_shutdown`.
    pub fn requested(&self) -> impl Future<Output = ()> + Send + 'static {
        let mut stage = self.stage.clone();
        async move {
            // An `Err` means the signal task is gone, which only happens at
            // process exit; never treat that as a request to stop.
            if stage.wait_for(|s| *s != Stage::Running).await.is_err() {
                std::future::pending::<()>().await;
            }
        }
    }

    /// Resolves once waiting for requests in flight should end.
    async fn forced(&self) -> ForceReason {
        let mut stage = self.stage.clone();
        let reason = match stage.wait_for(|s| matches!(s, Stage::Forced(_))).await {
            Ok(stage) => match *stage {
                Stage::Forced(reason) => Some(reason),
                Stage::Running | Stage::Requested => None,
            },
            Err(_) => None,
        };
        match reason {
            Some(reason) => reason,
            None => std::future::pending().await,
        }
    }

    /// Runs `server` (already wired to [`requested`](Self::requested) through
    /// `with_graceful_shutdown`) until it has drained or the grace period
    /// is over, whichever comes first.
    pub async fn serve<S>(&self, server: S) -> io::Result<ServeEnd>
    where
        S: IntoFuture<Output = io::Result<()>>,
    {
        tokio::select! {
            result = server.into_future() => result.map(|()| ServeEnd::Drained),
            reason = self.forced() => Ok(ServeEnd::Forced(reason)),
        }
    }
}

/// What [`release_bus`] did.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct BusRelease {
    pub monitor_stopped: bool,
    pub line_scan_cancelled: bool,
    /// The release did not finish within its time limit.
    pub timed_out: bool,
}

/// Stops the bus monitor session and cancels a running line scan so their
/// tunnels are disconnected rather than abandoned. Bounded by `limit`.
pub async fn release_bus(state: &AppState, limit: Duration) -> BusRelease {
    let mut release = BusRelease::default();
    let work = async {
        let monitor = state.bus_session.lock().await.take();
        if let Some(session) = monitor {
            session.stop().await;
            release.monitor_stopped = true;
        }
        let scan = state.line_scan_session.lock().await.take();
        if let Some(mut scan) = scan {
            scan.cancel().await;
            release.line_scan_cancelled = true;
        }
    };
    if tokio::time::timeout(limit, work).await.is_err() {
        release.timed_out = true;
    }
    release
}

/// How the wait for device writes ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WriteWaitEnd {
    /// No device write was running when the wait began.
    NothingRunning,
    /// Every running device write ended within the limit.
    Finished,
    /// The limit passed with a device write still running.
    TimedOut,
    /// A second signal ended the wait with a device write still running.
    SecondSignal,
}

/// What [`drain_device_writes`] found and did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WriteDrain {
    /// A download to this device was running when the wait began.
    pub download: Option<knx_core::IndividualAddress>,
    /// An address programming was still waiting for its button and was
    /// asked to stop through its own stop (nothing had been written yet).
    pub programming_wait_stopped: bool,
    /// An address programming had already begun writing (MP §2.3).
    pub programming_writing: Option<knx_core::IndividualAddress>,
    /// What a repeated stop request to that download answered. A second
    /// request is a diagnostic, not the guarantee: the first one was made
    /// by [`StopAdmission::close`].
    pub download_stop: Option<StopRequest>,
    pub end: WriteWaitEnd,
}

/// Step 3 of a stop: waits up to `limit` for a device download or address
/// programming that is already running, so a stop does not cut a device
/// write short when it does not have to. Only an address programming that
/// is still waiting for its button is stopped (its own defined stop), and a
/// regular download is asked once more to stop before its first change;
/// a running write is never cancelled here. `second_signal` ends the wait.
pub async fn drain_device_writes(
    state: &AppState,
    limit: Duration,
    second_signal: impl Future<Output = ()>,
) -> WriteDrain {
    let mut drain = WriteDrain {
        download: None,
        programming_wait_stopped: false,
        programming_writing: None,
        download_stop: None,
        end: WriteWaitEnd::NothingRunning,
    };
    if let Ok(programming_guard) = state.address_programming.try_lock() {
        if let Some(programming) = programming_guard.as_ref() {
            if programming.is_active() {
                match programming.request_stop() {
                    crate::address_programming::StopOutcome::WillStop => {
                        drain.programming_wait_stopped = true;
                    }
                    crate::address_programming::StopOutcome::TooLate => {
                        drain.programming_writing = Some(programming.new_address());
                    }
                }
            }
        }
    }
    if let Ok(download_guard) = state.device_download.try_lock() {
        if let Some(download) = download_guard.as_ref() {
            if download.is_running() {
                drain.download = Some(download.target());
                drain.download_stop = Some(download.request_stop());
            }
        }
    }
    if !device_writes_active(state) {
        return drain;
    }
    let settled = async {
        while device_writes_active(state) {
            tokio::time::sleep(WRITE_POLL).await;
        }
    };
    drain.end = tokio::select! {
        () = settled => WriteWaitEnd::Finished,
        () = tokio::time::sleep(limit) => WriteWaitEnd::TimedOut,
        () = second_signal => WriteWaitEnd::SecondSignal,
    };
    drain
}

fn device_writes_active(state: &AppState) -> bool {
    // System-2 holds these same admission locks through disconnect. Inspectors
    // must not wait outside the timer/second-signal boundary. A busy lock is
    // unknown active work, never proof that an ordinary write has ended.
    let download = state
        .device_download
        .try_lock()
        .map(|s| {
            s.as_ref()
                .is_some_and(crate::DeviceDownloadSession::is_running)
        })
        .unwrap_or(true);
    let programming = state
        .address_programming
        .try_lock()
        .map(|s| {
            s.as_ref()
                .is_some_and(crate::AddressProgrammingSession::is_active)
        })
        .unwrap_or(true);
    download || programming
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fake::{FakeConnector, FakeTunnel};
    use crate::{BusSession, GroupAddressContext};
    use knx_core::IndividualAddress;

    /// A [`GracefulStop`] whose "signals" are sends on the returned channel.
    fn channel_stop(grace: Duration) -> (GracefulStop, mpsc::UnboundedSender<()>) {
        let (tx, rx) = mpsc::unbounded_channel();
        (GracefulStop::from_signals(rx, grace), tx)
    }

    #[tokio::test(start_paused = true)]
    async fn nothing_happens_without_a_signal() {
        let (stop, _tx) = channel_stop(Duration::from_secs(5));
        let waited = tokio::time::timeout(Duration::from_secs(3600), stop.requested()).await;
        assert!(waited.is_err(), "requested() resolved without a signal");
    }

    #[tokio::test(start_paused = true)]
    async fn a_server_that_drains_in_time_ends_drained() {
        let (stop, tx) = channel_stop(Duration::from_secs(5));
        let requested = stop.requested();
        let server = async move {
            requested.await;
            tokio::time::sleep(Duration::from_secs(1)).await;
            Ok(())
        };
        tx.send(()).unwrap();
        assert_eq!(stop.serve(server).await.unwrap(), ServeEnd::Drained);
    }

    #[tokio::test(start_paused = true)]
    async fn a_server_that_hangs_is_given_up_on_after_the_grace() {
        let (stop, tx) = channel_stop(Duration::from_secs(5));
        let server = std::future::pending::<io::Result<()>>();
        tx.send(()).unwrap();
        let started = tokio::time::Instant::now();
        assert_eq!(
            stop.serve(server).await.unwrap(),
            ServeEnd::Forced(ForceReason::GraceExpired)
        );
        assert_eq!(started.elapsed(), Duration::from_secs(5));
    }

    #[tokio::test(start_paused = true)]
    async fn a_second_signal_ends_the_grace_at_once() {
        let (stop, tx) = channel_stop(Duration::from_secs(5));
        let server = std::future::pending::<io::Result<()>>();
        tx.send(()).unwrap();
        tx.send(()).unwrap();
        let started = tokio::time::Instant::now();
        assert_eq!(
            stop.serve(server).await.unwrap(),
            ServeEnd::Forced(ForceReason::SecondSignal)
        );
        assert!(started.elapsed() < Duration::from_secs(5));
    }

    #[tokio::test(start_paused = true)]
    async fn a_server_error_is_passed_on() {
        let (stop, _tx) = channel_stop(Duration::from_secs(5));
        let server = async { Err(io::Error::other("listener died")) };
        let error = stop.serve(server).await.unwrap_err();
        assert_eq!(error.to_string(), "listener died");
    }

    #[tokio::test]
    async fn releasing_the_bus_disconnects_the_monitor_tunnel() {
        let address = IndividualAddress::new(1, 1, 5).unwrap();
        let (tunnel, handle) = FakeTunnel::new(address, 16);
        let connector = FakeConnector::succeeding(tunnel);
        let session = BusSession::start(
            1,
            "127.0.0.1:3671".parse().unwrap(),
            &connector,
            GroupAddressContext::from_project(None),
        )
        .await
        .unwrap();
        let state = AppState::new(std::env::temp_dir());
        *state.bus_session.lock().await = Some(session);

        let release = release_bus(&state, Duration::from_secs(2)).await;

        assert_eq!(
            release,
            BusRelease {
                monitor_stopped: true,
                line_scan_cancelled: false,
                timed_out: false,
            }
        );
        assert!(handle.disconnected(), "the tunnel was not disconnected");
        assert!(state.bus_session.lock().await.is_none());
    }

    #[tokio::test(start_paused = true)]
    async fn a_second_signal_after_the_grace_is_still_seen() {
        let (stop, tx) = channel_stop(Duration::from_secs(5));
        tx.send(()).unwrap();
        assert_eq!(
            stop.serve(std::future::pending::<io::Result<()>>())
                .await
                .unwrap(),
            ServeEnd::Forced(ForceReason::GraceExpired)
        );
        let early = tokio::time::timeout(Duration::from_secs(60), stop.second_signal()).await;
        assert!(early.is_err(), "no second signal was sent yet");
        tx.send(()).unwrap();
        tokio::time::timeout(Duration::from_secs(1), stop.second_signal())
            .await
            .expect("the late second signal ends the write wait");
    }

    #[tokio::test(start_paused = true)]
    async fn a_second_signal_during_the_grace_counts_for_the_write_wait_too() {
        let (stop, tx) = channel_stop(Duration::from_secs(5));
        tx.send(()).unwrap();
        tx.send(()).unwrap();
        stop.serve(std::future::pending::<io::Result<()>>())
            .await
            .unwrap();
        tokio::time::timeout(Duration::from_millis(1), stop.second_signal())
            .await
            .expect("already seen");
    }

    #[test]
    fn the_write_wait_comes_from_the_environment_and_refuses_nonsense() {
        assert_eq!(write_wait_from_env(None), Ok(DEFAULT_WRITE_WAIT));
        assert_eq!(write_wait_from_env(Some("0")), Ok(Duration::ZERO));
        assert_eq!(
            write_wait_from_env(Some(" 300 ")),
            Ok(Duration::from_secs(300))
        );
        assert_eq!(write_wait_from_env(Some("3600")), Ok(MAX_WRITE_WAIT));
        for bad in ["", "-1", "1.5", "2m", "3601", "18446744073709551616"] {
            assert!(write_wait_from_env(Some(bad)).is_err(), "{bad:?}");
        }
    }

    #[test]
    fn admission_closes_once_and_refuses_with_a_named_kind() {
        let admission = StopAdmission::default();
        assert!(admission.admit().is_ok());
        admission.close();
        let refused = admission.admit().unwrap_err();
        assert!(
            format!("{refused:?}").contains("serverStopping"),
            "{refused:?}"
        );
        let response = axum::response::IntoResponse::into_response(refused);
        assert_eq!(
            response.status(),
            axum::http::StatusCode::SERVICE_UNAVAILABLE
        );
        admission.close();
        assert!(admission.is_closed());
    }

    fn device() -> IndividualAddress {
        IndividualAddress::new(1, 1, 67).unwrap()
    }

    /// A download whose worker runs until `release` is notified.
    fn blocked_download(
        release: std::sync::Arc<tokio::sync::Notify>,
    ) -> crate::DeviceDownloadSession {
        let task = tokio::spawn(async move { release.notified().await });
        crate::DeviceDownloadSession::synthetic_running(device(), task)
    }

    #[tokio::test(start_paused = true)]
    async fn nothing_running_means_no_wait_at_all() {
        let state = AppState::new(std::env::temp_dir());
        let started = tokio::time::Instant::now();
        let drain =
            drain_device_writes(&state, Duration::from_secs(120), std::future::pending()).await;
        assert_eq!(drain.end, WriteWaitEnd::NothingRunning);
        assert_eq!(drain.download, None);
        assert_eq!(started.elapsed(), Duration::ZERO);
    }

    #[tokio::test(start_paused = true)]
    async fn a_running_download_is_waited_for_until_it_ends() {
        let state = AppState::new(std::env::temp_dir());
        let release = std::sync::Arc::new(tokio::sync::Notify::new());
        *state.device_download.lock().await = Some(blocked_download(release.clone()));
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_secs(30)).await;
            release.notify_one();
        });
        let started = tokio::time::Instant::now();

        let drain =
            drain_device_writes(&state, Duration::from_secs(120), std::future::pending()).await;

        assert_eq!(drain.end, WriteWaitEnd::Finished);
        assert_eq!(drain.download, Some(device()));
        assert!(started.elapsed() >= Duration::from_secs(30));
        assert!(started.elapsed() < Duration::from_secs(31));
    }

    #[tokio::test(start_paused = true)]
    async fn the_wait_is_bounded_and_never_cancels_the_download() {
        let state = AppState::new(std::env::temp_dir());
        let release = std::sync::Arc::new(tokio::sync::Notify::new());
        *state.device_download.lock().await = Some(blocked_download(release.clone()));
        let started = tokio::time::Instant::now();

        let drain =
            drain_device_writes(&state, Duration::from_secs(120), std::future::pending()).await;

        assert_eq!(drain.end, WriteWaitEnd::TimedOut);
        assert!(started.elapsed() >= Duration::from_secs(120));
        assert!(started.elapsed() < Duration::from_secs(121));
        assert!(
            state
                .device_download
                .lock()
                .await
                .as_ref()
                .unwrap()
                .is_running(),
            "the stop never cancels a write it cannot end safely"
        );
        release.notify_one();
    }

    #[tokio::test(start_paused = true)]
    async fn a_second_signal_ends_the_write_wait() {
        let state = AppState::new(std::env::temp_dir());
        let release = std::sync::Arc::new(tokio::sync::Notify::new());
        *state.device_download.lock().await = Some(blocked_download(release.clone()));
        let started = tokio::time::Instant::now();

        let drain = drain_device_writes(
            &state,
            Duration::from_secs(120),
            tokio::time::sleep(Duration::from_secs(7)),
        )
        .await;

        assert_eq!(drain.end, WriteWaitEnd::SecondSignal);
        assert!(started.elapsed() < Duration::from_secs(8));
        release.notify_one();
    }

    #[tokio::test(start_paused = true)]
    async fn a_zero_wait_reports_the_running_download_without_waiting() {
        let state = AppState::new(std::env::temp_dir());
        let release = std::sync::Arc::new(tokio::sync::Notify::new());
        *state.device_download.lock().await = Some(blocked_download(release.clone()));
        let drain = drain_device_writes(&state, Duration::ZERO, std::future::pending()).await;
        assert_eq!(drain.end, WriteWaitEnd::TimedOut);
        assert_eq!(drain.download, Some(device()));
        release.notify_one();
    }

    #[tokio::test(start_paused = true)]
    async fn a_programming_still_waiting_for_its_button_is_stopped_through_its_own_stop() {
        let state = AppState::new(std::env::temp_dir());
        *state.address_programming.lock().await =
            Some(crate::AddressProgrammingSession::synthetic(device(), false));

        let drain =
            drain_device_writes(&state, Duration::from_secs(120), std::future::pending()).await;

        assert_eq!(drain.end, WriteWaitEnd::Finished);
        assert!(drain.programming_wait_stopped);
        assert_eq!(drain.programming_writing, None);
        assert!(matches!(
            state
                .address_programming
                .lock()
                .await
                .as_ref()
                .unwrap()
                .status(),
            crate::address_programming::AddressProgrammingStatus::Stopped { .. }
        ));
    }

    #[tokio::test(start_paused = true)]
    async fn a_programming_that_already_writes_is_waited_for_not_stopped() {
        let state = AppState::new(std::env::temp_dir());
        *state.address_programming.lock().await =
            Some(crate::AddressProgrammingSession::synthetic(device(), true));
        let drain =
            drain_device_writes(&state, Duration::from_secs(5), std::future::pending()).await;
        assert_eq!(drain.end, WriteWaitEnd::TimedOut);
        assert!(!drain.programming_wait_stopped);
        assert_eq!(drain.programming_writing, Some(device()));

        state
            .address_programming
            .lock()
            .await
            .as_ref()
            .unwrap()
            .synthetic_finish();
        let drain =
            drain_device_writes(&state, Duration::from_secs(5), std::future::pending()).await;
        assert_eq!(drain.end, WriteWaitEnd::NothingRunning);
    }

    #[test]
    fn registering_after_the_close_is_refused_and_starts_nothing() {
        let admission = StopAdmission::default();
        assert_eq!(admission.close_and_request_download_stop(), None);
        let refused = admission.register_download().unwrap_err();
        assert!(format!("{refused:?}").contains("serverStopping"));
    }

    #[test]
    fn registering_before_the_close_gets_that_exact_token_requested() {
        let admission = StopAdmission::default();
        let token = admission.register_download().expect("open");
        assert!(!token.stop_pending());
        assert_eq!(
            admission.close_and_request_download_stop(),
            Some(StopRequest::WillStop)
        );
        assert!(token.stop_pending());
        assert!(admission.is_closed());
    }

    #[test]
    fn a_dropped_registration_is_not_requested_by_a_later_close() {
        let admission = StopAdmission::default();
        drop(admission.register_download().expect("open"));
        assert_eq!(admission.close_and_request_download_stop(), None);
    }

    #[test]
    fn a_new_registration_replaces_the_previous_slot_and_only_it_is_requested() {
        let admission = StopAdmission::default();
        let first = admission.register_download().expect("open");
        let second = admission.register_download().expect("open");
        assert_eq!(
            admission.close_and_request_download_stop(),
            Some(StopRequest::WillStop)
        );
        assert!(second.stop_pending());
        assert!(
            !first.stop_pending(),
            "the earlier run's token is untouched"
        );
    }

    #[test]
    fn every_racing_registration_is_either_refused_or_requested_by_the_close() {
        for _ in 0..500 {
            let admission = std::sync::Arc::new(StopAdmission::default());
            let registering = {
                let admission = std::sync::Arc::clone(&admission);
                std::thread::spawn(move || admission.register_download())
            };
            let closing = {
                let admission = std::sync::Arc::clone(&admission);
                std::thread::spawn(move || admission.close_and_request_download_stop())
            };
            let registered = registering.join().unwrap();
            closing.join().unwrap();
            match registered {
                Ok(token) => assert!(token.stop_pending(), "the close missed a registration"),
                Err(refused) => assert!(format!("{refused:?}").contains("serverStopping")),
            }
        }
    }

    #[tokio::test(start_paused = true)]
    async fn a_download_past_its_first_change_answers_too_late_and_keeps_running() {
        let state = AppState::new(std::env::temp_dir());
        let release = std::sync::Arc::new(tokio::sync::Notify::new());
        *state.device_download.lock().await = Some(blocked_download(release.clone()));
        let drain =
            drain_device_writes(&state, Duration::from_secs(5), std::future::pending()).await;
        assert_eq!(drain.download_stop, Some(StopRequest::TooLate));
        assert_eq!(drain.end, WriteWaitEnd::TimedOut);
        assert!(state
            .device_download
            .lock()
            .await
            .as_ref()
            .unwrap()
            .is_running());
        release.notify_one();
    }

    #[tokio::test(start_paused = true)]
    async fn no_download_means_no_download_stop_answer() {
        let state = AppState::new(std::env::temp_dir());
        let drain =
            drain_device_writes(&state, Duration::from_secs(5), std::future::pending()).await;
        assert_eq!(drain.download_stop, None);
    }

    #[tokio::test]
    async fn releasing_an_idle_bus_does_nothing() {
        let state = AppState::new(std::env::temp_dir());
        assert_eq!(
            release_bus(&state, Duration::from_secs(2)).await,
            BusRelease::default()
        );
    }
}
