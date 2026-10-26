//! DIAG-02: one failed run's cause, step and write uncertainty, the same in every reader.

use super::*;
use crate::session_log::{LogEntry, Severity};

/// What each reader of one ended run says.
struct Readers {
    status: Value,
    report: Value,
    history: Value,
    /// The session log's `DeviceDownload` entries.
    log: Vec<LogEntry>,
}

fn readers(started: &Started) -> Readers {
    Readers {
        status: serde_json::to_value(started.session.status()).unwrap(),
        report: serde_json::to_value(started.session.report().unwrap()).unwrap(),
        history: reopened(&started.history),
        log: started
            ._server
            .session_log
            .lock()
            .unwrap()
            .entries()
            .iter()
            .filter(|entry| entry.source == "DeviceDownload")
            .cloned()
            .collect(),
    }
}

async fn run(device: &Arc<SimulatedDevice>, plan: MemoryDownloadPlan) -> Started {
    let mut started = start_receipted(
        device,
        plan,
        AfterRestart::Pass,
        None,
        Arc::new(DownloadStop::new()),
        |_| {},
    );
    started.session.join().await;
    started
}

fn simulated(config: SimulatorConfig) -> Arc<SimulatedDevice> {
    device(APPLICATION, config)
}

/// The expected diagnostic of one failed run.
struct Expected<'a> {
    category: &'static str,
    code: &'a str,
    phase: &'static str,
    step: Option<u64>,
    written: &'static str,
    restart: &'static str,
    severity: Severity,
}

/// Asserts that status, report, durable history and log agree on one failure,
/// and that the raw error text reaches neither shareable surface.
fn assert_failure(readers: &Readers, expected: &Expected<'_>) -> Value {
    let Readers {
        status,
        report,
        history,
        log,
    } = readers;
    assert_eq!(status["state"], "failed", "{status}");
    let failure = &status["failure"];
    assert_eq!(failure["category"], expected.category, "{status}");
    assert_eq!(failure["code"], expected.code, "{status}");
    assert_eq!(failure["phase"], expected.phase, "{status}");
    assert_eq!(failure["restart"], expected.restart, "{status}");
    assert_eq!(status["written"], expected.written, "{status}");
    match expected.step {
        Some(number) => {
            assert_eq!(failure["step"]["number"], number, "{status}");
            if expected.phase == "inStep" {
                assert_eq!(status["stoppedInStep"], number, "the step that ran");
            }
            assert_eq!(failure["step"]["of"], plan().steps.len());
            assert!(failure["step"]["summary"]
                .as_str()
                .is_some_and(|summary| !summary.is_empty()));
        }
        None => {
            assert!(failure["step"].is_null(), "no invented step: {status}");
            assert!(status["stoppedInStep"].is_null());
        }
    }
    // The report carries the same value next to its outcome.
    assert_eq!(report["failure"], *failure, "{report}");
    assert_eq!(report["outcome"]["completed"], false);
    assert_eq!(report["outcome"]["written"], expected.written);
    assert_eq!(report["outcome"]["stoppedInStep"], status["stoppedInStep"]);
    // The durable history, read back by a fresh reader.
    assert_eq!(history["state"], "failed", "{history}");
    assert_eq!(
        history["downloadEvidence"]["failure"], *failure,
        "{history}"
    );
    assert_eq!(history["downloadEvidence"]["written"], expected.written);
    // Exactly one cause entry, linked to the run.
    assert_eq!(log.len(), 1, "{log:?}");
    let entry = &log[0];
    assert_eq!(entry.severity, expected.severity, "{entry:?}");
    assert_eq!(entry.detail.as_deref(), Some("download 1"));
    assert!(
        entry.message.contains(&format!("[{}]", expected.code))
            && entry
                .message
                .ends_with(&format!("written {}", expected.written)),
        "{entry:?}"
    );
    // Never the raw error text in a shareable or durable record.
    let error = status["error"].as_str().unwrap();
    assert!(!error.is_empty());
    for shared in [report.to_string(), history.to_string()] {
        assert!(!shared.contains(error), "{error} leaked into {shared}");
    }
    failure.clone()
}

fn expected(
    category: &'static str,
    code: &'static str,
    step: u64,
    written: &'static str,
) -> Expected<'static> {
    Expected {
        category,
        code,
        phase: "inStep",
        step: Some(step),
        written,
        restart: "notSent",
        severity: Severity::Error,
    }
}

#[tokio::test]
async fn a_pre_write_identity_refusal_names_step_one_and_writes_nothing() {
    let device = simulated(SimulatorConfig::default());
    let mut wrong = plan();
    wrong.manufacturer = 0x0084;
    let started = run(&device, wrong).await;
    assert_eq!(mutating_frames(&device), 0);
    assert_failure(
        &readers(&started),
        &expected("identityMismatch", "manufacturerMismatch", 1, "no"),
    );
}

#[tokio::test]
async fn a_property_mismatch_keeps_the_device_octets_out_of_every_shared_record() {
    let device = simulated(SimulatorConfig::default());
    let mut wrong = plan();
    wrong.steps[1] = MemoryDownloadStep::CompareProperty {
        object_index: 0,
        property_id: PID_HARDWARE_TYPE,
        inline_data: vec![0, 0, 0, 0, 0x02, 0x99, 0, 0, 0, 0],
    };
    let started = run(&device, wrong).await;
    let readers = readers(&started);
    let failure = assert_failure(
        &readers,
        &expected("identityMismatch", "propertyMismatch", 2, "no"),
    );
    assert_eq!(failure["step"]["summary"], "compare property 0/78");
    let shared = format!("{}{}", readers.report, readers.history);
    for octets in ["01 27", "02 99", "0127", "0299"] {
        assert!(!shared.contains(octets), "{octets} leaked");
    }
}

#[tokio::test]
async fn a_protected_region_fails_the_backup_before_the_first_change() {
    let device = simulated(SimulatorConfig {
        protected_memory: Some((0x4400, 0x4407)),
        ..SimulatorConfig::default()
    });
    let started = run(&device, plan()).await;
    assert_eq!(mutating_frames(&device), 0);
    let readers = readers(&started);
    let failure = assert_failure(
        &readers,
        &Expected {
            phase: "backupBeforeStep",
            ..expected("backupFailed", "backupRead", 3, "no")
        },
    );
    // The backup preceded step 3, which never started; step 2 was the last
    // to run, and the status keeps saying so.
    assert_eq!(readers.status["stoppedInStep"], 2);
    assert_eq!(failure["step"]["kind"], "loadRecord");
}

#[tokio::test]
async fn a_read_back_mismatch_after_the_first_write_is_partial_and_names_its_step() {
    let device = simulated(SimulatorConfig {
        corrupt_memory_writes: true,
        ..SimulatorConfig::default()
    });
    let started = run(&device, plan()).await;
    let failure = assert_failure(
        &readers(&started),
        &expected("readBackMismatch", "readBackMismatch", 8, "partially"),
    );
    assert_eq!(failure["step"]["kind"], "writeMemory");
    assert_eq!(
        failure["step"]["summary"],
        "write memory 4000h..4003h, 4 octets"
    );
    assert_eq!(failure["readBackOctets"], 0, "nothing read back unchanged");
}

#[tokio::test]
async fn an_unanswered_connect_times_out_before_any_change() {
    let device = simulated(SimulatorConfig {
        unanswered_connects: Some(0..1000),
        ..SimulatorConfig::default()
    });
    let started = run(&device, plan()).await;
    assert_eq!(mutating_frames(&device), 0);
    assert_failure(
        &readers(&started),
        &expected("timeout", "noAnswer", 1, "no"),
    );
}

#[tokio::test]
async fn a_silent_device_is_a_released_connection_not_an_invented_timeout() {
    let device = simulated(SimulatorConfig {
        silent: true,
        ..SimulatorConfig::default()
    });
    let started = run(&device, plan()).await;
    assert_eq!(mutating_frames(&device), 0);
    assert_failure(
        &readers(&started),
        &expected("connectionLost", "connectionReleased", 1, "no"),
    );
}

#[tokio::test]
async fn a_transport_loss_after_the_writes_is_partial_and_counts_what_was_read_back() {
    let device = simulated(SimulatorConfig {
        drop_connection_after: Some(70),
        ..SimulatorConfig::default()
    });
    let started = run(&device, plan()).await;
    let failure = assert_failure(
        &readers(&started),
        &expected("connectionLost", "connectionLost", 13, "partially"),
    );
    assert_eq!(failure["readBackOctets"], 12, "both segments read back");
}

#[tokio::test]
async fn a_load_state_error_names_the_load_step() {
    let device = simulated(SimulatorConfig {
        fail_on_event: Some(LoadEvent::LoadCompleted),
        error_code: 1,
        ..SimulatorConfig::default()
    });
    let started = run(&device, plan()).await;
    let failure = assert_failure(
        &readers(&started),
        &expected("loadState", "memoryLoadIllegalTransition", 9, "partially"),
    );
    assert_eq!(failure["readBackOctets"], 4);
}

#[tokio::test]
async fn a_stop_on_request_is_a_warning_naming_the_step_that_did_not_start() {
    let device = simulated(SimulatorConfig::default());
    let mut started = start_receipted(
        &device,
        plan(),
        AfterRestart::Pass,
        None,
        Arc::new(DownloadStop::new()),
        |stop| assert_eq!(stop.request(), StopRequest::WillStop),
    );
    started.session.join().await;
    assert_eq!(mutating_frames(&device), 0);
    let readers = readers(&started);
    let next = readers.status["stoppedInStep"]
        .as_u64()
        .map(|last| last + 1);
    let failure = assert_failure(
        &readers,
        &Expected {
            category: "stopped",
            code: "stopped",
            phase: "stoppedBeforeStep",
            step: next,
            written: "no",
            restart: "notSent",
            severity: Severity::Warning,
        },
    );
    assert_eq!(failure["readBackOctets"], 0);
    assert!(
        readers.log[0].message.contains(" stopped before step "),
        "{:?}",
        readers.log
    );
}

#[tokio::test]
async fn an_unwitnessed_end_is_task_ended_with_unknown_restart_and_no_durable_cause() {
    let device = simulated(SimulatorConfig::default());
    let gate = Gate::at(GateAt::FirstMemoryWrite);
    let mut started = start_receipted(
        &device,
        plan(),
        AfterRestart::Pass,
        Some(gate.clone()),
        Arc::new(DownloadStop::new()),
        |_| {},
    );
    tokio::time::timeout(Duration::from_secs(5), gate.entered.notified())
        .await
        .expect("writing");
    started.session.task.as_ref().unwrap().abort();
    started.session.join().await;
    let readers = readers(&started);
    let failure = &readers.status["failure"];
    assert_eq!(failure["category"], "taskEnded", "{}", readers.status);
    assert_eq!(failure["restart"], "unknown");
    assert_eq!(failure["phase"], "inStep");
    assert_eq!(failure["step"]["number"], 3, "the first load record ran");
    assert_eq!(readers.status["written"], "partially");
    assert_eq!(readers.report["failure"], *failure);
    // The history guard was dropped with the task: the row stays `unknown`
    // and carries no cause it never witnessed.
    assert_eq!(readers.history["state"], "unknown", "{}", readers.history);
    assert!(readers.history["downloadEvidence"].get("failure").is_none());
    assert_eq!(readers.log.len(), 1);
    assert_eq!(readers.log[0].severity, Severity::Error);
    assert!(readers.log[0].message.contains("[taskEnded]"));
}

#[tokio::test]
async fn polling_a_failed_run_never_logs_its_cause_twice() {
    let device = simulated(SimulatorConfig::default());
    let mut wrong = plan();
    wrong.manufacturer = 0x0084;
    let started = run(&device, wrong).await;
    for since in 0..20 {
        let _ = started.session.snapshot_since(since);
        let _ = started.session.status();
        let _ = started.session.report();
    }
    assert_eq!(readers(&started).log.len(), 1);
}

#[tokio::test]
async fn successful_and_restart_unconfirmed_runs_carry_no_failure() {
    let device_a = simulated(SimulatorConfig::default());
    let completed = run(&device_a, plan()).await;
    let device_b = simulated(SimulatorConfig {
        restart_unanswered: true,
        ..SimulatorConfig::default()
    });
    let unconfirmed = run(&device_b, loaded_plan()).await;
    for (started, restart) in [(&completed, "acknowledged"), (&unconfirmed, "unconfirmed")] {
        let readers = readers(started);
        assert_eq!(readers.status["state"], "finished", "{}", readers.status);
        assert_eq!(readers.status["restart"], restart);
        assert!(readers.status.get("failure").is_none());
        assert!(
            readers.report.get("failure").is_none(),
            "{}",
            readers.report
        );
        assert!(readers.history["downloadEvidence"].get("failure").is_none());
        assert!(readers
            .log
            .iter()
            .all(|entry| entry.severity == Severity::Info));
    }
}
