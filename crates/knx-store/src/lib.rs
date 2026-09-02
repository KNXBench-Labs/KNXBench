//! SQLite project storage, schema migrations, and the opaque passthrough store.

pub mod migration;
pub mod opaque;

pub use migration::{open_and_migrate, MigrationError, CURRENT_SCHEMA_VERSION};
pub use opaque::{insert_opaque, load_opaque, StoredOpaqueEntry};
