//! Catalogue update requests keep owner waits off the runtime and finish detached mutations.

use std::sync::{mpsc, Arc};
use std::time::Duration;

use axum::body::Body;
use axum::http::Request;
use tower::ServiceExt;

use super::*;
use crate::AppState;

const BACKUP: &str = "products.sqlite.backup-1700000000-0123456789ab.sqlite";

fn fixture() -> (tempfile::TempDir, Arc<AppState>) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("products.sqlite");
    let conn = knx_productdb::open_and_migrate(&path).unwrap();
    std::fs::write(dir.path().join(BACKUP), b"backup").unwrap();
    let state = Arc::new(AppState {
        product_db: Some(crate::CatalogueOwner::adopt_opened(conn, path).unwrap()),
        ..Default::default()
    });
    (dir, state)
}

#[derive(Clone, Copy, Debug)]
enum Route {
    Preview,
    Apply,
    Restore,
    Delete,
}

fn upload(route: Route, token: Option<&str>) -> Request<Body> {
    let uri = match route {
        Route::Preview => "/api/catalog/update/preview",
        Route::Apply => "/api/catalog/update/apply",
        _ => unreachable!(),
    };
    let token = token.map_or(String::new(), |token| {
        format!("--update-boundary\r\nContent-Disposition: form-data; name=\"token\"\r\n\r\n{token}\r\n")
    });
    let body = format!("{token}--update-boundary\r\nContent-Disposition: form-data; name=\"file\"; filename=\"old.vd4\"\r\n\r\nnot a zip\r\n--update-boundary--\r\n");
    Request::builder()
        .method("POST")
        .uri(uri)
        .header(
            "content-type",
            "multipart/form-data; boundary=update-boundary",
        )
        .body(Body::from(body))
        .unwrap()
}

fn json_request(uri: &str, body: serde_json::Value) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri(uri)
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .unwrap()
}

fn route_request(route: Route, dir: &Path) -> Request<Body> {
    match route {
        Route::Preview | Route::Apply => upload(route, Some("preview-token")),
        Route::Restore => json_request("/api/catalog/restore", serde_json::json!({"name": BACKUP})),
        Route::Delete => json_request(
            "/api/catalog/backups/delete",
            serde_json::json!({
                "name": BACKUP,
                "identity": file_identity(&std::fs::metadata(dir.join(BACKUP)).unwrap()),
                "confirmed": true
            }),
        ),
    }
}

async fn send(router: axum::Router, request: Request<Body>) -> (StatusCode, String) {
    let response = router.oneshot(request).await.unwrap();
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    (status, String::from_utf8(bytes.to_vec()).unwrap())
}

#[derive(Clone, Copy, Debug)]
enum Hold {
    Read,
    Mutation,
}

fn hold_owner(
    state: &Arc<AppState>,
    hold: Hold,
) -> (mpsc::Sender<()>, std::thread::JoinHandle<bool>) {
    let (held_tx, held_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let state = Arc::clone(state);
    let thread = std::thread::spawn(move || {
        let owner = state.product_db.as_ref().unwrap();
        let wait = || {
            held_tx.send(()).unwrap();
            Ok::<_, ()>(release_rx.recv_timeout(Duration::from_secs(10)).is_ok())
        };
        match hold {
            Hold::Read => owner.with_read(|_| wait()),
            Hold::Mutation => owner.with_legacy_mutation(
                LegacyMutationKind::DirectPackage,
                |_| wait(),
                |_| LegacySettlementHint::SuccessfulReturn,
            ),
        }
        .unwrap()
        .unwrap()
    });
    held_rx.recv().unwrap();
    (release_tx, thread)
}

// Exact bodies are pinned from the unchanged handlers in a base archive.
const PACKAGE_REFUSAL: &str = r#"{"error":"legacy ETS filename extension .vd4 is unsupported; legacy import is not implemented","kind":"legacyProductDatabase"}"#;
const RESTORE_REFUSAL: &str = r#"{"error":"not a usable product database backup: file is not a database","kind":"invalidBackup"}"#;

async fn witness(route: Route, status: StatusCode, expected: &str) {
    let mut blocked = Vec::new();
    let mut early = Vec::new();
    for hold in [Hold::Read, Hold::Mutation] {
        let (dir, state) = fixture();
        let live = std::fs::read(dir.path().join("products.sqlite")).unwrap();
        let request = route_request(route, dir.path());
        let router = crate::app(Arc::clone(&state), None);
        let (release, holder) = hold_owner(&state, hold);
        let call = tokio::spawn(send(router, request));
        tokio::task::yield_now().await;
        tokio::time::sleep(Duration::from_millis(20)).await;
        if call.is_finished() {
            early.push(format!("{hold:?} {route:?}"));
        }
        let _ = release.send(());
        if !holder.join().unwrap() {
            blocked.push(format!("{hold:?} {route:?}"));
        }
        let (actual_status, body) = call.await.unwrap();
        assert_eq!(actual_status, status, "{hold:?} {route:?}: {body}");
        assert_eq!(body, expected, "{hold:?} {route:?}");
        assert_eq!(
            std::fs::read(dir.path().join("products.sqlite")).unwrap(),
            live
        );
        assert_eq!(
            dir.path().join(BACKUP).exists(),
            !matches!(route, Route::Delete),
            "only a successful deletion removes the backup"
        );
        assert!(state
            .product_db
            .as_ref()
            .unwrap()
            .try_read(|_| Ok::<_, ()>(()))
            .unwrap()
            .is_some());
    }
    assert!(
        blocked.is_empty(),
        "these updates blocked the runtime: {blocked:?}"
    );
    assert!(
        early.is_empty(),
        "these answered before the owner lock: {early:?}"
    );
}

#[tokio::test(flavor = "current_thread")]
async fn preview_leaves_the_runtime_free() {
    witness(
        Route::Preview,
        StatusCode::UNPROCESSABLE_ENTITY,
        PACKAGE_REFUSAL,
    )
    .await;
}

#[tokio::test(flavor = "current_thread")]
async fn apply_leaves_the_runtime_free() {
    witness(
        Route::Apply,
        StatusCode::UNPROCESSABLE_ENTITY,
        PACKAGE_REFUSAL,
    )
    .await;
}

#[tokio::test(flavor = "current_thread")]
async fn restore_leaves_the_runtime_free() {
    witness(
        Route::Restore,
        StatusCode::UNPROCESSABLE_ENTITY,
        RESTORE_REFUSAL,
    )
    .await;
}

#[tokio::test(flavor = "current_thread")]
async fn delete_leaves_the_runtime_free() {
    witness(
        Route::Delete,
        StatusCode::OK,
        &format!(r#"{{"deleted":"{BACKUP}"}}"#),
    )
    .await;
}

#[tokio::test(flavor = "current_thread")]
async fn missing_apply_tokens_are_refused_before_the_job() {
    for token in [None, Some("")] {
        let (_dir, state) = fixture();
        let router = crate::app(Arc::clone(&state), None);
        let (release, holder) = hold_owner(&state, Hold::Read);
        let response = tokio::time::timeout(
            Duration::from_secs(1),
            send(router, upload(Route::Apply, token)),
        )
        .await;
        let _ = release.send(());
        assert!(holder.join().unwrap());
        assert_eq!(
            response.unwrap(),
            (
                StatusCode::BAD_REQUEST,
                r#"{"error":"a catalogue update needs the preview's token"}"#.to_owned()
            )
        );
    }
}

#[tokio::test(flavor = "current_thread")]
async fn update_owner_and_backup_refusals_are_unchanged() {
    let (dir, state) = fixture();
    let bare = Arc::new(AppState::default());
    let (_poison_dir, poisoned) = fixture();
    let poison = Arc::clone(&poisoned);
    assert!(std::thread::spawn(move || {
        let _ = poison
            .product_db
            .as_ref()
            .unwrap()
            .with_read(|_| -> Result<(), ()> { panic!("poison the catalogue owner") });
    })
    .join()
    .is_err());
    for (state, status, body) in [
        (
            &bare,
            StatusCode::SERVICE_UNAVAILABLE,
            r#"{"error":"no product database configured"}"#,
        ),
        (
            &poisoned,
            StatusCode::INTERNAL_SERVER_ERROR,
            r#"{"error":"catalogue owner unavailable: catalogue owner not healthy: Quarantined(Poisoned)"}"#,
        ),
    ] {
        for route in [Route::Preview, Route::Apply, Route::Restore, Route::Delete] {
            assert_eq!(
                send(
                    crate::app(Arc::clone(state), None),
                    route_request(route, dir.path())
                )
                .await,
                (status, body.to_owned()),
                "{route:?}"
            );
        }
    }
    let live = std::fs::read(dir.path().join("products.sqlite")).unwrap();
    for (uri, body, status, expected) in [
        (
            "/api/catalog/restore",
            serde_json::json!({"name":"unlisted"}),
            StatusCode::BAD_REQUEST,
            r#"{"error":"not a product database backup listed by this server","kind":"unknownBackup"}"#,
        ),
        (
            "/api/catalog/backups/delete",
            serde_json::json!({"name":BACKUP,"identity":"wrong","confirmed":false}),
            StatusCode::BAD_REQUEST,
            r#"{"error":"deleting a catalogue backup needs an explicit confirmation","kind":"confirmationRequired"}"#,
        ),
        (
            "/api/catalog/backups/delete",
            serde_json::json!({"name":"unlisted","identity":"wrong","confirmed":true}),
            StatusCode::BAD_REQUEST,
            r#"{"error":"not a product database backup listed by this server","kind":"unknownBackup"}"#,
        ),
        (
            "/api/catalog/backups/delete",
            serde_json::json!({"name":BACKUP,"identity":"wrong","confirmed":true}),
            StatusCode::CONFLICT,
            r#"{"error":"this backup changed since it was listed; list the backups again","kind":"catalogueBackupChanged"}"#,
        ),
    ] {
        assert_eq!(
            send(
                crate::app(Arc::clone(&state), None),
                json_request(uri, body)
            )
            .await,
            (status, expected.to_owned())
        );
    }
    assert_eq!(
        std::fs::read(dir.path().join("products.sqlite")).unwrap(),
        live
    );
    assert!(dir.path().join(BACKUP).exists());
}

// A per-state observer fires from inside the real blocking closure, not when
// its join handle is created. It does not stall work or change lock ordering.
// Completion is observed only after the closure's response inputs and managed
// settlement are done, independently of whether the HTTP future still exists.
struct ObserverSenders {
    started: tokio::sync::oneshot::Sender<bool>,
    finished: tokio::sync::oneshot::Sender<()>,
}

static DELETE_OBSERVERS: std::sync::Mutex<Vec<(usize, ObserverSenders)>> =
    std::sync::Mutex::new(Vec::new());

pub(super) struct DeleteObserver(Option<tokio::sync::oneshot::Sender<()>>);

impl Drop for DeleteObserver {
    fn drop(&mut self) {
        if let Some(finished) = self.0.take() {
            let _ = finished.send(());
        }
    }
}

pub(super) fn observe_delete(state: &Arc<AppState>) -> Option<DeleteObserver> {
    let key = Arc::as_ptr(state) as usize;
    let observer = {
        let mut observers = DELETE_OBSERVERS.lock().unwrap();
        let index = observers.iter().position(|(id, _)| *id == key)?;
        observers.swap_remove(index).1
    };
    let busy = state
        .product_db
        .as_ref()
        .unwrap()
        .try_read(|_| Ok::<_, ()>(()))
        .unwrap()
        .is_none();
    let _ = observer.started.send(busy);
    Some(DeleteObserver(Some(observer.finished)))
}

#[tokio::test(flavor = "current_thread")]
async fn a_dropped_delete_finishes_its_started_job_and_settlement() {
    let (dir, state) = fixture();
    let live = std::fs::read(dir.path().join("products.sqlite")).unwrap();
    let request = route_request(Route::Delete, dir.path());
    let router = crate::app(Arc::clone(&state), None);
    let (release, holder) = hold_owner(&state, Hold::Read);
    let (started_tx, started_rx) = tokio::sync::oneshot::channel();
    let (finished_tx, finished_rx) = tokio::sync::oneshot::channel();
    DELETE_OBSERVERS.lock().unwrap().push((
        Arc::as_ptr(&state) as usize,
        ObserverSenders {
            started: started_tx,
            finished: finished_tx,
        },
    ));
    let call = tokio::spawn(send(router, request));
    let busy = tokio::time::timeout(Duration::from_secs(10), started_rx)
        .await
        .unwrap()
        .unwrap();
    // The mounted request has reached the blocking closure and that closure
    // has observed the actual owner busy. No sleep or enqueue acknowledgement.
    call.abort();
    assert!(call.await.unwrap_err().is_cancelled());
    let kept_while_held = dir.path().join(BACKUP).exists();
    let still_held = state
        .product_db
        .as_ref()
        .unwrap()
        .try_read(|_| Ok::<_, ()>(()))
        .unwrap()
        .is_none();
    let _ = release.send(());
    let released_in_time = holder.join().unwrap();
    tokio::time::timeout(Duration::from_secs(10), finished_rx)
        .await
        .unwrap()
        .unwrap();
    assert!(released_in_time, "the request blocked the runtime");
    assert!(
        busy && still_held && kept_while_held,
        "the started job must wait for the real owner"
    );
    assert!(
        !dir.path().join(BACKUP).exists(),
        "a started mutation survives its HTTP waiter"
    );
    assert_eq!(
        std::fs::read(dir.path().join("products.sqlite")).unwrap(),
        live
    );
    let owner = state.product_db.as_ref().unwrap();
    // Reads are refused by an unsettled/unhealthy managed owner. A successful
    // nonblocking lease proves health and that the blocking job released it.
    assert!(
        owner
            .try_read(|conn| {
                conn.query_row("SELECT count(*) FROM sqlite_master", [], |row| {
                    row.get::<_, i64>(0)
                })
            })
            .unwrap()
            .unwrap()
            .unwrap()
            > 0
    );
    owner
        .with_legacy_mutation(
            LegacyMutationKind::BackupDelete,
            |_| Ok::<_, ()>(()),
            |_| LegacySettlementHint::SuccessfulReturn,
        )
        .unwrap()
        .unwrap();
}
