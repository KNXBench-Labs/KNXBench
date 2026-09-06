//! `ROUTING_LOST_MESSAGE`/`ROUTING_BUSY` body decoding (Routing v01.05.02 AS
//! §6.2/§6.3) — decoded far enough to log, never acted on (design spec's
//! Cycle 4 scope cut: no send-throttle reaction to `ROUTING_BUSY`).
//! `ROUTING_INDICATION` needs no decoder here: its body is exactly an
//! `L_Data.ind` cEMI frame, already handled by `cemi::decode_l_data`/
//! `encode_l_data` and `frame::decode_frame`/`encode_frame` — `client.rs`
//! wires those together directly.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RoutingLostMessage {
    pub device_state: u8,
    pub lost_message_count: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RoutingBusy {
    pub device_state: u8,
    pub wait_time_ms: u16,
    pub control_field: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RoutingError {
    TooShort { needed: usize, got: usize },
}

impl std::fmt::Display for RoutingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RoutingError::TooShort { needed, got } => write!(
                f,
                "routing frame body too short: needed {needed} octets, got {got}"
            ),
        }
    }
}

impl std::error::Error for RoutingError {}

/// Routing v01.05.02 AS §6.2: 4-octet body — structure length (unused,
/// always 04h), device state, 2-octet lost-message count (big-endian).
pub fn decode_routing_lost_message(buf: &[u8]) -> Result<RoutingLostMessage, RoutingError> {
    if buf.len() < 4 {
        return Err(RoutingError::TooShort {
            needed: 4,
            got: buf.len(),
        });
    }
    Ok(RoutingLostMessage {
        device_state: buf[1],
        lost_message_count: u16::from_be_bytes([buf[2], buf[3]]),
    })
}

/// Routing v01.05.02 AS §5.4/§6.3: 6-octet body — structure length
/// (unused), device state, 2-octet wait time in ms, 2-octet control field
/// (both big-endian).
pub fn decode_routing_busy(buf: &[u8]) -> Result<RoutingBusy, RoutingError> {
    if buf.len() < 6 {
        return Err(RoutingError::TooShort {
            needed: 6,
            got: buf.len(),
        });
    }
    Ok(RoutingBusy {
        device_state: buf[1],
        wait_time_ms: u16::from_be_bytes([buf[2], buf[3]]),
        control_field: u16::from_be_bytes([buf[4], buf[5]]),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Routing v01.05.02 AS §6.2's worked example: structure length 04h,
    /// device state 00h, lost-message count 0005h.
    #[test]
    fn decode_routing_lost_message_matches_spec_worked_example() {
        let buf = [0x04, 0x00, 0x00, 0x05];
        let msg = decode_routing_lost_message(&buf).unwrap();
        assert_eq!(msg.device_state, 0x00);
        assert_eq!(msg.lost_message_count, 5);
    }

    #[test]
    fn decode_routing_lost_message_rejects_too_short_buffer() {
        let err = decode_routing_lost_message(&[0x04, 0x00, 0x00]).unwrap_err();
        assert_eq!(
            err,
            RoutingError::TooShort {
                needed: 4,
                got: 3
            }
        );
    }

    /// Hand-built from §5.4's field layout: structure length 04h, device
    /// state 01h, wait time 100ms (0064h), control field 0000h.
    #[test]
    fn decode_routing_busy_matches_hand_built_fixture() {
        let buf = [0x04, 0x01, 0x00, 0x64, 0x00, 0x00];
        let busy = decode_routing_busy(&buf).unwrap();
        assert_eq!(busy.device_state, 0x01);
        assert_eq!(busy.wait_time_ms, 100);
        assert_eq!(busy.control_field, 0x0000);
    }

    #[test]
    fn decode_routing_busy_rejects_too_short_buffer() {
        let err = decode_routing_busy(&[0x04, 0x01, 0x00, 0x64, 0x00]).unwrap_err();
        assert_eq!(
            err,
            RoutingError::TooShort {
                needed: 6,
                got: 5
            }
        );
    }
}
