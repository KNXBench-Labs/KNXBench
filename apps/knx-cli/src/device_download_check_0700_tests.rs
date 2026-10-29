//! Verifies CLI formatting, saved tester JSON and durable reopen for exact0700 unavailable facts.

use super::*;

#[test]
fn exact0700_cli_unavailable_property_facts_never_reclassify_the_written_transfer() {
    for (raw, state, expected, words) in [
        (
            Some([0, 0, 0, 0]),
            1,
            json!({"status":"notQualified", "reason":"descriptionProblem"}),
            "unspecified description problem",
        ),
        (
            Some([0x40, 0, 0, 0]),
            1,
            json!({"status":"notQualified", "reason":"descriptionProblem"}),
            "unspecified description problem",
        ),
        (
            Some([0, 0, 0, 1]),
            1,
            json!({"status":"notQualified", "reason":"indeterminateMetadata"}),
            "indeterminate metadata",
        ),
        (
            Some([0, 0x10, 1, 0]),
            1,
            json!({"status":"notQualified", "reason":"unqualifiedElements"}),
            "unqualified maximum elements",
        ),
        (
            Some([2, 0, 1, 0]),
            1,
            json!({"status":"notQualified", "reason":"unsupportedDatatype", "pdt":2}),
            "PDT 2",
        ),
        (
            None,
            1,
            json!({"status":"unavailable", "step":"description", "cause":"noCorrelatedAnswer"}),
            "optional run-state description",
        ),
        (
            Some([0, 0, 1, 0]),
            0x81,
            json!({"status":"undefinedValue"}),
            "undefined in the specification",
        ),
    ] {
        let d = SimulatedDevice::with_config(SimulatorConfig {
            mask_version: 0x0700,
            ..SimulatorConfig::default()
        });
        d.preset_property(0, PID_MANUFACTURER_ID, &[0, 0x83]);
        d.preset_property(0, PID_HARDWARE_TYPE, &HARDWARE);
        d.preset_property(3, PID_PROGRAM_VERSION, &APPLICATION);
        d.preset_property(3, 1, &[0, 3]);
        d.preset_property(3, 6, &[state]);
        if let Some(raw) = raw {
            d.preset_property_description(3, 6, 0x37, raw);
        }
        let mut p = plan();
        p.mask = MaskVersion(0x0700);
        let run = run_regular(&d, d.address(), &p);
        assert_eq!(run.exit, ExitCode::SUCCESS);
        assert!(d.memory_was_written());
        assert_eq!(restarts(&d), 1);
        let check = check_json(&run);
        assert_eq!(check["runtime"], "unknown");
        let m = &check["observation"]["reach"]["identity"];
        assert_eq!(m["runStateMask"], "0700");
        assert_eq!(m["runState"], expected);
        assert!(run.out.contains(words), "{}", run.out);
        assert!(!run.out.contains("run state: running"));
        let report = run.report.as_ref().unwrap();
        assert_eq!(report["program"]["mask"], "0700");
        assert_eq!(report["outcome"]["written"], "yes");
        assert_eq!(report["postDownloadCheck"], check);
        let row = reopened(&run);
        assert_eq!(row["state"], "finished");
        assert_eq!(row["downloadEvidence"]["written"], "yes");
        assert_eq!(row["downloadEvidence"]["postDownloadCheck"], check);
        assert_check_read_only(&d);
    }
}
