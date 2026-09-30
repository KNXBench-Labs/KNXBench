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
        });
        prune(&mut inner);
        ActionGuard {
            log: Arc::clone(self),
            id,
            done: false,
        }
    }

    pub(crate) fn snapshot_with_dropped(&self) -> (Vec<OneShotActivity>, u64) {
        let inner = self.inner.lock().expect("activity log poisoned");
        (inner.entries.iter().cloned().collect(), inner.dropped_count)
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
