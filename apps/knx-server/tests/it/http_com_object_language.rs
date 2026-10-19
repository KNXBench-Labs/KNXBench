//! T33 Task 2: `GET /api/device/{id}?language=` overlays a communication
//! object's `name`/`description` with a product-database translation.
//! Fixture setup idiom follows `http_product_language.rs` (read there
//! first): a synthetic `Hardware.xml` + program XML, ingested through
//! `knx_productdb::ingest_file`. The program XML additionally carries a
//! `ComObjectTable`/`ComObjectRefs` pair and a `<Languages>` block whose
//! `TranslationElement/@RefId` names the `ComObject`'s own `Id` — measured
//! on the real corpus, that is where communication-object translations
//! point, never at the `ComObjectRef`.

use std::sync::{Arc, Mutex};

use axum::body::Body;
use axum::http::{Request, StatusCode};
use knx_core::{
    ComObjectInstance, ComObjectInstanceId, CommissioningState, CompletionStatus, DeviceId,
    DeviceInstance, Installation, InstallationId, Language, Layer, Override, Project, Resolved,
    ResolvedFlags, SourceRef, Text, Topology,
};
use serde_json::Value;
use tower::ServiceExt;

const HARDWARE: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<KNX xmlns="http://knx.org/xml/project/11"><ManufacturerData><Manufacturer RefId="M-1">
<Hardware><Hardware Id="H-1" Name="X" SerialNumber="S" VersionNumber="1">
<Products><Product Id="M-1_P-1" /></Products>
<Hardware2Programs><Hardware2Program Id="H-1_HP-1" MediumTypes="MT-0">
<ApplicationProgramRef RefId="A-1" /></Hardware2Program></Hardware2Programs>
</Hardware></Hardware></Manufacturer></ManufacturerData></KNX>"#;

/// One communication object (`A-1_O-1`, `Program`-layer `Text`/
/// `VisibleDescription`) referenced by one `ComObjectRef` (`A-1_O-1_R-1`,
/// no override of its own). `de-DE` translates the `ComObject`'s `Text`
/// and `VisibleDescription` — the `TranslationElement/@RefId` is `A-1_O-1`,
/// the `ComObject`'s own id, not the `ComObjectRef`'s (see this file's own
/// header comment).
const TRANSLATED_PROGRAM: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<KNX xmlns="http://knx.org/xml/project/11"><ManufacturerData><Manufacturer RefId="M-1">
<ApplicationPrograms><ApplicationProgram Id="A-1" Name="P" ApplicationVersion="1" MaskVersion="MV-0701">
<Static>
<ComObjectTable>
  <ComObject Id="A-1_O-1" Number="1" Text="Switch" VisibleDescription="Switch description"
             ObjectSize="1 Bit" DatapointType="DPST-1-1" />
</ComObjectTable>
<ComObjectRefs>
  <ComObjectRef Id="A-1_O-1_R-1" RefId="A-1_O-1" />
</ComObjectRefs>
</Static>
<Languages>
  <Language Identifier="de-DE">
    <TranslationUnit RefId="A-1">
      <TranslationElement RefId="A-1_O-1">
        <Translation AttributeName="Text" Text="Schalten" />
        <Translation AttributeName="VisibleDescription" Text="Schalterbeschreibung" />
      </TranslationElement>
    </TranslationUnit>
  </Language>
</Languages>
</ApplicationProgram></ApplicationPrograms></Manufacturer></ManufacturerData></KNX>"#;

/// Same `ComObject`/`ComObjectRef` pair as `TRANSLATED_PROGRAM`, but `de-DE`
/// translates only `VisibleDescription` for `A-1_O-1`, never `Text` — a
/// package with partial translation coverage, which shipped packages do
/// have (not every attribute of every object is translated into every
/// language). Exists for Finding M6's regression test: `de-DE` is a real,
/// packaged language, yet this specific com object's `Text` has no
/// `translation` row for it.
const PROGRAM_WITHOUT_A_TEXT_TRANSLATION: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<KNX xmlns="http://knx.org/xml/project/11"><ManufacturerData><Manufacturer RefId="M-1">
<ApplicationPrograms><ApplicationProgram Id="A-1" Name="P" ApplicationVersion="1" MaskVersion="MV-0701">
<Static>
<ComObjectTable>
  <ComObject Id="A-1_O-1" Number="1" Text="Switch" VisibleDescription="Switch description"
             ObjectSize="1 Bit" DatapointType="DPST-1-1" />
</ComObjectTable>
<ComObjectRefs>
  <ComObjectRef Id="A-1_O-1_R-1" RefId="A-1_O-1" />
</ComObjectRefs>
</Static>
<Languages>
  <Language Identifier="de-DE">
    <TranslationUnit RefId="A-1">
      <TranslationElement RefId="A-1_O-1">
        <Translation AttributeName="VisibleDescription" Text="Schalterbeschreibung" />
      </TranslationElement>
    </TranslationUnit>
  </Language>
</Languages>
</ApplicationProgram></ApplicationPrograms></Manufacturer></ManufacturerData></KNX>"#;

fn source(ets_id: &str) -> SourceRef {
    SourceRef {
        path: "device-1".into(),
        ets_id: ets_id.into(),
    }
}

fn temp_product_db(program_xml: &str) -> (tempfile::TempDir, knx_productdb::Connection) {
    let dir = tempfile::tempdir().unwrap();
    let conn = knx_productdb::open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
    knx_productdb::ingest_file(&conn, "M-1/Hardware.xml", HARDWARE.as_bytes()).unwrap();
    knx_productdb::ingest_file(&conn, "M-1/A.xml", program_xml.as_bytes()).unwrap();
    (dir, conn)
}

/// A project with one installation and one device (`DeviceId(1)`) carrying
/// exactly one communication object (`ComObjectInstanceId(1)`, ref id
/// `A-1_O-1_R-1`, matching `temp_product_db`'s fixture), whose `text`/
/// `description` overrides are exactly `text`/`description`. `program_ref`
/// is `"H-1_HP-1"` when `program_installed`, otherwise a ref id no
/// `Hardware2Program` in the fixture ever declares.
#[allow(clippy::field_reassign_with_default)]
fn state_with_com_object(
    products: Option<knx_productdb::Connection>,
    program_installed: bool,
    text: Override<Text>,
    description: Override<Text>,
) -> knx_server::AppState {
    let mut project = Project::new(Language("en".into()));
    project.devices.insert(DeviceInstance {
        id: DeviceId(1),
        source: source("device-1"),
        name: "Device 1".into(),
        description: None,
        address: None,
        product_ref: "M-1_P-1".into(),
        program_ref: if program_installed {
            "H-1_HP-1".into()
        } else {
            "H-9_HP-9".into()
        },
        commissioning: CommissioningState::default(),
        visibility_calculated: true,
        com_objects: vec![ComObjectInstanceId(1)],
        binary_data: vec![],
    });
    project.devices.insert_com_object(ComObjectInstance {
        id: ComObjectInstanceId(1),
        source: source("A-1_O-1_R-1"),
        device: DeviceId(1),
        number: 1,
        text,
        description,
        dpt: Override::Absent,
        flags: ResolvedFlags::none(),
        size: None,
        is_active: true,
        links: vec![],
        module_instance: None,
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
        parameters: vec![],
    });
    let mut state = knx_server::AppState::default();
    state.product_db = products.map(Mutex::new);
    *state.project.lock().unwrap() = Some(project);
    state
}

fn program_layer(value: &str) -> Override<Text> {
    Override::Value(Resolved {
        value: Text::Literal(value.into()),
        layer: Layer::Program,
    })
}

fn instance_layer(value: &str) -> Override<Text> {
    Override::Value(Resolved {
        value: Text::Literal(value.into()),
        layer: Layer::Instance,
    })
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
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body = serde_json::from_slice(&bytes).unwrap();
    (status, body)
}

// Companion of `get` above, for the one case where the response body is
// not JSON at all: a `Query` extraction failure is an axum rejection that
// never reaches the handler (and so never reaches `ApiError`'s JSON
// encoding), so parsing its body as JSON the way `get` does would panic
// on the rejection's plain-text body before the status assertion runs.
async fn get_status(app: axum::Router, uri: &str) -> StatusCode {
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
    response.status()
}

fn com_object(detail: &Value) -> &Value {
    &detail["com_objects"][0]
}

// Test 1 (regression guard for Global Constraint 3): no `language` query
// parameter at all must issue no product-database query and return the
// project's own untranslated text, even though the product database has a
// de-DE translation for this exact communication object.
#[tokio::test]
async fn no_language_query_parameter_returns_the_untranslated_name() {
    let (_dir, products) = temp_product_db(TRANSLATED_PROGRAM);
    let state = Arc::new(state_with_com_object(
        Some(products),
        true,
        program_layer("Switch"),
        program_layer("Switch description"),
    ));
    let app = knx_server::app(Arc::clone(&state), None);

    let (status, detail) = get(app, "/api/device/1").await;
    assert_eq!(status, StatusCode::OK);
    let com = com_object(&detail);
    assert_eq!(com["name"], "Switch");
    assert_eq!(com["description"], "Switch description");
}

// Test 2: a `Program`-layer communication object is overlaid with the
// requested language's translation, for both `name` and `description`.
#[tokio::test]
async fn a_requested_language_translates_a_program_layer_com_object() {
    let (_dir, products) = temp_product_db(TRANSLATED_PROGRAM);
    let state = Arc::new(state_with_com_object(
        Some(products),
        true,
        program_layer("Switch"),
        program_layer("Switch description"),
    ));
    let app = knx_server::app(Arc::clone(&state), None);

    let (status, detail) = get(app, "/api/device/1?language=de-DE").await;
    assert_eq!(status, StatusCode::OK);
    let com = com_object(&detail);
    assert_eq!(com["name"], "Schalten");
    assert_eq!(com["description"], "Schalterbeschreibung");
}

const MASTER: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<KNX xmlns="http://knx.org/xml/project/11"><MasterData><DatapointTypes>
<DatapointType Id="DPT-1" Number="1" Name="1-bit"><DatapointSubtypes>
<DatapointSubtype Id="DPST-1-1" Number="1" Name="switch" Text="Switch" />
</DatapointSubtypes></DatapointType></DatapointTypes></MasterData>
<Languages><Language Identifier="de-DE"><TranslationUnit RefId="DPST-1-1">
<TranslationElement RefId="DPST-1-1"><Translation AttributeName="Text" Text="Schalten" /></TranslationElement>
</TranslationUnit></Language></Languages></KNX>"#;

// AR10: the datapoint-type text names the master-data language that
// answered (`de` → `de-DE`), and carries no marker when it fell back.
#[tokio::test]
async fn the_dpt_text_names_the_language_that_answered() {
    let (_dir, products) = temp_product_db(TRANSLATED_PROGRAM);
    knx_productdb::ingest_master_data(&products, MASTER.as_bytes()).unwrap();
    let state = Arc::new(state_with_com_object(
        Some(products),
        true,
        program_layer("Switch"),
        program_layer("Switch description"),
    ));
    {
        let mut project = state.project.lock().unwrap();
        let com = project
            .as_mut()
            .unwrap()
            .devices
            .com_object_mut(ComObjectInstanceId(1))
            .unwrap();
        com.dpt = Override::Value(Resolved {
            value: knx_core::DptRef {
                main: 1,
                sub: Some(1),
            },
            layer: Layer::Program,
        });
    }
    let app = knx_server::app(Arc::clone(&state), None);

    let (status, detail) = get(app.clone(), "/api/device/1?language=de").await;
    assert_eq!(status, StatusCode::OK);
    let com = com_object(&detail);
    assert_eq!(com["dpt_text"], "Schalten");
    assert_eq!(com["dpt_text_language"], "de-DE");

    let (_, detail) = get(app, "/api/device/1?language=fr-FR").await;
    let com = com_object(&detail);
    assert_eq!(com["dpt_text"], "Switch");
    assert!(
        com["dpt_text_language"].is_null(),
        "fallback carries no marker"
    );
}

// Test 3, the load-bearing one (Global Constraint 2): a communication
// object whose `text` override sits at `Layer::Instance` keeps its
// project-authored text verbatim even when the product database carries a
// translation for that exact ref id — overlaying it would show the user
// their own words back in a different language.
#[tokio::test]
async fn an_instance_layer_text_is_never_translated_because_the_project_owns_it() {
    let (_dir, products) = temp_product_db(TRANSLATED_PROGRAM);
    let state = Arc::new(state_with_com_object(
        Some(products),
        true,
        instance_layer("My Custom Switch Name"),
        instance_layer("My Custom Switch Description"),
    ));
    let app = knx_server::app(Arc::clone(&state), None);

    let (status, detail) = get(app, "/api/device/1?language=de-DE").await;
    assert_eq!(status, StatusCode::OK);
    let com = com_object(&detail);
    assert_eq!(
        com["name"], "My Custom Switch Name",
        "an Instance-layer value is the project's own words and must never be translated"
    );
    assert_eq!(
        com["description"], "My Custom Switch Description",
        "an Instance-layer value is the project's own words and must never be translated"
    );
}

// Test 4: a language the package does not carry returns the untranslated
// text — no cross-language fallback (Global Constraint 1).
#[tokio::test]
async fn an_unpackaged_language_returns_the_untranslated_text() {
    let (_dir, products) = temp_product_db(TRANSLATED_PROGRAM);
    let state = Arc::new(state_with_com_object(
        Some(products),
        true,
        program_layer("Switch"),
        program_layer("Switch description"),
    ));
    let app = knx_server::app(Arc::clone(&state), None);

    let (status, detail) = get(app, "/api/device/1?language=fr-FR").await;
    assert_eq!(status, StatusCode::OK);
    let com = com_object(&detail);
    assert_eq!(com["name"], "Switch");
    assert_eq!(com["description"], "Switch description");
}

// Test 5: a device whose program is not in the product database returns
// 200 with the untranslated detail, not an error — a device whose program
// is not installed is ordinary project state.
#[tokio::test]
async fn a_program_not_in_the_product_database_returns_200_untranslated() {
    let (_dir, products) = temp_product_db(TRANSLATED_PROGRAM);
    let state = Arc::new(state_with_com_object(
        Some(products),
        false,
        program_layer("Switch"),
        program_layer("Switch description"),
    ));
    let app = knx_server::app(Arc::clone(&state), None);

    let (status, detail) = get(app, "/api/device/1?language=de-DE").await;
    assert_eq!(status, StatusCode::OK);
    let com = com_object(&detail);
    assert_eq!(com["name"], "Switch");
    assert_eq!(com["description"], "Switch description");
}

// Test 6, Finding M6: `de-DE` is a genuinely packaged language, but this
// com object's `Text` has no `translation` row for it (only its
// `VisibleDescription` does — see `PROGRAM_WITHOUT_A_TEXT_TRANSLATION`).
// The project's own resolved `name` ("Old Switch Name") deliberately
// differs from the product database's current, untranslated `Text` column
// ("Switch"), so a naive overlay that falls back to that column on a miss
// is caught red-handed: it would show "Switch", not the project's own
// value. `description` has a real translation and must still come through.
#[tokio::test]
async fn a_missing_translation_row_leaves_the_projects_resolved_text_unchanged() {
    let (_dir, products) = temp_product_db(PROGRAM_WITHOUT_A_TEXT_TRANSLATION);
    let state = Arc::new(state_with_com_object(
        Some(products),
        true,
        program_layer("Old Switch Name"),
        program_layer("Switch description"),
    ));
    let app = knx_server::app(Arc::clone(&state), None);

    let (status, detail) = get(app, "/api/device/1?language=de-DE").await;
    assert_eq!(status, StatusCode::OK);
    let com = com_object(&detail);
    assert_eq!(
        com["name"], "Old Switch Name",
        "Finding M6: a miss on the requested language must leave the project's own resolved \
         value untouched, not fall back to the product database's untranslated column"
    );
    assert_eq!(
        com["description"], "Schalterbeschreibung",
        "a real translation for this attribute must still be applied"
    );
}

// Companion of Test 5: no product database open at all, still 200
// untranslated, and no panic taking a lock on a `None`.
#[tokio::test]
async fn no_product_database_open_returns_200_untranslated() {
    let state = Arc::new(state_with_com_object(
        None,
        true,
        program_layer("Switch"),
        program_layer("Switch description"),
    ));
    let app = knx_server::app(Arc::clone(&state), None);

    let (status, detail) = get(app, "/api/device/1?language=de-DE").await;
    assert_eq!(status, StatusCode::OK);
    let com = com_object(&detail);
    assert_eq!(com["name"], "Switch");
    assert_eq!(com["description"], "Switch description");
}

// Finding M5, ruled in T34 Task 4: `Query<ParameterLanguageQuery>` already
// guards `GET /api/parameters/{id}` and `POST /api/parameters/{id}/value`
// (T26), and now guards `GET /api/device/{id}` too (T33) — a malformed
// `language` query 400s here exactly as it does on the other two routes,
// rather than being silently ignored, while an absent one is not malformed
// at all and still returns the untranslated detail with 200 (companion of
// `no_language_query_parameter_returns_the_untranslated_name` above; named
// here explicitly so the M5 ruling has both halves in one place).
// `language=a&language=b` (a repeated key for a scalar field) is the form
// that actually trips `serde_urlencoded`'s deserializer, with "duplicate
// field `language`"; `language[]=de` was also tried and does not —
// `serde_urlencoded` parses `language[]` as an unrecognized key distinct
// from `language` and leaves the field at its `#[serde(default)]` of
// `None`, which reaches the handler as an absent language, not a rejected
// one.
#[tokio::test]
async fn a_malformed_language_query_is_rejected_and_an_absent_one_is_not() {
    let (_dir, products) = temp_product_db(TRANSLATED_PROGRAM);
    let state = Arc::new(state_with_com_object(
        Some(products),
        true,
        program_layer("Switch"),
        program_layer("Switch description"),
    ));
    let app = knx_server::app(Arc::clone(&state), None);

    let status = get_status(app.clone(), "/api/device/1?language=a&language=b").await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    let (status, detail) = get(app, "/api/device/1").await;
    assert_eq!(status, StatusCode::OK);
    let com = com_object(&detail);
    assert_eq!(com["name"], "Switch");
    assert_eq!(com["description"], "Switch description");
}
