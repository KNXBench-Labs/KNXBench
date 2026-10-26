//! Typed PID 7 failure facts through the real worker, private journal, restart and status v2.
use super::*;

/// The complete status v2 key set: version 1 keys plus `failureDiagnostics`.
const STATUS_KEYS: [&str; 20] = [
    "schemaVersion",
    "operationId",
    "binding",
    "address",
    "deviceName",
    "state",
    "completedSteps",
    "totalSteps",
    "backupAvailable",
    "recoveryPhrase",
    "downloadPhrase",
    "releasePhrase",
    "message",
    "refusal",
    "mutationMayHaveStarted",
    "prestateDiffers",
    "prestateUnreadableAllocations",
    "pointerObservations",
    "programVersionObservation",
    "failureDiagnostics",
];

/// Raw status body: the parsed value would hide a duplicated key.
async fn raw_status(router: &Router) -> String {
    raw_http(router, "/api/device-system2/status", None).await
}
/// Exact response bytes of a 200 request, GET without a body and POST with one.
async fn raw_http(router: &Router, path: &str, body: Option<Value>) -> String {
    let request = match body {
        Some(value) => Request::builder()
            .method("POST")
            .uri(path)
            .header("Content-Type", "application/json")
            .body(Body::from(serde_json::to_vec(&value).unwrap())),
        None => Request::builder().uri(path).body(Body::empty()),
    };
    let response = router.clone().oneshot(request.unwrap()).await.unwrap();
    assert_eq!(response.status().as_u16(), 200, "{path}");
    String::from_utf8(
        axum::body::to_bytes(response.into_body(), 128 * 1024)
            .await
            .unwrap()
            .to_vec(),
    )
    .unwrap()
}
/// Opt-in, test-only capture of exact backend response bytes for the web decoder. The
/// caller owns the directory; files are created new and never overwritten.
fn capture(name: &str, raw: &str) {
    let Some(directory) = std::env::var_os("KNXBENCH_S2_DIAGNOSTIC_CAPTURE") else {
        return;
    };
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options
        .open(PathBuf::from(directory).join(name))
        .expect("create capture without overwriting earlier output");
    std::io::Write::write_all(&mut file, raw.as_bytes()).unwrap();
}
async fn checked_status(router: &Router, operation: &Path, raws: &[&str]) -> Value {
    checked_raw(&raw_status(router).await, operation, raws).0
}
/// Status-shape and privacy assertions on exact bytes; returns the value and the bytes.
fn checked_raw(raw: &str, operation: &Path, raws: &[&str]) -> (Value, String) {
    assert_eq!(
        raw.matches("\"schemaVersion\"").count(),
        1,
        "one version key"
    );
    for forbidden in raws.iter().copied().chain([
        "local_diagnostic",
        "localDiagnostic",
        "pointer_failure",
        "pointerFailure",
        "resource pointer response",
        "checkpoint-",
        operation.to_str().unwrap(),
    ]) {
        assert!(!raw.contains(forbidden), "status leaks {forbidden}");
    }
    let status: Value = serde_json::from_str(raw).unwrap();
    let mut keys: Vec<_> = status.as_object().unwrap().keys().cloned().collect();
    keys.sort();
    let mut expected: Vec<_> = STATUS_KEYS.iter().map(|k| k.to_string()).collect();
    expected.sort();
    assert_eq!(keys, expected);
    assert_eq!(status["schemaVersion"], 2);
    (status, raw.to_owned())
}
/// Like `settled`, with a wider budget: a debug-build restore under load can outlast 10 s
/// while still progressing (observed on the unchanged base as well).
async fn settled_slow(state: &crate::SharedState, router: &Router, outcome: &str) {
    tokio::time::timeout(Duration::from_secs(180), async {
        while !state
            .device_system2
            .inner
            .lock()
            .unwrap()
            .current
            .as_ref()
            .and_then(|r| r.task.as_ref())
            .is_some_and(tokio::task::JoinHandle::is_finished)
        {
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .expect("actual mounted worker must finish and disconnect");
    state.device_system2.join_finished().await;
    let status = http(router, "/api/device-system2/status", None, 200).await;
    assert_eq!(status["state"], outcome);
}
fn last_checkpoint(journal: &Path) -> Value {
    let head: Value =
        serde_json::from_slice(&std::fs::read(journal.join("HEAD")).unwrap()).unwrap();
    let name = format!("checkpoint-{:08}.json", head["sequence"].as_u64().unwrap());
    serde_json::from_slice(&std::fs::read(journal.join(name)).unwrap()).unwrap()
}

struct Fixture {
    directory: tempfile::TempDir,
    hardware: Arc<TestHardware>,
    state: crate::SharedState,
    router: Router,
    device: DeviceId,
}
/// Public package, one addressed device with the nondefault PIR value, no data-write pause.
async fn fixture() -> Fixture {
    let package = std::fs::read(
        std::env::var_os("KNXBENCH_PUBLIC_UP258")
            .expect("set KNXBENCH_PUBLIC_UP258 to the existing read-only public fixture"),
    )
    .unwrap();
    assert_eq!(knx_productdb::sha256_hex(&package), PACKAGE_SHA256);
    let directory = tempfile::tempdir().unwrap();
    let products = directory.path().join("products.sqlite");
    let conn = knx_productdb::open_and_migrate(&products).unwrap();
    knx_productdb::install_package_with_limits(
        &conn,
        "public-up258.knxprod",
        &package,
        knx_productdb::PackageLimits::LARGE,
    )
    .unwrap();
    drop(conn);
    let hardware = TestHardware::new(directory.path());
    hardware.pause_data_write.store(false, Ordering::SeqCst);
    let state = state(directory.path(), Some(products), &hardware);
    let router = crate::app(state.clone(), None);
    http(&router, "/api/project/new", Some(json!({})), 200).await;
    http(
        &router,
        "/api/devices",
        Some(json!({"catalogItemId":CATALOG_ITEM,"name":"Diagnosed device"})),
        200,
    )
    .await;
    let device = {
        let mut project = state.project.lock().unwrap();
        let project = project.as_mut().unwrap();
        let id = project.devices.iter().map(|d| d.id).next().unwrap();
        project.devices.get_mut(id).unwrap().address = Some(TARGET.parse().unwrap());
        id
    };
    // The same nondefault PIR source value the backup check expects, via the real panel.
    let path = format!("/api/device/{}/parameters", device.0);
    let panel = http(&router, &path, None, 200).await;
    let field = pir_fields(&panel)[0].clone();
    let tree = http(&router, "/api/project", None, 200).await;
    http(
        &router,
        &path,
        Some(json!({
            "etsId": field["writeEtsId"], "raw": PIR_RAW,
            "serverIncarnation": tree["server_incarnation"],
            "projectIncarnation": tree["project_incarnation"],
            "snapshotRevision": tree["snapshot_revision"],
            "expectedValue": field["value"]
        })),
        200,
    )
    .await;
    Fixture {
        directory,
        hardware,
        state,
        router,
        device,
    }
}
fn operation_dir(directory: &Path, id: &str) -> PathBuf {
    directory.join(super::super::super::ROOT_NAME).join(id)
}

#[tokio::test]
#[ignore = "requires digest-pinned public package; set KNXBENCH_PUBLIC_UP258; no network or hardware"]
async fn download_and_restore_refusals_reach_status_v2_across_restarts() {
    let Fixture {
        directory: temp,
        hardware,
        state,
        router,
        device,
    } = fixture().await;
    let (directory, hardware) = (temp.path(), &hardware);
    // Download: object 1's second PID 7 read, after the first mutation, has two elements.
    hardware.arm_pid7(1, 1, 2, &[0xB7, 0xE5]);
    let preview = prepare(&router, device).await;
    let accepted_raw = raw_http(&router, "/api/device-system2/start", Some(start(&preview))).await;
    let accepted: Value = serde_json::from_str(&accepted_raw).unwrap();
    assert_eq!(accepted["schemaVersion"], 2);
    assert_eq!(
        accepted["failureDiagnostics"],
        json!([]),
        "a new operation starts empty"
    );
    let operation_id = accepted["operationId"].as_str().unwrap().to_owned();
    let operation = operation_dir(directory, &operation_id);
    checked_raw(&accepted_raw, &operation, &["b7e5"]);
    capture("accepted-running.json", &accepted_raw);
    settled_slow(&state, &router, "failed").await;
    let download = json!({"run":"download","reason":"elementCount","object":1,
        "propertyId":7,"elements":2,"octets":2});
    let (failed, failed_raw) = checked_raw(&raw_status(&router).await, &operation, &["b7e5"]);
    assert_eq!(failed["failureDiagnostics"], json!([download]));
    assert_eq!(failed["mutationMayHaveStarted"], true);
    assert_eq!(failed["backupAvailable"], true);
    capture("download-failed.json", &failed_raw);
    assert!(hardware.writes.load(Ordering::SeqCst) > 0);
    // The private journal keeps the raw marker; only its envelope moved to version 2.
    let journal = operation.join("journal");
    let private = last_checkpoint(&journal);
    assert_eq!(private["format"], 2);
    assert_eq!(private["checkpoint"]["event"], "failed");
    assert!(private["checkpoint"]["local_diagnostic"]
        .as_str()
        .unwrap()
        .contains("b7e5"));
    state.catalogue_jobs.close_admission_and_request_stop();
    drop(router);
    drop(state);

    // Restart without a project: the status comes only from the durable journal.
    let fresh = self::state(directory, None, hardware);
    let router = crate::app(fresh.clone(), None);
    let (reopened, reopened_raw) = checked_raw(&raw_status(&router).await, &operation, &["b7e5"]);
    assert_eq!(reopened["state"], "failed");
    assert_eq!(reopened["failureDiagnostics"], json!([download]));
    capture("reopened-download.json", &reopened_raw);
    // Restore: object 2's first restore-time PID 7 read is three octets for one element.
    hardware.arm_pid7(2, 0, 1, &[0x91, 0x92, 0x93]);
    let recovery = json!({
        "operationId":operation_id, "gateway":GATEWAY,
        "recoveryConfirmation":reopened["recoveryPhrase"],
        "downloadConfirmation":reopened["downloadPhrase"],
        "releaseConfirmation":reopened["releasePhrase"]
    });
    let restoring_raw = raw_http(
        &router,
        "/api/device-system2/recover",
        Some(recovery.clone()),
    )
    .await;
    let (restoring, _) = checked_raw(&restoring_raw, &operation, &["b7e5"]);
    assert_eq!(
        restoring["failureDiagnostics"],
        json!([download]),
        "restore inherits only the labelled download diagnosis"
    );
    capture("restoring.json", &restoring_raw);
    settled_slow(&fresh, &router, "restoreFailed").await;
    let restore = json!({"run":"restore","reason":"unsupportedWidth","object":2,
        "propertyId":7,"elements":1,"octets":3});
    let (restore_failed, restore_failed_raw) =
        checked_raw(&raw_status(&router).await, &operation, &["b7e5", "919293"]);
    assert_eq!(
        restore_failed["failureDiagnostics"],
        json!([download, restore])
    );
    capture("restore-failed.json", &restore_failed_raw);
    assert_eq!(last_checkpoint(&journal)["format"], 2);
    drop(router);
    drop(fresh);

    // A second restart reads both, and the consumed recovery is not retried.
    let restarted = self::state(directory, None, hardware);
    let router = crate::app(restarted, None);
    let (status, status_raw) =
        checked_raw(&raw_status(&router).await, &operation, &["b7e5", "919293"]);
    assert_eq!(status["state"], "restoreFailed");
    assert_eq!(status["failureDiagnostics"], json!([download, restore]));
    capture("reopened-restore.json", &status_raw);
    let connects = hardware.connects.load(Ordering::SeqCst);
    assert_eq!(
        http(&router, "/api/device-system2/recover", Some(recovery), 409).await["kind"],
        "system2RecoveryConsumed"
    );
    assert_eq!(hardware.connects.load(Ordering::SeqCst), connects);
}

#[tokio::test]
#[ignore = "requires digest-pinned public package; set KNXBENCH_PUBLIC_UP258; no network or hardware"]
async fn a_new_operation_and_selection_never_transfer_a_diagnosis() {
    let Fixture {
        directory: temp,
        hardware,
        state,
        router: first,
        device,
    } = fixture().await;
    let (directory, hardware, router) = (temp.path(), &hardware, &first);
    // A: refused at its first PID 7 read of object 2, before any write.
    hardware.arm_pid7(2, 0, 0, &[0xAB, 0xCD]);
    let preview_a = prepare(router, device).await;
    let a = http(
        router,
        "/api/device-system2/start",
        Some(start(&preview_a)),
        200,
    )
    .await;
    let a_id = a["operationId"].as_str().unwrap().to_owned();
    let a_dir = operation_dir(directory, &a_id);
    settled_slow(&state, router, "failed").await;
    let a_diagnosis = json!([{"run":"download","reason":"zeroElementsWithData","object":2,
        "propertyId":7,"elements":0,"octets":2}]);
    let status = checked_status(router, &a_dir, &["abcd"]).await;
    assert_eq!(status["failureDiagnostics"], a_diagnosis);
    assert_eq!(status["mutationMayHaveStarted"], false);
    // The consumed preparation cannot start A again: no connect, A's evidence unchanged.
    let (connects, writes) = (
        hardware.connects.load(Ordering::SeqCst),
        hardware.writes.load(Ordering::SeqCst),
    );
    let a_tree = evidence_tree(directory);
    assert_eq!(
        http(
            router,
            "/api/device-system2/start",
            Some(start(&preview_a)),
            409
        )
        .await["kind"],
        "system2PreparationStale"
    );
    assert_eq!(hardware.connects.load(Ordering::SeqCst), connects);
    assert_eq!(hardware.writes.load(Ordering::SeqCst), writes);
    assert_eq!(evidence_tree(directory), a_tree);
    // B: a new operation on the same server starts empty and finishes without a diagnosis.
    hardware.disarm_pid7();
    let preview_b = prepare(router, device).await;
    let b = http(
        router,
        "/api/device-system2/start",
        Some(start(&preview_b)),
        200,
    )
    .await;
    assert_ne!(b["operationId"], a["operationId"]);
    assert_eq!(b["failureDiagnostics"], json!([]));
    let b_dir = operation_dir(directory, b["operationId"].as_str().unwrap());
    settled_slow(&state, router, "finished").await;
    assert_eq!(
        checked_status(router, &b_dir, &["abcd"]).await["failureDiagnostics"],
        json!([])
    );
    // Selecting the retained A shows A's own diagnosis; after a restart, still only A's.
    let selected = http(
        router,
        "/api/device-system2/select",
        Some(json!({"operationId": a_id})),
        200,
    )
    .await;
    assert_eq!(selected["operationId"], a_id.as_str());
    assert_eq!(selected["failureDiagnostics"], a_diagnosis);
    // Restart: the first service and its router release the evidence root first.
    state.catalogue_jobs.close_admission_and_request_stop();
    drop(first);
    drop(state);
    let restarted = self::state(directory, None, hardware);
    let router = crate::app(restarted, None);
    let status = checked_status(&router, &a_dir, &["abcd"]).await;
    assert_eq!(status["operationId"], a_id.as_str());
    assert_eq!(status["state"], "failed");
    assert_eq!(status["failureDiagnostics"], a_diagnosis);
}
