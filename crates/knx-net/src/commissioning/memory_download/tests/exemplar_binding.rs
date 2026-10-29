//! A restore plan must not replay one device's backup onto a replacement with another serial.

use super::*;
use knx_core::commissioning::device_backup::restore_plan;
use knx_core::commissioning::device_backup::ExemplarEvidence;
use knx_core::commissioning::memory_download::ExemplarSerial;
use knx_core::commissioning::properties::PID_SERIAL_NUMBER;

use crate::cemi::{ApplicationService, Destination, Tpci};
use crate::client::{BusError, TunnelEvent};
use crate::commissioning::simulator::{PropertyReadFault, ReadFault};
use crate::management::ManagementTransport;
use knx_core::commissioning::mutation::TargetKind;
use std::cell::Cell;
use tokio::sync::broadcast;

const ORIGINAL_SERIAL: [u8; 6] = [0x00, 0x83, 0x12, 0x34, 0x56, 0x78];
const REPLACEMENT_SERIAL: [u8; 6] = [0x00, 0x83, 0x12, 0x34, 0x56, 0x79];

/// An old device of the plan's product and application that reports `serial`.
fn device_with_serial(serial: &[u8; 6]) -> SimulatedDevice {
    let device = old_device();
    device.preset_property(3, PID_PROGRAM_VERSION, &IDENTITY);
    device.preset_property(0, PID_SERIAL_NUMBER, serial);
    device
}

#[tokio::test]
async fn a_backup_of_one_serial_does_not_restore_onto_a_replacement_with_another() {
    let original = device_with_serial(&ORIGINAL_SERIAL);
    let mut kept = None;
    run_memory_download_with_backup(
        &mut session(&original, WriteScope::Download),
        &identified(plan()),
        |_| {},
        &mut |backup: &DeviceBackup| {
            kept = Some(backup.clone());
            Ok(())
        },
    )
    .await
    .expect("the download over the original runs");
    let backup = kept.expect("the original's backup was kept");

    // Same address, mask, manufacturer and application; only the serial differs.
    let replacement = device_with_serial(&REPLACEMENT_SERIAL);
    assert_eq!(replacement.address(), original.address());
    let before = replacement.memory(0x4000, 30);
    let restore = restore_plan(&identified(plan()), replacement.address(), &backup)
        .expect("the plan derives from the original's backup");

    let mut kept_again = 0_usize;
    let result = run_memory_download_with_backup(
        &mut session(&replacement, WriteScope::Download),
        &restore,
        |_| {},
        &mut |_: &DeviceBackup| {
            kept_again += 1;
            Ok(())
        },
    )
    .await;

    assert!(
        result.is_err(),
        "the restore must be refused on a device with another serial, but it ran"
    );
    assert_eq!(kept_again, 0, "no backup is kept before the refusal");
    assert!(
        memory_writes(&replacement).is_empty(),
        "no memory or load-record write reaches the replacement"
    );
    assert_eq!(replacement.memory(0x4000, 30), before);
    assert_eq!(
        replacement.load_state(ObjectIndex::new(1)),
        LoadState::Loaded
    );
}

const SERIAL_BYTES: [u8; 6] = ORIGINAL_SERIAL;

/// Whether any rendering of the original serial shows in `text`.
fn leaks(text: &str) -> bool {
    ["12 34 56", "1234", "18, 52, 86", "0x12", "123456"]
        .iter()
        .any(|needle| text.contains(needle))
}

fn serial_fault(from_read: u32, fault: ReadFault) -> SimulatorConfig {
    SimulatorConfig {
        property_read_fault: Some(PropertyReadFault {
            object_index: 0,
            property_id: PID_SERIAL_NUMBER,
            from_read,
            fault,
        }),
        ..SimulatorConfig::default()
    }
}

/// An old device of the plan's product and application; `serial` is what PID 11 answers.
fn device_holding_serial(serial: Option<&[u8]>, config: SimulatorConfig) -> SimulatedDevice {
    let device = device(config);
    device.preset_memory(0x4000, &[0x07, 0x11, 0x43]);
    device.preset_memory(0x4003, &(100..127).collect::<Vec<u8>>());
    device.preset_load_state(ObjectIndex::new(1), LoadState::Loaded);
    device.preset_property(3, PID_PROGRAM_VERSION, &IDENTITY);
    if let Some(serial) = serial {
        device.preset_property(0, PID_SERIAL_NUMBER, serial);
    }
    device
}

struct Taken {
    result: Result<MemoryDownloadReport, MemoryDownloadError>,
    kept: Vec<DeviceBackup>,
    progress: Vec<String>,
}

async fn take(device: &SimulatedDevice, plan: &MemoryDownloadPlan) -> Taken {
    let mut kept = Vec::new();
    let mut progress = Vec::new();
    let result = run_memory_download_with_backup(
        &mut session(device, WriteScope::Download),
        plan,
        |p| progress.push(format!("{p:?}")),
        &mut |backup: &DeviceBackup| {
            kept.push(backup.clone());
            Ok(())
        },
    )
    .await;
    Taken {
        result,
        kept,
        progress,
    }
}

fn serial_reads(device: &SimulatedDevice) -> Vec<usize> {
    device
        .seen()
        .iter()
        .enumerate()
        .filter(|(_, seen)| {
            matches!(
                seen,
                Seen::PropertyRead {
                    object_index: 0,
                    property_id: PID_SERIAL_NUMBER
                }
            )
        })
        .map(|(index, _)| index)
        .collect()
}

fn changes_anything(device: &SimulatedDevice) -> bool {
    device.seen().iter().any(|seen| {
        matches!(
            seen,
            Seen::MemoryWrite { .. } | Seen::PropertyWrite { .. } | Seen::Restart { .. }
        )
    })
}

fn recorded(backup: &DeviceBackup) -> [u8; 6] {
    match backup.exemplar_serial {
        ExemplarEvidence::Recorded(serial) => serial.octets(),
        ExemplarEvidence::NotRecorded => panic!("no serial recorded"),
    }
}

#[tokio::test]
async fn the_serial_is_read_once_after_the_application_and_before_load_states_and_memory() {
    let device = device_holding_serial(Some(&SERIAL_BYTES), SimulatorConfig::default());
    let taken = take(&device, &identified(plan())).await;
    taken.result.expect("the download runs");
    assert_eq!(recorded(&taken.kept[0]), SERIAL_BYTES);

    let seen = device.seen();
    let at = |wanted: &dyn Fn(&Seen) -> bool| seen.iter().position(wanted).expect("seen");
    let prior = at(&|s| {
        matches!(
            s,
            Seen::PropertyRead {
                object_index: 3,
                property_id: PID_PROGRAM_VERSION
            }
        )
    });
    // The prior-application read is also the plan's own compare; the backup's is the last one.
    let backup_prior = seen
        .iter()
        .rposition(|s| {
            matches!(
                s,
                Seen::PropertyRead {
                    object_index: 3,
                    property_id: PID_PROGRAM_VERSION
                }
            )
        })
        .unwrap();
    assert!(prior <= backup_prior);
    let serials = serial_reads(&device);
    assert_eq!(serials.len(), 1, "one read, no retry: {seen:?}");
    let first_memory_read = at(&|s| matches!(s, Seen::MemoryRead { .. }));
    assert!(backup_prior < serials[0], "after the PID 13 read");
    assert!(
        serials[0] < first_memory_read,
        "before load states and regions"
    );
    let first_write = at(&|s| matches!(s, Seen::MemoryWrite { .. }));
    assert!(serials[0] < first_write);
    // Strict request: object 0, PID 11, one element from start index 1.
    assert!(device
        .property_read_requests()
        .contains(&(0, PID_SERIAL_NUMBER, 1, 1)));
}

#[tokio::test]
async fn unbacked_runs_previews_and_comparisons_read_no_serial() {
    let device = device_holding_serial(Some(&SERIAL_BYTES), SimulatorConfig::default());
    run_memory_download(
        &mut session(&device, WriteScope::Download),
        &identified(plan()),
    )
    .await
    .expect("an unbacked download runs");
    assert!(
        serial_reads(&device).is_empty(),
        "no serial in an unbacked run"
    );

    let device = device_holding_serial(Some(&SERIAL_BYTES), SimulatorConfig::default());
    let mut read_only =
        ManagementSession::read_only(&device, device.address(), AuthorisationPlan::Skip, fast())
            .expect("a read-only session");
    let held = read_what_the_plan_overwrites(&mut read_only, &identified(plan()))
        .await
        .expect("a preview");
    assert_eq!(held.exemplar_serial, ExemplarEvidence::NotRecorded);
    assert!(serial_reads(&device).is_empty(), "no serial in a preview");

    let device = device_holding_serial(Some(&SERIAL_BYTES), SimulatorConfig::default());
    let target = knx_core::ContactableAddress::new(device.address()).expect("contactable");
    compare_with_plan(&device, target, fast(), &identified(plan()))
        .await
        .expect("a comparison");
    assert!(
        serial_reads(&device).is_empty(),
        "no serial in a comparison"
    );
}

#[tokio::test]
async fn an_unusable_serial_answer_still_downloads_but_the_backup_never_restores() {
    const SHORT: &[u8] = &[0x00, 0x83, 0x12, 0x34, 0x56];
    const LONG: &[u8] = &[0x00, 0x83, 0x12, 0x34, 0x56, 0x78, 0x9A];
    let cases: Vec<(&str, SimulatedDevice)> = vec![
        (
            "absent property",
            device_holding_serial(None, SimulatorConfig::default()),
        ),
        (
            "five octets",
            device_holding_serial(Some(&SERIAL_BYTES), serial_fault(1, ReadFault::Data(SHORT))),
        ),
        (
            "seven octets",
            device_holding_serial(Some(&SERIAL_BYTES), serial_fault(1, ReadFault::Data(LONG))),
        ),
        (
            "two elements",
            device_holding_serial(
                Some(&SERIAL_BYTES),
                serial_fault(1, ReadFault::ElementCount(2)),
            ),
        ),
        (
            "wrong start echoed",
            device_holding_serial(
                Some(&SERIAL_BYTES),
                serial_fault(1, ReadFault::EchoStart(2)),
            ),
        ),
        (
            "wrong object echoed",
            device_holding_serial(
                Some(&SERIAL_BYTES),
                serial_fault(1, ReadFault::EchoObject(1)),
            ),
        ),
        (
            "wrong property echoed",
            device_holding_serial(
                Some(&SERIAL_BYTES),
                serial_fault(1, ReadFault::EchoProperty(12)),
            ),
        ),
        (
            "acknowledged silence",
            device_holding_serial(Some(&SERIAL_BYTES), serial_fault(1, ReadFault::Unanswered)),
        ),
    ];
    for (name, device) in cases {
        let plan = identified(plan());
        let taken = take(&device, &plan).await;
        taken
            .result
            .as_ref()
            .unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_eq!(
            taken.kept[0].exemplar_serial,
            ExemplarEvidence::NotRecorded,
            "{name}"
        );
        assert!(
            device
                .property_read_requests()
                .iter()
                .filter(|r| (r.0, r.1) == (0, PID_SERIAL_NUMBER))
                .all(|r| *r == (0, PID_SERIAL_NUMBER, 1, 1)),
            "{name}: strict request"
        );
        assert_eq!(
            restore_plan(&plan, device.address(), &taken.kept[0]),
            Err(RestoreError::ExemplarNotRecorded),
            "{name}"
        );
    }
}

#[tokio::test]
async fn other_failures_of_the_serial_read_abort_before_keep_and_write() {
    for (name, config) in [(
        "connection torn down",
        serial_fault(1, ReadFault::Disconnect),
    )] {
        let device = device_holding_serial(Some(&SERIAL_BYTES), config);
        let taken = take(&device, &identified(plan())).await;
        let error = taken.result.expect_err(name);
        assert!(
            matches!(error, MemoryDownloadError::BackupRead { .. }),
            "{name}: {error}"
        );
        assert!(taken.kept.is_empty(), "{name}: keep must not run");
        assert!(!changes_anything(&device), "{name}: nothing written");
        assert_eq!(device.load_state(ObjectIndex::new(1)), LoadState::Loaded);
    }
}

/// Derives the restore plan from a backup of `original` and runs it on `replacement`.
async fn replay_on(
    replacement: &SimulatedDevice,
) -> (
    Result<MemoryDownloadReport, MemoryDownloadError>,
    usize,
    usize,
) {
    let original = device_holding_serial(Some(&ORIGINAL_SERIAL), SimulatorConfig::default());
    let backup = take(&original, &identified(plan())).await.kept.remove(0);
    let restore = restore_plan(&identified(plan()), replacement.address(), &backup).expect("plan");
    let guard = restore
        .steps
        .iter()
        .position(|s| matches!(s, MemoryDownloadStep::RequireExemplar(_)))
        .expect("the guard");
    let taken = take(replacement, &restore).await;
    (taken.result, taken.kept.len(), guard)
}

#[tokio::test]
async fn every_live_refusal_of_the_guard_stops_before_backup_keep_and_writes() {
    const SHORT: &[u8] = &[0x00, 0x83, 0x12, 0x34, 0x56];
    type Check = fn(&MemoryDownloadError, usize) -> bool;
    let cases: Vec<(&str, SimulatedDevice, Check)> = vec![
        (
            "another serial",
            device_holding_serial(Some(&REPLACEMENT_SERIAL), SimulatorConfig::default()),
            |e, guard| matches!(e, MemoryDownloadError::ExemplarMismatch { index } if *index == guard),
        ),
        (
            "no serial property",
            device_holding_serial(None, SimulatorConfig::default()),
            |e, _| {
                matches!(
                    e,
                    MemoryDownloadError::Session {
                        error: SessionError::PropertyRefused { .. },
                        ..
                    }
                )
            },
        ),
        (
            "five octets",
            device_holding_serial(
                Some(&ORIGINAL_SERIAL),
                serial_fault(1, ReadFault::Data(SHORT)),
            ),
            |e, _| {
                matches!(
                    e,
                    MemoryDownloadError::Session {
                        error: SessionError::MalformedProperty { .. },
                        ..
                    }
                )
            },
        ),
        (
            "two elements",
            device_holding_serial(
                Some(&ORIGINAL_SERIAL),
                serial_fault(1, ReadFault::ElementCount(2)),
            ),
            |e, _| {
                matches!(
                    e,
                    MemoryDownloadError::ExemplarElementCount {
                        in_backup: false,
                        got: 2,
                        ..
                    }
                )
            },
        ),
        (
            "wrong start echoed",
            device_holding_serial(
                Some(&ORIGINAL_SERIAL),
                serial_fault(1, ReadFault::EchoStart(2)),
            ),
            |e, _| {
                matches!(
                    e,
                    MemoryDownloadError::Session {
                        error: SessionError::NoAnswer { .. },
                        ..
                    }
                )
            },
        ),
        (
            "acknowledged silence",
            device_holding_serial(
                Some(&ORIGINAL_SERIAL),
                serial_fault(1, ReadFault::Unanswered),
            ),
            |e, _| {
                matches!(
                    e,
                    MemoryDownloadError::Session {
                        error: SessionError::NoAnswer { .. },
                        ..
                    }
                )
            },
        ),
        (
            "torn down connection keeps its own cause",
            device_holding_serial(
                Some(&ORIGINAL_SERIAL),
                serial_fault(1, ReadFault::Disconnect),
            ),
            |e, _| matches!(e, MemoryDownloadError::Session { .. }),
        ),
    ];
    for (name, replacement, expected) in cases {
        let before = replacement.memory(0x4000, 30);
        let (result, kept, guard) = replay_on(&replacement).await;
        let error = result.expect_err(name);
        assert!(expected(&error, guard), "{name}: {error:?}");
        assert_eq!(kept, 0, "{name}: no backup callback");
        assert!(!changes_anything(&replacement), "{name}: nothing written");
        assert_eq!(replacement.memory(0x4000, 30), before, "{name}");
        assert_eq!(
            replacement.load_state(ObjectIndex::new(1)),
            LoadState::Loaded
        );
        let shown = format!("{error} {error:?}");
        assert!(!leaks(&shown), "{name}: {shown}");
        assert!(!serial_reads(&replacement).is_empty(), "{name}");
    }
}

#[tokio::test]
async fn the_guard_holds_for_the_unbacked_and_observed_executors_too() {
    let original = device_holding_serial(Some(&ORIGINAL_SERIAL), SimulatorConfig::default());
    let backup = take(&original, &identified(plan())).await.kept.remove(0);
    let replacement = device_holding_serial(Some(&REPLACEMENT_SERIAL), SimulatorConfig::default());
    let restore = restore_plan(&identified(plan()), replacement.address(), &backup).unwrap();

    let error = run_memory_download(&mut session(&replacement, WriteScope::Download), &restore)
        .await
        .expect_err("unbacked");
    assert!(
        matches!(error, MemoryDownloadError::ExemplarMismatch { .. }),
        "{error:?}"
    );
    let mut progress = Vec::new();
    let error = run_memory_download_observed(
        &mut session(&replacement, WriteScope::Download),
        &restore,
        |p| progress.push(format!("{p:?}")),
    )
    .await
    .expect_err("observed");
    assert!(
        matches!(error, MemoryDownloadError::ExemplarMismatch { .. }),
        "{error:?}"
    );
    assert!(!changes_anything(&replacement));
    assert!(!leaks(&progress.join(" ")), "{progress:?}");
}

#[tokio::test]
async fn a_matching_device_restores_and_the_backup_of_the_restore_restores_again() {
    let device = device_holding_serial(Some(&ORIGINAL_SERIAL), SimulatorConfig::default());
    let before = device.memory(0x4000, 30);
    let plan = identified(plan());
    let first = take(&device, &plan).await;
    first.result.expect("the download runs");
    let after_download = device.memory(0x4000, 30);
    assert_ne!(after_download, before);

    let restore = restore_plan(&plan, device.address(), &first.kept[0]).expect("restorable");
    let reads_before = serial_reads(&device).len();
    let restored = take(&device, &restore).await;
    restored.result.expect("the matching restore runs");
    assert_eq!(device.memory(0x4000, 30), before, "the old octets are back");
    assert_eq!(
        serial_reads(&device).len() - reads_before,
        2,
        "the guard and the restore's own backup each observe the serial"
    );
    assert!(
        !leaks(&restored.progress.join(" ")),
        "{:?}",
        restored.progress
    );

    // The restore's own backup is of the post-download state, bound to the same serial.
    let second = &restored.kept[0];
    assert_eq!(recorded(second), ORIGINAL_SERIAL);
    let again = restore_plan(&plan, device.address(), second).expect("restorable again");
    take(&device, &again)
        .await
        .result
        .expect("the second restore runs");
    assert_eq!(device.memory(0x4000, 30), after_download);
}

#[tokio::test]
async fn a_second_observation_that_differs_or_vanishes_stops_before_keep_and_writes() {
    const OTHER: &[u8] = &REPLACEMENT_SERIAL;
    const SHORT: &[u8] = &[0x00, 0x83, 0x12, 0x34, 0x56];
    for (name, fault) in [
        ("differs", ReadFault::Data(OTHER)),
        ("malformed", ReadFault::Data(SHORT)),
        ("unanswered", ReadFault::Unanswered),
    ] {
        let original = device_holding_serial(Some(&ORIGINAL_SERIAL), SimulatorConfig::default());
        let backup = take(&original, &identified(plan())).await.kept.remove(0);
        // Read 1 (the guard) is truthful; read 2 (the restore's own backup) is not.
        let device = device_holding_serial(Some(&ORIGINAL_SERIAL), serial_fault(2, fault));
        let restore = restore_plan(&identified(plan()), device.address(), &backup).unwrap();
        let taken = take(&device, &restore).await;
        let error = taken.result.expect_err(name);
        match name {
            "differs" => assert!(
                matches!(error, MemoryDownloadError::BackupExemplarMismatch { .. }),
                "{name}: {error:?}"
            ),
            _ => assert!(
                matches!(error, MemoryDownloadError::BackupRead { .. }),
                "{name}: {error:?}"
            ),
        }
        assert!(taken.kept.is_empty(), "{name}: keep must not run");
        assert!(!changes_anything(&device), "{name}: nothing written");
        assert!(!leaks(&format!("{error} {error:?}")), "{name}");
    }
}

#[tokio::test]
async fn a_late_manual_guard_is_refused_before_any_traffic() {
    let device = device_holding_serial(Some(&ORIGINAL_SERIAL), SimulatorConfig::default());
    let mut late = identified(plan());
    let first_change = late
        .steps
        .iter()
        .position(MemoryDownloadStep::changes_device)
        .unwrap();
    late.steps.insert(
        first_change + 1,
        MemoryDownloadStep::RequireExemplar(ExemplarSerial::new(ORIGINAL_SERIAL)),
    );
    for result in [
        run_memory_download(&mut session(&device, WriteScope::Download), &late).await,
        take(&device, &late).await.result,
    ] {
        let error = result.expect_err("refused");
        assert!(
            matches!(error, MemoryDownloadError::CheckAfterWrite { .. }),
            "{error:?}"
        );
    }
    assert!(device.seen().is_empty(), "no frame was sent");
}

/// A count of 2 with six data octets is a wrong element count, not "two octets".
fn says_octets_for_a_count(text: &str) -> bool {
    text.contains("2 octet")
}

#[tokio::test]
async fn a_wrong_element_count_is_reported_as_a_count_never_as_octets() {
    // Live guard.
    let original = device_holding_serial(Some(&ORIGINAL_SERIAL), SimulatorConfig::default());
    let backup = take(&original, &identified(plan())).await.kept.remove(0);
    let replacement = device_holding_serial(
        Some(&ORIGINAL_SERIAL),
        serial_fault(1, ReadFault::ElementCount(2)),
    );
    let restore = restore_plan(&identified(plan()), replacement.address(), &backup).unwrap();
    let error = take(&replacement, &restore)
        .await
        .result
        .expect_err("refused");
    let shown = error.to_string();
    assert!(!says_octets_for_a_count(&shown), "live: {shown}");
    assert!(shown.contains("element count"), "live: {shown}");
    assert!(!leaks(&format!("{shown} {error:?}")));

    // The restore's own backup read (second observation).
    let device = device_holding_serial(
        Some(&ORIGINAL_SERIAL),
        serial_fault(2, ReadFault::ElementCount(2)),
    );
    let restore = restore_plan(&identified(plan()), device.address(), &backup).unwrap();
    let taken = take(&device, &restore).await;
    let error = taken.result.expect_err("refused");
    let shown = error.to_string();
    assert!(!says_octets_for_a_count(&shown), "backup: {shown}");
    assert!(shown.contains("element count"), "backup: {shown}");
    assert!(taken.kept.is_empty() && !changes_anything(&device));
}

#[tokio::test]
async fn the_stoppable_executor_and_a_task_only_restore_enforce_the_guard_too() {
    // A restore whose plan writes no segment data: only load records change the device.
    let mut task_only = identified(plan());
    task_only
        .steps
        .retain(|step| !matches!(step, MemoryDownloadStep::WriteMemory { .. }));
    let original = device_holding_serial(Some(&ORIGINAL_SERIAL), SimulatorConfig::default());
    let backup = take(&original, &task_only).await.kept.remove(0);
    assert!(backup.regions.is_empty());
    let restore = restore_plan(&task_only, original.address(), &backup).expect("guarded plan");

    let replacement = device_holding_serial(Some(&REPLACEMENT_SERIAL), SimulatorConfig::default());
    let stop = DownloadStop::new();
    let mut kept = 0;
    let error = run_memory_download_with_backup_stoppable(
        &mut session(&replacement, WriteScope::Download),
        &restore,
        |_| {},
        |_| {
            kept += 1;
            Ok(())
        },
        &stop,
    )
    .await
    .expect_err("refused");
    assert!(
        matches!(error, MemoryDownloadError::ExemplarMismatch { .. }),
        "{error:?}"
    );
    assert_eq!(kept, 0);
    assert!(!changes_anything(&replacement));
    let error = run_memory_download(&mut session(&replacement, WriteScope::Download), &restore)
        .await
        .expect_err("refused");
    assert!(
        matches!(error, MemoryDownloadError::ExemplarMismatch { .. }),
        "{error:?}"
    );
    assert!(!changes_anything(&replacement));
}

/// What the tap does to a read of the Device Object serial (and, for the stop, after PID 13).
#[derive(Clone, Copy)]
enum Tap {
    /// Send nothing: no acknowledge, so the connection is released.
    SwallowSerial,
    /// The transport refuses the frame.
    RefuseSerial,
    /// Request a stop right after the second (backup) PID 13 read has gone out.
    StopAfterBackupApplication,
}

struct Wire<'a> {
    device: &'a SimulatedDevice,
    tap: Tap,
    stop: &'a DownloadStop,
    application_reads: Cell<usize>,
}

impl ManagementTransport for Wire<'_> {
    fn assigned_address(&self) -> knx_core::IndividualAddress {
        ManagementTransport::assigned_address(self.device)
    }

    fn subscribe(&self) -> broadcast::Receiver<TunnelEvent> {
        ManagementTransport::subscribe(self.device)
    }

    async fn send_frame(
        &self,
        destination: Destination,
        transport: Tpci,
        service: ApplicationService,
    ) -> Result<(), BusError> {
        match (&service, self.tap) {
            (
                ApplicationService::PropertyValueRead {
                    object_index: 0,
                    property_id: PID_SERIAL_NUMBER,
                    ..
                },
                Tap::SwallowSerial,
            ) => return Ok(()),
            (
                ApplicationService::PropertyValueRead {
                    object_index: 0,
                    property_id: PID_SERIAL_NUMBER,
                    ..
                },
                Tap::RefuseSerial,
            ) => return Err(BusError::Timeout),
            _ => {}
        }
        let is_application_read = matches!(
            service,
            ApplicationService::PropertyValueRead {
                object_index: 3,
                property_id: PID_PROGRAM_VERSION,
                ..
            }
        );
        let result =
            ManagementTransport::send_frame(self.device, destination, transport, service).await;
        if is_application_read {
            self.application_reads.set(self.application_reads.get() + 1);
            if matches!(self.tap, Tap::StopAfterBackupApplication)
                && self.application_reads.get() == 2
            {
                // The plan's own compare was read 1; this is the backup's PID 13 read.
                self.stop.request();
            }
        }
        result
    }

    fn target_kind(&self) -> TargetKind {
        ManagementTransport::target_kind(self.device)
    }
}

fn wire_session<'a>(wire: &'a Wire<'a>) -> ManagementSession<'a, Wire<'a>> {
    let authorisation =
        WriteAuthorisation::for_simulator(wire.device.address(), WriteScope::Download)
            .expect("not excluded");
    ManagementSession::authorised(wire, AuthorisationPlan::Skip, fast(), authorisation)
        .expect("a simulator session")
}

#[tokio::test]
async fn m1_a_stop_after_the_backups_application_read_sends_no_serial_read_and_keeps_nothing() {
    let device = device_holding_serial(Some(&ORIGINAL_SERIAL), SimulatorConfig::default());
    let stop = DownloadStop::new();
    let wire = Wire {
        device: &device,
        tap: Tap::StopAfterBackupApplication,
        stop: &stop,
        application_reads: Cell::new(0),
    };
    let mut kept = 0;
    let error = run_memory_download_with_backup_stoppable(
        &mut wire_session(&wire),
        &identified(plan()),
        |_| {},
        |_| {
            kept += 1;
            Ok(())
        },
        &stop,
    )
    .await
    .expect_err("stopped");
    assert!(
        matches!(
            error,
            MemoryDownloadError::Stopped {
                intent_recorded: false,
                ..
            }
        ),
        "{error:?}"
    );
    assert_eq!(
        wire.application_reads.get(),
        2,
        "the stop came after the backup's PID 13 read"
    );
    assert!(serial_reads(&device).is_empty(), "no PID 11 frame");
    assert_eq!(kept, 0);
    assert!(!changes_anything(&device));
}

#[tokio::test]
async fn m2_a_torn_down_or_refused_serial_read_keeps_its_exact_original_cause() {
    // Live guard on a replacement, torn down by the device.
    let original = device_holding_serial(Some(&ORIGINAL_SERIAL), SimulatorConfig::default());
    let backup = take(&original, &identified(plan())).await.kept.remove(0);
    let torn = device_holding_serial(
        Some(&ORIGINAL_SERIAL),
        serial_fault(1, ReadFault::Disconnect),
    );
    let restore = restore_plan(&identified(plan()), torn.address(), &backup).unwrap();
    let error = take(&torn, &restore).await.result.expect_err("torn down");
    assert!(
        matches!(
            error,
            MemoryDownloadError::Session {
                error: SessionError::ConnectionLost { .. },
                ..
            }
        ),
        "live torn-down: {error:?}"
    );

    // Two other transport failure kinds, on an ordinary backed-up download.
    for (name, tap) in [
        ("swallowed", Tap::SwallowSerial),
        ("refused", Tap::RefuseSerial),
    ] {
        let device = device_holding_serial(Some(&ORIGINAL_SERIAL), SimulatorConfig::default());
        let stop = DownloadStop::new();
        let wire = Wire {
            device: &device,
            tap,
            stop: &stop,
            application_reads: Cell::new(0),
        };
        let mut kept = 0;
        let error = run_memory_download_with_backup(
            &mut wire_session(&wire),
            &identified(plan()),
            |_| {},
            &mut |_: &DeviceBackup| {
                kept += 1;
                Ok(())
            },
        )
        .await
        .expect_err(name);
        match (name, &error) {
            (
                "swallowed",
                MemoryDownloadError::BackupRead {
                    error: SessionError::ConnectionReleased { .. },
                    ..
                },
            ) => {}
            (
                "refused",
                MemoryDownloadError::BackupRead {
                    error: SessionError::Transport(_),
                    ..
                },
            ) => {}
            _ => panic!("{name}: {error:?}"),
        }
        assert_eq!(kept, 0, "{name}");
        assert!(!changes_anything(&device), "{name}");
    }
}

#[tokio::test]
async fn m3_all_zero_and_all_ff_serials_are_recorded_by_a_download_but_never_restore() {
    for placeholder in [[0x00_u8; 6], [0xFF; 6]] {
        let device = device_holding_serial(Some(&placeholder), SimulatorConfig::default());
        let plan = identified(plan());
        let taken = take(&device, &plan).await;
        taken.result.expect("the download is admitted");
        assert_eq!(
            taken.kept[0].exemplar_serial,
            ExemplarEvidence::Recorded(ExemplarSerial::new(placeholder))
        );
        assert_eq!(
            restore_plan(&plan, device.address(), &taken.kept[0]),
            Err(RestoreError::ExemplarNotBindable)
        );
    }
}

#[tokio::test]
async fn m4_a_second_observation_with_count_zero_is_a_backup_refusal_before_keep() {
    let original = device_holding_serial(Some(&ORIGINAL_SERIAL), SimulatorConfig::default());
    let backup = take(&original, &identified(plan())).await.kept.remove(0);
    let device = device_holding_serial(
        Some(&ORIGINAL_SERIAL),
        serial_fault(2, ReadFault::ElementCount(0)),
    );
    let restore = restore_plan(&identified(plan()), device.address(), &backup).unwrap();
    let taken = take(&device, &restore).await;
    let error = taken.result.expect_err("refused");
    assert!(
        matches!(
            error,
            MemoryDownloadError::BackupRead {
                error: SessionError::PropertyRefused { .. },
                ..
            }
        ),
        "{error:?}"
    );
    assert!(taken.kept.is_empty());
    assert!(!changes_anything(&device));
}
