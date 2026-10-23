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
    parent_dev: u64,
    parent_ino: u64,
    dev: u64,
    ino: u64,
    nlink: u64,
}

impl ManagedFileIdentity {
    fn is_current(&self) -> bool {
        let Ok(main) = std::fs::symlink_metadata(&self.canonical) else {
            return false;
        };
        let Ok(parent) = std::fs::symlink_metadata(&self.parent) else {
            return false;
        };
        main.is_file()
            && !main.file_type().is_symlink()
            && main.dev() == self.dev
            && main.ino() == self.ino
            && main.nlink() == 1
            && self.nlink == 1
            && parent.is_dir()
            && !parent.file_type().is_symlink()
            && parent.dev() == self.parent_dev
            && parent.ino() == self.parent_ino
    }

    fn from_path(path: &Path) -> Option<Self> {
        let canonical = std::fs::canonicalize(path).ok()?;
        let parent = canonical
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_default();
        let meta = std::fs::metadata(&canonical).ok()?;
        let parent_meta = std::fs::metadata(&parent).ok()?;
        Some(Self {
            canonical,
            parent,
            parent_dev: parent_meta.dev(),
            parent_ino: parent_meta.ino(),
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

/// A failed explicit close is not disposal: the known source handle and its
/// original W acquisition receipt stay together on the managed owner.
struct UnresolvedPreviewSource {
    #[allow(dead_code)] // ownership is retained until a later explicit close/quarantine
    connection: Connection,
    #[allow(dead_code)] // immutable receipt, not a fresh replacement baseline
    stamp: crate::catalogue_stamp::CatalogueStamp,
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
    // A failed explicit S close returns a live handle. Keep it on the managed
    // owner rather than allowing a temporary error to silently dispose it.
    unresolved_preview_source: Option<UnresolvedPreviewSource>,
    apply_attempt_serial: u64,
    retained_apply_witness: Option<crate::orchestration::RetainedApplyWitness>,
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
            unresolved_preview_source: None,
            apply_attempt_serial: 0,
            retained_apply_witness: None,
        })
    }

    /// Metadata only; never repairs health or borrows SQLite.
    pub fn health_snapshot(&self) -> WriterHealth {
        self.health
    }

    /// Metadata only: available even while Mutating/Quarantined; no SQL or repair.
    #[allow(dead_code)] // consumed by the future private worker integration
    pub(crate) fn retained_apply_witness(
        &self,
    ) -> Option<crate::orchestration::RetainedApplyWitness> {
        self.retained_apply_witness
    }

    /// Explicit keyed ownership transfer, independent of settlement/health.
    #[allow(dead_code)]
    pub(crate) fn take_apply_witness(
        &mut self,
        key: crate::orchestration::ApplyAttemptKey,
    ) -> Result<
        crate::orchestration::RetainedApplyWitness,
        crate::orchestration::ApplyWitnessTransferError,
    > {
        use crate::orchestration::ApplyWitnessTransferError;
        match self.retained_apply_witness {
            None => Err(ApplyWitnessTransferError::Absent),
            Some(record) if record.key != key => Err(ApplyWitnessTransferError::KeyMismatch),
            Some(record) => {
                self.retained_apply_witness = None;
                Ok(record)
            }
        }
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
            configuration: crate::catalogue_stamp::WriterConfigurationReceipt::read(conn)?,
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
            AdmittedPreviewArtifact, ControlledPreview, OrchestrationError, PreviewCleanupError,
            PreviewFailure,
        };

        if self.health != WriterHealth::Healthy {
            return Err(OrchestrationError::NotHealthy(self.health));
        }
        op.gate.checkpoint()?;
        if source.limits != op.policy.limits() {
            return Err(OrchestrationError::PolicyMismatch);
        }

        // Retain one unchanged W reservation through stamp acquisition and S→D.
        // Every post-BEGIN result converges on the inspected rollback below.
        let identity = self
            .identity
            .as_ref()
            .ok_or(OrchestrationError::MissingConnection)?;
        if !identity.is_current() {
            self.health = WriterHealth::Quarantined(QuarantineReason::IdentityMismatch);
            return Err(OrchestrationError::Stale);
        }
        let path = identity.canonical.clone();
        let file = FileStamp::from_path(&path);
        let conn = self
            .conn
            .as_mut()
            .ok_or(OrchestrationError::MissingConnection)?;
        require_writer_configuration(conn)?;
        let configuration = crate::catalogue_stamp::WriterConfigurationReceipt::read(conn)?;
        self.health = WriterHealth::Mutating;
        let guard = MutationGuard {
            health: &mut self.health,
            armed: true,
        };
        let mut tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        tx.set_drop_behavior(DropBehavior::Ignore);
        let total_changes = tx.total_changes();
        let mut data_version = None;
        let mut cleanup = Vec::new();
        let preparation = (|| -> Result<_, OrchestrationError> {
            require_writer_configuration(&tx)?;
            if crate::catalogue_stamp::WriterConfigurationReceipt::read(&tx)? != configuration {
                return Err(OrchestrationError::Stale);
            }
            let _: i64 = tx.query_row("SELECT count(*) FROM sqlite_schema", [], |r| r.get(0))?;
            let before: i64 = tx.query_row("PRAGMA data_version", [], |r| r.get(0))?;
            data_version = Some(before);
            let witness = crate::catalogue_schema::witness_current(&tx, op.gate)
                .map_err(crate::catalogue_stamp::StampError::from)?;
            let stamp = crate::catalogue_stamp::CatalogueStamp {
                incarnation: self.incarnation,
                generation: self.generation,
                data_version: before,
                total_changes,
                file,
                witness,
                configuration,
            };
            #[cfg(test)]
            artifacts.before_source_open(&path);
            let source_ro = Connection::open_with_flags(
                &path,
                rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY
                    | rusqlite::OpenFlags::SQLITE_OPEN_PRIVATE_CACHE
                    | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
            )?;
            let copy = (|| -> Result<_, OrchestrationError> {
                if !source_ro.is_readonly("main")? {
                    return Err(crate::catalogue_copy::CopyError::NotReadOnly.into());
                }
                source_ro.pragma_update(None, "query_only", true)?;
                source_ro.busy_timeout(crate::migration::READ_ONLY_BUSY_TIMEOUT)?;
                let mut source_tx = source_ro.unchecked_transaction()?;
                source_tx.set_drop_behavior(DropBehavior::Ignore);
                let _: i64 =
                    source_tx.query_row("SELECT count(*) FROM sqlite_schema", [], |r| r.get(0))?;
                #[cfg(test)]
                artifacts.inspect_pinned_source(&source_tx);
                let source_witness = crate::catalogue_schema::witness_current(&source_tx, op.gate)
                    .map_err(crate::catalogue_stamp::StampError::from)?;
                if source_witness != stamp.witness {
                    return Err(OrchestrationError::Stale);
                }
                if crate::catalogue_copy::CopyFacts::read(&source_tx)?
                    != crate::catalogue_copy::CopyFacts::read(&tx)?
                {
                    return Err(OrchestrationError::Stale);
                }
                let page_size: i64 = source_tx.query_row("PRAGMA page_size", [], |r| r.get(0))?;
                let fresh = artifacts.fresh_rehearsal(page_size)?;
                let validated = crate::orchestration::validate_copy(
                    artifacts, fresh, &source_tx, op.gate, false,
                )?;
                Ok(validated)
            })();
            if !source_ro.is_autocommit() {
                if let Err(error) = source_ro.execute_batch("ROLLBACK") {
                    cleanup.push(PreviewCleanupError::SourceEnd(error));
                }
            }
            if let Err((connection, error)) = source_ro.close() {
                self.unresolved_preview_source =
                    Some(UnresolvedPreviewSource { connection, stamp });
                cleanup.push(PreviewCleanupError::SourceClose(error));
            }
            let validated = copy?;
            Ok((stamp, validated))
        })();
        // Baseline/identity inspection and rollback are deliberately not gated
        // by Stop: cancellation cannot skip settlement of an owned reservation.
        let baseline = check_reserved_writer(&tx, identity, file, total_changes, data_version);
        if let Err(error) = baseline {
            cleanup.push(error);
        }
        if let Err(error) = tx.execute_batch("ROLLBACK") {
            cleanup.push(PreviewCleanupError::WriterRollback(error));
        }
        if !tx.is_autocommit() {
            cleanup.push(PreviewCleanupError::WriterNotAutocommit);
        }
        drop(tx);
        if cleanup.is_empty() {
            guard.disarm();
        } else {
            drop(guard);
            return Err(OrchestrationError::PreviewSettlement(Box::new(
                PreviewFailure {
                    primary: preparation.err(),
                    cleanup,
                },
            )));
        }
        let (stamp, validated) = preparation?;

        // Keep D closed until inspected S/W settlement succeeds. A cleanup
        // refusal must not discard the only receipt for an open RW handle.
        // Preserve the direct Stop cause if cleanup latched cancellation.
        op.gate.checkpoint()?;
        let rehearsal = artifacts.inventory().open_rehearsal(validated, op.gate)?;
        let install = (|| -> Result<_, OrchestrationError> {
            op.gate.checkpoint()?;
            #[cfg(test)]
            artifacts.before_rehearsal(&rehearsal);
            Ok(crate::wrapper::install_admitted_product_file(
                artifacts.inventory().rehearsal(&rehearsal)?,
                source.name,
                source.bytes,
                source.limits,
                Cancellation::Controlled(op.gate),
            )?)
        })();
        let close = artifacts.inventory().close_rehearsal(rehearsal);
        let install = match (install, close) {
            (Err(primary), Err(close)) => {
                return Err(OrchestrationError::PreviewSettlement(Box::new(
                    PreviewFailure {
                        primary: Some(primary),
                        cleanup: vec![PreviewCleanupError::RehearsalClose(close)],
                    },
                )));
            }
            (Err(primary), Ok(())) => return Err(primary),
            (Ok(_), Err(close)) => return Err(close.into()),
            (Ok(install), Ok(())) => install,
        };
        let identity = crate::preview_identity::preview_identity(&install)?;

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
        self.apply_current_controlled_with_claim(source, shown, op, artifacts, op.gate)
    }

    pub(crate) fn apply_current_controlled_with_claim(
        &mut self,
        source: crate::controlled::ControlledSource<'_>,
        shown: &crate::preview_binding::PreviewBinding,
        op: &crate::orchestration::OperationContext<'_>,
        artifacts: &mut dyn crate::orchestration::CatalogueArtifacts,
        control: &dyn crate::operation::CommitArbitration,
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
        if self.retained_apply_witness.is_some() {
            return ControlledApplySettlement::refused(
                OrchestrationError::ApplyWitnessPending,
                self.health,
            );
        }
        let Some(serial) = self.apply_attempt_serial.checked_add(1) else {
            return ControlledApplySettlement::refused(
                OrchestrationError::ApplyAttemptExhausted,
                self.health,
            );
        };
        self.apply_attempt_serial = serial;
        self.retained_apply_witness = Some(crate::orchestration::RetainedApplyWitness {
            key: crate::orchestration::ApplyAttemptKey {
                incarnation: self.incarnation,
                serial,
            },
            shown_generation: shown.catalogue.generation,
            source_length: source.bytes.len() as u64,
            source_sha256: source.sha256,
            trace: crate::orchestration::LiveCommitTrace::NoLiveCommitInvocation,
        });
        if let Err(stop) = op.gate.checkpoint() {
            return ControlledApplySettlement::refused(OrchestrationError::Stop(stop), self.health);
        }
        if shown.version != crate::preview_binding::BINDING_VERSION {
            return ControlledApplySettlement::refused(OrchestrationError::Stale, self.health);
        }
        if source.limits != op.policy.limits()
            || shown.policy != crate::controlled::PolicyIdentity::from_policy(op.policy)
        {
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

        // Reserve W once, before comparing the original stamp, and retain this
        // same transaction through S→D/B, live DML and explicit settlement.
        let path = match self.identity.as_ref() {
            Some(id) => id.canonical.clone(),
            None => {
                return ControlledApplySettlement::refused(
                    OrchestrationError::MissingConnection,
                    self.health,
                );
            }
        };
        let identity = self.identity.as_ref().unwrap();
        if !identity.is_current() {
            self.health = WriterHealth::Quarantined(QuarantineReason::IdentityMismatch);
            return ControlledApplySettlement::refused(OrchestrationError::Stale, self.health);
        }
        let next_generation = match self.generation.checked_add(1) {
            Some(next) => next,
            None => {
                self.health = WriterHealth::Quarantined(QuarantineReason::UncertainSettlement);
                return ControlledApplySettlement::refused(
                    OrchestrationError::NotHealthy(self.health),
                    self.health,
                );
            }
        };
        let conn = match self.conn.as_mut() {
            Some(conn) => conn,
            None => {
                return ControlledApplySettlement::refused(
                    OrchestrationError::MissingConnection,
                    self.health,
                );
            }
        };
        if let Err(error) = require_writer_configuration(conn) {
            return ControlledApplySettlement::refused(error, self.health);
        }
        match crate::catalogue_stamp::WriterConfigurationReceipt::read(conn) {
            Ok(configuration) if configuration == shown.catalogue.configuration => {}
            Ok(_) => {
                return ControlledApplySettlement::refused(OrchestrationError::Stale, self.health);
            }
            Err(error) => return ControlledApplySettlement::refused(error.into(), self.health),
        }
        if let Err(error) = crate::catalogue_growth::require_synchronous(conn) {
            return ControlledApplySettlement::refused(
                OrchestrationError::Growth(error),
                self.health,
            );
        }
        self.health = WriterHealth::Mutating;
        let guard = MutationGuard {
            health: &mut self.health,
            armed: true,
        };
        let mut tx = match conn.transaction_with_behavior(TransactionBehavior::Immediate) {
            Ok(tx) => tx,
            Err(error) => {
                drop(guard);
                return ControlledApplySettlement::refused(error.into(), self.health);
            }
        };
        tx.set_drop_behavior(DropBehavior::Ignore);
        let mut source_unresolved = false;
        let baseline_file = FileStamp::from_path(&path);
        let baseline_changes = tx.total_changes();
        let mut baseline_version = None;
        let mut live_started = false;
        let mut prior_growth_cap = None;
        let mut cleanup = Vec::new();

        // Every preclaim failure shares one inspected rollback path. In
        // particular, no copy/installer/identity branch can invent a successful
        // rollback or independently authorize recovery disposal.
        let body = (|| -> Result<(), OrchestrationError> {
            require_writer_configuration(&tx)?;
            let _: i64 = tx.query_row("SELECT count(*) FROM sqlite_schema", [], |r| r.get(0))?;
            let before: i64 = tx.query_row("PRAGMA data_version", [], |r| r.get(0))?;
            baseline_version = Some(before);
            let witness = crate::catalogue_schema::witness_current(&tx, op.gate)
                .map_err(crate::catalogue_stamp::StampError::from)?;
            let after: i64 = tx.query_row("PRAGMA data_version", [], |r| r.get(0))?;
            let fresh_stamp = crate::catalogue_stamp::CatalogueStamp {
                incarnation: self.incarnation,
                generation: self.generation,
                data_version: before,
                total_changes: tx.total_changes(),
                file: FileStamp::from_path(&path),
                witness,
                configuration: crate::catalogue_stamp::WriterConfigurationReceipt::read(&tx)?,
            };
            if before != after || fresh_stamp != shown.catalogue {
                return Err(OrchestrationError::Stale);
            }
            let growth = crate::catalogue_growth::WriterGrowth::admit(&tx)
                .map_err(OrchestrationError::Growth)?;
            // Record the old connection-local cap before attempting a setter;
            // even a partial setter/readback failure must take inspected cleanup.
            prior_growth_cap = Some(growth.prior());
            growth.enforce(&tx).map_err(OrchestrationError::Growth)?;
            // The unchanged reservation is not a managed mutation until the
            // full stamp matches. Admission advances generation before copies.
            self.generation = next_generation;

            #[cfg(test)]
            artifacts.before_source_open(&path);
            let source_ro = Connection::open_with_flags(
                &path,
                rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY
                    | rusqlite::OpenFlags::SQLITE_OPEN_PRIVATE_CACHE
                    | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
            )?;
            let copies =
                (|| -> Result<crate::catalogue_copy::ValidatedDestination, OrchestrationError> {
                    if !source_ro.is_readonly("main")? {
                        return Err(crate::catalogue_copy::CopyError::NotReadOnly.into());
                    }
                    source_ro.pragma_update(None, "query_only", true)?;
                    source_ro.busy_timeout(crate::migration::READ_ONLY_BUSY_TIMEOUT)?;
                    let mut source_tx = source_ro.unchecked_transaction()?;
                    source_tx.set_drop_behavior(DropBehavior::Ignore);
                    let _: i64 =
                        source_tx
                            .query_row("SELECT count(*) FROM sqlite_schema", [], |r| r.get(0))?;
                    #[cfg(test)]
                    artifacts.inspect_pinned_source(&source_tx);
                    let source_witness =
                        crate::catalogue_schema::witness_current(&source_tx, op.gate)
                            .map_err(crate::catalogue_stamp::StampError::from)?;
                    if source_witness != fresh_stamp.witness
                        || crate::catalogue_copy::CopyFacts::read(&source_tx)?
                            != crate::catalogue_copy::CopyFacts::read(&tx)?
                    {
                        return Err(OrchestrationError::Stale);
                    }
                    let page_size: i64 =
                        source_tx.query_row("PRAGMA page_size", [], |r| r.get(0))?;
                    let rehearsal = artifacts.fresh_rehearsal(page_size)?;
                    let validated_d = crate::orchestration::validate_copy(
                        artifacts, rehearsal, &source_tx, op.gate, false,
                    )?;
                    let recovery = artifacts.fresh_recovery(page_size)?;
                    crate::orchestration::validate_copy(
                        artifacts, recovery, &source_tx, op.gate, true,
                    )?;
                    Ok(validated_d)
                })();
            // Cleanup is uncancellable, including a primary copy/witness Stop.
            let mut cleanup = Vec::new();
            if !source_ro.is_autocommit() {
                if let Err(error) = source_ro.execute_batch("ROLLBACK") {
                    cleanup.push(crate::orchestration::PreviewCleanupError::SourceEnd(error));
                }
            }
            if let Err((connection, error)) = source_ro.close() {
                self.unresolved_preview_source = Some(UnresolvedPreviewSource {
                    connection,
                    stamp: fresh_stamp,
                });
                cleanup.push(crate::orchestration::PreviewCleanupError::SourceClose(
                    error,
                ));
            }
            if !cleanup.is_empty() {
                source_unresolved = true;
                return Err(OrchestrationError::SourceSettlement(Box::new(
                    crate::orchestration::PreviewFailure {
                        primary: copies.err(),
                        cleanup,
                    },
                )));
            }
            let validated_d = copies?;
            // D stays closed until B validation and inspected S settlement.
            // Its disposable work never claims the live publication gate.
            op.gate.checkpoint()?;
            let rehearsal = artifacts.inventory().open_rehearsal(validated_d, op.gate)?;
            let rehearse = (|| -> Result<(), OrchestrationError> {
                op.gate.checkpoint()?;
                #[cfg(test)]
                artifacts.before_rehearsal(&rehearsal);
                let install = crate::wrapper::install_admitted_product_file(
                    artifacts.inventory().rehearsal(&rehearsal)?,
                    source.name,
                    source.bytes,
                    source.limits,
                    Cancellation::Controlled(op.gate),
                )?;
                let identity = crate::preview_identity::preview_identity(&install)?;
                if identity != shown.preview {
                    return Err(OrchestrationError::Stale);
                }
                Ok(())
            })();
            let close = artifacts.inventory().close_rehearsal(rehearsal);
            match (rehearse, close) {
                (Err(primary), Err(close)) => {
                    return Err(OrchestrationError::PreviewSettlement(Box::new(
                        crate::orchestration::PreviewFailure {
                            primary: Some(primary),
                            cleanup: vec![
                                crate::orchestration::PreviewCleanupError::RehearsalClose(close),
                            ],
                        },
                    )));
                }
                (Err(primary), Ok(())) => return Err(primary),
                (Ok(()), Err(close)) => return Err(close.into()),
                (Ok(()), Ok(())) => {}
            }
            op.gate.checkpoint()?;
            check_reserved_writer(
                &tx,
                identity,
                baseline_file,
                baseline_changes,
                baseline_version,
            )
            .map_err(|_| OrchestrationError::Stale)?;
            // The actual validated/closed B must be no-replace published and
            // file+both-directory sync acknowledged BEFORE the first W DML.
            // The public facade consumes an opaque receipt for this exact B.
            artifacts.publish_recovery()?;
            op.gate.checkpoint()?;
            live_started = true;
            #[cfg(test)]
            artifacts.before_live_install(&tx);

            let install = crate::wrapper::install_admitted_product_file(
                &tx,
                source.name,
                source.bytes,
                source.limits,
                Cancellation::Controlled(op.gate),
            )?;
            let identity = crate::preview_identity::preview_identity(&install)?;
            if identity != shown.preview {
                return Err(OrchestrationError::Stale);
            }
            let page_count: i64 = tx.query_row("PRAGMA main.page_count", [], |r| r.get(0))?;
            let page_size: i64 = tx.query_row("PRAGMA main.page_size", [], |r| r.get(0))?;
            let page_bytes = u64::try_from(page_count)
                .ok()
                .and_then(|n| u64::try_from(page_size).ok().and_then(|s| n.checked_mul(s)))
                .ok_or(OrchestrationError::ResultAdmission(
                    crate::managed_apply::RecoveryPublicationError::Refused,
                ))?;
            artifacts.admit_result(&install, page_bytes)?;
            Ok(())
        })();

        if !live_started {
            if let Err(error) = check_reserved_writer(
                &tx,
                identity,
                baseline_file,
                baseline_changes,
                baseline_version,
            ) {
                cleanup.push(error);
            }
        }
        // The slot was installed before fallible work; borrow it BEFORE COMMIT.
        // Recording native OK must not allocate, look up, lock or call user code.
        let retained = self
            .retained_apply_witness
            .as_mut()
            .expect("admitted apply receipt");
        let (commit, rollback, failure) = if let Err(error) = body {
            let rollback = rollback_and_record(&tx, &mut cleanup);
            (CommitEvidence::NotCommitted, rollback, Some(error))
        } else if control
            .claim_commit()
            .is_some_and(|claim| claim.belongs_to(op.gate))
        {
            // One live COMMIT, never retried. Retain the original COMMIT error
            // independently of the observed rollback disposition.
            retained.trace = crate::orchestration::LiveCommitTrace::LiveCommitInvoked;
            match tx.execute_batch("COMMIT") {
                Ok(()) => {
                    let autocommit = tx.is_autocommit();
                    retained.trace =
                        crate::orchestration::LiveCommitTrace::LiveCommitReturnedOk { autocommit };
                    if !autocommit {
                        (
                            CommitEvidence::Unknown,
                            rollback_and_record(&tx, &mut cleanup),
                            Some(OrchestrationError::CommitNotAutocommit),
                        )
                    } else {
                        #[cfg(test)]
                        artifacts.after_live_commit(&tx);
                        (
                            CommitEvidence::Committed,
                            RollbackDisposition::NotApplicable,
                            None,
                        )
                    }
                }
                Err(error) => {
                    let rollback = rollback_and_record(&tx, &mut cleanup);
                    let commit = if is_busy(&error) && rollback == RollbackDisposition::Explicit {
                        CommitEvidence::NotCommitted
                    } else {
                        CommitEvidence::Unknown
                    };
                    (commit, rollback, Some(OrchestrationError::Sqlite(error)))
                }
            }
        } else {
            // Cancellation won; no COMMIT was issued, regardless of whether
            // the independent rollback operation succeeds.
            (
                CommitEvidence::NotCommitted,
                rollback_and_record(&tx, &mut cleanup),
                if op.gate.state() == crate::operation::PublicationState::CancelAccepted {
                    None
                } else {
                    Some(OrchestrationError::CommitClaimRefused)
                },
            )
        };
        drop(tx);
        if let Some(prior) = prior_growth_cap {
            if let Err(error) = crate::catalogue_growth::restore(conn, prior) {
                cleanup.push(crate::orchestration::PreviewCleanupError::GrowthRestore(
                    error,
                ));
            }
        }
        let mut clean = !source_unresolved
            && cleanup.is_empty()
            && commit != CommitEvidence::Unknown
            && rollback != RollbackDisposition::Unresolved;
        if clean {
            match Self::observe(conn, &path) {
                Ok(observation) => self.observation = Some(observation),
                Err(_) => clean = false,
            }
        }
        let recovery_exists = artifacts
            .inventory()
            .allocated(crate::catalogue_copy::DestinationRole::Recovery);
        let artifacts = if !recovery_exists {
            ArtifactDisposition::None
        } else if commit == CommitEvidence::NotCommitted
            && rollback != RollbackDisposition::Unresolved
            && clean
        {
            ArtifactDisposition::Dispose
        } else {
            ArtifactDisposition::Retain
        };
        // No trait/fallible call follows clean disarm. Tail projection panics
        // therefore quarantine exactly like immediate post-COMMIT panics.
        if clean {
            guard.disarm();
        } else {
            drop(guard);
        }
        ControlledApplySettlement {
            failure,
            cleanup,
            commit,
            rollback,
            health: self.health,
            artifacts,
        }
    }
}

/// Observe only fixed, trusted native scalar pragmas. Both admission passes
/// use the actual W; no migration, defaulting or configuration repair occurs.
fn require_writer_configuration(
    conn: &Connection,
) -> Result<(), crate::orchestration::OrchestrationError> {
    use crate::orchestration::{OrchestrationError, WriterConfigurationError, WriterSetting};
    for (setting, sql, expected) in [
        (WriterSetting::ForeignKeys, "PRAGMA foreign_keys", 1),
        (
            WriterSetting::IgnoreCheckConstraints,
            "PRAGMA ignore_check_constraints",
            0,
        ),
        (WriterSetting::WritableSchema, "PRAGMA writable_schema", 0),
        (
            WriterSetting::DeferForeignKeys,
            "PRAGMA defer_foreign_keys",
            0,
        ),
        (WriterSetting::QueryOnly, "PRAGMA query_only", 0),
    ] {
        let observed: i64 = conn
            .query_row(sql, [], |row| row.get(0))
            .map_err(|source| {
                OrchestrationError::WriterConfiguration(WriterConfigurationError::Read {
                    setting,
                    source,
                })
            })?;
        if observed != expected {
            return Err(OrchestrationError::WriterConfiguration(
                WriterConfigurationError::Value {
                    setting,
                    expected,
                    observed,
                },
            ));
        }
    }
    Ok(())
}

/// Unchanged W reservation checks are not cancellable cleanup work.
fn check_reserved_writer(
    conn: &Connection,
    identity: &ManagedFileIdentity,
    file: Option<FileStamp>,
    changes: u64,
    version: Option<i64>,
) -> Result<(), crate::orchestration::PreviewCleanupError> {
    use crate::orchestration::PreviewCleanupError;
    let current: i64 = conn
        .query_row("PRAGMA data_version", [], |r| r.get(0))
        .map_err(PreviewCleanupError::WriterObservation)?;
    if version.is_some_and(|v| v != current)
        || conn.total_changes() != changes
        || file.is_none()
        || !identity.is_current()
        || FileStamp::from_path(&identity.canonical) != file
    {
        return Err(PreviewCleanupError::WriterBaseline);
    }
    Ok(())
}

/// Settles a preclaim transaction, distinguishing an already automatic end
/// from a successful explicit rollback and an unresolved native state.
fn rollback_and_record(
    tx: &rusqlite::Transaction<'_>,
    cleanup: &mut Vec<crate::orchestration::PreviewCleanupError>,
) -> crate::orchestration::RollbackDisposition {
    use crate::orchestration::{PreviewCleanupError, RollbackDisposition};
    if tx.is_autocommit() {
        return RollbackDisposition::Automatic;
    }
    let rollback = tx.execute_batch("ROLLBACK");
    let failed = rollback.is_err();
    if let Err(error) = rollback {
        cleanup.push(PreviewCleanupError::WriterRollback(error));
    }
    if !tx.is_autocommit() {
        cleanup.push(PreviewCleanupError::WriterNotAutocommit);
    }
    if failed || !tx.is_autocommit() {
        RollbackDisposition::Unresolved
    } else {
        RollbackDisposition::Explicit
    }
}
#[cfg(test)]
fn rollback_and_check(tx: &rusqlite::Transaction<'_>) -> crate::orchestration::RollbackDisposition {
    rollback_and_record(tx, &mut Vec::new())
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
mod reopened_preview_tests;

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
    fn preclaim_native_automatic_rollback_is_not_reported_as_explicit() {
        use crate::orchestration::RollbackDisposition;
        let mut conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("CREATE TABLE t(id INTEGER PRIMARY KEY); INSERT INTO t VALUES(1)")
            .unwrap();
        let mut tx = conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .unwrap();
        tx.set_drop_behavior(DropBehavior::Ignore);
        tx.execute("INSERT INTO t VALUES(2)", []).unwrap();
        assert!(tx
            .execute("INSERT OR ROLLBACK INTO t VALUES(1)", [])
            .is_err());
        assert!(
            tx.is_autocommit(),
            "SQLite must actually roll the transaction back"
        );
        assert_eq!(rollback_and_check(&tx), RollbackDisposition::Automatic);
        assert_eq!(
            tx.query_row("SELECT count(*) FROM t", [], |r| r.get::<_, i64>(0))
                .unwrap(),
            1
        );
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
    fn apply_attempt_exhaustion_refuses_without_native_work_or_artifacts() {
        use crate::controlled::{
            ControlledPolicy, ControlledSource, PolicyIdentity, SourceIdentity,
        };
        use crate::orchestration::{
            ArtifactDisposition, CatalogueArtifacts, CommitEvidence, OperationContext,
            OrchestrationError,
        };
        struct NoArtifacts;
        impl CatalogueArtifacts for NoArtifacts {
            fn inventory(&mut self) -> &mut crate::catalogue_copy::ArtifactInventory {
                panic!("exhausted attempt cannot access inventory")
            }
            fn fresh_rehearsal(
                &mut self,
                _: i64,
            ) -> Result<crate::catalogue_copy::FreshDestination, OrchestrationError> {
                panic!("exhausted attempt cannot allocate D")
            }
            fn fresh_recovery(
                &mut self,
                _: i64,
            ) -> Result<crate::catalogue_copy::FreshDestination, OrchestrationError> {
                panic!("exhausted attempt cannot allocate B")
            }
        }
        let (_dir, _path, mut owner) = migrated_owner();
        let gate = PublicationGate::new();
        let policy = ControlledPolicy::Standard;
        let source = ControlledSource::admit("counter.knxprod", b"not-an-archive", policy).unwrap();
        // Deliberately synthetic: only the pre-native exhaustion boundary is tested.
        let shown = crate::preview_binding::PreviewBinding::bind(
            owner.capture_stamp(&gate).unwrap(),
            SourceIdentity::from_source(&source),
            PolicyIdentity::from_policy(policy),
            crate::preview_identity::PreviewIdentity {
                version: 1,
                sha256: [0; 32],
                package_count: 0,
                other_member_count: 0,
                encoded_bytes: 0,
            },
        );
        owner.apply_attempt_serial = u64::MAX;
        unsafe extern "C" fn deny(
            _: *mut std::ffi::c_void,
            _: i32,
            _: *const std::ffi::c_char,
            _: *const std::ffi::c_char,
            _: *const std::ffi::c_char,
            _: *const std::ffi::c_char,
        ) -> i32 {
            rusqlite::ffi::SQLITE_DENY
        }
        let db = unsafe { owner.conn.as_ref().unwrap().handle() };
        unsafe {
            rusqlite::ffi::sqlite3_set_authorizer(db, Some(deny), std::ptr::null_mut());
        }
        let result = owner.apply_current_controlled(
            source,
            &shown,
            &OperationContext {
                gate: &gate,
                policy,
            },
            &mut NoArtifacts,
        );
        unsafe {
            rusqlite::ffi::sqlite3_set_authorizer(db, None, std::ptr::null_mut());
        }
        assert!(matches!(
            result.failure,
            Some(OrchestrationError::ApplyAttemptExhausted)
        ));
        assert_eq!(result.commit, CommitEvidence::NotCommitted);
        assert_eq!(result.artifacts, ArtifactDisposition::None);
        assert_eq!(owner.apply_attempt_serial, u64::MAX);
        assert!(owner.retained_apply_witness().is_none());
        assert_eq!(owner.generation, 0);
        assert_eq!(owner.health, WriterHealth::Healthy);
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
