//! Real acquisition-to-bundle readbacks for retained diagnostic result handoffs.

use super::*;
use std::io::Read;

pub(super) async fn bundle_readback(h: &Harness, token: &Value, expected: &Value) {
    let query = json!({"problemType":"deviceReading", "operation":{"relatedKey":"diagnosticsToken", "relatedValue":token}});
    let (status, preview) = send(&h.app, "/api/diagnostics/bundle/preview", query.clone()).await;
    assert_eq!(status, StatusCode::OK, "{preview}");
    let report = preview["offers"]
        .as_array()
        .unwrap()
        .iter()
        .find(|o| o["kind"] == "legacyReports")
        .unwrap();
    assert_eq!(report["unavailable"], Value::Null, "{preview}");
    let mut query = query;
    query["review"] = preview["review"].clone();
    query["consent"] = json!(true);
    let response = h
        .app
        .clone()
        .oneshot(request("/api/diagnostics/bundle", query.clone()))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let bytes = to_bytes(response.into_body(), 8 * 1024 * 1024)
        .await
        .unwrap();
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(bytes)).unwrap();
    if preview["offers"]
        .as_array()
        .unwrap()
        .iter()
        .any(|offer| offer["kind"] == "device" && !offer["unavailable"].is_null())
    {
        assert!(archive
            .file_names()
            .all(|name| !name.starts_with("device/")));
    }
    for section in preview["sections"].as_array().unwrap() {
        let kind: knx_app::diagnostic_bundle::SectionKind =
            serde_json::from_value(section["kind"].clone()).unwrap();
        let directory = match section["label"].as_str() {
            Some(label) => format!("{}/{label}", kind.directory()),
            None => kind.directory().to_string(),
        };
        let mut expected_text = String::new();
        let mut size = 0;
        for file in section["files"].as_array().unwrap() {
            let path = format!("{directory}/{}", file.as_str().unwrap());
            let mut bytes = Vec::new();
            archive
                .by_name(&path)
                .unwrap()
                .read_to_end(&mut bytes)
                .unwrap();
            size += bytes.len();
            expected_text.push_str(&format!("===== {path} =====\n"));
            expected_text.push_str(std::str::from_utf8(&bytes).unwrap());
            if !expected_text.ends_with('\n') {
                expected_text.push('\n');
            }
        }
        assert_eq!(section["bytes"], size);
        let mut text_query = query.clone();
        text_query["key"] = section["key"].clone();
        let (status, text) = send(&h.app, "/api/diagnostics/bundle/section", text_query).await;
        assert_eq!(status, StatusCode::OK, "{text}");
        assert_eq!(text["text"], expected_text);
    }
    for (name, value) in [
        ("report.json", &expected["report"]),
        ("properties.json", &expected["report"]["items"]),
    ] {
        let mut bytes = Vec::new();
        archive
            .by_name(&format!("reports/{name}"))
            .unwrap()
            .read_to_end(&mut bytes)
            .unwrap();
        assert_eq!(serde_json::from_slice::<Value>(&bytes).unwrap(), *value);
    }
    if !expected["memory"].is_null() {
        let mut bytes = Vec::new();
        archive
            .by_name("reports/memory.json")
            .unwrap()
            .read_to_end(&mut bytes)
            .unwrap();
        assert_eq!(
            serde_json::from_slice::<Value>(&bytes).unwrap(),
            expected["memory"]
        );
    }
    assert_eq!(
        h.calls.load(Ordering::SeqCst),
        1,
        "bundle must not reacquire"
    );
}

#[tokio::test]
async fn projectless_manual_support_result_is_exported_without_recording_or_another_read() {
    let h = harness_gated_recording("1.1.25", None, None, true);
    *h.state.project.lock().unwrap() = None;
    let (status, preview) = send(&h.app, "/api/support/preview", json!({"address":"1.1.25"})).await;
    assert_eq!(status, StatusCode::OK, "{preview}");
    assert_eq!(h.calls.load(Ordering::SeqCst), 0);
    let (status, completed) = send(
        &h.app,
        "/api/support/diagnose",
        json!({"token":preview["token"],"gateway":"192.0.2.10:3671","consent":true}),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{completed}");
    let mut query = json!({"problemType":"deviceReading","operation":{"relatedKey":"supportToken","relatedValue":preview["token"]}});
    let (status, preview) = send(&h.app, "/api/diagnostics/bundle/preview", query.clone()).await;
    assert_eq!(status, StatusCode::OK, "{preview}");
    assert_eq!(preview["operationRecordingId"], Value::Null);
    assert_eq!(preview["deviceId"], Value::Null);
    query["review"] = preview["review"].clone();
    query["consent"] = json!(true);
    let response = h
        .app
        .clone()
        .oneshot(request("/api/diagnostics/bundle", query))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let bytes = to_bytes(response.into_body(), 8 * 1024 * 1024)
        .await
        .unwrap();
    let mut zip = zip::ZipArchive::new(std::io::Cursor::new(bytes)).unwrap();
    let mut bytes = Vec::new();
    zip.by_name("reports/report.json")
        .unwrap()
        .read_to_end(&mut bytes)
        .unwrap();
    assert_eq!(
        serde_json::from_slice::<Value>(&bytes).unwrap(),
        completed["report"]
    );
    assert_eq!(h.calls.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn replacing_the_project_with_the_same_numeric_device_never_exports_foreign_bytes() {
    let h = harness("1.1.25");
    let query =
        json!({"problemType":"deviceReading","operation":{"selection":"none"},"deviceId":7});
    let (status, preview) = send(&h.app, "/api/diagnostics/bundle/preview", query.clone()).await;
    assert_eq!(status, StatusCode::OK, "{preview}");
    {
        let mut project = h.state.project.lock().unwrap();
        let mut replacement = project.as_ref().unwrap().clone();
        replacement.devices.get_mut(DeviceId(7)).unwrap().name = "FOREIGN_PROJECT_SENTINEL".into();
        h.state.project_incarnation.fetch_add(1, Ordering::SeqCst);
        h.state.project_revision.fetch_add(1, Ordering::SeqCst);
        *project = Some(replacement);
    }
    for uri in [
        "/api/diagnostics/bundle",
        "/api/diagnostics/bundle/section",
        "/api/diagnostics/bundle/preview",
    ] {
        let mut query = query.clone();
        query["review"] = preview["review"].clone();
        query["context"] = preview["review"]["context"].clone();
        query["consent"] = json!(true);
        query["key"] = json!("device:7");
        let (status, body) = send(&h.app, uri, query).await;
        assert_eq!(status, StatusCode::CONFLICT, "{uri}: {body}");
        assert_eq!(body["error"], "bundlePreviewChanged");
        assert!(!body.to_string().contains("FOREIGN_PROJECT_SENTINEL"));
    }
}

#[tokio::test]
async fn an_old_run_retains_its_report_but_never_borrows_the_new_projects_device() {
    let h = harness("1.1.25");
    let acquisition = diagnostic_preview(&h).await;
    let result = start_and_finish(&h, &acquisition).await;
    assert!(h
        .state
        .recordings
        .wait_until_idle(std::time::Duration::from_secs(10)));
    {
        let mut project = h.state.project.lock().unwrap();
        project
            .as_mut()
            .unwrap()
            .devices
            .get_mut(DeviceId(7))
            .unwrap()
            .name = "FOREIGN_PROJECT_SENTINEL".into();
        h.state.project_incarnation.fetch_add(1, Ordering::SeqCst);
        h.state.project_revision.fetch_add(1, Ordering::SeqCst);
    }
    let query = json!({"problemType":"deviceReading","operation":{"relatedKey":"diagnosticsToken","relatedValue":acquisition["token"]},"deviceId":7});
    let (_, preview) = send(&h.app, "/api/diagnostics/bundle/preview", query.clone()).await;
    let device = preview["offers"]
        .as_array()
        .unwrap()
        .iter()
        .find(|o| o["kind"] == "device")
        .unwrap();
    assert_eq!(device["selected"], false);
    assert!(device["unavailable"]
        .as_str()
        .unwrap()
        .contains("originating"));
    bundle_readback(&h, &acquisition["token"], &result).await;
    let (_, refusal) = send(
        &h.app,
        "/api/diagnostics/bundle/preview",
        json!({"problemType":"deviceReading","operation":{"selection":"none"},"deviceId":7}),
    )
    .await;
    assert_eq!(refusal["operationRecordingId"], Value::Null);
    let device = refusal["offers"]
        .as_array()
        .unwrap()
        .iter()
        .find(|o| o["kind"] == "device")
        .unwrap();
    assert_eq!(device["selected"], true);
    assert_eq!(device["unavailable"], Value::Null);
    let (status,body) = send(&h.app,"/api/diagnostics/bundle/preview",json!({"problemType":"deviceReading","operation":{"relatedKey":"diagnosticsToken","relatedValue":acquisition["token"]},"deviceId":8})).await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(body["error"], "bundleTargetMismatch");
}

#[tokio::test]
async fn unknown_recording_versions_and_malformed_typed_results_are_not_guessed() {
    let h = harness("1.1.25");
    let acquisition = diagnostic_preview(&h).await;
    start_and_finish(&h, &acquisition).await;
    assert!(h
        .state
        .recordings
        .wait_until_idle(std::time::Duration::from_secs(10)));
    let recording = h
        .state
        .recordings
        .list()
        .unwrap()
        .into_iter()
        .next()
        .unwrap();
    let original = h.state.recordings.read(&recording.id).unwrap();
    profile_preview(&h, IDENTITY).await; // Evict the job, requiring the historical typed reader.
    for version in [true, false] {
        let mut lines: Vec<Value> = original
            .split(|b| *b == b'\n')
            .filter(|line| !line.is_empty())
            .map(|line| serde_json::from_slice(line).unwrap())
            .collect();
        if version {
            lines[0]["format_version"] = json!(999);
        } else {
            lines.last_mut().unwrap()["summary"]["report"] = json!({"unknown":"not a report"});
        }
        let bytes = lines
            .into_iter()
            .map(|line| line.to_string() + "\n")
            .collect::<String>()
            .into_bytes();
        std::fs::write(
            h._dir
                .path()
                .join("diagnostics")
                .join(format!("{}.jsonl", recording.id)),
            &bytes,
        )
        .unwrap();
        let mut query = json!({"problemType":"deviceReading","operation":{"relatedKey":"diagnosticsToken","relatedValue":acquisition["token"]}});
        let (status, preview) =
            send(&h.app, "/api/diagnostics/bundle/preview", query.clone()).await;
        assert_eq!(status, StatusCode::OK, "{preview}");
        let reports = preview["offers"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["kind"] == "legacyReports")
            .unwrap();
        assert_eq!(
            reports["unavailable"],
            if version {
                "the diagnostic recording version is unsupported"
            } else {
                "the diagnostic recording result is malformed"
            }
        );
        query["review"] = preview["review"].clone();
        query["consent"] = json!(true);
        let response = h
            .app
            .clone()
            .oneshot(request("/api/diagnostics/bundle", query))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let zip_bytes = to_bytes(response.into_body(), 8 * 1024 * 1024)
            .await
            .unwrap();
        let mut zip = zip::ZipArchive::new(std::io::Cursor::new(zip_bytes)).unwrap();
        let mut raw = Vec::new();
        zip.by_name(&format!("operations/{}/recording.jsonl", recording.id))
            .unwrap()
            .read_to_end(&mut raw)
            .unwrap();
        assert_eq!(
            raw, bytes,
            "never backfill or rewrite historical raw recordings"
        );
        assert!(!zip.file_names().any(|name| name.starts_with("reports/")));
    }
}

#[tokio::test]
async fn completed_identity_and_memory_results_survive_disabled_and_deleted_recordings() {
    for profile in [IDENTITY, MEMORY] {
        for mode in ["recorded", "disabled", "deleted", "replaced-job"] {
            let products: Option<fn(&knx_productdb::Connection)> =
                (profile == MEMORY).then_some(|_| {});
            let h = harness_gated_recording("1.1.25", products, None, mode == "disabled");
            let p = profile_preview(&h, profile).await;
            let (status, _) = send(
                &h.app,
                "/api/device-diagnostics/start",
                json!({"token":p["token"],"consent":true}),
            )
            .await;
            assert_eq!(status, StatusCode::ACCEPTED);
            let result = finished(&h, p["token"].as_str().unwrap()).await;
            assert_eq!(result["exportable"], true, "{mode}: {result}");
            assert_eq!(result["report"]["items"][0]["bytes"], json!([0, 37]));
            assert!(h
                .state
                .recordings
                .wait_until_idle(std::time::Duration::from_secs(10)));
            if mode == "deleted" {
                assert_eq!(h.state.recordings.delete_all().unwrap().removed, 1);
            }
            if mode == "replaced-job" {
                profile_preview(&h, profile).await;
            }
            bundle_readback(&h, &p["token"], &result).await;
        }
    }
}
