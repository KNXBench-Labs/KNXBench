# Topology & group-range command layer — design

**Status.** Approved, ready for implementation planning.

## Context

[docs/GAP_ANALYSIS_ETS.md](../../GAP_ANALYSIS_ETS.md) audits KNXBench
against ETS's feature set. Its Tier 1 backlog (biggest usability gap: the
app can only *edit* what an ETS import already contains — `knx-core`'s
`Command` enum has six variants total, none of which create or restructure
topology, buildings, or group ranges) decomposes into two sub-projects:

1. **This spec** — topology CRUD (area/line), group-range CRUD, and
   group-link editing. Backend only: `knx-core` command layer plus
   `apps/knx-server` routes. No frontend UI this cycle.
2. **Later** — device-from-catalog creation/deletion and the catalog
   browser UI (needs (1) as a foundation: a device must land on a line,
   which (1) makes creatable).

**Scope decision: backend only.** `knx-projection`'s `GroupAddressNode` has
no concept of `GroupRange` at all — `InstallationNode.group_addresses` is a
flat list, not the real main/middle/address hierarchy ETS uses. Wiring
group-range CRUD into the UI properly means restructuring that projection
(and, downstream, `Search.tsx`'s index and `ProjectExplorer.tsx`'s tree) —
real work, deliberately deferred to its own future cycle rather than
bundled here, the same way Session 5 cycle 2 shipped `knx-store` entity
persistence with no UI and cycle 3 added the UI later. This cycle ships a
fully tested command layer and HTTP surface; the next cycle to touch this
area designs the projection change and the UI on top of a stable backend.

**Current state**, confirmed by reading the source rather than assumed:

- `knx_core::project::IdAllocators` already has `next_area_id`,
  `next_line_id`, and `next_group_range_id` — allocated but never consumed
  by any `Command`. The domain model anticipated this work.
- `knx_core::validation` already has `check_group_address_in_range` and
  `check_group_link_target_exists` — both fully implemented and unit
  tested, but never called from `command.rs`. `Command::CreateGroupAddress`
  today does **not** check a group address falls inside its stated range —
  a real, previously undocumented gap this spec also closes.
- `GroupRange` (`group.rs`) already models `parent`/`children` for two
  levels of nesting (main → middle), matching the reference project.
- [KNOWN_LIMITATIONS.md §21](../../KNOWN_LIMITATIONS.md#21-a-ui-created-group-address-has-no-ets_id-and-is-dropped-on-export):
  `create_group_address_impl` (`apps/knx-server/src/domain.rs`) sets
  `source.ets_id = String::new()` and `range: None` — the exporter silently
  drops any group address like this. Fixing it needs a `GroupRange` to nest
  in (this spec) and a synthetic id convention (below).
- `Command::apply`'s own doc comment: every command targets
  `installations[0]`; no command routes to a different installation. This
  spec does not change that — area/line/range commands stay scoped to the
  same single-installation assumption every existing command already makes.

## Domain model changes

No new fields on any existing type. `Area`, `Line`, `GroupRange` already
carry everything a create/delete needs (`ids.rs`, `group.rs`,
`topology.rs`). This is command-layer and validation-layer work only.

## New `Command` variants (`crates/knx-core/src/command.rs`)

Each pair mirrors `CreateGroupAddress`/`DeleteGroupAddress`'s existing
shape: the caller pre-allocates the id via `IdAllocators`, `apply` returns
the exact inverse.

```rust
CreateArea { area: Area },
DeleteArea { id: AreaId },

CreateLine { area: AreaId, line: Line },
DeleteLine { id: LineId },

/// `line: None` moves the device to `Topology::unassigned`.
MoveDeviceToLine { device: DeviceId, line: Option<LineId> },

CreateGroupRange { range: GroupRange },
DeleteGroupRange { id: GroupRangeId },
RenameGroupRange { id: GroupRangeId, name: String },

LinkComObject { com_object: ComObjectInstanceId, ga: GroupAddressId, direction: Direction },
UnlinkComObject { com_object: ComObjectInstanceId, ga: GroupAddressId, direction: Direction },
```

### Apply semantics

- **`CreateArea`** — validates no other area shares `area.address`
  (`check_no_duplicate_area_address`, new), pushes into
  `installation.topology.areas`. Inverse: `DeleteArea { id }`.
- **`DeleteArea`** — refuses (`CommandError::AreaNotEmpty`) if
  `area.lines` is non-empty. Removes the area. Inverse: `CreateArea { area }`
  (the full struct, saved before removal — same pattern
  `DeleteGroupAddress` uses for its own inverse).
- **`CreateLine`** — looks up `area` (`CommandError::AreaNotFound`),
  validates no other line *in that area* shares `line.address`
  (`check_no_duplicate_line_address`, new — scoped per-area, matching
  ETS's Area.Line.Device numbering, not global), pushes into
  `topology.lines` and into `area.lines`. Inverse: `DeleteLine { id }`.
- **`DeleteLine`** — refuses (`CommandError::LineNotEmpty`) if
  `line.devices` is non-empty. Removes the line from both `topology.lines`
  and its owning area's `lines` list. Inverse: `CreateLine { area, line }`
  (looks up the owning area via `Topology::area_of` before removing, to
  save it in the inverse).
- **`MoveDeviceToLine`** — validates the target line exists if `Some`
  (`CommandError::LineNotFound`). Removes `device` from wherever it
  currently sits (an existing line's `devices`, or `topology.unassigned`
  — exactly one of those must contain it, else
  `CommandError::DeviceNotFound`) and inserts it into the target (`Some`
  line's `devices`, or `unassigned` if `None`). Inverse:
  `MoveDeviceToLine { device, line: <old location> }`. Does **not** touch
  `DeviceInstance::address` — moving a device to a new line does not
  reassign its individual address; that stays `SetIndividualAddress`'s job,
  deliberately kept separate (a line move and a re-address are two
  different user intents, even though ETS often prompts for both together —
  out of scope to couple them here).
- **`CreateGroupRange`** — validates: if `range.parent` is `Some(parent)`,
  the parent exists (`CommandError::GroupRangeNotFound`) and
  `parent.contains(range.start) && parent.contains(range.end)`
  (reusing `GroupRange::contains`, new check
  `check_group_range_nests_in_parent`); no sibling range at the same
  nesting level overlaps `[start, end]` (`check_no_overlapping_group_range`,
  new). Pushes into `installation.group_ranges`, and into
  `parent.children` if nested. Inverse: `DeleteGroupRange { id }`.
- **`DeleteGroupRange`** — refuses (`CommandError::GroupRangeNotEmpty`) if
  `range.children` is non-empty, or (`CommandError::GroupRangeInUse`) if any
  `GroupAddressEntry.range == Some(id)`. Removes the range from
  `group_ranges` and from its parent's `children` if nested. Inverse:
  `CreateGroupRange { range }`.
- **`RenameGroupRange`** — no validation beyond existence
  (`CommandError::GroupRangeNotFound`); names are not required unique in
  the existing `GroupAddressEntry`/`Line`/`Area` model either. Inverse:
  `RenameGroupRange { id, name: <old name> }`.
- **`LinkComObject`** — validates the comm object exists
  (`CommandError::ComObjectNotFound`), the target group address exists
  (`check_group_link_target_exists`, now finally called), and the exact
  `GroupLink { ga, direction }` is not already present on that comm object
  (`CommandError::LinkAlreadyExists` — a comm object may send *and* receive
  the same group address as two distinct `GroupLink`s, so this checks the
  `(ga, direction)` pair, not just `ga`). Pushes onto
  `ComObjectInstance.links`. Inverse: `UnlinkComObject { com_object, ga, direction }`.
- **`UnlinkComObject`** — validates the link exists
  (`CommandError::LinkNotFound`). Removes it. Inverse:
  `LinkComObject { com_object, ga, direction }`.

### `Command::CreateGroupAddress` gets one more check

Not a new variant — closing the gap found while reading the existing code
(see "Current state" above): if `entry.range` is `Some(range_id)`, `apply`
now also calls `check_group_address_in_range` against that range,
returning `CommandError::Validation` on failure like every other
validation call already does. No behavior change for `entry.range: None`
(today's only caller, `create_group_address_impl`, before this cycle's
server-side fix below).

### New `CommandError` variants

```rust
AreaNotFound(AreaId),
AreaNotEmpty(AreaId),
LineNotFound(LineId),
LineNotEmpty(LineId),
GroupRangeNotFound(GroupRangeId),
GroupRangeNotEmpty(GroupRangeId),
GroupRangeInUse(GroupRangeId),
LinkAlreadyExists { com_object: ComObjectInstanceId, ga: GroupAddressId, direction: Direction },
LinkNotFound { com_object: ComObjectInstanceId, ga: GroupAddressId, direction: Direction },
```

`DeviceNotFound` already exists and is reused by `MoveDeviceToLine`.

### New `validation.rs` functions

```rust
pub fn check_no_duplicate_area_address(topology: &Topology, candidate: AreaId, address: u8) -> Result<(), ValidationError>;
pub fn check_no_duplicate_line_address(area: &Area, candidate: LineId, address: u8) -> Result<(), ValidationError>;
pub fn check_group_range_nests_in_parent(parent: &GroupRange, start: GroupAddress, end: GroupAddress) -> Result<(), ValidationError>;
pub fn check_no_overlapping_group_range(siblings: &[&GroupRange], candidate: GroupRangeId, start: GroupAddress, end: GroupAddress) -> Result<(), ValidationError>;
```

Each gets a matching `ValidationError` variant
(`DuplicateAreaAddress`/`DuplicateLineAddress`/`GroupRangeOutsideParent`/
`OverlappingGroupRange`), following the existing `DuplicateGroupAddress`/
`GroupAddressOutsideRange` naming pattern exactly.

## Synthetic id convention (closes KNOWN_LIMITATIONS.md §21)

Every entity this spec's commands create in `apps/knx-server` (group
address, area, line, group range) gets `SourceRef { path, ets_id }` set to
`format!("KB-{Kind}-{id}")` (e.g. `"KB-GA-42"`, `"KB-Area-3"`,
`"KB-Line-7"`, `"KB-Range-1"`) instead of the empty string
`create_group_address_impl` writes today — unique (numeric ids are unique
per project), stable across save/load, and visibly synthetic in exported
XML so a human reading an export can tell a KNXBench-authored entity from
an ETS-authored one at a glance. `path` gets the same value as `ets_id`
(today's group address creation leaves `path` empty too — same fix, same
place).

`create_group_address_impl`'s `range` parameter changes from optional
(`None` always, today) to a required `GroupRangeId` argument — a
UI-created group address must nest in a range that already exists, closing
the export-drop bug at the source rather than tolerating a range-less
address that only fails later at export time.

## `apps/knx-server` routes

One route per command, following the existing `/api/*` pattern
(`routes.rs`, `AppError` for error mapping):

| Route | Command |
|---|---|
| `POST /api/topology/area` | `CreateArea` |
| `DELETE /api/topology/area/:id` | `DeleteArea` |
| `POST /api/topology/line` | `CreateLine` |
| `DELETE /api/topology/line/:id` | `DeleteLine` |
| `POST /api/topology/move-device` | `MoveDeviceToLine` |
| `POST /api/group-range` | `CreateGroupRange` |
| `DELETE /api/group-range/:id` | `DeleteGroupRange` |
| `PATCH /api/group-range/:id` | `RenameGroupRange` |
| `POST /api/group-link` | `LinkComObject` |
| `DELETE /api/group-link` | `UnlinkComObject` (body carries `com_object`/`ga`/`direction`, since all three identify the link — no single path-segment id exists for one) |

Each `_impl` function in `domain.rs` follows the existing
`create_group_address_impl`/`delete_group_address_impl` shape exactly:
lock `state.project`, allocate any needed id via `project.ids`, build the
`Command`, call the shared `apply` helper (which runs it through
`CommandStack` and re-projects). `create_group_address_impl` itself is
edited in place for the required-`range`/synthetic-id change above, not
duplicated.

## Testing

- `crates/knx-core/src/command.rs`: one apply/inverse test and one
  validation-failure test per new variant, following the existing
  `CreateGroupAddress`/`DeleteGroupAddress` test pairs exactly (construct
  a project fixture, apply, assert the returned inverse, apply the inverse,
  assert the project matches the original).
- `crates/knx-core/src/validation.rs`: unit tests for each new `check_*`
  function, mirroring the existing ones' style (a passing case and a
  failing case each).
- `apps/knx-server/tests/http_edit_routes.rs`: one round-trip test per new
  route (call it, assert the response `ProjectTree` reflects the change),
  matching the file's existing coverage of `create_group_address`/
  `delete_group_address`.
- A dedicated test for the `CreateGroupAddress` in-range check addition,
  and one for `create_group_address_impl`'s new synthetic-id/required-range
  behavior.

## Out of scope for this cycle

- Any frontend/UI change — see "Scope decision" above.
- Multi-installation routing (`Command::apply` still only ever targets
  `installations[0]`) — unchanged, matches every existing command.
- Device creation/deletion (catalog-backed) — the next sub-project,
  depends on this one for a line to place a device on.
- Cascading delete for area/line/group-range — refuse-if-nonempty only,
  matching `DeleteGroupAddress`'s existing `GroupAddressInUse` precedent
  and CLAUDE.md's "choose data integrity over convenience."
- `knx-store` persistence (`sync_after_command`) for the new commands —
  IMPLEMENTATION_STATUS already documents that only four of today's
  commands have an incremental sync path and everything else round-trips
  via full `save_project`/`load_project` only "until a command exists for
  it." The nine new commands here fall into that same bucket: correct via
  a full save, no incremental sync written this cycle. Worth a follow-up
  once enough commands exist to make writing `sync_after_command` for all
  of them at once worthwhile, per that same doc's stated reasoning.
