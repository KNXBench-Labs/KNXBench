//! Retain bounded, per-server evidence for short bus actions.
//!
//! The guard records `unknown` if its request future is dropped without a
//! witnessed result; an async cancellation is not proof that the bus action
//! stopped before any frame. No keys, raw payloads or gateway are stored.
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use chrono::{SecondsFormat, Utc};
use serde::Serialize;

const MAX_COMPLETED: usize = 64;

/// Write-specific evidence is intentionally distinct from HTTP success. A
/// durable backup and a possible send are facts about the operation, not a
/// receipt from the device. Neither includes property bytes or a backup path.
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct WriteEvidence {
    backup_recorded: bool,
    send_possible: bool,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OneShotActivity {
    pub id: u64,
    pub kind: &'static str,
    /// None when an operation cannot know a physical address in advance.
    pub address: Option<String>,
    pub state: &'static str,
    pub started_at: String,
    pub finished_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    write_evidence: Option<WriteEvidence>,
}

#[derive(Default)]
struct Inner {
    next_id: u64,
    entries: VecDeque<OneShotActivity>,
    dropped_count: u64,
}

#[derive(Default)]
pub struct OneShotLog {
    inner: Mutex<Inner>,
}

fn now() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true)
}

impl OneShotLog {
    pub(crate) fn start(
        self: &Arc<Self>,
        kind: &'static str,
        address: Option<String>,
    ) -> ActionGuard {
        let id = self.start_entry(kind, address, None);
        ActionGuard {
            log: Arc::clone(self),
            id,
            done: false,
        }
    }

    pub(crate) fn start_write(
        self: &Arc<Self>,
        kind: &'static str,
        address: Option<String>,
    ) -> WriteGuard {
        let id = self.start_entry(
            kind,
            address,
            Some(WriteEvidence {
                backup_recorded: false,
                send_possible: false,
            }),
        );
        WriteGuard {
            log: Arc::clone(self),
            id,
            done: false,
        }
    }

    fn start_entry(
        &self,
        kind: &'static str,
        address: Option<String>,
        write_evidence: Option<WriteEvidence>,
    ) -> u64 {
        let mut inner = self.inner.lock().expect("activity log poisoned");
        let id = inner.next_id.checked_add(1).expect("activity id exhausted");
        inner.next_id = id;
        inner.entries.push_back(OneShotActivity {
            id,
            kind,
            address,
            state: "running",
            started_at: now(),
            finished_at: None,
            write_evidence,
        });
        prune(&mut inner);
        id
    }

    pub(crate) fn snapshot_with_dropped(&self) -> (Vec<OneShotActivity>, u64) {
        let inner = self.inner.lock().expect("activity log poisoned");
        (inner.entries.iter().cloned().collect(), inner.dropped_count)
    }

    fn mark_write_possible(&self, id: u64) {
        let mut inner = self.inner.lock().expect("activity log poisoned");
        let entry = inner
            .entries
            .iter_mut()
            .find(|entry| entry.id == id)
            .expect("running write activity retained");
        assert_eq!(entry.state, "running");
        let evidence = entry.write_evidence.as_mut().expect("write activity");
        evidence.backup_recorded = true;
        // The backup callback runs immediately before the transport write.
        // A later cancellation or transport error cannot prove non-delivery.
        evidence.send_possible = true;
    }

    fn write_possible(&self, id: u64) -> bool {
        let inner = self.inner.lock().expect("activity log poisoned");
        inner
            .entries
            .iter()
            .find(|entry| entry.id == id)
            .and_then(|entry| entry.write_evidence.as_ref())
            .expect("running write activity retained")
            .send_possible
    }

    fn finish(&self, id: u64, state: &'static str) {
        let mut inner = self.inner.lock().expect("activity log poisoned");
        if let Some(entry) = inner.entries.iter_mut().find(|entry| entry.id == id) {
            entry.state = state;
            entry.finished_at = Some(now());
        }
        prune(&mut inner);
    }
}

/// Never remove an active operation to make room for a completed one. Under
/// ordinary single-tunnel use the whole log is bounded; pathological numbers
/// of simultaneous active requests may temporarily exceed the retention cap.
fn prune(inner: &mut Inner) {
    while inner.entries.len() > MAX_COMPLETED {
        let Some(index) = inner
            .entries
            .iter()
            .position(|entry| entry.state != "running")
        else {
            break;
        };
        inner.entries.remove(index);
        inner.dropped_count = inner.dropped_count.saturating_add(1);
    }
}

pub(crate) struct ActionGuard {
    log: Arc<OneShotLog>,
    id: u64,
    done: bool,
}

impl ActionGuard {
    pub(crate) fn finish(mut self, state: &'static str) {
        self.log.finish(self.id, state);
        self.done = true;
    }
}

impl Drop for ActionGuard {
    fn drop(&mut self) {
        if !self.done {
            self.log.finish(self.id, "unknown");
        }
    }
}

/// A write is verified only after its route-specific readback. The guard is
/// deliberately not interchangeable with the read-only `ActionGuard`.
#[derive(Clone, Copy)]
pub(crate) enum WriteOutcome {
    Verified,
    NoChange,
    NotSent,
    EffectUnverified,
}

pub(crate) struct WriteGuard {
    log: Arc<OneShotLog>,
    id: u64,
    done: bool,
}

impl WriteGuard {
    /// Call only after the pre-write backup callback has durably persisted
    /// and read back the original property, immediately before the send.
    pub(crate) fn mark_send_possible(&self) {
        self.log.mark_write_possible(self.id);
    }

    pub(crate) fn send_possible(&self) -> bool {
        self.log.write_possible(self.id)
    }

    pub(crate) fn finish(mut self, outcome: WriteOutcome) {
        let possible = self.send_possible();
        let state = match outcome {
            WriteOutcome::Verified if possible => "verified",
            WriteOutcome::NoChange if !possible => "noChange",
            WriteOutcome::NotSent if !possible => "notSent",
            WriteOutcome::EffectUnverified if possible => "effectUnverified",
            _ => panic!("write evidence does not support the claimed outcome"),
        };
        self.log.finish(self.id, state);
        self.done = true;
    }
}

impl Drop for WriteGuard {
    fn drop(&mut self) {
        if !self.done {
            self.log.finish(self.id, "unknown");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_dropped_action_is_unknown_and_a_witnessed_result_is_not() {
        let log = Arc::new(OneShotLog::default());
        let abandoned = log.start("deviceCompare", Some("1.1.67".to_string()));
        assert_eq!(log.snapshot_with_dropped().0[0].state, "running");
        drop(abandoned);
        assert_eq!(log.snapshot_with_dropped().0[0].state, "unknown");
        assert!(log.snapshot_with_dropped().0[0].finished_at.is_some());
        let completed = log.start("deviceCompare", Some("1.1.67".to_string()));
        completed.finish("finished");
        assert_eq!(log.snapshot_with_dropped().0[1].state, "finished");
        assert_eq!(log.snapshot_with_dropped().0[1].id, 2);
    }

    #[test]
    fn write_guard_cannot_claim_verified_without_a_recorded_backup_boundary() {
        let log = Arc::new(OneShotLog::default());
        let guard = log.start_write("serviceControlWrite", Some("1.1.67".to_string()));
        let false_receipt = std::panic::catch_unwind(|| guard.finish(WriteOutcome::Verified));
        assert!(false_receipt.is_err());
        let entry = &log.snapshot_with_dropped().0[0];
        assert_eq!(entry.state, "unknown");
        assert!(!entry.write_evidence.as_ref().unwrap().backup_recorded);
        assert!(!entry.write_evidence.as_ref().unwrap().send_possible);
    }

    #[test]
    fn abandoned_write_retains_the_observed_prewrite_boundary() {
        let log = Arc::new(OneShotLog::default());
        let guard = log.start_write("serviceControlWrite", Some("1.1.67".to_string()));
        guard.mark_send_possible();
        drop(guard);
        let entry = &log.snapshot_with_dropped().0[0];
        assert_eq!(entry.state, "unknown");
        assert!(entry.write_evidence.as_ref().unwrap().backup_recorded);
        assert!(entry.write_evidence.as_ref().unwrap().send_possible);
    }

    #[test]
    fn retention_never_evicts_an_in_flight_action() {
        let log = Arc::new(OneShotLog::default());
        let live = log.start("deviceCompare", Some("1.1.67".to_string()));
        for _ in 0..65 {
            log.start("deviceCompare", Some("1.1.67".to_string()))
                .finish("failed");
        }
        let before = log.snapshot_with_dropped().0;
        assert_eq!(before.len(), MAX_COMPLETED);
        assert_eq!(log.snapshot_with_dropped().1, 2);
        assert_eq!(before[0].id, 1);
        assert_eq!(before[0].state, "running");
        live.finish("finished");
        assert_eq!(log.snapshot_with_dropped().0.len(), MAX_COMPLETED);
    }
}
