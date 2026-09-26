//! Proof that a hardware write is refused at construction, with no frame sent.
//!
//! This test needs no gateway and no hardware: `ManagementSession::authorised`
//! runs `check_write_target()` before anything is sent, so the refusal is
//! observable without a socket. It exists to keep design spec §15's
//! "no writes to real hardware in phase 2 or phase 3" honest — a non-goal
//! nobody tests is a non-goal that quietly stops being true.

use knx_core::commissioning::authorisation::AuthorisationPlan;
use knx_core::commissioning::mutation::{TargetKind, WriteAuthorisation, WriteScope};
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
        panic!("no frame may be sent: the session must be refused before this");
    }
}

#[test]
fn a_hardware_authorisation_cannot_build_a_writing_session() {
    let target: IndividualAddress = "1.1.67".parse().expect("literal address must parse");
    let scope = WriteScope::IndividualAddressProgramming;

    // The confirmation phrase is the one the authorisation itself demands, so
    // this is the *best* case a caller could present — not a typo being caught.
    let phrase = knx_core::commissioning::mutation::required_confirmation_phrase(target, scope);
    let authorisation = WriteAuthorisation::for_hardware(target, scope, &phrase)
        .expect("a correctly confirmed hardware authorisation must be constructible");
    assert_eq!(authorisation.kind(), TargetKind::Hardware);

    let transport = HardwareTransport::new();
    assert_eq!(transport.target_kind(), TargetKind::Hardware);

    let err = ManagementSession::authorised(
        &transport,
        AuthorisationPlan::Skip,
        SessionTiming::default(),
        authorisation,
    )
    .expect_err("a hardware target must not yield a writing session");

    match err {
        SessionError::NotASimulator {
            target: refused,
            transport: transport_kind,
            authorised,
        } => {
            assert_eq!(refused, target);
            assert_eq!(transport_kind, TargetKind::Hardware);
            assert_eq!(authorised, TargetKind::Hardware);
        }
        other => panic!("expected NotASimulator, got {other}"),
    }
}
