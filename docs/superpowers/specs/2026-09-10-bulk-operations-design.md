# T9: Bulk/Multi-Select Operations — Design Spec

Closes GAP_ANALYSIS_ETS.md **B9**: every edit in the UI today targets exactly
one entity. This spec adds multi-select in the Project Explorer for devices
and group addresses, plus batch delete and batch topology reassignment
(move-to-line / move-to-building-part), all atomically undoable as one step.

## Problem

`CommandStack` (`crates/knx-core/src/command.rs`) is generic over `Command`,
but is shaped one-command-one-inverse: `do_command` applies one `Command` and
pushes the one inverse `apply` returns. A UI action that deletes 20 devices
today would need 20 separate undo steps, or the caller would need to loop
`do_command` itself with no atomicity — a partial failure at device 11 would
leave the project with 10 devices deleted and no way to cleanly undo just
that partial batch as a unit.

## Design

### `Command::Batch(Vec<Command>)`

A new variant, not a new mechanism: `CommandStack` needs no change at all —
it already just calls `.apply()` and pushes back whatever `Command` comes
out. `Batch`'s own `apply` supplies the atomicity:

```rust
Command::Batch(commands) => {
    let mut inverses = Vec::with_capacity(commands.len());
    for cmd in commands {
        match cmd.apply(project) {
            Ok(inverse) => inverses.push(inverse),
            Err(e) => {
                // Roll back everything this batch already applied, in
                // reverse order, before surfacing the original error —
                // `apply`'s contract is "leave `project` untouched on
                // `Err`", and that contract is per-`Command`, including
                // `Batch` itself.
                for inverse in inverses.into_iter().rev() {
                    inverse.apply(project).expect(
                        "an inverse of an already-applied command must re-apply",
                    );
                }
                return Err(e);
            }
        }
    }
    inverses.reverse();
    Ok(Command::Batch(inverses))
}
```

An empty `Batch` (nothing selected — should not reach `apply` in practice,
but the type allows it) applies as a no-op and inverts to itself.

This is the only change to `command.rs`'s enum and `apply` dispatch. No new
`CommandError` variant is needed — a sub-command's existing error propagates
unchanged (e.g. deleting a device with a line still attached to it reports
the same `CommandError` a single `DeleteDevice` would).

### Concrete batch operations

B9 asks for "batch address reassignment" and "batch delete" — both already
exist as single-entity commands, so `Batch` composes them rather than adding
new business logic:

- **Batch delete devices** — `Command::Batch(vec![Command::DeleteDevice { id }, ...])`
- **Batch delete group addresses** — `Command::Batch(vec![Command::DeleteGroupAddress { id }, ...])`
- **Batch move devices to a line** — `Command::Batch(vec![Command::MoveDeviceToLine { device, line }, ...])`
- **Batch move devices to a building part** — `Command::Batch(vec![Command::MoveDeviceToBuildingPart { device, building_part }, ...])`

No batch variant for individual-address reassignment: each device's address
is necessarily distinct, so there is nothing to fan one value out to —
"batch address reassignment" in B9's own wording means topology
reassignment (line/building part), which `MoveDeviceTo*` already covers.

### HTTP surface (`apps/knx-server`)

`Command` has no `Serialize`/`Deserialize` and is never sent over the wire
directly (see `domain.rs`'s existing one-`*_impl`-function-per-command
convention). Batch gets the same treatment — one typed function and route
per batch operation, each building its own `Command::Batch` internally and
calling the existing private `apply(state, cmd)` helper:

```rust
pub fn batch_delete_devices_impl(state: &AppState, ids: Vec<u32>) -> Result<ProjectTree, String>
pub fn batch_delete_group_addresses_impl(state: &AppState, ids: Vec<u32>) -> Result<ProjectTree, String>
pub fn batch_move_devices_to_line_impl(state: &AppState, device_ids: Vec<u32>, line_id: Option<u32>) -> Result<ProjectTree, String>
pub fn batch_move_devices_to_building_part_impl(state: &AppState, device_ids: Vec<u32>, building_part_id: Option<u32>) -> Result<ProjectTree, String>
```

Routes (mirroring the existing kebab-case `/api/...` convention):

```
POST /api/devices/batch-delete             { "ids": [1,2,3] }
POST /api/group-addresses/batch-delete     { "ids": [1,2,3] }
POST /api/devices/batch-move-line          { "deviceIds": [1,2], "lineId": 4 }        // lineId may be null
POST /api/devices/batch-move-building-part { "deviceIds": [1,2], "buildingPartId": 4 } // may be null
```

An empty `ids`/`deviceIds` array is a 400 (nothing to do — the UI should
never enable a bulk action with nothing selected, but the server does not
trust the client).

### `knx-web` — multi-select UI

`selection.ts`'s `Selection` type stays the single-selection type used by
the Inspector (a batch of mixed-kind entities has no single Inspector view).
Multi-select is additive, separate state in `ProjectExplorer`:

```ts
// selection.ts
export type MultiSelectionKind = "device" | "group_address";
export type MultiSelection = { kind: MultiSelectionKind; ids: Set<number> };
```

- Ctrl/Cmd-click a `DeviceItem` or `GroupAddressItem` toggles it into a
  `MultiSelection` of that kind. Starting a multi-select of one kind while a
  different kind is active replaces it (no mixed-kind batches — the bulk
  toolbar's actions are kind-specific and a mixed batch has no single
  coherent action).
  Shift-click extends the range from the last-clicked item to the clicked
  one (same list, i.e. device-to-device or GA-to-GA), same replace-on-kind-
  switch rule.
- Plain click clears any active `MultiSelection` and behaves exactly as
  today (sets the single `Selection`, opens the Inspector).
- While a `MultiSelection` is non-empty, a bulk-action toolbar appears above
  the Project Explorer: "N devices selected — Delete / Move to line… / Move
  to building part…" (or "N group addresses selected — Delete"). Each
  action confirms (native `confirm()`, consistent with the existing single
  delete buttons in `Inspector.tsx`) before calling its batch API and then
  clearing the `MultiSelection`.
- Escape, or clicking the toolbar's own dismiss control, clears the
  `MultiSelection` without acting.

### Undo/redo

No change needed beyond `Command::Batch` itself — `undo_impl`/`redo_impl`
already call `stack.undo`/`stack.redo` generically. A batch delete of 20
devices undoes in one `Ctrl+Z`, restoring all 20.

## Out of scope

- Copy/paste of a device with its parameters (named in B9's motivating
  example but not in T9's own backlog text) — separate gap, not addressed
  here.
- Batch edit of a shared property across mixed-kind selections.
- Persistence (`knx-store::command_sync`) gaining a `Batch` arm: incremental
  `sync_after_command` is already incomplete for most non-`SetIndividualAddress`
  commands (many arms are explicit no-ops, relying on a full `save_project`
  instead — see `crates/knx-store/src/command_sync.rs`) and is not wired
  into `knx-server` at all (`domain.rs` persists only via the explicit
  `/api/project/save*` routes' whole-project `save_project`). `Batch` needs
  no new `command_sync` arm for the same reason: nothing currently calls
  `sync_after_command` from the server's command path.
