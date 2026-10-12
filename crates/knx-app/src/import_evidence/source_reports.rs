//! Explicit access to already-retained selective-source report records.

use knx_store::StoredOpaqueEntry;
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};

pub const MAX_SOURCE_REPORTS: usize = 256;
pub const MAX_SOURCE_REPORT_BYTES: usize = 64 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SourceReportSummary {
    pub id: String,
    /// Closed producer vocabulary: recorded or unavailable, never a compatibility pass.
    pub status: String,
    pub source_name: Option<String>,
    pub bytes: Option<usize>,
    pub error_rows: Option<usize>,
    pub warning_rows: Option<usize>,
}

pub struct SourceReport {
    pub report_json: String,
}

fn source_id<'a>(entry: &'a StoredOpaqueEntry, suffix: &str) -> Result<&'a str, String> {
    if !entry.xpath.is_empty() || !entry.name.is_empty() {
        return Err("unsupported selective-source report identity".into());
    }
    let id = entry
        .source_path
        .strip_prefix("imports/")
        .and_then(|path| path.strip_suffix(suffix))
        .ok_or("unsupported selective-source report path")?;
    validate_id(id)?;
    Ok(id)
}

fn validate_id(id: &str) -> Result<(), String> {
    if id.len() != 64
        || !id
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err(
            "invalid selective-source identity; expected exact lowercase SHA-256 scope".into(),
        );
    }
    Ok(())
}

fn reports(
    entries: &[StoredOpaqueEntry],
) -> Result<(BTreeSet<&str>, BTreeMap<&str, &StoredOpaqueEntry>), String> {
    let mut sources = BTreeSet::new();
    let mut archives = BTreeSet::new();
    let mut reports = BTreeMap::new();
    let mut total = 0usize;
    for entry in entries {
        let id = match entry.kind.as_str() {
            "SelectiveImportArchive" => {
                let id = source_id(entry, "/source.knxproj")?;
                if !archives.insert(id) {
                    return Err("duplicate selective-source archive identity".into());
                }
                id
            }
            "SelectiveImportReport" => {
                let id = source_id(entry, "/import-report.json")?;
                if reports.insert(id, entry).is_some() {
                    return Err("duplicate selective-source report identity".into());
                }
                if entry.bytes.len() > super::MAX_EVIDENCE_BYTES {
                    return Err("selective-source report exceeds 16 MiB".into());
                }
                total = total
                    .checked_add(entry.bytes.len())
                    .ok_or("source report catalogue size overflow")?;
                if total > MAX_SOURCE_REPORT_BYTES {
                    return Err("source report catalogue exceeds 64 MiB".into());
                }
                id
            }
            kind if kind.starts_with("SelectiveImportReport")
                || kind.starts_with("SelectiveImportArchive") =>
            {
                return Err("unsupported selective-source evidence kind".into());
            }
            _ => continue,
        };
        sources.insert(id);
        if sources.len() > MAX_SOURCE_REPORTS {
            return Err("source report catalogue exceeds 256 sources".into());
        }
    }
    Ok((sources, reports))
}

fn admitted(entry: &StoredOpaqueEntry) -> Result<(serde_json::Value, (usize, usize)), String> {
    if knx_store::project_history::image_hash(&entry.bytes) != entry.sha256 {
        return Err("selective-source report SHA-256 mismatch".into());
    }
    let original = std::str::from_utf8(&entry.bytes)
        .map_err(|_| "invalid selective-source report encoding")?;
    let report: serde_json::Value =
        serde_json::from_str(original).map_err(|_| "invalid selective-source report JSON")?;
    let counts = super::report_counts(&report, original)?;
    Ok((report, counts))
}

/// Return complete admitted summaries only; no raw bodies or partial-success pages.
/// Scope labels are recorded claims, not proof of the original archive's authenticity.
pub fn catalogue(entries: &[StoredOpaqueEntry]) -> Result<Vec<SourceReportSummary>, String> {
    let (sources, reports) = reports(entries)?;
    let mut catalogue = Vec::with_capacity(sources.len());
    for id in sources {
        match reports.get(id) {
            Some(entry) => {
                let (report, (errors, warnings)) = admitted(entry)?;
                catalogue.push(SourceReportSummary {
                    id: id.into(),
                    status: "recorded".into(),
                    source_name: report["source"]["file_name"].as_str().map(str::to_owned),
                    bytes: Some(entry.bytes.len()),
                    error_rows: Some(errors),
                    warning_rows: Some(warnings),
                });
            }
            None => catalogue.push(SourceReportSummary {
                id: id.into(),
                status: "unavailable".into(),
                source_name: None,
                bytes: None,
                error_rows: None,
                warning_rows: None,
            }),
        }
    }
    Ok(catalogue)
}

/// Read exactly the selected retained report without reinterpretation or source execution.
pub fn read(entries: &[StoredOpaqueEntry], id: &str) -> Result<SourceReport, String> {
    validate_id(id)?;
    // Validate the complete inventory before serving one selected record.
    catalogue(entries)?;
    let (_, reports) = reports(entries)?;
    let entry = reports
        .get(id)
        .ok_or("recorded report unavailable for this source scope")?;
    let report_json = std::str::from_utf8(&entry.bytes)
        .map_err(|_| "invalid selective-source report encoding")?
        .to_owned();
    Ok(SourceReport { report_json })
}
