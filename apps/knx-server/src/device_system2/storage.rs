//! Private immutable operation evidence and explicit restart selection.
use super::dto::{valid_id, Metadata};
use knx_app::property_validation::{
    private_projection::{admit_snapshot, read_snapshot, PrivateSnapshot},
    read_bundle, write_prepared_bundle, ExecutionBuild, OperationProgress, OperationStore,
};
use knx_app::property_validation_bundle::PreparedBundle;
use std::{
    fs::{self, File, OpenOptions},
    io::{self, Read, Write},
    path::{Path, PathBuf},
};

#[cfg(test)]
#[derive(Default)]
pub(super) struct AdmissionReadCounts {
    metadata: std::sync::atomic::AtomicUsize,
    observed: std::sync::atomic::AtomicUsize,
    snapshot: std::sync::atomic::AtomicUsize,
}

#[cfg(test)]
impl AdmissionReadCounts {
    fn record(&self, read: AdmissionRead) {
        use std::sync::atomic::Ordering;
        match read {
            AdmissionRead::Metadata => self.metadata.fetch_add(1, Ordering::Relaxed),
            AdmissionRead::Observed => self.observed.fetch_add(1, Ordering::Relaxed),
            AdmissionRead::Snapshot => self.snapshot.fetch_add(1, Ordering::Relaxed),
        };
    }

    pub(super) fn values(&self) -> (usize, usize, usize) {
        use std::sync::atomic::Ordering;
        (
            self.metadata.load(Ordering::Relaxed),
            self.observed.load(Ordering::Relaxed),
            self.snapshot.load(Ordering::Relaxed),
        )
    }
}

#[derive(Clone, Copy)]
enum AdmissionRead {
    Metadata,
    Observed,
    Snapshot,
}

pub(crate) const ROOT_NAME: &str = ".knxbench-system2";
const MAX_OPERATIONS: usize = 128;
fn invalid() -> io::Error {
    io::Error::other("System-2 retained evidence unavailable or inconsistent")
}
fn sync_chain(path: &Path) -> io::Result<()> {
    for p in path.ancestors() {
        File::open(p)?.sync_all()?;
    }
    Ok(())
}
fn private(path: &Path, directory: bool) -> io::Result<()> {
    let m = fs::symlink_metadata(path)?;
    if (directory && !m.is_dir()) || (!directory && !m.is_file()) || m.file_type().is_symlink() {
        return Err(invalid());
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::{MetadataExt, PermissionsExt};
        if m.permissions().mode() & 0o077 != 0 || (!directory && m.nlink() != 1) {
            return Err(invalid());
        }
    }
    Ok(())
}
fn directory(path: &Path) -> io::Result<()> {
    let mut b = fs::DirBuilder::new();
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        b.mode(0o700);
    }
    b.create(path)?;
    sync_chain(path)
}
fn options(new: bool) -> OpenOptions {
    let mut o = OpenOptions::new();
    o.read(true).write(true).create_new(new);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        o.mode(0o600)
            .custom_flags(rustix::fs::OFlags::NOFOLLOW.bits() as i32);
    }
    o
}
fn read(path: &Path, bound: u64) -> io::Result<Vec<u8>> {
    private(path, false)?;
    let mut bytes = Vec::new();
    options(false)
        .open(path)?
        .take(bound + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() as u64 > bound {
        return Err(invalid());
    }
    Ok(bytes)
}
fn write_new(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let mut file = options(true).open(path)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    sync_chain(path.parent().ok_or_else(invalid)?)?;
    if read(path, bytes.len() as u64)? != bytes {
        return Err(invalid());
    }
    Ok(())
}
pub(super) struct Storage {
    root: PathBuf,
    _lease: File,
}

pub(super) struct BundleAdmission {
    pub metadata: Metadata,
    pub progress: OperationProgress,
    pub projection: knx_app::property_validation::private_projection::AdmittedPrivateProjection,
}

struct ReopenAdmission {
    path: PathBuf,
    metadata: Metadata,
    bundle: PreparedBundle,
    journal: PathBuf,
}

impl Storage {
    pub fn open(data_dir: &Path, create: bool) -> io::Result<Option<Self>> {
        let root = data_dir.join(ROOT_NAME);
        match fs::symlink_metadata(&root) {
            Err(e) if e.kind() == io::ErrorKind::NotFound && !create => return Ok(None),
            Err(e) if e.kind() == io::ErrorKind::NotFound => directory(&root)?,
            Err(e) => return Err(e),
            Ok(_) => private(&root, true)?,
        }
        let lease_path = root.join("service.lock");
        let lease = match options(true).open(&lease_path) {
            Ok(file) => {
                file.sync_all()?;
                sync_chain(&root)?;
                file
            }
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {
                private(&lease_path, false)?;
                options(false).open(&lease_path)?
            }
            Err(e) => return Err(e),
        };
        lease.try_lock().map_err(io::Error::other)?;
        Ok(Some(Self {
            root,
            _lease: lease,
        }))
    }
    pub fn ids(&self) -> io::Result<Vec<String>> {
        let mut ids = Vec::new();
        for entry in fs::read_dir(&self.root)? {
            let entry = entry?;
            let name = entry.file_name().into_string().map_err(|_| invalid())?;
            if name == "service.lock" || name == "selected" {
                continue;
            }
            // Uncommitted selection/unknown names require operator inspection,
            // never silently remove evidence or reclaim a possibly active operation.
            if !valid_id(&name) || ids.len() >= MAX_OPERATIONS {
                return Err(invalid());
            }
            private(&entry.path(), true)?;
            ids.push(name);
        }
        ids.sort();
        Ok(ids)
    }
    pub fn selected(&self) -> io::Result<Option<String>> {
        match read(&self.root.join("selected"), 32) {
            Ok(bytes) => {
                let id = String::from_utf8(bytes).map_err(|_| invalid())?;
                if !valid_id(&id) {
                    return Err(invalid());
                }
                Ok(Some(id))
            }
            Err(e) if e.kind() == io::ErrorKind::NotFound => {
                if !self.ids()?.is_empty() {
                    return Err(invalid());
                }
                Ok(None)
            }
            Err(e) => Err(e),
        }
    }
    pub fn select(&self, id: &str) -> io::Result<()> {
        if !valid_id(id) {
            return Err(invalid());
        }
        self.metadata(id)?;
        let next = self.root.join("selected-next");
        write_new(&next, id.as_bytes())?;
        fs::rename(next, self.root.join("selected"))?;
        sync_chain(&self.root)?;
        if self.selected()?.as_deref() != Some(id) {
            return Err(invalid());
        }
        Ok(())
    }
    fn path(&self, id: &str) -> io::Result<PathBuf> {
        if !valid_id(id) {
            return Err(invalid());
        }
        let path = self.root.join(id);
        private(&path, true)?;
        Ok(path)
    }
    pub fn create(&self, meta: &Metadata) -> io::Result<()> {
        if !valid_id(&meta.operation_id) || self.ids()?.len() >= MAX_OPERATIONS {
            return Err(invalid());
        }
        let path = self.root.join(&meta.operation_id);
        directory(&path)?;
        write_new(
            &path.join("metadata.json"),
            &serde_json::to_vec(meta).map_err(io::Error::other)?,
        )?;
        self.select(&meta.operation_id)
    }
    /// Operation-bound record of the build that executes this run; written once, before any
    /// device contact. Old operations have none and report an unknown execution build.
    pub fn record_execution_build(&self, id: &str, version: &str) -> io::Result<()> {
        if !matches!(
            ExecutionBuild::recorded(version),
            ExecutionBuild::Recorded { .. }
        ) {
            return Err(invalid());
        }
        let body = serde_json::json!({ "format": 1, "operationId": id, "version": version });
        write_new(
            &self.path(id)?.join("execution-build.json"),
            &serde_json::to_vec(&body).map_err(io::Error::other)?,
        )
    }
    /// The recorded execution build; any absent, oversized, foreign or malformed record is unknown.
    pub fn execution_build(&self, id: &str) -> ExecutionBuild {
        let read_record = || -> io::Result<ExecutionBuild> {
            let bytes = read(&self.path(id)?.join("execution-build.json"), 512)?;
            let value: serde_json::Value = serde_json::from_slice(&bytes).map_err(|_| invalid())?;
            let object = value.as_object().ok_or_else(invalid)?;
            if object.len() != 3
                || object.get("format") != Some(&serde_json::json!(1))
                || object.get("operationId").and_then(|v| v.as_str()) != Some(id)
            {
                return Err(invalid());
            }
            Ok(object
                .get("version")
                .and_then(|v| v.as_str())
                .map_or(ExecutionBuild::Unknown, ExecutionBuild::recorded))
        };
        read_record().unwrap_or(ExecutionBuild::Unknown)
    }
    fn metadata_at(path: &Path, id: &str) -> io::Result<Metadata> {
        let meta: Metadata = serde_json::from_slice(&read(&path.join("metadata.json"), 8192)?)
            .map_err(|_| invalid())?;
        let canonical_u64 = |s: &str| s.parse::<u64>().is_ok_and(|v| v.to_string() == s);
        if meta.schema_version != 1
            || meta.operation_id != id
            || meta.device_name.len() > 4096
            || meta.binding.server_incarnation.is_empty()
            || meta.binding.server_incarnation.len() > 128
            || !canonical_u64(&meta.binding.project_incarnation)
            || !canonical_u64(&meta.binding.project_revision)
            || meta.address.parse::<knx_core::IndividualAddress>().is_err()
        {
            return Err(invalid());
        }
        Ok(meta)
    }

    pub fn metadata(&self, id: &str) -> io::Result<Metadata> {
        Self::metadata_at(&self.path(id)?, id)
    }
    pub fn observed(&self, id: &str, bundle: &PreparedBundle) -> io::Result<OperationStore> {
        let path = self.path(id)?;
        let metadata = self.metadata(id)?;
        Self::check_bundle(id, bundle, &metadata)?;
        write_prepared_bundle(&path.join("observed.json"), bundle)?;
        private(&path.join("observed.json"), false)?;
        let check = read_bundle(&path.join("observed.json"))?;
        if check.binding != bundle.binding {
            return Err(invalid());
        }
        OperationStore::create(&path.join("journal"), bundle.binding.clone())
    }
    fn check_bundle(id: &str, bundle: &PreparedBundle, metadata: &Metadata) -> io::Result<()> {
        if bundle.format != 3
            || bundle.binding.simulated
            || bundle
                .hardware_preparation
                .as_ref()
                .is_none_or(|p| p.operation != id)
            || bundle.binding.target != metadata.address
        {
            return Err(invalid());
        }
        Ok(())
    }
    pub fn require_unclaimed_recovery(&self, id: &str) -> io::Result<()> {
        let path = self.path(id)?.join("recovery-command");
        match fs::symlink_metadata(path) {
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
            _ => Err(invalid()),
        }
    }
    /// Once-only HTTP recovery admission, before IP connect. Losing the response
    /// or crashing before App's RestoreStarted never grants a second attempt.
    pub fn claim_recovery(&self, id: &str) -> io::Result<()> {
        self.require_unclaimed_recovery(id)?;
        write_new(&self.path(id)?.join("recovery-command"), id.as_bytes())
    }
    fn reopen_admission_with(
        &self,
        id: &str,
        observe: &mut impl FnMut(AdmissionRead),
    ) -> io::Result<Option<ReopenAdmission>> {
        let path = self.path(id)?;
        let observed = path.join("observed.json");
        let journal = path.join("journal");
        let observed_exists = observed.try_exists()?;
        let journal_exists = journal.try_exists()?;
        if !observed_exists && !journal_exists {
            return Ok(None);
        }
        if !observed_exists || !journal_exists {
            return Err(invalid());
        }

        observe(AdmissionRead::Metadata);
        let metadata = Self::metadata_at(&path, id)?;
        private(&observed, false)?;
        observe(AdmissionRead::Observed);
        let bundle = read_bundle(&observed)?;
        Self::check_bundle(id, &bundle, &metadata)?;
        private(&journal, true)?;
        // Bound restart admission before the shared opener collects/replays its
        // exact chain. Unknown names still reach its strict identity grammar.
        let mut entries = 0usize;
        let mut octets = 0u64;
        for entry in fs::read_dir(&journal)? {
            let entry = entry?;
            entries = entries.checked_add(1).ok_or_else(invalid)?;
            private(&entry.path(), false)?;
            octets = octets
                .checked_add(fs::symlink_metadata(entry.path())?.len())
                .ok_or_else(invalid)?;
            if entries > 16_384 || octets > 64 * 1024 * 1024 {
                return Err(invalid());
            }
        }
        Ok(Some(ReopenAdmission {
            path,
            metadata,
            bundle,
            journal,
        }))
    }

    pub fn reopen(&self, id: &str) -> io::Result<(PreparedBundle, OperationStore)> {
        let admission = self
            .reopen_admission_with(id, &mut |_| {})?
            .ok_or_else(invalid)?;
        let store = OperationStore::open(&admission.journal, &admission.bundle.binding)?;
        Ok((admission.bundle, store))
    }

    pub fn bundle_admission(&self, id: &str) -> io::Result<Option<BundleAdmission>> {
        self.bundle_admission_with(id, |_| {})
    }

    #[cfg(test)]
    pub(super) fn bundle_admission_counted(
        &self,
        id: &str,
        counts: &AdmissionReadCounts,
    ) -> io::Result<Option<BundleAdmission>> {
        self.bundle_admission_with(id, |read| counts.record(read))
    }

    fn bundle_admission_with(
        &self,
        id: &str,
        mut observe: impl FnMut(AdmissionRead),
    ) -> io::Result<Option<BundleAdmission>> {
        let Some(admission) = self.reopen_admission_with(id, &mut observe)? else {
            return Ok(None);
        };
        observe(AdmissionRead::Snapshot);
        let snapshot = match read_snapshot(&admission.journal, &admission.bundle.binding) {
            PrivateSnapshot::Available(snapshot) => *snapshot,
            PrivateSnapshot::Unavailable(_) => return Err(invalid()),
        };
        let (mut progress, projection) = admit_snapshot(snapshot).map_err(|_| invalid())?;
        match read(&admission.path.join("recovery-command"), 32) {
            Ok(bytes) if bytes == id.as_bytes() => {
                progress.restore_available = false;
                progress.recovery_phrases = None;
                if !matches!(
                    progress.outcome,
                    knx_app::property_validation::LocalOutcome::Restored
                        | knx_app::property_validation::LocalOutcome::RestoreFailed
                ) {
                    progress.outcome = knx_app::property_validation::LocalOutcome::Interrupted;
                }
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            _ => return Err(invalid()),
        }
        Ok(Some(BundleAdmission {
            metadata: admission.metadata,
            progress,
            projection,
        }))
    }
    pub fn progress(&self, id: &str) -> io::Result<Option<OperationProgress>> {
        let path = self.path(id)?;
        // Before identity acquisition no mutation is possible and there is no
        // actual-state backup. A partially constructed/corrupt journal refuses.
        if !path.join("observed.json").try_exists()? && !path.join("journal").try_exists()? {
            return Ok(None);
        }
        let (_, store) = self.reopen(id)?;
        let mut progress = store.progress();
        match read(&path.join("recovery-command"), 32) {
            Ok(bytes) if bytes == id.as_bytes() => {
                progress.restore_available = false;
                progress.recovery_phrases = None;
                if !matches!(
                    progress.outcome,
                    knx_app::property_validation::LocalOutcome::Restored
                        | knx_app::property_validation::LocalOutcome::RestoreFailed
                ) {
                    progress.outcome = knx_app::property_validation::LocalOutcome::Interrupted;
                }
            }
            Err(e) if e.kind() == io::ErrorKind::NotFound => {}
            _ => return Err(invalid()),
        }
        Ok(Some(progress))
    }
}
