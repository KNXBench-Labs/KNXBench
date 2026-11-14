//! Fictional evaluated owners reach the real parameter HTTP producer, command path and UI captures.

use super::{call, get_panel, state};
use axum::body::Body;
use axum::http::{Request, StatusCode};
use serde_json::{json, Value};
use std::io::Write;
use std::sync::Arc;
use tower::ServiceExt;

// Independently invented source: a root area, equal-caption but distinct owners,
// a conditional page, a declared grid and a manufacturer-restricted sibling.
const SOURCE: &str = r#"<KNX xmlns="http://knx.org/xml/project/20"><ManufacturerData><Manufacturer RefId="M-0001">
<ApplicationPrograms><ApplicationProgram Id="A-1" Name="Fable navigation" ApplicationVersion="1" MaskVersion="MV-0701">
<Static><ParameterTypes>
<ParameterType Id="NAV-N" Name="Count"><TypeNumber Type="unsignedInt" SizeInBit="8" minInclusive="0" maxInclusive="100" UIHint="Slider" /></ParameterType>
<ParameterType Id="NAV-T" Name="Words"><TypeText SizeInBit="160" /></ParameterType>
</ParameterTypes><Parameters>
<Parameter Id="NAV-P-C" Name="Controller" Text="Extra page" ParameterType="NAV-N" Value="0" />
<Parameter Id="NAV-P-A" Name="Amber" Text="Amber count" ParameterType="NAV-N" Value="10" />
<Parameter Id="NAV-P-X" Name="Extra" Text="Extra count" ParameterType="NAV-N" Value="20" />
<Parameter Id="NAV-P-B" Name="Blue" Text="Blue count" ParameterType="NAV-N" Value="30" />
<Parameter Id="NAV-P-L" Name="Locked" Text="Locked count" ParameterType="NAV-N" Value="40" />
<Parameter Id="NAV-P-T" Name="Title" ParameterType="NAV-T" Value="Fable" />
</Parameters><ParameterRefs>
<ParameterRef Id="NAV-C" RefId="NAV-P-C" /><ParameterRef Id="NAV-A" RefId="NAV-P-A" />
<ParameterRef Id="NAV-X" RefId="NAV-P-X" /><ParameterRef Id="NAV-B" RefId="NAV-P-B" />
<ParameterRef Id="NAV-L" RefId="NAV-P-L" /><ParameterRef Id="NAV-TEXT" RefId="NAV-P-T" />
</ParameterRefs></Static><Dynamic>
<ParameterBlock Id="NAV-ROOT" Text="Overview"><ParameterRefRef RefId="NAV-C" /></ParameterBlock>
<Channel Id="NAV-ARC-A" Text="Arc {{0}}" TextParameterRefId="NAV-TEXT">
<ParameterBlock Id="NAV-PAGE-A" Text="Amber page"><ParameterRefRef RefId="NAV-A" /></ParameterBlock>
<choose ParamRefId="NAV-C"><when test="1"><ParameterBlock Id="NAV-EXTRA" Text="Extra page"><ParameterRefRef RefId="NAV-X" /></ParameterBlock></when><when test="0" /></choose>
<ParameterBlock Id="NAV-NOTE" Text="Explanation"><ParameterSeparator Id="NAV-S" Text="Fictional note" /></ParameterBlock>
</Channel>
<Channel Id="NAV-ARC-B" Text="Arc Fable">
<ParameterBlock Id="NAV-PAGE-B" Text="Blue page" Layout="Table"><Rows><Row Id="NAV-ROW" Text="Blue row" /></Rows><Columns><Column Id="NAV-COL" Text="Blue column" /></Columns><ParameterRefRef RefId="NAV-B" Cell="1,1" /></ParameterBlock>
<ParameterBlock Id="NAV-LOCKED" Text="Locked page" Access="Read"><ParameterRefRef RefId="NAV-L" /></ParameterBlock>
</Channel></Dynamic></ApplicationProgram></ApplicationPrograms></Manufacturer></ManufacturerData></KNX>"#;

fn products_from(source: &str) -> (tempfile::TempDir, knx_productdb::Connection) {
    let dir = tempfile::tempdir().unwrap();
    let products = knx_productdb::open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
    let hardware = super::HARDWARE.replace(
        "http://knx.org/xml/project/11",
        "http://knx.org/xml/project/20",
    );
    super::super::native_source_fixture::install(
        &products,
        20,
        ("M-0001/Hardware.xml", hardware.as_bytes()),
        ("M-0001/Navigation.xml", source.as_bytes()),
    );
    (dir, products)
}

async fn response(app: axum::Router, request: Request<Body>) -> Vec<u8> {
    let response = app.oneshot(request).await.unwrap();
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap()
        .to_vec();
    assert_eq!(
        status,
        StatusCode::OK,
        "{}",
        String::from_utf8_lossy(&bytes)
    );
    bytes
}
fn get(uri: &str) -> Request<Body> {
    Request::get(uri).body(Body::empty()).unwrap()
}
fn parsed(bytes: &[u8]) -> Value {
    serde_json::from_slice(bytes).unwrap()
}
fn save(language: &str, name: &str, bytes: &[u8]) {
    if let Some(root) = std::env::var_os("KNXBENCH_PARAMETER_NAVIGATION_CAPTURE_DIR") {
        let root = std::path::PathBuf::from(root).join(language);
        std::fs::create_dir_all(&root).unwrap();
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(root.join(format!("{name}.json")))
            .unwrap();
        file.write_all(bytes).unwrap();
    }
}
async fn post(
    app: axum::Router,
    language: &str,
    tree: &Value,
    id: &str,
    old: &str,
    raw: &str,
) -> Vec<u8> {
    response(app, Request::post(format!("/api/device/1/parameters?language={language}"))
        .header("content-type", "application/json")
        .body(Body::from(json!({"etsId":id, "raw":raw, "expectedValue":old,
            "serverIncarnation":tree["server_incarnation"], "projectIncarnation":tree["project_incarnation"],
            "snapshotRevision":tree["snapshot_revision"]}).to_string())).unwrap()).await
}
fn field<'a>(panel: &'a Value, id: &str) -> &'a Value {
    panel["sections"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|s| s["fields"].as_array().unwrap())
        .find(|f| f["etsId"] == id)
        .unwrap()
}
fn assert_initial(panel: &Value, language: &str) {
    let fields = panel["sections"][0]["fields"].as_array().unwrap();
    assert_eq!(
        fields
            .iter()
            .map(|f| f["etsId"].as_str().unwrap())
            .collect::<Vec<_>>(),
        ["NAV-C", "NAV-A", "NAV-B", "NAV-L"]
    );
    assert!(field(panel, "NAV-C")["group"]["container"].is_null());
    let a = &field(panel, "NAV-A")["group"]["container"];
    let b = &field(panel, "NAV-B")["group"]["container"];
    assert_eq!(a["elementId"], "NAV-ARC-A");
    assert_eq!(b["elementId"], "NAV-ARC-B");
    assert_ne!(a["nodeId"], b["nodeId"]);
    assert_eq!(a["scope"], Value::Null);
    assert_eq!(
        a["label"],
        if language == "de" {
            "Bogen Fable"
        } else {
            "Arc Fable"
        }
    );
    assert_eq!(a["label"], b["label"]);
    assert_eq!(a["labelFill"]["source"], "ProductDefault");
    assert_eq!(
        a["labelFill"]["template"],
        if language == "de" {
            "Bogen {{0}}"
        } else {
            "Arc {{0}}"
        }
    );
    assert_eq!(
        panel["sections"][0]["separators"][0]["group"]["container"],
        *a
    );
    assert_eq!(field(panel, "NAV-L")["editable"], false);
    assert!(field(panel, "NAV-L")["writeEtsId"].is_null());
    assert_eq!(panel["sections"][0]["groupLayouts"][0]["status"], "grid");
}

#[tokio::test]
async fn evaluated_navigation_command_undo_and_original_browser_bodies() {
    for language in ["en", "de"] {
        let (dir, products) = products_from(SOURCE);
        products.execute_batch("INSERT INTO translation (scope,scope_id,language,ref_id,attribute_name,text) VALUES
            ('Program','A-1','de-DE','NAV-ARC-A','Text','Bogen {{0}}'),
            ('Program','A-1','de-DE','NAV-ARC-B','Text','Bogen Fable');").unwrap();
        let mut owned = state(products);
        owned.data_dir = dir.path().to_path_buf();
        let state = Arc::new(owned);
        let before = state.project.lock().unwrap().clone();
        let app = knx_server::app(state.clone(), None);
        let uri = format!("/api/device/1/parameters?language={language}");
        let initial = response(app.clone(), get(&uri)).await;
        let initial_dto = parsed(&initial);
        assert_initial(&initial_dto, language);
        let project = response(app.clone(), get("/api/project")).await;
        let source = response(
            app.clone(),
            get("/api/device/1/parameter-editor-source?etsId=NAV-A"),
        )
        .await;
        assert_eq!(parsed(&source)["evidence"]["source"]["child"], "TypeNumber");
        assert_eq!(
            *state.project.lock().unwrap(),
            before,
            "navigation reads are presentation-only"
        );
        assert!(!state.command_stack.lock().unwrap().can_undo());
        save(language, "initial", &initial);
        save(language, "project-initial", &project);
        save(language, "source-initial", &source);
        save(
            language,
            "device",
            &response(app.clone(), get("/api/device/1")).await,
        );

        let applied = post(
            app.clone(),
            language,
            &parsed(&project),
            "NAV-A",
            "10",
            "24",
        )
        .await;
        let applied_dto = parsed(&applied);
        assert_eq!(field(&applied_dto, "NAV-A")["value"], "24");
        assert_eq!(field(&applied_dto, "NAV-B"), field(&initial_dto, "NAV-B"));
        save(language, "applied", &applied);
        save(
            language,
            "project-applied",
            &response(app.clone(), get("/api/project")).await,
        );
        let source = response(
            app.clone(),
            get("/api/device/1/parameter-editor-source?etsId=NAV-A"),
        )
        .await;
        save(language, "source-applied", &source);

        let changed = post(
            app.clone(),
            language,
            &applied_dto["tree"],
            "NAV-C",
            "0",
            "1",
        )
        .await;
        let changed_dto = parsed(&changed);
        assert_eq!(
            field(&changed_dto, "NAV-X")["group"]["container"],
            field(&changed_dto, "NAV-A")["group"]["container"]
        );
        assert_eq!(field(&changed_dto, "NAV-A")["value"], "24");
        assert_eq!(field(&changed_dto, "NAV-B"), field(&initial_dto, "NAV-B"));
        save(language, "changed", &changed);
        save(
            language,
            "project-changed",
            &response(app.clone(), get("/api/project")).await,
        );
        let source = response(
            app.clone(),
            get("/api/device/1/parameter-editor-source?etsId=NAV-A"),
        )
        .await;
        save(language, "source-changed", &source);
        let undo_tree = response(
            app.clone(),
            Request::post("/api/undo").body(Body::empty()).unwrap(),
        )
        .await;
        let undo = response(app.clone(), get(&uri)).await;
        let undo_dto = parsed(&undo);
        assert_eq!(
            undo_dto["sections"], applied_dto["sections"],
            "one undo removes only the controlling command"
        );
        assert_eq!(field(&undo_dto, "NAV-A")["value"], "24");
        save(language, "undo-tree", &undo_tree);
        save(language, "undo", &undo);
        let source = response(
            app.clone(),
            get("/api/device/1/parameter-editor-source?etsId=NAV-A"),
        )
        .await;
        save(language, "source-undo", &source);
        // A second undo proves the two accepted values were two separate commands.
        assert_eq!(
            call(
                app.clone(),
                Request::post("/api/undo").body(Body::empty()).unwrap()
            )
            .await
            .0,
            StatusCode::OK
        );
        assert_eq!(
            parsed(&response(app, get(&uri)).await)["sections"],
            initial_dto["sections"]
        );
    }
}

#[tokio::test]
async fn missing_container_ids_and_captions_never_merge_or_invent_a_parent() {
    let source = SOURCE
        .replace(
            "Id=\"NAV-ARC-A\" Text=\"Arc {{0}}\" TextParameterRefId=\"NAV-TEXT\"",
            "",
        )
        .replace("Id=\"NAV-ARC-B\" Text=\"Arc Fable\"", "");
    assert!(source.contains("<Channel >"));
    let (dir, products) = products_from(&source);
    let mut owned = state(products);
    owned.data_dir = dir.path().to_path_buf();
    let dto = get_panel(knx_server::app(Arc::new(owned), None)).await;
    let a = &field(&dto, "NAV-A")["group"]["container"];
    let b = &field(&dto, "NAV-B")["group"]["container"];
    assert_ne!(a["nodeId"], b["nodeId"]);
    assert!(a["elementId"].is_null() && b["elementId"].is_null());
    assert!(a["label"].is_null() && b["label"].is_null());
    assert!(field(&dto, "NAV-C")["group"]["container"].is_null());
}

#[tokio::test]
async fn repeated_container_and_leaf_ids_stay_qualified_by_actual_module_owner_scope() {
    let head = SOURCE.split("<Dynamic>").next().unwrap();
    let source = format!(
        r#"{head}<Dynamic><Module Id="NAV_MD-7_M-3" RefId="NAV_MD-7" /><Module Id="NAV_MD-7_M-4" RefId="NAV_MD-7" /></Dynamic>
<ModuleDefs><ModuleDef Id="NAV_MD-7" Name="Fictional chamber"><Static>
<ParameterTypes><ParameterType Id="NAV_MD-7_PT" Name="Module count"><TypeNumber Type="unsignedInt" SizeInBit="8" /></ParameterType></ParameterTypes>
<Parameters><Parameter Id="NAV_MD-7_P-1" Name="Module count" ParameterType="NAV_MD-7_PT" Value="8" /></Parameters>
<ParameterRefs><ParameterRef Id="NAV_MD-7_P-1_R-1" RefId="NAV_MD-7_P-1" /></ParameterRefs></Static>
<Dynamic><Channel Id="NAV-REUSED" Text="Same caption"><ParameterBlock Id="NAV-REUSED-PAGE" Text="Same page"><ParameterRefRef RefId="NAV_MD-7_P-1_R-1" /></ParameterBlock></Channel></Dynamic>
</ModuleDef></ModuleDefs></ApplicationProgram></ApplicationPrograms></Manufacturer></ManufacturerData></KNX>"#
    );
    let (dir, products) = products_from(&source);
    let mut owned = state(products);
    owned.data_dir = dir.path().to_path_buf();
    let dto = get_panel(knx_server::app(Arc::new(owned), None)).await;
    let sections = dto["sections"].as_array().unwrap();
    assert_eq!(sections.len(), 2);
    let a = &sections[0]["fields"][0]["group"]["container"];
    let b = &sections[1]["fields"][0]["group"]["container"];
    assert_eq!(a["elementId"], "NAV-REUSED");
    assert_eq!(a["elementId"], b["elementId"]);
    assert_eq!(a["nodeId"], b["nodeId"]);
    assert_ne!(a["scope"]["nodeChain"], b["scope"]["nodeChain"]);
    assert_eq!(a["scope"], sections[0]["scope"]);
    assert_eq!(b["scope"], sections[1]["scope"]);
    assert_eq!(sections[0]["fields"][0]["editable"], false);
    // The same leaf scope can instead inherit a display owner outside the module.
    // Owner scope must not be reconstructed from the field's scope.
    let inherited = source
        .replace("<Dynamic><Module Id=\"NAV_MD-7_M-3\"", "<Dynamic><Channel Id=\"NAV-OUTSIDE\" Text=\"Outer display\"><Module Id=\"NAV_MD-7_M-3\"")
        .replace("RefId=\"NAV_MD-7\" /></Dynamic>", "RefId=\"NAV_MD-7\" /></Channel></Dynamic>")
        .replace("<Channel Id=\"NAV-REUSED\" Text=\"Same caption\">", "")
        .replace("</ParameterBlock></Channel></Dynamic>", "</ParameterBlock></Dynamic>");
    assert!(inherited.contains("<Channel Id=\"NAV-OUTSIDE\""));
    let (dir, products) = products_from(&inherited);
    let mut owned = state(products);
    owned.data_dir = dir.path().to_path_buf();
    let dto = get_panel(knx_server::app(Arc::new(owned), None)).await;
    for section in dto["sections"].as_array().unwrap() {
        assert!(section["scope"].is_object());
        let owner = &section["fields"][0]["group"]["container"];
        assert_eq!(owner["elementId"], "NAV-OUTSIDE");
        assert!(owner["scope"].is_null());
        assert_eq!(owner["label"], "Outer display");
    }
}

#[tokio::test]
async fn container_caption_copies_share_the_existing_projection_budget() {
    let text = "z".repeat(700_000);
    let source = SOURCE.replace("Arc {{0}}", &text);
    let (dir, products) = products_from(&source);
    let mut owned = state(products);
    owned.data_dir = dir.path().to_path_buf();
    let state = Arc::new(owned);
    let before = state.project.lock().unwrap().clone();
    // One field and one separator group copy are admitted; the dynamic label is bounded too.
    let (status, dto) = call(
        knx_server::app(state.clone(), None),
        get("/api/device/1/parameters"),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{dto}");
    assert_eq!(
        field(&dto, "NAV-A")["group"]["container"]["label"]
            .as_str()
            .unwrap()
            .len(),
        text.len()
    );
    assert_eq!(*state.project.lock().unwrap(), before);
    let source=source.replace("<choose ParamRefId=\"NAV-C\">", "<ParameterBlock Id=\"NAV-COPY-1\"><ParameterSeparator Text=\"note\" /></ParameterBlock><ParameterBlock Id=\"NAV-COPY-2\"><ParameterSeparator Text=\"note\" /></ParameterBlock><ParameterBlock Id=\"NAV-COPY-3\"><ParameterSeparator Text=\"note\" /></ParameterBlock><ParameterBlock Id=\"NAV-COPY-4\"><ParameterSeparator Text=\"note\" /></ParameterBlock><choose ParamRefId=\"NAV-C\">");
    assert!(source.contains("Id=\"NAV-COPY-4\""));
    let (dir, products) = products_from(&source);
    let mut owned = super::state(products);
    owned.data_dir = dir.path().to_path_buf();
    let state = Arc::new(owned);
    let before = state.project.lock().unwrap().clone();
    let (status, dto) = call(
        knx_server::app(state.clone(), None),
        get("/api/device/1/parameters"),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(dto["error"], "device text projection budget exhausted");
    assert_eq!(*state.project.lock().unwrap(), before);
    assert!(!state.command_stack.lock().unwrap().can_undo());
}
