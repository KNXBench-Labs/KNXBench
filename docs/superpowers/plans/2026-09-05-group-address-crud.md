# Group Address Create/Delete UI Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Let a user create and delete group addresses from `knx-desktop`'s UI, closing the last gap in `Command::CreateGroupAddress`/`DeleteGroupAddress` — the core/store layers have carried both since Session 5 cycle 4, but no UI has ever reached them.

**Architecture:** No new subsystem. Two data-integrity guards land in `knx-core`'s existing command/validation layer (duplicate group address on create, in-use refusal on delete). Two new Tauri commands wrap the existing `Command` variants the same way `set_individual_address`/`set_com_object_dpt` already do. The frontend adds a "Group Addresses" tree branch (existing `GroupAddressNode` data, never rendered as a branch before — only reachable via Search) with an inline create row, plus a Delete button on the existing `GroupAddressInspector`.

**Tech Stack:** Rust (knx-core, knx-desktop-lib/Tauri), TypeScript/React (knx-desktop frontend), vitest, cargo test.

**Spec:** No separate spec file — this is a bounded task; the design was agreed in chat directly (see the two paragraphs above, which are that design verbatim).

## Global Constraints

- `Command::apply` only ever targets `project.installations.first_mut()` (documented in `command.rs`) — the frontend's create-row therefore only renders for the first installation in the list, so the affordance never implies it could target a different one.
- `GroupAddressEntry.source` (`SourceRef`) has no ETS origin for a UI-created entry — use `SourceRef { path: String::new(), ets_id: String::new() }`, the first UI-created domain object in this codebase.
- `GroupAddressId` allocation goes through `project.ids.next_group_address_id()` (`IdAllocators`, `project.rs`) — never invent an id client-side.
- No new frontend test tooling: the project has no JSX component-test setup today (`ProjectExplorer.tsx`/`Inspector.tsx`/`App.tsx` have zero `.test.tsx` files). Follow that precedent — cover the new Rust logic with `cargo test`, leave a manual smoke-check note in `IMPLEMENTATION_STATUS.md` the same way cycles 4–7 did.
- Rust workspace commands: `cargo test -p knx-core`, `cargo test -p knx-desktop`. Frontend: `cd apps/knx-desktop && npm run test` and `npm run build` (the latter runs `tsc`, catching any prop-typing mistake).

---

### Task 1: `knx-core` — duplicate-address and in-use validation

**Files:**
- Modify: `crates/knx-core/src/validation.rs`
- Modify: `crates/knx-core/src/command.rs`

**Interfaces:**
- Consumes: `Installation` (`installation.rs`), `GroupAddress`/`GroupAddressId` (`address.rs`/`ids.rs`), `Devices::com_objects()` (`devices.rs`, already public), `GroupLink` (`flags.rs`).
- Produces: `ValidationError::DuplicateGroupAddress { address: GroupAddress, existing: GroupAddressId, new: GroupAddressId }`, `pub fn check_no_duplicate_group_address(installation: &Installation, candidate: GroupAddressId, address: GroupAddress) -> Result<(), ValidationError>`, `CommandError::GroupAddressInUse(GroupAddressId)` — both consumed by Task 2's Tauri layer via `Command::apply`'s existing `Result<Command, CommandError>` return.

- [ ] **Step 1: Add the failing validation test**

In `crates/knx-core/src/validation.rs`, inside `#[cfg(test)] mod tests`, add (after the existing tests, using the same `test_source`/fixture style already in that module):

```rust
#[test]
fn check_no_duplicate_group_address_rejects_a_second_entry_with_the_same_address() {
    let installation = Installation {
        id: crate::ids::InstallationId(0),
        name: "I".into(),
        default_line: None,
        multicast_address: None,
        completion: crate::commissioning::CompletionStatus::FinishedDesign,
        topology: crate::topology::Topology {
            areas: vec![],
            lines: vec![],
            unassigned: vec![],
        },
        buildings: vec![],
        group_ranges: vec![],
        group_addresses: vec![crate::group::GroupAddressEntry {
            id: GroupAddressId(1),
            source: test_source(),
            name: "Existing".into(),
            address: GroupAddress::from_raw(5),
            central: false,
            unfiltered: false,
            range: None,
        }],
        parameters: vec![],
    };
    let result =
        check_no_duplicate_group_address(&installation, GroupAddressId(2), GroupAddress::from_raw(5));
    assert!(matches!(
        result,
        Err(ValidationError::DuplicateGroupAddress { existing, new, .. })
            if existing == GroupAddressId(1) && new == GroupAddressId(2)
    ));
    // An entry keeping its own current address is not a duplicate of itself.
    assert!(check_no_duplicate_group_address(&installation, GroupAddressId(1), GroupAddress::from_raw(5)).is_ok());
}
```

- [ ] **Step 2: Run it to confirm it fails to compile**

Run: `cargo test -p knx-core check_no_duplicate_group_address_rejects`
Expected: FAIL — `check_no_duplicate_group_address` and `ValidationError::DuplicateGroupAddress` do not exist yet.

- [ ] **Step 3: Add the validation error variant and rule**

In `crates/knx-core/src/validation.rs`, add the variant to `ValidationError` (after `DanglingGroupLink`):

```rust
    DuplicateGroupAddress {
        address: GroupAddress,
        existing: GroupAddressId,
        new: GroupAddressId,
    },
```

Add its `Display` arm (after the `DanglingGroupLink` arm):

```rust
            ValidationError::DuplicateGroupAddress {
                address,
                existing,
                new,
            } => write!(
                f,
                "group address {} already used by group address {existing}, cannot assign to group address {new}",
                address.raw()
            ),
```

Add the rule function (after `check_no_duplicate_individual_address`):

```rust
/// Rejects assigning `address` to `candidate` if any other group address
/// entry in `installation` already has it. An entry keeping its own
/// current address is not a duplicate.
pub fn check_no_duplicate_group_address(
    installation: &Installation,
    candidate: GroupAddressId,
    address: GroupAddress,
) -> Result<(), ValidationError> {
    for entry in &installation.group_addresses {
        if entry.id != candidate && entry.address == address {
            return Err(ValidationError::DuplicateGroupAddress {
                address,
                existing: entry.id,
                new: candidate,
            });
        }
    }
    Ok(())
}
```

- [ ] **Step 4: Run the test to confirm it passes**

Run: `cargo test -p knx-core check_no_duplicate_group_address_rejects`
Expected: PASS

- [ ] **Step 5: Wire the check into `Command::apply`'s `CreateGroupAddress` arm**

In `crates/knx-core/src/command.rs`, change the `use crate::validation::{...}` line near the top to also import the new function:

```rust
use crate::validation::{
    check_no_duplicate_group_address, check_no_duplicate_individual_address, ValidationError,
};
```

Change the `CreateGroupAddress` arm of `Command::apply` (currently):

```rust
            Command::CreateGroupAddress { entry } => {
                let installation = project
                    .installations
                    .first_mut()
                    .ok_or(CommandError::InstallationNotFound)?;
                let id = entry.id;
                installation.group_addresses.push(entry.clone());
                Ok(Command::DeleteGroupAddress { id })
            }
```

to:

```rust
            Command::CreateGroupAddress { entry } => {
                let installation = project
                    .installations
                    .first_mut()
                    .ok_or(CommandError::InstallationNotFound)?;
                check_no_duplicate_group_address(installation, entry.id, entry.address)?;
                let id = entry.id;
                installation.group_addresses.push(entry.clone());
                Ok(Command::DeleteGroupAddress { id })
            }
```

- [ ] **Step 6: Add the failing command-level test for the duplicate rejection**

In `crates/knx-core/src/command.rs`'s `#[cfg(test)] mod tests`, add (after `create_then_delete_group_address_round_trips_through_undo`):

```rust
#[test]
fn create_group_address_rejects_a_duplicate_address_and_leaves_the_stack_untouched() {
    let mut project = test_project_with_one_device(None);
    project.installations[0].group_addresses.push(GroupAddressEntry {
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
        Err(CommandError::Validation(ValidationError::DuplicateGroupAddress { .. }))
    ));
    assert_eq!(project.installations[0].group_addresses.len(), 1);
    assert!(!stack.can_undo());
}
```

- [ ] **Step 7: Run it to confirm it passes**

Run: `cargo test -p knx-core create_group_address_rejects_a_duplicate_address`
Expected: PASS

- [ ] **Step 8: Add `CommandError::GroupAddressInUse` and the failing test for it**

In `crates/knx-core/src/command.rs`, add the variant to `CommandError` (after `GroupAddressNotFound`):

```rust
    /// A `DeleteGroupAddress` was refused because at least one
    /// communication object still links to it — deleting it now would
    /// leave a dangling `GroupLink` (`ValidationError::DanglingGroupLink`
    /// exists for the reverse direction: a link created against a group
    /// address that is already gone).
    GroupAddressInUse(GroupAddressId),
```

Add its `Display` arm (after the `GroupAddressNotFound` arm):

```rust
            CommandError::GroupAddressInUse(id) => {
                write!(f, "group address {id} is still linked from a communication object")
            }
```

Add the test (after `create_group_address_rejects_a_duplicate_address_and_leaves_the_stack_untouched`), extending the test module's `use crate::flags::ResolvedFlags;` line to `use crate::flags::{Direction, GroupLink, ResolvedFlags};`:

```rust
#[test]
fn delete_group_address_is_rejected_while_a_com_object_still_links_to_it() {
    let mut project = test_project_with_one_device(None);
    project.installations[0].group_addresses.push(GroupAddressEntry {
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
```

- [ ] **Step 9: Run it to confirm it fails**

Run: `cargo test -p knx-core delete_group_address_is_rejected_while_a_com_object_still_links_to_it`
Expected: FAIL — `GroupAddressInUse` variant not yet checked for in `apply`.

- [ ] **Step 10: Wire the in-use guard into `Command::apply`'s `DeleteGroupAddress` arm**

Change the `DeleteGroupAddress` arm (currently):

```rust
            Command::DeleteGroupAddress { id } => {
                let id = *id;
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
                Ok(Command::CreateGroupAddress { entry })
            }
```

to:

```rust
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
                Ok(Command::CreateGroupAddress { entry })
            }
```

- [ ] **Step 11: Run the full `knx-core` test suite**

Run: `cargo test -p knx-core`
Expected: PASS, all tests including the three new ones.

- [ ] **Step 12: Commit**

```bash
git add crates/knx-core/src/validation.rs crates/knx-core/src/command.rs
git commit -m "feat(knx-core): reject duplicate group addresses and in-use deletes"
```

---

### Task 2: `knx-desktop-lib` — `create_group_address`/`delete_group_address` Tauri commands

**Files:**
- Modify: `apps/knx-desktop/src-tauri/src/lib.rs`
- Modify: `apps/knx-desktop/src-tauri/tests/command_dispatch.rs`

**Interfaces:**
- Consumes: `AppState` (`lib.rs`, unchanged), `Command::CreateGroupAddress`/`Command::DeleteGroupAddress`, `ValidationError::DuplicateGroupAddress`, `CommandError::GroupAddressInUse` (Task 1), `apply(state: &AppState, cmd: Command) -> Result<ProjectTree, String>` (existing private helper in `lib.rs`).
- Produces: `pub fn create_group_address_impl(state: &AppState, name: String, address: String) -> Result<knx_projection::ProjectTree, String>`, `pub fn delete_group_address_impl(state: &AppState, id: u32) -> Result<knx_projection::ProjectTree, String>` — both consumed by Task 3/4's frontend via the Tauri commands `create_group_address`/`delete_group_address` (`invoke("create_group_address", { name, address })`, `invoke("delete_group_address", { id })`).

- [ ] **Step 1: Add the failing integration test fixture and first test**

In `apps/knx-desktop/src-tauri/tests/command_dispatch.rs`, extend the `use knx_core::{...}` block at the top to also bring in `CompletionStatus, GroupAddress, GroupAddressEntry, GroupAddressId, Installation, InstallationId, Topology`:

```rust
use knx_core::{
    ComObjectInstance, ComObjectInstanceId, CommissioningState, CompletionStatus, DeviceId,
    DeviceInstance, DptRef, GroupAddress, GroupAddressEntry, GroupAddressId, IndividualAddress,
    Installation, InstallationId, Language, Layer, Override, Project, Resolved, ResolvedFlags,
    SourceRef, Text, Topology,
};
```

Add a new fixture function (after `state_with_two_devices`):

```rust
fn state_with_one_installation() -> AppState {
    let mut project = Project::new(Language("en".into()));
    project.installations.push(Installation {
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
    let state = AppState::default();
    *state.project.lock().unwrap() = Some(project);
    state
}
```

Add the test:

```rust
#[test]
fn creating_a_group_address_then_deleting_it_round_trips_through_undo() {
    let state = state_with_one_installation();

    let tree =
        knx_desktop_lib::create_group_address_impl(&state, "Living room light".into(), "1/1/1".into())
            .unwrap();
    assert!(tree.can_undo);
    assert_eq!(tree.installations[0].group_addresses.len(), 1);
    let ga = &tree.installations[0].group_addresses[0];
    assert_eq!(ga.name, "Living room light");
    assert_eq!(ga.address, "1/1/1");
    let id = ga.id;

    let tree = knx_desktop_lib::delete_group_address_impl(&state, id).unwrap();
    assert!(tree.installations[0].group_addresses.is_empty());

    let tree = knx_desktop_lib::undo_impl(&state).unwrap(); // undoes the delete
    assert_eq!(tree.installations[0].group_addresses.len(), 1);
    let tree = knx_desktop_lib::undo_impl(&state).unwrap(); // undoes the create
    assert!(tree.installations[0].group_addresses.is_empty());
}
```

- [ ] **Step 2: Run it to confirm it fails to compile**

Run: `cargo test -p knx-desktop creating_a_group_address_then_deleting_it`
Expected: FAIL — `create_group_address_impl`/`delete_group_address_impl` do not exist yet.

- [ ] **Step 3: Implement `create_group_address_impl`/`delete_group_address_impl`**

In `apps/knx-desktop/src-tauri/src/lib.rs`, add after `set_com_object_dpt_impl` (before `undo_impl`):

```rust
/// Allocates a fresh `GroupAddressId` and creates a new group address in
/// `installations[0]` — the only installation any `Command` targets
/// (`Command::apply`'s own doc comment). `address` is parsed against the
/// project's own `GroupAddressStyle` (`ProjectInfo::group_address_style`),
/// the same style `GroupAddressNode`'s `address` string was formatted with.
/// `entry.source` is empty: this is the first UI-created domain object in
/// this codebase, with no ETS origin to preserve.
pub fn create_group_address_impl(
    state: &AppState,
    name: String,
    address: String,
) -> Result<knx_projection::ProjectTree, String> {
    let cmd = {
        let mut project = state.project.lock().expect("state mutex poisoned");
        let project = project.as_mut().ok_or("no project open")?;
        let style = project.info.group_address_style;
        let address = knx_core::GroupAddress::parse(&address, style).map_err(|e| e.to_string())?;
        let id = project.ids.next_group_address_id();
        knx_core::Command::CreateGroupAddress {
            entry: knx_core::GroupAddressEntry {
                id,
                source: knx_core::SourceRef {
                    path: String::new(),
                    ets_id: String::new(),
                },
                name,
                address,
                central: false,
                unfiltered: false,
                range: None,
            },
        }
    };
    apply(state, cmd)
}

pub fn delete_group_address_impl(
    state: &AppState,
    id: u32,
) -> Result<knx_projection::ProjectTree, String> {
    apply(
        state,
        knx_core::Command::DeleteGroupAddress {
            id: knx_core::GroupAddressId(id),
        },
    )
}
```

- [ ] **Step 4: Add the `#[tauri::command]` wrappers and register them**

Add after `set_com_object_dpt` (before `undo`):

```rust
#[tauri::command]
fn create_group_address(
    name: String,
    address: String,
    state: tauri::State<AppState>,
) -> Result<knx_projection::ProjectTree, String> {
    create_group_address_impl(&state, name, address)
}

#[tauri::command]
fn delete_group_address(id: u32, state: tauri::State<AppState>) -> Result<knx_projection::ProjectTree, String> {
    delete_group_address_impl(&state, id)
}
```

In `run()`'s `tauri::generate_handler!` list, add both after `set_com_object_dpt`:

```rust
        .invoke_handler(tauri::generate_handler![
            open_project,
            save_project,
            save_project_as,
            open_native_project,
            device_detail,
            set_individual_address,
            set_com_object_dpt,
            create_group_address,
            delete_group_address,
            undo,
            redo
        ])
```

- [ ] **Step 5: Run the new test to confirm it passes**

Run: `cargo test -p knx-desktop creating_a_group_address_then_deleting_it`
Expected: PASS

- [ ] **Step 6: Add the failing tests for the two rejection paths**

Add to `command_dispatch.rs`:

```rust
#[test]
fn creating_a_group_address_with_a_malformed_address_is_rejected() {
    let state = state_with_one_installation();
    let err = knx_desktop_lib::create_group_address_impl(&state, "GA".into(), "not-an-address".into())
        .unwrap_err();
    assert!(err.contains("malformed group address"), "{err}");
    assert!(!state.command_stack.lock().unwrap().can_undo());
}

#[test]
fn creating_a_duplicate_group_address_is_rejected() {
    let state = state_with_one_installation();
    knx_desktop_lib::create_group_address_impl(&state, "First".into(), "1/1/1".into()).unwrap();
    let err = knx_desktop_lib::create_group_address_impl(&state, "Second".into(), "1/1/1".into())
        .unwrap_err();
    assert!(err.contains("already used"), "{err}");
    let project = state.project.lock().unwrap();
    assert_eq!(project.as_ref().unwrap().installations[0].group_addresses.len(), 1);
}

#[test]
fn deleting_a_group_address_still_linked_from_a_com_object_is_rejected() {
    let state = state_with_one_installation();
    let tree =
        knx_desktop_lib::create_group_address_impl(&state, "GA".into(), "1/1/1".into()).unwrap();
    let ga_id = tree.installations[0].group_addresses[0].id;

    {
        let mut project = state.project.lock().unwrap();
        let project = project.as_mut().unwrap();
        project.devices.insert(DeviceInstance {
            id: DeviceId(1),
            source: SourceRef {
                path: "t".into(),
                ets_id: "t".into(),
            },
            name: "D".into(),
            description: None,
            address: None,
            product_ref: "P".into(),
            program_ref: "H".into(),
            commissioning: CommissioningState::default(),
            visibility_calculated: true,
            com_objects: vec![ComObjectInstanceId(1)],
            binary_data: vec![],
        });
        project.devices.insert_com_object(ComObjectInstance {
            id: ComObjectInstanceId(1),
            source: SourceRef {
                path: "t".into(),
                ets_id: "t".into(),
            },
            device: DeviceId(1),
            number: 0,
            text: Override::Absent,
            description: Override::Absent,
            dpt: Override::Absent,
            flags: ResolvedFlags::none(),
            size: None,
            is_active: true,
            links: vec![knx_core::GroupLink {
                ga: GroupAddressId(ga_id),
                direction: knx_core::Direction::Send,
            }],
        });
    }

    let err = knx_desktop_lib::delete_group_address_impl(&state, ga_id).unwrap_err();
    assert!(err.contains("still linked"), "{err}");
}
```

- [ ] **Step 7: Run them to confirm they fail or pass as expected**

Run: `cargo test -p knx-desktop creating_a_group_address_with_a_malformed_address_is_rejected creating_a_duplicate_group_address_is_rejected deleting_a_group_address_still_linked_from_a_com_object_is_rejected`
Expected: these exercise Task 1's already-implemented guards through the new `_impl` functions from Step 3 — PASS with no further code change. If any fails, re-check Step 3/4's wiring before touching Task 1's code again.

- [ ] **Step 8: Run the full `knx-desktop` test suite**

Run: `cargo test -p knx-desktop`
Expected: PASS, all tests including the four new ones.

- [ ] **Step 9: Commit**

```bash
git add apps/knx-desktop/src-tauri/src/lib.rs apps/knx-desktop/src-tauri/tests/command_dispatch.rs
git commit -m "feat(knx-desktop): add create_group_address/delete_group_address Tauri commands"
```

---

### Task 3: Frontend — "Group Addresses" tree branch with inline create

**Files:**
- Modify: `apps/knx-desktop/src/ProjectExplorer.tsx`
- Modify: `apps/knx-desktop/src/App.tsx`
- Modify: `apps/knx-desktop/src/styles.css`

**Interfaces:**
- Consumes: `GroupAddressNode` (`bindings/GroupAddressNode.ts`, unchanged), `ProjectTree` (`bindings/ProjectTree.ts`, unchanged), Tauri command `create_group_address` (Task 2), existing `Selection`/`TreeNode`/`SelectionProps` from `ProjectExplorer.tsx`.
- Produces: `ProjectExplorer` gains a required prop `onTreeUpdate: (tree: ProjectTree) => void`, consumed by `App.tsx` (wired to the existing `handleTreeUpdate`) and, transitively, needed again by no other task.

There is no isolated unit to TDD here — no component-test tooling exists in this project (Global Constraints). Steps below are implement-then-verify-by-build, matching how `Dashboard.tsx`/`ThemeToggle.tsx` were added in earlier cycles (their own logic modules got vitest coverage; the JSX shell did not).

- [ ] **Step 1: Add the `GroupAddressItem` and `NewGroupAddressRow` components**

In `apps/knx-desktop/src/ProjectExplorer.tsx`, replace the existing import block (the file's first 8 lines) with:

```tsx
import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { ProjectTree } from "./bindings/ProjectTree";
import type { InstallationNode } from "./bindings/InstallationNode";
import type { AreaNode } from "./bindings/AreaNode";
import type { LineNode } from "./bindings/LineNode";
import type { BuildingNode } from "./bindings/BuildingNode";
import type { DeviceNode } from "./bindings/DeviceNode";
import type { GroupAddressNode } from "./bindings/GroupAddressNode";
import type { Selection } from "./selection";
```

Add, after `DeviceItem` and before `LineItem`:

```tsx
function GroupAddressItem(props: { ga: GroupAddressNode } & SelectionProps) {
  const { ga, selection, onSelect } = props;
  return (
    <TreeNode
      label={`${ga.address} ${ga.name}`}
      selected={selection?.kind === "group_address" && selection.id === ga.id}
      onSelect={() => onSelect({ kind: "group_address", id: ga.id })}
    />
  );
}

// The only affordance in the tree that creates a domain object rather than
// selecting one — kept as an inline row rather than a dialog, the same way
// `AddressField`/`DptField` (Inspector.tsx) edit inline rather than popping
// a modal. Only rendered under the first installation (`InstallationItem`'s
// `isFirst`): `Command::apply` only ever targets `installations[0]`
// (command.rs), so this is the only installation the affordance could
// honestly promise to create into.
function NewGroupAddressRow(props: { onCreated: (tree: ProjectTree) => void }) {
  const { onCreated } = props;
  const [address, setAddress] = useState("");
  const [name, setName] = useState("");
  const [error, setError] = useState<string | null>(null);
  const canCreate = address.trim() !== "" && name.trim() !== "";

  async function create() {
    if (!canCreate) return;
    setError(null);
    try {
      const tree = await invoke<ProjectTree>("create_group_address", { name, address });
      onCreated(tree);
      setAddress("");
      setName("");
    } catch (e) {
      setError(String(e));
    }
  }

  return (
    <li className="tree-new-row">
      <input
        value={address}
        placeholder="1/1/1"
        onChange={(e) => setAddress(e.target.value)}
        onKeyDown={(e) => {
          if (e.key === "Enter") void create();
        }}
      />
      <input
        value={name}
        placeholder="New group address"
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

- [ ] **Step 2: Wire the branch into `InstallationItem` and thread `onTreeUpdate`/`isFirst` through**

Change `InstallationItem`'s signature and body (currently takes `{ installation: InstallationNode } & SelectionProps`) to:

```tsx
function InstallationItem(
  props: { installation: InstallationNode; isFirst: boolean; onTreeUpdate: (tree: ProjectTree) => void } & SelectionProps,
) {
  const { installation, isFirst, onTreeUpdate, selection, onSelect } = props;
  return (
    <TreeNode label={installation.name}>
      <TreeNode label="Topology">
        {installation.topology.map((a) => (
          <AreaItem key={a.id} area={a} selection={selection} onSelect={onSelect} />
        ))}
      </TreeNode>
      <TreeNode label="Buildings">
        {installation.buildings.map((b) => (
          <BuildingItem key={b.id} building={b} selection={selection} onSelect={onSelect} />
        ))}
      </TreeNode>
      {installation.unassigned.length > 0 && (
        <TreeNode label="Unassigned">
          {installation.unassigned.map((d) => (
            <DeviceItem key={d.id} device={d} selection={selection} onSelect={onSelect} />
          ))}
        </TreeNode>
      )}
      <TreeNode label="Group Addresses">
        {installation.group_addresses.map((ga) => (
          <GroupAddressItem key={ga.id} ga={ga} selection={selection} onSelect={onSelect} />
        ))}
        {isFirst && <NewGroupAddressRow onCreated={onTreeUpdate} />}
      </TreeNode>
    </TreeNode>
  );
}
```

- [ ] **Step 3: Pass `isFirst`/`onTreeUpdate` from `ProjectExplorer` and add the prop**

Change only the function signature line and the `<ul className="tree-root">` block — everything from the `{(tree.errors > 0 || ...` footer onward is untouched. Currently:

```tsx
export default function ProjectExplorer(props: { tree: ProjectTree } & SelectionProps) {
  const { tree, selection, onSelect } = props;
  return (
    <div className="project-explorer">
      <ul className="tree-root">
        {tree.installations.map((inst) => (
          <InstallationItem
            key={inst.id}
            installation={inst}
            selection={selection}
            onSelect={onSelect}
          />
        ))}
      </ul>
```

becomes:

```tsx
export default function ProjectExplorer(
  props: { tree: ProjectTree; onTreeUpdate: (tree: ProjectTree) => void } & SelectionProps,
) {
  const { tree, onTreeUpdate, selection, onSelect } = props;
  return (
    <div className="project-explorer">
      <ul className="tree-root">
        {tree.installations.map((inst, idx) => (
          <InstallationItem
            key={inst.id}
            installation={inst}
            isFirst={idx === 0}
            onTreeUpdate={onTreeUpdate}
            selection={selection}
            onSelect={onSelect}
          />
        ))}
      </ul>
```

The rest of the function (the `footer` block and closing tags) is unchanged.

- [ ] **Step 4: Pass `onTreeUpdate` from `App.tsx`**

In `apps/knx-desktop/src/App.tsx`, change the `<ProjectExplorer>` usage (inside the `{tree && ( <div className="workspace"> ... )}` block):

```tsx
<ProjectExplorer
  tree={tree}
  selection={selection}
  onSelect={selectEntity}
  onTreeUpdate={handleTreeUpdate}
/>
```

- [ ] **Step 5: Add the CSS for the inline create row**

In `apps/knx-desktop/src/styles.css`, add after the `.field-error` rule:

```css
.tree-new-row {
  display: flex;
  align-items: center;
  gap: 0.35rem;
  padding: 0.25rem 0;
}

.tree-new-row input {
  min-width: 0;
}
```

- [ ] **Step 6: Build and test the frontend**

Run: `cd apps/knx-desktop && npm run test && npm run build`
Expected: `vitest` PASS (no test touches these files, but this confirms nothing else broke); `tsc` reports no type errors — this is what catches a prop mismatch between `ProjectExplorer`/`InstallationItem`/`App.tsx`.

- [ ] **Step 7: Commit**

```bash
git add apps/knx-desktop/src/ProjectExplorer.tsx apps/knx-desktop/src/App.tsx apps/knx-desktop/src/styles.css
git commit -m "feat(knx-desktop): add Group Addresses tree branch with inline create"
```

---

### Task 4: Frontend — delete button on `GroupAddressInspector`

**Files:**
- Modify: `apps/knx-desktop/src/Inspector.tsx`
- Modify: `apps/knx-desktop/src/App.tsx`

**Interfaces:**
- Consumes: Tauri command `delete_group_address` (Task 2), `App.tsx`'s existing `resetTree` function (unchanged signature: `(newTree: ProjectTree) => void`).
- Produces: `Inspector`'s exported default function gains a required prop `onDeleted: (tree: ProjectTree) => void`; no other task consumes this.

- [ ] **Step 1: Add the delete button to `GroupAddressInspector`**

In `apps/knx-desktop/src/Inspector.tsx`, change `GroupAddressInspector` (currently a plain read-only display) to:

```tsx
function GroupAddressInspector(props: {
  ga: GroupAddressNode;
  onDeleted: (tree: ProjectTree) => void;
}) {
  const { ga, onDeleted } = props;
  const [error, setError] = useState<string | null>(null);

  async function remove() {
    setError(null);
    try {
      const tree = await invoke<ProjectTree>("delete_group_address", { id: ga.id });
      onDeleted(tree);
    } catch (e) {
      setError(String(e));
    }
  }

  return (
    <div className="inspector">
      <h2>{ga.name}</h2>
      <p className="inspector-address">{ga.address}</p>
      <button onClick={remove}>Delete</button>
      {error && <span className="field-error">{error}</span>}
    </div>
  );
}
```

- [ ] **Step 2: Thread `onDeleted` through the exported `Inspector` component**

Change the default-exported `Inspector` function's props type and body:

```tsx
export default function Inspector(props: {
  selection: Selection;
  tree: ProjectTree;
  deviceDetail: DeviceDetail | null;
  onApplied: (tree: ProjectTree) => void;
  onDeleted: (tree: ProjectTree) => void;
}) {
  const { selection, tree, deviceDetail, onApplied, onDeleted } = props;

  if (selection.kind === "device") {
    if (!deviceDetail) return null;
    return <DeviceInspector detail={deviceDetail} onApplied={onApplied} />;
  }

  if (selection.kind === "group_address") {
    const ga = findGroupAddress(tree, selection.id);
    if (!ga) return null;
    return <GroupAddressInspector ga={ga} onDeleted={onDeleted} />;
  }

  const found = findBuildingPart(tree, selection.id);
  if (!found) return null;
  return <BuildingPartInspector node={found.node} path={found.path} />;
}
```

- [ ] **Step 3: Pass `resetTree` as `onDeleted` from `App.tsx`**

In `apps/knx-desktop/src/App.tsx`, change the `<Inspector>` usage:

```tsx
<Inspector
  key={`${selection.kind}-${selection.id}`}
  selection={selection}
  tree={tree}
  deviceDetail={deviceDetail}
  onApplied={handleTreeUpdate}
  onDeleted={resetTree}
/>
```

`resetTree` already exists in `App.tsx` and already does exactly what a deletion needs — set the new tree, clear the selection, clear `deviceDetail` — so a deleted group address cannot linger as a stale selection pointing at nothing.

- [ ] **Step 4: Build and test the frontend**

Run: `cd apps/knx-desktop && npm run test && npm run build`
Expected: PASS, no type errors.

- [ ] **Step 5: Commit**

```bash
git add apps/knx-desktop/src/Inspector.tsx apps/knx-desktop/src/App.tsx
git commit -m "feat(knx-desktop): add delete button to the group address inspector"
```

---

### Task 5: Documentation and final verification

**Files:**
- Modify: `docs/IMPLEMENTATION_STATUS.md`

**Interfaces:** None — this task only updates prose. It runs last because it reports the actual test counts from Tasks 1–4.

- [ ] **Step 1: Run the full verification suite and record the counts**

Run, in order, from the worktree root:
- `cargo test --workspace`
- `cd apps/knx-desktop && npm run test`
- `cd apps/knx-desktop && npm run build`

Expected: all PASS/succeed. Note the total `cargo test -p knx-core` and `cargo test -p knx-desktop` pass counts printed in their summary lines (e.g. `test result: ok. 52 passed`) — Step 2 needs the new totals.

- [ ] **Step 2: Update `docs/IMPLEMENTATION_STATUS.md`**

Change the `Last updated` line at the top to:

```markdown
Last updated: 2026-09-05 (Session 5, cycle 9)
```

In the Session 5 table row, append to the existing cycle list (after "...cycle 7 (System/Light/Dark theme toggle, ...) done"):

```markdown
, and cycle 9 (group address create/delete: a "Group Addresses" tree branch with inline create, a Delete button on the group-address inspector, duplicate-address and still-linked-on-delete validation in `knx-core`)
```

Find the known-gaps bullet that currently reads (in the "Next session" section near the bottom):

```markdown
- Cycle 3 wires `knx-store`'s entity persistence to a Tauri
  `save_project`/`save_project_as`/`open_native_project` command with a
  save dialog and UX, but always as a full round trip. Cycle 4 adds a
  command layer to `knx-desktop` (`device_detail`/`set_individual_address`/
  `set_com_object_dpt`/`undo`/`redo`), but it only reaches two of the four
  `Command` variants that have an incremental `sync_after_command` path
  (`SetIndividualAddress`, `SetComObjectDpt`/`RestoreComObjectDpt`) — the
  other two (`CreateGroupAddress`/`DeleteGroupAddress`) have no UI yet.
  Every other entity/attribute is still written only by a full
  `save_project`, until both a command and UI exist for it. Undo history
  is session-only by design (`AppState.command_stack`, reset on
  open/import, never persisted to `.knxdb`).
```

Replace it with:

```markdown
- Cycle 3 wires `knx-store`'s entity persistence to a Tauri
  `save_project`/`save_project_as`/`open_native_project` command with a
  save dialog and UX, but always as a full round trip. Cycle 4 adds a
  command layer to `knx-desktop` (`device_detail`/`set_individual_address`/
  `set_com_object_dpt`/`undo`/`redo`); cycle 9 reaches the remaining two
  `Command` variants that already had an incremental `sync_after_command`
  path (`CreateGroupAddress`/`DeleteGroupAddress`) with a UI
  (`create_group_address`/`delete_group_address`). Every other
  entity/attribute is still written only by a full `save_project`, until
  both a command and UI exist for it. Undo history is session-only by
  design (`AppState.command_stack`, reset on open/import, never persisted
  to `.knxdb`).
```

Add a new bullet after it, matching the deferred-manual-smoke-check pattern of cycles 4–7:

```markdown
- Cycle 9's plan-mandated manual smoke check (open a project, expand
  "Group Addresses", create one via the inline row, select it, delete it,
  confirm Undo/Redo restores it via both the toolbar and `Ctrl+Z`/
  `Ctrl+Shift+Z`) was not performed — no display available in this
  environment for a Tauri GUI session. Rust-level tests
  (`command.rs`, `command_dispatch.rs`) cover the validation and
  persistence logic underneath it.
```

- [ ] **Step 3: Commit the documentation update**

```bash
git add docs/IMPLEMENTATION_STATUS.md
git commit -m "docs: record Session 5 cycle 9 (group address create/delete) as shipped"
```
