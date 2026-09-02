//! SQLite project storage, schema migrations, and the opaque passthrough store.

pub mod migration;

pub use migration::{open_and_migrate, MigrationError, CURRENT_SCHEMA_VERSION};
