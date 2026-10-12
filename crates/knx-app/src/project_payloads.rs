//! Read-only catalogue and verified local bytes of retained project payloads.

use knx_store::StoredOpaqueEntry;
use serde::Serialize;
pub const MAX_DOWNLOAD_BYTES: usize = 64 * 1024 * 1024;
pub const MAX_PAYLOADS: usize = 10_000;
/// Cumulative source-path/kind/hash text, before catalogue cloning.
pub const MAX_METADATA_BYTES: usize = 4 * 1024 * 1024;

#[derive(Debug, Clone, Serialize)]
pub struct PayloadSummary {
    pub id: u32,
    pub source_path: String,
    pub kind: String,
    pub role: String,
    pub content_class: String,
    pub bytes: usize,
    pub sha256: String,
    pub downloadable: bool,
}

fn role(entry: &StoredOpaqueEntry) -> Option<&'static str> {
    if !entry.xpath.is_empty()
        || entry.kind.starts_with("KnxBench")
        || entry.kind.starts_with("SelectiveImport")
    {
        return None;
    }
    let in_directory = |directory: &str| {
        entry
            .source_path
            .rsplit_once('/')
            .is_some_and(|(parent, _)| {
                parent
                    .split('/')
                    .skip(1)
                    .any(|part| part.eq_ignore_ascii_case(directory))
            })
    };
    if entry
        .source_path
        .rsplit_once('.')
        .is_some_and(|(_, suffix)| suffix.eq_ignore_ascii_case("etsapp"))
    {
        Some("ets-app-named-candidate")
    } else if in_directory("UserFiles") {
        Some("user-file-candidate")
    } else if in_directory("AddinData") {
        Some("add-in-state-candidate")
    } else if entry.kind == "Baggage" {
        Some("manufacturer-baggage")
    } else {
        None
    }
}

/// Labels describe retained bytes and path-derived candidates, not execution support.
pub fn catalogue(entries: &[StoredOpaqueEntry]) -> Result<Vec<PayloadSummary>, String> {
    let mut rows = Vec::new();
    let mut metadata_bytes = 0usize;
    for (index, entry) in entries.iter().enumerate() {
        let Some(role) = role(entry) else {
            continue;
        };
        if rows.len() >= MAX_PAYLOADS {
            return Err("retained payload catalogue exceeds 10000 entries".into());
        }
        for length in [
            entry.source_path.len(),
            entry.kind.len(),
            entry.sha256.len(),
        ] {
            metadata_bytes = metadata_bytes
                .checked_add(length)
                .filter(|total| *total <= MAX_METADATA_BYTES)
                .ok_or("retained payload catalogue exceeds 4 MiB of metadata")?;
        }
        rows.push(PayloadSummary {
            id: u32::try_from(index).map_err(|_| "payload selector exceeds integer range")?,
            source_path: entry.source_path.clone(),
            kind: entry.kind.clone(),
            role: role.into(),
            content_class: knx_productdb::sniff_media(&entry.bytes).as_str().into(),
            bytes: entry.bytes.len(),
            sha256: entry.sha256.clone(),
            downloadable: entry.bytes.len() <= MAX_DOWNLOAD_BYTES,
        });
    }
    Ok(rows)
}

/// Explicit bytes only; no rendering, path traversal, extraction, execution or repair.
pub fn download(entries: &[StoredOpaqueEntry], id: u32) -> Result<&[u8], String> {
    let entry = entries
        .get(id as usize)
        .filter(|entry| role(entry).is_some())
        .ok_or("retained payload unavailable")?;
    if entry.bytes.len() > MAX_DOWNLOAD_BYTES {
        return Err(
            "retained payload download exceeds 64 MiB; source bytes remain preserved".into(),
        );
    }
    if knx_store::project_history::image_hash(&entry.bytes) != entry.sha256 {
        return Err("retained payload SHA-256 mismatch; download refused".into());
    }
    Ok(&entry.bytes)
}
