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
//!    through its own defined stop. A second signal ends this wait too,
//! 4. exit.
//!
//! Steps 1 and 2 stay below Docker's default 10-second stop timeout; step 3
//! only waits while a device write is actually running, and Docker's own
//! timeout must be raised to cover it (`docker stop -t`). Nothing here can
//! end a partly written device safely; see the KL-163 dossier.

use std::future::{Future, IntoFuture};
use std::io;
use std::time::Duration;

use std::sync::atomic::{AtomicBool, Ordering};

use tokio::sync::{mpsc, watch};

use crate::errors::ApiError;
use crate::AppState;

/// How long requests in flight get to finish after the first signal.
pub const STOP_GRACE: Duration = Duration::from_secs(5);
/// How long releasing the bus may take after the HTTP side has stopped.
pub const BUS_RELEASE_TIMEOUT: Duration = Duration::from_secs(2);
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

/// Whether new bus jobs may still start. Closed by the first stop signal;
/// it never opens again in this process.
#[derive(Debug, Default)]
pub struct StopAdmission {
    closed: AtomicBool,
}

impl StopAdmission {
    pub fn close(&self) {
        self.closed.store(true, Ordering::SeqCst);
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
    pub end: WriteWaitEnd,
}

/// Step 3 of a stop: waits up to `limit` for a device download or address
/// programming that is already running, so a stop does not cut a device
/// write short when it does not have to. Only an address programming that
/// is still waiting for its button is stopped (its own defined stop); a
/// running write is never cancelled here. `second_signal` ends the wait.
pub async fn drain_device_writes(
    state: &AppState,
    limit: Duration,
    second_signal: impl Future<Output = ()>,
) -> WriteDrain {
    let mut drain = WriteDrain {
        download: None,
        programming_wait_stopped: false,
        programming_writing: None,
        end: WriteWaitEnd::NothingRunning,
    };
    if let Some(programming) = state.address_programming.lock().await.as_ref() {
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
    if let Some(download) = state.device_download.lock().await.as_ref() {
        if download.is_running() {
            drain.download = Some(download.target());
        }
    }
    if !device_writes_active(state).await {
        return drain;
    }
    let settled = async {
        while device_writes_active(state).await {
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

async fn device_writes_active(state: &AppState) -> bool {
    let download = state
        .device_download
        .lock()
        .await
        .as_ref()
        .is_some_and(crate::DeviceDownloadSession::is_running);
    let programming = state
        .address_programming
        .lock()
        .await
        .as_ref()
        .is_some_and(crate::AddressProgrammingSession::is_active);
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

    #[tokio::test]
    async fn releasing_an_idle_bus_does_nothing() {
        let state = AppState::new(std::env::temp_dir());
        assert_eq!(
            release_bus(&state, Duration::from_secs(2)).await,
            BusRelease::default()
        );
    }
}
