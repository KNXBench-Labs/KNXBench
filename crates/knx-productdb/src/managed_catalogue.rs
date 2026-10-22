//! One managed catalogue owner: incarnation, generation and health.
//!
//! Construction-only: no production route is wired here. The server-side
//! `Mutex<ManagedCatalogue>` wrapper, the full witness (§4) and the W/S/D copy
//! recipe (§6) are later increments. See
//! `docs/contracts/catalogue-jobs/managed-binding.md`.

use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};

use rusqlite::{Connection, DropBehavior, TransactionBehavior};

use crate::operation::{Cancellation, PublicationGate};

/// A fresh, never-reused 128-bit value identifying one adopted managed handle.
/// Distinct from the server incarnation and any job identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CatalogueIncarnation([u8; 16]);

impl CatalogueIncarnation {
    fn fresh() -> Self {
        let mut bytes = [0u8; 16];
        // A 128-bit random value is collision-negligible; it is never recycled
        // after a retirement because every adoption mints a new one.
        getrandom::fill(&mut bytes).expect("getrandom failure for catalogue incarnation");
        Self(bytes)
    }
}

/// The single mutable health of one managed handle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WriterHealth {
    Healthy,
    /// Lock is owned; the guard has not yet proved safe settlement.
    Mutating,
    /// Deliberate close/restore/reopen in progress.
    Retired(RetireReason),
    /// No reads or writes through this handle.
    Quarantined(QuarantineReason),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetireReason {
    Close,
    Reopen,
    Restore,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuarantineReason {
    Panic,
    UncertainSettlement,
    Poisoned,
    IdentityMismatch,
}

/// `dev/ino/nlink/len/mtime+ns/ctime+ns` of the live file; no atime.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct FileStamp {
    dev: u64,
    ino: u64,
    nlink: u64,
    len: u64,
    mtime_ns: i128,
    ctime_ns: i128,
}

impl FileStamp {
    fn from_path(path: &Path) -> Option<Self> {
        let meta = std::fs::metadata(path).ok()?;
        Some(Self {
            dev: meta.dev(),
            ino: meta.ino(),
            nlink: meta.nlink(),
            len: meta.len(),
            mtime_ns: i128::from(meta.mtime()) * 1_000_000_000 + i128::from(meta.mtime_nsec()),
            ctime_ns: i128::from(meta.ctime()) * 1_000_000_000 + i128::from(meta.ctime_nsec()),
        })
    }
}

/// Persistent bookkeeping on the one managed handle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct WriterObservation {
    data_version: i64,
    total_changes: u64,
    file: Option<FileStamp>,
}

/// The admitted absolute/canonical pathname (privately), parent directory
/// identity, device/inode/link count and the selected default-VFS choice.
/// Retained for the later identity/ABA checks; only `canonical` is read today.
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub(crate) struct ManagedFileIdentity {
    canonical: PathBuf,
    parent: PathBuf,
    dev: u64,
    ino: u64,
    nlink: u64,
}

impl ManagedFileIdentity {
    fn from_path(path: &Path) -> Option<Self> {
        let canonical = std::fs::canonicalize(path).ok()?;
        let parent = canonical
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_default();
        let meta = std::fs::metadata(&canonical).ok()?;
        Some(Self {
            canonical,
            parent,
            dev: meta.dev(),
            ino: meta.ino(),
            nlink: meta.nlink(),
        })
    }
}

/// Closed set of legacy server mutators admitted through the managed owner.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LegacyMutationKind {
    DirectPackage,
    LegacyPublish,
    PathUpdate,
    PlacementCleanup,
    BackupDelete,
}

/// A hint the mutation guard independently checks against native/health state;
/// never permission to claim a failed COMMIT rolled back.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LegacySettlementHint {
    SuccessfulReturn,
    PreCommitSemanticRefusal,
    Uncertain,
}

#[derive(Debug)]
pub enum OwnerError {
    NotHealthy(WriterHealth),
    GenerationOverflow,
    ReadDuringTransaction,
    MissingConnection,
    Sqlite(rusqlite::Error),
}

impl From<rusqlite::Error> for OwnerError {
    fn from(e: rusqlite::Error) -> Self {
        OwnerError::Sqlite(e)
    }
}

impl std::fmt::Display for OwnerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OwnerError::NotHealthy(health) => write!(f, "catalogue owner not healthy: {health:?}"),
            OwnerError::GenerationOverflow => write!(f, "catalogue generation overflow"),
            OwnerError::ReadDuringTransaction => {
                write!(f, "catalogue read refused during an active transaction")
            }
            OwnerError::MissingConnection => write!(f, "catalogue connection is absent"),
            OwnerError::Sqlite(e) => write!(f, "catalogue sqlite error: {e}"),
        }
    }
}

impl std::error::Error for OwnerError {}

#[derive(Debug)]
pub enum OwnerRestoreError {
    NotHealthy(WriterHealth),
    GenerationOverflow,
    /// The file offered for a restore is not a usable product database: a
    /// semantic refusal the caller can surface as `invalidBackup` (422), not
    /// an internal failure.
    InvalidBackup(String),
    Restore(String),
    Observation(String),
}

impl std::fmt::Display for OwnerRestoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OwnerRestoreError::NotHealthy(health) => {
                write!(f, "catalogue owner not healthy: {health:?}")
            }
            OwnerRestoreError::GenerationOverflow => write!(f, "catalogue generation overflow"),
            OwnerRestoreError::InvalidBackup(why) => {
                write!(
                    f,
                    "catalogue restore failed: not a usable product database backup: {why}"
                )
            }
            OwnerRestoreError::Restore(e) => write!(f, "catalogue restore failed: {e}"),
            OwnerRestoreError::Observation(why) => {
                write!(f, "catalogue observation failed: {why}")
            }
        }
    }
}

impl std::error::Error for OwnerRestoreError {}

/// Quarantines the handle on unwind unless the guard was disarmed by a proven
/// clean settlement.
struct MutationGuard<'a> {
    health: &'a mut WriterHealth,
    armed: bool,
}

impl MutationGuard<'_> {
    fn disarm(mut self) {
        self.armed = false;
        *self.health = WriterHealth::Healthy;
    }
}

impl Drop for MutationGuard<'_> {
    fn drop(&mut self) {
        if self.armed {
            let reason = if std::thread::panicking() {
                QuarantineReason::Panic
            } else {
                QuarantineReason::UncertainSettlement
            };
            *self.health = WriterHealth::Quarantined(reason);
        }
    }
}

/// One protected managed state. `conn` is `None` while retired/reopening.
/// Never store a `rusqlite::Transaction` here (it would borrow its own
/// connection); orchestration borrows the connection disjointly instead.
pub struct ManagedCatalogue {
    conn: Option<Connection>,
    incarnation: CatalogueIncarnation,
    generation: u64,
    health: WriterHealth,
    #[allow(dead_code)] // retained for the later identity/ABA checks
    identity: Option<ManagedFileIdentity>,
    observation: Option<WriterObservation>,
}

impl ManagedCatalogue {
    /// Adopts an already-opened, already-migrated connection. Takes a real
    /// main read first so the first observation is a real observation.
    pub fn adopt_opened(conn: Connection, configured_path: PathBuf) -> Result<Self, OwnerError> {
        let observation = Self::observe(&conn, &configured_path)?;
        let identity = ManagedFileIdentity::from_path(&configured_path);
        Ok(Self {
            conn: Some(conn),
            incarnation: CatalogueIncarnation::fresh(),
            generation: 0,
            health: WriterHealth::Healthy,
            identity,
            observation: Some(observation),
        })
    }

    /// A read lease scoped to a lock; `&Connection` never escapes the closure.
    /// Refuses a non-Healthy handle or an active transaction, and never
    /// mutates observation counters.
    pub fn with_read<T, E>(
        &self,
        read: impl FnOnce(&Connection) -> Result<T, E>,
    ) -> Result<Result<T, E>, OwnerError> {
        match self.health {
            WriterHealth::Healthy => {}
            other => return Err(OwnerError::NotHealthy(other)),
        }
        let conn = self.conn.as_ref().ok_or(OwnerError::MissingConnection)?;
        if !conn.is_autocommit() {
            return Err(OwnerError::ReadDuringTransaction);
        }
        Ok(read(conn))
    }

    /// A managed mutation guard around one legacy mutator. Checked generation
    /// increment before the body, `Mutating` set, quarantine-on-unwind; a
    /// clean return refreshes the observation and restores `Healthy`. Failure
    /// never rolls generation back.
    pub fn with_legacy_mutation<T, E>(
        &mut self,
        _kind: LegacyMutationKind,
        work: impl FnOnce(&mut Connection) -> Result<T, E>,
        classify: impl FnOnce(&Result<T, E>) -> LegacySettlementHint,
    ) -> Result<Result<T, E>, OwnerError> {
        if self.health != WriterHealth::Healthy {
            return Err(OwnerError::NotHealthy(self.health));
        }
        self.generation = self
            .generation
            .checked_add(1)
            .ok_or(OwnerError::GenerationOverflow)?;
        self.health = WriterHealth::Mutating;

        let outcome = {
            let conn = self.conn.as_mut().ok_or(OwnerError::MissingConnection)?;
            let guard = MutationGuard {
                health: &mut self.health,
                armed: true,
            };
            let result = work(conn);
            let hint = classify(&result);
            let autocommit = conn.is_autocommit();
            let clean = matches!(
                hint,
                LegacySettlementHint::SuccessfulReturn
                    | LegacySettlementHint::PreCommitSemanticRefusal
            ) && autocommit;
            if clean {
                guard.disarm();
            } else {
                drop(guard);
            }
            result
        };

        if self.health == WriterHealth::Healthy {
            self.refresh_observation()?;
        }
        Ok(outcome)
    }

    /// The dedicated restore adapter: copy the validated backup over the live
    /// file while the old connection is still open (quiescent), then retire
    /// and reopen with migration and a fresh incarnation only on success. A
    /// validation refusal leaves the old connection healthy; a mid-copy or
    /// reopen failure quarantines it, never silently usable.
    pub fn restore_legacy(
        &mut self,
        live_path: &Path,
        backup_path: &Path,
    ) -> Result<(), OwnerRestoreError> {
        if self.health != WriterHealth::Healthy {
            return Err(OwnerRestoreError::NotHealthy(self.health));
        }
        self.generation = self
            .generation
            .checked_add(1)
            .ok_or(OwnerRestoreError::GenerationOverflow)?;

        // Copy with the old connection still open. An invalid backup is a
        // semantic refusal that leaves the live file and the old connection
        // untouched; a mid-copy failure is uncertain and quarantines.
        if let Err(error) = crate::update::restore_product_db_backup(live_path, backup_path) {
            return Err(match error {
                crate::update::UpdateError::InvalidBackup(why) => {
                    OwnerRestoreError::InvalidBackup(why)
                }
                other => {
                    self.health = WriterHealth::Quarantined(QuarantineReason::UncertainSettlement);
                    OwnerRestoreError::Restore(other.to_string())
                }
            });
        }

        // The live file is replaced: retire the old connection and reopen with
        // migration, adopting a fresh incarnation only on success.
        self.health = WriterHealth::Retired(RetireReason::Restore);
        self.conn = None;
        match crate::migration::open_and_migrate(live_path) {
            Ok(conn) => {
                self.conn = Some(conn);
                self.incarnation = CatalogueIncarnation::fresh();
                self.health = WriterHealth::Healthy;
                if let Err(e) = self.refresh_observation() {
                    self.health = WriterHealth::Quarantined(QuarantineReason::UncertainSettlement);
                    return Err(OwnerRestoreError::Observation(e.to_string()));
                }
                Ok(())
            }
            Err(e) => {
                self.health = WriterHealth::Quarantined(QuarantineReason::UncertainSettlement);
                Err(OwnerRestoreError::Restore(e.to_string()))
            }
        }
    }

    /// Captures an immutable catalogue stamp under one read-only W IMMEDIATE
    /// reservation. Increments no generation and changes no rows: the
    /// transaction is explicitly rolled back and the connection is verified
    /// autocommit before the stamp is returned. A change underneath the
    /// snapshot refuses instead of resetting the baseline.
    #[allow(dead_code)] // used by the future preview/apply orchestration
    pub(crate) fn capture_stamp(
        &mut self,
        gate: &PublicationGate,
    ) -> Result<crate::catalogue_stamp::CatalogueStamp, crate::catalogue_stamp::StampError> {
        use crate::catalogue_stamp::{CatalogueStamp, StampError};

        if self.health != WriterHealth::Healthy {
            return Err(StampError::NotHealthy(self.health));
        }
        let incarnation = self.incarnation;
        let generation = self.generation;
        let file = self
            .identity
            .as_ref()
            .and_then(|id| FileStamp::from_path(&id.canonical));

        let conn = self.conn.as_mut().ok_or(StampError::MissingConnection)?;
        let mut tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        tx.set_drop_behavior(DropBehavior::Ignore);

        // Capture the snapshot under the reservation. Any refusal still falls
        // through to the explicit rollback below, so a typed Stop or witness
        // error never leaves the connection inside a transaction.
        let outcome =
            (|| -> Result<(i64, i64, crate::catalogue_schema::CatalogueWitness), StampError> {
                // A real main read pins the reservation to an actual snapshot;
                // BEGIN alone does not.
                let _: i64 =
                    tx.query_row("SELECT count(*) FROM sqlite_schema", [], |r| r.get(0))?;
                let before: i64 = tx.query_row("PRAGMA data_version", [], |r| r.get(0))?;
                let witness = crate::catalogue_schema::witness_current(&tx, gate)?;
                let after: i64 = tx.query_row("PRAGMA data_version", [], |r| r.get(0))?;
                Ok((before, after, witness))
            })();

        // End the unchanged reservation with one explicit rollback on every
        // path (success, witness stop and observation error alike).
        let rollback = tx.execute_batch("ROLLBACK");
        drop(tx);
        rollback?;

        let (before, after, witness) = outcome?;
        if before != after {
            return Err(StampError::Observation(
                "data_version changed during the reservation",
            ));
        }
        if !conn.is_autocommit() {
            return Err(StampError::Observation(
                "reservation did not roll back to autocommit",
            ));
        }

        let total_changes = conn.total_changes();
        Ok(CatalogueStamp {
            incarnation,
            generation,
            data_version: before,
            total_changes,
            file,
            witness,
        })
    }

    fn refresh_observation(&mut self) -> Result<(), OwnerError> {
        let conn = self.conn.as_ref().ok_or(OwnerError::MissingConnection)?;
        let path = self.identity.as_ref().map(|id| id.canonical.clone());
        let observation = Self::observe(conn, path.as_deref().unwrap_or(Path::new("")))?;
        self.observation = Some(observation);
        Ok(())
    }

    fn observe(conn: &Connection, path: &Path) -> Result<WriterObservation, OwnerError> {
        let mut statement = conn.prepare("PRAGMA data_version")?;
        let mut rows = statement.query([])?;
        let row = rows
            .next()?
            .ok_or(OwnerError::Sqlite(rusqlite::Error::QueryReturnedNoRows))?;
        let data_version: i64 = row.get(0)?;
        let total_changes = conn.total_changes();
        let file = FileStamp::from_path(path);
        Ok(WriterObservation {
            data_version,
            total_changes,
            file,
        })
    }
}

/// Controlled preview/apply orchestration (managed-binding §5–§7).
impl ManagedCatalogue {
    /// A read-only rehearsal that changes no live catalogue bytes: captures
    /// the original stamp, copies W→S→D, installs on the disposable D and
    /// returns the retained binding plus the full immutable rehearsal result.
    #[allow(dead_code)] // called by the future server worker
    pub(crate) fn preview_current_controlled(
        &mut self,
        source: crate::controlled::ControlledSource<'_>,
        op: &crate::orchestration::OperationContext<'_>,
        artifacts: &mut dyn crate::orchestration::CatalogueArtifacts,
    ) -> Result<crate::orchestration::ControlledPreview, crate::orchestration::OrchestrationError>
    {
        use crate::orchestration::{
            AdmittedPreviewArtifact, ControlledPreview, OrchestrationError,
        };

        if self.health != WriterHealth::Healthy {
            return Err(OrchestrationError::NotHealthy(self.health));
        }
        op.gate.checkpoint()?;
        if source.limits != op.policy.limits() {
            return Err(OrchestrationError::PolicyMismatch);
        }

        // The original acquisition stamp (read-only W reservation + rollback).
        let stamp = self.capture_stamp(op.gate)?;

        // A distinct read-only source pinned by a real main read.
        let path = self
            .identity
            .as_ref()
            .map(|id| id.canonical.clone())
            .ok_or(OrchestrationError::MissingConnection)?;
        let source_ro = crate::migration::open_read_only(&path)?;
        source_ro.conn.execute_batch("BEGIN")?;
        let _: i64 = source_ro
            .conn
            .query_row("SELECT count(*) FROM sqlite_schema", [], |r| r.get(0))?;

        // Fresh equal-page-size rehearsal destination copied from S.
        let page_size: i64 = source_ro
            .conn
            .query_row("PRAGMA page_size", [], |r| r.get(0))?;
        let mut rehearsal = artifacts.fresh_rehearsal(page_size)?;
        let copy =
            crate::catalogue_copy::copy_pinned_current(&source_ro.conn, &mut rehearsal, op.gate);

        // End S's read transaction and close it regardless of the copy outcome.
        let _ = source_ro.conn.execute_batch("ROLLBACK");
        drop(source_ro);
        copy?;

        // Rehearsal install + complete preview identity on the disposable D.
        let install = crate::wrapper::install_admitted_product_file(
            &rehearsal,
            source.name,
            source.bytes,
            source.limits,
            Cancellation::Controlled(op.gate),
        )?;
        let identity = crate::preview_identity::preview_identity(&install)?;
        drop(rehearsal);

        let binding = crate::preview_binding::PreviewBinding::bind(
            stamp,
            crate::controlled::SourceIdentity::from_source(&source),
            crate::controlled::PolicyIdentity::from_policy(op.policy),
            identity,
        );

        Ok(ControlledPreview {
            preview: AdmittedPreviewArtifact { identity, install },
            binding,
        })
    }

    /// The live, stale-checked, commit-gated install. Never panics on a
    /// business refusal: it always returns a [`ControlledApplySettlement`] the
    /// caller inspects to decide recovery-backup retention and disposition.
    #[allow(dead_code)] // called by the future server worker
    pub(crate) fn apply_current_controlled(
        &mut self,
        source: crate::controlled::ControlledSource<'_>,
        shown: &crate::preview_binding::PreviewBinding,
        op: &crate::orchestration::OperationContext<'_>,
        artifacts: &mut dyn crate::orchestration::CatalogueArtifacts,
    ) -> crate::orchestration::ControlledApplySettlement {
        use crate::orchestration::{
            ArtifactDisposition, CommitEvidence, ControlledApplySettlement, OrchestrationError,
            RollbackDisposition,
        };

        // ---- Phase 1: preconditions before any live work ----
        if self.health != WriterHealth::Healthy {
            return ControlledApplySettlement::refused(
                OrchestrationError::NotHealthy(self.health),
                self.health,
            );
        }
        if let Err(stop) = op.gate.checkpoint() {
            return ControlledApplySettlement::refused(OrchestrationError::Stop(stop), self.health);
        }
        if source.limits != op.policy.limits() {
            return ControlledApplySettlement::refused(
                OrchestrationError::PolicyMismatch,
                self.health,
            );
        }
        if crate::controlled::SourceIdentity::from_source(&source) != shown.source {
            return ControlledApplySettlement::refused(
                OrchestrationError::SourceMismatch,
                self.health,
            );
        }

        // ---- Phase 2: stale check (no live write) ----
        let fresh_stamp = match self.capture_stamp(op.gate) {
            Ok(stamp) => stamp,
            Err(error) => {
                return ControlledApplySettlement::refused(error.into(), self.health);
            }
        };
        if fresh_stamp != shown.catalogue {
            return ControlledApplySettlement::refused(OrchestrationError::Stale, self.health);
        }

        // ---- Phase 3: open S, copy D and B (still no live write) ----
        let path = match self.identity.as_ref() {
            Some(id) => id.canonical.clone(),
            None => {
                return ControlledApplySettlement::refused(
                    OrchestrationError::MissingConnection,
                    self.health,
                );
            }
        };
        let source_ro = match crate::migration::open_read_only(&path) {
            Ok(source_ro) => source_ro,
            Err(error) => {
                return ControlledApplySettlement::refused(error.into(), self.health);
            }
        };
        if let Err(error) = source_ro.conn.execute_batch("BEGIN") {
            drop(source_ro);
            return ControlledApplySettlement::refused(error.into(), self.health);
        }
        if let Err(error) =
            source_ro
                .conn
                .query_row("SELECT count(*) FROM sqlite_schema", [], |r| {
                    r.get::<_, i64>(0)
                })
        {
            drop(source_ro);
            return ControlledApplySettlement::refused(
                OrchestrationError::Sqlite(error),
                self.health,
            );
        }
        let page_size: i64 = match source_ro
            .conn
            .query_row("PRAGMA page_size", [], |r| r.get(0))
        {
            Ok(value) => value,
            Err(error) => {
                drop(source_ro);
                return ControlledApplySettlement::refused(
                    OrchestrationError::Sqlite(error),
                    self.health,
                );
            }
        };

        let mut rehearsal = match artifacts.fresh_rehearsal(page_size) {
            Ok(destination) => destination,
            Err(error) => {
                drop(source_ro);
                return ControlledApplySettlement::refused(error, self.health);
            }
        };
        if let Err(error) =
            crate::catalogue_copy::copy_pinned_current(&source_ro.conn, &mut rehearsal, op.gate)
        {
            drop(rehearsal);
            drop(source_ro);
            return ControlledApplySettlement::refused(error.into(), self.health);
        }
        drop(rehearsal);
        let mut recovery = match artifacts.fresh_recovery(page_size) {
            Ok(destination) => destination,
            Err(error) => {
                drop(source_ro);
                return ControlledApplySettlement::refused(error, self.health);
            }
        };
        let recovery_copy =
            crate::catalogue_copy::copy_pinned_current(&source_ro.conn, &mut recovery, op.gate);
        // Close S before any W DML: S's shared read lock must be released
        // before the W IMMEDIATE reservation below.
        drop(source_ro);
        drop(recovery);
        if let Err(error) = recovery_copy {
            // The recovery copy did not complete: no published backup exists
            // and we refuse before any live write.
            return ControlledApplySettlement::refused(error.into(), self.health);
        }

        // ---- Phase 4: live W mutation under the managed guard ----
        // Checked generation increment before any live write; overflow closes
        // admission rather than wrapping.
        match self.generation.checked_add(1) {
            Some(next) => self.generation = next,
            None => {
                self.health = WriterHealth::Quarantined(QuarantineReason::UncertainSettlement);
                return ControlledApplySettlement {
                    failure: Some(OrchestrationError::NotHealthy(self.health)),
                    commit: CommitEvidence::Unknown,
                    rollback: RollbackDisposition::NotApplicable,
                    health: self.health,
                    artifacts: ArtifactDisposition::Retain,
                };
            }
        }
        self.health = WriterHealth::Mutating;

        let (commit, rollback, failure, clean) = {
            let conn = match self.conn.as_mut() {
                Some(conn) => conn,
                None => {
                    return ControlledApplySettlement {
                        failure: Some(OrchestrationError::MissingConnection),
                        commit: CommitEvidence::Unknown,
                        rollback: RollbackDisposition::NotApplicable,
                        health: self.health,
                        artifacts: ArtifactDisposition::Retain,
                    };
                }
            };
            let guard = MutationGuard {
                health: &mut self.health,
                armed: true,
            };
            let mut tx = match conn.transaction_with_behavior(TransactionBehavior::Immediate) {
                Ok(tx) => tx,
                Err(error) => {
                    drop(guard);
                    return ControlledApplySettlement {
                        failure: Some(OrchestrationError::Sqlite(error)),
                        commit: CommitEvidence::Unknown,
                        rollback: RollbackDisposition::NotApplicable,
                        health: self.health,
                        artifacts: ArtifactDisposition::Retain,
                    };
                }
            };
            tx.set_drop_behavior(DropBehavior::Ignore);

            let identity = match crate::wrapper::install_admitted_product_file(
                &tx,
                source.name,
                source.bytes,
                source.limits,
                Cancellation::Controlled(op.gate),
            ) {
                Ok(install) => match crate::preview_identity::preview_identity(&install) {
                    Ok(identity) => identity,
                    Err(error) => {
                        let clean = rollback_and_check(&tx);
                        drop(tx);
                        if clean {
                            guard.disarm();
                        } else {
                            drop(guard);
                        }
                        return ControlledApplySettlement {
                            failure: Some(OrchestrationError::PreviewIdentity(error)),
                            commit: CommitEvidence::NotCommitted,
                            rollback: RollbackDisposition::Explicit,
                            health: self.health,
                            artifacts: ArtifactDisposition::Dispose,
                        };
                    }
                },
                Err(error) => {
                    let clean = rollback_and_check(&tx);
                    drop(tx);
                    if clean {
                        guard.disarm();
                    } else {
                        drop(guard);
                    }
                    return ControlledApplySettlement {
                        failure: Some(OrchestrationError::Install(error)),
                        commit: CommitEvidence::NotCommitted,
                        rollback: RollbackDisposition::Explicit,
                        health: self.health,
                        artifacts: ArtifactDisposition::Dispose,
                    };
                }
            };

            // A preview/report disagreement is a refusal: roll back, never live
            // commit (the rehearsal preview no longer matches the live install).
            if identity != shown.preview {
                let clean = rollback_and_check(&tx);
                drop(tx);
                if clean {
                    guard.disarm();
                } else {
                    drop(guard);
                }
                return ControlledApplySettlement {
                    failure: Some(OrchestrationError::Stale),
                    commit: CommitEvidence::NotCommitted,
                    rollback: RollbackDisposition::Explicit,
                    health: self.health,
                    artifacts: ArtifactDisposition::Dispose,
                };
            }

            // The one commit gate.
            if op.gate.try_claim_commit() {
                match tx.execute_batch("COMMIT") {
                    Ok(()) => {
                        if tx.is_autocommit() {
                            drop(tx);
                            guard.disarm();
                            (
                                CommitEvidence::Committed,
                                RollbackDisposition::NotApplicable,
                                None,
                                true,
                            )
                        } else {
                            let _ = tx.execute_batch("ROLLBACK");
                            drop(tx);
                            drop(guard);
                            (
                                CommitEvidence::Unknown,
                                RollbackDisposition::Unresolved,
                                Some(OrchestrationError::CommitNotAutocommit),
                                false,
                            )
                        }
                    }
                    Err(error) => {
                        let busy = is_busy(&error);
                        let rollback = tx.execute_batch("ROLLBACK");
                        let autocommit = tx.is_autocommit();
                        drop(tx);
                        if rollback.is_ok() && autocommit && busy {
                            guard.disarm();
                            (
                                CommitEvidence::NotCommitted,
                                RollbackDisposition::Explicit,
                                Some(OrchestrationError::Sqlite(error)),
                                true,
                            )
                        } else if rollback.is_ok() && autocommit {
                            drop(guard);
                            (
                                CommitEvidence::Unknown,
                                RollbackDisposition::Explicit,
                                Some(OrchestrationError::Sqlite(error)),
                                false,
                            )
                        } else {
                            drop(guard);
                            (
                                CommitEvidence::Unknown,
                                RollbackDisposition::Unresolved,
                                Some(OrchestrationError::Sqlite(error)),
                                false,
                            )
                        }
                    }
                }
            } else {
                // Cancellation won the gate: roll back, never commit.
                let rollback = tx.execute_batch("ROLLBACK");
                let autocommit = tx.is_autocommit();
                drop(tx);
                if rollback.is_ok() && autocommit {
                    guard.disarm();
                    (
                        CommitEvidence::NotCommitted,
                        RollbackDisposition::Explicit,
                        None,
                        true,
                    )
                } else {
                    drop(guard);
                    (
                        CommitEvidence::Unknown,
                        RollbackDisposition::Unresolved,
                        rollback.err().map(OrchestrationError::Sqlite),
                        false,
                    )
                }
            }
        };

        if clean && self.refresh_observation().is_err() {
            self.health = WriterHealth::Quarantined(QuarantineReason::UncertainSettlement);
        }

        let artifacts = match commit {
            CommitEvidence::Committed | CommitEvidence::Unknown => ArtifactDisposition::Retain,
            CommitEvidence::NotCommitted => ArtifactDisposition::Dispose,
        };
        ControlledApplySettlement {
            failure,
            commit,
            rollback,
            health: self.health,
            artifacts,
        }
    }
}

/// Ends an active transaction with one explicit rollback and reports whether
/// the connection returned to autocommit.
fn rollback_and_check(tx: &rusqlite::Transaction<'_>) -> bool {
    let _ = tx.execute_batch("ROLLBACK");
    tx.is_autocommit()
}

/// True only for the demonstrated `SQLITE_BUSY` commit failure, which the
/// settlement may classify as a proved rollback; every other category stays
/// conservative (managed-binding §7).
fn is_busy(error: &rusqlite::Error) -> bool {
    matches!(
        error,
        rusqlite::Error::SqliteFailure(
            rusqlite::ffi::Error {
                code: rusqlite::ErrorCode::DatabaseBusy,
                ..
            },
            _,
        )
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn adopted() -> (tempfile::TempDir, ManagedCatalogue) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("products.sqlite");
        let conn = Connection::open(&path).unwrap();
        conn.execute_batch("CREATE TABLE t (id INTEGER PRIMARY KEY, v TEXT) STRICT;")
            .unwrap();
        let owner = ManagedCatalogue::adopt_opened(conn, path).unwrap();
        (dir, owner)
    }

    #[test]
    fn adoption_starts_healthy_at_generation_zero_with_an_observation() {
        let (_dir, owner) = adopted();
        assert_eq!(owner.health, WriterHealth::Healthy);
        assert_eq!(owner.generation, 0);
        assert!(owner.observation.is_some());
        assert!(owner.identity.is_some());
    }

    #[test]
    fn a_read_lease_sees_the_connection_and_never_mutates_counters() {
        let (_dir, owner) = adopted();
        let seen = owner
            .with_read(|conn| -> Result<i64, rusqlite::Error> {
                conn.query_row("SELECT count(*) FROM t", [], |r| r.get(0))
            })
            .unwrap()
            .unwrap();
        assert_eq!(seen, 0);
        assert_eq!(owner.health, WriterHealth::Healthy);
    }

    #[test]
    fn a_read_lease_refuses_a_non_healthy_owner() {
        let (_dir, mut owner) = adopted();
        owner.health = WriterHealth::Quarantined(QuarantineReason::Panic);
        let err = owner.with_read(|_conn| Ok::<(), ()>(())).unwrap_err();
        assert!(matches!(err, OwnerError::NotHealthy(_)));
    }

    #[test]
    fn a_successful_mutation_consumes_a_generation_and_returns_healthy() {
        let (_dir, mut owner) = adopted();
        let result = owner
            .with_legacy_mutation(
                LegacyMutationKind::DirectPackage,
                |conn| {
                    conn.execute("INSERT INTO t (id, v) VALUES (1, 'x')", [])?;
                    Ok::<_, rusqlite::Error>(())
                },
                |_| LegacySettlementHint::SuccessfulReturn,
            )
            .unwrap();
        assert!(result.is_ok());
        assert_eq!(owner.generation, 1);
        assert_eq!(owner.health, WriterHealth::Healthy);
    }

    #[test]
    fn a_pre_commit_semantic_refusal_still_consumes_a_generation() {
        let (_dir, mut owner) = adopted();
        let result = owner
            .with_legacy_mutation(
                LegacyMutationKind::DirectPackage,
                |_conn| Err::<(), &str>("semantic refusal"),
                |r| match r {
                    Err(_) => LegacySettlementHint::PreCommitSemanticRefusal,
                    Ok(_) => LegacySettlementHint::SuccessfulReturn,
                },
            )
            .unwrap();
        assert!(result.is_err());
        assert_eq!(owner.generation, 1);
        assert_eq!(owner.health, WriterHealth::Healthy);
    }

    #[test]
    fn an_uncertain_settlement_quarantines_the_owner() {
        let (_dir, mut owner) = adopted();
        let result = owner
            .with_legacy_mutation(
                LegacyMutationKind::PathUpdate,
                |_conn| Err::<(), &str>("io failure"),
                |_| LegacySettlementHint::Uncertain,
            )
            .unwrap();
        assert!(result.is_err());
        assert_eq!(owner.generation, 1);
        assert!(matches!(
            owner.health,
            WriterHealth::Quarantined(QuarantineReason::UncertainSettlement)
        ));
    }

    #[test]
    fn a_panicking_mutation_quarantines_and_keeps_the_increment() {
        let (_dir, mut owner) = adopted();
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            owner.with_legacy_mutation(
                LegacyMutationKind::BackupDelete,
                |_conn: &mut Connection| -> Result<(), ()> { panic!("boom") },
                |_: &Result<(), ()>| LegacySettlementHint::SuccessfulReturn,
            )
        }));
        assert!(result.is_err());
        assert_eq!(owner.generation, 1);
        assert!(matches!(
            owner.health,
            WriterHealth::Quarantined(QuarantineReason::Panic)
        ));
    }

    #[test]
    fn generation_overflow_refuses_without_wrapping() {
        let (_dir, mut owner) = adopted();
        owner.generation = u64::MAX;
        let err = owner
            .with_legacy_mutation(
                LegacyMutationKind::DirectPackage,
                |_conn| Ok::<(), ()>(()),
                |_: &Result<(), ()>| LegacySettlementHint::SuccessfulReturn,
            )
            .unwrap_err();
        assert!(matches!(err, OwnerError::GenerationOverflow));
        assert_eq!(owner.generation, u64::MAX);
    }

    #[test]
    fn restore_adopts_a_fresh_incarnation_and_healthy_handle() {
        let dir = tempfile::tempdir().unwrap();
        let live = dir.path().join("products.sqlite");
        // A real, migrated product DB: restore reopens via `open_and_migrate`,
        // which refuses the synthetic schema used by `adopted()`.
        let conn = crate::migration::open_and_migrate(&live).unwrap();
        let mut owner = ManagedCatalogue::adopt_opened(conn, live.clone()).unwrap();
        let backup = dir.path().join("backup.sqlite");
        std::fs::copy(&live, &backup).unwrap();
        let before = owner.incarnation;
        owner.restore_legacy(&live, &backup).unwrap();
        assert_eq!(owner.health, WriterHealth::Healthy);
        assert_ne!(owner.incarnation, before);
    }

    #[test]
    fn an_invalid_backup_is_refused_and_the_owner_stays_healthy() {
        let (dir, mut owner) = adopted();
        let live = dir.path().join("products.sqlite");
        // A directory cannot be opened as a database backup: the validation
        // refusal must leave the old connection healthy and usable.
        let not_a_db = dir.path().join("subdir");
        std::fs::create_dir(&not_a_db).unwrap();
        let err = owner.restore_legacy(&live, &not_a_db).unwrap_err();
        assert!(matches!(err, OwnerRestoreError::InvalidBackup(_)));
        assert!(matches!(owner.health, WriterHealth::Healthy));
        assert!(owner.conn.is_some());
    }

    /// A real, migrated schema-28 product DB, since `capture_stamp` runs the
    /// full witness which admits only the supported schema shape.
    fn migrated_owner() -> (tempfile::TempDir, PathBuf, ManagedCatalogue) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("products.sqlite");
        let conn = crate::open_and_migrate(&path).unwrap();
        let owner = ManagedCatalogue::adopt_opened(conn, path.clone()).unwrap();
        (dir, path, owner)
    }

    #[test]
    fn a_captured_stamp_binds_the_full_witness_and_identity() {
        let (_dir, _path, mut owner) = migrated_owner();
        let gate = PublicationGate::new();
        let stamp = owner.capture_stamp(&gate).unwrap();
        assert_eq!(stamp.generation, 0);
        assert_eq!(stamp.incarnation, owner.incarnation);
        assert!(stamp.file.is_some());
        assert_eq!(stamp.witness.table_count, 47);
        assert_eq!(stamp.witness.schema_object_count, 120);
        assert_eq!(stamp.witness.row_count, 121);
        assert_ne!(stamp.witness.sha256, [0u8; 32]);
    }

    #[test]
    fn two_consecutive_captures_are_identical_and_leave_the_connection_autocommit() {
        let (_dir, _path, mut owner) = migrated_owner();
        let gate = PublicationGate::new();
        let first = owner.capture_stamp(&gate).unwrap();
        let second = owner.capture_stamp(&gate).unwrap();
        assert_eq!(first, second);
        assert!(owner.conn.as_ref().unwrap().is_autocommit());
        assert_eq!(owner.generation, 0);
    }

    #[test]
    fn an_external_write_changes_data_version_and_the_witness() {
        let (_dir, path, mut owner) = migrated_owner();
        let gate = PublicationGate::new();
        let first = owner.capture_stamp(&gate).unwrap();
        {
            let other = Connection::open(&path).unwrap();
            other
                .execute(
                    "INSERT INTO manufacturer (id, name) VALUES ('m1', 'Example Co')",
                    [],
                )
                .unwrap();
        }
        let second = owner.capture_stamp(&gate).unwrap();
        assert_ne!(first.data_version, second.data_version);
        assert_ne!(first.witness.sha256, second.witness.sha256);
    }

    #[test]
    fn a_same_handle_managed_write_consumes_a_generation_not_a_data_version() {
        let (_dir, _path, mut owner) = migrated_owner();
        let gate = PublicationGate::new();
        let first = owner.capture_stamp(&gate).unwrap();
        owner
            .with_legacy_mutation(
                LegacyMutationKind::DirectPackage,
                |conn| {
                    conn.execute(
                        "INSERT INTO manufacturer (id, name) VALUES ('m2', 'Acme')",
                        [],
                    )?;
                    Ok::<_, rusqlite::Error>(())
                },
                |_| LegacySettlementHint::SuccessfulReturn,
            )
            .unwrap()
            .unwrap();
        let second = owner.capture_stamp(&gate).unwrap();
        assert_eq!(first.generation, 0);
        assert_eq!(second.generation, 1);
        // A same-W commit does not move W's own data_version; generation is
        // the complementary detector (managed-binding §3).
        assert_eq!(first.data_version, second.data_version);
        assert_ne!(first.witness.sha256, second.witness.sha256);
    }

    #[test]
    fn capture_stamp_refuses_a_non_healthy_owner() {
        let (_dir, _path, mut owner) = migrated_owner();
        owner.health = WriterHealth::Quarantined(QuarantineReason::Panic);
        let err = owner.capture_stamp(&PublicationGate::new()).unwrap_err();
        assert!(matches!(
            err,
            crate::catalogue_stamp::StampError::NotHealthy(_)
        ));
    }

    #[test]
    fn a_precancelled_gate_stops_the_capture_and_still_rolls_back() {
        let (_dir, _path, mut owner) = migrated_owner();
        let gate = PublicationGate::new();
        assert_eq!(gate.try_cancel(), crate::operation::CancelOutcome::Accepted);
        let err = owner.capture_stamp(&gate).unwrap_err();
        assert!(matches!(
            err,
            crate::catalogue_stamp::StampError::Witness(
                crate::catalogue_schema::WitnessError::Stop(crate::operation::Stop::CancelAccepted)
            )
        ));
        // The read-only reservation is still rolled back to autocommit.
        assert!(owner.conn.as_ref().unwrap().is_autocommit());
    }
}
