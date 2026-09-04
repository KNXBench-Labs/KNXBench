# Session 5, cycle 8: project status dashboard — design

**Status.** Approved, ready for implementation planning.

## Context

Dark/light mode (cycle 7) closed out CLAUDE.md's Session 5 UI/UX
deliverable list. `ideas.md`/ROADMAP.md list "project status/dashboard
view" as the next cycle 8 candidate — a projection-layer sibling to
Search, no architectural blocker.

**Current state** (`apps/knx-desktop`):

- `App.tsx`'s `workspace` div renders `ProjectExplorer` always, and
  `Inspector` only `{selection && ...}` — when a project is loaded but
  nothing is selected, the space next to the tree is empty.
- `ProjectTree` (`knx-projection`) already carries `schema_version`,
  `errors`, `warnings` (both genuinely populated from `ImportReport` by
  `apps/knx-desktop/src-tauri/src/lib.rs`'s `apply_report_counts`, not
  placeholder zeros in practice), and the full installation/topology/
  building/group-address tree.
- `DeviceNode` has no communication-object count — `com_objects` only
  exists as a `Vec<ComObjectInstanceId>` on the domain `DeviceInstance`,
  never projected as a count.
- `ProjectExplorer.tsx:139-152` already renders an `errors`/`warnings`
  footer of its own (`.import-errors`/`.import-warnings`). The dashboard
  duplicates these two numbers in a project-wide summary; this is
  intentional overlap, not a refactor target — the explorer's footer is
  a tree-scoped hint, the dashboard is the project-wide overview.

**Scope decisions:**

- Placement: fills the existing empty workspace slot when
  `tree && !selection` — no new toolbar button, no overlay, no keyboard
  shortcut. Mutually exclusive with `Inspector` the same way `Inspector`
  and nothing-selected are today.
- Stat set: installations, areas, lines, devices (assigned + unassigned
  broken out), group addresses, building parts, communication objects,
  plus `schema_version`/`errors`/`warnings` already on `ProjectTree`.
  Every count except communication objects is derivable from the
  existing `ProjectTree` shape in the frontend; communication objects
  need one new backend field (below).
- Read-only this cycle — no clickable stat drills into a filtered view.
  `ideas.md` gets a note that a clickable-stats/error-drilldown cycle
  is a deliberate future candidate, not an oversight, since no
  error-detail view exists yet to drill into.
- Aggregation lives in the frontend (`dashboardStats.ts`), not a new
  Tauri command — everything but the com-object count is already in the
  loaded `ProjectTree`; adding a command for a client-side sum would be
  the unnecessary layer CLAUDE.md warns against.

## Backend

`crates/knx-projection/src/lib.rs`:

- `DeviceNode` gains `pub com_object_count: usize`.
- `build_device_node` sets it from `device.com_objects.len()`.
- One test assertion added to the existing device-node test(s)
  confirming the count matches a device with a known number of
  communication objects (the existing `project_with_one_device`
  fixture already gives one device one com object — extend its
  assertion rather than add a new fixture).

No Tauri command changes — `com_object_count` travels on the
already-returned `ProjectTree`/`DeviceNode` payload every existing
command (`open_project`, `open_native_project`, `undo`, `redo`, command
handlers) already returns.

## Frontend

### `dashboardStats.ts` (new) — pure aggregation

```ts
import type { ProjectTree } from "./bindings/ProjectTree";
import type { BuildingNode } from "./bindings/BuildingNode";
import type { DeviceNode } from "./bindings/DeviceNode";

export interface DashboardStats {
  installations: number;
  areas: number;
  lines: number;
  devicesAssigned: number;
  devicesUnassigned: number;
  groupAddresses: number;
  buildingParts: number;
  comObjects: number;
}

/** Walks every installation's topology, buildings, and unassigned bucket
 * once, summing counts. Pure and total: an empty tree (no installations)
 * produces all-zero stats, not a special case. */
export function computeStats(tree: ProjectTree): DashboardStats {
  const stats: DashboardStats = {
    installations: tree.installations.length,
    areas: 0,
    lines: 0,
    devicesAssigned: 0,
    devicesUnassigned: 0,
    groupAddresses: 0,
    buildingParts: 0,
    comObjects: 0,
  };

  const addDevice = (d: DeviceNode) => {
    stats.comObjects += d.com_object_count;
  };
  const walkBuildings = (nodes: BuildingNode[]) => {
    for (const b of nodes) {
      stats.buildingParts += 1;
      b.devices.forEach(addDevice);
      walkBuildings(b.children);
    }
  };

  for (const inst of tree.installations) {
    stats.groupAddresses += inst.group_addresses.length;
    stats.devicesUnassigned += inst.unassigned.length;
    inst.unassigned.forEach(addDevice);
    for (const area of inst.topology) {
      stats.areas += 1;
      for (const line of area.lines) {
        stats.lines += 1;
        stats.devicesAssigned += line.devices.length;
        line.devices.forEach(addDevice);
      }
    }
    walkBuildings(inst.buildings);
  }

  return stats;
}
```

Note: `comObjects` sums from both topology-assigned and unassigned
devices, but **not** from building-placed devices separately counted —
a device can appear in both a `Line` and a `BuildingNode` (topology and
building placement are independent per DATA_MODEL), so counting
`comObjects` again inside `walkBuildings` would double-count devices
placed both ways. `walkBuildings` only accumulates `buildingParts`
count; device/com-object totals are attributed solely from
topology/unassigned, the model's authoritative device placement (a
building is a secondary organizational view, not a second ownership
list — mirrors how `treeUtils`'s existing search indexing already
treats topology as authoritative for devices, see `treeUtils.ts`).

### `Dashboard.tsx` (new)

```tsx
import type { ProjectTree } from "./bindings/ProjectTree";
import { computeStats } from "./dashboardStats";

export default function Dashboard(props: { tree: ProjectTree }) {
  const stats = computeStats(props.tree);
  return (
    <div className="dashboard">
      <h2>Project status</h2>
      <dl className="dashboard-stats">
        <dt>Schema version</dt>
        <dd>{props.tree.schema_version}</dd>
        <dt>Installations</dt>
        <dd>{stats.installations}</dd>
        <dt>Areas</dt>
        <dd>{stats.areas}</dd>
        <dt>Lines</dt>
        <dd>{stats.lines}</dd>
        <dt>Devices</dt>
        <dd>
          {stats.devicesAssigned + stats.devicesUnassigned}
          {stats.devicesUnassigned > 0 && ` (${stats.devicesUnassigned} unassigned)`}
        </dd>
        <dt>Group addresses</dt>
        <dd>{stats.groupAddresses}</dd>
        <dt>Building parts</dt>
        <dd>{stats.buildingParts}</dd>
        <dt>Communication objects</dt>
        <dd>{stats.comObjects}</dd>
      </dl>
      {(props.tree.errors > 0 || props.tree.warnings > 0) && (
        <dl className="dashboard-stats dashboard-issues">
          {props.tree.errors > 0 && (
            <>
              <dt>Import errors</dt>
              <dd className="dashboard-errors">{props.tree.errors}</dd>
            </>
          )}
          {props.tree.warnings > 0 && (
            <>
              <dt>Import warnings</dt>
              <dd>{props.tree.warnings}</dd>
            </>
          )}
        </dl>
      )}
    </div>
  );
}
```

No props beyond `tree` — no `onApplied`/command wiring, since nothing
here is editable.

### `App.tsx` wiring

```tsx
{tree && (
  <div className="workspace">
    <ProjectExplorer tree={tree} selection={selection} onSelect={selectEntity} />
    {selection ? (
      <Inspector .../>
    ) : (
      <Dashboard tree={tree} />
    )}
  </div>
)}
```

Replaces the current bare `{selection && <Inspector .../>}` with an
if/else against the same `tree && ...` guard — `Dashboard` never
renders without a loaded project, matching every other workspace
child.

### `styles.css`

New rules: `.dashboard` (padding/layout matching `.inspector`'s
existing container spacing), `.dashboard-stats` as a two-column
`dl` grid (`display: grid; grid-template-columns: auto 1fr; gap: ...`,
matching the label/value pairing pattern `Inspector.tsx`'s
`.inspector-field` already establishes for consistency), and
`.dashboard-errors` using the existing `var(--knx-error-color)` token
from cycle 7 — no new color introduced.

## Testing

`dashboardStats.test.ts` (Vitest, alongside `treeUtils.test.ts`):

- Empty tree (no installations) produces all-zero stats.
- Multiple installations sum across all of them, not just the first.
- A device appearing in both a `Line` and a nested `BuildingNode`
  contributes its `com_object_count` once, not twice (the
  double-count guard above, made concrete as a test).
- Nested `BuildingNode` children are counted at every depth, not just
  roots.
- Unassigned devices count toward `devicesUnassigned` and `comObjects`,
  not `devicesAssigned`.

Rust: one assertion added to `knx-projection`'s existing device-node
test confirming `com_object_count` on a fixture device with a known
com-object count.

No test for `Dashboard.tsx` itself or the `App.tsx` wiring — same
reason `Search.tsx`/`CommandPalette.tsx`/`ThemeToggle.tsx` have none:
DOM-touching, no component-testing library in this project (see cycle
7's design doc). Manual smoke check (load a project, confirm no
selection shows the dashboard with correct counts, select a device and
confirm the dashboard is replaced by the Inspector, deselect and
confirm the dashboard reappears) is left unperformed here for the same
reason cycles 4-7 left theirs unperformed — no display available in
this environment. Should be run before, or at, merge.

## Follow-up noted, not this cycle

Clickable stats with drilldown (e.g. clicking "Import errors" jumping
to an error-detail view) needs an error-detail view that doesn't exist
yet — tracked in `ideas.md` as a future dashboard cycle, not folded in
here.
