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

#[derive(Debug)]
struct SharedScan {
    status: Mutex<LineScanStatus>,
    results: Mutex<Vec<LineScanResult>>,
}

pub struct LineScanSession {
    id: u64,
    range: ScannedRange,
    total_count: usize,
    omitted: Vec<IndividualAddress>,
    excluded: Vec<IndividualAddress>,
    shared: Arc<SharedScan>,
    cancel: Option<oneshot::Sender<()>>,
    task: Option<JoinHandle<()>>,
}

impl LineScanSession {
    pub async fn start(
        id: u64,
        range: ScannedRange,
        gateway: SocketAddrV4,
        connector: &dyn GatewayConnector,
        plan: ScanPlan,
        policy: ProbePolicy,
    ) -> Result<Self, BusSessionError> {
        let tunnel = connector.connect_tunnel(gateway).await?;
        let total_count = plan.addresses().len();
        let omitted = plan.omitted().to_vec();
        let mut excluded: Vec<_> = plan.excluded().iter().copied().collect();
        excluded.sort_unstable_by_key(|address| address.raw());
        let shared = Arc::new(SharedScan {
            status: Mutex::new(LineScanStatus::Running),
            results: Mutex::new(Vec::with_capacity(total_count)),
        });
        let task_shared = Arc::clone(&shared);
        let (cancel_tx, cancel_rx) = oneshot::channel();
        let task = tokio::spawn(async move {
            run_scan(tunnel, plan, policy, cancel_rx, task_shared).await;
        });
        Ok(Self {
            id,
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

    pub async fn cancel(&mut self) -> LineScanStatus {
        if let Some(cancel) = self.cancel.take() {
            let _ = cancel.send(());
        }
        if let Some(task) = self.task.take() {
            if task.await.is_err() {
                *self.shared.status.lock().expect("scan status poisoned") =
                    LineScanStatus::Failed("scan task stopped unexpectedly".to_string());
            }
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
    tunnel: Box<dyn BusTunnel>,
    plan: ScanPlan,
    policy: ProbePolicy,
    cancel: oneshot::Receiver<()>,
    shared: Arc<SharedScan>,
) {
    let result = {
        let transport = TunnelScanTransport(tunnel.as_ref());
        let progress_shared = Arc::clone(&shared);
        let scan = scan_line(&transport, &plan, &policy, move |address, outcome| {
            progress_shared
                .results
                .lock()
                .expect("scan results poisoned")
                .push(LineScanResult { address, outcome });
        });
        tokio::pin!(scan);
        tokio::select! {
            _ = cancel => None,
            result = &mut scan => Some(result),
        }
    };

    let status = match result {
        None => LineScanStatus::Cancelled,
        Some(Ok(_)) => LineScanStatus::Completed,
        Some(Err(ScanError::Plan(error))) => LineScanStatus::Failed(error.to_string()),
        Some(Err(ScanError::Transport { source, .. })) => {
            LineScanStatus::Failed(source.to_string())
        }
    };
    *shared.status.lock().expect("scan status poisoned") = status;
    let _ = tunnel.disconnect().await;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn completion_between_status_and_results_keeps_snapshot_non_terminal() {
        let shared = Arc::new(SharedScan {
            status: Mutex::new(LineScanStatus::Running),
            results: Mutex::new(Vec::new()),
        });
        let publisher = Arc::clone(&shared);
        let session = LineScanSession {
            id: 1,
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
