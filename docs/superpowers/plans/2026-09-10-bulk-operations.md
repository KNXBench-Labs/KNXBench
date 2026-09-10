# T9: Bulk/Multi-Select Operations Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Multi-select devices and group addresses in the Project Explorer;
batch delete and batch topology reassignment (move-to-line /
move-to-building-part), atomically undoable as one step. Closes
GAP_ANALYSIS_ETS.md B9.

**Architecture:** A new `Command::Batch(Vec<Command>)` variant in
`knx-core` composes existing single-entity commands with all-or-nothing
apply/rollback semantics; `CommandStack` needs no change. `knx-server`
exposes four typed batch routes following the existing one-`*_impl`-per-
command convention. `knx-web` adds additive multi-select state in
`ProjectExplorer` and a bulk-action toolbar.

**Tech Stack:** Rust, Axum, React, Vitest.

**Spec:** `docs/superpowers/specs/2026-09-10-bulk-operations-design.md`

## Global Constraints

- `Command::apply` contract holds for `Batch` too: on any `Err`, `project`
  is left completely untouched — a partial failure mid-batch must roll back
  every already-applied sub-command before returning.
- No new `CommandError` variant; a sub-command's own error propagates as-is.
- No `knx-store::command_sync` changes — nothing on the server's command
  path calls `sync_after_command` today (see spec's "Out of scope").
- Batch routes reject an empty id list with 400 rather than trusting the
  client never sends one.
- Mixed-kind multi-selection (devices + group addresses at once) is not
  supported; starting a multi-select of one kind replaces any active
  multi-select of a different kind.
- Follow existing naming/style conventions exactly: kebab-case routes,
  camelCase JSON bodies, the existing `*_impl` return shape
  (`Result<ProjectTree, String>`), the existing confirm()-before-destructive-
  action pattern already used in `Inspector.tsx`.

---

### Task 1: `Command::Batch` in `knx-core`

**Files:**
- Modify: `crates/knx-core/src/command.rs`

**Interfaces:**
- Produces: `Command::Batch(Vec<Command>)` variant; `apply` arm implementing
  all-or-nothing semantics per the spec's pseudocode.

- [ ] **Step 1: Write failing tests first.**

  In `command.rs`'s existing `#[cfg(test)]` module, add tests covering:
  - A `Batch` of two independently-valid commands (e.g. two
    `CreateGroupAddress`) applies both, and the returned inverse `Batch`
    undoes both in one `apply` call.
  - A `Batch` where the second of three commands fails (e.g.
    `DeleteDevice` on a nonexistent id) leaves `project` byte-for-byte
    equal (`assert_eq!`, `Project: PartialEq` already derived) to a clone
    taken before the `apply` call, and returns the second command's own
    `CommandError` unchanged.
  - An empty `Batch` applies as a no-op and its inverse is `Batch(vec![])`.
  - Round-trip through `CommandStack`: `do_command` a `Batch`, then `undo`,
    then `redo`, asserting project state matches at each step (reuses
    `CommandStack`'s existing test patterns in this file).

  Run: `cargo test -p knx-core command::tests -- --list` to confirm the new
  test names appear, then run them (Step 2) to confirm they fail to compile
  (the variant doesn't exist yet).

- [ ] **Step 2: Add the `Batch` variant and its `apply` arm.**

  Add `Batch(Vec<Command>)` to the `Command` enum (place it last, after
  `UnlinkComObject`) with a doc comment pointing at
  `docs/superpowers/specs/2026-09-10-bulk-operations-design.md` for the
  rollback rationale. Implement the arm exactly per the spec's pseudocode:
  apply each sub-command, collect inverses, roll back in reverse order and
  propagate the original error on any failure, otherwise return
  `Command::Batch(inverses.reversed())`.

- [ ] **Step 3: Run the focused tests.**

  Run: `cargo test -p knx-core command::`

- [ ] **Step 4: Run the workspace gates.**

  Run: `cargo fmt --all --check && cargo clippy --workspace --all-targets -- -D warnings`

- [ ] **Step 5: Commit.**

  Run: `git add crates/knx-core && git commit -m "feat(knx-core): add Command::Batch for atomic multi-entity undo"`

### Task 2: `knx-server` batch routes + `knx-web` API bindings

**Files:**
- Modify: `apps/knx-server/src/domain.rs`
- Modify: `apps/knx-server/src/routes.rs`
- Create/modify: `apps/knx-server/tests/http_batch_routes.rs` (new file;
  follow the existing per-feature `http_*.rs` test file convention)
- Modify: `apps/knx-web/src/api.ts`

**Interfaces:**
- Produces: `batch_delete_devices_impl`, `batch_delete_group_addresses_impl`,
  `batch_move_devices_to_line_impl`, `batch_move_devices_to_building_part_impl`
  in `domain.rs`, each `(state: &AppState, ...) -> Result<ProjectTree, String>`.
- Produces routes: `POST /api/devices/batch-delete`,
  `POST /api/group-addresses/batch-delete`,
  `POST /api/devices/batch-move-line`,
  `POST /api/devices/batch-move-building-part`, exact request bodies per
  the spec.
- Produces `knx-web/src/api.ts` functions: `batchDeleteDevices(ids: number[])`,
  `batchDeleteGroupAddresses(ids: number[])`,
  `batchMoveDevicesToLine(deviceIds: number[], lineId: number | null)`,
  `batchMoveDevicesToBuildingPart(deviceIds: number[], buildingPartId: number | null)`,
  each `Promise<ProjectTree>`, following the existing `request()` helper
  pattern already used by `deleteDevice`/`moveDeviceToLine` in this file.
- Consumes: Task 1's `Command::Batch`.

- [ ] **Step 1: Write failing HTTP tests first.**

  In the new `apps/knx-server/tests/http_batch_routes.rs`, following the
  existing `http_*.rs` test files' setup pattern (in-memory `AppState`,
  import a project or build one via the existing device/GA create routes),
  add tests for all four routes: happy path (batch of 2+ succeeds, returned
  `ProjectTree` reflects the change), empty-id-list 400, one bad id inside
  an otherwise-valid batch leaves the project state unchanged (call
  `GET`-equivalent / re-fetch and diff against pre-call state) and returns
  a non-2xx status, and one successful `undo` after a batch restores every
  deleted/moved entity in one call.

  Run: `cargo test -p knx-server --test http_batch_routes` to confirm they
  fail (routes don't exist yet).

- [ ] **Step 2: Implement the four `*_impl` functions in `domain.rs`.**

  Each parses its typed arguments (mirroring `set_individual_address_impl`'s
  existing address-parsing pattern where relevant), builds the
  `Command::Batch` of the corresponding single-entity commands, and calls
  the existing private `apply(state, cmd)` helper. Return a 400-shaped
  `Err` (matching this file's existing error-string convention) when the
  id list is empty — check this before constructing the `Batch`.

- [ ] **Step 3: Wire the four routes in `routes.rs`.**

  Add route registrations and thin handler functions following this file's
  existing handler pattern exactly (see `delete_device`/`move_device_to_line`
  for the shape: deserialize JSON body, call the `domain::*_impl`, map
  `Err` to `ApiError`).

- [ ] **Step 4: Add the four `knx-web/src/api.ts` functions.**

  Follow `deleteDevice`/`moveDeviceToLine`'s existing shape exactly
  (`request()` helper, camelCase JSON body field names matching Step 3's
  routes).

- [ ] **Step 5: Run focused tests.**

  Run: `cargo test -p knx-server --test http_batch_routes`

- [ ] **Step 6: Run the workspace gates.**

  Run: `cargo fmt --all --check && cargo clippy --workspace --all-targets -- -D warnings && cargo run -p xtask -- check-layering`

- [ ] **Step 7: Commit.**

  Run: `git add apps/knx-server apps/knx-web/src/api.ts && git commit -m "feat(knx-server): add batch delete/move routes over Command::Batch"`

### Task 3: `knx-web` multi-select UI + bulk-action toolbar

**Files:**
- Modify: `apps/knx-web/src/selection.ts`
- Modify: `apps/knx-web/src/ProjectExplorer.tsx`
- Create: `apps/knx-web/src/BulkActionToolbar.tsx`
- Modify or create matching `.test.tsx`/`.test.ts` files, following this
  directory's existing colocated-test convention (e.g.
  `ProjectExplorer.test.tsx` if it exists; otherwise add one)

**Interfaces:**
- Produces: `MultiSelectionKind`, `MultiSelection` in `selection.ts`.
- Produces: `BulkActionToolbar` component, rendered by `ProjectExplorer`
  above the tree when `multiSelection` is non-empty.
- Consumes: Task 2's four `knx-web/src/api.ts` functions.

- [ ] **Step 1: Write failing tests first.**

  Cover: ctrl-click toggles an id into a `MultiSelection`; ctrl-click on a
  different kind while one kind is active replaces it; shift-click extends
  a contiguous range within the same kind; plain click clears the
  `MultiSelection` and sets the single `Selection`; Escape clears it; the
  toolbar calls the correct batch API function with the correct id list and
  clears the selection on success (mock `api.ts`).

  Run the project's existing frontend test command
  (`cd apps/knx-web && npm test`) to confirm the new tests fail first.

- [ ] **Step 2: Add `MultiSelection` state and click handling to
  `ProjectExplorer.tsx`.**

  Thread ctrl/shift-click detection through `DeviceItem`/`GroupAddressItem`
  exactly as the existing `onSelect` prop is threaded — do not change
  `onSelect`'s existing plain-click contract, add alongside it.

- [ ] **Step 3: Build `BulkActionToolbar.tsx`.**

  Props: the active `MultiSelection`, an `onDone: () => void` callback
  (clears selection and triggers `onTreeUpdate`), and whatever
  `line`/`building part` picker the move actions need (reuse existing
  dropdown patterns from `Inspector.tsx` if present, rather than inventing
  a new one). Confirm via native `confirm()` before delete, consistent with
  `Inspector.tsx`'s existing single-delete buttons.

  This step needs a line/building-part picker for the two move actions. If
  no existing reusable picker component covers this, use a plain `<select>`
  populated from `tree` (already available in `ProjectExplorer`'s props) —
  do not build a new searchable/combobox component for this; that is out of
  scope for this task.

- [ ] **Step 4: Wire `BulkActionToolbar` into `ProjectExplorer`'s render.**

- [ ] **Step 5: Run focused tests.**

  Run: `cd apps/knx-web && npm test`

- [ ] **Step 6: Run the full frontend build.**

  Run: `cd apps/knx-web && npm run build`

- [ ] **Step 7: Commit.**

  Run: `git add apps/knx-web && git commit -m "feat(knx-web): multi-select devices/group addresses with bulk delete/move"`

### Task 4: Docs and gap-analysis closure

**Files:**
- Modify: `docs/GAP_ANALYSIS_ETS.md`
- Modify: `docs/IMPLEMENTATION_STATUS.md`

- [ ] **Step 1: Close B9/T9 in `GAP_ANALYSIS_ETS.md`**, matching the
  existing "Done"-entry style used for T1/T3/T8/T10 elsewhere in the file
  (dated, one line pointing at this spec and the merge).

- [ ] **Step 2: Add an `IMPLEMENTATION_STATUS.md` entry** for this cycle,
  following the existing per-cycle narrative style in the Session 5/7 rows.

- [ ] **Step 3: Run the full workspace gate suite once, end to end.**

  Run:
  ```
  cargo fmt --all --check
  cargo clippy --workspace --all-targets -- -D warnings
  cargo test --workspace
  cargo run -p xtask -- check-layering
  cd apps/knx-web && npm test && npm run build
  ```

- [ ] **Step 4: Commit.**

  Run: `git add docs && git commit -m "docs: close T9/B9 bulk operations in gap analysis and status"`
