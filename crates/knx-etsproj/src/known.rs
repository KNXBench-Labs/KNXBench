//! Per-schema-version tables of known elements and their known attributes.
//!
//! The table is evidence, not schema: no authoritative XSD is available for
//! any ETS project schema version (RESEARCH §2.2), so it lists what has been
//! measured in the reference projects. The tolerant parser (Task 6) reports
//! everything else rather than failing on it — an unknown element or
//! attribute is retained, not silently dropped and not a parse error.

/// One element's absolute path and the attribute names known to appear on
/// it, transcribed from a reference project's measured inventory.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KnownElement {
    /// Absolute element path, e.g.
    /// `/KNX/Project/Installations/Installation/Topology/Area/Line`.
    pub path: &'static str,
    pub attributes: &'static [&'static str],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KnownSchema {
    pub version: u32,
    pub elements: &'static [KnownElement],
}

/// The known-element table for `version`, or `None` if this session does not
/// yet know that schema version.
///
/// `known_schema(23)` is deliberately `None`: the schema-23 differences of
/// RESEARCH §3.3 are load-bearing (`RefId` loses its prefix, links move to
/// attributes, `GroupObjectTree` becomes the authoritative object list,
/// boolean spelling changes), and a table that pretended otherwise would
/// produce silently wrong data. Schema 23 import reports "no known-element
/// table for this schema version" and stops; that is the honest state until
/// schema 23 is implemented.
pub fn known_schema(version: u32) -> Option<&'static KnownSchema> {
    match version {
        11 => Some(&SCHEMA_11),
        _ => None,
    }
}

/// Transcribed from the ETS4 reference project's measured inventory: 27
/// element paths. `ProjectInformation` is measured from `Project.xml`
/// (`P-0512/Project.xml`, ETS4's spelling); every other path is measured
/// from `0.xml`. Both files share the same root `<KNX>` element and
/// namespace, so one table covers both (IMPORT_EXPORT §2, RESEARCH §2.1).
pub const SCHEMA_11: KnownSchema = KnownSchema {
    version: 11,
    elements: &[
        KnownElement {
            path: "/KNX",
            attributes: &["CreatedBy", "ToolVersion"],
        },
        KnownElement {
            path: "/KNX/Project",
            attributes: &["Id"],
        },
        KnownElement {
            path: "/KNX/Project/ProjectInformation",
            attributes: &[
                "Name",
                "LastModified",
                "ProjectStart",
                "ProjectId",
                "ProjectTracingLevel",
                "Hide16BitGroupsFromLegacyPlugins",
                "GroupAddressStyle",
                "CompletionStatus",
            ],
        },
        KnownElement {
            path: "/KNX/Project/Installations",
            attributes: &[],
        },
        KnownElement {
            path: "/KNX/Project/Installations/Installation",
            attributes: &[
                "InstallationId",
                "Name",
                "BCUKey",
                "DefaultLine",
                "IPRoutingMulticastAddress",
                "SplitType",
                "CompletionStatus",
            ],
        },
        KnownElement {
            path: "/KNX/Project/Installations/Installation/Topology",
            attributes: &[],
        },
        KnownElement {
            path: "/KNX/Project/Installations/Installation/Topology/Area",
            attributes: &["Id", "Name", "Address", "CompletionStatus"],
        },
        KnownElement {
            path: "/KNX/Project/Installations/Installation/Topology/Area/Line",
            attributes: &[
                "Id",
                "Name",
                "Address",
                "MediumTypeRefId",
                "DomainAddress",
                "DomainAddressIsChecked",
                "CompletionStatus",
                "IPRoutingMulticastAddress",
                "MulticastTTL",
            ],
        },
        KnownElement {
            path: "/KNX/Project/Installations/Installation/Topology/Area/Line/BusAccess",
            attributes: &["Name", "Edi", "Parameter"],
        },
        KnownElement {
            path: "/KNX/Project/Installations/Installation/Topology/Area/Line/DeviceInstance",
            attributes: &[
                "Id",
                "Name",
                "Description",
                "Address",
                "ProductRefId",
                "Hardware2ProgramRefId",
                "LastModified",
                "LastDownload",
                "CompletionStatus",
                "IndividualAddressLoaded",
                "ApplicationProgramLoaded",
                "ParametersLoaded",
                "CommunicationPartLoaded",
                "MediumConfigLoaded",
                "IsCommunicationObjectVisibilityCalculated",
                "Broken",
            ],
        },
        KnownElement {
            path: "/KNX/Project/Installations/Installation/Topology/Area/Line/DeviceInstance/ParameterInstanceRefs",
            attributes: &[],
        },
        KnownElement {
            path: "/KNX/Project/Installations/Installation/Topology/Area/Line/DeviceInstance/ParameterInstanceRefs/ParameterInstanceRef",
            attributes: &["RefId", "Value"],
        },
        KnownElement {
            path: "/KNX/Project/Installations/Installation/Topology/Area/Line/DeviceInstance/ComObjectInstanceRefs",
            attributes: &[],
        },
        KnownElement {
            path: "/KNX/Project/Installations/Installation/Topology/Area/Line/DeviceInstance/ComObjectInstanceRefs/ComObjectInstanceRef",
            attributes: &[
                "RefId",
                "IsActive",
                "DatapointType",
                "Text",
                "Description",
                "ReadFlag",
                "WriteFlag",
                "TransmitFlag",
                "UpdateFlag",
                "CommunicationFlag",
            ],
        },
        KnownElement {
            path: "/KNX/Project/Installations/Installation/Topology/Area/Line/DeviceInstance/ComObjectInstanceRefs/ComObjectInstanceRef/Connectors",
            attributes: &[],
        },
        KnownElement {
            path: "/KNX/Project/Installations/Installation/Topology/Area/Line/DeviceInstance/ComObjectInstanceRefs/ComObjectInstanceRef/Connectors/Send",
            attributes: &["GroupAddressRefId"],
        },
        KnownElement {
            path: "/KNX/Project/Installations/Installation/Topology/Area/Line/DeviceInstance/ComObjectInstanceRefs/ComObjectInstanceRef/Connectors/Receive",
            attributes: &["GroupAddressRefId"],
        },
        KnownElement {
            // The wrapper form: no attributes of its own, holds leaf
            // `BinaryData` children.
            path: "/KNX/Project/Installations/Installation/Topology/Area/Line/DeviceInstance/BinaryData",
            attributes: &[],
        },
        KnownElement {
            // The leaf form, same element name as its wrapper.
            path: "/KNX/Project/Installations/Installation/Topology/Area/Line/DeviceInstance/BinaryData/BinaryData",
            attributes: &["Id", "Name"],
        },
        KnownElement {
            path: "/KNX/Project/Installations/Installation/UnassignedDevices",
            attributes: &[],
        },
        KnownElement {
            path: "/KNX/Project/Installations/Installation/Buildings",
            attributes: &[],
        },
        KnownElement {
            path: "/KNX/Project/Installations/Installation/Buildings/BuildingPart",
            attributes: &[
                "Id",
                "Name",
                "Number",
                "Type",
                "DefaultLine",
                "CompletionStatus",
            ],
        },
        KnownElement {
            path: "/KNX/Project/Installations/Installation/Buildings/BuildingPart/DeviceInstanceRef",
            attributes: &["RefId"],
        },
        KnownElement {
            path: "/KNX/Project/Installations/Installation/GroupAddresses",
            attributes: &[],
        },
        KnownElement {
            path: "/KNX/Project/Installations/Installation/GroupAddresses/GroupRanges",
            attributes: &[],
        },
        KnownElement {
            path: "/KNX/Project/Installations/Installation/GroupAddresses/GroupRanges/GroupRange",
            attributes: &["Id", "Name", "RangeStart", "RangeEnd"],
        },
        KnownElement {
            path: "/KNX/Project/Installations/Installation/GroupAddresses/GroupRanges/GroupRange/GroupAddress",
            attributes: &["Id", "Name", "Address", "Central", "Unfiltered"],
        },
    ],
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schema_eleven_knows_the_measured_element_paths() {
        let s = known_schema(11).unwrap();
        let line = s
            .elements
            .iter()
            .find(|e| e.path == "/KNX/Project/Installations/Installation/Topology/Area/Line")
            .unwrap();
        assert!(line.attributes.contains(&"MediumTypeRefId"));
        assert!(line.attributes.contains(&"MulticastTTL"));
        assert!(!line.attributes.contains(&"Puid")); // schema 23 only
    }

    #[test]
    fn an_unknown_schema_version_has_no_table() {
        assert!(known_schema(23).is_none());
    }

    #[test]
    fn every_known_path_is_absolute_and_unique() {
        let s = known_schema(11).unwrap();
        let mut paths: Vec<_> = s.elements.iter().map(|e| e.path).collect();
        let before = paths.len();
        paths.sort_unstable();
        paths.dedup();
        assert_eq!(paths.len(), before, "duplicate element path in SCHEMA_11");
        assert!(s.elements.iter().all(|e| e.path.starts_with("/KNX")));
    }

    #[test]
    fn the_table_has_exactly_the_measured_element_count() {
        // 26 distinct element names; BinaryData occurs as both a wrapper
        // (no attributes) and a leaf (Id, Name), so 27 distinct paths.
        assert_eq!(known_schema(11).unwrap().elements.len(), 27);
    }
}
