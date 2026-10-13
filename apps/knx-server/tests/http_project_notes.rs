//! Verifies confirmed note HTTP editing without losing source context or history.
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use knx_testsupport::zip_with_entries;
use serde_json::{json, Value};
use std::sync::Arc;
use tower::ServiceExt;
async fn request(
    app: &axum::Router,
    method: &str,
    path: &str,
    payload: Value,
) -> (StatusCode, Value) {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(method)
                .uri(path)
                .header("content-type", "application/json")
                .body(Body::from(payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), 4 * 1024 * 1024)
        .await
        .unwrap();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or_else(|_| json!({"nonJsonError":true})),
    )
}
async fn fixture(
    value: &str,
) -> (
    tempfile::TempDir,
    Arc<knx_server::AppState>,
    axum::Router,
    u32,
) {
    let value = value
        .replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;");
    let xml = format!(
        r#"<KNX xmlns="http://knx.org/xml/project/23"><Project Id="P-0001"><Installations><Installation InstallationId="0"><Topology><UnassignedDevices><DeviceInstance Id="P-0001-0_DI-1" Name="Synthetic" InstallationHints="{value}"/></UnassignedDevices></Topology></Installation></Installations></Project></KNX>"#
    );
    let info = r#"<KNX xmlns="http://knx.org/xml/project/23"><Project Id="P-0001"><ProjectInformation Name="Synthetic instructions"/></Project></KNX>"#;
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("synthetic.knxproj");
    std::fs::write(
        &path,
        zip_with_entries(&[
            ("P-0001.signature", b""),
            ("P-0001/0.xml", xml.as_bytes()),
            ("P-0001/Project.xml", info.as_bytes()),
        ]),
    )
    .unwrap();
    let state = Arc::new(knx_server::AppState::new(dir.path().to_path_buf()));
    let app = knx_server::app(state.clone(), None);
    let opened = request(
        &app,
        "POST",
        "/api/project/import",
        json!({"path":path,"clientToken":"synthetic-instruction-import"}),
    )
    .await;
    assert_eq!(
        opened.0,
        StatusCode::OK,
        "synthetic fixture setup: {}",
        opened.1
    );
    let id = state
        .project
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .devices
        .iter()
        .next()
        .unwrap()
        .id
        .0;
    (dir, state, app, id)
}

fn note_action(title: &str, position: usize) -> Value {
    json!({"kind":"create","position":position,"draft":{
        "target":{"kind":"project"},"title":title,
        "body":"Literal Grüße🙂\r\n<img src=\"https://untrusted.invalid/never\">",
        "includeInDocumentation":false
    }})
}
fn preview_payload(view: &Value, action: Value) -> Value {
    json!({"serverIncarnation":view["serverIncarnation"],
        "projectIncarnation":view["projectIncarnation"],
        "snapshotRevision":view["snapshotRevision"],"action":action})
}
#[tokio::test]
async fn notes_read_preview_confirmation_and_apply_are_private_then_one_undo_step() {
    let (_dir, state, app, _) = fixture("retained original").await;
    let before = state.project.lock().unwrap().clone();
    let opaque = state.opaque.lock().unwrap().clone();
    let (status, view) = request(&app, "GET", "/api/project/notes", Value::Null).await;
    assert_eq!(status, StatusCode::OK, "note read route must exist");
    assert_eq!(view["notes"], json!([]));
    assert_eq!(view["persistence"], "session-only");
    let action = note_action(" title ", 0);
    let (status, plan) = request(
        &app,
        "POST",
        "/api/project/notes/preview",
        preview_payload(&view, action.clone()),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::OK,
        "valid note preview must exist: {plan}"
    );
    assert_eq!(plan["action"], action);
    assert_eq!(plan["createdId"], 1);
    assert_eq!(plan["previous"], Value::Null);
    assert_eq!(state.project.lock().unwrap().clone(), before);
    assert_eq!(
        state.command_stack.lock().unwrap().history_lengths(),
        (0, 0)
    );
    let (status, refused) = request(
        &app,
        "POST",
        "/api/project/notes/apply",
        json!({"token":plan["token"],"confirmed":false}),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(refused["kind"], "noteConfirmationRequired");
    assert_eq!(state.project.lock().unwrap().clone(), before);
    let (status, outcome) = request(
        &app,
        "POST",
        "/api/project/notes/apply",
        json!({"token":plan["token"],"confirmed":true}),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::OK,
        "confirmed apply must succeed: {outcome}"
    );
    assert_eq!(outcome["changed"], true);
    assert_eq!(outcome["nativeGeneration"], Value::Null);
    assert_eq!(
        state.command_stack.lock().unwrap().history_lengths(),
        (1, 0)
    );
    assert_eq!(*state.opaque.lock().unwrap(), opaque);
    let after = state.project.lock().unwrap().clone();
    assert_eq!(after.as_ref().unwrap().notes[0].title(), " title ");
    assert!(!after.as_ref().unwrap().notes[0].include_in_documentation());
    assert_eq!(
        request(&app, "POST", "/api/undo", Value::Null).await.0,
        StatusCode::OK
    );
    assert!(state
        .project
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .notes
        .is_empty());
    assert_eq!(
        state
            .project
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .ids
            .peek_project_note(),
        1
    );
    assert_eq!(
        request(&app, "POST", "/api/redo", Value::Null).await.0,
        StatusCode::OK
    );
    assert_eq!(state.project.lock().unwrap().clone(), after);
}

async fn read_notes(app: &axum::Router) -> Value {
    let (status, view) = request(app, "GET", "/api/project/notes", Value::Null).await;
    assert_eq!(status, StatusCode::OK, "{view}");
    view
}
async fn plan_note(app: &axum::Router, action: Value) -> Value {
    let view = read_notes(app).await;
    let (status, plan) = request(
        app,
        "POST",
        "/api/project/notes/preview",
        preview_payload(&view, action),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::OK,
        "valid note operation must preview: {plan}"
    );
    plan
}
async fn apply_plan(app: &axum::Router, plan: &Value) -> Value {
    let (status, outcome) = request(
        app,
        "POST",
        "/api/project/notes/apply",
        json!({"token":plan["token"],"confirmed":true}),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{outcome}");
    outcome
}
#[tokio::test]
async fn existing_note_actions_show_exact_before_and_restore_order() {
    let (_dir, state, app, device) = fixture("original").await;
    let original = state.opaque.lock().unwrap().clone();
    let first = plan_note(&app, note_action("one", 0)).await;
    apply_plan(&app, &first).await;
    let mut second = note_action("two", 1);
    second["draft"]["target"] = json!({"kind":"device","id":device});
    let second = plan_note(&app, second).await;
    apply_plan(&app, &second).await;
    let mut draft = note_action(" replaced Grüße ", 0)["draft"].clone();
    draft["includeInDocumentation"] = json!(true);
    for action in [
        json!({"kind":"replace","id":1,"draft":draft}),
        json!({"kind":"move","id":1,"position":1}),
        json!({"kind":"remove","id":1}),
    ] {
        let before = read_notes(&app).await;
        let lengths = state.command_stack.lock().unwrap().history_lengths();
        let plan = plan_note(&app, action.clone()).await;
        assert_eq!(plan["action"], action);
        assert_eq!(
            plan["previous"],
            before["notes"]
                .as_array()
                .unwrap()
                .iter()
                .find(|n| n["id"] == 1)
                .unwrap()
                .clone()
        );
        assert_eq!(plan["createdId"], Value::Null);
        assert_eq!(read_notes(&app).await["notes"], before["notes"]);
        assert_eq!(apply_plan(&app, &plan).await["changed"], true);
        assert_eq!(
            state.command_stack.lock().unwrap().history_lengths(),
            (lengths.0 + 1, 0)
        );
        let after = read_notes(&app).await;
        match action["kind"].as_str().unwrap() {
            "replace" => {
                assert_eq!(after["notes"][0]["title"], " replaced Grüße ");
                assert_eq!(after["notes"][0]["includeInDocumentation"], true);
            }
            "move" => {
                assert_eq!(after["notes"][0]["id"], 2);
                assert_eq!(after["notes"][1]["id"], 1);
            }
            _ => assert_eq!(after["notes"].as_array().unwrap().len(), 1),
        }
        assert_eq!(
            request(&app, "POST", "/api/undo", Value::Null).await.0,
            StatusCode::OK
        );
        assert_eq!(read_notes(&app).await["notes"], before["notes"]);
        assert_eq!(
            request(&app, "POST", "/api/redo", Value::Null).await.0,
            StatusCode::OK
        );
        assert_eq!(read_notes(&app).await["notes"], after["notes"]);
    }
    assert_eq!(*state.opaque.lock().unwrap(), original);
    assert_eq!(
        state
            .project
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .ids
            .peek_project_note(),
        2
    );
}
#[tokio::test]
async fn cancelled_replaced_or_consumed_tokens_cannot_apply_and_never_reserve_ids() {
    let (_dir, state, app, _) = fixture("original").await;
    let before = state.project.lock().unwrap().clone();
    let first = plan_note(&app, note_action("cancel", 0)).await;
    let (status, result) = request(
        &app,
        "POST",
        "/api/project/notes/cancel",
        json!({"token":first["token"]}),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::OK,
        "exact note plan cancellation must exist: {result}"
    );
    assert_eq!(result["cancelled"], true);
    assert_eq!(
        request(
            &app,
            "POST",
            "/api/project/notes/apply",
            json!({"token":first["token"],"confirmed":true})
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
    assert_eq!(state.project.lock().unwrap().clone(), before);
    let replacement = plan_note(&app, note_action("older", 0)).await;
    let latest = plan_note(&app, note_action("latest", 0)).await;
    assert_eq!(latest["createdId"], 1);
    assert_ne!(replacement["token"], latest["token"]);
    assert_eq!(
        request(
            &app,
            "POST",
            "/api/project/notes/cancel",
            json!({"token":replacement["token"]})
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
    assert_eq!(
        request(
            &app,
            "POST",
            "/api/project/notes/apply",
            json!({"token":replacement["token"],"confirmed":true})
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
    let mut substitution = json!({"token":latest["token"],"confirmed":true});
    substitution["action"] = note_action("not reviewed", 0);
    assert_eq!(
        request(&app, "POST", "/api/project/notes/apply", substitution)
            .await
            .0,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    assert_eq!(state.project.lock().unwrap().clone(), before);
    apply_plan(&app, &latest).await;
    assert_eq!(
        request(
            &app,
            "POST",
            "/api/project/notes/apply",
            json!({"token":latest["token"],"confirmed":true})
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
    assert_eq!(
        state.project.lock().unwrap().as_ref().unwrap().notes[0].title(),
        "latest"
    );
}

async fn request_with_cookie(
    app: &axum::Router,
    method: &str,
    path: &str,
    payload: Value,
    cookie: &str,
) -> (StatusCode, Value) {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(method)
                .uri(path)
                .header("content-type", "application/json")
                .header("cookie", cookie)
                .body(Body::from(payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    let body = axum::body::to_bytes(response.into_body(), 4 * 1024 * 1024)
        .await
        .unwrap();
    (
        status,
        serde_json::from_slice(&body).unwrap_or_else(|_| json!({"nonJsonError":true})),
    )
}
#[tokio::test]
async fn every_note_route_is_guarded_and_an_authenticated_cookie_can_confirm_an_edit() {
    let (_dir, state, _open, _) = fixture("original").await;
    // Deliberately synthetic, inexpensive test credential; never a real account.
    let credential = "synthetic note adapter fixture";
    let hash = knx_server::hash_password_with_iterations(credential, 1000).unwrap();
    let auth = knx_server::AuthConfig::from_password_hash(&hash).unwrap();
    let app = knx_server::app_with_auth(state.clone(), None, auth);
    let before = state.project.lock().unwrap().clone();
    for (method, path) in [
        ("GET", "/api/project/notes"),
        ("GET", "/api/project/notes/targets"),
        ("POST", "/api/project/notes/preview"),
        ("POST", "/api/project/notes/apply"),
        ("POST", "/api/project/notes/cancel"),
    ] {
        assert_eq!(
            request(&app, method, path, json!({})).await.0,
            StatusCode::UNAUTHORIZED
        );
    }
    assert_eq!(state.project.lock().unwrap().clone(), before);
    let login = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/login")
                .header("content-type", "application/json")
                .body(Body::from(json!({"password":credential}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(login.status(), StatusCode::OK);
    let cookie = login
        .headers()
        .get("set-cookie")
        .unwrap()
        .to_str()
        .unwrap()
        .split(';')
        .next()
        .unwrap()
        .to_owned();
    let (status, view) =
        request_with_cookie(&app, "GET", "/api/project/notes", Value::Null, &cookie).await;
    assert_eq!(status, StatusCode::OK);
    let (status, plan) = request_with_cookie(
        &app,
        "POST",
        "/api/project/notes/preview",
        preview_payload(&view, note_action("authenticated", 0)),
        &cookie,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(state.project.lock().unwrap().clone(), before);
    assert_eq!(
        request_with_cookie(
            &app,
            "POST",
            "/api/project/notes/apply",
            json!({"token":plan["token"],"confirmed":true}),
            &cookie
        )
        .await
        .0,
        StatusCode::OK
    );
    assert_eq!(
        state.project.lock().unwrap().as_ref().unwrap().notes[0].title(),
        "authenticated"
    );
    let plan_view = request_with_cookie(&app, "GET", "/api/project/notes", Value::Null, &cookie)
        .await
        .1;
    let plan = request_with_cookie(
        &app,
        "POST",
        "/api/project/notes/preview",
        preview_payload(&plan_view, note_action("discard", 1)),
        &cookie,
    )
    .await
    .1;
    assert_eq!(
        request_with_cookie(
            &app,
            "POST",
            "/api/project/notes/cancel",
            json!({"token":plan["token"]}),
            &cookie
        )
        .await
        .0,
        StatusCode::OK
    );
}
#[tokio::test]
async fn malformed_and_unsupported_actions_refuse_without_consuming_notes_or_allocators() {
    let (_dir, state, app, _) = fixture("original").await;
    let view = read_notes(&app).await;
    let before = state.project.lock().unwrap().clone();
    let original = state.opaque.lock().unwrap().clone();
    let mut bad_title = note_action(" \t\r\n", 0);
    let mut bad_body = note_action("valid", 0);
    bad_body["draft"]["body"] = json!("bad\0body");
    let mut long_body = note_action("valid", 0);
    long_body["draft"]["body"] = json!("x".repeat(64 * 1024 + 1));
    let mut no_opt_in = note_action("valid", 0);
    no_opt_in["draft"]
        .as_object_mut()
        .unwrap()
        .remove("includeInDocumentation");
    let mut unknown_field = note_action("valid", 0);
    unknown_field["draft"]["rawSource"] = json!("must not be accepted");
    let mut actions = vec![
        bad_title.take(),
        bad_body,
        long_body,
        no_opt_in,
        unknown_field,
        note_action(&"x".repeat(513), 0),
        note_action("valid", 1),
        json!({"kind":"remove","id":0}),
        json!({"kind":"remove","id":99}),
        json!({"kind":"move","id":99,"position":0}),
        json!({"kind":"replace","id":99,"draft":note_action("valid",0)["draft"]}),
        json!({"kind":"writeBus"}),
    ];
    for target in [
        json!({"kind":"device","id":4294967295_u64}),
        json!({"kind":"installation","id":256}),
        json!({"kind":"area","id":999}),
        json!({"kind":"line","id":999}),
        json!({"kind":"buildingPart","id":999}),
        json!({"kind":"groupRange","id":999}),
        json!({"kind":"groupAddress","id":999}),
        json!({"kind":"comObject","id":999}),
        json!({"kind":"device","id":-1}),
        json!({"kind":"device","id":1.5}),
        json!({"kind":"device","id":"1"}),
        json!({"kind":"project","extra":"unknown"}),
        json!({"kind":"parameter","id":1}),
    ] {
        let mut action = note_action("valid", 0);
        action["draft"]["target"] = target;
        actions.push(action);
    }
    for action in actions {
        let (status, _) = request(
            &app,
            "POST",
            "/api/project/notes/preview",
            preview_payload(&view, action),
        )
        .await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(state.project.lock().unwrap().clone(), before);
        assert_eq!(
            state.command_stack.lock().unwrap().history_lengths(),
            (0, 0)
        );
        assert_eq!(*state.opaque.lock().unwrap(), original);
    }
    let mut oversized = note_action("valid", 0);
    oversized["draft"]["body"] = json!("x".repeat(1024 * 1024));
    assert_eq!(
        request(
            &app,
            "POST",
            "/api/project/notes/preview",
            preview_payload(&view, oversized)
        )
        .await
        .0,
        StatusCode::PAYLOAD_TOO_LARGE
    );
    assert_eq!(state.project.lock().unwrap().clone(), before);
    assert_eq!(
        plan_note(&app, note_action("valid", 0)).await["createdId"],
        1
    );
}
#[tokio::test]
async fn stale_server_load_revision_and_changed_original_context_cannot_apply() {
    use std::sync::atomic::Ordering;
    for kind in 0..6 {
        let (_dir, state, app, _) = fixture("original").await;
        let view = read_notes(&app).await;
        let plan = plan_note(&app, note_action("pending", 0)).await;
        match kind {
            0 => {
                state.project_incarnation.fetch_add(1, Ordering::Relaxed);
            }
            1 => {
                assert_eq!(
                    request(
                        &app,
                        "POST",
                        "/api/project/group-address-style",
                        json!({"groupAddressStyle":"Free"}),
                    )
                    .await
                    .0,
                    StatusCode::OK
                );
            }
            2 => {
                state.opaque.lock().unwrap()[0].bytes.push(b' ');
            }
            3 => {
                state.project.lock().unwrap().as_mut().unwrap().info.name =
                    "direct concurrent mutation".into();
            }
            4 => {
                let mut stale = preview_payload(&view, note_action("new", 0));
                stale["serverIncarnation"] = json!("other server");
                assert_eq!(
                    request(&app, "POST", "/api/project/notes/preview", stale)
                        .await
                        .0,
                    StatusCode::CONFLICT
                );
                continue;
            }
            _ => {
                let mut stale = preview_payload(&view, note_action("new", 0));
                stale["snapshotRevision"] = json!(0);
                assert_eq!(
                    request(&app, "POST", "/api/project/notes/preview", stale)
                        .await
                        .0,
                    StatusCode::CONFLICT
                );
                continue;
            }
        }
        let before = state.project.lock().unwrap().clone();
        let original = state.opaque.lock().unwrap().clone();
        let lengths = state.command_stack.lock().unwrap().history_lengths();
        assert_eq!(
            request(
                &app,
                "POST",
                "/api/project/notes/apply",
                json!({"token":plan["token"],"confirmed":true})
            )
            .await
            .0,
            StatusCode::CONFLICT
        );
        assert_eq!(state.project.lock().unwrap().clone(), before);
        assert_eq!(*state.opaque.lock().unwrap(), original);
        assert_eq!(
            state.command_stack.lock().unwrap().history_lengths(),
            lengths
        );
    }
}
#[tokio::test]
async fn identical_replace_and_move_consume_only_the_plan_not_redo_or_revision() {
    let (_dir, state, app, _) = fixture("original").await;
    apply_plan(&app, &plan_note(&app, note_action("one", 0)).await).await;
    apply_plan(&app, &plan_note(&app, note_action("two", 1)).await).await;
    assert_eq!(
        request(&app, "POST", "/api/undo", Value::Null).await.0,
        StatusCode::OK
    );
    for action in [
        json!({"kind":"replace","id":1,"draft":note_action("one",0)["draft"]}),
        json!({"kind":"move","id":1,"position":0}),
    ] {
        let before = state.project.lock().unwrap().clone();
        let view = read_notes(&app).await;
        let plan = plan_note(&app, action).await;
        assert_eq!(plan["noOp"], true);
        let outcome = apply_plan(&app, &plan).await;
        assert_eq!(outcome["changed"], false);
        assert_eq!(outcome["snapshotRevision"], view["snapshotRevision"]);
        assert_eq!(state.project.lock().unwrap().clone(), before);
        assert_eq!(
            state.command_stack.lock().unwrap().history_lengths(),
            (1, 1)
        );
        assert_eq!(
            request(
                &app,
                "POST",
                "/api/project/notes/apply",
                json!({"token":plan["token"],"confirmed":true})
            )
            .await
            .0,
            StatusCode::CONFLICT
        );
    }
    assert_eq!(
        request(&app, "POST", "/api/redo", Value::Null).await.0,
        StatusCode::OK
    );
    assert_eq!(read_notes(&app).await["notes"].as_array().unwrap().len(), 2);
}
#[tokio::test]
async fn native_working_notes_reopen_with_both_stacks_and_preserved_originals_then_explicit_save() {
    let (dir, state, app, _) = fixture("retained original").await;
    let source = std::fs::read(dir.path().join("synthetic.knxproj")).unwrap();
    let path = dir.path().join("notes.knxdb");
    assert_eq!(
        request(&app, "POST", "/api/project/save-as", json!({"path":path}))
            .await
            .0,
        StatusCode::OK
    );
    let before = state.project.lock().unwrap().clone().unwrap();
    let original = state.opaque.lock().unwrap().clone();
    assert!(!original.is_empty());
    let opened = knx_store::open_existing_read_only(&path).unwrap();
    let seed = knx_store::project_history::NativeSnapshot::read(&opened.conn).unwrap();
    assert_eq!(seed.project, before);
    assert_eq!(seed.opaque, original);
    drop(opened);
    for (position, title) in ["one", "two", "three"].into_iter().enumerate() {
        let plan = plan_note(&app, note_action(title, position)).await;
        assert_eq!(
            apply_plan(&app, &plan).await["persistence"],
            "native-working-history"
        );
    }
    let stale = plan_note(&app, note_action("not applied", 3)).await;
    assert_eq!(
        request(&app, "POST", "/api/undo", Value::Null).await.0,
        StatusCode::OK
    );
    let working = state.project.lock().unwrap().clone().unwrap();
    assert_eq!(
        state.command_stack.lock().unwrap().history_lengths(),
        (2, 1)
    );
    let reopened = knx_store::open_existing_read_only(&path).unwrap();
    assert_eq!(
        knx_store::project_history::NativeSnapshot::read(&reopened.conn)
            .unwrap()
            .project,
        before,
        "working notes must not silently save the root"
    );
    drop(reopened);
    let second_state = Arc::new(knx_server::AppState::new(dir.path().join("second-server")));
    let second = knx_server::app(second_state.clone(), None);
    assert_eq!(
        request(&second, "POST", "/api/project/open", json!({"path":path}))
            .await
            .0,
        StatusCode::OK
    );
    assert_eq!(
        second_state.project.lock().unwrap().clone().unwrap(),
        working
    );
    assert_eq!(*second_state.opaque.lock().unwrap(), original);
    assert_eq!(
        second_state.command_stack.lock().unwrap().history_lengths(),
        (2, 1)
    );
    assert_eq!(
        request(
            &second,
            "POST",
            "/api/project/notes/apply",
            json!({"token":stale["token"],"confirmed":true})
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
    assert_eq!(
        request(&second, "POST", "/api/redo", Value::Null).await.0,
        StatusCode::OK
    );
    assert_eq!(
        read_notes(&second).await["notes"].as_array().unwrap().len(),
        3
    );
    assert_eq!(
        request(&second, "POST", "/api/undo", Value::Null).await.0,
        StatusCode::OK
    );
    assert_eq!(
        request(&second, "POST", "/api/project/save", Value::Null)
            .await
            .0,
        StatusCode::OK
    );
    let saved = knx_store::open_existing_read_only(&path).unwrap();
    let root = knx_store::project_history::NativeSnapshot::read(&saved.conn).unwrap();
    assert_eq!(root.project, working);
    assert_eq!(root.opaque, original);
    assert_eq!(
        std::fs::read(dir.path().join("synthetic.knxproj")).unwrap(),
        source
    );
}
#[tokio::test]
async fn failed_native_admission_never_publishes_notes_ids_history_or_paths() {
    let (dir, state, app, _) = fixture("original").await;
    let missing = dir.path().join("missing.knxdb");
    *state.store_path.lock().unwrap() = Some(missing.clone());
    *state.history_generation.lock().unwrap() = 1;
    let before = state.project.lock().unwrap().clone();
    let original = state.opaque.lock().unwrap().clone();
    let plan = plan_note(&app, note_action("pending", 0)).await;
    let (status, error) = request(
        &app,
        "POST",
        "/api/project/notes/apply",
        json!({"token":plan["token"],"confirmed":true}),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(error["kind"], "notePersistenceFailed");
    assert!(!error.to_string().contains(missing.to_str().unwrap()));
    assert_eq!(state.project.lock().unwrap().clone(), before);
    assert_eq!(*state.opaque.lock().unwrap(), original);
    assert_eq!(
        state.command_stack.lock().unwrap().history_lengths(),
        (0, 0)
    );
    assert_eq!(*state.history_generation.lock().unwrap(), 1);
    assert!(!missing.exists());
}
#[tokio::test]
async fn another_native_writer_is_not_overwritten_by_confirmed_http_apply() {
    let (dir, state, app, _) = fixture("original").await;
    let path = dir.path().join("shared.knxdb");
    assert_eq!(
        request(&app, "POST", "/api/project/save-as", json!({"path":path}))
            .await
            .0,
        StatusCode::OK
    );
    let before = state.project.lock().unwrap().clone();
    let original = state.opaque.lock().unwrap().clone();
    let plan = plan_note(&app, note_action("pending", 0)).await;
    let opened = knx_store::open_existing_read_only(&path).unwrap();
    let mut other = knx_store::project_history::NativeSnapshot::read(&opened.conn).unwrap();
    assert_eq!(Some(other.project.clone()), before);
    assert_eq!(other.opaque, original);
    drop(opened);
    other.project.info.name = "other native writer".into();
    let generation = *state.history_generation.lock().unwrap();
    let newer = knx_app::project_history::persist_working(
        &path,
        &other,
        &knx_core::CommandStack::new(),
        generation,
    )
    .unwrap();
    let bytes = std::fs::read(&path).unwrap();
    let (status, error) = request(
        &app,
        "POST",
        "/api/project/notes/apply",
        json!({"token":plan["token"],"confirmed":true}),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(error["kind"], "notePersistenceFailed");
    assert_eq!(state.project.lock().unwrap().clone(), before);
    assert_eq!(*state.opaque.lock().unwrap(), original);
    assert_eq!(*state.history_generation.lock().unwrap(), generation);
    assert_eq!(
        state.command_stack.lock().unwrap().history_lengths(),
        (0, 0)
    );
    assert_eq!(std::fs::read(&path).unwrap(), bytes);
    let read = knx_store::open_existing_read_only(&path).unwrap();
    let editor = knx_store::project_history::load_editor(&read.conn)
        .unwrap()
        .unwrap();
    assert_eq!(editor.working, other);
    assert_eq!(editor.generation, newer);
}

#[tokio::test]
async fn sqlite_lock_and_history_budget_refuse_without_publishing_or_changing_saved_bytes() {
    for locked in [false, true] {
        let (dir, state, app, _) = fixture("original").await;
        let path = dir.path().join("guarded.knxdb");
        assert_eq!(
            request(&app, "POST", "/api/project/save-as", json!({"path":path}))
                .await
                .0,
            StatusCode::OK
        );
        let before = state.project.lock().unwrap().clone();
        let original = state.opaque.lock().unwrap().clone();
        let opened = knx_store::open_existing_read_only(&path).unwrap();
        let seed = knx_store::project_history::NativeSnapshot::read(&opened.conn).unwrap();
        assert_eq!(Some(seed.project), before);
        assert_eq!(seed.opaque, original);
        drop(opened);
        if !locked {
            *state.command_stack.lock().unwrap() = knx_core::CommandStack::from_snapshot_states(
                vec![before.clone().unwrap(); knx_store::project_history::MAX_STACK_STATES],
                vec![],
            );
        }
        let lengths = state.command_stack.lock().unwrap().history_lengths();
        let generation = *state.history_generation.lock().unwrap();
        let plan = plan_note(&app, note_action("pending", 0)).await;
        let bytes = std::fs::read(&path).unwrap();
        let blocker = locked.then(|| knx_store::open_existing_and_migrate(&path).unwrap());
        if let Some(conn) = &blocker {
            conn.execute_batch("BEGIN EXCLUSIVE").unwrap();
        }
        let (status, error) = request(
            &app,
            "POST",
            "/api/project/notes/apply",
            json!({"token":plan["token"],"confirmed":true}),
        )
        .await;
        if let Some(conn) = &blocker {
            conn.execute_batch("ROLLBACK").unwrap();
        }
        drop(blocker);
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(error["kind"], "notePersistenceFailed");
        assert_eq!(state.project.lock().unwrap().clone(), before);
        assert_eq!(*state.opaque.lock().unwrap(), original);
        assert_eq!(
            state.command_stack.lock().unwrap().history_lengths(),
            lengths
        );
        assert_eq!(*state.history_generation.lock().unwrap(), generation);
        assert_eq!(std::fs::read(&path).unwrap(), bytes);
        let opened = knx_store::open_existing_read_only(&path).unwrap();
        let final_state = knx_store::project_history::NativeSnapshot::read(&opened.conn).unwrap();
        assert_eq!(Some(final_state.project), before);
        assert_eq!(final_state.opaque, original);
    }
}

#[tokio::test]
async fn target_catalogue_offers_objects_by_stable_id_with_the_snapshot_identity() {
    let (_dir, state, app, device) = fixture("original").await;
    {
        let mut lock = state.project.lock().unwrap();
        let project = lock.as_mut().unwrap();
        let id = knx_core::ComObjectInstanceId(4242);
        project
            .devices
            .get_mut(knx_core::DeviceId(device))
            .unwrap()
            .com_objects
            .push(id);
        project
            .devices
            .insert_com_object(knx_core::ComObjectInstance {
                id,
                source: knx_core::SourceRef {
                    path: "synthetic".into(),
                    ets_id: "synthetic".into(),
                },
                device: knx_core::DeviceId(device),
                number: 7,
                text: knx_core::Override::Value(knx_core::Resolved {
                    value: knx_core::Text::Literal("Synthetic switch".into()),
                    layer: knx_core::Layer::Instance,
                }),
                description: knx_core::Override::Absent,
                dpt: knx_core::Override::Absent,
                flags: knx_core::ResolvedFlags::none(),
                size: None,
                is_active: true,
                links: vec![],
                module_instance: None,
            });
    }
    let before = state.project.lock().unwrap().clone();
    let notes = read_notes(&app).await;
    let (status, view) = request(&app, "GET", "/api/project/notes/targets", Value::Null).await;
    assert_eq!(status, StatusCode::OK, "{view}");
    for key in [
        "serverIncarnation",
        "projectIncarnation",
        "snapshotRevision",
    ] {
        assert_eq!(view[key], notes[key], "{key} must match the notes read");
    }
    let targets = view["targets"].as_array().unwrap();
    assert!(
        targets.contains(&json!({
            "target": {"kind": "device", "id": device}, "owner": [], "parts": ["Synthetic"]
        })),
        "{view}"
    );
    assert!(
        targets.contains(&json!({
            "target": {"kind": "comObject", "id": 4242},
            "owner": ["Synthetic"], "parts": ["7", "Synthetic switch"]
        })),
        "{view}"
    );
    assert!(
        targets
            .iter()
            .all(|entry| entry["target"]["kind"] != "project"),
        "the editor owns the project choice"
    );
    assert_eq!(
        state.project.lock().unwrap().clone(),
        before,
        "reading targets is read-only"
    );

    // The offered object is accepted by a preview; nothing is applied.
    let action = json!({"kind":"create","position":0,"draft":{
        "target":{"kind":"comObject","id":4242},"title":"Object note","body":"",
        "includeInDocumentation":false}});
    let (status, plan) = request(
        &app,
        "POST",
        "/api/project/notes/preview",
        preview_payload(&notes, action),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{plan}");
    assert_eq!(state.project.lock().unwrap().clone(), before);
}

#[tokio::test]
async fn target_catalogue_without_a_project_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let state = Arc::new(knx_server::AppState::new(dir.path().to_path_buf()));
    let app = knx_server::app(state, None);
    let (status, _) = request(&app, "GET", "/api/project/notes/targets", Value::Null).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}
