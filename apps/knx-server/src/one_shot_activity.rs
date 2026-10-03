//! Retain short-operation metadata in a volatile ring and durable history.
//!
//! The guard records `unknown` if its request future is dropped without a
//! witnessed result; an async cancellation is not proof that the bus action
//! stopped before any frame. No keys, raw payloads or gateway are stored.
use std::collections::VecDeque;
use std::io;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use chrono::{SecondsFormat, Utc};
use serde::{Deserialize, Serialize};

const MAX_COMPLETED: usize = 64;

/// Write-specific evidence is intentionally distinct from HTTP success. A
/// durable backup and a possible send are facts about the operation, not a
/// receipt from the device. Neither includes property bytes or a backup path.
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct WriteEvidence {
    backup_recorded: bool,
    send_possible: bool,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OneShotActivity {
    pub id: u64,
    pub kind: String,
    /// None when an operation cannot know a physical address in advance.
    pub address: Option<String>,
    pub state: String,
    pub started_at: String,
    pub finished_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    write_evidence: Option<WriteEvidence>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct HistoryActivity {
    pub sequence: u64,
    pub server_incarnation: String,
    pub interrupted: bool,
    #[serde(flatten)]
    pub activity: OneShotActivity,
}

impl OneShotActivity {
    fn validate(&self, operation_id: u64) -> io::Result<()> {
        let write = self.kind == "serviceControlWrite";
        let valid_kind = matches!(
            self.kind.as_str(),
            "deviceCompare" | "serviceControlRead" | "serviceControlWrite" | "serialLookup"
        );
        let valid_address = match &self.address {
            Some(address) => {
                address.parse::<knx_core::IndividualAddress>().is_ok()
                    && self.kind != "serialLookup"
            }
            None => self.kind == "serialLookup",
        };
        let valid_state = if write {
            match (self.state.as_str(), &self.write_evidence) {
                ("verified" | "effectUnverified", Some(evidence)) => {
                    evidence.backup_recorded && evidence.send_possible
                }
                ("notSent" | "noChange", Some(evidence)) => !evidence.send_possible,
                ("running" | "unknown", Some(evidence)) => {
                    !evidence.send_possible || evidence.backup_recorded
                }
                _ => false,
            }
        } else {
            self.write_evidence.is_none()
                && matches!(
                    self.state.as_str(),
                    "running" | "finished" | "failed" | "unknown"
                )
        };
        let start = chrono::DateTime::parse_from_rfc3339(&self.started_at).ok();
        let valid_finish = match &self.finished_at {
            Some(value) => {
                self.state != "running" && chrono::DateTime::parse_from_rfc3339(value).is_ok()
            }
            None => self.state == "running",
        };
        if self.id != operation_id
            || self.id == 0
            || !valid_kind
            || !valid_address
            || !valid_state
            || start.is_none()
            || !valid_finish
        {
            return Err(io::Error::other("invalid activity metadata"));
        }
        Ok(())
    }
}

#[derive(Default)]
struct Inner {
    next_id: u64,
    entries: VecDeque<OneShotActivity>,
    dropped_count: u64,
    history_failed: bool,
}

#[derive(Default)]
pub struct OneShotLog {
    inner: Mutex<Inner>,
    history_path: Option<PathBuf>,
    // Serialize first-open schema admission. Release this lock before taking
    // `inner` on a read/admission error; writers already hold `inner` first.
    history_initialized: Mutex<bool>,
    incarnation: String,
}

fn now() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true)
}

impl OneShotLog {
    pub(crate) fn persistent(history_path: PathBuf, incarnation: String) -> Self {
        Self {
            history_path: Some(history_path),
            incarnation,
            ..Self::default()
        }
    }

    pub(crate) fn history_page(
        &self,
        after: u64,
        limit: usize,
    ) -> io::Result<(Vec<HistoryActivity>, bool)> {
        if self
            .inner
            .lock()
            .expect("activity log poisoned")
            .history_failed
        {
            return Err(io::Error::other("activity history recording failed"));
        }
        let result = self.read_history_page(after, limit);
        if result.is_err() {
            self.inner
                .lock()
                .expect("activity log poisoned")
                .history_failed = true;
        }
        result
    }

    fn open_history(&self) -> io::Result<knx_store::activity_history::ActivityHistory> {
        let path = self
            .history_path
            .as_ref()
            .ok_or_else(|| io::Error::other("activity history is not configured"))?;
        let mut initialized = self
            .history_initialized
            .lock()
            .expect("activity initialization poisoned");
        let history = if *initialized {
            knx_store::activity_history::ActivityHistory::open_existing(path)?
        } else {
            knx_store::activity_history::ActivityHistory::open(path)?
        };
        *initialized = true;
        Ok(history)
    }

    fn read_history_page(
        &self,
        after: u64,
        limit: usize,
    ) -> io::Result<(Vec<HistoryActivity>, bool)> {
        let history = self.open_history()?;
        let (rows, more) = history.page(after, limit)?;
        let mut entries = Vec::new();
        for row in rows {
            let mut entry: OneShotActivity = serde_json::from_str(&row.document)
                .map_err(|_| io::Error::other("invalid activity metadata"))?;
            entry.validate(row.operation_id)?;
            if !row
                .incarnation
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
            {
                return Err(io::Error::other("invalid activity identity"));
            }
            let interrupted = row.incarnation != self.incarnation && entry.state == "running";
            if interrupted {
                entry.state = "unknown".into();
            }
            entries.push(HistoryActivity {
                sequence: row.sequence,
                server_incarnation: row.incarnation,
                interrupted,
                activity: entry,
            });
        }
        Ok((entries, more))
    }

    fn persist(&self, entry: &OneShotActivity) -> io::Result<()> {
        if self.history_path.is_none() {
            return Ok(());
        }
        let document = serde_json::to_string(entry).map_err(io::Error::other)?;
        self.open_history()?
            .record(&self.incarnation, entry.id, &document)
    }

    pub(crate) fn history_state(&self) -> &'static str {
        if self.history_path.is_none() {
            "disabled"
        } else if self
            .inner
            .lock()
            .expect("activity log poisoned")
            .history_failed
        {
            "unavailable"
        } else {
            "configured"
        }
    }

    /// Storage admission is not a journal entry or a completed bus operation.
    pub(crate) fn ensure_write_available(&self) -> io::Result<()> {
        let failed = self
            .inner
            .lock()
            .expect("activity log poisoned")
            .history_failed;
        if self.history_path.is_none() || failed {
            return Err(io::Error::other(
                "activity history is unavailable; not sent",
            ));
        }
        if let Err(error) = self.open_history() {
            self.inner
                .lock()
                .expect("activity log poisoned")
                .history_failed = true;
            return Err(error);
        }
        Ok(())
    }

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
    ) -> io::Result<WriteGuard> {
        let id = self.start_entry(
            kind,
            address,
            Some(WriteEvidence {
                backup_recorded: false,
                send_possible: false,
            }),
        );
        let failed = self
            .inner
            .lock()
            .expect("activity log poisoned")
            .history_failed;
        if failed {
            self.finish(id, "notSent");
            return Err(io::Error::other(
                "activity history is unavailable; not sent",
            ));
        }
        Ok(WriteGuard {
            log: Arc::clone(self),
            id,
            done: false,
        })
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
            kind: kind.into(),
            address,
            state: "running".into(),
            started_at: now(),
            finished_at: None,
            write_evidence,
        });
        if inner.history_failed
            || self
                .persist(inner.entries.back().expect("new activity"))
                .is_err()
        {
            inner.history_failed = true;
        }
        prune(&mut inner);
        id
    }

    pub(crate) fn snapshot_with_dropped(&self) -> (Vec<OneShotActivity>, u64) {
        let inner = self.inner.lock().expect("activity log poisoned");
        (inner.entries.iter().cloned().collect(), inner.dropped_count)
    }

    fn mark_write_possible(&self, id: u64) -> io::Result<()> {
        let mut inner = self.inner.lock().expect("activity log poisoned");
        let history_failed = inner.history_failed;
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
        let mut candidate = entry.clone();
        candidate
            .write_evidence
            .as_mut()
            .expect("write activity")
            .send_possible = true;
        if history_failed || self.persist(&candidate).is_err() {
            inner.history_failed = true;
            return Err(io::Error::other(
                "activity write intent could not be persisted; not sent",
            ));
        }
        *entry = candidate;
        Ok(())
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
        let history_failed = inner.history_failed;
        if let Some(entry) = inner.entries.iter_mut().find(|entry| entry.id == id) {
            entry.state = state.into();
            entry.finished_at = Some(now());
            if !history_failed && self.persist(entry).is_err() {
                inner.history_failed = true;
            }
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
    pub(crate) fn mark_send_possible(&self) -> io::Result<()> {
        self.log.mark_write_possible(self.id)
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
    fn invalid_persisted_metadata_is_refused_not_rendered() {
        for field in [
            "kind",
            "state",
            "address",
            "startedAt",
            "finishedAt",
            "id",
            "futureField",
        ] {
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("activity.sqlite");
            let log = Arc::new(OneShotLog::persistent(
                path.clone(),
                "AAAAAAAAAAAAAAAAAAAAAA".into(),
            ));
            log.start("deviceCompare", Some("1.1.67".into()))
                .finish("finished");
            let mut document = serde_json::to_value(&log.snapshot_with_dropped().0[0]).unwrap();
            document[field] = if field == "id" {
                serde_json::json!(2)
            } else {
                serde_json::json!("unsupported")
            };
            knx_store::activity_history::ActivityHistory::open(&path)
                .unwrap()
                .record("AAAAAAAAAAAAAAAAAAAAAA", 1, &document.to_string())
                .unwrap();
            assert!(
                log.history_page(0, 100).is_err(),
                "invalid {field} was rendered"
            );
        }
    }

    #[test]
    fn invalid_write_receipts_are_not_accepted() {
        for evidence in [
            serde_json::json!({"backupRecorded": false, "sendPossible": true}),
            serde_json::json!({"backupRecorded": true, "sendPossible": false}),
            serde_json::json!({"backupRecorded": true, "sendPossible": true, "futureField": true}),
        ] {
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("activity.sqlite");
            let log = Arc::new(OneShotLog::persistent(path.clone(), "first".into()));
            let guard = log
                .start_write("serviceControlWrite", Some("1.1.67".into()))
                .unwrap();
            guard.mark_send_possible().unwrap();
            guard.finish(WriteOutcome::Verified);
            let mut document = serde_json::to_value(&log.snapshot_with_dropped().0[0]).unwrap();
            document["writeEvidence"] = evidence;
            knx_store::activity_history::ActivityHistory::open(&path)
                .unwrap()
                .record("first", 1, &document.to_string())
                .unwrap();
            assert!(log.history_page(0, 100).is_err());
        }
    }

    #[test]
    fn persisted_history_outlives_the_volatile_ring_and_server() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("activity.sqlite");
        let log = Arc::new(OneShotLog::persistent(
            path.clone(),
            "AAAAAAAAAAAAAAAAAAAAAA".into(),
        ));
        for _ in 0..67 {
            log.start("deviceCompare", Some("1.1.67".into()))
                .finish("finished");
        }
        assert_eq!(log.snapshot_with_dropped().0.len(), MAX_COMPLETED);
        drop(log);
        let restarted = OneShotLog::persistent(path, "BBBBBBBBBBBBBBBBBBBBBB".into());
        let (entries, more) = restarted.history_page(0, 100).unwrap();
        assert!(!more);
        assert_eq!(entries.len(), 67);
        assert!(entries
            .iter()
            .all(|entry| entry.activity.state == "finished"));
        assert_eq!(entries[0].activity.id, 1);
        assert_eq!(entries[66].activity.id, 67);
    }

    #[test]
    fn concurrent_first_history_reads_do_not_latch_a_spurious_storage_failure() {
        for _ in 0..8 {
            let dir = tempfile::tempdir().unwrap();
            let log = Arc::new(OneShotLog::persistent(
                dir.path().join("activity.sqlite"),
                "first".into(),
            ));
            let barrier = Arc::new(std::sync::Barrier::new(16));
            let handles: Vec<_> = (0..16)
                .map(|_| {
                    let log = Arc::clone(&log);
                    let barrier = Arc::clone(&barrier);
                    std::thread::spawn(move || {
                        barrier.wait();
                        log.history_page(0, 100)
                    })
                })
                .collect();
            for handle in handles {
                assert!(handle.join().unwrap().is_ok());
            }
            assert_eq!(log.history_state(), "configured");
            assert!(log.ensure_write_available().is_ok());
        }
    }

    #[test]
    fn previous_incarnation_running_is_unknown_without_rewriting_the_record() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("activity.sqlite");
        let log = Arc::new(OneShotLog::persistent(
            path.clone(),
            "AAAAAAAAAAAAAAAAAAAAAA".into(),
        ));
        let guard = log
            .start_write("serviceControlWrite", Some("1.1.67".into()))
            .unwrap();
        guard.mark_send_possible().unwrap();
        let before = std::fs::read(&path).unwrap();
        let reopened = OneShotLog::persistent(path.clone(), "BBBBBBBBBBBBBBBBBBBBBB".into());
        let entries = reopened.history_page(0, 100).unwrap().0;
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].activity.state, "unknown");
        assert!(entries[0].interrupted);
        assert!(entries[0].activity.finished_at.is_none());
        assert!(
            entries[0]
                .activity
                .write_evidence
                .as_ref()
                .unwrap()
                .send_possible
        );
        assert_eq!(std::fs::read(&path).unwrap(), before);
        drop(guard);
    }

    #[test]
    fn removed_history_is_not_silently_recreated_during_the_server_lifetime() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("activity.sqlite");
        let log = Arc::new(OneShotLog::persistent(path.clone(), "first".into()));
        log.start("deviceCompare", Some("1.1.67".into()))
            .finish("finished");
        std::fs::remove_file(&path).unwrap();
        assert!(log
            .start_write("serviceControlWrite", Some("1.1.67".into()))
            .is_err());
        assert!(!path.exists());
        assert_eq!(log.history_state(), "unavailable");
    }

    #[test]
    fn observed_history_corruption_blocks_intent_and_is_not_overwritten_on_finish() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("activity.sqlite");
        let log = Arc::new(OneShotLog::persistent(path.clone(), "first".into()));
        let guard = log
            .start_write("serviceControlWrite", Some("1.1.67".into()))
            .unwrap();
        knx_store::activity_history::ActivityHistory::open(&path)
            .unwrap()
            .record("first", 1, "invalid metadata")
            .unwrap();
        assert!(log.history_page(0, 100).is_err());
        let before = std::fs::read(&path).unwrap();
        assert!(guard.mark_send_possible().is_err());
        assert!(!guard.send_possible());
        guard.finish(WriteOutcome::NotSent);
        assert_eq!(std::fs::read(&path).unwrap(), before);
    }

    #[test]
    fn invalid_history_read_latches_write_refusal() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("activity.sqlite");
        let log = Arc::new(OneShotLog::persistent(path.clone(), "first".into()));
        log.start("deviceCompare", Some("1.1.67".into()))
            .finish("finished");
        knx_store::activity_history::ActivityHistory::open(&path)
            .unwrap()
            .record("first", 1, "invalid metadata")
            .unwrap();
        assert!(log.history_page(0, 100).is_err());
        assert_eq!(log.history_state(), "unavailable");
        assert!(log
            .start_write("serviceControlWrite", Some("1.1.67".into()))
            .is_err());
    }

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
        let guard = log
            .start_write("serviceControlWrite", Some("1.1.67".to_string()))
            .unwrap();
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
        let guard = log
            .start_write("serviceControlWrite", Some("1.1.67".to_string()))
            .unwrap();
        guard.mark_send_possible().unwrap();
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
