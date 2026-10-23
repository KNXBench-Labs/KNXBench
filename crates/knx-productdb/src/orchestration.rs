//! Typed preview/apply orchestration surface for the managed catalogue.
//!
//! This module holds the types the `ManagedCatalogue` orchestration methods
//! return and consume: the operation context, the minimal artifact-owner seam,
//! the controlled preview and the apply settlement. The methods themselves
//! live on `ManagedCatalogue` (they need its private connection/generation/
//! health fields). Construction-only: no production route is wired here. See
//! `docs/contracts/catalogue-jobs/orchestration-increment.md` and
//! `managed-binding.md` §5–§7.

use crate::catalogue_copy::{ArtifactInventory, CopyError, FreshDestination};
use crate::catalogue_stamp::StampError;
use crate::controlled::ControlledPolicy;
use crate::managed_catalogue::WriterHealth;
use crate::operation::{PublicationGate, Stop};
use crate::preview_binding::PreviewBinding;
use crate::preview_identity::{PreviewIdentity, PreviewIdentityError};
use crate::wrapper::ProductFileInstall;
use crate::{PackageError, ProductDbError};

/// The shared operation context for one controlled preview/apply: the
/// persistent cancellation gate and the effective limit policy.
///
/// No allocation, callback or SQL runs here; it is a plain borrow bundle.
#[derive(Debug, Clone, Copy)]
pub(crate) struct OperationContext<'a> {
    pub(crate) gate: &'a PublicationGate,
    pub(crate) policy: ControlledPolicy,
}

/// The minimal seam through which the orchestrator allocates fresh, disposable
/// destinations. The server's `ArtifactRoot` implements it later; this trait
/// returns role-bound Fresh receipts, never bare connections. Its fixed D/B
/// inventory owns every create-new allocation, including failed initialization;
/// validation and consuming errors never become the only owner of a handle.
pub(crate) trait CatalogueArtifacts {
    fn inventory(&mut self) -> &mut ArtifactInventory;
    /// Synthetic native-fault seam; never part of production construction.
    #[cfg(test)]
    fn before_source_open(&mut self, _path: &std::path::Path) {}

    #[cfg(test)]
    fn inspect_pinned_source(&mut self, _source: &crate::Connection) {}

    #[cfg(test)]
    fn after_copy(&mut self, _recovery: bool, _path: &std::path::Path) {}

    #[cfg(test)]
    fn before_rehearsal(&mut self, _receipt: &crate::catalogue_copy::RehearsalDestination) {}

    #[cfg(test)]
    fn before_live_install(&mut self, _writer: &crate::Connection) {}

    /// Construct D through the retained inventory's create-new constructor.
    /// Backup independently enforces empty schema, cookie zero and autocommit.
    fn fresh_rehearsal(&mut self, page_size: i64) -> Result<FreshDestination, OrchestrationError>;

    /// A fresh recovery destination with the given page size. Same freshness
    /// contract as [`CatalogueArtifacts::fresh_rehearsal`].
    fn fresh_recovery(&mut self, page_size: i64) -> Result<FreshDestination, OrchestrationError>;
}

/// One completed controlled preview: the binding the server retains plus the
/// full immutable raw rehearsal result and its exact identity.
#[derive(Debug)]
pub(crate) struct ControlledPreview {
    pub(crate) preview: AdmittedPreviewArtifact,
    pub(crate) binding: PreviewBinding,
}

/// The full immutable raw rehearsal result and its exact identity. The safe
/// public projection (P6) is a later server-side concern; this retains the
/// complete `ProductFileInstall`, never a redacted summary.
#[derive(Debug)]
pub(crate) struct AdmittedPreviewArtifact {
    pub(crate) identity: PreviewIdentity,
    pub(crate) install: ProductFileInstall,
}

/// Inspected cleanup failures are independent of the original preview refusal.
#[derive(Debug)]
pub(crate) enum PreviewCleanupError {
    GrowthRestore(crate::catalogue_growth::GrowthError),
    WriterBaseline,
    WriterObservation(rusqlite::Error),
    WriterRollback(rusqlite::Error),
    WriterNotAutocommit,
    SourceEnd(rusqlite::Error),
    SourceClose(rusqlite::Error),
    RehearsalClose(CopyError),
}

#[derive(Debug)]
pub(crate) struct PreviewFailure {
    pub(crate) primary: Option<OrchestrationError>,
    pub(crate) cleanup: Vec<PreviewCleanupError>,
}

/// Typed failures of the controlled preview/apply orchestration. Never a
/// string and never a partial digest.
#[derive(Debug)]
pub(crate) enum OrchestrationError {
    Growth(crate::catalogue_growth::GrowthError),
    PreviewSettlement(Box<PreviewFailure>),
    SourceSettlement(Box<PreviewFailure>),
    /// The managed handle is not healthy enough for a controlled operation.
    NotHealthy(WriterHealth),
    /// The connection is absent (retired/reopening).
    MissingConnection,
    /// The admitted source, operation or retained binding policies disagree.
    PolicyMismatch,
    /// The admitted source name/length/digest differs from the shown binding.
    SourceMismatch,
    /// The catalogue changed since the shown binding was captured (stale).
    Stale,
    /// A stamp acquisition failed.
    Stamp(StampError),
    /// The W/S/D or W/S/B copy failed.
    Copy(CopyError),
    /// The shared installer refused the source.
    Install(PackageError),
    /// The complete preview identity could not be encoded.
    PreviewIdentity(PreviewIdentityError),
    /// Opening the read-only source failed.
    Open(ProductDbError),
    /// A SQLite error.
    Sqlite(rusqlite::Error),
    /// A successful-looking COMMIT left the connection outside autocommit.
    CommitNotAutocommit,
    /// The operation stopped after accepted cancellation.
    Stop(Stop),
}

impl From<StampError> for OrchestrationError {
    fn from(error: StampError) -> Self {
        OrchestrationError::Stamp(error)
    }
}

impl From<CopyError> for OrchestrationError {
    fn from(error: CopyError) -> Self {
        OrchestrationError::Copy(error)
    }
}

impl From<PackageError> for OrchestrationError {
    fn from(error: PackageError) -> Self {
        OrchestrationError::Install(error)
    }
}

impl From<PreviewIdentityError> for OrchestrationError {
    fn from(error: PreviewIdentityError) -> Self {
        OrchestrationError::PreviewIdentity(error)
    }
}

impl From<ProductDbError> for OrchestrationError {
    fn from(error: ProductDbError) -> Self {
        OrchestrationError::Open(error)
    }
}

impl From<rusqlite::Error> for OrchestrationError {
    fn from(error: rusqlite::Error) -> Self {
        OrchestrationError::Sqlite(error)
    }
}

impl From<Stop> for OrchestrationError {
    fn from(stop: Stop) -> Self {
        OrchestrationError::Stop(stop)
    }
}

impl std::fmt::Display for OrchestrationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OrchestrationError::Growth(error) => {
                write!(f, "catalogue main-growth admission failed: {error:?}")
            }
            OrchestrationError::SourceSettlement(failure) => {
                write!(f, "catalogue source settlement failed: {failure:?}")
            }
            OrchestrationError::PreviewSettlement(failure) => {
                write!(f, "catalogue preview settlement failed: {failure:?}")
            }
            OrchestrationError::NotHealthy(health) => {
                write!(
                    f,
                    "catalogue orchestration refused: owner not healthy: {health:?}"
                )
            }
            OrchestrationError::MissingConnection => {
                write!(f, "catalogue orchestration refused: connection absent")
            }
            OrchestrationError::PolicyMismatch => {
                write!(
                    f,
                    "catalogue orchestration refused: source/policy limits disagree"
                )
            }
            OrchestrationError::SourceMismatch => {
                write!(
                    f,
                    "catalogue orchestration refused: source differs from the shown binding"
                )
            }
            OrchestrationError::Stale => {
                write!(
                    f,
                    "catalogue orchestration refused: catalogue changed since preview"
                )
            }
            OrchestrationError::Stamp(e) => write!(f, "catalogue stamp error: {e}"),
            OrchestrationError::Copy(e) => write!(f, "catalogue copy error: {e}"),
            OrchestrationError::Install(e) => write!(f, "catalogue install error: {e}"),
            OrchestrationError::PreviewIdentity(e) => {
                write!(f, "catalogue preview identity error: {e}")
            }
            OrchestrationError::Open(e) => write!(f, "catalogue read-only open error: {e}"),
            OrchestrationError::Sqlite(e) => write!(f, "catalogue sqlite error: {e}"),
            OrchestrationError::CommitNotAutocommit => {
                write!(f, "catalogue COMMIT left the connection outside autocommit")
            }
            OrchestrationError::Stop(_) => write!(f, "catalogue orchestration stopped"),
        }
    }
}

impl std::error::Error for OrchestrationError {}

/// The commit evidence of one controlled apply. Only a successful COMMIT with
/// autocommit=true witnesses `Committed`; every other classification is
/// conservative.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CommitEvidence {
    /// No live COMMIT was issued, or a proved rollback ended the transaction.
    NotCommitted,
    /// The one live COMMIT returned success and the connection is autocommit.
    Committed,
    /// The COMMIT/rollback outcome could not be safely classified.
    Unknown,
}

/// How the live transaction ended before/after the commit claim.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RollbackDisposition {
    /// No live transaction was ever opened.
    NotApplicable,
    /// An explicit ROLLBACK was issued and succeeded (autocommit=true).
    Explicit,
    /// The transaction ended before COMMIT and the connection returned to
    /// autocommit without an explicit rollback being inspected.
    Automatic,
    /// The transaction state after a failure could not be resolved.
    Unresolved,
}

/// What the caller should do with the owned recovery backup.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ArtifactDisposition {
    /// No recovery backup was created (refusal before copy).
    None,
    /// A recovery backup exists and must be retained (committed/unknown).
    Retain,
    /// A recovery backup exists and may be disposed (proved rollback).
    Dispose,
}

/// The complete settlement of one controlled apply: original failure, commit
/// evidence, rollback disposition, the resulting writer health and the owned
/// artifact disposition. These are independent fields — a committed apply can
/// still carry a later cleanup failure, and a proved rollback keeps its
/// original refusal.
#[derive(Debug)]
pub(crate) struct ControlledApplySettlement {
    pub(crate) failure: Option<OrchestrationError>,
    pub(crate) cleanup: Vec<PreviewCleanupError>,
    pub(crate) commit: CommitEvidence,
    pub(crate) rollback: RollbackDisposition,
    pub(crate) health: WriterHealth,
    pub(crate) artifacts: ArtifactDisposition,
}

impl ControlledApplySettlement {
    /// A settlement for a refusal that happened before any live mutation.
    pub(crate) fn refused(failure: OrchestrationError, health: WriterHealth) -> Self {
        Self {
            failure: Some(failure),
            cleanup: Vec::new(),
            commit: CommitEvidence::NotCommitted,
            rollback: RollbackDisposition::NotApplicable,
            health,
            artifacts: ArtifactDisposition::None,
        }
    }
}

/// One copy, explicit writer close and read-only reopened admission before use.
pub(crate) fn validate_copy(
    artifacts: &mut dyn CatalogueArtifacts,
    fresh: FreshDestination,
    source: &rusqlite::Transaction<'_>,
    gate: &PublicationGate,
    recovery: bool,
) -> Result<crate::catalogue_copy::ValidatedDestination, OrchestrationError> {
    let closed = artifacts.inventory().copy_and_close(
        fresh,
        if recovery {
            crate::catalogue_copy::DestinationRole::Recovery
        } else {
            crate::catalogue_copy::DestinationRole::Rehearsal
        },
        source,
        gate,
    )?;
    #[cfg(test)]
    {
        let path = artifacts.inventory().closed_path(&closed).to_path_buf();
        artifacts.after_copy(recovery, &path);
    }
    Ok(artifacts.inventory().validate(closed, gate)?)
}

#[cfg(test)]
mod tests;
