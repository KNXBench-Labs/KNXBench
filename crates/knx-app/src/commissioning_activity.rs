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

/// Witnessed device-write classification, independent of transport and UI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Written {
    Yes,
    No,
    Partially,
}

/// Closing-restart classification, never inferred from successful data writes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Restart {
    Acknowledged,
    NotInPlan,
    Unconfirmed,
}

/// Payload-free worker outcome; adapter errors and progress stay with the caller.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DownloadResult {
    Running,
    Finished { written: Written, restart: Restart },
    Failed { written: Written },
}

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

/// Metadata-only correlation and witnessed outcomes, never recovery payloads.
/// None is an unwitnessed device/restart outcome, not proof of no write.
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct DownloadEvidence {
    session_id: u64,
    written: Option<Written>,
    restart: Option<Restart>,
    cleanup: DownloadCleanup,
}

/// Describes the adapter return only, not a KNX disconnect acknowledgment.
#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
enum DownloadCleanup {
    Pending,
    ReturnedOk,
    ReturnedError,
    Unknown,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    download_evidence: Option<DownloadEvidence>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryActivity {
    pub sequence: u64,
    pub server_incarnation: String,
    pub interrupted: bool,
    #[serde(flatten)]
    pub activity: OneShotActivity,
}

impl OneShotActivity {
    fn validate(&self, operation_id: u64) -> io::Result<()> {
        let write = self.kind == "serviceControlWrite";
        let download = self.kind == "deviceDownload";
        let valid_kind = matches!(
            self.kind.as_str(),
            "deviceCompare"
                | "serviceControlRead"
                | "serviceControlWrite"
                | "serialLookup"
                | "deviceDownload"
        );
        let valid_address = match &self.address {
            Some(address) => {
                address.parse::<knx_core::IndividualAddress>().is_ok()
                    && self.kind != "serialLookup"
            }
            None => self.kind == "serialLookup",
        };
        let valid_state = if download {
            match (&self.write_evidence, &self.download_evidence) {
                (Some(write), Some(evidence)) => {
                    evidence.session_id > 0
                        && (!write.send_possible || write.backup_recorded)
                        && (evidence.written.is_none()
                            || evidence.written == Some(Written::No)
                            || (write.backup_recorded && write.send_possible))
                        && match self.state.as_str() {
                            "running" => {
                                evidence.written.is_none()
                                    && evidence.restart.is_none()
                                    && evidence.cleanup == DownloadCleanup::Pending
                            }
                            "finished" => evidence.written.is_some() && evidence.restart.is_some(),
                            "failed" => evidence.written.is_some() && evidence.restart.is_none(),
                            "unknown" => {
                                evidence.written.is_none()
                                    && evidence.restart.is_none()
                                    && evidence.cleanup == DownloadCleanup::Unknown
                            }
                            _ => false,
                        }
                }
                _ => false,
            }
        } else if write {
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
            || (!download && self.download_evidence.is_some())
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
    /// Create an independent run namespace without opening the store.
    /// Randomness failure refuses rather than reusing a prior identity.
    pub fn for_run(history_path: PathBuf) -> io::Result<Self> {
        let mut bytes = [0_u8; 16];
        getrandom::fill(&mut bytes)
            .map_err(|_| io::Error::other("OS randomness unavailable for activity correlation"))?;
        let incarnation =
            base64::Engine::encode(&base64::engine::general_purpose::URL_SAFE_NO_PAD, bytes);
        Ok(Self::persistent(history_path, incarnation))
    }
    pub fn persistent(history_path: PathBuf, incarnation: String) -> Self {
        Self {
            history_path: Some(history_path),
            incarnation,
            ..Self::default()
        }
    }

    pub fn history_page(
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
            let pending_cleanup = entry
                .download_evidence
                .as_ref()
                .is_some_and(|evidence| evidence.cleanup == DownloadCleanup::Pending);
            let interrupted = row.incarnation != self.incarnation
                && (entry.state == "running" || pending_cleanup);
            if interrupted {
                if entry.state == "running" {
                    entry.state = "unknown".into();
                }
                if let Some(evidence) = entry.download_evidence.as_mut() {
                    if evidence.cleanup == DownloadCleanup::Pending {
                        evidence.cleanup = DownloadCleanup::Unknown;
                    }
                }
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
        entry.validate(entry.id)?;
        let document = serde_json::to_string(entry).map_err(io::Error::other)?;
        self.open_history()?
            .record(&self.incarnation, entry.id, &document)
    }

    pub fn history_state(&self) -> &'static str {
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
    pub fn ensure_write_available(&self) -> io::Result<()> {
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

    pub fn start(self: &Arc<Self>, kind: &'static str, address: Option<String>) -> ActionGuard {
        let id = self.start_entry(kind, address, None);
        ActionGuard {
            log: Arc::clone(self),
            id,
            done: false,
        }
    }

    /// Admit and persist before the caller asks its connector for a tunnel.
    /// A long download owns its row separately from the bounded one-shot ring.
    pub fn start_download(
        self: &Arc<Self>,
        session_id: u64,
        address: knx_core::IndividualAddress,
    ) -> io::Result<DownloadGuard> {
        self.ensure_write_available()?;
        let id = {
            let mut inner = self.inner.lock().expect("activity log poisoned");
            let id = inner
                .next_id
                .checked_add(1)
                .ok_or_else(|| io::Error::other("activity id exhausted"))?;
            inner.next_id = id;
            id
        };
        let entry = OneShotActivity {
            id,
            kind: "deviceDownload".into(),
            address: Some(address.to_string()),
            state: "running".into(),
            started_at: now(),
            finished_at: None,
            write_evidence: Some(WriteEvidence {
                backup_recorded: false,
                send_possible: false,
            }),
            download_evidence: Some(DownloadEvidence {
                session_id,
                written: None,
                restart: None,
                cleanup: DownloadCleanup::Pending,
            }),
        };
        self.persist_download(&entry)?;
        Ok(DownloadGuard {
            log: Arc::clone(self),
            entry,
            done: false,
        })
    }

    fn persist_download(&self, entry: &OneShotActivity) -> io::Result<()> {
        let mut inner = self.inner.lock().expect("activity log poisoned");
        if inner.history_failed {
            return Err(io::Error::other("activity history is unavailable"));
        }
        if let Err(error) = self.persist(entry) {
            inner.history_failed = true;
            return Err(error);
        }
        Ok(())
    }

    pub fn start_write(
        self: &Arc<Self>,
        kind: &'static str,
        address: Option<String>,
    ) -> io::Result<WriteGuard> {
        if self.history_path.is_none() {
            return Err(io::Error::other(
                "activity history is unavailable; not sent",
            ));
        }
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
            download_evidence: None,
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

    pub fn snapshot_with_dropped(&self) -> (Vec<OneShotActivity>, u64) {
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

/// Owns bounded metadata across adapter acquisition and worker cleanup.
/// A drop is uncertainty, never a no-write or successful-disconnect receipt.
pub struct DownloadGuard {
    log: Arc<OneShotLog>,
    entry: OneShotActivity,
    done: bool,
}

impl DownloadGuard {
    /// Called by the keeper only after the original recovery image is retained.
    /// A recording error is propagated through that keeper before any mutation.
    pub fn mark_send_possible(&mut self) -> io::Result<()> {
        if self.entry.state != "running" {
            return Err(io::Error::other(
                "download intent cannot follow a terminal result",
            ));
        }
        let mut candidate = self.entry.clone();
        candidate.write_evidence = Some(WriteEvidence {
            backup_recorded: true,
            send_possible: true,
        });
        self.log.persist_download(&candidate)?;
        self.entry = candidate;
        Ok(())
    }

    /// Record the worker's classification, never its error strings or payloads.
    /// Keep the witnessed result in memory even if durable recording fails.
    pub fn record_result(&mut self, status: &DownloadResult) -> io::Result<()> {
        if self.entry.state != "running" {
            return Err(io::Error::other("download result was already recorded"));
        }
        use DownloadResult;
        let (state, written, restart) = match status {
            DownloadResult::Finished {
                written, restart, ..
            } => ("finished", *written, Some(*restart)),
            DownloadResult::Failed { written, .. } => ("failed", *written, None),
            DownloadResult::Running => return Err(io::Error::other("nonterminal download result")),
        };
        self.entry.state = state.into();
        self.entry.finished_at = Some(now());
        let evidence = self
            .entry
            .download_evidence
            .as_mut()
            .expect("download evidence");
        evidence.written = Some(written);
        evidence.restart = restart;
        self.log.persist_download(&self.entry)
    }

    /// The tunnel never opened, so nothing could have been sent (AR18 review
    /// M6). Recorded as `failed` with `written: no` instead of letting the
    /// drop say `unknown`, which told the user a write might have happened.
    /// `cleanup: returnedError` is the adapter's own return: acquiring it
    /// failed. Refused once a send was possible: from then on only the
    /// worker's witnessed result may close the entry.
    pub fn record_never_connected(mut self) -> io::Result<()> {
        let send_possible = self
            .entry
            .write_evidence
            .as_ref()
            .is_some_and(|evidence| evidence.send_possible);
        if self.entry.state != "running" || send_possible {
            return Err(io::Error::other(
                "only a download that could not have sent anything is recorded as never connected",
            ));
        }
        let mut candidate = self.entry.clone();
        candidate.state = "failed".into();
        candidate.finished_at = Some(now());
        let evidence = candidate
            .download_evidence
            .as_mut()
            .expect("download evidence");
        evidence.written = Some(Written::No);
        evidence.restart = None;
        evidence.cleanup = DownloadCleanup::ReturnedError;
        self.log.persist_download(&candidate)?;
        self.entry = candidate;
        self.done = true;
        Ok(())
    }

    /// This is the adapter's return, not a device-level disconnect receipt.
    pub fn record_cleanup(mut self, returned_ok: bool) -> io::Result<()> {
        self.entry
            .download_evidence
            .as_mut()
            .expect("download evidence")
            .cleanup = if returned_ok {
            DownloadCleanup::ReturnedOk
        } else {
            DownloadCleanup::ReturnedError
        };
        let result = self.log.persist_download(&self.entry);
        self.done = true;
        result
    }
}

impl Drop for DownloadGuard {
    fn drop(&mut self) {
        if !self.done {
            if self.entry.state == "running" {
                self.entry.state = "unknown".into();
                self.entry.finished_at = Some(now());
            }
            self.entry
                .download_evidence
                .as_mut()
                .expect("download evidence")
                .cleanup = DownloadCleanup::Unknown;
            let _ = self.log.persist_download(&self.entry);
        }
    }
}

pub struct ActionGuard {
    log: Arc<OneShotLog>,
    id: u64,
    done: bool,
}

impl ActionGuard {
    pub fn finish(mut self, state: &'static str) {
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
pub enum WriteOutcome {
    Verified,
    NoChange,
    NotSent,
    EffectUnverified,
}

pub struct WriteGuard {
    log: Arc<OneShotLog>,
    id: u64,
    done: bool,
}

impl WriteGuard {
    /// Call only after the pre-write backup callback has durably persisted
    /// and read back the original property, immediately before the send.
    pub fn mark_send_possible(&self) -> io::Result<()> {
        self.log.mark_write_possible(self.id)
    }

    pub fn send_possible(&self) -> bool {
        self.log.write_possible(self.id)
    }

    pub fn finish(mut self, outcome: WriteOutcome) {
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
    fn download_guard_cannot_replace_a_witnessed_terminal_result() {
        use super::{DownloadResult, Restart, Written};

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("activity.sqlite");
        let log = Arc::new(OneShotLog::persistent(path.clone(), "synthetic".into()));
        let mut guard = log.start_download(9, "1.1.1".parse().unwrap()).unwrap();
        guard.mark_send_possible().unwrap();
        guard
            .record_result(&DownloadResult::Finished {
                written: Written::Yes,
                restart: Restart::Acknowledged,
            })
            .unwrap();
        let history = knx_store::activity_history::ActivityHistory::open_existing(&path).unwrap();
        let original = history.page(0, 100).unwrap().0;
        assert_eq!(original.len(), 1);

        let replacement = guard.record_result(&DownloadResult::Failed {
            written: Written::No,
        });

        assert!(
            replacement.is_err(),
            "second terminal outcome replaced witnessed download result"
        );
        let retained = history.page(0, 100).unwrap().0;
        assert_eq!(retained.len(), 1);
        assert_eq!(retained[0].document, original[0].document);
        assert_eq!(retained[0].sequence, original[0].sequence);
        assert_eq!(log.history_state(), "configured");
        assert!(log.ensure_write_available().is_ok());
        guard.record_cleanup(true).unwrap();
        let (entries, more) = log.history_page(0, 100).unwrap();
        assert!(!more);
        assert_eq!(entries.len(), 1);
        let value = serde_json::to_value(&entries[0]).unwrap();
        assert_eq!(value["state"], "finished");
        assert_eq!(value["downloadEvidence"]["written"], "yes");
        assert_eq!(value["downloadEvidence"]["restart"], "acknowledged");
        assert_eq!(value["downloadEvidence"]["cleanup"], "returnedOk");
    }

    #[test]
    fn a_tunnel_that_never_opened_is_a_failed_download_that_wrote_nothing() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("activity.sqlite");
        let log = Arc::new(OneShotLog::persistent(path, "synthetic".into()));
        let guard = log.start_download(12, "1.1.1".parse().unwrap()).unwrap();
        guard.record_never_connected().unwrap();
        let (entries, _) = log.history_page(0, 100).unwrap();
        let value = serde_json::to_value(&entries[0]).unwrap();
        assert_eq!(value["state"], "failed");
        assert_eq!(value["downloadEvidence"]["written"], "no");
        assert!(value["downloadEvidence"]["restart"].is_null());
        assert_eq!(value["downloadEvidence"]["cleanup"], "returnedError");
        assert_eq!(value["writeEvidence"]["sendPossible"], false);
        assert!(!value["finishedAt"].is_null());
    }

    #[test]
    fn never_connected_is_refused_once_a_send_was_possible() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("activity.sqlite");
        let log = Arc::new(OneShotLog::persistent(path, "synthetic".into()));
        let mut guard = log.start_download(13, "1.1.1".parse().unwrap()).unwrap();
        guard.mark_send_possible().unwrap();
        assert!(guard.record_never_connected().is_err());
        // The refused guard dropped: uncertainty, as before.
        let (entries, _) = log.history_page(0, 100).unwrap();
        let value = serde_json::to_value(&entries[0]).unwrap();
        assert_eq!(value["state"], "unknown");
        assert!(value["downloadEvidence"]["written"].is_null());
    }

    #[test]
    fn download_guard_cannot_mark_send_possible_after_terminal_result() {
        use super::{DownloadResult, Written};

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("activity.sqlite");
        let log = Arc::new(OneShotLog::persistent(path.clone(), "synthetic".into()));
        let mut guard = log.start_download(11, "1.1.1".parse().unwrap()).unwrap();
        guard
            .record_result(&DownloadResult::Failed {
                written: Written::No,
            })
            .unwrap();
        let history = knx_store::activity_history::ActivityHistory::open_existing(&path).unwrap();
        let original = history.page(0, 100).unwrap().0;
        assert_eq!(original.len(), 1);

        let late_intent = guard.mark_send_possible();

        assert!(
            late_intent.is_err(),
            "late send-possible intent reopened terminal download"
        );
        let retained = history.page(0, 100).unwrap().0;
        assert_eq!(retained.len(), 1);
        assert_eq!(retained[0].document, original[0].document);
        assert_eq!(retained[0].sequence, original[0].sequence);
        assert_eq!(log.history_state(), "configured");
        assert!(log.ensure_write_available().is_ok());
        guard.record_cleanup(false).unwrap();
        let (entries, more) = log.history_page(0, 100).unwrap();
        assert!(!more);
        assert_eq!(entries.len(), 1);
        let value = serde_json::to_value(&entries[0]).unwrap();
        assert_eq!(value["state"], "failed");
        assert_eq!(value["writeEvidence"]["backupRecorded"], false);
        assert_eq!(value["writeEvidence"]["sendPossible"], false);
        assert_eq!(value["downloadEvidence"]["written"], "no");
        assert_eq!(
            value["downloadEvidence"]["restart"],
            serde_json::Value::Null
        );
        assert_eq!(value["downloadEvidence"]["cleanup"], "returnedError");
    }

    #[test]
    fn download_without_evidence_is_refused_before_history_creation() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("activity.sqlite");
        let log = Arc::new(OneShotLog::persistent(path.clone(), "synthetic".into()));

        let result = log.start_write("deviceDownload", Some("1.1.1".into()));

        assert!(
            result.is_err(),
            "download without lifecycle evidence accepted"
        );
        assert_eq!(log.history_state(), "unavailable");
        assert!(log.ensure_write_available().is_err());
        assert!(!path.exists());
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 0);
    }

    #[test]
    fn invalid_download_receipts_are_refused_without_overwriting_or_partial_history() {
        use serde_json::{json, Value};

        let valid = json!({
            "id": 1, "kind": "deviceDownload", "address": "1.1.1",
            "state": "finished", "startedAt": "2026-10-01T00:00:00Z",
            "finishedAt": "2026-10-01T00:00:01Z",
            "writeEvidence": {"backupRecorded": true, "sendPossible": true},
            "downloadEvidence": {"sessionId": 9, "written": "yes",
                "restart": "acknowledged", "cleanup": "returnedOk"}
        });
        let mut cases: Vec<(&str, Value)> = Vec::new();
        for (name, pointer, replacement) in [
            ("missing-download", "/downloadEvidence", Value::Null),
            ("missing-write", "/writeEvidence", Value::Null),
            ("zero-session", "/downloadEvidence/sessionId", json!(0)),
            ("negative-session", "/downloadEvidence/sessionId", json!(-1)),
            ("string-session", "/downloadEvidence/sessionId", json!("9")),
            (
                "unknown-written",
                "/downloadEvidence/written",
                json!("unsupported"),
            ),
            (
                "unknown-restart",
                "/downloadEvidence/restart",
                json!("unsupported"),
            ),
            (
                "unknown-cleanup",
                "/downloadEvidence/cleanup",
                json!("unsupported"),
            ),
            ("null-cleanup", "/downloadEvidence/cleanup", Value::Null),
            (
                "missing-written-result",
                "/downloadEvidence/written",
                Value::Null,
            ),
            (
                "missing-restart-result",
                "/downloadEvidence/restart",
                Value::Null,
            ),
            (
                "send-without-backup",
                "/writeEvidence/backupRecorded",
                json!(false),
            ),
            (
                "write-without-intent",
                "/writeEvidence/sendPossible",
                json!(false),
            ),
        ] {
            let mut document = valid.clone();
            *document.pointer_mut(pointer).unwrap() = replacement;
            cases.push((name, document));
        }
        for (name, field) in [
            ("unknown-download-field", "downloadEvidence"),
            ("unknown-write-field", "writeEvidence"),
        ] {
            let mut document = valid.clone();
            document[field]["futureField"] = json!(true);
            cases.push((name, document));
        }
        for (name, field) in [
            ("absent-session", "sessionId"),
            ("absent-cleanup", "cleanup"),
        ] {
            let mut document = valid.clone();
            document["downloadEvidence"]
                .as_object_mut()
                .unwrap()
                .remove(field);
            cases.push((name, document));
        }
        let mut running = valid.clone();
        running["state"] = json!("running");
        running["finishedAt"] = Value::Null;
        running["downloadEvidence"]["written"] = Value::Null;
        running["downloadEvidence"]["restart"] = Value::Null;
        running["downloadEvidence"]["cleanup"] = json!("pending");
        for (name, field, replacement) in [
            ("running-written", "written", json!("no")),
            ("running-restart", "restart", json!("notInPlan")),
            ("running-cleanup", "cleanup", json!("unknown")),
        ] {
            let mut document = running.clone();
            document["downloadEvidence"][field] = replacement;
            cases.push((name, document));
        }
        let mut failed = valid.clone();
        failed["state"] = json!("failed");
        cases.push(("failed-with-restart", failed));
        let mut unknown = valid.clone();
        unknown["state"] = json!("unknown");
        unknown["downloadEvidence"]["written"] = Value::Null;
        unknown["downloadEvidence"]["restart"] = Value::Null;
        unknown["downloadEvidence"]["cleanup"] = json!("unknown");
        for (name, field, replacement) in [
            ("unknown-written-result", "written", json!("no")),
            ("unknown-restart-result", "restart", json!("notInPlan")),
            ("unknown-cleanup-success", "cleanup", json!("returnedOk")),
        ] {
            let mut document = unknown.clone();
            document["downloadEvidence"][field] = replacement;
            cases.push((name, document));
        }
        let mut non_download = valid.clone();
        non_download["kind"] = json!("deviceCompare");
        non_download
            .as_object_mut()
            .unwrap()
            .remove("writeEvidence");
        cases.push(("download-evidence-on-read", non_download));

        for (name, mut malformed) in cases {
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("activity.sqlite");
            let log = Arc::new(OneShotLog::persistent(path.clone(), "synthetic".into()));
            let history = knx_store::activity_history::ActivityHistory::open(&path).unwrap();
            history.record("synthetic", 1, &valid.to_string()).unwrap();
            assert_eq!(log.history_page(0, 100).unwrap().0.len(), 1);
            malformed["id"] = json!(2);
            history
                .record("synthetic", 2, &malformed.to_string())
                .unwrap();
            let before = std::fs::read(&path).unwrap();
            let original = history.page(0, 100).unwrap().0;

            assert!(
                log.history_page(0, 100).is_err(),
                "invalid download receipt was rendered: {name}"
            );
            assert_eq!(log.history_state(), "unavailable");
            assert!(log.start_download(10, "1.1.1".parse().unwrap()).is_err());
            assert!(std::fs::read(&path).unwrap() == before);
            let retained = history.page(0, 100).unwrap().0;
            assert_eq!(retained.len(), 2);
            for (before, after) in original.iter().zip(&retained) {
                assert_eq!(before.sequence, after.sequence);
                assert_eq!(before.incarnation, after.incarnation);
                assert_eq!(before.operation_id, after.operation_id);
                assert_eq!(before.document, after.document);
            }
            assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 1);
        }
    }

    #[test]
    fn download_metadata_keeps_written_restart_and_cleanup_distinct() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("activity.sqlite");
        let log = OneShotLog::persistent(path.clone(), "synthetic".into());
        let document = serde_json::json!({
            "id": 1, "kind": "deviceDownload", "address": "1.1.1",
            "state": "finished", "startedAt": "2026-10-01T00:00:00Z",
            "finishedAt": "2026-10-01T00:00:01Z",
            "writeEvidence": {"backupRecorded": true, "sendPossible": true},
            "downloadEvidence": {"sessionId": 9, "written": "yes",
                "restart": "unconfirmed", "cleanup": "returnedError"}
        });
        knx_store::activity_history::ActivityHistory::open(&path)
            .unwrap()
            .record("synthetic", 1, &document.to_string())
            .unwrap();
        let (entries, more) = log.history_page(0, 100).unwrap();
        assert!(!more);
        assert_eq!(entries.len(), 1);
        let value = serde_json::to_value(&entries[0]).unwrap();
        assert_eq!(value["kind"], "deviceDownload");
        assert_eq!(value["state"], "finished");
        assert_eq!(value["interrupted"], false);
        assert_eq!(value["writeEvidence"], document["writeEvidence"]);
        assert_eq!(value["downloadEvidence"], document["downloadEvidence"]);
        assert!(log.snapshot_with_dropped().0.is_empty());
    }

    #[test]
    fn previous_incarnation_pending_cleanup_is_unknown_without_erasing_device_result() {
        for finished in [false, true] {
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("activity.sqlite");
            let log = OneShotLog::persistent(path.clone(), "current".into());
            let document = serde_json::json!({
                "id": 1, "kind": "deviceDownload", "address": "1.1.1",
                "state": if finished { "finished" } else { "running" },
                "startedAt": "2026-10-01T00:00:00Z",
                "finishedAt": if finished { Some("2026-10-01T00:00:01Z") } else { None },
                "writeEvidence": {"backupRecorded": true, "sendPossible": true},
                "downloadEvidence": {"sessionId": 9,
                    "written": if finished { Some("yes") } else { None },
                    "restart": if finished { Some("unconfirmed") } else { None },
                    "cleanup": "pending"}
            });
            let original = document.to_string();
            knx_store::activity_history::ActivityHistory::open(&path)
                .unwrap()
                .record("previous", 1, &original)
                .unwrap();
            let (entries, more) = log.history_page(0, 100).unwrap();
            assert!(!more);
            assert_eq!(entries.len(), 1);
            let value = serde_json::to_value(&entries[0]).unwrap();
            assert_eq!(value["interrupted"], true);
            assert_eq!(
                value["state"],
                if finished { "finished" } else { "unknown" }
            );
            assert_eq!(value["finishedAt"], document["finishedAt"]);
            assert_eq!(value["writeEvidence"], document["writeEvidence"]);
            assert_eq!(value["downloadEvidence"]["cleanup"], "unknown");
            assert_eq!(
                value["downloadEvidence"]["written"],
                document["downloadEvidence"]["written"]
            );
            assert_eq!(
                value["downloadEvidence"]["restart"],
                document["downloadEvidence"]["restart"]
            );
            let history =
                knx_store::activity_history::ActivityHistory::open_existing(&path).unwrap();
            let (retained, _) = history.page(0, 100).unwrap();
            assert_eq!(retained.len(), 1);
            assert_eq!(retained[0].document, original);
        }
    }

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
        let dir = tempfile::tempdir().unwrap();
        let log = Arc::new(OneShotLog::persistent(
            dir.path().join("activity.sqlite"),
            "synthetic".into(),
        ));
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
        let dir = tempfile::tempdir().unwrap();
        let log = Arc::new(OneShotLog::persistent(
            dir.path().join("activity.sqlite"),
            "synthetic".into(),
        ));
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
