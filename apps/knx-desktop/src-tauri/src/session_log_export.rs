//! Atomic native JSON session-log export after an OS save dialog chooses the path.
use std::io::Write;
use std::path::Path;

const MAX_EXPORT_BYTES: usize = 16 * 1024 * 1024;

/// The UI cannot supply an arbitrary destination; only the native dialog does.
/// Validate before creating any temporary file so invalid output never replaces
/// a previous export. A sibling temporary file makes the final rename atomic.
pub(crate) fn write_session_log(path: &Path, contents: &str) -> Result<(), String> {
    if contents.len() > MAX_EXPORT_BYTES {
        return Err("Session log export exceeds the 16 MiB desktop limit".into());
    }
    let document: serde_json::Value = serde_json::from_str(contents)
        .map_err(|error| format!("Invalid session log JSON: {error}"))?;
    if document.get("format").and_then(serde_json::Value::as_str) != Some("knxbench-session-log")
        || document.get("version").and_then(serde_json::Value::as_u64) != Some(1)
        || document.get("capacity").and_then(serde_json::Value::as_u64) != Some(1000)
        || !document
            .get("entries")
            .and_then(serde_json::Value::as_array)
            .is_some_and(|entries| entries.len() <= 1000)
    {
        return Err("Unsupported session log export document".into());
    }
    let directory = path
        .parent()
        .ok_or("Session log export has no parent directory")?;
    let mut temporary = tempfile::NamedTempFile::new_in(directory)
        .map_err(|error| format!("Cannot create session log file: {error}"))?;
    temporary
        .write_all(contents.as_bytes())
        .and_then(|_| temporary.flush())
        .and_then(|_| temporary.as_file().sync_all())
        .map_err(|error| format!("Cannot write session log: {error}"))?;
    temporary
        .persist(path)
        .map_err(|error| format!("Cannot finish session log export: {error}"))?;
    Ok(())
}
