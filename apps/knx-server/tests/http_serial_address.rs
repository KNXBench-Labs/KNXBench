//! K12: an individual address by serial number through the web API, in the simulator.
//!
//! The gateway is a [`SimTunnel`]: the production route and MP §2.4/§2.5
//! procedures, with the bus replaced by one simulated device that has a
//! serial number and no pressed button. No socket anywhere. Refusals are
//! checked to happen before the gateway is asked for a tunnel: the
//! connector counts its calls.

use std::future::Future;
use std::net::SocketAddrV4;
use std::pin::Pin;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use knx_core::IndividualAddress;
use knx_net::commissioning::simulator::{SimulatedDevice, SimulatorConfig};
use knx_net::{
    ApplicationService, BusError, Destination, DiscoveredGateway, ManagementTransport,
    SessionTiming, Tpci, TunnelEvent,
};
use knx_server::{BusSessionError, BusTunnel, GatewayConnector};
use serde_json::{json, Value};
use tokio::sync::broadcast;
use tower::ServiceExt;

const GATEWAY: &str = "192.0.2.10:3671";
const NEW: &str = "1.1.30";
const PHRASE: &str = "I confirm individual-address programming to 1.1.30";
const SERIAL: &str = "0083:12345678";
const SERIAL_OCTETS: [u8; 6] = [0x00, 0x83, 0x12, 0x34, 0x56, 0x78];

struct SimTunnel(Arc<SimulatedDevice>);

impl BusTunnel for SimTunnel {
    fn assigned_address(&self) -> IndividualAddress {
        ManagementTransport::assigned_address(self.0.as_ref())
    }

    fn subscribe(&self) -> broadcast::Receiver<TunnelEvent> {
        ManagementTransport::subscribe(self.0.as_ref())
    }

    fn send(
        &self,
        _destination: Destination,
        _service: ApplicationService,
    ) -> Pin<Box<dyn Future<Output = Result<(), BusSessionError>> + Send + '_>> {
        panic!("serial-number addressing sends no group telegram")
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
        Box::pin(async { Ok(()) })
    }
}

struct SimConnector {
    device: Arc<SimulatedDevice>,
    calls: Arc<AtomicUsize>,
}

impl GatewayConnector for SimConnector {
    fn connect_tunnel(
        &self,
        _gateway: SocketAddrV4,
    ) -> Pin<Box<dyn Future<Output = Result<Box<dyn BusTunnel>, BusSessionError>> + Send + '_>>
    {
        self.calls.fetch_add(1, Ordering::SeqCst);
        let tunnel: Box<dyn BusTunnel> = Box::new(SimTunnel(Arc::clone(&self.device)));
        Box::pin(async move { Ok(tunnel) })
    }

    fn discover(
        &self,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<DiscoveredGateway>, BusSessionError>> + Send + '_>>
    {
        Box::pin(async { Ok(Vec::new()) })
    }
}

fn fast() -> SessionTiming {
    SessionTiming {
        connection_timeout: Duration::from_millis(50),
        response_timeout: Duration::from_millis(50),
        poll_interval: Duration::from_millis(1),
        max_transition: Duration::from_millis(40),
        programming_delay: Duration::from_millis(0),
        restart_basic_t1: Duration::from_millis(1),
        restart_responsive_again: Duration::from_millis(5),
        post_restart_disconnect_wait: Duration::from_millis(60),
        programming_mode_broadcast_timeout: Duration::from_millis(20),
    }
}

struct Harness {
    _dir: tempfile::TempDir,
    app: axum::Router,
    device: Arc<SimulatedDevice>,
    calls: Arc<AtomicUsize>,
}

fn harness(config: SimulatorConfig) -> Harness {
    let dir = tempfile::tempdir().unwrap();
    let device = Arc::new(SimulatedDevice::with_config(config));
    let calls = Arc::new(AtomicUsize::new(0));
    let state = Arc::new(knx_server::AppState {
        data_dir: dir.path().to_path_buf(),
        connector: Box::new(SimConnector {
            device: Arc::clone(&device),
            calls: Arc::clone(&calls),
        }),
        address_programming_timing: fast(),
        address_programming_pause: Duration::from_millis(10),
        ..Default::default()
    });
    Harness {
        _dir: dir,
        app: knx_server::app(state, None),
        device,
        calls,
    }
}

fn post(uri: &str, body: Value) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri(uri)
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .unwrap()
}

fn get(uri: &str) -> Request<Body> {
    Request::builder().uri(uri).body(Body::empty()).unwrap()
}

async fn send(app: &axum::Router, request: Request<Body>) -> (StatusCode, Value) {
    let response = app.clone().oneshot(request).await.unwrap();
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body = serde_json::from_slice(&bytes)
        .unwrap_or_else(|_| Value::String(String::from_utf8_lossy(&bytes).into_owned()));
    (status, body)
}

fn with_serial() -> SimulatorConfig {
    SimulatorConfig {
        serial_number: Some(SERIAL_OCTETS),
        programming_mode: false,
        ..Default::default()
    }
}

fn write_request(confirmation: &str) -> Value {
    json!({
        "address": NEW,
        "gateway": GATEWAY,
        "confirmation": confirmation,
        "serialNumber": SERIAL,
    })
}

#[tokio::test]
async fn writes_by_serial_number_without_a_button() {
    let h = harness(with_serial());
    let before = h.device.address();
    let (status, body) = send(
        &h.app,
        post("/api/device-address/by-serial", write_request(PHRASE)),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["serialNumber"], SERIAL);
    assert_eq!(body["previousAddress"], before.to_string());
    assert_eq!(body["address"], NEW);
    assert_eq!(body["wrote"], true);
    assert_eq!(h.device.address().to_string(), NEW);
}

#[tokio::test]
async fn find_serial_answers_and_says_nobody_when_nobody_answers() {
    let h = harness(with_serial());
    let (status, body) = send(
        &h.app,
        get(&format!(
            "/api/device-address/find-serial?gateway={GATEWAY}&serialNumber={SERIAL}"
        )),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["address"], h.device.address().to_string());
    let (status, body) = send(
        &h.app,
        get(&format!(
            "/api/device-address/find-serial?gateway={GATEWAY}&serialNumber=0083:00000001"
        )),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["address"], Value::Null);
}

#[tokio::test]
async fn an_unknown_serial_number_is_not_found_and_writes_nothing() {
    let h = harness(with_serial());
    let before = h.device.address();
    let mut request = write_request(PHRASE);
    request["serialNumber"] = json!("0083:00000001");
    let (status, body) = send(&h.app, post("/api/device-address/by-serial", request)).await;
    assert_eq!(status, StatusCode::NOT_FOUND, "{body}");
    assert_eq!(h.device.address(), before);
    assert_eq!(h.device.serial_number_writes(), 0);
}

#[tokio::test]
async fn refusals_happen_before_any_tunnel_opens() {
    let h = harness(with_serial());
    let cases = [
        // Wrong phrase: another address, another scope.
        write_request("I confirm individual-address programming to 1.1.31"),
        write_request("I confirm restart to 1.1.30"),
        // No device named, or named twice.
        json!({"address": NEW, "gateway": GATEWAY, "confirmation": PHRASE}),
        json!({"address": NEW, "gateway": GATEWAY, "confirmation": PHRASE,
               "serialNumber": SERIAL, "device": "P-0001-0_DI-1"}),
        // A serial number one octet short is not padded.
        json!({"address": NEW, "gateway": GATEWAY, "confirmation": PHRASE,
               "serialNumber": "0083:123456"}),
        // The project records nothing for this device: no project is open.
        json!({"address": NEW, "gateway": GATEWAY, "confirmation": PHRASE,
               "device": "P-0001-0_DI-1"}),
        // Excluded or malformed addresses.
        json!({"address": knx_core::EXCLUDED_INDIVIDUAL_ADDRESSES[0].to_string(),
               "gateway": GATEWAY, "confirmation": PHRASE, "serialNumber": SERIAL}),
        json!({"address": "1.1.300", "gateway": GATEWAY, "confirmation": PHRASE,
               "serialNumber": SERIAL}),
        json!({"address": NEW, "gateway": "nowhere", "confirmation": PHRASE,
               "serialNumber": SERIAL}),
    ];
    for case in cases {
        let (status, body) =
            send(&h.app, post("/api/device-address/by-serial", case.clone())).await;
        assert!(status.is_client_error(), "{case} -> {status} {body}");
    }
    assert_eq!(h.calls.load(Ordering::SeqCst), 0, "no tunnel for a refusal");
    assert_eq!(h.device.serial_number_writes(), 0);
}
