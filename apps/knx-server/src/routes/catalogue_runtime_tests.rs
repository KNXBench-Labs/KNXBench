//! Catalogue reads wait for the product database off the runtime while an install holds it (#36).

use std::sync::mpsc;
use std::sync::Arc;
use std::time::Duration;

use axum::http::StatusCode;
use knx_productdb::managed_catalogue::{LegacyMutationKind, LegacySettlementHint};
use tower::ServiceExt;

use super::editor_runtime_tests::request;
use crate::AppState;

const HARDWARE: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<KNX xmlns="http://knx.org/xml/project/11"><ManufacturerData><Manufacturer RefId="M-1">
<Hardware><Hardware Id="H-1" Name="X" SerialNumber="S" VersionNumber="1">
<Products><Product Id="M-1_P-1" /></Products>
<Hardware2Programs><Hardware2Program Id="H-1_HP-1" MediumTypes="MT-0">
<ApplicationProgramRef RefId="A-1" /></Hardware2Program></Hardware2Programs>
</Hardware></Hardware></Manufacturer></ManufacturerData></KNX>"#;

const CATALOG: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<KNX xmlns="http://knx.org/xml/project/11"><ManufacturerData><Manufacturer RefId="M-1">
<Catalog><CatalogSection Id="M-1_CG-1" Name="Actuators" Number="1" DefaultLanguage="de-DE">
<CatalogItem Id="M-1_CI-1" Name="Schaltaktor" Number="ACT-1" DefaultLanguage="de-DE"
  ProductRefId="M-1_P-1" Hardware2ProgramRefId="H-1_HP-1" />
</CatalogSection></Catalog>
<Languages><Language Identifier="en-US"><TranslationUnit RefId="M-1_CI-1">
<TranslationElement RefId="M-1_CI-1"><Translation AttributeName="Name" Text="Switch actuator" />
</TranslationElement></TranslationUnit></Language></Languages>
</Manufacturer></ManufacturerData></KNX>"#;

/// A product database file with one manufacturer, one catalogue item with an
/// `en-US` name and one listed backup next to it. The directory must outlive
/// the state: the backup listing reads it.
fn with_products() -> (tempfile::TempDir, Arc<AppState>) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("products.sqlite");
    let conn = knx_productdb::open_and_migrate(&path).unwrap();
    knx_productdb::ingest_file(&conn, "M-1/Hardware.xml", HARDWARE.as_bytes()).unwrap();
    knx_productdb::ingest_file(&conn, "M-1/Catalog.xml", CATALOG.as_bytes()).unwrap();
    std::fs::write(dir.path().join(BACKUP), b"backup").unwrap();
    let state = Arc::new(AppState {
        product_db: Some(crate::CatalogueOwner::adopt_opened(conn, path).unwrap()),
        ..Default::default()
    });
    (dir, state)
}

const BACKUP: &str = "products.sqlite.backup-1700000000-0123456789ab.sqlite";

/// The fixture's one catalogue item as the unchanged handler answered it,
/// with the given name and the stored language that answered it.
fn item(name: &str, name_language: &str) -> String {
    format!(
        r#"[{{"id":"M-1_CI-1","manufacturerId":"M-1","name":"{name}","number":"ACT-1","visibleDescription":null,"productRefId":"M-1_P-1","hardware2programRefId":"H-1_HP-1","nameLanguage":{name_language},"visibleDescriptionLanguage":null,"sourceLanguage":"de-DE","details":{{"applicationProgramId":null,"applicationName":null,"applicationVersion":null,"catalogSectionId":"M-1_CG-1","sourceNames":["Catalog.xml"]}}}}]"#
    )
}

/// How the test holds the catalogue owner's mutex: as a read lease, or as an
/// install does (`with_legacy_mutation` for its whole transaction).
#[derive(Clone, Copy, Debug)]
enum Hold {
    Read,
    Install,
}

/// Holds the catalogue owner on a plain thread until released or 10 s pass;
/// answers whether the release came in time.
fn hold_catalogue(
    state: &Arc<AppState>,
    hold: Hold,
) -> (mpsc::Sender<()>, std::thread::JoinHandle<bool>) {
    let (held_tx, held_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel::<()>();
    let holder_state = Arc::clone(state);
    let holder = std::thread::spawn(move || {
        let owner = holder_state.product_db.as_ref().unwrap();
        let wait = move || {
            held_tx.send(()).unwrap();
            Ok::<_, ()>(release_rx.recv_timeout(Duration::from_secs(10)).is_ok())
        };
        match hold {
            Hold::Read => owner.with_read(|_| wait()),
            Hold::Install => owner.with_legacy_mutation(
                LegacyMutationKind::DirectPackage,
                |_| wait(),
                |_| LegacySettlementHint::SuccessfulReturn,
            ),
        }
        .unwrap()
        .unwrap()
    });
    held_rx.recv().unwrap();
    (release_tx, holder)
}

/// The backup listing's entry minus its timestamp and file identity, which
/// differ per run.
fn without_file_identity(body: &str) -> serde_json::Value {
    let mut value: serde_json::Value = serde_json::from_str(body).unwrap();
    for entry in value["backups"].as_array_mut().unwrap() {
        let entry = entry.as_object_mut().unwrap();
        assert!(entry.remove("modifiedUnixSeconds").unwrap().is_u64());
        assert!(entry.remove("identity").unwrap().is_string());
    }
    value
}

/// Each read meets the catalogue held by a read lease and by an install. On
/// this single-threaded runtime a read run inline blocks the timer until the
/// holder gives up; one answered while the mutex is still held did not wait
/// for it. Bodies are the unchanged handlers' answers, recorded once.
#[tokio::test(flavor = "current_thread")]
async fn catalogue_reads_leave_the_runtime_free() {
    let items = item("Schaltaktor", "null");
    let english = item("Switch actuator", r#""en-US""#);
    let backups = format!(r#"{{"backups":[{{"name":"{BACKUP}","bytes":6}}]}}"#);
    let routes: [(&str, &str); 7] = [
        (
            "/api/catalog/manufacturers",
            r#"[{"id":"M-1","name":null}]"#,
        ),
        ("/api/catalog/items", &items),
        ("/api/catalog/items?manufacturer=M-1&search=schalt", &items),
        ("/api/catalog/items?language=en-US", &english),
        // A bare language falls back to the stored `en-US`.
        ("/api/catalog/items?language=en", &english),
        (
            "/api/product-languages",
            r#"[{"language":"en-US","rows":1}]"#,
        ),
        ("/api/catalog/backups", &backups),
    ];
    let mut blocked = Vec::new();
    let mut before_the_lock = Vec::new();
    for hold in [Hold::Read, Hold::Install] {
        for (uri, expected) in routes {
            let (_dir, state) = with_products();
            let router = crate::app(Arc::clone(&state), None);
            let (release, holder) = hold_catalogue(&state, hold);
            let call = tokio::spawn(async move {
                let response = router.oneshot(request("GET", uri, "")).await.unwrap();
                let status = response.status();
                let body = axum::body::to_bytes(response.into_body(), usize::MAX)
                    .await
                    .unwrap();
                (status, String::from_utf8(body.to_vec()).unwrap())
            });
            tokio::task::yield_now().await;
            tokio::time::sleep(Duration::from_millis(20)).await;
            if call.is_finished() {
                before_the_lock.push(format!("{hold:?} {uri}"));
            }
            let _ = release.send(());
            if !holder.join().unwrap() {
                blocked.push(format!("{hold:?} {uri}"));
            }
            let (status, body) = call.await.unwrap();
            assert_eq!(status, StatusCode::OK, "{hold:?} {uri}: {body}");
            if uri == "/api/catalog/backups" {
                assert_eq!(
                    without_file_identity(&body),
                    serde_json::from_str::<serde_json::Value>(expected).unwrap(),
                    "{hold:?} {uri}"
                );
            } else {
                assert_eq!(body, expected, "{hold:?} {uri}");
            }
        }
    }
    assert!(
        blocked.is_empty(),
        "these reads blocked the runtime: {blocked:?}"
    );
    assert!(
        before_the_lock.is_empty(),
        "these answered without waiting for the catalogue: {before_the_lock:?}"
    );
}

/// The refusals the unchanged handlers gave without a product database and
/// with a poisoned owner, recorded once: the move kept statuses and texts.
#[tokio::test(flavor = "current_thread")]
async fn catalogue_read_refusals_are_unchanged() {
    let missing = r#"{"error":"no product database configured"}"#;
    let unhealthy = "catalogue owner not healthy: Quarantined(Poisoned)";
    let unavailable = format!(r#"{{"error":"the product database is unavailable: {unhealthy}"}}"#);
    let owner = format!(r#"{{"error":"catalogue owner unavailable: {unhealthy}"}}"#);

    let bare = Arc::new(AppState::default());
    let (_dir, poisoned) = with_products();
    let holder = Arc::clone(&poisoned);
    let poisoning = std::thread::spawn(move || {
        let _ = holder
            .product_db
            .as_ref()
            .unwrap()
            .with_read(|_| -> Result<(), ()> { panic!("poison the catalogue owner") });
    })
    .join();
    assert!(poisoning.is_err(), "the holder must have panicked");

    let cases: [(&Arc<AppState>, &str, StatusCode, &str); 8] = [
        (
            &bare,
            "/api/catalog/manufacturers",
            StatusCode::BAD_REQUEST,
            missing,
        ),
        (
            &bare,
            "/api/catalog/items",
            StatusCode::BAD_REQUEST,
            missing,
        ),
        (&bare, "/api/product-languages", StatusCode::OK, "[]"),
        (
            &bare,
            "/api/catalog/backups",
            StatusCode::SERVICE_UNAVAILABLE,
            missing,
        ),
        (
            &poisoned,
            "/api/catalog/manufacturers",
            StatusCode::BAD_REQUEST,
            &unavailable,
        ),
        (
            &poisoned,
            "/api/catalog/items",
            StatusCode::BAD_REQUEST,
            &unavailable,
        ),
        (
            &poisoned,
            "/api/product-languages",
            StatusCode::BAD_REQUEST,
            &unavailable,
        ),
        (
            &poisoned,
            "/api/catalog/backups",
            StatusCode::INTERNAL_SERVER_ERROR,
            &owner,
        ),
    ];
    for (state, uri, status, expected) in cases {
        let response = crate::app(Arc::clone(state), None)
            .oneshot(request("GET", uri, ""))
            .await
            .unwrap();
        assert_eq!(response.status(), status, "{uri}");
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        assert_eq!(String::from_utf8(body.to_vec()).unwrap(), expected, "{uri}");
    }
}
