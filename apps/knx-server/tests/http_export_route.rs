use std::path::PathBuf;
use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use serde_json::{json, Value};
use tower::ServiceExt;

fn reference_ets4_path() -> PathBuf {
    knx_testsupport::reference_ets4_path()
}

async fn body_json(response: axum::response::Response) -> Value {
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

#[tokio::test]
async fn exporting_without_a_store_path_is_a_400() {
    if !reference_ets4_path().exists() {
        eprintln!("skip: OriginalData/ corpus not present (gitignored, local-only)");
        return;
    }
    let state = Arc::new(knx_server::AppState::default());
    let app = knx_server::app(state, None);
    let dir = tempfile::tempdir().unwrap();
    let export_path = dir.path().join("out.knxproj");

    let import_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/project/import")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({ "path": reference_ets4_path().to_string_lossy() }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(import_response.status(), StatusCode::OK);

    // No save-as in between: `store_path` is still unset.
    let export_response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/project/export")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({ "path": export_path.to_string_lossy() }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(export_response.status(), StatusCode::BAD_REQUEST);
    let body = body_json(export_response).await;
    let error = body["error"].as_str().unwrap();
    assert!(!error.is_empty());
    assert!(
        error.contains(".knxdb"),
        "error should point at saving as .knxdb first, got: {error}"
    );
}

#[tokio::test]
async fn importing_saving_then_exporting_round_trips_and_reports_the_unsigned_warning() {
    if !reference_ets4_path().exists() {
        eprintln!("skip: OriginalData/ corpus not present (gitignored, local-only)");
        return;
    }
    let state = Arc::new(knx_server::AppState::default());
    let app = knx_server::app(state, None);
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("roundtrip.knxdb");
    let export_path = dir.path().join("roundtrip.knxproj");

    let import_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/project/import")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({ "path": reference_ets4_path().to_string_lossy() }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(import_response.status(), StatusCode::OK);

    let save_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/project/save-as")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({ "path": db_path.to_string_lossy() }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(save_response.status(), StatusCode::OK);

    let export_response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/project/export")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({ "path": export_path.to_string_lossy() }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(export_response.status(), StatusCode::OK);
    let report = body_json(export_response).await;
    let warnings = report["warnings"].as_array().unwrap();
    // Every export is unsigned (`export_knxproj`'s own doc comment: "Always
    // present"); the JSON tag is the externally-tagged, camelCase-rendered
    // unit-like-variant key `serde` actually emits for `ExportWarningDto`
    // (confirmed against a throwaway `serde_json::to_string` check, not
    // guessed) — `{"unsigned":{"detail": "..."}}`.
    assert!(
        warnings.iter().any(|w| w.get("unsigned").is_some()),
        "expected an 'unsigned' warning in {warnings:?}"
    );

    assert!(
        std::fs::metadata(&export_path).unwrap().len() > 0,
        "export should have written a non-empty .knxproj file"
    );
}

/// Regression test for the data-integrity bug the whole-branch review
/// caught (final-review.md, finding B1): a server-side ETS import used a
/// throwaway in-memory store whose opaque passthrough + manufacturer
/// manifest rows never survived into the `.knxdb` written by Save As, so
/// the exported `.knxproj` silently lost that data even though the export
/// route itself reported no error. Confirms the fix by reimporting the
/// exported file and checking its own opaque/manifest tables are non-empty
/// — a genuine content check, not just "the file is non-empty" (which the
/// previous, buggy 29.9KB-from-1.7MB export also satisfied).
#[tokio::test]
async fn exported_project_still_carries_opaque_and_manufacturer_data_after_save_as() {
    if !reference_ets4_path().exists() {
        eprintln!("skip: OriginalData/ corpus not present (gitignored, local-only)");
        return;
    }
    let state = Arc::new(knx_server::AppState::default());
    let app = knx_server::app(state, None);
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("roundtrip.knxdb");
    let export_path = dir.path().join("roundtrip.knxproj");

    let import_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/project/import")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({ "path": reference_ets4_path().to_string_lossy() }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(import_response.status(), StatusCode::OK);

    let save_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/project/save-as")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({ "path": db_path.to_string_lossy() }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(save_response.status(), StatusCode::OK);

    // The bug lived here: the `.knxdb` Save As just wrote must itself carry
    // the opaque/manifest rows, or the export below has nothing correct to
    // read from regardless of what the export route does.
    let saved_conn = knx_store::open_and_migrate(&db_path).unwrap();
    let saved_opaque = knx_store::load_opaque(&saved_conn).unwrap();
    let saved_manifest = knx_store::load_manufacturer_refs(&saved_conn).unwrap();
    assert!(
        !saved_opaque.is_empty(),
        "the saved .knxdb should carry the opaque entries the ETS import produced"
    );
    assert!(
        !saved_manifest.is_empty(),
        "the saved .knxdb should carry the manufacturer manifest the ETS import produced"
    );

    let export_response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/project/export")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({ "path": export_path.to_string_lossy() }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(export_response.status(), StatusCode::OK);

    // Reimport the exported .knxproj into a fresh throwaway store and
    // confirm it still carries opaque/manifest data of its own — the
    // end-to-end guarantee a user actually cares about.
    let reimport_conn = knx_store::open_and_migrate_in_memory().unwrap();
    knx_app::import_ets_project_with(
        &export_path,
        &reimport_conn,
        knx_app::ImportOptions::default(),
    )
    .unwrap();
    let reimported_opaque = knx_store::load_opaque(&reimport_conn).unwrap();
    let reimported_manifest = knx_store::load_manufacturer_refs(&reimport_conn).unwrap();
    assert!(
        !reimported_opaque.is_empty(),
        "the exported .knxproj should still carry opaque data on reimport"
    );
    assert!(
        !reimported_manifest.is_empty(),
        "the exported .knxproj should still carry manufacturer manifest data on reimport"
    );
}

/// Regression test for the whole-branch review's round-2 finding: the B1
/// fix made `save_project_as_impl` call `insert_opaque`/
/// `insert_manufacturer_refs` on every save, including a plain repeated
/// `POST /api/project/save` that reuses `store_path` — and those two
/// functions used to be pure `INSERT`s with no clear-first step, so every
/// repeated save duplicated every opaque/manifest row without bound.
/// Confirms the fix (clear-before-insert, now inside `knx-store` itself)
/// by saving the same already-populated store three times and asserting
/// the row counts stay put.
#[tokio::test]
async fn saving_the_same_project_twice_does_not_duplicate_opaque_and_manifest_rows() {
    if !reference_ets4_path().exists() {
        eprintln!("skip: OriginalData/ corpus not present (gitignored, local-only)");
        return;
    }
    let state = Arc::new(knx_server::AppState::default());
    let app = knx_server::app(state, None);
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("roundtrip.knxdb");

    let import_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/project/import")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({ "path": reference_ets4_path().to_string_lossy() }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(import_response.status(), StatusCode::OK);

    let save_as_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/project/save-as")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({ "path": db_path.to_string_lossy() }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(save_as_response.status(), StatusCode::OK);

    let conn = knx_store::open_and_migrate(&db_path).unwrap();
    let opaque_count_after_first_save = knx_store::load_opaque(&conn).unwrap().len();
    let manifest_count_after_first_save = knx_store::load_manufacturer_refs(&conn).unwrap().len();
    assert!(opaque_count_after_first_save > 0);
    assert!(manifest_count_after_first_save > 0);
    drop(conn);

    // Plain "Save" twice more, reusing `store_path` — the path the bug
    // lived on.
    for _ in 0..2 {
        let save_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/project/save")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(save_response.status(), StatusCode::OK);
    }

    let conn = knx_store::open_and_migrate(&db_path).unwrap();
    assert_eq!(
        knx_store::load_opaque(&conn).unwrap().len(),
        opaque_count_after_first_save,
        "repeated saves must not duplicate opaque rows"
    );
    assert_eq!(
        knx_store::load_manufacturer_refs(&conn).unwrap().len(),
        manifest_count_after_first_save,
        "repeated saves must not duplicate manufacturer manifest rows"
    );
}
