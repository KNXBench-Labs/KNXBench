//! Unit tests for the payload-free download failure diagnostic.

use std::time::Duration;

use knx_core::commissioning::load_control_memory::{event_record, MemoryLoadStateMachine};
use knx_core::commissioning::load_state::{LoadEvent, MaskVersion};
use knx_core::commissioning::memory_download::{MemoryDownloadPlan, MemoryDownloadStep};
use knx_net::commissioning::memory_download::MemoryDownloadError;
use knx_net::commissioning::SessionError;

use super::*;

fn plan(with_restart: bool) -> MemoryDownloadPlan {
    let machine = MemoryLoadStateMachine::ApplicationProgram;
    let mut steps = vec![
        MemoryDownloadStep::Connect,
        MemoryDownloadStep::CompareProperty {
            object_index: 0,
            property_id: 78,
            inline_data: vec![0xDE, 0xAD, 0xBE, 0xEF],
        },
        MemoryDownloadStep::LoadRecord(event_record(machine, LoadEvent::StartLoading).unwrap()),
        MemoryDownloadStep::WriteMemory {
            address: 0x4000,
            octets: vec![0xCA, 0xFE, 0x12],
        },
        MemoryDownloadStep::RequireLoaded(machine),
    ];
    if with_restart {
        steps.push(MemoryDownloadStep::Restart);
    }
    steps.push(MemoryDownloadStep::Disconnect);
    MemoryDownloadPlan {
        mask: MaskVersion(0x0701),
        manufacturer: 0x0083,
        steps,
    }
}

fn no_answer() -> SessionError {
    SessionError::NoAnswer {
        waiting_for: "A_Memory_Response",
        each: Duration::from_secs(3),
        attempts: 3,
    }
}

#[test]
fn a_timeout_in_a_write_step_names_the_step_without_its_octets() {
    let error = MemoryDownloadError::Session {
        index: Some(3),
        error: no_answer(),
    };
    let failure = DownloadFailure::from_error(&error, &plan(true), Some(4), 2);
    assert_eq!(failure.category, FailureCategory::Timeout);
    assert_eq!(failure.code, FailureCode::NoAnswer);
    assert_eq!(failure.restart, FailureRestart::NotSent);
    assert_eq!(failure.read_back_octets, 2);
    let step = failure.step.as_ref().unwrap();
    assert_eq!(
        (step.number, step.of, step.kind),
        (4, 7, StepKind::WriteMemory)
    );
    assert_eq!(step.summary, "write memory 4000h..4002h, 3 octets");
    let json = serde_json::to_string(&failure).unwrap();
    for leaked in ["CA", "FE", "ca fe", "A_Memory_Response"] {
        assert!(!json.contains(leaked), "{leaked} leaked into {json}");
    }
    assert_eq!(
        json,
        r#"{"category":"timeout","code":"noAnswer","phase":"inStep","step":{"number":4,"of":7,"kind":"writeMemory","summary":"write memory 4000h..4002h, 3 octets"},"restart":"notSent","readBackOctets":2}"#
    );
    assert!(failure.is_consistent());
    assert_eq!(
        failure.line(),
        "failed in step 4 of 7 (write memory 4000h..4002h, 3 octets): the device did not \
         answer in time [noAnswer]; 2 octets read back; restart not sent"
    );
}

#[test]
fn compare_and_load_steps_keep_their_inline_data_out() {
    let compare = FailedStep::of_plan(&plan(true), 2).unwrap();
    assert_eq!(compare.summary, "compare property 0/78");
    let load = FailedStep::of_plan(&plan(true), 3).unwrap();
    assert_eq!(load.summary, "load control: load (application program)");
    assert_eq!(load.kind, StepKind::LoadRecord);
    assert!(FailedStep::of_plan(&plan(true), 0).is_none());
    assert!(FailedStep::of_plan(&plan(true), 8).is_none());
}

#[test]
fn restart_evidence_follows_the_plan_and_never_guesses() {
    let lost = |index| MemoryDownloadError::Session {
        index,
        error: SessionError::ConnectionLost { during: "connect" },
    };
    // Before the first step: no step, however far the caller thinks it got.
    let before = DownloadFailure::from_error(&lost(None), &plan(true), Some(4), 0);
    assert_eq!(before.phase, FailurePhase::BeforeSteps);
    assert_eq!(before.restart, FailureRestart::NotSent);
    assert!(before.step.is_none());
    assert_eq!(
        before.line(),
        "failed before the first step: the connection to the device ended [connectionLost]; \
         0 octets read back; restart not sent"
    );
    // The error's own index wins over the last started step.
    let own = DownloadFailure::from_error(&lost(Some(1)), &plan(true), Some(4), 0);
    assert_eq!(own.step.as_ref().unwrap().number, 2);
    assert_eq!(own.restart, FailureRestart::NotSent);
    // In the restart step itself: unknown.
    let restart = DownloadFailure::from_error(&lost(Some(5)), &plan(true), Some(6), 3);
    assert_eq!(restart.step.as_ref().unwrap().kind, StepKind::Restart);
    assert_eq!(restart.restart, FailureRestart::Unknown);
    // After it (disconnect): unknown, never "sent".
    let after = DownloadFailure::from_error(&lost(Some(6)), &plan(true), Some(7), 3);
    assert_eq!(after.restart, FailureRestart::Unknown);
    // Without a restart step.
    let none = DownloadFailure::from_error(&lost(Some(1)), &plan(false), Some(2), 0);
    assert_eq!(none.restart, FailureRestart::NotInPlan);
    // An unwitnessed end before the restart step is still unknown.
    let ended = DownloadFailure::task_ended(&plan(true), Some(2), 0);
    assert_eq!(ended.code, FailureCode::TaskEnded);
    assert_eq!(ended.category, FailureCategory::TaskEnded);
    assert_eq!(ended.restart, FailureRestart::Unknown);
}

#[test]
fn a_backup_or_stop_names_the_step_that_did_not_start() {
    let read = MemoryDownloadError::BackupRead {
        index: 2,
        error: SessionError::MemoryRefused { address: 0x4000 },
    };
    // The caller last started step 2; the backup preceded step 3.
    let backup = DownloadFailure::from_error(&read, &plan(true), Some(2), 0);
    assert_eq!(backup.phase, FailurePhase::BackupBeforeStep);
    assert_eq!(backup.step.as_ref().unwrap().number, 3);
    assert_eq!(backup.category, FailureCategory::BackupFailed);
    assert_eq!(backup.restart, FailureRestart::NotSent);
    assert!(
        backup.line().contains("in the backup before step 3 of 7"),
        "{}",
        backup.line()
    );
    let kept = MemoryDownloadError::BackupNotKept("disk full".into());
    let kept = DownloadFailure::from_error(&kept, &plan(true), Some(2), 0);
    assert_eq!(
        kept.step.as_ref().unwrap().number,
        3,
        "the first changing step"
    );
    assert!(!serde_json::to_string(&kept).unwrap().contains("disk full"));
    let stop = MemoryDownloadError::Stopped {
        before_step: 2,
        intent_recorded: true,
    };
    let stop = DownloadFailure::from_error(&stop, &plan(true), Some(2), 0);
    assert_eq!(stop.phase, FailurePhase::StoppedBeforeStep);
    assert_eq!(stop.step.as_ref().unwrap().number, 3);
    assert!(
        stop.line().starts_with("stopped before step 3 of 7"),
        "{}",
        stop.line()
    );
    let shape = DownloadFailure::from_error(
        &MemoryDownloadError::DoesNotConnectFirst,
        &plan(true),
        Some(3),
        0,
    );
    assert_eq!(
        (shape.phase, shape.step.is_none()),
        (FailurePhase::BeforeSteps, true)
    );
}

#[test]
fn cancellation_refusal_and_mismatch_are_distinct_categories() {
    let stopped = MemoryDownloadError::Stopped {
        before_step: 2,
        intent_recorded: true,
    };
    let stopped = DownloadFailure::from_error(&stopped, &plan(true), Some(2), 0);
    assert_eq!(stopped.category, FailureCategory::Stopped);
    assert!(stopped.line().starts_with("stopped before step 3 of 7"));
    let mask = MemoryDownloadError::MaskMismatch {
        expected: MaskVersion(0x0701),
        found: MaskVersion(0x0705),
    };
    let mask = DownloadFailure::from_error(&mask, &plan(true), Some(1), 0);
    assert_eq!(mask.category, FailureCategory::IdentityMismatch);
    assert_eq!(
        (mask.phase, mask.step.as_ref().unwrap().number),
        (FailurePhase::InStep, 1)
    );
    let refused = MemoryDownloadError::Session {
        index: Some(3),
        error: SessionError::MemoryRefused { address: 0x4000 },
    };
    let refused = DownloadFailure::from_error(&refused, &plan(true), Some(4), 0);
    assert_eq!(refused.category, FailureCategory::DeviceRefused);
    let session = DownloadFailure::from_session_refusal(&SessionError::NotConnected, &plan(true));
    assert_eq!(session.category, FailureCategory::ConnectionLost);
    assert!(session.step.is_none());
}

#[test]
fn persisted_values_are_admitted_only_when_consistent() {
    let good = DownloadFailure::task_ended(&plan(true), Some(4), 1);
    let mut wrong_category = good.clone();
    wrong_category.category = FailureCategory::Timeout;
    assert!(!wrong_category.is_consistent());
    let mut stepless = good.clone();
    stepless.phase = FailurePhase::BeforeSteps;
    assert!(!stepless.is_consistent(), "a step before the first step");
    let mut out_of_range = good.clone();
    out_of_range.step.as_mut().unwrap().number = 9;
    assert!(!out_of_range.is_consistent());
    let mut long = good.clone();
    long.step.as_mut().unwrap().summary = "x".repeat(MAX_STEP_SUMMARY + 1);
    assert!(!long.is_consistent());
    let mut stopped_with_writes = DownloadFailure::from_error(
        &MemoryDownloadError::Stopped {
            before_step: 1,
            intent_recorded: false,
        },
        &plan(true),
        None,
        0,
    );
    stopped_with_writes.read_back_octets = 1;
    assert!(!stopped_with_writes.is_consistent());
    // Unknown fields and tokens are refused, not ignored.
    let json = serde_json::to_value(&good).unwrap();
    let mut extra = json.clone();
    extra["error"] = "raw text".into();
    assert!(serde_json::from_value::<DownloadFailure>(extra).is_err());
    let mut token = json.clone();
    token["code"] = "somethingNew".into();
    assert!(serde_json::from_value::<DownloadFailure>(token).is_err());
    assert_eq!(
        serde_json::from_value::<DownloadFailure>(json).unwrap(),
        good
    );
}

#[test]
fn every_code_token_round_trips_and_has_an_explanation() {
    use FailureCode as F;
    let all = [
        F::DoesNotConnectFirst,
        F::CheckAfterWrite,
        F::StopRefused,
        F::Excluded,
        F::SelfAddressed,
        F::NoAuthorisation,
        F::NotASimulator,
        F::AuthorisationRefused,
        F::LoadNotMemoryMapped,
        F::VerifyModeWithMemoryLoad,
        F::MaskMismatch,
        F::ManufacturerMismatch,
        F::PropertyMismatch,
        F::BackupRead,
        F::BackupShortRead,
        F::BackupNotKept,
        F::NoAnswer,
        F::TransitionTimedOut,
        F::Transport,
        F::NotConnected,
        F::ConnectionLost,
        F::ConnectRejected,
        F::ConnectionReleased,
        F::PropertyRefused,
        F::MemoryRefused,
        F::ReadBackMismatch,
        F::PropertyReadBackMismatch,
        F::NotLoaded,
        F::UnexpectedLoadState,
        F::UnknownLoadState,
        F::LoadErrorCode,
        F::LoadStateUnchanged,
        F::IllegalLoadTransition,
        F::AllocationFailed,
        F::MemoryLoadNotSettled,
        F::MemoryLoadIllegalTransition,
        F::MalformedProperty,
        F::MalformedDescriptor,
        F::MalformedRestartResponse,
        F::MalformedMemoryLoadState,
        F::Encode,
        F::Chunk,
        F::Lagged,
        F::Stopped,
        F::TaskEnded,
    ];
    let mut tokens = std::collections::BTreeSet::new();
    for code in all {
        let token = code_token(code);
        assert!(!token.is_empty() && tokens.insert(token.clone()), "{token}");
        let back: FailureCode = serde_json::from_value(token.into()).unwrap();
        assert_eq!(back, code);
        assert!(!explanation(code).is_empty());
    }
    assert_eq!(tokens.len(), 45);
}

#[test]
fn a_short_backup_read_and_a_backup_before_the_restart_say_not_sent() {
    let short = MemoryDownloadError::BackupShortRead {
        address: 0x4000,
        asked: 4,
        got: 2,
    };
    let short = DownloadFailure::from_error(&short, &plan(true), Some(2), 0);
    assert_eq!(short.code, FailureCode::BackupShortRead);
    assert_eq!(short.phase, FailurePhase::BackupBeforeStep);
    assert_eq!(
        short.step.as_ref().unwrap().number,
        3,
        "the first changing step"
    );
    // A backup or stop before the restart step itself: the restart never went.
    let restart_plan = MemoryDownloadPlan {
        steps: vec![
            MemoryDownloadStep::Connect,
            MemoryDownloadStep::Restart,
            MemoryDownloadStep::Disconnect,
        ],
        ..plan(true)
    };
    let before = MemoryDownloadError::BackupRead {
        index: 1,
        error: SessionError::NotConnected,
    };
    let before = DownloadFailure::from_error(&before, &restart_plan, Some(1), 0);
    assert_eq!(before.step.as_ref().unwrap().kind, StepKind::Restart);
    assert_eq!(before.restart, FailureRestart::NotSent);
    let stop = MemoryDownloadError::Stopped {
        before_step: 1,
        intent_recorded: true,
    };
    let stop = DownloadFailure::from_error(&stop, &restart_plan, Some(1), 0);
    assert_eq!(stop.restart, FailureRestart::NotSent);
    let mask = MemoryDownloadError::MaskMismatch {
        expected: MaskVersion(0x0701),
        found: MaskVersion(0x0705),
    };
    let mask = DownloadFailure::from_error(&mask, &plan(true), Some(1), 0);
    assert_eq!(mask.restart, FailureRestart::NotSent);
    for failure in [short, before, stop, mask] {
        assert!(failure.is_consistent(), "{failure:?}");
    }
}

#[test]
fn contradictory_stored_values_are_inconsistent() {
    let good = DownloadFailure::task_ended(&plan(true), Some(4), 1);
    assert!(good.is_consistent());
    let mut stop_in_step = DownloadFailure::from_error(
        &MemoryDownloadError::Stopped {
            before_step: 1,
            intent_recorded: false,
        },
        &plan(true),
        None,
        0,
    );
    stop_in_step.phase = FailurePhase::InStep;
    assert!(
        !stop_in_step.is_consistent(),
        "a stop is always before a step"
    );
    let mut backup_in_step = DownloadFailure::from_error(
        &MemoryDownloadError::BackupNotKept("full".into()),
        &plan(true),
        Some(2),
        0,
    );
    backup_in_step.phase = FailurePhase::InStep;
    assert!(
        !backup_in_step.is_consistent(),
        "a backup precedes its step"
    );
    let lost = MemoryDownloadError::Session {
        index: Some(5),
        error: SessionError::ConnectionLost { during: "restart" },
    };
    let mut optimistic = DownloadFailure::from_error(&lost, &plan(true), Some(6), 3);
    assert_eq!(optimistic.restart, FailureRestart::Unknown);
    optimistic.restart = FailureRestart::NotSent;
    assert!(
        !optimistic.is_consistent(),
        "in the restart step it is unknown"
    );
    let mut huge = good.clone();
    huge.read_back_octets = MAX_COUNT + 1;
    assert!(!huge.is_consistent());
    let mut accented = good.clone();
    accented.step.as_mut().unwrap().summary = "write mémoire".into();
    assert!(!accented.is_consistent(), "summaries are printable ASCII");
}
