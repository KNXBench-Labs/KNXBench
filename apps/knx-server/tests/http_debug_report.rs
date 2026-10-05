//! HTTP tests for `POST /api/debug-report` (T29 — the debug report).
//!
//! What these check, in one sentence each: the preview mode writes no file,
//! the save mode writes a zip holding exactly the files the response named,
//! the opt-ins are honestly off by default, an IP address that reached the
//! session log does not reach `log.json`, and the telegrams that are opted
//! in keep the KNX addresses and names the dialog promises they keep.

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
    let id = project.ids.next_group_address_id().unwrap();
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
            declared_dpt: Default::default(),
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
        log.push(knx_server::LogEntry {
            timestamp: "2026-09-19T10:00:00Z".into(),
            severity: knx_server::Severity::Error,
            source: "bus".into(),
            message: "tunnel to 203.0.113.4 refused".into(),
            location: None,
            diagnostic: None,
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
    assert!(!text.contains("203.0.113.4"), "{text}");
    assert!(!text.contains("fe80::1234"), "{text}");
    assert!(text.contains("[redacted-ipv4]"), "{text}");
    assert!(text.contains("[redacted-ipv6]"), "{text}");
    // Everything that is not an address is still there to read.
    assert!(text.contains("tunnel to"), "{text}");
    assert!(text.contains("refused"), "{text}");
}

#[tokio::test]
async fn a_relative_write_target_that_escapes_the_data_directory_is_refused() {
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

// ---------------------------------------------------------------------------
// The most privacy-sensitive file in the bundle, end to end: a telegram is
// fed through a fake tunnel, decoded against the open project, and must
// arrive in `bus-telegrams.json` with its addresses and its name intact.
// "Intact" is the whole point — the dialog promises exactly this, and a test
// that only checked the file existed would let a silent half-redaction pass.
// ---------------------------------------------------------------------------

fn device(number: u8) -> knx_core::IndividualAddress {
    knx_core::IndividualAddress::new(1, 1, number).expect("valid test address")
}

/// The fixture project plus one communication object linked to its group
/// address, so a telegram to `1/1/1` decodes to a value instead of stopping
/// at "no DPT resolved".
fn project_with_a_decodable_group_address() -> Project {
    let mut project = project();
    let ga = project.installations[0].group_addresses[0].id;
    project
        .devices
        .insert_com_object(knx_core::ComObjectInstance {
            id: knx_core::ComObjectInstanceId(1),
            source: source("co1"),
            device: knx_core::DeviceId(1),
            number: 0,
            text: knx_core::Override::Absent,
            description: knx_core::Override::Absent,
            dpt: knx_core::Override::Value(knx_core::Resolved {
                value: knx_core::DptRef {
                    main: 1,
                    sub: Some(1),
                },
                layer: knx_core::Layer::Instance,
            }),
            flags: knx_core::ResolvedFlags::none(),
            size: None,
            is_active: true,
            links: vec![knx_core::GroupLink {
                ga,
                direction: knx_core::Direction::Send,
            }],
            module_instance: None,
        });
    project
}

fn group_write(raw: u16, value: knx_core::GroupValue) -> knx_net::TunnelEvent {
    knx_net::TunnelEvent::Telegram(knx_net::LDataFrame {
        kind: knx_net::LDataMessageKind::Indication,
        source: device(9),
        destination: knx_net::Destination::Group(GroupAddress::from_raw(raw)),
        transport: knx_net::Tpci::UnnumberedData,
        service: knx_net::ApplicationService::GroupValueWrite(value),
        control: None,
    })
}

async fn poll_until_telegrams(app: &axum::Router, len: usize) {
    for _ in 0..200 {
        let body = call(app, "GET", "/api/bus/monitor/telegrams?since=0", None).await;
        if body["telegrams"].as_array().unwrap().len() >= len {
            return;
        }
        tokio::time::sleep(std::time::Duration::from_millis(2)).await;
    }
    panic!("telegrams never reached length {len}");
}

#[tokio::test]
async fn included_telegrams_keep_their_addresses_their_name_and_their_decoded_value() {
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("debug-report.zip");
    let (tunnel, handle) = knx_server::fake::FakeTunnel::new(device(5), 64);
    let state = knx_server::AppState {
        connector: Box::new(knx_server::fake::FakeConnector::succeeding(tunnel)),
        project: std::sync::Mutex::new(Some(project_with_a_decodable_group_address())),
        ..Default::default()
    };
    let app = knx_server::app(Arc::new(state), None);

    call(
        &app,
        "POST",
        "/api/bus/monitor/start",
        Some(json!({ "gateway": "192.0.2.10:3671" })),
    )
    .await;
    // `1/1/1` is the fixture's group address: raw 1 << 11 | 1 << 8 | 1.
    let style = knx_core::ProjectInfo::default().group_address_style;
    let linked = GroupAddress::parse("1/1/1", style).unwrap().raw();
    handle
        .sender()
        .send(group_write(linked, knx_core::GroupValue::Short(1)))
        .unwrap();
    // A second address the project does not know at all — the arm of the
    // decode match that must never guess.
    handle
        .sender()
        .send(group_write(linked + 1, knx_core::GroupValue::Short(0)))
        .unwrap();
    poll_until_telegrams(&app, 2).await;

    let response = call(
        &app,
        "POST",
        "/api/debug-report",
        Some(json!({
            "path": target.to_string_lossy(),
            "description": "a telegram went missing",
            "includeBusTelegrams": true,
        })),
    )
    .await;
    assert_eq!(response["written"], json!(true));
    assert!(zip_entry_names(&target).contains(&"bus-telegrams.json".to_string()));

    let text = zip_entry_text(&target, "bus-telegrams.json");
    let rows: Value = serde_json::from_str(&text).expect("bus-telegrams.json must be JSON");
    let rows = rows.as_array().unwrap();
    assert_eq!(rows.len(), 2, "{text}");

    assert_eq!(rows[0]["seq"], json!(0));
    assert_eq!(rows[0]["source"], json!("1.1.9"));
    assert_eq!(rows[0]["destination"], json!("1/1/1"));
    assert_eq!(rows[0]["destinationName"], json!("Living Room Light"));
    assert_eq!(rows[0]["service"], json!("GroupValueWrite"));
    assert_eq!(rows[0]["decoded"]["kind"], json!("value"));
    assert_eq!(rows[0]["decoded"]["dpt"], json!("DPST-1-1"));
    assert_eq!(rows[0]["decoded"]["text"], json!("on"));
    assert!(
        !rows[0]["timestamp"].as_str().unwrap().is_empty(),
        "a telegram without a timestamp cannot be correlated with the log"
    );

    assert_eq!(rows[1]["destination"], json!("1/1/2"));
    assert_eq!(rows[1]["destinationName"], Value::Null);
    assert_eq!(rows[1]["decoded"]["kind"], json!("unresolved"));
}
