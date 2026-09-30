//! K12 follow-up: `PID_SERVICE_CONTROL` bit 2 through the web API (ADR-0051).
//!
//! The gateway is a [`SimTunnel`] over one simulated device; no socket. The
//! routes are off unless the settings file says
//! `debugIndividualAddressWriteEnable: true`, and every refusal is checked
//! to happen before a tunnel is asked for: the connector counts its calls.

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
use tokio::sync::{broadcast, Notify};
use tower::ServiceExt;

const GATEWAY: &str = "192.0.2.10:3671";
const NEW: &str = "1.1.30";
const SERIAL: &str = "0083:12345678";
const SERIAL_OCTETS: [u8; 6] = [0x00, 0x83, 0x12, 0x34, 0x56, 0x78];

enum WriteTrap {
    Fail,
    SendThenFail,
    Block { entered: Notify, resume: Notify },
}

struct SimTunnel(Arc<SimulatedDevice>, Option<Arc<WriteTrap>>);

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
        panic!("service control sends no group telegram")
    }

    fn send_frame(
        &self,
        destination: Destination,
        transport: Tpci,
        service: ApplicationService,
    ) -> Pin<Box<dyn Future<Output = Result<(), BusError>> + Send + '_>> {
        Box::pin(async move {
            if matches!(
                service,
                ApplicationService::PropertyValueWrite { property_id: 8, .. }
            ) {
                match self.1.as_deref() {
                    Some(WriteTrap::Fail) => return Err(BusError::Timeout),
                    Some(WriteTrap::SendThenFail) => {
                        ManagementTransport::send_frame(
                            self.0.as_ref(),
                            destination,
                            transport,
                            service,
                        )
                        .await?;
                        return Err(BusError::Timeout);
                    }
                    Some(WriteTrap::Block { entered, resume }) => {
                        entered.notify_one();
                        resume.notified().await;
                    }
                    None => {}
                }
            }
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
    connect_delay: Duration,
    write_trap: Option<Arc<WriteTrap>>,
}

impl GatewayConnector for SimConnector {
    fn connect_tunnel(
        &self,
        _gateway: SocketAddrV4,
    ) -> Pin<Box<dyn Future<Output = Result<Box<dyn BusTunnel>, BusSessionError>> + Send + '_>>
    {
        self.calls.fetch_add(1, Ordering::SeqCst);
        let tunnel: Box<dyn BusTunnel> =
            Box::new(SimTunnel(Arc::clone(&self.device), self.write_trap.clone()));
        let delay = self.connect_delay;
        Box::pin(async move {
            tokio::time::sleep(delay).await;
            Ok(tunnel)
        })
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
    harness_with_delay(config, Duration::ZERO)
}

fn harness_with_delay(config: SimulatorConfig, connect_delay: Duration) -> Harness {
    harness_with_trap(config, connect_delay, None)
}

fn harness_with_trap(
    config: SimulatorConfig,
    connect_delay: Duration,
    write_trap: Option<Arc<WriteTrap>>,
) -> Harness {
    let dir = tempfile::tempdir().unwrap();
    let device = Arc::new(SimulatedDevice::with_config(config));
    let calls = Arc::new(AtomicUsize::new(0));
    let state = Arc::new(knx_server::AppState {
        data_dir: dir.path().to_path_buf(),
        connector: Box::new(SimConnector {
            device: Arc::clone(&device),
            calls: Arc::clone(&calls),
            connect_delay,
            write_trap,
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

fn locked_device() -> SimulatorConfig {
    SimulatorConfig {
        serial_number: Some(SERIAL_OCTETS),
        serial_number_write_enabled: false,
        programming_mode: false,
        ..Default::default()
    }
}

fn phrase(address: IndividualAddress) -> String {
    format!("I confirm individual-address write enable to {address}")
}

async fn enable_debug(h: &Harness, value: Value) {
    let request = Request::builder()
        .method("PUT")
        .uri("/api/settings")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({ "settings": { "debugIndividualAddressWriteEnable": value } }).to_string(),
        ))
        .unwrap();
    let (status, body) = send(&h.app, request).await;
    assert_eq!(status, StatusCode::OK, "{body}");
}

fn write_request(address: IndividualAddress, confirmation: &str, enable: bool) -> Value {
    json!({
        "address": address.to_string(),
        "gateway": GATEWAY,
        "confirmation": confirmation,
        "enable": enable,
    })
}

fn read_uri(address: IndividualAddress) -> String {
    format!("/api/device/service-control?address={address}&gateway={GATEWAY}")
}

#[tokio::test]
async fn off_by_default_both_routes_refuse_before_a_tunnel() {
    let h = harness(locked_device());
    let address = h.device.address();
    let (status, body) = send(&h.app, get(&read_uri(address))).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{body}");
    let (status, body) = send(
        &h.app,
        post(
            "/api/device/service-control",
            write_request(address, &phrase(address), true),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{body}");
    assert!(body.to_string().contains("Settings"), "{body}");
    assert_eq!(h.calls.load(Ordering::SeqCst), 0);
    let (_, activity) = send(&h.app, get("/api/bus/activity")).await;
    assert_eq!(activity["oneShot"], json!([]));
}

#[tokio::test]
async fn a_value_other_than_true_keeps_it_off() {
    let h = harness(locked_device());
    enable_debug(&h, json!("true")).await;
    let address = h.device.address();
    let (status, _) = send(&h.app, get(&read_uri(address))).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(h.calls.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn a_wrong_phrase_is_refused_before_a_tunnel() {
    let h = harness(locked_device());
    enable_debug(&h, json!(true)).await;
    let address = h.device.address();
    let download_phrase = format!("I confirm download to {address}");
    let (status, body) = send(
        &h.app,
        post(
            "/api/device/service-control",
            write_request(address, &download_phrase, true),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
    assert_eq!(h.calls.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn cancelled_debug_read_is_unknown_and_never_reports_property_bytes() {
    let h = harness_with_delay(locked_device(), Duration::from_secs(30));
    enable_debug(&h, json!(true)).await;
    let address = h.device.address();
    let app = h.app.clone();
    let task = tokio::spawn(async move { send(&app, get(&read_uri(address))).await });
    tokio::time::timeout(Duration::from_secs(1), async {
        while h.calls.load(Ordering::SeqCst) == 0 {
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("the simulated tunnel was not requested");
    let (_, activity) = send(&h.app, get("/api/bus/activity")).await;
    assert_eq!(activity["oneShot"][0]["state"], "running");
    assert_eq!(activity["oneShot"][0]["address"], address.to_string());

    task.abort();
    assert!(task.await.unwrap_err().is_cancelled());
    let (_, activity) = send(&h.app, get("/api/bus/activity")).await;
    assert_eq!(activity["oneShot"][0]["state"], "unknown");
    assert!(activity["oneShot"][0]["finishedAt"].is_string());
    assert!(activity["oneShot"][0].get("raw").is_none());
    assert!(activity["oneShot"][0].get("mask").is_none());
    assert!(h.device.seen().iter().all(|seen| !matches!(
        seen,
        knx_net::commissioning::simulator::Seen::PropertyWrite { property_id: 8, .. }
    )));
}

#[tokio::test]
async fn enabled_it_reads_sets_bit_2_but_serial_write_still_requires_recovery() {
    let h = harness(locked_device());
    enable_debug(&h, json!(true)).await;
    let address = h.device.address();

    let (status, body) = send(&h.app, get(&read_uri(address))).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["individualAddressWriteEnabled"], false);
    assert_eq!(body["raw"], "0000");
    let (_, activity) = send(&h.app, get("/api/bus/activity")).await;
    assert_eq!(activity["oneShot"][0]["kind"], "serviceControlRead");
    assert_eq!(activity["oneShot"][0]["address"], address.to_string());
    assert_eq!(activity["oneShot"][0]["state"], "finished");
    assert!(activity["oneShot"][0].get("raw").is_none());
    assert!(activity["oneShot"][0].get("mask").is_none());

    // The server refuses serial-address writes without a durable pre-write
    // recovery record, independently of bit 2's current value.
    let serial_phrase = format!("I confirm individual-address programming to {NEW}");
    let by_serial = json!({
        "address": NEW,
        "gateway": GATEWAY,
        "confirmation": serial_phrase,
        "serialNumber": SERIAL,
    });
    let (status, body) = send(
        &h.app,
        post("/api/device-address/by-serial", by_serial.clone()),
    )
    .await;
    assert_eq!(status, StatusCode::PRECONDITION_FAILED, "{body}");
    assert_eq!(h.device.serial_number_writes(), 0);

    let (status, body) = send(
        &h.app,
        post(
            "/api/device/service-control",
            write_request(address, &phrase(address), true),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["written"], true);
    let (_, activity) = send(&h.app, get("/api/bus/activity")).await;
    assert_eq!(activity["oneShot"].as_array().unwrap().len(), 2);
    assert_eq!(activity["oneShot"][0]["kind"], "serviceControlRead");
    assert_eq!(activity["oneShot"][1]["kind"], "serviceControlWrite");
    assert_eq!(activity["oneShot"][1]["state"], "verified");
    assert_eq!(
        activity["oneShot"][1]["writeEvidence"]["backupRecorded"],
        true
    );
    assert_eq!(
        activity["oneShot"][1]["writeEvidence"]["sendPossible"],
        true
    );
    assert_eq!(body["before"]["raw"], "0000");
    assert_eq!(body["after"], "0004");
    let path = body["backupPath"].as_str().expect("durable backup path");
    let record: knx_app::service_control_backup::ServiceControlBackup =
        serde_json::from_slice(&std::fs::read(path).expect("backed-up property")).unwrap();
    assert_eq!(record.device, address.to_string());
    assert_eq!(record.mask, body["before"]["mask"]);
    assert_eq!(record.octets, "0000");
    assert_eq!(body["individualAddressWriteEnabled"], true);

    let tunnels_before = h.calls.load(Ordering::SeqCst);
    let (status, body) = send(&h.app, post("/api/device-address/by-serial", by_serial)).await;
    assert_eq!(status, StatusCode::PRECONDITION_FAILED, "{body}");
    assert_eq!(h.calls.load(Ordering::SeqCst), tunnels_before);
    assert_eq!(h.device.address(), address);
    assert_eq!(h.device.serial_number_writes(), 0);
}

#[tokio::test]
async fn completed_property_change_and_noop_are_not_generic_write_receipts() {
    let h = harness(locked_device());
    enable_debug(&h, json!(true)).await;
    let address = h.device.address();
    let request = write_request(address, &phrase(address), true);
    let (status, body) = send(&h.app, post("/api/device/service-control", request.clone())).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["written"], true);
    assert!(body["backupPath"].is_string());
    let (status, body) = send(&h.app, post("/api/device/service-control", request)).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["written"], false);
    assert_eq!(body["backupPath"], Value::Null);

    let (_, activity) = send(&h.app, get("/api/bus/activity")).await;
    let entries = activity["oneShot"].as_array().unwrap();
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0]["kind"], "serviceControlWrite");
    assert_eq!(entries[0]["state"], "verified");
    assert_eq!(entries[0]["writeEvidence"]["backupRecorded"], true);
    assert_eq!(entries[0]["writeEvidence"]["sendPossible"], true);
    assert_eq!(entries[1]["kind"], "serviceControlWrite");
    assert_eq!(entries[1]["state"], "noChange");
    assert_eq!(entries[1]["writeEvidence"]["backupRecorded"], false);
    assert_eq!(entries[1]["writeEvidence"]["sendPossible"], false);
    assert!(!activity["untracked"]
        .as_array()
        .unwrap()
        .contains(&json!("serviceControlWrite")));
    let writes = h
        .device
        .seen()
        .iter()
        .filter(|seen| {
            matches!(
                seen,
                knx_net::commissioning::simulator::Seen::PropertyWrite { property_id: 8, .. }
            )
        })
        .count();
    assert_eq!(writes, 1);
}

#[tokio::test]
async fn a_failed_property_backup_refuses_before_the_write() {
    let h = harness(locked_device());
    enable_debug(&h, json!(true)).await;
    // No permissions tricks: a regular file in place of the backup directory
    // reliably fails even when tests happen to run as root.
    std::fs::write(h._dir.path().join("device-backups"), b"occupied").unwrap();
    let address = h.device.address();
    let (status, body) = send(
        &h.app,
        post(
            "/api/device/service-control",
            write_request(address, &phrase(address), true),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::INSUFFICIENT_STORAGE, "{body}");
    assert!(body.to_string().contains("backup"), "{body}");
    assert!(h.device.seen().iter().all(|seen| !matches!(
        seen,
        knx_net::commissioning::simulator::Seen::PropertyWrite { property_id: 8, .. }
    )));
    assert_eq!(h.device.serial_number_writes(), 0);
    let (_, activity) = send(&h.app, get("/api/bus/activity")).await;
    assert_eq!(activity["oneShot"][0]["kind"], "serviceControlWrite");
    assert_eq!(activity["oneShot"][0]["state"], "notSent");
    assert_eq!(
        activity["oneShot"][0]["writeEvidence"]["backupRecorded"],
        false
    );
    assert_eq!(
        activity["oneShot"][0]["writeEvidence"]["sendPossible"],
        false
    );
}

#[tokio::test]
async fn cancelled_service_control_write_during_connect_is_unknown_and_never_verified() {
    let h = harness_with_delay(locked_device(), Duration::from_secs(30));
    enable_debug(&h, json!(true)).await;
    let address = h.device.address();
    let app = h.app.clone();
    let task = tokio::spawn(async move {
        send(
            &app,
            post(
                "/api/device/service-control",
                write_request(address, &phrase(address), true),
            ),
        )
        .await
    });
    tokio::time::timeout(Duration::from_secs(1), async {
        while h.calls.load(Ordering::SeqCst) == 0 {
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("the simulated tunnel was not requested");
    let (_, activity) = send(&h.app, get("/api/bus/activity")).await;
    assert_eq!(activity["oneShot"][0]["state"], "running");
    task.abort();
    assert!(task.await.unwrap_err().is_cancelled());
    let (_, activity) = send(&h.app, get("/api/bus/activity")).await;
    assert_eq!(activity["oneShot"][0]["state"], "unknown");
    assert_eq!(
        activity["oneShot"][0]["writeEvidence"]["backupRecorded"],
        false
    );
    assert_eq!(
        activity["oneShot"][0]["writeEvidence"]["sendPossible"],
        false
    );
    assert!(h.device.seen().iter().all(|seen| !matches!(
        seen,
        knx_net::commissioning::simulator::Seen::PropertyWrite { property_id: 8, .. }
    )));
}

#[tokio::test]
async fn a_transport_failure_after_the_backup_is_not_a_verified_write() {
    let h = harness_with_trap(
        locked_device(),
        Duration::ZERO,
        Some(Arc::new(WriteTrap::Fail)),
    );
    enable_debug(&h, json!(true)).await;
    let address = h.device.address();
    let (status, body) = send(
        &h.app,
        post(
            "/api/device/service-control",
            write_request(address, &phrase(address), true),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_GATEWAY, "{body}");
    let (_, activity) = send(&h.app, get("/api/bus/activity")).await;
    assert_eq!(activity["oneShot"][0]["state"], "effectUnverified");
    assert_eq!(
        activity["oneShot"][0]["writeEvidence"]["backupRecorded"],
        true
    );
    assert_eq!(
        activity["oneShot"][0]["writeEvidence"]["sendPossible"],
        true
    );
    assert!(h.device.seen().iter().all(|seen| !matches!(
        seen,
        knx_net::commissioning::simulator::Seen::PropertyWrite { property_id: 8, .. }
    )));
}

#[tokio::test]
async fn a_property_changed_before_transport_failure_stays_effect_unverified() {
    let h = harness_with_trap(
        locked_device(),
        Duration::ZERO,
        Some(Arc::new(WriteTrap::SendThenFail)),
    );
    enable_debug(&h, json!(true)).await;
    let address = h.device.address();
    let (status, body) = send(
        &h.app,
        post(
            "/api/device/service-control",
            write_request(address, &phrase(address), true),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_GATEWAY, "{body}");
    assert!(h.device.seen().iter().any(|seen| matches!(
        seen,
        knx_net::commissioning::simulator::Seen::PropertyWrite { property_id: 8, .. }
    )));
    let (_, activity) = send(&h.app, get("/api/bus/activity")).await;
    assert_eq!(activity["oneShot"][0]["state"], "effectUnverified");
    assert_eq!(
        activity["oneShot"][0]["writeEvidence"]["backupRecorded"],
        true
    );
    assert_eq!(
        activity["oneShot"][0]["writeEvidence"]["sendPossible"],
        true
    );
}

#[tokio::test]
async fn cancellation_after_the_backup_retains_unknown_possible_write_evidence() {
    let trap = Arc::new(WriteTrap::Block {
        entered: Notify::new(),
        resume: Notify::new(),
    });
    let h = harness_with_trap(locked_device(), Duration::ZERO, Some(Arc::clone(&trap)));
    enable_debug(&h, json!(true)).await;
    let address = h.device.address();
    let app = h.app.clone();
    let task = tokio::spawn(async move {
        send(
            &app,
            post(
                "/api/device/service-control",
                write_request(address, &phrase(address), true),
            ),
        )
        .await
    });
    let WriteTrap::Block { entered, .. } = trap.as_ref() else {
        unreachable!()
    };
    tokio::time::timeout(Duration::from_secs(1), entered.notified())
        .await
        .expect("the write send boundary was not reached");
    let backups = std::fs::read_dir(h._dir.path().join("device-backups"))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert_eq!(backups.len(), 1);
    let (_, activity) = send(&h.app, get("/api/bus/activity")).await;
    assert_eq!(activity["oneShot"][0]["state"], "running");
    assert_eq!(
        activity["oneShot"][0]["writeEvidence"]["backupRecorded"],
        true
    );
    assert_eq!(
        activity["oneShot"][0]["writeEvidence"]["sendPossible"],
        true
    );
    task.abort();
    assert!(task.await.unwrap_err().is_cancelled());
    let (_, activity) = send(&h.app, get("/api/bus/activity")).await;
    assert_eq!(activity["oneShot"][0]["state"], "unknown");
    assert_eq!(
        activity["oneShot"][0]["writeEvidence"]["sendPossible"],
        true
    );
    assert!(h.device.seen().iter().all(|seen| !matches!(
        seen,
        knx_net::commissioning::simulator::Seen::PropertyWrite { property_id: 8, .. }
    )));
}

#[tokio::test]
async fn a_device_without_the_property_is_named_not_guessed() {
    let h = harness(SimulatorConfig {
        service_control_present: false,
        ..locked_device()
    });
    enable_debug(&h, json!(true)).await;
    let address = h.device.address();
    let (status, _) = send(&h.app, get(&read_uri(address))).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    let (_, activity) = send(&h.app, get("/api/bus/activity")).await;
    assert_eq!(activity["oneShot"][0]["state"], "failed");
    let (status, body) = send(
        &h.app,
        post(
            "/api/device/service-control",
            write_request(address, &phrase(address), true),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{body}");
    assert!(body.to_string().contains("PID_SERVICE_CONTROL"), "{body}");
    let (_, activity) = send(&h.app, get("/api/bus/activity")).await;
    assert_eq!(activity["oneShot"].as_array().unwrap().len(), 2);
    assert_eq!(activity["oneShot"][1]["kind"], "serviceControlWrite");
    assert_eq!(activity["oneShot"][1]["state"], "notSent");
    assert_eq!(
        activity["oneShot"][1]["writeEvidence"]["sendPossible"],
        false
    );
}
