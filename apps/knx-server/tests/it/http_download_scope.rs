//! `scope: "changes"` on `/api/device-download/plan`: the flag advice, bound before any start.
//!
//! Synthetic throughout ([`knx_testsupport::vendor_plugin`] without its
//! plug-in declaration): one device at `1.1.7`, a program that plans the
//! complete download and every partial one. No gateway: a `start` refused as
//! stale stops before discovery; an explicit `start` that passes the scope
//! guard meets a connector that answers a timeout, its marker.

use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use knx_core::commissioning::LoadPart;
use knx_core::{
    CommissioningState, CompletionStatus, DeviceId, DeviceInstance, Installation, InstallationId,
    Language, Project, SourceRef, Topology,
};
use knx_net::BusError;
use knx_server::BusSessionError;
use knx_testsupport::vendor_plugin::{
    program_xml, HARDWARE2PROGRAM_REF, HARDWARE_XML, PRODUCT_REF, TARGET,
};
use serde_json::{json, Value};
use tower::ServiceExt;

/// The five flags as bits 0..5 of `bits`, in schema order.
fn flags(bits: u8) -> CommissioningState {
    let mut state = CommissioningState::default();
    for (index, part) in LoadPart::ALL.into_iter().enumerate() {
        state.set_intends_loaded(part, bits & (1 << index) != 0);
    }
    state
}

fn project(state: CommissioningState) -> Project {
    let mut project = Project::new(Language("en".into()));
    project.installations.push(Installation {
        id: InstallationId(0),
        name: "I".into(),
        default_line: None,
        multicast_address: None,
        completion: CompletionStatus::FinishedDesign,
        topology: Topology {
            areas: vec![],
            lines: vec![],
            unassigned: vec![DeviceId(1)],
        },
        buildings: vec![],
        group_ranges: vec![],
        group_addresses: vec![],
        parameters: vec![],
    });
    project.devices.insert(DeviceInstance {
        id: DeviceId(1),
        source: SourceRef {
            path: "synthetic".into(),
            ets_id: "DEV-1".into(),
        },
        name: "Synthetic actuator".into(),
        description: None,
        address: Some(TARGET.parse().unwrap()),
        product_ref: PRODUCT_REF.into(),
        program_ref: HARDWARE2PROGRAM_REF.into(),
        commissioning: state,
        visibility_calculated: true,
        com_objects: vec![],
        binary_data: vec![],
    });
    project
}

/// A server with the synthetic program installed and `bits` as flags.
fn server(dir: &std::path::Path, bits: u8) -> Arc<knx_server::AppState> {
    server_with(dir, bits, true, never_connects())
}

/// A synthetic connector with no discovered gateways.
fn never_connects() -> Box<dyn knx_server::GatewayConnector> {
    Box::new(knx_server::fake::FakeConnector::discovering(Vec::new()))
}

/// A connector that answers one `start` with a timeout: proof that the
/// start got past every check before the tunnel, and nothing was written.
fn times_out() -> Box<dyn knx_server::GatewayConnector> {
    Box::new(knx_server::fake::FakeConnector::failing(
        BusSessionError::Transport(BusError::Timeout),
    ))
}

/// [`server`] with or without the product database, and `connector`.
fn server_with(
    dir: &std::path::Path,
    bits: u8,
    with_products: bool,
    connector: Box<dyn knx_server::GatewayConnector>,
) -> Arc<knx_server::AppState> {
    let state = {
        // A durable activity history under `dir`: a start that passes every
        // check records its run there before the connector is asked.
        let mut state = knx_server::AppState::new(dir.to_path_buf());
        if with_products {
            let products = knx_productdb::open_and_migrate(&dir.join("products.sqlite")).unwrap();
            knx_productdb::ingest_file(&products, "M-FFF6/A.xml", program_xml("").as_bytes())
                .unwrap();
            knx_productdb::ingest_file(&products, "M-FFF6/Hardware.xml", HARDWARE_XML.as_bytes())
                .unwrap();
            state.product_db = Some(
                knx_server::CatalogueOwner::adopt_opened(products, std::path::PathBuf::new())
                    .unwrap(),
            );
        }
        state.connector = connector;
        *state.project.lock().unwrap() = Some(project(flags(bits)));
        state
    };
    Arc::new(state)
}

/// Replaces the device's flags, as a programming mark between plan and start would.
fn mark(state: &knx_server::AppState, bits: u8) {
    state
        .project
        .lock()
        .unwrap()
        .as_mut()
        .unwrap()
        .devices
        .get_mut(DeviceId(1))
        .unwrap()
        .commissioning = flags(bits);
}

fn flags_now(state: &knx_server::AppState) -> CommissioningState {
    state
        .project
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .devices
        .get(DeviceId(1))
        .unwrap()
        .commissioning
        .clone()
}

/// Plans `changes` at `before`, marks `after`, and expects the start
/// refused by the scope guard: before any tunnel, run or project mutation.
async fn assert_changes_plan_goes_stale(before: u8, planned: &str, after: u8) {
    let dir = tempfile::tempdir().unwrap();
    let state = server(dir.path(), before);
    let (status, plan) = post(&state, "/api/device-download/plan", changes()).await;
    assert_eq!(status, StatusCode::OK, "{plan}");
    assert_eq!(plan["scope"], planned, "{plan}");
    mark(&state, after);
    let (status, body) = post(
        &state,
        "/api/device-download/start",
        start_body(&plan["planId"]),
    )
    .await;
    let case = format!("{before:05b} -> {after:05b}: {body}");
    assert_eq!(status, StatusCode::CONFLICT, "{case}");
    assert_eq!(body["kind"], "downloadPlanStale", "{case}");
    // The scope guard's refusal, not the plan-equality one.
    assert!(
        body["error"]
            .as_str()
            .unwrap()
            .contains("the pending parts changed"),
        "{case}"
    );
    assert!(
        state.device_download.try_lock().unwrap().is_none(),
        "{case}"
    );
    assert_eq!(flags_now(&state), flags(after), "{case}");
}

/// Plans `explicit` at `before`, marks `after`, and expects the start past
/// the scope guard: it reaches the connector, which times out.
async fn assert_explicit_plan_ignores_the_flags(explicit: Value, before: u8, after: u8) {
    let dir = tempfile::tempdir().unwrap();
    let state = server_with(dir.path(), before, true, times_out());
    let (status, plan) = post(&state, "/api/device-download/plan", explicit).await;
    assert_eq!(status, StatusCode::OK, "{plan}");
    assert_eq!(plan["scopeFromChanges"], false, "{plan}");
    mark(&state, after);
    let (status, body) = post(
        &state,
        "/api/device-download/start",
        start_body(&plan["planId"]),
    )
    .await;
    let case = format!("{before:05b} -> {after:05b}: {body}");
    assert_eq!(status, StatusCode::BAD_GATEWAY, "{case}");
    assert_eq!(body["kind"], "gatewayTimeout", "{case}");
    assert!(
        state.device_download.try_lock().unwrap().is_none(),
        "{case}"
    );
}

async fn post(state: &Arc<knx_server::AppState>, uri: &str, body: Value) -> (StatusCode, Value) {
    let response = knx_server::app(state.clone(), None)
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(uri)
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    (
        status,
        serde_json::from_slice(&bytes)
            .unwrap_or(Value::String(String::from_utf8_lossy(&bytes).into_owned())),
    )
}

async fn get(state: &Arc<knx_server::AppState>, uri: &str) -> (StatusCode, Value) {
    let response = knx_server::app(state.clone(), None)
        .oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    (status, serde_json::from_slice(&bytes).unwrap())
}

fn changes() -> Value {
    json!({ "address": TARGET, "scope": "changes" })
}

fn start_body(plan_id: &Value) -> Value {
    json!({
        "planId": plan_id,
        "gateway": "192.0.2.10:3671",
        "confirmation": format!("I confirm download to {TARGET}"),
        "acceptUntested": format!("I accept an untested download to {TARGET}"),
    })
}

#[tokio::test]
async fn changes_plans_the_advised_scope_for_every_flag_combination() {
    for bits in 0u8..32 {
        let dir = tempfile::tempdir().unwrap();
        let state = server(dir.path(), bits);
        let (status, body) = post(&state, "/api/device-download/plan", changes()).await;
        let case = format!("{bits:05b}: {body}");
        let advice = &body["scopeAdvice"];
        let application = bits & 0b00010 != 0;
        let parameters = bits & 0b00100 != 0;
        let group_addresses = bits & 0b01000 != 0;
        let write = match (application, parameters, group_addresses) {
            (false, _, _) => "complete",
            (true, false, false) => "both",
            (true, false, true) => "parameters",
            (true, true, false) => "groupAddresses",
            (true, true, true) => "nothing",
        };
        assert_eq!(advice["write"], write, "{case}");
        assert_eq!(advice["basis"], "projectFlags", "{case}");
        assert_eq!(
            advice["address"],
            if bits & 1 == 0 { "pending" } else { "current" },
            "{case}"
        );
        assert_eq!(advice["mediumPending"], bits & 0b10000 == 0, "{case}");
        assert_eq!(advice["fullyProgrammed"], bits == 0b11111, "{case}");
        if write == "nothing" {
            assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{case}");
            assert_eq!(body["kind"], "downloadNothingPending", "{case}");
            continue;
        }
        assert_eq!(status, StatusCode::OK, "{case}");
        assert_eq!(body["scopeFromChanges"], true, "{case}");
        assert_eq!(body["partial"], write != "complete", "{case}");
        assert_eq!(body["scope"], write, "{case}");
        // The same steps an explicit request for the resolved scope plans.
        let explicit = match write {
            "complete" => json!({ "address": TARGET }),
            other => json!({ "address": TARGET, "partial": {
                "parameters": other == "parameters" || other == "both",
                "groupAddresses": other == "groupAddresses" || other == "both",
            }}),
        };
        let (status, explicit) = post(&state, "/api/device-download/plan", explicit).await;
        assert_eq!(status, StatusCode::OK, "{case}");
        assert_eq!(explicit["steps"], body["steps"], "{case}");
        assert_eq!(explicit["scopeFromChanges"], false, "{case}");
        assert_eq!(explicit["scope"], write, "{case}");
    }
}

#[tokio::test]
async fn an_explicit_request_keeps_its_scope_and_still_carries_the_advice() {
    let dir = tempfile::tempdir().unwrap();
    // Everything marked programmed: the explicit complete download is
    // still planned, with its normal confirmation.
    let state = server(dir.path(), 0b11111);
    let (status, body) = post(
        &state,
        "/api/device-download/plan",
        json!({ "address": TARGET }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["partial"], false);
    assert_eq!(body["scope"], "complete");
    assert_eq!(body["scopeAdvice"]["write"], "nothing");
    assert_eq!(body["scopeAdvice"]["fullyProgrammed"], true);
    assert_eq!(
        body["confirmationPhrase"],
        format!("I confirm download to {TARGET}")
    );
}

#[tokio::test]
async fn nothing_pending_builds_no_plan_and_drops_the_shown_one() {
    let dir = tempfile::tempdir().unwrap();
    // Address and medium pending, application side programmed.
    let state = server(dir.path(), 0b01110);
    let (status, shown) = post(
        &state,
        "/api/device-download/plan",
        json!({ "address": TARGET }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{shown}");
    let (status, body) = post(&state, "/api/device-download/plan", changes()).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{body}");
    assert_eq!(body["kind"], "downloadNothingPending");
    assert_eq!(body["scopeAdvice"]["write"], "nothing");
    assert_eq!(body["scopeAdvice"]["address"], "pending");
    assert_eq!(body["scopeAdvice"]["mediumPending"], true);
    assert_eq!(body["scopeAdvice"]["fullyProgrammed"], false);
    assert!(body.get("planId").is_none(), "{body}");
    assert!(state.device_download_plan.lock().unwrap().is_none());
    // The complete plan shown before cannot be started after the answer.
    let (status, body) = post(
        &state,
        "/api/device-download/start",
        start_body(&shown["planId"]),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT, "{body}");
    assert_eq!(body["kind"], "downloadPlanStale");
}

#[tokio::test]
async fn changes_and_an_explicit_partial_together_are_refused() {
    let dir = tempfile::tempdir().unwrap();
    let state = server(dir.path(), 0);
    let (status, body) = post(
        &state,
        "/api/device-download/plan",
        json!({ "address": TARGET, "scope": "changes",
                "partial": { "parameters": true, "groupAddresses": false } }),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
    assert_eq!(body["kind"], "downloadScopeConflict");
    assert!(state.device_download_plan.lock().unwrap().is_none());
    let (status, _) = post(
        &state,
        "/api/device-download/plan",
        json!({ "address": TARGET, "scope": "everything" }),
    )
    .await;
    assert!(status.is_client_error(), "{status}");
}

#[tokio::test]
async fn a_changes_plan_whose_flags_moved_is_stale_before_any_tunnel() {
    // Parameters pending only, then the group addresses too. The plan
    // itself would be identical; the advice it was bound to is not.
    assert_changes_plan_goes_stale(0b11011, "parameters", 0b10011).await;
    // The operator marks the device programmed after planning: nothing
    // is pending, and nothing is written in its place.
    assert_changes_plan_goes_stale(0b11011, "parameters", 0b11111).await;
    assert_changes_plan_goes_stale(0b11101, "complete", 0b11111).await;
    // The application is marked loaded after a complete plan: only the
    // parameters are advised now.
    assert_changes_plan_goes_stale(0b11001, "complete", 0b11011).await;
}

#[tokio::test]
async fn an_explicit_plan_starts_past_the_scope_guard_whatever_the_flags_say() {
    let partial = |parameters: bool, group_addresses: bool| {
        json!({ "address": TARGET,
                "partial": { "parameters": parameters, "groupAddresses": group_addresses } })
    };
    assert_explicit_plan_ignores_the_flags(json!({ "address": TARGET }), 0b11111, 0b11101).await;
    assert_explicit_plan_ignores_the_flags(partial(true, false), 0b11011, 0b11111).await;
    assert_explicit_plan_ignores_the_flags(partial(true, true), 0b11111, 0b10001).await;
}

#[tokio::test]
async fn nothing_pending_needs_no_product_data_and_nothing_else_skips_it() {
    let dir = tempfile::tempdir().unwrap();
    // Application side programmed, no product database: the honest no-op.
    let state = server_with(dir.path(), 0b01110, false, never_connects());
    let (status, body) = post(&state, "/api/device-download/plan", changes()).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{body}");
    assert_eq!(body["kind"], "downloadNothingPending");
    assert_eq!(body["scopeAdvice"]["write"], "nothing");
    assert_eq!(body["scopeAdvice"]["address"], "pending");
    assert_eq!(body["scopeAdvice"]["mediumPending"], true);
    // Something pending, or an explicit scope, still needs the product
    // data, and says so.
    for (bits, request) in [
        (0b11011, changes()),
        (0b11111, json!({ "address": TARGET })),
    ] {
        let state = server_with(dir.path(), bits, false, never_connects());
        let (status, body) = post(&state, "/api/device-download/plan", request).await;
        assert_eq!(status, StatusCode::CONFLICT, "{bits:05b}: {body}");
        assert_eq!(
            body["kind"], "downloadProductDataMissing",
            "{bits:05b}: {body}"
        );
        assert!(state.device_download_plan.lock().unwrap().is_none());
    }
}

#[tokio::test]
async fn with_a_project_open_and_no_product_data_a_bad_target_is_named_first() {
    // Documented precedence: the project resolves the device before the
    // product database is asked for. Every answer refuses; nothing is
    // planned, connected or recorded, and no flag moves.
    let dir = tempfile::tempdir().unwrap();
    for ambiguous in [false, true] {
        let state = server_with(dir.path(), 0b11011, false, never_connects());
        let address = if ambiguous {
            let mut project = state.project.lock().unwrap();
            let project = project.as_mut().unwrap();
            let mut twin = project.devices.get(DeviceId(1)).unwrap().clone();
            twin.id = DeviceId(2);
            project.devices.insert(twin);
            TARGET
        } else {
            "1.1.99"
        };
        for request in [
            json!({ "address": address }),
            json!({ "address": address, "scope": "changes" }),
            json!({ "address": address,
                    "partial": { "parameters": true, "groupAddresses": false } }),
        ] {
            let case = format!("ambiguous {ambiguous}, {request}");
            let (status, body) = post(&state, "/api/device-download/plan", request).await;
            assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{case}: {body}");
            assert_eq!(body["kind"], "downloadPreparationRefused", "{case}: {body}");
            assert!(
                body["error"].as_str().unwrap().contains(address),
                "{case}: {body}"
            );
            assert!(
                state.device_download_plan.lock().unwrap().is_none(),
                "{case}"
            );
            assert!(
                state.device_download.try_lock().unwrap().is_none(),
                "{case}"
            );
            assert_eq!(flags_now(&state), flags(0b11011), "{case}");
        }
        let (_, activity) = get(&state, "/api/bus/activity").await;
        assert_eq!(activity["oneShot"], json!([]), "{activity}");
    }
}
