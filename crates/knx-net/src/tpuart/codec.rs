//! Translate standard TP1 frames and NCN5130 services without duplicating the cEMI codec.

use crate::{cemi, BusError, LDataFrame};

pub(super) fn protocol(message: impl Into<String>) -> BusError {
    BusError::Protocol(format!("TP-UART: {}", message.into()))
}

/// NCN5130 Table 12 / Send Frame Service: FCS occupies the final index.
pub(super) fn encode(frame: &LDataFrame) -> Result<Vec<u8>, BusError> {
    if frame.kind != cemi::LDataMessageKind::Request {
        return Err(protocol("only L_Data.req can be sent"));
    }
    if frame.destination == cemi::Destination::SystemBroadcast {
        return Err(protocol(
            "system broadcast is unsupported by this standard TP1 adapter",
        ));
    }
    let cemi = cemi::encode_l_data(frame).map_err(|e| protocol(e.to_string()))?;
    if cemi[2] & 0x80 == 0 || cemi[8] > 15 {
        return Err(protocol(
            "extended frames are unsupported (maximum TPDU: 16 octets)",
        ));
    }
    // cEMI has a separate Ctrl2 and L; TP1 packs AT/HC and L into one octet.
    // TP1's low two control bits are reserved, not cEMI A/C.
    let mut tp = vec![cemi[2] & 0xfc];
    tp.extend_from_slice(&cemi[4..8]);
    tp.push((cemi[3] & 0xf0) | cemi[8]);
    tp.extend_from_slice(&cemi[9..]);
    let fcs = tp.iter().fold(0xff, |xor, b| xor ^ b);
    tp.push(fcs);
    let mut wire = Vec::with_capacity(tp.len() * 2);
    for (index, byte) in tp.iter().copied().enumerate() {
        let command = if index + 1 == tp.len() {
            0x40 | index as u8
        } else {
            0x80 | index as u8
        };
        wire.extend_from_slice(&[command, byte]);
    }
    Ok(wire)
}

fn decode(tp: &[u8]) -> Result<LDataFrame, BusError> {
    if tp.len() < 8 || tp.len() != 8 + usize::from(tp[5] & 15) {
        return Err(protocol("invalid standard frame length"));
    }
    if tp.iter().fold(0, |xor, b| xor ^ b) != 0xff {
        return Err(protocol("invalid TP1 checksum"));
    }
    let mut cemi = vec![cemi::L_DATA_IND, 0, tp[0], tp[5] & 0xf0];
    cemi.extend_from_slice(&tp[1..5]);
    cemi.push(tp[5] & 15);
    cemi.extend_from_slice(&tp[6..tp.len() - 1]);
    cemi::decode_l_data(&cemi).map_err(|e| protocol(e.to_string()))
}

/// Compare complete TP1 octets, not decoded APCI equivalence. The request encoder
/// already strips cEMI A/C. Only the native repeat bit may change on the bus;
/// each FCS was validated separately and necessarily changes with that bit.
pub(super) fn matches_echo(expected: &[u8], received: &[u8]) -> bool {
    expected.len() == received.len()
        && expected.len() >= 8
        && expected[0] & !0x20 == received[0] & !0x20
        && expected[1..expected.len() - 1] == received[1..received.len() - 1]
}

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Service {
    Reset,
    State(u8),
    Configure(u8),
    Confirmation(bool),
    Acknowledgement(u8),
    FrameState,
    Telegram(LDataFrame, Vec<u8>),
}

#[derive(Default)]
pub(super) struct Parser {
    frame: Vec<u8>,
}

impl Parser {
    pub(super) fn assembling(&self) -> bool {
        !self.frame.is_empty()
    }

    pub(super) fn push(&mut self, byte: u8) -> Result<Option<Service>, BusError> {
        if !self.frame.is_empty() {
            self.frame.push(byte);
            if self.frame.len() >= 6 {
                let total = 8 + usize::from(self.frame[5] & 15);
                if self.frame.len() == total {
                    let tp = std::mem::take(&mut self.frame);
                    return decode(&tp).map(|f| Some(Service::Telegram(f, tp)));
                }
            }
            return Ok(None);
        }
        // NCN5130 Table 13: frame bytes are opaque until the length is satisfied.
        // Fail closed rather than guessing a boundary after corruption or extensions.
        if byte & 0xd3 == 0x90 {
            self.frame.push(byte);
            return Ok(None);
        }
        if byte & 0xd3 == 0x10 {
            return Err(protocol("extended receive frame is unsupported"));
        }
        let service = match byte {
            0x03 => Service::Reset,
            0x8b => Service::Confirmation(true),
            0x0b => Service::Confirmation(false),
            0x13 => Service::FrameState,
            b if b & 0x17 == 0x13 => return Err(protocol(format!("frame-state fault {b:#04x}"))),
            7 => Service::State(7),
            b if b & 7 == 7 => return Err(protocol(format!("state fault {b:#04x}"))),
            b if b & 0x83 == 1 => Service::Configure(b),
            b if b & 0x33 == 0 => Service::Acknowledgement(b),
            _ => return Err(protocol(format!("unknown/unsupported service {byte:#04x}"))),
        };
        Ok(Some(service))
    }
}
