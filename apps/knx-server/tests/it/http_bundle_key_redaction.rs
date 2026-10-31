//! DIAG-03 Original14 X1: keys through the real tap never reach a diagnostic bundle entry.
//!
//! The service-control read authorizes with the project's BCU key over a real
//! KNXnet/IP tunnel to the loopback simulator gateway, so the key crosses the
//! wire inside an `A_Authorize_Request` and the production tap must zero it
//! before the recorder sees it. A second, exported Compare recording takes
//! `A_Authorize_Request` and `A_Key_Write` datagrams in both directions through
//! the same tap (no production path sends `A_Key_Write`). Both recordings leave
//! raw, so every ZIP entry is scanned for each key in every encoding the
//! sentinel scanner knows. No real gateway, bus or device is involved.

use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use knx_app::operation_recording::{OperationKind, OperationStart, RecordedOutcome};
use knx_core::IndividualAddress;
use knx_net::commissioning::simulator::{SimulatedDevice, SimulatorConfig};
use knx_net::commissioning::simulator_gateway::{serve, GatewayConfig, GatewayEvent};
use knx_net::wire_tap::WireDirection;
use serde_json::{json, Value};
use std::io::Read;
use tokio::net::UdpSocket;
use tower::ServiceExt;

#[path = "../../../../crates/knx-app/src/sentinel_forms_tests.rs"]
mod sentinel_forms;

/// The project's BCU key, as the route sends it (big-endian octets).
const PROJECT_KEY: u32 = 0x5EC2_E7F1;
/// Keys only ever handed to the tap directly, one per APDU kind and direction.
const TAP_KEYS: [[u8; 4]; 4] = [
    [0xA1, 0x7B, 0x3C, 0x94],
    [0xB2, 0x8C, 0x4D, 0xA5],
    [0xC3, 0x9D, 0x5E, 0xB6],
    [0xD4, 0xAE, 0x6F, 0xC7],
];

type Seen = Arc<Mutex<Vec<(bool, Vec<u8>)>>>;

async fn loopback_gateway() -> (SocketAddrV4, Seen, Arc<SimulatedDevice>) {
    let socket = UdpSocket::bind((Ipv4Addr::LOCALHOST, 0)).await.unwrap();
    let SocketAddr::V4(endpoint) = socket.local_addr().unwrap() else {
        unreachable!("bound IPv4")
    };
    // A free level below maximum, so DM_Authorize2 really sends the key.
    let device = Arc::new(SimulatedDevice::with_config(SimulatorConfig {
        free_access_level: 3,
        key: Some(PROJECT_KEY),
        key_level: 1,
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

async fn call(
    app: &axum::Router,
    method: &str,
    uri: &str,
    body: Option<Value>,
) -> (StatusCode, Vec<u8>) {
    let request = Request::builder()
        .method(method)
        .uri(uri)
        .header("content-type", "application/json")
        .body(body.map_or_else(Body::empty, |b| Body::from(b.to_string())))
        .unwrap();
    let response = app.clone().oneshot(request).await.unwrap();
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap()
        .to_vec();
    (status, bytes)
}

fn json_of(bytes: &[u8]) -> Value {
    serde_json::from_slice(bytes)
        .unwrap_or_else(|_| Value::String(String::from_utf8_lossy(bytes).into_owned()))
}

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    haystack.windows(needle.len()).any(|w| w == needle)
}

fn bcu_key_entry() -> knx_store::StoredOpaqueEntry {
    let bytes = PROJECT_KEY.to_string().into_bytes();
    knx_store::StoredOpaqueEntry {
        source_path: "P-0001/0.xml".into(),
        xpath: "/KNX/Project/Installations/Installation".into(),
        kind: "RetainedAttribute".into(),
        name: "BCUKey".into(),
        sha256: "0".repeat(64),
        bytes,
    }
}

/// A tunnelling request carrying `apci` (authorize or key write), a level/reserved
/// octet and `key`, as a tool or another client would put it on the wire.
fn keyed_datagram(apci: u16, key: [u8; 4], channel: u8) -> Vec<u8> {
    let mut cemi = vec![0x29, 0x00, 0xBC, 0x60, 0x11, 0x05, 0x11, 0xFE, 0x06];
    cemi.push(0x40 | ((apci >> 8) as u8 & 0x03));
    cemi.push(apci as u8);
    cemi.push(0x00);
    cemi.extend_from_slice(&key);
    knx_net::frame::encode_frame(
        knx_net::tunnelling::TUNNELLING_REQUEST,
        &knx_net::tunnelling::encode_tunnelling_request(channel, 0, &cemi),
    )
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn keys_sent_and_received_through_the_real_tap_never_reach_a_bundle_entry() {
    let dir = tempfile::tempdir().unwrap();
    let (endpoint, seen, device) = loopback_gateway().await;
    let state = Arc::new(knx_server::AppState {
        recordings: Arc::new(knx_server::operation_recordings::store(dir.path(), false)),
        connector: Box::new(knx_server::RealConnector::default()),
        product_db: None,
        ..knx_server::AppState::new(dir.path().to_path_buf())
    });
    *state.opaque.lock().unwrap() = vec![bcu_key_entry()];
    let app = knx_server::app(Arc::clone(&state), None);
    let (status, body) = call(
        &app,
        "PUT",
        "/api/settings",
        Some(json!({"settings": {"debugIndividualAddressWriteEnable": true}})),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{}", json_of(&body));

    // 1. The real route: the key crosses the loopback wire, the tap zeroes it.
    let (status, body) = call(
        &app,
        "GET",
        &format!(
            "/api/device/service-control?address={}&gateway={endpoint}",
            device.address()
        ),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{}", json_of(&body));
    assert!(state.recordings.wait_until_idle(Duration::from_secs(10)));
    let project_key = PROJECT_KEY.to_be_bytes();
    assert!(
        seen.lock()
            .unwrap()
            .iter()
            .any(|(from_tool, bytes)| *from_tool && contains(bytes, &project_key)),
        "precondition: the key really went over the wire"
    );
    let routed = state.recordings.list().unwrap();
    assert_eq!(routed.len(), 1, "{routed:?}");
    let routed = routed[0].id.clone();
    let text = String::from_utf8(state.recordings.read(&routed).unwrap()).unwrap();
    assert!(
        text.contains("authorize-key"),
        "precondition: the recording names the redaction"
    );

    // 2. Both secret APDUs, both directions, through the same production tap.
    std::thread::sleep(Duration::from_millis(3));
    let recorder = state.recordings.start(
        OperationStart::new(OperationKind::Compare)
            .target(json!({"individualAddress": device.address().to_string()}))
            .gateway(endpoint),
    );
    let tap = recorder.wire_tap().unwrap();
    for (index, key) in TAP_KEYS.iter().enumerate() {
        let apci = if index % 2 == 0 {
            knx_net::cemi::APCI_AUTHORIZE_REQUEST
        } else {
            knx_net::cemi::APCI_KEY_WRITE
        };
        let direction = if index < 2 {
            WireDirection::Sent
        } else {
            WireDirection::Received
        };
        tap.datagram(direction, &keyed_datagram(apci, *key, index as u8 + 1));
    }
    recorder.finish(RecordedOutcome::succeeded(Value::Null));
    assert!(state.recordings.wait_until_idle(Duration::from_secs(10)));
    let tapped = recorder.id().unwrap().to_string();
    let tapped_text = String::from_utf8(state.recordings.read(&tapped).unwrap()).unwrap();
    assert_eq!(tapped_text.matches("authorize-key").count(), 2);
    assert_eq!(tapped_text.matches("key-write-key").count(), 2);

    // 3. Export both raw, and scan everything that leaves.
    let mut query = json!({
        "problemType": "deviceProgramming",
        "operation": {"recordingId": tapped},
        "earlierRecordingIds": [routed],
        "sections": ["operation", "earlierOperation"],
    });
    let (status, preview) = call(
        &app,
        "POST",
        "/api/diagnostics/bundle/preview",
        Some(query.clone()),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{}", json_of(&preview));
    let mut keys: Vec<&[u8]> = TAP_KEYS.iter().map(|key| key.as_slice()).collect();
    keys.push(&project_key);
    sentinel_forms::assert_no_sentinel_bytes("preview", &preview, &keys);
    query["review"] = json_of(&preview)["review"].clone();
    query["consent"] = json!(true);
    let (status, zip_bytes) = call(&app, "POST", "/api/diagnostics/bundle", Some(query)).await;
    assert_eq!(status, StatusCode::OK, "{}", json_of(&zip_bytes));
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(zip_bytes)).unwrap();
    let mut raw_recordings = 0;
    for index in 0..archive.len() {
        let mut entry = archive.by_index(index).unwrap();
        let name = entry.name().to_owned();
        let mut content = Vec::new();
        entry.read_to_end(&mut content).unwrap();
        if name.ends_with("recording.jsonl") {
            raw_recordings += 1;
            let id = name.split('/').nth(1).unwrap();
            assert_eq!(
                content,
                state.recordings.read(id).unwrap(),
                "{name} is the recorder's own file"
            );
        }
        sentinel_forms::assert_no_sentinel_bytes(&format!("entry {name}"), &content, &keys);
    }
    assert_eq!(
        raw_recordings, 2,
        "both recordings left raw, so the scan saw them"
    );
}
