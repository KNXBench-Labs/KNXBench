//! HTTP tests for `POST /api/project/documentation-export` (T13 —
//! `docs/superpowers/specs/2026-09-10-project-documentation-export-design.md`).
//! Builds small in-memory projects directly, same pattern as
//! `http_group_address_csv.rs`. This route never claims ETS-report parity
//! (no ETS-produced sample exists in this repository) — it exports "project
//! documentation", nothing more, nothing less.

use std::sync::{Arc, Mutex};

use axum::body::Body;
use axum::http::{Request, StatusCode};
use knx_core::{
    CommissioningState, CompletionStatus, DeviceId, DeviceInstance, GroupAddress,
    GroupAddressEntry, GroupRange, GroupRangeId, Installation, InstallationId, Language, Project,
    SourceRef, Topology,
};
use serde_json::{json, Value};
use tower::ServiceExt;

fn source(tag: &str) -> SourceRef {
    SourceRef {
        path: tag.into(),
        ets_id: tag.into(),
    }
}

/// A project named "Test Villa" with one installation, one group range
/// spanning the whole address space (so a well-formed address never also
/// triggers the unrelated "not inside any group range" warning), one group
/// address inside it (`1/1/1`/"Living Room Light"), the default
/// `GroupAddressStyle::ThreeLevel` style, and no devices at all — the
/// baseline "nothing structurally odd" fixture.
fn state_with_one_group_address() -> knx_server::AppState {
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

    let id1 = project.ids.next_group_address_id();
    project.installations[0]
        .group_addresses
        .push(GroupAddressEntry {
            id: id1,
            source: source("t1"),
            name: "Living Room Light".into(),
            address: GroupAddress::parse("1/1/1", project.info.group_address_style).unwrap(),
            central: false,
            unfiltered: false,
            range: Some(GroupRangeId(1)),
        });

    let state = knx_server::AppState::default();
    *state.project.lock().unwrap() = Some(project);
    state
}

/// Same as [`state_with_one_group_address`], plus one device
/// (`DeviceId(1)`) that is assigned to no line (`Topology::unassigned`) —
/// the structural oddity this brief's warning test exercises.
fn state_with_a_device_in_no_line() -> knx_server::AppState {
    let state = state_with_one_group_address();
    {
        let mut project = state.project.lock().unwrap();
        let project = project.as_mut().unwrap();
        project.installations[0].topology.unassigned = vec![DeviceId(1)];
        project.devices.insert(DeviceInstance {
            id: DeviceId(1),
            source: source("d1"),
            name: "Stray Device".into(),
            description: None,
            address: None,
            product_ref: "P".into(),
            program_ref: "H".into(),
            commissioning: CommissioningState::default(),
            visibility_calculated: true,
            com_objects: vec![],
            binary_data: vec![],
        });
    }
    state
}

async fn body_json(response: axum::response::Response) -> Value {
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

async fn call(
    app: &axum::Router,
    method: &str,
    uri: &str,
    body: Option<Value>,
) -> axum::response::Response {
    let mut builder = Request::builder().method(method).uri(uri);
    let body = match body {
        Some(v) => {
            builder = builder.header("content-type", "application/json");
            Body::from(v.to_string())
        }
        None => Body::empty(),
    };
    app.clone()
        .oneshot(builder.body(body).unwrap())
        .await
        .unwrap()
}

async fn export_documentation(
    app: &axum::Router,
    path: &std::path::Path,
) -> axum::response::Response {
    call(
        app,
        "POST",
        "/api/project/documentation-export",
        Some(json!({ "path": path.to_string_lossy() })),
    )
    .await
}

#[tokio::test]
async fn exporting_writes_a_self_contained_html_document_naming_the_project_and_its_group_address()
{
    let state = Arc::new(state_with_one_group_address());
    let app = knx_server::app(state, None);
    let dir = tempfile::tempdir().unwrap();
    let html_path = dir.path().join("documentation.html");

    let response = export_documentation(&app, &html_path).await;
    assert_eq!(response.status(), StatusCode::OK);
    let report = body_json(response).await;
    assert_eq!(report["warnings"].as_array().unwrap().len(), 0, "{report}");

    let text = std::fs::read_to_string(&html_path).unwrap();
    assert!(text.starts_with("<!DOCTYPE html>"), "{text}");
    assert!(text.contains("Test Villa"), "{text}");
    assert!(text.contains("1/1/1"), "{text}");
}

#[tokio::test]
async fn each_device_warning_appends_one_log_entry_without_clearing() {
    let state = Arc::new(state_with_a_device_in_no_line());
    let app = knx_server::app(state, None);
    let dir = tempfile::tempdir().unwrap();
    let html_path = dir.path().join("documentation.html");

    // A plain (non-doc-export) edit first, so the log already holds an
    // entry *before* the route runs — `session_log.rs`'s own contract is
    // that only opening a whole project resets it, and this is the
    // observable proof this route doesn't either.
    let seed = call(
        &app,
        "POST",
        "/api/group-addresses",
        Some(json!({ "name": "Seed Light", "address": "1/1/5", "rangeId": 1 })),
    )
    .await;
    assert_eq!(seed.status(), StatusCode::OK);

    let response = export_documentation(&app, &html_path).await;
    assert_eq!(response.status(), StatusCode::OK);
    let report = body_json(response).await;
    let warnings = report["warnings"].as_array().unwrap();
    assert_eq!(warnings.len(), 2, "{report}");
    assert!(
        warnings.iter().any(|warning| {
            warning["location"] == "device 1"
                && warning["detail"]
                    .as_str()
                    .is_some_and(|detail| detail.contains("unassigned"))
        }),
        "{report}"
    );
    assert!(
        warnings.iter().any(|warning| {
            warning["location"] == "device 1"
                && warning["detail"]
                    .as_str()
                    .is_some_and(|detail| detail.contains("product database"))
        }),
        "{report}"
    );

    let log_response = call(&app, "GET", "/api/log", None).await;
    assert_eq!(log_response.status(), StatusCode::OK);
    let entries = body_json(log_response).await;
    let entries = entries.as_array().unwrap();

    assert_eq!(
        entries[0]["source"], "CreateGroupAddress",
        "the pre-existing entry must survive the export: {entries:?}"
    );
    let doc_export_warnings: Vec<&Value> = entries
        .iter()
        .filter(|e| e["source"] == "doc-export" && e["location"] == "device 1")
        .collect();
    assert_eq!(doc_export_warnings.len(), 2, "{entries:?}");
}

#[tokio::test]
async fn a_path_outside_the_data_directory_is_rejected() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir(dir.path().join("sub")).unwrap();
    let state = Arc::new(knx_server::AppState::new(dir.path().to_path_buf()));
    *state.project.lock().unwrap() = Some({
        let mut p = Project::new(Language("en".into()));
        p.info.name = "Test Villa".into();
        p
    });
    let app = knx_server::app(state, None);

    let response = call(
        &app,
        "POST",
        "/api/project/documentation-export",
        Some(json!({ "path": "../escaped.html" })),
    )
    .await;

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn calling_it_with_no_project_open_is_a_400_not_a_500() {
    let state = Arc::new(knx_server::AppState::default());
    let app = knx_server::app(state, None);
    let dir = tempfile::tempdir().unwrap();
    let html_path = dir.path().join("documentation.html");

    let response = export_documentation(&app, &html_path).await;

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn preview_returns_localized_selected_html_without_writing_a_file() {
    let dir = tempfile::tempdir().unwrap();
    let mut state = state_with_one_group_address();
    state.data_dir = dir.path().to_path_buf();
    let state = Arc::new(state);
    let app = knx_server::app(state, None);

    let seed = call(
        &app,
        "POST",
        "/api/group-addresses",
        Some(json!({ "name": "Seed Light", "address": "1/1/5", "rangeId": 1 })),
    )
    .await;
    assert_eq!(seed.status(), StatusCode::OK);
    let log_before = body_json(call(&app, "GET", "/api/log", None).await).await;
    let files_before: Vec<_> = std::fs::read_dir(dir.path())
        .unwrap()
        .map(|entry| entry.unwrap().file_name())
        .collect();

    let response = call(
        &app,
        "POST",
        "/api/project/documentation-preview",
        Some(json!({ "language": "de", "sections": ["devices"] })),
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
    let report = body_json(response).await;
    let html = report["html"].as_str().unwrap();
    assert!(html.contains("<html lang=\"de\">"), "{html}");
    assert!(html.contains("<section id=\"devices\""), "{html}");
    assert!(!html.contains("<section id=\"topology\""), "{html}");

    let files_after: Vec<_> = std::fs::read_dir(dir.path())
        .unwrap()
        .map(|entry| entry.unwrap().file_name())
        .collect();
    assert_eq!(files_after, files_before, "preview must not create a file");
    let log_after = body_json(call(&app, "GET", "/api/log", None).await).await;
    assert_eq!(
        log_after, log_before,
        "preview must not mutate the session log"
    );
}

#[tokio::test]
async fn preview_rejects_an_unsupported_report_language() {
    let state = Arc::new(state_with_one_group_address());
    let app = knx_server::app(state, None);

    let response = call(
        &app,
        "POST",
        "/api/project/documentation-preview",
        Some(json!({ "language": "fr", "sections": ["devices"] })),
    )
    .await;

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn preview_rejects_an_unknown_section_instead_of_silently_omitting_it() {
    let state = Arc::new(state_with_one_group_address());
    let app = knx_server::app(state, None);

    let response = call(
        &app,
        "POST",
        "/api/project/documentation-preview",
        Some(json!({ "sections": ["devices", "invented"] })),
    )
    .await;

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn preview_resolves_product_identity_in_the_requested_language_seam() {
    let dir = tempfile::tempdir().unwrap();
    let products = knx_productdb::open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
    products.execute_batch(
        "INSERT INTO manufacturer VALUES ('M', 'Acme Controls');
         INSERT INTO hardware VALUES ('HW', 'M', 'Hardware', NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, 'x');
         INSERT INTO product VALUES ('P', 'M', 'HW', 'Room Controller', NULL, NULL, NULL, NULL, NULL, NULL, 'x');
         INSERT INTO application_program VALUES ('APP', 'M', 'Lighting 2.1', NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, 'x');
         INSERT INTO hardware2program VALUES ('H', 'M', 'HW', 'APP', NULL, NULL, NULL, NULL, NULL, 'x');",
    ).unwrap();
    let mut state = state_with_a_device_in_no_line();
    state.product_db = Some(Mutex::new(products));
    let app = knx_server::app(Arc::new(state), None);

    let response = call(
        &app,
        "POST",
        "/api/project/documentation-preview",
        Some(json!({ "language": "en", "sections": ["devices"] })),
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
    let report = body_json(response).await;
    let html = report["html"].as_str().unwrap();
    assert!(html.contains("Acme Controls"), "{html}");
    assert!(html.contains("Room Controller"), "{html}");
    assert!(html.contains("Lighting 2.1"), "{html}");
}
