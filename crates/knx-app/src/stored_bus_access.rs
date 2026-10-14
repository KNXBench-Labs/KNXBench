//! Stored bus access and IP configuration: read-only, endpoints hidden until asked (Goal B3).
//!
//! `[D]` KNX Project Schema 23 v01.00.00: `BusAccess_t` has `Name`
//! (required), `Edi` (a GUID, optional: *"If no Edi specified, the Parameter
//! contains the FalconConnectionString"*) and `Parameter` (required, *"The
//! parameters necessary for the connection"*, no syntax given). `IPConfig_t`
//! has `Assign` (`Auto`/`Fixed`), `IPAddress`, `SubnetMask`,
//! `DefaultGateway` and `MACAddress`.
//!
//! The importer keeps both elements verbatim as opaque rows; this module only
//! reads those rows. It never connects anywhere, never offers a stored
//! endpoint as a gateway and never writes. Every value that could name a host,
//! address, port or person is hidden unless the caller reveals one record;
//! only a few enumeration-like values are shown by default.
//!
//! `[V]` corpus classes, not values: `Parameter` is `key=value` parts
//! separated by `;` (sometimes ending in `;`), each part with exactly one
//! `=`. It is split only when that holds for every part, otherwise it stays
//! one hidden value and the record says so.

use knx_core::Project;
use knx_store::StoredOpaqueEntry;
use quick_xml::events::Event;
use quick_xml::{Reader, XmlVersion};
use serde::Serialize;

/// A record larger than this is listed with its provenance but not read. The
/// largest real record is a short attribute list; 64 KiB is far above it and
/// bounds every value shown.
pub const MAX_RECORD_BYTES: usize = 64 * 1024;

const RETAINED_ELEMENT_KIND: &str = "RetainedElement";
const BUS_ACCESS: &str = "BusAccess";
const IP_CONFIG: &str = "IPConfig";
/// Schema-named attributes, per element.
const BUS_ACCESS_ATTRIBUTES: &[&str] = &["Name", "Edi", "Parameter"];
const IP_CONFIG_ATTRIBUTES: &[&str] = &[
    "Assign",
    "IPAddress",
    "SubnetMask",
    "DefaultGateway",
    "MACAddress",
];
/// Values shown without revealing: enumeration-like settings, never an
/// endpoint. Shown only while the value itself is a short word.
const SHOWN_PARAMETER_KEYS: &[&str] = &["Type", "ProtocolType", "UseNat", "NAT"];
const SHOWN_IP_CONFIG_ATTRIBUTES: &[&str] = &["Assign"];
const SHOWN_WORD_MAX: usize = 32;
const PARAMETER: &str = "Parameter";

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StoredBusAccess {
    pub records: Vec<StoredRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StoredRecord {
    /// `<position>:<first 16 hex digits of the stored SHA-256>`; stable for
    /// one stored project, matches nothing after the rows change.
    pub record_id: String,
    /// `BusAccess` or `IPConfig`.
    pub element: String,
    pub owner: RecordOwner,
    pub source: RecordSource,
    pub status: RecordStatus,
    /// Attributes in document order; a split `Parameter` contributes its parts
    /// in place of itself.
    pub fields: Vec<RecordField>,
    /// What was kept but not read: child element names, and
    /// `parameterNotSplit` when `Parameter` did not split cleanly.
    pub unread: Vec<String>,
    /// True when this answer shows every value of this record.
    pub revealed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecordOwner {
    pub kind: OwnerKind,
    /// The project line or device id, when exactly one matches.
    pub id: Option<u32>,
    /// `1.1` for a line, `1.1.4` for a device, when known.
    pub address: Option<String>,
    pub name: Option<String>,
    pub reason: Option<UnattributedReason>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum OwnerKind {
    Line,
    Device,
    Unattributed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum UnattributedReason {
    /// The stored row names no line or device id.
    NotKeyed,
    /// No line or device of the project has that source id.
    NoMatch,
    /// More than one has it; picking one would be a guess.
    SeveralMatches,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecordSource {
    pub source_path: String,
    pub xpath: String,
    pub size: usize,
    pub sha256: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum RecordStatus {
    Read,
    TooLarge,
    NotWellFormed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecordField {
    pub name: String,
    pub part: FieldPart,
    /// Named by the schema (attributes) or seen in the corpus (parameter keys
    /// are all reported as known: the schema names none).
    pub known: bool,
    /// The value, when shown.
    pub value: Option<String>,
    /// What kind of value is hidden, when it is.
    pub hidden: Option<HiddenValue>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum FieldPart {
    Attribute,
    Parameter,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HiddenValue {
    pub class: ValueClass,
    /// Characters, so an empty value is not mistaken for a hidden secret.
    pub length: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ValueClass {
    Empty,
    Ipv4Address,
    MacAddress,
    Guid,
    Number,
    Text,
}

/// Every stored `BusAccess`/`IPConfig` record of the project, values hidden,
/// except the record whose id is `reveal`, which shows all of its values.
pub fn stored_bus_access(
    project: &Project,
    opaque: &[StoredOpaqueEntry],
    reveal: Option<&str>,
) -> StoredBusAccess {
    let records = opaque
        .iter()
        .filter(|e| {
            e.kind == RETAINED_ELEMENT_KIND && (e.name == BUS_ACCESS || e.name == IP_CONFIG)
        })
        .enumerate()
        .map(|(position, entry)| {
            let record_id = format!(
                "{position}:{}",
                entry.sha256.get(..16).unwrap_or(&entry.sha256)
            );
            let revealed = reveal == Some(record_id.as_str());
            read_record(project, entry, record_id, revealed)
        })
        .collect();
    StoredBusAccess { records }
}

fn read_record(
    project: &Project,
    entry: &StoredOpaqueEntry,
    record_id: String,
    revealed: bool,
) -> StoredRecord {
    let mut record = StoredRecord {
        record_id,
        element: entry.name.clone(),
        owner: owner(project, &entry.xpath),
        source: RecordSource {
            source_path: entry.source_path.clone(),
            xpath: entry.xpath.clone(),
            size: entry.bytes.len(),
            sha256: entry.sha256.clone(),
        },
        status: RecordStatus::Read,
        fields: Vec::new(),
        unread: Vec::new(),
        revealed,
    };
    if entry.bytes.len() > MAX_RECORD_BYTES {
        record.status = RecordStatus::TooLarge;
        return record;
    }
    match parse(&entry.bytes) {
        Some((attributes, children)) => {
            record.unread = children;
            for (name, value) in attributes {
                add_attribute(&mut record, &name, value, revealed);
            }
        }
        None => record.status = RecordStatus::NotWellFormed,
    }
    record
}

/// The root element's attributes (normalized values) and its direct child
/// element names, or `None` when the bytes are not one well-formed element.
type Parsed = (Vec<(String, String)>, Vec<String>);

fn parse(bytes: &[u8]) -> Option<Parsed> {
    let mut reader = Reader::from_reader(bytes);
    let mut attributes = Vec::new();
    let mut children = Vec::new();
    let mut depth = 0usize;
    let mut roots = 0usize;
    loop {
        let event = reader.read_event().ok()?;
        let (element, opens) = match &event {
            Event::Start(element) => (element, true),
            Event::Empty(element) => (element, false),
            Event::End(_) => {
                depth = depth.checked_sub(1)?;
                continue;
            }
            Event::Eof => return (roots == 1 && depth == 0).then_some((attributes, children)),
            _ => continue,
        };
        if depth == 0 {
            roots += 1;
            if roots > 1 {
                return None;
            }
            for attribute in element.attributes() {
                let attribute = attribute.ok()?;
                let local = attribute.key.local_name();
                let name: &str = local.as_ref();
                let name = name.to_string();
                let value = attribute.normalized_value(XmlVersion::Implicit1_0).ok()?;
                attributes.push((name, value.into_owned()));
            }
        } else if depth == 1 {
            let local = element.local_name();
            let name: &str = local.as_ref();
            children.push(name.to_string());
        }
        if opens {
            depth += 1;
        }
    }
}

fn add_attribute(record: &mut StoredRecord, name: &str, value: String, revealed: bool) {
    let (known_names, shown_names): (&[&str], &[&str]) = if record.element == BUS_ACCESS {
        (BUS_ACCESS_ATTRIBUTES, &[])
    } else {
        (IP_CONFIG_ATTRIBUTES, SHOWN_IP_CONFIG_ATTRIBUTES)
    };
    if record.element == BUS_ACCESS && name == PARAMETER {
        if let Some(parts) = split_parameter(&value) {
            for (key, part) in parts {
                let shown = SHOWN_PARAMETER_KEYS.contains(&key.as_str());
                record.fields.push(field(
                    key,
                    FieldPart::Parameter,
                    true,
                    part,
                    shown,
                    revealed,
                ));
            }
            return;
        }
        record.unread.push("parameterNotSplit".to_string());
    }
    let shown = shown_names.contains(&name);
    let known = known_names.contains(&name);
    record.fields.push(field(
        name.to_string(),
        FieldPart::Attribute,
        known,
        value,
        shown,
        revealed,
    ));
}

fn field(
    name: String,
    part: FieldPart,
    known: bool,
    value: String,
    shown_by_default: bool,
    revealed: bool,
) -> RecordField {
    if revealed || (shown_by_default && is_short_word(&value)) {
        return RecordField {
            name,
            part,
            known,
            value: Some(value),
            hidden: None,
        };
    }
    let hidden = HiddenValue {
        class: classify(&value),
        length: value.chars().count(),
    };
    RecordField {
        name,
        part,
        known,
        value: None,
        hidden: Some(hidden),
    }
}

/// `key=value` parts separated by `;`, only when every non-empty part has
/// exactly one `=` and an identifier key. Values are kept verbatim.
fn split_parameter(value: &str) -> Option<Vec<(String, String)>> {
    let mut parts = Vec::new();
    for part in value.split(';').filter(|p| !p.trim().is_empty()) {
        let (key, value) = part.split_once('=')?;
        if value.contains('=') {
            return None;
        }
        let key = key.trim();
        let mut chars = key.chars();
        if !chars.next().is_some_and(|c| c.is_ascii_alphabetic())
            || !chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
        {
            return None;
        }
        parts.push((key.to_string(), value.to_string()));
    }
    (!parts.is_empty()).then_some(parts)
}

/// A value that names no endpoint: letters only, optionally single-quoted.
fn is_short_word(value: &str) -> bool {
    let word = unquote(value);
    !word.is_empty()
        && word.len() <= SHOWN_WORD_MAX
        && word.chars().all(|c| c.is_ascii_alphabetic())
}

fn unquote(value: &str) -> &str {
    let value = value.trim();
    value
        .strip_prefix('\'')
        .and_then(|v| v.strip_suffix('\''))
        .unwrap_or(value)
}

fn classify(value: &str) -> ValueClass {
    let v = unquote(value);
    if v.is_empty() {
        ValueClass::Empty
    } else if v.parse::<std::net::Ipv4Addr>().is_ok() {
        ValueClass::Ipv4Address
    } else if is_mac(v) {
        ValueClass::MacAddress
    } else if is_guid(v) {
        ValueClass::Guid
    } else if v.chars().all(|c| c.is_ascii_digit()) {
        ValueClass::Number
    } else {
        ValueClass::Text
    }
}

fn is_mac(v: &str) -> bool {
    let parts: Vec<&str> = v.split([':', '-']).collect();
    parts.len() == 6
        && parts
            .iter()
            .all(|p| p.len() == 2 && p.chars().all(|c| c.is_ascii_hexdigit()))
}

fn is_guid(v: &str) -> bool {
    let v = v.trim_start_matches('{').trim_end_matches('}');
    let lengths: Vec<usize> = v.split('-').map(str::len).collect();
    lengths == [8, 4, 4, 4, 12] && v.chars().all(|c| c == '-' || c.is_ascii_hexdigit())
}

/// The last `Line[@Id='…']` or `DeviceInstance[@Id='…']` step of the stored
/// xpath decides the owner, and only when exactly one project element has
/// that source id.
fn owner(project: &Project, xpath: &str) -> RecordOwner {
    let unattributed = |reason| RecordOwner {
        kind: OwnerKind::Unattributed,
        id: None,
        address: None,
        name: None,
        reason: Some(reason),
    };
    let device_id = keyed_id(xpath, "DeviceInstance");
    let line_id = keyed_id(xpath, "Line");
    if let Some(source_id) = device_id {
        let matches: Vec<_> = project
            .devices
            .iter()
            .filter(|d| d.source.ets_id == source_id)
            .collect();
        return match matches.as_slice() {
            [device] => RecordOwner {
                kind: OwnerKind::Device,
                id: Some(device.id.0),
                address: device.address.map(|a| a.to_string()),
                name: Some(device.name.clone()),
                reason: None,
            },
            [] => unattributed(UnattributedReason::NoMatch),
            _ => unattributed(UnattributedReason::SeveralMatches),
        };
    }
    let Some(source_id) = line_id else {
        return unattributed(UnattributedReason::NotKeyed);
    };
    let matches: Vec<_> = project
        .installations
        .iter()
        .flat_map(|installation| {
            let topology = &installation.topology;
            topology
                .lines
                .iter()
                .filter(|l| l.source.ets_id == source_id)
                .map(move |line| {
                    let area = topology.areas.iter().find(|a| a.lines.contains(&line.id));
                    (line, area)
                })
        })
        .collect();
    match matches.as_slice() {
        [(line, area)] => RecordOwner {
            kind: OwnerKind::Line,
            id: Some(line.id.0),
            address: area.map(|a| format!("{}.{}", a.address, line.address)),
            name: Some(line.name.clone()),
            reason: None,
        },
        [] => unattributed(UnattributedReason::NoMatch),
        _ => unattributed(UnattributedReason::SeveralMatches),
    }
}

/// The id in the last `<element>[@Id='…']` step, if any.
fn keyed_id<'a>(xpath: &'a str, element: &str) -> Option<&'a str> {
    let marker = format!("{element}[@Id='");
    let start = xpath.rfind(&marker)? + marker.len();
    let len = xpath[start..].find("']")?;
    Some(&xpath[start..start + len])
}
