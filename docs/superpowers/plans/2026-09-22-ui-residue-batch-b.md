# UI Residue Batch B Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use
> `superpowers:subagent-driven-development` to implement this plan task-by-task.
> Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Close T13's honest modified-state, live group-address context,
documented building-space vocabulary, and theme-contrast gaps without UI-owned
domain workarounds.

**Architecture:** Four independent slices extend existing owners: normalized
project equality and server session metadata own dirty state; `BusSession` owns
its refreshable address context; `BuildingPartType` remains the exhaustive
normalized vocabulary; ADR-0022's build-time parser owns contrast enforcement.
Each slice is committed and reviewed independently, followed by documentation
reconciliation and whole-repository verification.

**Tech Stack:** Rust 2021 workspace (`knx-core`, `knx-etsproj`, `knx-store`,
`knx-projection`, `knx-server`), Axum, React 19, TypeScript 7, Vitest, CSS custom
properties, `ts-rs`.

**Spec:** `docs/superpowers/specs/2026-09-22-ui-residue-batch-b-design.md`

## Global Constraints

- `docs/LIMITATION_TRIAGE.md` must not change.
- The §60 web-diff slice is excluded because
  `.superpowers/sdd/goal/task-15-report.md` does not exist.
- No KNX, LAN, multicast, gateway, or hardware traffic may be generated.
- Do not add private-LAN literals or the prohibited T12 individual-address
  fixture.
- Group-address display remains slash-only with no selector.
- Every user-facing string is present in both English and German catalogues.
- UI styling uses existing `--knx-*` tokens; motion remains inside the existing
  reduced-motion boundary.
- No store migration: the native schema remains version 9.
- Every new `.rs`, `.ts`, or `.tsx` file starts with a purpose-sentence header.
- Use author `KNXBench <github@knxbench.com>`, no `Co-Authored-By`, and a
  concise mildly funny/gloomy commit subject plus explanatory body.
- Before each task, verify free space on `/mnt/daten-i` and the selected Cargo
  target filesystem. Use `/var/tmp/knxbench-t13-target` with
  `CARGO_INCREMENTAL=0`, `CARGO_PROFILE_DEV_DEBUG=0`,
  `CARGO_PROFILE_TEST_DEBUG=0`, and `CARGO_BUILD_JOBS=2` for Rust gates.

## Review Focus

- Allocator-only drift after undo must not keep a project dirty; Task 1's
  baseline/undo regression pins it.
- A failed save must not move the clean baseline; Task 1's save-error regression
  pins it.
- A style update racing an active fake session must refresh both monitor
  formatting and write parsing without reconnecting; Task 2's route integration
  test pins it.
- Unknown imported or stored building kinds must still fail/report explicitly;
  Task 3's unknown-token and corrupt-store regressions pin it.
- Unsupported or unresolved contrast colors must fail rather than be skipped;
  Task 4's parser diagnostics and deliberately broken palette regressions pin it.

---

### Task 1: Publish honest project modification state (§§81/103)

**Files:**

- Modify: `crates/knx-core/src/string_table.rs`
- Modify: `crates/knx-core/src/project.rs`
- Modify: `crates/knx-projection/src/lib.rs`
- Modify: `apps/knx-web/src/bindings/ProjectTree.ts`
- Modify: `apps/knx-server/src/domain.rs`
- Modify: `apps/knx-server/tests/http_project_routes.rs`
- Modify: `apps/knx-web/src/App.tsx`
- Modify: `apps/knx-web/src/App.test.tsx`
- Modify: typed `ProjectTree` fixtures in `apps/knx-web/src/App.test.tsx`,
  `BusMonitorPanel.test.tsx`, `CommandPalette.test.tsx`,
  `DeviceWorkspace.test.tsx`, `GroupAddressTable.test.tsx`,
  `Inspector.test.tsx`, `Inspector.tsx`, `LogPanel.test.tsx`,
  `NewProjectDialog.test.tsx`, `ParameterPanel.test.tsx`,
  `ProjectExplorer.test.tsx`, `StructureWorkspace.test.tsx`,
  `busContext.test.ts`, `commandRegistry.test.ts`,
  `dashboardStats.test.ts`, and `treeUtils.test.ts`
- Modify: `docs/KNOWN_LIMITATIONS.md` (§§81 and 103 only)

**Interfaces:**

- Produces: `Project::same_user_content_as(&self, other: &Self) -> bool`.
- Produces: `AppState.clean_project: Mutex<Option<Project>>` and
  `ProjectTree.is_modified: bool`.
- Consumes: existing replacement lock order and server `tree_with_state` overlay.
- Later tasks consume no Task 1 internals; only final docs consume its status.

- [ ] **Step 1: Write failing core and projection tests**

Add a `Project` comparison test proving allocator-only differences are ignored
and ordinary project content differences are not. Add the new projection field
with a pure-projection default assertion:

```rust
assert!(baseline.same_user_content_as(&allocator_advanced));
assert!(!baseline.same_user_content_as(&renamed));
assert!(!build_project_tree(&project).is_modified);
```

- [ ] **Step 2: Run the focused tests and verify RED**

Run:

```bash
cargo test -p knx-core same_user_content
cargo test -p knx-projection is_modified
```

Expected: compile/test failure because the comparison and field do not exist.

- [ ] **Step 3: Implement normalized content comparison**

Derive `Clone` for `StringTable` and `Project`. Implement the comparison by
cloning both values, assigning the same default `IdAllocators` to both clones,
then using structural equality. Do not hand-enumerate project fields.

- [ ] **Step 4: Add failing server state tests**

Cover all clean-boundary and disagreement cases in `domain.rs` and
`http_project_routes.rs`:

```rust
assert!(tree.can_redo);
assert!(!tree.is_modified); // edit then undo to baseline

state.project.lock().unwrap().as_mut().unwrap().info.name = "direct".into();
assert!(!tree.can_undo);
assert!(tree.is_modified); // mutation outside command stack
```

Also assert a failed save retains dirty state and successful Save/Save As clears
it.

- [ ] **Step 5: Run focused server tests and verify RED**

Run:

```bash
cargo test -p knx-server domain::tests::modified_state
cargo test -p knx-server --test http_project_routes modified
```

Expected: failures against the undo proxy and missing baseline.

- [ ] **Step 6: Implement the application-owned baseline**

Add `clean_project` to `AppState`. Extend replacement publication and
`tree_with_state` using one documented lock order. Establish the baseline only
after successful open/import/new/save operations. Change `new_project_impl` to
use the same predicate as the published tree. A failed store write must leave the
previous baseline untouched.

- [ ] **Step 7: Add failing frontend quit tests**

Regenerate/update `ProjectTree.ts`, update typed fixtures, then assert the two
disagreement directions:

```tsx
tree = { ...tree, canUndo: true, isModified: false }; // no quit prompt
tree = { ...tree, canUndo: false, isModified: true }; // prompt
```

- [ ] **Step 8: Change the quit guard and verify Task 1**

Use only `tree.isModified` for `quitRequested`; leave toolbar undo/redo behavior
unchanged. Run:

```bash
cargo test -p knx-core same_user_content
cargo test -p knx-projection
cargo test -p knx-server --lib modified_state
cargo test -p knx-server --test http_project_routes
cd apps/knx-web && npx tsc --noEmit
npm test -- --configLoader runner src/App.test.tsx
```

- [ ] **Step 9: Update §§81/103 and commit**

Keep both numbered headings stable. Describe the snapshot baseline, allocator
normalization, transient/non-persisted nature, and verified disagreement cases.
Commit only Task 1 files.

---

### Task 2: Refresh active session group-address context (§91)

**Files:**

- Modify: `apps/knx-server/src/bus.rs`
- Modify: `apps/knx-server/src/routes.rs`
- Modify: `apps/knx-server/tests/http_bus_write.rs`
- Modify: `apps/knx-server/tests/http_edit_routes.rs` or add the route-level
  regression to the existing bus integration test that owns the fake session
- Modify: `docs/KNOWN_LIMITATIONS.md` (§91 only)

**Interfaces:**

- Produces: `BusSession::update_group_address_context(GroupAddressContext)`.
- Consumes: `GroupAddressContext::from_project`, the existing fake connector,
  and `POST /api/project/group-address-style`.
- Must preserve: no project mutex held while awaiting or holding the async
  bus-session mutex.

- [ ] **Step 1: Write the failing fake-session route regression**

Start a fake monitor session against a project using one level style, mutate the
style through the public project route, then feed/read a subsequent telegram and
submit a write using its displayed address. Assert the new style is used and the
connector was neither restarted nor reconnected.

- [ ] **Step 2: Run focused tests and verify RED**

Run:

```bash
cargo test -p knx-server --test http_bus_write style_change
cargo test -p knx-server --test http_edit_routes group_address_style
```

Expected: session formatting/parsing remains at the start-time style.

- [ ] **Step 3: Make the session context refreshable**

Store the context in `Arc<std::sync::RwLock<GroupAddressContext>>`; hand the same
cell to the drain task. Read-lock only long enough to format/decode/resolve, and
write-lock only to replace the complete value. Convert poisoned-lock failures to
the module's existing deterministic failure convention; do not silently keep a
partially updated context.

- [ ] **Step 4: Refresh after successful style mutation**

In the HTTP handler, apply the domain command first, build a fresh context while
holding only the project mutex, release it, then lock `bus_session` and update an
active session. Do not reconnect or send any bus frame.

- [ ] **Step 5: Verify Task 2 and commit**

Run:

```bash
cargo test -p knx-server --lib bus::tests
cargo test -p knx-server --test http_bus_write
cargo test -p knx-server --test http_edit_routes
cargo clippy -p knx-server --all-targets -- -D warnings
```

Rewrite §91 as resolved while retaining the heading and the distinction between
slash notation and project address level. Commit only Task 2 files.

---

### Task 3: Preserve all documented `Space/@Type` values (§89)

**Files:**

- Modify: `crates/knx-core/src/building.rs`
- Modify: `crates/knx-etsproj/src/values.rs`
- Modify: `crates/knx-etsproj/src/map.rs`
- Modify: `crates/knx-etsproj/tests/golden_reference_project.rs`
- Modify: `crates/knx-store/src/building.rs`
- Modify: `crates/knx-store/src/lib.rs`
- Modify: `crates/knx-store/src/project.rs`
- Modify: `crates/knx-projection/src/lib.rs`
- Modify: `apps/knx-server/src/domain.rs`
- Modify: `apps/knx-server/tests/http_edit_routes.rs`
- Modify: `apps/knx-web/src/Inspector.tsx`
- Modify: `apps/knx-web/src/ProjectExplorer.tsx`
- Modify: `apps/knx-web/src/messages/en.ts`
- Modify: `apps/knx-web/src/messages/de.ts`
- Modify: focused Inspector/ProjectExplorer/i18n tests
- Modify: one focused `knx-diff` or `knx-report` regression
- Modify: `docs/DATA_MODEL.md`
- Modify: `docs/IMPORT_EXPORT.md`
- Modify: `docs/COMPATIBILITY.md`
- Modify: `docs/KNOWN_LIMITATIONS.md` (§89 only)

**Interfaces:**

- Produces five `BuildingPartType` variants with exact strings: `Stairway`,
  `RoomPart`, `Area`, `Ground`, `Segment`.
- Store remains schema version 9; existing `kind TEXT NOT NULL` carries values.
- Unknown ETS values remain map errors; unknown stored values become typed load
  errors rather than `Building`.

- [ ] **Step 1: Write failing enum/parser/map tests**

Table-drive the five exact strings through `parse_building_part_type` and the
mapper. Retain a `FutureSpace` rejection assertion and assert the documented
tokens generate no type-related `MapProblem`.

- [ ] **Step 2: Verify parser/map RED**

Run:

```bash
cargo test -p knx-etsproj building_part_type
cargo test -p knx-etsproj --test golden_reference_project space_type
```

Expected: five tokens currently return `UnknownEnumValue` or map to the generic
fallback.

- [ ] **Step 3: Extend the normalized enum and importer**

Add all five variants and exhaustive parser/map arms. Update the core comment to
separate locally observed fixture values from Schema23-documented values and cite
the PDF sections in repository documentation, not as an unsupported code claim.

- [ ] **Step 4: Write failing native-store regressions**

Save/load/re-save a hierarchy containing every variant. Insert a corrupt/future
`kind` string directly and assert load returns `StoreError` naming that value.

- [ ] **Step 5: Implement strict store codecs without migration**

Add `StoreError::UnknownBuildingPartType(String)`, make `kind_from_str` return
`Result<BuildingPartType, StoreError>`, and propagate it through row loading.
Add exact serialization arms. Do not change migration files or
`CURRENT_SCHEMA_VERSION`.

- [ ] **Step 6: Extend projection, API creation, and localized UI**

Add exhaustive projection/server parser arms, creation options, Inspector label
keys, and English/German values. Keep one source list/pattern already used by the
creation UI; do not add a second configurable registry.

- [ ] **Step 7: Add UI/API/report proof**

Assert the project tree returns exact strings, all five are selectable and
localized, and one diff/report case distinguishes a new kind from
`BuildingPart`.

- [ ] **Step 8: Verify Task 3**

Run:

```bash
cargo test -p knx-core building
cargo test -p knx-etsproj building_part
cargo test -p knx-store building
cargo test -p knx-projection building
cargo test -p knx-server building_part
cargo test -p knx-report building
cd apps/knx-web && npx tsc --noEmit
npm test -- --configLoader runner src/Inspector.test.tsx src/ProjectExplorer.test.tsx src/i18n.test.ts
```

- [ ] **Step 9: Reconcile §89 and model/compatibility docs, then commit**

State the Schema23 §1.1.2.3/§1.2.6.4 `Segment`/`RoomPart` contradiction, lack of
local fixture evidence, native-store roundtrip, and absence of ETS export after
ADR-0028. Commit only Task 3 files.

---

### Task 4: Enforce theme role contrast (§120)

**Files:**

- Modify: `apps/knx-web/src/themeTokens.ts`
- Modify: `apps/knx-web/src/themeTokens.test.ts`
- Modify: `docs/adr/0022-theme-token-boundary.md`
- Modify: `docs/KNOWN_LIMITATIONS.md` (§120 only)

**Interfaces:**

- Produces a test-facing color resolver and `contrastRatio` helper inside the
  existing theme-token ownership module.
- Consumes existing `ThemeBlock`, `ThemeVariationBlock`, declaration parser,
  `THEMES`, and registered accent variations.
- No runtime dependency or new package.

- [ ] **Step 1: Write failing color/contrast unit tests**

Cover black/white, equal colors, the 4.5 boundary, short/long hex forms used by
the selected roles, recursive token references, unknown notation, missing token,
and a reference cycle. Errors must name theme, pair, and offending value.

- [ ] **Step 2: Write the deliberately illegible palette test**

Build a synthetic theme block with foreground equal to background and assert the
gate returns a violation for that exact role pair. Add an unsupported `oklch()`
case and assert it is a violation rather than skipped.

- [ ] **Step 3: Run focused tests and verify RED**

Run:

```bash
cd apps/knx-web
npm test -- --configLoader runner src/themeTokens.test.ts
```

Expected: missing resolver/gate and no contrast failures.

- [ ] **Step 4: Implement strict role evaluation**

Resolve the three spec pairs for each base theme and overlay each registered
accent variation before evaluating the accent pair. Support only explicitly
tested current concrete forms and `var()` chains. Return named violations for
alpha/unsupported/unresolved/cyclic values; never filter them out.

- [ ] **Step 5: Gate every shipped palette and variation**

Add one table-driven test over the theme registry and variation registry. Assert
no violation and a ratio of at least `4.5` for each named pair.

- [ ] **Step 6: Verify Task 4 and commit**

Run:

```bash
cd apps/knx-web
npx tsc --noEmit
npm test -- --configLoader runner src/themeTokens.test.ts src/theme.test.ts
```

Update ADR-0022 and §120 with the exact enforced pairs, strict unsupported-color
behavior, and non-claim for arbitrary component composition. Commit only Task 4
files.

---

### Task 5: Reconcile status and run whole-repository gates

**Files:**

- Modify: `docs/IMPLEMENTATION_STATUS.md`
- Modify: `.ai/CURRENT_STATE.md`
- Create: `.ai/logs/2026-09-22_codex__t13_ui_residue_batch_b.md`

**Interfaces:**

- Consumes all four reviewed task commits.
- Produces durable cross-session evidence and no product behavior.

- [ ] **Step 1: Reconcile documentation truth**

Record exact behavior, tests, KNX PDF evidence, the Schema23 contradiction, no
schema migration, §60 exclusion, and remaining bounded limitations. Remove stale
claims that dirty equals undo availability, session context is immutable, or
only six space kinds are representable.

- [ ] **Step 2: Audit forbidden scope and generated bindings**

Run `git diff --name-only <merge-base>..HEAD` and prove no
`docs/LIMITATION_TRIAGE.md`, protocol transport, or unrelated report-preview
implementation changed. Verify generated TypeScript bindings match Rust and scan
new lines for forbidden literals.

- [ ] **Step 3: Run frontend gates**

```bash
cd apps/knx-web
npx tsc --noEmit
npm test -- --configLoader runner
```

- [ ] **Step 4: Run Rust and repository gates**

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --no-fail-fast
cargo run -q -p xtask -- check-layering
cargo run -q -p xtask -- check-headers
cargo run -q -p xtask -- check-anchors
cargo deny check
git diff --check
```

- [ ] **Step 5: Commit evidence and request whole-branch review**

Commit the documentation/evidence block. Generate one review package from the
branch merge base to `HEAD`; review all four slices together. Fix every Critical
or Important finding in the single final fix wave allowed by the SDD workflow,
then perform one scoped re-review.

- [ ] **Step 6: Integrate and prove remote state**

Non-fast-forward merge to `main`, repeat the merged-result gates, push, fetch,
and prove `HEAD == origin/main`. Remove the completed project-local worktree and
local feature branch. Do not publish a release tag; T18 remains a separate final
goal decision.
