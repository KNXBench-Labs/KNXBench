//! Explicit-TCP device diagnostics over the strict sibling route, with a real loopback TCP peer.
use super::*;
use knx_net::{TunnelAckWait, TunnelTransport};
use std::time::Duration;
// One loopback peer for comparison and diagnostics, never a second copy of it.
use crate::http_device_compare::link_adoption::tcp::peer;

type Selections = Arc<std::sync::Mutex<Vec<TunnelTransport>>>;
/// Refuses every connect after recording which transport the route selected.
struct Selected(Selections);
impl GatewayConnector for Selected {
    fn connect_tunnel(
        &self,
        _: SocketAddrV4,
    ) -> Pin<Box<dyn Future<Output = Result<Box<dyn BusTunnel>, BusSessionError>> + Send + '_>>
    {
        // The unchanged legacy route connects through the plain (UDP) seam.
        self.0.lock().unwrap().push(TunnelTransport::Udp);
        Box::pin(async {
            Err(BusSessionError::Transport(BusError::Protocol(
                "synthetic connect refusal".into(),
            )))
        })
    }
    fn connect_tunnel_recorded_with_transport(
        &self,
        _: SocketAddrV4,
        _: TunnelAckWait,
        _: Option<knx_net::wire_tap::WireTap>,
        transport: TunnelTransport,
    ) -> Pin<Box<dyn Future<Output = Result<Box<dyn BusTunnel>, BusSessionError>> + Send + '_>>
    {
        self.0.lock().unwrap().push(transport);
        Box::pin(async {
            Err(BusSessionError::Transport(BusError::Protocol(
                "synthetic connect refusal".into(),
            )))
        })
    }
    fn discover(
        &self,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<DiscoveredGateway>, BusSessionError>> + Send + '_>>
    {
        panic!("no discovery")
    }
}

struct Tcp {
    _dir: tempfile::TempDir,
    state: Arc<knx_server::AppState>,
    app: axum::Router,
    device: Arc<SimulatedDevice>,
}
fn tcp_harness(connector: Box<dyn GatewayConnector>) -> Tcp {
    let dir = tempfile::tempdir().unwrap();
    let device = Arc::new(SimulatedDevice::with_config_at(
        "1.1.25".parse().unwrap(),
        SimulatorConfig {
            mask_version: 0x0025,
            ..Default::default()
        },
    ));
    device.preset_property(0, 12, &[0, 1]);
    device.preset_property(0, 78, &[0, 1, 0, 0, 0, 0x5b]);
    let initial = knx_server::AppState::new(dir.path().into());
    let state = Arc::new(knx_server::AppState {
        connector,
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
        description: None,
        address: Some("1.1.25".parse().unwrap()),
        product_ref: "PRIVATE_PRODUCT".into(),
        program_ref: "PRIVATE_PROGRAM".into(),
        commissioning: Default::default(),
        visibility_calculated: true,
        com_objects: vec![],
        binary_data: vec![],
    });
    *state.project.lock().unwrap() = Some(p);
    let app = knx_server::app(state.clone(), None);
    Tcp {
        _dir: dir,
        state,
        app,
        device,
    }
}
fn body(h: &Tcp, gateway: &str) -> Value {
    json!({"deviceId":7,"gateway":gateway,"profile":IDENTITY,"context":context(&h.state)})
}
const PREVIEW_TCP: &str = "/api/device-diagnostics/preview/tcp";
const UNOPENED: fn() -> Value = || {
    json!({"requestedTransport":"tcp","actualTransport":null,
        "assignedAddress":null,"skippedFramesBeforeDisconnect":null})
};
async fn start(h: &Tcp, token: &Value) -> (StatusCode, Value) {
    send(
        &h.app,
        "/api/device-diagnostics/start",
        json!({"token":token,"consent":true}),
    )
    .await
}
async fn complete(h: &Tcp, token: &str) -> Value {
    for _ in 0..1500 {
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
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    panic!("TCP diagnosis did not finish");
}
fn capture(name: &str, value: &Value) {
    // Producer-to-browser contract: the Web fixture replays these exact bodies when asked.
    if let Some(dir) = std::env::var_os("KNX_DIAGNOSTICS_HTTP_CAPTURE_DIR") {
        std::fs::write(
            std::path::Path::new(&dir).join(format!("{name}.json")),
            value.to_string(),
        )
        .unwrap();
    }
}
fn no_writes(device: &SimulatedDevice) -> bool {
    !device.seen().into_iter().any(|seen| {
        matches!(
            seen,
            Seen::MemoryWrite { .. } | Seen::PropertyWrite { .. } | Seen::Restart { .. }
        )
    })
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn strict_sibling_refusals_do_no_io_and_legacy_routes_keep_their_shape() {
    let selections: Selections = Default::default();
    let h = tcp_harness(Box::new(Selected(selections.clone())));
    let valid = body(&h, "192.0.2.10:3671");
    for (expected, path, body) in [
        (422, PREVIEW_TCP, {
            let mut v = valid.clone();
            v.as_object_mut().unwrap().remove("context");
            v
        }),
        (422, PREVIEW_TCP, {
            let mut v = valid.clone();
            v["transport"] = json!("tcp");
            v
        }),
        (422, PREVIEW_TCP, {
            let mut v = valid.clone();
            v["context"]["extra"] = json!(1);
            v
        }),
        (400, PREVIEW_TCP, {
            let mut v = valid.clone();
            v["profile"] = json!("no-such-profile");
            v
        }),
        (400, PREVIEW_TCP, {
            let mut v = valid.clone();
            v["gateway"] = json!("usb:1");
            v
        }),
        (400, PREVIEW_TCP, {
            let mut v = valid.clone();
            v["gateway"] = json!("0.0.0.0:3671");
            v
        }),
        // The legacy body stays strict: no transport field can ride on it.
        (422, "/api/device-diagnostics/preview", {
            let mut v = valid.clone();
            v["transport"] = json!("tcp");
            v
        }),
    ] {
        let (status, rb) = send(&h.app, path, body).await;
        assert_eq!(status.as_u16(), expected, "{path}: {rb}");
    }
    assert!(selections.lock().unwrap().is_empty());

    // TCP preview: nothing contacted, transport shown as requested, nothing actual yet.
    let preview = {
        let (status, p) = send(&h.app, PREVIEW_TCP, valid.clone()).await;
        assert_eq!(status, StatusCode::OK, "{p}");
        p
    };
    assert_eq!(preview["connection"], UNOPENED());
    assert!(selections.lock().unwrap().is_empty());
    // Gateway busy: refused before any connect, facts unavailable and not zero.
    let reservation = h.state.address_programming.lock().await;
    assert_eq!(start(&h, &preview["token"]).await.0, StatusCode::ACCEPTED);
    let busy = complete(&h, preview["token"].as_str().unwrap()).await;
    drop(reservation);
    assert_eq!(busy["report"]["stopReason"], "gateway-busy");
    assert_eq!(busy["connection"], UNOPENED());
    assert!(selections.lock().unwrap().is_empty());

    // Refused connect: exactly one TCP selection, no UDP retry, unopened facts.
    let preview = send(&h.app, PREVIEW_TCP, valid.clone()).await.1;
    assert_eq!(start(&h, &preview["token"]).await.0, StatusCode::ACCEPTED);
    let failed = complete(&h, preview["token"].as_str().unwrap()).await;
    assert_eq!(failed["report"]["stopReason"], "gateway-connect-failed");
    assert_eq!(failed["connection"], UNOPENED());
    assert_eq!(*selections.lock().unwrap(), [TunnelTransport::Tcp]);
    // Identity (format 1) failures stay exportable, exactly like the UDP run.
    assert_eq!(failed["exportable"], true);

    // Control (refused connect only, not a completed UDP read): legacy picks UDP, no `connection`.
    let (status, legacy) = send(&h.app, "/api/device-diagnostics/preview", valid.clone()).await;
    assert_eq!(status, StatusCode::OK, "{legacy}");
    assert!(legacy.get("connection").is_none());
    assert_eq!(start(&h, &legacy["token"]).await.0, StatusCode::ACCEPTED);
    let legacy = complete(&h, legacy["token"].as_str().unwrap()).await;
    assert!(legacy.get("connection").is_none());
    assert_eq!(
        *selections.lock().unwrap(),
        [TunnelTransport::Tcp, TunnelTransport::Udp]
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn real_loopback_tcp_runs_report_actual_facts_and_export_stays_format_1() {
    // (peer injects skipped frames, peer closes the stream mid-read) → expected skipped frames.
    for (skips, eof, expected) in [(false, false, 0), (true, false, 2), (true, true, 4)] {
        let h = tcp_harness(Box::new(knx_server::RealConnector::default()));
        let device = h.device.clone();
        let (gateway, worker, trap) = peer::gateway(250, move |peer| {
            peer.simulate(&device, skips, |frame| {
                eof && matches!(
                    frame.service,
                    ApplicationService::PropertyValueRead {
                        property_id: 78,
                        ..
                    }
                )
            });
        });
        let (status, preview) = send(&h.app, PREVIEW_TCP, body(&h, &gateway.to_string())).await;
        assert_eq!(status, StatusCode::OK, "{preview}");
        assert_eq!(preview["connection"], UNOPENED());
        assert!(h.device.seen().is_empty(), "preview is no bus I/O");
        let (status, started) = start(&h, &preview["token"]).await;
        assert_eq!(status, StatusCode::ACCEPTED);
        let done = complete(&h, preview["token"].as_str().unwrap()).await;
        let wire = worker.join().unwrap();
        peer::no_retry(&trap);
        eprintln!(
            "diagnostics-tcp-receipt {}",
            json!({"skips":skips,"eof":eof,"stopReason":done["report"]["stopReason"],
                "connection":done["connection"],"wireFrames":wire.len()})
        );
        assert_eq!(
            done["connection"],
            json!({"requestedTransport":"tcp","actualTransport":"tcp",
                "assignedAddress":"1.1.250","skippedFramesBeforeDisconnect":expected})
        );
        assert!(no_writes(&h.device));
        // Default-on recording carries the requested transport and the sampled facts.
        assert!(h.state.recordings.wait_until_idle(Duration::from_secs(10)));
        let recordings = h.state.recordings.list().unwrap();
        assert_eq!(recordings.len(), 1, "{recordings:?}");
        let text = String::from_utf8(h.state.recordings.read(&recordings[0].id).unwrap()).unwrap();
        let lines: Vec<Value> = text
            .lines()
            .map(|l| serde_json::from_str(l).unwrap())
            .collect();
        assert_eq!(
            lines[0]["settings"]["requestedTransport"], "tcp",
            "{}",
            lines[0]
        );
        let noted = lines
            .iter()
            .find(|l| {
                l["event"] == "connection" || l["name"] == "connection" || l["key"] == "connection"
            })
            .unwrap_or_else(|| panic!("no connection note in {text}"));
        assert!(
            noted
                .to_string()
                .contains("\"assignedAddress\":\"1.1.250\""),
            "{noted}"
        );
        if eof {
            assert_eq!(done["exportable"], true);
            assert!(done["report"]["stopReason"].is_string());
            capture("eof", &done);
            assert_eq!(
                done["report"]["items"][2]["status"], "error",
                "{}",
                done["report"]["items"][2]
            );
        } else {
            assert_eq!(done["report"]["stopReason"], Value::Null);
            assert_eq!(done["report"]["items"][0]["bytes"], json!([0, 37]));
            assert_eq!(
                done["report"]["items"][2]["bytes"],
                json!([0, 1, 0, 0, 0, 91])
            );
            if skips {
                capture("preview", &preview);
                capture("start", &started);
                capture("complete", &done);
            }
        }
        // Export of the TCP run: exactly the format-1 bytes of its report, no transport facts.
        let (status, bytes) = {
            let r = h
                .app
                .clone()
                .oneshot(request(
                    "/api/device-diagnostics/export",
                    json!({"token":preview["token"],"consent":true}),
                ))
                .await
                .unwrap();
            let status = r.status();
            (
                status,
                to_bytes(r.into_body(), 1024 * 1024).await.unwrap().to_vec(),
            )
        };
        assert_eq!(status, StatusCode::OK);
        let report: knx_app::device_diagnostics::Report =
            serde_json::from_value(done["report"].clone()).unwrap();
        let again =
            knx_app::device_diagnostics::bundle_export(&knx_app::device_diagnostics::Export {
                report: report.clone(),
                memory: None,
            })
            .unwrap();
        assert_eq!(bytes, again, "export bytes derive from the report alone");
        assert_eq!(
            knx_app::device_diagnostics::read_bundle(&bytes)
                .unwrap()
                .format,
            1
        );
        let text = serde_json::to_string(&report).unwrap();
        assert!(!text.contains("requestedTransport") && !text.contains("actualTransport"));
        assert!(!text.contains("assignedAddress") && !text.contains("1.1.250"));
    }
}
