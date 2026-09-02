//! SQLite project storage, schema migrations, and the opaque passthrough store.

pub mod migration;
pub mod opaque;

pub use migration::{open_and_migrate, MigrationError, CURRENT_SCHEMA_VERSION};
pub use opaque::{insert_opaque, load_opaque, StoredOpaqueEntry};
/// Re-exported so `knx-app` names the connection type through the storage
/// crate rather than depending on `rusqlite` directly.
pub use rusqlite::{Connection, Error as SqlError};
