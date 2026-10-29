//! Carries real exact0700 worker results through HTTP status, report and reopened activity history.

use super::*;
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use http_body_util::BodyExt;
use tower::ServiceExt;

async fn get(app: &axum::Router, path: &str) -> Value {
    let response = app
        .clone()
        .oneshot(Request::builder().uri(path).body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK, "{path}");
    serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes()).unwrap()
}

#[tokio::test]
async fn exact0700_http_finished_report_and_reopened_history_keep_the_actual_property_fact() {
    for (raw, expected, runtime) in [
        (
            Some([0, 0, 1, 0]),
            json!({"status":"observed", "objectIndex":3, "state":"running"}),
            "running",
        ),
        (
            Some([0x40, 0, 0, 0]),
            json!({"status":"notQualified", "reason":"descriptionProblem"}),
            "unknown",
        ),
        (
            Some([2, 0, 1, 0]),
            json!({"status":"notQualified", "reason":"unsupportedDatatype", "pdt":2}),
            "unknown",
        ),
        (
            Some([0x80, 0, 0, 0]),
            json!({"status":"notQualified", "reason":"indeterminateMetadata"}),
            "unknown",
        ),
        (
            Some([0, 0x10, 1, 0]),
            json!({"status":"notQualified", "reason":"unqualifiedElements"}),
            "unknown",
        ),
        (
            None,
            json!({"status":"unavailable", "step":"description", "cause":"noCorrelatedAnswer"}),
            "unknown",
        ),
    ] {
        let device = Arc::new(SimulatedDevice::with_config(SimulatorConfig {
            mask_version: 0x0700,
            ..SimulatorConfig::default()
        }));
        device.preset_property(0, PID_MANUFACTURER_ID, &[0, 0x83]);
        device.preset_property(0, PID_HARDWARE_TYPE, &HARDWARE);
        device.preset_property(3, PID_PROGRAM_VERSION, &APPLICATION);
        device.preset_property(3, 1, &[0, 3]);
        device.preset_property(3, 6, &[1]);
        if let Some(raw) = raw {
            device.preset_property_description(3, 6, 0x27, raw);
        }
        let mut plan = plan();
        plan.mask = MaskVersion(0x0700);
        let mut started = start_receipted(
            &device,
            plan,
            AfterRestart::Pass,
            None,
            Arc::new(DownloadStop::new()),
            |_| {},
        );
        started.session.join().await;
        let result = serde_json::to_value(started.session.status()).unwrap();
        assert_eq!(result["state"], "finished");
        assert_eq!(result["written"], "yes");
        assert_eq!(result["restart"], "acknowledged");
        let check = result["postDownloadCheck"].clone();
        assert_eq!(check["runtime"], runtime);
        assert_eq!(
            check["observation"]["reach"]["identity"]["runStateMask"],
            "0700"
        );
        assert_eq!(
            check["observation"]["reach"]["identity"]["runState"],
            expected
        );
        assert_check_read_only(&device);
        assert_eq!(device.authorize_requests(), 0);
        assert_eq!(restarts(&device), 1);
        let row = reopened(&started.history);
        assert_eq!(row["downloadEvidence"]["postDownloadCheck"], check);
        assert_eq!(
            row["downloadEvidence"]["settlement"]["project"],
            "noProject"
        );
        let state = Arc::new(crate::AppState {
            one_shot_activity: Arc::new(OneShotLog::for_run(started.history.clone()).unwrap()),
            ..crate::AppState::default()
        });
        *state.device_download.lock().await = Some(started.session);
        let app = crate::app(Arc::clone(&state), None);
        let frames = device.seen();
        let status = get(&app, "/api/device-download/status?since=0").await;
        assert_eq!(status["status"], result);
        let report = get(&app, "/api/device-download/report").await;
        assert_eq!(report["report"]["postDownloadCheck"], check);
        assert!(report["summary"]
            .as_str()
            .unwrap()
            .contains(if runtime == "running" {
                "run state: running"
            } else {
                "unknown"
            }));
        let history = get(&app, "/api/bus/history").await;
        assert_eq!(history["entries"].as_array().unwrap().len(), 1);
        assert_eq!(
            history["entries"][0]["downloadEvidence"]["postDownloadCheck"],
            check
        );
        assert_eq!(
            device.seen(),
            frames,
            "HTTP consumers add zero device traffic"
        );
        // Exact shared parser reopens the HTTP fact without raw descriptor metadata.
        let parsed: knx_app::post_download_check::PostDownloadCheck =
            serde_json::from_value(check.clone()).unwrap();
        assert_eq!(serde_json::to_value(parsed).unwrap(), check);
        state.device_download.lock().await.take();
    }
}
