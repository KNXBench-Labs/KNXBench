//! DIAG-03: a real tunnel to a loopback simulator gateway is recorded byte for byte.
//!
//! The production `RealConnector` opens a KNXnet/IP tunnel to
//! `knx_net::commissioning::simulator_gateway` on 127.0.0.1. The gateway
//! reports every datagram it received and sent; the operation recording the
//! route leaves in `data_dir/diagnostics` must hold exactly those datagrams,
//! per direction and in order, and name the server's `GET /api/version`.
//! No real gateway, bus or device is involved.

use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use knx_core::IndividualAddress;
use knx_net::commissioning::simulator::{SimulatedDevice, SimulatorConfig};
use knx_net::commissioning::simulator_gateway::{serve, GatewayConfig, GatewayEvent};
use serde_json::Value;
use tokio::net::UdpSocket;
use tower::ServiceExt;

const SERIAL: &str = "0083:12345678";
const SERIAL_OCTETS: [u8; 6] = [0x00, 0x83, 0x12, 0x34, 0x56, 0x78];

type Seen = Arc<Mutex<Vec<(bool, Vec<u8>)>>>;

/// A simulated device behind a loopback gateway; returns its endpoint and log.
async fn loopback_gateway() -> (SocketAddrV4, Seen, Arc<SimulatedDevice>) {
    let socket = UdpSocket::bind((Ipv4Addr::LOCALHOST, 0)).await.unwrap();
    let SocketAddr::V4(endpoint) = socket.local_addr().unwrap() else {
        unreachable!("bound IPv4")
    };
    let device = Arc::new(SimulatedDevice::with_config(SimulatorConfig {
        serial_number: Some(SERIAL_OCTETS),
        programming_mode: false,
        ..Default::default()
    }));
    let seen: Seen = Arc::default();
    let log = Arc::clone(&seen);
    let mut config = GatewayConfig::new(endpoint, IndividualAddress::from_raw(0x11fe));
    config.observer = Some(Arc::new(move |event: GatewayEvent<'_>| {
        if let GatewayEvent::Datagram { from_tool, bytes } = event {
            log.lock().unwrap().push((from_tool, bytes.to_vec()));
        }
    }));
    tokio::spawn(serve(Arc::clone(&device), socket, config));
    (endpoint, seen, device)
}

async fn get(app: &axum::Router, uri: &str) -> (StatusCode, Value) {
    let response = app
        .clone()
        .oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body = serde_json::from_slice(&bytes)
        .unwrap_or_else(|_| Value::String(String::from_utf8_lossy(&bytes).into_owned()));
    (status, body)
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_serial_lookup_over_a_real_tunnel_is_recorded_byte_exactly() {
    let dir = tempfile::tempdir().unwrap();
    let (endpoint, seen, device) = loopback_gateway().await;
    let state = Arc::new(knx_server::AppState {
        recordings: Arc::new(knx_server::operation_recordings::store(dir.path(), false)),
        data_dir: dir.path().to_path_buf(),
        connector: Box::new(knx_server::RealConnector::default()),
        ..Default::default()
    });
    let app = knx_server::app(Arc::clone(&state), None);

    let (status, body) = get(
        &app,
        &format!("/api/device-address/find-serial?gateway={endpoint}&serialNumber={SERIAL}"),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["address"], device.address().to_string());
    let (_, version) = get(&app, "/api/version").await;

    assert!(state.recordings.wait_until_idle(Duration::from_secs(10)));
    // Datagrams that arrive after the route returned are not the operation's.
    let wire = seen.lock().unwrap().clone();
    let recordings = state.recordings.list().unwrap();
    assert_eq!(recordings.len(), 1, "{recordings:?}");
    let summary = &recordings[0];
    assert_eq!(summary.outcome.as_deref(), Some("succeeded"));
    assert_eq!(
        summary.operation_kind.as_deref(),
        Some("serial-number-address")
    );
    assert_eq!(
        summary.writer_version.as_ref(),
        version["version"].as_str().map(str::to_string).as_ref()
    );

    let text = String::from_utf8(state.recordings.read(&summary.id).unwrap()).unwrap();
    let lines: Vec<Value> = text
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert_eq!(lines[0]["writer"]["version"], version["version"]);
    assert_eq!(lines[0]["gateway"], endpoint.to_string());
    assert_eq!(
        lines[0]["related"]["serverIncarnation"],
        state.server_incarnation
    );
    let recorded = |dir: &str| -> Vec<String> {
        lines
            .iter()
            .filter(|l| l["type"] == "datagram" && l["dir"] == dir)
            .map(|l| l["raw"].as_str().unwrap().to_string())
            .collect()
    };
    let on_wire = |from_tool: bool| -> Vec<String> {
        wire.iter()
            .filter(|(from, _)| *from == from_tool)
            .map(|(_, bytes)| hex(bytes))
            .collect()
    };
    assert!(!recorded("sent").is_empty());
    assert_eq!(recorded("sent"), on_wire(true), "tool → gateway");
    assert_eq!(recorded("received"), on_wire(false), "gateway → tool");
    let end = lines.last().unwrap();
    assert_eq!(end["type"], "end");
    assert_eq!(end["records_dropped"], 0);
    assert_eq!(end["summary"]["address"], device.address().to_string());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_refused_connection_still_leaves_a_failed_recording() {
    let dir = tempfile::tempdir().unwrap();
    // A bound socket that never answers: the connect handshake times out.
    let silent = UdpSocket::bind((Ipv4Addr::LOCALHOST, 0)).await.unwrap();
    let SocketAddr::V4(endpoint) = silent.local_addr().unwrap() else {
        unreachable!("bound IPv4")
    };
    let state = Arc::new(knx_server::AppState {
        recordings: Arc::new(knx_server::operation_recordings::store(dir.path(), false)),
        data_dir: dir.path().to_path_buf(),
        connector: Box::new(knx_server::RealConnector::default()),
        ..Default::default()
    });
    let app = knx_server::app(Arc::clone(&state), None);
    let (status, _) = get(
        &app,
        &format!("/api/device-address/find-serial?gateway={endpoint}&serialNumber={SERIAL}"),
    )
    .await;
    assert_ne!(status, StatusCode::OK);
    assert!(state.recordings.wait_until_idle(Duration::from_secs(10)));
    let recordings = state.recordings.list().unwrap();
    assert_eq!(recordings.len(), 1);
    assert_eq!(recordings[0].outcome.as_deref(), Some("failed"));
    let text = String::from_utf8(state.recordings.read(&recordings[0].id).unwrap()).unwrap();
    assert!(text.contains("\"event\":\"connect-failed\""), "{text}");
    assert!(text.contains("CONNECT_REQUEST"), "{text}");
    drop(silent);
}

#[tokio::test]
async fn the_test_default_state_records_nothing() {
    let state = knx_server::AppState::default();
    assert!(!state.recordings.is_enabled());
}
