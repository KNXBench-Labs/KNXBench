//! Project reads and edits outside the project router wait for the project off the runtime (#36).

use std::sync::Arc;

use axum::http::StatusCode;

use super::editor_runtime_tests::{
    assert_routes_leave_the_runtime_free, assert_routes_leave_the_runtime_free_with,
};
use crate::AppState;

/// The whole application, as a user's server mounts it (no sign-in).
fn app(state: &Arc<AppState>) -> axum::Router {
    crate::app(Arc::clone(state), None)
}

/// An empty product database: readiness and the batch plan refuse without
/// one before they look at the project.
fn with_products() -> Arc<AppState> {
    let dir = tempfile::tempdir().unwrap();
    let conn = knx_productdb::open_and_migrate(&dir.path().join("p.sqlite")).unwrap();
    // The open connection keeps the file usable once the directory is gone.
    drop(dir);
    Arc::new(AppState {
        product_db: Some(crate::CatalogueOwner::adopt_opened(conn, Default::default()).unwrap()),
        ..Default::default()
    })
}

/// Each route meets a held project lock (see the helper, which also fails a
/// row answered before the lock). Stale bindings (`serverIncarnation` "x"),
/// unknown tokens and an unknown address make the handler refuse under it.
#[tokio::test(flavor = "current_thread")]
async fn module_routes_leave_the_runtime_free() {
    let routes: &[(&str, &str, &str, StatusCode)] = &[
        ("GET", "/api/project/metadata", "", StatusCode::OK),
        ("GET", "/api/devices/1/recorded-state", "", StatusCode::OK),
        ("GET", "/api/project/payloads", "", StatusCode::OK),
        ("GET", "/api/project/source-reports", "", StatusCode::OK),
        (
            "GET",
            "/api/project/source-reports/x",
            "",
            StatusCode::UNPROCESSABLE_ENTITY,
        ),
        ("GET", "/api/project/import-report", "", StatusCode::OK),
        (
            "POST",
            "/api/devices/1/instructions/preview",
            r#"{"serverIncarnation":"x","projectIncarnation":0,"snapshotRevision":0,"document":null}"#,
            StatusCode::CONFLICT,
        ),
        (
            "POST",
            "/api/devices/1/instructions/apply",
            r#"{"token":"t","confirmed":true,"confirmConversion":false}"#,
            StatusCode::CONFLICT,
        ),
        ("GET", "/api/project/notes/targets", "", StatusCode::OK),
        (
            "POST",
            "/api/project/notes/preview",
            r#"{"serverIncarnation":"x","projectIncarnation":0,"snapshotRevision":0,"action":{"kind":"remove","id":99}}"#,
            StatusCode::CONFLICT,
        ),
        (
            "POST",
            "/api/project/notes/apply",
            r#"{"token":"t","confirmed":true}"#,
            StatusCode::CONFLICT,
        ),
        (
            "POST",
            "/api/name-recipe/preview",
            r#"{"serverIncarnation":"x","projectIncarnation":0,"snapshotRevision":0,"groupAddresses":[],"prefix":"a","suffix":""}"#,
            StatusCode::CONFLICT,
        ),
        (
            "POST",
            "/api/name-recipe/apply",
            r#"{"token":"t","confirmed":true}"#,
            StatusCode::CONFLICT,
        ),
        (
            "POST",
            "/api/project/delete-with-notes",
            r#"{"serverIncarnation":"x","projectIncarnation":0,"snapshotRevision":0,"delete":{"kind":"devices","ids":[1]},"choices":[]}"#,
            StatusCode::CONFLICT,
        ),
        (
            "POST",
            "/api/device/1/programming",
            r#"{"programmed":true,"serverIncarnation":"x","projectIncarnation":0,"snapshotRevision":0}"#,
            StatusCode::CONFLICT,
        ),
        (
            "GET",
            "/api/device/1/parameter-editor-source?etsId=P-1",
            "",
            StatusCode::OK,
        ),
        (
            "POST",
            "/api/catalog/cleanup/preview",
            r#"{"itemIds":[]}"#,
            StatusCode::SERVICE_UNAVAILABLE,
        ),
        (
            "POST",
            "/api/device-download/plan",
            r#"{"address":"1.1.7"}"#,
            StatusCode::UNPROCESSABLE_ENTITY,
        ),
    ];
    assert_routes_leave_the_runtime_free(routes, app).await;
}

/// Adoption needs a browser's comparison session cookie before it takes the
/// project; an unknown session is refused under the lock.
#[tokio::test(flavor = "current_thread")]
async fn comparison_adoption_leaves_the_runtime_free() {
    let cookie = format!("knx_comparison_session={}", "a".repeat(64));
    let routes: &[(&str, &str, &str, StatusCode)] = &[
        (
            "POST",
            "/api/device-compare/preview",
            r#"{"observationToken":"x","selected":[]}"#,
            StatusCode::CONFLICT,
        ),
        (
            "POST",
            "/api/device-compare/apply",
            r#"{"previewToken":"x","confirmation":"AdoptDeviceComparison"}"#,
            StatusCode::CONFLICT,
        ),
    ];
    assert_routes_leave_the_runtime_free_with(
        routes,
        app,
        || Arc::new(AppState::default()),
        Some(("cookie", &cookie)),
    )
    .await;
}

#[tokio::test(flavor = "current_thread")]
async fn readiness_and_batch_plan_leave_the_runtime_free() {
    let routes: &[(&str, &str, &str, StatusCode)] = &[
        ("GET", "/api/device-readiness", "", StatusCode::OK),
        (
            "POST",
            "/api/device-download/batch/plan",
            "",
            StatusCode::OK,
        ),
    ];
    assert_routes_leave_the_runtime_free_with(routes, app, with_products, None).await;
}
