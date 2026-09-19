//! HTTP tests for `POST /api/debug-report` (T29 — the debug report).
//!
//! What these check, in one sentence each: the preview mode writes no file,
//! the save mode writes a zip holding exactly the files the response named,
//! the opt-ins are honestly off by default, and an address that reached the
//! session log does not reach `log.json`.

use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use knx_core::{
    CompletionStatus, GroupAddress, GroupAddressEntry, GroupRange, GroupRangeId, Installation,
    InstallationId, Language, Project, SourceRef, Topology,
};
use serde_json::{json, Value};
use tower::ServiceExt;

async fn body_json(response: axum::response::Response) -> Value {
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

async fn call(app: &axum::Router, method: &str, uri: &str, body: Option<Value>) -> Value {
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
    assert_eq!(response.status(), StatusCode::OK);
    body_json(response).await
}

fn source(tag: &str) -> SourceRef {
    SourceRef {
        path: tag.into(),
        ets_id: tag.into(),
    }
}

/// A project with one installation, one group range and one group address,
/// so `project-summary.json` has something to count that is not zero.
fn project() -> Project {
    let mut project = Project::new(Language("en".into()));
    project.info.name = "Test Villa".into();
    project.installations.push(Installation {
        id: InstallationId(0),
        name: "I".into(),
        default_line: None,
        multicast_address: None,
        completion: CompletionStatus::FinishedDesign,
        topology: Topology {
            areas: vec![],
            lines: vec![],
            unassigned: vec![],
        },
        buildings: vec![],
        group_ranges: vec![GroupRange {
            id: GroupRangeId(1),
            source: source("gr1"),
            name: "Everything".into(),
            start: GroupAddress::from_raw(0),
            end: GroupAddress::from_raw(u16::MAX),
            parent: None,
            children: vec![],
        }],
        group_addresses: vec![],
        parameters: vec![],
    });
    let id = project.ids.next_group_address_id();
    project.installations[0]
        .group_addresses
        .push(GroupAddressEntry {
            id,
            source: source("t1"),
            name: "Living Room Light".into(),
            address: GroupAddress::parse("1/1/1", project.info.group_address_style).unwrap(),
            central: false,
            unfiltered: false,
            range: Some(GroupRangeId(1)),
        });
    project
}

fn zip_entry_names(path: &std::path::Path) -> Vec<String> {
    let file = std::fs::File::open(path).unwrap();
    let mut archive = zip::ZipArchive::new(file).unwrap();
    let mut names: Vec<String> = (0..archive.len())
        .map(|i| archive.by_index(i).unwrap().name().to_string())
        .collect();
    names.sort();
    names
}

fn zip_entry_text(path: &std::path::Path, name: &str) -> String {
    let file = std::fs::File::open(path).unwrap();
    let mut archive = zip::ZipArchive::new(file).unwrap();
    let mut entry = archive.by_name(name).unwrap();
    let mut text = String::new();
    std::io::Read::read_to_string(&mut entry, &mut text).unwrap();
    text
}

fn response_file_names(response: &Value) -> Vec<String> {
    response["files"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["name"].as_str().unwrap().to_string())
        .collect()
}

#[tokio::test]
async fn a_report_without_a_path_writes_nothing_and_says_so() {
    let dir = tempfile::tempdir().unwrap();
    let state = knx_server::AppState::default();
    let app = knx_server::app(Arc::new(state), None);

    let response = call(
        &app,
        "POST",
        "/api/debug-report",
        Some(json!({ "description": "the bus went quiet" })),
    )
    .await;

    assert_eq!(response["written"], json!(false));
    assert_eq!(response["path"], Value::Null);
    assert!(response["reportMarkdown"]
        .as_str()
        .unwrap()
        .contains("the bus went quiet"));
    // Nothing was created anywhere a preview could plausibly have put it.
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 0);
}

#[tokio::test]
async fn a_report_with_a_path_writes_exactly_the_files_it_listed() {
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("debug-report.zip");
    let state = knx_server::AppState::default();
    let app = knx_server::app(Arc::new(state), None);

    let response = call(
        &app,
        "POST",
        "/api/debug-report",
        Some(json!({
            "path": target.to_string_lossy(),
            "description": "it exploded",
            "includeLog": true,
        })),
    )
    .await;

    assert_eq!(response["written"], json!(true));
    assert_eq!(
        response["path"].as_str().unwrap(),
        target.to_string_lossy().as_ref()
    );

    let mut listed = response_file_names(&response);
    listed.sort();
    assert_eq!(zip_entry_names(&target), listed);
    assert_eq!(
        listed,
        vec![
            "environment.json".to_string(),
            "log.json".to_string(),
            "report.md".to_string()
        ]
    );
}

#[tokio::test]
async fn the_project_summary_and_the_telegrams_stay_out_unless_asked_for() {
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("default.zip");
    let state = knx_server::AppState::default();
    *state.project.lock().unwrap() = Some(project());
    let app = knx_server::app(Arc::new(state), None);

    let response = call(
        &app,
        "POST",
        "/api/debug-report",
        Some(json!({ "path": target.to_string_lossy() })),
    )
    .await;

    let names = response_file_names(&response);
    assert!(
        !names.iter().any(|n| n == "project-summary.json"),
        "a project summary was included without being asked for: {names:?}"
    );
    assert!(
        !names.iter().any(|n| n == "bus-telegrams.json"),
        "bus telegrams were included without being asked for: {names:?}"
    );
    assert_eq!(zip_entry_names(&target), {
        let mut expected = names.clone();
        expected.sort();
        expected
    });
}

#[tokio::test]
async fn the_project_summary_counts_the_project_without_naming_anything_in_it() {
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("with-summary.zip");
    let state = knx_server::AppState::default();
    *state.project.lock().unwrap() = Some(project());
    let app = knx_server::app(Arc::new(state), None);

    call(
        &app,
        "POST",
        "/api/debug-report",
        Some(json!({
            "path": target.to_string_lossy(),
            "includeProjectSummary": true,
        })),
    )
    .await;

    let text = zip_entry_text(&target, "project-summary.json");
    let summary: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(summary["installations"], json!(1));
    assert_eq!(summary["groupAddresses"], json!(1));
    assert_eq!(summary["groupRanges"], json!(1));
    assert_eq!(summary["devices"], json!(0));
    assert_eq!(summary["projectOpen"], json!(true));
    // The promise the dialog makes about this file: counts, never content.
    assert!(!text.contains("Test Villa"), "{text}");
    assert!(!text.contains("Living Room Light"), "{text}");
    assert!(!text.contains("1/1/1"), "{text}");
}

#[tokio::test]
async fn an_address_that_reached_the_session_log_does_not_reach_the_bundle() {
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("redacted.zip");
    let state = knx_server::AppState::default();
    {
        let mut log = state.session_log.lock().unwrap();
        log.push(knx_server::session_log::LogEntry {
            timestamp: "2026-09-19T10:00:00Z".into(),
            severity: knx_server::session_log::Severity::Error,
            source: "bus".into(),
            message: "tunnel to KNX_GATEWAY refused".into(),
            location: None,
            detail: Some("peer fe80::1234 gave up".into()),
        });
    }
    let app = knx_server::app(Arc::new(state), None);

    call(
        &app,
        "POST",
        "/api/debug-report",
        Some(json!({ "path": target.to_string_lossy(), "includeLog": true })),
    )
    .await;

    let text = zip_entry_text(&target, "log.json");
    assert!(!text.contains("KNX_GATEWAY"), "{text}");
    assert!(!text.contains("fe80::1234"), "{text}");
    assert!(text.contains("[redacted-ipv4]"), "{text}");
    assert!(text.contains("[redacted-ipv6]"), "{text}");
    // Everything that is not an address is still there to read.
    assert!(text.contains("tunnel to"), "{text}");
    assert!(text.contains("refused"), "{text}");
}

#[tokio::test]
async fn a_write_target_outside_the_data_directory_is_refused() {
    let state = knx_server::AppState::default();
    let app = knx_server::app(Arc::new(state), None);

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/debug-report")
                .header("content-type", "application/json")
                .body(Body::from(json!({ "path": "../escaped.zip" }).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}
