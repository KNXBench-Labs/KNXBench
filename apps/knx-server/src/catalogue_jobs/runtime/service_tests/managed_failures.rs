//! Native failure regressions reserved for the combined candidate campaign.

use super::*;
use crate::SharedState;
use std::ffi::{c_char, c_void, CStr};
use std::io::Write;
use std::os::unix::fs::MetadataExt;
use std::sync::atomic::AtomicUsize;

#[derive(Clone, Copy)]
enum Fault {
    Install,
    InstallAndRestore,
    Commit,
}
struct NativeProbe {
    fault: Fault,
    commits: AtomicUsize,
    rollbacks: AtomicUsize,
    refused_installs: AtomicUsize,
    cap_sets: AtomicUsize,
    refused_restores: AtomicUsize,
}
unsafe extern "C" fn authorize(
    context: *mut c_void,
    action: i32,
    first: *const c_char,
    second: *const c_char,
    _: *const c_char,
    _: *const c_char,
) -> i32 {
    // Only atomics/borrowed bytes, no SQL, allocation or unwind across FFI.
    let probe = unsafe { &*context.cast::<NativeProbe>() };
    let first = if first.is_null() {
        b"".as_slice()
    } else {
        unsafe { CStr::from_ptr(first) }.to_bytes()
    };
    if action == rusqlite::ffi::SQLITE_TRANSACTION {
        if first == b"COMMIT" {
            probe.commits.fetch_add(1, Ordering::SeqCst);
            if matches!(probe.fault, Fault::Commit) {
                return rusqlite::ffi::SQLITE_DENY;
            }
        } else if first == b"ROLLBACK" {
            probe.rollbacks.fetch_add(1, Ordering::SeqCst);
        }
    }
    if action == rusqlite::ffi::SQLITE_INSERT
        && first == b"package"
        && matches!(probe.fault, Fault::Install | Fault::InstallAndRestore)
    {
        probe.refused_installs.fetch_add(1, Ordering::SeqCst);
        return rusqlite::ffi::SQLITE_DENY;
    }
    if action == rusqlite::ffi::SQLITE_PRAGMA && first == b"max_page_count" && !second.is_null() {
        let prior_sets = probe.cap_sets.fetch_add(1, Ordering::SeqCst);
        if prior_sets != 0 && matches!(probe.fault, Fault::InstallAndRestore) {
            probe.refused_restores.fetch_add(1, Ordering::SeqCst);
            return rusqlite::ffi::SQLITE_DENY;
        }
    }
    rusqlite::ffi::SQLITE_OK
}
struct Probe {
    // The quarantined managed owner retains this exact connection until drop.
    state: SharedState,
    db: *mut rusqlite::ffi::sqlite3,
    native: Box<NativeProbe>,
}
impl Drop for Probe {
    fn drop(&mut self) {
        // Healthy-owner detachment holds the same mutex as a queued/running
        // apply. An unhealthy/poisoned owner has already left that operation
        // and refuses subsequent SQL; no recovery or new job is sent here.
        let detached = self.state.product_db.as_ref().unwrap().with_read(|_| {
            unsafe {
                rusqlite::ffi::sqlite3_set_authorizer(self.db, None, std::ptr::null_mut());
            }
            Ok::<_, std::convert::Infallible>(())
        });
        if detached.is_err() {
            unsafe {
                rusqlite::ffi::sqlite3_set_authorizer(self.db, None, std::ptr::null_mut());
            }
        }
    }
}
impl Probe {
    fn attach(state: SharedState, fault: Fault) -> Self {
        let native = Box::new(NativeProbe {
            fault,
            commits: AtomicUsize::new(0),
            rollbacks: AtomicUsize::new(0),
            refused_installs: AtomicUsize::new(0),
            cap_sets: AtomicUsize::new(0),
            refused_restores: AtomicUsize::new(0),
        });
        let db = state
            .product_db
            .as_ref()
            .unwrap()
            .with_read(|conn| {
                let db = unsafe { conn.handle() };
                assert_eq!(
                    unsafe {
                        rusqlite::ffi::sqlite3_set_authorizer(
                            db,
                            Some(authorize),
                            (&*native as *const NativeProbe).cast_mut().cast(),
                        )
                    },
                    rusqlite::ffi::SQLITE_OK
                );
                Ok::<_, std::convert::Infallible>(db)
            })
            .unwrap()
            .unwrap();
        Self { state, db, native }
    }
}
async fn wait_for(
    runtime: &CatalogueRuntime,
    handle: crate::catalogue_jobs::Handle,
    ready: bool,
) -> Status {
    let capability = Capability::from_bytes([1; 32]);
    tokio::time::timeout(Duration::from_secs(20), async {
        loop {
            let s = runtime.status(handle, &capability).unwrap();
            s.validate().unwrap();
            if ready {
                assert_eq!(s.state, JobState::Running, "preview must reach Ready");
                if s.phase == Some(Phase::Ready) {
                    break s;
                }
            } else if s.state != JobState::Running {
                break s;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap()
}
async fn native_failure(fault: Fault) {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("catalogue.sqlite");
    let state = Arc::new(crate::AppState::with_product_db_at(
        tmp.path().to_path_buf(),
        Some(path.clone()),
    ));
    state
        .product_db
        .as_ref()
        .unwrap()
        .with_read(|conn| conn.execute_batch("PRAGMA main.max_page_count=100000000"))
        .unwrap()
        .unwrap();
    let _router = crate::app(Arc::clone(&state), None);
    let runtime = state.catalogue_jobs.attached_runtime().unwrap();
    let capability = Capability::from_bytes([1; 32]);
    let mut zip = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    zip.start_file("knx_master.xml", zip::write::SimpleFileOptions::default())
        .unwrap();
    zip.write_all(br#"<KNX xmlns="http://knx.org/xml/project/20"><MasterData><Manufacturers><Manufacturer Id="M-0001" Name="Example"/></Manufacturers></MasterData></KNX>"#).unwrap();
    zip.start_file(
        "M-0001/Hardware.xml",
        zip::write::SimpleFileOptions::default(),
    )
    .unwrap();
    zip.write_all(br#"<KNX xmlns="http://knx.org/xml/project/20"><ManufacturerData><Manufacturer RefId="M-0001"><Hardware><Hardware Id="H-1" Name="Example"/></Hardware></Manufacturer></ManufacturerData></KNX>"#).unwrap();
    let source = zip.finish().unwrap().into_inner();
    let (_, created) = runtime
        .create(
            &capability,
            Create {
                schema_version: 1,
                policy_version: POLICY_VERSION.into(),
                server_incarnation: runtime.admission().unwrap().server_incarnation,
                admission_serial: U64(1),
                source_name: "native-failure.knxprod".into(),
                declared_bytes: U64(source.len() as u64),
                profile: ProfileName::Standard,
                allow_large_package: false,
            },
        )
        .unwrap();
    let handle = internal_handle(&created.handle);
    runtime
        .upload(handle, &capability, Body::from(source))
        .await
        .unwrap();
    let ready = wait_for(&runtime, handle, true).await;
    let preview = ready.preview.unwrap();
    let probe = Probe::attach(Arc::clone(&state), fault);
    let command = ApplyCommand {
        schema_version: 1,
        command_id: JobId::from_bytes([3; 16]),
        consent_id: preview.consent_id,
        preview_artifact_id: preview.artifact.artifact_id,
        preview_public_sha256: preview.artifact.public_sha256,
        confirmed: true,
    };
    assert!(runtime.apply(handle, &capability, &command).unwrap().0);
    let done = wait_for(&runtime, handle, false).await;
    assert_eq!(done.state, JobState::Failed);
    assert_eq!(probe.native.rollbacks.load(Ordering::SeqCst), 1);
    assert_eq!(probe.native.cap_sets.load(Ordering::SeqCst), 2);
    // Authorizer never refuses rollback; verify its native transaction ended.
    assert_eq!(
        unsafe { rusqlite::ffi::sqlite3_get_autocommit(probe.db) },
        1
    );
    let ambiguous = matches!(fault, Fault::Commit);
    assert_eq!(
        done.failure.as_ref().unwrap().kind,
        if ambiguous {
            FailureKind::DatabaseFailure
        } else {
            FailureKind::PackageRefused
        }
    );
    assert_eq!(
        probe.native.commits.load(Ordering::SeqCst),
        usize::from(ambiguous)
    );
    assert_eq!(
        probe.native.refused_installs.load(Ordering::SeqCst),
        usize::from(!ambiguous)
    );
    assert_eq!(
        done.commit,
        Commit {
            outcome: if ambiguous {
                CommitOutcome::Unknown
            } else {
                CommitOutcome::NotCommitted
            },
            evidence: if ambiguous {
                CommitEvidence::AmbiguousCommit
            } else {
                CommitEvidence::ExplicitRollback
            },
        }
    );
    assert!(
        done.result.is_none(),
        "an admitted result is not a COMMIT witness"
    );
    let retain = !matches!(fault, Fault::Install);
    assert_eq!(
        done.cleanup.disposition,
        if retain {
            CleanupDisposition::Quarantined
        } else {
            CleanupDisposition::Complete
        }
    );
    assert_eq!(
        probe.native.refused_restores.load(Ordering::SeqCst),
        usize::from(matches!(fault, Fault::InstallAndRestore))
    );
    let workspace = runtime
        .lookup(handle, &capability)
        .unwrap()
        .workspace()
        .unwrap();
    let job_dir = workspace.lock().unwrap().directory().to_path_buf();
    let inventory: serde_json::Value =
        serde_json::from_slice(&std::fs::read(job_dir.join("inventory.json")).unwrap()).unwrap();
    if retain {
        assert_eq!(inventory["health"], "quarantined");
        assert_eq!(done.backup.state, BackupState::Retained);
        assert_eq!(
            done.backup.publication,
            BackupPublication::FileAndDirectorySyncAcknowledged
        );
        let publication = &inventory["recoveryPublication"];
        assert_eq!(publication["state"], "retained");
        assert_eq!(publication["name"], done.backup.name.as_deref().unwrap());
        assert_eq!(
            publication["identity"],
            done.backup.identity.as_deref().unwrap()
        );
        let backup = tmp.path().join(done.backup.name.as_ref().unwrap());
        let meta = std::fs::symlink_metadata(&backup).unwrap();
        assert_eq!(publication["device"], meta.dev().to_string());
        assert_eq!(publication["inode"], meta.ino().to_string());
        assert_eq!(publication["bytes"], meta.len().to_string());
        let before = knx_productdb::Connection::open_with_flags(
            &backup,
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
        )
        .unwrap();
        assert_eq!(
            before
                .query_row("SELECT count(*) FROM package", [], |r| r.get::<_, i64>(0))
                .unwrap(),
            0
        );
        assert!(probe
            .state
            .product_db
            .as_ref()
            .unwrap()
            .with_read(|_| Ok::<_, rusqlite::Error>(()))
            .is_err());
    } else {
        assert_eq!(done.backup.state, BackupState::None);
        assert!(inventory["recoveryPublication"].is_null());
        assert!(!std::fs::read_dir(tmp.path()).unwrap().any(|entry| entry
            .unwrap()
            .file_name()
            .to_string_lossy()
            .starts_with("catalogue.sqlite.backup-")));
    }
    assert!(!runtime.apply(handle, &capability, &command).unwrap().0);
    state.catalogue_jobs.close_admission_and_request_stop();
    assert_eq!(
        state
            .catalogue_jobs
            .drain(Duration::from_secs(5), std::future::pending())
            .await
            .end,
        DrainEnd::Finished
    );
    drop(probe);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn native_not_committed_restore_failure_retains_exact_published_backup() {
    native_failure(Fault::InstallAndRestore).await;
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn native_ambiguous_commit_with_successful_rollback_stays_unknown() {
    native_failure(Fault::Commit).await;
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn native_clean_not_committed_rollback_disposes_published_backup() {
    native_failure(Fault::Install).await;
}
