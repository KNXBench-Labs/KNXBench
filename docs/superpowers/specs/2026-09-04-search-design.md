# Session 5, cycle 5: search — design

**Status.** Approved, ready for implementation planning.

## Context

Cycle 4 (`docs/superpowers/specs/2026-09-04-selection-inspector-design.md`)
delivered device selection and a Properties Inspector. ROADMAP.md and
CLAUDE.md's UI/UX section list two deliverables still not scheduled:
Search, Command Palette, plus dark/light mode. This cycle is Search:
finding a device, group address, or building part by name/address across
a project too large to scan by eye in the Project Explorer alone.

**Current state** (`apps/knx-desktop`):

- `ProjectExplorer.tsx` only makes `DeviceItem` selectable.
  `BuildingItem`'s label click only expands/collapses — no selection.
- Group addresses are not represented in `ProjectTree` at all — cycle 4's
  own scope decision deferred them here explicitly, "since group addresses
  are not yet shown anywhere in the UI".
- `App.tsx` holds a single `selectedDeviceId: number | null` and fetches
  `DeviceDetail` for it. There is no selection concept for anything else.
- No frontend test runner exists (`package.json` has no test script);
  cycle 4 left this as an explicit, un-taken decision.

**Scope decisions:**

- Search reaches devices, group addresses, and building parts (matching
  ROADMAP's own wording), but does **not** add a permanent Group Address
  browsing panel to the Project Explorer — that is its own future chunk
  of work (a full group/sub-group hierarchy view). This cycle makes group
  addresses *searchable and selectable*; picking one opens it in the
  Inspector, nothing more.
- Search is a `Ctrl+K` modal overlay ("quick open"), not a live filter of
  the tree in place. An overlay's result list needs no tree
  expand/collapse/reveal logic, and keeps this cycle's surface disjoint
  from the later Command Palette overlay.
- Tree auto-expand/scroll-into-view when a search result lands inside a
  manually collapsed branch is explicitly **not** attempted this cycle —
  opening the Inspector is the "found it" signal; visually echoing the
  pick inside the tree is a nice-to-have, not a gap in what the user can
  accomplish.
- Group-address and building-part edits (`CreateGroupAddress`/
  `DeleteGroupAddress`, or anything on a building part) stay out of scope
  — no `Command` exists yet for building parts at all, and this cycle
  makes group addresses visible/selectable only, not editable.

## Backend

### Projection: group addresses join the tree

`GroupAddressEntry` (`crates/knx-core/src/group.rs`) already carries
everything worth showing — `id`, `name`, `address` — and lives on
`Installation.group_addresses`, one flat `Vec` per installation (not
nested under `GroupRange` for this cycle; range/central/unfiltered stay
internal, unused by the projection). The reference project has 514 group
addresses total — cheap enough to embed eagerly in `ProjectTree`, unlike
`DeviceDetail`'s 907 communication-object instances (cycle 4's reason for
making that one lazy).

```rust
// crates/knx-projection/src/lib.rs
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct GroupAddressNode {
    pub id: u32,
    pub name: String,
    /// Formatted per the project's own `GroupAddressStyle`
    /// (`GroupAddress::format`), e.g. `"4/2/100"`.
    pub address: String,
}
```

`InstallationNode` gains `pub group_addresses: Vec<GroupAddressNode>`,
populated in `build_project_tree` from `installation.group_addresses`,
formatted with `project.info.group_address_style` — the same style value
already used to format `DeviceNode.address`. No new lazy-loading command:
the data rides along on `ProjectTree`, which the frontend already holds
in full after every `open_project`/`open_native_project`/command/undo/
redo.

### Tauri layer

No changes. `group_addresses` reaches the frontend as part of the
existing `ProjectTree` payload; no new command is needed to select or
display a group address or a building part, since both are now fully
present client-side (a `BuildingNode` already carries `name`, `kind`,
`children`, `devices`).

## Frontend

### Selection generalized

`selectedDeviceId: number | null` becomes a tagged union:

```ts
type Selection =
  | { kind: "device"; id: number }
  | { kind: "group_address"; id: number }
  | { kind: "building_part"; id: number };
```

`App.tsx`'s `select(sel: Selection)` branches on `kind`:

- `"device"`: unchanged — the existing async `device_detail` Tauri round
  trip, including the stale-response guard (`selectedIdRef`) cycle 4
  built.
- `"group_address"` / `"building_part"`: resolved synchronously by
  walking the already-loaded `tree` (`findGroupAddress(tree, id)`,
  `findBuildingPart(tree, id)` — small pure helpers, `O(n)` over data
  that tops out in the hundreds). No network round trip, no stale-response
  race to guard against.

`ProjectExplorer.tsx`: `BuildingItem` becomes selectable the same way
`DeviceItem` already is (`selected` class, `onClick` calling `select`).

`Inspector.tsx` gains two new read-only render branches (no new
`Command`, so no editable fields):

- Group address: name, formatted address.
- Building part: name, kind, breadcrumb path (built by the same tree walk
  that finds it), immediate device count, immediate child-part count.

The existing device branch (address/DPT editing, undo/redo) is unchanged.

### Search overlay (`Search.tsx`, new)

- Opens on `Ctrl+K` — a `keydown` listener in `App.tsx` alongside the
  existing undo/redo one; `preventDefault()` so it can't collide with any
  browser/webview default binding.
- Renders a modal: text input (autofocused) + a result list. `Escape`
  closes it without changing selection; picking a result (click or
  `Enter` on the highlighted row; arrow keys move the highlight) calls
  `select()` and closes it.
- Search index: `useMemo` keyed on `tree`, producing a flat
  `SearchEntry[]`:

  ```ts
  type SearchEntry =
    | { kind: "device"; id: number; label: string; address: string | null }
    | { kind: "group_address"; id: number; label: string; address: string }
    | { kind: "building_part"; id: number; label: string; path: string };
  ```

  - Devices: walk topology (areas/lines) + buildings + unassigned,
    deduplicated by `id` in a `Map` — the same device otherwise appears
    twice (once under topology, once under its building).
  - Group addresses: flattened directly from every installation's
    `group_addresses`.
  - Building parts: recursive walk of `buildings`, `path` built from
    ancestor names joined by `" / "`.

- Matching: case-insensitive substring against `label` and, where
  present, `address`/`path`. Ranking: exact match first, then
  starts-with, then contains; ties broken alphabetically by `label`.
  Capped at 50 results, grouped under a heading per `kind` in the
  rendered list. Empty query shows no results (not the full index).

## Error handling

None new — search is read-only lookup over data already validated on
import; there is nothing to fail. Selecting a group address or building
part cannot error the way `device_detail` can (it's not a fallible
round trip), so no error banner path is needed for those two branches.

## Testing

- `knx-projection`: unit test — `build_project_tree` populates
  `group_addresses` per installation, address formatted per the
  project's `group_address_style`, alongside the existing
  `build_project_tree` tests' pattern.
- `apps/knx-desktop`: first Vitest setup — `vitest` + `@testing-library`
  is **not** pulled in this cycle (no component rendering under test,
  see below); just `vitest` itself, a `test` script in `package.json`,
  and a `vitest.config.ts` reusing the existing Vite config. Tests cover
  only the pure, non-React logic: the flatten-to-`SearchEntry[]`
  functions (dedup-by-id for devices, recursive building-part walk with
  correct `path`) and the match/rank function (exact > startsWith >
  contains, alphabetical tie-break, 50-cap, empty-query-returns-nothing).
  Rendering/interaction of `Search.tsx` itself is not unit-tested this
  cycle — no component-test infrastructure (`@testing-library/react`,
  jsdom) is introduced, matching cycle 4's precedent of leaving
  React-component behavior to manual review.
- CI (`.github/workflows/ci.yml`) gains an `npm test` (or
  `npx vitest run`) step alongside the existing Node.js setup.

## Out of scope (carried to later cycles)

- A permanent, browsable Group Address panel/tree (group/sub-group
  hierarchy) in the Project Explorer.
- `CreateGroupAddress`/`DeleteGroupAddress` commands and any UI for them.
- Editing anything on a group address or building part — no `Command`
  exists for either.
- Tree auto-expand/scroll-into-view to reveal a search result sitting
  inside a collapsed branch.
- Command Palette (builds on this cycle's overlay pattern and the
  selection/action groundwork, but is its own spec).
- Component-level frontend test infrastructure
  (`@testing-library/react`/jsdom) — this cycle's Vitest setup covers
  pure logic only.
