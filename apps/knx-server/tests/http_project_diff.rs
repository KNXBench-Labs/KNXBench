//! HTTP tests for `POST /api/project/diff` (T14 —
//! `docs/superpowers/specs/2026-09-10-project-diff-design.md`). Compares
//! the server's live, possibly edited, in-memory project (`left`) against
//! a `.knxdb` file at the given `path` (`right`) — never a re-read of
//! `store_path` (design spec §7). Builds small in-memory projects
//! directly, same pattern as `http_documentation_export.rs`; the
//! comparison-file fixture is written with `knx_store::open_and_migrate`/
//! `knx_store::save_project`, exactly as `save_project_as_impl` does
//! internally.

use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use knx_core::{
    CommissioningState, CompletionStatus, DeviceId, DeviceInstance, Installation, InstallationId,
    Language, Project, SourceRef, Topology,
};
use serde_json::{json, Value};
use tower::ServiceExt;

fn source(tag: &str) -> SourceRef {
    SourceRef {
        path: tag.into(),
        ets_id: tag.into(),
    }
}

/// A project named "Test Villa" with one installation and one device
/// (`DeviceId(1)`, unassigned to any line — irrelevant to this route)
/// whose description is `description`. The one field this suite varies
/// between `left`/`right`.
fn project_with_device_description(description: &str) -> Project {
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
            unassigned: vec![DeviceId(1)],
        },
        buildings: vec![],
        group_ranges: vec![],
        group_addresses: vec![],
        parameters: vec![],
    });
    project.devices.insert(DeviceInstance {
        id: DeviceId(1),
        source: source("d1"),
        name: "Stray Device".into(),
        description: Some(description.to_string()),
        address: None,
        product_ref: "P".into(),
        program_ref: "H".into(),
        commissioning: CommissioningState::default(),
        visibility_calculated: true,
        com_objects: vec![],
        binary_data: vec![],
    });
    project
}

fn state_with_project(project: Project) -> knx_server::AppState {
    let state = knx_server::AppState::default();
    *state.project.lock().unwrap() = Some(project);
    state
}

/// Writes `project` to a fresh `.knxdb` file at `path` — the comparison
/// fixture ("right"), independent of the server's live state.
fn write_knxdb_fixture(path: &std::path::Path, project: &Project) {
    let conn = knx_store::open_and_migrate(path).unwrap();
    knx_store::save_project(&conn, project).unwrap();
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

async fn diff_project(app: &axum::Router, path: &std::path::Path) -> axum::response::Response {
    call(
        app,
        "POST",
        "/api/project/diff",
        Some(json!({ "path": path.to_string_lossy() })),
    )
    .await
}

#[tokio::test]
async fn a_live_project_identical_to_the_comparison_file_produces_an_empty_diff() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("compare.knxdb");
    write_knxdb_fixture(&db_path, &project_with_device_description("Same"));

    let state = Arc::new(state_with_project(project_with_device_description("Same")));
    let app = knx_server::app(state, None);

    let response = diff_project(&app, &db_path).await;
    assert_eq!(response.status(), StatusCode::OK);
    let body = body_json(response).await;

    assert_eq!(body["infoChanges"].as_array().unwrap().len(), 0, "{body}");
    let installations = body["installations"].as_array().unwrap();
    assert_eq!(installations.len(), 1, "{body}");
    let installation = &installations[0];

    for table_name in [
        "areas",
        "lines",
        "groupRanges",
        "groupAddresses",
        "buildings",
    ] {
        let table = &installation[table_name];
        assert_eq!(
            table["added"].as_array().unwrap().len(),
            0,
            "{table_name}: {body}"
        );
        assert_eq!(
            table["removed"].as_array().unwrap().len(),
            0,
            "{table_name}: {body}"
        );
        assert_eq!(
            table["changed"].as_array().unwrap().len(),
            0,
            "{table_name}: {body}"
        );
        assert_eq!(
            table["ambiguous"].as_array().unwrap().len(),
            0,
            "{table_name}: {body}"
        );
    }
    let devices = &installation["devices"];
    assert_eq!(devices["added"].as_array().unwrap().len(), 0, "{body}");
    assert_eq!(devices["removed"].as_array().unwrap().len(), 0, "{body}");
    assert_eq!(devices["changed"].as_array().unwrap().len(), 0, "{body}");
    assert_eq!(devices["ambiguous"].as_array().unwrap().len(), 0, "{body}");
}

#[tokio::test]
async fn a_changed_device_description_is_named_in_the_response() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("compare.knxdb");
    write_knxdb_fixture(
        &db_path,
        &project_with_device_description("Original description"),
    );

    let state = Arc::new(state_with_project(project_with_device_description(
        "Changed description",
    )));
    let app = knx_server::app(state, None);

    let response = diff_project(&app, &db_path).await;
    assert_eq!(response.status(), StatusCode::OK);
    let body = body_json(response).await;

    let text = body.to_string();
    assert!(
        text.contains("Changed description"),
        "response must name the changed value: {body}"
    );

    let installations = body["installations"].as_array().unwrap();
    assert_eq!(installations.len(), 1, "{body}");
    let changed = installations[0]["devices"]["changed"].as_array().unwrap();
    assert_eq!(changed.len(), 1, "{body}");
    assert_eq!(
        changed[0]["changedFields"],
        json!(["description"]),
        "{body}"
    );
    assert_eq!(
        changed[0]["left"]["description"], "Changed description",
        "{body}"
    );
    assert_eq!(
        changed[0]["right"]["description"], "Original description",
        "{body}"
    );
}

#[tokio::test]
async fn a_comparison_path_that_does_not_exist_is_a_400_and_creates_no_file() {
    let dir = tempfile::tempdir().unwrap();
    let missing_path = dir.path().join("does-not-exist.knxdb");
    assert!(!missing_path.exists());

    let state = Arc::new(state_with_project(project_with_device_description("Same")));
    let app = knx_server::app(state, None);

    let response = diff_project(&app, &missing_path).await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    // The existence-check-gotcha regression: `knx_store::open_and_migrate`
    // creates an empty SQLite file if handed a path that doesn't exist —
    // `diff_project_impl` must reject the path before ever calling it.
    assert!(
        !missing_path.exists(),
        "a typo'd comparison path must not silently create a .knxdb file"
    );
}

#[tokio::test]
async fn calling_it_with_no_project_open_is_a_400_not_a_500() {
    let state = Arc::new(knx_server::AppState::default());
    let app = knx_server::app(state, None);
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("compare.knxdb");
    write_knxdb_fixture(&db_path, &project_with_device_description("Same"));

    let response = diff_project(&app, &db_path).await;

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}
