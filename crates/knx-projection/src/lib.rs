//! Display-shaped projections of [`knx_core::Project`] for the desktop UI
//! (ADR-0009). The UI never receives `Project` itself — only these types,
//! generated into TypeScript by `ts-rs` so the two sides cannot disagree
//! without the build failing. Depends on `knx-core` only: no IO, no format,
//! no storage (`xtask check-layering` enforces this, same rule as
//! `knx-core` itself).

use std::collections::HashMap;

use knx_core::{
    BuildingPart, BuildingPartId, BuildingPartType, Devices, DptRef, GroupAddressDpt,
    GroupAddressEntry, GroupAddressId, GroupAddressStyle, GroupRange, Project, Text, Topology,
};
use serde::Serialize;
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct ProjectTree {
    pub schema_version: u32,
    /// Count of genuine `Severity::Error` items from the `ImportReport` —
    /// data actually lost or misread, as `knx_etsproj::report::Severity`
    /// distinguishes it (see `ImportReport::has_losses()`) — that a caller
    /// with access to the `ImportReport` should fill in — always `0`
    /// straight out of [`build_project_tree`], since this crate never sees
    /// that type (CLAUDE.md: never silently discard information; full
    /// drill-down is a later cycle, this is the count that says something
    /// was genuinely lost, never to be shown to the user as a mere
    /// "warning").
    pub errors: usize,
    /// Count of everything else worth a look but not a real loss:
    /// `Severity::Warning` items, unknown constructs, DPT conflicts, and
    /// documented capability gaps — that a caller with access to the
    /// `ImportReport` should fill in — always `0` straight out of
    /// [`build_project_tree`], since this crate never sees that type
    /// (CLAUDE.md: never silently discard information; full drill-down is
    /// a later cycle, this is the count that says something is worth
    /// looking at).
    pub warnings: usize,
    /// Always `false` straight out of [`build_project_tree`] — this crate
    /// never sees a `CommandStack`. The desktop shell overlays the real
    /// value from its own `CommandStack` after every command/undo/redo.
    pub can_undo: bool,
    /// See `can_undo`.
    pub can_redo: bool,
    /// Always `false` straight out of [`build_project_tree`] because this
    /// pure projection has no clean baseline. The application layer overlays
    /// whether the live project differs from its last successful open,
    /// import, creation, or save snapshot.
    pub is_modified: bool,
    /// Opaque identity of the running server process that owns
    /// `snapshot_revision`. Pure/offline projections omit it together with
    /// the revision; it is transient application metadata, not project data.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub server_incarnation: Option<String>,
    /// Application-owned response ordering. Pure/offline projections omit it;
    /// the server stamps every UI-facing snapshot while holding its project
    /// lock. This is transient metadata and is never persisted in a KNX or
    /// native project format.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional, type = "number")]
    pub snapshot_revision: Option<u64>,
    /// Present only when this response also republished the complete group-
    /// address context into the named active bus session. The frontend may
    /// rebase that exact session's fingerprint after accepting the snapshot.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional, type = "number")]
    pub group_address_context_session_id: Option<u64>,
    /// The project-wide rendering choice every `GroupAddressNode`,
    /// `GroupRangeNode` and `GroupLinkNode` address string in this tree was
    /// already formatted with — carried through so the inspector can show
    /// it on the project node without a second round trip
    /// (KNOWN_LIMITATIONS.md §84). `GroupAddressStyle` as a plain string
    /// (`"Free"`, `"TwoLevel"`, `"ThreeLevel"`) — same choice as
    /// `BuildingPartType` below: the enum itself stays in `knx-core`, a
    /// typed TS union is not worth a mirror type for one read-only field.
    pub group_address_style: String,
    /// RFC3339 timestamp of the last successful save of the project
    /// currently open, or `None` if it has not been saved since it was
    /// created, opened or imported. Always `None` straight out
    /// of [`build_project_tree`] — this crate never sees the application's
    /// save bookkeeping. The desktop shell overlays the real value from its
    /// own `last_saved_at` state, in lockstep with `is_modified` (both are
    /// stamped only by a successful save, never by a failed one).
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub last_saved_at: Option<String>,
    pub installations: Vec<InstallationNode>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct InstallationNode {
    pub id: u8,
    pub name: String,
    pub topology: Vec<AreaNode>,
    pub buildings: Vec<BuildingNode>,
    pub unassigned: Vec<DeviceNode>,
    pub group_addresses: Vec<GroupAddressNode>,
    pub group_ranges: Vec<GroupRangeNode>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct AreaNode {
    pub id: u32,
    pub name: String,
    pub address: u8,
    pub lines: Vec<LineNode>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct LineNode {
    pub id: u32,
    pub name: String,
    pub address: u8,
    pub devices: Vec<DeviceNode>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct BuildingNode {
    pub id: u32,
    pub name: String,
    /// `BuildingPartType` as a plain string (e.g. `"Room"`, `"Floor"`) — the
    /// enum itself stays in `knx-core`; a typed TS union is not worth the
    /// extra `ts-rs` surface for a single label this cycle.
    pub kind: String,
    pub children: Vec<BuildingNode>,
    pub devices: Vec<DeviceNode>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct DeviceNode {
    pub id: u32,
    pub name: String,
    /// Formatted individual address (e.g. `"1.1.1"`) — `None` if the device
    /// has no address assigned, which is valid project state.
    pub address: Option<String>,
    pub description: Option<String>,
    /// Count of this device's communication objects, regardless of
    /// `is_active` — the dashboard's project-wide total sums this field
    /// across every `DeviceNode` it visits (Session 5, cycle 8).
    pub com_object_count: usize,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct GroupAddressNode {
    pub id: u32,
    pub name: String,
    /// Formatted per the project's own `GroupAddressStyle`
    /// (`GroupAddress::format`), e.g. `"4/2/100"`.
    pub address: String,
    /// The id of the `GroupRange` this address was imported under, `None`
    /// if it sits under none. Projected as an id, not a resolved path: the
    /// containing [`GroupRangeNode`] is already in the same installation's
    /// `group_ranges`, carrying its own name and `parent`, so resolving
    /// the path here would duplicate data the caller already holds.
    pub range: Option<u32>,
    /// Every datapoint type the communication objects linked to this
    /// address state, classified by `knx_core::group_address_dpt_from` —
    /// the same rule `resolve_group_address_dpt` applies, over the same
    /// set of communication objects.
    ///
    /// Empty means `GroupAddressDpt::None` (nothing linked states one, the
    /// ordinary case for 38% of the reference project's addresses); one
    /// entry means every linked object that states a DPT states that one;
    /// two or more is `GroupAddressDpt::Conflict` — the disagreement
    /// reported, never settled by picking a winner. Entries are `DptRef`'s
    /// `Display` text (`"DPST-1-1"`, `"DPT-1"`), never the dotted
    /// `"1.001"` form, which nothing in this repository produces.
    pub dpts: Vec<String>,
    /// Every communication object linked to this address, in
    /// `ComObjectInstanceId` order — the reverse of `ComObjectNode::links`.
    pub links: Vec<GroupAddressLinkNode>,
}

/// One communication object's link to a group address, seen from the
/// address's side: the mirror image of [`GroupLinkNode`], which sees the
/// same link from the communication object's side. An object linked to the
/// same address in both directions produces two of these, one per
/// `direction`, exactly as it holds two `GroupLink`s.
///
/// `device_name`/`device_address` are `None` only if `device_id` names no
/// device in the project — the same defensive stance [`GroupLinkNode`]
/// takes towards a dangling `ga_id`. The row is still projected rather
/// than dropped: a link the project states is not information to lose on
/// the way to the screen.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct GroupAddressLinkNode {
    pub device_id: u32,
    pub device_name: Option<String>,
    /// Formatted individual address — `None` if the device has none
    /// assigned (valid project state), same as `DeviceNode::address`.
    pub device_address: Option<String>,
    pub com_object_id: u32,
    /// `ComObjectInstance::number` — `_O-<n>` from the source `RefId`,
    /// same field `ComObjectNode::number` projects.
    pub com_object_number: u16,
    /// Resolved through the project's string table, same as
    /// `ComObjectNode::name`.
    pub com_object_name: Option<String>,
    /// `"Send"` or `"Receive"` (`Direction`'s `Debug` form, same
    /// convention as `GroupLinkNode::direction`).
    pub direction: String,
}

/// A flat (not nested) view of one `GroupRange` — `parent` names the
/// containing main range's id for a middle range, `None` for a main
/// range. Deliberately does not nest `GroupAddressNode`s inside their
/// range: `InstallationNode.group_addresses` stays a flat list, matching
/// its existing shape, until a future cycle redesigns the group-address
/// tree branch around the real main/middle/address hierarchy.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct GroupRangeNode {
    pub id: u32,
    pub name: String,
    /// Formatted per the project's own `GroupAddressStyle`, same as
    /// `GroupAddressNode::address`.
    pub start: String,
    pub end: String,
    pub parent: Option<u32>,
}

/// Builds the full display tree for every installation in `project`. Pure
/// and total: never panics on a project that imported successfully, even
/// one with dangling `BuildingPart` device references (knx-etsproj's
/// `validate.rs` does not check those — see the doc comment there).
pub fn build_project_tree(project: &Project) -> ProjectTree {
    let links = build_group_address_link_index(project);
    ProjectTree {
        schema_version: project.schema_version,
        errors: 0,
        warnings: 0,
        can_undo: false,
        can_redo: false,
        is_modified: false,
        server_incarnation: None,
        snapshot_revision: None,
        group_address_context_session_id: None,
        group_address_style: group_address_style_str(project.info.group_address_style).to_string(),
        last_saved_at: None,
        installations: project
            .installations
            .iter()
            .map(|inst| {
                build_installation(
                    inst,
                    &project.devices,
                    project.info.group_address_style,
                    &links,
                )
            })
            .collect(),
    }
}

/// What every group address in the project is linked to, and which DPTs
/// those links state, built in **one** pass over every communication
/// object.
///
/// One pass, not one `resolve_group_address_dpt` call per address: that
/// function rescans every communication object in the project each time it
/// is called, and [`build_project_tree`] runs after every command, undo and
/// redo. A project with a few thousand addresses and a few tens of
/// thousands of communication objects would pay that product on every
/// keystroke-sized edit. The classification of the gathered DPTs is still
/// the domain's own — `knx_core::group_address_dpt_from`, the same function
/// `resolve_group_address_dpt` ends in — so the answer here and the answer
/// there cannot drift apart.
///
/// Iterates `Devices::com_objects`, which is `ComObjectInstanceId`-ordered
/// and includes objects no device's own list currently names (see its doc
/// comment): the same set `resolve_group_address_dpt` resolves over, so a
/// projected address's DPT and its projected links always describe the same
/// objects. That ordering is what makes each address's `links` vector
/// deterministic across runs, with no sort of its own.
fn build_group_address_link_index(project: &Project) -> GroupAddressLinkIndex {
    let mut links: HashMap<GroupAddressId, Vec<GroupAddressLinkNode>> = HashMap::new();
    let mut dpts: HashMap<GroupAddressId, Vec<DptRef>> = HashMap::new();
    for com in project.devices.com_objects() {
        if com.links.is_empty() {
            continue;
        }
        let device = project.devices.get(com.device);
        let com_object_name = resolved_text(project, &com.text);
        let dpt = com.dpt.value().map(|resolved| resolved.value);
        for link in &com.links {
            links
                .entry(link.ga)
                .or_default()
                .push(GroupAddressLinkNode {
                    device_id: com.device.0,
                    device_name: device.map(|d| d.name.clone()),
                    device_address: device.and_then(|d| d.address).map(|a| a.to_string()),
                    com_object_id: com.id.0,
                    com_object_number: com.number,
                    com_object_name: com_object_name.clone(),
                    direction: format!("{:?}", link.direction),
                });
            // Pushed per link, not per object: harmless, because
            // `group_address_dpt_from` deduplicates, so an object linked in
            // both directions still contributes its DPT once.
            if let Some(dpt) = dpt {
                dpts.entry(link.ga).or_default().push(dpt);
            }
        }
    }
    GroupAddressLinkIndex {
        links,
        dpts: dpts
            .into_iter()
            .map(|(ga, stated)| {
                let formatted = match knx_core::group_address_dpt_from(stated) {
                    GroupAddressDpt::None => Vec::new(),
                    GroupAddressDpt::Single(dpt) => vec![dpt.to_string()],
                    GroupAddressDpt::Conflict(dpts) => {
                        dpts.iter().map(|dpt| dpt.to_string()).collect()
                    }
                };
                (ga, formatted)
            })
            .collect(),
    }
}

/// The output of [`build_group_address_link_index`]. An address absent from
/// either map has no links, and therefore no stated DPT — the maps hold no
/// empty entries.
struct GroupAddressLinkIndex {
    links: HashMap<GroupAddressId, Vec<GroupAddressLinkNode>>,
    dpts: HashMap<GroupAddressId, Vec<String>>,
}

/// Resolves one `Override<Text>` through the project's string table in its
/// default language — the single rule `ComObjectNode`'s `name` and
/// `description` and `GroupAddressLinkNode`'s `com_object_name` all follow.
fn resolved_text(project: &Project, value: &knx_core::Override<Text>) -> Option<String> {
    value.value().and_then(|resolved| {
        project
            .strings
            .text(&resolved.value, project.strings.default_language())
            .map(|s| s.to_string())
    })
}

fn build_installation(
    inst: &knx_core::Installation,
    devices: &Devices,
    ga_style: GroupAddressStyle,
    links: &GroupAddressLinkIndex,
) -> InstallationNode {
    InstallationNode {
        id: inst.id.0,
        name: inst.name.clone(),
        topology: build_topology(&inst.topology, devices),
        buildings: build_building_forest(&inst.buildings, devices),
        unassigned: inst
            .topology
            .unassigned
            .iter()
            .filter_map(|id| devices.get(*id))
            .map(build_device_node)
            .collect(),
        group_addresses: inst
            .group_addresses
            .iter()
            .map(|entry| build_group_address_node(entry, ga_style, links))
            .collect(),
        group_ranges: inst
            .group_ranges
            .iter()
            .map(|range| build_group_range_node(range, ga_style))
            .collect(),
    }
}

fn build_group_address_node(
    entry: &GroupAddressEntry,
    style: GroupAddressStyle,
    index: &GroupAddressLinkIndex,
) -> GroupAddressNode {
    GroupAddressNode {
        id: entry.id.0,
        name: entry.name.clone(),
        address: entry.address.format(style),
        range: entry.range.map(|r| r.0),
        dpts: index.dpts.get(&entry.id).cloned().unwrap_or_default(),
        links: index.links.get(&entry.id).cloned().unwrap_or_default(),
    }
}

fn build_group_range_node(range: &GroupRange, style: GroupAddressStyle) -> GroupRangeNode {
    GroupRangeNode {
        id: range.id.0,
        name: range.name.clone(),
        start: range.start.format(style),
        end: range.end.format(style),
        parent: range.parent.map(|p| p.0),
    }
}

fn build_topology(topology: &Topology, devices: &Devices) -> Vec<AreaNode> {
    topology
        .areas
        .iter()
        .map(|area| AreaNode {
            id: area.id.0,
            name: area.name.clone(),
            address: area.address,
            lines: area
                .lines
                .iter()
                .filter_map(|line_id| topology.line(*line_id))
                .map(|line| LineNode {
                    id: line.id.0,
                    name: line.name.clone(),
                    address: line.address,
                    devices: line
                        .devices
                        .iter()
                        .filter_map(|id| devices.get(*id))
                        .map(build_device_node)
                        .collect(),
                })
                .collect(),
        })
        .collect()
}

fn build_device_node(device: &knx_core::DeviceInstance) -> DeviceNode {
    DeviceNode {
        id: device.id.0,
        name: device.name.clone(),
        address: device.address.map(|a| a.to_string()),
        description: device.description.clone(),
        com_object_count: device.com_objects.len(),
    }
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct DeviceDetail {
    pub id: u32,
    pub name: String,
    pub description: Option<String>,
    /// Formatted individual address (e.g. `"1.1.1"`), `None` if unassigned.
    pub address: Option<String>,
    pub com_objects: Vec<ComObjectNode>,
    /// The device's product/hardware identity (T16): what the project
    /// itself states, plus whatever `apps/knx-server` can add from a
    /// product database. See [`DeviceProductNode`].
    pub product: DeviceProductNode,
}

/// A device's product identity, in two halves: `product_ref`/`program_ref`
/// are what `knx_core::DeviceInstance` states verbatim (this crate can
/// always fill those in); `catalog`/`resolution` are what a product
/// database says about them, which this crate has no way to check — it
/// depends on nothing but `knx-core` (`xtask check-layering`), and knowing
/// whether a database is even loaded is `apps/knx-server`'s business, not
/// this one's. [`build_device_detail`] fills the first half and leaves an
/// honest placeholder in the second; `apps/knx-server::domain::device_detail`
/// always overwrites that placeholder before a response leaves the process.
/// See [`ProductResolution`] for exactly what the placeholder is and why it
/// never leaks.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct DeviceProductNode {
    /// `DeviceInstance::product_ref` (ETS `ProductRefId`), verbatim. `None`
    /// when the project states an empty string — an empty ref names no
    /// product, same convention `knx_etsproj::compare` already uses for
    /// this field.
    pub product_ref: Option<String>,
    /// `DeviceInstance::program_ref` (ETS `Hardware2ProgramRefId`),
    /// verbatim, under the same empty-string-means-`None` rule as
    /// `product_ref`.
    pub program_ref: Option<String>,
    /// Filled by `apps/knx-server` from the product database. `None`
    /// unless `resolution` is `Resolved`.
    pub catalog: Option<DeviceProductCatalog>,
    /// Why `catalog` is what it is — always present, never a bare "unknown".
    ///
    /// [`build_device_detail`] (pure, no database access) can only tell
    /// `NoReference` (both refs empty) from "a ref is stated" — it cannot
    /// tell `NoDatabase` from `NotInDatabase`, since that distinction needs
    /// to know whether a product database is even loaded, which is
    /// `apps/knx-server`'s state, not this crate's. So when a ref is
    /// present it emits `NoDatabase` as a placeholder — a true statement at
    /// the moment this crate produces it ("as far as I can tell, no
    /// database was consulted") — and `apps/knx-server::domain::device_detail`
    /// **always** overwrites it with `Resolved`, `NoDatabase` (confirmed)
    /// or `NotInDatabase` before the response reaches the UI. A
    /// server-side test
    /// (`device_product_resolution_always_overwrites_the_projections_placeholder`)
    /// pins that replacement. No fifth "not yet resolved" variant exists;
    /// the UI never sees this field before the server has spoken.
    pub resolution: ProductResolution,
}

/// What a product database knows about a device's product, hardware and
/// application program, mirroring `knx_productdb::query::DeviceProductRow`
/// field-for-field (this crate cannot depend on `knx-productdb` —
/// `xtask check-layering` — so the shape is duplicated rather than shared).
/// Every field beyond `manufacturer_id` is `Option` because a
/// partially-installed manufacturer catalogue (a product installed without
/// its application program, for instance) is real, valid database state,
/// not an error — CLAUDE.md: never silently discard information.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct DeviceProductCatalog {
    pub manufacturer_id: String,
    /// `manufacturer.name` from the KNX master data (`knx_master.xml`).
    pub manufacturer_name: Option<String>,
    /// `product.text` — the product's display name.
    pub product_text: Option<String>,
    pub order_number: Option<String>,
    /// `hardware.name`. Never translated: unlike `product.text`, no
    /// manufacturer package this project has ingested has ever placed a
    /// `Hardware` element's own id inside a `Languages` block (see
    /// `knx-productdb`'s `device_product` doc comment for the measurement).
    pub hardware_name: Option<String>,
    pub hardware_version: Option<String>,
    pub hardware_serial_number: Option<String>,
    /// `catalog_item.name` — `None` when this product/hardware pair is not
    /// listed in any catalog section, which is valid: not every installed
    /// product needs a catalog entry.
    pub catalog_item_name: Option<String>,
    pub catalog_item_number: Option<String>,
    /// `application_program.id`, so the UI can cross-reference devices
    /// sharing the same program without a second round trip.
    pub application_program_id: Option<String>,
    pub application_name: Option<String>,
    pub application_number: Option<String>,
    pub application_version: Option<String>,
    pub mask_version: Option<String>,
}

/// Why [`DeviceProductNode::catalog`] is what it is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub enum ProductResolution {
    /// Both refs resolved against an installed product database.
    Resolved,
    /// The project states no product ref at all — both `product_ref` and
    /// `program_ref` are empty strings in the source, a device created
    /// without one or an import that carried none.
    NoReference,
    /// No product database is loaded to resolve the stated refs against.
    /// Also [`build_device_detail`]'s placeholder for "the server has not
    /// looked yet" when a ref is present — see
    /// [`DeviceProductNode::resolution`]'s doc comment for why that overload
    /// is safe and never reaches the UI unconfirmed.
    NoDatabase,
    /// The refs exist and a database is loaded, but it does not contain
    /// them — the manufacturer's catalogue is simply not installed here.
    NotInDatabase,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct ComObjectNode {
    pub id: u32,
    /// From `_O-<n>` in the source `RefId`.
    pub number: u16,
    pub name: Option<String>,
    /// Formatted datapoint type reference (e.g. `"DPST-1-1"`, `"DPT-1"`),
    /// `None` if never stated at any layer.
    pub dpt: Option<String>,
    /// The layer `dpt` resolved from (`"Program"`, `"ProgramRef"`,
    /// `"Instance"`, `"Inferred"`, `"UserEdit"`), `None` alongside `dpt:
    /// None`.
    pub dpt_layer: Option<String>,
    /// Resolved through the project's string table, same as `name` — `None`
    /// if never stated at any layer.
    pub description: Option<String>,
    /// The layer `description` resolved from, `None` alongside
    /// `description: None`.
    pub description_layer: Option<String>,
    pub is_active: bool,
    /// Editable via `Command::SetComObjectFlag` (one flag at a time) —
    /// see `ComFlagKind` in `knx-core`.
    pub read: bool,
    pub write: bool,
    pub transmit: bool,
    pub update: bool,
    pub communication: bool,
    /// Read-on-Init, the sixth flag (§117). Flattened to `false` when no
    /// layer stated it, exactly like its five neighbours — the projection
    /// is the read model, not the place where "absent" and "false" are told
    /// apart.
    pub read_on_init: bool,
    /// The `GroupLink`s already on this communication object —
    /// `knx_core::Command::LinkComObject`/`UnlinkComObject` (2026-09-06)
    /// had no projection field to read or drive from until this cycle.
    pub links: Vec<GroupLinkNode>,
    /// Whether the device's current parameter values activate this object
    /// in the application program's `Dynamic` tree (ISSUE-08). Independent
    /// of `is_active`, which is the project file's own stored claim: a
    /// device KNXBench created stores `true` for every object. Only the
    /// server can evaluate (it needs the product database), so
    /// [`build_device_detail`] alone always says `NotEvaluated`.
    ///
    /// On the wire (serde) since ISSUE-08's data half, but not yet in the
    /// generated TypeScript bindings: `apps/knx-web` is under the UI
    /// session's web lock. Its ISSUE-08 UI half (goal-ui.md U12) drops this
    /// `skip`, and the one on `channel`, adds `export` to the two types
    /// below, and regenerates the bindings.
    #[ts(skip)]
    pub activation: ComObjectActivation,
    /// The `Channel`/`ChannelIndependentBlock` of the `Dynamic` tree this
    /// object was activated under (ISSUE-08). `None` when `activation` is
    /// not `Active`, or the object sits outside every channel element.
    /// Not yet in the TypeScript bindings; see `activation`.
    #[ts(skip)]
    pub channel: Option<ComObjectChannel>,
    /// The application program's datapoint type for an object whose own
    /// `DatapointType` is stated empty (ADR-0027's program default). `None`
    /// whenever `dpt` is the value to show, and when the program has none.
    /// Never exported: an empty slot stays empty in the file (ISSUE-08).
    #[ts(skip)]
    pub program_dpt: Option<String>,
    /// The display text of the datapoint type the object shows: `dpt`, or
    /// `program_dpt` when `dpt` is `None`. In the requested language when
    /// the master data translates it. The canonical id stays in `dpt`/
    /// `program_dpt`. Server-only: [`build_device_detail`] leaves it `None`.
    #[ts(skip)]
    pub dpt_text: Option<String>,
    /// The product's `FunctionText` (`ComObjectRef` over `ComObject`), in
    /// the requested language when translated, module arguments of the
    /// object's own module instance substituted. The main-function label
    /// ETS shows next to the name (ISSUE-08). Server-only, like `dpt_text`.
    #[ts(skip)]
    pub function_text: Option<String>,
}

/// [`ComObjectNode::activation`]: the evaluated state, or why there is none.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
pub enum ComObjectActivation {
    /// The evaluated tree activates this object.
    Active,
    /// The evaluated tree does not activate this object, and the
    /// evaluation met nothing that makes that answer uncertain.
    Inactive,
    /// Not activated, but the evaluation reported something that could
    /// have hidden it (an unknown value, a missing module definition, a
    /// budget limit, ...), or the object's own module instance cannot be
    /// told apart. Shown, not hidden: the answer is not known.
    Undetermined,
    /// No evaluation ran: no product reference, no product database, the
    /// program is not installed, or the program has no `Dynamic` tree.
    NotEvaluated,
}

/// [`ComObjectNode::channel`]: the channel element that owns an object.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
pub struct ComObjectChannel {
    /// Stable within one device detail: two objects share a channel exactly
    /// when their keys are equal. Opaque; do not parse.
    pub key: String,
    /// `Channel` or `ChannelIndependentBlock`.
    pub kind: String,
    /// The element's `@Text` in the requested language, module arguments
    /// substituted. `None` when the element has no text (every
    /// `ChannelIndependentBlock` in the corpus has none).
    pub text: Option<String>,
    /// Sort key: the channel's position in the evaluated tree's document
    /// order (module expansions in place). Not contiguous: it counts every
    /// channel with an activated communication object reference, including
    /// ones that own none of this device's objects.
    pub order: u32,
}

/// One directional link from a communication object to a group address,
/// as seen from the communication object's side. `address`/`name` are
/// `None` only if `ga_id` names no address anywhere in the project — a
/// dangling link, which `Command::DeleteGroupAddress` already refuses to
/// create (`CommandError::GroupAddressInUse`) but this stays defensive
/// rather than panicking on data that reached the model some other way
/// (e.g. a future import path that doesn't route through that check).
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct GroupLinkNode {
    pub ga_id: u32,
    /// Formatted per the project's own `GroupAddressStyle`, same as
    /// `GroupAddressNode::address`.
    pub address: Option<String>,
    pub name: Option<String>,
    /// `"Send"` or `"Receive"` (`Direction`'s `Debug` form, same
    /// convention as `dpt_layer`/`description_layer`).
    pub direction: String,
}

/// Builds the detail panel for one device, resolving each communication
/// object's text through the project's string table. `None` if `id` does
/// not name a device in `project` (a stale selection after an edit, for
/// instance).
pub fn build_device_detail(project: &Project, id: knx_core::DeviceId) -> Option<DeviceDetail> {
    let device = project.devices.get(id)?;
    Some(DeviceDetail {
        id: device.id.0,
        name: device.name.clone(),
        description: device.description.clone(),
        address: device.address.map(|a| a.to_string()),
        com_objects: device
            .com_objects
            .iter()
            .filter_map(|com_id| project.devices.com_object(*com_id))
            .map(|com| build_com_object_node(com, project))
            .collect(),
        product: build_device_product_node(device),
    })
}

/// Fills the project-stated half of [`DeviceProductNode`] and leaves the
/// database half at the honest placeholder described on
/// [`DeviceProductNode::resolution`].
fn build_device_product_node(device: &knx_core::DeviceInstance) -> DeviceProductNode {
    let non_empty = |s: &str| Some(s.to_string()).filter(|s| !s.is_empty());
    let product_ref = non_empty(&device.product_ref);
    let program_ref = non_empty(&device.program_ref);
    let resolution = if product_ref.is_none() && program_ref.is_none() {
        ProductResolution::NoReference
    } else {
        ProductResolution::NoDatabase
    };
    DeviceProductNode {
        product_ref,
        program_ref,
        catalog: None,
        resolution,
    }
}

fn build_com_object_node(com: &knx_core::ComObjectInstance, project: &Project) -> ComObjectNode {
    let name = resolved_text(project, &com.text);
    let dpt = com.dpt.value().map(|resolved| resolved.value.to_string());
    let program_dpt = if dpt.is_none() {
        project
            .devices
            .program_defaults(com.id)
            .and_then(|defaults| defaults.dpt.as_ref())
            .map(|resolved| resolved.value.to_string())
    } else {
        None
    };
    let dpt_layer = com.dpt.layer().map(|layer| format!("{layer:?}"));
    let description = resolved_text(project, &com.description);
    let description_layer = com.description.layer().map(|layer| format!("{layer:?}"));
    let flag = |o: &knx_core::Override<bool>| o.value().map(|r| r.value).unwrap_or(false);
    ComObjectNode {
        id: com.id.0,
        number: com.number,
        name,
        dpt,
        dpt_layer,
        description,
        description_layer,
        is_active: com.is_active,
        read: flag(&com.flags.read),
        write: flag(&com.flags.write),
        transmit: flag(&com.flags.transmit),
        update: flag(&com.flags.update),
        communication: flag(&com.flags.communication),
        read_on_init: flag(&com.flags.read_on_init),
        links: com
            .links
            .iter()
            .map(|link| build_group_link_node(link, project))
            .collect(),
        activation: ComObjectActivation::NotEvaluated,
        channel: None,
        program_dpt,
        dpt_text: None,
        function_text: None,
    }
}

/// Finds a `GroupAddressEntry` by id across every installation — a
/// `GroupLink` names its target by id alone, with no installation
/// context of its own to narrow the search.
fn find_group_address_entry(
    project: &Project,
    id: knx_core::GroupAddressId,
) -> Option<&GroupAddressEntry> {
    project
        .installations
        .iter()
        .flat_map(|inst| inst.group_addresses.iter())
        .find(|entry| entry.id == id)
}

fn build_group_link_node(link: &knx_core::GroupLink, project: &Project) -> GroupLinkNode {
    let entry = find_group_address_entry(project, link.ga);
    GroupLinkNode {
        ga_id: link.ga.0,
        address: entry.map(|e| e.address.format(project.info.group_address_style)),
        name: entry.map(|e| e.name.clone()),
        direction: format!("{:?}", link.direction),
    }
}

/// `BuildingPart`s are stored flat, linked by `parent`/`children` ids
/// (DATA_MODEL §5) — this resolves that into the actual nested shape the
/// tree needs, once, in Rust, per ADR-0009.
fn build_building_forest(parts: &[BuildingPart], devices: &Devices) -> Vec<BuildingNode> {
    let by_id: HashMap<BuildingPartId, &BuildingPart> = parts.iter().map(|p| (p.id, p)).collect();

    parts
        .iter()
        .filter(|p| p.parent.is_none())
        .map(|root| build_building_node(root, &by_id, devices))
        .collect()
}

fn build_building_node(
    part: &BuildingPart,
    by_id: &HashMap<BuildingPartId, &BuildingPart>,
    devices: &Devices,
) -> BuildingNode {
    BuildingNode {
        id: part.id.0,
        name: part.name.clone(),
        kind: building_kind_str(part.kind).to_string(),
        children: part
            .children
            .iter()
            .filter_map(|id| by_id.get(id))
            .map(|child| build_building_node(child, by_id, devices))
            .collect(),
        devices: part
            .devices
            .iter()
            .filter_map(|id| devices.get(*id))
            .map(build_device_node)
            .collect(),
    }
}

fn building_kind_str(kind: BuildingPartType) -> &'static str {
    match kind {
        BuildingPartType::Building => "Building",
        BuildingPartType::Floor => "Floor",
        BuildingPartType::Room => "Room",
        BuildingPartType::Corridor => "Corridor",
        BuildingPartType::DistributionBoard => "DistributionBoard",
        BuildingPartType::BuildingPart => "BuildingPart",
        BuildingPartType::Stairway => "Stairway",
        BuildingPartType::RoomPart => "RoomPart",
        BuildingPartType::Area => "Area",
        BuildingPartType::Ground => "Ground",
        BuildingPartType::Segment => "Segment",
    }
}

/// Same wire spelling as `knx-store`'s `style_to_str`, `knx-server`'s
/// `parse_group_address_style` and `knx-etsproj`'s
/// `export::schema11::group_address_style_str` — four crates, one string
/// table, kept in sync only by the shared exhaustive match, since
/// `GroupAddressStyle` itself carries no `Serialize`/`TS` derive
/// (`knx-core` depends on neither crate).
fn group_address_style_str(style: GroupAddressStyle) -> &'static str {
    match style {
        GroupAddressStyle::Free => "Free",
        GroupAddressStyle::TwoLevel => "TwoLevel",
        GroupAddressStyle::ThreeLevel => "ThreeLevel",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use knx_core::{
        Area, BuildingPart, BuildingPartType, CommissioningState, CompletionStatus, DeviceId,
        DeviceInstance, IndividualAddress, Installation, InstallationId, Language, Line, Project,
        SourceRef, Topology,
    };

    fn source() -> SourceRef {
        SourceRef {
            path: "t".into(),
            ets_id: "t".into(),
        }
    }

    fn device(id: u32, name: &str, address: Option<(u8, u8, u8)>) -> DeviceInstance {
        DeviceInstance {
            id: DeviceId(id),
            source: source(),
            name: name.into(),
            description: None,
            address: address.map(|(a, l, d)| IndividualAddress::new(a, l, d).unwrap()),
            product_ref: String::new(),
            program_ref: String::new(),
            commissioning: CommissioningState::default(),
            visibility_calculated: true,
            com_objects: vec![],
            binary_data: vec![],
        }
    }

    #[test]
    fn building_projection_preserves_all_documented_space_kinds() {
        for (kind, token) in [
            (BuildingPartType::Stairway, "Stairway"),
            (BuildingPartType::RoomPart, "RoomPart"),
            (BuildingPartType::Area, "Area"),
            (BuildingPartType::Ground, "Ground"),
            (BuildingPartType::Segment, "Segment"),
        ] {
            let mut project = Project::new(Language("en".into()));
            let mut inst = empty_installation();
            inst.buildings = vec![building(1, "Test", kind, None, vec![], vec![])];
            project.installations.push(inst);
            assert_eq!(
                build_project_tree(&project).installations[0].buildings[0].kind,
                token
            );
        }
    }

    fn building(
        id: u32,
        name: &str,
        kind: BuildingPartType,
        parent: Option<u32>,
        children: Vec<u32>,
        devices: Vec<u32>,
    ) -> BuildingPart {
        BuildingPart {
            id: knx_core::BuildingPartId(id),
            source: source(),
            name: name.into(),
            number: None,
            kind,
            default_line: None,
            completion: CompletionStatus::Undefined,
            children: children.into_iter().map(knx_core::BuildingPartId).collect(),
            devices: devices.into_iter().map(DeviceId).collect(),
            parent: parent.map(knx_core::BuildingPartId),
        }
    }

    fn empty_installation() -> Installation {
        Installation {
            id: InstallationId(0),
            name: "I".into(),
            default_line: None,
            multicast_address: None,
            completion: CompletionStatus::Undefined,
            topology: Topology {
                areas: vec![],
                lines: vec![],
                unassigned: vec![],
            },
            buildings: vec![],
            group_ranges: vec![],
            group_addresses: vec![],
            parameters: vec![],
        }
    }

    #[test]
    fn empty_project_produces_an_empty_tree() {
        let project = Project::new(Language("en".into()));
        let tree = build_project_tree(&project);
        assert_eq!(tree.schema_version, project.schema_version);
        assert_eq!(tree.errors, 0);
        assert_eq!(tree.warnings, 0);
        assert!(!tree.is_modified);
        assert!(tree.server_incarnation.is_none());
        assert!(tree.snapshot_revision.is_none());
        assert!(tree.group_address_context_session_id.is_none());
        assert!(tree.installations.is_empty());
    }

    /// `ProjectTree::group_address_style` (KNOWN_LIMITATIONS.md §84) mirrors
    /// `Project::info.group_address_style` for every style, not just the
    /// `Project::new` default of `ThreeLevel`, so the inspector shows the
    /// project's real choice rather than a constant.
    #[test]
    fn project_tree_carries_the_projects_group_address_style() {
        let mut project = Project::new(Language("en".into()));
        assert_eq!(
            build_project_tree(&project).group_address_style,
            "ThreeLevel"
        );

        project.info.group_address_style = knx_core::GroupAddressStyle::Free;
        assert_eq!(build_project_tree(&project).group_address_style, "Free");

        project.info.group_address_style = knx_core::GroupAddressStyle::TwoLevel;
        assert_eq!(build_project_tree(&project).group_address_style, "TwoLevel");
    }

    #[test]
    fn installation_with_no_buildings_or_topology_has_empty_children() {
        let mut project = Project::new(Language("en".into()));
        project.installations.push(empty_installation());
        let tree = build_project_tree(&project);
        let inst = &tree.installations[0];
        assert!(inst.topology.is_empty());
        assert!(inst.buildings.is_empty());
        assert!(inst.unassigned.is_empty());
    }

    #[test]
    fn topology_resolves_area_line_device_in_order() {
        let mut project = Project::new(Language("en".into()));
        project.devices.insert(device(1, "Dimmer", Some((1, 1, 1))));
        project.devices.insert(device(2, "Switch", None));

        let mut inst = empty_installation();
        inst.topology.areas.push(Area {
            id: knx_core::AreaId(1),
            source: source(),
            name: "Area 1".into(),
            address: 1,
            completion: CompletionStatus::Undefined,
            lines: vec![knx_core::LineId(1)],
        });
        inst.topology.lines.push(Line {
            id: knx_core::LineId(1),
            source: source(),
            name: "Line 1".into(),
            address: 1,
            medium_ref: "TP".into(),
            domain_address: None,
            domain_address_is_checked: None,
            ip_routing_multicast_address: None,
            multicast_ttl: None,
            completion: CompletionStatus::Undefined,
            devices: vec![DeviceId(1), DeviceId(2)],
        });
        project.installations.push(inst);

        let tree = build_project_tree(&project);
        let area = &tree.installations[0].topology[0];
        assert_eq!(area.name, "Area 1");
        assert_eq!(area.lines[0].devices.len(), 2);
        assert_eq!(area.lines[0].devices[0].name, "Dimmer");
        assert_eq!(area.lines[0].devices[0].address.as_deref(), Some("1.1.1"));
        assert_eq!(area.lines[0].devices[1].address, None);
    }

    #[test]
    fn unassigned_devices_form_their_own_bucket() {
        let mut project = Project::new(Language("en".into()));
        project.devices.insert(device(9, "Orphan", None));
        let mut inst = empty_installation();
        inst.topology.unassigned.push(DeviceId(9));
        project.installations.push(inst);

        let tree = build_project_tree(&project);
        assert_eq!(tree.installations[0].unassigned.len(), 1);
        assert_eq!(tree.installations[0].unassigned[0].name, "Orphan");
    }

    #[test]
    fn building_hierarchy_nests_by_parent_child_not_flat() {
        let mut project = Project::new(Language("en".into()));
        project.devices.insert(device(5, "Lamp", None));
        let mut inst = empty_installation();
        inst.buildings = vec![
            building(
                1,
                "Building",
                BuildingPartType::Building,
                None,
                vec![2],
                vec![],
            ),
            building(
                2,
                "Floor 1",
                BuildingPartType::Floor,
                Some(1),
                vec![3],
                vec![],
            ),
            building(
                3,
                "Room 1",
                BuildingPartType::Room,
                Some(2),
                vec![],
                vec![5],
            ),
        ];
        project.installations.push(inst);

        let tree = build_project_tree(&project);
        let roots = &tree.installations[0].buildings;
        assert_eq!(roots.len(), 1);
        assert_eq!(roots[0].name, "Building");
        assert_eq!(roots[0].kind, "Building");
        assert_eq!(roots[0].children[0].name, "Floor 1");
        assert_eq!(roots[0].children[0].children[0].name, "Room 1");
        assert_eq!(roots[0].children[0].children[0].devices[0].name, "Lamp");
    }

    #[test]
    fn a_dangling_building_device_reference_is_dropped_not_panicked() {
        // knx-etsproj's validate.rs deliberately does not check BuildingPart
        // device references for dangling ids (no measured case has motivated
        // it yet) — the projection must stay resilient to that gap rather
        // than crash the whole desktop app over one malformed project.
        let mut project = Project::new(Language("en".into()));
        let mut inst = empty_installation();
        inst.buildings = vec![building(
            1,
            "Room",
            BuildingPartType::Room,
            None,
            vec![],
            vec![404],
        )];
        project.installations.push(inst);

        let tree = build_project_tree(&project);
        assert!(tree.installations[0].buildings[0].devices.is_empty());
    }

    #[test]
    fn build_device_detail_resolves_name_dpt_and_layer_for_each_com_object() {
        let project = project_with_one_device();
        let detail = build_device_detail(&project, knx_core::DeviceId(1)).unwrap();

        assert_eq!(detail.id, 1);
        assert_eq!(detail.name, "Switch");
        assert_eq!(detail.description.as_deref(), Some("Hallway switch"));
        assert_eq!(detail.address.as_deref(), Some("1.1.1"));
        assert_eq!(detail.com_objects.len(), 1);

        let com = &detail.com_objects[0];
        assert_eq!(com.number, 0);
        assert_eq!(com.name.as_deref(), Some("Switch on/off"));
        assert_eq!(com.dpt.as_deref(), Some("DPST-1-1"));
        assert_eq!(com.dpt_layer.as_deref(), Some("UserEdit"));
        assert_eq!(com.description.as_deref(), Some("Hallway light switch"));
        assert_eq!(com.description_layer.as_deref(), Some("Instance"));
        assert!(com.is_active);
        assert!(!com.read); // ResolvedFlags::none() sets nothing
        assert!(com.links.is_empty());
    }

    #[test]
    fn an_empty_dpt_slot_shows_the_program_default_beside_it_not_in_it() {
        let mut project = project_with_one_device();
        let id = knx_core::ComObjectInstanceId(1);
        project.devices.com_object_mut(id).unwrap().dpt = knx_core::Override::Empty;
        project.devices.set_program_defaults(
            id,
            knx_core::ProgramDefaults {
                dpt: Some(knx_core::Resolved {
                    value: DptRef {
                        main: 5,
                        sub: Some(1),
                    },
                    layer: knx_core::Layer::Program,
                }),
                ..Default::default()
            },
        );
        let com = &build_device_detail(&project, knx_core::DeviceId(1))
            .unwrap()
            .com_objects[0];
        assert_eq!(com.dpt, None, "the empty slot stays empty");
        assert_eq!(com.program_dpt.as_deref(), Some("DPST-5-1"));
        assert_eq!((&com.dpt_text, &com.function_text), (&None, &None));
    }

    #[test]
    fn a_stated_dpt_hides_the_program_default() {
        let mut project = project_with_one_device();
        let id = knx_core::ComObjectInstanceId(1);
        project.devices.set_program_defaults(
            id,
            knx_core::ProgramDefaults {
                dpt: Some(knx_core::Resolved {
                    value: DptRef {
                        main: 5,
                        sub: Some(1),
                    },
                    layer: knx_core::Layer::Program,
                }),
                ..Default::default()
            },
        );
        let com = &build_device_detail(&project, knx_core::DeviceId(1))
            .unwrap()
            .com_objects[0];
        assert_eq!(com.dpt.as_deref(), Some("DPST-1-1"));
        assert_eq!(com.program_dpt, None);
    }

    #[test]
    fn build_device_detail_projects_a_group_link_with_its_resolved_address_and_name() {
        let mut project = project_with_one_device();
        let mut inst = empty_installation();
        inst.group_addresses.push(knx_core::GroupAddressEntry {
            id: knx_core::GroupAddressId(9),
            source: source(),
            name: "Hallway light on/off".into(),
            address: knx_core::GroupAddress::parse(
                "1/1/1",
                knx_core::GroupAddressStyle::ThreeLevel,
            )
            .unwrap(),
            central: false,
            unfiltered: false,
            range: None,
        });
        project.installations.push(inst);
        project
            .devices
            .com_object_mut(knx_core::ComObjectInstanceId(1))
            .unwrap()
            .links
            .push(knx_core::GroupLink {
                ga: knx_core::GroupAddressId(9),
                direction: knx_core::Direction::Send,
            });

        let detail = build_device_detail(&project, knx_core::DeviceId(1)).unwrap();
        let links = &detail.com_objects[0].links;
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].ga_id, 9);
        assert_eq!(links[0].address.as_deref(), Some("1/1/1"));
        assert_eq!(links[0].name.as_deref(), Some("Hallway light on/off"));
        assert_eq!(links[0].direction, "Send");
    }

    #[test]
    fn build_device_detail_stays_defensive_on_a_dangling_group_link() {
        // `Command::DeleteGroupAddress` already refuses to create this state
        // (`CommandError::GroupAddressInUse`), but the projection stays
        // defensive against data that reached the model some other way,
        // same rationale as `a_dangling_building_device_reference_is_dropped_not_panicked`.
        let mut project = project_with_one_device();
        project
            .devices
            .com_object_mut(knx_core::ComObjectInstanceId(1))
            .unwrap()
            .links
            .push(knx_core::GroupLink {
                ga: knx_core::GroupAddressId(404),
                direction: knx_core::Direction::Receive,
            });

        let detail = build_device_detail(&project, knx_core::DeviceId(1)).unwrap();
        let links = &detail.com_objects[0].links;
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].ga_id, 404);
        assert_eq!(links[0].address, None);
        assert_eq!(links[0].name, None);
        assert_eq!(links[0].direction, "Receive");
    }

    #[test]
    fn build_device_detail_returns_none_for_an_unknown_device() {
        let project = project_with_one_device();
        assert!(build_device_detail(&project, knx_core::DeviceId(99)).is_none());
    }

    #[test]
    fn a_device_with_both_refs_gets_them_verbatim_and_an_unresolved_placeholder() {
        // `project_with_one_device` already sets `product_ref: "P"` and
        // `program_ref: "H"` — this crate has no database to check them
        // against, so `resolution` must sit at the `NoDatabase` placeholder
        // `apps/knx-server` is obliged to overwrite (see
        // `DeviceProductNode::resolution`'s doc comment), not `NoReference`.
        let project = project_with_one_device();
        let detail = build_device_detail(&project, knx_core::DeviceId(1)).unwrap();
        assert_eq!(detail.product.product_ref.as_deref(), Some("P"));
        assert_eq!(detail.product.program_ref.as_deref(), Some("H"));
        assert!(detail.product.catalog.is_none());
        assert_eq!(detail.product.resolution, ProductResolution::NoDatabase);
    }

    #[test]
    fn a_device_with_neither_ref_reports_no_reference() {
        // `device()` sets both `product_ref`/`program_ref` to `""` — an
        // empty ref is not a usable ref, so this must resolve to
        // `NoReference`, not the `NoDatabase` placeholder, and needs no
        // server-side overwrite at all.
        let mut project = Project::new(Language("en".into()));
        project.devices.insert(device(1, "Orphan", None));
        let detail = build_device_detail(&project, knx_core::DeviceId(1)).unwrap();
        assert_eq!(detail.product.product_ref, None);
        assert_eq!(detail.product.program_ref, None);
        assert!(detail.product.catalog.is_none());
        assert_eq!(detail.product.resolution, ProductResolution::NoReference);
    }

    #[test]
    fn device_node_carries_its_communication_object_count() {
        let mut project = project_with_one_device();
        let mut inst = empty_installation();
        inst.topology.unassigned.push(knx_core::DeviceId(1));
        project.installations.push(inst);

        let tree = build_project_tree(&project);
        assert_eq!(tree.installations[0].unassigned[0].com_object_count, 1);
    }

    fn project_with_one_device() -> Project {
        use knx_core::{
            ComObjectInstance, ComObjectInstanceId, CommissioningState, DeviceId, DptRef,
            IndividualAddress, Layer, ResolvedFlags, Text,
        };

        let mut project = Project::new(Language("en".into()));
        project.devices.insert(DeviceInstance {
            id: DeviceId(1),
            source: source(),
            name: "Switch".into(),
            description: Some("Hallway switch".into()),
            address: Some(IndividualAddress::new(1, 1, 1).unwrap()),
            product_ref: "P".into(),
            program_ref: "H".into(),
            commissioning: CommissioningState::default(),
            visibility_calculated: true,
            com_objects: vec![ComObjectInstanceId(1)],
            binary_data: vec![],
        });
        project.devices.insert_com_object(ComObjectInstance {
            id: ComObjectInstanceId(1),
            source: source(),
            device: DeviceId(1),
            number: 0,
            text: knx_core::Override::Value(knx_core::Resolved {
                value: Text::Literal("Switch on/off".into()),
                layer: Layer::Program,
            }),
            description: knx_core::Override::Value(knx_core::Resolved {
                value: Text::Literal("Hallway light switch".into()),
                layer: Layer::Instance,
            }),
            dpt: knx_core::Override::Value(knx_core::Resolved {
                value: DptRef {
                    main: 1,
                    sub: Some(1),
                },
                layer: Layer::UserEdit,
            }),
            flags: ResolvedFlags::none(),
            size: None,
            is_active: true,
            links: vec![],
            module_instance: None,
        });
        project
    }

    #[test]
    fn group_addresses_are_projected_and_formatted_per_project_style() {
        let mut project = Project::new(Language("en".into()));
        project.info.group_address_style = knx_core::GroupAddressStyle::TwoLevel;
        let mut inst = empty_installation();
        inst.group_addresses.push(knx_core::GroupAddressEntry {
            id: knx_core::GroupAddressId(1),
            source: source(),
            name: "Living room light".into(),
            address: knx_core::GroupAddress::parse("4/612", knx_core::GroupAddressStyle::TwoLevel)
                .unwrap(),
            central: false,
            unfiltered: false,
            range: None,
        });
        project.installations.push(inst);

        let tree = build_project_tree(&project);
        let ga = &tree.installations[0].group_addresses[0];
        assert_eq!(ga.id, 1);
        assert_eq!(ga.name, "Living room light");
        assert_eq!(ga.address, "4/612");
    }

    #[test]
    fn group_ranges_are_projected_with_their_parent_link() {
        let mut project = knx_core::Project::new(knx_core::Language("en".into()));
        project.installations.push(knx_core::Installation {
            id: knx_core::InstallationId(0),
            name: "I".into(),
            default_line: None,
            multicast_address: None,
            completion: knx_core::CompletionStatus::FinishedDesign,
            topology: knx_core::Topology {
                areas: vec![],
                lines: vec![],
                unassigned: vec![],
            },
            buildings: vec![],
            group_ranges: vec![
                knx_core::GroupRange {
                    id: knx_core::GroupRangeId(1),
                    source: knx_core::SourceRef {
                        path: "t".into(),
                        ets_id: "t".into(),
                    },
                    name: "Main".into(),
                    start: knx_core::GroupAddress::from_raw(0),
                    end: knx_core::GroupAddress::from_raw(2047),
                    parent: None,
                    children: vec![knx_core::GroupRangeId(2)],
                },
                knx_core::GroupRange {
                    id: knx_core::GroupRangeId(2),
                    source: knx_core::SourceRef {
                        path: "t".into(),
                        ets_id: "t".into(),
                    },
                    name: "Middle".into(),
                    start: knx_core::GroupAddress::from_raw(0),
                    end: knx_core::GroupAddress::from_raw(255),
                    parent: Some(knx_core::GroupRangeId(1)),
                    children: vec![],
                },
            ],
            group_addresses: vec![],
            parameters: vec![],
        });
        let tree = build_project_tree(&project);
        let ranges = &tree.installations[0].group_ranges;
        assert_eq!(ranges.len(), 2);
        assert_eq!(ranges[0].id, 1);
        assert_eq!(ranges[0].parent, None);
        assert_eq!(ranges[1].parent, Some(1));
    }

    /// One communication object on `device`, linked to each `(ga, direction)`
    /// pair, stating `dpt` when `dpt` is `Some`.
    fn linked_com_object(
        com_id: u32,
        device_id: u32,
        number: u16,
        name: &str,
        dpt: Option<(u16, u16)>,
        links: &[(u32, knx_core::Direction)],
    ) -> knx_core::ComObjectInstance {
        knx_core::ComObjectInstance {
            id: knx_core::ComObjectInstanceId(com_id),
            source: source(),
            device: DeviceId(device_id),
            number,
            text: knx_core::Override::Value(knx_core::Resolved {
                value: knx_core::Text::Literal(name.into()),
                layer: knx_core::Layer::Program,
            }),
            description: knx_core::Override::Absent,
            dpt: match dpt {
                Some((main, sub)) => knx_core::Override::Value(knx_core::Resolved {
                    value: DptRef {
                        main,
                        sub: Some(sub),
                    },
                    layer: knx_core::Layer::Program,
                }),
                None => knx_core::Override::Absent,
            },
            flags: knx_core::ResolvedFlags::none(),
            size: None,
            is_active: true,
            links: links
                .iter()
                .map(|(ga, direction)| knx_core::GroupLink {
                    ga: knx_core::GroupAddressId(*ga),
                    direction: *direction,
                })
                .collect(),
            module_instance: None,
        }
    }

    fn group_address(id: u32, name: &str, address: &str, range: Option<u32>) -> GroupAddressEntry {
        GroupAddressEntry {
            id: knx_core::GroupAddressId(id),
            source: source(),
            name: name.into(),
            address: knx_core::GroupAddress::parse(address, GroupAddressStyle::ThreeLevel).unwrap(),
            central: false,
            unfiltered: false,
            range: range.map(knx_core::GroupRangeId),
        }
    }

    #[test]
    fn a_group_address_projects_its_range_its_resolved_dpt_and_every_link_that_reaches_it() {
        let mut project = Project::new(Language("en".into()));
        project
            .devices
            .insert(device(1, "Push button", Some((1, 1, 13))));
        project
            .devices
            .insert(device(2, "Actuator", Some((1, 1, 11))));
        project.devices.insert_com_object(linked_com_object(
            10,
            1,
            0,
            "Switch light",
            Some((1, 1)),
            &[(7, knx_core::Direction::Send)],
        ));
        project.devices.insert_com_object(linked_com_object(
            11,
            2,
            3,
            "Switch light",
            Some((1, 1)),
            &[(7, knx_core::Direction::Receive)],
        ));
        let mut inst = empty_installation();
        inst.group_addresses
            .push(group_address(7, "Living room light", "1/0/1", Some(4)));
        project.installations.push(inst);

        let ga = &build_project_tree(&project).installations[0].group_addresses[0];
        assert_eq!(ga.range, Some(4));
        assert_eq!(ga.dpts, vec!["DPST-1-1".to_string()]);
        assert_eq!(ga.links.len(), 2);
        // `Devices::com_objects` is id-ordered, so the two rows arrive in a
        // fixed order rather than a hash-map one.
        assert_eq!(ga.links[0].device_name.as_deref(), Some("Push button"));
        assert_eq!(ga.links[0].device_address.as_deref(), Some("1.1.13"));
        assert_eq!(ga.links[0].com_object_number, 0);
        assert_eq!(ga.links[0].com_object_name.as_deref(), Some("Switch light"));
        assert_eq!(ga.links[0].direction, "Send");
        assert_eq!(ga.links[1].device_name.as_deref(), Some("Actuator"));
        assert_eq!(ga.links[1].com_object_id, 11);
        assert_eq!(ga.links[1].direction, "Receive");
    }

    #[test]
    fn linked_objects_that_disagree_on_the_dpt_report_the_conflict_instead_of_picking_one() {
        let mut project = Project::new(Language("en".into()));
        project.devices.insert(device(1, "A", None));
        project.devices.insert(device(2, "B", None));
        project.devices.insert_com_object(linked_com_object(
            10,
            1,
            0,
            "Value",
            Some((5, 1)),
            &[(7, knx_core::Direction::Send)],
        ));
        project.devices.insert_com_object(linked_com_object(
            11,
            2,
            0,
            "Value",
            Some((1, 1)),
            &[(7, knx_core::Direction::Receive)],
        ));
        let mut inst = empty_installation();
        inst.group_addresses
            .push(group_address(7, "Disputed", "1/0/1", None));
        project.installations.push(inst);

        let ga = &build_project_tree(&project).installations[0].group_addresses[0];
        // Sorted by `DptRef`'s own `Ord`, so the pair is reported in the same
        // order on every run.
        assert_eq!(
            ga.dpts,
            vec!["DPST-1-1".to_string(), "DPST-5-1".to_string()]
        );
    }

    #[test]
    fn one_object_linked_in_both_directions_states_its_dpt_once_but_shows_two_rows() {
        let mut project = Project::new(Language("en".into()));
        project.devices.insert(device(1, "Dimmer", None));
        project.devices.insert_com_object(linked_com_object(
            10,
            1,
            2,
            "Dim value",
            Some((5, 1)),
            &[
                (7, knx_core::Direction::Send),
                (7, knx_core::Direction::Receive),
            ],
        ));
        let mut inst = empty_installation();
        inst.group_addresses
            .push(group_address(7, "Dim", "1/0/1", None));
        project.installations.push(inst);

        let ga = &build_project_tree(&project).installations[0].group_addresses[0];
        assert_eq!(ga.dpts, vec!["DPST-5-1".to_string()]);
        assert_eq!(ga.links.len(), 2);
    }

    #[test]
    fn an_unlinked_group_address_states_no_dpt_and_no_links() {
        let mut project = Project::new(Language("en".into()));
        let mut inst = empty_installation();
        inst.group_addresses
            .push(group_address(7, "Nothing links here", "1/0/1", None));
        project.installations.push(inst);

        let ga = &build_project_tree(&project).installations[0].group_addresses[0];
        assert!(ga.dpts.is_empty());
        assert!(ga.links.is_empty());
    }

    #[test]
    fn a_link_from_an_object_whose_device_is_missing_still_reaches_the_projection() {
        // Same defensive stance `GroupLinkNode` takes towards a dangling
        // `ga_id`: the link is stated by the project, so it is shown, with
        // the parts that cannot be resolved left honestly empty rather than
        // the whole row dropped.
        let mut project = Project::new(Language("en".into()));
        project.devices.insert_com_object(linked_com_object(
            10,
            404,
            0,
            "Orphan object",
            None,
            &[(7, knx_core::Direction::Send)],
        ));
        let mut inst = empty_installation();
        inst.group_addresses
            .push(group_address(7, "Linked by a ghost", "1/0/1", None));
        project.installations.push(inst);

        let ga = &build_project_tree(&project).installations[0].group_addresses[0];
        assert_eq!(ga.links.len(), 1);
        assert_eq!(ga.links[0].device_id, 404);
        assert_eq!(ga.links[0].device_name, None);
        assert_eq!(ga.links[0].device_address, None);
        assert!(ga.dpts.is_empty());
    }

    #[test]
    fn links_are_projected_per_address_not_smeared_across_all_of_them() {
        let mut project = Project::new(Language("en".into()));
        project.devices.insert(device(1, "Sensor", None));
        project.devices.insert_com_object(linked_com_object(
            10,
            1,
            0,
            "Temperature",
            Some((9, 1)),
            &[(7, knx_core::Direction::Send)],
        ));
        let mut inst = empty_installation();
        inst.group_addresses
            .push(group_address(7, "Temperature", "3/0/1", None));
        inst.group_addresses
            .push(group_address(8, "Setpoint", "3/0/2", None));
        project.installations.push(inst);

        let addresses = &build_project_tree(&project).installations[0].group_addresses;
        assert_eq!(addresses[0].links.len(), 1);
        assert!(addresses[1].links.is_empty());
        assert!(addresses[1].dpts.is_empty());
    }
}
