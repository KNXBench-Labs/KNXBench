//! Explicit, preview-bound removal of catalogue placements.
//!
//! Canonical products, programs, original sources and project bindings are
//! deliberately retained, so existing devices keep resolving.

use std::collections::{BTreeSet, HashMap};
use std::path::{Path, PathBuf};

use rusqlite::{Connection, TransactionBehavior};
use sha2::{Digest, Sha256};

use crate::query::{catalog_items_by_id, CatalogItemRow};
use crate::update::{backup_path, write_backup};
use crate::{ProductDbError, UpdateError};

#[derive(Debug)]
pub struct CleanupPreview {
    pub token: String,
    pub items: Vec<CatalogItemRow>,
    /// Selected IDs whose exact product + Hardware2Program pair keeps no
    /// placement once the whole selection is gone. Devices bound to that pair
    /// still resolve their product and application, but no longer show a
    /// catalogue item name or number (`query::device_product`).
    pub bindings_without_placement: Vec<String>,
}

#[derive(Debug)]
pub struct AppliedCleanup {
    pub removed_item_ids: Vec<String>,
    pub backup: PathBuf,
}

#[derive(Debug)]
pub enum CleanupError {
    InvalidSelection(String),
    ConfirmationRequired,
    StaleToken,
    /// Cleanup writes a backup next to the database file; a database
    /// without one (in memory) is refused before anything is read.
    NotFileBacked,
    Database(ProductDbError),
    Backup(UpdateError),
}

impl std::fmt::Display for CleanupError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidSelection(why) => write!(f, "invalid catalogue selection: {why}"),
            Self::ConfirmationRequired => {
                write!(f, "catalogue cleanup needs explicit confirmation")
            }
            Self::StaleToken => write!(f, "the catalogue changed since the preview; preview again"),
            Self::NotFileBacked => write!(f, "catalogue cleanup needs a file-backed database"),
            Self::Database(error) => write!(f, "{error}"),
            Self::Backup(error) => write!(f, "the catalogue backup could not be written: {error}"),
        }
    }
}

impl std::error::Error for CleanupError {}

impl From<rusqlite::Error> for CleanupError {
    fn from(error: rusqlite::Error) -> Self {
        Self::Database(error.into())
    }
}

impl From<ProductDbError> for CleanupError {
    fn from(error: ProductDbError) -> Self {
        Self::Database(error)
    }
}

fn selection(item_ids: &[String]) -> Result<BTreeSet<&str>, CleanupError> {
    if item_ids.is_empty() {
        return Err(CleanupError::InvalidSelection(
            "select at least one item".into(),
        ));
    }
    let mut selected = BTreeSet::new();
    for id in item_ids {
        if id.is_empty() || id.trim() != id || id.chars().any(char::is_control) {
            return Err(CleanupError::InvalidSelection("malformed item ID".into()));
        }
        if !selected.insert(id.as_str()) {
            return Err(CleanupError::InvalidSelection("duplicate item ID".into()));
        }
    }
    Ok(selected)
}

fn database_path(conn: &Connection) -> Result<PathBuf, CleanupError> {
    conn.path()
        .filter(|path| !path.is_empty())
        .map(PathBuf::from)
        .ok_or(CleanupError::NotFileBacked)
}

fn bind_query(conn: &Connection, hash: &mut Sha256, sql: &str) -> Result<(), CleanupError> {
    hash.update(sql.as_bytes());
    let mut statement = conn.prepare(sql)?;
    let columns = statement.column_count();
    let mut rows = statement.query([])?;
    while let Some(row) = rows.next()? {
        for index in 0..columns {
            // ValueRef's typed, escaped Debug representation is unambiguous;
            // these metadata statements never select original package blobs.
            hash.update(format!("{:?};", row.get_ref(index)?).as_bytes());
        }
        hash.update(b"\n");
    }
    Ok(())
}

/// Also bind the physical database incarnation. Content alone would re-admit
/// an old confirmation after an exact restore (ABA). SQLite change counters
/// cover same-connection/WAL writes; file change times survive connection reopen
/// and distinguish a restore or replacement with otherwise identical contents.
fn bind_incarnation(conn: &Connection, hash: &mut Sha256, path: &Path) -> Result<(), CleanupError> {
    hash.update(conn.total_changes().to_le_bytes());
    bind_query(conn, hash, "PRAGMA data_version")?;
    for file in [
        path.to_path_buf(),
        PathBuf::from(format!("{}-wal", path.display())),
    ] {
        let metadata = match std::fs::metadata(&file) {
            Ok(metadata) => Some(metadata),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
            Err(error) => return Err(CleanupError::Backup(UpdateError::Io(error.to_string()))),
        };
        hash.update([u8::from(metadata.is_some())]);
        if let Some(metadata) = metadata {
            #[cfg(unix)]
            {
                use std::os::unix::fs::MetadataExt;
                hash.update(
                    format!(
                        "{}:{}:{}:{}:{}:{}:{}",
                        metadata.dev(),
                        metadata.ino(),
                        metadata.len(),
                        metadata.mtime(),
                        metadata.mtime_nsec(),
                        metadata.ctime(),
                        metadata.ctime_nsec()
                    )
                    .as_bytes(),
                );
            }
            #[cfg(not(unix))]
            hash.update(
                format!(
                    "{}:{:?}:{:?}",
                    metadata.len(),
                    metadata.modified(),
                    metadata.created()
                )
                .as_bytes(),
            );
        }
    }
    Ok(())
}

/// What a selected ID that names no placement means.
#[derive(Clone, Copy)]
enum Unknown {
    /// A preview: the caller sent an ID the catalogue never had.
    Refuse,
    /// An apply: the preview found it, so the catalogue changed since.
    Stale,
}

fn preview_in_snapshot(
    conn: &Connection,
    item_ids: &[String],
    unknown: Unknown,
) -> Result<CleanupPreview, CleanupError> {
    let path = database_path(conn)?;
    let selected = selection(item_ids)?;
    let ids: Vec<&str> = selected.iter().copied().collect();
    let items = catalog_items_by_id(conn, &ids)?;
    if items.len() != ids.len() {
        let found: BTreeSet<&str> = items.iter().map(|item| item.id.as_str()).collect();
        let missing = ids
            .iter()
            .find(|id| !found.contains(**id))
            .copied()
            .unwrap_or_default();
        return Err(match unknown {
            Unknown::Refuse => {
                CleanupError::InvalidSelection(format!("unknown item ID: {missing}"))
            }
            Unknown::Stale => CleanupError::StaleToken,
        });
    }
    let mut hash = Sha256::new();
    hash.update(b"catalogue-cleanup-v1\n");
    hash.update(format!("{selected:?}\n").as_bytes());
    for sql in [
        "PRAGMA user_version",
        "PRAGMA schema_version",
        "SELECT type, name, tbl_name, sql FROM sqlite_schema ORDER BY type, name",
        "SELECT * FROM catalog_item ORDER BY id",
        "SELECT * FROM catalog_section ORDER BY id",
        "SELECT * FROM hardware2program ORDER BY id",
        "SELECT * FROM application_program ORDER BY id",
        "SELECT package_sha256, path, source_sha256 FROM package_member ORDER BY package_sha256, path",
        "SELECT package_sha256, source_name FROM package_source_name ORDER BY package_sha256, source_name",
        "SELECT sha256, source_name FROM package ORDER BY sha256",
        "SELECT sha256, source_path FROM source_file ORDER BY sha256",
    ] {
        bind_query(conn, &mut hash, sql)?;
    }
    bind_incarnation(conn, &mut hash, &path)?;
    let token = hash.finalize().iter().map(|b| format!("{b:02x}")).collect();
    let bindings_without_placement = bindings_without_placement(conn, &selected, &items)?;
    Ok(CleanupPreview {
        token,
        items,
        bindings_without_placement,
    })
}

/// A device finds its catalogue entry by the exact pair, so only rows naming
/// both references can leave a binding behind; the token already binds the
/// whole `catalog_item` table this count is taken from. One pass over the
/// table, whatever the size of the selection.
fn bindings_without_placement(
    conn: &Connection,
    selected: &BTreeSet<&str>,
    items: &[CatalogItemRow],
) -> Result<Vec<String>, CleanupError> {
    let mut survivors: HashMap<(String, String), usize> = HashMap::new();
    let mut statement = conn.prepare(
        "SELECT id, product_ref_id, hardware2program_ref_id FROM catalog_item
         WHERE product_ref_id IS NOT NULL AND hardware2program_ref_id IS NOT NULL",
    )?;
    let mut rows = statement.query([])?;
    while let Some(row) = rows.next()? {
        let id: String = row.get(0)?;
        let count = survivors.entry((row.get(1)?, row.get(2)?)).or_default();
        if !selected.contains(id.as_str()) {
            *count += 1;
        }
    }
    Ok(items
        .iter()
        .filter(
            |item| match (&item.product_ref_id, &item.hardware2program_ref_id) {
                (Some(product), Some(binding)) => {
                    survivors.get(&(product.clone(), binding.clone())) == Some(&0)
                }
                _ => false,
            },
        )
        .map(|item| item.id.clone())
        .collect())
}

/// Reads one consistent snapshot without creating/migrating a database or
/// recording preview state. Sorting selections makes repeated previews stable.
pub fn preview_catalog_cleanup(
    conn: &Connection,
    item_ids: &[String],
) -> Result<CleanupPreview, CleanupError> {
    let tx = conn.unchecked_transaction()?;
    let preview = preview_in_snapshot(&tx, item_ids, Unknown::Refuse)?;
    tx.commit()?;
    Ok(preview)
}

/// Reserve the SQLite writer before validation and keep that reservation through
/// the full backup and deletion. The backup's second read-only connection can
/// read both rollback-journal and WAL databases: no DELETE has run yet, so this
/// transaction holds only a RESERVED lock, never the EXCLUSIVE commit lock.
/// If the deletion or the commit fails after the backup, the transaction rolls
/// back and that backup is removed again: it would describe a removal that did
/// not happen.
pub fn apply_catalog_cleanup(
    conn: &mut Connection,
    item_ids: &[String],
    token: &str,
    confirmed: bool,
) -> Result<AppliedCleanup, CleanupError> {
    if !confirmed {
        return Err(CleanupError::ConfirmationRequired);
    }
    let path = database_path(conn)?;
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let preview = preview_in_snapshot(&tx, item_ids, Unknown::Stale)?;
    if preview.token != token {
        return Err(CleanupError::StaleToken);
    }
    let backup = backup_path(&path, token).map_err(CleanupError::Backup)?;
    if let Err(error) = write_backup(&path, &backup) {
        // A partial file of a failed copy is no backup either.
        let _ = std::fs::remove_file(&backup);
        return Err(CleanupError::Backup(error));
    }
    let removed_item_ids: Vec<String> = preview.items.into_iter().map(|item| item.id).collect();
    let removed = (|| -> Result<(), rusqlite::Error> {
        for id in &removed_item_ids {
            tx.execute("DELETE FROM catalog_item WHERE id = ?1", [id])?;
        }
        tx.commit()
    })();
    if let Err(error) = removed {
        let _ = std::fs::remove_file(&backup);
        return Err(error.into());
    }
    Ok(AppliedCleanup {
        removed_item_ids,
        backup,
    })
}
