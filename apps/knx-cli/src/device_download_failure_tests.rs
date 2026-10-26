//! DIAG-02 for `knx device download`: one cause in the output, history and report.

use super::*;

/// What one failed CLI run must say, in every reader.
struct Expected {
    category: &'static str,
    code: &'static str,
    phase: &'static str,
    step: Option<u64>,
    of: usize,
    written: &'static str,
    restart: &'static str,
}

fn expected(
    category: &'static str,
    code: &'static str,
    step: u64,
    written: &'static str,
) -> Expected {
    Expected {
        category,
        code,
        phase: "inStep",
        step: Some(step),
        of: plan().steps.len(),
        written,
        restart: "notSent",
    }
}

/// Asserts that the run's value, the reopened history row and the report
/// carry one and the same diagnostic, and that the output prints it.
fn assert_failure(run: &Run, expected: &Expected) -> Value {
    assert_eq!(run.exit, ExitCode::FAILURE);
    let failure = run
        .outcome
        .failure
        .as_ref()
        .expect("a failed run keeps its cause");
    let value = serde_json::to_value(failure).unwrap();
    assert_eq!(value["category"], expected.category, "{value}");
    assert_eq!(value["code"], expected.code, "{value}");
    assert_eq!(value["phase"], expected.phase, "{value}");
    assert_eq!(value["restart"], expected.restart, "{value}");
    match expected.step {
        Some(number) => {
            assert_eq!(value["step"]["number"], number, "{value}");
            assert_eq!(value["step"]["of"], expected.of);
        }
        None => assert!(value["step"].is_null(), "no invented step: {value}"),
    }
    let row = reopened(run);
    assert_eq!(row["state"], "failed", "{row}");
    assert_eq!(row["downloadEvidence"]["written"], expected.written);
    assert_eq!(row["downloadEvidence"]["failure"], value, "{row}");
    let report = run.report.as_ref().expect("a stopped run is reported");
    assert_eq!(report["failure"], value, "{report}");
    assert_eq!(report["outcome"]["written"], expected.written);
    assert_diagnostic_before_written(&run.out, failure);
    // The executor's own text stays on the terminal, out of every shared record.
    if let Some(error) = run.out.lines().find_map(|line| {
        line.split_once(": FAILED: ")
            .map(|(_, rest)| rest.trim_end_matches(" =="))
    }) {
        for shared in [report.to_string(), row.to_string()] {
            assert!(!shared.contains(error), "{error} leaked into {shared}");
        }
    }
    value
}

fn assert_diagnostic_before_written(
    out: &str,
    failure: &knx_app::download_failure::DownloadFailure,
) {
    let diagnostic = out
        .lines()
        .position(|line| line == format!("diagnostic: {}", failure.line()))
        .expect("the output includes the exact diagnostic line");
    let written = out
        .lines()
        .position(|line| line.starts_with("written to the device: "))
        .expect("the output includes the written classification");
    assert!(
        diagnostic < written,
        "the diagnostic must precede the written classification: {out}"
    );
}

fn simulated(config: SimulatorConfig) -> SimulatedDevice {
    device(APPLICATION, config)
}

#[test]
fn a_pre_write_identity_refusal_names_step_one_and_writes_nothing() {
    let device = simulated(SimulatorConfig::default());
    let mut wrong = plan();
    wrong.manufacturer = 0x0084;
    let run = run_regular(&device, device.address(), &wrong);
    let failure = assert_failure(
        &run,
        &expected("identityMismatch", "manufacturerMismatch", 1, "no"),
    );
    assert_eq!(failure["readBackOctets"], 0);
    assert_eq!(run.report.as_ref().unwrap()["outcome"]["stoppedInStep"], 1);
}

#[test]
fn a_property_mismatch_keeps_the_device_octets_out_of_every_shared_record() {
    let device = simulated(SimulatorConfig::default());
    let mut wrong = plan();
    wrong.steps[1] = MemoryDownloadStep::CompareProperty {
        object_index: 0,
        property_id: PID_HARDWARE_TYPE,
        inline_data: vec![0, 0, 0, 0, 0x02, 0x99, 0, 0, 0, 0],
    };
    let run = run_regular(&device, device.address(), &wrong);
    let failure = assert_failure(
        &run,
        &expected("identityMismatch", "propertyMismatch", 2, "no"),
    );
    assert_eq!(failure["step"]["summary"], "compare property 0/78");
    let shared = format!("{}{}", run.report.as_ref().unwrap(), reopened(&run));
    for octets in ["01 27", "02 99", "0127", "0299"] {
        assert!(!shared.contains(octets), "{octets} leaked");
    }
}

#[test]
fn a_protected_region_fails_the_backup_before_the_first_change() {
    let device = simulated(SimulatorConfig {
        protected_memory: Some((0x4400, 0x4407)),
        ..SimulatorConfig::default()
    });
    let run = run_regular(&device, device.address(), &plan());
    let failure = assert_failure(
        &run,
        &Expected {
            phase: "backupBeforeStep",
            ..expected("backupFailed", "backupRead", 3, "no")
        },
    );
    assert_eq!(failure["step"]["kind"], "loadRecord");
    // Step 3 never started; step 2 was the last to run.
    assert_eq!(run.report.as_ref().unwrap()["outcome"]["stoppedInStep"], 2);
}

#[test]
fn a_read_back_mismatch_after_the_first_write_is_partial_and_names_its_step() {
    let device = simulated(SimulatorConfig {
        corrupt_memory_writes: true,
        ..SimulatorConfig::default()
    });
    let run = run_regular(&device, device.address(), &plan());
    let failure = assert_failure(
        &run,
        &expected("readBackMismatch", "readBackMismatch", 8, "partially"),
    );
    assert_eq!(
        failure["step"]["summary"],
        "write memory 4000h..4003h, 4 octets"
    );
    assert_eq!(failure["readBackOctets"], 0, "nothing read back unchanged");
}

#[test]
fn the_first_load_record_failure_is_partial_even_before_any_data_write() {
    let device = simulated(SimulatorConfig {
        fail_on_event: Some(LoadEvent::Unload),
        error_code: 1,
        ..SimulatorConfig::default()
    });
    let run = run_regular(&device, device.address(), &plan());
    let failure = assert_failure(
        &run,
        &expected("loadState", "memoryLoadIllegalTransition", 3, "partially"),
    );
    assert_eq!(failure["step"]["kind"], "loadRecord");
    assert_eq!(failure["readBackOctets"], 0);
}

#[test]
fn a_failure_without_a_restart_step_records_not_in_plan() {
    let device = simulated(SimulatorConfig::default());
    let mut wrong = plan();
    wrong.manufacturer = 0x0084;
    wrong
        .steps
        .retain(|step| !matches!(step, MemoryDownloadStep::Restart));
    let run = run_regular(&device, device.address(), &wrong);
    assert_failure(
        &run,
        &Expected {
            restart: "notInPlan",
            of: wrong.steps.len(),
            ..expected("identityMismatch", "manufacturerMismatch", 1, "no")
        },
    );
}

#[test]
fn an_unanswered_connect_times_out_before_any_change() {
    let device = simulated(SimulatorConfig {
        unanswered_connects: Some(0..1000),
        ..SimulatorConfig::default()
    });
    let run = run_regular(&device, device.address(), &plan());
    assert_failure(&run, &expected("timeout", "noAnswer", 1, "no"));
}

#[test]
fn a_transport_loss_after_the_writes_counts_what_was_read_back() {
    let device = simulated(SimulatorConfig {
        drop_connection_after: Some(70),
        ..SimulatorConfig::default()
    });
    let run = run_regular(&device, device.address(), &plan());
    let failure = assert_failure(
        &run,
        &expected("connectionLost", "connectionLost", 13, "partially"),
    );
    assert_eq!(failure["readBackOctets"], 12, "both segments read back");
}

#[test]
fn a_load_state_error_names_the_load_step() {
    let device = simulated(SimulatorConfig {
        fail_on_event: Some(LoadEvent::LoadCompleted),
        error_code: 1,
        ..SimulatorConfig::default()
    });
    let run = run_regular(&device, device.address(), &plan());
    let failure = assert_failure(
        &run,
        &expected("loadState", "memoryLoadIllegalTransition", 9, "partially"),
    );
    assert_eq!(failure["readBackOctets"], 4);
}

/// A session that cannot even be built sent nothing and names no step.
#[test]
fn a_session_refusal_fails_before_the_first_step() {
    let device = simulated(SimulatorConfig::default());
    let own = device.assigned_address();
    let run = run_regular(&device, own, &plan());
    assert_failure(
        &run,
        &Expected {
            phase: "beforeSteps",
            step: None,
            ..expected("planRefused", "selfAddressed", 0, "no")
        },
    );
    assert!(device.seen().is_empty(), "nothing was sent");
    assert!(run.report.as_ref().unwrap()["outcome"]["stoppedInStep"].is_null());
}

#[test]
fn successful_and_restart_unconfirmed_runs_carry_no_failure() {
    let completed_device = simulated(SimulatorConfig::default());
    let completed = run_regular(&completed_device, completed_device.address(), &plan());
    let unconfirmed_device = simulated(SimulatorConfig {
        restart_unanswered: true,
        ..SimulatorConfig::default()
    });
    let unconfirmed = run_regular(
        &unconfirmed_device,
        unconfirmed_device.address(),
        &loaded_plan(),
    );
    for (run, restart) in [(&completed, "acknowledged"), (&unconfirmed, "unconfirmed")] {
        assert!(run.outcome.failure.is_none());
        let row = reopened(run);
        assert_eq!(row["state"], "finished", "{row}");
        assert_eq!(row["downloadEvidence"]["restart"], restart);
        assert!(row["downloadEvidence"].get("failure").is_none());
        let report = run.report.as_ref().unwrap();
        assert!(report.get("failure").is_none(), "{report}");
        assert!(!run.out.contains("diagnostic: "), "{}", run.out);
    }
}

/// Simulator coverage of the exact execution/record helper used by restore,
/// not a full CLI invocation or restore-input/admission test.
#[test]
fn restore_execution_composition_keeps_a_partial_failure_in_reopened_history() {
    let dir = tempfile::tempdir().unwrap();
    let device = simulated(SimulatorConfig {
        corrupt_memory_writes: true,
        ..SimulatorConfig::default()
    });
    let plan = plan();
    let mut activity = start(&dir, device.address());
    let backups = dir.path().join("backups");
    let mut out = Vec::new();
    let (witnessed, recorded) = runtime().block_on(execute_and_record_download(
        &device,
        WriteAuthorisation::for_simulator(device.address(), WriteScope::Download).unwrap(),
        download_keying(plan.mask, None, KeySource::None),
        fast(),
        DownloadRun {
            plan: &plan,
            backup: &backup(&backups),
        },
        &mut activity,
        &mut out,
    ));
    assert!(recorded);
    assert_eq!(
        witnessed.result,
        DownloadResult::Failed {
            written: Written::Partially
        }
    );
    let failure = witnessed.failure.as_ref().expect("restore keeps its cause");
    assert_eq!(
        failure.code,
        knx_app::download_failure::FailureCode::ReadBackMismatch
    );
    assert_eq!(failure.step.as_ref().unwrap().number, 8);
    assert_eq!(witnessed.stopped_in_step, Some(8));
    assert_diagnostic_before_written(&String::from_utf8(out).unwrap(), failure);
    let row = reopened_at(&dir.path().join("history.sqlite"));
    assert_eq!(row["state"], "failed");
    assert_eq!(row["downloadEvidence"]["written"], "partially");
    assert_eq!(
        row["downloadEvidence"]["failure"],
        serde_json::to_value(failure).unwrap()
    );
    activity.record_cleanup(true).unwrap();
}

#[test]
fn a_latched_history_failure_keeps_the_original_recording_error() {
    let dir = tempfile::tempdir().unwrap();
    let history = dir.path().join("history.sqlite");
    let log = Arc::new(OneShotLog::for_run(history.clone()).unwrap());
    let device = simulated(SimulatorConfig::default());
    let mut activity = log.start_download(1, device.address()).unwrap();
    // A real storage failure latches history unavailable before recording.
    std::fs::rename(&history, dir.path().join("history-away.sqlite")).unwrap();
    std::fs::create_dir(&history).unwrap();
    assert!(log.ensure_write_available().is_err());
    assert_eq!(log.history_state(), "unavailable");
    let witnessed = Witnessed {
        result: DownloadResult::Failed {
            written: Written::No,
        },
        stopped_in_step: Some(1),
        failure: Some(knx_app::download_failure::DownloadFailure::task_ended(
            &plan(),
            Some(1),
            0,
        )),
    };
    let mut warnings = Vec::new();
    let error = witnessed.record(&mut activity, &mut warnings).unwrap_err();
    assert!(
        warnings.is_empty(),
        "no claim that unavailable history kept the failure"
    );
    assert_eq!(error.to_string(), "activity history is unavailable");
}

/// A diagnostic the history refuses still leaves the row failed, cause not
/// recorded; it never degrades to `unknown` or blocks the result.
#[test]
fn a_refused_diagnostic_still_records_the_failed_row() {
    let dir = tempfile::tempdir().unwrap();
    let device = simulated(SimulatorConfig::default());
    let mut activity = start(&dir, device.address());
    let mut failure = knx_app::download_failure::DownloadFailure::task_ended(&plan(), Some(8), 0);
    // Read-back octets on a row that wrote nothing: refused by the validator.
    failure.read_back_octets = 4;
    let witnessed = Witnessed {
        result: DownloadResult::Failed {
            written: Written::No,
        },
        stopped_in_step: Some(8),
        failure: Some(failure),
    };
    let mut warnings = Vec::new();
    witnessed.record(&mut activity, &mut warnings).unwrap();
    let row = reopened_at(&dir.path().join("history.sqlite"));
    assert_eq!(row["state"], "failed", "{row}");
    assert_eq!(row["downloadEvidence"]["written"], "no");
    assert!(row["downloadEvidence"].get("failure").is_none(), "{row}");
    assert_eq!(
        String::from_utf8(warnings).unwrap(),
        "WARNING: history kept the failure without its cause\n"
    );
}

#[test]
fn a_refused_diagnostic_warns_and_keeps_the_partial_write_classification() {
    let dir = tempfile::tempdir().unwrap();
    let device = simulated(SimulatorConfig::default());
    let mut activity = start(&dir, device.address());
    activity.mark_send_possible().unwrap();
    let mut failure = knx_app::download_failure::DownloadFailure::task_ended(&plan(), Some(8), 4);
    // A category/code mismatch is refused independently of the write witness.
    failure.category = knx_app::download_failure::FailureCategory::Timeout;
    let witnessed = Witnessed {
        result: DownloadResult::Failed {
            written: Written::Partially,
        },
        stopped_in_step: Some(8),
        failure: Some(failure),
    };
    let mut warnings = Vec::new();
    witnessed.record(&mut activity, &mut warnings).unwrap();
    let row = reopened_at(&dir.path().join("history.sqlite"));
    assert_eq!(row["state"], "failed", "{row}");
    assert_eq!(row["downloadEvidence"]["written"], "partially");
    assert!(row["downloadEvidence"].get("failure").is_none(), "{row}");
    assert_eq!(
        String::from_utf8(warnings).unwrap(),
        "WARNING: history kept the failure without its cause\n"
    );
}
