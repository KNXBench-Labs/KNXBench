//! One place where a client-supplied path string becomes a real
//! filesystem path.
//!
//! Two callers with different needs, deliberately sharing one confinement
//! rule:
//!
//! - `fs_routes.rs` (`/api/fs/*`) only ever speaks in paths *relative* to
//!   `AppState::data_dir` — the browser has no business naming host paths.
//! - `routes.rs` (`/api/project/*`) is dual-use. In the Tauri desktop
//!   build the path comes from a native OS dialog and is an absolute host
//!   path that must keep working (see `apps/knx-web/src/filePicker.ts`);
//!   in the web build it comes from `FsPicker.tsx` or `/api/fs/upload`
//!   and is relative to `data_dir` (`sub/a.knxproj`, `uploads/x.knxproj`).
//!
//! Hence the split below: absolute means "trust it, native dialog"
//! (unconfined by design — the LAN-only, no-auth trust model of
//! KNOWN_LIMITATIONS §22 covers it), relative means "resolve against
//! `data_dir` and stay there". Before this existed, a relative path was
//! passed through verbatim and resolved against the *process working
//! directory*: import 500'd with "No such file or directory", and — worse
//! — save-as happily wrote outside the mounted volume.

use std::path::{Path, PathBuf};

use crate::errors::ApiError;

/// Resolves `relative` against `data_dir` and confirms the result still
/// lives under it — the only thing standing between `/api/fs/list` and a
/// `path=../../etc` escape out of the mounted volume. `canonicalize`
/// requires the path to exist, which also rejects a nonexistent `path`
/// with a clear "does not exist" instead of a confusing filesystem error
/// later.
pub(crate) fn resolve_in_data_dir(data_dir: &Path, relative: &str) -> Result<PathBuf, ApiError> {
    let candidate = data_dir.join(relative.trim_start_matches('/'));
    let canonical_root = canonical_root(data_dir)?;
    let canonical = candidate
        .canonicalize()
        .map_err(|_| ApiError::bad_request("path does not exist"))?;
    if !canonical.starts_with(&canonical_root) {
        return Err(ApiError::bad_request("path escapes the data directory"));
    }
    Ok(canonical)
}

/// Resolves a path a `/api/project/*` route was handed for *reading*
/// (import/open). Absolute paths pass through untouched (desktop, native
/// dialog); relative ones are confined to `data_dir` exactly like
/// `/api/fs/*` is.
pub(crate) fn resolve_project_path(data_dir: &Path, path: &str) -> Result<PathBuf, ApiError> {
    if Path::new(path).is_absolute() {
        return Ok(PathBuf::from(path));
    }
    resolve_in_data_dir(data_dir, path)
}

/// Same as [`resolve_project_path`], for a *write* target (save-as) that
/// by definition does not exist yet. `canonicalize` therefore runs on the
/// parent directory — which must exist and must stay under `data_dir` —
/// and the file name is joined back on afterwards.
pub(crate) fn resolve_new_project_path(data_dir: &Path, path: &str) -> Result<PathBuf, ApiError> {
    if Path::new(path).is_absolute() {
        return Ok(PathBuf::from(path));
    }
    let candidate = data_dir.join(path.trim_start_matches('/'));
    // `file_name()` is `None` for a path ending in `.`/`..` or in a
    // separator — none of which name a file to write.
    let file_name = candidate
        .file_name()
        .ok_or_else(|| ApiError::bad_request("path does not name a file"))?
        .to_owned();
    let parent = candidate
        .parent()
        .ok_or_else(|| ApiError::bad_request("path has no parent directory"))?;

    let canonical_root = canonical_root(data_dir)?;
    let canonical_parent = parent
        .canonicalize()
        .map_err(|_| ApiError::bad_request("directory does not exist"))?;
    if !canonical_parent.starts_with(&canonical_root) {
        return Err(ApiError::bad_request("path escapes the data directory"));
    }
    Ok(canonical_parent.join(file_name))
}

fn canonical_root(data_dir: &Path) -> Result<PathBuf, ApiError> {
    data_dir
        .canonicalize()
        .map_err(|e| ApiError::internal(format!("data dir unreadable: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn data_dir() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("sub")).unwrap();
        std::fs::write(dir.path().join("sub/a.knxproj"), b"fixture").unwrap();
        dir
    }

    #[test]
    fn a_relative_read_path_resolves_under_the_data_dir() {
        let dir = data_dir();
        let resolved = resolve_project_path(dir.path(), "sub/a.knxproj").unwrap();
        assert_eq!(
            resolved,
            dir.path().canonicalize().unwrap().join("sub/a.knxproj")
        );
    }

    #[test]
    fn a_relative_read_path_that_escapes_is_rejected() {
        let dir = data_dir();
        assert!(resolve_project_path(dir.path(), "../../../../etc/passwd").is_err());
    }

    #[test]
    fn an_absolute_read_path_is_passed_through_untouched() {
        let dir = data_dir();
        let absolute = dir.path().join("sub/a.knxproj");
        let resolved = resolve_project_path(dir.path(), &absolute.to_string_lossy()).unwrap();
        assert_eq!(resolved, absolute);
    }

    #[test]
    fn an_absolute_read_path_outside_the_data_dir_is_still_allowed() {
        // The desktop build's native dialog returns arbitrary host paths.
        let dir = data_dir();
        let resolved = resolve_project_path(dir.path(), "/etc/hostname").unwrap();
        assert_eq!(resolved, Path::new("/etc/hostname"));
    }

    #[test]
    fn a_relative_write_target_resolves_even_though_it_does_not_exist_yet() {
        let dir = data_dir();
        let resolved = resolve_new_project_path(dir.path(), "sub/fresh.knxdb").unwrap();
        assert_eq!(
            resolved,
            dir.path().canonicalize().unwrap().join("sub/fresh.knxdb")
        );
    }

    #[test]
    fn a_relative_write_target_that_escapes_is_rejected() {
        let dir = data_dir();
        assert!(resolve_new_project_path(dir.path(), "../escaped.knxdb").is_err());
        assert!(resolve_new_project_path(dir.path(), "sub/../../escaped.knxdb").is_err());
    }

    #[test]
    fn a_relative_write_target_in_a_missing_directory_is_rejected() {
        let dir = data_dir();
        assert!(resolve_new_project_path(dir.path(), "nope/fresh.knxdb").is_err());
    }

    #[test]
    fn a_write_target_naming_no_file_is_rejected() {
        let dir = data_dir();
        assert!(resolve_new_project_path(dir.path(), "sub/..").is_err());
    }

    #[test]
    fn an_absolute_write_target_is_passed_through_untouched() {
        let dir = data_dir();
        let absolute = dir.path().join("elsewhere.knxdb");
        let resolved = resolve_new_project_path(dir.path(), &absolute.to_string_lossy()).unwrap();
        assert_eq!(resolved, absolute);
    }
}
