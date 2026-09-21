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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::dib::{ServiceFamily, DEVICE_INFO, SUPP_SVC_FAMILIES};
    use std::net::Ipv4Addr;

    fn control_hpai() -> Hpai {
        Hpai {
            addr: Ipv4Addr::new(192, 0, 2, 1),
            port: 3671,
        }
    }

    fn device_info_bytes(individual_address_raw: u16) -> Vec<u8> {
        let mut b = vec![0x36, DEVICE_INFO, 0x02, 0x00];
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
        assert_eq!(
            families.0,
            vec![ServiceFamily {
                id: 0x04,
                version: 0x01
            }]
        );
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

    #[test]
    fn decode_search_response_rejects_dib_with_zero_length() {
        // Regression: DIB claiming length 0 must not panic (Core v01.06.02
        // AS §6.2/§6.3 "ignore what you don't understand" requires safe skip).
        let mut body = control_hpai().encode().to_vec();
        body.push(0x00); // DIB length 0 is invalid
        body.push(0xFE); // DIB type (won't matter if length is 0)

        let err = decode_search_response(&body).unwrap_err();
        // Must error gracefully, not panic.
        assert!(matches!(err, DiscoveryError::TooShort { .. }));
    }

    #[test]
    fn decode_search_response_rejects_dib_claiming_more_than_buffer_has() {
        // Regression: DIB claiming length > remaining buffer must not panic
        // or read out of bounds (Core v01.06.02 AS §6.2/§6.3 robust parsing).
        let mut body = control_hpai().encode().to_vec();
        body.extend_from_slice(&[0xFF, 0xFE]); // Claims 255-byte DIB, only 2 bytes present

        let err = decode_search_response(&body).unwrap_err();
        // Must error gracefully, not panic or read past end.
        assert!(matches!(err, DiscoveryError::TooShort { .. }));
    }

    #[test]
    fn decode_search_response_rejects_truncated_dib() {
        // Regression: buffer truncated mid-DIB must not panic. Device Info
        // DIB requires 54 bytes (Core v01.06.02 AS §7.5.4.2); a partial one
        // followed by actual end-of-buffer must error, not crash.
        let mut body = control_hpai().encode().to_vec();
        // Start a Device Info DIB claiming 54 bytes total, but only provide 20.
        body.extend_from_slice(&[54, crate::core::dib::DEVICE_INFO]);
        body.extend_from_slice(&[0x02, 0x00]); // medium, status
        body.extend_from_slice(&[0x11, 0x01]); // individual address
        body.extend_from_slice(&[0x00; 16]); // Just enough to reach 20 bytes total
                                             // But Device Info needs 54; will error when trying to decode.

        let err = decode_search_response(&body).unwrap_err();
        // Must error gracefully, not panic or read out of bounds.
        assert!(matches!(
            err,
            DiscoveryError::Dib(_) | DiscoveryError::TooShort { .. }
        ));
    }
}
