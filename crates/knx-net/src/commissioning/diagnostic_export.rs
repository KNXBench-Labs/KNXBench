//! Bounded, read-only, manufacturer-independent device identity acquisition.
//!
//! No memory profile is admitted here: source offsets, mask families and executable download plans
//! confer no authority to read configuration or key material.
use std::future::Future;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, SystemTime};

use crate::{ManagementSession, ManagementTransport, SessionError, SessionTiming};
use knx_core::commissioning::authorisation::AuthorisationPlan;
use knx_core::commissioning::properties::{ObjectIndex, PID_HARDWARE_TYPE, PID_MANUFACTURER_ID};
use knx_core::ContactableAddress;

pub const DEADLINE: Duration = Duration::from_secs(30);
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
    let mut out = Acquisition {
        items: [Query::Descriptor, Query::Manufacturer, Query::HardwareType]
            .into_iter()
            .map(Item::pending)
            .collect(),
        stop_reason: None,
        disconnect_attempt_completed: false,
    };
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
    }
    // Always attempt device-side teardown, even if connect was cancelled after
    // sending T_Connect. This is separate from the caller's tunnel disconnect.
    out.disconnect_attempt_completed =
        tokio::time::timeout(Duration::from_secs(3), session.disconnect())
            .await
            .is_ok();
    observe(&out.items);
    out
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
