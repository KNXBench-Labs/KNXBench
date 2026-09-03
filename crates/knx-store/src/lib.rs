//! SQLite project storage, schema migrations, and the opaque passthrough store.

use std::fmt;

pub mod devices;
pub mod manifest;
pub mod migration;
pub mod opaque;
pub mod strings;
pub mod topology;

pub use manifest::{insert_manufacturer_refs, load_manufacturer_refs, ManufacturerRef};
pub use migration::{
    open_and_migrate, open_and_migrate_in_memory, MigrationError, CURRENT_SCHEMA_VERSION,
};
pub use opaque::{insert_opaque, load_opaque, StoredOpaqueEntry};
/// Re-exported so `knx-app` names the connection type through the storage
/// crate rather than depending on `rusqlite` directly.
pub use rusqlite::{Connection, Error as SqlError};

/// Errors from the entity-persistence layer (`project`, `strings`,
/// `topology`, `building`, `devices`, `group`, `parameter`,
/// `command_sync`) — distinct from `MigrationError`, which is only about
/// getting the schema to `CURRENT_SCHEMA_VERSION`.
#[derive(Debug)]
pub enum StoreError {
    Sqlite(rusqlite::Error),
    /// `load_project` was called against a database with no `project_info`
    /// row — it was migrated but never saved. Distinct from an empty
    /// project (`Project::new`), which is a valid in-memory value that has
    /// simply not been persisted yet.
    NotSaved,
}

impl std::error::Error for StoreError {}

impl fmt::Display for StoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StoreError::Sqlite(e) => write!(f, "{e}"),
            StoreError::NotSaved => write!(f, "no project has been saved to this database yet"),
        }
    }
}

impl From<rusqlite::Error> for StoreError {
    fn from(e: rusqlite::Error) -> Self {
        StoreError::Sqlite(e)
    }
}
