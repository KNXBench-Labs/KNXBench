//! The transport a management procedure needs, abstracted so no test needs a bus.
//!
//! Commissioning spec §11.1: *"A `ManagementTransport` with the same
//! fake-able shape keeps the domain testable with no socket."* The shape is
//! `ScanTransport`'s, deliberately — the two abstractions want the same
//! three capabilities, and a blanket implementation below means every
//! existing transport (and every existing fake) already satisfies this one.
//! The separate name is not ceremony: a scan probes addresses it may not
//! touch, a management session drives procedures that write, and a reader
//! of `commissioning.rs` should not have to know that the trait bound says
//! "scan".
//!
//! Nothing here talks to a device on its own. The timings are the cited
//! ones, gathered in one place so a procedure does not invent its own.

use std::time::Duration;

use knx_core::IndividualAddress;
use tokio::sync::broadcast;

use crate::cemi::{ApplicationService, Destination, Tpci};
use crate::client::{BusError, TunnelEvent};
use crate::scan::ScanTransport;

/// The acknowledge time-out of a connection-oriented exchange.
///
/// `[D]` Transport Layer v01.02.03 AS clause 4, as recorded in RESEARCH §8.5
/// and repeated in commissioning spec §11.1: 3 s to be acknowledged, 6 s
/// before an idle connection is dropped, `max_rep_count = 3`.
pub const ACKNOWLEDGE_TIMEOUT: Duration = Duration::from_secs(3);

/// How long a connection may sit idle before the device drops it.
pub const CONNECTION_TIMEOUT: Duration = Duration::from_secs(6);

/// How many times a frame is repeated before the connection is given up on.
pub const MAX_REP_COUNT: u8 = 3;

/// What a management procedure needs from its transport.
///
/// Identical in shape to [`ScanTransport`], and blanket-implemented for
/// every implementor of it, so a fake written for one serves the other.
#[allow(async_fn_in_trait)]
pub trait ManagementTransport {
    /// The individual address the gateway assigned this connection. A
    /// procedure needs it to recognise its own echoed frames — and to
    /// refuse to address itself.
    fn assigned_address(&self) -> IndividualAddress;

    /// Subscribes to telegrams and the connection-closed notice.
    ///
    /// Subscribe *before* sending, always: the answer to a management
    /// request can arrive before the send call has returned.
    fn subscribe(&self) -> broadcast::Receiver<TunnelEvent>;

    /// Sends one `L_Data.req` with the given TPCI and application service.
    async fn send_frame(
        &self,
        destination: Destination,
        transport: Tpci,
        service: ApplicationService,
    ) -> Result<(), BusError>;
}

impl<T: ScanTransport> ManagementTransport for T {
    fn assigned_address(&self) -> IndividualAddress {
        ScanTransport::assigned_address(self)
    }

    fn subscribe(&self) -> broadcast::Receiver<TunnelEvent> {
        ScanTransport::subscribe(self)
    }

    async fn send_frame(
        &self,
        destination: Destination,
        transport: Tpci,
        service: ApplicationService,
    ) -> Result<(), BusError> {
        ScanTransport::send_frame(self, destination, transport, service).await
    }
}
