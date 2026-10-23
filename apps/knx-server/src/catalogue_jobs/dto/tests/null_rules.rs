//! Required explicit-null fields in status and its nested wire objects.
use super::review_regressions::assemble;
use super::*;

fn with_nullable_fields() -> Status {
    let mut s = valid_failed();
    s.state = JobState::Running;
    s.phase = Some(Phase::Settling);
    s.cleanup.disposition = CleanupDisposition::Pending;
    s.lease.kind = LeaseKind::Working;
    s.progress = Some(Progress {
        stage: ProgressStage::Cleanup,
        pass: u64(1),
        unit: ProgressUnit::Records,
        completed: u64(0),
        total: None,
    });
    assert!(s.validate().is_ok());
    s
}

macro_rules! missing_is_not_null {
    ($name:ident, $parent:literal, $field:literal) => {
        #[test]
        fn $name() {
            let mut wire = serde_json::to_value(with_nullable_fields()).unwrap();
            assert!(wire
                .pointer_mut($parent)
                .unwrap()
                .as_object_mut()
                .unwrap()
                .remove($field)
                .is_some());
            assert_eq!(
                from_str::<Status>(&wire.to_string()),
                Err(WireError::Schema),
                "missing nullable field must not be defaulted to null"
            );
        }
    };
}
missing_is_not_null!(missing_phase_refuses, "", "phase");
missing_is_not_null!(missing_stop_cause_refuses, "", "stopCause");
missing_is_not_null!(missing_progress_refuses, "", "progress");
missing_is_not_null!(missing_preview_refuses, "", "preview");
missing_is_not_null!(missing_result_refuses, "", "result");
missing_is_not_null!(missing_failure_refuses, "", "failure");
missing_is_not_null!(missing_source_digest_refuses, "/source", "sha256");
missing_is_not_null!(missing_apply_command_refuses, "/apply", "acceptedCommandId");
missing_is_not_null!(missing_cleanup_failure_refuses, "/cleanup", "failureKind");
missing_is_not_null!(missing_backup_name_refuses, "/backup", "name");
missing_is_not_null!(missing_backup_identity_refuses, "/backup", "identity");
missing_is_not_null!(missing_progress_total_refuses, "/progress", "total");
missing_is_not_null!(missing_failure_context_refuses, "/failure", "context");

#[test]
fn explicit_null_fields_still_roundtrip() {
    let s = valid_failed();
    let wire = to_string(&s).unwrap();
    assert_eq!(from_str::<Status>(&wire).unwrap(), s);
    assert!(assemble(s).is_ok());
}

#[test]
fn assembly_refuses_nonnull_internal_failure_context() {
    let mut s = valid_failed();
    assert!(assemble(s.clone()).is_ok());
    s.failure.as_mut().unwrap().context = Some(());
    assert_eq!(s.validate(), Err(WireError::Const));
    assert!(assemble(s).is_err());
}
