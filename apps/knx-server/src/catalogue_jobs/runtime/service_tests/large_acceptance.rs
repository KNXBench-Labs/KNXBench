//! One private real-package Large-profile job, not a benchmark or quota claim.
//!
//! Run alone in a fresh process. The input must be explicitly supplied and
//! digest-pinned; at least 512 MiB decoded metadata prevents a tiny renamed
//! Standard fixture from being credited as resource acceptance. This is not a
//! claim to exercise the >4 GiB Standard/Large policy boundary.

use super::*;
use sha2::{Digest, Sha256 as Hasher};
use std::io::Write;
use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
use std::path::{Path, PathBuf};

fn proc_kib(field: &str) -> u64 {
    std::fs::read_to_string("/proc/self/status")
        .unwrap()
        .lines()
        .find_map(|line| line.strip_prefix(field))
        .and_then(|rest| rest.split_whitespace().next())
        .unwrap()
        .parse::<u64>()
        .unwrap()
        * 1024
}

// No symlink following, no corpus traversal. Only this test's new native and
// artifact directory is sampled; filesystem allocation includes SQLite journals.
fn disk(path: &Path) -> (u64, u64) {
    let meta = match std::fs::symlink_metadata(path) {
        Ok(meta) => meta,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return (0, 0),
        Err(e) => panic!("owned resource stat: {e}"),
    };
    if meta.file_type().is_symlink() {
        return (0, 0);
    }
    let mut bytes = (meta.len(), meta.blocks() * 512);
    if meta.is_dir() {
        let entries = match std::fs::read_dir(path) {
            Ok(entries) => entries,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return (0, 0),
            Err(e) => panic!("owned resource directory: {e}"),
        };
        for entry in entries {
            let next = disk(&entry.unwrap().path());
            bytes.0 += next.0;
            bytes.1 += next.1;
        }
    }
    bytes
}

struct Samples {
    stop: Option<sync::Sender<()>>,
    worker: Option<std::thread::JoinHandle<(u64, u64, u64, u64)>>,
}
impl Samples {
    fn start(root: PathBuf) -> Self {
        let (stop, receiver) = sync::channel();
        let worker = std::thread::spawn(move || {
            let mut peaks = (0u64, 0u64, 0u64, 0u64);
            loop {
                let rss = proc_kib("VmRSS:");
                let (apparent, allocated) = disk(&root);
                peaks.0 = peaks.0.max(rss);
                peaks.1 = peaks.1.max(apparent);
                peaks.2 = peaks.2.max(allocated);
                peaks.3 += 1;
                match receiver.recv_timeout(Duration::from_millis(10)) {
                    Err(sync::RecvTimeoutError::Timeout) => {}
                    _ => return peaks,
                }
            }
        });
        Self {
            stop: Some(stop),
            worker: Some(worker),
        }
    }
    fn finish_checked(&mut self) -> std::thread::Result<(u64, u64, u64, u64)> {
        let _ = self.stop.take().unwrap().send(());
        self.worker.take().unwrap().join()
    }
}
impl Drop for Samples {
    fn drop(&mut self) {
        self.stop.take();
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

// Test-private, fixed-size history: only the last completed status GET and last
// validated public DTO, plus the current request milestone. Never copy SQLite
// or inspect a worker-held workspace/native/SQL mutex during failure capture.
struct Evidence {
    file: std::fs::File,
    samples: Option<Samples>,
    started: Instant,
    rss_before: u64,
    hwm_before: u64,
    polls: u64,
    wait_target: &'static str,
    milestone: &'static str,
    milestone_seconds: f64,
    request_started_seconds: Option<f64>,
    last_get: Option<serde_json::Value>,
    last_validated: Option<serde_json::Value>,
    failure: Option<&'static str>,
    success: bool,
    peaks: Option<(u64, u64, u64, u64)>,
    sampler_panicked: bool,
}
impl Evidence {
    fn mark(&mut self, milestone: &'static str) {
        self.milestone = milestone;
        self.milestone_seconds = self.started.elapsed().as_secs_f64();
    }
    fn finish_samples(&mut self) -> (u64, u64, u64, u64) {
        let mut samples = self.samples.take().unwrap();
        let peaks = match samples.finish_checked() {
            Ok(peaks) => peaks,
            Err(_) => {
                self.sampler_panicked = true;
                panic!("resource sampler");
            }
        };
        self.peaks = Some(peaks);
        peaks
    }
}
impl Drop for Evidence {
    fn drop(&mut self) {
        // Join only our sampler, never the native worker. Capture before TempDir
        // teardown, including assertion/GET-validation panics, not just timeout.
        if let Some(mut samples) = self.samples.take() {
            match samples.finish_checked() {
                Ok(peaks) => self.peaks = Some(peaks),
                Err(_) => self.sampler_panicked = true,
            }
        }
        let report = serde_json::json!({
            "schemaVersion": 1, "success": self.success, "resourceAcceptance": false,
            "failure": self.failure, "unwinding": std::thread::panicking(),
            "waitTarget": self.wait_target, "watchdogSeconds": 1800,
            "pollCount": self.polls, "milestone": self.milestone,
            "milestoneElapsedSeconds": self.milestone_seconds,
            "requestStartedElapsedSeconds": self.request_started_seconds,
            "requestInFlight": matches!(self.milestone, "statusRequestStarted" | "statusResponseHeadersReceived"),
            "lastCompletedGet": self.last_get, "lastValidatedDto": self.last_validated,
            "observedWindowSeconds": self.started.elapsed().as_secs_f64(),
            "rssBeforeBytes": self.rss_before, "processLifetimeHwmBeforeBytes": self.hwm_before,
            "sampledPeakProcessRssBytes": self.peaks.map(|p| p.0),
            "sampledPeakOwnedApparentBytes": self.peaks.map(|p| p.1),
            "sampledPeakOwnedAllocatedBytes": self.peaks.map(|p| p.2),
            "sampleCount": self.peaks.map(|p| p.3),
            "samplerPanicked": self.sampler_panicked, "samplingIntervalMillis": 10,
            "failureNativeSettlement": "not observed; no native-worker join, disposal or drain attempted by this diagnostic",
            "scope": "Test-owned root metadata samples and process RSS only; last actual bounded public status GET/DTO. No active SQLite copy, source payload, native stack, importer substage or sync count. This diagnostic alone is not resource acceptance or timeout attribution."
        });
        // Do not replace a primary panic with a diagnostic-write panic.
        let write = (|| -> std::io::Result<()> {
            serde_json::to_writer_pretty(&mut self.file, &report)?;
            self.file.write_all(b"\n")?;
            self.file.sync_all()
        })();
        if write.is_err() {
            eprintln!("Large private failure diagnostic write failed");
        }
    }
}

async fn status(
    router: &axum::Router,
    base: &str,
    incarnation: Incarnation,
    job: JobId,
    evidence: &mut Evidence,
) -> Status {
    evidence.polls += 1;
    evidence.request_started_seconds = Some(evidence.started.elapsed().as_secs_f64());
    evidence.mark("statusRequestStarted");
    let response = router
        .clone()
        .oneshot(request("GET", base, incarnation, Some(job), Body::empty()))
        .await
        .unwrap();
    let code = response.status();
    evidence.mark("statusResponseHeadersReceived");
    // Independent test evidence cap, not a public service/schema change.
    let bytes = axum::body::to_bytes(response.into_body(), 64 * 1024)
        .await
        .unwrap();
    evidence.mark("statusResponseBodyComplete");
    evidence.last_get = Some(serde_json::json!({
        "poll": evidence.polls, "httpStatus": code.as_u16(),
        "requestStartedElapsedSeconds": evidence.request_started_seconds,
        "completedElapsedSeconds": evidence.milestone_seconds,
        "responseBodyBytes": bytes.as_ref()
    }));
    assert_eq!(code, StatusCode::OK);
    let status: Status = serde_json::from_slice(&bytes).unwrap();
    evidence.mark("statusDtoParsed");
    status.validate().unwrap();
    evidence.mark("statusDtoValidated");
    evidence.last_validated = Some(serde_json::json!({
        "poll": evidence.polls, "validatedElapsedSeconds": evidence.milestone_seconds,
        "dto": &status
    }));
    status
}

async fn wait(
    router: &axum::Router,
    base: &str,
    incarnation: Incarnation,
    job: JobId,
    ready: bool,
    evidence: &mut Evidence,
) -> Status {
    // A finite acceptance watchdog, not a service latency guarantee. The Large
    // profile's native deadlines remain authoritative and are not extended here.
    evidence.wait_target = if ready { "ready" } else { "terminal" };
    let result = tokio::time::timeout(Duration::from_secs(1800), async {
        loop {
            let current = status(router, base, incarnation, job, evidence).await;
            if current.state != JobState::Running || (ready && current.phase == Some(Phase::Ready))
            {
                return current;
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
    })
    .await;
    match result {
        Ok(status) => status,
        Err(_) => {
            evidence.failure = Some("finite Large acceptance watchdog");
            panic!("finite Large acceptance watchdog");
        }
    }
}

async fn details(
    router: &axum::Router,
    base: &str,
    incarnation: Incarnation,
    job: JobId,
    artifact: &dto::Artifact,
) -> serde_json::Value {
    let uri = format!(
        "{base}/details?artifactId={}&publicSha256={}&offset=0&limit=1",
        artifact.artifact_id, artifact.public_sha256
    );
    let response = router
        .clone()
        .oneshot(request("GET", &uri, incarnation, Some(job), Body::empty()))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes()).unwrap()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "private digest-pinned >=512 MiB decoded package; explicit Large-profile acceptance, run alone"]
async fn production_mount_large_private_package_commits_retains_backup_and_records_scoped_resources(
) {
    let input = PathBuf::from(
        std::env::var_os("KNXBENCH_LARGE_JOB_INPUT")
            .expect("supply one authorized private .knxprod input"),
    );
    let expected =
        std::env::var("KNXBENCH_LARGE_JOB_SHA256").expect("pin input SHA-256 before admission");
    assert!(
        expected.len() == 64
            && expected
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    );
    let output = PathBuf::from(
        std::env::var_os("KNXBENCH_LARGE_JOB_EVIDENCE")
            .expect("supply a new private evidence JSON path"),
    );
    assert!(input.is_absolute() && input.is_file());
    assert!(
        std::fs::metadata(&input).unwrap().len() <= knx_productdb::MAX_PACKAGE_INPUT_BYTES,
        "input exceeds retained package admission; do not read an oversized fixture"
    );
    assert!(output.is_absolute() && !output.exists());
    assert!(output.parent().unwrap().is_dir());
    let tmp = tempfile::tempdir().unwrap();
    let started = Instant::now();
    let rss_before = proc_kib("VmRSS:");
    let hwm_before = proc_kib("VmHWM:");
    let diagnostic = output.with_extension("diagnostic.json");
    assert_ne!(diagnostic, output);
    assert!(!diagnostic.starts_with(tmp.path()));
    let mut evidence = Evidence {
        file: std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(diagnostic)
            .expect("new private diagnostic outside owned TempDir"),
        samples: Some(Samples::start(tmp.path().to_path_buf())),
        started,
        rss_before,
        hwm_before,
        polls: 0,
        wait_target: "notStarted",
        milestone: "setup",
        milestone_seconds: 0.0,
        request_started_seconds: None,
        last_get: None,
        last_validated: None,
        failure: None,
        success: false,
        peaks: None,
        sampler_panicked: false,
    };
    let source = std::fs::read(&input).expect("read private input");
    assert_eq!(
        format!("{:x}", Hasher::digest(&source)),
        expected,
        "private input digest mismatch"
    );
    let source_len = source.len() as u64;
    let mut archive =
        zip::ZipArchive::new(std::io::Cursor::new(&source)).expect("real product ZIP");
    let expanded: u64 = (0..archive.len())
        .map(|i| archive.by_index(i).unwrap().size())
        .try_fold(0u64, u64::checked_add)
        .expect("decoded size sum");
    assert!(
        expanded >= 512 * 1024 * 1024,
        "tiny Standard fixtures do not satisfy this resource scenario"
    );
    assert!(expanded <= knx_productdb::PackageLimits::LARGE.max_expanded_size);
    drop(archive);
    let catalogue = tmp.path().join("catalogue.sqlite");
    let state = Arc::new(crate::AppState::with_product_db_at(
        tmp.path().to_path_buf(),
        Some(catalogue.clone()),
    ));
    let router = crate::app(Arc::clone(&state), None);
    let runtime = state.catalogue_jobs.attached_runtime().unwrap();
    assert!(Arc::ptr_eq(runtime.owner(), &state.catalogue_jobs));
    let incarnation = runtime.admission().unwrap().server_incarnation;
    let create = Create {
        schema_version: 1,
        policy_version: POLICY_VERSION.into(),
        server_incarnation: incarnation,
        admission_serial: U64(1),
        source_name: "private-acceptance.knxprod".into(),
        declared_bytes: U64(source_len),
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
    let created: Status =
        serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes()).unwrap();
    created.validate().unwrap();
    assert_eq!(created.phase, Some(Phase::Receiving));
    let job = created.handle.job_id;
    let base = format!("{}/{job}", http::PREFIX);
    let mut req = request(
        "PUT",
        &format!("{base}/upload"),
        incarnation,
        Some(job),
        Body::from(source),
    );
    req.headers_mut()
        .insert("content-type", "application/octet-stream".parse().unwrap());
    assert_eq!(
        router.clone().oneshot(req).await.unwrap().status(),
        StatusCode::ACCEPTED
    );
    let ready = wait(&router, &base, incarnation, job, true, &mut evidence).await;
    assert_eq!(ready.state, JobState::Running, "Large preview must succeed");
    assert_eq!(ready.phase, Some(Phase::Ready));
    assert_eq!(ready.commit.outcome, CommitOutcome::Pending);
    assert_eq!(ready.profile.name, ProfileName::Large);
    assert_eq!(
        ready.profile.max_expanded_bytes.get(),
        knx_productdb::PackageLimits::LARGE.max_expanded_size
    );
    assert_eq!(ready.source.received_bytes.get(), source_len);
    assert_eq!(ready.source.sha256.unwrap().to_string(), expected);
    assert_eq!(ready.lease.kind, LeaseKind::Ready);
    let preview = ready.preview.unwrap();
    assert!(
        preview.programs.get() > 0,
        "real package must produce installed programs"
    );
    let page = details(&router, &base, incarnation, job, &preview.artifact).await;
    assert!(!page["records"].as_array().unwrap().is_empty());
    let ready_elapsed = started.elapsed().as_secs_f64();
    let ready_disk = disk(tmp.path());
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
        Some(job),
        Body::from(serde_json::to_vec(&command).unwrap()),
    );
    req.headers_mut()
        .insert("content-type", "application/json".parse().unwrap());
    assert_eq!(
        router.clone().oneshot(req).await.unwrap().status(),
        StatusCode::ACCEPTED
    );
    let done = wait(&router, &base, incarnation, job, false, &mut evidence).await;
    assert_eq!(
        done.state,
        JobState::Succeeded,
        "Large explicit apply must succeed"
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
    let result = done.result.unwrap();
    let page = details(&router, &base, incarnation, job, &result.artifact).await;
    assert_eq!(page["records"][0]["kind"], "resultSummary");
    assert_eq!(
        page["records"][0]["cataloguePageBytes"],
        result.catalogue_page_bytes.to_string()
    );
    let backup = catalogue.parent().unwrap().join(done.backup.name.unwrap());
    let old = knx_productdb::open_and_migrate(&backup).unwrap();
    assert_eq!(
        old.query_row("SELECT count(*) FROM application_program", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        0
    );
    drop(old);
    let live_count = state
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
    assert!(live_count > 0);
    let workspace = runtime
        .lookup(
            internal_handle(&created.handle),
            &Capability::from_bytes([1; 32]),
        )
        .unwrap()
        .workspace()
        .unwrap();
    let job_dir = workspace.lock().unwrap().directory().to_path_buf();
    let artifact_disk = disk(&job_dir);
    let terminal_disk = disk(tmp.path());
    let settled_elapsed = started.elapsed().as_secs_f64();
    runtime
        .dispose(
            internal_handle(&created.handle),
            &Capability::from_bytes([1; 32]),
        )
        .await
        .unwrap();
    assert!(backup.is_file(), "disposal retains committed user backup");
    state.catalogue_jobs.close_admission_and_request_stop();
    assert_eq!(
        state
            .catalogue_jobs
            .drain(Duration::from_secs(5), std::future::pending())
            .await
            .end,
        DrainEnd::Finished
    );
    let peaks = evidence.finish_samples();
    let report = serde_json::json!({
        "schemaVersion": 1, "success": true, "profile": "large", "allowLargePackage": true,
        "inputSha256": expected, "inputBytes": source_len, "declaredDecodedBytes": expanded,
        "exceedsStandardDecodedCeiling": expanded > knx_productdb::PackageLimits::STANDARD.max_expanded_size,
        "readyElapsedSeconds": ready_elapsed, "settledElapsedSeconds": settled_elapsed,
        "observedWindowSeconds": started.elapsed().as_secs_f64(),
        "rssBeforeBytes": rss_before, "sampledPeakProcessRssBytes": peaks.0,
        "processLifetimeHwmBeforeBytes": hwm_before, "processLifetimeHwmAfterBytes": proc_kib("VmHWM:"),
        "samplingIntervalMillis": 10, "sampleCount": peaks.3,
        "sampledPeakOwnedApparentBytes": peaks.1, "sampledPeakOwnedAllocatedBytes": peaks.2,
        "readyOwnedDisk": {"apparentBytes": ready_disk.0, "allocatedBytes": ready_disk.1},
        "terminalOwnedDisk": {"apparentBytes": terminal_disk.0, "allocatedBytes": terminal_disk.1},
        "terminalArtifacts": {"apparentBytes": artifact_disk.0, "allocatedBytes": artifact_disk.1},
        "programCount": live_count, "backupRetainedAfterDispose": true,
        "scope": "One fresh test process; source read/hash, mounted upload/preview/apply, native backup verification and disposal/drain. RSS includes harness/runtime/native allocations, not isolated installer RSS. HWM is process-lifetime, before/after; sampled peaks may miss transients. Disk covers only owned test root (catalogue/journals/artifacts/backup), not corpus/build/output or all filesystem temp. No aggregate quota, universal memory bound or latency guarantee."
    });
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(output)
        .unwrap();
    serde_json::to_writer_pretty(&mut file, &report).unwrap();
    file.write_all(b"\n").unwrap();
    file.sync_all().unwrap();
    evidence.success = true;
    evidence.mark("acceptanceReportSynced");
}
