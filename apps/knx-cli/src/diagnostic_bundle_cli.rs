//! CLI `--diagnostic-bundle <file>`: record this command's device operations, then write one ZIP.
//!
//! The CLI has no data directory, so the recordings of one invocation live
//! in a hidden temporary directory next to the requested bundle file and
//! are removed after the bundle is written. The flag is the user's explicit
//! request (and consent) for this one bundle; without it nothing is
//! recorded. One process-wide session is the deliberate exception to "no
//! global state": every hand-parsed device subcommand opens its tunnel
//! through [`tapped`] without each parser having to carry the session.

use std::net::SocketAddrV4;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

use knx_app::diagnostic_bundle::{
    self as bundle, BundleFile, BundleRequest, BundleSection, ProblemType, SectionKind,
    MAX_RAW_RECORDING_BYTES, PRIVATE_RAW_NOTE, TORN_TAIL_NOTE,
};
use knx_app::operation_recording::{
    OperationKind, OperationRecorder, OperationStart, RecordedOutcome, RecordingStore,
    RetentionLimits, WriterIdentity,
};
use knx_app::property_validation::{
    private_projection::{
        admit_snapshot, read_snapshot, AdmittedPrivateProjection, PrivateSnapshot,
    },
    Binding, ExecutionBuild, OperationProgress, System2Report,
};
use knx_net::KnxNetIpClient;
use serde_json::json;

/// The flag, as it appears anywhere on the command line.
pub const FLAG: &str = "--diagnostic-bundle";

/// The System-2 subcommand; only it produces (or misses) System-2 evidence.
const SYSTEM2_COMMAND: &str = "system2-validation";

struct Session {
    store: RecordingStore,
    bundle: PathBuf,
    command: String,
    recorders: Mutex<Vec<OperationRecorder>>,
    /// The recordings' directory; statics are never dropped, so [`finish`]
    /// takes and removes it explicitly.
    scratch: Mutex<Option<tempfile::TempDir>>,
    system2: Mutex<CliSystem2Slot>,
}

enum CliSystem2Slot {
    Empty,
    Captured {
        progress: Box<OperationProgress>,
        projection: AdmittedPrivateProjection,
    },
    Missing,
    Unavailable,
    Duplicate,
    Consumed,
}

static SESSION: OnceLock<Session> = OnceLock::new();

/// Removes `--diagnostic-bundle <file>` (or `--diagnostic-bundle=<file>`)
/// from `args`; returns the remaining arguments and the file, if given.
pub fn take_flag(args: Vec<String>) -> Result<(Vec<String>, Option<PathBuf>), String> {
    let mut rest = Vec::with_capacity(args.len());
    let mut file = None;
    let mut iter = args.into_iter();
    while let Some(arg) = iter.next() {
        let value = if arg == FLAG {
            Some(iter.next().ok_or(format!("{FLAG} needs a file name"))?)
        } else {
            arg.strip_prefix(&format!("{FLAG}=")).map(str::to_string)
        };
        match value {
            Some(value) if file.is_some() => {
                return Err(format!("{FLAG} given twice (second: {value})"))
            }
            Some(value) if value.is_empty() || value.starts_with("--") => {
                return Err(format!("{FLAG} needs a file name"))
            }
            Some(value) => file = Some(PathBuf::from(value)),
            None => rest.push(arg),
        }
    }
    Ok((rest, file))
}

fn writer_identity() -> WriterIdentity {
    WriterIdentity {
        program: "knx-cli".into(),
        version: crate::version_string(),
        commit: option_env!("KNX_BUILD_SHA")
            .filter(|sha| !sha.is_empty())
            .map(str::to_string),
        build_profile: if cfg!(debug_assertions) {
            "debug"
        } else {
            "release"
        }
        .into(),
    }
}

/// Starts the session for `bundle`, refusing an existing file before any
/// device contact. `command` names the subcommand for the manifest.
pub fn begin(bundle: &Path, command: &str) -> Result<(), String> {
    if bundle.exists() {
        return Err(format!(
            "{FLAG}: {} already exists; nothing was started",
            bundle.display()
        ));
    }
    let parent = match bundle.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent.to_path_buf(),
        _ => PathBuf::from("."),
    };
    let scratch = tempfile::Builder::new()
        .prefix(".knxbench-recordings-")
        .tempdir_in(&parent)
        .map_err(|e| {
            format!("{FLAG}: cannot create a recording directory beside the bundle: {e}")
        })?;
    let store = RecordingStore::with_options(
        scratch.path().to_path_buf(),
        writer_identity(),
        false,
        RetentionLimits::default(),
        std::sync::Arc::new(|path: &Path| {
            let file = std::fs::OpenOptions::new()
                .create_new(true)
                .append(true)
                .open(path)?;
            Ok(Box::new(file) as Box<dyn std::io::Write + Send>)
        }),
    );
    SESSION
        .set(Session {
            store,
            bundle: bundle.to_path_buf(),
            command: command.to_string(),
            recorders: Mutex::new(Vec::new()),
            scratch: Mutex::new(Some(scratch)),
            system2: Mutex::new(CliSystem2Slot::Empty),
        })
        .map_err(|_| format!("{FLAG}: a session already exists"))
}

/// Captures one closed System-2 projection after its writer released the store.
/// Without a bundle session this returns before inspecting either path or binding.
pub fn capture_system2(operation: &Path, binding: &Binding) -> Result<(), String> {
    let Some(session) = SESSION.get() else {
        return Ok(());
    };
    let mut slot = session
        .system2
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if !matches!(*slot, CliSystem2Slot::Empty) {
        *slot = CliSystem2Slot::Duplicate;
        return Err(format!(
            "{FLAG}: System-2 evidence was captured more than once"
        ));
    }
    *slot = match read_snapshot(operation, binding) {
        PrivateSnapshot::Available(snapshot) => match admit_snapshot(*snapshot) {
            Ok((progress, projection)) => CliSystem2Slot::Captured {
                progress: Box::new(progress),
                projection,
            },
            Err(_) => CliSystem2Slot::Unavailable,
        },
        PrivateSnapshot::Unavailable(_) => CliSystem2Slot::Unavailable,
    };
    match *slot {
        CliSystem2Slot::Captured { .. } => Ok(()),
        _ => Err(format!("{FLAG}: System-2 evidence is unavailable")),
    }
}

/// `client`, tapped into a new recording when a bundle was requested;
/// unchanged otherwise. The recording ends when [`finish`] runs.
pub fn tapped(
    client: KnxNetIpClient,
    kind: OperationKind,
    gateway: SocketAddrV4,
) -> KnxNetIpClient {
    let Some(session) = SESSION.get() else {
        return client;
    };
    let recorder = session.store.start(
        OperationStart::new(kind)
            .gateway(gateway)
            .related("command", &session.command),
    );
    let client = match recorder.wire_tap() {
        Some(tap) => client.with_wire_tap(tap),
        None => client,
    };
    session
        .recorders
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .push(recorder);
    client
}

/// What kind of problem a subcommand's bundle is about.
fn problem_for(command: &str) -> ProblemType {
    match command {
        "download" | "restore" | "system2-validation" | "service-control" | "program-address"
        | "reset-address" | "address-by-serial" => ProblemType::DeviceProgramming,
        _ => ProblemType::DeviceReading,
    }
}

/// Ends every recording with the command's exit status and writes the
/// bundle. Returns the bundle path; an error leaves no bundle file.
pub fn finish(succeeded: bool) -> Option<Result<PathBuf, String>> {
    let session = SESSION.get()?;
    let recorders = std::mem::take(
        &mut *session
            .recorders
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner),
    );
    for recorder in &recorders {
        recorder.finish(if succeeded {
            RecordedOutcome::succeeded(json!({ "command": session.command }))
        } else {
            RecordedOutcome::failed(
                vec![format!(
                    "the command `{}` ended with a failure; its messages went to the terminal",
                    session.command
                )],
                json!({ "command": session.command }),
            )
        });
    }
    drop(recorders);
    let written = write(session);
    let scratch = session
        .scratch
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .take();
    let removed = scratch.map_or(Ok(()), tempfile::TempDir::close);
    Some(match (written, removed) {
        (Ok(path), Err(error)) => {
            eprintln!("{FLAG}: the temporary recording directory could not be removed: {error}");
            Ok(path)
        }
        (written, _) => written,
    })
}

fn write(session: &Session) -> Result<PathBuf, String> {
    if !session.store.wait_until_idle(Duration::from_secs(30)) {
        return Err(format!("{FLAG}: the recordings did not finish writing"));
    }
    let created_at = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    let problem = problem_for(&session.command);
    let mut environment = BundleSection::new(SectionKind::Environment);
    environment.entries = 1;
    environment.files.push(BundleFile::json(
        "environment.json",
        &json!({
            "writer": writer_identity(),
            "platform": {
                "os": std::env::consts::OS,
                "family": std::env::consts::FAMILY,
                "arch": std::env::consts::ARCH,
            },
            "commandClass": problem,
        }),
    ));
    let mut sections = vec![environment];
    let captured = {
        let mut slot = session
            .system2
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if matches!(*slot, CliSystem2Slot::Empty) {
            *slot = CliSystem2Slot::Missing;
        }
        std::mem::replace(&mut *slot, CliSystem2Slot::Consumed)
    };
    match captured {
        CliSystem2Slot::Captured {
            progress,
            projection,
        } => {
            let report = System2Report::from_progress(
                &progress,
                ExecutionBuild::Unknown,
                &crate::version_string(),
            )
            .ok_or_else(|| format!("{FLAG}: System-2 report is unavailable"))?;
            let mut operation = BundleSection::new(SectionKind::Operation);
            operation.entries = 1;
            operation.files.push(BundleFile::new(
                "system2-private.json",
                projection.bytes().to_vec(),
            ));
            sections.push(operation);
            let mut reports = BundleSection::new(SectionKind::LegacyReports);
            reports.entries = 1;
            reports.files.push(BundleFile::json(
                "system2-report.json",
                &serde_json::to_value(report)
                    .map_err(|_| format!("{FLAG}: System-2 report is unavailable"))?,
            ));
            sections.push(reports);
        }
        CliSystem2Slot::Duplicate => {
            return Err(format!(
                "{FLAG}: System-2 evidence was captured more than once"
            ));
        }
        slot @ (CliSystem2Slot::Unavailable | CliSystem2Slot::Empty | CliSystem2Slot::Missing) => {
            sections[0]
                .notes
                .extend(system2_note(&session.command, &slot).map(str::to_string));
        }
        CliSystem2Slot::Consumed => {
            return Err(format!("{FLAG}: diagnostic evidence was already consumed"));
        }
    }
    let raw = raw_recording_sections(&session.command, &session.store, &mut sections[0]);
    sections.extend(raw);
    let request = BundleRequest {
        problem,
        writer: writer_identity(),
        recording_enabled: true,
        consent_at: created_at.clone(),
        description: Some("User description withheld from diagnostic bundle.".into()),
    };
    let zip = bundle::build_zip(&request, &sections, &created_at).map_err(|e| e.to_string())?;
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&session.bundle)
        .map_err(|e| format!("{FLAG}: {}: {e}", session.bundle.display()))?;
    std::io::Write::write_all(&mut file, &zip).map_err(|e| e.to_string())?;
    file.sync_all().map_err(|e| e.to_string())?;
    Ok(session.bundle.clone())
}

/// The environment note for a System-2 slot without evidence. Only the
/// System-2 validation command, or a capture that failed, has System-2
/// evidence to miss; every other command, `system2-*` or not, says nothing.
fn system2_note(command: &str, slot: &CliSystem2Slot) -> Option<&'static str> {
    const NOTE: &str = "System-2 evidence was unavailable for this invocation";
    match slot {
        CliSystem2Slot::Unavailable => Some(NOTE),
        CliSystem2Slot::Empty | CliSystem2Slot::Missing if command == SYSTEM2_COMMAND => Some(NOTE),
        _ => None,
    }
}

/// This invocation's non-System-2 recordings, exactly as recorded (keys are
/// redacted at capture). The flag is the user's consent for this one bundle;
/// owner decision 31 October 2026 lets such raw recordings leave after consent
/// with the keep-private note. System-2 recordings never leave raw: their
/// admitted projection is the only System-2 evidence, and no recording of a
/// `system2-*` subcommand leaves raw whatever its kind. Only the newest
/// recording can be the operation; older ones are earlier operations. Every
/// recording left out is named in a note on `environment`, never dropped silently.
fn raw_recording_sections(
    command: &str,
    store: &RecordingStore,
    environment: &mut BundleSection,
) -> Vec<BundleSection> {
    let recordings = match store.list() {
        Ok(recordings) => recordings,
        Err(_) => {
            environment
                .notes
                .push("the recordings of this invocation could not be listed".into());
            return Vec::new();
        }
    };
    if command.starts_with("system2-") {
        if !recordings.is_empty() {
            environment.notes.push(format!(
                "{} recording(s) of this System-2 command left out: \
                 raw recordings of System-2 commands are never included",
                recordings.len()
            ));
        }
        return Vec::new();
    }
    let mut sections = Vec::new();
    for (index, summary) in recordings.into_iter().enumerate() {
        let kind = summary.operation_kind.as_deref();
        if summary.format_version != Some(1) || kind.is_none() {
            environment.notes.push(format!(
                "recording {} was left out: its header could not be read",
                summary.id
            ));
            continue;
        }
        if kind.is_some_and(|kind| kind.starts_with("system2-")) {
            environment.notes.push(format!(
                "recording {} was left out: raw System-2 recordings are never included",
                summary.id
            ));
            continue;
        }
        let Ok(bytes) = store.read_bounded(&summary.id, MAX_RAW_RECORDING_BYTES) else {
            environment.notes.push(format!(
                "recording {} was left out: unreadable or larger than 64 MiB",
                summary.id
            ));
            continue;
        };
        let kind = if index == 0 {
            SectionKind::Operation
        } else {
            SectionKind::EarlierOperation
        };
        let mut section = BundleSection::new(kind);
        section.label = Some(summary.id.clone());
        section.entries = bytes.iter().filter(|byte| **byte == b'\n').count();
        section.markers = bundle::ContentMarkers {
            addresses: true,
            names: true,
            parameter_values: true,
            network_addresses: true,
        };
        section.notes.push(PRIVATE_RAW_NOTE.into());
        if summary.torn_tail {
            section.notes.push(TORN_TAIL_NOTE.into());
        }
        section
            .files
            .push(BundleFile::new("recording.jsonl", bytes));
        sections.push(section);
    }
    sections
}

#[cfg(test)]
mod tests {
    use super::*;

    fn store(dir: &Path) -> RecordingStore {
        RecordingStore::with_options(
            dir.to_path_buf(),
            writer_identity(),
            false,
            RetentionLimits::default(),
            std::sync::Arc::new(|path: &Path| {
                let file = std::fs::OpenOptions::new()
                    .create_new(true)
                    .append(true)
                    .open(path)?;
                Ok(Box::new(file) as Box<dyn std::io::Write + Send>)
            }),
        )
    }

    fn record(store: &RecordingStore, kind: OperationKind) -> String {
        let recorder = store.start(OperationStart::new(kind));
        recorder.finish(RecordedOutcome::succeeded(json!({})));
        assert!(store.wait_until_idle(Duration::from_secs(10)));
        // Distinct, ordered ids: the id carries milliseconds.
        std::thread::sleep(Duration::from_millis(3));
        recorder.id().unwrap().to_string()
    }

    fn raw(command: &str, store: &RecordingStore) -> (Vec<BundleSection>, Vec<String>) {
        let mut environment = BundleSection::new(SectionKind::Environment);
        let sections = raw_recording_sections(command, store, &mut environment);
        (sections, environment.notes)
    }

    #[test]
    fn a_system2_command_never_ships_a_raw_recording_whatever_its_kind() {
        let dir = tempfile::tempdir().unwrap();
        let store = store(dir.path());
        // `system2-project` taps a plain device read.
        record(&store, OperationKind::DeviceRead);
        let (sections, notes) = raw("system2-project", &store);
        assert!(sections.is_empty());
        assert_eq!(
            notes,
            ["1 recording(s) of this System-2 command left out: \
              raw recordings of System-2 commands are never included"]
        );
        // A System-2 command that never connected says nothing about recordings.
        let empty = tempfile::tempdir().unwrap();
        let (sections, notes) = raw("system2-prepare", &self::store(empty.path()));
        assert!(sections.is_empty() && notes.is_empty(), "{notes:?}");
    }

    #[test]
    fn system2_kinds_are_left_out_with_a_note_and_the_newest_alone_is_the_operation() {
        let dir = tempfile::tempdir().unwrap();
        let store = store(dir.path());
        let older = record(&store, OperationKind::DeviceRead);
        let system2 = record(&store, OperationKind::System2Download);
        let (sections, notes) = raw("compare", &store);
        // The newest listed recording (System-2) is left out, so no section is the operation.
        let kinds: Vec<_> = sections.iter().map(|s| (s.kind, s.label.clone())).collect();
        assert_eq!(kinds, [(SectionKind::EarlierOperation, Some(older))]);
        assert_eq!(
            notes,
            [format!(
                "recording {system2} was left out: raw System-2 recordings are never included"
            )]
        );
        assert_eq!(sections[0].notes, [PRIVATE_RAW_NOTE]);
        assert_eq!(sections[0].files[0].name, "recording.jsonl");
    }

    #[test]
    fn only_the_validation_command_or_a_failed_capture_misses_system2_evidence() {
        let note = Some("System-2 evidence was unavailable for this invocation");
        for command in [
            "system2-prepare",
            "system2-project",
            "find-serial",
            "compare",
        ] {
            assert_eq!(
                system2_note(command, &CliSystem2Slot::Missing),
                None,
                "{command}"
            );
            assert_eq!(
                system2_note(command, &CliSystem2Slot::Empty),
                None,
                "{command}"
            );
            assert_eq!(
                system2_note(command, &CliSystem2Slot::Unavailable),
                note,
                "{command}"
            );
        }
        assert_eq!(
            system2_note(SYSTEM2_COMMAND, &CliSystem2Slot::Missing),
            note
        );
        assert_eq!(system2_note(SYSTEM2_COMMAND, &CliSystem2Slot::Empty), note);
        assert_eq!(
            system2_note(SYSTEM2_COMMAND, &CliSystem2Slot::Duplicate),
            None
        );
    }

    #[test]
    fn a_recording_without_a_readable_header_is_named_not_dropped() {
        let dir = tempfile::tempdir().unwrap();
        let store = store(dir.path());
        let id = "op-20261031T120000000Z-0000abcd";
        std::fs::write(dir.path().join(format!("{id}.jsonl")), b"not json\n").unwrap();
        let (sections, notes) = raw("compare", &store);
        assert!(sections.is_empty());
        assert_eq!(
            notes,
            [format!(
                "recording {id} was left out: its header could not be read"
            )]
        );
    }

    fn args(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn the_flag_is_taken_out_wherever_it_stands() {
        let (rest, file) = take_flag(args(&[
            "device",
            "compare",
            "--diagnostic-bundle",
            "b.zip",
            "1.1.5",
        ]))
        .unwrap();
        assert_eq!(rest, args(&["device", "compare", "1.1.5"]));
        assert_eq!(file, Some(PathBuf::from("b.zip")));
        let (rest, file) =
            take_flag(args(&["device", "--diagnostic-bundle=x.zip", "scan"])).unwrap();
        assert_eq!(rest, args(&["device", "scan"]));
        assert_eq!(file, Some(PathBuf::from("x.zip")));
        let (rest, file) = take_flag(args(&["device", "compare"])).unwrap();
        assert_eq!(rest, args(&["device", "compare"]));
        assert_eq!(file, None);
        assert!(take_flag(args(&["--diagnostic-bundle"])).is_err());
        assert!(take_flag(args(&["--diagnostic-bundle", "--gateway"])).is_err());
        assert!(take_flag(args(&[
            "--diagnostic-bundle",
            "a.zip",
            "--diagnostic-bundle",
            "b.zip"
        ]))
        .is_err());
    }

    #[test]
    fn writing_commands_are_programming_problems() {
        assert_eq!(problem_for("download"), ProblemType::DeviceProgramming);
        assert_eq!(
            problem_for("system2-validation"),
            ProblemType::DeviceProgramming
        );
        assert_eq!(problem_for("compare"), ProblemType::DeviceReading);
        assert_eq!(problem_for("scan"), ProblemType::DeviceReading);
    }

    #[test]
    fn without_a_session_the_client_is_untouched() {
        // No session is begun in unit tests: `tapped` adds nothing.
        let gateway = "203.0.113.250:3671".parse().unwrap();
        let _client = tapped(KnxNetIpClient::new(), OperationKind::Compare, gateway);
        assert!(finish(true).is_none());
    }

    #[test]
    fn without_the_flag_system2_capture_performs_zero_path_io() {
        let binding = Binding {
            target: "1.1.25".into(),
            mask: 0x0025,
            manufacturer: 2,
            serial: [0, 2, 1, 2, 3, 4],
            hardware: [1; 6],
            application: [0, 2, 0x12, 0x34, 1],
            profile_id: "zero-io-control".into(),
            profile_version: 3,
            profile_sha256: "a".repeat(64),
            source_sha256: "b".repeat(64),
            plan_sha256: "c".repeat(64),
            simulated: true,
            project_preparation_sha256: None,
        };
        let unreachable = Path::new("/path-that-must-not-be-inspected/operation");
        assert_eq!(capture_system2(unreachable, &binding), Ok(()));
        assert!(finish(false).is_none());
    }
}
