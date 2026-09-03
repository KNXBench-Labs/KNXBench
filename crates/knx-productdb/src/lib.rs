//! Product database: manufacturer, hardware, application program and version
//! data in a separate SQLite file shared across projects (ADR-0005).
//!
//! Every ingested file is kept twice: verbatim as a blob keyed by its
//! SHA-256, and as parsed rows. The blob is the integrity guarantee — the
//! parser may not understand a construct, but nothing is ever lost.

pub mod migration;

pub use migration::{default_path, open_and_migrate, ProductDbError, CURRENT_PRODUCTDB_VERSION};
/// Re-exported so callers name the connection type through this crate
/// rather than depending on `rusqlite` directly.
pub use rusqlite::Connection;
