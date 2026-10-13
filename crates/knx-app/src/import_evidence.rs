//! Versioned historical import evidence interpreted only by the application layer.

pub mod source_reports;

use knx_store::StoredOpaqueEntry;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::io::Write;

const KIND: &str = "KnxBenchImportEvidence";
const SOURCE_PATH: &str = "knxbench-native/import-report.json";
const NAME: &str = "import-report.json";
const FORMAT: &str = "knxbench-native-import-evidence";
pub const MAX_EVIDENCE_BYTES: usize = 16 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq)]
pub struct HistoricalImportReport {
    pub report: Value,
    pub report_json: String,
    counts: (usize, usize),
}

impl HistoricalImportReport {
    /// Same units as the original importer: category rows, not unique losses.
    pub fn counts(&self) -> (usize, usize) {
        self.counts
    }
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Envelope {
    format: String,
    version: u32,
    producer: String,
    report_json: String,
}

#[derive(Deserialize)]
struct SeverityWitness {
    severity: String,
}

#[derive(Deserialize)]
struct CounterWitness {
    errors: Vec<SeverityWitness>,
    unknown: Vec<serde::de::IgnoredAny>,
    conflicts: Vec<serde::de::IgnoredAny>,
    unsupported: Vec<UnsupportedWitness>,
}

/// Only what the shared counting rule reads (`import_finding_counts`). Both
/// fields are optional so a row the rule cannot group still counts once;
/// a duplicate or non-string value is ambiguous and refused like the rest.
#[derive(Deserialize)]
struct UnsupportedWitness {
    #[serde(default)]
    what: Option<String>,
    #[serde(default)]
    consequence: Option<String>,
}

struct BoundedBytes(Vec<u8>);
impl Write for BoundedBytes {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        if self
            .0
            .len()
            .checked_add(bytes.len())
            .is_none_or(|len| len > MAX_EVIDENCE_BYTES)
        {
            return Err(std::io::Error::other(
                "native import evidence exceeds 16 MiB",
            ));
        }
        self.0.extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
fn bounded_json(value: &impl Serialize) -> Result<Vec<u8>, String> {
    let mut out = BoundedBytes(Vec::new());
    serde_json::to_writer(&mut out, value)
        .map_err(|_| "native import evidence serialization refused (16 MiB budget)".to_string())?;
    Ok(out.0)
}

pub fn encode(report: &knx_etsproj::ImportReport) -> Result<StoredOpaqueEntry, String> {
    let report_json = String::from_utf8(bounded_json(report)?)
        .map_err(|_| "invalid serialized import evidence".to_string())?;
    let envelope = Envelope {
        format: FORMAT.into(),
        version: 1,
        producer: concat!("KNXBench knx-app/", env!("CARGO_PKG_VERSION")).into(),
        report_json,
    };
    let bytes = bounded_json(&envelope)?;
    let entry = StoredOpaqueEntry {
        source_path: SOURCE_PATH.into(),
        xpath: String::new(),
        kind: KIND.into(),
        name: NAME.into(),
        sha256: knx_store::project_history::image_hash(&bytes),
        bytes,
    };
    // Admission uses the same decoder as native opening, before state publication.
    read(std::slice::from_ref(&entry))?;
    Ok(entry)
}

/// Attach primary historical evidence before any destination is opened.
pub fn capture_primary(
    entries: &mut Vec<StoredOpaqueEntry>,
    report: &knx_etsproj::ImportReport,
) -> Result<(), String> {
    if contains_primary(entries) {
        return Err(
            "primary import evidence already attached; explicit replacement required".into(),
        );
    }
    let entry = encode(report)?;
    entries.push(entry);
    Ok(())
}

/// Identity lookup only; callers must admit the envelope with `read` first.
pub fn contains_primary(entries: &[StoredOpaqueEntry]) -> bool {
    entries
        .iter()
        .any(|entry| entry.source_path == SOURCE_PATH && entry.kind.starts_with(KIND))
}

/// Missing legacy/scoped source evidence is unavailable, never a measured zero.
/// Reserved primary evidence is read-only and must be admitted completely.
pub fn read(entries: &[StoredOpaqueEntry]) -> Result<Option<HistoricalImportReport>, String> {
    let mut primary = entries
        .iter()
        .filter(|entry| entry.source_path == SOURCE_PATH && entry.kind.starts_with(KIND));
    let Some(entry) = primary.next() else {
        return Ok(None);
    };
    if primary.next().is_some() {
        return Err("duplicate primary native import evidence".into());
    }
    if entry.kind != KIND || entry.name != NAME || !entry.xpath.is_empty() {
        return Err("unsupported native import evidence identity".into());
    }
    if entry.bytes.len() > MAX_EVIDENCE_BYTES {
        return Err("native import evidence exceeds 16 MiB".into());
    }
    if knx_store::project_history::image_hash(&entry.bytes) != entry.sha256 {
        return Err("native import evidence SHA-256 mismatch".into());
    }
    let envelope: Envelope = serde_json::from_slice(&entry.bytes)
        .map_err(|_| "invalid native import evidence envelope".to_string())?;
    if envelope.format != FORMAT || envelope.version != 1 || envelope.producer.is_empty() {
        return Err("unsupported native import evidence version or producer".into());
    }
    let report: Value = serde_json::from_str(&envelope.report_json)
        .map_err(|_| "invalid historical import report JSON".to_string())?;
    let counts = report_counts(&report, &envelope.report_json)?;
    Ok(Some(HistoricalImportReport {
        report,
        report_json: envelope.report_json,
        counts,
    }))
}

fn report_counts(report: &Value, original_json: &str) -> Result<(usize, usize), String> {
    let invalid = || "invalid historical import report shape".to_string();
    let root = report.as_object().ok_or_else(invalid)?;
    let source = root
        .get("source")
        .and_then(Value::as_object)
        .ok_or_else(invalid)?;
    for key in ["file_name", "namespace"] {
        if !source.get(key).is_some_and(Value::is_string) {
            return Err(invalid());
        }
    }
    if source.get("file_size").and_then(Value::as_u64).is_none()
        || !source
            .get("schema_version")
            .and_then(Value::as_u64)
            .is_some_and(|v| u32::try_from(v).is_ok())
    {
        return Err(invalid());
    }
    let rows = root
        .get("counts")
        .and_then(|v| v.get("rows"))
        .and_then(Value::as_array)
        .ok_or_else(invalid)?;
    for row in rows {
        if !row.get("entity").is_some_and(Value::is_string)
            || !["read", "mapped"].iter().all(|key| {
                row.get(key)
                    .and_then(Value::as_u64)
                    .is_some_and(|v| u32::try_from(v).is_ok())
            })
        {
            return Err(invalid());
        }
    }
    for key in [
        "unknown",
        "opaque",
        "inferred",
        "conflicts",
        "unsupported",
        "errors",
    ] {
        let rows = root
            .get(key)
            .and_then(Value::as_array)
            .ok_or_else(invalid)?;
        if !rows.iter().all(Value::is_object) {
            return Err(invalid());
        }
    }
    let errors = root["errors"].as_array().ok_or_else(invalid)?;
    for item in errors {
        if !["stage", "xpath", "detail"]
            .iter()
            .all(|key| item.get(key).is_some_and(Value::is_string))
        {
            return Err(invalid());
        }
    }
    // Typed fields reject duplicate interpreted counters and duplicate severity
    // keys; Value alone would silently keep the last occurrence.
    let counters: CounterWitness = serde_json::from_str(original_json)
        .map_err(|_| "ambiguous or invalid historical import counters".to_string())?;
    let mut error_count = 0usize;
    let mut warning_count = 0usize;
    for item in counters.errors {
        match item.severity.as_str() {
            "error" => error_count += 1,
            "warning" => warning_count += 1,
            _ => return Err(invalid()),
        }
    }
    // The same rule as a fresh import (`import_finding_counts`), so a
    // reopened project shows the counts it showed when it was imported.
    let unsupported = crate::import_payload_evidence::unsupported_warning_rows(
        counters.unsupported.iter().map(|row| {
            (
                row.what.as_deref().unwrap_or_default(),
                row.consequence.as_deref().unwrap_or_default(),
            )
        }),
    );
    for len in [
        counters.unknown.len(),
        counters.conflicts.len(),
        unsupported,
    ] {
        warning_count = warning_count.checked_add(len).ok_or_else(invalid)?;
    }
    Ok((error_count, warning_count))
}
