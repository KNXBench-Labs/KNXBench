//! Actual W no-DML boundary and closed-backup publication, not a fake success.
use super::*;
use crate::managed_apply::{
    AdmittedSource, ApplyReceipt, ApplyWorkspace, ControlledFailureKind, PublishedRecovery,
    RecoveryPublicationError, RecoveryPublisher, ValidatedRecovery,
};
struct RefuseResult;
impl RecoveryPublisher for RefuseResult {
    fn allocated(
        &mut self,
        _: &std::path::Path,
        _: &std::fs::File,
    ) -> Result<(), RecoveryPublicationError> {
        Ok(())
    }
    fn publish(
        &mut self,
        mut copy: ValidatedRecovery<'_>,
    ) -> Result<PublishedRecovery, RecoveryPublicationError> {
        let parent = copy.path().parent().unwrap().to_path_buf();
        copy.publish_to(&parent, "published-before-refusal.sqlite")
    }
    fn admit_result(
        &mut self,
        _: &crate::ProductFileInstall,
        _: u64,
    ) -> Result<(), RecoveryPublicationError> {
        Err(RecoveryPublicationError::Refused)
    }
}
#[test]
fn complete_result_refusal_rolls_back_before_the_commit_claim() {
    let (dir, mut owner) = migrated_owner();
    let bytes = valid_package();
    let source =
        AdmittedSource::admit("publication.knxprod", &bytes, ControlledPolicy::Standard).unwrap();
    let gate = PublicationGate::new();
    let mut preview = ApplyWorkspace::create(dir.path().join("preview")).unwrap();
    let shown = owner
        .prepare_controlled(source, &gate, &mut preview)
        .unwrap();
    let baseline = owner
        .with_read(|c| Ok::<_, ()>(c.total_changes()))
        .unwrap()
        .unwrap();
    let mut workspace = ApplyWorkspace::create(dir.path().join("apply")).unwrap();
    let mut receipt = ApplyReceipt::default();
    let result = owner
        .execute_controlled(
            source,
            &shown,
            &gate,
            &mut workspace,
            &mut receipt,
            &mut RefuseResult,
        )
        .unwrap();
    assert_eq!(
        result.commit(),
        CommitEvidence::NotCommitted,
        "a refused complete result artifact must prevent the live COMMIT"
    );
    assert_eq!(result.rollback(), RollbackDisposition::Explicit);
    assert_eq!(package_count(&owner), 0);
    assert_eq!(gate.state(), crate::operation::PublicationState::Open);
    assert!(
        owner
            .with_read(|c| Ok::<_, ()>(c.total_changes()))
            .unwrap()
            .unwrap()
            > baseline
    );
    assert_eq!(
        result.failure_kind(),
        Some(ControlledFailureKind::ResultAdmission)
    );
}

struct Refuse {
    calls: usize,
}
impl RecoveryPublisher for Refuse {
    fn admit_result(
        &mut self,
        _: &crate::ProductFileInstall,
        _: u64,
    ) -> Result<(), RecoveryPublicationError> {
        Ok(())
    }
    fn allocated(
        &mut self,
        _: &std::path::Path,
        _: &std::fs::File,
    ) -> Result<(), RecoveryPublicationError> {
        Ok(())
    }
    fn publish(
        &mut self,
        _: ValidatedRecovery<'_>,
    ) -> Result<PublishedRecovery, RecoveryPublicationError> {
        self.calls += 1;
        Err(RecoveryPublicationError::Refused)
    }
}
#[test]
fn durable_backup_refusal_precedes_any_native_live_dml() {
    let (dir, mut owner) = migrated_owner();
    let bytes = valid_package();
    let source =
        AdmittedSource::admit("publication.knxprod", &bytes, ControlledPolicy::Standard).unwrap();
    let gate = PublicationGate::new();
    let mut preview = ApplyWorkspace::create(dir.path().join("preview")).unwrap();
    let shown = owner
        .prepare_controlled(source, &gate, &mut preview)
        .unwrap();
    let baseline = owner
        .with_read(|c| Ok::<_, ()>(c.total_changes()))
        .unwrap()
        .unwrap();
    let mut workspace = ApplyWorkspace::create(dir.path().join("apply")).unwrap();
    let mut receipt = ApplyReceipt::default();
    let mut publisher = Refuse { calls: 0 };
    let result = owner
        .execute_controlled(
            source,
            &shown,
            &gate,
            &mut workspace,
            &mut receipt,
            &mut publisher,
        )
        .unwrap();
    let observed = owner
        .with_read(|c| Ok::<_, ()>(c.total_changes()))
        .unwrap()
        .unwrap();
    assert_eq!(
        observed, baseline,
        "a refused durable backup must prevent the FIRST native W DML, not merely roll it back"
    );
    assert_eq!(publisher.calls, 1);
    assert_eq!(result.commit(), CommitEvidence::NotCommitted);
    assert_eq!(result.rollback(), RollbackDisposition::Explicit);
    assert_eq!(
        result.failure_kind(),
        Some(ControlledFailureKind::RecoveryPublication)
    );
    assert_eq!(package_count(&owner), 0);
}

#[test]
fn real_published_backup_is_closed_complete_and_before_the_live_install() {
    let (dir, mut owner) = migrated_owner();
    let bytes = valid_package();
    let source =
        AdmittedSource::admit("publication.knxprod", &bytes, ControlledPolicy::Standard).unwrap();
    let gate = PublicationGate::new();
    let mut preview = ApplyWorkspace::create(dir.path().join("preview")).unwrap();
    let shown = owner
        .prepare_controlled(source, &gate, &mut preview)
        .unwrap();
    let mut workspace = ApplyWorkspace::create(dir.path().join("apply")).unwrap();
    let mut receipt = ApplyReceipt::default();
    let result = owner
        .execute_controlled(
            source,
            &shown,
            &gate,
            &mut workspace,
            &mut receipt,
            &mut crate::managed_apply::publication::NativeTestPublisher,
        )
        .unwrap();
    assert_eq!(result.commit(), CommitEvidence::Committed);
    assert_eq!(package_count(&owner), 1);
    let backup = crate::Connection::open_with_flags(
        dir.path().join("apply/published-recovery.sqlite"),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    assert_eq!(
        backup
            .query_row("SELECT count(*) FROM package", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        0
    );
    backup.close().unwrap();
    assert!(workspace.has_recovery());
}
