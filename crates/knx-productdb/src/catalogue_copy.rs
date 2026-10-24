//! Bounded copy of a pinned read-only source into one fresh destination.
//!
//! Implements the disk-copy half of the W/S/D recipe (managed-binding §6): a
//! read-only source S is copied into a fresh, equal-page-size destination D
//! with manual 128-page backup steps, a finite busy/locked retry budget and
//! persistent Stop checks, then the pinned fresh-zero destination-cookie rule
//! `0 -> 1` is enforced. Construction-only — no production route is wired and
//! no copy is accepted as a live Ready/apply witness.

use std::time::{Duration, Instant};

use crate::operation::{PublicationGate, Stop};
use rusqlite::backup::{Backup, StepResult};
use rusqlite::Connection;

/// Pages per backup step.
const COPY_PAGES_PER_STEP: i32 = 128;
/// Pause between busy/locked retries.
const COPY_BUSY_PAUSE: Duration = Duration::from_millis(50);
/// Total busy/locked retry budget.
const COPY_BUSY_BUDGET: Duration = Duration::from_secs(30);
/// Maximum admitted source extent: `page_count * page_size`.
pub(crate) const COPY_PAGE_BYTE_BOUND: u64 = 8 * 1024 * 1024 * 1024;

/// One observed completed copy: extent, step accounting and the destination
/// cookie receipt (`0` before, `1` after the pinned fresh-zero rule).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CopyDone {
    page_count: u64,
    page_size: u64,
    steps: u64,
    cookie_before: i64,
    cookie_after: i64,
}

#[derive(Debug)]
pub(crate) enum CopyError {
    // Share the existing typed error with its inventory owner; cloning or
    // consuming a projection must not discard independent native D facts.
    Destination(std::sync::Arc<destination::DestinationFailure>),
    NotReadOnly,
    NoActiveSnapshot,
    UnsupportedPageSize(i64),
    PageSizeMismatch { source: i64, destination: i64 },
    PageCountOverflow,
    DestinationNotFresh(i64),
    DestinationNonEmpty,
    DestinationInTransaction,
    BackupBusy,
    UnsupportedStep,
    CookieRule(i64),
    Observation(rusqlite::Error),
    Stop(Stop),
}

impl From<rusqlite::Error> for CopyError {
    fn from(error: rusqlite::Error) -> Self {
        Self::Observation(error)
    }
}

impl From<Stop> for CopyError {
    fn from(stop: Stop) -> Self {
        Self::Stop(stop)
    }
}

impl std::fmt::Display for CopyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CopyError::Destination(error) => write!(f, "disk destination refused: {error:?}"),
            CopyError::NotReadOnly => write!(f, "copy source is not read-only"),
            CopyError::NoActiveSnapshot => write!(f, "copy source has no active read snapshot"),
            CopyError::UnsupportedPageSize(size) => {
                write!(f, "copy source page size unsupported: {size}")
            }
            CopyError::PageSizeMismatch {
                source,
                destination,
            } => write!(
                f,
                "copy page size mismatch: source {source}, destination {destination}"
            ),
            CopyError::PageCountOverflow => {
                write!(f, "copy source extent exceeds the admission bound")
            }
            CopyError::DestinationNotFresh(cookie) => {
                write!(f, "copy destination is not fresh (cookie {cookie})")
            }
            CopyError::DestinationNonEmpty => write!(f, "copy destination schema is not empty"),
            CopyError::DestinationInTransaction => {
                write!(f, "copy destination has an active transaction")
            }
            CopyError::BackupBusy => write!(f, "copy backup stayed busy beyond the retry budget"),
            CopyError::UnsupportedStep => write!(f, "copy backup returned an unknown step result"),
            CopyError::CookieRule(cookie) => {
                write!(f, "copy destination cookie rule 0 -> 1 violated: {cookie}")
            }
            CopyError::Observation(e) => write!(f, "copy observation error: {e}"),
            CopyError::Stop(_) => write!(f, "copy stopped"),
        }
    }
}

impl std::error::Error for CopyError {}

fn is_supported_page_size(size: i64) -> bool {
    matches!(
        size,
        512 | 1024 | 2048 | 4096 | 8192 | 16384 | 32768 | 65536
    )
}

fn scalar(conn: &Connection, sql: &str) -> Result<i64, rusqlite::Error> {
    conn.query_row(sql, [], |row| row.get(0))
}

/// Copies a pinned read-only source into one fresh destination with bounded
/// backup steps and the fresh-zero destination-cookie rule.
pub(crate) fn copy_pinned_current(
    source: &Connection,
    destination: &mut Connection,
    gate: &PublicationGate,
) -> Result<CopyDone, CopyError> {
    gate.checkpoint()?;

    // The source must be a read-only connection (query_only) with an active
    // read snapshot; a write-capable W is rejected before Backup::new.
    let query_only: i64 = scalar(source, "PRAGMA query_only")?;
    if query_only != 1 {
        return Err(CopyError::NotReadOnly);
    }
    if source.is_autocommit() {
        return Err(CopyError::NoActiveSnapshot);
    }

    let page_size: i64 = scalar(source, "PRAGMA page_size")?;
    let page_count: i64 = scalar(source, "PRAGMA page_count")?;
    if !is_supported_page_size(page_size) {
        return Err(CopyError::UnsupportedPageSize(page_size));
    }
    let bytes = (page_count as u64)
        .checked_mul(page_size as u64)
        .ok_or(CopyError::PageCountOverflow)?;
    if bytes > COPY_PAGE_BYTE_BOUND {
        return Err(CopyError::PageCountOverflow);
    }

    let destination_page_size: i64 = scalar(destination, "PRAGMA page_size")?;
    if destination_page_size != page_size {
        return Err(CopyError::PageSizeMismatch {
            source: page_size,
            destination: destination_page_size,
        });
    }
    let cookie_before: i64 = scalar(destination, "PRAGMA schema_version")?;
    if cookie_before != 0 {
        return Err(CopyError::DestinationNotFresh(cookie_before));
    }
    let object_count: i64 = scalar(destination, "SELECT count(*) FROM main.sqlite_schema")?;
    if object_count != 0 {
        return Err(CopyError::DestinationNonEmpty);
    }
    if !destination.is_autocommit() {
        return Err(CopyError::DestinationInTransaction);
    }

    let mut steps: u64 = 0;
    let mut busy_budget = COPY_BUSY_BUDGET;
    let start = Instant::now();
    {
        let backup = Backup::new(source, destination)?;
        loop {
            gate.checkpoint()?;
            let result = backup.step(COPY_PAGES_PER_STEP)?;
            steps = steps.checked_add(1).ok_or(CopyError::PageCountOverflow)?;
            match result {
                StepResult::Done => break,
                StepResult::More => continue,
                StepResult::Busy | StepResult::Locked => {
                    if busy_budget.is_zero() || start.elapsed() >= COPY_BUSY_BUDGET {
                        return Err(CopyError::BackupBusy);
                    }
                    let pause = COPY_BUSY_PAUSE.min(busy_budget);
                    busy_budget = busy_budget.saturating_sub(pause);
                    std::thread::sleep(pause);
                }
                _ => return Err(CopyError::UnsupportedStep),
            }
        }
    }

    let cookie_after: i64 = scalar(destination, "PRAGMA schema_version")?;
    if cookie_after != 1 {
        return Err(CopyError::CookieRule(cookie_after));
    }

    Ok(CopyDone {
        page_count: page_count as u64,
        page_size: page_size as u64,
        steps,
        cookie_before,
        cookie_after,
    })
}

mod destination;
pub(crate) use destination::{
    ArtifactInventory, CopyFacts, DestinationRole, FreshDestination, RehearsalDestination,
    ValidatedDestination,
};
#[cfg(test)]
pub(crate) use destination::{
    DestinationCleanup, DestinationError, DestinationFailure, DestinationStage,
};
pub use destination::{PublishedRecovery, RecoveryPublicationError, ValidatedRecovery};

#[cfg(test)]
mod tests;
