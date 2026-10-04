//! The command layer: every mutation is a `Command` whose `apply` returns
//! its own inverse, so undo and redo replay the same mechanism (ARCHITECTURE
//! §6). Validation lives here, not in the UI and not in storage. Any command
//! that changes a `Resolved<T>` sets its layer to `Layer::UserEdit`.

use std::collections::HashSet;
use std::fmt;

use crate::address::GroupAddressStyle;
use crate::building::BuildingPart;
use crate::device::{ComObjectInstance, DeviceInstance, ProgramDefaults};
use crate::devices::Devices;
use crate::dpt::DptRef;
use crate::flags::{ComFlagKind, Direction, GroupLink};
use crate::group::{GroupAddressEntry, GroupRange};
use crate::ids::{
    AreaId, BuildingPartId, ComObjectInstanceId, DeviceId, GroupAddressId, GroupRangeId,
    InstallationId, LineId, ParameterInstanceId, SourceRef,
};
use crate::installation::Installation;
use crate::parameter::ParameterInstance;
use crate::project::{IdAllocators, Project};
use crate::provenance::{Layer, Override, Resolved};
use crate::string_table::Text;
use crate::topology::{Area, Line, Topology};
use crate::validation::{
    check_group_address_in_range, check_group_link_target_exists,
    check_group_range_is_well_ordered, check_group_range_nests_in_parent,
    check_individual_address_on_line, check_no_duplicate_area_address,
    check_no_duplicate_group_address, check_no_duplicate_individual_address,
    check_no_duplicate_line_address, check_no_overlapping_group_range, ValidationError,
};
use crate::{GroupAddress, IndividualAddress};

/// Manufacturer evidence that a device's hardware is a coupler
/// (`Hardware/@IsCoupler` in the product database). Carries the product
/// reference it was read for so a command cannot apply it to another device
/// kind; the core does not read manufacturer data itself.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CouplerEvidence {
    pub product_ref: String,
}

/// One topology slot a device can occupy: a line's device list or an
/// installation's unassigned list. Names the placement the user keeps in a
/// [`Command::RepairDevicePlacement`] (MODEL-02).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DevicePlacementSlot {
    Line(LineId),
    Unassigned(InstallationId),
}

/// One occurrence removed by a repair, in removal order: the container
/// (`line: None` = the installation's unassigned list) and the index it
/// had when it was removed. Reinserting in reverse order restores the exact
/// imported lists.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RemovedDevicePlacement {
    pub installation: InstallationId,
    pub line: Option<LineId>,
    pub position: usize,
}

/// One area→line reference removed by [`Command::RepairLineOwner`], in
/// removal order, with the index it had in that area's line list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RemovedLineReference {
    pub installation: InstallationId,
    pub area: AreaId,
    pub position: usize,
}

/// A single reversible mutation. Commands addressed by an entity id act in
/// the installation that owns that entity (MODEL-01); an id found in several
/// installations is refused as ambiguous. Root-level creates take an
/// optional explicit installation (`None` = the first one). No command
/// connects two installations: such a move or link is refused with
/// `CommandError::CrossInstallation`.
#[derive(Debug, Clone, PartialEq)]
pub enum Command {
    SetIndividualAddress {
        device: DeviceId,
        address: Option<IndividualAddress>,
    },
    /// Assigns an address to a device whose manufacturer data classifies its
    /// hardware as a coupler, which is what permits device octet 0 on its
    /// line (RESEARCH §25, MODEL-03). The application layer supplies the
    /// evidence from the product database; the command re-checks that it
    /// still describes this device's product. Line prefix and uniqueness
    /// are validated exactly as for `SetIndividualAddress`.
    SetCouplerIndividualAddress {
        device: DeviceId,
        address: IndividualAddress,
        evidence: CouplerEvidence,
    },
    /// Undo-only restore of a pre-existing imported address, including a
    /// mismatched line prefix or a coupler address ending in zero. Never
    /// constructed by an HTTP route: repairs must remain reversible without
    /// accepting a *new* invalid assignment from the editor.
    ///
    /// `redo_coupler` is set when this restore undoes a
    /// `SetCouplerIndividualAddress`, so that redo re-applies the coupler
    /// assignment with its evidence instead of the plain command, which
    /// would refuse the zero.
    RestoreIndividualAddress {
        device: DeviceId,
        address: Option<IndividualAddress>,
        redo_coupler: Option<CouplerEvidence>,
    },
    /// Sets a device's description as a user edit. Unlike
    /// `ComObjectInstance::description`, `DeviceInstance::description` is a
    /// bare `Option<String>`, not an `Override<Text>` — no provenance layer
    /// exists to preserve, so this doubles as its own undo/redo form (like
    /// `SetIndividualAddress`).
    SetDeviceDescription {
        device: DeviceId,
        description: Option<String>,
    },
    /// Sets a communication object instance's datapoint type as a user
    /// edit. Always resolves to `Layer::UserEdit` — use `RestoreComObjectDpt`
    /// to put back an exact prior `Resolved<DptRef>` (that is what undo
    /// does; this variant is not it).
    SetComObjectDpt {
        com_object: ComObjectInstanceId,
        dpt: Option<DptRef>,
    },
    /// The undo/redo form of `SetComObjectDpt`: restores an exact resolved
    /// value, layer included. Not constructed directly by UI code — a
    /// `Layer::Program` value undone back into place must stay
    /// `Layer::Program`, not become `Layer::UserEdit` again, so this cannot
    /// share `SetComObjectDpt`'s bare-`DptRef` shape.
    RestoreComObjectDpt {
        com_object: ComObjectInstanceId,
        dpt: Override<DptRef>,
    },
    /// Sets a communication object instance's description as a user edit.
    /// Always resolves to `Layer::UserEdit` — use
    /// `RestoreComObjectDescription` to put back an exact prior
    /// `Resolved<Text>` (that is what undo does; this variant is not it).
    SetComObjectDescription {
        com_object: ComObjectInstanceId,
        description: Option<String>,
    },
    /// The undo/redo form of `SetComObjectDescription` — see
    /// `RestoreComObjectDpt` for why this cannot share the bare-`String`
    /// shape.
    RestoreComObjectDescription {
        com_object: ComObjectInstanceId,
        description: Override<Text>,
    },
    /// Sets one of a communication object instance's six flags as a user
    /// edit. Always resolves to `Layer::UserEdit` — use
    /// `RestoreComObjectFlag` to put back an exact prior `Override<bool>`
    /// (what undo does). Unlike `SetComObjectDpt`/`SetComObjectDescription`,
    /// a flag checkbox has only two states, so `value` is a bare `bool`,
    /// never `Option<bool>` — there is no "clear the override" gesture here.
    SetComObjectFlag {
        com_object: ComObjectInstanceId,
        flag: ComFlagKind,
        value: bool,
    },
    /// The undo/redo form of `SetComObjectFlag` — see `RestoreComObjectDpt`
    /// for why this cannot share the bare-`bool` shape.
    RestoreComObjectFlag {
        com_object: ComObjectInstanceId,
        flag: ComFlagKind,
        value: Override<bool>,
    },
    /// Writes one `ParameterInstance`'s raw value as a user edit —
    /// overwriting `raw` in place if `(device, ets_id)` already names an
    /// entry in `installations[0].parameters`, else inserting a new one.
    /// `ets_id` is `ParameterInstance::source::ets_id` (`parameter.rs`'s own
    /// doc comment), matched verbatim; `knx-core` has no way to check it
    /// names something a program actually declares — that validation lives
    /// in `knx-server`, which alone can reach `knx-productdb`'s tables (see
    /// the parameter-editor design, D24). `id` is used only for the
    /// insert case — the existing row's own id is kept on an overwrite, the
    /// same "this command does not allocate" split `CreateDevice` already
    /// documents; the caller looks up an existing id or allocates a fresh
    /// one via `Project::ids::next_parameter_instance_id` before
    /// constructing this command.
    SetParameterValue {
        id: ParameterInstanceId,
        device: DeviceId,
        ets_id: String,
        raw: String,
    },
    /// The undo/redo form of `SetParameterValue` — see
    /// `RestoreComObjectFlag` for why the two cannot share one shape, with
    /// one difference from that pair: a `ParameterInstance` row can itself
    /// be created or deleted by these two commands (unlike a com object
    /// instance, which `SetComObjectFlag` only ever toggles a flag on), so
    /// a bare `String` cannot say "no row existed before this `Set`" the
    /// way `Override<bool>` says "no override existed" for a flag. Nothing
    /// here reuses `provenance::Override<T>` either: that type pairs a
    /// value with a `Resolved`/`Layer` provenance chain, and
    /// `ParameterInstance` carries none (`parameter.rs`'s own doc comment:
    /// "retained but uninterpreted") — the same reasoning
    /// `SetDeviceDescription` already gives for its own bare
    /// `Option<String>`. `raw: None` means undoing a creation, so `apply`
    /// deletes the row outright rather than leaving one behind with an
    /// empty string; `raw: Some(prior)` means undoing an overwrite, so
    /// `apply` restores `prior` onto the existing row.
    RestoreParameterValue {
        id: ParameterInstanceId,
        device: DeviceId,
        ets_id: String,
        raw: Option<String>,
    },
    /// `entry.id` is pre-allocated by the caller via
    /// `Project::ids::next_group_address_id`.
    CreateGroupAddress {
        entry: GroupAddressEntry,
        /// Target installation when `entry.range` is `None`; `None` means
        /// the first installation (legacy default). With a range, the range's
        /// installation is used and a different explicit target is refused.
        installation: Option<InstallationId>,
    },
    DeleteGroupAddress {
        id: GroupAddressId,
    },
    /// Internal inverse of [`Command::DeleteGroupAddress`]. Restores the
    /// exact list position so undo/redo cannot reorder CSV exports.
    RestoreGroupAddress {
        entry: GroupAddressEntry,
        position: usize,
        installation: InstallationId,
    },
    /// Overwrites `name`, `central`, and `unfiltered` on an existing group
    /// address — `address` and `range` are untouched, matching
    /// `MoveDeviceToLine`'s split of "which value" from "where it lives".
    /// No name validation happens here; the CSV reader that is this
    /// command's only caller so far validates before ever constructing it
    /// (see the CSV exchange design, §5).
    /// The inverse carries the entry's previous three values.
    UpdateGroupAddress {
        id: GroupAddressId,
        name: String,
        central: bool,
        unfiltered: bool,
    },
    /// Changes an existing group address's numeric address while preserving
    /// its stable id, and therefore every communication-object link that
    /// targets that id. `range` is selected explicitly by the caller for the
    /// new address and validated before mutation. The inverse carries the
    /// previous address and range.
    ReaddressGroupAddress {
        id: GroupAddressId,
        address: GroupAddress,
        range: Option<GroupRangeId>,
    },
    /// `area.id` is pre-allocated by the caller via
    /// `Project::ids::next_area_id`.
    CreateArea {
        area: Area,
        /// `None` means the first installation (legacy default).
        installation: Option<InstallationId>,
    },
    DeleteArea {
        id: AreaId,
    },
    /// Undo-only restore preserving the original area-list position.
    RestoreArea {
        area: Area,
        position: usize,
        installation: InstallationId,
    },
    RenameArea {
        id: AreaId,
        name: String,
    },
    /// `line.id` is pre-allocated by the caller via
    /// `Project::ids::next_line_id`. `area` names the owning area, which
    /// must already exist.
    CreateLine {
        area: AreaId,
        line: Line,
    },
    DeleteLine {
        id: LineId,
    },
    /// Undo-only restore preserving both the flat line order and area child order.
    RestoreLine {
        area: AreaId,
        line: Line,
        line_position: usize,
        area_position: usize,
        installation: InstallationId,
    },
    RenameLine {
        id: LineId,
        name: String,
    },
    /// Moves a line between areas without rewriting its device addresses.
    /// Addressed devices must already match the destination's prefix.
    MoveLineToArea {
        id: LineId,
        area: AreaId,
    },
    /// Internal inverse restoring the exact area child-list position. An
    /// imported invalid original placement must remain undoable.
    RestoreLinePlacement {
        id: LineId,
        area: Option<AreaId>,
        /// Unused when restoring an orphan line (no owning area).
        position: usize,
    },
    /// Moves a device to `line`, or to `Topology::unassigned` if `None`.
    /// Does not touch `DeviceInstance::address` — a line move and a
    /// re-address are two separate user intents; `SetIndividualAddress`
    /// is the command for the latter.
    MoveDeviceToLine {
        device: DeviceId,
        line: Option<LineId>,
    },
    /// Replaces the project-id allocator snapshot absolutely. Kept as an
    /// exact-restore form only (ADR-0039 Decision 2): a snapshot taken
    /// before another edit *lowers* the high-water mark when applied, so no
    /// production path should emit it — use `ReserveIds`.
    SetIdAllocators {
        ids: IdAllocators,
    },
    /// Raises each allocator counter to `max(current, through)` and never
    /// lowers one (ADR-0039 Decision 2). Its inverse is itself, so undo
    /// leaves the high-water mark where it was and an id is never issued
    /// twice in one project's history. A caller that allocated from a clone
    /// of `project.ids` submits `Batch([ReserveIds { through: clone }, …])`.
    ReserveIds {
        through: IdAllocators,
    },
    /// Creates a device with its communication-object instances already
    /// attached, placed in `line` or, if `None`, `Topology::unassigned` —
    /// mirrors `MoveDeviceToLine`'s own placement rule, since a device is
    /// placed in a line xor left unassigned the same way in both commands.
    /// `device.id` and every entry in `com_objects` carry ids
    /// pre-allocated by the caller via `Project::ids::next_device_id`/
    /// `next_com_object_instance_id`; `device.com_objects` already lists
    /// their ids, so no separate id list is threaded through twice.
    /// `program_defaults` carries the side-table entries (ADR-0027) that
    /// belong to those communication objects — empty for a genuine
    /// creation, populated when this command is `DeleteDevice`'s inverse,
    /// because `Devices::program_defaults` lives beside `ComObjectInstance`
    /// rather than inside it and would otherwise not survive an undo.
    CreateDevice {
        device: DeviceInstance,
        com_objects: Vec<ComObjectInstance>,
        program_defaults: Vec<(ComObjectInstanceId, ProgramDefaults)>,
        /// Target installation. `None` preserves the legacy first-installation
        /// behavior for existing callers.
        installation: Option<InstallationId>,
        line: Option<LineId>,
    },
    /// Refuses (`CommandError::DeviceHasLinks`) if any of the device's
    /// communication objects still links to a group address — the
    /// device equivalent of `DeleteGroupAddress`'s `GroupAddressInUse`
    /// check.
    DeleteDevice {
        id: DeviceId,
    },
    /// Internal inverse of [`Command::DeleteDevice`], retaining the exact
    /// topology position so undo restores byte-for-byte ordering.
    RestoreDevice {
        device: DeviceInstance,
        com_objects: Vec<ComObjectInstance>,
        program_defaults: Vec<(ComObjectInstanceId, ProgramDefaults)>,
        installation: InstallationId,
        line: Option<LineId>,
        position: usize,
    },
    /// `part.id` is pre-allocated by the caller via
    /// `Project::ids::next_building_part_id`. `part.parent` names the
    /// owning building part, which must already exist — `None` creates a
    /// root part. `installation.buildings` is a flat list (DATA_MODEL
    /// §5, `building.rs`'s own doc comment); this just links `part.id`
    /// into its parent's `children`, same as `CreateGroupRange`.
    CreateBuildingPart {
        part: BuildingPart,
        /// Target installation for a root part; `None` means the first
        /// installation. A part with a parent goes to the parent's
        /// installation, and a different explicit target is refused.
        installation: Option<InstallationId>,
    },
    /// Refuses (`CommandError::BuildingPartNotEmpty`) if the part still
    /// has children or devices — the building-part equivalent of
    /// `DeleteGroupRange`'s `GroupRangeNotEmpty`/`LineNotEmpty`'s single
    /// non-empty check, just covering both at once since either leaves
    /// something dangling.
    DeleteBuildingPart {
        id: BuildingPartId,
    },
    /// Undo-only restore retaining the flat row and parent's child position.
    RestoreBuildingPart {
        part: BuildingPart,
        position: usize,
        child_position: Option<usize>,
        installation: InstallationId,
    },
    RenameBuildingPart {
        id: BuildingPartId,
        name: String,
    },
    /// Reparents a part without changing its identity or its referenced devices.
    /// `None` makes it a root building part; the target's child list appends it.
    MoveBuildingPart {
        id: BuildingPartId,
        parent: Option<BuildingPartId>,
    },
    /// Internal undo/redo form, retaining the original sibling position.
    RestoreBuildingPartPlacement {
        id: BuildingPartId,
        parent: Option<BuildingPartId>,
        /// Unused for a root part, whose order is the flat `buildings` order.
        position: usize,
    },
    /// Moves a device into `part`, or out of any building part entirely
    /// if `None` — independent of `MoveDeviceToLine`'s topology
    /// placement, the same way `building.rs`'s own doc comment
    /// describes a `BuildingPart` as referencing a device, not owning
    /// it. Unlike `MoveDeviceToLine`, `None` is not itself a tracked
    /// location (there is no building-side "unassigned" bucket) — it
    /// just means the device is not currently placed in any part.
    MoveDeviceToBuildingPart {
        device: DeviceId,
        part: Option<BuildingPartId>,
    },
    /// Undo-only inverse of [`Command::MoveDeviceToBuildingPart`]: puts the
    /// device back into its previous part without the cross-installation
    /// check, so an imported placement in another installation stays
    /// undoable. Its own inverse is the checked forward move.
    RestoreDeviceBuildingPlacement {
        device: DeviceId,
        part: Option<BuildingPartId>,
    },
    /// `range.id` is pre-allocated by the caller via
    /// `Project::ids::next_group_range_id`.
    CreateGroupRange {
        range: GroupRange,
        /// Target installation for a main range; `None` means the first
        /// installation. A middle range goes to its parent's installation,
        /// and a different explicit target is refused.
        installation: Option<InstallationId>,
    },
    DeleteGroupRange {
        id: GroupRangeId,
    },
    /// Undo-only restore retaining the flat row and parent's child position.
    RestoreGroupRange {
        range: GroupRange,
        position: usize,
        child_position: Option<usize>,
        installation: InstallationId,
    },
    RenameGroupRange {
        id: GroupRangeId,
        name: String,
    },
    /// Reparents a range while retaining its span, identity and group addresses.
    MoveGroupRange {
        id: GroupRangeId,
        parent: Option<GroupRangeId>,
    },
    /// Internal inverse restoring the original child-list position. Undo may
    /// recover an imported out-of-bounds or cyclic original hierarchy.
    RestoreGroupRangePlacement {
        id: GroupRangeId,
        parent: Option<GroupRangeId>,
        /// Unused for a root range; flat `group_ranges` order is unchanged.
        position: usize,
    },
    /// Adds a directional link from a communication object instance to a
    /// group address. `direction` distinguishes a send link from a
    /// receive link — a comm object may hold both for the same `ga` as
    /// two distinct `GroupLink`s (DATA_MODEL §6).
    LinkComObject {
        com_object: ComObjectInstanceId,
        ga: GroupAddressId,
        direction: Direction,
    },
    UnlinkComObject {
        com_object: ComObjectInstanceId,
        ga: GroupAddressId,
        direction: Direction,
    },
    /// Undo-only inverse of an unlink. Preserve the original list position:
    /// appending during rollback could silently change primary link order.
    /// Like other restore commands, it also preserves imported duplicate or
    /// dangling links rather than inventing data during a repair.
    RestoreGroupLink {
        com_object: ComObjectInstanceId,
        link: GroupLink,
        position: usize,
    },
    /// Changes the project-wide `GroupAddressStyle` — refused
    /// (`CommandError::GroupAddressDoesNotFitStyle`) if any existing group
    /// address, in any installation, would not fit `style`
    /// (`GroupAddress::fits_style`; see [KNOWN_LIMITATIONS.md
    /// §84](../../../docs/KNOWN_LIMITATIONS.md)), and likewise
    /// (`CommandError::GroupRangeDoesNotFitStyle`) if any `GroupRange`'s
    /// `start` or `end` would not fit — both checks complete for every
    /// installation before anything mutates. Self-inverting like
    /// `SetIndividualAddress`/`SetDeviceDescription` — its own inverse
    /// carries the style it replaced.
    SetGroupAddressStyle {
        style: GroupAddressStyle,
    },
    /// MODEL-02 repair: `device` occurs in several topology slots (imported
    /// ambiguity). Keeps the first occurrence in `keep` — which must be a
    /// slot the device occupies now — and removes every other occurrence in
    /// every installation. Addresses, links and building placement are
    /// untouched. Refused when the device is placed at most once (a repair
    /// is not a move).
    RepairDevicePlacement {
        device: DeviceId,
        keep: DevicePlacementSlot,
    },
    /// Undo-only inverse of [`Command::RepairDevicePlacement`]: reinserts
    /// `removed` in reverse removal order.
    RestoreDevicePlacements {
        device: DeviceId,
        keep: DevicePlacementSlot,
        removed: Vec<RemovedDevicePlacement>,
    },
    /// MODEL-02 repair: `line` is listed by several area entries (two areas,
    /// twice in one area, or by an area of another installation). Keeps the
    /// first reference in `keep`, which must list the line now and belong to
    /// the line's installation; removes all other references. Line and device
    /// addresses are untouched. Refused when the line has a single owner.
    RepairLineOwner {
        line: LineId,
        keep: AreaId,
    },
    /// Undo-only inverse of [`Command::RepairLineOwner`].
    RestoreLineOwners {
        line: LineId,
        keep: AreaId,
        removed: Vec<RemovedLineReference>,
    },
    /// Renames an installation (MODEL-01). Self-inverting: the inverse
    /// carries the previous name.
    RenameInstallation {
        id: InstallationId,
        name: String,
    },
    /// Applies every sub-command as one atomic, one-undo-step unit — see
    /// `docs/superpowers/specs/2026-09-10-bulk-operations-design.md` for the
    /// rollback rationale. On any sub-command's `Err`, every already-applied
    /// sub-command is rolled back (its inverse re-applied, in reverse order)
    /// before the original error is returned, so `apply`'s
    /// leave-`project`-untouched-on-`Err` contract holds for `Batch` too.
    Batch(Vec<Command>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandError {
    Validation(ValidationError),
    DeviceNotFound(DeviceId),
    /// Coupler evidence names a different product than the device now has.
    CouplerEvidenceMismatch {
        device: DeviceId,
        evidence_product_ref: String,
    },
    ComObjectNotFound(ComObjectInstanceId),
    GroupAddressNotFound(GroupAddressId),
    /// A group address id occurs in several installations.
    GroupAddressPlacementAmbiguous(GroupAddressId),
    /// Parameter rows for one device and `ets_id` exist in several
    /// installations; no single row can be edited safely.
    ParameterPlacementAmbiguous(DeviceId),
    /// A move, link or create would connect two installations, which are
    /// separate infrastructures (ADR-0038).
    CrossInstallation {
        from: InstallationId,
        to: InstallationId,
    },
    /// A topology repair was requested for something that is not ambiguous;
    /// use the ordinary edit command instead.
    RepairNotNeeded,
    /// The placement chosen to keep is not one the entity occupies now; a
    /// repair only removes, it never adds.
    RepairKeepNotPresent,
    /// A `DeleteGroupAddress` was refused because at least one
    /// communication object still links to it — deleting it now would
    /// leave a dangling `GroupLink` (`ValidationError::DanglingGroupLink`
    /// exists for the reverse direction: a link created against a group
    /// address that is already gone).
    GroupAddressInUse(GroupAddressId),
    AreaNotFound(AreaId),
    /// Numeric ID does not identify one area across the project.
    AreaPlacementAmbiguous(AreaId),
    /// A `DeleteArea` was refused because it still owns at least one line.
    AreaNotEmpty(AreaId),
    LineNotFound(LineId),
    InvalidTopologyPosition {
        device: DeviceId,
        position: usize,
    },
    /// A `DeleteLine` was refused because it still owns at least one
    /// device.
    LineNotEmpty(LineId),
    /// Source/target topology has duplicate line ids or multiple owners.
    LinePlacementAmbiguous(LineId),
    /// An internal undo position exceeds the target area's line list.
    InvalidLinePosition {
        area: AreaId,
        position: usize,
    },
    /// Undo-only flat-list position exceeds the number of remaining rows.
    InvalidStructurePosition {
        kind: IdKind,
        id: u32,
        position: usize,
    },
    /// A `DeleteDevice` was refused because at least one of the device's
    /// communication object instances still links to a group address —
    /// deleting it now would leave a dangling `GroupLink`, the device
    /// equivalent of `GroupAddressInUse`.
    DeviceHasLinks(DeviceId),
    /// A device is still referenced by a building placement, parameter, or
    /// module instance that `DeleteDevice` cannot safely discard implicitly.
    DeviceHasDependentData(DeviceId),
    BuildingPartNotFound(BuildingPartId),
    /// A `DeleteBuildingPart` was refused because it still has a child
    /// part or a device located in it.
    BuildingPartNotEmpty(BuildingPartId),
    /// Reparenting a part under itself or one of its descendants would cycle.
    BuildingPartCycle {
        id: BuildingPartId,
        parent: BuildingPartId,
    },
    /// Imported parent and children references disagree or name multiple owners.
    BuildingPartPlacementAmbiguous(BuildingPartId),
    /// An internal undo position exceeds the target parent's child list.
    InvalidBuildingPartPosition {
        parent: BuildingPartId,
        position: usize,
    },
    GroupRangeNotFound(GroupRangeId),
    /// A `DeleteGroupRange` was refused because it still has nested
    /// (middle) ranges.
    GroupRangeNotEmpty(GroupRangeId),
    /// A `DeleteGroupRange` was refused because at least one group
    /// address still names it as its `range`.
    GroupRangeInUse(GroupRangeId),
    /// Reparenting under self or a descendant would create a cycle.
    GroupRangeCycle {
        id: GroupRangeId,
        parent: GroupRangeId,
    },
    /// The stored parent field and children lists disagree or duplicate ownership.
    GroupRangePlacementAmbiguous(GroupRangeId),
    /// An internal undo position exceeds the target range's child list.
    InvalidGroupRangePosition {
        parent: GroupRangeId,
        position: usize,
    },
    LinkAlreadyExists {
        com_object: ComObjectInstanceId,
        ga: GroupAddressId,
        direction: Direction,
    },
    LinkNotFound {
        com_object: ComObjectInstanceId,
        ga: GroupAddressId,
        direction: Direction,
    },
    InvalidGroupLinkPosition {
        com_object: ComObjectInstanceId,
        position: usize,
    },
    /// A `SetGroupAddressStyle` was refused because `id`'s raw value does
    /// not survive a round trip through `style`'s own `format`/`parse` pair
    /// (`GroupAddress::fits_style`) — named concretely so the refusal is
    /// actionable, not "some address doesn't fit".
    GroupAddressDoesNotFitStyle {
        id: GroupAddressId,
        raw: u16,
        style: GroupAddressStyle,
    },
    /// A `SetGroupAddressStyle` was refused because a `GroupRange`'s
    /// `start` or `end` — itself a `GroupAddress`, per `GroupRange`'s own
    /// definition — does not fit `style`. Same class of problem as
    /// `GroupAddressDoesNotFitStyle`, kept as a sibling variant naming the
    /// range rather than stretched over the entry's id type; `raw` alone
    /// is enough to find which of `start`/`end` failed by hand.
    GroupRangeDoesNotFitStyle {
        id: GroupRangeId,
        raw: u16,
        style: GroupAddressStyle,
    },
    InstallationNotFound,
    /// A command tried to insert an entity under an id another entity of
    /// the same kind already holds (ADR-0039 Decision 3). Refused so a stale
    /// allocator snapshot becomes a typed error instead of a duplicate that
    /// `save_project`'s upsert would later collapse, losing one entity.
    IdInUse {
        kind: IdKind,
        id: u32,
    },
    /// A child of an atomic batch failed. The zero-based index includes
    /// internal commands such as `ReserveIds`; callers can map it to their
    /// own item numbering without losing the typed underlying error.
    BatchItem {
        index: usize,
        source: Box<CommandError>,
    },
    NothingToUndo,
    NothingToRedo,
}

/// Which id space an `CommandError::IdInUse` refers to — one per synthetic
/// id newtype that a command can insert.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdKind {
    Device,
    Area,
    Line,
    ComObjectInstance,
    GroupRange,
    GroupAddress,
    BuildingPart,
    ParameterInstance,
}

impl fmt::Display for IdKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            IdKind::Device => "device",
            IdKind::Area => "area",
            IdKind::Line => "line",
            IdKind::ComObjectInstance => "communication object instance",
            IdKind::GroupRange => "group range",
            IdKind::GroupAddress => "group address",
            IdKind::BuildingPart => "building part",
            IdKind::ParameterInstance => "parameter instance",
        })
    }
}

impl std::error::Error for CommandError {}

impl fmt::Display for CommandError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CommandError::Validation(e) => write!(f, "{e}"),
            CommandError::DeviceNotFound(id) => write!(f, "device {id} not found"),
            CommandError::CouplerEvidenceMismatch {
                device,
                evidence_product_ref,
            } => write!(
                f,
                "coupler evidence for product {evidence_product_ref} does not describe device {device}'s product"
            ),
            CommandError::ComObjectNotFound(id) => {
                write!(f, "communication object instance {id} not found")
            }
            CommandError::GroupAddressNotFound(id) => write!(f, "group address {id} not found"),
            CommandError::GroupAddressPlacementAmbiguous(id) => write!(
                f,
                "group address {id} exists in several installations; repair the project before editing it"
            ),
            CommandError::ParameterPlacementAmbiguous(device) => write!(
                f,
                "parameters of device {device} exist in several installations; repair the project before editing them"
            ),
            CommandError::RepairNotNeeded => write!(
                f,
                "nothing to repair: the placement is not ambiguous; use the ordinary edit"
            ),
            CommandError::RepairKeepNotPresent => write!(
                f,
                "the placement to keep is not one of the current placements; a repair only removes duplicates"
            ),
            CommandError::CrossInstallation { from, to } => write!(
                f,
                "installation {from} and installation {to} are separate infrastructures; this would connect them"
            ),
            CommandError::GroupAddressInUse(id) => {
                write!(
                    f,
                    "group address {id} is still linked from a communication object"
                )
            }
            CommandError::AreaNotFound(id) => write!(f, "area {id} not found"),
            CommandError::AreaPlacementAmbiguous(id) => write!(f, "area {id} has duplicate identities; select an unambiguous area before editing it"),
            CommandError::AreaNotEmpty(id) => {
                write!(f, "area {id} still has lines, cannot delete")
            }
            CommandError::LineNotFound(id) => write!(f, "line {id} not found"),
            CommandError::InvalidTopologyPosition { device, position } => write!(
                f,
                "topology position {position} is invalid while restoring device {device}"
            ),
            CommandError::LineNotEmpty(id) => {
                write!(f, "line {id} still has devices, cannot delete")
            }
            CommandError::LinePlacementAmbiguous(id) => {
                write!(f, "line {id} has ambiguous ownership or duplicate references; repair the topology before editing it")
            }
            CommandError::InvalidLinePosition { area, position } => {
                write!(f, "line position {position} is invalid for area {area}")
            }
            CommandError::InvalidStructurePosition { kind, id, position } => {
                write!(f, "position {position} is invalid while restoring {kind} {id}")
            }
            CommandError::DeviceHasLinks(id) => {
                write!(f, "device {id} still has linked communication objects, cannot delete")
            }
            CommandError::DeviceHasDependentData(id) => write!(
                f,
                "device {id} still has building, parameter, or module data, cannot delete"
            ),
            CommandError::BuildingPartNotFound(id) => write!(f, "building part {id} not found"),
            CommandError::BuildingPartNotEmpty(id) => {
                write!(f, "building part {id} still has children or devices, cannot delete")
            }
            CommandError::BuildingPartCycle { id, parent } => {
                write!(f, "building part {id} cannot move under {parent}: that would create a cycle")
            }
            CommandError::BuildingPartPlacementAmbiguous(id) => {
                write!(f, "building part {id} has inconsistent parent/child references; repair the hierarchy before editing it")
            }
            CommandError::InvalidBuildingPartPosition { parent, position } => {
                write!(f, "child position {position} is invalid for building part {parent}")
            }
            CommandError::GroupRangeNotFound(id) => write!(f, "group range {id} not found"),
            CommandError::GroupRangeNotEmpty(id) => {
                write!(f, "group range {id} still has nested ranges, cannot delete")
            }
            CommandError::GroupRangeInUse(id) => write!(
                f,
                "group range {id} still has group addresses assigned to it"
            ),
            CommandError::GroupRangeCycle { id, parent } => {
                write!(f, "group range {id} cannot move under {parent}: that would create a cycle")
            }
            CommandError::GroupRangePlacementAmbiguous(id) => {
                write!(f, "group range {id} has inconsistent parent/child references; repair the hierarchy before editing it")
            }
            CommandError::InvalidGroupRangePosition { parent, position } => {
                write!(f, "child position {position} is invalid for group range {parent}")
            }
            CommandError::LinkAlreadyExists {
                com_object,
                ga,
                direction,
            } => write!(
                f,
                "communication object {com_object} already links to group address {ga} ({direction:?})"
            ),
            CommandError::LinkNotFound {
                com_object,
                ga,
                direction,
            } => write!(
                f,
                "communication object {com_object} has no {direction:?} link to group address {ga}"
            ),
            CommandError::InvalidGroupLinkPosition {
                com_object,
                position,
            } => write!(
                f,
                "group link position {position} is invalid while restoring communication object {com_object}"
            ),
            CommandError::GroupAddressDoesNotFitStyle { id, raw, style } => write!(
                f,
                "group address {id} (raw value {raw}) does not fit style {style:?}, refusing the whole restyle"
            ),
            CommandError::GroupRangeDoesNotFitStyle { id, raw, style } => write!(
                f,
                "group range {id} (boundary raw value {raw}) does not fit style {style:?}, refusing the whole restyle"
            ),
            CommandError::InstallationNotFound => write!(f, "project has no installation"),
            CommandError::IdInUse { kind, id } => write!(
                f,
                "{kind} id {id} is already in use; refusing to create a second entity under it"
            ),
            CommandError::BatchItem { index, source } => {
                write!(f, "batch command {}: {source}", index + 1)
            }
            CommandError::NothingToUndo => write!(f, "nothing to undo"),
            CommandError::NothingToRedo => write!(f, "nothing to redo"),
        }
    }
}

impl From<ValidationError> for CommandError {
    fn from(e: ValidationError) -> Self {
        CommandError::Validation(e)
    }
}

/// Refuses `id` if an entity of `kind` already holds it anywhere in
/// `project` (ADR-0039 Decision 3). Ids are project-unique, not
/// installation-unique, so every installation is searched. The inverse forms
/// that re-insert an id their own forward command just freed pass this
/// check naturally, because the id is no longer present.
fn check_id_free(project: &Project, kind: IdKind, id: u32) -> Result<(), CommandError> {
    let installations = project.installations.iter();
    let taken = match kind {
        IdKind::Device => project.devices.get(DeviceId(id)).is_some(),
        IdKind::ComObjectInstance => project
            .devices
            .com_object(ComObjectInstanceId(id))
            .is_some(),
        IdKind::Area => installations
            .flat_map(|i| &i.topology.areas)
            .any(|a| a.id == AreaId(id)),
        IdKind::Line => installations
            .flat_map(|i| &i.topology.lines)
            .any(|l| l.id == LineId(id)),
        IdKind::GroupRange => installations
            .flat_map(|i| &i.group_ranges)
            .any(|r| r.id == GroupRangeId(id)),
        IdKind::GroupAddress => installations
            .flat_map(|i| &i.group_addresses)
            .any(|g| g.id == GroupAddressId(id)),
        IdKind::BuildingPart => installations
            .flat_map(|i| &i.buildings)
            .any(|b| b.id == BuildingPartId(id)),
        IdKind::ParameterInstance => installations
            .flat_map(|i| &i.parameters)
            .any(|p| p.id == ParameterInstanceId(id)),
    };
    if taken {
        Err(CommandError::IdInUse { kind, id })
    } else {
        Ok(())
    }
}

/// MODEL-02: removes every occurrence of `device` except the first one in
/// `keep`. Validates everything before the first removal so a refusal leaves
/// the project untouched. Returns the removals in order.
fn repair_device_placement(
    project: &mut Project,
    device: DeviceId,
    keep: DevicePlacementSlot,
) -> Result<Vec<RemovedDevicePlacement>, CommandError> {
    if project.devices.get(device).is_none() {
        return Err(CommandError::DeviceNotFound(device));
    }
    let keep_index = match keep {
        DevicePlacementSlot::Line(line) => require_unique_line(project, line)?,
        DevicePlacementSlot::Unassigned(id) => target_installation(project, Some(id))?,
    };
    let keep_present = match keep {
        DevicePlacementSlot::Line(line) => project.installations[keep_index]
            .topology
            .lines
            .iter()
            .any(|l| l.id == line && l.devices.contains(&device)),
        DevicePlacementSlot::Unassigned(_) => project.installations[keep_index]
            .topology
            .unassigned
            .contains(&device),
    };
    let occurrences: usize = project
        .installations
        .iter()
        .map(|installation| {
            installation
                .topology
                .unassigned
                .iter()
                .chain(installation.topology.lines.iter().flat_map(|l| &l.devices))
                .filter(|&&d| d == device)
                .count()
        })
        .sum();
    if occurrences < 2 {
        return Err(CommandError::RepairNotNeeded);
    }
    if !keep_present {
        return Err(CommandError::RepairKeepNotPresent);
    }
    let mut removed = Vec::new();
    for (index, installation) in project.installations.iter_mut().enumerate() {
        let id = installation.id;
        let keeps_unassigned =
            index == keep_index && matches!(keep, DevicePlacementSlot::Unassigned(_));
        remove_occurrences(
            &mut installation.topology.unassigned,
            device,
            keeps_unassigned,
            |position| RemovedDevicePlacement {
                installation: id,
                line: None,
                position,
            },
            &mut removed,
        );
        for line in &mut installation.topology.lines {
            let keeps_line = index == keep_index && keep == DevicePlacementSlot::Line(line.id);
            let line_id = line.id;
            remove_occurrences(
                &mut line.devices,
                device,
                keeps_line,
                |position| RemovedDevicePlacement {
                    installation: id,
                    line: Some(line_id),
                    position,
                },
                &mut removed,
            );
        }
    }
    Ok(removed)
}

/// Removes every `item` from `list`, sparing the first one when
/// `keep_first`; records each removal (index at removal time) via `record`.
fn remove_occurrences<T: PartialEq + Copy, R>(
    list: &mut Vec<T>,
    item: T,
    keep_first: bool,
    record: impl Fn(usize) -> R,
    removed: &mut Vec<R>,
) {
    let mut spared = !keep_first;
    let mut position = 0;
    while position < list.len() {
        if list[position] == item {
            if spared {
                list.remove(position);
                removed.push(record(position));
                continue;
            }
            spared = true;
        }
        position += 1;
    }
}

/// Undo of [`repair_device_placement`]: reinserts in reverse removal order.
/// Validates every container and index first.
fn restore_device_placements(
    project: &mut Project,
    device: DeviceId,
    removed: &[RemovedDevicePlacement],
) -> Result<(), CommandError> {
    let mut targets = Vec::with_capacity(removed.len());
    for entry in removed {
        let index = restore_installation(project, entry.installation)?;
        let line = match entry.line {
            Some(line_id) => Some(
                project.installations[index]
                    .topology
                    .lines
                    .iter()
                    .position(|l| l.id == line_id)
                    .ok_or(CommandError::LineNotFound(line_id))?,
            ),
            None => None,
        };
        targets.push((index, line, entry.position));
    }
    for &(index, line, position) in targets.iter().rev() {
        let topology = &mut project.installations[index].topology;
        let list = match line {
            Some(line) => &mut topology.lines[line].devices,
            None => &mut topology.unassigned,
        };
        list.insert(position.min(list.len()), device);
    }
    Ok(())
}

/// MODEL-02: removes every area reference to `line` except the first one in
/// `keep`. All checks run before the first removal.
fn repair_line_owner(
    project: &mut Project,
    line: LineId,
    keep: AreaId,
) -> Result<Vec<RemovedLineReference>, CommandError> {
    let line_index = require_unique_line(project, line)?;
    let area_index = require_unique_area(project, keep)?;
    let references: usize = project
        .installations
        .iter()
        .flat_map(|installation| &installation.topology.areas)
        .map(|area| area.lines.iter().filter(|&&l| l == line).count())
        .sum();
    if references < 2 {
        return Err(CommandError::RepairNotNeeded);
    }
    same_installation(project, line_index, area_index)?;
    if !project.installations[area_index]
        .topology
        .areas
        .iter()
        .any(|area| area.id == keep && area.lines.contains(&line))
    {
        return Err(CommandError::RepairKeepNotPresent);
    }
    let mut removed = Vec::new();
    for installation in &mut project.installations {
        let id = installation.id;
        for area in &mut installation.topology.areas {
            let area_id = area.id;
            remove_occurrences(
                &mut area.lines,
                line,
                area_id == keep,
                |position| RemovedLineReference {
                    installation: id,
                    area: area_id,
                    position,
                },
                &mut removed,
            );
        }
    }
    Ok(removed)
}

/// Undo of [`repair_line_owner`]: reinserts in reverse removal order.
fn restore_line_owners(
    project: &mut Project,
    line: LineId,
    removed: &[RemovedLineReference],
) -> Result<(), CommandError> {
    let mut targets = Vec::with_capacity(removed.len());
    for entry in removed {
        let index = restore_installation(project, entry.installation)?;
        let area = project.installations[index]
            .topology
            .areas
            .iter()
            .position(|a| a.id == entry.area)
            .ok_or(CommandError::AreaNotFound(entry.area))?;
        targets.push((index, area, entry.position));
    }
    for &(index, area, position) in targets.iter().rev() {
        let lines = &mut project.installations[index].topology.areas[area].lines;
        lines.insert(position.min(lines.len()), line);
    }
    Ok(())
}

/// Index of installation `id`, or of the first installation for `None` —
/// the legacy target of a create that names no parent.
fn target_installation(
    project: &Project,
    id: Option<InstallationId>,
) -> Result<usize, CommandError> {
    match id {
        Some(id) => project
            .installations
            .iter()
            .position(|installation| installation.id == id)
            .ok_or(CommandError::InstallationNotFound),
        None if project.installations.is_empty() => Err(CommandError::InstallationNotFound),
        None => Ok(0),
    }
}

/// The one installation that holds an entity. `count` reports how often the
/// entity occurs in one installation; zero in total is `not_found`, more than
/// one occurrence anywhere is `ambiguous` — never the first of several.
fn owning_installation(
    project: &Project,
    count: impl Fn(&Installation) -> usize,
    not_found: CommandError,
    ambiguous: CommandError,
) -> Result<usize, CommandError> {
    let mut owner = None;
    let mut total = 0usize;
    for (index, installation) in project.installations.iter().enumerate() {
        let occurrences = count(installation);
        if occurrences > 0 {
            total += occurrences;
            owner = Some(index);
        }
    }
    match (owner, total) {
        (None, _) => Err(not_found),
        (Some(index), 1) => Ok(index),
        _ => Err(ambiguous),
    }
}

/// Commands addressed only by a numeric id must not select the first of
/// several imported rows. The UI projection may show these ids in several
/// installations; the core still has to refuse a direct HTTP caller.
fn require_unique_area(project: &Project, id: AreaId) -> Result<usize, CommandError> {
    owning_installation(
        project,
        |i| i.topology.areas.iter().filter(|area| area.id == id).count(),
        CommandError::AreaNotFound(id),
        CommandError::AreaPlacementAmbiguous(id),
    )
}

fn require_unique_line(project: &Project, id: LineId) -> Result<usize, CommandError> {
    owning_installation(
        project,
        |i| i.topology.lines.iter().filter(|line| line.id == id).count(),
        CommandError::LineNotFound(id),
        CommandError::LinePlacementAmbiguous(id),
    )
}

fn require_unique_building_part(
    project: &Project,
    id: BuildingPartId,
) -> Result<usize, CommandError> {
    owning_installation(
        project,
        |i| i.buildings.iter().filter(|part| part.id == id).count(),
        CommandError::BuildingPartNotFound(id),
        CommandError::BuildingPartPlacementAmbiguous(id),
    )
}

fn require_unique_group_range(project: &Project, id: GroupRangeId) -> Result<usize, CommandError> {
    owning_installation(
        project,
        |i| i.group_ranges.iter().filter(|range| range.id == id).count(),
        CommandError::GroupRangeNotFound(id),
        CommandError::GroupRangePlacementAmbiguous(id),
    )
}

fn require_unique_group_address(
    project: &Project,
    id: GroupAddressId,
) -> Result<usize, CommandError> {
    owning_installation(
        project,
        |i| {
            i.group_addresses
                .iter()
                .filter(|entry| entry.id == id)
                .count()
        },
        CommandError::GroupAddressNotFound(id),
        CommandError::GroupAddressPlacementAmbiguous(id),
    )
}

/// The installation whose topology places `device` (on a line or
/// unassigned), `None` when no installation places it. Placements in two
/// installations are refused rather than guessed.
fn device_installation(project: &Project, device: DeviceId) -> Result<Option<usize>, CommandError> {
    let mut owner = None;
    for (index, installation) in project.installations.iter().enumerate() {
        let placed = installation.topology.unassigned.contains(&device)
            || installation
                .topology
                .lines
                .iter()
                .any(|line| line.devices.contains(&device));
        if placed && owner.replace(index).is_some() {
            return Err(ValidationError::MultipleTopologyPlacements { device }.into());
        }
    }
    Ok(owner)
}

/// Refuses an operation joining installation `from` with `to`.
fn same_installation(project: &Project, from: usize, to: usize) -> Result<(), CommandError> {
    if from == to {
        Ok(())
    } else {
        Err(CommandError::CrossInstallation {
            from: project.installations[from].id,
            to: project.installations[to].id,
        })
    }
}

/// Where a create lands: the parent's installation if it has a parent,
/// otherwise the explicit target (or the first installation). An explicit
/// target that differs from the parent's installation is refused.
fn create_installation(
    project: &Project,
    parent_owner: Option<usize>,
    explicit: Option<InstallationId>,
) -> Result<usize, CommandError> {
    match (parent_owner, explicit) {
        (Some(owner), Some(id)) => {
            let target = target_installation(project, Some(id))?;
            same_installation(project, owner, target)?;
            Ok(owner)
        }
        (Some(owner), None) => Ok(owner),
        (None, explicit) => target_installation(project, explicit),
    }
}

/// The installation holding the parameter row for `(device, ets_id)`, or,
/// for a new row, the device's own installation (first as a fallback for a
/// device placed nowhere).
fn parameter_installation(
    project: &Project,
    device: DeviceId,
    ets_id: &str,
) -> Result<usize, CommandError> {
    match owning_installation(
        project,
        |i| {
            i.parameters
                .iter()
                .filter(|p| p.device == device && p.source.ets_id == ets_id)
                .count()
        },
        CommandError::DeviceNotFound(device),
        CommandError::ParameterPlacementAmbiguous(device),
    ) {
        Ok(index) => Ok(index),
        Err(CommandError::DeviceNotFound(_)) => match device_installation(project, device)? {
            Some(index) => Ok(index),
            None => target_installation(project, None),
        },
        Err(other) => Err(other),
    }
}

/// Index of installation `id` for an undo-only restore.
fn restore_installation(project: &Project, id: InstallationId) -> Result<usize, CommandError> {
    target_installation(project, Some(id))
}

/// A malformed topology may attach the same line to two areas. Never let
/// `Topology::area_of` silently choose the first for an address write.
fn unique_line_owner(topology: &Topology, line: LineId) -> Result<&Area, CommandError> {
    let mut owners = topology
        .areas
        .iter()
        .filter(|area| area.lines.contains(&line));
    let first = owners
        .next()
        .ok_or(ValidationError::LineWithoutArea { line })?;
    if owners.next().is_some() {
        return Err(ValidationError::LineWithMultipleAreas { line }.into());
    }
    Ok(first)
}

/// Identify a line and its one owning area without choosing arbitrarily from
/// duplicate imported references. Orphan lines have no area but can be moved
/// into a valid one; their inverse must recover that original orphan state.
fn line_placement(
    topology: &Topology,
    id: LineId,
) -> Result<(usize, Option<(AreaId, usize)>), CommandError> {
    let line_index = topology
        .lines
        .iter()
        .position(|line| line.id == id)
        .ok_or(CommandError::LineNotFound(id))?;
    if topology.lines.iter().filter(|line| line.id == id).count() != 1 {
        return Err(CommandError::LinePlacementAmbiguous(id));
    }
    let references: Vec<_> = topology
        .areas
        .iter()
        .flat_map(|area| {
            area.lines
                .iter()
                .enumerate()
                .filter_map(move |(position, &line)| (line == id).then_some((area.id, position)))
        })
        .collect();
    let previous = match references.as_slice() {
        [] => None,
        [(owner, position)] => Some((*owner, *position)),
        _ => return Err(CommandError::LinePlacementAmbiguous(id)),
    };
    if let Some((owner, _)) = previous {
        if topology
            .areas
            .iter()
            .filter(|area| area.id == owner)
            .count()
            != 1
        {
            return Err(CommandError::LinePlacementAmbiguous(id));
        }
    }
    Ok((line_index, previous))
}

/// Look across all installations before editing an individual address or
/// moving a device. A malformed import may place it both on a line and in
/// the unassigned list; choosing just the first placement would turn a move
/// into a duplicate reference and silently guess an address prefix.
fn assigned_line_prefix(
    project: &Project,
    device: DeviceId,
) -> Result<Option<(u8, u8)>, CommandError> {
    let mut assigned = None;
    let mut placements = 0usize;
    for installation in &project.installations {
        placements += installation
            .topology
            .unassigned
            .iter()
            .filter(|&&candidate| candidate == device)
            .count();
        for line in &installation.topology.lines {
            let occurrences = line
                .devices
                .iter()
                .filter(|&&candidate| candidate == device)
                .count();
            if occurrences > 1 {
                return Err(ValidationError::MultipleTopologyPlacements { device }.into());
            }
            if occurrences == 1 {
                let area = unique_line_owner(&installation.topology, line.id)?;
                if assigned.replace((area.address, line.address)).is_some() {
                    return Err(ValidationError::MultipleLineMembership { device }.into());
                }
                placements += 1;
            }
        }
    }
    if placements > 1 {
        return Err(ValidationError::MultipleTopologyPlacements { device }.into());
    }
    Ok(assigned)
}

/// Removes `device` from wherever it currently sits in `installation`'s
/// topology — `unassigned` or a line's `devices` — returning the line it
/// was in, if any. Shared by `MoveDeviceToLine` (which repositions the
/// device elsewhere) and `DeleteDevice` (which needs the same lookup to
/// know what line its own inverse `CreateDevice` should name).
fn remove_device_from_topology(
    installation: &mut Installation,
    device: DeviceId,
) -> Result<(Option<LineId>, usize), CommandError> {
    if let Some(pos) = installation
        .topology
        .unassigned
        .iter()
        .position(|&d| d == device)
    {
        installation.topology.unassigned.remove(pos);
        Ok((None, pos))
    } else if let Some(current_line) = installation
        .topology
        .lines
        .iter_mut()
        .find(|l| l.devices.contains(&device))
    {
        let id = current_line.id;
        let pos = current_line
            .devices
            .iter()
            .position(|&candidate| candidate == device)
            .expect("line was selected because it contains the device");
        current_line.devices.remove(pos);
        Ok((Some(id), pos))
    } else {
        Err(CommandError::DeviceNotFound(device))
    }
}

/// Reparent a line without touching its flat line row or any device address.
/// Forward moves verify the target area/line address prefix; the undo-only
/// restore may recover a pre-existing imported mismatch or duplicate.
fn relocate_line(
    installation: &mut Installation,
    devices: &Devices,
    id: LineId,
    area: Option<AreaId>,
    position: Option<usize>,
    validate_new_area: bool,
) -> Result<(Option<AreaId>, usize), CommandError> {
    let topology = &mut installation.topology;
    let (line_index, previous) = line_placement(topology, id)?;
    let previous_area = previous.map(|(owner, _)| owner);
    let previous_position = previous.map_or(0, |(_, index)| index);
    let destination_index = if let Some(target) = area {
        let index = topology
            .areas
            .iter()
            .position(|candidate| candidate.id == target)
            .ok_or(CommandError::AreaNotFound(target))?;
        if topology
            .areas
            .iter()
            .filter(|candidate| candidate.id == target)
            .count()
            != 1
        {
            return Err(CommandError::LinePlacementAmbiguous(id));
        }
        let mut seen = HashSet::new();
        for &member in &topology.areas[index].lines {
            if !seen.insert(member)
                || topology
                    .lines
                    .iter()
                    .filter(|line| line.id == member)
                    .count()
                    != 1
            {
                return Err(CommandError::LinePlacementAmbiguous(id));
            }
        }
        Some(index)
    } else {
        None
    };
    if let (Some(index), Some(position)) = (destination_index, position) {
        let max_position = topology.areas[index].lines.len() - usize::from(previous_area == area);
        if position > max_position {
            return Err(CommandError::InvalidLinePosition {
                area: area.unwrap(),
                position,
            });
        }
    }
    if previous_area == area && (position.is_none() || position == Some(previous_position)) {
        return Ok((previous_area, previous_position));
    }

    if validate_new_area {
        let index = destination_index.ok_or(CommandError::LinePlacementAmbiguous(id))?;
        let destination = &topology.areas[index];
        let line = &topology.lines[line_index];
        check_no_duplicate_line_address(destination, &topology.lines, id, line.address)?;
        for &device_id in &line.devices {
            let device = devices
                .get(device_id)
                .ok_or(CommandError::DeviceNotFound(device_id))?;
            if let Some(address) = device.address {
                check_individual_address_on_line(
                    device_id,
                    address,
                    destination.address,
                    line.address,
                    true, // unchanged imported .0 addresses are still reversible
                )?;
            }
        }
    }

    if let Some((old_area, old_position)) = previous {
        topology
            .areas
            .iter_mut()
            .find(|area| area.id == old_area)
            .unwrap()
            .lines
            .remove(old_position);
    }
    if let Some(index) = destination_index {
        let destination = &mut topology.areas[index].lines;
        let insertion = position.unwrap_or(destination.len());
        destination.insert(insertion, id);
    }
    Ok((previous_area, previous_position))
}

/// Removes `device` from whichever building part currently lists it, if
/// any, returning that part's id. Unlike `remove_device_from_topology`,
/// absence is not an error: a device with no building placement at all
/// is a normal state (building placement isn't exhaustive the way
/// topology's unassigned/line split is), so this returns `Ok`-shaped
/// `None` rather than `Err`. `installation.buildings` is searched flat
/// — no recursion needed, since it is already a flat list linked by
/// `parent`/`children` ids, not a nested structure.
fn remove_device_from_buildings(
    installation: &mut Installation,
    device: DeviceId,
) -> Option<BuildingPartId> {
    installation.buildings.iter_mut().find_map(|part| {
        let pos = part.devices.iter().position(|&d| d == device)?;
        part.devices.remove(pos);
        Some(part.id)
    })
}

/// Reparent a building part only after checking both sides of the flat
/// parent/children relation. Return its old parent and child-list position so
/// the inverse can restore ordering, including after an atomic batch fails.
/// Undo may restore an imported cycle: rejecting that inverse would turn a
/// successful repair into an irreversible edit of the imported project.
fn relocate_building_part(
    installation: &mut Installation,
    id: BuildingPartId,
    parent: Option<BuildingPartId>,
    position: Option<usize>,
    validate_new_parent: bool,
) -> Result<(Option<BuildingPartId>, usize), CommandError> {
    let parts = &mut installation.buildings;
    let part_index = parts
        .iter()
        .position(|part| part.id == id)
        .ok_or(CommandError::BuildingPartNotFound(id))?;
    if parts.iter().filter(|part| part.id == id).count() != 1 {
        return Err(CommandError::BuildingPartPlacementAmbiguous(id));
    }
    let references: Vec<_> = parts
        .iter()
        .flat_map(|part| {
            part.children
                .iter()
                .enumerate()
                .filter_map(move |(index, &child)| (child == id).then_some((part.id, index)))
        })
        .collect();
    let (previous_parent, previous_position) =
        match (parts[part_index].parent, references.as_slice()) {
            (None, []) => (None, 0),
            (Some(expected), [(actual, index)]) if expected == *actual => (Some(expected), *index),
            _ => return Err(CommandError::BuildingPartPlacementAmbiguous(id)),
        };
    if let Some(previous) = previous_parent {
        if parts.iter().filter(|part| part.id == previous).count() != 1 {
            return Err(CommandError::BuildingPartPlacementAmbiguous(id));
        }
    }
    if let Some(target) = parent {
        let count = parts.iter().filter(|part| part.id == target).count();
        if count == 0 {
            return Err(CommandError::BuildingPartNotFound(target));
        }
        if count > 1 {
            return Err(CommandError::BuildingPartPlacementAmbiguous(id));
        }
    }

    if validate_new_parent {
        // Bound the walk even for an imported hierarchy that already cycles.
        let mut ancestor = parent;
        for _ in 0..=parts.len() {
            let Some(ancestor_id) = ancestor else { break };
            if ancestor_id == id {
                return Err(CommandError::BuildingPartCycle {
                    id,
                    parent: parent.unwrap(),
                });
            }
            let mut matches = parts.iter().filter(|part| part.id == ancestor_id);
            let candidate = matches
                .next()
                .ok_or(CommandError::BuildingPartNotFound(ancestor_id))?;
            if matches.next().is_some() {
                return Err(CommandError::BuildingPartPlacementAmbiguous(id));
            }
            ancestor = candidate.parent;
        }
        if ancestor.is_some() {
            return Err(CommandError::BuildingPartPlacementAmbiguous(id));
        }
    }
    if let (Some(target), Some(position)) = (parent, position) {
        let destination = parts.iter().find(|part| part.id == target).unwrap();
        let max_position = destination.children.len() - usize::from(previous_parent == parent);
        if position > max_position {
            return Err(CommandError::InvalidBuildingPartPosition {
                parent: target,
                position,
            });
        }
    }
    if previous_parent == parent && (position.is_none() || position == Some(previous_position)) {
        return Ok((previous_parent, previous_position));
    }

    if let Some(old_parent) = previous_parent {
        parts
            .iter_mut()
            .find(|part| part.id == old_parent)
            .unwrap()
            .children
            .remove(previous_position);
    }
    parts[part_index].parent = parent;
    if let Some(target) = parent {
        let destination = parts.iter_mut().find(|part| part.id == target).unwrap();
        let position = position.unwrap_or(destination.children.len());
        destination.children.insert(position, id);
    }
    Ok((previous_parent, previous_position))
}

/// Move a range's parent reference and both child lists together. Forward
/// edits check address spans; inverse restores may recover an imported invalid
/// placement so repairing a project does not make its undo lossy.
fn relocate_group_range(
    installation: &mut Installation,
    id: GroupRangeId,
    parent: Option<GroupRangeId>,
    position: Option<usize>,
    validate_new_parent: bool,
) -> Result<(Option<GroupRangeId>, usize), CommandError> {
    let ranges = &mut installation.group_ranges;
    let range_index = ranges
        .iter()
        .position(|range| range.id == id)
        .ok_or(CommandError::GroupRangeNotFound(id))?;
    if ranges.iter().filter(|range| range.id == id).count() != 1 {
        return Err(CommandError::GroupRangePlacementAmbiguous(id));
    }
    let references: Vec<_> = ranges
        .iter()
        .flat_map(|range| {
            range
                .children
                .iter()
                .enumerate()
                .filter_map(move |(index, &child)| (child == id).then_some((range.id, index)))
        })
        .collect();
    let (previous_parent, previous_position) =
        match (ranges[range_index].parent, references.as_slice()) {
            (None, []) => (None, 0),
            (Some(expected), [(actual, index)]) if expected == *actual => (Some(expected), *index),
            _ => return Err(CommandError::GroupRangePlacementAmbiguous(id)),
        };
    if let Some(previous) = previous_parent {
        if ranges.iter().filter(|range| range.id == previous).count() != 1 {
            return Err(CommandError::GroupRangePlacementAmbiguous(id));
        }
    }
    if let Some(target) = parent {
        let count = ranges.iter().filter(|range| range.id == target).count();
        if count == 0 {
            return Err(CommandError::GroupRangeNotFound(target));
        }
        if count > 1 {
            return Err(CommandError::GroupRangePlacementAmbiguous(id));
        }
    }
    if let (Some(target), Some(position)) = (parent, position) {
        let destination = ranges.iter().find(|range| range.id == target).unwrap();
        let max_position = destination.children.len() - usize::from(previous_parent == parent);
        if position > max_position {
            return Err(CommandError::InvalidGroupRangePosition {
                parent: target,
                position,
            });
        }
    }
    if previous_parent == parent && (position.is_none() || position == Some(previous_position)) {
        return Ok((previous_parent, previous_position));
    }

    if validate_new_parent {
        let mut ancestor = parent;
        for _ in 0..=ranges.len() {
            let Some(ancestor_id) = ancestor else { break };
            if ancestor_id == id {
                return Err(CommandError::GroupRangeCycle {
                    id,
                    parent: parent.unwrap(),
                });
            }
            let mut matches = ranges.iter().filter(|range| range.id == ancestor_id);
            let candidate = matches
                .next()
                .ok_or(CommandError::GroupRangeNotFound(ancestor_id))?;
            if matches.next().is_some() {
                return Err(CommandError::GroupRangePlacementAmbiguous(id));
            }
            ancestor = candidate.parent;
        }
        if ancestor.is_some() {
            return Err(CommandError::GroupRangePlacementAmbiguous(id));
        }
        let range = &ranges[range_index];
        check_group_range_is_well_ordered(id, range.start, range.end)?;
        if let Some(target) = parent {
            let destination = ranges.iter().find(|range| range.id == target).unwrap();
            check_group_range_nests_in_parent(destination, id, range.start, range.end)?;
        }
        check_no_overlapping_group_range(
            ranges.iter().filter(|sibling| sibling.parent == parent),
            id,
            range.start,
            range.end,
        )?;
    }

    if let Some(old_parent) = previous_parent {
        ranges
            .iter_mut()
            .find(|range| range.id == old_parent)
            .unwrap()
            .children
            .remove(previous_position);
    }
    ranges[range_index].parent = parent;
    if let Some(target) = parent {
        let destination = ranges.iter_mut().find(|range| range.id == target).unwrap();
        let position = position.unwrap_or(destination.children.len());
        destination.children.insert(position, id);
    }
    Ok((previous_parent, previous_position))
}

/// Finds `(device, ets_id)` in `installation.parameters` and overwrites its
/// `raw` in place, or inserts a new `ParameterInstance` using `id`/
/// `source_path` if none exists yet — the one piece of find-or-create logic
/// `SetParameterValue::apply` and `RestoreParameterValue::apply`'s
/// "overwrite" branch both need (T18 slice 3 task 2, design D24). Returns
/// `None` for an insert, or `Some((existing row's own id, its raw before
/// this call))` for an overwrite — the caller uses this to build the
/// correct inverse without a new `ParameterInstanceId` ever being minted
/// for an update.
fn upsert_parameter_value(
    installation: &mut Installation,
    id: ParameterInstanceId,
    device: DeviceId,
    ets_id: &str,
    raw: &str,
    source_path: String,
) -> Option<(ParameterInstanceId, String)> {
    if let Some(entry) = installation
        .parameters
        .iter_mut()
        .find(|p| p.device == device && p.source.ets_id == ets_id)
    {
        let previous = (entry.id, entry.raw.clone());
        entry.raw = raw.to_string();
        Some(previous)
    } else {
        installation.parameters.push(ParameterInstance {
            id,
            device,
            source: SourceRef {
                path: source_path,
                ets_id: ets_id.to_string(),
            },
            raw: raw.to_string(),
        });
        None
    }
}

impl Command {
    /// Applies the command to `project`, returning its inverse on success.
    /// On failure, `project` is left untouched.
    pub fn apply(&self, project: &mut Project) -> Result<Command, CommandError> {
        match self {
            Command::SetIndividualAddress { device, address } => {
                let device = *device;
                let address = *address;
                let previous = project
                    .devices
                    .get(device)
                    .ok_or(CommandError::DeviceNotFound(device))?
                    .address;
                if let Some(addr) = address {
                    if let Some((area, line)) = assigned_line_prefix(project, device)? {
                        check_individual_address_on_line(
                            device,
                            addr,
                            area,
                            line,
                            previous == Some(addr),
                        )?;
                    }
                    check_no_duplicate_individual_address(&project.devices, device, addr)?;
                }
                project.devices.get_mut(device).unwrap().address = address;
                Ok(Command::RestoreIndividualAddress {
                    device,
                    address: previous,
                    redo_coupler: None,
                })
            }
            Command::SetCouplerIndividualAddress {
                device,
                address,
                evidence,
            } => {
                let device = *device;
                let address = *address;
                let current = project
                    .devices
                    .get(device)
                    .ok_or(CommandError::DeviceNotFound(device))?;
                if current.product_ref != evidence.product_ref {
                    return Err(CommandError::CouplerEvidenceMismatch {
                        device,
                        evidence_product_ref: evidence.product_ref.clone(),
                    });
                }
                let previous = current.address;
                if let Some((area, line)) = assigned_line_prefix(project, device)? {
                    check_individual_address_on_line(device, address, area, line, true)?;
                }
                check_no_duplicate_individual_address(&project.devices, device, address)?;
                project.devices.get_mut(device).unwrap().address = Some(address);
                Ok(Command::RestoreIndividualAddress {
                    device,
                    address: previous,
                    redo_coupler: Some(evidence.clone()),
                })
            }
            Command::RestoreIndividualAddress {
                device,
                address,
                redo_coupler,
            } => {
                let target = project
                    .devices
                    .get_mut(*device)
                    .ok_or(CommandError::DeviceNotFound(*device))?;
                let previous = std::mem::replace(&mut target.address, *address);
                Ok(match (redo_coupler, previous) {
                    (Some(evidence), Some(coupler_address)) => {
                        Command::SetCouplerIndividualAddress {
                            device: *device,
                            address: coupler_address,
                            evidence: evidence.clone(),
                        }
                    }
                    _ => Command::SetIndividualAddress {
                        device: *device,
                        address: previous,
                    },
                })
            }
            Command::SetDeviceDescription {
                device,
                description,
            } => {
                let device = *device;
                let target = project
                    .devices
                    .get_mut(device)
                    .ok_or(CommandError::DeviceNotFound(device))?;
                let previous = target.description.clone();
                target.description = description.clone();
                Ok(Command::SetDeviceDescription {
                    device,
                    description: previous,
                })
            }
            Command::SetComObjectDpt { com_object, dpt } => {
                let com_object = *com_object;
                let com: &mut ComObjectInstance = project
                    .devices
                    .com_object_mut(com_object)
                    .ok_or(CommandError::ComObjectNotFound(com_object))?;
                let previous = com.dpt.clone();
                com.dpt = match dpt {
                    Some(value) => Override::Value(Resolved {
                        value: *value,
                        layer: Layer::UserEdit,
                    }),
                    // A user-initiated clear is a deliberate empty, mirroring
                    // ETS's own `DatapointType=""` convention; `Absent` would
                    // misrepresent a value the user just acted on.
                    None => Override::Empty,
                };
                Ok(Command::RestoreComObjectDpt {
                    com_object,
                    dpt: previous,
                })
            }
            Command::RestoreComObjectDpt { com_object, dpt } => {
                let com_object = *com_object;
                let com: &mut ComObjectInstance = project
                    .devices
                    .com_object_mut(com_object)
                    .ok_or(CommandError::ComObjectNotFound(com_object))?;
                let previous = com.dpt.clone();
                com.dpt = dpt.clone();
                Ok(Command::RestoreComObjectDpt {
                    com_object,
                    dpt: previous,
                })
            }
            Command::SetComObjectDescription {
                com_object,
                description,
            } => {
                let com_object = *com_object;
                let com: &mut ComObjectInstance = project
                    .devices
                    .com_object_mut(com_object)
                    .ok_or(CommandError::ComObjectNotFound(com_object))?;
                let previous = com.description.clone();
                com.description = match description {
                    Some(value) => Override::Value(Resolved {
                        value: Text::Literal(value.clone()),
                        layer: Layer::UserEdit,
                    }),
                    // A user-initiated clear is a deliberate empty, mirroring
                    // `SetComObjectDpt`'s own convention (see its comment).
                    None => Override::Empty,
                };
                Ok(Command::RestoreComObjectDescription {
                    com_object,
                    description: previous,
                })
            }
            Command::RestoreComObjectDescription {
                com_object,
                description,
            } => {
                let com_object = *com_object;
                let com: &mut ComObjectInstance = project
                    .devices
                    .com_object_mut(com_object)
                    .ok_or(CommandError::ComObjectNotFound(com_object))?;
                let previous = com.description.clone();
                com.description = description.clone();
                Ok(Command::RestoreComObjectDescription {
                    com_object,
                    description: previous,
                })
            }
            Command::SetComObjectFlag {
                com_object,
                flag,
                value,
            } => {
                let com_object = *com_object;
                let flag = *flag;
                let com: &mut ComObjectInstance = project
                    .devices
                    .com_object_mut(com_object)
                    .ok_or(CommandError::ComObjectNotFound(com_object))?;
                let previous = com.flags.get(flag).clone();
                *com.flags.get_mut(flag) = Override::Value(Resolved {
                    value: *value,
                    layer: Layer::UserEdit,
                });
                Ok(Command::RestoreComObjectFlag {
                    com_object,
                    flag,
                    value: previous,
                })
            }
            Command::RestoreComObjectFlag {
                com_object,
                flag,
                value,
            } => {
                let com_object = *com_object;
                let flag = *flag;
                let com: &mut ComObjectInstance = project
                    .devices
                    .com_object_mut(com_object)
                    .ok_or(CommandError::ComObjectNotFound(com_object))?;
                let previous = com.flags.get(flag).clone();
                *com.flags.get_mut(flag) = value.clone();
                Ok(Command::RestoreComObjectFlag {
                    com_object,
                    flag,
                    value: previous,
                })
            }
            Command::SetParameterValue {
                id,
                device,
                ets_id,
                raw,
            } => {
                let id = *id;
                let device_id = *device;
                let source_path = project
                    .devices
                    .get(device_id)
                    .ok_or(CommandError::DeviceNotFound(device_id))?
                    .source
                    .path
                    .clone();
                // Only a *new* instance inserts `id`; overwriting the value
                // of an existing (device, ets_id) row keeps that row's id.
                // The row lives in the installation that already holds it,
                // else in the device's own installation (MODEL-01).
                let index = parameter_installation(project, device_id, ets_id)?;
                let is_new_instance = !project.installations[index]
                    .parameters
                    .iter()
                    .any(|p| p.device == device_id && p.source.ets_id == *ets_id);
                if is_new_instance {
                    check_id_free(project, IdKind::ParameterInstance, id.0)?;
                }
                let installation = &mut project.installations[index];
                match upsert_parameter_value(installation, id, device_id, ets_id, raw, source_path)
                {
                    Some((previous_id, previous_raw)) => Ok(Command::RestoreParameterValue {
                        id: previous_id,
                        device: device_id,
                        ets_id: ets_id.clone(),
                        raw: Some(previous_raw),
                    }),
                    None => Ok(Command::RestoreParameterValue {
                        id,
                        device: device_id,
                        ets_id: ets_id.clone(),
                        raw: None,
                    }),
                }
            }
            Command::RestoreParameterValue {
                id: _,
                device,
                ets_id,
                raw,
            } => {
                let device_id = *device;
                let index = parameter_installation(project, device_id, ets_id)?;
                let installation = &mut project.installations[index];
                match raw {
                    Some(prior) => {
                        // Constructed only from a prior overwrite (`SetParameterValue`'s
                        // "found" branch), so the row is expected to still be there;
                        // `DeviceNotFound` is the closest existing variant if it
                        // somehow is not (no new `CommandError` variant for this task).
                        let entry = installation
                            .parameters
                            .iter_mut()
                            .find(|p| p.device == device_id && p.source.ets_id == *ets_id)
                            .ok_or(CommandError::DeviceNotFound(device_id))?;
                        let entry_id = entry.id;
                        let current_raw = entry.raw.clone();
                        entry.raw = prior.clone();
                        Ok(Command::SetParameterValue {
                            id: entry_id,
                            device: device_id,
                            ets_id: ets_id.clone(),
                            raw: current_raw,
                        })
                    }
                    None => {
                        let pos = installation
                            .parameters
                            .iter()
                            .position(|p| p.device == device_id && p.source.ets_id == *ets_id)
                            .ok_or(CommandError::DeviceNotFound(device_id))?;
                        let removed = installation.parameters.remove(pos);
                        Ok(Command::SetParameterValue {
                            id: removed.id,
                            device: device_id,
                            ets_id: ets_id.clone(),
                            raw: removed.raw,
                        })
                    }
                }
            }
            Command::CreateGroupAddress {
                entry,
                installation,
            } => {
                check_id_free(project, IdKind::GroupAddress, entry.id.0)?;
                let parent = entry
                    .range
                    .map(|range| require_unique_group_range(project, range))
                    .transpose()?;
                let index = create_installation(project, parent, *installation)?;
                let installation = &mut project.installations[index];
                check_no_duplicate_group_address(installation, entry.id, entry.address)?;
                if let Some(range_id) = entry.range {
                    let range = installation
                        .group_ranges
                        .iter()
                        .find(|r| r.id == range_id)
                        .ok_or(CommandError::GroupRangeNotFound(range_id))?;
                    check_group_address_in_range(range, entry.address)?;
                }
                let id = entry.id;
                installation.group_addresses.push(entry.clone());
                Ok(Command::DeleteGroupAddress { id })
            }
            Command::DeleteGroupAddress { id } => {
                let id = *id;
                if project
                    .devices
                    .com_objects()
                    .any(|com| com.links.iter().any(|link| link.ga == id))
                {
                    return Err(CommandError::GroupAddressInUse(id));
                }
                let index = require_unique_group_address(project, id)?;
                let installation = &mut project.installations[index];
                let pos = installation
                    .group_addresses
                    .iter()
                    .position(|e| e.id == id)
                    .ok_or(CommandError::GroupAddressNotFound(id))?;
                let entry = installation.group_addresses.remove(pos);
                Ok(Command::RestoreGroupAddress {
                    entry,
                    position: pos,
                    installation: installation.id,
                })
            }
            Command::RestoreGroupAddress {
                entry,
                position,
                installation,
            } => {
                let index = restore_installation(project, *installation)?;
                let installation = &mut project.installations[index];
                check_no_duplicate_group_address(installation, entry.id, entry.address)?;
                if let Some(range_id) = entry.range {
                    let range = installation
                        .group_ranges
                        .iter()
                        .find(|range| range.id == range_id)
                        .ok_or(CommandError::GroupRangeNotFound(range_id))?;
                    check_group_address_in_range(range, entry.address)?;
                }
                let position = (*position).min(installation.group_addresses.len());
                let id = entry.id;
                installation.group_addresses.insert(position, entry.clone());
                Ok(Command::DeleteGroupAddress { id })
            }
            Command::CreateArea { area, installation } => {
                check_id_free(project, IdKind::Area, area.id.0)?;
                let index = target_installation(project, *installation)?;
                let installation = &mut project.installations[index];
                check_no_duplicate_area_address(&installation.topology, area.id, area.address)?;
                let id = area.id;
                installation.topology.areas.push(area.clone());
                Ok(Command::DeleteArea { id })
            }
            Command::DeleteArea { id } => {
                let id = *id;
                let index = require_unique_area(project, id)?;
                let installation = &mut project.installations[index];
                let pos = installation
                    .topology
                    .areas
                    .iter()
                    .position(|a| a.id == id)
                    .ok_or(CommandError::AreaNotFound(id))?;
                if !installation.topology.areas[pos].lines.is_empty() {
                    return Err(CommandError::AreaNotEmpty(id));
                }
                let area = installation.topology.areas.remove(pos);
                Ok(Command::RestoreArea {
                    area,
                    position: pos,
                    installation: installation.id,
                })
            }
            Command::RestoreArea {
                area,
                position,
                installation,
            } => {
                check_id_free(project, IdKind::Area, area.id.0)?;
                let index = restore_installation(project, *installation)?;
                let installation = &mut project.installations[index];
                if *position > installation.topology.areas.len() {
                    return Err(CommandError::InvalidStructurePosition {
                        kind: IdKind::Area,
                        id: area.id.0,
                        position: *position,
                    });
                }
                installation.topology.areas.insert(*position, area.clone());
                Ok(Command::DeleteArea { id: area.id })
            }
            Command::RenameArea { id, name } => {
                let id = *id;
                let index = require_unique_area(project, id)?;
                let installation = &mut project.installations[index];
                let area = installation
                    .topology
                    .areas
                    .iter_mut()
                    .find(|area| area.id == id)
                    .ok_or(CommandError::AreaNotFound(id))?;
                let previous = std::mem::replace(&mut area.name, name.clone());
                Ok(Command::RenameArea { id, name: previous })
            }
            Command::CreateLine { area, line } => {
                check_id_free(project, IdKind::Line, line.id.0)?;
                let index = require_unique_area(project, *area)?;
                let area_id = *area;
                let installation = &mut project.installations[index];
                let area_ref = installation
                    .topology
                    .areas
                    .iter()
                    .find(|a| a.id == area_id)
                    .ok_or(CommandError::AreaNotFound(area_id))?;
                check_no_duplicate_line_address(
                    area_ref,
                    &installation.topology.lines,
                    line.id,
                    line.address,
                )?;
                let id = line.id;
                installation.topology.lines.push(line.clone());
                installation
                    .topology
                    .areas
                    .iter_mut()
                    .find(|a| a.id == area_id)
                    .unwrap()
                    .lines
                    .push(id);
                Ok(Command::DeleteLine { id })
            }
            Command::DeleteLine { id } => {
                let id = *id;
                let index = require_unique_line(project, id)?;
                let installation = &mut project.installations[index];
                let (pos, placement) = line_placement(&installation.topology, id)?;
                let (area_id, area_position) = placement.ok_or(CommandError::LineNotFound(id))?;
                if !installation.topology.lines[pos].devices.is_empty() {
                    return Err(CommandError::LineNotEmpty(id));
                }
                let line = installation.topology.lines.remove(pos);
                let area = installation
                    .topology
                    .areas
                    .iter_mut()
                    .find(|area| area.id == area_id && area.lines.get(area_position) == Some(&id))
                    .expect("line_placement verified the owning area");
                area.lines.remove(area_position);
                Ok(Command::RestoreLine {
                    area: area_id,
                    line,
                    line_position: pos,
                    area_position,
                    installation: installation.id,
                })
            }
            Command::RestoreLine {
                area,
                line,
                line_position,
                area_position,
                installation,
            } => {
                check_id_free(project, IdKind::Line, line.id.0)?;
                let index = restore_installation(project, *installation)?;
                let installation = &mut project.installations[index];
                if *line_position > installation.topology.lines.len() {
                    return Err(CommandError::InvalidStructurePosition {
                        kind: IdKind::Line,
                        id: line.id.0,
                        position: *line_position,
                    });
                }
                let areas = &mut installation.topology.areas;
                let mut owners = areas
                    .iter()
                    .enumerate()
                    .filter(|(_, candidate)| candidate.id == *area);
                let index = owners
                    .next()
                    .map(|(index, _)| index)
                    .ok_or(CommandError::AreaNotFound(*area))?;
                if owners.next().is_some() {
                    return Err(CommandError::AreaPlacementAmbiguous(*area));
                }
                if areas
                    .iter()
                    .any(|candidate| candidate.lines.contains(&line.id))
                {
                    return Err(CommandError::LinePlacementAmbiguous(line.id));
                }
                if *area_position > areas[index].lines.len() {
                    return Err(CommandError::InvalidLinePosition {
                        area: *area,
                        position: *area_position,
                    });
                }
                installation
                    .topology
                    .lines
                    .insert(*line_position, line.clone());
                installation.topology.areas[index]
                    .lines
                    .insert(*area_position, line.id);
                Ok(Command::DeleteLine { id: line.id })
            }
            Command::RenameLine { id, name } => {
                let id = *id;
                let index = require_unique_line(project, id)?;
                let installation = &mut project.installations[index];
                let line = installation
                    .topology
                    .lines
                    .iter_mut()
                    .find(|line| line.id == id)
                    .ok_or(CommandError::LineNotFound(id))?;
                let previous = std::mem::replace(&mut line.name, name.clone());
                Ok(Command::RenameLine { id, name: previous })
            }
            Command::MoveLineToArea { id, area } => {
                let index = require_unique_line(project, *id)?;
                let area_index = require_unique_area(project, *area)?;
                same_installation(project, index, area_index)?;
                let installation = &project.installations[index];
                let (line_index, previous) = line_placement(&installation.topology, *id)?;
                if previous.map(|(owner, _)| owner) != Some(*area) {
                    for &device in &installation.topology.lines[line_index].devices {
                        project
                            .devices
                            .get(device)
                            .ok_or(CommandError::DeviceNotFound(device))?;
                        // Refuse a second placement in another line or in
                        // `unassigned` before changing the area relationship.
                        assigned_line_prefix(project, device)?;
                    }
                }
                let devices = &project.devices;
                let installation = &mut project.installations[index];
                let (old_area, position) =
                    relocate_line(installation, devices, *id, Some(*area), None, true)?;
                Ok(Command::RestoreLinePlacement {
                    id: *id,
                    area: old_area,
                    position,
                })
            }
            Command::RestoreLinePlacement { id, area, position } => {
                let index = require_unique_line(project, *id)?;
                // Undo may restore an imported area id that also exists in another installation.
                let devices = &project.devices;
                let installation = &mut project.installations[index];
                let (old_area, old_position) =
                    relocate_line(installation, devices, *id, *area, Some(*position), false)?;
                Ok(Command::RestoreLinePlacement {
                    id: *id,
                    area: old_area,
                    position: old_position,
                })
            }
            Command::MoveDeviceToLine { device, line } => {
                let device = *device;
                let line = *line;
                let existing_address = project
                    .devices
                    .get(device)
                    .ok_or(CommandError::DeviceNotFound(device))?
                    .address;
                if let (Some(address), Some((area, current_line))) =
                    (existing_address, assigned_line_prefix(project, device)?)
                {
                    // A move must also be undoable. An imported mismatch
                    // cannot be restored by the inverse Move command; clear
                    // its address explicitly before changing its placement.
                    check_individual_address_on_line(device, address, area, current_line, true)?;
                }
                let current = device_installation(project, device)?;
                let index = match line {
                    Some(line_id) => {
                        let target = require_unique_line(project, line_id)?;
                        if let Some(current) = current {
                            same_installation(project, current, target)?;
                        }
                        target
                    }
                    None => match current {
                        Some(current) => current,
                        None => target_installation(project, None)?,
                    },
                };
                let installation = &mut project.installations[index];
                if let Some(line_id) = line {
                    if !installation.topology.lines.iter().any(|l| l.id == line_id) {
                        return Err(CommandError::LineNotFound(line_id));
                    }
                    let area = unique_line_owner(&installation.topology, line_id)?;
                    if let Some(address) = existing_address {
                        let target = installation
                            .topology
                            .lines
                            .iter()
                            .find(|l| l.id == line_id)
                            .expect("target line was checked above");
                        let same_line = target.devices.contains(&device);
                        check_individual_address_on_line(
                            device,
                            address,
                            area.address,
                            target.address,
                            same_line,
                        )?;
                    }
                }
                let (previous, _) = remove_device_from_topology(installation, device)?;
                match line {
                    Some(line_id) => {
                        installation
                            .topology
                            .lines
                            .iter_mut()
                            .find(|l| l.id == line_id)
                            .unwrap()
                            .devices
                            .push(device);
                    }
                    None => installation.topology.unassigned.push(device),
                }
                Ok(Command::MoveDeviceToLine {
                    device,
                    line: previous,
                })
            }
            Command::SetIdAllocators { ids } => {
                let previous = std::mem::replace(&mut project.ids, ids.clone());
                Ok(Command::SetIdAllocators { ids: previous })
            }
            Command::ReserveIds { through } => {
                project.ids.raise_to(through);
                Ok(Command::ReserveIds {
                    through: through.clone(),
                })
            }
            Command::CreateDevice {
                device,
                com_objects,
                program_defaults,
                installation,
                line,
            } => {
                let device = device.clone();
                let com_objects = com_objects.clone();
                let program_defaults = program_defaults.clone();
                let installation_id = *installation;
                let line = *line;
                let device_id = device.id;
                check_id_free(project, IdKind::Device, device_id.0)?;
                for (index, com) in com_objects.iter().enumerate() {
                    check_id_free(project, IdKind::ComObjectInstance, com.id.0)?;
                    if com_objects[..index]
                        .iter()
                        .any(|earlier| earlier.id == com.id)
                    {
                        return Err(CommandError::IdInUse {
                            kind: IdKind::ComObjectInstance,
                            id: com.id.0,
                        });
                    }
                }
                let installation = match installation_id {
                    Some(id) => project
                        .installations
                        .iter_mut()
                        .find(|installation| installation.id == id)
                        .ok_or(CommandError::InstallationNotFound)?,
                    None => project
                        .installations
                        .first_mut()
                        .ok_or(CommandError::InstallationNotFound)?,
                };
                if let Some(line_id) = line {
                    if !installation.topology.lines.iter().any(|l| l.id == line_id) {
                        return Err(CommandError::LineNotFound(line_id));
                    }
                }
                for com in &com_objects {
                    project.devices.insert_com_object(com.clone());
                }
                // After the instances exist: `set_program_defaults` is keyed
                // by `ComObjectInstanceId`, so restoring before the insert
                // would attach defaults to nothing.
                for (com_id, defaults) in program_defaults {
                    project.devices.set_program_defaults(com_id, defaults);
                }
                project.devices.insert(device);
                match line {
                    Some(line_id) => {
                        installation
                            .topology
                            .lines
                            .iter_mut()
                            .find(|l| l.id == line_id)
                            .unwrap()
                            .devices
                            .push(device_id);
                    }
                    None => installation.topology.unassigned.push(device_id),
                }
                Ok(Command::DeleteDevice { id: device_id })
            }
            Command::RestoreDevice {
                device,
                com_objects,
                program_defaults,
                installation,
                line,
                position,
            } => {
                let device = device.clone();
                let com_objects = com_objects.clone();
                let program_defaults = program_defaults.clone();
                let installation_id = *installation;
                let line = *line;
                let position = *position;
                let device_id = device.id;
                let installation = project
                    .installations
                    .iter_mut()
                    .find(|candidate| candidate.id == installation_id)
                    .ok_or(CommandError::InstallationNotFound)?;
                let target = match line {
                    Some(line_id) => {
                        &mut installation
                            .topology
                            .lines
                            .iter_mut()
                            .find(|candidate| candidate.id == line_id)
                            .ok_or(CommandError::LineNotFound(line_id))?
                            .devices
                    }
                    None => &mut installation.topology.unassigned,
                };
                if position > target.len() {
                    return Err(CommandError::InvalidTopologyPosition {
                        device: device_id,
                        position,
                    });
                }
                for com in &com_objects {
                    project.devices.insert_com_object(com.clone());
                }
                for (com_id, defaults) in program_defaults {
                    project.devices.set_program_defaults(com_id, defaults);
                }
                project.devices.insert(device);
                target.insert(position, device_id);
                Ok(Command::DeleteDevice { id: device_id })
            }
            Command::DeleteDevice { id } => {
                let id = *id;
                let device_ref = project
                    .devices
                    .get(id)
                    .ok_or(CommandError::DeviceNotFound(id))?;
                let has_links = device_ref.com_objects.iter().any(|&com_id| {
                    project
                        .devices
                        .com_object(com_id)
                        .is_some_and(|c| !c.links.is_empty())
                });
                if has_links {
                    return Err(CommandError::DeviceHasLinks(id));
                }
                let has_project_references = project.installations.iter().any(|installation| {
                    installation
                        .buildings
                        .iter()
                        .any(|part| part.devices.contains(&id))
                        || installation
                            .parameters
                            .iter()
                            .any(|parameter| parameter.device == id)
                }) || project
                    .devices
                    .module_instances()
                    .any(|module| module.device == id);
                if has_project_references {
                    return Err(CommandError::DeviceHasDependentData(id));
                }
                let installation_index = project
                    .installations
                    .iter()
                    .position(|installation| {
                        installation.topology.unassigned.contains(&id)
                            || installation
                                .topology
                                .lines
                                .iter()
                                .any(|line| line.devices.contains(&id))
                    })
                    .ok_or(CommandError::DeviceNotFound(id))?;
                let installation = &mut project.installations[installation_index];
                let installation_id = installation.id;
                let (line, position) = remove_device_from_topology(installation, id)?;
                let device = project.devices.remove(id).unwrap();
                // `remove_com_object` also drops the com object's
                // `program_defaults` entry (ADR-0027), so the inverse has to
                // read it out first or undo would resurrect the device
                // without the values enrichment lifted for it. A com object
                // with no defaults contributes nothing, so undo cannot
                // invent an empty-but-present record either.
                let mut com_objects = Vec::with_capacity(device.com_objects.len());
                let mut program_defaults = Vec::new();
                for &com_id in &device.com_objects {
                    let defaults = project.devices.program_defaults(com_id).cloned();
                    let Some(com) = project.devices.remove_com_object(com_id) else {
                        continue;
                    };
                    com_objects.push(com);
                    if let Some(defaults) = defaults {
                        program_defaults.push((com_id, defaults));
                    }
                }
                Ok(Command::RestoreDevice {
                    device,
                    com_objects,
                    program_defaults,
                    installation: installation_id,
                    line,
                    position,
                })
            }
            Command::CreateBuildingPart { part, installation } => {
                check_id_free(project, IdKind::BuildingPart, part.id.0)?;
                let parent = part
                    .parent
                    .map(|parent| require_unique_building_part(project, parent))
                    .transpose()?;
                let index = create_installation(project, parent, *installation)?;
                let installation = &mut project.installations[index];
                if let Some(parent_id) = part.parent {
                    if !installation.buildings.iter().any(|p| p.id == parent_id) {
                        return Err(CommandError::BuildingPartNotFound(parent_id));
                    }
                }
                let id = part.id;
                if let Some(parent_id) = part.parent {
                    installation
                        .buildings
                        .iter_mut()
                        .find(|p| p.id == parent_id)
                        .unwrap()
                        .children
                        .push(id);
                }
                installation.buildings.push(part.clone());
                Ok(Command::DeleteBuildingPart { id })
            }
            Command::DeleteBuildingPart { id } => {
                let id = *id;
                let index = require_unique_building_part(project, id)?;
                let installation = &mut project.installations[index];
                let pos = installation
                    .buildings
                    .iter()
                    .position(|p| p.id == id)
                    .ok_or(CommandError::BuildingPartNotFound(id))?;
                if !installation.buildings[pos].children.is_empty()
                    || !installation.buildings[pos].devices.is_empty()
                    || installation
                        .buildings
                        .iter()
                        .any(|part| part.parent == Some(id))
                {
                    return Err(CommandError::BuildingPartNotEmpty(id));
                }
                let parent = installation.buildings[pos].parent;
                let (_, child_position) =
                    relocate_building_part(installation, id, parent, None, false)?;
                let part = installation.buildings.remove(pos);
                if let Some(parent_id) = parent {
                    installation
                        .buildings
                        .iter_mut()
                        .find(|candidate| candidate.id == parent_id)
                        .expect("relocate_building_part verified the parent")
                        .children
                        .remove(child_position);
                }
                Ok(Command::RestoreBuildingPart {
                    part,
                    position: pos,
                    child_position: parent.map(|_| child_position),
                    installation: installation.id,
                })
            }
            Command::RestoreBuildingPart {
                part,
                position,
                child_position,
                installation,
            } => {
                check_id_free(project, IdKind::BuildingPart, part.id.0)?;
                let index = restore_installation(project, *installation)?;
                let installation = &mut project.installations[index];
                if *position > installation.buildings.len() {
                    return Err(CommandError::InvalidStructurePosition {
                        kind: IdKind::BuildingPart,
                        id: part.id.0,
                        position: *position,
                    });
                }
                let parent_position = match (part.parent, child_position) {
                    (None, None) => None,
                    (Some(parent_id), Some(child_position)) => {
                        let matches: Vec<_> = installation
                            .buildings
                            .iter()
                            .enumerate()
                            .filter(|(_, candidate)| candidate.id == parent_id)
                            .collect();
                        if matches.len() != 1 {
                            return Err(CommandError::BuildingPartPlacementAmbiguous(part.id));
                        }
                        let index = matches[0].0;
                        if *child_position > installation.buildings[index].children.len() {
                            return Err(CommandError::InvalidBuildingPartPosition {
                                parent: parent_id,
                                position: *child_position,
                            });
                        }
                        Some((parent_id, *child_position))
                    }
                    _ => return Err(CommandError::BuildingPartPlacementAmbiguous(part.id)),
                };
                if installation
                    .buildings
                    .iter()
                    .any(|candidate| candidate.children.contains(&part.id))
                {
                    return Err(CommandError::BuildingPartPlacementAmbiguous(part.id));
                }
                installation.buildings.insert(*position, part.clone());
                if let Some((parent_id, child_position)) = parent_position {
                    installation
                        .buildings
                        .iter_mut()
                        .find(|candidate| candidate.id == parent_id)
                        .expect("preflight verified the parent")
                        .children
                        .insert(child_position, part.id);
                }
                Ok(Command::DeleteBuildingPart { id: part.id })
            }
            Command::RenameBuildingPart { id, name } => {
                let id = *id;
                let index = require_unique_building_part(project, id)?;
                let installation = &mut project.installations[index];
                let part = installation
                    .buildings
                    .iter_mut()
                    .find(|p| p.id == id)
                    .ok_or(CommandError::BuildingPartNotFound(id))?;
                let previous = std::mem::replace(&mut part.name, name.clone());
                Ok(Command::RenameBuildingPart { id, name: previous })
            }
            Command::MoveBuildingPart { id, parent } => {
                let index = require_unique_building_part(project, *id)?;
                if let Some(parent_id) = parent {
                    let parent_index = require_unique_building_part(project, *parent_id)?;
                    same_installation(project, index, parent_index)?;
                }
                let installation = &mut project.installations[index];
                let (old_parent, position) =
                    relocate_building_part(installation, *id, *parent, None, true)?;
                Ok(Command::RestoreBuildingPartPlacement {
                    id: *id,
                    parent: old_parent,
                    position,
                })
            }
            Command::RestoreBuildingPartPlacement {
                id,
                parent,
                position,
            } => {
                let index = require_unique_building_part(project, *id)?;
                // Restoring an imported parent reference must not become a new edit.
                let installation = &mut project.installations[index];
                let (old_parent, old_position) =
                    relocate_building_part(installation, *id, *parent, Some(*position), false)?;
                Ok(Command::RestoreBuildingPartPlacement {
                    id: *id,
                    parent: old_parent,
                    position: old_position,
                })
            }
            Command::MoveDeviceToBuildingPart { device, part } => {
                let device = *device;
                let part = *part;
                if project.devices.get(device).is_none() {
                    return Err(CommandError::DeviceNotFound(device));
                }
                // The device's current building placement, in any installation.
                let placed_in = project
                    .installations
                    .iter()
                    .position(|i| i.buildings.iter().any(|p| p.devices.contains(&device)));
                let target = part
                    .map(|part_id| require_unique_building_part(project, part_id))
                    .transpose()?;
                if let Some(target) = target {
                    if let Some(current) = device_installation(project, device)? {
                        same_installation(project, current, target)?;
                    }
                }
                let previous = match placed_in {
                    Some(index) => {
                        remove_device_from_buildings(&mut project.installations[index], device)
                    }
                    None => None,
                };
                if let (Some(part_id), Some(index)) = (part, target) {
                    project.installations[index]
                        .buildings
                        .iter_mut()
                        .find(|p| p.id == part_id)
                        .unwrap()
                        .devices
                        .push(device);
                }
                Ok(Command::RestoreDeviceBuildingPlacement {
                    device,
                    part: previous,
                })
            }
            Command::RestoreDeviceBuildingPlacement { device, part } => {
                let device = *device;
                if project.devices.get(device).is_none() {
                    return Err(CommandError::DeviceNotFound(device));
                }
                let target = part
                    .map(|part_id| require_unique_building_part(project, part_id))
                    .transpose()?;
                let placed_in = project
                    .installations
                    .iter()
                    .position(|i| i.buildings.iter().any(|p| p.devices.contains(&device)));
                let previous = match placed_in {
                    Some(index) => {
                        remove_device_from_buildings(&mut project.installations[index], device)
                    }
                    None => None,
                };
                if let (Some(part_id), Some(index)) = (*part, target) {
                    project.installations[index]
                        .buildings
                        .iter_mut()
                        .find(|p| p.id == part_id)
                        .expect("require_unique_building_part found it")
                        .devices
                        .push(device);
                }
                Ok(Command::MoveDeviceToBuildingPart {
                    device,
                    part: previous,
                })
            }
            Command::CreateGroupRange {
                range,
                installation,
            } => {
                check_id_free(project, IdKind::GroupRange, range.id.0)?;
                let parent = range
                    .parent
                    .map(|parent| require_unique_group_range(project, parent))
                    .transpose()?;
                let index = create_installation(project, parent, *installation)?;
                let installation = &mut project.installations[index];
                check_group_range_is_well_ordered(range.id, range.start, range.end)?;
                if let Some(parent_id) = range.parent {
                    let parent = installation
                        .group_ranges
                        .iter()
                        .find(|r| r.id == parent_id)
                        .ok_or(CommandError::GroupRangeNotFound(parent_id))?;
                    check_group_range_nests_in_parent(parent, range.id, range.start, range.end)?;
                }
                check_no_overlapping_group_range(
                    installation
                        .group_ranges
                        .iter()
                        .filter(|r| r.parent == range.parent),
                    range.id,
                    range.start,
                    range.end,
                )?;
                let id = range.id;
                if let Some(parent_id) = range.parent {
                    installation
                        .group_ranges
                        .iter_mut()
                        .find(|r| r.id == parent_id)
                        .unwrap()
                        .children
                        .push(id);
                }
                installation.group_ranges.push(range.clone());
                Ok(Command::DeleteGroupRange { id })
            }
            Command::DeleteGroupRange { id } => {
                let id = *id;
                let index = require_unique_group_range(project, id)?;
                let installation = &mut project.installations[index];
                let pos = installation
                    .group_ranges
                    .iter()
                    .position(|r| r.id == id)
                    .ok_or(CommandError::GroupRangeNotFound(id))?;
                if !installation.group_ranges[pos].children.is_empty()
                    || installation
                        .group_ranges
                        .iter()
                        .any(|range| range.parent == Some(id))
                {
                    return Err(CommandError::GroupRangeNotEmpty(id));
                }
                if installation
                    .group_addresses
                    .iter()
                    .any(|ga| ga.range == Some(id))
                {
                    return Err(CommandError::GroupRangeInUse(id));
                }
                let parent = installation.group_ranges[pos].parent;
                let (_, child_position) =
                    relocate_group_range(installation, id, parent, None, false)?;
                let range = installation.group_ranges.remove(pos);
                if let Some(parent_id) = parent {
                    installation
                        .group_ranges
                        .iter_mut()
                        .find(|candidate| candidate.id == parent_id)
                        .expect("relocate_group_range verified the parent")
                        .children
                        .remove(child_position);
                }
                Ok(Command::RestoreGroupRange {
                    range,
                    position: pos,
                    child_position: parent.map(|_| child_position),
                    installation: installation.id,
                })
            }
            Command::RestoreGroupRange {
                range,
                position,
                child_position,
                installation,
            } => {
                check_id_free(project, IdKind::GroupRange, range.id.0)?;
                let index = restore_installation(project, *installation)?;
                let installation = &mut project.installations[index];
                if *position > installation.group_ranges.len() {
                    return Err(CommandError::InvalidStructurePosition {
                        kind: IdKind::GroupRange,
                        id: range.id.0,
                        position: *position,
                    });
                }
                let parent_position = match (range.parent, child_position) {
                    (None, None) => None,
                    (Some(parent_id), Some(child_position)) => {
                        let matches: Vec<_> = installation
                            .group_ranges
                            .iter()
                            .enumerate()
                            .filter(|(_, candidate)| candidate.id == parent_id)
                            .collect();
                        if matches.len() != 1 {
                            return Err(CommandError::GroupRangePlacementAmbiguous(range.id));
                        }
                        let index = matches[0].0;
                        if *child_position > installation.group_ranges[index].children.len() {
                            return Err(CommandError::InvalidGroupRangePosition {
                                parent: parent_id,
                                position: *child_position,
                            });
                        }
                        Some((parent_id, *child_position))
                    }
                    _ => return Err(CommandError::GroupRangePlacementAmbiguous(range.id)),
                };
                if installation
                    .group_ranges
                    .iter()
                    .any(|candidate| candidate.children.contains(&range.id))
                {
                    return Err(CommandError::GroupRangePlacementAmbiguous(range.id));
                }
                installation.group_ranges.insert(*position, range.clone());
                if let Some((parent_id, child_position)) = parent_position {
                    installation
                        .group_ranges
                        .iter_mut()
                        .find(|candidate| candidate.id == parent_id)
                        .expect("preflight verified the parent")
                        .children
                        .insert(child_position, range.id);
                }
                Ok(Command::DeleteGroupRange { id: range.id })
            }
            Command::RenameGroupRange { id, name } => {
                let id = *id;
                let index = require_unique_group_range(project, id)?;
                let installation = &mut project.installations[index];
                let range = installation
                    .group_ranges
                    .iter_mut()
                    .find(|r| r.id == id)
                    .ok_or(CommandError::GroupRangeNotFound(id))?;
                let previous = std::mem::replace(&mut range.name, name.clone());
                Ok(Command::RenameGroupRange { id, name: previous })
            }
            Command::MoveGroupRange { id, parent } => {
                let index = require_unique_group_range(project, *id)?;
                if let Some(parent_id) = parent {
                    let parent_index = require_unique_group_range(project, *parent_id)?;
                    same_installation(project, index, parent_index)?;
                }
                let installation = &mut project.installations[index];
                let (old_parent, position) =
                    relocate_group_range(installation, *id, *parent, None, true)?;
                Ok(Command::RestoreGroupRangePlacement {
                    id: *id,
                    parent: old_parent,
                    position,
                })
            }
            Command::RestoreGroupRangePlacement {
                id,
                parent,
                position,
            } => {
                let index = require_unique_group_range(project, *id)?;
                // Undo may recover an imported parent id shared with another installation.
                let installation = &mut project.installations[index];
                let (old_parent, old_position) =
                    relocate_group_range(installation, *id, *parent, Some(*position), false)?;
                Ok(Command::RestoreGroupRangePlacement {
                    id: *id,
                    parent: old_parent,
                    position: old_position,
                })
            }
            Command::UpdateGroupAddress {
                id,
                name,
                central,
                unfiltered,
            } => {
                let id = *id;
                let central = *central;
                let unfiltered = *unfiltered;
                let index = require_unique_group_address(project, id)?;
                let installation = &mut project.installations[index];
                let entry = installation
                    .group_addresses
                    .iter_mut()
                    .find(|e| e.id == id)
                    .ok_or(CommandError::GroupAddressNotFound(id))?;
                let previous_name = std::mem::replace(&mut entry.name, name.clone());
                let previous_central = std::mem::replace(&mut entry.central, central);
                let previous_unfiltered = std::mem::replace(&mut entry.unfiltered, unfiltered);
                Ok(Command::UpdateGroupAddress {
                    id,
                    name: previous_name,
                    central: previous_central,
                    unfiltered: previous_unfiltered,
                })
            }
            Command::ReaddressGroupAddress { id, address, range } => {
                let id = *id;
                let address = *address;
                let range = *range;
                let index = require_unique_group_address(project, id)?;
                if let Some(range_id) = range {
                    let range_index = require_unique_group_range(project, range_id)?;
                    same_installation(project, index, range_index)?;
                }
                let installation = &mut project.installations[index];
                check_no_duplicate_group_address(installation, id, address)?;
                if let Some(range_id) = range {
                    let target_range = installation
                        .group_ranges
                        .iter()
                        .find(|candidate| candidate.id == range_id)
                        .ok_or(CommandError::GroupRangeNotFound(range_id))?;
                    check_group_address_in_range(target_range, address)?;
                }
                let entry = installation
                    .group_addresses
                    .iter_mut()
                    .find(|entry| entry.id == id)
                    .ok_or(CommandError::GroupAddressNotFound(id))?;
                let previous_address = std::mem::replace(&mut entry.address, address);
                let previous_range = std::mem::replace(&mut entry.range, range);
                Ok(Command::ReaddressGroupAddress {
                    id,
                    address: previous_address,
                    range: previous_range,
                })
            }
            Command::LinkComObject {
                com_object,
                ga,
                direction,
            } => {
                let com_object = *com_object;
                let ga = *ga;
                let direction = *direction;
                let device = project
                    .devices
                    .com_object(com_object)
                    .ok_or(CommandError::ComObjectNotFound(com_object))?
                    .device;
                let index = match require_unique_group_address(project, ga) {
                    Ok(index) => index,
                    // Keep the existing dangling-link diagnostic for an absent id.
                    Err(CommandError::GroupAddressNotFound(_)) => {
                        target_installation(project, None)?
                    }
                    Err(other) => return Err(other),
                };
                if let Some(current) = device_installation(project, device)? {
                    if project.installations[index]
                        .group_addresses
                        .iter()
                        .any(|entry| entry.id == ga)
                    {
                        same_installation(project, current, index)?;
                    }
                }
                let installation = &project.installations[index];
                check_group_link_target_exists(installation, com_object, ga)?;
                let com = project
                    .devices
                    .com_object_mut(com_object)
                    .ok_or(CommandError::ComObjectNotFound(com_object))?;
                if com
                    .links
                    .iter()
                    .any(|l| l.ga == ga && l.direction == direction)
                {
                    return Err(CommandError::LinkAlreadyExists {
                        com_object,
                        ga,
                        direction,
                    });
                }
                com.links.push(GroupLink { ga, direction });
                Ok(Command::UnlinkComObject {
                    com_object,
                    ga,
                    direction,
                })
            }
            Command::UnlinkComObject {
                com_object,
                ga,
                direction,
            } => {
                let com_object = *com_object;
                let ga = *ga;
                let direction = *direction;
                let com = project
                    .devices
                    .com_object_mut(com_object)
                    .ok_or(CommandError::ComObjectNotFound(com_object))?;
                let pos = com
                    .links
                    .iter()
                    .position(|l| l.ga == ga && l.direction == direction)
                    .ok_or(CommandError::LinkNotFound {
                        com_object,
                        ga,
                        direction,
                    })?;
                let link = com.links.remove(pos);
                Ok(Command::RestoreGroupLink {
                    com_object,
                    link,
                    position: pos,
                })
            }
            Command::RestoreGroupLink {
                com_object,
                link,
                position,
            } => {
                let com = project
                    .devices
                    .com_object_mut(*com_object)
                    .ok_or(CommandError::ComObjectNotFound(*com_object))?;
                if *position > com.links.len() {
                    return Err(CommandError::InvalidGroupLinkPosition {
                        com_object: *com_object,
                        position: *position,
                    });
                }
                // An inverse restores the exact imported link, even if it
                // was duplicated or its group address has since gone away.
                com.links.insert(*position, *link);
                Ok(Command::UnlinkComObject {
                    com_object: *com_object,
                    ga: link.ga,
                    direction: link.direction,
                })
            }
            Command::SetGroupAddressStyle { style } => {
                let style = *style;
                for installation in &project.installations {
                    for entry in &installation.group_addresses {
                        if !entry.address.fits_style(style) {
                            return Err(CommandError::GroupAddressDoesNotFitStyle {
                                id: entry.id,
                                raw: entry.address.raw(),
                                style,
                            });
                        }
                    }
                    for range in &installation.group_ranges {
                        if !range.start.fits_style(style) {
                            return Err(CommandError::GroupRangeDoesNotFitStyle {
                                id: range.id,
                                raw: range.start.raw(),
                                style,
                            });
                        }
                        if !range.end.fits_style(style) {
                            return Err(CommandError::GroupRangeDoesNotFitStyle {
                                id: range.id,
                                raw: range.end.raw(),
                                style,
                            });
                        }
                    }
                }
                let previous = project.info.group_address_style;
                project.info.group_address_style = style;
                Ok(Command::SetGroupAddressStyle { style: previous })
            }
            Command::RepairDevicePlacement { device, keep } => {
                let removed = repair_device_placement(project, *device, *keep)?;
                Ok(Command::RestoreDevicePlacements {
                    device: *device,
                    keep: *keep,
                    removed,
                })
            }
            Command::RestoreDevicePlacements {
                device,
                keep,
                removed,
            } => {
                restore_device_placements(project, *device, removed)?;
                Ok(Command::RepairDevicePlacement {
                    device: *device,
                    keep: *keep,
                })
            }
            Command::RepairLineOwner { line, keep } => {
                let removed = repair_line_owner(project, *line, *keep)?;
                Ok(Command::RestoreLineOwners {
                    line: *line,
                    keep: *keep,
                    removed,
                })
            }
            Command::RestoreLineOwners {
                line,
                keep,
                removed,
            } => {
                restore_line_owners(project, *line, removed)?;
                Ok(Command::RepairLineOwner {
                    line: *line,
                    keep: *keep,
                })
            }
            Command::RenameInstallation { id, name } => {
                let index = target_installation(project, Some(*id))?;
                let previous =
                    std::mem::replace(&mut project.installations[index].name, name.clone());
                Ok(Command::RenameInstallation {
                    id: *id,
                    name: previous,
                })
            }
            Command::Batch(commands) => {
                // A rollback is not an undo: `ReserveIds` never rewinds on
                // undo, but a batch that fails must leave the project exactly
                // as it found it, reservation included (ADR-0039: a refused
                // create does not consume an id).
                let ids_before = project.ids.clone();
                let mut inverses = Vec::with_capacity(commands.len());
                for (index, cmd) in commands.iter().enumerate() {
                    match cmd.apply(project) {
                        Ok(inverse) => inverses.push(inverse),
                        Err(e) => {
                            // Roll back everything this batch already
                            // applied, in reverse order, before surfacing
                            // the original error with its child index — `apply`'s contract is
                            // "leave `project` untouched on `Err`", and that
                            // contract is per-`Command`, including `Batch`
                            // itself.
                            for inverse in inverses.into_iter().rev() {
                                inverse.apply(project).expect(
                                    "an inverse of an already-applied command must re-apply",
                                );
                            }
                            project.ids = ids_before;
                            return Err(CommandError::BatchItem {
                                index,
                                source: Box::new(e),
                            });
                        }
                    }
                }
                inverses.reverse();
                Ok(Command::Batch(inverses))
            }
        }
    }
}

/// Undo/redo stacks of applied commands' inverses. A failed `do_command`
/// leaves both stacks untouched.
#[derive(Debug, Default)]
pub struct CommandStack {
    undo: Vec<Command>,
    redo: Vec<Command>,
}

impl CommandStack {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn do_command(&mut self, project: &mut Project, cmd: Command) -> Result<(), CommandError> {
        let inverse = cmd.apply(project)?;
        self.undo.push(inverse);
        self.redo.clear();
        Ok(())
    }

    pub fn undo(&mut self, project: &mut Project) -> Result<(), CommandError> {
        let cmd = self.undo.pop().ok_or(CommandError::NothingToUndo)?;
        let inverse = cmd.apply(project)?;
        self.redo.push(inverse);
        Ok(())
    }

    pub fn redo(&mut self, project: &mut Project) -> Result<(), CommandError> {
        let cmd = self.redo.pop().ok_or(CommandError::NothingToRedo)?;
        let inverse = cmd.apply(project)?;
        self.undo.push(inverse);
        Ok(())
    }

    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::building::{BuildingPart, BuildingPartType};
    use crate::commissioning::{CommissioningState, CompletionStatus};
    use crate::device::DeviceInstance;
    use crate::flags::ResolvedFlags;
    use crate::group::GroupRange;
    use crate::ids::{InstallationId, LineId, SourceRef};
    use crate::installation::Installation;
    use crate::parameter::ParameterInstance;
    use crate::string_table::Text;
    use crate::topology::{Area, Topology};
    use crate::{GroupAddress, Language};

    fn source() -> SourceRef {
        SourceRef {
            path: "t".into(),
            ets_id: "t".into(),
        }
    }

    fn test_project_with_one_device(address: Option<IndividualAddress>) -> Project {
        let mut p = Project::new(Language("en".into()));
        p.installations.push(Installation {
            id: InstallationId(0),
            name: "I".into(),
            default_line: None,
            multicast_address: None,
            completion: CompletionStatus::FinishedDesign,
            topology: Topology {
                areas: vec![],
                lines: vec![],
                unassigned: vec![],
            },
            buildings: Vec::<BuildingPart>::new(),
            group_ranges: Vec::<GroupRange>::new(),
            group_addresses: vec![],
            parameters: vec![],
        });
        p.devices.insert(DeviceInstance {
            id: DeviceId(1),
            source: source(),
            name: "D".into(),
            description: None,
            address,
            product_ref: "P".into(),
            program_ref: "H".into(),
            commissioning: CommissioningState::default(),
            visibility_calculated: true,
            com_objects: vec![],
            binary_data: vec![],
        });
        p
    }

    #[test]
    fn set_individual_address_then_undo_restores_previous_value() {
        let mut project = test_project_with_one_device(None);
        let mut stack = CommandStack::new();
        let addr = IndividualAddress::new(1, 1, 1).unwrap();
        stack
            .do_command(
                &mut project,
                Command::SetIndividualAddress {
                    device: DeviceId(1),
                    address: Some(addr),
                },
            )
            .unwrap();
        assert_eq!(
            project.devices.get(DeviceId(1)).unwrap().address,
            Some(addr)
        );
        stack.undo(&mut project).unwrap();
        assert_eq!(project.devices.get(DeviceId(1)).unwrap().address, None);
        stack.redo(&mut project).unwrap();
        assert_eq!(
            project.devices.get(DeviceId(1)).unwrap().address,
            Some(addr)
        );
    }

    #[test]
    fn set_individual_address_rejects_duplicate() {
        let mut project =
            test_project_with_one_device(Some(IndividualAddress::new(1, 1, 1).unwrap()));
        project.devices.insert(DeviceInstance {
            id: DeviceId(2),
            source: source(),
            name: "D2".into(),
            description: None,
            address: None,
            product_ref: "P".into(),
            program_ref: "H".into(),
            commissioning: CommissioningState::default(),
            visibility_calculated: true,
            com_objects: vec![],
            binary_data: vec![],
        });
        let mut stack = CommandStack::new();
        let result = stack.do_command(
            &mut project,
            Command::SetIndividualAddress {
                device: DeviceId(2),
                address: Some(IndividualAddress::new(1, 1, 1).unwrap()),
            },
        );
        assert!(matches!(
            result,
            Err(CommandError::Validation(
                ValidationError::DuplicateIndividualAddress { .. }
            ))
        ));
        assert!(!stack.can_undo());
    }

    fn line_bound_device(address: Option<IndividualAddress>) -> Project {
        let mut project = project_with_line_and_unassigned_device();
        project.installations[0].topology.unassigned.clear();
        project.installations[0].topology.lines[0]
            .devices
            .push(DeviceId(1));
        project.devices.get_mut(DeviceId(1)).unwrap().address = address;
        project
    }

    #[test]
    fn line_bound_address_rejects_a_different_area_or_line_without_mutation() {
        let mut project = line_bound_device(Some(IndividualAddress::new(1, 1, 9).unwrap()));
        let before = project.clone();
        let mut stack = CommandStack::new();
        for address in [
            IndividualAddress::new(2, 1, 17).unwrap(),
            IndividualAddress::new(1, 2, 17).unwrap(),
        ] {
            let error = stack
                .do_command(
                    &mut project,
                    Command::SetIndividualAddress {
                        device: DeviceId(1),
                        address: Some(address),
                    },
                )
                .unwrap_err();
            assert!(error.to_string().contains("line 1.1"), "{error}");
            assert_eq!(project, before);
            assert!(!stack.can_undo());
        }
    }

    #[test]
    fn line_with_two_owning_areas_cannot_pick_an_arbitrary_address_prefix() {
        let mut project = line_bound_device(Some(IndividualAddress::new(1, 1, 9).unwrap()));
        let mut duplicate_owner = project.installations[0].topology.areas[0].clone();
        duplicate_owner.id = AreaId(2);
        duplicate_owner.address = 2;
        project.installations[0]
            .topology
            .areas
            .push(duplicate_owner);
        let before = project.clone();
        let mut stack = CommandStack::new();
        let error = stack
            .do_command(
                &mut project,
                Command::SetIndividualAddress {
                    device: DeviceId(1),
                    address: Some(IndividualAddress::new(1, 1, 17).unwrap()),
                },
            )
            .unwrap_err();
        assert!(error.to_string().contains("multiple areas"), "{error}");
        assert_eq!(project, before);
        assert!(!stack.can_undo());
    }

    #[test]
    fn later_installation_line_owns_the_device_address_prefix() {
        let original = IndividualAddress::new(2, 3, 9).unwrap();
        let mut project = line_bound_device(Some(original));
        let mut later = project.installations[0].clone();
        later.id = InstallationId(99);
        later.topology.areas[0].id = AreaId(99);
        later.topology.areas[0].address = 2;
        later.topology.areas[0].lines = vec![LineId(99)];
        later.topology.lines[0].id = LineId(99);
        later.topology.lines[0].address = 3;
        project.installations[0].topology.lines[0].devices.clear();
        project.installations.push(later);
        let mut stack = CommandStack::new();
        let changed = IndividualAddress::new(2, 3, 18).unwrap();
        stack
            .do_command(
                &mut project,
                Command::SetIndividualAddress {
                    device: DeviceId(1),
                    address: Some(changed),
                },
            )
            .unwrap();
        let before_invalid = project.clone();
        let error = stack
            .do_command(
                &mut project,
                Command::SetIndividualAddress {
                    device: DeviceId(1),
                    address: Some(IndividualAddress::new(1, 1, 18).unwrap()),
                },
            )
            .unwrap_err();
        assert!(error.to_string().contains("line 2.3"), "{error}");
        assert_eq!(project, before_invalid);
        stack.undo(&mut project).unwrap();
        assert_eq!(
            project.devices.get(DeviceId(1)).unwrap().address,
            Some(original)
        );
    }

    #[test]
    fn moving_into_a_line_owned_by_two_areas_is_refused_before_mutation() {
        let mut project = project_with_line_and_unassigned_device();
        let mut duplicate_owner = project.installations[0].topology.areas[0].clone();
        duplicate_owner.id = AreaId(2);
        duplicate_owner.address = 2;
        project.installations[0]
            .topology
            .areas
            .push(duplicate_owner);
        let before = project.clone();
        let mut stack = CommandStack::new();
        let error = stack
            .do_command(
                &mut project,
                Command::MoveDeviceToLine {
                    device: DeviceId(1),
                    line: Some(LineId(1)),
                },
            )
            .unwrap_err();
        assert!(error.to_string().contains("multiple areas"), "{error}");
        assert_eq!(project, before);
        assert!(!stack.can_undo());
    }

    #[test]
    fn device_in_two_lines_is_never_assigned_a_guessed_prefix() {
        let mut project = line_bound_device(Some(IndividualAddress::new(1, 1, 9).unwrap()));
        project.installations[0].topology.areas[0]
            .lines
            .push(LineId(2));
        project.installations[0]
            .topology
            .lines
            .push(test_line(LineId(2), 2, vec![DeviceId(1)]));
        let before = project.clone();
        let mut stack = CommandStack::new();
        let error = stack
            .do_command(
                &mut project,
                Command::SetIndividualAddress {
                    device: DeviceId(1),
                    address: Some(IndividualAddress::new(1, 1, 17).unwrap()),
                },
            )
            .unwrap_err();
        assert!(error.to_string().contains("multiple lines"), "{error}");
        assert_eq!(project, before);
        assert!(!stack.can_undo());
    }

    #[test]
    fn a_device_listed_as_both_line_bound_and_unassigned_cannot_be_readdressed_or_moved() {
        let original = IndividualAddress::new(1, 1, 9).unwrap();
        let mut project = line_bound_device(Some(original));
        project.installations[0]
            .topology
            .unassigned
            .push(DeviceId(1));
        let before = project.clone();
        let mut stack = CommandStack::new();
        for command in [
            Command::SetIndividualAddress {
                device: DeviceId(1),
                address: Some(IndividualAddress::new(1, 1, 18).unwrap()),
            },
            Command::MoveDeviceToLine {
                device: DeviceId(1),
                line: None,
            },
        ] {
            let error = stack.do_command(&mut project, command).unwrap_err();
            assert!(
                error.to_string().contains("multiple topology placements"),
                "{error}"
            );
            assert_eq!(project, before);
            assert!(!stack.can_undo());
        }
        // Clearing remains a deliberate, undoable repair action.
        stack
            .do_command(
                &mut project,
                Command::SetIndividualAddress {
                    device: DeviceId(1),
                    address: None,
                },
            )
            .unwrap();
        stack.undo(&mut project).unwrap();
        assert_eq!(project, before);
    }

    #[test]
    fn moving_a_device_unassigned_in_two_installations_cannot_leave_a_duplicate_placement() {
        let mut project = project_with_line_and_unassigned_device();
        let mut second = project.installations[0].clone();
        second.id = InstallationId(99);
        second.topology.areas[0].id = AreaId(99);
        second.topology.areas[0].lines[0] = LineId(99);
        second.topology.lines[0].id = LineId(99);
        project.installations.push(second);
        let before = project.clone();
        let mut stack = CommandStack::new();
        let error = stack
            .do_command(
                &mut project,
                Command::MoveDeviceToLine {
                    device: DeviceId(1),
                    line: Some(LineId(1)),
                },
            )
            .unwrap_err();
        assert!(
            error.to_string().contains("multiple topology placements"),
            "{error}"
        );
        assert_eq!(project, before);
        assert!(!stack.can_undo());
    }

    #[test]
    fn repeated_device_in_one_line_is_not_a_single_valid_placement() {
        let mut project = line_bound_device(None);
        project.installations[0].topology.lines[0]
            .devices
            .push(DeviceId(1));
        let before = project.clone();
        let mut stack = CommandStack::new();
        for command in [
            Command::SetIndividualAddress {
                device: DeviceId(1),
                address: Some(IndividualAddress::new(1, 1, 18).unwrap()),
            },
            Command::MoveDeviceToLine {
                device: DeviceId(1),
                line: None,
            },
        ] {
            let error = stack.do_command(&mut project, command).unwrap_err();
            assert!(
                error.to_string().contains("multiple topology placements"),
                "{error}"
            );
            assert_eq!(project, before);
            assert!(!stack.can_undo());
        }
    }

    #[test]
    fn assigning_coupler_only_zero_to_a_line_bound_device_is_explicitly_unsupported() {
        let mut project = line_bound_device(None);
        let mut stack = CommandStack::new();
        let coupler_address = IndividualAddress::new(1, 1, 0).unwrap();
        let error = stack
            .do_command(
                &mut project,
                Command::SetIndividualAddress {
                    device: DeviceId(1),
                    address: Some(coupler_address),
                },
            )
            .unwrap_err();
        assert!(error.to_string().contains("coupler"), "{error}");
        assert_eq!(project.devices.get(DeviceId(1)).unwrap().address, None);
        assert!(!stack.can_undo());
        // Existing imported couplers retain their address; clearing it remains possible.
        project.devices.get_mut(DeviceId(1)).unwrap().address = Some(coupler_address);
        stack
            .do_command(
                &mut project,
                Command::SetIndividualAddress {
                    device: DeviceId(1),
                    address: Some(coupler_address),
                },
            )
            .unwrap();
        stack
            .do_command(
                &mut project,
                Command::SetIndividualAddress {
                    device: DeviceId(1),
                    address: None,
                },
            )
            .unwrap();
        assert_eq!(project.devices.get(DeviceId(1)).unwrap().address, None);
    }

    #[test]
    fn coupler_evidence_permits_zero_only_for_the_evidenced_product() {
        let mut project = line_bound_device(None);
        let product_ref = project
            .devices
            .get(DeviceId(1))
            .unwrap()
            .product_ref
            .clone();
        let zero = IndividualAddress::new(1, 1, 0).unwrap();
        let mut stack = CommandStack::new();

        let before = project.clone();
        let error = stack
            .do_command(
                &mut project,
                Command::SetCouplerIndividualAddress {
                    device: DeviceId(1),
                    address: zero,
                    evidence: CouplerEvidence {
                        product_ref: format!("{product_ref}-other"),
                    },
                },
            )
            .unwrap_err();
        assert!(
            matches!(error, CommandError::CouplerEvidenceMismatch { .. }),
            "{error}"
        );
        assert_eq!(project, before);
        assert!(!stack.can_undo());

        let evidence = CouplerEvidence { product_ref };
        for wrong_line in [
            IndividualAddress::new(1, 2, 0).unwrap(),
            IndividualAddress::new(2, 1, 0).unwrap(),
        ] {
            let error = stack
                .do_command(
                    &mut project,
                    Command::SetCouplerIndividualAddress {
                        device: DeviceId(1),
                        address: wrong_line,
                        evidence: evidence.clone(),
                    },
                )
                .unwrap_err();
            assert!(error.to_string().contains("line 1.1"), "{error}");
            assert_eq!(project, before);
        }

        stack
            .do_command(
                &mut project,
                Command::SetCouplerIndividualAddress {
                    device: DeviceId(1),
                    address: zero,
                    evidence: evidence.clone(),
                },
            )
            .unwrap();
        assert_eq!(
            project.devices.get(DeviceId(1)).unwrap().address,
            Some(zero)
        );
        stack.undo(&mut project).unwrap();
        assert_eq!(project, before);
        stack.redo(&mut project).unwrap();
        assert_eq!(
            project.devices.get(DeviceId(1)).unwrap().address,
            Some(zero)
        );
        stack.undo(&mut project).unwrap();
        assert_eq!(project, before);
    }

    #[test]
    fn coupler_evidence_does_not_bypass_duplicate_addresses() {
        let mut project = line_bound_device(None);
        let product_ref = project
            .devices
            .get(DeviceId(1))
            .unwrap()
            .product_ref
            .clone();
        let zero = IndividualAddress::new(1, 1, 0).unwrap();
        let mut other = project.devices.get(DeviceId(1)).unwrap().clone();
        other.id = DeviceId(2);
        other.address = Some(zero);
        project.devices.insert(other);
        let before = project.clone();
        let error = CommandStack::new()
            .do_command(
                &mut project,
                Command::SetCouplerIndividualAddress {
                    device: DeviceId(1),
                    address: zero,
                    evidence: CouplerEvidence { product_ref },
                },
            )
            .unwrap_err();
        assert!(error.to_string().contains("already used"), "{error}");
        assert_eq!(project, before);
    }

    #[test]
    fn line_bound_device_address_can_change_its_device_octet_and_undo() {
        let mut project = line_bound_device(Some(IndividualAddress::new(1, 1, 9).unwrap()));
        let mut stack = CommandStack::new();
        let changed = IndividualAddress::new(1, 1, 18).unwrap();
        stack
            .do_command(
                &mut project,
                Command::SetIndividualAddress {
                    device: DeviceId(1),
                    address: Some(changed),
                },
            )
            .unwrap();
        assert_eq!(
            project.devices.get(DeviceId(1)).unwrap().address,
            Some(changed)
        );
        stack.undo(&mut project).unwrap();
        assert_eq!(
            project.devices.get(DeviceId(1)).unwrap().address,
            Some(IndividualAddress::new(1, 1, 9).unwrap())
        );
        stack.redo(&mut project).unwrap();
        assert_eq!(
            project.devices.get(DeviceId(1)).unwrap().address,
            Some(changed)
        );
    }

    #[test]
    fn undo_restores_imported_line_mismatch_and_existing_coupler_zero_losslessly() {
        for original in [
            IndividualAddress::new(2, 3, 9).unwrap(),
            IndividualAddress::new(1, 1, 0).unwrap(),
        ] {
            let mut project = line_bound_device(Some(original));
            let mut stack = CommandStack::new();
            stack
                .do_command(
                    &mut project,
                    Command::SetIndividualAddress {
                        device: DeviceId(1),
                        address: Some(IndividualAddress::new(1, 1, 18).unwrap()),
                    },
                )
                .unwrap();
            stack.undo(&mut project).unwrap();
            assert_eq!(
                project.devices.get(DeviceId(1)).unwrap().address,
                Some(original)
            );
            stack.redo(&mut project).unwrap();
            assert_eq!(
                project.devices.get(DeviceId(1)).unwrap().address,
                Some(IndividualAddress::new(1, 1, 18).unwrap())
            );
        }
    }

    #[test]
    fn move_rejects_a_nonmatching_existing_address_before_changing_line_membership() {
        let original = IndividualAddress::new(1, 1, 9).unwrap();
        let mut project = line_bound_device(Some(original));
        project.installations[0].topology.areas[0]
            .lines
            .push(LineId(2));
        project.installations[0]
            .topology
            .lines
            .push(test_line(LineId(2), 2, vec![]));
        let before = project.clone();
        let mut stack = CommandStack::new();
        let error = stack
            .do_command(
                &mut project,
                Command::MoveDeviceToLine {
                    device: DeviceId(1),
                    line: Some(LineId(2)),
                },
            )
            .unwrap_err();
        assert!(error.to_string().contains("line 1.2"), "{error}");
        assert_eq!(project, before);
        assert!(!stack.can_undo());
        // Intentional repair: clear the physical address, move, then re-address.
        stack
            .do_command(
                &mut project,
                Command::SetIndividualAddress {
                    device: DeviceId(1),
                    address: None,
                },
            )
            .unwrap();
        stack
            .do_command(
                &mut project,
                Command::MoveDeviceToLine {
                    device: DeviceId(1),
                    line: Some(LineId(2)),
                },
            )
            .unwrap();
        assert_eq!(
            project.installations[0].topology.lines[1].devices,
            vec![DeviceId(1)]
        );
        assert_eq!(project.devices.get(DeviceId(1)).unwrap().address, None);
        stack.undo(&mut project).unwrap();
        stack.undo(&mut project).unwrap();
        assert_eq!(project, before);
    }

    #[test]
    fn moving_an_imported_mismatch_cannot_create_an_unundoable_repair() {
        let mut project = line_bound_device(Some(IndividualAddress::new(1, 2, 9).unwrap()));
        project.installations[0].topology.areas[0]
            .lines
            .push(LineId(2));
        project.installations[0]
            .topology
            .lines
            .push(test_line(LineId(2), 2, vec![]));
        let before = project.clone();
        let mut stack = CommandStack::new();
        for target in [Some(LineId(2)), None] {
            let error = stack
                .do_command(
                    &mut project,
                    Command::MoveDeviceToLine {
                        device: DeviceId(1),
                        line: target,
                    },
                )
                .unwrap_err();
            assert!(error.to_string().contains("line 1.1"), "{error}");
            assert_eq!(project, before);
            assert!(!stack.can_undo());
        }
    }

    #[test]
    fn create_then_delete_group_address_round_trips_through_undo() {
        let mut project = test_project_with_one_device(None);
        let mut stack = CommandStack::new();
        let entry = GroupAddressEntry {
            id: GroupAddressId(1),
            source: source(),
            name: "GA".into(),
            address: GroupAddress::from_raw(1),
            central: false,
            unfiltered: false,
            range: None,
        };
        stack
            .do_command(
                &mut project,
                Command::CreateGroupAddress {
                    entry: entry.clone(),
                    installation: None,
                },
            )
            .unwrap();
        assert_eq!(project.installations[0].group_addresses.len(), 1);
        stack
            .do_command(
                &mut project,
                Command::DeleteGroupAddress {
                    id: GroupAddressId(1),
                },
            )
            .unwrap();
        assert!(project.installations[0].group_addresses.is_empty());
        stack.undo(&mut project).unwrap(); // undoes the delete -> recreates
        assert_eq!(project.installations[0].group_addresses.len(), 1);
        stack.undo(&mut project).unwrap(); // undoes the create -> empty again
        assert!(project.installations[0].group_addresses.is_empty());
    }

    #[test]
    fn deleting_and_undoing_a_middle_group_address_restores_exact_order() {
        let mut project = test_project_with_one_device(None);
        project.installations[0].group_addresses = vec![
            test_group_address_entry(GroupAddressId(1), 100),
            test_group_address_entry(GroupAddressId(2), 200),
            test_group_address_entry(GroupAddressId(3), 300),
        ];
        let mut stack = CommandStack::new();

        stack
            .do_command(
                &mut project,
                Command::DeleteGroupAddress {
                    id: GroupAddressId(2),
                },
            )
            .unwrap();
        stack.undo(&mut project).unwrap();

        let ids: Vec<_> = project.installations[0]
            .group_addresses
            .iter()
            .map(|entry| entry.id)
            .collect();
        assert_eq!(
            ids,
            vec![GroupAddressId(1), GroupAddressId(2), GroupAddressId(3)]
        );
        stack.redo(&mut project).unwrap();
        let ids: Vec<_> = project.installations[0]
            .group_addresses
            .iter()
            .map(|entry| entry.id)
            .collect();
        assert_eq!(ids, vec![GroupAddressId(1), GroupAddressId(3)]);
    }

    #[test]
    fn create_group_address_rejects_a_duplicate_address_and_leaves_the_stack_untouched() {
        let mut project = test_project_with_one_device(None);
        project.installations[0]
            .group_addresses
            .push(GroupAddressEntry {
                id: GroupAddressId(1),
                source: source(),
                name: "Existing".into(),
                address: GroupAddress::from_raw(5),
                central: false,
                unfiltered: false,
                range: None,
            });
        let mut stack = CommandStack::new();
        let result = stack.do_command(
            &mut project,
            Command::CreateGroupAddress {
                entry: GroupAddressEntry {
                    id: GroupAddressId(2),
                    source: source(),
                    name: "New".into(),
                    address: GroupAddress::from_raw(5),
                    central: false,
                    unfiltered: false,
                    range: None,
                },
                installation: None,
            },
        );
        assert!(matches!(
            result,
            Err(CommandError::Validation(
                ValidationError::DuplicateGroupAddress { .. }
            ))
        ));
        assert_eq!(project.installations[0].group_addresses.len(), 1);
        assert!(!stack.can_undo());
    }

    #[test]
    fn delete_group_address_is_rejected_while_a_com_object_still_links_to_it() {
        let mut project = test_project_with_one_device(None);
        project.installations[0]
            .group_addresses
            .push(GroupAddressEntry {
                id: GroupAddressId(1),
                source: source(),
                name: "GA".into(),
                address: GroupAddress::from_raw(1),
                central: false,
                unfiltered: false,
                range: None,
            });
        project.devices.insert_com_object(ComObjectInstance {
            id: ComObjectInstanceId(1),
            source: source(),
            device: DeviceId(1),
            number: 0,
            text: Override::Absent,
            description: Override::Absent,
            dpt: Override::Absent,
            flags: ResolvedFlags::none(),
            size: None,
            is_active: true,
            links: vec![GroupLink {
                ga: GroupAddressId(1),
                direction: Direction::Send,
            }],
            module_instance: None,
        });
        let mut stack = CommandStack::new();
        let result = stack.do_command(
            &mut project,
            Command::DeleteGroupAddress {
                id: GroupAddressId(1),
            },
        );
        assert!(matches!(
            result,
            Err(CommandError::GroupAddressInUse(GroupAddressId(1)))
        ));
        assert_eq!(project.installations[0].group_addresses.len(), 1);
        assert!(!stack.can_undo());
    }

    #[test]
    fn set_com_object_dpt_marks_layer_as_user_edit_and_undoes() {
        let mut project = test_project_with_one_device(None);
        let com = ComObjectInstance {
            id: ComObjectInstanceId(1),
            source: source(),
            device: DeviceId(1),
            number: 0,
            text: Override::Value(Resolved {
                value: Text::Literal("t".into()),
                layer: Layer::Program,
            }),
            description: Override::Absent,
            dpt: Override::Value(Resolved {
                value: DptRef {
                    main: 1,
                    sub: Some(1),
                },
                layer: Layer::Program,
            }),
            flags: ResolvedFlags {
                read: Override::Value(Resolved {
                    value: true,
                    layer: Layer::Program,
                }),
                write: Override::Absent,
                transmit: Override::Absent,
                update: Override::Absent,
                communication: Override::Value(Resolved {
                    value: true,
                    layer: Layer::Program,
                }),
                read_on_init: Override::Absent,
            },
            size: None,
            is_active: true,
            links: vec![],
            module_instance: None,
        };
        project.devices.insert_com_object(com);
        let mut stack = CommandStack::new();
        let new_dpt = DptRef {
            main: 5,
            sub: Some(1),
        };
        stack
            .do_command(
                &mut project,
                Command::SetComObjectDpt {
                    com_object: ComObjectInstanceId(1),
                    dpt: Some(new_dpt),
                },
            )
            .unwrap();
        let updated = project.devices.com_object(ComObjectInstanceId(1)).unwrap();
        assert_eq!(updated.dpt.value().unwrap().value, new_dpt);
        assert_eq!(updated.dpt.value().unwrap().layer, Layer::UserEdit);
        stack.undo(&mut project).unwrap();
        let restored = project.devices.com_object(ComObjectInstanceId(1)).unwrap();
        assert_eq!(
            restored.dpt.value().unwrap().value,
            DptRef {
                main: 1,
                sub: Some(1)
            }
        );
        assert_eq!(restored.dpt.value().unwrap().layer, Layer::Program);
    }

    #[test]
    fn set_device_description_then_undo_restores_previous_value() {
        let mut project = test_project_with_one_device(None);
        let mut stack = CommandStack::new();
        stack
            .do_command(
                &mut project,
                Command::SetDeviceDescription {
                    device: DeviceId(1),
                    description: Some("new description".into()),
                },
            )
            .unwrap();
        assert_eq!(
            project.devices.get(DeviceId(1)).unwrap().description,
            Some("new description".to_string())
        );
        stack.undo(&mut project).unwrap();
        assert_eq!(project.devices.get(DeviceId(1)).unwrap().description, None);
        stack.redo(&mut project).unwrap();
        assert_eq!(
            project.devices.get(DeviceId(1)).unwrap().description,
            Some("new description".to_string())
        );
    }

    #[test]
    fn set_com_object_description_marks_layer_as_user_edit_and_undoes() {
        let mut project = test_project_with_one_device(None);
        let com = ComObjectInstance {
            id: ComObjectInstanceId(1),
            source: source(),
            device: DeviceId(1),
            number: 0,
            text: Override::Absent,
            description: Override::Value(Resolved {
                value: Text::Literal("old".into()),
                layer: Layer::Program,
            }),
            dpt: Override::Absent,
            flags: ResolvedFlags::none(),
            size: None,
            is_active: true,
            links: vec![],
            module_instance: None,
        };
        project.devices.insert_com_object(com);
        let mut stack = CommandStack::new();
        stack
            .do_command(
                &mut project,
                Command::SetComObjectDescription {
                    com_object: ComObjectInstanceId(1),
                    description: Some("new".into()),
                },
            )
            .unwrap();
        let updated = project.devices.com_object(ComObjectInstanceId(1)).unwrap();
        assert_eq!(
            updated.description.value().unwrap().value,
            Text::Literal("new".into())
        );
        assert_eq!(updated.description.value().unwrap().layer, Layer::UserEdit);
        stack.undo(&mut project).unwrap();
        let restored = project.devices.com_object(ComObjectInstanceId(1)).unwrap();
        assert_eq!(
            restored.description.value().unwrap().value,
            Text::Literal("old".into())
        );
        assert_eq!(restored.description.value().unwrap().layer, Layer::Program);
    }

    #[test]
    fn setting_com_object_description_to_none_writes_empty_not_absent() {
        let mut project = test_project_with_one_device(None);
        let com = ComObjectInstance {
            id: ComObjectInstanceId(1),
            source: source(),
            device: DeviceId(1),
            number: 0,
            text: Override::Absent,
            description: Override::Value(Resolved {
                value: Text::Literal("old".into()),
                layer: Layer::Program,
            }),
            dpt: Override::Absent,
            flags: ResolvedFlags::none(),
            size: None,
            is_active: true,
            links: vec![],
            module_instance: None,
        };
        project.devices.insert_com_object(com);
        let mut stack = CommandStack::new();
        stack
            .do_command(
                &mut project,
                Command::SetComObjectDescription {
                    com_object: ComObjectInstanceId(1),
                    description: None,
                },
            )
            .unwrap();
        let updated = project.devices.com_object(ComObjectInstanceId(1)).unwrap();
        assert_eq!(updated.description, Override::Empty);
        stack.undo(&mut project).unwrap();
        let restored = project.devices.com_object(ComObjectInstanceId(1)).unwrap();
        assert_eq!(restored.description.value().unwrap().layer, Layer::Program);
    }

    #[test]
    fn setting_com_object_dpt_to_none_writes_empty_not_absent() {
        let mut project = test_project_with_one_device(None);
        let com = ComObjectInstance {
            id: ComObjectInstanceId(1),
            source: source(),
            device: DeviceId(1),
            number: 0,
            text: Override::Value(Resolved {
                value: Text::Literal("t".into()),
                layer: Layer::Program,
            }),
            description: Override::Absent,
            dpt: Override::Value(Resolved {
                value: DptRef {
                    main: 1,
                    sub: Some(1),
                },
                layer: Layer::Program,
            }),
            flags: ResolvedFlags::none(),
            size: None,
            is_active: true,
            links: vec![],
            module_instance: None,
        };
        project.devices.insert_com_object(com);
        let mut stack = CommandStack::new();
        stack
            .do_command(
                &mut project,
                Command::SetComObjectDpt {
                    com_object: ComObjectInstanceId(1),
                    dpt: None,
                },
            )
            .unwrap();
        let updated = project.devices.com_object(ComObjectInstanceId(1)).unwrap();
        assert_eq!(updated.dpt, Override::Empty);
        stack.undo(&mut project).unwrap();
        let restored = project.devices.com_object(ComObjectInstanceId(1)).unwrap();
        assert_eq!(restored.dpt.value().unwrap().layer, Layer::Program);
    }

    #[test]
    fn set_com_object_flag_marks_layer_as_user_edit_and_undoes() {
        let mut project = test_project_with_one_device(None);
        let com = ComObjectInstance {
            id: ComObjectInstanceId(1),
            source: source(),
            device: DeviceId(1),
            number: 0,
            text: Override::Absent,
            description: Override::Absent,
            dpt: Override::Absent,
            flags: ResolvedFlags {
                read: Override::Value(Resolved {
                    value: true,
                    layer: Layer::Program,
                }),
                ..ResolvedFlags::none()
            },
            size: None,
            is_active: true,
            links: vec![],
            module_instance: None,
        };
        project.devices.insert_com_object(com);
        let mut stack = CommandStack::new();
        stack
            .do_command(
                &mut project,
                Command::SetComObjectFlag {
                    com_object: ComObjectInstanceId(1),
                    flag: ComFlagKind::Read,
                    value: false,
                },
            )
            .unwrap();
        let updated = project.devices.com_object(ComObjectInstanceId(1)).unwrap();
        assert!(!updated.flags.read.value().unwrap().value);
        assert_eq!(updated.flags.read.value().unwrap().layer, Layer::UserEdit);
        stack.undo(&mut project).unwrap();
        let restored = project.devices.com_object(ComObjectInstanceId(1)).unwrap();
        assert!(restored.flags.read.value().unwrap().value);
        assert_eq!(restored.flags.read.value().unwrap().layer, Layer::Program);
    }

    #[test]
    fn setting_a_never_stated_com_object_flag_writes_value_not_absent() {
        let mut project = test_project_with_one_device(None);
        let com = ComObjectInstance {
            id: ComObjectInstanceId(1),
            source: source(),
            device: DeviceId(1),
            number: 0,
            text: Override::Absent,
            description: Override::Absent,
            dpt: Override::Absent,
            flags: ResolvedFlags::none(),
            size: None,
            is_active: true,
            links: vec![],
            module_instance: None,
        };
        project.devices.insert_com_object(com);
        let mut stack = CommandStack::new();
        stack
            .do_command(
                &mut project,
                Command::SetComObjectFlag {
                    com_object: ComObjectInstanceId(1),
                    flag: ComFlagKind::Communication,
                    value: true,
                },
            )
            .unwrap();
        let updated = project.devices.com_object(ComObjectInstanceId(1)).unwrap();
        assert!(updated.flags.communication.value().unwrap().value);
        assert_eq!(
            updated.flags.communication.value().unwrap().layer,
            Layer::UserEdit
        );
        stack.undo(&mut project).unwrap();
        let restored = project.devices.com_object(ComObjectInstanceId(1)).unwrap();
        assert!(!restored.flags.communication.is_present());
    }

    /// §117's command-layer acceptance test. The sixth flag rides the same
    /// generic `ComFlagKind` path as the other five, so this proves the
    /// generic path really is generic rather than five arms in a trench
    /// coat.
    #[test]
    fn setting_read_on_init_undoes_and_redoes_like_any_other_flag() {
        let mut project = test_project_with_one_device(None);
        project.devices.insert_com_object(ComObjectInstance {
            id: ComObjectInstanceId(1),
            source: source(),
            device: DeviceId(1),
            number: 0,
            text: Override::Absent,
            description: Override::Absent,
            dpt: Override::Absent,
            flags: ResolvedFlags::none(),
            size: None,
            is_active: true,
            links: vec![],
            module_instance: None,
        });
        let mut stack = CommandStack::new();
        stack
            .do_command(
                &mut project,
                Command::SetComObjectFlag {
                    com_object: ComObjectInstanceId(1),
                    flag: ComFlagKind::ReadOnInit,
                    value: true,
                },
            )
            .unwrap();
        let updated = project.devices.com_object(ComObjectInstanceId(1)).unwrap();
        assert!(updated.flags.read_on_init.value().unwrap().value);
        assert_eq!(
            updated.flags.read_on_init.value().unwrap().layer,
            Layer::UserEdit
        );
        // Setting the sixth must not have disturbed the fifth.
        assert!(!updated.flags.communication.is_present());

        stack.undo(&mut project).unwrap();
        let restored = project.devices.com_object(ComObjectInstanceId(1)).unwrap();
        assert!(!restored.flags.read_on_init.is_present());

        stack.redo(&mut project).unwrap();
        let redone = project.devices.com_object(ComObjectInstanceId(1)).unwrap();
        assert!(redone.flags.read_on_init.value().unwrap().value);
        assert_eq!(
            redone.flags.read_on_init.value().unwrap().layer,
            Layer::UserEdit
        );
    }

    #[test]
    fn create_then_delete_area_round_trips_through_undo() {
        let mut project = test_project_with_one_device(None);
        let mut stack = CommandStack::new();
        let area = Area {
            id: AreaId(1),
            source: source(),
            name: "Area 1".into(),
            address: 1,
            completion: CompletionStatus::FinishedDesign,
            lines: vec![],
        };
        stack
            .do_command(
                &mut project,
                Command::CreateArea {
                    area: area.clone(),
                    installation: None,
                },
            )
            .unwrap();
        assert_eq!(project.installations[0].topology.areas.len(), 1);
        stack
            .do_command(&mut project, Command::DeleteArea { id: AreaId(1) })
            .unwrap();
        assert!(project.installations[0].topology.areas.is_empty());
        stack.undo(&mut project).unwrap(); // undoes the delete -> recreates
        assert_eq!(project.installations[0].topology.areas.len(), 1);
        stack.undo(&mut project).unwrap(); // undoes the create -> empty again
        assert!(project.installations[0].topology.areas.is_empty());
    }

    #[test]
    fn deleting_the_middle_area_restores_its_position_on_undo_and_redo() {
        let mut project = test_project_with_one_device(None);
        let area = |id: u32| Area {
            id: AreaId(id),
            source: source(),
            name: format!("Area {id}"),
            address: id as u8,
            completion: CompletionStatus::FinishedDesign,
            lines: vec![],
        };
        project.installations[0].topology.areas = vec![area(1), area(2), area(3)];
        let mut stack = CommandStack::new();
        stack
            .do_command(&mut project, Command::DeleteArea { id: AreaId(2) })
            .unwrap();
        assert_eq!(
            project.installations[0]
                .topology
                .areas
                .iter()
                .map(|a| a.id)
                .collect::<Vec<_>>(),
            [AreaId(1), AreaId(3)],
        );
        stack.undo(&mut project).unwrap();
        assert_eq!(
            project.installations[0]
                .topology
                .areas
                .iter()
                .map(|a| a.id)
                .collect::<Vec<_>>(),
            [AreaId(1), AreaId(2), AreaId(3)],
        );
        stack.redo(&mut project).unwrap();
        assert_eq!(
            project.installations[0]
                .topology
                .areas
                .iter()
                .map(|a| a.id)
                .collect::<Vec<_>>(),
            [AreaId(1), AreaId(3)],
        );
    }

    #[test]
    fn structure_commands_refuse_ids_duplicated_across_installations_without_mutation() {
        let mut project = test_project_with_one_device(None);
        project.installations[0].topology.areas.push(Area {
            id: AreaId(1),
            source: source(),
            name: "A".into(),
            address: 1,
            completion: CompletionStatus::FinishedDesign,
            lines: vec![LineId(1)],
        });
        project.installations[0]
            .topology
            .lines
            .push(test_line(LineId(1), 1, vec![]));
        project.installations[0].buildings.push(test_building_part(
            BuildingPartId(1),
            BuildingPartType::Building,
            None,
        ));
        project.installations[0]
            .group_ranges
            .push(test_range(GroupRangeId(1), 0, 2047, None));
        let mut second = project.installations[0].clone();
        second.id = InstallationId(2);
        project.installations.push(second);

        let cases = vec![
            (
                Command::DeleteArea { id: AreaId(1) },
                CommandError::AreaPlacementAmbiguous(AreaId(1)),
            ),
            (
                Command::RenameArea {
                    id: AreaId(1),
                    name: "Other".into(),
                },
                CommandError::AreaPlacementAmbiguous(AreaId(1)),
            ),
            (
                Command::CreateLine {
                    area: AreaId(1),
                    line: test_line(LineId(2), 2, vec![]),
                },
                CommandError::AreaPlacementAmbiguous(AreaId(1)),
            ),
            (
                Command::DeleteLine { id: LineId(1) },
                CommandError::LinePlacementAmbiguous(LineId(1)),
            ),
            (
                Command::RenameLine {
                    id: LineId(1),
                    name: "Other".into(),
                },
                CommandError::LinePlacementAmbiguous(LineId(1)),
            ),
            (
                Command::MoveLineToArea {
                    id: LineId(1),
                    area: AreaId(1),
                },
                CommandError::LinePlacementAmbiguous(LineId(1)),
            ),
            (
                Command::DeleteBuildingPart {
                    id: BuildingPartId(1),
                },
                CommandError::BuildingPartPlacementAmbiguous(BuildingPartId(1)),
            ),
            (
                Command::RenameBuildingPart {
                    id: BuildingPartId(1),
                    name: "Other".into(),
                },
                CommandError::BuildingPartPlacementAmbiguous(BuildingPartId(1)),
            ),
            (
                Command::MoveBuildingPart {
                    id: BuildingPartId(1),
                    parent: None,
                },
                CommandError::BuildingPartPlacementAmbiguous(BuildingPartId(1)),
            ),
            (
                Command::CreateBuildingPart {
                    part: test_building_part(
                        BuildingPartId(2),
                        BuildingPartType::Room,
                        Some(BuildingPartId(1)),
                    ),
                    installation: None,
                },
                CommandError::BuildingPartPlacementAmbiguous(BuildingPartId(1)),
            ),
            (
                Command::DeleteGroupRange {
                    id: GroupRangeId(1),
                },
                CommandError::GroupRangePlacementAmbiguous(GroupRangeId(1)),
            ),
            (
                Command::RenameGroupRange {
                    id: GroupRangeId(1),
                    name: "Other".into(),
                },
                CommandError::GroupRangePlacementAmbiguous(GroupRangeId(1)),
            ),
            (
                Command::MoveGroupRange {
                    id: GroupRangeId(1),
                    parent: None,
                },
                CommandError::GroupRangePlacementAmbiguous(GroupRangeId(1)),
            ),
            (
                Command::CreateGroupRange {
                    range: test_range(GroupRangeId(2), 0, 99, Some(GroupRangeId(1))),
                    installation: None,
                },
                CommandError::GroupRangePlacementAmbiguous(GroupRangeId(1)),
            ),
        ];
        for (command, error) in cases {
            let mut candidate = project.clone();
            let before = format!("{candidate:#?}");
            let label = format!("{command:?}");
            let mut stack = CommandStack::new();
            assert_eq!(
                stack.do_command(&mut candidate, command),
                Err(error),
                "{label}"
            );
            assert_eq!(format!("{candidate:#?}"), before, "{label}");
            assert!(!stack.can_undo(), "{label}");
        }
    }

    #[test]
    fn failed_batch_restores_deleted_structure_and_all_original_orders() {
        let mut project = test_project_with_one_device(None);
        let area = |id: u32, lines: Vec<LineId>| Area {
            id: AreaId(id),
            source: source(),
            name: format!("A{id}"),
            address: id as u8,
            completion: CompletionStatus::FinishedDesign,
            lines,
        };
        project.installations[0].topology.areas = vec![
            area(1, vec![]),
            area(2, vec![LineId(1), LineId(2), LineId(3)]),
            area(3, vec![]),
        ];
        project.installations[0].topology.lines = (1..=3)
            .map(|id| test_line(LineId(id), id as u8, vec![]))
            .collect();
        let mut parent = test_building_part(BuildingPartId(1), BuildingPartType::Building, None);
        parent.children = vec![BuildingPartId(2), BuildingPartId(3), BuildingPartId(4)];
        project.installations[0].buildings = vec![parent];
        for id in 2..=4 {
            project.installations[0].buildings.push(test_building_part(
                BuildingPartId(id),
                BuildingPartType::Room,
                Some(BuildingPartId(1)),
            ));
        }
        let mut parent = test_range(GroupRangeId(1), 0, 2047, None);
        parent.children = vec![GroupRangeId(2), GroupRangeId(3), GroupRangeId(4)];
        project.installations[0].group_ranges = vec![
            parent,
            test_range(GroupRangeId(2), 0, 99, Some(GroupRangeId(1))),
            test_range(GroupRangeId(3), 100, 199, Some(GroupRangeId(1))),
            test_range(GroupRangeId(4), 200, 299, Some(GroupRangeId(1))),
        ];
        let original = format!("{project:#?}");
        let mut stack = CommandStack::new();
        let result = stack.do_command(
            &mut project,
            Command::Batch(vec![
                Command::DeleteLine { id: LineId(2) },
                Command::DeleteBuildingPart {
                    id: BuildingPartId(3),
                },
                Command::DeleteGroupRange {
                    id: GroupRangeId(3),
                },
                Command::DeleteArea { id: AreaId(1) },
                Command::DeleteLine { id: LineId(99) },
            ]),
        );
        assert_eq!(
            result,
            Err(CommandError::BatchItem {
                index: 4,
                source: Box::new(CommandError::LineNotFound(LineId(99))),
            })
        );
        assert_eq!(format!("{project:#?}"), original);
        assert!(!stack.can_undo());
    }

    #[test]
    fn invalid_deleted_structure_restore_positions_leave_every_list_untouched() {
        let mut project = test_project_with_one_device(None);
        project.installations[0].topology.areas.push(Area {
            id: AreaId(1),
            source: source(),
            name: "A".into(),
            address: 1,
            completion: CompletionStatus::FinishedDesign,
            lines: vec![],
        });
        project.installations[0].buildings.push(test_building_part(
            BuildingPartId(1),
            BuildingPartType::Building,
            None,
        ));
        project.installations[0]
            .group_ranges
            .push(test_range(GroupRangeId(1), 0, 2047, None));
        let area = Area {
            id: AreaId(2),
            source: source(),
            name: "B".into(),
            address: 2,
            completion: CompletionStatus::FinishedDesign,
            lines: vec![],
        };
        let cases = vec![
            (
                Command::RestoreArea {
                    area,
                    position: 3,
                    installation: InstallationId(0),
                },
                CommandError::InvalidStructurePosition {
                    kind: IdKind::Area,
                    id: 2,
                    position: 3,
                },
            ),
            (
                Command::RestoreLine {
                    area: AreaId(1),
                    line: test_line(LineId(2), 2, vec![]),
                    line_position: 3,
                    area_position: 0,
                    installation: InstallationId(0),
                },
                CommandError::InvalidStructurePosition {
                    kind: IdKind::Line,
                    id: 2,
                    position: 3,
                },
            ),
            (
                Command::RestoreLine {
                    area: AreaId(1),
                    line: test_line(LineId(2), 2, vec![]),
                    line_position: 0,
                    area_position: 3,
                    installation: InstallationId(0),
                },
                CommandError::InvalidLinePosition {
                    area: AreaId(1),
                    position: 3,
                },
            ),
            (
                Command::RestoreBuildingPart {
                    part: test_building_part(
                        BuildingPartId(2),
                        BuildingPartType::Room,
                        Some(BuildingPartId(1)),
                    ),
                    position: 3,
                    child_position: Some(0),
                    installation: InstallationId(0),
                },
                CommandError::InvalidStructurePosition {
                    kind: IdKind::BuildingPart,
                    id: 2,
                    position: 3,
                },
            ),
            (
                Command::RestoreBuildingPart {
                    part: test_building_part(
                        BuildingPartId(2),
                        BuildingPartType::Room,
                        Some(BuildingPartId(1)),
                    ),
                    position: 1,
                    child_position: Some(3),
                    installation: InstallationId(0),
                },
                CommandError::InvalidBuildingPartPosition {
                    parent: BuildingPartId(1),
                    position: 3,
                },
            ),
            (
                Command::RestoreGroupRange {
                    range: test_range(GroupRangeId(2), 0, 99, Some(GroupRangeId(1))),
                    position: 3,
                    child_position: Some(0),
                    installation: InstallationId(0),
                },
                CommandError::InvalidStructurePosition {
                    kind: IdKind::GroupRange,
                    id: 2,
                    position: 3,
                },
            ),
            (
                Command::RestoreGroupRange {
                    range: test_range(GroupRangeId(2), 0, 99, Some(GroupRangeId(1))),
                    position: 1,
                    child_position: Some(3),
                    installation: InstallationId(0),
                },
                CommandError::InvalidGroupRangePosition {
                    parent: GroupRangeId(1),
                    position: 3,
                },
            ),
        ];
        let original = format!("{project:#?}");
        let mut stack = CommandStack::new();
        for (command, error) in cases {
            assert_eq!(stack.do_command(&mut project, command), Err(error));
            assert_eq!(format!("{project:#?}"), original);
            assert!(!stack.can_undo());
        }
    }

    #[test]
    fn create_area_rejects_a_duplicate_address_and_leaves_the_stack_untouched() {
        let mut project = test_project_with_one_device(None);
        project.installations[0].topology.areas.push(Area {
            id: AreaId(1),
            source: source(),
            name: "Existing".into(),
            address: 1,
            completion: CompletionStatus::FinishedDesign,
            lines: vec![],
        });
        let mut stack = CommandStack::new();
        let result = stack.do_command(
            &mut project,
            Command::CreateArea {
                area: Area {
                    id: AreaId(2),
                    source: source(),
                    name: "New".into(),
                    address: 1,
                    completion: CompletionStatus::FinishedDesign,
                    lines: vec![],
                },
                installation: None,
            },
        );
        assert!(matches!(
            result,
            Err(CommandError::Validation(
                ValidationError::DuplicateAreaAddress { .. }
            ))
        ));
        assert!(!stack.can_undo());
    }

    #[test]
    fn delete_area_refuses_when_it_still_has_a_line() {
        let mut project = test_project_with_one_device(None);
        project.installations[0].topology.areas.push(Area {
            id: AreaId(1),
            source: source(),
            name: "A".into(),
            address: 1,
            completion: CompletionStatus::FinishedDesign,
            lines: vec![LineId(1)],
        });
        let mut stack = CommandStack::new();
        let result = stack.do_command(&mut project, Command::DeleteArea { id: AreaId(1) });
        assert_eq!(result, Err(CommandError::AreaNotEmpty(AreaId(1))));
        assert!(!stack.can_undo());
    }

    #[test]
    fn delete_unknown_area_is_rejected() {
        let mut project = test_project_with_one_device(None);
        let mut stack = CommandStack::new();
        let result = stack.do_command(&mut project, Command::DeleteArea { id: AreaId(99) });
        assert_eq!(result, Err(CommandError::AreaNotFound(AreaId(99))));
    }

    #[test]
    fn rename_area_changes_only_its_name_and_round_trips_through_undo_redo() {
        let mut project = test_project_with_one_device(None);
        project.installations[0].topology.areas.push(Area {
            id: AreaId(1),
            source: source(),
            name: "Existing area".into(),
            address: 1,
            completion: CompletionStatus::FinishedDesign,
            lines: vec![],
        });
        let original = project.installations[0].topology.clone();
        let mut expected = original.clone();
        expected.areas[0].name = "North wing".into();
        let mut stack = CommandStack::new();
        stack
            .do_command(
                &mut project,
                Command::RenameArea {
                    id: AreaId(1),
                    name: "North wing".into(),
                },
            )
            .unwrap();
        assert_eq!(project.installations[0].topology, expected);
        stack.undo(&mut project).unwrap();
        assert_eq!(project.installations[0].topology, original);
        stack.redo(&mut project).unwrap();
        assert_eq!(project.installations[0].topology, expected);

        let result = stack.do_command(
            &mut project,
            Command::RenameArea {
                id: AreaId(99),
                name: "Not an area".into(),
            },
        );
        assert_eq!(result, Err(CommandError::AreaNotFound(AreaId(99))));
        assert_eq!(project.installations[0].topology, expected);
    }

    fn test_line(id: LineId, address: u8, devices: Vec<DeviceId>) -> Line {
        Line {
            id,
            source: source(),
            name: "L".into(),
            address,
            medium_ref: "TP".into(),
            domain_address: None,
            domain_address_is_checked: None,
            ip_routing_multicast_address: None,
            multicast_ttl: None,
            completion: CompletionStatus::FinishedDesign,
            devices,
        }
    }

    fn project_with_movable_lines() -> Project {
        let mut project = test_project_with_one_device(None);
        project.installations[0].topology = Topology {
            areas: vec![
                Area {
                    id: AreaId(1),
                    source: source(),
                    name: "Source".into(),
                    address: 1,
                    completion: CompletionStatus::FinishedDesign,
                    lines: vec![LineId(10), LineId(11), LineId(12)],
                },
                Area {
                    id: AreaId(2),
                    source: source(),
                    name: "Target".into(),
                    address: 2,
                    completion: CompletionStatus::FinishedDesign,
                    lines: vec![LineId(13)],
                },
            ],
            lines: vec![
                test_line(LineId(10), 1, vec![]),
                test_line(LineId(11), 2, vec![]),
                test_line(LineId(12), 3, vec![]),
                test_line(LineId(13), 4, vec![]),
            ],
            unassigned: vec![],
        };
        project
    }

    #[test]
    fn move_line_to_area_preserves_sibling_order_through_undo_and_redo() {
        let mut project = project_with_movable_lines();
        let original = project.installations[0].topology.clone();
        let mut stack = CommandStack::new();
        stack
            .do_command(
                &mut project,
                Command::MoveLineToArea {
                    id: LineId(11),
                    area: AreaId(2),
                },
            )
            .unwrap();
        let moved = project.installations[0].topology.clone();
        assert_eq!(moved.areas[0].lines, vec![LineId(10), LineId(12)]);
        assert_eq!(moved.areas[1].lines, vec![LineId(13), LineId(11)]);
        assert_eq!(moved.lines, original.lines);
        stack.undo(&mut project).unwrap();
        assert_eq!(project.installations[0].topology, original);
        stack.redo(&mut project).unwrap();
        assert_eq!(project.installations[0].topology, moved);
    }

    #[test]
    fn move_line_to_area_rejects_duplicate_address_and_unknown_area_without_mutation() {
        let mut project = project_with_movable_lines();
        project.installations[0].topology.lines[3].address = 2;
        let original = project.installations[0].topology.clone();
        let mut stack = CommandStack::new();
        assert_eq!(
            stack.do_command(
                &mut project,
                Command::MoveLineToArea {
                    id: LineId(11),
                    area: AreaId(2),
                }
            ),
            Err(CommandError::Validation(
                ValidationError::DuplicateLineAddress {
                    address: 2,
                    existing: LineId(13),
                    new: LineId(11),
                }
            ))
        );
        assert_eq!(
            stack.do_command(
                &mut project,
                Command::MoveLineToArea {
                    id: LineId(11),
                    area: AreaId(99),
                }
            ),
            Err(CommandError::AreaNotFound(AreaId(99)))
        );
        assert_eq!(project.installations[0].topology, original);
        assert!(!stack.can_undo());
    }

    #[test]
    fn move_line_to_area_refuses_a_device_address_outside_target_prefix() {
        let mut project = project_with_movable_lines();
        let stored = IndividualAddress::new(1, 2, 9).unwrap();
        project.devices.get_mut(DeviceId(1)).unwrap().address = Some(stored);
        project.installations[0].topology.lines[1]
            .devices
            .push(DeviceId(1));
        let original = project.installations[0].topology.clone();
        let mut stack = CommandStack::new();
        assert!(
            matches!(stack.do_command(&mut project, Command::MoveLineToArea {
            id: LineId(11), area: AreaId(2),
        }), Err(CommandError::Validation(ValidationError::AddressOutsideAssignedLine {
            device: DeviceId(1), address, area: 2, line: 2,
        })) if address == stored)
        );
        assert_eq!(project.installations[0].topology, original);
        assert_eq!(
            project.devices.get(DeviceId(1)).unwrap().address,
            Some(stored)
        );
        assert!(!stack.can_undo());
    }

    #[test]
    fn move_line_to_area_repairs_imported_address_prefix_and_undo_restores_it() {
        for device_number in [0, 9] {
            let mut project = project_with_movable_lines();
            let stored = IndividualAddress::new(2, 2, device_number).unwrap();
            project.devices.get_mut(DeviceId(1)).unwrap().address = Some(stored);
            project.installations[0].topology.lines[1]
                .devices
                .push(DeviceId(1));
            let original = project.installations[0].topology.clone();
            let mut stack = CommandStack::new();
            stack
                .do_command(
                    &mut project,
                    Command::MoveLineToArea {
                        id: LineId(11),
                        area: AreaId(2),
                    },
                )
                .unwrap();
            let moved = project.installations[0].topology.clone();
            assert_eq!(
                project.devices.get(DeviceId(1)).unwrap().address,
                Some(stored)
            );
            stack.undo(&mut project).unwrap();
            assert_eq!(project.installations[0].topology, original);
            assert_eq!(
                project.devices.get(DeviceId(1)).unwrap().address,
                Some(stored)
            );
            stack.redo(&mut project).unwrap();
            assert_eq!(project.installations[0].topology, moved);
        }
    }

    #[test]
    fn move_line_to_area_refuses_ambiguous_source_membership_before_mutation() {
        let mut project = project_with_movable_lines();
        project.installations[0].topology.areas[1]
            .lines
            .push(LineId(11));
        let original = project.installations[0].topology.clone();
        let mut stack = CommandStack::new();
        assert_eq!(
            stack.do_command(
                &mut project,
                Command::MoveLineToArea {
                    id: LineId(11),
                    area: AreaId(2),
                }
            ),
            Err(CommandError::LinePlacementAmbiguous(LineId(11)))
        );
        assert_eq!(project.installations[0].topology, original);
        assert!(!stack.can_undo());
    }

    #[test]
    fn failed_line_move_batch_and_invalid_restore_leave_original_order_intact() {
        let mut project = project_with_movable_lines();
        let original = project.installations[0].topology.clone();
        let mut stack = CommandStack::new();
        assert_eq!(
            stack.do_command(
                &mut project,
                Command::RestoreLinePlacement {
                    id: LineId(11),
                    area: Some(AreaId(2)),
                    position: 2,
                }
            ),
            Err(CommandError::InvalidLinePosition {
                area: AreaId(2),
                position: 2
            })
        );
        assert_eq!(project.installations[0].topology, original);
        assert_eq!(
            stack.do_command(
                &mut project,
                Command::Batch(vec![
                    Command::MoveLineToArea {
                        id: LineId(11),
                        area: AreaId(2)
                    },
                    Command::MoveLineToArea {
                        id: LineId(12),
                        area: AreaId(99)
                    },
                ])
            ),
            Err(CommandError::BatchItem {
                index: 1,
                source: Box::new(CommandError::AreaNotFound(AreaId(99))),
            })
        );
        assert_eq!(project.installations[0].topology, original);
        assert!(!stack.can_undo());
    }

    #[test]
    fn line_move_can_repair_orphan_membership_and_undo_it_without_loss() {
        let mut project = project_with_movable_lines();
        project.installations[0].topology.areas[0].lines.remove(1);
        let original = project.installations[0].topology.clone();
        let mut stack = CommandStack::new();
        stack
            .do_command(
                &mut project,
                Command::MoveLineToArea {
                    id: LineId(11),
                    area: AreaId(2),
                },
            )
            .unwrap();
        assert_eq!(
            project.installations[0].topology.areas[1].lines,
            vec![LineId(13), LineId(11)]
        );
        stack.undo(&mut project).unwrap();
        assert_eq!(project.installations[0].topology, original);
    }

    #[test]
    fn line_move_undo_restores_an_imported_area_id_repeated_in_another_installation() {
        let mut project = project_with_movable_lines();
        let mut second = project.installations[0].clone();
        second.id = InstallationId(2);
        second.topology.areas.retain(|area| area.id == AreaId(1));
        second.topology.areas[0].lines.clear();
        second.topology.lines.clear();
        project.installations.push(second);
        let original = project.installations[0].topology.clone();
        let mut stack = CommandStack::new();
        stack
            .do_command(
                &mut project,
                Command::MoveLineToArea {
                    id: LineId(11),
                    area: AreaId(2),
                },
            )
            .unwrap();
        stack.undo(&mut project).unwrap();
        assert_eq!(project.installations[0].topology, original);
    }

    #[test]
    fn create_then_delete_line_round_trips_through_undo() {
        let mut project = test_project_with_one_device(None);
        project.installations[0].topology.areas.push(Area {
            id: AreaId(1),
            source: source(),
            name: "A".into(),
            address: 1,
            completion: CompletionStatus::FinishedDesign,
            lines: vec![],
        });
        let mut stack = CommandStack::new();
        let line = test_line(LineId(1), 1, vec![]);
        stack
            .do_command(
                &mut project,
                Command::CreateLine {
                    area: AreaId(1),
                    line: line.clone(),
                },
            )
            .unwrap();
        assert_eq!(project.installations[0].topology.lines.len(), 1);
        assert_eq!(
            project.installations[0].topology.areas[0].lines,
            vec![LineId(1)]
        );
        stack
            .do_command(&mut project, Command::DeleteLine { id: LineId(1) })
            .unwrap();
        assert!(project.installations[0].topology.lines.is_empty());
        assert!(project.installations[0].topology.areas[0].lines.is_empty());
        stack.undo(&mut project).unwrap();
        assert_eq!(project.installations[0].topology.lines.len(), 1);
        stack.undo(&mut project).unwrap();
        assert!(project.installations[0].topology.lines.is_empty());
    }

    #[test]
    fn deleting_the_middle_line_restores_both_line_and_area_sibling_order() {
        let mut project = test_project_with_one_device(None);
        project.installations[0].topology.areas.push(Area {
            id: AreaId(1),
            source: source(),
            name: "A".into(),
            address: 1,
            completion: CompletionStatus::FinishedDesign,
            lines: vec![LineId(1), LineId(2), LineId(3)],
        });
        project.installations[0].topology.lines = (1..=3)
            .map(|id| test_line(LineId(id), id as u8, vec![]))
            .collect();
        let original = project.installations[0].topology.clone();
        let mut stack = CommandStack::new();
        stack
            .do_command(&mut project, Command::DeleteLine { id: LineId(2) })
            .unwrap();
        stack.undo(&mut project).unwrap();
        assert_eq!(project.installations[0].topology, original);
        stack.redo(&mut project).unwrap();
        assert_eq!(
            project.installations[0].topology.areas[0].lines,
            [LineId(1), LineId(3)]
        );
    }

    #[test]
    fn create_line_rejects_a_duplicate_address_in_the_same_area() {
        let mut project = test_project_with_one_device(None);
        project.installations[0].topology.areas.push(Area {
            id: AreaId(1),
            source: source(),
            name: "A".into(),
            address: 1,
            completion: CompletionStatus::FinishedDesign,
            lines: vec![LineId(1)],
        });
        project.installations[0]
            .topology
            .lines
            .push(test_line(LineId(1), 1, vec![]));
        let mut stack = CommandStack::new();
        let result = stack.do_command(
            &mut project,
            Command::CreateLine {
                area: AreaId(1),
                line: test_line(LineId(2), 1, vec![]),
            },
        );
        assert!(matches!(
            result,
            Err(CommandError::Validation(
                ValidationError::DuplicateLineAddress { .. }
            ))
        ));
        assert!(!stack.can_undo());
    }

    #[test]
    fn create_line_rejects_an_unknown_area() {
        let mut project = test_project_with_one_device(None);
        let mut stack = CommandStack::new();
        let result = stack.do_command(
            &mut project,
            Command::CreateLine {
                area: AreaId(99),
                line: test_line(LineId(1), 1, vec![]),
            },
        );
        assert_eq!(result, Err(CommandError::AreaNotFound(AreaId(99))));
    }

    #[test]
    fn delete_line_refuses_when_it_still_has_a_device() {
        let mut project = test_project_with_one_device(None);
        project.installations[0].topology.areas.push(Area {
            id: AreaId(1),
            source: source(),
            name: "A".into(),
            address: 1,
            completion: CompletionStatus::FinishedDesign,
            lines: vec![LineId(1)],
        });
        project.installations[0]
            .topology
            .lines
            .push(test_line(LineId(1), 1, vec![DeviceId(1)]));
        let mut stack = CommandStack::new();
        let result = stack.do_command(&mut project, Command::DeleteLine { id: LineId(1) });
        assert_eq!(result, Err(CommandError::LineNotEmpty(LineId(1))));
    }

    #[test]
    fn delete_unknown_line_is_rejected() {
        let mut project = test_project_with_one_device(None);
        let mut stack = CommandStack::new();
        let result = stack.do_command(&mut project, Command::DeleteLine { id: LineId(99) });
        assert_eq!(result, Err(CommandError::LineNotFound(LineId(99))));
    }

    #[test]
    fn rename_line_changes_only_its_name_and_round_trips_through_undo_redo() {
        let mut project = test_project_with_one_device(None);
        project.installations[0].topology.areas.push(Area {
            id: AreaId(1),
            source: source(),
            name: "Existing area".into(),
            address: 1,
            completion: CompletionStatus::FinishedDesign,
            lines: vec![LineId(1)],
        });
        project.installations[0]
            .topology
            .lines
            .push(test_line(LineId(1), 2, vec![]));
        let original = project.installations[0].topology.clone();
        let mut expected = original.clone();
        expected.lines[0].name = "Main line".into();
        let mut stack = CommandStack::new();
        stack
            .do_command(
                &mut project,
                Command::RenameLine {
                    id: LineId(1),
                    name: "Main line".into(),
                },
            )
            .unwrap();
        assert_eq!(project.installations[0].topology, expected);
        stack.undo(&mut project).unwrap();
        assert_eq!(project.installations[0].topology, original);
        stack.redo(&mut project).unwrap();
        assert_eq!(project.installations[0].topology, expected);

        let result = stack.do_command(
            &mut project,
            Command::RenameLine {
                id: LineId(99),
                name: "Not a line".into(),
            },
        );
        assert_eq!(result, Err(CommandError::LineNotFound(LineId(99))));
        assert_eq!(project.installations[0].topology, expected);
    }

    fn project_with_line_and_unassigned_device() -> Project {
        let mut p = test_project_with_one_device(None);
        p.installations[0].topology.areas.push(Area {
            id: AreaId(1),
            source: source(),
            name: "A".into(),
            address: 1,
            completion: CompletionStatus::FinishedDesign,
            lines: vec![LineId(1)],
        });
        p.installations[0]
            .topology
            .lines
            .push(test_line(LineId(1), 1, vec![]));
        p.installations[0].topology.unassigned.push(DeviceId(1));
        p
    }

    #[test]
    fn move_device_from_unassigned_to_a_line_and_back_via_undo() {
        let mut project = project_with_line_and_unassigned_device();
        let mut stack = CommandStack::new();
        stack
            .do_command(
                &mut project,
                Command::MoveDeviceToLine {
                    device: DeviceId(1),
                    line: Some(LineId(1)),
                },
            )
            .unwrap();
        assert!(project.installations[0].topology.unassigned.is_empty());
        assert_eq!(
            project.installations[0].topology.lines[0].devices,
            vec![DeviceId(1)]
        );
        stack.undo(&mut project).unwrap();
        assert_eq!(
            project.installations[0].topology.unassigned,
            vec![DeviceId(1)]
        );
        assert!(project.installations[0].topology.lines[0]
            .devices
            .is_empty());
    }

    #[test]
    fn move_device_between_two_lines() {
        let mut project = project_with_line_and_unassigned_device();
        project.installations[0].topology.areas[0]
            .lines
            .push(LineId(2));
        project.installations[0]
            .topology
            .lines
            .push(test_line(LineId(2), 2, vec![]));
        let mut stack = CommandStack::new();
        stack
            .do_command(
                &mut project,
                Command::MoveDeviceToLine {
                    device: DeviceId(1),
                    line: Some(LineId(1)),
                },
            )
            .unwrap();
        stack
            .do_command(
                &mut project,
                Command::MoveDeviceToLine {
                    device: DeviceId(1),
                    line: Some(LineId(2)),
                },
            )
            .unwrap();
        assert!(project.installations[0].topology.lines[0]
            .devices
            .is_empty());
        assert_eq!(
            project.installations[0].topology.lines[1].devices,
            vec![DeviceId(1)]
        );
    }

    #[test]
    fn move_device_to_a_nonexistent_line_is_rejected_and_leaves_the_device_in_place() {
        let mut project = project_with_line_and_unassigned_device();
        let mut stack = CommandStack::new();
        let result = stack.do_command(
            &mut project,
            Command::MoveDeviceToLine {
                device: DeviceId(1),
                line: Some(LineId(99)),
            },
        );
        assert_eq!(result, Err(CommandError::LineNotFound(LineId(99))));
        assert_eq!(
            project.installations[0].topology.unassigned,
            vec![DeviceId(1)]
        );
        assert!(!stack.can_undo());
    }

    #[test]
    fn move_an_unknown_device_is_rejected() {
        let mut project = project_with_line_and_unassigned_device();
        let mut stack = CommandStack::new();
        let result = stack.do_command(
            &mut project,
            Command::MoveDeviceToLine {
                device: DeviceId(99),
                line: Some(LineId(1)),
            },
        );
        assert_eq!(result, Err(CommandError::DeviceNotFound(DeviceId(99))));
    }

    fn test_com_object_instance(id: ComObjectInstanceId, device: DeviceId) -> ComObjectInstance {
        ComObjectInstance {
            id,
            source: source(),
            device,
            number: 0,
            text: Override::Absent,
            description: Override::Absent,
            dpt: Override::Absent,
            flags: ResolvedFlags::none(),
            size: None,
            is_active: true,
            links: vec![],
            module_instance: None,
        }
    }

    fn test_device_instance(id: DeviceId, com_objects: Vec<ComObjectInstanceId>) -> DeviceInstance {
        DeviceInstance {
            id,
            source: source(),
            name: "New device".into(),
            description: None,
            address: None,
            product_ref: "P".into(),
            program_ref: "H".into(),
            commissioning: CommissioningState::default(),
            visibility_calculated: true,
            com_objects,
            binary_data: vec![],
        }
    }

    #[test]
    fn create_device_lands_in_unassigned_when_no_line_is_given_and_undo_removes_it() {
        let mut project = test_project_with_one_device(None);
        let mut stack = CommandStack::new();
        let com = test_com_object_instance(ComObjectInstanceId(10), DeviceId(2));
        let device = test_device_instance(DeviceId(2), vec![ComObjectInstanceId(10)]);
        stack
            .do_command(
                &mut project,
                Command::CreateDevice {
                    device: device.clone(),
                    com_objects: vec![com.clone()],
                    program_defaults: vec![],
                    installation: None,
                    line: None,
                },
            )
            .unwrap();
        assert!(project.devices.get(DeviceId(2)).is_some());
        assert!(project
            .devices
            .com_object(ComObjectInstanceId(10))
            .is_some());
        assert_eq!(
            project.installations[0].topology.unassigned,
            vec![DeviceId(2)]
        );
        stack.undo(&mut project).unwrap();
        assert!(project.devices.get(DeviceId(2)).is_none());
        assert!(project
            .devices
            .com_object(ComObjectInstanceId(10))
            .is_none());
        assert!(project.installations[0].topology.unassigned.is_empty());
    }

    #[test]
    fn create_device_on_a_line_places_it_there_and_rejects_an_unknown_line() {
        let mut project = test_project_with_one_device(None);
        project.installations[0].topology.areas.push(Area {
            id: AreaId(1),
            source: source(),
            name: "A".into(),
            address: 1,
            completion: CompletionStatus::FinishedDesign,
            lines: vec![LineId(1)],
        });
        project.installations[0]
            .topology
            .lines
            .push(test_line(LineId(1), 1, vec![]));
        let mut stack = CommandStack::new();
        stack
            .do_command(
                &mut project,
                Command::CreateDevice {
                    device: test_device_instance(DeviceId(2), vec![]),
                    com_objects: vec![],
                    program_defaults: vec![],
                    installation: None,
                    line: Some(LineId(1)),
                },
            )
            .unwrap();
        assert_eq!(
            project.installations[0].topology.lines[0].devices,
            vec![DeviceId(2)]
        );

        let result = stack.do_command(
            &mut project,
            Command::CreateDevice {
                device: test_device_instance(DeviceId(3), vec![]),
                com_objects: vec![],
                program_defaults: vec![],
                installation: None,
                line: Some(LineId(99)),
            },
        );
        assert_eq!(result, Err(CommandError::LineNotFound(LineId(99))));
        assert!(project.devices.get(DeviceId(3)).is_none());
    }

    #[test]
    fn delete_device_refuses_while_a_com_object_still_has_a_link() {
        let mut project = test_project_with_one_device(None);
        project.installations[0]
            .group_addresses
            .push(GroupAddressEntry {
                id: GroupAddressId(1),
                source: source(),
                name: "GA".into(),
                address: GroupAddress::from_raw(1),
                central: false,
                unfiltered: false,
                range: None,
            });
        let mut com = test_com_object_instance(ComObjectInstanceId(1), DeviceId(1));
        com.links.push(GroupLink {
            ga: GroupAddressId(1),
            direction: Direction::Send,
        });
        project.devices.insert_com_object(com);
        project
            .devices
            .get_mut(DeviceId(1))
            .unwrap()
            .com_objects
            .push(ComObjectInstanceId(1));

        let mut stack = CommandStack::new();
        let result = stack.do_command(&mut project, Command::DeleteDevice { id: DeviceId(1) });
        assert_eq!(result, Err(CommandError::DeviceHasLinks(DeviceId(1))));
        assert!(project.devices.get(DeviceId(1)).is_some());
        assert!(!stack.can_undo());
    }

    #[test]
    fn delete_device_refuses_while_project_structures_still_reference_it() {
        let mut project = test_project_with_one_device(None);
        project.installations[0]
            .topology
            .unassigned
            .push(DeviceId(1));
        project.installations[0].buildings.push(BuildingPart {
            id: BuildingPartId(1),
            source: source(),
            name: "Room".into(),
            number: None,
            kind: BuildingPartType::Room,
            default_line: None,
            completion: CompletionStatus::FinishedDesign,
            children: vec![],
            devices: vec![DeviceId(1)],
            parent: None,
        });
        let before = format!("{project:#?}");
        let mut stack = CommandStack::new();

        let result = stack.do_command(&mut project, Command::DeleteDevice { id: DeviceId(1) });

        assert!(result.is_err());
        assert_eq!(format!("{project:#?}"), before);
        assert!(!stack.can_undo());
    }

    #[test]
    fn deleting_then_undoing_and_redoing_preserves_values_captured_at_delete_time() {
        let mut project = test_project_with_one_device(None);
        let mut stack = CommandStack::new();
        let com = test_com_object_instance(ComObjectInstanceId(10), DeviceId(2));
        let device = test_device_instance(DeviceId(2), vec![ComObjectInstanceId(10)]);
        stack
            .do_command(
                &mut project,
                Command::CreateDevice {
                    device,
                    com_objects: vec![com],
                    program_defaults: vec![],
                    installation: None,
                    line: None,
                },
            )
            .unwrap();

        // Stand-in for `knx_productdb::enrich::apply` filling an `Absent`
        // slot after creation (design doc §3, step 3) — a direct
        // mutation, not a `Command`, exactly like the real enrichment
        // pass.
        project
            .devices
            .com_object_mut(ComObjectInstanceId(10))
            .unwrap()
            .dpt = Override::Value(Resolved {
            value: DptRef {
                main: 1,
                sub: Some(1),
            },
            layer: Layer::Program,
        });

        stack
            .do_command(&mut project, Command::DeleteDevice { id: DeviceId(2) })
            .unwrap();
        assert!(project.devices.get(DeviceId(2)).is_none());
        assert!(project
            .devices
            .com_object(ComObjectInstanceId(10))
            .is_none());

        stack.undo(&mut project).unwrap(); // undoes the delete -> recreates
        let restored = project.devices.com_object(ComObjectInstanceId(10)).unwrap();
        assert_eq!(
            restored.dpt.value().unwrap().value,
            DptRef {
                main: 1,
                sub: Some(1)
            }
        );
        assert_eq!(restored.dpt.value().unwrap().layer, Layer::Program);

        stack.redo(&mut project).unwrap(); // re-deletes
        assert!(project.devices.get(DeviceId(2)).is_none());

        stack.undo(&mut project).unwrap(); // undoes the re-delete -> recreates again
        let restored_again = project.devices.com_object(ComObjectInstanceId(10)).unwrap();
        assert_eq!(
            restored_again.dpt.value().unwrap().layer,
            Layer::Program,
            "the enriched value must survive a delete/undo/redo/undo cycle unchanged"
        );
    }

    /// The side-table half of the same promise. `program_defaults` lives
    /// beside `ComObjectInstance` rather than inside it (ADR-0027), so
    /// `DeleteDevice`'s inverse has to carry it explicitly or undo hands
    /// the user a device that looks whole and quietly is not — and a save
    /// afterwards makes that permanent.
    #[test]
    fn deleting_then_undoing_restores_the_program_defaults_the_delete_destroyed() {
        let mut project = test_project_with_one_device(None);
        let mut stack = CommandStack::new();
        let com = test_com_object_instance(ComObjectInstanceId(10), DeviceId(2));
        let device = test_device_instance(DeviceId(2), vec![ComObjectInstanceId(10)]);
        stack
            .do_command(
                &mut project,
                Command::CreateDevice {
                    device,
                    com_objects: vec![com],
                    program_defaults: vec![],
                    installation: None,
                    line: None,
                },
            )
            .unwrap();

        // Stand-in for `knx_productdb::enrich::apply` lifting a program
        // value behind an `Override::Empty` slot into the side table — a
        // direct mutation, not a `Command`, exactly like the real pass.
        let defaults = ProgramDefaults {
            text: None,
            description: None,
            dpt: Some(Resolved {
                value: DptRef {
                    main: 9,
                    sub: Some(1),
                },
                layer: Layer::Program,
            }),
        };
        project
            .devices
            .set_program_defaults(ComObjectInstanceId(10), defaults.clone());

        stack
            .do_command(&mut project, Command::DeleteDevice { id: DeviceId(2) })
            .unwrap();
        assert!(project
            .devices
            .program_defaults(ComObjectInstanceId(10))
            .is_none());

        stack.undo(&mut project).unwrap();
        assert_eq!(
            project.devices.program_defaults(ComObjectInstanceId(10)),
            Some(&defaults),
            "undo must bring the lifted program default back, not just the com object"
        );

        // And around again, so redo cannot quietly drop what undo restored.
        stack.redo(&mut project).unwrap();
        assert!(project
            .devices
            .program_defaults(ComObjectInstanceId(10))
            .is_none());
        stack.undo(&mut project).unwrap();
        assert_eq!(
            project.devices.program_defaults(ComObjectInstanceId(10)),
            Some(&defaults)
        );
    }

    /// The other direction: a com object that never had defaults must not
    /// come back from an undo carrying an empty-but-present record.
    /// `None` and "present and empty" are different claims about what the
    /// program states, and the side table exists to keep them apart.
    #[test]
    fn undoing_a_delete_does_not_invent_program_defaults_for_a_device_that_had_none() {
        let mut project = test_project_with_one_device(None);
        let mut stack = CommandStack::new();
        let com = test_com_object_instance(ComObjectInstanceId(10), DeviceId(2));
        let device = test_device_instance(DeviceId(2), vec![ComObjectInstanceId(10)]);
        stack
            .do_command(
                &mut project,
                Command::CreateDevice {
                    device,
                    com_objects: vec![com],
                    program_defaults: vec![],
                    installation: None,
                    line: None,
                },
            )
            .unwrap();

        stack
            .do_command(&mut project, Command::DeleteDevice { id: DeviceId(2) })
            .unwrap();
        stack.undo(&mut project).unwrap();
        assert!(project
            .devices
            .com_object(ComObjectInstanceId(10))
            .is_some());
        assert!(
            project
                .devices
                .program_defaults(ComObjectInstanceId(10))
                .is_none(),
            "no defaults went in, so none may come out"
        );
    }

    fn test_range(
        id: GroupRangeId,
        start: u16,
        end: u16,
        parent: Option<GroupRangeId>,
    ) -> GroupRange {
        GroupRange {
            id,
            source: source(),
            name: "R".into(),
            start: GroupAddress::from_raw(start),
            end: GroupAddress::from_raw(end),
            parent,
            children: vec![],
        }
    }

    #[test]
    fn create_then_delete_group_range_round_trips_through_undo() {
        let mut project = test_project_with_one_device(None);
        let mut stack = CommandStack::new();
        let range = test_range(GroupRangeId(1), 0, 2047, None);
        stack
            .do_command(
                &mut project,
                Command::CreateGroupRange {
                    range: range.clone(),
                    installation: None,
                },
            )
            .unwrap();
        assert_eq!(project.installations[0].group_ranges.len(), 1);
        stack
            .do_command(
                &mut project,
                Command::DeleteGroupRange {
                    id: GroupRangeId(1),
                },
            )
            .unwrap();
        assert!(project.installations[0].group_ranges.is_empty());
        stack.undo(&mut project).unwrap();
        assert_eq!(project.installations[0].group_ranges.len(), 1);
    }

    #[test]
    fn deleting_nested_group_range_restores_flat_and_parent_child_order() {
        let mut project = test_project_with_one_device(None);
        let mut parent = test_range(GroupRangeId(1), 0, 2047, None);
        parent.children = vec![GroupRangeId(2), GroupRangeId(3), GroupRangeId(4)];
        project.installations[0].group_ranges = vec![
            parent,
            test_range(GroupRangeId(2), 0, 99, Some(GroupRangeId(1))),
            test_range(GroupRangeId(3), 100, 199, Some(GroupRangeId(1))),
            test_range(GroupRangeId(4), 200, 299, Some(GroupRangeId(1))),
        ];
        let original = project.installations[0].group_ranges.clone();
        let mut stack = CommandStack::new();
        stack
            .do_command(
                &mut project,
                Command::DeleteGroupRange {
                    id: GroupRangeId(3),
                },
            )
            .unwrap();
        stack.undo(&mut project).unwrap();
        assert_eq!(project.installations[0].group_ranges, original);
        stack.redo(&mut project).unwrap();
        assert_eq!(
            project.installations[0].group_ranges[0].children,
            [GroupRangeId(2), GroupRangeId(4)]
        );
    }

    #[test]
    fn deleting_group_range_with_missing_parent_refuses_without_panic_or_mutation() {
        let mut project = test_project_with_one_device(None);
        project.installations[0].group_ranges.push(test_range(
            GroupRangeId(2),
            0,
            99,
            Some(GroupRangeId(999)),
        ));
        let original = project.installations[0].group_ranges.clone();
        let mut stack = CommandStack::new();
        assert_eq!(
            stack.do_command(
                &mut project,
                Command::DeleteGroupRange {
                    id: GroupRangeId(2)
                }
            ),
            Err(CommandError::GroupRangePlacementAmbiguous(GroupRangeId(2))),
        );
        assert_eq!(project.installations[0].group_ranges, original);
        assert!(!stack.can_undo());
    }

    #[test]
    fn deleting_group_range_with_an_unlisted_child_refuses_without_mutation() {
        let mut project = test_project_with_one_device(None);
        project.installations[0]
            .group_ranges
            .push(test_range(GroupRangeId(1), 0, 2047, None));
        project.installations[0].group_ranges.push(test_range(
            GroupRangeId(2),
            0,
            99,
            Some(GroupRangeId(1)),
        ));
        let original = project.installations[0].group_ranges.clone();
        let mut stack = CommandStack::new();
        assert_eq!(
            stack.do_command(
                &mut project,
                Command::DeleteGroupRange {
                    id: GroupRangeId(1)
                }
            ),
            Err(CommandError::GroupRangeNotEmpty(GroupRangeId(1))),
        );
        assert_eq!(project.installations[0].group_ranges, original);
        assert!(!stack.can_undo());
    }

    #[test]
    fn create_nested_group_range_registers_with_its_parent_and_undo_deregisters_it() {
        let mut project = test_project_with_one_device(None);
        project.installations[0]
            .group_ranges
            .push(test_range(GroupRangeId(1), 0, 2047, None));
        let mut stack = CommandStack::new();
        stack
            .do_command(
                &mut project,
                Command::CreateGroupRange {
                    range: test_range(GroupRangeId(2), 0, 255, Some(GroupRangeId(1))),
                    installation: None,
                },
            )
            .unwrap();
        assert_eq!(
            project.installations[0].group_ranges[0].children,
            vec![GroupRangeId(2)]
        );
        stack.undo(&mut project).unwrap();
        assert!(project.installations[0].group_ranges[0].children.is_empty());
    }

    #[test]
    fn create_group_range_rejects_a_span_outside_its_parent() {
        let mut project = test_project_with_one_device(None);
        project.installations[0]
            .group_ranges
            .push(test_range(GroupRangeId(1), 0, 255, None));
        let mut stack = CommandStack::new();
        let result = stack.do_command(
            &mut project,
            Command::CreateGroupRange {
                range: test_range(GroupRangeId(2), 0, 2047, Some(GroupRangeId(1))),
                installation: None,
            },
        );
        assert!(matches!(
            result,
            Err(CommandError::Validation(
                ValidationError::GroupRangeOutsideParent { .. }
            ))
        ));
    }

    #[test]
    fn create_group_range_rejects_an_inverted_span() {
        let mut project = test_project_with_one_device(None);
        let mut stack = CommandStack::new();
        let result = stack.do_command(
            &mut project,
            Command::CreateGroupRange {
                range: test_range(GroupRangeId(1), 255, 0, None),
                installation: None,
            },
        );
        assert!(matches!(
            result,
            Err(CommandError::Validation(
                ValidationError::GroupRangeInverted { .. }
            ))
        ));
        assert!(!stack.can_undo());
    }

    #[test]
    fn create_group_range_rejects_an_unknown_parent() {
        let mut project = test_project_with_one_device(None);
        let mut stack = CommandStack::new();
        let result = stack.do_command(
            &mut project,
            Command::CreateGroupRange {
                range: test_range(GroupRangeId(1), 0, 255, Some(GroupRangeId(99))),
                installation: None,
            },
        );
        assert_eq!(
            result,
            Err(CommandError::GroupRangeNotFound(GroupRangeId(99)))
        );
    }

    #[test]
    fn create_group_range_rejects_overlap_with_a_sibling() {
        let mut project = test_project_with_one_device(None);
        project.installations[0]
            .group_ranges
            .push(test_range(GroupRangeId(1), 0, 255, None));
        let mut stack = CommandStack::new();
        let result = stack.do_command(
            &mut project,
            Command::CreateGroupRange {
                range: test_range(GroupRangeId(2), 200, 500, None),
                installation: None,
            },
        );
        assert!(matches!(
            result,
            Err(CommandError::Validation(
                ValidationError::OverlappingGroupRange { .. }
            ))
        ));
    }

    #[test]
    fn delete_group_range_refuses_when_it_still_has_children() {
        let mut project = test_project_with_one_device(None);
        let mut range = test_range(GroupRangeId(1), 0, 2047, None);
        range.children.push(GroupRangeId(2));
        project.installations[0].group_ranges.push(range);
        let mut stack = CommandStack::new();
        let result = stack.do_command(
            &mut project,
            Command::DeleteGroupRange {
                id: GroupRangeId(1),
            },
        );
        assert_eq!(
            result,
            Err(CommandError::GroupRangeNotEmpty(GroupRangeId(1)))
        );
    }

    #[test]
    fn delete_group_range_refuses_when_a_group_address_still_uses_it() {
        let mut project = test_project_with_one_device(None);
        project.installations[0]
            .group_ranges
            .push(test_range(GroupRangeId(1), 0, 2047, None));
        project.installations[0]
            .group_addresses
            .push(GroupAddressEntry {
                id: GroupAddressId(1),
                source: source(),
                name: "GA".into(),
                address: GroupAddress::from_raw(1),
                central: false,
                unfiltered: false,
                range: Some(GroupRangeId(1)),
            });
        let mut stack = CommandStack::new();
        let result = stack.do_command(
            &mut project,
            Command::DeleteGroupRange {
                id: GroupRangeId(1),
            },
        );
        assert_eq!(result, Err(CommandError::GroupRangeInUse(GroupRangeId(1))));
    }

    #[test]
    fn rename_group_range_round_trips_through_undo() {
        let mut project = test_project_with_one_device(None);
        project.installations[0]
            .group_ranges
            .push(test_range(GroupRangeId(1), 0, 2047, None));
        project.installations[0].group_ranges[0].name = "Old name".into();
        let mut stack = CommandStack::new();
        stack
            .do_command(
                &mut project,
                Command::RenameGroupRange {
                    id: GroupRangeId(1),
                    name: "New name".into(),
                },
            )
            .unwrap();
        assert_eq!(project.installations[0].group_ranges[0].name, "New name");
        stack.undo(&mut project).unwrap();
        assert_eq!(project.installations[0].group_ranges[0].name, "Old name");
    }

    #[test]
    fn rename_unknown_group_range_is_rejected() {
        let mut project = test_project_with_one_device(None);
        let mut stack = CommandStack::new();
        let result = stack.do_command(
            &mut project,
            Command::RenameGroupRange {
                id: GroupRangeId(99),
                name: "X".into(),
            },
        );
        assert_eq!(
            result,
            Err(CommandError::GroupRangeNotFound(GroupRangeId(99)))
        );
    }

    // A misplaced imported range whose span actually belongs under range 5.
    // The editor may repair it, but undo must preserve the original bytes.
    fn project_with_misplaced_group_range() -> Project {
        let mut project = test_project_with_one_device(None);
        let mut root = test_range(GroupRangeId(1), 0, 2047, None);
        root.children = vec![GroupRangeId(2), GroupRangeId(3), GroupRangeId(4)];
        project.installations[0].group_ranges = vec![
            root,
            test_range(GroupRangeId(2), 0, 255, Some(GroupRangeId(1))),
            test_range(GroupRangeId(3), 2560, 2815, Some(GroupRangeId(1))),
            test_range(GroupRangeId(4), 512, 767, Some(GroupRangeId(1))),
            test_range(GroupRangeId(5), 2048, 4095, None),
        ];
        project
    }

    #[test]
    fn move_group_range_repairs_a_misplaced_import_without_losing_undo_order() {
        let mut project = project_with_misplaced_group_range();
        let original = project.installations[0].group_ranges.clone();
        let mut stack = CommandStack::new();
        stack
            .do_command(
                &mut project,
                Command::MoveGroupRange {
                    id: GroupRangeId(3),
                    parent: Some(GroupRangeId(5)),
                },
            )
            .unwrap();
        let moved = project.installations[0].group_ranges.clone();
        assert_eq!(moved[0].children, vec![GroupRangeId(2), GroupRangeId(4)]);
        assert_eq!(moved[4].children, vec![GroupRangeId(3)]);
        assert_eq!(moved[2].parent, Some(GroupRangeId(5)));
        stack.undo(&mut project).unwrap();
        assert_eq!(project.installations[0].group_ranges, original);
        stack.redo(&mut project).unwrap();
        assert_eq!(project.installations[0].group_ranges, moved);
    }

    #[test]
    fn group_range_move_undo_restores_an_imported_parent_id_repeated_later() {
        let mut project = project_with_misplaced_group_range();
        let mut second = project.installations[0].clone();
        second.id = InstallationId(2);
        second
            .group_ranges
            .retain(|range| range.id == GroupRangeId(1));
        second.group_ranges[0].children.clear();
        project.installations.push(second);
        let original = project.installations[0].group_ranges.clone();
        let mut stack = CommandStack::new();
        stack
            .do_command(
                &mut project,
                Command::MoveGroupRange {
                    id: GroupRangeId(3),
                    parent: Some(GroupRangeId(5)),
                },
            )
            .unwrap();
        stack.undo(&mut project).unwrap();
        assert_eq!(project.installations[0].group_ranges, original);
    }

    #[test]
    fn move_group_range_rejects_unknown_parent_cycle_and_overlapping_or_outside_spans() {
        let mut project = project_with_misplaced_group_range();
        let original = project.installations[0].group_ranges.clone();
        let mut stack = CommandStack::new();
        for (id, parent, expected) in [
            (
                GroupRangeId(3),
                Some(GroupRangeId(99)),
                CommandError::GroupRangeNotFound(GroupRangeId(99)),
            ),
            (
                GroupRangeId(1),
                Some(GroupRangeId(2)),
                CommandError::GroupRangeCycle {
                    id: GroupRangeId(1),
                    parent: GroupRangeId(2),
                },
            ),
            (
                GroupRangeId(3),
                Some(GroupRangeId(2)),
                CommandError::Validation(ValidationError::GroupRangeOutsideParent {
                    range: GroupRangeId(3),
                    parent: GroupRangeId(2),
                }),
            ),
            (
                GroupRangeId(3),
                None,
                CommandError::Validation(ValidationError::OverlappingGroupRange {
                    range: GroupRangeId(3),
                    existing: GroupRangeId(5),
                }),
            ),
        ] {
            assert_eq!(
                stack.do_command(&mut project, Command::MoveGroupRange { id, parent }),
                Err(expected)
            );
            assert_eq!(project.installations[0].group_ranges, original);
            assert!(!stack.can_undo());
        }
        let mut destination = test_range(GroupRangeId(6), 2500, 3000, Some(GroupRangeId(5)));
        destination.name = "Overlapping child".into();
        project.installations[0].group_ranges[4]
            .children
            .push(GroupRangeId(6));
        project.installations[0].group_ranges.push(destination);
        let original = project.installations[0].group_ranges.clone();
        assert_eq!(
            stack.do_command(
                &mut project,
                Command::MoveGroupRange {
                    id: GroupRangeId(3),
                    parent: Some(GroupRangeId(5)),
                }
            ),
            Err(CommandError::Validation(
                ValidationError::OverlappingGroupRange {
                    range: GroupRangeId(3),
                    existing: GroupRangeId(6),
                }
            ))
        );
        assert_eq!(project.installations[0].group_ranges, original);
    }

    #[test]
    fn failed_group_range_batch_and_direct_restore_keep_original_children() {
        let mut project = project_with_misplaced_group_range();
        let original = project.installations[0].group_ranges.clone();
        let mut stack = CommandStack::new();
        assert_eq!(
            stack.do_command(
                &mut project,
                Command::RestoreGroupRangePlacement {
                    id: GroupRangeId(3),
                    parent: Some(GroupRangeId(5)),
                    position: 1,
                }
            ),
            Err(CommandError::InvalidGroupRangePosition {
                parent: GroupRangeId(5),
                position: 1,
            })
        );
        assert_eq!(project.installations[0].group_ranges, original);
        assert_eq!(
            stack.do_command(
                &mut project,
                Command::Batch(vec![
                    Command::MoveGroupRange {
                        id: GroupRangeId(3),
                        parent: Some(GroupRangeId(5))
                    },
                    Command::MoveGroupRange {
                        id: GroupRangeId(1),
                        parent: Some(GroupRangeId(2))
                    },
                ])
            ),
            Err(CommandError::BatchItem {
                index: 1,
                source: Box::new(CommandError::GroupRangeCycle {
                    id: GroupRangeId(1),
                    parent: GroupRangeId(2),
                }),
            })
        );
        assert_eq!(project.installations[0].group_ranges, original);
        assert!(!stack.can_undo());
    }

    #[test]
    fn move_group_range_refuses_ambiguous_imported_parent_without_mutation() {
        let mut project = project_with_misplaced_group_range();
        project.installations[0].group_ranges[4]
            .children
            .push(GroupRangeId(3));
        let original = project.installations[0].group_ranges.clone();
        let mut stack = CommandStack::new();
        assert_eq!(
            stack.do_command(
                &mut project,
                Command::MoveGroupRange {
                    id: GroupRangeId(3),
                    parent: Some(GroupRangeId(5)),
                }
            ),
            Err(CommandError::GroupRangePlacementAmbiguous(GroupRangeId(3)))
        );
        assert_eq!(project.installations[0].group_ranges, original);
    }

    #[test]
    fn moving_out_of_an_imported_group_range_cycle_can_be_undone() {
        let mut project = project_with_misplaced_group_range();
        project.installations[0].group_ranges[0].parent = Some(GroupRangeId(2));
        project.installations[0].group_ranges[1]
            .children
            .push(GroupRangeId(1));
        let original = project.installations[0].group_ranges.clone();
        let mut stack = CommandStack::new();
        stack
            .do_command(
                &mut project,
                Command::MoveGroupRange {
                    id: GroupRangeId(1),
                    parent: None,
                },
            )
            .unwrap();
        let repaired = project.installations[0].group_ranges.clone();
        stack.undo(&mut project).unwrap();
        assert_eq!(project.installations[0].group_ranges, original);
        stack.redo(&mut project).unwrap();
        assert_eq!(project.installations[0].group_ranges, repaired);
    }

    #[test]
    fn update_group_address_changes_name_central_and_unfiltered_and_nothing_else() {
        let mut project = test_project_with_one_device(None);
        let address = GroupAddress::from_raw(1);
        project.installations[0]
            .group_addresses
            .push(GroupAddressEntry {
                id: GroupAddressId(1),
                source: source(),
                name: "Old name".into(),
                address,
                central: false,
                unfiltered: false,
                range: Some(GroupRangeId(1)),
            });
        let mut stack = CommandStack::new();
        stack
            .do_command(
                &mut project,
                Command::UpdateGroupAddress {
                    id: GroupAddressId(1),
                    name: "New name".into(),
                    central: true,
                    unfiltered: true,
                },
            )
            .unwrap();
        let entry = &project.installations[0].group_addresses[0];
        assert_eq!(entry.name, "New name");
        assert!(entry.central);
        assert!(entry.unfiltered);
        assert_eq!(entry.address, address);
        assert_eq!(entry.range, Some(GroupRangeId(1)));
        assert_eq!(entry.id, GroupAddressId(1));
    }

    #[test]
    fn update_group_address_inverse_carries_the_previous_values() {
        let mut project = test_project_with_one_device(None);
        project.installations[0]
            .group_addresses
            .push(GroupAddressEntry {
                id: GroupAddressId(1),
                source: source(),
                name: "Old name".into(),
                address: GroupAddress::from_raw(1),
                central: false,
                unfiltered: true,
                range: None,
            });
        let inverse = Command::UpdateGroupAddress {
            id: GroupAddressId(1),
            name: "New name".into(),
            central: true,
            unfiltered: false,
        }
        .apply(&mut project)
        .unwrap();
        assert_eq!(
            inverse,
            Command::UpdateGroupAddress {
                id: GroupAddressId(1),
                name: "Old name".into(),
                central: false,
                unfiltered: true,
            }
        );
    }

    #[test]
    fn update_unknown_group_address_is_rejected() {
        let mut project = test_project_with_one_device(None);
        let mut stack = CommandStack::new();
        let result = stack.do_command(
            &mut project,
            Command::UpdateGroupAddress {
                id: GroupAddressId(99),
                name: "X".into(),
                central: false,
                unfiltered: false,
            },
        );
        assert_eq!(
            result,
            Err(CommandError::GroupAddressNotFound(GroupAddressId(99)))
        );
    }

    #[test]
    fn update_group_address_round_trips_through_undo_and_redo() {
        let mut project = test_project_with_one_device(None);
        project.installations[0]
            .group_addresses
            .push(GroupAddressEntry {
                id: GroupAddressId(1),
                source: source(),
                name: "Old name".into(),
                address: GroupAddress::from_raw(1),
                central: false,
                unfiltered: false,
                range: None,
            });
        let mut stack = CommandStack::new();
        stack
            .do_command(
                &mut project,
                Command::UpdateGroupAddress {
                    id: GroupAddressId(1),
                    name: "New name".into(),
                    central: true,
                    unfiltered: true,
                },
            )
            .unwrap();
        assert_eq!(project.installations[0].group_addresses[0].name, "New name");
        assert!(project.installations[0].group_addresses[0].central);
        assert!(project.installations[0].group_addresses[0].unfiltered);

        stack.undo(&mut project).unwrap();
        assert_eq!(project.installations[0].group_addresses[0].name, "Old name");
        assert!(!project.installations[0].group_addresses[0].central);
        assert!(!project.installations[0].group_addresses[0].unfiltered);

        stack.redo(&mut project).unwrap();
        assert_eq!(project.installations[0].group_addresses[0].name, "New name");
        assert!(project.installations[0].group_addresses[0].central);
        assert!(project.installations[0].group_addresses[0].unfiltered);
    }

    #[test]
    fn create_group_address_rejects_an_address_outside_its_stated_range() {
        let mut project = test_project_with_one_device(None);
        project.installations[0]
            .group_ranges
            .push(test_range(GroupRangeId(1), 0, 100, None));
        let mut stack = CommandStack::new();
        let result = stack.do_command(
            &mut project,
            Command::CreateGroupAddress {
                entry: GroupAddressEntry {
                    id: GroupAddressId(1),
                    source: source(),
                    name: "GA".into(),
                    address: GroupAddress::from_raw(200),
                    central: false,
                    unfiltered: false,
                    range: Some(GroupRangeId(1)),
                },
                installation: None,
            },
        );
        assert!(matches!(
            result,
            Err(CommandError::Validation(
                ValidationError::GroupAddressOutsideRange { .. }
            ))
        ));
    }

    #[test]
    fn create_group_address_accepts_an_address_inside_its_stated_range() {
        let mut project = test_project_with_one_device(None);
        project.installations[0]
            .group_ranges
            .push(test_range(GroupRangeId(1), 0, 100, None));
        let mut stack = CommandStack::new();
        stack
            .do_command(
                &mut project,
                Command::CreateGroupAddress {
                    entry: GroupAddressEntry {
                        id: GroupAddressId(1),
                        source: source(),
                        name: "GA".into(),
                        address: GroupAddress::from_raw(50),
                        central: false,
                        unfiltered: false,
                        range: Some(GroupRangeId(1)),
                    },
                    installation: None,
                },
            )
            .unwrap();
        assert_eq!(project.installations[0].group_addresses.len(), 1);
    }

    #[test]
    fn create_group_address_rejects_an_unknown_range() {
        let mut project = test_project_with_one_device(None);
        let mut stack = CommandStack::new();
        let result = stack.do_command(
            &mut project,
            Command::CreateGroupAddress {
                entry: GroupAddressEntry {
                    id: GroupAddressId(1),
                    source: source(),
                    name: "GA".into(),
                    address: GroupAddress::from_raw(50),
                    central: false,
                    unfiltered: false,
                    range: Some(GroupRangeId(99)),
                },
                installation: None,
            },
        );
        assert_eq!(
            result,
            Err(CommandError::GroupRangeNotFound(GroupRangeId(99)))
        );
    }

    fn test_project_with_one_com_object() -> Project {
        let mut p = test_project_with_one_device(None);
        p.devices
            .get_mut(DeviceId(1))
            .unwrap()
            .com_objects
            .push(ComObjectInstanceId(1));
        p.devices.insert_com_object(ComObjectInstance {
            id: ComObjectInstanceId(1),
            source: source(),
            device: DeviceId(1),
            number: 0,
            text: Override::Value(Resolved {
                value: Text::Literal("t".into()),
                layer: Layer::Program,
            }),
            description: Override::Absent,
            dpt: Override::Absent,
            flags: ResolvedFlags::none(),
            size: None,
            is_active: true,
            links: vec![],
            module_instance: None,
        });
        p.installations[0].group_addresses.push(GroupAddressEntry {
            id: GroupAddressId(1),
            source: source(),
            name: "GA".into(),
            address: GroupAddress::from_raw(1),
            central: false,
            unfiltered: false,
            range: None,
        });
        p
    }

    #[test]
    fn both_direction_links_are_one_undoable_batch_and_roll_back_a_late_duplicate() {
        let mut project = test_project_with_one_com_object();
        let mut stack = CommandStack::new();
        let link = |direction| Command::LinkComObject {
            com_object: ComObjectInstanceId(1),
            ga: GroupAddressId(1),
            direction,
        };
        let unlink = |direction| Command::UnlinkComObject {
            com_object: ComObjectInstanceId(1),
            ga: GroupAddressId(1),
            direction,
        };
        let links = |project: &Project| {
            project
                .devices
                .com_object(ComObjectInstanceId(1))
                .unwrap()
                .links
                .clone()
        };
        stack
            .do_command(
                &mut project,
                Command::Batch(vec![link(Direction::Send), link(Direction::Receive)]),
            )
            .unwrap();
        assert_eq!(links(&project).len(), 2);
        stack.undo(&mut project).unwrap();
        assert!(links(&project).is_empty());
        stack.redo(&mut project).unwrap();
        assert_eq!(links(&project).len(), 2);
        stack
            .do_command(
                &mut project,
                Command::Batch(vec![unlink(Direction::Send), unlink(Direction::Receive)]),
            )
            .unwrap();
        assert!(links(&project).is_empty());
        stack.undo(&mut project).unwrap();
        assert_eq!(links(&project).len(), 2);
        stack
            .do_command(&mut project, unlink(Direction::Send))
            .unwrap();
        assert_eq!(
            links(&project),
            vec![GroupLink {
                ga: GroupAddressId(1),
                direction: Direction::Receive
            }]
        );
        let before = project.clone();
        let error = stack
            .do_command(
                &mut project,
                Command::Batch(vec![link(Direction::Send), link(Direction::Receive)]),
            )
            .unwrap_err();
        assert!(matches!(error, CommandError::BatchItem { index: 1, .. }));
        assert_eq!(project, before);
        stack.undo(&mut project).unwrap();
        assert_eq!(links(&project).len(), 2); // Failed batch added no undo step.
    }

    #[test]
    fn failed_unlink_both_and_undo_preserve_original_group_link_order() {
        let mut project = test_project_with_one_com_object();
        let mut other = project.installations[0].group_addresses[0].clone();
        other.id = GroupAddressId(2);
        other.address = GroupAddress::from_raw(2);
        project.installations[0].group_addresses.push(other);
        let first = GroupLink {
            ga: GroupAddressId(1),
            direction: Direction::Send,
        };
        let second = GroupLink {
            ga: GroupAddressId(2),
            direction: Direction::Receive,
        };
        project
            .devices
            .com_object_mut(ComObjectInstanceId(1))
            .unwrap()
            .links = vec![first, second];
        let before = project.clone();
        let mut stack = CommandStack::new();
        let unlink = |direction| Command::UnlinkComObject {
            com_object: ComObjectInstanceId(1),
            ga: GroupAddressId(1),
            direction,
        };
        let error = stack
            .do_command(
                &mut project,
                Command::Batch(vec![unlink(Direction::Send), unlink(Direction::Receive)]),
            )
            .unwrap_err();
        assert!(matches!(error, CommandError::BatchItem { index: 1, .. }));
        assert_eq!(project, before, "a failed batch must not reorder links");
        assert!(!stack.can_undo());

        stack
            .do_command(&mut project, unlink(Direction::Send))
            .unwrap();
        stack.undo(&mut project).unwrap();
        assert_eq!(
            project, before,
            "undo must restore the original link position"
        );
    }

    #[test]
    fn restore_group_link_rejects_an_out_of_bounds_position_without_mutation() {
        let mut project = test_project_with_one_com_object();
        let before = project.clone();
        let mut stack = CommandStack::new();
        let error = stack
            .do_command(
                &mut project,
                Command::RestoreGroupLink {
                    com_object: ComObjectInstanceId(1),
                    link: GroupLink {
                        ga: GroupAddressId(1),
                        direction: Direction::Send,
                    },
                    position: 1,
                },
            )
            .unwrap_err();
        assert_eq!(
            error,
            CommandError::InvalidGroupLinkPosition {
                com_object: ComObjectInstanceId(1),
                position: 1,
            }
        );
        assert_eq!(project, before);
        assert!(!stack.can_undo());
    }

    #[test]
    fn undo_preserves_imported_duplicate_group_links_and_redo() {
        let mut project = test_project_with_one_com_object();
        let link = GroupLink {
            ga: GroupAddressId(1),
            direction: Direction::Send,
        };
        project
            .devices
            .com_object_mut(ComObjectInstanceId(1))
            .unwrap()
            .links = vec![link, link];
        let before = project.clone();
        let mut stack = CommandStack::new();
        stack
            .do_command(
                &mut project,
                Command::UnlinkComObject {
                    com_object: ComObjectInstanceId(1),
                    ga: GroupAddressId(1),
                    direction: Direction::Send,
                },
            )
            .unwrap();
        let after_unlink = project.clone();
        assert_eq!(
            project
                .devices
                .com_object(ComObjectInstanceId(1))
                .unwrap()
                .links,
            vec![link]
        );
        stack.undo(&mut project).unwrap();
        assert_eq!(project, before);
        stack.redo(&mut project).unwrap();
        assert_eq!(project, after_unlink);
        stack.undo(&mut project).unwrap();
        assert_eq!(project, before);
    }

    #[test]
    fn unlink_both_undo_and_redo_keep_interleaved_links_in_order() {
        let mut project = test_project_with_one_com_object();
        let mut other = project.installations[0].group_addresses[0].clone();
        other.id = GroupAddressId(2);
        other.address = GroupAddress::from_raw(2);
        project.installations[0].group_addresses.push(other);
        let send = GroupLink {
            ga: GroupAddressId(1),
            direction: Direction::Send,
        };
        let unrelated = GroupLink {
            ga: GroupAddressId(2),
            direction: Direction::Receive,
        };
        let receive = GroupLink {
            ga: GroupAddressId(1),
            direction: Direction::Receive,
        };
        project
            .devices
            .com_object_mut(ComObjectInstanceId(1))
            .unwrap()
            .links = vec![send, unrelated, receive];
        let before = project.clone();
        let mut stack = CommandStack::new();
        let unlink = |direction| Command::UnlinkComObject {
            com_object: ComObjectInstanceId(1),
            ga: GroupAddressId(1),
            direction,
        };
        stack
            .do_command(
                &mut project,
                Command::Batch(vec![unlink(Direction::Send), unlink(Direction::Receive)]),
            )
            .unwrap();
        let after_unlink = project.clone();
        assert_eq!(
            project
                .devices
                .com_object(ComObjectInstanceId(1))
                .unwrap()
                .links,
            vec![unrelated]
        );
        stack.undo(&mut project).unwrap();
        assert_eq!(project, before);
        stack.redo(&mut project).unwrap();
        assert_eq!(project, after_unlink);
        stack.undo(&mut project).unwrap();
        assert_eq!(project, before);
    }

    #[test]
    fn link_then_unlink_com_object_round_trips_through_undo() {
        let mut project = test_project_with_one_com_object();
        let mut stack = CommandStack::new();
        stack
            .do_command(
                &mut project,
                Command::LinkComObject {
                    com_object: ComObjectInstanceId(1),
                    ga: GroupAddressId(1),
                    direction: Direction::Send,
                },
            )
            .unwrap();
        assert_eq!(
            project
                .devices
                .com_object(ComObjectInstanceId(1))
                .unwrap()
                .links,
            vec![GroupLink {
                ga: GroupAddressId(1),
                direction: Direction::Send
            }]
        );
        stack
            .do_command(
                &mut project,
                Command::UnlinkComObject {
                    com_object: ComObjectInstanceId(1),
                    ga: GroupAddressId(1),
                    direction: Direction::Send,
                },
            )
            .unwrap();
        assert!(project
            .devices
            .com_object(ComObjectInstanceId(1))
            .unwrap()
            .links
            .is_empty());
        stack.undo(&mut project).unwrap();
        assert_eq!(
            project
                .devices
                .com_object(ComObjectInstanceId(1))
                .unwrap()
                .links
                .len(),
            1
        );
        stack.undo(&mut project).unwrap();
        assert!(project
            .devices
            .com_object(ComObjectInstanceId(1))
            .unwrap()
            .links
            .is_empty());
    }

    #[test]
    fn link_com_object_rejects_a_nonexistent_group_address() {
        let mut project = test_project_with_one_com_object();
        let mut stack = CommandStack::new();
        let result = stack.do_command(
            &mut project,
            Command::LinkComObject {
                com_object: ComObjectInstanceId(1),
                ga: GroupAddressId(99),
                direction: Direction::Send,
            },
        );
        assert!(matches!(
            result,
            Err(CommandError::Validation(
                ValidationError::DanglingGroupLink { .. }
            ))
        ));
    }

    #[test]
    fn link_com_object_rejects_an_exact_duplicate_link() {
        let mut project = test_project_with_one_com_object();
        let mut stack = CommandStack::new();
        stack
            .do_command(
                &mut project,
                Command::LinkComObject {
                    com_object: ComObjectInstanceId(1),
                    ga: GroupAddressId(1),
                    direction: Direction::Send,
                },
            )
            .unwrap();
        let result = stack.do_command(
            &mut project,
            Command::LinkComObject {
                com_object: ComObjectInstanceId(1),
                ga: GroupAddressId(1),
                direction: Direction::Send,
            },
        );
        assert_eq!(
            result,
            Err(CommandError::LinkAlreadyExists {
                com_object: ComObjectInstanceId(1),
                ga: GroupAddressId(1),
                direction: Direction::Send,
            })
        );
    }

    #[test]
    fn link_com_object_allows_send_and_receive_on_the_same_group_address() {
        let mut project = test_project_with_one_com_object();
        let mut stack = CommandStack::new();
        stack
            .do_command(
                &mut project,
                Command::LinkComObject {
                    com_object: ComObjectInstanceId(1),
                    ga: GroupAddressId(1),
                    direction: Direction::Send,
                },
            )
            .unwrap();
        stack
            .do_command(
                &mut project,
                Command::LinkComObject {
                    com_object: ComObjectInstanceId(1),
                    ga: GroupAddressId(1),
                    direction: Direction::Receive,
                },
            )
            .unwrap();
        assert_eq!(
            project
                .devices
                .com_object(ComObjectInstanceId(1))
                .unwrap()
                .links
                .len(),
            2
        );
    }

    #[test]
    fn unlink_a_nonexistent_link_is_rejected() {
        let mut project = test_project_with_one_com_object();
        let mut stack = CommandStack::new();
        let result = stack.do_command(
            &mut project,
            Command::UnlinkComObject {
                com_object: ComObjectInstanceId(1),
                ga: GroupAddressId(1),
                direction: Direction::Send,
            },
        );
        assert_eq!(
            result,
            Err(CommandError::LinkNotFound {
                com_object: ComObjectInstanceId(1),
                ga: GroupAddressId(1),
                direction: Direction::Send,
            })
        );
    }

    fn test_building_part(
        id: BuildingPartId,
        kind: BuildingPartType,
        parent: Option<BuildingPartId>,
    ) -> BuildingPart {
        BuildingPart {
            id,
            source: source(),
            name: "B".into(),
            number: None,
            kind,
            default_line: None,
            completion: CompletionStatus::Editing,
            children: vec![],
            devices: vec![],
            parent,
        }
    }

    #[test]
    fn create_then_delete_building_part_round_trips_through_undo() {
        let mut project = test_project_with_one_device(None);
        let mut stack = CommandStack::new();
        let part = test_building_part(BuildingPartId(1), BuildingPartType::Building, None);
        stack
            .do_command(
                &mut project,
                Command::CreateBuildingPart {
                    part: part.clone(),
                    installation: None,
                },
            )
            .unwrap();
        assert_eq!(project.installations[0].buildings.len(), 1);
        stack
            .do_command(
                &mut project,
                Command::DeleteBuildingPart {
                    id: BuildingPartId(1),
                },
            )
            .unwrap();
        assert!(project.installations[0].buildings.is_empty());
        stack.undo(&mut project).unwrap(); // undoes the delete -> recreates
        assert_eq!(project.installations[0].buildings.len(), 1);
        stack.undo(&mut project).unwrap(); // undoes the create -> empty again
        assert!(project.installations[0].buildings.is_empty());
    }

    #[test]
    fn deleting_nested_building_part_restores_flat_and_parent_child_order() {
        let mut project = test_project_with_one_device(None);
        let mut parent = test_building_part(BuildingPartId(1), BuildingPartType::Building, None);
        parent.children = vec![BuildingPartId(2), BuildingPartId(3), BuildingPartId(4)];
        project.installations[0].buildings.push(parent);
        for id in 2..=4 {
            project.installations[0].buildings.push(test_building_part(
                BuildingPartId(id),
                BuildingPartType::Room,
                Some(BuildingPartId(1)),
            ));
        }
        let original = project.installations[0].buildings.clone();
        let mut stack = CommandStack::new();
        stack
            .do_command(
                &mut project,
                Command::DeleteBuildingPart {
                    id: BuildingPartId(3),
                },
            )
            .unwrap();
        stack.undo(&mut project).unwrap();
        assert_eq!(project.installations[0].buildings, original);
        stack.redo(&mut project).unwrap();
        assert_eq!(
            project.installations[0].buildings[0].children,
            [BuildingPartId(2), BuildingPartId(4)]
        );
    }

    #[test]
    fn deleting_building_part_with_missing_parent_refuses_without_panic_or_mutation() {
        let mut project = test_project_with_one_device(None);
        project.installations[0].buildings.push(test_building_part(
            BuildingPartId(2),
            BuildingPartType::Room,
            Some(BuildingPartId(999)),
        ));
        let original = project.installations[0].buildings.clone();
        let mut stack = CommandStack::new();
        assert_eq!(
            stack.do_command(
                &mut project,
                Command::DeleteBuildingPart {
                    id: BuildingPartId(2)
                }
            ),
            Err(CommandError::BuildingPartPlacementAmbiguous(
                BuildingPartId(2)
            )),
        );
        assert_eq!(project.installations[0].buildings, original);
        assert!(!stack.can_undo());
    }

    #[test]
    fn deleting_building_part_with_an_unlisted_child_refuses_without_mutation() {
        let mut project = test_project_with_one_device(None);
        project.installations[0].buildings.push(test_building_part(
            BuildingPartId(1),
            BuildingPartType::Building,
            None,
        ));
        project.installations[0].buildings.push(test_building_part(
            BuildingPartId(2),
            BuildingPartType::Room,
            Some(BuildingPartId(1)),
        ));
        let original = project.installations[0].buildings.clone();
        let mut stack = CommandStack::new();
        assert_eq!(
            stack.do_command(
                &mut project,
                Command::DeleteBuildingPart {
                    id: BuildingPartId(1)
                }
            ),
            Err(CommandError::BuildingPartNotEmpty(BuildingPartId(1))),
        );
        assert_eq!(project.installations[0].buildings, original);
        assert!(!stack.can_undo());
    }

    #[test]
    fn creating_a_nested_building_part_links_it_into_its_parents_children() {
        let mut project = test_project_with_one_device(None);
        project.installations[0].buildings.push(test_building_part(
            BuildingPartId(1),
            BuildingPartType::Building,
            None,
        ));
        let mut stack = CommandStack::new();
        stack
            .do_command(
                &mut project,
                Command::CreateBuildingPart {
                    part: test_building_part(
                        BuildingPartId(2),
                        BuildingPartType::Floor,
                        Some(BuildingPartId(1)),
                    ),
                    installation: None,
                },
            )
            .unwrap();
        assert_eq!(
            project.installations[0].buildings[0].children,
            vec![BuildingPartId(2)]
        );
        stack.undo(&mut project).unwrap();
        assert!(project.installations[0].buildings[0].children.is_empty());
    }

    #[test]
    fn create_building_part_rejects_an_unknown_parent() {
        let mut project = test_project_with_one_device(None);
        let mut stack = CommandStack::new();
        let result = stack.do_command(
            &mut project,
            Command::CreateBuildingPart {
                part: test_building_part(
                    BuildingPartId(1),
                    BuildingPartType::Room,
                    Some(BuildingPartId(99)),
                ),
                installation: None,
            },
        );
        assert_eq!(
            result,
            Err(CommandError::BuildingPartNotFound(BuildingPartId(99)))
        );
        assert!(!stack.can_undo());
    }

    #[test]
    fn delete_building_part_refuses_when_it_still_has_a_child() {
        let mut project = test_project_with_one_device(None);
        let mut parent = test_building_part(BuildingPartId(1), BuildingPartType::Building, None);
        parent.children.push(BuildingPartId(2));
        project.installations[0].buildings.push(parent);
        project.installations[0].buildings.push(test_building_part(
            BuildingPartId(2),
            BuildingPartType::Floor,
            Some(BuildingPartId(1)),
        ));
        let mut stack = CommandStack::new();
        let result = stack.do_command(
            &mut project,
            Command::DeleteBuildingPart {
                id: BuildingPartId(1),
            },
        );
        assert_eq!(
            result,
            Err(CommandError::BuildingPartNotEmpty(BuildingPartId(1)))
        );
        assert!(!stack.can_undo());
    }

    #[test]
    fn delete_building_part_refuses_when_it_still_has_a_device() {
        let mut project = test_project_with_one_device(None);
        let mut part = test_building_part(BuildingPartId(1), BuildingPartType::Room, None);
        part.devices.push(DeviceId(1));
        project.installations[0].buildings.push(part);
        let mut stack = CommandStack::new();
        let result = stack.do_command(
            &mut project,
            Command::DeleteBuildingPart {
                id: BuildingPartId(1),
            },
        );
        assert_eq!(
            result,
            Err(CommandError::BuildingPartNotEmpty(BuildingPartId(1)))
        );
    }

    #[test]
    fn delete_unknown_building_part_is_rejected() {
        let mut project = test_project_with_one_device(None);
        let mut stack = CommandStack::new();
        let result = stack.do_command(
            &mut project,
            Command::DeleteBuildingPart {
                id: BuildingPartId(99),
            },
        );
        assert_eq!(
            result,
            Err(CommandError::BuildingPartNotFound(BuildingPartId(99)))
        );
    }

    #[test]
    fn rename_building_part_then_undo_restores_previous_name() {
        let mut project = test_project_with_one_device(None);
        project.installations[0].buildings.push(test_building_part(
            BuildingPartId(1),
            BuildingPartType::Room,
            None,
        ));
        let mut stack = CommandStack::new();
        stack
            .do_command(
                &mut project,
                Command::RenameBuildingPart {
                    id: BuildingPartId(1),
                    name: "Living room".into(),
                },
            )
            .unwrap();
        assert_eq!(project.installations[0].buildings[0].name, "Living room");
        stack.undo(&mut project).unwrap();
        assert_eq!(project.installations[0].buildings[0].name, "B");
    }

    #[test]
    fn rename_unknown_building_part_is_rejected() {
        let mut project = test_project_with_one_device(None);
        let mut stack = CommandStack::new();
        let result = stack.do_command(
            &mut project,
            Command::RenameBuildingPart {
                id: BuildingPartId(99),
                name: "X".into(),
            },
        );
        assert_eq!(
            result,
            Err(CommandError::BuildingPartNotFound(BuildingPartId(99)))
        );
    }

    fn project_with_reparentable_buildings() -> Project {
        let mut project = test_project_with_one_device(None);
        let mut source = test_building_part(BuildingPartId(1), BuildingPartType::Building, None);
        source.children = vec![BuildingPartId(2), BuildingPartId(3), BuildingPartId(4)];
        project.installations[0].buildings = vec![
            source,
            test_building_part(
                BuildingPartId(2),
                BuildingPartType::Floor,
                Some(BuildingPartId(1)),
            ),
            test_building_part(
                BuildingPartId(3),
                BuildingPartType::Room,
                Some(BuildingPartId(1)),
            ),
            test_building_part(
                BuildingPartId(4),
                BuildingPartType::Room,
                Some(BuildingPartId(1)),
            ),
            test_building_part(BuildingPartId(5), BuildingPartType::Building, None),
        ];
        project
    }

    #[test]
    fn move_building_part_preserves_sibling_order_through_undo_and_redo() {
        let mut project = project_with_reparentable_buildings();
        let original = project.installations[0].buildings.clone();
        let mut stack = CommandStack::new();
        stack
            .do_command(
                &mut project,
                Command::MoveBuildingPart {
                    id: BuildingPartId(3),
                    parent: Some(BuildingPartId(5)),
                },
            )
            .unwrap();
        let moved = project.installations[0].buildings.clone();
        assert_eq!(
            moved[0].children,
            vec![BuildingPartId(2), BuildingPartId(4)]
        );
        assert_eq!(moved[4].children, vec![BuildingPartId(3)]);
        assert_eq!(moved[2].parent, Some(BuildingPartId(5)));
        stack.undo(&mut project).unwrap();
        assert_eq!(project.installations[0].buildings, original);
        stack.redo(&mut project).unwrap();
        assert_eq!(project.installations[0].buildings, moved);
    }

    #[test]
    fn building_move_undo_restores_an_imported_parent_id_repeated_later() {
        let mut project = project_with_reparentable_buildings();
        let mut second = project.installations[0].clone();
        second.id = InstallationId(2);
        second.buildings.retain(|part| part.id == BuildingPartId(1));
        second.buildings[0].children.clear();
        project.installations.push(second);
        let original = project.installations[0].buildings.clone();
        let mut stack = CommandStack::new();
        stack
            .do_command(
                &mut project,
                Command::MoveBuildingPart {
                    id: BuildingPartId(3),
                    parent: Some(BuildingPartId(5)),
                },
            )
            .unwrap();
        stack.undo(&mut project).unwrap();
        assert_eq!(project.installations[0].buildings, original);
    }

    #[test]
    fn move_building_part_refuses_missing_parent_self_and_descendant_without_mutation() {
        let mut project = project_with_reparentable_buildings();
        project.installations[0].buildings[2]
            .children
            .push(BuildingPartId(6));
        project.installations[0].buildings.push(test_building_part(
            BuildingPartId(6),
            BuildingPartType::Room,
            Some(BuildingPartId(3)),
        ));
        let original = project.installations[0].buildings.clone();
        let mut stack = CommandStack::new();
        for (parent, expected) in [
            (
                BuildingPartId(99),
                CommandError::BuildingPartNotFound(BuildingPartId(99)),
            ),
            (
                BuildingPartId(3),
                CommandError::BuildingPartCycle {
                    id: BuildingPartId(3),
                    parent: BuildingPartId(3),
                },
            ),
            (
                BuildingPartId(6),
                CommandError::BuildingPartCycle {
                    id: BuildingPartId(1),
                    parent: BuildingPartId(6),
                },
            ),
        ] {
            let id = if parent == BuildingPartId(6) {
                BuildingPartId(1)
            } else {
                BuildingPartId(3)
            };
            assert_eq!(
                stack.do_command(
                    &mut project,
                    Command::MoveBuildingPart {
                        id,
                        parent: Some(parent),
                    }
                ),
                Err(expected)
            );
            assert_eq!(project.installations[0].buildings, original);
            assert!(!stack.can_undo());
        }
    }

    #[test]
    fn move_building_part_refuses_ambiguous_imported_parent_without_mutation() {
        let mut project = project_with_reparentable_buildings();
        project.installations[0].buildings[4]
            .children
            .push(BuildingPartId(3));
        let original = project.installations[0].buildings.clone();
        let mut stack = CommandStack::new();
        assert_eq!(
            stack.do_command(
                &mut project,
                Command::MoveBuildingPart {
                    id: BuildingPartId(3),
                    parent: Some(BuildingPartId(5)),
                }
            ),
            Err(CommandError::BuildingPartPlacementAmbiguous(
                BuildingPartId(3)
            ))
        );
        assert_eq!(project.installations[0].buildings, original);
        assert!(!stack.can_undo());
    }

    #[test]
    fn failed_building_part_batch_restores_original_sibling_order() {
        let mut project = project_with_reparentable_buildings();
        let original = project.installations[0].buildings.clone();
        let mut stack = CommandStack::new();
        assert_eq!(
            stack.do_command(
                &mut project,
                Command::Batch(vec![
                    Command::MoveBuildingPart {
                        id: BuildingPartId(3),
                        parent: Some(BuildingPartId(5))
                    },
                    Command::MoveBuildingPart {
                        id: BuildingPartId(1),
                        parent: Some(BuildingPartId(4))
                    },
                ])
            ),
            Err(CommandError::BatchItem {
                index: 1,
                source: Box::new(CommandError::BuildingPartCycle {
                    id: BuildingPartId(1),
                    parent: BuildingPartId(4),
                }),
            })
        );
        assert_eq!(project.installations[0].buildings, original);
        assert!(!stack.can_undo());
    }

    #[test]
    fn building_part_restore_rejects_out_of_bounds_position_before_mutation() {
        let mut project = project_with_reparentable_buildings();
        let original = project.installations[0].buildings.clone();
        let mut stack = CommandStack::new();
        assert_eq!(
            stack.do_command(
                &mut project,
                Command::RestoreBuildingPartPlacement {
                    id: BuildingPartId(3),
                    parent: Some(BuildingPartId(5)),
                    position: 2,
                }
            ),
            Err(CommandError::InvalidBuildingPartPosition {
                parent: BuildingPartId(5),
                position: 2,
            })
        );
        assert_eq!(project.installations[0].buildings, original);
        assert!(!stack.can_undo());
    }

    #[test]
    fn moving_out_of_an_imported_building_cycle_can_be_undone_losslessly() {
        let mut project = test_project_with_one_device(None);
        let mut first = test_building_part(
            BuildingPartId(1),
            BuildingPartType::Building,
            Some(BuildingPartId(2)),
        );
        first.children = vec![BuildingPartId(2)];
        let mut second = test_building_part(
            BuildingPartId(2),
            BuildingPartType::Building,
            Some(BuildingPartId(1)),
        );
        second.children = vec![BuildingPartId(1)];
        project.installations[0].buildings = vec![first, second];
        let original = project.installations[0].buildings.clone();
        let mut stack = CommandStack::new();
        stack
            .do_command(
                &mut project,
                Command::MoveBuildingPart {
                    id: BuildingPartId(1),
                    parent: None,
                },
            )
            .unwrap();
        let repaired = project.installations[0].buildings.clone();
        assert_ne!(repaired, original);
        stack.undo(&mut project).unwrap();
        assert_eq!(project.installations[0].buildings, original);
        stack.redo(&mut project).unwrap();
        assert_eq!(project.installations[0].buildings, repaired);
    }

    #[test]
    fn move_device_into_a_building_part_and_back_via_undo() {
        let mut project = test_project_with_one_device(None);
        project.installations[0].buildings.push(test_building_part(
            BuildingPartId(1),
            BuildingPartType::Room,
            None,
        ));
        let mut stack = CommandStack::new();
        stack
            .do_command(
                &mut project,
                Command::MoveDeviceToBuildingPart {
                    device: DeviceId(1),
                    part: Some(BuildingPartId(1)),
                },
            )
            .unwrap();
        assert_eq!(
            project.installations[0].buildings[0].devices,
            vec![DeviceId(1)]
        );
        stack.undo(&mut project).unwrap();
        assert!(project.installations[0].buildings[0].devices.is_empty());
    }

    /// ADR-0038: the commands ISSUE-05's site UI will call already build a
    /// `Ground` site over two `Building`s in one installation, and moving a
    /// device from one building to the other never leaves it referenced
    /// twice — the site itself never takes a device.
    #[test]
    fn a_ground_site_holds_two_buildings_and_a_device_moves_between_them() {
        let mut project = test_project_with_one_device(None);
        let mut stack = CommandStack::new();
        for part in [
            test_building_part(BuildingPartId(1), BuildingPartType::Ground, None),
            test_building_part(
                BuildingPartId(2),
                BuildingPartType::Building,
                Some(BuildingPartId(1)),
            ),
            test_building_part(
                BuildingPartId(3),
                BuildingPartType::Building,
                Some(BuildingPartId(1)),
            ),
        ] {
            stack
                .do_command(
                    &mut project,
                    Command::CreateBuildingPart {
                        part,
                        installation: None,
                    },
                )
                .unwrap();
        }
        for target in [BuildingPartId(2), BuildingPartId(3)] {
            stack
                .do_command(
                    &mut project,
                    Command::MoveDeviceToBuildingPart {
                        device: DeviceId(1),
                        part: Some(target),
                    },
                )
                .unwrap();
        }
        let installation = &project.installations[0];
        assert_eq!(installation.buildings.len(), 3);
        let find = |id| {
            installation
                .buildings
                .iter()
                .find(|part| part.id == id)
                .unwrap()
        };
        assert_eq!(
            find(BuildingPartId(1)).children,
            vec![BuildingPartId(2), BuildingPartId(3)]
        );
        assert!(find(BuildingPartId(1)).devices.is_empty());
        assert!(find(BuildingPartId(2)).devices.is_empty());
        assert_eq!(find(BuildingPartId(3)).devices, vec![DeviceId(1)]);
        let references = installation
            .buildings
            .iter()
            .flat_map(|part| part.devices.iter())
            .filter(|device| **device == DeviceId(1))
            .count();
        assert_eq!(references, 1);
    }

    #[test]
    fn move_device_between_two_building_parts() {
        let mut project = test_project_with_one_device(None);
        project.installations[0].buildings.push(test_building_part(
            BuildingPartId(1),
            BuildingPartType::Room,
            None,
        ));
        project.installations[0].buildings.push(test_building_part(
            BuildingPartId(2),
            BuildingPartType::Room,
            None,
        ));
        let mut stack = CommandStack::new();
        stack
            .do_command(
                &mut project,
                Command::MoveDeviceToBuildingPart {
                    device: DeviceId(1),
                    part: Some(BuildingPartId(1)),
                },
            )
            .unwrap();
        stack
            .do_command(
                &mut project,
                Command::MoveDeviceToBuildingPart {
                    device: DeviceId(1),
                    part: Some(BuildingPartId(2)),
                },
            )
            .unwrap();
        assert!(project.installations[0].buildings[0].devices.is_empty());
        assert_eq!(
            project.installations[0].buildings[1].devices,
            vec![DeviceId(1)]
        );
    }

    #[test]
    fn moving_a_never_placed_device_to_none_is_a_harmless_no_op() {
        let mut project = test_project_with_one_device(None);
        let mut stack = CommandStack::new();
        stack
            .do_command(
                &mut project,
                Command::MoveDeviceToBuildingPart {
                    device: DeviceId(1),
                    part: None,
                },
            )
            .unwrap();
        stack.undo(&mut project).unwrap();
    }

    #[test]
    fn move_device_to_building_part_rejects_an_unknown_part() {
        let mut project = test_project_with_one_device(None);
        let mut stack = CommandStack::new();
        let result = stack.do_command(
            &mut project,
            Command::MoveDeviceToBuildingPart {
                device: DeviceId(1),
                part: Some(BuildingPartId(99)),
            },
        );
        assert_eq!(
            result,
            Err(CommandError::BuildingPartNotFound(BuildingPartId(99)))
        );
    }

    #[test]
    fn move_unknown_device_to_a_building_part_is_rejected() {
        let mut project = test_project_with_one_device(None);
        project.installations[0].buildings.push(test_building_part(
            BuildingPartId(1),
            BuildingPartType::Room,
            None,
        ));
        let mut stack = CommandStack::new();
        let result = stack.do_command(
            &mut project,
            Command::MoveDeviceToBuildingPart {
                device: DeviceId(99),
                part: Some(BuildingPartId(1)),
            },
        );
        assert_eq!(result, Err(CommandError::DeviceNotFound(DeviceId(99))));
    }

    fn test_group_address_entry(id: GroupAddressId, address: u16) -> GroupAddressEntry {
        GroupAddressEntry {
            id,
            source: source(),
            name: format!("GA{}", address),
            address: GroupAddress::from_raw(address),
            central: false,
            unfiltered: false,
            range: None,
        }
    }

    #[test]
    fn batch_of_two_valid_commands_applies_both_and_its_inverse_undoes_both() {
        let mut project = test_project_with_one_device(None);
        let mut stack = CommandStack::new();
        let batch = Command::Batch(vec![
            Command::CreateGroupAddress {
                entry: test_group_address_entry(GroupAddressId(1), 1),
                installation: None,
            },
            Command::CreateGroupAddress {
                entry: test_group_address_entry(GroupAddressId(2), 2),
                installation: None,
            },
        ]);
        stack.do_command(&mut project, batch).unwrap();
        assert_eq!(project.installations[0].group_addresses.len(), 2);
        stack.undo(&mut project).unwrap();
        assert!(project.installations[0].group_addresses.is_empty());
        stack.redo(&mut project).unwrap();
        assert_eq!(project.installations[0].group_addresses.len(), 2);
    }

    #[test]
    fn batch_rolls_back_completely_when_a_later_command_fails() {
        let mut project = test_project_with_one_device(None);
        // `Project` derives `PartialEq` but not `Clone`; a separately-built
        // instance from the same deterministic helper is equally valid as
        // the untouched-baseline to compare against.
        let before = test_project_with_one_device(None);
        let mut stack = CommandStack::new();
        let batch = Command::Batch(vec![
            Command::CreateGroupAddress {
                entry: test_group_address_entry(GroupAddressId(1), 1),
                installation: None,
            },
            Command::DeleteDevice { id: DeviceId(99) },
            Command::CreateGroupAddress {
                entry: test_group_address_entry(GroupAddressId(2), 2),
                installation: None,
            },
        ]);
        let result = stack.do_command(&mut project, batch);
        assert_eq!(
            result,
            Err(CommandError::BatchItem {
                index: 1,
                source: Box::new(CommandError::DeviceNotFound(DeviceId(99))),
            })
        );
        assert_eq!(project, before);
        assert!(!stack.can_undo());
    }

    #[test]
    fn empty_batch_is_a_no_op_and_inverts_to_an_empty_batch() {
        let mut project = test_project_with_one_device(None);
        let before = test_project_with_one_device(None);
        let mut stack = CommandStack::new();
        assert_eq!(
            Command::Batch(vec![]).apply(&mut project).unwrap(),
            Command::Batch(vec![])
        );
        assert_eq!(project, before);
        stack
            .do_command(&mut project, Command::Batch(vec![]))
            .unwrap();
        assert_eq!(project, before);
        stack.undo(&mut project).unwrap();
        assert_eq!(project, before);
    }

    #[test]
    fn set_parameter_value_creates_a_new_instance_when_none_exists_and_undo_removes_it() {
        let mut project = test_project_with_one_device(None);
        let before_len = project.installations[0].parameters.len();
        let inverse = Command::SetParameterValue {
            id: ParameterInstanceId(1),
            device: DeviceId(1),
            ets_id: "M-1_P-1_R-1".into(),
            raw: "7".into(),
        }
        .apply(&mut project)
        .unwrap();
        assert_eq!(
            inverse,
            Command::RestoreParameterValue {
                id: ParameterInstanceId(1),
                device: DeviceId(1),
                ets_id: "M-1_P-1_R-1".into(),
                raw: None,
            }
        );
        assert_eq!(project.installations[0].parameters.len(), before_len + 1);
        let created = &project.installations[0].parameters[0];
        assert_eq!(created.raw, "7");
        assert_eq!(created.source.ets_id, "M-1_P-1_R-1");
        // The device's own `source.path` ("t", from `test_project_with_one_device`),
        // not a fresh or empty one.
        assert_eq!(created.source.path, "t");

        inverse.apply(&mut project).unwrap();
        assert_eq!(project.installations[0].parameters.len(), before_len);
    }

    #[test]
    fn set_parameter_value_overwrites_an_existing_instance_and_undo_restores_the_same_id() {
        let mut project = test_project_with_one_device(None);
        project.installations[0].parameters.push(ParameterInstance {
            id: ParameterInstanceId(9),
            device: DeviceId(1),
            source: SourceRef {
                path: "t".into(),
                ets_id: "M-1_P-1_R-1".into(),
            },
            raw: "3".into(),
        });
        let inverse = Command::SetParameterValue {
            id: ParameterInstanceId(1), // a fresh id — must be ignored, an entry already exists
            device: DeviceId(1),
            ets_id: "M-1_P-1_R-1".into(),
            raw: "7".into(),
        }
        .apply(&mut project)
        .unwrap();
        assert_eq!(
            inverse,
            Command::RestoreParameterValue {
                id: ParameterInstanceId(9),
                device: DeviceId(1),
                ets_id: "M-1_P-1_R-1".into(),
                raw: Some("3".into()),
            }
        );
        assert_eq!(project.installations[0].parameters.len(), 1);
        assert_eq!(project.installations[0].parameters[0].raw, "7");
        assert_eq!(
            project.installations[0].parameters[0].id,
            ParameterInstanceId(9)
        );

        inverse.apply(&mut project).unwrap();
        assert_eq!(project.installations[0].parameters.len(), 1);
        let restored = &project.installations[0].parameters[0];
        assert_eq!(restored.raw, "3");
        // No new `ParameterInstanceId` was allocated for an update.
        assert_eq!(restored.id, ParameterInstanceId(9));
    }

    #[test]
    fn set_parameter_value_against_an_unknown_device_is_rejected() {
        let mut project = test_project_with_one_device(None);
        let result = Command::SetParameterValue {
            id: ParameterInstanceId(1),
            device: DeviceId(99),
            ets_id: "M-1_P-1_R-1".into(),
            raw: "7".into(),
        }
        .apply(&mut project);
        assert_eq!(result, Err(CommandError::DeviceNotFound(DeviceId(99))));
        assert!(project.installations[0].parameters.is_empty());
    }

    #[test]
    fn set_parameter_value_do_undo_redo_round_trips_through_the_command_stack() {
        let mut project = test_project_with_one_device(None);
        let mut stack = CommandStack::new();

        // Load the redo stack before the `do_command` under test, so the
        // `!stack.can_redo()` assertion below is load-bearing: on a fresh
        // stack the redo list starts empty, and an assertion that a stack
        // clears something already empty passes whether or not the clear
        // actually happened.
        stack
            .do_command(
                &mut project,
                Command::SetParameterValue {
                    id: ParameterInstanceId(1),
                    device: DeviceId(1),
                    ets_id: "M-1_P-1_R-1".into(),
                    raw: "1".into(),
                },
            )
            .unwrap();
        stack.undo(&mut project).unwrap();
        assert!(stack.can_redo());

        stack
            .do_command(
                &mut project,
                Command::SetParameterValue {
                    id: ParameterInstanceId(1),
                    device: DeviceId(1),
                    ets_id: "M-1_P-1_R-1".into(),
                    raw: "7".into(),
                },
            )
            .unwrap();
        assert!(stack.can_undo());
        assert!(!stack.can_redo());
        assert_eq!(project.installations[0].parameters.len(), 1);
        assert_eq!(project.installations[0].parameters[0].raw, "7");
        assert_eq!(
            project.installations[0].parameters[0].id,
            ParameterInstanceId(1)
        );

        stack.undo(&mut project).unwrap();
        assert!(project.installations[0].parameters.is_empty());

        stack.redo(&mut project).unwrap();
        assert_eq!(project.installations[0].parameters.len(), 1);
        assert_eq!(project.installations[0].parameters[0].raw, "7");
        assert_eq!(
            project.installations[0].parameters[0].id,
            ParameterInstanceId(1)
        );
        assert_eq!(
            project.installations[0].parameters[0].source.ets_id,
            "M-1_P-1_R-1"
        );
    }

    #[test]
    fn set_group_address_style_do_undo_redo_round_trips_through_the_command_stack() {
        let mut project = test_project_with_one_device(None);
        project.installations[0]
            .group_addresses
            .push(test_group_address_entry(GroupAddressId(1), u16::MAX));
        assert_eq!(
            project.info.group_address_style,
            GroupAddressStyle::ThreeLevel
        );
        let mut stack = CommandStack::new();

        stack
            .do_command(
                &mut project,
                Command::SetGroupAddressStyle {
                    style: GroupAddressStyle::Free,
                },
            )
            .unwrap();
        assert_eq!(project.info.group_address_style, GroupAddressStyle::Free);
        assert!(stack.can_undo());
        assert!(!stack.can_redo());

        stack.undo(&mut project).unwrap();
        assert_eq!(
            project.info.group_address_style,
            GroupAddressStyle::ThreeLevel
        );
        assert!(stack.can_redo());

        stack.redo(&mut project).unwrap();
        assert_eq!(project.info.group_address_style, GroupAddressStyle::Free);
    }

    /// This fires only if the check wrongly *rejects* a representable
    /// address — the opposite failure direction from what its old name
    /// claimed. `installations[1]`'s address is `u16::MAX`, which
    /// `fits_style` accepts under every style (see that function's own
    /// exhaustive proof), so this test cannot observe whether the loop
    /// actually visits installation two: with the guard deleted, reading
    /// only `installations[0]`, or checking every installation, the
    /// result is `Ok` either way. What it *does* show is that a real,
    /// multi-installation project's addresses all pass the check when
    /// they should. Coverage for the bit-layout regression the guard
    /// exists to catch lives in `address.rs`'s exhaustive
    /// `format`/`parse` round-trip test, not here.
    #[test]
    fn set_group_address_style_accepts_every_installations_addresses() {
        let mut project = test_project_with_one_device(None);
        project.installations[0]
            .group_addresses
            .push(test_group_address_entry(GroupAddressId(1), 1));
        let mut second = project.installations[0].clone();
        second.id = InstallationId(1);
        second.group_addresses = vec![test_group_address_entry(GroupAddressId(2), u16::MAX)];
        project.installations.push(second);

        let result = Command::SetGroupAddressStyle {
            style: GroupAddressStyle::TwoLevel,
        }
        .apply(&mut project);
        assert!(result.is_ok());
        assert_eq!(
            project.info.group_address_style,
            GroupAddressStyle::TwoLevel
        );
    }

    /// There is no matching "...is refused because an address does not
    /// fit" test: as `GroupAddress::fits_style`'s own doc comment proves
    /// exhaustively, no `u16` value fails to fit any style, so
    /// `CommandError::GroupAddressDoesNotFitStyle` cannot actually be
    /// triggered through `apply` with real data. This test instead pins
    /// down its `Display` wording directly, so the message stays
    /// actionable if the variant is ever constructed (a future change to
    /// the bit layout, or a manually-built error in a test like this one).
    #[test]
    fn group_address_does_not_fit_style_error_names_the_offender() {
        let err = CommandError::GroupAddressDoesNotFitStyle {
            id: GroupAddressId(7),
            raw: 42,
            style: GroupAddressStyle::TwoLevel,
        };
        assert_eq!(
            err.to_string(),
            "group address 7 (raw value 42) does not fit style TwoLevel, refusing the whole restyle"
        );
    }

    /// Sibling of `group_address_does_not_fit_style_error_names_the_offender`,
    /// same reason it exists: `CommandError::GroupRangeDoesNotFitStyle`
    /// cannot be triggered through `apply` with real data either (a
    /// `GroupRange`'s `start`/`end` are `GroupAddress` values, subject to
    /// the same exhaustive `fits_style` proof), so this pins the `Display`
    /// wording directly instead.
    #[test]
    fn group_range_does_not_fit_style_error_names_the_offender() {
        let err = CommandError::GroupRangeDoesNotFitStyle {
            id: GroupRangeId(9),
            raw: 99,
            style: GroupAddressStyle::ThreeLevel,
        };
        assert_eq!(
            err.to_string(),
            "group range 9 (boundary raw value 99) does not fit style ThreeLevel, refusing the whole restyle"
        );
    }

    #[test]
    fn readdress_group_address_preserves_identity_and_is_reversible() {
        let mut project = test_project_with_one_device(None);
        project.installations[0]
            .group_addresses
            .push(test_group_address_entry(GroupAddressId(1), 100));
        let mut stack = CommandStack::new();

        stack
            .do_command(
                &mut project,
                Command::ReaddressGroupAddress {
                    id: GroupAddressId(1),
                    address: GroupAddress::from_raw(200),
                    range: None,
                },
            )
            .unwrap();

        let entry = &project.installations[0].group_addresses[0];
        assert_eq!(entry.id, GroupAddressId(1));
        assert_eq!(entry.address, GroupAddress::from_raw(200));
        stack.undo(&mut project).unwrap();
        assert_eq!(
            project.installations[0].group_addresses[0].address,
            GroupAddress::from_raw(100)
        );
    }

    #[test]
    fn readdress_group_address_rejects_a_duplicate_target() {
        let mut project = test_project_with_one_device(None);
        project.installations[0]
            .group_addresses
            .push(test_group_address_entry(GroupAddressId(1), 100));
        project.installations[0]
            .group_addresses
            .push(test_group_address_entry(GroupAddressId(2), 200));

        let result = Command::ReaddressGroupAddress {
            id: GroupAddressId(1),
            address: GroupAddress::from_raw(200),
            range: None,
        }
        .apply(&mut project);

        assert!(matches!(
            result,
            Err(CommandError::Validation(
                ValidationError::DuplicateGroupAddress { .. }
            ))
        ));
        assert_eq!(
            project.installations[0].group_addresses[0].address,
            GroupAddress::from_raw(100)
        );
    }
}

/// ADR-0039 phase 1: every id-inserting command refuses an id already in
/// use, and `ReserveIds` only ever raises the allocator.
#[cfg(test)]
mod id_integrity_tests {
    use super::*;
    use crate::building::BuildingPartType;
    use crate::commissioning::{CommissioningState, CompletionStatus};
    use crate::flags::ResolvedFlags;
    use crate::ids::{InstallationId, SourceRef};
    use crate::string_table::Language;
    use crate::topology::Topology;

    fn source() -> SourceRef {
        SourceRef {
            path: "t".into(),
            ets_id: "t".into(),
        }
    }

    fn project() -> Project {
        let mut p = Project::new(Language("en".into()));
        p.installations.push(Installation {
            id: InstallationId(0),
            name: "I".into(),
            default_line: None,
            multicast_address: None,
            completion: CompletionStatus::FinishedDesign,
            topology: Topology {
                areas: vec![],
                lines: vec![],
                unassigned: vec![],
            },
            buildings: vec![],
            group_ranges: vec![],
            group_addresses: vec![],
            parameters: vec![],
        });
        p
    }

    fn ga(id: u32, raw: u16) -> GroupAddressEntry {
        GroupAddressEntry {
            id: GroupAddressId(id),
            source: source(),
            name: format!("GA{raw}"),
            address: GroupAddress::from_raw(raw),
            central: false,
            unfiltered: false,
            range: None,
        }
    }

    fn area(id: u32, address: u8) -> Area {
        Area {
            id: AreaId(id),
            source: source(),
            name: format!("Area {address}"),
            address,
            completion: CompletionStatus::FinishedDesign,
            lines: vec![],
        }
    }

    fn line(id: u32, address: u8) -> Line {
        Line {
            id: LineId(id),
            source: source(),
            name: format!("Line {address}"),
            address,
            medium_ref: "TP".into(),
            domain_address: None,
            domain_address_is_checked: None,
            ip_routing_multicast_address: None,
            multicast_ttl: None,
            completion: CompletionStatus::FinishedDesign,
            devices: vec![],
        }
    }

    fn range(id: u32, start: u16, end: u16) -> GroupRange {
        GroupRange {
            id: GroupRangeId(id),
            source: source(),
            name: "R".into(),
            start: GroupAddress::from_raw(start),
            end: GroupAddress::from_raw(end),
            parent: None,
            children: vec![],
        }
    }

    fn part(id: u32) -> BuildingPart {
        BuildingPart {
            id: BuildingPartId(id),
            source: source(),
            name: "B".into(),
            number: None,
            kind: BuildingPartType::Building,
            default_line: None,
            completion: CompletionStatus::Editing,
            children: vec![],
            devices: vec![],
            parent: None,
        }
    }

    fn com(id: u32, device: u32) -> ComObjectInstance {
        ComObjectInstance {
            id: ComObjectInstanceId(id),
            source: source(),
            device: DeviceId(device),
            number: 0,
            text: Override::Absent,
            description: Override::Absent,
            dpt: Override::Absent,
            flags: ResolvedFlags::none(),
            size: None,
            is_active: true,
            links: vec![],
            module_instance: None,
        }
    }

    fn create_device(id: u32, name: &str, coms: &[u32]) -> Command {
        Command::CreateDevice {
            device: DeviceInstance {
                id: DeviceId(id),
                source: source(),
                name: name.into(),
                description: None,
                address: None,
                product_ref: "P".into(),
                program_ref: "H".into(),
                commissioning: CommissioningState::default(),
                visibility_calculated: true,
                com_objects: coms.iter().map(|&c| ComObjectInstanceId(c)).collect(),
                binary_data: vec![],
            },
            com_objects: coms.iter().map(|&c| com(c, id)).collect(),
            program_defaults: vec![],
            installation: None,
            line: None,
        }
    }

    /// Applies `first`, then asserts `second` is refused with
    /// `IdInUse { kind, id }` and leaves the project exactly as `first` left it.
    fn assert_second_is_refused(first: Command, second: Command, kind: IdKind, id: u32) {
        let mut p = project();
        let mut stack = CommandStack::new();
        stack
            .do_command(&mut p, first)
            .expect("first create applies");
        let before = format!("{p:#?}");
        let result = stack.do_command(&mut p, second);
        assert_eq!(result, Err(CommandError::IdInUse { kind, id }));
        assert_eq!(
            format!("{p:#?}"),
            before,
            "a refused command must not mutate"
        );
    }

    #[test]
    fn create_group_address_refuses_an_id_in_use() {
        assert_second_is_refused(
            Command::CreateGroupAddress {
                entry: ga(1, 1),
                installation: None,
            },
            Command::CreateGroupAddress {
                entry: ga(1, 2),
                installation: None,
            },
            IdKind::GroupAddress,
            1,
        );
    }

    #[test]
    fn create_area_refuses_an_id_in_use() {
        assert_second_is_refused(
            Command::CreateArea {
                area: area(1, 1),
                installation: None,
            },
            Command::CreateArea {
                area: area(1, 2),
                installation: None,
            },
            IdKind::Area,
            1,
        );
    }

    #[test]
    fn create_line_refuses_an_id_in_use_even_in_another_area() {
        assert_second_is_refused(
            Command::Batch(vec![
                Command::CreateArea {
                    area: area(1, 1),
                    installation: None,
                },
                Command::CreateArea {
                    area: area(2, 2),
                    installation: None,
                },
                Command::CreateLine {
                    area: AreaId(1),
                    line: line(1, 1),
                },
            ]),
            Command::CreateLine {
                area: AreaId(2),
                line: line(1, 1),
            },
            IdKind::Line,
            1,
        );
    }

    #[test]
    fn create_group_range_refuses_an_id_in_use() {
        assert_second_is_refused(
            Command::CreateGroupRange {
                range: range(1, 0, 2047),
                installation: None,
            },
            Command::CreateGroupRange {
                range: range(1, 2048, 4095),
                installation: None,
            },
            IdKind::GroupRange,
            1,
        );
    }

    #[test]
    fn create_building_part_refuses_an_id_in_use() {
        assert_second_is_refused(
            Command::CreateBuildingPart {
                part: part(1),
                installation: None,
            },
            Command::CreateBuildingPart {
                part: part(1),
                installation: None,
            },
            IdKind::BuildingPart,
            1,
        );
    }

    #[test]
    fn create_device_refuses_a_device_id_in_use_instead_of_overwriting_it() {
        assert_second_is_refused(
            create_device(1, "first", &[]),
            create_device(1, "second", &[]),
            IdKind::Device,
            1,
        );
    }

    #[test]
    fn create_device_refuses_a_com_object_id_in_use() {
        assert_second_is_refused(
            create_device(1, "first", &[7]),
            create_device(2, "second", &[7]),
            IdKind::ComObjectInstance,
            7,
        );
    }

    #[test]
    fn create_device_refuses_the_same_com_object_id_twice_in_one_command() {
        let mut p = project();
        let result = create_device(1, "d", &[7, 7]).apply(&mut p);
        assert_eq!(
            result,
            Err(CommandError::IdInUse {
                kind: IdKind::ComObjectInstance,
                id: 7,
            })
        );
        assert!(p.devices.get(DeviceId(1)).is_none());
    }

    #[test]
    fn a_new_parameter_value_refuses_an_instance_id_in_use() {
        let set = |id: u32, ets_id: &str| Command::SetParameterValue {
            id: ParameterInstanceId(id),
            device: DeviceId(1),
            ets_id: ets_id.into(),
            raw: "1".into(),
        };
        assert_second_is_refused(
            Command::Batch(vec![create_device(1, "d", &[]), set(1, "P-1")]),
            set(1, "P-2"),
            IdKind::ParameterInstance,
            1,
        );
    }

    #[test]
    fn overwriting_an_existing_parameter_value_is_not_an_id_clash() {
        let mut p = project();
        let mut stack = CommandStack::new();
        let set = |raw: &str| Command::SetParameterValue {
            id: ParameterInstanceId(1),
            device: DeviceId(1),
            ets_id: "P-1".into(),
            raw: raw.into(),
        };
        stack
            .do_command(
                &mut p,
                Command::Batch(vec![create_device(1, "d", &[]), set("1")]),
            )
            .unwrap();
        stack.do_command(&mut p, set("2")).unwrap();
        assert_eq!(p.installations[0].parameters.len(), 1);
        assert_eq!(p.installations[0].parameters[0].raw, "2");
    }

    #[test]
    fn delete_then_undo_still_restores_the_same_id() {
        // The `Restore*`/inverse forms re-insert ids their own forward
        // command just freed; the uniqueness check must not block them.
        let mut p = project();
        let mut stack = CommandStack::new();
        stack
            .do_command(
                &mut p,
                Command::CreateGroupAddress {
                    entry: ga(1, 1),
                    installation: None,
                },
            )
            .unwrap();
        stack
            .do_command(
                &mut p,
                Command::DeleteGroupAddress {
                    id: GroupAddressId(1),
                },
            )
            .unwrap();
        stack.undo(&mut p).unwrap();
        assert_eq!(p.installations[0].group_addresses.len(), 1);
        stack.undo(&mut p).unwrap();
        stack.redo(&mut p).unwrap();
        assert_eq!(p.installations[0].group_addresses[0].id, GroupAddressId(1));
    }

    #[test]
    fn reserve_ids_raises_each_counter_and_never_lowers_one() {
        let mut p = project();
        p.ids = IdAllocators::from_counts(5, 5, 5, 5, 5, 5, 5, 5, 5);
        let through = IdAllocators::from_counts(9, 1, 5, 0, 7, 2, 6, 3, 8);
        Command::ReserveIds { through }.apply(&mut p).unwrap();
        assert_eq!(p.ids, IdAllocators::from_counts(9, 5, 5, 5, 7, 5, 6, 5, 8));
    }

    #[test]
    fn reserve_ids_is_self_inverse_so_undo_keeps_the_high_water_mark() {
        let mut p = project();
        let mut stack = CommandStack::new();
        let mut clone = p.ids.clone();
        let id = clone.next_group_address_id().unwrap();
        stack
            .do_command(
                &mut p,
                Command::Batch(vec![
                    Command::ReserveIds { through: clone },
                    Command::CreateGroupAddress {
                        entry: ga(id.0, 1),
                        installation: None,
                    },
                ]),
            )
            .unwrap();
        assert_eq!(p.ids.peek_group_address(), 1);
        stack.undo(&mut p).unwrap();
        assert!(p.installations[0].group_addresses.is_empty());
        assert_eq!(
            p.ids.peek_group_address(),
            1,
            "undo must not rewind: the next create has to get a fresh id"
        );
        let next = p.ids.clone().next_group_address_id().unwrap();
        assert_eq!(next, GroupAddressId(2));
    }

    #[test]
    fn a_stale_snapshot_can_no_longer_duplicate_an_id() {
        // ADR-0039 appendix, at the core level: request A plans from a
        // snapshot, request B creates with the live counter in between,
        // then A applies its stale plan.
        let mut p = project();
        let mut stack = CommandStack::new();
        let mut stale = p.ids.clone();
        let planned = stale.next_group_address_id().unwrap();

        let live = p.ids.next_group_address_id().unwrap();
        stack
            .do_command(
                &mut p,
                Command::CreateGroupAddress {
                    entry: ga(live.0, 10),
                    installation: None,
                },
            )
            .unwrap();

        let before = format!("{p:#?}");
        let result = stack.do_command(
            &mut p,
            Command::Batch(vec![
                Command::CreateGroupAddress {
                    entry: ga(planned.0, 20),
                    installation: None,
                },
                Command::ReserveIds { through: stale },
            ]),
        );
        assert_eq!(
            result,
            Err(CommandError::BatchItem {
                index: 0,
                source: Box::new(CommandError::IdInUse {
                    kind: IdKind::GroupAddress,
                    id: 1,
                }),
            })
        );
        assert_eq!(format!("{p:#?}"), before);
    }

    fn second_installation(p: &mut Project) {
        let mut second = p.installations[0].clone();
        second.id = InstallationId(1);
        second.name = "J".into();
        p.installations.push(second);
    }

    #[test]
    fn an_id_held_in_another_installation_is_refused_too() {
        let mut p = project();
        second_installation(&mut p);
        p.installations[1].group_addresses.push(ga(3, 9));
        let before = format!("{p:#?}");
        let result = Command::CreateGroupAddress {
            entry: ga(3, 1),
            installation: None,
        }
        .apply(&mut p);
        assert_eq!(
            result,
            Err(CommandError::IdInUse {
                kind: IdKind::GroupAddress,
                id: 3,
            })
        );
        assert_eq!(format!("{p:#?}"), before);
    }

    /// MODEL-01: an existing `(device, ets_id)` row is edited where it lives,
    /// even in a later installation — never duplicated into the first one.
    /// A genuinely new row still needs a free id.
    #[test]
    fn a_parameter_row_in_another_installation_is_edited_in_place() {
        let mut p = project();
        second_installation(&mut p);
        let row = |id: u32| crate::parameter::ParameterInstance {
            id: ParameterInstanceId(id),
            device: DeviceId(1),
            source: SourceRef {
                path: "t".into(),
                ets_id: "P-1".into(),
            },
            raw: "0".into(),
        };
        p.installations[1].parameters.push(row(5));
        p.installations[0]
            .parameters
            .push(crate::parameter::ParameterInstance {
                source: SourceRef {
                    path: "t".into(),
                    ets_id: "P-2".into(),
                },
                ..row(7)
            });
        create_device(1, "d", &[]).apply(&mut p).unwrap();
        Command::SetParameterValue {
            id: ParameterInstanceId(99),
            device: DeviceId(1),
            ets_id: "P-1".into(),
            raw: "1".into(),
        }
        .apply(&mut p)
        .unwrap();
        assert_eq!(p.installations[1].parameters[0].raw, "1");
        assert_eq!(p.installations[1].parameters[0].id, ParameterInstanceId(5));
        assert!(!p.installations[0]
            .parameters
            .iter()
            .any(|row| row.source.ets_id == "P-1"));

        let before = format!("{p:#?}");
        let result = Command::SetParameterValue {
            id: ParameterInstanceId(5),
            device: DeviceId(1),
            ets_id: "P-3".into(),
            raw: "1".into(),
        }
        .apply(&mut p);
        assert_eq!(
            result,
            Err(CommandError::IdInUse {
                kind: IdKind::ParameterInstance,
                id: 5,
            })
        );
        assert_eq!(format!("{p:#?}"), before);
    }

    /// `apply`'s contract is "untouched on `Err`", and a failed batch's
    /// rollback is not an undo: a reservation the batch made must go too,
    /// or a refused create would still consume ids (ADR-0039 Consequences).
    #[test]
    fn a_failed_batch_rolls_back_its_own_reservation() {
        let mut p = project();
        Command::CreateArea {
            area: area(1, 1),
            installation: None,
        }
        .apply(&mut p)
        .unwrap();
        let before = format!("{p:#?}");
        let mut through = p.ids.clone();
        through.next_area_id().unwrap();
        through.next_area_id().unwrap();
        let result = Command::Batch(vec![
            Command::ReserveIds { through },
            Command::CreateArea {
                area: area(1, 2),
                installation: None,
            },
        ])
        .apply(&mut p);
        assert!(matches!(
            result,
            Err(CommandError::BatchItem { index: 1, source })
                if matches!(*source, CommandError::IdInUse { .. })
        ));
        assert_eq!(format!("{p:#?}"), before);
    }
}
