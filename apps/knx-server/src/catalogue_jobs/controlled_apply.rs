//! Same-gate managed apply bridge. The job supervisor owns the receipt and
//! workspace outside worker/response unwind; this module does not publish HTTP.
use super::{Clock, Control, Error, Phase};
use crate::catalogue_owner::{CatalogueOwner, ControlledOwnerError};
use knx_productdb::managed_apply::{
    AdmittedSource, ApplyExecution, ApplyReceipt, ApplyWorkspace, PreparedPreview,
};
use knx_productdb::operation::CommitArbitration;
use std::sync::OnceLock;

/// Immutable actual ProductDB preview, set once before Ready consent is offered.
pub(crate) type ManagedApplyControl<C> = Control<OnceLock<PreparedPreview>, C>;

#[derive(Debug)]
pub(crate) enum BridgeError {
    Control(Error),
    Owner(ControlledOwnerError),
    PreviewPending,
    PreviewAlreadyBound,
}

pub(crate) fn prepare<C: Clock>(
    owner: &CatalogueOwner,
    source: AdmittedSource<'_>,
    control: &ManagedApplyControl<C>,
    workspace: &mut ApplyWorkspace,
) -> Result<(), BridgeError> {
    let view = control.snapshot().map_err(BridgeError::Control)?;
    if view.phase != Phase::Previewing {
        return Err(BridgeError::Control(Error::InvalidPhase));
    }
    if control.binding().get().is_some() {
        return Err(BridgeError::PreviewAlreadyBound);
    }
    let preview = owner
        .try_prepare_controlled(source, control.gate(), workspace)
        .map_err(BridgeError::Owner)?;
    control
        .binding()
        .set(preview)
        .map_err(|_| BridgeError::PreviewAlreadyBound)
}

pub(crate) fn prepare_owned<C: Clock>(
    owner: &CatalogueOwner,
    source: AdmittedSource<'_>,
    control: &ManagedApplyControl<C>,
    workspace: &mut ApplyWorkspace,
    publisher: &mut dyn knx_productdb::managed_apply::RecoveryPublisher,
) -> Result<(), BridgeError> {
    let view = control.snapshot().map_err(BridgeError::Control)?;
    if view.phase != Phase::Previewing {
        return Err(BridgeError::Control(Error::InvalidPhase));
    }
    if control.binding().get().is_some() {
        return Err(BridgeError::PreviewAlreadyBound);
    }
    let preview = owner
        .try_prepare_controlled_owned(source, control.gate(), workspace, publisher)
        .map_err(BridgeError::Owner)?;
    control
        .binding()
        .set(preview)
        .map_err(|_| BridgeError::PreviewAlreadyBound)
}

/// Called only after the exact command has consumed Ready. Busy means no owner
/// was acquired: the supervisor may wait with cancellation/deadline checkpoints,
/// without holding the short control mutex. No detached retry or task is spawned.
pub(crate) fn apply<C: Clock>(
    owner: &CatalogueOwner,
    source: AdmittedSource<'_>,
    control: &ManagedApplyControl<C>,
    workspace: &mut ApplyWorkspace,
    receipt: &mut ApplyReceipt,
    publisher: &mut dyn knx_productdb::managed_apply::RecoveryPublisher,
) -> Result<ApplyExecution, BridgeError> {
    let view = control.snapshot().map_err(BridgeError::Control)?;
    if view.phase != Phase::WaitingForWriter || view.accepted.is_none() {
        return Err(BridgeError::Control(Error::InvalidPhase));
    }
    let preview = control.binding().get().ok_or(BridgeError::PreviewPending)?;
    owner
        .try_apply_controlled(source, preview, control, workspace, receipt, publisher)
        .map_err(BridgeError::Owner)
}

#[cfg(test)]
mod tests {
    use super::super::{Cause, Command, Identity, ReadyEvidence};
    use super::*;
    use knx_productdb::managed_apply::{CommitEvidence, ControlledPolicy, LiveCommitTrace};
    use knx_productdb::operation::{CancelOutcome, CommitClaim, PublicationGate, PublicationState};
    use std::io::Write;
    use std::panic::{catch_unwind, AssertUnwindSafe};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{mpsc, Arc, Mutex};
    use std::time::Duration;

    struct TestPublisher;
    impl knx_productdb::managed_apply::RecoveryPublisher for TestPublisher {
        fn admit_result(
            &mut self,
            _: &knx_productdb::ProductFileInstall,
            _: u64,
        ) -> Result<(), knx_productdb::managed_apply::RecoveryPublicationError> {
            Ok(())
        }
        fn allocated(
            &mut self,
            _: &std::path::Path,
            _: &std::fs::File,
        ) -> Result<(), knx_productdb::managed_apply::RecoveryPublicationError> {
            Ok(())
        }
        fn publish(
            &mut self,
            mut copy: knx_productdb::managed_apply::ValidatedRecovery<'_>,
        ) -> Result<
            knx_productdb::managed_apply::PublishedRecovery,
            knx_productdb::managed_apply::RecoveryPublicationError,
        > {
            let parent = copy.path().parent().unwrap().to_path_buf();
            copy.publish_to(&parent, "published-recovery.sqlite")
        }
    }

    #[derive(Clone)]
    struct ManualClock(Arc<Mutex<Duration>>);
    impl Clock for ManualClock {
        fn sample(&self) -> Result<Duration, ()> {
            Ok(*self.0.lock().unwrap())
        }
    }
    impl ManualClock {
        fn set(&self, value: Duration) {
            *self.0.lock().unwrap() = value;
        }
    }
    #[derive(Default)]
    struct Native {
        commits: AtomicUsize,
        rollbacks: AtomicUsize,
    }
    unsafe extern "C" fn observe(
        context: *mut std::ffi::c_void,
        action: i32,
        first: *const std::ffi::c_char,
        _: *const std::ffi::c_char,
        _: *const std::ffi::c_char,
        _: *const std::ffi::c_char,
    ) -> i32 {
        // Stable context, detached while its owning connection is still alive.
        // No SQL, allocations, assertions or unwinding through SQLite FFI.
        let native = unsafe { &*context.cast::<Native>() };
        if action == rusqlite::ffi::SQLITE_TRANSACTION && !first.is_null() {
            let name = unsafe { std::ffi::CStr::from_ptr(first) }.to_bytes();
            if name == b"COMMIT" {
                native.commits.fetch_add(1, Ordering::SeqCst);
            }
            if name == b"ROLLBACK" {
                native.rollbacks.fetch_add(1, Ordering::SeqCst);
            }
        }
        rusqlite::ffi::SQLITE_OK
    }
    struct Probe {
        _owner: Arc<CatalogueOwner>,
        db: *mut rusqlite::ffi::sqlite3,
        native: Box<Native>,
    }
    impl Drop for Probe {
        fn drop(&mut self) {
            unsafe {
                rusqlite::ffi::sqlite3_set_authorizer(self.db, None, std::ptr::null_mut());
            }
        }
    }
    fn source(bytes: &[u8]) -> AdmittedSource<'_> {
        AdmittedSource::admit("gate.knxprod", bytes, ControlledPolicy::Standard).unwrap()
    }
    fn identity() -> Identity {
        Identity {
            incarnation: [1; 16],
            serial: 1,
            job: [2; 16],
        }
    }
    fn evidence() -> ReadyEvidence {
        ReadyEvidence {
            consent: [3; 16],
            artifact: [4; 16],
            digest: [5; 32],
        }
    }
    fn command() -> Command {
        Command {
            command: [6; 16],
            consent: [3; 16],
            artifact: [4; 16],
            digest: [5; 32],
            confirmed: true,
        }
    }
    fn fixture(
        dir: &std::path::Path,
    ) -> (
        Arc<CatalogueOwner>,
        ManagedApplyControl<ManualClock>,
        ManualClock,
        Vec<u8>,
        Probe,
    ) {
        let path = dir.join("catalogue.sqlite");
        let conn = knx_productdb::open_and_migrate(&path).unwrap();
        let db = unsafe { conn.handle() };
        let owner = Arc::new(CatalogueOwner::adopt_opened(conn, path).unwrap());
        let clock = ManualClock(Arc::new(Mutex::new(Duration::ZERO)));
        let control = Control::new(identity(), OnceLock::new(), clock.clone()).unwrap();
        let mut zip = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
        zip.start_file("knx_master.xml", zip::write::SimpleFileOptions::default())
            .unwrap();
        zip.write_all(br#"<KNX xmlns="http://knx.org/xml/project/11"><MasterData><Manufacturers><Manufacturer Id="M-0001" Name="Example"/></Manufacturers></MasterData></KNX>"#).unwrap();
        zip.start_file(
            "M-0001/Hardware.xml",
            zip::write::SimpleFileOptions::default(),
        )
        .unwrap();
        zip.write_all(br#"<KNX xmlns="http://knx.org/xml/project/11"><ManufacturerData><Manufacturer RefId="M-0001"><Hardware><Hardware Id="H-1" Name="Example"/></Hardware></Manufacturer></ManufacturerData></KNX>"#).unwrap();
        let bytes = zip.finish().unwrap().into_inner();
        let mut workspace = ApplyWorkspace::create(dir.join("preview")).unwrap();
        prepare(&owner, source(&bytes), &control, &mut workspace).unwrap();
        control
            .enter_ready(control.ticket().unwrap(), evidence())
            .unwrap();
        control.apply(identity(), command()).unwrap();
        let probe = Probe {
            _owner: Arc::clone(&owner),
            db,
            native: Box::default(),
        };
        assert_eq!(
            unsafe {
                rusqlite::ffi::sqlite3_set_authorizer(
                    db,
                    Some(observe),
                    (&*probe.native as *const Native).cast_mut().cast(),
                )
            },
            rusqlite::ffi::SQLITE_OK
        );
        (owner, control, clock, bytes, probe)
    }
    struct Fence<'a> {
        control: &'a ManagedApplyControl<ManualClock>,
        reached: mpsc::Sender<()>,
        release: mpsc::Receiver<()>,
        after_claim: bool,
    }
    impl CommitArbitration for Fence<'_> {
        fn gate(&self) -> &PublicationGate {
            self.control.gate()
        }
        fn claim_commit(&self) -> Option<CommitClaim<'_>> {
            if self.after_claim {
                let claim = self.control.claim_commit();
                self.reached.send(()).unwrap();
                self.release.recv_timeout(Duration::from_secs(10)).unwrap();
                claim
            } else {
                self.reached.send(()).unwrap();
                self.release.recv_timeout(Duration::from_secs(10)).unwrap();
                self.control.claim_commit()
            }
        }
    }

    #[test]
    fn b10_real_native_commit_and_stop_deadline_expiry_share_one_gate() {
        for schedule in ["cancel", "stop", "deadline", "expiry", "commit"] {
            let dir = tempfile::tempdir().unwrap();
            let (owner, control, clock, bytes, probe) = fixture(dir.path());
            let mut workspace = ApplyWorkspace::create(dir.path().join("apply")).unwrap();
            let mut receipt = ApplyReceipt::default();
            let (reached_tx, reached_rx) = mpsc::channel();
            let (release_tx, release_rx) = mpsc::channel();
            let (execution, at_claim, request, reached) = std::thread::scope(|scope| {
                let owner = &owner;
                let control = &control;
                let bytes = &bytes;
                let workspace = &mut workspace;
                let receipt = &mut receipt;
                let worker = scope.spawn(move || {
                    let fence = Fence {
                        control,
                        reached: reached_tx,
                        release: release_rx,
                        after_claim: schedule == "commit",
                    };
                    owner
                        .try_apply_controlled(
                            source(bytes),
                            control.binding().get().unwrap(),
                            &fence,
                            workspace,
                            receipt,
                            &mut TestPublisher,
                        )
                        .unwrap()
                });
                let reached = reached_rx.recv_timeout(Duration::from_secs(10));
                let at_claim = probe.native.commits.load(Ordering::SeqCst);
                let request = match schedule {
                    "cancel" => Some(control.request_stop(identity(), Cause::UserCancel)),
                    "stop" => Some(control.request_stop(identity(), Cause::ServerStop)),
                    "deadline" => {
                        clock.set(Duration::from_secs(7200));
                        None
                    }
                    "expiry" => {
                        clock.set(Duration::from_secs(7200));
                        Some(
                            control
                                .ticket()
                                .and_then(|ticket| control.expire(ticket))
                                .and_then(|expiry| match expiry {
                                    super::super::Expiry::Stop(outcome, _) => Ok(outcome),
                                    _ => Err(Error::InvalidPhase),
                                }),
                        )
                    }
                    "commit" => Some(control.request_stop(identity(), Cause::ServerStop)),
                    _ => unreachable!(),
                };
                let _ = release_tx.send(());
                (worker.join(), at_claim, request, reached)
            });
            // Every worker is joined before assertions, including defect paths.
            let execution = execution.unwrap();
            assert!(
                reached.is_ok(),
                "actual precommit arbitration seam must be reached"
            );
            assert_eq!(at_claim, 0, "claim is not native COMMIT");
            assert_eq!(
                request,
                if schedule == "deadline" {
                    None
                } else if schedule == "commit" {
                    Some(Ok(CancelOutcome::TooLate))
                } else {
                    Some(Ok(CancelOutcome::Accepted))
                }
            );
            assert!(workspace.has_recovery());
            let native_commits = probe.native.commits.load(Ordering::SeqCst);
            let native_rollbacks = probe.native.rollbacks.load(Ordering::SeqCst);
            let read = knx_productdb::Connection::open_with_flags(
                dir.path().join("catalogue.sqlite"),
                rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
            )
            .unwrap();
            let rows: i64 = read
                .query_row("SELECT count(*) FROM package", [], |r| r.get(0))
                .unwrap();
            if schedule == "commit" {
                assert_eq!(control.gate().state(), PublicationState::CommitClaimed);
                assert_eq!(execution.commit(), CommitEvidence::Committed);
                assert_eq!(
                    receipt.witness().unwrap().trace(),
                    LiveCommitTrace::LiveCommitReturnedOk { autocommit: true }
                );
                assert_eq!((native_commits, native_rollbacks, rows), (1, 0, 1));
            } else {
                assert_eq!(control.gate().state(), PublicationState::CancelAccepted);
                assert_eq!(execution.commit(), CommitEvidence::NotCommitted);
                assert_eq!(
                    receipt.witness().unwrap().trace(),
                    LiveCommitTrace::NoLiveCommitInvocation
                );
                assert_eq!((native_commits, native_rollbacks, rows), (0, 1, 0));
            }
        }
    }

    struct NativeResultObservation(usize);
    impl crate::catalogue_jobs::publication::JobResultAdmission for NativeResultObservation {
        fn admit_result(
            &mut self,
            _: &knx_productdb::ProductFileInstall,
            bytes: u64,
            _: &mut crate::catalogue_jobs::persistence::JobWorkspace,
        ) -> Result<(), knx_productdb::managed_apply::RecoveryPublicationError> {
            assert!(bytes > 0);
            self.0 += 1;
            Ok(())
        }
    }

    #[test]
    fn b10_actual_job_artifact_publisher_acknowledges_listed_backup_before_live_dml() {
        use crate::catalogue_jobs::{
            persistence::JobWorkspace, publication::JobRecoveryPublisher, ArtifactRoot, Handle,
            Incarnation, JobId, Sha256 as WireSha, U64,
        };
        use sha2::{Digest, Sha256};
        let dir = tempfile::tempdir().unwrap();
        let (owner, control, _clock, bytes, probe) = fixture(dir.path());
        let root = Arc::new(ArtifactRoot::acquire(dir.path()).unwrap());
        let incarnation = Incarnation::from_bytes([1; 16]);
        let incdir = root.create_incarnation_dir(&incarnation).unwrap();
        let mut job = JobWorkspace::create(
            root,
            &incdir,
            Handle {
                incarnation,
                serial: U64(1),
                job: JobId::from_bytes([2; 16]),
            },
        )
        .unwrap();
        let mut workspace =
            ApplyWorkspace::in_owned_directory(job.directory().to_path_buf()).unwrap();
        let mut receipt = ApplyReceipt::default();
        let digest = WireSha::from_bytes(Sha256::digest(&bytes).into());
        let mut result_observation = NativeResultObservation(0);
        let result = {
            let mut publisher = JobRecoveryPublisher::new(
                &mut job,
                &dir.path().join("catalogue.sqlite"),
                digest,
                &mut result_observation,
            );
            apply(
                &owner,
                source(&bytes),
                &control,
                &mut workspace,
                &mut receipt,
                &mut publisher,
            )
            .unwrap()
        };
        assert_eq!(result.commit(), CommitEvidence::Committed);
        assert_eq!(result_observation.0, 1);
        assert_eq!(probe.native.commits.load(Ordering::SeqCst), 1);
        assert_eq!(probe.native.rollbacks.load(Ordering::SeqCst), 0);
        let publication = job.recovery_publication().unwrap();
        assert!(publication.sync_acknowledged);
        let backup = dir.path().join(&publication.name);
        assert!(backup.is_file());
        assert!(publication.name.starts_with("catalogue.sqlite.backup-"));
        let before = rusqlite::Connection::open_with_flags(
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
        before.close().unwrap();
        assert_eq!(
            owner
                .with_read(
                    |c| c.query_row("SELECT count(*) FROM package", [], |r| r.get::<_, i64>(0))
                )
                .unwrap()
                .unwrap(),
            1
        );
        assert!(workspace.native_handles_closed());
        workspace.release_closed_ownership().unwrap();
        job.cleanup_transients().unwrap();
        job.retain_recovery().unwrap();
        job.dispose().unwrap();
        assert!(backup.is_file());
    }

    #[test]
    fn b10_outer_receipt_and_recovery_survive_server_projection_panic_and_poison() {
        let dir = tempfile::tempdir().unwrap();
        let (owner, control, _clock, bytes, probe) = fixture(dir.path());
        let mut workspace = ApplyWorkspace::create(dir.path().join("apply")).unwrap();
        let mut receipt = ApplyReceipt::default();
        owner.panic_after_handoff_for_test();
        let unwind = catch_unwind(AssertUnwindSafe(|| {
            apply(
                &owner,
                source(&bytes),
                &control,
                &mut workspace,
                &mut receipt,
                &mut TestPublisher,
            )
        }));
        let panic = unwind.expect_err("server result projection must unwind");
        assert_eq!(
            panic.downcast_ref::<&str>().copied(),
            Some("server-post-handoff-panic")
        );
        assert_eq!(probe.native.commits.load(Ordering::SeqCst), 1);
        assert_eq!(probe.native.rollbacks.load(Ordering::SeqCst), 0);
        let witness = receipt.witness().unwrap();
        assert_eq!(
            witness.trace(),
            LiveCommitTrace::LiveCommitReturnedOk { autocommit: true }
        );
        assert!(workspace.has_recovery());
        assert!(
            owner.with_read(|_| Ok::<_, ()>(())).is_err(),
            "poison must refuse SQL"
        );
        assert_eq!(receipt.take(witness.key()), Ok(witness));
        assert!(receipt.witness().is_none());
        assert!(
            owner.with_read(|_| Ok::<_, ()>(())).is_err(),
            "metadata handoff must not clear poison"
        );
    }
}
