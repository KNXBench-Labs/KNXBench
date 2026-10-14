//! Read-only original device instructions; never changes retained evidence.
use knx_core::{DeviceId, Project};
use knx_etsproj::instructions::{InstallationHints, InstructionReadError};
use knx_store::StoredOpaqueEntry;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OriginalInstructions {
    pub xml_sha256: String,
    /// `None` means the admitted source owner has no InstallationHints field;
    /// `Some` with an empty original is explicitly empty, not absent.
    pub hints: Option<InstallationHints>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstructionSourceError {
    DeviceNotFound,
    MissingSource,
    AmbiguousSource,
    Integrity,
    Adapter(InstructionReadError),
}

/// Inspects the original for one stable domain device ID. Callers retain the
/// returned source digest for later edit-consent/conflict binding. This is not
/// permission to write, and a derived preview never replaces opaque entries.
pub fn original_device_instructions(
    project: &Project,
    opaque: &[StoredOpaqueEntry],
    device: DeviceId,
) -> Result<OriginalInstructions, InstructionSourceError> {
    use InstructionSourceError as E;
    let source = &project.devices.get(device).ok_or(E::DeviceNotFound)?.source;
    if source.path.is_empty() || source.ets_id.is_empty() {
        return Err(E::MissingSource);
    }
    let mut originals = opaque.iter().filter(|entry| {
        entry.source_path == source.path
            && entry.xpath.is_empty()
            && entry.name.is_empty()
            && entry.kind == "ContainerEntry"
    });
    let original = originals.next().ok_or(E::MissingSource)?;
    if originals.next().is_some() {
        return Err(E::AmbiguousSource);
    }
    // Refuse oversized sources before hashing. This guard preserves the
    // adapter's bounded read contract without allocating another byte buffer.
    if original.bytes.len() > knx_etsproj::instructions::MAX_XML_BYTES {
        return Err(E::Adapter(InstructionReadError::SourceLimit));
    }
    if knx_etsproj::opaque::sha256_hex(&original.bytes) != original.sha256 {
        return Err(E::Integrity);
    }
    let hints = knx_etsproj::instructions::read_installation_hints(
        &original.bytes,
        &source.path,
        &source.ets_id,
    )
    .map_err(E::Adapter)?;
    Ok(OriginalInstructions {
        xml_sha256: original.sha256.clone(),
        hints,
    })
}

/// Where a device's instructions start from (Goal B4).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InstructionOrigin {
    /// Imported: the retained original, exactly as [`original_device_instructions`] reads it.
    Imported(OriginalInstructions),
    /// Made in KNXBench: there is no imported original, and none is invented.
    Native,
}

/// The prefix KNXBench gives the provenance of what it creates itself
/// (`KB-<Kind>-<id>`, ADR-0093; `KB-DEV-<id>` and `KB-SCAN-<address>` for devices).
const KNXBENCH_SOURCE_PREFIX: &str = "KB-";

/// [`InstructionOrigin::Native`] only for a device KNXBench created: its source
/// path and id are the same `KB-…` text, and nothing at all was retained from
/// that path. Every other device reads its imported original; one whose
/// retained source is missing is still refused, never treated as native.
pub fn instruction_origin(
    project: &Project,
    opaque: &[StoredOpaqueEntry],
    device: DeviceId,
) -> Result<InstructionOrigin, InstructionSourceError> {
    let source = &project
        .devices
        .get(device)
        .ok_or(InstructionSourceError::DeviceNotFound)?
        .source;
    let issued_here =
        source.path.starts_with(KNXBENCH_SOURCE_PREFIX) && source.path == source.ets_id;
    if issued_here && !opaque.iter().any(|entry| entry.source_path == source.path) {
        return Ok(InstructionOrigin::Native);
    }
    original_device_instructions(project, opaque, device).map(InstructionOrigin::Imported)
}
