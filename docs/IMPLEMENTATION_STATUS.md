# IMPLEMENTATION_STATUS.md

Last updated: 2026-09-11 (T18 slice 2: `Module` expansion in the `Dynamic` evaluator, `knx-productdb`, see below)

**Rebrand (2026-09-05):** the project is now named **KNXBench** — product
name, app title, and GitHub repo (`KNXBench-Labs/KNX` → `KNXBench-Labs/KNXBench`)
only. Crate names (`knx-core`, `knx-desktop`, ...), the `knx` CLI binary, and
"KNX" as the protocol/standard term throughout the docs are unchanged and not
in scope.

## Where the project stands

| Session | Scope | Status |
| --- | --- | --- |
| 0 | Technical research | **Done** — see [RESEARCH.md](RESEARCH.md) |
| 1 | Architecture | **Done** — see [ARCHITECTURE.md](ARCHITECTURE.md), [adr/](adr/), [design spec](superpowers/specs/2026-09-02-knx-architecture-design.md) |
| 2 | KNX core | **Done** — see [DATA_MODEL.md](DATA_MODEL.md) |
| 3 | ETS project import | **Done** — see [IMPORT_EXPORT.md](IMPORT_EXPORT.md), [COMPATIBILITY.md](COMPATIBILITY.md) |
| 4 | Manufacturer database | **Done** — see [IMPORT_EXPORT.md §10](IMPORT_EXPORT.md), [ADR-0011](adr/0011-product-database-storage.md), [ADR-0012](adr/0012-enrichment-into-absent-slots.md) |
| 5 | UI / UX | **Done** — cycle 1 (shell, projection, Project Explorer), cycle 2 (`knx-store` entity persistence, [design spec](superpowers/specs/2026-09-03-knx-entity-persistence-design.md)), cycle 3 (`knx-desktop` save/load wiring), cycle 4 (device selection, properties inspector, undo/redo, [design spec](superpowers/specs/2026-09-04-selection-inspector-design.md)), cycle 5 (`Ctrl+K` search across devices, group addresses, building parts, [design spec](superpowers/specs/2026-09-04-search-design.md)), cycle 6 (`Ctrl+Shift+P` command palette, [design spec](superpowers/specs/2026-09-04-command-palette-design.md)), cycle 7 (System/Light/Dark theme toggle, [design spec](superpowers/specs/2026-09-04-dark-light-mode-design.md)), cycle 8 (project status dashboard, [design spec](superpowers/specs/2026-09-04-dashboard-design.md)), cycle 9 (group address create/delete: a "Group Addresses" tree branch with inline create, a Delete button on the group-address inspector, duplicate-address and still-linked-on-delete validation in `knx-core`) — CLAUDE.md's full UI/UX deliverable list complete as of cycle 9 — and cycle 10 (a toast notification stack replacing the old persistent error banner, humor-wrapped error text, and a one-shot holiday/late-night startup toast, [design spec](superpowers/specs/2026-09-05-toast-easter-eggs-design.md)) and cycle 11 (user-customizable theme tokens — accent/background/surface/text — plus a three-level motion setting, layered on top of the cycle 7 theme toggle, via a new `ThemePanel.tsx`) and cycle 12 (device and communication-object descriptions are now editable, not just displayed: `Command::SetDeviceDescription` and `Command::SetComObjectDescription`/`RestoreComObjectDescription` clone the `SetIndividualAddress`/`SetComObjectDpt` command-layer pattern exactly, wired through `knx-store::command_sync`, `knx-server`'s `/api/device-description` and `/api/com-object-description` routes, and two new `Inspector.tsx` fields; `ComObjectNode` also gains `description`/`description_layer` so a communication object's description — modelled and persisted since Session 5 cycle 2 but never shown — is finally visible at all. An ETS feature audit done alongside this found no other silently-missing field: `GroupAddress`/`GroupRange`/`BuildingPart` genuinely carry no `Description` attribute in the one schema-11 project this project's evidence comes from — see [KNOWN_LIMITATIONS.md #1](KNOWN_LIMITATIONS.md#1-single-sample-bias), not a bug here) and cycle 13 (a named, selectable theme replacing cycle 7's System/Light/Dark cycle and cycle 11's four-token palette override outright — a complete visual package, not a per-user tweak layered on a light/dark base — with "Bitcoin DeFi" as the first theme and today's default, [design spec](superpowers/specs/2026-09-08-bitcoin-defi-theme-design.md)): `theme.ts` rewritten from a cycling function into a `ThemeDef`/`THEMES` registry (`loadThemeId`/`saveThemeId`/`useThemeId`); `palette.ts`, `palette.test.ts`, and `ThemePanel.tsx` deleted outright; `ThemeToggle.tsx` replaced by `ThemeSwitcher.tsx`, a `<select>` built against the registry; `index.html` now always sets `data-theme` (Bitcoin DeFi is dark-only by design, no more "system"/unthemed state) and silently falls back cycle 7's old stored values to the new default; self-hosted `@fontsource` fonts and a Google Fonts `<link>` both load Space Grotesk/Inter/JetBrains Mono; and a full `styles.css` restyle — a 24-custom-property design-token layer plus component recipes app-wide (pill gradient/glow buttons, glass-morphism overlays, mono/gold technical text, gradient-text Dashboard heading, card hover-lift, fading grid-pattern background) done, see [ROADMAP.md](ROADMAP.md) |
| 6 | KNXnet/IP | Cycles 1-5 shipped (tunnelling, sending, discovery, routing, connection management/diagnostics). KNX IP Secure scoped, then shelved indefinitely (2026-09-06) — see [ROADMAP.md](ROADMAP.md), [KNOWN_LIMITATIONS.md §26](KNOWN_LIMITATIONS.md) |
| 7 | Integration & hardening | In progress (cycle 1) — see below |

**The repository is a buildable Cargo workspace with eight crates.**
`knx-core` holds the full domain model of [DATA_MODEL.md](DATA_MODEL.md):
identity (`ids.rs`), the provenance types `Layer`/`Resolved<T>`/`Override<T>`
(`provenance.rs` — `Override<T>` added in Session 3, [ADR-0010](adr/0010-per-attribute-override-representation.md)),
typed addresses (`address.rs`), datapoint type references (`dpt.rs`), the
string table (`string_table.rs`), flags and directional links (`flags.rs`),
commissioning state (`commissioning.rs`), group ranges/addresses
(`group.rs`), building parts (`building.rs`), topology (`topology.rs`),
devices and communication objects (`device.rs`, `parameter.rs`,
`devices.rs`), installation and project (`installation.rs`, `project.rs`),
validation rules (`validation.rs`), and the undo/redo command layer
(`command.rs`). Schema version bumped to 3 in Session 4 alongside
`knx-store`'s own (lockstep by design, [ADR-0003](adr/0003-sqlite-project-format.md)),
though the Rust shape of `Project` did not change. Session 5 cycle 2 bumps
it again to 4 for `knx-store`'s new entity persistence (below), and this
time does touch the Rust shape, but only by derive: `Project`, `Devices`,
`StringTable` and `IdAllocators` gain `#[derive(PartialEq)]`,
`StringTable` gains a public `iter()`, and `Devices` gains a public
`com_objects()` (enumerates every communication object instance,
including one an owning device's `com_objects` list doesn't name — what
`knx-store`'s `save_project` uses to detect and refuse an unreachable one
rather than silently drop it) — all additive, no behavior change (see
[DATA_MODEL.md §11](DATA_MODEL.md)). 49 tests.

**`knx-store` gains full entity persistence for `knx_core::Project`**
this cycle (schema v4, [design spec](superpowers/specs/2026-09-03-knx-entity-persistence-design.md)):
one persistence module per entity area — `strings.rs`
(`string_table_entry`), `topology.rs` (`installation`/`area`/`line`),
`building.rs` (`building_part`/`building_part_device`), `devices.rs`
(`device`/`binary_data_ref`/`com_object_instance`/`group_link`, plus the
`com_object_override` codec for all eight `Override<T>` attributes on
`ComObjectInstance`, normalized as one row per
`(com_object_instance_id, attr)` rather than wide columns), `group.rs`
(`group_range`/`group_address`), `parameter.rs` (`parameter_instance`) —
orchestrated by `project.rs`'s `save_project`/`load_project` (one
whole-project-replace transaction each way, not diffed) and
`command_sync.rs`'s `sync_after_command(conn, &Project, &Command)` for
incremental writes after a command. `PRAGMA foreign_keys = ON` is now set.
Still holds the schema-version migration chain (`migration.rs`, now
through v4), the opaque passthrough table (`opaque.rs`,
`insert_opaque`/`load_opaque`), and the manufacturer manifest table added
in Session 4 (`manifest.rs`, `insert_manufacturer_refs`/
`load_manufacturer_refs` — schema v3, [ADR-0011](adr/0011-product-database-storage.md)),
with four frozen fixtures (`fixtures/v1-empty.sqlite` through
`v4-empty.sqlite`). Incremental sync is honest, not complete: only the
four `Command` variants that exist today (`SetIndividualAddress`,
`SetComObjectDpt`/`RestoreComObjectDpt`, `CreateGroupAddress`/
`DeleteGroupAddress`) have a `sync_after_command` path — every other
entity and every other `Override<T>` attribute is written only by a full
`save_project`, until a command exists for it. A cross-task bug surfaced
and was fixed during this cycle: `strings.rs`'s `upsert_string_table`
originally opened its own `BEGIN`/`COMMIT` transaction internally, which
returned an error when called from inside another already-open transaction
(as `save_project` does); it now uses `SAVEPOINT`/`RELEASE`/`ROLLBACK TO`
instead, so it composes correctly both standalone and nested. A second,
more serious integration bug surfaced in final review and was fixed before
merge: `save_project` assumed `Installation::buildings`/`::group_ranges`
arrive in pre-order (parent before child) — `knx-etsproj`'s importer
actually produces post-order, which made `save_project` fail with a
foreign-key violation on every real project with a nested building or
group-range hierarchy, including this repo's own reference project.
Fixed with `PRAGMA defer_foreign_keys = ON` for the whole transaction
(which also made the `building_part`/`group_range` `parent_id`-neutralizing
statements from the original design redundant; removed). `knx-store` now
carries `knx-etsproj` as a `[dev-dependencies]` entry (not a layering
violation — `check-layering`'s rule is that `knx-etsproj` must not reach
`knx-store`, not the reverse, and a dev-dependency never enters the
production graph) so a reference-project round-trip test
(`tests/reference_project.rs`) can exercise exactly this path against the
real reference `.knxproj`. `save_project` also now refuses (rather than
silently drops) a device or communication-object instance that exists in
`Devices` but is unreachable from any topology/building list —
`StoreError::UnreachableDevices`/`UnreachableComObjects`. 51 tests (49
unit, 2 integration).

**Known limitation carried from this cycle** ([KNOWN_LIMITATIONS.md §17](KNOWN_LIMITATIONS.md)):
`knx_core::command::Command::DeleteGroupAddress` removes the
`GroupAddressEntry` but does not clean up any `GroupLink` left pointing at
it from a communication object — a `knx-core` command-layer gap, not a
`knx-store` one. `sync_after_command` still deletes the address correctly,
but a subsequent full `save_project` will fail with a foreign-key violation
against the dangling link. Fixing it means deciding cascade-delete vs.
block-the-delete vs. something else in `command.rs`'s own design,
deliberately not done in this cycle.

**`knx-etsproj` holds the full six-stage import/export pipeline**: the ZIP
container (`container.rs`, with a 64 MB per-entry size guard), schema
detection (`detect.rs`), the tolerant streaming parser for both `0.xml`
and `Project.xml` (`parse/`, `known.rs`'s schema-11 table), attribute value
conversions (`values.rs`), structural validation (`validate.rs`), the
mapper into `knx_core::Project` (`map.rs`), datapoint-type inference
(`infer.rs`), the opaque-entry collector (`opaque.rs` — now handing
manufacturer files out separately as `ManufacturerFile`, Session 4 Task
12), the import report (`report.rs`), orchestration
(`import_knxproj`/`import_knxproj_bytes` in `lib.rs`), schema-11 XML
writers and container export (`export/`), and the declared
semantic-equality comparison (`compare.rs`). 89 tests in the crate (68
unit, plus the golden, oracle, roundtrip and malformed-input integration
suites).

**`knx-productdb` is no longer an empty crate.** It owns its own SQLite
migration chain (`migration.rs`, v1) and parser, and depends on neither
`knx-etsproj` nor `knx-store` (the third `check-layering` root, ADR-0011):
the content-hashed blob store (`blob.rs`), streaming XML helpers
(`xml.rs`), the ingest report (`report.rs`), one parser module per
manufacturer file kind (`parse/catalog.rs`, `hardware.rs`, `program.rs`,
`comobject.rs`, `translation.rs`, `master.rs`), per-file orchestration with
a content-hash skip and one transaction per file (`ingest.rs`), the read
side (`query.rs`), and enrichment of `ComObjectInstance` from the
application program into `Override::Absent` slots only (`enrich.rs`,
[ADR-0012](adr/0012-enrichment-into-absent-slots.md)). 69 tests (60 unit,
plus the golden ingest of the reference project's manufacturer data, the
`xknxproject` oracle comparison, and the malformed-input suite).

**`knx-app` holds both the import and export services**
(`import.rs`: `import_ets_project`/`import_ets_project_with`;
`export.rs`: `export_ets_project`) — the one crate that sees
`knx-etsproj`, `knx-store` and `knx-productdb` together, enforced by
`check-layering`. `ImportOptions { product_db }` decides whether
manufacturer data routes through the shared product database (ingested
and enriched) or falls back to the project's own opaque store exactly as
Session 3 wrote it — both paths tested, including byte-identical export
either way. 7 tests.

**`apps/knx-cli`'s `import` subcommand gains `--product-db <path>` /
`--no-product-db`**, defaulting to `$XDG_DATA_HOME/knx/products.sqlite`
when neither flag is given, and a new **`knx products`** subcommand
(`list`, `ingest`, `show <program-id>`, `verify`) for inspecting the
database and ingesting a `.knxproj`'s manufacturer data separately from a
full import. 10 tests.

**`knx-projection` is a new crate this session**: a pure `Project` →
`ProjectTree` projection (`lib.rs`) with no IO of its own, `ts-rs`-derived
TypeScript bindings for the desktop frontend, and a dependency on nothing
but `knx-core` — the fourth `check-layering` root, held to the same
IO-free bar as `knx-core` itself. Cycle 4 adds `ProjectTree.can_undo`/
`.can_redo` (plain booleans the projection itself always sets `false`;
the desktop shell overlays the real `CommandStack` state after projecting,
below) and `build_device_detail`/`DeviceDetail`/`ComObjectNode` — the
properties-inspector projection for one device, resolving each
communication object's DPT through the same `Program`/`ProgramRef`/
`Instance`/`Inferred`/`UserEdit` layer order as the tree view, plus which
layer it resolved from (`dpt_layer`) and the read/write/transmit/update/
communication flags (display-only this cycle — no `Command` exists yet to
edit a flag). 16 tests.

Cycle 5 (`docs/superpowers/specs/2026-09-04-search-design.md`) adds
`GroupAddressNode` and a `group_addresses: Vec<GroupAddressNode>` field on
`InstallationNode`, projected from `Installation.group_addresses` and
formatted through `GroupAddress::format` per `project.info.
group_address_style` — the reference project's 514 group addresses are
cheap enough to embed eagerly, unlike `DeviceDetail`'s lazy communication
objects. 18 tests.

**`apps/knx-desktop` is the desktop shell**: Tauri v2 with a React + Vite
frontend, scaffolded this session rather than in Session 1. `open_project`
imports a `.knxproj`, projects it through `knx-projection`, and returns
the resulting `ProjectTree` to the frontend's Project Explorer; the store
connection it opens for that import is in-memory
(`knx_store::open_and_migrate_in_memory`) and discarded on drop — ETS
import never touches a `.knxdb` file. 1 test.

Cycle 3 adds knx-desktop's own persistence, a second and entirely
separate file format: `save_project`/`save_project_as`/
`open_native_project` persist/restore the in-memory `Project` as a
`.knxdb` SQLite file via `knx_store::{save_project, load_project}`.
`AppState` gains `store_path: Mutex<Option<PathBuf>>` so plain `Save`
knows where to write without asking again; `Save As…` and the `.knxdb`
variant of `Open` always go through a file dialog
(`@tauri-apps/plugin-dialog`'s `save`/`open`). A native load has no
`ImportReport` — nothing was reinterpreted from an external format — so
`ProjectTree.errors`/`.warnings` are genuinely zero, not merely
unmeasured. 1 test (round-trips the reference project through
`save_project_as` → `open_native_project` against the same golden counts
`open_reference_project.rs` already established for ETS import).

Cycle 4 adds device selection, a properties inspector, and undo/redo
([design spec](superpowers/specs/2026-09-04-selection-inspector-design.md)).
`AppState` gains `command_stack: Mutex<CommandStack>` (every applied
command's inverse; reset on `open_project`/`open_native_project`, never
persisted to `.knxdb` — undo history is session-only by design) and
`import_counts: Mutex<(usize, usize)>` (the initial import's error/warning
counts, reapplied to every tree rebuilt after a command/undo/redo, since
an edit doesn't change what import lost). Five new Tauri commands:
`device_detail` projects one device (`knx_projection::build_device_detail`)
for the Inspector panel; `set_individual_address`/`set_com_object_dpt`
parse the frontend's string input into `knx_core::IndividualAddress`/
`DptRef`, wrap it in a `knx_core::Command`, and run it through the shared
`apply` helper (`do_command` on the stack, then re-project with
`tree_with_state` overlaying `can_undo`/`can_redo`/the carried-forward
import counts); `undo`/`redo` call the same overlay after `CommandStack::
undo`/`redo`. Every command is split `<name>_impl(state, ...)` /
`#[tauri::command] fn <name>(...)`, so `command_dispatch.rs` and
`device_detail.rs` exercise the logic without any running `tauri::App`.
`ProjectExplorer` gained `selectedId`/`onSelectDevice` props (a clicked
device row calls back into `App.tsx`'s `selectDevice`, which fetches
`device_detail` and guards the response with a `selectedDeviceIdRef`
against a stale reply landing after the selection has moved on — the same
guard covers `handleTreeUpdate`'s post-command refetch). The new
`Inspector.tsx` renders the selected device's name/description, an
address field, and one DPT field per communication object with its
resolved-layer badge; each field applies on blur or Enter, no-ops if the
value didn't change, and reverts to the last-known-good value with an
inline error on a rejected edit. The toolbar gains Undo/Redo buttons
(disabled from `tree.can_undo`/`.can_redo`) and a window-level `Ctrl+Z`/
`Ctrl+Shift+Z` keyboard shortcut. 9 `knx-desktop` integration tests
total: 7 new this cycle (`command_dispatch.rs` — 5, `device_detail.rs` —
2) plus the 2 pre-existing (`open_reference_project.rs`,
`save_load_roundtrip.rs`) unchanged.

Cycle 5 (`docs/superpowers/specs/2026-09-04-search-design.md`) adds
`Ctrl+K` search across devices, group addresses, and building parts.
`selection.ts`'s `Selection` generalizes from device-only to a union over
the three kinds; `ProjectExplorer` and `Inspector` generalize their
selection/render paths accordingly, with group addresses and building
parts rendered read-only in the Inspector (no `Command` exists for either
yet). `treeUtils.ts`'s `buildSearchIndex` flattens `ProjectTree` into one
`SearchEntry[]` per kind (deduplicating a device that appears under both
topology and a building), and `searchMatch.ts`'s `matchEntries` ranks
exact label matches first, then starts-with, then contains-only,
alphabetically within a rank, over label plus each kind's own second
field (address or breadcrumb path), capped at 50 results. `Search.tsx` is
the overlay component itself — autofocus, `Escape` to close, arrow keys
and `Enter` to navigate/pick against the same grouped-by-kind order the
list renders in, click to pick. This is also the first cycle with a
frontend test runner: `package.json` gains a `vitest` test script, and CI
now runs it. 14 `vitest` tests (`treeUtils.test.ts`, `searchMatch.test.ts`)
alongside the existing 9 Rust integration tests, unchanged.

Cycle 6 (`docs/superpowers/specs/2026-09-04-command-palette-design.md`)
adds the `Ctrl+Shift+P` command palette. `commandRegistry.ts` is a new,
IO-free module: `COMMANDS`, a static array of the seven actions
`App.tsx` already exposes as toolbar buttons, each an `id`/`label`/
optional `shortcutHint`/`isEnabled(ctx)`/`run(ctx)` record against a
`CommandContext` of `tree` plus the seven existing callback functions;
and `filterCommands`, a plain case-insensitive substring match over each
command's label — no fuzzy ranking, since seven static entries need no
scoring algorithm. `isEnabled` mirrors, by hand, the `disabled` condition
on each command's toolbar button (`!tree`, `!tree?.can_undo`,
`!tree?.can_redo`); the two are not derived from each other and must be
kept in sync if either changes. `CommandPalette.tsx` reuses `Search.tsx`'s
modal-overlay structure (autofocused input, click-outside-to-close,
`Escape`/arrow-key/`Enter` handling) as one flat, ungrouped list; rows
where `isEnabled` is false render with a `disabled` class and
`aria-disabled="true"`, are skipped by `ArrowUp`/`ArrowDown` traversal,
and are a no-op on click or `Enter`. `App.tsx` gains `paletteOpen` state,
a `Ctrl+Shift+P`/`Cmd+Shift+P` keyboard branch, a `ctx: CommandContext`
built as a plain object literal each render (deliberately not
memoized, matching every other prop `App.tsx` passes its children), and
a toolbar button ("Commands… (Ctrl+Shift+P)",
never `disabled`, same as the two Open buttons) alongside the existing
"Search… (Ctrl+K)" button. `Ctrl+K` and `Ctrl+Shift+P` are mutually
exclusive in both directions — each keyboard branch closes the other
overlay before opening its own — and the palette works with no project
loaded. 9 new `vitest` tests in `commandRegistry.test.ts`, for 23 total
alongside the existing 9 Rust integration tests, unchanged.

Cycle 7 (`docs/superpowers/specs/2026-09-04-dark-light-mode-design.md`)
adds a three-state (System/Light/Dark) theme toggle. `theme.ts` is a new,
mostly IO-free module: `nextTheme` cycles System → Light → Dark → System;
`loadTheme`/`saveTheme` take an injected `localStorage`-shaped object
rather than reading the global, which is what makes them unit-testable
under Vitest's `environment: "node"`; `useTheme()` composes them into a
hook that applies `data-theme` to `<html>` (removed entirely for
"system", so a `prefers-color-scheme` media query governs) and persists
on every change. `ThemeToggle.tsx` is a hand-written inline-SVG icon
button (sun/moon/monitor) — no icon-library dependency, matching
`commandRegistry.ts`'s no-fuzzy-ranking restraint from cycle 6.
`styles.css` gains three custom properties (`--knx-error-color`,
`--knx-overlay-backdrop`, `--knx-overlay-shadow`) for the only three
colors that don't already adapt via `currentColor`/system color
keywords, with a `prefers-color-scheme` default and `:root[data-theme]`
overrides that outrank it by CSS specificity (an attribute selector on
`:root` beats a bare `:root` inside a media query) regardless of source
order. `index.html` gets a small inline script applying a persisted
explicit choice before React mounts, avoiding a one-frame flash of the
wrong theme; it necessarily duplicates `theme.ts`'s storage key and
valid-value literals by hand, since it runs before any module graph
exists. 5 new `vitest` tests in `theme.test.ts`, for 28 total alongside
the existing 9 Rust integration tests, unchanged.

Cycle 11 adds user-customizable theme tokens and motion, layered on top
of cycle 7's System/Light/Dark cycle rather than replacing it. `palette.ts`
is a new, mostly IO-free module mirroring `theme.ts`'s shape: a closed set
of four `TOKENS` (`accent`, `bg`, `surface`, `text`), `loadPalette`/
`savePalette` against an injected `localStorage`-shaped object (unit-tested
under `environment: "node"`, no DOM needed), and `applyPalette` writing one
CSS custom property per token as an inline style on `<html>` — present only
for the tokens the user has actually overridden, so an unset token falls
through to the base theme's own value via ordinary CSS cascade rather than
needing its own removal-tracking logic. Motion is a three-level
`off`/`subtle`/`standard` setting stored alongside the colors, mapped to a
`--knx-transition-duration` custom property; `styles.css` only applies that
duration inside a `prefers-reduced-motion: no-preference` block, so the OS
setting always overrides the user's motion choice, never the other way
round. `styles.css` also gains three new base tokens (`--knx-accent`,
`--knx-bg`, `--knx-surface` — `--knx-text` makes four) defaulting to the
`AccentColor`/`Canvas`/`CanvasText` system keywords, and every previously
hardcoded `Canvas`/`CanvasText` usage (`.toast--error`, `.toast--fun`,
`.search-panel`, `.fs-picker`) now reads through one of them instead, plus
the two selection highlights (`.tree-label.selected`,
`.search-result.selected`) now tint from `--knx-accent` instead of
`currentColor` — the only two places an accent override was otherwise
invisible. `usePalette()` composes the above into a hook shaped like
`useTheme()`'s but exposing per-field setters (`setColor`, `setMotion`,
`resetAll`) rather than a single cycle function, since a settings panel
edits one field at a time. `ThemePanel.tsx` is a small modal (a color
`<input>` per token with a per-token reset button, a motion `<select>`, a
reset-all button) opened from a new gear button next to `ThemeToggle` in
`App.tsx`; like `ThemeToggle.tsx` it has no dedicated component test — the
codebase's established split is that `.tsx` files are untested render
wiring, and the logic underneath (`palette.ts`) carries the unit tests. 9
new `vitest` tests in `palette.test.ts`, for 64 total.

`knx-net`, `knx-secure` remain empty crates with their responsibility
stated in a doc comment. There is still no manufacturer parameter
*interpretation* (the `Dynamic` tree, `when/@test`), and the desktop UI so
far covers the Project Explorer (including a "Group Addresses" branch
with inline create, cycle 9), the four save/load/import buttons, a
properties inspector with undo/redo for two editable fields (individual
address, communication-object DPT) plus group address create/delete
(cycle 9), `Ctrl+K` search over devices, group addresses, and building
parts, a `Ctrl+Shift+P` command palette over the app's seven existing
actions, a System/Light/Dark theme toggle with user-customizable accent/
background/surface/text tokens and motion (cycle 11), a read-only project
status dashboard shown when nothing is selected (cycle 8), and a toast
notification stack with humor-wrapped error text and a one-shot
holiday/late-night startup toast (cycle 10) — no editing of anything
beyond the individual address, communication-object DPT, and group
address entities yet.

Three architectural rules are enforced mechanically rather than by
discipline, and all three have been observed to fail on a deliberate
violation:

- `cargo run -p xtask -- check-layering` — `knx-core` reaches none of
  `serde_json`, `quick-xml`, `rusqlite`, `tokio`; `knx-etsproj` reaches no
  `knx-store`; `knx-productdb` reaches neither `knx-etsproj` nor
  `knx-store` (Session 4); `knx-projection` reaches none of the same
  packages forbidden to `knx-core` (Session 5, the fourth root).
- `cargo deny check` — no licence outside the allowlist enters the graph; GPL
  is not on the allowlist.

314 Rust tests passed across the workspace as of cycle 10, plus 46
`vitest` tests in `apps/knx-desktop` (run separately, `npm test`, not part
of `cargo test --workspace`) — up from 32 with cycle 10's new
`toast.test.ts` (14 tests: `isLateNight`, `findHoliday`,
`pickStartupToast`, `humorizeError`).

**Web/Docker deployment target** (cross-cutting, added alongside Session 5
rather than as one of its cycles — not on the original Session 0-7
roadmap, see [ROADMAP.md](ROADMAP.md) and
[the design spec](superpowers/specs/2026-09-05-web-docker-deployment-design.md)).
`apps/knx-desktop`'s Tauri IPC layer (the `#[tauri::command]` wrappers that
used to live in `src-tauri/src/lib.rs`) is gone. All of it — routes, state,
the `_impl` functions underneath — moved into a new crate, **`apps/knx-server`**:
an axum HTTP API serving `/api/*` (one route per former Tauri command,
plus new web-only routes for directory listing, upload and `.knxdb`
download under `/api/fs/*` and `/api/project/download`, and `/healthz` for
container orchestration) and, when a `static_dir` is supplied to
`knx_server::app`, the built frontend as a `tower_http::ServeDir`
fallback. The frontend itself moved, file-for-file, from
`apps/knx-desktop/src` to a new npm package, **`apps/knx-web`**; its API
client (`api.ts`) is rewritten from Tauri's `invoke()` to plain `fetch()`
against the same routes, and a new `FsPicker.tsx` provides the
mount-directory listing/upload UI for the web build. Native file
dialogs stay native on desktop: `@tauri-apps/plugin-dialog` still runs
behind a `window.__TAURI__` check in `apps/knx-web/src/filePicker.ts`; only
the web build (no `__TAURI__`) falls back to `FsPicker.tsx`.
`/api/project/download` exists and is tested server-side (see
`http_fs_routes.rs`) but has no frontend caller yet — `FsPicker.tsx` only
wires up listing and upload; see
[KNOWN_LIMITATIONS.md #26](KNOWN_LIMITATIONS.md#26-apiprojectdownload-has-no-frontend-caller).
`apps/knx-desktop/src-tauri` is now an 80-line thin wrapper (`lib.rs`): it
spawns `knx-server`'s router in-process — a fixed dev port
(`knx_server::DEV_PORT`, the target of `apps/knx-web`'s Vite dev proxy) in
debug builds, an OS-assigned ephemeral port serving the bundled frontend
resource in release builds — and points one `WebviewWindowBuilder` at
whichever URL results. Command-level integration test coverage that used
to live in `knx-desktop/src-tauri`'s own tests moved with the logic to
`apps/knx-server/tests` (`command_dispatch.rs`, `device_detail.rs`,
`open_reference_project.rs`, `save_load_roundtrip.rs`, unchanged in
substance, now exercised over HTTP via `tower::ServiceExt::oneshot` where
applicable) alongside new route-level tests (`healthz.rs`,
`http_device_detail.rs`, `http_edit_routes.rs`, `http_fs_routes.rs`,
`http_project_routes.rs`) — `knx-desktop/src-tauri` itself carries none
today. `apps/knx-server/src/main.rs` reads `KNX_PORT`/`KNX_STATIC_DIR`/
`KNX_DATA_DIR` from the environment, the last falling back to the OS temp
dir if unset, so `/api/fs/*` and `/api/project/download` respect a mounted
Docker volume in production; this was a real gap found and fixed mid-plan
— the original `main.rs` never read `KNX_DATA_DIR` at all, despite the
Dockerfile already setting `ENV KNX_DATA_DIR=/data`. Docker packaging
(`apps/knx-server/Dockerfile`) is a three-stage build — `node:20-alpine`
builds the frontend, `rust:1.98-slim` builds the release binary,
`debian:bookworm-slim` ships just the binary and static assets, no
GTK/WebKit2GTK anywhere in the image — verified by
`apps/knx-server/scripts/smoke-test.sh` (builds the image, runs the
container against a mounted `/data`, imports the reference project over
HTTP, asserts zero errors in the response). `.github/workflows/ci.yml`'s
frontend steps and ts-rs staleness check were updated to point at
`apps/knx-web` (they still named `apps/knx-desktop` after the move, which
would have failed CI on the next push — caught and fixed in this same
pass, not a separate finding left for later). 327 Rust tests now pass
across the workspace (up from 314), plus 64 `vitest` tests in
`apps/knx-web` across 9 files (up from 46 in `apps/knx-desktop`: moved
1-for-1 plus a new `api.test.ts` against a mocked `fetch`, then cycle 11's
`palette.test.ts`).

## What exists

| Path | Purpose |
| --- | --- |
| `Cargo.toml`, `rust-toolchain.toml` | Workspace root; toolchain pinned to Rust 1.98.0. |
| `crates/knx-core/` | Domain model per [DATA_MODEL.md](DATA_MODEL.md), sections 1–9 and 11. No IO. |
| `crates/knx-store/` | SQLite schema-version migration chain through v5 (`migration.rs`), the opaque passthrough table (`opaque.rs`), the manufacturer manifest table (`manifest.rs`, Session 4), full `knx_core::Project` entity persistence (`project.rs`, `strings.rs`, `topology.rs`, `building.rs`, `devices.rs`, `group.rs`, `parameter.rs`, `command_sync.rs` — Session 5 cycle 2), and four frozen fixtures. |
| `crates/knx-etsproj/` | The full six-stage `.knxproj` import/export pipeline — see the Session 3 paragraph above. Hands manufacturer files out separately from opaque entries (Session 4). No dependency on `knx-store`. |
| `crates/knx-productdb/` | The shared product database: own SQLite migration chain, streaming manufacturer-XML ingest, and enrichment of `ComObjectInstance` — see the Session 4 paragraph above. No dependency on `knx-etsproj` or `knx-store`. |
| `crates/knx-projection/` | Pure `Project` → `ProjectTree` projection with `ts-rs` TypeScript bindings, including `GroupAddressNode` on `InstallationNode` (Session 5 cycle 5) — see the Session 5 paragraph above. No dependency beyond `knx-core`; the fourth `check-layering` root. |
| `crates/knx-app/` | The import and export services (`import.rs`, `export.rs`) — the one crate that sees `knx-etsproj`, `knx-store` and `knx-productdb` together. |
| `crates/knx-net/`, `knx-secure/` | Empty crates with their responsibility stated in a doc comment. `knx-secure` deliberately has no dependencies at all. |
| `apps/knx-cli/` | Headless entry point, binary `knx`. `import` subcommand (Session 3, `--product-db`/`--no-product-db` added Session 4) and `products` subcommand (Session 4); prints its version otherwise. |
| `apps/knx-server/` | **New, web/Docker deployment target.** The axum HTTP API binary (`knx-server`) and library (`knx_server`) — see the paragraph above. `src/domain.rs` holds `AppState` and the same `_impl` functions the old Tauri commands wrapped; `src/routes.rs`/`fs_routes.rs` are the axum route handlers; `src/errors.rs` maps `AppError` to an HTTP status plus a `{"error": ...}` body. `main.rs` reads `KNX_PORT`/`KNX_STATIC_DIR`/`KNX_DATA_DIR` from the environment. `Dockerfile` is the three-stage build (Node frontend, Rust backend, Debian-slim runtime); `scripts/smoke-test.sh` builds and runs the image and exercises `/healthz` plus an import over HTTP. |
| `apps/knx-web/` | **New, moved from `apps/knx-desktop/src`.** The React + Vite frontend, now a standalone npm package consumed by both `knx-server`'s static-file serving and the Tauri desktop shell. `src/api.ts` is the `fetch()`-based client (replaces Tauri's `invoke()`); `src/FsPicker.tsx` is the mount-directory listing/upload UI shown when `window.__TAURI__` is absent (the server's `/api/project/download` route has no UI caller yet, see [KNOWN_LIMITATIONS.md #26](KNOWN_LIMITATIONS.md#26-apiprojectdownload-has-no-frontend-caller)); `src/filePicker.ts` picks between it and the native Tauri dialog. `src/theme.ts`/`src/ThemeSwitcher.tsx` are cycle 13's named-theme registry and `<select>` picker, replacing cycle 7's `theme.ts`/`ThemeToggle.tsx` cycle and cycle 11's now-deleted `palette.ts`/`ThemePanel.tsx` token overrides outright — see the Session 5 paragraph above. Everything else (`ProjectExplorer`, `Inspector`, `Search.tsx`/`CommandPalette.tsx`, `Dashboard.tsx`, `Toast.tsx`, the `ts-rs`-generated bindings under `src/bindings/`) moved unchanged from `knx-desktop`. `vitest` suite: 89 tests across 8 files, including `api.test.ts` against a mocked `fetch` and cycle 13's rewritten `theme.test.ts` (`palette.test.ts` is gone with `palette.ts`). |
| `apps/knx-desktop/` | **Thin native wrapper as of the web/Docker deployment target** — see the paragraph above. `src-tauri/` is now just window/process wiring (`lib.rs`, ~80 lines): spawn `knx-server`'s router locally, point one `WebviewWindowBuilder` at it, keep the native file-dialog plugin available for `apps/knx-web`'s `window.__TAURI__` check. No `#[tauri::command]` handlers and no integration tests remain here — both moved to `apps/knx-server`. No `src/` of its own any more; it loads `apps/knx-web`'s build output (dev: Vite HMR on a fixed port; release: bundled as a Tauri resource). |
| `xtask/` | Repository verification tasks. `check-layering` walks the resolved dependency graph and reports the shortest path to any forbidden package, for four roots (`knx-core`, `knx-etsproj`, `knx-productdb` — the third added Session 4 — and `knx-projection`, the fourth, added Session 5); `freeze-fixture` (Session 3) regenerates a canonical migration-test fixture. |
| `deny.toml` | Licence, advisory, ban and source policy for `cargo-deny`. |
| `.github/workflows/ci.yml` | CI: Tauri Linux prerequisites and Node.js setup (Session 5), formatting, clippy with `-D warnings`, tests, `knx-web`'s own `npm test` (Vitest, Session 5 cycle 5; path updated from `knx-desktop` to `knx-web` with the web/Docker deployment target), the layering gate, `cargo deny check`, and a check that `knx-projection`'s `ts-rs` bindings under `apps/knx-web/src/bindings` are not stale (Session 5; path likewise updated). Does not build or smoke-test the `knx-server` Docker image — that stays a local/manual step (`apps/knx-server/scripts/smoke-test.sh`), not yet wired into CI. |
| `docs/ARCHITECTURE.md` | Layering, workspace layout, enforced rules, core approach, UI boundary, KNXnet/IP, key material, test strategy. |
| `docs/DATA_MODEL.md` | The target domain model, per section marked implemented / planned / retained-but-uninterpreted. |
| `docs/IMPORT_EXPORT.md` | The six-stage pipeline, container handling, tolerant parsing, opaque store, import report, export rules, roundtrip guarantees. |
| `docs/COMPATIBILITY.md` | What is verified, what is expected but unverified, what is not supported — every verified row now names the test that verifies it. |
| `docs/KNOWN_LIMITATIONS.md` | Twenty-five limitations with cause, impact and the condition that would lift each. |
| `docs/ROADMAP.md` | Sessions 2–7 with deliverables and entry conditions. |
| `docs/adr/` | Ten ADRs, a template and an index. |
| `docs/RESEARCH.md` | Session 0 result plus Session 3 amendments: verified findings on the `.knxproj` format, manufacturer data, master data, KNXnet/IP, KNX Secure, licensing, risks. |
| `tools/inspect_knxproj.py` | Stdlib-only inspector that reproduces every container/project number quoted in `RESEARCH.md`. |
| `monitor_bus.py` | Captures live telegrams from the KNXnet/IP gateway into `bus_traffic.jsonl`. |
| `Unser Zuhause ets4 - 2025-12-15.knxproj` | Real ETS 4.1.8 reference project (schema 11), unprotected. |
| `Unser Zuhause ets 6.3.0 - 2026-09-02.knxproj` | Same installation, re-exported unchanged from ETS 6.3.7959.0 (schema 23), unprotected. Diffed against the ETS4 export in [RESEARCH.md §2.4/§3.3](RESEARCH.md#24-container-differences-ets4-schema-11-vs-ets6-schema-23-v). |
| `project_dump.json`, `group_addresses.json`, `devices.json` | `xknxproject` output for the same project — a cross-check baseline, known to be lossy (RESEARCH.md §7.1). |
| `bus_traffic.jsonl` | 280 captured live telegrams (gitignored). |

## Environment

* Rust 1.98.0, pinned in `rust-toolchain.toml`, with `rustfmt` and `clippy`.
* `cargo-deny` 0.20.2 — `cargo install --locked cargo-deny`.
* Python 3.14 venv at `.venv/`. Use `.venv/bin/python`.
* `xknx` 3.20.0 (MIT) — live bus access.
* `xknxproject` 3.10.0 (**GPL-2.0-only**) — reference and test use only. It must
  never become a runtime dependency; the Rust graph cannot reach it, and
  `cargo deny check` enforces the licence rule independently. See RESEARCH.md
  §10 and [ADR-0002](adr/0002-own-knxproj-parser.md).
* KNXnet/IP gateway at `192.0.2.1:3671`, tunnelling verified working.

Everything CI runs is runnable locally with the same command:

```bash
cargo build --workspace
cargo test --workspace
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo run -p xtask -- check-layering
cargo deny check
```

**Session 6, Cycle 1 (2026-09-06) — KNXnet/IP read-only tunnelling.**
`crates/knx-net` was built with five codec modules (`frame.rs`, `core/hpai.rs`,
`core/services.rs`, `tunnelling.rs`, `cemi.rs`), unit tests for each, a
`TunnelClient` state machine managing the connect/heartbeat/receive/disconnect
lifecycle, an `#[ignore]`d live-gateway integration test (first of its kind —
a pattern for future hardware-dependent tests), and the `knx bus monitor`
CLI subcommand resolving live telegrams against the project's group addresses.
The implementation is built directly from the KNX Association specification
(Core v01.06.02 AS, Tunnelling v01.07.01 AS, EMI_IMI v01.04.02 AS), not from
reading `xknx` or other stacks. Live-gateway verification ran 2026-09-06
against a real gateway (`cargo test -p knx-net -- --ignored`) and found one
real bug (see below) — now fixed and re-verified: connect, heartbeat,
receive and disconnect all confirmed against physical hardware, not just
unit tests.

**Bug found and fixed by live verification:** `encode_connect_request`
built the CONNECT_REQUEST body as control HPAI, CRI, data HPAI — which is
the order Core v01.06.02 AS §7.8.1's prose describes, but not what any
interoperable stack actually puts on the wire. The real gateway silently
dropped every such request (no CONNECT_RESPONSE, ever — indistinguishable
from a dead gateway or a firewall). Comparing against `xknx`'s
`ConnectRequest.to_knx()` (control HPAI, data HPAI, CRI) and confirming
with a hand-crafted UDP packet pinned the order; switching to it fixed the
timeout immediately. Lesson for future codec work against this spec:
where the prose and the wire disagree, trust the wire — cross-check a new
service against a second interoperable implementation before assuming the
prose is unambiguous.

Known gaps added this cycle (not bugs, scope decisions):

- `TunnelClient`'s heartbeat retry logic has a narrow, low-probability race
  condition where a stale wakeup can burn one retry attempt. Not fixed this
  cycle; no observed impact; noted for a future hardening pass.
- `apps/knx-cli`'s `bus monitor` always formats group addresses as
  three-level and merges names across installations into one flat map
  (last-seen wins on collision). Acceptable for this cycle's dev/smoke-testing
  use; not yet a general-purpose tool.

**Session 6, Cycle 2 (2026-09-06) — KNXnet/IP sending.** Bounded task,
brainstormed directly in chat (no separate design spec, per the
brainstorming skill's classification: extends Cycle 1's existing
`TunnelClient::send` stub and `bus` CLI subcommand rather than introducing
a new subsystem). `cemi::encode_l_data` is the inverse of Cycle 1's
`decode_l_data` — round-trip-tested for `GroupValueWrite`/`GroupValueRead`,
group and individual destinations, and the `Other` APCI fallback.
`TunnelClient::send(destination, service)` implements Tunnelling
v01.07.01 AS §2.6's rule exactly: send, wait up to the 1-second
`TUNNELLING_REQUEST_TIMEOUT` for a matching `TUNNELLING_ACK`, repeat once
on timeout or error status, then terminate the connection
(`DISCONNECT_REQUEST` + background-task shutdown) if the repeat also
fails. `receive_loop` gained the `TUNNELLING_ACK` match arm this required
(previously silently ignored by its catch-all). No DPT interpretation —
callers pass raw `GroupValue::Short`/`Bytes`, same scope cut as Cycle 1's
receive side. `knx bus write --gateway <host:port> <ga> <0|1|hex>` is the
CLI-facing piece; `crates/knx-net/tests/live_gateway.rs` gained a second
`#[ignore]`d test that only runs against an explicit, human-chosen
`KNX_TEST_GA` env var — deliberately not defaulted to any address in the
live project, since a `GroupValueWrite` physically actuates whatever it's
linked to. Live-hardware verification of `send` itself (as opposed to the
unit/round-trip tests above) was left for whoever sets `KNX_TEST_GA` —
not run as part of this cycle's own verification, since choosing a safe
target address is a human decision, not this session's to make. Run
2026-09-06 with `KNX_TEST_GA=0/0/1` (user-chosen): `TUNNELLING_ACK`
received and matched, confirming `send`'s wire format and ack-correlation
against the real gateway, not just the round-trip tests above.

**Session 6, Cycle 3 (2026-09-06) — KNXnet/IP discovery.** Own design
spec (`docs/superpowers/specs/2026-09-06-knxnet-ip-discovery-design.md`),
per the brainstorming skill's classification: a new subsystem, not an
extension of Cycle 1/2's existing tunnel connection. `core::dib` decodes
the Device Info and Supported Service Families DIBs a `SEARCH_RESPONSE`
carries; `discovery` encodes `SEARCH_REQUEST` and decodes
`SEARCH_RESPONSE` bodies, skipping any DIB type it doesn't recognize
rather than rejecting the whole response. `KnxNetIpClient::discover` no
longer returns `BusError::NotImplemented`: it multicasts one
`SEARCH_REQUEST` to `224.0.23.12:3671` and collects replies for the full
spec `SEARCH_TIMEOUT` (10s), deduping by control endpoint. Only the
original `SEARCH_REQUEST`/`SEARCH_RESPONSE` form is implemented — not
`SEARCH_REQUEST_EXTENDED`/`RESPONSE_EXTENDED` (Core v2) — and the
multicast address/timeout are hardcoded constants, not CLI flags, both
deliberate scope cuts recorded in the design spec. `knx bus discover`
(no arguments — that's the feature) is the CLI-facing piece.
`crates/knx-net/tests/live_gateway.rs` gained a third `#[ignore]`d test
that multicasts for real and checks the reference gateway both answers
and advertises tunnelling support. Live-hardware verification was left
for the user to run, same as Cycle 2's `send` — choosing when to probe
the LAN isn't this session's call to make unsupervised.

Known gaps added this cycle (not bugs, scope decisions):

- `SEARCH_REQUEST_EXTENDED`/`RESPONSE_EXTENDED` (Core v2) are not
  implemented — no gateway encountered so far has needed them.
- The discovery multicast group/port and the collection timeout are
  hardcoded constants; no CLI override exists yet.
- Discovery does not work unmodified inside the `knx-server` Docker
  container (needs `--network host`) — a known, not-yet-solved
  constraint (ROADMAP.md, Session 6 entry); `knx-server` does not call
  `discover` yet, so nothing regresses, but the gap is now reachable from
  a CLI a container user might reasonably try.

**Session 6, Cycle 4 (2026-09-06) — KNXnet/IP routing.** Own design spec
(`docs/superpowers/specs/2026-09-06-knxnet-ip-routing-design.md`), per the
brainstorming skill's classification: a new subsystem, not an extension
of the existing tunnel connection. `routing.rs` decodes
`ROUTING_LOST_MESSAGE`/`ROUTING_BUSY` far enough to log them;
`ROUTING_INDICATION` needed no new codec at all, since its body is
exactly an `L_Data.ind` cEMI frame — the same `cemi::encode_l_data`/
`decode_l_data` Cycles 1-2 already built. `RoutingClient` joins the
standard routing multicast group (`224.0.23.12:3671`, shared with
discovery's default) with multicast loopback disabled, and implements
`BusConnection::connect_routing(own_address)` — `own_address` is a
required parameter, not negotiated, since routing has no
`CONNECT_REQUEST`/`CRD` handshake to assign one through the way
tunnelling does. `RoutingClient::send` is a single unconfirmed multicast
send with no ACK wait and no retry (Routing v01.05.02 AS §5.1 marks the
service unconfirmed outright, unlike Tunnelling's `TUNNELLING_REQUEST`/
`ACK` pair). `knx bus route-monitor --source-address <addr>` and
`knx bus route-send --source-address <addr> <ga> <value>` are the
CLI-facing pieces. Unlike Cycles 1-3, this cycle's core round trip
(`RoutingClient` send/receive) is tested on loopback multicast directly —
no real gateway needed, since routing is plain UDP multicast rather than
a protocol exchange with one specific peer.

Known gaps added this cycle (not bugs, scope decisions):

- No `--multicast` override for a non-default routing multicast address —
  hardcoded to the standard group (KNOWN_LIMITATIONS.md #31).
- `ROUTING_BUSY` is decoded and logged, never used to throttle sends
  (KNOWN_LIMITATIONS.md #32).
- The loopback round-trip test for `RoutingClient` send/receive skips
  gracefully in a sandbox lacking multicast loopback, which cannot
  distinguish it from a real regression (KNOWN_LIMITATIONS.md #33).
- Whether the reference gateway (`192.0.2.1`) supports routing at all
  is unconfirmed — tunnelling and discovery are verified against it,
  routing isn't yet. Manual verification (same policy as Cycles 2-3: a
  human chooses when to probe the LAN) is left for the user:
  `cargo run -p knx-cli -- bus route-monitor --source-address <spare-address>`
  against a running installation.

**Session 6, Cycle 5 (2026-09-06) — Connection management and diagnostics
hardening.** Bounded task, brainstormed directly in chat (per the
brainstorming skill's classification: three already-documented gaps in
existing `TunnelClient`/`RoutingClient` flows, not a new subsystem) after
KNX IP Secure — the other item on Session 6's remaining backlog — was
explicitly shelved for a later cycle. Closes KNOWN_LIMITATIONS.md #27,
#28 and #32:

- **#27 (heartbeat retry race).** `send_heartbeat_with_retries` and
  `TunnelClient::send`'s ack wait both replaced a single
  `timeout(..., notify.notified())` with the new shared `wait_for_reply`
  helper, which loops on the same deadline instead of returning on the
  first wakeup — a stale `Notify` permit from a reply that arrived just
  after a previous attempt gave up can no longer be mistaken for a
  timeout and burn an attempt early.
- **#28 (no shutdown signal to subscribers).** `TunnelClient::subscribe()`
  now yields `TunnelEvent` (`Telegram(LDataFrame)` or `Closed`) instead of
  a bare `LDataFrame`. `receive_loop` sends one `Closed` event as its last
  action, reached from every exit path (explicit `disconnect()`, the
  heartbeat loop exhausting its retries, a dead socket, or a
  server-initiated `DISCONNECT_REQUEST`) since they all funnel through
  that loop before it returns. `apps/knx-cli`'s `bus monitor` prints
  "gateway closed the tunnel" on it instead of sitting in silence
  indistinguishable from a quiet bus.
- **#32 (`ROUTING_BUSY` not honored).** `RoutingState` gained a
  `busy_until` deadline that `routing_receive_loop` extends on each
  `ROUTING_BUSY` received (`merge_busy_deadline`: the higher of the
  remaining time already in effect and the new frame's `tw`, per Routing
  v01.05.02 AS §2.3.5) and `RoutingClient::send` waits out before
  transmitting. The spec's optional `trandom` back-off (a `MAY`) is not
  implemented — only the mandatory stop-and-wait rule (a `SHALL`) is.

Four new tests in `crates/knx-net/src/client.rs`:
`wait_for_reply_survives_a_stale_non_matching_wakeup` and
`wait_for_reply_times_out_when_nothing_ever_matches` exercise the race
fix without any networking; `merge_busy_deadline_keeps_the_later_of_the_two`
checks the pure deadline-merge rule; `routing_client_send_waits_out_a_routing_busy_deadline`
confirms `send()` actually blocks on it end-to-end over loopback
multicast (skipped, not failed, if this sandbox has no multicast route,
same policy as Cycle 4's round-trip test). All three touched call sites
(`apps/knx-cli/src/main.rs`'s `bus monitor`, and
`crates/knx-net/tests/live_gateway.rs`'s live-gateway test) were updated
for `TunnelEvent`; `bus route-monitor` is unaffected, since it subscribes
to `RoutingClient`, not `TunnelClient`.

Known gaps carried forward (not new, restated for context): #31 (no
custom routing multicast address) and #33 (the loopback round-trip test's
environment-dependent skip) remain open — out of this cycle's scope,
which was specifically the three gaps above.

**Topology & group-range command layer (2026-09-06).** `knx-core::command`
gains ten `Command` variants closing the first item of
[GAP_ANALYSIS_ETS.md](GAP_ANALYSIS_ETS.md)'s Tier 1 backlog: `CreateArea`/
`DeleteArea`, `CreateLine`/`DeleteLine`, `MoveDeviceToLine`,
`CreateGroupRange`/`DeleteGroupRange`/`RenameGroupRange`, and
`LinkComObject`/`UnlinkComObject` — each following `CreateGroupAddress`/
`DeleteGroupAddress`'s existing shape exactly (caller pre-allocates the id
via `Project::ids`, `apply` returns its own inverse). Two validation
functions that existed but were never called from any command
(`check_group_address_in_range`, `check_group_link_target_exists`) are
finally wired up — the former into `CreateGroupAddress` itself, as a
previously-missing check on existing behavior; the latter into
`LinkComObject` only (`UnlinkComObject` removes a link and so has no
target to check exists). `CreateGroupRange` gets its own pair of checks
instead, from two of the four new `validation.rs` functions that close
the remaining gaps: duplicate area/line address (line addresses unique
per area, not project-wide, matching ETS's Area.Line.Device numbering)
and group-range nesting/overlap (`check_group_range_nests_in_parent`,
`check_no_overlapping_group_range`). All ten commands are backend-only
this cycle: `apps/knx-server` gains one route each (`/api/areas`,
`/api/lines`, `/api/move-device`, `/api/group-ranges`,
`/api/group-links`), but no frontend UI exists for any of them yet — see
[the design spec](superpowers/specs/2026-09-06-topology-group-range-commands-design.md)
for the deliberate scope cut (`knx-projection`'s `GroupAddressNode` has no
real main/middle/address nesting yet; that redesign is its own future
cycle). `knx-projection` gains a small, additive `GroupRangeNode`
(id/name/start/end/parent) on `InstallationNode` — not the fuller nesting
redesign, just enough for an HTTP caller to discover a newly-created
range's id. `sync_after_command` gains no new incremental-sync paths for
any of the ten — same as every command since Session 5 cycle 2 that
hasn't gotten one yet, correct via a full `save_project`/`load_project`
round trip until a later cycle's incremental-sync pass covers all of
them together. 49 Rust tests added across
`crates/knx-core`/`crates/knx-projection`/`apps/knx-server`.

## Next session

Session 7 is in progress (cycle 1, 2026-09-06). Its entry condition — all
earlier sessions' deliverables exist and are tested — is met well enough to
start: Session 6 shipped cycles 1-5 (tunnelling, sending, discovery,
routing, diagnostics), with KNX IP Secure shelved indefinitely rather than
attempted (see below), not blocking. Cycle 1 audited "full roundtrip and
migration suite with frozen fixtures per schema version" against what
actually exists: schema 11 (ETS4 reference project) already had full
coverage, unchanged. A real, independent second sample — `KV v2.5 -
demo.knxproj`, schema 21, found in `OriginalData/` — got a real-file
detection-and-refusal regression test (schema 23 already had one). Comparing
schema 21 against schema 11 (RESEARCH §3.4) found real structural deltas
previously attributed to schema 23 alone already exist at 21, plus one new
one (`ModuleInstances`) — flagged as the next major format-support task, not
attempted this cycle (see [KNOWN_LIMITATIONS.md §1](KNOWN_LIMITATIONS.md)).
`knx-store`'s `.knxdb` migration chain (v1→v4) was verified, not touched —
already complete. Remaining Session 7 deliverables: performance measurement
on large projects, Linux packaging, the licence decision.

Session 6's KNX IP Secure was scoped after cycle 4 (routing), then shelved
indefinitely (2026-09-06, not just deferred to "a later cycle") — plain
tunnelling/routing covers the common case; revisit only when a real
secure-only gateway needs it, research spike first. See
[ROADMAP.md](ROADMAP.md), [KNOWN_LIMITATIONS.md §26](KNOWN_LIMITATIONS.md).

Session 6 is in progress (Cycles 1-5 done). ROADMAP's entry condition — "a
project can be opened and its group addresses resolved" — is met: the domain
model, import/export pipeline, shared product database, and now tunnelling
(receive and send) to a known gateway — plus discovery of gateways on the
LAN — all exist, and telegrams from the live bus resolve against group
addresses from the open project.

Cycle 5 shipped Search: finding a device, group address, or building part
by name/address across a project too large to scan by eye in the Project
Explorer alone. Cycle 6 shipped the command palette: `Ctrl+Shift+P` over
the app's seven existing actions, reusing Search's overlay structure.
Cycle 7 shipped a System/Light/Dark theme toggle, persisted to
`localStorage`. CLAUDE.md's UI/UX deliverable list for Session 5 —
Project Explorer, properties inspector, search, command palette, dark
and light mode — was complete as of cycle 7; see [ROADMAP.md](ROADMAP.md).
Cycle 8 added a project status dashboard, cycle 9 group address
create/delete, and cycle 10 (`ideas.md`, three small unscheduled items
bundled together since two needed the same new infrastructure) a toast
notification stack: `kind: "error"` toasts replace the old persistent
banner one-for-one, wrapped in a randomly chosen humor template that
keeps the original backend message verbatim inside it; `kind: "fun"`
toasts auto-dismiss after ~6s and appear at most once at startup, for a
listed holiday or (failing that) a late-night session — never both, see
the [design spec](superpowers/specs/2026-09-05-toast-easter-eggs-design.md).

Known gaps carried forward, none blocking Session 5:

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
- Cycle 9's plan-mandated manual smoke check (open a project, expand
  "Group Addresses", create one via the inline row, select it, delete it,
  confirm Undo/Redo restores it via both the toolbar and `Ctrl+Z`/
  `Ctrl+Shift+Z`) was not performed — no display available in this
  environment for a Tauri GUI session. Rust-level tests
  (`command.rs`, `command_dispatch.rs`) cover the validation and
  persistence logic underneath it.
- Cycle 4's properties inspector edits exactly two fields (individual
  address, communication-object DPT); `ComObjectNode`'s read/write/
  transmit/update/communication flags are projected but display-only —
  no `Command` exists yet to edit a flag. No group-address, building, or
  parameter editing exists in the UI yet either.
- Cycle 4's plan-mandated manual smoke check (launch the app, edit a
  device's address, confirm Undo/Redo via both the toolbar and
  `Ctrl+Z`/`Ctrl+Shift+Z`) was not performed — no display available in
  this environment for a Tauri GUI session. Code-level review (the
  change-guard in `AddressField`/`DptField`, the stale-selection-race
  trace in `App.tsx`) covered the correctness that mattered; left for the
  user, or a future session with a display, to confirm interactively.
- Cycle 5's plan-mandated manual smoke check for the `Ctrl+K` search
  overlay (open it, type a query, navigate with the arrow keys, pick a
  result) was likewise not performed in this environment — no isolated
  display available for a Tauri GUI session. Should be run before, or at,
  merge. `vitest` coverage (`treeUtils.test.ts`, `searchMatch.test.ts`)
  covers the ranking/indexing logic underneath it.
- Cycle 6's plan-mandated manual smoke check for the command palette
  (open it with `Ctrl+Shift+P`, type a filter, arrow-navigate skipping a
  disabled row, invoke an entry with Enter, verify `Ctrl+K`/
  `Ctrl+Shift+P` mutual exclusivity) was likewise not performed — no
  display available in this environment. Should be run before, or at,
  merge. `vitest` coverage (`commandRegistry.test.ts`) covers the
  filtering/enablement logic underneath it.
- Cycle 7's plan-mandated manual smoke check (click through all three
  theme states, confirm the icon matches each state, confirm the error
  banner and search/palette overlay stay legible in both explicit Light
  and Dark, restart the app after picking an explicit theme and confirm
  it survives) was likewise not performed — no display available in this
  environment. Should be run before, or at, merge; the placeholder
  dark-mode color values (`#ff6b6b` error text, `rgba(0,0,0,0.6)`/
  `rgba(0,0,0,0.5)` overlay backdrop/shadow) are an untested starting
  point, not a measured contrast-ratio result. `vitest` coverage
  (`theme.test.ts`) covers the cycling/persistence logic underneath it.
- Cycle 10's plan-mandated manual smoke check (trigger a real error and
  confirm the wrapped message still contains the real text and stays
  until dismissed or replaced; temporarily fake the system clock/date to
  confirm a holiday and a late-night toast each appear once at startup
  and auto-dismiss) was likewise not performed — no display available in
  this environment. Should be run before, or at, merge. `vitest`
  coverage (`toast.test.ts`) covers the precedence/boundary logic
  underneath it; `useToasts`/`ToastStack.tsx`/the `App.tsx` wiring are
  untested for the same reason `useTheme`/`ThemeToggle.tsx` are (no
  component-testing library in this project).
- A search result that lands inside a manually collapsed Project Explorer
  branch is not auto-revealed — the tree does not expand or scroll to it,
  only the Inspector reflects the new selection. This is an explicit,
  approved scope decision recorded in
  [the search design spec](superpowers/specs/2026-09-04-search-design.md),
  not an oversight; see [KNOWN_LIMITATIONS.md §19](KNOWN_LIMITATIONS.md).
- `manifest.rs`/`opaque.rs` still open their own internal SQL transaction
  the same way `strings.rs` did before this cycle's `SAVEPOINT` fix
  (above); not currently reachable from inside `save_project`'s
  transaction, so no live bug, but the same hazard would resurface if a
  future task ever wires them into it. Two ad-hoc `SELECT` queries in
  `command_sync.rs` (a device's current placement; the next
  `group_address` position) should move behind named helpers in
  `devices.rs`/`group.rs` if that module grows — harmless today.
- The `when/@test` value grammar that would make device parameters
  interpretable is now documented (RESEARCH R3/§4.3, spike run
  2026-09-11); the `Dynamic` tree's structural grammar remains
  corpus-observed only. T18's first slice (also 2026-09-11) built a
  headless evaluator over the stored tree in `knx-productdb`, and slice 2
  (same day) taught it to expand a `Module` node into its `ModuleDef`'s
  own tree — but device parameters are still not interpretable *by a
  user*: nothing wires the evaluator into any UI, and no parameter editor
  exists. See the dated entries below and T18
  ([GAP_ANALYSIS_ETS.md](GAP_ANALYSIS_ETS.md) Tier 5).
- A program value behind an instance-level `Empty` slot stays invisible in
  the model ([KNOWN_LIMITATIONS.md](KNOWN_LIMITATIONS.md) §12); lifted by
  a layer stack in `Override<T>`, a domain-model change deliberately not
  taken this session ([ADR-0012](adr/0012-enrichment-into-absent-slots.md)).
- An ambiguous, space-separated `DatapointType` list fills nothing
  (RESEARCH §4.2, KNOWN_LIMITATIONS §12) — resolving it needs more context
  (e.g. a linked group address's own DPT) than one communication object
  alone carries.
- Schema 21 import+export shipped and round-trip verified against one
  sample (the KNX Association `KV v2.5` demo project); schema 23 import
  shipped, module-based application-program handling inferred from schema
  21's measured shape, not independently evidenced — no round-trip claim
  (RESEARCH §3.4, [KNOWN_LIMITATIONS.md](KNOWN_LIMITATIONS.md) §1). Schema
  23 manufacturer data ingestion is unaffected by this and remains its own
  gap (KNOWN_LIMITATIONS §12).
- `.knxprod` direct ingest for master data scheme ≥ 12 was unsupported as
  of this cycle (2026-09-06/07); 2026-09-10's standalone package
  installer (T24 below) lifted this for schemes 11 and 20 specifically —
  see KNOWN_LIMITATIONS §11 for current status. Schemes 12-19/21/22 still
  have no route in; manufacturer data at those schemes still reaches the
  product database only via a `.knxproj` that already contains it.
- Whether ETS re-imports an unsigned third-party `.knxproj` remains
  untested (risk R9) — see [COMPATIBILITY.md](COMPATIBILITY.md).

**T23, first slice (2026-09-07) — Group-Ranges UI.** `apps/knx-web`
gains a "Group Ranges" tree branch (sibling to cycle 9's "Group
Addresses" branch) and a range picker on group-address creation, closing
**B5** ([GAP_ANALYSIS_ETS.md](GAP_ANALYSIS_ETS.md)) and the picker half
of [KNOWN_LIMITATIONS.md §21](KNOWN_LIMITATIONS.md#21-a-ui-created-group-address-without-a-range-is-still-dropped-on-export--partially-resolved).
Brainstormed and scoped as the first of three deliberately separate
bounded slices of T23 (Group Ranges → link/unlink → topology tree-edit),
not one combined cycle. `treeUtils.ts` gains `nestGroupRanges` (rebuilds
the main/middle hierarchy from `GroupRangeNode`'s flat `parent` pointer —
`knx-projection` deliberately keeps that type flat, per its own doc
comment) and `findGroupRange`; `ProjectExplorer.tsx` gains
`GroupRangeItem`/`NewGroupRangeRow` (create only offered on a main range,
matching the two-level depth the reference project actually shows, though
the model itself doesn't cap nesting); `Inspector.tsx` gains
`GroupRangeInspector`/`GroupRangeNameField` (rename inline, same
blur-to-apply shape as `AddressField`, plus Delete), gated by the same
`installations[0]`-only check every other create/delete/rename affordance
already carries (`Command::apply`'s hard-coded target). No backend change
was needed — `create_group_address`'s `range_id` and the
`create_group_range`/`delete_group_range`/`rename_group_range` routes
already existed (2026-09-06 topology/group-range command layer) with no
frontend caller until now. A real, pre-existing bug was found and fixed
in passing: `dashboardStats.test.ts` never got its four installation
fixtures updated with `group_ranges: []` when that field was added to
`InstallationNode` in the same 2026-09-06 cycle, which meant `apps/knx-web`'s
own `npm run build` (`tsc && vite build`) — part of `ci.yml` — has been
failing type-check on `main` since that merge; not caused by this slice,
fixed alongside it since it blocked verifying this slice's own `tsc`
run. 10 new `vitest` tests (`treeUtils.test.ts` — `findGroupRange`,
`nestGroupRanges`; `api.test.ts` — the three new endpoints plus the
`rangeId`-omitted/-included cases), for 76 total.

**T23, second slice (2026-09-07) — Link/Unlink UI.** `apps/knx-web`'s
comm-object rows in the properties Inspector gain a link list with an
Unlink button per existing `GroupLink` and an inline add-link row
(group-address picker + Send/Receive select), closing **B7**
([GAP_ANALYSIS_ETS.md](GAP_ANALYSIS_ETS.md)). Unlike the first slice, a
real backend gap surfaced during scoping, not just a missing frontend
caller: `knx-projection`'s `ComObjectNode` had no field for a comm
object's *existing* `GroupLink`s at all — `Command::LinkComObject`/
`UnlinkComObject` (2026-09-06) could change the model but nothing could
display what was already linked. `ComObjectNode` gains
`links: Vec<GroupLinkNode>` (`ga_id`, resolved `address`/`name` — `None`
on a dangling link rather than panicking, since a `GroupLink` names its
target by id alone with no installation of its own to search, and this
stays defensive even though `DeleteGroupAddress` already refuses to
create that state — and `direction` as `Direction`'s `Debug` form,
`"Send"`/`"Receive"`, matching `dpt_layer`'s convention and, not
coincidentally, exactly the string `apps/knx-server`'s
`parse_direction` expects back from `unlinkComObject`). `Inspector.tsx`
gains `GroupLinkRow` (Unlink — no `installations[0]` restriction,
since `Command::UnlinkComObject` doesn't have one) and `NewGroupLinkRow`
(Link — gated to `installations[0]`'s group addresses, matching
`Command::LinkComObject`'s own check). 3 new `cargo test -p
knx-projection` tests (a resolved link, a dangling one, and the existing
device-detail test gains a `links.is_empty()` assertion) and 2 new
`vitest` tests (`api.test.ts` — `linkComObject`/`unlinkComObject`), for
78 `vitest` total; backend route coverage for link/unlink already
existed (`http_edit_routes.rs`'s
`linking_then_unlinking_a_com_object_to_a_group_address`, 2026-09-06)
and needed no change.

**Pre-existing bug found, not fixed this slice (out of scope — see
below).** `cargo clippy --workspace --all-targets -- -D warnings` — a
`ci.yml` gate — currently fails on `main`:
`crates/knx-etsproj/src/parse/installation.rs` and
`installation_v21.rs`'s `Frame` enums trip `clippy::large_enum_variant`
(744 vs. 360 bytes between `Device`/`ComObject` variants), evidently
introduced by the schema-21/23 import work
(`57c233b`/2026-09-07) and not caught then since that merge predates
this session running clippy. Confirmed pre-existing (reproduces
identically on `main` before this slice's changes) and unrelated to
`knx-web`/`knx-projection`; fixing it means boxing large `Frame`
variants across two parser files, a separate bounded task, not folded
into this one. `knx-projection` itself is clippy-clean
(`cargo clippy -p knx-projection --all-targets -- -D warnings`, run
standalone since the workspace-wide invocation cannot get past
`knx-etsproj`'s failure to even reach `knx-projection`'s or
`knx-server`'s own lints).

**T23, third slice (2026-09-07) — area/line tree-edit UI. T23
complete.** `apps/knx-web`'s `AreaItem`/`LineItem` in the Project
Explorer become selectable — previously expand-only, the only topology
rows without one, unlike every other tree kind since cycle 5's search —
with `NewAreaRow`/`NewLineRow` create rows (`isFirst`-gated, same
convention as every other create affordance; `medium_ref` pre-filled
`"MT-0"`, ETS's own default for twisted-pair, since `Line.medium_ref` is
an opaque product reference `knx-core` deliberately doesn't interpret,
per that field's own doc comment, so no dropdown is possible). Closes
**B3** ([GAP_ANALYSIS_ETS.md](GAP_ANALYSIS_ETS.md)) and, with it, T23 as
a whole. `Inspector.tsx` gains `AreaInspector`/`LineInspector`
(summary — line/device count — plus Delete, `installations[0]`-gated
like every other Delete in this file; no rename field, since no
`RenameArea`/`RenameLine` command exists, unlike `GroupRangeInspector`)
and `LineMoveField` on the device Inspector, driving `Command::
MoveDeviceToLine` from a `<select>` of every area's lines plus
"(unassigned)" — independent of `AddressField`'s individual address, per
that command's own doc comment ("a line move and a re-address are two
separate user intents"). `LineMoveField` renders nothing (not a disabled
control) when `treeUtils.ts`'s new `findDeviceLineInFirstInstallation`
returns `undefined` — a device the command can't target at all because
it isn't reachable from `installations[0]`'s topology (a building-only
placement, or a later installation) — since there's no sensible current
value to show even disabled, unlike the `canDelete`-gated buttons
elsewhere that do have one. `treeUtils.ts` also gains `findArea`/
`findLine`, matching every other selectable kind's own find helper
(`findGroupAddress`, `findGroupRange`, `findBuildingPart`). No backend
change needed — `CreateArea`/`DeleteArea`/`CreateLine`/`DeleteLine`/
`MoveDeviceToLine` and their routes already existed (2026-09-06 topology
command layer) with no frontend caller until now. 14 new `vitest` tests
(`treeUtils.test.ts` — `findArea`, `findLine`,
`findDeviceLineInFirstInstallation`; `api.test.ts` — the five new
endpoints), for 92 total.

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
frontend in T23 a day later. 16 new tests across
`crates/knx-core`/`crates/knx-productdb`/`apps/knx-server`. Closes
**B1**/**B2** ([GAP_ANALYSIS_ETS.md](GAP_ANALYSIS_ETS.md)), backend
only; **T2** stays open.

**T2, device catalog browser UI (2026-09-08).** Frontend-only, bounded
task (no design spec — same shape as cycle 9's group-address UI, per the
brainstorming skill's classification), closing the gap left open above.
`apps/knx-web` gains `CatalogBrowser.tsx`, a modal reusing `.search-overlay`/
`.search-panel` (now shared by three consumers — `Search.tsx`,
`CommandPalette.tsx`, and this) with a manufacturer `<select>` and a
debounced (~200ms) search box querying the already-existing
`GET /api/catalog/manufacturers`/`GET /api/catalog/items` routes; picking
an item and naming the device calls `POST /api/devices`. Opened from a
new `+ Add device` row rendered on every `LineItem` and once under the
Unassigned bucket, both `isFirst`-gated the same way every other create
affordance in `ProjectExplorer.tsx` already is (`Command::CreateDevice`
only ever targets `installations[0]`) — `LineItem` gains the `isFirst`
prop for this, threaded down from `AreaItem`, which already had it.
`api.ts` gains `catalogManufacturers`/`catalogItems`/`createDevice` plus
two hand-written DTO types (`CatalogManufacturer`/`CatalogItem`) — server
route-local structs, not `knx-projection` types, so no `ts-rs` binding
exists for them, same treatment every other request body already gets.
No backend change was needed; no arrow-key navigation in the modal
(click-only), a deliberate scope cut unlike `Search.tsx`. Device-delete
UI (T3's own frontend, backend done 2026-09-08 above) is its own
following cycle, below. 5 new `vitest` tests in `api.test.ts`, for 97 total.

**Regression fix, cycle 13's `mask-image` ate every fixed overlay
(2026-09-08).** User report: every "open" toolbar button was clickable but
produced no visible dialog. Root cause was cycle 13's fading grid-pattern
background — `mask-image` set directly on `body` in `styles.css`. Per spec
`mask`/`filter` clip an element's *entire* painted subtree, same as
`opacity`; `body` has no explicit height, so its own box is only as tall as
the toolbar, and the mask's gradient geometry is sized to that box. Every
`position: fixed` overlay appended straight to `document.body`
(`FsPicker.tsx`'s file-open/save modal, `Search.tsx`, `CommandPalette.tsx`,
the toast stack) fell outside that short box and rendered fully invisible —
confirmed with a Playwright repro (`document.body.style.maskImage = "none"`
made the "Open" dialog appear instantly). Fix: the grid-pattern and its
mask now live on a `body::before` pseudo-element (`position: fixed; inset:
0; z-index: -1; pointer-events: none`) instead of on `body` itself, so
decoration no longer clips content it merely sits behind. No test caught
this — `styles.css` has no coverage and every prior vitest run mocks
`fetch`/DOM without ever painting a real layout — a gap worth keeping in
mind for future full-page-visual changes. 89/89 existing `vitest` tests
still green (unaffected, CSS-only fix).

**T3, device-delete UI (2026-09-08).** Closes the frontend half T2 left
open. `api.ts` gains `deleteDevice`; `Inspector.tsx`'s `DeviceInspector`
gains a `canDelete`-gated Delete button, same shape as
`AreaInspector`/`LineInspector`'s own — `canDelete` reuses
`findDeviceLineInFirstInstallation`'s existing three-way result
(a line id, `null` for unassigned-but-reachable, or `undefined` for
unreachable) rather than a new helper, since `!== undefined` is exactly
`Command::DeleteDevice`'s own reachability test
(`remove_device_from_topology` only searches `installations[0]`'s lines
and `unassigned`, command.rs). A `CommandError::DeviceHasLinks` refusal
surfaces through the same `api.errorMessage`-into-`field-error` path
every other Delete button already uses; no new error-handling shape was
needed. `onDeleted` (already threaded into `Inspector` for every other
selectable kind) now also reaches the device branch, so a successful
delete clears the selection via `App.tsx`'s existing `resetTree`. No
backend change — `DELETE /api/devices/{id}` shipped with T1/T3's backend
cycle above. 1 new `vitest` test in `api.test.ts`, for 90 total. Closes
**T3**'s frontend half ([GAP_ANALYSIS_ETS.md](GAP_ANALYSIS_ETS.md)).

**T7, communication-object flag editing (2026-09-08).** `crates/knx-core`
gains `Command::SetComObjectFlag`/`RestoreComObjectFlag`, generic over
which of the five flags it targets via a new `ComFlagKind` enum and
`ResolvedFlags::get`/`get_mut` accessors, rather than five near-identical
commands — same "resolves to `Layer::UserEdit`, undo restores the exact
prior `Override<bool>`" shape `SetComObjectDpt`/`SetComObjectDescription`
already use, except `value` is a bare `bool` (a checkbox has only two
states — no "clear the override" gesture exists here, unlike those two
text-field commands' empty-string convention). `apps/knx-server` gains
`POST /api/com-object-flag`, parsing the wire-format flag name the same
way `parse_direction` already parses group-link directions. `apps/knx-web`
gains `ComObjectFlagsRow`, five checkboxes rendered on every comm-object's
Inspector row — `ComObjectNode` already carried all five flags as plain
`bool`s from an earlier cycle, but nothing in the UI ever rendered them
before this cycle, contrary to what this task's own backlog text assumed.
No `knx-projection` change was needed. 3 new `cargo test` tests
(`knx-core` x2, `knx-server` x2 — one positive, one rejecting an unknown
flag name) and 1 new `vitest` test. Closes **T7**
([GAP_ANALYSIS_ETS.md](GAP_ANALYSIS_ETS.md)), **B6**.

**T24, standalone `.knxprod` product-package install + honest creation
diagnostics (2026-09-10, plan
[2026-09-09-standalone-product-database-install.md](superpowers/plans/2026-09-09-standalone-product-database-install.md),
Tasks 1-5).** Four code tasks plus this documentation reconciliation.

*Task 1 — first-winner provenance.* Fixed a pre-existing gap where a
catalog-item id conflict on re-ingest could silently overwrite the first
winner's hardware row rather than keeping it; now the first successful
insert of a given id always wins, later conflicting inserts are recorded
as `IdConflict`s, never applied.

*Task 2 — atomic standalone package installer.* New
`knx_productdb::install_package(conn, name, bytes)`: validates the whole
ZIP archive before publishing any row (full pre-scan, not
validate-as-you-go), stores every member's raw bytes by SHA-256, records
an ordered member inventory, and rejects — as a typed `PackageError`,
`Display`ed as an exact user-facing string — an invalid ZIP, an encrypted
member, path traversal, a duplicate member name, an oversized member, a
missing `knx_master.xml`, an unsupported namespace, a full `.knxproj`
project archive passed where a product package was expected, or a
`.vd2` filename (checked by suffix only — never opened as a ZIP,
never decrypted or parsed — but, since 2026-09-11, the whole-archive
bytes are hashed and the error carries the evidence:
`PackageError::LegacyVd2 { sha256, len }` →
`"legacy .vd2 product data is unsupported (sha256 <64 hex chars>,
<len> bytes)"`; the archive size limit is enforced before that hash is
computed, so hashing itself stays bounded). A second install of
byte-identical content is a no-op
(`InstallReport.skipped == true`, zero new rows). Verified against the
full 6-file real-world corpus in `OriginalData/ProductDatabases/`: 3
files at master data scheme 11, 2 at scheme 20 install cleanly
(`installs_the_readable_corpus`,
`crates/knx-productdb/tests/standalone_packages.rs`); the 6th,
`Weinzierl_730_KNX_IP_Interface_ETS2-3.vd2`, is confirmed by direct
`unzip` inspection to be a pre-2013 ETS2-era SFX/`.vd_`-style archive —
no `knx_master.xml`, not the same ZIP/XML container family as
`.knxprod`/`.knxproj` at all — and is rejected by the filename-suffix
check. `Project Schema23 v01.00.00` §4.2.2-§4.2.3 (MasterData/`M-iiii`
layout, `knx_master.xml` root) and `03_01_01 Architecture v03.00.02 AS`
§6.2 (manufacturer product template as tool-side configuration input)
both checked directly against the primary spec text, not cited on
faith.

*Task 3 — truthful CLI/HTTP results.* `knx products ingest
<file.knxproj|file.knxprod|file.vd2>` and `POST /api/catalog/install`
both return the same typed `PackageError` strings rather than a generic
failure; `malformed_and_legacy_product_uploads_are_typed_bad_requests`
(`knx-server/tests/http_product_install.rs`) pins the exact three
messages (`"invalid product ZIP"`, `"legacy .vd2 product data is
unsupported"`, `"encrypted product ZIP member"`).

*Task 4 — honest catalog creation and diagnostics.*
`knx_productdb::query::resolve_catalog_item_program` validates the full
catalog item → product → hardware → hardware2program → program chain
before `create_device_impl` builds a `Command::CreateDevice` — only a
hardware row that explicitly declares itself programless may skip
program seeding, every other dangling relation is a typed 400 with no
command ever applied. `POST /api/devices` returns
`CreateDeviceResponse { tree, diagnostics }` covering
`ProgramlessProduct`/`AmbiguousDpt`/`ComObjectRefMissing`/
`ProgramRefMissing`/`DynamicOrModuleNotEvaluated`, each carrying a
server-computed `.detail()` string. `CatalogBrowser.tsx` gained an
install file-picker (`installProductPackage`), install-report/error
display, a post-install catalog refresh, and in-modal diagnostics
rendering with a "Done" button instead of auto-close when diagnostics
exist. Resolves
[KNOWN_LIMITATIONS.md §35](KNOWN_LIMITATIONS.md#35-device-creation-enrichmentissues-are-silently-dropped--resolved-2026-09-10).

*Task 5 — this reconciliation.* Updated
[GAP_ANALYSIS_ETS.md](GAP_ANALYSIS_ETS.md) (A5 partially closed; B1/B2/
B3/B5/B6/B7 marked closed against their already-"Done" task-backlog
entries, which the table rows had not reflected; D3 closed and
extended; new **T24** entry), [KNOWN_LIMITATIONS.md](KNOWN_LIMITATIONS.md)
(§11 rewritten for the 11/20 split and the named `.vd2` blocker; §35
marked resolved), [ROADMAP.md](ROADMAP.md) (the stale "`.knxprod` ingest
... remains out of v1 scope" open question corrected), and
[COMPATIBILITY.md](COMPATIBILITY.md) (new §2 rows for package install and
rejection; §3's schema-20 row clarified as `.knxproj`-project-specific,
distinct from the now-verified `.knxprod` package claim; §4's blanket
"not supported" row narrowed to schemes 12-19/21/22 plus `.vd2`).
`docs/superpowers/specs/2026-09-09-standalone-product-database-install-design.md`'s
acceptance criterion "the caller receives the archive hash/size in the
error report where available" was **not implemented** for the `.vd2`
case at the time (the filename check ran before any hash/size was
computed) — recorded then as a known gap, out of scope for Task 5's
documentation-only pass. **Closed 2026-09-11:** the `MAX_PACKAGE_SIZE`
guard now runs first (so hashing itself stays bounded), then the
`.vd2` filename check hashes the whole archive and returns
`PackageError::LegacyVd2 { sha256, len }`, whose `Display` reports
both; see `rejects_the_real_legacy_vd2_corpus_file_with_its_hash_and_size`
and `a_small_vd2_still_reports_hash_and_length`
(`crates/knx-productdb/tests/standalone_packages.rs`). `.vd2` itself
remains unsupported — no byte of it is decrypted, parsed, or
installed — only the evidence attached to its rejection changed.

Gates run for Task 5 (`KNXBENCH_PRODUCT_CORPUS` pointed at
`OriginalData/ProductDatabases`): `cargo fmt --all --check` (found and
fixed two leftover unformatted spots from Tasks 3-4's fix-loop commits,
whitespace only), `cargo clippy --workspace --all-targets -- -D
warnings` (clean except the pre-existing, out-of-scope
`clippy::large_enum_variant` on `knx-etsproj`'s `Frame` enum,
`crates/knx-etsproj/src/parse/installation.rs:36` and
`installation_v21.rs:48` — deliberately not touched; CLAUDE.md requires
performance/size optimizations to be measurement-driven, not applied to
satisfy a lint), `cargo test --workspace` (all green, corpus tests
included), `cargo run -p xtask -- check-layering` (clean), `npm test`
(96/96), `npm run build` (clean). A second pre-existing clippy issue
also found and fixed in passing (test-only, zero semantic risk): three
`clippy::bool_assert_comparison` lints in
`crates/knx-core/src/command.rs` (`assert_eq!(x, false/true)` →
`assert!(!x)`/`assert!(x)`), not the `Frame` enum, not product-code
behavior.

**T8, building-part CRUD (2026-09-08).** `crates/knx-core` gains
`Command::CreateBuildingPart`/`DeleteBuildingPart`/`RenameBuildingPart`/
`MoveDeviceToBuildingPart`, the same flat-list tree-CRUD shape
`CreateGroupRange`/`DeleteGroupRange`/`RenameGroupRange` already use —
`installation.buildings` was already flat, linked by `parent`/`children`
ids, so this only adds the commands, not the nesting. A new
`remove_device_from_buildings` helper deliberately does *not* mirror
`remove_device_from_topology`'s error-on-absent behavior: building
placement isn't exhaustive (a device can have no building part at all),
so "not found anywhere" is a normal `None`, not a `CommandError`.
`apps/knx-server` gains `POST /api/building-parts`,
`DELETE`/`PATCH /api/building-parts/{id}`, and
`POST /api/move-device-to-building-part`, parsing the wire-format kind
string the same way `parse_direction` already parses its own enum.
`apps/knx-web` gains a `NewBuildingPartRow` in
`ProjectExplorer` (every `BuildingItem` gets one, since building parts
nest to unbounded depth, unlike group ranges' two-level cap), and in
`Inspector`, a `BuildingPartNameField`/Delete button on the building-part
panel plus a `BuildingPartMoveField` on `DeviceInspector` — both mirror
`GroupRangeInspector`/`LineMoveField`'s own shape. `treeUtils.ts` gains
`findDeviceBuildingPartInFirstInstallation` and exports the
previously-private `flattenBuildingParts`. New `cargo test` tests in
`knx-core` (create/delete/rename/move round trips, plus the not-found/
not-empty rejection paths) and `knx-server` (4 HTTP integration tests),
and 8 new `vitest` tests (`api.test.ts` x5, `treeUtils.test.ts` x3).
Closes **T8** ([GAP_ANALYSIS_ETS.md](GAP_ANALYSIS_ETS.md)), **B4**.

**T10, wire up `export_ets_project` to a real interface (2026-09-10).**
`export_ets_project` (in `knx-app`) previously had no user-facing caller
anywhere — import worked end to end, export did not. Three callers added:
`apps/knx-cli` gains a `knx export <store.knxdb> <out.knxproj>
[--product-db <path>] [--no-product-db]` subcommand, mirroring `knx
import`'s argument shape; every `ExportWarning` is printed to stderr,
never silently dropped, and the exit code stays 0 (warnings are not
errors). `apps/knx-server` gains `domain::export_project` and `POST
/api/project/export`, requiring `store_path` already set (export reads
the opaque/manifest data from the saved `.knxdb`, the project content
itself from the live in-memory `Project`) — a 400, not 500, when it
isn't, since that's a client-fixable precondition, not a server fault.
`apps/knx-web` gains an "Export to .knxproj…" button next to "Save
As…", gated on the same `hasStorePath` state, surfacing every warning as
one joined toast message via a new `describeExportWarning()` helper
(needed because `ExportWarningDto` is externally tagged —
`{"unsigned": {"detail": "..."}}` — not the flat shape a first attempt
assumed).

Closing this gap surfaced a real, previously-undetected data-integrity
bug, caught by the plan's own final whole-branch review rather than any
per-task test: server-side ETS import (`domain::import_and_project`) ran
against a throwaway in-memory `knx-store` connection, discarded once
`Project` was extracted into `AppState`; `save_project_as_impl` only ever
wrote the domain-model tables. An import → Save As → Export round trip on
the server therefore silently dropped every opaque passthrough entry and
manufacturer-manifest row with no warning (measured: a 1.7 MB source
`.knxproj` round-tripped to a 29.9 KB output). This falsified the plan's
own foundational assumption that a `.knxdb` save already persists that
data — true only by accident on the CLI path, where import and save
happen to share one connection. Fixed in the same branch:
`AppState` now carries `opaque`/`manufacturer_refs` fields, filled by
`open_project` (read back from the import's connection before it's
dropped) and `open_native_project` (read back from the `.knxdb` just
loaded); `save_project`/`save_project_as`/`/api/project/download` all
write them into the target `.knxdb` alongside the domain tables.
A new regression test,
`exported_project_still_carries_opaque_and_manufacturer_data_after_save_as`,
reimports the exported file and asserts non-empty opaque/manifest tables
— a content check, not just "the file is non-empty", which is the class
of assertion that let the bug through Tasks 1-3's own tests undetected.

A **second** bug surfaced by a round-2 whole-branch re-review, dispatched
after the first fix: `knx_store::insert_opaque`/`insert_manufacturer_refs`
were plain `INSERT`s with no clear-first step, unlike `save_project`'s own
DELETE-then-insert convention. The first fix above made
`save_project_as_impl` call them on *every* save, not just the first —
so a plain repeated `POST /api/project/save`, reusing an already-populated
`store_path`, duplicated every opaque/manifest row without bound. The
reviewer reproduced this empirically in a standalone crate outside the
worktree. Fixed at the `knx-store` level (`DELETE FROM` before `INSERT`,
inside the existing transaction), so the fix covers every caller, not
just `save_project_as_impl`. Regression tests added in both
`knx-store` (repeated `insert_opaque`/`insert_manufacturer_refs` calls)
and `knx-server` (`saving_the_same_project_twice_does_not_duplicate_opaque_and_manifest_rows`,
exercising the actual HTTP `/api/project/save` path).

While fixing the second bug, `export_project` was also changed: it used
to re-open `store_path` off disk to read the opaque/manifest tables
(the original doc comment's "no change needed" above was true at the
time but became a second, sharper consumer of the stale-`store_path`
gap in [KNOWN_LIMITATIONS.md #18](KNOWN_LIMITATIONS.md#18-open_project-does-not-clear-the-previous-knxdb-store_path)
once the data those tables carry could go stale relative to
`state.project`). It now reads `AppState.opaque`/`AppState.manufacturer_refs`
directly — the same live, in-memory copies every save path already
writes through — copied into a throwaway in-memory `.knxdb` for
`export_ets_project`'s `Connection`-shaped interface, instead of
re-opening the file: `opaque`/`manufacturer_refs` are now locked, copied,
and dropped before `project`/`product_db` are touched at all, rather than
holding all four locks at once for the duration of the call. (Round-3
review correction: this did *not* touch the pre-existing
`project`-then-`product_db` lock ordering itself, which round 2 flagged
as a Minor, unaddressed inconsistency with the rest of the codebase —
still safe, no deadlock counterpart exists for it, just left as-is.)

New tests: `apps/knx-cli/tests/cli_export.rs` (2), 5 in
`apps/knx-server/tests/http_export_route.rs` (including both regression
tests above), 2 in `knx-store` (`opaque.rs`, `manifest.rs`). Gates:
`cargo fmt --all --check` clean, `cargo test --workspace` all green
(one pre-existing, environment-dependent failure in `knx-productdb`'s
`installs_the_readable_corpus`, unrelated — needs `KNXBENCH_PRODUCT_CORPUS`
pointed at a fixture corpus not present in this environment), `cargo
clippy --workspace --all-targets` clean except two known pre-existing
issues (`knx-etsproj`'s `large_enum_variant`, `knx-server`'s
`http_product_install.rs` `field_reassign_with_default`), `xtask
check-layering` clean, `npx tsc --noEmit` / `npm test` (104/104) /
`npm run build` clean on `knx-web`. Closes **C4**
([GAP_ANALYSIS_ETS.md](GAP_ANALYSIS_ETS.md)).

**T9, bulk/multi-select operations (2026-09-10).** `crates/knx-core`
gains `Command::Batch(Vec<Command>)`, composing existing single-entity
commands with all-or-nothing apply/rollback: on any sub-command `Err`,
already-applied sub-commands are rolled back (their inverses re-applied
in reverse order) before the original error propagates unchanged; on
success, the returned inverse is the reversed list of collected inverses,
itself a `Batch`. `CommandStack` needed no change at all — it was
already generic over `Command`, just calling `.apply()` and pushing
whatever came back. `knx-store::command_sync.rs` gained one more
no-op stub arm for exhaustiveness; this is genuinely inert, since
`knx-server`'s command path never calls `sync_after_command` at all —
the server persists only via explicit whole-project `save_project`/
`save_project_as`, an existing gap this cycle didn't need to touch.
`apps/knx-server` gains `batch_delete_devices_impl`,
`batch_delete_group_addresses_impl`, `batch_move_devices_to_line_impl`,
`batch_move_devices_to_building_part_impl` in `domain.rs` (the same
one-`*_impl`-per-command shape every other command already uses) and
four routes: `POST /api/devices/batch-delete`, `POST
/api/group-addresses/batch-delete`, `POST
/api/devices/batch-move-line`, `POST
/api/devices/batch-move-building-part` — each rejects an empty id list
with 400 before constructing the `Batch`, rather than trusting the
client. `apps/knx-web` gains `MultiSelectionKind`/`MultiSelection` in
`selection.ts`, ctrl/shift-click multi-select threaded additively
through `ProjectExplorer.tsx` (the existing plain-click `onSelect`
contract is unchanged), and a new `BulkActionToolbar.tsx` rendered above
the tree whenever a multi-selection is active, wired to the four new
`api.ts` functions (`batchDeleteDevices`, `batchDeleteGroupAddresses`,
`batchMoveDevicesToLine`, `batchMoveDevicesToBuildingPart`). Starting a
multi-select of one kind (devices vs. group addresses) replaces any
active multi-select of the other kind; mixed-kind selection isn't
supported. The plan text and design spec both claimed the toolbar should
confirm via `confirm()` "consistent with `Inspector.tsx`'s existing
single-delete buttons" — false on inspection, `Inspector.tsx` has no
`confirm()` calls anywhere and its Delete buttons act immediately,
relying on undo as the only safety net. Built to match the actual
existing behavior instead: bulk actions act immediately, with the whole
batch undoable as one `Ctrl+Z`. No batch "address reassignment" for
individual addresses — B9's own wording described topology
reassignment, already covered by `MoveDeviceToLine`/
`MoveDeviceToBuildingPart`. Out of scope, recorded in the design spec:
copy/paste with parameters, mixed-kind batch edit. New tests: 3 in
`knx-core` (`command::tests`, full suite 125 passing), 13 in
`apps/knx-server/tests/http_batch_routes.rs`, 10 new `vitest` tests in
`knx-web` (114/114 passing). Gates: `cargo fmt --all --check`, `cargo
clippy --workspace --all-targets -- -D warnings`, `cargo test
--workspace`, `cargo run -p xtask -- check-layering`, and `npm test` /
`npm run build` on `knx-web` all clean. Closes **T9**
([GAP_ANALYSIS_ETS.md](GAP_ANALYSIS_ETS.md)), **B9**. Design spec:
`docs/superpowers/specs/2026-09-10-bulk-operations-design.md`.

**T11, session log / import-report review screen (2026-09-10).** A new
`SessionLog` module (`apps/knx-server/src/session_log.rs`) owns an
in-memory, per-server-process `Vec<LogEntry>` — never written to
`.knxdb`, held in `AppState.session_log: Mutex<SessionLog>`. `LogEntry {
timestamp, severity, source, message, location, detail }` serializes
`#[serde(rename_all = "camelCase")]`, `severity` exactly `"error"` |
`"warning"` | `"info"`, matching `knx_etsproj::report::Severity`'s own
convention rather than inventing a second one. `from_import_report()`
converts the same `ImportReport` `import_and_project` already produces
for `ProjectTree`'s counts (its return type gained the report as a
4th tuple element rather than discarding it) into warning/info/error
entries, in order: `ImportError` (severity per its own field), then
every `UnknownConstruct`, then every `OpaqueSummary` (info-level — the
opaque store already keeps the bytes, so nothing is lost either way, but
CLAUDE.md's "never silently discard information" rule put it in the log
alongside every other report category), then every `Conflict`, then
every `UnsupportedFeature`. `report.inferred` and
`SourceInfo::namespace_disagreement` are deliberately not mapped —
a documented residual, not a silent drop (see
`docs/KNOWN_LIMITATIONS.md`). Every other project-level operation logs
one info entry on success or one error entry (the server's existing
user-facing error string) on failure — but only once the operation
reaches an actual `Command` dispatch or a project-level operation's own
top-level `Result`: a failure caught earlier (a bad address parse, an
empty id list, `"no project open"`, `"no product database configured"`,
`"catalog item not found"`) produces a toast but never reaches the log.
`open_project`/`open_native_project` additionally reset the log on
success only — a failed import/open appends an error entry without
touching whatever was already there, so a user re-trying a bad import
doesn't lose earlier context. `apply()`, the shared dispatcher every
`*_impl` command function funnels through, and `create_device_impl`
(which cannot call `apply()` itself — it needs product-catalog
enrichment to run under the same project lock and returns a richer
`CreateDeviceResponse`) both log one entry per command: `source`/
`message` on success are the command's own short variant name (e.g.
`"SetIndividualAddress"`, `"Batch"` — the first token of its `Debug`
form), `message` on failure is still the error string, and `detail`
carries the full `Debug` dump exactly once. A whole-branch review found
three near-identical logging helpers (`log_outcome`, `log_command_outcome`,
`log_undo_redo`) that had accreted one per call-site family, and that
the full `Debug` dump was being stored — and rendered — twice per edit
entry (as both `source` and `message`); a fix round (still 2026-09-10)
collapsed all three into one `log_outcome()` plus a small
`command_name()` helper, fixing the duplication for `Command::Batch`/
`Command::CreateDevice` entries, which can otherwise run to several KB.
`export` was folded in using the same info/error shape as `save`, even
though the approved design doc's own operation list ("import/open/save/
undo/redo/edit") never named it — excluding the one other fallible
project-level operation would have been an arbitrary, undocumented gap
the design's own "operational feedback" rationale argues against.
`open_project`'s own import-summary entry now reports `mapped/read` per
entity (was `mapped` only) — the same fix round found the `read` count,
the actual import-loss signal when it exceeds `mapped`, was being
silently dropped from the one log line that summarizes the whole
import. `GET /api/log` (new route in `routes.rs`) returns every entry
for the session, oldest-first, as a bare JSON array, always `200`
(never `404` — an absent project is just `[]`). `apps/knx-web` adds
`LogEntry`/`getSessionLog()` to `api.ts` (hand-written interface, no
`ts-rs` binding, same convention as `CatalogInstallReport`) and a new
`LogPanel.tsx`, wired into `App.tsx` via a `logOpen` boolean and a "Log"
toolbar button (disabled until a project is open) that swaps into the
same `.workspace` slot as Inspector/Dashboard; selecting an entity
closes the panel, matching how selection already dismisses the
Dashboard. `LogPanel` fetches on mount and whenever its `tree` prop
changes (so it stays current across an edit/undo/import left open),
renders newest-first (a client-side reversal of the API's oldest-first
order), and has three Error/Warning/Info toggle filters, all on by
default, that only affect already-fetched entries and never re-fetch.
Task review caught one gap before merge: the initial fetch had no
`.catch`, so a failed `GET /api/log` (server restart mid-session, etc.)
produced a silent unhandled rejection with no user feedback — fixed to
mirror `CatalogBrowser.tsx`'s existing `.catch` + `.field-error`
convention (a deliberate deviation from the original design spec, which
described a toast for this case; the inline `.field-error` rendering
was introduced during Task 2's own fix round but never written down
until this entry), and the empty state was split into "No log entries
yet." (truly empty) vs. "No log entries match the current filters."
(entries exist, all severities toggled off) rather than conflating the
two. The final whole-branch review round above also found the open Log
tab never refreshed after a *failed* operation — `tree` (the fetch's
only dependency) only changes on success, so a failed save/export/edit/
undo/redo/import logged correctly on the server but the open tab
wouldn't show it until an unrelated successful operation happened to
change `tree`, which was precisely the scenario this feature exists
for. Fixed with a `logVersion` counter in `App.tsx`, bumped by a new
`reportError()` wrapper on every error path (all 9 `catch`-block
`pushError` call sites in that file), threaded into `LogPanel` as a
second `refreshKey` prop/effect-dependency alongside `tree`.
New tests: 6 new unit tests in `apps/knx-server` — 3 in
`session_log.rs`, 3 in `domain.rs` (verified via `cargo test -p
knx-server --lib -- --list`; the crate's `--lib` total is 20, the other
14 predate T11 or cover unrelated modules such as `paths.rs`) — plus a
dedicated `apps/knx-server/tests/http_log_route.rs` integration test
suite, now 2 tests after the fix round split out a corpus-free
fresh-state-returns-`[]` check (previously the file's only test
returned early before any assertion ran when the gitignored
`OriginalData/` corpus was absent, so `GET /api/log` had never once
been exercised in CI) from the gated import → failed edit → successful
edit, correct-append-order test. `knx-web` gains `LogPanel.test.tsx`, 8
tests (newest-first, per-severity filter show/hide with no re-fetch,
both empty states, fetch-error render and clear, refetch-on-tree-change,
refetch-on-refreshKey-change), full suite 122/122 passing (was 114
before T11, 121 before this fix round). Gates: `cargo fmt --check`,
`cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D
warnings`, `npx tsc --noEmit`, `npm test`, `npm run build` (with the
pre-existing `dist/.gitkeep` restore) all clean on the merged branch.
Parked as a new `KNOWN_LIMITATIONS.md` entry rather than fixed in this
round: the Log tab is unreachable without an open project even though
`GET /api/log` deliberately works with none, and `SessionLog` has no cap
on entry count. Closes **T11**, **D7**
([GAP_ANALYSIS_ETS.md](GAP_ANALYSIS_ETS.md)). Design spec:
`docs/superpowers/specs/2026-09-08-session-log-design.md`.

**T11 follow-up, Log tab reachability and a session-log growth cap
(2026-09-10).** Closed both gaps `KNOWN_LIMITATIONS.md` #36 parked above.
`App.tsx`'s "Log" button is unconditionally enabled, and the
`.workspace` slot now renders on `tree || logOpen` instead of `tree`
alone, so `LogPanel` (prop type now `tree: ProjectTree | null`) is
reachable with no project open — `ProjectExplorer` stays gated on
`tree`, since it genuinely needs one, so an empty-project Log tab is the
only thing in that slot; with a project open the layout is unchanged.
`session_log.rs` gained a documented `MAX_ENTRIES: usize = 1000` const;
past it, `SessionLog::push` evicts the oldest real entries and pins a
synthetic `Severity::Warning`/`source: "log"` entry at index 0 naming
the running total of real entries dropped, refreshed on every further
drop, itself never dropped/duplicated, and counted against the cap so
`entries().len()` never exceeds 1000; `reset()` clears the dropped count
too. `dropped` counts real entries actually removed, not overflowing
calls: it jumps by 2 on the push that first exceeds the cap (one entry
evicted for being oldest, one more to make room for the synthetic entry
itself) and by 1 on every push after that — an earlier round of this fix
counted overflowing calls instead and undercounted by one from the
first drop onward, which is exactly the kind of silent-discard CLAUDE.md
rules out, so it was corrected before merge. `GET /api/log`'s bare-array
wire shape is unchanged, so T12's `from_csv_import_report` and
`apps/knx-server/tests/http_log_route.rs` needed no changes. New tests:
6 in `session_log.rs` (under/at/one-past/well-past the cap,
reset-after-a-drop, and an invariant test pinning "dropped plus retained
equals total pushed" at two different overflow sizes) and a new
`App.test.tsx` (2 tests: reachable with no project, unchanged with
one) — full suites `cargo test --workspace` and `npm test -- --run`
(139/139) both clean, plus `cargo clippy --workspace --all-targets -- -D
warnings`, `cargo run -p xtask -- check-layering`, `npx tsc --noEmit`,
`npm run build`.

**T12, CSV group-address import/export (2026-09-10).** A new
`crates/knx-csv` crate — pure, depending on nothing but `knx-core`, `csv`,
and `serde`, with a matching `xtask check-layering` rule keeping it away
from `knx-store`/`knx-etsproj`/`knx-productdb` — reads and writes
"KNXBench group-address CSV v1": `export_group_addresses(&Project) ->
CsvExport` (`write.rs`), `parse_group_addresses(text, style) -> ParsedCsv`
(`read.rs`), and `plan_import(&Project, &ParsedCsv) -> ImportPlan`
(`plan.rs`). **The format is KNXBench's own, not ETS's.** No sample of
ETS's "Export Group Addresses" CSV output exists in this repository or in
the KNX Standard v3.0.0 corpus, and it is not a KNX Association standard
either, so nothing here — docs, UI text, commit messages — claims ETS
compatibility; the stated upgrade path if a real ETS sample ever turns up
is a second column profile in the same importer, not a rewrite.
`crates/knx-core/src/command.rs` gains `Command::UpdateGroupAddress { id,
name, central, unfiltered }` (its own inverse, following
`SetDeviceDescription`/`RenameGroupRange`'s existing pattern) — the one
command this feature needed and the first thing able to rename a group
address at all; it deliberately never touches the address or the range,
since the address is import's match key and range placement is a separate
concern. The importer auto-detects `,`/`;` separators, accepts a BOM or
none and CRLF or LF, matches rows to existing entries by address (never by
name), and produces one of four outcomes per row — create, update,
unchanged, or error — all-or-nothing per file: any row-level error blocks
the whole import, and a successful one applies as a single `Command::Batch`
so it is one undo step. `DatapointType`, `MainGroup`, and `MiddleGroup` are
written on export (derived from linked communication objects and
containing group ranges) but never applied on import — read back and
reported as recognized-but-ignored, since a group address itself carries
no DPT in this domain model and no range is ever created by a CSV import.
Import never deletes an address absent from the file and never
re-addresses an existing one (an address change in the file reads as a new
row); both are recorded in `KNOWN_LIMITATIONS.md` alongside the missing
`Description`/`Comment` columns (the domain model has no such fields to
round-trip). Surfaces: `POST /api/group-addresses/csv-export`/
`csv-import` (`apps/knx-server/src/routes.rs`, both logging to the T11
session log through a new `session_log::from_csv_import_report`, neither
resetting it); `knx ga-export <store.knxdb> <out.csv>` and `knx ga-import
<store.knxdb> <in.csv> [--dry-run]` on the CLI, with `--dry-run` printing a
report body byte-identical to a real import (the two share one
`print_import_report` function) and a trailing `store written: yes`/`no
(dry run|nothing to do|rejected|error)` line appended after that shared
body on every path, so the report itself can never diverge between a dry
run and a real one; a `GroupAddressCsvButtons.tsx` pair of toolbar buttons
in the web group-address view, reusing the existing `pickSavePath`/
`pickOpenPath` file dialogs and the Log tab for detail. `knx-app`'s test
suite gains a corpus-gated round trip,
`exporting_and_replanning_the_reference_project_is_entirely_unchanged`
(`tests/csv_roundtrip.rs`): import the reference `.knxproj`, export its
group addresses, re-parse that text, and re-plan against the same
project — the plan comes back entirely `unchanged` with no problems, which
is the real proof that the writer and the reader agree, including on names
containing commas, quotes, and umlauts a hand-built fixture cannot exercise
realistically. It lives in `knx-app`, not `knx-csv`, because it needs
`knx-etsproj` to produce a real `Project`, and `knx-csv` must reach neither
`knx-etsproj` nor `knx-store` — `cargo metadata`'s dependency graph does
not distinguish `[dev-dependencies]` from `[dependencies]`, so
`check-layering` genuinely rejected a `knx-etsproj` dev-dependency inside
`knx-csv` itself during development, and `knx-app` is deliberately the one
crate already permitted to see both sides. New tests, each figure
re-verified via `cargo test -p <crate> -- --list` at documentation time:
`knx-csv`'s own suite (separator detection, BOM, CRLF, quoted fields with
embedded separators/quotes, unknown columns, every row-level error, every
documented boolean spelling, all three address styles, plan outcomes,
range-by-containment placement, the all-or-nothing rule) is 50 tests; 4 new
`Command::UpdateGroupAddress` tests in `knx-core::command::tests`; 5 new
server integration tests
(`apps/knx-server/tests/http_group_address_csv.rs`) covering both routes,
the rejected-file case, and the session-log append; 5 new CLI integration
tests (`apps/knx-cli/tests/cli_group_address_csv.rs`) including the
`--dry-run` byte-identical-report case and a read-only-store save-failure
case; 11 new frontend tests in `GroupAddressCsvButtons.test.tsx` plus 4
more in `api.test.ts` for the two new client functions. Two items parked
rather than fixed in this branch: the server's session-log source strings
use the shape `csv-import:<kind>` (`apps/knx-server/src/session_log.rs`,
`from_csv_import_report`), where `<kind>` is `"error"`/`"warning"` derived
straight from the same `problem.severity` the entry's own `severity` field
already carries — a Minor noted during review and left as-is rather than
reworked late in the cycle;
and `crates/knx-store/src/command_sync.rs`'s module doc, pre-existing and
unrelated to this task, describes `sync_after_command` as *the*
incremental-persistence mechanism, but grepping `crates/` and `apps/` for
`sync_after_command` finds no caller anywhere outside its own tests and
`lib.rs`'s re-export — persistence in this codebase actually runs through
`save_project`, which is what both `POST /api/group-addresses/csv-import`
and `knx ga-import` call. This task's own `Command::UpdateGroupAddress`
arm in that same file is one more no-op stub alongside the topology/
group-range/group-link/device-create-delete arms already there, which made
the doc/reality gap easier to notice, not the cause of it — see
`KNOWN_LIMITATIONS.md`. Closes **T12**, **C2**
([GAP_ANALYSIS_ETS.md](GAP_ANALYSIS_ETS.md)). Design spec:
`docs/superpowers/specs/2026-09-10-csv-group-address-exchange-design.md`.

**T13, project documentation export, HTML only (2026-09-10).** A new
`crates/knx-report` crate — pure, depending only on `knx-core`,
`knx-projection`, and `chrono`, with a matching `xtask check-layering`
rule keeping it away from `knx-store`/`knx-etsproj`/`knx-productdb` and
every `CORE_FORBIDDEN` dependency — renders a `&knx_core::Project` into
one self-contained "project documentation" HTML file via its one public
entry point, `render_html(&Project, &ReportOptions) -> HtmlReport`
(`lib.rs`). `ReportOptions::generated_at` is the only source of "now"
inside the crate, so the same project and timestamp always render to
byte-identical HTML — the property a determinism test holds it to.
`render_html` cannot fail: `HtmlReport::warnings` describes structural
oddities *in the project* (a device in no line, a group address in no
range, a building part with a dangling parent, a communication object
owned by no device, a link naming a group address that does not exist,
or an `Override::Malformed` field), never a rendering error, and every
warning is rendered inline in the document's own body as well as
returned to the caller (CLAUDE.md: never silently discard information).
Internally split into `model.rs` (pure derived indices — the building
forest, group-range nesting, the group-address → communication-object
inverse index, every orphan list — unit-testable without an angle
bracket) and `render.rs` (walks `&Project` plus those indices into HTML).
Eight sections in one document: Header, Contents, Summary, Topology,
Buildings, Group addresses, Devices, and "What this report does not
contain" — the device section deliberately reads from two sources, not
one: most fields come from `knx_projection::build_device_detail`
(already resolving a communication object's text/description/DPT through
the string table and provenance layers, and its links to formatted
addresses — reusing it rather than duplicating that resolution), but
`commissioning`, `product_ref`, `program_ref`, and `binary_data` are not
in what that function returns, so those four are read directly off
`project.devices.get(id)` in the same loop. **This is KNXBench's own
document, never called, described, or commit-messaged as an ETS report**
(`crates/knx-report/src/lib.rs`'s own module doc states this) — no
ETS-produced report sample of any kind (PDF, printout, or export) exists
anywhere in this repository, and `docs/RESEARCH.md` has no section on
ETS's report layout, the same evidence gap T12's CSV format already
documents (`KNOWN_LIMITATIONS.md §38`). A group address has no datapoint
type of its own in this domain model, so rather than printing one
guessed consensus DPT, the document lists every linked communication
object with its own DPT — a deliberately different computation from
`knx-csv`'s `derive_dpt` (which answers "is there a unanimous DPT?" for a
CSV column), so no logic is shared between the two crates. The document
is one self-contained UTF-8 file: no JavaScript, no external assets, one
inline `<style>` block with an `@media print` rule (no page breaks inside
a table row, each top-level section starts a new page) and no animation
or transition of any kind — PDF comes from the browser's own print
dialog, not from KNXBench, since no Rust PDF renderer exists in this
workspace and none is planned. Surfaces: `POST
/api/project/documentation-export {path} -> {warnings}`
(`apps/knx-server/src/routes.rs`), writing through the same
`resolve_new_project_path` helper the `.knxproj`/CSV exports use and
logging one T11 session-log entry per warning under `source:
"doc-export"` without resetting the log; `knx doc-export
<store.knxdb> <out.html>` (`apps/knx-cli`), printing a summary and every
warning and exiting `1` only when no file could be produced at all (a
report with warnings is still a complete, correct report, so there is no
separate warning exit code); an "Export documentation…" button
(`DocumentationExportButton.tsx`, `apps/knx-web`) in the same toolbar row
as the `.knxproj`/CSV export controls. Tests, every count re-verified via
`cargo test -p <crate> -- --list` at documentation time: `knx-report`'s
own suite (`html.rs`, `model.rs`, `render.rs`) is 43 tests, covering
determinism, escaping of `&`/`<`/`>`/`"`/`'` in device/group-address/
building-part names, self-containment (no `<script`, no `http://`/
`https://`, no `transition:`/`animation:`), every orphan/dangling-
reference finding, and that every device/group-address/building-part
name in a small project appears exactly once; `apps/knx-server/tests/
http_documentation_export.rs` (4 tests: a successful export, a
warning-generating export with session-log entries, a
path-outside-data-directory rejection, and the no-project-open 400 case);
`apps/knx-cli/tests/cli_documentation_export.rs` (4 tests, including
per-warning printing and a missing-store exit-1 case);
`DocumentationExportButton.test.tsx` (6 tests). A corpus-gated
integration test,
`rendering_the_reference_project_produces_a_complete_self_contained_document`
(`crates/knx-app/tests/documentation_export.rs`), imports the reference
`.knxproj` (36 devices, 907 communication objects, 514 group addresses on
this corpus) and asserts: every group address's formatted string appears
in the output; every device name appears, escaped; `<table>`/`</table>`
and `<tr>`/`</tr>` counts balance; the Summary section's nine stated
counts equal counts computed independently from the `Project`, not just
re-read from the model; the output contains no `<script`, `http://`, or
`https://`; and a second render with the same timestamp is
byte-identical — a real proof against a project a hand-built fixture
cannot exercise realistically (907 communication objects is large enough
that a stray `HashMap` iteration would show up as flaky output). It lives
in `knx-app`, not `knx-report`, for the same dev-dependency-layering
reason `csv_roundtrip.rs` does: `check-layering` walks dev-dependency
edges too, so a `knx-etsproj` dev-dependency inside `knx-report` would
trip `knx-report`'s own rule, and `knx-app` is deliberately the one crate
already permitted to see both sides. **Closed for HTML only**: printing
from inside the application and native PDF generation (without a
browser) remain open, and new `KNOWN_LIMITATIONS.md` entries (§44-§50)
record those plus no ETS report parity, no manufacturer/product/program
name resolution, no parameter/module-argument listing, single-language
rendering, and no section selection. Closes **T13**, **D4**
([GAP_ANALYSIS_ETS.md](GAP_ANALYSIS_ETS.md)). `ROADMAP.md` was checked
and names neither T13 nor D4, so it was left untouched by this task (a
separate, unrelated memo about future motion/animation style direction
was added to its existing Motion and animation section, at explicit
request, mid-task). Design spec:
`docs/superpowers/specs/2026-09-10-project-documentation-export-design.md`.

**T14, project diff/compare, `.knxdb`-to-`.knxdb` only (2026-09-10).** A
new `crates/knx-diff` crate — pure, depending on `knx-core` only, with a
matching `xtask check-layering` rule keeping it away from
`knx-store`/`knx-etsproj`/`knx-productdb` and every `CORE_FORBIDDEN`
dependency, and carrying no `serde` (matching `knx-report`'s own
no-`serde` precedent) — computes what changed between two
`&knx_core::Project`s via its one public entry point,
`diff_projects(&Project, &Project) -> ProjectDiff` (`lib.rs`). This is an
independent reimplementation of `knx-etsproj::compare`'s matching
technique, not a reuse of `compare.rs` itself: `compare.rs` answers "did
import → export → import preserve everything" (byte-identical fields);
`knx-diff` answers "what did a human change between two saves" (matched
primarily by identity, a looser and different question) — folding the
two into one type would make a future change to either risk silently
breaking the other (design spec §2). Internally split into `key.rs`
(the generic three-pass matching engine: an `ets_id` match wins
regardless of field agreement; failing that, a natural key per entity
type is tried only among each side's post-`ets_id` leftovers; a natural
key with more than one leftover candidate on either side is never
guessed at — both candidates land in `added`/`removed` and one
`AmbiguityNote` records the collision), `semantic.rs` (per-entity key/
field extraction, direct against `knx_core`, reimplementing
`compare.rs`'s `Override<T>`/`Layer::is_exported()` resolution rather
than depending on `knx-etsproj` to reuse it), and `diff.rs`
(`diff_projects` itself, ties installations then each entity table
within each installation together; devices get their own
`DeviceTable`/`DeviceChange` instead of the generic `EntityTable`/
`EntityChange`, so their nested `com_objects`/`parameters` tables live in
one place, not duplicated across `left`/`right`). A matched pair with
zero differing fields produces no output at all (`git diff` convention,
not "unchanged"). `diff_projects` is pure — no clock, filesystem, or RNG
— and every output list is sorted by the entity's own display key, never
`HashMap` iteration order (design spec §4), the same determinism
discipline `knx-report` holds itself to. **This is KNXBench's own diff,
never described as an ETS comparison or a replacement for one** — no
ETS-produced comparison sample of any kind exists anywhere in this
repository, so no parity claim is made anywhere, the same evidence gap
T12's CSV format (`KNOWN_LIMITATIONS.md` §38) and T13's HTML report
(`KNOWN_LIMITATIONS.md` §44) already record. Surfaces: `POST
/api/project/diff {path} -> ProjectDiffDto` (`apps/knx-server/src/routes.rs`),
comparing the server's open, possibly-edited, in-memory project against a
`.knxdb` file at `path` — deliberately "what would Save change," not
"diff two files on disk" — rejecting a missing comparison path or no
open project with `400`, and checking `path.exists()` itself before
calling `knx_store::open_and_migrate` so a typo'd comparison target never
silently becomes an empty, freshly created `.knxdb` reporting every
entity as "removed" (design spec §7's own named correctness gotcha, not
inherited here); every `knx-diff` type gets a hand-written
`#[derive(Serialize)] #[serde(rename_all = "camelCase")]` DTO in
`routes.rs`, generic where `knx-diff`'s own types are generic
(`EntityTable`/`EntityChange`/`AmbiguityNote`) and bespoke where
`knx-diff`'s are (`DeviceTable`/`DeviceChange`), the exact pattern
`DocumentationWarningDto`/`DocumentationExportReportDto` already
establish. `knx diff <a.knxdb> <b.knxdb>` (`apps/knx-cli`), loading both
stores independently and printing plain text, `+`/`-`/`~` prefixed lines
grouped by section, `"no differences found"` when nothing differs
anywhere, exiting `0` whenever a comparison is successfully produced —
mirroring `knx doc-export`'s own "a report with content is not a failed
report" reasoning, so a diff with changes is not a failed diff either. A
"Compare with…" button (`ProjectDiffPanel.tsx`, `apps/knx-web`) opens an
*existing*-file picker (`pickOpenPath`, filtered to `.knxdb`, unlike
`DocumentationExportButton`'s save-target picker) and renders one
grouped-count summary line per non-empty table across every installation
(e.g. `Devices: 1 added, 2 changed`; installations are prefixed with
their id only when the report has more than one) — no tree view, no
inline before/after value highlighting, the same visual register as the
existing Log tab (design spec §5, §9). Tests, every count re-verified via
`cargo test -p <crate> -- --list` at documentation time: `knx-diff`'s own
suite (`key.rs`, `semantic.rs`, `diff.rs`) is 48 tests, covering the
three-pass matching algorithm over hand-built collections, every
entity's field extraction, `Override` resolution at every provenance
layer, a determinism check (two runs on the same inputs produce
`PartialEq`-equal `ProjectDiff`s), a genuine two-project test (one change
per entity kind via `knx_core::Command` where a `Command` exists and a
direct struct mutation where it does not), and
`diff_projects(&p, &p)`'s own anchor property (design spec §3.7) against
a hand-built fixture containing one of every entity type, including both
a module-based and a monolithic device; `apps/knx-server/tests/http_project_diff.rs`
(4 tests: an identical-project empty diff, a changed-device-description
diff naming the change, a missing-comparison-path 400 that creates no
file, and the no-project-open 400); `apps/knx-cli/tests/cli_project_diff.rs`
(4 tests: two identical stores report no changes, one changed
group-address name is printed, a missing first store exits 1 without
creating a stray `.knxdb`, and wrong argument count prints usage and
exits 1); `ProjectDiffPanel.test.tsx` (8 tests: disabled with no project
open, a cancelled picker calls nothing, the picked path is passed with
the right filter, an empty diff says so, a diff with changes renders
grouped counts, a table with only ambiguous entries still renders a
line, a rejected comparison surfaces through `onError`, and a prior error
toast is cleared before comparing). A corpus-gated integration test,
`rendering_diff_projects_between_two_independent_imports_of_the_reference_project_is_empty`
(`crates/knx-app/tests/project_diff.rs`), imports the reference
`.knxproj` **twice, independently** — not one project cloned, the
strongest form of design spec §3.7's property, since it exercises two
separate mapper runs' `ets_id` agreement end to end — getting 36 devices,
907 communication objects, and 514 group addresses on this corpus each
time, and asserts `diff_projects` between the two imports has empty
`info_changes`, every installation `Matched` with empty
`field_changes`, and every one of `areas`/`lines`/`devices`/
`group_ranges`/`group_addresses`/`buildings` entirely empty
(`added`/`removed`/`changed`/`ambiguous` all zero-length) at every
level, plus a determinism re-run. It lives in `knx-app`, not `knx-diff`,
for the same dev-dependency-layering reason `csv_roundtrip.rs`/
`documentation_export.rs` do: `check-layering` walks dev-dependency
edges too, so a `knx-etsproj` dev-dependency inside `knx-diff` would trip
`knx-diff`'s own rule, and `knx-app` is deliberately the one crate
already permitted to see both sides. Run standalone with `cargo test -p
knx-app --test project_diff -- --nocapture`, it printed: `project_diff
corpus test: 36 devices, 907 communication objects, 514 group addresses`
and passed — proof the corpus path ran, not the skip path. **Closed for
`.knxdb`-to-`.knxdb` comparison only**: comparing against a raw
`.knxproj` is not supported on either side (only two `.knxdb` files on
the CLI, or the open project against one `.knxdb` file on the
server/web); there is no merge/apply of a diff back onto a project; no
three-way comparison; no detection of an ETS re-import's regenerated
`RefId`s as "the same entity"; no CI-friendly "exit nonzero on any
difference" CLI flag; a device with no individual address and no
matching `ets_id` cannot be correlated across two projects and surfaces
as an unrelated add+remove; two same-named sibling building parts under
the same matched parent collide under the path-based key and trigger the
ambiguity path — recorded in new `KNOWN_LIMITATIONS.md` §51-§58. Two more
entries (§59, §60) record rendering-scope findings from implementation
and review, not from the original brief: the text/web renderers report
the *names* of an entity's changed fields, not their before/after
values — project-level and installation-level `FieldChange`s are the
exception and do render both values — and the web panel shows grouped
counts only, no tree view, no inline before/after highlighting (design
spec §9 names both as out of scope). Closes **T14**, **C1**
([GAP_ANALYSIS_ETS.md](GAP_ANALYSIS_ETS.md)). `ROADMAP.md` was checked
and names neither T14 nor C1, so it was left untouched by this task.
Design spec: `docs/superpowers/specs/2026-09-10-project-diff-design.md`.

**R3 research spike, no code change (2026-09-11) — the `when/@test`
grammar.** A read-only spike against 4 `.knxprod` product databases and 3
`.knxproj` demo/reference projects (`OriginalData/`, 34 `ApplicationProgram`
elements, 22630 `when`, 12149 `choose`) found that the KNX Standard v3.0.0
normatively specifies the `@test` value grammar (`Condition_t`,
`Project Schema23 v01.00.00.md` §1.1.3.18) and that the
`choose`→`ParameterRef`→`ParameterType` resolution chain is 100%
resolvable with zero dangling references. It also found the surrounding
structural grammar (`Dynamic`, `Channel`, `ParameterBlock`, `choose`,
`When_t`, and a previously-undocumented `ChannelIndependentBlock`) is not
covered by any schema document in this repository's KNX Standard
extraction, and remains corpus-observed only. Full findings:
[RESEARCH.md §4.3](RESEARCH.md); log:
`.ai/logs/2026-09-11_claude_r3_dynamic_grammar.md`. **This closes risk R3
as a research question. It changes no code and lifts no limitation**: no
parameter evaluator or editor exists, `ParameterInstance` values are still
held as opaque raw strings, and [KNOWN_LIMITATIONS.md §3](KNOWN_LIMITATIONS.md)
stays open. What changed is that T18 (parameter interpretation and editor,
[GAP_ANALYSIS_ETS.md](GAP_ANALYSIS_ETS.md) Tier 5) is no longer blocked on
research — it now needs a design decision on the no-match-branch policy
(common in the corpus: 5570/8732 no-default `choose` elements have a legal
value no `when` covers) and a defensive parser posture, both ordinary
implementation work, not research.

**T18 slice 1, `Dynamic` tree parse/store/evaluate (2026-09-11).** The
first of three planned slices ([design spec](superpowers/specs/2026-09-11-dynamic-tree-parse-and-evaluate-design.md),
[plan](superpowers/plans/2026-09-11-dynamic-tree-parse-and-evaluate.md)).
`knx-productdb` gains a `dynamic` module, no other crate touched:

- **Parse and store (schema v3).** A new `dynamic_node` table stores one
  row per element of the `ApplicationProgram`'s own `Dynamic` tree and
  each `ModuleDef`'s own, in document order, `kind` as the literal XML
  element name (never a mapped enum — an unrecognized kind is stored
  under its own name, not dropped). Modelled attributes get dedicated
  columns; every other attribute lands in `extra` and is reported through
  the existing `UnknownCollector`. `@test` is stored verbatim, unparsed.
  The `Static` parser's pre-existing `Dynamic`-skip is untouched; the new
  parser runs as a second pass over the same bytes in the same ingest
  transaction.
- **v2→v3 backfill.** The migration re-reads every stored `source_file`
  blob through the new parser, so a database installed before this build
  gains its `dynamic_node` rows without a re-install (the payoff
  [ADR-0011](adr/0011-product-database-storage.md)'s blob store was kept
  for). Each blob's parse runs inside its own `SAVEPOINT`: a blob that
  fails to parse leaves zero `dynamic_node` rows and a recorded
  `ingest_unknown` diagnostic (`DynamicBackfillError`), and never aborts
  the rest of the migration.
- **A pure, headless evaluator** (`dynamic::evaluate`), single-pass,
  depth-first, document order: given a loaded tree and a parameter-value
  map (3-tier fallback — supplied value, then `parameter_ref.value`, then
  `parameter.value`), it returns the active `ParameterRef`/`ComObjectRef`
  ids (deduplicated by first occurrence) plus diagnostics. Every `Test`
  shape in `Condition_t` is implemented (`=`, `!=`, `>`, `<`, `>=`, `<=`,
  a single number, a space-separated list). No matching branch activates
  nothing under that `choose` and is reported (`NoBranchMatched`) — an
  **inference** (RESEARCH §4.3), not a documented rule. A missing or
  non-numeric controlling value gets its own diagnostics
  (`MissingValue`/`NonNumericValue`) rather than being folded into
  `NoBranchMatched`. A `TypeNone`-controlled `choose` takes its sole
  default branch without a comparison, exactly as all 604 corpus
  occurrences look; any other shape under it is
  `UnexpectedTypeNoneShape`. An unrecognized element kind is
  `UnrecognizedNode` and its subtree is not descended. **At this point in
  the slice, `Module` is recognized but not expanded — it evaluates to
  `ModuleNotExpanded`.** *(Superseded the same day: T18 slice 2, below,
  expands `Module` and removes this diagnostic. It is described here
  exactly as slice 1 shipped it, for the record.)* Module expansion
  (slice 2) and the editor (slice 3) are not built yet.
  Nothing outside the crate's own tests calls the evaluator; it is dead
  code from every other crate's perspective, exactly as planned. Import
  is unaffected — it still reads `GroupObjectTree` ([ADR-0014](adr/0014-group-object-tree-authoritative-source.md))
  and never evaluates this tree.
- **Corpus evidence**, measured against the four `.knxprod` archives under
  `OriginalData/ProductDatabases/`: stored `choose`/`when` counts match
  RESEARCH §4.3 exactly (1646/2252, 5/5, 509/982, 0/0), zero dangling
  `choose/@ParamRefId`, zero `UnparsableTest`/`UnresolvedParamRef`/
  `UnexpectedTypeNoneShape` diagnostics anywhere, and the `@test` shape
  histogram matches §4.3 restricted to these archives. One research note
  came out of this measurement, added to [RESEARCH.md §4.3](RESEARCH.md):
  all 62 corpus `SPACE_LIST_OF_INTEGERS` `@test` values, like all 13
  `OP_NUMBER` values, occur in the MDT archive (`prod3`) alone.

Two commits (`66ef369` parse/store, `44b06a1` evaluate/backfill), one fix
round each after review (`c7d9ed5`, `2368292`) — a `default="false"`
attribute silently dropped, a self-closing `<ModuleDef/>` leaking its id
onto later siblings, and a mid-file backfill parse failure leaving stray
rows behind (the `SAVEPOINT` fix above) were the load-bearing findings;
full detail in each fix round's own report. `cargo test --workspace`: 809
passed / 0 failed / 3 ignored, up from 784 before this slice. Closes no
`GAP_ANALYSIS_ETS.md` item outright — **A3** moves from "not interpreted"
to "partially closed": an evaluator exists, nothing surfaces it. This
docs-only pass (T18 slice 1's third task) reconciles
[KNOWN_LIMITATIONS.md §3/§12/§47](KNOWN_LIMITATIONS.md), [COMPATIBILITY.md](COMPATIBILITY.md),
[DATA_MODEL.md §10](DATA_MODEL.md), [GAP_ANALYSIS_ETS.md](GAP_ANALYSIS_ETS.md),
[ROADMAP.md](ROADMAP.md), [ARCHITECTURE.md](ARCHITECTURE.md) and
[IMPORT_EXPORT.md](IMPORT_EXPORT.md) with what actually shipped, and
retires the two source comments (`knx-core/src/parameter.rs`,
`knx-core/src/module.rs`) that still called the `Dynamic` grammar
unresearched.

**T18 slice 2, `Module` expansion (2026-09-11).** The second of the three
slices planned above, same day as slice 1
([design spec](superpowers/specs/2026-09-11-module-expansion-design.md),
decisions D12-D19; [plan](superpowers/plans/2026-09-11-module-expansion.md)).
`knx-productdb`'s evaluator now follows a `Module` node into its
referenced `ModuleDef`'s own stored tree instead of stopping at it. No
schema change — the product database stays at **v3**; `dynamic_node`
already stored everything this slice reads (D12).

- **`evaluate`'s input grows from one tree to a tree set, and stays pure.**
  `evaluate(trees: &ProgramTrees, values: &ValueMap) -> Activation` —
  still no `Connection`, no I/O, no logging (D13). `ProgramTrees` holds the
  program's own tree plus one tree per `ModuleDef` the program references;
  `load_program_trees(conn, program_id)` is the only new
  database-touching function, loading the program tree and then every
  `ModuleDef` tree named by `SELECT DISTINCT module_def_id FROM
  dynamic_node WHERE program_id = ?1`. `ProgramTrees::single(tree)` is the
  no-modules form every hand-built-tree unit test now uses.
- **Every activation and every diagnostic carries its module scope
  (D14).** `Activation`'s three fields became `Vec<ActiveRef>` /
  `Vec<ActiveRef>` / `Vec<ScopedDiagnostic>`, each qualified by an
  `Option<ModuleScope>` (`module_node`, `module_id`, `module_def_id`).
  `module_node` — the instantiating `Module` element's own `node_id` in
  the program's own tree — is what dedup actually keys on, because a
  `ModuleDef`'s local `ParameterRef`/`ComObjectRef` ids are reused
  verbatim by every sibling `Module` instantiating it, and
  `dynamic_node.node_id` collides across trees (it resets at each
  `Dynamic` root). The dedup key became `(Option<module_node>, ref_id)`
  (D18); without this, twelve `Module`s instantiating one `ModuleDef`
  would collapse into one set of results instead of twelve.
- **Exactly one level of expansion (D15).** A `Module` found while already
  inside a module scope is not followed: it produces
  `Diagnostic::NestedModuleNotExpanded` and its subtree is not descended.
  The corpus has zero nested modules and the Standard extraction defines
  no application-program-side `ModuleDef` complexType at all, so there is
  nothing to recurse against and no documented cycle rule to appeal to —
  one level plus a loud diagnostic is complete for everything the corpus
  contains and incapable of looping on anything it does not.
- **`Diagnostic::ModuleNotExpanded` is gone.** `ModuleDefNotFound {
  node_id, ref_id }` (no `@RefId`, or the named `ModuleDef` has no stored
  tree) and `NestedModuleNotExpanded { node_id, ref_id }` (D15) replace it;
  an empty-but-present `ModuleDef` tree is not a diagnostic (D17) — it
  legitimately activates nothing.
- **Corpus regression coverage**, over the four installed `.knxprod`
  archives: zero `ModuleDefNotFound`, zero `NestedModuleNotExpanded` —
  every `Module/@RefId` in the corpus resolves. For `prod3`'s three
  programs, activation totals grow from 22/18/14 (program tree only,
  slice 1's behaviour) to 382/258/134 (expanded), independently derived
  from the raw `ApplicationProgram` XML by a from-scratch Python
  reimplementation before a single Rust assertion was written, matching
  the real implementation's output on the first non-sabotaged run. A
  module-free control program (`prod1`,
  `M-000C_A-5703-10-085F`) is pinned unchanged at 145 activations,
  proving the slice is additive for everything that has no modules
  (AC#7). **Scope note, worth stating precisely:** RESEARCH.md §4.4 Q7
  lists seven module-bearing programs (`prod3`'s three, `kv25`'s four);
  only `prod3`'s three are reachable from these tests — `kv25` is a
  `.knxproj` demo project the corpus tests do not install, not one of the
  four `.knxprod` archives. Nothing above is a claim about `kv25`.
- **A finding worth recording honestly, not smoothing over:** `prod3`'s
  three programs hold 44/28/14 structural `Module` rows each, but only
  12/8/4 are actually walked by `evaluate` under the corpus's own default
  parameter values — the `Module`s naming `MD-2`/`MD-3`/`MD-4` sit on
  `choose` branches the defaults never select. This was independently
  verified twice (the Python reimplementation and the real
  `evaluate`/`--nocapture` output agree exactly). It is a fact about
  evaluation under the corpus's own default values, not a bug or a gap in
  the expansion itself — the plan's own Task 2 wording ("distinct
  `ModuleScope`s equals `Module` rows in the program's own tree") turned
  out to describe the structural count, not the reachable one; the tests
  assert the reachable count (12/8/4), which is what `evaluate` actually
  produces.
- **What this slice deliberately does not do, per the design's own scope
  cut (D16):** all instantiations of one `ModuleDef` still evaluate
  against **identical** parameter values. A `ModuleDef`'s
  `ParameterRef`/`ComObjectRef` ids — and therefore its `ValueMap` keys —
  are shared by every instantiating `Module`; genuinely per-instantiation
  values are a *project*-side construct (`ParameterInstanceRef`, the
  mangled `_M-<m>_MI-<k>_` id scheme) that `knx-productdb` does not model
  and that ADR-0014 keeps out of the import path entirely. This is a real,
  documented limitation carried forward, not an oversight — see
  [KNOWN_LIMITATIONS.md §3](KNOWN_LIMITATIONS.md#3-device-parameters-are-preserved-but-not-interpreted).
  Structured argument values (`NumericArg`/`TextArg`), memory-offset
  placement and text-template substitution, and project-side
  `ModuleInstance` resolution are all still out of scope, for the same
  reasons slice 1 left them out.

Two commits (`d140923` the evaluator change, `22d1099` the corpus
regression coverage), reviewed and passed after each. `cargo test
--workspace`: 817 passed / 0 failed / 3 ignored, up from 809 before this
slice (task 1: +6 net to 815 — 7 new unit tests minus the one deleted;
task 2: +2 to 817 — the two new corpus regression tests). This docs-only
pass (T18 slice 2's third task) reconciles
[KNOWN_LIMITATIONS.md §3/§12](KNOWN_LIMITATIONS.md),
[GAP_ANALYSIS_ETS.md](GAP_ANALYSIS_ETS.md), [DATA_MODEL.md §10](DATA_MODEL.md),
[ARCHITECTURE.md](ARCHITECTURE.md) and [ROADMAP.md](ROADMAP.md) with what
actually shipped, and retires every remaining `ModuleNotExpanded`
reference in `docs/` and `crates/` that stated the old behaviour as
current rather than as history. Closes no `GAP_ANALYSIS_ETS.md` item
outright — **A3** stays "partially closed": module expansion at the
application-program level is done, but the evaluator is still dead code
from every other crate's perspective, and **T18 slice 3** (a parameter
editor, the only thing that would give per-instantiation values in D16 a
real consumer) has not started. Never claimed here or anywhere else:
ETS behavioural parity, or KNX certification.
