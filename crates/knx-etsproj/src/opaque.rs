//! The opaque passthrough store's values: anything the domain model does
//! not carry, kept as bytes with a SHA-256 rather than silently dropped
//! (CLAUDE.md's data-integrity rule).
//!
//! Three sources feed it: whole container entries this session's model
//! cannot regenerate (`collect_container_entries`), and the two flavors of
//! Stage-3 leftovers export needs to write back — a known-but-unmodelled
//! attribute (`from_retained_attribute`) and an unrecognized element's raw
//! bytes (`from_retained_element`). All three produce the same
//! [`OpaqueEntry`] shape, so Task 13's storage layer and Task 19's export
//! writer handle them uniformly regardless of where they came from.
//!
//! `ManufacturerData` and `Baggage` entries are collected here too, but
//! [`collect_container_entries`] hands them out separately as
//! [`ManufacturerFile`] rather than folding them into the `OpaqueEntry`
//! list: ADR-0005 and IMPORT_EXPORT §10 put manufacturer data in the
//! shared product database (Session 4), stored once rather than once per
//! project. `M-xxxx.signature` entries are the one exception and stay in
//! the project's own opaque store (spec §3) — they sign a container state,
//! not a product.

use sha2::{Digest, Sha256};

use crate::container::{Container, ContainerError};
use crate::source::{RetainedAttribute, RetainedElement};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpaqueEntry {
    pub source_path: String,
    /// Empty for a whole container entry; an XPath for a retained fragment.
    pub xpath: String,
    pub kind: OpaqueKind,
    /// Attribute name for `RetainedAttribute`, element name for
    /// `RetainedElement`, empty for a container entry.
    pub name: String,
    pub bytes: Vec<u8>,
    pub sha256: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpaqueKind {
    /// Any container entry we do not regenerate.
    ContainerEntry,
    /// `<M-xxxx>/*`; moves to the product database in Session 4.
    ManufacturerData,
    /// `<M-xxxx>/Baggages/*` — never executed.
    Baggage,
    /// `<P-xxxx>/BinaryData/*.dat`.
    BinaryData,
    /// `<P-xxxx>/ExtraData/*.azp`, `*.rbg`.
    ExtraData,
    /// `*.signature` — cannot be regenerated.
    Signature,
    /// `knx_master.xml`.
    MasterData,
    /// A known or unknown attribute the model does not carry.
    RetainedAttribute,
    /// An element the model does not carry, raw bytes.
    RetainedElement,
}

/// A manufacturer file on its way to the product database (ADR-0005).
/// Same bytes and same hash as an `OpaqueEntry` would have carried — this
/// type exists so the destination is visible in the type system rather
/// than decided by a `match` in the caller.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManufacturerFile {
    pub source_path: String,
    pub bytes: Vec<u8>,
    pub sha256: String,
    pub kind: OpaqueKind,
}

/// The two destinations a container entry can be sorted into:
/// `opaque` for the project's own passthrough store, `manufacturer` for
/// the shared product database.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CollectedEntries {
    pub opaque: Vec<OpaqueEntry>,
    pub manufacturer: Vec<ManufacturerFile>,
}

/// Reads every container entry except those named in `regenerated` (paths
/// this session's own exporter writes fresh, so keeping the original bytes
/// too would just mean export has to choose between two copies), and sorts
/// each into the opaque store or the manufacturer-data list by its
/// [`OpaqueKind`]. Matched case-insensitively, the same convention
/// [`Container::find`] uses.
pub fn collect_container_entries(
    container: &mut Container,
    regenerated: &[&str],
) -> Result<CollectedEntries, ContainerError> {
    collect_container_entries_observed(container, regenerated, &())
}

/// [`collect_container_entries`] reporting its progress as it goes. This is
/// the one place in the import with a real total to report (ADR-0023): the
/// archive's entry count is known before the loop starts, so `completed`
/// of `total` here is a measurement rather than an estimate. `completed`
/// counts entries *walked*, including the regenerated ones skipped below —
/// those are the entries the loop is done with, and a counter that skipped
/// them would stall short of its own total.
pub fn collect_container_entries_observed(
    container: &mut Container,
    regenerated: &[&str],
    observer: &dyn crate::ImportObserver,
) -> Result<CollectedEntries, ContainerError> {
    let paths: Vec<String> = container.entries().iter().map(|e| e.path.clone()).collect();
    let total = paths.len() as u64;
    let mut walked = 0u64;
    let mut out = CollectedEntries {
        opaque: Vec::with_capacity(paths.len()),
        manufacturer: Vec::new(),
    };
    for path in paths {
        walked += 1;
        observer.items(walked, total);
        if regenerated.iter().any(|r| r.eq_ignore_ascii_case(&path)) {
            continue;
        }
        let bytes = container.read(&path)?;
        let sha256 = sha256_hex(&bytes);
        let kind = classify(&path);
        match kind {
            OpaqueKind::ManufacturerData | OpaqueKind::Baggage => {
                out.manufacturer.push(ManufacturerFile {
                    source_path: path,
                    bytes,
                    sha256,
                    kind,
                });
            }
            _ => {
                out.opaque.push(OpaqueEntry {
                    source_path: path,
                    xpath: String::new(),
                    kind,
                    name: String::new(),
                    bytes,
                    sha256,
                });
            }
        }
    }
    Ok(out)
}

/// Classifies a container entry by where it sits in the archive, not by
/// its content — cheap, and the archive layout is exactly what
/// IMPORT_EXPORT §2/§5 documents as stable across schema versions.
fn classify(path: &str) -> OpaqueKind {
    if path.eq_ignore_ascii_case("knx_master.xml") {
        return OpaqueKind::MasterData;
    }
    if path.ends_with(".signature") {
        return OpaqueKind::Signature;
    }
    let top = path.split('/').next().unwrap_or(path);
    if top.starts_with("M-") {
        return if path.contains("/Baggages/") {
            OpaqueKind::Baggage
        } else {
            OpaqueKind::ManufacturerData
        };
    }
    if top.starts_with("P-") {
        if path.contains("/BinaryData/") {
            return OpaqueKind::BinaryData;
        }
        if path.contains("/ExtraData/") {
            return OpaqueKind::ExtraData;
        }
    }
    OpaqueKind::ContainerEntry
}

pub fn from_retained_attribute(source_path: &str, a: &RetainedAttribute) -> OpaqueEntry {
    let bytes = a.value.clone().into_bytes();
    let sha256 = sha256_hex(&bytes);
    OpaqueEntry {
        source_path: source_path.to_string(),
        xpath: a.xpath.clone(),
        kind: OpaqueKind::RetainedAttribute,
        name: a.name.clone(),
        bytes,
        sha256,
    }
}

pub fn from_retained_element(source_path: &str, e: &RetainedElement) -> OpaqueEntry {
    let sha256 = sha256_hex(&e.raw);
    OpaqueEntry {
        source_path: source_path.to_string(),
        xpath: e.xpath.clone(),
        kind: OpaqueKind::RetainedElement,
        name: e.name.clone(),
        bytes: e.raw.clone(),
        sha256,
    }
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hasher
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::reference_ets4_bytes;

    #[test]
    fn manufacturer_files_are_handed_out_separately_from_opaque_entries() {
        if !crate::testutil::corpus_available() {
            eprintln!("skip: OriginalData/ corpus not present (gitignored, local-only)");
            return;
        }
        let mut c = Container::open(reference_ets4_bytes()).unwrap();
        let collected =
            collect_container_entries(&mut c, &["P-0512/0.xml", "P-0512/Project.xml"]).unwrap();
        // 36 entries less the two regenerated ones, split into the
        // manufacturer files (which now go to the product database) and
        // everything else (which stays in the project's opaque store).
        assert_eq!(collected.opaque.len() + collected.manufacturer.len(), 36);
        assert!(collected
            .manufacturer
            .iter()
            .all(|m| m.source_path.starts_with("M-")));
        assert!(collected
            .opaque
            .iter()
            .all(|e| !matches!(e.kind, OpaqueKind::ManufacturerData | OpaqueKind::Baggage)));
    }

    #[test]
    fn manufacturer_signatures_stay_in_the_opaque_store() {
        if !crate::testutil::corpus_available() {
            eprintln!("skip: OriginalData/ corpus not present (gitignored, local-only)");
            return;
        }
        // M-0008.signature signs a container state, not a product; leaving
        // it here keeps the export path for signatures unchanged (spec §3).
        let mut c = Container::open(reference_ets4_bytes()).unwrap();
        let collected = collect_container_entries(&mut c, &[]).unwrap();
        assert!(collected
            .opaque
            .iter()
            .any(|e| e.source_path == "M-0008.signature"));
        assert!(!collected
            .manufacturer
            .iter()
            .any(|m| m.source_path.ends_with(".signature")));
    }

    #[test]
    fn every_manufacturer_file_carries_the_hash_of_its_own_bytes() {
        if !crate::testutil::corpus_available() {
            eprintln!("skip: OriginalData/ corpus not present (gitignored, local-only)");
            return;
        }
        let mut c = Container::open(reference_ets4_bytes()).unwrap();
        let collected = collect_container_entries(&mut c, &[]).unwrap();
        assert!(collected
            .manufacturer
            .iter()
            .all(|m| m.sha256 == sha256_hex(&m.bytes)));
        // The vendor DLL travels with the manufacturer data, byte for byte.
        let dll = collected
            .manufacturer
            .iter()
            .find(|m| m.source_path.ends_with("econEts3.dll"))
            .unwrap();
        assert_eq!(dll.bytes.len(), 641536);
        assert_eq!(dll.kind, OpaqueKind::Baggage);
    }

    #[test]
    fn entries_are_classified_by_where_they_sit_in_the_container() {
        if !crate::testutil::corpus_available() {
            eprintln!("skip: OriginalData/ corpus not present (gitignored, local-only)");
            return;
        }
        let mut c = Container::open(reference_ets4_bytes()).unwrap();
        let collected = collect_container_entries(&mut c, &[]).unwrap();
        let kind = |p: &str| {
            collected
                .opaque
                .iter()
                .find(|e| e.source_path == p)
                .map(|e| e.kind)
                .or_else(|| {
                    collected
                        .manufacturer
                        .iter()
                        .find(|m| m.source_path == p)
                        .map(|m| m.kind)
                })
                .unwrap()
        };
        assert_eq!(kind("knx_master.xml"), OpaqueKind::MasterData);
        assert_eq!(kind("P-0512.signature"), OpaqueKind::Signature);
        assert_eq!(kind("M-0008/Baggages/econEts3.dll"), OpaqueKind::Baggage);
        assert_eq!(kind("M-0008/Catalog.xml"), OpaqueKind::ManufacturerData);
        assert_eq!(
            kind("P-0512/BinaryData/2868e24a-9fc3-4099-82f0-3f535bde8fc1.dat"),
            OpaqueKind::BinaryData
        );
        assert_eq!(kind("P-0512/ExtraData/20001.azp"), OpaqueKind::ExtraData);
    }

    #[test]
    fn a_retained_attribute_carries_its_element_path_and_name() {
        let a = RetainedAttribute {
            xpath: "/KNX/Project/Installations/Installation".into(),
            name: "BCUKey".into(),
            value: "4294967295".into(),
        };
        let e = from_retained_attribute("P-0512/0.xml", &a);
        assert_eq!(e.kind, OpaqueKind::RetainedAttribute);
        assert_eq!(e.name, "BCUKey");
        assert_eq!(e.bytes, b"4294967295");
        assert_eq!(e.sha256, sha256_hex(b"4294967295"));
    }
}
