//! T26 Task 2: `GET /api/product-languages`, and the `language` query
//! parameter on both `/api/device/{id}/parameters` endpoints. Fixture setup
//! idiom follows `http_parameter_panel.rs` (read there first); the program
//! XML here additionally carries a `<Languages>` block, following
//! `crates/knx-productdb/src/query.rs`'s own `PARAMETER_PROGRAM_TRANSLATED`
//! fixture for that shape.

use std::sync::{Arc, Mutex};

use axum::body::Body;
use axum::http::{Request, StatusCode};
use knx_core::{
    CommissioningState, CompletionStatus, DeviceId, DeviceInstance, Installation, InstallationId,
    Language, ParameterInstance, ParameterInstanceId, Project, SourceRef, Topology,
};
use serde_json::{json, Value};
use tower::ServiceExt;

const HARDWARE: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<KNX xmlns="http://knx.org/xml/project/11"><ManufacturerData><Manufacturer RefId="M-1">
<Hardware><Hardware Id="H-1" Name="X" SerialNumber="S" VersionNumber="1">
<Products><Product Id="M-1_P-1" /></Products>
<Hardware2Programs><Hardware2Program Id="H-1_HP-1" MediumTypes="MT-0">
<ApplicationProgramRef RefId="A-1" /></Hardware2Program></Hardware2Programs>
</Hardware></Hardware></Manufacturer></ManufacturerData></KNX>"#;

/// One top-level Number parameter (`P-1`, default `5`) whose `Parameter/
/// @Text` ("Delay") has a `de-DE` translation ("Verzoegerung") — enough to
/// exercise the `?language=` query parameter end to end without dragging in
/// the module/`choose` machinery `http_parameter_panel.rs` covers already.
///
/// Plus a second, `Restriction`-kind parameter (`P-2`, an enum over
/// `PT-Enum`'s two options `"0"`/`"1"`) whose option labels — not their
/// `Value`s — carry `de-DE` translations, mirroring
/// `crates/knx-productdb/src/query.rs`'s own `PARAMETER_PROGRAM_TRANSLATED`
/// fixture. Fix round 1, Finding 3: `a_value_translation_never_changes_a_stored_value`
/// proves this at the `knx-productdb` layer already; this fixture lets
/// `enum_write_with_a_language_keeps_the_raw_value_but_translates_its_label`
/// prove the same thing over HTTP, end to end.
const TRANSLATED_PROGRAM: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<KNX xmlns="http://knx.org/xml/project/11"><ManufacturerData><Manufacturer RefId="M-1">
<ApplicationPrograms><ApplicationProgram Id="A-1" Name="P" ApplicationVersion="1" MaskVersion="MV-0701" DefaultLanguage="en-US">
<Static>
<ParameterTypes>
  <ParameterType Id="PT-Num" Name="num"><TypeNumber maxInclusive="255" minInclusive="0" SizeInBit="8" Type="unsignedInt" /></ParameterType>
  <ParameterType Id="PT-Enum" Name="enum"><TypeRestriction Base="Value" SizeInBit="8">
    <Enumeration Id="PT-Enum_EN-0" Text="Off" Value="0" DisplayOrder="0" />
    <Enumeration Id="PT-Enum_EN-1" Text="On" Value="1" DisplayOrder="1" />
  </TypeRestriction></ParameterType>
</ParameterTypes>
<Parameters>
  <Parameter Id="P-1" Name="Delay" Text="Delay" ParameterType="PT-Num" Access="ReadWrite" Value="5" />
  <Parameter Id="P-2" Name="Mode" Text="Mode" ParameterType="PT-Enum" Access="ReadWrite" Value="0" />
</Parameters>
<ParameterRefs>
  <ParameterRef Id="P-1_R-1" RefId="P-1" DisplayOrder="10" Tag="1" />
  <ParameterRef Id="P-2_R-1" RefId="P-2" DisplayOrder="20" Tag="2" />
</ParameterRefs>
</Static>
<Dynamic>
  <ParameterRefRef RefId="P-1_R-1" />
  <ParameterRefRef RefId="P-2_R-1" />
</Dynamic>
<Languages>
  <Language Identifier="de-DE">
    <TranslationUnit RefId="A-1">
      <TranslationElement RefId="P-1">
        <Translation AttributeName="Text" Text="Verzoegerung" />
      </TranslationElement>
      <TranslationElement RefId="PT-Enum_EN-0">
        <Translation AttributeName="Text" Text="Aus" />
      </TranslationElement>
      <TranslationElement RefId="PT-Enum_EN-1">
        <Translation AttributeName="Text" Text="An" />
      </TranslationElement>
    </TranslationUnit>
  </Language>
</Languages>
</ApplicationProgram></ApplicationPrograms></Manufacturer></ManufacturerData></KNX>"#;

fn temp_product_db(program_xml: &str) -> (tempfile::TempDir, knx_productdb::Connection) {
    let dir = tempfile::tempdir().unwrap();
    let conn = knx_productdb::open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
    // `ingest_file`, not `parse::program::ingest_program` directly, so the
    // `Dynamic` pass also runs (same reasoning `http_parameter_panel.rs`'s
    // own `temp_product_db` gives).
    knx_productdb::ingest_file(&conn, "M-1/Hardware.xml", HARDWARE.as_bytes()).unwrap();
    knx_productdb::ingest_file(&conn, "M-1/A.xml", program_xml.as_bytes()).unwrap();
    (dir, conn)
}

/// A project with one installation and one device (`DeviceId(1)`) whose
/// `program_ref` is `"H-1_HP-1"` (matching `temp_product_db`'s hardware
/// fixture), pre-seeded with `parameters`.
#[allow(clippy::field_reassign_with_default)]
fn state_with_device(
    products: knx_productdb::Connection,
    parameters: Vec<(&str, &str)>,
) -> knx_server::AppState {
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
        product_ref: "M-1_P-1".into(),
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
        parameters: parameters
            .into_iter()
            .enumerate()
            .map(|(i, (ets_id, raw))| ParameterInstance {
                id: ParameterInstanceId(i as u32 + 1),
                device: DeviceId(1),
                source: SourceRef {
                    path: "device-1".into(),
                    ets_id: ets_id.into(),
                },
                raw: raw.into(),
            })
            .collect(),
    });
    let mut state = knx_server::AppState::default();
    state.product_db = Some(Mutex::new(products));
    *state.project.lock().unwrap() = Some(project);
    state
}

/// `AppState::default()` runs the real `default_path()` lookup — on a
/// machine that already has a product database installed, that would make
/// "no product database configured" tests depend on the environment. Force
/// `None` explicitly, same reasoning `http_device_routes.rs`'s own
/// `state_with_one_installation` carries.
#[allow(clippy::field_reassign_with_default)]
fn state_without_product_db() -> knx_server::AppState {
    let mut state = knx_server::AppState::default();
    state.product_db = None;
    state
}

async fn body_json(response: axum::response::Response) -> Value {
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

async fn get(app: axum::Router, uri: &str) -> (StatusCode, Value) {
    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(uri)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    (status, body_json(response).await)
}

async fn post_panel(app: axum::Router, uri: &str, ets_id: &str, raw: &str) -> (StatusCode, Value) {
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(uri)
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({ "etsId": ets_id, "raw": raw }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    (status, body_json(response).await)
}

fn field<'a>(dto: &'a Value, ets_id: &str) -> Option<&'a Value> {
    dto["sections"].as_array().unwrap().iter().find_map(|s| {
        s["fields"]
            .as_array()
            .unwrap()
            .iter()
            .find(|f| f["etsId"] == ets_id)
    })
}

// If `state.product_db` being `None` ever regressed into an error instead
// of an empty list, the Settings panel that lists these languages would
// fail to render on a fresh install that has never imported a package.
#[tokio::test]
async fn product_languages_with_no_product_database_returns_an_empty_list() {
    let state = Arc::new(state_without_product_db());
    let app = knx_server::app(state, None);

    let (status, body) = get(app, "/api/product-languages").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, json!([]));
}

// Catches a route/handler that forgets to read `translation_languages` at
// all, or one that reports zero rows for a package that plainly has some.
#[tokio::test]
async fn product_languages_lists_the_languages_of_the_installed_package() {
    let (_dir, products) = temp_product_db(TRANSLATED_PROGRAM);
    let state = Arc::new(state_with_device(products, vec![]));
    let app = knx_server::app(Arc::clone(&state), None);

    let (status, body) = get(app, "/api/product-languages").await;
    assert_eq!(status, StatusCode::OK);
    let langs = body.as_array().unwrap();
    assert!(!langs.is_empty(), "the fixture package declares de-DE");
    for entry in langs {
        assert!(entry["language"].is_string());
        assert!(entry["rows"].is_i64());
    }
    assert!(langs.iter().any(|l| l["language"] == "de-DE"));
}

// Catches the single most likely defect named in the brief for the GET
// path: `assemble_parameter_panel` still passing `None` regardless of the
// query parameter.
#[tokio::test]
async fn parameter_panel_returns_translated_text_for_a_requested_language() {
    let (_dir, products) = temp_product_db(TRANSLATED_PROGRAM);
    let state = Arc::new(state_with_device(products, vec![]));
    let app = knx_server::app(Arc::clone(&state), None);

    let (status, dto) = get(app, "/api/device/1/parameters?language=de-DE").await;
    assert_eq!(status, StatusCode::OK);
    let p1 = field(&dto, "P-1_R-1").expect("P-1_R-1 present");
    assert_eq!(
        p1["text"], "Verzoegerung",
        "the requested language's translation must overlay the field's text"
    );
}

// AR10: the panel says which stored language answered (a bare `de` is
// answered by `de-DE`), says `null` where it fell back to the package's own
// text, and names the program's declared language of that text.
#[tokio::test]
async fn parameter_panel_exposes_the_answering_language_and_the_fallback() {
    let (_dir, products) = temp_product_db(TRANSLATED_PROGRAM);
    let state = Arc::new(state_with_device(products, vec![]));
    let app = knx_server::app(Arc::clone(&state), None);

    let (status, dto) = get(app, "/api/device/1/parameters?language=de").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(dto["sourceLanguage"], "en-US");
    let p1 = field(&dto, "P-1_R-1").expect("P-1_R-1 present");
    assert_eq!(p1["text"], "Verzoegerung");
    assert_eq!(p1["textLanguage"], "de-DE");
    assert!(
        p1["nameLanguage"].is_null(),
        "no Name translation is stored"
    );
    let p2 = field(&dto, "P-2_R-1").expect("P-2_R-1 present");
    assert!(
        p2["textLanguage"].is_null(),
        "Mode has no de-DE text: fallback"
    );
    assert_eq!(p2["text"], "Mode");
    for option in p2["enumOptions"].as_array().unwrap() {
        assert_eq!(option["language"], "de-DE");
    }
}

// The companion of the previous test: an absent `language` query parameter
// must still behave exactly as before this task — the package's own text,
// untouched.
#[tokio::test]
async fn parameter_panel_without_a_language_returns_the_untranslated_text() {
    let (_dir, products) = temp_product_db(TRANSLATED_PROGRAM);
    let state = Arc::new(state_with_device(products, vec![]));
    let app = knx_server::app(Arc::clone(&state), None);

    let (status, dto) = get(app, "/api/device/1/parameters").await;
    assert_eq!(status, StatusCode::OK);
    let p1 = field(&dto, "P-1_R-1").expect("P-1_R-1 present");
    assert_eq!(p1["text"], "Delay");
}

// The regression test for this task's named defect: a `POST` that omits
// `?language=` on its own write path (or forgets it on the final
// `parameter_panel_impl` call that produces the client-visible DTO) would
// make every parameter edit silently reset the panel to English.
#[tokio::test]
async fn setting_a_parameter_value_keeps_the_requested_language() {
    let (_dir, products) = temp_product_db(TRANSLATED_PROGRAM);
    let state = Arc::new(state_with_device(products, vec![]));
    let app = knx_server::app(Arc::clone(&state), None);

    let (status, dto) = post_panel(
        app,
        "/api/device/1/parameters?language=de-DE",
        "P-1_R-1",
        "7",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let p1 = field(&dto, "P-1_R-1").expect("P-1_R-1 present");
    assert_eq!(p1["value"], "7", "the write itself still applies");
    assert_eq!(
        p1["text"], "Verzoegerung",
        "the response DTO returned by the write must still be translated"
    );
}

// Fix round 1, Finding 3: an end-to-end version of `knx-productdb`'s
// `a_value_translation_never_changes_a_stored_value` unit test and
// `domain.rs`'s `validate_kind_and_bounds`, which together guarantee a
// `Restriction`-kind write is checked against the package's own
// untranslated option values. Neither test proves it over HTTP: a route
// that validated/stored the *translated* option label instead of the raw
// `Value` — or one that lost the translation on the way back out — would
// pass both of those in isolation and still corrupt this exact write.
#[tokio::test]
async fn enum_write_with_a_language_keeps_the_raw_value_but_translates_its_label() {
    let (_dir, products) = temp_product_db(TRANSLATED_PROGRAM);
    let state = Arc::new(state_with_device(products, vec![]));
    let app = knx_server::app(Arc::clone(&state), None);

    let (status, dto) = post_panel(
        app,
        "/api/device/1/parameters?language=de-DE",
        "P-2_R-1",
        "1",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let p2 = field(&dto, "P-2_R-1").expect("P-2_R-1 present");
    assert_eq!(
        p2["value"], "1",
        "the stored value must be exactly the untranslated raw value that was posted, \
         never the translated label or anything derived from it"
    );
    let options = p2["enumOptions"].as_array().unwrap();
    let on_option = options
        .iter()
        .find(|o| o["value"] == "1")
        .expect("option '1' present");
    assert_eq!(
        on_option["text"], "An",
        "the option label for the posted value must be the de-DE translation"
    );
}
