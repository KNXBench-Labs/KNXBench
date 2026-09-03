# knx-desktop shell, projection layer, Project Explorer — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A user can launch the desktop app, pick a `.knxproj` file, and see
its topology and building hierarchy as a tree (the Project Explorer).

**Architecture:** A new pure-Rust crate `knx-projection` (depends only on
`knx-core`) turns a `knx_core::Project` into a display-shaped `ProjectTree`,
with `ts-rs`-generated TypeScript bindings. A new Tauri v2 app,
`apps/knx-desktop`, wraps `knx_app::import_ets_project` and
`knx_projection::build_project_tree` behind one command, `open_project`, and
a React frontend renders the returned tree.

**Tech Stack:** Rust 1.98.0 (workspace pin), Tauri 2.11.5, `tauri-plugin-dialog`
2.7.3, `ts-rs` 12.0.1, React 19.2.8, TypeScript 7.0.2, Vite 8.2.2, Node.js
≥22.12 (pinned to 22 in CI). Plain CSS, no UI component library.

**Spec:** [docs/superpowers/specs/2026-09-03-knx-desktop-shell-design.md](../specs/2026-09-03-knx-desktop-shell-design.md)

## Global Constraints

- Rust edition 2021, workspace toolchain 1.98.0 (`rust-toolchain.toml`) — do not bump.
- Every new crate: `edition.workspace = true`, `license.workspace = true`, `repository.workspace = true`, `rust-version.workspace = true`, `publish = false` — matches every existing crate's `Cargo.toml` header.
- `knx-projection` must depend on `knx-core` only. No `serde_json`, `quick-xml`, `rusqlite`, `tokio` (enforced by `xtask check-layering`, Task 3).
- No UI component/styling library in the frontend — plain CSS, hand-built components (spec decision).
- "Open project" this cycle is import-and-display only — no save/reload of knx-desktop's own state.
- All version numbers below (`tauri = "2.11.5"` etc.) were resolved and verified to compile/pass `cargo deny check` during planning (2026-09-03) — use them as pinned minimums (`"2.11"` style ranges are fine; do not go below what is listed).
- `cargo fmt --all` and `cargo clippy --workspace --all-targets -- -D warnings` must stay clean after every task.

---

### Task 1: In-memory store connection for knx-store

**Files:**
- Modify: `crates/knx-store/src/migration.rs`
- Modify: `crates/knx-store/src/lib.rs:8` (re-export)

**Interfaces:**
- Produces: `pub fn knx_store::open_and_migrate_in_memory() -> Result<Connection, MigrationError>` — same migration chain as `open_and_migrate`, but against `rusqlite::Connection::open_in_memory()`. Used by Task 4's `open_project` command, which needs a `Connection` to satisfy `knx_app::import_ets_project`'s signature without creating a project file of its own (spec: "Store-Connection" decision).

- [ ] **Step 1: Write the failing test**

Add to `crates/knx-store/src/migration.rs`, inside the existing `#[cfg(test)] mod tests` block:

```rust
    #[test]
    fn in_memory_connection_migrates_to_current_version() {
        let conn = open_and_migrate_in_memory().unwrap();
        let version: i64 = conn
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .unwrap();
        assert_eq!(version, CURRENT_SCHEMA_VERSION);
        // The opaque table exists, same as a fresh file-backed connection.
        conn.execute("SELECT COUNT(*) FROM opaque_entry", [])
            .unwrap();
    }
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p knx-store in_memory_connection_migrates_to_current_version`
Expected: FAIL — `open_and_migrate_in_memory` not found.

- [ ] **Step 3: Extract the shared migration step and add the in-memory entry point**

Replace the existing `open_and_migrate` function in `crates/knx-store/src/migration.rs` with:

```rust
/// Opens (creating if absent) the SQLite file at `path`, runs every pending
/// migration in order, and returns the connection at
/// `CURRENT_SCHEMA_VERSION`.
pub fn open_and_migrate(path: &Path) -> Result<Connection, MigrationError> {
    let conn = Connection::open(path)?;
    migrate(&conn)?;
    Ok(conn)
}

/// Opens an in-memory database and runs every migration in order — the same
/// chain as [`open_and_migrate`], but with nothing written to disk and
/// nothing left behind when the connection is dropped. For callers that
/// need a `Connection` to satisfy an API built around persistence (the
/// opaque store, the manifest table) without wanting a project file of
/// their own — e.g. the desktop app importing a `.knxproj` purely to
/// display it, with no save/reload feature yet (Session 5 cycle 1).
pub fn open_and_migrate_in_memory() -> Result<Connection, MigrationError> {
    let conn = Connection::open_in_memory()?;
    migrate(&conn)?;
    Ok(conn)
}

/// Runs every pending migration against an already-open connection and
/// brings its `user_version` to `CURRENT_SCHEMA_VERSION`. Shared by the
/// file-backed and in-memory entry points so the two can never drift.
fn migrate(conn: &Connection) -> Result<(), MigrationError> {
    let found: i64 = conn.query_row("PRAGMA user_version", [], |row| row.get(0))?;

    if found > CURRENT_SCHEMA_VERSION {
        return Err(MigrationError::FutureSchemaVersion {
            found,
            supported: CURRENT_SCHEMA_VERSION,
        });
    }

    let pending = &migrations()[found as usize..CURRENT_SCHEMA_VERSION as usize];
    for migration in pending {
        migration(conn)?;
    }

    if found < CURRENT_SCHEMA_VERSION {
        conn.pragma_update(None, "user_version", CURRENT_SCHEMA_VERSION)?;
    }

    Ok(())
}
```

Then in `crates/knx-store/src/lib.rs:8`, change:

```rust
pub use migration::{open_and_migrate, MigrationError, CURRENT_SCHEMA_VERSION};
```

to:

```rust
pub use migration::{
    open_and_migrate, open_and_migrate_in_memory, MigrationError, CURRENT_SCHEMA_VERSION,
};
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p knx-store`
Expected: all pass, including `in_memory_connection_migrates_to_current_version`.

- [ ] **Step 5: Format, lint, commit**

```bash
cargo fmt --all
cargo clippy -p knx-store --all-targets -- -D warnings
git add crates/knx-store/src/migration.rs crates/knx-store/src/lib.rs
git commit -m "feat(store): in-memory migrated connection for display-only imports"
```

---

### Task 2: `knx-projection` crate — ProjectTree and build_project_tree

**Files:**
- Create: `crates/knx-projection/Cargo.toml`
- Create: `crates/knx-projection/src/lib.rs`
- Modify: `Cargo.toml` (workspace members + `knx-projection` workspace dependency + `ts-rs` workspace dependency)

**Interfaces:**
- Consumes: `knx_core::{Project, Installation, Topology, Area, Line, Devices, DeviceInstance, BuildingPart, BuildingPartType, DeviceId, AreaId, LineId, BuildingPartId, InstallationId}` (all re-exported at `knx_core`'s crate root).
- Produces (used by Task 4): `pub fn knx_projection::build_project_tree(project: &knx_core::Project) -> ProjectTree`, and the public types `ProjectTree { schema_version: u32, warnings: usize, installations: Vec<InstallationNode> }`, `InstallationNode`, `AreaNode`, `LineNode`, `BuildingNode`, `DeviceNode` (fields as below). `ProjectTree::warnings` is always `0` from `build_project_tree` alone — the caller (Task 4) overwrites it with the import report's count, since `knx-projection` does not depend on `knx-etsproj`/`knx-app` and never sees `ImportReport`.

**File structure inside the crate:** everything in one `src/lib.rs` — the types and the one building function are small enough together (mirrors `knx-core`'s own single-file modules like `topology.rs`); split out only if this file grows past what one task's reviewer can hold in view (not expected this cycle).

- [ ] **Step 1: Register the crate in the workspace**

In the root `Cargo.toml`, add to `members`:

```toml
    "crates/knx-projection",
```
(placed after `"crates/knx-productdb",` and before `"crates/knx-net",`, matching the existing dependency order.)

Add to `[workspace.dependencies]`:

```toml
knx-projection = { path = "crates/knx-projection" }
```
(placed after `knx-productdb = { path = "crates/knx-productdb" }`.)

Add `ts-rs` to `[workspace.dependencies]` (after `serde_json = "1"`):

```toml
ts-rs = "12"
```

- [ ] **Step 2: Create the crate manifest**

`crates/knx-projection/Cargo.toml`:

```toml
[package]
name = "knx-projection"
version = "0.0.0"
edition.workspace = true
license.workspace = true
repository.workspace = true
rust-version.workspace = true
publish = false

[dependencies]
knx-core.workspace = true
serde.workspace = true
ts-rs.workspace = true
```

- [ ] **Step 3: Write the failing tests**

`crates/knx-projection/src/lib.rs` — tests first, at the bottom of the file (the implementation in Step 4 goes above them, same file, matching every other crate in this workspace):

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use knx_core::{
        Area, BuildingPart, BuildingPartType, CommissioningState, CompletionStatus, DeviceId,
        DeviceInstance, IndividualAddress, Installation, InstallationId, Language, Line, Project,
        SourceRef, Topology,
    };

    fn source() -> SourceRef {
        SourceRef {
            path: "t".into(),
            ets_id: "t".into(),
        }
    }

    fn device(id: u32, name: &str, address: Option<(u8, u8, u8)>) -> DeviceInstance {
        DeviceInstance {
            id: DeviceId(id),
            source: source(),
            name: name.into(),
            description: None,
            address: address.map(|(a, l, d)| IndividualAddress::new(a, l, d).unwrap()),
            product_ref: String::new(),
            program_ref: String::new(),
            commissioning: CommissioningState::default(),
            visibility_calculated: true,
            com_objects: vec![],
            binary_data: vec![],
        }
    }

    fn building(
        id: u32,
        name: &str,
        kind: BuildingPartType,
        parent: Option<u32>,
        children: Vec<u32>,
        devices: Vec<u32>,
    ) -> BuildingPart {
        BuildingPart {
            id: knx_core::BuildingPartId(id),
            source: source(),
            name: name.into(),
            number: None,
            kind,
            default_line: None,
            completion: CompletionStatus::Undefined,
            children: children.into_iter().map(knx_core::BuildingPartId).collect(),
            devices: devices.into_iter().map(DeviceId).collect(),
            parent: parent.map(knx_core::BuildingPartId),
        }
    }

    fn empty_installation() -> Installation {
        Installation {
            id: InstallationId(0),
            name: "I".into(),
            default_line: None,
            multicast_address: None,
            completion: CompletionStatus::Undefined,
            topology: Topology {
                areas: vec![],
                lines: vec![],
                unassigned: vec![],
            },
            buildings: vec![],
            group_ranges: vec![],
            group_addresses: vec![],
            parameters: vec![],
        }
    }

    #[test]
    fn empty_project_produces_an_empty_tree() {
        let project = Project::new(Language("en".into()));
        let tree = build_project_tree(&project);
        assert_eq!(tree.schema_version, project.schema_version);
        assert_eq!(tree.warnings, 0);
        assert!(tree.installations.is_empty());
    }

    #[test]
    fn installation_with_no_buildings_or_topology_has_empty_children() {
        let mut project = Project::new(Language("en".into()));
        project.installations.push(empty_installation());
        let tree = build_project_tree(&project);
        let inst = &tree.installations[0];
        assert!(inst.topology.is_empty());
        assert!(inst.buildings.is_empty());
        assert!(inst.unassigned.is_empty());
    }

    #[test]
    fn topology_resolves_area_line_device_in_order() {
        let mut project = Project::new(Language("en".into()));
        project.devices.insert(device(1, "Dimmer", Some((1, 1, 1))));
        project.devices.insert(device(2, "Switch", None));

        let mut inst = empty_installation();
        inst.topology.areas.push(Area {
            id: knx_core::AreaId(1),
            source: source(),
            name: "Area 1".into(),
            address: 1,
            completion: CompletionStatus::Undefined,
            lines: vec![knx_core::LineId(1)],
        });
        inst.topology.lines.push(Line {
            id: knx_core::LineId(1),
            source: source(),
            name: "Line 1".into(),
            address: 1,
            medium_ref: "TP".into(),
            domain_address: None,
            domain_address_is_checked: None,
            ip_routing_multicast_address: None,
            multicast_ttl: None,
            completion: CompletionStatus::Undefined,
            devices: vec![DeviceId(1), DeviceId(2)],
        });
        project.installations.push(inst);

        let tree = build_project_tree(&project);
        let area = &tree.installations[0].topology[0];
        assert_eq!(area.name, "Area 1");
        assert_eq!(area.lines[0].devices.len(), 2);
        assert_eq!(area.lines[0].devices[0].name, "Dimmer");
        assert_eq!(area.lines[0].devices[0].address.as_deref(), Some("1.1.1"));
        assert_eq!(area.lines[0].devices[1].address, None);
    }

    #[test]
    fn unassigned_devices_form_their_own_bucket() {
        let mut project = Project::new(Language("en".into()));
        project
            .devices
            .insert(device(9, "Orphan", None));
        let mut inst = empty_installation();
        inst.topology.unassigned.push(DeviceId(9));
        project.installations.push(inst);

        let tree = build_project_tree(&project);
        assert_eq!(tree.installations[0].unassigned.len(), 1);
        assert_eq!(tree.installations[0].unassigned[0].name, "Orphan");
    }

    #[test]
    fn building_hierarchy_nests_by_parent_child_not_flat() {
        let mut project = Project::new(Language("en".into()));
        project.devices.insert(device(5, "Lamp", None));
        let mut inst = empty_installation();
        inst.buildings = vec![
            building(1, "Building", BuildingPartType::Building, None, vec![2], vec![]),
            building(2, "Floor 1", BuildingPartType::Floor, Some(1), vec![3], vec![]),
            building(3, "Room 1", BuildingPartType::Room, Some(2), vec![], vec![5]),
        ];
        project.installations.push(inst);

        let tree = build_project_tree(&project);
        let roots = &tree.installations[0].buildings;
        assert_eq!(roots.len(), 1);
        assert_eq!(roots[0].name, "Building");
        assert_eq!(roots[0].kind, "Building");
        assert_eq!(roots[0].children[0].name, "Floor 1");
        assert_eq!(roots[0].children[0].children[0].name, "Room 1");
        assert_eq!(roots[0].children[0].children[0].devices[0].name, "Lamp");
    }

    #[test]
    fn a_dangling_building_device_reference_is_dropped_not_panicked() {
        // knx-etsproj's validate.rs deliberately does not check BuildingPart
        // device references for dangling ids (no measured case has motivated
        // it yet) — the projection must stay resilient to that gap rather
        // than crash the whole desktop app over one malformed project.
        let mut project = Project::new(Language("en".into()));
        let mut inst = empty_installation();
        inst.buildings = vec![building(
            1,
            "Room",
            BuildingPartType::Room,
            None,
            vec![],
            vec![404],
        )];
        project.installations.push(inst);

        let tree = build_project_tree(&project);
        assert!(tree.installations[0].buildings[0].devices.is_empty());
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p knx-projection`
Expected: FAIL to compile — no types/`build_project_tree` defined yet.

- [ ] **Step 4: Write the implementation**

Above the `#[cfg(test)]` block in `crates/knx-projection/src/lib.rs`:

```rust
//! Display-shaped projections of [`knx_core::Project`] for the desktop UI
//! (ADR-0009). The UI never receives `Project` itself — only these types,
//! generated into TypeScript by `ts-rs` so the two sides cannot disagree
//! without the build failing. Depends on `knx-core` only: no IO, no format,
//! no storage (`xtask check-layering` enforces this, same rule as
//! `knx-core` itself).

use std::collections::HashMap;

use knx_core::{
    BuildingPart, BuildingPartId, BuildingPartType, Devices, Project, Topology,
};
use serde::Serialize;
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct ProjectTree {
    pub schema_version: u32,
    /// Count of import-report items (errors, unknown constructs, DPT
    /// conflicts, documented capability gaps) that a caller with access to
    /// the `ImportReport` should fill in — always `0` straight out of
    /// [`build_project_tree`], since this crate never sees that type
    /// (CLAUDE.md: never silently discard information; full drill-down is
    /// a later cycle, this is the count that says something is worth
    /// looking at).
    pub warnings: usize,
    pub installations: Vec<InstallationNode>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct InstallationNode {
    pub id: u8,
    pub name: String,
    pub topology: Vec<AreaNode>,
    pub buildings: Vec<BuildingNode>,
    pub unassigned: Vec<DeviceNode>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct AreaNode {
    pub id: u32,
    pub name: String,
    pub address: u8,
    pub lines: Vec<LineNode>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct LineNode {
    pub id: u32,
    pub name: String,
    pub address: u8,
    pub devices: Vec<DeviceNode>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct BuildingNode {
    pub id: u32,
    pub name: String,
    /// `BuildingPartType` as a plain string (e.g. `"Room"`, `"Floor"`) — the
    /// enum itself stays in `knx-core`; a typed TS union is not worth the
    /// extra `ts-rs` surface for a single label this cycle.
    pub kind: String,
    pub children: Vec<BuildingNode>,
    pub devices: Vec<DeviceNode>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct DeviceNode {
    pub id: u32,
    pub name: String,
    /// Formatted individual address (e.g. `"1.1.1"`) — `None` if the device
    /// has no address assigned, which is valid project state.
    pub address: Option<String>,
    pub description: Option<String>,
}

/// Builds the full display tree for every installation in `project`. Pure
/// and total: never panics on a project that imported successfully, even
/// one with dangling `BuildingPart` device references (knx-etsproj's
/// `validate.rs` does not check those — see the doc comment there).
pub fn build_project_tree(project: &Project) -> ProjectTree {
    ProjectTree {
        schema_version: project.schema_version,
        warnings: 0,
        installations: project
            .installations
            .iter()
            .map(|inst| build_installation(inst, &project.devices))
            .collect(),
    }
}

fn build_installation(
    inst: &knx_core::Installation,
    devices: &Devices,
) -> InstallationNode {
    InstallationNode {
        id: inst.id.0,
        name: inst.name.clone(),
        topology: build_topology(&inst.topology, devices),
        buildings: build_building_forest(&inst.buildings, devices),
        unassigned: inst
            .topology
            .unassigned
            .iter()
            .filter_map(|id| devices.get(*id))
            .map(build_device_node)
            .collect(),
    }
}

fn build_topology(topology: &Topology, devices: &Devices) -> Vec<AreaNode> {
    topology
        .areas
        .iter()
        .map(|area| AreaNode {
            id: area.id.0,
            name: area.name.clone(),
            address: area.address,
            lines: area
                .lines
                .iter()
                .filter_map(|line_id| topology.line(*line_id))
                .map(|line| LineNode {
                    id: line.id.0,
                    name: line.name.clone(),
                    address: line.address,
                    devices: line
                        .devices
                        .iter()
                        .filter_map(|id| devices.get(*id))
                        .map(build_device_node)
                        .collect(),
                })
                .collect(),
        })
        .collect()
}

fn build_device_node(device: &knx_core::DeviceInstance) -> DeviceNode {
    DeviceNode {
        id: device.id.0,
        name: device.name.clone(),
        address: device.address.map(|a| a.to_string()),
        description: device.description.clone(),
    }
}

/// `BuildingPart`s are stored flat, linked by `parent`/`children` ids
/// (DATA_MODEL §5) — this resolves that into the actual nested shape the
/// tree needs, once, in Rust, per ADR-0009.
fn build_building_forest(parts: &[BuildingPart], devices: &Devices) -> Vec<BuildingNode> {
    let by_id: HashMap<BuildingPartId, &BuildingPart> =
        parts.iter().map(|p| (p.id, p)).collect();

    parts
        .iter()
        .filter(|p| p.parent.is_none())
        .map(|root| build_building_node(root, &by_id, devices))
        .collect()
}

fn build_building_node(
    part: &BuildingPart,
    by_id: &HashMap<BuildingPartId, &BuildingPart>,
    devices: &Devices,
) -> BuildingNode {
    BuildingNode {
        id: part.id.0,
        name: part.name.clone(),
        kind: building_kind_str(part.kind).to_string(),
        children: part
            .children
            .iter()
            .filter_map(|id| by_id.get(id))
            .map(|child| build_building_node(child, by_id, devices))
            .collect(),
        devices: part
            .devices
            .iter()
            .filter_map(|id| devices.get(*id))
            .map(build_device_node)
            .collect(),
    }
}

fn building_kind_str(kind: BuildingPartType) -> &'static str {
    match kind {
        BuildingPartType::Building => "Building",
        BuildingPartType::Floor => "Floor",
        BuildingPartType::Room => "Room",
        BuildingPartType::Corridor => "Corridor",
        BuildingPartType::DistributionBoard => "DistributionBoard",
        BuildingPartType::BuildingPart => "BuildingPart",
    }
}
```

Also add, to the test module's `use super::*;` scope, a small helper trait-free check: `Topology` needs no extra helper since `Topology::line` already exists in `knx-core`.

Note: `InstallationNode::id` is `u8` (matches `InstallationId(pub u8)`), every other node's `id` is `u32` (matches every other id newtype's `u32` repr) — keep this distinction, do not widen `InstallationNode::id` to `u32`.

- [ ] **Step 5: Run test to verify it passes**

Run: `cargo test -p knx-projection`
Expected: all 6 tests pass.

- [ ] **Step 6: Generate the TypeScript bindings**

```bash
TS_RS_EXPORT_DIR=../../apps/knx-desktop/src/bindings cargo test -p knx-projection
```

This writes `ProjectTree.ts`, `InstallationNode.ts`, `AreaNode.ts`, `LineNode.ts`, `BuildingNode.ts`, `DeviceNode.ts` into `apps/knx-desktop/src/bindings/` (created automatically). Task 5 creates the rest of `apps/knx-desktop`; these six files may exist alone in that directory until then — that is expected, do not delete them.

Verify:
```bash
ls apps/knx-desktop/src/bindings/
```
Expected: the six `.ts` files listed above.

- [ ] **Step 7: Gitignore the default ts-rs export location**

Every plain `cargo test` (not just the `TS_RS_EXPORT_DIR`-prefixed one above)
re-runs the same `#[ts(export)]`-generated tests, and without that
environment variable set they fall back to writing into
`crates/knx-projection/bindings/` (ts-rs's own default, relative to the
crate). That happens on every future `cargo test --workspace`, including
CI's plain `Test` step (Task 8) — harmless, but untracked clutter unless
ignored. Add to the root `.gitignore`:

```
crates/knx-projection/bindings/
```

- [ ] **Step 8: Format, lint, commit**

```bash
cargo fmt --all
cargo clippy -p knx-projection --all-targets -- -D warnings
git add Cargo.toml Cargo.lock .gitignore crates/knx-projection apps/knx-desktop/src/bindings
git commit -m "feat(projection): ProjectTree and build_project_tree, with ts-rs bindings"
```

---

### Task 3: `knx-projection` as a fourth `check-layering` root

**Files:**
- Modify: `xtask/src/main.rs`

**Interfaces:**
- Consumes: `layering::forbidden_reachable`, `layering::CORE_FORBIDDEN` (both already public, unchanged).

- [ ] **Step 1: Write the failing test**

`check-layering` has no unit test of its own (it is an integration-style binary checked by running it) — the "test" here is running it against the real workspace graph, both before and after the change, per the existing pattern for this task type. Skip to Step 3; there is no separate test file to add.

- [ ] **Step 2: Run the current check to confirm the starting point**

Run: `cargo run -p xtask -- check-layering`
Expected: passes (three roots, as today) — this is the "before" baseline, not a failing test, since there is nothing yet to check for `knx-projection`.

- [ ] **Step 3: Add the fourth root**

In `xtask/src/main.rs`, inside `check_layering()`, after the `knx-productdb` block:

```rust
    // knx-projection turns Project into display-shaped structs for the
    // desktop UI (ADR-0009, Session 5). It must stay exactly as free of IO
    // and storage as knx-core itself — a projection layer that reached
    // rusqlite or quick-xml directly would defeat the point of having one.
    violations.extend(layering::forbidden_reachable(
        &graph,
        "knx-projection",
        layering::CORE_FORBIDDEN,
    ));
```

Update the success message right below to mention it:

```rust
    if violations.is_empty() {
        println!(
            "layering ok: knx-core reaches none of {:?}; knx-etsproj does not reach knx-store; \
             knx-productdb reaches neither knx-etsproj nor knx-store; knx-projection reaches \
             none of {:?}",
            layering::CORE_FORBIDDEN,
            layering::CORE_FORBIDDEN
        );
        return ExitCode::SUCCESS;
    }
```

- [ ] **Step 4: Run to verify it passes**

Run: `cargo run -p xtask -- check-layering`
Expected: `layering ok: ...` printed, `ExitCode::SUCCESS`.

- [ ] **Step 5: Format, lint, commit**

```bash
cargo fmt --all
cargo clippy -p xtask --all-targets -- -D warnings
git add xtask/src/main.rs
git commit -m "chore(xtask): knx-projection as a fourth check-layering root"
```

---

### Task 4: `apps/knx-desktop/src-tauri` — the Tauri shell and `open_project`

**Files:**
- Create: `apps/knx-desktop/src-tauri/Cargo.toml`
- Create: `apps/knx-desktop/src-tauri/build.rs`
- Create: `apps/knx-desktop/src-tauri/tauri.conf.json`
- Create: `apps/knx-desktop/src-tauri/capabilities/default.json`
- Create: `apps/knx-desktop/src-tauri/icons/icon.png`
- Create: `apps/knx-desktop/src-tauri/src/lib.rs`
- Create: `apps/knx-desktop/src-tauri/src/main.rs`
- Create: `apps/knx-desktop/src-tauri/tests/open_reference_project.rs`
- Modify: `Cargo.toml` (workspace members + `tauri`/`tauri-build`/`tauri-plugin-dialog` workspace dependencies)

**Interfaces:**
- Consumes: `knx_store::open_and_migrate_in_memory` (Task 1), `knx_projection::build_project_tree` (Task 2), `knx_app::import_ets_project`, `knx_app::AppError` (existing).
- Produces (used by Task 6): the Tauri command `open_project(path: String) -> Result<ProjectTree, String>`, registered via `tauri::generate_handler![open_project]`; the app-managed state `AppState { project: Mutex<Option<knx_core::Project>> }`.

- [ ] **Step 1: Register the crate and its new dependencies**

In the root `Cargo.toml`, add to `members` (after `"apps/knx-cli",`):

```toml
    "apps/knx-desktop/src-tauri",
```

Add to `[workspace.dependencies]` (after `tempfile = "3"`):

```toml
tauri = "2.11"
tauri-build = "2.6"
tauri-plugin-dialog = "2.7"
```

- [ ] **Step 2: Write the crate manifest**

`apps/knx-desktop/src-tauri/Cargo.toml`:

```toml
[package]
name = "knx-desktop"
version = "0.0.0"
edition.workspace = true
license.workspace = true
repository.workspace = true
rust-version.workspace = true
publish = false

[lib]
name = "knx_desktop_lib"
crate-type = ["staticlib", "cdylib", "rlib"]

[build-dependencies]
tauri-build.workspace = true

[dependencies]
knx-app.workspace = true
knx-core.workspace = true
knx-projection.workspace = true
knx-store.workspace = true
tauri.workspace = true
tauri-plugin-dialog.workspace = true
```

- [ ] **Step 3: Tauri configuration, capability and icon**

`apps/knx-desktop/src-tauri/build.rs`:

```rust
fn main() {
    tauri_build::build()
}
```

`apps/knx-desktop/src-tauri/tauri.conf.json`:

```json
{
  "$schema": "https://schema.tauri.app/config/2",
  "productName": "knx-desktop",
  "version": "0.0.0",
  "identifier": "com.knxbench.knxbench-labs",
  "build": {
    "frontendDist": "../dist",
    "devUrl": "http://localhost:1420",
    "beforeDevCommand": "npm run dev",
    "beforeBuildCommand": "npm run build"
  },
  "app": {
    "windows": [
      { "label": "main", "title": "KNX", "width": 1200, "height": 800 }
    ]
  },
  "bundle": {
    "active": false
  }
}
```

(`bundle.active: false` — packaging, and the icon set a real bundle needs,
is Session 7's job per ROADMAP.md; this cycle only needs the one icon file
below, which Tauri's code generation reads regardless of bundling.)

`apps/knx-desktop/src-tauri/capabilities/default.json`:

```json
{
  "identifier": "default",
  "description": "Capabilities for the main window",
  "windows": ["main"],
  "permissions": ["core:default", "dialog:default"]
}
```

`apps/knx-desktop/src-tauri/icons/icon.png`: a placeholder application icon.
Tauri's `generate_context!()` macro reads this file at compile time
regardless of `bundle.active`, and rejects anything that is not 8-bit RGBA.
Generate one with ImageMagick:

```bash
magick -size 32x32 xc:"#2b6cb0" -alpha set -define png:color-type=6 -depth 8 \
  apps/knx-desktop/src-tauri/icons/icon.png
```

Verify: `file apps/knx-desktop/src-tauri/icons/icon.png` reports
`PNG image data, 32 x 32, 8-bit/color RGBA, non-interlaced`. A real icon
(and the rest of the bundle icon set) is Session 7's packaging work, not
this cycle's.

- [ ] **Step 4: Write the failing test**

`apps/knx-desktop/src-tauri/tests/open_reference_project.rs`:

```rust
//! Confirms the projection did not drop or duplicate anything the importer
//! produced, against the same reference project and the same golden counts
//! Session 3's import golden test already established
//! (crates/knx-etsproj/tests/golden_reference_project.rs).

use std::path::PathBuf;

fn workspace_root() -> PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(std::path::Path::parent)
        .and_then(std::path::Path::parent)
        .expect("crate lives at <root>/apps/knx-desktop/src-tauri")
        .to_path_buf()
}

fn reference_ets4_path() -> PathBuf {
    workspace_root().join("Unser Zuhause ets4 - 2025-12-15.knxproj")
}

#[test]
fn opening_the_reference_project_yields_the_measured_counts() {
    let tree = knx_desktop_lib::open_project_impl(&reference_ets4_path()).unwrap();

    assert_eq!(tree.installations.len(), 1);
    let inst = &tree.installations[0];

    assert_eq!(inst.topology.len(), 1); // one area
    assert_eq!(inst.topology[0].lines.len(), 1);

    let device_count: usize = inst
        .topology
        .iter()
        .flat_map(|a| a.lines.iter())
        .map(|l| l.devices.len())
        .sum();
    assert_eq!(device_count, 35); // 35 on the line
    assert_eq!(inst.unassigned.len(), 1); // plus the one unassigned device

    assert_eq!(count_buildings(&inst.buildings), 22);
}

fn count_buildings(nodes: &[knx_projection::BuildingNode]) -> usize {
    nodes
        .iter()
        .map(|n| 1 + count_buildings(&n.children))
        .sum()
}
```

- [ ] **Step 5: Run test to verify it fails**

Run: `cargo test -p knx-desktop --test open_reference_project`
Expected: FAIL to compile — `open_project_impl` not defined yet.

- [ ] **Step 6: Write the implementation**

`apps/knx-desktop/src-tauri/src/lib.rs`:

```rust
//! The Tauri shell. Holds the imported project in memory
//! (`Mutex<Option<Project>>`) and exposes it to the frontend only as
//! display-shaped projections (ADR-0009) — `open_project` is the one
//! command this cycle needs.

use std::path::Path;
use std::sync::Mutex;

use knx_app::{AppError, ImportOptions};
use knx_projection::ProjectTree;

pub struct AppState {
    pub project: Mutex<Option<knx_core::Project>>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            project: Mutex::new(None),
        }
    }
}

/// Imports `path` and projects it, without touching any Tauri machinery —
/// this is what both the `open_project` command and the integration test
/// call, so the test needs no running `tauri::App` at all.
///
/// The store connection is in-memory and discarded when it drops
/// (`knx_store::open_and_migrate_in_memory`, Task 1): this cycle only
/// displays a project, it does not save or reload knx-desktop's own state.
pub fn open_project_impl(path: &Path) -> Result<ProjectTree, AppError> {
    let conn = knx_store::open_and_migrate_in_memory()?;
    let imported = knx_app::import_ets_project_with(path, &conn, ImportOptions::default())?;

    let mut tree = knx_projection::build_project_tree(&imported.project);
    tree.warnings = imported.report.errors.len()
        + imported.report.unknown.len()
        + imported.report.conflicts.len()
        + imported.report.unsupported.len();

    Ok(tree)
}

#[tauri::command]
fn open_project(
    path: String,
    state: tauri::State<AppState>,
) -> Result<ProjectTree, String> {
    let (tree, project) = {
        let conn = knx_store::open_and_migrate_in_memory().map_err(|e| e.to_string())?;
        let imported =
            knx_app::import_ets_project_with(Path::new(&path), &conn, ImportOptions::default())
                .map_err(|e| e.to_string())?;
        let mut tree = knx_projection::build_project_tree(&imported.project);
        tree.warnings = imported.report.errors.len()
            + imported.report.unknown.len()
            + imported.report.conflicts.len()
            + imported.report.unsupported.len();
        (tree, imported.project)
    };
    *state.project.lock().expect("state mutex poisoned") = Some(project);
    Ok(tree)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![open_project])
        .run(tauri::generate_context!())
        .expect("error while running knx-desktop");
}
```

`apps/knx-desktop/src-tauri/src/main.rs`:

```rust
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    knx_desktop_lib::run();
}
```

Note the duplication between `open_project_impl` and the `#[tauri::command]
open_project`: they cannot simply call one another because `open_project`
also needs to update `state.project`, and `open_project_impl` returns only
the tree (so the integration test does not need a `State` to construct).
This is deliberate, not an oversight — do not "simplify" it by making the
command call `open_project_impl` and then separately re-derive `project`,
which would mean importing twice.

- [ ] **Step 7: Run test to verify it passes**

Run: `cargo test -p knx-desktop`
Expected: `opening_the_reference_project_yields_the_measured_counts` passes.

- [ ] **Step 8: Run the whole workspace to confirm nothing else broke**

Run: `cargo test --workspace`
Expected: all pass (this is the first time `knx-desktop` compiles as part of
the workspace — if the Linux Tauri system libraries described in Task 8 are
missing locally, this step fails to link; install them first, see Task 8's
apt package list).

- [ ] **Step 9: Format, lint, commit**

```bash
cargo fmt --all
cargo clippy -p knx-desktop --all-targets -- -D warnings
git add Cargo.toml Cargo.lock apps/knx-desktop/src-tauri
git commit -m "feat(desktop): Tauri shell with the open_project command"
```

---

### Task 5: `deny.toml` — Tauri's Linux GTK3 backend is unmaintained upstream, not by us

**Files:**
- Modify: `deny.toml`
- Modify: `docs/KNOWN_LIMITATIONS.md`

**Why this task exists:** verified during planning — with `knx-desktop`
now in the workspace, `cargo deny check` fails with 16 `unmaintained`
RUSTSEC advisories, none of them vulnerabilities: 10 are the archived
gtk-rs GTK3 bindings (`RUSTSEC-2024-0411` through `-0420`, minus one gap)
that `tauri`'s own Linux backend depends on, 5 are the `unic-*` Unicode
crates (`RUSTSEC-2025-0075`, `-0080`, `-0081`, `-0098`, `-0100`) pulled in
transitively through `urlpattern` (used by `tauri-utils` for navigation/CSP
matching), and 1 is `proc-macro-error` (`RUSTSEC-2024-0370`) via the
tray/menu stack. Every one names "no safe upgrade is available" in its own
advisory text — this is the state of Tauri v2 on Linux today (2026-09-03,
`tauri` 2.11.5), not a version we picked and should instead bump; Tauri's
own migration to GTK4 is what will retire the ten GTK3 ones.

**Interfaces:** none — configuration only.

- [ ] **Step 1: Confirm the failure**

Run: `cargo deny check advisories`
Expected: FAILS, listing the 16 IDs above.

- [ ] **Step 2: Add the ignore list**

In `deny.toml`, change:

```toml
[advisories]
version = 2
yanked = "deny"
```

to:

```toml
[advisories]
version = 2
yanked = "deny"
# Every ID below is an "unmaintained" notice, not a vulnerability, and every
# one's own advisory text says no safe upgrade exists yet. All 16 are
# transitive dependencies of tauri 2.11 on Linux (Session 5), not something
# this repository chose or can route around independently:
#  - RUSTSEC-2024-0411/-0412/-0413/-0414/-0415/-0416/-0417/-0418/-0419/-0420:
#    the archived gtk-rs GTK3 bindings (atk/gdk/gdk-pixbuf/gio/glib/gtk/
#    pango/soup3/webkit2gtk and friends) — retired once Tauri ships its
#    GTK4 backend, tracked upstream, no fixed date.
#  - RUSTSEC-2025-0075/-0080/-0081/-0098/-0100: the `unic-*` Unicode crates,
#    via `urlpattern` (tauri-utils's navigation/CSP matching).
#  - RUSTSEC-2024-0370: `proc-macro-error`, via the tray/menu stack (`muda`).
# Re-check this list whenever `tauri`/`tauri-*` is bumped — some of these
# may already be gone.
ignore = [
    "RUSTSEC-2024-0411", "RUSTSEC-2024-0412", "RUSTSEC-2024-0413",
    "RUSTSEC-2024-0414", "RUSTSEC-2024-0415", "RUSTSEC-2024-0416",
    "RUSTSEC-2024-0417", "RUSTSEC-2024-0418", "RUSTSEC-2024-0419",
    "RUSTSEC-2024-0420", "RUSTSEC-2024-0370",
    "RUSTSEC-2025-0075", "RUSTSEC-2025-0080", "RUSTSEC-2025-0081",
    "RUSTSEC-2025-0098", "RUSTSEC-2025-0100",
]
```

- [ ] **Step 3: Run to verify it passes**

Run: `cargo deny check`
Expected: `advisories ok, bans ok, licenses ok, sources ok`.

- [ ] **Step 4: Document the limitation**

Add a new numbered entry to `docs/KNOWN_LIMITATIONS.md` (following its
existing format — read the file first to match its exact per-entry
structure), stating: Tauri v2's Linux backend depends on the archived
gtk-rs GTK3 bindings; `cargo deny check`'s advisory gate has 16 upstream
"unmaintained" notices suppressed in `deny.toml` with justification (no
vulnerabilities, no safe upgrade exists); the condition that lifts it is
Tauri shipping its GTK4 backend upstream.

- [ ] **Step 5: Commit**

```bash
git add deny.toml docs/KNOWN_LIMITATIONS.md
git commit -m "chore(deny): acknowledge Tauri's unmaintained GTK3 Linux backend"
```

---

### Task 6: `apps/knx-desktop` frontend scaffold

**Files:**
- Create: `apps/knx-desktop/package.json`
- Create: `apps/knx-desktop/tsconfig.json`
- Create: `apps/knx-desktop/vite.config.ts`
- Create: `apps/knx-desktop/index.html`
- Create: `apps/knx-desktop/src/main.tsx`
- Create: `apps/knx-desktop/src/App.tsx`
- Create: `apps/knx-desktop/src/styles.css`
- Create: `apps/knx-desktop/.gitignore`

**Interfaces:**
- Consumes: the six binding files Task 2 already generated into `apps/knx-desktop/src/bindings/`.
- Produces (used by Task 7): a building, type-checking frontend with a stub `App` that can be replaced. `pickProject(): Promise<void>` in `App.tsx` is the integration point Task 7 extends into the Explorer.

- [ ] **Step 1: Package manifest**

`apps/knx-desktop/package.json`:

```json
{
  "name": "knx-desktop",
  "private": true,
  "version": "0.0.0",
  "type": "module",
  "engines": {
    "node": ">=22.12.0"
  },
  "scripts": {
    "dev": "vite",
    "build": "tsc && vite build",
    "preview": "vite preview"
  },
  "dependencies": {
    "@tauri-apps/api": "2.11.1",
    "@tauri-apps/plugin-dialog": "2.7.3",
    "react": "19.2.8",
    "react-dom": "19.2.8"
  },
  "devDependencies": {
    "@types/react": "19.2.18",
    "@types/react-dom": "19.2.7",
    "@vitejs/plugin-react": "6.1.1",
    "typescript": "7.0.2",
    "vite": "8.2.2"
  }
}
```

- [ ] **Step 2: TypeScript and Vite configuration**

`apps/knx-desktop/tsconfig.json`:

```json
{
  "compilerOptions": {
    "target": "ES2022",
    "useDefineForClassFields": true,
    "lib": ["ES2022", "DOM", "DOM.Iterable"],
    "module": "ESNext",
    "skipLibCheck": true,
    "moduleResolution": "bundler",
    "allowImportingTsExtensions": true,
    "resolveJsonModule": true,
    "isolatedModules": true,
    "noEmit": true,
    "jsx": "react-jsx",
    "strict": true,
    "noUnusedLocals": true,
    "noUnusedParameters": true,
    "noFallthroughCasesInSwitch": true
  },
  "include": ["src"]
}
```

`apps/knx-desktop/vite.config.ts`:

```typescript
import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
  },
});
```

`apps/knx-desktop/index.html`:

```html
<!doctype html>
<html lang="en">
  <head>
    <meta charset="UTF-8" />
    <title>KNX</title>
  </head>
  <body>
    <div id="root"></div>
    <script type="module" src="/src/main.tsx"></script>
  </body>
</html>
```

`apps/knx-desktop/.gitignore`:

```
node_modules
dist
```

- [ ] **Step 3: Entry point and a stub App**

`apps/knx-desktop/src/main.tsx`:

```typescript
import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import App from "./App";
import "./styles.css";

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <App />
  </StrictMode>,
);
```

`apps/knx-desktop/src/App.tsx` (stub — Task 7 replaces the body with the
Project Explorer):

```typescript
import { useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { invoke } from "@tauri-apps/api/core";
import type { ProjectTree } from "./bindings/ProjectTree";

function App() {
  const [tree, setTree] = useState<ProjectTree | null>(null);
  const [error, setError] = useState<string | null>(null);

  async function pickProject() {
    const path = await open({
      multiple: false,
      filters: [{ name: "ETS project", extensions: ["knxproj"] }],
    });
    if (typeof path !== "string") return;
    setError(null);
    try {
      setTree(await invoke<ProjectTree>("open_project", { path }));
    } catch (e) {
      setError(String(e));
    }
  }

  return (
    <main>
      <button onClick={pickProject}>Open project…</button>
      {error && <p role="alert">{error}</p>}
      {tree && <pre>{JSON.stringify(tree, null, 2)}</pre>}
    </main>
  );
}

export default App;
```

`apps/knx-desktop/src/styles.css`:

```css
:root {
  color-scheme: light dark;
  font-family: system-ui, sans-serif;
}

body {
  margin: 0;
}

main {
  padding: 1rem;
}
```

- [ ] **Step 4: Install and build to verify**

```bash
cd apps/knx-desktop && npm install && npm run build
```
Expected: `tsc` reports no errors, `vite build` succeeds, `apps/knx-desktop/dist/` is produced.

- [ ] **Step 5: Commit**

```bash
git add apps/knx-desktop/package.json apps/knx-desktop/package-lock.json \
  apps/knx-desktop/tsconfig.json apps/knx-desktop/vite.config.ts \
  apps/knx-desktop/index.html apps/knx-desktop/src apps/knx-desktop/.gitignore
git commit -m "feat(desktop): frontend scaffold (Vite + React + TypeScript)"
```

---

### Task 7: Project Explorer component

**Files:**
- Create: `apps/knx-desktop/src/ProjectExplorer.tsx`
- Modify: `apps/knx-desktop/src/App.tsx`
- Modify: `apps/knx-desktop/src/styles.css`

**Interfaces:**
- Consumes: `ProjectTree`, `InstallationNode`, `AreaNode`, `LineNode`, `BuildingNode`, `DeviceNode` from `./bindings/*` (Task 2); `App`'s `pickProject`/`tree`/`error` state (Task 6).
- Produces: `export default function ProjectExplorer(props: { tree: ProjectTree }): JSX.Element` — no other component in this cycle needs to consume it.

- [ ] **Step 1: Write the component**

`apps/knx-desktop/src/ProjectExplorer.tsx`:

```typescript
import { useState } from "react";
import type { ProjectTree } from "./bindings/ProjectTree";
import type { InstallationNode } from "./bindings/InstallationNode";
import type { AreaNode } from "./bindings/AreaNode";
import type { LineNode } from "./bindings/LineNode";
import type { BuildingNode } from "./bindings/BuildingNode";
import type { DeviceNode } from "./bindings/DeviceNode";

function TreeNode(props: { label: string; children?: React.ReactNode }) {
  const [open, setOpen] = useState(true);
  const hasChildren = props.children !== undefined;
  return (
    <li>
      <span
        className={hasChildren ? "tree-label expandable" : "tree-label"}
        onClick={hasChildren ? () => setOpen(!open) : undefined}
      >
        {hasChildren ? (open ? "▾ " : "▸ ") : ""}
        {props.label}
      </span>
      {hasChildren && open && <ul>{props.children}</ul>}
    </li>
  );
}

function DeviceItem(props: { device: DeviceNode }) {
  const { device } = props;
  const label = device.address ? `${device.address} ${device.name}` : device.name;
  return <TreeNode label={label} />;
}

function LineItem(props: { line: LineNode }) {
  const { line } = props;
  return (
    <TreeNode label={`Line ${line.address}: ${line.name}`}>
      {line.devices.map((d) => (
        <DeviceItem key={d.id} device={d} />
      ))}
    </TreeNode>
  );
}

function AreaItem(props: { area: AreaNode }) {
  const { area } = props;
  return (
    <TreeNode label={`Area ${area.address}: ${area.name}`}>
      {area.lines.map((l) => (
        <LineItem key={l.id} line={l} />
      ))}
    </TreeNode>
  );
}

function BuildingItem(props: { building: BuildingNode }) {
  const { building } = props;
  return (
    <TreeNode label={`${building.name} (${building.kind})`}>
      {building.children.map((c) => (
        <BuildingItem key={c.id} building={c} />
      ))}
      {building.devices.map((d) => (
        <DeviceItem key={d.id} device={d} />
      ))}
    </TreeNode>
  );
}

function InstallationItem(props: { installation: InstallationNode }) {
  const { installation } = props;
  return (
    <TreeNode label={installation.name}>
      <TreeNode label="Topology">
        {installation.topology.map((a) => (
          <AreaItem key={a.id} area={a} />
        ))}
      </TreeNode>
      <TreeNode label="Buildings">
        {installation.buildings.map((b) => (
          <BuildingItem key={b.id} building={b} />
        ))}
      </TreeNode>
      {installation.unassigned.length > 0 && (
        <TreeNode label="Unassigned">
          {installation.unassigned.map((d) => (
            <DeviceItem key={d.id} device={d} />
          ))}
        </TreeNode>
      )}
    </TreeNode>
  );
}

export default function ProjectExplorer(props: { tree: ProjectTree }) {
  const { tree } = props;
  return (
    <div className="project-explorer">
      <ul className="tree-root">
        {tree.installations.map((inst) => (
          <InstallationItem key={inst.id} installation={inst} />
        ))}
      </ul>
      {tree.warnings > 0 && (
        <footer>{tree.warnings} import warning{tree.warnings === 1 ? "" : "s"}</footer>
      )}
    </div>
  );
}
```

- [ ] **Step 2: Wire it into App**

Replace `apps/knx-desktop/src/App.tsx`'s body with:

```typescript
import { useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { invoke } from "@tauri-apps/api/core";
import type { ProjectTree } from "./bindings/ProjectTree";
import ProjectExplorer from "./ProjectExplorer";

function App() {
  const [tree, setTree] = useState<ProjectTree | null>(null);
  const [error, setError] = useState<string | null>(null);

  async function pickProject() {
    const path = await open({
      multiple: false,
      filters: [{ name: "ETS project", extensions: ["knxproj"] }],
    });
    if (typeof path !== "string") return;
    setError(null);
    try {
      setTree(await invoke<ProjectTree>("open_project", { path }));
    } catch (e) {
      setError(String(e));
    }
  }

  return (
    <main>
      <button onClick={pickProject}>Open project…</button>
      {error && (
        <p role="alert" className="error-banner">
          {error}
        </p>
      )}
      {tree && <ProjectExplorer tree={tree} />}
    </main>
  );
}

export default App;
```

- [ ] **Step 3: Minimal tree styling**

Append to `apps/knx-desktop/src/styles.css`:

```css
.error-banner {
  color: #b00020;
  border: 1px solid #b00020;
  padding: 0.5rem;
  border-radius: 4px;
}

.project-explorer ul {
  list-style: none;
  padding-left: 1.25rem;
}

.project-explorer .tree-root {
  padding-left: 0;
}

.tree-label {
  cursor: default;
}

.tree-label.expandable {
  cursor: pointer;
}

.project-explorer footer {
  margin-top: 1rem;
  font-style: italic;
  opacity: 0.8;
}
```

- [ ] **Step 4: Build to verify**

```bash
cd apps/knx-desktop && npm run build
```
Expected: no `tsc` errors, `vite build` succeeds.

- [ ] **Step 5: Manual verification**

Run the app (see the `run` skill, or directly):
```bash
cargo install tauri-cli --version "^2" --locked   # once, if not already installed
cd apps/knx-desktop/src-tauri && cargo tauri dev
```
Click "Open project…", pick `Unser Zuhause ets4 - 2025-12-15.knxproj` from
the workspace root, and confirm the Project Explorer renders both a
Topology and a Buildings subtree, expandable/collapsible, with the
reference project's devices visible under their line and under their room.
This step has no automated check this cycle (spec: no frontend test
framework yet) — record in the task's completion note that it was done.

- [ ] **Step 6: Commit**

```bash
git add apps/knx-desktop/src/ProjectExplorer.tsx apps/knx-desktop/src/App.tsx \
  apps/knx-desktop/src/styles.css
git commit -m "feat(desktop): Project Explorer tree view"
```

---

### Task 8: CI — Node, Tauri Linux prerequisites, ts-rs binding staleness

**Files:**
- Modify: `.github/workflows/ci.yml`

**Interfaces:** none — CI configuration only.

- [ ] **Step 1: Add the Tauri Linux system dependencies and Node setup**

In `.github/workflows/ci.yml`, in the `build` job, insert right after the
`actions/checkout@v4` step and before `Install Rust toolchain`:

```yaml
      - name: Install Tauri Linux prerequisites
        run: |
          sudo apt-get update
          sudo apt-get install -y \
            libwebkit2gtk-4.1-dev \
            libgtk-3-dev \
            libayatana-appindicator3-dev \
            librsvg2-dev

      - name: Install Node.js
        uses: actions/setup-node@v4
        with:
          node-version: "22"
```

(These are needed because `knx-desktop`, once in the workspace, is built by
every `cargo build`/`test`/`clippy` step below — `tauri`'s Linux backend
links against webkit2gtk/GTK3 even when only compiling the library, not
running it.)

- [ ] **Step 2: Add the frontend build step**

After the existing `Test` step (`cargo test --workspace`), before
`Layering gate`:

```yaml
      - name: Frontend build
        run: |
          cd apps/knx-desktop
          npm ci
          npm run build
```

- [ ] **Step 3: Add the ts-rs binding staleness check**

After `Frontend build`, still before `Layering gate`:

```yaml
      - name: ts-rs bindings up to date
        run: |
          TS_RS_EXPORT_DIR=../../apps/knx-desktop/src/bindings cargo test -p knx-projection
          git diff --exit-code -- apps/knx-desktop/src/bindings
```

- [ ] **Step 4: Verify the full job locally where possible**

The apt-get step cannot be run outside a Debian/Ubuntu CI image; on this
development machine (Arch-based) the equivalent libraries are already
present under different package names, so the Rust build/test/clippy
commands can still be verified directly (see Tasks 1–4's own steps). The
`npm ci && npm run build` and `git diff --exit-code` lines can be run as
written from the repository root:

```bash
(cd apps/knx-desktop && npm ci && npm run build)
TS_RS_EXPORT_DIR=../../apps/knx-desktop/src/bindings cargo test -p knx-projection
git diff --exit-code -- apps/knx-desktop/src/bindings
```
Expected: both succeed and the second reports no diff (the bindings
committed in Task 2 already match what regeneration produces, since
nothing in the projection types changed since then).

- [ ] **Step 5: Commit**

```bash
git add .github/workflows/ci.yml
git commit -m "ci: Tauri Linux prerequisites, frontend build, ts-rs staleness check"
```

---

### Task 9: Documentation — implementation status and roadmap

**Files:**
- Modify: `docs/IMPLEMENTATION_STATUS.md`
- Modify: `docs/ROADMAP.md`

**Interfaces:** none — documentation only.

- [ ] **Step 1: Update `IMPLEMENTATION_STATUS.md`**

Change the Session 5 row in the status table from "Not started" to
"**In progress** — cycle 1 (shell, projection, Project Explorer) done, see
[ROADMAP.md](ROADMAP.md)". Add a new paragraph, in the same style as the
existing per-crate paragraphs, covering: `knx-projection` (pure
`Project` → `ProjectTree` projection, ts-rs bindings, depends only on
`knx-core`, a fourth `check-layering` root); `apps/knx-desktop` (Tauri v2 +
React + Vite, the `open_project` command, in-memory store connection since
this cycle does not persist); the updated total test count (run
`cargo test --workspace 2>&1 | tail -5`-equivalent count, or sum the
per-crate counts, and state the new total explicitly — do not leave the old
238 figure standing unchanged next to new tests). Add the new "What
exists" table rows for `crates/knx-projection/` and
`apps/knx-desktop/`.

- [ ] **Step 2: Update `ROADMAP.md`**

In the Session 5 section, add a line after the **Deliverables** paragraph:
"Cycle 1 (this document's own scope split, see
`docs/superpowers/specs/2026-09-03-knx-desktop-shell-design.md`) delivered
the shell, the projection layer, and Project Explorer. Inspector, search,
command palette and dark/light mode are later cycles of this same
session, not yet scheduled."

- [ ] **Step 3: Commit**

```bash
git add docs/IMPLEMENTATION_STATUS.md docs/ROADMAP.md
git commit -m "docs(session5): record cycle 1 (desktop shell, projection, Project Explorer)"
```
