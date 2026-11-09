//! The background journal writer: project commands are answered first, journaled after.
//!
//! ADR-0166. A project command hands the writer the newest project and stack;
//! a newer hand-over replaces one still pending, so rapid edits become one
//! write of the newest state. Everything that reads the history generation
//! waits for the writer first ([`HistoryGeneration::lock`]), so Save, Save As,
//! version actions and the other journal writers never interleave with it.
//! The writer never takes the project lock: a caller holding it may wait.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, LockResult, Mutex, MutexGuard};
use std::time::Instant;

use knx_store::project_history::{JournalWriteReport, NativeSnapshot};

use crate::domain::PersistedHistory;
use crate::journal_write_log::{JournalWriteRecord, RecentJournalWrites};

/// The journal state the project tree and the status bar show.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase", tag = "state")]
pub enum JournalStatus {
    /// Every acknowledged command is in the file.
    Saved,
    /// A write is pending or running.
    Saving,
    /// The last write failed; project commands are refused until a write succeeds.
    Failed { message: String },
}

/// What one journal write needs; the shared cells are the server's own.
pub(crate) struct JournalJob {
    /// Assigned by [`JournalWriter::enqueue`]; later jobs hold newer states.
    pub sequence: u64,
    pub path: PathBuf,
    pub snapshot: NativeSnapshot,
    pub stack: knx_core::CommandStack,
    pub cells: JournalCells,
}

/// The server state a journal write reads and updates.
#[derive(Clone, Default)]
pub(crate) struct JournalCells {
    pub generation: Arc<Mutex<i64>>,
    pub persisted: Arc<Mutex<Option<PersistedHistory>>>,
    pub writes: Arc<Mutex<RecentJournalWrites>>,
}

#[derive(Default)]
struct Queue {
    pending: Option<JournalJob>,
    busy: bool,
    /// The failure and the sequence of the job that failed.
    failure: Option<(String, u64)>,
    thread: bool,
    /// The sequence of the newest job handed over.
    sequence: u64,
}

#[derive(Default)]
struct Inner {
    background: AtomicBool,
    queue: Mutex<Queue>,
    changed: Condvar,
}

/// The server's journal writer; synchronous until [`Self::run_in_background`].
#[derive(Clone, Default)]
pub struct JournalWriter {
    inner: Arc<Inner>,
}

impl JournalWriter {
    /// Journals project commands on a background thread from now on (ADR-0166).
    pub fn run_in_background(&self) {
        self.inner.background.store(true, Ordering::SeqCst);
    }

    pub fn is_background(&self) -> bool {
        self.inner.background.load(Ordering::SeqCst)
    }

    fn queue(&self) -> MutexGuard<'_, Queue> {
        self.inner
            .queue
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    pub fn status(&self) -> JournalStatus {
        let queue = self.queue();
        match &queue.failure {
            Some((message, _)) => JournalStatus::Failed {
                message: message.clone(),
            },
            None if queue.busy || queue.pending.is_some() => JournalStatus::Saving,
            None => JournalStatus::Saved,
        }
    }

    /// Refuses a project command while the last journal write failed.
    pub fn refuse_if_failed(&self) -> Result<(), String> {
        match &self.queue().failure {
            Some((message, _)) => Err(format!(
                "the project file could not be written ({message}); nothing shown was \
                 discarded. Retry the write or save the project under another name"
            )),
            None => Ok(()),
        }
    }

    /// Hands `job` to the writer; a job still pending is replaced by it.
    pub(crate) fn enqueue(&self, mut job: JournalJob) {
        let mut queue = self.queue();
        queue.sequence += 1;
        job.sequence = queue.sequence;
        queue.pending = Some(job);
        if !queue.thread {
            queue.thread = true;
            let writer = self.clone();
            std::thread::Builder::new()
                .name("journal-writer".into())
                .spawn(move || writer.run())
                .expect("journal writer thread starts");
        }
        self.inner.changed.notify_all();
    }

    /// Waits until no write is pending or running, or the writer failed.
    pub fn flush(&self) {
        let mut queue = self.queue();
        while queue.busy || (queue.pending.is_some() && queue.failure.is_none()) {
            queue = self
                .inner
                .changed
                .wait(queue)
                .unwrap_or_else(std::sync::PoisonError::into_inner);
        }
    }

    /// Clears a failure and drops the job that failed: the caller hands over
    /// the current state next ([`crate::domain::retry_journal`]), because
    /// another writer may have stored a newer one since.
    pub(crate) fn clear_failure(&self) {
        let mut queue = self.queue();
        queue.failure = None;
        queue.pending = None;
        self.inner.changed.notify_all();
    }

    /// The sequence of the newest job handed over so far.
    pub fn sequence(&self) -> u64 {
        self.queue().sequence
    }

    /// A synchronous writer stored a state at least as new as every job up to
    /// `through` ([`Self::sequence`] read while it held the generation): those
    /// jobs, and their failure, are obsolete. A newer job stays.
    pub fn superseded(&self, through: u64) {
        let mut queue = self.queue();
        if queue
            .pending
            .as_ref()
            .is_some_and(|job| job.sequence <= through)
        {
            queue.pending = None;
        }
        if queue
            .failure
            .as_ref()
            .is_some_and(|(_, failed)| *failed <= through)
        {
            queue.failure = None;
        }
        self.inner.changed.notify_all();
    }

    fn run(&self) {
        let mut kept: Option<(PathBuf, i64, knx_app::project_history::KeptJournal)> = None;
        loop {
            let job = {
                let mut queue = self.queue();
                while queue.pending.is_none() || queue.failure.is_some() {
                    queue = self
                        .inner
                        .changed
                        .wait(queue)
                        .unwrap_or_else(std::sync::PoisonError::into_inner);
                }
                queue.busy = true;
                queue.pending.take().expect("a pending job")
            };
            // A panic must not leave the writer busy forever: every reader of
            // the generation and every stop would wait for it.
            let written = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                write_job(&job, &mut kept)
            }));
            let failure = match written {
                Ok(result) => result.err(),
                Err(_) => {
                    kept = None;
                    Some("the journal writer failed unexpectedly".to_string())
                }
            };
            let mut queue = self.queue();
            queue.busy = false;
            if let Some(message) = failure {
                let failed = job.sequence;
                // Keep the newest state for the retry; a newer one replaced it already.
                if queue.pending.is_none() {
                    queue.pending = Some(job);
                }
                queue.failure = Some((message, failed));
            }
            self.inner.changed.notify_all();
        }
    }
}

/// One background write: the own-commit path while the kept connection proves
/// that only this writer committed since, the full ADR-0165 path otherwise.
fn write_job(
    job: &JournalJob,
    kept: &mut Option<(PathBuf, i64, knx_app::project_history::KeptJournal)>,
) -> Result<(), String> {
    #[cfg(test)]
    if INJECT_PANIC.swap(false, Ordering::SeqCst) {
        panic!("injected journal writer fault");
    }
    let started = Instant::now();
    let mut report = JournalWriteReport::default();
    let result = (|| {
        let mut generation = job
            .cells
            .generation
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let mut persisted = job
            .cells
            .persisted
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let own = match kept.take() {
            Some((path, at, connection))
                if path == job.path && at == *generation && connection.still_own(&job.path) =>
            {
                Some(connection)
            }
            _ => None,
        };
        let trust = persisted.take().map_or_else(Default::default, |earlier| {
            earlier.into_trust(&job.snapshot.project, &job.stack)
        });
        report.phase("prepare", started);
        let (next, trust, connection) = knx_app::project_history::persist_working_kept(
            &job.path,
            own,
            &job.snapshot,
            &job.stack,
            *generation,
            &trust,
            &mut report,
        )?;
        *generation = next;
        let retained = Instant::now();
        *persisted = Some(PersistedHistory::new(
            job.snapshot.project.clone(),
            job.stack.clone(),
            trust,
        ));
        report.phase("retain", retained);
        *kept = Some((job.path.clone(), next, connection));
        Ok(())
    })();
    job.cells
        .writes
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .record(JournalWriteRecord {
            at: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
            total: started.elapsed(),
            ok: result.is_ok(),
            report,
        });
    result
}

/// The journal generation of the open native project. Locking it waits for
/// the background writer first, so whoever reads or writes the generation
/// sees every acknowledged command in the file.
#[derive(Default)]
pub struct HistoryGeneration {
    pub(crate) value: Arc<Mutex<i64>>,
    pub(crate) writer: JournalWriter,
}

impl HistoryGeneration {
    pub fn lock(&self) -> LockResult<MutexGuard<'_, i64>> {
        self.writer.flush();
        self.value.lock()
    }
}

#[cfg(test)]
impl JournalWriter {
    /// Whether a write is running right now.
    pub(crate) fn is_busy(&self) -> bool {
        self.queue().busy
    }

    /// Makes the next background write panic, as an unexpected fault would.
    pub(crate) fn inject_panic_for_tests(&self) {
        INJECT_PANIC.store(true, Ordering::SeqCst);
    }
}

#[cfg(test)]
static INJECT_PANIC: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
