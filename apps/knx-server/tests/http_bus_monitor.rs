//! `POST /api/bus/monitor/start`, `POST /api/bus/monitor/stop`,
//! `GET /api/bus/monitor/telegrams` — design spec `docs/superpowers/specs/
//! 2026-09-11-group-monitor-design.md` §4.3, tests enumerated at §3 D6
//! (six numbered cases, referenced by number in each test's own doc
//! comment below), plus the extra coverage the brief for this task calls
//! out by name: the `502` path, the `404`-vs-`409` split for `/telegrams`,
//! a `since` below the buffer floor, and a decode test against a real
//! (not `None`) project.
//!
//! `AppState { connector: Box::new(FakeConnector::...), ..Default::default() }`
//! is the injection pattern `apps/knx-server/tests/http_product_install.rs`
//! already uses for `product_db` (design spec §3 D6's own "Wiring"
//! paragraph names this exact precedent). No gateway, no socket, anywhere
//! in this file.

use std::path::PathBuf;
use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use serde_json::{json, Value};
use tower::ServiceExt;

use knx_core::{
    ComObjectInstance, ComObjectInstanceId, Direction, GroupAddress, GroupAddressEntry,
    GroupAddressId, GroupLink, GroupValue, Installation, InstallationId, Language, Layer, Override,
    Resolved, ResolvedFlags, SourceRef,
};
use knx_net::{ApplicationService, Destination};
use knx_server::fake::{FakeConnector, FakeTunnel, FakeTunnelHandle};

fn addr(device: u8) -> knx_core::IndividualAddress {
    knx_core::IndividualAddress::new(1, 1, device).expect("valid test address")
}

/// A fake tunnel with a generous broadcast capacity — the default for every
/// test that is not specifically trying to force a lag/overflow.
fn fake_tunnel() -> (FakeTunnel, FakeTunnelHandle) {
    FakeTunnel::new(addr(5), 64)
}

fn small_fake_tunnel(capacity: usize) -> (FakeTunnel, FakeTunnelHandle) {
    FakeTunnel::new(addr(5), capacity)
}

fn telegram(destination: Destination, service: ApplicationService) -> knx_net::TunnelEvent {
    knx_net::TunnelEvent::Telegram(knx_net::LDataFrame {
        kind: knx_net::LDataMessageKind::Indication,
        source: addr(9),
        destination,
        transport: knx_net::Tpci::UnnumberedData,
        service,
        control: None,
    })
}

fn group_value_write(raw: u16, value: GroupValue) -> knx_net::TunnelEvent {
    telegram(
        Destination::Group(GroupAddress::from_raw(raw)),
        ApplicationService::GroupValueWrite(value),
    )
}

async fn body_json(response: axum::response::Response) -> Value {
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

async fn call(
    app: &axum::Router,
    method: &str,
    uri: &str,
    body: Option<Value>,
) -> axum::response::Response {
    let mut builder = Request::builder().method(method).uri(uri);
    let body = match body {
        Some(v) => {
            builder = builder.header("content-type", "application/json");
            Body::from(v.to_string())
        }
        None => Body::empty(),
    };
    app.clone()
        .oneshot(builder.body(body).unwrap())
        .await
        .unwrap()
}

fn state_with_connector(connector: FakeConnector) -> knx_server::AppState {
    knx_server::AppState {
        connector: Box::new(connector),
        ..Default::default()
    }
}

/// A minimal, real `knx_core::Project` — not `None` — with one group
/// address that resolves to a single DPT and one that has no linked
/// communication object at all (mirrors `bus.rs`'s own
/// `project_with_group_addresses` fixture, since this crate has no `lib`
/// target for that unit-test helper to be shared from — same reasoning
/// `bus.rs`'s `GroupAddressContext::decode` doc comment gives for not
/// sharing `format_decoded_value` from the CLI). Group address `1`
/// resolves to `DPST-1-1` (boolean); group address `2` has no linked
/// communication object, so it decodes as `unresolved`.
fn project_with_resolving_and_unresolving_group_addresses() -> knx_core::Project {
    let source = SourceRef {
        path: "t".into(),
        ets_id: "t".into(),
    };
    let mut project = knx_core::Project::new(Language("en".into()));
    project.devices.insert_com_object(ComObjectInstance {
        id: ComObjectInstanceId(1),
        source: source.clone(),
        device: knx_core::DeviceId(1),
        number: 0,
        text: Override::Absent,
        description: Override::Absent,
        dpt: Override::Value(Resolved {
            value: knx_core::DptRef {
                main: 1,
                sub: Some(1),
            },
            layer: Layer::Instance,
        }),
        flags: ResolvedFlags::none(),
        size: None,
        is_active: true,
        links: vec![GroupLink {
            ga: GroupAddressId(1),
            direction: Direction::Send,
        }],
        module_instance: None,
    });
    project.installations.push(Installation {
        id: InstallationId(1),
        name: "I".into(),
        default_line: None,
        multicast_address: None,
        completion: knx_core::CompletionStatus::FinishedDesign,
        topology: knx_core::Topology {
            areas: vec![],
            lines: vec![],
            unassigned: vec![],
        },
        buildings: vec![],
        group_ranges: vec![],
        group_addresses: vec![
            GroupAddressEntry {
                id: GroupAddressId(1),
                source: source.clone(),
                name: "Living room / light / switch".into(),
                address: GroupAddress::from_raw(1),
                central: false,
                unfiltered: false,
                range: None,
                declared_dpt: Default::default(),
            },
            GroupAddressEntry {
                id: GroupAddressId(2),
                source,
                name: "Unlinked".into(),
                address: GroupAddress::from_raw(2),
                central: false,
                unfiltered: false,
                range: None,
                declared_dpt: Default::default(),
            },
        ],
        parameters: vec![],
    });
    project
}

// ---------------------------------------------------------------------------
// D6 test 1 — start opens a session via a FakeConnector that always
// succeeds; response shape and `bus_session` state.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn start_opens_a_session_and_returns_session_id_and_assigned_address() {
    let (tunnel, _handle) = fake_tunnel();
    let state = state_with_connector(FakeConnector::succeeding(tunnel));
    let app = knx_server::app(Arc::new(state), None);

    let response = call(
        &app,
        "POST",
        "/api/bus/monitor/start",
        Some(json!({ "gateway": "192.0.2.10:3671" })),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let body = body_json(response).await;
    assert_eq!(body["sessionId"], 1);
    assert_eq!(body["assignedAddress"], addr(5).to_string());
    let incarnation = body["serverIncarnation"]
        .as_str()
        .expect("start response carries its server process identity");
    assert!(!incarnation.is_empty());

    let activity = body_json(call(&app, "GET", "/api/bus/activity", None).await).await;
    assert_eq!(activity["serverIncarnation"], incarnation);
    assert_eq!(activity["sessions"][0]["kind"], "busMonitor");
    assert_eq!(activity["sessions"][0]["id"], 1);
    assert_eq!(activity["sessions"][0]["state"], "active");
    assert!(activity["sessions"][0].get("gateway").is_none());

    let response = call(&app, "GET", "/api/bus/monitor/telegrams?since=0", None).await;
    assert_eq!(response.status(), StatusCode::OK);
    let body = body_json(response).await;
    assert_eq!(body["sessionId"], 1);
    assert_eq!(body["serverIncarnation"], incarnation);
}

#[tokio::test]
async fn monitor_context_compares_the_actual_server_project_not_a_browser_record() {
    let (tunnel, _handle) = fake_tunnel();
    let mut state = state_with_connector(FakeConnector::succeeding(tunnel));
    state.product_db = None;
    *state.project.lock().unwrap() = Some(project_with_resolving_and_unresolving_group_addresses());
    let state = Arc::new(state);
    let app = knx_server::app(state.clone(), None);
    let started = call(
        &app,
        "POST",
        "/api/bus/monitor/start",
        Some(json!({"gateway":"192.0.2.10:3671"})),
    )
    .await;
    assert_eq!(started.status(), StatusCode::OK);
    let initial = body_json(call(&app, "GET", "/api/bus/monitor/telegrams", None).await).await;
    assert_eq!(initial["contextStatus"], "current");
    assert_eq!(initial["projectOpen"], true);

    state
        .project
        .lock()
        .unwrap()
        .as_mut()
        .unwrap()
        .installations[0]
        .group_addresses[0]
        .name = "Changed by another client".into();
    let changed = body_json(call(&app, "GET", "/api/bus/monitor/telegrams", None).await).await;
    assert_eq!(changed["contextStatus"], "stale");
    assert_eq!(changed["sessionId"], initial["sessionId"]);
    assert_eq!(changed["serverIncarnation"], initial["serverIncarnation"]);
    assert_eq!(changed["projectOpen"], true);
}

#[tokio::test]
async fn context_comparison_covers_dpts_links_style_project_close_and_irrelevant_edits() {
    for (change, expected) in [
        ("same", "current"),
        ("installationName", "current"),
        ("dpt", "stale"),
        ("links", "stale"),
        ("style", "stale"),
        ("close", "stale"),
    ] {
        let (tunnel, _handle) = fake_tunnel();
        let mut state = state_with_connector(FakeConnector::succeeding(tunnel));
        state.product_db = None;
        *state.project.lock().unwrap() =
            Some(project_with_resolving_and_unresolving_group_addresses());
        let state = Arc::new(state);
        let app = knx_server::app(state.clone(), None);
        assert_eq!(
            call(
                &app,
                "POST",
                "/api/bus/monitor/start",
                Some(json!({"gateway":"192.0.2.10:3671"}))
            )
            .await
            .status(),
            StatusCode::OK
        );
        let mut project = project_with_resolving_and_unresolving_group_addresses();
        match change {
            "installationName" => project.installations[0].name = "No decode effect".into(),
            "dpt" => {
                project
                    .devices
                    .com_object_mut(ComObjectInstanceId(1))
                    .unwrap()
                    .dpt = Override::Value(Resolved {
                    value: knx_core::DptRef {
                        main: 9,
                        sub: Some(1),
                    },
                    layer: Layer::Instance,
                })
            }
            "links" => project
                .devices
                .com_object_mut(ComObjectInstanceId(1))
                .unwrap()
                .links
                .clear(),
            "style" => project.info.group_address_style = knx_core::GroupAddressStyle::Free,
            _ => (),
        }
        *state.project.lock().unwrap() = (change != "close").then_some(project);
        let response = body_json(call(&app, "GET", "/api/bus/monitor/telegrams", None).await).await;
        assert_eq!(response["contextStatus"], expected, "{change}");
        assert_eq!(response["projectOpen"], change != "close", "{change}");
    }
}

#[tokio::test]
async fn a_busy_project_is_unavailable_not_fresh_and_does_not_block_the_poll() {
    let (tunnel, _handle) = fake_tunnel();
    let mut state = state_with_connector(FakeConnector::succeeding(tunnel));
    state.product_db = None;
    let state = Arc::new(state);
    let app = knx_server::app(state.clone(), None);
    assert_eq!(
        call(
            &app,
            "POST",
            "/api/bus/monitor/start",
            Some(json!({"gateway":"192.0.2.10:3671"}))
        )
        .await
        .status(),
        StatusCode::OK
    );
    let (ready_tx, ready_rx) = std::sync::mpsc::channel();
    let (release_tx, release_rx) = std::sync::mpsc::channel();
    let worker = std::thread::spawn(move || {
        let _guard = state.project.lock().unwrap();
        ready_tx.send(()).unwrap();
        release_rx.recv().unwrap();
    });
    ready_rx.recv().unwrap();
    let response = tokio::time::timeout(
        std::time::Duration::from_secs(1),
        call(&app, "GET", "/api/bus/monitor/telegrams", None),
    )
    .await;
    release_tx.send(()).unwrap();
    worker.join().unwrap();
    let response = body_json(response.expect("poll must not wait for the project")).await;
    assert_eq!(response["contextStatus"], "unavailable");
    assert_eq!(response["projectOpen"], Value::Null);
}

#[tokio::test]
async fn context_only_poll_does_not_return_buffered_rows_or_move_the_client_cursor() {
    let (tunnel, handle) = fake_tunnel();
    let mut state = state_with_connector(FakeConnector::succeeding(tunnel));
    state.product_db = None;
    let app = knx_server::app(Arc::new(state), None);
    assert_eq!(
        call(
            &app,
            "POST",
            "/api/bus/monitor/start",
            Some(json!({"gateway":"192.0.2.10:3671"}))
        )
        .await
        .status(),
        StatusCode::OK
    );
    handle
        .sender()
        .send(group_value_write(1, GroupValue::Short(1)))
        .unwrap();
    let complete = tokio::time::timeout(std::time::Duration::from_secs(1), async {
        loop {
            let body =
                body_json(call(&app, "GET", "/api/bus/monitor/telegrams?since=0", None).await)
                    .await;
            if body["telegrams"].as_array().unwrap().len() == 1 {
                break body;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    let paused = body_json(
        call(
            &app,
            "GET",
            "/api/bus/monitor/telegrams?since=0&contextOnly=true",
            None,
        )
        .await,
    )
    .await;
    assert_eq!(paused["telegrams"], json!([]));
    assert_eq!(paused["nextSince"], 0);
    assert_eq!(paused["contextStatus"], "current");
    assert_eq!(paused["projectOpen"], false);
    assert_eq!(paused["sessionId"], complete["sessionId"]);
    let resumed =
        body_json(call(&app, "GET", "/api/bus/monitor/telegrams?since=0", None).await).await;
    // `observedAgeMs` is measured per response (TELEGRAM_FLOW_VISUALIZATION
    // §10), so a millisecond tick between the two polls changes it; the
    // rows must otherwise be identical and the age may only grow.
    let age = |body: &Value| body["telegrams"][0]["observedAgeMs"].as_u64().unwrap();
    assert!(age(&resumed) >= age(&complete));
    let without_age = |body: &Value| {
        let mut rows = body["telegrams"].clone();
        for row in rows.as_array_mut().unwrap() {
            row.as_object_mut().unwrap().remove("observedAgeMs");
        }
        rows
    };
    assert_eq!(without_age(&resumed), without_age(&complete));
    assert_eq!(resumed["nextSince"], 1);
}

#[tokio::test]
async fn an_unparsable_gateway_address_is_a_bad_request() {
    let (tunnel, _handle) = fake_tunnel();
    let state = state_with_connector(FakeConnector::succeeding(tunnel));
    let app = knx_server::app(Arc::new(state), None);

    let response = call(
        &app,
        "POST",
        "/api/bus/monitor/start",
        Some(json!({ "gateway": "not-an-address" })),
    )
    .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

/// The `502` path this task's brief calls out by name: the connector
/// reports a `BusSessionError::Transport`, e.g. a real gateway timing out
/// or refusing the connection — surfaced as `502`, never `400`/`500`.
#[tokio::test]
async fn a_connector_transport_failure_is_reported_as_a_bad_gateway() {
    let state = state_with_connector(FakeConnector::failing(
        knx_server::BusSessionError::Transport(knx_net::BusError::Timeout),
    ));
    let app = knx_server::app(Arc::new(state), None);

    let response = call(
        &app,
        "POST",
        "/api/bus/monitor/start",
        Some(json!({ "gateway": "192.0.2.10:3671" })),
    )
    .await;
    assert_eq!(response.status(), StatusCode::BAD_GATEWAY);
    let body = body_json(response).await;
    assert!(
        body["error"].as_str().unwrap().contains("timed out"),
        "expected BusError::Timeout's own Display text, got: {body}"
    );
}

// ---------------------------------------------------------------------------
// D6 test 2 — a second start while one is active is 409, names the
// existing session, and never touches its tunnel.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn a_second_start_while_active_is_a_conflict_naming_the_existing_session() {
    let (tunnel, handle) = fake_tunnel();
    // Only one scripted outcome: if the handler incorrectly reached the
    // connector a second time, `FakeConnector` would panic on the second
    // call (no outcome left to hand back) — the test does not need a
    // separate call-count assertion for that to be caught.
    let state = state_with_connector(FakeConnector::succeeding(tunnel));
    let app = knx_server::app(Arc::new(state), None);

    let first = call(
        &app,
        "POST",
        "/api/bus/monitor/start",
        Some(json!({ "gateway": "192.0.2.10:3671" })),
    )
    .await;
    assert_eq!(first.status(), StatusCode::OK);

    let second = call(
        &app,
        "POST",
        "/api/bus/monitor/start",
        Some(json!({ "gateway": "203.0.113.5:3671" })),
    )
    .await;
    assert_eq!(second.status(), StatusCode::CONFLICT);
    let body = body_json(second).await;
    let message = body["error"].as_str().unwrap();
    assert!(
        message.contains('1'),
        "expected the existing session's id (1) named: {message}"
    );
    assert!(
        message.contains("192.0.2.10:3671"),
        "expected the existing session's gateway named: {message}"
    );

    // The original tunnel is untouched by the rejected second attempt.
    assert!(!handle.disconnected());
}

// ---------------------------------------------------------------------------
// D6 test 3 — telegrams fed through the fake's sender are decoded against
// a real project and returned by a poll. This is also this task's
// "decode test with a real project fixture" (brief item 6): one address
// resolves to a value, one does not.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn polling_returns_telegrams_decoded_against_the_open_project() {
    let (tunnel, handle) = fake_tunnel();
    let project = project_with_resolving_and_unresolving_group_addresses();
    let state = knx_server::AppState {
        connector: Box::new(FakeConnector::succeeding(tunnel)),
        project: std::sync::Mutex::new(Some(project)),
        ..Default::default()
    };
    let app = knx_server::app(Arc::new(state), None);

    let start = call(
        &app,
        "POST",
        "/api/bus/monitor/start",
        Some(json!({ "gateway": "192.0.2.10:3671" })),
    )
    .await;
    assert_eq!(start.status(), StatusCode::OK);

    handle
        .sender()
        .send(group_value_write(1, GroupValue::Short(1)))
        .unwrap();
    handle
        .sender()
        .send(group_value_write(2, GroupValue::Short(1)))
        .unwrap();

    let telegrams = poll_until_len(&app, 2).await;
    assert_eq!(telegrams["sessionId"], 1);
    assert_eq!(telegrams["status"], "active");
    assert_eq!(telegrams["droppedBefore"], 0);

    let rows = telegrams["telegrams"].as_array().unwrap();
    assert_eq!(rows.len(), 2);

    // Group address 1 — resolves to DPST-1-1, decodes as a value. Formatted
    // with the project's default `GroupAddressStyle::ThreeLevel`
    // (`ProjectInfo::default`): raw `1` is main 0 / middle 0 / sub 1.
    assert_eq!(rows[0]["destination"], "0/0/1");
    assert_eq!(rows[0]["destinationName"], "Living room / light / switch");
    assert_eq!(rows[0]["decoded"]["kind"], "value");
    assert_eq!(rows[0]["decoded"]["dpt"], "DPST-1-1");
    assert_eq!(rows[0]["decoded"]["text"], "on");

    // Group address 2 — no linked communication object, decodes as
    // unresolved, never guessed.
    assert_eq!(rows[1]["destination"], "0/0/2");
    assert_eq!(rows[1]["decoded"]["kind"], "unresolved");
    assert_eq!(
        rows[1]["decoded"]["text"],
        "no DPT resolved for this group address"
    );
}

/// ADR-0078: an address with no linked object but its own declared DPT is
/// decoded with that declaration instead of being reported as unresolved.
#[tokio::test]
async fn a_declared_group_address_dpt_decodes_an_otherwise_unresolved_address() {
    let (tunnel, handle) = fake_tunnel();
    let mut project = project_with_resolving_and_unresolving_group_addresses();
    project.installations[0].group_addresses[1].declared_dpt =
        knx_core::Override::Value(knx_core::Resolved {
            value: knx_core::DptRef {
                main: 1,
                sub: Some(1),
            },
            layer: knx_core::Layer::Instance,
        });
    let state = knx_server::AppState {
        connector: Box::new(FakeConnector::succeeding(tunnel)),
        project: std::sync::Mutex::new(Some(project)),
        ..Default::default()
    };
    let app = knx_server::app(Arc::new(state), None);
    let start = call(
        &app,
        "POST",
        "/api/bus/monitor/start",
        Some(json!({ "gateway": "192.0.2.10:3671" })),
    )
    .await;
    assert_eq!(start.status(), StatusCode::OK);
    handle
        .sender()
        .send(group_value_write(2, GroupValue::Short(1)))
        .unwrap();
    let telegrams = poll_until_len(&app, 1).await;
    let row = &telegrams["telegrams"][0];
    assert_eq!(row["destination"], "0/0/2");
    assert_eq!(row["decoded"]["kind"], "value");
    assert_eq!(row["decoded"]["dpt"], "DPST-1-1");
    assert_eq!(row["decoded"]["text"], "on");
}

async fn poll_until_len(app: &axum::Router, len: usize) -> Value {
    for _ in 0..200 {
        let response = call(app, "GET", "/api/bus/monitor/telegrams?since=0", None).await;
        assert_eq!(response.status(), StatusCode::OK);
        let body = body_json(response).await;
        if body["telegrams"].as_array().unwrap().len() >= len {
            return body;
        }
        tokio::time::sleep(std::time::Duration::from_millis(2)).await;
    }
    panic!("telegrams never reached length {len}");
}

// ---------------------------------------------------------------------------
// D6 test 4 — RecvError::Lagged accounting surfaces as a non-zero
// droppedBefore, plus this task's own "since below the floor" case.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn a_forced_lag_is_reported_as_a_nonzero_dropped_before() {
    // Capacity 2, 10 sends: the third send before anything drains forces a
    // real `RecvError::Lagged` on the session's own broadcast receiver.
    const CAPACITY: usize = 2;
    const SENT: u16 = 10;
    let (tunnel, handle) = small_fake_tunnel(CAPACITY);
    let state = state_with_connector(FakeConnector::succeeding(tunnel));
    let app = knx_server::app(Arc::new(state), None);

    // `#[tokio::test]` defaults to the `current_thread` flavor, and
    // neither `call()` (a bare `tower::ServiceExt::oneshot`, no spawned
    // task) nor `POST /api/bus/monitor/start`'s own handler chain
    // (`FakeConnector::connect_tunnel`'s `Box::pin` resolves on its first
    // poll with no real await inside it; `state.bus_session`'s
    // `tokio::sync::Mutex` is uncontended; `app()` adds no middleware
    // layers) ever actually yields to the executor. So by the time this
    // `.await` returns, the drain task has been *spawned* but never
    // *polled* — cooperative scheduling guarantees that, exactly as
    // `bus.rs`'s `session_receiver_lag_is_accounted_exactly_in_dropped_before`
    // (`current_thread` flavor, explicit) relies on for the unit-level
    // case this test mirrors at the HTTP layer.
    call(
        &app,
        "POST",
        "/api/bus/monitor/start",
        Some(json!({ "gateway": "192.0.2.10:3671" })),
    )
    .await;

    // Flood past the channel capacity before the drain task gets a chance
    // to read any of them — this loop is synchronous (`Sender::send` is
    // not `async`), so it cannot yield either. All `SENT` sends land
    // before the drain task's receiver is ever polled.
    for i in 0..SENT {
        let _ = handle
            .sender()
            .send(group_value_write(i, GroupValue::Short(1)));
    }

    // Exact count, not `> 0`: once the drain task's receiver is finally
    // polled, `tokio::sync::broadcast` reports the *entire* backlog it
    // missed as a single `RecvError::Lagged(SENT - CAPACITY)` on its first
    // `recv()` — not incrementally, and no further messages are sent
    // after the flood above, so `droppedBefore` jumps straight to its
    // final value and never changes again. The polling loop below only
    // has to find the first non-zero read; that value already equals the
    // exact final one. `bus.rs`'s
    // `session_receiver_lag_is_accounted_exactly_in_dropped_before` proves
    // the same formula directly against `BusSession`, without going
    // through HTTP; this test is the HTTP-layer half design spec §8
    // criterion 4 asks for.
    let expected_dropped = u64::from(SENT) - CAPACITY as u64;
    let mut dropped_before = 0u64;
    for _ in 0..200 {
        let response = call(&app, "GET", "/api/bus/monitor/telegrams?since=0", None).await;
        let body = body_json(response).await;
        dropped_before = body["droppedBefore"].as_u64().unwrap();
        if dropped_before > 0 {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(2)).await;
    }
    assert_eq!(
        dropped_before, expected_dropped,
        "expected a forced Lagged(n) to advance droppedBefore by exactly \
         SENT - CAPACITY"
    );
}

/// This task's own extra case: a `since` value below the buffer's current
/// floor (every row with that `seq` already evicted, or simply never
/// existed) is not an error — the response still comes back `200` with
/// whatever rows remain, and `droppedBefore` explains the gap.
#[tokio::test]
async fn a_since_value_below_the_buffer_floor_is_not_an_error() {
    let (tunnel, handle) = fake_tunnel();
    let state = state_with_connector(FakeConnector::succeeding(tunnel));
    let app = knx_server::app(Arc::new(state), None);

    call(
        &app,
        "POST",
        "/api/bus/monitor/start",
        Some(json!({ "gateway": "192.0.2.10:3671" })),
    )
    .await;
    handle
        .sender()
        .send(group_value_write(1, GroupValue::Short(1)))
        .unwrap();
    poll_until_len(&app, 1).await;

    // `since=999999` is far past anything this buffer ever held.
    let response = call(&app, "GET", "/api/bus/monitor/telegrams?since=999999", None).await;
    assert_eq!(response.status(), StatusCode::OK);
    let body = body_json(response).await;
    assert_eq!(body["telegrams"].as_array().unwrap().len(), 0);
    // Not evicted in this small test (`droppedBefore` still 0) — the point
    // of this test is the status code and shape, not eviction itself
    // (covered by `bus.rs`'s own eviction-accounting unit test).
    assert_eq!(body["droppedBefore"], 0);
}

// ---------------------------------------------------------------------------
// D6 test 5 — a gateway-side Closed transitions the session to "closed",
// still readable, until /stop is called.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn a_gateway_close_transitions_status_to_closed_but_stays_readable() {
    let (tunnel, handle) = fake_tunnel();
    let state = state_with_connector(FakeConnector::succeeding(tunnel));
    let app = knx_server::app(Arc::new(state), None);

    call(
        &app,
        "POST",
        "/api/bus/monitor/start",
        Some(json!({ "gateway": "192.0.2.10:3671" })),
    )
    .await;
    handle
        .sender()
        .send(group_value_write(1, GroupValue::Short(1)))
        .unwrap();
    handle.sender().send(knx_net::TunnelEvent::Closed).unwrap();

    let mut status = String::new();
    for _ in 0..200 {
        let response = call(&app, "GET", "/api/bus/monitor/telegrams?since=0", None).await;
        let body = body_json(response).await;
        status = body["status"].as_str().unwrap().to_string();
        if status == "closed" {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(2)).await;
    }
    assert_eq!(status, "closed");

    // Still readable after the close — `bus_session` itself is untouched
    // until `/stop` is actually called.
    let response = call(&app, "GET", "/api/bus/monitor/telegrams?since=0", None).await;
    assert_eq!(response.status(), StatusCode::OK);

    // KNOWN_LIMITATIONS §147: a real telegram's row carries the control
    // fields it arrived with (this fake frame has none, so the decoder's
    // defaults); the closed-session marker was never a frame and has none.
    let rows = body_json(response).await["telegrams"].clone();
    assert_eq!(
        rows[0]["control"],
        json!({ "priority": "low", "repeated": false, "hopCount": 6 })
    );
    assert_eq!(rows[1]["service"], "SessionClosed");
    assert_eq!(rows[1]["control"], Value::Null);
}

// ---------------------------------------------------------------------------
// stop: awaits the drain task, 409 with no session, reports a panic as a
// warning rather than discarding it (carried Task 2 finding).
// ---------------------------------------------------------------------------

#[tokio::test]
async fn stop_awaits_the_drain_task_and_reports_the_final_tally() {
    let (tunnel, handle) = fake_tunnel();
    let state = state_with_connector(FakeConnector::succeeding(tunnel));
    let app = knx_server::app(Arc::new(state), None);

    let started = call(
        &app,
        "POST",
        "/api/bus/monitor/start",
        Some(json!({ "gateway": "192.0.2.10:3671" })),
    )
    .await;
    let started = body_json(started).await;
    handle
        .sender()
        .send(group_value_write(1, GroupValue::Short(1)))
        .unwrap();
    poll_until_len(&app, 1).await;

    let response = call(&app, "POST", "/api/bus/monitor/stop", None).await;
    assert_eq!(response.status(), StatusCode::OK);
    let body = body_json(response).await;
    assert_eq!(body["sessionId"], 1);
    assert_eq!(body["serverIncarnation"], started["serverIncarnation"]);
    assert_eq!(body["telegramCount"], 1);
    assert_eq!(body["droppedCount"], 0);
    assert!(body.get("warning").is_none());

    // The stop only returned after the drain task actually disconnected —
    // not fire-and-forget (design spec §4.1, acceptance criterion 5).
    assert!(handle.disconnected());
}

#[tokio::test]
async fn stop_with_no_active_session_is_a_conflict() {
    let state = knx_server::AppState::default();
    let app = knx_server::app(Arc::new(state), None);
    let response = call(&app, "POST", "/api/bus/monitor/stop", None).await;
    assert_eq!(response.status(), StatusCode::CONFLICT);
}

/// The carried Task 2 review finding, at the HTTP layer: a drain task that
/// panics on its way out must be reported as a `200` carrying a `warning`
/// field, never as a silently clean stop, and the buffer's contents must
/// still be reported accurately.
#[tokio::test]
async fn stop_surfaces_a_panicked_drain_task_as_a_warning_not_a_clean_stop() {
    let (tunnel, handle) = fake_tunnel();
    handle.panic_on_disconnect();
    let state = state_with_connector(FakeConnector::succeeding(tunnel));
    let app = knx_server::app(Arc::new(state), None);

    call(
        &app,
        "POST",
        "/api/bus/monitor/start",
        Some(json!({ "gateway": "192.0.2.10:3671" })),
    )
    .await;
    handle
        .sender()
        .send(group_value_write(1, GroupValue::Short(1)))
        .unwrap();
    poll_until_len(&app, 1).await;

    let response = call(&app, "POST", "/api/bus/monitor/stop", None).await;
    assert_eq!(response.status(), StatusCode::OK);
    let body = body_json(response).await;
    assert_eq!(
        body["telegramCount"], 1,
        "the telegram buffered before the panic must still be reported"
    );
    let warning = body["warning"]
        .as_str()
        .expect("a panicked drain task must surface as a warning, not silently");
    assert!(warning.contains("scripted disconnect panic"));
}

// ---------------------------------------------------------------------------
// telegrams: 404 vs 409 — a GET against a session that never existed is a
// not-found, distinct from the mutating routes' 409 for the same fact.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn telegrams_with_no_session_is_not_found_not_conflict() {
    let state = knx_server::AppState::default();
    let app = knx_server::app(Arc::new(state), None);

    let response = call(&app, "GET", "/api/bus/monitor/telegrams", None).await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    // Same fact, `since` given — still 404, not 409, matching the
    // "no active session" case of every other route.
    let response = call(&app, "GET", "/api/bus/monitor/telegrams?since=5", None).await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

// ---------------------------------------------------------------------------
// A corpus-gated smoke check against the real reference `.knxproj` — not
// the required "real project fixture" decode test (that's the always-run
// test above, which needs to know in advance which address resolves and
// which doesn't; a gitignored corpus offers no such guarantee), but an
// end-to-end sanity check that a session built from an actually-imported
// ETS project does not panic and does produce *some* decode outcome.
// ---------------------------------------------------------------------------

fn reference_ets4_path() -> PathBuf {
    knx_testsupport::reference_ets4_path()
}

#[tokio::test]
#[ignore = "requires the gitignored OriginalData/ corpus; run with --ignored"]
async fn a_session_over_a_real_imported_project_does_not_panic_while_decoding() {
    assert!(
        reference_ets4_path().exists(),
        "OriginalData/ corpus not present (gitignored, local-only); this test is #[ignore]d and must be run explicitly on a machine that has it"
    );
    let (tunnel, handle) = fake_tunnel();
    let state = state_with_connector(FakeConnector::succeeding(tunnel));
    let app = knx_server::app(Arc::new(state), None);

    let import = call(
        &app,
        "POST",
        "/api/project/import",
        Some(json!({ "path": reference_ets4_path().to_string_lossy() })),
    )
    .await;
    assert_eq!(import.status(), StatusCode::OK);

    call(
        &app,
        "POST",
        "/api/bus/monitor/start",
        Some(json!({ "gateway": "192.0.2.10:3671" })),
    )
    .await;

    // Any address at all — this is a smoke check, not an assertion on the
    // corpus's actual DPT resolution (unknown to this test in advance).
    handle
        .sender()
        .send(group_value_write(1, GroupValue::Short(1)))
        .unwrap();
    let telegrams = poll_until_len(&app, 1).await;
    assert!(telegrams["telegrams"][0]["decoded"]["kind"].is_string());
}
