//! Diagnostic bundle: problem-type defaults, section preview, manifest and ZIP (DIAG-03).
//!
//! One ZIP a user may mail to the project. Front ends collect sections
//! (environment, session log, operation recordings, device extract, ...);
//! this module checks them against the problem type's offer (owner table
//! O1), previews them, and writes the ZIP with a `manifest.json` that lists
//! every file with its size and SHA-256. It reads no files and opens no
//! connection: secrets must already be absent from the section bytes
//! (recordings redact at the tap; project copies use `secret_free_copy`).

use std::collections::BTreeSet;
use std::io::{Cursor, Write};

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

use crate::operation_recording::WriterIdentity;

mod retained_admission;
pub use retained_admission::{
    admit_retained_diagnostics, admit_retained_support, RetainedAdmissionUnavailable,
    RetainedBundleFiles,
};

/// Value of the manifest's `format` field.
pub const BUNDLE_FORMAT: &str = "knxbench-diagnostic-bundle";
/// Version of the bundle layout this build writes.
pub const BUNDLE_FORMAT_VERSION: u32 = 1;
/// The project's report mailbox (the same as ADR-0121's).
pub const REPORT_MAILBOX: &str = "issue@knxbench.com";
/// Above this total size the wizard warns about common mail limits.
pub const MAIL_SIZE_HINT_BYTES: u64 = 10 * 1024 * 1024;

/// What kind of problem the user reports (owner decision 16).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ProblemType {
    DeviceProgramming,
    DeviceReading,
    ProjectImport,
    Catalogue,
    UserInterface,
}

impl ProblemType {
    pub const ALL: [ProblemType; 5] = [
        ProblemType::DeviceProgramming,
        ProblemType::DeviceReading,
        ProblemType::ProjectImport,
        ProblemType::Catalogue,
        ProblemType::UserInterface,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            ProblemType::DeviceProgramming => "deviceProgramming",
            ProblemType::DeviceReading => "deviceReading",
            ProblemType::ProjectImport => "projectImport",
            ProblemType::Catalogue => "catalogue",
            ProblemType::UserInterface => "userInterface",
        }
    }
}

/// One kind of bundle section (the rows of owner table O1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SectionKind {
    /// Version, commit, OS, settings, recording state. Always included.
    Environment,
    SessionLog,
    /// The operation the report is about: recording plus typed results.
    Operation,
    /// Further operations the user picks.
    EarlierOperation,
    /// The affected device: application, parameter values, linked addresses.
    Device,
    /// The minimised reports that existed before this bundle.
    LegacyReports,
    ImportReport,
    /// The original import file, secret material removed.
    ImportFile,
    Catalogue,
    /// The whole project, secret material removed.
    Project,
    BusMonitor,
    ClientState,
}

impl SectionKind {
    pub const ALL: [SectionKind; 12] = [
        SectionKind::Environment,
        SectionKind::SessionLog,
        SectionKind::Operation,
        SectionKind::EarlierOperation,
        SectionKind::Device,
        SectionKind::LegacyReports,
        SectionKind::ImportReport,
        SectionKind::ImportFile,
        SectionKind::Catalogue,
        SectionKind::Project,
        SectionKind::BusMonitor,
        SectionKind::ClientState,
    ];

    /// The directory of this section's files inside the ZIP.
    pub fn directory(self) -> &'static str {
        match self {
            SectionKind::Environment => "environment",
            SectionKind::SessionLog => "session-log",
            SectionKind::Operation => "operations",
            SectionKind::EarlierOperation => "operations",
            SectionKind::Device => "device",
            SectionKind::LegacyReports => "reports",
            SectionKind::ImportReport => "import",
            SectionKind::ImportFile => "import",
            SectionKind::Catalogue => "catalogue",
            SectionKind::Project => "project",
            SectionKind::BusMonitor => "bus-monitor",
            SectionKind::ClientState => "client",
        }
    }
}

/// Whether the wizard offers a section for a problem type, and how.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Offer {
    /// Always included; the user cannot remove it.
    Fixed,
    /// Offered and ticked by default.
    Preselected,
    /// Offered, not ticked.
    Offered,
    /// Not offered for this problem type.
    NotOffered,
}

/// Owner table O1, verbatim: the default of `kind` for `problem`.
pub fn default_offer(problem: ProblemType, kind: SectionKind) -> Offer {
    use Offer::{Fixed as F, NotOffered as N, Offered as O, Preselected as P};
    use ProblemType as T;
    // Columns: programming, reading, import, catalogue, UI/other.
    let row: [Offer; 5] = match kind {
        SectionKind::Environment => [F, F, F, F, F],
        SectionKind::SessionLog => [P, P, P, P, P],
        SectionKind::Operation => [P, P, N, N, O],
        SectionKind::EarlierOperation => [O, O, N, N, O],
        SectionKind::Device => [P, P, N, O, O],
        SectionKind::LegacyReports => [P, P, N, N, N],
        SectionKind::ImportReport => [N, N, P, N, O],
        SectionKind::ImportFile => [N, N, O, N, N],
        SectionKind::Catalogue => [O, O, N, P, N],
        SectionKind::Project => [O, O, O, N, O],
        SectionKind::BusMonitor => [O, O, N, N, O],
        SectionKind::ClientState => [N, N, N, N, P],
    };
    row[match problem {
        T::DeviceProgramming => 0,
        T::DeviceReading => 1,
        T::ProjectImport => 2,
        T::Catalogue => 3,
        T::UserInterface => 4,
    }]
}

/// What kinds of personal or project data a section contains (decision 17).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContentMarkers {
    pub addresses: bool,
    pub names: bool,
    pub parameter_values: bool,
    /// Gateway endpoints, host network addresses.
    pub network_addresses: bool,
}

/// One file of a section, its path relative to the section directory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BundleFile {
    pub name: String,
    pub bytes: Vec<u8>,
}

impl BundleFile {
    pub fn new(name: impl Into<String>, bytes: Vec<u8>) -> Self {
        Self {
            name: name.into(),
            bytes,
        }
    }

    pub fn json(name: impl Into<String>, value: &Value) -> Self {
        Self::new(
            name,
            serde_json::to_vec_pretty(value).expect("a JSON value serializes"),
        )
    }
}

/// One collected section, ready for preview and export.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BundleSection {
    pub kind: SectionKind,
    /// Distinguishes several sections of one kind, e.g. the operation id.
    pub label: Option<String>,
    pub files: Vec<BundleFile>,
    /// Entries a reader would count: log lines, telegrams, parameters.
    pub entries: usize,
    pub markers: ContentMarkers,
    /// What was removed or left out for privacy or secrecy, without values.
    pub notes: Vec<String>,
}

impl BundleSection {
    pub fn new(kind: SectionKind) -> Self {
        Self {
            kind,
            label: None,
            files: Vec::new(),
            entries: 0,
            markers: ContentMarkers::default(),
            notes: Vec::new(),
        }
    }

    pub fn size(&self) -> u64 {
        self.files.iter().map(|f| f.bytes.len() as u64).sum()
    }

    /// The section's key in previews and selections: kind, then label.
    pub fn key(&self) -> String {
        match &self.label {
            Some(label) => format!("{}:{label}", self.kind_name()),
            None => self.kind_name(),
        }
    }

    fn kind_name(&self) -> String {
        serde_json::to_value(self.kind)
            .ok()
            .and_then(|v| v.as_str().map(str::to_string))
            .unwrap_or_default()
    }

    /// The directory of this section's files inside the ZIP.
    pub fn directory(&self) -> String {
        match &self.label {
            Some(label) => format!("{}/{label}", self.kind.directory()),
            None => self.kind.directory().to_string(),
        }
    }
}

/// What the wizard shows per section in its preview step.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SectionPreview {
    pub key: String,
    pub kind: SectionKind,
    pub label: Option<String>,
    pub bytes: u64,
    pub entries: usize,
    pub markers: ContentMarkers,
    pub notes: Vec<String>,
    pub files: Vec<String>,
}

pub fn preview(section: &BundleSection) -> SectionPreview {
    SectionPreview {
        key: section.key(),
        kind: section.kind,
        label: section.label.clone(),
        bytes: section.size(),
        entries: section.entries,
        markers: section.markers,
        notes: section.notes.clone(),
        files: section.files.iter().map(|f| f.name.clone()).collect(),
    }
}

/// The section as text for the preview's "show content": UTF-8 files in
/// full, anything else as a size line.
pub fn section_text(section: &BundleSection) -> String {
    let mut out = String::new();
    for file in &section.files {
        out.push_str(&format!(
            "===== {}/{} =====\n",
            section.directory(),
            file.name
        ));
        match std::str::from_utf8(&file.bytes) {
            Ok(text) => out.push_str(text),
            Err(_) => out.push_str(&format!("<{} octets, not text>", file.bytes.len())),
        }
        if !out.ends_with('\n') {
            out.push('\n');
        }
    }
    out
}

/// Facts the manifest records about the export itself.
#[derive(Debug, Clone)]
pub struct BundleRequest {
    pub problem: ProblemType,
    pub writer: WriterIdentity,
    /// `false` when the operator disabled recording; said plainly.
    pub recording_enabled: bool,
    /// When the user ticked the consent box, RFC 3339.
    pub consent_at: String,
    /// The user's own description, if any.
    pub description: Option<String>,
}

/// Why a bundle was refused; nothing is written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BundleError {
    /// The environment section is fixed and must be present exactly once.
    MissingEnvironment,
    /// A section this problem type does not offer.
    NotOffered(SectionKind),
    /// Two sections share a key, or two files a path.
    Duplicate(String),
    /// A label or file name that is not a plain relative name.
    BadName(String),
    Zip(String),
}

impl std::fmt::Display for BundleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BundleError::MissingEnvironment => f.write_str("the environment section is missing"),
            BundleError::NotOffered(kind) => {
                write!(f, "section {kind:?} is not offered for this problem type")
            }
            BundleError::Duplicate(what) => write!(f, "duplicate section or file: {what}"),
            BundleError::BadName(name) => write!(f, "not a plain relative name: {name:?}"),
            BundleError::Zip(reason) => write!(f, "the ZIP could not be written: {reason}"),
        }
    }
}

impl std::error::Error for BundleError {}

/// A name segment the ZIP may carry: no separators, no dot-only names.
fn plain(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 120
        && name != "."
        && name != ".."
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
}

fn check(problem: ProblemType, sections: &[BundleSection]) -> Result<(), BundleError> {
    let environments = sections
        .iter()
        .filter(|s| s.kind == SectionKind::Environment)
        .count();
    if environments != 1 {
        return Err(BundleError::MissingEnvironment);
    }
    let mut keys = BTreeSet::new();
    let mut paths = BTreeSet::new();
    for section in sections {
        if default_offer(problem, section.kind) == Offer::NotOffered {
            return Err(BundleError::NotOffered(section.kind));
        }
        if let Some(label) = &section.label {
            if !plain(label) {
                return Err(BundleError::BadName(label.clone()));
            }
        }
        if !keys.insert(section.key()) {
            return Err(BundleError::Duplicate(section.key()));
        }
        for file in &section.files {
            if !plain(&file.name) {
                return Err(BundleError::BadName(file.name.clone()));
            }
            let path = format!("{}/{}", section.directory(), file.name);
            if path == "manifest.json" || !paths.insert(path.clone()) {
                return Err(BundleError::Duplicate(path));
            }
        }
    }
    Ok(())
}

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// The manifest of `sections` for `request`, as written into the ZIP.
pub fn manifest(request: &BundleRequest, sections: &[BundleSection], created_at: &str) -> Value {
    let listed: Vec<Value> = sections
        .iter()
        .map(|section| {
            json!({
                "key": section.key(),
                "kind": section.kind,
                "label": section.label,
                "entries": section.entries,
                "markers": section.markers,
                "notes": section.notes,
                "files": section.files.iter().map(|file| json!({
                    "path": format!("{}/{}", section.directory(), file.name),
                    "bytes": file.bytes.len(),
                    "sha256": sha256_hex(&file.bytes),
                })).collect::<Vec<_>>(),
            })
        })
        .collect();
    json!({
        "format": BUNDLE_FORMAT,
        "formatVersion": BUNDLE_FORMAT_VERSION,
        "createdAt": created_at,
        "writer": request.writer,
        "problemType": request.problem,
        "consentAt": request.consent_at,
        "description": request.description,
        "recording": {
            "enabled": request.recording_enabled,
            "statement": if request.recording_enabled {
                "device operations are recorded on this installation"
            } else {
                "the operator disabled recording of device operations on this installation; no new recordings exist"
            },
        },
        "redaction": {
            "alwaysRemoved": [
                "authorize and key-write keys (BCU keys)",
                "KNX Secure keys and tool keys",
                "tunnel and login passwords",
            ],
            "marker": "removed octets are zero and named by a `redacted` object; removed values are blank",
            "notRemoved": "individual and group addresses, names, parameter values, gateway endpoints and, in raw recordings and declared memory, device configuration that may contain passwords are included as selected; keep the file private",
        },
        "totalBytes": sections.iter().map(BundleSection::size).sum::<u64>(),
        "sections": listed,
    })
}

/// Writes the bundle ZIP. Refuses sections the problem type does not offer,
/// a missing environment section, duplicates and unsafe names.
pub fn build_zip(
    request: &BundleRequest,
    sections: &[BundleSection],
    created_at: &str,
) -> Result<Vec<u8>, BundleError> {
    check(request.problem, sections)?;
    let manifest = manifest(request, sections, created_at);
    let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated)
        .unix_permissions(0o644);
    let zip_error = |e: zip::result::ZipError| BundleError::Zip(e.to_string());
    let io_error = |e: std::io::Error| BundleError::Zip(e.to_string());
    zip.start_file("manifest.json", options)
        .map_err(zip_error)?;
    zip.write_all(&serde_json::to_vec_pretty(&manifest).expect("manifest serializes"))
        .map_err(io_error)?;
    for section in sections {
        for file in &section.files {
            zip.start_file(format!("{}/{}", section.directory(), file.name), options)
                .map_err(zip_error)?;
            zip.write_all(&file.bytes).map_err(io_error)?;
        }
    }
    Ok(zip.finish().map_err(zip_error)?.into_inner())
}

/// A ZIP file name for the bundle: version and, when known, operation id.
pub fn file_name(version: &str, operation_id: Option<&str>, created_at: &str) -> String {
    let safe = |text: &str| -> String {
        text.chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() || c == '-' {
                    c
                } else {
                    '-'
                }
            })
            .collect()
    };
    let stamp: String = created_at
        .chars()
        .filter(|c| c.is_ascii_digit())
        .take(14)
        .collect();
    match operation_id {
        Some(id) => format!("knxbench-diagnostics-{}-{}.zip", safe(version), safe(id)),
        None => format!("knxbench-diagnostics-{}-{stamp}.zip", safe(version)),
    }
}

fn percent_encode(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for byte in text.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(byte as char)
            }
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

/// The consent note on every section that carries declared memory or a raw
/// non-System-2 recording (owner decision 31 October 2026). Server and CLI
/// emit exactly this text; the web translates it by exact match.
pub const PRIVATE_RAW_NOTE: &str = "contains device configuration, possibly passwords; \
     recognised KNX keys are redacted; keep private";

/// The note on a raw recording whose last line was cut off by an interrupted write.
pub const TORN_TAIL_NOTE: &str = "the recording's last line is incomplete (interrupted write)";

/// Largest raw recording copied into a bundle by server or CLI; a larger one is left out.
pub const MAX_RAW_RECORDING_BYTES: u64 = 64 * 1024 * 1024;

/// A prefilled mail draft (decision 6): subject with version and operation
/// id, a body asking to attach the saved ZIP. It never carries the bundle,
/// a section or any value from it.
pub fn mail_draft(
    version: &str,
    problem: ProblemType,
    operation_id: Option<&str>,
    file_name: &str,
) -> String {
    let subject = match operation_id {
        Some(id) => format!(
            "KNXBench {version} diagnostic bundle ({}, {id})",
            problem.as_str()
        ),
        None => format!(
            "KNXBench {version} diagnostic bundle ({})",
            problem.as_str()
        ),
    };
    let body = format!(
        "Please attach the saved file {file_name} to this mail.\n\n\
         What happened (optional):\n\n\n\
         Note: email is not encrypted. The bundle may contain addresses, names, \
         parameter values and device configuration with possible passwords; \
         recognised KNX keys are redacted. Keep it private."
    );
    format!(
        "mailto:{REPORT_MAILBOX}?subject={}&body={}",
        percent_encode(&subject),
        percent_encode(&body)
    )
}

#[cfg(test)]
#[path = "diagnostic_bundle_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "diagnostic_bundle/retained_admission_tests.rs"]
mod retained_admission_tests;
