//! Keep the activity database separate from input files before any adapter opens them.

use std::io;
use std::path::{Path, PathBuf};

// Resolve existing parents as well as the final entry: a missing file can still
// have a parent alias (a symlink or `child/..`). Never create a path to compare it.
fn requested_location(directory: &Path, path: &Path) -> io::Result<PathBuf> {
    let requested = directory.join(path);
    let Some(name) = requested.file_name() else {
        return Ok(requested);
    };
    let Some(parent) = requested.parent() else {
        return Ok(requested);
    };
    match std::fs::canonicalize(parent) {
        Ok(parent) => Ok(parent.join(name)),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(requested),
        Err(error) => Err(error),
    }
}

/// Compare requested paths and existing identities without opening or reading files.
/// Missing destinations are compared through their resolved existing parents.
/// Unix also includes hard-link identity for existing files.
/// This is admission-time protection, not a filesystem replacement/race guarantee.
pub fn ensure_separate(history: &Path, inputs: &[&Path]) -> io::Result<()> {
    let directory = std::env::current_dir()?;
    let requested_history = requested_location(&directory, history)?;
    for input in inputs {
        if requested_location(&directory, input)? == requested_history {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "activity history and input must be separate files; not sent",
            ));
        }
    }
    let canonical_history = match std::fs::canonicalize(history) {
        Ok(path) => path,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error),
    };
    for input in inputs {
        let canonical_input = match std::fs::canonicalize(input) {
            Ok(path) => path,
            Err(error) if error.kind() == io::ErrorKind::NotFound => continue,
            Err(error) => return Err(error),
        };
        let same = canonical_history == canonical_input;
        #[cfg(unix)]
        let same = {
            use std::os::unix::fs::MetadataExt;
            let history_metadata = std::fs::metadata(history)?;
            let input_metadata = std::fs::metadata(input)?;
            same || (history_metadata.dev() == input_metadata.dev()
                && history_metadata.ino() == input_metadata.ino())
        };
        if same {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "activity history and input must be separate files; not sent",
            ));
        }
    }
    Ok(())
}
