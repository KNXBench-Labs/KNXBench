//! KNXnet/IP: discovery, tunnelling, routing, cEMI and telegrams.
//!
//! Cycle 1 (Session 6) implemented read-only tunnelling: connect, receive,
//! decode, disconnect. Cycle 2 adds sending (`TunnelClient::send`) —
//! encode, `TUNNELLING_REQUEST`/`ACK` with the spec's retry-once-then-
//! disconnect rule. `discover` remains an explicit `NotImplemented` stub,
//! not a silent no-op; routing and KNX IP Secure are later cycles still.
//! See `docs/superpowers/specs/2026-09-06-knxnet-ip-tunneling-design.md`.

pub mod cemi;
pub mod client;
pub mod core;
pub mod discovery;
pub mod frame;
pub mod tunnelling;

pub use cemi::{ApplicationService, Destination, GroupValue, LDataFrame, LDataMessageKind};
pub use client::{BusConnection, BusError, KnxNetIpClient, TunnelClient};
