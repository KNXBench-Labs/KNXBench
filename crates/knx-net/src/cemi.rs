//! cEMI `L_Data` frame decode (EMI_IMI v01.04.02 AS §4.1.4, §4.1.5.3).
//! This cycle only interprets the group-communication trio
//! (`A_GroupValue_Read/Response/Write`, Application Layer v02.01.01 AS
//! §2.2 Table 1); every other APCI is preserved as raw bytes
//! (`ApplicationService::Other`), never silently dropped.

use knx_core::{GroupAddress, IndividualAddress};
// Re-exported (not just imported) so `crate::cemi::GroupValue` keeps
// resolving for call sites that named this module directly — `GroupValue`
// itself now lives in `knx-core` (spec E4-D2).
pub use knx_core::GroupValue;

pub const L_DATA_REQ: u8 = 0x11;
pub const L_DATA_CON: u8 = 0x2E;
pub const L_DATA_IND: u8 = 0x29;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LDataMessageKind {
    Request,
    /// `error` is Ctrl1's Confirm flag (EMI_IMI v01.04.02 AS §4.1.5.3.4):
    /// `false` = no error, `true` = error.
    Confirmation {
        error: bool,
    },
    Indication,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Destination {
    Individual(IndividualAddress),
    Group(GroupAddress),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApplicationService {
    GroupValueRead,
    GroupValueResponse(GroupValue),
    GroupValueWrite(GroupValue),
    /// Any APCI this cycle does not interpret — the raw TPCI/APCI-high
    /// octet combined with the APCI-low octet, and every data octet that
    /// followed, so nothing is lost (CLAUDE.md: never silently discard).
    Other {
        apci: u16,
        data: Vec<u8>,
    },
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
                write!(
                    f,
                    "cEMI frame too short: needed at least {needed} octets, got {got}"
                )
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
        return Err(CemiError::TooShort {
            needed: 2,
            got: buf.len(),
        });
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
        return Err(CemiError::TooShort {
            needed: fixed_part_start + 7,
            got: buf.len(),
        });
    }
    let ctrl1 = buf[fixed_part_start];
    let ctrl2 = buf[fixed_part_start + 1];
    let kind = match kind {
        LDataMessageKind::Confirmation { .. } => LDataMessageKind::Confirmation {
            error: ctrl1 & 0x01 != 0,
        },
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
        return Err(CemiError::TooShort {
            needed: tpci_apci_start + 2,
            got: buf.len(),
        });
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
        return Err(CemiError::TooShort {
            needed: extra_start + extra_len,
            got: buf.len(),
        });
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
    Ok(LDataFrame {
        kind,
        source,
        destination,
        service,
    })
}

fn group_value(length: usize, inline6: u8, extra: &[u8]) -> GroupValue {
    if length <= 1 {
        GroupValue::Short(inline6)
    } else {
        GroupValue::Bytes(extra.to_vec())
    }
}

/// Encodes an `L_Data.req`/`.con`/`.ind` cEMI frame — the inverse of
/// `decode_l_data`, same fixed layout (EMI_IMI v01.04.02 AS §4.1.5.3.2),
/// never emitting additional information (`add_info_len = 0`; this cycle
/// never needs any). Ctrl1 0xBC (standard frame, no repeat, domain
/// broadcast, low priority, no ack request) and Ctrl2's hop-count-6 are
/// this crate's only outbound defaults — the same values already implied
/// by every hand-built fixture `decode_l_data` is tested against above.
pub fn encode_l_data(frame: &LDataFrame) -> Vec<u8> {
    let message_code = match frame.kind {
        LDataMessageKind::Request => L_DATA_REQ,
        LDataMessageKind::Indication => L_DATA_IND,
        LDataMessageKind::Confirmation { .. } => L_DATA_CON,
    };
    let ctrl1 = match frame.kind {
        LDataMessageKind::Confirmation { error: true } => 0xBD,
        _ => 0xBC,
    };
    let (address_type_bit, dest_raw) = match frame.destination {
        Destination::Group(addr) => (0x80, addr.raw()),
        Destination::Individual(addr) => (0x00, addr.raw()),
    };
    let ctrl2 = address_type_bit | 0x60; // hop count 6, standard EFF (0000)
    let source_raw = frame.source.raw();

    let (short_apci, length, inline6, extra): (u8, usize, u8, &[u8]) = match &frame.service {
        ApplicationService::GroupValueRead => (0b0000, 1, 0, &[]),
        ApplicationService::GroupValueResponse(v) => {
            let (length, inline6, extra) = encode_group_value(v);
            (0b0001, length, inline6, extra)
        }
        ApplicationService::GroupValueWrite(v) => {
            let (length, inline6, extra) = encode_group_value(v);
            (0b0010, length, inline6, extra)
        }
        ApplicationService::Other { apci, data } => {
            // Round-trips exactly what `decode_l_data` reconstructs
            // `apci` from: its own encoding below stores the same two
            // octets in the same places, only ever reached when the
            // 4-bit `short_apci` scheme above doesn't apply.
            let tpci_apci_hi = (apci >> 8) as u8;
            let apci_lo = *apci as u8;
            return finish_l_data(
                message_code,
                ctrl1,
                ctrl2,
                source_raw,
                dest_raw,
                1 + data.len(),
                tpci_apci_hi,
                apci_lo,
                data,
            );
        }
    };
    let tpci_apci_hi = (short_apci >> 2) & 0x03;
    let apci_lo = ((short_apci & 0x03) << 6) | inline6;
    finish_l_data(
        message_code,
        ctrl1,
        ctrl2,
        source_raw,
        dest_raw,
        length,
        tpci_apci_hi,
        apci_lo,
        extra,
    )
}

fn encode_group_value(value: &GroupValue) -> (usize, u8, &[u8]) {
    match value {
        // A `Short` whose value needs more than six bits does not fit the
        // inline APCI-octet field: `short_apci`'s own two low bits share
        // that octet with the top two bits of `inline6` (see the encoding
        // below), so a `Short(v)` with `v > 0x3F` would overwrite them and
        // change which `A_GroupValue_*` service the frame decodes as.
        // Application Layer v02.01.01 AS §3.1.3 already draws the line at
        // six bits ("Values that only consist of 6 bits or less have the
        // following optimized A_GroupValue_Write-PDU format"), so an
        // out-of-range `Short` is promoted to the one-octet `Bytes` form
        // instead — the value survives, and the APCI is not touched.
        GroupValue::Short(v) if *v > 0x3F => (2, 0, std::slice::from_ref(v)),
        GroupValue::Short(inline6) => (1, *inline6, &[]),
        GroupValue::Bytes(bytes) => (1 + bytes.len(), 0, bytes),
    }
}

#[allow(clippy::too_many_arguments)]
fn finish_l_data(
    message_code: u8,
    ctrl1: u8,
    ctrl2: u8,
    source_raw: u16,
    dest_raw: u16,
    length: usize,
    tpci_apci_hi: u8,
    apci_lo: u8,
    extra: &[u8],
) -> Vec<u8> {
    let mut buf = Vec::with_capacity(11 + extra.len());
    buf.push(message_code);
    buf.push(0x00); // additional info length
    buf.push(ctrl1);
    buf.push(ctrl2);
    buf.extend_from_slice(&source_raw.to_be_bytes());
    buf.extend_from_slice(&dest_raw.to_be_bytes());
    buf.push(length as u8);
    buf.push(tpci_apci_hi);
    buf.push(apci_lo);
    buf.extend_from_slice(extra);
    buf
}

#[cfg(test)]
mod tests {
    use super::*;
    use knx_core::{GroupAddress, GroupValue, IndividualAddress};

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
        assert_eq!(
            frame.destination,
            Destination::Group(GroupAddress::from_raw(0x0903))
        );
        assert_eq!(
            frame.service,
            ApplicationService::GroupValueWrite(GroupValue::Short(0x01))
        );
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
        assert_eq!(
            frame.destination,
            Destination::Individual(IndividualAddress::from_raw(0x0903))
        );
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

    /// `L_Data.req` (message code 11h) form of `write_on_frame` — same
    /// Ctrl1/Ctrl2/addresses/TPCI-APCI, sent rather than received. Ctrl1
    /// 0xBC and Ctrl2's hop-count-6/AT-bit form here match the incoming
    /// fixture above; RESEARCH.md notes no reason cEMI would ask for a
    /// different default outbound.
    fn write_on_request() -> Vec<u8> {
        let mut bytes = write_on_frame();
        bytes[0] = L_DATA_REQ;
        bytes
    }

    #[test]
    fn encode_l_data_matches_hand_built_group_value_write_request() {
        let frame = LDataFrame {
            kind: LDataMessageKind::Request,
            source: IndividualAddress::from_raw(0x1101),
            destination: Destination::Group(GroupAddress::from_raw(0x0903)),
            service: ApplicationService::GroupValueWrite(GroupValue::Short(0x01)),
        };
        assert_eq!(encode_l_data(&frame), write_on_request());
    }

    #[test]
    fn encode_l_data_round_trips_through_decode() {
        let frame = LDataFrame {
            kind: LDataMessageKind::Request,
            source: IndividualAddress::from_raw(0x1101),
            destination: Destination::Group(GroupAddress::from_raw(0x0903)),
            service: ApplicationService::GroupValueWrite(GroupValue::Bytes(vec![0x2A, 0x99])),
        };
        let encoded = encode_l_data(&frame);
        assert_eq!(decode_l_data(&encoded).unwrap(), frame);
    }

    #[test]
    fn encode_l_data_round_trips_group_value_read() {
        let frame = LDataFrame {
            kind: LDataMessageKind::Request,
            source: IndividualAddress::from_raw(0x0000),
            destination: Destination::Group(GroupAddress::from_raw(0x0903)),
            service: ApplicationService::GroupValueRead,
        };
        let encoded = encode_l_data(&frame);
        assert_eq!(decode_l_data(&encoded).unwrap(), frame);
    }

    #[test]
    fn encode_l_data_round_trips_individual_destination() {
        let frame = LDataFrame {
            kind: LDataMessageKind::Request,
            source: IndividualAddress::from_raw(0x1101),
            destination: Destination::Individual(IndividualAddress::from_raw(0x1102)),
            service: ApplicationService::GroupValueWrite(GroupValue::Short(0x00)),
        };
        let encoded = encode_l_data(&frame);
        assert_eq!(decode_l_data(&encoded).unwrap(), frame);
    }

    #[test]
    fn encode_l_data_round_trips_other_apci() {
        let frame = LDataFrame {
            kind: LDataMessageKind::Request,
            source: IndividualAddress::from_raw(0x1101),
            destination: Destination::Group(GroupAddress::from_raw(0x0903)),
            service: ApplicationService::Other {
                apci: 0x03C0,
                data: vec![0xAB, 0xCD],
            },
        };
        let encoded = encode_l_data(&frame);
        assert_eq!(decode_l_data(&encoded).unwrap(), frame);
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

    /// Regression for the APCI-corruption bug (spec E4-D3): before the
    /// `encode_group_value` fix, `Short(0x40)` set `apci_lo`'s top two bits
    /// (meant for `short_apci`) from `inline6`'s own top bit, so a
    /// `GroupValueWrite` decoded back as `short_apci = 0b0011` —
    /// `ApplicationService::Other`, with the value gone. `0x40` is the
    /// smallest value that does not fit the six inline bits (`0x3F` is the
    /// largest that does), so it is the minimal case that exercises the
    /// bug.
    #[test]
    fn encode_promotes_out_of_range_short_instead_of_corrupting_apci() {
        let frame = LDataFrame {
            kind: LDataMessageKind::Request,
            source: IndividualAddress::from_raw(0x1101),
            destination: Destination::Group(GroupAddress::from_raw(0x0903)),
            service: ApplicationService::GroupValueWrite(GroupValue::Short(0x40)),
        };
        let encoded = encode_l_data(&frame);
        let decoded = decode_l_data(&encoded).unwrap();
        assert_eq!(
            decoded.service,
            ApplicationService::GroupValueWrite(GroupValue::Bytes(vec![0x40]))
        );
    }
}
