//! Identity-bound private workspaces. No recursive deletion or guessed ownership.

use std::collections::BTreeMap;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom, Write};
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use rustix::fs::{openat, renameat_with, Mode, OFlags, RenameFlags, CWD};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::artifacts::INVENTORY_BYTES;
use super::dto::{self, FailureKind};
use super::strict::Sha256 as WireSha256;
use super::{ArtifactError, ArtifactRoot};
use super::{Handle, U64};

const MAX_FILES: usize = 26;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OwnedIdentity {
    device: U64,
    inode: U64,
    link_count: U64,
    uid: U64,
    mode_octal: String,
    bytes: U64,
}
impl OwnedIdentity {
    fn read(file: &File, uid: u32) -> Result<Self, ArtifactError> {
        let m = file.metadata().map_err(|_| ArtifactError::Io)?;
        if !m.is_file() || m.nlink() != 1 || m.uid() != uid || m.mode() & 0o777 != 0o600 {
            return Err(ArtifactError::RootInvalid);
        }
        Ok(Self {
            device: U64(m.dev()),
            inode: U64(m.ino()),
            link_count: U64(1),
            uid: U64(u64::from(m.uid())),
            mode_octal: "0600".into(),
            bytes: U64(m.len()),
        })
    }
    fn same_inode(&self, other: &Self) -> bool {
        self.device == other.device && self.inode == other.inode && self.uid == other.uid
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct OwnedFile {
    name: String,
    identity: OwnedIdentity,
    role: String,
    state: String,
    #[serde(skip)]
    stamp: Option<(u64, i64, i64, i64, i64)>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct InventoryDocument {
    schema_version: u32,
    policy_version: String,
    server_incarnation: super::Incarnation,
    job_id: super::JobId,
    admission_serial: U64,
    files: Vec<OwnedFile>,
    recovery_publication: Option<RecoveryPublication>,
    health: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RecoveryPublication {
    pub name: String,
    device: U64,
    inode: U64,
    bytes: U64,
    pub identity: String,
    state: String,
    pub sync_acknowledged: bool,
}

/// This owner retains the root lease and exact filesystem identities even on
/// failure. Native connections remain in B10's existing destination inventory;
/// registering its returned create-new descriptor does not take/drop that handle.
pub struct JobWorkspace {
    _root: Arc<ArtifactRoot>,
    directory: PathBuf,
    directory_fd: File,
    directory_identity: (u64, u64),
    uid: u32,
    handle: Handle,
    files: BTreeMap<String, OwnedFile>,
    inventory_identity: Option<OwnedIdentity>,
    inventory_next: Option<OwnedIdentity>,
    recovery: Option<RecoveryPublication>,
    quarantined: bool,
    pub(crate) artifact_bytes: u64,
    pub(crate) artifact_records: u64,
    pub(crate) published: [Option<dto::Artifact>; 2],
}

impl JobWorkspace {
    pub fn create(
        root: Arc<ArtifactRoot>,
        incarnation_dir: &Path,
        handle: Handle,
    ) -> Result<Self, ArtifactError> {
        let directory = ArtifactRoot::create_job_dir(incarnation_dir, &handle.job)?;
        let directory_fd = open_directory(&directory)?;
        let meta = directory_fd.metadata().map_err(|_| ArtifactError::Io)?;
        let mut value = Self {
            _root: root,
            directory,
            directory_fd,
            directory_identity: (meta.dev(), meta.ino()),
            uid: rustix::process::geteuid().as_raw(),
            handle,
            files: BTreeMap::new(),
            inventory_identity: None,
            inventory_next: None,
            recovery: None,
            quarantined: false,
            artifact_bytes: 0,
            artifact_records: 0,
            published: [None, None],
        };
        value.persist()?;
        File::open(incarnation_dir)
            .and_then(|d| d.sync_all())
            .map_err(|_| ArtifactError::Io)?;
        Ok(value)
    }

    /// Sealing changes only the name of the already-inventoried source inode.
    pub(crate) fn register_sealed_source(&mut self) -> Result<(), ArtifactError> {
        let old = self
            .files
            .get("source.part")
            .ok_or(ArtifactError::RootInvalid)?
            .clone();
        let file = open_read(&self.directory.join("source.sealed"))?;
        let actual = OwnedIdentity::read(&file, self.uid)?;
        if !old.identity.same_inode(&actual) {
            return Err(ArtifactError::RootInvalid);
        }
        self.files.remove("source.part");
        self.files.insert(
            "source.sealed".into(),
            OwnedFile {
                name: "source.sealed".into(),
                identity: actual,
                role: "source".into(),
                state: "sealed".into(),
                stamp: Some(content_stamp(
                    &file.metadata().map_err(|_| ArtifactError::Io)?,
                )),
            },
        );
        self.persist()
    }

    /// Actual native handles must already be closed. Exact known transient
    /// files only; report mirrors/manifests and final user backups are retained.
    pub fn cleanup_rehearsal(&mut self) -> Result<(), ArtifactError> {
        self.cleanup_selected_transients(true)
    }

    pub fn cleanup_transients(&mut self) -> Result<(), ArtifactError> {
        self.cleanup_selected_transients(false)
    }

    fn cleanup_selected_transients(&mut self, rehearsal_only: bool) -> Result<(), ArtifactError> {
        let names: Vec<_> = self
            .files
            .keys()
            .filter(|n| !rehearsal_only || n.starts_with("rehearsal.sqlite"))
            .filter(|n| {
                role(n).is_some_and(|r| {
                    matches!(
                        r,
                        "source" | "rehearsal" | "recoveryStaging" | "sqliteSidecar"
                    )
                })
            })
            .cloned()
            .collect();
        for name in &names {
            self.checked_file(name)?;
        }
        // Unknown native sidecars cannot gain ownership merely by suffix.
        for main in ["rehearsal.sqlite", "recovery.part.sqlite"] {
            for suffix in ["-journal", "-wal", "-shm"] {
                let name = format!("{main}{suffix}");
                match std::fs::symlink_metadata(self.directory.join(&name)) {
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => (),
                    Ok(_) if self.files.contains_key(&name) => (),
                    _ => return Err(ArtifactError::RootInvalid),
                }
            }
        }
        for name in names {
            std::fs::remove_file(self.directory.join(&name)).map_err(|_| ArtifactError::Io)?;
            self.files.remove(&name);
        }
        self.directory_fd
            .sync_all()
            .map_err(|_| ArtifactError::Io)?;
        self.persist()
    }

    /// Only the running job's PROVED notCommitted path may call this. Retention
    /// and restart disposal never delete a final published backup.
    pub fn remove_uncommitted_backup(&mut self, parent: &Path) -> Result<(), ArtifactError> {
        let Some(publication) = self.recovery.as_ref() else {
            return Ok(());
        };
        let path = parent.join(&publication.name);
        let file = open_read(&path)?;
        let meta = file.metadata().map_err(|_| ArtifactError::Io)?;
        let current = OwnedIdentity::read(&file, self.uid)?;
        if current.device != publication.device
            || current.inode != publication.inode
            || current.bytes != publication.bytes
            || metadata_identity(&meta) != publication.identity
        {
            return Err(ArtifactError::RootInvalid);
        }
        std::fs::remove_file(path).map_err(|_| ArtifactError::Io)?;
        open_directory(parent)?
            .sync_all()
            .map_err(|_| ArtifactError::Io)?;
        self.recovery = None;
        self.persist()
    }

    /// Restart cleanup accepts ONLY complete bounded inventories and exact
    /// identities under the held stable lease; it never resumes old outcomes.
    pub fn reclaim_previous(root: &Arc<ArtifactRoot>) -> Result<(), ArtifactError> {
        let uid = std::fs::symlink_metadata(root.root())
            .map_err(|_| ArtifactError::Io)?
            .uid();
        let mut incarnations = 0usize;
        for entry in std::fs::read_dir(root.root()).map_err(|_| ArtifactError::Io)? {
            let entry = entry.map_err(|_| ArtifactError::Io)?;
            if entry.file_name() == super::artifacts::LEASE_NAME {
                continue;
            }
            incarnations += 1;
            if incarnations > 10 {
                return Err(ArtifactError::RootInvalid);
            }
            let name = entry.file_name();
            let name = name.to_str().ok_or(ArtifactError::RootInvalid)?;
            let incarnation = name
                .strip_prefix("i-")
                .and_then(|s| s.parse::<super::Incarnation>().ok())
                .ok_or(ArtifactError::RootInvalid)?;
            let incarnation_fd = open_directory(&entry.path())?;
            let meta = incarnation_fd.metadata().map_err(|_| ArtifactError::Io)?;
            if meta.uid() != uid || meta.mode() & 0o777 != 0o700 {
                return Err(ArtifactError::RootInvalid);
            }
            let mut jobs = 0;
            for child in std::fs::read_dir(entry.path()).map_err(|_| ArtifactError::Io)? {
                jobs += 1;
                if jobs > 9 {
                    return Err(ArtifactError::RootInvalid);
                }
                let child = child.map_err(|_| ArtifactError::Io)?;
                let job = child.file_name();
                let job = job
                    .to_str()
                    .and_then(|n| n.strip_prefix("j-"))
                    .and_then(|n| n.parse::<super::JobId>().ok())
                    .ok_or(ArtifactError::RootInvalid)?;
                let directory = child.path();
                let directory_fd = open_directory(&directory)?;
                let dm = directory_fd.metadata().map_err(|_| ArtifactError::Io)?;
                if dm.uid() != uid || dm.mode() & 0o777 != 0o700 {
                    return Err(ArtifactError::RootInvalid);
                }
                // Only a truly empty unknown directory is removable without an inventory.
                if std::fs::read_dir(&directory)
                    .map_err(|_| ArtifactError::Io)?
                    .next()
                    .is_none()
                {
                    std::fs::remove_dir(&directory).map_err(|_| ArtifactError::Io)?;
                    continue;
                }
                let mut file = open_read(&directory.join("inventory.json"))?;
                let inventory_identity = OwnedIdentity::read(&file, uid)?;
                if inventory_identity.bytes.get() > INVENTORY_BYTES as u64 {
                    return Err(ArtifactError::InventoryTooLarge);
                }
                let mut bytes = Vec::new();
                file.read_to_end(&mut bytes)
                    .map_err(|_| ArtifactError::Io)?;
                if bytes.len() > INVENTORY_BYTES {
                    return Err(ArtifactError::InventoryTooLarge);
                }
                let text = std::str::from_utf8(&bytes).map_err(|_| ArtifactError::RootInvalid)?;
                let doc: InventoryDocument =
                    dto::from_str(text).map_err(|_| ArtifactError::RootInvalid)?;
                if doc.schema_version != 1
                    || doc.policy_version != dto::POLICY_VERSION
                    || doc.server_incarnation != incarnation
                    || doc.job_id != job
                    || doc.admission_serial.get() == 0
                    || doc.admission_serial == U64::MAX
                    || doc.health != "healthy"
                    || doc.recovery_publication.is_some()
                    || doc.files.len() > MAX_FILES
                {
                    return Err(ArtifactError::RootInvalid);
                }
                let mut files = BTreeMap::new();
                for owned in doc.files {
                    if role(&owned.name) != Some(owned.role.as_str())
                        || !matches!(
                            owned.state.as_str(),
                            "created" | "writing" | "sealed" | "complete" | "removePending"
                        )
                        || files.contains_key(&owned.name)
                    {
                        return Err(ArtifactError::RootInvalid);
                    }
                    files.insert(owned.name.clone(), owned);
                }
                let mut workspace = Self {
                    _root: Arc::clone(root),
                    directory,
                    directory_fd,
                    directory_identity: (dm.dev(), dm.ino()),
                    uid,
                    handle: Handle {
                        incarnation,
                        serial: doc.admission_serial,
                        job,
                    },
                    files,
                    inventory_identity: Some(inventory_identity),
                    inventory_next: None,
                    recovery: None,
                    quarantined: false,
                    artifact_bytes: 0,
                    artifact_records: 0,
                    published: [None, None],
                };
                workspace.dispose()?;
            }
            incarnation_fd.sync_all().map_err(|_| ArtifactError::Io)?;
            std::fs::remove_dir(entry.path()).map_err(|_| ArtifactError::Io)?;
        }
        open_directory(root.root())?
            .sync_all()
            .map_err(|_| ArtifactError::Io)
    }

    pub(super) fn catalogue_parent(&self) -> Option<&Path> {
        self._root.root().parent()
    }

    pub fn directory(&self) -> &Path {
        &self.directory
    }
    pub fn handle(&self) -> Handle {
        self.handle
    }
    pub(super) fn record_native_publication(
        &mut self,
        name: &str,
        file: &File,
        synced: bool,
    ) -> Result<RecoveryPublication, ArtifactError> {
        if name.is_empty() || name.len() > 512 || name.contains(['/', '\\', ':']) {
            return Err(ArtifactError::RootInvalid);
        }
        let expected = self
            .files
            .get("recovery.part.sqlite")
            .ok_or(ArtifactError::RootInvalid)?;
        let identity = OwnedIdentity::read(file, self.uid)?;
        let target = self
            ._root
            .root()
            .parent()
            .ok_or(ArtifactError::RootInvalid)?
            .join(name);
        let actual = OwnedIdentity::read(&open_read(&target)?, self.uid)?;
        if !expected.identity.same_inode(&identity) || !identity.same_inode(&actual) {
            return Err(ArtifactError::RootInvalid);
        }
        let meta = file.metadata().map_err(|_| ArtifactError::Io)?;
        let publication = RecoveryPublication {
            name: name.into(),
            device: identity.device,
            inode: identity.inode,
            bytes: identity.bytes,
            identity: metadata_identity(&meta),
            state: if synced { "published" } else { "quarantined" }.into(),
            sync_acknowledged: synced,
        };
        self.files.remove("recovery.part.sqlite");
        self.recovery = Some(publication.clone());
        if !synced {
            self.quarantined = true;
        }
        self.persist()?;
        Ok(publication)
    }

    pub fn recovery_publication(&self) -> Option<&RecoveryPublication> {
        self.recovery.as_ref()
    }

    pub fn retain_recovery(&mut self) -> Result<(), ArtifactError> {
        let publication = self.recovery.as_mut().ok_or(ArtifactError::RootInvalid)?;
        if !publication.sync_acknowledged {
            return Err(ArtifactError::RootInvalid);
        }
        publication.state = "retained".into();
        self.persist()
    }

    pub fn quarantined(&self) -> bool {
        self.quarantined
    }

    fn check_directory(&self) -> Result<(), ArtifactError> {
        let m = std::fs::symlink_metadata(&self.directory).map_err(|_| ArtifactError::Io)?;
        if !m.is_dir()
            || m.file_type().is_symlink()
            || m.uid() != self.uid
            || m.mode() & 0o777 != 0o700
            || self.directory_identity != (m.dev(), m.ino())
        {
            return Err(ArtifactError::RootInvalid);
        }
        Ok(())
    }

    fn checked_file(&self, name: &str) -> Result<File, ArtifactError> {
        self.check_directory()?;
        let entry = self.files.get(name).ok_or(ArtifactError::RootInvalid)?;
        let file = open_read(&self.directory.join(name))?;
        let identity = OwnedIdentity::read(&file, self.uid)?;
        if !entry.identity.same_inode(&identity)
            || entry.stamp.is_some_and(|stamp| {
                file.metadata()
                    .map(|m| content_stamp(&m) != stamp)
                    .unwrap_or(true)
            })
        {
            return Err(ArtifactError::RootInvalid);
        }
        Ok(file)
    }

    /// Allocates only fixed application filenames, records the actual returned
    /// descriptor before any payload/native I/O, and persists that ownership.
    pub fn create_file(&mut self, name: &str) -> Result<File, ArtifactError> {
        self.check_directory()?;
        let role = role(name).ok_or(ArtifactError::RootInvalid)?;
        if self.quarantined || self.files.len() >= MAX_FILES || self.files.contains_key(name) {
            return Err(ArtifactError::RootInvalid);
        }
        let file = create_new(&self.directory.join(name))?;
        let identity = OwnedIdentity::read(&file, self.uid)?;
        self.files.insert(
            name.into(),
            OwnedFile {
                name: name.into(),
                identity,
                role: role.into(),
                state: "created".into(),
                stamp: None,
            },
        );
        self.persist()?;
        Ok(file)
    }

    /// Adapter for the existing B10 destination allocator. Call immediately on
    /// its create-new descriptor, BEFORE SQLite opens it. Never adopt by path.
    pub fn register_database(
        &mut self,
        name: &str,
        descriptor: &File,
    ) -> Result<(), ArtifactError> {
        if !matches!(name, "rehearsal.sqlite" | "recovery.part.sqlite")
            || self.files.contains_key(name)
        {
            return Err(ArtifactError::RootInvalid);
        }
        self.check_directory()?;
        let identity = OwnedIdentity::read(descriptor, self.uid)?;
        let actual = OwnedIdentity::read(&open_read(&self.directory.join(name))?, self.uid)?;
        if !identity.same_inode(&actual) {
            return Err(ArtifactError::RootInvalid);
        }
        self.files.insert(
            name.into(),
            OwnedFile {
                name: name.into(),
                identity,
                role: role(name).unwrap().into(),
                state: "created".into(),
                stamp: None,
            },
        );
        self.persist()
    }

    pub fn refresh(&mut self, name: &str, state: &str) -> Result<(), ArtifactError> {
        if !matches!(
            state,
            "created" | "writing" | "sealed" | "complete" | "removePending" | "quarantined"
        ) {
            return Err(ArtifactError::RootInvalid);
        }
        let file = self.checked_file(name)?;
        let identity = OwnedIdentity::read(&file, self.uid)?;
        let entry = self.files.get_mut(name).ok_or(ArtifactError::RootInvalid)?;
        entry.identity = identity;
        entry.state = state.into();
        entry.stamp = if matches!(state, "complete" | "sealed") {
            Some(content_stamp(
                &file.metadata().map_err(|_| ArtifactError::Io)?,
            ))
        } else {
            None
        };
        self.persist()
    }

    pub fn seal_file(&mut self, part: &str, complete: &str) -> Result<(), ArtifactError> {
        if role(part) != role(complete) || part == complete || self.files.contains_key(complete) {
            return Err(ArtifactError::RootInvalid);
        }
        let file = self.checked_file(part)?;
        file.sync_all().map_err(|_| ArtifactError::Io)?;
        renameat_with(
            &self.directory_fd,
            part,
            &self.directory_fd,
            complete,
            RenameFlags::NOREPLACE,
        )
        .map_err(|_| ArtifactError::Io)?;
        let mut entry = self.files.remove(part).ok_or(ArtifactError::RootInvalid)?;
        entry.name = complete.into();
        entry.state = "complete".into();
        entry.identity = OwnedIdentity::read(&file, self.uid)?;
        entry.stamp = Some(content_stamp(
            &file.metadata().map_err(|_| ArtifactError::Io)?,
        ));
        self.files.insert(complete.into(), entry);
        self.directory_fd
            .sync_all()
            .map_err(|_| ArtifactError::Io)?;
        self.persist()
    }

    /// Bounded raw Vec for the existing importer, not a streaming/RSS claim.
    pub fn read_source(
        &self,
        expected_bytes: u64,
        expected_hash: WireSha256,
    ) -> Result<Vec<u8>, FailureKind> {
        if expected_bytes > dto::MAX_SOURCE_BYTES {
            return Err(FailureKind::UploadLengthMismatch);
        }
        let mut file = self
            .checked_file("source.sealed")
            .map_err(|_| FailureKind::IoFailure)?;
        if file.metadata().map_err(|_| FailureKind::IoFailure)?.len() != expected_bytes {
            return Err(FailureKind::UploadLengthMismatch);
        }
        let capacity = usize::try_from(expected_bytes).map_err(|_| FailureKind::BodyLimit)?;
        let mut bytes = Vec::new();
        bytes
            .try_reserve_exact(capacity)
            .map_err(|_| FailureKind::BodyLimit)?;
        let mut hash = Sha256::new();
        let mut scratch = [0u8; super::spool::IO_CHUNK_BYTES];
        loop {
            let n = file
                .read(&mut scratch)
                .map_err(|_| FailureKind::IoFailure)?;
            if n == 0 {
                break;
            }
            if bytes.len().checked_add(n).is_none_or(|n| n > capacity) {
                return Err(FailureKind::UploadLengthMismatch);
            }
            hash.update(&scratch[..n]);
            bytes.extend_from_slice(&scratch[..n]);
        }
        let observed: [u8; 32] = hash.finalize().into();
        if bytes.len() != capacity || &observed != expected_hash.as_bytes() {
            return Err(FailureKind::ReportMismatch);
        }
        Ok(bytes)
    }

    /// No native handles may be live while this is called. WAL/SHM are never
    /// adopted in the DELETE lane; a surviving journal needs B10's explicit
    /// closed/non-hot evidence, not merely a known suffix.
    pub fn admit_database_interval(&self, name: &str) -> Result<(), ArtifactError> {
        if !matches!(name, "rehearsal.sqlite" | "recovery.part.sqlite") {
            return Err(ArtifactError::RootInvalid);
        }
        self.checked_file(name)?;
        for suffix in ["-journal", "-wal", "-shm"] {
            match std::fs::symlink_metadata(self.directory.join(format!("{name}{suffix}"))) {
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => (),
                _ => return Err(ArtifactError::RootInvalid),
            }
        }
        Ok(())
    }

    /// Called only after B10 explicitly closed every native handle and proved a
    /// surviving rollback journal non-hot. The actual returned FD is inventoried.
    pub fn reconcile_closed_database(
        &mut self,
        name: &str,
        journal_non_hot: bool,
    ) -> Result<(), ArtifactError> {
        self.refresh(name, "complete")?;
        for suffix in ["-wal", "-shm"] {
            match std::fs::symlink_metadata(self.directory.join(format!("{name}{suffix}"))) {
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => (),
                _ => return Err(ArtifactError::RootInvalid),
            }
        }
        let journal = format!("{name}-journal");
        match std::fs::symlink_metadata(self.directory.join(&journal)) {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Ok(_) if journal_non_hot => {
                let file = open_read(&self.directory.join(&journal))?;
                let identity = OwnedIdentity::read(&file, self.uid)?;
                if identity.bytes.get() > 64 * 1024 * 1024 * 1024 {
                    return Err(ArtifactError::RootInvalid);
                }
                self.files.insert(
                    journal.clone(),
                    OwnedFile {
                        name: journal,
                        identity,
                        role: "sqliteSidecar".into(),
                        state: "complete".into(),
                        stamp: Some(content_stamp(
                            &file.metadata().map_err(|_| ArtifactError::Io)?,
                        )),
                    },
                );
                self.persist()
            }
            _ => Err(ArtifactError::RootInvalid),
        }
    }

    /// Publishes only a closed, fully validated B10 recovery destination. The
    /// caller's validation receipt must precede this call; publication itself
    /// never treats SQLite Done or a pathname as a validation witness.
    #[cfg(test)]
    fn publish_recovery(
        &mut self,
        catalogue: &Path,
        seconds: u64,
        upload_sha: WireSha256,
    ) -> Result<RecoveryPublication, ArtifactError> {
        self.admit_database_interval("recovery.part.sqlite")?;
        let file = self.checked_file("recovery.part.sqlite")?;
        file.sync_all().map_err(|_| ArtifactError::Io)?;
        let basename = catalogue
            .file_name()
            .and_then(|s| s.to_str())
            .ok_or(ArtifactError::RootInvalid)?;
        if basename.len() > 400 || basename.contains(['/', '\\', ':']) {
            return Err(ArtifactError::RootInvalid);
        }
        let parent = catalogue.parent().ok_or(ArtifactError::RootInvalid)?;
        let parent_fd = open_directory(parent)?;
        let digest = upload_sha.to_string();
        for attempt in 0..1000 {
            let suffix = if attempt == 0 {
                String::new()
            } else {
                format!("-{attempt}")
            };
            let name = format!(
                "{basename}.backup-{seconds}-{}{suffix}.sqlite",
                &digest[..12]
            );
            match renameat_with(
                &self.directory_fd,
                "recovery.part.sqlite",
                &parent_fd,
                name.as_str(),
                RenameFlags::NOREPLACE,
            ) {
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(_) => return Err(ArtifactError::Io),
                Ok(()) => {
                    let m = file.metadata().map_err(|_| ArtifactError::Io)?;
                    // Match the existing backup identity algorithm (metadata,
                    // not content); no raw pathname is retained in wire state.
                    let identity = metadata_identity(&m);
                    let mut publication = RecoveryPublication {
                        name,
                        device: U64(m.dev()),
                        inode: U64(m.ino()),
                        bytes: U64(m.len()),
                        identity,
                        state: "quarantined".into(),
                        sync_acknowledged: false,
                    };
                    self.files.remove("recovery.part.sqlite");
                    self.recovery = Some(publication.clone());
                    // Retain actual publication even when directory sync fails.
                    if parent_fd
                        .sync_all()
                        .and_then(|_| self.directory_fd.sync_all())
                        .is_err()
                    {
                        self.quarantined = true;
                        let _ = self.persist();
                        return Err(ArtifactError::Io);
                    }
                    publication.state = "published".into();
                    publication.sync_acknowledged = true;
                    self.recovery = Some(publication.clone());
                    self.persist()?;
                    return Ok(publication);
                }
            }
        }
        Err(ArtifactError::Io)
    }

    pub fn quarantine(&mut self) {
        self.quarantined = true;
        let _ = self.persist();
    }

    fn persist(&mut self) -> Result<(), ArtifactError> {
        self.check_directory()?;
        let doc = InventoryDocument {
            schema_version: 1,
            policy_version: dto::POLICY_VERSION.into(),
            server_incarnation: self.handle.incarnation,
            job_id: self.handle.job,
            admission_serial: self.handle.serial,
            files: self.files.values().cloned().collect(),
            recovery_publication: self.recovery.clone(),
            health: if self.quarantined {
                "quarantined"
            } else {
                "healthy"
            }
            .into(),
        };
        let bytes =
            bounded_json(&doc, INVENTORY_BYTES).map_err(|_| ArtifactError::InventoryTooLarge)?;
        let mut next = create_new(&self.directory.join("inventory.next"))?;
        self.inventory_next = Some(OwnedIdentity::read(&next, self.uid)?);
        next.write_all(&bytes)
            .and_then(|_| next.sync_all())
            .map_err(|_| ArtifactError::Io)?;
        self.inventory_next = Some(OwnedIdentity::read(&next, self.uid)?);
        if let Some(old) = &self.inventory_identity {
            let current = open_read(&self.directory.join("inventory.json"))?;
            if !old.same_inode(&OwnedIdentity::read(&current, self.uid)?) {
                return Err(ArtifactError::RootInvalid);
            }
            std::fs::rename(
                self.directory.join("inventory.next"),
                self.directory.join("inventory.json"),
            )
            .map_err(|_| ArtifactError::Io)?;
        } else {
            renameat_with(
                &self.directory_fd,
                "inventory.next",
                &self.directory_fd,
                "inventory.json",
                RenameFlags::NOREPLACE,
            )
            .map_err(|_| ArtifactError::Io)?;
        }
        self.inventory_identity = self.inventory_next.take();
        self.directory_fd.sync_all().map_err(|_| ArtifactError::Io)
    }

    /// Exact identity removal only. Published backups are NEVER removed here.
    /// A prior read/worker must have released this workspace before disposal.
    pub fn dispose(&mut self) -> Result<(), ArtifactError> {
        if self.quarantined {
            return Err(ArtifactError::RootInvalid);
        }
        self.check_directory()?;
        let names: Vec<_> = std::fs::read_dir(&self.directory)
            .map_err(|_| ArtifactError::Io)?
            .take(MAX_FILES + 3)
            .map(|e| e.map(|e| e.file_name()))
            .collect::<Result<_, _>>()
            .map_err(|_| ArtifactError::Io)?;
        if names.len() > MAX_FILES + 2 {
            return Err(ArtifactError::RootInvalid);
        }
        for name in &names {
            let name = name.to_str().ok_or(ArtifactError::RootInvalid)?;
            if name == "inventory.json" {
                continue;
            }
            if name == "inventory.next" && self.inventory_next.is_some() {
                continue;
            }
            self.checked_file(name)?;
        }
        for name in self.files.keys() {
            self.checked_file(name)?;
        }
        // Check every identity before the first unlink; changed entries never
        // gain deletion authority through a partial cleanup.
        for (name, expected) in [
            ("inventory.json", self.inventory_identity.as_ref()),
            ("inventory.next", self.inventory_next.as_ref()),
        ] {
            if let Some(expected) = expected {
                let actual =
                    OwnedIdentity::read(&open_read(&self.directory.join(name))?, self.uid)?;
                if !expected.same_inode(&actual) {
                    return Err(ArtifactError::RootInvalid);
                }
            }
        }
        for name in self.files.keys() {
            std::fs::remove_file(self.directory.join(name)).map_err(|_| ArtifactError::Io)?;
        }
        for name in ["inventory.next", "inventory.json"] {
            if names.iter().any(|n| n == name) {
                std::fs::remove_file(self.directory.join(name)).map_err(|_| ArtifactError::Io)?;
            }
        }
        self.directory_fd
            .sync_all()
            .map_err(|_| ArtifactError::Io)?;
        std::fs::remove_dir(&self.directory).map_err(|_| ArtifactError::Io)?;
        File::open(self.directory.parent().ok_or(ArtifactError::Io)?)
            .and_then(|f| f.sync_all())
            .map_err(|_| ArtifactError::Io)
    }

    pub(crate) fn open_artifact(&self, name: &str) -> Result<File, ArtifactError> {
        self.checked_file(name)
    }

    pub(crate) fn read_slice(
        &self,
        name: &str,
        offset: u64,
        length: usize,
    ) -> Result<Vec<u8>, ArtifactError> {
        let mut file = self.checked_file(name)?;
        file.seek(SeekFrom::Start(offset))
            .map_err(|_| ArtifactError::Io)?;
        let mut bytes = Vec::new();
        bytes
            .try_reserve_exact(length)
            .map_err(|_| ArtifactError::Io)?;
        bytes.resize(length, 0);
        file.read_exact(&mut bytes).map_err(|_| ArtifactError::Io)?;
        Ok(bytes)
    }
}

fn content_stamp(m: &std::fs::Metadata) -> (u64, i64, i64, i64, i64) {
    (
        m.len(),
        m.mtime(),
        m.mtime_nsec(),
        m.ctime(),
        m.ctime_nsec(),
    )
}

fn metadata_identity(m: &std::fs::Metadata) -> String {
    let fingerprint = format!(
        "{}:{}:{}:{}.{}:{}.{}",
        m.dev(),
        m.ino(),
        m.len(),
        m.mtime(),
        m.mtime_nsec(),
        m.ctime(),
        m.ctime_nsec()
    );
    Sha256::digest(fingerprint.as_bytes())[..16]
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

fn role(name: &str) -> Option<&'static str> {
    Some(match name {
        "source.part" | "source.sealed" => "source",
        "rehearsal.sqlite" => "rehearsal",
        "recovery.part.sqlite" => "recoveryStaging",
        "rehearsal.sqlite-journal" | "recovery.part.sqlite-journal" => "sqliteSidecar",
        "preview.raw.part" | "preview.raw" => "previewRaw",
        "preview.public.part" | "preview.public" => "previewPublic",
        "preview.index.part" | "preview.index" => "previewIndex",
        "preview.manifest.part" | "preview.manifest" => "previewManifest",
        "result.raw.part" | "result.raw" => "resultRaw",
        "result.public.part" | "result.public" => "resultPublic",
        "result.index.part" | "result.index" => "resultIndex",
        "result.manifest.part" | "result.manifest" => "resultManifest",
        _ => return None,
    })
}
fn create_new(path: &Path) -> Result<File, ArtifactError> {
    let fd = openat(
        CWD,
        path,
        OFlags::RDWR | OFlags::CREATE | OFlags::EXCL | OFlags::NOFOLLOW | OFlags::CLOEXEC,
        Mode::RUSR | Mode::WUSR,
    )
    .map_err(|_| ArtifactError::Io)?;
    Ok(File::from(fd))
}
fn open_read(path: &Path) -> Result<File, ArtifactError> {
    let fd = openat(
        CWD,
        path,
        OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
        Mode::empty(),
    )
    .map_err(|_| ArtifactError::RootInvalid)?;
    Ok(File::from(fd))
}
fn open_directory(path: &Path) -> Result<File, ArtifactError> {
    let fd = openat(
        CWD,
        path,
        OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
        Mode::empty(),
    )
    .map_err(|_| ArtifactError::RootInvalid)?;
    Ok(File::from(fd))
}

/// Counts first, reserves exactly once, and never grows beyond the supplied cap.
pub(crate) fn bounded_json<T: Serialize + ?Sized>(
    value: &T,
    cap: usize,
) -> Result<Vec<u8>, FailureKind> {
    struct Counter {
        used: usize,
        cap: usize,
    }
    impl Write for Counter {
        fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
            self.used = self
                .used
                .checked_add(b.len())
                .filter(|n| *n <= self.cap)
                .ok_or_else(|| std::io::Error::other("bounded serialization"))?;
            Ok(b.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let mut count = Counter { used: 0, cap };
    serde_json::to_writer(&mut count, value).map_err(|_| FailureKind::RecordLimit)?;
    let mut out = Vec::new();
    out.try_reserve_exact(count.used)
        .map_err(|_| FailureKind::ResultLimit)?;
    struct BoundedOutput<'a> {
        bytes: &'a mut Vec<u8>,
        limit: usize,
    }
    impl Write for BoundedOutput<'_> {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            if self
                .bytes
                .len()
                .checked_add(bytes.len())
                .is_none_or(|n| n > self.limit)
            {
                return Err(std::io::Error::other(
                    "bounded serialization changed between passes",
                ));
            }
            self.bytes.extend_from_slice(bytes);
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    serde_json::to_writer(
        &mut BoundedOutput {
            bytes: &mut out,
            limit: count.used,
        },
        value,
    )
    .map_err(|_| FailureKind::RecordLimit)?;
    if out.len() != count.used {
        return Err(FailureKind::ReportMismatch);
    }
    Ok(out)
}

#[cfg(test)]
mod ownership_tests;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalogue_jobs::{Incarnation, JobId};

    fn workspace() -> (tempfile::TempDir, JobWorkspace) {
        let tmp = tempfile::tempdir().unwrap();
        let root = Arc::new(ArtifactRoot::acquire(tmp.path()).unwrap());
        let incarnation = Incarnation::from_bytes([1; 16]);
        let dir = root.create_incarnation_dir(&incarnation).unwrap();
        let handle = Handle {
            incarnation,
            serial: U64(1),
            job: JobId::from_bytes([2; 16]),
        };
        let workspace = JobWorkspace::create(root, &dir, handle).unwrap();
        (tmp, workspace)
    }

    #[test]
    fn new_workspace_has_durable_bounded_identity_inventory() {
        let (_tmp, workspace) = workspace();
        assert!(
            workspace.directory.join("inventory.json").is_file(),
            "workspace must publish its bounded ownership inventory before use"
        );
        let bytes = std::fs::read(workspace.directory.join("inventory.json")).unwrap();
        assert!(bytes.len() <= 16_384);
        let value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(value["jobId"], workspace.handle.job.to_string());
        assert_eq!(value["files"], serde_json::json!([]));
        assert_eq!(value["health"], "healthy");
    }
}
