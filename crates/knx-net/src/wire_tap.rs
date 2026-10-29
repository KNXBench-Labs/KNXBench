//! Non-blocking tap handing every tunnel datagram and connection event to a recorder.
//!
//! DIAG-03 (operation recorder): a tunnel built with a [`WireTap`] reports each
//! KNXnet/IP datagram it sends or receives, byte for byte, plus the connection
//! events that no datagram shows on its own (the cause that closed the tunnel,
//! a request that went unacknowledged, a frame discarded for its sequence).
//!
//! Two rules make the tap safe to leave on for every device operation:
//!
//! - **It never changes the exchange.** [`WireSink::record`] must return at
//!   once and must not block, await or fail into the caller; a sink that cannot
//!   keep up drops records and accounts for the gap itself. The tunnel calls it
//!   after the bytes are already on (or off) the socket.
//! - **Secrets never leave this crate.** A datagram carrying an
//!   `A_Authorize_Request` or `A_Key_Write` key is redacted *before* the sink
//!   sees it: the key octets are replaced by zero and a [`Redaction`] names
//!   the kind and length. No later layer has to remember to redact.

use std::sync::Arc;
use std::time::{Instant, SystemTime};

use crate::cemi::{APCI_AUTHORIZE_REQUEST, APCI_KEY_WRITE};
use crate::client::TunnelClosure;
use crate::tunnelling;

/// Which way a datagram crossed the socket, seen from this client.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WireDirection {
    Sent,
    Received,
}

impl WireDirection {
    pub fn as_str(self) -> &'static str {
        match self {
            WireDirection::Sent => "sent",
            WireDirection::Received => "received",
        }
    }
}

/// Which secret a redaction removed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RedactionKind {
    /// The 4-octet key of an `A_Authorize_Request`.
    AuthorizeKey,
    /// The 4-octet key of an `A_Key_Write`.
    KeyWriteKey,
}

impl RedactionKind {
    pub fn as_str(self) -> &'static str {
        match self {
            RedactionKind::AuthorizeKey => "authorize-key",
            RedactionKind::KeyWriteKey => "key-write-key",
        }
    }
}

/// Where in a recorded datagram the secret was, and how long it was.
///
/// The octets at `offset..offset + octets` of the recorded bytes are zero;
/// they are a placeholder, never the value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Redaction {
    pub kind: RedactionKind,
    pub offset: usize,
    pub octets: usize,
}

/// A connection event that no single datagram expresses.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TunnelTapEvent {
    /// `CONNECT_REQUEST` sent to this gateway endpoint.
    Connecting { gateway: std::net::SocketAddrV4 },
    /// The gateway accepted: channel and assigned individual address.
    Connected {
        channel_id: u8,
        assigned_address: knx_core::IndividualAddress,
    },
    /// The connect handshake failed before a channel existed.
    ConnectFailed { reason: String },
    /// A `TUNNELLING_REQUEST` attempt went out; `attempt` starts at 1.
    RequestAttempt { sequence: u8, attempt: u8 },
    /// No matching `TUNNELLING_ACK` arrived within the attempt's wait.
    AckTimeout { sequence: u8, attempt: u8 },
    /// A received `TUNNELLING_REQUEST` was not forwarded, and why.
    ReceiveDiscarded { sequence: u8, reason: &'static str },
    /// A received cEMI frame failed to decode; the datagram is recorded.
    CemiDecodeFailed { reason: String },
    /// A heartbeat attempt (`CONNECTIONSTATE_REQUEST`); `attempt` starts at 1.
    HeartbeatAttempt { attempt: u8 },
    /// The first cause that ended the tunnel; recorded once.
    Closed(TunnelClosure),
    /// A socket operation failed.
    SocketError {
        operation: &'static str,
        reason: String,
    },
}

/// One thing the tap saw, stamped with both clocks at the moment it happened.
#[derive(Debug, Clone)]
pub enum WireRecord {
    Datagram {
        at: Instant,
        wall: SystemTime,
        direction: WireDirection,
        /// The datagram exactly as on the socket, except redacted octets.
        bytes: Vec<u8>,
        redaction: Option<Redaction>,
    },
    Event {
        at: Instant,
        wall: SystemTime,
        event: TunnelTapEvent,
    },
}

/// Receives tap records. Implementations must return immediately: no
/// blocking I/O, no waiting for a lock another thread may hold for long.
pub trait WireSink: Send + Sync {
    fn record(&self, record: WireRecord);
}

/// A cheap, cloneable handle to a [`WireSink`].
#[derive(Clone)]
pub struct WireTap(Arc<dyn WireSink>);

impl std::fmt::Debug for WireTap {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("WireTap")
    }
}

impl WireTap {
    pub fn new(sink: Arc<dyn WireSink>) -> Self {
        Self(sink)
    }

    /// Records one datagram, redacting any key it carries first.
    pub fn datagram(&self, direction: WireDirection, bytes: &[u8]) {
        let (bytes, redaction) = redact_datagram(bytes);
        self.0.record(WireRecord::Datagram {
            at: Instant::now(),
            wall: SystemTime::now(),
            direction,
            bytes,
            redaction,
        });
    }

    /// Records one connection event.
    pub fn event(&self, event: TunnelTapEvent) {
        self.0.record(WireRecord::Event {
            at: Instant::now(),
            wall: SystemTime::now(),
            event,
        });
    }
}

/// KNXnet/IP `ROUTING_INDICATION`; carries a cEMI frame right after the header.
const ROUTING_INDICATION: u16 = 0x0530;

/// Returns `bytes` with any authorize/key-write key zeroed, and where it was.
///
/// Recognises the cEMI inside a `TUNNELLING_REQUEST` (behind its connection
/// header) and a `ROUTING_INDICATION`. Any other datagram carries no cEMI and
/// is returned unchanged.
pub fn redact_datagram(bytes: &[u8]) -> (Vec<u8>, Option<Redaction>) {
    let mut out = bytes.to_vec();
    let Some(cemi_start) = cemi_offset(bytes) else {
        return (out, None);
    };
    let Some((kind, key_start)) = secret_in_cemi(&bytes[cemi_start..]) else {
        return (out, None);
    };
    let offset = cemi_start + key_start;
    let end = (offset + KEY_OCTETS).min(out.len());
    out[offset..end].fill(0);
    (
        out,
        Some(Redaction {
            kind,
            offset,
            octets: end - offset,
        }),
    )
}

/// Length of an `A_Authorize_Request`/`A_Key_Write` key (AL §3.5.7/§3.5.8).
const KEY_OCTETS: usize = 4;

fn cemi_offset(datagram: &[u8]) -> Option<usize> {
    if datagram.len() < 6 {
        return None;
    }
    let header_len = datagram[0] as usize;
    let service = u16::from_be_bytes([datagram[2], datagram[3]]);
    match service {
        tunnelling::TUNNELLING_REQUEST => {
            // The connection header states its own length (normally 4).
            let connection_header = *datagram.get(header_len)? as usize;
            Some(header_len + connection_header).filter(|start| *start < datagram.len())
        }
        ROUTING_INDICATION => Some(header_len).filter(|start| *start < datagram.len()),
        _ => None,
    }
}

/// Finds a secret-bearing APDU in an `L_Data` cEMI frame: its kind and the
/// offset of the key within `cemi`. Works on the raw octets so a frame the
/// codec would reject is still checked.
fn secret_in_cemi(cemi: &[u8]) -> Option<(RedactionKind, usize)> {
    let add_info_len = *cemi.get(1)? as usize;
    // Message code, add-info length, add-info, Ctrl1, Ctrl2, source (2),
    // destination (2), L, then the TPCI/APCI octets.
    let tpci_start = 2 + add_info_len + 7;
    let first = *cemi.get(tpci_start)?;
    let second = *cemi.get(tpci_start + 1)?;
    let apci = (u16::from(first & 0x03) << 8) | u16::from(second);
    let kind = match apci {
        APCI_AUTHORIZE_REQUEST => RedactionKind::AuthorizeKey,
        APCI_KEY_WRITE => RedactionKind::KeyWriteKey,
        _ => return None,
    };
    // One octet (reserved for authorize, level for key write) precedes the key.
    let key_start = tpci_start + 3;
    (key_start < cemi.len()).then_some((kind, key_start))
}

/// What a recorded datagram says, decoded for a human reader of a recording.
///
/// Only fields this crate can decode with certainty are filled; anything
/// else stays `None` and the raw bytes remain the authority.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DatagramDescription {
    pub service_type: Option<u16>,
    pub service: Option<&'static str>,
    pub channel_id: Option<u8>,
    /// Tunnelling sequence counter (`TUNNELLING_REQUEST`/`_ACK`).
    pub sequence: Option<u8>,
    /// Status octet of an ACK or response.
    pub status: Option<u8>,
    pub cemi: Option<CemiDescription>,
    /// Why the datagram or its cEMI could not be decoded.
    pub decode_error: Option<String>,
}

/// The decoded `L_Data` frame inside a datagram.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CemiDescription {
    pub message: &'static str,
    /// `Some(true)` for an `L_Data.con` reporting a failed transmission.
    pub confirm_error: Option<bool>,
    pub source: String,
    pub destination: String,
    pub tpci: String,
    pub apci: &'static str,
    /// Payload summary; keys are rendered as `<redacted>` by the codec.
    pub summary: Option<String>,
}

/// Decodes a recorded datagram as far as it safely goes.
pub fn describe_datagram(bytes: &[u8]) -> DatagramDescription {
    use crate::core::services;
    let mut out = DatagramDescription::default();
    let (header, body) = match crate::frame::decode_frame(bytes) {
        Ok(decoded) => decoded,
        Err(error) => {
            out.decode_error = Some(error.to_string());
            return out;
        }
    };
    out.service_type = Some(header.service_type);
    out.service = service_name(header.service_type);
    match header.service_type {
        tunnelling::TUNNELLING_REQUEST => match tunnelling::decode_tunnelling_request(body) {
            Ok(request) => {
                out.channel_id = Some(request.channel_id);
                out.sequence = Some(request.sequence_counter);
                describe_cemi(request.cemi, &mut out);
            }
            Err(error) => out.decode_error = Some(error.to_string()),
        },
        tunnelling::TUNNELLING_ACK => match tunnelling::decode_tunnelling_ack(body) {
            Ok(ack) => {
                out.channel_id = Some(ack.channel_id);
                out.sequence = Some(ack.sequence_counter);
                out.status = Some(ack.status);
            }
            Err(error) => out.decode_error = Some(error.to_string()),
        },
        services::CONNECT_RESPONSE
        | services::CONNECTIONSTATE_RESPONSE
        | services::DISCONNECT_RESPONSE => {
            out.channel_id = body.first().copied();
            out.status = body.get(1).copied();
        }
        services::CONNECTIONSTATE_REQUEST | services::DISCONNECT_REQUEST => {
            out.channel_id = body.first().copied();
        }
        ROUTING_INDICATION => describe_cemi(body, &mut out),
        _ => {}
    }
    out
}

fn describe_cemi(cemi: &[u8], out: &mut DatagramDescription) {
    use crate::cemi::{Destination, LDataMessageKind};
    match crate::cemi::decode_l_data(cemi) {
        Ok(frame) => {
            let (message, confirm_error) = match frame.kind {
                LDataMessageKind::Request => ("L_Data.req", None),
                LDataMessageKind::Indication => ("L_Data.ind", None),
                LDataMessageKind::Confirmation { error } => ("L_Data.con", Some(error)),
            };
            let destination = match frame.destination {
                Destination::Individual(address) => address.to_string(),
                Destination::Group(group) => format!("group {:#06x}", group.raw()),
                Destination::SystemBroadcast => "system broadcast".to_string(),
            };
            out.cemi = Some(CemiDescription {
                message,
                confirm_error,
                source: frame.source.to_string(),
                destination,
                tpci: format!("{:?}", frame.transport),
                apci: frame.service.variant_name(),
                summary: frame.service.payload_summary(),
            });
        }
        Err(error) => out.decode_error = Some(error.to_string()),
    }
}

fn service_name(service_type: u16) -> Option<&'static str> {
    use crate::core::services;
    Some(match service_type {
        services::SEARCH_REQUEST => "SEARCH_REQUEST",
        services::SEARCH_RESPONSE => "SEARCH_RESPONSE",
        services::CONNECT_REQUEST => "CONNECT_REQUEST",
        services::CONNECT_RESPONSE => "CONNECT_RESPONSE",
        services::CONNECTIONSTATE_REQUEST => "CONNECTIONSTATE_REQUEST",
        services::CONNECTIONSTATE_RESPONSE => "CONNECTIONSTATE_RESPONSE",
        services::DISCONNECT_REQUEST => "DISCONNECT_REQUEST",
        services::DISCONNECT_RESPONSE => "DISCONNECT_RESPONSE",
        tunnelling::TUNNELLING_REQUEST => "TUNNELLING_REQUEST",
        tunnelling::TUNNELLING_ACK => "TUNNELLING_ACK",
        services::ROUTING_INDICATION => "ROUTING_INDICATION",
        services::ROUTING_LOST_MESSAGE => "ROUTING_LOST_MESSAGE",
        services::ROUTING_BUSY => "ROUTING_BUSY",
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_redacted_authorize_request_describes_without_its_key() {
        let (bytes, _) = redact_datagram(&authorize_datagram(SENTINEL));
        let description = describe_datagram(&bytes);
        assert_eq!(description.service, Some("TUNNELLING_REQUEST"));
        assert_eq!(description.channel_id, Some(7));
        assert_eq!(description.sequence, Some(3));
        let cemi = description.cemi.expect("cEMI decoded");
        assert_eq!(cemi.message, "L_Data.req");
        assert_eq!(cemi.destination, "1.1.5");
        assert_eq!(cemi.apci, "AuthorizeRequest");
        assert_eq!(cemi.summary.as_deref(), Some("key=<redacted>"));
    }

    #[test]
    fn acks_and_garbage_describe_what_is_certain() {
        let ack = frame::encode_frame(
            tunnelling::TUNNELLING_ACK,
            &tunnelling::encode_tunnelling_ack(9, 4, 0x29),
        );
        let description = describe_datagram(&ack);
        assert_eq!(description.service, Some("TUNNELLING_ACK"));
        assert_eq!(
            (
                description.channel_id,
                description.sequence,
                description.status
            ),
            (Some(9), Some(4), Some(0x29))
        );
        let garbage = describe_datagram(&[1, 2, 3]);
        assert!(garbage.decode_error.is_some());
        assert_eq!(garbage.service, None);
    }
    use crate::cemi::{self, ApplicationService, Destination, LDataFrame, LDataMessageKind, Tpci};
    use crate::frame;
    use knx_core::IndividualAddress;
    use std::sync::Mutex;

    fn authorize_datagram(key: [u8; 4]) -> Vec<u8> {
        let frame = LDataFrame {
            kind: LDataMessageKind::Request,
            source: IndividualAddress::from_raw(0),
            destination: Destination::Individual(IndividualAddress::from_raw(0x1105)),
            transport: Tpci::NumberedData { seq: 2 },
            service: ApplicationService::AuthorizeRequest { key },
            control: None,
        };
        let cemi = cemi::encode_l_data(&frame).unwrap();
        frame::encode_frame(
            tunnelling::TUNNELLING_REQUEST,
            &tunnelling::encode_tunnelling_request(7, 3, &cemi),
        )
    }

    fn key_write_datagram(key: [u8; 4]) -> Vec<u8> {
        // No encoder variant exists for A_Key_Write on purpose; build the
        // APDU by hand: TPCI data-connected seq 0 + APCI 0x3D3, level, key.
        let mut cemi = vec![0x11, 0x00, 0xB0, 0x60, 0x00, 0x00, 0x11, 0x05, 0x06];
        let apci = APCI_KEY_WRITE;
        cemi.push(0x40 | ((apci >> 8) as u8 & 0x03));
        cemi.push(apci as u8);
        cemi.push(0x01); // level
        cemi.extend_from_slice(&key);
        frame::encode_frame(
            tunnelling::TUNNELLING_REQUEST,
            &tunnelling::encode_tunnelling_request(7, 4, &cemi),
        )
    }

    const SENTINEL: [u8; 4] = [0xC0, 0xFF, 0xEE, 0x42];

    fn contains(haystack: &[u8], needle: &[u8]) -> bool {
        haystack.windows(needle.len()).any(|w| w == needle)
    }

    #[test]
    fn an_authorize_key_is_zeroed_and_named() {
        let original = authorize_datagram(SENTINEL);
        assert!(contains(&original, &SENTINEL));
        let (redacted, redaction) = redact_datagram(&original);
        let redaction = redaction.expect("authorize request is redacted");
        assert_eq!(redaction.kind, RedactionKind::AuthorizeKey);
        assert_eq!(redaction.octets, 4);
        assert!(!contains(&redacted, &SENTINEL));
        assert_eq!(&redacted[redaction.offset..redaction.offset + 4], &[0; 4]);
        // Everything outside the key is byte-identical.
        assert_eq!(redacted.len(), original.len());
        for (index, (a, b)) in original.iter().zip(&redacted).enumerate() {
            if !(redaction.offset..redaction.offset + 4).contains(&index) {
                assert_eq!(a, b, "octet {index} changed");
            }
        }
    }

    #[test]
    fn a_key_write_key_is_zeroed_and_named() {
        let original = key_write_datagram(SENTINEL);
        let (redacted, redaction) = redact_datagram(&original);
        let redaction = redaction.expect("key write is redacted");
        assert_eq!(redaction.kind, RedactionKind::KeyWriteKey);
        assert_eq!(redaction.octets, 4);
        assert!(!contains(&redacted, &SENTINEL));
    }

    #[test]
    fn ordinary_datagrams_pass_unchanged() {
        let frame = LDataFrame {
            kind: LDataMessageKind::Request,
            source: IndividualAddress::from_raw(0),
            destination: Destination::Individual(IndividualAddress::from_raw(0x1105)),
            transport: Tpci::NumberedData { seq: 0 },
            service: ApplicationService::MemoryWrite {
                address: 0x0116,
                data: SENTINEL.to_vec(),
            },
            control: None,
        };
        let cemi = cemi::encode_l_data(&frame).unwrap();
        let datagram = frame::encode_frame(
            tunnelling::TUNNELLING_REQUEST,
            &tunnelling::encode_tunnelling_request(1, 0, &cemi),
        );
        let (out, redaction) = redact_datagram(&datagram);
        assert_eq!(out, datagram);
        assert_eq!(redaction, None);
        // Not a cEMI carrier at all, and too short to be anything.
        assert_eq!(redact_datagram(&[0x06, 0x10]).1, None);
        let ack = frame::encode_frame(
            tunnelling::TUNNELLING_ACK,
            &tunnelling::encode_tunnelling_ack(1, 0, 0),
        );
        assert_eq!(redact_datagram(&ack), (ack.clone(), None));
    }

    #[test]
    fn a_truncated_key_is_still_redacted_as_far_as_it_goes() {
        let mut datagram = authorize_datagram(SENTINEL);
        datagram.truncate(datagram.len() - 2);
        let (out, redaction) = redact_datagram(&datagram);
        let redaction = redaction.expect("partial key still redacted");
        assert_eq!(redaction.octets, 2);
        assert!(!contains(&out, &SENTINEL[..2]));
    }

    struct Collect(Mutex<Vec<WireRecord>>);
    impl WireSink for Collect {
        fn record(&self, record: WireRecord) {
            self.0.lock().unwrap().push(record);
        }
    }

    #[test]
    fn the_tap_redacts_before_the_sink_sees_anything() {
        let sink = Arc::new(Collect(Mutex::new(Vec::new())));
        let tap = WireTap::new(sink.clone());
        tap.datagram(WireDirection::Sent, &authorize_datagram(SENTINEL));
        tap.datagram(WireDirection::Received, &key_write_datagram(SENTINEL));
        let records = sink.0.lock().unwrap();
        assert_eq!(records.len(), 2);
        for record in records.iter() {
            let WireRecord::Datagram {
                bytes, redaction, ..
            } = record
            else {
                panic!("datagram expected");
            };
            assert!(!contains(bytes, &SENTINEL));
            assert!(redaction.is_some());
        }
    }
}
