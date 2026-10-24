//! Exercises native carried sources and the catalogue-authoring portability boundary.
//!
//! The import tests save a fictional `.knxproj` whose device runs a tiny
//! application, then reopen it next to an empty product database. The separate
//! catalogue-authoring cases acquire exact originals and dependency sources on
//! Save/Save As, then reopen without external product data.

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use serde_json::{json, Value};
use std::{
    io::{Cursor, Write},
    path::Path,
    sync::Arc,
};
use tower::ServiceExt;

const PROJECT_INFO: &[u8] = br#"<KNX xmlns="http://knx.org/xml/project/11"><Project Id="P-0001"><ProjectInformation Name="Fictional dimmer" GroupAddressStyle="ThreeLevel" CompletionStatus="Undefined" /></Project></KNX>"#;

const TOPOLOGY: &[u8] = br#"<KNX xmlns="http://knx.org/xml/project/11"><Project Id="P-0001"><Installations><Installation InstallationId="0" Name="Fictional" CompletionStatus="Undefined"><Topology><Area Id="P-0001-0_A-1" Name="A" Address="0" CompletionStatus="Undefined"><Line Id="P-0001-0_L-1" Name="L" Address="2" MediumTypeRefId="MT-0" CompletionStatus="Undefined"><DeviceInstance Id="P-0001-0_DI-1" Name="Dimmaktor 4-fach" ProductRefId="M-0001_H-1_P-1" Hardware2ProgramRefId="M-0001_H-1_HP-1" Address="1" CompletionStatus="Undefined" /></Line></Area></Topology><GroupAddresses><GroupRanges /></GroupAddresses></Installation></Installations></Project></KNX>"#;

const HARDWARE: &[u8] = br#"<KNX xmlns="http://knx.org/xml/project/11"><ManufacturerData><Manufacturer RefId="M-0001"><Hardware><Hardware Id="M-0001_H-1" Name="Fictional dimmer" SerialNumber="S" VersionNumber="1"><Products><Product Id="M-0001_H-1_P-1" Text="Fictional dimmer" OrderNumber="FD-4" /></Products><Hardware2Programs><Hardware2Program Id="M-0001_H-1_HP-1" MediumTypes="MT-0"><ApplicationProgramRef RefId="M-0001_A-1" /></Hardware2Program></Hardware2Programs></Hardware></Hardware></Manufacturer></ManufacturerData></KNX>"#;

const PROGRAM: &[u8] = br#"<KNX xmlns="http://knx.org/xml/project/11"><ManufacturerData><Manufacturer RefId="M-0001"><ApplicationPrograms><ApplicationProgram Id="M-0001_A-1" Name="Fictional dimmer program" ApplicationVersion="1" MaskVersion="MV-0701"><Static><ParameterTypes><ParameterType Id="M-0001_A-1_PT-Num" Name="num"><TypeNumber maxInclusive="255" minInclusive="0" SizeInBit="8" Type="unsignedInt" /></ParameterType></ParameterTypes><Parameters><Parameter Id="M-0001_A-1_P-1" Name="Delay" Text="Delay" ParameterType="M-0001_A-1_PT-Num" Access="ReadWrite" Value="5" /></Parameters><ParameterRefs><ParameterRef Id="M-0001_A-1_P-1_R-1" RefId="M-0001_A-1_P-1" DisplayOrder="1" Tag="1" /></ParameterRefs></Static><Dynamic><ParameterRefRef RefId="M-0001_A-1_P-1_R-1" /></Dynamic></ApplicationProgram></ApplicationPrograms></Manufacturer></ManufacturerData></KNX>"#;

const HARDWARE_PATH: &str = "M-0001/Hardware.xml";
const PROGRAM_PATH: &str = "M-0001/M-0001_A-1.xml";
const CATALOG_ITEM: &str = "M-0001_CI-1";
const CATALOG_MASTER: &[u8] = br#"<KNX xmlns="http://knx.org/xml/project/11"><MasterData><Manufacturers><Manufacturer Id="M-0001" Name="Fictional"/></Manufacturers></MasterData></KNX>"#;
const CATALOG_SOURCE: &[u8] = br#"<KNX xmlns="http://knx.org/xml/project/11"><ManufacturerData><Manufacturer RefId="M-0001"><Catalog><CatalogSection Id="M-0001_CG-1" Name="Fictional devices" Number="1"><CatalogItem Id="M-0001_CI-1" Name="Fictional dimmer" Number="FD-4" ProductRefId="M-0001_H-1_P-1" Hardware2ProgramRefId="M-0001_H-1_HP-1"/></CatalogSection></Catalog></Manufacturer></ManufacturerData></KNX>"#;

fn zip(entries: &[(&str, &[u8])]) -> Vec<u8> {
    let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for (name, bytes) in entries {
        zip.start_file(*name, zip::write::SimpleFileOptions::default())
            .unwrap();
        zip.write_all(bytes).unwrap();
    }
    zip.finish().unwrap().into_inner()
}

fn knxproj() -> Vec<u8> {
    zip(&[
        ("P-0001.signature", b"x".as_slice()),
        ("P-0001/Project.xml", PROJECT_INFO),
        ("P-0001/0.xml", TOPOLOGY),
        (HARDWARE_PATH, HARDWARE),
        (PROGRAM_PATH, PROGRAM),
    ])
}

fn state_with_products(data_dir: &Path, products: &Path) -> Arc<knx_server::AppState> {
    Arc::new(knx_server::AppState {
        product_db: Some(
            knx_server::CatalogueOwner::adopt_opened(
                knx_productdb::open_and_migrate(products).unwrap(),
                std::path::PathBuf::new(),
            )
            .unwrap(),
        ),
        ..knx_server::AppState::new(data_dir.into())
    })
}

async fn response(app: &axum::Router, path: &str, data: Option<Value>) -> (StatusCode, Value) {
    let request = Request::builder()
        .method(if data.is_some() { "POST" } else { "GET" })
        .uri(path)
        .header("content-type", "application/json")
        .body(data.map_or_else(Body::empty, |v| Body::from(v.to_string())))
        .unwrap();
    let response = app.clone().oneshot(request).await.unwrap();
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), 20 * 1024 * 1024)
        .await
        .unwrap();
    let body = if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&bytes).unwrap()
    };
    (status, body)
}

async fn ok(app: &axum::Router, path: &str, data: Option<Value>) -> Value {
    let (status, body) = response(app, path, data).await;
    assert_eq!(status, StatusCode::OK, "{path} refused: {body}");
    body
}

fn only_device(state: &knx_server::AppState) -> u32 {
    let project = state.project.lock().unwrap();
    let ids: Vec<u32> = project
        .as_ref()
        .unwrap()
        .devices
        .iter()
        .map(|device| device.id.0)
        .collect();
    assert_eq!(ids.len(), 1, "the fixture has exactly one device");
    ids[0]
}

async fn parameter_program(app: &axum::Router, device: u32) -> Value {
    ok(app, &format!("/api/device/{device}/parameters"), None).await["programId"].clone()
}

/// Imports the fixture with product database `products_a` and saves it as
/// `name`. Returns the saved path.
async fn import_and_save(dir: &Path, products_a: &Path, name: &str) -> std::path::PathBuf {
    let source = dir.join("fictional.knxproj");
    std::fs::write(&source, knxproj()).unwrap();
    let native = dir.join(name);
    let state = state_with_products(dir, products_a);
    let app = knx_server::app(state.clone(), None);
    ok(&app, "/api/project/import", Some(json!({ "path": source }))).await;
    let device = only_device(&state);
    assert_eq!(
        parameter_program(&app, device).await,
        "M-0001_A-1",
        "the import itself resolves the program"
    );
    ok(
        &app,
        "/api/project/save-as",
        Some(json!({ "path": native })),
    )
    .await;
    native
}

#[tokio::test]
async fn a_reopened_knxdb_resolves_its_program_with_an_empty_product_database() {
    let dir = tempfile::tempdir().unwrap();
    let native = import_and_save(dir.path(), &dir.path().join("a.sqlite"), "saved.knxdb").await;

    // A rebuilt container or another computer: same file, empty products.
    let fresh = dir.path().join("b.sqlite");
    let state = state_with_products(dir.path(), &fresh);
    let app = knx_server::app(state.clone(), None);
    ok(&app, "/api/project/open", Some(json!({ "path": native }))).await;
    let device = only_device(&state);
    assert_eq!(
        parameter_program(&app, device).await,
        "M-0001_A-1",
        "the reopened project must resolve its application program (#21)"
    );
}

fn carried_rows(native: &Path) -> i64 {
    let read = knx_store::open_existing_read_only(native).unwrap();
    read.conn
        .query_row("SELECT count(*) FROM manufacturer_file", [], |r| r.get(0))
        .unwrap()
}

/// What a `.knxdb` saved before ADR-0128 looks like after migration: the
/// manifest without a single carried file.
fn forget_carried_files(native: &Path) {
    let conn = knx_store::open_existing_and_migrate(native).unwrap();
    conn.execute("DELETE FROM manufacturer_file", []).unwrap();
}

fn log_messages(state: &knx_server::AppState) -> Vec<String> {
    state
        .session_log
        .lock()
        .unwrap()
        .entries()
        .iter()
        .map(|entry| entry.message.clone())
        .collect()
}

#[tokio::test]
async fn saving_stores_each_manifest_file_once_and_compressed() {
    let dir = tempfile::tempdir().unwrap();
    let products = dir.path().join("a.sqlite");
    let native = import_and_save(dir.path(), &products, "saved.knxdb").await;
    assert_eq!(carried_rows(&native), 2, "hardware and program");
    // A second save in place neither duplicates nor drops a row.
    let state = state_with_products(dir.path(), &products);
    let app = knx_server::app(state.clone(), None);
    ok(&app, "/api/project/open", Some(json!({ "path": native }))).await;
    ok(&app, "/api/project/save", Some(json!({}))).await;
    assert_eq!(carried_rows(&native), 2);
    let read = knx_store::open_existing_read_only(&native).unwrap();
    let sha = knx_store::manufacturer_file_hash(PROGRAM);
    let program = knx_store::load_manufacturer_file(&read.conn, &sha)
        .unwrap()
        .unwrap();
    assert_eq!(program.bytes, PROGRAM);
    assert_eq!(program.source_path, PROGRAM_PATH);
}

#[tokio::test]
async fn an_older_knxdb_names_the_gap_and_restores_from_the_original_knxproj() {
    let dir = tempfile::tempdir().unwrap();
    let native = import_and_save(dir.path(), &dir.path().join("a.sqlite"), "old.knxdb").await;
    forget_carried_files(&native);

    let state = state_with_products(dir.path(), &dir.path().join("b.sqlite"));
    let app = knx_server::app(state.clone(), None);
    ok(&app, "/api/project/open", Some(json!({ "path": native }))).await;
    assert!(
        log_messages(&state)
            .iter()
            .any(|m| m.contains("2 manufacturer file(s) of this project are neither")),
        "the open names the gap: {:?}",
        log_messages(&state)
    );
    let device = only_device(&state);
    let panel = ok(&app, &format!("/api/device/{device}/parameters"), None).await;
    assert_eq!(panel["programId"], Value::Null);
    assert_eq!(
        panel["unresolvedProgram"],
        json!({ "programRef": "M-0001_H-1_HP-1", "productDatabase": true, "missingFiles": 2 })
    );

    let before = ok(&app, "/api/project", None).await;
    let restored = ok(
        &app,
        "/api/project/manufacturer-files/restore",
        Some(json!({ "path": dir.path().join("fictional.knxproj") })),
    )
    .await;
    assert_eq!(restored["installed"], json!([HARDWARE_PATH, PROGRAM_PATH]));
    assert_eq!(restored["stillMissing"], 0);
    assert_eq!(restored["mismatched"], json!([]));
    assert_eq!(
        ok(&app, "/api/project", None).await,
        before,
        "restoring product data changes nothing in the project"
    );
    assert_eq!(parameter_program(&app, device).await, "M-0001_A-1");
    let panel = ok(&app, &format!("/api/device/{device}/parameters"), None).await;
    assert_eq!(panel["unresolvedProgram"], Value::Null);

    assert_eq!(carried_rows(&native), 0, "restore writes no project file");
    ok(&app, "/api/project/save", Some(json!({}))).await;
    assert_eq!(carried_rows(&native), 2, "the next save stores them");
}

#[tokio::test]
async fn an_archive_with_other_bytes_installs_nothing_of_them() {
    let dir = tempfile::tempdir().unwrap();
    let native = import_and_save(dir.path(), &dir.path().join("a.sqlite"), "old.knxdb").await;
    forget_carried_files(&native);
    let changed = String::from_utf8(PROGRAM.to_vec())
        .unwrap()
        .replace("Text=\"Delay\"", "Text=\"Delay (edited)\"");
    let other = dir.path().join("other.knxproj");
    std::fs::write(
        &other,
        zip(&[
            ("P-0001.signature", b"x".as_slice()),
            ("P-0001/Project.xml", PROJECT_INFO),
            ("P-0001/0.xml", TOPOLOGY),
            (HARDWARE_PATH, HARDWARE),
            (PROGRAM_PATH, changed.as_bytes()),
        ]),
    )
    .unwrap();

    let state = state_with_products(dir.path(), &dir.path().join("b.sqlite"));
    let app = knx_server::app(state.clone(), None);
    ok(&app, "/api/project/open", Some(json!({ "path": native }))).await;
    let restored = ok(
        &app,
        "/api/project/manufacturer-files/restore",
        Some(json!({ "path": other })),
    )
    .await;
    assert_eq!(restored["installed"], json!([HARDWARE_PATH]));
    assert_eq!(restored["mismatched"], json!([PROGRAM_PATH]));
    assert_eq!(restored["stillMissing"], 1);
    assert_eq!(
        parameter_program(&app, only_device(&state)).await,
        Value::Null,
        "a program with other bytes is not the one the project was made with"
    );
}

#[tokio::test]
async fn a_password_protected_original_restores_without_its_password() {
    let dir = tempfile::tempdir().unwrap();
    let native = import_and_save(dir.path(), &dir.path().join("a.sqlite"), "old.knxdb").await;
    forget_carried_files(&native);
    // The project part sits in an encrypted nested archive; the
    // manufacturer files stay outside it, as in a protected ETS export.
    let protected = dir.path().join("protected.knxproj");
    std::fs::write(
        &protected,
        zip(&[
            ("P-0001.signature", b"x".as_slice()),
            ("P-0001.zip", b"encrypted payload, never read".as_slice()),
            (HARDWARE_PATH, HARDWARE),
            (PROGRAM_PATH, PROGRAM),
        ]),
    )
    .unwrap();
    let state = state_with_products(dir.path(), &dir.path().join("b.sqlite"));
    let app = knx_server::app(state.clone(), None);
    ok(&app, "/api/project/open", Some(json!({ "path": native }))).await;
    let restored = ok(
        &app,
        "/api/project/manufacturer-files/restore",
        Some(json!({ "path": protected })),
    )
    .await;
    assert_eq!(restored["stillMissing"], 0);
    assert_eq!(
        parameter_program(&app, only_device(&state)).await,
        "M-0001_A-1"
    );
}

#[tokio::test]
async fn save_as_without_a_product_database_copies_the_carried_files() {
    let dir = tempfile::tempdir().unwrap();
    let native = import_and_save(dir.path(), &dir.path().join("a.sqlite"), "first.knxdb").await;
    let state = Arc::new(knx_server::AppState {
        product_db: None,
        ..knx_server::AppState::new(dir.path().into())
    });
    let app = knx_server::app(state.clone(), None);
    ok(&app, "/api/project/open", Some(json!({ "path": native }))).await;
    let device = only_device(&state);
    let panel = ok(&app, &format!("/api/device/{device}/parameters"), None).await;
    assert_eq!(
        panel["unresolvedProgram"],
        json!({ "programRef": "M-0001_H-1_HP-1", "productDatabase": false, "missingFiles": 0 })
    );
    let copy = dir.path().join("copy.knxdb");
    ok(&app, "/api/project/save-as", Some(json!({ "path": copy }))).await;
    assert_eq!(carried_rows(&copy), 2, "the earlier file supplied them");
    assert!(
        !log_messages(&state)
            .iter()
            .any(|m| m.contains("cannot carry")),
        "nothing was missing: {:?}",
        log_messages(&state)
    );

    // The copy restores into an empty product database like the original.
    let fresh = state_with_products(dir.path(), &dir.path().join("c.sqlite"));
    let app = knx_server::app(fresh.clone(), None);
    ok(&app, "/api/project/open", Some(json!({ "path": copy }))).await;
    assert_eq!(
        parameter_program(&app, only_device(&fresh)).await,
        "M-0001_A-1"
    );
}

#[tokio::test]
async fn a_plain_save_without_a_product_database_keeps_the_carried_files() {
    let dir = tempfile::tempdir().unwrap();
    let native = import_and_save(dir.path(), &dir.path().join("a.sqlite"), "kept.knxdb").await;
    let state = Arc::new(knx_server::AppState {
        product_db: None,
        ..knx_server::AppState::new(dir.path().into())
    });
    let app = knx_server::app(state.clone(), None);
    ok(&app, "/api/project/open", Some(json!({ "path": native }))).await;
    // A real edit, so the save writes the project again.
    ok(
        &app,
        "/api/group-addresses",
        Some(json!({ "name": "Fictional edit", "address": "1/2/3" })),
    )
    .await;
    ok(&app, "/api/project/save", Some(json!({}))).await;
    assert_eq!(
        carried_rows(&native),
        2,
        "saving in place without any product database must not drop them"
    );
    let fresh = state_with_products(dir.path(), &dir.path().join("b.sqlite"));
    let app = knx_server::app(fresh.clone(), None);
    ok(&app, "/api/project/open", Some(json!({ "path": native }))).await;
    assert_eq!(
        parameter_program(&app, only_device(&fresh)).await,
        "M-0001_A-1"
    );
}

#[tokio::test]
async fn a_save_that_cannot_carry_a_file_says_so() {
    let dir = tempfile::tempdir().unwrap();
    let native = import_and_save(dir.path(), &dir.path().join("a.sqlite"), "old.knxdb").await;
    forget_carried_files(&native);
    let state = state_with_products(dir.path(), &dir.path().join("b.sqlite"));
    let app = knx_server::app(state.clone(), None);
    ok(&app, "/api/project/open", Some(json!({ "path": native }))).await;
    ok(&app, "/api/project/save", Some(json!({}))).await;
    assert!(
        log_messages(&state)
            .iter()
            .any(|m| m.contains("so the project file cannot carry them")),
        "{:?}",
        log_messages(&state)
    );
    assert!(log_messages(&state)
        .iter()
        .any(|m| m.contains("source ownership is unavailable")
            && m.contains("portability is not established")));
    assert_eq!(carried_rows(&native), 0);
}

#[tokio::test]
async fn restore_refusals_change_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let state = state_with_products(dir.path(), &dir.path().join("a.sqlite"));
    let app = knx_server::app(state.clone(), None);
    let source = dir.path().join("fictional.knxproj");
    std::fs::write(&source, knxproj()).unwrap();
    let (status, _) = response(
        &app,
        "/api/project/manufacturer-files/restore",
        Some(json!({ "path": source })),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "no project open");

    let native = import_and_save(dir.path(), &dir.path().join("a.sqlite"), "x.knxdb").await;
    ok(&app, "/api/project/open", Some(json!({ "path": native }))).await;
    let not_zip = dir.path().join("notes.txt");
    std::fs::write(&not_zip, b"not an archive").unwrap();
    let (status, body) = response(
        &app,
        "/api/project/manufacturer-files/restore",
        Some(json!({ "path": not_zip })),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{body}");

    let without = Arc::new(knx_server::AppState {
        product_db: None,
        ..knx_server::AppState::new(dir.path().into())
    });
    let app = knx_server::app(without, None);
    ok(&app, "/api/project/open", Some(json!({ "path": native }))).await;
    let (status, body) = response(
        &app,
        "/api/project/manufacturer-files/restore",
        Some(json!({ "path": source })),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(body["kind"], "noProductDatabase", "{body}");
}

#[tokio::test]
async fn a_downloaded_knxdb_carries_the_manufacturer_files_too() {
    let dir = tempfile::tempdir().unwrap();
    let products = dir.path().join("a.sqlite");
    let source = dir.path().join("fictional.knxproj");
    std::fs::write(&source, knxproj()).unwrap();
    let state = state_with_products(dir.path(), &products);
    let app = knx_server::app(state.clone(), None);
    // Imported, never saved: the browser download is the only file.
    ok(&app, "/api/project/import", Some(json!({ "path": source }))).await;
    let request = Request::builder()
        .uri("/api/project/download")
        .body(Body::empty())
        .unwrap();
    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(response.into_body(), 64 * 1024 * 1024)
        .await
        .unwrap();
    let downloaded = dir.path().join("downloaded.knxdb");
    std::fs::write(&downloaded, &bytes).unwrap();
    assert_eq!(carried_rows(&downloaded), 2);
}

fn catalogue_fixture_package() -> Vec<u8> {
    zip(&[
        ("knx_master.xml", CATALOG_MASTER),
        ("M-0001/Catalog.xml", CATALOG_SOURCE),
        (HARDWARE_PATH, HARDWARE),
        (PROGRAM_PATH, PROGRAM),
        ("M-0001/Baggages/opaque.bin", b"\x00opaque vendor bytes\xff"),
    ])
}

async fn install_catalogue_fixture(app: &axum::Router) {
    install_catalogue_bytes(app, &catalogue_fixture_package()).await;
}

async fn install_catalogue_bytes(app: &axum::Router, package: &[u8]) {
    let boundary = "fictional-catalogue-portability";
    let mut body = format!(
        "--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"fictional.knxprod\"\r\n\r\n"
    )
    .into_bytes();
    body.extend_from_slice(package);
    body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/catalog/install")
                .header(
                    "content-type",
                    format!("multipart/form-data; boundary={boundary}"),
                )
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK, "fixture install refused");
    let bytes = axum::body::to_bytes(response.into_body(), 20 * 1024 * 1024)
        .await
        .unwrap();
    let report: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(report["skipped"], false);
}

#[tokio::test]
async fn a_catalogue_created_device_reopens_without_external_products_after_save() {
    let dir = tempfile::tempdir().unwrap();
    let state = state_with_products(dir.path(), &dir.path().join("authoring.sqlite"));
    let app = knx_server::app(state.clone(), None);
    ok(&app, "/api/project/new", Some(json!({}))).await;
    install_catalogue_fixture(&app).await;
    let items = ok(&app, "/api/catalog/items", None).await;
    assert_eq!(items.as_array().unwrap().len(), 1);
    assert_eq!(items[0]["id"], CATALOG_ITEM);
    ok(
        &app,
        "/api/devices",
        Some(json!({ "catalogItemId": CATALOG_ITEM, "name": "Authored dimmer" })),
    )
    .await;
    let device = only_device(&state);
    assert_eq!(parameter_program(&app, device).await, "M-0001_A-1");
    let original = state.project.lock().unwrap().clone().unwrap();
    let native = dir.path().join("authored.knxdb");
    ok(
        &app,
        "/api/project/save-as",
        Some(json!({ "path": native })),
    )
    .await;
    assert!(
        native.is_file(),
        "Save As must persist the authored project"
    );
    ok(&app, "/api/project/save", Some(json!({}))).await;
    {
        let read = knx_store::open_existing_read_only(&native).unwrap();
        let refs = knx_store::load_manufacturer_refs(&read.conn).unwrap();
        assert_eq!(
            refs.len(),
            5,
            "exact original plus master, catalog, hardware and program"
        );
        assert_eq!(carried_rows(&native), 5);
        let original_hash = knx_store::manufacturer_file_hash(&catalogue_fixture_package());
        assert!(refs
            .iter()
            .any(|r| r.kind == "CataloguePackage" && r.sha256 == original_hash));
        assert_eq!(
            knx_store::load_manufacturer_file(&read.conn, &original_hash)
                .unwrap()
                .unwrap()
                .bytes,
            catalogue_fixture_package()
        );
        for bytes in [CATALOG_MASTER, CATALOG_SOURCE, HARDWARE, PROGRAM] {
            let hash = knx_store::manufacturer_file_hash(bytes);
            assert!(refs.iter().any(|r| r.sha256 == hash));
            assert_eq!(
                knx_store::load_manufacturer_file(&read.conn, &hash)
                    .unwrap()
                    .unwrap()
                    .bytes,
                bytes
            );
        }
        state
            .product_db
            .as_ref()
            .unwrap()
            .with_read(|products| {
                for bytes in [HARDWARE, PROGRAM, CATALOG_SOURCE] {
                    let hash = knx_store::manufacturer_file_hash(bytes);
                    assert!(knx_productdb::has_source_file(products, &hash).unwrap());
                }
                let original_bytes = catalogue_fixture_package();
                let expected: usize = [
                    original_bytes.as_slice(),
                    CATALOG_MASTER,
                    CATALOG_SOURCE,
                    HARDWARE,
                    PROGRAM,
                ]
                .iter()
                .map(|bytes| bytes.len())
                .sum();
                let max_file = [
                    original_bytes.as_slice(),
                    CATALOG_MASTER,
                    CATALOG_SOURCE,
                    HARDWARE,
                    PROGRAM,
                ]
                .iter()
                .map(|bytes| bytes.len())
                .max()
                .unwrap();
                let before = products.total_changes();
                let sources = knx_productdb::catalogue_sources::catalogue_dependency_sources(
                    products,
                    "M-0001_H-1_P-1",
                    "M-0001_H-1_HP-1",
                    max_file,
                    expected as u64,
                )
                .unwrap();
                assert_eq!(sources.len(), 5);
                assert_eq!(
                    sources.iter().map(|s| s.bytes.len()).sum::<usize>(),
                    expected
                );
                for (per_file, total, reason) in [
                    (0, expected as u64, "retained-value limit"),
                    (max_file, (expected - 1) as u64, "expanded archive limit"),
                ] {
                    let refusal =
                        match knx_productdb::catalogue_sources::catalogue_dependency_sources(
                            products,
                            "M-0001_H-1_P-1",
                            "M-0001_H-1_HP-1",
                            per_file,
                            total,
                        ) {
                            Err(error) => error,
                            Ok(_) => panic!("source limit must refuse"),
                        };
                    assert!(refusal.to_string().contains(reason));
                }
                assert_eq!(
                    products.total_changes(),
                    before,
                    "source acquisition is read only"
                );
                Ok::<(), std::convert::Infallible>(())
            })
            .unwrap()
            .unwrap();
    }

    // The saved model and package are valid: the same file resolves while
    // the original installed product data remain available.
    let same = state_with_products(dir.path(), &dir.path().join("authoring.sqlite"));
    let same_app = knx_server::app(same.clone(), None);
    ok(
        &same_app,
        "/api/project/open",
        Some(json!({ "path": native })),
    )
    .await;
    assert_eq!(same.project.lock().unwrap().as_ref(), Some(&original));
    assert_eq!(parameter_program(&same_app, device).await, "M-0001_A-1");

    let fresh = state_with_products(dir.path(), &dir.path().join("empty.sqlite"));
    {
        let count: i64 = fresh
            .product_db
            .as_ref()
            .unwrap()
            .with_read(|products| {
                Ok::<_, std::convert::Infallible>(
                    products
                        .query_row("SELECT count(*) FROM application_program", [], |r| r.get(0))
                        .unwrap(),
                )
            })
            .unwrap()
            .unwrap();
        assert_eq!(count, 0, "reopen must start without installed programs");
    }
    let reopened = knx_server::app(fresh.clone(), None);
    ok(
        &reopened,
        "/api/project/open",
        Some(json!({ "path": native })),
    )
    .await;
    assert_eq!(fresh.project.lock().unwrap().as_ref(), Some(&original));
    fresh
        .product_db
        .as_ref()
        .unwrap()
        .with_read(|products| {
            let opaque = b"\x00opaque vendor bytes\xff";
            assert_eq!(
                knx_productdb::load_source_file(
                    products,
                    &knx_store::manufacturer_file_hash(opaque)
                )
                .unwrap()
                .as_deref(),
                Some(opaque.as_slice())
            );
            let master_name: String = products
                .query_row("SELECT name FROM manufacturer WHERE id='M-0001'", [], |r| {
                    r.get(0)
                })
                .unwrap();
            assert_eq!(master_name, "Fictional");
            assert!(knx_productdb::catalogue_sources::has_catalogue_original(
                products,
                &knx_store::manufacturer_file_hash(&catalogue_fixture_package())
            )
            .unwrap());
            Ok::<(), std::convert::Infallible>(())
        })
        .unwrap()
        .unwrap();
    let panel = ok(&reopened, &format!("/api/device/{device}/parameters"), None).await;
    assert_eq!(panel["programId"], "M-0001_A-1");
    assert_eq!(panel["unresolvedProgram"], Value::Null);
    assert_eq!(carried_rows(&native), 5);

    // Save As must carry the same originals even without a product database.
    let offline = Arc::new(knx_server::AppState {
        product_db: None,
        ..knx_server::AppState::new(dir.path().into())
    });
    let offline_app = knx_server::app(offline, None);
    ok(
        &offline_app,
        "/api/project/open",
        Some(json!({ "path": native })),
    )
    .await;
    let copy = dir.path().join("authored-copy.knxdb");
    ok(
        &offline_app,
        "/api/project/save-as",
        Some(json!({ "path": copy })),
    )
    .await;
    assert_eq!(carried_rows(&copy), 5);
    let another = state_with_products(dir.path(), &dir.path().join("another-empty.sqlite"));
    let another_app = knx_server::app(another.clone(), None);
    ok(
        &another_app,
        "/api/project/open",
        Some(json!({ "path": copy })),
    )
    .await;
    assert_eq!(parameter_program(&another_app, device).await, "M-0001_A-1");
    assert_eq!(another.project.lock().unwrap().as_ref(), Some(&original));
}

#[tokio::test]
async fn a_plain_save_acquires_new_catalogue_sources_before_empty_database_reopen() {
    let dir = tempfile::tempdir().unwrap();
    let state = state_with_products(dir.path(), &dir.path().join("authoring.sqlite"));
    let app = knx_server::app(state.clone(), None);
    ok(&app, "/api/project/new", Some(json!({}))).await;
    let native = dir.path().join("plain-save.knxdb");
    ok(
        &app,
        "/api/project/save-as",
        Some(json!({ "path": native })),
    )
    .await;
    assert_eq!(carried_rows(&native), 0);
    install_catalogue_fixture(&app).await;
    ok(
        &app,
        "/api/devices",
        Some(json!({ "catalogItemId": CATALOG_ITEM, "name": "Added after Save As" })),
    )
    .await;
    let original = state.project.lock().unwrap().clone();
    ok(&app, "/api/project/save", Some(json!({}))).await;
    assert_eq!(carried_rows(&native), 5);
    let fresh = state_with_products(dir.path(), &dir.path().join("empty.sqlite"));
    let reopened = knx_server::app(fresh.clone(), None);
    ok(
        &reopened,
        "/api/project/open",
        Some(json!({ "path": native })),
    )
    .await;
    assert_eq!(
        parameter_program(&reopened, only_device(&fresh)).await,
        "M-0001_A-1"
    );
    assert_eq!(*fresh.project.lock().unwrap(), original);
}

#[tokio::test]
async fn catalogue_source_refusals_preserve_save_and_save_as_state() {
    use std::sync::atomic::Ordering;
    for (control, expected_error) in [
        (
            "missing-original",
            "required original package is unavailable",
        ),
        ("corrupt-original", "original package hash mismatch"),
        ("corrupt-source", "required source member hash mismatch"),
        (
            "ambiguous-package",
            "package ownership is unavailable or ambiguous",
        ),
        (
            "missing-membership",
            "package ownership is unavailable or ambiguous",
        ),
        ("missing-mapping", "mapping is unavailable"),
        (
            "missing-mapping-with-legacy-manifest",
            "mapping is unavailable",
        ),
        (
            "corrupt-membership",
            "required member is absent from the original package",
        ),
        ("divergent-source", "ambiguous source declarations"),
    ] {
        let dir = tempfile::tempdir().unwrap();
        let products_path = dir.path().join("authoring.sqlite");
        let state = state_with_products(dir.path(), &products_path);
        let app = knx_server::app(state.clone(), None);
        ok(&app, "/api/project/new", Some(json!({}))).await;
        let native = dir.path().join("baseline.knxdb");
        ok(
            &app,
            "/api/project/save-as",
            Some(json!({ "path": native })),
        )
        .await;
        install_catalogue_fixture(&app).await;
        ok(
            &app,
            "/api/devices",
            Some(json!({ "catalogItemId": CATALOG_ITEM, "name": "Unsaved dimmer" })),
        )
        .await;
        // Deliberate fixture damage, through a separate idle connection.
        let conn = rusqlite::Connection::open(&products_path).unwrap();
        conn.pragma_update(None, "foreign_keys", false).unwrap();
        match control {
            "missing-original" => {
                conn.execute("DELETE FROM package", []).unwrap();
            }
            "corrupt-original" => {
                conn.execute("UPDATE package SET bytes=x'00'", []).unwrap();
            }
            "corrupt-source" => {
                conn.execute(
                    "UPDATE source_file SET bytes=zeroblob(length(bytes)) WHERE sha256=?1",
                    [knx_store::manufacturer_file_hash(PROGRAM)],
                )
                .unwrap();
            }
            "ambiguous-package" => {
                let other = zip(&[
                    ("knx_master.xml", CATALOG_MASTER),
                    ("M-0001/Catalog.xml", CATALOG_SOURCE),
                    (HARDWARE_PATH, HARDWARE),
                    (PROGRAM_PATH, PROGRAM),
                    ("M-0001/Baggages/other.bin", b"another original"),
                ]);
                knx_productdb::install_package(&conn, "other.knxprod", &other).unwrap();
            }
            "missing-membership" => {
                conn.execute("DELETE FROM package_member", []).unwrap();
            }
            "corrupt-membership" => {
                conn.execute(
                    "UPDATE package_member SET path='M-0001/Missing.xml' WHERE source_sha256=?1",
                    [knx_store::manufacturer_file_hash(PROGRAM)],
                )
                .unwrap();
            }
            "divergent-source" => {
                let other = String::from_utf8(PROGRAM.to_vec())
                    .unwrap()
                    .replace("Fictional dimmer program", "Divergent program");
                knx_productdb::ingest_file(&conn, "M-0001/Divergent.xml", other.as_bytes())
                    .unwrap();
            }
            "missing-mapping" | "missing-mapping-with-legacy-manifest" => {
                conn.execute("DELETE FROM hardware2program", []).unwrap();
                if control == "missing-mapping-with-legacy-manifest" {
                    // A real retained Hardware member is not evidence that this
                    // newly authored device's missing application mapping exists.
                    state
                        .manufacturer_refs
                        .lock()
                        .unwrap()
                        .push(knx_store::ManufacturerRef {
                            source_path: HARDWARE_PATH.into(),
                            sha256: knx_store::manufacturer_file_hash(HARDWARE),
                            len: HARDWARE.len() as i64,
                            kind: "Hardware".into(),
                        });
                }
            }
            _ => unreachable!(),
        }
        drop(conn);
        let before = std::fs::read(&native).unwrap();
        let model = state.project.lock().unwrap().clone();
        let clean = state.clean_project.lock().unwrap().clone();
        let refs = state.manufacturer_refs.lock().unwrap().clone();
        let generation = *state.history_generation.lock().unwrap();
        let revision = state.project_revision.load(Ordering::SeqCst);
        let saved_at = state.last_saved_at.lock().unwrap().clone();
        let fresh = dir.path().join("refused.knxdb");
        for (route, data) in [
            ("/api/project/save", json!({})),
            ("/api/project/save-as", json!({ "path": fresh })),
        ] {
            let (status, body) = response(&app, route, Some(data)).await;
            assert!(!status.is_success(), "{control} must refuse {route}");
            assert!(
                body.to_string().contains(expected_error),
                "{control}: {body}"
            );
            assert_eq!(
                std::fs::read(&native).unwrap(),
                before,
                "{control}: destination untouched"
            );
            assert!(!fresh.exists());
            assert_eq!(*state.project.lock().unwrap(), model);
            assert_eq!(*state.clean_project.lock().unwrap(), clean);
            assert_eq!(*state.manufacturer_refs.lock().unwrap(), refs);
            assert_eq!(*state.history_generation.lock().unwrap(), generation);
            assert_eq!(state.project_revision.load(Ordering::SeqCst), revision);
            assert_eq!(*state.last_saved_at.lock().unwrap(), saved_at);
            assert_eq!(
                state.store_path.lock().unwrap().as_deref(),
                Some(native.as_path())
            );
        }
    }
}

#[tokio::test]
async fn unresolved_imports_without_a_manifest_preserve_model_and_opaque_bytes_on_save() {
    for with_products in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("unresolved.knxproj");
        std::fs::write(
            &source,
            knx_testsupport::PluginFixture::default().knxproj_bytes(),
        )
        .unwrap();
        let state = if with_products {
            state_with_products(dir.path(), &dir.path().join("empty.sqlite"))
        } else {
            Arc::new(knx_server::AppState::new(dir.path().into()))
        };
        let app = knx_server::app(state.clone(), None);
        ok(&app, "/api/project/import", Some(json!({"path":source}))).await;
        let model = state.project.lock().unwrap().clone();
        let opaque = state.opaque.lock().unwrap().clone();
        assert!(!opaque.is_empty());
        assert!(state.manufacturer_refs.lock().unwrap().is_empty());
        let native = dir.path().join("unresolved.knxdb");
        ok(&app, "/api/project/save-as", Some(json!({"path":native}))).await;
        ok(&app, "/api/project/save", Some(json!({}))).await;
        assert_eq!(*state.project.lock().unwrap(), model);
        assert_eq!(*state.opaque.lock().unwrap(), opaque);
        assert!(state.manufacturer_refs.lock().unwrap().is_empty());
        assert_eq!(carried_rows(&native), 0, "do not invent ownership");
        assert!(log_messages(&state).iter().any(|message| {
            message.contains("source ownership is unavailable")
                && message.contains("portability is not established")
        }));
        ok(&app, "/api/project/open", Some(json!({"path":native}))).await;
        assert_eq!(*state.project.lock().unwrap(), model);
        assert_eq!(*state.opaque.lock().unwrap(), opaque);
        let copy = dir.path().join("unresolved-copy.knxdb");
        ok(&app, "/api/project/save-as", Some(json!({"path":copy}))).await;
        let read = knx_store::open_existing_read_only(&copy).unwrap();
        let snapshot = knx_store::project_history::NativeSnapshot::read(&read.conn).unwrap();
        assert_eq!(Some(snapshot.project), model);
        assert_eq!(snapshot.opaque, opaque);
        assert!(snapshot.manufacturer_refs.is_empty());
        assert_eq!(carried_rows(&copy), 0);
    }
}

#[tokio::test]
async fn unresolved_imports_remain_unknown_beside_acquired_catalogue_packages() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("unresolved.knxproj");
    let imported = knx_testsupport::PluginFixture {
        product: "Unknown imported product",
        program: "Unknown imported program",
        ..Default::default()
    };
    std::fs::write(&source, imported.knxproj_bytes()).unwrap();
    let state = state_with_products(dir.path(), &dir.path().join("products.sqlite"));
    let app = knx_server::app(state.clone(), None);
    ok(&app, "/api/project/import", Some(json!({"path":source}))).await;

    let opaque = state.opaque.lock().unwrap().clone();
    install_catalogue_fixture(&app).await;
    ok(
        &app,
        "/api/devices",
        Some(json!({"catalogItemId":CATALOG_ITEM,"name":"New catalogue device"})),
    )
    .await;
    let model = state.project.lock().unwrap().clone();
    let native = dir.path().join("mixed.knxdb");
    ok(&app, "/api/project/save-as", Some(json!({"path":native}))).await;
    ok(&app, "/api/project/save", Some(json!({}))).await;
    assert_eq!(carried_rows(&native), 5);
    assert_eq!(*state.project.lock().unwrap(), model);
    assert_eq!(*state.opaque.lock().unwrap(), opaque);
    assert!(log_messages(&state).iter().any(|message| {
        message.contains("Unknown imported product")
            && message.contains("portability is not established")
    }));
    let fresh = state_with_products(dir.path(), &dir.path().join("fresh.sqlite"));
    let reopened = knx_server::app(fresh.clone(), None);
    ok(&reopened, "/api/project/open", Some(json!({"path":native}))).await;
    ok(&reopened, "/api/project/save", Some(json!({}))).await;
    assert_eq!(*fresh.project.lock().unwrap(), model);
    assert_eq!(*fresh.opaque.lock().unwrap(), opaque);
    assert_eq!(carried_rows(&native), 5);
}

#[tokio::test]
async fn a_resolvable_import_still_requires_recorded_package_ownership_for_new_acquisition() {
    use std::sync::atomic::Ordering;
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("unresolved.knxproj");
    std::fs::write(
        &source,
        knx_testsupport::PluginFixture::default().knxproj_bytes(),
    )
    .unwrap();
    let products = dir.path().join("products.sqlite");
    let state = state_with_products(dir.path(), &products);
    let app = knx_server::app(state.clone(), None);
    ok(&app, "/api/project/import", Some(json!({"path":source}))).await;
    let native = dir.path().join("baseline.knxdb");
    ok(&app, "/api/project/save-as", Some(json!({"path":native}))).await;
    assert!(state.manufacturer_refs.lock().unwrap().is_empty());
    install_catalogue_fixture(&app).await;
    let conn = rusqlite::Connection::open(&products).unwrap();
    conn.execute("DELETE FROM package_member", []).unwrap();
    drop(conn);
    let model = state.project.lock().unwrap().clone();
    let opaque = state.opaque.lock().unwrap().clone();
    let clean = state.clean_project.lock().unwrap().clone();
    let generation = *state.history_generation.lock().unwrap();
    let revision = state.project_revision.load(Ordering::SeqCst);
    let saved_at = state.last_saved_at.lock().unwrap().clone();
    let history = format!("{:?}", state.command_stack.lock().unwrap());
    let bytes = std::fs::read(&native).unwrap();
    let copy = dir.path().join("refused.knxdb");
    for (route, data) in [
        ("/api/project/save", json!({})),
        ("/api/project/save-as", json!({"path":copy})),
    ] {
        let (status, body) = response(&app, route, Some(data)).await;
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
        assert!(body
            .to_string()
            .contains("package ownership is unavailable or ambiguous"));
        assert_eq!(std::fs::read(&native).unwrap(), bytes);
        assert!(!copy.exists());
        assert_eq!(*state.project.lock().unwrap(), model);
        assert_eq!(*state.opaque.lock().unwrap(), opaque);
        assert_eq!(*state.clean_project.lock().unwrap(), clean);
        assert!(state.manufacturer_refs.lock().unwrap().is_empty());
        assert_eq!(*state.history_generation.lock().unwrap(), generation);
        assert_eq!(state.project_revision.load(Ordering::SeqCst), revision);
        assert_eq!(*state.last_saved_at.lock().unwrap(), saved_at);
        assert_eq!(
            format!("{:?}", state.command_stack.lock().unwrap()),
            history
        );
        assert_eq!(
            state.store_path.lock().unwrap().as_deref(),
            Some(native.as_path())
        );
    }
}

#[tokio::test]
async fn an_unbacked_authored_working_device_refuses_offline_save_and_save_as() {
    use std::sync::atomic::Ordering;
    for saved_other in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let state = state_with_products(dir.path(), &dir.path().join("products.sqlite"));
        let app = knx_server::app(state.clone(), None);
        ok(&app, "/api/project/new", Some(json!({}))).await;
        let native = dir.path().join("working.knxdb");
        ok(&app, "/api/project/save-as", Some(json!({"path":native}))).await;
        let item = if saved_other {
            install_catalogue_fixture(&app).await;
            ok(
                &app,
                "/api/devices",
                Some(json!({"catalogItemId":CATALOG_ITEM,"name":"Saved device"})),
            )
            .await;
            ok(&app, "/api/project/save", Some(json!({}))).await;
            let members = [CATALOG_MASTER, CATALOG_SOURCE, HARDWARE, PROGRAM].map(|bytes| {
                String::from_utf8(bytes.to_vec())
                    .unwrap()
                    .replace("M-0001", "M-0002")
            });
            let other = zip(&[
                ("knx_master.xml", members[0].as_bytes()),
                ("M-0002/Catalog.xml", members[1].as_bytes()),
                ("M-0002/Hardware.xml", members[2].as_bytes()),
                ("M-0002/M-0002_A-1.xml", members[3].as_bytes()),
            ]);
            install_catalogue_bytes(&app, &other).await;
            "M-0002_CI-1"
        } else {
            install_catalogue_fixture(&app).await;
            CATALOG_ITEM
        };
        ok(
            &app,
            "/api/devices",
            Some(json!({"catalogItemId":item,"name":"Unbacked working device"})),
        )
        .await;
        let offline = Arc::new(knx_server::AppState::new(dir.path().into()));
        let offline_app = knx_server::app(offline.clone(), None);
        ok(
            &offline_app,
            "/api/project/open",
            Some(json!({"path":native})),
        )
        .await;
        assert_eq!(
            *offline.project.lock().unwrap(),
            *state.project.lock().unwrap()
        );
        let refs = offline.manufacturer_refs.lock().unwrap().clone();
        assert_eq!(refs.len(), if saved_other { 5 } else { 0 });
        let model = offline.project.lock().unwrap().clone();
        let opaque = offline.opaque.lock().unwrap().clone();
        let clean = offline.clean_project.lock().unwrap().clone();
        let generation = *offline.history_generation.lock().unwrap();
        let revision = offline.project_revision.load(Ordering::SeqCst);
        let saved_at = offline.last_saved_at.lock().unwrap().clone();
        let history = format!("{:?}", offline.command_stack.lock().unwrap());
        let bytes = std::fs::read(&native).unwrap();
        let copy = dir.path().join("refused.knxdb");
        for (route, data) in [
            ("/api/project/save", json!({})),
            ("/api/project/save-as", json!({"path":copy})),
        ] {
            let (status, body) = response(&offline_app, route, Some(data)).await;
            assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
            assert!(body
                .to_string()
                .contains("required sources are unavailable"));
            assert_eq!(std::fs::read(&native).unwrap(), bytes);
            assert!(!copy.exists());
            assert_eq!(*offline.project.lock().unwrap(), model);
            assert_eq!(*offline.opaque.lock().unwrap(), opaque);
            assert_eq!(*offline.clean_project.lock().unwrap(), clean);
            assert_eq!(*offline.manufacturer_refs.lock().unwrap(), refs);
            assert_eq!(*offline.history_generation.lock().unwrap(), generation);
            assert_eq!(offline.project_revision.load(Ordering::SeqCst), revision);
            assert_eq!(*offline.last_saved_at.lock().unwrap(), saved_at);
            assert_eq!(
                format!("{:?}", offline.command_stack.lock().unwrap()),
                history
            );
            assert_eq!(
                offline.store_path.lock().unwrap().as_deref(),
                Some(native.as_path())
            );
        }
    }
}

#[tokio::test]
async fn restore_is_behind_the_login_guard() {
    let dir = tempfile::tempdir().unwrap();
    let hash =
        knx_server::hash_password_with_iterations("fictional restore guard fixture", 1000).unwrap();
    let config = knx_server::AuthConfig::from_password_hash(&hash).unwrap();
    let app = knx_server::app_with_auth(
        Arc::new(knx_server::AppState::new(dir.path().into())),
        None,
        config,
    );
    let (status, _) = response(
        &app,
        "/api/project/manufacturer-files/restore",
        Some(json!({ "path": "anything.knxproj" })),
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED, "restore bypassed login");
}
