//! Writer thread, listing, retention and deletion of operation recordings on disk.

use std::collections::HashSet;
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::Receiver;
use std::sync::{Arc, Mutex};
use std::time::{Instant, SystemTime};

use knx_net::wire_tap::WireRecord;
use serde::Serialize;
use serde_json::Value;

use super::encode::{self, EndCounts};
use super::{is_operation_id, OutcomeStatus, RecordedOutcome, RecordingOpener, WriterIdentity};

/// Keep at most this many recordings and bytes; failed ones are evicted last.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RetentionLimits {
    pub max_recordings: usize,
    pub max_bytes: u64,
}

impl Default for RetentionLimits {
    /// Owner decision 5: the last 100 operations or 512 MB.
    fn default() -> Self {
        Self {
            max_recordings: 100,
            max_bytes: 512 * 1024 * 1024,
        }
    }
}

pub(super) struct Shared {
    pub(super) root: PathBuf,
    pub(super) identity: WriterIdentity,
    pub(super) disabled: bool,
    pub(super) limits: RetentionLimits,
    pub(super) opener: RecordingOpener,
    pub(super) active: Mutex<HashSet<String>>,
}

impl Shared {
    pub(super) fn mark_active(&self, id: &str) {
        self.active_ids().insert(id.to_string());
    }

    pub(super) fn mark_inactive(&self, id: &str) {
        self.active_ids().remove(id);
    }

    fn active_ids(&self) -> std::sync::MutexGuard<'_, HashSet<String>> {
        self.active
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    fn is_active(&self, id: &str) -> bool {
        self.active_ids().contains(id)
    }

    pub(super) fn active_count(&self) -> usize {
        self.active_ids().len()
    }
}

/// What the writer thread needs from a recorder, without its sender: the
/// channel closes when the last recorder handle and tap are gone.
pub(crate) struct WriterSide {
    pub(crate) id: String,
    pub(crate) started: Instant,
    pub(crate) dropped: Arc<AtomicU64>,
    pub(crate) writer_failed: Arc<AtomicBool>,
}

pub(crate) enum Message {
    Wire {
        seq: u64,
        record: WireRecord,
    },
    Note {
        seq: u64,
        at: Instant,
        wall: SystemTime,
        name: String,
        data: Value,
    },
    End {
        seq: u64,
        at: Instant,
        wall: SystemTime,
        outcome: RecordedOutcome,
    },
}

struct LineWriter {
    out: Box<dyn Write + Send>,
    records: u64,
    failed_writes: u64,
    /// Records lost to failed writes since the last successful one.
    pending_lost: u64,
    last_error: Option<String>,
    /// A failed write may have left half a line; end it before the next.
    needs_newline: bool,
    reported_dropped: u64,
}

impl LineWriter {
    fn write_raw(&mut self, value: &Value) -> io::Result<()> {
        let mut bytes = Vec::with_capacity(256);
        if self.needs_newline {
            bytes.push(b'\n');
        }
        serde_json::to_writer(&mut bytes, value).map_err(io::Error::other)?;
        bytes.push(b'\n');
        self.out.write_all(&bytes)?;
        self.out.flush()
    }

    /// Writes one record line; reports earlier write losses first.
    fn write(&mut self, value: &Value) {
        if self.pending_lost > 0 {
            let reason = format!(
                "write failed: {}",
                self.last_error.as_deref().unwrap_or("unknown")
            );
            let gap = encode::gap(self.pending_lost, &reason, SystemTime::now());
            if self.write_raw(&gap).is_ok() {
                self.pending_lost = 0;
                self.needs_newline = false;
            }
        }
        match self.write_raw(value) {
            Ok(()) => {
                self.records += 1;
                self.needs_newline = false;
            }
            Err(error) => {
                self.failed_writes += 1;
                self.pending_lost += 1;
                self.last_error = Some(error.to_string());
                self.needs_newline = true;
            }
        }
    }

    /// Writes a gap line for records the queue dropped since the last one.
    fn report_drops(&mut self, dropped: &AtomicU64) {
        let now = dropped.load(Ordering::Relaxed);
        if now > self.reported_dropped {
            let lost = now - self.reported_dropped;
            self.reported_dropped = now;
            let gap = encode::gap(lost, "recorder queue full", SystemTime::now());
            if self.write_raw(&gap).is_ok() {
                self.needs_newline = false;
            }
        }
    }
}

pub(super) fn run_writer(
    shared: Arc<Shared>,
    side: WriterSide,
    header: Value,
    receiver: Receiver<Message>,
) {
    // Retention first, so the new recording fits within the limits.
    let _ = enforce_retention(&shared);
    let path = shared.root.join(format!("{}.jsonl", side.id));
    let opened = std::fs::create_dir_all(&shared.root).and_then(|()| (shared.opener)(&path));
    let out = match opened {
        Ok(out) => out,
        Err(_) => {
            side.writer_failed.store(true, Ordering::Relaxed);
            // Keep consuming so producers never see a full queue for long.
            while receiver.recv().is_ok() {}
            shared.mark_inactive(&side.id);
            return;
        }
    };
    let mut writer = LineWriter {
        out,
        records: 0,
        failed_writes: 0,
        pending_lost: 0,
        last_error: None,
        needs_newline: false,
        reported_dropped: 0,
    };
    writer.write(&header);
    let mut ended = false;
    for message in receiver.iter() {
        writer.report_drops(&side.dropped);
        match message {
            Message::Wire { seq, record } => {
                writer.write(&encode::wire(seq, side.started, &record));
            }
            Message::Note {
                seq,
                at,
                wall,
                name,
                data,
            } => writer.write(&encode::note(seq, side.started, at, wall, &name, &data)),
            Message::End {
                seq,
                at,
                wall,
                outcome,
            } => {
                write_end(&mut writer, &side, seq, at, wall, &outcome);
                ended = true;
                break;
            }
        }
    }
    if !ended {
        let outcome = RecordedOutcome {
            status: OutcomeStatus::Abandoned,
            error_chain: vec!["the operation ended without recording an outcome".into()],
            summary: Value::Null,
        };
        write_end(
            &mut writer,
            &side,
            0,
            Instant::now(),
            SystemTime::now(),
            &outcome,
        );
    }
    drop(writer);
    shared.mark_inactive(&side.id);
    let _ = enforce_retention(&shared);
}

fn write_end(
    writer: &mut LineWriter,
    side: &WriterSide,
    seq: u64,
    at: Instant,
    wall: SystemTime,
    outcome: &RecordedOutcome,
) {
    writer.report_drops(&side.dropped);
    let counts = EndCounts {
        // The end line itself is counted once written.
        records: writer.records + 1,
        dropped: side.dropped.load(Ordering::Relaxed),
        failed_writes: writer.failed_writes,
    };
    writer.write(&encode::end(seq, side.started, at, wall, outcome, &counts));
}

/// One recording as listed; read from its header and last line only.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct RecordingSummary {
    pub id: String,
    pub size_bytes: u64,
    pub format_version: Option<u64>,
    pub writer_version: Option<String>,
    pub operation_kind: Option<String>,
    pub started_at: Option<String>,
    pub target: Value,
    pub gateway: Option<String>,
    pub related: Value,
    /// The `end` line's outcome; `None` while the recording is incomplete.
    pub outcome: Option<String>,
    /// Still being written by this process.
    pub active: bool,
    /// The last line is cut off (a crash or a full disk); reported, not fatal.
    pub torn_tail: bool,
}

impl RecordingSummary {
    /// Anything but a recorded success is kept longest.
    fn is_failed(&self) -> bool {
        self.outcome.as_deref() != Some("succeeded")
    }
}

/// The path of recording `id`, refusing anything that is not an operation id.
pub(super) fn path_for(root: &Path, id: &str) -> io::Result<PathBuf> {
    if !is_operation_id(id) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "not an operation id",
        ));
    }
    Ok(root.join(format!("{id}.jsonl")))
}

pub(super) fn list(shared: &Shared) -> io::Result<Vec<RecordingSummary>> {
    let entries = match std::fs::read_dir(&shared.root) {
        Ok(entries) => entries,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(error),
    };
    let mut out = Vec::new();
    for entry in entries {
        let entry = entry?;
        let name = entry.file_name();
        let Some(id) = name.to_str().and_then(|n| n.strip_suffix(".jsonl")) else {
            continue;
        };
        if !is_operation_id(id) || !entry.file_type()?.is_file() {
            continue;
        }
        out.push(summarize(&entry.path(), id, shared.is_active(id))?);
    }
    out.sort_by(|a, b| b.id.cmp(&a.id));
    Ok(out)
}

fn summarize(path: &Path, id: &str, active: bool) -> io::Result<RecordingSummary> {
    let mut file = std::fs::File::open(path)?;
    let size_bytes = file.metadata()?.len();
    let header = first_line(&mut file)?
        .and_then(|line| serde_json::from_str::<Value>(&line).ok())
        .unwrap_or(Value::Null);
    let (last, torn_tail) = last_line(&mut file, size_bytes)?;
    let end = last
        .and_then(|line| serde_json::from_str::<Value>(&line).ok())
        .filter(|value| value["type"] == "end");
    let text = |value: &Value| value.as_str().map(str::to_string);
    Ok(RecordingSummary {
        id: id.to_string(),
        size_bytes,
        format_version: header["format_version"].as_u64(),
        writer_version: text(&header["writer"]["version"]),
        operation_kind: text(&header["operation_kind"]),
        started_at: text(&header["started_at"]),
        target: header["target"].clone(),
        gateway: text(&header["gateway"]),
        related: header["related"].clone(),
        outcome: end.and_then(|end| text(&end["outcome"])),
        active,
        torn_tail,
    })
}

/// The first line, read at most 1 MiB deep.
fn first_line(file: &mut std::fs::File) -> io::Result<Option<String>> {
    file.seek(SeekFrom::Start(0))?;
    let mut buffer = Vec::new();
    file.take(1024 * 1024).read_to_end(&mut buffer)?;
    let line = match buffer.iter().position(|b| *b == b'\n') {
        Some(end) => &buffer[..end],
        None => return Ok(None),
    };
    Ok(Some(String::from_utf8_lossy(line).into_owned()))
}

/// The last complete line, and whether bytes after it lack a newline.
fn last_line(file: &mut std::fs::File, size: u64) -> io::Result<(Option<String>, bool)> {
    const TAIL: u64 = 64 * 1024;
    let start = size.saturating_sub(TAIL);
    file.seek(SeekFrom::Start(start))?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;
    let torn = !buffer.is_empty() && buffer.last() != Some(&b'\n');
    let complete = match buffer.iter().rposition(|b| *b == b'\n') {
        Some(end) => &buffer[..end],
        None => return Ok((None, torn)),
    };
    let begin = complete
        .iter()
        .rposition(|b| *b == b'\n')
        .map_or(0, |p| p + 1);
    if begin == 0 && start > 0 {
        // The line is longer than the tail window; not an end line.
        return Ok((None, torn));
    }
    Ok((
        Some(String::from_utf8_lossy(&complete[begin..]).into_owned()),
        torn,
    ))
}

/// What a delete-all removed and what it had to keep.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct DeleteOutcome {
    pub removed: usize,
    /// Recordings being written right now; deleted with a later request.
    pub kept_active: Vec<String>,
}

pub(super) fn delete_all(shared: &Shared) -> io::Result<DeleteOutcome> {
    let mut removed = 0;
    let mut kept_active = Vec::new();
    for summary in list(shared)? {
        if summary.active {
            kept_active.push(summary.id);
            continue;
        }
        match std::fs::remove_file(path_for(&shared.root, &summary.id)?) {
            Ok(()) => removed += 1,
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(error),
        }
    }
    Ok(DeleteOutcome {
        removed,
        kept_active,
    })
}

/// Evicts the oldest succeeded recordings first, then the oldest failed
/// ones, until both limits hold. Active recordings count but are never
/// evicted.
pub(super) fn enforce_retention(shared: &Shared) -> io::Result<Vec<String>> {
    let mut all = list(shared)?;
    // `list` is newest first; walk candidates oldest first.
    all.reverse();
    let mut count = all.len();
    let mut bytes: u64 = all.iter().map(|s| s.size_bytes).sum();
    let limits = shared.limits;
    let over = |count: usize, bytes: u64| count > limits.max_recordings || bytes > limits.max_bytes;
    let mut evicted = Vec::new();
    let candidates = all
        .iter()
        .filter(|s| !s.active && !s.is_failed())
        .chain(all.iter().filter(|s| !s.active && s.is_failed()));
    for summary in candidates {
        if !over(count, bytes) {
            break;
        }
        // A file that vanished meanwhile is gone either way.
        let _ = std::fs::remove_file(path_for(&shared.root, &summary.id)?);
        count -= 1;
        bytes = bytes.saturating_sub(summary.size_bytes);
        evicted.push(summary.id.clone());
    }
    Ok(evicted)
}
