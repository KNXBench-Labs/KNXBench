//! Projects fictional literal axis translations through real HTTP, writes and native saves.
use super::*;
use std::sync::atomic::Ordering;

const AXES: &str = r#"<ParameterBlock Id="AX-B" Text="Literal axes" Layout="Table">
<Rows><Row Id="AX-R8" Text="Upper bank" CollapseIfEmpty="true"/><Row Id="AX-R2" Text="Lower bank"/></Rows>
<Columns><Column Id="AX-C9" Text="Select" Width="42%"/><Column Id="AX-C1" Text="Source fallback" Width="58%"/></Columns>
<ParameterRefRef RefId="R-N" Cell="1,1"/>
<choose ParamRefId="R-N"><when test="1"><ParameterRefRef RefId="R-T" Cell="2,2"/></when></choose>
</ParameterBlock>"#;

fn axis_source() -> String {
    let head = PROGRAM.split("<Dynamic>").next().unwrap();
    format!(
        r#"{head}<Dynamic>{AXES}</Dynamic></ApplicationProgram></ApplicationPrograms>
<Languages><Language Identifier="de-DE"><TranslationUnit RefId="A-1">
<TranslationElement RefId="AX-R8"><Translation AttributeName="Text" Text="Obere Bank"/></TranslationElement>
<TranslationElement RefId="AX-C9"><Translation AttributeName="Text" Text="Auswahl"/></TranslationElement>
</TranslationUnit></Language></Languages></Manufacturer></ManufacturerData></KNX>"#
    )
}

fn assert_axes(dto: &Value, translated: bool) {
    let grid = &dto["sections"][0]["groupLayouts"][0];
    assert_eq!(grid["groupKey"], "AX-B");
    assert_eq!(
        grid["rows"],
        json!([
            {"id":"AX-R8","text":if translated {"Obere Bank"} else {"Upper bank"},"textParameterRefId":null,"collapseIfEmpty":"true"},
            {"id":"AX-R2","text":"Lower bank","textParameterRefId":null,"collapseIfEmpty":null}
        ])
    );
    assert_eq!(
        grid["columns"],
        json!([
            {"id":"AX-C9","text":if translated {"Auswahl"} else {"Select"},"textParameterRefId":null,"width":"42%"},
            {"id":"AX-C1","text":"Source fallback","textParameterRefId":null,"width":"58%"}
        ])
    );
}

async fn language_panel(app: axum::Router, language: Option<&str>) -> Value {
    let uri = language.map_or("/api/device/1/parameters".to_owned(), |l| {
        format!("/api/device/1/parameters?language={l}")
    });
    let (status, dto) = call(app, Request::get(uri).body(Body::empty()).unwrap()).await;
    assert_eq!(status, StatusCode::OK, "{dto}");
    dto
}

// Each legitimate mutation starts a new byte baseline; caption reads must change nothing.
async fn native_read_phase(
    state: &Arc<knx_server::AppState>,
    app: &axum::Router,
    path: &std::path::Path,
    phase: &str,
    expected_history: (usize, usize),
    expected_generation: i64,
) -> Value {
    let bytes = std::fs::read(path).unwrap();
    let project = state.project.lock().unwrap().clone();
    let clean = state.clean_project.lock().unwrap().clone();
    let history = format!("{:?}", state.command_stack.lock().unwrap());
    let revision = state.project_revision.load(Ordering::Relaxed);
    let generation = *state.history_generation.lock().unwrap();
    assert_eq!(generation, expected_generation, "{phase}");
    assert_eq!(
        state.command_stack.lock().unwrap().history_lengths(),
        expected_history,
        "{phase}"
    );
    let unchanged = || {
        assert_eq!(*state.project.lock().unwrap(), project, "{phase}");
        assert_eq!(*state.clean_project.lock().unwrap(), clean, "{phase}");
        assert_eq!(
            format!("{:?}", state.command_stack.lock().unwrap()),
            history,
            "{phase}"
        );
        assert_eq!(state.project_revision.load(Ordering::Relaxed), revision);
        assert_eq!(*state.history_generation.lock().unwrap(), generation);
        assert_eq!(std::fs::read(path).unwrap(), bytes, "{phase}");
    };
    let original = language_panel(app.clone(), None).await;
    assert_axes(&original, false);
    unchanged();
    for language in ["de-DE", "de", "fr"] {
        let dto = language_panel(app.clone(), Some(language)).await;
        assert_axes(&dto, language != "fr");
        assert_eq!(
            dto["sections"][0]["fields"],
            original["sections"][0]["fields"]
        );
        assert_eq!(dto["diagnostics"], original["diagnostics"]);
        unchanged();
    }
    let (status, view) = call(
        app.clone(),
        Request::get("/api/project/history")
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{phase}: {view}");
    assert_eq!(view["persistence"], "native");
    assert_eq!(view["undoSteps"], expected_history.0);
    assert_eq!(view["redoSteps"], expected_history.1);
    assert_eq!(view["generation"], expected_generation);
    unchanged();
    println!("WP05_PHASE|http|lifecycle|{phase}|complete");
    original
}

#[tokio::test]
async fn literal_axis_http_translation_preserves_values_activation_identity_and_native_history() {
    let (dir, products) = products_from(&axis_source());
    let mut s = state(products);
    s.data_dir = dir.path().to_path_buf();
    {
        let mut held = s.project.lock().unwrap();
        let project = held.as_mut().unwrap();
        // Edit an existing fictional value: first-override allocation intentionally
        // retains its high-water mark on Undo (ADR-0039), unlike this value-only edit.
        let id = project.ids.next_parameter_instance_id().unwrap();
        project.installations[0]
            .parameters
            .push(knx_core::ParameterInstance {
                id,
                device: DeviceId(1),
                source: SourceRef {
                    path: "device-1".into(),
                    ets_id: "R-N".into(),
                },
                raw: "1".into(),
            });
    }
    let state = Arc::new(s);
    let app = knx_server::app(state.clone(), None);
    let save = Request::post("/api/project/save-as")
        .header("content-type", "application/json")
        .body(Body::from(json!({"path":"literal-axes.knxdb"}).to_string()))
        .unwrap();
    assert_eq!(call(app.clone(), save).await.0, StatusCode::OK);
    let path = state.store_path.lock().unwrap().clone().unwrap();
    super::super::native_source_fixture::assert_carried(&state, &path);
    let before = state.project.lock().unwrap().clone();
    let opaque = state.opaque.lock().unwrap().clone();
    let sources = state.manufacturer_refs.lock().unwrap().clone();
    let revision = state.project_revision.load(Ordering::Relaxed);
    let generation = *state.history_generation.lock().unwrap();
    let original = native_read_phase(&state, &app, &path, "saved", (0, 0), generation).await;

    let post = Request::post("/api/device/1/parameters?language=de-DE")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({"etsId":"R-N","raw":"2",
            "serverIncarnation":state.server_incarnation,
            "projectIncarnation":state.project_incarnation.load(Ordering::Relaxed),
            "snapshotRevision":revision,"expectedValue":"1"})
            .to_string(),
        ))
        .unwrap();
    let (status, changed) = call(app.clone(), post).await;
    assert_eq!(status, StatusCode::OK, "{changed}");
    assert_axes(&changed, true);
    assert_eq!(
        changed["sections"][0]["fields"].as_array().unwrap().len(),
        1
    );
    assert_eq!(changed["sections"][0]["fields"][0]["value"], "2");
    assert_eq!(
        changed["sections"][0]["fields"][0]["cell"],
        json!({"row":1,"column":1})
    );
    let edited = state.project.lock().unwrap().clone();
    assert_ne!(edited, before);
    let edited_panel =
        native_read_phase(&state, &app, &path, "written", (1, 0), generation + 1).await;
    assert_eq!(
        edited_panel["sections"][0]["fields"],
        changed["sections"][0]["fields"]
    );
    assert_eq!(
        call(
            app.clone(),
            Request::post("/api/undo").body(Body::empty()).unwrap()
        )
        .await
        .0,
        StatusCode::OK
    );
    // Undo restores the complete domain model, not the pre-edit history file:
    // ADR-0100 keeps the edited state as a persisted redo frame.
    assert_eq!(*state.project.lock().unwrap(), before);
    assert_eq!(*state.opaque.lock().unwrap(), opaque);
    assert_eq!(*state.manufacturer_refs.lock().unwrap(), sources);
    let restored = native_read_phase(&state, &app, &path, "undone", (0, 1), generation + 2).await;
    assert_eq!(
        restored["sections"][0]["fields"],
        original["sections"][0]["fields"]
    );

    // A fresh server must recover the redo frame from the real native open route.
    // Reuse only the fictional catalogue, never the old in-memory project or stack.
    let mut fresh = knx_server::AppState::new(dir.path().to_path_buf());
    fresh.product_db = Some(
        knx_server::CatalogueOwner::adopt_opened(
            knx_productdb::open_and_migrate(&dir.path().join("products.sqlite")).unwrap(),
            dir.path().join("products.sqlite"),
        )
        .unwrap(),
    );
    let reopened = Arc::new(fresh);
    assert!(reopened.project.lock().unwrap().is_none());
    assert_eq!(
        reopened.command_stack.lock().unwrap().history_lengths(),
        (0, 0)
    );
    let app2 = knx_server::app(reopened.clone(), None);
    let bytes = std::fs::read(&path).unwrap();
    let open = || {
        Request::post("/api/project/open")
            .header("content-type", "application/json")
            .body(Body::from(json!({"path":"literal-axes.knxdb"}).to_string()))
            .unwrap()
    };
    let (status, loaded) = call(app2.clone(), open()).await;
    assert_eq!(status, StatusCode::OK, "{loaded}");
    assert_eq!(std::fs::read(&path).unwrap(), bytes);
    assert_eq!(*reopened.project.lock().unwrap(), before);
    assert_eq!(*reopened.opaque.lock().unwrap(), opaque);
    assert_eq!(*reopened.manufacturer_refs.lock().unwrap(), sources);
    super::super::native_source_fixture::assert_carried(&reopened, &path);
    let panel =
        native_read_phase(&reopened, &app2, &path, "reopened", (0, 1), generation + 2).await;
    assert_eq!(
        panel["sections"][0]["fields"],
        original["sections"][0]["fields"]
    );

    for (operation, expected, fields, lengths, next_generation, phase) in [
        (
            "redo",
            &edited,
            &edited_panel,
            (1, 0),
            generation + 3,
            "reopened-redo",
        ),
        (
            "undo",
            &before,
            &original,
            (0, 1),
            generation + 4,
            "reopened-undo",
        ),
    ] {
        let (status, body) = call(
            app2.clone(),
            Request::post(format!("/api/{operation}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{operation}: {body}");
        assert_eq!(*reopened.project.lock().unwrap(), *expected);
        assert_eq!(*reopened.opaque.lock().unwrap(), opaque);
        assert_eq!(*reopened.manufacturer_refs.lock().unwrap(), sources);
        let panel =
            native_read_phase(&reopened, &app2, &path, phase, lengths, next_generation).await;
        assert_eq!(
            panel["sections"][0]["fields"],
            fields["sections"][0]["fields"]
        );
    }
    // Reopen again after the recovered stack itself has been used and persisted.
    let bytes = std::fs::read(&path).unwrap();
    let (status, loaded) = call(app2.clone(), open()).await;
    assert_eq!(status, StatusCode::OK, "{loaded}");
    assert_eq!(std::fs::read(&path).unwrap(), bytes);
    assert_eq!(*reopened.project.lock().unwrap(), before);
    assert_eq!(*reopened.opaque.lock().unwrap(), opaque);
    assert_eq!(*reopened.manufacturer_refs.lock().unwrap(), sources);
    let panel = native_read_phase(
        &reopened,
        &app2,
        &path,
        "reopened-final",
        (0, 1),
        generation + 4,
    )
    .await;
    assert_eq!(
        panel["sections"][0]["fields"],
        original["sections"][0]["fields"]
    );
    super::super::native_source_fixture::assert_carried(&reopened, &path);
    println!("WP05_CELL|http|lifecycle|complete");
}

#[tokio::test]
async fn literal_axis_http_empty_missing_and_translated_placeholder_have_distinct_fallbacks() {
    for (selected, expected, refused) in [
        ("", "", false),
        ("{{0}}", "Upper bank", true),
        ("{{Argument}}", "Upper bank", true),
    ] {
        let source = axis_source().replace("Text=\"Obere Bank\"", &format!("Text=\"{selected}\""));
        assert!(source.contains(&format!("Text=\"{selected}\"")));
        let (_dir, products) = products_from(&source);
        let dto = language_panel(
            knx_server::app(Arc::new(state(products)), None),
            Some("de-DE"),
        )
        .await;
        let grid = &dto["sections"][0]["groupLayouts"][0];
        assert_eq!(grid["rows"][0]["text"], expected);
        assert_eq!(grid["rows"][1]["text"], "Lower bank");
        assert_eq!(grid["columns"][0]["text"], "Auswahl");
        assert_eq!(
            grid["issues"],
            if refused {
                json!([{"kind":"unshownContent","element":"Row","reference":"AX-R8","cell":null}])
            } else {
                json!([])
            }
        );
        println!("WP05_CELL|http|empty-placeholder|{selected}");
    }
}

#[tokio::test]
async fn literal_axis_http_text_reference_keeps_its_own_source_caption() {
    let source = axis_source().replace(
        "Text=\"Upper bank\"",
        "Text=\"Upper bank\" TextParameterRefId=\"R-T\"",
    );
    assert!(source.contains("Text=\"Upper bank\" TextParameterRefId=\"R-T\""));
    let (_dir, products) = products_from(&source);
    let dto = language_panel(
        knx_server::app(Arc::new(state(products)), None),
        Some("de-DE"),
    )
    .await;
    assert_eq!(
        dto["sections"][0]["groupLayouts"][0]["rows"][0]["text"],
        "Upper bank"
    );
    assert_eq!(
        dto["sections"][0]["groupLayouts"][0]["rows"][0]["textParameterRefId"],
        "R-T"
    );
    assert_eq!(
        dto["sections"][0]["groupLayouts"][0]["columns"][0]["text"],
        "Auswahl"
    );
    println!("WP05_CELL|http|text-reference|complete");
}

fn unrelated_axis_translations(
    products: &knx_productdb::Connection,
    rows: usize,
    payload_bytes: usize,
) {
    let tx = products.unchecked_transaction().unwrap();
    let text = "z".repeat(payload_bytes);
    for i in 0..rows {
        for language in ["de-AT", "de-DE"] {
            tx.execute(
                "INSERT INTO translation(scope,scope_id,language,ref_id,attribute_name,text)
                 VALUES('Program','A-1',?1,?2,'Text',?3)",
                rusqlite::params![language, format!("Unrelated-{i:05}"), text],
            )
            .unwrap();
        }
    }
    tx.commit().unwrap();
}

#[tokio::test]
async fn literal_axis_http_unrelated_translation_growth_keeps_the_complete_panel_unchanged() {
    for rows in [4, 2048] {
        for payload_bytes in [8, 4096] {
            let (_dir, products) = products_from(&axis_source());
            unrelated_axis_translations(&products, rows, payload_bytes);
            let changes = products.total_changes();
            let state = Arc::new(state(products));
            let before = state.project.lock().unwrap().clone();
            let history = format!("{:?}", state.command_stack.lock().unwrap());
            let revision = state.project_revision.load(Ordering::Relaxed);
            let generation = *state.history_generation.lock().unwrap();
            let app = knx_server::app(state.clone(), None);
            let original = language_panel(app.clone(), None).await;
            assert_axes(&original, false);
            for language in ["de-DE", "de", "fr"] {
                let dto = language_panel(app.clone(), Some(language)).await;
                assert_axes(&dto, language != "fr");
                assert_eq!(
                    dto["sections"][0]["fields"],
                    original["sections"][0]["fields"]
                );
                assert_eq!(dto["diagnostics"], original["diagnostics"]);
            }
            assert_eq!(*state.project.lock().unwrap(), before);
            assert_eq!(
                format!("{:?}", state.command_stack.lock().unwrap()),
                history
            );
            assert_eq!(state.project_revision.load(Ordering::Relaxed), revision);
            assert_eq!(*state.history_generation.lock().unwrap(), generation);
            let after = state
                .product_db
                .as_ref()
                .unwrap()
                .with_read(|conn| Ok::<_, rusqlite::Error>(conn.total_changes()))
                .unwrap()
                .unwrap();
            assert_eq!(after, changes);
            println!("WP05_CELL|http|unrelated|{rows}|{payload_bytes}|all-three-languages");
        }
    }
}

#[tokio::test]
async fn foreign_namespace_owner_ancestors_never_admit_axis_translations() {
    let canonical = axis_source();
    let namespace = "http://knx.org/xml/project/11";
    // The foreign container leaves legacy grid rendering intact, so assertions
    // below must reach real DTO axes, not pass by hiding the complete grid.
    for (uri, translated) in [
        (namespace, true),
        ("urn:fictional:owner", false),
        ("http://knx.org/xml/project/11-lookalike", false),
    ] {
        let source = canonical
            .replace(
                "<ApplicationPrograms>",
                &format!("<x:ApplicationPrograms xmlns:x=\"{uri}\">"),
            )
            .replace("</ApplicationPrograms>", "</x:ApplicationPrograms>");
        assert!(source.contains(&format!("<x:ApplicationPrograms xmlns:x=\"{uri}\">")));
        let (_dir, products) = products_from(&source);
        let state = Arc::new(state(products));
        let before = state.project.lock().unwrap().clone();
        let history = format!("{:?}", state.command_stack.lock().unwrap());
        let revision = state.project_revision.load(Ordering::Relaxed);
        let generation = *state.history_generation.lock().unwrap();
        let app = knx_server::app(state.clone(), None);
        let original = language_panel(app.clone(), None).await;
        assert_axes(&original, false);
        let dto = language_panel(app, Some("de-DE")).await;
        assert_axes(&dto, translated);
        assert_eq!(
            dto["sections"][0]["fields"],
            original["sections"][0]["fields"]
        );
        assert_eq!(dto["diagnostics"], original["diagnostics"]);
        assert_eq!(*state.project.lock().unwrap(), before);
        assert_eq!(
            format!("{:?}", state.command_stack.lock().unwrap()),
            history
        );
        assert_eq!(state.project_revision.load(Ordering::Relaxed), revision);
        assert_eq!(*state.history_generation.lock().unwrap(), generation);
        println!("WP05_CELL|http|ancestor|{uri}");
    }
}

#[tokio::test]
async fn literal_axis_http_translation_and_dto_copies_share_one_utf8_budget() {
    for (id, axis) in [("AX-R8", "rows"), ("AX-C9", "columns")] {
        // Query selection alone fits. The DTO's second copy crosses the unchanged ceiling.
        let text = "é".repeat(knx_productdb::dynamic::MAX_TEXT_PROJECTION_WORK / 4 + 1);
        let (dir, products) = products_from(&axis_source());
        unrelated_axis_translations(&products, 2048, 4096);
        products
            .execute(
                "UPDATE translation SET text=?1 WHERE ref_id=?2",
                rusqlite::params![text, id],
            )
            .unwrap();
        let changes = products.total_changes();
        let mut s = state(products);
        s.data_dir = dir.path().to_path_buf();
        let state = Arc::new(s);
        let before = state.project.lock().unwrap().clone();
        let revision = state.project_revision.load(Ordering::Relaxed);
        let history = format!("{:?}", state.command_stack.lock().unwrap());
        let generation = *state.history_generation.lock().unwrap();
        let app = knx_server::app(state.clone(), None);
        assert_axes(&language_panel(app.clone(), None).await, false);
        let (status, dto) = call(
            app,
            Request::get("/api/device/1/parameters?language=de-DE")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{axis}: {dto}");
        assert!(dto["error"]
            .as_str()
            .unwrap()
            .contains("device text projection budget exhausted"));
        assert!(dto.get("sections").is_none());
        assert_eq!(*state.project.lock().unwrap(), before);
        assert_eq!(state.project_revision.load(Ordering::Relaxed), revision);
        assert_eq!(
            format!("{:?}", state.command_stack.lock().unwrap()),
            history
        );
        assert_eq!(*state.history_generation.lock().unwrap(), generation);
        let after = state
            .product_db
            .as_ref()
            .unwrap()
            .with_read(|conn| Ok::<_, rusqlite::Error>(conn.total_changes()))
            .unwrap()
            .unwrap();
        assert_eq!(after, changes);
        println!("WP05_CELL|http|utf8-budget|{id}|{axis}");
    }
}
