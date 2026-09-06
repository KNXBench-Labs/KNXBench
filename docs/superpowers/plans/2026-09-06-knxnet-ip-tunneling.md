# KNXnet/IP Tunnelling (Read-Only) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Connect to a real KNXnet/IP gateway over UDP tunnelling, receive KNX telegrams, decode them, and print them resolved against a loaded project's group address names, from a new `knx bus monitor` CLI subcommand.

**Architecture:** `crates/knx-net` gets five pure, IO-free codec modules (`frame`, `core::hpai`, `core::services`, `tunnelling`, `cemi`) each unit-tested against byte layouts taken directly from the KNX Association specification, plus one IO module (`client`) that drives a `tokio` UDP socket through the connect/heartbeat/receive/disconnect lifecycle and exposes a `BusConnection` trait (`discover`, `connect_tunnel`) and a `TunnelClient` handle (`send`, `subscribe`). `apps/knx-cli` gets a `bus monitor` subcommand wiring it all together.

**Tech Stack:** Rust, `tokio` (UDP socket, timers, `broadcast` channel), no new external dependency beyond `tokio` (already a workspace dependency).

**Spec:** [docs/superpowers/specs/2026-09-06-knxnet-ip-tunneling-design.md](../specs/2026-09-06-knxnet-ip-tunneling-design.md)

## Global Constraints

* Byte layouts cite KNX Association specification section numbers in comments (e.g. `Core v01.06.02 AS §7.5.1`); the spec text itself is never copied into this repository (RESEARCH.md §10, spec doc's "Handling rule").
* No new crate dependencies beyond `tokio` feature additions — no `async-trait` (Rust 1.98 supports native `async fn` in traits for the non-`dyn` usage this cycle needs), no `thiserror`/`anyhow` (this workspace hand-rolls `Display`/`Error` impls — see `crates/knx-core/src/address.rs`'s `AddressError` for the existing convention).
* `xtask check-layering`'s `tokio` ban applies only to `knx-core`/`knx-projection`; `knx-net` is infrastructure and is unaffected.
* Discovery (`SEARCH_REQUEST`) and sending (`GroupValueWrite` out) are explicitly out of scope this cycle — both return `BusError::NotImplemented`, never `todo!()`/panic and never a silent no-op.
* Every module's error type has a hand-written `Display` + `std::error::Error` impl, matching `AddressError`'s style.
* `cargo fmt --all --check` and `cargo clippy --workspace --all-targets -- -D warnings` must stay clean after every task.

---

### Task 1: `knx-net` crate scaffolding

**Files:**
- Modify: `crates/knx-net/Cargo.toml`
- Modify: `crates/knx-net/src/lib.rs`

**Interfaces:**
- Consumes: nothing (first task).
- Produces: module declarations (`frame`, `core`, `tunnelling`, `cemi`, `client`) that later tasks fill in. Nothing to re-export yet — `lib.rs`'s `pub use client::...` line is added in Task 7 once those types exist.

- [ ] **Step 1: Add `tokio` to `knx-net`'s dependencies**

```toml
[dependencies]
knx-core.workspace = true
tokio = { workspace = true, features = ["net", "sync", "time", "macros", "rt"] }
```

(`net` for `UdpSocket`, `sync` for `broadcast`/`Notify`/`Mutex`, `time` for `timeout`/`interval`, `macros` for `select!`/`#[tokio::test]`, `rt` for the test runtime and `Builder` used by callers.)

- [ ] **Step 2: Declare the module tree in `lib.rs`**

```rust
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
```

- [ ] **Step 3: Verify it builds (with empty submodules failing to resolve is expected — fix by stubbing each module file with just its doc comment)**

Create empty-but-valid files so the crate compiles before any real content lands:

`crates/knx-net/src/frame.rs`:
```rust
//! Generic KNXnet/IP frame header (Core v01.06.02 AS §2.3, §7.1/§7.2).
```

`crates/knx-net/src/core/mod.rs`:
```rust
//! Host-protocol-independent KNXnet/IP structures (Core v01.06.02 AS §7).

pub mod hpai;
pub mod services;
```

`crates/knx-net/src/core/hpai.rs`:
```rust
//! Host Protocol Address Information, IPv4/UDP form (Core v01.06.02 AS
//! §7.5.1, §8.6.2.1).
```

`crates/knx-net/src/core/services.rs`:
```rust
//! Core connection-management services: CONNECT/CONNECTIONSTATE/DISCONNECT
//! (Core v01.06.02 AS §7.4, §7.8).
```

`crates/knx-net/src/tunnelling.rs`:
```rust
//! Tunnelling-specific CRI/CRD and TUNNELLING_REQUEST/ACK (Tunnelling
//! v01.07.01 AS §5.4).
```

`crates/knx-net/src/cemi.rs`:
```rust
//! cEMI `L_Data` frame decode (EMI_IMI v01.04.02 AS §4.1.5.3).
```

`crates/knx-net/src/client.rs`:
```rust
//! The `BusConnection` trait and its tunnelling implementation
//! (ARCHITECTURE.md §8).
```

Run: `cargo build -p knx-net`
Expected: builds cleanly (empty modules, no items yet).

- [ ] **Step 4: Commit**

```bash
git add crates/knx-net
git commit -m "feat(knx-net): scaffold module tree for tunnelling cycle

Co-Authored-By: Claude Sonnet 5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_019Armveec8wNNuGLVN7WhQv"
```

---

### Task 2: `frame` — generic KNXnet/IP header

**Files:**
- Modify: `crates/knx-net/src/frame.rs`

**Interfaces:**
- Consumes: nothing.
- Produces: `pub const HEADER_SIZE_10: u8`, `pub const KNXNETIP_VERSION_10: u8`, `pub struct FrameHeader { pub service_type: u16, pub total_length: u16 }`, `pub enum FrameError`, `pub fn encode_frame(service_type: u16, body: &[u8]) -> Vec<u8>`, `pub fn decode_frame(buf: &[u8]) -> Result<(FrameHeader, &[u8]), FrameError>`. Every later task that builds a full datagram calls `encode_frame`/`decode_frame`.

- [ ] **Step 1: Write the failing tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_then_decode_round_trips() {
        let body = [0xAAu8, 0xBB, 0xCC];
        let datagram = encode_frame(0x0203, &body);
        assert_eq!(datagram, vec![0x06, 0x10, 0x02, 0x03, 0x00, 0x09, 0xAA, 0xBB, 0xCC]);

        let (header, decoded_body) = decode_frame(&datagram).unwrap();
        assert_eq!(header.service_type, 0x0203);
        assert_eq!(header.total_length, 9);
        assert_eq!(decoded_body, &body);
    }

    #[test]
    fn decode_rejects_short_buffer() {
        let err = decode_frame(&[0x06, 0x10, 0x02]).unwrap_err();
        assert_eq!(err, FrameError::TooShort { needed: 6, got: 3 });
    }

    #[test]
    fn decode_rejects_bad_header_length() {
        let err = decode_frame(&[0x07, 0x10, 0x02, 0x03, 0x00, 0x06]).unwrap_err();
        assert_eq!(err, FrameError::BadHeaderLength(0x07));
    }

    #[test]
    fn decode_rejects_unsupported_version() {
        let err = decode_frame(&[0x06, 0x20, 0x02, 0x03, 0x00, 0x06]).unwrap_err();
        assert_eq!(err, FrameError::UnsupportedVersion(0x20));
    }

    #[test]
    fn decode_rejects_length_mismatch() {
        // total_length field says 9, but the buffer is only 6 octets.
        let err = decode_frame(&[0x06, 0x10, 0x02, 0x03, 0x00, 0x09]).unwrap_err();
        assert_eq!(
            err,
            FrameError::LengthMismatch { header_said: 9, actual: 6 }
        );
    }
}
```

- [ ] **Step 2: Run to verify it fails**

Run: `cargo test -p knx-net frame::`
Expected: FAIL — `encode_frame`/`decode_frame`/`FrameHeader`/`FrameError` not found.

- [ ] **Step 3: Implement**

```rust
//! Generic KNXnet/IP frame header (Core v01.06.02 AS §2.3, §7.1/§7.2).
//! Pure byte <-> struct, no IO. Byte order is big-endian throughout
//! (Core v01.06.02 AS §2.1.2).

/// Core v01.06.02 AS §7.2.1 — identifies the header as protocol version 1.0.
pub const HEADER_SIZE_10: u8 = 0x06;
/// Core v01.06.02 AS §7.2.2 — the only protocol version this implements.
pub const KNXNETIP_VERSION_10: u8 = 0x10;

/// The common KNXnet/IP header (Core v01.06.02 AS §7.1, Figure 8):
/// header length, protocol version, service type, total length. Header
/// length and protocol version are fixed constants in this cycle (only
/// version 1.0 is implemented), so only the two variable fields are kept.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameHeader {
    pub service_type: u16,
    pub total_length: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameError {
    TooShort { needed: usize, got: usize },
    BadHeaderLength(u8),
    UnsupportedVersion(u8),
    LengthMismatch { header_said: u16, actual: usize },
}

impl std::fmt::Display for FrameError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FrameError::TooShort { needed, got } => {
                write!(f, "frame too short: needed {needed} octets, got {got}")
            }
            FrameError::BadHeaderLength(len) => {
                write!(f, "unexpected header length {len:#04x}, expected {HEADER_SIZE_10:#04x}")
            }
            FrameError::UnsupportedVersion(v) => {
                write!(f, "unsupported KNXnet/IP protocol version {v:#04x}")
            }
            FrameError::LengthMismatch { header_said, actual } => {
                write!(f, "header says {header_said} octets, buffer has {actual}")
            }
        }
    }
}

impl std::error::Error for FrameError {}

/// Builds a complete KNXnet/IP datagram: header followed by `body`.
pub fn encode_frame(service_type: u16, body: &[u8]) -> Vec<u8> {
    let total_length = (HEADER_SIZE_10 as usize + body.len()) as u16;
    let mut out = Vec::with_capacity(6 + body.len());
    out.push(HEADER_SIZE_10);
    out.push(KNXNETIP_VERSION_10);
    out.extend_from_slice(&service_type.to_be_bytes());
    out.extend_from_slice(&total_length.to_be_bytes());
    out.extend_from_slice(body);
    out
}

/// Splits a raw datagram into its header and body slice.
pub fn decode_frame(buf: &[u8]) -> Result<(FrameHeader, &[u8]), FrameError> {
    if buf.len() < 6 {
        return Err(FrameError::TooShort { needed: 6, got: buf.len() });
    }
    let header_length = buf[0];
    if header_length != HEADER_SIZE_10 {
        return Err(FrameError::BadHeaderLength(header_length));
    }
    let version = buf[1];
    if version != KNXNETIP_VERSION_10 {
        return Err(FrameError::UnsupportedVersion(version));
    }
    let service_type = u16::from_be_bytes([buf[2], buf[3]]);
    let total_length = u16::from_be_bytes([buf[4], buf[5]]);
    if total_length as usize != buf.len() {
        return Err(FrameError::LengthMismatch {
            header_said: total_length,
            actual: buf.len(),
        });
    }
    Ok((FrameHeader { service_type, total_length }, &buf[6..]))
}
```

- [ ] **Step 4: Run to verify it passes**

Run: `cargo test -p knx-net frame::`
Expected: PASS (5 tests).

- [ ] **Step 5: Commit**

```bash
git add crates/knx-net/src/frame.rs
git commit -m "feat(knx-net): generic KNXnet/IP frame header encode/decode

Co-Authored-By: Claude Sonnet 5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_019Armveec8wNNuGLVN7WhQv"
```

---

### Task 3: `core::hpai` — Host Protocol Address Information

**Files:**
- Modify: `crates/knx-net/src/core/hpai.rs`

**Interfaces:**
- Consumes: nothing.
- Produces: `pub const IPV4_UDP: u8`, `pub const HPAI_LEN: u8`, `pub struct Hpai { pub addr: std::net::Ipv4Addr, pub port: u16 }` with `pub fn encode(self) -> [u8; 8]` and `pub fn decode(buf: &[u8]) -> Result<(Hpai, &[u8]), HpaiError>`, `pub enum HpaiError`. Used by `core::services` (Task 4) and `client` (Task 7) wherever a CONNECT/CONNECTIONSTATE/DISCONNECT frame carries a return address.

- [ ] **Step 1: Write the failing tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::net::Ipv4Addr;

    #[test]
    fn encode_then_decode_round_trips() {
        let hpai = Hpai { addr: Ipv4Addr::new(172, 18, 250, 1), port: 3671 };
        let bytes = hpai.encode();
        assert_eq!(bytes, [0x08, 0x01, 172, 18, 250, 1, 0x0E, 0x57]);

        let (decoded, rest) = Hpai::decode(&bytes).unwrap();
        assert_eq!(decoded, hpai);
        assert!(rest.is_empty());
    }

    #[test]
    fn decode_leaves_trailing_bytes_for_the_caller() {
        let mut bytes = Hpai { addr: Ipv4Addr::new(10, 0, 0, 1), port: 1 }.encode().to_vec();
        bytes.extend_from_slice(&[0xFF, 0xEE]);
        let (_, rest) = Hpai::decode(&bytes).unwrap();
        assert_eq!(rest, &[0xFF, 0xEE]);
    }

    #[test]
    fn decode_rejects_short_buffer() {
        let err = Hpai::decode(&[0x08, 0x01, 1, 2, 3]).unwrap_err();
        assert_eq!(err, HpaiError::TooShort { needed: 8, got: 5 });
    }

    #[test]
    fn decode_rejects_bad_length() {
        let err = Hpai::decode(&[0x09, 0x01, 1, 2, 3, 4, 0, 0]).unwrap_err();
        assert_eq!(err, HpaiError::BadLength(0x09));
    }

    #[test]
    fn decode_rejects_tcp_host_protocol() {
        // IPV4_TCP (02h) is only valid in the all-zero "Route Back" form
        // (Core v01.06.02 AS §8.6.2.2), which this cycle does not support.
        let err = Hpai::decode(&[0x08, 0x02, 0, 0, 0, 0, 0, 0]).unwrap_err();
        assert_eq!(err, HpaiError::UnsupportedHostProtocol(0x02));
    }
}
```

- [ ] **Step 2: Run to verify it fails**

Run: `cargo test -p knx-net core::hpai::`
Expected: FAIL — nothing defined yet.

- [ ] **Step 3: Implement**

```rust
//! Host Protocol Address Information, IPv4/UDP form (Core v01.06.02 AS
//! §7.5.1, §8.6.2.1, Table 10). This cycle implements only `IPV4_UDP`;
//! `IPV4_TCP` is restricted to the all-zero "Route Back" encoding
//! (§8.6.2.2), which this cycle's single-gateway UDP client never needs.

use std::net::Ipv4Addr;

pub const IPV4_UDP: u8 = 0x01;
pub const HPAI_LEN: u8 = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Hpai {
    pub addr: Ipv4Addr,
    pub port: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HpaiError {
    TooShort { needed: usize, got: usize },
    BadLength(u8),
    UnsupportedHostProtocol(u8),
}

impl std::fmt::Display for HpaiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HpaiError::TooShort { needed, got } => {
                write!(f, "HPAI too short: needed {needed} octets, got {got}")
            }
            HpaiError::BadLength(len) => {
                write!(f, "unexpected HPAI structure length {len:#04x}, expected {HPAI_LEN:#04x}")
            }
            HpaiError::UnsupportedHostProtocol(code) => {
                write!(f, "unsupported host protocol code {code:#04x}")
            }
        }
    }
}

impl std::error::Error for HpaiError {}

impl Hpai {
    pub fn encode(self) -> [u8; 8] {
        let mut out = [0u8; 8];
        out[0] = HPAI_LEN;
        out[1] = IPV4_UDP;
        out[2..6].copy_from_slice(&self.addr.octets());
        out[6..8].copy_from_slice(&self.port.to_be_bytes());
        out
    }

    /// Decodes one HPAI from the start of `buf`, returning it along with
    /// whatever octets follow it (callers building up a larger frame body
    /// keep decoding from that remainder).
    pub fn decode(buf: &[u8]) -> Result<(Self, &[u8]), HpaiError> {
        if buf.len() < HPAI_LEN as usize {
            return Err(HpaiError::TooShort { needed: HPAI_LEN as usize, got: buf.len() });
        }
        let len = buf[0];
        if len != HPAI_LEN {
            return Err(HpaiError::BadLength(len));
        }
        let code = buf[1];
        if code != IPV4_UDP {
            return Err(HpaiError::UnsupportedHostProtocol(code));
        }
        let addr = Ipv4Addr::new(buf[2], buf[3], buf[4], buf[5]);
        let port = u16::from_be_bytes([buf[6], buf[7]]);
        Ok((Hpai { addr, port }, &buf[8..]))
    }
}
```

- [ ] **Step 4: Run to verify it passes**

Run: `cargo test -p knx-net core::hpai::`
Expected: PASS (5 tests).

- [ ] **Step 5: Commit**

```bash
git add crates/knx-net/src/core/hpai.rs
git commit -m "feat(knx-net): HPAI (IPv4/UDP) encode/decode

Co-Authored-By: Claude Sonnet 5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_019Armveec8wNNuGLVN7WhQv"
```

---

### Task 4: `core::services` — CONNECT / CONNECTIONSTATE / DISCONNECT

**Files:**
- Modify: `crates/knx-net/src/core/services.rs`

**Interfaces:**
- Consumes: `super::hpai::{Hpai, HpaiError}` (Task 3).
- Produces: service-type constants (`CONNECT_REQUEST`, `CONNECT_RESPONSE`, `CONNECTIONSTATE_REQUEST`, `CONNECTIONSTATE_RESPONSE`, `DISCONNECT_REQUEST`, `DISCONNECT_RESPONSE`: all `u16`), status constant `E_NO_ERROR: u8`, `pub struct ConnectResponse { pub channel_id: u8, pub status: u8, pub server_data_hpai: Option<Hpai>, pub crd: Vec<u8> }`, `pub struct ConnectionStateResponse { pub channel_id: u8, pub status: u8 }`, `pub struct DisconnectResponse { pub channel_id: u8, pub status: u8 }`, `pub enum ServiceError`, and:
  - `pub fn encode_connect_request(control: Hpai, cri: &[u8], data: Hpai) -> Vec<u8>`
  - `pub fn decode_connect_response(body: &[u8]) -> Result<ConnectResponse, ServiceError>`
  - `pub fn encode_connectionstate_request(channel_id: u8, control: Hpai) -> Vec<u8>`
  - `pub fn decode_connectionstate_response(body: &[u8]) -> Result<ConnectionStateResponse, ServiceError>`
  - `pub fn encode_disconnect_request(channel_id: u8, control: Hpai) -> Vec<u8>`
  - `pub fn decode_disconnect_response(body: &[u8]) -> Result<DisconnectResponse, ServiceError>`
  - `pub fn encode_disconnect_response(channel_id: u8, status: u8) -> Vec<u8>`

  All six are consumed by `client.rs` (Task 7).

- [ ] **Step 1: Write the failing tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::net::Ipv4Addr;

    fn hpai() -> Hpai {
        Hpai { addr: Ipv4Addr::new(10, 0, 0, 5), port: 50000 }
    }

    #[test]
    fn connect_request_lays_out_control_hpai_cri_data_hpai_in_order() {
        let cri = [0x04, 0x04, 0x02, 0x00]; // opaque to this module
        let body = encode_connect_request(hpai(), &cri, hpai());
        let mut expected = hpai().encode().to_vec();
        expected.extend_from_slice(&cri);
        expected.extend_from_slice(&hpai().encode());
        assert_eq!(body, expected);
    }

    #[test]
    fn connect_response_success_decodes_hpai_and_crd() {
        let mut body = vec![0x15, E_NO_ERROR];
        body.extend_from_slice(&hpai().encode());
        body.extend_from_slice(&[0x04, 0x04, 0x11, 0x22]); // opaque CRD bytes
        let response = decode_connect_response(&body).unwrap();
        assert_eq!(response.channel_id, 0x15);
        assert_eq!(response.status, E_NO_ERROR);
        assert_eq!(response.server_data_hpai, Some(hpai()));
        assert_eq!(response.crd, vec![0x04, 0x04, 0x11, 0x22]);
    }

    #[test]
    fn connect_response_failure_has_no_hpai_or_crd() {
        let body = vec![0x00, E_CONNECTION_TYPE];
        let response = decode_connect_response(&body).unwrap();
        assert_eq!(response.status, E_CONNECTION_TYPE);
        assert_eq!(response.server_data_hpai, None);
        assert!(response.crd.is_empty());
    }

    #[test]
    fn connectionstate_request_lays_out_channel_id_reserved_hpai() {
        let body = encode_connectionstate_request(0x15, hpai());
        let mut expected = vec![0x15, 0x00];
        expected.extend_from_slice(&hpai().encode());
        assert_eq!(body, expected);
    }

    #[test]
    fn connectionstate_response_decodes_channel_and_status() {
        let response = decode_connectionstate_response(&[0x15, E_NO_ERROR]).unwrap();
        assert_eq!(response, ConnectionStateResponse { channel_id: 0x15, status: E_NO_ERROR });
    }

    #[test]
    fn disconnect_request_lays_out_channel_id_reserved_hpai() {
        let body = encode_disconnect_request(0x15, hpai());
        let mut expected = vec![0x15, 0x00];
        expected.extend_from_slice(&hpai().encode());
        assert_eq!(body, expected);
    }

    #[test]
    fn disconnect_response_round_trips() {
        let body = encode_disconnect_response(0x15, E_NO_ERROR);
        assert_eq!(body, vec![0x15, E_NO_ERROR]);
        let response = decode_disconnect_response(&body).unwrap();
        assert_eq!(response, DisconnectResponse { channel_id: 0x15, status: E_NO_ERROR });
    }

    #[test]
    fn decode_rejects_short_body() {
        let err = decode_connectionstate_response(&[0x15]).unwrap_err();
        assert_eq!(err, ServiceError::TooShort { needed: 2, got: 1 });
    }

    #[test]
    fn service_type_constants_match_the_spec() {
        assert_eq!(CONNECT_REQUEST, 0x0205);
        assert_eq!(CONNECT_RESPONSE, 0x0206);
        assert_eq!(CONNECTIONSTATE_REQUEST, 0x0207);
        assert_eq!(CONNECTIONSTATE_RESPONSE, 0x0208);
        assert_eq!(DISCONNECT_REQUEST, 0x0209);
        assert_eq!(DISCONNECT_RESPONSE, 0x020A);
    }
}
```

- [ ] **Step 2: Run to verify it fails**

Run: `cargo test -p knx-net core::services::`
Expected: FAIL — nothing defined yet.

- [ ] **Step 3: Implement**

```rust
//! Core connection-management services: CONNECT/CONNECTIONSTATE/DISCONNECT
//! (Core v01.06.02 AS §7.4.1, §7.8). The CRI/CRD bodies these frames wrap
//! are connection-type specific (§7.5.2/§7.5.3 defer to the connection
//! type's own clause) — this module treats them as opaque byte slices;
//! `tunnelling.rs` owns their actual layout.

use super::hpai::Hpai;

// Service type identifiers, Core v01.06.02 AS §7.4.1.
pub const CONNECT_REQUEST: u16 = 0x0205;
pub const CONNECT_RESPONSE: u16 = 0x0206;
pub const CONNECTIONSTATE_REQUEST: u16 = 0x0207;
pub const CONNECTIONSTATE_RESPONSE: u16 = 0x0208;
pub const DISCONNECT_REQUEST: u16 = 0x0209;
pub const DISCONNECT_RESPONSE: u16 = 0x020A;

// Status codes actually produced/consumed by this cycle's client, Core
// v01.06.02 AS §7.3, Table 8, Table 9.
pub const E_NO_ERROR: u8 = 0x00;
pub const E_CONNECTION_TYPE: u8 = 0x22;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectResponse {
    pub channel_id: u8,
    pub status: u8,
    pub server_data_hpai: Option<Hpai>,
    pub crd: Vec<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConnectionStateResponse {
    pub channel_id: u8,
    pub status: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DisconnectResponse {
    pub channel_id: u8,
    pub status: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServiceError {
    TooShort { needed: usize, got: usize },
    Hpai(super::hpai::HpaiError),
}

impl std::fmt::Display for ServiceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ServiceError::TooShort { needed, got } => {
                write!(f, "service body too short: needed {needed} octets, got {got}")
            }
            ServiceError::Hpai(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for ServiceError {}

impl From<super::hpai::HpaiError> for ServiceError {
    fn from(e: super::hpai::HpaiError) -> Self {
        ServiceError::Hpai(e)
    }
}

/// Core v01.06.02 AS §7.8.1, Figure 35: client control HPAI, CRI, client
/// data HPAI, in that order.
pub fn encode_connect_request(control: Hpai, cri: &[u8], data: Hpai) -> Vec<u8> {
    let mut body = Vec::with_capacity(8 + cri.len() + 8);
    body.extend_from_slice(&control.encode());
    body.extend_from_slice(cri);
    body.extend_from_slice(&data.encode());
    body
}

/// Core v01.06.02 AS §7.8.2, Figure 36: channel ID, status, then — only if
/// `status == E_NO_ERROR` — the server's data HPAI and the CRD.
pub fn decode_connect_response(body: &[u8]) -> Result<ConnectResponse, ServiceError> {
    if body.len() < 2 {
        return Err(ServiceError::TooShort { needed: 2, got: body.len() });
    }
    let channel_id = body[0];
    let status = body[1];
    if status != E_NO_ERROR {
        return Ok(ConnectResponse { channel_id, status, server_data_hpai: None, crd: Vec::new() });
    }
    let (server_data_hpai, rest) = Hpai::decode(&body[2..])?;
    Ok(ConnectResponse {
        channel_id,
        status,
        server_data_hpai: Some(server_data_hpai),
        crd: rest.to_vec(),
    })
}

/// Core v01.06.02 AS §7.8.3: channel ID, reserved octet, client control HPAI.
pub fn encode_connectionstate_request(channel_id: u8, control: Hpai) -> Vec<u8> {
    let mut body = Vec::with_capacity(2 + 8);
    body.push(channel_id);
    body.push(0x00);
    body.extend_from_slice(&control.encode());
    body
}

/// Core v01.06.02 AS §7.8.4: channel ID, status.
pub fn decode_connectionstate_response(body: &[u8]) -> Result<ConnectionStateResponse, ServiceError> {
    let (channel_id, status) = decode_channel_and_status(body)?;
    Ok(ConnectionStateResponse { channel_id, status })
}

/// Core v01.06.02 AS §7.8.5: channel ID, reserved octet, client control HPAI.
pub fn encode_disconnect_request(channel_id: u8, control: Hpai) -> Vec<u8> {
    let mut body = Vec::with_capacity(2 + 8);
    body.push(channel_id);
    body.push(0x00);
    body.extend_from_slice(&control.encode());
    body
}

/// Core v01.06.02 AS §7.8.6: channel ID, status.
pub fn decode_disconnect_response(body: &[u8]) -> Result<DisconnectResponse, ServiceError> {
    let (channel_id, status) = decode_channel_and_status(body)?;
    Ok(DisconnectResponse { channel_id, status })
}

/// Core v01.06.02 AS §7.8.6: channel ID, status — same shape as the
/// request's response is expected to send back when we are the one being
/// asked to disconnect.
pub fn encode_disconnect_response(channel_id: u8, status: u8) -> Vec<u8> {
    vec![channel_id, status]
}

fn decode_channel_and_status(body: &[u8]) -> Result<(u8, u8), ServiceError> {
    if body.len() < 2 {
        return Err(ServiceError::TooShort { needed: 2, got: body.len() });
    }
    Ok((body[0], body[1]))
}
```

- [ ] **Step 4: Run to verify it passes**

Run: `cargo test -p knx-net core::services::`
Expected: PASS (9 tests).

- [ ] **Step 5: Commit**

```bash
git add crates/knx-net/src/core/services.rs
git commit -m "feat(knx-net): CONNECT/CONNECTIONSTATE/DISCONNECT service bodies

Co-Authored-By: Claude Sonnet 5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_019Armveec8wNNuGLVN7WhQv"
```

---

### Task 5: `tunnelling` — CRI/CRD and TUNNELLING_REQUEST/ACK

**Files:**
- Modify: `crates/knx-net/src/tunnelling.rs`

**Interfaces:**
- Consumes: `knx_core::IndividualAddress`.
- Produces: `pub const TUNNEL_CONNECTION: u8`, `pub const TUNNEL_LINKLAYER: u8`, `pub const TUNNELLING_REQUEST: u16`, `pub const TUNNELLING_ACK: u16`, `pub const E_NO_ERROR: u8`, `pub struct TunnelCri { pub layer: u8 }` with `encode(self) -> [u8; 4]`, `pub struct TunnelCrd { pub individual_address: IndividualAddress }` with `decode(buf) -> Result<Self, TunnellingError>`, `pub struct TunnellingRequest<'a> { pub channel_id: u8, pub sequence_counter: u8, pub cemi: &'a [u8] }`, `pub fn encode_tunnelling_request(channel_id: u8, sequence_counter: u8, cemi: &[u8]) -> Vec<u8>`, `pub fn decode_tunnelling_request(body: &[u8]) -> Result<TunnellingRequest<'_>, TunnellingError>`, `pub struct TunnellingAck { pub channel_id: u8, pub sequence_counter: u8, pub status: u8 }`, `pub fn encode_tunnelling_ack(channel_id: u8, sequence_counter: u8, status: u8) -> [u8; 4]`, `pub fn decode_tunnelling_ack(body: &[u8]) -> Result<TunnellingAck, TunnellingError>`, `pub enum TunnellingError`. Consumed by `client.rs` (Task 7).

- [ ] **Step 1: Write the failing tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use knx_core::IndividualAddress;

    #[test]
    fn basic_cri_encodes_layer_and_connection_type() {
        let cri = TunnelCri { layer: TUNNEL_LINKLAYER };
        assert_eq!(cri.encode(), [0x04, TUNNEL_CONNECTION, TUNNEL_LINKLAYER, 0x00]);
    }

    #[test]
    fn crd_decodes_the_assigned_individual_address() {
        let crd = TunnelCrd::decode(&[0x04, TUNNEL_CONNECTION, 0x11, 0xFF]).unwrap();
        assert_eq!(crd.individual_address, IndividualAddress::from_raw(0x11FF));
    }

    #[test]
    fn crd_rejects_wrong_connection_type() {
        let err = TunnelCrd::decode(&[0x04, 0x03, 0x11, 0xFF]).unwrap_err();
        assert_eq!(err, TunnellingError::UnexpectedConnectionType(0x03));
    }

    /// The exact worked example from Tunnelling v01.07.01 AS §6.2, Figure
    /// 16: a full TUNNELLING_ACK datagram, communication channel ID 0x15,
    /// sequence counter 0, status E_NO_ERROR.
    #[test]
    fn tunnelling_ack_matches_the_spec_worked_example() {
        let datagram: [u8; 10] = [0x06, 0x10, 0x04, 0x21, 0x00, 0x0A, 0x04, 0x15, 0x00, 0x00];
        let (header, body) = crate::frame::decode_frame(&datagram).unwrap();
        assert_eq!(header.service_type, TUNNELLING_ACK);
        let ack = decode_tunnelling_ack(body).unwrap();
        assert_eq!(ack, TunnellingAck { channel_id: 0x15, sequence_counter: 0x00, status: E_NO_ERROR });
    }

    #[test]
    fn tunnelling_ack_round_trips() {
        let body = encode_tunnelling_ack(0x21, 0x03, E_NO_ERROR);
        let ack = decode_tunnelling_ack(&body).unwrap();
        assert_eq!(ack, TunnellingAck { channel_id: 0x21, sequence_counter: 0x03, status: E_NO_ERROR });
    }

    #[test]
    fn tunnelling_request_round_trips_opaque_cemi_bytes() {
        let cemi = [0x11u8, 0x00, 0xAA, 0xBB, 0xCC];
        let body = encode_tunnelling_request(0x15, 0x00, &cemi);
        // First 6 octets of the header + these 4 match Tunnelling
        // v01.07.01 AS §6.1, Figure 15's worked example verbatim.
        assert_eq!(&body[..4], &[0x04, 0x15, 0x00, 0x00]);
        let req = decode_tunnelling_request(&body).unwrap();
        assert_eq!(req.channel_id, 0x15);
        assert_eq!(req.sequence_counter, 0x00);
        assert_eq!(req.cemi, &cemi);
    }

    #[test]
    fn decode_tunnelling_request_rejects_short_body() {
        let err = decode_tunnelling_request(&[0x04, 0x15]).unwrap_err();
        assert_eq!(err, TunnellingError::TooShort { needed: 4, got: 2 });
    }
}
```

- [ ] **Step 2: Run to verify it fails**

Run: `cargo test -p knx-net tunnelling::`
Expected: FAIL — nothing defined yet.

- [ ] **Step 3: Implement**

```rust
//! Tunnelling-specific CRI/CRD and TUNNELLING_REQUEST/ACK (Tunnelling
//! v01.07.01 AS §5.4). Connection type `TUNNEL_CONNECTION` (§5.4.2); this
//! cycle only ever requests `TUNNEL_LINKLAYER` (§5.4.3.1, Table 10) — Raw
//! and Busmonitor modes are optional server features this cycle does not
//! use.

use knx_core::IndividualAddress;

pub const TUNNEL_CONNECTION: u8 = 0x04;
pub const TUNNEL_LINKLAYER: u8 = 0x02;
pub const TUNNELLING_REQUEST: u16 = 0x0420;
pub const TUNNELLING_ACK: u16 = 0x0421;
pub const E_NO_ERROR: u8 = 0x00;

/// Basic CRI (Tunnelling v01.07.01 AS §5.4.3.1, Figure 4) — the Extended
/// CRI (§5.4.3.2, requesting a specific Individual Address) is not needed
/// this cycle; a Basic CRI lets the server assign any free one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TunnelCri {
    pub layer: u8,
}

impl TunnelCri {
    pub fn encode(self) -> [u8; 4] {
        [0x04, TUNNEL_CONNECTION, self.layer, 0x00]
    }
}

/// Tunnelling CRD (Tunnelling v01.07.01 AS §5.4.4, Figure 7) — the
/// Individual Address the server assigned to this connection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TunnelCrd {
    pub individual_address: IndividualAddress,
}

impl TunnelCrd {
    pub fn decode(buf: &[u8]) -> Result<Self, TunnellingError> {
        if buf.len() < 4 {
            return Err(TunnellingError::TooShort { needed: 4, got: buf.len() });
        }
        if buf[0] != 0x04 {
            return Err(TunnellingError::BadStructureLength(buf[0]));
        }
        if buf[1] != TUNNEL_CONNECTION {
            return Err(TunnellingError::UnexpectedConnectionType(buf[1]));
        }
        let raw = u16::from_be_bytes([buf[2], buf[3]]);
        Ok(TunnelCrd { individual_address: IndividualAddress::from_raw(raw) })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TunnellingRequest<'a> {
    pub channel_id: u8,
    pub sequence_counter: u8,
    pub cemi: &'a [u8],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TunnellingAck {
    pub channel_id: u8,
    pub sequence_counter: u8,
    pub status: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TunnellingError {
    TooShort { needed: usize, got: usize },
    BadStructureLength(u8),
    UnexpectedConnectionType(u8),
}

impl std::fmt::Display for TunnellingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TunnellingError::TooShort { needed, got } => {
                write!(f, "tunnelling structure too short: needed {needed} octets, got {got}")
            }
            TunnellingError::BadStructureLength(len) => {
                write!(f, "unexpected structure length {len:#04x}, expected 0x04")
            }
            TunnellingError::UnexpectedConnectionType(code) => {
                write!(f, "unexpected connection type {code:#04x}, expected TUNNEL_CONNECTION")
            }
        }
    }
}

impl std::error::Error for TunnellingError {}

/// Tunnelling v01.07.01 AS §5.4.6, Figure 9: connection header (structure
/// length 04h, channel ID, sequence counter, reserved) followed by the
/// cEMI frame.
pub fn encode_tunnelling_request(channel_id: u8, sequence_counter: u8, cemi: &[u8]) -> Vec<u8> {
    let mut body = Vec::with_capacity(4 + cemi.len());
    body.push(0x04);
    body.push(channel_id);
    body.push(sequence_counter);
    body.push(0x00);
    body.extend_from_slice(cemi);
    body
}

pub fn decode_tunnelling_request(body: &[u8]) -> Result<TunnellingRequest<'_>, TunnellingError> {
    if body.len() < 4 {
        return Err(TunnellingError::TooShort { needed: 4, got: body.len() });
    }
    if body[0] != 0x04 {
        return Err(TunnellingError::BadStructureLength(body[0]));
    }
    Ok(TunnellingRequest {
        channel_id: body[1],
        sequence_counter: body[2],
        cemi: &body[4..],
    })
}

/// Tunnelling v01.07.01 AS §5.4.7, Figure 10: structure length 04h,
/// channel ID, sequence counter, status — no cEMI payload.
pub fn encode_tunnelling_ack(channel_id: u8, sequence_counter: u8, status: u8) -> [u8; 4] {
    [0x04, channel_id, sequence_counter, status]
}

pub fn decode_tunnelling_ack(body: &[u8]) -> Result<TunnellingAck, TunnellingError> {
    if body.len() < 4 {
        return Err(TunnellingError::TooShort { needed: 4, got: body.len() });
    }
    if body[0] != 0x04 {
        return Err(TunnellingError::BadStructureLength(body[0]));
    }
    Ok(TunnellingAck {
        channel_id: body[1],
        sequence_counter: body[2],
        status: body[3],
    })
}
```

- [ ] **Step 4: Run to verify it passes**

Run: `cargo test -p knx-net tunnelling::`
Expected: PASS (7 tests).

- [ ] **Step 5: Commit**

```bash
git add crates/knx-net/src/tunnelling.rs
git commit -m "feat(knx-net): tunnelling CRI/CRD and TUNNELLING_REQUEST/ACK

Includes the exact TUNNELLING_ACK worked example from Tunnelling
v01.07.01 AS §6.2 as a regression fixture.

Co-Authored-By: Claude Sonnet 5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_019Armveec8wNNuGLVN7WhQv"
```

---

### Task 6: `cemi` — `L_Data` frame decode

**Files:**
- Modify: `crates/knx-net/src/cemi.rs`

**Interfaces:**
- Consumes: `knx_core::{GroupAddress, IndividualAddress}`.
- Produces: `pub enum LDataMessageKind { Request, Confirmation { error: bool }, Indication }`, `pub enum Destination { Individual(IndividualAddress), Group(GroupAddress) }`, `pub enum GroupValue { Short(u8), Bytes(Vec<u8>) }`, `pub enum ApplicationService { GroupValueRead, GroupValueResponse(GroupValue), GroupValueWrite(GroupValue), Other { apci: u16, data: Vec<u8> } }`, `pub struct LDataFrame { pub kind: LDataMessageKind, pub source: IndividualAddress, pub destination: Destination, pub service: ApplicationService }`, `pub enum CemiError`, `pub fn decode_l_data(buf: &[u8]) -> Result<LDataFrame, CemiError>`. Consumed by `client.rs` (Task 7) and `apps/knx-cli` (Task 9).

- [ ] **Step 1: Write the failing tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use knx_core::{GroupAddress, IndividualAddress};

    /// A minimal, hand-built `L_Data.ind` carrying `A_GroupValue_Write`
    /// with a 6-bit inline value (e.g. DPT-1 "on"): message code 29h, no
    /// additional info, Ctrl1/Ctrl2 for a standard group-addressed frame,
    /// source raw 0x1101, destination group address raw 0x0903, L=1,
    /// TPCI/APCI = GroupValueWrite (EMI_IMI v01.04.02 AS §4.1.3.2 Table 1 /
    /// Application Layer v02.01.01 AS §2.2 Table 1) with inline data = 1
    /// (on). The raw address values are arbitrary test fixtures, not
    /// meant to format to any particular `main/middle/sub` string.
    fn write_on_frame() -> Vec<u8> {
        vec![
            0x29, // L_Data.ind
            0x00, // no additional information
            0xBC, // Ctrl1: standard frame, no repeat, ack requested — exact
                  // flag values do not matter to this decoder, only the
                  // message code / Ctrl2 / addresses / TPCI-APCI do
            0xE0, // Ctrl2: AT=1 (group), hop count 6, EFF 0000
            0x11, 0x01, // source, raw 0x1101
            0x09, 0x03, // destination group address, raw 0x0903
            0x01, // L = 1 (one NPDU octet after TPCI)
            0x00, // TPCI: UDT, APCI high bits 00
            0x81, // APCI low bits 10 (Write) | data 000001 (value 1)
        ]
    }

    #[test]
    fn decodes_group_value_write_with_inline_data() {
        let frame = decode_l_data(&write_on_frame()).unwrap();
        assert_eq!(frame.kind, LDataMessageKind::Indication);
        assert_eq!(frame.source, IndividualAddress::from_raw(0x1101));
        assert_eq!(frame.destination, Destination::Group(GroupAddress::from_raw(0x0903)));
        assert_eq!(frame.service, ApplicationService::GroupValueWrite(GroupValue::Short(0x01)));
    }

    #[test]
    fn decodes_group_value_read() {
        let mut bytes = write_on_frame();
        let len = bytes.len();
        bytes[len - 2] = 0x00; // TPCI high bits 00
        bytes[len - 1] = 0x00; // APCI low bits 00 (Read), no data
        let frame = decode_l_data(&bytes).unwrap();
        assert_eq!(frame.service, ApplicationService::GroupValueRead);
    }

    #[test]
    fn decodes_group_value_response_with_a_full_data_byte() {
        let mut bytes = write_on_frame();
        let len = bytes.len();
        bytes[len - 3] = 0x02; // L = 2 (TPCI/APCI-low octet + one data octet)
        bytes[len - 2] = 0x00; // TPCI high bits 00
        bytes[len - 1] = 0x40; // APCI low bits 01 (Response), no inline data
        bytes.push(0x2A); // the one data octet: 42
        let frame = decode_l_data(&bytes).unwrap();
        assert_eq!(
            frame.service,
            ApplicationService::GroupValueResponse(GroupValue::Bytes(vec![0x2A]))
        );
    }

    #[test]
    fn destination_individual_when_at_bit_clear() {
        let mut bytes = write_on_frame();
        bytes[3] = 0x60; // Ctrl2: AT=0 (individual), hop count 6
        let frame = decode_l_data(&bytes).unwrap();
        assert_eq!(frame.destination, Destination::Individual(IndividualAddress::from_raw(0x0903)));
    }

    #[test]
    fn confirmation_carries_the_error_flag() {
        let mut bytes = write_on_frame();
        bytes[0] = 0x2E; // L_Data.con
        bytes[2] = 0xBD; // Ctrl1 with the Confirm (C, lsb) bit set
        let frame = decode_l_data(&bytes).unwrap();
        assert_eq!(frame.kind, LDataMessageKind::Confirmation { error: true });
    }

    #[test]
    fn unsupported_message_code_is_reported_not_panicked() {
        let mut bytes = write_on_frame();
        bytes[0] = 0xFC; // not a defined cEMI message code
        let err = decode_l_data(&bytes).unwrap_err();
        assert_eq!(err, CemiError::UnsupportedMessageCode(0xFC));
    }

    #[test]
    fn decode_rejects_short_buffer() {
        let err = decode_l_data(&[0x29]).unwrap_err();
        assert_eq!(err, CemiError::TooShort { needed: 2, got: 1 });
    }

    #[test]
    fn unknown_apci_is_reported_as_other_not_dropped() {
        let mut bytes = write_on_frame();
        let len = bytes.len();
        bytes[len - 2] = 0x03; // TPCI high bits 11 (not one of the three group
                                // services this cycle interprets)
        bytes[len - 1] = 0xC0;
        let frame = decode_l_data(&bytes).unwrap();
        match frame.service {
            ApplicationService::Other { .. } => {}
            other => panic!("expected Other, got {other:?}"),
        }
    }
}
```

- [ ] **Step 2: Run to verify it fails**

Run: `cargo test -p knx-net cemi::`
Expected: FAIL — nothing defined yet.

- [ ] **Step 3: Implement**

```rust
//! cEMI `L_Data` frame decode (EMI_IMI v01.04.02 AS §4.1.4, §4.1.5.3).
//! This cycle only interprets the group-communication trio
//! (`A_GroupValue_Read/Response/Write`, Application Layer v02.01.01 AS
//! §2.2 Table 1); every other APCI is preserved as raw bytes
//! (`ApplicationService::Other`), never silently dropped.

use knx_core::{GroupAddress, IndividualAddress};

pub const L_DATA_REQ: u8 = 0x11;
pub const L_DATA_CON: u8 = 0x2E;
pub const L_DATA_IND: u8 = 0x29;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LDataMessageKind {
    Request,
    /// `error` is Ctrl1's Confirm flag (EMI_IMI v01.04.02 AS §4.1.5.3.4):
    /// `false` = no error, `true` = error.
    Confirmation { error: bool },
    Indication,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Destination {
    Individual(IndividualAddress),
    Group(GroupAddress),
}

/// The wire form of a group value, before any DPT interpretation (no DPT
/// is resolved this cycle — RESEARCH.md §8.1's `DPTBinary`/`DPTArray`
/// distinction, kept as raw bytes rather than decoded engineering values).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GroupValue {
    /// Fits in the TPCI/APCI-low octet's 6 data bits (e.g. a DPT-1 boolean).
    Short(u8),
    /// One or more full octets follow the TPCI/APCI-low octet.
    Bytes(Vec<u8>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApplicationService {
    GroupValueRead,
    GroupValueResponse(GroupValue),
    GroupValueWrite(GroupValue),
    /// Any APCI this cycle does not interpret — the raw TPCI/APCI-high
    /// octet combined with the APCI-low octet, and every data octet that
    /// followed, so nothing is lost (CLAUDE.md: never silently discard).
    Other { apci: u16, data: Vec<u8> },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LDataFrame {
    pub kind: LDataMessageKind,
    pub source: IndividualAddress,
    pub destination: Destination,
    pub service: ApplicationService,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CemiError {
    TooShort { needed: usize, got: usize },
    UnsupportedMessageCode(u8),
}

impl std::fmt::Display for CemiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CemiError::TooShort { needed, got } => {
                write!(f, "cEMI frame too short: needed at least {needed} octets, got {got}")
            }
            CemiError::UnsupportedMessageCode(code) => {
                write!(f, "unsupported cEMI message code {code:#04x}")
            }
        }
    }
}

impl std::error::Error for CemiError {}

/// Decodes an `L_Data.req`/`.con`/`.ind` cEMI frame (EMI_IMI v01.04.02 AS
/// §4.1.5.3.2, generic layout shared by all three services).
pub fn decode_l_data(buf: &[u8]) -> Result<LDataFrame, CemiError> {
    if buf.len() < 2 {
        return Err(CemiError::TooShort { needed: 2, got: buf.len() });
    }
    let message_code = buf[0];
    let kind = match message_code {
        L_DATA_REQ => LDataMessageKind::Request,
        L_DATA_IND => LDataMessageKind::Indication,
        L_DATA_CON => LDataMessageKind::Confirmation { error: false }, // filled in below
        other => return Err(CemiError::UnsupportedMessageCode(other)),
    };
    let add_info_len = buf[1] as usize;
    let fixed_part_start = 2 + add_info_len;
    // Ctrl1, Ctrl2, 2*source, 2*destination, L = 7 octets minimum before
    // the TPCI/APCI octets even start.
    if buf.len() < fixed_part_start + 7 {
        return Err(CemiError::TooShort { needed: fixed_part_start + 7, got: buf.len() });
    }
    let ctrl1 = buf[fixed_part_start];
    let ctrl2 = buf[fixed_part_start + 1];
    let kind = match kind {
        LDataMessageKind::Confirmation { .. } => {
            LDataMessageKind::Confirmation { error: ctrl1 & 0x01 != 0 }
        }
        other => other,
    };
    let source = IndividualAddress::from_raw(u16::from_be_bytes([
        buf[fixed_part_start + 2],
        buf[fixed_part_start + 3],
    ]));
    let dest_raw = u16::from_be_bytes([buf[fixed_part_start + 4], buf[fixed_part_start + 5]]);
    // Ctrl2 bit 7: Address Type — 0 individual, 1 group (EMI_IMI v01.04.02
    // AS §4.1.5.3.2).
    let destination = if ctrl2 & 0x80 != 0 {
        Destination::Group(GroupAddress::from_raw(dest_raw))
    } else {
        Destination::Individual(IndividualAddress::from_raw(dest_raw))
    };
    let length = buf[fixed_part_start + 6] as usize;
    let tpci_apci_start = fixed_part_start + 7;
    if buf.len() < tpci_apci_start + 2 {
        return Err(CemiError::TooShort { needed: tpci_apci_start + 2, got: buf.len() });
    }
    let tpci_apci_hi = buf[tpci_apci_start];
    let apci_lo_and_data = buf[tpci_apci_start + 1];
    // The three `A_GroupValue_*` services use only a 4-bit APCI (Application
    // Layer v02.01.01 AS §2.2, Table 1): its top 2 bits are TPCI-octet bits
    // 1-0, its bottom 2 bits are APCI-octet bits 7-6. The APCI-octet's
    // remaining 6 bits (bits 5-0) carry inline data when `length <= 1`.
    let short_apci = ((tpci_apci_hi & 0x03) << 2) | (apci_lo_and_data >> 6);
    let inline6 = apci_lo_and_data & 0x3F;
    let extra_len = length.saturating_sub(1);
    let extra_start = tpci_apci_start + 2;
    if buf.len() < extra_start + extra_len {
        return Err(CemiError::TooShort { needed: extra_start + extra_len, got: buf.len() });
    }
    let extra = &buf[extra_start..extra_start + extra_len];
    let service = match short_apci {
        0b0000 => ApplicationService::GroupValueRead,
        0b0001 => ApplicationService::GroupValueResponse(group_value(length, inline6, extra)),
        0b0010 => ApplicationService::GroupValueWrite(group_value(length, inline6, extra)),
        _ => ApplicationService::Other {
            apci: ((tpci_apci_hi as u16) << 8) | apci_lo_and_data as u16,
            data: extra.to_vec(),
        },
    };
    Ok(LDataFrame { kind, source, destination, service })
}

fn group_value(length: usize, inline6: u8, extra: &[u8]) -> GroupValue {
    if length <= 1 {
        GroupValue::Short(inline6)
    } else {
        GroupValue::Bytes(extra.to_vec())
    }
}
```

- [ ] **Step 4: Run to verify it passes**

Run: `cargo test -p knx-net cemi::`
Expected: PASS (8 tests).

- [ ] **Step 5: Commit**

```bash
git add crates/knx-net/src/cemi.rs
git commit -m "feat(knx-net): decode cEMI L_Data frames (GroupValue Read/Response/Write)

Co-Authored-By: Claude Sonnet 5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_019Armveec8wNNuGLVN7WhQv"
```

---

### Task 7: `client` — `BusConnection` trait and `TunnelClient`

**Files:**
- Modify: `crates/knx-net/src/client.rs`
- Modify: `crates/knx-net/src/lib.rs`

**Interfaces:**
- Consumes: `frame::{encode_frame, decode_frame}` (Task 2), `core::hpai::Hpai` (Task 3), `core::services::*` (Task 4), `tunnelling::*` (Task 5), `cemi::{decode_l_data, LDataFrame}` (Task 6), `knx_core::IndividualAddress`.
- Produces: `pub enum BusError`, `pub trait BusConnection { async fn discover(&self) -> Result<Vec<std::net::SocketAddrV4>, BusError>; async fn connect_tunnel(&self, gateway: std::net::SocketAddrV4) -> Result<TunnelClient, BusError>; }`, `pub struct KnxNetIpClient` (implements `BusConnection`), `pub struct TunnelClient` with `pub fn assigned_address(&self) -> IndividualAddress`, `pub fn subscribe(&self) -> tokio::sync::broadcast::Receiver<LDataFrame>`, `pub async fn send(&self, frame: &LDataFrame) -> Result<(), BusError>` (stub, `NotImplemented`), `pub async fn disconnect(self) -> Result<(), BusError>`. This is what `apps/knx-cli` (Task 9) and the live integration test (Task 8) call.

This task has no isolated unit test of its own (a real or fake UDP peer is out of scope this cycle — see the design spec's testing section); its correctness is exercised end-to-end by Task 8's `#[ignore]`d live test. Steps here are "write the implementation, then confirm the whole crate still builds and every earlier task's tests still pass."

- [ ] **Step 1: Implement `BusError`**

```rust
//! The `BusConnection` trait and its tunnelling implementation
//! (ARCHITECTURE.md §8). `discover`/`TunnelClient::send` are explicit
//! `BusError::NotImplemented` stubs this cycle — never `todo!()`, never a
//! silent no-op (CLAUDE.md).

use std::net::{SocketAddr, SocketAddrV4};
use std::sync::Arc;
use std::time::Duration;

use knx_core::IndividualAddress;
use tokio::net::UdpSocket;
use tokio::sync::{broadcast, Mutex, Notify};

use crate::cemi::{self, LDataFrame};
use crate::core::hpai::Hpai;
use crate::core::services;
use crate::frame;
use crate::tunnelling;

#[derive(Debug)]
pub enum BusError {
    Io(std::io::Error),
    Timeout,
    ConnectionRefused(u8),
    NotImplemented,
    /// Any lower-layer codec error, carried as text — `frame`/`core::hpai`/
    /// `core::services`/`tunnelling`/`cemi` each already have their own
    /// precise error type; this cycle's client only ever needs to report
    /// them upward, not branch on which one it was.
    Protocol(String),
}

impl std::fmt::Display for BusError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BusError::Io(e) => write!(f, "I/O error: {e}"),
            BusError::Timeout => write!(f, "timed out waiting for a response"),
            BusError::ConnectionRefused(status) => {
                write!(f, "gateway refused the connection (status {status:#04x})")
            }
            BusError::NotImplemented => write!(f, "not implemented in this cycle"),
            BusError::Protocol(msg) => write!(f, "protocol error: {msg}"),
        }
    }
}

impl std::error::Error for BusError {}
```

Run: `cargo build -p knx-net`
Expected: builds (only `BusError` defined so far; nothing references it yet).

- [ ] **Step 2: Implement the trait and `KnxNetIpClient`**

Append to `client.rs`:

```rust
/// A local KNXnet/IP client, not yet connected to any gateway. Cycle 1
/// implements `connect_tunnel` only; `discover` (multicast `SEARCH_REQUEST`,
/// Core v01.06.02 AS §4.2) is a later cycle's work.
pub struct KnxNetIpClient;

impl KnxNetIpClient {
    pub fn new() -> Self {
        Self
    }
}

impl Default for KnxNetIpClient {
    fn default() -> Self {
        Self::new()
    }
}

// Native `async fn` in a trait (stable since Rust 1.75, well within this
// workspace's 1.98 floor) — no `async-trait` dependency needed as long as
// nothing needs `dyn BusConnection`, which this cycle never does.
pub trait BusConnection {
    async fn discover(&self) -> Result<Vec<SocketAddrV4>, BusError>;
    async fn connect_tunnel(&self, gateway: SocketAddrV4) -> Result<TunnelClient, BusError>;
}

impl BusConnection for KnxNetIpClient {
    async fn discover(&self) -> Result<Vec<SocketAddrV4>, BusError> {
        Err(BusError::NotImplemented)
    }

    async fn connect_tunnel(&self, gateway: SocketAddrV4) -> Result<TunnelClient, BusError> {
        TunnelClient::connect(gateway).await
    }
}
```

Run: `cargo build -p knx-net`
Expected: fails — `TunnelClient` does not exist yet. Expected at this point in the task; continue to the next step before running again.

- [ ] **Step 3: Implement `TunnelState` and `TunnelClient::connect`**

Append to `client.rs`:

```rust
struct TunnelState {
    socket: UdpSocket,
    channel_id: u8,
    assigned_address: IndividualAddress,
    tx: broadcast::Sender<LDataFrame>,
    heartbeat_reply: Mutex<Option<u8>>,
    heartbeat_notify: Notify,
    shutdown: Notify,
}

pub struct TunnelClient {
    state: Arc<TunnelState>,
}

impl TunnelClient {
    async fn connect(gateway: SocketAddrV4) -> Result<Self, BusError> {
        let socket = UdpSocket::bind("0.0.0.0:0").await.map_err(BusError::Io)?;
        socket.connect(gateway).await.map_err(BusError::Io)?;
        let control_hpai = local_hpai(&socket)?;

        let cri = tunnelling::TunnelCri { layer: tunnelling::TUNNEL_LINKLAYER }.encode();
        let body = services::encode_connect_request(control_hpai, &cri, control_hpai);
        let datagram = frame::encode_frame(services::CONNECT_REQUEST, &body);
        socket.send(&datagram).await.map_err(BusError::Io)?;

        let mut buf = [0u8; 64];
        let n = tokio::time::timeout(Duration::from_secs(10), socket.recv(&mut buf))
            .await
            .map_err(|_| BusError::Timeout)?
            .map_err(BusError::Io)?;
        let (header, resp_body) =
            frame::decode_frame(&buf[..n]).map_err(|e| BusError::Protocol(e.to_string()))?;
        if header.service_type != services::CONNECT_RESPONSE {
            return Err(BusError::Protocol(format!(
                "expected CONNECT_RESPONSE, got service type {:#06x}",
                header.service_type
            )));
        }
        let response = services::decode_connect_response(resp_body)
            .map_err(|e| BusError::Protocol(e.to_string()))?;
        if response.status != services::E_NO_ERROR {
            return Err(BusError::ConnectionRefused(response.status));
        }
        let crd = tunnelling::TunnelCrd::decode(&response.crd)
            .map_err(|e| BusError::Protocol(e.to_string()))?;

        let (tx, _rx) = broadcast::channel(64);
        let state = Arc::new(TunnelState {
            socket,
            channel_id: response.channel_id,
            assigned_address: crd.individual_address,
            tx,
            heartbeat_reply: Mutex::new(None),
            heartbeat_notify: Notify::new(),
            shutdown: Notify::new(),
        });

        tokio::spawn(receive_loop(state.clone()));
        tokio::spawn(heartbeat_loop(state.clone()));

        Ok(TunnelClient { state })
    }

    pub fn assigned_address(&self) -> IndividualAddress {
        self.state.assigned_address
    }

    pub fn subscribe(&self) -> broadcast::Receiver<LDataFrame> {
        self.state.tx.subscribe()
    }

    /// Sending is out of scope this cycle (design spec §Scope) — this is a
    /// typed, catchable stub, not a silent no-op.
    pub async fn send(&self, _frame: &LDataFrame) -> Result<(), BusError> {
        Err(BusError::NotImplemented)
    }

    /// Best-effort graceful disconnect (Core v01.06.02 AS §5.5): sends
    /// `DISCONNECT_REQUEST` and signals the background tasks to stop. Does
    /// not block on the server's `DISCONNECT_RESPONSE` — `receive_loop`
    /// observes it (or the socket simply going quiet) and exits on its own.
    pub async fn disconnect(self) -> Result<(), BusError> {
        let control_hpai = local_hpai(&self.state.socket)?;
        let body = services::encode_disconnect_request(self.state.channel_id, control_hpai);
        let datagram = frame::encode_frame(services::DISCONNECT_REQUEST, &body);
        self.state.socket.send(&datagram).await.map_err(BusError::Io)?;
        self.state.shutdown.notify_waiters();
        Ok(())
    }
}

fn local_hpai(socket: &UdpSocket) -> Result<Hpai, BusError> {
    match socket.local_addr().map_err(BusError::Io)? {
        SocketAddr::V4(addr) => Ok(Hpai { addr: *addr.ip(), port: addr.port() }),
        SocketAddr::V6(_) => Err(BusError::Protocol(
            "local socket bound to an IPv6 address, expected IPv4".to_string(),
        )),
    }
}
```

Run: `cargo build -p knx-net`
Expected: fails — `receive_loop`/`heartbeat_loop` not defined yet. Continue to the next step.

- [ ] **Step 4: Implement `receive_loop` and `heartbeat_loop`**

Append to `client.rs`:

```rust
/// The sole reader of `state.socket` for the lifetime of the connection —
/// `heartbeat_loop` only ever sends, then waits on `heartbeat_notify`, so
/// there is never a second concurrent reader racing this one.
async fn receive_loop(state: Arc<TunnelState>) {
    let mut recv_seq: u8 = 0;
    let mut buf = [0u8; 1024];
    loop {
        let n = tokio::select! {
            _ = state.shutdown.notified() => break,
            result = state.socket.recv(&mut buf) => match result {
                Ok(n) => n,
                Err(_) => break, // socket gone; nothing more to receive
            },
        };
        let Ok((header, body)) = frame::decode_frame(&buf[..n]) else {
            continue; // malformed datagram: ignore it (Core v01.06.02 AS §6.2/§6.3), don't crash
        };
        match header.service_type {
            tunnelling::TUNNELLING_REQUEST => {
                let Ok(req) = tunnelling::decode_tunnelling_request(body) else { continue };
                if req.channel_id != state.channel_id {
                    continue;
                }
                if req.sequence_counter == recv_seq {
                    send_ack(&state, req.sequence_counter, tunnelling::E_NO_ERROR).await;
                    recv_seq = recv_seq.wrapping_add(1);
                    if let Ok(telegram) = cemi::decode_l_data(req.cemi) {
                        let _ = state.tx.send(telegram);
                    }
                } else if req.sequence_counter == recv_seq.wrapping_sub(1) {
                    // Duplicate of the frame just processed (our own ACK
                    // was presumably lost) — ack again, discard (Tunnelling
                    // v01.07.01 AS §2.6.1).
                    send_ack(&state, req.sequence_counter, tunnelling::E_NO_ERROR).await;
                }
                // Any other sequence number: no ack, discard (§2.6.1).
            }
            services::CONNECTIONSTATE_RESPONSE => {
                if let Ok(resp) = services::decode_connectionstate_response(body) {
                    if resp.channel_id == state.channel_id {
                        *state.heartbeat_reply.lock().await = Some(resp.status);
                        state.heartbeat_notify.notify_one();
                    }
                }
            }
            services::DISCONNECT_REQUEST => break, // server-initiated disconnect
            _ => {} // unsupported/unknown service type: ignore (§6.2/§6.3)
        }
    }
}

async fn send_ack(state: &TunnelState, sequence_counter: u8, status: u8) {
    let body = tunnelling::encode_tunnelling_ack(state.channel_id, sequence_counter, status);
    let datagram = frame::encode_frame(tunnelling::TUNNELLING_ACK, &body);
    // Best-effort: a lost ACK makes the peer retry the TUNNELLING_REQUEST
    // (Tunnelling v01.07.01 AS §2.6.1), which this loop will then see again.
    let _ = state.socket.send(&datagram).await;
}

/// Core v01.06.02 AS §5.4: a `CONNECTIONSTATE_REQUEST` every 60 seconds;
/// on no reply (or an error status) within 10 seconds, repeat up to three
/// times, then disconnect.
async fn heartbeat_loop(state: Arc<TunnelState>) {
    let mut interval = tokio::time::interval(Duration::from_secs(60));
    interval.tick().await; // the first tick fires immediately; consume it
                            // so the first heartbeat happens 60s in, not at connect time
    loop {
        tokio::select! {
            _ = state.shutdown.notified() => return,
            _ = interval.tick() => {}
        }
        if !send_heartbeat_with_retries(&state).await {
            state.shutdown.notify_waiters();
            let _ = state.socket.send(&frame::encode_frame(
                services::DISCONNECT_REQUEST,
                &services::encode_disconnect_request(
                    state.channel_id,
                    match local_hpai(&state.socket) {
                        Ok(hpai) => hpai,
                        Err(_) => return,
                    },
                ),
            )).await;
            return;
        }
    }
}

async fn send_heartbeat_with_retries(state: &Arc<TunnelState>) -> bool {
    for _attempt in 0..4 {
        let Ok(control_hpai) = local_hpai(&state.socket) else { return false };
        let body = services::encode_connectionstate_request(state.channel_id, control_hpai);
        let datagram = frame::encode_frame(services::CONNECTIONSTATE_REQUEST, &body);
        *state.heartbeat_reply.lock().await = None;
        if state.socket.send(&datagram).await.is_err() {
            return false;
        }
        let waited =
            tokio::time::timeout(Duration::from_secs(10), state.heartbeat_notify.notified()).await;
        let status = state.heartbeat_reply.lock().await.take();
        if waited.is_ok() && status == Some(services::E_NO_ERROR) {
            return true;
        }
    }
    false
}
```

Run: `cargo build -p knx-net`
Expected: builds cleanly.

- [ ] **Step 5: Re-export the public surface from `lib.rs`**

```rust
pub use cemi::{ApplicationService, Destination, GroupValue, LDataFrame, LDataMessageKind};
pub use client::{BusConnection, BusError, KnxNetIpClient, TunnelClient};
```

- [ ] **Step 6: Run every earlier task's tests to confirm nothing regressed**

Run: `cargo test -p knx-net`
Expected: PASS (all `frame`/`core::hpai`/`core::services`/`tunnelling`/`cemi` tests from Tasks 2-6, `client` itself has none yet).

Run: `cargo clippy -p knx-net --all-targets -- -D warnings`
Expected: clean.

- [ ] **Step 7: Commit**

```bash
git add crates/knx-net/src/client.rs crates/knx-net/src/lib.rs
git commit -m "feat(knx-net): BusConnection trait and TunnelClient (connect/heartbeat/receive/disconnect)

discover() and TunnelClient::send() are explicit NotImplemented stubs
this cycle. Exercised end-to-end by the #[ignore]'d live-gateway test,
not a unit test — see the design spec's testing section.

Co-Authored-By: Claude Sonnet 5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_019Armveec8wNNuGLVN7WhQv"
```

---

### Task 8: Live-gateway integration test

**Files:**
- Create: `crates/knx-net/tests/live_gateway.rs`

**Interfaces:**
- Consumes: `knx_net::{BusConnection, KnxNetIpClient}` (Task 7).
- Produces: nothing further downstream — this is the end of `knx-net`'s own test surface.

- [ ] **Step 1: Write the test**

```rust
//! Integration test against a real KNXnet/IP gateway. Requires actual
//! hardware reachable on the LAN — never runs in CI. Run explicitly:
//! `cargo test -p knx-net -- --ignored`.

use std::net::SocketAddrV4;
use std::time::Duration;

use knx_net::BusConnection;

fn gateway_addr() -> SocketAddrV4 {
    std::env::var("KNX_GATEWAY")
        .unwrap_or_else(|_| "192.0.2.1:3671".to_string())
        .parse()
        .expect("KNX_GATEWAY must be host:port, e.g. 192.0.2.1:3671")
}

#[tokio::test]
#[ignore = "needs a real KNXnet/IP gateway on the LAN"]
async fn connects_and_receives_at_least_one_telegram() {
    let client = knx_net::KnxNetIpClient::new();
    let tunnel = client
        .connect_tunnel(gateway_addr())
        .await
        .expect("connect_tunnel against a real gateway");
    let mut telegrams = tunnel.subscribe();
    let received = tokio::time::timeout(Duration::from_secs(60), telegrams.recv()).await;
    tunnel.disconnect().await.expect("clean disconnect");
    received
        .expect("at least one telegram within 60s — trigger a switch on the bus if this times out")
        .expect("broadcast channel still open");
}
```

- [ ] **Step 2: Run it against the real gateway to confirm the whole cycle actually works**

Run: `KNX_GATEWAY=192.0.2.1:3671 cargo test -p knx-net -- --ignored --nocapture`
Expected: PASS, having received at least one telegram within 60 seconds. If it times out, trigger any switch on the bus during the test window and re-run — this is the manual verification step the design spec calls for before RESEARCH.md gets its `[V]` entry (see Task 10).

Run (confirm CI is unaffected): `cargo test -p knx-net`
Expected: PASS, and `connects_and_receives_at_least_one_telegram` shows as `ignored`, not run.

- [ ] **Step 3: Commit**

```bash
git add crates/knx-net/tests/live_gateway.rs
git commit -m "test(knx-net): live-gateway integration test (ignored by default)

First test of this kind in the repo -- needs real hardware, never runs
in CI. Verified passing against 192.0.2.1:3671.

Co-Authored-By: Claude Sonnet 5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_019Armveec8wNNuGLVN7WhQv"
```

---

### Task 9: `knx bus monitor` CLI subcommand

**Files:**
- Modify: `apps/knx-cli/Cargo.toml`
- Modify: `apps/knx-cli/src/main.rs`

**Interfaces:**
- Consumes: `knx_net::{BusConnection, KnxNetIpClient, LDataFrame, ApplicationService, Destination, GroupValue}` (Task 7), `knx_store::migration::open_and_migrate`, `knx_store::project::load_project`, `knx_core::GroupAddressStyle`.
- Produces: the `knx bus monitor --gateway <host:port> [--project <path.knxdb>]` command. Nothing downstream depends on this — it is the cycle's end-user-visible deliverable.

- [ ] **Step 1: Add dependencies**

```toml
[dependencies]
knx-app.workspace = true
knx-core.workspace = true
knx-etsproj.workspace = true
knx-net.workspace = true
knx-productdb.workspace = true
knx-store.workspace = true
tempfile.workspace = true
tokio = { workspace = true, features = ["rt", "signal"] }
```

- [ ] **Step 2: Extend the usage string and dispatch**

In `apps/knx-cli/src/main.rs`, add to `USAGE`:

```rust
const USAGE: &str =
    "usage: knx import <file.knxproj> [--store <path.knxdb>] [--report-json <path.json>]\n\
     \x20                  [--product-db <path>] [--no-product-db]\n\
     \x20     knx products list [--manufacturer M-xxxx] [--product-db <path>]\n\
     \x20     knx products ingest <file.knxproj> [--product-db <path>]\n\
     \x20     knx products show <program-id> [--product-db <path>]\n\
     \x20     knx products verify [--product-db <path>]\n\
     \x20     knx bus monitor --gateway <host:port> [--project <path.knxdb>]\n\
     exit codes: 0 = imported cleanly (warnings allowed), 1 = could not import,\n\
     2 = imported, but the report contains errors";
```

and in `main`'s dispatch:

```rust
Some("bus") => run_bus(&args[1..]),
```

- [ ] **Step 3: Implement the subcommand**

Append to `main.rs`:

```rust
fn run_bus(args: &[String]) -> ExitCode {
    match args.first().map(String::as_str) {
        Some("monitor") => run_bus_monitor(&args[1..]),
        _ => {
            eprintln!("{USAGE}");
            ExitCode::FAILURE
        }
    }
}

struct BusMonitorArgs {
    gateway: String,
    project: Option<String>,
}

fn parse_bus_monitor_args(args: &[String]) -> Result<BusMonitorArgs, String> {
    let mut gateway = None;
    let mut project = None;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--gateway" => {
                gateway = Some(take_value(args, i + 1, "--gateway")?);
                i += 2;
            }
            "--project" => {
                project = Some(take_value(args, i + 1, "--project")?);
                i += 2;
            }
            other => return Err(format!("unrecognized argument: {other}")),
        }
    }
    Ok(BusMonitorArgs {
        gateway: gateway.ok_or_else(|| "--gateway is required".to_string())?,
        project,
    })
}

fn run_bus_monitor(args: &[String]) -> ExitCode {
    let parsed = match parse_bus_monitor_args(args) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("{e}\n{USAGE}");
            return ExitCode::FAILURE;
        }
    };
    let gateway: std::net::SocketAddrV4 = match parsed.gateway.parse() {
        Ok(g) => g,
        Err(_) => {
            eprintln!("--gateway must be host:port, e.g. 192.0.2.1:3671");
            return ExitCode::FAILURE;
        }
    };
    let ga_names: std::collections::HashMap<u16, String> = match &parsed.project {
        Some(path) => match load_group_address_names(Path::new(path)) {
            Ok(names) => names,
            Err(e) => {
                eprintln!("could not load project {path}: {e}");
                return ExitCode::FAILURE;
            }
        },
        None => std::collections::HashMap::new(),
    };

    let runtime = match tokio::runtime::Builder::new_current_thread().enable_all().build() {
        Ok(rt) => rt,
        Err(e) => {
            eprintln!("could not start async runtime: {e}");
            return ExitCode::FAILURE;
        }
    };

    runtime.block_on(run_bus_monitor_async(gateway, ga_names))
}

async fn run_bus_monitor_async(
    gateway: std::net::SocketAddrV4,
    ga_names: std::collections::HashMap<u16, String>,
) -> ExitCode {
    use knx_net::BusConnection;
    let client = knx_net::KnxNetIpClient::new();
    let tunnel = match client.connect_tunnel(gateway).await {
        Ok(t) => t,
        Err(e) => {
            eprintln!("could not connect to {gateway}: {e}");
            return ExitCode::FAILURE;
        }
    };
    eprintln!(
        "connected to {gateway}, assigned individual address {}. Ctrl-C to stop.",
        tunnel.assigned_address()
    );
    let mut telegrams = tunnel.subscribe();
    loop {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {
                eprintln!("disconnecting...");
                if let Err(e) = tunnel.disconnect().await {
                    eprintln!("disconnect: {e}");
                }
                break;
            }
            received = telegrams.recv() => match received {
                Ok(telegram) => println!("{}", format_telegram(&telegram, &ga_names)),
                Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                    eprintln!("warning: {n} telegram(s) dropped (receiver too slow)");
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
            },
        }
    }
    ExitCode::SUCCESS
}

fn load_group_address_names(
    path: &Path,
) -> Result<std::collections::HashMap<u16, String>, String> {
    let conn = knx_store::migration::open_and_migrate(path).map_err(|e| e.to_string())?;
    let project = knx_store::project::load_project(&conn).map_err(|e| e.to_string())?;
    let mut names = std::collections::HashMap::new();
    for installation in &project.installations {
        for entry in &installation.group_addresses {
            names.insert(entry.address.raw(), entry.name.clone());
        }
    }
    Ok(names)
}

fn format_telegram(
    telegram: &knx_net::LDataFrame,
    ga_names: &std::collections::HashMap<u16, String>,
) -> String {
    use knx_net::Destination;
    let dest = match telegram.destination {
        Destination::Group(ga) => {
            let formatted = ga.format(knx_core::GroupAddressStyle::ThreeLevel);
            match ga_names.get(&ga.raw()) {
                Some(name) => format!("{formatted} ({name})"),
                None => formatted,
            }
        }
        Destination::Individual(ia) => ia.to_string(),
    };
    format!("{} -> {dest}: {}", telegram.source, format_service(&telegram.service))
}

fn format_service(service: &knx_net::ApplicationService) -> String {
    use knx_net::{ApplicationService, GroupValue};
    let format_value = |v: &GroupValue| match v {
        GroupValue::Short(bits) => format!("{bits:#04x} (6-bit)"),
        GroupValue::Bytes(bytes) => format!("{bytes:02x?}"),
    };
    match service {
        ApplicationService::GroupValueRead => "GroupValueRead".to_string(),
        ApplicationService::GroupValueResponse(v) => format!("GroupValueResponse {}", format_value(v)),
        ApplicationService::GroupValueWrite(v) => format!("GroupValueWrite {}", format_value(v)),
        ApplicationService::Other { apci, data } => format!("APCI {apci:#06x} data {data:02x?}"),
    }
}
```

- [ ] **Step 4: Verify it builds and existing CLI tests still pass**

Run: `cargo build -p knx-cli`
Expected: builds cleanly.

Run: `cargo test -p knx-cli`
Expected: PASS (existing `import`/`products` tests unaffected — no new automated test for `bus monitor` itself, since it needs the same real gateway as Task 8; see the design spec's testing section and Task 10's manual-check note).

Run: `cargo clippy -p knx-cli --all-targets -- -D warnings`
Expected: clean.

- [ ] **Step 5: Manual smoke check against the real gateway**

Run: `cargo run -p knx-cli -- bus monitor --gateway 192.0.2.1:3671`, trigger a switch on the bus, confirm a decoded line prints, then Ctrl-C and confirm "disconnecting..." prints and the process exits.

- [ ] **Step 6: Commit**

```bash
git add apps/knx-cli
git commit -m "feat(knx-cli): add 'bus monitor' subcommand

Connects to a real KNXnet/IP gateway, prints decoded telegrams resolved
against a loaded project's group address names, disconnects cleanly on
Ctrl-C. Manually verified against 192.0.2.1:3671.

Co-Authored-By: Claude Sonnet 5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_019Armveec8wNNuGLVN7WhQv"
```

---

### Task 10: Documentation updates

**Files:**
- Modify: `docs/RESEARCH.md`
- Modify: `docs/ARCHITECTURE.md`
- Modify: `docs/IMPLEMENTATION_STATUS.md`
- Modify: `docs/KNOWN_LIMITATIONS.md`
- Modify: `docs/ROADMAP.md`

**Interfaces:** None — documentation only, no code.

- [ ] **Step 1: `docs/RESEARCH.md` §8 — add the `[V]` verification entry**

Add, immediately after the existing §8.1 paragraph (the one ending "...the project model and the live bus agree."):

```markdown
Session 6, Cycle 1 (2026-09) went one step further: an own Rust
implementation of the tunnelling connect/heartbeat/receive/disconnect
lifecycle and cEMI `L_Data` decode, built directly from the KNX
Association specification (Core v01.06.02 AS, Tunnelling v01.07.01 AS,
EMI_IMI v01.04.02 AS — not from reading `xknx`'s implementation),
connected to the same gateway and correctly decoded a live telegram.
`xknx`/`monitor_bus.py` remain the cross-check oracle, not a wire-format
source.
```

- [ ] **Step 2: `docs/ARCHITECTURE.md` §8 — update from "the work is Session 6" to what shipped**

Replace the paragraph beginning "The work is Session 6, but the interface is fixed now" with:

```markdown
Session 6, Cycle 1 delivered read-only tunnelling: `crates/knx-net` connects
to a KNXnet/IP gateway by known IP, receives KNX telegrams, and decodes
them, exposed as a `BusConnection` trait (`discover`, `connect_tunnel`) and
a `TunnelClient` handle (`send`, `subscribe`) — the same names this
document already fixed, now backed by a real implementation grounded in
the KNX Association specification rather than a port of an existing
stack. `discover` and `TunnelClient::send` remain explicit
`BusError::NotImplemented` stubs; later cycles cover them, plus routing
and KNX IP Secure. The bus monitor is a consumer that resolves telegrams
against the open project (`apps/knx-cli`'s `bus monitor` subcommand,
resolving against group address names only, no DPT interpretation yet);
the connection itself knows nothing about projects, as this section
originally specified.
```

- [ ] **Step 3: `docs/IMPLEMENTATION_STATUS.md` — new cycle entry and updated "Next session"**

Add a new paragraph after the existing Session 5 cycle history (before "Known gaps carried forward"), describing what Cycle 1 of Session 6 shipped: `crates/knx-net`'s five codec modules, the `TunnelClient` state machine, the live-gateway `#[ignore]`d test (first of its kind — noted as a new pattern for future hardware-dependent tests), and the `knx bus monitor` CLI subcommand, verified manually against `192.0.2.1:3671`.

Add to "Known gaps carried forward": discovery and sending remain `NotImplemented`; `TunnelClient`'s internal state machine has no unit test of its own (only the live-gateway integration test and the pure codec modules' unit tests) — noted as a deliberate scope decision (design spec's testing section), not an oversight.

Update "Next session" to reflect Session 6 is now in progress (Cycle 1 done), not "Session 5 (UI/UX)".

- [ ] **Step 4: `docs/KNOWN_LIMITATIONS.md` — new entry**

Add an entry: `knx-net`'s `BusConnection` does not yet support discovery, sending telegrams, routing, or KNX IP Secure — only tunnelling connect/receive/disconnect against a known gateway IP. A reader should not assume the trait is feature-complete because it compiles.

- [ ] **Step 5: `docs/ROADMAP.md` — mark Cycle 1 done under Session 6**

In the "Session 6 — KNXnet/IP" section, add a sentence noting Cycle 1 (read-only tunnelling) shipped, per the 2026-09-06 design spec, and that discovery/sending/routing/secure remain open for later cycles.

- [ ] **Step 6: Run the full workspace check**

Run:
```bash
cargo build --workspace
cargo test --workspace
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo run -p xtask -- check-layering
cargo deny check
```
Expected: all clean (the `#[ignore]`d live-gateway test is skipped by `cargo test --workspace` automatically, exactly as intended).

- [ ] **Step 7: Commit**

```bash
git add docs/RESEARCH.md docs/ARCHITECTURE.md docs/IMPLEMENTATION_STATUS.md docs/KNOWN_LIMITATIONS.md docs/ROADMAP.md
git commit -m "docs: Session 6 Cycle 1 (KNXnet/IP tunnelling) shipped

Co-Authored-By: Claude Sonnet 5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_019Armveec8wNNuGLVN7WhQv"
```
