//! Assembles the full opaque-entry list `knx_etsproj::export::export_knxproj`
//! needs: every stored opaque entry plus every manufacturer file the
//! project's manifest names, fetched back out of wherever it lives —
//! `knx-store`'s own opaque table (the `--no-product-db` fallback) or the
//! shared product database keyed by content hash.
//!
//! `export_knxproj`'s signature does not change (spec §3): this module
//! assembles the entry list before calling it, exactly as `import.rs`
//! already assembles stored opaque entries on the way in.

use knx_etsproj::export::{ExportOutcome, ExportWarning};
use knx_etsproj::opaque::{OpaqueEntry, OpaqueKind};
use knx_store::{load_manufacturer_refs, load_opaque, Connection, StoredOpaqueEntry};

use crate::AppError;

/// Rebuilds the full opaque-entry list — stored entries plus every
/// manufacturer file fetched back out of the product database — and hands
/// it to `knx-etsproj`'s writer, whose signature does not change.
pub fn export_ets_project(
    project: &knx_core::Project,
    conn: &Connection,
    products: Option<&knx_productdb::Connection>,
) -> Result<ExportOutcome, AppError> {
    let mut entries: Vec<OpaqueEntry> = load_opaque(conn)?.iter().map(from_stored).collect();
    let mut missing = Vec::new();

    for reference in load_manufacturer_refs(conn)? {
        let bytes = match products {
            Some(products) => knx_productdb::load_source_file(products, &reference.sha256)?,
            None => None,
        };
        match bytes {
            Some(bytes) => entries.push(OpaqueEntry {
                source_path: reference.source_path,
                xpath: String::new(),
                kind: kind_from_str(&reference.kind),
                name: String::new(),
                bytes,
                sha256: reference.sha256,
            }),
            None => missing.push(ExportWarning::MissingManufacturerData {
                source_path: reference.source_path,
                sha256: reference.sha256,
            }),
        }
    }

    let mut outcome = knx_etsproj::export::export_knxproj(project, &entries)?;
    outcome.warnings.extend(missing);
    Ok(outcome)
}

fn from_stored(e: &StoredOpaqueEntry) -> OpaqueEntry {
    OpaqueEntry {
        source_path: e.source_path.clone(),
        xpath: e.xpath.clone(),
        kind: kind_from_str(&e.kind),
        name: e.name.clone(),
        bytes: e.bytes.clone(),
        sha256: e.sha256.clone(),
    }
}

/// The inverse of `format!("{:?}", kind)`. A label this build does not
/// know falls back to `ContainerEntry` — a forward-compatibility rule, not
/// a silent drop, since the bytes are written either way regardless of
/// which kind they are tagged with.
fn kind_from_str(s: &str) -> OpaqueKind {
    match s {
        "ManufacturerData" => OpaqueKind::ManufacturerData,
        "Baggage" => OpaqueKind::Baggage,
        "BinaryData" => OpaqueKind::BinaryData,
        "ExtraData" => OpaqueKind::ExtraData,
        "Signature" => OpaqueKind::Signature,
        "MasterData" => OpaqueKind::MasterData,
        "RetainedAttribute" => OpaqueKind::RetainedAttribute,
        "RetainedElement" => OpaqueKind::RetainedElement,
        _ => OpaqueKind::ContainerEntry,
    }
}
