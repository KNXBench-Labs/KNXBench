//! Bounded, read-only, manufacturer-independent device identity acquisition.
//!
//! `collect` reads identity only. `collect_declared_memory` (ADR-0143) adds
//! `A_Memory_Read`s of source-declared candidate ranges, and only after the
//! device has shown the asserted mask, manufacturer and hardware type. Mask
//! families and executable download plans confer no authority to read memory.
use std::future::Future;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, SystemTime};

use crate::{ManagementSession, ManagementTransport, SessionError, SessionTiming};
use knx_core::commissioning::authorisation::AuthorisationPlan;
use knx_core::commissioning::declared_memory::{Chunk, DeclaredRanges};
use knx_core::commissioning::memory_download::property_matches;
use knx_core::commissioning::properties::{ObjectIndex, PID_HARDWARE_TYPE, PID_MANUFACTURER_ID};
use knx_core::ContactableAddress;

pub const DEADLINE: Duration = Duration::from_secs(30);
/// Overall bound for `collect_declared_memory`. The profile ceiling of
/// 2048 octets in 12-octet standard-frame chunks over at most eight ranges
/// is at most 178 `A_Memory_Read`s; with connect and three identity reads
/// that is about one second per confirmed exchange. The UP 258D12 profile
/// needs 128 chunks.
pub const MEMORY_DEADLINE: Duration = Duration::from_secs(180);
pub const MAX_RESPONSE_BYTES: usize = 512;
/// These are assertions from project/catalogue data, not observations or a
/// product/application match. Missing assertions do not prohibit generic reads.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Identity {
    pub mask: Option<u16>,
    pub manufacturer: Option<u16>,
}
/// Closed allowlist: no caller-controlled PID, object, memory address or key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Query {
    Descriptor,
    Manufacturer,
    HardwareType,
}
impl Query {
    pub fn expected_bytes(self) -> usize {
        match self {
            Self::Descriptor | Self::Manufacturer => 2,
            Self::HardwareType => 6,
        }
    }
    pub fn property(self) -> Option<u8> {
        match self {
            Self::Descriptor => None,
            Self::Manufacturer => Some(PID_MANUFACTURER_ID),
            Self::HardwareType => Some(PID_HARDWARE_TYPE),
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            Self::Descriptor => "descriptor-type-0",
            Self::Manufacturer => "manufacturer-id",
            Self::HardwareType => "hardware-type",
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    Read,
    Partial,
    Refused,
    Error,
    NotRead,
}
impl Status {
    pub fn name(self) -> &'static str {
        match self {
            Self::Read => "read",
            Self::Partial => "partial",
            Self::Refused => "refused-ambiguous",
            Self::Error => "error",
            Self::NotRead => "not-read",
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Item {
    pub query: Query,
    pub status: Status,
    pub bytes: Vec<u8>,
    pub received_bytes: usize,
    pub returned_elements: Option<u8>,
    pub attempted_at: Option<SystemTime>,
    pub completed_at: Option<SystemTime>,
    pub error: Option<&'static str>,
}
impl Item {
    pub fn pending(query: Query) -> Self {
        Self {
            query,
            status: Status::NotRead,
            bytes: vec![],
            received_bytes: 0,
            returned_elements: None,
            attempted_at: None,
            completed_at: None,
            error: None,
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Acquisition {
    pub items: Vec<Item>,
    pub stop_reason: Option<&'static str>,
    pub disconnect_attempt_completed: bool,
}
fn failure(e: SessionError) -> &'static str {
    match e {
        SessionError::NoAnswer { .. } => "timeout",
        SessionError::ConnectionReleased { .. } => "connection-released",
        SessionError::ConnectionLost { .. } => "connection-lost",
        SessionError::ConnectRejected { .. } => "connect-rejected",
        SessionError::SelfAddressed(_) => "self-addressed",
        SessionError::Excluded(_) => "excluded",
        SessionError::Lagged { .. } => "event-loss",
        SessionError::NotConnected => "not-connected",
        SessionError::Transport(_) => "transport-error",
        // AL §3.5.3's zero-length answer; the identity recipe never reads memory.
        SessionError::MemoryRefused { .. } => "memory-refused",
        _ => "management-error",
    }
}
/// Cancellation stops awaiting a response, not the cleanup. No operation is
/// retried here; ManagementSession retains its established four-transmission
/// transport ladder. The single overall deadline bounds even a stalled adapter.
async fn bounded<F, T>(
    cancel: &AtomicBool,
    deadline: tokio::time::Instant,
    f: F,
) -> Result<T, &'static str>
where
    F: Future<Output = Result<T, SessionError>>,
{
    if cancel.load(Ordering::SeqCst) {
        return Err("cancelled");
    }
    if tokio::time::Instant::now() >= deadline {
        return Err("deadline");
    }
    tokio::select! {
        result = tokio::time::timeout_at(deadline, f) => result.map_err(|_| "deadline")?.map_err(failure),
        _ = async { loop { tokio::time::sleep(Duration::from_millis(25)).await; if cancel.load(Ordering::SeqCst) { break; } } } => Err("cancelled"),
    }
}
fn pending_acquisition() -> Acquisition {
    Acquisition {
        items: [Query::Descriptor, Query::Manufacturer, Query::HardwareType]
            .into_iter()
            .map(Item::pending)
            .collect(),
        stop_reason: None,
        disconnect_attempt_completed: false,
    }
}
/// Connect → descriptor → Device Object PID12 → PID78 → disconnect. The fixed
/// optional properties may be absent on any device; absence is not compatibility.
/// Observer snapshots are emitted before/after reads, so cancellation and later
/// failures cannot erase acquired bytes. A caller must retain this future until
/// cleanup finishes, rather than aborting its task on an HTTP disconnect.
pub async fn collect<T: ManagementTransport>(
    transport: &T,
    target: ContactableAddress,
    timing: SessionTiming,
    expected: Identity,
    cancel: &AtomicBool,
    mut observe: impl FnMut(&[Item]),
) -> Acquisition {
    let mut out = pending_acquisition();
    observe(&out.items);
    if cancel.load(Ordering::SeqCst) {
        out.stop_reason = Some("cancelled");
        return out;
    }
    let mut session = match ManagementSession::read_only(
        transport,
        target.address(),
        AuthorisationPlan::Skip,
        timing,
    ) {
        Ok(s) => s,
        Err(e) => {
            out.stop_reason = Some(failure(e));
            return out;
        }
    };
    let deadline = tokio::time::Instant::now() + DEADLINE;
    match bounded(cancel, deadline, session.connect()).await {
        Err(reason) => out.stop_reason = Some(reason),
        Ok(()) => {
            identity_reads(
                &mut session,
                &mut out,
                expected,
                cancel,
                deadline,
                &mut observe,
            )
            .await
        }
    }
    out.disconnect_attempt_completed = teardown(&mut session).await;
    observe(&out.items);
    out
}
/// Always attempt device-side teardown, even if connect was cancelled after
/// sending T_Connect. This is separate from the caller's tunnel disconnect.
async fn teardown<T: ManagementTransport>(session: &mut ManagementSession<'_, T>) -> bool {
    tokio::time::timeout(Duration::from_secs(3), session.disconnect())
        .await
        .is_ok()
}
/// The three identity reads, shared by both recipes.
async fn identity_reads<T: ManagementTransport>(
    session: &mut ManagementSession<'_, T>,
    out: &mut Acquisition,
    expected: Identity,
    cancel: &AtomicBool,
    deadline: tokio::time::Instant,
    observe: &mut impl FnMut(&[Item]),
) {
    for index in 0..out.items.len() {
        if cancel.load(Ordering::SeqCst) {
            out.stop_reason = Some("cancelled");
            break;
        }
        out.items[index].attempted_at = Some(SystemTime::now());
        observe(&out.items);
        let query = out.items[index].query;
        let read = bounded(cancel, deadline, async {
            match query.property() {
                None => session.read_descriptor_raw().await.map(|v| (None, v)),
                Some(pid) => session
                    .read_property_raw(ObjectIndex::DEVICE, pid, 1, 1)
                    .await
                    .map(|(n, v)| (Some(n), v)),
            }
        })
        .await;
        let item = &mut out.items[index];
        item.completed_at = Some(SystemTime::now());
        match read {
            Err(reason) => {
                item.status = Status::Error;
                item.error = Some(reason);
                out.stop_reason = Some(reason);
            }
            Ok((elements, bytes)) => {
                item.returned_elements = elements;
                item.received_bytes = bytes.len();
                if bytes.len() > MAX_RESPONSE_BYTES {
                    item.status = Status::Error;
                    item.error = Some("response-size-limit-raw-not-retained");
                    out.stop_reason = item.error;
                } else {
                    item.bytes = bytes;
                    item.status = if elements == Some(0) {
                        item.error = Some("access-denied-or-property-unsupported");
                        Status::Refused
                    } else if item.bytes.len() != query.expected_bytes()
                        || elements.is_some_and(|n| n != 1)
                    {
                        item.error = Some("unexpected-length");
                        Status::Partial
                    } else {
                        Status::Read
                    };
                }
            }
        }
        // An unverified descriptor prevents all subsequent reads. A
        // manufacturer conflict stops before hardware metadata; neither
        // identity pair identifies a product/application or grants writes.
        if item.status == Status::Read {
            let actual = u16::from_be_bytes([item.bytes[0], item.bytes[1]]);
            let asserted = match query {
                Query::Descriptor => expected.mask,
                Query::Manufacturer => expected.manufacturer,
                Query::HardwareType => None,
            };
            if asserted.is_some_and(|v| v != actual) {
                out.stop_reason = Some("identity-mismatch");
            }
        } else if query == Query::Descriptor && out.stop_reason.is_none() {
            out.stop_reason = Some("descriptor-unverified");
        }
        observe(&out.items);
        if out.stop_reason.is_some() {
            break;
        }
    }
}

/// What `collect_declared_memory` reads and what the device must prove first.
#[derive(Debug, Clone, Copy)]
pub struct MemoryRequest<'a> {
    /// Both values are required; a missing one refuses before connecting.
    pub expected: Identity,
    /// The source's `PID_HARDWARE_TYPE` comparison value (`property_matches`).
    pub hardware_type: &'a [u8],
    /// Validated, source-declared candidate ranges.
    pub ranges: &'a DeclaredRanges,
}
/// One planned `A_Memory_Read` and its outcome.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryItem {
    /// Zero-based declared range.
    pub range: usize,
    pub address: u16,
    pub requested: u8,
    pub status: Status,
    /// Exactly what the device answered; never padded or zero-filled.
    pub bytes: Vec<u8>,
    pub attempted_at: Option<SystemTime>,
    pub completed_at: Option<SystemTime>,
    pub error: Option<&'static str>,
}
impl MemoryItem {
    pub fn pending(chunk: Chunk) -> Self {
        Self {
            range: chunk.range,
            address: chunk.address,
            requested: chunk.length,
            status: Status::NotRead,
            bytes: vec![],
            attempted_at: None,
            completed_at: None,
            error: None,
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryAcquisition {
    /// The identity items, the overall stop reason and the teardown.
    pub acquisition: Acquisition,
    /// One item per planned chunk, in plan order.
    pub memory: Vec<MemoryItem>,
}
/// Connect → descriptor → PID12 → PID78 → memory chunks → disconnect.
///
/// Memory is read only if all three identity items are exactly read, the
/// descriptor and PID12 equal the asserted values and PID78
/// `property_matches` the source comparison; otherwise the stop reason is
/// `identity-mismatch` or `identity-unverified` and no `A_Memory_Read` is
/// sent. A refused chunk (`refused-ambiguous`) or an answer of another
/// length (`partial`, bytes kept) ends its own range only. Timeout,
/// connection loss, cancellation or the deadline end everything; earlier
/// chunks stay. No retry beyond the session's transport ladder.
pub async fn collect_declared_memory<T: ManagementTransport>(
    transport: &T,
    target: ContactableAddress,
    timing: SessionTiming,
    request: MemoryRequest<'_>,
    cancel: &AtomicBool,
    mut observe: impl FnMut(&[Item], &[MemoryItem]),
) -> MemoryAcquisition {
    let mut out = MemoryAcquisition {
        acquisition: pending_acquisition(),
        memory: request
            .ranges
            .chunks()
            .into_iter()
            .map(MemoryItem::pending)
            .collect(),
    };
    observe(&out.acquisition.items, &out.memory);
    if request.expected.mask.is_none()
        || request.expected.manufacturer.is_none()
        || request.hardware_type.is_empty()
    {
        out.acquisition.stop_reason = Some("identity-assertion-missing");
        return out;
    }
    if cancel.load(Ordering::SeqCst) {
        out.acquisition.stop_reason = Some("cancelled");
        return out;
    }
    let mut session = match ManagementSession::read_only(
        transport,
        target.address(),
        AuthorisationPlan::Skip,
        timing,
    ) {
        Ok(s) => s,
        Err(e) => {
            out.acquisition.stop_reason = Some(failure(e));
            return out;
        }
    };
    let deadline = tokio::time::Instant::now() + MEMORY_DEADLINE;
    match bounded(cancel, deadline, session.connect()).await {
        Err(reason) => out.acquisition.stop_reason = Some(reason),
        Ok(()) => {
            let memory = &out.memory;
            identity_reads(
                &mut session,
                &mut out.acquisition,
                request.expected,
                cancel,
                deadline,
                &mut |items| observe(items, memory),
            )
            .await;
            let items = &out.acquisition.items;
            if out.acquisition.stop_reason.is_none() {
                if items.iter().any(|item| item.status != Status::Read) {
                    out.acquisition.stop_reason = Some("identity-unverified");
                } else if !property_matches(&items[2].bytes, request.hardware_type) {
                    out.acquisition.stop_reason = Some("identity-mismatch");
                }
            }
            if out.acquisition.stop_reason.is_none() {
                out.acquisition.stop_reason =
                    memory_reads(&mut session, &mut out, cancel, deadline, &mut observe).await;
            }
        }
    }
    out.acquisition.disconnect_attempt_completed = teardown(&mut session).await;
    observe(&out.acquisition.items, &out.memory);
    out
}
/// The chunks in plan order; returns the stop reason, if any.
async fn memory_reads<T: ManagementTransport>(
    session: &mut ManagementSession<'_, T>,
    out: &mut MemoryAcquisition,
    cancel: &AtomicBool,
    deadline: tokio::time::Instant,
    observe: &mut impl FnMut(&[Item], &[MemoryItem]),
) -> Option<&'static str> {
    let mut ended_range = None;
    for index in 0..out.memory.len() {
        if ended_range == Some(out.memory[index].range) {
            continue;
        }
        if cancel.load(Ordering::SeqCst) {
            return Some("cancelled");
        }
        out.memory[index].attempted_at = Some(SystemTime::now());
        observe(&out.acquisition.items, &out.memory);
        let (address, requested) = (out.memory[index].address, out.memory[index].requested);
        let read = bounded(
            cancel,
            deadline,
            session.read_memory(u32::from(address), requested),
        )
        .await;
        let item = &mut out.memory[index];
        item.completed_at = Some(SystemTime::now());
        let mut stop = None;
        match read {
            Ok(bytes) if bytes.len() == usize::from(requested) => {
                item.bytes = bytes;
                item.status = Status::Read;
            }
            // The answer's length comes from one frame, far below MAX_RESPONSE_BYTES.
            Ok(bytes) => {
                item.bytes = bytes;
                item.status = Status::Partial;
                item.error = Some("unexpected-length");
                ended_range = Some(item.range);
            }
            Err("memory-refused") => {
                item.status = Status::Refused;
                item.error = Some("access-denied-or-memory-unavailable");
                ended_range = Some(item.range);
            }
            Err(reason) => {
                item.status = Status::Error;
                item.error = Some(reason);
                stop = Some(reason);
            }
        }
        observe(&out.acquisition.items, &out.memory);
        if stop.is_some() {
            return stop;
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn an_expired_deadline_must_not_poll_or_send_the_next_read() {
        let polled = AtomicBool::new(false);
        let result = bounded(
            &AtomicBool::new(false),
            tokio::time::Instant::now() - Duration::from_secs(1),
            async {
                polled.store(true, Ordering::SeqCst);
                Ok::<(), SessionError>(())
            },
        )
        .await;
        assert_eq!(result, Err("deadline"));
        assert!(!polled.load(Ordering::SeqCst));
    }
}
