# Device create/delete commands (T1/T3 backend) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Give `knx-core` a `CreateDevice`/`DeleteDevice` command pair (with undo/redo), let `knx-productdb` back device creation with a catalog lookup and one-shot enrichment, and wire four new `apps/knx-server` routes so a device can be inserted from the product-database catalog and deleted again — backend only, no frontend caller yet (same "backend now, UI later" shape as the 2026-09-06 topology/group-range/group-link command layer).

**Architecture:** `Command::CreateDevice`/`DeleteDevice` are exact inverses of each other in `knx-core::command` (DeleteDevice's inverse is a `CreateDevice` rebuilt from whatever it just removed, so redo restores post-enrichment state, not creation-time state). `knx-productdb::query` gains three read-only catalog queries backing the future catalog browser (T2) and, this slice, device creation's own lookup; `knx-productdb::enrich::apply` becomes `pub` so device creation reuses it directly instead of duplicating its DPT/text/flags mapping. `apps/knx-server::domain` gains `AppState.product_db`, a `create_device_impl`/`delete_device_impl` pair, and (bonus fix, same root cause) actually wires that connection through project import for the first time.

**Tech Stack:** Rust workspace; `rusqlite` (knx-productdb), `axum`/`tokio` (knx-server), `cargo test` per crate.

**Spec:** [docs/superpowers/specs/2026-09-07-device-create-delete-design.md](../specs/2026-09-07-device-create-delete-design.md) — read it alongside this plan; this plan does not repeat its rationale, only the exact code.

## Global Constraints

- Device deletion refuses (does not cascade) when the device still has linked communication objects — matches `DeleteArea`/`DeleteLine`/`DeleteGroupAddress`'s existing convention, the only one this codebase has ever used for a delete with dependents.
- `AppState`'s product database lookup is never a startup error: any failure to derive a path or open/migrate the file leaves `AppState.product_db` as `None` (ADR-0012's "missing product database is ordinary, not an error").
- No frontend caller for any of the four new routes in this slice — that is a separate, future cycle.
- No co-author line in commits; commit as `github@knxbench.com` per `CLAUDE.md` (the repo's own git rule, not this session's attribution footer, which still applies to this plan document's own authorship metadata if any tooling adds it).
- Keep the repository buildable after every task: run the affected crate's tests (and `cargo build --workspace` at the end of the whole plan) before moving on.

---

### Task 1: `knx-core` command layer — `CreateDevice`/`DeleteDevice`

**Files:**
- Modify: `crates/knx-core/src/command.rs`
- Modify: `crates/knx-core/src/devices.rs`

**Interfaces:**
- Consumes: `knx_core::device::{ComObjectInstance, DeviceInstance}`, `knx_core::devices::Devices` (`insert`, `insert_com_object`, `get`, `get_mut`, `remove`, `com_object`, `com_object_mut`), `knx_core::installation::Installation`, `knx_core::topology::Line`, `knx_core::ids::{DeviceId, ComObjectInstanceId, LineId}` — all already exist.
- Produces: `Command::CreateDevice { device: DeviceInstance, com_objects: Vec<ComObjectInstance>, line: Option<LineId> }`, `Command::DeleteDevice { id: DeviceId }`, `CommandError::DeviceHasLinks(DeviceId)`, `Devices::remove_com_object(id: ComObjectInstanceId) -> Option<ComObjectInstance>` — every later task in this plan calls these exact names.

- [ ] **Step 1: Add `Devices::remove_com_object`**

In `crates/knx-core/src/devices.rs`, right after `insert_com_object`:

```rust
    pub fn remove_com_object(&mut self, id: ComObjectInstanceId) -> Option<ComObjectInstance> {
        self.com_objects.remove(&id)
    }
```

- [ ] **Step 2: Add imports `command.rs` needs**

In `crates/knx-core/src/command.rs`, change:

```rust
use crate::device::ComObjectInstance;
```

to:

```rust
use crate::device::{ComObjectInstance, DeviceInstance};
```

and add, near the other `crate::` imports:

```rust
use crate::installation::Installation;
```

- [ ] **Step 3: Add the two `Command` variants**

In `crates/knx-core/src/command.rs`, in the `Command` enum, insert right after `MoveDeviceToLine` and before `CreateGroupRange`:

```rust
    /// Creates a device with its communication-object instances already
    /// attached, placed in `line` or, if `None`, `Topology::unassigned` —
    /// mirrors `MoveDeviceToLine`'s own placement rule, since a device is
    /// placed in a line xor left unassigned the same way in both commands.
    /// `device.id` and every entry in `com_objects` carry ids
    /// pre-allocated by the caller via `Project::ids::next_device_id`/
    /// `next_com_object_instance_id`; `device.com_objects` already lists
    /// their ids, so no separate id list is threaded through twice.
    CreateDevice {
        device: DeviceInstance,
        com_objects: Vec<ComObjectInstance>,
        line: Option<LineId>,
    },
    /// Refuses (`CommandError::DeviceHasLinks`) if any of the device's
    /// communication objects still links to a group address — the
    /// device equivalent of `DeleteGroupAddress`'s `GroupAddressInUse`
    /// check.
    DeleteDevice {
        id: DeviceId,
    },
```

- [ ] **Step 4: Add the `CommandError::DeviceHasLinks` variant**

In `crates/knx-core/src/command.rs`'s `CommandError` enum, insert right after `LineNotEmpty(LineId)`'s doc comment and field (keep it near the other "delete refused because still in use" variants — exact position doesn't matter, but group it with its siblings for readability):

```rust
    /// A `DeleteDevice` was refused because at least one of the device's
    /// communication object instances still links to a group address —
    /// deleting it now would leave a dangling `GroupLink`, the device
    /// equivalent of `GroupAddressInUse`.
    DeviceHasLinks(DeviceId),
```

And in `impl fmt::Display for CommandError`, add the matching arm (next to `LineNotEmpty`'s arm):

```rust
            CommandError::DeviceHasLinks(id) => {
                write!(f, "device {id} still has linked communication objects, cannot delete")
            }
```

- [ ] **Step 5: Factor `MoveDeviceToLine`'s device-lookup into a shared helper**

`DeleteDevice` needs to find and remove a device from wherever it sits in the topology, exactly like `MoveDeviceToLine` already does. Add this free function in `crates/knx-core/src/command.rs`, right before `impl Command {`:

```rust
/// Removes `device` from wherever it currently sits in `installation`'s
/// topology — `unassigned` or a line's `devices` — returning the line it
/// was in, if any. Shared by `MoveDeviceToLine` (which repositions the
/// device elsewhere) and `DeleteDevice` (which needs the same lookup to
/// know what line its own inverse `CreateDevice` should name).
fn remove_device_from_topology(
    installation: &mut Installation,
    device: DeviceId,
) -> Result<Option<LineId>, CommandError> {
    if let Some(pos) = installation
        .topology
        .unassigned
        .iter()
        .position(|&d| d == device)
    {
        installation.topology.unassigned.remove(pos);
        Ok(None)
    } else if let Some(current_line) = installation
        .topology
        .lines
        .iter_mut()
        .find(|l| l.devices.contains(&device))
    {
        let id = current_line.id;
        current_line.devices.retain(|&d| d != device);
        Ok(Some(id))
    } else {
        Err(CommandError::DeviceNotFound(device))
    }
}
```

Now replace `Command::MoveDeviceToLine`'s `apply` arm body with the version that calls it. Find:

```rust
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
                let previous = if let Some(pos) = installation
                    .topology
                    .unassigned
                    .iter()
                    .position(|&d| d == device)
                {
                    installation.topology.unassigned.remove(pos);
                    None
                } else if let Some(current_line) = installation
                    .topology
                    .lines
                    .iter_mut()
                    .find(|l| l.devices.contains(&device))
                {
                    let id = current_line.id;
                    current_line.devices.retain(|&d| d != device);
                    Some(id)
                } else {
                    return Err(CommandError::DeviceNotFound(device));
                };
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
```

Replace with:

```rust
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
                let previous = remove_device_from_topology(installation, device)?;
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
```

This is a pure refactor — every existing `MoveDeviceToLine` test must still pass unchanged after this step.

- [ ] **Step 6: Run the existing `MoveDeviceToLine` tests to confirm the refactor changed nothing**

Run: `cargo test -p knx-core move_device`
Expected: PASS (same tests as before, `move_device_from_unassigned_to_a_line_and_back_via_undo`, `move_device_between_two_lines`, `move_device_to_a_nonexistent_line_is_rejected_and_leaves_the_device_in_place`, `move_an_unknown_device_is_rejected`)

- [ ] **Step 7: Add `Command::CreateDevice`'s `apply` arm**

In `crates/knx-core/src/command.rs`'s `Command::apply` match, insert right after the `MoveDeviceToLine` arm and before the `CreateGroupRange` arm:

```rust
            Command::CreateDevice {
                device,
                com_objects,
                line,
            } => {
                let device = device.clone();
                let com_objects = com_objects.clone();
                let line = *line;
                let device_id = device.id;
                let installation = project
                    .installations
                    .first_mut()
                    .ok_or(CommandError::InstallationNotFound)?;
                if let Some(line_id) = line {
                    if !installation.topology.lines.iter().any(|l| l.id == line_id) {
                        return Err(CommandError::LineNotFound(line_id));
                    }
                }
                for com in &com_objects {
                    project.devices.insert_com_object(com.clone());
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
```

(Holding `installation` — a mutable borrow of `project.installations[0]` — across the `project.devices...` calls compiles: they are disjoint fields of `Project`, so the borrow checker allows interleaving them. If your toolchain somehow disagrees, re-fetch with `let installation = project.installations.first_mut().unwrap();` right before the `match line` block — `installations` is already known non-empty from the check above.)

- [ ] **Step 8: Add `Command::DeleteDevice`'s `apply` arm**

Right after the `CreateDevice` arm you just added:

```rust
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
                let installation = project
                    .installations
                    .first_mut()
                    .ok_or(CommandError::InstallationNotFound)?;
                let line = remove_device_from_topology(installation, id)?;
                let device = project.devices.remove(id).unwrap();
                let com_objects = device
                    .com_objects
                    .iter()
                    .filter_map(|&com_id| project.devices.remove_com_object(com_id))
                    .collect();
                Ok(Command::CreateDevice {
                    device,
                    com_objects,
                    line,
                })
            }
```

- [ ] **Step 9: Write the failing tests**

Add these to `crates/knx-core/src/command.rs`'s `#[cfg(test)] mod tests`, anywhere after `test_project_with_one_device` (they use `source()`, `test_project_with_one_device`, `test_line`, `project_with_line_and_unassigned_device`, all already defined in that module):

```rust
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
                    line: None,
                },
            )
            .unwrap();
        assert!(project.devices.get(DeviceId(2)).is_some());
        assert!(project.devices.com_object(ComObjectInstanceId(10)).is_some());
        assert_eq!(
            project.installations[0].topology.unassigned,
            vec![DeviceId(2)]
        );
        stack.undo(&mut project).unwrap();
        assert!(project.devices.get(DeviceId(2)).is_none());
        assert!(project.devices.com_object(ComObjectInstanceId(10)).is_none());
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
        assert!(project.devices.com_object(ComObjectInstanceId(10)).is_none());

        stack.undo(&mut project).unwrap(); // undoes the delete -> recreates
        let restored = project
            .devices
            .com_object(ComObjectInstanceId(10))
            .unwrap();
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
        let restored_again = project
            .devices
            .com_object(ComObjectInstanceId(10))
            .unwrap();
        assert_eq!(
            restored_again.dpt.value().unwrap().layer,
            Layer::Program,
            "the enriched value must survive a delete/undo/redo/undo cycle unchanged"
        );
    }
```

- [ ] **Step 10: Run the tests to verify they pass**

Run: `cargo test -p knx-core command::`
Expected: PASS — all four new tests, plus every pre-existing `command.rs` test still green (proves Step 5's refactor broke nothing).

- [ ] **Step 11: Run the whole crate's test suite and clippy**

Run: `cargo test -p knx-core && cargo clippy -p knx-core --all-targets -- -D warnings`
Expected: PASS. (Note: `cargo clippy --workspace --all-targets -- -D warnings` is documented as already broken on `main` for unrelated reasons — `crates/knx-etsproj`'s `large_enum_variant` lint, see `IMPLEMENTATION_STATUS.md`'s 2026-09-07 entry. Run clippy scoped to `-p knx-core` here, not workspace-wide, so that pre-existing failure doesn't block this task.)

- [ ] **Step 12: Commit**

```bash
git add crates/knx-core/src/command.rs crates/knx-core/src/devices.rs
git commit -m "feat(knx-core): CreateDevice/DeleteDevice commands with undo/redo"
```

---

### Task 2: `knx-productdb` catalog queries + `enrich::apply` made `pub`

**Files:**
- Modify: `crates/knx-productdb/src/query.rs`
- Modify: `crates/knx-productdb/src/enrich.rs`

**Interfaces:**
- Consumes: existing `catalog_item` SQLite table (`crates/knx-productdb/src/migration.rs`, columns `id, manufacturer_id, section_id, name, number, visible_description, product_ref_id, hardware2program_ref_id, default_language, source_sha256`), existing `com_object_ref` table, existing `crate::parse::catalog::ingest_catalog`/`crate::parse::hardware::ingest_hardware`/`crate::parse::program::ingest_program` (test fixtures only).
- Produces: `pub struct CatalogItemRow { id, manufacturer_id, name: Option<String>, number: Option<String>, visible_description: Option<String>, product_ref_id: Option<String>, hardware2program_ref_id: Option<String> }`, `pub fn catalog_items(conn, manufacturer: Option<&str>, search: Option<&str>) -> Result<Vec<CatalogItemRow>, ProductDbError>`, `pub fn catalog_item(conn, id: &str) -> Result<Option<CatalogItemRow>, ProductDbError>`, `pub fn com_object_ref_ids(conn, program_id: &str) -> Result<Vec<String>, ProductDbError>`, and `enrich::apply` becoming `pub fn apply(project: &mut Project, com_id: ComObjectInstanceId, ref_id: &str, view: &ComObjectView, issues: &mut Vec<EnrichmentIssue>) -> bool` — Task 4 calls all four of these by exact name.

- [ ] **Step 1: Write the failing tests for the three new query functions**

Add to `crates/knx-productdb/src/query.rs`'s `#[cfg(test)] mod tests`, after the existing `use` lines at the top of that module (add one more `use`):

```rust
    use crate::parse::catalog::ingest_catalog;
```

Then add this fixture constant and the three tests, anywhere in the test module after `db()`:

```rust
    const CATALOG: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<KNX xmlns="http://knx.org/xml/project/11">
  <ManufacturerData>
    <Manufacturer RefId="M-006A">
      <Catalog>
        <CatalogSection Id="M-006A_CG-1" Name="Actuators" Number="1" DefaultLanguage="de-DE">
          <CatalogItem Id="M-006A_CI-1" Name="Schaltaktor" Number="EM12102"
                       DefaultLanguage="de-DE"
                       ProductRefId="M-006A_H-1_P-1"
                       Hardware2ProgramRefId="H-1_HP-1" />
        </CatalogSection>
      </Catalog>
    </Manufacturer>
  </ManufacturerData>
</KNX>"#;

    #[test]
    fn catalog_items_lists_and_filters_by_manufacturer_and_search() {
        let (_dir, conn) = db();
        ingest_catalog(&conn, "sha-c", "M-006A/Catalog.xml", CATALOG.as_bytes()).unwrap();

        let all = catalog_items(&conn, None, None).unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].id, "M-006A_CI-1");

        assert_eq!(catalog_items(&conn, Some("M-006A"), None).unwrap().len(), 1);
        assert_eq!(catalog_items(&conn, Some("M-999X"), None).unwrap().len(), 0);
        assert_eq!(
            catalog_items(&conn, None, Some("schalt")).unwrap().len(),
            1,
            "search is case-insensitive"
        );
        assert_eq!(
            catalog_items(&conn, None, Some("EM12102")).unwrap().len(),
            1,
            "search also matches on number"
        );
        assert_eq!(catalog_items(&conn, None, Some("nope")).unwrap().len(), 0);
    }

    #[test]
    fn catalog_item_looks_up_a_single_row_by_id() {
        let (_dir, conn) = db();
        ingest_catalog(&conn, "sha-c", "M-006A/Catalog.xml", CATALOG.as_bytes()).unwrap();

        let item = catalog_item(&conn, "M-006A_CI-1").unwrap().unwrap();
        assert_eq!(item.manufacturer_id, "M-006A");
        assert_eq!(item.hardware2program_ref_id.as_deref(), Some("H-1_HP-1"));
        assert!(catalog_item(&conn, "nope").unwrap().is_none());
    }

    #[test]
    fn com_object_ref_ids_returns_every_ref_in_document_order() {
        let (_dir, conn) = db();
        let ids = com_object_ref_ids(&conn, "A-1").unwrap();
        assert_eq!(ids, vec!["A-1_O-1_R-1".to_string(), "A-1_O-1_R-2".to_string()]);
    }
```

- [ ] **Step 2: Run the tests to verify they fail to compile**

Run: `cargo test -p knx-productdb catalog_items -- --list`
Expected: compile error — `catalog_items`/`catalog_item`/`com_object_ref_ids` not found.

- [ ] **Step 3: Implement the three query functions**

In `crates/knx-productdb/src/query.rs`, add after `programs`'s closing brace and before the `#[cfg(test)]` module:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CatalogItemRow {
    pub id: String,
    pub manufacturer_id: String,
    pub name: Option<String>,
    pub number: Option<String>,
    pub visible_description: Option<String>,
    pub product_ref_id: Option<String>,
    pub hardware2program_ref_id: Option<String>,
}

fn row_to_catalog_item(r: &rusqlite::Row) -> rusqlite::Result<CatalogItemRow> {
    Ok(CatalogItemRow {
        id: r.get(0)?,
        manufacturer_id: r.get(1)?,
        name: r.get(2)?,
        number: r.get(3)?,
        visible_description: r.get(4)?,
        product_ref_id: r.get(5)?,
        hardware2program_ref_id: r.get(6)?,
    })
}

const CATALOG_ITEM_COLUMNS: &str =
    "id, manufacturer_id, name, number, visible_description, product_ref_id, hardware2program_ref_id";

/// Every `catalog_item` row, optionally narrowed to one manufacturer and/or
/// a case-insensitive substring match on `name`/`number` — backs the future
/// catalog browser (T2). Device creation (`apps/knx-server`) goes straight
/// to `catalog_item` by id instead: nothing yet picks an id through this
/// listing.
pub fn catalog_items(
    conn: &Connection,
    manufacturer: Option<&str>,
    search: Option<&str>,
) -> Result<Vec<CatalogItemRow>, ProductDbError> {
    let sql = format!(
        "SELECT {CATALOG_ITEM_COLUMNS}
         FROM catalog_item
         WHERE (?1 IS NULL OR manufacturer_id = ?1)
           AND (?2 IS NULL
                OR LOWER(name) LIKE '%' || LOWER(?2) || '%'
                OR LOWER(number) LIKE '%' || LOWER(?2) || '%')
         ORDER BY manufacturer_id, name"
    );
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt
        .query_map([manufacturer, search], row_to_catalog_item)?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

/// The single-row lookup `apps/knx-server`'s device creation uses.
pub fn catalog_item(conn: &Connection, id: &str) -> Result<Option<CatalogItemRow>, ProductDbError> {
    let sql = format!("SELECT {CATALOG_ITEM_COLUMNS} FROM catalog_item WHERE id = ?1");
    conn.query_row(&sql, [id], row_to_catalog_item)
        .optional()
        .map_err(Into::into)
}

/// Every `com_object_ref.id` for `program_id`, in document/ingest order.
/// `ORDER BY rowid` rather than `ORDER BY id`: `com_object_ref` is not
/// declared `WITHOUT ROWID`, so `rowid` preserves insertion order, and the
/// ids themselves (`A-1_O-1_R-1`, `A-1_O-1_R-10`, `A-1_O-1_R-2`, …) do not
/// sort into that order lexically.
pub fn com_object_ref_ids(conn: &Connection, program_id: &str) -> Result<Vec<String>, ProductDbError> {
    let mut stmt =
        conn.prepare("SELECT id FROM com_object_ref WHERE program_id = ?1 ORDER BY rowid")?;
    let rows = stmt
        .query_map([program_id], |r| r.get(0))?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p knx-productdb catalog_items -- catalog_item -- com_object_ref_ids`

(If your shell mangles that multi-pattern form, run the three tests individually: `cargo test -p knx-productdb catalog_items_lists_and_filters_by_manufacturer_and_search`, `cargo test -p knx-productdb catalog_item_looks_up_a_single_row_by_id`, `cargo test -p knx-productdb com_object_ref_ids_returns_every_ref_in_document_order`.)

Expected: PASS.

- [ ] **Step 5: Make `enrich::apply` `pub` and add a standalone test**

In `crates/knx-productdb/src/enrich.rs`, change:

```rust
fn apply(
```

to:

```rust
/// Fills every `Override::Absent` slot on the communication object
/// instance named by `com_id` from `view`, exactly as `enrich()`'s own
/// per-project loop does for each device it resolves. Exposed as `pub` so
/// a caller seeding a single newly created communication object (device
/// creation, `apps/knx-server::domain::create_device_impl`) can reuse this
/// mapping directly instead of duplicating it.
pub fn apply(
```

Then add this test to `crates/knx-productdb/src/enrich.rs`'s `#[cfg(test)] mod tests`, after `an_absent_datapoint_type_is_filled_and_a_size_is_set`:

```rust
    #[test]
    fn apply_can_be_called_directly_without_going_through_enrich() {
        let (_dir, conn) = db();
        let view = com_object_view(&conn, "A-1", "A-1_O-1_R-1").unwrap().unwrap();
        let mut p = project_with("A-1_O-1_R-1", Override::Absent);
        let mut issues = Vec::new();
        let changed = apply(
            &mut p,
            knx_core::ComObjectInstanceId(1),
            "A-1_O-1_R-1",
            &view,
            &mut issues,
        );
        assert!(changed);
        assert!(issues.is_empty());
        let com = p
            .devices
            .com_object(knx_core::ComObjectInstanceId(1))
            .unwrap();
        assert!(com.dpt.value().is_some());
    }
```

- [ ] **Step 6: Run the test to verify it passes**

Run: `cargo test -p knx-productdb apply_can_be_called_directly_without_going_through_enrich`
Expected: PASS.

- [ ] **Step 7: Run the whole crate's test suite and clippy**

Run: `cargo test -p knx-productdb && cargo clippy -p knx-productdb --all-targets -- -D warnings`
Expected: PASS.

- [ ] **Step 8: Commit**

```bash
git add crates/knx-productdb/src/query.rs crates/knx-productdb/src/enrich.rs
git commit -m "feat(knx-productdb): catalog_items/catalog_item/com_object_ref_ids, pub enrich::apply"
```

---

### Task 3: `apps/knx-server` — `AppState.product_db` + the import wiring bonus fix

**Files:**
- Modify: `apps/knx-server/Cargo.toml`
- Modify: `apps/knx-server/src/domain.rs`

**Interfaces:**
- Consumes: `knx_productdb::{default_path, open_and_migrate, Connection}` (all pre-existing), `knx_app::{ImportOptions, import_ets_project_with}` (pre-existing).
- Produces: `AppState.product_db: Option<Mutex<knx_productdb::Connection>>` (public field, same visibility as every other `AppState` field), `import_and_project(path: &Path, product_db: Option<&knx_productdb::Connection>) -> Result<(ProjectTree, knx_core::Project), AppError>` (new second parameter — Task 4 does not call this directly, but Task 4's `create_device_impl`/`delete_device_impl` rely on `AppState.product_db` existing).

- [ ] **Step 1: Add the `knx-productdb` dependency**

In `apps/knx-server/Cargo.toml`, add to `[dependencies]` (alphabetically, after `knx-projection.workspace = true`):

```toml
knx-productdb.workspace = true
```

- [ ] **Step 2: Write the failing test for the wiring**

Add a `#[cfg(test)] mod tests` block at the end of `apps/knx-server/src/domain.rs` (there isn't one yet):

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn workspace_root() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .expect("crate lives at <root>/apps/knx-server")
            .to_path_buf()
    }

    fn reference_project_path() -> PathBuf {
        workspace_root().join("Unser Zuhause ets4 - 2025-12-15.knxproj")
    }

    #[test]
    fn opening_a_project_through_a_wired_product_db_enriches_more_than_without() {
        let dir = tempfile::tempdir().unwrap();
        let products_path = dir.path().join("products.sqlite");
        {
            // Ingest the reference project's own manufacturer files into a
            // fresh product database — same two-step dance
            // `crates/knx-app/tests/product_db.rs` already uses.
            let throwaway = knx_store::open_and_migrate_in_memory().unwrap();
            let products = knx_productdb::open_and_migrate(&products_path).unwrap();
            knx_app::import_ets_project_with(
                &reference_project_path(),
                &throwaway,
                ImportOptions {
                    product_db: Some(&products),
                },
            )
            .unwrap();
        }

        let (_, without) = import_and_project(&reference_project_path(), None).unwrap();
        let without_filled = without
            .devices
            .com_objects()
            .filter(|c| c.dpt.value().is_some())
            .count();

        let products = knx_productdb::open_and_migrate(&products_path).unwrap();
        let mut state = AppState::default();
        state.product_db = Some(Mutex::new(products));
        open_project(&state, &reference_project_path()).unwrap();
        let project = state.project.lock().unwrap();
        let project = project.as_ref().unwrap();
        let with_filled = project
            .devices
            .com_objects()
            .filter(|c| c.dpt.value().is_some())
            .count();

        assert!(
            with_filled > without_filled,
            "wiring state.product_db through open_project should enrich at least \
             one more com object's dpt ({with_filled} vs {without_filled})"
        );
    }
}
```

- [ ] **Step 3: Run the test to verify it fails to compile**

Run: `cargo test -p knx-server --lib opening_a_project_through_a_wired_product_db -- --list`
Expected: compile error — `AppState` has no field `product_db`, and `import_and_project` takes one argument, not two.

- [ ] **Step 4: Add the `product_db` field and wire it through**

In `apps/knx-server/src/domain.rs`, add to the `AppState` struct, after `import_counts`:

```rust
    /// The shared product database, opened once at startup from
    /// `knx_productdb::default_path()`. `None` if no path could be
    /// derived, the file doesn't exist yet, or it failed to open/migrate
    /// — never a startup error (ADR-0012's "missing product database is
    /// ordinary, not an error"). `Mutex`, not `RwLock`: every access here
    /// is a handful of `SELECT`s or one `enrich()` pass, never held long
    /// enough for reader/writer contention to matter.
    pub product_db: Option<Mutex<knx_productdb::Connection>>,
```

Update `AppState::new`:

```rust
impl AppState {
    pub fn new(data_dir: PathBuf) -> Self {
        let product_db = knx_productdb::default_path()
            .and_then(|path| knx_productdb::open_and_migrate(&path).ok())
            .map(Mutex::new);
        Self {
            project: Mutex::new(None),
            store_path: Mutex::new(None),
            command_stack: Mutex::new(knx_core::CommandStack::new()),
            import_counts: Mutex::new((0, 0)),
            product_db,
            data_dir,
        }
    }
}
```

Change `import_and_project`, `open_project_impl` and `open_project`:

```rust
/// Shared by `open_project_impl` (display-only) and `open_project`
/// (display + replaces `state`'s project) so there is exactly one import
/// implementation instead of two. `product_db` is locked by the caller —
/// this function only borrows it for the duration of the import call.
fn import_and_project(
    path: &Path,
    product_db: Option<&knx_productdb::Connection>,
) -> Result<(ProjectTree, knx_core::Project), AppError> {
    let conn = knx_store::open_and_migrate_in_memory()?;
    let imported = knx_app::import_ets_project_with(path, &conn, ImportOptions { product_db })?;
    let mut tree = knx_projection::build_project_tree(&imported.project);
    apply_report_counts(&mut tree, &imported.report);
    Ok((tree, imported.project))
}

/// Imports `path` and projects it without touching `state` — what
/// `open_reference_project.rs` exercises directly, no server needed. Never
/// enriched from a product database: there is no `state` here to read one
/// from, and this path exists specifically for a server-free golden test
/// whose counts must stay deterministic.
pub fn open_project_impl(path: &Path) -> Result<ProjectTree, AppError> {
    import_and_project(path, None).map(|(tree, _)| tree)
}

/// Imports `path`, replaces `state`'s project, and resets undo history and
/// import counts — what the `/api/project/import` route calls. Enriched
/// from `state.product_db` when one is configured (the bonus fix this
/// task adds: nothing previously wired a connection in for this path to
/// use, unlike `knx import --product-db` on the CLI).
pub fn open_project(state: &AppState, path: &Path) -> Result<ProjectTree, String> {
    let guard = state
        .product_db
        .as_ref()
        .map(|m| m.lock().expect("state mutex poisoned"));
    let (tree, project) =
        import_and_project(path, guard.as_deref()).map_err(|e| e.to_string())?;
    drop(guard);
    *state.project.lock().expect("state mutex poisoned") = Some(project);
    *state.command_stack.lock().expect("state mutex poisoned") = knx_core::CommandStack::new();
    *state.import_counts.lock().expect("state mutex poisoned") = (tree.errors, tree.warnings);
    Ok(tree)
}
```

- [ ] **Step 5: Run the test to verify it passes**

Run: `cargo test -p knx-server --lib opening_a_project_through_a_wired_product_db`
Expected: PASS.

- [ ] **Step 6: Run the full existing `knx-server` test suite to confirm no regression**

Run: `cargo test -p knx-server`
Expected: PASS — every pre-existing integration test (`http_project_routes.rs`'s golden-count test in particular) unaffected, since enrichment never changes `tree.errors`/`tree.warnings`/topology shape, only comm-object `dpt`/`text`/flags fields.

- [ ] **Step 7: Run clippy**

Run: `cargo clippy -p knx-server --all-targets -- -D warnings`
Expected: PASS.

- [ ] **Step 8: Commit**

```bash
git add apps/knx-server/Cargo.toml apps/knx-server/src/domain.rs
git commit -m "feat(knx-server): wire AppState.product_db through project import"
```

---

### Task 4: `apps/knx-server::domain` — `create_device_impl`/`delete_device_impl`

**Files:**
- Modify: `apps/knx-server/src/domain.rs`

**Interfaces:**
- Consumes: `Command::CreateDevice`/`DeleteDevice` (Task 1), `knx_productdb::query::{catalog_item, resolve_program, com_object_ref_ids, com_object_view, CatalogItemRow}` (Task 2), `knx_productdb::enrich::apply` (Task 2), `AppState.product_db` (Task 3), the existing private `apply(state, cmd)` helper and `tree_with_state` helper already in `domain.rs`.
- Produces: `pub fn create_device_impl(state: &AppState, line_id: Option<u32>, catalog_item_id: String, name: String) -> Result<ProjectTree, String>`, `pub fn delete_device_impl(state: &AppState, id: u32) -> Result<ProjectTree, String>` — Task 5's routes call these two by exact name and signature.

- [ ] **Step 1: Write the failing tests**

Add to `apps/knx-server/src/domain.rs`'s `#[cfg(test)] mod tests` (from Task 3), after the existing test:

```rust
    fn state_with_one_installation() -> AppState {
        let mut project = knx_core::Project::new(knx_core::Language("en".into()));
        project.installations.push(knx_core::Installation {
            id: knx_core::InstallationId(0),
            name: "I".into(),
            default_line: None,
            multicast_address: None,
            completion: knx_core::CompletionStatus::FinishedDesign,
            topology: knx_core::Topology {
                areas: vec![],
                lines: vec![],
                unassigned: vec![],
            },
            buildings: vec![],
            group_ranges: vec![],
            group_addresses: vec![],
            parameters: vec![],
        });
        let mut state = AppState::default();
        // `AppState::default()` now runs the real `default_path()` lookup
        // (Task 3) — on a machine that already has a product database at
        // e.g. `~/.local/share/knx/products.sqlite`, `default_path()`
        // would pick it up here, making
        // `creating_a_device_without_a_product_database_is_an_error`
        // depend on the environment. Force `None` explicitly so this
        // helper's guarantee ("no product database configured") holds
        // everywhere, not just on a machine without one.
        state.product_db = None;
        *state.project.lock().unwrap() = Some(project);
        state
    }

    const CATALOG_HARDWARE: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<KNX xmlns="http://knx.org/xml/project/11"><ManufacturerData><Manufacturer RefId="M-1">
<Hardware><Hardware Id="H-1" Name="X" SerialNumber="S" VersionNumber="1">
<Hardware2Programs><Hardware2Program Id="H-1_HP-1" MediumTypes="MT-0">
<ApplicationProgramRef RefId="A-1" /></Hardware2Program></Hardware2Programs>
</Hardware></Hardware></Manufacturer></ManufacturerData></KNX>"#;

    const CATALOG_PROGRAM: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<KNX xmlns="http://knx.org/xml/project/11"><ManufacturerData><Manufacturer RefId="M-1">
<ApplicationPrograms><ApplicationProgram Id="A-1" Name="P" ApplicationVersion="1"
  MaskVersion="MV-0701"><Static>
<ComObjectTable>
  <ComObject Id="A-1_O-1" Number="1" Text="Schalten" ObjectSize="1 Bit"
             DatapointType="DPST-1-1" WriteFlag="Enabled" />
</ComObjectTable>
<ComObjectRefs>
  <ComObjectRef Id="A-1_O-1_R-1" RefId="A-1_O-1" />
</ComObjectRefs>
</Static></ApplicationProgram></ApplicationPrograms></Manufacturer></ManufacturerData></KNX>"#;

    const CATALOG_ITEM: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<KNX xmlns="http://knx.org/xml/project/11">
  <ManufacturerData>
    <Manufacturer RefId="M-1">
      <Catalog>
        <CatalogSection Id="M-1_CG-1" Name="Actuators" Number="1" DefaultLanguage="de-DE">
          <CatalogItem Id="M-1_CI-1" Name="Schaltaktor" Number="ACT-1"
                       DefaultLanguage="de-DE"
                       ProductRefId="M-1_P-1"
                       Hardware2ProgramRefId="H-1_HP-1" />
        </CatalogSection>
      </Catalog>
    </Manufacturer>
  </ManufacturerData>
</KNX>"#;

    /// Returns the backing `TempDir` alongside the state — same shape as
    /// `knx-productdb`'s own `db()` test helpers — so the temp file isn't
    /// deleted out from under the connection while the test still needs
    /// it.
    fn state_with_product_db() -> (tempfile::TempDir, AppState) {
        let state = state_with_one_installation();
        let dir = tempfile::tempdir().unwrap();
        let products = knx_productdb::open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
        knx_productdb::parse::hardware::ingest_hardware(
            &products,
            "sha-h",
            "M-1/Hardware.xml",
            CATALOG_HARDWARE.as_bytes(),
        )
        .unwrap();
        knx_productdb::parse::program::ingest_program(
            &products,
            "sha-p",
            "M-1/A.xml",
            CATALOG_PROGRAM.as_bytes(),
        )
        .unwrap();
        knx_productdb::parse::catalog::ingest_catalog(
            &products,
            "sha-c",
            "M-1/Catalog.xml",
            CATALOG_ITEM.as_bytes(),
        )
        .unwrap();
        let mut state = state;
        state.product_db = Some(Mutex::new(products));
        (dir, state)
    }

    #[test]
    fn creating_a_device_without_a_product_database_is_an_error() {
        let state = state_with_one_installation();
        let result = create_device_impl(&state, None, "anything".into(), "D".into());
        assert_eq!(result, Err("no product database configured".to_string()));
    }

    #[test]
    fn creating_a_device_with_an_unknown_catalog_item_is_an_error() {
        let (_dir, state) = state_with_product_db();
        let result = create_device_impl(&state, None, "nope".into(), "D".into());
        assert_eq!(result, Err("catalog item not found".to_string()));
    }

    #[test]
    fn creating_a_device_seeds_its_com_objects_and_deleting_it_round_trips() {
        let (_dir, state) = state_with_product_db();
        let tree =
            create_device_impl(&state, None, "M-1_CI-1".into(), "Actuator 1".into()).unwrap();
        assert_eq!(
            tree.installations[0].unassigned.len(),
            1,
            "the new device lands in unassigned when no line is given"
        );
        let device_id = tree.installations[0].unassigned[0].id;

        let detail = device_detail(&state, device_id).unwrap();
        assert_eq!(detail.com_objects.len(), 1);
        assert!(
            detail.com_objects[0].dpt.is_some(),
            "the com object was enriched at creation time from the product database"
        );

        let tree = delete_device_impl(&state, device_id).unwrap();
        assert!(tree.installations[0].unassigned.is_empty());
    }
```

- [ ] **Step 2: Run the tests to verify they fail to compile**

Run: `cargo test -p knx-server --lib create_device -- --list`
Expected: compile error — `create_device_impl`/`delete_device_impl` not found.

(Before implementing, check `knx_projection::ComObjectNode`'s actual `dpt` field name/type by reading `crates/knx-projection/src/lib.rs`'s `ComObjectNode` struct — the test above assumes a `dpt: Option<...>` field with a truthy `Some` when a DPT is resolved. Adjust the assertion to match whatever that field is actually called if it differs; do not change `create_device_impl`'s own logic to make a wrong assertion pass.)

- [ ] **Step 3: Implement `create_device_impl` and `delete_device_impl`**

Add to `apps/knx-server/src/domain.rs`, after `unlink_com_object_impl` and before `undo_impl`:

```rust
/// Creates a device from a product-database catalog entry (design doc
/// §3). A catalog item with no resolvable hardware program — passive
/// hardware, or a `hardware2program_ref_id` this product database
/// doesn't have — still creates a device, just with zero communication
/// objects; that is not an error.
pub fn create_device_impl(
    state: &AppState,
    line_id: Option<u32>,
    catalog_item_id: String,
    name: String,
) -> Result<knx_projection::ProjectTree, String> {
    // Step 1 (design doc §3.1): everything the product database can tell
    // us, gathered while only `product_db` is locked — dropped before
    // `project` is locked below, so the two mutexes are never held at
    // once.
    let (product_ref, program_ref, seeds) = {
        let products = state
            .product_db
            .as_ref()
            .ok_or("no product database configured")?
            .lock()
            .expect("state mutex poisoned");
        let item = knx_productdb::query::catalog_item(&products, &catalog_item_id)
            .map_err(|e| e.to_string())?
            .ok_or("catalog item not found")?;
        let mut seeds: Vec<(String, knx_productdb::query::ComObjectView)> = Vec::new();
        if let Some(program_ref) = &item.hardware2program_ref_id {
            if let Some(program_id) =
                knx_productdb::query::resolve_program(&products, program_ref)
                    .map_err(|e| e.to_string())?
            {
                for ref_id in knx_productdb::query::com_object_ref_ids(&products, &program_id)
                    .map_err(|e| e.to_string())?
                {
                    if let Some(view) =
                        knx_productdb::query::com_object_view(&products, &program_id, &ref_id)
                            .map_err(|e| e.to_string())?
                    {
                        seeds.push((ref_id, view));
                    }
                }
            }
        }
        (
            item.product_ref_id.unwrap_or_default(),
            item.hardware2program_ref_id.unwrap_or_default(),
            seeds,
        )
    };

    // Step 2 (design doc §3.2): allocate ids and build the command,
    // holding `project`'s own lock continuously through step 3 below —
    // `product_db` is no longer held.
    let mut project = state.project.lock().expect("state mutex poisoned");
    let project = project.as_mut().ok_or("no project open")?;

    let device_id = project.ids.next_device_id();
    let mut com_objects = Vec::with_capacity(seeds.len());
    let mut enrich_inputs = Vec::with_capacity(seeds.len());
    for (ref_id, view) in &seeds {
        let com_id = project.ids.next_com_object_instance_id();
        com_objects.push(knx_core::ComObjectInstance {
            id: com_id,
            source: knx_core::SourceRef {
                path: ref_id.clone(),
                ets_id: ref_id.clone(),
            },
            device: device_id,
            number: view.number.unwrap_or(0) as u16,
            text: knx_core::Override::Absent,
            description: knx_core::Override::Absent,
            dpt: knx_core::Override::Absent,
            flags: knx_core::ResolvedFlags::none(),
            size: None,
            is_active: true,
            links: vec![],
            module_instance: None,
        });
        enrich_inputs.push((com_id, ref_id.clone(), view.clone()));
    }
    let device = knx_core::DeviceInstance {
        id: device_id,
        source: knx_core::SourceRef {
            path: format!("KB-DEV-{}", device_id.0),
            ets_id: format!("KB-DEV-{}", device_id.0),
        },
        name,
        description: None,
        address: None,
        product_ref,
        program_ref,
        commissioning: knx_core::CommissioningState::default(),
        visibility_calculated: true,
        com_objects: com_objects.iter().map(|c| c.id).collect(),
        binary_data: vec![],
    };
    let cmd = knx_core::Command::CreateDevice {
        device,
        com_objects,
        line: line_id.map(knx_core::LineId),
    };
    {
        let mut stack = state.command_stack.lock().expect("state mutex poisoned");
        stack.do_command(project, cmd).map_err(|e| e.to_string())?;
    }

    // Step 3 (design doc §3.3): seed enrichment once, same mapping
    // `knx_productdb::enrich()` uses on import, not pushed onto the undo
    // stack — undoing `CreateDevice` removes the device regardless of
    // which slots got filled, and `DeleteDevice`'s own inverse captures
    // the enriched state for redo (Task 1). `issues` (ambiguous DPT
    // lists, missing com-object-ref rows) are collected but not surfaced
    // anywhere this slice — see KNOWN_LIMITATIONS.md.
    let mut issues = Vec::new();
    for (com_id, ref_id, view) in &enrich_inputs {
        knx_productdb::enrich::apply(project, *com_id, ref_id, view, &mut issues);
    }

    let stack = state.command_stack.lock().expect("state mutex poisoned");
    let import_counts = *state.import_counts.lock().expect("state mutex poisoned");
    Ok(tree_with_state(project, &stack, import_counts))
}

pub fn delete_device_impl(state: &AppState, id: u32) -> Result<knx_projection::ProjectTree, String> {
    apply(state, knx_core::Command::DeleteDevice {
        id: knx_core::DeviceId(id),
    })
}
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p knx-server --lib create_device`
Expected: PASS. If the `ComObjectNode` field-name assumption from Step 2 was wrong, fix the *test's* assertion to the real field name/shape — do not change `create_device_impl`.

- [ ] **Step 5: Run the full `knx-server` test suite and clippy**

Run: `cargo test -p knx-server && cargo clippy -p knx-server --all-targets -- -D warnings`
Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add apps/knx-server/src/domain.rs
git commit -m "feat(knx-server): create_device_impl/delete_device_impl"
```

---

### Task 5: `apps/knx-server` routes — catalog browser reads + device create/delete

**Files:**
- Modify: `apps/knx-server/src/routes.rs`
- Create: `apps/knx-server/tests/http_device_routes.rs`

**Interfaces:**
- Consumes: `domain::{create_device_impl, delete_device_impl}` (Task 4), `knx_productdb::query::{manufacturers, catalog_items, CatalogItemRow}` (Task 2 for the new two; `manufacturers` already existed), `AppState.product_db` (Task 3).
- Produces: four new HTTP routes — `GET /api/catalog/manufacturers`, `GET /api/catalog/items`, `POST /api/devices`, `DELETE /api/devices/{id}` — no other crate depends on these names, they are the final consumer.

- [ ] **Step 1: Add the two catalog-read domain functions**

In `apps/knx-server/src/domain.rs`, add right before `create_device_impl` (kept here rather than in Task 4 since they're read-only catalog listings, not device mutations, but small enough to land in the same file edit as Task 5's routes call them):

```rust
pub fn catalog_manufacturers_impl(state: &AppState) -> Result<Vec<(String, Option<String>)>, String> {
    let products = state
        .product_db
        .as_ref()
        .ok_or("no product database configured")?
        .lock()
        .expect("state mutex poisoned");
    knx_productdb::query::manufacturers(&products).map_err(|e| e.to_string())
}

pub fn catalog_items_impl(
    state: &AppState,
    manufacturer: Option<String>,
    search: Option<String>,
) -> Result<Vec<knx_productdb::query::CatalogItemRow>, String> {
    let products = state
        .product_db
        .as_ref()
        .ok_or("no product database configured")?
        .lock()
        .expect("state mutex poisoned");
    knx_productdb::query::catalog_items(&products, manufacturer.as_deref(), search.as_deref())
        .map_err(|e| e.to_string())
}
```

- [ ] **Step 2: Wire the four routes**

In `apps/knx-server/src/routes.rs`, change the import line:

```rust
use axum::routing::{delete, post};
```

to:

```rust
use axum::extract::Query;
use axum::routing::{delete, get, post};
```

(`Query` goes on its own `use` line since the existing `use axum::extract::Path as AxumPath;`/`use axum::extract::State;` lines are already there — add `Query` as a third, don't merge into either.)

In `project_routes()`, add before the closing `.route("/api/redo", post(redo))` line (order doesn't matter functionally; grouping with the other CRUD routes reads best — add right after the `/api/group-links` routes and before `/api/undo`):

```rust
        .route("/api/catalog/manufacturers", get(catalog_manufacturers))
        .route("/api/catalog/items", get(catalog_items))
        .route("/api/devices", post(create_device))
        .route("/api/devices/{id}", delete(delete_device))
```

Then add the handlers, anywhere after `unlink_com_object` and before `undo`:

```rust
#[derive(serde::Serialize)]
struct CatalogManufacturerDto {
    id: String,
    name: Option<String>,
}

async fn catalog_manufacturers(
    State(state): State<SharedState>,
) -> Result<Json<Vec<CatalogManufacturerDto>>, ApiError> {
    domain::catalog_manufacturers_impl(&state)
        .map(|rows| {
            Json(
                rows.into_iter()
                    .map(|(id, name)| CatalogManufacturerDto { id, name })
                    .collect(),
            )
        })
        .map_err(ApiError::bad_request)
}

#[derive(Deserialize)]
struct CatalogItemsQuery {
    #[serde(default)]
    manufacturer: Option<String>,
    #[serde(default)]
    search: Option<String>,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct CatalogItemDto {
    id: String,
    manufacturer_id: String,
    name: Option<String>,
    number: Option<String>,
    visible_description: Option<String>,
    product_ref_id: Option<String>,
    hardware2program_ref_id: Option<String>,
}

impl From<knx_productdb::query::CatalogItemRow> for CatalogItemDto {
    fn from(r: knx_productdb::query::CatalogItemRow) -> Self {
        Self {
            id: r.id,
            manufacturer_id: r.manufacturer_id,
            name: r.name,
            number: r.number,
            visible_description: r.visible_description,
            product_ref_id: r.product_ref_id,
            hardware2program_ref_id: r.hardware2program_ref_id,
        }
    }
}

async fn catalog_items(
    State(state): State<SharedState>,
    Query(q): Query<CatalogItemsQuery>,
) -> Result<Json<Vec<CatalogItemDto>>, ApiError> {
    domain::catalog_items_impl(&state, q.manufacturer, q.search)
        .map(|rows| Json(rows.into_iter().map(CatalogItemDto::from).collect()))
        .map_err(ApiError::bad_request)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateDeviceBody {
    #[serde(default)]
    line_id: Option<u32>,
    catalog_item_id: String,
    name: String,
}

async fn create_device(
    State(state): State<SharedState>,
    Json(body): Json<CreateDeviceBody>,
) -> Result<Json<knx_projection::ProjectTree>, ApiError> {
    domain::create_device_impl(&state, body.line_id, body.catalog_item_id, body.name)
        .map(Json)
        .map_err(ApiError::bad_request)
}

async fn delete_device(
    State(state): State<SharedState>,
    AxumPath(id): AxumPath<u32>,
) -> Result<Json<knx_projection::ProjectTree>, ApiError> {
    domain::delete_device_impl(&state, id)
        .map(Json)
        .map_err(ApiError::bad_request)
}
```

- [ ] **Step 3: Write the failing route tests**

Create `apps/knx-server/tests/http_device_routes.rs`:

```rust
use std::sync::{Arc, Mutex};

use axum::body::Body;
use axum::http::{Request, StatusCode};
use knx_core::{
    CommissioningState, CompletionStatus, DeviceId, DeviceInstance, Direction, GroupAddress,
    GroupAddressEntry, GroupAddressId, GroupLink, Installation, InstallationId, Language,
    Override, Project, ResolvedFlags, SourceRef, Topology,
};
use serde_json::{json, Value};
use tower::ServiceExt;

fn state_with_one_installation() -> knx_server::AppState {
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
    let mut state = knx_server::AppState::default();
    // `AppState::default()` runs the real `default_path()` lookup (Task
    // 3) — on a machine that already has a product database at e.g.
    // `~/.local/share/knx/products.sqlite`, `default_path()` would pick
    // it up here, making `creating_a_device_without_a_product_database_
    // is_a_400` depend on the environment. Force `None` explicitly.
    state.product_db = None;
    *state.project.lock().unwrap() = Some(project);
    state
}

const HARDWARE: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<KNX xmlns="http://knx.org/xml/project/11"><ManufacturerData><Manufacturer RefId="M-1">
<Hardware><Hardware Id="H-1" Name="X" SerialNumber="S" VersionNumber="1">
<Hardware2Programs><Hardware2Program Id="H-1_HP-1" MediumTypes="MT-0">
<ApplicationProgramRef RefId="A-1" /></Hardware2Program></Hardware2Programs>
</Hardware></Hardware></Manufacturer></ManufacturerData></KNX>"#;

const PROGRAM: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<KNX xmlns="http://knx.org/xml/project/11"><ManufacturerData><Manufacturer RefId="M-1">
<ApplicationPrograms><ApplicationProgram Id="A-1" Name="P" ApplicationVersion="1"
  MaskVersion="MV-0701"><Static>
<ComObjectTable>
  <ComObject Id="A-1_O-1" Number="1" Text="Schalten" ObjectSize="1 Bit"
             DatapointType="DPST-1-1" WriteFlag="Enabled" />
</ComObjectTable>
<ComObjectRefs>
  <ComObjectRef Id="A-1_O-1_R-1" RefId="A-1_O-1" />
</ComObjectRefs>
</Static></ApplicationProgram></ApplicationPrograms></Manufacturer></ManufacturerData></KNX>"#;

const CATALOG: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<KNX xmlns="http://knx.org/xml/project/11">
  <ManufacturerData>
    <Manufacturer RefId="M-1">
      <Catalog>
        <CatalogSection Id="M-1_CG-1" Name="Actuators" Number="1" DefaultLanguage="de-DE">
          <CatalogItem Id="M-1_CI-1" Name="Schaltaktor" Number="ACT-1"
                       DefaultLanguage="de-DE"
                       ProductRefId="M-1_P-1"
                       Hardware2ProgramRefId="H-1_HP-1" />
        </CatalogSection>
      </Catalog>
    </Manufacturer>
  </ManufacturerData>
</KNX>"#;

fn temp_product_db() -> (tempfile::TempDir, knx_productdb::Connection) {
    let dir = tempfile::tempdir().unwrap();
    let conn = knx_productdb::open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
    knx_productdb::parse::hardware::ingest_hardware(&conn, "sha-h", "M-1/Hardware.xml", HARDWARE.as_bytes())
        .unwrap();
    knx_productdb::parse::program::ingest_program(&conn, "sha-p", "M-1/A.xml", PROGRAM.as_bytes())
        .unwrap();
    knx_productdb::parse::catalog::ingest_catalog(&conn, "sha-c", "M-1/Catalog.xml", CATALOG.as_bytes())
        .unwrap();
    (dir, conn)
}

async fn body_json(response: axum::response::Response) -> Value {
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

#[tokio::test]
async fn creating_a_device_from_a_catalog_item_seeds_its_com_objects_then_deletes() {
    let mut state = state_with_one_installation();
    let (_dir, products) = temp_product_db();
    state.product_db = Some(Mutex::new(products));
    let app = knx_server::app(Arc::new(state), None);

    let create = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/devices")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({ "catalogItemId": "M-1_CI-1", "name": "Actuator 1" }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(create.status(), StatusCode::OK);
    let tree = body_json(create).await;
    let devices = tree["installations"][0]["unassigned"].as_array().unwrap();
    assert_eq!(devices.len(), 1);
    let device_id = devices[0]["id"].as_u64().unwrap();

    let detail = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/device/{device_id}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(detail.status(), StatusCode::OK);
    let detail_json = body_json(detail).await;
    assert_eq!(detail_json["com_objects"].as_array().unwrap().len(), 1);

    let delete = app
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/devices/{device_id}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(delete.status(), StatusCode::OK);
    let tree = body_json(delete).await;
    assert!(tree["installations"][0]["unassigned"]
        .as_array()
        .unwrap()
        .is_empty());
}

#[tokio::test]
async fn creating_a_device_without_a_product_database_is_a_400() {
    let state = state_with_one_installation();
    let app = knx_server::app(Arc::new(state), None);

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/devices")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({ "catalogItemId": "anything", "name": "D" }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn deleting_a_device_with_a_linked_com_object_is_a_400() {
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
            unassigned: vec![DeviceId(1)],
        },
        buildings: vec![],
        group_ranges: vec![],
        group_addresses: vec![GroupAddressEntry {
            id: GroupAddressId(1),
            source: SourceRef {
                path: "t".into(),
                ets_id: "t".into(),
            },
            name: "GA".into(),
            address: GroupAddress::from_raw(1),
            central: false,
            unfiltered: false,
            range: None,
        }],
        parameters: vec![],
    });
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
        com_objects: vec![knx_core::ComObjectInstanceId(1)],
        binary_data: vec![],
    });
    project.devices.insert_com_object(knx_core::ComObjectInstance {
        id: knx_core::ComObjectInstanceId(1),
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
        links: vec![GroupLink {
            ga: GroupAddressId(1),
            direction: Direction::Send,
        }],
        module_instance: None,
    });
    let state = knx_server::AppState::default();
    *state.project.lock().unwrap() = Some(project);
    let app = knx_server::app(Arc::new(state), None);

    let response = app
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri("/api/devices/1")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn catalog_routes_list_the_manufacturer_and_item_just_ingested() {
    let mut state = state_with_one_installation();
    let (_dir, products) = temp_product_db();
    state.product_db = Some(Mutex::new(products));
    let app = knx_server::app(Arc::new(state), None);

    let manufacturers = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/catalog/manufacturers")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(manufacturers.status(), StatusCode::OK);
    let manufacturers = body_json(manufacturers).await;
    assert_eq!(manufacturers.as_array().unwrap().len(), 1);
    assert_eq!(manufacturers[0]["id"], "M-1");

    let items = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/catalog/items?manufacturer=M-1&search=schalt")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(items.status(), StatusCode::OK);
    let items = body_json(items).await;
    assert_eq!(items.as_array().unwrap().len(), 1);
    assert_eq!(items[0]["id"], "M-1_CI-1");
}
```

- [ ] **Step 4: Run the tests to verify they fail to compile / fail**

Run: `cargo test -p knx-server --test http_device_routes -- --list`
Expected: compile error until Step 2's routes exist; once routes exist but before this step is otherwise done, re-run the actual tests and confirm they fail for the right reason (404s) before Step 2's implementation lands. (In practice, do Step 2 first if your editor makes writing against a nonexistent route awkward — the important checkpoint is Step 5's green run, not a literal red run of a file that can't even compile.)

- [ ] **Step 5: Run the tests to verify they pass**

Run: `cargo test -p knx-server --test http_device_routes`
Expected: PASS, all four tests.

- [ ] **Step 6: Run the full `knx-server` test suite and clippy**

Run: `cargo test -p knx-server && cargo clippy -p knx-server --all-targets -- -D warnings`
Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add apps/knx-server/src/routes.rs apps/knx-server/src/domain.rs apps/knx-server/tests/http_device_routes.rs
git commit -m "feat(knx-server): catalog manufacturer/item routes, POST/DELETE /api/devices"
```

---

### Task 6: Documentation

**Files:**
- Modify: `docs/GAP_ANALYSIS_ETS.md`
- Modify: `docs/IMPLEMENTATION_STATUS.md`
- Modify: `docs/KNOWN_LIMITATIONS.md`

**Interfaces:** none — this task only writes prose, referencing the exact names Tasks 1-5 already created.

- [ ] **Step 1: Count the new tests**

Run:

```bash
git log --oneline -6
git diff --stat HEAD~5..HEAD -- crates/knx-core crates/knx-productdb apps/knx-server
```

(Adjust the commit range to however many commits Tasks 1-5 actually produced — one per task if each task's Step "Commit" ran as written.) Note the total count of new `#[test]`/`#[tokio::test]` functions added across `crates/knx-core/src/command.rs`, `crates/knx-productdb/src/query.rs`, `crates/knx-productdb/src/enrich.rs`, `apps/knx-server/src/domain.rs`, and `apps/knx-server/tests/http_device_routes.rs` — you will cite this exact number in Step 3.

- [ ] **Step 2: Update `docs/GAP_ANALYSIS_ETS.md`**

Replace the existing T1 bullet:

```markdown
- **T1. Device-from-catalog insertion command.** Add `Command::CreateDevice`
  (or equivalent), taking a target line and a `knx-productdb` product/
  hardware reference, producing a `Device` with its comm-object instances
  seeded from the application program (mirrors `enrich.rs`'s existing
  `Override::Absent`-filling logic, applied once at creation time rather
  than on every load). Closes **B1**.
  Depends on: none architecturally — `knx-productdb::query` and
  `knx_core::Devices` both already exist; this is new command-layer work
  only.
```

with:

```markdown
- **T1. Device-from-catalog insertion command. Done (2026-09-08, backend
  only — see [IMPLEMENTATION_STATUS.md](IMPLEMENTATION_STATUS.md)).**
  `Command::CreateDevice` lands, seeding a device's communication-object
  instances from `knx-productdb` once at creation time
  (`knx_productdb::enrich::apply`, made `pub` and reused directly rather
  than duplicated) instead of on every load. `apps/knx-server` gains
  `POST /api/devices` plus the `GET /api/catalog/manufacturers`/
  `GET /api/catalog/items` routes T2's future browser will call; no
  frontend caller yet. Closes **B1**.
```

Replace the existing T3 bullet:

```markdown
- **T3. Device deletion command.** `Command::DeleteDevice`, symmetric
  with T1, refusing (like `DeleteGroupAddress` today) if the device has
  active group links, or cascading with an explicit confirmation —
  a design decision to make explicitly, not default to either behavior
  silently. Closes **B2**.
```

with:

```markdown
- **T3. Device deletion command. Done (2026-09-08, backend only).**
  `Command::DeleteDevice` refuses (`CommandError::DeviceHasLinks`) if any
  of the device's communication objects still links to a group address —
  the explicit choice this task's own text asked for, matching
  `DeleteArea`/`DeleteLine`/`DeleteGroupAddress`'s existing
  refuse-with-dependents convention rather than a silent cascade.
  `apps/knx-server` gains `DELETE /api/devices/{id}`; no frontend caller
  yet. Closes **B2**.
```

- [ ] **Step 3: Append to `docs/IMPLEMENTATION_STATUS.md`**

Add, at the end of the file, a new paragraph (matching the style of the existing `**T23, third slice ...**` entries already there — bold lead sentence, then prose):

```markdown

**T1/T3, device create/delete commands (2026-09-08) — backend only.**
`knx-core` gains `Command::CreateDevice`/`DeleteDevice`
(`CommandError::DeviceHasLinks` for the latter's refuse-with-dependents
case, matching `DeleteArea`/`DeleteLine`/`DeleteGroupAddress`'s own
convention); `MoveDeviceToLine`'s device-lookup was factored into a
shared `remove_device_from_topology` helper both commands now use.
`knx-productdb` gains `catalog_items`/`catalog_item`/`com_object_ref_ids`
in `query.rs` (backing the future catalog browser, T2) and makes
`enrich::apply` `pub`, so device creation seeds a device's comm objects
from the product database once at creation time instead of duplicating
`enrich()`'s own DPT/text/flags mapping. `apps/knx-server` gains
`AppState.product_db: Option<Mutex<knx_productdb::Connection>>` (opened
from `knx_productdb::default_path()`, gracefully `None` on any failure,
per ADR-0012), a `create_device_impl`/`delete_device_impl`/
`catalog_manufacturers_impl`/`catalog_items_impl` set in `domain.rs`,
and four routes: `GET /api/catalog/manufacturers`,
`GET /api/catalog/items`, `POST /api/devices`, `DELETE /api/devices/{id}`.
Bonus fix, same root cause, bundled in: `import_and_project` (backing
`/api/project/import`) now actually wires `state.product_db` through to
`import_ets_project_with` — every `.knxproj` opened through
`apps/knx-server` had never been enriched from the product database
until now, unlike `knx import --product-db` on the CLI, since nothing
previously threaded a connection through for it to use.
`open_project_impl` (the server-free golden-count path
`open_reference_project.rs` exercises) deliberately stays unenriched, so
that test's counts remain deterministic. No frontend caller for any of
the four new routes — T2, the catalog browser UI, is its own future
cycle, the same "backend now, UI later" shape as the 2026-09-06
topology/group-range/group-link command layer (T4-T6), which got its
frontend in T23 a day later. <N> new tests across
`crates/knx-core`/`crates/knx-productdb`/`apps/knx-server`. Closes
**B1**/**B2** ([GAP_ANALYSIS_ETS.md](GAP_ANALYSIS_ETS.md)), backend
only; **T2** stays open.
```

Replace `<N>` with the exact count from Step 1.

- [ ] **Step 4: Append to `docs/KNOWN_LIMITATIONS.md`**

Confirm `## 34.` is still the last section number (`grep -n "^## " docs/KNOWN_LIMITATIONS.md | tail -3`); if a later limitation has landed in the meantime and 35 is taken, use the next free number instead throughout this step. Append at the end of the file:

```markdown

## 35. Device-creation `EnrichmentIssue`s are silently dropped

**Limitation.** `apps/knx-server`'s `create_device_impl` seeds a newly
created device's communication objects from the product database via
`knx_productdb::enrich::apply`, exactly like import's own `enrich()`
pass — except the `Vec<EnrichmentIssue>` it collects (ambiguous DPT
lists, a `ComObjectRef` id the resolved program doesn't have) is
discarded rather than surfaced anywhere. A device created against an
application program with an ambiguous DPT list on one of its
communication objects gets that communication object with no DPT set
and no visible warning.

**Cause.** Import has `ImportReport` as an existing, already-wired
channel for this; `POST /api/devices` has no equivalent yet — building
one was out of scope for this slice (see
[docs/superpowers/specs/2026-09-07-device-create-delete-design.md](superpowers/specs/2026-09-07-device-create-delete-design.md)).

**Impact.** Silent: the affected communication object is
indistinguishable, from the API's response alone, from one whose DPT
was never set on purpose. Recoverable by hand via the existing
`SetComObjectDpt` command/UI once a user notices, but nothing prompts
them to look.

**Lifted when.** `create_device_impl` returns its `issues` alongside the
projected tree (or a dedicated response field) and T2's future
catalog-browser UI surfaces them — the same role import's own report
screen (T11, still open) would play for import.
```

- [ ] **Step 5: Build the whole workspace to confirm nothing is broken end to end**

Run: `cargo build --workspace && cargo test --workspace --exclude knx-etsproj`

(`knx-etsproj` is excluded because its `clippy` failure is pre-existing and unrelated per Step 11 of Task 1's note; its own `cargo test` should still pass — running `cargo test --workspace` without the exclude is fine too and is the more thorough check if you have the time budget for it. Either way, every crate this plan touched — `knx-core`, `knx-productdb`, `knx-server` — must show all-green.)

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add docs/GAP_ANALYSIS_ETS.md docs/IMPLEMENTATION_STATUS.md docs/KNOWN_LIMITATIONS.md
git commit -m "docs: mark T1/T3 done, device create/delete cycle, new limitation #35"
```
