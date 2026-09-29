//! The command layer: every mutation is a `Command` whose `apply` returns
//! its own inverse, so undo and redo replay the same mechanism (ARCHITECTURE
//! §6). Validation lives here, not in the UI and not in storage. Any command
//! that changes a `Resolved<T>` sets its layer to `Layer::UserEdit`.

use std::fmt;

use crate::address::GroupAddressStyle;
use crate::building::BuildingPart;
use crate::device::{ComObjectInstance, DeviceInstance, ProgramDefaults};
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
use crate::topology::{Area, Line};
use crate::validation::{
    check_group_address_in_range, check_group_link_target_exists,
    check_group_range_is_well_ordered, check_group_range_nests_in_parent,
    check_no_duplicate_area_address, check_no_duplicate_group_address,
    check_no_duplicate_individual_address, check_no_duplicate_line_address,
    check_no_overlapping_group_range, ValidationError,
};
use crate::{GroupAddress, IndividualAddress};

/// A single reversible mutation. Most commands target the first installation;
/// `CreateDevice` can explicitly target the installation owning a selected
/// line. Other commands still need per-installation routing.
#[derive(Debug, Clone, PartialEq)]
pub enum Command {
    SetIndividualAddress {
        device: DeviceId,
        address: Option<IndividualAddress>,
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
    },
    DeleteGroupAddress {
        id: GroupAddressId,
    },
    /// Internal inverse of [`Command::DeleteGroupAddress`]. Restores the
    /// exact list position so undo/redo cannot reorder CSV exports.
    RestoreGroupAddress {
        entry: GroupAddressEntry,
        position: usize,
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
    },
    DeleteArea {
        id: AreaId,
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
    },
    /// Refuses (`CommandError::BuildingPartNotEmpty`) if the part still
    /// has children or devices — the building-part equivalent of
    /// `DeleteGroupRange`'s `GroupRangeNotEmpty`/`LineNotEmpty`'s single
    /// non-empty check, just covering both at once since either leaves
    /// something dangling.
    DeleteBuildingPart {
        id: BuildingPartId,
    },
    RenameBuildingPart {
        id: BuildingPartId,
        name: String,
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
    /// `range.id` is pre-allocated by the caller via
    /// `Project::ids::next_group_range_id`.
    CreateGroupRange {
        range: GroupRange,
    },
    DeleteGroupRange {
        id: GroupRangeId,
    },
    RenameGroupRange {
        id: GroupRangeId,
        name: String,
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
    ComObjectNotFound(ComObjectInstanceId),
    GroupAddressNotFound(GroupAddressId),
    /// A `DeleteGroupAddress` was refused because at least one
    /// communication object still links to it — deleting it now would
    /// leave a dangling `GroupLink` (`ValidationError::DanglingGroupLink`
    /// exists for the reverse direction: a link created against a group
    /// address that is already gone).
    GroupAddressInUse(GroupAddressId),
    AreaNotFound(AreaId),
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
    GroupRangeNotFound(GroupRangeId),
    /// A `DeleteGroupRange` was refused because it still has nested
    /// (middle) ranges.
    GroupRangeNotEmpty(GroupRangeId),
    /// A `DeleteGroupRange` was refused because at least one group
    /// address still names it as its `range`.
    GroupRangeInUse(GroupRangeId),
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
            CommandError::ComObjectNotFound(id) => {
                write!(f, "communication object instance {id} not found")
            }
            CommandError::GroupAddressNotFound(id) => write!(f, "group address {id} not found"),
            CommandError::GroupAddressInUse(id) => {
                write!(
                    f,
                    "group address {id} is still linked from a communication object"
                )
            }
            CommandError::AreaNotFound(id) => write!(f, "area {id} not found"),
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
            CommandError::GroupRangeNotFound(id) => write!(f, "group range {id} not found"),
            CommandError::GroupRangeNotEmpty(id) => {
                write!(f, "group range {id} still has nested ranges, cannot delete")
            }
            CommandError::GroupRangeInUse(id) => write!(
                f,
                "group range {id} still has group addresses assigned to it"
            ),
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
                    check_no_duplicate_individual_address(&project.devices, device, addr)?;
                }
                project.devices.get_mut(device).unwrap().address = address;
                Ok(Command::SetIndividualAddress {
                    device,
                    address: previous,
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
                // Same scope as `upsert_parameter_value` below — the first
                // installation — so a matching row elsewhere cannot hide
                // that this push needs a free id.
                let is_new_instance = !project
                    .installations
                    .first()
                    .ok_or(CommandError::InstallationNotFound)?
                    .parameters
                    .iter()
                    .any(|p| p.device == device_id && p.source.ets_id == *ets_id);
                if is_new_instance {
                    check_id_free(project, IdKind::ParameterInstance, id.0)?;
                }
                let installation = project
                    .installations
                    .first_mut()
                    .ok_or(CommandError::InstallationNotFound)?;
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
                let installation = project
                    .installations
                    .first_mut()
                    .ok_or(CommandError::InstallationNotFound)?;
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
            Command::CreateGroupAddress { entry } => {
                check_id_free(project, IdKind::GroupAddress, entry.id.0)?;
                let installation = project
                    .installations
                    .first_mut()
                    .ok_or(CommandError::InstallationNotFound)?;
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
                let installation = project
                    .installations
                    .first_mut()
                    .ok_or(CommandError::InstallationNotFound)?;
                let pos = installation
                    .group_addresses
                    .iter()
                    .position(|e| e.id == id)
                    .ok_or(CommandError::GroupAddressNotFound(id))?;
                let entry = installation.group_addresses.remove(pos);
                Ok(Command::RestoreGroupAddress {
                    entry,
                    position: pos,
                })
            }
            Command::RestoreGroupAddress { entry, position } => {
                let installation = project
                    .installations
                    .first_mut()
                    .ok_or(CommandError::InstallationNotFound)?;
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
            Command::CreateArea { area } => {
                check_id_free(project, IdKind::Area, area.id.0)?;
                let installation = project
                    .installations
                    .first_mut()
                    .ok_or(CommandError::InstallationNotFound)?;
                check_no_duplicate_area_address(&installation.topology, area.id, area.address)?;
                let id = area.id;
                installation.topology.areas.push(area.clone());
                Ok(Command::DeleteArea { id })
            }
            Command::DeleteArea { id } => {
                let id = *id;
                let installation = project
                    .installations
                    .first_mut()
                    .ok_or(CommandError::InstallationNotFound)?;
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
                Ok(Command::CreateArea { area })
            }
            Command::CreateLine { area, line } => {
                check_id_free(project, IdKind::Line, line.id.0)?;
                let area_id = *area;
                let installation = project
                    .installations
                    .first_mut()
                    .ok_or(CommandError::InstallationNotFound)?;
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
                let installation = project
                    .installations
                    .first_mut()
                    .ok_or(CommandError::InstallationNotFound)?;
                let area_id = installation
                    .topology
                    .area_of(id)
                    .map(|a| a.id)
                    .ok_or(CommandError::LineNotFound(id))?;
                let pos = installation
                    .topology
                    .lines
                    .iter()
                    .position(|l| l.id == id)
                    .ok_or(CommandError::LineNotFound(id))?;
                if !installation.topology.lines[pos].devices.is_empty() {
                    return Err(CommandError::LineNotEmpty(id));
                }
                let line = installation.topology.lines.remove(pos);
                installation
                    .topology
                    .areas
                    .iter_mut()
                    .find(|a| a.id == area_id)
                    .unwrap()
                    .lines
                    .retain(|&l| l != id);
                Ok(Command::CreateLine {
                    area: area_id,
                    line,
                })
            }
            Command::MoveDeviceToLine { device, line } => {
                let device = *device;
                let line = *line;
                let installation = project
                    .installations
                    .first_mut()
                    .ok_or(CommandError::InstallationNotFound)?;
                if let Some(line_id) = line {
                    if !installation.topology.lines.iter().any(|l| l.id == line_id) {
                        return Err(CommandError::LineNotFound(line_id));
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
            Command::CreateBuildingPart { part } => {
                check_id_free(project, IdKind::BuildingPart, part.id.0)?;
                let installation = project
                    .installations
                    .first_mut()
                    .ok_or(CommandError::InstallationNotFound)?;
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
                let installation = project
                    .installations
                    .first_mut()
                    .ok_or(CommandError::InstallationNotFound)?;
                let pos = installation
                    .buildings
                    .iter()
                    .position(|p| p.id == id)
                    .ok_or(CommandError::BuildingPartNotFound(id))?;
                if !installation.buildings[pos].children.is_empty()
                    || !installation.buildings[pos].devices.is_empty()
                {
                    return Err(CommandError::BuildingPartNotEmpty(id));
                }
                let part = installation.buildings.remove(pos);
                if let Some(parent_id) = part.parent {
                    installation
                        .buildings
                        .iter_mut()
                        .find(|p| p.id == parent_id)
                        .unwrap()
                        .children
                        .retain(|&c| c != id);
                }
                Ok(Command::CreateBuildingPart { part })
            }
            Command::RenameBuildingPart { id, name } => {
                let id = *id;
                let installation = project
                    .installations
                    .first_mut()
                    .ok_or(CommandError::InstallationNotFound)?;
                let part = installation
                    .buildings
                    .iter_mut()
                    .find(|p| p.id == id)
                    .ok_or(CommandError::BuildingPartNotFound(id))?;
                let previous = std::mem::replace(&mut part.name, name.clone());
                Ok(Command::RenameBuildingPart { id, name: previous })
            }
            Command::MoveDeviceToBuildingPart { device, part } => {
                let device = *device;
                let part = *part;
                if project.devices.get(device).is_none() {
                    return Err(CommandError::DeviceNotFound(device));
                }
                let installation = project
                    .installations
                    .first_mut()
                    .ok_or(CommandError::InstallationNotFound)?;
                if let Some(part_id) = part {
                    if !installation.buildings.iter().any(|p| p.id == part_id) {
                        return Err(CommandError::BuildingPartNotFound(part_id));
                    }
                }
                let previous = remove_device_from_buildings(installation, device);
                if let Some(part_id) = part {
                    installation
                        .buildings
                        .iter_mut()
                        .find(|p| p.id == part_id)
                        .unwrap()
                        .devices
                        .push(device);
                }
                Ok(Command::MoveDeviceToBuildingPart {
                    device,
                    part: previous,
                })
            }
            Command::CreateGroupRange { range } => {
                check_id_free(project, IdKind::GroupRange, range.id.0)?;
                let installation = project
                    .installations
                    .first_mut()
                    .ok_or(CommandError::InstallationNotFound)?;
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
                let installation = project
                    .installations
                    .first_mut()
                    .ok_or(CommandError::InstallationNotFound)?;
                let pos = installation
                    .group_ranges
                    .iter()
                    .position(|r| r.id == id)
                    .ok_or(CommandError::GroupRangeNotFound(id))?;
                if !installation.group_ranges[pos].children.is_empty() {
                    return Err(CommandError::GroupRangeNotEmpty(id));
                }
                if installation
                    .group_addresses
                    .iter()
                    .any(|ga| ga.range == Some(id))
                {
                    return Err(CommandError::GroupRangeInUse(id));
                }
                let range = installation.group_ranges.remove(pos);
                if let Some(parent_id) = range.parent {
                    installation
                        .group_ranges
                        .iter_mut()
                        .find(|r| r.id == parent_id)
                        .unwrap()
                        .children
                        .retain(|&c| c != id);
                }
                Ok(Command::CreateGroupRange { range })
            }
            Command::RenameGroupRange { id, name } => {
                let id = *id;
                let installation = project
                    .installations
                    .first_mut()
                    .ok_or(CommandError::InstallationNotFound)?;
                let range = installation
                    .group_ranges
                    .iter_mut()
                    .find(|r| r.id == id)
                    .ok_or(CommandError::GroupRangeNotFound(id))?;
                let previous = std::mem::replace(&mut range.name, name.clone());
                Ok(Command::RenameGroupRange { id, name: previous })
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
                let installation = project
                    .installations
                    .first_mut()
                    .ok_or(CommandError::InstallationNotFound)?;
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
                let installation = project
                    .installations
                    .first_mut()
                    .ok_or(CommandError::InstallationNotFound)?;
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
                let installation = project
                    .installations
                    .first()
                    .ok_or(CommandError::InstallationNotFound)?;
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
                com.links.remove(pos);
                Ok(Command::LinkComObject {
                    com_object,
                    ga,
                    direction,
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
            .do_command(&mut project, Command::CreateArea { area: area.clone() })
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
                Command::CreateBuildingPart { part: part.clone() },
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
                .do_command(&mut project, Command::CreateBuildingPart { part })
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
            },
            Command::CreateGroupAddress {
                entry: test_group_address_entry(GroupAddressId(2), 2),
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
            },
            Command::DeleteDevice { id: DeviceId(99) },
            Command::CreateGroupAddress {
                entry: test_group_address_entry(GroupAddressId(2), 2),
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
            Command::CreateGroupAddress { entry: ga(1, 1) },
            Command::CreateGroupAddress { entry: ga(1, 2) },
            IdKind::GroupAddress,
            1,
        );
    }

    #[test]
    fn create_area_refuses_an_id_in_use() {
        assert_second_is_refused(
            Command::CreateArea { area: area(1, 1) },
            Command::CreateArea { area: area(1, 2) },
            IdKind::Area,
            1,
        );
    }

    #[test]
    fn create_line_refuses_an_id_in_use_even_in_another_area() {
        assert_second_is_refused(
            Command::Batch(vec![
                Command::CreateArea { area: area(1, 1) },
                Command::CreateArea { area: area(2, 2) },
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
            },
            Command::CreateGroupRange {
                range: range(1, 2048, 4095),
            },
            IdKind::GroupRange,
            1,
        );
    }

    #[test]
    fn create_building_part_refuses_an_id_in_use() {
        assert_second_is_refused(
            Command::CreateBuildingPart { part: part(1) },
            Command::CreateBuildingPart { part: part(1) },
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
            .do_command(&mut p, Command::CreateGroupAddress { entry: ga(1, 1) })
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
        let id = clone.next_group_address_id();
        stack
            .do_command(
                &mut p,
                Command::Batch(vec![
                    Command::ReserveIds { through: clone },
                    Command::CreateGroupAddress { entry: ga(id.0, 1) },
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
        let next = p.ids.clone().next_group_address_id();
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
        let planned = stale.next_group_address_id();

        let live = p.ids.next_group_address_id();
        stack
            .do_command(
                &mut p,
                Command::CreateGroupAddress {
                    entry: ga(live.0, 10),
                },
            )
            .unwrap();

        let before = format!("{p:#?}");
        let result = stack.do_command(
            &mut p,
            Command::Batch(vec![
                Command::CreateGroupAddress {
                    entry: ga(planned.0, 20),
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
        let result = Command::CreateGroupAddress { entry: ga(3, 1) }.apply(&mut p);
        assert_eq!(
            result,
            Err(CommandError::IdInUse {
                kind: IdKind::GroupAddress,
                id: 3,
            })
        );
        assert_eq!(format!("{p:#?}"), before);
    }

    /// The edited row lives in `installations[0]` (where `upsert` writes); a
    /// same-(device, ets_id) row elsewhere must not hide that this is a new
    /// instance whose id is already taken.
    #[test]
    fn a_parameter_row_in_another_installation_does_not_bypass_the_check() {
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
        let before = format!("{p:#?}");
        let result = Command::SetParameterValue {
            id: ParameterInstanceId(5),
            device: DeviceId(1),
            ets_id: "P-1".into(),
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
        Command::CreateArea { area: area(1, 1) }
            .apply(&mut p)
            .unwrap();
        let before = format!("{p:#?}");
        let mut through = p.ids.clone();
        through.next_area_id();
        through.next_area_id();
        let result = Command::Batch(vec![
            Command::ReserveIds { through },
            Command::CreateArea { area: area(1, 2) },
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
