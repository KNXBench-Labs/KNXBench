//! The source layer: raw XML content, still shaped like the XML, before any
//! conversion to `knx-core` types.
//!
//! Every attribute is a `String` — exactly the bytes that were in the file.
//! This layer performs no conversion at all, so the parser (Task 6) can be
//! tested against raw XML with no domain model in sight, and so `map` (Task
//! 10) is the single place a conversion can fail.
//!
//! An ETS project part is split across two files sharing the same root
//! element and namespace: `Project.xml` (ETS4) / `project.xml` (ETS5/6)
//! carries only `ProjectInformation`; `0.xml` carries everything else
//! (installations, topology, devices, group addresses). `SourceDocument` is
//! the result of parsing both against the same known-element table.

/// Everything read from one project part, before mapping.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SourceDocument {
    pub schema_version: u32,
    pub created_by: Option<String>,
    pub tool_version: Option<String>,
    pub project_id: String,
    pub info: SourceProjectInfo,
    pub installations: Vec<SourceInstallation>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SourceProjectInfo {
    pub name: Option<String>,
    pub project_number: Option<String>,
    pub group_address_style: Option<String>,
    pub completion_status: Option<String>,
    pub last_modified: Option<String>,
    pub project_start: Option<String>,
    /// The whole `<ProjectTraces>...</ProjectTraces>` subtree (schema ≥21,
    /// an audit log — RESEARCH §3.4: "purpose not investigated"), retained
    /// verbatim for export.
    pub project_traces_raw: Option<RetainedElement>,
    pub other: Vec<RetainedAttribute>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SourceInstallation {
    pub installation_id: Option<String>,
    pub name: Option<String>,
    pub default_line: Option<String>,
    pub ip_routing_multicast_address: Option<String>,
    pub completion_status: Option<String>,
    pub areas: Vec<SourceArea>,
    pub unassigned_devices: Vec<SourceDevice>,
    pub buildings: Vec<SourceBuildingPart>,
    pub group_ranges: Vec<SourceGroupRange>,
    pub other: Vec<RetainedAttribute>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SourceArea {
    pub id: String,
    pub name: Option<String>,
    pub address: Option<String>,
    pub completion_status: Option<String>,
    pub lines: Vec<SourceLine>,
    pub other: Vec<RetainedAttribute>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SourceLine {
    pub id: String,
    pub name: Option<String>,
    pub address: Option<String>,
    pub medium_type_ref_id: Option<String>,
    pub domain_address: Option<String>,
    pub domain_address_is_checked: Option<String>,
    pub ip_routing_multicast_address: Option<String>,
    pub multicast_ttl: Option<String>,
    pub completion_status: Option<String>,
    pub devices: Vec<SourceDevice>,
    /// `BusAccess`, retained verbatim — ETS interface-driver configuration.
    pub bus_access: Option<RetainedElement>,
    pub other: Vec<RetainedAttribute>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SourceDevice {
    pub id: String,
    pub name: Option<String>,
    pub description: Option<String>,
    pub address: Option<String>,
    pub product_ref_id: Option<String>,
    pub hardware2program_ref_id: Option<String>,
    pub last_modified: Option<String>,
    pub last_download: Option<String>,
    pub completion_status: Option<String>,
    pub individual_address_loaded: Option<String>,
    pub application_program_loaded: Option<String>,
    pub parameters_loaded: Option<String>,
    pub communication_part_loaded: Option<String>,
    pub medium_config_loaded: Option<String>,
    pub visibility_calculated: Option<String>,
    pub broken: Option<String>,
    pub parameters: Vec<SourceParameterInstance>,
    pub com_objects: Vec<SourceComObjectInstance>,
    pub binary_data: Vec<SourceBinaryDataRef>,
    /// Schema ≥21 only. Structured for `map.rs`'s `ModuleInstance`
    /// construction (ADR-0013).
    pub module_instances: Vec<SourceModuleInstance>,
    /// The whole `<ModuleInstances>...</ModuleInstances>` subtree, verbatim,
    /// for export — see the plan's Global Constraints on why this is
    /// retained raw rather than reconstructed.
    pub module_instances_raw: Option<RetainedElement>,
    /// Schema ≥21 only. The authoritative communication-object id list
    /// (ADR-0014), unioned across nested `Nodes/Node` (schema 21) or read
    /// flat (schema 23) — the schema-version-specific walk lives in the
    /// parser, not here.
    pub group_object_tree: Vec<String>,
    /// The whole `<GroupObjectTree>...</GroupObjectTree>` subtree, verbatim,
    /// for export.
    pub group_object_tree_raw: Option<RetainedElement>,
    /// Schema ≥21 only. The whole `<Security>...</Security>` subtree,
    /// verbatim — per-device sequence-number/timestamp bookkeeping, purpose
    /// not investigated (RESEARCH). Known but deliberately not modeled
    /// beyond raw retention, exactly like `module_instances_raw`/
    /// `group_object_tree_raw` above; kept per-device (not in a document-
    /// wide bucket) so a future export task can recover which `Security`
    /// blob belongs to which device without relying on document order.
    pub security_raw: Option<RetainedElement>,
    pub other: Vec<RetainedAttribute>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SourceParameterInstance {
    pub ref_id: String,
    pub value: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SourceArgument {
    pub ref_id: String,
    pub value: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SourceModuleInstance {
    pub id: String,
    pub ref_id: String,
    pub repeat_index: Option<String>,
    pub arguments: Vec<SourceArgument>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SourceComObjectInstance {
    pub ref_id: String,
    pub is_active: Option<String>,
    pub datapoint_type: Option<String>,
    pub text: Option<String>,
    pub description: Option<String>,
    pub read_flag: Option<String>,
    pub write_flag: Option<String>,
    pub transmit_flag: Option<String>,
    pub update_flag: Option<String>,
    pub communication_flag: Option<String>,
    /// `Connectors/Send/@GroupAddressRefId`.
    pub sends: Vec<String>,
    /// `Connectors/Receive/@GroupAddressRefId`.
    pub receives: Vec<String>,
    /// Schema ≥21's flat `Links` attribute (space-separated short GA ids),
    /// parsed into a list. Empty for schema 11, which uses `sends`/
    /// `receives` instead.
    pub links: Vec<String>,
    /// Schema ≥21's `ChannelId` — known-but-not-modeled at instance level
    /// today (no channel-grouped UI yet, spec's explicit scope decision);
    /// carried here only so `map.rs` can fold it into `other` for export.
    pub channel_id: Option<String>,
    pub other: Vec<RetainedAttribute>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SourceBinaryDataRef {
    pub id: String,
    pub name: Option<String>,
    /// Schema ≥21 adds `DoNotCopy` here, and a future schema may add more:
    /// anything this leaf carries beyond `Id`/`Name` is kept rather than
    /// dropped, and goes back onto the same leaf on export.
    pub other: Vec<RetainedAttribute>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SourceBuildingPart {
    pub id: String,
    pub name: Option<String>,
    pub number: Option<String>,
    pub kind: Option<String>,
    pub default_line: Option<String>,
    pub completion_status: Option<String>,
    pub children: Vec<SourceBuildingPart>,
    pub device_refs: Vec<String>,
    pub other: Vec<RetainedAttribute>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SourceGroupRange {
    pub id: String,
    pub name: Option<String>,
    pub range_start: Option<String>,
    pub range_end: Option<String>,
    pub children: Vec<SourceGroupRange>,
    pub addresses: Vec<SourceGroupAddress>,
    pub other: Vec<RetainedAttribute>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SourceGroupAddress {
    pub id: String,
    pub name: Option<String>,
    pub address: Option<String>,
    pub central: Option<String>,
    pub unfiltered: Option<String>,
    /// `@DatapointType`, schema ≥21 only (ADR-0078).
    pub datapoint_type: Option<String>,
    pub other: Vec<RetainedAttribute>,
}

/// An attribute the known-element table for this schema version does not
/// list. Kept with its value so that neither the value nor the fact that it
/// was unknown is lost.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RetainedAttribute {
    pub xpath: String,
    pub name: String,
    pub value: String,
}

/// An element the known-element table does not list, or one that is known
/// but deliberately not modelled, kept as the raw bytes it occupied in the
/// source.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RetainedElement {
    pub xpath: String,
    pub name: String,
    pub raw: Vec<u8>,
}
