//! Product database: manufacturer, hardware, application program and version
//! data in a separate SQLite file shared across projects (ADR-0005).
//!
//! Every ingested file is kept twice: verbatim as a blob keyed by its
//! SHA-256, and as parsed rows. The blob is the integrity guarantee — the
//! parser may not understand a construct, but nothing is ever lost.

pub mod baggage;
pub mod blob;
pub mod dynamic;
pub mod enrich;
pub mod identity;
pub mod ingest;
pub mod migration;
pub mod package;
pub mod parse;
pub mod query;
pub mod report;
pub mod xml;

pub use baggage::{
    load_baggage_inventory, sniff_media, BaggageInventory, BaggagePayload, MediaClass,
    NestedArchive, Resolution, ResolvedDeclaration,
};
pub use blob::{
    has_source_file, load_source_file, sha256_hex, store_source_file, verify, BlobMismatch,
    SourceFile,
};
pub use enrich::{com_object_lookup_id, enrich, EnrichmentIssue, EnrichmentReport};
pub use identity::{
    identity_candidates, identity_divergences, package_source_names, products_by_order_number,
    program_family, Divergence, FamilyKey, FamilyMember, IdentityCandidate, IdentityKind,
    IdentityReport, OrderNumberProduct, ProgramFamily, ReplacesVersions, UnmeasuredSource,
};
pub use ingest::{ingest_file, FileKind, IngestOutcome};
pub use migration::{default_path, open_and_migrate, ProductDbError, CURRENT_PRODUCTDB_VERSION};
pub use package::{
    install_package, InstallCategory, InstallCount, InstallDiagnostic, InstallDiagnosticKind,
    InstallDisposition, InstallFacts, InstallReport, PackageError, PackageMember,
};
pub use parse::baggage::BaggageDeclaration;
pub use parse::master::{ingest_master_data, MasterIngest};
/// Re-exported so callers name the connection type through this crate
/// rather than depending on `rusqlite` directly.
pub use rusqlite::Connection;
