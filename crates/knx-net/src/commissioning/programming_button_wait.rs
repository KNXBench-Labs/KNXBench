//! MP §2.3 step 2's `repeat`: wait round by round until exactly one device is in programming mode.
//!
//! [`individual_address_write`] broadcasts once and returns a count error on
//! nobody or several (`KNOWN_LIMITATIONS.md` §116). This module is the loop
//! the Standard asks for, in the only place it can live: next to the
//! operator. MP §2.3, p. 14: *"2. wait until device is in Programming Mode:
//! repeat until one A_IndividualAddress_Response-PDU is received"*; p. 15,
//! footnote 2): *"The user of the Management Client should get an
//! information in how many devices are Programming Mode is active (none or
//! more than one)."* Every round is reported to an observer, which is how
//! the CLI and the UI tell the operator "press the button on exactly one
//! device", and the observer can stop the wait.
//!
//! The wait runs **before** step 1, not between steps 1 and 2 as the
//! clause orders them. Nothing is lost: step 1 only reads, and its verdict
//! is compared with the programming-mode device's address, which exists
//! only once somebody pressed a button. [`individual_address_write`] then
//! runs all four steps unchanged, including its own step 2 count and its
//! re-count right before the write, so a button released in between is
//! still caught there.
//!
//! Waiting writes nothing: every round is one broadcast
//! `A_IndividualAddress_Read`, a read. Stopping a wait is always safe.

use std::ops::ControlFlow;
use std::time::Duration;

use knx_core::commissioning::authorisation::AuthorisationPlan;
use knx_core::commissioning::mutation::WriteAuthorisation;
use knx_core::IndividualAddress;

use super::individual_address_write::{
    individual_address_write, IndividualAddressWriteError, IndividualAddressWriteReport,
};
use super::{ManagementSession, SessionError, SessionTiming};
use crate::management::ManagementTransport;

/// How long to keep asking for a button press.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ButtonWait {
    /// Stop waiting after this long without exactly one device. MP §2.3
    /// sets no limit; the operator does, because only they know how far
    /// away the device is.
    pub give_up_after: Duration,
    /// Quiet time between two rounds, on top of the round's own 1 s
    /// listening window. `[A]` A KNXBench choice, not the Standard's: it
    /// halves the broadcasts a waiting operator puts on the bus and does
    /// not delay a press noticeably.
    pub pause_between_rounds: Duration,
}

impl ButtonWait {
    /// The pause between rounds when the caller has no reason to change it.
    pub const DEFAULT_PAUSE: Duration = Duration::from_secs(1);

    /// Waits up to `give_up_after`, with the default pause.
    pub fn up_to(give_up_after: Duration) -> Self {
        Self {
            give_up_after,
            pause_between_rounds: Self::DEFAULT_PAUSE,
        }
    }
}

/// What one round of the wait found, for the operator.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ButtonEvent {
    /// A round ended. `in_programming_mode` lists every device that
    /// answered, by its current address: empty means "press the button",
    /// two or more means "release all but one".
    Round {
        /// 1-based.
        number: u32,
        in_programming_mode: Vec<IndividualAddress>,
    },
    /// Exactly one device answered; the procedure starts now.
    Found {
        /// Its address before the procedure changes it.
        current_address: IndividualAddress,
    },
}

/// Why the procedure did not complete.
#[derive(Debug)]
pub enum ButtonProgrammingError {
    /// Nobody pressed a button in time, or several stayed pressed. Nothing
    /// was written.
    GaveUp {
        rounds: u32,
        /// The last round's answer.
        in_programming_mode: Vec<IndividualAddress>,
    },
    /// The observer stopped the wait. Nothing was written.
    Stopped { rounds: u32 },
    /// A round could not be run (the tunnel, not the device). Nothing was
    /// written.
    Wait(SessionError),
    /// The procedure itself stopped; see its error for how far it got.
    Procedure(IndividualAddressWriteError),
}

impl std::error::Error for ButtonProgrammingError {}

impl std::fmt::Display for ButtonProgrammingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ButtonProgrammingError::GaveUp {
                rounds,
                in_programming_mode,
            } => {
                write!(f, "gave up after {rounds} rounds: ")?;
                write_count(f, in_programming_mode)?;
                write!(f, "; nothing was written")
            }
            ButtonProgrammingError::Stopped { rounds } => {
                write!(f, "stopped after {rounds} rounds; nothing was written")
            }
            ButtonProgrammingError::Wait(err) => {
                write!(f, "waiting for the button: {err}; nothing was written")
            }
            ButtonProgrammingError::Procedure(err) => write!(f, "{err}"),
        }
    }
}

/// "no device", "1 device (1.1.5)", "2 devices (1.1.5, 1.1.9)".
pub fn write_count(
    f: &mut impl std::fmt::Write,
    devices: &[IndividualAddress],
) -> std::fmt::Result {
    match devices {
        [] => write!(f, "no device is in programming mode"),
        [one] => write!(f, "1 device is in programming mode ({one})"),
        several => {
            write!(f, "{} devices are in programming mode (", several.len())?;
            for (index, device) in several.iter().enumerate() {
                if index > 0 {
                    write!(f, ", ")?;
                }
                write!(f, "{device}")?;
            }
            write!(f, ")")
        }
    }
}

/// The two authorisations MP §2.3 needs: the broadcast write (steps 2-3)
/// and step 4's restart. Both name the new address.
#[derive(Debug)]
pub struct AddressProgrammingAuthorisation {
    pub programming: WriteAuthorisation,
    pub restart: WriteAuthorisation,
}

/// The finished procedure, with the wait that preceded it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ButtonProgrammingReport {
    /// Rounds until exactly one device answered.
    pub rounds: u32,
    /// The device's address before the procedure.
    pub previous_address: IndividualAddress,
    /// The procedure's own report.
    pub procedure: IndividualAddressWriteReport,
}

/// Waits until exactly one device is in programming mode, then runs
/// [`individual_address_write`] to give it `new_address`.
///
/// `observe` hears every round and may return [`ControlFlow::Break`] to
/// stop the wait. Once the procedure has started it runs to its end: MP
/// §2.3 has no safe point to stop between the write and the restart.
pub async fn program_individual_address<T: ManagementTransport>(
    transport: &T,
    plan: AuthorisationPlan,
    timing: SessionTiming,
    new_address: IndividualAddress,
    authorisation: AddressProgrammingAuthorisation,
    wait: ButtonWait,
    mut observe: impl FnMut(&ButtonEvent) -> ControlFlow<()>,
) -> Result<ButtonProgrammingReport, ButtonProgrammingError> {
    // Read-only: a round is a broadcast read and needs no write scope.
    let counting =
        ManagementSession::read_only(transport, new_address, AuthorisationPlan::Skip, timing)
            .map_err(ButtonProgrammingError::Wait)?;
    let deadline = tokio::time::Instant::now() + wait.give_up_after;
    let mut rounds = 0;
    let previous_address = loop {
        rounds += 1;
        let responders = counting
            .broadcast_individual_address_read(timing.programming_mode_broadcast_timeout)
            .await
            .map_err(ButtonProgrammingError::Wait)?;
        let found: Vec<IndividualAddress> = responders.devices().collect();
        let round = ButtonEvent::Round {
            number: rounds,
            in_programming_mode: found.clone(),
        };
        if observe(&round).is_break() {
            return Err(ButtonProgrammingError::Stopped { rounds });
        }
        if let [one] = found.as_slice() {
            break *one;
        }
        if tokio::time::Instant::now() + wait.pause_between_rounds >= deadline {
            return Err(ButtonProgrammingError::GaveUp {
                rounds,
                in_programming_mode: found,
            });
        }
        tokio::time::sleep(wait.pause_between_rounds).await;
    };
    // Reported, not asked: after this the procedure runs to its end.
    let _ = observe(&ButtonEvent::Found {
        current_address: previous_address,
    });
    let procedure = individual_address_write(
        transport,
        plan,
        timing,
        new_address,
        authorisation.programming,
        authorisation.restart,
    )
    .await
    .map_err(ButtonProgrammingError::Procedure)?;
    Ok(ButtonProgrammingReport {
        rounds,
        previous_address,
        procedure,
    })
}

#[cfg(test)]
mod tests {
    use super::super::simulator::{SimulatedDevice, SimulatorConfig};
    use super::*;
    use knx_core::commissioning::mutation::WriteScope;
    use knx_core::commissioning::programming_mode::ProgrammingModeCountError;
    use std::sync::{Arc, Mutex};

    fn fast() -> SessionTiming {
        SessionTiming {
            connection_timeout: Duration::from_millis(50),
            response_timeout: Duration::from_millis(50),
            poll_interval: Duration::from_millis(1),
            max_transition: Duration::from_millis(40),
            programming_delay: Duration::from_millis(0),
            restart_basic_t1: Duration::from_millis(1),
            restart_responsive_again: Duration::from_millis(5),
            post_restart_disconnect_wait: Duration::from_millis(60),
            programming_mode_broadcast_timeout: Duration::from_millis(20),
        }
    }

    fn wait(give_up_after_ms: u64) -> ButtonWait {
        ButtonWait {
            give_up_after: Duration::from_millis(give_up_after_ms),
            pause_between_rounds: Duration::from_millis(5),
        }
    }

    fn addr(area: u8, line: u8, device: u8) -> IndividualAddress {
        IndividualAddress::new(area, line, device).expect("a valid individual address")
    }

    fn authorisations(target: IndividualAddress) -> AddressProgrammingAuthorisation {
        AddressProgrammingAuthorisation {
            programming: WriteAuthorisation::for_simulator(
                target,
                WriteScope::IndividualAddressProgramming,
            )
            .expect("not excluded"),
            restart: WriteAuthorisation::for_simulator(target, WriteScope::Restart)
                .expect("not excluded"),
        }
    }

    /// The Standard's `repeat`: nobody for a few rounds, then a press, then
    /// the address. Every round reached the operator.
    #[tokio::test]
    async fn waits_until_the_button_is_pressed_then_programs() {
        let device = Arc::new(SimulatedDevice::new());
        let original = device.address();
        let new_address = addr(1, 1, 30);
        let authorisation = authorisations(new_address);
        let events = Arc::new(Mutex::new(Vec::new()));
        let seen = Arc::clone(&events);
        let presser = Arc::clone(&device);
        let report = program_individual_address(
            device.as_ref(),
            AuthorisationPlan::Skip,
            fast(),
            new_address,
            authorisation,
            wait(5_000),
            move |event| {
                seen.lock().unwrap().push(event.clone());
                // The operator walks over during round 3.
                if matches!(event, ButtonEvent::Round { number: 3, .. }) {
                    presser.set_programming_mode(true);
                }
                ControlFlow::Continue(())
            },
        )
        .await
        .expect("a pressed button must end the wait and the procedure must succeed");

        assert_eq!(
            report.rounds, 4,
            "rounds 1-3 find nobody, round 4 the device"
        );
        assert_eq!(report.previous_address, original);
        assert!(report.procedure.wrote);
        assert_eq!(device.address(), new_address);
        let events = events.lock().unwrap();
        for number in 1..=3 {
            assert!(
                events.contains(&ButtonEvent::Round {
                    number,
                    in_programming_mode: vec![]
                }),
                "round {number} must be reported empty: {events:?}"
            );
        }
        assert_eq!(
            events.last(),
            Some(&ButtonEvent::Found {
                current_address: original
            })
        );
    }

    /// Several pressed buttons are not "one": the wait goes on until the
    /// operator releases the others, and says who is pressed meanwhile.
    #[tokio::test]
    async fn several_pressed_buttons_keep_it_waiting_and_are_named() {
        let other = addr(1, 1, 99);
        let device = Arc::new(SimulatedDevice::with_config(SimulatorConfig {
            programming_mode: true,
            other_programming_mode_devices: vec![other],
            ..Default::default()
        }));
        let original = device.address();
        let new_address = addr(1, 1, 31);
        let authorisation = authorisations(new_address);
        let events = Arc::new(Mutex::new(Vec::new()));
        let seen = Arc::clone(&events);
        let releaser = Arc::clone(&device);
        program_individual_address(
            device.as_ref(),
            AuthorisationPlan::Skip,
            fast(),
            new_address,
            authorisation,
            wait(5_000),
            move |event| {
                seen.lock().unwrap().push(event.clone());
                if matches!(event, ButtonEvent::Round { number: 2, .. }) {
                    releaser.set_other_programming_mode_devices(vec![]);
                }
                ControlFlow::Continue(())
            },
        )
        .await
        .expect("one button left must end the wait");
        let events = events.lock().unwrap();
        let ButtonEvent::Round {
            in_programming_mode,
            ..
        } = &events[0]
        else {
            panic!("the first event must be a round: {events:?}");
        };
        let mut expected = vec![original, other];
        expected.sort();
        let mut got = in_programming_mode.clone();
        got.sort();
        assert_eq!(got, expected, "both pressed devices must be named");
        assert_eq!(device.address(), new_address);
    }

    /// Nobody comes: the wait ends by itself, and nothing was written.
    #[tokio::test]
    async fn gives_up_when_nobody_presses_and_writes_nothing() {
        let device = SimulatedDevice::new();
        let original = device.address();
        let new_address = addr(1, 1, 32);
        let authorisation = authorisations(new_address);
        // Its own deadline, so a wait that never gives up fails this test
        // instead of hanging the suite.
        let err = tokio::time::timeout(
            Duration::from_secs(5),
            program_individual_address(
                &device,
                AuthorisationPlan::Skip,
                fast(),
                new_address,
                authorisation,
                wait(120),
                |_| ControlFlow::Continue(()),
            ),
        )
        .await
        .expect("the wait must end by itself")
        .expect_err("nobody pressed a button");
        let ButtonProgrammingError::GaveUp {
            rounds,
            in_programming_mode,
        } = err
        else {
            panic!("must give up, got {err:?}");
        };
        assert!(rounds >= 2, "it must have asked more than once: {rounds}");
        assert!(in_programming_mode.is_empty());
        assert_eq!(device.address(), original);
        assert!(
            !device
                .seen()
                .iter()
                .any(|entry| matches!(entry, super::super::simulator::Seen::Restart { .. })),
            "a wait that gave up must not restart anything"
        );
    }

    /// The operator cancels while waiting: nothing is written.
    #[tokio::test]
    async fn the_operator_can_stop_the_wait() {
        let device = SimulatedDevice::new();
        let original = device.address();
        let new_address = addr(1, 1, 33);
        let authorisation = authorisations(new_address);
        let err = program_individual_address(
            &device,
            AuthorisationPlan::Skip,
            fast(),
            new_address,
            authorisation,
            wait(5_000),
            |event| match event {
                ButtonEvent::Round { number: 2, .. } => ControlFlow::Break(()),
                _ => ControlFlow::Continue(()),
            },
        )
        .await
        .expect_err("the operator stopped");
        assert!(matches!(err, ButtonProgrammingError::Stopped { rounds: 2 }));
        assert_eq!(device.individual_address_read_broadcasts(), 2);
        assert_eq!(device.address(), original);
    }

    /// Pressed during the wait, released before the procedure's own count:
    /// the procedure's step 2 catches it, and nothing is written.
    #[tokio::test]
    async fn a_button_released_after_the_wait_is_caught_by_step_two() {
        let device = Arc::new(SimulatedDevice::with_config(SimulatorConfig {
            programming_mode: true,
            ..Default::default()
        }));
        let original = device.address();
        let new_address = addr(1, 1, 34);
        let authorisation = authorisations(new_address);
        let releaser = Arc::clone(&device);
        let err = program_individual_address(
            device.as_ref(),
            AuthorisationPlan::Skip,
            fast(),
            new_address,
            authorisation,
            wait(5_000),
            move |event| {
                if matches!(event, ButtonEvent::Found { .. }) {
                    releaser.set_programming_mode(false);
                }
                ControlFlow::Continue(())
            },
        )
        .await
        .expect_err("the button went away");
        assert!(
            matches!(
                err,
                ButtonProgrammingError::Procedure(IndividualAddressWriteError::Count(
                    ProgrammingModeCountError::NoDeviceInProgrammingMode
                ))
            ),
            "{err:?}"
        );
        assert_eq!(device.address(), original);
    }

    #[test]
    fn counts_read_like_the_operator_needs_them() {
        let mut out = String::new();
        write_count(&mut out, &[]).unwrap();
        assert_eq!(out, "no device is in programming mode");
        out.clear();
        write_count(&mut out, &[addr(1, 1, 5), addr(1, 1, 9)]).unwrap();
        assert_eq!(out, "2 devices are in programming mode (1.1.5, 1.1.9)");
    }
}
