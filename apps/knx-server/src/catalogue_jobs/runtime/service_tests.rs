//! Transport and production-mount assertions for bounded catalogue jobs.
//!
//! The refusing fixtures do not install. The production-mount candidate below
//! uses the real managed catalogue and is reserved for combined acceptance.

use super::*;
use crate::catalogue_jobs::{dto, http, Incarnation, JobId};
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use std::sync::mpsc as sync;
use tower::ServiceExt;

// Compiled/executed with the final integrated candidate, not a separate branch
// campaign. Unlike the refusing fixtures below, this uses the actual app mount,
// managed catalogue, B10, durable publisher and complete report adapter.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn production_mount_installs_with_complete_artifacts_and_a_real_retained_backup() {
    use std::io::Write;
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("catalogue.sqlite");
    let state = Arc::new(crate::AppState::with_product_db_at(
        tmp.path().to_path_buf(),
        Some(path.clone()),
    ));
    let guarded = crate::app_with_auth(
        Arc::clone(&state),
        None,
        crate::AuthConfig::from_plaintext_password("catalogue-test-password").unwrap(),
    );
    let response = guarded
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(http::PREFIX)
                .body(Body::new(UnreadBody))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(
        response.status(),
        StatusCode::UNAUTHORIZED,
        "auth must reject before reading the job body"
    );
    let router = crate::app(Arc::clone(&state), Some(tmp.path().to_path_buf()));
    let runtime = state.catalogue_jobs.attached_runtime().unwrap();
    assert!(Arc::ptr_eq(runtime.owner(), &state.catalogue_jobs));
    let incarnation = runtime.admission().unwrap().server_incarnation;
    let mut zip = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    for (name, bytes) in [
        ("knx_master.xml", br#"<KNX xmlns="http://knx.org/xml/project/20"><MasterData><Manufacturers><Manufacturer Id="M-0001" Name="Example"/></Manufacturers></MasterData></KNX>"#.as_slice()),
        ("M-0001/Hardware.xml", br#"<KNX xmlns="http://knx.org/xml/project/20"><ManufacturerData><Manufacturer RefId="M-0001"><Hardware><Hardware Id="H-1" Name="Example"/></Hardware></Manufacturer></ManufacturerData></KNX>"#.as_slice()),
        ("M-0001/M-0001_A-0001.xml", br#"<KNX xmlns="http://knx.org/xml/project/20"><ManufacturerData><Manufacturer RefId="M-0001"><ApplicationPrograms><ApplicationProgram Id="M-0001_A-0001-01-0001" Name="Private caption" ApplicationNumber="1" ApplicationVersion="1" ProgramType="ApplicationProgram" MaskVersion="MV-07B0" PeiType="0" LoadProcedureStyle="MergedProcedure" DefaultLanguage="en-US"><Static/></ApplicationProgram></ApplicationPrograms></Manufacturer></ManufacturerData></KNX>"#.as_slice()),
        ("M-0001/Baggages.xml", br#"<KNX xmlns="http://knx.org/xml/project/20"><ManufacturerData><Manufacturer RefId="M-0001"><Baggages><Baggage Id="B-1" Name="payload.bin" TargetPath=".." InstallOnImport="false" Extra="opaque"><FileInfo TimeInfo="" Version="1"/></Baggage><Baggage Id="B-2" Name="payload.bin" TargetPath=""/></Baggages></Manufacturer></ManufacturerData></KNX>"#.as_slice()),
        ("M-0001/Baggages/payload.bin", b"payload bytes".as_slice()),
    ] {
        zip.start_file(name, zip::write::SimpleFileOptions::default()).unwrap();
        zip.write_all(bytes).unwrap();
    }
    let source = zip.finish().unwrap().into_inner();
    let create = Create {
        schema_version: 1,
        policy_version: POLICY_VERSION.into(),
        server_incarnation: incarnation,
        admission_serial: U64(1),
        source_name: "actual.knxprod".into(),
        declared_bytes: U64(source.len() as u64),
        profile: ProfileName::Standard,
        allow_large_package: false,
    };
    let mut req = request(
        "POST",
        http::PREFIX,
        incarnation,
        None,
        Body::from(serde_json::to_vec(&create).unwrap()),
    );
    req.headers_mut()
        .insert("content-type", "application/json".parse().unwrap());
    let response = router.clone().oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);
    let created: Status =
        serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes()).unwrap();
    assert_eq!(created.phase, Some(Phase::Receiving));
    assert_eq!(created.lease.kind, LeaseKind::Receiving);
    let base = format!("{}/{}", http::PREFIX, created.handle.job_id);
    let mut upload = request(
        "PUT",
        &format!("{base}/upload"),
        incarnation,
        Some(created.handle.job_id),
        Body::from(source),
    );
    upload
        .headers_mut()
        .insert("content-type", "application/octet-stream".parse().unwrap());
    assert_eq!(
        router.clone().oneshot(upload).await.unwrap().status(),
        StatusCode::ACCEPTED
    );
    let capability = Capability::from_bytes([1; 32]);
    let handle = internal_handle(&created.handle);
    let ready = tokio::time::timeout(Duration::from_secs(20), async {
        loop {
            let s = runtime.status(handle, &capability).unwrap();
            s.validate().unwrap();
            assert_eq!(
                s.state,
                JobState::Running,
                "preview failed: {:?}",
                s.failure
            );
            if s.phase == Some(Phase::Ready) {
                break s;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    assert_eq!(ready.lease.kind, LeaseKind::Ready);
    assert!(ready.result.is_none());
    let preview = ready.preview.unwrap();
    assert_eq!(preview.programs, U64(1));
    let page = runtime
        .details(
            handle,
            &capability,
            preview.artifact.artifact_id,
            preview.artifact.public_sha256,
            U64(0),
            200,
        )
        .await
        .unwrap();
    let page: serde_json::Value = serde_json::from_slice(&page).unwrap();
    let rows = page["records"].as_array().unwrap();
    assert_eq!(rows.len() as u64, preview.artifact.records.get());
    assert_eq!(rows.iter().filter(|r| r["kind"] == "program").count(), 1);
    let program = rows.iter().find(|r| r["kind"] == "program").unwrap();
    assert_eq!(program["id"], "M-0001_A-0001-01-0001");
    assert!(program["name"].is_null());
    assert_eq!(program["withheld"][0]["rawUtf8Bytes"], "15");
    assert_eq!(
        rows.iter().filter(|r| r["kind"] == "packageReport").count(),
        1
    );
    assert_eq!(rows.iter().filter(|r| r["kind"] == "member").count(), 5);
    assert_eq!(rows.iter().filter(|r| r["kind"] == "provenance").count(), 1);
    let baggage = rows.iter().find(|r| r["kind"] == "baggageSummary").unwrap();
    assert_eq!(baggage["declarations"], "2");
    assert_eq!(baggage["payloads"], "1");
    let declarations: Vec<_> = rows
        .iter()
        .filter(|r| r["kind"] == "baggageDeclaration")
        .collect();
    assert_eq!(declarations.len(), 2);
    assert_eq!(declarations[0]["resolution"], "invalid");
    assert_eq!(declarations[0]["withheld"].as_array().unwrap().len(), 7);
    assert_eq!(declarations[1]["resolution"], "resolved");
    let payload = rows.iter().find(|r| r["kind"] == "baggagePayload").unwrap();
    assert_eq!(payload["memberPath"], "M-0001/Baggages/payload.bin");
    assert_eq!(payload["declarations"], "1");
    assert!(rows.iter().any(|r| r["kind"] == "diagnostic"
        && r["diagnosticKind"] == "unresolved-baggage-declaration"
        && r["detail"].is_null()));
    assert!(rows
        .iter()
        .any(|r| r["kind"] == "unknownConstruct" && r["name"] == "Extra" && r["sample"].is_null()));
    let command = ApplyCommand {
        schema_version: 1,
        command_id: JobId::from_bytes([3; 16]),
        consent_id: preview.consent_id,
        preview_artifact_id: preview.artifact.artifact_id,
        preview_public_sha256: preview.artifact.public_sha256,
        confirmed: true,
    };
    let mut req = request(
        "POST",
        &format!("{base}/apply"),
        incarnation,
        Some(handle.job),
        Body::from(serde_json::to_vec(&command).unwrap()),
    );
    req.headers_mut()
        .insert("content-type", "application/json".parse().unwrap());
    assert_eq!(
        router.clone().oneshot(req).await.unwrap().status(),
        StatusCode::ACCEPTED
    );
    let done = tokio::time::timeout(Duration::from_secs(20), async {
        loop {
            let s = runtime.status(handle, &capability).unwrap();
            s.validate().unwrap();
            if s.state != JobState::Running {
                break s;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    assert_eq!(
        done.state,
        JobState::Succeeded,
        "apply failed: {:?}",
        done.failure
    );
    assert_eq!(
        done.commit,
        Commit {
            outcome: CommitOutcome::Committed,
            evidence: CommitEvidence::CommitReturnedOk
        }
    );
    assert_eq!(done.cleanup.disposition, CleanupDisposition::Complete);
    assert_eq!(done.backup.state, BackupState::Retained);
    assert_eq!(
        done.backup.publication,
        BackupPublication::FileAndDirectorySyncAcknowledged
    );
    assert!(
        !runtime.apply(handle, &capability, &command).unwrap().0,
        "exact command is a replay after settlement"
    );
    let result = done.result.unwrap();
    let page = runtime
        .details(
            handle,
            &capability,
            result.artifact.artifact_id,
            result.artifact.public_sha256,
            U64(0),
            200,
        )
        .await
        .unwrap();
    let page: serde_json::Value = serde_json::from_slice(&page).unwrap();
    assert_eq!(
        page["records"].as_array().unwrap().len() as u64,
        result.artifact.records.get()
    );
    assert_eq!(page["records"][0]["kind"], "resultSummary");
    assert_eq!(
        page["records"][0]["cataloguePageBytes"],
        result.catalogue_page_bytes.to_string()
    );
    let backup = path.parent().unwrap().join(done.backup.name.unwrap());
    let before = knx_productdb::open_and_migrate(&backup).unwrap();
    let old_count: i64 = before
        .query_row("SELECT count(*) FROM application_program", [], |r| r.get(0))
        .unwrap();
    assert_eq!(
        old_count, 0,
        "published backup must contain the pre-live state"
    );
    let new_count = state
        .product_db
        .as_ref()
        .unwrap()
        .with_read(|conn| {
            conn.query_row("SELECT count(*) FROM application_program", [], |r| {
                r.get::<_, i64>(0)
            })
        })
        .unwrap()
        .unwrap();
    assert_eq!(new_count, 1);
    let workspace = runtime
        .lookup(handle, &capability)
        .unwrap()
        .workspace()
        .unwrap();
    let job_dir = workspace.lock().unwrap().directory().to_path_buf();
    let raw = job_dir.join("preview.raw");
    assert!(raw.is_file());
    // Every literal/encoded/private alias is denied by the real static mount.
    std::os::unix::fs::symlink(&job_dir, tmp.path().join("alias")).unwrap();
    std::os::unix::fs::symlink(&job_dir, tmp.path().join("uploads")).unwrap();
    let upload = router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/fs/upload")
                .header("content-type", "multipart/form-data; boundary=fixture")
                .body(Body::new(UnreadBody))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(
        upload.status(),
        StatusCode::BAD_REQUEST,
        "generic upload must refuse a private alias before staging/body reads"
    );
    let actual = format!("/{}", raw.strip_prefix(tmp.path()).unwrap().display());
    let encoded = actual.replace(".knxbench-catalogue-jobs", "%2eknxbench-catalogue-jobs");
    for suffix in [actual, encoded, "/alias/preview.raw".into()] {
        assert_eq!(
            router
                .clone()
                .oneshot(Request::builder().uri(suffix).body(Body::empty()).unwrap())
                .await
                .unwrap()
                .status(),
            StatusCode::NOT_FOUND
        );
    }

    runtime.dispose(handle, &capability).await.unwrap();
    assert!(
        backup.exists(),
        "report disposal must not remove a committed user backup"
    );
    state.catalogue_jobs.close_admission_and_request_stop();
    assert_eq!(
        state
            .catalogue_jobs
            .drain(Duration::from_secs(5), std::future::pending())
            .await
            .end,
        DrainEnd::Finished
    );
}

#[test]
fn disposed_tombstone_expires_without_reconstructing_a_successful_delete() {
    let incarnation = Incarnation::from_bytes([7; 16]);
    let capability = Capability::from_bytes([1; 32]);
    let mut manager = crate::catalogue_jobs::CatalogueJobManager::new(incarnation);
    manager.create(
        &capability,
        CreateBody {
            incarnation,
            serial: U64(1),
            source_name: "sample.knxprod".into(),
            declared_bytes: U64(0),
            profile: crate::catalogue_jobs::Profile::Standard,
            allow_large_package: false,
        },
    );
    let handle = manager.finish_active(true).unwrap();
    assert_eq!(
        manager.dispose(&handle, &capability),
        DisposeOutcome::Disposed
    );
    manager.state.tombstones.back_mut().unwrap().expires_at = Instant::now();
    assert_eq!(
        manager.dispose(&handle, &capability),
        DisposeOutcome::OutcomeUnavailable,
        "expired tombstone must not pretend a new DELETE succeeded"
    );
}

struct UnavailableBridge;
impl CatalogueJobBridge for UnavailableBridge {
    fn new_job(&self, _: Arc<JobStatus>) -> Result<Arc<dyn CatalogueJobExecution>, FailureKind> {
        Err(FailureKind::UnsupportedCatalogue)
    }
}

#[tokio::test]
async fn bridge_setup_refusal_is_a_retained_failed_create_not_an_invisible_serial() {
    let tmp = tempfile::tempdir().unwrap();
    let catalogue = tmp.path().join("catalogue.sqlite");
    std::fs::write(&catalogue, b"fixture").unwrap();
    let incarnation = Incarnation::from_bytes([9; 16]);
    let owner = Arc::new(CatalogueUpdateJobs::new(incarnation));
    let runtime =
        CatalogueRuntime::start(Arc::clone(&owner), &catalogue, Arc::new(UnavailableBridge))
            .unwrap();
    let create = Create {
        schema_version: 1,
        policy_version: POLICY_VERSION.into(),
        server_incarnation: incarnation,
        admission_serial: U64(1),
        source_name: "sample.knxprod".into(),
        declared_bytes: U64(0),
        profile: ProfileName::Standard,
        allow_large_package: false,
    };
    let capability = Capability::from_bytes([1; 32]);
    let result = runtime.create(&capability, create.clone());
    owner.close_admission_and_request_stop();
    let (_, status) = result
        .expect("a reserved create must retain a truthful failed status when bridge setup refuses");
    assert_eq!(status.state, JobState::Failed);
    assert_eq!(
        status.failure.unwrap().kind,
        FailureKind::UnsupportedCatalogue
    );
    let (fresh, replayed) = runtime.create(&capability, create).unwrap();
    assert!(!fresh);
    assert_eq!(replayed.handle, status.handle);
    assert_eq!(
        owner
            .drain(Duration::from_secs(5), std::future::pending())
            .await
            .end,
        DrainEnd::Finished
    );
}

struct RefusingBridge {
    entered: sync::Sender<()>,
    release: Mutex<Option<sync::Receiver<()>>>,
}
struct RefusingExecution {
    status: Arc<JobStatus>,
    entered: sync::Sender<()>,
    release: Mutex<sync::Receiver<()>>,
}
impl CatalogueJobBridge for RefusingBridge {
    fn new_job(
        &self,
        status: Arc<JobStatus>,
    ) -> Result<Arc<dyn CatalogueJobExecution>, FailureKind> {
        Ok(Arc::new(RefusingExecution {
            status,
            entered: self.entered.clone(),
            release: Mutex::new(self.release.lock().unwrap().take().unwrap()),
        }))
    }
}
impl RefusingExecution {
    fn fail(&self, kind: FailureKind) {
        self.status
            .update(|s| {
                s.state = if s.gate == Gate::CancelAccepted {
                    JobState::Cancelled
                } else {
                    JobState::Failed
                };
                s.phase = None;
                s.progress = None;
                s.apply.state = ApplyState::Spent;
                s.commit = Commit {
                    outcome: CommitOutcome::NotCommitted,
                    evidence: CommitEvidence::NoLiveMutation,
                };
                s.cleanup = Cleanup {
                    disposition: CleanupDisposition::Complete,
                    failure_kind: None,
                };
                s.failure = Some(Failure {
                    kind: if s.gate == Gate::CancelAccepted {
                        if s.stop_cause == Some(StopCause::ServerStop) {
                            FailureKind::ServerStop
                        } else {
                            FailureKind::Cancelled
                        }
                    } else {
                        kind
                    },
                    context: None,
                });
                s.lease.kind = LeaseKind::Retention;
            })
            .unwrap();
    }
}
impl CatalogueJobExecution for RefusingExecution {
    fn request_stop(&self, cause: StopCause) -> Result<CancelOutcome, FailureKind> {
        let s = self.status.snapshot()?;
        if s.state != JobState::Running {
            return Ok(CancelOutcome::AlreadyTerminal);
        }
        if s.gate == Gate::CancelAccepted {
            return Ok(CancelOutcome::AlreadyRequested);
        }
        self.status.update(|s| {
            s.gate = Gate::CancelAccepted;
            s.stop_cause = Some(cause);
            s.phase = Some(Phase::Settling);
            s.lease.kind = LeaseKind::Working;
            s.progress = None;
            s.apply.state = ApplyState::Spent;
        })?;
        Ok(CancelOutcome::Requested)
    }
    fn accept_apply(&self, _: &ApplyCommand) -> Result<bool, FailureKind> {
        Err(FailureKind::InvalidPhase)
    }
    fn expire(&self) -> Result<(), FailureKind> {
        Ok(())
    }
    fn preview(&self, source: &[u8], _: &mut JobWorkspace) -> Result<(), FailureKind> {
        assert_eq!(source, b"abc");
        self.entered.send(()).unwrap();
        let _ = self.release.lock().unwrap().recv();
        Err(FailureKind::PackageRefused)
    }
    fn apply(&self, _: &[u8], _: &mut JobWorkspace) -> Result<(), FailureKind> {
        panic!("refusing fixture must never install")
    }
    fn receive_failed(&self, kind: FailureKind) {
        self.fail(kind);
    }
    fn settle_idle(&self) {
        self.fail(FailureKind::Cancelled);
    }
    fn operation_failed(&self, kind: FailureKind) {
        self.fail(kind);
    }
    fn worker_lost(&self) {
        self.fail(FailureKind::WorkerFailed);
    }
    fn handles_closed(&self) -> bool {
        true
    } // This test adapter never opens SQLite.
}

struct UnreadBody;
impl http_body::Body for UnreadBody {
    type Data = axum::body::Bytes;
    type Error = std::convert::Infallible;
    fn poll_frame(
        self: std::pin::Pin<&mut Self>,
        _: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Result<http_body::Frame<Self::Data>, Self::Error>>> {
        panic!("rejected upload body was consumed")
    }
}

fn request(
    method: &str,
    path: &str,
    incarnation: Incarnation,
    job: Option<JobId>,
    body: Body,
) -> Request<Body> {
    let mut request = Request::builder()
        .method(method)
        .uri(path)
        .header("x-knx-catalogue-capability", "01".repeat(32));
    if job.is_some() {
        request = request
            .header("x-knx-catalogue-incarnation", incarnation.to_string())
            .header("x-knx-catalogue-serial", "1");
    }
    request.body(body).unwrap()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn same_owner_supervises_upload_replay_refusal_and_lost_dispose_reply() {
    let tmp = tempfile::tempdir().unwrap();
    let catalogue = tmp.path().join("catalogue.sqlite");
    std::fs::write(
        &catalogue,
        b"transport fixture; no native database is opened",
    )
    .unwrap();
    let incarnation = Incarnation::from_bytes([1; 16]);
    let owner = Arc::new(CatalogueUpdateJobs::new(incarnation));
    let (entered, started) = sync::channel();
    let (release, allowed) = sync::channel();
    let bridge = Arc::new(RefusingBridge {
        entered,
        release: Mutex::new(Some(allowed)),
    });
    let runtime = CatalogueRuntime::start(Arc::clone(&owner), &catalogue, bridge.clone()).unwrap();
    assert!(Arc::ptr_eq(runtime.owner(), &owner));
    assert!(matches!(
        CatalogueRuntime::start(Arc::clone(&owner), &catalogue, bridge),
        Err(FailureKind::JobBusy)
    ));
    let router = http::router(Arc::clone(&runtime));
    let create = Create {
        schema_version: 1,
        policy_version: POLICY_VERSION.into(),
        server_incarnation: incarnation,
        admission_serial: U64(1),
        source_name: "sample.knxprod".into(),
        declared_bytes: U64(3),
        profile: ProfileName::Large,
        allow_large_package: true,
    };
    let mut req = request(
        "POST",
        http::PREFIX,
        incarnation,
        None,
        Body::from(serde_json::to_vec(&create).unwrap()),
    );
    req.headers_mut()
        .insert("content-type", "application/json".parse().unwrap());
    let response = router.clone().oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);
    assert_eq!(response.headers()["cache-control"], "no-store");
    let created: Status = dto::from_str(
        std::str::from_utf8(&response.into_body().collect().await.unwrap().to_bytes()).unwrap(),
    )
    .unwrap();
    let handle = internal_handle(&created.handle);
    let capability = Capability::from_bytes([1; 32]);
    assert_eq!(runtime.admission().unwrap().admission, AdmissionState::Busy);
    let mut req = request(
        "POST",
        http::PREFIX,
        incarnation,
        None,
        Body::from(serde_json::to_vec(&create).unwrap()),
    );
    req.headers_mut()
        .insert("content-type", "application/json".parse().unwrap());
    assert_eq!(
        router.clone().oneshot(req).await.unwrap().status(),
        StatusCode::OK
    );
    let url = format!("{}/{}/upload", http::PREFIX, handle.job);
    let mut req = request(
        "PUT",
        &url,
        incarnation,
        Some(handle.job),
        Body::from("abc"),
    );
    req.headers_mut()
        .insert("content-type", "application/octet-stream".parse().unwrap());
    req.headers_mut()
        .insert("content-length", "3".parse().unwrap());
    assert_eq!(
        router.clone().oneshot(req).await.unwrap().status(),
        StatusCode::ACCEPTED
    );
    // Joinable supervisor owns the blocking worker independently of the reply.
    started.recv_timeout(Duration::from_secs(2)).unwrap();
    let mut duplicate = request(
        "PUT",
        &url,
        incarnation,
        Some(handle.job),
        Body::new(UnreadBody),
    );
    duplicate
        .headers_mut()
        .insert("content-type", "application/octet-stream".parse().unwrap());
    assert_eq!(
        router.clone().oneshot(duplicate).await.unwrap().status(),
        StatusCode::CONFLICT
    );
    let status_url = format!("{}/{}", http::PREFIX, handle.job);
    let mut wrong = request(
        "GET",
        &status_url,
        incarnation,
        Some(handle.job),
        Body::empty(),
    );
    wrong.headers_mut().insert(
        "x-knx-catalogue-capability",
        "02".repeat(32).parse().unwrap(),
    );
    assert_eq!(
        router.clone().oneshot(wrong).await.unwrap().status(),
        StatusCode::GONE
    );
    assert_eq!(
        router
            .clone()
            .oneshot(request(
                "DELETE",
                &status_url,
                incarnation,
                Some(handle.job),
                Body::empty()
            ))
            .await
            .unwrap()
            .status(),
        StatusCode::CONFLICT
    );
    release.send(()).unwrap();
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if runtime.status(handle, &capability).unwrap().state != JobState::Running {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    let terminal = runtime.status(handle, &capability).unwrap();
    assert_eq!(terminal.state, JobState::Failed);
    assert_eq!(terminal.failure.unwrap().kind, FailureKind::PackageRefused);
    assert_eq!(terminal.commit.outcome, CommitOutcome::NotCommitted);
    let record = runtime.lookup(handle, &capability).unwrap();
    let workspace = record.workspace().unwrap();
    let directory = workspace.lock().unwrap().directory().to_path_buf();
    assert!(!directory.join("source.sealed").exists());
    // Poll to the actual supervised DELETE handoff, then drop its HTTP waiter.
    let mut deletion = Box::pin(runtime.dispose(handle, &capability));
    std::future::poll_fn(|cx| {
        assert!(std::future::Future::poll(deletion.as_mut(), cx).is_pending());
        std::task::Poll::Ready(())
    })
    .await;
    drop(deletion);
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let disposed = owner
                .manager
                .lock()
                .unwrap()
                .state
                .tombstones
                .iter()
                .any(|t| t.meta.handle == handle && t.disposed);
            if disposed {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    assert!(!directory.exists());
    assert_eq!(
        router
            .clone()
            .oneshot(request(
                "DELETE",
                &status_url,
                incarnation,
                Some(handle.job),
                Body::empty()
            ))
            .await
            .unwrap()
            .status(),
        StatusCode::OK
    );
    assert_eq!(
        router
            .oneshot(request(
                "GET",
                &status_url,
                incarnation,
                Some(handle.job),
                Body::empty()
            ))
            .await
            .unwrap()
            .status(),
        StatusCode::GONE
    );
    owner.close_admission_and_request_stop();
    assert_eq!(
        owner
            .drain(Duration::from_secs(5), std::future::pending())
            .await
            .end,
        DrainEnd::Finished
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn close_of_receiving_job_reports_inspected_not_committed_after_join() {
    let tmp = tempfile::tempdir().unwrap();
    let catalogue = tmp.path().join("catalogue.sqlite");
    let incarnation = Incarnation::from_bytes([5; 16]);
    let owner = Arc::new(CatalogueUpdateJobs::new(incarnation));
    let (entered, _started) = sync::channel();
    let (_release, allowed) = sync::channel();
    let runtime = CatalogueRuntime::start(
        Arc::clone(&owner),
        &catalogue,
        Arc::new(RefusingBridge {
            entered,
            release: Mutex::new(Some(allowed)),
        }),
    )
    .unwrap();
    let capability = Capability::from_bytes([1; 32]);
    let (_, created) = runtime
        .create(
            &capability,
            Create {
                schema_version: 1,
                policy_version: POLICY_VERSION.into(),
                server_incarnation: incarnation,
                admission_serial: U64(1),
                source_name: "sample.knxprod".into(),
                declared_bytes: U64(3),
                profile: ProfileName::Standard,
                allow_large_package: false,
            },
        )
        .unwrap();
    owner.close_admission_and_request_stop();
    let drained = owner
        .drain(Duration::from_secs(5), std::future::pending())
        .await;
    assert_eq!(drained.end, DrainEnd::Finished);
    assert_eq!(
        drained.commit,
        DrainCommit::NotCommitted,
        "drain must report the joined job's actual no-live-mutation witness"
    );
    assert_eq!(drained.cleanup, DrainCleanup::Complete);
    let status = runtime
        .status(internal_handle(&created.handle), &capability)
        .unwrap();
    assert_eq!(status.state, JobState::Cancelled);
    assert_eq!(status.stop_cause, Some(StopCause::ServerStop));
}
