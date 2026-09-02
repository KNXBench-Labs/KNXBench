//! Imports a `.knxproj` file and persists its opaque entries into an
//! already-migrated store.
//!
//! `knx-app` is the only crate that sees both `knx-etsproj` and
//! `knx-store` — `check-layering` does not enforce this directly (it only
//! forbids `knx-core` from reaching storage/format crates), but keeping
//! the `OpaqueEntry` → `StoredOpaqueEntry` conversion here, and nowhere
//! else, is what keeps `knx-etsproj` itself free of a storage dependency:
//! nothing in `knx-etsproj` needs to know `knx-store`'s row shape exists.

use std::path::Path;

use knx_etsproj::opaque::OpaqueEntry;
use knx_etsproj::{ImportFailure, ImportReport};
use knx_store::{insert_opaque, Connection, SqlError, StoredOpaqueEntry};

pub struct ImportedProject {
    pub project: knx_core::Project,
    pub report: ImportReport,
    pub opaque_entries: usize,
}

#[derive(Debug)]
pub enum AppError {
    Import(ImportFailure),
    Store(knx_store::MigrationError),
    Sql(SqlError),
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AppError::Import(e) => write!(f, "{e}"),
            AppError::Store(e) => write!(f, "{e}"),
            AppError::Sql(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for AppError {}

impl From<ImportFailure> for AppError {
    fn from(e: ImportFailure) -> Self {
        AppError::Import(e)
    }
}

impl From<knx_store::MigrationError> for AppError {
    fn from(e: knx_store::MigrationError) -> Self {
        AppError::Store(e)
    }
}

impl From<SqlError> for AppError {
    fn from(e: SqlError) -> Self {
        AppError::Sql(e)
    }
}

/// Imports `path` and persists every opaque entry into `conn`, which the
/// caller has already opened and migrated — the CLI decides whether that
/// connection is backed by a file or lives only for this run. The whole
/// insert runs inside one transaction (`knx_store::insert_opaque`, Task
/// 13), so a failure during import itself, which happens entirely before
/// the first row is inserted, leaves the store untouched; a failure
/// partway through the insert leaves it exactly as it was before this
/// call, not half-written.
pub fn import_ets_project(path: &Path, conn: &Connection) -> Result<ImportedProject, AppError> {
    let outcome = knx_etsproj::import_knxproj(path)?;

    let stored: Vec<StoredOpaqueEntry> = outcome.opaque.iter().map(to_stored).collect();
    let opaque_entries = stored.len();
    insert_opaque(conn, &stored)?;

    Ok(ImportedProject {
        project: outcome.project,
        report: outcome.report,
        opaque_entries,
    })
}

/// `kind` renders as the `OpaqueKind` variant name (`Debug`-formatted, the
/// same convention `report::opaque_summary` already uses for the same
/// enum) — never interpreted, never executed, just carried through as a
/// label alongside the bytes.
fn to_stored(e: &OpaqueEntry) -> StoredOpaqueEntry {
    StoredOpaqueEntry {
        source_path: e.source_path.clone(),
        xpath: e.xpath.clone(),
        kind: format!("{:?}", e.kind),
        name: e.name.clone(),
        bytes: e.bytes.clone(),
        sha256: e.sha256.clone(),
    }
}
