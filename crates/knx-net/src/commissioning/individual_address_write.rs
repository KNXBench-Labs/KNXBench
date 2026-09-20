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
//! whether occupancy was learned from an `A_DeviceDescriptor_Response-PDU`
//! or from a `T_Disconnect` in its place — neither of which carries the
//! occupant's identity by itself. Only the Programming Mode witness's own
//! reported address answers "whose is it".
//!
//! **Step 2's `repeat` is not implemented as a loop.** MP §2.3, p. 14 reads
//! *"2. wait until device is in Programming Mode: repeat until one
//! A_IndividualAddress_Response-PDU is received … end repeat"*, and p. 13
//! says the same in prose: *"The procedure shall wait until exactly one
//! device is in Programming Mode."* This function broadcasts once and, on
//! nobody or on several, returns [`IndividualAddressWriteError::Count`] to
//! its caller instead of re-broadcasting. What the Standard's loop waits
//! for is a human walking to a device and pressing a button, which is not
//! a wait a library may impose on the thread that called it; this run also
//! forbids procedure-level retry loops outright. Written up as
//! `KNOWN_LIMITATIONS.md` §116.

use knx_core::commissioning::authorisation::AuthorisationPlan;
use knx_core::commissioning::mutation::WriteAuthorisation;
use knx_core::commissioning::procedure::ProcedureKind;
use knx_core::commissioning::programming_mode::ProgrammingModeCountError;
use knx_core::{ContactableAddress, IndividualAddress};

use super::download::StepRecord;
use super::{ManagementSession, SessionError, SessionTiming};
use crate::management::ManagementTransport;

/// How step 1 found `IA_new` occupied, if it did.
///
/// Every verdict here is MP §2.3, p. 14's own; none is this module's
/// invention. The two occupied variants stop the procedure under exactly
/// the same condition — neither occupied signal identifies the occupant, so
/// both are resolved the same way, by comparing against step 2's
/// Programming Mode witness (`KNOWN_LIMITATIONS.md` §108). The distinction
/// is kept only so a report can tell an operator which one happened on the
/// wire.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Occupancy {
    /// Nobody is at `IA_new`.
    ///
    /// Two signals, one verdict, both spelled out on p. 14. A negative
    /// confirmation for `T_Connect`: *"if negative A_Connect.Lcon ⇒ IA_new
    /// is not occupied; end procedure."* — the ordinary case on real
    /// hardware, because a free address produces no Layer-2 acknowledge.
    /// And an open connection on which the Device Descriptor read simply
    /// times out: *"If no A_DeviceDescriptor_Response-PDU is received
    /// after time-out ⇒ IA_new is not occupied"*.
    NotOccupied,
    /// A device answered the Device Descriptor read. p. 14: *"If the
    /// Management Client receives an A_DeviceDescriptor_Response-PDU it
    /// shall conclude that the Individual Address IA_new is occupied."*
    OccupiedWithResponse,
    /// The connection did not survive the Device Descriptor read: a
    /// `T_Disconnect` arrived in its place, or the Transport Layer
    /// released the connection because nothing acknowledged it.
    ///
    /// p. 14: *"if A_Disconnect-PDU is received then IA_new shall be
    /// regarded as occupied; end procedure."* p. 15, "to 1.", says why:
    /// *"a device with this Individual Address exists but it may either
    /// already have another Transport Layer connection open and not accept
    /// any further Transport Layer connections, or does not support
    /// connection oriented communication mode."* — and, in the same
    /// breath, that this is not a reason to stop: *"The Management Client
    /// shall continue with the Management Procedure in every case."*
    ///
    /// The Transport-Layer-release half is this module's reading, not a
    /// quotation: the Standard names only a *received* `A_Disconnect-PDU`.
    /// See `KNOWN_LIMITATIONS.md` §116.
    OccupiedAfterDisconnect,
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
    Session {
        /// Which of MP §2.3's four numbered steps was running. The clause
        /// gives its steps different meanings when they fail — "to 4.",
        /// p. 15, reads *"If no A_DeviceDescriptor_Response-PDU is
        /// received, than the programming of the Individual Address may
        /// have failed, or the system (Router) is not configured
        /// correctly"*, which is advice about a step 4 failure and
        /// nonsense about a step 1 one — so a caller that cannot tell them
        /// apart cannot act on the clause.
        step: u8,
        /// Everything the procedure had established before it stopped,
        /// including step 1's occupancy finding, which
        /// `KNOWN_LIMITATIONS.md` §108 undertakes to surface to the
        /// operator whether or not the procedure went on to succeed.
        report: IndividualAddressWriteReport,
        /// What went wrong.
        source: SessionError,
    },
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

/// Wraps a [`SessionError`] with the step it came from and the report as it
/// stood.
///
/// Deliberately not a `From<SessionError>` impl: the step number is not
/// derivable from the error, so an implicit `?` conversion would have to
/// invent one. Every call site names its own step, which is the only place
/// that knows it.
fn at_step(
    step: u8,
    report: &IndividualAddressWriteReport,
    source: SessionError,
) -> IndividualAddressWriteError {
    IndividualAddressWriteError::Session {
        step,
        report: report.clone(),
        source,
    }
}

impl std::fmt::Display for IndividualAddressWriteError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IndividualAddressWriteError::Session { step, source, .. } => {
                write!(f, "MP §2.3 step {step}: {source}")
            }
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
        ManagementSession::read_only(transport, new_address, AuthorisationPlan::Skip, timing)
            .map_err(|err| at_step(1, &report, err))?;
    let occupancy = match probe.connect().await {
        Ok(()) => match probe.probe_device_descriptor().await {
            Ok(()) => Occupancy::OccupiedWithResponse,
            // The connection went away in place of an answer. p. 14: *"if
            // A_Disconnect-PDU is received then IA_new shall be regarded
            // as occupied"*.
            Err(SessionError::ConnectionLost { .. } | SessionError::ConnectionReleased { .. }) => {
                Occupancy::OccupiedAfterDisconnect
            }
            // The connection is still open and nothing came. p. 14: *"If
            // no A_DeviceDescriptor_Response-PDU is received after
            // time-out ⇒ IA_new is not occupied"*.
            Err(SessionError::NoAnswer { .. }) => Occupancy::NotOccupied,
            Err(err) => {
                probe.disconnect().await;
                return Err(at_step(1, &report, err));
            }
        },
        // p. 14, the clause's own first line: *"if negative A_Connect.Lcon
        // ⇒ IA_new is not occupied"*. A free address earns no Layer-2
        // acknowledge, which is exactly what produces the negative
        // confirmation, so this is the ordinary case and not an exception.
        // Silence is the same verdict for the same reason.
        Err(SessionError::ConnectRejected { .. } | SessionError::NoAnswer { .. }) => {
            Occupancy::NotOccupied
        }
        Err(err) => return Err(at_step(1, &report, err)),
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
    )
    .map_err(|err| at_step(2, &report, err))?;
    let responders = counting
        .broadcast_individual_address_read(timing.programming_mode_broadcast_timeout)
        .await
        .map_err(|err| at_step(2, &report, err))?;
    // Nobody, or several, ends the call here rather than re-broadcasting:
    // MP §2.3's `repeat … end repeat` waits for a human, and this function
    // does not own one. See the module docs and `KNOWN_LIMITATIONS.md`
    // §116.
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
    // The device this step mutates is the one in Programming Mode, which is
    // not the address anything has checked so far: `programming_authorisation`
    // names `new_address`, the address being handed *out*, and the occupant
    // of `new_address` is a third party again. The witness's current address
    // reaches the exclusion guard here and nowhere else — design spec §2.1 puts the
    // guard at the lowest layer that knows what an individual address is,
    // and step 2's witness is the first moment this procedure knows one.
    let witness_target = ContactableAddress::new(witness.current_address())
        .map_err(|excluded| at_step(3, &report, SessionError::Excluded(excluded)))?;
    if new_address != witness_target.address() {
        // "re-verify step 2 immediately before, because programming mode
        // may have switched itself off" — procedure.rs's own words for this
        // step.
        let recount = counting
            .broadcast_individual_address_read(timing.programming_mode_broadcast_timeout)
            .await
            .map_err(|err| at_step(3, &report, err))?;
        let rewitness = recount
            .single_responder()
            .map_err(IndividualAddressWriteError::RecountBeforeWrite)?;
        // The re-verification may have found somebody else entirely, so
        // its answer goes through the same guard rather than inheriting
        // the first one's clearance.
        let rewitness_target = ContactableAddress::new(rewitness.current_address())
            .map_err(|excluded| at_step(3, &report, SessionError::Excluded(excluded)))?;
        if new_address != rewitness_target.address() {
            counting
                .broadcast_individual_address_write()
                .await
                .map_err(|err| at_step(3, &report, err))?;
            report.wrote = true;
        }
    }

    // ---- Step 4: connect to the new address, verify, and restart ----
    record(&mut report, 4, "verify and restart");
    let mut finishing =
        ManagementSession::authorised(transport, plan, timing, restart_authorisation)
            .map_err(|err| at_step(4, &report, err))?;
    if let Err(err) = verify_before_restart(&mut finishing).await {
        // MP §2.3, p. 15, the last line of the sequence: *"Abort the
        // connection of the client side Transport Layer."* It is not
        // conditioned on the verification having succeeded, and a failed
        // step 4 is precisely when a leaked connection hurts — "to 4."
        // sends the operator off to check the Routers, and the device is
        // left holding a connection that stops the next attempt from
        // opening one. [`ManagementSession::restart_basic`] does its own
        // disconnecting on every path out, so this is the only exit that
        // needed the help.
        finishing.disconnect().await;
        return Err(at_step(4, &report, err));
    }
    finishing
        .restart_basic()
        .await
        .map_err(|err| at_step(4, &report, err))?;

    Ok(report)
}

/// Step 4's half that runs under an open connection, so that its caller has
/// exactly one failure path to disconnect on.
async fn verify_before_restart<T: ManagementTransport>(
    session: &mut ManagementSession<'_, T>,
) -> Result<(), SessionError> {
    session.connect().await?;
    session.read_mask_version().await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::super::simulator::{Seen, SimulatedDevice, SimulatorConfig};
    use super::super::SessionTiming;
    use super::*;
    use knx_core::commissioning::mutation::WriteScope;
    use knx_core::commissioning::programming_mode::ProgrammingModeCountError;
    use knx_core::{ExcludedAddress, EXCLUDED_INDIVIDUAL_ADDRESSES};
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
        // Step 2's count and step 3's mandatory re-verification immediately
        // before the write (procedure.rs's own words: "programming mode may
        // have switched itself off") are two separate broadcasts, not one.
        // A step 3 that wrote on step 2's count alone would still pass every
        // other assertion here.
        assert_eq!(
            device.individual_address_read_broadcasts(),
            2,
            "step 3 must re-verify with its own broadcast, not reuse step 2's count"
        );
        // MP §2.3 step 4, p. 15: "shall ... execute a restart of the
        // device" after verifying. A step 4 that only connected and read
        // the descriptor, without ever sending A_Restart, would still pass
        // every assertion above.
        assert!(
            device
                .seen()
                .iter()
                .any(|entry| matches!(entry, Seen::Restart { .. })),
            "step 4 must restart the device, not just verify it: {:?}",
            device.seen()
        );
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
        // Step 3's re-verification exists to protect a write. With no write
        // to protect, it is a second second-long broadcast on a live bus
        // for nothing, so the skip has to cover the recount too.
        assert_eq!(
            device.individual_address_read_broadcasts(),
            1,
            "nothing is written, so nothing needs re-verifying"
        );
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

    /// MP §2.3, p. 14, the clause's very first decision: *"if negative
    /// A_Connect.Lcon ⇒ IA_new is not occupied"*. On real hardware this is
    /// the ordinary case, not an exception — a free address acknowledges
    /// nothing at Layer 2, and it is the missing acknowledge that makes
    /// the confirmation negative.
    ///
    /// The fixture keeps the device sitting at `IA_new` while refusing
    /// step 1's connect, so that reading the refusal as "occupied" is
    /// visible in more than the report: the Programming Mode witness is a
    /// different address, which would turn this run into
    /// `OccupiedByAnotherDevice` and stop the procedure that the Standard
    /// says should continue.
    #[tokio::test]
    async fn a_rejected_connect_at_step_one_means_the_address_is_free() {
        let other = addr(1, 1, 40);
        let device = SimulatedDevice::with_config(SimulatorConfig {
            programming_mode: false,
            other_programming_mode_devices: vec![other],
            // Step 1's connect only. Step 4 needs a working one.
            rejected_connects: Some(1..2),
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
        .expect("a refused T_Connect is a free address, not an occupied one");

        assert_eq!(report.occupancy, Occupancy::NotOccupied);
        assert!(
            report.wrote,
            "the witness is at {other} and IA_new is {new_address}: step 3 has work to do"
        );
    }

    /// MP §2.3, p. 14: *"If no A_DeviceDescriptor_Response-PDU is received
    /// after time-out ⇒ IA_new is not occupied"* — and, at step 4, p. 15:
    /// *"Abort the connection of the client side Transport Layer."*
    ///
    /// One fixture for both, because one fixture produces both: a device
    /// that acknowledges the Device Descriptor read and then says nothing
    /// leaves the Transport Layer connection open, which is the only state
    /// in which a missing disconnect is observable at all. Step 1 and step
    /// 4 therefore each owe the device a `T_Disconnect`, and the count is
    /// the assertion.
    #[tokio::test]
    async fn a_silent_descriptor_read_is_free_and_step_four_still_disconnects() {
        let device = SimulatedDevice::with_config(SimulatorConfig {
            programming_mode: true,
            device_descriptor_read_unanswered: true,
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
        .expect_err("step 4's verification never gets its answer either");

        match err {
            IndividualAddressWriteError::Session {
                step,
                report,
                source,
            } => {
                assert_eq!(step, 4, "MP §2.3 \"to 4.\" is advice about step 4 only");
                assert!(
                    matches!(source, SessionError::NoAnswer { .. }),
                    "an acknowledged request that goes unanswered leaves the \
                     connection open: {source:?}"
                );
                // §108's promise: the occupancy finding reaches the
                // operator whether or not the procedure finished.
                assert_eq!(report.occupancy, Occupancy::NotOccupied);
                assert!(!report.wrote);
                assert_eq!(report.steps.len(), 4);
            }
            other => panic!("expected a step-4 session failure, got {other:?}"),
        }

        let disconnects = device
            .seen()
            .iter()
            .filter(|entry| matches!(entry, Seen::Disconnect))
            .count();
        assert_eq!(
            disconnects,
            2,
            "step 1 and step 4 each abort their own connection: {:?}",
            device.seen()
        );
    }

    /// MP §2.3, p. 14: *"if A_Disconnect-PDU is received then IA_new shall
    /// be regarded as occupied"*, reached here the other way — the device
    /// acknowledges nothing, so this client's own Transport Layer releases
    /// the connection (TL §5.4.1's `A6`). The verdict is the same and the
    /// distinction is written up in `KNOWN_LIMITATIONS.md` §116.
    #[tokio::test]
    async fn an_unacknowledged_descriptor_read_at_step_one_is_occupied() {
        let device = SimulatedDevice::with_config(SimulatorConfig {
            programming_mode: true,
            silent: true,
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
        .expect_err("a device that acknowledges nothing fails step 4 as well");

        match err {
            IndividualAddressWriteError::Session { step, report, .. } => {
                assert_eq!(step, 4);
                assert_eq!(report.occupancy, Occupancy::OccupiedAfterDisconnect);
                assert!(!report.wrote, "the responder already holds IA_new");
            }
            other => panic!("expected a step-4 session failure, got {other:?}"),
        }
    }

    /// Spec §2.1: the device about to be renamed goes through
    /// [`ContactableAddress`] like everything else this crate can reach.
    ///
    /// `programming_authorisation` cannot catch this — it names
    /// `new_address`, the address being handed out, and the guard it ran
    /// was about that. The device actually being mutated is whoever is
    /// holding the programming button, and its address arrives from the
    /// bus, unchecked, at step 2.
    ///
    /// No frame is ever sent to the excluded address here: the simulator
    /// speaks its `A_IndividualAddress_Response` on its behalf, which is
    /// what a real bus would do without being asked, and step 3 refuses
    /// before anything leaves.
    #[tokio::test]
    async fn refuses_to_rename_a_device_on_the_exclusion_list() {
        let excluded = EXCLUDED_INDIVIDUAL_ADDRESSES[0];
        let device = SimulatedDevice::with_config(SimulatorConfig {
            programming_mode: false,
            other_programming_mode_devices: vec![excluded],
            ..Default::default()
        });
        let new_address = addr(1, 1, 30);
        assert_ne!(device.address(), new_address);
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
        .expect_err("the witness is on the exclusion list");

        match err {
            IndividualAddressWriteError::Session {
                step,
                source: SessionError::Excluded(ExcludedAddress(refused)),
                ..
            } => {
                assert_eq!(step, 3, "the refusal belongs to the write step");
                assert_eq!(refused, excluded);
            }
            other => panic!("expected a step-3 exclusion refusal, got {other:?}"),
        }
        assert_eq!(
            device.individual_address_read_broadcasts(),
            1,
            "step 3 must refuse before it re-verifies, let alone writes"
        );
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

    /// MP §2.3, p. 15, "to 1.": *"The Management Client shall continue with
    /// the Management Procedure in every case."* A `T_Disconnect` in place
    /// of the Device Descriptor answer is an occupancy finding and not a
    /// stop — the procedure continues past step 1 and step 2 exactly as a
    /// normal response would, reaching step 4 (which then fails for the
    /// same underlying reason, proving steps 2 and 3 were not the ones that
    /// stopped it). `KNOWN_LIMITATIONS.md` §108.
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
            IndividualAddressWriteError::Session {
                step,
                report,
                source: SessionError::ConnectionLost { .. },
            } => {
                assert_eq!(
                    step, 4,
                    "steps 2 and 3 must not have been the ones that stopped it"
                );
                assert_eq!(report.occupancy, Occupancy::OccupiedAfterDisconnect);
                assert!(!report.wrote, "the responder already holds IA_new");
            }
            other => {
                panic!("expected step 4 to be the one that failed, not an early stop: {other:?}")
            }
        }
    }
}
