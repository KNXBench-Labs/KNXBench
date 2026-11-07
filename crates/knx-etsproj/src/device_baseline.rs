//! Reads one device's original parameter values and object list from its retained topology XML.

use std::collections::HashMap;

use crate::known::{known_schema, uses_ets6_content_model};
use crate::source::SourceDevice;

/// KNXBench observation limit for re-reading one original member, not a format limit.
pub const MAX_BASELINE_XML_BYTES: usize = 64 * 1024 * 1024;

/// What the original import stated for one device, normalised as the importer maps it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceBaseline {
    /// `(ParameterInstanceRef/@RefId, @Value)`, a missing `Value` as `""` (as `map.rs` does).
    pub parameters: Vec<(String, String)>,
    /// `GroupObjectTree` object references, the importer's authoritative object list.
    pub objects: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BaselineReadError {
    SourceLimit,
    UnsupportedSchema,
    Unreadable,
    DeviceNotInSource,
    AmbiguousDevice,
}

/// Every device baseline of one parsed topology member, keyed by exact `DeviceInstance/@Id`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DocumentBaselines {
    /// More than one entry means the member declares that id more than once.
    devices: HashMap<String, Vec<DeviceBaseline>>,
}

impl DocumentBaselines {
    /// The single baseline of `ets_id`, or why there is none.
    pub fn device(&self, ets_id: &str) -> Result<&DeviceBaseline, BaselineReadError> {
        match self.devices.get(ets_id).map(Vec::as_slice) {
            None | Some([]) => Err(BaselineReadError::DeviceNotInSource),
            Some([single]) => Ok(single),
            Some(_) => Err(BaselineReadError::AmbiguousDevice),
        }
    }
}

/// Parses `xml` once with the importer's schema-20/21/23 parser and returns every device baseline.
///
/// Never reads files, URLs or external entities and never modifies `xml`.
pub fn read_document_baselines(
    xml: &[u8],
    source_path: &str,
    schema_version: u32,
) -> Result<DocumentBaselines, BaselineReadError> {
    if xml.len() > MAX_BASELINE_XML_BYTES {
        return Err(BaselineReadError::SourceLimit);
    }
    let schema = known_schema(schema_version)
        .filter(|_| uses_ets6_content_model(schema_version))
        .ok_or(BaselineReadError::UnsupportedSchema)?;
    let parsed = crate::parse::parse_installation_v21(xml, source_path, schema)
        .map_err(|_| BaselineReadError::Unreadable)?;
    let mut devices: HashMap<String, Vec<DeviceBaseline>> = HashMap::new();
    for device in parsed
        .document
        .installations
        .iter()
        .flat_map(|installation| {
            installation.unassigned_devices.iter().chain(
                installation
                    .areas
                    .iter()
                    .flat_map(|area| area.lines.iter().flat_map(|line| line.devices.iter())),
            )
        })
    {
        devices
            .entry(device.id.clone())
            .or_default()
            .push(baseline(device));
    }
    Ok(DocumentBaselines { devices })
}

/// [`read_document_baselines`] for a single device `ets_id`.
pub fn read_device_baseline(
    xml: &[u8],
    source_path: &str,
    schema_version: u32,
    ets_id: &str,
) -> Result<DeviceBaseline, BaselineReadError> {
    read_document_baselines(xml, source_path, schema_version)?
        .device(ets_id)
        .cloned()
}

fn baseline(device: &SourceDevice) -> DeviceBaseline {
    DeviceBaseline {
        parameters: device
            .parameters
            .iter()
            .map(|p| (p.ref_id.clone(), p.value.clone().unwrap_or_default()))
            .collect(),
        objects: device.group_object_tree.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PATH: &str = "P-TEST/0.xml";

    fn project(devices: &str) -> String {
        format!(
            r#"<?xml version="1.0" encoding="utf-8"?>
<KNX xmlns="http://knx.org/xml/project/23"><Project Id="P-TEST"><Installations>
<Installation Name="I"><Topology><Area Id="P-TEST-0_A-1" Address="1"><Line Id="P-TEST-0_L-1" Address="1">
<Segment Id="P-TEST-0_S-1" Number="0">{devices}</Segment></Line></Area>
<UnassignedDevices><DeviceInstance Id="P-TEST-0_DI-9" ProductRefId="M-1_P-1"
  Hardware2ProgramRefId="M-1_H-1_HP-1"><GroupObjectTree GroupObjectInstances="O-9_R-1" />
</DeviceInstance></UnassignedDevices></Topology></Installation></Installations></Project></KNX>"#
        )
    }

    const DEVICE: &str = r#"<DeviceInstance Id="P-TEST-0_DI-1" Address="1" ProductRefId="M-1_P-1"
  Hardware2ProgramRefId="M-1_H-1_HP-1"><ParameterInstanceRefs>
  <ParameterInstanceRef RefId="M-1_A-1_P-1_R-1" Value="7" />
  <ParameterInstanceRef RefId="M-1_A-1_P-2_R-1" />
  <ParameterInstanceRef RefId="M-1_A-1_P-3_R-1" Value=" x " />
</ParameterInstanceRefs>
<GroupObjectTree GroupObjectInstances="O-1_R-1 O-2_R-1" /></DeviceInstance>"#;

    #[test]
    fn reads_exact_segment_and_unassigned_devices_like_the_importer() {
        let xml = project(DEVICE);
        let base = read_device_baseline(xml.as_bytes(), PATH, 23, "P-TEST-0_DI-1").unwrap();
        assert_eq!(
            base.parameters,
            vec![
                ("M-1_A-1_P-1_R-1".into(), "7".into()),
                ("M-1_A-1_P-2_R-1".into(), String::new()),
                ("M-1_A-1_P-3_R-1".into(), " x ".into()),
            ]
        );
        assert_eq!(base.objects, vec!["O-1_R-1", "O-2_R-1"]);
        let unassigned = read_device_baseline(xml.as_bytes(), PATH, 23, "P-TEST-0_DI-9").unwrap();
        assert!(unassigned.parameters.is_empty());
        assert_eq!(unassigned.objects, vec!["O-9_R-1"]);
    }

    #[test]
    fn reads_a_schema20_device_directly_under_its_line() {
        // Schema 20 has no Segment: DeviceInstance is a direct Line child.
        let xml = r#"<?xml version="1.0" encoding="utf-8"?>
<KNX xmlns="http://knx.org/xml/project/20"><Project Id="P-TEST"><Installations>
<Installation Name="I"><Topology><Area Id="P-TEST-0_A-1" Address="1"><Line Id="P-TEST-0_L-1" Address="1">
<DeviceInstance Id="P-TEST-0_DI-4" Address="4" ProductRefId="M-1_P-1" Hardware2ProgramRefId="M-1_H-1_HP-1">
<ParameterInstanceRefs><ParameterInstanceRef RefId="P-4" Value="a&amp;b" /></ParameterInstanceRefs>
<GroupObjectTree GroupObjectInstances="O-4_R-1" /></DeviceInstance></Line></Area>
</Topology></Installation></Installations></Project></KNX>"#;
        let base = read_device_baseline(xml.as_bytes(), PATH, 20, "P-TEST-0_DI-4").unwrap();
        assert_eq!(base.parameters, vec![("P-4".into(), "a&b".into())]);
        assert_eq!(base.objects, vec!["O-4_R-1"]);
    }

    #[test]
    fn refuses_what_cannot_be_attributed_or_read() {
        use BaselineReadError as E;
        let xml = project(DEVICE);
        assert_eq!(
            read_device_baseline(xml.as_bytes(), PATH, 23, "P-TEST-0_DI-2"),
            Err(E::DeviceNotInSource)
        );
        // Exact id only: no prefix or case match.
        assert_eq!(
            read_device_baseline(xml.as_bytes(), PATH, 23, "p-test-0_di-1"),
            Err(E::DeviceNotInSource)
        );
        let twice = project(&format!("{DEVICE}{DEVICE}"));
        assert_eq!(
            read_device_baseline(twice.as_bytes(), PATH, 23, "P-TEST-0_DI-1"),
            Err(E::AmbiguousDevice)
        );
        for version in [11, 14, 22, 24] {
            assert_eq!(
                read_device_baseline(xml.as_bytes(), PATH, version, "P-TEST-0_DI-1"),
                Err(E::UnsupportedSchema),
                "schema {version}"
            );
        }
        assert_eq!(
            read_device_baseline(b"<KNX><Project", PATH, 23, "P-TEST-0_DI-1"),
            Err(E::Unreadable)
        );
        let huge = vec![b' '; MAX_BASELINE_XML_BYTES + 1];
        assert_eq!(
            read_device_baseline(&huge, PATH, 23, "P-TEST-0_DI-1"),
            Err(E::SourceLimit)
        );
    }
}
