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
                write!(
                    f,
                    "unexpected header length {len:#04x}, expected {HEADER_SIZE_10:#04x}"
                )
            }
            FrameError::UnsupportedVersion(v) => {
                write!(f, "unsupported KNXnet/IP protocol version {v:#04x}")
            }
            FrameError::LengthMismatch {
                header_said,
                actual,
            } => {
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
        return Err(FrameError::TooShort {
            needed: 6,
            got: buf.len(),
        });
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
    Ok((
        FrameHeader {
            service_type,
            total_length,
        },
        &buf[6..],
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_then_decode_round_trips() {
        let body = [0xAAu8, 0xBB, 0xCC];
        let datagram = encode_frame(0x0203, &body);
        assert_eq!(
            datagram,
            vec![0x06, 0x10, 0x02, 0x03, 0x00, 0x09, 0xAA, 0xBB, 0xCC]
        );

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
            FrameError::LengthMismatch {
                header_said: 9,
                actual: 6
            }
        );
    }
}
