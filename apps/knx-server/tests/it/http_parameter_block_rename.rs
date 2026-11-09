//! A conditional block rename follows the stored selector through the real parameter routes.
//!
//! Fictional program. The replacement is display only: edits go through the existing validated
//! parameter command, undo/redo move the caption with the value, reads change nothing.

use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use knx_core::{
    CommissioningState, CompletionStatus, DeviceId, DeviceInstance, Installation, InstallationId,
    Language, ParameterInstance, ParameterInstanceId, Project, SourceRef, Topology,
};
use serde_json::{json, Value};
use tower::ServiceExt;

const HARDWARE: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<KNX xmlns="http://knx.org/xml/project/11"><ManufacturerData><Manufacturer RefId="M-0001">
<Hardware><Hardware Id="H-1" Name="X" SerialNumber="S" VersionNumber="1">
<Products><Product Id="M-0001_P-1" /></Products>
<Hardware2Programs><Hardware2Program Id="H-1_HP-1" MediumTypes="MT-0">
<ApplicationProgramRef RefId="A-1" /></Hardware2Program></Hardware2Programs>
</Hardware></Hardware></Manufacturer></ManufacturerData></KNX>"#;

/// `PB-1` is the writable page; `PB-2` is read-only and renamed by a stored selector;
/// `PB-3` has two leaves that are active together, so neither may win.
const PROGRAM: &str = r##"<?xml version="1.0" encoding="utf-8"?>
<KNX xmlns="http://knx.org/xml/project/11"><ManufacturerData><Manufacturer RefId="M-0001">
<ApplicationPrograms><ApplicationProgram Id="A-1" Name="Fictional" ApplicationVersion="1" MaskVersion="MV-0701">
<Static><ParameterTypes>
<ParameterType Id="PT-N" Name="Number"><TypeNumber SizeInBit="8" Type="unsignedInt" minInclusive="0" maxInclusive="255" /></ParameterType>
</ParameterTypes><Parameters>
<Parameter Id="P-S" Name="Selector" ParameterType="PT-N" Value="0" />
<Parameter Id="P-R" Name="ReadOnlySelector" ParameterType="PT-N" Value="0" />
<Parameter Id="P-C" Name="Competing" ParameterType="PT-N" Value="1" />
</Parameters><ParameterRefs>
<ParameterRef Id="R-S" RefId="P-S" /><ParameterRef Id="R-R" RefId="P-R" /><ParameterRef Id="R-C" RefId="P-C" />
</ParameterRefs></Static><Dynamic>
<ParameterBlock Id="PB-1" Text="Base page"><ParameterRefRef RefId="R-S" />
<choose ParamRefId="R-S">
<when test="1"><ParameterBlockRename Id="PR-1" RefId="PB-1" Text="fixture_switch_py page" /></when>
<when test="2"><ParameterBlockRename Id="PR-2" RefId="PB-1" Text="fixture_switch_pz page" /></when>
</choose></ParameterBlock>
<ParameterBlock Id="PB-2" Text="Base locked" Access="Read"><ParameterRefRef RefId="R-R" />
<choose ParamRefId="R-R"><when test="1"><ParameterBlockRename Id="PR-3" RefId="PB-2" Text="Renamed locked" /></when></choose></ParameterBlock>
<ParameterBlock Id="PB-3" Text="Base competing"><ParameterRefRef RefId="R-C" />
<choose ParamRefId="R-C"><when test="1">
<ParameterBlockRename Id="PR-4" RefId="PB-3" Text="Rival one" /><ParameterBlockRename Id="PR-5" RefId="PB-3" Text="Rival two" />
</when></choose></ParameterBlock>
</Dynamic></ApplicationProgram></ApplicationPrograms></Manufacturer></ManufacturerData></KNX>"##;

fn products() -> (tempfile::TempDir, knx_productdb::Connection) {
    let dir = tempfile::tempdir().unwrap();
    let conn = knx_productdb::open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
    super::native_source_fixture::install(
        &conn,
        11,
        ("M-0001/Hardware.xml", HARDWARE.as_bytes()),
        ("M-0001/A.xml", PROGRAM.as_bytes()),
    );
    conn.execute_batch(
        "INSERT INTO translation (scope,scope_id,language,ref_id,attribute_name,text) VALUES
         ('Program','A-1','de-DE','PB-1','Text','Basisseite'),
         ('Program','A-1','de-DE','PR-1','Text','probe_de_py'),
         ('Program','A-1','de-DE','PR-2','Text','probe_de_pz'),
         ('Program','A-1','fr-FR','PB-1','Text','Page de base'),
         ('Program','A-1','de-DE','PB-2','Text','Gesperrt'),
         ('Program','A-1','de-DE','PR-3','Text','Umbenannt gesperrt');",
    )
    .unwrap();
    (dir, conn)
}

#[allow(clippy::field_reassign_with_default)]
fn state(products: knx_productdb::Connection, stored: &[(&str, &str)]) -> knx_server::AppState {
    let mut project = Project::new(Language("en".into()));
    project.devices.insert(DeviceInstance {
        id: DeviceId(1),
        source: SourceRef {
            path: "device-1".into(),
            ets_id: "device-1".into(),
        },
        name: "Device 1".into(),
        description: None,
        address: None,
        product_ref: "M-0001_P-1".into(),
        program_ref: "H-1_HP-1".into(),
        commissioning: CommissioningState::default(),
        visibility_calculated: true,
        com_objects: vec![],
        binary_data: vec![],
    });
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
        parameters: stored
            .iter()
            .enumerate()
            .map(|(i, (ets_id, raw))| ParameterInstance {
                id: ParameterInstanceId(i as u32 + 1),
                device: DeviceId(1),
                source: SourceRef {
                    path: "device-1".into(),
                    ets_id: (*ets_id).into(),
                },
                raw: (*raw).into(),
            })
            .collect(),
    });
    let count = stored.len() as u32;
    project.ids = knx_core::project::IdAllocators::from_counts(1, 0, 0, 0, 0, 0, 0, count, 0);
    let mut state = knx_server::AppState::default();
    state.product_db = Some(
        knx_server::CatalogueOwner::adopt_opened(products, std::path::PathBuf::new()).unwrap(),
    );
    *state.project.lock().unwrap() = Some(project);
    state
}

async fn call(app: axum::Router, request: Request<Body>) -> (StatusCode, Value) {
    let response = app.oneshot(request).await.unwrap();
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

async fn panel(app: &axum::Router, language: Option<&str>) -> Value {
    let uri = match language {
        Some(language) => format!("/api/device/1/parameters?language={language}"),
        None => "/api/device/1/parameters".to_string(),
    };
    let request = Request::builder().uri(uri).body(Body::empty()).unwrap();
    let (status, dto) = call(app.clone(), request).await;
    assert_eq!(status, StatusCode::OK);
    dto
}

async fn set(app: &axum::Router, ets_id: &str, raw: &str) -> StatusCode {
    let request = Request::builder()
        .method("POST")
        .uri("/api/device/1/parameters")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({ "etsId": ets_id, "raw": raw }).to_string(),
        ))
        .unwrap();
    call(app.clone(), request).await.0
}

async fn history(app: &axum::Router, step: &str) {
    let request = Request::post(format!("/api/{step}"))
        .body(Body::empty())
        .unwrap();
    assert_eq!(call(app.clone(), request).await.0, StatusCode::OK);
}

/// `(key, label)` of the group every field of `etsId` sits in.
fn group_of(dto: &Value, ets_id: &str) -> (String, Value) {
    let field = dto["sections"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|s| s["fields"].as_array().unwrap())
        .find(|f| f["etsId"] == ets_id)
        .unwrap_or_else(|| panic!("{ets_id} is not shown"));
    (
        field["group"]["key"].as_str().unwrap().to_string(),
        field["group"]["label"].clone(),
    )
}

fn unrecognized(dto: &Value) -> usize {
    dto["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|d| d["kind"] == "unrecognizedNode")
        .count()
}

#[tokio::test]
async fn edits_undo_and_redo_move_the_caption_with_the_value_under_a_stable_identity() {
    let (_dir, products) = products();
    let state = Arc::new(state(products, &[]));
    let app = knx_server::app(state.clone(), None);
    let base = panel(&app, None).await;
    assert_eq!(group_of(&base, "R-S"), ("PB-1".into(), json!("Base page")));
    // Reading changed nothing: project, product source and history.
    let before = state.project.lock().unwrap().clone();
    for _ in 0..3 {
        assert_eq!(panel(&app, None).await, base);
        assert_eq!(
            panel(&app, Some("de-DE")).await,
            panel(&app, Some("de")).await
        );
    }
    assert_eq!(*state.project.lock().unwrap(), before);

    let mut seen = Vec::new();
    for (raw, english, german) in [
        ("1", "fixture_switch_py page", "probe_de_py"),
        ("2", "fixture_switch_pz page", "probe_de_pz"),
        ("0", "Base page", "Basisseite"),
    ] {
        assert_eq!(set(&app, "R-S", raw).await, StatusCode::OK);
        let (en, de) = (panel(&app, None).await, panel(&app, Some("de-DE")).await);
        assert_eq!(group_of(&en, "R-S"), ("PB-1".into(), json!(english)));
        assert_eq!(group_of(&de, "R-S"), ("PB-1".into(), json!(german)));
        // Only the competing block's two leaves are reported; an admitted rename is not.
        assert_eq!(unrecognized(&en), 2);
        seen.push(english);
    }
    assert_eq!(
        seen,
        [
            "fixture_switch_py page",
            "fixture_switch_pz page",
            "Base page"
        ]
    );

    // 0 -> undo -> 2, undo -> 1, redo -> 2, redo -> 0, one caption per step.
    for (step, expected) in [
        ("undo", "fixture_switch_pz page"),
        ("undo", "fixture_switch_py page"),
        ("redo", "fixture_switch_pz page"),
        ("redo", "Base page"),
    ] {
        history(&app, step).await;
        let dto = panel(&app, None).await;
        assert_eq!(
            group_of(&dto, "R-S"),
            ("PB-1".into(), json!(expected)),
            "{step}"
        );
    }
}

#[tokio::test]
async fn the_replacement_uses_its_own_translation_and_never_the_blocks() {
    let (_dir, products) = products();
    let state = Arc::new(state(products, &[("R-S", "1")]));
    let app = knx_server::app(state, None);
    // The leaf has a German entry: it wins over the block's "Basisseite".
    assert_eq!(
        group_of(&panel(&app, Some("de")).await, "R-S").1,
        json!("probe_de_py")
    );
    // French translates the block only. The active leaf shows its source text, not "Page de base".
    assert_eq!(
        group_of(&panel(&app, Some("fr-FR")).await, "R-S").1,
        json!("fixture_switch_py page")
    );
    // An unrelated language falls back to the source leaf text too.
    assert_eq!(
        group_of(&panel(&app, Some("it")).await, "R-S").1,
        json!("fixture_switch_py page")
    );
}

#[tokio::test]
async fn a_restricted_block_is_renamed_without_widening_its_access() {
    let (_dir, products) = products();
    let state = Arc::new(state(products, &[("R-R", "1")]));
    let app = knx_server::app(state, None);
    for (language, label) in [
        (None, "Renamed locked"),
        (Some("de-DE"), "Umbenannt gesperrt"),
    ] {
        let dto = panel(&app, language).await;
        assert_eq!(group_of(&dto, "R-R"), ("PB-2".into(), json!(label)));
        let field = dto["sections"][0]["fields"]
            .as_array()
            .unwrap()
            .iter()
            .find(|f| f["etsId"] == "R-R")
            .unwrap()
            .clone();
        assert_eq!(field["blockAccess"], "Read");
    }
    // The read-only rule is the unchanged one: the selector itself cannot be written.
    assert_eq!(set(&app, "R-R", "0").await, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn competing_leaves_leave_the_base_caption_and_are_reported_once_each() {
    let (_dir, products) = products();
    let state = Arc::new(state(products, &[]));
    let app = knx_server::app(state, None);
    let dto = panel(&app, None).await;
    assert_eq!(
        group_of(&dto, "R-C"),
        ("PB-3".into(), json!("Base competing"))
    );
    assert_eq!(unrecognized(&dto), 2);
}

#[tokio::test]
async fn source_bytes_and_stored_leaves_survive_every_state() {
    let (_dir, products) = products();
    let sha = knx_productdb::sha256_hex(PROGRAM.as_bytes());
    let leaves = |conn: &knx_productdb::Connection| -> Vec<(String, String, String)> {
        conn.prepare(
            "SELECT element_id, ref_id, text FROM dynamic_node
             WHERE kind = 'ParameterBlockRename' ORDER BY node_id",
        )
        .unwrap()
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap()
    };
    let stored_before = leaves(&products);
    assert_eq!(stored_before.len(), 5);
    let state = Arc::new(state(products, &[]));
    let app = knx_server::app(state.clone(), None);
    for raw in ["1", "2", "0"] {
        assert_eq!(set(&app, "R-S", raw).await, StatusCode::OK);
        panel(&app, Some("de")).await;
    }
    let (original, stored_after) = state
        .product_db
        .as_ref()
        .unwrap()
        .with_read(|db| {
            Ok::<_, std::convert::Infallible>((
                knx_productdb::load_source_file(db, &sha).unwrap(),
                leaves(db),
            ))
        })
        .unwrap()
        .unwrap();
    assert_eq!(original.as_deref(), Some(PROGRAM.as_bytes()));
    assert_eq!(stored_after, stored_before);
}

fn node_of(products: &knx_productdb::Connection, element: &str) -> i64 {
    products
        .query_row(
            "SELECT node_id FROM dynamic_node WHERE element_id = ?1",
            [element],
            |r| r.get(0),
        )
        .unwrap()
}

/// The unsupported-translation diagnostics of a panel, by their `detail`.
fn placeholder_refusals(dto: &Value) -> Vec<String> {
    dto["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|d| d["kind"] == "unresolvedTextPlaceholder")
        .map(|d| d["detail"].as_str().unwrap().to_string())
        .collect()
}

#[tokio::test]
async fn a_missing_an_empty_and_an_unsupported_translation_stay_distinguishable() {
    let (_dir, products) = products();
    products
        .execute_batch(
            "INSERT INTO translation (scope,scope_id,language,ref_id,attribute_name,text) VALUES
             ('Program','A-1','es-ES','PR-1','Text',''),
             ('Program','A-1','nl-NL','PR-1','Text','probe_nl_py {{0}}');",
        )
        .unwrap();
    let leaf = node_of(&products, "PR-1");
    let state = Arc::new(state(products, &[("R-S", "1")]));
    let app = knx_server::app(state.clone(), None);
    let before = state.project.lock().unwrap().clone();
    for (language, label, refusals) in [
        // Exact and prefix selection use the leaf's own entry.
        (Some("de-DE"), json!("probe_de_py"), vec![]),
        (Some("de"), json!("probe_de_py"), vec![]),
        // Only the block is translated, or nothing is: the leaf's source text.
        (Some("fr-FR"), json!("fixture_switch_py page"), vec![]),
        (Some("it"), json!("fixture_switch_py page"), vec![]),
        (None, json!("fixture_switch_py page"), vec![]),
        // An explicitly empty entry is not a missing one: no source fallback, no caption.
        (Some("es-ES"), Value::Null, vec![]),
        // A selected entry with a placeholder: the permitted source fallback, named.
        (
            Some("nl-NL"),
            json!("fixture_switch_py page"),
            vec![format!(
                "UnresolvedTextPlaceholder {{ node_id: {leaf}, name: \"0\" }}"
            )],
        ),
    ] {
        let dto = panel(&app, language).await;
        assert_eq!(
            group_of(&dto, "R-S"),
            ("PB-1".into(), label),
            "{language:?}"
        );
        assert_eq!(placeholder_refusals(&dto), refusals, "{language:?}");
        // The block's other diagnostics are those of every language.
        assert_eq!(unrecognized(&dto), 2, "{language:?}");
    }
    assert_eq!(*state.project.lock().unwrap(), before);
}

/// A panel whose active leaf has a `size`-byte translation in `sv-SE`, and `R-S` stored at
/// `stored`. The leaf text is admitted three times: by the translation query, by the overlay
/// and by the one field's group label. Other copies are a few hundred bytes.
fn large_translation(size: usize, stored: &str) -> (tempfile::TempDir, Arc<knx_server::AppState>) {
    let (dir, products) = products();
    products
        .execute(
            "INSERT INTO translation (scope,scope_id,language,ref_id,attribute_name,text)
             VALUES ('Program','A-1','sv-SE','PR-1','Text',?1)",
            [format!("probe_sv_{}", "y".repeat(size - 9))],
        )
        .unwrap();
    let mut state = state(products, &[("R-S", stored)]);
    state.data_dir = dir.path().to_path_buf();
    (dir, Arc::new(state))
}

#[tokio::test]
async fn a_large_selected_translation_is_charged_for_every_copy_it_takes() {
    let budget = knx_productdb::dynamic::MAX_TEXT_PROJECTION_WORK;
    // Two copies fit, three do not: only an uncharged overlay copy would let this pass.
    let size = budget * 2 / 5;
    assert!(2 * size < budget && 3 * size > budget);

    // Control: a small translation through the same path.
    let (_small_dir, small) = large_translation(1_000, "1");
    let dto = panel(&knx_server::app(small, None), Some("sv-SE")).await;
    assert_eq!(group_of(&dto, "R-S").1.as_str().unwrap().len(), 1_000);

    let (_dir, state) = large_translation(size, "1");
    let app = knx_server::app(state.clone(), None);
    let before = state.project.lock().unwrap().clone();
    let request = Request::builder()
        .uri("/api/device/1/parameters?language=sv-SE")
        .body(Body::empty())
        .unwrap();
    let (status, dto) = call(app.clone(), request).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(dto["error"], "device text projection budget exhausted");
    assert!(dto.get("sections").is_none());
    assert_eq!(*state.project.lock().unwrap(), before);
    // Without the language the same panel is ordinary.
    assert_eq!(
        group_of(&panel(&app, None).await, "R-S").1,
        json!("fixture_switch_py page")
    );

    // The edit that would select the leaf is refused whole: no project, revision or history.
    let (_post_dir, state) = large_translation(size, "0");
    let app = knx_server::app(state.clone(), None);
    let save = Request::post("/api/project/save-as")
        .header("content-type", "application/json")
        .body(Body::from(json!({"path": "rename.knxdb"}).to_string()))
        .unwrap();
    assert_eq!(call(app.clone(), save).await.0, StatusCode::OK);
    let path = state.store_path.lock().unwrap().clone().unwrap();
    let native = std::fs::read(&path).unwrap();
    let before = state.project.lock().unwrap().clone();
    let revision = state
        .project_revision
        .load(std::sync::atomic::Ordering::Relaxed);
    let generation = *state.history_generation.lock().unwrap();
    let history = format!("{:?}", state.command_stack.lock().unwrap());
    let post = Request::post("/api/device/1/parameters?language=sv-SE")
        .header("content-type", "application/json")
        .body(Body::from(json!({"etsId": "R-S", "raw": "1"}).to_string()))
        .unwrap();
    let (status, dto) = call(app.clone(), post).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(dto["error"], "device text projection budget exhausted");
    assert_eq!(*state.project.lock().unwrap(), before);
    assert_eq!(
        state
            .project_revision
            .load(std::sync::atomic::Ordering::Relaxed),
        revision
    );
    assert_eq!(*state.history_generation.lock().unwrap(), generation);
    assert_eq!(
        format!("{:?}", state.command_stack.lock().unwrap()),
        history
    );
    assert_eq!(std::fs::read(&path).unwrap(), native);
    assert_eq!(
        group_of(&panel(&app, None).await, "R-S").1,
        json!("Base page")
    );
}
