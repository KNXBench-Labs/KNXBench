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
/// Schema 21 and 23 both account for the load-bearing content-model
/// differences of RESEARCH §3.3/§3.4 (`RefId` losing its prefix, links moving
/// to attributes, `GroupObjectTree` becoming the authoritative object list,
/// boolean spelling changes, the `Segment` level, `Locations`/`Space`
/// replacing `Buildings`/`BuildingPart`) — see [`SCHEMA_21`] and
/// [`SCHEMA_23`]'s own doc comments for what is and is not yet covered.
pub fn known_schema(version: u32) -> Option<&'static KnownSchema> {
    match version {
        11 => Some(&SCHEMA_11),
        21 => Some(&SCHEMA_21),
        23 => Some(&SCHEMA_23),
        _ => None,
    }
}

/// `DeviceInstance`'s attribute list and the shape of its subtree do not
/// depend on where it sits: the reference project has `DeviceInstance`
/// under both `Line` (assigned to a line) and `UnassignedDevices` (parsed
/// off the bus, not yet placed) with identical attributes and identical
/// children. Shared here so the two locations' table entries cannot drift
/// apart.
const DEVICE_INSTANCE_ATTRS: &[&str] = &[
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
];
const PARAMETER_INSTANCE_REF_ATTRS: &[&str] = &["RefId", "Value"];
const COM_OBJECT_INSTANCE_REF_ATTRS: &[&str] = &[
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
];
const CONNECTOR_ATTRS: &[&str] = &["GroupAddressRefId"];
const BINARY_DATA_LEAF_ATTRS: &[&str] = &["Id", "Name"];
/// Schema ≥21's own spelling: measured (Task 3) on the ETS 6.3.0 reference
/// project, whose six `BinaryData` leaves all carry `DoNotCopy`.
const BINARY_DATA_LEAF_ATTRS_21: &[&str] = &["Id", "Name", "DoNotCopy"];

/// Transcribed from the ETS4 reference project's measured inventory: 37
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
            attributes: DEVICE_INSTANCE_ATTRS,
        },
        KnownElement {
            path: "/KNX/Project/Installations/Installation/Topology/Area/Line/DeviceInstance/ParameterInstanceRefs",
            attributes: &[],
        },
        KnownElement {
            path: "/KNX/Project/Installations/Installation/Topology/Area/Line/DeviceInstance/ParameterInstanceRefs/ParameterInstanceRef",
            attributes: PARAMETER_INSTANCE_REF_ATTRS,
        },
        KnownElement {
            path: "/KNX/Project/Installations/Installation/Topology/Area/Line/DeviceInstance/ComObjectInstanceRefs",
            attributes: &[],
        },
        KnownElement {
            path: "/KNX/Project/Installations/Installation/Topology/Area/Line/DeviceInstance/ComObjectInstanceRefs/ComObjectInstanceRef",
            attributes: COM_OBJECT_INSTANCE_REF_ATTRS,
        },
        KnownElement {
            path: "/KNX/Project/Installations/Installation/Topology/Area/Line/DeviceInstance/ComObjectInstanceRefs/ComObjectInstanceRef/Connectors",
            attributes: &[],
        },
        KnownElement {
            path: "/KNX/Project/Installations/Installation/Topology/Area/Line/DeviceInstance/ComObjectInstanceRefs/ComObjectInstanceRef/Connectors/Send",
            attributes: CONNECTOR_ATTRS,
        },
        KnownElement {
            path: "/KNX/Project/Installations/Installation/Topology/Area/Line/DeviceInstance/ComObjectInstanceRefs/ComObjectInstanceRef/Connectors/Receive",
            attributes: CONNECTOR_ATTRS,
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
            attributes: BINARY_DATA_LEAF_ATTRS,
        },
        KnownElement {
            // Measured (Task 6): nested inside `Topology`, a sibling of
            // `Area`, not a sibling of `Topology` under `Installation` as
            // first transcribed in Task 5. `grep -n` against the reference
            // project's `0.xml` shows `</Area>` immediately followed by
            // `<UnassignedDevices>` before `</Topology>` closes.
            path: "/KNX/Project/Installations/Installation/Topology/UnassignedDevices",
            attributes: &[],
        },
        KnownElement {
            // Same element, same attributes, same subtree shape as
            // `.../Line/DeviceInstance` — an unassigned device just has no
            // `Line` parent yet. Measured (Task 6) at the same reference
            // project location as `UnassignedDevices` above.
            path: "/KNX/Project/Installations/Installation/Topology/UnassignedDevices/DeviceInstance",
            attributes: DEVICE_INSTANCE_ATTRS,
        },
        KnownElement {
            path: "/KNX/Project/Installations/Installation/Topology/UnassignedDevices/DeviceInstance/ParameterInstanceRefs",
            attributes: &[],
        },
        KnownElement {
            path: "/KNX/Project/Installations/Installation/Topology/UnassignedDevices/DeviceInstance/ParameterInstanceRefs/ParameterInstanceRef",
            attributes: PARAMETER_INSTANCE_REF_ATTRS,
        },
        KnownElement {
            path: "/KNX/Project/Installations/Installation/Topology/UnassignedDevices/DeviceInstance/ComObjectInstanceRefs",
            attributes: &[],
        },
        KnownElement {
            path: "/KNX/Project/Installations/Installation/Topology/UnassignedDevices/DeviceInstance/ComObjectInstanceRefs/ComObjectInstanceRef",
            attributes: COM_OBJECT_INSTANCE_REF_ATTRS,
        },
        KnownElement {
            path: "/KNX/Project/Installations/Installation/Topology/UnassignedDevices/DeviceInstance/ComObjectInstanceRefs/ComObjectInstanceRef/Connectors",
            attributes: &[],
        },
        KnownElement {
            path: "/KNX/Project/Installations/Installation/Topology/UnassignedDevices/DeviceInstance/ComObjectInstanceRefs/ComObjectInstanceRef/Connectors/Send",
            attributes: CONNECTOR_ATTRS,
        },
        KnownElement {
            path: "/KNX/Project/Installations/Installation/Topology/UnassignedDevices/DeviceInstance/ComObjectInstanceRefs/ComObjectInstanceRef/Connectors/Receive",
            attributes: CONNECTOR_ATTRS,
        },
        KnownElement {
            path: "/KNX/Project/Installations/Installation/Topology/UnassignedDevices/DeviceInstance/BinaryData",
            attributes: &[],
        },
        KnownElement {
            path: "/KNX/Project/Installations/Installation/Topology/UnassignedDevices/DeviceInstance/BinaryData/BinaryData",
            attributes: BINARY_DATA_LEAF_ATTRS,
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

/// `DeviceInstance`'s attribute list at schema ≥21 (RESEARCH §3.3/§3.4):
/// `Comment`/`SerialNumber`/`IsActivityCalculated`/`LastUsedAPDULength`/
/// `ReadMaxAPDULength`/`Puid` are new; `CompletionStatus`,
/// `IsCommunicationObjectVisibilityCalculated` and `Broken` are gone
/// (measured against `KV v2.5 - demo.knxproj`, all 4 devices).
///
/// `LoadedImage`, `CheckSums` and `DownloadCounter` (C14) are documented,
/// not measured: `Project Schema23 v01.00.00.pdf`, p. 44, lists all three as
/// optional `DeviceInstance` attributes holding ETS's differential-download
/// state from the device's last download. None of the three reference
/// projects (schema 11, 21 or 23) happens to carry them — ETS only writes
/// them after a real download, and the reference projects were exported
/// without one — so there is no local measurement to cite, only the spec.
const DEVICE_INSTANCE_ATTRS_21: &[&str] = &[
    "Id",
    "Name",
    "Address",
    "ProductRefId",
    "Hardware2ProgramRefId",
    "Comment",
    "Description",
    "SerialNumber",
    "ApplicationProgramLoaded",
    "CommunicationPartLoaded",
    "IndividualAddressLoaded",
    "MediumConfigLoaded",
    "ParametersLoaded",
    "IsActivityCalculated",
    "CompletionStatus",
    "InstallationHints",
    "LastModified",
    "LastDownload",
    "LastUsedAPDULength",
    "ReadMaxAPDULength",
    "Puid",
    "LoadedImage",
    "CheckSums",
    "DownloadCounter",
];
/// `ComObjectInstanceRef`'s attribute list at schema ≥21. Measured (RESEARCH
/// §3.4): every instance in the reference project carries only `RefId`,
/// `ChannelId` and `Links` — the override attributes (`IsActive`,
/// `DatapointType`, `Text`, `Description`) never actually appear on
/// module-based devices there, but are kept in the known table since they
/// are attested for schema 23 (RESEARCH §3.3) on the same element and are
/// expected, not exotic, on non-module devices.
const COM_OBJECT_INSTANCE_REF_ATTRS_21: &[&str] = &[
    "RefId",
    "ChannelId",
    "Links",
    "IsActive",
    "DatapointType",
    "Text",
    "Description",
    // Measured (Task 3, ETS 6.3.0 reference project, schema 23): 119 flag
    // attributes on `ComObjectInstanceRef` elements — `ReadFlag` 38,
    // `WriteFlag` 18, `TransmitFlag` 27, `UpdateFlag` 28,
    // `CommunicationFlag` 8, spelled `"Enabled"`/`"Disabled"` exactly as
    // schema 11 spells them. The KV demo project carries none, which is why
    // the initial transcription concluded they never occur at schema ≥21.
    "ReadFlag",
    "WriteFlag",
    "TransmitFlag",
    "UpdateFlag",
    "CommunicationFlag",
];

/// Transcribed from `KV v2.5 - demo.knxproj` (`P-03DE/0.xml` and
/// `P-03DE/project.xml`), a genuinely independent schema-21 sample (RESEARCH
/// §2.5/§3.4) — 4 devices, 13 group addresses, manufacturer `M-00FA`.
///
/// This table is *not yet* exhaustive the way [`SCHEMA_11`]'s is: some
/// entries below (`Area`, `Line`, `Installation`, …) reflect only the
/// attributes this one sample happens to set (e.g. neither `Area` nor `Line`
/// carries a `Name` in this project — genuinely absent from the measured
/// XML, not an omission), and it has not yet been closed against a tolerant
/// parser's `UnknownConstruct` reports (no schema-≥21 parser exists yet —
/// that is a later task). Mirrors exactly how [`SCHEMA_11`] itself started
/// out; see this module's own doc comment.
pub const SCHEMA_21: KnownSchema = KnownSchema {
    version: 21,
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
                "GroupAddressStyle",
                "LastModified",
                "ProjectStart",
                "Comment",
                "LastUsedPuid",
                "Guid",
                "ProjectType",
                "CompletionStatus",
                "CodePage",
                "ArchivedVersion",
            ],
        },
        KnownElement {
            path: "/KNX/Project/ProjectInformation/ProjectTraces",
            attributes: &[],
        },
        KnownElement {
            path: "/KNX/Project/ProjectInformation/ProjectTraces/ProjectTrace",
            attributes: &["Date", "UserName", "Comment"],
        },
        KnownElement {
            path: "/KNX/Project/Installations",
            attributes: &[],
        },
        KnownElement {
            path: "/KNX/Project/Installations/Installation",
            attributes: &["Name", "BCUKey", "DefaultLine", "IPRoutingLatencyTolerance", "CompletionStatus"],
        },
        KnownElement {
            path: "/KNX/Project/Installations/Installation/Topology",
            attributes: &[],
        },
        KnownElement {
            path: "/KNX/Project/Installations/Installation/Topology/Area",
            attributes: &["Id", "Name", "Address", "Puid", "Description", "Comment", "CompletionStatus"],
        },
        KnownElement {
            path: "/KNX/Project/Installations/Installation/Topology/Area/Line",
            attributes: &["Id", "Name", "Address", "Puid", "Description", "Comment", "CompletionStatus"],
        },
        KnownElement {
            path: "/KNX/Project/Installations/Installation/Topology/Area/Line/Segment",
            attributes: &[
                "Id",
                "Number",
                "MediumTypeRefId",
                "DomainAddress",
                "DomainAddressIsChecked",
                "IPRoutingMulticastAddress",
                "MulticastTTL",
                "Puid",
            ],
        },
        KnownElement {
            path: "/KNX/Project/Installations/Installation/Topology/Area/Line/Segment/DeviceInstance",
            attributes: DEVICE_INSTANCE_ATTRS_21,
        },
        KnownElement {
            // Measured (Task 5): every device in `KV v2.5 - demo.knxproj`
            // carries this — an oversight in this table's initial Task 3
            // transcription, not a genuine schema-21 absence. Same shape as
            // schema 11's entry of the same name.
            path: "/KNX/Project/Installations/Installation/Topology/Area/Line/Segment/DeviceInstance/ParameterInstanceRefs",
            attributes: &[],
        },
        KnownElement {
            path: "/KNX/Project/Installations/Installation/Topology/Area/Line/Segment/DeviceInstance/ParameterInstanceRefs/ParameterInstanceRef",
            attributes: PARAMETER_INSTANCE_REF_ATTRS,
        },
        KnownElement {
            // Measured (Task 3): six of these in the ETS 6.3.0 reference
            // project, never transcribed, so every one was classified as an
            // unknown element and dropped on export. Wrapper form, no
            // attributes of its own.
            path: "/KNX/Project/Installations/Installation/Topology/Area/Line/Segment/DeviceInstance/BinaryData",
            attributes: &[],
        },
        KnownElement {
            // The leaf form, same element name as its wrapper.
            path: "/KNX/Project/Installations/Installation/Topology/Area/Line/Segment/DeviceInstance/BinaryData/BinaryData",
            attributes: BINARY_DATA_LEAF_ATTRS_21,
        },
        KnownElement {
            path: "/KNX/Project/Installations/Installation/Topology/Area/Line/Segment/DeviceInstance/ComObjectInstanceRefs",
            attributes: &[],
        },
        KnownElement {
            path: "/KNX/Project/Installations/Installation/Topology/Area/Line/Segment/DeviceInstance/ComObjectInstanceRefs/ComObjectInstanceRef",
            attributes: COM_OBJECT_INSTANCE_REF_ATTRS_21,
        },
        KnownElement {
            path: "/KNX/Project/Installations/Installation/Topology/Area/Line/Segment/DeviceInstance/ModuleInstances",
            attributes: &[],
        },
        KnownElement {
            path: "/KNX/Project/Installations/Installation/Topology/Area/Line/Segment/DeviceInstance/ModuleInstances/ModuleInstance",
            attributes: &["Id", "RefId", "RepeatIndex"],
        },
        KnownElement {
            path: "/KNX/Project/Installations/Installation/Topology/Area/Line/Segment/DeviceInstance/ModuleInstances/ModuleInstance/Arguments",
            attributes: &[],
        },
        KnownElement {
            path: "/KNX/Project/Installations/Installation/Topology/Area/Line/Segment/DeviceInstance/ModuleInstances/ModuleInstance/Arguments/Argument",
            attributes: &["RefId", "Value"],
        },
        KnownElement {
            path: "/KNX/Project/Installations/Installation/Topology/Area/Line/Segment/DeviceInstance/GroupObjectTree",
            attributes: &[],
        },
        KnownElement {
            path: "/KNX/Project/Installations/Installation/Topology/Area/Line/Segment/DeviceInstance/GroupObjectTree/Nodes",
            attributes: &[],
        },
        KnownElement {
            path: "/KNX/Project/Installations/Installation/Topology/Area/Line/Segment/DeviceInstance/GroupObjectTree/Nodes/Node",
            attributes: &["Type", "RefId", "GroupObjectInstances"],
        },
        KnownElement {
            // Measured (Task 5): per-device sequence-number/timestamp
            // bookkeeping, purpose not investigated (RESEARCH). Known but
            // its content is not interpreted — `SourceDevice::security_raw`
            // retains it verbatim, per device (see `installation_v21.rs`'s
            // own `"Security"` handling for why, and `SourceDevice`'s own
            // doc comment on the field).
            path: "/KNX/Project/Installations/Installation/Topology/Area/Line/Segment/DeviceInstance/Security",
            attributes: &["SequenceNumber", "SequenceNumberTimestamp"],
        },
        KnownElement {
            path: "/KNX/Project/Installations/Installation/Locations",
            attributes: &[],
        },
        KnownElement {
            path: "/KNX/Project/Installations/Installation/Locations/Space",
            attributes: &["Id", "Name", "Number", "Type", "DefaultLine", "Puid", "Usage", "Description", "Comment", "CompletionStatus"],
        },
        KnownElement {
            // Measured (Task 3): the ETS 6.3.0 reference project carries 35
            // of these. Missing from the initial transcription, so every
            // device-to-room assignment was classified as an unknown
            // element, retained verbatim and then dropped on export. Same
            // shape as schema 11's `Buildings/BuildingPart/DeviceInstanceRef`.
            path: "/KNX/Project/Installations/Installation/Locations/Space/DeviceInstanceRef",
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
            attributes: &["Id", "Name", "RangeStart", "RangeEnd", "Puid"],
        },
        KnownElement {
            path: "/KNX/Project/Installations/Installation/GroupAddresses/GroupRanges/GroupRange/GroupAddress",
            // `Central`/`Unfiltered` measured (Task 3) on the ETS 6.3.0
            // reference project, spelled `"true"`/`"false"`. Without them
            // here the parser never takes them, the model defaults both to
            // `false`, and export writes an actively wrong value back —
            // worse than leaving the attribute out.
            attributes: &[
                "Id",
                "Name",
                "Address",
                "DatapointType",
                "Description",
                "Central",
                "Unfiltered",
                "Puid",
            ],
        },
    ],
};

/// Schema 23's own confirmed deltas over schema 21 (RESEARCH §3.3): short
/// `RefId` (already schema-21-shaped, no change needed), a flat
/// `GroupObjectTree/@GroupObjectInstances` attribute directly on
/// `DeviceInstance` instead of nested `Nodes/Node`, and `"true"`/`"false"`
/// booleans (already accepted by `parse_bool` regardless of source schema).
/// Module-related entries (`ModuleInstances`, `ModuleDef`) are carried over
/// from schema 21 *by inference*, not independent schema-23 evidence — see
/// the spec's scope decision and KNOWN_LIMITATIONS §1.
pub const SCHEMA_23: KnownSchema = KnownSchema {
    version: 23,
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
                "GroupAddressStyle",
                "LastModified",
                "ProjectStart",
                "Comment",
                "LastUsedPuid",
                "Guid",
                "ProjectType",
                "CompletionStatus",
                "CodePage",
                "ArchivedVersion",
            ],
        },
        KnownElement {
            path: "/KNX/Project/ProjectInformation/ProjectTraces",
            attributes: &[],
        },
        KnownElement {
            path: "/KNX/Project/ProjectInformation/ProjectTraces/ProjectTrace",
            attributes: &["Date", "UserName", "Comment"],
        },
        KnownElement {
            path: "/KNX/Project/Installations",
            attributes: &[],
        },
        KnownElement {
            path: "/KNX/Project/Installations/Installation",
            attributes: &["Name", "BCUKey", "DefaultLine", "IPRoutingLatencyTolerance", "CompletionStatus"],
        },
        KnownElement {
            path: "/KNX/Project/Installations/Installation/Topology",
            attributes: &[],
        },
        KnownElement {
            path: "/KNX/Project/Installations/Installation/Topology/Area",
            attributes: &["Id", "Name", "Address", "Puid", "Description", "Comment", "CompletionStatus"],
        },
        KnownElement {
            path: "/KNX/Project/Installations/Installation/Topology/Area/Line",
            attributes: &["Id", "Name", "Address", "Puid", "Description", "Comment", "CompletionStatus"],
        },
        KnownElement {
            path: "/KNX/Project/Installations/Installation/Topology/Area/Line/Segment",
            attributes: &[
                "Id",
                "Number",
                "MediumTypeRefId",
                "DomainAddress",
                "DomainAddressIsChecked",
                "IPRoutingMulticastAddress",
                "MulticastTTL",
                "Puid",
            ],
        },
        KnownElement {
            path: "/KNX/Project/Installations/Installation/Topology/Area/Line/Segment/DeviceInstance",
            attributes: DEVICE_INSTANCE_ATTRS_21,
        },
        KnownElement {
            path: "/KNX/Project/Installations/Installation/Topology/Area/Line/Segment/DeviceInstance/ParameterInstanceRefs",
            attributes: &[],
        },
        KnownElement {
            path: "/KNX/Project/Installations/Installation/Topology/Area/Line/Segment/DeviceInstance/ParameterInstanceRefs/ParameterInstanceRef",
            attributes: PARAMETER_INSTANCE_REF_ATTRS,
        },
        KnownElement {
            // Measured (Task 3): six of these in the ETS 6.3.0 reference
            // project, never transcribed, so every one was classified as an
            // unknown element and dropped on export. Wrapper form, no
            // attributes of its own.
            path: "/KNX/Project/Installations/Installation/Topology/Area/Line/Segment/DeviceInstance/BinaryData",
            attributes: &[],
        },
        KnownElement {
            // The leaf form, same element name as its wrapper.
            path: "/KNX/Project/Installations/Installation/Topology/Area/Line/Segment/DeviceInstance/BinaryData/BinaryData",
            attributes: BINARY_DATA_LEAF_ATTRS_21,
        },
        KnownElement {
            path: "/KNX/Project/Installations/Installation/Topology/Area/Line/Segment/DeviceInstance/ComObjectInstanceRefs",
            attributes: &[],
        },
        KnownElement {
            path: "/KNX/Project/Installations/Installation/Topology/Area/Line/Segment/DeviceInstance/ComObjectInstanceRefs/ComObjectInstanceRef",
            attributes: COM_OBJECT_INSTANCE_REF_ATTRS_21,
        },
        KnownElement {
            path: "/KNX/Project/Installations/Installation/Topology/Area/Line/Segment/DeviceInstance/ModuleInstances",
            attributes: &[],
        },
        KnownElement {
            path: "/KNX/Project/Installations/Installation/Topology/Area/Line/Segment/DeviceInstance/ModuleInstances/ModuleInstance",
            attributes: &["Id", "RefId", "RepeatIndex"],
        },
        KnownElement {
            path: "/KNX/Project/Installations/Installation/Topology/Area/Line/Segment/DeviceInstance/ModuleInstances/ModuleInstance/Arguments",
            attributes: &[],
        },
        KnownElement {
            path: "/KNX/Project/Installations/Installation/Topology/Area/Line/Segment/DeviceInstance/ModuleInstances/ModuleInstance/Arguments/Argument",
            attributes: &["RefId", "Value"],
        },
        KnownElement {
            path: "/KNX/Project/Installations/Installation/Topology/Area/Line/Segment/DeviceInstance/GroupObjectTree",
            attributes: &["GroupObjectInstances"],
        },
        KnownElement {
            path: "/KNX/Project/Installations/Installation/Topology/Area/Line/Segment/DeviceInstance/GroupObjectTree/Nodes",
            attributes: &[],
        },
        KnownElement {
            path: "/KNX/Project/Installations/Installation/Topology/Area/Line/Segment/DeviceInstance/GroupObjectTree/Nodes/Node",
            attributes: &["Type", "RefId", "GroupObjectInstances"],
        },
        KnownElement {
            path: "/KNX/Project/Installations/Installation/Topology/Area/Line/Segment/DeviceInstance/Security",
            attributes: &["SequenceNumber", "SequenceNumberTimestamp"],
        },
        KnownElement {
            path: "/KNX/Project/Installations/Installation/Locations",
            attributes: &[],
        },
        KnownElement {
            path: "/KNX/Project/Installations/Installation/Locations/Space",
            attributes: &["Id", "Name", "Number", "Type", "DefaultLine", "Puid", "Usage", "Description", "Comment", "CompletionStatus"],
        },
        KnownElement {
            // Measured (Task 3): the ETS 6.3.0 reference project carries 35
            // of these. Missing from the initial transcription, so every
            // device-to-room assignment was classified as an unknown
            // element, retained verbatim and then dropped on export. Same
            // shape as schema 11's `Buildings/BuildingPart/DeviceInstanceRef`.
            path: "/KNX/Project/Installations/Installation/Locations/Space/DeviceInstanceRef",
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
            attributes: &["Id", "Name", "RangeStart", "RangeEnd", "Puid"],
        },
        KnownElement {
            path: "/KNX/Project/Installations/Installation/GroupAddresses/GroupRanges/GroupRange/GroupAddress",
            // `Central`/`Unfiltered` measured (Task 3) on the ETS 6.3.0
            // reference project, spelled `"true"`/`"false"`. Without them
            // here the parser never takes them, the model defaults both to
            // `false`, and export writes an actively wrong value back —
            // worse than leaving the attribute out.
            attributes: &[
                "Id",
                "Name",
                "Address",
                "DatapointType",
                "Description",
                "Central",
                "Unfiltered",
                "Puid",
            ],
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
        assert!(!line.attributes.contains(&"Puid")); // schema ≥21 only (RESEARCH §3.4 correction)
    }

    #[test]
    fn an_unknown_schema_version_has_no_table() {
        assert!(known_schema(99).is_none());
    }

    #[test]
    fn schema_23_agrees_with_schema_21_except_the_group_object_tree_shape() {
        let paths = |s: &KnownSchema| -> Vec<&str> {
            s.elements
                .iter()
                .map(|e| e.path)
                .filter(|p| !p.contains("GroupObjectTree"))
                .collect()
        };
        assert_eq!(paths(&SCHEMA_21), paths(&SCHEMA_23));
        let ga_tree = |s: &KnownSchema| {
            s.elements
                .iter()
                .find(|e| e.path.ends_with("GroupObjectTree"))
                .unwrap()
        };
        assert!(!ga_tree(&SCHEMA_21)
            .attributes
            .contains(&"GroupObjectInstances"));
        assert!(ga_tree(&SCHEMA_23)
            .attributes
            .contains(&"GroupObjectInstances"));
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
        // (no attributes) and a leaf (Id, Name), so 27 distinct paths under
        // Line, plus the same 10-element DeviceInstance subtree repeated
        // under UnassignedDevices (measured, Task 6), so 37 distinct paths.
        assert_eq!(known_schema(11).unwrap().elements.len(), 37);
    }
}
