# KNXnet/IP Discovery Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Find KNXnet/IP gateways on the LAN without a known IP — `KnxNetIpClient::discover()` sends a multicast `SEARCH_REQUEST` and collects `SEARCH_RESPONSE`s, exposed via a new `knx bus discover` CLI subcommand.

**Architecture:** Two new pure codec modules (`core::dib` for the Device Info / Supported Service Families DIBs, `discovery` for the `SEARCH_REQUEST`/`SEARCH_RESPONSE` bodies), following the exact convention Cycle 1's `core::hpai`/`core::services` already established — byte slice in, typed struct out, hand-rolled `Display`/`Error`, unit-tested against hand-built byte fixtures. `client.rs`'s `discover()` stops returning `BusError::NotImplemented`: it binds a UDP socket, sends one `SEARCH_REQUEST` to the standard discovery multicast group, collects replies for the full spec timeout, and returns a `Vec<DiscoveredGateway>`.

**Tech Stack:** Rust, `tokio` (UDP socket, `tokio::time::timeout`/`Instant`, already a `knx-net` dependency since Cycle 1). No new external dependency.

**Spec:** [docs/superpowers/specs/2026-09-06-knxnet-ip-discovery-design.md](../specs/2026-09-06-knxnet-ip-discovery-design.md)

## Global Constraints

* Byte layouts cite KNX Association specification section numbers in comments (e.g. `Core v01.06.02 AS §7.5.4.2`); spec text itself is never copied into this repository (RESEARCH.md §10; spec doc's "Handling rule").
* No new crate dependencies.
* Every new error type gets a hand-written `Display` + `std::error::Error` impl, matching `HpaiError`'s/`ServiceError`'s existing style — no `thiserror`/`anyhow`.
* Only the original `SEARCH_REQUEST`/`SEARCH_RESPONSE` (0x0201/0x0202) — not `SEARCH_REQUEST_EXTENDED`/`RESPONSE_EXTENDED` (Core v2).
* The discovery multicast group/port (`224.0.23.12:3671`) and the full spec `SEARCH_TIMEOUT` (10s) are hardcoded constants this cycle — no CLI flag (design spec Q2/Q3).
* `cargo fmt --all --check` and `cargo clippy --workspace --all-targets -- -D warnings` must stay clean after every task.
* Git commits: **no** `Co-Authored-By` trailer (`CLAUDE.md`: "No co-author. ALWAYS commit as github@knxbench.com") — this repo's own explicit rule, followed by every commit on this branch so far (`git log`). Commit with `git -c user.email=github@knxbench.com commit -m "..."`.

---

### Task 1: `core::dib` — Device Info and Supported Service Families

**Files:**
- Create: `crates/knx-net/src/core/dib.rs`
- Modify: `crates/knx-net/src/core/mod.rs` (add `pub mod dib;`)

**Interfaces:**
- Consumes: `knx_core::IndividualAddress` (`from_raw(u16) -> Self`, already used elsewhere in this crate).
- Produces: `pub const DEVICE_INFO: u8`, `pub const SUPP_SVC_FAMILIES: u8`, `pub const SERVICE_FAMILY_TUNNELLING: u8`, `pub struct DeviceInfo { pub medium: u8, pub status: u8, pub individual_address: IndividualAddress, pub project_installation_id: u16, pub serial_number: [u8; 6], pub routing_multicast: std::net::Ipv4Addr, pub mac_address: [u8; 6], pub friendly_name: String }`, `pub struct ServiceFamily { pub id: u8, pub version: u8 }`, `pub struct ServiceFamilies(pub Vec<ServiceFamily>)` with `pub fn supports(&self, family_id: u8) -> bool`, `pub enum DibError`, `pub fn decode_device_info(buf: &[u8]) -> Result<(DeviceInfo, &[u8]), DibError>`, `pub fn decode_service_families(buf: &[u8]) -> Result<(ServiceFamilies, &[u8]), DibError>`. Used by `discovery.rs` (Task 2).

- [ ] **Step 1: Write the failing tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::net::Ipv4Addr;

    fn device_info_bytes() -> Vec<u8> {
        let mut b = vec![
            0x36, DEVICE_INFO, // structure length 54, type 0x01
            0x02,             // KNX medium: TP1
            0x00,             // device status
            0x11, 0x01,       // individual address 1.1.1
            0x00, 0x00,       // project-installation identifier
            0x00, 0xFA, 0x12, 0x34, 0x56, 0x78, // serial number
            224, 0, 23, 12,   // routing multicast address
            0x00, 0x01, 0x02, 0x03, 0x04, 0x05, // MAC address
        ];
        let mut name = b"KNX IP Gateway".to_vec();
        name.resize(30, 0); // zero-padded to 30 octets
        b.extend_from_slice(&name);
        assert_eq!(b.len(), 54);
        b
    }

    #[test]
    fn decode_device_info_reads_every_field() {
        let bytes = device_info_bytes();
        let (info, rest) = decode_device_info(&bytes).unwrap();
        assert_eq!(info.medium, 0x02);
        assert_eq!(info.status, 0x00);
        assert_eq!(info.individual_address, IndividualAddress::from_raw(0x1101));
        assert_eq!(info.project_installation_id, 0);
        assert_eq!(info.serial_number, [0x00, 0xFA, 0x12, 0x34, 0x56, 0x78]);
        assert_eq!(info.routing_multicast, Ipv4Addr::new(224, 0, 23, 12));
        assert_eq!(info.mac_address, [0x00, 0x01, 0x02, 0x03, 0x04, 0x05]);
        assert_eq!(info.friendly_name, "KNX IP Gateway");
        assert!(rest.is_empty());
    }

    #[test]
    fn decode_device_info_leaves_trailing_bytes_for_the_caller() {
        let mut bytes = device_info_bytes();
        bytes.extend_from_slice(&[0xFF, 0xEE]);
        let (_, rest) = decode_device_info(&bytes).unwrap();
        assert_eq!(rest, &[0xFF, 0xEE]);
    }

    #[test]
    fn decode_device_info_rejects_short_buffer() {
        let err = decode_device_info(&[0x36, DEVICE_INFO, 0x02]).unwrap_err();
        assert_eq!(err, DibError::TooShort { needed: 54, got: 3 });
    }

    #[test]
    fn decode_device_info_rejects_bad_length() {
        let mut bytes = device_info_bytes();
        bytes[0] = 0x37;
        let err = decode_device_info(&bytes).unwrap_err();
        assert_eq!(err, DibError::BadLength(0x37));
    }

    #[test]
    fn decode_device_info_rejects_wrong_type() {
        let mut bytes = device_info_bytes();
        bytes[1] = SUPP_SVC_FAMILIES;
        let err = decode_device_info(&bytes).unwrap_err();
        assert_eq!(
            err,
            DibError::UnexpectedType { expected: DEVICE_INFO, got: SUPP_SVC_FAMILIES }
        );
    }

    #[test]
    fn decode_service_families_reads_every_pair_and_reports_tunnelling_support() {
        let bytes = [0x06, SUPP_SVC_FAMILIES, 0x02, 0x01, SERVICE_FAMILY_TUNNELLING, 0x01];
        let (families, rest) = decode_service_families(&bytes).unwrap();
        assert_eq!(
            families,
            ServiceFamilies(vec![
                ServiceFamily { id: 0x02, version: 0x01 },
                ServiceFamily { id: SERVICE_FAMILY_TUNNELLING, version: 0x01 },
            ])
        );
        assert!(families.supports(SERVICE_FAMILY_TUNNELLING));
        assert!(!families.supports(0x05));
        assert!(rest.is_empty());
    }

    #[test]
    fn decode_service_families_rejects_odd_length() {
        let bytes = [0x05, SUPP_SVC_FAMILIES, 0x02, 0x01, 0x00];
        let err = decode_service_families(&bytes).unwrap_err();
        assert_eq!(err, DibError::OddServiceFamiliesLength(0x05));
    }

    #[test]
    fn decode_service_families_rejects_wrong_type() {
        let bytes = [0x04, DEVICE_INFO, 0x02, 0x01];
        let err = decode_service_families(&bytes).unwrap_err();
        assert_eq!(
            err,
            DibError::UnexpectedType { expected: SUPP_SVC_FAMILIES, got: DEVICE_INFO }
        );
    }
}
```

- [ ] **Step 2: Run to verify it fails**

Run: `cargo test -p knx-net core::dib::`
Expected: FAIL to compile — nothing defined yet.

- [ ] **Step 3: Implement**

```rust
//! Description Information Blocks carried in a `SEARCH_RESPONSE`: Device
//! Info (mandatory, Core v01.06.02 AS §7.5.4.2) and Supported Service
//! Families (optional, §7.5.4.3). Pure byte <-> struct, no IO.

use std::net::Ipv4Addr;

use knx_core::IndividualAddress;

pub const DEVICE_INFO: u8 = 0x01;
pub const SUPP_SVC_FAMILIES: u8 = 0x02;

/// KNXnet/IP Tunnelling service family ID (Core v01.06.02 AS §7.5.4.3,
/// Service Family IDs table) — the one `DiscoveredGateway::supports_tunnelling`
/// checks for.
pub const SERVICE_FAMILY_TUNNELLING: u8 = 0x04;

pub const DEVICE_INFO_LEN: u8 = 54;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceInfo {
    pub medium: u8,
    pub status: u8,
    pub individual_address: IndividualAddress,
    pub project_installation_id: u16,
    pub serial_number: [u8; 6],
    pub routing_multicast: Ipv4Addr,
    pub mac_address: [u8; 6],
    pub friendly_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceFamily {
    pub id: u8,
    pub version: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ServiceFamilies(pub Vec<ServiceFamily>);

impl ServiceFamilies {
    pub fn supports(&self, family_id: u8) -> bool {
        self.0.iter().any(|f| f.id == family_id)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DibError {
    TooShort { needed: usize, got: usize },
    BadLength(u8),
    UnexpectedType { expected: u8, got: u8 },
    OddServiceFamiliesLength(u8),
}

impl std::fmt::Display for DibError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DibError::TooShort { needed, got } => {
                write!(f, "DIB too short: needed {needed} octets, got {got}")
            }
            DibError::BadLength(len) => {
                write!(f, "unexpected DIB structure length {len:#04x}")
            }
            DibError::UnexpectedType { expected, got } => {
                write!(
                    f,
                    "unexpected DIB description type {got:#04x}, expected {expected:#04x}"
                )
            }
            DibError::OddServiceFamiliesLength(len) => {
                write!(
                    f,
                    "Supported Service Families DIB length {len:#04x} does not hold a whole number of (ID, version) pairs"
                )
            }
        }
    }
}

impl std::error::Error for DibError {}

/// Decodes a Device Info DIB (Core v01.06.02 AS §7.5.4.2, Figure 40) from
/// the start of `buf` — structure length, type code `DEVICE_INFO`, KNX
/// medium, device status, individual address, project-installation
/// identifier, serial number, routing multicast address, MAC address, and
/// a 30-octet friendly name (ISO 8859-1, zero-padded; trailing zeros are
/// trimmed). Returns the decoded struct along with whatever octets follow
/// it in `buf`.
pub fn decode_device_info(buf: &[u8]) -> Result<(DeviceInfo, &[u8]), DibError> {
    if buf.len() < DEVICE_INFO_LEN as usize {
        return Err(DibError::TooShort {
            needed: DEVICE_INFO_LEN as usize,
            got: buf.len(),
        });
    }
    let len = buf[0];
    if len != DEVICE_INFO_LEN {
        return Err(DibError::BadLength(len));
    }
    let type_code = buf[1];
    if type_code != DEVICE_INFO {
        return Err(DibError::UnexpectedType {
            expected: DEVICE_INFO,
            got: type_code,
        });
    }
    let medium = buf[2];
    let status = buf[3];
    let individual_address = IndividualAddress::from_raw(u16::from_be_bytes([buf[4], buf[5]]));
    let project_installation_id = u16::from_be_bytes([buf[6], buf[7]]);
    let serial_number: [u8; 6] = buf[8..14].try_into().unwrap();
    let routing_multicast = Ipv4Addr::new(buf[14], buf[15], buf[16], buf[17]);
    let mac_address: [u8; 6] = buf[18..24].try_into().unwrap();
    let name_bytes = &buf[24..54];
    let name_end = name_bytes
        .iter()
        .position(|&b| b == 0)
        .unwrap_or(name_bytes.len());
    let friendly_name = name_bytes[..name_end].iter().map(|&b| b as char).collect();

    Ok((
        DeviceInfo {
            medium,
            status,
            individual_address,
            project_installation_id,
            serial_number,
            routing_multicast,
            mac_address,
            friendly_name,
        },
        &buf[54..],
    ))
}

/// Decodes a Supported Service Families DIB (§7.5.4.3, Figure 41):
/// structure length, type code `SUPP_SVC_FAMILIES`, then `(len - 2) / 2`
/// pairs of (Family ID, Family Version). Returns the decoded list along
/// with whatever octets follow it in `buf`.
pub fn decode_service_families(buf: &[u8]) -> Result<(ServiceFamilies, &[u8]), DibError> {
    if buf.len() < 2 {
        return Err(DibError::TooShort {
            needed: 2,
            got: buf.len(),
        });
    }
    let len = buf[0];
    if (buf.len() as u8) < len {
        return Err(DibError::TooShort {
            needed: len as usize,
            got: buf.len(),
        });
    }
    let type_code = buf[1];
    if type_code != SUPP_SVC_FAMILIES {
        return Err(DibError::UnexpectedType {
            expected: SUPP_SVC_FAMILIES,
            got: type_code,
        });
    }
    let pairs_len = len - 2;
    if pairs_len % 2 != 0 {
        return Err(DibError::OddServiceFamiliesLength(len));
    }
    let pairs = &buf[2..len as usize];
    let families = pairs
        .chunks_exact(2)
        .map(|pair| ServiceFamily { id: pair[0], version: pair[1] })
        .collect();
    Ok((ServiceFamilies(families), &buf[len as usize..]))
}
```

Also add `pub mod dib;` to `crates/knx-net/src/core/mod.rs`, alongside the existing `pub mod hpai;`/`pub mod services;`.

- [ ] **Step 4: Run to verify it passes**

Run: `cargo test -p knx-net core::dib::`
Expected: PASS (8 tests).

- [ ] **Step 5: Commit**

```bash
git add crates/knx-net/src/core/dib.rs crates/knx-net/src/core/mod.rs
git -c user.email=github@knxbench.com commit -m "feat(knx-net): Device Info / Supported Service Families DIB codec"
```

---

### Task 2: `discovery` — `SEARCH_REQUEST`/`SEARCH_RESPONSE`

**Files:**
- Create: `crates/knx-net/src/discovery.rs`
- Modify: `crates/knx-net/src/core/services.rs` (add `SEARCH_REQUEST`/`SEARCH_RESPONSE` constants)
- Modify: `crates/knx-net/src/lib.rs` (add `pub mod discovery;`)

**Interfaces:**
- Consumes: `crate::core::hpai::{Hpai, HpaiError}` (`encode(self) -> [u8; 8]`, `decode(buf) -> Result<(Hpai, &[u8]), HpaiError>`, from Cycle 1), `crate::core::dib::{self, DeviceInfo, DibError, ServiceFamilies}` (Task 1), `crate::core::services::{SEARCH_REQUEST, SEARCH_RESPONSE}` (this task).
- Produces: `pub struct SearchResponse { pub control_endpoint: Hpai, pub device_info: DeviceInfo, pub service_families: Option<ServiceFamilies> }`, `pub enum DiscoveryError`, `pub fn encode_search_request(discovery_endpoint: Hpai) -> Vec<u8>`, `pub fn decode_search_response(body: &[u8]) -> Result<SearchResponse, DiscoveryError>`. Used by `client.rs`'s `discover()` (Task 3).

- [ ] **Step 1: Add the service type constants and their test**

In `crates/knx-net/src/core/services.rs`, add near the existing `CONNECT_REQUEST` etc.:

```rust
// Discovery service type identifiers, Core v01.06.02 AS §7.4.1.
pub const SEARCH_REQUEST: u16 = 0x0201;
pub const SEARCH_RESPONSE: u16 = 0x0202;
```

Extend the existing `service_type_constants_match_the_spec` test:

```rust
#[test]
fn service_type_constants_match_the_spec() {
    assert_eq!(SEARCH_REQUEST, 0x0201);
    assert_eq!(SEARCH_RESPONSE, 0x0202);
    assert_eq!(CONNECT_REQUEST, 0x0205);
    assert_eq!(CONNECT_RESPONSE, 0x0206);
    assert_eq!(CONNECTIONSTATE_REQUEST, 0x0207);
    assert_eq!(CONNECTIONSTATE_RESPONSE, 0x0208);
    assert_eq!(DISCONNECT_REQUEST, 0x0209);
    assert_eq!(DISCONNECT_RESPONSE, 0x020A);
}
```

- [ ] **Step 2: Write `discovery.rs`'s failing tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::dib::{ServiceFamily, DEVICE_INFO, SUPP_SVC_FAMILIES};
    use std::net::Ipv4Addr;

    fn control_hpai() -> Hpai {
        Hpai { addr: Ipv4Addr::new(172, 18, 250, 1), port: 3671 }
    }

    fn device_info_bytes(individual_address_raw: u16) -> Vec<u8> {
        let mut b = vec![
            0x36, DEVICE_INFO, 0x02, 0x00,
        ];
        b.extend_from_slice(&individual_address_raw.to_be_bytes());
        b.extend_from_slice(&[0x00, 0x00]); // project-installation id
        b.extend_from_slice(&[0x00, 0xFA, 0x12, 0x34, 0x56, 0x78]); // serial
        b.extend_from_slice(&[224, 0, 23, 12]); // routing multicast
        b.extend_from_slice(&[0x00, 0x01, 0x02, 0x03, 0x04, 0x05]); // MAC
        let mut name = b"Gateway".to_vec();
        name.resize(30, 0);
        b.extend_from_slice(&name);
        b
    }

    fn service_families_bytes() -> Vec<u8> {
        vec![0x04, SUPP_SVC_FAMILIES, 0x04, 0x01] // Tunnelling, version 1
    }

    #[test]
    fn encode_search_request_is_just_the_discovery_hpai() {
        let body = encode_search_request(control_hpai());
        assert_eq!(body, control_hpai().encode().to_vec());
    }

    #[test]
    fn decode_search_response_reads_hpai_and_both_dibs() {
        let mut body = control_hpai().encode().to_vec();
        body.extend_from_slice(&device_info_bytes(0x1101));
        body.extend_from_slice(&service_families_bytes());

        let response = decode_search_response(&body).unwrap();
        assert_eq!(response.control_endpoint, control_hpai());
        assert_eq!(response.device_info.friendly_name, "Gateway");
        let families = response.service_families.unwrap();
        assert_eq!(families.0, vec![ServiceFamily { id: 0x04, version: 0x01 }]);
    }

    #[test]
    fn decode_search_response_accepts_missing_service_families() {
        let mut body = control_hpai().encode().to_vec();
        body.extend_from_slice(&device_info_bytes(0x1101));

        let response = decode_search_response(&body).unwrap();
        assert!(response.service_families.is_none());
    }

    #[test]
    fn decode_search_response_skips_an_unknown_dib() {
        let mut body = control_hpai().encode().to_vec();
        body.extend_from_slice(&[0x03, 0xFE, 0xAA]); // unknown 3-byte DIB, type 0xFE
        body.extend_from_slice(&device_info_bytes(0x1101));

        let response = decode_search_response(&body).unwrap();
        assert_eq!(response.device_info.friendly_name, "Gateway");
    }

    #[test]
    fn decode_search_response_rejects_missing_device_info() {
        let body = control_hpai().encode().to_vec();
        let err = decode_search_response(&body).unwrap_err();
        assert_eq!(err, DiscoveryError::MissingDeviceInfo);
    }
}
```

- [ ] **Step 3: Run to verify it fails**

Run: `cargo test -p knx-net discovery::`
Expected: FAIL to compile — nothing defined yet.

- [ ] **Step 4: Implement `discovery.rs`**

```rust
//! `SEARCH_REQUEST`/`SEARCH_RESPONSE` bodies (Core v01.06.02 AS §7.4.1,
//! Figures 20-21) — the original discovery form, not
//! `SEARCH_REQUEST_EXTENDED`/`RESPONSE_EXTENDED` (Core v2). Pure byte <->
//! struct, no IO; `client.rs` owns the socket and the multicast send.

use crate::core::dib::{self, DeviceInfo, DibError, ServiceFamilies};
use crate::core::hpai::{Hpai, HpaiError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchResponse {
    pub control_endpoint: Hpai,
    pub device_info: DeviceInfo,
    pub service_families: Option<ServiceFamilies>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiscoveryError {
    TooShort { needed: usize, got: usize },
    Hpai(HpaiError),
    Dib(DibError),
    MissingDeviceInfo,
}

impl std::fmt::Display for DiscoveryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DiscoveryError::TooShort { needed, got } => write!(
                f,
                "SEARCH_RESPONSE body too short: needed {needed} octets, got {got}"
            ),
            DiscoveryError::Hpai(e) => write!(f, "{e}"),
            DiscoveryError::Dib(e) => write!(f, "{e}"),
            DiscoveryError::MissingDeviceInfo => {
                write!(f, "SEARCH_RESPONSE has no Device Info DIB")
            }
        }
    }
}

impl std::error::Error for DiscoveryError {}

impl From<HpaiError> for DiscoveryError {
    fn from(e: HpaiError) -> Self {
        DiscoveryError::Hpai(e)
    }
}

impl From<DibError> for DiscoveryError {
    fn from(e: DibError) -> Self {
        DiscoveryError::Dib(e)
    }
}

/// Core v01.06.02 AS §7.4.1, Figure 20: `SEARCH_REQUEST`'s body is just the
/// discovery HPAI a gateway should send its `SEARCH_RESPONSE` back to.
pub fn encode_search_request(discovery_endpoint: Hpai) -> Vec<u8> {
    discovery_endpoint.encode().to_vec()
}

/// Core v01.06.02 AS §7.4.1, Figure 21: `SEARCH_RESPONSE`'s body is the
/// gateway's control HPAI, followed by a mandatory Device Info DIB and
/// zero or more further DIBs (this cycle only interprets Supported
/// Service Families; any other DIB type — a future one this client
/// doesn't know about yet — is skipped by its own length prefix rather
/// than rejected, per Core v01.06.02 AS §6.2/§6.3's "ignore what you don't
/// understand" rule).
pub fn decode_search_response(body: &[u8]) -> Result<SearchResponse, DiscoveryError> {
    let (control_endpoint, mut rest) = Hpai::decode(body)?;

    let mut device_info = None;
    let mut service_families = None;
    while !rest.is_empty() {
        let dib_len = rest[0] as usize;
        if dib_len == 0 || dib_len > rest.len() {
            return Err(DiscoveryError::TooShort {
                needed: dib_len,
                got: rest.len(),
            });
        }
        let type_code = rest.get(1).copied().unwrap_or(0);
        match type_code {
            dib::DEVICE_INFO => {
                let (info, tail) = dib::decode_device_info(rest)?;
                device_info = Some(info);
                rest = tail;
            }
            dib::SUPP_SVC_FAMILIES => {
                let (families, tail) = dib::decode_service_families(rest)?;
                service_families = Some(families);
                rest = tail;
            }
            _ => {
                // Unknown DIB type: skip it by its own length prefix.
                rest = &rest[dib_len..];
            }
        }
    }

    Ok(SearchResponse {
        control_endpoint,
        device_info: device_info.ok_or(DiscoveryError::MissingDeviceInfo)?,
        service_families,
    })
}
```

Also add `pub mod discovery;` to `crates/knx-net/src/lib.rs`, alongside the existing `pub mod cemi;`/`pub mod client;`/`pub mod core;`/`pub mod frame;`/`pub mod tunnelling;`.

- [ ] **Step 5: Run to verify it passes**

Run: `cargo test -p knx-net -- core::services:: discovery::`
Expected: PASS (1 new `services` assertion folded into the existing test, 5 `discovery` tests).

- [ ] **Step 6: Commit**

```bash
git add crates/knx-net/src/discovery.rs crates/knx-net/src/core/services.rs crates/knx-net/src/lib.rs
git -c user.email=github@knxbench.com commit -m "feat(knx-net): SEARCH_REQUEST/SEARCH_RESPONSE codec"
```

---

### Task 3: `client` — `KnxNetIpClient::discover`

**Files:**
- Modify: `crates/knx-net/src/client.rs`
- Modify: `crates/knx-net/src/lib.rs` (re-export `DiscoveredGateway`)

**Interfaces:**
- Consumes: `discovery::{encode_search_request, decode_search_response}` (Task 2), `core::dib::SERVICE_FAMILY_TUNNELLING` (Task 1), `core::services::{SEARCH_REQUEST, SEARCH_RESPONSE}` (Task 2), `frame::{encode_frame, decode_frame}` (Cycle 1), `local_hpai` (already private in `client.rs`, Cycle 1).
- Produces: `pub struct DiscoveredGateway { pub control_endpoint: SocketAddrV4, pub individual_address: IndividualAddress, pub friendly_name: String, pub supports_tunnelling: bool }`, `BusConnection::discover`'s signature changes to `async fn discover(&self) -> Result<Vec<DiscoveredGateway>, BusError>`. Used by `apps/knx-cli` (Task 5) and `tests/live_gateway.rs` (Task 4).

`discover()` needs a real socket and a real peer to exercise meaningfully — same situation Cycle 1's `client.rs` task was already in (its plan: *"This task has no isolated unit test of its own ... its correctness is exercised end-to-end by [the] live test"*). This task's wire-format correctness is already covered by Task 1's/Task 2's unit tests; this step is "implement, then confirm the whole crate still builds and every earlier task's tests still pass" — the live-hardware round trip is Task 4.

- [ ] **Step 1: Implement**

Add near the top of `client.rs`, alongside the existing `use` block:

```rust
use std::net::Ipv4Addr;

use crate::core::dib;
use crate::core::services;
use crate::discovery;
```

(`SocketAddr, SocketAddrV4` are already imported; add `Ipv4Addr` to that same `use std::net::{...}` line rather than a new one.)

Add the discovery multicast constant and timeout, near `TunnelState`/`TunnelClient`:

```rust
/// Standard KNXnet/IP discovery/routing multicast group and port (Core
/// v01.06.02 AS §4.2). Hardcoded this cycle — not a CLI override (design
/// spec's Cycle 3 Q2).
const DISCOVERY_MULTICAST: SocketAddrV4 = SocketAddrV4::new(Ipv4Addr::new(224, 0, 23, 12), 3671);

/// Core v01.06.02 AS §5.2.4 `SEARCH_TIMEOUT`: how long to keep collecting
/// `SEARCH_RESPONSE`s after sending one `SEARCH_REQUEST` (design spec's
/// Cycle 3 Q3 — full spec value, not a shortened one).
const SEARCH_TIMEOUT_SECS: u64 = 10;
```

Add the new struct, next to `BusError`:

```rust
/// A gateway found via `discover()` (Core v01.06.02 AS §7.4.1's
/// `SEARCH_RESPONSE`). `supports_tunnelling` is derived from the
/// Supported Service Families DIB, when the gateway sends one — some
/// gateways only support routing, and this is how a caller finds out
/// before trying `connect_tunnel` on it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoveredGateway {
    pub control_endpoint: SocketAddrV4,
    pub individual_address: IndividualAddress,
    pub friendly_name: String,
    pub supports_tunnelling: bool,
}
```

Change the trait signature:

```rust
#[allow(async_fn_in_trait)]
pub trait BusConnection {
    async fn discover(&self) -> Result<Vec<DiscoveredGateway>, BusError>;
    async fn connect_tunnel(&self, gateway: SocketAddrV4) -> Result<TunnelClient, BusError>;
}
```

Replace the `discover` stub in `impl BusConnection for KnxNetIpClient`:

```rust
impl BusConnection for KnxNetIpClient {
    async fn discover(&self) -> Result<Vec<DiscoveredGateway>, BusError> {
        let socket = UdpSocket::bind("0.0.0.0:0").await.map_err(BusError::Io)?;
        let discovery_endpoint = local_hpai(&socket)?;
        let request_body = discovery::encode_search_request(discovery_endpoint);
        let datagram = frame::encode_frame(services::SEARCH_REQUEST, &request_body);
        socket
            .send_to(&datagram, DISCOVERY_MULTICAST)
            .await
            .map_err(BusError::Io)?;

        let mut gateways: Vec<DiscoveredGateway> = Vec::new();
        let deadline = tokio::time::Instant::now() + Duration::from_secs(SEARCH_TIMEOUT_SECS);
        let mut buf = [0u8; 1024];
        loop {
            let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
            if remaining.is_zero() {
                break;
            }
            let Ok(Ok((n, _src))) =
                tokio::time::timeout(remaining, socket.recv_from(&mut buf)).await
            else {
                break; // window elapsed, or the socket errored: stop collecting
            };
            let Ok((header, resp_body)) = frame::decode_frame(&buf[..n]) else {
                continue;
            };
            if header.service_type != services::SEARCH_RESPONSE {
                continue;
            }
            let Ok(response) = discovery::decode_search_response(resp_body) else {
                continue;
            };
            let control_endpoint = SocketAddrV4::new(
                response.control_endpoint.addr,
                response.control_endpoint.port,
            );
            if gateways.iter().any(|g| g.control_endpoint == control_endpoint) {
                continue;
            }
            let supports_tunnelling = response
                .service_families
                .as_ref()
                .is_some_and(|f| f.supports(dib::SERVICE_FAMILY_TUNNELLING));
            gateways.push(DiscoveredGateway {
                control_endpoint,
                individual_address: response.device_info.individual_address,
                friendly_name: response.device_info.friendly_name,
                supports_tunnelling,
            });
        }
        Ok(gateways)
    }

    async fn connect_tunnel(&self, gateway: SocketAddrV4) -> Result<TunnelClient, BusError> {
        TunnelClient::connect(gateway).await
    }
}
```

Update the two doc comments that currently say `discover` is unimplemented:
- The module doc comment at the top of `client.rs` (currently: `"discover remains an explicit BusError::NotImplemented stub..."`) — change to note `discover` is implemented as of Session 6 Cycle 3.
- `KnxNetIpClient`'s own doc comment (currently: `"Cycle 1 implements connect_tunnel only; discover ... is a later cycle's work."`) — same update.

In `crates/knx-net/src/lib.rs`, add `DiscoveredGateway` to the existing re-export line:

```rust
pub use client::{BusConnection, BusError, DiscoveredGateway, KnxNetIpClient, TunnelClient};
```

Also update `lib.rs`'s crate-level doc comment (currently: `"discover remains an explicit NotImplemented stub..."`) to match.

- [ ] **Step 2: Run to verify it builds and every earlier task's tests still pass**

Run: `cargo test -p knx-net`
Expected: PASS — every Task 1/Task 2 test still passes; no test of `discover` itself yet (that's Task 4).

Run: `cargo clippy -p knx-net --all-targets -- -D warnings`
Expected: clean.

- [ ] **Step 3: Commit**

```bash
git add crates/knx-net/src/client.rs crates/knx-net/src/lib.rs
git -c user.email=github@knxbench.com commit -m "feat(knx-net): implement KnxNetIpClient::discover"
```

---

### Task 4: Live-gateway integration test

**Files:**
- Modify: `crates/knx-net/tests/live_gateway.rs`

**Interfaces:**
- Consumes: `knx_net::{BusConnection, KnxNetIpClient}` (Task 3), `gateway_addr()` (already defined in this file, Cycle 1).
- Produces: nothing new for other tasks — this is a leaf, `#[ignore]`d, human-run-only test.

- [ ] **Step 1: Add the test**

Append to `crates/knx-net/tests/live_gateway.rs`:

```rust
/// Multicasts a `SEARCH_REQUEST` and expects the known reference gateway
/// to answer with tunnelling support advertised. Needs LAN access to the
/// discovery multicast group (`224.0.23.12:3671`) — not just a route to
/// one known gateway IP, unlike the other tests in this file.
#[tokio::test]
#[ignore = "needs a real KNXnet/IP gateway reachable via multicast on the LAN"]
async fn discovers_the_known_gateway_with_tunnelling_support() {
    let client = knx_net::KnxNetIpClient::new();
    let gateways = client
        .discover()
        .await
        .expect("discover against a real network");
    let expected = gateway_addr();
    let found = gateways
        .iter()
        .find(|g| g.control_endpoint == expected)
        .unwrap_or_else(|| panic!("expected gateway {expected} among {gateways:?}"));
    assert!(
        found.supports_tunnelling,
        "expected the known gateway to advertise tunnelling support"
    );
}
```

- [ ] **Step 2: Run to verify it compiles (it will report `ignored`, not run, in this environment)**

Run: `cargo test -p knx-net --test live_gateway`
Expected: compiles; the new test (and the two existing ones) show as `ignored`, 0 failures.

- [ ] **Step 3: Commit**

```bash
git add crates/knx-net/tests/live_gateway.rs
git -c user.email=github@knxbench.com commit -m "test(knx-net): live-gateway discovery test"
```

---

### Task 5: `knx bus discover` CLI subcommand

**Files:**
- Modify: `apps/knx-cli/src/main.rs`

**Interfaces:**
- Consumes: `knx_net::{BusConnection, KnxNetIpClient, DiscoveredGateway}` (Task 3), `run_bus` (existing dispatcher, `Some("monitor") => ...` / `Some("write") => ...` match arms).
- Produces: `fn run_bus_discover(args: &[String]) -> ExitCode`. Nothing downstream depends on it — this is the plan's leaf CLI-facing task.

- [ ] **Step 1: Update `USAGE`**

In `apps/knx-cli/src/main.rs`, extend the `USAGE` constant (it currently lists `bus monitor` and `bus write`):

```rust
const USAGE: &str =
    "usage: knx import <file.knxproj> [--store <path.knxdb>] [--report-json <path.json>]\n\
     \x20                  [--product-db <path>] [--no-product-db]\n\
     \x20     knx products list [--manufacturer M-xxxx] [--product-db <path>]\n\
     \x20     knx products ingest <file.knxproj> [--product-db <path>]\n\
     \x20     knx products show <program-id> [--product-db <path>]\n\
     \x20     knx products verify [--product-db <path>]\n\
     \x20     knx bus discover\n\
     \x20     knx bus monitor --gateway <host:port> [--project <path.knxdb>]\n\
     \x20     knx bus write --gateway <host:port> <main/middle/sub> <0|1|hex>\n\
     exit codes: 0 = imported cleanly (warnings allowed), 1 = could not import,\n\
     2 = imported, but the report contains errors";
```

- [ ] **Step 2: Add the dispatch arm**

In `run_bus`, add `discover` alongside `monitor`/`write`:

```rust
fn run_bus(args: &[String]) -> ExitCode {
    match args.first().map(String::as_str) {
        Some("discover") => run_bus_discover(&args[1..]),
        Some("monitor") => run_bus_monitor(&args[1..]),
        Some("write") => run_bus_write(&args[1..]),
        _ => {
            eprintln!("{USAGE}");
            ExitCode::FAILURE
        }
    }
}
```

- [ ] **Step 3: Implement `run_bus_discover`/`run_bus_discover_async`**

Add near `run_bus_monitor`/`run_bus_write`:

```rust
/// `knx bus discover` — multicasts a `SEARCH_REQUEST` and prints every
/// gateway that answers within the spec's 10s window. Takes no arguments;
/// that's the point (no `--gateway` to already know).
fn run_bus_discover(args: &[String]) -> ExitCode {
    if !args.is_empty() {
        eprintln!("knx bus discover takes no arguments\n{USAGE}");
        return ExitCode::FAILURE;
    }
    let runtime = match tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    {
        Ok(rt) => rt,
        Err(e) => {
            eprintln!("could not start async runtime: {e}");
            return ExitCode::FAILURE;
        }
    };
    runtime.block_on(run_bus_discover_async())
}

async fn run_bus_discover_async() -> ExitCode {
    use knx_net::BusConnection;
    let client = knx_net::KnxNetIpClient::new();
    match client.discover().await {
        Ok(gateways) if gateways.is_empty() => {
            println!("no gateways responded");
            ExitCode::SUCCESS
        }
        Ok(gateways) => {
            for g in gateways {
                let tag = if g.supports_tunnelling { " [tunnelling]" } else { "" };
                println!(
                    "{}  {}  {}{}",
                    g.individual_address, g.friendly_name, g.control_endpoint, tag
                );
            }
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("discovery failed: {e}");
            ExitCode::FAILURE
        }
    }
}
```

- [ ] **Step 4: Build and smoke-check argument handling**

Run: `cargo build -p knx-cli`
Expected: builds cleanly.

Run: `cargo run -p knx-cli -- bus discover extra-arg`
Expected: prints `knx bus discover takes no arguments` plus usage, exits non-zero. (Actually running discovery against a live network is this cycle's own manual-check item, documented in Task 6 — this step only checks argument parsing, no network access needed.)

- [ ] **Step 5: Commit**

```bash
git add apps/knx-cli/src/main.rs
git -c user.email=github@knxbench.com commit -m "feat(knx-cli): knx bus discover subcommand"
```

---

### Task 6: Documentation updates

**Files:**
- Modify: `docs/RESEARCH.md`
- Modify: `docs/ARCHITECTURE.md`
- Modify: `docs/IMPLEMENTATION_STATUS.md`
- Modify: `docs/KNOWN_LIMITATIONS.md`

**Interfaces:**
- Consumes: nothing (documentation only).
- Produces: nothing (leaf task).

- [ ] **Step 1: `docs/RESEARCH.md` §8.1**

Add a new paragraph after the existing "Session 6, Cycle 1" one:

```markdown
**Session 6, Cycle 3 (2026-09):** `discover()` implemented — a
`SEARCH_REQUEST` multicast to `224.0.23.12:3671` (Core v01.06.02 AS §4.2),
collecting `SEARCH_RESPONSE`s for the full 10s `SEARCH_TIMEOUT` and
parsing both the Device Info and Supported Service Families DIBs
(§7.5.4.2/§7.5.4.3). `crates/knx-net` gained `core::dib` and `discovery`,
plus a `knx bus discover` CLI subcommand. Live verification against the
reference gateway — confirming it answers and advertises tunnelling
support — is the next step, to be run by the user via
`cargo test -p knx-net -- --ignored` and `knx bus discover` on their own
LAN.
```

- [ ] **Step 2: `docs/ARCHITECTURE.md` §8**

Replace the sentence `"discover and TunnelClient::send remain explicit BusError::NotImplemented stubs; later cycles cover them, plus routing and KNX IP Secure."` with:

```markdown
`TunnelClient::send` (Cycle 2) and `discover` (Cycle 3, multicast
`SEARCH_REQUEST`/`SEARCH_RESPONSE`) are both implemented now; routing and
KNX IP Secure remain later cycles.
```

- [ ] **Step 3: `docs/IMPLEMENTATION_STATUS.md`**

Add a new entry after the existing "Session 6, Cycle 2" one, matching that entry's style:

```markdown
**Session 6, Cycle 3 (2026-09-06) — KNXnet/IP discovery.** Own design
spec (`docs/superpowers/specs/2026-09-06-knxnet-ip-discovery-design.md`),
per the brainstorming skill's classification: a new subsystem, not an
extension of Cycle 1/2's existing tunnel connection. `core::dib` decodes
the Device Info and Supported Service Families DIBs a `SEARCH_RESPONSE`
carries; `discovery` encodes `SEARCH_REQUEST` and decodes
`SEARCH_RESPONSE` bodies, skipping any DIB type it doesn't recognize
rather than rejecting the whole response. `KnxNetIpClient::discover` no
longer returns `BusError::NotImplemented`: it multicasts one
`SEARCH_REQUEST` to `224.0.23.12:3671` and collects replies for the full
spec `SEARCH_TIMEOUT` (10s), deduping by control endpoint. Only the
original `SEARCH_REQUEST`/`SEARCH_RESPONSE` form is implemented — not
`SEARCH_REQUEST_EXTENDED`/`RESPONSE_EXTENDED` (Core v2) — and the
multicast address/timeout are hardcoded constants, not CLI flags, both
deliberate scope cuts recorded in the design spec. `knx bus discover`
(no arguments — that's the feature) is the CLI-facing piece.
`crates/knx-net/tests/live_gateway.rs` gained a third `#[ignore]`d test
that multicasts for real and checks the reference gateway both answers
and advertises tunnelling support. Live-hardware verification was left
for the user to run, same as Cycle 2's `send` — choosing when to probe
the LAN isn't this session's call to make unsupervised.

Known gaps added this cycle (not bugs, scope decisions):

- `SEARCH_REQUEST_EXTENDED`/`RESPONSE_EXTENDED` (Core v2) are not
  implemented — no gateway encountered so far has needed them.
- The discovery multicast group/port and the collection timeout are
  hardcoded constants; no CLI override exists yet.
- Discovery does not work unmodified inside the `knx-server` Docker
  container (needs `--network host`) — a known, not-yet-solved
  constraint (ROADMAP.md, Session 6 entry); `knx-server` does not call
  `discover` yet, so nothing regresses, but the gap is now reachable from
  a CLI a container user might reasonably try.
```

- [ ] **Step 4: `docs/KNOWN_LIMITATIONS.md` §26**

Update the limitation to narrow it now that discovery is implemented. Replace the `**Limitation.**` paragraph's `discover` sentence and the `**Cause.**`/`**Impact.**`/`**Lifted when.**` paragraphs:

```markdown
**Limitation.** `crates/knx-net`'s `BusConnection` trait implements
tunnelling and discovery: `discover` multicasts a `SEARCH_REQUEST`
(original form only, not Core v2's `SEARCH_REQUEST_EXTENDED`) and
collects `SEARCH_RESPONSE`s; `connect_tunnel` opens a tunnel to a
gateway by known IP; `subscribe` receives telegrams; `TunnelClient::send`
writes one (`GroupValueWrite` or any other `ApplicationService`, no DPT
interpretation — raw bytes only, same scope cut as the receive side).
Routing and all secure-protocol paths remain `BusError::NotImplemented`
stubs. A reader should not assume the trait is feature-complete because
it compiles.

**Cause.** Session 6 Cycle 1 delivered read-only tunnelling as the
foundation for bus monitoring; Cycle 2 added sending; Cycle 3 added
discovery. Routing is still a later cycle. Secure protocols are out of
v1 scope, handled by the isolated `knx-secure` crate.

**Impact.** A real KNX installation's gateways can now be found on the
LAN without a known IP, and monitored/actuated by group address over a
tunnel, but it still cannot be reached over routing (multicast) or KNX
IP Secure. Discovery does not work unmodified inside the `knx-server`
Docker container (needs `--network host`) — untouched by this cycle,
since `knx-server` doesn't call `discover` yet.

**Lifted when.** Routing and KNX IP Secure each land in their own later
cycle of Session 6, or in Session 7.
```

- [ ] **Step 5: Verify docs build/lint clean and commit**

Run: `cargo fmt --all --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`
Expected: all clean, all passing (ignored tests reported as ignored, not failed).

```bash
git add docs/RESEARCH.md docs/ARCHITECTURE.md docs/IMPLEMENTATION_STATUS.md docs/KNOWN_LIMITATIONS.md
git -c user.email=github@knxbench.com commit -m "docs: Session 6 Cycle 3 KNXnet/IP discovery"
```
