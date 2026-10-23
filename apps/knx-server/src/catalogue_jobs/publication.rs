//! Actual server durable-publication adapter for the B10 pre-live barrier.
use super::{persistence::JobWorkspace, Sha256};
use knx_productdb::managed_apply::{
    PublishedRecovery, RecoveryPublicationError, RecoveryPublisher, ValidatedRecovery,
};
use std::path::{Path, PathBuf};

/// Required complete-record adapter. Seal/admit the real full result and retain
/// its descriptor outside the worker unwind boundary before returning Ok.
/// There is deliberately no default/quiet result sink in production.
pub trait JobResultAdmission {
    fn admit_result(
        &mut self,
        install: &knx_productdb::ProductFileInstall,
        catalogue_page_bytes: u64,
        workspace: &mut JobWorkspace,
    ) -> Result<(), RecoveryPublicationError>;
}

pub struct JobRecoveryPublisher<'a> {
    workspace: &'a mut JobWorkspace,
    catalogue: PathBuf,
    upload_sha: Sha256,
    seconds: u64,
    result: &'a mut dyn JobResultAdmission,
}
impl<'a> JobRecoveryPublisher<'a> {
    pub fn new(
        workspace: &'a mut JobWorkspace,
        catalogue: &Path,
        upload_sha: Sha256,
        result: &'a mut dyn JobResultAdmission,
    ) -> Self {
        let seconds = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        Self {
            workspace,
            catalogue: catalogue.to_path_buf(),
            upload_sha,
            seconds,
            result,
        }
    }
}
impl RecoveryPublisher for JobRecoveryPublisher<'_> {
    fn admit_result(
        &mut self,
        install: &knx_productdb::ProductFileInstall,
        bytes: u64,
    ) -> Result<(), RecoveryPublicationError> {
        self.result.admit_result(install, bytes, self.workspace)
    }
    fn allocated(
        &mut self,
        path: &Path,
        descriptor: &std::fs::File,
    ) -> Result<(), RecoveryPublicationError> {
        if path.parent() != Some(self.workspace.directory()) {
            return Err(RecoveryPublicationError::Refused);
        }
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or(RecoveryPublicationError::Refused)?;
        self.workspace
            .register_database(name, descriptor)
            .map_err(|_| RecoveryPublicationError::Refused)?;
        self.workspace
            .admit_database_interval(name)
            .map_err(|_| RecoveryPublicationError::Refused)
    }
    fn publish(
        &mut self,
        mut copy: ValidatedRecovery<'_>,
    ) -> Result<PublishedRecovery, RecoveryPublicationError> {
        use RecoveryPublicationError as E;
        if copy.path() != self.workspace.directory().join("recovery.part.sqlite") {
            return Err(E::Refused);
        }
        // The callback is after inspected S and D native settlement. Unknown
        // native sidecars refuse; no live/source sidecar can be adopted here.
        self.workspace
            .reconcile_closed_database("recovery.part.sqlite", false)
            .map_err(|_| E::Refused)?;
        self.workspace
            .reconcile_closed_database("rehearsal.sqlite", false)
            .map_err(|_| E::Refused)?;
        let parent = self.catalogue.parent().ok_or(E::Refused)?;
        if self.workspace.catalogue_parent() != Some(parent) {
            return Err(E::Refused);
        }
        let basename = self
            .catalogue
            .file_name()
            .and_then(|n| n.to_str())
            .filter(|n| {
                n.len() <= 400
                    && !n.contains(['/', '\\', ':'])
                    && !n.chars().any(super::dto::is_forbidden_control)
            })
            .ok_or(E::Refused)?;
        let digest = self.upload_sha.to_string();
        for attempt in 0..1000 {
            let suffix = if attempt == 0 {
                String::new()
            } else {
                format!("-{attempt}")
            };
            let name = format!(
                "{basename}.backup-{}-{}{suffix}.sqlite",
                self.seconds,
                &digest[..12]
            );
            match copy.publish_to(parent, &name) {
                Ok(receipt) => {
                    if self
                        .workspace
                        .record_native_publication(&name, copy.descriptor(), true)
                        .is_err()
                    {
                        self.workspace.quarantine();
                        return Err(E::Io);
                    }
                    return Ok(receipt);
                }
                Err(E::AlreadyExists) => continue,
                Err(error) => {
                    if copy.moved() {
                        let _ = self.workspace.record_native_publication(
                            &name,
                            copy.descriptor(),
                            false,
                        );
                        self.workspace.quarantine();
                    }
                    return Err(error);
                }
            }
        }
        Err(E::Io)
    }
}
