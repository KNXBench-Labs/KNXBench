//! Runs MP §2.18 `NM_IndividualAddress_Reset`: devices in programming mode back to `FFFFh`.
//!
//! `[D]` MP §2.18, pp. 33–34. *"This Network Management Procedure shall be
//! used to reset the Individual Address of one or more devices in which
//! Programming Mode is active to the default Individual Address FFFFh."*
//! No parameters. The sequence, repeated *"until no
//! A_IndividualAddress_Response-PDU is received"*:
//!
//! 1. broadcast `A_IndividualAddress_Write` with `FFFFh`;
//! 2. `A_Connect`, `A_Restart`, `A_Disconnect` to `FFFFh` (*"deactivate
//!    Programming Mode"*; MP §3.7.1.1.1, p. 80: a Basic Restart shall switch
//!    Programming Mode off);
//! 3. broadcast `A_IndividualAddress_Read`; every answer is a device still
//!    in programming mode.
//!
//! *"Do not evaluate any local confirmation, or received telegrams, except
//! the A_IndividualAddress_Read.Lcon and the A_IndividualAddress_Response-PDU."*
//!
//! Ours, not the procedure's, and said so in the report:
//!
//! - **A first read** before step 1, which lists the devices about to be
//!   reset. With none in programming mode nothing is written.
//! - **A round cap** ([`MAX_ROUNDS`]): MP §2.18 repeats without a bound; a
//!   device that never leaves programming mode must not keep the procedure
//!   on the bus for ever.
//! - **The read window** is `programming_mode_broadcast_timeout`, MP §2.3
//!   step 2's 1 s; §2.18 names none.
//! - **The expected devices** (added for hardware, 2026-09-30): the caller
//!   names the devices whose buttons it expects to be pressed, and the first
//!   read must find exactly that set. MP §2.18 resets whoever is in
//!   programming mode, including a device somebody else is commissioning
//!   on the same line; a mismatch in either direction writes nothing. An
//!   excluded address in programming mode is refused even if named.

use std::collections::BTreeSet;

use knx_core::commissioning::mutation::{WriteAuthorisation, WriteScope};
use knx_core::{ContactableAddress, ExcludedAddress, IndividualAddress};

use crate::commissioning::{AuthorisationPlan, ManagementSession, SessionError, SessionTiming};
use crate::management::ManagementTransport;

/// The default individual address, MP §2.18: `FFFFh`, printed `15.15.255`.
pub const DEFAULT_INDIVIDUAL_ADDRESS: IndividualAddress = IndividualAddress::from_raw(0xFFFF);

/// Rounds of MP §2.18's loop before this project gives up. Ours.
pub const MAX_ROUNDS: u32 = 3;

/// What a reset did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndividualAddressResetReport {
    /// The devices that answered the first read, by the address they had.
    /// Empty: nothing was in programming mode, and nothing was written.
    pub in_programming_mode: Vec<IndividualAddress>,
    /// How many times the loop ran; 0 when nothing was in programming mode.
    pub rounds: u32,
}

/// Why `NM_IndividualAddress_Reset` stopped.
#[derive(Debug)]
pub enum IndividualAddressResetError {
    /// [`MAX_ROUNDS`] rounds, and devices still answer the read.
    StillInProgrammingMode {
        rounds: u32,
        /// Where they answered from.
        still_answering: Vec<IndividualAddress>,
    },
    /// The first read did not find exactly the devices the caller expected
    /// in programming mode. Nothing was written.
    NotTheExpectedDevices {
        /// What the caller named, sorted and without repeats.
        expected: Vec<IndividualAddress>,
        /// What answered the first read.
        found: Vec<IndividualAddress>,
    },
    /// A device on the project exclusion list is in programming mode.
    /// Nothing was written.
    ExcludedDeviceInProgrammingMode(ExcludedAddress),
    /// A session or transport failure in the named step.
    Session {
        step: &'static str,
        source: SessionError,
    },
}

impl std::fmt::Display for IndividualAddressResetError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::StillInProgrammingMode {
                rounds,
                still_answering,
            } => {
                let list: Vec<_> = still_answering.iter().map(ToString::to_string).collect();
                write!(
                    f,
                    "MP §2.18: after {rounds} rounds these devices still answer in programming \
                     mode: {}",
                    list.join(", ")
                )
            }
            Self::NotTheExpectedDevices { expected, found } => {
                let list = |devices: &[IndividualAddress]| -> String {
                    if devices.is_empty() {
                        return "none".to_string();
                    }
                    let names: Vec<_> = devices.iter().map(ToString::to_string).collect();
                    names.join(", ")
                };
                write!(
                    f,
                    "in programming mode: {}; expected: {}. MP §2.18 would reset every one of \
                     them, so nothing was written",
                    list(found),
                    list(expected)
                )
            }
            Self::ExcludedDeviceInProgrammingMode(excluded) => {
                write!(
                    f,
                    "{excluded}, and it is in programming mode; nothing was written"
                )
            }
            Self::Session { step, source } => write!(f, "MP §2.18 {step}: {source}"),
        }
    }
}

impl std::error::Error for IndividualAddressResetError {}

fn at(step: &'static str) -> impl Fn(SessionError) -> IndividualAddressResetError {
    move |source| IndividualAddressResetError::Session { step, source }
}

/// Resets every device in programming mode to `FFFFh`, provided those are
/// exactly `expected`.
///
/// `authorisation` must name `FFFFh` and [`WriteScope::IndividualAddressReset`].
/// `expected` lists the devices, by their current address, whose buttons the
/// operator pressed; order and repeats do not matter. The first read must
/// find that set and no excluded address, or nothing is written. An empty
/// `expected` with nobody pressed is a no-op.
pub async fn individual_address_reset<T: ManagementTransport>(
    transport: &T,
    timing: SessionTiming,
    authorisation: WriteAuthorisation,
    expected: &[IndividualAddress],
) -> Result<IndividualAddressResetReport, IndividualAddressResetError> {
    let mut session =
        ManagementSession::authorised(transport, AuthorisationPlan::Skip, timing, authorisation)
            .map_err(at("authorisation"))?;
    // The scope is checked by the session on each write; checking it here
    // too makes a wrong authorisation fail before anything is sent.
    if session.session_scope() != Some(WriteScope::IndividualAddressReset) {
        return Err(at("authorisation")(SessionError::NoAuthorisation {
            scope: WriteScope::IndividualAddressReset,
        }));
    }
    // The broadcast write sends the session's target as the new address,
    // so an authorisation for any other address would program that one.
    if session.target() != DEFAULT_INDIVIDUAL_ADDRESS {
        return Err(at("authorisation")(SessionError::Refused(
            knx_core::commissioning::mutation::AuthorisationRefused::WrongTarget {
                authorised: session.target(),
                attempted: DEFAULT_INDIVIDUAL_ADDRESS,
            },
        )));
    }

    let first = session
        .broadcast_individual_address_read(timing.programming_mode_broadcast_timeout)
        .await
        .map_err(at("who is in programming mode"))?;
    let in_programming_mode: Vec<_> = first.devices().collect();
    for device in &in_programming_mode {
        ContactableAddress::new(*device)
            .map_err(IndividualAddressResetError::ExcludedDeviceInProgrammingMode)?;
    }
    let expected: BTreeSet<_> = expected.iter().copied().collect();
    let found: BTreeSet<_> = in_programming_mode.iter().copied().collect();
    if expected != found {
        return Err(IndividualAddressResetError::NotTheExpectedDevices {
            expected: expected.into_iter().collect(),
            found: in_programming_mode,
        });
    }
    if in_programming_mode.is_empty() {
        return Ok(IndividualAddressResetReport {
            in_programming_mode,
            rounds: 0,
        });
    }

    let mut still_answering = in_programming_mode.clone();
    for round in 1..=MAX_ROUNDS {
        session
            .broadcast_individual_address_write_as(WriteScope::IndividualAddressReset)
            .await
            .map_err(at("A_IndividualAddress_Write FFFFh"))?;
        session
            .restart_at_default_address_unevaluated()
            .await
            .map_err(at("A_Restart to FFFFh"))?;
        let verify = session
            .broadcast_individual_address_read(timing.programming_mode_broadcast_timeout)
            .await
            .map_err(at("A_IndividualAddress_Read"))?;
        still_answering = verify.devices().collect();
        if still_answering.is_empty() {
            return Ok(IndividualAddressResetReport {
                in_programming_mode,
                rounds: round,
            });
        }
    }
    Err(IndividualAddressResetError::StillInProgrammingMode {
        rounds: MAX_ROUNDS,
        still_answering,
    })
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use knx_core::commissioning::mutation::{
        required_confirmation_phrase, WriteAuthorisation, WriteScope,
    };

    use super::*;
    use crate::commissioning::simulator::{Seen, SimulatedDevice, SimulatorConfig};

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

    fn addr(area: u8, line: u8, device: u8) -> IndividualAddress {
        IndividualAddress::new(area, line, device).expect("a valid individual address")
    }

    fn reset() -> WriteAuthorisation {
        WriteAuthorisation::for_simulator(
            DEFAULT_INDIVIDUAL_ADDRESS,
            WriteScope::IndividualAddressReset,
        )
        .expect("FFFFh is not an excluded address")
    }

    fn pressed(others: Vec<IndividualAddress>, stuck: bool) -> SimulatedDevice {
        SimulatedDevice::with_config(SimulatorConfig {
            programming_mode: true,
            other_programming_mode_devices: others,
            other_programming_mode_devices_ignore_restart: stuck,
            ..Default::default()
        })
    }

    #[tokio::test]
    async fn one_device_goes_to_ffff_and_leaves_programming_mode() {
        let device = pressed(Vec::new(), false);
        let before = device.address();
        let report = individual_address_reset(&device, fast(), reset(), &[before])
            .await
            .unwrap();
        assert_eq!(report.in_programming_mode, vec![before]);
        assert_eq!(report.rounds, 1);
        assert_eq!(device.address(), DEFAULT_INDIVIDUAL_ADDRESS);
        assert!(
            !device.programming_mode(),
            "MP §3.7.1.1.1: the restart ends it"
        );
        // The sequence on the wire: read, [write, connect, restart,
        // disconnect, read]. Nothing waits on the restart's T_ACK.
        let seen = device.seen();
        assert!(seen.contains(&Seen::Connect));
        assert!(seen.iter().any(|s| matches!(
            s,
            Seen::Restart {
                response: false,
                restart_type: 0,
                ..
            }
        )));
        assert_eq!(device.individual_address_read_broadcasts(), 2);
    }

    #[tokio::test]
    async fn every_device_in_programming_mode_is_reset_together() {
        let others = vec![addr(1, 1, 40), addr(1, 2, 7)];
        let device = pressed(others.clone(), false);
        let expected = [device.address(), others[0], others[1]];
        let report = individual_address_reset(&device, fast(), reset(), &expected)
            .await
            .unwrap();
        assert_eq!(report.in_programming_mode.len(), 3);
        for other in &others {
            assert!(report.in_programming_mode.contains(other));
        }
        assert_eq!(device.address(), DEFAULT_INDIVIDUAL_ADDRESS);
        assert!(device.other_programming_mode_devices().is_empty());
    }

    #[tokio::test]
    async fn nothing_in_programming_mode_writes_nothing() {
        let device = SimulatedDevice::with_config(SimulatorConfig {
            programming_mode: false,
            ..Default::default()
        });
        let before = device.address();
        // Nobody named and nobody pressed: the no-op. Naming a device that
        // is not pressed is the guard's case, below.
        let report = individual_address_reset(&device, fast(), reset(), &[])
            .await
            .unwrap();
        assert!(report.in_programming_mode.is_empty());
        assert_eq!(report.rounds, 0);
        assert_eq!(device.address(), before);
        assert!(!device.seen().contains(&Seen::Connect));
        assert_eq!(device.individual_address_read_broadcasts(), 1);
    }

    #[tokio::test]
    async fn a_device_that_stays_in_programming_mode_stops_the_loop_at_the_cap() {
        let device = pressed(vec![addr(1, 1, 40)], true);
        let expected = [device.address(), addr(1, 1, 40)];
        let err = individual_address_reset(&device, fast(), reset(), &expected)
            .await
            .unwrap_err();
        match err {
            IndividualAddressResetError::StillInProgrammingMode {
                rounds,
                still_answering,
            } => {
                assert_eq!(rounds, MAX_ROUNDS);
                assert_eq!(still_answering, vec![DEFAULT_INDIVIDUAL_ADDRESS]);
            }
            other => panic!("expected the cap, got {other}"),
        }
        assert_eq!(
            device.individual_address_read_broadcasts(),
            1 + MAX_ROUNDS as usize
        );
    }

    #[tokio::test]
    async fn another_scope_or_target_is_refused_before_anything_is_sent() {
        let device = pressed(Vec::new(), false);
        let before = device.address();
        for authorisation in [
            WriteAuthorisation::for_simulator(
                DEFAULT_INDIVIDUAL_ADDRESS,
                WriteScope::IndividualAddressProgramming,
            )
            .unwrap(),
            WriteAuthorisation::for_simulator(addr(1, 1, 30), WriteScope::IndividualAddressReset)
                .unwrap(),
        ] {
            assert!(
                individual_address_reset(&device, fast(), authorisation, &[before])
                    .await
                    .is_err()
            );
        }
        assert_eq!(device.address(), before);
        assert!(device.seen().is_empty(), "{:?}", device.seen());
    }

    /// Nothing was written: the address is the old one, no connection was
    /// opened, and only the first read went out.
    fn assert_untouched(device: &SimulatedDevice, before: IndividualAddress) {
        assert_eq!(device.address(), before);
        assert!(device.programming_mode(), "nothing ended programming mode");
        assert!(
            !device.seen().contains(&Seen::Connect),
            "{:?}",
            device.seen()
        );
        assert_eq!(device.individual_address_read_broadcasts(), 1);
    }

    /// The hardware guard: MP §2.18 resets whoever has a button pressed, so
    /// a second pressed device the operator did not name stops the
    /// procedure before its first write.
    #[tokio::test]
    async fn an_unexpected_device_in_programming_mode_stops_before_writing() {
        let stranger = addr(1, 1, 40);
        let device = pressed(vec![stranger], false);
        let before = device.address();
        let err = individual_address_reset(&device, fast(), reset(), &[before])
            .await
            .unwrap_err();
        match &err {
            IndividualAddressResetError::NotTheExpectedDevices { expected, found } => {
                assert_eq!(expected, &vec![before]);
                assert!(
                    found.contains(&stranger) && found.contains(&before),
                    "{err}"
                );
            }
            other => panic!("expected the guard, got {other}"),
        }
        assert!(err.to_string().contains("nothing was written"), "{err}");
        assert_untouched(&device, before);
        assert_eq!(device.other_programming_mode_devices(), vec![stranger]);
    }

    /// The other direction: a named device whose button is not pressed.
    /// Resetting only some of what the operator named is not what they
    /// confirmed either.
    #[tokio::test]
    async fn a_named_device_that_is_not_pressed_stops_before_writing() {
        let device = pressed(Vec::new(), false);
        let before = device.address();
        let err = individual_address_reset(&device, fast(), reset(), &[before, addr(1, 1, 40)])
            .await
            .unwrap_err();
        assert!(
            matches!(
                err,
                IndividualAddressResetError::NotTheExpectedDevices { .. }
            ),
            "{err}"
        );
        assert_untouched(&device, before);
    }

    /// An excluded address in programming mode is never reset, not even
    /// when the operator lists it.
    #[tokio::test]
    async fn an_excluded_device_in_programming_mode_is_never_reset() {
        let excluded = knx_core::EXCLUDED_INDIVIDUAL_ADDRESSES[0];
        let device = pressed(vec![excluded], false);
        let before = device.address();
        let err = individual_address_reset(&device, fast(), reset(), &[before, excluded])
            .await
            .unwrap_err();
        match &err {
            IndividualAddressResetError::ExcludedDeviceInProgrammingMode(found) => {
                assert_eq!(found.0, excluded)
            }
            other => panic!("expected the exclusion, got {other}"),
        }
        assert_untouched(&device, before);
    }

    /// The order the operator names devices in, and a repeat, do not
    /// matter: the check compares sets.
    #[tokio::test]
    async fn the_expected_devices_are_compared_as_a_set() {
        let other = addr(1, 2, 7);
        let device = pressed(vec![other], false);
        let before = device.address();
        let report = individual_address_reset(&device, fast(), reset(), &[other, before, other])
            .await
            .unwrap();
        assert_eq!(report.rounds, 1);
        assert_eq!(device.address(), DEFAULT_INDIVIDUAL_ADDRESS);
    }

    /// K13 on hardware (2026-09-30): the scope is allowed, on the phrase
    /// that names `15.15.255`.
    #[test]
    fn the_scope_is_allowed_on_hardware_with_the_phrase() {
        let phrase = required_confirmation_phrase(
            DEFAULT_INDIVIDUAL_ADDRESS,
            WriteScope::IndividualAddressReset,
        );
        assert_eq!(phrase, "I confirm individual-address reset to 15.15.255");
        assert!(
            knx_core::commissioning::mutation::hardware_write_is_authorised(
                WriteScope::IndividualAddressReset
            )
        );
        assert!(WriteAuthorisation::for_hardware(
            DEFAULT_INDIVIDUAL_ADDRESS,
            WriteScope::IndividualAddressReset,
            &phrase
        )
        .is_ok());
    }
}
