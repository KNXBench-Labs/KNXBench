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
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};
use tokio::sync::broadcast;
use tower::ServiceExt;
struct Tunnel(Arc<SimulatedDevice>);
impl BusTunnel for Tunnel {
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
}
impl GatewayConnector for Connector {
    fn connect_tunnel(
        &self,
        _: SocketAddrV4,
    ) -> Pin<Box<dyn Future<Output = Result<Box<dyn BusTunnel>, BusSessionError>> + Send + '_>>
    {
        self.calls.fetch_add(1, Ordering::SeqCst);
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
    let state = Arc::new(knx_server::AppState {
        connector: Box::new(Connector {
            device: d.clone(),
            calls: calls.clone(),
        }),
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
    }
}
fn context(s: &knx_server::AppState) -> Value {
    json!({"serverIncarnation":s.server_incarnation,"projectIncarnation":s.project_incarnation.load(Ordering::SeqCst),"revision":s.project_revision.load(Ordering::SeqCst)})
}
#[tokio::test]
async fn a_new_acquisition_replaces_its_live_projection_without_mixing_observations() {
    let h = harness("1.1.67");
    let (status, p) = send(
        &h.app,
        "/api/support/preview",
        json!({"deviceId":7,"context":context(&h.state),"input":{"mask":37}}),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{p}");
    let body = json!({"token":p["token"],"gateway":"192.0.2.10:3671","consent":true});
    let (status, first) = send(&h.app, "/api/support/diagnose", body.clone()).await;
    assert_eq!(status, StatusCode::OK, "{first}");
    h.device.preset_property(0, 78, &[0, 1, 0, 0, 0, 0x5c]);
    let (status, next) = send(&h.app, "/api/support/diagnose", body).await;
    assert_eq!(status, StatusCode::OK, "{next}");
    let facts = next["report"]["facts"].as_array().unwrap();
    let hardware: Vec<_> = facts
        .iter()
        .filter(|f| f["field"] == "hardwareType" && f["origin"] == "live")
        .collect();
    assert_eq!(
        hardware.len(),
        1,
        "current acquisition must not retain unlabelled prior facts"
    );
    assert_eq!(hardware[0]["value"], "00010000005C");
    assert_ne!(first["digest"], next["digest"]);
    let (status, _) = send(
        &h.app,
        "/api/support/export",
        json!({"token":first["token"],"digest":first["digest"],"consent":true}),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::CONFLICT,
        "old acquisition digest cannot export new facts"
    );
    h.device.preset_property(0, 78, &[0]);
    let (status, partial) = send(
        &h.app,
        "/api/support/diagnose",
        json!({"token":next["token"],"gateway":"192.0.2.10:3671","consent":true}),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{partial}");
    let unavailable: Vec<_> = partial["report"]["facts"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|f| f["field"] == "hardwareType" && f["origin"] == "live")
        .collect();
    assert_eq!(unavailable.len(), 1);
    assert!(unavailable[0]["value"].is_null());
    assert_eq!(unavailable[0]["status"], "unavailable-or-refused");
    assert_eq!(h.calls.load(Ordering::SeqCst), 3);
}

#[tokio::test]
async fn live_collection_needs_no_download_plan_but_requires_explicit_consent() {
    let h = harness("1.1.67");
    let (status, p) = send(
        &h.app,
        "/api/support/preview",
        json!({"deviceId":7,"context":context(&h.state),"input":{"mask":37}}),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{p}");
    assert_eq!(h.calls.load(Ordering::SeqCst), 0);
    let (status, _) = send(
        &h.app,
        "/api/support/diagnose",
        json!({"token":p["token"],"gateway":"192.0.2.10:3671","consent":false}),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(h.calls.load(Ordering::SeqCst), 0);
    let (status, p) = send(
        &h.app,
        "/api/support/diagnose",
        json!({"token":p["token"],"gateway":"192.0.2.10:3671","consent":true}),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{p}");
    assert_eq!(h.calls.load(Ordering::SeqCst), 1);
    let text = p["report"].to_string();
    assert!(text.contains("hardwareType"));
    for secret in [
        "CUSTOMER_SECRET",
        "PRIVATE_PATH",
        "PRIVATE_DESCRIPTION",
        "PRIVATE_PRODUCT",
        "PRIVATE_PROGRAM",
        "1.1.67",
        "192.0.2.10",
    ] {
        assert!(!text.contains(secret), "{secret}");
    }
    assert!(!h.device.seen().iter().any(|x| matches!(
        x,
        Seen::MemoryWrite { .. }
            | Seen::PropertyWrite { .. }
            | Seen::Authorize(_)
            | Seen::Restart { .. }
    )));
}
#[tokio::test]
async fn changed_project_or_replaced_preview_cannot_export_or_contact_hardware() {
    let h = harness("1.1.67");
    let (_, p) = send(
        &h.app,
        "/api/support/preview",
        json!({"deviceId":7,"context":context(&h.state),"input":{}}),
    )
    .await;
    h.state.project_revision.fetch_add(1, Ordering::SeqCst);
    let (status, _) = send(
        &h.app,
        "/api/support/diagnose",
        json!({"token":p["token"],"gateway":"192.0.2.10:3671","consent":true}),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(h.calls.load(Ordering::SeqCst), 0);
    let (status, _) = send(
        &h.app,
        "/api/support/export",
        json!({"token":p["token"],"digest":p["digest"],"consent":true}),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
    let (_, new) = send(&h.app, "/api/support/preview", json!({"input":{}})).await;
    let (status, _) = send(
        &h.app,
        "/api/support/export",
        json!({"token":p["token"],"digest":p["digest"],"consent":true}),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
    let (status, _) = send(
        &h.app,
        "/api/support/export",
        json!({"token":new["token"],"digest":"wrong","consent":true}),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
}
#[tokio::test]
async fn source_rights_and_request_grammar_are_enforced_on_the_server() {
    let h = harness("1.1.67");
    for body in [
        json!({"input":{"includeContext":true,"contextRights":false}}),
        json!({"input":{"password":"secret"}}),
        json!({"input":{},"deviceId":7}),
    ] {
        let (status, _) = send(&h.app, "/api/support/preview", body).await;
        assert!(status.is_client_error());
    }
    assert_eq!(h.calls.load(Ordering::SeqCst), 0);
    assert_eq!(
        h.state
            .project
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .devices
            .iter()
            .count(),
        1
    );
}
#[tokio::test]
async fn a_manual_unknown_target_is_reportable_without_a_project() {
    let h = harness("1.1.67");
    *h.state.project.lock().unwrap() = None;
    let (status, p) = send(
        &h.app,
        "/api/support/preview",
        json!({"address":"1.1.67","input":{}}),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{p}");
    assert_eq!(p["localAddress"], "1.1.67");
    assert!(!p["report"].to_string().contains("1.1.67"));
    let (status, p) = send(
        &h.app,
        "/api/support/diagnose",
        json!({"token":p["token"],"gateway":"192.0.2.10:3671","consent":true}),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{p}");
    assert_eq!(p["report"]["support"], "unknown");
    assert_eq!(h.calls.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn excluded_target_can_report_offline_but_cannot_be_read() {
    let addr = knx_core::EXCLUDED_INDIVIDUAL_ADDRESSES[0].to_string();
    let h = harness(&addr);
    for manual in [false, true] {
        if manual {
            *h.state.project.lock().unwrap() = None;
        }
        let body = if manual {
            json!({"address":addr,"input":{}})
        } else {
            json!({"deviceId":7,"context":context(&h.state),"input":{}})
        };
        let (status, p) = send(&h.app, "/api/support/preview", body).await;
        assert_eq!(status, StatusCode::OK, "{p}");
        assert_eq!(
            p["report"]["support"], "excluded",
            "known target policy must not depend on catalogue availability"
        );
        assert!(!p["report"].to_string().contains(&addr));
        let (status, _) = send(
            &h.app,
            "/api/support/diagnose",
            json!({"token":p["token"],"gateway":"192.0.2.10:3671","consent":true}),
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(h.calls.load(Ordering::SeqCst), 0);
    }
}
