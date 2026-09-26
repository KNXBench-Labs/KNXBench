//! The hardware write gate: two scopes are open, everything else is refused.
//!
//! Needs no gateway and no hardware — `ManagementSession::authorised` decides
//! this before anything is sent, so the whole gate is observable without a
//! socket.
//!
//! This file used to be `hardware_write_is_refused.rs` and asserted that *no*
//! write to real hardware could ever build a session. That changed on
//! 2026-09-26, deliberately and narrowly: an operator named a device and an
//! operation (assign `1.1.67` to the device held in Programming Mode), which
//! is exactly the *"fresh, specific go-ahead"* design spec §15 always
//! required. `hardware_write_is_authorised` is the allowlist; the tests below
//! hold down both of its sides, so re-closing or widening the gate cannot
//! happen quietly.

use knx_core::commissioning::authorisation::AuthorisationPlan;
use knx_core::commissioning::mutation::{
    hardware_write_is_authorised, required_confirmation_phrase, TargetKind, WriteAuthorisation,
    WriteScope,
};
use knx_core::IndividualAddress;
use knx_net::cemi::{ApplicationService, Destination, Tpci};
use knx_net::client::{BusError, TunnelEvent};
use knx_net::{ManagementSession, ManagementTransport, SessionError, SessionTiming};
use tokio::sync::broadcast;

/// A transport that says what a real `TunnelClient` says: this reaches a
/// building. `ManagementTransport::target_kind` defaults to `Hardware`, and
/// this deliberately does not override it.
struct HardwareTransport {
    events: broadcast::Sender<TunnelEvent>,
}

impl HardwareTransport {
    fn new() -> Self {
        let (events, _) = broadcast::channel(4);
        Self { events }
    }
}

impl ManagementTransport for HardwareTransport {
    fn assigned_address(&self) -> IndividualAddress {
        "1.0.255".parse().expect("literal address must parse")
    }

    fn subscribe(&self) -> broadcast::Receiver<TunnelEvent> {
        self.events.subscribe()
    }

    async fn send_frame(
        &self,
        _destination: Destination,
        _transport: Tpci,
        _service: ApplicationService,
    ) -> Result<(), BusError> {
        panic!("no frame may be sent: constructing a session must not transmit");
    }
}

fn target() -> IndividualAddress {
    "1.1.67".parse().expect("literal address must parse")
}

/// The allowlist itself, stated as a table so the intent is readable without
/// reading the gate: exactly two scopes, and the three destructive ones are
/// not among them.
#[test]
fn only_the_two_procedure_scopes_are_authorised_on_hardware() {
    assert!(hardware_write_is_authorised(
        WriteScope::IndividualAddressProgramming
    ));
    assert!(hardware_write_is_authorised(WriteScope::Restart));

    assert!(!hardware_write_is_authorised(WriteScope::Download));
    assert!(!hardware_write_is_authorised(WriteScope::Unload));
    assert!(!hardware_write_is_authorised(
        WriteScope::ProgrammingModeToggle
    ));
}

/// The permitting side: the address write MP §2.3 performs does build a
/// writing session against a hardware transport.
#[test]
fn the_address_write_scope_builds_a_writing_session() {
    let scope = WriteScope::IndividualAddressProgramming;
    let phrase = required_confirmation_phrase(target(), scope);
    let authorisation = WriteAuthorisation::for_hardware(target(), scope, &phrase)
        .expect("a correctly confirmed hardware authorisation must be constructible");
    assert_eq!(authorisation.kind(), TargetKind::Hardware);

    let transport = HardwareTransport::new();
    let session = ManagementSession::authorised(
        &transport,
        AuthorisationPlan::Skip,
        SessionTiming::default(),
        authorisation,
    )
    .expect("an operator-authorised address write must build against hardware");
    assert!(session.may_write());
}

/// The refusing side, and the one that matters most: a *fully confirmed*
/// hardware authorisation for a download is still refused. The operator
/// authorised an address change, not a re-flash.
#[test]
fn a_download_to_hardware_is_still_refused() {
    let scope = WriteScope::Download;
    let phrase = required_confirmation_phrase(target(), scope);
    let authorisation = WriteAuthorisation::for_hardware(target(), scope, &phrase)
        .expect("the phrase is the required one");

    let transport = HardwareTransport::new();
    let err = ManagementSession::authorised(
        &transport,
        AuthorisationPlan::Skip,
        SessionTiming::default(),
        authorisation,
    )
    .expect_err("a download to hardware must not yield a writing session");

    match err {
        SessionError::NotASimulator {
            target: refused,
            transport: transport_kind,
            authorised,
        } => {
            assert_eq!(refused, target());
            assert_eq!(transport_kind, TargetKind::Hardware);
            assert_eq!(authorised, TargetKind::Hardware);
        }
        other => panic!("expected NotASimulator, got {other}"),
    }
}

/// A simulator authorisation cannot be pointed at hardware even for an
/// allowlisted scope: the confirmation phrase would never have been typed,
/// so the operator never named this device.
#[test]
fn a_simulator_authorisation_cannot_write_to_hardware() {
    let authorisation =
        WriteAuthorisation::for_simulator(target(), WriteScope::IndividualAddressProgramming)
            .expect("the target is contactable");
    assert_eq!(authorisation.kind(), TargetKind::Simulator);

    let transport = HardwareTransport::new();
    let err = ManagementSession::authorised(
        &transport,
        AuthorisationPlan::Skip,
        SessionTiming::default(),
        authorisation,
    )
    .expect_err("an unconfirmed simulator authorisation must not reach hardware");
    assert!(matches!(err, SessionError::NotASimulator { .. }));
}

/// The excluded alarm panel is refused before any of this is consulted, and
/// the scope allowlist does not create a way around it.
#[test]
fn the_alarm_panel_is_refused_even_for_an_authorised_scope() {
    let panel: IndividualAddress = "1.1.220".parse().expect("literal address must parse");
    let scope = WriteScope::IndividualAddressProgramming;
    let phrase = required_confirmation_phrase(panel, scope);

    // The refusal happens in `WriteAuthorisation` itself, so no session and
    // no transport are ever involved.
    WriteAuthorisation::for_hardware(panel, scope, &phrase)
        .expect_err("the alarm panel must never be authorisable");
}
