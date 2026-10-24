//! Real facade catches retain independent D cleanup in the workspace owner.

use super::*;
use crate::catalogue_copy::{DestinationCleanup, DestinationFailure};
use crate::managed_apply::{
    AdmittedSource, ApplyReceipt, ApplyWorkspace, ControlledFailureKind, PreparedPreview,
};

fn prepare(owner: &mut ManagedCatalogue, bytes: &[u8]) -> PreparedPreview {
    let dir = tempfile::tempdir().unwrap();
    let mut workspace = ApplyWorkspace::create(dir.path().join("preview")).unwrap();
    owner
        .prepare_controlled_owned(
            source(bytes),
            &PublicationGate::new(),
            &mut workspace,
            &mut crate::managed_apply::publication::NativeTestPublisher,
        )
        .unwrap()
}
fn source(bytes: &[u8]) -> AdmittedSource<'_> {
    AdmittedSource::admit("second.knxprod", bytes, ControlledPolicy::Standard).unwrap()
}
fn workspace(dir: &std::path::Path, probe: &mut Probe) -> ApplyWorkspace {
    let mut workspace = ApplyWorkspace::create(dir.join("public-attempt")).unwrap();
    // Move only the existing real native observation hook, not a fabricated
    // settlement, into the same workspace used by the mounted facade.
    std::mem::swap(workspace.inventory(), &mut probe.inner.inventory);
    workspace
}
fn retained(workspace: &mut ApplyWorkspace) -> Option<Arc<DestinationFailure>> {
    workspace.inventory().rehearsal_cleanup_failure().cloned()
}
fn assert_facts(failure: &DestinationFailure, cause: &str) {
    assert!(failure.primary.is_none());
    match (cause, &failure.cleanup[..]) {
        ("busy", [DestinationCleanup::RehearsalClose(rusqlite::Error::SqliteFailure(e, _))]) => {
            assert_eq!(e.code, rusqlite::ErrorCode::DatabaseBusy);
            assert_eq!(e.extended_code, rusqlite::ffi::SQLITE_BUSY);
        }
        (
            "rollback",
            [DestinationCleanup::RehearsalRollback(rusqlite::Error::SqliteFailure(e, _)), DestinationCleanup::RehearsalNotAutocommit],
        ) => {
            assert_eq!(e.code, rusqlite::ErrorCode::AuthorizationForStatementDenied);
            assert_eq!(e.extended_code, rusqlite::ffi::SQLITE_AUTH);
        }
        _ => panic!("expected exact independent native D cleanup facts"),
    }
}
fn assert_owner_after_drop(
    workspace: &mut ApplyWorkspace,
    facts: Option<Arc<DestinationFailure>>,
    cause: &str,
) {
    assert_eq!(workspace.native_handles_closed(), cause != "busy");
    assert_eq!(workspace.cleanup_failed(), cause != "clean");
    if let Some(facts) = facts {
        let still_owned = retained(workspace).expect("workspace retains typed facts");
        assert!(Arc::ptr_eq(&facts, &still_owned));
        assert_facts(&still_owned, cause);
        assert!(workspace.release_closed_ownership().is_err());
        assert!(Arc::ptr_eq(&facts, &retained(workspace).unwrap()));
    } else {
        assert!(retained(workspace).is_none());
    }
    let connection = workspace
        .inventory()
        .retained_connection(DestinationRole::Rehearsal);
    if cause == "busy" {
        assert!(connection.unwrap().is_autocommit());
    } else {
        assert!(connection.is_none());
    }
}

fn assert_closed_rows(dir: &std::path::Path) {
    let d = crate::Connection::open_with_flags(
        dir.join("public-attempt/rehearsal.sqlite"),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    assert!(d.is_autocommit());
    for sql in [
        "SELECT count(*) FROM package",
        "SELECT count(*) FROM manufacturer WHERE id='unwind'",
    ] {
        assert_eq!(d.query_row(sql, [], |r| r.get::<_, i64>(0)).unwrap(), 0);
    }
    d.close().unwrap();
}

#[test]
fn public_apply_d_unwind_retains_cleanup_after_projection_drop() {
    for cause in ["clean", "busy", "rollback"] {
        let (dir, mut owner) = migrated_owner();
        let bytes = valid_package();
        let shown = prepare(&mut owner, &bytes);
        let gate = Arc::new(PublicationGate::new());
        let mut probe = Probe::new(
            &owner,
            &dir.path().join("products.sqlite"),
            gate.clone(),
            false,
        );
        probe.panic_install.set(true);
        probe.busy_close.set(cause == "busy");
        probe.d.deny_rollback.set(cause == "rollback");
        let mut workspace = workspace(dir.path(), &mut probe);
        let mut receipt = ApplyReceipt::default();
        let execution = owner
            .execute_controlled(
                source(&bytes),
                &shown,
                &*gate,
                &mut workspace,
                &mut receipt,
                &mut crate::managed_apply::publication::NativeTestPublisher,
            )
            .unwrap();
        // This is the real outcome.ok() catch, not a private panic downcast.
        assert!(execution.panicked());
        assert_eq!(
            execution.failure_kind(),
            Some(ControlledFailureKind::Panicked)
        );
        assert!(execution.cleanup_failed());
        assert!(matches!(execution.health(), WriterHealth::Quarantined(_)));
        assert_eq!(execution.commit(), CommitEvidence::Unknown);
        let facts = retained(&mut workspace);
        assert_eq!(facts.is_some(), cause != "clean");
        drop(execution);
        assert_owner_after_drop(&mut workspace, facts, cause);
        if cause != "busy" {
            assert_closed_rows(dir.path());
        }
        assert_eq!(probe.d.installer_entries.get(), 1);
        assert_eq!(&*probe.d.transactions.borrow(), &["BEGIN", "ROLLBACK"]);
        assert_eq!(probe.closes.get(), 1);
        probe.assert_no_w_dml();
        assert_eq!(gate.state(), PublicationState::Open);
        assert_eq!(
            receipt.witness().unwrap().trace(),
            LiveCommitTrace::NoLiveCommitInvocation
        );
        // Probe owns/finalizes any raw Busy statement before workspace Drop.
        drop(probe);
    }
}

#[test]
fn public_preview_d_unwind_retains_cleanup_after_outer_catch() {
    for cause in ["clean", "busy", "rollback"] {
        let (dir, mut owner) = migrated_owner();
        let bytes = valid_package();
        let gate = Arc::new(PublicationGate::new());
        let mut probe = Probe::new(
            &owner,
            &dir.path().join("products.sqlite"),
            gate.clone(),
            false,
        );
        probe.panic_install.set(true);
        probe.busy_close.set(cause == "busy");
        probe.d.deny_rollback.set(cause == "rollback");
        let mut workspace = workspace(dir.path(), &mut probe);
        // Same ownership shape as Session::preview: Native.workspace is outside
        // its outer catch. Discard the opaque panic, just as that catch does.
        let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            owner.prepare_controlled_owned(
                source(&bytes),
                &gate,
                &mut workspace,
                &mut crate::managed_apply::publication::NativeTestPublisher,
            )
        }));
        assert!(outcome.is_err());
        drop(outcome);
        let facts = retained(&mut workspace);
        assert_eq!(facts.is_some(), cause != "clean");
        assert_owner_after_drop(&mut workspace, facts, cause);
        if cause != "busy" {
            assert_closed_rows(dir.path());
        }
        assert_eq!(probe.d.installer_entries.get(), 1);
        assert_eq!(&*probe.d.transactions.borrow(), &["BEGIN", "ROLLBACK"]);
        assert_eq!(probe.closes.get(), 1);
        probe.assert_no_w_dml();
        assert_eq!(gate.state(), PublicationState::Open);
        drop(probe);
    }
}

#[test]
fn public_apply_closed_d_rollback_failure_preserves_cancel_and_refusal() {
    for cancelled in [false, true] {
        let (dir, mut owner) = migrated_owner();
        let bytes = valid_package();
        let shown = prepare(&mut owner, &bytes);
        let gate = Arc::new(PublicationGate::new());
        let mut probe = Probe::new(
            &owner,
            &dir.path().join("products.sqlite"),
            gate.clone(),
            !cancelled,
        );
        if cancelled {
            *probe.d.cancel_gate.borrow_mut() = Some(gate.clone());
        }
        probe.d.deny_rollback.set(true);
        // Deliberately no outstanding statement: failed ROLLBACK is observed
        // independently of successful native close and W's clean settlement.
        let mut workspace = workspace(dir.path(), &mut probe);
        let mut receipt = ApplyReceipt::default();
        let execution = owner
            .execute_controlled(
                source(&bytes),
                &shown,
                &*gate,
                &mut workspace,
                &mut receipt,
                &mut crate::managed_apply::publication::NativeTestPublisher,
            )
            .unwrap();
        assert!(!execution.panicked());
        assert!(execution.failed());
        assert_eq!(
            execution.failure_kind(),
            Some(if cancelled {
                ControlledFailureKind::Stopped
            } else {
                ControlledFailureKind::Install
            })
        );
        assert!(
            execution.cleanup_failed(),
            "cannot project cleanupComplete or clean Cancelled"
        );
        assert!(execution.native_settled());
        assert_eq!(execution.health(), WriterHealth::Healthy);
        assert_eq!(execution.commit(), CommitEvidence::NotCommitted);
        assert_eq!(execution.rollback(), RollbackDisposition::Explicit);
        let Some(OrchestrationError::PreviewSettlement(failure)) = execution.failure_for_test()
        else {
            panic!("preserve original work failure separately from D cleanup")
        };
        if cancelled {
            assert!(matches!(
                failure.primary,
                Some(OrchestrationError::Install(crate::PackageError::Cancelled(
                    crate::operation::Stop::CancelAccepted
                )))
            ));
            assert_eq!(probe.d.package_inserts.get(), 1);
            assert_eq!(
                probe.d.cancellation.get(),
                Some(crate::operation::CancelOutcome::Accepted)
            );
        } else {
            assert!(
                matches!(&failure.primary, Some(OrchestrationError::Install(crate::PackageError::Database(crate::ProductDbError::Sqlite(rusqlite::Error::SqliteFailure(e, _))))) if e.code == rusqlite::ErrorCode::AuthorizationForStatementDenied)
            );
            assert_eq!(probe.d.denials.get(), 1);
        }
        let [PreviewCleanupError::RehearsalClose(crate::catalogue_copy::CopyError::Destination(
            nested,
        ))] = &failure.cleanup[..]
        else {
            panic!("preserve exact nested cleanup error")
        };
        let facts = retained(&mut workspace).unwrap();
        assert!(Arc::ptr_eq(nested, &facts));
        assert_facts(&facts, "rollback");
        drop(execution);
        assert_owner_after_drop(&mut workspace, Some(facts), "rollback");
        assert!(probe.statement.get().is_null(), "no Busy-close proxy");
        assert_eq!(probe.closes.get(), 1);
        probe.assert_no_w_dml();
        assert_eq!(&*probe.w.transactions.borrow(), &["BEGIN", "ROLLBACK"]);
        assert_eq!(
            gate.state(),
            if cancelled {
                PublicationState::CancelAccepted
            } else {
                PublicationState::Open
            }
        );
        assert_eq!(
            receipt.witness().unwrap().trace(),
            LiveCommitTrace::NoLiveCommitInvocation
        );
        assert_closed_rows(dir.path());
        assert_eq!(package_count(&owner), 0);
    }
}
