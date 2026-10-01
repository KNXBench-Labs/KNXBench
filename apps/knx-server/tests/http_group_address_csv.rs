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

    let id1 = project.ids.next_group_address_id().unwrap();
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

    let id2 = project.ids.next_group_address_id().unwrap();
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
    import_csv_with_confirmation(app, path, None).await
}

async fn import_csv_with_confirmation(
    app: &axum::Router,
    path: &std::path::Path,
    confirmation_token: Option<&str>,
) -> axum::response::Response {
    call(
        app,
        "POST",
        "/api/group-addresses/csv-import",
        Some(json!({
            "path": path.to_string_lossy(),
            "confirmationToken": confirmation_token,
        })),
    )
    .await
}

#[tokio::test]
async fn structural_creation_issues_the_final_id_then_refuses_without_history_changes() {
    let cases = [
        ("area", "/api/areas", json!({"address":2,"name":"Last"})),
        (
            "line",
            "/api/lines",
            json!({"areaId":1,"address":1,"mediumRef":"MT-0","name":"Last"}),
        ),
        (
            "group_address",
            "/api/group-addresses",
            json!({"address":"1/1/3","name":"Last"}),
        ),
        (
            "group_range",
            "/api/group-ranges",
            json!({"start":"1/0/0","end":"1/7/255","name":"Last"}),
        ),
        (
            "building_part",
            "/api/building-parts",
            json!({"kind":"Building","name":"Last"}),
        ),
    ];
    for (kind, uri, body) in cases {
        let state = Arc::new(state_with_two_group_addresses());
        let app = knx_server::app(state.clone(), None);
        if kind == "line" {
            let response = call(
                &app,
                "POST",
                "/api/areas",
                Some(json!({"address":1,"name":"Parent"})),
            )
            .await;
            assert_eq!(response.status(), StatusCode::OK);
            *state.command_stack.lock().unwrap() = knx_core::CommandStack::new();
        }
        let n = u32::MAX - 1;
        state.project.lock().unwrap().as_mut().unwrap().ids =
            knx_core::IdAllocators::from_counts(n, n, n, n, n, n, n, n, n);
        let mut baseline = state.project.lock().unwrap().clone().unwrap();
        let response = call(&app, "POST", uri, Some(body.clone())).await;
        assert_eq!(
            response.status(),
            StatusCode::OK,
            "{uri}: {}",
            body_json(response).await
        );
        let created = state.project.lock().unwrap().clone().unwrap();
        let installation = &created.installations[0];
        let id = match kind {
            "area" => installation.topology.areas.last().unwrap().id.0,
            "line" => installation.topology.lines.last().unwrap().id.0,
            "group_address" => installation.group_addresses.last().unwrap().id.0,
            "group_range" => installation.group_ranges.last().unwrap().id.0,
            "building_part" => installation.buildings.last().unwrap().id.0,
            _ => unreachable!(),
        };
        assert_eq!(id, u32::MAX, "{kind}");
        let refused = call(&app, "POST", uri, Some(body)).await;
        assert_eq!(refused.status(), StatusCode::BAD_REQUEST);
        assert!(body_json(refused).await["error"]
            .as_str()
            .unwrap()
            .contains(&format!("project {kind} ID range exhausted")));
        assert_eq!(state.project.lock().unwrap().as_ref(), Some(&created));
        assert_eq!(
            call(&app, "POST", "/api/undo", None).await.status(),
            StatusCode::OK
        );
        baseline.ids = created.ids.clone();
        assert_eq!(state.project.lock().unwrap().as_ref(), Some(&baseline));
        assert!(!state.command_stack.lock().unwrap().can_undo());
        assert_eq!(
            call(&app, "POST", "/api/redo", None).await.status(),
            StatusCode::OK
        );
        assert_eq!(state.project.lock().unwrap().as_ref(), Some(&created));
        let dir = tempfile::tempdir().unwrap();
        let conn = knx_store::open_and_migrate(&dir.path().join("last.knxdb")).unwrap();
        knx_store::save_project(&conn, &created).unwrap();
        assert_eq!(knx_store::load_project(&conn).unwrap(), created);
    }
}

#[tokio::test]
async fn csv_exhaustion_refuses_the_whole_request_without_consuming_ids() {
    for counter in [u32::MAX - 1, u32::MAX] {
        let state = Arc::new(state_with_two_group_addresses());
        state.project.lock().unwrap().as_mut().unwrap().ids =
            knx_core::IdAllocators::from_counts(0, 0, 0, 0, 0, counter, 0, 0, 0);
        let before = state.project.lock().unwrap().clone();
        let app = knx_server::app(state.clone(), None);
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("edits.csv");
        std::fs::write(
            &path,
            "Address,Name\n1/1/1,Changed\n1/1/3,First\n1/1/4,Second\n",
        )
        .unwrap();
        let response = import_csv(&app, &path).await;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let report = body_json(response).await;
        assert!(
            report
                .to_string()
                .contains("project group_address ID range exhausted"),
            "{report}"
        );
        assert_eq!(*state.project.lock().unwrap(), before);
        assert!(!state.command_stack.lock().unwrap().can_undo());
        assert!(!state.command_stack.lock().unwrap().can_redo());
    }
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
        "Address,Action,NewAddress,Name,Central,Unfiltered,DatapointType (read-only),MainGroup (read-only),MiddleGroup (read-only)"
    );
    assert_eq!(lines[1], "1/1/1,upsert,,Living Room Light,false,false,,,");
    assert_eq!(lines[2], "1/1/2,upsert,,Kitchen Light,false,false,,,");
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
async fn readdress_requires_preview_then_exact_confirmation() {
    let state = Arc::new(state_with_two_group_addresses());
    let app = knx_server::app(state, None);
    let dir = tempfile::tempdir().unwrap();
    let csv_path = dir.path().join("readdress.csv");
    std::fs::write(
        &csv_path,
        "Address,Action,NewAddress,Name\n1/1/1,readdress,1/1/3,Living Room Light\n",
    )
    .unwrap();

    let preview = import_csv(&app, &csv_path).await;
    assert_eq!(preview.status(), StatusCode::OK);
    let preview = body_json(preview).await;
    assert_eq!(preview["applied"], false, "{preview}");
    assert_eq!(preview["report"]["readdressed"], 1, "{preview}");
    let token = preview["confirmationToken"].as_str().unwrap();

    let confirmed = import_csv_with_confirmation(&app, &csv_path, Some(token)).await;
    assert_eq!(confirmed.status(), StatusCode::OK);
    let confirmed = body_json(confirmed).await;
    assert_eq!(confirmed["applied"], true, "{confirmed}");

    let exported = dir.path().join("after.csv");
    assert_eq!(export_csv(&app, &exported).await.status(), StatusCode::OK);
    let text = std::fs::read_to_string(exported).unwrap();
    assert!(text.contains("1/1/3,upsert,,Living Room Light"), "{text}");
    assert!(!text.contains("1/1/1,upsert,,Living Room Light"), "{text}");
}

#[tokio::test]
async fn confirmation_is_rejected_when_the_csv_changes_after_preview() {
    let state = Arc::new(state_with_two_group_addresses());
    let app = knx_server::app(state, None);
    let dir = tempfile::tempdir().unwrap();
    let csv_path = dir.path().join("readdress.csv");
    std::fs::write(
        &csv_path,
        "Address,Action,NewAddress,Name\n1/1/1,readdress,1/1/3,Living Room Light\n",
    )
    .unwrap();
    let preview = body_json(import_csv(&app, &csv_path).await).await;
    let token = preview["confirmationToken"].as_str().unwrap().to_string();

    std::fs::write(
        &csv_path,
        "Address,Action,NewAddress,Name\n1/1/1,readdress,1/1/4,Living Room Light\n",
    )
    .unwrap();
    let response = import_csv_with_confirmation(&app, &csv_path, Some(&token)).await;

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body = body_json(response).await;
    assert!(body["error"].as_str().unwrap().contains("stale"), "{body}");
    let exported = dir.path().join("unchanged.csv");
    assert_eq!(export_csv(&app, &exported).await.status(), StatusCode::OK);
    let text = std::fs::read_to_string(exported).unwrap();
    assert!(text.contains("1/1/1,upsert,,Living Room Light"), "{text}");
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
