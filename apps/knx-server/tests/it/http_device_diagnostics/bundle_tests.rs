//! Real acquisition-to-bundle readbacks for retained diagnostic result handoffs.

use super::*;
use std::{collections::BTreeMap, io::Read};

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
    // Numbers of an unproven namespace are never compared: no 409, only the own report.
    plant_device_8(&h);
    assert_own_report_only(&h, &acquisition["token"], 8, &result, "old run").await;
}

#[tokio::test]
async fn a_replaced_project_never_refuses_or_borrows_by_device_number() {
    for (case, incarnation, revision) in [
        ("incarnation+revision", true, true),
        ("incarnation", true, false),
        ("revision", false, true),
    ] {
        for chosen in [7, 8] {
            let h = harness("1.1.25");
            let acquisition = diagnostic_preview(&h).await;
            let result = start_and_finish(&h, &acquisition).await;
            assert!(h
                .state
                .recordings
                .wait_until_idle(std::time::Duration::from_secs(10)));
            plant_device_8(&h);
            plant_foreign_device_7(&h);
            {
                let _project = h.state.project.lock().unwrap();
                if incarnation {
                    h.state.project_incarnation.fetch_add(1, Ordering::SeqCst);
                }
                if revision {
                    h.state.project_revision.fetch_add(1, Ordering::SeqCst);
                }
            }
            let what = format!("replaced {case}, chosen {chosen}");
            assert_own_report_only(&h, &acquisition["token"], chosen, &result, &what).await;
        }
    }
}

#[tokio::test]
async fn missing_or_foreign_recorded_attribution_never_refuses_or_borrows_by_device_number() {
    for case in ["missing", "server", "incarnation", "revision"] {
        for chosen in [7, 8] {
            let h = harness("1.1.25");
            let acquisition = diagnostic_preview(&h).await;
            let result = start_and_finish(&h, &acquisition).await;
            assert!(h
                .state
                .recordings
                .wait_until_idle(std::time::Duration::from_secs(10)));
            profile_preview(&h, IDENTITY).await; // Evict the job: the recording is the source.
            reattribute_recording(&h, case);
            plant_device_8(&h);
            plant_foreign_device_7(&h);
            let what = format!("recorded {case}, chosen {chosen}");
            assert_own_report_only(&h, &acquisition["token"], chosen, &result, &what).await;
        }
    }
}

#[tokio::test]
async fn a_proven_current_run_refuses_another_device_and_exports_only_its_own() {
    for evicted in [false, true] {
        let h = harness("1.1.25");
        let acquisition = diagnostic_preview(&h).await;
        let result = start_and_finish(&h, &acquisition).await;
        assert!(h
            .state
            .recordings
            .wait_until_idle(std::time::Duration::from_secs(10)));
        if evicted {
            profile_preview(&h, IDENTITY).await;
        }
        plant_device_8(&h);
        let reference =
            json!({"relatedKey":"diagnosticsToken","relatedValue":acquisition["token"]});
        let mut query = json!({"problemType":"deviceReading","operation":reference,"deviceId":8,
            "sections":OFFERED});
        let (status, body) = send(&h.app, PREVIEW, query.clone()).await;
        assert_eq!(status, StatusCode::CONFLICT, "evicted {evicted}: {body}");
        assert_eq!(body["error"], "bundleTargetMismatch");
        assert_no_foreign(body.to_string().as_bytes(), "proven mismatch body");
        query["deviceId"] = json!(7);
        let (status, preview) = send(&h.app, PREVIEW, query.clone()).await;
        assert_eq!(status, StatusCode::OK, "evicted {evicted}: {preview}");
        for kind in ["device", "catalogue", "legacyReports", "operation"] {
            let row = offer(&preview, kind);
            assert_eq!(row["selected"], true, "evicted {evicted}: {kind}: {row}");
            assert_eq!(row["unavailable"], Value::Null, "{kind}");
        }
        query["review"] = preview["review"].clone();
        query["consent"] = json!(true);
        let entries = zip_entries(&h, query).await;
        let device: Value = serde_json::from_slice(&entries["device/7/device.json"]).unwrap();
        assert!(device.to_string().contains("CUSTOMER_SECRET"), "{device}");
        let catalogue: Value =
            serde_json::from_slice(&entries["catalogue/catalogue.json"]).unwrap();
        assert!(
            catalogue["deviceProductIdentity"].is_object(),
            "{catalogue}"
        );
        assert_eq!(report_entry(&entries, "report.json"), result["report"]);
        assert_eq!(
            report_entry(&entries, "properties.json"),
            result["report"]["items"]
        );
        for (name, bytes) in &entries {
            let text = String::from_utf8_lossy(bytes);
            assert!(!text.contains("FOREIGN_DEVICE_8_SENTINEL"), "{name}");
        }
    }
}

const PREVIEW: &str = "/api/diagnostics/bundle/preview";
const OFFERED: [&str; 5] = [
    "device",
    "catalogue",
    "legacyReports",
    "operation",
    "environment",
];
const FOREIGN: [&str; 5] = [
    "FOREIGN_PROJECT_SENTINEL",
    "FOREIGN_DEVICE_7_SENTINEL",
    "FOREIGN_DEVICE_8_SENTINEL",
    "CUSTOMER_SECRET",
    "PRIVATE_DESCRIPTION",
];

fn offer<'a>(preview: &'a Value, kind: &str) -> &'a Value {
    preview["offers"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["kind"] == kind)
        .unwrap()
}

fn assert_no_foreign(bytes: &[u8], what: &str) {
    let text = String::from_utf8_lossy(bytes);
    for sentinel in FOREIGN {
        assert!(!text.contains(sentinel), "{what} leaks {sentinel}");
    }
}

/// A second device of the current project; counters stay, so the context is unchanged.
fn plant_device_8(h: &Harness) {
    let mut project = h.state.project.lock().unwrap();
    let devices = &mut project.as_mut().unwrap().devices;
    let mut eight = devices.get_mut(DeviceId(7)).unwrap().clone();
    eight.id = DeviceId(8);
    eight.name = "FOREIGN_DEVICE_8_SENTINEL".into();
    eight.description = Some("FOREIGN_DEVICE_8_SENTINEL".into());
    eight.address = Some("1.1.26".parse().unwrap());
    devices.insert(eight);
}

fn plant_foreign_device_7(h: &Harness) {
    let mut project = h.state.project.lock().unwrap();
    let seven = project
        .as_mut()
        .unwrap()
        .devices
        .get_mut(DeviceId(7))
        .unwrap();
    seven.name = "FOREIGN_DEVICE_7_SENTINEL".into();
    seven.description = Some("FOREIGN_DEVICE_7_SENTINEL".into());
}

/// Rewrites the recording header's project attribution; the job must already be evicted.
fn reattribute_recording(h: &Harness, case: &str) {
    let recording = h
        .state
        .recordings
        .list()
        .unwrap()
        .into_iter()
        .next()
        .unwrap();
    let mut lines: Vec<Value> = h
        .state
        .recordings
        .read(&recording.id)
        .unwrap()
        .split(|b| *b == b'\n')
        .filter(|line| !line.is_empty())
        .map(|line| serde_json::from_slice(line).unwrap())
        .collect();
    let related = lines[0]["related"].as_object_mut().unwrap();
    let mut context: Value =
        serde_json::from_str(related["projectContext"].as_str().unwrap()).unwrap();
    match case {
        "missing" => {
            related.remove("projectContext");
        }
        "server" => context["serverIncarnation"] = json!("another-server-run"),
        "incarnation" => {
            context["projectIncarnation"] =
                json!(context["projectIncarnation"].as_u64().unwrap() + 1)
        }
        "revision" => context["revision"] = json!(context["revision"].as_u64().unwrap() + 1),
        _ => unreachable!("{case}"),
    }
    if case != "missing" {
        related.insert("projectContext".into(), json!(context.to_string()));
    }
    let bytes: Vec<u8> = lines
        .into_iter()
        .flat_map(|line| (line.to_string() + "\n").into_bytes())
        .collect();
    std::fs::write(
        h._dir
            .path()
            .join("diagnostics")
            .join(format!("{}.jsonl", recording.id)),
        bytes,
    )
    .unwrap();
}

async fn zip_entries(h: &Harness, query: Value) -> BTreeMap<String, Vec<u8>> {
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
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(bytes)).unwrap();
    (0..archive.len())
        .map(|index| {
            let mut file = archive.by_index(index).unwrap();
            let mut bytes = Vec::new();
            file.read_to_end(&mut bytes).unwrap();
            (file.name().to_string(), bytes)
        })
        .collect()
}

fn report_entry(entries: &BTreeMap<String, Vec<u8>>, name: &str) -> Value {
    serde_json::from_slice(&entries[&format!("reports/{name}")]).unwrap()
}

/// With an unproven or foreign attribution the exact retained report stays downloadable
/// whatever device number is chosen, while device and catalogue stay unavailable.
async fn assert_own_report_only(
    h: &Harness,
    token: &Value,
    chosen: u32,
    expected: &Value,
    what: &str,
) {
    let reference = json!({"relatedKey":"diagnosticsToken","relatedValue":token});
    let mut query = json!({"problemType":"deviceReading","operation":reference,
        "deviceId":chosen,"sections":OFFERED});
    let (status, preview) = send(&h.app, PREVIEW, query.clone()).await;
    assert_eq!(status, StatusCode::OK, "{what}: {preview}");
    for kind in ["device", "catalogue"] {
        let row = offer(&preview, kind);
        assert_eq!(row["selected"], false, "{what}: {kind}: {row}");
        assert!(row["unavailable"].is_string(), "{what}: {kind}: {row}");
    }
    for kind in ["legacyReports", "operation", "environment"] {
        let row = offer(&preview, kind);
        assert_eq!(row["selected"], true, "{what}: {kind}: {row}");
        assert_eq!(row["unavailable"], Value::Null, "{what}: {kind}");
    }
    let sections = preview["sections"].as_array().unwrap();
    assert!(sections
        .iter()
        .all(|s| s["kind"] != "device" && s["kind"] != "catalogue"));
    assert!(sections.iter().any(|s| s["kind"] == "legacyReports"));
    query["review"] = preview["review"].clone();
    query["consent"] = json!(true);
    let entries = zip_entries(h, query.clone()).await;
    assert!(entries
        .keys()
        .all(|name| !name.starts_with("device/") && !name.starts_with("catalogue/")));
    assert!(!expected["report"]["items"].as_array().unwrap().is_empty());
    assert_eq!(
        report_entry(&entries, "report.json"),
        expected["report"],
        "{what}"
    );
    assert_eq!(
        report_entry(&entries, "properties.json"),
        expected["report"]["items"],
        "{what}"
    );
    for (name, bytes) in &entries {
        assert_no_foreign(bytes, &format!("{what}: {name}"));
    }
    for section in sections {
        let kind: knx_app::diagnostic_bundle::SectionKind =
            serde_json::from_value(section["kind"].clone()).unwrap();
        let directory = match section["label"].as_str() {
            Some(label) => format!("{}/{label}", kind.directory()),
            None => kind.directory().to_string(),
        };
        let mut text_query = query.clone();
        text_query["key"] = section["key"].clone();
        let (status, text) = send(&h.app, "/api/diagnostics/bundle/section", text_query).await;
        assert_eq!(status, StatusCode::OK, "{what}: {text}");
        let text = text["text"].as_str().unwrap();
        for file in section["files"].as_array().unwrap() {
            let path = format!("{directory}/{}", file.as_str().unwrap());
            assert!(
                text.contains(&format!("===== {path} =====\n")),
                "{what}: {path}"
            );
        }
        assert_no_foreign(text.as_bytes(), what);
    }
    for key in [format!("device:{chosen}"), "catalogue".to_string()] {
        let mut text_query = query.clone();
        text_query["key"] = json!(key);
        let (status, body) = send(&h.app, "/api/diagnostics/bundle/section", text_query).await;
        assert_ne!(status, StatusCode::OK, "{what}: {key}: {body}");
        assert_no_foreign(body.to_string().as_bytes(), what);
    }
    assert_eq!(
        h.calls.load(Ordering::SeqCst),
        1,
        "{what}: must not reacquire"
    );
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
