//! Plug-in data adoption over HTTP: list, bound preview, token-only apply, cancel.
//!
//! Archives are the synthetic `PluginFixture` pair; their plug-in "bytes" are
//! arbitrary strings that nothing in KNXBench interprets.

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use knx_testsupport::{PluginFixture, PLUGIN_FIXTURE_BLOB_PATH, PLUGIN_FIXTURE_RBG_PATH};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tower::ServiceExt;

const ROLLBACK_LABEL: &str = "Before plug-in data adoption";

async fn send(app: &axum::Router, method: &str, path: &str, payload: Value) -> (StatusCode, Value) {
    let mut request = Request::builder().method(method).uri(path);
    let body = if payload.is_null() {
        Body::empty()
    } else {
        request = request.header("content-type", "application/json");
        Body::from(payload.to_string())
    };
    let response = app
        .clone()
        .oneshot(request.body(body).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), 16 * 1024 * 1024)
        .await
        .unwrap();
    let value = if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&bytes).unwrap_or_else(|_| json!(String::from_utf8_lossy(&bytes)))
    };
    (status, value)
}

async fn post(app: &axum::Router, path: &str, payload: Value) -> (StatusCode, Value) {
    send(app, "POST", path, payload).await
}

struct Fixture {
    _dir: tempfile::TempDir,
    source: PathBuf,
    store: PathBuf,
    state: Arc<knx_server::AppState>,
    app: axum::Router,
}

fn write(dir: &Path, name: &str, fixture: &PluginFixture) -> PathBuf {
    let path = dir.join(name);
    std::fs::write(&path, fixture.knxproj_bytes()).unwrap();
    path
}

/// The default archive imported and saved as `target.knxdb`, and the edited
/// archive next to it as the source.
async fn fixture_with(source: &PluginFixture) -> Fixture {
    let dir = tempfile::tempdir().unwrap();
    let target = write(dir.path(), "target.knxproj", &PluginFixture::default());
    let source = write(dir.path(), "source.knxproj", source);
    let state = Arc::new(knx_server::AppState::new(dir.path().to_path_buf()));
    let app = knx_server::app(state.clone(), None);
    let (status, body) = post(&app, "/api/project/import", json!({"path": target})).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let store = dir.path().join("target.knxdb");
    let (status, body) = post(&app, "/api/project/save-as", json!({"path": store})).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    Fixture {
        _dir: dir,
        source,
        store,
        state,
        app,
    }
}

async fn fixture() -> Fixture {
    fixture_with(&PluginFixture::edited()).await
}

async fn tree(app: &axum::Router) -> Value {
    let (status, tree) = send(app, "GET", "/api/project", Value::Null).await;
    assert_eq!(status, StatusCode::OK);
    tree
}

fn binding(tree: &Value) -> Value {
    json!({
        "serverIncarnation": tree["server_incarnation"],
        "projectIncarnation": tree["project_incarnation"],
        "snapshotRevision": tree["snapshot_revision"],
    })
}

async fn candidates(f: &Fixture) -> Value {
    let (status, body) = post(
        &f.app,
        "/api/project/plugin-data/candidates",
        json!({"path": f.source}),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    body
}

/// Preview body for the first candidate under the current binding.
async fn preview_body(f: &Fixture) -> Value {
    let listing = candidates(f).await;
    let first = &listing["candidates"][0];
    let mut body = binding(&tree(&f.app).await);
    body["path"] = json!(f.source);
    body["sourceHash"] = listing["sourceHash"].clone();
    body["request"] = json!({
        "sourceDevice": first["sourceDevice"]["id"],
        "targetDevice": first["targetDevice"]["id"],
    });
    body
}

async fn preview(f: &Fixture) -> Value {
    let (status, body) = post(
        &f.app,
        "/api/project/plugin-data/preview",
        preview_body(f).await,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    body
}

fn opaque_bytes(state: &knx_server::AppState, path: &str) -> Vec<u8> {
    let opaque = state.opaque.lock().unwrap();
    let mut hits = opaque
        .iter()
        .filter(|e| e.xpath.is_empty() && e.source_path == path);
    let entry = hits.next().unwrap_or_else(|| panic!("{path} missing"));
    assert!(hits.next().is_none(), "{path} duplicated");
    entry.bytes.clone()
}

fn stored_working(store: &Path) -> knx_store::project_history::NativeSnapshot {
    let conn = knx_store::open_existing_read_only(store).unwrap().conn;
    knx_store::project_history::load_editor(&conn)
        .unwrap()
        .expect("a working journal exists")
        .working
}

/// Everything the user could lose or see change: model, opaque store, undo
/// and redo depth, published revision and the file's bytes.
struct Untouched {
    project: knx_core::Project,
    opaque: Vec<knx_store::StoredOpaqueEntry>,
    lengths: (usize, usize),
    revision: Value,
    file: Vec<u8>,
}

async fn untouched(f: &Fixture) -> Untouched {
    // One statement per lock: a guard temporary would otherwise live across
    // the `GET /api/project` below and deadlock on the same mutex.
    let project = f.state.project.lock().unwrap().clone().unwrap();
    let opaque = f.state.opaque.lock().unwrap().clone();
    let lengths = f.state.command_stack.lock().unwrap().history_lengths();
    let revision = tree(&f.app).await["snapshot_revision"].clone();
    Untouched {
        project,
        opaque,
        lengths,
        revision,
        file: std::fs::read(&f.store).unwrap(),
    }
}

async fn assert_untouched(f: &Fixture, before: &Untouched) {
    assert_eq!(
        f.state.project.lock().unwrap().as_ref(),
        Some(&before.project)
    );
    assert_eq!(*f.state.opaque.lock().unwrap(), before.opaque);
    assert_eq!(
        f.state.command_stack.lock().unwrap().history_lengths(),
        before.lengths
    );
    assert_eq!(tree(&f.app).await["snapshot_revision"], before.revision);
    assert_eq!(std::fs::read(&f.store).unwrap(), before.file);
}

#[tokio::test]
async fn listing_and_preview_name_every_entry_and_write_nothing() {
    let f = fixture().await;
    let before = untouched(&f).await;

    let listing = candidates(&f).await;
    assert_eq!(listing["persistence"], "native");
    assert!(listing["sourceHash"].as_str().unwrap().len() == 64);
    let found = listing["candidates"].as_array().unwrap();
    assert_eq!(found.len(), 1);
    assert_eq!(found[0]["refusal"], Value::Null);
    assert_eq!(found[0]["refusalDetail"], Value::Null);
    assert_eq!(found[0]["replacements"], 2);

    let shown = preview(&f).await;
    let token = shown["confirmationToken"].as_str().unwrap();
    assert_eq!(token.len(), 64);
    let replaced: Vec<_> = shown["preview"]["replacements"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| {
            (
                r["channel"].as_str().unwrap().to_string(),
                r["path"].as_str().unwrap().to_string(),
            )
        })
        .collect();
    assert_eq!(
        replaced,
        vec![
            ("BinaryData".into(), PLUGIN_FIXTURE_BLOB_PATH.to_string()),
            ("ExtraData".into(), PLUGIN_FIXTURE_RBG_PATH.to_string()),
        ]
    );
    for r in shown["preview"]["replacements"].as_array().unwrap() {
        assert_eq!(r["sha256Before"].as_str().unwrap().len(), 64);
        assert_ne!(r["sha256Before"], r["sha256After"]);
    }
    let text = shown.to_string();
    assert!(
        !text.contains("blob v1") && !text.contains("blob v2"),
        "the preview names bytes by digest, it never carries them"
    );
    assert_untouched(&f, &before).await;
}

#[tokio::test]
async fn confirmed_apply_replaces_the_bytes_without_an_undo_step_and_keeps_the_way_back() {
    let f = fixture().await;
    let lengths = f.state.command_stack.lock().unwrap().history_lengths();
    let project = f.state.project.lock().unwrap().clone().unwrap();
    let revision = tree(&f.app).await["snapshot_revision"].as_u64().unwrap();
    let shown = preview(&f).await;

    let (status, applied) = post(
        &f.app,
        "/api/project/plugin-data/apply",
        json!({"confirmationToken": shown["confirmationToken"]}),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{applied}");
    assert_eq!(applied["preview"], shown["preview"]);
    assert_eq!(applied["rollbackVersion"]["label"], ROLLBACK_LABEL);
    assert!(applied["project"]["snapshot_revision"].as_u64().unwrap() > revision);

    assert_eq!(
        opaque_bytes(&f.state, PLUGIN_FIXTURE_BLOB_PATH),
        PluginFixture::edited().blob
    );
    assert_eq!(
        opaque_bytes(&f.state, PLUGIN_FIXTURE_RBG_PATH),
        PluginFixture::edited().rbg
    );
    assert_eq!(
        f.state.project.lock().unwrap().as_ref(),
        Some(&project),
        "the model is not touched"
    );
    assert_eq!(
        f.state.command_stack.lock().unwrap().history_lengths(),
        lengths,
        "adoption is not an undo step"
    );
    assert_eq!(
        applied["project"]["can_undo"],
        tree(&f.app).await["can_undo"]
    );

    // Durable before published: the working journal already holds the bytes.
    let working = stored_working(&f.store);
    assert_eq!(working.opaque, *f.state.opaque.lock().unwrap());

    // The way back: the named version holds the state before the adoption.
    let (status, history) = send(&f.app, "GET", "/api/project/history", Value::Null).await;
    assert_eq!(status, StatusCode::OK);
    let version = history["versions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|v| v["label"] == ROLLBACK_LABEL)
        .expect("rollback version listed")
        .clone();
    assert_eq!(version["id"], applied["rollbackVersion"]["id"]);
    assert_eq!(version["reason"], "named");
    let (status, restored) = post(
        &f.app,
        &format!("/api/project/history/versions/{}/restore", version["id"]),
        json!({
            "serverIncarnation": history["serverIncarnation"],
            "snapshotRevision": history["snapshotRevision"],
            "generation": history["generation"],
            "confirmed": true,
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{restored}");
    assert_eq!(
        opaque_bytes(&f.state, PLUGIN_FIXTURE_BLOB_PATH),
        PluginFixture::default().blob
    );
    assert_eq!(
        opaque_bytes(&f.state, PLUGIN_FIXTURE_RBG_PATH),
        PluginFixture::default().rbg
    );
}

#[tokio::test]
async fn the_adoption_persists_through_save_and_reopen_and_without_save() {
    let f = fixture().await;
    let shown = preview(&f).await;
    let (status, _) = post(
        &f.app,
        "/api/project/plugin-data/apply",
        json!({"confirmationToken": shown["confirmationToken"]}),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let adopted = f.state.opaque.lock().unwrap().clone();
    let kinds = |opaque: &[knx_store::StoredOpaqueEntry], kind: &str| {
        opaque.iter().filter(|e| e.kind == kind).count()
    };
    assert_eq!(
        kinds(&adopted, knx_app::plugin_data::SUPERSEDED_KIND),
        2,
        "both superseded entries are kept"
    );
    assert_eq!(kinds(&adopted, knx_app::plugin_data::ADOPTION_KIND), 1);

    // Reopened unsaved: the working journal comes back with the adoption.
    let (status, body) = post(&f.app, "/api/project/open", json!({"path": f.store})).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(*f.state.opaque.lock().unwrap(), adopted);

    // Saved and reopened: the saved root holds it too.
    let (status, body) = post(&f.app, "/api/project/save", json!({})).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let (status, body) = post(&f.app, "/api/project/open", json!({"path": f.store})).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(*f.state.opaque.lock().unwrap(), adopted);
    let conn = knx_store::open_existing_read_only(&f.store).unwrap().conn;
    let mut root = knx_store::load_opaque(&conn).unwrap();
    let mut expected = adopted.clone();
    root.sort_by(|a, b| a.source_path.cmp(&b.source_path));
    expected.sort_by(|a, b| a.source_path.cmp(&b.source_path));
    assert_eq!(root, expected);
}

#[tokio::test]
async fn each_plan_refusal_keeps_its_code_and_changes_nothing() {
    for (source, code) in [
        (
            PluginFixture {
                group_address: 2,
                ..PluginFixture::edited()
            },
            "groupLinksDiffer",
        ),
        (
            PluginFixture {
                product: "M-0001_H-1_P-2",
                ..PluginFixture::edited()
            },
            "productReferenceDiffers",
        ),
        (
            PluginFixture {
                program: "M-0001_H-1_HP-2",
                ..PluginFixture::edited()
            },
            "applicationReferenceDiffers",
        ),
        (
            PluginFixture {
                blob_id: "2868e24a-9fc3-4099-82f0-3f535bde8fc1",
                ..PluginFixture::edited()
            },
            "binaryDataReferencesDiffer",
        ),
        (
            PluginFixture {
                extra_value: "393257",
                ..PluginFixture::edited()
            },
            "extraDataAssociationDiffers",
        ),
        (PluginFixture::default(), "alreadyIdentical"),
    ] {
        let f = fixture_with(&source).await;
        let before = untouched(&f).await;
        let listing = candidates(&f).await;
        assert_eq!(
            listing["candidates"][0]["refusalDetail"]["code"], code,
            "{listing}"
        );
        let (status, body) = post(
            &f.app,
            "/api/project/plugin-data/preview",
            preview_body(&f).await,
        )
        .await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{body}");
        assert_eq!(body["kind"], "pluginDataRefused");
        assert_eq!(body["refusal"]["code"], code);
        assert_eq!(body["error"], body["refusal"]["message"]);
        assert_untouched(&f, &before).await;
    }
}

#[tokio::test]
async fn an_unknown_device_is_refused_by_name() {
    let f = fixture().await;
    let mut body = preview_body(&f).await;
    body["request"]["targetDevice"] = json!(9999);
    let (status, refused) = post(&f.app, "/api/project/plugin-data/preview", body).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(refused["refusal"]["code"], "unknownDevice");
    assert_eq!(refused["refusal"]["side"], "destination");
    assert_eq!(refused["refusal"]["id"], 9999);
}

#[tokio::test]
async fn a_stale_binding_or_changed_source_is_a_conflict_at_preview() {
    let f = fixture().await;
    let before = untouched(&f).await;
    let good = preview_body(&f).await;

    for (field, value) in [
        (
            "snapshotRevision",
            json!(good["snapshotRevision"].as_u64().unwrap() + 1),
        ),
        (
            "projectIncarnation",
            json!(good["projectIncarnation"].as_u64().unwrap() + 1),
        ),
        ("serverIncarnation", json!("another-server-run")),
    ] {
        let mut body = good.clone();
        body[field] = value;
        let (status, refused) = post(&f.app, "/api/project/plugin-data/preview", body).await;
        assert_eq!(status, StatusCode::CONFLICT, "{field}");
        assert_eq!(refused["kind"], "pluginDataStale", "{field}");
    }

    let mut body = good.clone();
    body["sourceHash"] = json!("0".repeat(64));
    let (status, refused) = post(&f.app, "/api/project/plugin-data/preview", body).await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(refused["kind"], "pluginDataSourceChanged");

    let mut body = good.clone();
    body.as_object_mut().unwrap().remove("snapshotRevision");
    let (status, _) = post(&f.app, "/api/project/plugin-data/preview", body).await;
    assert_eq!(
        status,
        StatusCode::UNPROCESSABLE_ENTITY,
        "a partial binding is rejected, never downgraded"
    );
    let mut body = good.clone();
    body["confirmed"] = json!(true);
    let (status, _) = post(&f.app, "/api/project/plugin-data/preview", body).await;
    assert_eq!(
        status,
        StatusCode::UNPROCESSABLE_ENTITY,
        "unknown fields are rejected"
    );
    assert_untouched(&f, &before).await;
}

#[tokio::test]
async fn a_project_change_after_the_preview_makes_apply_stale_and_writes_nothing() {
    let f = fixture().await;
    let shown = preview(&f).await;
    // Another client renames the device: a new revision, one undo step.
    let current = tree(&f.app).await;
    let device = f
        .state
        .project
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .devices
        .iter()
        .next()
        .map(|d| (d.id.0, d.name.clone()))
        .unwrap();
    let mut rename = binding(&current);
    rename["name"] = json!("Renamed elsewhere");
    rename["expectedName"] = json!(device.1);
    let (status, body) = send(
        &f.app,
        "PATCH",
        &format!("/api/devices/{}/name", device.0),
        rename,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let before = untouched(&f).await;

    let apply = json!({"confirmationToken": shown["confirmationToken"]});
    let (status, refused) = post(&f.app, "/api/project/plugin-data/apply", apply.clone()).await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(refused["kind"], "pluginDataStale");
    assert_untouched(&f, &before).await;
    let (status, _) = post(&f.app, "/api/project/plugin-data/apply", apply).await;
    assert_eq!(
        status,
        StatusCode::CONFLICT,
        "a stale plan is dropped, never retried"
    );
    assert_untouched(&f, &before).await;
}

#[tokio::test]
async fn a_wrong_replaced_or_cancelled_token_is_stale() {
    let f = fixture().await;
    let before = untouched(&f).await;
    let first = preview(&f).await;

    let (status, refused) = post(
        &f.app,
        "/api/project/plugin-data/apply",
        json!({"confirmationToken": "f".repeat(64)}),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(refused["kind"], "pluginDataStale");

    // A newer preview replaces the shown plan; the same plan gives the same token.
    let second = preview(&f).await;
    assert_eq!(first["confirmationToken"], second["confirmationToken"]);

    let (status, cancelled) = post(
        &f.app,
        "/api/project/plugin-data/cancel",
        json!({"confirmationToken": "e".repeat(64)}),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        cancelled["cancelled"], false,
        "another token cancels nothing"
    );
    let (status, cancelled) = post(
        &f.app,
        "/api/project/plugin-data/cancel",
        json!({"confirmationToken": second["confirmationToken"]}),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(cancelled["cancelled"], true);
    let (status, refused) = post(
        &f.app,
        "/api/project/plugin-data/apply",
        json!({"confirmationToken": second["confirmationToken"]}),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(refused["kind"], "pluginDataStale");

    let (status, _) = post(&f.app, "/api/project/plugin-data/apply", json!({})).await;
    assert_eq!(
        status,
        StatusCode::UNPROCESSABLE_ENTITY,
        "apply needs the token"
    );
    assert_untouched(&f, &before).await;
}

#[tokio::test]
async fn opening_another_project_makes_a_shown_plan_stale() {
    let f = fixture().await;
    let shown = preview(&f).await;
    let (status, _) = post(&f.app, "/api/project/open", json!({"path": f.store})).await;
    assert_eq!(status, StatusCode::OK);
    let before = untouched(&f).await;
    let (status, refused) = post(
        &f.app,
        "/api/project/plugin-data/apply",
        json!({"confirmationToken": shown["confirmationToken"]}),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(refused["kind"], "pluginDataStale");
    assert_untouched(&f, &before).await;
}

#[tokio::test]
async fn a_session_only_project_can_list_but_not_adopt() {
    let dir = tempfile::tempdir().unwrap();
    let target = write(dir.path(), "target.knxproj", &PluginFixture::default());
    let source = write(dir.path(), "source.knxproj", &PluginFixture::edited());
    let state = Arc::new(knx_server::AppState::new(dir.path().to_path_buf()));
    let app = knx_server::app(state.clone(), None);
    assert_eq!(
        post(&app, "/api/project/import", json!({"path": target}))
            .await
            .0,
        StatusCode::OK
    );
    let opaque = state.opaque.lock().unwrap().clone();
    let (status, listing) = post(
        &app,
        "/api/project/plugin-data/candidates",
        json!({"path": source}),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(listing["persistence"], "session");
    let first = &listing["candidates"][0];
    let mut body = binding(&tree(&app).await);
    body["path"] = json!(source);
    body["sourceHash"] = listing["sourceHash"].clone();
    body["request"] = json!({
        "sourceDevice": first["sourceDevice"]["id"],
        "targetDevice": first["targetDevice"]["id"],
    });
    let (status, refused) = post(&app, "/api/project/plugin-data/preview", body).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(refused["kind"], "pluginDataNativeRequired");
    assert_eq!(*state.opaque.lock().unwrap(), opaque);
}

#[tokio::test]
async fn an_unreadable_source_or_a_missing_project_is_refused_by_kind() {
    let dir = tempfile::tempdir().unwrap();
    let source = write(dir.path(), "source.knxproj", &PluginFixture::edited());
    let state = Arc::new(knx_server::AppState::new(dir.path().to_path_buf()));
    let app = knx_server::app(state, None);
    let (status, refused) = post(
        &app,
        "/api/project/plugin-data/candidates",
        json!({"path": source}),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(refused["kind"], "pluginDataTarget");

    let f = fixture().await;
    let broken = f.source.with_file_name("broken.knxproj");
    std::fs::write(&broken, b"not a zip archive").unwrap();
    let (status, refused) = post(
        &f.app,
        "/api/project/plugin-data/candidates",
        json!({"path": broken}),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(refused["kind"], "pluginDataSource");
}

#[tokio::test]
async fn plugin_data_routes_remain_inside_existing_authentication() {
    let f = fixture().await;
    let auth = knx_server::AuthConfig::from_plaintext_password("synthetic-test-only").unwrap();
    let app = knx_server::app_with_auth(f.state.clone(), None, auth);
    let before = untouched(&f).await;
    for (path, body) in [
        ("candidates", json!({"path": f.source})),
        ("preview", json!({"path": f.source})),
        ("apply", json!({"confirmationToken": "a".repeat(64)})),
        ("cancel", json!({"confirmationToken": "a".repeat(64)})),
    ] {
        let (status, _) = post(&app, &format!("/api/project/plugin-data/{path}"), body).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED, "{path}");
    }
    assert_untouched(&f, &before).await;
}
