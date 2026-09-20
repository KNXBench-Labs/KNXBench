# Topology & group-range command layer Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Give `knx-core` a command layer for topology (area/line) CRUD,
group-range CRUD, and group-link editing, and expose all of it over
`apps/knx-server`'s HTTP API — backend only, no frontend UI this cycle.

**Architecture:** Nine new `knx_core::Command` variants following the
existing `CreateGroupAddress`/`DeleteGroupAddress` pattern exactly
(caller pre-allocates the id via `Project::ids`, `apply` returns the exact
inverse for undo/redo). Four new `validation.rs` functions close
previously-unwired checks. `apps/knx-server`'s `domain.rs`/`routes.rs`
gain one `_impl` function and one route per command, unchanged in shape
from the six that already exist there.

**Tech Stack:** Rust, `knx-core` (no IO), `axum` (`apps/knx-server`),
`ts-rs` for the one small, additive `knx-projection` change Task 7 needs.

**Spec:** [docs/superpowers/specs/2026-09-06-topology-group-range-commands-design.md](../specs/2026-09-06-topology-group-range-commands-design.md)

## Global Constraints

- Every command's `apply` leaves `project` untouched on `Err` — validate
  before mutating, always.
- Every command targets `installations[0]` only (`Command::apply`'s
  existing doc comment) — no new command routes to a different
  installation.
- Delete commands refuse (never cascade) when the target still owns
  children — `CommandError::*NotEmpty`/`*InUse`, matching
  `DeleteGroupAddress`'s existing `GroupAddressInUse` precedent.
- No frontend/UI change in this plan. `apps/knx-web` is untouched.
- **Deviation from the spec, decided during planning, noted here for the
  record:** the spec's "synthetic id convention" section says
  `create_group_address_impl`'s `range` parameter becomes a *required*
  argument. Implementing that literally breaks the already-shipped
  frontend group-address creation feature (Session 5 cycle 9), which
  never sends a range and has no UI yet to pick one (that UI is
  Sub-Project 2's job). Task 8 below keeps `range_id` **optional** at the
  HTTP boundary (`#[serde(default)]`) — today's no-range creation path
  keeps working exactly as before, unchanged. The synthetic-id fix
  (KNOWN_LIMITATIONS §21's other half) still lands unconditionally. Task 9
  documents this honestly: §21 is *partially*, not fully, closed by this
  plan.

---

### Task 1: Validation helpers for areas, lines, and group ranges

**Files:**
- Modify: `crates/knx-core/src/validation.rs`

**Interfaces:**
- Consumes: `crate::topology::{Area, Line, Topology}`, `crate::group::GroupRange`, `crate::address::GroupAddress`, `crate::ids::{AreaId, LineId, GroupRangeId}` (all pre-existing).
- Produces (used by Tasks 2, 3, 5):
  - `check_no_duplicate_area_address(topology: &Topology, candidate: AreaId, address: u8) -> Result<(), ValidationError>`
  - `check_no_duplicate_line_address(area: &Area, lines: &[Line], candidate: LineId, address: u8) -> Result<(), ValidationError>`
  - `check_group_range_nests_in_parent(parent: &GroupRange, candidate: GroupRangeId, start: GroupAddress, end: GroupAddress) -> Result<(), ValidationError>`
  - `check_no_overlapping_group_range<'a>(siblings: impl Iterator<Item = &'a GroupRange>, candidate: GroupRangeId, start: GroupAddress, end: GroupAddress) -> Result<(), ValidationError>`
  - New `ValidationError` variants: `DuplicateAreaAddress { address: u8, existing: AreaId, new: AreaId }`, `DuplicateLineAddress { address: u8, existing: LineId, new: LineId }`, `GroupRangeOutsideParent { range: GroupRangeId, parent: GroupRangeId }`, `OverlappingGroupRange { range: GroupRangeId, existing: GroupRangeId }`.

- [ ] **Step 1: Write the failing tests**

Add to `crates/knx-core/src/validation.rs`'s existing `#[cfg(test)] mod tests` block (it already has `test_source()` and imports `CompletionStatus`, `DeviceInstance`, `Topology` — add `Area`, `Line`, `GroupRange`, `AreaId`, `LineId`, `GroupRangeId`, `GroupAddress` to its `use` lines):

```rust
    #[test]
    fn duplicate_area_address_is_rejected() {
        let topology = Topology {
            areas: vec![Area {
                id: AreaId(1),
                source: test_source(),
                name: "A1".into(),
                address: 1,
                completion: CompletionStatus::FinishedDesign,
                lines: vec![],
            }],
            lines: vec![],
            unassigned: vec![],
        };
        let err = check_no_duplicate_area_address(&topology, AreaId(2), 1);
        assert!(matches!(
            err,
            Err(ValidationError::DuplicateAreaAddress { .. })
        ));
    }

    #[test]
    fn an_area_reusing_its_own_address_is_not_a_duplicate() {
        let topology = Topology {
            areas: vec![Area {
                id: AreaId(1),
                source: test_source(),
                name: "A1".into(),
                address: 1,
                completion: CompletionStatus::FinishedDesign,
                lines: vec![],
            }],
            lines: vec![],
            unassigned: vec![],
        };
        assert!(check_no_duplicate_area_address(&topology, AreaId(1), 1).is_ok());
    }

    fn test_line(id: LineId, address: u8) -> Line {
        Line {
            id,
            source: test_source(),
            name: "L".into(),
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

    #[test]
    fn duplicate_line_address_within_the_same_area_is_rejected() {
        let area = Area {
            id: AreaId(1),
            source: test_source(),
            name: "A".into(),
            address: 1,
            completion: CompletionStatus::FinishedDesign,
            lines: vec![LineId(1)],
        };
        let lines = vec![test_line(LineId(1), 1)];
        let err = check_no_duplicate_line_address(&area, &lines, LineId(2), 1);
        assert!(matches!(
            err,
            Err(ValidationError::DuplicateLineAddress { .. })
        ));
    }

    #[test]
    fn a_line_reusing_its_own_address_is_not_a_duplicate() {
        let area = Area {
            id: AreaId(1),
            source: test_source(),
            name: "A".into(),
            address: 1,
            completion: CompletionStatus::FinishedDesign,
            lines: vec![LineId(1)],
        };
        let lines = vec![test_line(LineId(1), 1)];
        assert!(check_no_duplicate_line_address(&area, &lines, LineId(1), 1).is_ok());
    }

    #[test]
    fn the_same_address_in_a_different_area_is_not_a_duplicate() {
        // check_no_duplicate_line_address only ever sees one area's own
        // line list, so a different area's line sharing the same address
        // number never reaches it — this documents that scoping choice.
        let area = Area {
            id: AreaId(2),
            source: test_source(),
            name: "A2".into(),
            address: 2,
            completion: CompletionStatus::FinishedDesign,
            lines: vec![],
        };
        let other_areas_line = vec![test_line(LineId(1), 1)];
        assert!(check_no_duplicate_line_address(&area, &other_areas_line, LineId(2), 1).is_ok());
    }

    fn test_range(id: GroupRangeId, start: u16, end: u16, parent: Option<GroupRangeId>) -> GroupRange {
        GroupRange {
            id,
            source: test_source(),
            name: "R".into(),
            start: GroupAddress::from_raw(start),
            end: GroupAddress::from_raw(end),
            parent,
            children: vec![],
        }
    }

    #[test]
    fn a_range_nesting_inside_its_parent_is_accepted() {
        let parent = test_range(GroupRangeId(1), 0, 2047, None);
        assert!(check_group_range_nests_in_parent(
            &parent,
            GroupRangeId(2),
            GroupAddress::from_raw(0),
            GroupAddress::from_raw(255)
        )
        .is_ok());
    }

    #[test]
    fn a_range_extending_past_its_parent_is_rejected() {
        let parent = test_range(GroupRangeId(1), 0, 255, None);
        let err = check_group_range_nests_in_parent(
            &parent,
            GroupRangeId(2),
            GroupAddress::from_raw(0),
            GroupAddress::from_raw(2047),
        );
        assert!(matches!(
            err,
            Err(ValidationError::GroupRangeOutsideParent { .. })
        ));
    }

    #[test]
    fn overlapping_sibling_ranges_are_rejected() {
        let existing = test_range(GroupRangeId(1), 0, 255, None);
        let siblings = vec![existing];
        let err = check_no_overlapping_group_range(
            siblings.iter(),
            GroupRangeId(2),
            GroupAddress::from_raw(200),
            GroupAddress::from_raw(500),
        );
        assert!(matches!(
            err,
            Err(ValidationError::OverlappingGroupRange { .. })
        ));
    }

    #[test]
    fn non_overlapping_sibling_ranges_are_accepted() {
        let existing = test_range(GroupRangeId(1), 0, 255, None);
        let siblings = vec![existing];
        assert!(check_no_overlapping_group_range(
            siblings.iter(),
            GroupRangeId(2),
            GroupAddress::from_raw(256),
            GroupAddress::from_raw(500)
        )
        .is_ok());
    }

    #[test]
    fn a_range_checked_against_its_own_current_span_is_not_an_overlap() {
        let existing = test_range(GroupRangeId(1), 0, 255, None);
        let siblings = vec![existing];
        assert!(check_no_overlapping_group_range(
            siblings.iter(),
            GroupRangeId(1),
            GroupAddress::from_raw(0),
            GroupAddress::from_raw(255)
        )
        .is_ok());
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p knx-core validation:: 2>&1 | tail -40`
Expected: FAIL to compile — `check_no_duplicate_area_address` and the
other three functions, plus the four `ValidationError` variants, do not
exist yet.

- [ ] **Step 3: Add the four `ValidationError` variants**

In `crates/knx-core/src/validation.rs`, add to the `ValidationError` enum
(after `DuplicateGroupAddress`):

```rust
    DuplicateAreaAddress {
        address: u8,
        existing: AreaId,
        new: AreaId,
    },
    DuplicateLineAddress {
        address: u8,
        existing: LineId,
        new: LineId,
    },
    GroupRangeOutsideParent {
        range: GroupRangeId,
        parent: GroupRangeId,
    },
    OverlappingGroupRange {
        range: GroupRangeId,
        existing: GroupRangeId,
    },
```

Add matching arms to the `impl fmt::Display for ValidationError` block
(after the `DuplicateGroupAddress` arm):

```rust
            ValidationError::DuplicateAreaAddress {
                address,
                existing,
                new,
            } => write!(
                f,
                "area address {address} already used by area {existing}, cannot assign to area {new}"
            ),
            ValidationError::DuplicateLineAddress {
                address,
                existing,
                new,
            } => write!(
                f,
                "line address {address} already used by line {existing} in the same area, cannot assign to line {new}"
            ),
            ValidationError::GroupRangeOutsideParent { range, parent } => write!(
                f,
                "group range {range} does not nest inside its parent range {parent}"
            ),
            ValidationError::OverlappingGroupRange { range, existing } => write!(
                f,
                "group range {range} overlaps existing range {existing}"
            ),
```

Update the top-of-file imports to add the newly-needed types:

```rust
use crate::address::{GroupAddress, IndividualAddress};
use crate::devices::Devices;
use crate::group::GroupRange;
use crate::ids::{AreaId, ComObjectInstanceId, DeviceId, GroupAddressId, GroupRangeId, LineId};
use crate::installation::Installation;
use crate::topology::{Area, Line, Topology};
```

- [ ] **Step 4: Implement the four functions**

Add after `check_no_duplicate_group_address` (before `check_group_address_in_range`):

```rust
/// Rejects assigning `address` to `candidate` if any other area in
/// `topology` already has it. An area reusing its own current address is
/// not a duplicate.
pub fn check_no_duplicate_area_address(
    topology: &Topology,
    candidate: AreaId,
    address: u8,
) -> Result<(), ValidationError> {
    for area in &topology.areas {
        if area.id != candidate && area.address == address {
            return Err(ValidationError::DuplicateAreaAddress {
                address,
                existing: area.id,
                new: candidate,
            });
        }
    }
    Ok(())
}

/// Rejects assigning `address` to `candidate` if any other line *owned by
/// `area`* already has it — line addresses are unique per area
/// (Area.Line.Device numbering), not project-wide, so this only ever
/// looks at `area`'s own line list, resolved against `lines`.
pub fn check_no_duplicate_line_address(
    area: &Area,
    lines: &[Line],
    candidate: LineId,
    address: u8,
) -> Result<(), ValidationError> {
    for &line_id in &area.lines {
        if line_id == candidate {
            continue;
        }
        if let Some(line) = lines.iter().find(|l| l.id == line_id) {
            if line.address == address {
                return Err(ValidationError::DuplicateLineAddress {
                    address,
                    existing: line.id,
                    new: candidate,
                });
            }
        }
    }
    Ok(())
}

/// Rejects a group range whose `[start, end]` span is not entirely
/// contained by `parent`'s own span.
pub fn check_group_range_nests_in_parent(
    parent: &GroupRange,
    candidate: GroupRangeId,
    start: GroupAddress,
    end: GroupAddress,
) -> Result<(), ValidationError> {
    if parent.contains(start) && parent.contains(end) {
        Ok(())
    } else {
        Err(ValidationError::GroupRangeOutsideParent {
            range: candidate,
            parent: parent.id,
        })
    }
}

/// Rejects a group range whose `[start, end]` span overlaps any sibling's
/// (ranges at the same nesting level — all main ranges, or all middle
/// ranges under the same parent). A range checked against its own current
/// span is not an overlap with itself.
pub fn check_no_overlapping_group_range<'a>(
    siblings: impl Iterator<Item = &'a GroupRange>,
    candidate: GroupRangeId,
    start: GroupAddress,
    end: GroupAddress,
) -> Result<(), ValidationError> {
    for sibling in siblings {
        if sibling.id == candidate {
            continue;
        }
        let overlaps = sibling.start.raw() <= end.raw() && start.raw() <= sibling.end.raw();
        if overlaps {
            return Err(ValidationError::OverlappingGroupRange {
                range: candidate,
                existing: sibling.id,
            });
        }
    }
    Ok(())
}
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test -p knx-core validation:: 2>&1 | tail -60`
Expected: PASS, all new tests plus every pre-existing `validation.rs`
test.

- [ ] **Step 6: Commit**

```bash
git add crates/knx-core/src/validation.rs
git commit -m "feat(knx-core): validation for area/line/group-range duplicates and nesting"
```

---

### Task 2: `Command::CreateArea` / `DeleteArea`

**Files:**
- Modify: `crates/knx-core/src/command.rs`

**Interfaces:**
- Consumes: Task 1's `check_no_duplicate_area_address`; `crate::topology::Area`; `Project::ids::next_area_id()` (pre-existing).
- Produces (used by Task 3): `CommandError::AreaNotFound(AreaId)`, `CommandError::AreaNotEmpty(AreaId)` — Task 3's `DeleteLine` reuses `AreaNotFound` is not needed but Task 3's tests construct areas the same way this task's tests do.

- [ ] **Step 1: Write the failing tests**

Add to `crates/knx-core/src/command.rs`'s `#[cfg(test)] mod tests` block.
Add `Area` to its existing `use crate::topology::Topology;` line (making
it `use crate::topology::{Area, Topology};`) and add `use crate::ids::AreaId;` — actually `AreaId` will already be in scope via `use super::*;` once
the production code (Step 3 below) imports it at the top of the file, so
no separate test-only import is needed.

```rust
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
```

Note: `LineId` is already imported in the test module's scope via
`use super::*;` once Step 3 (below) adds it to the file's top-level
imports.

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p knx-core command:: 2>&1 | tail -40`
Expected: FAIL to compile — `Command::CreateArea`/`DeleteArea`,
`CommandError::AreaNotFound`/`AreaNotEmpty` do not exist yet.

- [ ] **Step 3: Add imports, `Command` variants, and `CommandError` variants**

At the top of `crates/knx-core/src/command.rs`, change:

```rust
use crate::group::GroupAddressEntry;
use crate::ids::{ComObjectInstanceId, DeviceId, GroupAddressId};
```

to:

```rust
use crate::group::GroupAddressEntry;
use crate::ids::{AreaId, ComObjectInstanceId, DeviceId, GroupAddressId, LineId};
use crate::topology::{Area, Line};
```

(`Line`/`LineId` are unused until Task 3 — adding them now avoids a
second edit to the same import lines two tasks in a row; an unused-import
warning for `Line` alone between this task and Task 3 is expected and
harmless, `cargo test` still passes. If a `-D warnings` clippy run in CI
matters to you before Task 3 lands, `#[allow(unused_imports)]` is not
worth adding for a two-task gap — Task 3 consumes it immediately after.)

Add to the `Command` enum (after `DeleteGroupAddress`):

```rust
    /// `area.id` is pre-allocated by the caller via
    /// `Project::ids::next_area_id`.
    CreateArea {
        area: Area,
    },
    DeleteArea {
        id: AreaId,
    },
```

Add to the `CommandError` enum (after `GroupAddressInUse`):

```rust
    AreaNotFound(AreaId),
    /// A `DeleteArea` was refused because it still owns at least one line.
    AreaNotEmpty(AreaId),
```

Add to `impl fmt::Display for CommandError` (after the `GroupAddressInUse` arm):

```rust
            CommandError::AreaNotFound(id) => write!(f, "area {id} not found"),
            CommandError::AreaNotEmpty(id) => {
                write!(f, "area {id} still has lines, cannot delete")
            }
```

- [ ] **Step 4: Add the `apply` match arms**

In `Command::apply`'s `match self { ... }`, add after the
`Command::DeleteGroupAddress { .. } => { ... }` arm (still inside the
same `match`, before its closing `}`):

```rust
            Command::CreateArea { area } => {
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
```

Add `check_no_duplicate_area_address` to the existing `use
crate::validation::{...}` import line at the top of the file, so it
reads:

```rust
use crate::validation::{
    check_group_address_in_range, check_no_duplicate_area_address,
    check_no_duplicate_group_address, check_no_duplicate_individual_address, ValidationError,
};
```

(`check_group_address_in_range` is not used yet — it is added to this
same import list, unused, by this step only if your editor/linter
complains; if `cargo build` succeeds without it, leave it out until
Task 5, which is the task that actually calls it. Only add
`check_no_duplicate_area_address`.)

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test -p knx-core command:: 2>&1 | tail -60`
Expected: PASS, all new tests plus every pre-existing `command.rs` test.

- [ ] **Step 6: Commit**

```bash
git add crates/knx-core/src/command.rs
git commit -m "feat(knx-core): Command::CreateArea / DeleteArea"
```

---

### Task 3: `Command::CreateLine` / `DeleteLine`

**Files:**
- Modify: `crates/knx-core/src/command.rs`

**Interfaces:**
- Consumes: Task 1's `check_no_duplicate_line_address`; Task 2's `CommandError::AreaNotFound`; `crate::topology::{Line, Topology::area_of}` (pre-existing); `Project::ids::next_line_id()` (pre-existing).
- Produces (used by Task 4): `CommandError::LineNotFound(LineId)`, `CommandError::LineNotEmpty(LineId)`.

- [ ] **Step 1: Write the failing tests**

Add to `command.rs`'s test module:

```rust
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
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p knx-core command:: 2>&1 | tail -40`
Expected: FAIL to compile — `Command::CreateLine`/`DeleteLine`,
`CommandError::LineNotFound`/`LineNotEmpty` do not exist yet.

- [ ] **Step 3: Add `Command`/`CommandError` variants**

Add to the `Command` enum (after `DeleteArea`):

```rust
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
```

Add to `CommandError` (after `AreaNotEmpty`):

```rust
    LineNotFound(LineId),
    /// A `DeleteLine` was refused because it still owns at least one
    /// device.
    LineNotEmpty(LineId),
```

Add to `impl fmt::Display for CommandError` (after the `AreaNotEmpty` arm):

```rust
            CommandError::LineNotFound(id) => write!(f, "line {id} not found"),
            CommandError::LineNotEmpty(id) => {
                write!(f, "line {id} still has devices, cannot delete")
            }
```

Add `check_no_duplicate_line_address` to the top-of-file `use
crate::validation::{...}` block from Task 2's Step 4, so it reads:

```rust
use crate::validation::{
    check_group_address_in_range, check_no_duplicate_area_address,
    check_no_duplicate_group_address, check_no_duplicate_individual_address,
    check_no_duplicate_line_address, ValidationError,
};
```

- [ ] **Step 4: Add the `apply` match arms**

Add after the `Command::DeleteArea { .. }` arm:

```rust
            Command::CreateLine { area, line } => {
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
                Ok(Command::CreateLine { area: area_id, line })
            }
```

If `cargo build` reports a borrow-checker conflict on `area_ref` in
`CreateLine` (holding an immutable borrow of `installation.topology.areas`
across the later mutable `installation.topology.lines.push`), that means
this Rust toolchain's NLL pass didn't shrink the borrow as expected —
fix by capturing what you need out of `area_ref` into a local before the
`lines.push` call, e.g. nothing extra is actually needed since
`check_no_duplicate_line_address` is `area_ref`'s last use; if the
compiler still complains, wrap the lookup+check in its own block
`{ ... }` to force the borrow to end there.

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test -p knx-core command:: 2>&1 | tail -60`
Expected: PASS, all new tests plus every pre-existing test.

- [ ] **Step 6: Commit**

```bash
git add crates/knx-core/src/command.rs
git commit -m "feat(knx-core): Command::CreateLine / DeleteLine"
```

---

### Task 4: `Command::MoveDeviceToLine`

**Files:**
- Modify: `crates/knx-core/src/command.rs`

**Interfaces:**
- Consumes: Task 3's `CommandError::LineNotFound`; pre-existing `CommandError::DeviceNotFound`.
- Produces: no new public interface consumed by a later task in this plan.

- [ ] **Step 1: Write the failing tests**

Add to `command.rs`'s test module:

```rust
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
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p knx-core command:: 2>&1 | tail -40`
Expected: FAIL to compile — `Command::MoveDeviceToLine` does not exist
yet.

- [ ] **Step 3: Add the `Command` variant**

Add to the `Command` enum (after `DeleteLine`):

```rust
    /// Moves a device to `line`, or to `Topology::unassigned` if `None`.
    /// Does not touch `DeviceInstance::address` — a line move and a
    /// re-address are two separate user intents; `SetIndividualAddress`
    /// is the command for the latter.
    MoveDeviceToLine {
        device: DeviceId,
        line: Option<LineId>,
    },
```

- [ ] **Step 4: Add the `apply` match arm**

Add after the `Command::DeleteLine { .. }` arm:

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

Target-line existence is checked *before* the device is removed from its
current location specifically so a `LineNotFound` failure leaves `project`
completely untouched, per this plan's global constraint.

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test -p knx-core command:: 2>&1 | tail -60`
Expected: PASS, all new tests plus every pre-existing test.

- [ ] **Step 6: Commit**

```bash
git add crates/knx-core/src/command.rs
git commit -m "feat(knx-core): Command::MoveDeviceToLine"
```

---

### Task 5: `Command::CreateGroupRange` / `DeleteGroupRange` / `RenameGroupRange`, plus the `CreateGroupAddress` in-range check

**Files:**
- Modify: `crates/knx-core/src/command.rs`

**Interfaces:**
- Consumes: Task 1's `check_group_range_nests_in_parent`, `check_no_overlapping_group_range`; pre-existing `check_group_address_in_range` (never called from here before this task); `crate::group::GroupRange`; `Project::ids::next_group_range_id()`.
- Produces (used by Task 6 only indirectly — no shared interface; used by Task 7/8 as the commands they wire to HTTP): `CommandError::GroupRangeNotFound/NotEmpty/InUse(GroupRangeId)`.

- [ ] **Step 1: Write the failing tests**

Add to `command.rs`'s test module. Add `GroupRange` to the existing
`use crate::group::GroupRange;` test-module import (it is already
imported there per the file's current state — verify, and if not, add
it).

```rust
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
            .do_command(&mut project, Command::CreateGroupRange { range: range.clone() })
            .unwrap();
        assert_eq!(project.installations[0].group_ranges.len(), 1);
        stack
            .do_command(&mut project, Command::DeleteGroupRange { id: GroupRangeId(1) })
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
        assert!(project.installations[0].group_ranges[0]
            .children
            .is_empty());
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
    fn create_group_range_rejects_an_unknown_parent() {
        let mut project = test_project_with_one_device(None);
        let mut stack = CommandStack::new();
        let result = stack.do_command(
            &mut project,
            Command::CreateGroupRange {
                range: test_range(GroupRangeId(1), 0, 255, Some(GroupRangeId(99))),
            },
        );
        assert_eq!(result, Err(CommandError::GroupRangeNotFound(GroupRangeId(99))));
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
        let result =
            stack.do_command(&mut project, Command::DeleteGroupRange { id: GroupRangeId(1) });
        assert_eq!(result, Err(CommandError::GroupRangeNotEmpty(GroupRangeId(1))));
    }

    #[test]
    fn delete_group_range_refuses_when_a_group_address_still_uses_it() {
        let mut project = test_project_with_one_device(None);
        project.installations[0]
            .group_ranges
            .push(test_range(GroupRangeId(1), 0, 2047, None));
        project.installations[0].group_addresses.push(GroupAddressEntry {
            id: GroupAddressId(1),
            source: source(),
            name: "GA".into(),
            address: GroupAddress::from_raw(1),
            central: false,
            unfiltered: false,
            range: Some(GroupRangeId(1)),
        });
        let mut stack = CommandStack::new();
        let result =
            stack.do_command(&mut project, Command::DeleteGroupRange { id: GroupRangeId(1) });
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
        assert_eq!(result, Err(CommandError::GroupRangeNotFound(GroupRangeId(99))));
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
        assert_eq!(result, Err(CommandError::GroupRangeNotFound(GroupRangeId(99))));
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p knx-core command:: 2>&1 | tail -40`
Expected: FAIL to compile — `Command::CreateGroupRange` etc. and
`CommandError::GroupRangeNotFound` etc. do not exist yet.

- [ ] **Step 3: Add imports, `Command`/`CommandError` variants**

Update the top-of-file `use crate::group::GroupAddressEntry;` to:

```rust
use crate::group::{GroupAddressEntry, GroupRange};
```

Update the top-of-file `use crate::ids::{...}` line to add `GroupRangeId`:

```rust
use crate::ids::{AreaId, ComObjectInstanceId, DeviceId, GroupAddressId, GroupRangeId, LineId};
```

Update the `use crate::validation::{...}` block to its final form for
this task:

```rust
use crate::validation::{
    check_group_address_in_range, check_group_range_nests_in_parent, check_no_duplicate_area_address,
    check_no_duplicate_group_address, check_no_duplicate_individual_address,
    check_no_duplicate_line_address, check_no_overlapping_group_range, ValidationError,
};
```

Add to the `Command` enum (after `MoveDeviceToLine`):

```rust
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
```

Add to `CommandError` (after `LineNotEmpty`):

```rust
    GroupRangeNotFound(GroupRangeId),
    /// A `DeleteGroupRange` was refused because it still has nested
    /// (middle) ranges.
    GroupRangeNotEmpty(GroupRangeId),
    /// A `DeleteGroupRange` was refused because at least one group
    /// address still names it as its `range`.
    GroupRangeInUse(GroupRangeId),
```

Add to `impl fmt::Display for CommandError` (after the `LineNotEmpty` arm):

```rust
            CommandError::GroupRangeNotFound(id) => write!(f, "group range {id} not found"),
            CommandError::GroupRangeNotEmpty(id) => {
                write!(f, "group range {id} still has nested ranges, cannot delete")
            }
            CommandError::GroupRangeInUse(id) => write!(
                f,
                "group range {id} still has group addresses assigned to it"
            ),
```

- [ ] **Step 4: Add the `apply` match arms, and modify `CreateGroupAddress`**

Add after the `Command::MoveDeviceToLine { .. }` arm:

```rust
            Command::CreateGroupRange { range } => {
                let installation = project
                    .installations
                    .first_mut()
                    .ok_or(CommandError::InstallationNotFound)?;
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
```

Modify the existing `Command::CreateGroupAddress { entry } => { ... }` arm
(find it — it currently reads `check_no_duplicate_group_address(...)?`
then immediately pushes) to add the range check between those two lines:

```rust
            Command::CreateGroupAddress { entry } => {
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
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test -p knx-core command:: 2>&1 | tail -80`
Expected: PASS, all new tests plus every pre-existing test — including
the pre-existing `create_then_delete_group_address_round_trips_through_undo`
and `create_group_address_rejects_a_duplicate_address_and_leaves_the_stack_untouched`,
whose fixtures use `range: None` and must be unaffected by this change.

- [ ] **Step 6: Commit**

```bash
git add crates/knx-core/src/command.rs
git commit -m "feat(knx-core): Command::CreateGroupRange / DeleteGroupRange / RenameGroupRange; range-check CreateGroupAddress"
```

---

### Task 6: `Command::LinkComObject` / `UnlinkComObject`

**Files:**
- Modify: `crates/knx-core/src/command.rs`

**Interfaces:**
- Consumes: pre-existing `check_group_link_target_exists` (never called from here before this task); `crate::flags::{Direction, GroupLink}`; pre-existing `CommandError::ComObjectNotFound`.
- Produces: `CommandError::LinkAlreadyExists { .. }`, `CommandError::LinkNotFound { .. }`.

- [ ] **Step 1: Write the failing tests**

Add to `command.rs`'s test module:

```rust
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
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p knx-core command:: 2>&1 | tail -40`
Expected: FAIL to compile — `Command::LinkComObject`/`UnlinkComObject`
and the two new `CommandError` variants do not exist yet.

- [ ] **Step 3: Add imports, `Command`/`CommandError` variants**

Add `Direction` and `GroupLink` to the top-of-file imports — since
`crate::flags` is not yet imported at the top of the production code
(only inside the test module today), add a new line:

```rust
use crate::flags::{Direction, GroupLink};
```

Add `check_group_link_target_exists` to the `use crate::validation::{...}`
block, its final form for this plan:

```rust
use crate::validation::{
    check_group_address_in_range, check_group_link_target_exists, check_group_range_nests_in_parent,
    check_no_duplicate_area_address, check_no_duplicate_group_address,
    check_no_duplicate_individual_address, check_no_duplicate_line_address,
    check_no_overlapping_group_range, ValidationError,
};
```

Add to the `Command` enum (after `RenameGroupRange`):

```rust
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
```

Add to `CommandError` (after `GroupRangeInUse`):

```rust
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
```

Add to `impl fmt::Display for CommandError` (after the `GroupRangeInUse` arm):

```rust
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
```

- [ ] **Step 4: Add the `apply` match arms**

Add after the `Command::RenameGroupRange { .. }` arm:

```rust
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
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test -p knx-core command:: 2>&1 | tail -80`
Expected: PASS, all new tests plus every pre-existing test.

- [ ] **Step 6: Run the full `knx-core` suite**

Run: `cargo test -p knx-core 2>&1 | tail -20`
Expected: PASS — every test in the crate, not just `command::`/
`validation::`, confirming Tasks 1-6 haven't regressed anything in
`knx-core`'s other modules (`devices.rs`, `installation.rs`, etc.).

- [ ] **Step 7: Commit**

```bash
git add crates/knx-core/src/command.rs
git commit -m "feat(knx-core): Command::LinkComObject / UnlinkComObject"
```

---

### Task 7: `knx-projection` — minimal `GroupRangeNode`

**Files:**
- Modify: `crates/knx-projection/src/lib.rs`
- Create (generated by `cargo test`, then committed): `apps/knx-web/src/bindings/GroupRangeNode.ts`

**Interfaces:**
- Consumes: `knx_core::GroupRange` (pre-existing), `project.info.group_address_style` (pre-existing, already threaded through `build_installation`).
- Produces (used by Task 8's HTTP route tests): `InstallationNode.group_ranges: Vec<GroupRangeNode>`, `GroupRangeNode { id: u32, name: String, start: String, end: String, parent: Option<u32> }` — the only way an HTTP caller (route handler or test) can discover a newly-created group range's id, since `GroupAddressNode`'s flat list has no range information and there is no other read endpoint for ranges. This is a small, additive field — **not** the fuller "nest addresses inside their range in the tree" redesign the design spec explicitly defers; that stays deferred.

- [ ] **Step 1: Write the failing test**

Add to `crates/knx-projection/src/lib.rs`'s existing `#[cfg(test)] mod
tests` block (check the bottom of the file for where it lives and what
it already imports/builds — follow that file's existing fixture-building
pattern for a `Project`/`Installation`, the same shape `command.rs`'s and
`http_edit_routes.rs`'s fixtures use):

```rust
    #[test]
    fn group_ranges_are_projected_with_their_parent_link() {
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
            group_ranges: vec![
                knx_core::GroupRange {
                    id: knx_core::GroupRangeId(1),
                    source: knx_core::SourceRef {
                        path: "t".into(),
                        ets_id: "t".into(),
                    },
                    name: "Main".into(),
                    start: knx_core::GroupAddress::from_raw(0),
                    end: knx_core::GroupAddress::from_raw(2047),
                    parent: None,
                    children: vec![knx_core::GroupRangeId(2)],
                },
                knx_core::GroupRange {
                    id: knx_core::GroupRangeId(2),
                    source: knx_core::SourceRef {
                        path: "t".into(),
                        ets_id: "t".into(),
                    },
                    name: "Middle".into(),
                    start: knx_core::GroupAddress::from_raw(0),
                    end: knx_core::GroupAddress::from_raw(255),
                    parent: Some(knx_core::GroupRangeId(1)),
                    children: vec![],
                },
            ],
            group_addresses: vec![],
            parameters: vec![],
        });
        let tree = build_project_tree(&project);
        let ranges = &tree.installations[0].group_ranges;
        assert_eq!(ranges.len(), 2);
        assert_eq!(ranges[0].id, 1);
        assert_eq!(ranges[0].parent, None);
        assert_eq!(ranges[1].parent, Some(1));
    }
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test -p knx-projection group_ranges_are_projected 2>&1 | tail -30`
Expected: FAIL to compile — `InstallationNode` has no `group_ranges`
field yet.

- [ ] **Step 3: Add `GroupRangeNode` and the `InstallationNode` field**

In `crates/knx-projection/src/lib.rs`, add `GroupRange` to the top-of-file
`use knx_core::{...}` import:

```rust
use knx_core::{
    BuildingPart, BuildingPartId, BuildingPartType, Devices, GroupAddressEntry, GroupAddressStyle,
    GroupRange, Project, Topology,
};
```

Add `pub group_ranges: Vec<GroupRangeNode>` to `InstallationNode` (after
`group_addresses`):

```rust
pub struct InstallationNode {
    pub id: u8,
    pub name: String,
    pub topology: Vec<AreaNode>,
    pub buildings: Vec<BuildingNode>,
    pub unassigned: Vec<DeviceNode>,
    pub group_addresses: Vec<GroupAddressNode>,
    pub group_ranges: Vec<GroupRangeNode>,
}
```

Add the new type (after `GroupAddressNode`):

```rust
/// A flat (not nested) view of one `GroupRange` — `parent` names the
/// containing main range's id for a middle range, `None` for a main
/// range. Deliberately does not nest `GroupAddressNode`s inside their
/// range: `InstallationNode.group_addresses` stays a flat list, matching
/// its existing shape, until a future cycle redesigns the group-address
/// tree branch around the real main/middle/address hierarchy.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct GroupRangeNode {
    pub id: u32,
    pub name: String,
    /// Formatted per the project's own `GroupAddressStyle`, same as
    /// `GroupAddressNode::address`.
    pub start: String,
    pub end: String,
    pub parent: Option<u32>,
}
```

- [ ] **Step 4: Populate it in `build_installation`**

Add a `build_group_range_node` function (after `build_group_address_node`):

```rust
fn build_group_range_node(range: &GroupRange, style: GroupAddressStyle) -> GroupRangeNode {
    GroupRangeNode {
        id: range.id.0,
        name: range.name.clone(),
        start: range.start.format(style),
        end: range.end.format(style),
        parent: range.parent.map(|p| p.0),
    }
}
```

Update `build_installation` to populate the new field (add after the
`group_addresses:` field):

```rust
        group_addresses: inst
            .group_addresses
            .iter()
            .map(|entry| build_group_address_node(entry, ga_style))
            .collect(),
        group_ranges: inst
            .group_ranges
            .iter()
            .map(|range| build_group_range_node(range, ga_style))
            .collect(),
```

- [ ] **Step 5: Run the test to verify it passes**

Run: `cargo test -p knx-projection 2>&1 | tail -30`
Expected: PASS — the new test, plus every pre-existing `knx-projection`
test (confirming `group_ranges: vec![]` on every existing fixture that
constructs an `InstallationNode`/`Installation` without group ranges
still compiles and produces an empty `Vec`, not a compile error from a
missing struct field — if any existing test constructs an
`InstallationNode` literal directly rather than via `build_project_tree`,
it needs `group_ranges: vec![]` added too; check for any such literal
before assuming only `build_installation` needed changes).

- [ ] **Step 6: Regenerate and commit the `ts-rs` binding**

`#[ts(export)]` writes `apps/knx-web/src/bindings/GroupRangeNode.ts` (and
updates `InstallationNode.ts`) as a side effect of running the crate's
tests — this is the same mechanism that produced every existing file
under `apps/knx-web/src/bindings/`.

Run: `cargo test -p knx-projection`
Then check what changed: `git status apps/knx-web/src/bindings/`
Expected: a new `GroupRangeNode.ts` and a modified `InstallationNode.ts`
(gains a `group_ranges: GroupRangeNode[]` field).

```bash
git add crates/knx-projection/src/lib.rs apps/knx-web/src/bindings/GroupRangeNode.ts apps/knx-web/src/bindings/InstallationNode.ts
git commit -m "feat(knx-projection): add GroupRangeNode to InstallationNode"
```

---

### Task 8: `apps/knx-server` — routes for all nine commands, plus the `create_group_address_impl` synthetic-id fix

**Files:**
- Modify: `apps/knx-server/src/domain.rs`
- Modify: `apps/knx-server/src/routes.rs`
- Modify: `apps/knx-server/tests/http_edit_routes.rs`

**Interfaces:**
- Consumes: every `Command` variant from Tasks 2-6; `GroupRangeNode`/`InstallationNode.group_ranges` from Task 7 (route tests read a new range's id from there); the pre-existing `apply(state, cmd) -> Result<ProjectTree, String>` helper and `AppState` (both in `domain.rs`, unchanged).
- Produces: nothing consumed by a later task — this is the plan's last task before docs/verification.

- [ ] **Step 1: Write the failing tests**

Add to `apps/knx-server/tests/http_edit_routes.rs`. First, extend its
top-of-file `use knx_core::{...}` import to add what the new fixture
helper needs:

```rust
use knx_core::{
    CommissioningState, CompletionStatus, DeviceId, DeviceInstance, Installation, InstallationId,
    Language, Project, SourceRef, Topology,
};
```

Add a second fixture helper (after `state_with_one_installation`):

```rust
fn state_with_one_installation_and_device() -> knx_server::AppState {
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
        group_addresses: vec![],
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
        com_objects: vec![],
        binary_data: vec![],
    });
    let state = knx_server::AppState::default();
    *state.project.lock().unwrap() = Some(project);
    state
}
```

Add the route tests:

```rust
#[tokio::test]
async fn creating_then_deleting_an_area_round_trips() {
    let state = Arc::new(state_with_one_installation());
    let app = knx_server::app(state, None);

    let create = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/areas")
                .header("content-type", "application/json")
                .body(Body::from(json!({ "name": "Area 1", "address": 1 }).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(create.status(), StatusCode::OK);
    let tree = body_json(create).await;
    let area_id = tree["installations"][0]["topology"][0]["id"]
        .as_u64()
        .unwrap();

    let delete = app
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/areas/{area_id}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(delete.status(), StatusCode::OK);
    let tree = body_json(delete).await;
    assert!(tree["installations"][0]["topology"]
        .as_array()
        .unwrap()
        .is_empty());
}

#[tokio::test]
async fn deleting_a_nonempty_area_is_a_400() {
    let state = Arc::new(state_with_one_installation());
    let app = knx_server::app(state, None);

    let area_create = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/areas")
                .header("content-type", "application/json")
                .body(Body::from(json!({ "name": "Area 1", "address": 1 }).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    let tree = body_json(area_create).await;
    let area_id = tree["installations"][0]["topology"][0]["id"]
        .as_u64()
        .unwrap();

    app.clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/lines")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "areaId": area_id,
                        "name": "Line 1",
                        "address": 1,
                        "mediumRef": "MT-0"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    let delete = app
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/areas/{area_id}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(delete.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn creating_a_line_nests_it_under_its_area_then_deletes() {
    let state = Arc::new(state_with_one_installation());
    let app = knx_server::app(state, None);

    let area_create = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/areas")
                .header("content-type", "application/json")
                .body(Body::from(json!({ "name": "Area 1", "address": 1 }).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    let tree = body_json(area_create).await;
    let area_id = tree["installations"][0]["topology"][0]["id"]
        .as_u64()
        .unwrap();

    let line_create = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/lines")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "areaId": area_id,
                        "name": "Line 1",
                        "address": 1,
                        "mediumRef": "MT-0"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(line_create.status(), StatusCode::OK);
    let tree = body_json(line_create).await;
    let lines = tree["installations"][0]["topology"][0]["lines"]
        .as_array()
        .unwrap();
    assert_eq!(lines.len(), 1);
    let line_id = lines[0]["id"].as_u64().unwrap();

    let delete = app
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/lines/{line_id}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(delete.status(), StatusCode::OK);
    let tree = body_json(delete).await;
    assert!(tree["installations"][0]["topology"][0]["lines"]
        .as_array()
        .unwrap()
        .is_empty());
}

#[tokio::test]
async fn moving_a_device_between_unassigned_and_a_line() {
    let state = Arc::new(state_with_one_installation_and_device());
    let app = knx_server::app(state, None);

    let area_create = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/areas")
                .header("content-type", "application/json")
                .body(Body::from(json!({ "name": "Area 1", "address": 1 }).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    let tree = body_json(area_create).await;
    let area_id = tree["installations"][0]["topology"][0]["id"]
        .as_u64()
        .unwrap();

    let line_create = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/lines")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "areaId": area_id,
                        "name": "Line 1",
                        "address": 1,
                        "mediumRef": "MT-0"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    let tree = body_json(line_create).await;
    let line_id = tree["installations"][0]["topology"][0]["lines"][0]["id"]
        .as_u64()
        .unwrap();

    let moved = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/move-device")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({ "deviceId": 1, "lineId": line_id }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(moved.status(), StatusCode::OK);
    let tree = body_json(moved).await;
    assert!(tree["installations"][0]["unassigned"]
        .as_array()
        .unwrap()
        .is_empty());
    assert_eq!(
        tree["installations"][0]["topology"][0]["lines"][0]["devices"]
            .as_array()
            .unwrap()
            .len(),
        1
    );

    let back = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/move-device")
                .header("content-type", "application/json")
                .body(Body::from(json!({ "deviceId": 1, "lineId": null }).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(back.status(), StatusCode::OK);
    let tree = body_json(back).await;
    assert_eq!(
        tree["installations"][0]["unassigned"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
}

#[tokio::test]
async fn creating_a_nested_group_range_then_renaming_and_deleting_it() {
    let state = Arc::new(state_with_one_installation());
    let app = knx_server::app(state, None);

    let main = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/group-ranges")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({ "name": "Main", "start": "0/0/0", "end": "0/7/255" }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(main.status(), StatusCode::OK);
    let tree = body_json(main).await;
    let main_id = tree["installations"][0]["group_ranges"][0]["id"]
        .as_u64()
        .unwrap();

    let middle = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/group-ranges")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "name": "Middle",
                        "start": "0/0/0",
                        "end": "0/0/255",
                        "parentId": main_id
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(middle.status(), StatusCode::OK);
    let tree = body_json(middle).await;
    let ranges = tree["installations"][0]["group_ranges"]
        .as_array()
        .unwrap();
    assert_eq!(ranges.len(), 2);
    let middle_id = ranges
        .iter()
        .find(|r| r["parent"].as_u64() == Some(main_id))
        .unwrap()["id"]
        .as_u64()
        .unwrap();

    let renamed = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri(format!("/api/group-ranges/{middle_id}"))
                .header("content-type", "application/json")
                .body(Body::from(json!({ "name": "Renamed" }).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(renamed.status(), StatusCode::OK);
    let tree = body_json(renamed).await;
    let renamed_range = tree["installations"][0]["group_ranges"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["id"].as_u64() == Some(middle_id))
        .unwrap();
    assert_eq!(renamed_range["name"], "Renamed");

    let delete_middle = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/group-ranges/{middle_id}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(delete_middle.status(), StatusCode::OK);

    let delete_main = app
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/group-ranges/{main_id}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(delete_main.status(), StatusCode::OK);
    let tree = body_json(delete_main).await;
    assert!(tree["installations"][0]["group_ranges"]
        .as_array()
        .unwrap()
        .is_empty());
}

#[tokio::test]
async fn creating_a_group_address_without_a_range_still_works_unchanged() {
    // Regression guard: the shipped frontend (Session 5 cycle 9) never
    // sends `rangeId` — this must keep working exactly as before.
    let state = Arc::new(state_with_one_installation());
    let app = knx_server::app(state, None);

    let create = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/group-addresses")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({ "name": "Living room", "address": "1/1/1" }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(create.status(), StatusCode::OK);
}

#[tokio::test]
async fn creating_a_group_address_with_a_range_id_validates_it_falls_inside() {
    let state = Arc::new(state_with_one_installation());
    let app = knx_server::app(state, None);

    let range_create = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/group-ranges")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({ "name": "Main", "start": "0/0/0", "end": "0/7/255" }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    let tree = body_json(range_create).await;
    let range_id = tree["installations"][0]["group_ranges"][0]["id"]
        .as_u64()
        .unwrap();

    let inside = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/group-addresses")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({ "name": "GA", "address": "0/0/1", "rangeId": range_id }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(inside.status(), StatusCode::OK);

    let outside = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/group-addresses")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({ "name": "GA2", "address": "5/0/1", "rangeId": range_id }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(outside.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn linking_then_unlinking_a_com_object_to_a_group_address() {
    // No device-creation route exists yet (Sub-Project 2), so this test
    // seeds a com object directly the same way command.rs's own fixtures
    // do, via a raw sqlite-free in-memory Project built by hand.
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
        group_addresses: vec![knx_core::GroupAddressEntry {
            id: knx_core::GroupAddressId(1),
            source: SourceRef {
                path: "t".into(),
                ets_id: "t".into(),
            },
            name: "GA".into(),
            address: knx_core::GroupAddress::from_raw(1),
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
        text: knx_core::Override::Absent,
        description: knx_core::Override::Absent,
        dpt: knx_core::Override::Absent,
        flags: knx_core::ResolvedFlags::none(),
        size: None,
        is_active: true,
        links: vec![],
    });
    let state = knx_server::AppState::default();
    *state.project.lock().unwrap() = Some(project);
    let app = knx_server::app(Arc::new(state), None);

    let link = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/group-links")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({ "comObjectId": 1, "gaId": 1, "direction": "Send" }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(link.status(), StatusCode::OK);

    let unlink = app
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri("/api/group-links")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({ "comObjectId": 1, "gaId": 1, "direction": "Send" }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(unlink.status(), StatusCode::OK);
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p knx-server --test http_edit_routes 2>&1 | tail -60`
Expected: FAIL to compile — none of the new routes/`_impl` functions
exist yet, and `ResolvedFlags`/`Override`/`ComObjectInstance`/
`ComObjectInstanceId`/`GroupAddressEntry`/`GroupAddressId`/`GroupAddress`
need to be reachable via `knx_core::` (they already are, as public
re-exports from `knx-core`'s crate root — confirm with `cargo doc -p
knx-core --no-deps` or by grepping `crates/knx-core/src/lib.rs`'s `pub
use` list if the compile error says otherwise).

- [ ] **Step 3: Add the nine `_impl` functions and fix `create_group_address_impl`**

In `apps/knx-server/src/domain.rs`, replace the existing
`create_group_address_impl` (keep `delete_group_address_impl` as-is
immediately after it) with:

```rust
/// Allocates a fresh `GroupAddressId` and creates a new group address in
/// `installations[0]` — the only installation any `Command` targets
/// (`Command::apply`'s own doc comment). `address` is parsed against the
/// project's own `GroupAddressStyle`. `entry.source` gets a synthetic,
/// stable id (`KB-GA-<id>`) instead of the empty string this used to
/// write — a UI-created entity has no ETS origin to preserve, but an
/// empty `ets_id` produced an invalid, colliding `Id=""` attribute if it
/// ever reached export (KNOWN_LIMITATIONS.md #21). `range_id` stays
/// optional: forcing every UI-created address into a range needs a range
/// *picker* in the UI, which does not exist yet (Sub-Project 2) — until
/// then, a `None` range keeps behaving exactly as before, and a `Some`
/// range is now validated (`Command::CreateGroupAddress`'s own
/// in-range check) rather than trusted blindly.
pub fn create_group_address_impl(
    state: &AppState,
    name: String,
    address: String,
    range_id: Option<u32>,
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
                    path: format!("KB-GA-{}", id.0),
                    ets_id: format!("KB-GA-{}", id.0),
                },
                name,
                address,
                central: false,
                unfiltered: false,
                range: range_id.map(knx_core::GroupRangeId),
            },
        }
    };
    apply(state, cmd)
}
```

Add the nine new functions after `delete_group_address_impl` (before
`undo_impl`):

```rust
pub fn create_area_impl(
    state: &AppState,
    name: String,
    address: u8,
) -> Result<knx_projection::ProjectTree, String> {
    let cmd = {
        let mut project = state.project.lock().expect("state mutex poisoned");
        let project = project.as_mut().ok_or("no project open")?;
        let id = project.ids.next_area_id();
        knx_core::Command::CreateArea {
            area: knx_core::Area {
                id,
                source: knx_core::SourceRef {
                    path: format!("KB-Area-{}", id.0),
                    ets_id: format!("KB-Area-{}", id.0),
                },
                name,
                address,
                completion: knx_core::CompletionStatus::Editing,
                lines: vec![],
            },
        }
    };
    apply(state, cmd)
}

pub fn delete_area_impl(state: &AppState, id: u32) -> Result<knx_projection::ProjectTree, String> {
    apply(state, knx_core::Command::DeleteArea { id: knx_core::AreaId(id) })
}

pub fn create_line_impl(
    state: &AppState,
    area_id: u32,
    name: String,
    address: u8,
    medium_ref: String,
) -> Result<knx_projection::ProjectTree, String> {
    let cmd = {
        let mut project = state.project.lock().expect("state mutex poisoned");
        let project = project.as_mut().ok_or("no project open")?;
        let id = project.ids.next_line_id();
        knx_core::Command::CreateLine {
            area: knx_core::AreaId(area_id),
            line: knx_core::Line {
                id,
                source: knx_core::SourceRef {
                    path: format!("KB-Line-{}", id.0),
                    ets_id: format!("KB-Line-{}", id.0),
                },
                name,
                address,
                medium_ref,
                domain_address: None,
                domain_address_is_checked: None,
                ip_routing_multicast_address: None,
                multicast_ttl: None,
                completion: knx_core::CompletionStatus::Editing,
                devices: vec![],
            },
        }
    };
    apply(state, cmd)
}

pub fn delete_line_impl(state: &AppState, id: u32) -> Result<knx_projection::ProjectTree, String> {
    apply(state, knx_core::Command::DeleteLine { id: knx_core::LineId(id) })
}

pub fn move_device_to_line_impl(
    state: &AppState,
    device_id: u32,
    line_id: Option<u32>,
) -> Result<knx_projection::ProjectTree, String> {
    apply(
        state,
        knx_core::Command::MoveDeviceToLine {
            device: knx_core::DeviceId(device_id),
            line: line_id.map(knx_core::LineId),
        },
    )
}

pub fn create_group_range_impl(
    state: &AppState,
    name: String,
    start: String,
    end: String,
    parent_id: Option<u32>,
) -> Result<knx_projection::ProjectTree, String> {
    let cmd = {
        let mut project = state.project.lock().expect("state mutex poisoned");
        let project = project.as_mut().ok_or("no project open")?;
        let style = project.info.group_address_style;
        let start = knx_core::GroupAddress::parse(&start, style).map_err(|e| e.to_string())?;
        let end = knx_core::GroupAddress::parse(&end, style).map_err(|e| e.to_string())?;
        let id = project.ids.next_group_range_id();
        knx_core::Command::CreateGroupRange {
            range: knx_core::GroupRange {
                id,
                source: knx_core::SourceRef {
                    path: format!("KB-Range-{}", id.0),
                    ets_id: format!("KB-Range-{}", id.0),
                },
                name,
                start,
                end,
                parent: parent_id.map(knx_core::GroupRangeId),
                children: vec![],
            },
        }
    };
    apply(state, cmd)
}

pub fn delete_group_range_impl(
    state: &AppState,
    id: u32,
) -> Result<knx_projection::ProjectTree, String> {
    apply(
        state,
        knx_core::Command::DeleteGroupRange { id: knx_core::GroupRangeId(id) },
    )
}

pub fn rename_group_range_impl(
    state: &AppState,
    id: u32,
    name: String,
) -> Result<knx_projection::ProjectTree, String> {
    apply(
        state,
        knx_core::Command::RenameGroupRange { id: knx_core::GroupRangeId(id), name },
    )
}

fn parse_direction(direction: &str) -> Result<knx_core::Direction, String> {
    match direction {
        "Send" => Ok(knx_core::Direction::Send),
        "Receive" => Ok(knx_core::Direction::Receive),
        other => Err(format!("unknown direction '{other}', expected 'Send' or 'Receive'")),
    }
}

pub fn link_com_object_impl(
    state: &AppState,
    com_object_id: u32,
    ga_id: u32,
    direction: String,
) -> Result<knx_projection::ProjectTree, String> {
    let direction = parse_direction(&direction)?;
    apply(
        state,
        knx_core::Command::LinkComObject {
            com_object: knx_core::ComObjectInstanceId(com_object_id),
            ga: knx_core::GroupAddressId(ga_id),
            direction,
        },
    )
}

pub fn unlink_com_object_impl(
    state: &AppState,
    com_object_id: u32,
    ga_id: u32,
    direction: String,
) -> Result<knx_projection::ProjectTree, String> {
    let direction = parse_direction(&direction)?;
    apply(
        state,
        knx_core::Command::UnlinkComObject {
            com_object: knx_core::ComObjectInstanceId(com_object_id),
            ga: knx_core::GroupAddressId(ga_id),
            direction,
        },
    )
}
```

If `knx_core::Area`/`Line`/`GroupRange`/`Direction`/`AreaId`/`LineId`/
`GroupRangeId` are not already re-exported from `knx-core`'s crate root
(`crates/knx-core/src/lib.rs`), `cargo build` will report an unresolved
path here — check that file's `pub use` list and add whatever is missing
to it, following its existing pattern exactly (every other type
`domain.rs` already references via `knx_core::X`, e.g. `SourceRef`,
`CompletionStatus`, is re-exported there the same way).

- [ ] **Step 4: Wire the routes**

In `apps/knx-server/src/routes.rs`, add `axum::routing::patch` to the
top-of-file import:

```rust
use axum::routing::{delete, patch, post};
```

Update the `CreateGroupAddressBody` struct and its handler:

```rust
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateGroupAddressBody {
    name: String,
    address: String,
    #[serde(default)]
    range_id: Option<u32>,
}

async fn create_group_address(
    State(state): State<SharedState>,
    Json(body): Json<CreateGroupAddressBody>,
) -> Result<Json<knx_projection::ProjectTree>, ApiError> {
    domain::create_group_address_impl(&state, body.name, body.address, body.range_id)
        .map(Json)
        .map_err(ApiError::bad_request)
}
```

Add the new route registrations to `project_routes()`'s `Router::new()`
chain (after the existing `group-addresses` routes, before `/api/undo`):

```rust
        .route("/api/areas", post(create_area))
        .route("/api/areas/{id}", delete(delete_area))
        .route("/api/lines", post(create_line))
        .route("/api/lines/{id}", delete(delete_line))
        .route("/api/move-device", post(move_device_to_line))
        .route("/api/group-ranges", post(create_group_range))
        .route(
            "/api/group-ranges/{id}",
            delete(delete_group_range).patch(rename_group_range),
        )
        .route(
            "/api/group-links",
            post(link_com_object).delete(unlink_com_object),
        )
```

Add the new request-body structs and handler functions (anywhere after
`delete_group_address`, before `undo`):

```rust
#[derive(Deserialize)]
struct CreateAreaBody {
    name: String,
    address: u8,
}

async fn create_area(
    State(state): State<SharedState>,
    Json(body): Json<CreateAreaBody>,
) -> Result<Json<knx_projection::ProjectTree>, ApiError> {
    domain::create_area_impl(&state, body.name, body.address)
        .map(Json)
        .map_err(ApiError::bad_request)
}

async fn delete_area(
    State(state): State<SharedState>,
    AxumPath(id): AxumPath<u32>,
) -> Result<Json<knx_projection::ProjectTree>, ApiError> {
    domain::delete_area_impl(&state, id)
        .map(Json)
        .map_err(ApiError::bad_request)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateLineBody {
    area_id: u32,
    name: String,
    address: u8,
    medium_ref: String,
}

async fn create_line(
    State(state): State<SharedState>,
    Json(body): Json<CreateLineBody>,
) -> Result<Json<knx_projection::ProjectTree>, ApiError> {
    domain::create_line_impl(&state, body.area_id, body.name, body.address, body.medium_ref)
        .map(Json)
        .map_err(ApiError::bad_request)
}

async fn delete_line(
    State(state): State<SharedState>,
    AxumPath(id): AxumPath<u32>,
) -> Result<Json<knx_projection::ProjectTree>, ApiError> {
    domain::delete_line_impl(&state, id)
        .map(Json)
        .map_err(ApiError::bad_request)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct MoveDeviceBody {
    device_id: u32,
    line_id: Option<u32>,
}

async fn move_device_to_line(
    State(state): State<SharedState>,
    Json(body): Json<MoveDeviceBody>,
) -> Result<Json<knx_projection::ProjectTree>, ApiError> {
    domain::move_device_to_line_impl(&state, body.device_id, body.line_id)
        .map(Json)
        .map_err(ApiError::bad_request)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateGroupRangeBody {
    name: String,
    start: String,
    end: String,
    #[serde(default)]
    parent_id: Option<u32>,
}

async fn create_group_range(
    State(state): State<SharedState>,
    Json(body): Json<CreateGroupRangeBody>,
) -> Result<Json<knx_projection::ProjectTree>, ApiError> {
    domain::create_group_range_impl(&state, body.name, body.start, body.end, body.parent_id)
        .map(Json)
        .map_err(ApiError::bad_request)
}

async fn delete_group_range(
    State(state): State<SharedState>,
    AxumPath(id): AxumPath<u32>,
) -> Result<Json<knx_projection::ProjectTree>, ApiError> {
    domain::delete_group_range_impl(&state, id)
        .map(Json)
        .map_err(ApiError::bad_request)
}

#[derive(Deserialize)]
struct RenameGroupRangeBody {
    name: String,
}

async fn rename_group_range(
    State(state): State<SharedState>,
    AxumPath(id): AxumPath<u32>,
    Json(body): Json<RenameGroupRangeBody>,
) -> Result<Json<knx_projection::ProjectTree>, ApiError> {
    domain::rename_group_range_impl(&state, id, body.name)
        .map(Json)
        .map_err(ApiError::bad_request)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct GroupLinkBody {
    com_object_id: u32,
    ga_id: u32,
    direction: String,
}

async fn link_com_object(
    State(state): State<SharedState>,
    Json(body): Json<GroupLinkBody>,
) -> Result<Json<knx_projection::ProjectTree>, ApiError> {
    domain::link_com_object_impl(&state, body.com_object_id, body.ga_id, body.direction)
        .map(Json)
        .map_err(ApiError::bad_request)
}

async fn unlink_com_object(
    State(state): State<SharedState>,
    Json(body): Json<GroupLinkBody>,
) -> Result<Json<knx_projection::ProjectTree>, ApiError> {
    domain::unlink_com_object_impl(&state, body.com_object_id, body.ga_id, body.direction)
        .map(Json)
        .map_err(ApiError::bad_request)
}
```

- [ ] **Step 5: Run the tests to verify they pass**

Run: `cargo test -p knx-server 2>&1 | tail -100`
Expected: PASS — every new test in `http_edit_routes.rs`, plus every
pre-existing `knx-server` test in every other test file
(`healthz.rs`, `http_device_detail.rs`, `command_dispatch.rs`,
`device_detail.rs`, `open_reference_project.rs`,
`save_load_roundtrip.rs`, `http_fs_routes.rs`, `http_project_routes.rs`)
— none of those touch the routes this task changed, so this run is
confirming no accidental regression, not just the new coverage.

- [ ] **Step 6: Commit**

```bash
git add apps/knx-server/src/domain.rs apps/knx-server/src/routes.rs apps/knx-server/tests/http_edit_routes.rs
git commit -m "feat(knx-server): HTTP routes for topology, group-range, and group-link commands"
```

---

### Task 9: Full workspace verification and documentation

**Files:**
- Modify: `docs/IMPLEMENTATION_STATUS.md`
- Modify: `docs/KNOWN_LIMITATIONS.md`
- Modify: `docs/GAP_ANALYSIS_ETS.md`

**Interfaces:** None — this task touches no code.

- [ ] **Step 1: Run the full verification suite**

```bash
cargo build --workspace
cargo test --workspace
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo run -p xtask -- check-layering
cargo deny check
```

Expected: every command exits 0. Fix anything that doesn't before
proceeding — `cargo fmt --all` (without `--check`) to auto-fix
formatting, then re-run `--check`; address any `clippy` finding directly
in the file it names.

- [ ] **Step 2: Update `docs/KNOWN_LIMITATIONS.md` §21**

Find entry "21. A UI-created group address has no `ets_id` and is dropped
on export". Replace its content to reflect partial resolution — the
synthetic id is fixed, the range requirement is not, by design (see
"Global Constraints" above):

```markdown
## 21. A UI-created group address without a range is still dropped on export — partially resolved

**Partially resolved.** `create_group_address_impl`
(`apps/knx-server/src/domain.rs`) now writes a synthetic, stable
`ets_id`/`path` (`KB-GA-<id>`) instead of the empty string it used to —
the "colliding `Id=""` attribute if export were ever wired up" half of
this limitation is fixed regardless of whether a range is given.

**Still open.** `range_id` stays optional at the HTTP boundary — a
UI-created group address with no range assigned is still silently
omitted by `crates/knx-etsproj/src/export/schema11.rs`'s exporter, which
only emits a group address nested inside its `GroupRange`. Forcing every
creation through a range needs a range *picker* in the UI, which does not
exist yet; `knx_core::Command::CreateGroupRange` (this cycle) makes
ranges creatable, but the frontend has no screen to create or choose one
from. `Command::CreateGroupAddress` now validates a *given* range
(`check_group_address_in_range`), so a range, once chosen, cannot
disagree with the address — only the choice itself isn't enforced yet.

**Originally.** [as before — the empty-`ets_id`/`range: None` behavior
this entry first documented].

**Lifted when.** The frontend gains a group-range create/pick UI
(Sub-Project 2 or later, see
[GAP_ANALYSIS_ETS.md](GAP_ANALYSIS_ETS.md)) and `range_id` becomes a
required argument to group-address creation at that point — not before,
since making it required today would break the already-shipped
range-less creation UI with nothing to replace it.
```

- [ ] **Step 3: Update `docs/IMPLEMENTATION_STATUS.md`**

Add a new paragraph after the most recent Session 6 entry (before "##
Next session"), describing what landed — follow the file's existing
prose style (see the Session 6 Cycle 1-5 paragraphs immediately above for
the level of detail and tone expected):

```markdown
**Topology & group-range command layer (2026-09-06).** `knx-core::command`
gains nine `Command` variants closing the first item of
[GAP_ANALYSIS_ETS.md](GAP_ANALYSIS_ETS.md)'s Tier 1 backlog: `CreateArea`/
`DeleteArea`, `CreateLine`/`DeleteLine`, `MoveDeviceToLine`,
`CreateGroupRange`/`DeleteGroupRange`/`RenameGroupRange`, and
`LinkComObject`/`UnlinkComObject` — each following `CreateGroupAddress`/
`DeleteGroupAddress`'s existing shape exactly (caller pre-allocates the id
via `Project::ids`, `apply` returns its own inverse). Two validation
functions that existed but were never called from any command
(`check_group_address_in_range`, `check_group_link_target_exists`) are
finally wired up — the latter into the two new link commands, the former
into both `CreateGroupRange` and, as a previously-missing check on
existing behavior, `CreateGroupAddress` itself. Four new `validation.rs`
functions close the remaining gaps: duplicate area/line address
(line addresses unique per area, not project-wide, matching ETS's
Area.Line.Device numbering) and group-range nesting/overlap. All nine
commands are backend-only this cycle: `apps/knx-server` gains one route
each (`/api/areas`, `/api/lines`, `/api/move-device`, `/api/group-ranges`,
`/api/group-links`), but no frontend UI exists for any of them yet — see
[the design spec](superpowers/specs/2026-09-06-topology-group-range-commands-design.md)
for the deliberate scope cut (`knx-projection`'s `GroupAddressNode` has no
real main/middle/address nesting yet; that redesign is its own future
cycle). `knx-projection` gains a small, additive `GroupRangeNode`
(id/name/start/end/parent) on `InstallationNode` — not the fuller nesting
redesign, just enough for an HTTP caller to discover a newly-created
range's id. `sync_after_command` gains no new incremental-sync paths for
any of the nine — same as every command since Session 5 cycle 2 that
hasn't gotten one yet, correct via a full `save_project`/`load_project`
round trip until a later cycle's incremental-sync pass covers all of
them together. [count] Rust tests added across
`crates/knx-core`/`crates/knx-projection`/`apps/knx-server`.
```

(Replace `[count]` with the actual number of new tests added across
Tasks 1-8 — count them by running `cargo test --workspace 2>&1 | grep
"test result:"` before and after this plan's changes, or simply sum the
`#[test]` functions added in Tasks 1, 2, 3, 4, 5, 6, 7, 8's steps above.)

- [ ] **Step 4: Update `docs/GAP_ANALYSIS_ETS.md`'s task backlog**

Find the "Tier 1" section. Mark T4, T5, T6 as done, each with a one-line
pointer to what shipped and what's still open:

```markdown
- **T4. Topology CRUD commands.** **Done** (2026-09-06, backend only —
  see [IMPLEMENTATION_STATUS.md](IMPLEMENTATION_STATUS.md)). `CreateArea`/
  `DeleteArea`/`CreateLine`/`DeleteLine`/`MoveDeviceToLine` land as
  `knx-core` commands with `apps/knx-server` routes; no frontend UI yet.
- **T5. Group-range CRUD commands.** **Done** (2026-09-06, backend only).
  `CreateGroupRange`/`DeleteGroupRange`/`RenameGroupRange` land; the
  export-drop bug ([KNOWN_LIMITATIONS.md §21](../../KNOWN_LIMITATIONS.md#21-a-ui-created-group-address-without-a-range-is-still-dropped-on-export--partially-resolved))
  is only partially closed — see that entry for why `range_id` stays
  optional until a UI exists to pick one.
- **T6. Group-link editing command.** **Done** (2026-09-06, backend
  only). `LinkComObject`/`UnlinkComObject` land, finally calling the
  validation.rs function that already existed for this
  (`check_group_link_target_exists`).
```

- [ ] **Step 5: Commit**

```bash
git add docs/IMPLEMENTATION_STATUS.md docs/KNOWN_LIMITATIONS.md docs/GAP_ANALYSIS_ETS.md
git commit -m "docs: record topology/group-range/group-link command layer"
```

---

## Self-review notes (from plan authoring, not a task to execute)

- **Spec coverage:** every apply-semantics bullet in the design spec has
  a task (Tasks 2-6); the synthetic-id section is Task 8; testing is
  folded into each task's own steps rather than a separate task, per the
  spec's own testing section listing the same breakdown.
- **Deviation recorded:** the required-`range_id` deviation is called out
  in "Global Constraints" and again in Task 8/9 — the spec itself is not
  edited (its historical record stays accurate to what was approved),
  but every task that touches it explains the corrected decision inline.
- **Out of scope, confirmed still out of scope:** no frontend file under
  `apps/knx-web/src` is modified anywhere in this plan.
