//! Verifies CLI formatting, saved tester JSON and durable reopen for exact0702 qualified run state.

use super::*;

#[test]
fn exact0702_cli_qualified_observation_reports_running_with_the_0702_marker() {
    let d = SimulatedDevice::with_config(SimulatorConfig {
        mask_version: 0x0702,
        ..SimulatorConfig::default()
    });
    d.preset_property(0, PID_MANUFACTURER_ID, &[0, 0x83]);
    d.preset_property(0, PID_HARDWARE_TYPE, &HARDWARE);
    d.preset_property(3, PID_PROGRAM_VERSION, &APPLICATION);
    d.preset_property(3, 1, &[0, 3]);
    d.preset_property(3, 6, &[1]);
    d.preset_property_description(3, 6, 0x37, [0, 0, 1, 0]);
    let mut p = plan();
    p.mask = MaskVersion(0x0702);
    let run = run_regular(&d, d.address(), &p);
    assert_eq!(run.exit, ExitCode::SUCCESS);
    assert!(d.memory_was_written());
    assert_eq!(restarts(&d), 1);
    let check = check_json(&run);
    assert_eq!(check["runtime"], "running");
    let m = &check["observation"]["reach"]["identity"];
    assert_eq!(m["runStateMask"], "0702");
    assert_eq!(
        m["runState"],
        json!({"status":"observed", "objectIndex":3, "state":"running"})
    );
    assert!(run.out.contains("run state: running"), "{}", run.out);
    let report = run.report.as_ref().unwrap();
    assert_eq!(report["program"]["mask"], "0702");
    assert_eq!(report["outcome"]["written"], "yes");
    assert_eq!(report["postDownloadCheck"], check);
    let row = reopened(&run);
    assert_eq!(row["state"], "finished");
    assert_eq!(row["downloadEvidence"]["written"], "yes");
    assert_eq!(row["downloadEvidence"]["postDownloadCheck"], check);
    assert_check_read_only(&d);
}

#[test]
fn exact0702_cli_unqualified_description_never_reclassifies_the_written_transfer() {
    for (raw, expected, words) in [
        (
            Some([0, 0, 0, 0]),
            json!({"status":"notQualified", "reason":"descriptionProblem"}),
            "unspecified description problem",
        ),
        (
            Some([2, 0, 1, 0]),
            json!({"status":"notQualified", "reason":"unsupportedDatatype", "pdt":2}),
            "PDT 2",
        ),
        (
            None,
            json!({"status":"unavailable", "step":"description", "cause":"noCorrelatedAnswer"}),
            "optional run-state description",
        ),
    ] {
        let d = SimulatedDevice::with_config(SimulatorConfig {
            mask_version: 0x0702,
            ..SimulatorConfig::default()
        });
        d.preset_property(0, PID_MANUFACTURER_ID, &[0, 0x83]);
        d.preset_property(0, PID_HARDWARE_TYPE, &HARDWARE);
        d.preset_property(3, PID_PROGRAM_VERSION, &APPLICATION);
        d.preset_property(3, 1, &[0, 3]);
        d.preset_property(3, 6, &[1]);
        if let Some(raw) = raw {
            d.preset_property_description(3, 6, 0x37, raw);
        }
        let mut p = plan();
        p.mask = MaskVersion(0x0702);
        let run = run_regular(&d, d.address(), &p);
        assert_eq!(run.exit, ExitCode::SUCCESS);
        assert!(d.memory_was_written());
        let check = check_json(&run);
        assert_eq!(check["runtime"], "unknown");
        let m = &check["observation"]["reach"]["identity"];
        assert_eq!(m["runStateMask"], "0702");
        assert_eq!(m["runState"], expected);
        assert!(run.out.contains(words), "{}", run.out);
        assert!(!run.out.contains("run state: running"));
        let report = run.report.as_ref().unwrap();
        assert_eq!(report["program"]["mask"], "0702");
        assert_eq!(report["outcome"]["written"], "yes");
        assert_eq!(report["postDownloadCheck"], check);
        let row = reopened(&run);
        assert_eq!(row["downloadEvidence"]["postDownloadCheck"], check);
        assert_check_read_only(&d);
    }
}
