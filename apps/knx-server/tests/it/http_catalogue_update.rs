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

/// The session log's lines that name why the product database is not open.
async fn product_database_lines(app: &axum::Router) -> Vec<Value> {
    let (_, log) = send(app, json_request("GET", "/api/log", Value::Null)).await;
    log.as_array()
        .unwrap()
        .iter()
        .filter(|entry| entry["source"] == json!("product database"))
        .cloned()
        .collect()
}

/// Every refusal of a route that needs products, as `(route, status, body)`:
/// listing, preview, install, readiness, download plan and, with a project
/// open, the manufacturer-file restore.
async fn product_refusals(
    app: &axum::Router,
    project_name: &str,
) -> Vec<(String, StatusCode, Value)> {
    let mut refusals = Vec::new();
    for uri in [
        "/api/catalog/manufacturers",
        "/api/catalog/items",
        "/api/device-readiness",
    ] {
        let (status, body) = send(app, json_request("GET", uri, Value::Null)).await;
        refusals.push((uri.to_string(), status, body));
    }
    for uri in ["/api/catalog/update/preview", "/api/catalog/install"] {
        let (status, body) = send(app, upload(uri, "dimmer.knxprod", &package(0x10), &[])).await;
        refusals.push((uri.to_string(), status, body));
    }
    let (status, body) = send(
        app,
        json_request(
            "POST",
            "/api/device-download/plan",
            json!({ "address": "1.1.1" }),
        ),
    )
    .await;
    refusals.push(("/api/device-download/plan".to_string(), status, body));
    let (status, body) = send(
        app,
        json_request(
            "POST",
            "/api/project/manufacturer-files/restore",
            json!({ "path": project_name }),
        ),
    )
    .await;
    refusals.push((
        "/api/project/manufacturer-files/restore".to_string(),
        status,
        body,
    ));
    refusals
}

/// The status, kind and wording each refusal had before the reason existed.
const PLAIN_REFUSALS: [(&str, StatusCode, Option<&str>, &str); 7] = [
    (
        "/api/catalog/manufacturers",
        StatusCode::BAD_REQUEST,
        None,
        "no product database configured",
    ),
    (
        "/api/catalog/items",
        StatusCode::BAD_REQUEST,
        None,
        "no product database configured",
    ),
    (
        "/api/device-readiness",
        StatusCode::CONFLICT,
        Some("downloadProductDataMissing"),
        "no product database is configured, so no application program can be read",
    ),
    (
        "/api/catalog/update/preview",
        StatusCode::SERVICE_UNAVAILABLE,
        None,
        "no product database configured",
    ),
    (
        "/api/catalog/install",
        StatusCode::INTERNAL_SERVER_ERROR,
        None,
        "no product database configured",
    ),
    (
        "/api/device-download/plan",
        StatusCode::CONFLICT,
        Some("downloadProductDataMissing"),
        "no product database is configured, so no application program can be read",
    ),
    (
        "/api/project/manufacturer-files/restore",
        StatusCode::UNPROCESSABLE_ENTITY,
        Some("noProductDatabase"),
        "this server has no product database to install manufacturer files into",
    ),
];

/// Imports the minimal project into `data` and returns its file name.
async fn import_minimal_project(app: &axum::Router, data: &Path) -> String {
    std::fs::create_dir_all(data).unwrap();
    let project = knx_testsupport::write_minimal_knxproj(data);
    let name = project.file_name().unwrap().to_str().unwrap().to_string();
    let (status, body) = send(
        app,
        json_request(
            "POST",
            "/api/project/import",
            json!({ "path": name, "discardChanges": true }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    name
}

/// A product database a newer KNXBench already migrated (a newer build, or
/// a test run that reached the user's real file) used to vanish silently:
/// the server ran without one and every refusal said only "no product
/// database configured". Now every refusal keeps its status and kind and
/// names the file and both versions, the log line survives each reset
/// (new, import, native open), and the file keeps its bytes.
#[tokio::test]
async fn a_newer_product_database_is_named_in_every_refusal_and_left_untouched() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("products.sqlite");
    let newer = knx_productdb::CURRENT_PRODUCTDB_VERSION + 1;
    let conn = knx_productdb::open_and_migrate(&path).unwrap();
    conn.pragma_update(None, "user_version", newer).unwrap();
    drop(conn);
    let before = std::fs::read(&path).unwrap();

    let data = dir.path().join("data");
    let state = Arc::new(knx_server::AppState::with_product_db_at(
        data.clone(),
        Some(path.clone()),
    ));
    assert!(state.product_db.is_none());
    let app = knx_server::app(state.clone(), None);
    let names_the_cause = |message: &str| {
        message.contains(&path.display().to_string())
            && message.contains(&format!("product database is version {newer}"))
            && message.contains(&format!(
                "this build supports up to {}",
                knx_productdb::CURRENT_PRODUCTDB_VERSION
            ))
    };
    let assert_logged = |lines: Vec<Value>, after: &str| {
        assert_eq!(lines.len(), 1, "after {after}: {lines:?}");
        assert_eq!(lines[0]["severity"], json!("error"), "after {after}");
        assert!(
            names_the_cause(lines[0]["message"].as_str().unwrap()),
            "after {after}: {lines:?}"
        );
    };
    assert_logged(product_database_lines(&app).await, "startup");

    let (status, _) = send(&app, json_request("POST", "/api/project/new", json!({}))).await;
    assert_eq!(status, StatusCode::OK);
    assert_logged(product_database_lines(&app).await, "new project");

    let project_name = import_minimal_project(&app, &data).await;
    assert_logged(product_database_lines(&app).await, "ETS import");

    for ((route, status, body), (plain_route, plain_status, kind, plain)) in
        product_refusals(&app, &project_name)
            .await
            .into_iter()
            .zip(PLAIN_REFUSALS)
    {
        assert_eq!(route, plain_route);
        assert_eq!(status, plain_status, "{route}: {body}");
        assert_eq!(body["kind"].as_str(), kind, "{route}: {body}");
        let message = body["error"].as_str().unwrap();
        assert!(names_the_cause(message), "{route}: {body}");
        assert_ne!(message, plain, "{route}");
    }

    let (status, body) = send(
        &app,
        json_request(
            "POST",
            "/api/project/save-as",
            json!({ "path": "saved.knxdb" }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let (status, body) = send(
        &app,
        json_request(
            "POST",
            "/api/project/open",
            json!({ "path": "saved.knxdb", "discardChanges": true }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_logged(product_database_lines(&app).await, "native open");

    assert_eq!(std::fs::read(&path).unwrap(), before);
}

/// Without a startup reason (no product database was ever asked for) every
/// refusal keeps its old status, kind and exact wording, and the log has no
/// product-database line.
#[tokio::test]
async fn without_a_startup_reason_the_refusals_keep_their_plain_wording() {
    let dir = tempfile::tempdir().unwrap();
    let data = dir.path().join("data");
    let app = knx_server::app(Arc::new(knx_server::AppState::new(data.clone())), None);
    let project_name = import_minimal_project(&app, &data).await;

    for ((route, status, body), (plain_route, plain_status, kind, plain)) in
        product_refusals(&app, &project_name)
            .await
            .into_iter()
            .zip(PLAIN_REFUSALS)
    {
        assert_eq!(route, plain_route);
        assert_eq!(status, plain_status, "{route}: {body}");
        assert_eq!(body["kind"].as_str(), kind, "{route}: {body}");
        assert_eq!(body["error"], json!(plain), "{route}");
    }
    assert!(product_database_lines(&app).await.is_empty());
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
    assert_eq!(body["packages"].as_array().unwrap().len(), 1, "{body}");
    assert_eq!(body["packages"][0]["path"], json!(null));
    assert_eq!(
        body["packages"][0]["report"]["sha256"],
        body["preview"]["packageSha256"]
    );
    assert_eq!(body["preview"]["wrapper"], json!(null));
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

/// The listing's entry for `name`, as the web shows it before a deletion.
async fn listed(f: &Fixture, name: &str) -> Value {
    let (status, body) = send(
        &f.app,
        json_request("GET", "/api/catalog/backups", Value::Null),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    body["backups"]
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| entry["name"] == json!(name))
        .cloned()
        .unwrap_or(Value::Null)
}

fn delete(name: &str, identity: &Value, confirmed: bool) -> Request<Body> {
    json_request(
        "POST",
        "/api/catalog/backups/delete",
        json!({ "name": name, "identity": identity, "confirmed": confirmed }),
    )
}

#[tokio::test]
async fn a_listed_backup_is_deleted_only_when_confirmed_with_the_identity_it_was_listed_with() {
    let f = fixture();
    let name = apply(&f, 0x11).await["backup"].as_str().unwrap().to_owned();
    let live = std::fs::read(f.db()).unwrap();
    let entry = listed(&f, &name).await;
    let identity = entry["identity"].clone();
    assert!(
        identity.as_str().is_some_and(|id| id.len() >= 16),
        "{entry}"
    );
    assert!(!entry.to_string().contains(f.dir.path().to_str().unwrap()));

    let (status, body) = send(&f.app, delete(&name, &identity, false)).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
    assert_eq!(body["kind"], json!("confirmationRequired"));
    assert!(f.dir.path().join(&name).exists(), "unconfirmed: kept");

    let (status, body) = send(&f.app, delete(&name, &identity, true)).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["deleted"], json!(name));
    assert!(!f.dir.path().join(&name).exists());
    assert_eq!(listed(&f, &name).await, Value::Null);
    // Only the backup went: the live catalogue and its connection are untouched.
    assert_eq!(std::fs::read(f.db()).unwrap(), live);
    assert_eq!(
        server_program_ids(&f.state),
        ["M-0001_A-0001-10-0001", "M-0001_A-0001-11-0001"]
    );
    // A second request finds nothing to delete.
    let (status, body) = send(&f.app, delete(&name, &identity, true)).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
    assert_eq!(body["kind"], json!("unknownBackup"));
}

#[tokio::test]
async fn deletion_refuses_names_it_did_not_list_and_never_follows_a_link() {
    let f = fixture();
    let real = apply(&f, 0x11).await["backup"].as_str().unwrap().to_owned();
    let identity = listed(&f, &real).await["identity"].clone();
    std::fs::write(f.dir.path().join("elsewhere.sqlite"), b"x").unwrap();
    let link = "products.sqlite.backup-9-abcdefabcdef.sqlite";
    std::os::unix::fs::symlink(f.db(), f.dir.path().join(link)).unwrap();
    let live = std::fs::read(f.db()).unwrap();
    for name in [
        "../products.sqlite",
        "products.sqlite",
        "elsewhere.sqlite",
        "sub/products.sqlite.backup-1-abcdefabcdef.sqlite",
        link,
        "",
    ] {
        let (status, body) = send(&f.app, delete(name, &identity, true)).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{name}: {body}");
        assert_eq!(body["kind"], json!("unknownBackup"), "{name}");
    }
    assert_eq!(std::fs::read(f.db()).unwrap(), live);
    assert!(f.dir.path().join("elsewhere.sqlite").exists());
    assert!(std::fs::symlink_metadata(f.dir.path().join(link)).is_ok());
    assert!(f.dir.path().join(&real).exists());
}

#[tokio::test]
async fn a_backup_that_changed_since_it_was_listed_is_kept() {
    let f = fixture();
    let first = apply(&f, 0x11).await["backup"].as_str().unwrap().to_owned();
    let shown = listed(&f, &first).await["identity"].clone();
    // Same name, size, bytes and modification time; only the file is another.
    let path = f.dir.path().join(&first);
    let bytes = std::fs::read(&path).unwrap();
    let modified = std::fs::metadata(&path).unwrap().modified().unwrap();
    std::fs::remove_file(&path).unwrap();
    std::fs::write(&path, &bytes).unwrap();
    std::fs::File::options()
        .write(true)
        .open(&path)
        .unwrap()
        .set_modified(modified)
        .unwrap();
    let (status, body) = send(&f.app, delete(&first, &shown, true)).await;
    assert_eq!(status, StatusCode::CONFLICT, "{body}");
    assert_eq!(body["kind"], json!("catalogueBackupChanged"));
    assert!(path.exists());
    // Another backup's identity does not unlock this one either.
    let second = apply(&f, 0x12).await["backup"].as_str().unwrap().to_owned();
    let other = listed(&f, &second).await["identity"].clone();
    let (status, body) = send(&f.app, delete(&first, &other, true)).await;
    assert_eq!(status, StatusCode::CONFLICT, "{body}");
    assert!(path.exists() && f.dir.path().join(&second).exists());
}

#[tokio::test]
async fn a_backup_that_cannot_be_removed_is_reported_and_stays_listed() {
    use std::os::unix::fs::PermissionsExt;
    let f = fixture();
    let name = apply(&f, 0x11).await["backup"].as_str().unwrap().to_owned();
    let identity = listed(&f, &name).await["identity"].clone();
    let dir = f.dir.path();
    std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o555)).unwrap();
    // A privileged runner (root, or a mapped user namespace) ignores the mode;
    // then this failure cannot be provoked and the test says so instead of passing.
    let privileged = std::fs::write(dir.join("probe"), b"x").is_ok();
    let result = send(&f.app, delete(&name, &identity, true)).await;
    std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o755)).unwrap();
    if privileged {
        eprintln!(
            "NOT RUN: the runner can write to a 0555 directory; removal failure untested here"
        );
        return;
    }
    let (status, body) = result;
    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR, "{body}");
    assert!(
        body["error"]
            .as_str()
            .unwrap_or_default()
            .contains("could not be deleted"),
        "{body}"
    );
    assert!(dir.join(&name).exists());
    assert_eq!(listed(&f, &name).await["identity"], identity);
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
        delete("x", &json!("y"), true),
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
        delete("x", &json!("y"), true),
    ] {
        assert_eq!(send(&app, request).await.0, StatusCode::UNAUTHORIZED);
    }
}

fn wrapper(members: &[(&str, &[u8])]) -> Vec<u8> {
    let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for (name, bytes) in members {
        zip.start_file(*name, SimpleFileOptions::default()).unwrap();
        zip.write_all(bytes).unwrap();
    }
    zip.finish().unwrap().into_inner()
}

#[tokio::test]
async fn a_vendor_zip_is_previewed_and_installed_package_by_package() {
    let f = fixture();
    let bytes = wrapper(&[
        ("Dimmer/dimmer-v17.knxprod", &package(0x11)),
        ("Dimmer/Manual.pdf", b"%PDF-1.4 manual"),
    ]);
    let before = std::fs::read(f.db()).unwrap();
    let (status, body) = send(
        &f.app,
        upload("/api/catalog/update/preview", "vendor.zip", &bytes, &[]),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(
        std::fs::read(f.db()).unwrap(),
        before,
        "a preview never writes"
    );
    let preview = &body["preview"];
    assert_eq!(
        preview["wrapper"]["packages"][0]["path"],
        json!("Dimmer/dimmer-v17.knxprod")
    );
    assert_eq!(
        preview["wrapper"]["packages"][0]["alreadyInstalled"],
        json!(false)
    );
    assert_eq!(
        preview["wrapper"]["otherMembers"][0]["path"],
        json!("Dimmer/Manual.pdf")
    );
    assert_eq!(preview["wrapper"]["otherMembers"][0]["size"], json!(15));
    assert_eq!(preview["programs"][0]["status"], json!("newVersion"));
    let token = preview["token"].as_str().unwrap().to_string();

    let (status, body) = send(
        &f.app,
        upload(
            "/api/catalog/update/apply",
            "vendor.zip",
            &bytes,
            &[("token", &token)],
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(
        body["packages"][0]["path"],
        json!("Dimmer/dimmer-v17.knxprod")
    );
    assert_eq!(
        body["packages"][0]["report"]["sha256"],
        body["preview"]["wrapper"]["packages"][0]["sha256"]
    );
    assert_eq!(
        server_program_ids(&f.state),
        ["M-0001_A-0001-10-0001", "M-0001_A-0001-11-0001"]
    );
}

#[tokio::test]
async fn a_refused_inner_package_names_itself_and_changes_nothing() {
    let f = fixture();
    let bytes = wrapper(&[
        ("good.knxprod", &package(0x11)),
        ("broken.knxprod", b"not a zip"),
    ]);
    let before = std::fs::read(f.db()).unwrap();
    let (status, body) = send(
        &f.app,
        upload("/api/catalog/update/preview", "vendor.zip", &bytes, &[]),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
    let message = body["error"].as_str().unwrap();
    assert!(
        message.contains("inner product package broken.knxprod"),
        "{message}"
    );
    assert_eq!(std::fs::read(f.db()).unwrap(), before);
}

// --- Catalogue placement cleanup -------------------------------------------

const PRODUCT: &str = "M-0001_H-1_P-1";
const BINDING: &str = "M-0001_H-1_HP-0001-10-0001";

/// Two placements of the same product + application pair.
fn add_twin_placements(f: &Fixture) {
    let conn = f.state.product_db.as_ref().unwrap().lock().unwrap();
    conn.execute_batch(&format!(
        "INSERT INTO catalog_item (id, manufacturer_id, section_id, name, product_ref_id,
             hardware2program_ref_id, source_sha256)
         SELECT 'CI-A', 'M-0001', 'SECTION', 'Dimmer', '{PRODUCT}', '{BINDING}', source_sha256
         FROM application_program;
         INSERT INTO catalog_item (id, manufacturer_id, section_id, name, product_ref_id,
             hardware2program_ref_id, source_sha256)
         SELECT 'CI-B', 'M-0001', 'SECTION', 'Dimmer', '{PRODUCT}', '{BINDING}', source_sha256
         FROM application_program;"
    ))
    .unwrap();
}

fn placement_ids(state: &knx_server::AppState) -> Vec<String> {
    let conn = state.product_db.as_ref().unwrap().lock().unwrap();
    let mut statement = conn
        .prepare("SELECT id FROM catalog_item ORDER BY id")
        .unwrap();
    let ids = statement
        .query_map([], |row| row.get(0))
        .unwrap()
        .collect::<Result<Vec<String>, _>>()
        .unwrap();
    ids
}

fn open_project_using_the_binding(f: &Fixture, devices: u32) {
    let mut project = knx_core::Project::new(knx_core::Language("en".into()));
    for id in 1..=devices {
        project.devices.insert(knx_core::DeviceInstance {
            id: knx_core::DeviceId(id),
            source: knx_core::SourceRef {
                path: "t".into(),
                ets_id: format!("d{id}"),
            },
            name: format!("Dimmer {id}"),
            description: None,
            address: None,
            product_ref: PRODUCT.into(),
            program_ref: BINDING.into(),
            commissioning: knx_core::CommissioningState::default(),
            visibility_calculated: true,
            com_objects: vec![],
            binary_data: vec![],
        });
    }
    *f.state.project.lock().unwrap() = Some(project);
}

async fn cleanup_preview(f: &Fixture, ids: &[&str]) -> (StatusCode, Value) {
    send(
        &f.app,
        json_request(
            "POST",
            "/api/catalog/cleanup/preview",
            json!({ "itemIds": ids }),
        ),
    )
    .await
}

#[tokio::test]
async fn catalogue_items_show_application_version_and_source() {
    let f = fixture();
    add_twin_placements(&f);
    let (status, body) = send(
        &f.app,
        json_request("GET", "/api/catalog/items", Value::Null),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let details = &body[0]["details"];
    assert_eq!(details["applicationProgramId"], "M-0001_A-0001-10-0001");
    assert_eq!(details["applicationVersion"], "16");
    assert_eq!(details["catalogSectionId"], "SECTION");
    assert_eq!(details["sourceNames"], json!(["dimmer-v16.knxprod"]));
}

#[tokio::test]
async fn cleanup_preview_counts_open_project_devices_and_last_placements() {
    let f = fixture();
    add_twin_placements(&f);
    open_project_using_the_binding(&f, 2);
    let before = placement_ids(&f.state);
    let (status, one) = cleanup_preview(&f, &["CI-A"]).await;
    assert_eq!(status, StatusCode::OK, "{one}");
    assert_eq!(one["items"][0]["id"], "CI-A");
    assert_eq!(one["items"][0]["impact"]["lastPlacementForBinding"], false);
    assert_eq!(one["items"][0]["impact"]["openProjectDevices"], 2);
    let (status, both) = cleanup_preview(&f, &["CI-B", "CI-A"]).await;
    assert_eq!(status, StatusCode::OK, "{both}");
    let ids: Vec<&str> = both["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|i| i["id"].as_str().unwrap())
        .collect();
    assert_eq!(ids, ["CI-A", "CI-B"]);
    assert_eq!(both["items"][0]["impact"]["lastPlacementForBinding"], true);
    assert_eq!(both["items"][1]["impact"]["lastPlacementForBinding"], true);
    assert_eq!(placement_ids(&f.state), before, "a preview writes nothing");
    assert!(backup_names(f.dir.path()).is_empty());
}

#[tokio::test]
async fn cleanup_apply_needs_confirmation_and_the_shown_preview_then_backs_up_first() {
    let f = fixture();
    add_twin_placements(&f);
    let (_, preview) = cleanup_preview(&f, &["CI-A"]).await;
    let token = preview["token"].as_str().unwrap().to_string();
    let apply = |body: Value| json_request("POST", "/api/catalog/cleanup/apply", body);

    let (status, body) = send(
        &f.app,
        apply(json!({ "itemIds": ["CI-A"], "token": token })),
    )
    .await;
    assert_eq!(
        (status, body["kind"].as_str()),
        (StatusCode::BAD_REQUEST, Some("confirmationRequired")),
        "{body}"
    );
    let (status, body) = send(
        &f.app,
        apply(json!({ "itemIds": ["CI-A"], "token": "stale", "confirmed": true })),
    )
    .await;
    assert_eq!(
        (status, body["kind"].as_str()),
        (StatusCode::CONFLICT, Some("catalogueCleanupStale")),
        "{body}"
    );
    let (status, body) = send(
        &f.app,
        apply(json!({ "itemIds": ["CI-B"], "token": token, "confirmed": true })),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::CONFLICT,
        "another selection never reuses a token: {body}"
    );
    assert_eq!(placement_ids(&f.state), ["CI-A", "CI-B"]);
    assert!(
        backup_names(f.dir.path()).is_empty(),
        "no backup for a refusal"
    );

    let (status, body) = send(
        &f.app,
        apply(json!({ "itemIds": ["CI-A"], "token": token, "confirmed": true })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["removedItemIds"], json!(["CI-A"]));
    assert_eq!(placement_ids(&f.state), ["CI-B"]);
    let backup = body["backup"].as_str().unwrap();
    assert!(!backup.contains('/'), "file name only");
    let (_, listed) = send(
        &f.app,
        json_request("GET", "/api/catalog/backups", Value::Null),
    )
    .await;
    assert_eq!(listed["backups"][0]["name"], backup);
    let (status, body) = send(
        &f.app,
        apply(json!({ "itemIds": ["CI-A"], "token": token, "confirmed": true })),
    )
    .await;
    assert_eq!(
        (status, body["kind"].as_str()),
        (StatusCode::CONFLICT, Some("catalogueCleanupStale")),
        "a spent selection means the catalogue changed since its preview: {body}"
    );

    // The backup restores the removed placement through the existing route.
    let (status, body) = send(
        &f.app,
        json_request("POST", "/api/catalog/restore", json!({ "name": backup })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(placement_ids(&f.state), ["CI-A", "CI-B"]);
}

#[tokio::test]
async fn cleanup_refuses_malformed_selections_and_a_missing_database() {
    let f = fixture();
    add_twin_placements(&f);
    for ids in [
        json!([]),
        json!(["CI-A", "CI-A"]),
        json!(["CI-A", "missing"]),
    ] {
        let (status, body) = send(
            &f.app,
            json_request(
                "POST",
                "/api/catalog/cleanup/preview",
                json!({ "itemIds": ids }),
            ),
        )
        .await;
        assert_eq!(
            (status, body["kind"].as_str()),
            (StatusCode::BAD_REQUEST, Some("invalidCatalogueSelection")),
            "{ids}: {body}"
        );
    }
    assert_eq!(placement_ids(&f.state), ["CI-A", "CI-B"]);
    let state = Arc::new(knx_server::AppState::new(f.dir.path().join("none")));
    let app = knx_server::app(state, None);
    let (status, _) = send(
        &app,
        json_request(
            "POST",
            "/api/catalog/cleanup/preview",
            json!({ "itemIds": ["CI-A"] }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
}

#[tokio::test]
async fn cleanup_refuses_unknown_request_fields_without_touching_anything() {
    let f = fixture();
    add_twin_placements(&f);
    let (_, preview) = cleanup_preview(&f, &["CI-A"]).await;
    let token = preview["token"].as_str().unwrap().to_string();
    let (status, _) = send(
        &f.app,
        json_request(
            "POST",
            "/api/catalog/cleanup/apply",
            json!({ "itemIds": ["CI-A"], "token": token, "confirmed": true, "force": true }),
        ),
    )
    .await;
    assert!(status.is_client_error(), "{status}");
    assert_eq!(placement_ids(&f.state), ["CI-A", "CI-B"]);
    assert!(backup_names(f.dir.path()).is_empty());
}

/// ABA: a restore brings back exactly the previewed contents, but through a
/// reopened connection. The old confirmation must not remove anything.
#[tokio::test]
async fn cleanup_preview_from_before_a_restore_is_stale_even_for_equal_contents() {
    let f = fixture();
    add_twin_placements(&f);
    // A backup of exactly this state: remove nothing real (a third placement)
    // and restore the backup taken right before.
    {
        let conn = f.state.product_db.as_ref().unwrap().lock().unwrap();
        conn.execute_batch(
            "INSERT INTO catalog_item (id, manufacturer_id, section_id, name, source_sha256)
             SELECT 'CI-TEMP', manufacturer_id, section_id, name, source_sha256
             FROM catalog_item WHERE id = 'CI-A';",
        )
        .unwrap();
    }
    let (_, temp) = cleanup_preview(&f, &["CI-TEMP"]).await;
    let (status, removed) = send(
        &f.app,
        json_request(
            "POST",
            "/api/catalog/cleanup/apply",
            json!({ "itemIds": ["CI-TEMP"], "token": temp["token"], "confirmed": true }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{removed}");
    // Preview on the state without CI-TEMP, then restore the backup (with it),
    // then remove CI-TEMP again so the contents equal the previewed state.
    let (_, preview) = cleanup_preview(&f, &["CI-A"]).await;
    let token = preview["token"].as_str().unwrap().to_string();
    let (status, body) = send(
        &f.app,
        json_request(
            "POST",
            "/api/catalog/restore",
            json!({ "name": removed["backup"] }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    {
        let conn = f.state.product_db.as_ref().unwrap().lock().unwrap();
        conn.execute_batch("DELETE FROM catalog_item WHERE id = 'CI-TEMP'")
            .unwrap();
    }
    assert_eq!(placement_ids(&f.state), ["CI-A", "CI-B"]);
    let (status, body) = send(
        &f.app,
        json_request(
            "POST",
            "/api/catalog/cleanup/apply",
            json!({ "itemIds": ["CI-A"], "token": token, "confirmed": true }),
        ),
    )
    .await;
    assert_eq!(
        (status, body["kind"].as_str()),
        (StatusCode::CONFLICT, Some("catalogueCleanupStale")),
        "{body}"
    );
    assert_eq!(placement_ids(&f.state), ["CI-A", "CI-B"]);
}
