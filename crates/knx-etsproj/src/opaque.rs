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
//! `ManufacturerData` is a Session 3 arrangement, not the destination.
//! ADR-0005 and IMPORT_EXPORT §10 put manufacturer data in the shared
//! product database (Session 4), stored once rather than once per project.
//! Until that exists, keeping it in the per-project opaque store is what
//! lets export write a complete container instead of one ETS cannot read.

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

/// Reads every container entry except those named in `regenerated` (paths
/// this session's own exporter writes fresh, so keeping the original bytes
/// too would just mean export has to choose between two copies). Matched
/// case-insensitively, the same convention [`Container::find`] uses.
pub fn collect_container_entries(
    container: &mut Container,
    regenerated: &[&str],
) -> Result<Vec<OpaqueEntry>, ContainerError> {
    let paths: Vec<String> = container.entries().iter().map(|e| e.path.clone()).collect();
    let mut out = Vec::with_capacity(paths.len());
    for path in paths {
        if regenerated.iter().any(|r| r.eq_ignore_ascii_case(&path)) {
            continue;
        }
        let bytes = container.read(&path)?;
        let sha256 = sha256_hex(&bytes);
        out.push(OpaqueEntry {
            source_path: path.clone(),
            xpath: String::new(),
            kind: classify(&path),
            name: String::new(),
            bytes,
            sha256,
        });
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
    fn every_container_entry_except_the_regenerated_ones_is_collected() {
        let mut c = Container::open(reference_ets4_bytes()).unwrap();
        let entries =
            collect_container_entries(&mut c, &["P-0512/0.xml", "P-0512/Project.xml"]).unwrap();
        assert_eq!(entries.len(), 36); // 38 archive entries less the two we rewrite
        assert!(entries.iter().all(|e| !e.sha256.is_empty()));
    }

    #[test]
    fn entries_are_classified_by_where_they_sit_in_the_container() {
        let mut c = Container::open(reference_ets4_bytes()).unwrap();
        let entries = collect_container_entries(&mut c, &[]).unwrap();
        let kind = |p: &str| entries.iter().find(|e| e.source_path == p).unwrap().kind;
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
    fn the_baggage_dll_is_copied_byte_for_byte() {
        let mut c = Container::open(reference_ets4_bytes()).unwrap();
        let entries = collect_container_entries(&mut c, &[]).unwrap();
        let dll = entries
            .iter()
            .find(|e| e.source_path.ends_with("econEts3.dll"))
            .unwrap();
        assert_eq!(dll.bytes.len(), 641536);
        assert_eq!(&dll.bytes[..2], b"MZ"); // a PE image, and we do nothing with it
        assert_eq!(dll.sha256, sha256_hex(&dll.bytes));
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
