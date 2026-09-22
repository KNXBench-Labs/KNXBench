# Structural Drag-and-Drop Implementation Plan

**Execution status (2026-09-22): Complete.** Tasks 1–3, full branch gates, fresh
whole-branch review plus its single RED→GREEN fix round, non-fast-forward merge
`02237eb`, and full merged-result verification are complete.

> **For agentic workers:** REQUIRED SUB-SKILL: Use
> `superpowers:subagent-driven-development` or `superpowers:executing-plans`
> to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax.

**Goal:** Add device-to-line and device-to-building-part gestures through
existing validated, undoable commands while retaining the Inspector controls
as keyboard equivalents.

**Architecture:** `ProjectExplorer` owns transient native drag state and
revalidates an application-specific device payload against the current first
installation before dispatch. Existing API functions remain the only mutation
path; `App` supplies its existing translated toast callbacks. No optimistic
tree edit, server change, dependency, or generic drag framework is introduced.

**Tech Stack:** React 19, TypeScript, native HTML Drag and Drop API, Vitest,
happy-dom, typed English/German catalogues, existing Rust/Axum command stack.

**Spec:**
[`docs/superpowers/specs/2026-09-22-structural-drag-drop-design.md`](../specs/2026-09-22-structural-drag-drop-design.md)

## Global Constraints

- Implement exactly device → line and device → building part.
- Keep group address → communication object excluded because link direction is
  an explicit user choice with no honest silent default.
- Call only `api.moveDeviceToLine` and `api.moveDeviceToBuildingPart`; never
  edit `ProjectTree` optimistically.
- Only devices found in the first installation topology are drag sources; only
  first-installation lines and building parts are targets.
- Treat every `DataTransfer` value as untrusted. Require the exact custom MIME,
  a complete safe positive base-10 integer, and current-tree eligibility.
- Existing Inspector selects remain the keyboard equivalents.
- Use `App`'s existing `pushFun` and `reportError` toast paths for outcomes.
- Use native drag events and existing `--knx-*` CSS tokens; add no dependency,
  hard-coded colour, per-theme rule, or reduced-motion animation.
- Every label and summary exists in typed English and German catalogues.
- Do not edit `docs/LIMITATION_TRIAGE.md`.
- Run no real KNX, multicast, discovery, tunnelling, LAN, gateway, or hardware
  operation. HTTP calls in tests are mocked.

## Review Focus

- Foreign or malformed payloads must not enable targets or call an API; Task 1
  pins exact MIME and integer parsing.
- A source removed before drop must not mutate stale state; Task 1 rerenders
  without the source before dropping.
- Second-installation sources and targets must not acquire first-installation
  mutation affordances; Tasks 1 and 2 pin both sides.
- API rejection must retain the projected tree and announce the error; Tasks 1
  and 2 assert no `onTreeUpdate` and one error callback.
- Success must be announced in the active language; Task 3 checks English and
  German `ToastStack` output.

---

### Task 1: Device-to-line drag through the existing command

**Files:**

- Modify: `apps/knx-web/src/ProjectExplorer.tsx`
- Modify: `apps/knx-web/src/ProjectExplorer.test.tsx`
- Modify: `apps/knx-web/src/messages/en.ts`
- Modify: `apps/knx-web/src/messages/de.ts`

**Interfaces:**

- Consumes `api.moveDeviceToLine(deviceId: number, lineId: number | null): Promise<ProjectTree>`.
- Extends `ProjectExplorer` props with `onSummary: (message: string) => void`
  and `onError: (error: unknown) => void`.
- Produces local MIME `application/x-knxbench-device-id` and native device/line
  event wiring reused by Task 2.

- [ ] **Step 1: Add failing source-eligibility and payload tests**

  Extend the API mock with both single-device move functions:

  ```ts
  moveDeviceToLine: vi.fn(),
  moveDeviceToBuildingPart: vi.fn(),
  ```

  Extend `ExplorerHarness` and `renderExplorer` with `onSummary` and `onError`
  spies. Add a second installation fixture. Because happy-dom does not promise a
  native `DataTransfer` constructor, use one test-local fake with a `Map`,
  `types`, `setData`, `getData`, `effectAllowed`, and `dropEffect`, cast only at
  the event boundary. Assert:

  ```ts
  expect(labelFor("Device A").getAttribute("draggable")).toBe("true");
  expect(labelFor("Second device").getAttribute("draggable")).not.toBe("true");
  ```

  Dispatch `dragstart` with a test `DataTransfer` and assert the exact MIME
  payload is the decimal device id. Dispatch a foreign MIME payload and values
  `"1x"`, `"0"`, `"-1"`, and `"9007199254740992"`; assert no line target
  advertises readiness and no move API is called.

- [ ] **Step 2: Run source tests to verify RED**

  ```bash
  cd apps/knx-web
  npx vitest run src/ProjectExplorer.test.tsx -t "drag source|foreign drag"
  ```

  Expected: FAIL because tree labels have no draggable contract or typed
  payload handling.

- [ ] **Step 3: Add minimal typed native-drag state**

  In `ProjectExplorer.tsx`, add:

  ```ts
  const DEVICE_DRAG_MIME = "application/x-knxbench-device-id";
  type DragSource = { deviceId: number };

  function parseDraggedDevice(dataTransfer: DataTransfer): number | null {
    if (!Array.from(dataTransfer.types).includes(DEVICE_DRAG_MIME)) return null;
    const raw = dataTransfer.getData(DEVICE_DRAG_MIME);
    if (!/^[1-9]\d*$/.test(raw)) return null;
    const id = Number(raw);
    return Number.isSafeInteger(id) ? id : null;
  }
  ```

  Derive a `Set<number>` from devices on first-installation lines plus its
  `unassigned` list, matching
  `findDeviceLineInFirstInstallation(tree, id) !== undefined` exactly.
  Thread optional native label props through `TreeNode`:

  ```ts
  draggable?: boolean;
  dragging?: boolean;
  onDragStart?: React.DragEventHandler<HTMLButtonElement>;
  onDragEnd?: React.DragEventHandler<HTMLButtonElement>;
  dropReady?: boolean;
  onDragOver?: React.DragEventHandler<HTMLButtonElement>;
  onDrop?: React.DragEventHandler<HTMLButtonElement>;
  ```

  `DeviceItem` writes the MIME payload and sets `{ deviceId }` only while the id
  remains eligible. Drag end clears state. `TreeNode` emits
  `data-dragging="true"` for the active source and `data-drop-ready="true"`
  only when `dropReady` is true.

- [ ] **Step 4: Run source tests to verify GREEN**

  ```bash
  cd apps/knx-web
  npx vitest run src/ProjectExplorer.test.tsx -t "drag source|foreign drag"
  ```

  Expected: PASS.

- [ ] **Step 5: Add failing line-drop success, rejection, and stale tests**

  Use native events against actual device and line label buttons. Success:

  ```ts
  apiMock.moveDeviceToLine.mockResolvedValueOnce(nextTree);
  await dragAndDrop(labelFor("Device A"), labelFor("1.1 Line A"));
  expect(apiMock.moveDeviceToLine).toHaveBeenCalledWith(1, 11);
  expect(onTreeUpdate).toHaveBeenCalledWith(nextTree);
  expect(onSummary).toHaveBeenCalledWith("Device A moved to line 1.1 Line A.");
  ```

  Rejection uses `mockRejectedValueOnce(new Error("move refused"))` and asserts
  no tree update or summary while `onError` receives that error. For stale
  source, start a drag, rerender without that device, drop on the former valid
  line, and assert no API or callback ran. A second-installation line must never
  gain `data-drop-ready`.

- [ ] **Step 6: Run line-drop tests to verify RED**

  ```bash
  cd apps/knx-web
  npx vitest run src/ProjectExplorer.test.tsx -t "line drop|stale drag"
  ```

  Expected: FAIL because line labels do not dispatch the existing API.

- [ ] **Step 7: Implement line drop and translated result**

  A first-installation `LineItem` prevents `dragover` only while the current
  source is eligible. On drop, parse again and confirm current-tree eligibility
  before calling:

  ```ts
  try {
    const next = await api.moveDeviceToLine(deviceId, line.id);
    onTreeUpdate(next);
    onSummary(t("dragDrop.movedToLine", {
      device: deviceName,
      line: t("inspector.lineLabel", { address: line.address, name: line.name }),
    }));
  } catch (error) {
    onError(error);
  } finally {
    setDragSource(null);
  }
  ```

  Add catalogue entries with identical placeholders:

  ```ts
  // en.ts
  "dragDrop.movedToLine": "{device} moved to line {line}.",
  // de.ts
  "dragDrop.movedToLine": "{device} wurde in Linie {line} verschoben.",
  ```

  Resolve `deviceName` from the current projection, never from drag text.

- [ ] **Step 8: Run Task 1 tests and TypeScript**

  ```bash
  cd apps/knx-web
  npx vitest run src/ProjectExplorer.test.tsx
  npx tsc --noEmit
  ```

  Expected: PASS.

- [ ] **Step 9: Commit the line gesture**

  ```bash
  git add apps/knx-web/src/ProjectExplorer.tsx apps/knx-web/src/ProjectExplorer.test.tsx apps/knx-web/src/messages/en.ts apps/knx-web/src/messages/de.ts
  git commit -m "feat(drag): let devices catch the right line" -m "Route native device-to-line drops through the existing validated command and announce every outcome."
  ```

### Task 2: Device-to-building-part drag through the existing command

**Files:**

- Modify: `apps/knx-web/src/ProjectExplorer.tsx`
- Modify: `apps/knx-web/src/ProjectExplorer.test.tsx`
- Modify: `apps/knx-web/src/messages/en.ts`
- Modify: `apps/knx-web/src/messages/de.ts`

**Interfaces:**

- Consumes Task 1's eligible source, parser, `TreeNode` props, and callbacks.
- Consumes `api.moveDeviceToBuildingPart(deviceId: number, partId: number | null): Promise<ProjectTree>`.
- Produces the second gesture without another drag state or parser.

- [ ] **Step 1: Add failing building-part success and rejection tests**

  Give the first installation one building part. Success asserts:

  ```ts
  apiMock.moveDeviceToBuildingPart.mockResolvedValueOnce(nextTree);
  await dragAndDrop(labelFor("Device A"), labelFor("Room A (Room)"));
  expect(apiMock.moveDeviceToBuildingPart).toHaveBeenCalledWith(1, 501);
  expect(onTreeUpdate).toHaveBeenCalledWith(nextTree);
  expect(onSummary).toHaveBeenCalledWith("Device A moved to Room A.");
  ```

  Rejection asserts no tree update or summary and one error callback. A building
  part in the second installation must not prevent `dragover`, gain
  `data-drop-ready`, or call the API.

- [ ] **Step 2: Run building-part tests to verify RED**

  ```bash
  cd apps/knx-web
  npx vitest run src/ProjectExplorer.test.tsx -t "building-part drop"
  ```

  Expected: FAIL because building labels are not drop targets.

- [ ] **Step 3: Reuse Task 1 ownership for building-part drop**

  `BuildingItem` accepts drops only when `isFirst`. Its handler performs the
  same current-source recheck and calls:

  ```ts
  const next = await api.moveDeviceToBuildingPart(deviceId, building.id);
  onTreeUpdate(next);
  onSummary(t("dragDrop.movedToBuildingPart", {
    device: deviceName,
    buildingPart: building.name,
  }));
  ```

  Add:

  ```ts
  // en.ts
  "dragDrop.movedToBuildingPart": "{device} moved to {buildingPart}.",
  // de.ts
  "dragDrop.movedToBuildingPart": "{device} wurde nach {buildingPart} verschoben.",
  ```

  Extract only the small shared closure selecting the existing API call. Do not
  introduce a drag service or command registry.

- [ ] **Step 4: Run Task 2 and complete explorer tests**

  ```bash
  cd apps/knx-web
  npx vitest run src/ProjectExplorer.test.tsx
  npx tsc --noEmit
  ```

  Expected: PASS.

- [ ] **Step 5: Commit the building gesture**

  ```bash
  git add apps/knx-web/src/ProjectExplorer.tsx apps/knx-web/src/ProjectExplorer.test.tsx apps/knx-web/src/messages/en.ts apps/knx-web/src/messages/de.ts
  git commit -m "feat(drag): give devices some room to move" -m "Reuse the validated building-part command for native drops while keeping second-installation targets honest."
  ```

### Task 3: Accessible outcomes, keyboard parity, and token-only feedback

**Files:**

- Modify: `apps/knx-web/src/App.tsx`
- Modify: `apps/knx-web/src/App.test.tsx`
- Modify: `apps/knx-web/src/Inspector.test.tsx`
- Modify: `apps/knx-web/src/styles.css`
- Modify: `apps/knx-web/src/motionGuard.test.ts`

**Interfaces:**

- Consumes Task 1's `ProjectExplorer.onSummary` and `.onError` callbacks.
- Reuses `useToasts().pushFun`, `App.reportError`, and `ToastStack` live roles.
- Reuses both existing Inspector fields and API functions.

- [ ] **Step 1: Add failing App toast integration tests**

  Extend `App.test.tsx`'s API mock with both move functions. Open a synthetic
  project containing one first-installation device, line, and room. Perform the
  actual explorer drag/drop.

  For success:

  ```ts
  expect(host!.querySelector('[role="status"]')!.textContent)
    .toContain("Device A moved to line 1.1 Line A.");
  ```

  Switch UI language to German and repeat the building-part drop, asserting the
  German sentence in `role="status"`. Reject a move and assert an existing
  `role="alert"` error toast appears while the old tree remains rendered.

- [ ] **Step 2: Run App tests to verify RED**

  ```bash
  cd apps/knx-web
  npx vitest run src/App.test.tsx -t "drag-and-drop announcement"
  ```

  Expected: FAIL because `App` does not pass toast callbacks.

- [ ] **Step 3: Wire explorer outcomes into existing toast ownership**

  Change the existing render only:

  ```tsx
  <ProjectExplorer
    tree={tree}
    selection={selection}
    onSelect={selectEntity}
    onTreeUpdate={handleTreeUpdate}
    multiSelection={multiSelection}
    onItemClick={onItemClick}
    onSummary={pushFun}
    onError={reportError}
  />
  ```

  Do not add a live region or another toast hook.

- [ ] **Step 4: Add or strengthen keyboard-equivalence tests**

  In `Inspector.test.tsx`, render a selected first-installation device with both
  labelled native selects. Dispatch keyboard-compatible `change` events and
  assert exact calls and returned-tree propagation:

  ```ts
  expect(apiMock.moveDeviceToLine).toHaveBeenCalledWith(device.id, line.id);
  expect(apiMock.moveDeviceToBuildingPart).toHaveBeenCalledWith(device.id, room.id);
  expect(onApplied).toHaveBeenCalledWith(nextTree);
  ```

  Retain stronger existing coverage instead of duplicating it.

- [ ] **Step 5: Run keyboard tests and record existing coverage ruling**

  ```bash
  cd apps/knx-web
  npx vitest run src/Inspector.test.tsx -t "keyboard equivalent"
  ```

  Expected: FAIL where exact API/callback coverage is absent. If an existing
  test already proves both facts, record that evidence in the execution ledger
  and do not rename production code merely to manufacture a failure.

- [ ] **Step 6: Add static token-only feedback and guard it**

  Add no transition:

  ```css
  .workbench .tree-label[draggable="true"] { cursor: grab; }
  .workbench .tree-label[data-dragging="true"] { opacity: 0.72; }
  .workbench .tree-label[data-drop-ready="true"] {
    outline: 2px solid var(--knx-accent);
    outline-offset: -2px;
    background: color-mix(in srgb, var(--knx-accent) 16%, transparent);
  }
  ```

  Extend `motionGuard.test.ts` to extract these selectors and assert no
  `transition`, `animation`, hexadecimal/rgb colour literal, or per-theme
  prefix appears.

- [ ] **Step 7: Run Task 3 tests and full frontend suite**

  ```bash
  cd apps/knx-web
  npx vitest run src/App.test.tsx src/Inspector.test.tsx src/ProjectExplorer.test.tsx src/motionGuard.test.ts
  npx tsc --noEmit
  npx vitest run
  ```

  Expected: PASS with outcomes exposed through existing live roles in English
  and German.

- [ ] **Step 8: Commit accessible integration**

  ```bash
  git add apps/knx-web/src/App.tsx apps/knx-web/src/App.test.tsx apps/knx-web/src/Inspector.test.tsx apps/knx-web/src/styles.css apps/knx-web/src/motionGuard.test.ts
  git commit -m "feat(drag): announce arrivals without interpretive dance" -m "Wire structural drops into existing toasts, prove keyboard equivalents, and keep target feedback token-only and motion-free."
  ```

### Task 4: Documentation, full verification, review, and integration

**Files:**

- Modify: `docs/IMPLEMENTATION_STATUS.md`
- Create: `.ai/logs/2026-09-22_codex__t11_structural_drag_drop.md`
- Modify: `.ai/CURRENT_STATE.md`
- Modify: `docs/superpowers/plans/2026-09-22-structural-drag-drop.md`

**Interfaces:**

- Records B10 as partially closed by two complete gestures.
- Records group address → communication object as deliberately omitted because
  direction remains explicit.
- Produces durable test, review, no-network, merge, and next-task evidence.

- [ ] **Step 1: Reconcile durable documentation**

  Add a dated T11 section to `IMPLEMENTATION_STATUS.md` naming both gestures,
  command ownership, first-installation boundary, keyboard paths, accessible
  outcomes, and the omitted direction-sensitive third gesture. Do not claim
  general drag support and do not edit `LIMITATION_TRIAGE.md`.

- [ ] **Step 2: Run focused source audits**

  ```bash
  rg -n "LinkComObject|linkComObject" apps/knx-web/src/ProjectExplorer.tsx || true
  git diff --unified=0 main...HEAD -- apps/knx-web/src | rg '^\+' | rg "1\.1\.220|192\.168\.|10\.[0-9]|172\.(1[6-9]|2[0-9]|3[01])\." || true
  git diff --check
  ```

  Expected: no group-link drag, forbidden installation/private-network literal,
  or whitespace error.

- [ ] **Step 3: Run every repository gate**

  ```bash
  export CARGO_TARGET_DIR=/tmp/knxbench-t11-target
  export CARGO_INCREMENTAL=0
  export CARGO_PROFILE_DEV_DEBUG=0
  export CARGO_PROFILE_TEST_DEBUG=0
  cargo fmt --all --check
  cargo clippy --workspace --all-targets -- -D warnings
  cargo test --workspace --no-fail-fast
  cargo run -q -p xtask -- check-layering
  cargo run -q -p xtask -- check-headers
  cargo run -q -p xtask -- check-anchors
  cargo deny check
  cd apps/knx-web
  npx tsc --noEmit
  npx vitest run
  ```

  Expected: every command exits 0. If disk pressure affects Cargo, remove only
  `/tmp/knxbench-t11-target`, recreate the low-footprint environment, and rerun
  the missing gate.

- [ ] **Step 4: Write handover evidence and commit documentation**

  Record commands, counts, RED→GREEN observations, reviewer findings, and
  no-network proof. Prepend `.ai/CURRENT_STATE.md`. Then:

  ```bash
  git add docs/IMPLEMENTATION_STATUS.md docs/superpowers/plans/2026-09-22-structural-drag-drop.md
  git add -f .ai/CURRENT_STATE.md .ai/logs/2026-09-22_codex__t11_structural_drag_drop.md
  git commit -m "docs(drag): file the devices' travel paperwork" -m "Record two validated gestures, keyboard parity, verification, and the deliberately direction-sensitive omission."
  git push
  ```

- [ ] **Step 5: Fresh whole-branch review and one fix round**

  Review the complete base-to-head diff against spec and plan. Fix every
  Critical or Important finding with a failing regression test first, rerun
  affected suites, update evidence, commit with the required author and body,
  and push. Record Minor findings honestly.

- [ ] **Step 6: Merge, repeat full gates, and push `main`**

  Merge the reviewed branch into `main` with a non-fast-forward merge. Repeat
  every Step 3 gate on the merge commit, mark this plan complete, update the
  handover, commit that evidence, push `main`, fetch, and prove:

  ```bash
  test "$(git rev-parse HEAD)" = "$(git rev-parse origin/main)"
  ```

  Expected: exit 0 and a clean `main` worktree.
