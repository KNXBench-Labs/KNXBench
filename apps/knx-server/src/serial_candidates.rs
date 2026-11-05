//! List bounded server-local USB-serial aliases without opening or probing a device.

#[cfg(any(target_os = "linux", test))]
use std::{
    collections::BTreeMap,
    ffi::OsString,
    io,
    path::{Path, PathBuf},
};

use axum::{extract::State, Json};
use serde::Serialize;

use crate::SharedState;

#[cfg(any(target_os = "linux", test))]
const ALIASES: &str = "/dev/serial/by-id";
#[cfg(any(target_os = "linux", test))]
const DEVICES: &str = "/dev";
pub const MAX_SCANNED_ENTRIES: usize = 512;
pub const MAX_CANDIDATES: usize = 64;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SerialCandidate {
    pub path: String,
    pub target: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum UnavailableReason {
    MissingDirectory,
    PermissionDenied,
    Io,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "status", rename_all = "camelCase")]
pub enum SerialCandidates {
    #[serde(rename_all = "camelCase")]
    Available {
        candidates: Vec<SerialCandidate>,
        truncated: bool,
        omitted_entries: usize,
    },
    Unavailable {
        reason: UnavailableReason,
    },
    Unsupported,
}

/// Narrow passive scanner seam; production never derives roots from environment or HTTP input.
pub trait SerialCandidateScanner: Send + Sync {
    fn scan(&self) -> SerialCandidates;
}

pub struct LocalSerialCandidates;
impl SerialCandidateScanner for LocalSerialCandidates {
    fn scan(&self) -> SerialCandidates {
        platform_scan()
    }
}

#[cfg(target_os = "linux")]
fn platform_scan() -> SerialCandidates {
    scan(&LocalFilesystem, Path::new(ALIASES), Path::new(DEVICES))
}
#[cfg(not(target_os = "linux"))]
fn platform_scan() -> SerialCandidates {
    SerialCandidates::Unsupported
}

pub(crate) async fn list(State(state): State<SharedState>) -> Json<SerialCandidates> {
    let scanner = state.serial_candidates.clone();
    // Filesystem traversal/metadata must not block the async reactor. No session/connector locks.
    Json(
        tokio::task::spawn_blocking(move || scanner.scan())
            .await
            .unwrap_or(SerialCandidates::Unavailable {
                reason: UnavailableReason::Io,
            }),
    )
}

#[cfg(any(target_os = "linux", test))]
type Entries = Box<dyn Iterator<Item = io::Result<OsString>>>;
#[cfg(any(target_os = "linux", test))]
trait Filesystem {
    fn entries(&self, directory: &Path) -> io::Result<Entries>;
    fn is_symlink(&self, path: &Path) -> io::Result<bool>;
    fn canonicalize(&self, path: &Path) -> io::Result<PathBuf>;
    fn is_character_device(&self, path: &Path) -> io::Result<bool>;
}

#[cfg(target_os = "linux")]
struct LocalFilesystem;
#[cfg(target_os = "linux")]
impl Filesystem for LocalFilesystem {
    fn entries(&self, directory: &Path) -> io::Result<Entries> {
        Ok(Box::new(
            std::fs::read_dir(directory)?.map(|entry| entry.map(|entry| entry.file_name())),
        ))
    }
    fn is_symlink(&self, path: &Path) -> io::Result<bool> {
        Ok(std::fs::symlink_metadata(path)?.file_type().is_symlink())
    }
    fn canonicalize(&self, path: &Path) -> io::Result<PathBuf> {
        std::fs::canonicalize(path)
    }
    fn is_character_device(&self, path: &Path) -> io::Result<bool> {
        use std::os::unix::fs::FileTypeExt;
        Ok(std::fs::metadata(path)?.file_type().is_char_device())
    }
}

#[cfg(any(target_os = "linux", test))]
fn unavailable(error: io::Error, directory: bool) -> SerialCandidates {
    let reason = match error.kind() {
        io::ErrorKind::NotFound if directory => UnavailableReason::MissingDirectory,
        io::ErrorKind::PermissionDenied => UnavailableReason::PermissionDenied,
        _ => UnavailableReason::Io,
    };
    SerialCandidates::Unavailable { reason }
}

#[cfg(any(target_os = "linux", test))]
fn safe_alias(name: &str) -> bool {
    name.starts_with("usb-")
        && name.len() > 4
        && name.len() <= 255
        && name.trim() == name
        && !name.chars().any(char::is_control)
        && !name.contains(['/', '\\'])
}
#[cfg(any(target_os = "linux", test))]
fn tty_target(path: &Path, devices: &Path) -> bool {
    path.parent() == Some(devices)
        && path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| {
                name.strip_prefix("ttyUSB")
                    .or_else(|| name.strip_prefix("ttyACM"))
                    .is_some_and(|digits| {
                        !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit())
                    })
            })
}

#[cfg(any(target_os = "linux", test))]
fn scan(fs: &impl Filesystem, aliases: &Path, devices: &Path) -> SerialCandidates {
    let mut entries = match fs.entries(aliases) {
        Ok(entries) => entries,
        Err(error) => return unavailable(error, true),
    };
    let mut admitted = BTreeMap::<String, String>::new(); // canonical target -> smallest alias
    let mut visited = 0;
    for entry in entries.by_ref().take(MAX_SCANNED_ENTRIES) {
        visited += 1;
        let name = match entry {
            Ok(name) => name,
            Err(error) => return unavailable(error, false),
        };
        let Some(name) = name.to_str().filter(|name| safe_alias(name)) else {
            continue;
        };
        let alias = aliases.join(name);
        let result = (|| -> io::Result<Option<String>> {
            if !fs.is_symlink(&alias)? {
                return Ok(None);
            }
            let target = fs.canonicalize(&alias)?;
            if !tty_target(&target, devices) || !fs.is_character_device(&target)? {
                return Ok(None);
            }
            Ok(target.to_str().map(str::to_owned))
        })();
        match result {
            Ok(Some(target)) => {
                let path = format!("{ALIASES}/{name}");
                admitted
                    .entry(target)
                    .and_modify(|old| {
                        if path < *old {
                            *old = path.clone();
                        }
                    })
                    .or_insert(path);
            }
            Ok(None) => {}
            Err(error) if error.kind() == io::ErrorKind::NotFound => {} // dangling/raced entry
            Err(error) => return unavailable(error, false),
        }
    }
    // One lookahead detects that the bounded prefix is not a census, without traversing the tail.
    let scan_truncated = match entries.next() {
        Some(Err(error)) => return unavailable(error, false),
        Some(Ok(_)) => true,
        None => false,
    };
    let mut candidates: Vec<_> = admitted
        .into_iter()
        .map(|(target, path)| SerialCandidate {
            path,
            // Injected device roots are only a test seam; the wire contract stays server-local /dev.
            target: format!(
                "{DEVICES}/{}",
                Path::new(&target).file_name().unwrap().to_str().unwrap()
            ),
        })
        .collect();
    candidates.sort_by(|a, b| a.path.cmp(&b.path));
    let truncated = scan_truncated || candidates.len() > MAX_CANDIDATES;
    candidates.truncate(MAX_CANDIDATES);
    SerialCandidates::Available {
        omitted_entries: visited - candidates.len(),
        candidates,
        truncated,
    }
}

#[cfg(test)]
#[path = "serial_candidates_tests.rs"]
mod tests;
