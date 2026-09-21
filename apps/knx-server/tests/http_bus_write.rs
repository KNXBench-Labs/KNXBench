//! `POST /api/bus/write` — design spec `docs/superpowers/specs/
//! 2026-09-11-group-monitor-design.md` §4.3/§6, D6 test 6 ("write with no
//! session active returns 409; with one active, asserts the fake tunnel's
//! call log recorded the exact `Destination`/`ApplicationService`"), plus
//! this task's own extra coverage: bad address, encode failure, unresolved/
//! conflicting DPT with none given, and the `502` path when the tunnel's
//! `send` itself fails. No gateway, no socket, anywhere in this file.

use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use serde_json::{json, Value};
use tower::ServiceExt;

use knx_core::{
    ComObjectInstance, ComObjectInstanceId, Direction, GroupAddress, GroupAddressEntry,
    GroupAddressId, GroupAddressStyle, GroupLink, GroupValue, Installation, InstallationId,
    Language, Layer, Override, Resolved, ResolvedFlags, SourceRef,
};
use knx_net::{ApplicationService, Destination};
use knx_server::fake::{FakeConnector, FakeTunnel, FakeTunnelHandle};

fn addr(device: u8) -> knx_core::IndividualAddress {
    knx_core::IndividualAddress::new(1, 1, device).expect("valid test address")
}

fn fake_tunnel() -> (FakeTunnel, FakeTunnelHandle) {
    FakeTunnel::new(addr(5), 64)
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

async fn start_session(app: &axum::Router) {
    let response = call(
        app,
        "POST",
        "/api/bus/monitor/start",
        Some(json!({ "gateway": "192.0.2.10:3671" })),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
}

/// A project with one group address (`1`) resolving to a single DPT
/// (`DPST-1-1`, boolean), one (`2`) with no linked communication object
/// (`None`), and one (`3`) with two disagreeing communication objects
/// (`Conflict`) — same three-outcome shape as `bus.rs`'s own
/// `project_with_group_addresses` unit-test fixture, built here directly
/// since this crate has no `lib` target to share it from.
fn project_with_write_dpt_outcomes() -> knx_core::Project {
    let source = || SourceRef {
        path: "t".into(),
        ets_id: "t".into(),
    };
    let stated = |main: u16, sub: u16| {
        Override::Value(Resolved {
            value: knx_core::DptRef {
                main,
                sub: Some(sub),
            },
            layer: Layer::Instance,
        })
    };
    let com_object = |id: u32,
                      resolved_dpt: knx_core::Override<knx_core::DptRef>,
                      ga: GroupAddressId| ComObjectInstance {
        id: ComObjectInstanceId(id),
        source: source(),
        device: knx_core::DeviceId(1),
        number: 0,
        text: Override::Absent,
        description: Override::Absent,
        dpt: resolved_dpt,
        flags: ResolvedFlags::none(),
        size: None,
        is_active: true,
        links: vec![GroupLink {
            ga,
            direction: Direction::Send,
        }],
        module_instance: None,
    };

    let mut project = knx_core::Project::new(Language("en".into()));
    project
        .devices
        .insert_com_object(com_object(1, stated(1, 1), GroupAddressId(1)));
    project
        .devices
        .insert_com_object(com_object(3, stated(5, 1), GroupAddressId(3)));
    project
        .devices
        .insert_com_object(com_object(4, stated(1, 1), GroupAddressId(3)));

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
                source: source(),
                name: "Resolves".into(),
                address: GroupAddress::from_raw(1),
                central: false,
                unfiltered: false,
                range: None,
            },
            GroupAddressEntry {
                id: GroupAddressId(2),
                source: source(),
                name: "Unlinked".into(),
                address: GroupAddress::from_raw(2),
                central: false,
                unfiltered: false,
                range: None,
            },
            GroupAddressEntry {
                id: GroupAddressId(3),
                source: source(),
                name: "Conflicting".into(),
                address: GroupAddress::from_raw(3),
                central: false,
                unfiltered: false,
                range: None,
            },
        ],
        parameters: vec![],
    });
    project
}

fn state_with_project_and_connector(
    project: knx_core::Project,
    connector: FakeConnector,
) -> knx_server::AppState {
    knx_server::AppState {
        connector: Box::new(connector),
        project: std::sync::Mutex::new(Some(project)),
        ..Default::default()
    }
}

// ---------------------------------------------------------------------------
// D6 test 6a — no active session is 409, never opens a second connection.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn write_with_no_active_session_is_a_conflict() {
    let state = knx_server::AppState::default();
    let app = knx_server::app(Arc::new(state), None);

    let response = call(
        &app,
        "POST",
        "/api/bus/write",
        Some(json!({ "destination": "0/0/1", "dpt": "DPST-1-1", "value": "on" })),
    )
    .await;
    assert_eq!(response.status(), StatusCode::CONFLICT);
}

// ---------------------------------------------------------------------------
// D6 test 6b — with a session active, the fake tunnel's call log records
// exactly the Destination/ApplicationService the request should produce.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn write_with_an_explicit_dpt_sends_the_encoded_value_through_the_open_tunnel() {
    let (tunnel, handle) = fake_tunnel();
    let state = state_with_project_and_connector(
        project_with_write_dpt_outcomes(),
        FakeConnector::succeeding(tunnel),
    );
    let app = knx_server::app(Arc::new(state), None);
    start_session(&app).await;

    let response = call(
        &app,
        "POST",
        "/api/bus/write",
        Some(json!({ "destination": "0/0/1", "dpt": "DPST-1-1", "value": "on" })),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let body = body_json(response).await;
    assert_eq!(body["service"], "GroupValueWrite");
    assert_eq!(body["encodedPayload"], "0x01 (6-bit)");

    let sent = handle.sent_calls();
    assert_eq!(sent.len(), 1);
    assert_eq!(sent[0].0, Destination::Group(GroupAddress::from_raw(1)));
    assert!(matches!(
        &sent[0].1,
        ApplicationService::GroupValueWrite(v) if *v == knx_core::GroupValue::Short(1)
    ));
}

// ---------------------------------------------------------------------------
// Task 27, fix round 1 — `decodedEcho` decodes the bytes the tunnel
// actually received, not `body.value`. The original version of this test
// wrote `"on"` as `DPST-1-1`, whose decode also renders `"on"` — input and
// wire-decode were textually identical, so a handler that built
// `decodedEcho.text` straight from `body.value.clone()` (the exact
// shortcut this task's brief forbids) passed it too. This version writes
// `DPST-5-1` (`DPT_Percent_U8`, `decode_u8`/`encode_u8` in
// `crates/knx-core/src/dpt/codec.rs`): `"50"` in encodes to `round(50 *
// 255 / 100) = 128` (not the exact `127.5`), and `128` decodes back to
// `128 * 100 / 255 = 50.19607843137255`, formatted with its unit as
// `"50.19607843137255 %"` — visibly different text from the `"50"` that
// was typed, on the same codec pair the CLI and the bus monitor already
// trust. A handler reading `body.value` would print `"50"` here, not
// `"50.19607843137255 %"`; only a handler reading the wire bytes gets this
// test to pass. A dedicated "decode failure survives as 200" test does not
// appear here: the report explains why no shipped DPT's `encode`/`decode`
// pair can be driven to disagree through the public API (every
// asymmetric-looking case this task and its review checked turned out to
// be guarded identically on both sides), so a fabricated failure would
// assert nothing real.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn write_response_echoes_the_decoded_form_of_the_bytes_actually_sent() {
    let (tunnel, handle) = fake_tunnel();
    let state = state_with_project_and_connector(
        project_with_write_dpt_outcomes(),
        FakeConnector::succeeding(tunnel),
    );
    let app = knx_server::app(Arc::new(state), None);
    start_session(&app).await;

    let response = call(
        &app,
        "POST",
        "/api/bus/write",
        Some(json!({ "destination": "0/0/1", "dpt": "DPST-5-1", "value": "50" })),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let body = body_json(response).await;
    assert_eq!(body["decodedEcho"]["kind"], "value");
    assert_eq!(body["decodedEcho"]["dpt"], "DPST-5-1");
    assert_eq!(body["decodedEcho"]["text"], "50.19607843137255 %");
    assert!(body["decodedEcho"]["error"].is_null());

    // Same wire bytes the send assertions above already checked — the
    // decode ran against exactly this rounded raw byte (128), not against
    // the request's own `"50"`.
    let sent = handle.sent_calls();
    assert_eq!(sent.len(), 1);
    assert!(matches!(
        &sent[0].1,
        ApplicationService::GroupValueWrite(v) if *v == knx_core::GroupValue::Bytes(vec![128])
    ));
}

/// Same as above, but relying on the session's cached project resolution
/// instead of an explicit `dpt` — mirrors `resolve_write_value`'s
/// `--project` path.
#[tokio::test]
async fn write_without_an_explicit_dpt_resolves_it_from_the_session_project() {
    let (tunnel, handle) = fake_tunnel();
    let state = state_with_project_and_connector(
        project_with_write_dpt_outcomes(),
        FakeConnector::succeeding(tunnel),
    );
    let app = knx_server::app(Arc::new(state), None);
    start_session(&app).await;

    let response = call(
        &app,
        "POST",
        "/api/bus/write",
        Some(json!({ "destination": "0/0/1", "value": "on" })),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(handle.sent_calls().len(), 1);
}

// ---------------------------------------------------------------------------
// Bad address / encode failure / unresolved / conflicting DPT — all 400,
// never a guess.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn a_malformed_destination_is_a_bad_request() {
    let (tunnel, _handle) = fake_tunnel();
    let state = state_with_project_and_connector(
        project_with_write_dpt_outcomes(),
        FakeConnector::succeeding(tunnel),
    );
    let app = knx_server::app(Arc::new(state), None);
    start_session(&app).await;

    let response = call(
        &app,
        "POST",
        "/api/bus/write",
        Some(json!({ "destination": "not-an-address", "dpt": "DPST-1-1", "value": "on" })),
    )
    .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn an_encode_failure_is_a_bad_request() {
    let (tunnel, _handle) = fake_tunnel();
    let state = state_with_project_and_connector(
        project_with_write_dpt_outcomes(),
        FakeConnector::succeeding(tunnel),
    );
    let app = knx_server::app(Arc::new(state), None);
    start_session(&app).await;

    // "banana" is not a valid DPST-1-1 (boolean) value.
    let response = call(
        &app,
        "POST",
        "/api/bus/write",
        Some(json!({ "destination": "0/0/1", "dpt": "DPST-1-1", "value": "banana" })),
    )
    .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn an_unresolved_dpt_with_none_given_is_a_bad_request_naming_the_reason() {
    let (tunnel, handle) = fake_tunnel();
    let state = state_with_project_and_connector(
        project_with_write_dpt_outcomes(),
        FakeConnector::succeeding(tunnel),
    );
    let app = knx_server::app(Arc::new(state), None);
    start_session(&app).await;

    let response = call(
        &app,
        "POST",
        "/api/bus/write",
        Some(json!({ "destination": "0/0/2", "value": "on" })),
    )
    .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body = body_json(response).await;
    // Fix 3 (Task 5 review) — worded exactly like `BusComposeForm.tsx`'s
    // own client-side `NO_DPT_RESOLVED_MESSAGE`, not a second, differently
    // phrased opinion about the same fact.
    assert_eq!(
        body["error"].as_str().unwrap(),
        "No DPT resolved for this group address — enter one explicitly."
    );
    // Final review, criterion 7: an unresolved-DPT 400 is rejected before
    // ever reaching the tunnel — same pattern as
    // `a_tunnel_send_failure_is_reported_as_a_bad_gateway` below.
    assert_eq!(
        handle.sent_calls().len(),
        0,
        "a bad request must not touch the tunnel at all"
    );
}

#[tokio::test]
async fn a_conflicting_dpt_with_none_given_is_a_bad_request_naming_the_reason() {
    let (tunnel, handle) = fake_tunnel();
    let state = state_with_project_and_connector(
        project_with_write_dpt_outcomes(),
        FakeConnector::succeeding(tunnel),
    );
    let app = knx_server::app(Arc::new(state), None);
    start_session(&app).await;

    let response = call(
        &app,
        "POST",
        "/api/bus/write",
        Some(json!({ "destination": "0/0/3", "value": "on" })),
    )
    .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body = body_json(response).await;
    // Fix 3 (Task 5 review) — worded exactly like `BusComposeForm.tsx`'s own
    // client-side `conflictingDptsMessage()`, names and all.
    assert_eq!(
        body["error"].as_str().unwrap(),
        "Conflicting DPTs for this group address: DPST-1-1, DPST-5-1 — enter one explicitly."
    );
    // Final review, criterion 7: a conflicting-DPT 400 is rejected before
    // ever reaching the tunnel — same pattern as
    // `a_tunnel_send_failure_is_reported_as_a_bad_gateway` below.
    assert_eq!(
        handle.sent_calls().len(),
        0,
        "a bad request must not touch the tunnel at all"
    );
}

// ---------------------------------------------------------------------------
// Task 5 fix — `/write` parses `destination` in the *session's own* project
// style, so whatever `/telegrams` rendered parses back. Raw group address 1
// renders as the plain decimal `"1"` under `Free` and as `"0/1"` under
// `TwoLevel` — both of which the old hardcoded-`ThreeLevel` parse rejected
// outright (`GroupAddress::parse` needs exactly two slashes for
// `ThreeLevel`, and `"0/1"` has only one) — the exact round trip this task
// exists to fix. `ThreeLevel` itself needs no case here: it is already the
// project default and is exercised end-to-end by
// `write_with_an_explicit_dpt_sends_the_encoded_value_through_the_open_tunnel`
// above, via its `"0/0/1"` destination.
// ---------------------------------------------------------------------------

fn project_with_style_and_single_dpt(style: GroupAddressStyle) -> knx_core::Project {
    let mut project = project_with_write_dpt_outcomes();
    project.info.group_address_style = style;
    project
}

async fn poll_until_first_destination(app: &axum::Router) -> String {
    for _ in 0..200 {
        let response = call(app, "GET", "/api/bus/monitor/telegrams?since=0", None).await;
        assert_eq!(response.status(), StatusCode::OK);
        let body = body_json(response).await;
        if let Some(row) = body["telegrams"].as_array().unwrap().first() {
            return row["destination"].as_str().unwrap().to_string();
        }
        tokio::time::sleep(std::time::Duration::from_millis(2)).await;
    }
    panic!("no telegram ever appeared on the poll");
}

/// Table-driven over both non-three-level styles rather than one function
/// per style — Fix 1 from Task 5's review: the original version of this
/// test covered `Free` only, leaving `TwoLevel` (a distinct code path in
/// both `GroupAddress::parse` and `::format`, not just a different string)
/// unverified.
#[tokio::test]
async fn a_non_three_level_projects_telegram_destination_round_trips_through_write() {
    let cases = [
        (GroupAddressStyle::Free, "1"),
        (GroupAddressStyle::TwoLevel, "0/1"),
    ];

    for (style, expected_destination) in cases {
        let (tunnel, handle) = fake_tunnel();
        let state = state_with_project_and_connector(
            project_with_style_and_single_dpt(style),
            FakeConnector::succeeding(tunnel),
        );
        let app = knx_server::app(Arc::new(state), None);
        start_session(&app).await;

        // A telegram arrives on group address 1; /telegrams renders it in
        // the project's own style, not three-level.
        handle
            .sender()
            .send(knx_net::TunnelEvent::Telegram(knx_net::LDataFrame {
                kind: knx_net::LDataMessageKind::Indication,
                source: addr(9),
                destination: Destination::Group(GroupAddress::from_raw(1)),
                transport: knx_net::Tpci::UnnumberedData,
                service: ApplicationService::GroupValueWrite(GroupValue::Short(1)),
            }))
            .unwrap();

        let destination = poll_until_first_destination(&app).await;
        assert_eq!(
            destination, expected_destination,
            "{style:?} renders group address 1 in its own format"
        );

        // The exact string /telegrams just emitted must be accepted by
        // /write — before this fix it failed with 400 (the hardcoded
        // ThreeLevel parse rejects both of these strings for want of a
        // second slash).
        let response = call(
            &app,
            "POST",
            "/api/bus/write",
            Some(json!({ "destination": destination, "dpt": "DPST-1-1", "value": "on" })),
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK, "{style:?} round trip");
        let sent = handle.sent_calls();
        assert_eq!(sent.len(), 1, "{style:?} round trip");
        assert_eq!(sent[0].0, Destination::Group(GroupAddress::from_raw(1)));
    }
}

// ---------------------------------------------------------------------------
// 502 — the tunnel's own `send` fails.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn a_tunnel_send_failure_is_reported_as_a_bad_gateway() {
    let (tunnel, handle) = fake_tunnel();
    let state = state_with_project_and_connector(
        project_with_write_dpt_outcomes(),
        FakeConnector::succeeding(tunnel),
    );
    let app = knx_server::app(Arc::new(state), None);
    start_session(&app).await;
    handle.fail_next_send();

    let response = call(
        &app,
        "POST",
        "/api/bus/write",
        Some(json!({ "destination": "0/0/1", "dpt": "DPST-1-1", "value": "on" })),
    )
    .await;
    assert_eq!(response.status(), StatusCode::BAD_GATEWAY);
    assert_eq!(
        handle.sent_calls().len(),
        0,
        "a failed send must not be recorded as sent"
    );
}
