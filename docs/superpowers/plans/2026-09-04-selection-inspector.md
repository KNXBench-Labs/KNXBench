# Selection + Properties Inspector Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Let a user click a device in the tree, see its properties and communication objects in a new Inspector panel, edit its individual address and communication-object datapoint types with inline validation, and undo/redo those edits.

**Architecture:** A lazy `device_detail` projection (knx-projection) feeds a new Inspector React component. Four new narrow Tauri commands (`device_detail`, `set_individual_address`, `set_com_object_dpt`, `undo`, `redo`) wire the existing, already-tested `knx_core::Command`/`CommandStack` into the desktop app for the first time — using plain `Option<String>` parameters parsed server-side by the existing `IndividualAddress`/`DptRef` parsers, so no new type crosses the Tauri IPC boundary and `knx-core` gains no new dependency.

**Tech Stack:** Rust workspace (knx-core, knx-projection, knx-desktop's `src-tauri`), Tauri 2, React 19 + TypeScript, `ts-rs` for generated bindings.

**Spec:** `docs/superpowers/specs/2026-09-04-selection-inspector-design.md`

## Global Constraints

- Data integrity: never fire a mutating command for a field the user didn't actually change (a no-op edit must not promote a `Program`-layer value to `UserEdit`).
- `knx-core` gains no new dependency; no new type crosses the Tauri IPC boundary beyond primitives (`u32`, `Option<String>`, `bool`) and the existing `ts-rs`-bound projection types.
- `CommandStack`/import-loss counts reset on `open_project`/`open_native_project`, never persisted to `.knxdb`.
- No frontend test infrastructure exists; frontend tasks verify with `npm run build` (runs `tsc` then `vite build`) plus the description of what to click to confirm by hand.
- Follow the existing `<command>_impl` split (a plain function the Tauri `#[tauri::command]` wrapper calls) so every new command is testable without Tauri machinery, exactly like `open_project_impl`/`save_project_as_impl`.

---

## Task 1: `knx-projection` — device detail projection

**Files:**
- Modify: `crates/knx-projection/src/lib.rs`

**Interfaces:**
- Produces: `pub struct DeviceDetail { id: u32, name: String, description: Option<String>, address: Option<String>, com_objects: Vec<ComObjectNode> }`, `pub struct ComObjectNode { id: u32, number: u16, name: Option<String>, dpt: Option<String>, dpt_layer: Option<String>, is_active: bool, read: bool, write: bool, transmit: bool, update: bool, communication: bool }`, `pub fn build_device_detail(project: &Project, id: DeviceId) -> Option<DeviceDetail>`. `ProjectTree` gains `pub can_undo: bool, pub can_redo: bool` (always `false` out of `build_project_tree`; Task 3 overlays the real values).
- Consumes: `knx_core::{Project, DeviceId, ComObjectInstance, Override, Resolved}` (all already re-exported from the `knx_core` crate root).

- [ ] **Step 1: Write the failing tests**

Add to the bottom of `crates/knx-projection/src/lib.rs`, inside a new `#[cfg(test)] mod tests` block (create one if none exists yet — check the file first; if a `mod tests` already exists, add these functions inside it):

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use knx_core::{
        CommissioningState, ComObjectInstance, ComObjectInstanceId, DeviceInstance, DptRef,
        IndividualAddress, Language, Layer, ResolvedFlags, SourceRef, Text,
    };

    fn source() -> SourceRef {
        SourceRef {
            path: "t".into(),
            ets_id: "t".into(),
        }
    }

    fn project_with_one_device() -> Project {
        let mut project = Project::new(Language("en".into()));
        project.devices.insert(DeviceInstance {
            id: DeviceId(1),
            source: source(),
            name: "Switch".into(),
            description: Some("Hallway switch".into()),
            address: Some(IndividualAddress::new(1, 1, 1).unwrap()),
            product_ref: "P".into(),
            program_ref: "H".into(),
            commissioning: CommissioningState::default(),
            visibility_calculated: true,
            com_objects: vec![ComObjectInstanceId(1)],
            binary_data: vec![],
        });
        project.devices.insert_com_object(ComObjectInstance {
            id: ComObjectInstanceId(1),
            source: source(),
            device: DeviceId(1),
            number: 0,
            text: Override::Value(Resolved {
                value: Text::Literal("Switch on/off".into()),
                layer: Layer::Program,
            }),
            description: Override::Absent,
            dpt: Override::Value(Resolved {
                value: DptRef {
                    main: 1,
                    sub: Some(1),
                },
                layer: Layer::UserEdit,
            }),
            flags: ResolvedFlags::none(),
            size: None,
            is_active: true,
            links: vec![],
        });
        project
    }

    #[test]
    fn build_device_detail_resolves_name_dpt_and_layer_for_each_com_object() {
        let project = project_with_one_device();
        let detail = build_device_detail(&project, DeviceId(1)).unwrap();

        assert_eq!(detail.id, 1);
        assert_eq!(detail.name, "Switch");
        assert_eq!(detail.description.as_deref(), Some("Hallway switch"));
        assert_eq!(detail.address.as_deref(), Some("1.1.1"));
        assert_eq!(detail.com_objects.len(), 1);

        let com = &detail.com_objects[0];
        assert_eq!(com.number, 0);
        assert_eq!(com.name.as_deref(), Some("Switch on/off"));
        assert_eq!(com.dpt.as_deref(), Some("DPST-1-1"));
        assert_eq!(com.dpt_layer.as_deref(), Some("UserEdit"));
        assert!(com.is_active);
        assert!(!com.read); // ResolvedFlags::none() sets nothing
    }

    #[test]
    fn build_device_detail_returns_none_for_an_unknown_device() {
        let project = project_with_one_device();
        assert!(build_device_detail(&project, DeviceId(99)).is_none());
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p knx-projection build_device_detail`
Expected: FAIL with "cannot find function `build_device_detail`" (and `DeviceDetail`/`ComObjectNode` unresolved)

- [ ] **Step 3: Add `DeviceDetail`, `ComObjectNode`, `build_device_detail`, and the `ProjectTree` fields**

In `crates/knx-projection/src/lib.rs`, add `pub can_undo: bool` and `pub can_redo: bool` to `ProjectTree` (after `pub warnings: usize,`) with this doc comment:

```rust
    /// Always `false` straight out of [`build_project_tree`] — this crate
    /// never sees a `CommandStack`. The desktop shell overlays the real
    /// value from its own `CommandStack` after every command/undo/redo.
    pub can_undo: bool,
    /// See `can_undo`.
    pub can_redo: bool,
```

In `build_project_tree`, add `can_undo: false, can_redo: false,` to the `ProjectTree { ... }` literal.

Then add, after `build_device_node` and its helpers:

```rust
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct DeviceDetail {
    pub id: u32,
    pub name: String,
    pub description: Option<String>,
    /// Formatted individual address (e.g. `"1.1.1"`), `None` if unassigned.
    pub address: Option<String>,
    pub com_objects: Vec<ComObjectNode>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct ComObjectNode {
    pub id: u32,
    /// From `_O-<n>` in the source `RefId`.
    pub number: u16,
    pub name: Option<String>,
    /// Formatted datapoint type reference (e.g. `"DPST-1-1"`, `"DPT-1"`),
    /// `None` if never stated at any layer.
    pub dpt: Option<String>,
    /// The layer `dpt` resolved from (`"Program"`, `"ProgramRef"`,
    /// `"Instance"`, `"Inferred"`, `"UserEdit"`), `None` alongside `dpt:
    /// None`.
    pub dpt_layer: Option<String>,
    pub is_active: bool,
    /// Display-only this cycle — no `Command` exists yet to edit flags.
    pub read: bool,
    pub write: bool,
    pub transmit: bool,
    pub update: bool,
    pub communication: bool,
}

/// Builds the detail panel for one device, resolving each communication
/// object's text through the project's string table. `None` if `id` does
/// not name a device in `project` (a stale selection after an edit, for
/// instance).
pub fn build_device_detail(project: &Project, id: knx_core::DeviceId) -> Option<DeviceDetail> {
    let device = project.devices.get(id)?;
    Some(DeviceDetail {
        id: device.id.0,
        name: device.name.clone(),
        description: device.description.clone(),
        address: device.address.map(|a| a.to_string()),
        com_objects: device
            .com_objects
            .iter()
            .filter_map(|com_id| project.devices.com_object(*com_id))
            .map(|com| build_com_object_node(com, project))
            .collect(),
    })
}

fn build_com_object_node(
    com: &knx_core::ComObjectInstance,
    project: &Project,
) -> ComObjectNode {
    let name = com.text.value().and_then(|resolved| {
        project
            .strings
            .text(&resolved.value, project.strings.default_language())
            .map(|s| s.to_string())
    });
    let dpt = com.dpt.value().map(|resolved| resolved.value.to_string());
    let dpt_layer = com.dpt.layer().map(|layer| format!("{layer:?}"));
    let flag = |o: &knx_core::Override<bool>| o.value().map(|r| r.value).unwrap_or(false);
    ComObjectNode {
        id: com.id.0,
        number: com.number,
        name,
        dpt,
        dpt_layer,
        is_active: com.is_active,
        read: flag(&com.flags.read),
        write: flag(&com.flags.write),
        transmit: flag(&com.flags.transmit),
        update: flag(&com.flags.update),
        communication: flag(&com.flags.communication),
    }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p knx-projection build_device_detail`
Expected: PASS (2 tests)

- [ ] **Step 5: Regenerate the ts-rs bindings**

Run: `TS_RS_EXPORT_DIR=../../apps/knx-desktop/src/bindings cargo test -p knx-projection`

This writes `apps/knx-desktop/src/bindings/DeviceDetail.ts` and `ComObjectNode.ts`, and rewrites `ProjectTree.ts` to add `can_undo`/`can_redo`. Confirm both new files exist:

Run: `ls apps/knx-desktop/src/bindings/DeviceDetail.ts apps/knx-desktop/src/bindings/ComObjectNode.ts`
Expected: both paths printed, no "No such file" error

- [ ] **Step 6: Run the full workspace test suite**

Run: `cargo test --workspace`
Expected: PASS, no regressions

- [ ] **Step 7: Commit**

```bash
git add crates/knx-projection/src/lib.rs apps/knx-desktop/src/bindings/DeviceDetail.ts apps/knx-desktop/src/bindings/ComObjectNode.ts apps/knx-desktop/src/bindings/ProjectTree.ts
git commit -m "feat(projection): DeviceDetail/ComObjectNode, ProjectTree.can_undo/can_redo"
```

---

## Task 2: `knx-desktop` backend — `device_detail` command

**Files:**
- Modify: `apps/knx-desktop/src-tauri/src/lib.rs`
- Test: `apps/knx-desktop/src-tauri/tests/device_detail.rs` (new)

**Interfaces:**
- Consumes: `knx_projection::{DeviceDetail, build_device_detail}` (Task 1), `AppState` (existing).
- Produces: `pub fn device_detail_impl(project: &knx_core::Project, device_id: u32) -> Result<knx_projection::DeviceDetail, String>`, Tauri command `device_detail(device_id: u32, state) -> Result<DeviceDetail, String>`. Both used by Task 5's frontend fetch and Task 3's tests.

- [ ] **Step 1: Write the failing test**

Create `apps/knx-desktop/src-tauri/tests/device_detail.rs`:

```rust
//! `device_detail_impl` without any Tauri machinery — see
//! `open_reference_project.rs` for the same pattern against a real import.

use knx_core::{
    CommissioningState, ComObjectInstance, ComObjectInstanceId, DeviceInstance, DeviceId, DptRef,
    Language, Layer, Override, Project, Resolved, ResolvedFlags, SourceRef, Text,
};

fn source() -> SourceRef {
    SourceRef {
        path: "t".into(),
        ets_id: "t".into(),
    }
}

fn project_with_one_device() -> Project {
    let mut project = Project::new(Language("en".into()));
    project.devices.insert(DeviceInstance {
        id: DeviceId(1),
        source: source(),
        name: "Switch".into(),
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
        source: source(),
        device: DeviceId(1),
        number: 0,
        text: Override::Value(Resolved {
            value: Text::Literal("Obj".into()),
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
    });
    project
}

#[test]
fn device_detail_impl_returns_the_projection_for_a_known_device() {
    let project = project_with_one_device();
    let detail = knx_desktop_lib::device_detail_impl(&project, 1).unwrap();
    assert_eq!(detail.name, "Switch");
    assert_eq!(detail.com_objects.len(), 1);
}

#[test]
fn device_detail_impl_reports_an_unknown_device_by_id() {
    let project = project_with_one_device();
    let err = knx_desktop_lib::device_detail_impl(&project, 42).unwrap_err();
    assert!(err.contains("42"));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p knx-desktop --test device_detail`
Expected: FAIL with "cannot find function `device_detail_impl` in crate `knx_desktop_lib`"

- [ ] **Step 3: Implement `device_detail_impl` and the Tauri command**

In `apps/knx-desktop/src-tauri/src/lib.rs`, add after `open_native_project_impl`:

```rust
/// Projects one device's detail. `Err` names the device id when it no
/// longer exists in `project` — a stale selection after an edit, for
/// instance.
pub fn device_detail_impl(
    project: &knx_core::Project,
    device_id: u32,
) -> Result<knx_projection::DeviceDetail, String> {
    knx_projection::build_device_detail(project, knx_core::DeviceId(device_id))
        .ok_or_else(|| format!("device {device_id} not found"))
}

#[tauri::command]
fn device_detail(
    device_id: u32,
    state: tauri::State<AppState>,
) -> Result<knx_projection::DeviceDetail, String> {
    let project = state.project.lock().expect("state mutex poisoned");
    let project = project.as_ref().ok_or("no project open")?;
    device_detail_impl(project, device_id)
}
```

Add `device_detail` to the `generate_handler!` list in `run()`:

```rust
        .invoke_handler(tauri::generate_handler![
            open_project,
            save_project,
            save_project_as,
            open_native_project,
            device_detail
        ])
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p knx-desktop --test device_detail`
Expected: PASS (2 tests)

- [ ] **Step 5: Run the full workspace build and test suite**

Run: `cargo build --workspace && cargo test --workspace`
Expected: PASS, no regressions

- [ ] **Step 6: Commit**

```bash
git add apps/knx-desktop/src-tauri/src/lib.rs apps/knx-desktop/src-tauri/tests/device_detail.rs
git commit -m "feat(knx-desktop): device_detail Tauri command"
```

---

## Task 3: `knx-desktop` backend — apply, undo, redo

**Files:**
- Modify: `apps/knx-desktop/src-tauri/src/lib.rs`
- Test: `apps/knx-desktop/src-tauri/tests/command_dispatch.rs` (new)

**Interfaces:**
- Consumes: `knx_core::{Command, CommandStack, DeviceId, IndividualAddress, DptRef}` (all crate-root re-exports, `knx-desktop` already depends on `knx-core`), `AppState` (extended here).
- Produces: `AppState.command_stack: Mutex<CommandStack>`, `AppState.import_counts: Mutex<(usize, usize)>` (both `pub`); `pub fn set_individual_address_impl(state: &AppState, device_id: u32, address: Option<String>) -> Result<ProjectTree, String>`, `pub fn set_com_object_dpt_impl(state: &AppState, com_object_id: u32, dpt: Option<String>) -> Result<ProjectTree, String>`, `pub fn undo_impl(state: &AppState) -> Result<ProjectTree, String>`, `pub fn redo_impl(state: &AppState) -> Result<ProjectTree, String>`; Tauri commands `set_individual_address`, `set_com_object_dpt`, `undo`, `redo`. Used by Task 5/6's frontend.

- [ ] **Step 1: Write the failing tests**

Create `apps/knx-desktop/src-tauri/tests/command_dispatch.rs`:

```rust
//! `set_individual_address_impl`/`set_com_object_dpt_impl`/`undo_impl`/
//! `redo_impl` without any Tauri machinery — see `open_reference_project.rs`
//! for the same no-Tauri pattern.

use knx_core::{
    CommissioningState, ComObjectInstance, ComObjectInstanceId, DeviceInstance, DeviceId, DptRef,
    IndividualAddress, Language, Layer, Override, Project, Resolved, ResolvedFlags, SourceRef, Text,
};
use knx_desktop_lib::AppState;

fn source() -> SourceRef {
    SourceRef {
        path: "t".into(),
        ets_id: "t".into(),
    }
}

fn state_with_two_devices() -> AppState {
    let mut project = Project::new(Language("en".into()));
    project.devices.insert(DeviceInstance {
        id: DeviceId(1),
        source: source(),
        name: "D1".into(),
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
        source: source(),
        device: DeviceId(1),
        number: 0,
        text: Override::Value(Resolved {
            value: Text::Literal("Obj".into()),
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
    });
    project.devices.insert(DeviceInstance {
        id: DeviceId(2),
        source: source(),
        name: "D2".into(),
        description: None,
        address: Some(IndividualAddress::new(1, 1, 1).unwrap()),
        product_ref: "P".into(),
        program_ref: "H".into(),
        commissioning: CommissioningState::default(),
        visibility_calculated: true,
        com_objects: vec![],
        binary_data: vec![],
    });

    let state = AppState::default();
    *state.project.lock().unwrap() = Some(project);
    state
}

#[test]
fn setting_individual_address_then_undo_then_redo_round_trips() {
    let state = state_with_two_devices();

    let tree = knx_desktop_lib::set_individual_address_impl(&state, 1, Some("1.1.2".into()))
        .unwrap();
    assert!(tree.can_undo);
    assert!(!tree.can_redo);
    let project = state.project.lock().unwrap();
    let detail = knx_desktop_lib::device_detail_impl(project.as_ref().unwrap(), 1).unwrap();
    assert_eq!(detail.address.as_deref(), Some("1.1.2"));
    drop(project);

    let tree = knx_desktop_lib::undo_impl(&state).unwrap();
    assert!(!tree.can_undo);
    assert!(tree.can_redo);
    let project = state.project.lock().unwrap();
    let detail = knx_desktop_lib::device_detail_impl(project.as_ref().unwrap(), 1).unwrap();
    assert_eq!(detail.address, None);
    drop(project);

    let tree = knx_desktop_lib::redo_impl(&state).unwrap();
    assert!(tree.can_undo);
    assert!(!tree.can_redo);
    let project = state.project.lock().unwrap();
    let detail = knx_desktop_lib::device_detail_impl(project.as_ref().unwrap(), 1).unwrap();
    assert_eq!(detail.address.as_deref(), Some("1.1.2"));
}

#[test]
fn a_duplicate_individual_address_is_rejected_and_leaves_the_stack_untouched() {
    let state = state_with_two_devices();
    let err =
        knx_desktop_lib::set_individual_address_impl(&state, 1, Some("1.1.1".into())).unwrap_err();
    assert!(err.contains("already used by device 2"), "{err}");

    let project = state.project.lock().unwrap();
    let detail = knx_desktop_lib::device_detail_impl(project.as_ref().unwrap(), 1).unwrap();
    assert_eq!(detail.address, None); // unchanged
    drop(project);

    assert!(!state.command_stack.lock().unwrap().can_undo());
}

#[test]
fn a_malformed_address_string_is_rejected_before_touching_the_project() {
    let state = state_with_two_devices();
    let err =
        knx_desktop_lib::set_individual_address_impl(&state, 1, Some("not-an-address".into()))
            .unwrap_err();
    assert!(err.contains("malformed individual address"), "{err}");
    assert!(!state.command_stack.lock().unwrap().can_undo());
}

#[test]
fn setting_com_object_dpt_marks_it_user_edit_and_undo_restores_the_program_layer() {
    let state = state_with_two_devices();
    knx_desktop_lib::set_com_object_dpt_impl(&state, 1, Some("DPST-5-1".into())).unwrap();

    let project = state.project.lock().unwrap();
    let detail = knx_desktop_lib::device_detail_impl(project.as_ref().unwrap(), 1).unwrap();
    let com = &detail.com_objects[0];
    assert_eq!(com.dpt.as_deref(), Some("DPST-5-1"));
    assert_eq!(com.dpt_layer.as_deref(), Some("UserEdit"));
    drop(project);

    knx_desktop_lib::undo_impl(&state).unwrap();
    let project = state.project.lock().unwrap();
    let detail = knx_desktop_lib::device_detail_impl(project.as_ref().unwrap(), 1).unwrap();
    let com = &detail.com_objects[0];
    assert_eq!(com.dpt.as_deref(), Some("DPST-1-1"));
    assert_eq!(com.dpt_layer.as_deref(), Some("Program"));
}

#[test]
fn undo_with_nothing_to_undo_is_an_error() {
    let state = state_with_two_devices();
    let err = knx_desktop_lib::undo_impl(&state).unwrap_err();
    assert!(err.contains("nothing to undo"), "{err}");
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p knx-desktop --test command_dispatch`
Expected: FAIL — `AppState` has no field `command_stack`, and the `*_impl` functions don't exist yet

- [ ] **Step 3: Extend `AppState` and add the dispatch helpers**

In `apps/knx-desktop/src-tauri/src/lib.rs`, replace the `AppState` struct and its `Default` impl:

```rust
pub struct AppState {
    pub project: Mutex<Option<knx_core::Project>>,
    /// The `.knxdb` file the in-memory project was last saved to or loaded
    /// from, if any. `None` until `save_project_as`/`open_native_project`
    /// sets it; plain `save_project` requires it already set.
    pub store_path: Mutex<Option<PathBuf>>,
    /// Every applied command's inverse, for undo/redo. Reset to empty on
    /// `open_project`/`open_native_project` — undo history never survives
    /// loading a different project, and is never persisted to `.knxdb`.
    pub command_stack: Mutex<knx_core::CommandStack>,
    /// (errors, warnings) from the initial import's `ImportReport`,
    /// reapplied to every tree rebuilt after a command/undo/redo — edits
    /// don't change what import lost. `(0, 0)` for a `.knxdb` native load.
    pub import_counts: Mutex<(usize, usize)>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            project: Mutex::new(None),
            store_path: Mutex::new(None),
            command_stack: Mutex::new(knx_core::CommandStack::new()),
            import_counts: Mutex::new((0, 0)),
        }
    }
}
```

In `open_project` (the `#[tauri::command]`), after `*state.project.lock()...= Some(project);`, add:

```rust
    *state.command_stack.lock().expect("state mutex poisoned") = knx_core::CommandStack::new();
    *state.import_counts.lock().expect("state mutex poisoned") = (tree.errors, tree.warnings);
```

In `open_native_project`, after `*state.project.lock()...= Some(project);` and the existing `store_path` line, add:

```rust
    *state.command_stack.lock().expect("state mutex poisoned") = knx_core::CommandStack::new();
    *state.import_counts.lock().expect("state mutex poisoned") = (0, 0);
```

Then, after `device_detail`, add:

```rust
/// Rebuilds `tree` from `project` and overlays the counts/undo-redo state
/// that `build_project_tree` alone cannot know about.
fn tree_with_state(
    project: &knx_core::Project,
    stack: &knx_core::CommandStack,
    import_counts: (usize, usize),
) -> knx_projection::ProjectTree {
    let mut tree = knx_projection::build_project_tree(project);
    tree.errors = import_counts.0;
    tree.warnings = import_counts.1;
    tree.can_undo = stack.can_undo();
    tree.can_redo = stack.can_redo();
    tree
}

fn apply(state: &AppState, cmd: knx_core::Command) -> Result<knx_projection::ProjectTree, String> {
    let mut project = state.project.lock().expect("state mutex poisoned");
    let project = project.as_mut().ok_or("no project open")?;
    let mut stack = state.command_stack.lock().expect("state mutex poisoned");
    stack.do_command(project, cmd).map_err(|e| e.to_string())?;
    let import_counts = *state.import_counts.lock().expect("state mutex poisoned");
    Ok(tree_with_state(project, &stack, import_counts))
}

pub fn set_individual_address_impl(
    state: &AppState,
    device_id: u32,
    address: Option<String>,
) -> Result<knx_projection::ProjectTree, String> {
    let address = match address {
        Some(s) => Some(
            s.parse::<knx_core::IndividualAddress>()
                .map_err(|e| e.to_string())?,
        ),
        None => None,
    };
    apply(
        state,
        knx_core::Command::SetIndividualAddress {
            device: knx_core::DeviceId(device_id),
            address,
        },
    )
}

pub fn set_com_object_dpt_impl(
    state: &AppState,
    com_object_id: u32,
    dpt: Option<String>,
) -> Result<knx_projection::ProjectTree, String> {
    let dpt = match dpt {
        Some(s) => Some(knx_core::DptRef::parse(&s).map_err(|e| e.to_string())?),
        None => None,
    };
    apply(
        state,
        knx_core::Command::SetComObjectDpt {
            com_object: knx_core::ComObjectInstanceId(com_object_id),
            dpt,
        },
    )
}

pub fn undo_impl(state: &AppState) -> Result<knx_projection::ProjectTree, String> {
    let mut project = state.project.lock().expect("state mutex poisoned");
    let project = project.as_mut().ok_or("no project open")?;
    let mut stack = state.command_stack.lock().expect("state mutex poisoned");
    stack.undo(project).map_err(|e| e.to_string())?;
    let import_counts = *state.import_counts.lock().expect("state mutex poisoned");
    Ok(tree_with_state(project, &stack, import_counts))
}

pub fn redo_impl(state: &AppState) -> Result<knx_projection::ProjectTree, String> {
    let mut project = state.project.lock().expect("state mutex poisoned");
    let project = project.as_mut().ok_or("no project open")?;
    let mut stack = state.command_stack.lock().expect("state mutex poisoned");
    stack.redo(project).map_err(|e| e.to_string())?;
    let import_counts = *state.import_counts.lock().expect("state mutex poisoned");
    Ok(tree_with_state(project, &stack, import_counts))
}

#[tauri::command]
fn set_individual_address(
    device_id: u32,
    address: Option<String>,
    state: tauri::State<AppState>,
) -> Result<knx_projection::ProjectTree, String> {
    set_individual_address_impl(&state, device_id, address)
}

#[tauri::command]
fn set_com_object_dpt(
    com_object_id: u32,
    dpt: Option<String>,
    state: tauri::State<AppState>,
) -> Result<knx_projection::ProjectTree, String> {
    set_com_object_dpt_impl(&state, com_object_id, dpt)
}

#[tauri::command]
fn undo(state: tauri::State<AppState>) -> Result<knx_projection::ProjectTree, String> {
    undo_impl(&state)
}

#[tauri::command]
fn redo(state: tauri::State<AppState>) -> Result<knx_projection::ProjectTree, String> {
    redo_impl(&state)
}
```

Add the four new commands to `generate_handler!`:

```rust
        .invoke_handler(tauri::generate_handler![
            open_project,
            save_project,
            save_project_as,
            open_native_project,
            device_detail,
            set_individual_address,
            set_com_object_dpt,
            undo,
            redo
        ])
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p knx-desktop --test command_dispatch`
Expected: PASS (5 tests)

- [ ] **Step 5: Run the full workspace build, tests, and lints**

Run: `cargo build --workspace && cargo test --workspace && cargo fmt --all --check && cargo clippy --workspace --all-targets -- -D warnings`
Expected: all PASS, no regressions

- [ ] **Step 6: Commit**

```bash
git add apps/knx-desktop/src-tauri/src/lib.rs apps/knx-desktop/src-tauri/tests/command_dispatch.rs
git commit -m "feat(knx-desktop): apply/undo/redo command dispatch"
```

---

## Task 4: Frontend — clickable, selectable devices in the tree

**Files:**
- Modify: `apps/knx-desktop/src/ProjectExplorer.tsx`
- Modify: `apps/knx-desktop/src/styles.css`

**Interfaces:**
- Consumes: nothing new.
- Produces: `ProjectExplorer` gains props `selectedId: number | null` and `onSelectDevice: (id: number) => void`, threaded down to every `DeviceItem`. Used by Task 5's `App.tsx`.

- [ ] **Step 1: Replace `ProjectExplorer.tsx`**

```tsx
import { useState } from "react";
import type { ProjectTree } from "./bindings/ProjectTree";
import type { InstallationNode } from "./bindings/InstallationNode";
import type { AreaNode } from "./bindings/AreaNode";
import type { LineNode } from "./bindings/LineNode";
import type { BuildingNode } from "./bindings/BuildingNode";
import type { DeviceNode } from "./bindings/DeviceNode";

function TreeNode(props: {
  label: string;
  children?: React.ReactNode;
  selected?: boolean;
  onClick?: () => void;
}) {
  const [open, setOpen] = useState(true);
  const hasChildren = props.children !== undefined;
  const classes = ["tree-label"];
  if (hasChildren) classes.push("expandable");
  if (props.selected) classes.push("selected");
  const handleClick = hasChildren ? () => setOpen(!open) : props.onClick;
  return (
    <li>
      <span className={classes.join(" ")} onClick={handleClick}>
        {hasChildren ? (open ? "▾ " : "▸ ") : ""}
        {props.label}
      </span>
      {hasChildren && open && <ul>{props.children}</ul>}
    </li>
  );
}

type SelectionProps = {
  selectedId: number | null;
  onSelectDevice: (id: number) => void;
};

function DeviceItem(props: { device: DeviceNode } & SelectionProps) {
  const { device, selectedId, onSelectDevice } = props;
  const label = device.address ? `${device.address} ${device.name}` : device.name;
  return (
    <TreeNode
      label={label}
      selected={device.id === selectedId}
      onClick={() => onSelectDevice(device.id)}
    />
  );
}

function LineItem(props: { line: LineNode } & SelectionProps) {
  const { line, selectedId, onSelectDevice } = props;
  return (
    <TreeNode label={`Line ${line.address}: ${line.name}`}>
      {line.devices.map((d) => (
        <DeviceItem key={d.id} device={d} selectedId={selectedId} onSelectDevice={onSelectDevice} />
      ))}
    </TreeNode>
  );
}

function AreaItem(props: { area: AreaNode } & SelectionProps) {
  const { area, selectedId, onSelectDevice } = props;
  return (
    <TreeNode label={`Area ${area.address}: ${area.name}`}>
      {area.lines.map((l) => (
        <LineItem key={l.id} line={l} selectedId={selectedId} onSelectDevice={onSelectDevice} />
      ))}
    </TreeNode>
  );
}

function BuildingItem(props: { building: BuildingNode } & SelectionProps) {
  const { building, selectedId, onSelectDevice } = props;
  return (
    <TreeNode label={`${building.name} (${building.kind})`}>
      {building.children.map((c) => (
        <BuildingItem key={c.id} building={c} selectedId={selectedId} onSelectDevice={onSelectDevice} />
      ))}
      {building.devices.map((d) => (
        <DeviceItem key={d.id} device={d} selectedId={selectedId} onSelectDevice={onSelectDevice} />
      ))}
    </TreeNode>
  );
}

function InstallationItem(props: { installation: InstallationNode } & SelectionProps) {
  const { installation, selectedId, onSelectDevice } = props;
  return (
    <TreeNode label={installation.name}>
      <TreeNode label="Topology">
        {installation.topology.map((a) => (
          <AreaItem key={a.id} area={a} selectedId={selectedId} onSelectDevice={onSelectDevice} />
        ))}
      </TreeNode>
      <TreeNode label="Buildings">
        {installation.buildings.map((b) => (
          <BuildingItem key={b.id} building={b} selectedId={selectedId} onSelectDevice={onSelectDevice} />
        ))}
      </TreeNode>
      {installation.unassigned.length > 0 && (
        <TreeNode label="Unassigned">
          {installation.unassigned.map((d) => (
            <DeviceItem key={d.id} device={d} selectedId={selectedId} onSelectDevice={onSelectDevice} />
          ))}
        </TreeNode>
      )}
    </TreeNode>
  );
}

export default function ProjectExplorer(props: { tree: ProjectTree } & SelectionProps) {
  const { tree, selectedId, onSelectDevice } = props;
  return (
    <div className="project-explorer">
      <ul className="tree-root">
        {tree.installations.map((inst) => (
          <InstallationItem
            key={inst.id}
            installation={inst}
            selectedId={selectedId}
            onSelectDevice={onSelectDevice}
          />
        ))}
      </ul>
      {(tree.errors > 0 || tree.warnings > 0) && (
        <footer>
          {tree.errors > 0 && (
            <div className="import-errors">
              {tree.errors} import error{tree.errors === 1 ? "" : "s"} — data may be missing or
              incorrect
            </div>
          )}
          {tree.warnings > 0 && (
            <div className="import-warnings">
              {tree.warnings} import warning{tree.warnings === 1 ? "" : "s"}
            </div>
          )}
        </footer>
      )}
    </div>
  );
}
```

- [ ] **Step 2: Add the `selected` style**

In `apps/knx-desktop/src/styles.css`, after the `.tree-label.expandable` rule:

```css
.tree-label.selected {
  background: color-mix(in srgb, currentColor 15%, transparent);
  border-radius: 3px;
}
```

- [ ] **Step 3: Type-check**

`ProjectExplorer` now requires two new props its only caller (`App.tsx`) doesn't pass yet — this will fail until Task 5 updates `App.tsx`. Confirm the failure is exactly that:

Run: `cd apps/knx-desktop && npm run build`
Expected: FAIL — `Property 'selectedId' is missing in type '{ tree: ProjectTree; }'` (or `onSelectDevice`) at the `<ProjectExplorer tree={tree} />` call in `App.tsx`

- [ ] **Step 4: Commit**

```bash
git add apps/knx-desktop/src/ProjectExplorer.tsx apps/knx-desktop/src/styles.css
git commit -m "feat(knx-desktop): clickable, selectable devices in the tree"
```

(The build stays red until Task 5 — that task's own build step is the real gate.)

---

## Task 5: Frontend — Inspector panel

**Files:**
- Create: `apps/knx-desktop/src/Inspector.tsx`
- Modify: `apps/knx-desktop/src/App.tsx`
- Modify: `apps/knx-desktop/src/styles.css`

**Interfaces:**
- Consumes: `ProjectExplorer` (Task 4), Tauri commands `device_detail`/`set_individual_address`/`set_com_object_dpt` (Tasks 2–3), bindings `DeviceDetail`/`ComObjectNode`/`ProjectTree` (Task 1).
- Produces: `Inspector` component, `App.tsx`'s `selectedDeviceId`/`deviceDetail` state and `handleTreeUpdate`, reused by Task 6.

- [ ] **Step 1: Create `Inspector.tsx`**

```tsx
import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { DeviceDetail } from "./bindings/DeviceDetail";
import type { ComObjectNode } from "./bindings/ComObjectNode";
import type { ProjectTree } from "./bindings/ProjectTree";

function AddressField(props: { detail: DeviceDetail; onApplied: (tree: ProjectTree) => void }) {
  const { detail, onApplied } = props;
  const [value, setValue] = useState(detail.address ?? "");
  const [error, setError] = useState<string | null>(null);

  async function apply() {
    const current = detail.address ?? "";
    if (value === current) return;
    setError(null);
    try {
      const tree = await invoke<ProjectTree>("set_individual_address", {
        deviceId: detail.id,
        address: value === "" ? null : value,
      });
      onApplied(tree);
    } catch (e) {
      setError(String(e));
      setValue(current);
    }
  }

  return (
    <label className="inspector-field">
      Address
      <input
        value={value}
        placeholder="1.1.1"
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

function DptField(props: { com: ComObjectNode; onApplied: (tree: ProjectTree) => void }) {
  const { com, onApplied } = props;
  const [value, setValue] = useState(com.dpt ?? "");
  const [error, setError] = useState<string | null>(null);

  async function apply() {
    const current = com.dpt ?? "";
    if (value === current) return;
    setError(null);
    try {
      const tree = await invoke<ProjectTree>("set_com_object_dpt", {
        comObjectId: com.id,
        dpt: value === "" ? null : value,
      });
      onApplied(tree);
    } catch (e) {
      setError(String(e));
      setValue(current);
    }
  }

  return (
    <label className="inspector-field">
      DPT
      <input
        value={value}
        placeholder="DPST-9-1"
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

export default function Inspector(props: {
  detail: DeviceDetail;
  onApplied: (tree: ProjectTree) => void;
}) {
  const { detail, onApplied } = props;
  return (
    <div className="inspector">
      <h2>{detail.name}</h2>
      {detail.description && <p className="inspector-description">{detail.description}</p>}
      <AddressField detail={detail} onApplied={onApplied} />
      <h3>Communication objects</h3>
      <ul className="com-object-list">
        {detail.com_objects.map((com) => (
          <li key={com.id}>
            <span className="com-object-label">
              {com.number}: {com.name ?? "(unnamed)"}
            </span>
            <DptField com={com} onApplied={onApplied} />
            {com.dpt_layer && <span className="provenance-badge">{com.dpt_layer}</span>}
          </li>
        ))}
      </ul>
    </div>
  );
}
```

- [ ] **Step 2: Wire selection and the Inspector into `App.tsx`**

Replace `apps/knx-desktop/src/App.tsx` entirely:

```tsx
import { useState } from "react";
import { open, save } from "@tauri-apps/plugin-dialog";
import { invoke } from "@tauri-apps/api/core";
import type { ProjectTree } from "./bindings/ProjectTree";
import type { DeviceDetail } from "./bindings/DeviceDetail";
import ProjectExplorer from "./ProjectExplorer";
import Inspector from "./Inspector";

const KNXDB_FILTER = [{ name: "knx-desktop project", extensions: ["knxdb"] }];

function App() {
  const [tree, setTree] = useState<ProjectTree | null>(null);
  const [error, setError] = useState<string | null>(null);
  // Whether the backend's `AppState.store_path` is set — mirrored here only
  // so "Save" knows whether it can skip the dialog; the backend remains the
  // source of truth and still refuses `save_project` if this ever drifts.
  const [hasStorePath, setHasStorePath] = useState(false);
  const [selectedDeviceId, setSelectedDeviceId] = useState<number | null>(null);
  const [deviceDetail, setDeviceDetail] = useState<DeviceDetail | null>(null);

  function resetTree(newTree: ProjectTree) {
    setTree(newTree);
    setSelectedDeviceId(null);
    setDeviceDetail(null);
  }

  async function selectDevice(id: number) {
    setSelectedDeviceId(id);
    setError(null);
    try {
      setDeviceDetail(await invoke<DeviceDetail>("device_detail", { deviceId: id }));
    } catch (e) {
      setError(String(e));
      setDeviceDetail(null);
    }
  }

  // After any command/undo/redo: the tree refreshes unconditionally (an
  // address edit changes its label), and the currently selected device's
  // detail refreshes alongside it (its own fields, or nothing if the edit
  // targeted a different device — device_detail is cheap enough to always
  // refetch rather than track which device a given command touched).
  async function handleTreeUpdate(newTree: ProjectTree) {
    setTree(newTree);
    if (selectedDeviceId !== null) {
      try {
        setDeviceDetail(await invoke<DeviceDetail>("device_detail", { deviceId: selectedDeviceId }));
      } catch (e) {
        setError(String(e));
      }
    }
  }

  async function pickProject() {
    const path = await open({
      multiple: false,
      filters: [{ name: "ETS project", extensions: ["knxproj"] }],
    });
    if (typeof path !== "string") return;
    setError(null);
    try {
      resetTree(await invoke<ProjectTree>("open_project", { path }));
      setHasStorePath(false); // ETS import has no `.knxdb` location yet
    } catch (e) {
      setError(String(e));
    }
  }

  async function openNativeProject() {
    const path = await open({ multiple: false, filters: KNXDB_FILTER });
    if (typeof path !== "string") return;
    setError(null);
    try {
      resetTree(await invoke<ProjectTree>("open_native_project", { path }));
      setHasStorePath(true);
    } catch (e) {
      setError(String(e));
    }
  }

  async function saveProjectAs() {
    const path = await save({ filters: KNXDB_FILTER, defaultPath: "project.knxdb" });
    if (typeof path !== "string") return;
    setError(null);
    try {
      await invoke("save_project_as", { path });
      setHasStorePath(true);
    } catch (e) {
      setError(String(e));
    }
  }

  async function saveProject() {
    if (!hasStorePath) return saveProjectAs();
    setError(null);
    try {
      await invoke("save_project");
    } catch (e) {
      setError(String(e));
    }
  }

  return (
    <main>
      <button onClick={pickProject}>Open project…</button>
      <button onClick={openNativeProject}>Open (.knxdb)…</button>
      <button onClick={saveProject} disabled={!tree}>
        Save
      </button>
      <button onClick={saveProjectAs} disabled={!tree}>
        Save As…
      </button>
      {error && (
        <p role="alert" className="error-banner">
          {error}
        </p>
      )}
      {tree && (
        <div className="workspace">
          <ProjectExplorer tree={tree} selectedId={selectedDeviceId} onSelectDevice={selectDevice} />
          {deviceDetail && (
            <Inspector key={deviceDetail.id} detail={deviceDetail} onApplied={handleTreeUpdate} />
          )}
        </div>
      )}
    </main>
  );
}

export default App;
```

- [ ] **Step 3: Add layout and inspector styles**

In `apps/knx-desktop/src/styles.css`, append:

```css
.workspace {
  display: flex;
  gap: 1.5rem;
  align-items: flex-start;
}

.project-explorer {
  flex: 1 1 40%;
  min-width: 0;
}

.inspector {
  flex: 1 1 60%;
  min-width: 0;
}

.inspector-description {
  opacity: 0.8;
  font-style: italic;
}

.inspector-field {
  display: block;
  margin-bottom: 0.75rem;
}

.inspector-field input {
  display: block;
  margin-top: 0.25rem;
}

.field-error {
  display: block;
  color: #b00020;
  font-size: 0.85em;
}

.com-object-list {
  list-style: none;
  padding-left: 0;
}

.com-object-list li {
  padding: 0.5rem 0;
  border-top: 1px solid color-mix(in srgb, currentColor 15%, transparent);
}

.com-object-label {
  display: block;
  font-weight: bold;
}

.provenance-badge {
  display: inline-block;
  font-size: 0.75em;
  opacity: 0.7;
  border: 1px solid currentColor;
  border-radius: 3px;
  padding: 0 0.35em;
}
```

- [ ] **Step 4: Type-check and build**

Run: `cd apps/knx-desktop && npm run build`
Expected: PASS, no TypeScript errors

- [ ] **Step 5: Run the full workspace build**

Run: `cargo build --workspace`
Expected: PASS, no regressions

- [ ] **Step 6: Commit**

```bash
git add apps/knx-desktop/src/Inspector.tsx apps/knx-desktop/src/App.tsx apps/knx-desktop/src/styles.css
git commit -m "feat(knx-desktop): properties inspector with address/DPT editing"
```

- [ ] **Step 7: Manual smoke check**

Not gating (no frontend test infra — see Global Constraints), but worth doing before moving on: run the app (`cargo tauri dev` from `apps/knx-desktop/src-tauri`, or use the `run` skill), open the reference `.knxproj`, click a device, confirm the Inspector shows its name/description/communication objects, edit the address to an invalid value (e.g. `9.9.9.9`) and confirm an inline error appears without changing the tree label, then edit it to a valid free address and confirm the tree label updates.

---

## Task 6: Frontend — undo/redo

**Files:**
- Modify: `apps/knx-desktop/src/App.tsx`
- Modify: `apps/knx-desktop/src/styles.css`

**Interfaces:**
- Consumes: Tauri commands `undo`/`redo` (Task 3), `tree.can_undo`/`tree.can_redo` (Task 1), `handleTreeUpdate` (Task 5).
- Produces: nothing consumed by a later task — this is the last task in the plan.

- [ ] **Step 1: Add undo/redo handlers, toolbar buttons, and keyboard shortcuts**

In `apps/knx-desktop/src/App.tsx`, add these two functions next to `saveProject` (both reuse `handleTreeUpdate` from Task 5, so `deviceDetail` refreshes if it's stale after the undo/redo):

```tsx
  async function undo() {
    setError(null);
    try {
      await handleTreeUpdate(await invoke<ProjectTree>("undo"));
    } catch (e) {
      setError(String(e));
    }
  }

  async function redo() {
    setError(null);
    try {
      await handleTreeUpdate(await invoke<ProjectTree>("redo"));
    } catch (e) {
      setError(String(e));
    }
  }
```

Add the keyboard shortcut effect (needs `useEffect` — add it to the `react` import: `import { useEffect, useState } from "react";`). Place this after the state declarations, before `resetTree`:

```tsx
  useEffect(() => {
    function handleKeyDown(e: KeyboardEvent) {
      if (!(e.ctrlKey || e.metaKey) || e.key.toLowerCase() !== "z") return;
      e.preventDefault();
      if (e.shiftKey) {
        if (tree?.can_redo) void redo();
      } else {
        if (tree?.can_undo) void undo();
      }
    }
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  });
```

(No dependency array: the handler closes over `tree`/`undo`/`redo`, which change on every render — re-subscribing each render keeps it correct at the cost of one extra listener churn per render, cheap for a single global key handler.)

Add the toolbar buttons in the JSX, after the existing "Save As…" button:

```tsx
      <button onClick={undo} disabled={!tree?.can_undo}>
        Undo
      </button>
      <button onClick={redo} disabled={!tree?.can_redo}>
        Redo
      </button>
```

- [ ] **Step 2: Type-check and build**

Run: `cd apps/knx-desktop && npm run build`
Expected: PASS, no TypeScript errors

- [ ] **Step 3: Commit**

```bash
git add apps/knx-desktop/src/App.tsx
git commit -m "feat(knx-desktop): undo/redo toolbar and Ctrl+Z/Ctrl+Shift+Z"
```

- [ ] **Step 4: Manual smoke check**

Run the app, edit a device's address, confirm Undo reverts the tree label and Redo reapplies it, both via the toolbar buttons and via `Ctrl+Z`/`Ctrl+Shift+Z`; confirm both buttons are disabled with nothing to undo/redo.

- [ ] **Step 5: Update `docs/IMPLEMENTATION_STATUS.md`**

Move this cycle's summary from "Next session" into the session log the same way Cycle 3 was recorded, and update "Next session" to describe the remaining Session 5 slice: Search. Follow the existing entries' structure and detail level — read the current file first.

- [ ] **Step 6: Final workspace check and commit the docs**

Run: `cargo build --workspace && cargo test --workspace && cargo fmt --all --check && cargo clippy --workspace --all-targets -- -D warnings && cd apps/knx-desktop && npm run build`
Expected: all PASS

```bash
git add docs/IMPLEMENTATION_STATUS.md
git commit -m "docs(session5): record selection + properties inspector (cycle 4)"
```
