//! Descriptor-pinned directory acquisition and verified pre-write JSON persistence.
//!
//! Traversal preserves static aliases and `..`, but is not a namespace snapshot or confinement.
//! Receipts are absolute, retaining caller spelling after anchoring relative input to the initial
//! CWD. Final identity checks are bounded observations, not protection against subsequent changes.

use std::fs::File;
use std::io;
#[cfg(unix)]
use std::io::{Read, Seek, Write};
use std::path::{Component, Path, PathBuf};

// The property-validation recorder still uses its legacy path-based protocol. It is outside
// this JSON-writer contract; keep its private API unchanged rather than claim it is pinned.
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
    let mut seen = std::collections::HashSet::new();
    for ancestor in absolute.ancestors().chain(canonical.ancestors()) {
        if seen.insert(ancestor.to_path_buf()) {
            sync(ancestor)?;
        }
    }
    Ok(())
}

// Unix drives these stages; unsupported platforms deliberately never reach one.
#[cfg_attr(not(unix), allow(dead_code))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Boundary {
    CwdOpened,
    CwdSpelled,
    Acquired,
    Written,
    FileSync,
    Seek,
    Read,
    ReadBack,
    DirectorySync,
    BeforeReceipt,
}

// Per-call observations/faults exist only in test builds. They cannot replace any real operation.
#[cfg(test)]
type Callback<'a> = dyn FnMut(Boundary, &File, &Path) -> io::Result<()> + 'a;

#[derive(Default)]
pub(crate) struct Hooks<'a> {
    #[cfg(test)]
    callback: Option<&'a mut Callback<'a>>,
    #[cfg(not(test))]
    marker: std::marker::PhantomData<&'a ()>,
}

impl<'a> Hooks<'a> {
    #[cfg(test)]
    pub(crate) fn new(callback: &'a mut Callback<'a>) -> Self {
        Self {
            callback: Some(callback),
        }
    }

    #[cfg_attr(not(unix), allow(dead_code))]
    fn at(&mut self, boundary: Boundary, file: &File, path: &Path) -> io::Result<()> {
        #[cfg(test)]
        if let Some(callback) = &mut self.callback {
            callback(boundary, file, path)?;
        }
        let _ = (boundary, file, path);
        Ok(())
    }
}

/// Both writers share the actual file lifecycle; their semantic parsers remain independent.
pub(crate) fn write_verified<E: From<io::Error>>(
    directory: &Path,
    basename: &str,
    bytes: &[u8],
    verify: impl FnOnce(&str, &Path) -> Result<(), E>,
    hooks: &mut Hooks<'_>,
) -> Result<PathBuf, E> {
    // Refuse even before opening a directory; an empty name must never imply CWD.
    if directory.as_os_str().is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "backup directory must not be empty",
        )
        .into());
    }
    if !matches!(
        Path::new(basename)
            .components()
            .collect::<Vec<_>>()
            .as_slice(),
        [Component::Normal(_)]
    ) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "backup name must be a basename",
        )
        .into());
    }
    #[cfg(not(unix))]
    {
        let _ = (bytes, verify, hooks);
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "descriptor-pinned backups require Unix",
        )
        .into())
    }
    #[cfg(unix)]
    {
        let directory = Directory::acquire(directory, hooks)?;
        let path = directory.path().join(basename);
        hooks.at(Boundary::Acquired, directory.file(), &path)?;
        let mut file = directory.create(basename)?;
        file.write_all(bytes)?;
        hooks.at(Boundary::Written, &file, &path)?;
        hooks.at(Boundary::FileSync, &file, &path)?;
        file.sync_all()?;
        hooks.at(Boundary::Seek, &file, &path)?;
        file.rewind()?;
        hooks.at(Boundary::Read, &file, &path)?;
        let mut text = String::new();
        file.read_to_string(&mut text)?;
        verify(&text, &path)?;
        hooks.at(Boundary::ReadBack, &file, &path)?;
        directory.sync(hooks)?;
        hooks.at(Boundary::BeforeReceipt, &file, &path)?;
        directory.verify(basename, &file)?;
        // Failed attempts deliberately leave their private file: path-based cleanup could hit a substitute.
        Ok(path)
    }
}

#[cfg(unix)]
use unix::Directory;

#[cfg(unix)]
mod unix {
    use super::*;
    use std::collections::{HashMap, HashSet, VecDeque};
    use std::ffi::OsString;
    use std::os::unix::ffi::OsStringExt;
    use std::os::unix::fs::MetadataExt;

    use rustix::fs::{mkdirat, open, openat, readlinkat, statat, AtFlags, FileType, Mode, OFlags};
    use rustix::io::Errno;

    type Identity = (u64, u64);

    fn identity(file: &File) -> io::Result<Identity> {
        let metadata = file.metadata()?;
        Ok((metadata.dev(), metadata.ino()))
    }

    fn changed() -> io::Error {
        io::Error::other("backup pathname no longer identifies the retained objects")
    }

    fn flags() -> OFlags {
        OFlags::RDONLY | OFlags::DIRECTORY | OFlags::CLOEXEC | OFlags::NOFOLLOW
    }

    fn open_directory(parent: &File, name: &Path) -> io::Result<File> {
        Ok(File::from(openat(parent, name, flags(), Mode::empty())?))
    }

    fn components(path: &Path) -> VecDeque<OsString> {
        path.components()
            .map(|part| part.as_os_str().to_owned())
            .collect()
    }

    /// Explicit symlink expansion retains alias-containing directories AND each resolved ancestor.
    /// NOFOLLOW on each open refuses a newly substituted symlink at that boundary.
    fn resolve(
        start: &File,
        root: &File,
        path: &Path,
        create: bool,
        visited: &mut Vec<File>,
    ) -> io::Result<File> {
        let mut current = start.try_clone()?;
        visited.push(current.try_clone()?);
        let mut pending = components(path);
        let mut links = 0;
        while let Some(part) = pending.pop_front() {
            let part = Path::new(&part);
            if part == Path::new("/") {
                current = root.try_clone()?;
            } else {
                let stat = match statat(&current, part, AtFlags::SYMLINK_NOFOLLOW) {
                    Ok(stat) => stat,
                    Err(Errno::NOENT) if create => {
                        match mkdirat(&current, part, Mode::from_raw_mode(0o777)) {
                            Ok(()) | Err(Errno::EXIST) => {}
                            Err(error) => return Err(error.into()),
                        }
                        statat(&current, part, AtFlags::SYMLINK_NOFOLLOW)?
                    }
                    Err(error) => return Err(error.into()),
                };
                if FileType::from_raw_mode(stat.st_mode) == FileType::Symlink {
                    links += 1;
                    if links > 40 {
                        return Err(io::Error::other("too many backup directory symlinks"));
                    }
                    let target = readlinkat(&current, part, Vec::new())?;
                    let target = PathBuf::from(OsString::from_vec(target.into_bytes()));
                    for component in components(&target).into_iter().rev() {
                        pending.push_front(component);
                    }
                    continue;
                }
                current = open_directory(&current, part)?;
            }
            visited.push(current.try_clone()?);
        }
        Ok(current)
    }

    struct Held {
        file: File,
        id: Identity,
        parent: Option<Identity>,
    }

    pub(super) struct Directory {
        root: File,
        cwd: Option<(File, PathBuf)>,
        original: PathBuf,
        receipt: PathBuf,
        target: File,
        chain: Vec<Held>,
    }

    impl Directory {
        pub(super) fn acquire(path: &Path, hooks: &mut Hooks<'_>) -> io::Result<Self> {
            let root = File::from(open("/", flags(), Mode::empty())?);
            let cwd = if path.is_relative() {
                let cwd = File::from(open(".", flags(), Mode::empty())?);
                hooks.at(Boundary::CwdOpened, &cwd, Path::new("."))?;
                let spelling = std::env::current_dir()?;
                hooks.at(Boundary::CwdSpelled, &cwd, &spelling)?;
                let observed = resolve(&root, &root, &spelling, false, &mut Vec::new())?;
                if identity(&observed)? != identity(&cwd)? {
                    return Err(changed());
                }
                Some((cwd, spelling))
            } else {
                None
            };
            let receipt = cwd
                .as_ref()
                .map_or_else(|| path.to_owned(), |(_, spelling)| spelling.join(path));
            let start = cwd.as_ref().map_or(&root, |(file, _)| file);
            let mut visited = Vec::new();
            let target = resolve(start, &root, path, true, &mut visited)?;
            let mut chain = Vec::new();
            let mut seen = HashSet::new();
            // Reverse traversal prefers the final containing directory over unrelated alias branches.
            for file in visited.into_iter().rev() {
                let mut file = file;
                loop {
                    let id = identity(&file)?;
                    if !seen.insert(id) {
                        break;
                    }
                    let parent = open_directory(&file, Path::new(".."))?;
                    let parent_id = identity(&parent)?;
                    chain.push(Held {
                        file,
                        id,
                        parent: (id != parent_id).then_some(parent_id),
                    });
                    if id == parent_id {
                        break;
                    }
                    file = parent;
                }
            }
            Ok(Self {
                root,
                cwd,
                original: path.to_owned(),
                receipt,
                target,
                chain,
            })
        }

        pub(super) fn path(&self) -> &Path {
            &self.receipt
        }
        pub(super) fn file(&self) -> &File {
            &self.target
        }

        pub(super) fn create(&self, basename: &str) -> io::Result<File> {
            Ok(File::from(openat(
                &self.target,
                basename,
                OFlags::CREATE | OFlags::EXCL | OFlags::RDWR | OFlags::CLOEXEC | OFlags::NOFOLLOW,
                Mode::from_raw_mode(0o600),
            )?))
        }

        pub(super) fn sync(&self, hooks: &mut Hooks<'_>) -> io::Result<()> {
            // Topological child-first order across both alias and target branches; inode/dev dedup.
            let mut children = HashMap::<Identity, usize>::new();
            for held in &self.chain {
                if let Some(parent) = held.parent {
                    *children.entry(parent).or_default() += 1;
                }
            }
            let mut done = HashSet::new();
            while done.len() < self.chain.len() {
                let held = self
                    .chain
                    .iter()
                    .find(|held| {
                        !done.contains(&held.id)
                            && children.get(&held.id).copied().unwrap_or(0) == 0
                    })
                    .ok_or_else(|| {
                        io::Error::other("backup ancestor identities contain a cycle")
                    })?;
                hooks.at(Boundary::DirectorySync, &held.file, &self.receipt)?;
                held.file.sync_all()?;
                done.insert(held.id);
                if let Some(parent) = held.parent {
                    *children.get_mut(&parent).expect("held parent has a child") -= 1;
                }
            }
            Ok(())
        }

        pub(super) fn verify(&self, basename: &str, file: &File) -> io::Result<()> {
            let start = if let Some((cwd, spelling)) = &self.cwd {
                let observed = resolve(&self.root, &self.root, spelling, false, &mut Vec::new())?;
                if identity(&observed)? != identity(cwd)? {
                    return Err(changed());
                }
                cwd
            } else {
                &self.root
            };
            let original = resolve(start, &self.root, &self.original, false, &mut Vec::new())?;
            let receipt = resolve(
                &self.root,
                &self.root,
                &self.receipt,
                false,
                &mut Vec::new(),
            )?;
            for directory in [&original, &receipt] {
                if identity(directory)? != identity(&self.target)? {
                    return Err(changed());
                }
                let stat = statat(directory, basename, AtFlags::SYMLINK_NOFOLLOW)?;
                let held = rustix::fs::fstat(file)?;
                if FileType::from_raw_mode(stat.st_mode) != FileType::RegularFile
                    || stat.st_dev != held.st_dev
                    || stat.st_ino != held.st_ino
                {
                    return Err(changed());
                }
            }
            // A receipt must also fit ordinary pathname lookup used by public backup readers.
            let named = File::from(open(
                self.receipt.join(basename),
                OFlags::RDONLY | OFlags::CLOEXEC | OFlags::NOFOLLOW | OFlags::NONBLOCK,
                Mode::empty(),
            )?);
            if identity(&named)? != identity(file)? {
                return Err(changed());
            }
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_empty_directory_cannot_be_an_implicit_cwd_receipt() {
        let mut called = false;
        let mut observe = |_, _: &File, _: &Path| {
            called = true;
            Ok(())
        };
        let result = write_verified::<io::Error>(
            Path::new(""),
            "backup.json",
            b"{}",
            |_, _| panic!("empty directory reached readback"),
            &mut Hooks::new(&mut observe),
        );
        assert_eq!(result.unwrap_err().kind(), io::ErrorKind::InvalidInput);
        assert!(!called);
    }

    #[cfg(unix)]
    #[test]
    fn relative_paths_are_anchored_without_an_empty_ancestor() {
        let mut synced = Vec::new();
        let mut observe = |stage, file: &File, path: &Path| {
            if stage == Boundary::DirectorySync {
                assert!(path.is_absolute());
                synced.push(test_support::id(file));
            }
            Ok(())
        };
        let mut hooks = Hooks::new(&mut observe);
        let directory = Directory::acquire(Path::new("."), &mut hooks).unwrap();
        let cwd = std::env::current_dir().unwrap();
        assert_eq!(directory.path(), cwd.join("."));
        directory.sync(&mut hooks).unwrap();
        let expected: Vec<_> = cwd
            .ancestors()
            .map(|path| test_support::id(&File::open(path).unwrap()))
            .collect();
        assert_eq!(synced, expected);
    }

    #[cfg(unix)]
    #[test]
    fn parent_failure_stops_the_chain_and_preserves_the_error() {
        let temp = tempfile::tempdir().unwrap();
        let dir = temp.path().join("one/two/backups");
        let mut synced = Vec::new();
        let mut observe = |stage, file: &File, _: &Path| {
            if stage == Boundary::DirectorySync {
                synced.push(test_support::id(file));
                if synced.len() == 2 {
                    return Err(io::Error::new(io::ErrorKind::PermissionDenied, "injected"));
                }
            }
            Ok(())
        };
        let error = write_verified::<io::Error>(
            &dir,
            "backup.json",
            b"{}",
            |text, _| {
                assert_eq!(text, "{}");
                Ok(())
            },
            &mut Hooks::new(&mut observe),
        )
        .unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::PermissionDenied);
        assert_eq!(
            synced,
            [
                test_support::id(&File::open(&dir).unwrap()),
                test_support::id(&File::open(dir.parent().unwrap()).unwrap())
            ]
        );
    }

    #[cfg(not(unix))]
    #[test]
    fn unsupported_platform_never_acquires_or_mutates_directories() {
        let temp = tempfile::tempdir().unwrap();
        let dir = temp.path().join("must-not-exist");
        let mut called = false;
        let mut observe = |_, _: &File, _: &Path| {
            called = true;
            Ok(())
        };
        let error = write_verified::<io::Error>(
            &dir,
            "backup.json",
            b"{}",
            |_, _| panic!("unsupported readback"),
            &mut Hooks::new(&mut observe),
        )
        .unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::Unsupported);
        assert!(!dir.exists() && !called);
    }
}

#[cfg(all(test, unix))]
pub(crate) mod test_support {
    use super::*;
    use std::fs;
    use std::os::fd::AsRawFd;
    use std::os::unix::fs::{symlink, MetadataExt, PermissionsExt};

    pub(crate) fn id(file: &File) -> (u64, u64) {
        let metadata = file.metadata().unwrap();
        (metadata.dev(), metadata.ino())
    }

    fn path_id(path: &Path) -> (u64, u64) {
        id(&File::open(path).unwrap())
    }

    /// Exercise the real writer at each finite namespace boundary, without scheduler races.
    pub(crate) fn namespace_changes(
        mut write: impl FnMut(&Path, &mut Hooks<'_>) -> io::Result<PathBuf>,
        expected: &str,
    ) {
        for boundary in [
            Boundary::Acquired,
            Boundary::Written,
            Boundary::ReadBack,
            Boundary::BeforeReceipt,
        ] {
            for alias in [false, true] {
                let temp = tempfile::tempdir().unwrap();
                let original = temp.path().join("original");
                let alternative = temp.path().join("alternative");
                let parked = temp.path().join("parked");
                fs::create_dir_all(original.join("backups")).unwrap();
                fs::create_dir_all(alternative.join("backups")).unwrap();
                fs::write(alternative.join("control"), b"unchanged").unwrap();
                let alias_path = temp.path().join("alias");
                let directory = if alias {
                    symlink(&original, &alias_path).unwrap();
                    alias_path.join("backups")
                } else {
                    original.join("backups")
                };
                let held_directory = path_id(&directory);
                let held_parent = path_id(&original);
                let mut replacement_id = None;
                let mut replaced = false;
                let mut read_back = false;
                let mut synced = Vec::new();
                let mut basename = OsString::new();
                let mut callback = |stage, file: &File, path: &Path| {
                    if stage == Boundary::ReadBack {
                        read_back = true;
                    }
                    if stage == Boundary::DirectorySync {
                        synced.push(id(file));
                    }
                    if stage == boundary {
                        assert!(!replaced);
                        basename = path.file_name().unwrap().to_owned();
                        if alias {
                            fs::remove_file(&alias_path)?;
                            symlink(&alternative, &alias_path)?;
                        } else {
                            fs::rename(&original, &parked)?;
                            fs::create_dir_all(original.join("backups"))?;
                        }
                        let replacement = if alias {
                            alternative.join("backups")
                        } else {
                            original.join("backups")
                        };
                        replacement_id = Some(path_id(&replacement));
                        fs::write(
                            replacement.join(&basename),
                            b"replacement must not be read, written or removed",
                        )?;
                        replaced = true;
                    }
                    Ok(())
                };
                let result = write(&directory, &mut Hooks::new(&mut callback));
                assert!(
                    result.is_err(),
                    "namespace substitution must refuse {boundary:?}, alias={alias}"
                );
                assert!(
                    replaced && read_back,
                    "same-descriptor semantic readback must finish"
                );
                let old = if alias {
                    original.join("backups")
                } else {
                    parked.join("backups")
                };
                assert_eq!(fs::read_to_string(old.join(&basename)).unwrap(), expected);
                let replacement = if alias {
                    alternative.join("backups")
                } else {
                    original.join("backups")
                };
                assert_eq!(
                    fs::read(replacement.join(&basename)).unwrap(),
                    b"replacement must not be read, written or removed"
                );
                assert_eq!(fs::read_dir(&replacement).unwrap().count(), 1);
                assert_eq!(fs::read(alternative.join("control")).unwrap(), b"unchanged");
                assert_eq!(synced.first(), Some(&held_directory));
                assert!(synced.contains(&held_parent));
                assert!(
                    !synced.contains(&replacement_id.unwrap()),
                    "must not sync replacement directories"
                );
            }
        }
    }

    pub(crate) fn same_file_and_final_identity(
        mut write: impl FnMut(&Path, &mut Hooks<'_>) -> io::Result<PathBuf>,
        expected: &str,
    ) {
        for boundary in [
            Boundary::Written,
            Boundary::ReadBack,
            Boundary::BeforeReceipt,
        ] {
            for replacement in [false, true] {
                let temp = tempfile::tempdir().unwrap();
                let mut descriptor = None;
                let mut inode = None;
                let mut read_back = false;
                let mut observed_path = PathBuf::new();
                let mut callback = |stage, file: &File, path: &Path| {
                    if matches!(
                        stage,
                        Boundary::Written
                            | Boundary::FileSync
                            | Boundary::Seek
                            | Boundary::Read
                            | Boundary::ReadBack
                            | Boundary::BeforeReceipt
                    ) {
                        assert_eq!(
                            *descriptor.get_or_insert(file.as_raw_fd()),
                            file.as_raw_fd()
                        );
                        assert_eq!(*inode.get_or_insert(id(file)), id(file));
                    }
                    if stage == Boundary::ReadBack {
                        read_back = true;
                    }
                    if stage == boundary {
                        observed_path = path.to_owned();
                        fs::rename(path, temp.path().join("retained"))?;
                        if replacement {
                            fs::write(path, b"not JSON: a reopened-path reader must fail")?;
                        }
                    }
                    Ok(())
                };
                assert!(write(temp.path(), &mut Hooks::new(&mut callback)).is_err());
                assert!(read_back, "readback must use the still-open original inode");
                assert_eq!(
                    fs::read_to_string(temp.path().join("retained")).unwrap(),
                    expected
                );
                if replacement {
                    assert_eq!(
                        fs::read(observed_path).unwrap(),
                        b"not JSON: a reopened-path reader must fail"
                    );
                } else {
                    assert!(!observed_path.exists());
                }
            }
        }
    }

    pub(crate) fn failures(mut write: impl FnMut(&Path, &mut Hooks<'_>) -> io::Result<PathBuf>) {
        let temp = tempfile::tempdir().unwrap();
        let blocker = temp.path().join("blocked");
        fs::write(&blocker, b"existing directory blocker").unwrap();
        let mut reached = false;
        let mut observe = |_, _: &File, _: &Path| {
            reached = true;
            Ok(())
        };
        assert!(write(&blocker.join("backups"), &mut Hooks::new(&mut observe)).is_err());
        assert!(!reached, "failed acquisition must not reach file creation");
        assert_eq!(fs::read(&blocker).unwrap(), b"existing directory blocker");
        let loop_path = temp.path().join("loop");
        symlink("loop", &loop_path).unwrap();
        assert!(write(&loop_path, &mut Hooks::default()).is_err());
        assert_eq!(fs::read_dir(temp.path()).unwrap().count(), 2);
        for boundary in [
            Boundary::Acquired,
            Boundary::FileSync,
            Boundary::Seek,
            Boundary::Read,
            Boundary::DirectorySync,
        ] {
            for fail_at in 0..=usize::from(boundary == Boundary::DirectorySync) {
                let temp = tempfile::tempdir().unwrap();
                let dir = temp.path().join("new/parent/backups");
                let mut calls = 0;
                let mut later = false;
                let mut failed = false;
                let mut callback = |stage, _: &File, _: &Path| {
                    if failed {
                        later = true;
                    }
                    if stage == boundary {
                        let call = calls;
                        calls += 1;
                        if call == fail_at {
                            failed = true;
                            return Err(io::Error::new(
                                io::ErrorKind::PermissionDenied,
                                "injected backup failure",
                            ));
                        }
                    }
                    Ok(())
                };
                let error = write(&dir, &mut Hooks::new(&mut callback)).unwrap_err();
                assert_eq!(error.kind(), io::ErrorKind::PermissionDenied);
                assert!(failed && !later, "failure must stop before any receipt");
                assert_eq!(
                    fs::read_dir(&dir).unwrap().count(),
                    usize::from(boundary != Boundary::Acquired),
                    "failure before creation leaves no file; later failures retain it"
                );
            }
        }
    }

    pub(crate) fn static_paths_and_collisions(
        mut write: impl FnMut(&Path, &mut Hooks<'_>) -> io::Result<PathBuf>,
        expected: &str,
    ) {
        let temp = tempfile::tempdir().unwrap();
        let target = tempfile::tempdir().unwrap();
        let target = target.path().canonicalize().unwrap().join("extra/nested");
        fs::create_dir_all(&target).unwrap();
        let alias = temp.path().join("alias");
        symlink(&target, &alias).unwrap();
        let dir = alias.join("hop/../new/backups");
        let mut synced = Vec::new();
        let mut callback = |stage, file: &File, _: &Path| {
            if stage == Boundary::DirectorySync {
                synced.push(id(file));
            }
            Ok(())
        };
        let receipt = write(&dir, &mut Hooks::new(&mut callback)).unwrap();
        assert_eq!(
            receipt.parent(),
            Some(dir.as_path()),
            "preserve absolute caller aliases and dot-dot"
        );
        assert_eq!(fs::read_to_string(&receipt).unwrap(), expected);
        assert_eq!(
            fs::metadata(&receipt).unwrap().permissions().mode() & 0o777,
            0o600
        );
        assert_eq!(synced.first(), Some(&path_id(&dir)));
        assert!(synced.contains(&path_id(&alias.join("hop"))));
        assert!(
            synced.contains(&path_id(temp.path())),
            "alias container must be retained"
        );
        assert!(
            synced.contains(&path_id(target.parent().unwrap())),
            "resolved target ancestors must be retained"
        );
        let mut unique = synced.clone();
        unique.sort_unstable();
        unique.dedup();
        assert_eq!(
            unique.len(),
            synced.len(),
            "deduplicate by inode and device"
        );
        let hop = alias.join("hop");
        let mut edges = std::collections::HashSet::new();
        for branch in [dir.as_path(), temp.path(), hop.as_path()] {
            let resolved = branch.canonicalize().unwrap();
            let ancestors: Vec<_> = resolved.ancestors().collect();
            for pair in ancestors.windows(2) {
                let edge = (path_id(pair[0]), path_id(pair[1]));
                if edges.insert(edge) {
                    let child = synced.iter().position(|id| *id == edge.0).unwrap();
                    let parent = synced.iter().position(|id| *id == edge.1).unwrap();
                    assert!(
                        child < parent,
                        "every retained branch must sync child before parent"
                    );
                }
            }
        }
        assert_eq!(
            write(&dir, &mut Hooks::default()).unwrap_err().kind(),
            io::ErrorKind::AlreadyExists
        );
        assert_eq!(fs::read_to_string(&receipt).unwrap(), expected);
        fs::remove_file(&receipt).unwrap();
        let control = temp.path().join("control");
        fs::write(&control, b"do not follow leaf symlink").unwrap();
        fs::set_permissions(&control, fs::Permissions::from_mode(0o640)).unwrap();
        symlink(&control, &receipt).unwrap();
        assert_eq!(
            write(&dir, &mut Hooks::default()).unwrap_err().kind(),
            io::ErrorKind::AlreadyExists
        );
        assert_eq!(fs::read(&control).unwrap(), b"do not follow leaf symlink");
        assert_eq!(
            fs::metadata(&control).unwrap().permissions().mode() & 0o777,
            0o640,
            "an existing control file's mode must not change"
        );
        assert!(fs::symlink_metadata(receipt)
            .unwrap()
            .file_type()
            .is_symlink());
    }

    #[cfg(target_os = "linux")]
    pub(crate) fn pathname_limit(
        mut write: impl FnMut(&Path, &mut Hooks<'_>) -> io::Result<PathBuf>,
    ) {
        let temp = tempfile::tempdir().unwrap();
        let mut directory = temp.path().to_owned();
        // Keep individual names modest; only the complete ordinary pathname exceeds its limit.
        let component = "d".repeat(200);
        while directory.as_os_str().len() <= 5000 {
            directory.push(&component);
        }
        let mut read_back = false;
        let mut callback = |stage, _: &File, _: &Path| {
            if stage == Boundary::ReadBack {
                read_back = true;
            }
            Ok(())
        };
        let result = write(&directory, &mut Hooks::new(&mut callback));
        assert!(
            result.is_err(),
            "an unusable ordinary pathname must not become a receipt"
        );
        assert_eq!(
            result.unwrap_err().raw_os_error(),
            Some(rustix::io::Errno::NAMETOOLONG.raw_os_error())
        );
        assert!(
            read_back,
            "the descriptor-relative writer must reach verified readback"
        );
    }

    pub(crate) fn relative_cwd(
        test_name: &str,
        mut write: impl FnMut(&Path, &mut Hooks<'_>) -> io::Result<PathBuf>,
        expected: &str,
    ) {
        const CHILD: &str = "KNXBENCH_PINNED_BACKUP_CWD_CHILD";
        if std::env::var_os(CHILD).as_deref() != Some(std::ffi::OsStr::new(test_name)) {
            let temp = tempfile::tempdir().unwrap();
            let output = std::process::Command::new(std::env::current_exe().unwrap())
                .args(["--exact", test_name, "--nocapture"])
                .env(CHILD, test_name)
                .current_dir(temp.path())
                .output()
                .unwrap();
            assert!(
                output.status.success(),
                "CWD child failed: {}\n{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
            assert!(
                String::from_utf8_lossy(&output.stdout).contains("1 passed"),
                "exact child filter must execute one test"
            );
            return;
        }
        // Only this isolated exact-test subprocess may change process-global CWD.
        let initial = std::env::current_dir().unwrap();
        for (boundary, rename) in [
            (Boundary::CwdOpened, false),
            (Boundary::CwdSpelled, true),
            (Boundary::Acquired, false),
            (Boundary::Written, false),
            (Boundary::ReadBack, false),
            (Boundary::Acquired, true),
            (Boundary::Written, true),
        ] {
            let temp = tempfile::tempdir_in(&initial).unwrap();
            let start = temp.path().join("start");
            let other = temp.path().join("other");
            let parked = temp.path().join("parked");
            fs::create_dir(&start).unwrap();
            fs::create_dir(&other).unwrap();
            std::env::set_current_dir(&start).unwrap();
            let mut reached = false;
            let mut basename = OsString::new();
            let mut callback = |stage, _: &File, path: &Path| {
                if stage == boundary {
                    reached = true;
                    if rename {
                        fs::rename(&start, &parked)?;
                        fs::create_dir(&start)?;
                    } else {
                        std::env::set_current_dir(&other)?;
                    }
                    if matches!(stage, Boundary::Acquired | Boundary::Written) {
                        basename = path.file_name().unwrap().to_owned();
                    }
                }
                Ok(())
            };
            let result = write(Path::new("new/backups"), &mut Hooks::new(&mut callback));
            std::env::set_current_dir(&initial).unwrap();
            assert!(reached);
            assert_eq!(
                fs::read_dir(&other).unwrap().count(),
                0,
                "a changed process CWD must never redirect writes"
            );
            if rename || boundary == Boundary::CwdOpened {
                assert!(result.is_err(), "inconsistent CWD spelling must refuse");
                assert_eq!(
                    fs::read_dir(&start).unwrap().count(),
                    0,
                    "replacement CWD must stay untouched"
                );
                if matches!(boundary, Boundary::Acquired | Boundary::Written) {
                    assert_eq!(
                        fs::read_to_string(parked.join("new/backups").join(&basename)).unwrap(),
                        expected
                    );
                } else if rename {
                    assert_eq!(
                        fs::read_dir(&parked).unwrap().count(),
                        0,
                        "CWD mismatch must precede mutation"
                    );
                }
            } else {
                let receipt = result.unwrap();
                assert!(receipt.is_absolute());
                assert_eq!(receipt.parent(), Some(start.join("new/backups").as_path()));
                assert_eq!(fs::read_to_string(receipt).unwrap(), expected);
            }
        }
    }

    use std::ffi::OsString;
}
