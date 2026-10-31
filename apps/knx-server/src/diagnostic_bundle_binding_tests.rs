//! HTTP regression tests binding review, section text and ZIP to the same collected bytes.

use super::tests::*;
use axum::http::StatusCode;
use knx_app::operation_recording::OperationKind;
use serde_json::{json, Value};
use std::{io::Read, sync::Arc};

struct EnvironmentConsumers {
    fingerprint: String,
    preview: Value,
    section: Value,
    text: String,
    json: Value,
    zip_entries: Vec<(String, Vec<u8>)>,
}

fn environment_state(
    dir: &std::path::Path,
    incarnation: &str,
    return_path: knx_net::TunnelReturnPath,
    unavailable_reason: &str,
) -> Arc<crate::AppState> {
    let mut state = crate::AppState {
        recordings: Arc::new(crate::operation_recordings::store(dir, false)),
        data_dir: dir.to_path_buf(),
        tunnel_return_path: return_path,
        product_db_unavailable: Some(unavailable_reason.to_string()),
        ..Default::default()
    };
    state.server_incarnation = incarnation.to_string();
    Arc::new(state)
}

async fn environment_consumers(state: &Arc<crate::AppState>) -> EnvironmentConsumers {
    let query = json!({"problemType":"userInterface", "sections":["environment"]});
    let preview = preview_query(state, &query).await;
    let fingerprint = preview["review"]["fingerprint"]
        .as_str()
        .unwrap()
        .to_string();
    let mut section_query = reviewed(&query, &preview);
    section_query["key"] = json!("environment");
    let (status, section, _) = call(
        state,
        "POST",
        "/api/diagnostics/bundle/section",
        Some(section_query),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{}", json_of(&section));
    let section = json_of(&section);
    let text = section["text"].as_str().unwrap().to_string();
    let (status, bundle, _) = call(
        state,
        "POST",
        "/api/diagnostics/bundle",
        Some(reviewed(&query, &preview)),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{}", json_of(&bundle));
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(bundle)).unwrap();
    let mut zip_entries = Vec::new();
    for index in 0..archive.len() {
        let mut entry = archive.by_index(index).unwrap();
        let name = entry.name().to_string();
        let mut bytes = Vec::new();
        entry.read_to_end(&mut bytes).unwrap();
        zip_entries.push((name, bytes));
    }
    let raw = zip_entries
        .iter()
        .find(|(name, _)| name == "environment/environment.json")
        .map(|(_, bytes)| bytes)
        .unwrap();
    EnvironmentConsumers {
        fingerprint,
        preview,
        section,
        text,
        json: serde_json::from_slice(raw).unwrap(),
        zip_entries,
    }
}

fn assert_environment_sentinels_absent(consumers: &EnvironmentConsumers, sentinels: &[&str]) {
    let public_responses = format!(
        "{}\n{}\n{}",
        consumers.preview, consumers.section, consumers.text
    );
    for sentinel in sentinels {
        assert!(
            !public_responses.contains(sentinel),
            "response leaked {sentinel}"
        );
        for (name, bytes) in &consumers.zip_entries {
            assert!(
                !String::from_utf8_lossy(bytes).contains(sentinel),
                "ZIP entry {name} leaked {sentinel}"
            );
        }
    }
}

async fn preview_query(state: &Arc<crate::AppState>, query: &Value) -> Value {
    let (status, body, _) = call(
        state,
        "POST",
        "/api/diagnostics/bundle/preview",
        Some(query.clone()),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{}", json_of(&body));
    let view = json_of(&body);
    let fingerprint = view["review"]["fingerprint"]
        .as_str()
        .expect("mandatory fingerprint");
    assert_eq!(fingerprint.len(), 64);
    assert!(fingerprint
        .bytes()
        .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)));
    assert_eq!(
        view["review"]["context"]["serverIncarnation"],
        state.server_incarnation
    );
    view
}

fn reviewed(query: &Value, preview: &Value) -> Value {
    let mut query = query.clone();
    // The fallback deliberately reaches the original collector, calibrating semantic drift.
    query["review"] = preview.get("review").cloned().unwrap_or(json!({}));
    query["consent"] = json!(true);
    query
}

async fn changed(state: &Arc<crate::AppState>, query: Value) {
    for uri in ["/api/diagnostics/bundle", "/api/diagnostics/bundle/section"] {
        let mut query = query.clone();
        query["key"] = json!("environment");
        let (status, body, disposition) = call(state, "POST", uri, Some(query)).await;
        assert_eq!(status, StatusCode::CONFLICT, "{uri}: {}", json_of(&body));
        assert!(String::from_utf8_lossy(&body).contains("bundlePreviewChanged"));
        assert_eq!(disposition, None);
        assert!(!body.starts_with(b"PK"));
    }
}

async fn unchanged(state: &Arc<crate::AppState>, query: Value) {
    let (status, bytes, _) = call(
        state,
        "POST",
        "/api/diagnostics/bundle",
        Some(query.clone()),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{}", json_of(&bytes));
    assert!(bytes.starts_with(b"PK"));
    let mut section = query;
    section["key"] = json!("environment");
    let (status, _, _) = call(
        state,
        "POST",
        "/api/diagnostics/bundle/section",
        Some(section),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
async fn product_database_failure_text_is_noninterfering_across_environment_consumers() {
    const RAW_FLAG_ONE: &str = "false PRIVATE_ROUTE_FLAG=/home/alice/private-one";
    const RAW_FLAG_TWO: &str = "off PRIVATE_ROUTE_FLAG=/home/alice/private-two";
    const DB_ONE: &str = "PRIVATE_DB_REASON=/home/alice/catalogue-one.sqlite";
    const DB_TWO: &str = "PRIVATE_DB_REASON=/home/alice/catalogue-two.sqlite";
    let one_dir = tempfile::tempdir().unwrap();
    let two_dir = tempfile::tempdir().unwrap();
    let one = environment_state(
        one_dir.path(),
        "environment-contract",
        crate::tunnel_return_path_from_flag(Some(RAW_FLAG_ONE)),
        DB_ONE,
    );
    let two = environment_state(
        two_dir.path(),
        "environment-contract",
        crate::tunnel_return_path_from_flag(Some(RAW_FLAG_TWO)),
        DB_TWO,
    );

    let one = environment_consumers(&one).await;
    let two = environment_consumers(&two).await;

    assert_eq!(one.fingerprint, two.fingerprint);
    assert_eq!(one.text, two.text);
    assert_eq!(one.json, two.json);
    assert_eq!(
        one.json["productDatabase"],
        json!({"availability":"unavailable"})
    );
    assert_environment_sentinels_absent(&one, &[RAW_FLAG_ONE, DB_ONE, DB_TWO]);
    assert_environment_sentinels_absent(&two, &[RAW_FLAG_TWO, DB_ONE, DB_TWO]);
    assert!(!one.json.to_string().contains("unavailableReason"));
}

#[tokio::test]
async fn effective_route_back_changes_preview_text_fingerprint_and_zip() {
    const RAW_ROUTE_FLAG: &str = "PRIVATE_ROUTE_BACK=/home/alice/route-back";
    let local_dir = tempfile::tempdir().unwrap();
    let route_dir = tempfile::tempdir().unwrap();
    let local = environment_state(
        local_dir.path(),
        "environment-contract",
        crate::tunnel_return_path_from_flag(None),
        "same unavailable fact",
    );
    let route = environment_state(
        route_dir.path(),
        "environment-contract",
        crate::tunnel_return_path_from_flag(Some(RAW_ROUTE_FLAG)),
        "same unavailable fact",
    );

    let local = environment_consumers(&local).await;
    let route = environment_consumers(&route).await;

    assert_ne!(local.fingerprint, route.fingerprint);
    assert_ne!(local.text, route.text);
    assert_ne!(local.json, route.json);
    assert_eq!(local.json["settings"]["tunnelRouteBack"], false);
    assert_eq!(route.json["settings"]["tunnelRouteBack"], true);
    assert_environment_sentinels_absent(&route, &[RAW_ROUTE_FLAG]);
}

#[tokio::test]
async fn newer_matching_operation_cannot_replace_the_reviewed_run() {
    let dir = tempfile::tempdir().unwrap();
    let state = state(dir.path(), true);
    let first = record(&state, OperationKind::Download, ("downloadId", "1"), true);
    let query = json!({"problemType":"deviceProgramming"});
    let preview = preview_query(&state, &query).await;
    assert_eq!(preview["operationRecordingId"], first);
    record(&state, OperationKind::Download, ("downloadId", "2"), true);
    changed(&state, reviewed(&query, &preview)).await;
}

#[tokio::test]
async fn project_revision_and_replacement_invalidate_review_before_device_lookup() {
    let dir = tempfile::tempdir().unwrap();
    let state = state(dir.path(), true);
    let query = json!({"problemType":"deviceReading", "deviceId":1});
    let preview = preview_query(&state, &query).await;
    {
        let _project = state.project.lock().unwrap();
        crate::domain::next_project_revision(&state);
    }
    changed(&state, reviewed(&query, &preview)).await;
    let preview = preview_query(&state, &query).await;
    {
        let _project = state.project.lock().unwrap();
        state
            .project_incarnation
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }
    changed(&state, reviewed(&query, &preview)).await;
}

#[tokio::test]
async fn unadmitted_client_bytes_are_noninterfering_and_description_is_withheld() {
    let dir = tempfile::tempdir().unwrap();
    let state = state(dir.path(), true);
    let query = json!({"problemType":"userInterface", "client":{"recentErrors":["reviewed"]}});
    let preview = preview_query(&state, &query).await;
    let client = offer(&preview, "clientState");
    assert_eq!(client["selected"], false);
    assert_eq!(
        client["unavailable"],
        "this section is withheld until its content is admitted as free of credentials"
    );
    let mut altered = reviewed(&query, &preview);
    altered["client"]["recentErrors"] = json!(["unreviewed"]);
    unchanged(&state, altered).await;
    let mut query = reviewed(&query, &preview);
    query["description"] = json!("entered only after review");
    let (status, bytes, _) = call(&state, "POST", "/api/diagnostics/bundle", Some(query)).await;
    assert_eq!(status, StatusCode::OK, "{}", json_of(&bytes));
    let mut zip = zip::ZipArchive::new(std::io::Cursor::new(bytes)).unwrap();
    assert!(zip.by_name("client/client.json").is_err());
    let mut manifest = String::new();
    zip.by_name("manifest.json")
        .unwrap()
        .read_to_string(&mut manifest)
        .unwrap();
    assert!(manifest.contains("User description withheld from diagnostic bundle."));
    assert!(!manifest.contains("entered only after review"));
    assert!(!manifest.contains("reviewed"));
    assert!(!manifest.contains("unreviewed"));
}

#[tokio::test]
async fn run_target_cannot_be_replaced_with_an_explicit_foreign_device() {
    let dir = tempfile::tempdir().unwrap();
    let state = state(dir.path(), true);
    let recorder = crate::operation_recordings::start(
        &state,
        knx_app::operation_recording::OperationStart::new(OperationKind::Download)
            .target(json!({"deviceId":1, "individualAddress":"1.1.1"})),
    );
    recorder.finish(knx_app::operation_recording::RecordedOutcome::succeeded(
        Value::Null,
    ));
    assert!(state
        .recordings
        .wait_until_idle(std::time::Duration::from_secs(10)));
    let (status, body, _) = call(
        &state,
        "POST",
        "/api/diagnostics/bundle/preview",
        Some(json!({
            "problemType":"deviceProgramming", "deviceId":2,
            "operation":{"recordingId":recorder.id().unwrap()}
        })),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT, "{}", json_of(&body));
    assert!(String::from_utf8_lossy(&body).contains("bundleTargetMismatch"));
}

/// How a recording's project attribution relates to the project the server holds now.
#[derive(Clone, Copy, PartialEq, Debug)]
enum Provenance {
    Proven,
    Missing,
    ForeignServer,
    OtherIncarnation,
    OtherRevision,
}

const UNPROVEN: [Provenance; 4] = [
    Provenance::Missing,
    Provenance::ForeignServer,
    Provenance::OtherIncarnation,
    Provenance::OtherRevision,
];

/// Opens a project whose devices carry sentinel names; the counters stay untouched.
fn open_project(state: &Arc<crate::AppState>, devices: &[(u32, &str)]) {
    let mut project = knx_core::Project::new(knx_core::Language("en".into()));
    for (id, address) in devices {
        project.devices.insert(knx_core::DeviceInstance {
            id: knx_core::DeviceId(*id),
            source: knx_core::SourceRef {
                path: "PRIVATE_PATH".into(),
                ets_id: format!("PRIVATE_SOURCE_{id}"),
            },
            name: format!("FOREIGN_DEVICE_{id}_SENTINEL"),
            description: Some(format!("FOREIGN_DEVICE_{id}_SENTINEL")),
            address: Some(address.parse().unwrap()),
            product_ref: "PRIVATE_PRODUCT".into(),
            program_ref: "PRIVATE_PROGRAM".into(),
            commissioning: Default::default(),
            visibility_calculated: true,
            com_objects: vec![],
            binary_data: vec![],
        });
    }
    *state.project.lock().unwrap() = Some(project);
}

/// Records a finished run with `target` and the project attribution `provenance` describes.
fn recorded_target(state: &Arc<crate::AppState>, target: Value, provenance: Provenance) -> String {
    use crate::diagnostic_bundle_context::ProjectContext;
    let current = ProjectContext::capture(state);
    let mut start =
        knx_app::operation_recording::OperationStart::new(OperationKind::Download).target(target);
    let stated = match provenance {
        Provenance::Proven | Provenance::Missing => None,
        Provenance::ForeignServer => Some(ProjectContext {
            server_incarnation: "another-server-run".into(),
            ..current
        }),
        Provenance::OtherIncarnation => Some(ProjectContext {
            project_incarnation: current.project_incarnation + 1,
            ..current
        }),
        Provenance::OtherRevision => Some(ProjectContext {
            revision: current.revision + 1,
            ..current
        }),
    };
    if let Some(stated) = stated {
        start = start.related("projectContext", serde_json::to_string(&stated).unwrap());
    }
    // A holder of the project lock cannot sample the context: the recording stays unattributed.
    let held = (provenance == Provenance::Missing).then(|| state.project.lock().unwrap());
    let recorder = crate::operation_recordings::start(state, start);
    drop(held);
    recorder.finish(knx_app::operation_recording::RecordedOutcome::succeeded(
        Value::Null,
    ));
    assert!(state
        .recordings
        .wait_until_idle(std::time::Duration::from_secs(10)));
    recorder.id().unwrap().to_string()
}

fn offer<'a>(view: &'a Value, kind: &str) -> &'a Value {
    view["offers"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["kind"] == kind)
        .unwrap()
}

/// Every ZIP entry of the reviewed export, as text.
async fn exported_entries(
    state: &Arc<crate::AppState>,
    query: &Value,
    preview: &Value,
) -> std::collections::BTreeMap<String, String> {
    let (status, bytes, _) = call(
        state,
        "POST",
        "/api/diagnostics/bundle",
        Some(reviewed(query, preview)),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{}", json_of(&bytes));
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(bytes)).unwrap();
    (0..archive.len())
        .map(|index| {
            let mut file = archive.by_index(index).unwrap();
            let mut bytes = Vec::new();
            file.read_to_end(&mut bytes).unwrap();
            (
                file.name().to_string(),
                String::from_utf8_lossy(&bytes).into_owned(),
            )
        })
        .collect()
}

/// The run is exported by itself, but neither device nor catalogue extract is offered.
async fn assert_extracts_unavailable(
    state: &Arc<crate::AppState>,
    query: &Value,
    id: &str,
    what: &str,
) {
    let preview = preview_query(state, query).await;
    assert_eq!(preview["operationRecordingId"], id, "{what}");
    for kind in ["device", "catalogue"] {
        let row = offer(&preview, kind);
        assert_eq!(row["selected"], false, "{what}: {kind}: {row}");
        assert!(row["unavailable"].is_string(), "{what}: {kind}: {row}");
    }
    assert!(preview["sections"]
        .as_array()
        .unwrap()
        .iter()
        .all(|s| s["kind"] != "device" && s["kind"] != "catalogue"));
    assert!(!preview.to_string().contains("FOREIGN_DEVICE_"), "{what}");
    let entries = exported_entries(state, query, &preview).await;
    // Owner decision 2026-10-31: the run's redacted raw recording is exported by itself.
    assert!(
        entries.keys().any(|name| name.starts_with("operations/")),
        "{what}"
    );
    for (name, text) in &entries {
        assert!(
            !name.starts_with("device/") && !name.starts_with("catalogue/"),
            "{what}: {name}"
        );
        assert!(!text.contains("FOREIGN_DEVICE_"), "{what}: {name}");
    }
}

/// The device and the catalogue extract of `device` are offered, none of `other`'s bytes.
async fn assert_extracts_of(
    state: &Arc<crate::AppState>,
    query: &Value,
    device: u32,
    other: u32,
    what: &str,
) {
    let preview = preview_query(state, query).await;
    for kind in ["device", "catalogue"] {
        let row = offer(&preview, kind);
        assert_eq!(row["selected"], true, "{what}: {kind}: {row}");
        assert_eq!(row["unavailable"], Value::Null, "{what}: {kind}");
    }
    let entries = exported_entries(state, query, &preview).await;
    assert!(entries[&format!("device/{device}/device.json")]
        .contains(&format!("FOREIGN_DEVICE_{device}_SENTINEL")));
    assert!(entries.contains_key("catalogue/catalogue.json"), "{what}");
    for (name, text) in &entries {
        assert!(
            !text.contains(&format!("FOREIGN_DEVICE_{other}_SENTINEL")),
            "{what}: {name}"
        );
    }
}

async fn assert_target_mismatch(state: &Arc<crate::AppState>, query: &Value, what: &str) {
    let (status, body, _) = call(
        state,
        "POST",
        "/api/diagnostics/bundle/preview",
        Some(query.clone()),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT, "{what}: {}", json_of(&body));
    assert!(
        String::from_utf8_lossy(&body).contains("bundleTargetMismatch"),
        "{what}"
    );
}

fn extract_query(id: &str, explicit: bool, device: u32) -> Value {
    let mut query = json!({"problemType":"deviceProgramming","deviceId":device,
        "sections":["device","catalogue","operation"]});
    if explicit {
        query["operation"] = json!({"recordingId":id});
    }
    query
}

#[tokio::test]
async fn unproven_run_targets_never_conflict_and_never_offer_device_or_catalogue() {
    for provenance in UNPROVEN {
        for chosen in [1, 2] {
            for explicit in [true, false] {
                let dir = tempfile::tempdir().unwrap();
                let state = state(dir.path(), true);
                open_project(&state, &[(1, "1.1.1"), (2, "1.1.2")]);
                let target = json!({"deviceId":1, "individualAddress":"1.1.1"});
                let id = recorded_target(&state, target, provenance);
                let what = format!("{provenance:?}, chosen {chosen}, explicit {explicit}");
                assert_extracts_unavailable(
                    &state,
                    &extract_query(&id, explicit, chosen),
                    &id,
                    &what,
                )
                .await;
            }
        }
    }
}

#[tokio::test]
async fn a_proven_run_target_refuses_another_device_and_exports_only_its_own() {
    for explicit in [true, false] {
        let dir = tempfile::tempdir().unwrap();
        let state = state(dir.path(), true);
        open_project(&state, &[(1, "1.1.1"), (2, "1.1.2")]);
        let target = json!({"deviceId":1, "individualAddress":"1.1.1"});
        let id = recorded_target(&state, target, Provenance::Proven);
        let what = format!("explicit {explicit}");
        assert_target_mismatch(&state, &extract_query(&id, explicit, 2), &what).await;
        assert_extracts_of(&state, &extract_query(&id, explicit, 1), 1, 2, &what).await;
    }
}

#[tokio::test]
async fn address_only_targets_resolve_only_when_proven_and_unambiguous() {
    for explicit in [true, false] {
        let dir = tempfile::tempdir().unwrap();
        let state = state(dir.path(), true);
        open_project(&state, &[(1, "1.1.1"), (2, "1.1.2")]);
        let target = json!({"individualAddress":"1.1.1"});
        let id = recorded_target(&state, target.clone(), Provenance::Proven);
        let what = format!("proven address, explicit {explicit}");
        assert_target_mismatch(&state, &extract_query(&id, explicit, 2), &what).await;
        assert_extracts_of(&state, &extract_query(&id, explicit, 1), 1, 2, &what).await;
        for provenance in UNPROVEN {
            for chosen in [1, 2] {
                let dir = tempfile::tempdir().unwrap();
                let state = super::tests::state(dir.path(), true);
                open_project(&state, &[(1, "1.1.1"), (2, "1.1.2")]);
                let id = recorded_target(&state, target.clone(), provenance);
                let what = format!("{provenance:?} address, chosen {chosen}, explicit {explicit}");
                assert_extracts_unavailable(
                    &state,
                    &extract_query(&id, explicit, chosen),
                    &id,
                    &what,
                )
                .await;
            }
        }
        // Two devices share the address: nothing is guessed, in a proven context either.
        for chosen in [1, 2] {
            let dir = tempfile::tempdir().unwrap();
            let state = super::tests::state(dir.path(), true);
            open_project(&state, &[(1, "1.1.1"), (2, "1.1.1")]);
            let id = recorded_target(&state, target.clone(), Provenance::Proven);
            let what = format!("ambiguous address, chosen {chosen}, explicit {explicit}");
            assert_extracts_unavailable(&state, &extract_query(&id, explicit, chosen), &id, &what)
                .await;
        }
    }
}

#[tokio::test]
async fn a_proven_run_that_names_no_device_says_so_and_never_projects_the_chosen_one() {
    // A serial-number lookup records no project device: the explicit choice is
    // never projected next to it, and the reason says why, not "source unavailable".
    let target = json!({"serialNumber":"0083:12345678","write":false});
    for explicit in [true, false] {
        for chosen in [1, 2] {
            let dir = tempfile::tempdir().unwrap();
            let state = state(dir.path(), true);
            open_project(&state, &[(1, "1.1.1"), (2, "1.1.2")]);
            let id = recorded_target(&state, target.clone(), Provenance::Proven);
            let what = format!("no device, chosen {chosen}, explicit {explicit}");
            let query = extract_query(&id, explicit, chosen);
            let preview = preview_query(&state, &query).await;
            for kind in ["device", "catalogue"] {
                assert_eq!(
                    offer(&preview, kind)["unavailable"],
                    "the selected operation names no device of this project",
                    "{what}: {kind}"
                );
            }
            assert_extracts_unavailable(&state, &query, &id, &what).await;
        }
    }
    for provenance in UNPROVEN {
        let dir = tempfile::tempdir().unwrap();
        let state = state(dir.path(), true);
        open_project(&state, &[(1, "1.1.1"), (2, "1.1.2")]);
        let id = recorded_target(&state, target.clone(), provenance);
        let preview = preview_query(&state, &extract_query(&id, true, 1)).await;
        assert_eq!(
            offer(&preview, "device")["unavailable"],
            "the originating device source is unavailable",
            "{provenance:?}"
        );
    }
}

#[tokio::test]
async fn without_an_operation_the_chosen_current_device_is_unchanged() {
    let dir = tempfile::tempdir().unwrap();
    let state = state(dir.path(), true);
    open_project(&state, &[(1, "1.1.1"), (2, "1.1.2")]);
    let mut query = json!({"problemType":"deviceProgramming","deviceId":2,
        "sections":["device","catalogue"]});
    assert_extracts_of(&state, &query, 2, 1, "no recordings").await;
    recorded_target(&state, json!({"deviceId":1}), Provenance::ForeignServer);
    query["operation"] = json!({"selection":"none"});
    assert_extracts_of(&state, &query, 2, 1, "operation none").await;
}

#[tokio::test]
async fn unadmitted_session_log_is_noninterfering_but_selected_recording_deletion_is_bound() {
    let dir = tempfile::tempdir().unwrap();
    let state = state(dir.path(), true);
    let query = json!({"problemType":"deviceReading"});
    let preview = preview_query(&state, &query).await;
    state
        .session_log
        .lock()
        .unwrap()
        .push(crate::session_log::LogEntry {
            timestamp: crate::session_log::now(),
            severity: crate::session_log::Severity::Error,
            source: "test".into(),
            message: "UNREVIEWED_LOG_SENTINEL".into(),
            location: None,
            detail: None,
            diagnostic: None,
        });
    unchanged(&state, reviewed(&query, &preview)).await;
    let id = record(&state, OperationKind::Compare, ("x", "1"), true);
    let query = json!({"problemType":"deviceReading","operation":{"recordingId":id}});
    let preview = preview_query(&state, &query).await;
    assert_eq!(state.recordings.delete_all().unwrap().removed, 1);
    changed(&state, reviewed(&query, &preview)).await;
}

#[tokio::test]
async fn selected_recording_deletion_is_bound_independently_of_the_session_log() {
    let dir = tempfile::tempdir().unwrap();
    let state = state(dir.path(), true);
    let id = record(&state, OperationKind::Compare, ("x", "1"), true);
    let query = json!({"problemType":"deviceReading","operation":{"recordingId":id}});
    let preview = preview_query(&state, &query).await;
    assert_eq!(state.recordings.delete_all().unwrap().removed, 1);
    changed(&state, reviewed(&query, &preview)).await;
}

#[tokio::test]
async fn effective_identity_matches_when_client_pins_the_automatically_resolved_recording() {
    let dir = tempfile::tempdir().unwrap();
    let state = state(dir.path(), true);
    let id = record(&state, OperationKind::Compare, ("x", "1"), true);
    let query = json!({"problemType":"deviceReading"});
    let preview = preview_query(&state, &query).await;
    let mut pinned = reviewed(&query, &preview);
    pinned["operation"] = json!({"recordingId":id});
    let (status, _, _) = call(&state, "POST", "/api/diagnostics/bundle", Some(pinned)).await;
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
async fn context_from_another_server_is_refused_on_initial_preview_and_export() {
    let dir = tempfile::tempdir().unwrap();
    let one = state(dir.path(), true);
    let other = state(dir.path(), true);
    let query = json!({"problemType":"userInterface", "sections":["environment"]});
    let preview = preview_query(&one, &query).await;
    changed(&other, reviewed(&query, &preview)).await;
    let mut query = query;
    query["context"] = preview["review"]["context"].clone();
    let (status, body, _) = call(
        &other,
        "POST",
        "/api/diagnostics/bundle/preview",
        Some(query),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert!(String::from_utf8_lossy(&body).contains("bundlePreviewChanged"));
}

#[tokio::test]
async fn explicit_none_never_selects_a_previous_run_and_client_can_be_selected_again() {
    let dir = tempfile::tempdir().unwrap();
    let state = state(dir.path(), true);
    let id = record(&state, OperationKind::Download, ("downloadId", "1"), true);
    let query = json!({"problemType":"userInterface", "operation":{"selection":"none"},"sections":["environment"]});
    let view = preview_query(&state, &query).await;
    assert_eq!(view["operationRecordingId"], Value::Null);
    let client = view["offers"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["kind"] == "clientState")
        .unwrap();
    assert_eq!(
        client["unavailable"],
        "this section is withheld until its content is admitted as free of credentials"
    );
    assert_eq!(client["selected"], false);
    for reference in [
        json!({"selection":"none","recordingId":id}),
        json!({"selection":"latest"}),
    ] {
        let (status, body, _) = call(
            &state,
            "POST",
            "/api/diagnostics/bundle/preview",
            Some(json!({"problemType":"deviceProgramming","operation":reference})),
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert!(String::from_utf8_lossy(&body).contains("bundleOperationReferenceRefused"));
    }
}

#[tokio::test]
async fn unadmitted_bus_monitor_availability_is_noninterfering() {
    monitor_changes(false).await;
}

#[tokio::test]
async fn unadmitted_bus_monitor_content_is_noninterfering() {
    monitor_changes(true).await;
}

async fn monitor_changes(change_contents: bool) {
    use crate::bus::{
        fake::{FakeConnector, FakeTunnel},
        BusSession, GroupAddressContext,
    };
    use knx_net::{
        ApplicationService, Destination, LDataFrame, LDataMessageKind, Tpci, TunnelEvent,
    };
    let dir = tempfile::tempdir().unwrap();
    let state = state(dir.path(), true);
    let query = json!({"problemType":"deviceReading","sections":["environment","busMonitor"]});
    let absent = preview_query(&state, &query).await;
    let (tunnel, handle) = FakeTunnel::new("1.1.2".parse().unwrap(), 16);
    let connector = FakeConnector::succeeding(tunnel);
    let session = BusSession::start(
        1,
        "203.0.113.250:3671".parse().unwrap(),
        &connector,
        GroupAddressContext::from_project(None),
    )
    .await
    .unwrap();
    let buffer = session.buffer();
    *state.bus_session.lock().await = Some(session);
    if !change_contents {
        unchanged(&state, reviewed(&query, &absent)).await;
        let session = state.bus_session.lock().await.take().unwrap();
        session.stop().await;
        assert!(handle.disconnected());
        return;
    }
    let empty = preview_query(&state, &query).await;
    handle
        .sender()
        .send(TunnelEvent::Telegram(LDataFrame {
            kind: LDataMessageKind::Indication,
            source: "1.1.5".parse().unwrap(),
            destination: Destination::Group(knx_core::GroupAddress::from_raw(0x0a03)),
            transport: Tpci::UnnumberedData,
            service: ApplicationService::GroupValueRead,
            control: None,
        }))
        .unwrap();
    tokio::time::timeout(std::time::Duration::from_secs(2), async {
        loop {
            if !buffer.lock().unwrap().telegrams_since(0).is_empty() {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    unchanged(&state, reviewed(&query, &empty)).await;
    state.bus_session.lock().await.take().unwrap().stop().await;
    assert!(handle.disconnected());
}

#[tokio::test]
async fn a_context_change_during_collection_is_refused_before_any_owned_bytes_are_returned() {
    use axum::{extract::State, response::IntoResponse, Json};
    use std::{future::Future, task::Poll};
    let dir = tempfile::tempdir().unwrap();
    let state = state(dir.path(), true);
    let monitor = state.bus_session.lock().await;
    let query = serde_json::from_value(json!({"problemType":"deviceReading"})).unwrap();
    let mut pending = Box::pin(super::preview(State(state.clone()), Json(query)));
    // The real handler captures/validates context synchronously before its first await.
    // Holding monitor guarantees collection cannot finish in this first poll.
    let first = std::future::poll_fn(|cx| Poll::Ready(pending.as_mut().poll(cx))).await;
    assert!(first.is_pending());
    {
        let _project = state.project.lock().unwrap();
        crate::domain::next_project_revision(&state);
    }
    drop(monitor);
    let error = pending
        .await
        .err()
        .expect("context drift during collection must refuse");
    let response = error.into_response();
    assert_eq!(response.status(), StatusCode::CONFLICT);
    let bytes = axum::body::to_bytes(response.into_body(), 4096)
        .await
        .unwrap();
    assert!(String::from_utf8_lossy(&bytes).contains("bundlePreviewChanged"));
}

#[tokio::test]
async fn section_requires_review_and_malformed_binding_fails_closed() {
    let dir = tempfile::tempdir().unwrap();
    let state = state(dir.path(), true);
    let (status, body, _) = call(
        &state,
        "POST",
        "/api/diagnostics/bundle/section",
        Some(json!({"problemType":"deviceReading", "key":"environment"})),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(String::from_utf8_lossy(&body).contains("bundlePreviewRequired"));
    for review in [json!({}), json!({"fingerprint":"NOT-A-HASH", "context":{}})] {
        let (status, body, disposition) = call(
            &state,
            "POST",
            "/api/diagnostics/bundle",
            Some(json!({"problemType":"deviceReading", "consent":true,"review":review})),
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert!(String::from_utf8_lossy(&body).contains("bundlePreviewRequired"));
        assert_eq!(disposition, None);
    }
}
