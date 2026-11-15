//! Explicit-TCP service control over the strict sibling routes, with a real loopback TCP peer.
use super::*;
use crate::http_device_compare::link_adoption::tcp::peer;
use knx_net::{TunnelAckWait, TunnelTransport};
use std::sync::atomic::Ordering as O;

type Selections = Arc<Mutex<Vec<TunnelTransport>>>;
/// Refuses every connect after recording which transport the route selected.
struct Selected(Selections);
impl GatewayConnector for Selected {
    fn connect_tunnel(
        &self,
        _: SocketAddrV4,
    ) -> Pin<Box<dyn Future<Output = Result<Box<dyn BusTunnel>, BusSessionError>> + Send + '_>>
    {
        // The unchanged legacy routes connect through the plain (UDP) seam.
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
async fn tcp_harness(connector: Box<dyn GatewayConnector>) -> Tcp {
    let dir = tempfile::tempdir().unwrap();
    let device = Arc::new(SimulatedDevice::with_config(locked_device()));
    let state = Arc::new(knx_server::AppState {
        connector,
        ..knx_server::AppState::new(dir.path().to_path_buf())
    });
    let h = Tcp {
        app: knx_server::app(state.clone(), None),
        _dir: dir,
        state,
        device,
    };
    let (status, body) = send(
        &h.app,
        Request::builder()
            .method("PUT")
            .uri("/api/settings")
            .header("content-type", "application/json")
            .body(Body::from(
                json!({"settings":{"debugIndividualAddressWriteEnable":true}}).to_string(),
            ))
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    h
}

const READ_TCP: &str = "/api/device/service-control/tcp/read";
const WRITE_TCP: &str = "/api/device/service-control/tcp";
const UNOPENED: fn() -> Value = || {
    json!({"requestedTransport":"tcp","actualTransport":null,
        "assignedAddress":null,"skippedFramesBeforeDisconnect":null})
};
fn binding(h: &Tcp) -> Value {
    json!({"serverIncarnation":h.state.server_incarnation,
        "projectIncarnation":h.state.project_incarnation.load(O::SeqCst),
        "revision":h.state.project_revision.load(O::SeqCst)})
}
fn read_body(h: &Tcp, gateway: &str) -> Value {
    json!({"address":h.device.address().to_string(),"gateway":gateway,"binding":binding(h)})
}
fn write_body(h: &Tcp, gateway: &str, enable: bool) -> Value {
    let address = h.device.address();
    json!({"address":address.to_string(),"gateway":gateway,"confirmation":phrase(address),
        "enable":enable,"binding":binding(h)})
}
fn capture(name: &str, status: StatusCode, value: &Value) {
    // Producer-to-browser contract: the Web fixture replays these exact bodies when asked.
    if let Some(dir) = std::env::var_os("KNX_SERVICE_CONTROL_HTTP_CAPTURE_DIR") {
        std::fs::write(
            std::path::Path::new(&dir).join(format!("{name}.json")),
            json!({"status":status.as_u16(),"body":value}).to_string(),
        )
        .unwrap();
    }
}
fn no_writes(device: &SimulatedDevice) -> bool {
    !device.seen().iter().any(|s| {
        matches!(
            s,
            knx_net::commissioning::simulator::Seen::PropertyWrite { property_id: 8, .. }
        )
    })
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn strict_refusals_do_no_io_and_legacy_routes_keep_their_shape() {
    let selections: Selections = Default::default();
    let h = tcp_harness(Box::new(Selected(selections.clone()))).await;
    let read = read_body(&h, GATEWAY);
    let write = write_body(&h, GATEWAY, true);
    let mut cases: Vec<(u16, &str, Value)> = Vec::new();
    for (path, valid) in [(READ_TCP, &read), (WRITE_TCP, &write)] {
        let mut missing = valid.clone();
        missing.as_object_mut().unwrap().remove("binding");
        cases.push((422, path, missing));
        let mut unknown = valid.clone();
        unknown["transport"] = json!("tcp");
        cases.push((422, path, unknown));
        let mut nested = valid.clone();
        nested["binding"]["extra"] = json!(1);
        cases.push((422, path, nested));
        let mut gateway = valid.clone();
        gateway["gateway"] = json!("usb:1");
        cases.push((400, path, gateway));
        let mut stale = valid.clone();
        stale["binding"]["revision"] = json!(999);
        cases.push((409, path, stale));
        let mut foreign = valid.clone();
        foreign["binding"]["serverIncarnation"] = json!("someone-else");
        cases.push((409, path, foreign));
        let mut wrong_project = valid.clone();
        wrong_project["binding"]["projectIncarnation"] = json!(999);
        cases.push((409, path, wrong_project));
    }
    let mut bad_phrase = write.clone();
    bad_phrase["confirmation"] = json!("yes");
    cases.push((400, WRITE_TCP, bad_phrase));
    for (expected, path, body) in cases {
        let (status, rb) = send(&h.app, post(path, body)).await;
        assert_eq!(status.as_u16(), expected, "{path}: {rb}");
    }
    assert!(selections.lock().unwrap().is_empty(), "refusals do no I/O");
    assert_eq!(h.device.seen().len(), 0);

    // Debug gate: off means 403 before binding or I/O, even for the TCP siblings.
    let (status, _) = send(
        &h.app,
        Request::builder()
            .method("PUT")
            .uri("/api/settings")
            .header("content-type", "application/json")
            .body(Body::from(
                json!({"settings":{"debugIndividualAddressWriteEnable":false}}).to_string(),
            ))
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    for (path, body) in [(READ_TCP, &read), (WRITE_TCP, &write)] {
        let mut stale = body.clone();
        stale["binding"]["revision"] = json!(999);
        assert_eq!(
            send(&h.app, post(path, stale)).await.0,
            StatusCode::FORBIDDEN
        );
        let mut wrong_project = body.clone();
        wrong_project["binding"]["projectIncarnation"] = json!(999);
        assert_eq!(
            send(&h.app, post(path, wrong_project)).await.0,
            StatusCode::FORBIDDEN
        );
        assert_eq!(
            send(&h.app, post(path, body.clone())).await.0,
            StatusCode::FORBIDDEN
        );
    }
    assert!(selections.lock().unwrap().is_empty());
    let (status, _) = send(
        &h.app,
        Request::builder()
            .method("PUT")
            .uri("/api/settings")
            .header("content-type", "application/json")
            .body(Body::from(
                json!({"settings":{"debugIndividualAddressWriteEnable":true}}).to_string(),
            ))
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    // Refused TCP connect: exactly one TCP selection, no UDP retry, unavailable facts.
    let (status, refused) = send(&h.app, post(READ_TCP, read.clone())).await;
    assert_eq!(status, StatusCode::BAD_GATEWAY, "{refused}");
    assert_eq!(refused["connection"], UNOPENED());
    capture("refused-read", status, &refused);
    let (status, refused) = send(&h.app, post(WRITE_TCP, write.clone())).await;
    assert_eq!(status, StatusCode::BAD_GATEWAY, "{refused}");
    assert_eq!(refused["connection"], UNOPENED());
    assert_eq!(
        *selections.lock().unwrap(),
        [TunnelTransport::Tcp, TunnelTransport::Tcp]
    );

    // Control (refused connect only, not a completed UDP read): legacy picks UDP, no `connection`.
    let (status, legacy) = send(&h.app, get(&read_uri(h.device.address()))).await;
    assert_eq!(status, StatusCode::BAD_GATEWAY, "{legacy}");
    assert!(legacy.get("connection").is_none(), "{legacy}");
    let legacy_write = {
        let address = h.device.address();
        write_request(address, &phrase(address), true)
    };
    let (status, legacy) = send(&h.app, post("/api/device/service-control", legacy_write)).await;
    assert_eq!(status, StatusCode::BAD_GATEWAY);
    assert!(legacy.get("connection").is_none(), "{legacy}");
    assert_eq!(
        selections.lock().unwrap()[2..],
        [TunnelTransport::Udp, TunnelTransport::Udp]
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn real_loopback_tcp_read_then_write_report_actual_facts() {
    // (peer injects skipped frames) → expected skipped frames before disconnect.
    for (skips, expected) in [(false, 0), (true, 2)] {
        let h = tcp_harness(Box::new(knx_server::RealConnector::default())).await;
        let device = h.device.clone();
        // One tunnel per operation: a read, then a write, each with its own peer.
        let (gateway, worker, trap) = peer::gateway(250, move |p| {
            p.simulate(&device, skips, |_| false);
        });
        let (status, read) =
            send(&h.app, post(READ_TCP, read_body(&h, &gateway.to_string()))).await;
        assert_eq!(status, StatusCode::OK, "{read}");
        let wire = worker.join().unwrap();
        peer::no_retry(&trap);
        let facts = json!({"requestedTransport":"tcp","actualTransport":"tcp",
            "assignedAddress":"1.1.250","skippedFramesBeforeDisconnect":expected});
        assert_eq!(read["connection"], facts);
        assert_eq!(read["individualAddressWriteEnabled"], false);
        assert!(no_writes(&h.device));
        eprintln!(
            "service-control-tcp-receipt read skips={skips} wire={}",
            wire.len()
        );
        if skips {
            capture("read", status, &read);
        }

        let device = h.device.clone();
        let (gateway, worker, trap) = peer::gateway(250, move |p| {
            p.simulate(&device, skips, |_| false);
        });
        let (status, write) = send(
            &h.app,
            post(WRITE_TCP, write_body(&h, &gateway.to_string(), true)),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{write}");
        worker.join().unwrap();
        peer::no_retry(&trap);
        assert_eq!(write["connection"], facts);
        assert_eq!(write["written"], true);
        assert_eq!(write["individualAddressWriteEnabled"], true);
        // The legacy body keys are all still there; `connection` is the only addition.
        let mut keys: Vec<_> = write.as_object().unwrap().keys().cloned().collect();
        keys.sort();
        assert_eq!(
            keys,
            [
                "after",
                "backupPath",
                "before",
                "connection",
                "individualAddressWriteEnabled",
                "written"
            ]
        );
        // The backup bytes carry no transport facts.
        let backup = std::fs::read_to_string(write["backupPath"].as_str().unwrap()).unwrap();
        for word in ["tcp", "transport", "assigned", "1.1.250", "connection"] {
            assert!(!backup.to_lowercase().contains(word), "{word} in {backup}");
        }
        if skips {
            capture("write", status, &write);
        }

        // Recordings: requested transport in the header, sampled facts as a note.
        assert!(h.state.recordings.wait_until_idle(Duration::from_secs(10)));
        let recordings = h.state.recordings.list().unwrap();
        assert_eq!(recordings.len(), 2, "{recordings:?}");
        for recording in &recordings {
            let text = String::from_utf8(h.state.recordings.read(&recording.id).unwrap()).unwrap();
            let lines: Vec<Value> = text
                .lines()
                .map(|l| serde_json::from_str(l).unwrap())
                .collect();
            assert_eq!(
                lines[0]["settings"]["requestedTransport"], "tcp",
                "{}",
                lines[0]
            );
            assert!(
                text.contains("\"assignedAddress\":\"1.1.250\""),
                "no connection note in {text}"
            );
        }
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn peer_eof_mid_read_is_a_failed_run_with_connection_facts() {
    let h = tcp_harness(Box::new(knx_server::RealConnector::default())).await;
    let device = h.device.clone();
    let (gateway, worker, trap) = peer::gateway(250, move |p| {
        p.simulate(&device, true, |frame| {
            matches!(
                frame.service,
                ApplicationService::PropertyValueRead { property_id: 8, .. }
            )
        });
    });
    let (status, failed) = send(&h.app, post(READ_TCP, read_body(&h, &gateway.to_string()))).await;
    worker.join().unwrap();
    peer::no_retry(&trap);
    eprintln!("service-control-tcp-eof {status} {failed}");
    assert_eq!(status, StatusCode::BAD_GATEWAY, "{failed}");
    assert_eq!(failed["kind"], "serviceControlFailed");
    assert!(
        failed["error"].as_str().unwrap().contains("connection was broken down"),
        "{failed}"
    );
    assert_eq!(
        failed["connection"],
        json!({"requestedTransport":"tcp","actualTransport":"tcp",
            "assignedAddress":"1.1.250","skippedFramesBeforeDisconnect":4})
    );
    capture("eof", status, &failed);
    assert!(no_writes(&h.device));
}
