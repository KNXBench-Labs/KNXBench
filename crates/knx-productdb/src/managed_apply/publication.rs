//! Required artifact adapter: no no-op/default publication acknowledgement.
use super::*;

#[cfg(test)]
pub(crate) struct NativeTestPublisher;
#[cfg(test)]
impl RecoveryPublisher for NativeTestPublisher {
    fn admit_result(
        &mut self,
        _: &crate::ProductFileInstall,
        _: u64,
    ) -> Result<(), RecoveryPublicationError> {
        Ok(())
    }
    fn allocated(&mut self, _: &Path, _: &File) -> Result<(), RecoveryPublicationError> {
        Ok(())
    }
    fn publish(
        &mut self,
        mut copy: ValidatedRecovery<'_>,
    ) -> Result<PublishedRecovery, RecoveryPublicationError> {
        let parent = copy.path().parent().unwrap().to_path_buf();
        copy.publish_to(&parent, "published-recovery.sqlite")
    }
}

use std::fs::File;
use std::path::Path;

pub use crate::catalogue_copy::{PublishedRecovery, RecoveryPublicationError, ValidatedRecovery};

pub trait RecoveryPublisher {
    /// Complete actual live result, before the same gate's COMMIT claim.
    fn admit_result(
        &mut self,
        install: &crate::ProductFileInstall,
        catalogue_page_bytes: u64,
    ) -> Result<(), RecoveryPublicationError>;
    /// Called on the actual create-new descriptor BEFORE SQLite opens it.
    fn allocated(&mut self, path: &Path, descriptor: &File)
        -> Result<(), RecoveryPublicationError>;
    /// Must return the opaque receipt minted by this copy's publish_to method.
    /// A private B or a caller boolean cannot produce that acknowledgement.
    fn publish(
        &mut self,
        recovery: ValidatedRecovery<'_>,
    ) -> Result<PublishedRecovery, RecoveryPublicationError>;
}

pub(super) struct ObservedWorkspace<'a> {
    pub workspace: &'a mut ApplyWorkspace,
    pub publisher: &'a mut dyn RecoveryPublisher,
}
impl CatalogueArtifacts for ObservedWorkspace<'_> {
    fn admit_result(
        &mut self,
        install: &crate::ProductFileInstall,
        bytes: u64,
    ) -> Result<(), OrchestrationError> {
        self.publisher
            .admit_result(install, bytes)
            .map_err(OrchestrationError::ResultAdmission)
    }
    fn inventory(&mut self) -> &mut ArtifactInventory {
        &mut self.workspace.inventory
    }
    fn fresh_rehearsal(&mut self, page_size: i64) -> Result<FreshDestination, OrchestrationError> {
        let publisher = &mut self.publisher;
        Ok(self.workspace.inventory.create_new_observed(
            DestinationRole::Rehearsal,
            self.workspace.directory.join("rehearsal.sqlite"),
            page_size,
            &mut |path, file| publisher.allocated(path, file),
        )?)
    }
    fn fresh_recovery(&mut self, page_size: i64) -> Result<FreshDestination, OrchestrationError> {
        let publisher = &mut self.publisher;
        Ok(self.workspace.inventory.create_new_observed(
            DestinationRole::Recovery,
            self.workspace.directory.join("recovery.part.sqlite"),
            page_size,
            &mut |path, file| publisher.allocated(path, file),
        )?)
    }
    fn publish_recovery(&mut self) -> Result<(), OrchestrationError> {
        let copy = self.workspace.inventory.validated_recovery()?;
        let receipt = self
            .publisher
            .publish(copy)
            .map_err(OrchestrationError::RecoveryPublication)?;
        self.workspace.inventory.require_published(receipt)?;
        Ok(())
    }
    #[cfg(test)]
    fn after_live_commit(&mut self, writer: &crate::Connection) {
        self.workspace.after_live_commit(writer);
    }
}
