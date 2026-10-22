//! Product database: manufacturer, hardware, application program and version
//! data in a separate SQLite file shared across projects (ADR-0005).
//!
//! Every ingested file is kept twice: verbatim as a blob keyed by its
//! SHA-256, and as parsed rows. The blob is the integrity guarantee — the
//! parser may not understand a construct, but nothing is ever lost.

pub mod baggage;
pub mod blob;
pub mod byte_order_probe;
mod catalog_details;
// Private schema admission remains unwired until the managed witness exists.
#[allow(dead_code)]
mod catalogue_schema;
pub mod cleanup;
pub mod code;
pub mod declared_segments;
pub mod device_evaluation;
pub mod download_plan;
pub mod dynamic;
pub mod enrich;
pub mod identity;
pub mod image;
pub mod image_comparison;
pub mod image_request;
pub mod inference;
pub mod ingest;
pub mod legacy;
// Managed owner primitive: wired into the server by the owner-wrapper
// increment (WP-04.3 stage B, ADR-0150).
pub mod managed_catalogue;
// Private bounded copy primitive remains unwired until the W/S/D orchestration
// exists (WP-04.3 stage B, ADR-0150).
#[allow(dead_code)]
mod catalogue_copy;
// Private stamp/binding primitive remains unwired until the preview/apply
// orchestration exists (WP-04.3 stage D, ADR-0150).
#[allow(dead_code)]
mod catalogue_stamp;
// Private controlled source/policy identity remains unwired until the
// preview/apply orchestration exists (WP-04.3 stage D, ADR-0150).
#[allow(dead_code)]
mod controlled;
pub mod master_evidence;
pub mod master_provenance;
pub mod migration;
pub mod operation;
pub mod package;
pub mod parameter_editor;
pub mod parameter_layout;
pub mod parse;
pub mod procedure_resolution;
mod promotion_repair;
pub mod query;
pub use catalog_details::CatalogItemDetails;
pub use cleanup::{
    apply_catalog_cleanup, preview_catalog_cleanup, AppliedCleanup, CleanupError, CleanupPreview,
};
pub mod report;
pub mod source_diagnostics;
pub mod system2;
pub mod system2_procedure;
pub mod text_parameters;
pub mod update;
pub mod wrapper;
pub use source_diagnostics::{source_diagnostics, SourceDiagnostic};
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
pub use master_evidence::{rederive_master_language_evidence, MasterLanguageEvidenceReport};
pub use migration::{
    default_path, open_and_migrate, open_read_only, ProductDbError, ReadOnlyProductDb,
    CURRENT_PRODUCTDB_VERSION,
};
pub use package::{
    install_package, install_package_with_limits, InstallCategory, InstallCount, InstallDiagnostic,
    InstallDiagnosticKind, InstallDisposition, InstallFacts, InstallReport, PackageError,
    PackageLimits, PackageMember, MAX_PACKAGE_INPUT_BYTES,
};
pub use parse::baggage::BaggageDeclaration;
pub use parse::master::{ingest_master_data, MasterIngest};
/// Re-exported so callers name the connection type through this crate
/// rather than depending on `rusqlite` directly.
pub use rusqlite::Connection;
pub use update::{
    apply_package_update, preview_package_update, restore_product_db_backup, AppliedUpdate,
    InstalledVersion, ProgramUpdate, ProgramUpdateStatus, UpdateError, UpdatePreview,
    WrapperPackagePreview, WrapperPreview,
};
pub use wrapper::{
    install_product_file, InstalledPackage, ProductFileInstall, WrapperContents, WrapperMember,
    WrapperMemberKind, WRAPPER_SOURCE_SEPARATOR,
};
