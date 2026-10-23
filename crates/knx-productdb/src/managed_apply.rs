//! Explicit controlled ProductDB bridge; no arbitrary SQL or installer callback.
//!
//! The caller owns the workspace and receipt outside worker/response unwind.
//! The public apply requires durable recovery publication and complete-result
//! admission before COMMIT claim. Outcomes remain process-local, not a durable
//! job ledger or HTTP authorization. Quiet APIs are unchanged.
use crate::catalogue_copy::{ArtifactInventory, DestinationRole, FreshDestination};
use crate::controlled::ControlledSource;
use crate::managed_catalogue::{ManagedCatalogue, WriterHealth};
use crate::operation::{CommitArbitration, PublicationGate};
use crate::orchestration::{
    CatalogueArtifacts, ControlledApplySettlement, ControlledPreview, OperationContext,
    OrchestrationError,
};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::PathBuf;
pub(crate) mod publication;
pub(crate) mod report;
pub use publication::{
    PublishedRecovery, RecoveryPublicationError, RecoveryPublisher, ValidatedRecovery,
};
pub use report::{ControlledReport, PackageProgram};
/// Actual next-operation copy admission ceiling; not the live growth ceiling.
pub const CATALOGUE_INPUT_BYTE_LIMIT: u64 = crate::catalogue_copy::COPY_PAGE_BYTE_BOUND;

pub use crate::controlled::{ControlledError, ControlledPolicy, NameRule};
pub use crate::orchestration::{
    ApplyAttemptKey, ApplyWitnessTransferError, ArtifactDisposition, CommitEvidence,
    LiveCommitTrace, RetainedApplyWitness, RollbackDisposition,
};

/// An admitted immutable borrow; no caller-supplied digest or connection.
#[derive(Clone, Copy)]
pub struct AdmittedSource<'a>(ControlledSource<'a>);
impl<'a> AdmittedSource<'a> {
    pub fn admit(
        name: &'a str,
        bytes: &'a [u8],
        policy: ControlledPolicy,
    ) -> Result<Self, ControlledError> {
        ControlledSource::admit(name, bytes, policy).map(Self)
    }
    fn policy(self) -> ControlledPolicy {
        if self.0.limits == ControlledPolicy::Large.limits() {
            ControlledPolicy::Large
        } else {
            ControlledPolicy::Standard
        }
    }
}
/// Only actual controlled rehearsal constructs this immutable binding.
#[derive(Debug)]
pub struct PreparedPreview(ControlledPreview);
impl PreparedPreview {
    pub fn report(&self) -> &ControlledReport {
        &self.0.report
    }
    /// Complete immutable raw result, never an authority-bearing connection.
    /// Public artifact projection/admission remains the server's responsibility.
    pub fn install(&self) -> &crate::ProductFileInstall {
        &self.0.preview.install
    }
}

/// Controlled-only diagnostic category; not an HTTP error or quiet API change.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlledFailureKind {
    ReceiptPending,
    ResultAdmission,
    RecoveryPublication,
    CounterExhausted,
    ClaimRefused,
    Configuration,
    Growth,
    AdmissionLimit,
    Cleanup,
    Unavailable,
    PolicyMismatch,
    SourceMismatch,
    Stale,
    Stamp,
    Copy,
    Install,
    LegacyProductDatabase,
    PreviewIdentity,
    Native,
    CommitNotAutocommit,
    Stopped,
    Panicked,
}
fn failure_kind(error: &OrchestrationError) -> ControlledFailureKind {
    use ControlledFailureKind as K;
    use OrchestrationError as E;
    match error {
        E::ApplyWitnessPending => K::ReceiptPending,
        E::ApplyAttemptExhausted => K::CounterExhausted,
        E::CommitClaimRefused => K::ClaimRefused,
        E::RecoveryPublication(_) => K::RecoveryPublication,
        E::ResultAdmission(_) => K::ResultAdmission,
        E::WriterConfiguration(_) => K::Configuration,
        E::Growth(_) => K::Growth,
        E::PreviewSettlement(_) | E::SourceSettlement(_) => K::Cleanup,
        E::NotHealthy(_) | E::MissingConnection => K::Unavailable,
        E::PolicyMismatch => K::PolicyMismatch,
        E::SourceMismatch => K::SourceMismatch,
        E::Stale => K::Stale,
        E::Stamp(_) => K::Stamp,
        E::Copy(crate::catalogue_copy::CopyError::PageCountOverflow) => K::AdmissionLimit,
        E::Copy(crate::catalogue_copy::CopyError::Stop(_)) => K::Stopped,
        E::Copy(_) => K::Copy,
        E::Install(crate::PackageError::Cancelled(_)) | E::Stop(_) => K::Stopped,
        E::Install(
            crate::PackageError::LegacyVd2 { .. }
            | crate::PackageError::LegacyExIm { .. }
            | crate::PackageError::UnsupportedLegacyFormat { .. },
        ) => K::LegacyProductDatabase,
        E::Install(_) => K::Install,
        E::PreviewIdentity(_) => K::PreviewIdentity,
        E::Open(_) | E::Sqlite(_) => K::Native,
        E::CommitNotAutocommit => K::CommitNotAutocommit,
    }
}

/// Exact internal cause retained without expanding quiet error enums.
#[derive(Debug)]
pub struct PreviewError(OrchestrationError, bool);
impl PreviewError {
    /// Captured under the actual managed owner before releasing its mutex.
    pub fn native_settled(&self) -> bool {
        self.1
    }
    pub fn kind(&self) -> ControlledFailureKind {
        failure_kind(&self.0)
    }
}
impl std::fmt::Display for PreviewError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}
impl std::error::Error for PreviewError {}

/// Fixed D/B ownership. No destructor unlinks recovery files. The caller must
/// retain this owner across unwind; durable inventory/publication is separate.
pub struct ApplyWorkspace {
    directory: PathBuf,
    inventory: ArtifactInventory,
    #[cfg(test)]
    panic_after_commit: bool,
}
impl ApplyWorkspace {
    /// Exclusively create a private directory beneath the caller's owned root.
    /// Existing directories are refused, never adopted or repaired.
    pub fn create(directory: PathBuf) -> std::io::Result<Self> {
        use std::os::unix::fs::DirBuilderExt;
        std::fs::DirBuilder::new().mode(0o700).create(&directory)?;
        Ok(Self {
            directory,
            inventory: ArtifactInventory::default(),
            #[cfg(test)]
            panic_after_commit: false,
        })
    }
    /// Adopts an already leased private job directory; allocation remains
    /// create-new and the supplied publisher observes its actual returned FD.
    pub fn in_owned_directory(directory: PathBuf) -> std::io::Result<Self> {
        use std::os::unix::fs::MetadataExt;
        let m = std::fs::symlink_metadata(&directory)?;
        if !m.is_dir() || m.file_type().is_symlink() || m.mode() & 0o777 != 0o700 {
            return Err(std::io::Error::other("private catalogue directory refused"));
        }
        Ok(Self {
            directory,
            inventory: ArtifactInventory::default(),
            #[cfg(test)]
            panic_after_commit: false,
        })
    }
    pub fn native_handles_closed(&self) -> bool {
        self.inventory.native_handles_closed()
    }
    /// Refuses failed native closes. Only after this inspected check may the
    /// outer durable inventory release/remove primary files and exact sidecars.
    pub fn release_closed_ownership(&mut self) -> Result<(), RecoveryPublicationError> {
        if !self.native_handles_closed() {
            return Err(RecoveryPublicationError::Refused);
        }
        self.inventory = ArtifactInventory::default();
        Ok(())
    }

    pub fn has_recovery(&self) -> bool {
        self.inventory.allocated(DestinationRole::Recovery)
    }
    #[cfg(test)]
    pub(crate) fn panic_after_commit_for_test(&mut self) {
        self.panic_after_commit = true;
    }
}
impl CatalogueArtifacts for ApplyWorkspace {
    #[cfg(test)]
    fn after_live_commit(&mut self, _writer: &crate::Connection) {
        if self.panic_after_commit {
            panic!("workspace-post-commit-panic");
        }
    }
    fn inventory(&mut self) -> &mut ArtifactInventory {
        &mut self.inventory
    }
    fn fresh_rehearsal(&mut self, page_size: i64) -> Result<FreshDestination, OrchestrationError> {
        Ok(self.inventory.create_new(
            DestinationRole::Rehearsal,
            self.directory.join("rehearsal.sqlite"),
            page_size,
        )?)
    }
    fn fresh_recovery(&mut self, page_size: i64) -> Result<FreshDestination, OrchestrationError> {
        Ok(self.inventory.create_new(
            DestinationRole::Recovery,
            self.directory.join("recovery.part.sqlite"),
            page_size,
        )?)
    }
}

/// One caller-owned pending handoff, deliberately not Clone. Returning/dropping
/// a result does not consume it; only exact-key transfer moves the evidence.
#[derive(Debug, Default)]
pub struct ApplyReceipt {
    witness: Option<RetainedApplyWitness>,
}
impl ApplyReceipt {
    pub fn witness(&self) -> Option<RetainedApplyWitness> {
        self.witness
    }
    pub fn take(
        &mut self,
        key: ApplyAttemptKey,
    ) -> Result<RetainedApplyWitness, ApplyWitnessTransferError> {
        match self.witness {
            None => Err(ApplyWitnessTransferError::Absent),
            Some(w) if w.key != key => Err(ApplyWitnessTransferError::KeyMismatch),
            Some(w) => {
                self.witness = None;
                Ok(w)
            }
        }
    }
}
impl RetainedApplyWitness {
    pub fn key(&self) -> ApplyAttemptKey {
        self.key
    }
    pub fn trace(&self) -> LiveCommitTrace {
        self.trace
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApplyBridgeError {
    ReceiptPending,
    WitnessHandoff,
}

/// Original ordinary failure/cleanup remain retained, independently of the
/// fixed outcome projection. Panic does not become clean cancellation.
#[derive(Debug)]
pub struct ApplyExecution {
    native_settled: bool,
    settlement: Option<ControlledApplySettlement>,
    health: WriterHealth,
    commit: CommitEvidence,
    artifacts: ArtifactDisposition,
}
impl ApplyExecution {
    /// Per-attempt inspected W/S disposition, independent of later owner users.
    pub fn native_settled(&self) -> bool {
        self.native_settled
    }
    pub fn failure_kind(&self) -> Option<ControlledFailureKind> {
        match &self.settlement {
            None => Some(ControlledFailureKind::Panicked),
            Some(settlement) => settlement.failure.as_ref().map(failure_kind),
        }
    }
    pub fn panicked(&self) -> bool {
        self.settlement.is_none()
    }
    pub fn commit(&self) -> CommitEvidence {
        self.commit
    }
    pub fn health(&self) -> WriterHealth {
        self.health
    }
    pub fn failed(&self) -> bool {
        self.settlement.as_ref().is_none_or(|s| s.failure.is_some())
    }
    pub fn cleanup_failed(&self) -> bool {
        self.health != WriterHealth::Healthy
            || self
                .settlement
                .as_ref()
                .is_none_or(|s| !s.cleanup.is_empty())
    }
    pub fn rollback(&self) -> RollbackDisposition {
        self.settlement
            .as_ref()
            .map_or(RollbackDisposition::Unresolved, |s| s.rollback)
    }
    pub fn artifacts(&self) -> ArtifactDisposition {
        self.artifacts
    }
}

impl ManagedCatalogue {
    /// Inspect retained native source/W disposition without repairing health.
    pub fn controlled_native_settled(&self) -> bool {
        self.unresolved_preview_source_is_absent()
    }
    pub fn prepare_controlled(
        &mut self,
        source: AdmittedSource<'_>,
        gate: &PublicationGate,
        workspace: &mut ApplyWorkspace,
    ) -> Result<PreparedPreview, PreviewError> {
        self.preview_current_controlled(
            source.0,
            &OperationContext {
                gate,
                policy: source.policy(),
            },
            workspace,
        )
        .map(PreparedPreview)
        .map_err(|error| PreviewError(error, self.controlled_native_settled()))
    }
    pub fn prepare_controlled_owned(
        &mut self,
        source: AdmittedSource<'_>,
        gate: &PublicationGate,
        workspace: &mut ApplyWorkspace,
        publisher: &mut dyn RecoveryPublisher,
    ) -> Result<PreparedPreview, PreviewError> {
        let mut observed = publication::ObservedWorkspace {
            workspace,
            publisher,
        };
        self.preview_current_controlled(
            source.0,
            &OperationContext {
                gate,
                policy: source.policy(),
            },
            &mut observed,
        )
        .map(PreparedPreview)
        .map_err(|error| PreviewError(error, self.controlled_native_settled()))
    }

    /// Catch while the managed owner is still exclusively borrowed, then move
    /// the exact keyed trace outside the caller's mutex before any projection.
    /// No poison recovery, SQLite read or health repair is used for handoff.
    pub fn execute_controlled(
        &mut self,
        source: AdmittedSource<'_>,
        shown: &PreparedPreview,
        control: &dyn CommitArbitration,
        workspace: &mut ApplyWorkspace,
        receipt: &mut ApplyReceipt,
        publisher: &mut dyn RecoveryPublisher,
    ) -> Result<ApplyExecution, ApplyBridgeError> {
        if receipt.witness.is_some() || self.retained_apply_witness().is_some() {
            return Err(ApplyBridgeError::ReceiptPending);
        }
        let outcome = catch_unwind(AssertUnwindSafe(|| {
            let mut observed = publication::ObservedWorkspace {
                workspace,
                publisher,
            };
            self.apply_current_controlled_with_claim(
                source.0,
                &shown.0.binding,
                &OperationContext {
                    gate: control.gate(),
                    policy: source.policy(),
                },
                &mut observed,
                control,
            )
        }));
        if let Some(witness) = self.retained_apply_witness() {
            // Exact key from this admitted attempt, never a prior/rejected call.
            receipt.witness = Some(
                self.take_apply_witness(witness.key)
                    .map_err(|_| ApplyBridgeError::WitnessHandoff)?,
            );
        }
        let settlement = outcome.ok();
        let health = self.health_snapshot();
        let commit = if receipt
            .witness
            .is_some_and(|w| w.trace == LiveCommitTrace::LiveCommitReturnedOk { autocommit: true })
        {
            CommitEvidence::Committed
        } else {
            settlement
                .as_ref()
                .map_or(CommitEvidence::Unknown, |s| s.commit)
        };
        let artifacts = settlement.as_ref().map_or_else(
            || {
                if workspace.has_recovery() {
                    ArtifactDisposition::Retain
                } else {
                    ArtifactDisposition::None
                }
            },
            |settlement| settlement.artifacts,
        );
        Ok(ApplyExecution {
            native_settled: self.controlled_native_settled(),
            settlement,
            health,
            commit,
            artifacts,
        })
    }
}
