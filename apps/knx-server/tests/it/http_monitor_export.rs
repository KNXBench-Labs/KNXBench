//! Monitoring export must read the reviewed live snapshot, never the saved root.

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use knx_core::{
    Command, CompletionStatus, GroupAddress, GroupAddressEntry, GroupAddressId, Installation,
    InstallationId, Language, Project, SourceRef, Topology,
};
use serde_json::{json, Value};
use std::sync::{atomic::Ordering, Arc};
use tower::ServiceExt;

const PRODUCT_HARDWARE: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<KNX xmlns="http://knx.org/xml/project/11"><ManufacturerData><Manufacturer RefId="M-1">
<Hardware><Hardware Id="H-1" Name="Hardware" SerialNumber="S" VersionNumber="1">
<Products><Product Id="M-1_P-1" Text="Initial catalogue name" OrderNumber="ORD-1" /></Products>
<Hardware2Programs><Hardware2Program Id="H-1_HP-1" MediumTypes="MT-0">
<ApplicationProgramRef RefId="A-1" /></Hardware2Program></Hardware2Programs>
</Hardware></Hardware></Manufacturer></ManufacturerData></KNX>"#;

#[tokio::test]
async fn catalogue_changes_invalidate_bundle_without_revision_and_export_never_writes_products() {
    use knx_core::{CommissioningState, DeviceId, DeviceInstance, IndividualAddress};
    use std::sync::Mutex;
    let dir = tempfile::tempdir().unwrap();
    let state = setup(dir.path());
    {
        let mut project = state.project.lock().unwrap();
        let project = project.as_mut().unwrap();
        project.installations[0]
            .topology
            .unassigned
            .push(DeviceId(1));
        project.devices.insert(DeviceInstance {
            id: DeviceId(1),
            source: SourceRef {
                path: "fictional-device".into(),
                ets_id: "fictional-device".into(),
            },
            name: "Current authored device".into(),
            description: None,
            address: Some(IndividualAddress::new(1, 1, 1).unwrap()),
            product_ref: "M-1_P-1".into(),
            program_ref: "H-1_HP-1".into(),
            commissioning: CommissioningState::default(),
            visibility_calculated: true,
            com_objects: vec![],
            binary_data: vec![],
        });
    }
    let products = knx_productdb::open_and_migrate(&dir.path().join("catalogue.sqlite")).unwrap();
    knx_productdb::ingest_file(&products, "M-1/Hardware.xml", PRODUCT_HARDWARE.as_bytes()).unwrap();
    assert_eq!(
        knx_productdb::query::device_product(&products, "M-1_P-1", "H-1_HP-1", None)
            .unwrap()
            .unwrap()
            .product_text
            .as_deref(),
        Some("Initial catalogue name")
    );
    products.execute_batch("PRAGMA query_only = ON").unwrap();
    let mut state =
        Arc::try_unwrap(state).unwrap_or_else(|_| panic!("fixture unexpectedly shared"));
    state.product_db = Some(Mutex::new(products));
    let state = Arc::new(state);
    let before = state.project.lock().unwrap().clone();
    let app = knx_server::app(state.clone(), None);
    let input = binding(&state);
    let changes = || {
        state
            .product_db
            .as_ref()
            .unwrap()
            .lock()
            .unwrap()
            .query_row("SELECT total_changes()", [], |row| row.get::<_, i64>(0))
            .unwrap()
    };
    let writes_before = changes();
    let preview = preview_ok(&app, &input).await;
    let request = reviewed_download(&input, &preview);
    assert_eq!(
        post(&app, DOWNLOAD, request.clone()).await.status(),
        StatusCode::OK
    );
    assert_eq!(changes(), writes_before, "export changed catalogue");
    {
        let products = state.product_db.as_ref().unwrap().lock().unwrap();
        products.execute_batch("PRAGMA query_only = OFF").unwrap();
        // Re-ingest deliberately preserves first-admitted product winners; change
        // the actual synthetic catalogue row to model a catalogue update instead.
        assert_eq!(
            products
                .execute(
                    "UPDATE product SET text = ?1 WHERE id = ?2",
                    ["Updated catalogue name", "M-1_P-1"],
                )
                .unwrap(),
            1
        );
        assert_eq!(
            knx_productdb::query::device_product(&products, "M-1_P-1", "H-1_HP-1", None)
                .unwrap()
                .unwrap()
                .product_text
                .as_deref(),
            Some("Updated catalogue name")
        );
        products.execute_batch("PRAGMA query_only = ON").unwrap();
    }
    assert_eq!(binding(&state), input);
    let writes_after_update = changes();
    let changed_preview = preview_ok(&app, &input).await;
    assert_ne!(
        changed_preview["bundleSha256"], preview["bundleSha256"],
        "catalogue-derived output did not change"
    );
    assert_eq!(
        post(&app, DOWNLOAD, request).await.status(),
        StatusCode::CONFLICT
    );
    assert_eq!(changes(), writes_after_update);
    assert_eq!(*state.project.lock().unwrap(), before);
}

#[tokio::test]
async fn poisoned_product_lock_is_refused_instead_of_silently_omitting_catalogue() {
    use std::sync::Mutex;
    let dir = tempfile::tempdir().unwrap();
    let state = setup(dir.path());
    let products = knx_productdb::open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
    let mut state =
        Arc::try_unwrap(state).unwrap_or_else(|_| panic!("fixture unexpectedly shared"));
    state.product_db = Some(Mutex::new(products));
    let state = Arc::new(state);
    let poisoner = state.clone();
    assert!(std::thread::spawn(move || {
        let _guard = poisoner.product_db.as_ref().unwrap().lock().unwrap();
        panic!("fictional product lock poisoning");
    })
    .join()
    .is_err());
    let before = state.project.lock().unwrap().clone();
    let app = knx_server::app(state.clone(), None);
    assert_eq!(
        post(&app, PREVIEW, binding(&state)).await.status(),
        StatusCode::INTERNAL_SERVER_ERROR
    );
    assert_eq!(*state.project.lock().unwrap(), before);
}

#[tokio::test]
async fn oversized_snapshot_admission_precedes_catalogue_access() {
    let dir = tempfile::tempdir().unwrap();
    let state = setup(dir.path());
    let mut state =
        Arc::try_unwrap(state).unwrap_or_else(|_| panic!("fixture unexpectedly shared"));
    state.product_db_unavailable = Some("fictional catalogue open failure".into());
    state.project.lock().unwrap().as_mut().unwrap().info.name = "x".repeat(32 * 1024 * 1024 + 1);
    let state = Arc::new(state);
    let app = knx_server::app(state.clone(), None);
    let reply = post(&app, PREVIEW, binding(&state)).await;
    assert_eq!(
        reply.status(),
        StatusCode::UNPROCESSABLE_ENTITY,
        "snapshot input admission must run before clone/catalogue work"
    );
}

#[tokio::test]
async fn strict_request_refuses_non_u8_installation_and_unknown_fields() {
    let dir = tempfile::tempdir().unwrap();
    let state = setup(dir.path());
    let app = knx_server::app(state.clone(), None);
    for value in [json!(-1), json!(256), json!(1.5), json!("1")] {
        let mut input = binding(&state);
        input["installation"] = value;
        assert_eq!(
            post(&app, PREVIEW, input).await.status(),
            StatusCode::BAD_REQUEST
        );
    }
    let mut unknown = binding(&state);
    unknown["path"] = json!("must-not-write.zip");
    assert_eq!(
        post(&app, PREVIEW, unknown).await.status(),
        StatusCode::BAD_REQUEST
    );
    let mut former_contract = binding(&state);
    former_contract["bundleSha256"] = json!("0".repeat(64));
    former_contract["projectSha256"] = json!("0".repeat(64));
    assert_eq!(
        post(&app, DOWNLOAD, former_contract).await.status(),
        StatusCode::BAD_REQUEST,
        "download contract is exactly binding/options plus bundleSha256"
    );
}

const PREVIEW: &str = "/api/project/monitor-export/preview";
const DOWNLOAD: &str = "/api/project/monitor-export/download";

fn setup(dir: &std::path::Path) -> Arc<knx_server::AppState> {
    let state = Arc::new(knx_server::AppState::new(dir.into()));
    let mut project = Project::new(Language("en".into()));
    project.info.name = "Live edited monitoring snapshot".into();
    project.installations.push(Installation {
        id: InstallationId(1),
        name: "Installation".into(),
        default_line: None,
        multicast_address: None,
        completion: CompletionStatus::FinishedDesign,
        topology: Topology {
            areas: vec![],
            lines: vec![],
            unassigned: vec![],
        },
        buildings: vec![],
        group_ranges: vec![],
        group_addresses: vec![],
        parameters: vec![],
    });
    project.installations[0]
        .group_addresses
        .push(GroupAddressEntry {
            id: GroupAddressId(1),
            source: SourceRef {
                path: "fictional-group".into(),
                ets_id: "fictional-group".into(),
            },
            name: "Saved light name".into(),
            address: GroupAddress::parse("1/1/1", project.info.group_address_style).unwrap(),
            central: false,
            unfiltered: false,
            range: None,
            declared_dpt: Default::default(),
        });
    project.notes.push(
        knx_core::note::ProjectNote::new(
            project.ids.next_project_note_id().unwrap(),
            knx_core::note::ProjectNoteTarget::Project,
            "Private project note".into(),
            "fictional-content-not-for-monitor-export".into(),
        )
        .unwrap(),
    );
    let saved = dir.join("source.knxdb");
    knx_server::save_project_as_impl(&saved, &project, &[], &[]).unwrap();
    *state.clean_project.lock().unwrap() = Some(project.clone());
    *state.store_path.lock().unwrap() = Some(saved);
    {
        let mut commands = state.command_stack.lock().unwrap();
        commands
            .do_command(
                &mut project,
                Command::RenameGroupAddress {
                    id: GroupAddressId(1),
                    name: "Live edited light name".into(),
                },
            )
            .unwrap();
    }
    state.project_revision.store(7, Ordering::Relaxed);
    state.project_incarnation.store(3, Ordering::Relaxed);
    *state.project.lock().unwrap() = Some(project);
    state
}

fn binding(state: &knx_server::AppState) -> Value {
    json!({
        "serverIncarnation": state.server_incarnation,
        "projectIncarnation": state.project_incarnation.load(Ordering::Relaxed),
        "projectRevision": state.project_revision.load(Ordering::Relaxed),
    })
}

async fn post(app: &axum::Router, uri: &str, input: Value) -> axum::response::Response {
    app.clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(uri)
                .header("content-type", "application/json")
                .body(Body::from(input.to_string()))
                .unwrap(),
        )
        .await
        .unwrap()
}

async fn json_body(reply: axum::response::Response) -> Value {
    serde_json::from_slice(
        &axum::body::to_bytes(reply.into_body(), 1024 * 1024)
            .await
            .unwrap(),
    )
    .unwrap()
}

#[tokio::test]
async fn preview_route_exports_current_snapshot() {
    let dir = tempfile::tempdir().unwrap();
    let state = setup(dir.path());
    let app = knx_server::app(state.clone(), None);
    let reply = post(&app, PREVIEW, binding(&state)).await;
    assert_eq!(
        reply.status(),
        StatusCode::OK,
        "monitor preview route unavailable"
    );
    assert_eq!(reply.headers()["cache-control"], "no-store");
    assert_eq!(reply.headers()["x-content-type-options"], "nosniff");
    let preview = json_body(reply).await;
    assert_eq!(preview["binding"], binding(&state));
    assert_eq!(preview["report"]["profile"], "monitoring-sparse-v1");
    assert!(preview["bundleSha256"]
        .as_str()
        .is_some_and(|hash| hash.len() == 64));
}

#[tokio::test]
async fn download_route_requires_reviewed_hashes() {
    let dir = tempfile::tempdir().unwrap();
    let state = setup(dir.path());
    let app = knx_server::app(state.clone(), None);
    let mut input = binding(&state);
    input["bundleSha256"] = json!("0".repeat(64));
    assert_eq!(
        post(&app, DOWNLOAD, input).await.status(),
        StatusCode::CONFLICT,
        "monitor download route unavailable or does not bind exact bytes"
    );
}

fn reviewed_download(input: &Value, preview: &Value) -> Value {
    let mut request = input.clone();
    request["bundleSha256"] = preview["bundleSha256"].clone();
    request
}

async fn preview_ok(app: &axum::Router, input: &Value) -> Value {
    let reply = post(app, PREVIEW, input.clone()).await;
    let status = reply.status();
    let body = json_body(reply).await;
    assert_eq!(status, StatusCode::OK, "preview error: {}", body["error"]);
    body
}

#[tokio::test]
async fn exact_zip_report_hashes_live_edits_and_undo_are_unchanged() {
    use sha2::{Digest, Sha256};
    use std::io::Read;
    let dir = tempfile::tempdir().unwrap();
    let state = setup(dir.path());
    let app = knx_server::app(state.clone(), None);
    let before = state.project.lock().unwrap().clone();
    let clean = state.clean_project.lock().unwrap().clone();
    let stack = format!("{:?}", state.command_stack.lock().unwrap());
    let path = state.store_path.lock().unwrap().clone().unwrap();
    let file_before = std::fs::read(&path).unwrap();
    let mut input = binding(&state);
    input["installation"] = json!(1);
    input["language"] = json!("de-DE");
    let preview = preview_ok(&app, &input).await;
    assert_eq!(preview["binding"], binding(&state));
    assert_eq!(
        preview_ok(&app, &input).await,
        preview,
        "preview is time-independent"
    );
    let request = reviewed_download(&input, &preview);
    assert!(!preview
        .to_string()
        .contains("fictional-content-not-for-monitor-export"));
    let reply = post(&app, DOWNLOAD, request.clone()).await;
    assert_eq!(reply.status(), StatusCode::OK);
    assert_eq!(reply.headers()["content-type"], "application/zip");
    assert_eq!(
        reply.headers()["content-disposition"],
        "attachment; filename=\"monitoring-export.zip\""
    );
    assert_eq!(reply.headers()["cache-control"], "no-store");
    assert_eq!(reply.headers()["x-content-type-options"], "nosniff");
    let bytes = axum::body::to_bytes(reply.into_body(), 64 * 1024 * 1024)
        .await
        .unwrap();
    assert_eq!(
        format!("{:x}", Sha256::digest(&bytes)),
        preview["bundleSha256"]
    );
    let again = post(&app, DOWNLOAD, request).await;
    assert_eq!(again.status(), StatusCode::OK);
    assert_eq!(
        axum::body::to_bytes(again.into_body(), 64 * 1024 * 1024)
            .await
            .unwrap(),
        bytes
    );
    let mut zip = zip::ZipArchive::new(std::io::Cursor::new(bytes)).unwrap();
    assert_eq!(zip.len(), 3);
    let mut project_bytes = vec![];
    zip.by_name("project.json")
        .unwrap()
        .read_to_end(&mut project_bytes)
        .unwrap();
    assert_eq!(
        format!("{:x}", Sha256::digest(&project_bytes)),
        preview["report"]["project_sha256"]
    );
    assert!(!String::from_utf8_lossy(&project_bytes)
        .contains("fictional-content-not-for-monitor-export"));
    let project: Value = serde_json::from_slice(&project_bytes).unwrap();
    assert_eq!(project["info"]["name"], "Live edited monitoring snapshot");
    assert_eq!(
        project["group_addresses"]["1/1/1"]["name"],
        "Live edited light name"
    );
    let mut report_bytes = vec![];
    zip.by_name("export-report.json")
        .unwrap()
        .read_to_end(&mut report_bytes)
        .unwrap();
    assert_eq!(
        serde_json::from_slice::<Value>(&report_bytes).unwrap(),
        preview["report"]
    );
    assert_eq!(*state.project.lock().unwrap(), before);
    assert_eq!(*state.clean_project.lock().unwrap(), clean);
    assert_eq!(format!("{:?}", state.command_stack.lock().unwrap()), stack);
    assert_eq!(
        state.command_stack.lock().unwrap().history_lengths(),
        (1, 0)
    );
    assert_eq!(std::fs::read(&path).unwrap(), file_before);
    assert_eq!(binding(&state), preview["binding"]);
}

#[tokio::test]
async fn all_binding_dimensions_stale_partial_unknown_and_hash_changes_refuse() {
    let dir = tempfile::tempdir().unwrap();
    let state = setup(dir.path());
    let app = knx_server::app(state.clone(), None);
    let input = binding(&state);
    let preview = preview_ok(&app, &input).await;
    let download = reviewed_download(&input, &preview);
    let before = state.project.lock().unwrap().clone();
    let stack = format!("{:?}", state.command_stack.lock().unwrap());
    for endpoint in [PREVIEW, DOWNLOAD] {
        let base = if endpoint == PREVIEW {
            &input
        } else {
            &download
        };
        for (key, value) in [
            ("serverIncarnation", json!("different-server")),
            ("projectIncarnation", json!(999)),
            ("projectRevision", json!(999)),
        ] {
            let mut changed = base.clone();
            changed[key] = value;
            assert_eq!(
                post(&app, endpoint, changed).await.status(),
                StatusCode::CONFLICT,
                "{key}"
            );
        }
        for key in ["serverIncarnation", "projectIncarnation", "projectRevision"] {
            let mut partial = base.clone();
            partial.as_object_mut().unwrap().remove(key);
            assert_eq!(
                post(&app, endpoint, partial).await.status(),
                StatusCode::BAD_REQUEST,
                "{key}"
            );
        }
        let mut unknown = base.clone();
        unknown["snapshotRevision"] = json!(7);
        assert_eq!(
            post(&app, endpoint, unknown).await.status(),
            StatusCode::BAD_REQUEST
        );
    }
    {
        let key = "bundleSha256";
        let mut altered = download.clone();
        altered[key] = json!("0".repeat(64));
        assert_eq!(
            post(&app, DOWNLOAD, altered).await.status(),
            StatusCode::CONFLICT
        );
        let mut missing = download.clone();
        missing.as_object_mut().unwrap().remove(key);
        assert_eq!(
            post(&app, DOWNLOAD, missing).await.status(),
            StatusCode::BAD_REQUEST
        );
        let mut malformed = download.clone();
        malformed[key] = json!("not-a-sha256");
        assert_eq!(
            post(&app, DOWNLOAD, malformed).await.status(),
            StatusCode::BAD_REQUEST
        );
    }
    // Options are part of the recomputed byte identity, even with unchanged project binding.
    let mut language = download;
    language["language"] = json!("de-DE");
    assert_eq!(
        post(&app, DOWNLOAD, language).await.status(),
        StatusCode::CONFLICT
    );
    state.project_revision.fetch_add(1, Ordering::Relaxed);
    assert_eq!(
        post(&app, PREVIEW, input).await.status(),
        StatusCode::CONFLICT
    );
    assert_eq!(*state.project.lock().unwrap(), before);
    assert_eq!(format!("{:?}", state.command_stack.lock().unwrap()), stack);
}

#[tokio::test]
async fn input_budget_is_checked_before_project_lock_or_clone() {
    let dir = tempfile::tempdir().unwrap();
    let state = setup(dir.path());
    let app = knx_server::app(state.clone(), None);
    let mut input = binding(&state);
    let poisoner = state.clone();
    assert!(std::thread::spawn(move || {
        let _guard = poisoner.project.lock().unwrap();
        panic!("fictional project lock poisoning");
    })
    .join()
    .is_err());
    let oversized_tag = format!("x-{}", ["a"; 64].join("-"));
    assert_eq!(oversized_tag.len(), 129);
    assert!(oversized_tag.parse::<language_tags::LanguageTag>().is_ok());
    for language in [oversized_tag, "not_a_tag".to_string()] {
        input["language"] = json!(language);
        assert_eq!(
            post(&app, PREVIEW, input.clone()).await.status(),
            StatusCode::BAD_REQUEST,
            "language validation must precede project access"
        );
    }
    let mut parts = vec!["a"; 62];
    parts.push("ab");
    let boundary_tag = format!("x-{}", parts.join("-"));
    assert_eq!(boundary_tag.len(), 128);
    input["language"] = json!(boundary_tag);
    assert_eq!(
        post(&app, PREVIEW, input.clone()).await.status(),
        StatusCode::INTERNAL_SERVER_ERROR,
        "inclusive language allowance must reach project lock"
    );
    let raw = binding(&state).to_string();
    for (length, expected) in [
        (4096, StatusCode::INTERNAL_SERVER_ERROR),
        (4097, StatusCode::PAYLOAD_TOO_LARGE),
    ] {
        let padded = format!("{raw}{}", " ".repeat(length - raw.len()));
        assert_eq!(padded.len(), length);
        let reply = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(PREVIEW)
                    .header("content-type", "application/json")
                    .body(Body::from(padded))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(reply.status(), expected, "request body boundary {length}");
    }
    input["language"] = json!("a".repeat(5000));
    assert_eq!(
        post(&app, PREVIEW, input).await.status(),
        StatusCode::PAYLOAD_TOO_LARGE
    );
    assert_eq!(
        post(&app, PREVIEW, binding(&state)).await.status(),
        StatusCode::INTERNAL_SERVER_ERROR,
        "control request must reach poisoned project lock"
    );
}

#[tokio::test]
async fn no_project_invalid_selection_product_failure_and_auth_are_explicit() {
    let dir = tempfile::tempdir().unwrap();
    let empty = Arc::new(knx_server::AppState::new(dir.path().into()));
    let app = knx_server::app(empty.clone(), None);
    assert_eq!(
        post(&app, PREVIEW, binding(&empty)).await.status(),
        StatusCode::BAD_REQUEST
    );
    let mut download = binding(&empty);
    download["bundleSha256"] = json!("0".repeat(64));
    assert_eq!(
        post(&app, DOWNLOAD, download).await.status(),
        StatusCode::BAD_REQUEST
    );
    let state = setup(dir.path());
    let app = knx_server::app(state.clone(), None);
    let mut input = binding(&state);
    input["installation"] = json!(99);
    assert_eq!(
        post(&app, PREVIEW, input).await.status(),
        StatusCode::UNPROCESSABLE_ENTITY
    );
    let mut broken = knx_server::AppState::new(dir.path().into());
    *broken.project.lock().unwrap() = state.project.lock().unwrap().clone();
    broken.product_db_unavailable = Some("fictional catalogue open failure".into());
    let broken = Arc::new(broken);
    let app = knx_server::app(broken.clone(), None);
    assert_eq!(
        post(&app, PREVIEW, binding(&broken)).await.status(),
        StatusCode::INTERNAL_SERVER_ERROR
    );
    let hash =
        knx_server::hash_password_with_iterations("fictional monitoring guard fixture", 1000)
            .unwrap();
    let guarded = knx_server::app_with_auth(
        state.clone(),
        None,
        knx_server::AuthConfig::from_password_hash(&hash).unwrap(),
    );
    for endpoint in [PREVIEW, DOWNLOAD] {
        assert_eq!(
            post(&guarded, endpoint, binding(&state)).await.status(),
            StatusCode::UNAUTHORIZED
        );
    }
}
