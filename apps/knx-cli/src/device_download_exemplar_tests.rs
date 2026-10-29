//! CLI restore through the real execute path: bound to the backed-up serial, synthetic device.

use super::*;
use knx_core::commissioning::device_backup::{restore_plan, ExemplarEvidence};
use knx_core::commissioning::memory_download::{source_procedure_descriptions, ExemplarSerial};
use knx_core::commissioning::properties::PID_SERIAL_NUMBER;

const ORIGINAL: [u8; 6] = [0x00, 0x83, 0x12, 0x34, 0x56, 0x78];
const OTHER: [u8; 6] = [0x00, 0x83, 0x12, 0x34, 0x56, 0x79];
const PROGRAM: &str = "synthetic-application";

fn exemplar_device(serial: &[u8; 6]) -> SimulatedDevice {
    let device = device(APPLICATION, SimulatorConfig::default());
    device.preset_property(0, PID_SERIAL_NUMBER, serial);
    for machine in MemoryLoadStateMachine::ALL {
        device.preset_load_state(ObjectIndex::new(machine.type_number()), LoadState::Loaded);
    }
    device
}

/// [`run_regular_in`], keeping the backup under `PROGRAM` like a real download.
fn run_with(dir: tempfile::TempDir, transport: &SimulatedDevice, plan: &MemoryDownloadPlan) -> Run {
    let target = transport.address();
    let history = dir.path().join("history.sqlite");
    let mut activity = start(&dir, target);
    let report_path = dir.path().join("report.json");
    let backups = dir.path().join("backups");
    let mut out = Vec::new();
    let outcome = runtime().block_on(execute_regular_download(
        transport,
        WriteAuthorisation::for_simulator(target, WriteScope::Download).unwrap(),
        download_keying(plan.mask, None, KeySource::None),
        fast(),
        DownloadRun {
            plan,
            backup: &BackupTarget {
                dir: &backups,
                application: PROGRAM,
                partial: None,
                taken: "2026-10-25T07:00:00Z".into(),
            },
        },
        &mut activity,
        &mut out,
    ));
    let exit = finish_regular_download(
        &outcome,
        true,
        activity,
        Some((report_path.as_path(), &report_template())),
        &mut out,
    );
    let report = std::fs::read_to_string(&report_path)
        .ok()
        .map(|text| serde_json::from_str(&text).unwrap());
    Run {
        outcome,
        exit,
        out: String::from_utf8(out).unwrap(),
        history,
        report,
        _dir: dir,
    }
}

fn backup_files(run: &Run) -> Vec<PathBuf> {
    match std::fs::read_dir(run._dir.path().join("backups")) {
        Ok(entries) => entries.map(|entry| entry.unwrap().path()).collect(),
        Err(_) => Vec::new(),
    }
}

fn leaks(text: &str) -> bool {
    [
        "008312345678",
        "00831234567",
        "12 34 56",
        "1234",
        "18, 52, 86",
    ]
    .iter()
    .any(|needle| text.to_uppercase().contains(&needle.to_uppercase()))
}

fn mutating(device: &SimulatedDevice) -> bool {
    device.seen().iter().any(|seen| {
        matches!(
            seen,
            Seen::MemoryWrite { .. } | Seen::PropertyWrite { .. } | Seen::Restart { .. }
        )
    })
}

#[test]
fn a_restore_onto_another_serial_fails_before_any_write_or_backup_file() {
    let original = exemplar_device(&ORIGINAL);
    let taken = run_with(tempfile::tempdir().unwrap(), &original, &plan());
    assert_eq!(taken.exit, ExitCode::SUCCESS, "{}", taken.out);
    let files = backup_files(&taken);
    assert_eq!(files.len(), 1, "{files:?}");
    let stored = knx_app::device_backup::read_backup(&files[0]).unwrap();
    assert_eq!(
        stored.backup.exemplar_serial,
        ExemplarEvidence::Recorded(ExemplarSerial::new(ORIGINAL))
    );
    let restore = restore_plan(&plan(), original.address(), &stored.backup).expect("guarded");

    let replacement = exemplar_device(&OTHER);
    let run = run_with(tempfile::tempdir().unwrap(), &replacement, &restore);
    assert_eq!(run.exit, ExitCode::FAILURE, "{}", run.out);
    assert!(run.out.contains("FAILED"), "{}", run.out);
    let failure = serde_json::to_value(run.outcome.failure.as_ref().expect("a cause")).unwrap();
    assert_eq!(failure["code"], "propertyMismatch", "{failure}");
    assert_eq!(failure["category"], "identityMismatch", "{failure}");
    assert_eq!(failure["step"]["kind"], "compareProperty", "{failure}");
    assert!(run.out.contains("written to the device: no"), "{}", run.out);
    assert!(!run.out.contains("partially"), "{}", run.out);
    // The durable history row and the report say the same, payload-free.
    let row = reopened(&run);
    assert_eq!(row["state"], "failed", "{row}");
    assert_eq!(row["downloadEvidence"]["written"], "no", "{row}");
    assert_eq!(row["downloadEvidence"]["failure"], failure, "{row}");
    let report = run.report.as_ref().expect("a report");
    assert_eq!(report["failure"], failure, "{report}");
    assert_eq!(report["outcome"]["written"], "no", "{report}");
    assert!(backup_files(&run).is_empty(), "no backup file");
    assert!(!mutating(&replacement), "{:?}", replacement.seen());
    for shown in [run.out.clone(), row.to_string(), report.to_string()] {
        assert!(!leaks(&shown), "serial leaked into: {shown}");
    }
}

#[test]
fn a_restore_onto_the_same_serial_runs_and_leaves_a_backup_that_restores_again() {
    let original = exemplar_device(&ORIGINAL);
    let taken = run_with(tempfile::tempdir().unwrap(), &original, &plan());
    let first = knx_app::device_backup::read_backup(&backup_files(&taken)[0]).unwrap();
    let restore = restore_plan(&plan(), original.address(), &first.backup).expect("guarded");

    let same = exemplar_device(&ORIGINAL);
    let run = run_with(tempfile::tempdir().unwrap(), &same, &restore);
    assert_eq!(run.exit, ExitCode::SUCCESS, "{}", run.out);
    assert!(mutating(&same), "the restore wrote");
    let files = backup_files(&run);
    assert_eq!(files.len(), 1, "{files:?}");
    let second = knx_app::device_backup::read_backup(&files[0]).unwrap();
    let text = std::fs::read_to_string(&files[0]).unwrap();
    assert!(text.contains("\"format\": 3"), "{text}");
    assert!(
        text.contains("\"exemplar_serial\": \"008312345678\""),
        "{text}"
    );
    assert_eq!(
        second.backup.exemplar_serial,
        ExemplarEvidence::Recorded(ExemplarSerial::new(ORIGINAL))
    );
    // The guard is not a manufacturer step: the stored procedure equals the source one.
    assert_eq!(second.plan_steps, source_procedure_descriptions(&restore));
    assert_eq!(second.plan_steps, first.plan_steps);
    restore_plan(&plan(), same.address(), &second.backup).expect("restorable again");
}

/// A restore plan from a backup of `ORIGINAL`, and a replacement whose serial reads go wrong.
fn failing_restore(
    from_read: u32,
    fault: knx_net::commissioning::simulator::ReadFault,
) -> (Run, SimulatedDevice) {
    let original = exemplar_device(&ORIGINAL);
    let taken = run_with(tempfile::tempdir().unwrap(), &original, &plan());
    let stored = knx_app::device_backup::read_backup(&backup_files(&taken)[0]).unwrap();
    let restore = restore_plan(&plan(), original.address(), &stored.backup).expect("guarded");
    let device = SimulatedDevice::with_config(SimulatorConfig {
        mask_version: 0x0701,
        property_read_fault: Some(knx_net::commissioning::simulator::PropertyReadFault {
            object_index: 0,
            property_id: PID_SERIAL_NUMBER,
            from_read,
            fault,
        }),
        ..SimulatorConfig::default()
    });
    device.preset_property(0, PID_MANUFACTURER_ID, &[0x00, 0x83]);
    device.preset_property(0, PID_HARDWARE_TYPE, &HARDWARE);
    device.preset_property(3, PID_PROGRAM_VERSION, &APPLICATION);
    device.preset_property(0, PID_SERIAL_NUMBER, &ORIGINAL);
    for machine in MemoryLoadStateMachine::ALL {
        device.preset_load_state(ObjectIndex::new(machine.type_number()), LoadState::Loaded);
    }
    let run = run_with(tempfile::tempdir().unwrap(), &device, &restore);
    (run, device)
}

fn assert_composed(run: &Run, device: &SimulatedDevice, code: &str, category: &str, phase: &str) {
    assert_eq!(run.exit, ExitCode::FAILURE, "{}", run.out);
    let failure = serde_json::to_value(run.outcome.failure.as_ref().expect("a cause")).unwrap();
    assert_eq!(failure["code"], code, "{failure}");
    assert_eq!(failure["category"], category, "{failure}");
    assert_eq!(failure["phase"], phase, "{failure}");
    let row = reopened(run);
    assert_eq!(row["downloadEvidence"]["failure"], failure, "{row}");
    assert_eq!(row["downloadEvidence"]["written"], "no", "{row}");
    let report = run.report.as_ref().expect("a report");
    assert_eq!(report["failure"], failure, "{report}");
    assert!(!run.out.contains("without its cause"), "{}", run.out);
    assert!(backup_files(run).is_empty(), "no backup file");
    assert!(!mutating(device));
    for shown in [run.out.clone(), row.to_string(), report.to_string()] {
        assert!(!leaks(&shown), "serial leaked into: {shown}");
    }
}

#[test]
fn a_second_observation_that_differs_composes_a_payload_free_history_and_report() {
    use knx_net::commissioning::simulator::ReadFault;
    let (run, device) = failing_restore(2, ReadFault::Data(&OTHER));
    assert_composed(
        &run,
        &device,
        "propertyMismatch",
        "identityMismatch",
        "backupBeforeStep",
    );
}

#[test]
fn a_wrong_serial_element_count_composes_a_malformed_property_history_and_report() {
    use knx_net::commissioning::simulator::ReadFault;
    let (run, device) = failing_restore(1, ReadFault::ElementCount(2));
    assert_composed(
        &run,
        &device,
        "malformedProperty",
        "malformedResponse",
        "inStep",
    );
    let failed = run
        .out
        .lines()
        .find(|l| l.contains("FAILED"))
        .expect("a FAILED line");
    assert!(failed.contains("element count 2"), "{failed}");
    assert!(!failed.contains("octet"), "{failed}");
}

#[test]
fn a_second_observation_with_a_wrong_element_count_keeps_its_cause_in_history_and_report() {
    use knx_net::commissioning::simulator::ReadFault;
    let (run, device) = failing_restore(2, ReadFault::ElementCount(2));
    assert_composed(
        &run,
        &device,
        "malformedProperty",
        "malformedResponse",
        "backupBeforeStep",
    );
    let failed = run
        .out
        .lines()
        .find(|l| l.contains("FAILED"))
        .expect("a FAILED line");
    assert!(failed.contains("element count 2"), "{failed}");
    assert!(!failed.contains("octet"), "{failed}");
}
