//! Coordinates native history persistence without protocol or UI dependencies.

use std::fmt;
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
    save_with_sources(path, snapshot, stack, generation, &[])
}

/// Explicit save plus newly acquired file-owned manufacturer payloads.
pub fn save_with_sources(
    path: &Path,
    snapshot: &NativeSnapshot,
    stack: &CommandStack,
    generation: i64,
    sources: &[knx_store::CarriedManufacturerFile],
) -> Result<i64, String> {
    let conn = history::open_editor_store(path, false).map_err(|e| e.to_string())?;
    history::save_editor_with_sources(&conn, snapshot, stack, Some(generation), sources)
        .map_err(|e| e.to_string())
}

/// Why a Save As wrote nothing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SaveAsError {
    /// Something already exists at the destination and the caller did not
    /// ask to replace it. Nothing was written.
    Exists,
    /// Any other failure, as text.
    Failed(String),
}

impl fmt::Display for SaveAsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SaveAsError::Exists => {
                f.write_str("a file with this name already exists; nothing was written")
            }
            SaveAsError::Failed(message) => f.write_str(message),
        }
    }
}

impl std::error::Error for SaveAsError {}

/// [`save_as_with`] that replaces an existing native project, as Save As
/// always did before the overwrite question existed. For callers that have
/// already settled the question themselves.
pub fn save_as(
    path: &Path,
    snapshot: &NativeSnapshot,
    stack: &CommandStack,
) -> Result<i64, String> {
    save_as_with(path, snapshot, stack, true).map_err(|e| e.to_string())
}

/// Admit the entire new project/history in memory before touching a destination.
/// A new file is published only after a complete SQLite backup and fsync, without
/// replacing a file that appeared while the candidate was being constructed.
///
/// An existing destination is refused with [`SaveAsError::Exists`] unless
/// `overwrite` is set. With it, the project is written into that file as
/// its new saved state; the file's previous saved state stays in its
/// project history, and a file that is not a native project is refused by
/// the store instead of being replaced.
pub fn save_as_with(
    path: &Path,
    snapshot: &NativeSnapshot,
    stack: &CommandStack,
    overwrite: bool,
) -> Result<i64, SaveAsError> {
    save_as_with_sources(path, snapshot, stack, overwrite, &[])
}

/// Save As with source payloads in the unpublished candidate / save transaction.
pub fn save_as_with_sources(
    path: &Path,
    snapshot: &NativeSnapshot,
    stack: &CommandStack,
    overwrite: bool,
    sources: &[knx_store::CarriedManufacturerFile],
) -> Result<i64, SaveAsError> {
    let failed = |e: &dyn fmt::Display| SaveAsError::Failed(e.to_string());
    // Any directory entry counts, a dangling symlink included.
    if !overwrite && path.symlink_metadata().is_ok() {
        return Err(SaveAsError::Exists);
    }
    let candidate = knx_store::open_and_migrate_in_memory().map_err(|e| failed(&e))?;
    let generation = history::save_editor_with_sources(&candidate, snapshot, stack, None, sources)
        .map_err(|e| failed(&e))?;
    if overwrite && path.exists() {
        let conn = history::open_editor_store(path, true).map_err(|e| failed(&e))?;
        return history::save_editor_with_sources(&conn, snapshot, stack, None, sources)
            .map_err(|e| failed(&e));
    }
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let file = tempfile::NamedTempFile::new_in(parent).map_err(|e| failed(&e))?;
    candidate
        .backup("main", file.path(), None)
        .map_err(|e| failed(&e))?;
    file.as_file().sync_all().map_err(|e| failed(&e))?;
    // The file may have appeared since the check above; never replace it.
    file.persist_noclobber(path).map_err(|e| {
        if e.error.kind() == std::io::ErrorKind::AlreadyExists {
            SaveAsError::Exists
        } else {
            failed(&e.error)
        }
    })?;
    // Linux-first: synchronize the directory entry as well as database bytes.
    std::fs::File::open(parent)
        .and_then(|dir| dir.sync_all())
        .map_err(|e| failed(&e))?;
    Ok(generation)
}
