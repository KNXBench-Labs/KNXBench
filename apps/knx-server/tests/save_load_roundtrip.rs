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
fn saving_then_reopening_a_native_project_round_trips_the_golden_counts() {
    if !reference_ets4_path().exists() {
        eprintln!("skip: OriginalData/ corpus not present (gitignored, local-only)");
        return;
    }
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
