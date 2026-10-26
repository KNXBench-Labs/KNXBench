//! DIAG-02 for `knx device download`: one cause in the output, history and report.

use super::*;

/// What one failed CLI run must say, in every reader.
struct Expected {
    category: &'static str,
    code: &'static str,
    phase: &'static str,
    step: Option<u64>,
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
            assert_eq!(value["step"]["of"], plan().steps.len());
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
    assert!(
        run.out.contains(&format!("diagnostic: {}", failure.line())),
        "{}",
        run.out
    );
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
    witnessed.record(&mut activity).unwrap();
    let row = reopened_at(&dir.path().join("history.sqlite"));
    assert_eq!(row["state"], "failed", "{row}");
    assert_eq!(row["downloadEvidence"]["written"], "no");
    assert!(row["downloadEvidence"].get("failure").is_none(), "{row}");
}
