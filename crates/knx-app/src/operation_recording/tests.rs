//! Tests for the operation recorder: format, secrets, faults, retention and deletion.

use std::io;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use knx_core::IndividualAddress;
use knx_net::cemi::{self, ApplicationService, Destination, LDataFrame, LDataMessageKind, Tpci};
use knx_net::frame;
use knx_net::tunnelling;
use knx_net::wire_tap::{TunnelTapEvent, WireDirection};
use serde_json::{json, Value};

use super::*;

const SENTINEL_KEY: [u8; 4] = [0xC0, 0xFF, 0xEE, 0x42];
const IDLE: Duration = Duration::from_secs(10);

fn identity() -> WriterIdentity {
    WriterIdentity {
        program: "knx-server".into(),
        version: "0.1.0-alpha.18+gtest".into(),
        commit: Some("0123456789abcdef".into()),
        build_profile: "debug".into(),
    }
}

fn store_in(dir: &Path) -> RecordingStore {
    RecordingStore::new(dir, identity(), false)
}

fn store_with(dir: &Path, limits: RetentionLimits, opener: RecordingOpener) -> RecordingStore {
    RecordingStore::with_options(
        dir.join(DIAGNOSTICS_DIR_NAME),
        identity(),
        false,
        limits,
        opener,
    )
}

fn authorize_datagram() -> Vec<u8> {
    let frame = LDataFrame {
        kind: LDataMessageKind::Request,
        source: IndividualAddress::from_raw(0),
        destination: Destination::Individual(IndividualAddress::from_raw(0x1105)),
        transport: Tpci::NumberedData { seq: 1 },
        service: ApplicationService::AuthorizeRequest { key: SENTINEL_KEY },
        control: None,
    };
    let cemi = cemi::encode_l_data(&frame).unwrap();
    frame::encode_frame(
        tunnelling::TUNNELLING_REQUEST,
        &tunnelling::encode_tunnelling_request(5, 2, &cemi),
    )
}

fn memory_write_datagram(seq: u8) -> Vec<u8> {
    let frame = LDataFrame {
        kind: LDataMessageKind::Request,
        source: IndividualAddress::from_raw(0),
        destination: Destination::Individual(IndividualAddress::from_raw(0x1105)),
        transport: Tpci::NumberedData { seq: 0 },
        service: ApplicationService::MemoryWrite {
            address: 0x0116,
            data: vec![1, 2, 3],
        },
        control: None,
    };
    let cemi = cemi::encode_l_data(&frame).unwrap();
    frame::encode_frame(
        tunnelling::TUNNELLING_REQUEST,
        &tunnelling::encode_tunnelling_request(5, seq, &cemi),
    )
}

fn lines(bytes: &[u8]) -> Vec<Value> {
    // An empty line closes a line a failed write may have cut short.
    String::from_utf8_lossy(bytes)
        .lines()
        .filter(|line| !line.is_empty())
        .map(|line| serde_json::from_str(line).expect("every line is JSON"))
        .collect()
}

fn only_recording(store: &RecordingStore) -> (RecordingSummary, Vec<Value>) {
    let list = store.list().unwrap();
    assert_eq!(list.len(), 1, "{list:?}");
    let bytes = store.read(&list[0].id).unwrap();
    (list[0].clone(), lines(&bytes))
}

#[test]
fn a_recording_has_a_header_its_datagrams_notes_and_an_end() {
    let dir = tempfile::tempdir().unwrap();
    let store = store_in(dir.path());
    let recorder = store.start(
        OperationStart::new(OperationKind::System2Download)
            .target(json!({"device_id": "dev-1", "individual_address": "1.1.5"}))
            .gateway("203.0.113.250:3671")
            .related("run_id", "run-7")
            .settings(json!({"ack_wait": "patient"})),
    );
    assert_eq!(recorder.status(), RecorderStatus::Recording);
    let tap = recorder.wire_tap().expect("a recording recorder has a tap");
    tap.event(TunnelTapEvent::Connected {
        channel_id: 5,
        assigned_address: IndividualAddress::from_raw(0x11ff),
    });
    let sent = memory_write_datagram(0);
    tap.datagram(WireDirection::Sent, &sent);
    recorder.note("plan", json!({"allocations": 2}));
    recorder.finish(RecordedOutcome::succeeded(json!({"written": 27})));
    assert!(store.wait_until_idle(IDLE));

    let (summary, lines) = only_recording(&store);
    assert_eq!(Some(summary.id.as_str()), recorder.id());
    assert_eq!(summary.outcome.as_deref(), Some("succeeded"));
    assert_eq!(summary.operation_kind.as_deref(), Some("system2-download"));
    assert_eq!(
        summary.writer_version.as_deref(),
        Some("0.1.0-alpha.18+gtest")
    );
    assert_eq!(summary.format_version, Some(1));
    assert!(!summary.torn_tail && !summary.active);

    let header = &lines[0];
    assert_eq!(header["type"], "header");
    assert_eq!(header["format"], RECORDING_FORMAT);
    assert_eq!(header["writer"]["version"], "0.1.0-alpha.18+gtest");
    assert_eq!(header["gateway"], "203.0.113.250:3671");
    assert_eq!(header["related"]["run_id"], "run-7");
    assert_eq!(header["settings"]["ack_wait"], "patient");
    assert!(header["platform"]["os"].is_string());

    let types: Vec<&str> = lines.iter().map(|l| l["type"].as_str().unwrap()).collect();
    assert_eq!(types, ["header", "tunnel", "datagram", "note", "end"]);
    assert_eq!(lines[1]["event"], "connected");
    let datagram = &lines[2];
    assert_eq!(datagram["raw"], encode::hex(&sent));
    assert_eq!(datagram["dir"], "sent");
    assert_eq!(datagram["service"], "TUNNELLING_REQUEST");
    assert_eq!(datagram["cemi"]["apci"], "MemoryWrite");
    assert_eq!(datagram["cemi"]["destination"], "1.1.5");
    assert!(datagram["t_us"].is_u64() && datagram["wall"].is_string());
    assert_eq!(lines[3]["data"]["allocations"], 2);
    let end = &lines[4];
    assert_eq!(end["outcome"], "succeeded");
    assert_eq!(end["summary"]["written"], 27);
    assert_eq!(end["records_dropped"], 0);
    assert_eq!(end["records_written"], 5);
    // Sequence numbers are strictly increasing over records.
    let seqs: Vec<u64> = lines[1..]
        .iter()
        .map(|l| l["seq"].as_u64().unwrap())
        .collect();
    assert!(seqs.windows(2).all(|w| w[0] < w[1]), "{seqs:?}");
}

#[test]
fn a_key_never_reaches_the_file_but_its_redaction_marker_does() {
    let dir = tempfile::tempdir().unwrap();
    let store = store_in(dir.path());
    let recorder = store.start(OperationStart::new(OperationKind::Download));
    let tap = recorder.wire_tap().unwrap();
    tap.datagram(WireDirection::Sent, &authorize_datagram());
    tap.datagram(WireDirection::Received, &authorize_datagram());
    recorder.finish(RecordedOutcome::succeeded(Value::Null));
    assert!(store.wait_until_idle(IDLE));
    let (summary, lines) = only_recording(&store);
    let raw = store.read(&summary.id).unwrap();
    let text = String::from_utf8(raw.clone()).unwrap();
    assert!(!raw.windows(4).any(|w| w == SENTINEL_KEY));
    assert!(!text.to_ascii_lowercase().contains("c0ffee42"));
    let redacted: Vec<&Value> = lines.iter().filter(|l| !l["redacted"].is_null()).collect();
    assert_eq!(redacted.len(), 2);
    for line in redacted {
        assert_eq!(line["redacted"]["kind"], "authorize-key");
        assert_eq!(line["redacted"]["octets"], 4);
        assert_eq!(line["cemi"]["summary"], "key=<redacted>");
    }
}

#[test]
fn a_recorder_dropped_without_an_outcome_ends_abandoned() {
    let dir = tempfile::tempdir().unwrap();
    let store = store_in(dir.path());
    {
        let recorder = store.start(OperationStart::new(OperationKind::Compare));
        let tap = recorder.wire_tap().unwrap();
        tap.datagram(WireDirection::Sent, &memory_write_datagram(0));
        // Both the recorder and its tap go out of scope here.
    }
    assert!(store.wait_until_idle(IDLE));
    let (summary, lines) = only_recording(&store);
    assert_eq!(summary.outcome.as_deref(), Some("abandoned"));
    assert_eq!(lines.last().unwrap()["type"], "end");
}

#[test]
fn records_after_the_outcome_are_ignored() {
    let dir = tempfile::tempdir().unwrap();
    let store = store_in(dir.path());
    let recorder = store.start(OperationStart::new(OperationKind::Compare));
    let tap = recorder.wire_tap().unwrap();
    recorder.finish(RecordedOutcome::from_error(&io::Error::other("refused")));
    recorder.finish(RecordedOutcome::succeeded(Value::Null));
    tap.datagram(WireDirection::Sent, &memory_write_datagram(0));
    recorder.note("late", Value::Null);
    assert!(store.wait_until_idle(IDLE));
    let (summary, lines) = only_recording(&store);
    assert_eq!(summary.outcome.as_deref(), Some("failed"));
    assert_eq!(lines.len(), 2, "header and end only");
    assert_eq!(lines[1]["error_chain"], json!(["refused"]));
}

#[test]
fn a_disabled_store_records_nothing_and_says_so() {
    let dir = tempfile::tempdir().unwrap();
    let store = RecordingStore::new(dir.path(), identity(), true);
    assert!(!store.is_enabled());
    let recorder = store.start(OperationStart::new(OperationKind::Download));
    assert_eq!(recorder.status(), RecorderStatus::Disabled);
    assert!(recorder.wire_tap().is_none());
    assert_eq!(recorder.id(), None);
    recorder.note("plan", Value::Null);
    recorder.finish(RecordedOutcome::succeeded(Value::Null));
    assert!(store.list().unwrap().is_empty());
    assert!(!dir.path().join(DIAGNOSTICS_DIR_NAME).exists());
}

#[test]
fn only_an_explicit_off_value_disables_recording() {
    for off in ["off", "OFF", " 0 ", "false", "disabled"] {
        assert!(disabled_by_env_value(Some(off)), "{off}");
    }
    for on in [
        None,
        Some(""),
        Some("on"),
        Some("1"),
        Some("yes"),
        Some("of"),
    ] {
        assert!(!disabled_by_env_value(on), "{on:?}");
    }
}

/// A writer that takes `delay` for every write: a slow disk or fsync.
struct Slow {
    delay: Duration,
    file: std::fs::File,
}

impl Write for Slow {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        std::thread::sleep(self.delay);
        self.file.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.file.flush()
    }
}

#[test]
fn a_slow_writer_never_slows_the_operation_and_its_losses_are_a_visible_gap() {
    let dir = tempfile::tempdir().unwrap();
    let opener: RecordingOpener = Arc::new(|path: &Path| {
        Ok(Box::new(Slow {
            delay: Duration::from_millis(2),
            file: std::fs::File::create(path)?,
        }) as Box<dyn Write + Send>)
    });
    let store = store_with(dir.path(), RetentionLimits::default(), opener);
    let recorder = store.start(OperationStart::new(OperationKind::Download));
    let tap = recorder.wire_tap().unwrap();
    let datagram = memory_write_datagram(1);
    let started = Instant::now();
    let total = QUEUE_CAPACITY as u64 * 2;
    for _ in 0..total {
        tap.datagram(WireDirection::Sent, &datagram);
    }
    let enqueue_time = started.elapsed();
    // 8192 records at 2 ms each would take over 16 s if record() waited.
    assert!(
        enqueue_time < Duration::from_secs(3),
        "recording took {enqueue_time:?}"
    );
    recorder.finish(RecordedOutcome::succeeded(Value::Null));
    assert!(store.wait_until_idle(Duration::from_secs(120)));
    let (summary, lines) = only_recording(&store);
    assert_eq!(summary.outcome.as_deref(), Some("succeeded"));
    let gaps: u64 = lines
        .iter()
        .filter(|l| l["type"] == "gap")
        .map(|l| l["lost_records"].as_u64().unwrap())
        .sum();
    let datagrams = lines.iter().filter(|l| l["type"] == "datagram").count() as u64;
    let end = lines.last().unwrap();
    assert!(gaps > 0, "the queue overflowed");
    assert_eq!(end["records_dropped"].as_u64().unwrap(), gaps);
    assert_eq!(
        datagrams + gaps,
        total,
        "every record is written or counted"
    );
    assert!(lines
        .iter()
        .any(|l| l["type"] == "gap" && l["reason"] == "recorder queue full"));
}

/// A writer whose writes 3 to 6 fail: a disk that filled up and freed again.
struct Flaky {
    calls: Arc<AtomicUsize>,
    file: std::fs::File,
}

impl Write for Flaky {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let call = self.calls.fetch_add(1, Ordering::SeqCst) + 1;
        if (3..=6).contains(&call) {
            return Err(io::Error::new(io::ErrorKind::StorageFull, "disk full"));
        }
        self.file.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.file.flush()
    }
}

#[test]
fn failed_writes_never_reach_the_operation_and_leave_a_gap() {
    let dir = tempfile::tempdir().unwrap();
    let calls = Arc::new(AtomicUsize::new(0));
    let opener_calls = calls.clone();
    let opener: RecordingOpener = Arc::new(move |path: &Path| {
        Ok(Box::new(Flaky {
            calls: opener_calls.clone(),
            file: std::fs::File::create(path)?,
        }) as Box<dyn Write + Send>)
    });
    let store = store_with(dir.path(), RetentionLimits::default(), opener);
    let recorder = store.start(OperationStart::new(OperationKind::Download));
    for index in 0..8 {
        recorder.note("step", json!({"index": index}));
    }
    recorder.finish(RecordedOutcome::succeeded(Value::Null));
    assert!(store.wait_until_idle(IDLE));
    let (summary, lines) = only_recording(&store);
    assert_eq!(summary.outcome.as_deref(), Some("succeeded"));
    let gap = lines
        .iter()
        .find(|l| l["type"] == "gap")
        .expect("a gap line after the failures");
    assert!(
        gap["reason"].as_str().unwrap().contains("disk full"),
        "{gap}"
    );
    let end = lines.last().unwrap();
    assert!(end["failed_writes"].as_u64().unwrap() >= 1);
    let written: Vec<u64> = lines
        .iter()
        .filter(|l| l["name"] == "step")
        .map(|l| l["data"]["index"].as_u64().unwrap())
        .collect();
    let lost = gap["lost_records"].as_u64().unwrap();
    assert_eq!(written.len() as u64 + lost, 8, "{written:?} lost {lost}");
}

#[test]
fn a_recording_that_cannot_be_created_is_unavailable_not_an_error() {
    let dir = tempfile::tempdir().unwrap();
    let opener: RecordingOpener = Arc::new(|_: &Path| Err(io::Error::other("read-only volume")));
    let store = store_with(dir.path(), RetentionLimits::default(), opener);
    let recorder = store.start(OperationStart::new(OperationKind::Download));
    let tap = recorder.wire_tap().unwrap();
    for _ in 0..100 {
        tap.datagram(WireDirection::Sent, &memory_write_datagram(0));
    }
    recorder.finish(RecordedOutcome::succeeded(Value::Null));
    assert!(store.wait_until_idle(IDLE));
    assert_eq!(recorder.status(), RecorderStatus::Unavailable);
    assert!(store.list().unwrap().is_empty());
}

fn finished(store: &RecordingStore, status: OutcomeStatus) -> String {
    let recorder = store.start(OperationStart::new(OperationKind::Download));
    let id = recorder.id().unwrap().to_string();
    recorder.finish(RecordedOutcome {
        status,
        error_chain: Vec::new(),
        summary: Value::Null,
    });
    assert!(store.wait_until_idle(IDLE));
    // Distinct, ordered ids: the id carries milliseconds.
    std::thread::sleep(Duration::from_millis(3));
    id
}

#[test]
fn retention_evicts_succeeded_recordings_before_failed_ones() {
    let dir = tempfile::tempdir().unwrap();
    let limits = RetentionLimits {
        max_recordings: 3,
        max_bytes: u64::MAX,
    };
    let store = store_with(dir.path(), limits, default_opener());
    let failed_old = finished(&store, OutcomeStatus::Failed);
    let ok_1 = finished(&store, OutcomeStatus::Succeeded);
    let abandoned = finished(&store, OutcomeStatus::Abandoned);
    let ok_2 = finished(&store, OutcomeStatus::Succeeded);
    let ok_3 = finished(&store, OutcomeStatus::Succeeded);
    let ids: Vec<String> = store.list().unwrap().into_iter().map(|s| s.id).collect();
    assert_eq!(ids, [ok_3.clone(), abandoned.clone(), failed_old.clone()]);
    assert!(!ids.contains(&ok_1) && !ids.contains(&ok_2));
    // The last success still goes before any failure.
    let failed_new = finished(&store, OutcomeStatus::Failed);
    let ids: Vec<String> = store.list().unwrap().into_iter().map(|s| s.id).collect();
    assert_eq!(
        ids,
        [failed_new.clone(), abandoned.clone(), failed_old.clone()]
    );
    // Once only failures remain, the oldest failure goes first.
    let failed_newest = finished(&store, OutcomeStatus::Failed);
    let ids: Vec<String> = store.list().unwrap().into_iter().map(|s| s.id).collect();
    assert_eq!(ids, [failed_newest, failed_new, abandoned]);
}

#[test]
fn retention_also_holds_the_byte_limit() {
    let dir = tempfile::tempdir().unwrap();
    let probe = store_in(dir.path());
    finished(&probe, OutcomeStatus::Succeeded);
    let one = probe.list().unwrap()[0].size_bytes;
    probe.delete_all().unwrap();
    let limits = RetentionLimits {
        max_recordings: 100,
        max_bytes: one * 2 + one / 2,
    };
    let store = store_with(dir.path(), limits, default_opener());
    for _ in 0..4 {
        finished(&store, OutcomeStatus::Succeeded);
    }
    let list = store.list().unwrap();
    assert_eq!(list.len(), 2, "{list:?}");
    assert!(list.iter().map(|s| s.size_bytes).sum::<u64>() <= limits.max_bytes);
}

#[test]
fn a_torn_last_line_is_reported_and_the_recording_still_lists() {
    let dir = tempfile::tempdir().unwrap();
    let store = store_in(dir.path());
    let id = finished(&store, OutcomeStatus::Succeeded);
    let path = store.root().join(format!("{id}.jsonl"));
    let mut bytes = std::fs::read(&path).unwrap();
    // Cut the end line in half, as a crash mid-write would.
    let cut = bytes.len() - 20;
    bytes.truncate(cut);
    std::fs::write(&path, bytes).unwrap();
    let (summary, _) = {
        let list = store.list().unwrap();
        (list[0].clone(), ())
    };
    assert!(summary.torn_tail);
    assert_eq!(summary.outcome, None, "no complete end line");
    assert_eq!(summary.operation_kind.as_deref(), Some("download"));
}

#[test]
fn delete_all_removes_finished_recordings_and_keeps_the_running_one() {
    let dir = tempfile::tempdir().unwrap();
    let store = store_in(dir.path());
    finished(&store, OutcomeStatus::Succeeded);
    finished(&store, OutcomeStatus::Failed);
    let running = store.start(OperationStart::new(OperationKind::LineScan));
    running.note("probe", Value::Null);
    // Wait until the running recording's file exists.
    let deadline = Instant::now() + IDLE;
    while store.list().unwrap().len() < 3 {
        assert!(
            Instant::now() < deadline,
            "running recording never appeared"
        );
        std::thread::sleep(Duration::from_millis(5));
    }
    let outcome = store.delete_all().unwrap();
    assert_eq!(outcome.removed, 2);
    assert_eq!(outcome.kept_active, [running.id().unwrap().to_string()]);
    running.finish(RecordedOutcome::succeeded(Value::Null));
    assert!(store.wait_until_idle(IDLE));
    assert_eq!(store.delete_all().unwrap().removed, 1);
    assert!(store.list().unwrap().is_empty());
}

#[test]
fn only_generated_operation_ids_name_a_recording_path() {
    let dir = tempfile::tempdir().unwrap();
    let store = store_in(dir.path());
    let id = finished(&store, OutcomeStatus::Succeeded);
    assert!(is_operation_id(&id), "{id}");
    for bad in [
        "../etc/passwd",
        "op-../../x",
        "op-20261029T120000000Z-zzzzzzzz",
        "",
        "op-20261029T120000000Z-0123456",
    ] {
        assert!(!is_operation_id(bad), "{bad}");
        assert_eq!(
            store.read(bad).unwrap_err().kind(),
            io::ErrorKind::InvalidInput
        );
    }
    // Unrelated files in the directory are not recordings.
    std::fs::write(store.root().join("notes.txt"), b"x").unwrap();
    std::fs::write(store.root().join("op-x.jsonl"), b"x").unwrap();
    assert_eq!(store.list().unwrap().len(), 1);
}
