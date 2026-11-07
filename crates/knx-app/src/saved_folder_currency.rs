//! Says whether a device still matches its original import, so saved folders are not overstated.
//!
//! Contract: `docs/contracts/saved-folder-currency.md`. The saved `GroupObjectTree`
//! describes the device as imported. This module compares the device's current
//! parameter values and object set with the retained original topology member
//! and never changes, hides or recomputes a folder.

use knx_core::{DeviceId, Project, SourceRef};
use knx_etsproj::device_baseline::{
    read_device_baseline, BaselineReadError, MAX_BASELINE_XML_BYTES,
};
use knx_store::StoredOpaqueEntry;

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
    let baseline = match read_device_baseline(
        &original.bytes,
        &input.source.path,
        input.schema_version,
        &input.source.ets_id,
    ) {
        Ok(baseline) => baseline,
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
}
