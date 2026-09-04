# Session 5, cycle 4: selection + properties inspector — design

**Status.** Approved, ready for implementation planning.

## Context

Session 5 (UI/UX) cycle 3 landed native `.knxdb` save/load. ROADMAP.md and
CLAUDE.md's UI/UX section list four remaining deliverables: Properties
Inspector, Search, Command Palette, Dark/Light mode. This is too large for
one spec; the four decompose into an ordered sequence:

1. **Selection + Properties Inspector** (this spec)
2. Search — builds on the selection mechanism from (1)
3. Command Palette — builds on the action registry that naturally falls out
   of (1) and (2)
4. Dark/Light mode — independent CSS/theme work, can slot in anytime

**Current state** (`apps/knx-desktop`):

- `ProjectExplorer.tsx` renders the tree read-only. Clicking a node only
  expands/collapses it — there is no selection concept and no click handler
  on `DeviceItem`.
- `src-tauri/src/lib.rs` exposes exactly four Tauri commands: `open_project`,
  `save_project_as`, `save_project`, `open_native_project`. None of them
  invoke `knx_core::Command` — the command layer
  (`crates/knx-core/src/command.rs`, `Command` + `CommandStack` with full
  undo/redo) exists and is unit-tested but is not wired into the desktop
  app anywhere.
- `DeviceNode` (the tree projection, `crates/knx-projection/src/lib.rs`)
  carries only `id`, `name`, `address`, `description` — no communication
  objects. Group addresses are not represented in `ProjectTree` at all.
- `knx_core::Command` and `CommandError` derive neither `Serialize` nor
  `ts-rs`'s `TS` — they cannot cross the Tauri IPC boundary as-is.

**Scope decision.** Group address create/delete (two of the four existing
`Command` variants) is deferred to the Search cycle, since group addresses
are not yet shown anywhere in the UI — adding them belongs with the work
that makes them browsable. This cycle covers device selection and editing
`SetIndividualAddress` (device) and `SetComObjectDpt`/`RestoreComObjectDpt`
(the device's communication objects), plus undo/redo for both.

## Backend

### Projection: lazy device detail

`DeviceNode` in the tree stays as it is — the reference project has 907
communication object instances; embedding them in every tree node would
mean loading all of them on every `open_project`/tree refresh. Instead, a
new Tauri command loads a device's detail lazily, on selection:

```rust
// crates/knx-projection/src/lib.rs
pub struct DeviceDetail {
    pub id: u32,
    pub name: String,
    pub description: Option<String>,
    pub address: Option<String>,
    pub com_objects: Vec<ComObjectNode>,
}

pub struct ComObjectNode {
    pub id: u32,
    pub number: u16,
    pub name: Option<String>,       // from `text: Override<Text>`
    pub dpt: Option<String>,        // formatted "main.sub", e.g. "9.001"
    pub dpt_layer: Option<String>,  // "Program" | "ProgramRef" | "Instance" | "Inferred" | "UserEdit"
    pub is_active: bool,
    // flags (read/write/transmit/update/communication) as plain bools,
    // `Override::Value` -> the bool, anything else -> false for display
    // purposes only (not round-tripped from here — display-only, no
    // command edits them this cycle).
    pub read: bool,
    pub write: bool,
    pub transmit: bool,
    pub update: bool,
    pub communication: bool,
}
```

Only `address` (device) and `dpt` (com object) are editable this cycle —
everything else in `DeviceDetail`/`ComObjectNode` is display-only, matching
the fact that no `Command` variant exists yet to change name, description,
or flags.

`build_device_detail(device: &DeviceInstance) -> DeviceDetail` lives next to
`build_device_node` in `knx-projection`, pure and total like the rest of
that module.

### Command dispatch

**Correction from the first draft of this section:** giving
`knx_core::Command`/`CommandError` `Serialize`/`Deserialize`/`TS` derives
would drag every type reachable from them (`ValidationError`,
`GroupAddressEntry`, `GroupAddress`, `Override<T>`, ...) into IPC surface
for the sake of two fields, and would add `serde`/`ts-rs` as dependencies
of `knx-core` for the first time. Simpler: two narrow, per-field Tauri
commands that take plain `Option<String>` and parse it server-side with
the parsers that already exist and are already unit-tested
(`IndividualAddress: FromStr`, `DptRef::parse`) — no new type crosses IPC,
`knx-core` gains no new dependency, and there is no parsing logic to
duplicate in TypeScript. The single-text-field UX from the frontend
section is unaffected; only where parsing happens changed.

`AppState` gains:

```rust
pub command_stack: Mutex<CommandStack>,
/// (errors, warnings) from the initial import's `ImportReport`, carried
/// into every tree rebuilt after a command/undo/redo — edits don't change
/// what import lost. `(0, 0)` for a `.knxdb` native load.
pub import_counts: Mutex<(usize, usize)>,
```

Both reset inside `open_project` (`import_counts` set from the same
counts `apply_report_counts` computes) and `open_native_project` (both
`(0, 0)`, fresh `CommandStack`) — undo history and the import-loss counts
never survive loading a different project. They do survive a
`save_project`/`save_project_as` on the current project (in-memory only,
never persisted to `.knxdb` — out of scope, same as every other app's
session-only undo history).

Four new Tauri commands, mirroring the existing `open_project` shape
(return the full refreshed tree so tree labels — e.g. a changed individual
address — update without a second round trip):

```rust
#[tauri::command]
fn set_individual_address(device_id: u32, address: Option<String>, state: tauri::State<AppState>) -> Result<ProjectTree, String>;

#[tauri::command]
fn set_com_object_dpt(com_object_id: u32, dpt: Option<String>, state: tauri::State<AppState>) -> Result<ProjectTree, String>;

#[tauri::command]
fn undo(state: tauri::State<AppState>) -> Result<ProjectTree, String>;

#[tauri::command]
fn redo(state: tauri::State<AppState>) -> Result<ProjectTree, String>;
```

`set_individual_address`/`set_com_object_dpt` parse their `Option<String>`
first (`s.parse::<IndividualAddress>()` / `DptRef::parse(&s)`), returning
the parser's own `Display`ed error on failure without touching the
project; on success they build the matching `Command` and go through a
shared `apply` helper with `undo`/`redo`. `apply` locks `project` and
`command_stack` together, calls `CommandStack::do_command`, and — on
success — rebuilds the tree via `knx_projection::build_project_tree` and
overlays `import_counts` and `command_stack.can_undo()`/`can_redo()` onto
it before returning. On failure the project is untouched (`Command::apply`'s
existing guarantee) and the error is `CommandError`'s/`ValidationError`'s
`Display` text — already human-readable (e.g. "individual address 1.1.1
already used by device 5, cannot assign to device 3"). No structured
error payload this cycle.

`ProjectTree` gains `can_undo: bool` / `can_redo: bool` so the frontend's
undo/redo buttons know their enabled state without a separate round trip.

## Frontend

- `ProjectExplorer` takes `selectedId: number | null` and
  `onSelectDevice: (id: number) => void`; `DeviceItem` becomes clickable
  and gets a `selected` class when its id matches.
- New `Inspector.tsx`: renders a `DeviceDetail`. Address field: text input,
  format `"1.1.1"`, parsed client-side; empty clears the address
  (`address: null`). Each com object's DPT: text input, format `"9.001"`
  (`main.sub`, sub optional), empty clears it (`dpt: None`). Auto-apply on
  blur/Enter — no separate save button. On `apply_command` rejection, the
  field shows the error text inline and keeps its previous value until
  corrected; on success the field clears its error and the whole
  `DeviceDetail` (and tree) refresh.
- Toolbar gains Undo/Redo buttons (enabled from `tree.can_undo`/
  `can_redo`) plus global `Ctrl+Z` / `Ctrl+Shift+Z` shortcuts.
- `App.tsx` holds `selectedDeviceId` and the fetched `DeviceDetail`; fetches
  `device_detail` on selection change and again after every
  `apply_command`/`undo`/`redo` call while a device is selected.

## Error handling

Errors stay plain strings end to end — `CommandError`/`ValidationError`'s
existing `Display` impls are already specific enough for inline display.
Structured error variants (so the frontend could e.g. highlight the
*other* device holding a duplicate address) are future work, not needed
for a single inline message under the field that was edited.

## Testing

No frontend test infrastructure exists yet (`package.json` has no test
runner); adding one is a separate decision, not part of this cycle.
Backend-only, following `apps/knx-desktop/src-tauri/tests/`'s existing
no-Tauri-machinery pattern (`open_reference_project.rs`,
`save_load_roundtrip.rs`):

- `set_individual_address_impl`/`set_com_object_dpt_impl`/`undo_impl`/
  `redo_impl`/`device_detail_impl` (the `pub fn`s the Tauri commands wrap,
  same split as `open_project_impl`/`save_project_as_impl`) get a new
  `command_dispatch.rs` integration test: apply an individual-address
  change, confirm the tree and `device_detail` reflect it, undo, confirm
  it reverts, redo, confirm it reapplies.
- A duplicate-individual-address case: `set_individual_address_impl`
  returns `Err`, the project is unchanged, `command_stack` gained nothing
  to undo.
- A malformed address/DPT string case: the parser's own error surfaces,
  project untouched.
- `set_com_object_dpt_impl` through the same round trip, confirming
  `dpt_layer` reports `"UserEdit"` after a manual edit.

## Out of scope (carried to later cycles)

- Group address browsing and `CreateGroupAddress`/`DeleteGroupAddress` —
  Search cycle.
- Editing device name/description or communication object flags — no
  `Command` variant exists for them; adding one is its own decision.
- Structured/typed command errors.
- Frontend test infrastructure.
- Persisting undo history into `.knxdb`.
