//! Product database: manufacturer, hardware, application program and version
//! data in a separate SQLite file shared across projects (ADR-0005).
//!
//! Every ingested file is kept twice: verbatim as a blob keyed by its
//! SHA-256, and as parsed rows. The blob is the integrity guarantee — the
//! parser may not understand a construct, but nothing is ever lost.

pub mod blob;
pub mod enrich;
pub mod ingest;
pub mod migration;
pub mod package;
pub mod parse;
pub mod query;
pub mod report;
pub mod xml;

pub use blob::{
    has_source_file, load_source_file, sha256_hex, store_source_file, verify, BlobMismatch,
    SourceFile,
};
pub use enrich::{enrich, EnrichmentIssue, EnrichmentReport};
pub use ingest::{ingest_file, FileKind, IngestOutcome};
pub use migration::{default_path, open_and_migrate, ProductDbError, CURRENT_PRODUCTDB_VERSION};
pub use package::{install_package, InstallReport, PackageError, PackageMember};
pub use parse::master::ingest_master_data;
/// Re-exported so callers name the connection type through this crate
/// rather than depending on `rusqlite` directly.
pub use rusqlite::Connection;
