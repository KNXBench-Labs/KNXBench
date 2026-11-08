//! Allowlisted session-log, bus-monitor and client-state projections for the diagnostic bundle.
//!
//! Nothing here is a secret detector. Each value leaves only when it matches a closed
//! grammar (an enum, a bounded integer, a fixed server sentence, a known name); every
//! other value is withheld in place and counted in the section's `withheld` object, so a
//! reduction is never silent and never echoes the withheld text or field name.

use std::collections::BTreeMap;
use std::time::{Duration, Instant};

use knx_app::diagnostic_bundle::{BundleFile, BundleSection, ContentMarkers, SectionKind};
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};

use crate::bus::{DecodedValue, SessionStatus, TelegramRow};
use crate::session_log::{LogEntry, SettingsDiagnostic};

pub(super) const NOTE_REDUCED: &str =
    "values that could not be classified are withheld; the section counts them";
pub(super) const NOTE_SESSION_LOG: &str =
    "the session log keeps time, severity and known sources and messages only";
pub(super) const NOTE_MONITOR: &str =
    "group telegrams of the last 10 minutes from the bus monitor's existing buffer";
pub(super) const NOTE_CLIENT: &str =
    "only the known browser facts are kept; other fields are counted, never shown";

/// Decision 14: the bus monitor section covers the last ten minutes, boundary included.
pub(super) const MONITOR_WINDOW: Duration = Duration::from_secs(600);
/// What a withheld path inside a known sentence is replaced with.
const WITHHELD: &str = "[withheld]";
/// The web keeps at most this many recent errors; more is not the known schema.
const MAX_CLIENT_ERRORS: usize = 50;

/// Counts of withheld values per kind, serialized in a fixed order.
#[derive(Default)]
struct Withheld(BTreeMap<&'static str, u64>);

impl Withheld {
    fn new(kinds: &[&'static str]) -> Self {
        Self(kinds.iter().map(|kind| (*kind, 0)).collect())
    }

    fn add(&mut self, kind: &'static str) {
        *self.0.entry(kind).or_default() += 1;
    }

    fn any(&self) -> bool {
        self.0.values().any(|count| *count > 0)
    }

    fn to_value(&self) -> Value {
        Value::Object(
            self.0
                .iter()
                .map(|(k, v)| (k.to_string(), json!(v)))
                .collect(),
        )
    }
}

/// Session-log sources written by server code as literals.
const LOG_SOURCES: &[&str] = &[
    "log",
    "import",
    "import:validate",
    "import:map",
    "import:source-observations",
    "import:unknown",
    "import:opaque",
    "import:opaque:raw-text",
    "import:conflict",
    "import:unsupported",
    "csv-import",
    "csv-import:ignored-column",
    "csv-export",
    "doc-export",
    "open",
    "save",
    "new",
    "settings",
    "achievements",
    "product data",
    "product database",
    "undo",
    "redo",
    "AddressProgramming",
    "DeviceDownload",
    "AdoptDeviceComparison",
];

/// `knx_core::Command` variant names, which edit entries use as their source. A new
/// variant that is missing here is withheld and counted, never guessed.
const COMMAND_SOURCES: &[&str] = &[
    "AddComObjects",
    "AddFunctionMember",
    "Batch",
    "CreateArea",
    "CreateBuildingPart",
    "CreateDevice",
    "CreateFunction",
    "CreateGroupAddress",
    "CreateGroupRange",
    "CreateLine",
    "DeleteArea",
    "DeleteBuildingPart",
    "DeleteDevice",
    "DeleteFunction",
    "DeleteGroupAddress",
    "DeleteGroupRange",
    "DeleteLine",
    "InsertProjectNote",
    "LinkComObject",
    "MoveBuildingPart",
    "MoveDeviceToBuildingPart",
    "MoveDeviceToLine",
    "MoveFunction",
    "MoveGroupRange",
    "MoveLineToArea",
    "MoveProjectNote",
    "ReaddressGroupAddress",
    "RemoveAddedComObjects",
    "RemoveFunctionMember",
    "RemoveProjectNote",
    "RenameArea",
    "RenameBuildingPart",
    "RenameDevice",
    "RenameGroupAddress",
    "RenameGroupRange",
    "RenameInstallation",
    "RenameLine",
    "RepairDevicePlacement",
    "RepairLineOwner",
    "ReplaceProjectNote",
    "ReserveIds",
    "RestoreArea",
    "RestoreBuildingPart",
    "RestoreBuildingPartPlacement",
    "RestoreComObjectDescription",
    "RestoreComObjectDpt",
    "RestoreComObjectFlag",
    "RestoreDevice",
    "RestoreDeviceBuildingPlacement",
    "RestoreDeviceName",
    "RestoreDevicePlacements",
    "RestoreFunction",
    "RestoreFunctionDetails",
    "RestoreFunctionMember",
    "RestoreGroupAddress",
    "RestoreGroupAddressName",
    "RestoreGroupLink",
    "RestoreGroupRange",
    "RestoreGroupRangePlacement",
    "RestoreIndividualAddress",
    "RestoreLine",
    "RestoreLineOwners",
    "RestoreLinePlacement",
    "RestoreParameterValue",
    "SetComObjectDescription",
    "SetComObjectDpt",
    "SetComObjectFlag",
    "SetComObjectPriority",
    "SetCouplerIndividualAddress",
    "SetDeviceDescription",
    "SetDeviceInstructions",
    "SetDeviceProgramming",
    "SetGroupAddressStyle",
    "SetIdAllocators",
    "SetIndividualAddress",
    "SetParameterValue",
    "UnlinkComObject",
    "UpdateFunction",
    "UpdateGroupAddress",
];

fn known_source(source: &str) -> bool {
    LOG_SOURCES.contains(&source) || COMMAND_SOURCES.contains(&source)
}

/// Server sentences that carry no value of their own; `{n}` is a bounded decimal integer
/// and `{source}` one of [`LOG_SOURCES`].
const MESSAGE_FORMS: &[&str] = &[
    "saved",
    "undo",
    "redo",
    "XML element observations include retained subtrees; not semantic acceptance",
    "refused to create a new project: the open one has unsaved edits",
    "refused to {source}: the open project has unsaved edits",
    "raised a stored id counter that was below an id already in use; no project content was changed",
    "Adopted observed device values (named parameters and/or object links) into the project \
     (one undoable edit; no device write)",
    "the project was saved, but its manufacturer files could not be stored in the file",
    "the project file now carries {n} manufacturer file(s) itself, so it opens with its \
     application programs on any computer",
    "{n} manufacturer file(s) of this project are not in the product database, so the project \
     file cannot carry them; restore them from the original .knxproj (a device's Parameters \
     tab) and save again",
    "{n} log entries dropped after exceeding the {n}-entry session log cap ({n} info, {n} \
     warning, {n} error; least severe and oldest first)",
];

/// Sentences whose whole tail is a path or file name; only the prefix leaves.
const PATH_PREFIXES: &[&str] = &["opened ", "saved as ", "imported "];

fn matches_form(form: &str, message: &str) -> bool {
    let mut rest = message;
    let mut pieces = form.split('{').peekable();
    let first = pieces.next().unwrap_or_default();
    let Some(after) = rest.strip_prefix(first) else {
        return false;
    };
    rest = after;
    for piece in pieces {
        let (placeholder, literal) = piece.split_once('}').unwrap_or((piece, ""));
        let end = if literal.is_empty() {
            rest.len()
        } else {
            match rest.find(literal) {
                Some(end) => end,
                None => return false,
            }
        };
        let value = &rest[..end];
        let valid = match placeholder {
            "n" => (1..=10).contains(&value.len()) && value.bytes().all(|b| b.is_ascii_digit()),
            "source" => LOG_SOURCES.contains(&value),
            _ => false,
        };
        if !valid {
            return false;
        }
        rest = &rest[end + literal.len()..];
    }
    rest.is_empty()
}

/// Entity names of `knx_etsproj::ImportReport` count rows.
const IMPORT_ENTITIES: &[&str] = &[
    "Area",
    "BuildingPart",
    "ComObjectInstanceRef",
    "DeviceInstance",
    "Function",
    "GroupAddress",
    "GroupAddressRef",
    "GroupRange",
    "Line",
    "ParameterInstanceRef",
];

/// `Entity: mapped/read, ...` exactly as the importer's success line joins them.
fn import_counts(counts: &str) -> bool {
    let number = |s: &str| (1..=10).contains(&s.len()) && s.bytes().all(|b| b.is_ascii_digit());
    !counts.is_empty()
        && counts.split(", ").all(|row| {
            row.split_once(": ").is_some_and(|(entity, pair)| {
                IMPORT_ENTITIES.contains(&entity)
                    && pair
                        .split_once('/')
                        .is_some_and(|(mapped, read)| number(mapped) && number(read))
            })
        })
}

/// A message that leaves (possibly with its path withheld), or `None`.
fn classify_message(message: &str, withheld: &mut Withheld) -> Option<String> {
    if MESSAGE_FORMS.iter().any(|form| matches_form(form, message)) {
        return Some(message.to_string());
    }
    if let Some(prefix) = PATH_PREFIXES.iter().find(|p| message.starts_with(**p)) {
        withheld.add("path");
        // `imported <file> (<counts>)`: the importer's own entity counts survive.
        let counts = (*prefix == "imported ")
            .then(|| message.rsplit_once(" ("))
            .flatten()
            .and_then(|(_, tail)| tail.strip_suffix(')'))
            .filter(|counts| import_counts(counts));
        return Some(match counts {
            Some(counts) => format!("{prefix}{WITHHELD} ({counts})"),
            None => format!("{prefix}{WITHHELD}"),
        });
    }
    withheld.add("message");
    None
}

/// RFC 3339 re-rendered as UTC with milliseconds, or `None`.
fn timestamp(value: &str) -> Option<String> {
    chrono::DateTime::parse_from_rfc3339(value).ok().map(|t| {
        t.with_timezone(&chrono::Utc)
            .to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
    })
}

fn source_value(source: &str, withheld: &mut Withheld) -> Value {
    if known_source(source) {
        json!(source)
    } else {
        withheld.add("source");
        Value::Null
    }
}

fn diagnostic(diagnostic: &SettingsDiagnostic, withheld: &mut Withheld) -> Value {
    match diagnostic {
        SettingsDiagnostic::Quarantined { reason, .. } => {
            withheld.add("path");
            json!({"kind": "quarantined", "reason": reason})
        }
        other => serde_json::to_value(other).unwrap_or(Value::Null),
    }
}

fn section(
    kind: SectionKind,
    file: &str,
    body: &Value,
    entries: usize,
    note: &str,
    reduced: bool,
) -> BundleSection {
    let mut section = BundleSection::new(kind);
    section.files.push(BundleFile::json(file, body));
    section.entries = entries;
    section.notes.push(note.into());
    if reduced {
        section.notes.push(NOTE_REDUCED.into());
    }
    section
}

/// The session log, from one owned snapshot of its entries, in their order.
pub(super) fn session_log(entries: &[LogEntry]) -> BundleSection {
    let mut withheld = Withheld::new(&[
        "timestamp",
        "source",
        "message",
        "location",
        "detail",
        "path",
    ]);
    let projected: Vec<Value> = entries
        .iter()
        .map(|entry| {
            let mut row = Map::new();
            row.insert(
                "timestamp".into(),
                timestamp(&entry.timestamp).map_or_else(
                    || {
                        withheld.add("timestamp");
                        Value::Null
                    },
                    Value::String,
                ),
            );
            row.insert("severity".into(), json!(entry.severity));
            row.insert("source".into(), source_value(&entry.source, &mut withheld));
            row.insert(
                "message".into(),
                classify_message(&entry.message, &mut withheld).map_or(Value::Null, Value::String),
            );
            if entry.location.is_some() {
                withheld.add("location");
            }
            if entry.detail.is_some() {
                withheld.add("detail");
            }
            if let Some(found) = &entry.diagnostic {
                row.insert("diagnostic".into(), diagnostic(found, &mut withheld));
            }
            Value::Object(row)
        })
        .collect();
    let body = json!({
        "format": "knxbench-session-log",
        "formatVersion": 1,
        "entriesTotal": projected.len(),
        "entries": projected,
        "withheld": withheld.to_value(),
    });
    let mut section = section(
        SectionKind::SessionLog,
        "session-log.json",
        &body,
        entries.len(),
        NOTE_SESSION_LOG,
        withheld.any(),
    );
    section.markers = ContentMarkers::default();
    section
}

/// One owned copy of the running monitor's buffer, taken under its lock at `taken`.
pub(super) struct MonitorSnapshot {
    /// [`crate::bus::BusSession::id`]: a new session restarts `seq` at 0.
    pub session_id: u64,
    /// Actual IP transport of this immutable session; absent for serial/unrecorded sources.
    pub transport: Option<knx_net::TunnelTransport>,
    /// Cumulative skipped TCP frames when the snapshot was taken; `None` unless a TCP session.
    pub skipped_frames: Option<u64>,
    pub status: SessionStatus,
    pub dropped_before: u64,
    pub next_seq: u64,
    pub rows: Vec<TelegramRow>,
    pub taken: Instant,
}

/// The existing buffer, if a monitor session exists; never starts or contacts anything.
pub(super) async fn monitor_snapshot(state: &crate::AppState) -> Option<MonitorSnapshot> {
    let guard = state.bus_session.lock().await;
    let session = guard.as_ref()?;
    let buffer = session.buffer();
    let buffer = buffer.lock().expect("bus session buffer poisoned");
    Some(MonitorSnapshot {
        session_id: session.id(),
        transport: session.transport(),
        skipped_frames: session.skipped_frames(),
        status: buffer.status(),
        dropped_before: buffer.dropped_before(),
        next_seq: buffer.next_seq(),
        rows: buffer.telegrams_since(0),
        taken: Instant::now(),
    })
}

/// Group services whose payload is a group value; every other service is metadata only.
const VALUE_SERVICES: &[&str] = &["GroupValueWrite", "GroupValueResponse"];
/// Every service name a monitor row can carry.
const MONITOR_SERVICES: &[&str] = &[
    "GroupValueRead",
    "GroupValueResponse",
    "GroupValueWrite",
    "DeviceDescriptorRead",
    "DeviceDescriptorResponse",
    "NoApplicationPdu",
    "Other",
    "SessionClosed",
    "IndividualAddressWrite",
    "IndividualAddressRead",
    "IndividualAddressResponse",
    "IndividualAddressSerialNumberRead",
    "IndividualAddressSerialNumberResponse",
    "IndividualAddressSerialNumberWrite",
    "FunctionPropertyCommand",
    "FunctionPropertyStateRead",
    "FunctionPropertyStateResponse",
    "DomainAddressWrite",
    "DomainAddressRead",
    "DomainAddressResponse",
    "DomainAddressSerialNumberRead",
    "DomainAddressSerialNumberResponse",
    "DomainAddressSerialNumberWrite",
    "MemoryRead",
    "MemoryResponse",
    "MemoryWrite",
    "UserMemoryRead",
    "UserMemoryResponse",
    "UserMemoryWrite",
    "Restart",
    "AuthorizeRequest",
    "AuthorizeResponse",
    "PropertyDescriptionRead",
    "PropertyDescriptionResponse",
    "PropertyValueRead",
    "PropertyValueResponse",
    "PropertyValueWrite",
];
/// DPT main numbers whose decoded value is withheld with its octets: the character
/// strings (4, 16, 24, 28) and DPT 15 access data, which carries entry-control codes.
/// Few octets are no classification: a numeric PIN fits in two.
const WITHHELD_DPT_MAINS: &[u16] = &[4, 15, 16, 24, 28];
/// Projection limit for typed group-value octets; larger payloads are withheld and counted.
const MAX_PAYLOAD_OCTETS: usize = 14;
/// Longest project group-address name that leaves; longer names are cut and counted.
const MAX_NAME_CHARS: usize = 128;

/// The bus formatter's own payload forms (`format_group_value_payload`).
fn formatter_payload(payload: &str) -> bool {
    let hex2 = |s: &str| s.len() == 2 && s.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'));
    if let Some(bits) = payload
        .strip_prefix("0x")
        .and_then(|p| p.strip_suffix(" (6-bit)"))
    {
        return hex2(bits);
    }
    payload
        .strip_prefix('[')
        .and_then(|p| p.strip_suffix(']'))
        .is_some_and(|inner| {
            let octets: Vec<&str> = inner.split(", ").collect();
            octets.len() <= MAX_PAYLOAD_OCTETS && octets.iter().all(|o| hex2(o))
        })
}

fn dpt(value: &str) -> Option<knx_core::DptRef> {
    knx_core::DptRef::parse(value).ok()
}

fn address_text(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 16
        && value
            .split(['/', '.'])
            .all(|part| (1..=5).contains(&part.len()) && part.bytes().all(|b| b.is_ascii_digit()))
}

/// Decoded outcome without any free codec text; `true` only for a successful decode of a
/// known type whose value may leave, the one case where the raw octets may leave too.
fn decoded(value: &DecodedValue, withheld: &mut Withheld) -> (Value, bool) {
    match value {
        DecodedValue::Value { dpt: name, text } => match dpt(name) {
            Some(parsed) if WITHHELD_DPT_MAINS.contains(&parsed.main) => {
                withheld.add("decodedText");
                (json!({"kind": "value", "dpt": parsed.to_string()}), false)
            }
            Some(parsed) => (
                json!({"kind": "value", "dpt": parsed.to_string(), "text": text}),
                true,
            ),
            None => {
                withheld.add("decodedText");
                (json!({"kind": "value"}), false)
            }
        },
        DecodedValue::Unresolved { reason, .. } => (
            json!({"kind": "unresolved", "reason": reason.as_str()}),
            false,
        ),
        DecodedValue::Conflict { .. } => {
            withheld.add("decodedText");
            (json!({"kind": "conflict"}), false)
        }
        DecodedValue::Error {
            dpt: name, reason, ..
        } => {
            withheld.add("decodedText");
            let reason = match reason {
                crate::bus::DecodeFailureReason::UnsupportedDpt => "unsupportedDpt",
                crate::bus::DecodeFailureReason::DecodeFailed => "decodeFailed",
            };
            let text = dpt(name).map(|parsed| parsed.to_string());
            (
                json!({"kind": "error", "dpt": text, "reason": reason}),
                false,
            )
        }
    }
}

fn monitor_row(row: &TelegramRow, withheld: &mut Withheld) -> Value {
    let service = if MONITOR_SERVICES.contains(&row.service.as_str()) {
        Some(row.service.as_str())
    } else {
        withheld.add("service");
        None
    };
    let value_service = service.is_some_and(|s| VALUE_SERVICES.contains(&s));
    let mut typed_value = false;
    let decoded = match &row.decoded {
        Some(value) if value_service => {
            let (projected, typed) = decoded(value, withheld);
            typed_value = typed;
            Some(projected)
        }
        Some(_) => {
            withheld.add("decodedText");
            None
        }
        None => None,
    };
    let payload = match &row.raw_payload {
        None => Value::Null,
        Some(raw) if typed_value && formatter_payload(raw) => json!(raw),
        Some(_) => {
            withheld.add("payload");
            Value::Null
        }
    };
    let address = |value: &str, withheld: &mut Withheld| {
        if address_text(value) || (row.service == "SessionClosed" && value == "-") {
            json!(value)
        } else {
            withheld.add("address");
            Value::Null
        }
    };
    // Owner decision 3 admits project labels; this is not a claim that names are credential-free.
    let name = row.destination_name.as_ref().map(|name| {
        if name.chars().count() > MAX_NAME_CHARS {
            withheld.add("destinationNameTruncated");
            name.chars().take(MAX_NAME_CHARS).collect::<String>()
        } else {
            name.clone()
        }
    });
    json!({
        "seq": row.seq,
        "timestamp": timestamp(&row.timestamp).or_else(|| {
            withheld.add("timestamp");
            None
        }),
        "source": address(&row.source, withheld),
        "destination": address(&row.destination, withheld),
        "destinationName": name,
        "service": service,
        "payload": payload,
        "decoded": decoded,
        "control": row.control.map(|control| json!({
            "priority": crate::bus::priority_name(control.priority),
            "repeated": control.repeated,
            "hopCount": control.hop_count,
        })),
        "flowGeneration": row.flow_generation,
    })
}

/// The window a preview reviewed: the monitor session and the exact `seq` interval, plus
/// the facts shown about it. Later text and export requests project exactly these rows.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct MonitorBinding {
    pub session_id: u64,
    /// First `seq` of the window; equal to `end_seq` for an empty window.
    pub first_seq: u64,
    /// One past the last `seq`: the buffer's `next_seq` when the preview was taken.
    pub end_seq: u64,
    pub dropped_before: u64,
    /// TCP frame-skip count captured by this review, never refreshed by text/ZIP collection.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub skipped_frames_at_review: Option<u64>,
    pub status_at_review: MonitorStatus,
    /// UTC RFC 3339 with milliseconds, as rendered by the preview.
    pub captured_at: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum MonitorStatus {
    Active,
    Closed,
}

impl MonitorBinding {
    /// Closed bounds a well-formed binding keeps; anything else is not a reviewed window.
    pub(crate) fn is_well_formed(&self) -> bool {
        const MAX_SAFE_INTEGER: u64 = 9_007_199_254_740_991;
        self.first_seq <= self.end_seq
            && self.end_seq - self.first_seq <= crate::bus::MAX_TELEGRAMS as u64
            && self.end_seq <= MAX_SAFE_INTEGER
            && self.session_id <= MAX_SAFE_INTEGER
            && self.dropped_before <= MAX_SAFE_INTEGER
            && self
                .skipped_frames_at_review
                .is_none_or(|count| count <= MAX_SAFE_INTEGER)
            && timestamp(&self.captured_at).as_deref() == Some(self.captured_at.as_str())
    }
}

/// The window of a fresh preview: rows at most [`MONITOR_WINDOW`] old at `snapshot.taken`.
pub(super) fn monitor_window(snapshot: &MonitorSnapshot) -> MonitorBinding {
    let first_seq = snapshot
        .rows
        .iter()
        .filter(|row| snapshot.taken.saturating_duration_since(row.observed_at) <= MONITOR_WINDOW)
        .map(|row| row.seq)
        .min()
        .unwrap_or(snapshot.next_seq);
    MonitorBinding {
        session_id: snapshot.session_id,
        first_seq,
        end_seq: snapshot.next_seq,
        dropped_before: snapshot.dropped_before,
        skipped_frames_at_review: snapshot.skipped_frames,
        status_at_review: match snapshot.status {
            SessionStatus::Active => MonitorStatus::Active,
            SessionStatus::Closed => MonitorStatus::Closed,
        },
        captured_at: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
    }
}

/// The monitor section for exactly `binding`'s rows, or `None` when the snapshot belongs to
/// another session or lacks any pinned row (evicted, duplicated or out of order).
pub(super) fn bus_monitor(
    snapshot: &MonitorSnapshot,
    binding: &MonitorBinding,
) -> Option<BundleSection> {
    if snapshot.session_id != binding.session_id
        || !binding.is_well_formed()
        || binding.end_seq > snapshot.next_seq
        || binding.skipped_frames_at_review.is_some()
            != (snapshot.transport == Some(knx_net::TunnelTransport::Tcp))
        || binding.skipped_frames_at_review.is_some_and(|reviewed| {
            snapshot
                .skipped_frames
                .is_none_or(|current| current < reviewed)
        })
    {
        return None;
    }
    let pinned: Vec<&TelegramRow> = snapshot
        .rows
        .iter()
        .filter(|row| (binding.first_seq..binding.end_seq).contains(&row.seq))
        .collect();
    let contiguous = pinned
        .iter()
        .enumerate()
        .all(|(index, row)| row.seq == binding.first_seq + index as u64);
    if pinned.len() as u64 != binding.end_seq - binding.first_seq || !contiguous {
        return None;
    }
    let mut withheld = Withheld::new(&[
        "payload",
        "decodedText",
        "timestamp",
        "address",
        "service",
        "destinationNameTruncated",
    ]);
    let telegrams: Vec<Value> = pinned
        .into_iter()
        .map(|row| monitor_row(row, &mut withheld))
        .collect();
    let entries = telegrams.len();
    let mut body = json!({
        "format": "knxbench-bus-monitor",
        "formatVersion": 1,
        "sessionId": binding.session_id,
        "statusAtReview": binding.status_at_review,
        "capturedAt": binding.captured_at,
        "windowSeconds": MONITOR_WINDOW.as_secs(),
        "firstSeq": binding.first_seq,
        "endSeq": binding.end_seq,
        "droppedBeforeAtReview": binding.dropped_before,
        "telegrams": telegrams,
        "withheld": withheld.to_value(),
    });
    if let Some(transport) = snapshot.transport {
        body["transport"] = json!(transport.as_str());
    }
    // The exact skip count reviewed with this window, not later live observations.
    if let Some(skipped) = binding.skipped_frames_at_review {
        body["skippedFramesAtReview"] = json!(skipped);
    }
    let mut section = section(
        SectionKind::BusMonitor,
        "bus-monitor.json",
        &body,
        entries,
        NOTE_MONITOR,
        withheld.any(),
    );
    section.markers = ContentMarkers {
        addresses: true,
        names: true,
        parameter_values: false,
        network_addresses: false,
    };
    Some(section)
}

const CLIENT_FIELDS: &[&str] = &[
    "appVersion",
    "shell",
    "uiLanguage",
    "theme",
    "viewport",
    "uiScale",
    "truncated",
    "metadataTruncated",
    "omittedErrorCount",
    "truncatedErrorCount",
    "recentErrors",
];
/// Built-in languages and the shipped language packs (`bundledLanguagePacks.ts`).
const UI_LANGUAGES: &[&str] = &["en", "de", "bar", "tlh"];
/// Built-in themes and the shipped theme packs (`apps/knx-web/themes`); imported ids are "other".
const SHIPPED_THEMES: &[&str] = &[
    "porcelain",
    "graphite",
    "cupertino",
    "lcars",
    "rail",
    "rail-dark",
    "system",
    "user-modern-retro-green-crt",
];

fn app_version(value: &str) -> bool {
    let (core, pre) = value.split_once('-').unwrap_or((value, ""));
    let number = |s: &str| (1..=4).contains(&s.len()) && s.bytes().all(|b| b.is_ascii_digit());
    let parts: Vec<&str> = core.split('.').collect();
    parts.len() == 3
        && parts.iter().all(|p| number(p))
        && (pre.is_empty()
            || pre
                .split_once('.')
                .is_some_and(|(label, n)| matches!(label, "alpha" | "beta" | "rc") && number(n)))
}

/// One closed-schema field of the client facts; `None` when absent.
fn client_field(key: &str, value: &Value, withheld: &mut Withheld) -> Value {
    let invalid = |withheld: &mut Withheld| {
        withheld.add("invalidFields");
        Value::Null
    };
    match key {
        "appVersion" => match value.as_str() {
            Some(text) if app_version(text) => json!(text),
            _ => invalid(withheld),
        },
        "shell" => match value.as_str() {
            Some(text @ ("tauri" | "browser")) => json!(text),
            _ => invalid(withheld),
        },
        "uiLanguage" | "theme" => {
            let known = if key == "theme" {
                SHIPPED_THEMES
            } else {
                UI_LANGUAGES
            };
            match value.as_str() {
                Some(text) if known.contains(&text) => json!(text),
                Some(_) => json!("other"),
                None => invalid(withheld),
            }
        }
        "viewport" => match value.as_object() {
            Some(object) => {
                let mut out = Map::new();
                for (name, dimension) in object {
                    if name != "width" && name != "height" {
                        withheld.add("unknownFields");
                        continue;
                    }
                    let fits = dimension.as_u64().filter(|v| *v <= 100_000);
                    out.insert(
                        name.clone(),
                        fits.map_or_else(|| invalid(withheld), |v| json!(v)),
                    );
                }
                Value::Object(out)
            }
            None => invalid(withheld),
        },
        "uiScale" => match value.as_f64() {
            Some(scale) if (0.1..=10.0).contains(&scale) => value.clone(),
            _ => invalid(withheld),
        },
        "truncated" | "metadataTruncated" => match value.as_bool() {
            Some(flag) => json!(flag),
            None => invalid(withheld),
        },
        "omittedErrorCount" | "truncatedErrorCount" => match value.as_u64() {
            Some(count) if count <= 1_000_000 => json!(count),
            _ => invalid(withheld),
        },
        "recentErrors" => match value.as_array() {
            Some(errors) if errors.len() <= MAX_CLIENT_ERRORS => Value::Array(
                errors
                    .iter()
                    .map(|error| client_error(error, withheld))
                    .collect(),
            ),
            _ => invalid(withheld),
        },
        _ => unreachable!("only closed-schema fields are projected"),
    }
}

fn client_error(error: &Value, withheld: &mut Withheld) -> Value {
    let Some(object) = error.as_object() else {
        withheld.add("invalidFields");
        return Value::Null;
    };
    let mut out = Map::new();
    for (name, value) in object {
        match name.as_str() {
            "timestamp" => {
                let time = value.as_str().and_then(timestamp);
                if time.is_none() {
                    withheld.add("timestamp");
                }
                out.insert(name.clone(), json!(time));
            }
            "source" => {
                let source = match value.as_str() {
                    Some(text) => source_value(text, withheld),
                    None => {
                        withheld.add("source");
                        Value::Null
                    }
                };
                out.insert(name.clone(), source);
            }
            "message" => {
                let message = match value.as_str() {
                    Some(text) => classify_message(text, withheld),
                    None => {
                        withheld.add("message");
                        None
                    }
                };
                out.insert(name.clone(), json!(message));
            }
            "detail" => {
                if !value.is_null() {
                    withheld.add("detail");
                }
            }
            _ => withheld.add("unknownFields"),
        }
    }
    Value::Object(out)
}

/// The browser's facts, reduced to the closed schema the web sends.
pub(super) fn client_state(client: &Map<String, Value>) -> BundleSection {
    let mut withheld = Withheld::new(&[
        "unknownFields",
        "invalidFields",
        "timestamp",
        "source",
        "message",
        "detail",
        "path",
    ]);
    let mut body = Map::new();
    body.insert("format".into(), json!("knxbench-client-state"));
    body.insert("formatVersion".into(), json!(1));
    for key in CLIENT_FIELDS {
        if let Some(value) = client.get(*key) {
            body.insert(key.to_string(), client_field(key, value, &mut withheld));
        }
    }
    for key in client.keys() {
        if !CLIENT_FIELDS.contains(&key.as_str()) {
            withheld.add("unknownFields");
        }
    }
    body.insert("withheld".into(), withheld.to_value());
    section(
        SectionKind::ClientState,
        "client.json",
        &Value::Object(body),
        1,
        NOTE_CLIENT,
        withheld.any(),
    )
}

#[cfg(test)]
#[path = "diagnostic_bundle_sections_tests.rs"]
mod tests;
