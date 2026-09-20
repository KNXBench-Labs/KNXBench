//! The one place that spells a retained value's key, one key per source element instance.
//!
//! A key here is the element's schema-shaped path plus its own ETS id:
//! `".../Segment/DeviceInstance[@Id='P-0001-0_DI-1']"`. Three properties
//! matter, and all three are load-bearing rather than stylistic:
//!
//! - **Instance-exact.** A key with no predicate (`".../DeviceInstance"`)
//!   collapses every device's `SerialNumber` into one entry. Writing that
//!   entry back out would splice one device's hardware serial number onto
//!   all the others — the corruption `KNOWN_LIMITATIONS.md` §34 exists to
//!   forbid. With the id in the key, a value either goes back where it came
//!   from or nowhere at all.
//! - **Reconstructible from the domain model alone.** Only the element's own
//!   id appears, never its ancestors': ETS ids are unique across a project
//!   (`P-0512-0_DI-2` carries project, installation and entity kind), so
//!   ancestors would add nothing but the obligation to thread a parent xpath
//!   through every writer.
//!
//! # What the rule rests on
//!
//! [`crate::export::retained`] decides *ambiguous* by looking at the values
//! a key collected, not by counting the elements that produced it: two
//! values under one key cannot both be written, so neither is. That is an
//! exact rule only while each key belongs to exactly one element, which in
//! turn rests on two assumptions:
//!
//! 1. **ETS `@Id` values are unique within an installation.** Break it, and
//!    a key produced by two elements where only *one* of them carried the
//!    attribute still looks unique, and the value is written onto both
//!    copies (`tests/retained_ambiguity.rs`,
//!    `a_duplicate_ets_id_puts_a_retained_value_on_both_copies`, pins it).
//!    No ETS file measured does this, and such a file is already damaged
//!    long before export: the model's own id tables collapse the two
//!    elements into one, so the first element's name, address and
//!    everything else are gone before the retained store is consulted. The
//!    place to catch it is import validation, which is why this is written
//!    down for the task that owns `validate.rs` rather than guarded here.
//! 2. **The writers emit at most one element per key.** They do — one
//!    `Segment` per `Line`, one element per modeled entity — so a retained
//!    value that finds a home finds exactly one.
//!
//! Carrying an element-instance count instead of deriving ambiguity from
//! the values would mean carrying that count from `map.rs` to the exporter
//! through the opaque store, which is a stored-project format change for a
//! shape no ETS file has. The assumptions are cheaper written down and
//! tested than encoded.
//! - **Single-sourced.** `map.rs`, `lib.rs` and `export/` each used to spell
//!   the device path themselves, and they had already drifted: the importer
//!   wrote `…/Line/Segment/DeviceInstance[@Id='…']` while the schema-≥21
//!   exporter looked up `…/Line/DeviceInstance[@Id='…']`, so every
//!   `ComObjectInstanceRef/@ChannelId` silently failed to come back. One
//!   module, one formula, no drift.
//!
//! Two elements have no id to key by. `Segment` sits between `Line` and
//! `DeviceInstance` at schema ≥21, but `knx_core` has no segment entity and
//! the exporter re-synthesizes exactly one per line, so its attributes are
//! keyed by the owning line. A line carrying two segments therefore
//! produces one key twice: where both segments spell the same attribute,
//! the values differ and [`crate::export::retained::RetainedAttrs`] refuses
//! to write either; where only one of them spells it, the value goes onto
//! the synthesized segment, which is the merge of both — the same merge the
//! parser already performs on `MediumTypeRefId` and the domain-address
//! attributes. Pinned by
//! `an_asymmetric_segment_attribute_lands_on_the_merged_segment`.
//! `ComObjectInstanceRef` has a `RefId` that repeats across devices, so it
//! is keyed by its owning device's xpath plus that `RefId`.

/// Schema 11 and schema ≥21 agree on this prefix, and neither predicates it
/// by installation — `Installation` carries no `@Id` at all at either
/// schema version, so there is nothing to predicate it *by*. A file with
/// two installations is therefore not distinguished by this key: the second
/// installation's retained values land on the same keys as the first, and
/// what happens next is the rule above — a differing value under a shared
/// key is dropped with a warning, an attribute only one installation
/// carries is written onto every installation the writer reaches — and the
/// writers do emit one `<Installation>` element per modeled installation.
///
/// The identity to predicate by would be `@InstallationId`, not `@Id`: the
/// ETS4 reference project spells `InstallationId="0"`, and both schema-≥21
/// projects measured omit it entirely. All three carry exactly one
/// installation, so the collision has never had an instance to occur on.
/// Keying by it would mean threading the enclosing installation's number
/// through every writer, which is the ancestor-path obligation this module
/// exists to avoid, for a file shape nothing has produced yet.
pub(crate) const INSTALLATION: &str = "/KNX/Project/Installations/Installation";

pub(crate) fn area(area_ets_id: &str) -> String {
    format!("{INSTALLATION}/Topology/Area[@Id='{area_ets_id}']")
}

pub(crate) fn line(line_ets_id: &str) -> String {
    format!("{INSTALLATION}/Topology/Area/Line[@Id='{line_ets_id}']")
}

/// Schema ≥21 only. See the module doc on why this step is keyed by its
/// line.
pub(crate) fn segment(line_ets_id: &str) -> String {
    format!("{}/Segment", line(line_ets_id))
}

pub(crate) fn device_v11(device_ets_id: &str) -> String {
    format!("{INSTALLATION}/Topology/Area/Line/DeviceInstance[@Id='{device_ets_id}']")
}

pub(crate) fn device_v21(device_ets_id: &str) -> String {
    format!("{INSTALLATION}/Topology/Area/Line/Segment/DeviceInstance[@Id='{device_ets_id}']")
}

/// A device on no line at all. Same element, different branch of the
/// topology, so a different path — and the same at both schema versions,
/// since `UnassignedDevices` has no `Segment` under it.
pub(crate) fn unassigned_device(device_ets_id: &str) -> String {
    format!("{INSTALLATION}/Topology/UnassignedDevices/DeviceInstance[@Id='{device_ets_id}']")
}

/// Keyed by the owning device's xpath as well as the `RefId`: a `RefId`
/// (`"O-2_R-237"`) names a communication object within its device's
/// application program and recurs verbatim on every device running the same
/// program.
pub(crate) fn com_object(device_xpath: &str, ref_id: &str) -> String {
    format!("{device_xpath}/ComObjectInstanceRefs/ComObjectInstanceRef[@RefId='{ref_id}']")
}

/// A device's `BinaryData` leaf, keyed by its owning device as well as its
/// own id: the id is a GUID in every sample measured, but the device is what
/// makes the key mean "this blob on this device".
pub(crate) fn binary_data(device_xpath: &str, binary_data_id: &str) -> String {
    format!("{device_xpath}/BinaryData/BinaryData[@Id='{binary_data_id}']")
}

pub(crate) fn group_range(range_ets_id: &str) -> String {
    format!("{INSTALLATION}/GroupAddresses/GroupRanges/GroupRange[@Id='{range_ets_id}']")
}

pub(crate) fn group_address(address_ets_id: &str) -> String {
    format!(
        "{INSTALLATION}/GroupAddresses/GroupRanges/GroupRange/GroupAddress[@Id='{address_ets_id}']"
    )
}

/// `Buildings/BuildingPart` at schema 11, `Locations/Space` at schema ≥21 —
/// the same tree under different names (`export/schema21.rs`'s module doc),
/// so the caller names the pair it is writing.
pub(crate) fn building_part(container: &str, element: &str, part_ets_id: &str) -> String {
    format!("{INSTALLATION}/{container}/{element}[@Id='{part_ets_id}']")
}
