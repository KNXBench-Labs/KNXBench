//! The immutable binding a future preview/apply compares: one catalogue stamp.
//!
//! A stamp captures the actual managed-handle values at one snapshot —
//! incarnation, generation, connection-local `data_version`, conservative
//! `total_changes`, the file stamp and the full live witness — never a global
//! latest value overwritten by a later read. Construction-only: no production
//! route is wired here. See
//! `docs/contracts/catalogue-jobs/managed-binding.md` §3 and
//! `stamp-binding-increment.md`.

use crate::catalogue_schema::{CatalogueWitness, WitnessError};
use crate::managed_catalogue::{CatalogueIncarnation, FileStamp, WriterHealth};

/// One immutable snapshot of the managed catalogue, bound to a full witness.
///
/// All fields are the actual W values observed at capture, so a later capture
/// after any committed change (or managed same-W mutation, via `generation`)
/// yields a different stamp. Equality is exact field equality, never a partial
/// summary or a reset baseline.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CatalogueStamp {
    pub(crate) incarnation: CatalogueIncarnation,
    pub(crate) generation: u64,
    pub(crate) data_version: i64,
    pub(crate) total_changes: u64,
    pub(crate) file: Option<FileStamp>,
    pub(crate) witness: CatalogueWitness,
    pub(crate) configuration: WriterConfigurationReceipt,
}

/// Actual connection-local variable settings, bound rather than repaired.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct WriterConfigurationReceipt {
    trusted_schema: bool,
    recursive_triggers: bool,
}
impl WriterConfigurationReceipt {
    pub(crate) fn read(conn: &crate::Connection) -> rusqlite::Result<Self> {
        fn boolean(conn: &crate::Connection, sql: &str) -> rusqlite::Result<bool> {
            match conn.query_row(sql, [], |row| row.get::<_, i64>(0))? {
                0 => Ok(false),
                1 => Ok(true),
                _ => Err(rusqlite::Error::InvalidQuery),
            }
        }
        Ok(Self {
            trusted_schema: boolean(conn, "PRAGMA trusted_schema")?,
            recursive_triggers: boolean(conn, "PRAGMA recursive_triggers")?,
        })
    }
}

/// Typed failures of a stamp acquisition; never a string and never a partial
/// digest.
#[derive(Debug)]
pub(crate) enum StampError {
    /// The managed handle is not healthy enough for a reservation.
    NotHealthy(WriterHealth),
    /// The connection is absent (retired/reopening).
    MissingConnection,
    /// A SQLite error while reserving, reading, sampling or rolling back.
    Sqlite(rusqlite::Error),
    /// The witness stream refused or stopped.
    Witness(WitnessError),
    /// The reservation changed underneath the snapshot or did not roll back.
    Observation(&'static str),
}

impl From<rusqlite::Error> for StampError {
    fn from(error: rusqlite::Error) -> Self {
        StampError::Sqlite(error)
    }
}

impl From<WitnessError> for StampError {
    fn from(error: WitnessError) -> Self {
        StampError::Witness(error)
    }
}

impl std::fmt::Display for StampError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StampError::NotHealthy(health) => {
                write!(f, "catalogue stamp refused: owner not healthy: {health:?}")
            }
            StampError::MissingConnection => {
                write!(f, "catalogue stamp refused: connection absent")
            }
            StampError::Sqlite(e) => write!(f, "catalogue stamp sqlite error: {e}"),
            StampError::Witness(e) => write!(f, "catalogue stamp witness error: {e}"),
            StampError::Observation(why) => write!(f, "catalogue stamp observation failed: {why}"),
        }
    }
}

impl std::error::Error for StampError {}
