//! Round-trips the reference project through knx-desktop's own `.knxdb`
//! save/load path (distinct from the ETS `.knxproj` import path covered by
//! `open_reference_project.rs`): import once, save to a fresh `.knxdb` file,
//! reopen it, and confirm the projection carries the same golden counts.

use std::path::PathBuf;
use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use serde_json::{json, Value};
use tower::ServiceExt;

fn reference_ets4_path() -> PathBuf {
    knx_testsupport::reference_ets4_path()
}

async fn body_json(response: axum::response::Response) -> Value {
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

#[test]
#[ignore = "requires the gitignored OriginalData/ corpus; run with --ignored"]
fn saving_then_reopening_a_native_project_round_trips_the_golden_counts() {
    assert!(
        reference_ets4_path().exists(),
        "OriginalData/ corpus not present (gitignored, local-only); this test is #[ignore]d and must be run explicitly on a machine that has it"
    );
    let conn = knx_store::open_and_migrate_in_memory().unwrap();
    let imported = knx_app::import_ets_project_with(
        &reference_ets4_path(),
        &conn,
        knx_app::ImportOptions::default(),
    )
    .unwrap();

    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("roundtrip.knxdb");

    knx_server::save_project_as_impl(&db_path, &imported.project, &[], &[]).unwrap();
    let tree = knx_server::open_native_project_impl(&db_path).unwrap();

    // A native load has no `ImportReport` — nothing was reinterpreted from
    // an external format, so both counts are genuinely zero, not merely
    // unmeasured.
    assert_eq!(tree.errors, 0);
    assert_eq!(tree.warnings, 0);

    assert_eq!(tree.installations.len(), 1);
    let inst = &tree.installations[0];

    assert_eq!(inst.topology.len(), 1); // one area
    assert_eq!(inst.topology[0].lines.len(), 1);

    let device_count: usize = inst
        .topology
        .iter()
        .flat_map(|a| a.lines.iter())
        .map(|l| l.devices.len())
        .sum();
    assert_eq!(device_count, 35); // 35 on the line
    assert_eq!(inst.unassigned.len(), 1); // plus the one unassigned device

    assert_eq!(count_buildings(&inst.buildings), 22);
}

fn count_buildings(nodes: &[knx_projection::BuildingNode]) -> usize {
    nodes.iter().map(|n| 1 + count_buildings(&n.children)).sum()
}

/// Item 6 of the T4 fix round: restyling was covered at store level
/// (`knx-store::project`'s round-trip tests) and the HTTP route's forward
/// effect is covered by `http_edit_routes.rs`, but nothing before this took
/// a style through the HTTP API and *then* through a save/load cycle — the
/// end-to-end claim in `docs/KNOWN_LIMITATIONS.md` §84 and
/// `docs/IMPLEMENTATION_STATUS.md`'s T4 entry was inferred from those two
/// facts, not tested directly. No corpus needed: an empty project created
/// through the same route the UI uses is enough to prove the style itself
/// survives the trip.
#[tokio::test]
async fn restyling_over_http_then_saving_and_reloading_keeps_the_new_style() {
    let state = Arc::new(knx_server::AppState::default());
    let app = knx_server::app(state, None);
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("restyled.knxdb");

    let new_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/project/new")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(new_response.status(), StatusCode::OK);
    // `Project::new`'s default, confirmed before changing it.
    assert_eq!(
        body_json(new_response).await["group_address_style"],
        "ThreeLevel"
    );

    let restyle_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/project/group-address-style")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({ "groupAddressStyle": "Free" }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(restyle_response.status(), StatusCode::OK);

    let save_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/project/save-as")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({ "path": db_path.to_string_lossy() }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(save_response.status(), StatusCode::OK);

    let reopen_response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/project/open")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({ "path": db_path.to_string_lossy() }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(reopen_response.status(), StatusCode::OK);
    assert_eq!(
        body_json(reopen_response).await["group_address_style"],
        "Free"
    );
}

#[tokio::test]
async fn renamed_area_and_line_survive_native_save_and_reopen() {
    let app = knx_server::app(Arc::new(knx_server::AppState::default()), None);
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("topology-names.knxdb");
    let new_project = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/project/new")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(new_project.status(), StatusCode::OK);

    let area = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/areas")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({ "name": "Old area", "address": 1 }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(area.status(), StatusCode::OK);
    let area_id = body_json(area).await["installations"][0]["topology"][0]["id"]
        .as_u64()
        .unwrap();
    let line = app
        .clone()
        .oneshot(Request::builder().method("POST").uri("/api/lines")
            .header("content-type", "application/json")
            .body(Body::from(json!({ "areaId": area_id, "name": "Old line", "address": 2, "mediumRef": "MT-0" }).to_string())).unwrap())
        .await
        .unwrap();
    assert_eq!(line.status(), StatusCode::OK);
    let line_id = body_json(line).await["installations"][0]["topology"][0]["lines"][0]["id"]
        .as_u64()
        .unwrap();

    for (uri, name) in [
        (format!("/api/areas/{area_id}"), "New area"),
        (format!("/api/lines/{line_id}"), "New line"),
    ] {
        let renamed = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("PATCH")
                    .uri(uri)
                    .header("content-type", "application/json")
                    .body(Body::from(json!({ "name": name }).to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(renamed.status(), StatusCode::OK);
    }
    let save = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/project/save-as")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({ "path": db_path.to_string_lossy() }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(save.status(), StatusCode::OK);
    let open = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/project/open")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({ "path": db_path.to_string_lossy() }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(open.status(), StatusCode::OK);
    let tree = body_json(open).await;
    let area = &tree["installations"][0]["topology"][0];
    assert_eq!(area["id"], area_id);
    assert_eq!(area["name"], "New area");
    assert_eq!(area["address"], 1);
    let line = &area["lines"][0];
    assert_eq!(line["id"], line_id);
    assert_eq!(line["name"], "New line");
    assert_eq!(line["address"], 2);
}

#[tokio::test]
async fn reparented_building_survives_native_save_and_reopen_with_sibling_order() {
    let app = knx_server::app(Arc::new(knx_server::AppState::default()), None);
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("building-move.knxdb");
    let created = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/project/new")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(created.status(), StatusCode::OK);

    let mut ids = Vec::new();
    for name in ["Source", "First", "Middle", "Last", "Target"] {
        let parent_id = (!ids.is_empty() && ids.len() < 4).then(|| ids[0]);
        let created = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/building-parts")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        json!({
                            "name": name,
                            "kind": if parent_id.is_some() { "Room" } else { "Building" },
                            "parentId": parent_id,
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(created.status(), StatusCode::OK);
        let tree = body_json(created).await;
        let roots = tree["installations"][0]["buildings"].as_array().unwrap();
        let id = if parent_id.is_some() {
            roots[0]["children"].as_array().unwrap().last().unwrap()["id"]
                .as_u64()
                .unwrap()
        } else {
            roots.last().unwrap()["id"].as_u64().unwrap()
        };
        ids.push(id);
    }

    let moved = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/move-building-part")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({ "id": ids[2], "parentId": ids[4] }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(moved.status(), StatusCode::OK);
    let save = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/project/save-as")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({ "path": db_path.to_string_lossy() }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(save.status(), StatusCode::OK);
    let reopened = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/project/open")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({ "path": db_path.to_string_lossy() }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(reopened.status(), StatusCode::OK);
    let tree = body_json(reopened).await;
    let roots = tree["installations"][0]["buildings"].as_array().unwrap();
    assert_eq!(roots.len(), 2);
    assert_eq!(roots[0]["id"], ids[0]);
    assert_eq!(roots[1]["id"], ids[4]);
    let children = roots[0]["children"].as_array().unwrap();
    assert_eq!(
        children
            .iter()
            .map(|child| child["id"].as_u64().unwrap())
            .collect::<Vec<_>>(),
        vec![ids[1], ids[3]]
    );
    assert_eq!(children[0]["name"], "First");
    assert_eq!(children[1]["name"], "Last");
    assert_eq!(roots[1]["children"][0]["id"], ids[2]);
    assert_eq!(roots[1]["children"][0]["name"], "Middle");
}

#[tokio::test]
async fn repaired_group_range_parent_and_linked_address_survive_native_save_and_reopen() {
    let state = Arc::new(knx_server::AppState::default());
    let app = knx_server::app(state.clone(), None);
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("group-range-repair.knxdb");
    let created = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/project/new")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(created.status(), StatusCode::OK);

    let mut ids = Vec::new();
    for (name, start, end, nested) in [
        ("Original", "0/0/0", "0/7/255", false),
        ("Child", "0/1/0", "0/1/255", true),
        ("Target", "1/0/0", "1/7/255", false),
    ] {
        let parent_id = nested.then(|| ids[0]);
        let created = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/group-ranges")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        json!({ "name": name, "start": start, "end": end, "parentId": parent_id })
                            .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(created.status(), StatusCode::OK);
        ids.push(
            body_json(created).await["installations"][0]["group_ranges"]
                .as_array()
                .unwrap()
                .last()
                .unwrap()["id"]
                .as_u64()
                .unwrap(),
        );
    }
    // Reproduce an imported range whose stored parent is wrong, without
    // ever changing the user's corpus. The edit must repair the hierarchy
    // while preserving this group's linked address and its raw span.
    {
        let mut guard = state.project.lock().unwrap();
        let range = &mut guard.as_mut().unwrap().installations[0].group_ranges[1];
        assert_eq!(range.id.0 as u64, ids[1]);
        range.start = knx_core::GroupAddress::from_raw(2560);
        range.end = knx_core::GroupAddress::from_raw(2815);
    }
    let address = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/group-addresses")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({ "name": "Light", "address": "1/2/1", "rangeId": ids[1] }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(address.status(), StatusCode::OK);
    let moved = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/move-group-range")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({ "id": ids[1], "parentId": ids[2] }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(moved.status(), StatusCode::OK);
    let saved = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/project/save-as")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({ "path": db_path.to_string_lossy() }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(saved.status(), StatusCode::OK);
    let reopened = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/project/open")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({ "path": db_path.to_string_lossy() }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(reopened.status(), StatusCode::OK);
    let tree = body_json(reopened).await;
    let ranges = tree["installations"][0]["group_ranges"].as_array().unwrap();
    assert_eq!(ranges[1]["id"], ids[1]);
    assert_eq!(ranges[1]["parent"], ids[2]);
    assert_eq!(ranges[1]["start"], "1/2/0");
    assert_eq!(ranges[1]["end"], "1/2/255");
    let address = &tree["installations"][0]["group_addresses"][0];
    assert_eq!(address["name"], "Light");
    assert_eq!(address["address"], "1/2/1");
    assert_eq!(address["range"], ids[1]);
    let guard = state.project.lock().unwrap();
    let ranges = &guard.as_ref().unwrap().installations[0].group_ranges;
    assert!(ranges[0].children.is_empty());
    assert_eq!(
        ranges[2].children,
        vec![knx_core::GroupRangeId(ids[1] as u32)]
    );
}

#[tokio::test]
async fn reparented_line_survives_native_save_with_address_and_area_order_intact() {
    let app = knx_server::app(Arc::new(knx_server::AppState::default()), None);
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("line-reparent.knxdb");
    let created = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/project/new")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(created.status(), StatusCode::OK);

    let mut areas = Vec::new();
    for (name, address) in [("Source", 1), ("Destination", 2)] {
        let created = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/areas")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        json!({ "name": name, "address": address }).to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(created.status(), StatusCode::OK);
        areas.push(
            body_json(created).await["installations"][0]["topology"]
                .as_array()
                .unwrap()
                .last()
                .unwrap()["id"]
                .as_u64()
                .unwrap(),
        );
    }
    let mut lines = Vec::new();
    for (area_index, name, address) in [
        (0, "First", 1),
        (0, "Middle", 2),
        (0, "Last", 3),
        (1, "Target line", 1),
    ] {
        let created = app.clone().oneshot(
            Request::builder().method("POST").uri("/api/lines")
                .header("content-type", "application/json")
                .body(Body::from(json!({
                    "areaId": areas[area_index], "name": name, "address": address, "mediumRef": "MT-0",
                }).to_string())).unwrap()
        ).await.unwrap();
        assert_eq!(created.status(), StatusCode::OK);
        lines.push(
            body_json(created).await["installations"][0]["topology"][area_index]["lines"]
                .as_array()
                .unwrap()
                .last()
                .unwrap()["id"]
                .as_u64()
                .unwrap(),
        );
    }
    let moved = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/move-line-to-area")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({ "id": lines[1], "areaId": areas[1] }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(moved.status(), StatusCode::OK);
    let saved = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/project/save-as")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({ "path": db_path.to_string_lossy() }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(saved.status(), StatusCode::OK);
    let reopened = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/project/open")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({ "path": db_path.to_string_lossy() }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(reopened.status(), StatusCode::OK);
    let tree = body_json(reopened).await;
    let topology = tree["installations"][0]["topology"].as_array().unwrap();
    assert_eq!(topology[0]["id"], areas[0]);
    assert_eq!(topology[1]["id"], areas[1]);
    assert_eq!(
        topology[0]["lines"]
            .as_array()
            .unwrap()
            .iter()
            .map(|line| line["id"].as_u64().unwrap())
            .collect::<Vec<_>>(),
        vec![lines[0], lines[2]]
    );
    assert_eq!(
        topology[1]["lines"]
            .as_array()
            .unwrap()
            .iter()
            .map(|line| line["id"].as_u64().unwrap())
            .collect::<Vec<_>>(),
        vec![lines[3], lines[1]]
    );
    assert_eq!(topology[1]["lines"][1]["name"], "Middle");
    assert_eq!(topology[1]["lines"][1]["address"], 2);
}
