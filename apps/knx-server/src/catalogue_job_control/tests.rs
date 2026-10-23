//! Deterministic admission, clock, replay, stop, and fault controls for the private job kernel.

use super::*;

#[test]
fn receiving_expiry_and_upload_working_origin_use_distinct_exact_tickets() {
    let clock = ManualClock::new(Duration::ZERO);
    let abandoned =
        Control::new_receiving(identity(), binding(), clock.clone(), 3_600_000).unwrap();
    assert_eq!(abandoned.snapshot().unwrap().phase, Phase::Receiving);
    clock.set(Duration::from_millis(3_600_000));
    assert!(matches!(
        abandoned.expire(abandoned.ticket().unwrap()),
        Ok(Expiry::Stop(CancelOutcome::Accepted, _))
    ));
    assert_eq!(
        abandoned.snapshot().unwrap().stop_cause,
        Some(Cause::ReceivingTimeout)
    );
    assert_eq!(abandoned.checkpoint(), Ok(Err(Stop::CancelAccepted)));

    let clock = ManualClock::new(Duration::ZERO);
    let uploaded = Control::new_receiving(identity(), binding(), clock.clone(), 3_600_000).unwrap();
    let receiving = uploaded.ticket().unwrap();
    clock.set(Duration::from_millis(3_599_999));
    let deadline = 3_599_999u64.checked_add(7_200_000).unwrap();
    let working = uploaded.begin_preview(deadline).unwrap();
    assert_eq!(working.lease.kind, LeaseKind::Working);
    assert_eq!(working.lease.deadline_tick_ms, deadline);
    clock.set(Duration::from_millis(3_600_000));
    assert_eq!(
        uploaded.expire(receiving),
        Ok(Expiry::Inert(working)),
        "delayed Receiving timer must not expire the new worker"
    );
}

#[test]
fn joint_start_repeated_apply_and_stop_contenders_have_only_serial_outcomes() {
    let (control, _) = ready_fixture();
    let before = control.snapshot().unwrap();
    let control = Arc::new(control);
    let start = Arc::new(std::sync::Barrier::new(5));
    let spawn_apply = || {
        let control = Arc::clone(&control);
        let start = Arc::clone(&start);
        std::thread::spawn(move || {
            start.wait();
            control.apply(identity(), command())
        })
    };
    let spawn_stop = || {
        let control = Arc::clone(&control);
        let start = Arc::clone(&start);
        std::thread::spawn(move || {
            start.wait();
            control.request_stop(identity(), Cause::ServerStop)
        })
    };
    let apply_one = spawn_apply();
    let apply_two = spawn_apply();
    let stop_one = spawn_stop();
    let stop_two = spawn_stop();
    start.wait();
    let apply_results = [apply_one.join(), apply_two.join()];
    let stop_results = [stop_one.join(), stop_two.join()];
    // All four workers are joined before checking any outcome.
    let applies = apply_results.map(Result::unwrap);
    let stops = stop_results.map(Result::unwrap);
    assert_eq!(
        stops
            .iter()
            .filter(|r| **r == Ok(CancelOutcome::Accepted))
            .count(),
        1
    );
    assert_eq!(
        stops
            .iter()
            .filter(|r| **r == Ok(CancelOutcome::AlreadyAccepted))
            .count(),
        1
    );
    let admitted = applies
        .iter()
        .filter(|r| matches!(r, Ok(Apply::AcceptedOnce(_))))
        .count();
    let replayed = applies
        .iter()
        .filter(|r| matches!(r, Ok(Apply::Replay(_))))
        .count();
    let refused = applies
        .iter()
        .filter(|r| **r == Err(Error::ServerStopping))
        .count();
    assert!(matches!(
        (admitted, replayed, refused),
        (0, 0, 2) | (1, 1, 0)
    ));
    let mut expected = before;
    expected.phase = Phase::Settling;
    expected.revision += 1 + admitted as u64;
    expected.generation += 1 + admitted as u64;
    expected.consent = Consent::Spent;
    expected.stop_cause = Some(Cause::ServerStop);
    expected.admission_closed = true;
    expected.gate = PublicationState::CancelAccepted;
    if admitted == 1 {
        expected.accepted = Some(command());
        expected.lease = Lease {
            kind: LeaseKind::Working,
            deadline: Duration::from_secs(7200),
            deadline_tick_ms: 7_200_000,
        };
        let mut accepted = expected;
        accepted.phase = Phase::WaitingForWriter;
        accepted.revision -= 1;
        accepted.generation -= 1;
        accepted.stop_cause = None;
        accepted.admission_closed = false;
        accepted.gate = PublicationState::Open;
        for result in applies {
            match result {
                Ok(Apply::AcceptedOnce(view)) => assert_eq!(view, accepted),
                Ok(Apply::Replay(view)) => assert!(view == accepted || view == expected),
                _ => panic!("nonserial apply outcome"),
            }
        }
    }
    assert_eq!(control.snapshot(), Ok(expected));
    assert_eq!(control.checkpoint(), Ok(Err(Stop::CancelAccepted)));
}

#[test]
fn injected_native_lease_failure_uses_same_locked_settlement_without_fabrication() {
    type Kernel = Control<SyntheticEvidence, ManualClock>;
    for kind in [LeaseKind::Ready, LeaseKind::Working] {
        let (control, _) = ready_fixture();
        let before = control.snapshot().unwrap();
        let result = {
            let mut s = control.state.lock().unwrap();
            // Native addition cannot overflow in a healthy tick-representable state.
            // Inject it directly at the real preflight helper, under the real lock.
            Kernel::replacement_lease(&mut s, Duration::MAX, kind)
        };
        let mut expected = before;
        expected.phase = Phase::Settling;
        expected.revision += 1;
        expected.generation += 1;
        expected.consent = Consent::Spent;
        expected.stop_cause = Some(Cause::CounterExhausted);
        expected.admission_closed = true;
        expected.gate = PublicationState::CancelAccepted;
        assert_eq!(result, Err(Error::CounterExhausted));
        assert_eq!(control.snapshot(), Ok(expected));
    }
}

#[test]
fn final_representable_reserved_stop_is_idempotent_without_revision_reuse() {
    let (control, _) = ready_fixture();
    {
        let mut s = control.state.lock().unwrap();
        s.view.revision = u64::MAX - 1;
        s.view.generation = u64::MAX - 1;
    }
    let before = control.snapshot().unwrap();
    let mut expected = before;
    expected.phase = Phase::Settling;
    expected.revision = u64::MAX;
    expected.generation = u64::MAX;
    expected.consent = Consent::Spent;
    expected.stop_cause = Some(Cause::CounterExhausted);
    expected.admission_closed = true;
    expected.gate = PublicationState::CancelAccepted;
    assert_eq!(
        control.apply(identity(), command()),
        Err(Error::CounterExhausted)
    );
    assert_eq!(control.snapshot(), Ok(expected));
    assert_eq!(
        control.request_stop(identity(), Cause::ServerStop),
        Ok(CancelOutcome::AlreadyAccepted)
    );
    assert_eq!(
        control.expire(control.ticket().unwrap()),
        Ok(Expiry::Inert(expected))
    );
    assert_eq!(control.snapshot(), Ok(expected));
}

#[test]
fn simultaneous_faults_follow_apply_precedence_and_never_report_false_clean() {
    for case in 0..5 {
        let (control, clock) = ready_fixture();
        let mut bad = command();
        bad.confirmed = false;
        bad.digest = [0; 32];
        {
            let mut s = control.state.lock().unwrap();
            s.view.revision = if case >= 3 { u64::MAX } else { u64::MAX - 65 };
        }
        match case {
            0 | 3 => clock.set(Duration::from_secs(900)),
            1 | 4 => *clock.0.lock().unwrap() = Err(()),
            _ => clock.set(Duration::from_secs(899)),
        }
        let before = control.snapshot().unwrap();
        let error = match case {
            0 => Error::ConsentExpired,
            1 | 2 => Error::CounterExhausted,
            _ => Error::Unavailable,
        };
        assert_eq!(control.apply(identity(), bad), Err(error));
        if case >= 3 {
            assert_eq!(control.snapshot(), Err(Error::Unavailable));
            let s = control.state.lock().unwrap();
            assert_eq!(s.view, before);
            assert_eq!(s.gate.state(), PublicationState::Open);
        } else {
            let mut expected = before;
            expected.phase = Phase::Settling;
            expected.revision += 1;
            expected.generation += 1;
            expected.consent = Consent::Spent;
            expected.stop_cause = Some(if case == 0 {
                Cause::ReadyExpired
            } else {
                Cause::CounterExhausted
            });
            expected.admission_closed = true;
            expected.gate = PublicationState::CancelAccepted;
            assert_eq!(control.snapshot(), Ok(expected));
        }
    }
    // A retained first cause precedes even a failed clock and malformed command.
    let (control, clock) = ready_fixture();
    control.request_stop(identity(), Cause::ServerStop).unwrap();
    let stopped = control.snapshot().unwrap();
    *clock.0.lock().unwrap() = Err(());
    let mut bad = command();
    bad.confirmed = false;
    assert_eq!(control.apply(identity(), bad), Err(Error::ServerStopping));
    assert_eq!(control.snapshot(), Ok(stopped));
}

#[test]
fn submillisecond_before_deadline_admits_without_rounding_to_display_tick() {
    let (control, clock) = fixture(Duration::from_nanos(123));
    control
        .enter_ready(control.ticket().unwrap(), evidence())
        .unwrap();
    clock.set(Duration::new(900, 122));
    let before = control.snapshot().unwrap();
    let mut expected = before;
    expected.phase = Phase::WaitingForWriter;
    expected.revision = 3;
    expected.generation = 3;
    expected.consent = Consent::Spent;
    expected.accepted = Some(command());
    expected.lease = Lease {
        kind: LeaseKind::Working,
        deadline: Duration::new(8100, 122),
        deadline_tick_ms: 8_100_000,
    };
    assert_eq!(before.lease.deadline_tick_ms, 900_000);
    assert_eq!(
        control.apply(identity(), command()),
        Ok(Apply::AcceptedOnce(expected))
    );
    assert_eq!(control.snapshot(), Ok(expected));
}

#[test]
fn poison_refuses_every_kernel_access_without_recovery() {
    let (control, _) = ready_fixture();
    let control = Arc::new(control);
    let ticket = control.ticket().unwrap();
    let copy = Arc::clone(&control);
    let worker = std::thread::spawn(move || {
        let _guard = copy.state.lock().unwrap();
        panic!("intentional control poison");
    });
    let joined = worker.join();
    assert!(joined.is_err());
    assert!(control.state.is_poisoned());
    assert_eq!(control.snapshot(), Err(Error::Unavailable));
    assert_eq!(control.ticket(), Err(Error::Unavailable));
    assert_eq!(
        control.apply(identity(), command()),
        Err(Error::Unavailable)
    );
    assert_eq!(
        control.enter_ready(ticket, evidence()),
        Err(Error::Unavailable)
    );
    assert_eq!(control.expire(ticket), Err(Error::Unavailable));
    assert_eq!(
        control.request_stop(identity(), Cause::ServerStop),
        Err(Error::Unavailable)
    );
    assert_eq!(control.checkpoint(), Err(Error::Unavailable));
    assert_eq!(control.claim_test(), Err(Error::Unavailable));
    assert!(control.state.is_poisoned());
}

#[test]
fn initialization_clock_and_native_or_tick_overflow_return_no_control() {
    let samples = [
        Err(()),
        Ok(Duration::MAX),
        Ok(Duration::from_millis(u64::MAX - 7_200_000 + 1)),
    ];
    for sample in samples {
        let clock = ManualClock(Arc::new(Mutex::new(sample)));
        let result = Control::new_test(identity(), binding(), clock);
        let error = match result {
            Err(error) => error,
            Ok(_) => panic!("invalid initial time constructed a control"),
        };
        assert_eq!(error, Error::Unavailable);
    }
}

#[test]
fn test_constructor_refuses_zero_and_sentinel_serial_without_object() {
    for serial in [0, u64::MAX] {
        let mut id = identity();
        id.serial = serial;
        let result = Control::new_test(id, binding(), ManualClock::new(Duration::ZERO));
        let error = match result {
            Err(error) => error,
            Ok(_) => panic!("invalid serial constructed a control"),
        };
        assert_eq!(error, Error::InvalidHandle);
    }
}

#[test]
fn native_addition_overflow_is_checked_and_fault_uses_reserved_unavailability() {
    type Kernel = Control<SyntheticEvidence, ManualClock>;
    // This branch is unreachable from a healthy display-representable lease.
    // Pin native addition itself independently, then inject the damaged lease
    // to verify the exceptional refusal never invents a replacement deadline.
    assert_eq!(Kernel::lease(Duration::MAX, LeaseKind::Working), None);
    assert_eq!(Kernel::lease(Duration::MAX, LeaseKind::Ready), None);
    for operation in 0..3 {
        let (control, clock) = ready_fixture();
        let mut old = control.snapshot().unwrap();
        {
            let mut s = control.state.lock().unwrap();
            s.view.lease.deadline = Duration::MAX;
            s.view.revision = u64::MAX;
            old.lease.deadline = Duration::MAX;
            old.revision = u64::MAX;
        }
        let ticket = control.ticket().unwrap();
        // No valid reserved increment exists for the clock fault.
        *clock.0.lock().unwrap() = Err(());
        let result = match operation {
            0 => control.apply(identity(), command()).map(|_| ()),
            1 => control.expire(ticket).map(|_| ()),
            _ => control
                .request_stop(identity(), Cause::CounterExhausted)
                .map(|_| ()),
        };
        assert_eq!(result, Err(Error::Unavailable));
        assert_eq!(control.snapshot(), Err(Error::Unavailable));
        let s = control.state.lock().unwrap();
        assert_eq!(s.view, old);
        assert_eq!(s.gate.state(), PublicationState::Open);
    }
}

#[test]
fn new_lease_tick_overflow_refuses_before_command_fields_or_consent() {
    // New Working deadline: largest representable ms is accepted exactly;
    // one additional native millisecond refuses without consuming the command.
    for overflow in [false, true] {
        let now = Duration::from_millis(u64::MAX - 7_200_000 + u64::from(overflow));
        let (control, clock) = fixture(Duration::from_millis(u64::MAX - 7_200_000));
        control
            .enter_ready(control.ticket().unwrap(), evidence())
            .unwrap();
        clock.set(now);
        let before = control.snapshot().unwrap();
        let result = control.apply(identity(), command());
        if overflow {
            assert_eq!(result, Err(Error::CounterExhausted));
            let mut expected = before;
            expected.phase = Phase::Settling;
            expected.revision += 1;
            expected.generation += 1;
            expected.consent = Consent::Spent;
            expected.stop_cause = Some(Cause::CounterExhausted);
            expected.admission_closed = true;
            expected.gate = PublicationState::CancelAccepted;
            assert_eq!(control.snapshot(), Ok(expected));
        } else {
            let after = control.snapshot().unwrap();
            assert_eq!(result, Ok(Apply::AcceptedOnce(after)));
            assert_eq!(after.lease.deadline_tick_ms, u64::MAX);
            assert_eq!(after.lease.deadline, Duration::from_millis(u64::MAX));
        }
    }
}

#[test]
fn clock_failure_backward_and_tick_overflow_settle_without_new_lease() {
    for operation in 0..3 {
        for fault in 0..3 {
            let (control, clock) = fixture(Duration::from_secs(10));
            if operation != 0 {
                control
                    .enter_ready(control.ticket().unwrap(), evidence())
                    .unwrap();
            }
            let ticket = control.ticket().unwrap();
            let before = control.snapshot().unwrap();
            *clock.0.lock().unwrap() = match fault {
                0 => Err(()),
                1 => Ok(Duration::new(9, 999_999_999)),
                _ => Ok(Duration::from_millis(u64::MAX) + Duration::from_millis(1)),
            };
            let result = match operation {
                0 => control.enter_ready(ticket, evidence()).map(|_| ()),
                1 => control.apply(identity(), command()).map(|_| ()),
                _ => control.expire(ticket).map(|_| ()),
            };
            assert_eq!(result, Err(Error::CounterExhausted));
            let mut expected = before;
            expected.phase = Phase::Settling;
            expected.revision += 1;
            expected.generation += 1;
            expected.consent = Consent::Spent;
            expected.stop_cause = Some(Cause::CounterExhausted);
            expected.admission_closed = true;
            expected.gate = PublicationState::CancelAccepted;
            assert_eq!(control.snapshot(), Ok(expected));
            assert_eq!(control.checkpoint(), Ok(Err(Stop::CancelAccepted)));
        }
    }
}

#[test]
fn unrepresentable_reserved_increment_makes_every_operation_unavailable_atomically() {
    for claimed in [false, true] {
        for revision_counter in [true, false] {
            let (control, _) = ready_fixture();
            if claimed {
                control.apply(identity(), command()).unwrap();
                control.claim_test().unwrap();
            }
            {
                let mut s = control.state.lock().unwrap();
                if revision_counter {
                    s.view.revision = u64::MAX;
                } else {
                    s.view.generation = u64::MAX;
                }
            }
            let before = control.state.lock().unwrap().view;
            let ticket = control.ticket().unwrap();
            assert_eq!(
                control.request_stop(identity(), Cause::ServerStop),
                Err(Error::Unavailable)
            );
            assert_eq!(control.snapshot(), Err(Error::Unavailable));
            assert_eq!(
                control.apply(identity(), command()),
                Err(Error::Unavailable)
            );
            assert_eq!(
                control.enter_ready(ticket, evidence()),
                Err(Error::Unavailable)
            );
            assert_eq!(control.expire(ticket), Err(Error::Unavailable));
            assert_eq!(
                control.request_stop(identity(), Cause::UserCancel),
                Err(Error::Unavailable)
            );
            assert_eq!(control.checkpoint(), Err(Error::Unavailable));
            let s = control.state.lock().unwrap();
            assert!(s.unavailable);
            assert_eq!(s.view, before);
            assert_eq!(s.gate.state(), before.gate);
        }
    }
}

#[test]
fn ordinary_next_counter_at_reserve_boundary_settles_instead_of_accepting() {
    // Pinned independently: exactly 64 reserved increments; equality is refused.
    let threshold = u64::MAX - 64;
    for ready_entry in [true, false] {
        for revision_counter in [true, false] {
            for current in [threshold - 2, threshold - 1, threshold] {
                let (control, _) = if ready_entry {
                    fixture(Duration::ZERO)
                } else {
                    ready_fixture()
                };
                {
                    let mut s = control.state.lock().unwrap();
                    if revision_counter {
                        s.view.revision = current;
                    } else {
                        s.view.generation = current;
                    }
                }
                let before = control.snapshot().unwrap();
                let result = if ready_entry {
                    control
                        .enter_ready(control.ticket().unwrap(), evidence())
                        .map(|_| ())
                } else {
                    control.apply(identity(), command()).map(|_| ())
                };
                let after = control.snapshot().unwrap();
                if current == threshold - 2 {
                    assert_eq!(result, Ok(()));
                    assert_eq!(
                        after.phase,
                        if ready_entry {
                            Phase::Ready
                        } else {
                            Phase::WaitingForWriter
                        }
                    );
                    assert_eq!(after.revision, before.revision + 1);
                    assert_eq!(after.generation, before.generation + 1);
                } else {
                    assert_eq!(result, Err(Error::CounterExhausted));
                    let mut expected = before;
                    expected.phase = Phase::Settling;
                    expected.revision += 1;
                    expected.generation += 1;
                    expected.consent = Consent::Spent;
                    expected.stop_cause = Some(Cause::CounterExhausted);
                    expected.admission_closed = true;
                    expected.gate = PublicationState::CancelAccepted;
                    assert_eq!(after, expected);
                    assert_eq!(after.accepted, None);
                    assert_eq!(
                        control.request_stop(identity(), Cause::ServerStop),
                        Ok(CancelOutcome::AlreadyAccepted)
                    );
                    assert_eq!(control.snapshot(), Ok(expected));
                }
            }
        }
    }
}

#[test]
fn claimed_real_gate_makes_repeated_stop_too_late_without_stop_cause() {
    let (control, _) = ready_fixture();
    control.apply(identity(), command()).unwrap();
    assert_eq!(control.claim_test(), Ok(true));
    let mut expected = control.snapshot().unwrap();
    assert_eq!(expected.phase, Phase::Committing);
    assert_eq!(expected.gate, PublicationState::CommitClaimed);
    expected.admission_closed = true;
    expected.revision += 1;
    expected.generation += 1;
    assert_eq!(
        control.request_stop(identity(), Cause::ServerStop),
        Ok(CancelOutcome::TooLate)
    );
    assert_eq!(control.snapshot(), Ok(expected));
    assert_eq!(
        control.request_stop(identity(), Cause::UserCancel),
        Ok(CancelOutcome::TooLate)
    );
    assert_eq!(control.snapshot(), Ok(expected));
    assert_eq!(control.checkpoint(), Ok(Ok(())));
    let (cancelled, _) = ready_fixture();
    cancelled
        .request_stop(identity(), Cause::UserCancel)
        .unwrap();
    let frozen = cancelled.snapshot().unwrap();
    assert_eq!(cancelled.claim_test(), Ok(false));
    assert_eq!(cancelled.snapshot(), Ok(frozen));
}

#[test]
fn both_ordered_apply_stop_schedules_share_one_control_and_gate() {
    for stop_first in [true, false] {
        let (control, _) = ready_fixture();
        let control = Arc::new(control);
        let first = Arc::clone(&control);
        let second = Arc::clone(&control);
        let (tx, rx) = std::sync::mpsc::channel();
        let stop = std::thread::spawn(move || {
            if !stop_first {
                rx.recv().unwrap();
            }
            first.request_stop(identity(), Cause::ServerStop)
        });
        // For stop-first, join the stop worker before releasing apply. For apply-first,
        // the stop worker is blocked until apply has returned under the same lock.
        let (stop_result, apply_result) = if stop_first {
            let s = stop.join();
            let apply = std::thread::spawn(move || second.apply(identity(), command()));
            (s, apply.join())
        } else {
            let apply = std::thread::spawn(move || {
                let result = second.apply(identity(), command());
                tx.send(()).unwrap();
                result
            });
            let a = apply.join();
            let s = stop.join();
            (s, a)
        };
        assert_eq!(stop_result.unwrap(), Ok(CancelOutcome::Accepted));
        let a = apply_result.unwrap();
        let final_state = control.snapshot().unwrap();
        if stop_first {
            assert_eq!(a, Err(Error::ServerStopping));
            assert_eq!(final_state.revision, 3);
            assert_eq!(final_state.accepted, None);
        } else {
            assert!(matches!(a, Ok(Apply::AcceptedOnce(_))));
            assert_eq!(final_state.revision, 4);
            assert_eq!(final_state.accepted, Some(command()));
            assert_eq!(
                control.apply(identity(), command()),
                Ok(Apply::Replay(final_state))
            );
        }
        assert_eq!(final_state.generation, final_state.revision);
        assert_eq!(final_state.phase, Phase::Settling);
        assert_eq!(final_state.gate, PublicationState::CancelAccepted);
        assert_eq!(final_state.stop_cause, Some(Cause::ServerStop));
        assert_eq!(final_state.consent, Consent::Spent);
        assert!(final_state.admission_closed);
        assert_eq!(control.checkpoint(), Ok(Err(Stop::CancelAccepted)));
    }
}

#[derive(Clone)]
struct LockWitnessClock {
    time: ManualClock,
    owner: Arc<std::sync::OnceLock<std::sync::Weak<Control<SyntheticEvidence, LockWitnessClock>>>>,
    observed_locked: Arc<Mutex<Vec<bool>>>,
}
impl Clock for LockWitnessClock {
    fn sample(&self) -> Result<Duration, ()> {
        if let Some(owner) = self.owner.get().and_then(std::sync::Weak::upgrade) {
            self.observed_locked
                .lock()
                .unwrap()
                .push(owner.state.try_lock().is_err());
        }
        self.time.sample()
    }
}
#[test]
fn uncontended_apply_samples_clock_while_owning_actual_control_mutex() {
    let clock = LockWitnessClock {
        time: ManualClock::new(Duration::ZERO),
        owner: Arc::new(std::sync::OnceLock::new()),
        observed_locked: Arc::new(Mutex::new(Vec::new())),
    };
    let control = Arc::new(
        Control::new_test(identity(), binding(), clock.clone())
            .ok()
            .unwrap(),
    );
    control
        .enter_ready(control.ticket().unwrap(), evidence())
        .unwrap();
    assert!(clock.owner.set(Arc::downgrade(&control)).is_ok());
    // This fixture has no workers or retained state guard: only this apply can hold it.
    assert!(!control.state.is_poisoned());
    assert!(control.state.try_lock().is_ok());
    let result = control.apply(identity(), command());
    assert_eq!(
        *clock.observed_locked.lock().unwrap(),
        [true],
        "uncontended apply must own the actual control mutex when sampling"
    );
    assert!(matches!(result, Ok(Apply::AcceptedOnce(_))));
}

#[test]
fn apply_expires_when_clock_reaches_deadline_before_mutex_release() {
    let clock = LockWitnessClock {
        time: ManualClock::new(Duration::ZERO),
        owner: Arc::new(std::sync::OnceLock::new()),
        observed_locked: Arc::new(Mutex::new(Vec::new())),
    };
    let control = Arc::new(
        Control::new_test(identity(), binding(), clock.clone())
            .ok()
            .unwrap(),
    );
    let _ = clock.owner.set(Arc::downgrade(&control));
    control
        .enter_ready(control.ticket().unwrap(), evidence())
        .unwrap();
    clock.time.set(Duration::new(899, 999_999_999));
    let held = control.state.lock().unwrap();
    let (started_tx, started_rx) = std::sync::mpsc::channel();
    let copy = Arc::clone(&control);
    let worker = std::thread::spawn(move || {
        started_tx.send(()).unwrap();
        copy.apply(identity(), command())
    });
    started_rx.recv().unwrap();
    // The signal proves worker start, not that apply has reached or blocked on the lock.
    // Cross the deadline before release; the separate uncontended test proves ownership.
    clock.time.set(Duration::from_secs(900));
    drop(held);
    let joined = worker.join();
    // No outcome assertions until all workers have been joined.
    assert_eq!(joined.unwrap(), Err(Error::ConsentExpired));
    assert_eq!(*clock.observed_locked.lock().unwrap(), [true, true]);
    let current = control.snapshot().unwrap();
    assert_eq!(current.phase, Phase::Settling);
    assert_eq!(current.stop_cause, Some(Cause::ReadyExpired));
    assert_eq!(current.accepted, None);
    assert_eq!(current.revision, 3);
}

#[test]
fn old_ready_expiry_and_replay_do_not_renew_new_working_lease() {
    let (control, clock) = ready_fixture();
    let old_ready = control.ticket().unwrap();
    clock.set(Duration::from_secs(899));
    control.apply(identity(), command()).unwrap();
    let working = control.snapshot().unwrap();
    assert_eq!(
        working.lease,
        Lease {
            kind: LeaseKind::Working,
            deadline: Duration::from_secs(8099),
            deadline_tick_ms: 8_099_000
        }
    );
    clock.set(Duration::from_secs(900));
    assert_eq!(control.expire(old_ready), Ok(Expiry::Inert(working)));
    assert_eq!(
        control.apply(identity(), command()),
        Ok(Apply::Replay(working))
    );
    assert_eq!(control.snapshot(), Ok(working));
    let ticket = control.ticket().unwrap();
    clock.set(Duration::from_secs(8099));
    let mut settled = working;
    settled.phase = Phase::Settling;
    settled.revision = 4;
    settled.generation = 4;
    settled.stop_cause = Some(Cause::WorkingTimeout);
    settled.admission_closed = true;
    settled.gate = PublicationState::CancelAccepted;
    assert_eq!(
        control.expire(ticket),
        Ok(Expiry::Stop(CancelOutcome::Accepted, settled))
    );
    assert_eq!(
        control.apply(identity(), command()),
        Ok(Apply::Replay(settled))
    );
    assert_eq!(
        control.request_stop(identity(), Cause::ServerStop),
        Ok(CancelOutcome::AlreadyAccepted)
    );
    assert_eq!(control.snapshot(), Ok(settled));
}

#[test]
fn expiry_tickets_recheck_all_native_tags_before_clock_and_stop() {
    let (control, clock) = ready_fixture();
    let current = control.ticket().unwrap();
    let expected = control.snapshot().unwrap();
    let mut variants = [current; 7];
    variants[0].identity.incarnation[0] ^= 1;
    variants[1].identity.serial += 1;
    variants[2].identity.job[0] ^= 1;
    variants[3].phase = Phase::Previewing;
    variants[4].generation += 1;
    variants[5].kind = LeaseKind::Working;
    variants[6].deadline += Duration::from_nanos(1);
    *clock.0.lock().unwrap() = Err(());
    for stale in variants {
        assert_eq!(control.expire(stale), Ok(Expiry::Inert(expected)));
        assert_eq!(control.snapshot(), Ok(expected));
    }
    clock.set(Duration::new(899, 999_999_999));
    assert_eq!(control.expire(current), Ok(Expiry::Inert(expected)));
    clock.set(Duration::from_secs(900));
    let mut stopped = expected;
    stopped.phase = Phase::Settling;
    stopped.revision = 3;
    stopped.generation = 3;
    stopped.consent = Consent::Spent;
    stopped.stop_cause = Some(Cause::ReadyExpired);
    stopped.admission_closed = true;
    stopped.gate = PublicationState::CancelAccepted;
    assert_eq!(
        control.expire(current),
        Ok(Expiry::Stop(CancelOutcome::Accepted, stopped))
    );
    assert_eq!(control.expire(current), Ok(Expiry::Inert(stopped)));
    assert_eq!(control.snapshot(), Ok(stopped));
}

#[test]
fn ready_entry_due_preview_settles_working_timeout_and_never_revives() {
    for now in [Duration::from_secs(7200), Duration::new(7200, 1)] {
        let (control, clock) = fixture(Duration::ZERO);
        let ticket = control.ticket().unwrap();
        let mut expected = control.snapshot().unwrap();
        clock.set(now);
        expected.phase = Phase::Settling;
        expected.revision = 2;
        expected.generation = 2;
        expected.consent = Consent::Spent;
        expected.stop_cause = Some(Cause::WorkingTimeout);
        expected.admission_closed = true;
        expected.gate = PublicationState::CancelAccepted;
        assert_eq!(
            control.enter_ready(ticket, evidence()),
            Err(Error::InvalidPhase)
        );
        assert_eq!(control.snapshot(), Ok(expected));
        assert_eq!(
            control.enter_ready(ticket, evidence()),
            Err(Error::InvalidPhase)
        );
        assert_eq!(control.snapshot(), Ok(expected));
    }
    let (control, _) = fixture(Duration::ZERO);
    let old = control.ticket().unwrap();
    control.request_stop(identity(), Cause::ServerStop).unwrap();
    let expected = control.snapshot().unwrap();
    assert_eq!(
        control.enter_ready(old, evidence()),
        Err(Error::InvalidPhase)
    );
    assert_eq!(control.snapshot(), Ok(expected));
}

#[test]
fn ready_entry_refuses_each_stale_token_without_changing_current_state() {
    let (control, _) = fixture(Duration::ZERO);
    let original = control.ticket().unwrap();
    let mut variants = [original; 7];
    variants[0].identity.incarnation[0] ^= 1;
    variants[1].identity.serial += 1;
    variants[2].identity.job[0] ^= 1;
    variants[3].phase = Phase::Ready;
    variants[4].generation += 1;
    variants[5].kind = LeaseKind::Ready;
    variants[6].deadline += Duration::from_nanos(1);
    let expected = control.snapshot().unwrap();
    for stale in variants {
        assert_eq!(
            control.enter_ready(stale, evidence()),
            Err(Error::InvalidPhase)
        );
        assert_eq!(control.snapshot(), Ok(expected));
    }
    control.enter_ready(original, evidence()).unwrap();
    let ready = control.snapshot().unwrap();
    assert_eq!(
        control.enter_ready(control.ticket().unwrap(), evidence()),
        Err(Error::InvalidPhase)
    );
    assert_eq!(control.snapshot(), Ok(ready));
}

#[test]
fn each_nonsecret_identity_component_is_checked_before_replay_or_stop() {
    let (control, clock) = ready_fixture();
    control.apply(identity(), command()).unwrap();
    let expected = control.snapshot().unwrap();
    *clock.0.lock().unwrap() = Err(());
    let mut variants = [identity(); 3];
    variants[0].incarnation[0] ^= 1;
    variants[1].serial += 1;
    variants[2].job[0] ^= 1;
    for wrong in variants {
        assert_eq!(control.apply(wrong, command()), Err(Error::InvalidHandle));
        assert_eq!(
            control.request_stop(wrong, Cause::ServerStop),
            Err(Error::InvalidHandle)
        );
        assert_eq!(control.snapshot(), Ok(expected));
    }
    assert_eq!(control.binding.source, [1, 2, 3]);
    assert_eq!(control.binding.profile, [4, 5]);
    assert_eq!(control.binding.policy, 6);
    assert_eq!(control.binding.raw_preview, [7, 8, 9]);
}

#[test]
fn fresh_apply_refusal_uses_closed_first_cause_before_old_deadline() {
    for (cause, error) in [
        (Cause::ServerStop, Error::ServerStopping),
        (Cause::UserCancel, Error::InvalidPhase),
        (Cause::ReadyExpired, Error::ConsentExpired),
        (Cause::WorkingTimeout, Error::InvalidPhase),
        (Cause::CounterExhausted, Error::CounterExhausted),
    ] {
        let (control, clock) = ready_fixture();
        control.request_stop(identity(), cause).unwrap();
        let expected = control.snapshot().unwrap();
        clock.set(Duration::from_secs(10_000));
        assert_eq!(control.apply(identity(), command()), Err(error));
        assert_eq!(control.snapshot(), Ok(expected));
    }
    let (control, _) = fixture(Duration::ZERO);
    let expected = control.snapshot().unwrap();
    assert_eq!(
        control.apply(identity(), command()),
        Err(Error::InvalidPhase)
    );
    assert_eq!(control.snapshot(), Ok(expected));
}

#[test]
fn fresh_invalid_commands_leave_consent_and_replay_unspent() {
    let (control, _) = ready_fixture();
    let expected = control.snapshot().unwrap();
    let mut variants = [(command(), Error::CatalogueUpdateStale); 4];
    variants[0].0.consent[0] ^= 1;
    variants[1].0.artifact[0] ^= 1;
    variants[2].0.digest[0] ^= 1;
    variants[3].0.confirmed = false;
    variants[3].1 = Error::InvalidRequest;
    for (bad, error) in variants {
        assert_eq!(control.apply(identity(), bad), Err(error));
        assert_eq!(control.snapshot(), Ok(expected));
    }
    assert!(matches!(
        control.apply(identity(), command()),
        Ok(Apply::AcceptedOnce(_))
    ));
}

#[test]
fn whole_command_replay_and_conflict_precede_stop_phase_and_clock() {
    let (control, clock) = ready_fixture();
    control.apply(identity(), command()).unwrap();
    control.request_stop(identity(), Cause::ServerStop).unwrap();
    let expected = control.snapshot().unwrap();
    // A replay must not even sample this failed clock.
    *clock.0.lock().unwrap() = Err(());
    assert_eq!(
        control.apply(identity(), command()),
        Ok(Apply::Replay(expected))
    );
    let mut variants = [command(); 5];
    variants[0].command[0] ^= 1;
    variants[1].consent[0] ^= 1;
    variants[2].artifact[0] ^= 1;
    variants[3].digest[0] ^= 1;
    variants[4].confirmed = false;
    for changed in variants {
        assert_eq!(
            control.apply(identity(), changed),
            Err(Error::CommandConflict)
        );
        assert_eq!(control.snapshot(), Ok(expected));
    }
}

#[test]
fn apply_equality_and_after_expire_without_reaper_at_native_precision() {
    for advance in [
        Duration::ZERO,
        Duration::from_nanos(1),
        Duration::from_secs(1),
    ] {
        let (control, clock) = fixture(Duration::from_nanos(123));
        control
            .enter_ready(control.ticket().unwrap(), evidence())
            .unwrap();
        let mut expected = control.snapshot().unwrap();
        // 900s+123ns and the immediately preceding sample have the same display ms.
        clock.set(Duration::new(900, 123) + advance);
        expected.phase = Phase::Settling;
        expected.revision = 3;
        expected.generation = 3;
        expected.consent = Consent::Spent;
        expected.stop_cause = Some(Cause::ReadyExpired);
        expected.admission_closed = true;
        expected.gate = PublicationState::CancelAccepted;
        let mut bad = command();
        bad.digest = [0; 32];
        bad.confirmed = false;
        assert_eq!(control.apply(identity(), bad), Err(Error::ConsentExpired));
        assert_eq!(control.snapshot(), Ok(expected));
        assert_eq!(control.checkpoint(), Ok(Err(Stop::CancelAccepted)));
    }
}

#[test]
fn cancellation_latches_real_gate_and_first_cause_without_renewal() {
    let (control, _) = ready_fixture();
    let mut expected = control.snapshot().unwrap();
    expected.phase = Phase::Settling;
    expected.revision = 3;
    expected.generation = 3;
    expected.consent = Consent::Spent;
    expected.stop_cause = Some(Cause::UserCancel);
    expected.admission_closed = true;
    expected.gate = PublicationState::CancelAccepted;
    assert_eq!(
        control.request_stop(identity(), Cause::UserCancel),
        Ok(CancelOutcome::Accepted)
    );
    assert_eq!(control.snapshot(), Ok(expected));
    assert_eq!(
        control.request_stop(identity(), Cause::ServerStop),
        Ok(CancelOutcome::AlreadyAccepted)
    );
    assert_eq!(control.snapshot(), Ok(expected));
    assert_eq!(control.checkpoint(), Ok(Err(Stop::CancelAccepted)));
    assert!(!control.state.lock().unwrap().gate.try_claim_commit());
}

fn command() -> Command {
    Command {
        command: [6; 16],
        consent: [3; 16],
        artifact: [4; 16],
        digest: [5; 32],
        confirmed: true,
    }
}
fn ready_fixture() -> (Control<SyntheticEvidence, ManualClock>, ManualClock) {
    let (control, clock) = fixture(Duration::ZERO);
    control
        .enter_ready(control.ticket().unwrap(), evidence())
        .unwrap();
    (control, clock)
}
#[test]
fn apply_before_native_deadline_is_one_exact_transition() {
    let (control, clock) = ready_fixture();
    clock.set(Duration::new(899, 999_999_999));
    let mut expected = control.snapshot().unwrap();
    expected.phase = Phase::WaitingForWriter;
    expected.revision = 3;
    expected.generation = 3;
    expected.lease = Lease {
        kind: LeaseKind::Working,
        deadline: Duration::new(8099, 999_999_999),
        deadline_tick_ms: 8_099_999,
    };
    expected.consent = Consent::Spent;
    expected.accepted = Some(command());
    assert_eq!(
        control.apply(identity(), command()),
        Ok(Apply::AcceptedOnce(expected))
    );
    assert_eq!(control.snapshot(), Ok(expected));
}

use std::sync::Arc;

#[derive(Clone)]
struct ManualClock(Arc<Mutex<Result<Duration, ()>>>);
impl Clock for ManualClock {
    fn sample(&self) -> Result<Duration, ()> {
        *self.0.lock().unwrap()
    }
}
impl ManualClock {
    fn new(now: Duration) -> Self {
        Self(Arc::new(Mutex::new(Ok(now))))
    }
    fn set(&self, now: Duration) {
        *self.0.lock().unwrap() = Ok(now);
    }
}
// Deliberately neither Clone nor Eq: this is retained synthetic evidence, not verification.
struct SyntheticEvidence {
    source: Vec<u8>,
    profile: [u64; 2],
    policy: u32,
    raw_preview: Vec<u8>,
}
fn binding() -> SyntheticEvidence {
    SyntheticEvidence {
        source: vec![1, 2, 3],
        profile: [4, 5],
        policy: 6,
        raw_preview: vec![7, 8, 9],
    }
}
fn identity() -> Identity {
    Identity {
        incarnation: [1; 16],
        serial: 1,
        job: [2; 16],
    }
}
fn fixture(now: Duration) -> (Control<SyntheticEvidence, ManualClock>, ManualClock) {
    let clock = ManualClock::new(now);
    let control = Control::new_test(identity(), binding(), clock.clone()).expect("valid fixture");
    (control, clock)
}
#[test]
fn preview_initialization_has_exact_working_state() {
    let (control, _) = fixture(Duration::from_nanos(123));
    assert_eq!(
        control.snapshot(),
        Ok(Snapshot {
            identity: identity(),
            phase: Phase::Previewing,
            revision: 1,
            generation: 1,
            lease: Lease {
                kind: LeaseKind::Working,
                deadline: Duration::new(7200, 123),
                deadline_tick_ms: 7_200_000
            },
            consent: Consent::Unavailable,
            accepted: None,
            stop_cause: None,
            admission_closed: false,
            gate: PublicationState::Open,
        })
    );
    assert_eq!(control.binding.source, [1, 2, 3]);
    assert_eq!(control.binding.profile, [4, 5]);
    assert_eq!(control.binding.policy, 6);
    assert_eq!(control.binding.raw_preview, [7, 8, 9]);
}
fn evidence() -> ReadyEvidence {
    ReadyEvidence {
        consent: [3; 16],
        artifact: [4; 16],
        digest: [5; 32],
    }
}
#[test]
fn ready_entry_replaces_working_lease_with_exact_consent_and_counters() {
    let (control, clock) = fixture(Duration::ZERO);
    let preview = control.ticket().unwrap();
    clock.set(Duration::new(20, 456_789));
    let mut expected = control.snapshot().unwrap();
    expected.phase = Phase::Ready;
    expected.revision = 2;
    expected.generation = 2;
    expected.lease = Lease {
        kind: LeaseKind::Ready,
        deadline: Duration::new(920, 456_789),
        deadline_tick_ms: 920_000,
    };
    expected.consent = Consent::Available;
    assert_eq!(control.enter_ready(preview, evidence()), Ok(expected));
    assert_eq!(control.snapshot(), Ok(expected));
    assert_eq!(control.state.lock().unwrap().ready, Some(evidence()));
}

#[test]
fn b10_claim_refuses_elapsed_working_deadline_without_reaper() {
    let (control, clock) = ready_fixture();
    control.apply(identity(), command()).unwrap();
    clock.set(Duration::from_secs(7200));
    assert!(
        !control.claim_test().unwrap(),
        "elapsed working deadline must exclude native COMMIT claim"
    );
    let view = control.snapshot().unwrap();
    assert_eq!(view.gate, PublicationState::CancelAccepted);
    assert_eq!(view.stop_cause, Some(Cause::WorkingTimeout));
}
