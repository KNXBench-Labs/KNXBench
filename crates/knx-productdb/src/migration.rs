//! `products.sqlite`'s own schema and migration chain, keyed off SQLite's
//! `user_version` pragma.
//!
//! Deliberately not `knx-store`'s chain: a project-schema bump must not
//! force a product-database migration, or the other way round (ADR-0005,
//! ADR-0011). The two databases have different lifetimes — a project file
//! is per project, this one is shared across all of them.

use std::fmt;
use std::path::{Path, PathBuf};

use rusqlite::Connection;

/// The product-database schema version this build writes.
pub const CURRENT_PRODUCTDB_VERSION: i64 = 1;

#[derive(Debug)]
pub enum ProductDbError {
    Sqlite(rusqlite::Error),
    Xml { source_path: String, cause: String },
    FutureVersion { found: i64, supported: i64 },
}

impl std::error::Error for ProductDbError {}

impl fmt::Display for ProductDbError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProductDbError::Sqlite(e) => write!(f, "{e}"),
            ProductDbError::Xml { source_path, cause } => {
                write!(f, "{source_path}: {cause}")
            }
            ProductDbError::FutureVersion { found, supported } => write!(
                f,
                "product database is version {found}, this build supports up to {supported} — no downgrade path exists"
            ),
        }
    }
}

impl From<rusqlite::Error> for ProductDbError {
    fn from(e: rusqlite::Error) -> Self {
        ProductDbError::Sqlite(e)
    }
}

/// v0 -> v1. Extended in place while this plan runs, because the database
/// has no released state yet; frozen once the plan's last task lands.
fn migrate_v0_to_v1(conn: &Connection) -> Result<(), ProductDbError> {
    conn.execute_batch(
        "CREATE TABLE schema_meta (key TEXT PRIMARY KEY, value TEXT NOT NULL) STRICT;
         INSERT INTO schema_meta (key, value) VALUES ('created_by', 'knx-productdb');
         CREATE TABLE source_file (
             sha256          TEXT PRIMARY KEY,
             source_path     TEXT NOT NULL,
             manufacturer_id TEXT,
             len             INTEGER NOT NULL,
             bytes           BLOB NOT NULL
         ) STRICT;
         CREATE INDEX source_file_manufacturer ON source_file (manufacturer_id);
         CREATE TABLE ingest_unknown (
             id            INTEGER PRIMARY KEY,
             source_sha256 TEXT NOT NULL,
             program_id    TEXT,
             xpath         TEXT NOT NULL,
             kind          TEXT NOT NULL,
             name          TEXT NOT NULL,
             occurrences   INTEGER NOT NULL,
             sample        TEXT
         ) STRICT;
         CREATE INDEX ingest_unknown_source ON ingest_unknown (source_sha256);",
    )?;
    Ok(())
}

type Migration = fn(&Connection) -> Result<(), ProductDbError>;

fn migrations() -> Vec<Migration> {
    vec![migrate_v0_to_v1]
}

/// Opens (creating if absent) the product database at `path`, runs every
/// pending migration in order, and returns the connection at
/// `CURRENT_PRODUCTDB_VERSION`. Creates the parent directory, since the
/// default path lives under a data directory the user may not have yet.
pub fn open_and_migrate(path: &Path) -> Result<Connection, ProductDbError> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent).map_err(|e| ProductDbError::Xml {
                source_path: parent.display().to_string(),
                cause: e.to_string(),
            })?;
        }
    }
    let conn = Connection::open(path)?;
    let found: i64 = conn.query_row("PRAGMA user_version", [], |row| row.get(0))?;
    if found > CURRENT_PRODUCTDB_VERSION {
        return Err(ProductDbError::FutureVersion {
            found,
            supported: CURRENT_PRODUCTDB_VERSION,
        });
    }
    for migration in &migrations()[found as usize..CURRENT_PRODUCTDB_VERSION as usize] {
        migration(&conn)?;
    }
    if found < CURRENT_PRODUCTDB_VERSION {
        conn.pragma_update(None, "user_version", CURRENT_PRODUCTDB_VERSION)?;
    }
    Ok(conn)
}

/// `$XDG_DATA_HOME/knx/products.sqlite`, falling back to
/// `$HOME/.local/share/knx/products.sqlite`. `None` when neither variable
/// is set, which the caller reports rather than guessing a location.
pub fn default_path() -> Option<PathBuf> {
    if let Some(dir) = std::env::var_os("XDG_DATA_HOME").filter(|v| !v.is_empty()) {
        return Some(PathBuf::from(dir).join("knx").join("products.sqlite"));
    }
    let home = std::env::var_os("HOME").filter(|v| !v.is_empty())?;
    Some(
        PathBuf::from(home)
            .join(".local")
            .join("share")
            .join("knx")
            .join("products.sqlite"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fresh_file_migrates_to_current_version() {
        let dir = tempfile::tempdir().unwrap();
        let conn = open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
        let version: i64 = conn
            .query_row("PRAGMA user_version", [], |r| r.get(0))
            .unwrap();
        assert_eq!(version, CURRENT_PRODUCTDB_VERSION);
        let marker: String = conn
            .query_row(
                "SELECT value FROM schema_meta WHERE key = 'created_by'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(marker, "knx-productdb");
    }

    #[test]
    fn reopening_an_already_migrated_file_is_a_no_op() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("products.sqlite");
        open_and_migrate(&path).unwrap();
        let conn = open_and_migrate(&path).unwrap();
        let version: i64 = conn
            .query_row("PRAGMA user_version", [], |r| r.get(0))
            .unwrap();
        assert_eq!(version, CURRENT_PRODUCTDB_VERSION);
    }

    #[test]
    fn a_file_from_a_newer_version_is_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("products.sqlite");
        {
            let conn = rusqlite::Connection::open(&path).unwrap();
            conn.pragma_update(None, "user_version", CURRENT_PRODUCTDB_VERSION + 1)
                .unwrap();
        }
        assert!(matches!(
            open_and_migrate(&path),
            Err(ProductDbError::FutureVersion { .. })
        ));
    }

    #[test]
    fn default_path_sits_under_the_xdg_data_directory() {
        // `default_path` reads the environment; assert its shape, not a
        // machine-specific absolute path.
        let p = default_path().expect("HOME or XDG_DATA_HOME is set in CI");
        assert!(p.ends_with("knx/products.sqlite"), "{}", p.display());
    }
}
