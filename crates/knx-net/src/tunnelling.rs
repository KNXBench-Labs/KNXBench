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
            return Err(TunnellingError::TooShort {
                needed: 4,
                got: buf.len(),
            });
        }
        if buf[0] != 0x04 {
            return Err(TunnellingError::BadStructureLength(buf[0]));
        }
        if buf[1] != TUNNEL_CONNECTION {
            return Err(TunnellingError::UnexpectedConnectionType(buf[1]));
        }
        let raw = u16::from_be_bytes([buf[2], buf[3]]);
        Ok(TunnelCrd {
            individual_address: IndividualAddress::from_raw(raw),
        })
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
                write!(
                    f,
                    "tunnelling structure too short: needed {needed} octets, got {got}"
                )
            }
            TunnellingError::BadStructureLength(len) => {
                write!(f, "unexpected structure length {len:#04x}, expected 0x04")
            }
            TunnellingError::UnexpectedConnectionType(code) => {
                write!(
                    f,
                    "unexpected connection type {code:#04x}, expected TUNNEL_CONNECTION"
                )
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
        return Err(TunnellingError::TooShort {
            needed: 4,
            got: body.len(),
        });
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
        return Err(TunnellingError::TooShort {
            needed: 4,
            got: body.len(),
        });
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

#[cfg(test)]
mod tests {
    use super::*;
    use knx_core::IndividualAddress;

    #[test]
    fn basic_cri_encodes_layer_and_connection_type() {
        let cri = TunnelCri {
            layer: TUNNEL_LINKLAYER,
        };
        assert_eq!(
            cri.encode(),
            [0x04, TUNNEL_CONNECTION, TUNNEL_LINKLAYER, 0x00]
        );
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
        assert_eq!(
            ack,
            TunnellingAck {
                channel_id: 0x15,
                sequence_counter: 0x00,
                status: E_NO_ERROR
            }
        );
    }

    #[test]
    fn tunnelling_ack_round_trips() {
        let body = encode_tunnelling_ack(0x21, 0x03, E_NO_ERROR);
        let ack = decode_tunnelling_ack(&body).unwrap();
        assert_eq!(
            ack,
            TunnellingAck {
                channel_id: 0x21,
                sequence_counter: 0x03,
                status: E_NO_ERROR
            }
        );
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
