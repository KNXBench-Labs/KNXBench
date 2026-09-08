# Building-Part CRUD Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Give `BuildingPart` (building/floor/room/…) the same create/delete/rename/move CRUD surface `GroupRange` and `Line` already have — closes gap-analysis **T8**/**B4**.

**Architecture:** Four new `knx-core::Command` variants (`CreateBuildingPart`, `DeleteBuildingPart`, `RenameBuildingPart`, `MoveDeviceToBuildingPart`), mirroring the existing `CreateGroupRange`/`DeleteGroupRange`/`RenameGroupRange` (flat-list tree CRUD) and `MoveDeviceToLine` (reassignment) commands byte-for-byte in shape. Four new `knx-server` routes wired the same way as the group-range/move-device routes. Three `knx-web` UI additions: a create row in `ProjectExplorer`, a rename+delete block in `Inspector`'s building-part panel, and a move-field on `DeviceInspector` (mirrors `LineMoveField`).

**Tech Stack:** Rust (`knx-core`, `knx-server`, `axum`), TypeScript/React (`knx-web`, `vitest`).

**Spec:** This plan's design was approved in-chat (bounded-path brainstorming, no separate spec file) — see the chat transcript preceding this plan for the approved design summary. No `docs/superpowers/specs/` doc exists for this feature; this plan is authoritative.

## Global Constraints

- Every `Command` only ever targets `installations[0]` — same restriction every existing command in `command.rs` already has (its own doc comment). Do not add multi-installation support here.
- `installation.buildings` is a **flat** `Vec<BuildingPart>` linked by `parent`/`children` ids (see `building.rs`'s and `knx-projection`'s own doc comments) — never nest it in `knx-core`. Nesting into `BuildingNode` happens once, server-side, in `knx-projection` (already implemented, untouched by this plan).
- A device's building placement is **not exhaustive** the way topology placement is: a device can have zero or one building part. `None` after a move means "not placed anywhere", a normal state — not an error, and not equivalent to `MoveDeviceToLine`'s "explicitly unassigned" bucket. Do not reuse `remove_device_from_topology`'s error-on-absent behavior for buildings.
- No new `ValidationError` variants — mirror `CreateArea`/`CreateLine`'s existing style of returning bespoke `CommandError` variants directly, not routing through `validation.rs`, since building parts have no address-uniqueness constraint to check.
- Only `name`/`kind`/`parent` are settable from the UI this cycle. `number`, `default_line`, `completion` stay import-only fields (YAGNI — not in the approved design).
- Follow the "No co-author" + `github@knxbench.com` commit rule from this repo's own `CLAUDE.md` for every commit in this plan — this project's convention overrides the session's default Claude attribution footer.

---

## File Structure

- Modify `crates/knx-core/src/command.rs` — 4 new `Command` variants, 2 new `CommandError` variants, one new private helper (`remove_device_from_buildings`), tests.
- Modify `apps/knx-server/src/domain.rs` — 4 new `*_impl` functions, 1 new private `parse_building_part_kind` helper.
- Modify `apps/knx-server/src/routes.rs` — 4 new routes, 4 new body structs/handlers.
- Modify `apps/knx-server/tests/http_edit_routes.rs` — integration tests for the 4 new routes.
- Modify `apps/knx-web/src/api.ts` — 4 new client functions.
- Modify `apps/knx-web/src/api.test.ts` — tests for the 4 new client functions.
- Modify `apps/knx-web/src/treeUtils.ts` — export `flattenBuildingParts` (already exists, private), add `findDeviceBuildingPartInFirstInstallation`.
- Modify `apps/knx-web/src/treeUtils.test.ts` — tests for the new helper.
- Modify `apps/knx-web/src/ProjectExplorer.tsx` — new `NewBuildingPartRow`, thread `isFirst`/`onCreated` through `BuildingItem`.
- Modify `apps/knx-web/src/Inspector.tsx` — new `BuildingPartNameField`, new `BuildingPartMoveField`, update `BuildingPartInspector` and `DeviceInspector`.
- Modify `docs/GAP_ANALYSIS_ETS.md`, `docs/IMPLEMENTATION_STATUS.md`, `.ai/CURRENT_STATE.md` — close out T8.

---

### Task 1: `Command::CreateBuildingPart` / `DeleteBuildingPart`

**Files:**
- Modify: `crates/knx-core/src/command.rs`

**Interfaces:**
- Consumes: `crate::building::BuildingPart`/`BuildingPartType`, `crate::ids::BuildingPartId`, `crate::installation::Installation` — all pre-existing.
- Produces: `Command::CreateBuildingPart { part: BuildingPart }`, `Command::DeleteBuildingPart { id: BuildingPartId }`, `CommandError::BuildingPartNotFound(BuildingPartId)`, `CommandError::BuildingPartNotEmpty(BuildingPartId)` — consumed by Task 3 (move) and Task 4 (server).

- [ ] **Step 1: Write the failing tests**

Add to the `#[cfg(test)] mod tests` block at the bottom of `crates/knx-core/src/command.rs` (add `BuildingPartType` to the existing `use crate::building::BuildingPart;` test import — change it to `use crate::building::{BuildingPart, BuildingPartType};`):

```rust
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
            Command::DeleteBuildingPart { id: BuildingPartId(1) },
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
    project.installations[0]
        .buildings
        .push(test_building_part(BuildingPartId(1), BuildingPartType::Building, None));
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
    project.installations[0]
        .buildings
        .push(test_building_part(
            BuildingPartId(2),
            BuildingPartType::Floor,
            Some(BuildingPartId(1)),
        ));
    let mut stack = CommandStack::new();
    let result = stack.do_command(
        &mut project,
        Command::DeleteBuildingPart { id: BuildingPartId(1) },
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
        Command::DeleteBuildingPart { id: BuildingPartId(1) },
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
        Command::DeleteBuildingPart { id: BuildingPartId(99) },
    );
    assert_eq!(
        result,
        Err(CommandError::BuildingPartNotFound(BuildingPartId(99)))
    );
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p knx-core create_then_delete_building_part_round_trips_through_undo` (and the other 4 new test names)
Expected: FAIL to compile — `Command::CreateBuildingPart` does not exist yet.

- [ ] **Step 3: Add the `Command`/`CommandError` variants and `apply` arms**

In `crates/knx-core/src/command.rs`, change the `use crate::building::...` — there is no such import in non-test code yet, so add one. Change:

```rust
use crate::device::{ComObjectInstance, DeviceInstance};
```

to (insert the new line right above it, keeping the existing alphabetical-ish grouping):

```rust
use crate::building::BuildingPart;
use crate::device::{ComObjectInstance, DeviceInstance};
```

Change the `ids` import line:

```rust
use crate::ids::{AreaId, ComObjectInstanceId, DeviceId, GroupAddressId, GroupRangeId, LineId};
```

to:

```rust
use crate::ids::{
    AreaId, BuildingPartId, ComObjectInstanceId, DeviceId, GroupAddressId, GroupRangeId, LineId,
};
```

In the `Command` enum, insert after the `DeleteDevice` variant (right before `/// \`range.id\` is pre-allocated ... CreateGroupRange`):

```rust
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
```

In `CommandError`, insert after `DeviceHasLinks(DeviceId),`:

```rust
    BuildingPartNotFound(BuildingPartId),
    /// A `DeleteBuildingPart` was refused because it still has a child
    /// part or a device located in it.
    BuildingPartNotEmpty(BuildingPartId),
```

In `impl fmt::Display for CommandError`, insert after the `DeviceHasLinks` arm:

```rust
            CommandError::BuildingPartNotFound(id) => write!(f, "building part {id} not found"),
            CommandError::BuildingPartNotEmpty(id) => {
                write!(f, "building part {id} still has children or devices, cannot delete")
            }
```

In `impl Command { pub fn apply(...) }`, insert a new match arm after the `Command::DeleteDevice { .. } => { ... }` arm and before `Command::CreateGroupRange { range } => {`:

```rust
            Command::CreateBuildingPart { part } => {
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
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p knx-core building_part`
Expected: PASS (6 tests: the 5 above plus any pre-existing match).

- [ ] **Step 5: Commit**

```bash
git add crates/knx-core/src/command.rs
git commit -m "feat(knx-core): CreateBuildingPart/DeleteBuildingPart commands"
```

---

### Task 2: `Command::RenameBuildingPart`

**Files:**
- Modify: `crates/knx-core/src/command.rs`

**Interfaces:**
- Consumes: `Command::CreateBuildingPart`/`DeleteBuildingPart` from Task 1 (test setup only).
- Produces: `Command::RenameBuildingPart { id: BuildingPartId, name: String }` — consumed by Task 4 (server).

- [ ] **Step 1: Write the failing test**

Add to the test module:

```rust
#[test]
fn rename_building_part_then_undo_restores_previous_name() {
    let mut project = test_project_with_one_device(None);
    project.installations[0]
        .buildings
        .push(test_building_part(BuildingPartId(1), BuildingPartType::Room, None));
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
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p knx-core rename_building_part`
Expected: FAIL to compile — `Command::RenameBuildingPart` does not exist yet.

- [ ] **Step 3: Add the variant and `apply` arm**

In the `Command` enum, insert right after `DeleteBuildingPart { id: BuildingPartId },`:

```rust
    RenameBuildingPart {
        id: BuildingPartId,
        name: String,
    },
```

In `Command::apply`, insert right after the `Command::DeleteBuildingPart { id } => { ... }` arm:

```rust
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
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p knx-core rename_building_part`
Expected: PASS (2 tests).

- [ ] **Step 5: Commit**

```bash
git add crates/knx-core/src/command.rs
git commit -m "feat(knx-core): RenameBuildingPart command"
```

---

### Task 3: `Command::MoveDeviceToBuildingPart`

**Files:**
- Modify: `crates/knx-core/src/command.rs`

**Interfaces:**
- Consumes: `Command::CreateBuildingPart` from Task 1.
- Produces: `Command::MoveDeviceToBuildingPart { device: DeviceId, part: Option<BuildingPartId> }` — consumed by Task 4 (server).

- [ ] **Step 1: Write the failing tests**

Add to the test module:

```rust
#[test]
fn move_device_into_a_building_part_and_back_via_undo() {
    let mut project = test_project_with_one_device(None);
    project.installations[0]
        .buildings
        .push(test_building_part(BuildingPartId(1), BuildingPartType::Room, None));
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

#[test]
fn move_device_between_two_building_parts() {
    let mut project = test_project_with_one_device(None);
    project.installations[0]
        .buildings
        .push(test_building_part(BuildingPartId(1), BuildingPartType::Room, None));
    project.installations[0]
        .buildings
        .push(test_building_part(BuildingPartId(2), BuildingPartType::Room, None));
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
    project.installations[0]
        .buildings
        .push(test_building_part(BuildingPartId(1), BuildingPartType::Room, None));
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
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p knx-core move_device_to_building_part` (and the other new test names in this task)
Expected: FAIL to compile — `Command::MoveDeviceToBuildingPart` does not exist yet.

- [ ] **Step 3: Add the helper, variant, and `apply` arm**

Add a new private helper right after `remove_device_from_topology` (which stays untouched — this is a separate, deliberately non-exhaustive helper, not a variant of it):

```rust
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
```

In the `Command` enum, insert right after `RenameBuildingPart { id: BuildingPartId, name: String },`:

```rust
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
```

In `Command::apply`, insert right after the `Command::RenameBuildingPart { .. } => { ... }` arm:

```rust
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
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p knx-core move_device`
Expected: PASS (7 tests total: 5 new plus the 2 pre-existing `MoveDeviceToLine` ones sharing the name prefix).

- [ ] **Step 5: Run the full `knx-core` suite**

Run: `cargo test -p knx-core`
Expected: PASS, no regressions.

- [ ] **Step 6: Commit**

```bash
git add crates/knx-core/src/command.rs
git commit -m "feat(knx-core): MoveDeviceToBuildingPart command"
```

---

### Task 4: `knx-server` routes

**Files:**
- Modify: `apps/knx-server/src/domain.rs`
- Modify: `apps/knx-server/src/routes.rs`
- Test: `apps/knx-server/tests/http_edit_routes.rs`

**Interfaces:**
- Consumes: `knx_core::Command::CreateBuildingPart`/`DeleteBuildingPart`/`RenameBuildingPart`/`MoveDeviceToBuildingPart` from Tasks 1–3; `knx_core::BuildingPart`/`BuildingPartType`/`BuildingPartId`; the existing `apply(state, cmd) -> Result<ProjectTree, String>` helper and `Project::ids::next_building_part_id` in `domain.rs`.
- Produces: `domain::create_building_part_impl`, `domain::delete_building_part_impl`, `domain::rename_building_part_impl`, `domain::move_device_to_building_part_impl` (all `pub fn(&AppState, ...) -> Result<ProjectTree, String>`); routes `POST /api/building-parts`, `DELETE`/`PATCH /api/building-parts/{id}`, `POST /api/move-device-to-building-part` — consumed by Task 5 (`knx-web/src/api.ts`).

- [ ] **Step 1: Write the failing integration tests**

Add to `apps/knx-server/tests/http_edit_routes.rs`, after the `creating_a_nested_group_range_then_renaming_and_deleting_it` test (ends around line 582):

```rust
#[tokio::test]
async fn creating_a_nested_building_part_then_renaming_and_deleting_it() {
    let state = Arc::new(state_with_one_installation());
    let app = knx_server::app(state, None);

    let root = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/building-parts")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({ "name": "Main building", "kind": "Building" }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(root.status(), StatusCode::OK);
    let tree = body_json(root).await;
    let root_id = tree["installations"][0]["buildings"][0]["id"]
        .as_u64()
        .unwrap();

    let child = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/building-parts")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({ "name": "Floor 1", "kind": "Floor", "parentId": root_id })
                        .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(child.status(), StatusCode::OK);
    let tree = body_json(child).await;
    let child_id = tree["installations"][0]["buildings"][0]["children"][0]["id"]
        .as_u64()
        .unwrap();

    let renamed = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri(format!("/api/building-parts/{child_id}"))
                .header("content-type", "application/json")
                .body(Body::from(json!({ "name": "Ground floor" }).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(renamed.status(), StatusCode::OK);
    let tree = body_json(renamed).await;
    assert_eq!(
        tree["installations"][0]["buildings"][0]["children"][0]["name"],
        "Ground floor"
    );

    let delete_child = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/building-parts/{child_id}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(delete_child.status(), StatusCode::OK);

    let delete_root = app
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/building-parts/{root_id}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(delete_root.status(), StatusCode::OK);
    let tree = body_json(delete_root).await;
    assert!(tree["installations"][0]["buildings"]
        .as_array()
        .unwrap()
        .is_empty());
}

#[tokio::test]
async fn deleting_a_nonempty_building_part_is_a_400() {
    let state = Arc::new(state_with_one_installation());
    let app = knx_server::app(state, None);

    let root = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/building-parts")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({ "name": "Main building", "kind": "Building" }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    let tree = body_json(root).await;
    let root_id = tree["installations"][0]["buildings"][0]["id"]
        .as_u64()
        .unwrap();

    app.clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/building-parts")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({ "name": "Floor 1", "kind": "Floor", "parentId": root_id })
                        .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    let delete_root = app
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/building-parts/{root_id}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(delete_root.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn moving_a_device_into_a_building_part_and_back_out() {
    let state = Arc::new(state_with_one_installation_and_device());
    let app = knx_server::app(state, None);

    let create = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/building-parts")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({ "name": "Living room", "kind": "Room" }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    let tree = body_json(create).await;
    let part_id = tree["installations"][0]["buildings"][0]["id"]
        .as_u64()
        .unwrap();

    let moved = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/move-device-to-building-part")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({ "deviceId": 1, "partId": part_id }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(moved.status(), StatusCode::OK);
    let tree = body_json(moved).await;
    assert_eq!(
        tree["installations"][0]["buildings"][0]["devices"]
            .as_array()
            .unwrap()
            .len(),
        1
    );

    let back = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/move-device-to-building-part")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({ "deviceId": 1, "partId": null }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(back.status(), StatusCode::OK);
    let tree = body_json(back).await;
    assert!(tree["installations"][0]["buildings"][0]["devices"]
        .as_array()
        .unwrap()
        .is_empty());
}

#[tokio::test]
async fn creating_a_building_part_with_an_unknown_kind_is_a_400() {
    let state = Arc::new(state_with_one_installation());
    let app = knx_server::app(state, None);

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/building-parts")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({ "name": "X", "kind": "Basement" }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p knx-server --test http_edit_routes building_part`
Expected: FAIL — `404`/route-not-found (the routes don't exist yet), or compile stays fine (this test file doesn't reference new Rust symbols directly, only JSON/HTTP).

- [ ] **Step 3: Add the `domain.rs` functions**

In `apps/knx-server/src/domain.rs`, add after `rename_group_range_impl` (right before `fn parse_direction`):

```rust
fn parse_building_part_kind(kind: &str) -> Result<knx_core::BuildingPartType, String> {
    match kind {
        "Building" => Ok(knx_core::BuildingPartType::Building),
        "Floor" => Ok(knx_core::BuildingPartType::Floor),
        "Room" => Ok(knx_core::BuildingPartType::Room),
        "Corridor" => Ok(knx_core::BuildingPartType::Corridor),
        "DistributionBoard" => Ok(knx_core::BuildingPartType::DistributionBoard),
        "BuildingPart" => Ok(knx_core::BuildingPartType::BuildingPart),
        other => Err(format!(
            "unknown building-part kind '{other}', expected one of Building/Floor/Room/Corridor/DistributionBoard/BuildingPart"
        )),
    }
}

pub fn create_building_part_impl(
    state: &AppState,
    name: String,
    kind: String,
    parent_id: Option<u32>,
) -> Result<knx_projection::ProjectTree, String> {
    let kind = parse_building_part_kind(&kind)?;
    let cmd = {
        let mut project = state.project.lock().expect("state mutex poisoned");
        let project = project.as_mut().ok_or("no project open")?;
        let id = project.ids.next_building_part_id();
        knx_core::Command::CreateBuildingPart {
            part: knx_core::BuildingPart {
                id,
                source: knx_core::SourceRef {
                    path: format!("KB-Building-{}", id.0),
                    ets_id: format!("KB-Building-{}", id.0),
                },
                name,
                number: None,
                kind,
                default_line: None,
                completion: knx_core::CompletionStatus::Editing,
                children: vec![],
                devices: vec![],
                parent: parent_id.map(knx_core::BuildingPartId),
            },
        }
    };
    apply(state, cmd)
}

pub fn delete_building_part_impl(
    state: &AppState,
    id: u32,
) -> Result<knx_projection::ProjectTree, String> {
    apply(
        state,
        knx_core::Command::DeleteBuildingPart {
            id: knx_core::BuildingPartId(id),
        },
    )
}

pub fn rename_building_part_impl(
    state: &AppState,
    id: u32,
    name: String,
) -> Result<knx_projection::ProjectTree, String> {
    apply(
        state,
        knx_core::Command::RenameBuildingPart {
            id: knx_core::BuildingPartId(id),
            name,
        },
    )
}

pub fn move_device_to_building_part_impl(
    state: &AppState,
    device_id: u32,
    part_id: Option<u32>,
) -> Result<knx_projection::ProjectTree, String> {
    apply(
        state,
        knx_core::Command::MoveDeviceToBuildingPart {
            device: knx_core::DeviceId(device_id),
            part: part_id.map(knx_core::BuildingPartId),
        },
    )
}
```

- [ ] **Step 4: Add the routes**

In `apps/knx-server/src/routes.rs`, add to the `project_routes()` router, right after the `.route("/api/move-device", post(move_device_to_line))` line:

```rust
        .route("/api/move-device-to-building-part", post(move_device_to_building_part))
        .route("/api/building-parts", post(create_building_part))
        .route(
            "/api/building-parts/{id}",
            delete(delete_building_part).patch(rename_building_part),
        )
```

Add the body structs and handlers right after `rename_group_range` (before the `GroupLinkBody` block):

```rust
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateBuildingPartBody {
    name: String,
    kind: String,
    #[serde(default)]
    parent_id: Option<u32>,
}

async fn create_building_part(
    State(state): State<SharedState>,
    Json(body): Json<CreateBuildingPartBody>,
) -> Result<Json<knx_projection::ProjectTree>, ApiError> {
    domain::create_building_part_impl(&state, body.name, body.kind, body.parent_id)
        .map(Json)
        .map_err(ApiError::bad_request)
}

async fn delete_building_part(
    State(state): State<SharedState>,
    AxumPath(id): AxumPath<u32>,
) -> Result<Json<knx_projection::ProjectTree>, ApiError> {
    domain::delete_building_part_impl(&state, id)
        .map(Json)
        .map_err(ApiError::bad_request)
}

#[derive(Deserialize)]
struct RenameBuildingPartBody {
    name: String,
}

async fn rename_building_part(
    State(state): State<SharedState>,
    AxumPath(id): AxumPath<u32>,
    Json(body): Json<RenameBuildingPartBody>,
) -> Result<Json<knx_projection::ProjectTree>, ApiError> {
    domain::rename_building_part_impl(&state, id, body.name)
        .map(Json)
        .map_err(ApiError::bad_request)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct MoveDeviceToBuildingPartBody {
    device_id: u32,
    part_id: Option<u32>,
}

async fn move_device_to_building_part(
    State(state): State<SharedState>,
    Json(body): Json<MoveDeviceToBuildingPartBody>,
) -> Result<Json<knx_projection::ProjectTree>, ApiError> {
    domain::move_device_to_building_part_impl(&state, body.device_id, body.part_id)
        .map(Json)
        .map_err(ApiError::bad_request)
}
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test -p knx-server --test http_edit_routes building_part`
Expected: PASS (5 new tests).

- [ ] **Step 6: Run the full `knx-server` suite**

Run: `cargo test -p knx-server`
Expected: PASS, no regressions.

- [ ] **Step 7: Commit**

```bash
git add apps/knx-server/src/domain.rs apps/knx-server/src/routes.rs apps/knx-server/tests/http_edit_routes.rs
git commit -m "feat(knx-server): building-part CRUD + move-device-to-building-part routes"
```

---

### Task 5: `knx-web/src/api.ts` client functions

**Files:**
- Modify: `apps/knx-web/src/api.ts`
- Test: `apps/knx-web/src/api.test.ts`

**Interfaces:**
- Consumes: routes from Task 4 (`POST /api/building-parts`, `DELETE`/`PATCH /api/building-parts/{id}`, `POST /api/move-device-to-building-part`); the existing `request<T>` helper and `ProjectTree` type in `api.ts`.
- Produces: `api.createBuildingPart(name: string, kind: string, parentId?: number): Promise<ProjectTree>`, `api.deleteBuildingPart(id: number): Promise<ProjectTree>`, `api.renameBuildingPart(id: number, name: string): Promise<ProjectTree>`, `api.moveDeviceToBuildingPart(deviceId: number, partId: number | null): Promise<ProjectTree>` — consumed by Tasks 7 and 8.

- [ ] **Step 1: Write the failing tests**

Add to `apps/knx-web/src/api.test.ts`, right after the `renameGroupRange issues a PATCH...` test (before the `linkComObject...` test):

```ts
  it("createBuildingPart posts to /api/building-parts with camelCase parentId", async () => {
    mockFetchOnce({ installations: [] });
    await api.createBuildingPart("Main building", "Building", 5);
    const [url, init] = (fetch as ReturnType<typeof vi.fn>).mock.calls[0];
    expect(url).toBe("/api/building-parts");
    expect(JSON.parse(init.body as string)).toEqual({
      name: "Main building",
      kind: "Building",
      parentId: 5,
    });
  });

  it("deleteBuildingPart issues a DELETE to /api/building-parts/:id", async () => {
    mockFetchOnce({ installations: [] });
    await api.deleteBuildingPart(9);
    const [url, init] = (fetch as ReturnType<typeof vi.fn>).mock.calls[0];
    expect(url).toBe("/api/building-parts/9");
    expect(init.method).toBe("DELETE");
  });

  it("renameBuildingPart issues a PATCH with the new name", async () => {
    mockFetchOnce({ installations: [] });
    await api.renameBuildingPart(9, "Living room");
    const [url, init] = (fetch as ReturnType<typeof vi.fn>).mock.calls[0];
    expect(url).toBe("/api/building-parts/9");
    expect(init.method).toBe("PATCH");
    expect(JSON.parse(init.body as string)).toEqual({ name: "Living room" });
  });

  it("moveDeviceToBuildingPart posts a part id", async () => {
    mockFetchOnce({ installations: [] });
    await api.moveDeviceToBuildingPart(9, 7);
    const [url, init] = (fetch as ReturnType<typeof vi.fn>).mock.calls[0];
    expect(url).toBe("/api/move-device-to-building-part");
    expect(JSON.parse(init.body as string)).toEqual({ deviceId: 9, partId: 7 });
  });

  it("moveDeviceToBuildingPart posts null to un-place the device", async () => {
    mockFetchOnce({ installations: [] });
    await api.moveDeviceToBuildingPart(9, null);
    const [, init] = (fetch as ReturnType<typeof vi.fn>).mock.calls[0];
    expect(JSON.parse(init.body as string)).toEqual({ deviceId: 9, partId: null });
  });
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd apps/knx-web && npx vitest run api.test.ts`
Expected: FAIL — `api.createBuildingPart is not a function` (and similar for the other 3).

- [ ] **Step 3: Add the client functions**

In `apps/knx-web/src/api.ts`, add right after `renameGroupRange` (before the `linkComObject` block):

```ts
export function createBuildingPart(
  name: string,
  kind: string,
  parentId?: number,
): Promise<ProjectTree> {
  return request("/api/building-parts", {
    method: "POST",
    body: JSON.stringify({ name, kind, parentId }),
  });
}

export function deleteBuildingPart(id: number): Promise<ProjectTree> {
  return request(`/api/building-parts/${id}`, { method: "DELETE" });
}

export function renameBuildingPart(id: number, name: string): Promise<ProjectTree> {
  return request(`/api/building-parts/${id}`, {
    method: "PATCH",
    body: JSON.stringify({ name }),
  });
}

export function moveDeviceToBuildingPart(
  deviceId: number,
  partId: number | null,
): Promise<ProjectTree> {
  return request("/api/move-device-to-building-part", {
    method: "POST",
    body: JSON.stringify({ deviceId, partId }),
  });
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cd apps/knx-web && npx vitest run api.test.ts`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add apps/knx-web/src/api.ts apps/knx-web/src/api.test.ts
git commit -m "feat(knx-web): building-part CRUD + move-device-to-building-part client calls"
```

---

### Task 6: `treeUtils.ts` — `findDeviceBuildingPartInFirstInstallation`

**Files:**
- Modify: `apps/knx-web/src/treeUtils.ts`
- Test: `apps/knx-web/src/treeUtils.test.ts`

**Interfaces:**
- Consumes: `flattenBuildingParts` (already defined in this file, currently module-private — this task exports it), `ProjectTree`/`BuildingNode` types.
- Produces: `export function flattenBuildingParts(nodes: BuildingNode[], parentPath: string[]): { node: BuildingNode; path: string }[]` (existing function, newly exported), `export function findDeviceBuildingPartInFirstInstallation(tree: ProjectTree, deviceId: number): number | null` — consumed by Task 7 (`ProjectExplorer` reachability is not needed there) and Task 8 (`Inspector`'s `canEdit` check and `BuildingPartMoveField`'s current value).

- [ ] **Step 1: Write the failing tests**

Add to `apps/knx-web/src/treeUtils.test.ts`, right after the `findDeviceLineInFirstInstallation` describe block (after line 268's closing `});`):

```ts
describe("findDeviceBuildingPartInFirstInstallation", () => {
  it("finds a device nested inside a building part", () => {
    const t = tree([
      installation({
        buildings: [building(1, "Main building", "Building", [
          building(2, "Living room", "Room", [], [device(9, "Switch")]),
        ])],
      }),
    ]);
    expect(findDeviceBuildingPartInFirstInstallation(t, 9)).toBe(2);
  });

  it("returns null for a device not placed in any building part", () => {
    const t = tree([installation({ buildings: [building(1, "Main building", "Building")] })]);
    expect(findDeviceBuildingPartInFirstInstallation(t, 9)).toBeNull();
  });

  it("returns null when there is no first installation at all", () => {
    expect(findDeviceBuildingPartInFirstInstallation(tree([]), 9)).toBeNull();
  });
});
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd apps/knx-web && npx vitest run treeUtils.test.ts`
Expected: FAIL — `findDeviceBuildingPartInFirstInstallation is not a function`.

- [ ] **Step 3: Export `flattenBuildingParts` and add the new helper**

In `apps/knx-web/src/treeUtils.ts`, change:

```ts
function flattenBuildingParts(
```

to:

```ts
export function flattenBuildingParts(
```

Add the new function at the end of the file, right after `findDeviceLineInFirstInstallation`:

```ts

// The building-part counterpart of `findDeviceLineInFirstInstallation`.
// Building placement isn't exhaustive the way topology placement is
// (`Command::MoveDeviceToBuildingPart`'s own doc comment) — a device
// with no building part at all is a normal state, not a third bucket to
// distinguish from "unreachable" the way `null` vs `undefined` does for
// lines. So this only ever returns a part id or `null`; the "is this
// device even reachable from installations[0]" question is already
// answered by `findDeviceLineInFirstInstallation` wherever both fields
// are shown together (`DeviceInspector`), since both commands share the
// same `installations[0]`-only restriction.
export function findDeviceBuildingPartInFirstInstallation(
  tree: ProjectTree,
  deviceId: number,
): number | null {
  const inst = tree.installations[0];
  if (!inst) return null;
  for (const { node } of flattenBuildingParts(inst.buildings, [])) {
    if (node.devices.some((d) => d.id === deviceId)) return node.id;
  }
  return null;
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cd apps/knx-web && npx vitest run treeUtils.test.ts`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add apps/knx-web/src/treeUtils.ts apps/knx-web/src/treeUtils.test.ts
git commit -m "feat(knx-web): findDeviceBuildingPartInFirstInstallation"
```

---

### Task 7: `ProjectExplorer.tsx` — create UI

**Files:**
- Modify: `apps/knx-web/src/ProjectExplorer.tsx`

**Interfaces:**
- Consumes: `api.createBuildingPart` (Task 5), `BuildingNode` type, existing `TreeNode`/`DeviceItem` components.
- Produces: `NewBuildingPartRow` component; `BuildingItem` gains `isFirst`/`onCreated` props (breaking change to its existing call sites in this same file, both updated in this task).

No dedicated test file exists for `ProjectExplorer.tsx` today (no React Testing Library render tests in this codebase for any tree component — verified: `ls apps/knx-web/src/*.test.tsx` finds none). This task is manual-verification only, consistent with `GroupRangeItem`/`NewGroupRangeRow`'s own lack of a render test.

- [ ] **Step 1: Add `NewBuildingPartRow`**

In `apps/knx-web/src/ProjectExplorer.tsx`, add right after `NewGroupRangeRow` (before `function GroupRangeItem`):

```tsx
const BUILDING_PART_KINDS = [
  "Building",
  "Floor",
  "Room",
  "Corridor",
  "DistributionBoard",
  "BuildingPart",
] as const;

// The building-part counterpart of `NewGroupRangeRow`. `parentId` is
// `undefined` when rendered directly under the "Buildings" branch
// (creates a root part) and set to a `BuildingItem`'s own id when
// rendered under that item (creates a nested part) — unlike
// `NewGroupRangeRow`, nesting isn't capped at one level here: every
// `BuildingItem` gets its own row, since `BuildingPart` has no depth
// limit (`building.rs`'s own doc comment), unlike `GroupRange`'s
// observed two-level depth.
function NewBuildingPartRow(props: {
  parentId?: number;
  onCreated: (tree: ProjectTree) => void;
}) {
  const { parentId, onCreated } = props;
  const [name, setName] = useState("");
  const [kind, setKind] = useState<(typeof BUILDING_PART_KINDS)[number]>("Room");
  const [error, setError] = useState<string | null>(null);
  const canCreate = name.trim() !== "";

  async function create() {
    if (!canCreate) return;
    setError(null);
    try {
      const tree = await api.createBuildingPart(name, kind, parentId);
      onCreated(tree);
      setName("");
    } catch (e) {
      setError(api.errorMessage(e));
    }
  }

  return (
    <li className="tree-new-row">
      <select value={kind} onChange={(e) => setKind(e.target.value as typeof kind)}>
        {BUILDING_PART_KINDS.map((k) => (
          <option key={k} value={k}>
            {k}
          </option>
        ))}
      </select>
      <input
        value={name}
        placeholder={parentId === undefined ? "New building" : "New building part"}
        onChange={(e) => setName(e.target.value)}
        onKeyDown={(e) => {
          if (e.key === "Enter") void create();
        }}
      />
      <button onClick={create} disabled={!canCreate}>
        Add
      </button>
      {error && <span className="field-error">{error}</span>}
    </li>
  );
}
```

- [ ] **Step 2: Thread `isFirst`/`onCreated` through `BuildingItem`**

Replace the existing `BuildingItem` function:

```tsx
function BuildingItem(props: { building: BuildingNode } & SelectionProps) {
  const { building, selection, onSelect } = props;
  return (
    <TreeNode
      label={`${building.name} (${building.kind})`}
      selected={selection?.kind === "building_part" && selection.id === building.id}
      onSelect={() => onSelect({ kind: "building_part", id: building.id })}
    >
      {building.children.map((c) => (
        <BuildingItem key={c.id} building={c} selection={selection} onSelect={onSelect} />
      ))}
      {building.devices.map((d) => (
        <DeviceItem key={d.id} device={d} selection={selection} onSelect={onSelect} />
      ))}
    </TreeNode>
  );
}
```

with:

```tsx
function BuildingItem(
  props: {
    building: BuildingNode;
    isFirst: boolean;
    onCreated: (tree: ProjectTree) => void;
  } & SelectionProps,
) {
  const { building, isFirst, onCreated, selection, onSelect } = props;
  return (
    <TreeNode
      label={`${building.name} (${building.kind})`}
      selected={selection?.kind === "building_part" && selection.id === building.id}
      onSelect={() => onSelect({ kind: "building_part", id: building.id })}
    >
      {building.children.map((c) => (
        <BuildingItem
          key={c.id}
          building={c}
          isFirst={isFirst}
          onCreated={onCreated}
          selection={selection}
          onSelect={onSelect}
        />
      ))}
      {building.devices.map((d) => (
        <DeviceItem key={d.id} device={d} selection={selection} onSelect={onSelect} />
      ))}
      {isFirst && <NewBuildingPartRow parentId={building.id} onCreated={onCreated} />}
    </TreeNode>
  );
}
```

- [ ] **Step 3: Update the "Buildings" branch in `InstallationItem`**

Replace:

```tsx
      <TreeNode label="Buildings">
        {installation.buildings.map((b) => (
          <BuildingItem key={b.id} building={b} selection={selection} onSelect={onSelect} />
        ))}
      </TreeNode>
```

with:

```tsx
      <TreeNode label="Buildings">
        {installation.buildings.map((b) => (
          <BuildingItem
            key={b.id}
            building={b}
            isFirst={isFirst}
            onCreated={onTreeUpdate}
            selection={selection}
            onSelect={onSelect}
          />
        ))}
        {isFirst && <NewBuildingPartRow onCreated={onTreeUpdate} />}
      </TreeNode>
```

- [ ] **Step 4: Verify it compiles and the existing suite still passes**

Run: `cd apps/knx-web && npx tsc --noEmit && npx vitest run`
Expected: PASS, no type errors, no regressions.

- [ ] **Step 5: Commit**

```bash
git add apps/knx-web/src/ProjectExplorer.tsx
git commit -m "feat(knx-web): create building parts from the Project Explorer"
```

---

### Task 8: `Inspector.tsx` — rename, delete, move UI

**Files:**
- Modify: `apps/knx-web/src/Inspector.tsx`

**Interfaces:**
- Consumes: `api.deleteBuildingPart`/`renameBuildingPart`/`moveDeviceToBuildingPart` (Task 5), `flattenBuildingParts`/`findDeviceBuildingPartInFirstInstallation` (Task 6), `findDeviceLineInFirstInstallation` (existing) for the shared installations[0]-reachability gate.

Same "no render-test file for `Inspector.tsx`" note as Task 7 applies — `GroupRangeInspector`/`LineMoveField` have no render test either. Manual-verification only.

- [ ] **Step 1: Add the new imports**

In `apps/knx-web/src/Inspector.tsx`, change:

```ts
import {
  findArea,
  findBuildingPart,
  findDeviceLineInFirstInstallation,
  findGroupAddress,
  findGroupRange,
  findLine,
} from "./treeUtils";
```

to:

```ts
import {
  findArea,
  findBuildingPart,
  findDeviceBuildingPartInFirstInstallation,
  findDeviceLineInFirstInstallation,
  findGroupAddress,
  findGroupRange,
  findLine,
  flattenBuildingParts,
} from "./treeUtils";
```

- [ ] **Step 2: Add `BuildingPartMoveField` and wire it into `DeviceInspector`**

Add right after `LineMoveField`'s closing `}` (before `function DeviceInspector`):

```tsx
// The building-part counterpart of `LineMoveField`, via
// `Command::MoveDeviceToBuildingPart`. Unlike `LineMoveField`, `null` in
// the select means "not placed in any building part" — a normal steady
// state, not a bucket the device is moved *into* the way `unassigned`
// is for `MoveDeviceToLine` — so there is no dedicated "(unassigned)"
// semantic beyond the same empty option every optional select here
// uses. Rendering is gated by `findDeviceLineInFirstInstallation`, not
// a building-specific check: both commands share the same
// `installations[0]`-only restriction, and that helper already reports
// it accurately (`current === undefined` below).
function BuildingPartMoveField(props: {
  detail: DeviceDetail;
  tree: ProjectTree;
  onApplied: (tree: ProjectTree) => void;
}) {
  const { detail, tree, onApplied } = props;
  const current = findDeviceLineInFirstInstallation(tree, detail.id);
  const currentPart = findDeviceBuildingPartInFirstInstallation(tree, detail.id);
  const [error, setError] = useState<string | null>(null);

  if (current === undefined) return null;

  async function move(partId: number | null) {
    setError(null);
    try {
      const tree = await api.moveDeviceToBuildingPart(detail.id, partId);
      onApplied(tree);
    } catch (e) {
      setError(api.errorMessage(e));
    }
  }

  const parts = flattenBuildingParts(tree.installations[0]?.buildings ?? [], []);

  return (
    <label className="inspector-field">
      Building part
      <select
        value={currentPart ?? ""}
        onChange={(e) => move(e.target.value === "" ? null : Number(e.target.value))}
      >
        <option value="">(none)</option>
        {parts.map(({ node, path }) => (
          <option key={node.id} value={node.id}>
            {path}
          </option>
        ))}
      </select>
      {error && <span className="field-error">{error}</span>}
    </label>
  );
}
```

In `DeviceInspector`'s render body, change:

```tsx
      <AddressField detail={detail} onApplied={onApplied} />
      <LineMoveField detail={detail} tree={tree} onApplied={onApplied} />
      <DeviceDescriptionField detail={detail} onApplied={onApplied} />
```

to:

```tsx
      <AddressField detail={detail} onApplied={onApplied} />
      <LineMoveField detail={detail} tree={tree} onApplied={onApplied} />
      <BuildingPartMoveField detail={detail} tree={tree} onApplied={onApplied} />
      <DeviceDescriptionField detail={detail} onApplied={onApplied} />
```

- [ ] **Step 3: Add `BuildingPartNameField` and update `BuildingPartInspector`**

Replace the existing `BuildingPartInspector`:

```tsx
function BuildingPartInspector(props: { node: BuildingNode; path: string }) {
  const { node, path } = props;
  return (
    <div className="inspector">
      <h2>{node.name}</h2>
      <p className="inspector-description">{node.kind}</p>
      <p className="inspector-path">{path}</p>
      <p>
        {node.devices.length} device{node.devices.length === 1 ? "" : "s"},{" "}
        {node.children.length} child part{node.children.length === 1 ? "" : "s"}
      </p>
    </div>
  );
}
```

with:

```tsx
function BuildingPartNameField(props: {
  part: BuildingNode;
  onApplied: (tree: ProjectTree) => void;
}) {
  const { part, onApplied } = props;
  const [value, setValue] = useState(part.name);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    setValue(part.name);
    setError(null);
  }, [part.name]);

  async function apply() {
    if (value === part.name || value.trim() === "") {
      setValue(part.name);
      return;
    }
    setError(null);
    try {
      const tree = await api.renameBuildingPart(part.id, value);
      onApplied(tree);
    } catch (e) {
      setError(api.errorMessage(e));
      setValue(part.name);
    }
  }

  return (
    <label className="inspector-field">
      Name
      <input
        value={value}
        onChange={(e) => setValue(e.target.value)}
        onBlur={apply}
        onKeyDown={(e) => {
          if (e.key === "Enter") (e.target as HTMLInputElement).blur();
        }}
      />
      {error && <span className="field-error">{error}</span>}
    </label>
  );
}

function BuildingPartInspector(props: {
  node: BuildingNode;
  path: string;
  // Same `installations[0]`-only gate as `GroupRangeInspector`'s
  // `canEdit` — `Command::CreateBuildingPart`/`DeleteBuildingPart`/
  // `RenameBuildingPart` only ever search the first installation
  // (command.rs).
  canEdit: boolean;
  onApplied: (tree: ProjectTree) => void;
  onDeleted: (tree: ProjectTree) => void;
}) {
  const { node, path, canEdit, onApplied, onDeleted } = props;
  const [error, setError] = useState<string | null>(null);

  async function remove() {
    setError(null);
    try {
      const tree = await api.deleteBuildingPart(node.id);
      onDeleted(tree);
    } catch (e) {
      // Also where `CommandError::BuildingPartNotEmpty` surfaces — the
      // server refuses to delete a part that still has a child or a
      // device, so the user sees why instead of a silent no-op.
      setError(api.errorMessage(e));
    }
  }

  return (
    <div className="inspector">
      <h2>{node.name}</h2>
      <p className="inspector-description">{node.kind}</p>
      <p className="inspector-path">{path}</p>
      <p>
        {node.devices.length} device{node.devices.length === 1 ? "" : "s"},{" "}
        {node.children.length} child part{node.children.length === 1 ? "" : "s"}
      </p>
      {canEdit ? (
        <>
          <BuildingPartNameField part={node} onApplied={onApplied} />
          <button onClick={remove}>Delete</button>
        </>
      ) : (
        <p className="inspector-description">
          Rename and Delete are only available for building parts in the first installation.
        </p>
      )}
      {error && <span className="field-error">{error}</span>}
    </div>
  );
}
```

- [ ] **Step 4: Update the call site in the default-exported `Inspector`**

Change:

```tsx
  const found = findBuildingPart(tree, selection.id);
  if (!found) return null;
  return <BuildingPartInspector node={found.node} path={found.path} />;
}
```

to:

```tsx
  const found = findBuildingPart(tree, selection.id);
  if (!found) return null;
  const canEdit = flattenBuildingParts(tree.installations[0]?.buildings ?? [], []).some(
    ({ node }) => node.id === found.node.id,
  );
  return (
    <BuildingPartInspector
      node={found.node}
      path={found.path}
      canEdit={canEdit}
      onApplied={onApplied}
      onDeleted={onDeleted}
    />
  );
}
```

- [ ] **Step 5: Verify it compiles and the existing suite still passes**

Run: `cd apps/knx-web && npx tsc --noEmit && npx vitest run`
Expected: PASS, no type errors, no regressions.

- [ ] **Step 6: Commit**

```bash
git add apps/knx-web/src/Inspector.tsx
git commit -m "feat(knx-web): rename/delete building parts, move a device between them"
```

---

### Task 9: Close out T8 in the docs and handover state

**Files:**
- Modify: `docs/GAP_ANALYSIS_ETS.md`
- Modify: `docs/IMPLEMENTATION_STATUS.md`
- Modify: `.ai/CURRENT_STATE.md`

**Interfaces:**
- Consumes: nothing — this is a documentation-only task, run last.
- Produces: nothing consumed by other tasks; this is the final task in the plan.

- [ ] **Step 1: Rewrite T8's bullet in `docs/GAP_ANALYSIS_ETS.md`**

Find (around line 238):

```markdown
- **T8. Building-part CRUD commands.** `CreateBuildingPart`/
  `DeleteBuildingPart`/`RenameBuildingPart`/`MoveDeviceToBuildingPart`.
  Closes **B4**.
```

Replace with (mirror T7's own "Done (date)" rewrite immediately above it in the same file):

```markdown
- **T8. Building-part CRUD commands. Done (2026-09-08).**
  `CreateBuildingPart`/`DeleteBuildingPart`/`RenameBuildingPart`/
  `MoveDeviceToBuildingPart` land in `knx-core`, mirroring
  `CreateGroupRange`/`DeleteGroupRange`/`RenameGroupRange`'s flat-list
  tree-CRUD shape — `installation.buildings` was already a flat
  `Vec<BuildingPart>` linked by `parent`/`children` ids (ADR-0009), so
  no new nesting logic was needed, only the commands to mutate it.
  Building placement, unlike topology placement, isn't exhaustive: a
  device can have zero or one building part, so `MoveDeviceToBuildingPart`
  targeting `None` means "not placed" rather than a tracked
  "unassigned" bucket the way `MoveDeviceToLine` has one. `apps/knx-web`
  gains a building-part create row in the Project Explorer (unbounded
  nesting depth, unlike group ranges' observed two levels) and a
  rename/delete/move UI in the Inspector, mirroring `GroupRangeInspector`/
  `LineMoveField`. Closes **B4**.
```

- [ ] **Step 2: Add a T8 entry to `docs/IMPLEMENTATION_STATUS.md`**

Add right after the existing T7 entry (ends around line 1096, after `**B6**.`):

```markdown

**T8, building-part CRUD (2026-09-08).** `crates/knx-core` gains
`Command::CreateBuildingPart`/`DeleteBuildingPart`/`RenameBuildingPart`/
`MoveDeviceToBuildingPart`, the same flat-list tree-CRUD shape
`CreateGroupRange`/`DeleteGroupRange`/`RenameGroupRange` already use —
`installation.buildings` was already flat, linked by `parent`/`children`
ids (ADR-0009), so this only adds the commands, not the nesting. A new
`remove_device_from_buildings` helper deliberately does *not* mirror
`remove_device_from_topology`'s error-on-absent behavior: building
placement isn't exhaustive (a device can have no building part at all),
so "not found anywhere" is a normal `None`, not a `CommandError`.
`apps/knx-server` gains `POST /api/building-parts`,
`DELETE`/`PATCH /api/building-parts/{id}`, and
`POST /api/move-device-to-building-part`, parsing the wire-format kind
string the same way `parse_direction`/`parse_com_flag_kind` already
parse their own enums. `apps/knx-web` gains a `NewBuildingPartRow` in
`ProjectExplorer` (every `BuildingItem` gets one, since building parts
nest to unbounded depth, unlike group ranges' two-level cap), and in
`Inspector`, a `BuildingPartNameField`/Delete button on the building-part
panel plus a `BuildingPartMoveField` on `DeviceInspector` — both mirror
`GroupRangeInspector`/`LineMoveField`'s own shape. `treeUtils.ts` gains
`findDeviceBuildingPartInFirstInstallation` and exports the
previously-private `flattenBuildingParts`. New `cargo test` tests in
`knx-core` (create/delete/rename/move round trips, plus the not-found/
not-empty rejection paths) and `knx-server` (5 HTTP integration tests),
and 9 new `vitest` tests (`api.test.ts` x5, `treeUtils.test.ts` x4).
Closes **T8** ([GAP_ANALYSIS_ETS.md](GAP_ANALYSIS_ETS.md)), **B4**.
```

- [ ] **Step 3: Update `.ai/CURRENT_STATE.md`**

Replace the file's contents with (following the format `CLAUDE.md`'s Handover Protocol requires):

```markdown
- **Last Agent:** Claude
- **Timestamp:** 2026-09-08 <fill in actual time>
- **Completed:** T8, building-part CRUD. `knx-core` gains
  `Command::CreateBuildingPart`/`DeleteBuildingPart`/`RenameBuildingPart`/
  `MoveDeviceToBuildingPart`, mirroring `CreateGroupRange`/`DeleteGroupRange`/
  `RenameGroupRange`'s flat-list tree-CRUD shape (`installation.buildings`
  was already flat + `parent`/`children` ids, ADR-0009). Building
  placement is not exhaustive like topology placement — a device can
  have zero building parts, so `MoveDeviceToBuildingPart`'s `None` means
  "not placed", not a tracked bucket. `apps/knx-server` gains
  `POST /api/building-parts`, `DELETE`/`PATCH /api/building-parts/{id}`,
  `POST /api/move-device-to-building-part`. `apps/knx-web` gains a
  `NewBuildingPartRow` (unbounded nesting, one per `BuildingItem`) in
  `ProjectExplorer`, and rename/delete/move UI in `Inspector`
  (`BuildingPartNameField`, `BuildingPartMoveField`), mirroring
  `GroupRangeInspector`/`LineMoveField`. `docs/GAP_ANALYSIS_ETS.md` and
  `docs/IMPLEMENTATION_STATUS.md` updated (T8 closed, B4 closed).
- **Pending/Next Steps:** Tier 2's remaining item: T9 (bulk/multi-select
  operations — flagged in `GAP_ANALYSIS_ETS.md` as needing its own design
  spec for a `Command::Batch` wrapper before implementation, unlike every
  other item in this backlog so far). Tier 3 (export & reporting parity,
  starting at T10) is next after that.
- **Notes for Codex:** Work happened in a git worktree per this project's
  usual flow; merge/rebase onto `main` before continuing from here. No
  new `ValidationError` variant was needed for T8 — `CommandError::
  BuildingPartNotFound`/`BuildingPartNotEmpty` are returned directly from
  `Command::apply`, the same style `CreateArea`/`CreateLine`/`DeleteArea`/
  `DeleteLine` already use (no address-uniqueness constraint exists for
  building parts, so there was nothing for `validation.rs` to check).
```

- [ ] **Step 4: Commit**

```bash
git add docs/GAP_ANALYSIS_ETS.md docs/IMPLEMENTATION_STATUS.md .ai/CURRENT_STATE.md
git commit -m "docs: close T8 (building-part CRUD)"
```
