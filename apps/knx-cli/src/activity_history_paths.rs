//! Keep the activity database separate from input files before any adapter opens them.

use std::io;
use std::path::Path;

/// Compare requested paths and existing identities without opening or reading files.
/// Identical requested destinations are refused even when they do not exist yet.
/// Unix includes hard-link identity; other platforms compare canonical paths only.
/// This is admission-time protection, not a filesystem replacement/race guarantee.
pub fn ensure_separate(history: &Path, inputs: &[&Path]) -> io::Result<()> {
    let directory = std::env::current_dir()?;
    let requested_history = directory.join(history);
    if inputs
        .iter()
        .any(|input| directory.join(input) == requested_history)
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "activity history and input must be separate files; not sent",
        ));
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
