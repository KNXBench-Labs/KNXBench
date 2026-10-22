//! T16: `domain::device_detail`'s product/hardware resolution overlay.
//
// Fixture idiom follows `http_product_language.rs`'s own `temp_product_db`
// (read there first) — a `Hardware.xml` giving one `Product`/
// `Hardware2Program` pair, plus an `ApplicationProgram.xml` for the program
// side of the chain.

use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use knx_core::{CommissioningState, DeviceId, DeviceInstance, Language, Project, SourceRef};
use serde_json::Value;
use tower::ServiceExt;

const HARDWARE: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<KNX xmlns="http://knx.org/xml/project/11"><ManufacturerData><Manufacturer RefId="M-1">
<Hardware><Hardware Id="H-1" Name="X" SerialNumber="S" VersionNumber="1">
<Products><Product Id="M-1_P-1" Text="Switch Actuator" OrderNumber="ORD-1" /></Products>
<Hardware2Programs><Hardware2Program Id="H-1_HP-1" MediumTypes="MT-0">
<ApplicationProgramRef RefId="A-1" /></Hardware2Program></Hardware2Programs>
</Hardware></Hardware></Manufacturer></ManufacturerData></KNX>"#;

const PROGRAM: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<KNX xmlns="http://knx.org/xml/project/11"><ManufacturerData><Manufacturer RefId="M-1">
<ApplicationPrograms><ApplicationProgram Id="A-1" Name="P" ApplicationNumber="1" ApplicationVersion="1" MaskVersion="MV-0701" DefaultLanguage="en-US">
<Static><ParameterTypes /><Parameters /><ParameterRefs /></Static>
<Dynamic />
<Languages><Language Identifier="de-DE"><TranslationUnit RefId="A-1">
<TranslationElement RefId="A-1"><Translation AttributeName="Name" Text="Programm P" /></TranslationElement>
</TranslationUnit></Language></Languages>
</ApplicationProgram></ApplicationPrograms></Manufacturer></ManufacturerData></KNX>"#;

fn temp_product_db() -> (tempfile::TempDir, knx_productdb::Connection) {
    let dir = tempfile::tempdir().unwrap();
    let conn = knx_productdb::open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
    knx_productdb::ingest_file(&conn, "M-1/Hardware.xml", HARDWARE.as_bytes()).unwrap();
    knx_productdb::ingest_file(&conn, "M-1/A.xml", PROGRAM.as_bytes()).unwrap();
    (dir, conn)
}

/// One device, `DeviceId(1)`, with the given `product_ref`/`program_ref`.
#[allow(clippy::field_reassign_with_default)]
fn state_with_device(
    products: Option<knx_productdb::Connection>,
    product_ref: &str,
    program_ref: &str,
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
        product_ref: product_ref.into(),
        program_ref: program_ref.into(),
        commissioning: CommissioningState::default(),
        visibility_calculated: true,
        com_objects: vec![],
        binary_data: vec![],
    });
    let mut state = knx_server::AppState::default();
    state.product_db = products
        .map(|c| knx_server::CatalogueOwner::adopt_opened(c, std::path::PathBuf::new()).unwrap());
    *state.project.lock().unwrap() = Some(project);
    state
}

async fn body_json(response: axum::response::Response) -> Value {
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

/// Global Constraint (T16): the pure projection's `resolution` placeholder
/// (`"NoDatabase"`, see `knx_projection::DeviceProductNode`'s doc comment)
/// must never reach the wire once a real database is loaded and the refs
/// resolve — the server overlay is obliged to overwrite it.
#[tokio::test]
async fn device_product_resolution_always_overwrites_the_projections_placeholder() {
    let (_dir, products) = temp_product_db();
    let state = Arc::new(state_with_device(Some(products), "M-1_P-1", "H-1_HP-1"));
    let app = knx_server::app(state, None);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/device/1")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = body_json(response).await;
    let product = &body["product"];
    assert_eq!(product["resolution"], "Resolved");
    assert_ne!(product["resolution"], "NoDatabase");
    assert_eq!(product["catalog"]["product_text"], "Switch Actuator");
    assert_eq!(product["catalog"]["order_number"], "ORD-1");
    assert_eq!(product["catalog"]["application_name"], "P");
    // AR10: no language requested, so no text claims to be a translation.
    assert!(product["catalog"]["application_name_language"].is_null());
    assert_eq!(product["catalog"]["application_source_language"], "en-US");
}

/// AR10: a requested language names the stored language that answered
/// (`de` → `de-DE`); texts nobody translated stay the package's own and
/// carry no language marker.
#[tokio::test]
async fn device_product_names_the_language_that_answered_and_the_fallback() {
    let (_dir, products) = temp_product_db();
    let state = Arc::new(state_with_device(Some(products), "M-1_P-1", "H-1_HP-1"));
    let app = knx_server::app(state, None);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/device/1?language=de")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let catalog = &body_json(response).await["product"]["catalog"];
    assert_eq!(catalog["application_name"], "Programm P");
    assert_eq!(catalog["application_name_language"], "de-DE");
    assert_eq!(catalog["application_source_language"], "en-US");
    assert_eq!(catalog["product_text"], "Switch Actuator");
    assert!(catalog["product_text_language"].is_null(), "fallback");
    assert!(catalog["product_source_language"].is_null(), "undeclared");
}

/// No product database configured at all: refs are present, so the pure
/// builder's `NoDatabase` placeholder is, this once, the honest answer —
/// but it must come from the server actually checking, not merely surviving
/// unchanged. `state_without_product_db` mirrors `http_product_language.rs`'s
/// own reasoning for forcing `None` explicitly rather than relying on
/// `AppState::default()`'s environment-dependent lookup.
#[tokio::test]
async fn device_product_with_no_database_loaded_is_no_database() {
    let state = Arc::new(state_with_device(None, "M-1_P-1", "H-1_HP-1"));
    let app = knx_server::app(state, None);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/device/1")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = body_json(response).await;
    let product = &body["product"];
    assert_eq!(product["resolution"], "NoDatabase");
    assert!(product["catalog"].is_null());
}

/// A database is loaded, but this device's refs are not in it — the
/// manufacturer's catalogue is simply not installed here.
#[tokio::test]
async fn device_product_with_unmatched_refs_is_not_in_database() {
    let (_dir, products) = temp_product_db();
    let state = Arc::new(state_with_device(
        Some(products),
        "M-1_P-does-not-exist",
        "H-1_HP-1",
    ));
    let app = knx_server::app(state, None);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/device/1")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = body_json(response).await;
    let product = &body["product"];
    assert_eq!(product["resolution"], "NotInDatabase");
    assert!(product["catalog"].is_null());
}

/// A device with neither ref stated needs no product-database lookup at
/// all — `NoReference` is final at the pure-projection layer already, and
/// the overlay must leave it alone even with a database loaded.
#[tokio::test]
async fn device_product_with_no_stated_ref_stays_no_reference_even_with_a_database_loaded() {
    let (_dir, products) = temp_product_db();
    let state = Arc::new(state_with_device(Some(products), "", ""));
    let app = knx_server::app(state, None);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/device/1")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = body_json(response).await;
    let product = &body["product"];
    assert_eq!(product["resolution"], "NoReference");
    assert!(product["catalog"].is_null());
    assert!(product["product_ref"].is_null());
    assert!(product["program_ref"].is_null());
}

/// Resolution must not depend on `?language=` — it's a database-membership
/// fact, not a translation. Same fixture as the first test, no query
/// parameter needed since `/api/device/{id}` already omits one, but this
/// pins that omission is exactly what makes the untranslated leg run too.
#[tokio::test]
async fn device_product_resolves_without_a_language_query_parameter() {
    let (_dir, products) = temp_product_db();
    let state = Arc::new(state_with_device(Some(products), "M-1_P-1", "H-1_HP-1"));
    let app = knx_server::app(state, None);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/device/1")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let body = body_json(response).await;
    assert_eq!(body["product"]["resolution"], "Resolved");
}

#[tokio::test]
async fn device_catalog_is_a_read_only_snapshot_bound_batch() {
    let (_dir, products) = temp_product_db();
    let state = Arc::new(state_with_device(Some(products), "M-1_P-1", "H-1_HP-1"));
    let before = state.project.lock().unwrap().clone();
    let app = knx_server::app(state.clone(), None);
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/devices")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(
        response.status(),
        StatusCode::OK,
        "device catalogue batch GET must exist"
    );
    let body = body_json(response).await;
    assert_eq!(body["schemaVersion"], 1);
    assert_eq!(body["serverIncarnation"], state.server_incarnation);
    assert_eq!(body["snapshotRevision"], 0);
    assert_eq!(body["devices"].as_array().unwrap().len(), 1);
    let row = &body["devices"][0];
    assert_eq!(row["id"], 1);
    assert_eq!(
        row["device"]["name"], "Device 1",
        "unplaced devices still have their canonical identity"
    );
    assert_eq!(row["resolution"], "Resolved");
    assert_eq!(row["productText"], "Switch Actuator");
    assert_eq!(row["orderNumber"], "ORD-1");
    assert_eq!(*state.project.lock().unwrap(), before);
}

#[tokio::test]
async fn device_catalog_keeps_canonical_devices_when_product_lookup_fails() {
    let (_dir, products) = temp_product_db();
    products.execute_batch("DROP TABLE product").unwrap();
    let state = Arc::new(state_with_device(Some(products), "M-1_P-1", "H-1_HP-1"));
    let before = state.project.lock().unwrap().clone();
    let response = knx_server::app(state.clone(), None)
        .oneshot(
            Request::builder()
                .uri("/api/devices")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(
        response.status(),
        StatusCode::OK,
        "canonical device identity survives product lookup failure"
    );
    let body = body_json(response).await;
    assert_eq!(body["problem"], "lookupFailed");
    assert_eq!(body["devices"][0]["device"]["name"], "Device 1");
    assert_eq!(body["devices"][0]["resolution"], "Unavailable");
    assert!(body["devices"][0]["productText"].is_null());
    assert_eq!(*state.project.lock().unwrap(), before);
}

#[tokio::test]
async fn device_catalog_distinguishes_missing_reference_database_and_product() {
    for (products, product_ref, expected) in [
        (None, "", "NoReference"),
        (None, "missing", "NoDatabase"),
        (Some(temp_product_db().1), "missing", "NotInDatabase"),
    ] {
        let state = Arc::new(state_with_device(products, product_ref, ""));
        let response = knx_server::app(state.clone(), None)
            .oneshot(
                Request::builder()
                    .uri("/api/devices?language=de")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = body_json(response).await;
        assert_eq!(body["devices"][0]["resolution"], expected);
        assert!(body["devices"][0]["productText"].is_null());
        assert!(body["problem"].is_null());
    }
}

#[tokio::test]
async fn device_catalog_reuses_shared_product_metadata_and_keeps_every_device() {
    let (_dir, products) = temp_product_db();
    let state = Arc::new(state_with_device(Some(products), "M-1_P-1", "H-1_HP-1"));
    {
        let mut guard = state.project.lock().unwrap();
        let project = guard.as_mut().unwrap();
        let original = project.devices.get(DeviceId(1)).unwrap().clone();
        for id in [9, 2, 7] {
            let mut device = original.clone();
            device.id = DeviceId(id);
            device.name = format!("Device {id}");
            project.devices.insert(device);
        }
    }
    state
        .project_revision
        .store(7, std::sync::atomic::Ordering::Relaxed);
    let before = state.project.lock().unwrap().clone();
    let response = knx_server::app(state.clone(), None)
        .oneshot(
            Request::builder()
                .uri("/api/devices?language=de")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = body_json(response).await;
    assert_eq!(body["snapshotRevision"], 7);
    let rows = body["devices"].as_array().unwrap();
    assert_eq!(
        rows.iter()
            .map(|row| row["id"].as_u64().unwrap())
            .collect::<Vec<_>>(),
        vec![1, 2, 7, 9]
    );
    assert!(
        rows.iter()
            .all(|row| row["productText"] == "Switch Actuator"
                && row["productTextLanguage"].is_null())
    );
    assert_eq!(*state.project.lock().unwrap(), before);
}

#[tokio::test]
async fn device_catalog_refuses_no_project_without_changing_server_state() {
    let state = Arc::new(knx_server::AppState::default());
    let response = knx_server::app(state.clone(), None)
        .oneshot(
            Request::builder()
                .uri("/api/devices")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert!(state.project.lock().unwrap().is_none());
    assert_eq!(
        state
            .project_revision
            .load(std::sync::atomic::Ordering::Relaxed),
        0
    );
}

/// WP-20: one product per case, each with its own hardware, program link and
/// application program declaring `mask` (`None` omits the attribute).
fn system_family_product_db(
    cases: &[(&str, Option<&str>)],
) -> (tempfile::TempDir, knx_productdb::Connection) {
    let mut hardware = String::new();
    let mut programs = String::new();
    for (key, mask) in cases {
        hardware.push_str(&format!(
            r#"<Hardware Id="H-{key}" Name="X" SerialNumber="S" VersionNumber="1">
<Products><Product Id="M-1_P-{key}" Text="Product {key}" OrderNumber="ORD-{key}" /></Products>
<Hardware2Programs><Hardware2Program Id="H-{key}_HP-1" MediumTypes="MT-0">
<ApplicationProgramRef RefId="A-{key}" /></Hardware2Program></Hardware2Programs></Hardware>"#
        ));
        let mask = mask.map_or(String::new(), |m| format!(r#" MaskVersion="{m}""#));
        programs.push_str(&format!(
            r#"<ApplicationProgram Id="A-{key}" Name="P" ApplicationNumber="1" ApplicationVersion="1"{mask}>
<Static><ParameterTypes /><Parameters /><ParameterRefs /></Static><Dynamic /></ApplicationProgram>"#
        ));
    }
    let wrap = |body: &str, tag: &str| {
        format!(
            r#"<?xml version="1.0" encoding="utf-8"?>
<KNX xmlns="http://knx.org/xml/project/11"><ManufacturerData><Manufacturer RefId="M-1">
<{tag}>{body}</{tag}></Manufacturer></ManufacturerData></KNX>"#
        )
    };
    let dir = tempfile::tempdir().unwrap();
    let conn = knx_productdb::open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
    knx_productdb::ingest_file(
        &conn,
        "M-1/Hardware.xml",
        wrap(&hardware, "Hardware").as_bytes(),
    )
    .unwrap();
    knx_productdb::ingest_file(
        &conn,
        "M-1/A.xml",
        wrap(&programs, "ApplicationPrograms").as_bytes(),
    )
    .unwrap();
    (dir, conn)
}

/// Devices `1..` in order, each `(product_ref, program_ref)`.
fn state_with_devices(
    products: Option<knx_productdb::Connection>,
    refs: &[(&str, &str)],
) -> knx_server::AppState {
    let state = state_with_device(products, refs[0].0, refs[0].1);
    {
        let mut guard = state.project.lock().unwrap();
        let project = guard.as_mut().unwrap();
        let original = project.devices.get(DeviceId(1)).unwrap().clone();
        for (index, (product_ref, program_ref)) in refs.iter().enumerate().skip(1) {
            let mut device = original.clone();
            device.id = DeviceId(index as u32 + 1);
            device.product_ref = (*product_ref).into();
            device.program_ref = (*program_ref).into();
            project.devices.insert(device);
        }
    }
    state
}

async fn device_systems(state: knx_server::AppState) -> Vec<Value> {
    let state = Arc::new(state);
    let before = state.project.lock().unwrap().clone();
    let response = knx_server::app(state.clone(), None)
        .oneshot(
            Request::builder()
                .uri("/api/devices?language=de")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = body_json(response).await;
    assert_eq!(*state.project.lock().unwrap(), before, "read-only");
    body["devices"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| row["system"].clone())
        .collect()
}

#[tokio::test]
async fn device_catalog_names_each_mapped_system_family_from_the_declared_mask() {
    let cases = [
        ("s1", Some("MV-0012")),
        ("s2", Some("MV-0025")),
        ("s7", Some("MV-0701")),
        ("sb", Some("MV-07B0")),
        ("s300", Some("MV-0300")),
        ("cpl", Some("MV-0912")),
    ];
    let (_dir, products) = system_family_product_db(&cases);
    let refs: Vec<(String, String)> = cases
        .iter()
        .map(|(key, _)| (format!("M-1_P-{key}"), format!("H-{key}_HP-1")))
        .collect();
    let refs: Vec<(&str, &str)> = refs.iter().map(|(a, b)| (a.as_str(), b.as_str())).collect();
    let systems = device_systems(state_with_devices(Some(products), &refs)).await;
    assert_eq!(
        systems,
        vec![
            serde_json::json!({ "state": "family", "mask": 0x0012, "family": "system1" }),
            serde_json::json!({ "state": "family", "mask": 0x0025, "family": "system2" }),
            serde_json::json!({ "state": "family", "mask": 0x0701, "family": "system7" }),
            serde_json::json!({ "state": "family", "mask": 0x07B0, "family": "systemB" }),
            serde_json::json!({ "state": "family", "mask": 0x0300, "family": "system300" }),
            serde_json::json!({ "state": "family", "mask": 0x0912, "family": "coupler" }),
        ]
    );
}

#[tokio::test]
async fn device_catalog_reports_unknown_missing_and_unreadable_masks_explicitly() {
    let cases = [
        ("unknown", Some("MV-0AFD")),
        ("absent", None),
        ("bad", Some("MV-07")),
    ];
    let (_dir, products) = system_family_product_db(&cases);
    let systems = device_systems(state_with_devices(
        Some(products),
        &[
            ("M-1_P-unknown", "H-unknown_HP-1"),
            ("M-1_P-absent", "H-absent_HP-1"),
            ("M-1_P-bad", "H-bad_HP-1"),
        ],
    ))
    .await;
    assert_eq!(
        systems,
        vec![
            serde_json::json!({ "state": "unknownMask", "mask": 0x0AFD }),
            serde_json::json!({ "state": "noMaskVersion" }),
            serde_json::json!({ "state": "invalidMask", "declared": "MV-07" }),
        ]
    );
}

#[tokio::test]
async fn device_catalog_separates_missing_product_and_program_states() {
    let cases = [("a", Some("MV-0701")), ("b", Some("MV-07B0"))];
    let (_dir, products) = system_family_product_db(&cases);
    // A program link that is installed while its application program is not.
    knx_productdb::ingest_file(
        &products,
        "M-1/Hardware-ghost.xml",
        br#"<?xml version="1.0" encoding="utf-8"?>
<KNX xmlns="http://knx.org/xml/project/11"><ManufacturerData><Manufacturer RefId="M-1"><Hardware>
<Hardware Id="H-g" Name="X" SerialNumber="S" VersionNumber="1">
<Products><Product Id="M-1_P-g" Text="Product g" OrderNumber="ORD-g" /></Products>
<Hardware2Programs><Hardware2Program Id="H-g_HP-1" MediumTypes="MT-0">
<ApplicationProgramRef RefId="A-g" /></Hardware2Program></Hardware2Programs></Hardware>
</Hardware></Manufacturer></ManufacturerData></KNX>"#,
    )
    .unwrap();
    let systems = device_systems(state_with_devices(
        Some(products),
        &[
            ("", ""),                      // no product reference
            ("M-1_P-missing", "H-a_HP-1"), // product not installed
            ("M-1_P-a", ""),               // no program reference
            ("M-1_P-a", "H-a_HP-missing"), // program link not installed
            ("M-1_P-g", "H-g_HP-1"),       // link installed, program not
            ("M-1_P-a", "H-b_HP-1"),       // link belongs to other hardware
            ("M-1_P-a", "H-a_HP-1"),       // same product, matching program
        ],
    ))
    .await;
    let no_product = serde_json::json!({ "state": "noProduct" });
    let not_installed = serde_json::json!({ "state": "applicationNotInDatabase" });
    assert_eq!(
        systems,
        vec![
            no_product.clone(),
            no_product,
            serde_json::json!({ "state": "noApplicationProgram" }),
            not_installed.clone(),
            not_installed,
            serde_json::json!({ "state": "applicationNotOfProduct" }),
            serde_json::json!({ "state": "family", "mask": 0x0701, "family": "system7" }),
        ]
    );
}

#[tokio::test]
async fn device_catalog_without_database_or_with_failed_lookup_has_no_product_system() {
    let systems = device_systems(state_with_device(None, "M-1_P-1", "H-1_HP-1")).await;
    assert_eq!(systems, vec![serde_json::json!({ "state": "noProduct" })]);
    let (_dir, products) = temp_product_db();
    products.execute_batch("DROP TABLE product").unwrap();
    let systems = device_systems(state_with_device(Some(products), "M-1_P-1", "H-1_HP-1")).await;
    assert_eq!(systems, vec![serde_json::json!({ "state": "noProduct" })]);
}
