//! MODEL-01: CSV group-address exchange can target any installation.
//!
//! Installations are separate infrastructures (ADR-0038), so the same group
//! address may exist in two of them. `installationId` on
//! `POST /api/group-addresses/csv-import` / `csv-export` selects which one is
//! read and written; without it the first installation stays the target,
//! exactly as before. A destructive preview's confirmation token is bound to
//! the installation it was previewed for.

use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use knx_core::{
    CompletionStatus, GroupAddress, GroupAddressEntry, GroupAddressId, GroupRange, GroupRangeId,
    IdAllocators, Installation, InstallationId, Language, Project, SourceRef, Topology,
};
use serde_json::{json, Value};
use tower::ServiceExt;

fn source(tag: &str) -> SourceRef {
    SourceRef {
        path: tag.into(),
        ets_id: tag.into(),
    }
}

fn installation(id: u8, name: &str) -> Installation {
    Installation {
        id: InstallationId(id),
        name: name.into(),
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
    }
}

fn ga(id: u32, address: &str, name: &str, range: Option<u32>) -> GroupAddressEntry {
    GroupAddressEntry {
        id: GroupAddressId(id),
        source: source(&format!("ga{id}")),
        name: name.into(),
        address: GroupAddress::parse(address, knx_core::GroupAddressStyle::ThreeLevel).unwrap(),
        central: false,
        unfiltered: false,
        range: range.map(GroupRangeId),
    }
}

/// Home (installation 0) and Garage (installation 1) both use `1/1/1`;
/// only Garage has a group range, `1/1/0`–`1/1/255`.
fn app() -> axum::Router {
    let mut project = Project::new(Language("en".into()));
    let mut home = installation(0, "Home");
    home.group_addresses
        .push(ga(1, "1/1/1", "Home light", None));
    let mut garage = installation(1, "Garage");
    garage.group_ranges.push(GroupRange {
        id: GroupRangeId(1),
        source: source("gr1"),
        name: "Garage lights".into(),
        start: GroupAddress::parse("1/1/0", knx_core::GroupAddressStyle::ThreeLevel).unwrap(),
        end: GroupAddress::parse("1/1/255", knx_core::GroupAddressStyle::ThreeLevel).unwrap(),
        parent: None,
        children: vec![],
    });
    garage
        .group_addresses
        .push(ga(2, "1/1/1", "Garage light", Some(1)));
    project.installations.extend([home, garage]);
    project
        .ids
        .raise_to(&IdAllocators::from_counts(0, 0, 0, 0, 1, 2, 0, 0, 0));
    let state = knx_server::AppState::default();
    *state.project.lock().unwrap() = Some(project);
    knx_server::app(Arc::new(state), None)
}

async fn call(app: &axum::Router, uri: &str, body: Value) -> (StatusCode, Value) {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(uri)
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let value = serde_json::from_slice(&bytes)
        .unwrap_or_else(|_| Value::String(String::from_utf8_lossy(&bytes).into_owned()));
    (status, value)
}

fn names(tree: &Value, installation: usize) -> Vec<String> {
    let mut names: Vec<String> = tree["installations"][installation]["group_addresses"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|node| {
            // Ranges nest addresses; flatten whatever shape the tree uses.
            let mut out = Vec::new();
            collect_names(node, &mut out);
            out
        })
        .collect();
    names.sort();
    names
}

fn collect_names(node: &Value, out: &mut Vec<String>) {
    if let (Some(name), Some(_)) = (node["name"].as_str(), node["address"].as_str()) {
        out.push(name.to_string());
    }
    for key in ["children", "addresses", "group_addresses"] {
        if let Some(children) = node[key].as_array() {
            for child in children {
                collect_names(child, out);
            }
        }
    }
}

#[tokio::test]
async fn importing_into_the_garage_edits_only_the_garage() {
    let app = app();
    let dir = tempfile::tempdir().unwrap();
    let csv = dir.path().join("garage.csv");
    std::fs::write(
        &csv,
        "Address,Name\n1/1/1,Garage light renamed\n1/1/5,Garage new\n1/2/9,Garage loose\n",
    )
    .unwrap();
    let (status, body) = call(
        &app,
        "/api/group-addresses/csv-import",
        json!({ "path": csv.to_string_lossy(), "installationId": 1 }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["applied"], true, "{body}");
    assert_eq!(body["report"]["created"], 2, "{body}");
    assert_eq!(body["report"]["updated"], 1, "{body}");
    // 1/1/5 lands in the Garage's own range; only 1/2/9 (row 4) has none.
    let warnings: Vec<_> = body["report"]["problems"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|p| {
            p["detail"]
                .as_str()
                .unwrap()
                .contains("created without one")
        })
        .map(|p| p["row"].clone())
        .collect();
    assert_eq!(warnings, [json!(4)], "{body}");
    let tree = &body["tree"];
    assert_eq!(names(tree, 0), ["Home light"], "{tree}");
    // A range-less create still goes to the chosen installation.
    assert_eq!(
        names(tree, 1),
        ["Garage light renamed", "Garage loose", "Garage new"],
        "{tree}"
    );
}

#[tokio::test]
async fn exporting_the_garage_writes_only_the_garage() {
    let app = app();
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("garage-out.csv");
    let (status, body) = call(
        &app,
        "/api/group-addresses/csv-export",
        json!({ "path": out.to_string_lossy(), "installationId": 1 }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let text = std::fs::read_to_string(&out).unwrap();
    assert!(text.contains("Garage light"), "{text}");
    assert!(!text.contains("Home light"), "{text}");

    let home = dir.path().join("home-out.csv");
    let (status, _) = call(
        &app,
        "/api/group-addresses/csv-export",
        json!({ "path": home.to_string_lossy() }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let text = std::fs::read_to_string(&home).unwrap();
    assert!(
        text.contains("Home light") && !text.contains("Garage light"),
        "{text}"
    );
}

#[tokio::test]
async fn an_unknown_installation_is_refused_for_import_and_export() {
    let app = app();
    let dir = tempfile::tempdir().unwrap();
    let csv = dir.path().join("x.csv");
    std::fs::write(&csv, "Address,Name\n1/1/9,X\n").unwrap();
    let (status, body) = call(
        &app,
        "/api/group-addresses/csv-import",
        json!({ "path": csv.to_string_lossy(), "installationId": 7 }),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
    assert!(body.to_string().contains("installation 7"), "{body}");
    let out = dir.path().join("y.csv");
    let (status, body) = call(
        &app,
        "/api/group-addresses/csv-export",
        json!({ "path": out.to_string_lossy(), "installationId": 7 }),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
    assert!(!out.exists(), "nothing written for an unknown installation");
}

/// A delete previewed for Home must not be confirmable against the Garage,
/// where the same address names a different group address.
#[tokio::test]
async fn a_confirmation_token_is_bound_to_its_installation() {
    let app = app();
    let dir = tempfile::tempdir().unwrap();
    let csv = dir.path().join("delete.csv");
    std::fs::write(&csv, "Address,Action,Name\n1/1/1,delete,Home light\n").unwrap();
    let (status, preview) = call(
        &app,
        "/api/group-addresses/csv-import",
        json!({ "path": csv.to_string_lossy(), "installationId": 0 }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{preview}");
    assert_eq!(preview["applied"], false, "{preview}");
    let token = preview["confirmationToken"].as_str().unwrap().to_string();

    let (status, body) = call(
        &app,
        "/api/group-addresses/csv-import",
        json!({ "path": csv.to_string_lossy(), "installationId": 1, "confirmationToken": token }),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");

    let (status, body) = call(
        &app,
        "/api/group-addresses/csv-import",
        json!({ "path": csv.to_string_lossy(), "installationId": 0, "confirmationToken": token }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["applied"], true, "{body}");
    assert!(names(&body["tree"], 0).is_empty(), "{body}");
    assert_eq!(names(&body["tree"], 1), ["Garage light"]);
}
