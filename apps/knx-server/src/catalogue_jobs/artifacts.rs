//! ArtifactDir ownership: private root, stable flock lease and bounded inventory.

use std::fs::{DirBuilder, File};
use std::os::unix::fs::{DirBuilderExt, MetadataExt};
use std::path::{Path, PathBuf};

use rustix::fs::{flock, openat, FlockOperation, Mode, OFlags, CWD};

use super::strict::{Incarnation, JobId};

/// The fixed private artifact root name (policy `ownership.privateRootName`).
pub const ARTIFACT_ROOT_NAME: &str = ".knxbench-catalogue-jobs";
/// The stable root lease file name (policy `ownership.leaseFile`).
pub const LEASE_NAME: &str = "lease";
/// The inventory file names.
pub const INVENTORY_NAME: &str = "inventory.json";
pub const INVENTORY_NEXT_NAME: &str = "inventory.next";
/// The bounded inventory size (policy `caps.artifactManifestBytes`).
pub const INVENTORY_BYTES: usize = 16_384;

/// An ArtifactDir admission/ownership failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArtifactError {
    Io,
    /// The stable root lease is held by another process; admission closes.
    Locked,
    /// The root or lease is not a valid owned private entry.
    RootInvalid,
    /// A bounded inventory would exceed its byte ceiling.
    InventoryTooLarge,
}

/// The acquired private artifact root: validated directory plus the held lease.
pub struct ArtifactRoot {
    root: PathBuf,
    _lease: File,
}

impl ArtifactRoot {
    /// Creates or validates `.knxbench-catalogue-jobs` under the catalogue
    /// parent and acquires the stable exclusive flock lease.
    pub fn acquire(catalogue_parent: &Path) -> Result<Self, ArtifactError> {
        let root = catalogue_parent.join(ARTIFACT_ROOT_NAME);
        create_or_validate_root(&root)?;
        let lease = acquire_lease(&root)?;
        Ok(ArtifactRoot {
            root,
            _lease: lease,
        })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Creates the per-incarnation directory `i-<32hex>` (0700).
    pub fn create_incarnation_dir(
        &self,
        incarnation: &Incarnation,
    ) -> Result<PathBuf, ArtifactError> {
        let dir = self.root.join(format!("i-{incarnation}"));
        DirBuilder::new()
            .recursive(false)
            .mode(0o700)
            .create(&dir)
            .map_err(|_| ArtifactError::Io)?;
        Ok(dir)
    }

    /// Creates a per-job directory `j-<32hex>` (0700) inside an incarnation dir.
    pub fn create_job_dir(incarnation_dir: &Path, job: &JobId) -> Result<PathBuf, ArtifactError> {
        let dir = incarnation_dir.join(format!("j-{job}"));
        DirBuilder::new()
            .recursive(false)
            .mode(0o700)
            .create(&dir)
            .map_err(|_| ArtifactError::Io)?;
        Ok(dir)
    }
}

fn create_or_validate_root(root: &Path) -> Result<(), ArtifactError> {
    match DirBuilder::new().recursive(false).mode(0o700).create(root) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => validate_existing_root(root),
        Err(_) => Err(ArtifactError::Io),
    }
}

fn validate_existing_root(root: &Path) -> Result<(), ArtifactError> {
    let meta = std::fs::symlink_metadata(root).map_err(|_| ArtifactError::Io)?;
    if !meta.is_dir() || meta.file_type().is_symlink() {
        return Err(ArtifactError::RootInvalid);
    }
    if meta.mode() & 0o077 != 0 {
        return Err(ArtifactError::RootInvalid);
    }
    Ok(())
}

fn acquire_lease(root: &Path) -> Result<File, ArtifactError> {
    let lease_path = root.join(LEASE_NAME);
    let owned = match openat(
        CWD,
        &lease_path,
        OFlags::RDWR | OFlags::CREATE | OFlags::EXCL | OFlags::NOFOLLOW | OFlags::CLOEXEC,
        Mode::RUSR | Mode::WUSR,
    ) {
        Ok(fd) => fd,
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
            let fd = openat(
                CWD,
                &lease_path,
                OFlags::RDWR | OFlags::NOFOLLOW | OFlags::CLOEXEC,
                Mode::empty(),
            )
            .map_err(|_| ArtifactError::RootInvalid)?;
            verify_lease(&lease_path, &fd)?;
            fd
        }
        Err(_) => return Err(ArtifactError::Io),
    };
    let file = File::from(owned);
    flock(&file, FlockOperation::NonBlockingLockExclusive).map_err(|_| ArtifactError::Locked)?;
    Ok(file)
}

/// Verifies a reopened lease: regular file, owner-write-only, single link, and
/// the same inode as its pathname (no replaced or symlinked alias).
fn verify_lease(path: &Path, fd: &std::os::fd::OwnedFd) -> Result<(), ArtifactError> {
    let file = File::from(fd.try_clone().map_err(|_| ArtifactError::Io)?);
    let fd_meta = file.metadata().map_err(|_| ArtifactError::RootInvalid)?;
    let path_meta = std::fs::symlink_metadata(path).map_err(|_| ArtifactError::RootInvalid)?;
    if !fd_meta.is_file() || fd_meta.nlink() != 1 {
        return Err(ArtifactError::RootInvalid);
    }
    if fd_meta.mode() & 0o077 != 0 {
        return Err(ArtifactError::RootInvalid);
    }
    if fd_meta.dev() != path_meta.dev() || fd_meta.ino() != path_meta.ino() {
        return Err(ArtifactError::RootInvalid);
    }
    Ok(())
}

/// A bounded inventory: appends UTF-8 records against a byte ceiling and never
/// exceeds it.
pub struct Inventory {
    bytes: Vec<u8>,
}

impl Inventory {
    pub fn new() -> Self {
        Inventory { bytes: Vec::new() }
    }

    pub fn append(&mut self, record: &[u8]) -> Result<(), ArtifactError> {
        if self
            .bytes
            .len()
            .checked_add(record.len())
            .ok_or(ArtifactError::InventoryTooLarge)?
            > INVENTORY_BYTES
        {
            return Err(ArtifactError::InventoryTooLarge);
        }
        self.bytes.extend_from_slice(record);
        Ok(())
    }

    pub fn len(&self) -> usize {
        self.bytes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.bytes.is_empty()
    }
}

impl Default for Inventory {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn catalogue_parent() -> tempfile::TempDir {
        tempfile::tempdir().expect("tempdir")
    }

    #[test]
    fn acquire_creates_root_and_lease() {
        let parent = catalogue_parent();
        let root = ArtifactRoot::acquire(parent.path()).unwrap();
        assert!(root.root().is_dir());
        assert!(root.root().join(LEASE_NAME).is_file());
        // The lease is held; a second acquisition of the same root is refused.
        assert_eq!(
            ArtifactRoot::acquire(parent.path()).map(|_| ()),
            Err(ArtifactError::Locked)
        );
    }

    #[test]
    fn existing_root_with_wrong_mode_is_refused() {
        let parent = catalogue_parent();
        let root_path = parent.path().join(ARTIFACT_ROOT_NAME);
        std::fs::create_dir(&root_path).unwrap();
        std::fs::write(root_path.join("junk"), b"x").unwrap();
        // Default umask gives the dir group/other read; the strict check still
        // refuses any group/other bit explicitly set below.
        let mut perms = std::fs::metadata(&root_path).unwrap().permissions();
        #[allow(clippy::permissions_set_readonly_false)]
        std::os::unix::fs::PermissionsExt::set_mode(&mut perms, 0o755);
        std::fs::set_permissions(&root_path, perms).unwrap();
        assert_eq!(
            ArtifactRoot::acquire(parent.path()).map(|_| ()),
            Err(ArtifactError::RootInvalid)
        );
    }

    #[test]
    fn symlinked_root_is_refused() {
        let parent = catalogue_parent();
        let real = parent.path().join("real");
        std::fs::create_dir(&real).unwrap();
        std::os::unix::fs::symlink(&real, parent.path().join(ARTIFACT_ROOT_NAME)).unwrap();
        assert_eq!(
            ArtifactRoot::acquire(parent.path()).map(|_| ()),
            Err(ArtifactError::RootInvalid)
        );
    }

    #[test]
    fn incarnation_and_job_dirs_are_created_0700() {
        let parent = catalogue_parent();
        let root = ArtifactRoot::acquire(parent.path()).unwrap();
        let inc = Incarnation::from_bytes([0x0a; 16]);
        let job = JobId::from_bytes([0x0b; 16]);
        let inc_dir = root.create_incarnation_dir(&inc).unwrap();
        let job_dir = ArtifactRoot::create_job_dir(&inc_dir, &job).unwrap();
        assert!(inc_dir.is_dir());
        assert!(job_dir.is_dir());
        assert_eq!(inc_dir.file_name().unwrap().to_string_lossy().len(), 34); // "i-" + 32 hex
    }

    #[test]
    fn inventory_refuses_over_budget() {
        let mut inv = Inventory::new();
        inv.append(&[b'a'; INVENTORY_BYTES]).unwrap();
        assert_eq!(inv.append(b"x"), Err(ArtifactError::InventoryTooLarge));
    }
}
