//! Imports a `.knxproj` file and persists its opaque entries into an
//! already-migrated store.
//!
//! `knx-app` is the only crate that sees both `knx-etsproj` and
//! `knx-store` — `check-layering` does not enforce this directly (it only
//! forbids `knx-core` from reaching storage/format crates), but keeping
//! the `OpaqueEntry` → `StoredOpaqueEntry` conversion here, and nowhere
//! else, is what keeps `knx-etsproj` itself free of a storage dependency:
//! nothing in `knx-etsproj` needs to know `knx-store`'s row shape exists.
//!
//! Manufacturer data (Task 12) routes through the shared product database
//! when the caller supplies one (`ImportOptions::product_db`); otherwise it
//! falls back to the project's own opaque store, exactly as Session 3
//! wrote it. Either way the manifest (`knx-store` schema v3) is written,
//! so the project can always name what it was imported with.

use std::path::Path;

use knx_etsproj::opaque::{ManufacturerFile, OpaqueEntry};
use knx_etsproj::{ImportFailure, ImportReport};
use knx_store::{
    insert_manufacturer_refs, insert_opaque, Connection, ManufacturerRef, SqlError,
    StoredOpaqueEntry,
};

use crate::progress::{LoadObserver, LoadStage};

pub struct ImportedProject {
    pub project: knx_core::Project,
    pub report: ImportReport,
    pub opaque_entries: usize,
    pub manufacturer_ingested: usize,
    pub manufacturer_skipped: usize,
    pub enrichment: Option<knx_productdb::EnrichmentReport>,
}

#[derive(Debug)]
pub enum AppError {
    Import(ImportFailure),
    Store(knx_store::MigrationError),
    Sql(SqlError),
    ProductDb(knx_productdb::ProductDbError),
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AppError::Import(e) => write!(f, "{e}"),
            AppError::Store(e) => write!(f, "{e}"),
            AppError::Sql(e) => write!(f, "{e}"),
            AppError::ProductDb(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for AppError {}

impl From<ImportFailure> for AppError {
    fn from(e: ImportFailure) -> Self {
        AppError::Import(e)
    }
}

impl From<knx_store::MigrationError> for AppError {
    fn from(e: knx_store::MigrationError) -> Self {
        AppError::Store(e)
    }
}

impl From<SqlError> for AppError {
    fn from(e: SqlError) -> Self {
        AppError::Sql(e)
    }
}

impl From<knx_productdb::ProductDbError> for AppError {
    fn from(e: knx_productdb::ProductDbError) -> Self {
        AppError::ProductDb(e)
    }
}

/// What a caller wants done with manufacturer data. `None` runs exactly
/// the Session 3 path: the files go into the project's opaque store and
/// nothing is enriched. That path stays supported, and stays tested,
/// because a user who does not want a shared database must still get a
/// complete project.
#[derive(Default)]
pub struct ImportOptions<'a> {
    pub product_db: Option<&'a knx_productdb::Connection>,
}

/// Imports `path` and persists every opaque entry into `conn`, which the
/// caller has already opened and migrated — the CLI decides whether that
/// connection is backed by a file or lives only for this run. The whole
/// insert runs inside one transaction (`knx_store::insert_opaque`, Task
/// 13), so a failure during import itself, which happens entirely before
/// the first row is inserted, leaves the store untouched; a failure
/// partway through the insert leaves it exactly as it was before this
/// call, not half-written.
pub fn import_ets_project(path: &Path, conn: &Connection) -> Result<ImportedProject, AppError> {
    import_ets_project_with(path, conn, ImportOptions::default())
}

/// The general form: `options.product_db` decides where manufacturer data
/// goes and whether the project's communication objects get enriched from
/// it.
pub fn import_ets_project_with(
    path: &Path,
    conn: &Connection,
    options: ImportOptions<'_>,
) -> Result<ImportedProject, AppError> {
    import_ets_project_observed(path, conn, options, &())
}

/// The general form with somebody watching: `observer` is told which stage
/// of the load is running, parser stages included (ADR-0023). Every other
/// entry point above is this one with `&()`, so the observed and the
/// unobserved import are the same code.
///
/// Where the `stage(..)` calls sit is the whole contract: each is announced
/// before the work it names, and [`LoadObserver::items`] is called only in
/// the manufacturer loop, the one place here whose total is known before it
/// starts.
pub fn import_ets_project_observed(
    path: &Path,
    conn: &Connection,
    options: ImportOptions<'_>,
    observer: &dyn LoadObserver,
) -> Result<ImportedProject, AppError> {
    import_ets_project_with_password(path, conn, options, None, observer)
}

/// [`import_ets_project_observed`] for a project that may be
/// password-protected (KNOWN_LIMITATIONS §13). `password` reaches only the
/// container decryptor; it is not stored, reported or logged. A missing or
/// wrong password fails before the first row is written, like every other
/// import failure, so the store is left untouched.
pub fn import_ets_project_with_password(
    path: &Path,
    conn: &Connection,
    options: ImportOptions<'_>,
    password: Option<&knx_etsproj::ProjectPassword>,
    observer: &dyn LoadObserver,
) -> Result<ImportedProject, AppError> {
    let outcome =
        knx_etsproj::import_knxproj_with(path, password, &crate::progress::ParseStages(observer))?;

    persist_outcome(outcome, conn, options, observer)
}

/// Imports already bounded/captured source bytes without reopening a private input.
/// This reuses the exact path-based pipeline and persistence, not a second parser.
pub fn import_ets_project_bytes(
    bytes: Vec<u8>,
    file_name: &str,
    conn: &Connection,
    options: ImportOptions<'_>,
) -> Result<ImportedProject, AppError> {
    let outcome = knx_etsproj::import_knxproj_bytes_with(bytes, file_name, None, &())?;
    persist_outcome(outcome, conn, options, &())
}

fn persist_outcome(
    mut outcome: knx_etsproj::ImportOutcome,
    conn: &Connection,
    options: ImportOptions<'_>,
    observer: &dyn LoadObserver,
) -> Result<ImportedProject, AppError> {
    // Classify preserved payloads using the existing product adapter. A picture
    // or document is not plugin code; this does not decode or execute anything.
    for file in &outcome.manufacturer {
        if file.kind == knx_etsproj::opaque::OpaqueKind::Baggage {
            let class = knx_productdb::sniff_media(&file.bytes).as_str();
            let description = format!(
                "manufacturer payload ({class}); retained byte-exact, not rendered or executed"
            );
            for feature in &mut outcome.report.unsupported {
                if feature.what == file.source_path {
                    feature.consequence = description.clone();
                }
            }
            for summary in &mut outcome.report.opaque {
                if summary.source_path == file.source_path && summary.kind == "Baggage" {
                    summary.reason = description.clone();
                }
            }
        }
    }
    // The manifest is written whichever way the manufacturer files are
    // stored: it describes what the project was imported with, not where
    // the bytes ended up.
    let manifest: Vec<ManufacturerRef> = outcome
        .manufacturer
        .iter()
        .map(|m| ManufacturerRef {
            source_path: m.source_path.clone(),
            sha256: m.sha256.clone(),
            len: m.bytes.len() as i64,
            kind: format!("{:?}", m.kind),
        })
        .collect();

    let mut stored: Vec<StoredOpaqueEntry> = outcome.opaque.iter().map(to_stored).collect();
    let mut ingested = 0usize;
    let mut skipped = 0usize;
    let mut enrichment = None;

    match options.product_db {
        Some(products) => {
            // Validate cached evidence for the complete source set before a new
            // file is installed. A late corrupted cache row must not leave an
            // earlier manufacturer's newly committed data behind.
            for file in &outcome.manufacturer {
                knx_productdb::source_diagnostics(products, &file.sha256)?;
            }
            observer.stage(LoadStage::IngestManufacturerData);
            let total = outcome.manufacturer.len() as u64;
            for (index, file) in outcome.manufacturer.iter().enumerate() {
                match knx_productdb::ingest_file(products, &file.source_path, &file.bytes)? {
                    knx_productdb::IngestOutcome::Ingested { .. } => ingested += 1,
                    knx_productdb::IngestOutcome::Skipped { .. } => skipped += 1,
                }
                match knx_productdb::source_diagnostics(products, &file.sha256)? {
                    Some(findings) => {
                        for finding in findings {
                            match finding.kind.as_str() {
                                "Element" | "Attribute" => {
                                    outcome.report.unknown.push(knx_etsproj::UnknownConstruct {
                                        source_path: file.source_path.clone(),
                                        xpath: finding.xpath,
                                        kind: if finding.kind == "Element" {
                                            knx_etsproj::UnknownKind::Element
                                        } else {
                                            knx_etsproj::UnknownKind::Attribute
                                        },
                                        name: finding.name,
                                        occurrences: finding.occurrences,
                                        sample: finding.sample,
                                    })
                                }
                                _ => outcome
                                    .report
                                    .errors
                                    .push(knx_etsproj::report::ImportError {
                                        source_path: Some(file.source_path.clone()),
                                        stage: "manufacturer",
                                        severity: knx_etsproj::report::Severity::Error,
                                        xpath: String::new(),
                                        detail: format!(
                                            "{}: {}/{} ({} occurrences); original source retained",
                                            finding.kind,
                                            finding.xpath,
                                            finding.name,
                                            finding.occurrences
                                        ),
                                    }),
                            }
                        }
                    }
                    None => {
                        outcome
                            .report
                            .unsupported
                            .push(knx_etsproj::report::UnsupportedFeature {
                                what: format!("manufacturer diagnostics: {}", file.source_path),
                                consequence:
                                    "unavailable: retained source has no measured parser evidence"
                                        .into(),
                            })
                    }
                }
                observer.items(index as u64 + 1, total);
            }
            if let Some(master) = outcome.opaque.iter().find(|e| {
                // Do not give a foreign/unreadable root typed KNX master
                // semantics. Its bytes stay in `stored`; its boundary was
                // already reported by the authoritative detection pass.
                e.kind == knx_etsproj::opaque::OpaqueKind::MasterData
                    && outcome.master_metadata_error.is_none()
            }) {
                observer.stage(LoadStage::IngestMasterData);
                let findings = knx_productdb::ingest_master_data(products, &master.bytes)?;
                outcome
                    .report
                    .unknown
                    .extend(findings.unknown.into_iter().map(|finding| {
                        knx_etsproj::UnknownConstruct {
                            source_path: master.source_path.clone(),
                            xpath: finding.xpath,
                            kind: match finding.kind {
                                knx_productdb::report::UnknownKind::Element => {
                                    knx_etsproj::UnknownKind::Element
                                }
                                knx_productdb::report::UnknownKind::Attribute => {
                                    knx_etsproj::UnknownKind::Attribute
                                }
                            },
                            name: finding.name,
                            occurrences: finding.occurrences,
                            sample: finding.sample,
                        }
                    }));
            }
            observer.stage(LoadStage::EnrichFromProductDatabase);
            enrichment = Some(knx_productdb::enrich(&mut outcome.project, products)?);
        }
        None => {
            stored.extend(outcome.manufacturer.iter().map(manufacturer_to_stored));
            if !outcome.manufacturer.is_empty() {
                outcome.report.unsupported.push(knx_etsproj::report::UnsupportedFeature {
                    what: "manufacturer semantic diagnostics".into(),
                    consequence: "unavailable without a product database; source files are retained, not semantically installed".into(),
                });
            }
        }
    }

    let opaque_entries = stored.len();
    observer.stage(LoadStage::PersistOpaque);
    insert_opaque(conn, &stored)?;
    insert_manufacturer_refs(conn, &manifest)?;

    Ok(ImportedProject {
        project: outcome.project,
        report: outcome.report,
        opaque_entries,
        manufacturer_ingested: ingested,
        manufacturer_skipped: skipped,
        enrichment,
    })
}

/// `kind` renders as the `OpaqueKind` variant name (`Debug`-formatted, the
/// same convention `report::opaque_summary` already uses for the same
/// enum) — never interpreted, never executed, just carried through as a
/// label alongside the bytes.
fn to_stored(e: &OpaqueEntry) -> StoredOpaqueEntry {
    StoredOpaqueEntry {
        source_path: e.source_path.clone(),
        xpath: e.xpath.clone(),
        kind: format!("{:?}", e.kind),
        name: e.name.clone(),
        bytes: e.bytes.clone(),
        sha256: e.sha256.clone(),
    }
}

/// The `--no-product-db` fallback: a manufacturer file becomes a whole-file
/// opaque entry, same shape `to_stored` would have produced for it before
/// Task 12 split the two apart.
fn manufacturer_to_stored(m: &ManufacturerFile) -> StoredOpaqueEntry {
    StoredOpaqueEntry {
        source_path: m.source_path.clone(),
        xpath: String::new(),
        kind: format!("{:?}", m.kind),
        name: String::new(),
        bytes: m.bytes.clone(),
        sha256: m.sha256.clone(),
    }
}
