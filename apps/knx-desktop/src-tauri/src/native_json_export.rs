//! Validated native JSON exports share one atomic file-write primitive.
use std::io::Write;
use std::path::Path;

pub(crate) const MAX_EXPORT_BYTES: usize = 16 * 1024 * 1024;

/// Callers validate format and size before this touches a destination chosen
/// by the OS dialog. A sibling temporary file keeps the previous file intact
/// until the full JSON has been flushed and synced.
pub(crate) fn write_atomically(path: &Path, contents: &str, kind: &str) -> Result<(), String> {
    let directory = path
        .parent()
        .ok_or_else(|| format!("{kind} export has no parent directory"))?;
    let mut temporary = tempfile::NamedTempFile::new_in(directory)
        .map_err(|error| format!("Cannot create {kind} file: {error}"))?;
    temporary
        .write_all(contents.as_bytes())
        .and_then(|_| temporary.flush())
        .and_then(|_| temporary.as_file().sync_all())
        .map_err(|error| format!("Cannot write {kind}: {error}"))?;
    temporary
        .persist(path)
        .map_err(|error| format!("Cannot finish {kind} export: {error}"))?;
    Ok(())
}
