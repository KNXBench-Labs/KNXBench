//! Previewed, recoverable catalogue updates from manufacturer files (ADR-0117).
//!
//! A preview applies the exact installation to an in-memory copy of the
//! product database and reports what would change per application program.
//! Adoption re-checks the preview's token, writes a backup with SQLite's
//! online backup API and then runs the ordinary transactional install. A
//! restore copies a validated backup back the same way. Nothing here reaches
//! the network: the package is a file the user obtained from the
//! manufacturer.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::time::Duration;

use rusqlite::{Connection, OpenFlags};
use serde::Serialize;

use crate::blob::sha256_hex;
use crate::migration::{writable_memory_copy, READ_ONLY_BUSY_TIMEOUT};
use crate::{
    install_package_with_limits, open_and_migrate, InstallReport, PackageError, PackageLimits,
    ProductDbError, CURRENT_PRODUCTDB_VERSION,
};

/// Pause between online-backup steps when the source is busy.
const BACKUP_RETRY: Duration = Duration::from_millis(20);

/// What installing a package would do to one of its application programs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ProgramUpdateStatus {
    /// No program of this manufacturer and application number is installed.
    New,
    /// Other versions of the same application are installed; this one is
    /// added beside them and no existing binding changes.
    NewVersion,
    /// This exact program ID is already installed.
    AlreadyInstalled,
}

/// An installed program sharing manufacturer and application number.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstalledVersion {
    pub id: String,
    pub application_version: Option<String>,
}

/// One application program of the package.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProgramUpdate {
    pub id: String,
    pub manufacturer_id: String,
    pub name: Option<String>,
    pub application_number: Option<String>,
    pub application_version: Option<String>,
    pub status: ProgramUpdateStatus,
    pub installed_versions: Vec<InstalledVersion>,
}

/// The outcome of [`preview_package_update`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdatePreview {
    pub source_name: String,
    pub package_sha256: String,
    pub package_already_installed: bool,
    pub programs: Vec<ProgramUpdate>,
    pub products_added: usize,
    pub hardware2programs_added: usize,
    pub id_conflicts: usize,
    /// Hash of the installed package and source-file set before the update.
    pub catalogue_fingerprint: String,
    /// Binds adoption to this preview and catalogue state.
    pub token: String,
}

/// The outcome of [`apply_package_update`].
#[derive(Debug)]
pub struct AppliedUpdate {
    pub preview: UpdatePreview,
    /// A full copy of the product database taken right before the install.
    pub backup: PathBuf,
    pub report: InstallReport,
}

/// Why an update or restore was refused.
#[derive(Debug)]
pub enum UpdateError {
    Package(PackageError),
    Database(ProductDbError),
    Io(String),
    /// The catalogue or the package no longer matches the preview's token.
    StaleToken,
    /// The file offered for a restore is not a usable product database.
    InvalidBackup(String),
}

impl std::fmt::Display for UpdateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Package(error) => write!(f, "{error}"),
            Self::Database(error) => write!(f, "{error}"),
            Self::Io(error) => write!(f, "{error}"),
            Self::StaleToken => write!(
                f,
                "the product database or the package changed since the preview; preview again"
            ),
            Self::InvalidBackup(why) => write!(f, "not a usable product database backup: {why}"),
        }
    }
}

impl std::error::Error for UpdateError {}

impl From<ProductDbError> for UpdateError {
    fn from(error: ProductDbError) -> Self {
        Self::Database(error)
    }
}

impl From<rusqlite::Error> for UpdateError {
    fn from(error: rusqlite::Error) -> Self {
        Self::Database(ProductDbError::from(error))
    }
}

/// Hash of what is installed: schema version, package and source-file sets.
fn catalogue_fingerprint(conn: &Connection) -> Result<String, UpdateError> {
    let version: i64 = conn.query_row("PRAGMA user_version", [], |row| row.get(0))?;
    let column = |sql: &str| -> Result<Vec<String>, UpdateError> {
        let mut statement = conn.prepare(sql)?;
        let values = statement
            .query_map([], |row| row.get(0))?
            .collect::<Result<Vec<String>, _>>()?;
        Ok(values)
    };
    let packages = column("SELECT sha256 FROM package ORDER BY sha256")?;
    let sources = column("SELECT sha256 FROM source_file ORDER BY sha256")?;
    Ok(sha256_hex(
        format!(
            "productdb-catalogue-v1\n{version}\n{}\n{}",
            packages.join(","),
            sources.join(",")
        )
        .as_bytes(),
    ))
}

fn count(conn: &Connection, table: &str) -> Result<usize, UpdateError> {
    let n: i64 = conn.query_row(&format!("SELECT count(*) FROM {table}"), [], |row| {
        row.get(0)
    })?;
    Ok(usize::try_from(n).unwrap_or(0))
}

struct ProgramRow {
    id: String,
    manufacturer_id: String,
    name: Option<String>,
    application_number: Option<String>,
    application_version: Option<String>,
}

fn program_rows(
    conn: &Connection,
    sql: &str,
    package: Option<&str>,
) -> Result<Vec<ProgramRow>, UpdateError> {
    let mut statement = conn.prepare(sql)?;
    let map = |row: &rusqlite::Row<'_>| {
        Ok(ProgramRow {
            id: row.get(0)?,
            manufacturer_id: row.get(1)?,
            name: row.get(2)?,
            application_number: row.get(3)?,
            application_version: row.get(4)?,
        })
    };
    let rows = match package {
        Some(sha256) => statement
            .query_map([sha256], map)?
            .collect::<Result<Vec<_>, _>>()?,
        None => statement
            .query_map([], map)?
            .collect::<Result<Vec<_>, _>>()?,
    };
    Ok(rows)
}

const ALL_PROGRAMS: &str =
    "SELECT id, manufacturer_id, name, application_number, application_version
     FROM application_program ORDER BY id";
const PACKAGE_PROGRAMS: &str = "SELECT DISTINCT ap.id, ap.manufacturer_id, ap.name,
            ap.application_number, ap.application_version
     FROM application_program ap
     JOIN package_member pm ON pm.source_sha256 = ap.source_sha256
     WHERE pm.package_sha256 = ?1
     ORDER BY ap.id";

/// Reports what installing `bytes` would change, without writing `path`.
pub fn preview_package_update(
    path: &Path,
    source_name: &str,
    bytes: &[u8],
    limits: PackageLimits,
) -> Result<UpdatePreview, UpdateError> {
    let conn = writable_memory_copy(path)?;
    let catalogue_fingerprint = catalogue_fingerprint(&conn)?;
    let before = program_rows(&conn, ALL_PROGRAMS, None)?;
    let before_ids: BTreeSet<&str> = before.iter().map(|row| row.id.as_str()).collect();
    let products_before = count(&conn, "product")?;
    let links_before = count(&conn, "hardware2program")?;

    let report = install_package_with_limits(&conn, source_name, bytes, limits)
        .map_err(UpdateError::Package)?;

    let programs = program_rows(&conn, PACKAGE_PROGRAMS, Some(&report.sha256))?
        .into_iter()
        .map(|row| {
            let installed_versions: Vec<InstalledVersion> = before
                .iter()
                .filter(|old| {
                    old.id != row.id
                        && old.manufacturer_id == row.manufacturer_id
                        && old.application_number.is_some()
                        && old.application_number == row.application_number
                })
                .map(|old| InstalledVersion {
                    id: old.id.clone(),
                    application_version: old.application_version.clone(),
                })
                .collect();
            let status = if before_ids.contains(row.id.as_str()) {
                ProgramUpdateStatus::AlreadyInstalled
            } else if installed_versions.is_empty() {
                ProgramUpdateStatus::New
            } else {
                ProgramUpdateStatus::NewVersion
            };
            ProgramUpdate {
                id: row.id,
                manufacturer_id: row.manufacturer_id,
                name: row.name,
                application_number: row.application_number,
                application_version: row.application_version,
                status,
                installed_versions,
            }
        })
        .collect::<Vec<_>>();
    let products_added = count(&conn, "product")?.saturating_sub(products_before);
    let hardware2programs_added = count(&conn, "hardware2program")?.saturating_sub(links_before);
    let token = sha256_hex(
        format!(
            "catalogue-update-v1\n{catalogue_fingerprint}\n{}\n{source_name}\n{programs:?}\n{products_added}\n{hardware2programs_added}",
            report.sha256
        )
        .as_bytes(),
    );
    Ok(UpdatePreview {
        source_name: source_name.to_string(),
        package_sha256: report.sha256.clone(),
        package_already_installed: report.skipped,
        programs,
        products_added,
        hardware2programs_added,
        id_conflicts: report.conflicts.len(),
        catalogue_fingerprint,
        token,
    })
}

/// A new file name next to `path` for a backup taken before installing `sha256`.
fn backup_path(path: &Path, sha256: &str) -> Result<PathBuf, UpdateError> {
    let seconds = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|elapsed| elapsed.as_secs())
        .unwrap_or(0);
    let stem = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| "products.sqlite".into());
    let short = sha256.get(..12).unwrap_or(sha256);
    for attempt in 0..1000u32 {
        let suffix = if attempt == 0 {
            String::new()
        } else {
            format!("-{attempt}")
        };
        let candidate =
            path.with_file_name(format!("{stem}.backup-{seconds}-{short}{suffix}.sqlite"));
        if !candidate.exists() {
            return Ok(candidate);
        }
    }
    Err(UpdateError::Io("no free backup file name".into()))
}

/// Copies the database at `from` into the new file `to` with the online backup API.
fn write_backup(from: &Path, to: &Path) -> Result<(), UpdateError> {
    let flags = OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX;
    let source = Connection::open_with_flags(from, flags)?;
    source.busy_timeout(READ_ONLY_BUSY_TIMEOUT)?;
    let mut target = Connection::open_with_flags(
        to,
        OpenFlags::SQLITE_OPEN_READ_WRITE
            | OpenFlags::SQLITE_OPEN_CREATE
            | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )?;
    let backup = rusqlite::backup::Backup::new(&source, &mut target)?;
    backup.run_to_completion(i32::MAX, BACKUP_RETRY, None)?;
    Ok(())
}

/// Installs `bytes` when `token` still matches a fresh preview, after a backup.
pub fn apply_package_update(
    path: &Path,
    source_name: &str,
    bytes: &[u8],
    limits: PackageLimits,
    token: &str,
) -> Result<AppliedUpdate, UpdateError> {
    let preview = preview_package_update(path, source_name, bytes, limits)?;
    if preview.token != token {
        return Err(UpdateError::StaleToken);
    }
    // A missing database is created first, so the backup records the empty
    // catalogue; an existing file is copied before anything migrates it.
    if !path.exists() {
        drop(open_and_migrate(path)?);
    }
    let backup = backup_path(path, &preview.package_sha256)?;
    write_backup(path, &backup)?;
    let conn = open_and_migrate(path)?;
    let report = install_package_with_limits(&conn, source_name, bytes, limits)
        .map_err(UpdateError::Package)?;
    Ok(AppliedUpdate {
        preview,
        backup,
        report,
    })
}

/// Replaces the product database at `path` with a validated `backup`.
pub fn restore_product_db_backup(path: &Path, backup: &Path) -> Result<(), UpdateError> {
    let invalid = |error: rusqlite::Error| UpdateError::InvalidBackup(error.to_string());
    let flags = OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX;
    let source = Connection::open_with_flags(backup, flags).map_err(invalid)?;
    let check: String = source
        .query_row("PRAGMA quick_check", [], |row| row.get(0))
        .map_err(invalid)?;
    if check != "ok" {
        return Err(UpdateError::InvalidBackup(format!(
            "integrity check: {check}"
        )));
    }
    let version: i64 = source
        .query_row("PRAGMA user_version", [], |row| row.get(0))
        .map_err(invalid)?;
    if !(1..=CURRENT_PRODUCTDB_VERSION).contains(&version) {
        return Err(UpdateError::InvalidBackup(format!(
            "product database version {version} is not between 1 and {CURRENT_PRODUCTDB_VERSION}"
        )));
    }
    let tables: i64 = source
        .query_row(
            "SELECT count(*) FROM sqlite_master WHERE type = 'table'
             AND name IN ('application_program', 'package', 'source_file')",
            [],
            |row| row.get(0),
        )
        .map_err(invalid)?;
    if tables != 3 {
        return Err(UpdateError::InvalidBackup(
            "missing catalogue tables".into(),
        ));
    }
    let mut target = Connection::open(path)?;
    target.busy_timeout(READ_ONLY_BUSY_TIMEOUT)?;
    let restore = rusqlite::backup::Backup::new(&source, &mut target)?;
    restore.run_to_completion(i32::MAX, BACKUP_RETRY, None)?;
    Ok(())
}
