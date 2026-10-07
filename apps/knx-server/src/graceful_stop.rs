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
//! 1. stop accepting connections and let requests in flight finish
//!    ([`STOP_GRACE`] at most; a second signal ends the wait at once),
//! 2. release the bus: stop the monitor session and cancel a line scan, so
//!    the gateway receives its `DISCONNECT_REQUEST` ([`BUS_RELEASE_TIMEOUT`]),
//! 3. exit.
//!
//! Both limits together stay below Docker's default 10-second stop timeout.
//! A device download or address programming still running at that point is
//! not waited for; see docs/KNOWN_LIMITATIONS.md §163.

use std::future::{Future, IntoFuture};
use std::io;
use std::time::Duration;

use tokio::sync::{mpsc, watch};

use crate::AppState;

/// How long requests in flight get to finish after the first signal.
pub const STOP_GRACE: Duration = Duration::from_secs(5);
/// How long releasing the bus may take after the HTTP side has stopped.
pub const BUS_RELEASE_TIMEOUT: Duration = Duration::from_secs(2);

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
        });
        GracefulStop { stage: rx }
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

    #[tokio::test]
    async fn releasing_an_idle_bus_does_nothing() {
        let state = AppState::new(std::env::temp_dir());
        assert_eq!(
            release_bus(&state, Duration::from_secs(2)).await,
            BusRelease::default()
        );
    }
}
