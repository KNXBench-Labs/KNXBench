//! Turns recorder messages into the JSON lines of the recording format, version 1.

use std::time::{Instant, SystemTime};

use knx_net::wire_tap::{describe_datagram, TunnelTapEvent, WireRecord};
use serde_json::{json, Map, Value};

use super::{
    OperationStart, RecordedOutcome, WriterIdentity, RECORDING_FORMAT, RECORDING_FORMAT_VERSION,
};

/// RFC 3339 UTC with microseconds.
pub(super) fn wall(time: SystemTime) -> String {
    chrono::DateTime::<chrono::Utc>::from(time).to_rfc3339_opts(chrono::SecondsFormat::Micros, true)
}

/// Microseconds since the recording started, on the monotonic clock.
pub(super) fn since(started: Instant, at: Instant) -> u64 {
    u64::try_from(at.saturating_duration_since(started).as_micros()).unwrap_or(u64::MAX)
}

pub(super) fn hex(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push_str(&format!("{byte:02x}"));
    }
    out
}

/// What the platform says about itself; no host name, no user name.
pub(super) fn platform() -> Value {
    json!({
        "os": std::env::consts::OS,
        "family": std::env::consts::FAMILY,
        "arch": std::env::consts::ARCH,
        "container": container(),
    })
}

/// `true` inside a Docker/Podman container, `false` on Linux outside one,
/// `null` where the check means nothing.
fn container() -> Value {
    if cfg!(target_os = "linux") {
        let markers = ["/.dockerenv", "/run/.containerenv"];
        Value::Bool(markers.iter().any(|m| std::path::Path::new(m).exists()))
    } else {
        Value::Null
    }
}

pub(super) fn header(
    id: &str,
    start: &OperationStart,
    identity: &WriterIdentity,
    now: SystemTime,
) -> Value {
    json!({
        "type": "header",
        "format": RECORDING_FORMAT,
        "format_version": RECORDING_FORMAT_VERSION,
        "operation_id": id,
        "operation_kind": start.kind.map(|k| k.as_str()),
        "started_at": wall(now),
        "writer": identity,
        "platform": platform(),
        "target": start.target,
        "gateway": start.gateway,
        "related": start.related,
        "settings": start.settings,
        "redaction": {
            "policy": "authorize and key-write keys, KNX Secure keys and passwords are never recorded",
            "marker": "redacted octets are zero and named by a `redacted` object",
        },
    })
}

pub(super) fn wire(seq: u64, started: Instant, record: &WireRecord) -> Value {
    match record {
        WireRecord::Datagram {
            at,
            wall: time,
            direction,
            bytes,
            redaction,
        } => {
            let description = describe_datagram(bytes);
            let mut line = Map::new();
            line.insert("type".into(), "datagram".into());
            line.insert("seq".into(), seq.into());
            line.insert("t_us".into(), since(started, *at).into());
            line.insert("wall".into(), wall(*time).into());
            line.insert("dir".into(), direction.as_str().into());
            line.insert("octets".into(), bytes.len().into());
            line.insert("raw".into(), hex(bytes).into());
            if let Some(service) = description.service {
                line.insert("service".into(), service.into());
            } else if let Some(service_type) = description.service_type {
                line.insert("service".into(), format!("{service_type:#06x}").into());
            }
            insert_opt(&mut line, "channel", description.channel_id);
            insert_opt(&mut line, "tunnel_seq", description.sequence);
            insert_opt(&mut line, "status", description.status);
            if let Some(cemi) = description.cemi {
                line.insert(
                    "cemi".into(),
                    json!({
                        "message": cemi.message,
                        "confirm_error": cemi.confirm_error,
                        "source": cemi.source,
                        "destination": cemi.destination,
                        "tpci": cemi.tpci,
                        "apci": cemi.apci,
                        "summary": cemi.summary,
                    }),
                );
            }
            if let Some(error) = description.decode_error {
                line.insert("decode_error".into(), error.into());
            }
            if let Some(redaction) = redaction {
                line.insert(
                    "redacted".into(),
                    json!({
                        "kind": redaction.kind.as_str(),
                        "offset": redaction.offset,
                        "octets": redaction.octets,
                    }),
                );
            }
            Value::Object(line)
        }
        WireRecord::Event {
            at,
            wall: time,
            event,
        } => {
            let mut line = Map::new();
            line.insert("type".into(), "tunnel".into());
            line.insert("seq".into(), seq.into());
            line.insert("t_us".into(), since(started, *at).into());
            line.insert("wall".into(), wall(*time).into());
            for (key, value) in tunnel_event(event) {
                line.insert(key.into(), value);
            }
            Value::Object(line)
        }
    }
}

fn insert_opt(line: &mut Map<String, Value>, key: &str, value: Option<u8>) {
    if let Some(value) = value {
        line.insert(key.into(), value.into());
    }
}

fn tunnel_event(event: &TunnelTapEvent) -> Vec<(&'static str, Value)> {
    match event {
        TunnelTapEvent::Connecting { gateway } => vec![
            ("event", "connecting".into()),
            ("gateway", gateway.to_string().into()),
        ],
        TunnelTapEvent::Connected {
            channel_id,
            assigned_address,
        } => vec![
            ("event", "connected".into()),
            ("channel", (*channel_id).into()),
            ("assigned_address", assigned_address.to_string().into()),
        ],
        TunnelTapEvent::ConnectFailed { reason } => vec![
            ("event", "connect-failed".into()),
            ("reason", reason.clone().into()),
        ],
        TunnelTapEvent::RequestAttempt { sequence, attempt } => vec![
            ("event", "request-attempt".into()),
            ("tunnel_seq", (*sequence).into()),
            ("attempt", (*attempt).into()),
        ],
        TunnelTapEvent::AckTimeout { sequence, attempt } => vec![
            ("event", "ack-timeout".into()),
            ("tunnel_seq", (*sequence).into()),
            ("attempt", (*attempt).into()),
        ],
        TunnelTapEvent::ReceiveDiscarded { sequence, reason } => vec![
            ("event", "receive-discarded".into()),
            ("tunnel_seq", (*sequence).into()),
            ("reason", (*reason).into()),
        ],
        TunnelTapEvent::CemiDecodeFailed { reason } => vec![
            ("event", "cemi-decode-failed".into()),
            ("reason", reason.clone().into()),
        ],
        TunnelTapEvent::HeartbeatAttempt { attempt } => vec![
            ("event", "heartbeat-attempt".into()),
            ("attempt", (*attempt).into()),
        ],
        TunnelTapEvent::Closed(cause) => vec![
            ("event", "closed".into()),
            ("cause", format!("{cause:?}").into()),
            ("description", cause.to_string().into()),
        ],
        TunnelTapEvent::SocketError { operation, reason } => vec![
            ("event", "socket-error".into()),
            ("operation", (*operation).into()),
            ("reason", reason.clone().into()),
        ],
    }
}

pub(super) fn note(
    seq: u64,
    started: Instant,
    at: Instant,
    time: SystemTime,
    name: &str,
    data: &Value,
) -> Value {
    json!({
        "type": "note",
        "seq": seq,
        "t_us": since(started, at),
        "wall": wall(time),
        "name": name,
        "data": data,
    })
}

pub(super) fn gap(lost_records: u64, reason: &str, time: SystemTime) -> Value {
    json!({
        "type": "gap",
        "wall": wall(time),
        "lost_records": lost_records,
        "reason": reason,
    })
}

pub(super) struct EndCounts {
    pub records: u64,
    pub dropped: u64,
    pub failed_writes: u64,
}

pub(super) fn end(
    seq: u64,
    started: Instant,
    at: Instant,
    time: SystemTime,
    outcome: &RecordedOutcome,
    counts: &EndCounts,
) -> Value {
    json!({
        "type": "end",
        "seq": seq,
        "t_us": since(started, at),
        "wall": wall(time),
        "outcome": outcome.status,
        "error_chain": outcome.error_chain,
        "summary": outcome.summary,
        "records_written": counts.records,
        "records_dropped": counts.dropped,
        "failed_writes": counts.failed_writes,
    })
}
