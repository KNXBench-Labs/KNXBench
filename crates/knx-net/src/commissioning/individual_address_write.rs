//! Runs MP §2.3 `NM_IndividualAddress_Write`, the one procedure `procedure.rs` only described.
//!
//! `procedure.rs:195` names the four steps and `commissioning.rs` already
//! carries their primitives — [`ManagementSession::probe_device_descriptor`],
//! [`ManagementSession::broadcast_individual_address_read`],
//! [`ManagementSession::broadcast_individual_address_write`],
//! [`ManagementSession::restart_basic`] — but nothing had walked them in
//! order until this module. `[C16]`.
//!
//! Three sessions, not one: step 1 probes `IA_new` with no authorisation at
//! all ([`AuthorisationPlan::Skip`]), steps 2 and 3 never open a connection
//! and need only the individual-address-programming scope, and step 4 opens
//! a fully authorised connection to `IA_new` for the restart. A single
//! [`ManagementSession`] cannot be all three: its target address and its
//! authorisation are fixed at construction (`commissioning.rs`'s own
//! `build`), and step 1 must be free to find `IA_new` occupied by a device
//! this session never intends to write to.
//!
//! Occupancy (step 1) is decided by address comparison, not by which signal
//! produced it — `KNOWN_LIMITATIONS.md` §108: MP §2.3's "to 2." exception
//! stops the procedure only when `IA_new` is held by a device *other than*
//! the one step 2 finds in Programming Mode, and that comparison applies
//! uniformly whether occupancy was learned from a
//! `DeviceDescriptorResponse`, a `T_Disconnect`, or a rejected `T_Connect` —
//! none of which carry the occupant's identity by themselves. Only the
//! Programming Mode witness's own reported address answers "whose is it".

use knx_core::commissioning::authorisation::AuthorisationPlan;
use knx_core::commissioning::mutation::WriteAuthorisation;
use knx_core::commissioning::procedure::ProcedureKind;
use knx_core::commissioning::programming_mode::ProgrammingModeCountError;
use knx_core::IndividualAddress;

use super::download::StepRecord;
use super::{ManagementSession, SessionError, SessionTiming};
use crate::management::ManagementTransport;

/// How step 1 found `IA_new` occupied, if it did.
///
/// Three of the four variants stop the procedure under exactly the same
/// condition — none of the three occupied signals identifies the occupant,
/// so all three are resolved the same way, by comparing against step 2's
/// Programming Mode witness (`KNOWN_LIMITATIONS.md` §108). The distinction
/// is kept here only so a report can tell an operator which one actually
/// happened on the wire.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Occupancy {
    /// Nothing answered `T_Connect` at all: no device is at `IA_new`.
    NotOccupied,
    /// A device answered the Device Descriptor read.
    OccupiedWithResponse,
    /// The connection opened, but no Device Descriptor answer ever came
    /// back — either a `T_Disconnect` arrived instead (MP §2.3, p. 14 body
    /// text vs. p. 15 "to 1." — this project follows the exception text;
    /// see `KNOWN_LIMITATIONS.md` §108), or the descriptor read simply
    /// timed out, a case MP §2.3's text does not describe at all. Both are
    /// folded together here because both prove exactly the same thing —
    /// something answered `T_Connect` — and neither says anything more.
    OccupiedWithoutDescriptor,
    /// `T_Connect` itself got a negative confirmation, so no connection
    /// ever opened. Not in MP §2.3's text either, but a rejection still
    /// proves a device answered at the Data Link Layer, so it is treated as
    /// occupied rather than silently folded into "nobody's there".
    OccupiedWithRejectedConnect,
}

impl Occupancy {
    fn is_occupied(self) -> bool {
        !matches!(self, Occupancy::NotOccupied)
    }
}

/// What happened, step by step, running `NM_IndividualAddress_Write`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndividualAddressWriteReport {
    /// Every numbered step that ran, in order.
    pub steps: Vec<StepRecord>,
    /// Step 1's finding.
    pub occupancy: Occupancy,
    /// Whether step 3 actually broadcast `A_IndividualAddress_Write`, or
    /// skipped it because the responder already held `IA_new`.
    pub wrote: bool,
}

impl IndividualAddressWriteReport {
    fn new() -> Self {
        Self {
            steps: Vec::new(),
            occupancy: Occupancy::NotOccupied,
            wrote: false,
        }
    }
}

fn record(report: &mut IndividualAddressWriteReport, number: u8, title: &'static str) {
    report.steps.push(StepRecord {
        kind: ProcedureKind::IndividualAddressWrite,
        number,
        title,
    });
}

/// Why `NM_IndividualAddress_Write` stopped.
#[derive(Debug)]
pub enum IndividualAddressWriteError {
    /// The session refused, or the device did.
    Session(SessionError),
    /// Step 1 found `IA_new` occupied by a device other than the one step 2
    /// found in Programming Mode (MP §2.3 "to 2.", p. 15).
    OccupiedByAnotherDevice {
        /// How step 1 found it occupied.
        occupancy: Occupancy,
        /// The address the Programming Mode witness reported holding
        /// instead.
        witness_address: IndividualAddress,
    },
    /// Step 2 did not find exactly one device in Programming Mode.
    Count(ProgrammingModeCountError),
    /// Step 3's mandatory re-verification, immediately before the write,
    /// did not find exactly one device in Programming Mode — "because
    /// programming mode may have switched itself off", in `procedure.rs`'s
    /// own words for this step.
    RecountBeforeWrite(ProgrammingModeCountError),
}

impl std::error::Error for IndividualAddressWriteError {}

impl From<SessionError> for IndividualAddressWriteError {
    fn from(err: SessionError) -> Self {
        IndividualAddressWriteError::Session(err)
    }
}

impl std::fmt::Display for IndividualAddressWriteError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IndividualAddressWriteError::Session(err) => write!(f, "{err}"),
            IndividualAddressWriteError::OccupiedByAnotherDevice {
                witness_address, ..
            } => write!(
                f,
                "the new address is occupied by a device other than the one in \
                 programming mode ({witness_address}), so the procedure stops rather \
                 than write over it (MP §2.3 \"to 2.\", p. 15)"
            ),
            IndividualAddressWriteError::Count(err) => write!(f, "{err}"),
            IndividualAddressWriteError::RecountBeforeWrite(err) => {
                write!(f, "re-verifying immediately before the write: {err}")
            }
        }
    }
}

/// Runs MP §2.3 `NM_IndividualAddress_Write` end to end against `transport`.
///
/// Two authorisations, not one, because [`WriteAuthorisation`] is
/// single-scope and single-target by design (design spec §2.3):
/// `programming_authorisation` must name `new_address` and
/// [`knx_core::commissioning::mutation::WriteScope::IndividualAddressProgramming`],
/// for steps 2 and 3's broadcasts, and `restart_authorisation` must name
/// `new_address` and
/// [`knx_core::commissioning::mutation::WriteScope::Restart`], for step 4's
/// `A_Restart`. `plan` is the caller's real authorisation plan (key or none)
/// for step 4's fully authorised connection — steps 1 through 3 never
/// authorise a connection at all: step 1 connects with
/// [`AuthorisationPlan::Skip`], and steps 2 and 3 never open a connection to
/// begin with.
pub async fn individual_address_write<T: ManagementTransport>(
    transport: &T,
    plan: AuthorisationPlan,
    timing: SessionTiming,
    new_address: IndividualAddress,
    programming_authorisation: WriteAuthorisation,
    restart_authorisation: WriteAuthorisation,
) -> Result<IndividualAddressWriteReport, IndividualAddressWriteError> {
    let mut report = IndividualAddressWriteReport::new();

    // ---- Step 1: is IA_new occupied? ----
    record(&mut report, 1, "check whether the new address is occupied");
    let mut probe =
        ManagementSession::read_only(transport, new_address, AuthorisationPlan::Skip, timing)?;
    let occupancy = match probe.connect().await {
        Ok(()) => match probe.probe_device_descriptor().await {
            Ok(()) => Occupancy::OccupiedWithResponse,
            Err(
                SessionError::ConnectionLost { .. }
                | SessionError::NoAnswer { .. }
                | SessionError::ConnectionReleased { .. },
            ) => Occupancy::OccupiedWithoutDescriptor,
            Err(err) => {
                probe.disconnect().await;
                return Err(err.into());
            }
        },
        Err(SessionError::ConnectRejected { .. }) => Occupancy::OccupiedWithRejectedConnect,
        Err(SessionError::NoAnswer { .. }) => Occupancy::NotOccupied,
        Err(err) => return Err(err.into()),
    };
    probe.disconnect().await;
    report.occupancy = occupancy;

    // ---- Step 2: count devices in Programming Mode ----
    record(&mut report, 2, "count devices in programming mode");
    let counting = ManagementSession::authorised(
        transport,
        AuthorisationPlan::Skip,
        timing,
        programming_authorisation,
    )?;
    let responders = counting
        .broadcast_individual_address_read(timing.programming_mode_broadcast_timeout)
        .await?;
    let witness = responders
        .single_responder()
        .map_err(IndividualAddressWriteError::Count)?;

    if occupancy.is_occupied() && new_address != witness.current_address() {
        return Err(IndividualAddressWriteError::OccupiedByAnotherDevice {
            occupancy,
            witness_address: witness.current_address(),
        });
    }

    // ---- Step 3: write the new address, only if it is not already there ----
    record(&mut report, 3, "write the new address");
    if new_address != witness.current_address() {
        // "re-verify step 2 immediately before, because programming mode
        // may have switched itself off" — procedure.rs's own words for this
        // step.
        let recount = counting
            .broadcast_individual_address_read(timing.programming_mode_broadcast_timeout)
            .await?;
        let rewitness = recount
            .single_responder()
            .map_err(IndividualAddressWriteError::RecountBeforeWrite)?;
        if new_address != rewitness.current_address() {
            counting.broadcast_individual_address_write().await?;
            report.wrote = true;
        }
    }

    // ---- Step 4: connect to the new address, verify, and restart ----
    record(&mut report, 4, "verify and restart");
    let mut finishing =
        ManagementSession::authorised(transport, plan, timing, restart_authorisation)?;
    finishing.connect().await?;
    finishing.read_mask_version().await?;
    finishing.restart_basic().await?;

    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::super::simulator::{SimulatedDevice, SimulatorConfig};
    use super::super::SessionTiming;
    use super::*;
    use knx_core::commissioning::mutation::WriteScope;
    use knx_core::commissioning::programming_mode::ProgrammingModeCountError;
    use std::time::Duration;

    /// The cited timings with the waiting taken out: this module tests the
    /// procedure's order and its stop/continue decisions, not that 1 s is
    /// 1 s.
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

    fn authorisations(target: IndividualAddress) -> (WriteAuthorisation, WriteAuthorisation) {
        let programming =
            WriteAuthorisation::for_simulator(target, WriteScope::IndividualAddressProgramming)
                .expect("the target is not an excluded address");
        let restart = WriteAuthorisation::for_simulator(target, WriteScope::Restart)
            .expect("the target is not an excluded address");
        (programming, restart)
    }

    /// The ordinary case: `IA_new` is free, exactly one device answers in
    /// Programming Mode, and it does not already hold `IA_new` — so step 3
    /// actually writes.
    #[tokio::test]
    async fn writes_the_new_address_when_free_and_one_responder() {
        let device = SimulatedDevice::with_config(SimulatorConfig {
            programming_mode: true,
            ..Default::default()
        });
        let original = device.address();
        let new_address = addr(1, 1, 30);
        assert_ne!(
            original, new_address,
            "the fixture must actually rename the device"
        );
        let (programming, restart) = authorisations(new_address);

        let report = individual_address_write(
            &device,
            AuthorisationPlan::Skip,
            fast(),
            new_address,
            programming,
            restart,
        )
        .await
        .expect("a free address with one responder must succeed");

        assert_eq!(report.occupancy, Occupancy::NotOccupied);
        assert!(report.wrote);
        assert_eq!(report.steps.len(), 4);
        assert_eq!(report.steps[2].number, 3);
        assert_eq!(device.address(), new_address);
    }

    /// MP §2.3's own re-assignment case: `IA_new` already answers, and the
    /// answer is from the same device step 2 finds in Programming Mode.
    /// Step 3 must not broadcast a write nobody needs
    /// (`occupancy_only_stops_the_procedure_for_a_different_device` in
    /// `procedure.rs`, and `KNOWN_LIMITATIONS.md` §108).
    #[tokio::test]
    async fn skips_the_write_when_the_responder_already_holds_the_address() {
        let device = SimulatedDevice::with_config(SimulatorConfig {
            programming_mode: true,
            ..Default::default()
        });
        let new_address = device.address();
        let (programming, restart) = authorisations(new_address);

        let report = individual_address_write(
            &device,
            AuthorisationPlan::Skip,
            fast(),
            new_address,
            programming,
            restart,
        )
        .await
        .expect("re-assigning a device to its own address must succeed");

        assert_eq!(report.occupancy, Occupancy::OccupiedWithResponse);
        assert!(!report.wrote);
        assert_eq!(device.address(), new_address);
    }

    /// MP §2.3 "to 2.": `IA_new` answers, but the Programming Mode witness
    /// is a *different* address, so the procedure must stop rather than
    /// write over whoever is occupying `IA_new`.
    #[tokio::test]
    async fn stops_when_a_different_device_occupies_the_new_address() {
        let other = addr(1, 1, 40);
        let device = SimulatedDevice::with_config(SimulatorConfig {
            programming_mode: false,
            other_programming_mode_devices: vec![other],
            ..Default::default()
        });
        // The device itself sits at IA_new and answers a normal connect,
        // even though it is not the one in Programming Mode.
        let new_address = device.address();
        let (programming, restart) = authorisations(new_address);

        let err = individual_address_write(
            &device,
            AuthorisationPlan::Skip,
            fast(),
            new_address,
            programming,
            restart,
        )
        .await
        .expect_err("a different device's occupancy must stop the procedure");

        match err {
            IndividualAddressWriteError::OccupiedByAnotherDevice {
                occupancy,
                witness_address,
            } => {
                assert_eq!(occupancy, Occupancy::OccupiedWithResponse);
                assert_eq!(witness_address, other);
            }
            other => panic!("expected OccupiedByAnotherDevice, got {other:?}"),
        }
    }

    /// MP §2.3 step 2: nobody in Programming Mode is nothing to write to.
    #[tokio::test]
    async fn stops_when_nobody_is_in_programming_mode() {
        let device = SimulatedDevice::with_config(SimulatorConfig {
            programming_mode: false,
            ..Default::default()
        });
        let new_address = addr(1, 1, 30);
        let (programming, restart) = authorisations(new_address);

        let err = individual_address_write(
            &device,
            AuthorisationPlan::Skip,
            fast(),
            new_address,
            programming,
            restart,
        )
        .await
        .expect_err("no responder must stop the procedure");

        assert!(matches!(
            err,
            IndividualAddressWriteError::Count(
                ProgrammingModeCountError::NoDeviceInProgrammingMode
            )
        ));
    }

    /// MP §2.3 step 2: two buttons pressed at once is not a count this
    /// procedure may act on.
    #[tokio::test]
    async fn stops_when_several_devices_are_in_programming_mode() {
        let other = addr(1, 1, 40);
        let device = SimulatedDevice::with_config(SimulatorConfig {
            programming_mode: true,
            other_programming_mode_devices: vec![other],
            ..Default::default()
        });
        let new_address = addr(1, 1, 30);
        let (programming, restart) = authorisations(new_address);

        let err = individual_address_write(
            &device,
            AuthorisationPlan::Skip,
            fast(),
            new_address,
            programming,
            restart,
        )
        .await
        .expect_err("two responders must stop the procedure");

        match err {
            IndividualAddressWriteError::Count(ProgrammingModeCountError::SeveralDevices {
                devices,
            }) => {
                assert_eq!(devices.len(), 2);
            }
            other => panic!("expected SeveralDevices, got {other:?}"),
        }
    }

    /// `KNOWN_LIMITATIONS.md` §108: a `T_Disconnect` in place of the Device
    /// Descriptor answer does not by itself stop the procedure — it
    /// continues past step 1 and step 2 exactly as a normal response would,
    /// reaching step 4 (which then fails for the same underlying reason,
    /// proving steps 2 and 3 were not the ones that stopped it).
    #[tokio::test]
    async fn a_disconnect_at_step_one_does_not_stop_the_procedure_by_itself() {
        let device = SimulatedDevice::with_config(SimulatorConfig {
            programming_mode: true,
            device_descriptor_read_gets_disconnect: true,
            ..Default::default()
        });
        let new_address = device.address();
        let (programming, restart) = authorisations(new_address);

        let err = individual_address_write(
            &device,
            AuthorisationPlan::Skip,
            fast(),
            new_address,
            programming,
            restart,
        )
        .await
        .expect_err("this fixture also disconnects step 4's descriptor read");

        match err {
            IndividualAddressWriteError::Session(SessionError::ConnectionLost { .. }) => {}
            other => {
                panic!("expected step 4 to be the one that failed, not an early stop: {other:?}")
            }
        }
    }
}
