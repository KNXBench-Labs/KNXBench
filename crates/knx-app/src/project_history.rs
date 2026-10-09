//! Coordinates native history persistence without protocol or UI dependencies.

use std::path::Path;

use knx_core::CommandStack;
use knx_store::project_history::{self as history, NativeSnapshot};

/// An acknowledged edit of an already saved native project. The saved root
/// stays untouched; the recoverable working journal receives the candidate.
pub fn persist_working(
    path: &Path,
    snapshot: &NativeSnapshot,
    stack: &CommandStack,
    generation: i64,
) -> Result<i64, String> {
    let conn = history::open_editor_store(path, false).map_err(|e| e.to_string())?;
    history::save_editor(&conn, snapshot, stack, Some(generation), false).map_err(|e| e.to_string())
}

pub fn save(
    path: &Path,
    snapshot: &NativeSnapshot,
    stack: &CommandStack,
    generation: i64,
) -> Result<i64, String> {
    let conn = history::open_editor_store(path, false).map_err(|e| e.to_string())?;
    history::save_editor(&conn, snapshot, stack, Some(generation), true).map_err(|e| e.to_string())
}

/// Admit the entire new project/history in memory before touching a destination.
/// A new file is published only after a complete SQLite backup and fsync, without
/// replacing a file that appeared while the candidate was being constructed.
pub fn save_as(
    path: &Path,
    snapshot: &NativeSnapshot,
    stack: &CommandStack,
) -> Result<i64, String> {
    let candidate = knx_store::open_and_migrate_in_memory().map_err(|e| e.to_string())?;
    let generation =
        history::save_editor(&candidate, snapshot, stack, None, true).map_err(|e| e.to_string())?;
    if path.exists() {
        let conn = history::open_editor_store(path, true).map_err(|e| e.to_string())?;
        return history::save_editor(&conn, snapshot, stack, None, true).map_err(|e| e.to_string());
    }
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let file = tempfile::NamedTempFile::new_in(parent).map_err(|e| e.to_string())?;
    candidate
        .backup("main", file.path(), None)
        .map_err(|e| e.to_string())?;
    file.as_file().sync_all().map_err(|e| e.to_string())?;
    file.persist_noclobber(path)
        .map_err(|e| e.error.to_string())?;
    // Linux-first: synchronize the directory entry as well as database bytes.
    std::fs::File::open(parent)
        .and_then(|dir| dir.sync_all())
        .map_err(|e| e.to_string())?;
    Ok(generation)
}
