//! HTTP regressions for the AR20 telegram-flow backend contract.
//!
//! AR20 telegram-flow contract (`docs/TELEGRAM_FLOW_VISUALIZATION.md` §9.2,
//! as delivered): typed raw addresses, server-monotonic observation age and
//! the flow-context generation on every monitor row, and the read-only
//! `GET /api/bus/monitor/flow-snapshot` route with configured participants.

use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use serde_json::{json, Value};
use tower::ServiceExt;

use knx_core::{
    ComObjectInstance, ComObjectInstanceId, DeviceId, Direction, GroupAddress, GroupAddressEntry,
    GroupAddressId, GroupLink, GroupValue, Installation, InstallationId, Language, Layer, Override,
    Resolved, ResolvedFlags, SourceRef,
};
use knx_net::{ApplicationService, Destination};
use knx_server::fake::{FakeConnector, FakeTunnel, FakeTunnelHandle};

fn addr(device: u8) -> knx_core::IndividualAddress {
    knx_core::IndividualAddress::new(1, 1, device).expect("valid test address")
}

fn telegram(source: u8, raw: u16) -> knx_net::TunnelEvent {
    knx_net::TunnelEvent::Telegram(knx_net::LDataFrame {
        kind: knx_net::LDataMessageKind::Indication,
        source: addr(source),
        destination: Destination::Group(GroupAddress::from_raw(raw)),
        transport: knx_net::Tpci::UnnumberedData,
        service: ApplicationService::GroupValueWrite(GroupValue::Short(1)),
        control: None,
    })
}

async fn call(
    app: &axum::Router,
    method: &str,
    uri: &str,
    body: Option<Value>,
) -> (StatusCode, Value) {
    let mut builder = Request::builder().method(method).uri(uri);
    let body = match body {
        Some(v) => {
            builder = builder.header("content-type", "application/json");
            Body::from(v.to_string())
        }
        None => Body::empty(),
    };
    let response = app
        .clone()
        .oneshot(builder.body(body).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

fn source() -> SourceRef {
    SourceRef {
        path: "t".into(),
        ets_id: "t".into(),
    }
}

/// Device 1 (1.1.9) sends on GA 1 (raw 1, DPST-1-1) with the write flag
/// stated; GA 2 has no members.
fn project() -> knx_core::Project {
    let mut project = knx_core::Project::new(Language("en".into()));
    project.devices.insert(knx_core::DeviceInstance {
        id: DeviceId(1),
        source: source(),
        name: "Switch".into(),
        description: None,
        address: Some(addr(9)),
        product_ref: "P".into(),
        program_ref: "H".into(),
        commissioning: Default::default(),
        visibility_calculated: true,
        com_objects: vec![ComObjectInstanceId(1)],
        binary_data: vec![],
    });
    project.devices.insert_com_object(ComObjectInstance {
        id: ComObjectInstanceId(1),
        source: source(),
        device: DeviceId(1),
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
        flags: ResolvedFlags {
            write: Override::Value(Resolved {
                value: true,
                layer: Layer::Instance,
            }),
            ..ResolvedFlags::none()
        },
        size: None,
        is_active: true,
        links: vec![GroupLink {
            ga: GroupAddressId(1),
            direction: Direction::Send,
        }],
        module_instance: None,
    });
    let entry = |id: u32, name: &str| GroupAddressEntry {
        id: GroupAddressId(id),
        source: source(),
        name: name.into(),
        address: GroupAddress::from_raw(id as u16),
        central: false,
        unfiltered: false,
        range: None,
    };
    project.installations.push(Installation {
        id: InstallationId(1),
        name: "I".into(),
        default_line: None,
        multicast_address: None,
        completion: knx_core::CompletionStatus::FinishedDesign,
        topology: knx_core::Topology {
            areas: vec![],
            lines: vec![],
            unassigned: vec![DeviceId(1)],
        },
        buildings: vec![],
        group_ranges: vec![],
        group_addresses: vec![entry(1, "Light"), entry(2, "Unlinked")],
        parameters: vec![],
    });
    project
}

struct Harness {
    app: axum::Router,
    state: Arc<knx_server::AppState>,
    handle: FakeTunnelHandle,
}

async fn started(with_project: bool) -> Harness {
    let (tunnel, handle) = FakeTunnel::new(addr(5), 64);
    let mut state = knx_server::AppState {
        connector: Box::new(FakeConnector::succeeding(tunnel)),
        ..Default::default()
    };
    state.product_db = None;
    if with_project {
        *state.project.lock().unwrap() = Some(project());
    }
    let state = Arc::new(state);
    let app = knx_server::app(state.clone(), None);
    let (status, _) = call(
        &app,
        "POST",
        "/api/bus/monitor/start",
        Some(json!({ "gateway": "192.0.2.10:3671" })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    Harness { app, state, handle }
}

async fn poll_until(app: &axum::Router, len: usize) -> Value {
    for _ in 0..200 {
        let (status, body) = call(app, "GET", "/api/bus/monitor/telegrams?since=0", None).await;
        assert_eq!(status, StatusCode::OK);
        if body["telegrams"].as_array().unwrap().len() >= len {
            return body;
        }
        tokio::time::sleep(std::time::Duration::from_millis(5)).await;
    }
    panic!("telegrams never reached {len}");
}

#[tokio::test]
async fn rows_carry_raw_addresses_observation_age_and_their_generation() {
    let h = started(true).await;
    h.handle.sender().send(telegram(9, 1)).unwrap();
    let body = poll_until(&h.app, 1).await;
    assert_eq!(body["flowGeneration"], "1");
    let row = &body["telegrams"][0];
    assert_eq!(row["sourceRaw"], addr(9).raw());
    assert_eq!(row["destinationRaw"], 1);
    assert_eq!(row["flowGeneration"], "1");
    let first_age = row["observedAgeMs"].as_u64().expect("age is a number");
    // Existing fields are unchanged.
    assert_eq!(row["source"], "1.1.9");
    assert_eq!(row["destination"], "0/0/1");

    tokio::time::sleep(std::time::Duration::from_millis(60)).await;
    let (_, later) = call(&h.app, "GET", "/api/bus/monitor/telegrams?since=0", None).await;
    let later_age = later["telegrams"][0]["observedAgeMs"].as_u64().unwrap();
    assert!(
        later_age >= first_age + 50,
        "age is measured at response time: {first_age} then {later_age}"
    );
}

#[tokio::test]
async fn the_snapshot_lists_configured_participants_of_the_current_generation() {
    let h = started(true).await;
    let (status, snap) = call(
        &h.app,
        "GET",
        "/api/bus/monitor/flow-snapshot?sessionId=1&generation=1",
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{snap}");
    assert_eq!(snap["status"], "current");
    assert_eq!(snap["sessionId"], 1);
    assert_eq!(snap["generation"], "1");
    assert_eq!(snap["groupAddressStyle"], "ThreeLevel");
    assert!(!snap["serverIncarnation"].as_str().unwrap().is_empty());
    assert_eq!(snap["devices"][0]["deviceId"], 1);
    assert_eq!(snap["devices"][0]["individualAddressRaw"], addr(9).raw());
    assert_eq!(snap["devices"][0]["installationId"], 1);
    let light = &snap["groups"][0];
    assert_eq!(light["gaRaw"], 1);
    assert_eq!(light["dpt"], "DPST-1-1");
    assert_eq!(light["members"][0]["direction"], "Send");
    assert_eq!(light["members"][0]["active"], true);
    assert_eq!(light["members"][0]["flags"]["write"], true);
    assert!(light["members"][0]["flags"]["read"].is_null());
    assert_eq!(snap["groups"][1]["members"], json!([]));
    assert_eq!(
        snap["truncated"],
        json!({ "devices": 0, "groups": 0, "members": 0, "diagnostics": 0 })
    );

    // Without parameters: the current generation.
    let (_, current) = call(&h.app, "GET", "/api/bus/monitor/flow-snapshot", None).await;
    assert_eq!(current["status"], "current");
    assert_eq!(current["generation"], "1");
}

#[tokio::test]
async fn a_changed_context_bumps_the_generation_and_old_rows_keep_theirs() {
    let h = started(true).await;
    h.handle.sender().send(telegram(9, 1)).unwrap();
    poll_until(&h.app, 1).await;

    let (status, _) = call(
        &h.app,
        "POST",
        "/api/project/group-address-style",
        Some(json!({ "groupAddressStyle": "Free" })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    h.handle.sender().send(telegram(9, 1)).unwrap();
    let body = poll_until(&h.app, 2).await;
    assert_eq!(body["flowGeneration"], "2");
    assert_eq!(body["telegrams"][0]["flowGeneration"], "1");
    assert_eq!(body["telegrams"][1]["flowGeneration"], "2");
    assert_eq!(body["telegrams"][1]["destination"], "1");

    let (_, old) = call(
        &h.app,
        "GET",
        "/api/bus/monitor/flow-snapshot?sessionId=1&generation=1",
        None,
    )
    .await;
    assert_eq!(old["status"], "historical");
    assert_eq!(old["generation"], "1");
    assert_eq!(old["groups"], json!([]));
    let (_, new) = call(
        &h.app,
        "GET",
        "/api/bus/monitor/flow-snapshot?sessionId=1&generation=2",
        None,
    )
    .await;
    assert_eq!(new["status"], "current");
    assert_eq!(new["groupAddressStyle"], "Free");

    // Republishing an unchanged context keeps the generation.
    let (status, _) = call(
        &h.app,
        "POST",
        "/api/project/group-address-style",
        Some(json!({ "groupAddressStyle": "Free" })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let (_, again) = call(&h.app, "GET", "/api/bus/monitor/telegrams?since=0", None).await;
    assert_eq!(again["flowGeneration"], "2");
}

#[tokio::test]
async fn link_flag_activation_and_device_edits_make_the_context_stale() {
    for change in [
        "flag",
        "activation",
        "deviceName",
        "deviceAddress",
        "secondLink",
    ] {
        let h = started(true).await;
        let mut edited = project();
        match change {
            "flag" => {
                edited
                    .devices
                    .com_object_mut(ComObjectInstanceId(1))
                    .unwrap()
                    .flags
                    .read = Override::Value(Resolved {
                    value: true,
                    layer: Layer::UserEdit,
                })
            }
            "activation" => {
                edited
                    .devices
                    .com_object_mut(ComObjectInstanceId(1))
                    .unwrap()
                    .is_active = false
            }
            "deviceName" => edited.devices.get_mut(DeviceId(1)).unwrap().name = "New".into(),
            "deviceAddress" => {
                edited.devices.get_mut(DeviceId(1)).unwrap().address = Some(addr(10))
            }
            // Same object, same DPT, one more link: DPTs and names are equal.
            _ => edited
                .devices
                .com_object_mut(ComObjectInstanceId(1))
                .unwrap()
                .links
                .push(GroupLink {
                    ga: GroupAddressId(1),
                    direction: Direction::Receive,
                }),
        }
        *h.state.project.lock().unwrap() = Some(edited);
        let (_, body) = call(&h.app, "GET", "/api/bus/monitor/telegrams", None).await;
        assert_eq!(body["contextStatus"], "stale", "{change}");
    }
}

#[tokio::test]
async fn without_a_project_the_snapshot_is_unavailable() {
    let h = started(false).await;
    let (status, snap) = call(&h.app, "GET", "/api/bus/monitor/flow-snapshot", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(snap["status"], "unavailable");
    assert_eq!(snap["devices"], json!([]));
    assert!(snap["groupAddressStyle"].is_null());
    h.handle.sender().send(telegram(9, 1)).unwrap();
    let body = poll_until(&h.app, 1).await;
    assert_eq!(body["telegrams"][0]["destinationRaw"], 1);
    assert_eq!(body["telegrams"][0]["destination"], "1");
}

#[tokio::test]
async fn another_session_is_historical_and_bad_parameters_are_refused() {
    let h = started(true).await;
    let (_, other) = call(
        &h.app,
        "GET",
        "/api/bus/monitor/flow-snapshot?sessionId=7&generation=1",
        None,
    )
    .await;
    assert_eq!(other["status"], "historical");
    assert_eq!(other["groups"], json!([]));
    for query in ["generation=abc", "generation=-1", "sessionId=1.5"] {
        let (status, _) = call(
            &h.app,
            "GET",
            &format!("/api/bus/monitor/flow-snapshot?{query}"),
            None,
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{query}");
    }
}

#[tokio::test]
async fn the_snapshot_without_a_session_is_not_found() {
    let state = Arc::new(knx_server::AppState::default());
    let app = knx_server::app(state, None);
    let (status, _) = call(&app, "GET", "/api/bus/monitor/flow-snapshot", None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn the_closed_marker_has_no_raw_addresses_or_generation() {
    let h = started(true).await;
    h.handle
        .sender()
        .send(knx_net::TunnelEvent::Closed)
        .unwrap();
    let body = poll_until(&h.app, 1).await;
    let marker = &body["telegrams"][0];
    assert_eq!(marker["service"], "SessionClosed");
    assert!(marker["sourceRaw"].is_null());
    assert!(marker["destinationRaw"].is_null());
    assert!(marker["flowGeneration"].is_null());
    assert!(marker["observedAgeMs"].is_u64());
}

fn frame(destination: Destination, service: ApplicationService) -> knx_net::TunnelEvent {
    knx_net::TunnelEvent::Telegram(knx_net::LDataFrame {
        kind: knx_net::LDataMessageKind::Indication,
        source: addr(9),
        destination,
        transport: knx_net::Tpci::UnnumberedData,
        service,
        control: None,
    })
}

/// Write, Read, Response and an opaque group service all carry the raw
/// identity and generation; Read and the opaque service keep `decoded:
/// null`; an individually addressed frame still produces no row.
#[tokio::test]
async fn every_admitted_group_service_carries_the_new_fields_and_individual_frames_stay_out() {
    let h = started(true).await;
    let ga = || Destination::Group(GroupAddress::from_raw(1));
    for event in [
        frame(
            ga(),
            ApplicationService::GroupValueWrite(GroupValue::Short(1)),
        ),
        frame(ga(), ApplicationService::GroupValueRead),
        frame(
            ga(),
            ApplicationService::GroupValueResponse(GroupValue::Short(0)),
        ),
        frame(
            Destination::Individual(addr(20)),
            ApplicationService::GroupValueRead,
        ),
        frame(
            ga(),
            ApplicationService::Other {
                apci: 0x3c0,
                data: vec![1],
            },
        ),
    ] {
        h.handle.sender().send(event).unwrap();
    }
    let body = poll_until(&h.app, 4).await;
    let rows = body["telegrams"].as_array().unwrap();
    assert_eq!(
        rows.len(),
        4,
        "the individually addressed frame is not admitted"
    );
    let services: Vec<&str> = rows
        .iter()
        .map(|r| r["service"].as_str().unwrap())
        .collect();
    assert_eq!(
        services,
        [
            "GroupValueWrite",
            "GroupValueRead",
            "GroupValueResponse",
            "Other"
        ]
    );
    for row in rows {
        assert_eq!(row["sourceRaw"], addr(9).raw());
        assert_eq!(row["destinationRaw"], 1);
        assert_eq!(row["flowGeneration"], "1");
    }
    assert!(rows[1]["decoded"].is_null(), "a Read carries no value");
    assert_eq!(rows[2]["decoded"]["kind"], "value");
    assert!(rows[3]["decoded"].is_null());
    let seqs: Vec<u64> = rows.iter().map(|r| r["seq"].as_u64().unwrap()).collect();
    assert_eq!(
        seqs,
        [0, 1, 2, 3],
        "no sequence consumed by the dropped frame"
    );
}
