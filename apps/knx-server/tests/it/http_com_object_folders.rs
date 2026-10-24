//! WP-06a: saved project folders reach the device detail through import, save, reopen and undo.
//!
//! Synthetic schema-23 project; the tree follows Project Schema 23
//! §1.2.5.21–23 (`Node/@Type` `Channel`/`Folder`, required `@RefId`). No
//! product database is open, so evaluated activation stays `NotEvaluated`.
use std::{collections::BTreeMap, sync::Arc};

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use serde_json::{json, Value};
use tower::ServiceExt;

const TREE: &str = concat!(
    r#"<GroupObjectTree GroupObjectInstances="O-1_R-1"><Nodes>"#,
    r#"<Node Type="Channel" RefId="CH-1" GroupObjectInstances="O-2_R-1"><Nodes>"#,
    r#"<Node Type="Folder" RefId="PB-3" GroupObjectInstances="O-3_R-1 O-4_R-1 MD-1_M-1_MI-1_O-3-0_R-1"/>"#,
    r#"</Nodes></Node>"#,
    r#"<Node Type="Folder" GroupObjectInstances="O-5_R-1"/>"#,
    r#"<Node Type="Folder" RefId="PB-6" GroupObjectInstances="O-6_R-1 O-1_R-1"/>"#,
    r#"</Nodes></GroupObjectTree>"#,
);

fn knxproj() -> Vec<u8> {
    let topology = format!(
        r#"<KNX xmlns="http://knx.org/xml/project/23"><Project Id="P-0001"><Installations><Installation InstallationId="0"><Topology><Area Id="P-0001-0_A-1" Address="1"><Line Id="P-0001-0_L-1" Address="1"><Segment Id="P-0001-0_S-1" Number="1"><DeviceInstance Id="P-0001-0_DI-1" Address="1" ProductRefId="M-0001_P-1" Hardware2ProgramRefId="M-0001_H-1_HP-1"><ModuleInstances><ModuleInstance Id="MD-1_M-1_MI-1" RefId="MD-1_M-1"/></ModuleInstances><ComObjectInstanceRefs><ComObjectInstanceRef RefId="O-4_R-1" IsActive="false"/></ComObjectInstanceRefs>{TREE}</DeviceInstance></Segment></Line></Area></Topology></Installation></Installations></Project></KNX>"#
    );
    let info = r#"<KNX xmlns="http://knx.org/xml/project/23"><Project Id="P-0001"><ProjectInformation Name="Synthetic folders"/></Project></KNX>"#;
    knx_testsupport::zip_with_entries(&[
        ("P-0001.signature", b""),
        ("P-0001/0.xml", topology.as_bytes()),
        ("P-0001/project.xml", info.as_bytes()),
    ])
}

async fn call(app: &axum::Router, method: &str, path: &str, body: Option<Value>) -> Value {
    let request = Request::builder().method(method).uri(path);
    let request = match body {
        Some(body) => request
            .header("content-type", "application/json")
            .body(Body::from(body.to_string())),
        None => request.body(Body::empty()),
    };
    let response = app.clone().oneshot(request.unwrap()).await.unwrap();
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), 16 * 1024 * 1024)
        .await
        .unwrap();
    assert_eq!(
        status,
        StatusCode::OK,
        "{method} {path}: {}",
        String::from_utf8_lossy(&bytes)
    );
    if bytes.is_empty() {
        return Value::Null;
    }
    serde_json::from_slice(&bytes).unwrap()
}

/// `source.ets_id` -> the `project_folder` the device detail reports for it.
async fn folders(app: &axum::Router, state: &knx_server::AppState) -> BTreeMap<String, Value> {
    let (device, ets_ids) = {
        let project = state.project.lock().unwrap();
        let project = project.as_ref().unwrap();
        let device = project.devices.iter().next().unwrap();
        let ets_ids: BTreeMap<u32, String> = project
            .devices
            .com_objects()
            .map(|com| (com.id.0, com.source.ets_id.clone()))
            .collect();
        (device.id.0, ets_ids)
    };
    let detail = call(app, "GET", &format!("/api/device/{device}"), None).await;
    detail["com_objects"]
        .as_array()
        .unwrap()
        .iter()
        .map(|com| {
            let id = com["id"].as_u64().unwrap() as u32;
            (ets_ids[&id].clone(), com["project_folder"].clone())
        })
        .collect()
}

fn refused(status: &str) -> Value {
    json!({ "status": status, "key": null, "path": [] })
}

#[tokio::test]
async fn saved_folders_survive_save_reopen_and_undo_without_inventing_any() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("synthetic.knxproj");
    std::fs::write(&source, knxproj()).unwrap();
    let state = Arc::new(knx_server::AppState {
        data_dir: dir.path().to_path_buf(),
        ..Default::default()
    });
    let app = knx_server::app(state.clone(), None);
    call(
        &app,
        "POST",
        "/api/project/import",
        Some(json!({ "path": source })),
    )
    .await;

    let expected: BTreeMap<String, Value> = [
        ("O-1_R-1", refused("Ambiguous")),
        ("O-2_R-1", refused("NoFolder")),
        (
            "O-3_R-1",
            json!({
                "status": "Assigned",
                "key": "Channel:4:CH-1/Folder:4:PB-3",
                "path": [
                    { "kind": "Channel", "ref_id": "CH-1" },
                    { "kind": "Folder", "ref_id": "PB-3" },
                ],
            }),
        ),
        ("O-4_R-1", refused("Inactive")),
        ("MD-1_M-1_MI-1_O-3-0_R-1", refused("ModuleObject")),
        ("O-5_R-1", refused("UnidentifiedNode")),
        (
            "O-6_R-1",
            json!({
                "status": "Assigned",
                "key": "Folder:4:PB-6",
                "path": [{ "kind": "Folder", "ref_id": "PB-6" }],
            }),
        ),
    ]
    .into_iter()
    .map(|(id, folder)| (id.to_owned(), folder))
    .collect();
    assert_eq!(folders(&app, &state).await, expected);

    // Native save and reopen: the retained tree is the only folder source.
    let native = dir.path().join("native.knxdb");
    call(
        &app,
        "POST",
        "/api/project/save-as",
        Some(json!({ "path": native })),
    )
    .await;
    call(
        &app,
        "POST",
        "/api/project/open",
        Some(json!({ "path": native })),
    )
    .await;
    assert_eq!(folders(&app, &state).await, expected);

    // An undoable object edit and its undo leave the placements alone: they
    // depend only on the retained tree and the objects' identities.
    let assigned = state
        .project
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .devices
        .com_objects()
        .find(|com| com.source.ets_id == "O-3_R-1")
        .unwrap()
        .id
        .0;
    let body = json!({ "comObjectId": assigned, "description": "edited" });
    let edited = call(&app, "POST", "/api/com-object-description", Some(body)).await;
    assert_eq!(edited["can_undo"], true);
    assert_eq!(folders(&app, &state).await, expected);
    call(&app, "POST", "/api/undo", None).await;
    assert_eq!(folders(&app, &state).await, expected);
}

#[tokio::test]
async fn a_device_without_a_retained_tree_reports_every_object_outside_it() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("minimal.knxproj");
    std::fs::write(&source, knx_testsupport::minimal_knxproj_bytes()).unwrap();
    let state = Arc::new(knx_server::AppState {
        data_dir: dir.path().to_path_buf(),
        ..Default::default()
    });
    let app = knx_server::app(state.clone(), None);
    call(
        &app,
        "POST",
        "/api/project/import",
        Some(json!({ "path": source })),
    )
    .await;
    // Schema 11 has no `GroupObjectTree`, so nothing is retained to read.
    let placed = folders(&app, &state).await;
    assert!(!placed.is_empty());
    for folder in placed.values() {
        assert_eq!(*folder, refused("NotInSavedTree"));
    }
}
