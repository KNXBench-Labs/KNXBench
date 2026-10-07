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
use crate::tls_cert::TLS_DIR_NAME;

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
    refuse_reserved(&canonical_root, &canonical)?;
    Ok(canonical)
}

/// Resolves a path a `/api/project/*` route was handed for *reading*
/// (import/open). Absolute paths pass through untouched (desktop, native
/// dialog); relative ones are confined to `data_dir` exactly like
/// `/api/fs/*` is.
pub(crate) fn resolve_project_path(data_dir: &Path, path: &str) -> Result<PathBuf, ApiError> {
    if Path::new(path).is_absolute() {
        // Unconfined by design, except for the one directory no route may
        // touch. A path that does not exist cannot be inside it.
        if let (Ok(root), Ok(canonical)) = (data_dir.canonicalize(), Path::new(path).canonicalize())
        {
            refuse_reserved(&root, &canonical)?;
        }
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
        let target = Path::new(path);
        if let (Ok(root), Some(parent), Some(name)) =
            (data_dir.canonicalize(), target.parent(), target.file_name())
        {
            if let Ok(parent) = parent.canonicalize() {
                refuse_reserved(&root, &parent.join(name))?;
            }
        }
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
    let target = canonical_parent.join(file_name);
    refuse_reserved(&canonical_root, &target)?;
    Ok(target)
}

/// Refuses any path at or under `<data dir>/.knxbench-tls`, where the
/// server keeps its generated TLS key (ADR-0088). Without this, `save-as`
/// could overwrite the key with a project file and `/api/fs/list` would
/// walk into the directory; neither leaks the key, but the first turns the
/// next restart into a new certificate the operator never asked for.
/// Both the literal and the resolved form of the directory are checked,
/// so a symlink in its place does not open a side door.
fn refuse_reserved(canonical_root: &Path, canonical: &Path) -> Result<(), ApiError> {
    let reserved = canonical_root.join(TLS_DIR_NAME);
    let resolved = reserved.canonicalize().unwrap_or_else(|_| reserved.clone());
    if canonical.starts_with(&reserved) || canonical.starts_with(&resolved) {
        return Err(ApiError::bad_request(
            "path is reserved for the server's TLS certificate and key",
        ));
    }
    Ok(())
}

/// Whether `name`, listed directly in `dir`, is the reserved TLS directory
/// — for `/api/fs/list` to leave it out of the data directory's listing.
pub(crate) fn is_reserved_entry(data_dir: &Path, dir: &Path, name: &std::ffi::OsStr) -> bool {
    name == TLS_DIR_NAME && canonical_root(data_dir).is_ok_and(|root| dir == root)
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

    fn data_dir_with_tls() -> tempfile::TempDir {
        let dir = data_dir();
        std::fs::create_dir(dir.path().join(TLS_DIR_NAME)).unwrap();
        std::fs::write(dir.path().join(TLS_DIR_NAME).join("key.pem"), b"secret").unwrap();
        dir
    }

    #[test]
    fn the_tls_directory_cannot_be_listed_read_or_written_relatively() {
        let dir = data_dir_with_tls();
        for path in [
            ".knxbench-tls",
            ".knxbench-tls/key.pem",
            "sub/../.knxbench-tls",
        ] {
            let error = resolve_in_data_dir(dir.path(), path).unwrap_err();
            assert!(
                format!("{error:?}").contains("reserved"),
                "{path}: {error:?}"
            );
            assert!(resolve_project_path(dir.path(), path).is_err(), "{path}");
        }
        for path in [
            ".knxbench-tls/key.pem",
            ".knxbench-tls/new.knxdb",
            ".knxbench-tls",
        ] {
            assert!(
                resolve_new_project_path(dir.path(), path).is_err(),
                "{path}"
            );
        }
    }

    #[test]
    fn the_tls_directory_cannot_be_reached_by_absolute_path_either() {
        let dir = data_dir_with_tls();
        let key = dir.path().join(TLS_DIR_NAME).join("key.pem");
        assert!(resolve_project_path(dir.path(), &key.to_string_lossy()).is_err());
        assert!(resolve_new_project_path(dir.path(), &key.to_string_lossy()).is_err());
        let fresh = dir.path().join(TLS_DIR_NAME).join("fresh.knxdb");
        assert!(resolve_new_project_path(dir.path(), &fresh.to_string_lossy()).is_err());
    }

    #[cfg(unix)]
    #[test]
    fn a_symlink_into_the_tls_directory_is_no_side_door() {
        let dir = data_dir_with_tls();
        std::os::unix::fs::symlink(dir.path().join(TLS_DIR_NAME), dir.path().join("sub/door"))
            .unwrap();
        assert!(resolve_in_data_dir(dir.path(), "sub/door/key.pem").is_err());
        assert!(resolve_new_project_path(dir.path(), "sub/door/key.pem").is_err());
    }

    #[test]
    fn neighbours_of_the_tls_directory_are_unaffected() {
        let dir = data_dir_with_tls();
        std::fs::write(dir.path().join(".knxbench-tls-notes.knxdb"), b"x").unwrap();
        assert!(resolve_in_data_dir(dir.path(), ".knxbench-tls-notes.knxdb").is_ok());
        assert!(resolve_new_project_path(dir.path(), "sub/.knxbench-tls").is_ok());
    }

    #[test]
    fn only_the_root_listing_hides_the_tls_directory() {
        let dir = data_dir_with_tls();
        let root = dir.path().canonicalize().unwrap();
        let name = std::ffi::OsStr::new(TLS_DIR_NAME);
        assert!(is_reserved_entry(dir.path(), &root, name));
        assert!(!is_reserved_entry(dir.path(), &root.join("sub"), name));
        assert!(!is_reserved_entry(
            dir.path(),
            &root,
            std::ffi::OsStr::new("sub")
        ));
    }
}
