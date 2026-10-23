//! Nested status admission regressions at the authoritative assembler.
use super::*;
use crate::catalogue_jobs::status::{assemble_status, StatusError, StatusSnapshot};

pub(super) fn assemble(s: Status) -> Result<Status, StatusError> {
    let revision = s.revision;
    assemble_status(
        StatusSnapshot {
            handle: s.handle,
            source: s.source,
            profile: s.profile,
            state: s.state,
            phase: s.phase,
            gate: s.gate,
            stop_cause: s.stop_cause,
            progress: s.progress,
            lease: s.lease,
            apply: s.apply,
            preview: s.preview,
            result: s.result,
            failure: s.failure,
            commit: s.commit,
            cleanup: s.cleanup,
            backup: s.backup,
        },
        revision,
    )
}

#[test]
fn review_status_assembly_refuses_invalid_nested_preview() {
    assert!(assemble(valid_succeeded()).is_ok());
    let mut s = valid_succeeded();
    s.preview.as_mut().unwrap().growth_cliff_possible = false;
    assert!(s.preview.as_ref().unwrap().validate().is_err());
    assert!(
        assemble(s).is_err(),
        "status assembler must apply the preview's own validation"
    );
}

#[test]
fn review_status_assembly_refuses_wrong_result_artifact_kind() {
    assert!(assemble(valid_succeeded()).is_ok());
    let mut s = valid_succeeded();
    s.result.as_mut().unwrap().artifact.kind = ArtifactKind::Preview;
    assert!(s.result.as_ref().unwrap().validate().is_err());
    assert!(
        assemble(s).is_err(),
        "status assembler must reject a preview artifact presented as a result"
    );
}
