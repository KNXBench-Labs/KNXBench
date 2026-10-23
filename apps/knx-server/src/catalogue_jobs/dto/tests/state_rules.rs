//! Phase and terminal cross-field checks through validation and assembly.
use super::review_regressions::assemble;
use super::*;

const PHASES: [Phase; 9] = [
    Phase::Receiving,
    Phase::Previewing,
    Phase::Ready,
    Phase::WaitingForWriter,
    Phase::Validating,
    Phase::BackingUp,
    Phase::Installing,
    Phase::Committing,
    Phase::Settling,
];

fn running(phase: Phase) -> Status {
    let mut s = base_status();
    s.phase = Some(phase);
    if phase != Phase::Receiving {
        s.source = std_source();
    }
    s.lease.kind = match phase {
        Phase::Receiving => LeaseKind::Receiving,
        Phase::Ready => LeaseKind::Ready,
        _ => LeaseKind::Working,
    };
    s.apply.state = match phase {
        Phase::Receiving | Phase::Previewing => ApplyState::Unavailable,
        Phase::Ready => ApplyState::Available,
        Phase::Settling => ApplyState::Spent,
        _ => ApplyState::Accepted,
    };
    if s.apply.state == ApplyState::Accepted {
        s.apply.accepted_command_id = Some(id(0xaa));
    }
    if !matches!(
        phase,
        Phase::Receiving | Phase::Previewing | Phase::Settling
    ) {
        s.preview = Some(valid_preview());
    }
    if matches!(phase, Phase::Installing | Phase::Committing) {
        s.backup = valid_succeeded().backup;
        s.backup.state = BackupState::Published;
    }
    if phase == Phase::Committing {
        s.gate = Gate::CommitClaimed;
    }
    s
}

fn refused(s: Status, reason: &str) {
    assert!(
        s.validate().is_err(),
        "validator admitted {reason}: {:?}",
        s.phase
    );
    assert_eq!(
        assemble(s),
        Err(crate::catalogue_jobs::status::StatusError::InvalidState),
        "assembler admitted {reason}"
    );
}

#[test]
fn audit_valid_phase_shapes_are_admitted() {
    for phase in PHASES {
        let s = running(phase);
        assert!(s.validate().is_ok(), "phase {phase:?}");
        assert!(assemble(s).is_ok(), "assembly phase {phase:?}");
    }
}

#[test]
fn audit_running_lease_matches_each_phase() {
    for phase in PHASES {
        let mut s = running(phase);
        s.lease.kind = LeaseKind::Retention;
        refused(s, "terminal lease in running phase");
    }
}

#[test]
fn audit_phase_requires_exact_source_sealing() {
    for phase in PHASES.into_iter().filter(|p| *p != Phase::Settling) {
        let mut s = running(phase);
        s.source.sha256 = if phase == Phase::Receiving {
            Some(sha(0x11))
        } else {
            None
        };
        refused(s, "wrong sealed-source state");
    }
}

#[test]
fn audit_phase_requires_exact_preview_nullability() {
    for phase in PHASES.into_iter().filter(|p| *p != Phase::Settling) {
        let mut s = running(phase);
        s.preview = if s.preview.is_some() {
            None
        } else {
            Some(valid_preview())
        };
        refused(s, "wrong preview presence");
    }
}

#[test]
fn audit_running_result_requires_settling_and_committed() {
    for phase in PHASES {
        let mut s = running(phase);
        s.result = Some(valid_result());
        refused(s, "result before witnessed commit");
    }
}

#[test]
fn audit_phase_requires_exact_apply_state() {
    for phase in PHASES {
        let mut s = running(phase);
        s.apply = Apply {
            state: ApplyState::Available,
            accepted_command_id: None,
        };
        if phase == Phase::Ready {
            s.apply.state = ApplyState::Unavailable;
        }
        refused(s, "wrong apply authority");
    }
}

#[test]
fn audit_phase_requires_exact_gate() {
    for phase in PHASES.into_iter().filter(|p| *p != Phase::Settling) {
        let mut s = running(phase);
        s.gate = if phase == Phase::Committing {
            Gate::Open
        } else {
            Gate::CommitClaimed
        };
        refused(s, "wrong gate");
    }
}

#[test]
fn audit_phase_requires_pending_commit_until_settling() {
    for phase in PHASES.into_iter().filter(|p| *p != Phase::Settling) {
        let mut s = running(phase);
        s.commit = Commit {
            outcome: CommitOutcome::Committed,
            evidence: CommitEvidence::CommitReturnedOk,
        };
        refused(s, "nonpending commit before settling");
    }
}

#[test]
fn audit_phase_requires_allowed_backup_state() {
    for phase in PHASES.into_iter().filter(|p| *p != Phase::Settling) {
        let mut s = running(phase);
        s.backup = valid_succeeded().backup; // Retained only after settlement starts.
        refused(s, "retained backup before settling");
    }
}

#[test]
fn audit_settling_may_retain_failure_and_committed_result() {
    let mut s = running(Phase::Settling);
    s.failure = Some(Failure {
        kind: FailureKind::WorkerFailed,
        context: None,
    });
    s.gate = Gate::CommitClaimed;
    s.commit = Commit {
        outcome: CommitOutcome::Committed,
        evidence: CommitEvidence::CommitReturnedOk,
    };
    s.preview = Some(valid_preview());
    s.result = Some(valid_result());
    s.apply.accepted_command_id = Some(id(0xaa));
    assert!(
        s.validate().is_ok(),
        "settling must preserve witnessed outcome and failure"
    );
    assert!(assemble(s).is_ok());
}

fn committed_settling() -> Status {
    let mut s = valid_succeeded();
    s.state = JobState::Running;
    s.phase = Some(Phase::Settling);
    s.lease.kind = LeaseKind::Working;
    s.cleanup.disposition = CleanupDisposition::Pending;
    s
}

fn committed_failed() -> Status {
    let mut s = valid_succeeded();
    s.state = JobState::Failed;
    s.failure = Some(Failure {
        kind: FailureKind::WorkerFailed,
        context: None,
    });
    s
}

#[test]
fn committed_settling_and_failed_canonical_positives() {
    let mut settling_failure = committed_settling();
    settling_failure.failure = committed_failed().failure;
    for s in [committed_settling(), settling_failure, committed_failed()] {
        assert_eq!(s.validate(), Ok(()));
        assert!(assemble(s.clone()).is_ok());
        println!("COMMITTED_POSITIVE {}", to_string(&s).unwrap());
    }
}

fn committed_implication_refused(reason: &str, remove: impl Fn(&mut Status)) {
    // Exercise both early-return branches before asserting either outcome.
    let outcomes: Vec<_> = [committed_settling(), committed_failed()]
        .into_iter()
        .map(|mut s| {
            remove(&mut s);
            println!("COMMITTED_NEGATIVE {reason} {}", to_string(&s).unwrap());
            (
                s.state,
                s.validate() == Err(WireError::State),
                assemble(s).err() == Some(crate::catalogue_jobs::status::StatusError::InvalidState),
            )
        })
        .collect();
    assert_eq!(
        outcomes,
        vec![
            (JobState::Running, true, true),
            (JobState::Failed, true, true)
        ],
        "validation and assembly must refuse missing committed {reason}"
    );
}

#[test]
fn committed_everywhere_requires_claimed_gate() {
    committed_implication_refused("gate", |s| s.gate = Gate::Open);
}

#[test]
fn committed_everywhere_requires_accepted_command() {
    committed_implication_refused("command", |s| s.apply.accepted_command_id = None);
}

#[test]
fn committed_everywhere_requires_sealed_source() {
    committed_implication_refused("source", |s| s.source.sha256 = None);
}

#[test]
fn committed_everywhere_requires_preview() {
    committed_implication_refused("preview", |s| s.preview = None);
}

#[test]
fn committed_everywhere_requires_result() {
    committed_implication_refused("result", |s| s.result = None);
}

#[test]
fn audit_terminal_consent_is_spent_in_every_state() {
    for mut s in [
        valid_succeeded(),
        valid_cancelled(),
        valid_stale(),
        valid_expired(),
        valid_failed(),
    ] {
        s.apply = Apply {
            state: ApplyState::Available,
            accepted_command_id: None,
        };
        refused(s, "available terminal consent");
    }
}

#[test]
fn audit_succeeded_and_stale_require_sealed_source() {
    for mut s in [valid_succeeded(), valid_stale()] {
        s.source.sha256 = None;
        refused(s, "unsealed completed preview");
    }
}

#[test]
fn audit_succeeded_requires_accepted_command() {
    let mut s = valid_succeeded();
    s.apply.accepted_command_id = None;
    refused(s, "success without accepted command");
}

#[test]
fn audit_cancelled_requires_matching_stop_failure() {
    let mut s = valid_cancelled();
    s.stop_cause = Some(StopCause::WorkingTimeout);
    refused(s, "cancelled with timeout cause");
    let mut s = valid_cancelled();
    s.failure.as_mut().unwrap().kind = FailureKind::ServerStop;
    refused(s, "user cancellation labelled server stop");
}

#[test]
fn audit_expired_requires_matching_deadline_failure() {
    let mut s = valid_expired();
    s.stop_cause = Some(StopCause::UserCancel);
    refused(s, "expiry without deadline cause");
    let mut s = valid_expired();
    s.failure.as_mut().unwrap().kind = FailureKind::WorkingTimeout;
    refused(s, "wrong deadline failure");
}

#[test]
fn audit_nested_preview_is_checked_when_retained_outside_success() {
    for mut s in [running(Phase::Ready), valid_stale(), valid_failed()] {
        let mut preview = valid_preview();
        preview.artifact.kind = ArtifactKind::Result;
        s.preview = Some(preview);
        refused(s, "result artifact used as retained preview");
    }
}
