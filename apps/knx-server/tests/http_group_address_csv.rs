//! HTTP tests for the two CSV group-address routes (T12 —
//! `docs/superpowers/specs/2026-09-10-csv-group-address-exchange-design.md`
//! §7): `POST /api/group-addresses/csv-export` and
//! `POST /api/group-addresses/csv-import`. Builds a small in-memory project
//! directly (`state_with_two_group_addresses`, same pattern
//! `http_edit_routes.rs`/`http_batch_routes.rs` already use) rather than
//! depending on the `OriginalData/` corpus — these routes have nothing to
//! do with `.knxproj` import.

use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use knx_core::{
    CompletionStatus, GroupAddress, GroupAddressEntry, Installation, InstallationId, Language,
    Project, SourceRef, Topology,
};
use serde_json::{json, Value};
use tower::ServiceExt;

fn source(tag: &str) -> SourceRef {
    SourceRef {
        path: tag.into(),
        ets_id: tag.into(),
    }
}

/// A project with one installation and two group addresses
/// (`1/1/1`/"Living Room Light", `1/1/2`/"Kitchen Light"), the project's
/// default `GroupAddressStyle::ThreeLevel` style, and no group ranges — so
/// every CSV-created address is expected to land with `range: None` plus
/// the "no range contains this address" warning. Ids come from
/// `project.ids.next_group_address_id()`, exactly like
/// `create_group_address_impl` would, so a CSV-planned create afterwards
/// never collides with them.
fn state_with_two_group_addresses() -> knx_server::AppState {
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
            unassigned: vec![],
        },
        buildings: vec![],
        group_ranges: vec![],
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
            range: None,
        });

    let id2 = project.ids.next_group_address_id();
    project.installations[0]
        .group_addresses
        .push(GroupAddressEntry {
            id: id2,
            source: source("t2"),
            name: "Kitchen Light".into(),
            address: GroupAddress::parse("1/1/2", project.info.group_address_style).unwrap(),
            central: false,
            unfiltered: false,
            range: None,
        });

    let state = knx_server::AppState::default();
    *state.project.lock().unwrap() = Some(project);
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

async fn export_csv(app: &axum::Router, path: &std::path::Path) -> axum::response::Response {
    call(
        app,
        "POST",
        "/api/group-addresses/csv-export",
        Some(json!({ "path": path.to_string_lossy() })),
    )
    .await
}

async fn import_csv(app: &axum::Router, path: &std::path::Path) -> axum::response::Response {
    call(
        app,
        "POST",
        "/api/group-addresses/csv-import",
        Some(json!({ "path": path.to_string_lossy() })),
    )
    .await
}

#[tokio::test]
async fn exporting_writes_a_csv_file_whose_first_data_row_matches_the_project() {
    let state = Arc::new(state_with_two_group_addresses());
    let app = knx_server::app(state, None);
    let dir = tempfile::tempdir().unwrap();
    let csv_path = dir.path().join("export.csv");

    let response = export_csv(&app, &csv_path).await;
    assert_eq!(response.status(), StatusCode::OK);
    let report = body_json(response).await;
    assert_eq!(report["warnings"].as_array().unwrap().len(), 0);

    let bytes = std::fs::read(&csv_path).unwrap();
    assert_eq!(
        &bytes[0..3],
        [0xEF, 0xBB, 0xBF],
        "the writer's own contract is a leading UTF-8 BOM"
    );
    let text = String::from_utf8(bytes).unwrap();
    let text = text.strip_prefix('\u{FEFF}').unwrap_or(&text);
    let lines: Vec<&str> = text.trim_end_matches("\r\n").split("\r\n").collect();
    assert_eq!(
        lines[0],
        "Address,Name,Central,Unfiltered,DatapointType,MainGroup,MiddleGroup"
    );
    assert_eq!(lines[1], "1/1/1,Living Room Light,false,false,,,");
    assert_eq!(lines[2], "1/1/2,Kitchen Light,false,false,,,");
}

#[tokio::test]
async fn reimporting_a_freshly_exported_file_reports_everything_as_unchanged() {
    let state = Arc::new(state_with_two_group_addresses());
    let app = knx_server::app(state, None);
    let dir = tempfile::tempdir().unwrap();
    let csv_path = dir.path().join("export.csv");

    let export_response = export_csv(&app, &csv_path).await;
    assert_eq!(export_response.status(), StatusCode::OK);

    let import_response = import_csv(&app, &csv_path).await;
    assert_eq!(import_response.status(), StatusCode::OK);
    let body = body_json(import_response).await;
    assert_eq!(body["report"]["rowsRead"], 2);
    assert_eq!(body["report"]["created"], 0);
    assert_eq!(body["report"]["updated"], 0);
    assert_eq!(body["report"]["unchanged"], 2);
    assert_eq!(body["report"]["problems"].as_array().unwrap().len(), 0);
    let addresses = body["tree"]["installations"][0]["group_addresses"]
        .as_array()
        .unwrap();
    assert_eq!(addresses.len(), 2, "{addresses:?}");
}

#[tokio::test]
async fn importing_one_new_and_one_renamed_address_reports_the_right_counts_and_tree() {
    let state = Arc::new(state_with_two_group_addresses());
    let app = knx_server::app(state, None);
    let dir = tempfile::tempdir().unwrap();
    let csv_path = dir.path().join("edits.csv");
    // "1/1/1" renamed (update); "1/1/3" is new (create); "1/1/2" is simply
    // absent from the file, so it must be left alone (design §4: "import
    // never deletes").
    std::fs::write(
        &csv_path,
        "Address,Name\n1/1/1,Living Room Light (renamed)\n1/1/3,Hallway Light\n",
    )
    .unwrap();

    let response = import_csv(&app, &csv_path).await;
    assert_eq!(response.status(), StatusCode::OK);
    let body = body_json(response).await;
    assert_eq!(body["report"]["created"], 1, "{body}");
    assert_eq!(body["report"]["updated"], 1, "{body}");
    assert_eq!(body["report"]["unchanged"], 0, "{body}");

    let addresses = body["tree"]["installations"][0]["group_addresses"]
        .as_array()
        .unwrap();
    assert_eq!(addresses.len(), 3, "{addresses:?}");
    let names: Vec<&str> = addresses
        .iter()
        .map(|a| a["name"].as_str().unwrap())
        .collect();
    assert!(names.contains(&"Living Room Light (renamed)"), "{names:?}");
    assert!(names.contains(&"Kitchen Light"), "{names:?}");
    assert!(names.contains(&"Hallway Light"), "{names:?}");
}

#[tokio::test]
async fn importing_a_file_with_a_bad_row_is_a_400_naming_the_row_and_leaves_the_project_untouched()
{
    let state = Arc::new(state_with_two_group_addresses());
    let app = knx_server::app(state, None);
    let dir = tempfile::tempdir().unwrap();

    // Snapshot the project before the rejected import, through the same
    // (pure, non-mutating) export route the first test already trusts —
    // the strongest available proof that the bad import changed nothing.
    let before_path = dir.path().join("before.csv");
    assert_eq!(
        export_csv(&app, &before_path).await.status(),
        StatusCode::OK
    );
    let before_bytes = std::fs::read(&before_path).unwrap();

    let bad_path = dir.path().join("bad.csv");
    // Row 2 (line 3, counting the header) has no name at all.
    std::fs::write(&bad_path, "Address,Name\n1/1/1,Still Fine\n1/1/9,\n").unwrap();

    let response = import_csv(&app, &bad_path).await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body = body_json(response).await;
    let error = body["error"].as_str().unwrap();
    assert!(
        error.contains("row 3"),
        "expected the offending row number in the error, got: {error}"
    );

    let after_path = dir.path().join("after.csv");
    assert_eq!(export_csv(&app, &after_path).await.status(), StatusCode::OK);
    let after_bytes = std::fs::read(&after_path).unwrap();
    assert_eq!(
        before_bytes, after_bytes,
        "a rejected import must leave the project exactly as it was"
    );
}

#[tokio::test]
async fn both_csv_operations_append_to_the_session_log_without_clearing_it() {
    let state = Arc::new(state_with_two_group_addresses());
    let app = knx_server::app(state, None);
    let dir = tempfile::tempdir().unwrap();

    // A plain (non-CSV) edit first, so the log already holds an entry
    // *before* either CSV route runs — `session_log.rs`'s own contract is
    // that only opening a whole project resets it, and this is the
    // observable proof that neither CSV route does either.
    let seed = call(
        &app,
        "POST",
        "/api/group-addresses",
        Some(json!({ "name": "Seed Light", "address": "1/1/5" })),
    )
    .await;
    assert_eq!(seed.status(), StatusCode::OK);

    let export_path = dir.path().join("export.csv");
    assert_eq!(
        export_csv(&app, &export_path).await.status(),
        StatusCode::OK
    );

    // A brand-new address with no group range in the project to contain
    // it: this exercises the "created without one" warning path, so this
    // one test proves both the per-problem entries and the trailing
    // summary entry make it into the log, not just a bare success note.
    let import_path = dir.path().join("new-address.csv");
    std::fs::write(&import_path, "Address,Name\n1/1/9,New One\n").unwrap();
    assert_eq!(
        import_csv(&app, &import_path).await.status(),
        StatusCode::OK
    );

    let log_response = call(&app, "GET", "/api/log", None).await;
    assert_eq!(log_response.status(), StatusCode::OK);
    let entries = body_json(log_response).await;
    let entries = entries.as_array().unwrap();

    assert_eq!(
        entries[0]["source"], "CreateGroupAddress",
        "the pre-existing entry must survive both operations: {entries:?}"
    );
    assert!(
        entries.iter().any(|e| e["source"] == "csv-export"),
        "{entries:?}"
    );
    assert!(
        entries
            .iter()
            .any(|e| e["source"] == "csv-import:warning" && e["location"] == "row 2"),
        "expected the 'no range contains this address' warning at row 2: {entries:?}"
    );
    assert!(
        entries.iter().any(|e| e["source"] == "csv-import"),
        "expected a csv-import summary entry: {entries:?}"
    );
}
