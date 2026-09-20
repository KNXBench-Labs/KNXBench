//! The one place that spells a retained value's key, one key per source element instance.
//!
//! A key here is the element's schema-shaped path plus its own ETS id:
//! `".../Segment/DeviceInstance[@Id='P-0001-0_DI-1']"`. Two properties
//! matter, and both are load-bearing rather than stylistic:
//!
//! - **Instance-exact.** A key with no predicate (`".../DeviceInstance"`)
//!   collapses every device's `SerialNumber` into one entry, so the import
//!   report and the opaque store would both claim one device's hardware
//!   serial number belongs to all the others. With the id in the key, a
//!   preserved value names the element it actually came from.
//! - **Reconstructible from the domain model alone.** Only the element's own
//!   id appears, never its ancestors': ETS ids are unique across a project
//!   (`P-0512-0_DI-2` carries project, installation and entity kind), so
//!   ancestors would add nothing but the obligation to thread a parent xpath
//!   through every caller.
//! - **Single-sourced.** `map.rs` and `lib.rs` each used to spell the device
//!   path themselves, and they had already drifted. One module, one formula,
//!   no drift.
//!
//! # What the rule rests on
//!
//! Exactness rests on **ETS `@Id` values being unique within an
//! installation**. Break it, and a key produced by two elements where only
//! *one* of them carried the attribute still looks unique, so the report
//! attributes the value to both copies. No ETS file measured here does
//! this, and such a file is already damaged long before the opaque store is
//! consulted: the model's own id tables collapse the two elements into one,
//! so the first element's name, address and everything else are gone first.
//! The place to catch it is import validation, which is why this is written
//! down for the task that owns `validate.rs` rather than guarded here.
//!
//! Two elements have no id to key by. `Segment` sits between `Line` and
//! `DeviceInstance` at schema ≥21, but `knx_core` has no segment entity, so
//! its attributes are keyed by the owning line — the same merge the parser
//! already performs on `MediumTypeRefId` and the domain-address attributes.
//! `ComObjectInstanceRef` has a `RefId` that repeats across devices, so it
//! is keyed by its owning device's xpath plus that `RefId`.
//!
//! Writing `.knxproj` was withdrawn on 2026-09-20 (ADR-0028). These keys
//! outlive it: they are how the opaque store (ADR-0006) and the import
//! report say *where in the source file* a preserved value was found.

pub(crate) const INSTALLATION: &str = "/KNX/Project/Installations/Installation";

pub(crate) fn area(area_ets_id: &str) -> String {
    format!("{INSTALLATION}/Topology/Area[@Id='{area_ets_id}']")
}

pub(crate) fn line(line_ets_id: &str) -> String {
    format!("{INSTALLATION}/Topology/Area/Line[@Id='{line_ets_id}']")
}

/// Schema ≥21 only. See the module doc on why this step is keyed by its
/// line rather than by itself.
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
/// the same tree under different names, so the caller names the pair it is
/// reading.
pub(crate) fn building_part(container: &str, element: &str, part_ets_id: &str) -> String {
    format!("{INSTALLATION}/{container}/{element}[@Id='{part_ets_id}']")
}
