//! A stoppable download stops before its first change and never after it, on one simulator.

use std::cell::Cell;

use super::*;
use knx_core::commissioning::mutation::TargetKind;
use tokio::sync::broadcast;

use crate::cemi::{ApplicationService, Destination, Tpci};
use crate::client::{BusError, TunnelEvent};

/// When the [`Trip`] fires: on the first frame this says yes to.
type Trigger = fn(&ApplicationService, Tpci) -> bool;

/// Forwards to the simulator. On the first frame `when` accepts, it asks
/// `stop` to stop (the frame is then in flight) and, if `refuse`, fails it.
struct Trip<'a> {
    device: &'a SimulatedDevice,
    stop: &'a DownloadStop,
    when: Trigger,
    refuse: bool,
    answer: Cell<Option<StopRequest>>,
}

impl<'a> Trip<'a> {
    fn new(device: &'a SimulatedDevice, stop: &'a DownloadStop, when: Trigger) -> Self {
        Self {
            device,
            stop,
            when,
            refuse: false,
            answer: Cell::new(None),
        }
    }
}

impl ManagementTransport for Trip<'_> {
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
        if self.answer.get().is_none() && (self.when)(&service, transport) {
            self.answer.set(Some(self.stop.request()));
            if self.refuse {
                return Err(BusError::Timeout);
            }
        }
        ManagementTransport::send_frame(self.device, destination, transport, service).await
    }

    fn target_kind(&self) -> TargetKind {
        ManagementTransport::target_kind(self.device)
    }
}

fn trip_session<'a>(trip: &'a Trip<'a>) -> ManagementSession<'a, Trip<'a>> {
    let authorisation =
        WriteAuthorisation::for_simulator(trip.device.address(), WriteScope::Download)
            .expect("not excluded");
    ManagementSession::authorised(trip, AuthorisationPlan::Skip, fast(), authorisation)
        .expect("a simulator session")
}

fn nothing(_: &DeviceBackup) -> Result<(), String> {
    Ok(())
}

fn mutating(seen: &[Seen]) -> Vec<&Seen> {
    seen.iter()
        .filter(|seen| {
            matches!(
                seen,
                Seen::MemoryWrite { .. } | Seen::PropertyWrite { .. } | Seen::Restart { .. }
            )
        })
        .collect()
}

fn snapshot(device: &SimulatedDevice) -> (Vec<Option<u8>>, LoadState) {
    (
        device.memory(0x4000, 30),
        device.load_state(ObjectIndex::new(1)),
    )
}

/// The stoppable run on a fresh old device with a fresh token and `keep`.
async fn plain(device: &SimulatedDevice, stop: &DownloadStop) -> Result<(), MemoryDownloadError> {
    let mut session = session(device, WriteScope::Download);
    run_memory_download_with_backup_stoppable(&mut session, &plan(), |_| {}, nothing, stop)
        .await
        .map(|_| ())
}

fn is_data_write(service: &ApplicationService, _: Tpci) -> bool {
    matches!(service, ApplicationService::MemoryWrite { address, .. } if *address == 0x4000)
}

fn is_connect(_: &ApplicationService, transport: Tpci) -> bool {
    transport == Tpci::Connect
}

fn reads_first_backup_chunk(service: &ApplicationService, _: Tpci) -> bool {
    matches!(
        service,
        ApplicationService::MemoryRead {
            address: 0x4003,
            ..
        }
    )
}

fn reads_last_backup_chunk(service: &ApplicationService, _: Tpci) -> bool {
    matches!(service, ApplicationService::MemoryRead { address, .. } if *address > 0x4003 + 12 && *address < 0x4100)
}

#[tokio::test]
async fn a_stop_before_the_run_sends_no_frame_at_all() {
    let device = old_device();
    let stop = DownloadStop::new();
    assert_eq!(stop.request(), StopRequest::WillStop);
    let mut kept = 0;
    let mut session = session(&device, WriteScope::Download);
    let error = run_memory_download_with_backup_stoppable(
        &mut session,
        &plan(),
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
                before_step: 0,
                intent_recorded: false
            }
        ),
        "{error}"
    );
    assert_eq!(
        device.seen(),
        vec![],
        "S0 sends nothing, not even a disconnect"
    );
    assert_eq!(kept, 0);
    assert_eq!(stop.request(), StopRequest::Ended);
    assert!(!stop.mutation_committed());
}

#[tokio::test]
async fn a_stop_after_a_read_only_step_disconnects_and_writes_nothing() {
    let device = old_device();
    let old = snapshot(&device);
    let stop = DownloadStop::new();
    let mut session = session(&device, WriteScope::Download);
    let error = run_memory_download_with_backup_stoppable(
        &mut session,
        &plan(),
        |progress| {
            if matches!(&progress, Progress::StepDone(done) if done.index == 0) {
                assert_eq!(stop.request(), StopRequest::WillStop);
            }
        },
        nothing,
        &stop,
    )
    .await
    .expect_err("stopped");
    assert!(
        matches!(
            error,
            MemoryDownloadError::Stopped {
                before_step: 1,
                intent_recorded: false
            }
        ),
        "{error}"
    );
    let seen = device.seen();
    assert_eq!(mutating(&seen), Vec::<&Seen>::new());
    assert_eq!(seen.last(), Some(&Seen::Disconnect));
    assert!(session.connection().is_none());
    assert_eq!(snapshot(&device), old);
}

#[tokio::test]
async fn a_stop_between_backup_chunks_never_reaches_keep_or_a_write() {
    let device = old_device();
    let stop = DownloadStop::new();
    let trip = Trip::new(&device, &stop, reads_first_backup_chunk);
    let mut session = trip_session(&trip);
    let mut kept = 0;
    let error = run_memory_download_with_backup_stoppable(
        &mut session,
        &plan(),
        |_| {},
        |_| {
            kept += 1;
            Ok(())
        },
        &stop,
    )
    .await
    .expect_err("stopped");
    assert_eq!(trip.answer.get(), Some(StopRequest::WillStop));
    assert!(
        matches!(
            error,
            MemoryDownloadError::Stopped {
                intent_recorded: false,
                ..
            }
        ),
        "{error}"
    );
    assert_eq!(kept, 0, "a partial backup is never handed over");
    let seen = device.seen();
    assert_eq!(mutating(&seen), Vec::<&Seen>::new());
    assert_eq!(seen.last(), Some(&Seen::Disconnect));
    assert!(
        !memory_reads(&device)
            .iter()
            .any(|(address, _)| (0x4003 + 12..0x4100).contains(address)),
        "no region read after the chunk that was in flight: {:?}",
        memory_reads(&device)
    );
}

#[tokio::test]
async fn a_stop_after_the_whole_backup_but_before_keep_does_not_call_keep() {
    let device = old_device();
    let stop = DownloadStop::new();
    let trip = Trip::new(&device, &stop, reads_last_backup_chunk);
    let mut session = trip_session(&trip);
    let mut kept = 0;
    let error = run_memory_download_with_backup_stoppable(
        &mut session,
        &plan(),
        |_| {},
        |_| {
            kept += 1;
            Ok(())
        },
        &stop,
    )
    .await
    .expect_err("stopped");
    assert_eq!(trip.answer.get(), Some(StopRequest::WillStop));
    assert!(
        matches!(
            error,
            MemoryDownloadError::Stopped {
                intent_recorded: false,
                ..
            }
        ),
        "{error}"
    );
    assert_eq!(kept, 0);
    assert_eq!(mutating(&device.seen()), Vec::<&Seen>::new());
}

#[tokio::test]
async fn a_stop_inside_keep_after_it_is_durable_wins_the_commit() {
    let device = old_device();
    let old = snapshot(&device);
    let stop = DownloadStop::new();
    let mut kept = Vec::new();
    let mut session = session(&device, WriteScope::Download);
    let error = run_memory_download_with_backup_stoppable(
        &mut session,
        &plan(),
        |_| {},
        |backup| {
            kept.push(backup.clone());
            assert_eq!(stop.request(), StopRequest::WillStop);
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
                intent_recorded: true,
                ..
            }
        ),
        "{error}"
    );
    assert_eq!(kept.len(), 1, "the actual backup was kept");
    assert_eq!(mutating(&device.seen()), Vec::<&Seen>::new());
    assert_eq!(snapshot(&device), old);
    assert_eq!(device.seen().last(), Some(&Seen::Disconnect));
    assert!(!stop.mutation_committed());
    assert_eq!(locked_device_hint(&error, None, false), None);
}

#[tokio::test]
async fn a_stop_requested_when_the_first_change_starts_is_too_late_and_changes_nothing() {
    let reference = old_device();
    plain(&reference, &DownloadStop::new()).await.expect("runs");

    let device = old_device();
    let stop = DownloadStop::new();
    let first_change = plan()
        .steps
        .iter()
        .position(MemoryDownloadStep::changes_device)
        .unwrap();
    let answer = Cell::new(None);
    let mut session = session(&device, WriteScope::Download);
    run_memory_download_with_backup_stoppable(
        &mut session,
        &plan(),
        |progress| {
            if matches!(&progress, Progress::StepStarted { index, .. } if *index == first_change) {
                answer.set(Some(stop.request()));
            }
        },
        nothing,
        &stop,
    )
    .await
    .expect("the committed run finishes");
    assert_eq!(answer.get(), Some(StopRequest::TooLate));
    assert_eq!(
        device.seen(),
        reference.seen(),
        "same schedule as unstopped"
    );
    assert!(stop.mutation_committed());
    assert_eq!(stop.request(), StopRequest::Ended);
}

#[tokio::test]
async fn a_stop_while_a_write_awaits_its_acknowledgement_is_too_late() {
    let reference = old_device();
    plain(&reference, &DownloadStop::new()).await.expect("runs");

    let device = old_device();
    let stop = DownloadStop::new();
    let trip = Trip::new(&device, &stop, is_data_write);
    let mut session = trip_session(&trip);
    run_memory_download_with_backup_stoppable(&mut session, &plan(), |_| {}, nothing, &stop)
        .await
        .expect("finishes");
    assert_eq!(trip.answer.get(), Some(StopRequest::TooLate));
    assert_eq!(device.seen(), reference.seen());
}

#[tokio::test]
async fn a_lost_acknowledgement_after_a_late_stop_is_the_original_failure_without_retry() {
    let config = || SimulatorConfig {
        interrupt_at: Some(Interruption::MemoryWriteAt(0x4003)),
        ..SimulatorConfig::default()
    };
    let make = || {
        let device = device(config());
        device.preset_memory(0x4000, &[0x07, 0x11, 0x43]);
        device.preset_load_state(ObjectIndex::new(1), LoadState::Loaded);
        device
    };
    let reference = make();
    plain(&reference, &DownloadStop::new())
        .await
        .expect_err("the connection is lost");

    let device = make();
    let stop = DownloadStop::new();
    let trip = Trip::new(&device, &stop, |service, _| {
        matches!(
            service,
            ApplicationService::MemoryWrite {
                address: 0x4003,
                ..
            }
        )
    });
    let mut session = trip_session(&trip);
    let error =
        run_memory_download_with_backup_stoppable(&mut session, &plan(), |_| {}, nothing, &stop)
            .await
            .expect_err("the connection is lost");
    assert_eq!(trip.answer.get(), Some(StopRequest::TooLate));
    assert!(
        matches!(error, MemoryDownloadError::Session { .. }),
        "{error}"
    );
    assert_eq!(device.seen(), reference.seen(), "no resend, no restore");
}

#[tokio::test]
async fn a_genuine_failure_in_flight_beats_the_stop() {
    let device = old_device();
    let stop = DownloadStop::new();
    let trip = Trip::new(&device, &stop, is_connect);
    let mut session = trip_session(&trip);
    let mut wrong = plan();
    wrong.manufacturer = 0x0084;
    let error =
        run_memory_download_with_backup_stoppable(&mut session, &wrong, |_| {}, nothing, &stop)
            .await
            .expect_err("mismatch");
    assert_eq!(trip.answer.get(), Some(StopRequest::WillStop));
    assert!(
        matches!(error, MemoryDownloadError::ManufacturerMismatch { .. }),
        "{error}"
    );
}

#[tokio::test]
async fn a_failed_backup_read_beats_the_stop() {
    let device = old_device();
    let stop = DownloadStop::new();
    let mut trip = Trip::new(&device, &stop, reads_first_backup_chunk);
    trip.refuse = true;
    let mut session = trip_session(&trip);
    let error =
        run_memory_download_with_backup_stoppable(&mut session, &plan(), |_| {}, nothing, &stop)
            .await
            .expect_err("read failed");
    assert_eq!(trip.answer.get(), Some(StopRequest::WillStop));
    assert!(
        matches!(error, MemoryDownloadError::BackupRead { .. }),
        "{error}"
    );
}

#[tokio::test]
async fn a_plan_without_a_change_finishes_when_the_stop_comes_late_and_ends_the_token() {
    let device = old_device();
    let mut read_only = plan();
    read_only.steps.retain(|step| !step.changes_device());
    let stop = DownloadStop::new();
    let mut session = session(&device, WriteScope::Download);
    let report = run_memory_download_with_backup_stoppable(
        &mut session,
        &read_only,
        |progress| {
            if matches!(&progress, Progress::StepStarted { step, .. } if step == "disconnect") {
                assert_eq!(stop.request(), StopRequest::WillStop);
            }
        },
        nothing,
        &stop,
    )
    .await
    .expect("nothing to stop");
    assert!(report.backup.is_none());
    assert_eq!(mutating(&device.seen()), Vec::<&Seen>::new());
    assert!(!stop.mutation_committed());
    assert_eq!(stop.request(), StopRequest::Ended);
}

#[tokio::test]
async fn a_session_without_download_scope_is_refused_before_any_frame() {
    let device = old_device();
    let stop = DownloadStop::new();
    let mut read_only =
        ManagementSession::read_only(&device, device.address(), AuthorisationPlan::Skip, fast())
            .expect("read only");
    let error =
        run_memory_download_with_backup_stoppable(&mut read_only, &plan(), |_| {}, nothing, &stop)
            .await
            .expect_err("refused");
    assert!(
        matches!(
            error,
            MemoryDownloadError::StopRefused(StopRefusal::NotDownloadScope)
        ),
        "{error}"
    );
    let mut unload = session(&device, WriteScope::Unload);
    let error =
        run_memory_download_with_backup_stoppable(&mut unload, &plan(), |_| {}, nothing, &stop)
            .await
            .expect_err("refused");
    assert!(
        matches!(
            error,
            MemoryDownloadError::StopRefused(StopRefusal::NotDownloadScope)
        ),
        "{error}"
    );
    assert_eq!(device.seen(), vec![]);
    assert_eq!(stop.request(), StopRequest::WillStop, "the token is unused");
}

#[tokio::test]
async fn a_mask_that_does_not_load_through_memory_is_refused_before_any_frame() {
    let device = old_device();
    let stop = DownloadStop::new();
    let mut other = plan();
    other.mask = MaskVersion(0x0020);
    let mut session = session(&device, WriteScope::Download);
    let error =
        run_memory_download_with_backup_stoppable(&mut session, &other, |_| {}, nothing, &stop)
            .await
            .expect_err("refused");
    assert!(
        matches!(
            error,
            MemoryDownloadError::StopRefused(StopRefusal::NotMemoryMapped)
        ),
        "{error}"
    );
    assert_eq!(device.seen(), vec![]);
}

#[tokio::test]
async fn a_shape_the_executor_refuses_is_still_refused_before_any_frame() {
    let device = old_device();
    let mut bad = plan();
    bad.steps.remove(0);
    let mut session = session(&device, WriteScope::Download);
    let error = run_memory_download_with_backup_stoppable(
        &mut session,
        &bad,
        |_| {},
        nothing,
        &DownloadStop::new(),
    )
    .await
    .expect_err("refused");
    assert!(
        matches!(error, MemoryDownloadError::DoesNotConnectFirst),
        "{error}"
    );
    assert_eq!(device.seen(), vec![]);
}

#[tokio::test]
async fn a_session_that_is_already_connected_is_refused_and_left_connected() {
    let device = old_device();
    let stop = DownloadStop::new();
    let mut session = session(&device, WriteScope::Download);
    session.connect().await.expect("connects");
    let error =
        run_memory_download_with_backup_stoppable(&mut session, &plan(), |_| {}, nothing, &stop)
            .await
            .expect_err("refused");
    assert!(
        matches!(
            error,
            MemoryDownloadError::StopRefused(StopRefusal::AlreadyConnected)
        ),
        "{error}"
    );
    assert!(
        session.connection().is_some(),
        "the caller's connection stays"
    );
    assert!(
        !device.seen().contains(&Seen::Disconnect),
        "not even a disconnect"
    );
}

#[tokio::test]
async fn a_token_serves_one_run_whatever_the_first_one_did() {
    // Finished, failed before any change, and stopped before connecting.
    for case in ["finished", "failed", "stopped"] {
        let device = old_device();
        let stop = DownloadStop::new();
        match case {
            "finished" => plain(&device, &stop).await.expect("runs"),
            "failed" => {
                let mut wrong = plan();
                wrong.manufacturer = 0x0084;
                let mut session = session(&device, WriteScope::Download);
                run_memory_download_with_backup_stoppable(
                    &mut session,
                    &wrong,
                    |_| {},
                    nothing,
                    &stop,
                )
                .await
                .expect_err("mismatch");
            }
            _ => {
                stop.request();
                plain(&device, &stop).await.expect_err("stopped");
            }
        }
        let frames = device.seen().len();
        let error = plain(&device, &stop).await.expect_err("reuse");
        assert!(
            matches!(
                error,
                MemoryDownloadError::StopRefused(StopRefusal::TokenUsed)
            ),
            "{case}: {error}"
        );
        assert_eq!(
            device.seen().len(),
            frames,
            "{case}: the reuse sent nothing"
        );
    }
}

#[tokio::test]
async fn two_runs_polled_together_cannot_claim_one_token() {
    let first = old_device();
    let second = old_device();
    let stop = DownloadStop::new();
    let (a, b) = tokio::join!(plain(&first, &stop), plain(&second, &stop));
    let refused = |result: &Result<(), MemoryDownloadError>| {
        matches!(
            result,
            Err(MemoryDownloadError::StopRefused(StopRefusal::TokenUsed))
        )
    };
    assert_eq!([refused(&a), refused(&b)].iter().filter(|r| **r).count(), 1);
    assert_eq!([a.is_ok(), b.is_ok()].iter().filter(|r| **r).count(), 1);
    let idle = if a.is_ok() { &second } else { &first };
    assert_eq!(idle.seen(), vec![], "the loser sent nothing");
}

#[tokio::test]
async fn a_prewrite_stop_leaves_the_device_so_a_fresh_run_succeeds() {
    let device = old_device();
    let old = snapshot(&device);
    let stop = DownloadStop::new();
    let trip = Trip::new(&device, &stop, is_connect);
    let mut stopped = trip_session(&trip);
    let error =
        run_memory_download_with_backup_stoppable(&mut stopped, &plan(), |_| {}, nothing, &stop)
            .await
            .expect_err("stopped");
    assert!(
        matches!(error, MemoryDownloadError::Stopped { .. }),
        "{error}"
    );
    assert_eq!(snapshot(&device), old);

    plain(&device, &DownloadStop::new())
        .await
        .expect("a new run");
    assert_eq!(device.load_state(ObjectIndex::new(1)), LoadState::Loaded);
}

#[test]
fn the_token_state_machine_answers_each_request_by_state() {
    let fresh = DownloadStop::new();
    assert_eq!(fresh.request(), StopRequest::WillStop);
    assert_eq!(
        fresh.request(),
        StopRequest::WillStop,
        "idempotent while pending"
    );

    let committed = DownloadStop::new();
    let claim = committed.claim().expect("fresh");
    assert!(committed.claim().is_none(), "one claim only");
    assert!(!claim.stop_requested());
    assert!(claim.commit());
    assert_eq!(committed.request(), StopRequest::TooLate);
    assert_eq!(committed.request(), StopRequest::TooLate);
    drop(claim);
    assert_eq!(committed.request(), StopRequest::Ended);
    assert!(
        committed.mutation_committed(),
        "the evidence outlives the run"
    );

    let stopped = DownloadStop::new();
    let claim = stopped.claim().expect("fresh");
    assert_eq!(stopped.request(), StopRequest::WillStop);
    assert!(claim.stop_requested());
    assert!(!claim.commit(), "an accepted stop wins");
    assert!(!stopped.mutation_committed());
    drop(claim);
    assert_eq!(stopped.request(), StopRequest::Ended, "even a stopped run");
    assert!(stopped.claim().is_none());
}

#[test]
fn a_claim_dropped_by_an_unwind_ends_the_token() {
    let stop = DownloadStop::new();
    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _claim = stop.claim().expect("fresh");
        panic!("the run dies");
    }));
    assert!(outcome.is_err());
    assert_eq!(stop.request(), StopRequest::Ended);
    assert!(stop.claim().is_none());
}
