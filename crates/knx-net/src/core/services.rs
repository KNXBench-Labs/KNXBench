//! Core connection-management services: CONNECT/CONNECTIONSTATE/DISCONNECT
//! (Core v01.06.02 AS §7.4.1, §7.8). The CRI/CRD bodies these frames wrap
//! are connection-type specific (§7.5.2/§7.5.3 defer to the connection
//! type's own clause) — this module treats them as opaque byte slices;
//! `tunnelling.rs` owns their actual layout.

use super::hpai::Hpai;

// Service type identifiers, Core v01.06.02 AS §7.4.1.
// Discovery service type identifiers, Core v01.06.02 AS §7.4.1.
pub const SEARCH_REQUEST: u16 = 0x0201;
pub const SEARCH_RESPONSE: u16 = 0x0202;

// Routing service type identifiers, Routing v01.05.02 AS §5.1.
pub const ROUTING_INDICATION: u16 = 0x0530;
pub const ROUTING_LOST_MESSAGE: u16 = 0x0531;
pub const ROUTING_BUSY: u16 = 0x0532;

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
                write!(
                    f,
                    "service body too short: needed {needed} octets, got {got}"
                )
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

/// Core v01.06.02 AS §7.8.1's prose reads control HPAI, CRI, data HPAI —
/// but real gateways (and every interoperable stack, e.g. xknx's
/// `ConnectRequest.to_knx()`) put the data HPAI before the CRI. Verified
/// against a physical gateway: the prose order gets silently dropped (no
/// CONNECT_RESPONSE, ever), this order gets one every time. Wire format
/// wins over the paragraph.
pub fn encode_connect_request(control: Hpai, data: Hpai, cri: &[u8]) -> Vec<u8> {
    let mut body = Vec::with_capacity(8 + 8 + cri.len());
    body.extend_from_slice(&control.encode());
    body.extend_from_slice(&data.encode());
    body.extend_from_slice(cri);
    body
}

/// Core v01.06.02 AS §7.8.2, Figure 36: channel ID, status, then — only if
/// `status == E_NO_ERROR` — the server's data HPAI and the CRD.
pub fn decode_connect_response(body: &[u8]) -> Result<ConnectResponse, ServiceError> {
    if body.len() < 2 {
        return Err(ServiceError::TooShort {
            needed: 2,
            got: body.len(),
        });
    }
    let channel_id = body[0];
    let status = body[1];
    if status != E_NO_ERROR {
        return Ok(ConnectResponse {
            channel_id,
            status,
            server_data_hpai: None,
            crd: Vec::new(),
        });
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
pub fn decode_connectionstate_response(
    body: &[u8],
) -> Result<ConnectionStateResponse, ServiceError> {
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
        return Err(ServiceError::TooShort {
            needed: 2,
            got: body.len(),
        });
    }
    Ok((body[0], body[1]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::Ipv4Addr;

    fn hpai() -> Hpai {
        Hpai {
            addr: Ipv4Addr::new(10, 0, 0, 5),
            port: 50000,
        }
    }

    #[test]
    fn connect_request_lays_out_control_hpai_data_hpai_cri_in_order() {
        let cri = [0x04, 0x04, 0x02, 0x00]; // opaque to this module
        let body = encode_connect_request(hpai(), hpai(), &cri);
        let mut expected = hpai().encode().to_vec();
        expected.extend_from_slice(&hpai().encode());
        expected.extend_from_slice(&cri);
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
        assert_eq!(
            response,
            ConnectionStateResponse {
                channel_id: 0x15,
                status: E_NO_ERROR
            }
        );
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
        assert_eq!(
            response,
            DisconnectResponse {
                channel_id: 0x15,
                status: E_NO_ERROR
            }
        );
    }

    #[test]
    fn decode_rejects_short_body() {
        let err = decode_connectionstate_response(&[0x15]).unwrap_err();
        assert_eq!(err, ServiceError::TooShort { needed: 2, got: 1 });
    }

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
}
