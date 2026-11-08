//! Incremental decoder that cuts a KNXnet/IP TCP byte stream into whole frames in bounded memory.
//!
//! Core v01.06.02 AS §8.4.3.3 (printed p. 45, Note 8 p. 46): over TCP the
//! KNXnet/IP frames follow one another with no prefix, separator or padding,
//! and a read boundary says nothing about a frame boundary. This decoder
//! therefore collects the six header octets first, then exactly
//! `total length - 6` body octets, then the next header. It never searches a
//! body for something that looks like a header, and it never resynchronises
//! on read boundaries.
//!
//! What it does with each header, and where that rule comes from:
//!
//! - Header length other than 06h, or a total length below the header's own
//!   six octets (§2.3, §7.1–7.2: the total length includes the header), is
//!   a header that is not well formed. §8.4.3.3 requires the TCP connection
//!   to be closed, so the decoder reports [`StreamError`] and stays failed.
//! - A protocol version other than 10h is reported as
//!   [`StreamError::UnsupportedVersion`]. Closing the stream for it is a
//!   **local policy**: Core §6.2–6.3 asks for a service-specific negative
//!   reply (E_VERSION_NOT_SUPPORTED) that the inspected clauses do not define
//!   for every service over TCP, so no reply is fabricated and no full
//!   conformance is claimed for this case.
//! - A well-formed frame whose service the caller does not handle, or whose
//!   total length exceeds the local cap, is skipped octet by octet without
//!   being stored (§6.2–6.3, §8.4.3.3), up to the protocol maximum of 65535.
//!   The cap is a local buffer size, not a wire limit: skipping is not an
//!   error and the stream continues with the next header.

use crate::frame::{HEADER_SIZE_10, KNXNETIP_VERSION_10};

/// Octets of the KNXnet/IP 1.0 header.
const HEADER: usize = HEADER_SIZE_10 as usize;

/// One unit the decoder produced from the stream.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StreamItem {
    /// A complete frame, header included, exactly as it was on the stream.
    Frame(Vec<u8>),
    /// A well-formed frame that was read past without being stored.
    Skipped {
        service_type: u16,
        total_length: u16,
        reason: SkipReason,
    },
}

/// Why a well-formed frame was skipped instead of stored.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkipReason {
    /// The caller does not handle this service type.
    UnhandledService,
    /// The frame is larger than the local receive cap.
    AboveLocalCap,
}

impl SkipReason {
    /// Stable lower-case name for diagnostics and recordings.
    pub const fn as_str(self) -> &'static str {
        match self {
            SkipReason::UnhandledService => "unhandled-service",
            SkipReason::AboveLocalCap => "above-local-cap",
        }
    }
}

/// A header the stream cannot continue after.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StreamError {
    /// The first header octet is not 06h.
    BadHeaderLength(u8),
    /// The version octet is not 10h (local fail-closed policy, see module docs).
    UnsupportedVersion(u8),
    /// The total length is below the six header octets it includes.
    TotalLengthBelowHeader(u16),
}

impl std::fmt::Display for StreamError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StreamError::BadHeaderLength(length) => write!(
                f,
                "malformed KNXnet/IP header on the stream: header length {length:#04x}, \
                 expected {HEADER_SIZE_10:#04x}"
            ),
            StreamError::UnsupportedVersion(version) => write!(
                f,
                "unsupported KNXnet/IP protocol version {version:#04x} on the stream"
            ),
            StreamError::TotalLengthBelowHeader(total) => write!(
                f,
                "malformed KNXnet/IP header on the stream: total length {total} is below \
                 the {HEADER} header octets it includes"
            ),
        }
    }
}

impl std::error::Error for StreamError {}

/// Where the decoder is inside the current frame.
#[derive(Debug)]
enum Position {
    /// Collecting header octets; `filled` of them are in `header`.
    Header { filled: usize },
    /// Storing a frame that fits the cap; `frame` holds the octets so far.
    Keep { frame: Vec<u8>, total: usize },
    /// Reading past a frame; `remaining` body octets are still to come.
    Skip {
        service_type: u16,
        total_length: u16,
        reason: SkipReason,
        remaining: usize,
    },
}

/// Cuts a byte stream into frames. Feed it every octet read, in order.
///
/// All progress lives in the decoder, so a caller may stop reading at any
/// point and continue later on the same stream without losing a partial
/// header or body. Memory held is at most one frame no larger than the cap.
#[derive(Debug)]
pub struct StreamDecoder {
    cap: usize,
    handles: fn(u16) -> bool,
    header: [u8; HEADER],
    position: Position,
    failed: Option<StreamError>,
}

impl StreamDecoder {
    /// A decoder that stores frames of at most `cap` octets (never less than
    /// a bare header) whose service type `handles` accepts.
    pub fn new(cap: usize, handles: fn(u16) -> bool) -> Self {
        Self {
            cap: cap.max(HEADER),
            handles,
            header: [0; HEADER],
            position: Position::Header { filled: 0 },
            failed: None,
        }
    }

    /// `true` while a header or body has started but not finished.
    pub fn is_mid_frame(&self) -> bool {
        !matches!(self.position, Position::Header { filled: 0 })
    }

    /// Consumes `input`, appending every unit it completes to `out`.
    ///
    /// Units completed before a malformed header are still appended. After
    /// an error the decoder stays failed and returns the same error again.
    pub fn push(&mut self, mut input: &[u8], out: &mut Vec<StreamItem>) -> Result<(), StreamError> {
        if let Some(error) = self.failed {
            return Err(error);
        }
        while !input.is_empty() {
            match &mut self.position {
                Position::Header { filled } => {
                    let take = (HEADER - *filled).min(input.len());
                    self.header[*filled..*filled + take].copy_from_slice(&input[..take]);
                    *filled += take;
                    input = &input[take..];
                    let filled = *filled;
                    if let Err(error) = self.check_header(filled) {
                        self.failed = Some(error);
                        return Err(error);
                    }
                    if filled == HEADER {
                        self.begin_frame(out);
                    }
                }
                Position::Keep { frame, total } => {
                    let take = (*total - frame.len()).min(input.len());
                    frame.extend_from_slice(&input[..take]);
                    input = &input[take..];
                    if frame.len() == *total {
                        let frame = std::mem::take(frame);
                        out.push(StreamItem::Frame(frame));
                        self.position = Position::Header { filled: 0 };
                    }
                }
                Position::Skip {
                    service_type,
                    total_length,
                    reason,
                    remaining,
                } => {
                    let take = (*remaining).min(input.len());
                    *remaining -= take;
                    input = &input[take..];
                    if *remaining == 0 {
                        out.push(StreamItem::Skipped {
                            service_type: *service_type,
                            total_length: *total_length,
                            reason: *reason,
                        });
                        self.position = Position::Header { filled: 0 };
                    }
                }
            }
        }
        Ok(())
    }

    /// Checks the header octets received so far, as early as each is known.
    fn check_header(&self, filled: usize) -> Result<(), StreamError> {
        if filled >= 1 && self.header[0] != HEADER_SIZE_10 {
            return Err(StreamError::BadHeaderLength(self.header[0]));
        }
        if filled >= 2 && self.header[1] != KNXNETIP_VERSION_10 {
            return Err(StreamError::UnsupportedVersion(self.header[1]));
        }
        if filled == HEADER {
            let total = u16::from_be_bytes([self.header[4], self.header[5]]);
            if usize::from(total) < HEADER {
                return Err(StreamError::TotalLengthBelowHeader(total));
            }
        }
        Ok(())
    }

    /// Starts the body of the frame whose complete, valid header is stored.
    fn begin_frame(&mut self, out: &mut Vec<StreamItem>) {
        let service_type = u16::from_be_bytes([self.header[2], self.header[3]]);
        let total_length = u16::from_be_bytes([self.header[4], self.header[5]]);
        let total = usize::from(total_length);
        let skip = if !(self.handles)(service_type) {
            Some(SkipReason::UnhandledService)
        } else if total > self.cap {
            Some(SkipReason::AboveLocalCap)
        } else {
            None
        };
        let remaining = total - HEADER;
        self.position = match skip {
            Some(reason) if remaining == 0 => {
                out.push(StreamItem::Skipped {
                    service_type,
                    total_length,
                    reason,
                });
                Position::Header { filled: 0 }
            }
            Some(reason) => Position::Skip {
                service_type,
                total_length,
                reason,
                remaining,
            },
            None => {
                let mut frame = Vec::with_capacity(total);
                frame.extend_from_slice(&self.header);
                if remaining == 0 {
                    out.push(StreamItem::Frame(frame));
                    Position::Header { filled: 0 }
                } else {
                    Position::Keep { frame, total }
                }
            }
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frame::encode_frame;

    const HANDLED: u16 = 0x0420;
    const OTHER_HANDLED: u16 = 0x0208;

    fn handles(service: u16) -> bool {
        matches!(service, HANDLED | OTHER_HANDLED)
    }

    /// A body that contains a complete-looking KNXnet/IP header of its own.
    fn magic_body() -> Vec<u8> {
        vec![
            0x04, 0x2a, 0x00, 0x00, 0x06, 0x10, 0x04, 0x20, 0x00, 0x0a, 0x06, 0x10,
        ]
    }

    fn decode_in_chunks(stream: &[u8], chunk: usize, cap: usize) -> Vec<StreamItem> {
        let mut decoder = StreamDecoder::new(cap, handles);
        let mut out = Vec::new();
        for piece in stream.chunks(chunk) {
            decoder.push(piece, &mut out).expect("well-formed stream");
        }
        assert!(
            !decoder.is_mid_frame(),
            "the stream ended on a frame boundary"
        );
        out
    }

    #[test]
    fn every_split_of_two_coalesced_frames_yields_the_same_two_frames() {
        let first = encode_frame(HANDLED, &magic_body());
        let second = encode_frame(OTHER_HANDLED, &[0x2a, 0x00]);
        let mut stream = first.clone();
        stream.extend_from_slice(&second);
        let expected = vec![StreamItem::Frame(first), StreamItem::Frame(second)];
        for chunk in 1..=stream.len() {
            assert_eq!(
                decode_in_chunks(&stream, chunk, 1024),
                expected,
                "chunk {chunk}"
            );
        }
        // Every two-piece cut, including inside both headers and the body.
        for cut in 0..=stream.len() {
            let mut decoder = StreamDecoder::new(1024, handles);
            let mut out = Vec::new();
            decoder.push(&stream[..cut], &mut out).unwrap();
            decoder.push(&stream[cut..], &mut out).unwrap();
            assert_eq!(out, expected, "cut at {cut}");
        }
    }

    #[test]
    fn a_header_only_frame_completes_without_a_body() {
        let empty = encode_frame(HANDLED, &[]);
        assert_eq!(empty.len(), 6);
        let mut decoder = StreamDecoder::new(1024, handles);
        let mut out = Vec::new();
        decoder.push(&empty, &mut out).unwrap();
        assert_eq!(out, vec![StreamItem::Frame(empty)]);
        assert!(!decoder.is_mid_frame());
    }

    #[test]
    fn partial_progress_is_kept_between_pushes() {
        let frame = encode_frame(HANDLED, &magic_body());
        let mut decoder = StreamDecoder::new(1024, handles);
        let mut out = Vec::new();
        decoder.push(&frame[..3], &mut out).unwrap();
        assert!(decoder.is_mid_frame());
        assert!(out.is_empty());
        decoder.push(&frame[3..9], &mut out).unwrap();
        assert!(decoder.is_mid_frame());
        assert!(out.is_empty());
        decoder.push(&frame[9..], &mut out).unwrap();
        assert_eq!(out, vec![StreamItem::Frame(frame)]);
    }

    #[test]
    fn an_unhandled_service_is_skipped_by_its_length_and_the_stream_continues() {
        let unknown = encode_frame(0x0999, &magic_body());
        let valid = encode_frame(HANDLED, &[0x04, 0x2a, 0x00, 0x00]);
        let mut stream = unknown;
        stream.extend_from_slice(&valid);
        let expected = vec![
            StreamItem::Skipped {
                service_type: 0x0999,
                total_length: 18,
                reason: SkipReason::UnhandledService,
            },
            StreamItem::Frame(valid),
        ];
        for chunk in [1, 2, 5, 7, stream.len()] {
            assert_eq!(
                decode_in_chunks(&stream, chunk, 1024),
                expected,
                "chunk {chunk}"
            );
        }
    }

    #[test]
    fn the_largest_frame_the_header_can_state_is_drained_in_small_pieces() {
        // Total length 65535: the protocol maximum, far above the local cap.
        let body = vec![0x06; 65535 - 6];
        let oversize = encode_frame(HANDLED, &body);
        assert_eq!(&oversize[4..6], &[0xff, 0xff]);
        let valid = encode_frame(HANDLED, &magic_body());
        let mut stream = oversize;
        stream.extend_from_slice(&valid);
        let out = decode_in_chunks(&stream, 512, 1024);
        assert_eq!(
            out,
            vec![
                StreamItem::Skipped {
                    service_type: HANDLED,
                    total_length: 65535,
                    reason: SkipReason::AboveLocalCap,
                },
                StreamItem::Frame(valid),
            ]
        );
    }

    #[test]
    fn the_cap_is_inclusive_and_one_octet_more_is_skipped_not_refused() {
        let at_cap = encode_frame(HANDLED, &[0; 26]);
        let above = encode_frame(HANDLED, &[0; 27]);
        assert_eq!(at_cap.len(), 32);
        let mut stream = at_cap.clone();
        stream.extend_from_slice(&above);
        stream.extend_from_slice(&at_cap);
        assert_eq!(
            decode_in_chunks(&stream, 4, 32),
            vec![
                StreamItem::Frame(at_cap.clone()),
                StreamItem::Skipped {
                    service_type: HANDLED,
                    total_length: 33,
                    reason: SkipReason::AboveLocalCap,
                },
                StreamItem::Frame(at_cap),
            ]
        );
    }

    #[test]
    fn a_bad_header_length_fails_at_its_first_octet_and_stays_failed() {
        let valid = encode_frame(HANDLED, &[0x04, 0x2a, 0x00, 0x00]);
        let mut stream = valid.clone();
        stream.push(0x07);
        let mut decoder = StreamDecoder::new(1024, handles);
        let mut out = Vec::new();
        assert_eq!(
            decoder.push(&stream, &mut out),
            Err(StreamError::BadHeaderLength(0x07))
        );
        assert_eq!(
            out,
            vec![StreamItem::Frame(valid.clone())],
            "earlier frame kept"
        );
        let mut later = Vec::new();
        assert_eq!(
            decoder.push(&valid, &mut later),
            Err(StreamError::BadHeaderLength(0x07))
        );
        assert!(later.is_empty(), "nothing is decoded after a stream error");
    }

    #[test]
    fn an_unsupported_version_fails_at_its_octet() {
        let mut decoder = StreamDecoder::new(1024, handles);
        let mut out = Vec::new();
        assert_eq!(decoder.push(&[0x06], &mut out), Ok(()));
        assert_eq!(
            decoder.push(&[0x20], &mut out),
            Err(StreamError::UnsupportedVersion(0x20))
        );
        assert!(out.is_empty());
    }

    #[test]
    fn total_lengths_below_six_are_malformed() {
        for total in [0u16, 1, 5] {
            let mut header = vec![0x06, 0x10, 0x04, 0x20];
            header.extend_from_slice(&total.to_be_bytes());
            let mut decoder = StreamDecoder::new(1024, handles);
            let mut out = Vec::new();
            assert_eq!(
                decoder.push(&header, &mut out),
                Err(StreamError::TotalLengthBelowHeader(total)),
                "total {total}"
            );
            assert!(out.is_empty());
        }
        // Six is the smallest valid total: the bare header.
        let mut decoder = StreamDecoder::new(1024, handles);
        let mut out = Vec::new();
        assert_eq!(
            decoder.push(&[0x06, 0x10, 0x04, 0x20, 0x00, 0x06], &mut out),
            Ok(())
        );
        assert_eq!(out.len(), 1);
    }

    #[test]
    fn skip_reasons_have_stable_names() {
        assert_eq!(SkipReason::UnhandledService.as_str(), "unhandled-service");
        assert_eq!(SkipReason::AboveLocalCap.as_str(), "above-local-cap");
    }

    #[test]
    fn the_cap_never_drops_below_a_bare_header() {
        let empty = encode_frame(HANDLED, &[]);
        let mut decoder = StreamDecoder::new(0, handles);
        let mut out = Vec::new();
        decoder.push(&empty, &mut out).unwrap();
        assert_eq!(out, vec![StreamItem::Frame(empty)]);
    }
}
