//! Reach strict comparison HTTP handlers with typed selection and real loopback TCP reads.
use super::*;
use knx_net::{TunnelAckWait, TunnelTransport};
use std::net::Ipv4Addr;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
mod peer;

fn binding(h: &Harness) -> Value {
    json!({"serverIncarnation": h.state.server_incarnation,
        "projectIncarnation": h.state.project_incarnation.load(Ordering::Relaxed),
        "deviceId": 0, "comparisonRevision": 0})
}
fn payload(h: &Harness, gateway: &str) -> Value {
    let project = h.state.project.lock().unwrap();
    let id = project.as_ref().unwrap().devices.iter().next().unwrap().id;
    let revision = h
        .state
        .command_stack
        .lock()
        .unwrap()
        .comparison_revision(id);
    let mut binding = binding(h);
    binding["deviceId"] = json!(id.0);
    binding["comparisonRevision"] = json!(revision);
    json!({"address": TARGET, "gateway": gateway, "binding": binding})
}

struct Selected {
    selections: Arc<std::sync::Mutex<Vec<TunnelTransport>>>,
}
impl GatewayConnector for Selected {
    fn connect_tunnel(
        &self,
        _: SocketAddrV4,
    ) -> Pin<Box<dyn Future<Output = Result<Box<dyn BusTunnel>, BusSessionError>> + Send + '_>>
    {
        panic!("typed route must use recorded transport seam")
    }
    fn connect_tunnel_recorded_with_transport(
        &self,
        _: SocketAddrV4,
        _: TunnelAckWait,
        _: Option<knx_net::wire_tap::WireTap>,
        transport: TunnelTransport,
    ) -> Pin<Box<dyn Future<Output = Result<Box<dyn BusTunnel>, BusSessionError>> + Send + '_>>
    {
        self.selections.lock().unwrap().push(transport);
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

#[tokio::test]
async fn strict_sibling_requires_binding_and_refuses_unknown_malformed_busy_stale_without_io() {
    let selections = Arc::new(std::sync::Mutex::new(Vec::new()));
    let h = link_harness_with_connector(Some(Box::new(Selected {
        selections: selections.clone(),
    })))
    .await;
    let valid = payload(&h, GATEWAY);
    for (expected, body) in [
        (422, json!({"address": TARGET, "gateway": GATEWAY})),
        (422, {
            let mut v = valid.clone();
            v["transport"] = json!("tcp");
            v
        }),
        (422, {
            let mut v = valid.clone();
            v["binding"]["extra"] = json!(true);
            v
        }),
        (422, {
            let mut v = valid.clone();
            v["binding"]["deviceId"] = json!("1");
            v
        }),
        (400, {
            let mut v = valid.clone();
            v["gateway"] = json!("not a gateway");
            v
        }),
        (409, {
            let mut v = valid.clone();
            v["binding"]["comparisonRevision"] = json!(999);
            v
        }),
    ] {
        let (status, rb) = send(&h.app, post("/api/device-compare/tcp", body)).await;
        assert_eq!(status.as_u16(), expected, "{rb}");
    }
    assert!(selections.lock().unwrap().is_empty());
    let reservation = h.state.address_programming.lock().await;
    let (status, body) = send(&h.app, post("/api/device-compare/tcp", valid.clone())).await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(body["kind"], "gatewayBusy");
    assert!(selections.lock().unwrap().is_empty());
    drop(reservation);
    for (path, transport) in [
        ("/api/device-compare", TunnelTransport::Udp),
        ("/api/device-compare/tcp", TunnelTransport::Tcp),
    ] {
        let (status, body) = send(&h.app, post(path, valid.clone())).await;
        assert_eq!(status, StatusCode::BAD_GATEWAY, "{body}");
        if transport == TunnelTransport::Tcp {
            assert_eq!(
                body["connection"],
                json!({"requestedTransport":"tcp", "actualTransport":null,
                "assignedAddress":null, "skippedFramesBeforeDisconnect":null})
            );
        } else {
            assert!(body.get("connection").is_none());
        }
    }
    assert_eq!(
        *selections.lock().unwrap(),
        [TunnelTransport::Udp, TunnelTransport::Tcp]
    );
}

async fn http(h: &Harness, body: Value) -> (u16, Value) {
    let listener = tokio::net::TcpListener::bind((Ipv4Addr::LOCALHOST, 0))
        .await
        .unwrap();
    let address = listener.local_addr().unwrap();
    let (stop, stopped) = tokio::sync::oneshot::channel::<()>();
    let app = h.app.clone();
    let server = tokio::spawn(async move {
        axum::serve(listener, app)
            .with_graceful_shutdown(async {
                let _ = stopped.await;
            })
            .await
            .unwrap();
    });
    let mut stream = tokio::net::TcpStream::connect(address).await.unwrap();
    let body = body.to_string();
    stream.write_all(format!("POST /api/device-compare/tcp HTTP/1.1\r\nHost: localhost\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).as_bytes()).await.unwrap();
    let mut response = Vec::new();
    tokio::time::timeout(Duration::from_secs(15), stream.read_to_end(&mut response))
        .await
        .unwrap()
        .unwrap();
    stop.send(()).unwrap();
    server.await.unwrap();
    let response = String::from_utf8(response).unwrap();
    let (headers, body) = response.split_once("\r\n\r\n").unwrap();
    let status = headers.split_whitespace().nth(1).unwrap().parse().unwrap();
    assert!(headers.to_lowercase().contains("cache-control: no-store"));
    (status, serde_json::from_str(body).unwrap())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn real_tcp_http_same_different_and_peer_eof_report_actual_pre_disconnect_skips_without_recording(
) {
    for (different, eof, skips) in [
        (false, false, false),
        (true, false, true),
        (false, true, true),
    ] {
        let h =
            link_harness_with_connector(Some(Box::new(knx_server::RealConnector::default()))).await;
        // Use the public synthetic admitted project, not the private corpus.
        let project = h.state.project.lock().unwrap().clone().unwrap();
        h.state.product_db.as_ref().unwrap().with_read(|products| {
            let prepared = knx_app::device_download::prepare_device_download_for_reading(products, &project, TARGET.parse().unwrap()).unwrap();
            for step in prepared.plan.steps {
                if let knx_core::commissioning::memory_download::MemoryDownloadStep::WriteMemory { address, octets } = step {
                    h.device.preset_memory(u32::from(address), &octets);
                }
            }
            Ok::<(), ()>(())
        }).unwrap().unwrap();
        if different {
            h.device.preset_memory(0x440f, &[1]);
        }
        for machine in [1, 2, 3] {
            h.device
                .preset_load_state(ObjectIndex::new(machine), LoadState::Loaded);
        }
        let device = h.device.clone();
        let (gateway, worker, trap) = peer::gateway(250, move |peer| {
            peer.simulate(&device, skips, |frame| {
                eof && matches!(frame.service, ApplicationService::MemoryRead { .. })
            });
        });
        let (status, body) = http(&h, payload(&h, &gateway.to_string())).await;
        // EOF run: two skips before the first numbered reply plus two sent just before the close.
        let expected_skips = match (skips, eof) {
            (false, _) => 0,
            (true, false) => 2,
            (true, true) => 4,
        };
        let wire = worker.join().unwrap();
        peer::no_retry(&trap);
        // Producer-to-browser contract: the Web fixture replays these exact bodies when asked.
        if let Some(dir) = std::env::var_os("KNX_COMPARE_HTTP_CAPTURE_DIR") {
            let name = if eof {
                "eof"
            } else if different {
                "different"
            } else {
                "same"
            };
            std::fs::write(
                std::path::Path::new(&dir).join(format!("{name}.json")),
                body.to_string(),
            )
            .unwrap();
        }
        eprintln!(
            "comparison-http-receipt {}",
            json!({"different":different,"eof":eof,"status":status,"body":body,"wireFrames":wire.len()})
        );
        assert_eq!(
            body["connection"],
            json!({"requestedTransport":"tcp", "actualTransport":"tcp",
            "assignedAddress":"1.1.250", "skippedFramesBeforeDisconnect": expected_skips})
        );
        if eof {
            assert_eq!(status, 502);
            assert_eq!(body["kind"], "deviceCompareNotComparable");
            assert!(body.get("observationToken").is_none());
            assert!(body.get("same").is_none());
        } else {
            assert_eq!(status, 200, "{body}");
            assert_eq!(body["written"], false);
            assert_eq!(body["same"], !different);
            assert_eq!(body["outdated"], false);
            assert_eq!(body["differingOctets"], if different { 1 } else { 0 });
            assert_eq!(body["observationToken"].as_str().unwrap().len(), 64);
            if different {
                assert_eq!(
                    body["changes"],
                    json!([{"address":0x440f,"segment":"PS","device":[1],"project":[0]}])
                );
            }
        }
        assert!(no_writes(&h.device));
        assert!(!h._dir.path().join("operation-recordings").exists());
    }
}
