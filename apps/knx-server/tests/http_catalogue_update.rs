//! Previewed, backed-up catalogue updates and restores through real HTTP (ADR-0118).

use std::io::{Cursor, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use axum::body::Body;
use axum::http::{Request, StatusCode};
use serde_json::{json, Value};
use tower::ServiceExt;
use zip::write::SimpleFileOptions;

const BOUNDARY: &str = "knx-catalogue-update-boundary";
const MASTER: &[u8] = br#"<KNX xmlns="http://knx.org/xml/project/20"><MasterData><Manufacturers><Manufacturer Id="M-0001" Name="Example"/></Manufacturers></MasterData></KNX>"#;

fn program(version: u8) -> String {
    format!(
        r#"<KNX xmlns="http://knx.org/xml/project/20"><ManufacturerData><Manufacturer RefId="M-0001"><ApplicationPrograms><ApplicationProgram Id="M-0001_A-0001-{version:02X}-0001" Name="Dimmer" ApplicationNumber="1" ApplicationVersion="{version}" ProgramType="ApplicationProgram" MaskVersion="MV-07B0" PeiType="0" LoadProcedureStyle="MergedProcedure" DefaultLanguage="en-US"><Static/></ApplicationProgram></ApplicationPrograms></Manufacturer></ManufacturerData></KNX>"#
    )
}

fn hardware(version: u8) -> String {
    format!(
        r#"<KNX xmlns="http://knx.org/xml/project/20"><ManufacturerData><Manufacturer RefId="M-0001"><Hardware><Hardware Id="M-0001_H-1" Name="Dimmer hardware"><Products><Product Id="M-0001_H-1_P-1" Text="Dimmer" OrderNumber="DIM-1"/></Products><Hardware2Programs><Hardware2Program Id="M-0001_H-1_HP-0001-{version:02X}-0001" MediumTypes="MT-0"><ApplicationProgramRef RefId="M-0001_A-0001-{version:02X}-0001"/></Hardware2Program></Hardware2Programs></Hardware></Hardware></Manufacturer></ManufacturerData></KNX>"#
    )
}

fn package(version: u8) -> Vec<u8> {
    let program = program(version);
    let hardware = hardware(version);
    let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for (name, bytes) in [
        ("knx_master.xml", MASTER),
        ("M-0001/M-0001_A-0001.xml", program.as_bytes()),
        ("M-0001/Hardware.xml", hardware.as_bytes()),
    ] {
        zip.start_file(name, SimpleFileOptions::default()).unwrap();
        zip.write_all(bytes).unwrap();
    }
    zip.finish().unwrap().into_inner()
}

struct Fixture {
    dir: tempfile::TempDir,
    state: Arc<knx_server::AppState>,
    app: axum::Router,
}

impl Fixture {
    fn db(&self) -> PathBuf {
        self.dir.path().join("products.sqlite")
    }
}

/// A file-backed product database with version 0x10 of the synthetic dimmer.
fn fixture() -> Fixture {
    let dir = tempfile::tempdir().unwrap();
    let products = knx_productdb::open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
    knx_productdb::install_package(&products, "dimmer-v16.knxprod", &package(0x10)).unwrap();
    let state = Arc::new(knx_server::AppState {
        product_db: Some(Mutex::new(products)),
        ..knx_server::AppState::new(dir.path().join("data"))
    });
    let app = knx_server::app(state.clone(), None);
    Fixture { dir, state, app }
}

fn upload(uri: &str, filename: &str, bytes: &[u8], fields: &[(&str, &str)]) -> Request<Body> {
    let mut body = Vec::new();
    for (name, value) in fields {
        body.extend_from_slice(
            format!(
                "--{BOUNDARY}\r\nContent-Disposition: form-data; name=\"{name}\"\r\n\r\n{value}\r\n"
            )
            .as_bytes(),
        );
    }
    body.extend_from_slice(
        format!(
            "--{BOUNDARY}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"{filename}\"\r\n\r\n"
        )
        .as_bytes(),
    );
    body.extend_from_slice(bytes);
    body.extend_from_slice(format!("\r\n--{BOUNDARY}--\r\n").as_bytes());
    Request::builder()
        .method("POST")
        .uri(uri)
        .header(
            "content-type",
            format!("multipart/form-data; boundary={BOUNDARY}"),
        )
        .body(Body::from(body))
        .unwrap()
}

fn json_request(method: &str, uri: &str, body: Value) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(uri)
        .header("content-type", "application/json")
        .body(if body.is_null() {
            Body::empty()
        } else {
            Body::from(body.to_string())
        })
        .unwrap()
}

async fn send(app: &axum::Router, request: Request<Body>) -> (StatusCode, Value) {
    let response = app.clone().oneshot(request).await.unwrap();
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

/// Program IDs as the server's own shared connection sees them.
fn server_program_ids(state: &knx_server::AppState) -> Vec<String> {
    let conn = state.product_db.as_ref().unwrap().lock().unwrap();
    let mut statement = conn
        .prepare("SELECT id FROM application_program ORDER BY id")
        .unwrap();
    let ids = statement
        .query_map([], |row| row.get(0))
        .unwrap()
        .collect::<Result<Vec<String>, _>>()
        .unwrap();
    ids
}

async fn preview(f: &Fixture, version: u8) -> Value {
    let (status, body) = send(
        &f.app,
        upload(
            "/api/catalog/update/preview",
            &format!("dimmer-v{version}.knxprod"),
            &package(version),
            &[],
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    body
}

async fn apply(f: &Fixture, version: u8) -> Value {
    let token = preview(f, version).await["preview"]["token"]
        .as_str()
        .unwrap()
        .to_string();
    let (status, body) = send(
        &f.app,
        upload(
            "/api/catalog/update/apply",
            &format!("dimmer-v{version}.knxprod"),
            &package(version),
            &[("token", &token)],
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    body
}

fn backup_names(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(dir)
        .unwrap()
        .filter_map(|entry| entry.ok()?.file_name().into_string().ok())
        .filter(|name| name.contains(".backup-"))
        .collect();
    names.sort();
    names
}

#[tokio::test]
async fn preview_reports_the_new_version_and_leaves_the_database_file_unchanged() {
    let f = fixture();
    let before = std::fs::read(f.db()).unwrap();
    let body = preview(&f, 0x11).await;
    assert_eq!(body["applied"], json!(false));
    let program = &body["preview"]["programs"][0];
    assert_eq!(program["id"], json!("M-0001_A-0001-11-0001"));
    assert_eq!(program["status"], json!("newVersion"));
    assert_eq!(
        program["installedVersions"][0]["id"],
        json!("M-0001_A-0001-10-0001")
    );
    assert_eq!(body["preview"]["sourceName"], json!("dimmer-v17.knxprod"));
    assert_eq!(body["preview"]["packageSha256"].as_str().unwrap().len(), 64);
    assert_eq!(std::fs::read(f.db()).unwrap(), before);
    assert!(backup_names(f.dir.path()).is_empty());
    assert_eq!(server_program_ids(&f.state), ["M-0001_A-0001-10-0001"]);
}

#[tokio::test]
async fn apply_needs_the_current_token_writes_a_backup_and_keeps_old_programs() {
    let f = fixture();
    let before = std::fs::read(f.db()).unwrap();
    let (status, body) = send(
        &f.app,
        upload(
            "/api/catalog/update/apply",
            "dimmer-v17.knxprod",
            &package(0x11),
            &[("token", "not-the-token")],
        ),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(body["kind"], json!("catalogueUpdateStale"));
    assert_eq!(std::fs::read(f.db()).unwrap(), before);
    assert!(backup_names(f.dir.path()).is_empty());

    let token = preview(&f, 0x11).await["preview"]["token"]
        .as_str()
        .unwrap()
        .to_string();
    let (status, body) = send(
        &f.app,
        upload(
            "/api/catalog/update/apply",
            "dimmer-v17.knxprod",
            &package(0x11),
            &[("token", &token)],
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["applied"], json!(true));
    let backup = body["backup"].as_str().unwrap();
    assert!(!backup.contains('/') && !backup.contains('\\'), "{backup}");
    assert_eq!(backup_names(f.dir.path()), [backup.to_string()]);
    assert_eq!(body["report"]["sha256"], body["preview"]["packageSha256"]);
    assert_eq!(
        server_program_ids(&f.state),
        ["M-0001_A-0001-10-0001", "M-0001_A-0001-11-0001"]
    );

    // The token is spent: the catalogue it was bound to is gone.
    let (status, body) = send(
        &f.app,
        upload(
            "/api/catalog/update/apply",
            "dimmer-v17.knxprod",
            &package(0x11),
            &[("token", &token)],
        ),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT, "{body}");
}

#[tokio::test]
async fn apply_without_a_token_is_refused() {
    let f = fixture();
    for fields in [&[][..], &[("token", "")][..]] {
        let (status, _) = send(
            &f.app,
            upload(
                "/api/catalog/update/apply",
                "dimmer-v17.knxprod",
                &package(0x11),
                fields,
            ),
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{fields:?}");
    }
    assert!(backup_names(f.dir.path()).is_empty());
}

#[tokio::test]
async fn malformed_and_legacy_packages_are_refused_before_anything_is_written() {
    let f = fixture();
    let before = std::fs::read(f.db()).unwrap();
    let (status, _) = send(
        &f.app,
        upload(
            "/api/catalog/update/preview",
            "broken.knxprod",
            b"not a zip",
            &[],
        ),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let (status, body) = send(
        &f.app,
        upload("/api/catalog/update/preview", "old.vd4", b"not a zip", &[]),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{body}");
    assert_eq!(body["kind"], json!("legacyProductDatabase"));
    assert_eq!(std::fs::read(f.db()).unwrap(), before);
}

#[tokio::test]
async fn backups_list_only_backup_names_newest_first() {
    let f = fixture();
    let applied = apply(&f, 0x11).await;
    for stray in [
        "other.sqlite",
        "products.sqlite.backup-x.sqlite",
        "products.sqlite.backup-1-abcdefabcdef.sqlite.txt",
        "other.sqlite.backup-1-abcdefabcdef.sqlite",
        "products.sqlite.backup-1-ABCDEFABCDEF.sqlite",
        "products.sqlite.backup-1-abcdef.sqlite",
        "products.sqlite.backup-1-abcdefabcdef-x.sqlite",
    ] {
        std::fs::write(f.dir.path().join(stray), b"stray").unwrap();
    }
    // A link with a backup name is not a backup, whatever it points at.
    std::os::unix::fs::symlink(
        f.db(),
        f.dir
            .path()
            .join("products.sqlite.backup-9-abcdefabcdef.sqlite"),
    )
    .unwrap();
    // Created in an order unrelated to the time in their names.
    let older = "products.sqlite.backup-1-abcdefabcdef.sqlite";
    let named = [
        "products.sqlite.backup-5-abcdefabcdef.sqlite",
        older,
        "products.sqlite.backup-7-abcdefabcdef-1.sqlite",
        "products.sqlite.backup-3-abcdefabcdef.sqlite",
        "products.sqlite.backup-7-abcdefabcdef.sqlite",
    ];
    for name in named {
        std::fs::write(f.dir.path().join(name), b"older").unwrap();
    }
    let (status, body) = send(
        &f.app,
        json_request("GET", "/api/catalog/backups", Value::Null),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let names: Vec<&str> = body["backups"]
        .as_array()
        .unwrap()
        .iter()
        .map(|b| b["name"].as_str().unwrap())
        .collect();
    assert_eq!(
        names,
        [
            applied["backup"].as_str().unwrap(),
            "products.sqlite.backup-7-abcdefabcdef-1.sqlite",
            "products.sqlite.backup-7-abcdefabcdef.sqlite",
            "products.sqlite.backup-5-abcdefabcdef.sqlite",
            "products.sqlite.backup-3-abcdefabcdef.sqlite",
            older,
        ]
    );
    assert_eq!(body["backups"][5]["bytes"], json!(5));
    assert!(body["backups"][0]["modifiedUnixSeconds"].as_u64().unwrap() > 0);
    assert!(!body.to_string().contains(f.dir.path().to_str().unwrap()));
}

#[tokio::test]
async fn restore_by_listed_name_brings_the_old_catalogue_back_to_the_server_connection() {
    let f = fixture();
    let applied = apply(&f, 0x11).await;
    let name = applied["backup"].as_str().unwrap();
    // Connection-local state: it disappears only if the connection is replaced.
    f.state
        .product_db
        .as_ref()
        .unwrap()
        .lock()
        .unwrap()
        .execute_batch("CREATE TEMP TABLE old_connection(x)")
        .unwrap();
    let (status, body) = send(
        &f.app,
        json_request("POST", "/api/catalog/restore", json!({ "name": name })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["restored"], json!(name));
    assert_eq!(server_program_ids(&f.state), ["M-0001_A-0001-10-0001"]);
    let reopened: i64 = f
        .state
        .product_db
        .as_ref()
        .unwrap()
        .lock()
        .unwrap()
        .query_row(
            "SELECT count(*) FROM temp.sqlite_master WHERE name = 'old_connection'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(
        reopened, 0,
        "the server must reopen (and migrate) its connection after a restore"
    );
    // The connection still works for an ordinary install afterwards.
    apply(&f, 0x12).await;
    assert_eq!(
        server_program_ids(&f.state),
        ["M-0001_A-0001-10-0001", "M-0001_A-0001-12-0001"]
    );
}

#[tokio::test]
async fn restore_refuses_unlisted_names_and_keeps_the_live_file() {
    let f = fixture();
    apply(&f, 0x11).await;
    std::fs::write(f.dir.path().join("elsewhere.sqlite"), b"x").unwrap();
    let before = std::fs::read(f.db()).unwrap();
    for name in [
        "../products.sqlite",
        "products.sqlite",
        "elsewhere.sqlite",
        "sub/products.sqlite.backup-1-abcdefabcdef.sqlite",
        "products.sqlite.backup-2-abcdefabcdef.sqlite",
        "",
    ] {
        let (status, body) = send(
            &f.app,
            json_request("POST", "/api/catalog/restore", json!({ "name": name })),
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{name}: {body}");
        assert_eq!(body["kind"], json!("unknownBackup"), "{name}");
    }
    assert_eq!(std::fs::read(f.db()).unwrap(), before);
}

#[tokio::test]
async fn an_unusable_backup_is_refused_and_the_live_catalogue_stays() {
    let f = fixture();
    apply(&f, 0x11).await;
    let broken = "products.sqlite.backup-1-abcdefabcdef.sqlite";
    std::fs::write(f.dir.path().join(broken), b"definitely not sqlite").unwrap();
    let (status, body) = send(
        &f.app,
        json_request("POST", "/api/catalog/restore", json!({ "name": broken })),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{body}");
    assert_eq!(body["kind"], json!("invalidBackup"));
    assert_eq!(
        server_program_ids(&f.state),
        ["M-0001_A-0001-10-0001", "M-0001_A-0001-11-0001"]
    );
}

#[tokio::test]
async fn without_a_product_database_every_route_says_so() {
    let dir = tempfile::tempdir().unwrap();
    let app = knx_server::app(Arc::new(knx_server::AppState::new(dir.path().into())), None);
    for request in [
        upload(
            "/api/catalog/update/preview",
            "dimmer.knxprod",
            &package(0x11),
            &[],
        ),
        upload(
            "/api/catalog/update/apply",
            "dimmer.knxprod",
            &package(0x11),
            &[("token", "t")],
        ),
        json_request("GET", "/api/catalog/backups", Value::Null),
        json_request("POST", "/api/catalog/restore", json!({ "name": "x" })),
    ] {
        assert_eq!(send(&app, request).await.0, StatusCode::SERVICE_UNAVAILABLE);
    }
}

#[tokio::test]
async fn catalogue_update_routes_remain_inside_existing_authentication() {
    let f = fixture();
    let auth = knx_server::AuthConfig::from_plaintext_password("synthetic-test-only").unwrap();
    let app = knx_server::app_with_auth(f.state.clone(), None, auth);
    for request in [
        upload(
            "/api/catalog/update/preview",
            "dimmer.knxprod",
            &package(0x11),
            &[],
        ),
        upload(
            "/api/catalog/update/apply",
            "dimmer.knxprod",
            &package(0x11),
            &[("token", "t")],
        ),
        json_request("GET", "/api/catalog/backups", Value::Null),
        json_request("POST", "/api/catalog/restore", json!({ "name": "x" })),
    ] {
        assert_eq!(send(&app, request).await.0, StatusCode::UNAUTHORIZED);
    }
}
