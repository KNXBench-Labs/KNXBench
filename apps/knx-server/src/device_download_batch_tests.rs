//! Unit tests of the batch decision and of the reservation it holds between its devices.

use super::*;
use axum::body::Body;
use axum::http::Request;
use serde_json::{json, Value};
use tower::ServiceExt;

fn check(value: Value) -> PostDownloadCheck {
    serde_json::from_value(value).expect("a valid stored check")
}

fn matched(verdict: &str) -> Value {
    // The machines the stored verdict must agree with.
    let observed = match verdict {
        "matchesPlan" => json!({ "result": "observed", "loadState": "loaded" }),
        "differsFromPlan" => json!({ "result": "observed", "loadState": "unloaded" }),
        _ => json!({ "result": "unavailable" }),
    };
    json!({
        "observation": { "status": "attempted", "reach": { "result": "reached",
            "identity": { "result": "match", "exemplar": "notVerified",
                "loadStates": [{ "machine": "applicationProgram", "expected": "loaded",
                    "observed": observed }],
                "planLoadVerdict": verdict } } },
        "runtime": "unknown"
    })
}

fn observation(observation: Value) -> Value {
    json!({ "observation": observation, "runtime": "unknown" })
}

fn reached(identity: Value) -> Value {
    observation(
        json!({ "status": "attempted", "reach": { "result": "reached", "identity": identity } }),
    )
}

fn finished(written: Written, restart: Restart, check_value: Value) -> DownloadStatus {
    DownloadStatus::Finished {
        written,
        restart,
        restart_note: None,
        post_download_check: check(check_value),
    }
}

/// ADR-0172 §5: only a written, restarted and fully matching device lets the
/// next one start; every other outcome, the unknown ones included, stops.
#[test]
fn only_the_full_match_continues_and_every_other_outcome_names_its_doubt() {
    let full = finished(Written::Yes, Restart::Acknowledged, matched("matchesPlan"));
    let (state, go_on) = judge(5, full, None);
    assert!(go_on);
    assert!(
        matches!(state, DeviceState::Done { download_id: 5, .. }),
        "{state:?}"
    );
    let not_in_plan = finished(Written::Yes, Restart::NotInPlan, matched("matchesPlan"));
    assert!(
        judge(5, not_in_plan, None).1,
        "a plan without restart may continue"
    );

    let cases: Vec<(DownloadStatus, Doubt)> = vec![
        (
            finished(Written::Yes, Restart::Unconfirmed, matched("matchesPlan")),
            Doubt::RestartUnconfirmed,
        ),
        (
            finished(
                Written::Partially,
                Restart::Acknowledged,
                matched("matchesPlan"),
            ),
            Doubt::NotWritten,
        ),
        (
            finished(
                Written::Yes,
                Restart::Acknowledged,
                matched("differsFromPlan"),
            ),
            Doubt::LoadStateDiffers,
        ),
        (
            finished(Written::Yes, Restart::Acknowledged, matched("unknown")),
            Doubt::LoadStateUnknown,
        ),
        (
            finished(
                Written::Yes,
                Restart::Acknowledged,
                observation(json!({ "status": "notChecked" })),
            ),
            Doubt::CheckNotChecked,
        ),
        (
            finished(
                Written::Yes,
                Restart::Acknowledged,
                observation(json!({ "status": "unsupported", "reason": "applicationUnknown" })),
            ),
            Doubt::CheckUnsupported,
        ),
        (
            finished(
                Written::Yes,
                Restart::Acknowledged,
                observation(json!({ "status": "notPerformed" })),
            ),
            Doubt::CheckNotPerformed,
        ),
        (
            finished(
                Written::Yes,
                Restart::Acknowledged,
                observation(json!({ "status": "interrupted" })),
            ),
            Doubt::CheckInterrupted,
        ),
        (
            finished(
                Written::Yes,
                Restart::Acknowledged,
                observation(json!({ "status": "attempted", "reach": { "result": "notReached" } })),
            ),
            Doubt::CheckNotReached,
        ),
        (
            finished(
                Written::Yes,
                Restart::Acknowledged,
                observation(json!({ "status": "attempted", "reach": { "result": "unknown" } })),
            ),
            Doubt::CheckReachUnknown,
        ),
        (
            finished(
                Written::Yes,
                Restart::Acknowledged,
                reached(json!({ "result": "unavailable", "part": { "kind": "application" } })),
            ),
            Doubt::IdentityUnavailable,
        ),
        (
            finished(
                Written::Yes,
                Restart::Acknowledged,
                reached(
                    json!({ "result": "mismatch", "part": { "kind": "manufacturer" },
                    "kind": "differs" }),
                ),
            ),
            Doubt::IdentityMismatch,
        ),
    ];
    for (status, expected) in cases {
        let (state, go_on) = judge(9, status, None);
        assert!(!go_on, "{expected:?} must stop the batch");
        match state {
            DeviceState::Unconfirmed { doubt, .. } => assert_eq!(doubt, expected),
            other => panic!("{expected:?}: {other:?}"),
        }
    }
}

#[test]
fn a_failed_or_stopped_device_stops_the_batch_and_keeps_its_backup() {
    let failed = DownloadStatus::Failed {
        written: Written::Partially,
        stopped_in_step: Some(3),
        error: "connection lost".into(),
        hint: None,
        failure: crate::device_download::synthetic_failure(Some(3)),
    };
    let (state, go_on) = judge(4, failed, Some("device-backups/x.json".into()));
    assert!(!go_on);
    match state {
        DeviceState::Failed {
            written,
            stopped_in_step,
            backup_file,
            ..
        } => {
            assert_eq!(written, Written::Partially);
            assert_eq!(stopped_in_step, Some(3));
            assert_eq!(backup_file.as_deref(), Some("device-backups/x.json"));
        }
        other => panic!("{other:?}"),
    }

    let mut stopped_failure = crate::device_download::synthetic_failure(None);
    stopped_failure.code = FailureCode::Stopped;
    let stopped = DownloadStatus::Failed {
        written: Written::No,
        stopped_in_step: None,
        error: "stopped".into(),
        hint: None,
        failure: stopped_failure,
    };
    let (state, go_on) = judge(4, stopped, None);
    assert!(!go_on);
    assert!(
        matches!(state, DeviceState::Stopped { download_id: 4, .. }),
        "{state:?}"
    );
}

async fn send(state: &SharedState, request: Request<Body>) -> (StatusCode, Value) {
    let response = crate::app(Arc::clone(state), None)
        .oneshot(request)
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

fn post(uri: &str, body: Value) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri(uri)
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .unwrap()
}

/// A running batch between two devices: no download session runs, and still
/// every tunnel route refuses, naming the batch (ADR-0172 §4).
#[tokio::test]
async fn between_two_devices_the_batch_keeps_the_gateway_and_is_named() {
    let dir = tempfile::tempdir().unwrap();
    let state: SharedState = Arc::new(crate::AppState::new(dir.path().to_path_buf()));
    let target: IndividualAddress = "1.1.7".parse().unwrap();
    *state.device_download_batch.lock().unwrap() =
        Some(Arc::new(DownloadBatch::running_for_test(40, target)));
    let expected_holder = json!({
        "kind": "deviceDownloadBatch", "batchId": 40, "address": "1.1.7", "position": 1, "total": 1
    });
    assert_eq!(
        serde_json::to_value(crate::tunnel_holder::download_holder(&state)).unwrap(),
        expected_holder
    );
    assert!(running(&state));

    // A single plan shown beside the batch cannot start.
    *state.device_download_plan.lock().unwrap() = Some(ShownDownload::Single(crate::ShownPlan {
        id: 7,
        target,
        plan: crate::device_download::synthetic_plan(),
        partial: None,
        changes: false,
    }));
    let (status, body) = send(
        &state,
        post(
            "/api/device-download/start",
            json!({ "planId": 7, "gateway": "192.0.2.10:3671",
                "confirmation": "I confirm download to 1.1.7" }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT, "{body}");
    assert_eq!(body["kind"], "gatewayBusy", "{body}");
    assert_eq!(body["holder"], expected_holder, "{body}");
    assert!(
        body["error"]
            .as_str()
            .unwrap()
            .contains("batch download is running"),
        "{body}"
    );

    let (status, body) = send(
        &state,
        post(
            "/api/bus/monitor/start",
            json!({ "gateway": "192.0.2.10:3671" }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT, "{body}");
    assert_eq!(body["holder"], expected_holder, "{body}");
    // The graceful stop waits for it as running device work.
    assert!(crate::graceful_stop::device_writes_active_for_test(&state));

    let (status, body) = send(
        &state,
        post("/api/device-download/batch/stop", json!({ "batchId": 40 })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["stop"], "afterThisDevice");

    // Once it ends, it holds nothing.
    let batch = state.device_download_batch.lock().unwrap().clone().unwrap();
    batch.end(NotStartedReason::StopRequest);
    assert!(crate::tunnel_holder::download_holder(&state).is_none());
    assert!(!running(&state));
    let (status, body) = send(
        &state,
        Request::get("/api/device-download/batch/status?batchId=40")
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["state"], "ended");
    assert_eq!(body["stopRequested"], true);
    assert_eq!(body["devices"][0]["state"], "notStarted");
    assert_eq!(body["devices"][0]["reason"], "stopRequest");
}
