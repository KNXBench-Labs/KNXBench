//! Unknown targets, privacy boundaries and diagnostic admission through real HTTP handlers.
use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
};
use knx_core::{DeviceId, DeviceInstance, IndividualAddress, Language, Project, SourceRef};
use knx_net::commissioning::simulator::{Seen, SimulatedDevice, SimulatorConfig};
use knx_net::{
    ApplicationService, BusError, Destination, DiscoveredGateway, ManagementTransport, Tpci,
    TunnelEvent,
};
use knx_server::{BusSessionError, BusTunnel, GatewayConnector};
use serde_json::{json, Value};
use std::{
    future::Future,
    net::SocketAddrV4,
    pin::Pin,
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc,
    },
};
use tokio::sync::broadcast;
use tower::ServiceExt;
struct Tunnel(Arc<SimulatedDevice>);
impl BusTunnel for Tunnel {
    fn diagnostics_are_simulated(&self) -> bool {
        true
    }
    fn assigned_address(&self) -> IndividualAddress {
        self.0.assigned_address()
    }
    fn subscribe(&self) -> broadcast::Receiver<TunnelEvent> {
        self.0.subscribe()
    }
    fn send(
        &self,
        _: Destination,
        _: ApplicationService,
    ) -> Pin<Box<dyn Future<Output = Result<(), BusSessionError>> + Send + '_>> {
        panic!("no group telegrams")
    }
    fn send_frame(
        &self,
        d: Destination,
        t: Tpci,
        s: ApplicationService,
    ) -> Pin<Box<dyn Future<Output = Result<(), BusError>> + Send + '_>> {
        Box::pin(async move { self.0.send_frame(d, t, s).await })
    }
    fn disconnect(
        self: Box<Self>,
    ) -> Pin<Box<dyn Future<Output = Result<(), BusSessionError>> + Send>> {
        Box::pin(async { Ok(()) })
    }
}
struct Connector {
    device: Arc<SimulatedDevice>,
    calls: Arc<AtomicUsize>,
    refuse: Arc<AtomicBool>,
}
impl GatewayConnector for Connector {
    fn connect_tunnel(
        &self,
        _: SocketAddrV4,
    ) -> Pin<Box<dyn Future<Output = Result<Box<dyn BusTunnel>, BusSessionError>> + Send + '_>>
    {
        self.calls.fetch_add(1, Ordering::SeqCst);
        if self.refuse.load(Ordering::SeqCst) {
            return Box::pin(async { Err(BusSessionError::Transport(BusError::Timeout)) });
        }
        let t: Box<dyn BusTunnel> = Box::new(Tunnel(self.device.clone()));
        Box::pin(async move { Ok(t) })
    }
    fn discover(
        &self,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<DiscoveredGateway>, BusSessionError>> + Send + '_>>
    {
        Box::pin(async { Ok(vec![]) })
    }
}
fn request(uri: &str, v: Value) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri(uri)
        .header("content-type", "application/json")
        .body(Body::from(v.to_string()))
        .unwrap()
}
async fn send(app: &axum::Router, uri: &str, v: Value) -> (StatusCode, Value) {
    let r = app.clone().oneshot(request(uri, v)).await.unwrap();
    let status = r.status();
    let bytes = to_bytes(r.into_body(), 1024 * 1024).await.unwrap();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}
struct Harness {
    _dir: tempfile::TempDir,
    state: Arc<knx_server::AppState>,
    app: axum::Router,
    device: Arc<SimulatedDevice>,
    calls: Arc<AtomicUsize>,
    refuse: Arc<AtomicBool>,
}
fn harness(address: &str) -> Harness {
    let dir = tempfile::tempdir().unwrap();
    let d = Arc::new(SimulatedDevice::with_config_at(
        address.parse().unwrap(),
        SimulatorConfig {
            mask_version: 0x0025,
            ..Default::default()
        },
    ));
    d.preset_property(0, 12, &[0, 1]);
    d.preset_property(0, 78, &[0, 1, 0, 0, 0, 0x5b]);
    let calls = Arc::new(AtomicUsize::new(0));
    let refuse = Arc::new(AtomicBool::new(false));
    let state = Arc::new(knx_server::AppState {
        connector: Box::new(Connector {
            device: d.clone(),
            calls: calls.clone(),
            refuse: refuse.clone(),
        }),
        // Test-owned lease: never the user's real per-user runtime lease.
        diagnostic_lease_dir: Some(dir.path().join("lease")),
        ..knx_server::AppState::new(dir.path().into())
    });
    let mut p = Project::new(Language("en".into()));
    p.devices.insert(DeviceInstance {
        id: DeviceId(7),
        source: SourceRef {
            path: "PRIVATE_PATH".into(),
            ets_id: "PRIVATE_SOURCE".into(),
        },
        name: "CUSTOMER_SECRET".into(),
        description: Some("PRIVATE_DESCRIPTION".into()),
        address: Some(address.parse().unwrap()),
        product_ref: "PRIVATE_PRODUCT".into(),
        program_ref: "PRIVATE_PROGRAM".into(),
        commissioning: Default::default(),
        visibility_calculated: true,
        com_objects: vec![],
        binary_data: vec![],
    });
    *state.project.lock().unwrap() = Some(p);
    let app = knx_server::app(state.clone(), None);
    Harness {
        _dir: dir,
        state,
        app,
        device: d,
        calls,
        refuse,
    }
}
fn context(s: &knx_server::AppState) -> Value {
    json!({"serverIncarnation":s.server_incarnation,"projectIncarnation":s.project_incarnation.load(Ordering::SeqCst),"revision":s.project_revision.load(Ordering::SeqCst)})
}

async fn diagnostic_preview(h: &Harness) -> Value {
    let (status, p) = send(
        &h.app,
        "/api/device-diagnostics/preview",
        json!({"deviceId":7,"gateway":"192.0.2.10:3671","context":context(&h.state)}),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{p}");
    p
}
async fn finished(h: &Harness, token: &str) -> Value {
    for _ in 0..100 {
        let r = h
            .app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/api/device-diagnostics/{token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(r.status(), StatusCode::OK);
        let v: Value =
            serde_json::from_slice(&to_bytes(r.into_body(), 1024 * 1024).await.unwrap()).unwrap();
        if v["phase"] == "complete" {
            return v;
        }
        tokio::time::sleep(std::time::Duration::from_millis(5)).await;
    }
    panic!("diagnosis did not finish");
}
#[tokio::test]
async fn selected_device_preview_has_no_bus_side_effect_and_explicit_start_exports_raw_bytes() {
    let h = harness("1.1.25");
    let p = diagnostic_preview(&h).await;
    assert_eq!(h.calls.load(Ordering::SeqCst), 0);
    assert!(h.device.seen().is_empty());
    assert_eq!(p["report"]["profile"], "generic-identity-v1");
    let (status, _) = send(
        &h.app,
        "/api/device-diagnostics/start",
        json!({"token":p["token"],"consent":false}),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(h.calls.load(Ordering::SeqCst), 0);
    let (status, _) = send(
        &h.app,
        "/api/device-diagnostics/start",
        json!({"token":p["token"],"consent":true}),
    )
    .await;
    assert_eq!(status, StatusCode::ACCEPTED);
    let result = finished(&h, p["token"].as_str().unwrap()).await;
    assert_eq!(result["report"]["items"][0]["bytes"], json!([0, 37]));
    assert_eq!(
        result["report"]["items"][2]["bytes"],
        json!([0, 1, 0, 0, 0, 91])
    );
    assert_eq!(h.calls.load(Ordering::SeqCst), 1);
    let r = h
        .app
        .clone()
        .oneshot(request(
            "/api/device-diagnostics/export",
            json!({"token":p["token"],"consent":true}),
        ))
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::OK);
    assert_eq!(r.headers()["content-type"], "application/zip");
    let bytes = to_bytes(r.into_body(), 1024 * 1024).await.unwrap();
    let report = knx_app::device_diagnostics::read_bundle(&bytes).unwrap();
    assert!(
        report.simulated,
        "simulator-backed API must not masquerade as a device capture"
    );
    assert_eq!(report.items[1].bytes, [0, 1]);
    let text = serde_json::to_string(&report).unwrap();
    assert!(!text.contains("PRIVATE"));
    assert!(!text.contains("CUSTOMER"));
    assert!(!text.contains("1.1.25"));
    assert!(h.device.seen().iter().all(|s| matches!(
        s,
        Seen::Connect
            | Seen::Disconnect
            | Seen::PropertyRead {
                object_index: 0,
                property_id: 12 | 78
            }
            | Seen::Other("DeviceDescriptorRead")
    )));
    assert_eq!(h.device.authorize_requests(), 0);
}
#[tokio::test]
async fn excluded_ambiguous_stale_unknown_or_unconfirmed_orders_never_open_a_tunnel() {
    let h = harness("1.1.25");
    for body in [
        json!({"deviceId":999,"gateway":"192.0.2.10:3671","context":context(&h.state)}),
        json!({"deviceId":7,"gateway":"usb:1","context":context(&h.state)}),
        json!({"deviceId":7,"gateway":"192.0.2.10:3671","context":context(&h.state),"ranges":[{"base":0,"length":65536}]}),
    ] {
        let (status, _) = send(&h.app, "/api/device-diagnostics/preview", body).await;
        assert!(!status.is_success());
    }
    let p = diagnostic_preview(&h).await;
    h.state.project_revision.fetch_add(1, Ordering::SeqCst);
    let (status, _) = send(
        &h.app,
        "/api/device-diagnostics/start",
        json!({"token":p["token"],"consent":true}),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
    let (status, _) = send(
        &h.app,
        "/api/device-diagnostics/start",
        json!({"token":"bogus","consent":true}),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(h.calls.load(Ordering::SeqCst), 0);
    for a in knx_core::EXCLUDED_INDIVIDUAL_ADDRESSES {
        let h = harness(&a.to_string());
        let (status, _) = send(
            &h.app,
            "/api/device-diagnostics/preview",
            json!({"deviceId":7,"gateway":"192.0.2.10:3671","context":context(&h.state)}),
        )
        .await;
        assert!(!status.is_success());
        assert_eq!(h.calls.load(Ordering::SeqCst), 0);
    }
}
#[tokio::test]
async fn a_held_cross_process_diagnostic_lease_refuses_before_any_tunnel() {
    let h = harness("1.1.25");
    let p = diagnostic_preview(&h).await;
    let _other =
        knx_app::device_diagnostics::lease_in(h.state.diagnostic_lease_dir.as_ref().unwrap())
            .unwrap();
    let (status, _) = send(
        &h.app,
        "/api/device-diagnostics/start",
        json!({"token":p["token"],"consent":true}),
    )
    .await;
    assert_eq!(status, StatusCode::ACCEPTED);
    let r = finished(&h, p["token"].as_str().unwrap()).await;
    assert_eq!(r["report"]["stopReason"], "gateway-busy");
    assert_eq!(h.calls.load(Ordering::SeqCst), 0);
    assert!(h.device.seen().is_empty());
}
#[tokio::test]
async fn existing_operation_lock_prevents_a_second_tunnel_and_keeps_failure_exportable() {
    let h = harness("1.1.25");
    let p = diagnostic_preview(&h).await;
    let _held = h.state.address_programming.lock().await;
    let (status, _) = send(
        &h.app,
        "/api/device-diagnostics/start",
        json!({"token":p["token"],"consent":true}),
    )
    .await;
    assert_eq!(status, StatusCode::ACCEPTED);
    let r = finished(&h, p["token"].as_str().unwrap()).await;
    assert_eq!(r["report"]["stopReason"], "gateway-busy");
    assert_eq!(h.calls.load(Ordering::SeqCst), 0);
    let (status, _) = send(
        &h.app,
        "/api/device-diagnostics/export",
        json!({"token":p["token"],"consent":true}),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
}
/// The durable activity history must accept the diagnostic run (an unknown
/// kind would mark history unavailable and block every later write), record
/// a known connect failure as failed rather than unknown, and never reuse the
/// capability token as the reference written into a shareable archive.
#[tokio::test]
async fn diagnostics_keep_activity_history_available_and_tokens_private() {
    let h = harness("1.1.25");
    let p = diagnostic_preview(&h).await;
    assert_ne!(p["token"], p["report"]["projectReference"]);
    let (status, _) = send(
        &h.app,
        "/api/device-diagnostics/start",
        json!({"token":p["token"],"consent":true}),
    )
    .await;
    assert_eq!(status, StatusCode::ACCEPTED);
    finished(&h, p["token"].as_str().unwrap()).await;
    h.refuse.store(true, Ordering::SeqCst);
    let q = diagnostic_preview(&h).await;
    let (status, _) = send(
        &h.app,
        "/api/device-diagnostics/start",
        json!({"token":q["token"],"consent":true}),
    )
    .await;
    assert_eq!(status, StatusCode::ACCEPTED);
    let failed = finished(&h, q["token"].as_str().unwrap()).await;
    assert_eq!(failed["report"]["stopReason"], "gateway-connect-failed");
    assert_eq!(h.state.one_shot_activity.history_state(), "configured");
    assert!(h.state.one_shot_activity.ensure_write_available().is_ok());
    let (entries, _) = h.state.one_shot_activity.history_page(0, 100).unwrap();
    let rows: Vec<(String, String)> = entries
        .iter()
        .map(|e| serde_json::to_value(e).unwrap())
        .map(|v| {
            (
                v["kind"].as_str().unwrap().into(),
                v["state"].as_str().unwrap().into(),
            )
        })
        .collect();
    // Chronological: the successful run, then the refused connect.
    assert_eq!(
        rows,
        vec![
            ("deviceDiagnostics".to_string(), "finished".to_string()),
            ("deviceDiagnostics".to_string(), "failed".to_string()),
        ]
    );
}
