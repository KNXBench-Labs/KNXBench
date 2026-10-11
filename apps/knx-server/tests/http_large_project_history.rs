//! Synthetic application lifecycle and an explicit private opt-in replay.
//!
//! No source values are emitted or embedded in public fixtures.

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    path::Path,
    sync::{Arc, Mutex},
};
use tower::ServiceExt;

async fn request(app: &axum::Router, path: &str, payload: Value) -> Value {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(path)
                .header("content-type", "application/json")
                .body(Body::from(payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), 16 * 1024 * 1024)
        .await
        .unwrap();
    if let Some(evidence) = std::env::var_os("KNXBENCH_PRIVATE_LIFECYCLE_EVIDENCE") {
        use std::io::Write;
        let root = std::path::PathBuf::from(evidence);
        let mut receipt = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(root.join("lifecycle-steps.private.jsonl"))
            .unwrap();
        writeln!(
            receipt,
            "{}",
            json!({"route": path, "status": status.as_u16()})
        )
        .unwrap();
        if status != StatusCode::OK {
            std::fs::write(root.join("lifecycle-refusal.private.json"), &bytes).unwrap();
        }
    }
    assert!(
        status == StatusCode::OK,
        "application lifecycle step refused: {path} ({status})"
    );
    if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&bytes).unwrap()
    }
}

fn state(dir: &Path) -> Arc<knx_server::AppState> {
    let products = knx_productdb::open_and_migrate(&dir.join("products.sqlite")).unwrap();
    Arc::new(knx_server::AppState {
        product_db: Some(Mutex::new(products)),
        ..knx_server::AppState::new(dir.to_path_buf())
    })
}

async fn lifecycle(dir: &Path, state: Arc<knx_server::AppState>, app: axum::Router) {
    let path = dir.join("application-history.knxdb");
    let original = state.project.lock().unwrap().clone().unwrap();
    let opaque = state.opaque.lock().unwrap().clone();
    let refs = state.manufacturer_refs.lock().unwrap().clone();
    request(&app, "/api/project/save-as", json!({"path": path})).await;
    assert!(path.is_file(), "Save As must publish its file");
    drop(app);
    drop(state);

    let reopened = self::state(dir);
    let app = knx_server::app(reopened.clone(), None);
    request(&app, "/api/project/open", json!({"path": path})).await;
    assert!(
        reopened.project.lock().unwrap().as_ref() == Some(&original),
        "native open model changed"
    );
    assert!(
        *reopened.opaque.lock().unwrap() == opaque,
        "native open opaque context changed"
    );
    assert!(
        *reopened.manufacturer_refs.lock().unwrap() == refs,
        "native open manifest changed"
    );
    let old_style = original.info.group_address_style;
    let new_style = if old_style == knx_core::GroupAddressStyle::Free {
        "ThreeLevel"
    } else {
        "Free"
    };
    request(
        &app,
        "/api/project/group-address-style",
        json!({"groupAddressStyle": new_style}),
    )
    .await;
    assert!(reopened.command_stack.lock().unwrap().can_undo());
    let modified = reopened.project.lock().unwrap().clone().unwrap();
    assert!(modified != original, "ordinary edit was not applied");
    request(&app, "/api/project/save", Value::Null).await;
    drop(app);
    drop(reopened);

    let reopened = self::state(dir);
    let app = knx_server::app(reopened.clone(), None);
    request(&app, "/api/project/open", json!({"path": path})).await;
    assert!(
        reopened.project.lock().unwrap().as_ref() == Some(&modified),
        "saved edit did not reopen"
    );
    request(&app, "/api/undo", Value::Null).await;
    assert!(
        reopened.project.lock().unwrap().as_ref() == Some(&original),
        "durable undo changed model"
    );
    request(&app, "/api/redo", Value::Null).await;
    assert!(
        reopened.project.lock().unwrap().as_ref() == Some(&modified),
        "durable redo changed model"
    );
    request(&app, "/api/project/save", Value::Null).await;
    let admitted = knx_store::open_existing_read_only(&path).unwrap();
    assert!(
        knx_store::load_opaque(&admitted.conn).unwrap() == opaque,
        "opaque context changed after edit/save"
    );
    assert!(
        knx_store::load_manufacturer_refs(&admitted.conn).unwrap() == refs,
        "manifest changed after edit/save"
    );
    let versions = knx_store::project_history::list_versions(&admitted.conn).unwrap();
    assert!(!versions.is_empty(), "save must preserve predecessor");
    let context_bytes: i64 = admitted
        .conn
        .query_row(
            "SELECT sum(length(image)) FROM project_history_context",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert!(context_bytes <= knx_store::project_history::MAX_CONTEXT_BYTES as i64);
}

#[tokio::test]
async fn large_synthetic_context_survives_save_as_open_edit_save_restart_undo_redo() {
    let dir = tempfile::tempdir().unwrap();
    let state = self::state(dir.path());
    let app = knx_server::app(state.clone(), None);
    request(
        &app,
        "/api/project/new",
        json!({"name": "Synthetic large-context witness"}),
    )
    .await;
    *state.opaque.lock().unwrap() = vec![knx_store::StoredOpaqueEntry {
        source_path: "synthetic/preserved.bin".into(),
        xpath: "".into(),
        kind: "File".into(),
        name: "preserved.bin".into(),
        bytes: vec![b'x'; 65 * 1024 * 1024],
        sha256: "synthetic-source-identity".into(),
    }];
    *state.manufacturer_refs.lock().unwrap() = vec![knx_store::ManufacturerRef {
        source_path: "synthetic/manifest.xml".into(),
        sha256: "synthetic-manifest-identity".into(),
        len: 123,
        kind: "ManufacturerData".into(),
    }];
    lifecycle(dir.path(), state, app).await;
}

#[tokio::test]
#[ignore = "requires explicit private project/evidence environment; never skips when selected"]
async fn private_import_save_open_edit_save_lifecycle() {
    let source = std::env::var_os("KNXBENCH_PRIVATE_LIFECYCLE_PROJECT")
        .expect("private lifecycle source opt-in missing");
    let evidence = std::env::var_os("KNXBENCH_PRIVATE_LIFECYCLE_EVIDENCE")
        .expect("private lifecycle evidence opt-in missing");
    let source = std::path::PathBuf::from(source);
    assert!(source.is_file(), "private lifecycle source unavailable");
    let before = Sha256::digest(std::fs::read(&source).expect("private source read failed"));
    let dir = tempfile::Builder::new()
        .prefix("private-lifecycle-")
        .tempdir_in(evidence)
        .unwrap();
    let state = self::state(dir.path());
    let app = knx_server::app(state.clone(), None);
    request(&app, "/api/project/import", json!({"path": source})).await;
    assert!(
        state
            .project
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .devices
            .iter()
            .next()
            .is_some(),
        "private import exercised no devices"
    );
    assert!(
        !state.opaque.lock().unwrap().is_empty(),
        "private import exercised no retained context"
    );
    lifecycle(dir.path(), state, app).await;
    assert!(
        Sha256::digest(std::fs::read(&source).expect("private source reread failed")) == before,
        "private source changed"
    );
}
