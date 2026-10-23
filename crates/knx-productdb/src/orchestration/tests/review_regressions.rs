//! Regression controls for retained reservations, policy and native rollback faults.

use super::*;
use crate::managed_catalogue::QuarantineReason;
use std::ffi::{c_char, c_int, c_void, CStr};

fn preview(owner: &mut ManagedCatalogue, bytes: &[u8], policy: ControlledPolicy) -> PreviewBinding {
    let gate = PublicationGate::new();
    owner
        .preview_current_controlled(
            ControlledSource::admit("review.knxprod", bytes, policy).unwrap(),
            &OperationContext {
                gate: &gate,
                policy,
            },
            &mut TestArtifacts::new(),
        )
        .unwrap()
        .binding
}

fn refuses_changed_policy(before: ControlledPolicy, after: ControlledPolicy) {
    let (_dir, mut owner) = migrated_owner();
    let bytes = valid_package();
    let shown = preview(&mut owner, &bytes, before);
    let gate = PublicationGate::new();
    let mut artifacts = TestArtifacts::new();
    let settlement = owner.apply_current_controlled(
        ControlledSource::admit("review.knxprod", &bytes, after).unwrap(),
        &shown,
        &OperationContext {
            gate: &gate,
            policy: after,
        },
        &mut artifacts,
    );
    assert_eq!(
        settlement.commit,
        CommitEvidence::NotCommitted,
        "changed policy must invalidate preview"
    );
    assert!(matches!(
        settlement.failure,
        Some(OrchestrationError::PolicyMismatch)
    ));
    assert_eq!(artifacts.counter, 0, "refuse before artifact allocation");
    assert_eq!(
        owner.capture_stamp(&PublicationGate::new()).unwrap(),
        shown.catalogue
    );
    assert_eq!(package_count(&owner), 0);
}

#[test]
fn review_apply_refuses_standard_to_large() {
    refuses_changed_policy(ControlledPolicy::Standard, ControlledPolicy::Large);
}

#[test]
fn review_apply_refuses_large_to_standard() {
    refuses_changed_policy(ControlledPolicy::Large, ControlledPolicy::Standard);
}

#[test]
fn review_apply_refuses_unknown_binding_versions_before_work() {
    refuses_binding_version(0);
}

#[test]
fn review_apply_refuses_future_binding_version_before_work() {
    refuses_binding_version(crate::preview_binding::BINDING_VERSION + 1);
}

fn refuses_binding_version(version: u32) {
    let (_dir, mut owner) = migrated_owner();
    let bytes = valid_package();
    let mut shown = preview(&mut owner, &bytes, ControlledPolicy::Standard);
    shown.version = version;
    let gate = PublicationGate::new();
    let mut artifacts = TestArtifacts::new();
    let settlement = owner.apply_current_controlled(
        admit("review.knxprod", &bytes),
        &shown,
        &OperationContext {
            gate: &gate,
            policy: ControlledPolicy::Standard,
        },
        &mut artifacts,
    );
    assert_eq!(
        settlement.commit,
        CommitEvidence::NotCommitted,
        "unknown binding version must refuse"
    );
    assert!(matches!(
        settlement.failure,
        Some(OrchestrationError::Stale)
    ));
    assert_eq!(artifacts.counter, 0);
    assert_eq!(
        owner.capture_stamp(&PublicationGate::new()).unwrap(),
        shown.catalogue
    );
}

struct ReservationArtifacts {
    inner: TestArtifacts,
    contender: crate::Connection,
    blocked: Vec<bool>,
}

impl ReservationArtifacts {
    fn observe_reservation(&mut self) {
        let result = self.contender.execute_batch("BEGIN IMMEDIATE");
        self.blocked.push(
            matches!(result, Err(rusqlite::Error::SqliteFailure(error, _))
            if error.code == rusqlite::ErrorCode::DatabaseBusy),
        );
        if !self.contender.is_autocommit() {
            self.contender.execute_batch("ROLLBACK").unwrap();
        }
    }
}

impl CatalogueArtifacts for ReservationArtifacts {
    fn inventory(&mut self) -> &mut ArtifactInventory {
        self.inner.inventory()
    }
    fn fresh_rehearsal(&mut self, size: i64) -> Result<FreshDestination, OrchestrationError> {
        self.observe_reservation();
        self.inner.fresh_rehearsal(size)
    }

    fn fresh_recovery(&mut self, size: i64) -> Result<FreshDestination, OrchestrationError> {
        self.observe_reservation();
        self.inner.fresh_recovery(size)
    }
}

#[test]
fn reopened_preview_retains_writer_reservation_through_copy_allocation() {
    let (dir, mut owner) = migrated_owner();
    let before = owner.capture_stamp(&PublicationGate::new()).unwrap();
    let bytes = valid_package();
    let contender = crate::Connection::open(dir.path().join("products.sqlite")).unwrap();
    contender.busy_timeout(std::time::Duration::ZERO).unwrap();
    let mut artifacts = ReservationArtifacts {
        inner: TestArtifacts::new(),
        contender,
        blocked: Vec::new(),
    };
    let gate = PublicationGate::new();
    let result = owner
        .preview_current_controlled(
            admit("review.knxprod", &bytes),
            &OperationContext {
                gate: &gate,
                policy: ControlledPolicy::Standard,
            },
            &mut artifacts,
        )
        .unwrap();
    artifacts
        .contender
        .execute_batch("BEGIN IMMEDIATE; ROLLBACK")
        .unwrap();
    assert_eq!(
        artifacts.blocked,
        [true],
        "preview must not release W before copying S"
    );
    assert_eq!(result.binding.catalogue, before);
    assert_eq!(
        owner.capture_stamp(&PublicationGate::new()).unwrap(),
        before
    );
    assert_eq!(package_count(&owner), 0);
}

#[test]
fn review_apply_retains_writer_reservation_through_both_copies() {
    let (dir, mut owner) = migrated_owner();
    let bytes = valid_package();
    let shown = preview(&mut owner, &bytes, ControlledPolicy::Standard);
    let contender = crate::Connection::open(dir.path().join("products.sqlite")).unwrap();
    contender.busy_timeout(std::time::Duration::ZERO).unwrap();
    let mut artifacts = ReservationArtifacts {
        inner: TestArtifacts::new(),
        contender,
        blocked: Vec::new(),
    };
    let gate = PublicationGate::new();
    let settlement = owner.apply_current_controlled(
        admit("review.knxprod", &bytes),
        &shown,
        &OperationContext {
            gate: &gate,
            policy: ControlledPolicy::Standard,
        },
        &mut artifacts,
    );
    // Verify the contender is released only after apply has settled.
    artifacts
        .contender
        .execute_batch("BEGIN IMMEDIATE; ROLLBACK")
        .unwrap();
    assert_eq!(settlement.commit, CommitEvidence::Committed);
    assert_eq!(
        artifacts.blocked,
        [true, true],
        "W must remain reserved after stamp validation through both copies"
    );
    assert_eq!(package_count(&owner), 1);
}

#[test]
fn review_apply_refuses_an_external_commit_before_reservation() {
    let (dir, mut owner) = migrated_owner();
    let bytes = valid_package();
    let shown = preview(&mut owner, &bytes, ControlledPolicy::Standard);
    let external = crate::Connection::open(dir.path().join("products.sqlite")).unwrap();
    external
        .execute(
            "INSERT INTO manufacturer(id,name) VALUES('unrelated','external')",
            [],
        )
        .unwrap();
    let gate = PublicationGate::new();
    let mut artifacts = TestArtifacts::new();
    let settlement = owner.apply_current_controlled(
        admit("review.knxprod", &bytes),
        &shown,
        &OperationContext {
            gate: &gate,
            policy: ControlledPolicy::Standard,
        },
        &mut artifacts,
    );
    assert!(matches!(
        settlement.failure,
        Some(OrchestrationError::Stale)
    ));
    assert_eq!(settlement.commit, CommitEvidence::NotCommitted);
    assert_eq!(artifacts.counter, 0);
    assert_eq!(package_count(&owner), 0);
}

struct PreviewStopArtifacts<'a> {
    inner: FaultArtifacts,
    gate: &'a PublicationGate,
}

impl CatalogueArtifacts for PreviewStopArtifacts<'_> {
    fn inventory(&mut self) -> &mut ArtifactInventory {
        self.inner.inventory()
    }
    fn fresh_rehearsal(&mut self, size: i64) -> Result<FreshDestination, OrchestrationError> {
        assert_eq!(
            self.gate.try_cancel(),
            crate::operation::CancelOutcome::Accepted
        );
        self.inner.fresh_rehearsal(size)
    }
    fn fresh_recovery(&mut self, size: i64) -> Result<FreshDestination, OrchestrationError> {
        self.inner.fresh_recovery(size)
    }
}

#[test]
fn reopened_preview_stop_rolls_back_once_without_advancing_generation() {
    let (_dir, mut owner) = migrated_owner();
    let before = owner.capture_stamp(&PublicationGate::new()).unwrap();
    let bytes = valid_package();
    let gate = PublicationGate::new();
    let mut artifacts = PreviewStopArtifacts {
        inner: FaultArtifacts::new(&owner, false, false),
        gate: &gate,
    };
    artifacts.inner.attach();
    let result = owner.preview_current_controlled(
        admit("review.knxprod", &bytes),
        &OperationContext {
            gate: &gate,
            policy: ControlledPolicy::Standard,
        },
        &mut artifacts,
    );
    assert!(matches!(
        result,
        Err(OrchestrationError::Copy(
            crate::catalogue_copy::CopyError::Stop(_)
        ))
    ));
    assert_eq!(artifacts.inner.fault.transactions, ["BEGIN", "ROLLBACK"]);
    assert_eq!(artifacts.inner.fault.rollback_attempts, 1);
    assert!(!artifacts.inner.active());
    assert_eq!(
        owner.capture_stamp(&PublicationGate::new()).unwrap(),
        before
    );
    assert_eq!(package_count(&owner), 0);
}

#[test]
fn reopened_preview_refuses_replaced_writer_before_copy() {
    let (dir, mut owner) = migrated_owner();
    let path = dir.path().join("products.sqlite");
    let replacement = dir.path().join("identical.sqlite");
    std::fs::copy(&path, &replacement).unwrap();
    std::fs::rename(&path, dir.path().join("original.sqlite")).unwrap();
    std::fs::rename(&replacement, &path).unwrap();
    let bytes = valid_package();
    let gate = PublicationGate::new();
    let mut artifacts = TestArtifacts::new();
    let result = owner.preview_current_controlled(
        admit("review.knxprod", &bytes),
        &OperationContext {
            gate: &gate,
            policy: ControlledPolicy::Standard,
        },
        &mut artifacts,
    );
    assert!(
        matches!(result, Err(OrchestrationError::Stale)),
        "identical content cannot repair a replaced W identity"
    );
    assert_eq!(artifacts.counter, 0);
    assert!(owner.with_read(|_| Ok::<_, rusqlite::Error>(())).is_err());
}

#[test]
fn reopened_preview_preserves_stop_and_failed_rollback_independently() {
    let (_dir, mut owner) = migrated_owner();
    let bytes = valid_package();
    let gate = PublicationGate::new();
    let mut artifacts = PreviewStopArtifacts {
        inner: FaultArtifacts::new(&owner, false, true),
        gate: &gate,
    };
    artifacts.inner.attach();
    let result = owner.preview_current_controlled(
        admit("review.knxprod", &bytes),
        &OperationContext {
            gate: &gate,
            policy: ControlledPolicy::Standard,
        },
        &mut artifacts,
    );
    assert!(
        artifacts.inner.active(),
        "denied rollback must leave native W active"
    );
    assert_eq!(artifacts.inner.fault.transactions, ["BEGIN", "ROLLBACK"]);
    assert_eq!(artifacts.inner.fault.rollback_attempts, 1);
    assert!(owner.with_read(|_| Ok::<_, rusqlite::Error>(())).is_err());
    let failure = match result {
        Err(OrchestrationError::PreviewSettlement(failure)) => failure,
        _ => panic!("preview must retain primary Stop independently of rollback failure"),
    };
    assert!(matches!(
        failure.primary,
        Some(OrchestrationError::Copy(
            crate::catalogue_copy::CopyError::Stop(_)
        ))
    ));
    assert!(failure
        .cleanup
        .iter()
        .any(|error| matches!(error, PreviewCleanupError::WriterRollback(_))));
    assert!(failure
        .cleanup
        .iter()
        .any(|error| matches!(error, PreviewCleanupError::WriterNotAutocommit)));
}

struct NativeFault {
    deny_insert: bool,
    deny_rollback: bool,
    deny_commit: bool,
    rollback_attempts: usize,
    transactions: Vec<String>,
    contender: Option<crate::Connection>,
    live_probes: Vec<bool>,
}

unsafe extern "C" fn authorizer(
    context: *mut c_void,
    action: c_int,
    first: *const c_char,
    _second: *const c_char,
    _db: *const c_char,
    _trigger: *const c_char,
) -> c_int {
    // SAFETY: FaultArtifacts owns the boxed context until the hook is removed;
    // SQLite invokes this synchronously on this thread only.
    let fault = unsafe { &mut *context.cast::<NativeFault>() };
    let transaction = if action == rusqlite::ffi::SQLITE_TRANSACTION && !first.is_null() {
        Some(
            unsafe { CStr::from_ptr(first) }
                .to_string_lossy()
                .into_owned(),
        )
    } else {
        None
    };
    if let Some(name) = &transaction {
        fault.transactions.push(name.clone());
    }
    if action == rusqlite::ffi::SQLITE_INSERT || transaction.as_deref() == Some("COMMIT") {
        if let Some(conn) = &fault.contender {
            let attempt = conn.execute_batch("BEGIN IMMEDIATE");
            fault.live_probes.push(
                matches!(attempt, Err(rusqlite::Error::SqliteFailure(error, _))
                if error.code == rusqlite::ErrorCode::DatabaseBusy),
            );
            if !conn.is_autocommit() {
                let _ = conn.execute_batch("ROLLBACK");
            }
        }
    }
    if transaction.as_deref() == Some("COMMIT") && fault.deny_commit {
        return rusqlite::ffi::SQLITE_DENY;
    }
    if action == rusqlite::ffi::SQLITE_INSERT && fault.deny_insert {
        return rusqlite::ffi::SQLITE_DENY;
    }
    if action == rusqlite::ffi::SQLITE_TRANSACTION
        && !first.is_null()
        && unsafe { CStr::from_ptr(first) }.to_bytes() == b"ROLLBACK"
    {
        fault.rollback_attempts += 1;
        if fault.deny_rollback {
            return rusqlite::ffi::SQLITE_DENY;
        }
    }
    rusqlite::ffi::SQLITE_OK
}

struct FaultArtifacts {
    inner: TestArtifacts,
    writer: *mut rusqlite::ffi::sqlite3,
    fault: Box<NativeFault>,
    invalid_recovery: bool,
}

impl FaultArtifacts {
    fn new(owner: &ManagedCatalogue, deny_insert: bool, deny_rollback: bool) -> Self {
        let writer = owner
            .with_read(|conn| Ok::<_, rusqlite::Error>(unsafe { conn.handle() }))
            .unwrap()
            .unwrap();
        Self {
            inner: TestArtifacts::new(),
            writer,
            fault: Box::new(NativeFault {
                deny_insert,
                deny_rollback,
                deny_commit: false,
                rollback_attempts: 0,
                transactions: Vec::new(),
                contender: None,
                live_probes: Vec::new(),
            }),
            invalid_recovery: false,
        }
    }

    fn attach(&mut self) {
        let context = (&mut *self.fault as *mut NativeFault).cast();
        assert_eq!(
            unsafe {
                rusqlite::ffi::sqlite3_set_authorizer(self.writer, Some(authorizer), context)
            },
            rusqlite::ffi::SQLITE_OK
        );
    }

    fn active(&self) -> bool {
        unsafe { rusqlite::ffi::sqlite3_get_autocommit(self.writer) == 0 }
    }
}

impl Drop for FaultArtifacts {
    fn drop(&mut self) {
        // The owner outlives this fixture. Detach before the boxed callback
        // context is dropped; any unresolved transaction is closed by owner.
        assert_eq!(
            unsafe {
                rusqlite::ffi::sqlite3_set_authorizer(self.writer, None, std::ptr::null_mut())
            },
            rusqlite::ffi::SQLITE_OK
        );
    }
}

impl CatalogueArtifacts for FaultArtifacts {
    fn inventory(&mut self) -> &mut ArtifactInventory {
        self.inner.inventory()
    }
    fn fresh_rehearsal(&mut self, size: i64) -> Result<FreshDestination, OrchestrationError> {
        self.inner.fresh_rehearsal(size)
    }

    fn fresh_recovery(&mut self, size: i64) -> Result<FreshDestination, OrchestrationError> {
        self.attach();
        let conn = self.inner.fresh_recovery(size)?;
        if self.invalid_recovery {
            self.inventory()
                .test_writer(&conn)
                .execute_batch("CREATE TABLE invalid(id INTEGER)")?;
        }
        Ok(conn)
    }
}

struct InitializationFaultArtifacts {
    inner: FaultArtifacts,
}
impl CatalogueArtifacts for InitializationFaultArtifacts {
    fn inventory(&mut self) -> &mut ArtifactInventory {
        self.inner.inventory()
    }
    fn fresh_rehearsal(&mut self, size: i64) -> Result<FreshDestination, OrchestrationError> {
        self.inner.fresh_rehearsal(size)
    }
    fn fresh_recovery(&mut self, size: i64) -> Result<FreshDestination, OrchestrationError> {
        self.inner.attach();
        self.inventory().set_hook(
            DestinationRole::Recovery,
            Box::new(|stage, path, _| {
                if stage == crate::catalogue_copy::DestinationStage::Allocated {
                    std::fs::hard_link(path, path.with_extension("retained")).unwrap();
                }
            }),
        );
        self.inner.inner.fresh_recovery(size)
    }
}
fn initialization_refusal(deny_rollback: bool) {
    use crate::catalogue_copy::{CopyError, DestinationError};
    let (_dir, mut owner) = migrated_owner();
    let bytes = valid_package();
    let shown = preview(&mut owner, &bytes, ControlledPolicy::Standard);
    let mut artifacts = InitializationFaultArtifacts {
        inner: FaultArtifacts::new(&owner, false, deny_rollback),
    };
    let gate = PublicationGate::new();
    let result = owner.apply_current_controlled(
        admit("review.knxprod", &bytes),
        &shown,
        &OperationContext {
            gate: &gate,
            policy: ControlledPolicy::Standard,
        },
        &mut artifacts,
    );
    assert!(
        matches!(result.failure,Some(OrchestrationError::Copy(CopyError::Destination(ref e))) if matches!(e.primary,Some(DestinationError::Identity)))
    );
    assert_eq!(result.commit, CommitEvidence::NotCommitted);
    assert_eq!(
        result.rollback,
        if deny_rollback {
            RollbackDisposition::Unresolved
        } else {
            RollbackDisposition::Explicit
        }
    );
    assert_eq!(
        result.artifacts,
        if deny_rollback {
            ArtifactDisposition::Retain
        } else {
            ArtifactDisposition::Dispose
        },
        "allocated B counts even when identity/open initialization fails"
    );
    assert!(artifacts.inventory().allocated(DestinationRole::Recovery));
    assert!(
        artifacts
            .inventory()
            .retained_connection(DestinationRole::Recovery)
            .is_none(),
        "failure precedes native SQLite open"
    );
    assert_eq!(artifacts.inner.inner.counter, 2);
    let path = artifacts.inner.inner.dir.path().join("dest-1.sqlite");
    assert!(path.is_file());
    assert!(path.with_extension("retained").is_file());
    assert_eq!(
        std::os::unix::fs::MetadataExt::nlink(&std::fs::metadata(path).unwrap()),
        2,
        "uncertain B is never deleted"
    );
    assert_eq!(artifacts.inner.fault.rollback_attempts, 1);
    assert_eq!(artifacts.inner.active(), deny_rollback);
    assert_eq!(result.health == WriterHealth::Healthy, !deny_rollback);
    if deny_rollback {
        assert!(
            result
                .cleanup
                .iter()
                .any(|e| matches!(e, PreviewCleanupError::WriterRollback(_))),
            "native rollback error must survive separately from the initialization refusal"
        );
        assert!(result
            .cleanup
            .iter()
            .any(|e| matches!(e, PreviewCleanupError::WriterNotAutocommit)));
    } else {
        assert!(result.cleanup.is_empty());
    }
}
#[test]
fn reopened_allocated_b_initialization_failure_is_disposable_after_checked_rollback() {
    initialization_refusal(false);
}
#[test]
fn reopened_allocated_b_initialization_failure_is_retained_after_failed_rollback() {
    initialization_refusal(true);
}

fn failed_rollback(deny_insert: bool, mismatch: bool, invalid_recovery: bool) {
    let (_dir, mut owner) = migrated_owner();
    let bytes = valid_package();
    let mut shown = preview(&mut owner, &bytes, ControlledPolicy::Standard);
    if mismatch {
        shown.preview.sha256[0] ^= 1;
    }
    let mut artifacts = FaultArtifacts::new(&owner, deny_insert, true);
    artifacts.invalid_recovery = invalid_recovery;
    let gate = PublicationGate::new();
    let settlement = owner.apply_current_controlled(
        admit("review.knxprod", &bytes),
        &shown,
        &OperationContext {
            gate: &gate,
            policy: ControlledPolicy::Standard,
        },
        &mut artifacts,
    );
    assert!(settlement.failure.is_some());
    assert_eq!(
        settlement.artifacts,
        ArtifactDisposition::Retain,
        "unresolved rollback must retain recovery"
    );
    assert!(
        artifacts.active(),
        "native fault must actually leave W in a transaction"
    );
    assert_eq!(
        artifacts.fault.rollback_attempts, 1,
        "exactly one explicit rollback attempt"
    );
    assert_eq!(settlement.rollback, RollbackDisposition::Unresolved);
    assert_eq!(
        settlement.commit,
        CommitEvidence::NotCommitted,
        "no COMMIT was issued"
    );
    assert_eq!(
        settlement.health,
        WriterHealth::Quarantined(QuarantineReason::UncertainSettlement)
    );
    assert!(owner.with_read(|_| Ok::<_, rusqlite::Error>(())).is_err());
    if deny_insert {
        assert!(matches!(
            settlement.failure,
            Some(OrchestrationError::Install(_))
        ));
    }
    if mismatch {
        assert!(matches!(
            settlement.failure,
            Some(OrchestrationError::Stale)
        ));
    }
    if invalid_recovery {
        assert!(matches!(
            settlement.failure,
            Some(OrchestrationError::Copy(_))
        ));
    }
}

#[test]
fn review_failed_install_rollback_retains_recovery() {
    failed_rollback(true, false, false);
}

#[test]
fn review_failed_report_mismatch_rollback_retains_recovery() {
    failed_rollback(false, true, false);
}

#[test]
fn review_failed_copy_rollback_quarantines_and_retains_recovery() {
    failed_rollback(false, false, true);
}

#[test]
fn review_one_reservation_covers_stamp_live_dml_and_commit() {
    let (dir, mut owner) = migrated_owner();
    let bytes = valid_package();
    let shown = preview(&mut owner, &bytes, ControlledPolicy::Standard);
    let mut artifacts = FaultArtifacts::new(&owner, false, false);
    let contender = crate::Connection::open(dir.path().join("products.sqlite")).unwrap();
    contender.busy_timeout(std::time::Duration::ZERO).unwrap();
    artifacts.fault.contender = Some(contender);
    artifacts.attach();
    let gate = PublicationGate::new();
    let settlement = owner.apply_current_controlled(
        admit("review.knxprod", &bytes),
        &shown,
        &OperationContext {
            gate: &gate,
            policy: ControlledPolicy::Standard,
        },
        &mut artifacts,
    );
    assert_eq!(settlement.commit, CommitEvidence::Committed);
    assert_eq!(
        artifacts.fault.transactions,
        ["BEGIN", "COMMIT"],
        "no rollback/reacquire between stamp and live DML"
    );
    assert!(
        artifacts.fault.live_probes.len() > 1,
        "probe actual INSERTs and COMMIT"
    );
    assert!(artifacts.fault.live_probes.iter().all(|blocked| *blocked));
    artifacts
        .fault
        .contender
        .as_ref()
        .unwrap()
        .execute_batch("BEGIN IMMEDIATE; ROLLBACK")
        .unwrap();
}

#[test]
fn review_stale_refusal_with_failed_rollback_quarantines_without_copy() {
    let (_dir, mut owner) = migrated_owner();
    let bytes = valid_package();
    let mut shown = preview(&mut owner, &bytes, ControlledPolicy::Standard);
    shown.catalogue.generation += 1;
    let mut artifacts = FaultArtifacts::new(&owner, false, true);
    artifacts.attach();
    let gate = PublicationGate::new();
    let settlement = owner.apply_current_controlled(
        admit("review.knxprod", &bytes),
        &shown,
        &OperationContext {
            gate: &gate,
            policy: ControlledPolicy::Standard,
        },
        &mut artifacts,
    );
    assert!(matches!(
        settlement.failure,
        Some(OrchestrationError::Stale)
    ));
    assert_eq!(settlement.commit, CommitEvidence::NotCommitted);
    assert_eq!(settlement.rollback, RollbackDisposition::Unresolved);
    assert_eq!(
        settlement.health,
        WriterHealth::Quarantined(QuarantineReason::UncertainSettlement)
    );
    assert_eq!(
        settlement.artifacts,
        ArtifactDisposition::None,
        "no recovery was allocated"
    );
    assert_eq!(artifacts.inner.counter, 0);
    assert!(artifacts.active());
    assert_eq!(artifacts.fault.rollback_attempts, 1);
}

#[test]
fn review_apply_refuses_each_retained_policy_identity_field_before_sql() {
    for field in 0..3 {
        let (_dir, mut owner) = migrated_owner();
        let bytes = valid_package();
        let mut shown = preview(&mut owner, &bytes, ControlledPolicy::Standard);
        match field {
            0 => shown.policy.version += 1,
            1 => shown.policy.max_member_size += 1,
            _ => shown.policy.max_expanded_size += 1,
        }
        let mut artifacts = FaultArtifacts::new(&owner, false, false);
        artifacts.attach();
        let gate = PublicationGate::new();
        let settlement = owner.apply_current_controlled(
            admit("review.knxprod", &bytes),
            &shown,
            &OperationContext {
                gate: &gate,
                policy: ControlledPolicy::Standard,
            },
            &mut artifacts,
        );
        assert!(matches!(
            settlement.failure,
            Some(OrchestrationError::PolicyMismatch)
        ));
        assert_eq!(settlement.commit, CommitEvidence::NotCommitted);
        assert_eq!(artifacts.inner.counter, 0);
        assert!(
            artifacts.fault.transactions.is_empty(),
            "policy refusal must precede W reservation"
        );
        assert_eq!(
            owner.capture_stamp(&PublicationGate::new()).unwrap(),
            shown.catalogue
        );
    }
}

#[test]
fn review_nonbusy_commit_error_retains_recovery_after_explicit_rollback() {
    commit_failure(false, false);
}

#[test]
fn review_nonbusy_commit_and_rollback_errors_retain_recovery() {
    commit_failure(false, true);
}

#[test]
fn review_busy_commit_with_explicit_rollback_is_not_committed() {
    commit_failure(true, false);
}

fn commit_failure(busy: bool, deny_rollback: bool) {
    let (dir, mut owner) = migrated_owner();
    let bytes = valid_package();
    let shown = preview(&mut owner, &bytes, ControlledPolicy::Standard);
    owner
        .with_read(|conn| conn.busy_timeout(std::time::Duration::ZERO))
        .unwrap()
        .unwrap();
    let mut artifacts = FaultArtifacts::new(&owner, false, deny_rollback);
    artifacts.fault.deny_commit = !busy;
    let reader = crate::Connection::open(dir.path().join("products.sqlite")).unwrap();
    if busy {
        reader
            .execute_batch("BEGIN; SELECT count(*) FROM sqlite_schema")
            .unwrap();
    }
    let gate = PublicationGate::new();
    let settlement = owner.apply_current_controlled(
        admit("review.knxprod", &bytes),
        &shown,
        &OperationContext {
            gate: &gate,
            policy: ControlledPolicy::Standard,
        },
        &mut artifacts,
    );
    if busy {
        reader.execute_batch("ROLLBACK").unwrap();
    }
    assert!(matches!(
        settlement.failure,
        Some(OrchestrationError::Sqlite(_))
    ));
    assert_eq!(
        settlement.rollback,
        if deny_rollback {
            RollbackDisposition::Unresolved
        } else {
            RollbackDisposition::Explicit
        }
    );
    assert_eq!(
        settlement.commit,
        if busy {
            CommitEvidence::NotCommitted
        } else {
            CommitEvidence::Unknown
        }
    );
    assert_eq!(
        settlement.artifacts,
        if busy {
            ArtifactDisposition::Dispose
        } else {
            ArtifactDisposition::Retain
        }
    );
    assert_eq!(
        settlement.health,
        if busy {
            WriterHealth::Healthy
        } else {
            WriterHealth::Quarantined(QuarantineReason::UncertainSettlement)
        }
    );
    assert_eq!(artifacts.active(), deny_rollback);
    assert_eq!(artifacts.fault.transactions, ["COMMIT", "ROLLBACK"]);
    assert_eq!(
        owner.retained_apply_witness().unwrap().trace,
        LiveCommitTrace::LiveCommitInvoked,
        "failed native COMMIT cannot become positive through rollback/autocommit"
    );
}

#[test]
fn review_successful_install_refusal_rollback_disposes_recovery() {
    let (_dir, mut owner) = migrated_owner();
    let bytes = valid_package();
    let shown = preview(&mut owner, &bytes, ControlledPolicy::Standard);
    let mut artifacts = FaultArtifacts::new(&owner, true, false);
    let gate = PublicationGate::new();
    let settlement = owner.apply_current_controlled(
        admit("review.knxprod", &bytes),
        &shown,
        &OperationContext {
            gate: &gate,
            policy: ControlledPolicy::Standard,
        },
        &mut artifacts,
    );
    assert!(matches!(
        settlement.failure,
        Some(OrchestrationError::Install(_))
    ));
    assert_eq!(settlement.commit, CommitEvidence::NotCommitted);
    assert_eq!(settlement.rollback, RollbackDisposition::Explicit);
    assert_eq!(settlement.health, WriterHealth::Healthy);
    assert_eq!(settlement.artifacts, ArtifactDisposition::Dispose);
    assert!(!artifacts.active());
    assert_eq!(package_count(&owner), 0);
}
