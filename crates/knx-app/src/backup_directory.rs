//! Directory-entry synchronization shared by the pre-write backup writers.
//!
//! A verified, synced file still needs its containing directory entry synced.
//! Newly created ancestors need the same treatment. Keep the supplied path's
//! symlink and `..` components: recovery may need those entries to resolve it.
//! Also sync the resolved target's ancestors when it follows a different path.
//! This requests OS synchronization, not a power-loss or path-confinement proof.

use std::collections::HashSet;
use std::io;
use std::path::Path;

pub(crate) fn sync_chain(
    directory: &Path,
    mut sync: impl FnMut(&Path) -> io::Result<()>,
) -> io::Result<()> {
    if directory.as_os_str().is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "backup directory must not be empty",
        ));
    }
    let absolute = if directory.is_absolute() {
        directory.to_path_buf()
    } else {
        std::env::current_dir()?.join(directory)
    };
    let canonical = absolute.canonicalize()?;
    let mut seen = HashSet::new();
    for ancestor in absolute.ancestors().chain(canonical.ancestors()) {
        if seen.insert(ancestor.to_path_buf()) {
            sync(ancestor)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn an_empty_directory_cannot_be_an_implicit_cwd_receipt() {
        let mut invoked = false;
        let result = sync_chain(Path::new(""), |_| {
            invoked = true;
            Ok(())
        });
        assert!(
            result.is_err(),
            "empty backup directory must remain refused"
        );
        assert!(!invoked);
    }

    #[test]
    fn relative_paths_are_anchored_without_an_empty_ancestor() {
        let mut seen = Vec::new();
        sync_chain(Path::new("."), |directory| {
            assert!(directory.is_absolute());
            seen.push(directory.to_path_buf());
            Ok(())
        })
        .unwrap();
        let absolute = std::env::current_dir().unwrap().join(".");
        assert_eq!(seen.first().unwrap(), &absolute);
        assert_eq!(seen.last().unwrap(), absolute.ancestors().last().unwrap());
    }

    #[test]
    fn parent_failure_stops_the_chain_and_preserves_the_error() {
        let temp = tempfile::tempdir().unwrap();
        let dir = temp.path().join("one/two/backups");
        fs::create_dir_all(&dir).unwrap();
        let mut seen = Vec::new();
        let error = sync_chain(&dir, |directory| {
            seen.push(directory.to_path_buf());
            if directory == dir.parent().unwrap() {
                return Err(io::Error::new(io::ErrorKind::PermissionDenied, "injected"));
            }
            fs::File::open(directory)?.sync_all()
        })
        .unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::PermissionDenied);
        assert_eq!(seen, [dir.clone(), dir.parent().unwrap().to_path_buf()]);
    }

    #[cfg(unix)]
    #[test]
    fn a_symlink_path_keeps_the_directory_containing_its_alias() {
        let temp = tempfile::tempdir().unwrap();
        let target = tempfile::tempdir().unwrap();
        let target_path = target.path().join("extra/nested");
        fs::create_dir_all(&target_path).unwrap();
        let alias = temp.path().join("alias");
        std::os::unix::fs::symlink(&target_path, &alias).unwrap();
        let dir = alias.join("new/backups");
        fs::create_dir_all(&dir).unwrap();
        let mut seen = Vec::new();
        sync_chain(&dir, |directory| {
            seen.push(directory.to_path_buf());
            fs::File::open(directory)?.sync_all()
        })
        .unwrap();
        let expected: Vec<_> = dir.ancestors().map(Path::to_path_buf).collect();
        assert_eq!(&seen[..expected.len()], &expected);
        assert!(seen.contains(&temp.path().to_path_buf()));
        assert!(
            seen.contains(&target_path.parent().unwrap().to_path_buf()),
            "the alias target needs its own parent entries persisted"
        );
    }

    #[test]
    fn dot_dot_keeps_components_needed_to_resolve_the_saved_path() {
        let temp = tempfile::tempdir().unwrap();
        let dir = temp.path().join("hop/../new/backups");
        fs::create_dir_all(&dir).unwrap();
        let mut seen = Vec::new();
        sync_chain(&dir, |directory| {
            seen.push(directory.to_path_buf());
            fs::File::open(directory)?.sync_all()
        })
        .unwrap();
        let expected: Vec<_> = dir.ancestors().map(Path::to_path_buf).collect();
        assert_eq!(&seen[..expected.len()], &expected);
        assert!(seen.contains(&temp.path().join("hop")));
    }
}
