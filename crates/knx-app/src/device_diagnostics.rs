//! Shared read-only device diagnostic service for HTTP and CLI.
//!
//! No download admission, source
//! images or private research are consulted. Public archives contain only the
//! closed acquisition allowlist, never a project or arbitrary error strings.
//! Format 1 is the identity recipe; format 2 (ADR-0143) adds `memory.json`
//! with the ProductDB-declared candidate ranges of `system2-declared-segments-v1`.
mod central_directory;
mod memory_evidence;

use chrono::{DateTime, Utc};
use knx_core::{ContactableAddress, DeviceId, Project};
use knx_net::commissioning::diagnostic_export::{self as net, Identity, Item, Query};
use knx_net::{ManagementTransport, SessionTiming};
pub use memory_evidence::{
    acquire_memory, pending_memory, prepare_memory, ChunkReading, Comparison, Coverage,
    MemoryEvidence, MemoryPrepared, RangeEvidence, MEMORY_PROFILE,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs::OpenOptions;
use std::io::{Cursor, Read, Seek, SeekFrom, Write};
use std::path::Path;
use std::sync::atomic::AtomicBool;

/// One nonblocking process lease for participating diagnostic callers. Existing
/// server tunnel locks additionally exclude every in-server bus operation.
/// External clients do not necessarily cooperate; never reconnect/force a tunnel.
pub fn lease_in(directory: &Path) -> Result<std::fs::File, String> {
    std::fs::create_dir_all(directory).map_err(|_| "diagnostics-lease-directory")?;
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(directory.join("knx-gateway-tunnel.lock"))
        .map_err(|_| "diagnostics-lease-file")?;
    file.try_lock().map_err(|_| "diagnostics-gateway-busy")?;
    Ok(file)
}
/// Stable per-user runtime location, independent of worktree and TMPDIR.
pub fn gateway_lease() -> Result<std::fs::File, String> {
    let directory = if let Some(runtime) = std::env::var_os("XDG_RUNTIME_DIR") {
        std::path::PathBuf::from(runtime).join("knxbench")
    } else {
        std::path::PathBuf::from(std::env::var_os("HOME").ok_or("diagnostics-runtime-unavailable")?)
            .join(".cache/knxbench")
    };
    lease_in(&directory)
}

pub const PROFILE: &str = "generic-identity-v1";
/// The stop reason a caller records when its own tunnel teardown is not
/// confirmed and the acquisition itself had none.
pub const TUNNEL_DISCONNECT_UNCONFIRMED: &str = "tunnel-disconnect-unconfirmed";
const MAX_ARCHIVE: usize = 256 * 1024;
const MAX_MEMBER: u64 = 64 * 1024;
/// `memory.json` of the largest admitted profile (2048 octets, at most 178
/// chunks with timestamps) is well below this; the archive limit still holds.
const MAX_MEMORY_MEMBER: u64 = 192 * 1024;
const KIND: &str = "knxbench-device-diagnostics";
const README: &str = "KNXBench read-only device diagnostics, format 1.\nOnly descriptor type 0 and Device Object properties 12/78 are requested.\nNo configuration memory, keys, serial number, project or gateway metadata.\nA property refusal cannot distinguish missing rights from a missing service.\nMask/manufacturer agreement is not product/application identity.\nNot a recovery backup or programming approval. No complete anonymity is promised.\nSIMULATED is recorded in report.json and manifest.json when using a simulator.\nNo automatic upload. Review the files, then send the ZIP to issue@knxbench.com or share a file link privately.\nChecksums detect corruption, not maliciously rewritten evidence.\n";
#[derive(Debug, Clone)]
pub struct Prepared {
    pub target: ContactableAddress,
    pub expected: Identity,
    /// Opaque selection reference. The UI/CLI shows its local device mapping;
    /// the shared package does not reveal addresses or low-entropy name hashes.
    pub reference: String,
}
/// Exactly one selected project device; unavailable catalogue data is unknown,
/// not a prerequisite for safely attempting the generic identity recipe.
pub fn prepare(
    project: &Project,
    device: DeviceId,
    db: Option<&knx_productdb::Connection>,
) -> Result<Prepared, String> {
    let d = project
        .devices
        .iter()
        .find(|d| d.id == device)
        .ok_or("diagnostics-device-missing")?;
    let address = d.address.ok_or("diagnostics-address-missing")?;
    let target = ContactableAddress::new(address).map_err(|_| "diagnostics-target-excluded")?;
    if project
        .devices
        .iter()
        .filter(|d| d.address == Some(address))
        .count()
        != 1
    {
        return Err("diagnostics-address-ambiguous".into());
    }
    let mut expected = Identity::default();
    if let Some(db) = db {
        if let Ok(Some(id)) = knx_productdb::query::resolve_program(db, &d.program_ref) {
            let row: Result<(Option<String>, String), _> = db.query_row(
                "SELECT mask_version, manufacturer_id FROM application_program WHERE id=?1",
                [id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            );
            if let Ok((mask, manufacturer)) = row {
                expected.mask = mask
                    .as_deref()
                    .and_then(|s| s.strip_prefix("MV-"))
                    .and_then(hex16);
                expected.manufacturer = manufacturer.strip_prefix("M-").and_then(hex16);
            }
        }
    }
    let mut random = [0u8; 32];
    getrandom::fill(&mut random).map_err(|_| "diagnostics-reference-unavailable")?;
    Ok(Prepared {
        target,
        expected,
        reference: hex(&random),
    })
}
/// Errors and stop reasons are fixed code tokens, never free text that could
/// carry a path, name or address into a shared archive.
pub(crate) fn is_token(s: &str) -> bool {
    (1..=64).contains(&s.len()) && s.bytes().all(|b| b.is_ascii_lowercase() || b == b'-')
}
fn hex16(s: &str) -> Option<u16> {
    (s.len() == 4)
        .then(|| u16::from_str_radix(s, 16).ok())
        .flatten()
}
pub(crate) fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Reading {
    pub order: usize,
    pub query: String,
    pub object_index: Option<u8>,
    pub property_id: Option<u8>,
    pub start_index: Option<u16>,
    pub requested_elements: Option<u8>,
    pub expected_bytes: usize,
    pub status: String,
    pub received_bytes: usize,
    pub returned_elements: Option<u8>,
    pub bytes: Vec<u8>,
    pub attempted_at: Option<String>,
    pub completed_at: Option<String>,
    pub error: Option<String>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Report {
    pub kind: String,
    pub format: u32,
    pub build: String,
    pub collected_at: String,
    pub device: String,
    pub project_reference: String,
    pub profile: String,
    pub asserted_mask: Option<u16>,
    pub asserted_manufacturer: Option<u16>,
    pub simulated: bool,
    pub written: bool,
    pub complete: bool,
    pub items: Vec<Reading>,
    pub stop_reason: Option<String>,
    pub memory_status: String,
    pub memory_reason: String,
    /// T_Disconnect is best effort and unconfirmed; this records completion of
    /// the attempt only, never claims that a physical device acknowledged it.
    pub disconnect_attempt_completed: bool,
}
pub(crate) fn readings(items: &[Item]) -> Vec<Reading> {
    items
        .iter()
        .enumerate()
        .map(|(i, item)| {
            let property = item.query.property();
            Reading {
                order: i + 1,
                query: item.query.name().into(),
                object_index: property.map(|_| 0),
                property_id: property,
                start_index: property.map(|_| 1),
                requested_elements: property.map(|_| 1),
                expected_bytes: item.query.expected_bytes(),
                status: item.status.name().into(),
                received_bytes: item.received_bytes,
                returned_elements: item.returned_elements,
                bytes: item.bytes.clone(),
                attempted_at: item
                    .attempted_at
                    .map(|t| DateTime::<Utc>::from(t).to_rfc3339()),
                completed_at: item
                    .completed_at
                    .map(|t| DateTime::<Utc>::from(t).to_rfc3339()),
                error: item.error.map(str::to_string),
            }
        })
        .collect()
}
pub fn pending(plan: &Prepared, build: &str, simulated: bool) -> Report {
    Report {kind:KIND.into(),format:1,build:build.into(),collected_at:Utc::now().to_rfc3339(),device:"device-1".into(),
        project_reference:plan.reference.clone(),profile:PROFILE.into(),asserted_mask:plan.expected.mask,asserted_manufacturer:plan.expected.manufacturer,
        simulated,written:false,complete:false,items:readings(&[Query::Descriptor,Query::Manufacturer,Query::HardwareType].map(Item::pending)),stop_reason:None,
        memory_status:"not-requested".into(),memory_reason:"No independently admitted safe memory profile; source offsets are not runtime addresses.".into(),disconnect_attempt_completed:false }
}
/// The one acquisition entry point used by API and CLI. Observer snapshots
/// remain exportable after cancellation/failure. The caller owns tunnel teardown
/// and gateway exclusivity, since it owns the transport's entire lifecycle.
pub async fn acquire<T: ManagementTransport>(
    transport: &T,
    plan: &Prepared,
    timing: SessionTiming,
    build: &str,
    cancel: &AtomicBool,
    mut observe: impl FnMut(&Report),
) -> Report {
    let mut report = pending(
        plan,
        build,
        matches!(
            transport.target_kind(),
            knx_core::commissioning::mutation::TargetKind::Simulator
        ),
    );
    let collected = net::collect(
        transport,
        plan.target,
        timing,
        plan.expected,
        cancel,
        |items| {
            report.items = readings(items);
            observe(&report);
        },
    )
    .await;
    report.items = readings(&collected.items);
    report.stop_reason = collected.stop_reason.map(str::to_string);
    report.disconnect_attempt_completed = collected.disconnect_attempt_completed;
    report.complete = true;
    observe(&report);
    report
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Member {
    bytes: u64,
    sha256: String,
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Manifest {
    kind: String,
    format: u32,
    build: String,
    collected_at: String,
    simulated: bool,
    /// Format 2 only; absent from format 1, which keeps it byte-identical.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    notice: Option<String>,
    files: BTreeMap<String, Member>,
}
/// What each archive format promises about memory.
fn format_matches(r: &Report) -> bool {
    match r.format {
        1 => r.profile == PROFILE && r.memory_status == "not-requested",
        2 => {
            r.profile == MEMORY_PROFILE
                && r.memory_status == "requested"
                && r.memory_reason == memory_evidence::MEMORY_REASON
                && r.asserted_mask.is_some()
                && r.asserted_manufacturer.is_some()
        }
        _ => false,
    }
}
fn validate(r: &Report) -> Result<(), String> {
    if r.kind != KIND
        || !format_matches(r)
        || r.device != "device-1"
        || r.written
        || r.project_reference.len() != 64
        || !r.project_reference.bytes().all(|b| b.is_ascii_hexdigit())
        || r.build.is_empty()
        || r.build.len() > 128
        || !r
            .build
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b".-_+".contains(&b))
        || DateTime::parse_from_rfc3339(&r.collected_at).is_err()
        || r.items.len() != 3
        || !r.stop_reason.as_deref().is_none_or(is_token)
    {
        return Err("diagnostics-invalid-report".into());
    }
    for (i, (item, query)) in r
        .items
        .iter()
        .zip([Query::Descriptor, Query::Manufacturer, Query::HardwareType])
        .enumerate()
    {
        if item.order != i + 1
            || item.query != query.name()
            || item.property_id != query.property()
            || item.object_index != query.property().map(|_| 0)
            || item.start_index != query.property().map(|_| 1)
            || item.requested_elements != query.property().map(|_| 1)
            || item.expected_bytes != query.expected_bytes()
            || item.bytes.len() > net::MAX_RESPONSE_BYTES
            || !item.error.as_deref().is_none_or(is_token)
            || !matches!(
                item.status.as_str(),
                "read" | "partial" | "refused-ambiguous" | "error" | "not-read"
            )
        {
            return Err("diagnostics-invalid-reading".into());
        }
        if item.status == "read"
            && (item.bytes.len() != item.expected_bytes
                || item.received_bytes != item.bytes.len()
                || item.error.is_some())
        {
            return Err("diagnostics-invalid-coverage".into());
        }
        if item.received_bytes != item.bytes.len()
            && item.error.as_deref() != Some("response-size-limit-raw-not-retained")
        {
            return Err("diagnostics-invalid-coverage".into());
        }
        if item.status == "not-read" && (!item.bytes.is_empty() || item.attempted_at.is_some()) {
            return Err("diagnostics-invalid-pending".into());
        }
        for at in [&item.attempted_at, &item.completed_at]
            .into_iter()
            .flatten()
        {
            if DateTime::parse_from_rfc3339(at).is_err() {
                return Err("diagnostics-invalid-time".into());
            }
        }
    }
    Ok(())
}
/// One diagnostic archive: the identity report, plus memory evidence in
/// format 2 only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Export {
    pub report: Report,
    /// `Some` exactly when `report.format` is 2.
    pub memory: Option<MemoryEvidence>,
}
fn validate_export(export: &Export) -> Result<(), String> {
    validate(&export.report)?;
    match (export.report.format, &export.memory) {
        (1, None) => Ok(()),
        (2, Some(memory)) => memory_evidence::validate(&export.report, memory),
        _ => Err("diagnostics-invalid-format".into()),
    }
}
/// Format 1 only; a format-2 export goes through [`bundle_export`].
pub fn bundle(report: &Report) -> Result<Vec<u8>, String> {
    if report.format != 1 {
        return Err("diagnostics-invalid-format".into());
    }
    bundle_export(&Export {
        report: report.clone(),
        memory: None,
    })
}
/// Fixed names and checksummed lengths; no ZIP entry can be a host path.
pub fn bundle_export(export: &Export) -> Result<Vec<u8>, String> {
    validate_export(export)?;
    let report = &export.report;
    let report_bytes = serde_json::to_vec_pretty(report).map_err(|_| "diagnostics-encoding")?;
    let properties =
        serde_json::to_vec_pretty(&report.items).map_err(|_| "diagnostics-encoding")?;
    let readme = if export.memory.is_some() {
        memory_evidence::README
    } else {
        README
    };
    let mut files = BTreeMap::from([
        ("report.json".to_string(), report_bytes),
        ("properties.json".to_string(), properties),
        ("README.txt".to_string(), readme.as_bytes().to_vec()),
    ]);
    if let Some(memory) = &export.memory {
        files.insert(
            "memory.json".into(),
            serde_json::to_vec_pretty(memory).map_err(|_| "diagnostics-encoding")?,
        );
    }
    let manifest = Manifest {
        kind: KIND.into(),
        format: report.format,
        build: report.build.clone(),
        collected_at: report.collected_at.clone(),
        simulated: report.simulated,
        notice: export
            .memory
            .is_some()
            .then(|| memory_evidence::NOTICE.to_string()),
        files: files
            .iter()
            .map(|(k, v)| {
                (
                    k.clone(),
                    Member {
                        bytes: v.len() as u64,
                        sha256: hash(v),
                    },
                )
            })
            .collect(),
    };
    files.insert(
        "manifest.json".into(),
        serde_json::to_vec_pretty(&manifest).map_err(|_| "diagnostics-encoding")?,
    );
    let mut zip = zip::ZipWriter::new(Cursor::new(vec![]));
    for (name, bytes) in files {
        zip.start_file(name, zip::write::SimpleFileOptions::default())
            .map_err(|_| "diagnostics-zip")?;
        zip.write_all(&bytes).map_err(|_| "diagnostics-zip")?;
    }
    let bytes = zip.finish().map_err(|_| "diagnostics-zip")?.into_inner();
    if read_export(&bytes)? != *export {
        return Err("diagnostics-readback-differs".into());
    }
    Ok(bytes)
}
/// Format 1 only: a format-2 archive is refused rather than silently reduced
/// to its identity part; read it with [`read_export`].
pub fn read_bundle(bytes: &[u8]) -> Result<Report, String> {
    let export = read_export(bytes)?;
    if export.memory.is_some() {
        return Err("diagnostics-memory-evidence-needs-read-export".into());
    }
    Ok(export.report)
}
/// Bounded, non-extracting independent readback of format 1 or 2.
/// Unexpected/duplicate (checked in the raw central directory)/missing members, ZIP64, unknown versions, CRC failures,
/// length/hash mismatches and evidence that does not follow from its own
/// plan all refuse.
pub fn read_export(bytes: &[u8]) -> Result<Export, String> {
    if bytes.len() > MAX_ARCHIVE {
        return Err("diagnostics-archive-limit".into());
    }
    // Before `ZipArchive`, which silently keeps one entry per name.
    let raw_names = central_directory::entry_names(bytes)?;
    let mut zip =
        zip::ZipArchive::new(Cursor::new(bytes)).map_err(|_| "diagnostics-invalid-zip")?;
    if !(4..=5).contains(&zip.len()) || zip.len() != raw_names.len() {
        return Err("diagnostics-member-count".into());
    }
    let mut files = BTreeMap::new();
    for (i, raw_name) in raw_names.into_iter().enumerate() {
        let mut f = zip.by_index(i).map_err(|_| "diagnostics-invalid-member")?;
        let name = f.name().to_string();
        if name.as_bytes() != raw_name {
            return Err("diagnostics-invalid-member".into());
        }
        let limit = if name == "memory.json" {
            MAX_MEMORY_MEMBER
        } else {
            MAX_MEMBER
        };
        if !matches!(
            name.as_str(),
            "manifest.json" | "report.json" | "properties.json" | "README.txt" | "memory.json"
        ) || f.size() > limit
            || f.is_dir()
            || files.contains_key(&name)
        {
            return Err("diagnostics-invalid-member".into());
        }
        let size = f.size();
        let mut data = vec![];
        (&mut f)
            .take(limit + 1)
            .read_to_end(&mut data)
            .map_err(|_| "diagnostics-member-read")?;
        if data.len() as u64 != size || data.len() as u64 > limit {
            return Err("diagnostics-member-limit".into());
        }
        files.insert(name, data);
    }
    let m: Manifest = serde_json::from_slice(
        files
            .get("manifest.json")
            .ok_or("diagnostics-invalid-member")?,
    )
    .map_err(|_| "diagnostics-invalid-manifest")?;
    let members: &[&str] = match (m.format, m.notice.as_deref()) {
        (1, None) => &["report.json", "properties.json", "README.txt"],
        (2, Some(memory_evidence::NOTICE)) => &[
            "report.json",
            "properties.json",
            "README.txt",
            "memory.json",
        ],
        _ => return Err("diagnostics-unknown-format".into()),
    };
    if m.kind != KIND || m.files.len() != members.len() || files.len() != members.len() + 1 {
        return Err("diagnostics-unknown-format".into());
    }
    for name in members {
        let v = files.get(*name).ok_or("diagnostics-invalid-member")?;
        let claim = m.files.get(*name).ok_or("diagnostics-missing-hash")?;
        if v.len() as u64 != claim.bytes || hash(v) != claim.sha256 {
            return Err("diagnostics-checksum-mismatch".into());
        }
    }
    let r: Report =
        serde_json::from_slice(&files["report.json"]).map_err(|_| "diagnostics-invalid-report")?;
    let memory: Option<MemoryEvidence> = files
        .get("memory.json")
        .map(|bytes| serde_json::from_slice(bytes).map_err(|_| "diagnostics-invalid-memory"))
        .transpose()?;
    let readme = if memory.is_some() {
        memory_evidence::README
    } else {
        README
    };
    let export = Export { report: r, memory };
    validate_export(&export)?;
    let r = &export.report;
    let properties: Vec<Reading> = serde_json::from_slice(&files["properties.json"])
        .map_err(|_| "diagnostics-invalid-properties")?;
    if r.items != properties
        || m.format != r.format
        || m.build != r.build
        || m.collected_at != r.collected_at
        || m.simulated != r.simulated
        || files["README.txt"] != readme.as_bytes()
    {
        return Err("diagnostics-inconsistent-archive".into());
    }
    Ok(export)
}
/// Create-only, flush+sync, seek and verify the actual held file before success.
/// No path deletion on failure: preserve the failed artifact for inspection.
pub fn write_new_bundle(path: &Path, report: &Report) -> Result<(), String> {
    if report.format != 1 {
        return Err("diagnostics-invalid-format".into());
    }
    write_new_export(
        path,
        &Export {
            report: report.clone(),
            memory: None,
        },
    )
}
/// [`write_new_bundle`] for either format.
pub fn write_new_export(path: &Path, export: &Export) -> Result<(), String> {
    if path.extension().and_then(|s| s.to_str()) != Some("zip") {
        return Err("diagnostics-zip-extension-required".into());
    }
    let bytes = bundle_export(export)?;
    let mut options = OpenOptions::new();
    options.read(true).write(true).create_new(true);
    // Raw device replies can be sensitive: owner-only from the first byte.
    #[cfg(unix)]
    std::os::unix::fs::OpenOptionsExt::mode(&mut options, 0o600);
    let mut f = options
        .open(path)
        .map_err(|_| "diagnostics-create-new-failed")?;
    f.write_all(&bytes)
        .and_then(|_| f.flush())
        .and_then(|_| f.sync_all())
        .and_then(|_| f.seek(SeekFrom::Start(0)).map(|_| ()))
        .map_err(|_| "diagnostics-write-failed")?;
    let mut held = vec![];
    f.take((MAX_ARCHIVE + 1) as u64)
        .read_to_end(&mut held)
        .map_err(|_| "diagnostics-readback-failed")?;
    if held != bytes || read_export(&held)? != *export {
        return Err("diagnostics-readback-differs".into());
    }
    Ok(())
}
