//! One managed catalogue owner: incarnation, generation and health.
//!
//! Construction-only: no production route is wired here. The server-side
//! `Mutex<ManagedCatalogue>` wrapper, the full witness (§4) and the W/S/D copy
//! recipe (§6) are later increments. See
//! `docs/contracts/catalogue-jobs/managed-binding.md`.

use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};

use rusqlite::Connection;

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
pub(crate) enum WriterHealth {
    Healthy,
    /// Lock is owned; the guard has not yet proved safe settlement.
    Mutating,
    /// Deliberate close/restore/reopen in progress.
    Retired(RetireReason),
    /// No reads or writes through this handle.
    Quarantined(QuarantineReason),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RetireReason {
    Close,
    Reopen,
    Restore,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum QuarantineReason {
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
pub(crate) enum LegacyMutationKind {
    DirectPackage,
    LegacyPublish,
    PathUpdate,
    PlacementCleanup,
    BackupDelete,
}

/// A hint the mutation guard independently checks against native/health state;
/// never permission to claim a failed COMMIT rolled back.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LegacySettlementHint {
    SuccessfulReturn,
    PreCommitSemanticRefusal,
    Uncertain,
}

#[derive(Debug)]
pub(crate) enum OwnerError {
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
pub(crate) enum OwnerRestoreError {
    NotHealthy(WriterHealth),
    GenerationOverflow,
    Reopen(rusqlite::Error),
    Observation(String),
}

impl std::fmt::Display for OwnerRestoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OwnerRestoreError::NotHealthy(health) => {
                write!(f, "catalogue owner not healthy: {health:?}")
            }
            OwnerRestoreError::GenerationOverflow => write!(f, "catalogue generation overflow"),
            OwnerRestoreError::Reopen(e) => write!(f, "catalogue reopen failed: {e}"),
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
pub(crate) struct ManagedCatalogue {
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
    pub(crate) fn adopt_opened(
        conn: Connection,
        configured_path: PathBuf,
    ) -> Result<Self, OwnerError> {
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
    pub(crate) fn with_read<T, E>(
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
    pub(crate) fn with_legacy_mutation<T, E>(
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

    /// The dedicated restore adapter: retire and invalidate before touching the
    /// live handle, then adopt a fresh incarnation and healthy reopened handle
    /// only on success. Failure leaves the owner quarantined, never silently
    /// usable.
    pub(crate) fn restore_legacy(&mut self, listed_backup: &Path) -> Result<(), OwnerRestoreError> {
        if self.health != WriterHealth::Healthy {
            return Err(OwnerRestoreError::NotHealthy(self.health));
        }
        self.generation = self
            .generation
            .checked_add(1)
            .ok_or(OwnerRestoreError::GenerationOverflow)?;
        self.health = WriterHealth::Retired(RetireReason::Restore);
        self.conn = None;

        match Connection::open(listed_backup) {
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
                Err(OwnerRestoreError::Reopen(e))
            }
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    fn in_memory() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("CREATE TABLE t (id INTEGER PRIMARY KEY, v TEXT) STRICT;")
            .unwrap();
        conn
    }

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
        let (_dir, mut owner) = adopted();
        // Write through the owner so the restored file has the row.
        owner
            .with_legacy_mutation(
                LegacyMutationKind::DirectPackage,
                |conn| conn.execute("INSERT INTO t (id, v) VALUES (1, 'x')", []),
                |_: &Result<usize, rusqlite::Error>| LegacySettlementHint::SuccessfulReturn,
            )
            .unwrap()
            .unwrap();
        let backup_path = _dir.path().join("products.sqlite");
        let before = owner.incarnation;
        owner.restore_legacy(&backup_path).unwrap();
        assert_eq!(owner.health, WriterHealth::Healthy);
        assert_ne!(owner.incarnation, before);
    }

    #[test]
    fn a_failed_restore_leaves_the_owner_quarantined() {
        let (dir, mut owner) = adopted();
        // A directory cannot be opened as a database file.
        let not_a_db = dir.path().join("subdir");
        std::fs::create_dir(&not_a_db).unwrap();
        let err = owner.restore_legacy(&not_a_db).unwrap_err();
        assert!(matches!(err, OwnerRestoreError::Reopen(_)));
        assert!(matches!(
            owner.health,
            WriterHealth::Quarantined(QuarantineReason::UncertainSettlement)
        ));
    }
}
