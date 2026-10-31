//! Unknown targets, privacy boundaries and diagnostic admission through real HTTP handlers.
mod bundle_tests;
#[path = "../common/system2_diagnostic_fixture.rs"]
mod system2_fixture;
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
use tokio::sync::{broadcast, Semaphore};
use tower::ServiceExt;
/// The simulator's tunnel; with a gate, each `A_Memory_Read` first takes one
/// permit, so a test holds the run with exactly one chunk in flight.
struct Tunnel(Arc<SimulatedDevice>, Option<Arc<Semaphore>>);
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
        Box::pin(async move {
            if let Some(gate) = &self.1 {
                if matches!(s, ApplicationService::MemoryRead { .. }) {
                    gate.acquire().await.expect("gate open").forget();
                }
            }
            self.0.send_frame(d, t, s).await
        })
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
    gate: Option<Arc<Semaphore>>,
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
        let t: Box<dyn BusTunnel> = Box::new(Tunnel(self.device.clone(), self.gate.clone()));
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
    /// Permits for memory reads; `None` reads ungated.
    gate: Option<Arc<Semaphore>>,
}
fn harness(address: &str) -> Harness {
    harness_with(address, None)
}
fn harness_with(address: &str, products: Option<fn(&knx_productdb::Connection)>) -> Harness {
    harness_gated(address, products, None)
}
/// With `Some(edit)`, the device maps to the synthetic System-2 program in a
/// real product database (`edit` may then alter its catalogue rows) and the
/// simulator holds that program's declared memory.
fn harness_gated(
    address: &str,
    products: Option<fn(&knx_productdb::Connection)>,
    gate: Option<Arc<Semaphore>>,
) -> Harness {
    harness_gated_recording(address, products, gate, false)
}

fn harness_gated_recording(
    address: &str,
    products: Option<fn(&knx_productdb::Connection)>,
    gate: Option<Arc<Semaphore>>,
    disabled: bool,
) -> Harness {
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
    let product_db = products.map(|edit| {
        system2_fixture::preset_memory(&d);
        let path = dir.path().join("products.sqlite");
        let db = system2_fixture::products_at(&path, system2_fixture::PROGRAM);
        edit(&db);
        knx_server::CatalogueOwner::adopt_opened(db, path).unwrap()
    });
    let mapped = product_db.is_some();
    let mut initial = knx_server::AppState::new(dir.path().into());
    if disabled {
        initial.recordings = Arc::new(knx_app::operation_recording::RecordingStore::new(
            dir.path(),
            knx_app::operation_recording::WriterIdentity {
                program: "knx-server".into(),
                version: "test".into(),
                commit: None,
                build_profile: "test".into(),
            },
            true,
        ));
    }
    let state = Arc::new(knx_server::AppState {
        product_db,
        connector: Box::new(Connector {
            device: d.clone(),
            calls: calls.clone(),
            refuse: refuse.clone(),
            gate: gate.clone(),
        }),
        // Test-owned lease: never the user's real per-user runtime lease.
        diagnostic_lease_dir: Some(dir.path().join("lease")),
        ..initial
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
        product_ref: if mapped {
            system2_fixture::PRODUCT_REF.into()
        } else {
            "PRIVATE_PRODUCT".into()
        },
        program_ref: if mapped {
            system2_fixture::PROGRAM_REF.into()
        } else {
            "PRIVATE_PROGRAM".into()
        },
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
        gate,
    }
}
fn context(s: &knx_server::AppState) -> Value {
    json!({"serverIncarnation":s.server_incarnation,"projectIncarnation":s.project_incarnation.load(Ordering::SeqCst),"revision":s.project_revision.load(Ordering::SeqCst)})
}

const IDENTITY: &str = "generic-identity-v1";
const MEMORY: &str = "system2-declared-segments-v1";
fn preview_body(h: &Harness, profile: &str) -> Value {
    json!({"deviceId":7,"gateway":"192.0.2.10:3671","profile":profile,"context":context(&h.state)})
}
async fn diagnostic_preview(h: &Harness) -> Value {
    profile_preview(h, IDENTITY).await
}
async fn profile_preview(h: &Harness, profile: &str) -> Value {
    let (status, p) = send(
        &h.app,
        "/api/device-diagnostics/preview",
        preview_body(h, profile),
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
    assert_eq!(p["memory"], Value::Null);
    assert_eq!(p["exportable"], false);
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
    assert_eq!(result["memory"], Value::Null);
    assert_eq!(result["exportable"], true);
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
        json!({"deviceId":999,"gateway":"192.0.2.10:3671","profile":IDENTITY,"context":context(&h.state)}),
        json!({"deviceId":7,"gateway":"usb:1","profile":IDENTITY,"context":context(&h.state)}),
        json!({"deviceId":7,"gateway":"192.0.2.10:3671","profile":IDENTITY,"context":context(&h.state),"ranges":[{"base":0,"length":65536}]}),
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
            preview_body(&h, IDENTITY),
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

fn system2() -> Harness {
    harness_with("1.1.25", Some(|_| {}))
}
async fn start_and_finish(h: &Harness, p: &Value) -> Value {
    let (status, started) = send(
        &h.app,
        "/api/device-diagnostics/start",
        json!({"token":p["token"],"consent":true}),
    )
    .await;
    assert_eq!(status, StatusCode::ACCEPTED, "{started}");
    finished(h, p["token"].as_str().unwrap()).await
}
async fn export_bytes(h: &Harness, token: &Value) -> (StatusCode, Vec<u8>) {
    let r = h
        .app
        .clone()
        .oneshot(request(
            "/api/device-diagnostics/export",
            json!({"token":token,"consent":true}),
        ))
        .await
        .unwrap();
    let status = r.status();
    (
        status,
        to_bytes(r.into_body(), 1024 * 1024).await.unwrap().to_vec(),
    )
}
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// The web entry of ADR-0143: ranges come from the product database only, the
/// preview sends nothing, reads need consent, and the ZIP is a strict format 2.
#[tokio::test]
async fn the_memory_profile_reads_declared_ranges_after_consent_and_exports_format_2() {
    let h = system2();
    let p = profile_preview(&h, MEMORY).await;
    assert_eq!(h.calls.load(Ordering::SeqCst), 0);
    assert!(h.device.seen().is_empty());
    assert_eq!(p["phase"], "preview");
    assert_eq!(p["exportable"], false);
    assert_eq!(p["report"]["format"], 2);
    assert_eq!(p["report"]["profile"], MEMORY);
    assert_eq!(p["report"]["memoryStatus"], "requested");
    assert_eq!(p["report"]["assertedMask"], system2_fixture::MASK);
    assert_eq!(p["report"]["assertedManufacturer"], 1);
    let memory = &p["memory"];
    assert_eq!(memory["programId"], system2_fixture::PROGRAM_ID);
    let ranges: Vec<(u64, u64)> = memory["ranges"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| {
            (
                r["address"].as_u64().unwrap(),
                r["length"].as_u64().unwrap(),
            )
        })
        .collect();
    let declared: Vec<(u64, u64)> = system2_fixture::RANGES
        .iter()
        .map(|&(a, l)| (u64::from(a), u64::from(l)))
        .collect();
    assert_eq!(ranges, declared);
    assert_eq!(memory["ranges"][0]["coverage"]["notRead"], 30);
    let chunks = memory["chunks"].as_array().unwrap();
    assert_eq!(chunks.len(), 4);
    assert!(chunks.iter().all(|c| c["status"] == "not-read"));

    let (status, _) = send(
        &h.app,
        "/api/device-diagnostics/start",
        json!({"token":p["token"],"consent":false}),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(h.calls.load(Ordering::SeqCst), 0);
    let result = start_and_finish(&h, &p).await;
    assert_eq!(h.calls.load(Ordering::SeqCst), 1);
    assert_eq!(result["exportable"], true, "{result}");
    assert_eq!(result["report"]["stopReason"], Value::Null);
    assert_eq!(result["report"]["simulated"], true);
    // Independent oracle: the simulator's preset memory, chunk by chunk.
    let first: Vec<u8> = (1..=30).collect();
    let expected = [
        hex(&first[0..12]),
        hex(&first[12..24]),
        hex(&first[24..30]),
        hex(&[0xEE; 12]),
    ];
    let chunks = result["memory"]["chunks"].as_array().unwrap();
    let got: Vec<&str> = chunks
        .iter()
        .map(|c| c["bytes"].as_str().unwrap())
        .collect();
    assert_eq!(got, expected);
    assert_eq!(chunks[2]["address"], 0x0116 + 24);
    assert_eq!(chunks[3]["range"], 2);
    assert_eq!(result["memory"]["ranges"][0]["coverage"]["read"], 30);
    assert_eq!(result["memory"]["ranges"][1]["coverage"]["read"], 12);
    assert_eq!(result["memory"]["ranges"][1]["coverage"]["notRead"], 0);

    let (status, bytes) = export_bytes(&h, &p["token"]).await;
    assert_eq!(status, StatusCode::OK);
    let export = knx_app::device_diagnostics::read_export(&bytes).unwrap();
    assert_eq!(export.report.format, 2);
    assert_eq!(
        serde_json::to_value(&export.report).unwrap(),
        result["report"]
    );
    assert_eq!(
        serde_json::to_value(&export.memory).unwrap(),
        result["memory"]
    );
    // A format-2 archive is never silently reduced to its identity part.
    assert!(knx_app::device_diagnostics::read_bundle(&bytes).is_err());
    let text = format!(
        "{}{}",
        serde_json::to_string(&export.report).unwrap(),
        serde_json::to_string(&export.memory).unwrap()
    );
    for secret in [
        "PRIVATE",
        "CUSTOMER",
        "1.1.25",
        p["token"].as_str().unwrap(),
    ] {
        assert!(!text.contains(secret), "archive leaked {secret}");
    }
    let mut reads = vec![];
    for seen in h.device.seen() {
        match seen {
            Seen::Connect
            | Seen::Disconnect
            | Seen::PropertyRead {
                object_index: 0,
                property_id: 12 | 78,
            }
            | Seen::Other("DeviceDescriptorRead") => {}
            Seen::MemoryRead {
                address, number, ..
            } => reads.push((address, number)),
            other => panic!("forbidden service {other:?}"),
        }
    }
    assert_eq!(
        reads,
        [(0x0116, 12), (0x0122, 12), (0x012E, 6), (0x3C00, 12)]
    );
    assert_eq!(h.device.authorize_requests(), 0);
    let (entries, _) = h.state.one_shot_activity.history_page(0, 100).unwrap();
    let rows: Vec<Value> = entries
        .iter()
        .map(|e| serde_json::to_value(e).unwrap())
        .collect();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["kind"], "deviceDiagnostics");
    assert_eq!(rows[0]["state"], "finished");
}

#[tokio::test]
async fn memory_profile_refusals_come_before_any_tunnel() {
    async fn refusal(h: &Harness, body: Value) -> (StatusCode, Value) {
        let before = h.state.project_revision.load(Ordering::SeqCst);
        let r = send(&h.app, "/api/device-diagnostics/preview", body).await;
        assert_eq!(h.calls.load(Ordering::SeqCst), 0);
        assert!(h.device.seen().is_empty());
        assert_eq!(h.state.project_revision.load(Ordering::SeqCst), before);
        r
    }
    let h = system2();
    let (status, body) = refusal(&h, preview_body(&h, "system2-declared-segments-v2")).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"], "diagnostics-unknown-profile");
    let mut missing = preview_body(&h, MEMORY);
    missing.as_object_mut().unwrap().remove("profile");
    let (status, _) = refusal(&h, missing).await;
    assert!(status.is_client_error(), "{status}");
    // A start for a refused preview has nothing to start.
    let (status, _) = send(
        &h.app,
        "/api/device-diagnostics/start",
        json!({"token":"0".repeat(64),"consent":true}),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);

    let plain = harness("1.1.25");
    let (status, body) = refusal(&plain, preview_body(&plain, MEMORY)).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"], "diagnostics-product-db-required");

    // Without a decodable manufacturer the product data asserts no identity.
    let unasserted = harness_with(
        "1.1.25",
        Some(|db| {
            db.execute("UPDATE application_program SET manufacturer_id = 'M-X'", [])
                .unwrap();
        }),
    );
    let (status, body) = refusal(&unasserted, preview_body(&unasserted, MEMORY)).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"], "diagnostics-identity-assertion-missing");
    // The generic identity recipe still previews the same device.
    profile_preview(&unasserted, IDENTITY).await;

    let inconsistent = harness_with(
        "1.1.25",
        Some(|db| {
            db.execute(
                "UPDATE application_program SET mask_version = 'MV-0701'",
                [],
            )
            .unwrap();
        }),
    );
    let (status, body) = refusal(&inconsistent, preview_body(&inconsistent, MEMORY)).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"], "diagnostics-identity-inconsistent");

    // Excluded targets refuse for the memory profile as for identity.
    for a in knx_core::EXCLUDED_INDIVIDUAL_ADDRESSES {
        let h = harness_with(&a.to_string(), Some(|_| {}));
        let (status, _) = refusal(&h, preview_body(&h, MEMORY)).await;
        assert!(!status.is_success());
    }
}

/// The device shows another hardware type: identity stops the run, no memory
/// request is sent and the identity-only outcome remains exportable.
#[tokio::test]
async fn a_hardware_type_mismatch_sends_no_memory_read() {
    let h = system2();
    h.device.preset_property(0, 78, &[9, 9, 9, 9, 9, 9]);
    let p = profile_preview(&h, MEMORY).await;
    let result = start_and_finish(&h, &p).await;
    assert_eq!(result["report"]["stopReason"], "identity-mismatch");
    assert!(result["memory"]["chunks"]
        .as_array()
        .unwrap()
        .iter()
        .all(|c| c["status"] == "not-read" && c["attemptedAt"].is_null()));
    assert!(!h
        .device
        .seen()
        .iter()
        .any(|s| matches!(s, Seen::MemoryRead { .. })));
    assert_eq!(result["exportable"], true);
    let (status, bytes) = export_bytes(&h, &p["token"]).await;
    assert_eq!(status, StatusCode::OK);
    let export = knx_app::device_diagnostics::read_export(&bytes).unwrap();
    assert_eq!(
        export.report.stop_reason.as_deref(),
        Some("identity-mismatch")
    );
    bundle_tests::bundle_readback(&h, &p["token"], &result).await;
}

/// Format 2 admits no caller setup stop reason (ADR-0143); like the CLI,
/// which writes no archive then, the result is shown but not exportable.
#[tokio::test]
async fn a_memory_run_stopped_before_its_tunnel_is_shown_but_not_exportable() {
    let h = system2();
    let p = profile_preview(&h, MEMORY).await;
    let _other =
        knx_app::device_diagnostics::lease_in(h.state.diagnostic_lease_dir.as_ref().unwrap())
            .unwrap();
    let result = start_and_finish(&h, &p).await;
    assert_eq!(result["report"]["stopReason"], "gateway-busy");
    assert_eq!(result["exportable"], false);
    assert_eq!(h.calls.load(Ordering::SeqCst), 0);
    let (status, body) = export_bytes(&h, &p["token"]).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let body: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(body["error"], "diagnostics-invalid-memory");
}

/// The System-2 harness with memory reads held until the test adds permits.
fn gated_system2(permits: usize) -> Harness {
    harness_gated(
        "1.1.25",
        Some(|_| {}),
        Some(Arc::new(Semaphore::new(permits))),
    )
}
async fn view_of(h: &Harness, token: &str) -> Value {
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
    serde_json::from_slice(&to_bytes(r.into_body(), 1024 * 1024).await.unwrap()).unwrap()
}
/// Polls the running job until `ready` holds; a bounded wait, never a hang.
async fn poll_until(h: &Harness, token: &str, what: &str, ready: impl Fn(&Value) -> bool) -> Value {
    for _ in 0..400 {
        let v = view_of(h, token).await;
        if ready(&v) {
            return v;
        }
        tokio::time::sleep(std::time::Duration::from_millis(5)).await;
    }
    panic!("no progress snapshot with {what}");
}
fn chunks(v: &Value) -> &Vec<Value> {
    v["memory"]["chunks"].as_array().unwrap()
}
/// Attempted, not completed: the collector's one request on the bus.
fn in_flight(chunk: &Value) -> bool {
    chunk["status"] == "not-read"
        && !chunk["attemptedAt"].is_null()
        && chunk["completedAt"].is_null()
}
/// The running-view contract: not exportable, no stop reason, and exactly
/// the chunk at `index` in flight with nothing after it attempted.
fn assert_running_with_one_chunk_in_flight(v: &Value, index: usize) {
    assert_eq!(v["phase"], "running", "{v}");
    assert_eq!(
        v["exportable"], false,
        "a running view is never exportable: {v}"
    );
    assert_eq!(v["report"]["complete"], false);
    assert_eq!(v["report"]["stopReason"], Value::Null);
    let all = chunks(v);
    let flying: Vec<usize> = (0..all.len()).filter(|&i| in_flight(&all[i])).collect();
    assert_eq!(flying, [index], "exactly one chunk in flight: {v}");
    assert!(all[..index].iter().all(|c| c["status"] == "read"));
    assert!(all[index + 1..].iter().all(|c| c["attemptedAt"].is_null()));
}

/// Live progress reaches the browser chunk by chunk: each poll during a held
/// read shows one chunk in flight, and nothing is exportable before the end.
#[tokio::test]
async fn memory_progress_is_published_while_running_and_never_exportable_mid_run() {
    let h = gated_system2(0);
    let gate = h.gate.clone().unwrap();
    let p = profile_preview(&h, MEMORY).await;
    let token = p["token"].as_str().unwrap();
    let (status, _) = send(
        &h.app,
        "/api/device-diagnostics/start",
        json!({"token":token,"consent":true}),
    )
    .await;
    assert_eq!(status, StatusCode::ACCEPTED);
    let first = poll_until(&h, token, "chunk 1 in flight", |v| {
        chunks(v).first().is_some_and(in_flight)
    })
    .await;
    assert_running_with_one_chunk_in_flight(&first, 0);
    // The identity phase finished before any memory request.
    assert!(first["report"]["items"]
        .as_array()
        .unwrap()
        .iter()
        .all(|i| i["status"] == "read"));
    gate.add_permits(1);
    let second = poll_until(&h, token, "chunk 2 in flight", |v| {
        chunks(v).get(1).is_some_and(in_flight)
    })
    .await;
    assert_running_with_one_chunk_in_flight(&second, 1);
    let read: Vec<u8> = (1..=12).collect();
    assert_eq!(chunks(&second)[0]["bytes"], hex(&read));
    assert_eq!(second["memory"]["ranges"][0]["coverage"]["read"], 12);
    assert_eq!(second["memory"]["ranges"][0]["coverage"]["notRead"], 18);
    gate.add_permits(16);
    let done = finished(&h, token).await;
    assert_eq!(done["exportable"], true, "{done}");
    assert_eq!(done["report"]["stopReason"], Value::Null);
    assert!(chunks(&done).iter().all(|c| c["status"] == "read"));
}

/// Stop during the second chunk keeps the first: `cancelled`, partial
/// coverage, and a format-2 ZIP that reads back with exactly those chunks.
#[tokio::test]
async fn stopping_a_memory_run_keeps_the_chunks_read_so_far_and_exports_them() {
    let h = gated_system2(1);
    let p = profile_preview(&h, MEMORY).await;
    let token = p["token"].as_str().unwrap();
    let (status, _) = send(
        &h.app,
        "/api/device-diagnostics/start",
        json!({"token":token,"consent":true}),
    )
    .await;
    assert_eq!(status, StatusCode::ACCEPTED);
    let held = poll_until(&h, token, "chunk 1 read, chunk 2 in flight", |v| {
        chunks(v).get(1).is_some_and(in_flight)
    })
    .await;
    assert_running_with_one_chunk_in_flight(&held, 1);
    let (status, stopping) = send(
        &h.app,
        &format!("/api/device-diagnostics/{token}/cancel"),
        json!({}),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{stopping}");
    assert_eq!(stopping["cancelRequested"], true);
    // No permit is added: only the cancel can end the held read.
    let result = finished(&h, token).await;
    assert_eq!(result["report"]["stopReason"], "cancelled", "{result}");
    assert_eq!(result["exportable"], true, "{result}");
    let all = chunks(&result);
    let read: Vec<u8> = (1..=12).collect();
    assert_eq!(all[0]["status"], "read");
    assert_eq!(all[0]["bytes"], hex(&read));
    assert_eq!(all[1]["status"], "error");
    assert_eq!(all[1]["error"], "cancelled");
    assert!(all[2..]
        .iter()
        .all(|c| c["status"] == "not-read" && c["attemptedAt"].is_null()));
    assert_eq!(
        result["memory"]["ranges"][0]["coverage"],
        json!({"read":12,"refusedAmbiguous":0,"partial":0,"error":12,"notRead":6})
    );
    assert_eq!(
        result["memory"]["ranges"][1]["coverage"],
        json!({"read":0,"refusedAmbiguous":0,"partial":0,"error":0,"notRead":12})
    );
    let reads: Vec<(u32, u8)> = h
        .device
        .seen()
        .into_iter()
        .filter_map(|s| match s {
            Seen::MemoryRead {
                address, number, ..
            } => Some((address, number)),
            _ => None,
        })
        .collect();
    assert_eq!(
        reads,
        [(0x0116, 12)],
        "only the first chunk reached the device"
    );

    let (status, bytes) = export_bytes(&h, &p["token"]).await;
    assert_eq!(status, StatusCode::OK);
    let export = knx_app::device_diagnostics::read_export(&bytes).unwrap();
    assert_eq!(export.report.format, 2);
    assert_eq!(export.report.stop_reason.as_deref(), Some("cancelled"));
    bundle_tests::bundle_readback(&h, &p["token"], &result).await;
    assert_eq!(
        serde_json::to_value(&export.memory).unwrap(),
        result["memory"]
    );
}

/// A Stop that lands before the tunnel opens still ends as `cancelled` with
/// an exportable format-2 archive: nothing was requested from the device.
#[tokio::test]
async fn a_memory_run_cancelled_before_connecting_is_exportable_without_any_request() {
    let h = system2();
    let p = profile_preview(&h, MEMORY).await;
    let token = p["token"].as_str().unwrap();
    // On the test's single-threaded runtime the spawned run cannot start
    // before this task yields, so the cancel is recorded first.
    let (status, _) = send(
        &h.app,
        "/api/device-diagnostics/start",
        json!({"token":token,"consent":true}),
    )
    .await;
    assert_eq!(status, StatusCode::ACCEPTED);
    let (status, _) = send(
        &h.app,
        &format!("/api/device-diagnostics/{token}/cancel"),
        json!({}),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let result = finished(&h, token).await;
    assert_eq!(h.calls.load(Ordering::SeqCst), 0, "no tunnel was opened");
    assert!(h.device.seen().is_empty());
    assert_eq!(result["report"]["stopReason"], "cancelled");
    assert_eq!(result["exportable"], true, "{result}");
    let (status, bytes) = export_bytes(&h, &p["token"]).await;
    assert_eq!(status, StatusCode::OK);
    let export = knx_app::device_diagnostics::read_export(&bytes).unwrap();
    assert_eq!(export.report.format, 2);
    assert_eq!(export.report.stop_reason.as_deref(), Some("cancelled"));
    assert!(export
        .memory
        .unwrap()
        .chunks
        .iter()
        .all(|c| c.status == "not-read" && c.attempted_at.is_none()));
}
