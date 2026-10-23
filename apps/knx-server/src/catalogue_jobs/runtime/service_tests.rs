//! Service/HTTP tests with an explicitly pre-native refusing bridge. This
//! fixture cannot claim COMMIT, produce Ready or install anything; B10 combined
//! execution belongs to the parent integration, not this transport test.

use super::*;
use crate::catalogue_jobs::{dto, http, Incarnation, JobId};
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use std::sync::mpsc as sync;
use tower::ServiceExt;

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
