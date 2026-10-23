//! Native COMMIT-return retention and unwind ownership controls.

use super::*;
use crate::managed_catalogue::{OwnerError, QuarantineReason};
use std::cell::Cell;
use std::ffi::{c_char, c_int, c_void, CStr};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::rc::Rc;

#[derive(Default)]
struct Native {
    calls: Cell<usize>,
    commits: Cell<usize>,
    rollbacks: Cell<usize>,
    dml: Cell<usize>,
    restores: Cell<usize>,
    observations: Cell<usize>,
    releases: Cell<usize>,
}

unsafe extern "C" fn observe(
    ctx: *mut c_void,
    action: c_int,
    first: *const c_char,
    second: *const c_char,
    _: *const c_char,
    _: *const c_char,
) -> c_int {
    // SAFETY: the outer fixture pins the context and detaches before destruction.
    // No SQL, assertions or unwinding occur in this native callback.
    let n = unsafe { &*ctx.cast::<Native>() };
    n.calls.set(n.calls.get() + 1);
    let first = if first.is_null() {
        b"".as_slice()
    } else {
        unsafe { CStr::from_ptr(first) }.to_bytes()
    };
    if action == rusqlite::ffi::SQLITE_TRANSACTION {
        if first == b"COMMIT" {
            n.commits.set(n.commits.get() + 1);
        }
        if first == b"ROLLBACK" {
            n.rollbacks.set(n.rollbacks.get() + 1);
        }
    }
    if action == rusqlite::ffi::SQLITE_SAVEPOINT && first == b"RELEASE" {
        n.releases.set(n.releases.get() + 1);
    }
    if action == rusqlite::ffi::SQLITE_INSERT {
        n.dml.set(n.dml.get() + 1);
    }
    if action == rusqlite::ffi::SQLITE_PRAGMA && n.commits.get() > 0 {
        if first == b"max_page_count" && !second.is_null() {
            n.restores.set(n.restores.get() + 1);
        }
        if first == b"data_version" {
            n.observations.set(n.observations.get() + 1);
        }
    }
    rusqlite::ffi::SQLITE_OK
}

fn attach(db: *mut rusqlite::ffi::sqlite3, n: &Native) {
    assert_eq!(
        unsafe {
            rusqlite::ffi::sqlite3_set_authorizer(
                db,
                Some(observe),
                (n as *const Native).cast_mut().cast(),
            )
        },
        rusqlite::ffi::SQLITE_OK
    );
}
fn detach(db: *mut rusqlite::ffi::sqlite3) {
    assert_eq!(
        unsafe { rusqlite::ffi::sqlite3_set_authorizer(db, None, std::ptr::null_mut()) },
        rusqlite::ffi::SQLITE_OK
    );
}
fn rows(c: &crate::Connection) -> i64 {
    c.query_row("SELECT count(*) FROM package", [], |r| r.get(0))
        .unwrap()
}
fn shown(owner: &mut ManagedCatalogue, bytes: &[u8]) -> PreviewBinding {
    owner
        .preview_current_controlled(
            admit("witness.knxprod", bytes),
            &OperationContext {
                gate: &PublicationGate::new(),
                policy: ControlledPolicy::Standard,
            },
            &mut TestArtifacts::new(),
        )
        .unwrap()
        .binding
}
fn apply(
    owner: &mut ManagedCatalogue,
    bytes: &[u8],
    binding: &PreviewBinding,
    gate: &PublicationGate,
    a: &mut dyn CatalogueArtifacts,
) -> ControlledApplySettlement {
    owner.apply_current_controlled(
        admit("witness.knxprod", bytes),
        binding,
        &OperationContext {
            gate,
            policy: ControlledPolicy::Standard,
        },
        a,
    )
}

struct Probe {
    inner: TestArtifacts,
    writer: *mut rusqlite::ffi::sqlite3,
    w: Box<Native>,
    d: Rc<Native>,
    tail_panic: bool,
    immediate_panic: bool,
    immediate_observed: bool,
    precommit_panic: bool,
}
impl Probe {
    fn new(owner: &ManagedCatalogue) -> Self {
        let writer = owner
            .with_read(|c| Ok::<_, rusqlite::Error>(unsafe { c.handle() }))
            .unwrap()
            .unwrap();
        let d = Rc::new(Native::default());
        let dd = d.clone();
        let mut inner = TestArtifacts::new();
        inner.inventory.set_hook(
            DestinationRole::Rehearsal,
            Box::new(move |stage, _, c| {
                use crate::catalogue_copy::DestinationStage;
                if stage == DestinationStage::WriteOpened {
                    attach(unsafe { c.unwrap().handle() }, &dd);
                }
                if stage == DestinationStage::RehearsalClose {
                    detach(unsafe { c.unwrap().handle() });
                }
            }),
        );
        let p = Self {
            inner,
            writer,
            w: Box::default(),
            d,
            tail_panic: false,
            immediate_panic: false,
            immediate_observed: false,
            precommit_panic: false,
        };
        attach(writer, &p.w);
        p
    }
    fn autocommit(&self) -> bool {
        unsafe { rusqlite::ffi::sqlite3_get_autocommit(self.writer) != 0 }
    }
}
impl Drop for Probe {
    fn drop(&mut self) {
        detach(self.writer);
    }
}
impl CatalogueArtifacts for Probe {
    fn after_live_commit(&mut self, writer: &crate::Connection) {
        assert!(writer.is_autocommit());
        assert_eq!(rows(writer), 1);
        self.immediate_observed = true;
        if self.immediate_panic {
            panic!("witness-immediate-panic");
        }
    }
    fn before_live_install(&mut self, writer: &crate::Connection) {
        if self.precommit_panic {
            assert!(!writer.is_autocommit());
            assert_eq!(rows(writer), 0);
            assert!(self.d.dml.get() > 0);
            assert!(self.d.releases.get() > 0 || self.d.commits.get() > 0);
            assert!(self.inner.inventory.allocated(DestinationRole::Recovery));
            panic!("witness-precommit-panic");
        }
    }
    fn inventory(&mut self) -> &mut ArtifactInventory {
        if self.tail_panic && self.w.commits.get() > 0 {
            panic!("witness-tail-panic");
        }
        &mut self.inner.inventory
    }
    fn fresh_rehearsal(&mut self, size: i64) -> Result<FreshDestination, OrchestrationError> {
        self.inner.fresh_rehearsal(size)
    }
    fn fresh_recovery(&mut self, size: i64) -> Result<FreshDestination, OrchestrationError> {
        self.inner.fresh_recovery(size)
    }
}
fn panic_marker(result: std::thread::Result<ControlledApplySettlement>, marker: &str) {
    let err = result.expect_err("apply must unwind, not return a settlement");
    assert_eq!(err.downcast_ref::<&str>().copied(), Some(marker));
}

#[test]
fn public_bridge_retains_native_success_outside_its_caught_unwind() {
    use crate::managed_apply::{
        AdmittedSource, ApplyReceipt, ApplyWorkspace, ControlledFailureKind,
    };
    let (dir, mut owner) = migrated_owner();
    let bytes = valid_package();
    let source =
        AdmittedSource::admit("witness.knxprod", &bytes, ControlledPolicy::Standard).unwrap();
    let gate = PublicationGate::new();
    let mut preview_workspace = ApplyWorkspace::create(dir.path().join("public-preview")).unwrap();
    let shown = owner
        .prepare_controlled(source, &gate, &mut preview_workspace)
        .unwrap();
    let probe = Probe::new(&owner);
    let mut workspace = ApplyWorkspace::create(dir.path().join("public-apply")).unwrap();
    workspace.panic_after_commit_for_test();
    let mut receipt = ApplyReceipt::default();
    let result = owner
        .execute_controlled(source, &shown, &gate, &mut workspace, &mut receipt)
        .unwrap();
    assert!(result.panicked());
    assert_eq!(result.failure_kind(), Some(ControlledFailureKind::Panicked));
    assert_eq!(result.commit(), CommitEvidence::Committed);
    assert_eq!(
        result.health(),
        WriterHealth::Quarantined(QuarantineReason::Panic)
    );
    assert!(result.cleanup_failed());
    assert_eq!(result.artifacts(), ArtifactDisposition::Retain);
    assert!(workspace.has_recovery());
    assert_eq!(probe.w.commits.get(), 1);
    assert_eq!(probe.w.rollbacks.get(), 0);
    let witness = receipt.witness().unwrap();
    assert_eq!(
        witness.trace(),
        LiveCommitTrace::LiveCommitReturnedOk { autocommit: true }
    );
    drop(result);
    assert_eq!(receipt.take(witness.key()), Ok(witness));
    assert!(owner.retained_apply_witness().is_none());
    assert!(owner.with_read(|_| Ok::<_, ()>(())).is_err());
}

#[test]
fn foreign_gate_claim_never_authorizes_the_live_native_commit() {
    let (_dir, mut owner) = migrated_owner();
    let bytes = valid_package();
    let binding = shown(&mut owner, &bytes);
    let mut probe = Probe::new(&owner);
    let gate = PublicationGate::new();
    let foreign = PublicationGate::new();
    let result = owner.apply_current_controlled_with_claim(
        admit("witness.knxprod", &bytes),
        &binding,
        &OperationContext {
            gate: &gate,
            policy: ControlledPolicy::Standard,
        },
        &mut probe,
        &foreign,
    );
    assert_eq!(result.commit, CommitEvidence::NotCommitted);
    assert!(matches!(
        result.failure,
        Some(OrchestrationError::CommitClaimRefused)
    ));
    assert_eq!(probe.w.commits.get(), 0);
    assert_eq!(probe.w.rollbacks.get(), 1);
    assert_eq!(gate.state(), crate::operation::PublicationState::Open);
    assert_eq!(
        foreign.state(),
        crate::operation::PublicationState::CommitClaimed
    );
    assert_eq!(
        owner.retained_apply_witness().unwrap().trace,
        LiveCommitTrace::NoLiveCommitInvocation
    );
    assert!(probe.inner.inventory.allocated(DestinationRole::Recovery));
    assert_eq!(package_count(&owner), 0);
}

#[test]
fn tail_inventory_panic_quarantines_after_native_commit() {
    let (dir, mut owner) = migrated_owner();
    let bytes = valid_package();
    let binding = shown(&mut owner, &bytes);
    let mut p = Probe::new(&owner);
    p.tail_panic = true;
    let result = catch_unwind(AssertUnwindSafe(|| {
        apply(
            &mut owner,
            &bytes,
            &binding,
            &PublicationGate::new(),
            &mut p,
        )
    }));
    panic_marker(result, "witness-tail-panic");
    assert!(
        p.autocommit(),
        "actual W must be autocommit after the panic"
    );
    assert_eq!(p.w.commits.get(), 1);
    assert_eq!(p.w.rollbacks.get(), 0);
    assert!(p.w.dml.get() > 0);
    assert_eq!(p.w.restores.get(), 1);
    assert!(
        p.w.observations.get() > 0,
        "tail follows observation refresh"
    );
    let ro = crate::Connection::open_with_flags(
        dir.path().join("products.sqlite"),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    assert_eq!(rows(&ro), 1, "independent actual installed-row oracle");
    assert!(p.inner.inventory.allocated(DestinationRole::Recovery));
    println!(
        "native proof: W autocommit=true installed packages=1 commits=1 rollbacks=0 restore=1 observation=true B=owned"
    );
    assert!(
        matches!(
            owner.with_read(|_| Ok::<_, ()>(())),
            Err(OwnerError::NotHealthy(WriterHealth::Quarantined(
                QuarantineReason::Panic
            )))
        ),
        "tail inventory panic must quarantine W"
    );
    assert_eq!(
        owner.retained_apply_witness().unwrap().trace,
        LiveCommitTrace::LiveCommitReturnedOk { autocommit: true },
        "tail must retain native positive witness"
    );
}

#[test]
fn immediate_postcommit_panic_retains_native_positive_witness() {
    let (dir, mut owner) = migrated_owner();
    let bytes = valid_package();
    let binding = shown(&mut owner, &bytes);
    let mut p = Probe::new(&owner);
    p.immediate_panic = true;
    let result = catch_unwind(AssertUnwindSafe(|| {
        apply(
            &mut owner,
            &bytes,
            &binding,
            &PublicationGate::new(),
            &mut p,
        )
    }));
    panic_marker(result, "witness-immediate-panic");
    assert!(p.immediate_observed);
    assert!(p.autocommit());
    assert_eq!(p.w.commits.get(), 1);
    assert_eq!(p.w.rollbacks.get(), 0);
    assert_eq!(p.w.restores.get(), 0, "panic precedes restoration");
    let ro = crate::Connection::open_with_flags(
        dir.path().join("products.sqlite"),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    assert_eq!(rows(&ro), 1);
    assert!(p.inner.inventory.allocated(DestinationRole::Recovery));
    let record = owner
        .retained_apply_witness()
        .expect("admitted attempt retained");
    assert_eq!(
        record.trace,
        LiveCommitTrace::LiveCommitReturnedOk { autocommit: true },
        "native positive witness must survive immediate panic"
    );
    assert_eq!(record.key.incarnation, binding.catalogue.incarnation);
    assert_eq!(record.key.serial, 1);
    assert_eq!(record.shown_generation, binding.catalogue.generation);
    assert_eq!(record.source_length, bytes.len() as u64);
    assert_eq!(
        record.source_sha256,
        admit("witness.knxprod", &bytes).sha256
    );
    assert!(matches!(
        owner.with_read(|_| Ok::<_, ()>(())),
        Err(OwnerError::NotHealthy(WriterHealth::Quarantined(
            QuarantineReason::Panic
        )))
    ));
    let calls = p.w.calls.get();
    let receipt = owner.take_apply_witness(record.key).unwrap();
    assert_eq!(receipt, record);
    assert_eq!(p.w.calls.get(), calls, "handoff must not run SQL");
    assert!(owner.retained_apply_witness().is_none());
    assert!(
        owner.with_read(|_| Ok::<_, ()>(())).is_err(),
        "handoff cannot repair health"
    );
}

#[test]
fn precommit_panic_after_scratch_publication_has_no_live_witness() {
    let (_dir, mut owner) = migrated_owner();
    let bytes = valid_package();
    let binding = shown(&mut owner, &bytes);
    let mut p = Probe::new(&owner);
    p.precommit_panic = true;
    let result = catch_unwind(AssertUnwindSafe(|| {
        apply(
            &mut owner,
            &bytes,
            &binding,
            &PublicationGate::new(),
            &mut p,
        )
    }));
    panic_marker(result, "witness-precommit-panic");
    assert!(
        !p.autocommit(),
        "Ignore Drop must leave unresolved W active"
    );
    assert_eq!(p.w.commits.get(), 0);
    assert_eq!(p.w.rollbacks.get(), 0);
    assert!(p.d.dml.get() > 0);
    assert!(
        p.d.releases.get() > 0 || p.d.commits.get() > 0,
        "actual D publication only"
    );
    assert_eq!(p.w.dml.get(), 0);
    assert!(p.inner.inventory.allocated(DestinationRole::Recovery));
    let record = owner.retained_apply_witness().unwrap();
    assert_eq!(
        record.trace,
        LiveCommitTrace::NoLiveCommitInvocation,
        "D publication must not witness W"
    );
    assert!(matches!(
        owner.with_read(|_| Ok::<_, ()>(())),
        Err(OwnerError::NotHealthy(WriterHealth::Quarantined(
            QuarantineReason::Panic
        )))
    ));
    let calls = p.w.calls.get();
    assert_eq!(owner.take_apply_witness(record.key), Ok(record));
    assert_eq!(p.w.calls.get(), calls);
    assert!(!p.autocommit(), "metadata handoff must not settle W");
    assert!(owner.with_read(|_| Ok::<_, ()>(())).is_err());
    println!(
        "native precommit proof: active W commits=0 rollbacks=0 D releases={} D commits={} B=owned",
        p.d.releases.get(),
        p.d.commits.get()
    );
    // Fixture-only settlement AFTER all product-state observations.
    detach(p.writer);
    assert_eq!(
        unsafe {
            rusqlite::ffi::sqlite3_exec(
                p.writer,
                c"ROLLBACK".as_ptr(),
                None,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            )
        },
        rusqlite::ffi::SQLITE_OK
    );
}

#[test]
fn result_loss_pending_refusal_and_exact_key_transfer_preserve_identity() {
    let (_dir, mut owner) = migrated_owner();
    let bytes = valid_package();
    let binding = shown(&mut owner, &bytes);
    let mut p = Probe::new(&owner);
    let settlement = apply(
        &mut owner,
        &bytes,
        &binding,
        &PublicationGate::new(),
        &mut p,
    );
    assert_eq!(settlement.commit, CommitEvidence::Committed);
    let record = owner.retained_apply_witness().unwrap();
    drop(settlement);
    assert_eq!(
        owner.retained_apply_witness(),
        Some(record),
        "response loss cannot consume record"
    );
    assert_eq!(
        record.trace,
        LiveCommitTrace::LiveCommitReturnedOk { autocommit: true }
    );
    let calls = p.w.calls.get();
    let mut rejected_artifacts = TestArtifacts::new();
    let refused = apply(
        &mut owner,
        &bytes,
        &binding,
        &PublicationGate::new(),
        &mut rejected_artifacts,
    );
    assert!(
        matches!(
            refused.failure,
            Some(OrchestrationError::ApplyWitnessPending)
        ),
        "pending witness must refuse next apply"
    );
    assert_eq!(
        refused.commit,
        CommitEvidence::NotCommitted,
        "old record is not rejected call outcome"
    );
    assert_eq!(refused.rollback, RollbackDisposition::NotApplicable);
    assert_eq!(refused.artifacts, ArtifactDisposition::None);
    assert_eq!(rejected_artifacts.counter, 0);
    assert_eq!(
        p.w.calls.get(),
        calls,
        "pending refusal must precede all SQL"
    );
    assert_eq!(owner.retained_apply_witness(), Some(record));
    let wrong = ApplyAttemptKey {
        serial: record.key.serial + 1,
        ..record.key
    };
    assert_eq!(
        owner.take_apply_witness(wrong),
        Err(ApplyWitnessTransferError::KeyMismatch),
        "mismatched serial must refuse transfer"
    );
    let (_other_dir, mut other) = migrated_owner();
    let wrong_incarnation = ApplyAttemptKey {
        incarnation: other
            .capture_stamp(&PublicationGate::new())
            .unwrap()
            .incarnation,
        ..record.key
    };
    assert_eq!(
        owner.take_apply_witness(wrong_incarnation),
        Err(ApplyWitnessTransferError::KeyMismatch),
        "mismatched incarnation must refuse transfer"
    );
    assert_eq!(owner.retained_apply_witness(), Some(record));
    let receipt = owner.take_apply_witness(record.key).unwrap();
    assert_eq!(receipt, record);
    assert_eq!(p.w.calls.get(), calls, "handoff is metadata-only");
    assert!(owner.retained_apply_witness().is_none());
    assert_eq!(
        owner.take_apply_witness(record.key),
        Err(ApplyWitnessTransferError::Absent)
    );
    assert_eq!(package_count(&owner), 1);
    let next_binding = shown(&mut owner, &bytes);
    let cancel = PublicationGate::new();
    assert_eq!(
        cancel.try_cancel(),
        crate::operation::CancelOutcome::Accepted
    );
    let calls = p.w.calls.get();
    let refusal = apply(
        &mut owner,
        &bytes,
        &next_binding,
        &cancel,
        &mut rejected_artifacts,
    );
    assert!(matches!(refusal.failure, Some(OrchestrationError::Stop(_))));
    let next = owner.retained_apply_witness().unwrap();
    assert_ne!(next.key, receipt.key, "attempt keys must never be reused");
    assert_eq!(
        next.key.serial,
        receipt.key.serial + 1,
        "refused pending call cannot consume serial"
    );
    assert_eq!(next.key.incarnation, receipt.key.incarnation);
    assert_eq!(next.trace, LiveCommitTrace::NoLiveCommitInvocation);
    assert_eq!(
        receipt, record,
        "later attempt cannot alter transferred receipt"
    );
    assert_eq!(p.w.calls.get(), calls);
    assert_eq!(rejected_artifacts.counter, 0);
}
