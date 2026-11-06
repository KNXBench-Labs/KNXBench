//! Verifies saved folders and independent product declarations through the native HTTP lifecycle.
//!
//! The legacy synthetic schema-23 control follows Project Schema 23
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
    let value = serde_json::from_slice(&bytes).unwrap();
    // Opt-in acceptance captures keep the actual router bytes, never a reconstructed response.
    if let Some(directory) = std::env::var_os("KNX_FOLDER_CAPTION_CAPTURE_DIR") {
        use std::io::Write;
        use std::sync::atomic::{AtomicU64, Ordering};
        static CAPTURE: AtomicU64 = AtomicU64::new(0);
        let sequence = CAPTURE.fetch_add(1, Ordering::Relaxed);
        let path_token: String = path
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
            .collect();
        let thread = std::thread::current();
        let test_token: String = thread
            .name()
            .unwrap_or("unnamed")
            .rsplit("::")
            .next()
            .unwrap()
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
            .collect();
        let file = std::path::Path::new(&directory).join(format!(
            "{test_token}-{sequence:04}-{method}-{path_token}.json"
        ));
        std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(file)
            .unwrap()
            .write_all(&bytes)
            .unwrap();
    }
    value
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

fn schema20_tree() -> String {
    let oversized = "é".repeat(2049);
    format!(
        concat!(
            r#"<GroupObjectTree xmlns="http://knx.org/xml/project/20" GroupObjectInstances="O-1_R-1"><Nodes>"#,
            r#"<Node Type="Channel" RefId="CH-1" Text="Channel only" GroupObjectInstances="O-2_R-1"><Nodes>"#,
            r#"<Node Type="Folder" RefId="PB-3" Text="Saved &amp; &lt;b&gt;{{{{ChNo}}}}&lt;/b&gt;" GroupObjectInstances="O-3_R-1 O-4_R-1 MD-1_M-1_MI-1_O-3-0_R-1"><Nodes>"#,
            r#"<Node Type="Folder" RefId="PB-4" Text="" GroupObjectInstances="O-5_R-1"/>"#,
            r#"</Nodes></Node></Nodes></Node>"#,
            r#"<Node Type="Folder" RefId="PB-6" Text="   " GroupObjectInstances="O-6_R-1"/>"#,
            r#"<Node Type="Folder" RefId="PB-7" GroupObjectInstances="O-7_R-1"/>"#,
            r#"<Node xmlns:q="urn:caption" Type="Folder" RefId="PB-8" q:Text="Qualified only" GroupObjectInstances="O-8_R-1"/>"#,
            r#"<Node Type="Folder" RefId="PB-9" Text="Left" GroupObjectInstances="O-9_R-1"><Nodes>"#,
            r#"<Node Type="Folder" RefId="PB-10" Text="Descendant" GroupObjectInstances="O-11_R-1"/>"#,
            r#"</Nodes></Node><Node Type="Folder" RefId="PB-9" Text="Right" GroupObjectInstances="O-10_R-1"/>"#,
            r#"<Node Type="Folder" RefId="PB-12" Text="{oversized}" GroupObjectInstances="O-12_R-1"/>"#,
            r#"</Nodes></GroupObjectTree>"#,
        ),
        oversized = oversized,
    )
}

fn schema20_knxproj(tree: &str) -> Vec<u8> {
    // Schema 20: DeviceInstance is a direct Line child, never a Segment child.
    let topology = format!(
        r#"<KNX xmlns="http://knx.org/xml/project/20"><Project Id="P-0001"><Installations><Installation InstallationId="0"><Topology><Area Id="P-0001-0_A-1" Address="1"><Line Id="P-0001-0_L-1" Address="1"><DeviceInstance Id="P-0001-0_DI-1" Address="1" ProductRefId="M-0001_P-1" Hardware2ProgramRefId="M-0001_H-1_HP-1"><ModuleInstances><ModuleInstance Id="MD-1_M-1_MI-1" RefId="MD-1_M-1"/></ModuleInstances><ComObjectInstanceRefs><ComObjectInstanceRef RefId="O-4_R-1" IsActive="false"/></ComObjectInstanceRefs>{tree}</DeviceInstance></Line></Area></Topology></Installation></Installations></Project></KNX>"#
    );
    let info = r#"<KNX xmlns="http://knx.org/xml/project/20"><Project Id="P-0001"><ProjectInformation Name="Synthetic saved text"/></Project></KNX>"#;
    knx_testsupport::zip_with_entries(&[
        ("P-0001.signature", b""),
        ("P-0001/0.xml", topology.as_bytes()),
        ("P-0001/project.xml", info.as_bytes()),
    ])
}

fn schema20_expected() -> BTreeMap<String, Value> {
    [
        ("O-1_R-1", refused("NoFolder")),
        ("O-2_R-1", refused("NoFolder")),
        (
            "O-3_R-1",
            json!({ "status": "Assigned", "key": "Channel:4:CH-1/Folder:4:PB-3", "path": [
                { "kind": "Channel", "ref_id": "CH-1", "text": "Channel only" },
                { "kind": "Folder", "ref_id": "PB-3", "text": "Saved & <b>{{ChNo}}</b>" },
            ] }),
        ),
        ("O-4_R-1", refused("Inactive")),
        ("MD-1_M-1_MI-1_O-3-0_R-1", refused("ModuleObject")),
        (
            "O-5_R-1",
            json!({ "status": "Assigned", "key": "Channel:4:CH-1/Folder:4:PB-3/Folder:4:PB-4", "path": [
                { "kind": "Channel", "ref_id": "CH-1", "text": "Channel only" },
                { "kind": "Folder", "ref_id": "PB-3", "text": "Saved & <b>{{ChNo}}</b>" },
                { "kind": "Folder", "ref_id": "PB-4", "text": "" },
            ] }),
        ),
        (
            "O-6_R-1",
            json!({ "status": "Assigned", "key": "Folder:4:PB-6", "path": [
                { "kind": "Folder", "ref_id": "PB-6", "text": "   " },
            ] }),
        ),
        (
            "O-7_R-1",
            json!({ "status": "Assigned", "key": "Folder:4:PB-7", "path": [
                { "kind": "Folder", "ref_id": "PB-7" },
            ] }),
        ),
        (
            "O-8_R-1",
            json!({ "status": "Assigned", "key": "Folder:4:PB-8", "path": [
                { "kind": "Folder", "ref_id": "PB-8" },
            ] }),
        ),
        ("O-9_R-1", refused("UnidentifiedNode")),
        ("O-10_R-1", refused("UnidentifiedNode")),
        ("O-11_R-1", refused("UnidentifiedNode")),
        (
            "O-12_R-1",
            json!({ "status": "Assigned", "key": "Folder:5:PB-12", "path": [
                { "kind": "Folder", "ref_id": "PB-12" },
            ] }),
        ),
    ]
    .into_iter()
    .map(|(id, value)| (id.to_owned(), value))
    .collect()
}

fn assert_schema20_retained(rows: &[knx_store::StoredOpaqueEntry], tree: &str) {
    use sha2::{Digest, Sha256};
    let trees: Vec<_> = rows
        .iter()
        .filter(|row| row.kind == "RetainedElement" && row.name == "GroupObjectTree")
        .collect();
    assert_eq!(trees.len(), 1);
    assert_eq!(trees[0].source_path, "P-0001/0.xml");
    assert_eq!(trees[0].xpath, "/KNX/Project/Installations/Installation/Topology/Area/Line/DeviceInstance[@Id='P-0001-0_DI-1']/GroupObjectTree");
    assert_eq!(trees[0].bytes, tree.as_bytes());
    assert_eq!(
        trees[0].sha256,
        format!("{:x}", Sha256::digest(tree.as_bytes()))
    );
}

/// Read-only assertions are relative to the state established by each real command.
async fn assert_schema20_display_is_read_only(
    app: &axum::Router,
    state: &knx_server::AppState,
    expected: &BTreeMap<String, Value>,
    tree: &str,
) {
    use std::sync::atomic::Ordering;
    let project = state.project.lock().unwrap().clone();
    let clean = state.clean_project.lock().unwrap().clone();
    let history = state.command_stack.lock().unwrap().clone();
    let generation = *state.history_generation.lock().unwrap();
    let revision = state.project_revision.load(Ordering::Relaxed);
    let opaque = state.opaque.lock().unwrap().clone();
    assert_schema20_retained(&opaque, tree);
    let before = call(app, "GET", "/api/project", None).await;
    assert_eq!(&folders(app, state).await, expected);
    assert_eq!(call(app, "GET", "/api/project", None).await, before);
    assert_eq!(*state.project.lock().unwrap(), project);
    assert_eq!(*state.clean_project.lock().unwrap(), clean);
    assert_eq!(*state.command_stack.lock().unwrap(), history);
    assert_eq!(*state.history_generation.lock().unwrap(), generation);
    assert_eq!(state.project_revision.load(Ordering::Relaxed), revision);
    assert_eq!(*state.opaque.lock().unwrap(), opaque);
}

#[tokio::test]
async fn saved_schema20_text_survives_import_native_save_reopen_edit_and_undo() {
    use sha2::{Digest, Sha256};
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("saved-text.knxproj");
    let tree = schema20_tree();
    let original = schema20_knxproj(&tree);
    let original_hash = Sha256::digest(&original);
    std::fs::write(&source, &original).unwrap();
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
    let expected = schema20_expected();
    {
        let project = state.project.lock().unwrap();
        let project = project.as_ref().unwrap();
        assert_eq!(project.info.ets_schema_version, 20);
        assert_eq!(project.devices.com_objects().count(), 13);
    }
    assert_eq!(
        state.command_stack.lock().unwrap().history_lengths(),
        (0, 0)
    );
    assert_schema20_display_is_read_only(&app, &state, &expected, &tree).await;
    let imported = state.project.lock().unwrap().clone().unwrap();

    let native = dir.path().join("saved-text.knxdb");
    call(
        &app,
        "POST",
        "/api/project/save-as",
        Some(json!({ "path": native })),
    )
    .await;
    assert_schema20_display_is_read_only(&app, &state, &expected, &tree).await;
    call(
        &app,
        "POST",
        "/api/project/open",
        Some(json!({ "path": native })),
    )
    .await;
    assert_schema20_display_is_read_only(&app, &state, &expected, &tree).await;
    let before_edit = state.project.lock().unwrap().clone().unwrap();
    assert_eq!(before_edit.info.ets_schema_version, 20);
    assert_eq!(before_edit.devices, imported.devices);
    let assigned = before_edit
        .devices
        .com_objects()
        .find(|com| com.source.ets_id == "O-3_R-1")
        .unwrap()
        .id;
    let lengths = state.command_stack.lock().unwrap().history_lengths();
    assert_eq!(lengths, (0, 0));
    let edited = call(
        &app,
        "POST",
        "/api/com-object-description",
        Some(json!({ "comObjectId": assigned.0, "description": "Edited object, not saved folder text" })),
    )
    .await;
    assert_eq!(edited["can_undo"], true);
    assert_eq!(edited["can_redo"], false);
    assert_eq!(edited["is_modified"], true);
    assert_eq!(
        state.command_stack.lock().unwrap().history_lengths(),
        (lengths.0 + 1, 0)
    );
    assert_eq!(
        state
            .project
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .devices
            .com_object(assigned)
            .unwrap()
            .description,
        knx_core::Override::Value(knx_core::Resolved {
            value: knx_core::Text::Literal("Edited object, not saved folder text".into()),
            layer: knx_core::Layer::UserEdit,
        })
    );
    assert_schema20_display_is_read_only(&app, &state, &expected, &tree).await;

    let undone = call(&app, "POST", "/api/undo", None).await;
    assert_eq!(undone["can_undo"], false);
    assert_eq!(undone["can_redo"], true);
    assert_eq!(undone["is_modified"], false);
    assert_eq!(
        state.command_stack.lock().unwrap().history_lengths(),
        (lengths.0, 1)
    );
    assert!(state
        .project
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .same_user_content_as(&before_edit));
    assert_schema20_display_is_read_only(&app, &state, &expected, &tree).await;

    // Read the persisted original-source row independently of the live projection.
    let conn =
        rusqlite::Connection::open_with_flags(&native, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .unwrap();
    assert_schema20_retained(&knx_store::load_opaque(&conn).unwrap(), &tree);
    conn.close().unwrap();
    let source_after = std::fs::read(&source).unwrap();
    assert_eq!(source_after, original);
    assert_eq!(Sha256::digest(&source_after), original_hash);
}

const CAPTION_AP: &str = "M-0001_A-0001-01-0000";
const CAPTION_HARDWARE: &str = r#"<KNX xmlns="http://knx.org/xml/project/20"><ManufacturerData><Manufacturer RefId="M-0001"><Hardware><Hardware Id="M-0001_H-1" Name="Fictional folder product" SerialNumber="S" VersionNumber="1"><Products><Product Id="M-0001_P-1"/></Products><Hardware2Programs><Hardware2Program Id="M-0001_H-1_HP-1"><ApplicationProgramRef RefId="M-0001_A-0001-01-0000"/></Hardware2Program></Hardware2Programs></Hardware></Hardware></Manufacturer></ManufacturerData></KNX>"#;

fn caption_program(literal: &str) -> String {
    let objects: String = (1..=12).map(|n| format!(r#"<ComObject Id="{CAPTION_AP}_O-{n}" Number="{n}" Name="Object {n}" ObjectSize="1 Bit" CommunicationFlag="Enabled"/>"#)).collect();
    let refs: String = (1..=12)
        .map(|n| {
            format!(r#"<ComObjectRef Id="{CAPTION_AP}_O-{n}_R-1" RefId="{CAPTION_AP}_O-{n}"/>"#)
        })
        .collect();
    let active: String = (1..=12)
        .filter(|n| *n != 4)
        .map(|n| format!(r#"<ComObjectRefRef RefId="{CAPTION_AP}_O-{n}_R-1"/>"#))
        .collect();
    format!(
        r#"<KNX xmlns="http://knx.org/xml/project/20"><ManufacturerData><Manufacturer RefId="M-0001"><ApplicationPrograms><ApplicationProgram Id="{CAPTION_AP}" Name="Fictional" ApplicationVersion="1" MaskVersion="MV-0701"><Static><ComObjectTable>{objects}</ComObjectTable><ComObjectRefs>{refs}</ComObjectRefs></Static><ModuleDefs><ModuleDef Id="{CAPTION_AP}_MD-1"><Dynamic><ParameterBlock Id="{CAPTION_AP}_PB-7" Text="Earlier module shadow" ShowInComObjectTree="true"/></Dynamic></ModuleDef></ModuleDefs><Dynamic>{active}<Channel Id="{CAPTION_AP}_CH-1"><ParameterBlock Id="{CAPTION_AP}_PB-3" Text="{literal}" ShowInComObjectTree="true"><ParameterBlock Id="{CAPTION_AP}_PB-4" Text="Blank saved remains blank" ShowInComObjectTree="1"/></ParameterBlock></Channel><ParameterBlock Id="{CAPTION_AP}_PB-6" Text="Whitespace saved remains whitespace" ShowInComObjectTree="true"/><ParameterBlock Id="{CAPTION_AP}_PB-7" Text="Equal literal &amp; &lt;b&gt;inert&lt;/b&gt;" ShowInComObjectTree="true"/><ParameterBlock Id="{CAPTION_AP}_PB-8" Text="Equal literal &amp; &lt;b&gt;inert&lt;/b&gt;" ShowInComObjectTree="true"/><ParameterBlock Id="{CAPTION_AP}_PB-12" Text="Product declaration" ShowInComObjectTree="true"/></Dynamic></ApplicationProgram></ApplicationPrograms></Manufacturer></ManufacturerData></KNX>"#
    )
}

fn caption_expected(program: &str, literal: &str) -> BTreeMap<String, Value> {
    let mut expected = schema20_expected();
    let sha = knx_productdb::sha256_hex(program.as_bytes());
    for folder in expected.values_mut() {
        for node in folder["path"].as_array_mut().unwrap() {
            if node["kind"] != "Folder" {
                continue;
            }
            let text = match node["ref_id"].as_str().unwrap() {
                "PB-3" => literal,
                "PB-4" => "Blank saved remains blank",
                "PB-6" => "Whitespace saved remains whitespace",
                "PB-7" | "PB-8" => "Equal literal & <b>inert</b>",
                "PB-12" => "Product declaration",
                other => panic!("unexpected admitted synthetic folder {other}"),
            };
            node["product_caption"] = json!({ "source": "ProductDeclaration", "text": text,
                "application_program_id": CAPTION_AP,
                "parameter_block_id": format!("{}_{}", CAPTION_AP, node["ref_id"].as_str().unwrap()),
                "source_sha256": sha });
        }
    }
    expected
}

fn caption_state(
    dir: &std::path::Path,
    program: &str,
    missing_source: bool,
) -> Arc<knx_server::AppState> {
    caption_state_with_identity_sources(dir, program, missing_source, 0)
}

fn caption_state_with_identity_sources(
    dir: &std::path::Path,
    program: &str,
    missing_source: bool,
    extra_sources: usize,
) -> Arc<knx_server::AppState> {
    caption_state_with_sources(
        dir,
        program,
        missing_source,
        extra_sources,
        CAPTION_HARDWARE,
    )
}

fn caption_state_with_sources(
    dir: &std::path::Path,
    program: &str,
    missing_source: bool,
    extra_sources: usize,
    hardware: &str,
) -> Arc<knx_server::AppState> {
    let db = knx_productdb::open_and_migrate(&dir.join("products.sqlite")).unwrap();
    super::native_source_fixture::install(
        &db,
        20,
        ("M-0001/Hardware.xml", hardware.as_bytes()),
        ("M-0001/Application.xml", program.as_bytes()),
    );
    for index in 0..extra_sources {
        // Outside the ApplicationProgram: distinct valid blobs, identical measured identity.
        let equivalent = program.replace(
            "</KNX>",
            &format!("<!-- equivalent caption source {index} --></KNX>"),
        );
        knx_productdb::ingest_file(
            &db,
            &format!("M-0001/Equivalent-{index}.xml"),
            equivalent.as_bytes(),
        )
        .unwrap();
    }
    assert_eq!(
        db.query_row(
            "SELECT count(*) FROM source_identity WHERE table_name='application_program' AND logical_id=?1",
            [CAPTION_AP],
            |row| row.get::<_, i64>(0),
        ).unwrap(),
        i64::try_from(extra_sources + 1).unwrap(),
    );
    if missing_source {
        db.execute_batch("PRAGMA foreign_keys=OFF").unwrap();
        db.execute(
            "DELETE FROM source_file WHERE sha256=?1",
            [knx_productdb::sha256_hex(program.as_bytes())],
        )
        .unwrap();
    }
    Arc::new(knx_server::AppState {
        data_dir: dir.to_path_buf(),
        product_db: Some(
            knx_server::CatalogueOwner::adopt_opened(db, dir.join("products.sqlite")).unwrap(),
        ),
        ..Default::default()
    })
}

#[tokio::test]
async fn installed_literal_declarations_preserve_saved_text_keys_bytes_history_and_native_lifecycle(
) {
    use sha2::{Digest, Sha256};
    let dir = tempfile::tempdir().unwrap();
    let program = caption_program("Installed declaration");
    let state = caption_state(dir.path(), &program, false);
    let app = knx_server::app(state.clone(), None);
    let tree = schema20_tree();
    let bytes = schema20_knxproj(&tree);
    let original_sha = Sha256::digest(&bytes);
    let source = dir.path().join("product-captions.knxproj");
    std::fs::write(&source, &bytes).unwrap();
    call(
        &app,
        "POST",
        "/api/project/import",
        Some(json!({"path":source})),
    )
    .await;
    let expected = caption_expected(&program, "Installed declaration");
    assert_schema20_display_is_read_only(&app, &state, &expected, &tree).await;
    // Same declared text is not a shared saved identity; blank saved text stays present.
    assert_ne!(expected["O-7_R-1"]["key"], expected["O-8_R-1"]["key"]);
    assert_eq!(expected["O-5_R-1"]["path"][2]["text"], "");
    assert_eq!(expected["O-6_R-1"]["path"][0]["text"], "   ");
    assert!(expected["O-7_R-1"]["path"][0].get("text").is_none());
    let native = dir.path().join("product-captions.knxdb");
    call(
        &app,
        "POST",
        "/api/project/save-as",
        Some(json!({"path":native})),
    )
    .await;
    super::native_source_fixture::assert_carried(&state, &native);
    assert_schema20_display_is_read_only(&app, &state, &expected, &tree).await;
    call(
        &app,
        "POST",
        "/api/project/open",
        Some(json!({"path":native})),
    )
    .await;
    assert_schema20_display_is_read_only(&app, &state, &expected, &tree).await;
    let before = state.project.lock().unwrap().clone().unwrap();
    let assigned = before
        .devices
        .com_objects()
        .find(|c| c.source.ets_id == "O-3_R-1")
        .unwrap()
        .id;
    let lengths = state.command_stack.lock().unwrap().history_lengths();
    let result = call(
        &app,
        "POST",
        "/api/com-object-description",
        Some(json!({"comObjectId":assigned.0,"description":"Only the object changes"})),
    )
    .await;
    assert_eq!(result["can_undo"], true);
    assert_eq!(
        state.command_stack.lock().unwrap().history_lengths(),
        (lengths.0 + 1, 0)
    );
    assert_schema20_display_is_read_only(&app, &state, &expected, &tree).await;
    call(&app, "POST", "/api/undo", None).await;
    assert!(state
        .project
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .same_user_content_as(&before));
    assert_eq!(
        state.command_stack.lock().unwrap().history_lengths(),
        (lengths.0, 1)
    );
    assert_schema20_display_is_read_only(&app, &state, &expected, &tree).await;
    let conn =
        rusqlite::Connection::open_with_flags(&native, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .unwrap();
    assert_schema20_retained(&knx_store::load_opaque(&conn).unwrap(), &tree);
    conn.close().unwrap();
    assert_eq!(std::fs::read(&source).unwrap(), bytes);
    assert_eq!(
        Sha256::digest(std::fs::read(&source).unwrap()),
        original_sha
    );
}

#[tokio::test]
async fn absent_or_changed_installed_source_changes_only_optional_declaration_metadata() {
    let tree = schema20_tree();
    for (literal, missing) in [
        ("First installed", false),
        ("Changed installed", false),
        ("Missing source", true),
    ] {
        let dir = tempfile::tempdir().unwrap();
        let program = caption_program(literal);
        let state = caption_state(dir.path(), &program, missing);
        let app = knx_server::app(state.clone(), None);
        let source = dir.path().join("source-control.knxproj");
        std::fs::write(&source, schema20_knxproj(&tree)).unwrap();
        call(
            &app,
            "POST",
            "/api/project/import",
            Some(json!({"path":source})),
        )
        .await;
        let expected = if missing {
            schema20_expected()
        } else {
            caption_expected(&program, literal)
        };
        assert_schema20_display_is_read_only(&app, &state, &expected, &tree).await;
        let mut plain = folders(&app, &state).await;
        for folder in plain.values_mut() {
            for node in folder["path"].as_array_mut().unwrap() {
                node.as_object_mut().unwrap().remove("product_caption");
            }
        }
        assert_eq!(plain, schema20_expected());
    }
}

#[tokio::test]
async fn caption_identity_admission_exhaustion_keeps_saved_projection_and_catalogue_read_only() {
    use knx_productdb::com_object_folder_captions::{
        folder_captions, CaptionEvidence, CaptionIssue, MAX_IDENTITY_ADMISSION_ROWS,
    };
    let tree = schema20_tree();
    let program = caption_program("Bounded declaration");
    for extra_sources in [0, MAX_IDENTITY_ADMISSION_ROWS] {
        let dir = tempfile::tempdir().unwrap();
        let state = caption_state_with_identity_sources(dir.path(), &program, false, extra_sources);
        let app = knx_server::app(state.clone(), None);
        let source = dir.path().join("bounded-captions.knxproj");
        let original = schema20_knxproj(&tree);
        std::fs::write(&source, &original).unwrap();
        call(
            &app,
            "POST",
            "/api/project/import",
            Some(json!({"path":source})),
        )
        .await;
        let owner = state.product_db.as_ref().unwrap();
        let changes = owner
            .with_read(|db| {
                let before = db.total_changes();
                let evidence = folder_captions(db, "M-0001_P-1", "M-0001_H-1_HP-1", 20)?;
                if extra_sources == 0 {
                    assert!(matches!(evidence, CaptionEvidence::Declared(ref captions) if !captions.is_empty()));
                } else {
                    assert_eq!(evidence, CaptionEvidence::Unavailable(CaptionIssue::ResourceLimit));
                }
                assert_eq!(db.total_changes(), before);
                Ok::<_, knx_productdb::ProductDbError>(before)
            })
            .unwrap()
            .unwrap();
        let expected = if extra_sources == 0 {
            caption_expected(&program, "Bounded declaration")
        } else {
            schema20_expected()
        };
        // Includes exact statuses, keys, full paths and saved blank/whitespace text,
        // plus project revision, clean state, history and retained opaque-source equality.
        assert_schema20_display_is_read_only(&app, &state, &expected, &tree).await;
        let after = owner
            .with_read(|db| Ok::<_, rusqlite::Error>(db.total_changes()))
            .unwrap()
            .unwrap();
        assert_eq!(after, changes);
        assert_eq!(std::fs::read(&source).unwrap(), original);
    }
}

#[tokio::test]
async fn unsupported_product_source_never_authorizes_captions_or_changes_saved_state() {
    let tree = schema20_tree();
    let program = caption_program("Canonical declaration");
    for (hardware, canonical) in [
        (CAPTION_HARDWARE.to_string(), true),
        (
            CAPTION_HARDWARE.replace("<Product Id=", "<Product xmlns=\"urn:foreign\" Id="),
            false,
        ),
        (
            CAPTION_HARDWARE
                .replace("<Products>", "<Wrapper><Products>")
                .replace("</Products>", "</Products></Wrapper>"),
            false,
        ),
    ] {
        let dir = tempfile::tempdir().unwrap();
        let state = caption_state_with_sources(dir.path(), &program, false, 0, &hardware);
        let app = knx_server::app(state.clone(), None);
        let source = dir.path().join("product-owner.knxproj");
        let original = schema20_knxproj(&tree);
        std::fs::write(&source, &original).unwrap();
        call(
            &app,
            "POST",
            "/api/project/import",
            Some(json!({"path":source})),
        )
        .await;
        let owner = state.product_db.as_ref().unwrap();
        let changes = owner
            .with_read(|db| Ok::<_, rusqlite::Error>(db.total_changes()))
            .unwrap()
            .unwrap();
        let expected = if canonical {
            caption_expected(&program, "Canonical declaration")
        } else {
            schema20_expected()
        };
        assert_schema20_display_is_read_only(&app, &state, &expected, &tree).await;
        assert_eq!(
            owner
                .with_read(|db| Ok::<_, rusqlite::Error>(db.total_changes()))
                .unwrap()
                .unwrap(),
            changes
        );
        assert_eq!(std::fs::read(&source).unwrap(), original);
    }
}
