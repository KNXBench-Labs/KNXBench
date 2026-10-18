//! Format-2 memory evidence: ProductDB-declared candidate ranges, chunks and coverage.
//!
//! ADR-0143. The ranges are what one System-2 load procedure declares; they
//! are not proven runtime addresses, and the archive is neither a backup nor
//! a restore basis. Unknown octets stay absent, never zero-filled.
use chrono::{DateTime, Utc};
use knx_core::commissioning::declared_memory::{
    DeclaredRange, DeclaredRanges, STANDARD_FRAME_CHUNK_OCTETS,
};
use knx_core::commissioning::memory_download::property_matches;
use knx_core::commissioning::properties::PID_HARDWARE_TYPE;
use knx_core::{DeviceId, Project};
use knx_net::commissioning::diagnostic_export::{self as net, MemoryItem, MemoryRequest, Status};
use knx_net::{ManagementTransport, SessionTiming};
use knx_productdb::declared_segments::{derive_declared_segments, DeclaredSegments};
use serde::{Deserialize, Serialize};
use std::sync::atomic::AtomicBool;

use super::{hex, is_token, prepare, readings, Export, Prepared, Reading, Report, KIND};

pub use knx_productdb::declared_segments::PROFILE as MEMORY_PROFILE;

/// `report.json`'s `memoryReason` in format 2.
pub(crate) const MEMORY_REASON: &str = "Source-declared candidate ranges of the device's application, read only after the device showed the asserted identity; see memory.json.";
/// `manifest.json`'s `notice`, and the same statement in `memory.json`.
pub(crate) const NOTICE: &str = "Source-declared candidate locations, not proven runtime addresses. This is not a backup and not a restore basis. It contains device configuration: keep private.";
pub(crate) const README: &str = "KNXBench read-only device diagnostics, format 2 (system2-declared-segments-v1).\nRequested: descriptor type 0, Device Object properties 12/78 and, only after the device\nshowed the asserted mask, manufacturer and hardware type, A_Memory_Read of the memory\nranges its application's load procedure declares (memory.json).\nThese are source-declared candidate locations, not proven runtime addresses.\nThis is not a backup and not a restore basis. Nothing was written to the device.\nThe memory contains device configuration: keep private.\nUnread or refused octets are listed as such and never filled with zeros.\nA refusal cannot distinguish missing rights from absent memory.\nSIMULATED is recorded in report.json and manifest.json when using a simulator.\nNo automatic upload. Review the files, then send the ZIP to issue@knxbench.com or share a file link privately.\nChecksums detect corruption, not maliciously rewritten evidence.\n";

/// The device-side identity comparison the profile requires.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Comparison {
    pub object_index: u8,
    pub property_id: u8,
    /// Zero-based position of `LdCtrlCompareProp` in the load procedure.
    pub procedure_position: usize,
    /// `@InlineData`, lowercase hex.
    pub inline_data: String,
}
/// Octets of one range by chunk outcome; the five always sum to its length.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Coverage {
    pub read: u32,
    pub refused_ambiguous: u32,
    pub partial: u32,
    pub error: u32,
    pub not_read: u32,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RangeEvidence {
    pub order: usize,
    pub address: u16,
    pub length: u16,
    /// Zero-based position of its `LdCtrlAbsSegment` in the load procedure.
    pub procedure_position: usize,
    pub coverage: Coverage,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ChunkReading {
    pub order: usize,
    /// One-based `RangeEvidence::order`.
    pub range: usize,
    pub address: u16,
    pub requested_octets: u8,
    pub status: String,
    pub received_octets: usize,
    /// Exactly the octets answered, lowercase hex; empty when none.
    pub bytes: String,
    pub attempted_at: Option<String>,
    pub completed_at: Option<String>,
    pub error: Option<String>,
}
/// `memory.json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MemoryEvidence {
    pub profile: String,
    pub notice: String,
    /// The application program the ranges come from (a catalogue id).
    pub program_id: String,
    pub hardware_type_comparison: Comparison,
    pub chunk_octets: u8,
    pub ranges: Vec<RangeEvidence>,
    pub chunks: Vec<ChunkReading>,
}

/// One selected device and the profile its application declares.
///
/// Opaque: only [`prepare_memory`] makes one, so every range
/// [`acquire_memory`] reads comes from the ProductDB derivation. Outside code
/// reads it through accessors and can neither build nor edit it:
///
/// ```compile_fail,E0451
/// use knx_app::device_diagnostics::{MemoryPrepared, Prepared};
/// use knx_productdb::declared_segments::DeclaredSegments;
/// fn forge(base: Prepared, profile: DeclaredSegments) -> MemoryPrepared {
///     MemoryPrepared { base, profile }
/// }
/// ```
///
/// ```compile_fail,E0616
/// use knx_app::device_diagnostics::MemoryPrepared;
/// use knx_productdb::declared_segments::DeclaredSegments;
/// fn swap(plan: &mut MemoryPrepared, profile: DeclaredSegments) {
///     plan.profile = profile;
/// }
/// ```
///
/// ```compile_fail,E0616
/// use knx_app::device_diagnostics::{MemoryPrepared, Prepared};
/// fn retarget(plan: &mut MemoryPrepared, base: Prepared) {
///     plan.base = base;
/// }
/// ```
#[derive(Debug, Clone)]
pub struct MemoryPrepared {
    base: Prepared,
    profile: DeclaredSegments,
}

impl MemoryPrepared {
    /// The selected device, its asserted identity and the opaque reference.
    pub fn base(&self) -> &Prepared {
        &self.base
    }
    /// The ProductDB-derived profile, for previews.
    pub fn profile(&self) -> &DeclaredSegments {
        &self.profile
    }
}

/// Resolves the device's application through the project → product mapping
/// and derives the profile; refuses unless mask and manufacturer are asserted
/// by the product data and agree with the program's declared mask.
pub fn prepare_memory(
    project: &Project,
    device: DeviceId,
    db: &knx_productdb::Connection,
) -> Result<MemoryPrepared, String> {
    let base = prepare(project, device, Some(db))?;
    let instance = project
        .devices
        .iter()
        .find(|d| d.id == device)
        .ok_or("diagnostics-device-missing")?;
    if instance.program_ref.is_empty() {
        return Err("diagnostics-program-missing: the device names no application".into());
    }
    let program_id = knx_productdb::query::resolve_program(db, &instance.program_ref)
        .map_err(|_| "diagnostics-product-db-unreadable")?
        .ok_or(
            "diagnostics-program-unresolved: the product database lacks the device's application",
        )?;
    let code = knx_productdb::code::load_program_code(db, &program_id)
        .map_err(|_| "diagnostics-program-code-unreadable")?
        .ok_or("diagnostics-program-code-missing")?;
    let profile = derive_declared_segments(&code)
        .map_err(|e| format!("diagnostics-profile-unavailable: {e}"))?;
    match (base.expected.mask, base.expected.manufacturer) {
        (Some(mask), Some(_)) if mask == profile.mask() => {}
        (Some(_), Some(_)) => return Err("diagnostics-identity-inconsistent".into()),
        _ => return Err("diagnostics-identity-assertion-missing".into()),
    }
    Ok(MemoryPrepared { base, profile })
}

fn time(t: Option<std::time::SystemTime>) -> Option<String> {
    t.map(|t| DateTime::<Utc>::from(t).to_rfc3339())
}
fn chunk_readings(items: &[MemoryItem]) -> Vec<ChunkReading> {
    items
        .iter()
        .enumerate()
        .map(|(i, item)| ChunkReading {
            order: i + 1,
            range: item.range + 1,
            address: item.address,
            requested_octets: item.requested,
            status: item.status.name().into(),
            received_octets: item.bytes.len(),
            bytes: hex(&item.bytes),
            attempted_at: time(item.attempted_at),
            completed_at: time(item.completed_at),
            error: item.error.map(str::to_string),
        })
        .collect()
}
/// Octets per range by chunk status; recomputed by the reader.
fn coverage(range_count: usize, chunks: &[ChunkReading]) -> Vec<Coverage> {
    let mut out = vec![Coverage::default(); range_count];
    for chunk in chunks {
        let Some(c) = chunk.range.checked_sub(1).and_then(|i| out.get_mut(i)) else {
            continue;
        };
        let octets = u32::from(chunk.requested_octets);
        match chunk.status.as_str() {
            "read" => c.read += octets,
            "refused-ambiguous" => c.refused_ambiguous += octets,
            "partial" => c.partial += octets,
            "error" => c.error += octets,
            _ => c.not_read += octets,
        }
    }
    out
}
fn set_chunks(memory: &mut MemoryEvidence, items: &[MemoryItem]) {
    memory.chunks = chunk_readings(items);
    let computed = coverage(memory.ranges.len(), &memory.chunks);
    for (range, coverage) in memory.ranges.iter_mut().zip(computed) {
        range.coverage = coverage;
    }
}

/// The export before any read: every chunk `not-read`.
pub fn pending_memory(plan: &MemoryPrepared, build: &str, simulated: bool) -> Export {
    let mut report = super::pending(&plan.base, build, simulated);
    report.format = 2;
    report.profile = MEMORY_PROFILE.into();
    report.memory_status = "requested".into();
    report.memory_reason = MEMORY_REASON.into();
    let profile = &plan.profile;
    let mut memory = MemoryEvidence {
        profile: MEMORY_PROFILE.into(),
        notice: NOTICE.into(),
        program_id: profile.program_id().to_string(),
        hardware_type_comparison: Comparison {
            object_index: 0,
            property_id: PID_HARDWARE_TYPE,
            procedure_position: profile.compare_position(),
            inline_data: hex(profile.hardware_type()),
        },
        chunk_octets: STANDARD_FRAME_CHUNK_OCTETS,
        ranges: profile
            .ranges()
            .ranges()
            .iter()
            .zip(profile.range_positions())
            .enumerate()
            .map(|(i, (range, position))| RangeEvidence {
                order: i + 1,
                address: range.address,
                length: range.length,
                procedure_position: *position,
                coverage: Coverage::default(),
            })
            .collect(),
        chunks: vec![],
    };
    let pending: Vec<MemoryItem> = profile
        .ranges()
        .chunks()
        .into_iter()
        .map(MemoryItem::pending)
        .collect();
    set_chunks(&mut memory, &pending);
    Export {
        report,
        memory: Some(memory),
    }
}

/// The memory profile's one acquisition entry point; same observer and
/// teardown contract as [`super::acquire`].
pub async fn acquire_memory<T: ManagementTransport>(
    transport: &T,
    plan: &MemoryPrepared,
    timing: SessionTiming,
    build: &str,
    cancel: &AtomicBool,
    mut observe: impl FnMut(&Export),
) -> Export {
    let mut export = pending_memory(
        plan,
        build,
        matches!(
            transport.target_kind(),
            knx_core::commissioning::mutation::TargetKind::Simulator
        ),
    );
    let collected = net::collect_declared_memory(
        transport,
        plan.base.target,
        timing,
        MemoryRequest {
            expected: plan.base.expected,
            hardware_type: plan.profile.hardware_type(),
            ranges: plan.profile.ranges(),
        },
        cancel,
        |items, memory| {
            export.report.items = readings(items);
            if let Some(evidence) = export.memory.as_mut() {
                set_chunks(evidence, memory);
            }
            observe(&export);
        },
    )
    .await;
    export.report.items = readings(&collected.acquisition.items);
    if let Some(evidence) = export.memory.as_mut() {
        set_chunks(evidence, &collected.memory);
    }
    export.report.stop_reason = collected.acquisition.stop_reason.map(str::to_string);
    export.report.disconnect_attempt_completed = collected.acquisition.disconnect_attempt_completed;
    export.report.complete = true;
    observe(&export);
    export
}

fn unhex(s: &str) -> Option<Vec<u8>> {
    if !s.len().is_multiple_of(2)
        || !s
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return None;
    }
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).ok())
        .collect()
}

/// The reader's and writer's check: the evidence must follow from its own
/// declared ranges, and memory may only have been read after a verified identity.
pub(crate) fn validate(report: &Report, m: &MemoryEvidence) -> Result<(), String> {
    let invalid = || Err::<(), String>("diagnostics-invalid-memory".into());
    debug_assert_eq!(report.kind, KIND);
    let comparison = &m.hardware_type_comparison;
    if m.profile != MEMORY_PROFILE
        || m.notice != NOTICE
        || !(1..=128).contains(&m.program_id.len())
        || !m
            .program_id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"-_.".contains(&b))
        || comparison.object_index != 0
        || comparison.property_id != PID_HARDWARE_TYPE
        || unhex(&comparison.inline_data).is_none_or(|v| v.is_empty() || v.len() > 64)
        || m.chunk_octets != STANDARD_FRAME_CHUNK_OCTETS
    {
        return invalid();
    }
    let declared: Vec<DeclaredRange> = m
        .ranges
        .iter()
        .map(|r| DeclaredRange {
            address: r.address,
            length: r.length,
        })
        .collect();
    let Ok(ranges) = DeclaredRanges::new(declared) else {
        return invalid();
    };
    let positions_increase = m
        .ranges
        .windows(2)
        .all(|w| w[0].procedure_position < w[1].procedure_position);
    if !positions_increase
        || m.ranges
            .iter()
            .enumerate()
            .any(|(i, r)| r.order != i + 1 || r.procedure_position == comparison.procedure_position)
    {
        return invalid();
    }
    let plan = ranges.chunks();
    if plan.len() != m.chunks.len() {
        return invalid();
    }
    for (i, (chunk, planned)) in m.chunks.iter().zip(&plan).enumerate() {
        let Some(bytes) = unhex(&chunk.bytes) else {
            return invalid();
        };
        let requested = usize::from(chunk.requested_octets);
        let shape_ok = chunk.order == i + 1
            && chunk.range == planned.range + 1
            && chunk.address == planned.address
            && chunk.requested_octets == planned.length
            && chunk.received_octets == bytes.len()
            && bytes.len() <= net::MAX_RESPONSE_BYTES
            && chunk.error.as_deref().is_none_or(is_token)
            && [&chunk.attempted_at, &chunk.completed_at]
                .into_iter()
                .flatten()
                .all(|t| DateTime::parse_from_rfc3339(t).is_ok());
        let status_ok = match chunk.status.as_str() {
            "read" => bytes.len() == requested && chunk.error.is_none(),
            // The session turns an empty answer into a refusal, so a
            // partial answer always holds at least one octet.
            "partial" => !bytes.is_empty() && bytes.len() != requested && chunk.error.is_some(),
            "refused-ambiguous" | "error" => bytes.is_empty() && chunk.error.is_some(),
            // Never attempted, or (observer snapshot only) in flight.
            "not-read" => {
                bytes.is_empty()
                    && (chunk.attempted_at.is_none() || !report.complete)
                    && chunk.completed_at.is_none()
                    && chunk.error.is_none()
            }
            _ => false,
        };
        if !shape_ok
            || !status_ok
            || !(chunk.status == "not-read" || attempted_then_completed(chunk))
        {
            return invalid();
        }
    }
    // One request at a time, in plan order: nothing after an in-flight
    // chunk has been attempted.
    if let Some(at) = m.chunks.iter().position(in_flight) {
        if m.chunks[at + 1..].iter().any(|c| c.attempted_at.is_some()) {
            return invalid();
        }
    }
    let computed = coverage(m.ranges.len(), &m.chunks);
    if m.ranges
        .iter()
        .zip(&computed)
        .any(|(r, c)| r.coverage != *c)
    {
        return invalid();
    }
    // Memory is requested only after the device showed the asserted identity.
    let memory_entered = m.chunks.iter().any(|c| {
        c.status != "not-read"
            || c.attempted_at.is_some()
            || c.completed_at.is_some()
            || !c.bytes.is_empty()
    });
    let Some(hardware_type) = unhex(&comparison.inline_data) else {
        return invalid();
    };
    let admitted = identity_admits(report, &hardware_type);
    if (memory_entered && !admitted) || !sequence_ok(report, &m.chunks, admitted) {
        return invalid();
    }
    Ok(())
}

/// The order `collect_declared_memory` reads chunks in: a refusal or a
/// partial answer ends its own range, an error ends the run with that error
/// as the stop reason, and a chunk left unread outside an ended range means
/// the run was cancelled before it. A complete run with nothing ended has no
/// stop reason, or only the caller's unconfirmed tunnel teardown; an
/// observer snapshot (`complete: false`) has none yet, and its one in-flight
/// chunk ends the sequence like an unread one.
fn sequence_ok(report: &Report, chunks: &[ChunkReading], identity_admitted: bool) -> bool {
    let mut ended_range = None;
    // `Some(Some(error))` after an error, `Some(None)` after an unread chunk.
    let mut run_end: Option<Option<&str>> = None;
    let mut entered = false;
    for chunk in chunks {
        let unread = chunk.status == "not-read";
        if run_end.is_some() || ended_range == Some(chunk.range) {
            // The collector never attempts a chunk it has already given up on.
            if !unread || in_flight(chunk) {
                return false;
            }
            continue;
        }
        match chunk.status.as_str() {
            "read" => {}
            "partial" | "refused-ambiguous" => ended_range = Some(chunk.range),
            "error" => run_end = Some(chunk.error.as_deref()),
            _ => run_end = Some(None),
        }
        entered |= !unread;
    }
    let stop = report.stop_reason.as_deref();
    if !report.complete {
        return stop.is_none();
    }
    match run_end {
        Some(Some(error)) => stop == Some(error),
        None => stop.is_none_or(|s| s == super::TUNNEL_DISCONNECT_UNCONFIRMED),
        // Admitted identity, then a chunk never attempted: cancelled first.
        Some(None) if entered || identity_admitted => stop == Some("cancelled"),
        // Stopped before memory: some identity-phase reason.
        Some(None) => stop.is_some_and(|s| s != super::TUNNEL_DISCONNECT_UNCONFIRMED),
    }
}

/// The collector's in-flight shape: requested, no outcome yet. Admitted
/// only in an observer snapshot (`complete: false`).
fn in_flight(chunk: &ChunkReading) -> bool {
    chunk.status == "not-read" && chunk.attempted_at.is_some()
}

/// A chunk with an outcome was attempted, then completed, in that order.
fn attempted_then_completed(chunk: &ChunkReading) -> bool {
    super::attempted_then_completed(&chunk.attempted_at, &chunk.completed_at)
}

/// The collector's own admission, re-checked from the archive: all three
/// identity items exactly read (one element for each property, none for the
/// descriptor), the descriptor equal to the asserted mask, PID12 equal to the
/// asserted manufacturer and PID78 `property_matches` the recorded comparison.
fn identity_admits(report: &Report, hardware_type: &[u8]) -> bool {
    let [descriptor, manufacturer, hardware] = &report.items[..] else {
        return false;
    };
    let exact = |item: &Reading, elements: Option<u8>| {
        item.status == Status::Read.name()
            && item.bytes.len() == item.expected_bytes
            && item.received_bytes == item.bytes.len()
            && item.returned_elements == elements
    };
    let word = |item: &Reading| match item.bytes[..] {
        [high, low] => Some(u16::from_be_bytes([high, low])),
        _ => None,
    };
    exact(descriptor, None)
        && exact(manufacturer, Some(1))
        && exact(hardware, Some(1))
        && report.asserted_mask.is_some()
        && word(descriptor) == report.asserted_mask
        && report.asserted_manufacturer.is_some()
        && word(manufacturer) == report.asserted_manufacturer
        && property_matches(&hardware.bytes, hardware_type)
}
