//! Always-on per-operation recorder writing JSONL off the async runtime, with retention (DIAG-03).
//!
//! Every operation that opens a device connection gets one recording:
//! `<data dir>/diagnostics/<operation id>.jsonl`, one JSON object per line.
//! The first line is a header (format version, writing KNXBench version,
//! operation kind, target, gateway, settings); then datagrams and tunnel
//! events from the [`knx_net::wire_tap::WireTap`], notes the operation adds
//! (plan, comparisons, results), and finally an `end` line with the outcome.
//!
//! Recording must never change the device operation:
//!
//! - Callers only enqueue into a bounded queue (`try_send`); a full queue
//!   drops the record and counts it. The writer thread later writes a `gap`
//!   line naming how many records were lost.
//! - All file I/O happens on one dedicated thread per recording, never on
//!   the async runtime. A failing write (disk full) is counted and reported
//!   as a gap on the next successful write; it never reaches the caller.
//! - Starting a recording never fails into the caller. A recorder that could
//!   not open its file stays inert and says so in [`OperationRecorder::status`].
//!
//! Retention keeps the newest recordings within [`RetentionLimits`]; failed
//! or incomplete recordings are evicted only after every succeeded one.

mod encode;
mod store;
#[cfg(test)]
mod tests;

use std::collections::BTreeMap;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{self, SyncSender, TrySendError};
use std::sync::{Arc, Mutex};
use std::time::{Instant, SystemTime};

use knx_net::wire_tap::{WireRecord, WireSink, WireTap};
use serde::Serialize;

pub use store::{DeleteOutcome, RecordingSummary, RetentionLimits};

/// Value of the header's `format` field.
pub const RECORDING_FORMAT: &str = "knxbench-operation-recording";
/// Version of the line format this build writes. Readers export any version
/// unchanged; they interpret only versions they know.
pub const RECORDING_FORMAT_VERSION: u32 = 1;
/// Directory under the data directory that holds the recordings.
pub const DIAGNOSTICS_DIR_NAME: &str = "diagnostics";
/// Environment variable with which an operator disables recording.
pub const DISABLE_ENV: &str = "KNX_DIAGNOSTIC_RECORDING";
/// How many records may wait for the writer before new ones are dropped.
const QUEUE_CAPACITY: usize = 4096;

/// Whether the operator switched recording off through [`DISABLE_ENV`].
/// Only an explicit `off`, `0`, `false` or `disabled` disables it.
pub fn disabled_by_env_value(value: Option<&str>) -> bool {
    matches!(
        value.map(|v| v.trim().to_ascii_lowercase()).as_deref(),
        Some("off" | "0" | "false" | "disabled")
    )
}

/// The program writing recordings; becomes the header's `writer`.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct WriterIdentity {
    /// `knx-server` or `knx-cli`.
    pub program: String,
    /// Exactly the version string `GET /api/version` / `--version` report.
    pub version: String,
    pub commit: Option<String>,
    /// `debug` or `release`.
    pub build_profile: String,
}

/// What kind of device operation a recording belongs to.
#[derive(Debug, Clone, Copy, Serialize, serde::Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "kebab-case")]
pub enum OperationKind {
    DeviceDiagnostics,
    DeviceRead,
    Download,
    System2Download,
    System2Recovery,
    Compare,
    IndividualAddressProgramming,
    SerialNumberAddress,
    IndividualAddressReset,
    ServiceControl,
    LineScan,
    SupportRead,
    ByteOrderProbe,
}

impl OperationKind {
    pub fn as_str(self) -> &'static str {
        match self {
            OperationKind::DeviceDiagnostics => "device-diagnostics",
            OperationKind::DeviceRead => "device-read",
            OperationKind::Download => "download",
            OperationKind::System2Download => "system2-download",
            OperationKind::System2Recovery => "system2-recovery",
            OperationKind::Compare => "compare",
            OperationKind::IndividualAddressProgramming => "individual-address-programming",
            OperationKind::SerialNumberAddress => "serial-number-address",
            OperationKind::IndividualAddressReset => "individual-address-reset",
            OperationKind::ServiceControl => "service-control",
            OperationKind::LineScan => "line-scan",
            OperationKind::SupportRead => "support-read",
            OperationKind::ByteOrderProbe => "byte-order-probe",
        }
    }
}

/// Everything the header records about an operation at its start.
#[derive(Debug, Clone, Default, Serialize)]
pub struct OperationStart {
    pub kind: Option<OperationKind>,
    /// Device the operation addresses: project device id, individual
    /// address, name; whatever the caller knows.
    pub target: serde_json::Value,
    /// Gateway endpoint, e.g. `203.0.113.250:3671`.
    pub gateway: Option<String>,
    /// Identities of the same operation elsewhere (run id, activity id).
    pub related: BTreeMap<String, String>,
    /// Effective settings: timeouts, ACK wait, return path, verify mode.
    pub settings: serde_json::Value,
}

impl OperationStart {
    pub fn new(kind: OperationKind) -> Self {
        Self {
            kind: Some(kind),
            target: serde_json::Value::Null,
            ..Self::default()
        }
    }

    pub fn target(mut self, target: serde_json::Value) -> Self {
        self.target = target;
        self
    }

    pub fn gateway(mut self, gateway: impl ToString) -> Self {
        self.gateway = Some(gateway.to_string());
        self
    }

    pub fn related(mut self, key: &str, value: impl ToString) -> Self {
        self.related.insert(key.to_string(), value.to_string());
        self
    }

    pub fn settings(mut self, settings: serde_json::Value) -> Self {
        self.settings = settings;
        self
    }
}

/// How an operation ended, as the `end` line records it.
#[derive(Debug, Clone, Copy, Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum OutcomeStatus {
    Succeeded,
    Failed,
    /// Refused before any device contact, or cancelled by the user.
    Cancelled,
    /// The recorder was dropped without a recorded outcome.
    Abandoned,
}

/// The `end` line's content.
#[derive(Debug, Clone, Serialize)]
pub struct RecordedOutcome {
    pub status: OutcomeStatus,
    /// The complete error chain, outermost first.
    pub error_chain: Vec<String>,
    pub summary: serde_json::Value,
}

impl RecordedOutcome {
    pub fn succeeded(summary: serde_json::Value) -> Self {
        Self {
            status: OutcomeStatus::Succeeded,
            error_chain: Vec::new(),
            summary,
        }
    }

    pub fn failed(error_chain: Vec<String>, summary: serde_json::Value) -> Self {
        Self {
            status: OutcomeStatus::Failed,
            error_chain,
            summary,
        }
    }

    /// A failure from an error and its `source()` chain.
    pub fn from_error(error: &(dyn std::error::Error + 'static)) -> Self {
        Self::failed(error_chain(error), serde_json::Value::Null)
    }
}

/// `error` followed by each of its sources, outermost first.
pub fn error_chain(error: &(dyn std::error::Error + 'static)) -> Vec<String> {
    let mut chain = vec![error.to_string()];
    let mut source = error.source();
    while let Some(next) = source {
        chain.push(next.to_string());
        source = next.source();
    }
    chain
}

/// Opens the file a recording writes to. Replaceable for fault injection.
pub type RecordingOpener =
    Arc<dyn Fn(&Path) -> io::Result<Box<dyn Write + Send>> + Send + Sync + 'static>;

fn default_opener() -> RecordingOpener {
    Arc::new(|path: &Path| {
        let file = std::fs::OpenOptions::new()
            .create_new(true)
            .append(true)
            .open(path)?;
        Ok(Box::new(file) as Box<dyn Write + Send>)
    })
}

/// The recordings of one data directory.
pub struct RecordingStore {
    shared: Arc<store::Shared>,
}

impl RecordingStore {
    /// Recordings under `<data_dir>/diagnostics`. `disabled` comes from
    /// [`disabled_by_env_value`]; a disabled store records nothing but
    /// still lists, exports and deletes existing recordings.
    pub fn new(data_dir: &Path, identity: WriterIdentity, disabled: bool) -> Self {
        Self::with_options(
            data_dir.join(DIAGNOSTICS_DIR_NAME),
            identity,
            disabled,
            RetentionLimits::default(),
            default_opener(),
        )
    }

    /// Full control over location, limits and file opening (tests, CLI).
    #[doc(hidden)]
    pub fn with_options(
        root: PathBuf,
        identity: WriterIdentity,
        disabled: bool,
        limits: RetentionLimits,
        opener: RecordingOpener,
    ) -> Self {
        Self {
            shared: Arc::new(store::Shared {
                root,
                identity,
                disabled,
                limits,
                opener,
                active: Mutex::new(std::collections::HashSet::new()),
            }),
        }
    }

    pub fn root(&self) -> &Path {
        &self.shared.root
    }

    pub fn is_enabled(&self) -> bool {
        !self.shared.disabled
    }

    pub fn identity(&self) -> &WriterIdentity {
        &self.shared.identity
    }

    /// Starts recording one operation. Never fails: a disabled store, or a
    /// recording that cannot be created, yields an inert recorder whose
    /// [`OperationRecorder::status`] says why.
    pub fn start(&self, start: OperationStart) -> OperationRecorder {
        if self.shared.disabled {
            return OperationRecorder::inert(RecorderStatus::Disabled);
        }
        let id = new_operation_id();
        let (sender, receiver) = mpsc::sync_channel(QUEUE_CAPACITY);
        let started = Instant::now();
        let dropped = Arc::new(AtomicU64::new(0));
        let writer_failed = Arc::new(AtomicBool::new(false));
        let inner = Arc::new(RecorderInner {
            id: id.clone(),
            sender: Mutex::new(Some(sender)),
            next_seq: AtomicU64::new(1),
            dropped: dropped.clone(),
            finished: AtomicBool::new(false),
            writer_failed: writer_failed.clone(),
        });
        self.shared.mark_active(&id);
        let header = encode::header(&id, &start, &self.shared.identity, SystemTime::now());
        let shared = self.shared.clone();
        let side = store::WriterSide {
            id: id.clone(),
            started,
            dropped,
            writer_failed,
        };
        let spawned = std::thread::Builder::new()
            .name(format!(
                "knx-recorder-{}",
                &id[id.len().saturating_sub(8)..]
            ))
            .spawn(move || store::run_writer(shared, side, header, receiver));
        if spawned.is_err() {
            self.shared.mark_inactive(&id);
            return OperationRecorder::inert(RecorderStatus::Unavailable);
        }
        OperationRecorder {
            inner: Some(inner),
            inert: RecorderStatus::Recording,
        }
    }

    /// Every recording on disk, newest first.
    pub fn list(&self) -> io::Result<Vec<RecordingSummary>> {
        store::list(&self.shared)
    }

    /// The raw bytes of one recording, exactly as stored.
    pub fn read(&self, id: &str) -> io::Result<Vec<u8>> {
        let path = store::path_for(&self.shared.root, id)?;
        std::fs::read(path)
    }

    /// Reads at most `max_bytes`, plus one byte used only to detect overflow.
    ///
    /// The operation-id path rules are identical to [`Self::read`]. An
    /// oversized source is refused without materialising the rest of the file.
    pub fn read_bounded(&self, id: &str, max_bytes: u64) -> io::Result<Vec<u8>> {
        store::read_bounded(&self.shared.root, id, max_bytes)
    }

    /// Deletes every recording that is not being written right now.
    pub fn delete_all(&self) -> io::Result<DeleteOutcome> {
        store::delete_all(&self.shared)
    }

    /// Applies the retention limits now; returns the evicted ids.
    pub fn enforce_retention(&self) -> io::Result<Vec<String>> {
        store::enforce_retention(&self.shared)
    }

    /// Waits until no recording of this store is being written, or until
    /// `timeout`; returns whether it became idle. A CLI calls it before it
    /// exits or builds a bundle, so the last lines are on disk.
    pub fn wait_until_idle(&self, timeout: std::time::Duration) -> bool {
        let deadline = Instant::now() + timeout;
        loop {
            if self.shared.active_count() == 0 {
                return true;
            }
            if Instant::now() >= deadline {
                return false;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
    }
}

/// Why a recorder records nothing, or that it does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum RecorderStatus {
    Recording,
    /// The operator disabled recording ([`DISABLE_ENV`]).
    Disabled,
    /// The writer could not be started or its file not created.
    Unavailable,
}

pub(crate) struct RecorderInner {
    id: String,
    sender: Mutex<Option<SyncSender<store::Message>>>,
    next_seq: AtomicU64,
    dropped: Arc<AtomicU64>,
    finished: AtomicBool,
    writer_failed: Arc<AtomicBool>,
}

impl RecorderInner {
    fn enqueue(&self, build: impl FnOnce(u64) -> store::Message) {
        let guard = self
            .sender
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let Some(sender) = guard.as_ref() else {
            return;
        };
        let seq = self.next_seq.fetch_add(1, Ordering::Relaxed);
        match sender.try_send(build(seq)) {
            Ok(()) => {}
            Err(TrySendError::Full(_)) | Err(TrySendError::Disconnected(_)) => {
                self.dropped.fetch_add(1, Ordering::Relaxed);
            }
        }
    }
}

impl WireSink for RecorderInner {
    fn record(&self, record: WireRecord) {
        self.enqueue(|seq| store::Message::Wire { seq, record });
    }
}

/// Handle to one operation's recording. Cheap to clone; every method
/// returns at once and never fails into the caller.
#[derive(Clone)]
pub struct OperationRecorder {
    inner: Option<Arc<RecorderInner>>,
    /// Why an inert recorder records nothing.
    inert: RecorderStatus,
}

impl std::fmt::Debug for OperationRecorder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OperationRecorder")
            .field("id", &self.id())
            .finish()
    }
}

impl OperationRecorder {
    fn inert(status: RecorderStatus) -> Self {
        Self {
            inner: None,
            inert: status,
        }
    }

    /// A recorder that records nothing (tests, callers without a store).
    pub fn disabled() -> Self {
        Self::inert(RecorderStatus::Disabled)
    }

    /// The operation id, when this recorder records.
    pub fn id(&self) -> Option<&str> {
        self.inner.as_deref().map(|inner| inner.id.as_str())
    }

    pub fn status(&self) -> RecorderStatus {
        match &self.inner {
            None => self.inert,
            Some(inner) if inner.writer_failed.load(Ordering::Relaxed) => {
                RecorderStatus::Unavailable
            }
            Some(_) => RecorderStatus::Recording,
        }
    }

    /// The tap to hand to `KnxNetIpClient::with_wire_tap`.
    pub fn wire_tap(&self) -> Option<WireTap> {
        self.inner
            .as_ref()
            .map(|inner| WireTap::new(inner.clone() as Arc<dyn WireSink>))
    }

    /// Records a named note: a plan, a comparison, an intermediate result.
    pub fn note(&self, name: &str, data: serde_json::Value) {
        if let Some(inner) = &self.inner {
            let name = name.to_string();
            inner.enqueue(|seq| store::Message::Note {
                seq,
                at: Instant::now(),
                wall: SystemTime::now(),
                name,
                data,
            });
        }
    }

    /// Records the outcome and closes the recording. Later calls, and
    /// records arriving afterwards, are ignored.
    pub fn finish(&self, outcome: RecordedOutcome) {
        let Some(inner) = &self.inner else {
            return;
        };
        if inner.finished.swap(true, Ordering::SeqCst) {
            return;
        }
        let sender = inner
            .sender
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .take();
        if let Some(sender) = sender {
            let seq = inner.next_seq.fetch_add(1, Ordering::Relaxed);
            // The end line must not be lost to a full queue: wait for room.
            // This runs once per operation, after its device exchange.
            let _ = sender.send(store::Message::End {
                seq,
                at: Instant::now(),
                wall: SystemTime::now(),
                outcome,
            });
        }
    }
}

/// `op-<UTC timestamp with milliseconds>-<8 random hex digits>`; the
/// timestamp makes the lexical order chronological.
fn new_operation_id() -> String {
    let stamp = chrono::Utc::now().format("%Y%m%dT%H%M%S%3fZ");
    let mut random = [0u8; 4];
    if getrandom::fill(&mut random).is_err() {
        // A clock-derived suffix keeps ids distinct enough when the OS
        // randomness source is unavailable; recording must not fail.
        let nanos = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|d| d.subsec_nanos())
            .unwrap_or_default();
        random = nanos.to_be_bytes();
    }
    format!(
        "op-{stamp}-{:02x}{:02x}{:02x}{:02x}",
        random[0], random[1], random[2], random[3]
    )
}

/// Whether `id` has the shape [`new_operation_id`] produces. Guards every
/// path built from a client-supplied id.
pub fn is_operation_id(id: &str) -> bool {
    let Some(rest) = id.strip_prefix("op-") else {
        return false;
    };
    rest.len() == 28
        && rest
            .chars()
            .all(|c| c.is_ascii_digit() || matches!(c, 'T' | 'Z' | '-' | 'a'..='f'))
}
