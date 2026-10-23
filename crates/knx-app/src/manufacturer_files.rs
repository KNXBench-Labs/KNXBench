//! Keeps a project file's manufacturer files and the product database in step (ADR-0128).
//!
//! An import installs every manufacturer file into the shared product
//! database and records it in the project's manifest. Three operations keep
//! a `.knxdb` usable when that database is empty or another one (GitHub
//! #21: a rebuilt container, another computer):
//!
//! - [`embed_into`] after a save: the file carries every manifest entry's
//!   bytes, taken from the product database or an earlier copy of the file.
//! - [`reinstall_from_file`] after an open: whatever the product database
//!   lacks is installed from the bytes the file carries.
//! - [`restore_from_knxproj`] for a file saved before carrying existed:
//!   the original `.knxproj`'s members are installed when, and only when,
//!   their SHA-256 matches the manifest.
//!
//! None of them changes the project itself.

use std::collections::BTreeSet;
use std::fmt;

use knx_productdb::ProductDbError;
use knx_store::{Connection, ManufacturerFileError, ManufacturerRef};

use crate::progress::{LoadObserver, LoadStage};

/// Why a product-data operation failed as a whole. Per-file problems are
/// reported in the operation's report instead, never as this error.
#[derive(Debug)]
pub enum ProductDataError {
    Sql(knx_store::SqlError),
    ProductDb(ProductDbError),
    Carried(ManufacturerFileError),
    Archive(knx_etsproj::ContainerError),
}

impl fmt::Display for ProductDataError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Sql(e) => write!(f, "{e}"),
            Self::ProductDb(e) => write!(f, "{e}"),
            Self::Carried(e) => write!(f, "{e}"),
            Self::Archive(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for ProductDataError {}

impl From<knx_store::SqlError> for ProductDataError {
    fn from(e: knx_store::SqlError) -> Self {
        Self::Sql(e)
    }
}

impl From<ProductDbError> for ProductDataError {
    fn from(e: ProductDbError) -> Self {
        Self::ProductDb(e)
    }
}

impl From<ManufacturerFileError> for ProductDataError {
    fn from(e: ManufacturerFileError) -> Self {
        Self::Carried(e)
    }
}

impl From<knx_etsproj::ContainerError> for ProductDataError {
    fn from(e: knx_etsproj::ContainerError) -> Self {
        Self::Archive(e)
    }
}

/// The manifest's entries, first occurrence of each hash, in manifest
/// order: the order the import installed them in.
fn distinct(refs: &[ManufacturerRef]) -> Vec<&ManufacturerRef> {
    let mut seen = BTreeSet::new();
    refs.iter()
        .filter(|r| seen.insert(r.sha256.as_str()))
        .collect()
}

/// The manifest entries `products` does not hold, in manifest order.
pub fn missing_from_products(
    products: &knx_productdb::Connection,
    refs: &[ManufacturerRef],
) -> Result<Vec<ManufacturerRef>, ProductDbError> {
    let mut missing = Vec::new();
    for r in distinct(refs) {
        let present = if r.kind == "CataloguePackage" {
            knx_productdb::catalogue_sources::has_catalogue_original(products, &r.sha256)?
        } else {
            knx_productdb::has_source_file(products, &r.sha256)?
        };
        if !present {
            missing.push(r.clone());
        }
    }
    Ok(missing)
}

/// What [`embed_into`] did.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct EmbedReport {
    /// Files written into the project file by this call.
    pub stored: usize,
    /// Files the project file already carried.
    pub carried: usize,
    /// Rows deleted because the saved manifest no longer names them.
    pub removed: usize,
    /// Manifest entries nobody could supply: the file stays without them.
    pub unavailable: Vec<ManufacturerRef>,
}

/// Makes `target`, a saved native project, carry every file its saved
/// manifest names. Bytes come from `products` first, then from each of
/// `fallbacks` (earlier copies of the project); every candidate is
/// hash-checked before it is stored. One transaction: a failure leaves the
/// file's carried rows as they were.
pub fn embed_into(
    target: &Connection,
    products: Option<&knx_productdb::Connection>,
    fallbacks: &[&Connection],
) -> Result<EmbedReport, ProductDataError> {
    let tx = target.unchecked_transaction()?;
    let refs = knx_store::load_manufacturer_refs(&tx)?;
    let carried = knx_store::carried_manufacturer_hashes(&tx)?;
    let mut report = EmbedReport::default();
    for r in distinct(&refs) {
        if carried.contains(&r.sha256) {
            report.carried += 1;
            continue;
        }
        match supply(r, products, fallbacks)? {
            Some(bytes) => {
                knx_store::store_manufacturer_file(&tx, &r.source_path, &bytes)?;
                report.stored += 1;
            }
            None => report.unavailable.push(r.clone()),
        }
    }
    let keep: BTreeSet<String> = refs.into_iter().map(|r| r.sha256).collect();
    report.removed = knx_store::retain_manufacturer_files(&tx, &keep)?;
    tx.commit()?;
    Ok(report)
}

/// The bytes for `sha256` from the first source that holds them intact.
/// A damaged fallback copy is skipped, not fatal: another source may hold
/// the file, and the report names it unavailable otherwise.
fn supply(
    source: &ManufacturerRef,
    products: Option<&knx_productdb::Connection>,
    fallbacks: &[&Connection],
) -> Result<Option<Vec<u8>>, ProductDataError> {
    let sha256 = source.sha256.as_str();
    if let Some(products) = products {
        let bytes = if source.kind == "CataloguePackage" {
            knx_productdb::catalogue_sources::load_catalogue_original(
                products,
                sha256,
                knx_store::manufacturer_files::MAX_MANUFACTURER_FILE_BYTES,
            )?
        } else {
            knx_productdb::load_source_file(products, sha256)?
        };
        if let Some(bytes) = bytes {
            if knx_store::manufacturer_file_hash(&bytes) == sha256 {
                return Ok(Some(bytes));
            }
        }
    }
    for fallback in fallbacks {
        if let Ok(Some(file)) = knx_store::load_manufacturer_file(fallback, sha256) {
            return Ok(Some(file.bytes));
        }
    }
    Ok(None)
}

/// What [`reinstall_from_file`] did.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ReinstallReport {
    /// Source paths installed into the product database.
    pub installed: Vec<String>,
    /// Entries the file does not carry: still missing afterwards.
    pub unavailable: Vec<ManufacturerRef>,
    /// Entries whose carried copy is damaged or failed to install.
    pub failed: Vec<(ManufacturerRef, String)>,
}

/// Installs each of `missing` into `products` from the bytes `carrier`
/// holds. One file's failure does not stop the others.
pub fn reinstall_from_file(
    products: &knx_productdb::Connection,
    carrier: &Connection,
    missing: &[ManufacturerRef],
    observer: &dyn LoadObserver,
) -> ReinstallReport {
    let mut report = ReinstallReport::default();
    if missing.is_empty() {
        return report;
    }
    observer.stage(LoadStage::IngestManufacturerData);
    let total = missing.len() as u64;
    let ordered: Vec<_> = missing
        .iter()
        .filter(|r| r.kind == "CataloguePackage")
        .chain(missing.iter().filter(|r| r.kind != "CataloguePackage"))
        .collect();
    for (index, r) in ordered.into_iter().enumerate() {
        match knx_store::load_manufacturer_file(carrier, &r.sha256) {
            Ok(Some(file)) => {
                let installed = if r.kind == "CataloguePackage" {
                    knx_productdb::install_package(products, &r.source_path, &file.bytes)
                        .map(|_| ())
                        .map_err(|e| e.to_string())
                } else {
                    knx_productdb::ingest_file(products, &r.source_path, &file.bytes)
                        .map(|_| ())
                        .map_err(|e| e.to_string())
                };
                match installed {
                    Ok(()) => report.installed.push(r.source_path.clone()),
                    Err(e) => report.failed.push((r.clone(), e)),
                }
            }
            Ok(None) => report.unavailable.push(r.clone()),
            Err(e) => report.failed.push((r.clone(), e.to_string())),
        }
        observer.items(index as u64 + 1, total);
    }
    report
}

/// What [`restore_from_knxproj`] did.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct RestoreReport {
    /// Source paths installed from the archive.
    pub installed: Vec<String>,
    /// Manifest entries the product database already held.
    pub already_installed: usize,
    /// Archive members with the manifest's path but other bytes: not
    /// installed, since they are not what the project was made with.
    pub mismatched: Vec<String>,
    /// Missing manifest entries the archive does not contain at all.
    pub not_in_archive: Vec<String>,
    /// Matching members the product database refused, with the reason.
    pub failed: Vec<(String, String)>,
}

/// Installs the missing manifest entries from an original `.knxproj`.
/// Only outer archive members are read, so a password-protected project
/// needs no password: its manufacturer files are not encrypted.
pub fn restore_from_knxproj(
    products: &knx_productdb::Connection,
    refs: &[ManufacturerRef],
    archive: Vec<u8>,
) -> Result<RestoreReport, ProductDataError> {
    let mut container = knx_etsproj::Container::open_outer(archive)?;
    let missing = missing_from_products(products, refs)?;
    let mut report = RestoreReport {
        already_installed: distinct(refs).len() - missing.len(),
        ..RestoreReport::default()
    };
    for r in missing {
        if container.find(&r.source_path).is_none() {
            report.not_in_archive.push(r.source_path);
            continue;
        }
        let bytes = match container.read(&r.source_path) {
            Ok(bytes) => bytes,
            Err(e) => {
                report.failed.push((r.source_path, e.to_string()));
                continue;
            }
        };
        if knx_store::manufacturer_file_hash(&bytes) != r.sha256 {
            report.mismatched.push(r.source_path);
            continue;
        }
        match knx_productdb::ingest_file(products, &r.source_path, &bytes) {
            Ok(_) => report.installed.push(r.source_path),
            Err(e) => report.failed.push((r.source_path, e.to_string())),
        }
    }
    Ok(report)
}
