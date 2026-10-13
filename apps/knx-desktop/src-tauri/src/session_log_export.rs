//! Atomic native JSON session-log export after an OS save dialog chooses the path.
use std::path::Path;

use crate::native_json_export::write_atomically;

/// `apps/knx-server/src/session_log.rs::MAX_ENTRIES`, as the web export states it.
pub(crate) const SESSION_LOG_CAPACITY: u64 = 16_000;
/// 16000 pretty-printed entries with grouped details need more room than the
/// 16 MiB other native exports allow; still a bound, not unlimited.
pub(crate) const SESSION_LOG_MAX_BYTES: usize = 64 * 1024 * 1024;

/// The UI cannot supply an arbitrary destination; only the native dialog does.
/// Validate before creating any temporary file so invalid output never replaces
/// a previous export. A sibling temporary file makes the final rename atomic.
pub(crate) fn write_session_log(path: &Path, contents: &str) -> Result<(), String> {
    if contents.len() > SESSION_LOG_MAX_BYTES {
        return Err("Session log export exceeds the 64 MiB desktop limit".into());
    }
    let document: serde_json::Value = serde_json::from_str(contents)
        .map_err(|error| format!("Invalid session log JSON: {error}"))?;
    if document.get("format").and_then(serde_json::Value::as_str) != Some("knxbench-session-log")
        || document.get("version").and_then(serde_json::Value::as_u64) != Some(1)
        || document.get("capacity").and_then(serde_json::Value::as_u64)
            != Some(SESSION_LOG_CAPACITY)
        || !document
            .get("entries")
            .and_then(serde_json::Value::as_array)
            .is_some_and(|entries| entries.len() as u64 <= SESSION_LOG_CAPACITY)
    {
        return Err("Unsupported session log export document".into());
    }
    write_atomically(path, contents, "session log")
}
