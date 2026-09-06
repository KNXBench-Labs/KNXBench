//! KNXnet/IP: discovery, tunnelling, routing, cEMI and telegrams.
//!
//! Cycle 1 (Session 6) implements read-only tunnelling: connect, receive,
//! decode, disconnect. Discovery and sending are explicit `NotImplemented`
//! stubs, not silent no-ops. See
//! `docs/superpowers/specs/2026-09-06-knxnet-ip-tunneling-design.md`.

pub mod cemi;
pub mod client;
pub mod core;
pub mod frame;
pub mod tunnelling;

pub use cemi::{ApplicationService, Destination, GroupValue, LDataFrame, LDataMessageKind};
pub use client::{BusConnection, BusError, KnxNetIpClient, TunnelClient};
