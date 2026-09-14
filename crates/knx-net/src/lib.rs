//! KNXnet/IP: discovery, tunnelling, routing, cEMI and telegrams.
//!
//! Cycle 1 (Session 6) implemented read-only tunnelling: connect, receive,
//! decode, disconnect. Cycle 2 adds sending (`TunnelClient::send`) —
//! encode, `TUNNELLING_REQUEST`/`ACK` with the spec's retry-once-then-
//! disconnect rule. Cycle 3 implements `discover` (multicast
//! `SEARCH_REQUEST`/`SEARCH_RESPONSE`). Cycle 4 implements
//! `RoutingClient`/`connect_routing`. Cycle 5 hardens connection
//! management: the `wait_for_reply` retry-race fix, `TunnelEvent::Closed`
//! shutdown signaling, and `ROUTING_BUSY` throttling. KNX IP Secure
//! remains a later cycle still.
//! See `docs/superpowers/specs/2026-09-06-knxnet-ip-tunneling-design.md`.
//!
//! T17 adds `scan`: probing individual addresses on a line for occupancy
//! (`ProbePolicy`/`ProbeOutcome`/`probe_address`/`scan_line`), built on
//! `knx_core::scan::ScanPlan`'s exclusion-safe candidate list.

pub mod cemi;
pub mod client;
pub mod commissioning;
pub mod core;
pub mod discovery;
pub mod frame;
pub mod management;
pub mod routing;
pub mod scan;
pub mod tunnelling;

pub use cemi::{ApplicationService, CemiError, Destination, LDataFrame, LDataMessageKind, Tpci};
pub use client::{
    BusConnection, BusError, DiscoveredGateway, KnxNetIpClient, RoutingClient, TunnelClient,
    TunnelEvent,
};
pub use commissioning::{
    ConnectionState, ManagementSession, SessionError, SessionTiming, VerifyMode,
};
pub use management::ManagementTransport;
pub use scan::{
    probe_address, scan_line, ProbeOutcome, ProbePolicy, ProbePolicyError, ScanError, ScanTransport,
};
// `GroupValue` is a KNX domain concept (the payload of a group telegram),
// not an IP-transport one — it lives in `knx-core` (spec E4-D2) and is
// re-exported here so every existing call site keeps compiling unchanged.
pub use knx_core::GroupValue;
