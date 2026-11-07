//! Says whether a device still matches its original import, so saved folders are not overstated.
//!
//! Contract: `docs/contracts/saved-folder-currency.md`. The saved `GroupObjectTree`
//! describes the device as imported. This module compares the device's current
//! parameter values and object set with the retained original topology member
//! and never changes, hides or recomputes a folder.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use knx_core::{DeviceId, Project, SourceRef};
use knx_etsproj::device_baseline::{
    read_document_baselines, BaselineReadError, DocumentBaselines, MAX_BASELINE_XML_BYTES,
};
use knx_store::StoredOpaqueEntry;

/// Session-only memo of the last parsed original member, keyed by its verified digest.
///
/// Holds derived baselines, never project data, and is never saved. Every lookup
/// still verifies the retained bytes' SHA-256 before the memo can answer, so a
/// changed original is re-parsed rather than served from memory.
#[derive(Debug, Default)]
pub struct BaselineCache {
    last: Mutex<Option<(BaselineKey, ParsedDocument)>>,
    parses: AtomicUsize,
}

type ParsedDocument = Arc<Result<DocumentBaselines, BaselineReadError>>;

#[derive(Debug, Clone, PartialEq, Eq)]
struct BaselineKey {
    sha256: String,
    source_path: String,
    schema_version: u32,
}

impl BaselineCache {
    /// How many times a member was actually parsed (tests and diagnostics).
    pub fn parses(&self) -> usize {
        self.parses.load(Ordering::Relaxed)
    }

    fn document(&self, key: BaselineKey, bytes: &[u8]) -> ParsedDocument {
        let mut last = self
            .last
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if let Some((cached, document)) = last.as_ref() {
            if *cached == key {
                return Arc::clone(document);
            }
        }
        self.parses.fetch_add(1, Ordering::Relaxed);
        let document = Arc::new(read_document_baselines(
            bytes,
            &key.source_path,
            key.schema_version,
        ));
        *last = Some((key, Arc::clone(&document)));
        document
    }
}

/// Whether the saved tree still describes the device as it is now.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SavedTreeCurrency {
    Unchanged,
    ChangedSinceImport,
    Unknown(CurrencyUnknown),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CurrencyUnknown {
    NoRetainedSource,
    AmbiguousSource,
    SourceIntegrity,
    SourceLimit,
    UnsupportedSchema,
    Unreadable,
    DeviceNotInSource,
    AmbiguousDevice,
}

/// What the comparison needs from the project, gathered under the project lock.
#[derive(Debug, Clone)]
pub struct CurrencyInput {
    source: SourceRef,
    schema_version: u32,
    /// Another device states the same source identity.
    shared_source: bool,
    parameters: Vec<(String, String)>,
    objects: Vec<String>,
}

impl CurrencyInput {
    /// `None` when the device does not exist.
    pub fn collect(project: &Project, device: DeviceId) -> Option<Self> {
        let dev = project.devices.get(device)?;
        let parameters = project
            .installations
            .iter()
            .flat_map(|installation| &installation.parameters)
            .filter(|p| p.device == device)
            .map(|p| (p.source.ets_id.clone(), p.raw.clone()))
            .collect();
        let objects = dev
            .com_objects
            .iter()
            .filter_map(|id| project.devices.com_object(*id))
            .map(|com| com.source.ets_id.clone())
            .collect();
        Some(Self {
            source: dev.source.clone(),
            schema_version: project.info.ets_schema_version,
            shared_source: project
                .devices
                .iter()
                .any(|other| other.id != dev.id && other.source == dev.source),
            parameters,
            objects,
        })
    }
}

/// Compares the device in `input` with its retained original (order-insensitive multisets).
pub fn saved_tree_currency(
    input: CurrencyInput,
    opaque: &[StoredOpaqueEntry],
) -> SavedTreeCurrency {
    saved_tree_currency_cached(input, opaque, &BaselineCache::default())
}

/// [`saved_tree_currency`] reusing `cache` for an unchanged, re-verified original member.
pub fn saved_tree_currency_cached(
    input: CurrencyInput,
    opaque: &[StoredOpaqueEntry],
    cache: &BaselineCache,
) -> SavedTreeCurrency {
    use CurrencyUnknown as U;
    let unknown = SavedTreeCurrency::Unknown;
    if input.source.path.is_empty() || input.source.ets_id.is_empty() {
        return unknown(U::NoRetainedSource);
    }
    if input.shared_source {
        return unknown(U::AmbiguousSource);
    }
    let mut originals = opaque.iter().filter(|entry| {
        entry.source_path == input.source.path
            && entry.xpath.is_empty()
            && entry.name.is_empty()
            && entry.kind == "ContainerEntry"
    });
    let Some(original) = originals.next() else {
        return unknown(U::NoRetainedSource);
    };
    if originals.next().is_some() {
        return unknown(U::AmbiguousSource);
    }
    // Refuse oversized members before hashing, as the reader would before parsing.
    if original.bytes.len() > MAX_BASELINE_XML_BYTES {
        return unknown(U::SourceLimit);
    }
    if knx_etsproj::opaque::sha256_hex(&original.bytes) != original.sha256 {
        return unknown(U::SourceIntegrity);
    }
    let document = cache.document(
        BaselineKey {
            sha256: original.sha256.clone(),
            source_path: input.source.path.clone(),
            schema_version: input.schema_version,
        },
        &original.bytes,
    );
    let baseline = match document
        .as_ref()
        .as_ref()
        .map_err(|error| *error)
        .and_then(|document| document.device(&input.source.ets_id))
    {
        Ok(baseline) => baseline.clone(),
        Err(error) => {
            return unknown(match error {
                BaselineReadError::SourceLimit => U::SourceLimit,
                BaselineReadError::UnsupportedSchema => U::UnsupportedSchema,
                BaselineReadError::Unreadable => U::Unreadable,
                BaselineReadError::DeviceNotInSource => U::DeviceNotInSource,
                BaselineReadError::AmbiguousDevice => U::AmbiguousDevice,
            })
        }
    };
    let (mut was_parameters, mut was_objects) = (baseline.parameters, baseline.objects);
    let (mut now_parameters, mut now_objects) = (input.parameters, input.objects);
    for list in [&mut was_parameters, &mut now_parameters] {
        list.sort_unstable();
    }
    for list in [&mut was_objects, &mut now_objects] {
        list.sort_unstable();
    }
    if was_parameters == now_parameters && was_objects == now_objects {
        SavedTreeCurrency::Unchanged
    } else {
        SavedTreeCurrency::ChangedSinceImport
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PATH: &str = "P-TEST/0.xml";
    const XML: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<KNX xmlns="http://knx.org/xml/project/23"><Project Id="P-TEST"><Installations>
<Installation Name="I"><Topology><UnassignedDevices>
<DeviceInstance Id="P-TEST-0_DI-1" ProductRefId="M-1_P-1" Hardware2ProgramRefId="M-1_H-1_HP-1">
<ParameterInstanceRefs><ParameterInstanceRef RefId="P-1" Value="7" /><ParameterInstanceRef RefId="P-2" />
</ParameterInstanceRefs><GroupObjectTree GroupObjectInstances="O-1_R-1 O-2_R-1" /></DeviceInstance>
</UnassignedDevices></Topology></Installation></Installations></Project></KNX>"#;

    fn entry(bytes: &[u8], sha: Option<String>) -> StoredOpaqueEntry {
        StoredOpaqueEntry {
            kind: "ContainerEntry".into(),
            source_path: PATH.into(),
            xpath: String::new(),
            name: String::new(),
            sha256: sha.unwrap_or_else(|| knx_etsproj::opaque::sha256_hex(bytes)),
            bytes: bytes.to_vec(),
        }
    }

    fn input(parameters: &[(&str, &str)], objects: &[&str]) -> CurrencyInput {
        CurrencyInput {
            source: SourceRef {
                path: PATH.into(),
                ets_id: "P-TEST-0_DI-1".into(),
            },
            schema_version: 23,
            shared_source: false,
            parameters: parameters
                .iter()
                .map(|(r, v)| ((*r).into(), (*v).into()))
                .collect(),
            objects: objects.iter().map(|o| (*o).into()).collect(),
        }
    }

    fn currency(input: CurrencyInput) -> SavedTreeCurrency {
        saved_tree_currency(input, &[entry(XML.as_bytes(), None)])
    }

    #[test]
    fn equal_parameters_and_objects_in_any_order_are_unchanged() {
        assert_eq!(
            currency(input(&[("P-2", ""), ("P-1", "7")], &["O-2_R-1", "O-1_R-1"])),
            SavedTreeCurrency::Unchanged
        );
    }

    #[test]
    fn any_parameter_or_object_difference_is_changed() {
        let objects = ["O-1_R-1", "O-2_R-1"];
        for parameters in [
            vec![("P-1", "8"), ("P-2", "")],
            vec![("P-1", "7")],
            vec![("P-1", "7"), ("P-2", ""), ("P-3", "1")],
            vec![("P-1", "7"), ("P-2", " ")],
            vec![("P-1", "7"), ("P-2", ""), ("P-2", "")],
        ] {
            assert_eq!(
                currency(input(&parameters, &objects)),
                SavedTreeCurrency::ChangedSinceImport,
                "{parameters:?}"
            );
        }
        let parameters = [("P-1", "7"), ("P-2", "")];
        for objects in [
            vec!["O-1_R-1"],
            vec!["O-1_R-1", "O-2_R-1", "O-3_R-1"],
            vec![],
        ] {
            assert_eq!(
                currency(input(&parameters, &objects)),
                SavedTreeCurrency::ChangedSinceImport,
                "{objects:?}"
            );
        }
    }

    #[test]
    fn unattributable_or_unverified_originals_are_unknown_never_unchanged() {
        use CurrencyUnknown as U;
        let ok = || input(&[("P-1", "7"), ("P-2", "")], &["O-1_R-1", "O-2_R-1"]);
        let bytes = XML.as_bytes();
        let unknown =
            |opaque: &[StoredOpaqueEntry], input: CurrencyInput| saved_tree_currency(input, opaque);
        assert_eq!(
            unknown(&[], ok()),
            SavedTreeCurrency::Unknown(U::NoRetainedSource)
        );
        assert_eq!(
            unknown(&[entry(bytes, None), entry(bytes, None)], ok()),
            SavedTreeCurrency::Unknown(U::AmbiguousSource)
        );
        assert_eq!(
            unknown(&[entry(bytes, Some("0".repeat(64)))], ok()),
            SavedTreeCurrency::Unknown(U::SourceIntegrity)
        );
        let mut other_row = entry(bytes, None);
        other_row.xpath = "/KNX/Project".into();
        assert_eq!(
            unknown(&[other_row], ok()),
            SavedTreeCurrency::Unknown(U::NoRetainedSource)
        );
        let mut shared = ok();
        shared.shared_source = true;
        assert_eq!(
            unknown(&[entry(bytes, None)], shared),
            SavedTreeCurrency::Unknown(U::AmbiguousSource)
        );
        let mut old = ok();
        old.schema_version = 11;
        assert_eq!(
            unknown(&[entry(bytes, None)], old),
            SavedTreeCurrency::Unknown(U::UnsupportedSchema)
        );
        let mut missing = ok();
        missing.source.ets_id = "P-TEST-0_DI-2".into();
        assert_eq!(
            unknown(&[entry(bytes, None)], missing),
            SavedTreeCurrency::Unknown(U::DeviceNotInSource)
        );
        assert_eq!(
            unknown(&[entry(b"<KNX><", None)], ok()),
            SavedTreeCurrency::Unknown(U::Unreadable)
        );
        let mut nameless = ok();
        nameless.source.path.clear();
        assert_eq!(
            unknown(&[entry(bytes, None)], nameless),
            SavedTreeCurrency::Unknown(U::NoRetainedSource)
        );
    }

    /// Two installations, one device each; each device's parameters live in its own installation.
    fn imported_two_installations() -> (Project, Vec<StoredOpaqueEntry>) {
        let device = |installation: u32, id: u32, value: &str| {
            format!(
                r#"<Installation Name="I{installation}"><Topology><UnassignedDevices>
<DeviceInstance Id="P-0001-0_DI-{id}" ProductRefId="M-1_P-1" Hardware2ProgramRefId="M-1_H-1_HP-1">
<ParameterInstanceRefs><ParameterInstanceRef RefId="M-1_A-1_P-{id}_R-1" Value="{value}" />
</ParameterInstanceRefs><GroupObjectTree GroupObjectInstances="O-{id}_R-1" /></DeviceInstance>
</UnassignedDevices></Topology></Installation>"#
            )
        };
        let topology = format!(
            r#"<KNX xmlns="http://knx.org/xml/project/23"><Project Id="P-0001"><Installations>{}{}</Installations></Project></KNX>"#,
            device(1, 1, "one"),
            device(2, 2, "two"),
        );
        let info = r#"<KNX xmlns="http://knx.org/xml/project/23"><Project Id="P-0001"><ProjectInformation Name="Two"/></Project></KNX>"#;
        let bytes = knx_testsupport::zip_with_entries(&[
            ("P-0001.signature", b""),
            ("P-0001/0.xml", topology.as_bytes()),
            ("P-0001/project.xml", info.as_bytes()),
        ]);
        let outcome = knx_etsproj::import_knxproj_bytes(bytes, "two.knxproj").unwrap();
        let opaque = outcome
            .opaque
            .iter()
            .map(|e| StoredOpaqueEntry {
                source_path: e.source_path.clone(),
                xpath: e.xpath.clone(),
                kind: format!("{:?}", e.kind),
                name: e.name.clone(),
                bytes: e.bytes.clone(),
                sha256: e.sha256.clone(),
            })
            .collect();
        (outcome.project, opaque)
    }

    fn device_by_source(project: &Project, ets_id: &str) -> DeviceId {
        project
            .devices
            .iter()
            .find(|d| d.source.ets_id == ets_id)
            .unwrap()
            .id
    }

    #[test]
    fn collect_reads_only_this_device_from_every_installation() {
        let (project, opaque) = imported_two_installations();
        assert_eq!(project.installations.len(), 2);
        for (ets_id, own) in [("P-0001-0_DI-1", "one"), ("P-0001-0_DI-2", "two")] {
            let device = device_by_source(&project, ets_id);
            let input = CurrencyInput::collect(&project, device).unwrap();
            // Only this device's own value, wherever its installation is.
            assert_eq!(input.parameters.len(), 1, "{ets_id}");
            assert_eq!(input.parameters[0].1, own, "{ets_id}");
            assert_eq!(
                saved_tree_currency(input, &opaque),
                SavedTreeCurrency::Unchanged,
                "{ets_id}"
            );
        }
    }

    #[test]
    fn a_source_identity_shared_by_two_devices_is_unknown() {
        let (mut project, opaque) = imported_two_installations();
        let first = device_by_source(&project, "P-0001-0_DI-1");
        let second = device_by_source(&project, "P-0001-0_DI-2");
        let source = project.devices.get(first).unwrap().source.clone();
        project.devices.get_mut(second).unwrap().source = source;
        let input = CurrencyInput::collect(&project, first).unwrap();
        assert_eq!(
            saved_tree_currency(input, &opaque),
            SavedTreeCurrency::Unknown(CurrencyUnknown::AmbiguousSource)
        );
    }

    #[test]
    fn the_cache_parses_once_but_still_verifies_and_follows_a_changed_original() {
        let (project, mut opaque) = imported_two_installations();
        let cache = BaselineCache::default();
        let input =
            |ets_id| CurrencyInput::collect(&project, device_by_source(&project, ets_id)).unwrap();
        for ets_id in ["P-0001-0_DI-1", "P-0001-0_DI-2", "P-0001-0_DI-1"] {
            assert_eq!(
                saved_tree_currency_cached(input(ets_id), &opaque, &cache),
                SavedTreeCurrency::Unchanged
            );
        }
        assert_eq!(cache.parses(), 1, "one member, one parse");
        let original = opaque
            .iter()
            .position(|e| {
                e.kind == "ContainerEntry" && e.source_path == "P-0001/0.xml" && e.xpath.is_empty()
            })
            .unwrap();
        // Tampered bytes under the old digest: refused before the memo answers.
        opaque[original].bytes.push(b' ');
        assert_eq!(
            saved_tree_currency_cached(input("P-0001-0_DI-1"), &opaque, &cache),
            SavedTreeCurrency::Unknown(CurrencyUnknown::SourceIntegrity)
        );
        assert_eq!(cache.parses(), 1);
        // A genuinely different original (new digest) is parsed again, not served stale.
        let changed = String::from_utf8(opaque[original].bytes.clone())
            .unwrap()
            .replace(r#"Value="one""#, r#"Value="uno""#);
        opaque[original].bytes = changed.into_bytes();
        opaque[original].sha256 = knx_etsproj::opaque::sha256_hex(&opaque[original].bytes);
        assert_eq!(
            saved_tree_currency_cached(input("P-0001-0_DI-1"), &opaque, &cache),
            SavedTreeCurrency::ChangedSinceImport
        );
        assert_eq!(cache.parses(), 2);
    }
}
