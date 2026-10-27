//! Verifies that the service-control file-backup boundary leaves tunnel tasks running.

use super::*;
use crate::bus::{BusSessionError, BusTunnel, GatewayConnector};
use knx_net::commissioning::simulator::{SimulatedDevice, SimulatorConfig};
use knx_net::{
    ApplicationService, BusError, Destination, DiscoveredGateway, ManagementTransport, Tpci,
};
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

struct SimConnector(Arc<SimulatedDevice>, Arc<AtomicBool>);
struct SimTunnel(Arc<SimulatedDevice>, Arc<AtomicBool>);

impl BusTunnel for SimTunnel {
    fn assigned_address(&self) -> IndividualAddress {
        ManagementTransport::assigned_address(self.0.as_ref())
    }

    fn subscribe(&self) -> tokio::sync::broadcast::Receiver<knx_net::TunnelEvent> {
        ManagementTransport::subscribe(self.0.as_ref())
    }

    fn send(
        &self,
        _: Destination,
        _: ApplicationService,
    ) -> Pin<Box<dyn Future<Output = Result<(), BusSessionError>> + Send + '_>> {
        panic!("no group telegram belongs in a service-control write")
    }

    fn send_frame(
        &self,
        destination: Destination,
        transport: Tpci,
        service: ApplicationService,
    ) -> Pin<Box<dyn Future<Output = Result<(), BusError>> + Send + '_>> {
        Box::pin(async move {
            ManagementTransport::send_frame(self.0.as_ref(), destination, transport, service).await
        })
    }

    fn disconnect(
        self: Box<Self>,
    ) -> Pin<Box<dyn Future<Output = Result<(), BusSessionError>> + Send>> {
        Box::pin(async move {
            self.1.store(true, Ordering::SeqCst);
            Ok(())
        })
    }
}

impl GatewayConnector for SimConnector {
    fn connect_tunnel(
        &self,
        _: SocketAddrV4,
    ) -> Pin<Box<dyn Future<Output = Result<Box<dyn BusTunnel>, BusSessionError>> + Send + '_>>
    {
        Box::pin(async move {
            Ok(Box::new(SimTunnel(Arc::clone(&self.0), Arc::clone(&self.1))) as Box<dyn BusTunnel>)
        })
    }

    fn discover(
        &self,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<DiscoveredGateway>, BusSessionError>> + Send + '_>>
    {
        Box::pin(async { Ok(vec![]) })
    }
}

/// The HTTP test locks only the journal row: this one stalls at the actual file
/// backup boundary. Keep it in the same blocking section as `write_backup`, so
/// wrapping only the marker cannot leave the file write inline unnoticed.
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn a_blocking_backup_file_leaves_tunnel_tasks_running() {
    let dir = tempfile::tempdir().unwrap();
    let device = Arc::new(SimulatedDevice::with_config(SimulatorConfig {
        serial_number_write_enabled: false,
        ..Default::default()
    }));
    let disconnected = Arc::new(AtomicBool::new(false));
    let address = device.address();
    let state = Arc::new(crate::AppState {
        connector: Box::new(SimConnector(device, Arc::clone(&disconnected))),
        ..crate::AppState::new(dir.path().to_path_buf())
    });
    // Enable through the real settings route; the private test hook is not JSON.
    use axum::body::Body;
    use axum::http::Request;
    use tower::ServiceExt;
    let response = crate::app(Arc::clone(&state), None)
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/api/settings")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"settings":{"debugIndividualAddressWriteEnable":true}}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let ticks = Arc::new(AtomicUsize::new(0));
    let window = Arc::new(Mutex::new(None));
    let ticker = tokio::spawn({
        let ticks = Arc::clone(&ticks);
        async move {
            loop {
                tokio::time::sleep(Duration::from_millis(5)).await;
                ticks.fetch_add(1, Ordering::SeqCst);
            }
        }
    });
    let request = WriteRequest {
        address: address.to_string(),
        gateway: "203.0.113.250:3671".into(),
        confirmation: format!("I confirm individual-address write enable to {address}"),
        enable: true,
        before_backup: Some(Box::new({
            let (ticks, window) = (Arc::clone(&ticks), Arc::clone(&window));
            move || {
                assert!(!disconnected.load(Ordering::SeqCst));
                let before = ticks.load(Ordering::SeqCst);
                std::thread::sleep(Duration::from_millis(1500));
                *window.lock().unwrap() = Some((before, ticks.load(Ordering::SeqCst)));
            }
        })),
    };
    // Spawn the actual handler on the one runtime worker, not the test thread.
    let result = tokio::time::timeout(
        Duration::from_secs(30),
        tokio::spawn(write(State(state), Json(request))),
    )
    .await
    .expect("the handler ended")
    .unwrap();
    ticker.abort();
    let Json(result) = match result {
        Ok(result) => result,
        Err(_) => panic!("the simulated service-control write failed"),
    };
    assert!(result.written);
    let (before, after) = window
        .lock()
        .unwrap()
        .expect("the file backup boundary was reached");
    assert!(
        after - before >= 10,
        "runtime tasks starved during the file backup: {before} -> {after}"
    );
    let backup = std::fs::read(result.backup_path.expect("the backup file exists")).unwrap();
    let _: knx_app::service_control_backup::ServiceControlBackup =
        serde_json::from_slice(&backup).unwrap();
}
