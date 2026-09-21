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
                write!(
                    f,
                    "unexpected HPAI structure length {len:#04x}, expected {HPAI_LEN:#04x}"
                )
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
            return Err(HpaiError::TooShort {
                needed: HPAI_LEN as usize,
                got: buf.len(),
            });
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::Ipv4Addr;

    #[test]
    fn encode_then_decode_round_trips() {
        let hpai = Hpai {
            addr: Ipv4Addr::new(192, 0, 2, 1),
            port: 3671,
        };
        let bytes = hpai.encode();
        assert_eq!(bytes, [0x08, 0x01, 192, 0, 2, 1, 0x0E, 0x57]);

        let (decoded, rest) = Hpai::decode(&bytes).unwrap();
        assert_eq!(decoded, hpai);
        assert!(rest.is_empty());
    }

    #[test]
    fn decode_leaves_trailing_bytes_for_the_caller() {
        let mut bytes = Hpai {
            addr: Ipv4Addr::new(203, 0, 113, 1),
            port: 1,
        }
        .encode()
        .to_vec();
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
