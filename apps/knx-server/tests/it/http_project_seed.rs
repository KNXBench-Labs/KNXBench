//! HTTP tests for the new-project wizard's starting structure on `POST /api/project/new`.

use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use serde_json::{json, Value};
use tower::ServiceExt;

async fn body_json(response: axum::response::Response) -> Value {
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

fn post(uri: &str, body: Value) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri(uri)
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .unwrap()
}

fn get(uri: &str) -> Request<Body> {
    Request::builder().uri(uri).body(Body::empty()).unwrap()
}

fn app() -> (tempfile::TempDir, axum::Router) {
    let dir = tempfile::tempdir().unwrap();
    let state = Arc::new(knx_server::AppState::new(dir.path().to_path_buf()));
    (dir, knx_server::app(state, None))
}

fn seed() -> Value {
    json!({
        "areas": [
            { "name": "Area 1", "address": 1, "lines": [
                { "name": "Line 1.1", "address": 1, "mediumRef": "MT-0" }
            ] }
        ],
        "buildings": [
            { "name": "House", "kind": "Building", "children": [
                { "name": "Ground floor", "kind": "Floor", "children": [
                    { "name": "Kitchen", "kind": "Room" }
                ] }
            ] }
        ],
        "groupRanges": [
            { "name": "Lighting", "main": 1, "middles": [
                { "name": "Ground floor", "middle": 0 }
            ] }
        ]
    })
}

/// Collects `name` of every node in a projected subtree, depth first.
fn names(nodes: &Value, children_key: &str, out: &mut Vec<String>) {
    for node in nodes.as_array().unwrap() {
        out.push(node["name"].as_str().unwrap().to_string());
        if let Some(children) = node.get(children_key) {
            names(children, children_key, out);
        }
    }
}

#[tokio::test]
async fn a_seeded_project_arrives_clean_with_its_structure_and_no_undo_history() {
    let (_dir, app) = app();
    let response = app
        .clone()
        .oneshot(post(
            "/api/project/new",
            json!({ "name": "Seeded", "installationName": "Main", "seed": seed() }),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let tree = body_json(response).await;
    assert_eq!(
        tree["is_modified"], false,
        "a new project is its own clean baseline"
    );
    assert_eq!(tree["can_undo"], false, "seeding leaves no undo entries");

    let installation = &tree["installations"][0];
    let mut topology = Vec::new();
    names(&installation["topology"], "lines", &mut topology);
    assert_eq!(topology, ["Area 1", "Line 1.1"]);
    let mut buildings = Vec::new();
    names(&installation["buildings"], "children", &mut buildings);
    assert_eq!(buildings, ["House", "Ground floor", "Kitchen"]);
    let mut ranges = Vec::new();
    names(&installation["group_ranges"], "children", &mut ranges);
    assert_eq!(ranges, ["Lighting", "Ground floor"]);

    let log = body_json(app.oneshot(get("/api/log")).await.unwrap()).await;
    let text = log.to_string();
    assert!(
        text.contains("1 area(s), 1 line(s), 3 building part(s) and 2 group range(s)"),
        "{text}"
    );
}

#[tokio::test]
async fn an_invalid_seed_replaces_nothing_and_names_the_node() {
    let (_dir, app) = app();
    let first = app
        .clone()
        .oneshot(post("/api/project/new", json!({ "name": "Keep me" })))
        .await
        .unwrap();
    assert_eq!(first.status(), StatusCode::OK);
    let before = body_json(app.clone().oneshot(get("/api/project")).await.unwrap()).await;

    let mut bad = seed();
    bad["areas"][0]["lines"]
        .as_array_mut()
        .unwrap()
        .push(json!({ "name": "Twin", "address": 1, "mediumRef": "MT-0" }));
    let response = app
        .clone()
        .oneshot(post(
            "/api/project/new",
            json!({ "name": "Replacement", "seed": bad, "discardChanges": true }),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let body = body_json(response).await;
    assert_eq!(body["kind"], "projectSeedInvalid");
    assert!(
        body["error"]
            .as_str()
            .unwrap()
            .starts_with("areas[0].lines[1]: "),
        "{body}"
    );

    let after = body_json(app.oneshot(get("/api/project")).await.unwrap()).await;
    assert_eq!(after["installations"], before["installations"]);
    assert_eq!(after["snapshot_revision"], before["snapshot_revision"]);
}

#[tokio::test]
async fn a_seed_with_an_unknown_field_or_kind_is_refused_before_anything_happens() {
    for bad in [
        json!({ "areas": [], "floors": [] }),
        json!({ "buildings": [ { "name": "Hall", "kind": "Corridor" } ] }),
        json!({ "areas": [ { "name": "A", "address": 1, "lines": [ { "name": "L", "address": 1 } ] } ] }),
    ] {
        let (_dir, app) = app();
        let response = app
            .clone()
            .oneshot(post(
                "/api/project/new",
                json!({ "name": "Never", "seed": bad }),
            ))
            .await
            .unwrap();
        assert!(
            response.status().is_client_error(),
            "{bad}: {}",
            response.status()
        );
        let current = app.oneshot(get("/api/project")).await.unwrap();
        assert_ne!(
            current.status(),
            StatusCode::OK,
            "{bad}: no project may have been created"
        );
    }
}

#[tokio::test]
async fn the_unsaved_changes_guard_still_comes_first_for_a_seeded_request() {
    let (_dir, app) = app();
    app.clone()
        .oneshot(post("/api/project/new", json!({})))
        .await
        .unwrap();
    let edited = app
        .clone()
        .oneshot(post(
            "/api/areas",
            json!({ "name": "Unsaved", "address": 4 }),
        ))
        .await
        .unwrap();
    assert_eq!(edited.status(), StatusCode::OK);

    let refused = app
        .clone()
        .oneshot(post("/api/project/new", json!({ "seed": seed() })))
        .await
        .unwrap();
    assert_eq!(refused.status(), StatusCode::CONFLICT);
    let tree = body_json(app.oneshot(get("/api/project")).await.unwrap()).await;
    let mut topology = Vec::new();
    names(
        &tree["installations"][0]["topology"],
        "lines",
        &mut topology,
    );
    assert_eq!(
        topology,
        ["Unsaved"],
        "the edited project survives the refusal"
    );
}

#[tokio::test]
async fn ranges_in_a_free_style_project_are_refused() {
    let (_dir, app) = app();
    let response = app
        .clone()
        .oneshot(post(
            "/api/project/new",
            json!({
                "groupAddressStyle": "Free",
                "seed": { "groupRanges": [ { "name": "M", "main": 1 } ] }
            }),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let body = body_json(response).await;
    assert!(
        body["error"].as_str().unwrap().contains("free-style"),
        "{body}"
    );
}
