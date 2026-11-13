//! Project edits, previews and the device list wait for the project off the runtime (#36).

use std::sync::Arc;
use std::time::Duration;

use axum::http::StatusCode;
use tower::ServiceExt;

use super::editor_runtime_tests::{
    assert_routes_leave_the_runtime_free, assert_routes_leave_the_runtime_free_with,
    project_with_device, request,
};
use crate::AppState;

fn router(state: &Arc<AppState>) -> axum::Router {
    super::project_routes()
        .merge(crate::function_routes::router())
        .with_state(Arc::clone(state))
}

/// Each route meets a held project lock (see the helper). Exact statuses: an
/// extractor refusal answers before the handler takes the lock and would prove
/// nothing, so each body is well-formed and an unknown id (99) makes the
/// handler refuse under the lock instead (400; 409 for a stale binding).
#[tokio::test(flavor = "current_thread")]
async fn edit_routes_leave_the_runtime_free() {
    let routes: &[(&str, &str, &str, StatusCode)] = &[
        ("POST", "/api/project/journal/retry", "", StatusCode::OK),
        (
            "POST",
            "/api/project/group-address-style",
            r#"{"groupAddressStyle":"TwoLevel"}"#,
            StatusCode::OK,
        ),
        ("POST", "/api/undo", "", StatusCode::BAD_REQUEST),
        ("POST", "/api/redo", "", StatusCode::BAD_REQUEST),
        (
            "POST",
            "/api/group-addresses",
            r#"{"name":"G","address":"1/1/1"}"#,
            StatusCode::OK,
        ),
        (
            "DELETE",
            "/api/group-addresses/99",
            "",
            StatusCode::BAD_REQUEST,
        ),
        (
            "PATCH",
            "/api/group-addresses/99/name",
            r#"{"name":"x","expectedName":"y","serverIncarnation":"x","snapshotRevision":0,"projectIncarnation":0}"#,
            StatusCode::CONFLICT,
        ),
        (
            "POST",
            "/api/group-addresses/batch-delete",
            r#"{"ids":[99]}"#,
            StatusCode::BAD_REQUEST,
        ),
        (
            "POST",
            "/api/project/documentation-preview",
            "{}",
            StatusCode::OK,
        ),
        (
            "PATCH",
            "/api/installations/99",
            r#"{"name":"x"}"#,
            StatusCode::BAD_REQUEST,
        ),
        (
            "POST",
            "/api/areas",
            r#"{"name":"A","address":1}"#,
            StatusCode::OK,
        ),
        ("DELETE", "/api/areas/99", "", StatusCode::BAD_REQUEST),
        (
            "PATCH",
            "/api/areas/99",
            r#"{"name":"x"}"#,
            StatusCode::BAD_REQUEST,
        ),
        (
            "POST",
            "/api/lines",
            r#"{"areaId":99,"name":"L","address":1,"mediumRef":"MT-0"}"#,
            StatusCode::BAD_REQUEST,
        ),
        ("DELETE", "/api/lines/99", "", StatusCode::BAD_REQUEST),
        (
            "PATCH",
            "/api/lines/99",
            r#"{"name":"x"}"#,
            StatusCode::BAD_REQUEST,
        ),
        (
            "POST",
            "/api/move-line-to-area",
            r#"{"id":99,"areaId":98}"#,
            StatusCode::BAD_REQUEST,
        ),
        (
            "POST",
            "/api/repair/device-placement",
            r#"{"deviceId":99,"keepLineId":98}"#,
            StatusCode::BAD_REQUEST,
        ),
        (
            "POST",
            "/api/repair/line-owner",
            r#"{"lineId":99,"keepAreaId":98}"#,
            StatusCode::BAD_REQUEST,
        ),
        (
            "POST",
            "/api/move-device",
            r#"{"deviceId":1,"lineId":99}"#,
            StatusCode::BAD_REQUEST,
        ),
        (
            "POST",
            "/api/move-building-part",
            r#"{"id":99,"parentId":null}"#,
            StatusCode::BAD_REQUEST,
        ),
        (
            "POST",
            "/api/move-device-to-building-part",
            r#"{"deviceId":1,"partId":99}"#,
            StatusCode::BAD_REQUEST,
        ),
        (
            "POST",
            "/api/building-parts",
            r#"{"name":"B","kind":"Building"}"#,
            StatusCode::OK,
        ),
        (
            "DELETE",
            "/api/building-parts/99",
            "",
            StatusCode::BAD_REQUEST,
        ),
        (
            "PATCH",
            "/api/building-parts/99",
            r#"{"name":"x"}"#,
            StatusCode::BAD_REQUEST,
        ),
        (
            "POST",
            "/api/group-ranges",
            r#"{"name":"R","start":"1/0/0","end":"1/7/255"}"#,
            StatusCode::OK,
        ),
        (
            "POST",
            "/api/move-group-range",
            r#"{"id":99,"parentId":null}"#,
            StatusCode::BAD_REQUEST,
        ),
        (
            "DELETE",
            "/api/group-ranges/99",
            "",
            StatusCode::BAD_REQUEST,
        ),
        (
            "PATCH",
            "/api/group-ranges/99",
            r#"{"name":"x"}"#,
            StatusCode::BAD_REQUEST,
        ),
        (
            "POST",
            "/api/group-links",
            r#"{"comObjectId":99,"gaId":99,"direction":"Send"}"#,
            StatusCode::BAD_REQUEST,
        ),
        (
            "DELETE",
            "/api/group-links",
            r#"{"comObjectId":99,"gaId":99,"direction":"Send"}"#,
            StatusCode::BAD_REQUEST,
        ),
        ("GET", "/api/devices", "", StatusCode::OK),
        (
            "POST",
            "/api/devices",
            // A request id is looked up under the lock before the products.
            r#"{"catalogItemId":"x","name":"D","requestId":"r1"}"#,
            StatusCode::BAD_REQUEST,
        ),
        ("DELETE", "/api/devices/1", "", StatusCode::OK),
        (
            "POST",
            "/api/devices/batch-delete",
            r#"{"ids":[99]}"#,
            StatusCode::BAD_REQUEST,
        ),
        (
            "POST",
            "/api/devices/batch-move-line",
            r#"{"deviceIds":[1],"lineId":99}"#,
            StatusCode::BAD_REQUEST,
        ),
        (
            "POST",
            "/api/devices/batch-move-building-part",
            r#"{"deviceIds":[1],"buildingPartId":99}"#,
            StatusCode::BAD_REQUEST,
        ),
        ("GET", "/api/project/functions", "", StatusCode::OK),
        (
            "POST",
            "/api/project/functions/edit",
            r#"{"serverIncarnation":"x","projectIncarnation":0,"snapshotRevision":0,"action":{"kind":"delete","id":99}}"#,
            StatusCode::CONFLICT,
        ),
    ];
    assert_routes_leave_the_runtime_free(routes, router).await;
}

const ITEM: &str = "M-0001_H-1_P-1_CI-1";

/// One catalogue item, enough for the device preview to reach the project.
fn products(dir: &std::path::Path) -> knx_productdb::Connection {
    let conn = knx_productdb::open_and_migrate(&dir.join("p.sqlite")).unwrap();
    conn.execute_batch(
        "INSERT INTO manufacturer (id, name) VALUES ('M-0001', 'M');
         INSERT INTO hardware (id, manufacturer_id, name, has_application_program, source_sha256)
              VALUES ('M-0001_H-1', 'M-0001', 'Power supply', 0, 'x');
         INSERT INTO product (id, manufacturer_id, hardware_id, text, source_sha256)
              VALUES ('M-0001_H-1_P-1', 'M-0001', 'M-0001_H-1', 'Power supply', 'x');
         INSERT INTO catalog_item (id, manufacturer_id, section_id, name, product_ref_id, source_sha256)
              VALUES ('M-0001_H-1_P-1_CI-1', 'M-0001', 'S', 'Power supply', 'M-0001_H-1_P-1', 'x');",
    )
    .unwrap();
    conn
}

/// The device preview reads the products first and only then the project, so
/// it needs a known catalogue item to meet the held lock; the unknown line
/// (99) is refused under it.
#[tokio::test(flavor = "current_thread")]
async fn the_device_preview_leaves_the_runtime_free() {
    let dir = tempfile::tempdir().unwrap();
    let body = format!(r#"{{"catalogItemId":"{ITEM}","name":"D","lineId":99}}"#);
    let routes: &[(&str, &str, &str, StatusCode)] = &[
        (
            "POST",
            "/api/devices/preview",
            &body,
            StatusCode::BAD_REQUEST,
        ),
        ("POST", "/api/devices", &body, StatusCode::BAD_REQUEST),
    ];
    let fresh = std::sync::atomic::AtomicUsize::new(0);
    assert_routes_leave_the_runtime_free_with(
        routes,
        router,
        || {
            let n = fresh.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            let dir = dir.path().join(n.to_string());
            std::fs::create_dir(&dir).unwrap();
            Arc::new(AppState {
                product_db: Some(
                    crate::CatalogueOwner::adopt_opened(products(&dir), Default::default())
                        .unwrap(),
                ),
                ..Default::default()
            })
        },
        None,
    )
    .await;
}

/// Undo, redo and the style change hold the group-address transaction until
/// their mutation is done and published. A client that goes away while the
/// mutation waits for the project must not release it early: a second
/// transaction would then run beside the first and its publication be lost.
#[tokio::test(flavor = "current_thread")]
async fn a_dropped_undo_keeps_the_transaction_until_its_mutation_is_done() {
    let state = Arc::new(AppState::default());
    project_with_device(&state);
    let router = router(&state);
    let (release, holder) = super::tests::hold_project_lock(&state);
    let call = tokio::spawn(router.oneshot(request("POST", "/api/undo", "")));
    tokio::task::yield_now().await;
    tokio::time::sleep(Duration::from_millis(20)).await;
    call.abort();
    assert!(call.await.unwrap_err().is_cancelled());
    assert!(
        state.group_address_style_publication.try_lock().is_err(),
        "the dropped request released the transaction while its mutation waited"
    );
    let _ = release.send(());
    assert!(holder.join().unwrap());
    let deadline = tokio::time::Instant::now() + Duration::from_secs(10);
    while state.group_address_style_publication.try_lock().is_err() {
        assert!(
            tokio::time::Instant::now() < deadline,
            "the transaction never ended"
        );
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
}
