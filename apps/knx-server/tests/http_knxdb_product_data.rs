//! A saved `.knxdb` carries its application programs (GitHub #21, ADR-0128).
//!
//! Every test imports a fictional `.knxproj` whose one device runs a real
//! (tiny) application program, saves it, and reopens the file in a fresh
//! server whose product database has never seen that program: the Docker
//! rebuild and the copied-to-another-computer cases from the issue.

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use serde_json::{json, Value};
use std::{
    io::{Cursor, Write},
    path::Path,
    sync::{Arc, Mutex},
};
use tower::ServiceExt;

const PROJECT_INFO: &[u8] = br#"<KNX xmlns="http://knx.org/xml/project/11"><Project Id="P-0001"><ProjectInformation Name="Fictional dimmer" GroupAddressStyle="ThreeLevel" CompletionStatus="Undefined" /></Project></KNX>"#;

const TOPOLOGY: &[u8] = br#"<KNX xmlns="http://knx.org/xml/project/11"><Project Id="P-0001"><Installations><Installation InstallationId="0" Name="Fictional" CompletionStatus="Undefined"><Topology><Area Id="P-0001-0_A-1" Name="A" Address="0" CompletionStatus="Undefined"><Line Id="P-0001-0_L-1" Name="L" Address="2" MediumTypeRefId="MT-0" CompletionStatus="Undefined"><DeviceInstance Id="P-0001-0_DI-1" Name="Dimmaktor 4-fach" ProductRefId="M-0001_H-1_P-1" Hardware2ProgramRefId="M-0001_H-1_HP-1" Address="1" CompletionStatus="Undefined" /></Line></Area></Topology><GroupAddresses><GroupRanges /></GroupAddresses></Installation></Installations></Project></KNX>"#;

const HARDWARE: &[u8] = br#"<KNX xmlns="http://knx.org/xml/project/11"><ManufacturerData><Manufacturer RefId="M-0001"><Hardware><Hardware Id="M-0001_H-1" Name="Fictional dimmer" SerialNumber="S" VersionNumber="1"><Products><Product Id="M-0001_H-1_P-1" Text="Fictional dimmer" OrderNumber="FD-4" /></Products><Hardware2Programs><Hardware2Program Id="M-0001_H-1_HP-1" MediumTypes="MT-0"><ApplicationProgramRef RefId="M-0001_A-1" /></Hardware2Program></Hardware2Programs></Hardware></Hardware></Manufacturer></ManufacturerData></KNX>"#;

const PROGRAM: &[u8] = br#"<KNX xmlns="http://knx.org/xml/project/11"><ManufacturerData><Manufacturer RefId="M-0001"><ApplicationPrograms><ApplicationProgram Id="M-0001_A-1" Name="Fictional dimmer program" ApplicationVersion="1" MaskVersion="MV-0701"><Static><ParameterTypes><ParameterType Id="M-0001_A-1_PT-Num" Name="num"><TypeNumber maxInclusive="255" minInclusive="0" SizeInBit="8" Type="unsignedInt" /></ParameterType></ParameterTypes><Parameters><Parameter Id="M-0001_A-1_P-1" Name="Delay" Text="Delay" ParameterType="M-0001_A-1_PT-Num" Access="ReadWrite" Value="5" /></Parameters><ParameterRefs><ParameterRef Id="M-0001_A-1_P-1_R-1" RefId="M-0001_A-1_P-1" DisplayOrder="1" Tag="1" /></ParameterRefs></Static><Dynamic><ParameterRefRef RefId="M-0001_A-1_P-1_R-1" /></Dynamic></ApplicationProgram></ApplicationPrograms></Manufacturer></ManufacturerData></KNX>"#;

const HARDWARE_PATH: &str = "M-0001/Hardware.xml";
const PROGRAM_PATH: &str = "M-0001/M-0001_A-1.xml";

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
        product_db: Some(Mutex::new(
            knx_productdb::open_and_migrate(products).unwrap(),
        )),
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
