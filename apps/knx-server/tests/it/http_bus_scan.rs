//! Incremental line-scan HTTP lifecycle tests over the in-process fake tunnel only.

use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use serde_json::{json, Value};
use tower::ServiceExt;

use knx_core::{
    Area, CommissioningState, CompletionStatus, DeviceId, DeviceInstance, Installation,
    InstallationId, Language, Line, Project, SourceRef, Topology,
};
use knx_net::{ApplicationService, Destination, LDataFrame, LDataMessageKind, Tpci, TunnelEvent};
use knx_server::fake::{FakeConnector, FakeTunnel, FakeTunnelHandle};

fn addr(device: u8) -> knx_core::IndividualAddress {
    knx_core::IndividualAddress::new(1, 1, device).expect("synthetic test address is valid")
}

fn fake_tunnel() -> (FakeTunnel, FakeTunnelHandle) {
    FakeTunnel::new(addr(9), 64)
}

fn state_with(tunnel: FakeTunnel) -> knx_server::AppState {
    knx_server::AppState {
        connector: Box::new(FakeConnector::succeeding(tunnel)),
        ..Default::default()
    }
}

fn source(tag: &str) -> SourceRef {
    SourceRef {
        path: tag.into(),
        ets_id: tag.into(),
    }
}

fn project_device(id: DeviceId, address: knx_core::IndividualAddress) -> DeviceInstance {
    DeviceInstance {
        id,
        source: source("test-device"),
        name: format!("Device {address}"),
        description: None,
        address: Some(address),
        product_ref: String::new(),
        program_ref: String::new(),
        commissioning: CommissioningState::default(),
        visibility_calculated: true,
        com_objects: vec![],
        binary_data: vec![],
    }
}

fn state_with_project(tunnel: FakeTunnel) -> knx_server::AppState {
    let mut project = Project::new(Language("en".into()));
    let area_id = project.ids.next_area_id().unwrap();
    let line_id = project.ids.next_line_id().unwrap();
    let missing_id = project.ids.next_device_id().unwrap();
    let excluded_id = project.ids.next_device_id().unwrap();
    project.devices.insert(project_device(missing_id, addr(2)));
    project.devices.insert(project_device(excluded_id, addr(3)));
    project.installations.push(Installation {
        id: InstallationId(0),
        name: "Installation".into(),
        default_line: None,
        multicast_address: None,
        completion: CompletionStatus::FinishedDesign,
        topology: Topology {
            areas: vec![Area {
                id: area_id,
                source: source("test-area"),
                name: "Area".into(),
                address: 1,
                completion: CompletionStatus::FinishedDesign,
                lines: vec![line_id],
            }],
            lines: vec![Line {
                id: line_id,
                source: source("test-line"),
                name: "Line".into(),
                address: 1,
                medium_ref: String::new(),
                domain_address: None,
                domain_address_is_checked: None,
                ip_routing_multicast_address: None,
                multicast_ttl: None,
                completion: CompletionStatus::FinishedDesign,
                devices: vec![missing_id, excluded_id],
            }],
            unassigned: vec![],
        },
        buildings: vec![],
        group_ranges: vec![],
        group_addresses: vec![],
        parameters: vec![],
    });

    let state = state_with(tunnel);
    *state.project.lock().unwrap() = Some(project);
    state
}

async fn call(
    app: &axum::Router,
    method: &str,
    uri: &str,
    body: Option<Value>,
) -> axum::response::Response {
    let mut request = Request::builder().method(method).uri(uri);
    let body = match body {
        Some(value) => {
            request = request.header("content-type", "application/json");
            Body::from(value.to_string())
        }
        None => Body::empty(),
    };
    app.clone()
        .oneshot(request.body(body).unwrap())
        .await
        .unwrap()
}

async fn body_json(response: axum::response::Response) -> Value {
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

fn scan_request(first: u8, last: u8, excluded: &[&str], timeout_ms: u64) -> Value {
    json!({
        "gateway": "192.0.2.10:3671",
        "area": 1,
        "line": 1,
        "firstDevice": first,
        "lastDevice": last,
        "excluded": excluded,
        "responseTimeoutMs": timeout_ms,
        "interProbePauseMs": 0
    })
}

async fn poll_until_terminal(app: &axum::Router) -> Value {
    for _ in 0..200 {
        let response = call(app, "GET", "/api/bus/scan/results?since=0", None).await;
        assert_eq!(response.status(), StatusCode::OK);
        let body = body_json(response).await;
        if body["status"] != "running" {
            return body;
        }
        tokio::time::sleep(std::time::Duration::from_millis(2)).await;
    }
    panic!("scan did not become terminal");
}

#[tokio::test]
async fn estimate_discloses_candidate_count_omissions_and_worst_case_basis() {
    let app = knx_server::app(Arc::new(knx_server::AppState::default()), None);
    let response = call(
        &app,
        "POST",
        "/api/bus/scan/estimate",
        Some(scan_request(2, 3, &["1.1.2"], 10)),
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
    let body = body_json(response).await;
    assert_eq!(body["candidateCount"], 1);
    assert_eq!(body["omittedAddresses"], json!(["1.1.2"]));
    assert_eq!(body["responseTimeoutMs"], 10);
    assert_eq!(body["vacantConfirmations"], 1);
    assert_eq!(body["interProbePauseMs"], 0);
    assert_eq!(body["worstCaseMs"], 10);
}

#[tokio::test]
async fn estimate_rejects_device_zero_as_the_line_coupler_address() {
    let app = knx_server::app(Arc::new(knx_server::AppState::default()), None);
    let response = call(
        &app,
        "POST",
        "/api/bus/scan/estimate",
        Some(scan_request(0, 3, &[], 10)),
    )
    .await;

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn a_scan_completes_and_streams_the_self_address_as_its_own_outcome() {
    let (tunnel, handle) = fake_tunnel();
    let app = knx_server::app(Arc::new(state_with(tunnel)), None);

    let start = call(
        &app,
        "POST",
        "/api/bus/scan/start",
        Some(scan_request(9, 9, &[], 10)),
    )
    .await;
    assert_eq!(start.status(), StatusCode::OK);

    let body = poll_until_terminal(&app).await;
    assert_eq!(body["status"], "completed");
    assert_eq!(body["completedCount"], 1);
    assert_eq!(body["totalCount"], 1);
    assert_eq!(body["results"][0]["address"], "1.1.9");
    assert_eq!(body["results"][0]["outcome"]["kind"], "selfAddress");
    assert!(handle.sent_frames().is_empty());
    assert!(handle.disconnected());
}

#[tokio::test]
async fn an_excluded_address_never_reaches_the_transport() {
    let (tunnel, handle) = fake_tunnel();
    let app = knx_server::app(Arc::new(state_with(tunnel)), None);

    let start = call(
        &app,
        "POST",
        "/api/bus/scan/start",
        Some(scan_request(2, 3, &["1.1.2"], 1)),
    )
    .await;
    assert_eq!(start.status(), StatusCode::OK);
    let body = poll_until_terminal(&app).await;
    assert_eq!(body["status"], "completed");

    let destinations: Vec<_> = handle
        .sent_frames()
        .into_iter()
        .map(|(destination, _, _)| destination)
        .collect();
    assert!(!destinations.contains(&Destination::Individual(addr(2))));
    assert!(destinations.contains(&Destination::Individual(addr(3))));
}

#[tokio::test]
async fn cancellation_stops_the_probe_and_disconnects_the_tunnel() {
    let (tunnel, handle) = fake_tunnel();
    let app = knx_server::app(Arc::new(state_with(tunnel)), None);

    let start = call(
        &app,
        "POST",
        "/api/bus/scan/start",
        Some(scan_request(2, 3, &[], 60_000)),
    )
    .await;
    assert_eq!(start.status(), StatusCode::OK);

    for _ in 0..200 {
        if !handle.sent_frames().is_empty() {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(2)).await;
    }
    assert!(!handle.sent_frames().is_empty(), "probe never started");

    let session_id = body_json(start).await["sessionId"].as_u64().unwrap();
    let activity = body_json(call(&app, "GET", "/api/bus/activity", None).await).await;
    assert_eq!(activity["sessions"][0]["kind"], "lineScan");
    assert_eq!(activity["sessions"][0]["id"], session_id);
    assert_eq!(activity["sessions"][0]["state"], "running");
    assert_eq!(activity["sessions"][0]["total"], 2);
    let cancel = call(
        &app,
        "POST",
        &format!("/api/bus/scan/cancel?sessionId={session_id}"),
        None,
    )
    .await;
    assert_eq!(cancel.status(), StatusCode::OK);
    assert_eq!(body_json(cancel).await["status"], "cancelled");
    let activity = body_json(call(&app, "GET", "/api/bus/activity", None).await).await;
    assert_eq!(activity["sessions"][0]["state"], "cancelled");
    assert_eq!(activity["sessions"][0]["id"], session_id);
    let sent_after_cancel = handle.sent_frames().len();
    tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    assert_eq!(handle.sent_frames().len(), sent_after_cancel);
    assert!(handle.disconnected());
}

#[tokio::test]
async fn poll_and_cancel_reject_a_different_scan_session() {
    let (tunnel, _handle) = fake_tunnel();
    let app = knx_server::app(Arc::new(state_with(tunnel)), None);
    let start = call(
        &app,
        "POST",
        "/api/bus/scan/start",
        Some(scan_request(2, 2, &[], 60_000)),
    )
    .await;
    let session_id = body_json(start).await["sessionId"].as_u64().unwrap();
    let wrong_id = session_id + 1;

    let poll = call(
        &app,
        "GET",
        &format!("/api/bus/scan/results?since=0&sessionId={wrong_id}"),
        None,
    )
    .await;
    assert_eq!(poll.status(), StatusCode::CONFLICT);

    let cancel = call(
        &app,
        "POST",
        &format!("/api/bus/scan/cancel?sessionId={wrong_id}"),
        None,
    )
    .await;
    assert_eq!(cancel.status(), StatusCode::CONFLICT);

    let cleanup = call(
        &app,
        "POST",
        &format!("/api/bus/scan/cancel?sessionId={session_id}"),
        None,
    )
    .await;
    assert_eq!(cleanup.status(), StatusCode::OK);
}

#[tokio::test]
async fn completed_scan_comparison_keeps_all_three_evidence_groups_separate() {
    let (tunnel, handle) = fake_tunnel();
    let handle = Arc::new(handle);
    let app = knx_server::app(Arc::new(state_with_project(tunnel)), None);

    let response_handle = Arc::clone(&handle);
    let responder = tokio::spawn(async move {
        for _ in 0..1_000 {
            let descriptor_was_requested =
                response_handle
                    .sent_frames()
                    .iter()
                    .any(|(destination, _, service)| {
                        *destination == Destination::Individual(addr(4))
                            && matches!(
                                service,
                                ApplicationService::DeviceDescriptorRead { descriptor_type: 0 }
                            )
                    });
            if descriptor_was_requested {
                let _ = response_handle
                    .sender()
                    .send(TunnelEvent::Telegram(LDataFrame {
                        kind: LDataMessageKind::Indication,
                        source: addr(4),
                        destination: Destination::Individual(addr(9)),
                        transport: Tpci::NumberedData { seq: 0 },
                        service: ApplicationService::DeviceDescriptorResponse {
                            descriptor_type: 0,
                            data: vec![0x07, 0x01],
                        },
                        control: None,
                    }));
                return;
            }
            tokio::time::sleep(std::time::Duration::from_millis(1)).await;
        }
        panic!("descriptor probe for the synthetic responder was never sent");
    });

    let start = call(
        &app,
        "POST",
        "/api/bus/scan/start",
        Some(scan_request(2, 4, &["1.1.3"], 20)),
    )
    .await;
    assert_eq!(start.status(), StatusCode::OK);
    let session_id = body_json(start).await["sessionId"].as_u64().unwrap();
    let terminal = poll_until_terminal(&app).await;
    assert_eq!(terminal["status"], "completed");
    responder.await.unwrap();

    let comparison = call(
        &app,
        "GET",
        &format!("/api/bus/scan/comparison?sessionId={session_id}"),
        None,
    )
    .await;
    assert_eq!(comparison.status(), StatusCode::OK);
    assert_eq!(
        body_json(comparison).await,
        json!({
            "unexpected": ["1.1.4"],
            "missing": ["1.1.2"],
            "excludedInProject": ["1.1.3"]
        })
    );
}

#[tokio::test]
async fn selected_scan_findings_apply_as_one_batch_and_undo_restores_content_exactly() {
    let (tunnel, handle) = fake_tunnel();
    let handle = Arc::new(handle);
    let state = Arc::new(state_with_project(tunnel));
    let original_project = state.project.lock().unwrap().as_ref().unwrap().clone();
    let app = knx_server::app(Arc::clone(&state), None);

    let response_handle = Arc::clone(&handle);
    let responder = tokio::spawn(async move {
        for _ in 0..1_000 {
            if response_handle
                .sent_frames()
                .iter()
                .any(|(destination, _, service)| {
                    *destination == Destination::Individual(addr(4))
                        && matches!(service, ApplicationService::DeviceDescriptorRead { .. })
                })
            {
                let _ = response_handle
                    .sender()
                    .send(TunnelEvent::Telegram(LDataFrame {
                        kind: LDataMessageKind::Indication,
                        source: addr(4),
                        destination: Destination::Individual(addr(9)),
                        transport: Tpci::NumberedData { seq: 0 },
                        service: ApplicationService::DeviceDescriptorResponse {
                            descriptor_type: 0,
                            data: vec![0x07, 0x01],
                        },
                        control: None,
                    }));
                return;
            }
            tokio::time::sleep(std::time::Duration::from_millis(1)).await;
        }
        panic!("descriptor probe for the synthetic responder was never sent");
    });

    let start = call(
        &app,
        "POST",
        "/api/bus/scan/start",
        Some(scan_request(2, 4, &["1.1.3"], 20)),
    )
    .await;
    assert_eq!(start.status(), StatusCode::OK);
    let session_id = body_json(start).await["sessionId"].as_u64().unwrap();
    assert_eq!(poll_until_terminal(&app).await["status"], "completed");
    responder.await.unwrap();
    let frames_before_reconcile = handle.sent_frames();

    let reconcile = call(
        &app,
        "POST",
        "/api/bus/scan/reconcile",
        Some(json!({
            "sessionId": session_id,
            "unexpected": ["1.1.4"],
            "missing": ["1.1.2"]
        })),
    )
    .await;
    assert_eq!(reconcile.status(), StatusCode::OK);
    assert_eq!(body_json(reconcile).await["can_undo"], true);
    assert_eq!(handle.sent_frames(), frames_before_reconcile);

    {
        let project = state.project.lock().unwrap();
        let project = project.as_ref().unwrap();
        assert!(project
            .devices
            .iter()
            .all(|device| device.address != Some(addr(2))));
        let added = project
            .devices
            .iter()
            .find(|device| device.address == Some(addr(4)))
            .expect("selected unexpected address creates one minimal device");
        assert!(added.product_ref.is_empty());
        assert!(added.program_ref.is_empty());
        assert!(added.com_objects.is_empty());
        assert!(project
            .devices
            .iter()
            .any(|device| device.address == Some(addr(3))));
        assert!(project.installations[0].topology.lines[0]
            .devices
            .contains(&added.id));
    }

    let undo = call(&app, "POST", "/api/undo", None).await;
    assert_eq!(undo.status(), StatusCode::OK);
    // ADR-0039 Decision 2: undo restores the user's content exactly —
    // devices, topology order, everything but the id high-water mark, which
    // `ReserveIds` never rewinds, so the undone device's id is never reissued.
    let project = state.project.lock().unwrap().as_ref().unwrap().clone();
    assert!(
        project.same_user_content_as(&original_project),
        "undo must restore every entity and its order exactly"
    );
    let mut ids_as_before = project.clone();
    ids_as_before.ids = original_project.ids.clone();
    assert_eq!(
        format!("{ids_as_before:#?}"),
        format!("{original_project:#?}"),
        "the id counters are the only permitted difference"
    );
    assert!(project.ids.peek_device() > original_project.ids.peek_device());
}

#[tokio::test]
async fn empty_reconciliation_is_a_true_no_op_and_excluded_evidence_is_not_actionable() {
    let (tunnel, _handle) = fake_tunnel();
    let state = Arc::new(state_with_project(tunnel));
    let original = format!("{:#?}", state.project.lock().unwrap().as_ref().unwrap());
    let app = knx_server::app(Arc::clone(&state), None);

    let start = call(
        &app,
        "POST",
        "/api/bus/scan/start",
        Some(scan_request(2, 3, &["1.1.3"], 1)),
    )
    .await;
    assert_eq!(start.status(), StatusCode::OK);
    let session_id = body_json(start).await["sessionId"].as_u64().unwrap();
    assert_eq!(poll_until_terminal(&app).await["status"], "completed");

    let excluded_attempt = call(
        &app,
        "POST",
        "/api/bus/scan/reconcile",
        Some(json!({
            "sessionId": session_id,
            "unexpected": [],
            "missing": ["1.1.3"]
        })),
    )
    .await;
    assert_eq!(excluded_attempt.status(), StatusCode::BAD_REQUEST);

    let no_op = call(
        &app,
        "POST",
        "/api/bus/scan/reconcile",
        Some(json!({
            "sessionId": session_id,
            "unexpected": [],
            "missing": []
        })),
    )
    .await;
    assert_eq!(no_op.status(), StatusCode::OK);
    assert_eq!(body_json(no_op).await["can_undo"], false);
    assert!(!state.command_stack.lock().unwrap().can_undo());
    assert_eq!(
        format!("{:#?}", state.project.lock().unwrap().as_ref().unwrap()),
        original
    );
}

#[tokio::test]
async fn running_and_cancelled_scans_cannot_be_reconciled() {
    let (tunnel, handle) = fake_tunnel();
    let state = Arc::new(state_with_project(tunnel));
    let app = knx_server::app(Arc::clone(&state), None);
    let start = call(
        &app,
        "POST",
        "/api/bus/scan/start",
        Some(scan_request(2, 2, &[], 60_000)),
    )
    .await;
    assert_eq!(start.status(), StatusCode::OK);
    let session_id = body_json(start).await["sessionId"].as_u64().unwrap();
    for _ in 0..200 {
        if !handle.sent_frames().is_empty() {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(2)).await;
    }

    let body = json!({
        "sessionId": session_id,
        "unexpected": [],
        "missing": []
    });
    let running = call(&app, "POST", "/api/bus/scan/reconcile", Some(body.clone())).await;
    assert_eq!(running.status(), StatusCode::CONFLICT);

    let cancel = call(
        &app,
        "POST",
        &format!("/api/bus/scan/cancel?sessionId={session_id}"),
        None,
    )
    .await;
    assert_eq!(cancel.status(), StatusCode::OK);
    let cancelled = call(&app, "POST", "/api/bus/scan/reconcile", Some(body)).await;
    assert_eq!(cancelled.status(), StatusCode::CONFLICT);
    assert!(!state.command_stack.lock().unwrap().can_undo());
}

/// KNOWN_LIMITATIONS §126 / AR14: the server recomputes the comparison
/// against the current project and refuses selections that are foreign,
/// duplicated, ambiguous or stale — each without touching the project or
/// the undo history.
#[tokio::test]
async fn reconciliation_refuses_foreign_duplicate_ambiguous_and_stale_selections() {
    let (tunnel, handle) = fake_tunnel();
    let state = Arc::new(state_with_project(tunnel));
    let app = knx_server::app(Arc::clone(&state), None);

    let start = call(
        &app,
        "POST",
        "/api/bus/scan/start",
        Some(scan_request(2, 2, &[], 1)),
    )
    .await;
    assert_eq!(start.status(), StatusCode::OK);
    let session_id = body_json(start).await["sessionId"].as_u64().unwrap();
    assert_eq!(poll_until_terminal(&app).await["status"], "completed");
    let frames_after_scan = handle.sent_frames();

    let reconcile = |session: u64, missing: Value| {
        let app = app.clone();
        async move {
            let response = call(
                &app,
                "POST",
                "/api/bus/scan/reconcile",
                Some(json!({ "sessionId": session, "unexpected": [], "missing": missing })),
            )
            .await;
            let status = response.status();
            let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
                .await
                .unwrap();
            (status, String::from_utf8_lossy(&bytes).into_owned())
        }
    };
    let snapshot = || format!("{:#?}", state.project.lock().unwrap().as_ref().unwrap());
    let unchanged = |before: &str| {
        assert_eq!(
            snapshot(),
            before,
            "a refused reconciliation changed the project"
        );
        assert!(!state.command_stack.lock().unwrap().can_undo());
    };

    let before = snapshot();
    let (status, _) = reconcile(session_id + 1, json!(["1.1.2"])).await;
    assert_eq!(status, StatusCode::CONFLICT, "foreign session");
    unchanged(&before);

    let (status, body) = reconcile(session_id, json!(["1.1.2", "1.1.2"])).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(body.contains("duplicate"), "{body}");
    unchanged(&before);

    // A second project device takes the same address: deleting "the"
    // missing device would be a guess.
    {
        let mut guard = state.project.lock().unwrap();
        let project = guard.as_mut().unwrap();
        let twin = project.ids.next_device_id().unwrap();
        project.devices.insert(project_device(twin, addr(2)));
    }
    let before = snapshot();
    let (status, body) = reconcile(session_id, json!(["1.1.2"])).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(body.contains("exactly one project device"), "{body}");
    unchanged(&before);

    // The project moved on: no device claims 1.1.2 any more, so the scan's
    // old "missing" finding is no longer part of the current comparison.
    {
        let mut guard = state.project.lock().unwrap();
        let project = guard.as_mut().unwrap();
        let ids: Vec<_> = project
            .devices
            .iter()
            .filter(|device| device.address == Some(addr(2)))
            .map(|device| device.id)
            .collect();
        for id in ids {
            project.devices.get_mut(id).unwrap().address = Some(addr(7));
        }
    }
    let before = snapshot();
    let (status, body) = reconcile(session_id, json!(["1.1.2"])).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(
        body.contains("not part of the current scan comparison"),
        "{body}"
    );
    unchanged(&before);

    assert_eq!(
        handle.sent_frames(),
        frames_after_scan,
        "reconciliation sent KNX traffic"
    );
}
