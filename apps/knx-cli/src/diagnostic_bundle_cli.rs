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
};
use knx_app::operation_recording::{
    OperationKind, OperationRecorder, OperationStart, RecordedOutcome, RecordingStore,
    RetentionLimits, WriterIdentity,
};
use knx_net::KnxNetIpClient;
use serde_json::json;

/// The flag, as it appears anywhere on the command line.
pub const FLAG: &str = "--diagnostic-bundle";

struct Session {
    store: RecordingStore,
    bundle: PathBuf,
    command: String,
    recorders: Mutex<Vec<OperationRecorder>>,
    /// The recordings' directory; statics are never dropped, so [`finish`]
    /// takes and removes it explicitly.
    scratch: Mutex<Option<tempfile::TempDir>>,
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
        })
        .map_err(|_| format!("{FLAG}: a session already exists"))
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
    let recordings = session.store.list().map_err(|e| e.to_string())?;
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
            "command": session.command,
            "recordings": recordings.len(),
        }),
    ));
    let mut sections = vec![environment];
    // Oldest first: the first operation is "the" operation, the rest earlier ones.
    for (index, summary) in recordings.iter().rev().enumerate() {
        let kind = if index == 0 {
            SectionKind::Operation
        } else {
            SectionKind::EarlierOperation
        };
        let mut section = BundleSection::new(kind);
        let bytes = session.store.read(&summary.id).map_err(|e| e.to_string())?;
        section.label = Some(summary.id.clone());
        section.entries = bytes.iter().filter(|b| **b == b'\n').count();
        section.markers.addresses = true;
        section.markers.network_addresses = true;
        section
            .files
            .push(BundleFile::new("recording.jsonl", bytes));
        sections.push(section);
    }
    if recordings.is_empty() {
        sections[0]
            .notes
            .push("the command opened no device connection, so nothing was recorded".into());
    }
    let request = BundleRequest {
        problem,
        writer: writer_identity(),
        recording_enabled: true,
        consent_at: created_at.clone(),
        description: Some(format!(
            "written by `knx device {}` with {FLAG}",
            session.command
        )),
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

#[cfg(test)]
mod tests {
    use super::*;

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
}
