//! Owns one cancellable, incrementally readable line-scan session.

use std::net::SocketAddrV4;
use std::sync::{Arc, Mutex};

use knx_core::scan::ScanPlan;
use knx_core::IndividualAddress;
use knx_net::{
    scan_line, ApplicationService, BusError, Destination, ProbeOutcome, ProbePolicy, ScanError,
    ScanTransport, ScannedRange, Tpci,
};
use tokio::sync::oneshot;
use tokio::task::JoinHandle;

use crate::bus::{BusSessionError, BusTunnel, GatewayConnector};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LineScanStatus {
    Running,
    Completed,
    Cancelled,
    Failed(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LineScanResult {
    pub address: IndividualAddress,
    pub outcome: ProbeOutcome,
}

struct SharedScan {
    status: Mutex<LineScanStatus>,
    results: Mutex<Vec<LineScanResult>>,
    // Readers borrow under this lock, never clone tunnel ownership into a snapshot.
    lifecycle: Mutex<ScanLifecycle>,
}

struct ScanLifecycle {
    tunnel: Option<Arc<Box<dyn BusTunnel>>>,
    skipped_frames: Option<u64>,
    disconnect_confirmed: Option<bool>,
    cleanup_error: Option<String>,
}

pub struct ScanSnapshot {
    pub status: LineScanStatus,
    pub next_since: usize,
    pub results: Vec<LineScanResult>,
    pub skipped_frames: Option<u64>,
    pub disconnect_confirmed: Option<bool>,
    pub cleanup_error: Option<String>,
}

pub struct LineScanSession {
    id: u64,
    transport: knx_net::TunnelTransport,
    range: ScannedRange,
    total_count: usize,
    omitted: Vec<IndividualAddress>,
    excluded: Vec<IndividualAddress>,
    shared: Arc<SharedScan>,
    cancel: Option<oneshot::Sender<()>>,
    task: Option<JoinHandle<()>>,
}

pub struct LineScanTarget {
    pub range: ScannedRange,
    pub gateway: SocketAddrV4,
    pub transport: knx_net::TunnelTransport,
}

impl LineScanSession {
    pub async fn start(
        id: u64,
        range: ScannedRange,
        gateway: SocketAddrV4,
        connector: &dyn GatewayConnector,
        plan: ScanPlan,
        policy: ProbePolicy,
        recorder: knx_app::operation_recording::OperationRecorder,
    ) -> Result<Self, BusSessionError> {
        Self::start_with_transport(
            id,
            LineScanTarget {
                range,
                gateway,
                transport: knx_net::TunnelTransport::Udp,
            },
            connector,
            plan,
            policy,
            recorder,
        )
        .await
    }

    pub async fn start_with_transport(
        id: u64,
        target: LineScanTarget,
        connector: &dyn GatewayConnector,
        plan: ScanPlan,
        policy: ProbePolicy,
        recorder: knx_app::operation_recording::OperationRecorder,
    ) -> Result<Self, BusSessionError> {
        let LineScanTarget {
            range,
            gateway,
            transport: requested_transport,
        } = target;
        recorder.note(
            "plan",
            serde_json::json!({
                "addresses": plan.addresses().iter().map(ToString::to_string).collect::<Vec<_>>(),
                "policy": format!("{policy:?}"),
            }),
        );
        let tunnel = match connector
            .connect_tunnel_recorded_with_transport(
                gateway,
                knx_net::TunnelAckWait::Standard,
                recorder.wire_tap(),
                requested_transport,
            )
            .await
        {
            Ok(tunnel) => tunnel,
            Err(error) => {
                crate::operation_recordings::finish_error(
                    &recorder,
                    &error,
                    serde_json::Value::Null,
                );
                return Err(error);
            }
        };
        let transport = tunnel.transport();
        if requested_transport == knx_net::TunnelTransport::Tcp && transport != requested_transport
        {
            let cleanup = tokio::spawn(async move { tunnel.disconnect().await }).await;
            let cleanup_error = match cleanup {
                Ok(Ok(())) => None,
                Ok(Err(error)) => Some(error.to_string()),
                Err(error) => Some(format!("disconnect task stopped unexpectedly: {error}")),
            };
            let reason = "explicit TCP scan received a non-TCP tunnel";
            crate::operation_recordings::finish_failed(
                &recorder,
                reason,
                serde_json::json!({
                    "disconnectConfirmed": cleanup_error.is_none(),
                    "cleanupError": cleanup_error,
                }),
            );
            return Err(BusSessionError::Transport(BusError::Protocol(
                reason.into(),
            )));
        }
        let tunnel = Arc::new(tunnel);
        let total_count = plan.addresses().len();
        let omitted = plan.omitted().to_vec();
        let mut excluded: Vec<_> = plan.excluded().iter().copied().collect();
        excluded.sort_unstable_by_key(|address| address.raw());
        let shared = Arc::new(SharedScan {
            status: Mutex::new(LineScanStatus::Running),
            results: Mutex::new(Vec::with_capacity(total_count)),
            lifecycle: Mutex::new(ScanLifecycle {
                tunnel: Some(Arc::clone(&tunnel)),
                skipped_frames: None,
                disconnect_confirmed: None,
                cleanup_error: None,
            }),
        });
        let task_shared = Arc::clone(&shared);
        let (cancel_tx, cancel_rx) = oneshot::channel();
        let task = tokio::spawn(async move {
            run_scan(tunnel, plan, policy, cancel_rx, task_shared, recorder).await;
        });
        Ok(Self {
            id,
            transport,
            range,
            total_count,
            omitted,
            excluded,
            shared,
            cancel: Some(cancel_tx),
            task: Some(task),
        })
    }

    pub fn id(&self) -> u64 {
        self.id
    }

    pub fn range(&self) -> ScannedRange {
        self.range
    }

    pub fn total_count(&self) -> usize {
        self.total_count
    }

    pub fn omitted(&self) -> &[IndividualAddress] {
        &self.omitted
    }

    pub fn excluded(&self) -> &[IndividualAddress] {
        &self.excluded
    }

    pub fn status(&self) -> LineScanStatus {
        self.shared
            .status
            .lock()
            .expect("scan status poisoned")
            .clone()
    }

    pub fn results_since(&self, since: usize) -> (usize, Vec<LineScanResult>) {
        let results = self.shared.results.lock().expect("scan results poisoned");
        let start = since.min(results.len());
        (results.len(), results[start..].to_vec())
    }

    /// Takes a coherent progress snapshot. Reading status first guarantees that
    /// a terminal status can only be paired with results read after the worker
    /// published its final callback.
    pub fn snapshot_since(&self, since: usize) -> (LineScanStatus, usize, Vec<LineScanResult>) {
        self.snapshot_since_after_status(since, || {})
    }

    fn snapshot_since_after_status(
        &self,
        since: usize,
        after_status: impl FnOnce(),
    ) -> (LineScanStatus, usize, Vec<LineScanResult>) {
        let status = self.status();
        after_status();
        let (next_since, results) = self.results_since(since);
        (status, next_since, results)
    }

    pub fn transport(&self) -> knx_net::TunnelTransport {
        self.transport
    }

    /// Status is read first, so terminal snapshots include settled cleanup metadata.
    pub fn snapshot(&self, since: usize) -> ScanSnapshot {
        let (status, next_since, results) = self.snapshot_since(since);
        let lifecycle = self
            .shared
            .lifecycle
            .lock()
            .expect("scan lifecycle poisoned");
        let skipped_frames = (self.transport == knx_net::TunnelTransport::Tcp).then(|| {
            lifecycle.tunnel.as_ref().map_or_else(
                || lifecycle.skipped_frames.expect("frozen before disconnect"),
                |tunnel| tunnel.skipped_frames(),
            )
        });
        let running = status == LineScanStatus::Running;
        ScanSnapshot {
            status,
            next_since,
            results,
            skipped_frames,
            disconnect_confirmed: if running {
                None
            } else {
                lifecycle.disconnect_confirmed
            },
            cleanup_error: if running {
                None
            } else {
                lifecycle.cleanup_error.clone()
            },
        }
    }

    pub async fn cancel(&mut self) -> LineScanStatus {
        if let Some(cancel) = self.cancel.take() {
            let _ = cancel.send(());
        }
        if let Some(task) = self.task.as_mut() {
            if task.await.is_err() {
                let mut lifecycle = self
                    .shared
                    .lifecycle
                    .lock()
                    .expect("scan lifecycle poisoned");
                lifecycle.disconnect_confirmed = Some(false);
                lifecycle.cleanup_error =
                    Some("scan owner stopped unexpectedly; cleanup unconfirmed".into());
                *self.shared.status.lock().expect("scan status poisoned") =
                    LineScanStatus::Failed("scan task stopped unexpectedly".to_string());
            }
            self.task.take();
        }
        self.status()
    }
}

struct TunnelScanTransport<'a>(&'a dyn BusTunnel);

impl ScanTransport for TunnelScanTransport<'_> {
    fn assigned_address(&self) -> IndividualAddress {
        self.0.assigned_address()
    }

    fn subscribe(&self) -> tokio::sync::broadcast::Receiver<knx_net::TunnelEvent> {
        self.0.subscribe()
    }

    async fn send_frame(
        &self,
        destination: Destination,
        transport: Tpci,
        service: ApplicationService,
    ) -> Result<(), BusError> {
        self.0.send_frame(destination, transport, service).await
    }
}

async fn run_scan(
    tunnel: Arc<Box<dyn BusTunnel>>,
    plan: ScanPlan,
    policy: ProbePolicy,
    cancel: oneshot::Receiver<()>,
    shared: Arc<SharedScan>,
    recorder: knx_app::operation_recording::OperationRecorder,
) {
    let actual_transport = tunnel.transport();
    // The supervisor owns the tunnel even when a probe panics or is cancelled.
    let probe_tunnel = Arc::clone(&tunnel);
    let progress_shared = Arc::clone(&shared);
    let mut probe = tokio::spawn(async move {
        let transport = TunnelScanTransport(probe_tunnel.as_ref().as_ref());
        scan_line(&transport, &plan, &policy, move |address, outcome| {
            progress_shared
                .results
                .lock()
                .expect("scan results poisoned")
                .push(LineScanResult { address, outcome });
        })
        .await
    });
    let result = tokio::select! {
        _ = cancel => {
            probe.abort();
            let _ = probe.await; // Reap before consuming the tunnel.
            None
        },
        result = &mut probe => Some(result),
    };
    let status = match result {
        None => LineScanStatus::Cancelled,
        Some(Ok(Ok(_))) => LineScanStatus::Completed,
        Some(Ok(Err(ScanError::Plan(error)))) => LineScanStatus::Failed(error.to_string()),
        Some(Ok(Err(ScanError::Transport { source, .. }))) => {
            LineScanStatus::Failed(source.to_string())
        }
        Some(Err(error)) => {
            LineScanStatus::Failed(format!("scan task stopped unexpectedly: {error}"))
        }
    };
    // Freeze the last pre-disconnect count under the same lock used by snapshot readers.
    // Only the supervisor and this slot ever own Arcs; snapshot readers only borrow.
    {
        let mut lifecycle = shared.lifecycle.lock().expect("scan lifecycle poisoned");
        lifecycle.skipped_frames = Some(tunnel.skipped_frames());
        lifecycle.tunnel.take();
    }
    let tunnel = Arc::try_unwrap(tunnel).unwrap_or_else(|_| unreachable!("probe joined"));
    let cleanup = tokio::spawn(async move { tunnel.disconnect().await }).await;
    let cleanup_error = match cleanup {
        Ok(Ok(())) => None,
        Ok(Err(error)) => Some(error.to_string()),
        Err(error) => Some(format!("disconnect task stopped unexpectedly: {error}")),
    };
    let disconnect_confirmed = cleanup_error.is_none();
    let results: Vec<String> = shared
        .results
        .lock()
        .expect("scan results poisoned")
        .iter()
        .map(|result| format!("{} {:?}", result.address, result.outcome))
        .collect();
    let summary = serde_json::json!({
        "status": format!("{status:?}"), "results": results,
        "transport": actual_transport.as_str(),
        "skippedFrames": (actual_transport == knx_net::TunnelTransport::Tcp).then(||
            shared.lifecycle.lock().expect("scan lifecycle poisoned").skipped_frames).flatten(),
        "disconnectConfirmed": disconnect_confirmed, "cleanupError": cleanup_error,
    });
    let failure = match &status {
        LineScanStatus::Failed(reason) => Some(reason.clone()),
        _ => cleanup_error.clone(),
    };
    {
        let mut lifecycle = shared.lifecycle.lock().expect("scan lifecycle poisoned");
        lifecycle.disconnect_confirmed = Some(disconnect_confirmed);
        lifecycle.cleanup_error = cleanup_error;
    }
    // Running (and therefore exclusive holder status) includes consuming cleanup.
    *shared.status.lock().expect("scan status poisoned") = status;
    match failure {
        Some(reason) => crate::operation_recordings::finish_failed(&recorder, reason, summary),
        None => recorder.finish(knx_app::operation_recording::RecordedOutcome::succeeded(
            summary,
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn dropped_cancel_wait_retains_the_join_until_cleanup_settles() {
        let (done, wait) = oneshot::channel();
        let shared = Arc::new(SharedScan {
            status: Mutex::new(LineScanStatus::Running),
            results: Mutex::new(vec![]),
            lifecycle: Mutex::new(ScanLifecycle {
                tunnel: None,
                skipped_frames: Some(0),
                disconnect_confirmed: None,
                cleanup_error: None,
            }),
        });
        let publisher = shared.clone();
        let task = tokio::spawn(async move {
            let _ = wait.await;
            *publisher.status.lock().unwrap() = LineScanStatus::Cancelled;
        });
        let mut session = LineScanSession {
            id: 1,
            transport: knx_net::TunnelTransport::Tcp,
            range: ScannedRange {
                area: 1,
                line: 1,
                first_device: 5,
                last_device: 5,
            },
            total_count: 1,
            omitted: vec![],
            excluded: vec![],
            shared,
            cancel: None,
            task: Some(task),
        };
        let mut cancel = Box::pin(session.cancel());
        std::future::poll_fn(|cx| {
            assert!(std::future::Future::poll(cancel.as_mut(), cx).is_pending());
            std::task::Poll::Ready(())
        })
        .await;
        drop(cancel);
        assert!(
            session.task.is_some(),
            "dropping the waiter must not detach the tracked join"
        );
        done.send(()).unwrap();
        assert_eq!(session.cancel().await, LineScanStatus::Cancelled);
        assert!(session.task.is_none());
    }

    #[test]
    fn completion_between_status_and_results_keeps_snapshot_non_terminal() {
        let shared = Arc::new(SharedScan {
            status: Mutex::new(LineScanStatus::Running),
            results: Mutex::new(Vec::new()),
            lifecycle: Mutex::new(ScanLifecycle {
                tunnel: None,
                skipped_frames: Some(0),
                disconnect_confirmed: None,
                cleanup_error: None,
            }),
        });
        let publisher = Arc::clone(&shared);
        let session = LineScanSession {
            id: 1,
            transport: knx_net::TunnelTransport::Udp,
            range: ScannedRange {
                area: 2,
                line: 3,
                first_device: 4,
                last_device: 4,
            },
            total_count: 1,
            omitted: Vec::new(),
            excluded: Vec::new(),
            shared,
            cancel: None,
            task: None,
        };

        let (status, next_since, results) = session.snapshot_since_after_status(0, || {
            publisher
                .results
                .lock()
                .expect("scan results poisoned")
                .push(LineScanResult {
                    address: IndividualAddress::new(2, 3, 4).unwrap(),
                    outcome: ProbeOutcome::OccupiedBusy,
                });
            *publisher.status.lock().expect("scan status poisoned") = LineScanStatus::Completed;
        });

        assert_eq!(status, LineScanStatus::Running);
        assert_eq!(next_since, 1);
        assert_eq!(results.len(), 1);
    }
}
