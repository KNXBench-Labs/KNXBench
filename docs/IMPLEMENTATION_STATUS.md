# IMPLEMENTATION_STATUS.md

Last updated: 2026-09-14 (T15: ZipCrypto decryption for ETS4/ETS5 password-protected `.knxproj` projects — read-only, synthetic fixtures, AES still refused; see the end of this document)

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
| 5 | UI / UX | **Done** — cycle 1 (shell, projection, Project Explorer), cycle 2 (`knx-store` entity persistence, [design spec](superpowers/specs/2026-09-03-knx-entity-persistence-design.md)), cycle 3 (`knx-desktop` save/load wiring), cycle 4 (device selection, properties inspector, undo/redo, [design spec](superpowers/specs/2026-09-04-selection-inspector-design.md)), cycle 5 (`Ctrl+K` search across devices, group addresses, building parts, [design spec](superpowers/specs/2026-09-04-search-design.md)), cycle 6 (`Ctrl+Shift+P` command palette, [design spec](superpowers/specs/2026-09-04-command-palette-design.md)), cycle 7 (System/Light/Dark theme toggle, [design spec](superpowers/specs/2026-09-04-dark-light-mode-design.md)), cycle 8 (project status dashboard, [design spec](superpowers/specs/2026-09-04-dashboard-design.md)), cycle 9 (group address create/delete: a "Group Addresses" tree branch with inline create, a Delete button on the group-address inspector, duplicate-address and still-linked-on-delete validation in `knx-core`) — CLAUDE.md's full UI/UX deliverable list complete as of cycle 9 — and cycle 10 (a toast notification stack replacing the old persistent error banner, humor-wrapped error text, and a one-shot holiday/late-night startup toast, [design spec](superpowers/specs/2026-09-05-toast-easter-eggs-design.md)) and cycle 11 (user-customizable theme tokens — accent/background/surface/text — plus a three-level motion setting, layered on top of the cycle 7 theme toggle, via a new `ThemePanel.tsx`) and cycle 12 (device and communication-object descriptions are now editable, not just displayed: `Command::SetDeviceDescription` and `Command::SetComObjectDescription`/`RestoreComObjectDescription` clone the `SetIndividualAddress`/`SetComObjectDpt` command-layer pattern exactly, wired through `knx-store::command_sync`, `knx-server`'s `/api/device-description` and `/api/com-object-description` routes, and two new `Inspector.tsx` fields; `ComObjectNode` also gains `description`/`description_layer` so a communication object's description — modelled and persisted since Session 5 cycle 2 but never shown — is finally visible at all. An ETS feature audit done alongside this found no other silently-missing field: `GroupAddress`/`GroupRange`/`BuildingPart` genuinely carry no `Description` attribute in the one schema-11 project this project's evidence comes from — see [KNOWN_LIMITATIONS.md #1](KNOWN_LIMITATIONS.md#1-single-sample-bias), not a bug here) and cycle 13 (a named, selectable theme replacing cycle 7's System/Light/Dark cycle and cycle 11's four-token palette override outright — a complete visual package, not a per-user tweak layered on a light/dark base — with "Bitcoin DeFi" as the first theme and today's default, [design spec](superpowers/specs/2026-09-08-bitcoin-defi-theme-design.md)): `theme.ts` rewritten from a cycling function into a `ThemeDef`/`THEMES` registry (`loadThemeId`/`saveThemeId`/`useThemeId`); `palette.ts`, `palette.test.ts`, and `ThemePanel.tsx` deleted outright; `ThemeToggle.tsx` replaced by `ThemeSwitcher.tsx`, a `<select>` built against the registry (itself deleted 2026-09-12 by T27, its `<select>` moved into a new `SettingsPanel.tsx` — see the T27 entry below); `index.html` now always sets `data-theme` (Bitcoin DeFi is dark-only by design, no more "system"/unthemed state) and silently falls back cycle 7's old stored values to the new default; self-hosted `@fontsource` fonts and a Google Fonts `<link>` both load Space Grotesk/Inter/JetBrains Mono; and a full `styles.css` restyle — a 24-custom-property design-token layer plus component recipes app-wide (pill gradient/glow buttons, glass-morphism overlays, mono/gold technical text, gradient-text Dashboard heading, card hover-lift, fading grid-pattern background) done, see [ROADMAP.md](ROADMAP.md) |
| 6 | KNXnet/IP | Cycles 1-5 shipped (tunnelling, sending, discovery, routing, connection management/diagnostics). KNX IP Secure scoped, then shelved indefinitely (2026-09-06) — see [ROADMAP.md](ROADMAP.md), [KNOWN_LIMITATIONS.md §26](KNOWN_LIMITATIONS.md) |
| 7 | Integration & hardening | In progress (cycles 1-2) — see below |

**The repository is a buildable Cargo workspace with twelve crates** (eleven
library crates plus the dev-only `knx-testsupport`).
`knx-core` holds the full domain model of [DATA_MODEL.md](DATA_MODEL.md):
identity (`ids.rs`), the provenance types `Layer`/`Resolved<T>`/`Override<T>`
(`provenance.rs` — `Override<T>` added in Session 3, [ADR-0010](adr/0010-per-attribute-override-representation.md)),
typed addresses (`address.rs`), datapoint type references, values, and the
codec/resolution logic that reads and writes them (`dpt/` — a module
directory since Session 7/T29, see below), the
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
group_address_style` (the node grew `range`, `dpts` and `links` on
2026-09-13; see the UI-workbench section at the end of this document) — the reference project's 514 group addresses are
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
`toast.test.ts` (24 tests: `isLateNight`, `findHoliday`,
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
(`apps/knx-server/Dockerfile`) is a three-stage build — `node:22-alpine`
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
| `crates/knx-net/` | Empty crate with its responsibility stated in a doc comment. |
| `crates/knx-secure/` | Was an empty crate with its responsibility stated in a doc comment; gained its first code in A6 (2026-09-13, see the dated entry at the end of this document): the `.knxproj` ZIP-password derivation, `pbkdf2`/`sha2`/`base64` as its first real dependencies. Gained its second body of code in T15 (2026-09-14): `zipcrypto.rs`, PKWARE Traditional Encryption, read-only — `knx-etsproj` now depends on this crate for `Container::open_with_password`. Still holds no KNX Secure runtime-key handling (the bus-level protocol), which is what the crate's name is actually reserved for. |
| `apps/knx-cli/` | Headless entry point, binary `knx`. `import` subcommand (Session 3, `--product-db`/`--no-product-db` added Session 4) and `products` subcommand (Session 4); prints its version otherwise. |
| `apps/knx-server/` | **New, web/Docker deployment target.** The axum HTTP API binary (`knx-server`) and library (`knx_server`) — see the paragraph above. `src/domain.rs` holds `AppState` and the same `_impl` functions the old Tauri commands wrapped; `src/routes.rs`/`fs_routes.rs` are the axum route handlers; `src/errors.rs` maps `AppError` to an HTTP status plus a `{"error": ...}` body. `main.rs` reads `KNX_PORT`/`KNX_STATIC_DIR`/`KNX_DATA_DIR` from the environment. `Dockerfile` is the three-stage build (Node frontend, Rust backend, Debian-slim runtime); `scripts/smoke-test.sh` builds and runs the image, exercises `/healthz`, and proves native save/reopen over the mounted volume; `KNXBENCH_REFERENCE_PROJECT` additionally exercises ETS import. |
| `apps/knx-web/` | **New, moved from `apps/knx-desktop/src`.** The React + Vite frontend, now a standalone npm package consumed by both `knx-server`'s static-file serving and the Tauri desktop shell. `src/api.ts` is the `fetch()`-based client (replaces Tauri's `invoke()`); `src/FsPicker.tsx` is the mount-directory listing/upload UI shown when `window.__TAURI__` is absent (the server's `/api/project/download` route has no UI caller yet, see [KNOWN_LIMITATIONS.md #26](KNOWN_LIMITATIONS.md#26-apiprojectdownload-has-no-frontend-caller)); `src/filePicker.ts` picks between it and the native Tauri dialog. `src/theme.ts` is cycle 13's named-theme registry, replacing cycle 7's `theme.ts`/`ThemeToggle.tsx` cycle and cycle 11's now-deleted `palette.ts`/`ThemePanel.tsx` token overrides outright — see the Session 5 paragraph above. Its `<select>` picker was `src/ThemeSwitcher.tsx` until T27 (2026-09-12) moved the Theme select into a new `src/SettingsPanel.tsx` alongside two new motion settings and deleted `ThemeSwitcher.tsx` outright, its one consumer gone. Everything else (`ProjectExplorer`, `Inspector`, `Search.tsx`/`CommandPalette.tsx`, `Dashboard.tsx`, `Toast.tsx`, the `ts-rs`-generated bindings under `src/bindings/`) moved unchanged from `knx-desktop`. `vitest` suite: 89 tests across 8 files, including `api.test.ts` against a mocked `fetch` and cycle 13's rewritten `theme.test.ts` (`palette.test.ts` is gone with `palette.ts`). |
| `apps/knx-desktop/` | **Thin native wrapper as of the web/Docker deployment target** — see the paragraph above. `src-tauri/` is now just window/process wiring (`lib.rs`, ~80 lines): spawn `knx-server`'s router locally, point one `WebviewWindowBuilder` at it, keep the native file-dialog plugin available for `apps/knx-web`'s `window.__TAURI__` check. No `#[tauri::command]` handlers and no integration tests remain here — both moved to `apps/knx-server`. No `src/` of its own any more; it loads `apps/knx-web`'s build output (dev: Vite HMR on a fixed port; release: bundled as a Tauri resource). |
| `xtask/` | Repository verification tasks. `check-layering` walks the resolved dependency graph and reports the shortest path to any forbidden package, for eight roots (`knx-core`, `knx-etsproj`, `knx-productdb` — the third added Session 4 — `knx-projection`, the fourth, added Session 5, then `knx-csv`, `knx-report` and `knx-diff`, and `knx-secure`, the eighth, added by A6 on 2026-09-13 to keep that crate away from `knx-core` and `serde`); `check-headers` (T35) checks the shape of every first-line header that exists — one sentence, one period, at most 100 columns — and ratchets the count of files without one (`ABSENT_CEILING`, lowered as headers are added, never raised); `freeze-fixture` (Session 3) regenerates a canonical migration-test fixture. |
| `deny.toml` | Licence, advisory, ban and source policy for `cargo-deny`. |
| `.github/workflows/ci.yml` | CI: Tauri Linux prerequisites and Node.js setup (Session 5), formatting, clippy with `-D warnings`, tests, `knx-web`'s own `npm test` (Vitest, Session 5 cycle 5; path updated from `knx-desktop` to `knx-web` with the web/Docker deployment target), the layering gate, `cargo deny check`, and a check that `knx-projection`'s `ts-rs` bindings under `apps/knx-web/src/bindings` are not stale (Session 5; path likewise updated). A separate Docker artifact job runs `apps/knx-server/scripts/smoke-test.sh` on every push and pull request, building the shipped image and exercising health plus native persistence without private fixtures. |
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
cargo run -p xtask -- check-headers
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
  a CLI a container user might reasonably try. **Documented, 2026-09-13
  (backlog E5):** this stays a deployment constraint, not a fix — see the
  E5 entry below and [KNOWN_LIMITATIONS.md §79](KNOWN_LIMITATIONS.md#79-discovery-needs-ip-multicast-which-dockers-default-bridge-network-does-not-carry).

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
cycle — as of 2026-09-13 it carries `range: Option<u32>`, the id of the
range that contains it, which is enough for a UI to show range context but
is still a flat list rather than a nested one). `knx-projection` gains a small, additive `GroupRangeNode`
(id/name/start/end/parent) on `InstallationNode` — not the fuller nesting
redesign, just enough for an HTTP caller to discover a newly-created
range's id. `sync_after_command` gains no new incremental-sync paths for
any of the ten — same as every command since Session 5 cycle 2 that
hasn't gotten one yet, correct via a full `save_project`/`load_project`
round trip until a later cycle's incremental-sync pass covers all of
them together. 49 Rust tests added across
`crates/knx-core`/`crates/knx-projection`/`apps/knx-server`.

**Backlog E5 (2026-09-13) — Docker discovery documented, not coded
around.** `discover()` (`crates/knx-net/src/client.rs:146`) needs IP
multicast, which Docker's default bridge network does not carry — a
constraint carried since the web/Docker deployment target and Session 6
Cycle 3, never fixed because there is no honest fix: no multicast relay,
no unicast subnet sweep, was added, since neither would actually make
bridge-network discovery work, only look like it does. What this task
did establish, and verify against actual sources rather than assume:
`apps/knx-server`'s Dockerfile builds and ships only the `knx-server`
binary — `grep -rn discover apps/knx-server/src/` finds no discovery
route — so the documented `docker run` deployment path in README.md
cannot reach `discover()` at all today; the gap is reachable only by
running `knx-cli` inside some other container a developer builds. Docker's
own docs ([bridge](https://docs.docker.com/engine/network/drivers/bridge/),
[host](https://docs.docker.com/engine/network/drivers/host/)) confirm the
NAT/publish model that explains the failure and confirm `--network host`
is Linux-only and removes network-namespace isolation, but neither page
states in so many words that bridge networking blocks multicast — that
inference is recorded as an assumption, not asserted as documented fact
(KNOWN_LIMITATIONS.md §79 carries the `[D]`/`[A]` split). `discover()`
itself binds `0.0.0.0:0` and lets the OS routing table pick the outgoing
interface (`local_discovery_hpai`, `client.rs:706`), so `--network host`
is necessary and, on an ordinary single-NIC host, sufficient — a
multi-homed host with no default route to the KNX LAN would need that
fixed regardless of Docker, unverified either way. `apps/knx-cli`'s
`run_bus_discover_async` (`main.rs:1413`) now prints a fixed, unconditional
stderr hint alongside "no gateways responded" naming multicast and
container networking, so an empty result no longer looks identical to a
quiet network; one new test (`discover_empty_hint_names_multicast_and_container_networking`)
checks the hint text without a socket. `docker` was available in this
sandbox and was used, but not to build/run the shipped
`apps/knx-server` image — the compiled `knx` binary was run inside a
plain `debian:bookworm-slim` container (bridge) and again with
`--network host`, with `tcpdump` on the host's real LAN interface and on
`docker0` (KNOWN_LIMITATIONS.md §79 has the packet-level result: the
`SEARCH_REQUEST` reaches `docker0` and stops there in bridge mode, and
reaches the LAN interface in host mode, matching a bare-host run). No
real KNXnet/IP gateway answered on this network segment, so the
gateway-round-trip half of discovery remains unverified against
hardware. Docs touched: `README.md` (CLI section), `apps/knx-server/Dockerfile`
(comment at `ENTRYPOINT`), `docs/GAP_ANALYSIS_ETS.md` row E5,
`docs/ROADMAP.md`'s Session 6 "carried in" paragraph (forward-pointer,
not rewritten), and the two known-gaps bullets above.

All eight gates green: `cargo fmt --all --check` clean; `cargo clippy
--workspace --all-targets -- -D warnings` clean; `cargo run -p xtask --
check-layering` clean (no new crate dependency — the change is one
`const` and a stderr line in `apps/knx-cli`, already the layer that owns
CLI-facing prose); `cargo run -p xtask -- check-headers`: ceiling
unchanged (no new file); `cargo deny check` clean. Web gates
(`npm test -- --run`, `./node_modules/.bin/tsc --noEmit`) both run and
green though untouched, per the task brief's own requirement to prove
the pair stays green. Rust test counts and their delta against this
branch's `60e4027` baseline are in the branch's own commit message and
dispatch report rather than repeated here a third time.

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
already complete. The `AGPL-3.0-or-later` licence decision was finalized and
the canonical `LICENSE` file added on 2026-09-16. The deterministic large-project
performance benchmark and x86_64 AppImage were also delivered, so Session 7 is
complete as of 2026-09-17.

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
  headless evaluator over the stored tree in `knx-productdb`, slice 2
  (same day) taught it to expand a `Module` node into its `ModuleDef`'s
  own tree, and **slice 3 (also 2026-09-11) wired it into a real
  editor** — `GET`/`POST /api/device/{id}/parameters` plus
  `apps/knx-web`'s parameter panel — for top-level fields; module-scoped
  (per-channel) fields are read and displayed but not editable (D25). See
  the dated entries below and T18
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
- **Exactly one level of expansion (D15).** *(Superseded 2026-09-14: T18
  task 11, below, expands nested modules to a bounded depth and removes
  `NestedModuleNotExpanded` from the codebase entirely. D15 is described
  here exactly as slice 2 shipped it, for the record; D44/D45 in
  [docs/superpowers/specs/2026-09-11-module-expansion-design.md](superpowers/specs/2026-09-11-module-expansion-design.md)'s
  task-11 addendum are its successors.)* A `Module` found while already
  inside a module scope was not followed: it produced
  `Diagnostic::NestedModuleNotExpanded` and its subtree was not descended.
  The corpus has zero nested modules and the Standard extraction defines
  no application-program-side `ModuleDef` complexType at all, so there is
  nothing to recurse against and no documented cycle rule to appeal to —
  one level plus a loud diagnostic is complete for everything the corpus
  contains and incapable of looping on anything it does not.
- **`Diagnostic::ModuleNotExpanded` is gone.** `ModuleDefNotFound {
  node_id, ref_id }` (no `@RefId`, or the named `ModuleDef` has no stored
  tree) and `NestedModuleNotExpanded { node_id, ref_id }` (D15) replaced
  it — the second of those is itself gone as of task 11, which replaced it
  with `ModuleCycleDetected` and `ModuleNestingTooDeep`;
  an empty-but-present `ModuleDef` tree is not a diagnostic (D17) — it
  legitimately activates nothing.
- **Corpus regression coverage**, over the four installed `.knxprod`
  archives: zero `ModuleDefNotFound`, zero `NestedModuleNotExpanded` —
  every `Module/@RefId` in the corpus resolves. *(The second count is a
  slice-2 measurement of a diagnostic that no longer exists. Task 11
  re-measured the same question against all five archives and found zero
  nested modules, which is why the diagnostic could be retired rather
  than merely renamed.)* For `prod3`'s three
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
- **Task 11 fix round 1 (2026-09-14, goal-completion task 11), four
  changes to what shipped above:** (1) `evaluate::MAX_MODULE_EXPANSIONS =
  100_000` now caps total `Module` expansions per `evaluate` call, on top
  of the per-chain `MAX_MODULE_NESTING_DEPTH`; a non-cyclic fan-out tree
  that multiplies activations across many shallow chains — measured at
  4,194,304 activations / 8,170 MiB peak RSS for a `depth=12, fanout=4`,
  44-node probe — now stops loudly with
  `Diagnostic::ModuleExpansionBudgetExhausted` instead of exhausting
  memory. (2) `apps/knx-server`'s parameter-panel section grouping now
  keys sections on the same `Option<Vec<i64>>` ancestor chain
  `evaluate`'s own dedup uses (`ModuleScope::node_chain()`, made `pub`),
  not the flat `module_node` it kept using after D46 widened everything
  else — two distinct nesting chains reusing one `module_node` no longer
  collide into one section. (3) The corpus's total stored `Module` row
  count, cited above and in RESEARCH.md as 90, is **86** — the design
  doc's number was never actually run; the branch's own corpus test
  prints the right figure and now also asserts it. (4) That same test's
  the duplicate `Weinzierl_730_KNX_IP_Interface_ETS4_v1.knxprod` filename
  is no longer in the local corpus. Four archive files now represent four
  distinct packages; idempotent re-install remains covered by installing
  the same package bytes twice and asserting `InstallReport::skipped`.
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

**T18 slice 3, the parameter editor (2026-09-11).** The third and last of
the three planned slices, same day as slices 1 and 2
([design spec](superpowers/specs/2026-09-11-parameter-editor-design.md),
decisions D20-D26). Wires the now-complete evaluator (slice 2) into
something a user can see and write to. No schema change in either
database — the product database stays at **v3**, `knx-store`'s
`parameter_instance` table is unchanged.

- **The read model (`crates/knx-productdb/src/query.rs`).**
  `ParameterView` (`display_order: Option<i64>` — `ParameterRef/@DisplayOrder`
  is genuinely optional in shipped packages: all 543 `parameter_ref`
  rows for `prod3`'s program `M-0083_A-0317-31-7DC6` omit it, so `None` is
  the common case, not an edge case) and `parameter_ref_ids()`, following
  `com_object_view`'s own bulk-query idiom rather than one query per field
  (D22).
- **Decomposing a stored `ets_id` (D21).** A `ParameterInstance.source.
  ets_id` is not always a bare declared `ParameterRef` id — ETS encodes a
  `Module` instantiation *inside* the id string itself
  (`<Module/@Id>_MI-<k>_<declared suffix>`). Verbatim match is the common
  case (1343/1343 and 1390/1390 rows in the two Unser Zuhause demo
  projects); when it fails, a regex split is attempted and validated
  against the program's own declared `ModuleScope`/`parameter_ref` sets
  before being trusted. A stored value that decomposes to nothing the
  current program declares lands in the response's `stale` list, never
  silently dropped and never silently treated as live.
- **The write path.** `POST /api/device/{id}/parameters` validates
  `etsId`/`raw` against the program's declared `parameter_ref`/
  `parameter`/`parameter_type` chain, kind by kind, in
  `validate_kind_and_bounds` (`domain.rs`): `Number` an integer within
  its declared bounds, `Restriction` enum membership, `None` rejected
  outright regardless of `raw` (it carries no writable value),
  `Float`/`Text`/`IPAddress` checked against the Project Schema's own
  documented or corpus-observed encoding for that kind (finite number
  plus declared bounds; UTF-8 byte length against declared
  `SizeInBit`; IPv4-dotted or eight-group-hex IPv6 — see
  `validate_kind_and_bounds`'s own doc comment for each arm's
  evidence) — not merely a non-empty string, since T18 slice 5
  (2026-09-13/14). Those two bounds columns did not always reach
  every row, either: `crates/knx-productdb` moved from schema v8 to
  v9 in the same slice's fix round, `migrate_v8_to_v9` backfilling
  `min_inclusive`/`max_inclusive`/`size_in_bit` for `Float`/`Text`
  rows a pre-v9 ingest left `NULL` — see this file's own v9 entry
  below, alongside v8's — and `Picture`/`Raw` a
  non-empty-string-plus-XML-safety check, the two kinds that appear
  nowhere in the schema's encoding table at all. Before constructing
  exactly one
  `knx_core::Command::SetParameterValue`, undo/redo through
  `RestoreParameterValue` (`raw: Option<String>`, since
  `ParameterInstance.raw` is a plain `String`, not an `Override<T>` —
  D24 corrects an earlier draft of the design that assumed the
  `Override` shape applied here). A rejected write is 400 with
  `{"error": "..."}` and changes nothing. `crates/knx-store/src/
  command_sync.rs` treats both commands as a no-op on save, since
  `save_project` already deletes and re-inserts `parameter_instance`
  wholesale.
- **Response shape.** Both `GET` and `POST` return the same
  `ParameterPanelDto` — a successful write's response carries the
  evaluator's freshly recomputed activation set and diagnostics, so a
  client never needs a follow-up `GET` to see which other fields or
  communication objects just became active (D24). A device whose program
  does not resolve returns **200** with `programId: null`, empty
  `sections`, and its `stale` list intact — never a 404 that would hide
  otherwise-valid stale data.
- **The web panel (`apps/knx-web/src/ParameterPanel.tsx`).** Wired into
  `Inspector.tsx`; fetches unconditionally on device selection (a device
  has no program id of its own to gate on). Renders one section per
  `Module` instantiation (D23) plus the program's own top-level section;
  module-scoped fields render disabled with the caption "Shared across
  every instantiation of this module; read-only in this release." (T18
  slice 4 later made some module-scoped fields editable, so this exact
  wording no longer fit and was revised — see that slice's own entry
  below.) A rejected write reverts the input and shows the server's rejection
  message; diagnostics render as a collapsed, expandable banner (D26),
  never the evaluator's raw `Debug` output.
- **Module-scoped values: read correctly, not written (D25).** An earlier
  draft of this design assumed `ParameterInstance` had nowhere to store a
  per-channel value — false, and corrected during design: ETS already
  writes a scope-qualified `ets_id`, and the KV v2.5 demo project's own
  `ParameterInstance` table stores 5 distinct values (17, 33, 49, 32, 48)
  for one declared `ParameterRef` across 5 `Module` instantiations, today,
  unmodified by this slice. Slice 3's decomposition (D21) reads and
  displays each correctly, per channel (D22/D23). What blocks a *write* is
  the evaluator, not storage: `evaluate`'s `ValueMap` is a flat
  `HashMap<String, String>` (`evaluate.rs:348`, one slot per declared id,
  project-wide) — a module-scoped value can never reach it in this
  slice's design, so accepting a module-scoped write would silently break
  D24's own "no second `GET` needed" guarantee. `editable: false`,
  checked server-side too. D16 (all instantiations of one `ModuleDef`
  evaluate against identical parameter values) stays true in this
  narrower sense: the *evaluated activation* is still identical across
  instantiations; only the *displayed value*, which D21/D22 now source
  per channel, differs. This is the deliberate boundary that keeps T18
  slice 3 one branch instead of the full editor — named in the design's
  Non-goals, not silently left out. **Closed by T18 slice 4 (2026-09-12,
  see that slice's own entry below):** a module-scoped value now reaches
  `ValueMap`, so D16 no longer holds unconditionally, only when the
  channels' own stored values agree or none exist.
- **What this slice deliberately does not do**, cross-referencing the
  design's own Non-goals list rather than re-deriving it: per-channel
  (`Module`-instantiation) value *editing* (needs a scope-aware
  `ValueMap` and a validated write path for a module-scoped `etsId`,
  including what `MI` means above `1`, unattested in the corpus); deep
  format validation for `Float`/`Text`/`IPAddress`/`Picture`/`Raw` beyond
  a non-empty-string check *(Superseded for `Float`/`Text`/`IPAddress`: T18
  slice 5 (2026-09-13/14) adds real format validation for those three
  kinds — see this file's "The write path" bullet above and
  `domain.rs`'s `validate_kind_and_bounds` for what each arm actually
  checks. `Picture`/`Raw` stay a non-empty-string-plus-XML-safety
  check — neither kind appears anywhere in the Project Schema's own
  encoding table, so there is no format to validate against. Left
  here, not deleted, for the record of what this slice's design
  originally scoped out.)*; `Access` used for write gating (display-only,
  RESEARCH §4.3 found no usable correlation); diagnostics gating a write;
  `Argument` values; union-parameter cross-field validation; bulk/
  multi-field write; pagination; product-database editing; search/filter
  UI; commissioning.

Four tasks (product-database read model, `Command` pair, HTTP endpoints,
web panel), each reviewed before the next started, plus one fix round on
the last task (two pinning tests, no production change — see
`.ai/CURRENT_STATE.md`). `cargo test --workspace`: **975 passed / 0
failed / 3 ignored**, up from 951 before this slice. `npm run test` in
`apps/knx-web`: **184 passed across 18 files**, up from 179 across 17
before this slice. Closes no `GAP_ANALYSIS_ETS.md` item outright — **A3**
moves from "partially closed" to a still-partial but stronger statement:
a UI now exists and can write a top-level value; module-scoped editing
(D25) is the named remainder. This docs-only pass (T18 slice 3's fifth
task) reconciles [KNOWN_LIMITATIONS.md §3/§12](KNOWN_LIMITATIONS.md),
[GAP_ANALYSIS_ETS.md](GAP_ANALYSIS_ETS.md), [DATA_MODEL.md §10](DATA_MODEL.md),
[ARCHITECTURE.md](ARCHITECTURE.md) and [RESEARCH.md §4.4](RESEARCH.md)
with what actually shipped. Never claimed here or anywhere else: ETS
behavioural parity, or KNX certification.

**T29, DPT codec (2026-09-11), branch `t29-dpt-codec`.** KNXBench's first
Datapoint Type codec ([design spec](superpowers/specs/2026-09-11-dpt-codec-design.md),
decisions E4-D1 through E4-D9; [plan](superpowers/plans/2026-09-11-dpt-codec.md);
[ADR-0016](adr/0016-dpt-codec-in-knx-core.md)). `crates/knx-core/src/dpt.rs`
became a module directory (`dpt/mod.rs`, `dpt/codec.rs`, `dpt/resolve.rs`);
`GroupValue` moved down out of `knx-net` into `knx-core`, re-exported so no
existing call site changed.

- **`decode`/`encode` (`dpt/codec.rs`)** cover main types 1, 2, 3, 5, 6
  (except `6.020 DPT_Status_Mode3`, whose `B5N3` layout has no matching
  `DptValue` shape), 7, 8, 9, 12, 13, 14, 16, 17, 18 — fourteen of the 46
  main types `knx_master.xml` defines, **as this branch stood on
  2026-09-11.** (The E4 entry below adds five more two days later, on
  2026-09-13, bringing the codec to nineteen — see
  `KNOWN_LIMITATIONS.md` §61, whose heading count is the current total,
  not this one.) Two sentinel collisions the
  Standard itself does not resolve were settled here rather than left
  ambiguous: `8.010`'s printed 327.67% maximum collides with its own
  invalid-data code (practical maximum 327.66%), and main type 9's
  arithmetic maximum at `M=2047,E=15` collides with the same reserved code
  (usable maximum 670433.28, matching DPT-AS's own printed figure over
  AN188 §4's inconsistent 670760.96). Scene numbers (main types 17, 18)
  are carried at wire value with no display offset applied, despite DPT-AS
  §3.19 NOTE 9 recommending one for 18.001 — that is a UI-layer decision,
  not this codec's.
- **`resolve_group_address_dpt`/`resolve_project_group_address_dpts`
  (`dpt/resolve.rs`)** infer a group address's DPT from its linked
  communication objects' stated types, classifying into `None` / `Single` /
  `Conflict`; a conflict is reported, never resolved to a guess (RESEARCH
  §6.1 rule 3).
- **A latent encoding bug surfaced and was fixed as part of the
  `GroupValue` move**: `knx_net::cemi::encode_group_value` used to let a
  `GroupValue::Short(v)` above the six-bit range overwrite two APCI
  service-selector bits sharing its octet; it now promotes such a value to
  `Bytes([v])` instead. See ADR-0016 for the full account.
- **`apps/knx-cli`**: `bus monitor --project <path>` decodes each telegram's
  value against the resolved DPT; `bus write --dpt <DPST-m-s>` (or a DPT
  resolved from `--project`) encodes a human-typed value instead of
  requiring the caller to already know the raw wire encoding.
- **What this slice deliberately does not do:** no GUI (`T15` builds on
  this codec, not the other way round); no `knx_master.xml` DPT catalogue
  consultation, so no enumeration wording and no units beyond what a
  scaled subtype's own arithmetic already implies; no `GroupAddress/@DatapointType`
  reading for schema ≥ 21 projects (preserved, not modelled); no hardware
  verification — every test checks the codec against the Standard's own
  stated encodings, not a real device's actual telegrams. Full accounting:
  [KNOWN_LIMITATIONS.md §61](KNOWN_LIMITATIONS.md#61-the-dpt-codec-covers-thirty-main-types-infers-rather-than-reads-its-input-and-leaves-several-encoding-questions-to-a-stated-ruling-rather-than-the-standard).

`cargo test --workspace`: **920 passed, 0 failed, 3 ignored** (baseline
before this cycle was 817/0/3, per the branch's Task 1 starting point; the
four implementation tasks account for the entire +103 net — Task 1 +43,
Task 2 +40, Task 3 +14, Task 4 +6 — independently re-run for this docs
pass rather than taken on trust; no regressions). A user can now run `knx
bus monitor
--project <path>` and see `On`/`Off`/a percentage/a temperature instead of
a raw hex payload, and `knx bus write --dpt DPST-9-1 23.5` instead of
having to hand-encode an F16 payload themselves — for the fourteen main
types this slice (T29) covered before E4 added five more two days
later. Closes `GAP_ANALYSIS_ETS.md` row **E4**
partially; **D5** (the GUI itself) is still open.

**E4, DPT codec main types 4/10/11/15/19 (2026-09-13), branch
`e4-dpt-main-types`.** Extends T29's codec with five more main types, each
read from DPT-AS directly rather than from a summary table:

- **Main type 4** (`A8`, character, DPT-AS §3.4) reuses main type 16's
  `char_set_is_ascii` charset switch instead of a second implementation;
  `4.001` (ASCII) and `4.002` (ISO-8859-1) both occupy a full octet —
  `require_bytes::<1>`, not `require_short` (its 6-bit inline threshold
  cannot hold an 8-bit field).
- **Main type 10** (`10.001`, time of day + day of week, DPT-AS §3.11,
  page 41) represents day-of-week `0` as "no day" (`Option::None`) — §3.11's
  own Day column prints `0 = no day` directly (range `[0...7]`); only the
  `Option<u8>` storage shape, not the `0 = no day` fact, is this codec's own
  choice, with an exact round trip.
- **Main type 11** (`11.001`, date, DPT-AS §3.12) resolves the two-digit
  year by the century window DPT-AS §3.12 EXAMPLE 5 states directly
  (raw `>= 90` → `1900 +` raw, else `2000 +` raw).
- **Main type 15** (`15.*`, access data, DPT-AS §3.16) packs six BCD
  digits plus error/accepted/direction/encrypted flags and a 4-bit index
  across four octets with no reserved bits in this format at all; a BCD
  nibble above 9 is rejected.
- **Main type 19** (`19.001`, date and time, DPT-AS §3.20) decodes all
  sixteen bits this codec can give meaning to (year, month, day, weekday,
  hour, minute, second, and eight status flags) into
  `DptValue::DateTime` — nothing is silently dropped. One genuine Standard
  inconsistency surfaced and is documented rather than guessed around:
  octet 1's own diagram contradicts itself — the field-*names* row gives a
  bit called `SRC` position 6, but the bit-*encoding* row directly beneath
  it marks that same bit `r` (reserved), and Note 15 sides with the
  encoding row; confirmed against the source PDF page directly. This codec
  follows the encoding row and Note 15 (a ruling between two contradictory
  rows, not an absent bit), so `DptValue` has no `src` field. Month/Day and
  Hour/Minute/Second range checks are enforced
  only when the matching invalid-flag says the field is valid, per
  Note 11's Hour=24 rule and the section comment in `codec.rs`.

All five follow the existing reserved-bit policy (nonzero reserved bit →
`InvalidData`) and the existing module's `decode_*`/`encode_*`/test
structure and naming. `cargo test -p knx-core --lib dpt::codec`: **121
passed, 0 failed** (up from the pre-E4 baseline on this branch). Full
per-type judgment-call accounting:
[KNOWN_LIMITATIONS.md §61](KNOWN_LIMITATIONS.md#61-the-dpt-codec-covers-thirty-main-types-infers-rather-than-reads-its-input-and-leaves-several-encoding-questions-to-a-stated-ruling-rather-than-the-standard).
Closed `GAP_ANALYSIS_ETS.md` row **E4** further; main types 20-30 and
`6.020` followed in a second round the next day (entry below), leaving
`knx_master.xml` catalogue consultation for units and enumeration wording
as the row's remaining open item. Out of scope by design: `apps/knx-web` and
`apps/knx-server` need no change, since neither pattern-matches on
`DptValue`'s variants directly — confirmed by grep before closing this
task, not assumed.

**T15, Group Monitor GUI (2026-09-11), branch `t15-group-monitor`.** Builds
on T29's codec ([design spec](superpowers/specs/2026-09-11-group-monitor-design.md);
[ADR-0017](adr/0017-knx-server-depends-on-knx-net.md)). Gives
`apps/knx-server` and `apps/knx-web` what T29 gave the CLI: a live,
DPT-decoded telegram table and a send-from-the-table form, over a
tunnelled KNXnet/IP connection.

- **`apps/knx-server` gains a direct `knx-net` dependency** (ADR-0017,
  first Infrastructure-crate-to-Infrastructure-crate edge of its kind for
  this crate) and a `GatewayConnector`/`BusTunnel` seam
  (`apps/knx-server/src/bus.rs`) narrower than `knx-net`'s own
  `BusConnection` — exactly the two operations (open a tunnel, use it) a
  monitor session needs. `RealConnector`/`RealTunnel` wrap
  `knx_net::KnxNetIpClient`/`TunnelClient` in production; a `fake` module
  (`FakeConnector`/`FakeTunnel`/`FakeTunnelHandle`) is `pub`, not
  `#[cfg(test)]` (integration tests compile as a separate crate and can't
  see `cfg(test)` items in the lib), and is what every test in this slice
  drives instead of a socket.
- **`BusSession`** (`apps/knx-server/src/bus.rs`) owns one open tunnel, a
  capped `TelegramBuffer` (`MAX_TELEGRAMS = 5000`, a monotonic `seq`, and
  a `dropped_before` counter advanced by both ring-buffer eviction *and*
  `RecvError::Lagged(n)` — a lagged receiver's missed telegrams are
  accounted exactly like an evicted one, never silently), and a
  `tokio::spawn`ed drain task mirroring `apps/knx-cli`'s own
  `run_bus_monitor_async` `tokio::select!` shape. `AppState.bus_session:
  tokio::sync::Mutex<Option<BusSession>>` (widened from `std::sync::Mutex`
  by Task 3, since `/write` must hold the guard across an `.await`) holds
  at most one session; a second `start` while one is active is `409`,
  naming the existing session, never a silent second gateway connection.
  A gateway-side close (`TunnelEvent::Closed`/`RecvError::Closed`) pushes
  a synthetic `"SessionClosed"` marker row, flips status to `closed`, and
  leaves the buffer readable until an explicit `/stop` — exactly one code
  path ever clears `bus_session`, avoiding a race between the drain task
  exiting on its own and a client-initiated stop.
- **Four routes**, a new sibling module `apps/knx-server/src/
  bus_routes.rs` (not a 1500-line addition to `routes.rs`):
  `POST /api/bus/monitor/start` (`{gateway}` → `{sessionId,
  assignedAddress}`, `400`/`409`/`502`), `POST /api/bus/monitor/stop`
  (`{sessionId, telegramCount, droppedCount, warning?}`, `409`),
  `GET /api/bus/monitor/telegrams?since=<seq>` (`{sessionId, status,
  nextSince, droppedBefore, telegrams[]}`, `404` with no session, never
  `409` — a `GET` doesn't mutate), `POST /api/bus/write`
  (`{destination, dpt?, value}` → `{encodedPayload, service}`,
  `400`/`409`/`502`). `errors.rs` gained a documented `502` category
  ("far-end failure" — the gateway refused, not this server, not the
  caller). `/write` parses `destination` in the open session's project's
  own `GroupAddressStyle` — fixed mid-branch (commit `b540264`) after
  being found hardcoding `ThreeLevel` regardless of project, the same bug
  `apps/knx-cli` still has and this branch deliberately left there (see
  `KNOWN_LIMITATIONS.md` §29's 2026-09-11 update and §62 item 13).
- **`apps/knx-web`**: `BusMonitorPanel.tsx` (session connect/disconnect,
  a 1-second poll of `/telegrams`, a client-side text + service-type
  filter, and a gap notice — `role="alert"`, never hidden by a filter —
  whenever `droppedBefore > 0`) and `BusComposeForm.tsx` (a separate
  component, prefilled by clicking a row, resolving DPT the way
  `resolve_write_value --project` does: explicit DPT wins, else the
  row's cached resolution, `None`/`Conflict` rejected client-side before
  any request with a verbatim message matching the server's own 400 text
  word-for-word since a later fix). All bus DTOs are hand-written TS
  interfaces in `api.ts` (no `ts-rs` binding — these are `knx-server`-local
  types, following the existing `LogEntry` precedent), including a
  `warning?: string` field on the stop response the design document's own
  prose never mentioned.
- **Divergences from the design document, found while building it, kept
  as the branch's own decision:** the wire's `dpt` field carries
  `DptRef`'s `Display` form (`"DPST-1-1"`), not the design's worked
  example's dotted `"1.001"` — `DptRef::parse` never accepted the dotted
  form, and `/write` has to accept back exactly what `/telegrams` sends;
  the stop response's `warning` field, absent from the design's prose,
  exists because the drain task's `JoinHandle` can report a panic and
  CLAUDE.md forbids swallowing that; and `service` is a plain `string` on
  the wire, not a closed 4-way union, because the synthetic
  `"SessionClosed"` marker is a real, intentional 5th value a narrow
  union would have had to either lie about or invent a category for.
- **What this slice deliberately does not do**, and three more limitations
  found during this cycle's review: full accounting in
  [KNOWN_LIMITATIONS.md §62](KNOWN_LIMITATIONS.md#62-the-group-monitor-gui-t15-is-tunnelling-only-single-session-client-filtered-and-only-its-passive-receive-path-has-real-gateway-evidence).
  In short — no routing, no auto-reconnect, no live DPT re-resolution
  mid-session, one session at a time, no server-side filtering, an uncapped
  browser-side row list, a `/write` round-trip test covering `Free`/`TwoLevel`
  but not a full round trip for `ThreeLevel`, and the CLI's own copy of the
  `GroupAddressStyle` bug left unfixed. The passive real-gateway receive path
  is now verified once; real transmit behavior remains unverified.

`cargo test --workspace`: **951 passed, 0 failed, 3 ignored** (re-run in
this worktree for this docs pass, not taken from any task's own report;
baseline before this branch, on `main` at `b88b286`, was 920/0/3).
`cargo test -p knx-server`: **147 passed, 0 failed, 0 ignored**. `npm run
test` (`apps/knx-web`, `vitest run`): **179 passed across 17 files**. A
user with an open project and a reachable gateway can now watch a live,
decoded telegram table and send a group value, from the web/desktop UI,
without a terminal — for one tunnelled gateway at a time, filtered only
by what the browser already has. T15 itself had fake-tunnel evidence only;
Task 16 below later verified the passive real-gateway receive path. Closes
`GAP_ANALYSIS_ETS.md` row **D5** for tunnelling;
finishes **E4**'s display side for tunnelling.

**Task 16, passive Group Monitor real-gateway re-verification
(2026-09-16).** A dedicated server exercised the production
`POST /api/bus/monitor/start`/poll/stop path against one physical gateway. The
first 133-second session used the empty project and received 52 telegrams with
zero drops. The second 107-second session opened the real schema-23 `Unser
Zuhause` project and received 65 telegrams from 9 sources to 21 group
destinations: all 65 names resolved, 10 values decoded, no conflict or decode
error was observed, and zero telegrams were dropped. Both tunnels remained
active until explicit successful stop. This was strictly passive: no group
read/write/response, management request, scan, or `/api/bus/write` call was
made. Gateway and bus addresses are deliberately omitted. This verifies one
gateway model's tunnelling receive and project-resolution path; routing,
transmit behavior, reconnect, other gateway models, and long sessions remain
unverified. Full accounting: [KNOWN_LIMITATIONS.md §62](KNOWN_LIMITATIONS.md#62-the-group-monitor-gui-t15-is-tunnelling-only-single-session-client-filtered-and-only-its-passive-receive-path-has-real-gateway-evidence).

**T27, in-app motion control (2026-09-12), branch `t27-motion-control`.**
Restores the user-facing motion setting cycle 13's theme rewrite deleted
by accident ([design spec](superpowers/specs/2026-09-12-motion-control-design.md),
decisions D27-D34; [plan](superpowers/plans/2026-09-12-motion-control.md)).
Closes `GAP_ANALYSIS_ETS.md` gap **D11**; partially addresses **D8**.

- **`apps/knx-web/src/motion.ts`** (new, modeled on `theme.ts`'s
  `Pick<Storage, ...>`-injection style): two independent registries,
  `MOTION_LEVELS` (`off`/`subtle`/`standard`, default `standard`) and
  `MOTION_STYLES` (`apple`/`glitch`, displayed "Smooth"/"Glitch", default
  `apple`), each with its own `localStorage` key
  (`knx-desktop:motion-level`, `knx-desktop:motion-style`) and its own
  always-safe-default fallback for a missing/empty/unknown stored value.
  `useMotion()` returns `{level, setLevel, style, setStyle}` and applies
  both as `data-motion-level`/`data-motion-style` on `<html>` via two
  independent effects. No `window.matchMedia` call anywhere in the file —
  deliberate, documented in a comment, so `prefers-reduced-motion` stays
  enforced only in CSS, never overridable from TypeScript.
- **`apps/knx-web/src/styles.css`**: a token layer —
  `--knx-transition-duration` (`0ms`/`120ms`/`250ms` per level) and
  `--knx-motion-easing` (a cubic-bezier ease for `apple`, `steps(4, end)`
  for `glitch`) — added to the existing `:root[data-theme="bitcoin-defi"]`
  block, selected by `:root[data-motion-level="…"]`/
  `:root[data-motion-style="…"]` attribute selectors.
- **`apps/knx-web/index.html`**: the pre-mount bootstrap script now also
  applies both `data-motion-*` attributes before React mounts, avoiding a
  flash of default motion. It hard-codes the same id lists as
  `motion.ts` (it cannot import a module at that point) — cross-commented
  in both files as a duplication to keep in sync by hand.
- **`apps/knx-web/src/motionGuard.test.ts`** (new): a brace-counting
  checker that reads `styles.css` off disk with `node:fs` and fails if
  any `transition:`/`animation:` declaration sits outside a
  `@media (prefers-reduced-motion: no-preference)` block or uses a
  literal duration instead of `var(--knx-transition-duration)`. Verified
  end-to-end by injecting a literal `200ms ease` into `styles.css` and
  watching the guard name the offending line, then restoring the file.
  Mid-slice fix (`056b4a0`): the `node:fs`/`node:url`/`node:path` imports
  type-checked under Vitest but failed `npm run build` (`tsc && vite
  build`, `include: ["src"]`, no `@types/node`) with TS2591 — caught
  because this task ran the production `tsc` as a seventh gate, which
  `npm run test` alone would not have exercised. Fixed with a 27-line
  `apps/knx-web/src/node-builtins.d.ts` declaring exactly the four
  functions used, rather than adding `@types/node`. The tidier-looking
  alternative, Vite's `import css from "./styles.css?raw"`, was tried and
  rejected: Vitest doesn't process CSS, so it resolves to the empty
  string and the guard passes against nothing — confirmed by injecting
  the same literal `200ms` and watching the suite stay green.
- **`apps/knx-web/src/SettingsPanel.tsx`** (new) + `App.tsx` wiring: a
  gear-button overlay panel reusing `Search.tsx`/`CommandPalette.tsx`'s
  `.search-overlay`/`.search-panel` shape and click-outside pattern
  verbatim, with three labelled `<select>`s (Theme, Motion style, Motion
  level), each applying immediately. `apps/knx-web/src/ThemeSwitcher.tsx`
  — its one consumer now this panel — is **deleted**; the toolbar's
  always-visible theme `<select>` is gone with it (one settings entry
  point, not two, especially since `THEMES` currently has exactly one
  entry). Escape closes the panel via a `window` keydown listener
  (`useEffect`), since three `<select>`s have no single natural field to
  hang Escape off of. Labels are visible `<span>`s plus `aria-label`s,
  not `.sr-only` — a deliberate reading of the brief's explicit "visible
  label" instruction over `ThemeSwitcher.tsx`'s original sr-only pattern.
  Coordinator follow-up (`0604180`): the now-orphaned `.theme-switcher
  select` CSS rule was removed, with a comment recording that `.sr-only`
  is kept despite also losing its only consumer, since it is the standard
  visually-hidden-label utility and the accessibility work
  `KNOWN_LIMITATIONS.md` §20 still owes will want it.
- **`apps/knx-web/src/BusMonitorPanel.tsx`** (T15 retrofit, design D34):
  a `newRowThreshold` state tracks the lowest `seq` from the most
  recently completed incremental poll; rows at or above it get a
  `bus-monitor-row-new` class in addition to the existing
  `bus-monitor-row-marker`. The threshold resets on *every* poll tick,
  including one that returns zero telegrams, so the highlight lasts
  exactly one poll interval and never accumulates — no timer, no
  per-row React state. `styles.css` pairs it with a new
  `@keyframes bus-monitor-row-new-highlight` (`background-color`
  fading from `--knx-overlay-shadow` to transparent) inside the same
  `no-preference` block, as a compliant `animation:` shorthand the guard
  above accepts without complaint. The mount-time reattach effect (a
  bulk-adopted backlog on connect) never sets the threshold, so nothing
  is marked new on first load.

`SettingsPanel` reuses the overlay CSS `Search.tsx`/`CommandPalette.tsx`
already shared — but it was not the third consumer of it:
`CatalogBrowser.tsx` (T2) already was, unrelated to this slice, so
`SettingsPanel` is a fourth. `GAP_ANALYSIS_ETS.md` row **D9** is updated
to say so; it stays open and is, if anything, slightly worse, since there
is now one more call site sharing CSS with no shared component behind it.

`cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D
warnings`, `cargo run -p xtask -- check-layering`, and `cargo deny check`
all clean, unchanged by a frontend-only slice. `cargo test --workspace`:
**975 passed / 0 failed / 3 ignored** across 72 `test result` lines,
identical to the branch's baseline — no Rust file touched anywhere in
this slice. `npm run test` (`apps/knx-web`, `vitest run`): **215 passed
across 21 files** (up from 184/18 before this slice: task 1 added 16
tests in a new `motion.test.ts`, task 2 added 5 in a new
`motionGuard.test.ts`, task 3 added 6 in a new `SettingsPanel.test.tsx`,
task 4 added 4 to `BusMonitorPanel.test.tsx`). A seventh gate, added
mid-slice after the `node:fs`/production-build break: `npx tsc -p
apps/knx-web/tsconfig.json --noEmit` — clean.

No claim of ETS parity or KNX certification is made anywhere in this
slice — ETS has no comparable motion control, so there is no parity
claim to make in either direction.

**T31, a shared modal overlay shell (2026-09-12), branch
`t31-overlay-shell`.** Four components — `Search.tsx`,
`CommandPalette.tsx`, `CatalogBrowser.tsx`, `SettingsPanel.tsx` — had
each hand-rolled `styles.css`'s `.search-overlay`/`.search-panel` shape,
and `KNOWN_LIMITATIONS.md` §20's "lifted when a third overlay is added"
trigger had already fired twice, unnoticed, before this slice ([design
spec](superpowers/specs/2026-09-12-modal-overlay-shell-design.md);
[plan](superpowers/plans/2026-09-12-modal-overlay-shell.md)). Closes
`GAP_ANALYSIS_ETS.md` gap **D9**.

- **`apps/knx-web/src/Overlay.tsx`** (new, 94 lines): the one component
  behind `.search-overlay`/`.search-panel` from here on. Renders
  `div.search-overlay > div.search-panel[role="dialog"][aria-modal="true"][tabIndex=-1]`;
  closes on backdrop click (`onClick` + `stopPropagation` on the panel,
  as every hand-rolled copy already did) and on `Escape` via a `keydown`
  listener on the panel itself — not `window`, not an input, so the key
  works no matter which control inside the dialog has focus and stops
  working the instant the dialog unmounts. On mount it focuses
  `initialFocusRef.current` if the caller named one, else the first
  focusable descendant (`FOCUSABLE_SELECTOR`, exported), else the panel
  itself. The explicit ref exists because `CatalogBrowser.tsx`'s first
  focusable descendant is the "Install product database" file input, not
  its search field — "first focusable" alone would have silently
  misdirected that dialog's opening focus. `Tab`/`Shift+Tab` cycle
  within the panel's focusable descendants (a hand-written trap, no
  library), and focus returns to whatever had it before the dialog
  opened, on unmount.
- **All four consumers migrated**: `Search.tsx` (`label="Search"`),
  `CommandPalette.tsx` (`label="Command palette"`), `CatalogBrowser.tsx`
  (`label="Device catalog"`), `SettingsPanel.tsx`
  (`labelledBy="settings-panel-title"`, `className="settings-panel"`
  preserving its distinct panel width). Each dropped its own overlay/panel
  divs, its own `Escape` handling and its `autoFocus`; `SettingsPanel.tsx`'s
  `window` `keydown` listener — the odd one out, since three `<select>`s
  gave it no single field to hang `Escape` off of — is deleted outright
  now that the shell provides it structurally.
- **Listbox semantics, applied uniformly.** In all three list-bearing
  overlays the text input becomes `role="combobox"` with
  `aria-haspopup="listbox"` and
  `aria-expanded`/`aria-controls`/`aria-activedescendant`; the `<ul>`
  becomes `role="listbox"`; each row becomes `role="option"` with
  `aria-selected` and a stable id — so `CommandPalette.tsx`'s
  `aria-disabled="true"` now sits on a row with a role to qualify it,
  and a screen reader has something to announce for the highlighted row
  in every one of the three lists, not none of them.
  `Search.tsx`'s kind-grouped `<li className="search-group">` wrappers
  cannot sit inside a `role="listbox"` as bare items, so each group
  became `role="group"`/`aria-label`, its inner `<ul>` dropped to
  `role="presentation"`, and the visible `.search-group-label` gained
  `aria-hidden="true"` since the group's `aria-label` already announces
  the same text.
- **`CatalogBrowser.tsx` gains the keyboard path it never had** — the
  one change in this slice that is a correctness fix, not a
  maintainability one. Its result rows were `<li onClick>` with no
  `tabIndex`, no key handler and no role: a keyboard-only user could not
  create a device from the catalog at all. `ArrowDown`/`ArrowUp` on the
  search input now move a `highlight` index (stopping, not wrapping, at
  the ends, matching `Search.tsx`), and `Enter` **picks** the highlighted
  item — selects it and pre-fills the device-name field, exactly what
  clicking the row already did. It does not create the device; creation
  stays behind the name field's own `Enter` and the Create button, since
  the name field only renders once an item is selected. Rows still carry
  no `tabIndex` on purpose: focus stays on the input, which drives the
  list through `aria-activedescendant`, the same combobox pattern
  `Search.tsx`/`CommandPalette.tsx` already use — a roving tabindex
  alongside it would be two competing keyboard models in one widget.
- **`apps/knx-web/src/overlayShell.test.ts`** (new): reads every
  non-test `.tsx` file under `apps/knx-web/src` with `node:fs` and fails
  the suite, naming the offender, if anything other than `Overlay.tsx`
  contains the literal `search-overlay` — modeled on `motionGuard.test.ts`,
  for the same reason: `KNOWN_LIMITATIONS.md` §20's lift trigger was
  prose, prose does not fail a build, and it was missed for two
  consumers running. Verified by injecting `search-overlay` into a
  second file and watching the guard name it, then reverting.

`styles.css` was not touched at all in this slice — no visual redesign,
per the design spec's non-goals. No npm dependency was added; the trap,
the `Escape` handling and the focus-restore logic are hand-written, as
the spec's "no `<dialog>` element, no focus-trap library" non-goal
required.

What this slice deliberately does not do, because the design spec ruled
it out of scope rather than missing it: no scroll-into-view for a
highlight moved off-panel by arrow keys; no `inert`/`aria-hidden` on
background content, so a screen reader's browse mode can still reach it
past the focus trap; no focus-visible styling pass; and no verification
against a real screen reader anywhere in this slice — the new and
extended tests run under jsdom, which asserts that focus moves, the trap
cycles and ARIA attributes point at the right elements, not what an
actual screen reader announces. `KNOWN_LIMITATIONS.md` §20 is rewritten
to state exactly this rather than closed outright.

`cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D
warnings`, `cargo run -p xtask -- check-layering`, and `cargo deny check`
are unchanged by this slice — no Rust file was touched anywhere in it.
`npm run test` (`apps/knx-web`, `vitest run`): **235 passed across 25
files** (up from 215/21 at the branch point, `ea54b0c`) — 11 files
changed under `apps/knx-web/src`, 875 insertions/225 deletions across
four code commits (`bdba7cb`, `a0c6dbe`, `8d6220d`, `3eff155`). `npx tsc
-p apps/knx-web/tsconfig.json --noEmit`: clean.

No claim of ETS parity or accessibility-standard conformance is made
anywhere in this slice — ETS has no directly comparable overlay
accessibility audit to compare against, and no WCAG or other audit was
performed here.

**T26, first slice: language-aware display of imported KNX data
(2026-09-12), branch `t26-product-language`.** The first of two slices
this task needs (design spec
`docs/superpowers/specs/2026-09-12-product-data-language-design.md`;
plan `docs/superpowers/plans/2026-09-12-product-data-language.md`).
Reads translations already sitting unread in `knx-productdb`'s
`translation` table at exactly one surface: the device parameter panel.
Closes half of `GAP_ANALYSIS_ETS.md` gap **D10**; rewrites
[KNOWN_LIMITATIONS.md §37](KNOWN_LIMITATIONS.md#37-imported-translations-are-stored-but-never-read-and-the-ui-is-english-only--partially-resolved-2026-09-12)
from open to partially resolved, and adds a new
[§64](KNOWN_LIMITATIONS.md#64-languages-blocks-outside-an-application-program-are-discarded-on-import)
for a gap this slice found but deliberately did not fix.

- **`crates/knx-productdb/src/query.rs`**: `parameter_views(conn,
  program_id, language: Option<&str>)` and `parameter_type_enum_options`
  gained the language parameter. When `Some`, a private `translation_overlay`
  query loads that language's `(ref_id, attribute_name) -> text` rows,
  restricted by SQL to `Text`, `FunctionText`, `SuffixText`,
  `VisibleDescription`, `Name` — `Value` is structurally excluded, not
  just avoided by convention, because a parameter's value is a key
  written into the project file and translating it would corrupt stored
  data. The overlay is applied per element *before* the existing `pick()`
  runs, so `ValueLayer`'s meaning (which layer a displayed value actually
  came from) is unaffected by translation. New `translation_languages`
  (database-wide) and `program_translation_languages` (one program) list
  what languages exist; the latter has no caller yet, left in place for a
  later communication-object slice. 8 new tests.
- **`apps/knx-server`**: `GET /api/product-languages` (returns `200 []`
  when no product database is installed, matching the Settings panel's
  fresh-install state rather than erroring) and an optional `?language=`
  query parameter on both `GET` and `POST /api/device/{id}/parameters`,
  threaded through `assemble_parameter_panel`/`parameter_panel_impl`/
  `set_parameter_value_impl` (both of the latter's internal panel-assembly
  call sites — missing either one would have meant editing a parameter
  silently reset the panel to English). 5 new tests.
- **`apps/knx-web`**: `productLanguage.ts` (new) —
  `loadProductLanguage`/`saveProductLanguage`/`useProductLanguage`,
  modelled on `theme.ts`; storage key `knx-desktop:product-language` in
  `localStorage`; `null` means "package default" and is never stored as
  the literal string `"null"`. `SettingsPanel.tsx` gains a fourth select,
  "Product data language", populated from `api.productLanguages()`
  (fetched once, in `App.tsx`, on mount), showing a disabled
  "No product database installed" option when the list is empty.
  `ParameterPanel.tsx` reads the active language via the hook and forwards
  it on every load and every write; fixed a pre-existing label-order bug
  in the same file while there — a field's `text` now takes precedence
  over its `name` (`field.text ?? field.name ?? field.etsId`, was
  reversed) — pinned by a test that fails without the fix. 9 new tests
  plus 1 renamed (10 total, matching the 235 → 244 delta below — a
  renamed test doesn't move that count) across `productLanguage.test.ts`
  (new), `SettingsPanel.test.tsx`,
  `ParameterPanel.test.tsx`, and one mock-shape fix in `App.test.tsx` (its
  `vi.mock("./api", ...)` needed a `productLanguages` stub once `App.tsx`
  started calling it on mount).

No new dependency, npm or cargo. No schema migration — `translation` is
read exactly as `migration.rs` created it. No fallback chain between
languages: a requested language with no row for a given element falls
straight to the package's own untranslated attribute, never through
`en-US` as a middle step. Nothing that writes a project file changed —
device creation, `.knxproj` import/export, and `set_parameter_value`'s
stored raw value are all untouched; only displayed text varies with
language. `com_object_view` was deliberately not given a language in this
slice — its only caller bakes text into the project at creation time, a
different problem than display-time translation.

`cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D
warnings`, `cargo run -p xtask -- check-layering`, and `cargo deny check`
all clean throughout. `cargo test --workspace`: **988 passed / 0 failed /
3 ignored** across 73 `test result` lines (up from the branch's baseline
of 975/0/3 across 72 — the delta is exactly the 8 `knx-productdb` tests
plus the new `knx-server` binary `http_product_language.rs`). `npm
--prefix apps/knx-web run test`: **244 passed across 26 files** (up from
235/25 at the branch point). `npx tsc -p
apps/knx-web/tsconfig.json --noEmit`: clean throughout.

No claim of KNX certification, ETS compatibility, or accessibility
conformance is made anywhere in this slice, and translations are not
described as "supported" in general — they are read at exactly one
surface, the device parameter panel, named explicitly above. What
remains for a later T26 slice: communication-object text (never
translated, baked into the project at device creation), the project's
own `Language` field (still the placeholder `"en"` both importers hand
`Project::new`), `StringTable`/`LocalizedString` resolution (untouched —
this slice's overlay is entirely `knx-productdb`-side), locale-prefix
matching and `navigator.language` detection (neither exists), and the
UI chrome itself (`T25`, a separate task). Also found by this slice and
not fixed in it: `Languages` blocks in `Catalog.xml`, `Hardware.xml`, and
`knx_master.xml` were dropped on import rather than merely unread —
1705 `<Translation>` rows on one measured package — tracked as its own
backlog item, **T32**, in `GAP_ANALYSIS_ETS.md`, since fixing it needed a
schema decision (`translation.program_id` was `NOT NULL`, and a catalog
item belongs to no program). T32 shipped later the same day; see its own
entry at the end of this document.

**T26, first slice, fix round 1 (2026-09-12).** A whole-branch review of
the five commits above, each clean at task level, found one blocker in
the seam between two of them plus two minor documentation/coverage gaps.

- **Blocker — the setting never reached an already-open panel.**
  `App.tsx`'s Settings select and `ParameterPanel.tsx` each called
  `useProductLanguage()` independently; each got its own `useState`, and
  `Inspector.tsx` renders `ParameterPanel` with no `key`, so it is never
  remounted on a device switch. Changing the language in Settings updated
  only `App`'s copy — the open panel kept showing whatever language it had
  when it first mounted, directly contradicting `SettingsPanel.tsx`'s own
  doc comment ("applies immediately"). Fixed by giving
  `apps/knx-web/src/productLanguage.ts` a single module-level store (a
  cached value plus a subscriber set) read through React 19's
  `useSyncExternalStore` — no new dependency, `react` already ships it.
  `useProductLanguage()` keeps its exact signature, so neither call site
  changed shape. The value is seeded from `localStorage` lazily, on first
  `getSnapshot()` call rather than at module-evaluation time, and a new
  `resetProductLanguageForTests()` export un-seeds it — without that,
  module state would leak between tests in the same file even after
  `localStorage` is cleared. `ParameterPanel.test.tsx` and
  `SettingsPanel.test.tsx` were updated to call it in their existing
  `afterEach`s. A regression test in `productLanguage.test.tsx` (renamed
  from `.test.ts` — the new test renders through `react-dom/client`, which
  needs a `.tsx` file to parse JSX) mounts a reader and a writer side by
  side, changes the language only through the writer, and asserts the
  reader observes it without remounting; reverted against the old
  two-`useState` implementation it fails with `AssertionError: expected
  '(default)' to be 'de-DE'` on the post-write assertion — i.e. the
  reader's copy never moved.
- **Minor — a miscounted test total.** The first slice's own summary
  said "12 new/renamed tests"; corrected to 9 new plus 1 renamed (10
  total), matching the 235 → 244 web delta quoted two paragraphs below it.
- **Minor — no end-to-end test for an enum write with a language.**
  `knx-productdb`'s `a_value_translation_never_changes_a_stored_value` and
  `domain.rs`'s `validate_kind_and_bounds` prove, separately, that a
  `Restriction`-kind option's `Value` is never translated. Neither proves
  it together over HTTP. Added
  `enum_write_with_a_language_keeps_the_raw_value_but_translates_its_label`
  to `apps/knx-server/tests/http_product_language.rs`, extending
  `TRANSLATED_PROGRAM` with a second, `Restriction`-kind parameter
  (`P-2`, options `"0"`/`"1"`) whose option labels carry `de-DE`
  translations: `POST /api/device/1/parameters?language=de-DE` writing
  raw `"1"` returns a DTO whose `value` is exactly `"1"` and whose
  matching `enumOptions` entry's `text` is the translated `"An"`.

No new dependency, no schema migration, no change to what a write stores
— same global constraints as the first slice, unchanged by this round.

`cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D
warnings`, `cargo run -p xtask -- check-layering`, and `cargo deny check`
all clean. `cargo test --workspace`: **989 passed / 0 failed / 3
ignored** across 73 `test result` lines (up from this slice's own
988/0/3 across 73 — the delta is exactly the one new
`http_product_language.rs` test; the fix touched no other Rust file).
`npm --prefix apps/knx-web run test`: **245 passed across 26 files** (up
from 244/26 — the one new `productLanguage.test.tsx` regression test;
the file count is unchanged because it's a rename, not a new file).
`npx tsc -p apps/knx-web/tsconfig.json --noEmit`: clean.

**T32: translations outside an application program (2026-09-12), branch
`t32-shared-translations`.** Closes the ingestion gap
[KNOWN_LIMITATIONS.md §64](KNOWN_LIMITATIONS.md#64-languages-blocks-outside-an-application-program-are-discarded-on-import)
was opened for by T26's first slice, and gives the catalog browser a
translated name. Design spec
`docs/superpowers/specs/2026-09-12-shared-translations-design.md`, plan
`docs/superpowers/plans/2026-09-12-shared-translations.md`, five tasks.

- **Schema v4 (`crates/knx-productdb/src/migration.rs`).** `translation`
  loses `program_id` and gains `(scope, scope_id)`: `Program` keys off the
  application program's `@Id` as before, `Catalog` and `Hardware` off the
  owning `Manufacturer/@RefId`, `Master` off `''`. The empty-string
  sentinel is there because SQLite considers NULLs in a non-`INTEGER`
  primary key pairwise distinct, so NULL would permit exactly the
  duplicates the key exists to reject — the same reason
  `dynamic_node.module_def_id` already carries one. The v3→v4 migration
  rebuilds the table and asserts, in test, that the row count before
  equals the row count after.
- **One shared pass (`crates/knx-productdb/src/parse/translation.rs`).**
  `ingest_translations(conn, scope, name, bytes)` walks
  `Languages/Language/TranslationUnit/TranslationElement/Translation` and
  reuses the existing `insert_translations`. `ingest.rs` runs it as a
  second pass over `Catalog.xml` and `Hardware.xml` in the same
  transaction; `classify()` gained `FileKind::MasterData` so
  `knx_master.xml` gets the same treatment through
  `ingest_master_data`. `parse/program.rs` keeps its own inline handling
  untouched — 48,057 known-good rows were not worth a tidy-up.
- **Backfill.** The v4 migration replays every stored
  `Catalog`/`Hardware`/`MasterData` blob through the new pass inside its
  own `SAVEPOINT`, so databases installed before this slice are not left
  translation-less (content-hash idempotence means nothing would ever
  re-ingest them). A blob that fails to parse writes a
  `TranslationBackfillError` into `ingest_unknown` and the migration
  continues; `record_backfill_failure` is now shared with the existing
  `dynamic_node` backfill rather than copied.
- **Reader (`query::catalog_items`, `apps/knx-server`, `apps/knx-web`).**
  `catalog_items(conn, manufacturer, search, language)` with `language =
  None` issues the byte-identical statement it always did — no join, no
  `COALESCE`, nothing that could change SQLite's plan or tie-break. With a
  language it `LEFT JOIN`s `translation` twice (`Name`,
  `VisibleDescription`) on `scope = 'Catalog' AND scope_id =
  catalog_item.manufacturer_id AND ref_id = catalog_item.id` and applies
  the overlay to the search filter and the `ORDER BY` as well as the
  output, so a translated-only match is findable and the list sorts the
  way it displays. `number` is never translated. `GET
  /api/catalog/items` gained `?language=` on its existing query struct;
  `CatalogBrowser.tsx` passes `useProductLanguage()` at both fetch sites
  and has `language` in the effect's dependency array, so changing the
  setting mid-browse refetches. `query::catalog_item`, the single-row
  lookup device creation uses, was deliberately left untranslated: no
  translated string may become a stored identifier.
- **Measured, not assumed.** Installing
  `MDT_KP_AMI_AMS_03_Switch_Actuator_V31a.knxprod` with `knx products
  ingest` and counting `translation` rows per scope gives Catalog 40 (5
  languages), Hardware 30 (5), Master 1635 (18), Program 18546 (5),
  20251 total. The golden corpus assertion moved from 48,057 to 48,190
  rows (48,057 program + 109 catalog + 24 hardware), re-measured rather
  than predicted. §64's published figure of "24 languages" for
  `knx_master.xml` was wrong and is corrected there: 18 languages carry
  those 1635 translations; the 24 are `<ProductLanguages>` catalogue
  entries with no translations attached. The original number came from
  grepping the whole file instead of the `Languages` block.

`cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D
warnings`, `cargo run -p xtask -- check-layering`, `cargo deny check` and
`npx tsc -p apps/knx-web/tsconfig.json --noEmit` all clean. `cargo test
--workspace`: **1006 passed / 0 failed / 3 ignored** across 74 `test
result` lines (branch baseline at `566406a`: 989/0/3 across 73 — 17 new
tests and one new test binary, `apps/knx-server/tests/http_catalog_translation.rs`).
`npm --prefix apps/knx-web run test`: **247 passed across 26 files** (up
from 245/26).

No claim of ETS parity is made here. Translations are ingested from every
`Languages` block a package carries, but only catalog-scope rows are read
by any surface: hardware- and master-scope text sits in the database
unread, and the import report still does not state how many translations
a package contributed. Both residues stay recorded in §64.

**T33: language-aware communication-object text (2026-09-12), branch
`t33-comobject-language`.** Closes the remaining half of
[KNOWN_LIMITATIONS.md §37](KNOWN_LIMITATIONS.md#37-imported-translations-are-stored-but-never-read-and-the-ui-is-english-only--partially-resolved-2026-09-12)
T26's first slice left open: communication-object text was ingested since
T26/T32 (it lives in the `Program`-scope rows of `translation`, not a new
table) but read by nothing. Design/plan
`docs/superpowers/plans/2026-09-12-com-object-language.md`, four tasks.

- **`com_object_view` learns a language
  (`crates/knx-productdb/src/query.rs`, commit `9b41290`).** A fourth
  parameter, `language: Option<&str>`, reuses `parameter_views`'s
  `translation_overlay`/`overlay_text` helpers verbatim and applies the
  overlay before `pick()`. The `SELECT` now also returns `co.id`, because
  a `ComObject`-layer translation's `RefId` is the `ComObject`'s own id,
  not the `ComObjectRef`'s — measured, not assumed (see below). Only
  `Text`, `FunctionText` and `VisibleDescription` are ever overlaid;
  `object_size`, `priority`, `dpt_list`, `number` and the four flags stay
  untranslated, the same values-vs-display-text line T26 drew for
  `Value`. `None` issues no `translation` query, byte-identical to
  before. Also extracted `enrich()`'s inline module-vs-plain `RefId`
  branch into a new public `com_object_lookup_id()`, now the one
  implementation of that reconstruction instead of one per caller; all
  four existing callers (`enrich.rs`'s two call sites and its unit test,
  `apps/knx-server/src/domain.rs`, and a fourth site in
  `parse/program.rs` the plan's own file list missed) now pass `None`,
  since none of them have a language to offer. Six new tests: the `None`
  passthrough, both translation layers, `FunctionText`/
  `VisibleDescription` landing in their own fields, a language with zero
  rows for the program, and an `ObjectSize` translation row present in the
  fixture purely to prove it is never applied.
- **`GET /api/device/{id}?language=` overlays com-object text
  (`apps/knx-server/src/domain.rs`, `routes.rs`, commit `b7bc747`).**
  `device_detail(state, device_id, language)` takes the project lock
  first, builds the base `DeviceDetail` via the unchanged
  `device_detail_impl`, and — only when `language` is `Some` — collects
  each com object's `program_ref`/`ets_id`/`module_instance` and the
  `Layer` of its `text`/`description` overrides while that lock is still
  held. It then drops the project lock, locks `product_db`, resolves the
  program, and calls `com_object_view(.., Some(lang))` per com object,
  overwriting `ComObjectNode::name`/`description` **only** where the
  matching override's stored layer is `Layer::Program` or
  `Layer::ProgramRef`. `Layer::Instance`, `Layer::Inferred` and
  `Layer::UserEdit` are project-authored and are never touched — a
  load-bearing invariant with its own test,
  `an_instance_layer_text_is_never_translated_because_the_project_owns_it`.
  `language: None`, an unresolvable program, or no open product database
  all return the untranslated detail, no product-database query issued.
  Deviation from the plan, accepted: `device_detail_impl` itself was left
  unchanged rather than also gaining a `language` parameter, since it
  only ever holds a bare `&Project` (never the product-database
  connection) and has 13 other call sites outside this task's file list
  that have no language to supply. New
  `apps/knx-server/tests/http_com_object_language.rs`, six tests.
- **The Inspector asks for the user's language
  (`apps/knx-web/src/api.ts`, `App.tsx`, commits `d4f4dc5`/`ca1cdfd`).**
  `deviceDetail(deviceId, language?)` reuses the existing `languageQuery`
  helper `deviceParameters`/`setParameterValue` already call. `App.tsx`'s
  two existing `deviceDetail` call sites now pass the persisted product
  language, and a new effect keyed on that setting refetches the
  currently selected device's detail when the language changes — guarded
  by a monotonic `languageRequestIdRef` (the same idiom
  `CatalogBrowser.tsx`/`ParameterPanel.tsx` already use) so an older
  language's response landing after a newer one's cannot overwrite it.
  That guard was added in a fix round after review found the original
  submission's selection-only check guarded nothing across a language
  switch, since the selection never moves when only the language does; a
  regression test proves the race by resolving two in-flight requests out
  of order and asserting the DOM shows the newer language's text.
- **Measured, not assumed.** Application program `M-0083_A-0317-31-7DC6`
  (`OriginalData/ProductDatabases/MDT_KP_AMI_AMS_03_Switch_Actuator_V31a.knxprod`)
  declares five `<Language>` blocks (`de-DE`, `en-US`, `fr-FR`, `es-ES`,
  `it-IT`), each carrying 53 `ComObject/@Text` and 50
  `ComObject/@FunctionText` translations, and every one of those
  `TranslationElement/@RefId`s matches a declared `ComObject/@Id` — none
  target a `ComObjectRef/@Id`. Confirmed independently by this
  documentation pass by extracting the package and cross-referencing
  every `TranslationElement/@RefId` in its `Languages` blocks against
  both `ComObject/@Id` and `ComObjectRef/@Id`: the count matches exactly,
  and there is no counter-example in either direction for `Text`. One
  thing this plan's own measurement did not mention: the same package
  also carries 39 `ComObjectRef`-scope `FunctionText` translations per
  language (zero `ComObjectRef`-scope `Text`) — so the plan's aside that
  "the `ComObjectRef` overlay path has no coverage in this package" is
  not quite right for `FunctionText`, only for `Text`. Not a code
  concern: the overlay is attribute-generic, and Task 1's synthetic test
  fixtures already exercise the `ComObjectRef`/`ProgramRef` layer
  directly rather than relying on this corpus package for coverage.

`cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D
warnings`, `cargo run -p xtask -- check-layering`, `cargo deny check`, and
`npx tsc -p apps/knx-web/tsconfig.json --noEmit` all clean. `cargo test
--workspace`: **1018 passed / 0 failed / 3 ignored** across 75 `test
result` lines (branch baseline: 1006/0/3 across 74 — 12 new tests, one
new test binary, `apps/knx-server/tests/http_com_object_language.rs`).
`npm --prefix apps/knx-web run test`: **251 passed across 26 files** (up
from 247/26).

No claim of ETS parity is made here, and no claim that the project's own
string table (`knx_core::string_table`) is translated: `StringTable` still
has no resolver anywhere except `build_device_detail`'s own fixed-default
`project.strings.default_language()` call, and the com-object overlay
above substitutes `knx-productdb` text before that call runs rather than
teaching the string table anything. The project's own `Language` field is
still the unread placeholder `"en"` both importers hand `Project::new`.
Device creation and `enrich()` are untouched and still bake untranslated
text into the project file — translation stays entirely display-only,
never stored. Correction to this entry's own earlier framing: `com_object_view`
overlays `FunctionText` on `ComObjectView` exactly like `Text` and
`VisibleDescription`, but nothing reads `view.function_text` back out —
`ComObjectNode` has no field for it, `enrich::apply()` never stores it,
and `crates/knx-report`'s documentation exporter renders com-object names
untranslated regardless of language, since `build_device_detail` takes no
language parameter at all. `FunctionText` is unread at every surface,
same as `SuffixText`. What §64 already named — hardware- and master-scope
translations ingested but read by nothing, and the chrome half tracked as
T25 — is unaffected by this slice and stays open.

**T35: program versions and first-line headers (2026-09-12), branch
`t35-versioning`.** Every program now has a version that admits what it
is: all 14 Cargo packages, `apps/knx-web/package.json` and its lockfile
moved from `0.0.0` to `0.1.0-alpha.1`, each in its own manifest with no
workspace inheritance (the requirement is that programs version
independently). `apps/knx-desktop/src-tauri/tauri.conf.json` *lost* its
`version` key instead of gaining a copy: Tauri falls back to `Cargo.toml`
when the key is absent (`tauri-utils` 2.9.3, `Config::version`), and one
number in one place is the only kind that stays right. The one existing
reader, `knx-etsproj`'s `KNX/@ToolVersion` on export, now writes
`0.1.0-alpha.1`; no fixture asserted the old value.

`knx --version` and `knx-server --version` (or `-V`) print
`<name> 0.1.0-alpha.1+g<short-sha>`. The sha comes from a new `build.rs`
in each binary package — `git rev-parse --short HEAD` via
`std::process::Command`, no dependency — which first checks that
`git rev-parse --show-toplevel` is this workspace (a tree unpacked inside
a foreign checkout gets nothing, not their sha), re-runs when `HEAD`, the
`HEAD` reflog or the loose branch ref moves (the reflog is what survives
`git pack-refs`; a watch on the branch file alone went stale after it,
found in review), honours an explicit `KNX_BUILD_SHA` first (the
`Dockerfile` gained the matching `ARG`, since `.dockerignore` drops
`.git`), and emits nothing when there is no git to ask, so the build
still builds and prints the bare version. Exercised by hand: override, no
git, restored, foreign toplevel, no repository; and the sha was observed
to follow `HEAD` across a commit. What it cannot say — whether the tree
was clean — is [KNOWN_LIMITATIONS.md §65](KNOWN_LIMITATIONS.md#65-version-names-a-commit-never-a-working-tree). `apps/knx-cli/tests/cli_version.rs` and
`apps/knx-server/tests/bin_version.rs` pin the shape against
`CARGO_PKG_VERSION` and check the metadata, when present, is `+g<hex>`;
both also fail if their manifest ever loses its pre-release identifier
before anyone means it to. `knx_server::version_line()` is
public so the desktop shell can say the same thing if it ever wants to.

A first-line header convention, and its lint. A source file's first line
is one sentence saying what the file is for: `//! Sentence.` in Rust
(with line 2 either a blank `//!` or not a doc line), `/** Sentence. */`
in TypeScript, at most 100 columns, one period, no second sentence. It
applies to files created or edited from now on — no repo-wide sweep, by
the user's explicit instruction — and this slice applied it to its own
three edited and four new Rust files. `cargo run -p xtask -- check-headers`
(`xtask/src/headers.rs`, 25 unit tests, wired into CI beside
`check-layering`) fails on a malformed header — including `/** */`, which
once panicked on an underflow, and a UTF-8 BOM on line 1, which once hid
a header silently — and *ratchets* the files without one: `ABSENT_CEILING`
is 201, the lint fails above it, and the constant only ever goes down, so
a new bare file or a header split back into a paragraph fails the gate
without a base ref or a sweep (tripped on both before commit). Counts
today: 32 well-formed, 201 without, 22 generated `ts-rs` files skipped (the
11 committed bindings and the 11 gitignored ones `cargo test -p
knx-projection` drops into `crates/knx-projection/bindings/` — the
latter had to be named explicitly, or the count moved by 11 depending on
whether tests had run), at the time of writing. The 24 pre-existing one-line module docs turned
out to satisfy the grammar as they stood. Observed to fail on two
injected violations, one per language, before they were removed.

**No per-file version.** The user asked for one "wenn überhaupt machbar",
and it is not: nothing can verify that a number was bumped when a file
changed, or by how much; SemVer's compatibility semantics have nothing to
attach to inside a single file; git already gives every file a precise,
machine-maintained history and the `+g<sha>` pins every file's state at
once; and bump-on-touch would conflict on line 1 across the concurrent
worktrees this repository runs. [ADR-0018](adr/0018-program-versions-and-file-headers.md)
argues it out, records the alpha bump rule (per program, when a merged
slice changes what it does or exposes — discipline, not mechanism, and
said so), and names what actually delivers the underlying want: the
sentence, the manifest version, the build sha. The ADR index also gains
the row for ADR-0017, which had never been listed.

`cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D
warnings`, `cargo run -p xtask -- check-layering`, `cargo run -p xtask --
check-headers` (32 well-formed / 201 without, ceiling 201 / 22 generated
skipped) and `cargo deny check` all clean. `cargo test --workspace`:
**1053 passed / 0 failed / 3 ignored** across 77 `test result` lines
(last documented baseline, T33: 1018/0/3 across 75 — the two new
binaries are `apps/knx-cli/tests/cli_version.rs` and
`apps/knx-server/tests/bin_version.rs`; the 35 new tests are 25 in
`xtask/src/headers.rs`, 3 + 3 in those two binaries, and 2 + 2 unit
tests beside each `format_version_line`). `npm --prefix apps/knx-web
test`: **251 passed across 26 files** (unchanged — no TypeScript file was
touched); `npx tsc --noEmit` clean. Whole-branch review (2026-09-12)
returned four required fixes, all landed before merge: the packed-ref
staleness and the foreign-toplevel stamp in `build.rs`, the `/** */`
panic and the BOM blind spot in the lint, and the ratchet; the ADR's
per-file-version ruling was upheld and its argument corrected as noted
in the ADR itself.

Deliberately not touched: any `apps/knx-web/src/*.ts`/`*.tsx` file (the
concurrent `t25-ui-chrome-language` branch owns those), so the TypeScript
header grammar is demonstrated by the lint's unit tests rather than by a
live file; every pre-existing multi-line module doc; and `.ai/`.

**T25: multi-language UI chrome (2026-09-12), branch
`t25-ui-chrome-language`.** Closes the chrome half of `GAP_ANALYSIS_ETS.md`
gap **D10**; partially addresses **D8**. Plans
`docs/superpowers/plans/2026-09-12-ui-chrome-language.md` and (the
language-pack format, added mid-task by explicit user ruling)
`docs/superpowers/plans/2026-09-12-ui-language-packs.md`. Seven tasks.

- **The catalogue and lookup (`apps/knx-web/src/i18n.ts`,
  `messages/en.ts`/`messages/de.ts`).** `messages/en.ts` holds **294
  keys**, measured directly from the shipped file (the plan's own recon
  estimated "~139 literals"; the gap comes from plural pairs — a
  `key.one`/`key.other` pair for every count-sensitive sentence — and
  per-attribute breakdowns, e.g. six delete-restriction sentences
  collapsed into composable pieces, both of which add more catalogue
  entries than a literal-string count predicts). `messages/de.ts` is kept
  at exact parity by a `Record<MessageKey, string>` type annotation
  TypeScript enforces at compile time — a missing German key is a build
  failure, not a runtime gap. `translate()`/`useTranslate()`/`Translate`
  resolve a key against the active language, falling back to English
  exactly once if the active language (built-in or an installed pack)
  has no value for it. `document.documentElement.lang` tracks the active
  language.
- **The UI-language setting (`SettingsPanel.tsx`).** A `<select>` listing
  the two built-in languages (`en`/`de`, detected from
  `navigator.language` on first run) plus any installed language pack,
  each by its own declared `name`, persisted the same `localStorage`
  pattern `theme.ts` already used.
- **The extraction sweep, roughly 30 files.** Every hard-coded English
  literal in the frontend now resolves through the catalogue.
  `Inspector.tsx` alone accounts for the largest concentration, as
  planned: 37 `t(...)` calls across 17 `useTranslate()` sites. Toast
  copy (`toastCopy.ts`), the holiday/late-night joke messages and error
  wrappers, the command palette, dashboard, catalog browser, bus monitor
  and compose form, log panel, group-address CSV buttons, project
  explorer and file picker were all swept in the same pass.
- **The language-pack format, loader and store
  (`apps/knx-web/src/languagePack.ts`, new).** An open-ended JSON format
  — `formatVersion`, `tag`, `name`, `messages` required, everything else
  (including fields a future format version might add) preserved
  unread — that lets a user author a translation for any BCP 47-shaped
  tag, including a language with no registry entry at all (`tlh`
  Klingon, `bar` Bavarian, `art-x-sindarin` a private-use tag for
  anything unregistered), validated for *shape*, never for registry
  membership. A pack that translates a subset of the catalogue is
  installed unconditionally — unknown/missing keys are reported, never
  vetoed. Full user-facing account of the format, written for a
  translator rather than a developer:
  [docs/LANGUAGE_PACKS.md](LANGUAGE_PACKS.md).
- **The Settings-panel surface for packs (`SettingsPanel.tsx`).** An
  import file picker, an "Export English template…" button (the
  intended starting point for a new pack) with a permanent hint about
  its own shadowed-tag trap, a per-outcome import report (applied/
  missing/unknown key counts, a shadowed-built-in-tag warning, a
  grandfathered-BCP-47-tag hint), and a management list of installed
  packs each with its own Export/Remove. Removing the active pack falls
  the UI back to English immediately rather than leaving it pointed at a
  tag that no longer resolves.

**The two fallback rules, deliberately opposite, documented side by
side so neither gets "fixed" into matching the other:** UI chrome always
falls back to English, and only English — never German, never another
installed pack, never a pack's own `basedOn` (a translator's-note field
nothing in the application reads) — because the English string *is* the
message key's own authoritative content, so a fallback shows real words.
Product data (T26/T32/T33) never falls back to another language at all —
it shows the original, untranslated text — because a manufacturer's
parameter text is not this application's own content to substitute a
guess for; a user configuring a physical device needs to know the text
came from that vendor's package, not from a fallback chain.

What no catalogue or pack can reach, because it is composed as plain
text on the server rather than requested as a catalogue key:
`ParameterDiagnostic.message`/`.detail`, `LogPanel`'s session-log entry
fields, the error text quoted inside a translated toast wrapper, and
`crates/knx-report`'s generated documentation export, which is not
language-aware in any respect — no language parameter of any kind, not
UI language, not product-data language. New backlog entry:
[KNOWN_LIMITATIONS.md §66](KNOWN_LIMITATIONS.md#66-server-composed-prose-and-the-documentation-export-are-not-translated-by-any-ui-language-or-pack).
A second, smaller instance: a rejected pack's own rejection reason is
validator text, not a catalogue key, inside an otherwise-translated
sentence —
[KNOWN_LIMITATIONS.md §67](KNOWN_LIMITATIONS.md#67-a-rejected-language-packs-own-reason-is-shown-untranslated-inside-a-translated-sentence).

`npx tsc -p apps/knx-web/tsconfig.json --noEmit`: clean. `npm --prefix
apps/knx-web run test`: **336 passed across 31 files** (branch baseline
before T25's first task: 271/29).

**T18, module-scoped editing slice, Task 2 — the `MI-` component, retained
(2026-09-12), branch `t18-module-scoped-editing`.** Second of six planned
tasks ([design spec](superpowers/specs/2026-09-12-module-scoped-editing-design.md),
D38). `crates/knx-etsproj/src/parse/installation_v21.rs` already parsed
`ModuleInstance/@Id`; `map.rs` used it only as a local wiring key and threw
it away afterward — exactly the datum a per-channel write needs, since it
carries the `_MI-<k>` component `@RefId` does not. Fixed at the source:
`knx_core::ModuleInstance` gains `instance_ets_id: String` (verbatim `@Id`,
retained uninterpreted, `crates/knx-core/src/module.rs`), populated from
`mi.id` in `map.rs` while `source.ets_id` keeps holding `@RefId` unchanged.
`knx-store` gained a matching `module_instance.instance_ets_id` column via
migration `v5 → v6` (`CURRENT_SCHEMA_VERSION` now 6 in both
`knx-core::project` and `knx-store::migration`, a frozen `v5-empty.sqlite`
fixture added, `v4`-`v1` untouched). Every other construction site the new
field broke (`knx-core::devices`, `knx-etsproj::compare`,
`knx-productdb::enrich`, `knx-diff::diff`, `knx-store::project`) got a
plausible-looking but inert test value — none of them feed real data.
`crates/knx-etsproj/src/compare.rs`'s `SemanticModuleInstance` was
deliberately left alone: it does not carry `instance_ets_id`, so the
semantic-diff surface `knx-diff` reports is unchanged. This task does not
resolve what the `MI-` token means (RESEARCH.md's "sharpest unknown #1"
stays open) and does not touch the read/write path, the parameter panel, or
the frontend — those are Tasks 3 and 4. A project saved before this
migration has `instance_ets_id == ""` for every existing `ModuleInstance`
row, same treatment D39 (a later task) gives a genuinely missing one:
read-only, reported, never guessed.

**T18, module-scoped editing slice, closing entry (2026-09-12), branch
`t18-module-scoped-editing`.** Covers Tasks 1, 3, 4 and 5 of the same six
([design spec](superpowers/specs/2026-09-12-module-scoped-editing-design.md),
decisions D35-D43); Task 2 (`instance_ets_id`, above) has its own entry.
Together these close **D25**, the deliberate hole T18 slice 3 left open
(`docs/superpowers/specs/2026-09-11-parameter-editor-design.md`): a
module-scoped field is now writable, and its write changes the same
response's recomputed activation set, per channel.

- **Task 1 — a scope-aware `ValueMap`
  (`crates/knx-productdb/src/dynamic/evaluate.rs`).** `ValueMap` stops
  being a bare `HashMap<String, String>` alias and becomes a struct
  holding an `unscoped` map plus a `scoped: HashMap<(String, String),
  String>` map keyed by `(module_id, ref_id)`. `get(scope, ref_id)` tries
  the scoped map first when `scope.module_id` is `Some`, falls back to
  `unscoped`, and never falls sideways to a different `module_id` (D36).
  New `Diagnostic::ModuleWithoutId` reports a `Module` expansion whose
  `@Id` is absent, once, at the expansion site (D37) — the case a
  per-channel write can never target.
- **Task 3 — write-target reconstruction and validation
  (`apps/knx-server/src/domain.rs`).** A module-scoped write arrives as a
  module-qualified `ets_id` (`MD-<d>_M-<n>_MI-<k>_...`); the server
  decomposes it, resolves which stored `ModuleInstance` (if any) is the
  authority for that `MI-` digit via `resolve_mi_authority` /
  `MiAuthority` (`Found`/`NoMatch`/`Ambiguous`/`Malformed`, D39 rules
  2-3), and refuses the write — loudly, via a `ParameterDiagnosticDto`
  naming both colliding instance ids, not a silent pick — when two or
  more stored instances claim the same `RefId` (D40). Stored-row
  validation gained the same `MI-` check and stopped overwriting one row
  with another silently (D41).
- **Task 4 — the web panel writes the server's own id
  (`apps/knx-web/src`).** A field's DTO now carries the exact id a write
  must use (D43); the panel writes that id verbatim instead of
  reconstructing one client-side, so client and server never disagree
  about which stored row a write targets.
- **Task 5 — fixture proof.** New KV v2.5-derived fixtures give two
  channels of the same `Module` different stored values for the same
  declared parameter and prove, end to end, that each channel's own
  `choose` now resolves differently — the general form of D16 ("all
  instantiations evaluate against the same values") no longer holds
  unconditionally; it holds only when the channels' own values happen to
  agree, or none exist. A deletion experiment (removing the fix) proved
  the new HTTP test genuinely depends on **D42**'s second `evaluate` call
  recomputing against the just-written scoped value — the thing a Task 3
  review round's finding S7 had flagged as asserted but unverified at the
  time. As of this slice, D42's second call is test-covered.

What stays out of scope, named in the design's own Non-goals rather than
silently left out: `Module` *arguments* (`NumericArg`/`TextArg`/
`AllocatorRef`) remain stored-but-uninterpreted; a project's own repeated
instantiation of one `ModuleDef` with two or more stored instances sharing
a `RefId` is refused, not supported (D40) — see
[KNOWN_LIMITATIONS.md §68](KNOWN_LIMITATIONS.md#68-repeated-module-instantiation-is-refused-not-supported);
a `Module` with no `@Id` cannot be matched to a project instance — see
[KNOWN_LIMITATIONS.md §69](KNOWN_LIMITATIONS.md#69-a-module-with-no-id-cannot-be-matched-to-a-project-instance);
a declared-but-not-currently-shown field can no longer be written, because
`parameter_ref` carries no `module_def_id` column to check it against —
see
[KNOWN_LIMITATIONS.md §70](KNOWN_LIMITATIONS.md#70-writing-a-declared-but-not-currently-shown-parameter-is-now-refused);
a project imported before store schema 6 has an empty `instance_ets_id`
on every `ModuleInstance`, so its module-scoped sections stay read-only
until re-import — see
[KNOWN_LIMITATIONS.md §71](KNOWN_LIMITATIONS.md#71-a-project-imported-before-store-schema-6-has-no-module-instance-ids-to-write-with).
Nothing here resolves what the `MI-` token itself means beyond "the
project's own repeat counter" — RESEARCH.md §4.4's "sharpest unknown #1"
stays open.

Four tasks (Task 1 evaluator, Task 3 server, Task 4 web panel, Task 5
fixtures), each reviewed before the next started; this entry and its
sibling docs-only pass are Task 6, the sixth and last. `cargo test
--workspace`: **1080 passed / 0 failed / 3 ignored**, up from 1053 before
this slice began (Task 1's own baseline). `apps/knx-web`: `npm test` —
**339 passed across 31 files**, up from 336 before this slice. `cargo run
-p xtask -- check-headers`: ceiling unchanged at **169**. Closes
`GAP_ANALYSIS_ETS.md` **A3** from "partially closed" to "mostly closed",
and **B8** to describe module-scoped write closure with its named
exceptions above; neither claims full closure, and nothing here or
anywhere else claims ETS behavioural parity. This docs-only pass (T18
slice 4's sixth task) reconciles
[KNOWN_LIMITATIONS.md §3/§68-§71](KNOWN_LIMITATIONS.md),
[GAP_ANALYSIS_ETS.md](GAP_ANALYSIS_ETS.md), [DATA_MODEL.md §11](DATA_MODEL.md),
and [RESEARCH.md §4.4](RESEARCH.md) with what actually shipped, and
corrects two passages in
[the design spec](superpowers/specs/2026-09-12-module-scoped-editing-design.md)
itself (D40's heading, D41's "unreachable" claim) that no longer matched
the shipped behaviour once a Task 3 reviewer traced through it.

**T34: com-object overlay batching, and three riders from T33's review
(2026-09-12), branch `t34-com-object-overlay-batch`.** Closes four
non-blocking findings T33's own whole-branch review raised against
itself. Plan
`docs/superpowers/plans/2026-09-12-com-object-overlay-batch.md`, four
tasks; full accounting in `GAP_ANALYSIS_ETS.md`'s T34 entry (Tier 6),
this entry is the short form.

- **The batch load.** `crates/knx-productdb/src/query.rs` gained
  `com_object_views(conn, program_id, com_object_ref_ids: &[&str],
  language) -> Result<HashMap<String, ComObjectView>, ProductDbError>`,
  the same bulk shape `parameter_views` already had: one `IN (...)` query
  per 900-id chunk, the overlay loaded exactly once for the whole call,
  and an empty slice never touching the database. `com_object_view`
  survives as a one-element wrapper around it, so `create_device`'s
  per-ref loop is untouched. It does hold a batch of ref ids — it loops
  `com_object_ref_ids`' own Vec — but it resolves them one at a time on
  purpose: each gets an `ok_or_else` naming the ref id that is missing
  from the catalog, and with `language: None` it pays no overlay cost
  worth batching away. `apps/knx-server/src/domain.rs`'s
  `device_detail` — described in T33's own entry above as calling
  `com_object_view(.., Some(lang))` per com object — now collects every
  lookup id first and calls `com_object_views` once per fetch instead;
  that sentence in T33's entry describes what shipped that day and is
  left as written, not edited, since this entry supersedes it going
  forward. Measured on `M-0083_A-0317-31-7DC6`
  (`MDT_KP_AMI_AMS_03_Switch_Actuator_V31a.knxprod`, 104 declared
  `com_object_ref` rows): before, 104 overlay loads of 1,249 `de-DE`
  `translation` rows each, 70–72 ms; after, 1 load of 1,249 rows,
  0.92–0.96 ms — roughly 75× faster for this device's fetch.
- **Finding M4.** `apps/knx-web/src/App.tsx`'s three `deviceDetail` call
  sites (`selectEntity`, `handleTreeUpdate`, the `[productLanguage]`
  effect) now share one monotonic counter, `deviceDetailRequestIdRef`
  (renamed from `languageRequestIdRef`), bumped before each request and
  checked on both the success and the error path — previously only the
  language effect had a request-id guard, so an in-flight language reply
  could land after, and overwrite, a later edit-triggered refetch.
- **Finding M6.** `ComObjectView` gained
  `text_translated`/`function_text_translated`/
  `visible_description_translated`, true iff the value `pick()` chose was
  itself an overlay hit. `device_detail` now overwrites
  `name`/`description` only when the stored layer is
  `Layer::Program`/`Layer::ProgramRef` **and** the matching flag is true,
  so a requested language with no translation row for a given attribute
  no longer falls back to the product database's current untranslated
  column in place of the project's own resolved value. Global
  Constraint 2 (layer gating from the project's own `ComObjectInstance`)
  is unaffected.
- **Finding M5, ruled.** `GET /api/device/{id}`'s `Query
  <ParameterLanguageQuery>` extractor (T33) 400s a malformed `language`
  query exactly as `GET /api/parameters/{id}` and `POST
  /api/parameters/{id}/value` already do (T26) — kept, not loosened, for
  consistency across all three routes the extractor guards. Pinned by
  `apps/knx-server/tests/http_com_object_language.rs`'s
  `a_malformed_language_query_is_rejected_and_an_absent_one_is_not`: a
  repeated `language` key (`?language=a&language=b`) is the form that
  actually trips `serde_urlencoded`'s deserializer ("duplicate field
  `language`"); `?language[]=de` does not — it parses as an unrecognized
  key distinct from `language` and reaches the handler as an absent
  language, not a rejected one.

Correction, not a silent swap: `GAP_ANALYSIS_ETS.md`'s T34 entry, before
this branch, cited "3,876 rows / ~1.26 ms" as the per-overlay-load cost.
That figure does not reproduce against any denominator either this
branch's implementer or an independent reviewer could construct — not
per-language filtered or unfiltered, not the all-languages-for-program
total, not the whole-database total. It is withdrawn as unverifiable;
the measured per-load cost above (1,249 rows) is what replaces it. One
property is deliberately not pinned by a test: "one overlay load per
device fetch" has no regression test, because proving it needs SQL
query-count instrumentation (a `rusqlite` trace feature) whose cost
exceeds the risk, and performance sits last in this project's stated
priority order. The property is structural — the batch call sits
outside the per-object loop — not asserted by a test.

`cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D
warnings`, `cargo run -p xtask -- check-layering` and `cargo run -p
xtask -- check-headers` (74 well-formed / 169 without, ceiling 169 / 22
generated skipped, unchanged) all clean. `cargo test --workspace
--no-fail-fast`: **1092 passed / 0 failed / 3 ignored** across 77 `test
result` lines, up from the T18 closing baseline of 1080/0/3 across 77 —
Task 1 added 9 (8 plus one fix-round reverse-polarity fixture), Task 2
added 1 (the M6 regression test, landing at 1090/0/3, independently
reviewer-confirmed), Task 3 touched only `apps/knx-web` and left the
Rust count unchanged, this task added the eleventh,
`a_malformed_language_query_is_rejected_and_an_absent_one_is_not` in
`apps/knx-server/tests/http_com_object_language.rs`, and the final
review's fix round added the twelfth,
`com_object_views_without_a_language_never_queries_the_translation_table`
in `crates/knx-productdb/src/query.rs` — the other half of Global
Constraint 1, pinned by dropping the `translation` table and resolving a
non-empty slice anyway. Web gates on the
branch head: **340 passed across 31 files**, `tsc --noEmit` clean. They
were not re-run after each Rust-only task; the branch's diff is
`crates/knx-productdb/src/query.rs`, `apps/knx-server/src/domain.rs`,
`apps/knx-web/src/App.tsx`/`App.test.tsx` (Tasks 1-3, already merged
before this entry's task) and `docs/`; Task 4 itself, which wrote this
entry, touched no `apps/knx-web` file.

**T16: a device's product and hardware, on `DeviceDetail` itself
(2026-09-13), branch `t16-device-product`.** A parallel session building
the topology view's device-catalog browser reported the gap: `knx_core::
DeviceInstance` has carried `product_ref` (`ProductRefId`) and
`program_ref` (`Hardware2ProgramRefId`) since import, and
`knx-diff`/`knx-etsproj::compare` already read them, but neither ever
reached the projection — a device-catalog browser had no catalog entry
to point back at. Follows T33 Task 2's two-layer shape: the pure
projection states what the *project* says, `apps/knx-server` overlays
what the *product database* says on top.

- **Layer 1, `crates/knx-projection/src/lib.rs`.** `DeviceDetail` gains
  `product: DeviceProductNode`. `DeviceProductNode` carries
  `product_ref`/`program_ref` (verbatim from `DeviceInstance`, empty
  string mapped to `None` — the same convention
  `knx_etsproj::compare.rs:301-302` already uses for these two fields),
  `catalog: Option<DeviceProductCatalog>` and `resolution:
  ProductResolution`. `build_device_detail` sets `resolution:
  NoReference` when both refs are empty; otherwise it cannot yet know
  whether a product database is even loaded, so — per the brief's own
  explicit instruction not to invent a fifth "not yet resolved" variant —
  it emits `ProductResolution::NoDatabase` as an honest placeholder ("no
  database was consulted, as far as this pure function can tell") that
  `apps/knx-server::domain::device_detail` is documented, in three
  places, to always overwrite. `DeviceProductCatalog` mirrors
  `knx_productdb::query::DeviceProductRow` field-for-field, duplicated
  rather than shared because `knx-projection` must not depend on
  `knx-productdb` (`xtask check-layering`, non-negotiable per the brief).
- **Layer 2, `crates/knx-productdb/src/query.rs`.** New `device_product(conn,
  product_ref_id, hardware2program_ref_id, language) -> Result<Option<
  DeviceProductRow>, ProductDbError>` resolves the whole chain in one
  place: `product` → `hardware` (for name/version/serial), `hardware2program`
  → `application_program`, an optional `catalog_item` match, and
  `manufacturer` for the display name. `Ok(None)` only when `product.id`
  does not exist; a product that resolves while its `hardware2program`
  does not is a partial row, not an absent one — the application fields
  come back `None` rather than losing the product name too, per
  CLAUDE.md's "never silently discard information." `language: Some(_)`
  overlays `product.text`, `catalog_item.name` and
  `application_program.name` via the `translation` table, following
  `catalog_items`' own two-full-literal-SQL-statement shape (translated
  and untranslated, not a spliced hybrid). `hardware.name` is
  deliberately **not** overlaid: inspecting four real manufacturer
  packages' `Hardware.xml` directly found zero `TranslationElement`s
  whose `@RefId` is a `Hardware/@Id` — only `Product/@Id`s are ever
  translation targets in `Hardware` scope in this schema. Documented in
  the function's own doc comment and in
  [KNOWN_LIMITATIONS.md §64](KNOWN_LIMITATIONS.md#64-languages-blocks-outside-an-application-program-are-discarded-on-import),
  not silently assumed.
- **Layer 3, `apps/knx-server/src/domain.rs`.** Extends `device_detail`'s
  existing overlay step (T33 Task 2) rather than adding a second lock
  acquisition: still locks only `project` first, drops it, then locks
  `product_db` once — now shared by both the new product-resolution
  lookup and the existing com-object translation overlay. The two are
  gated independently: product resolution runs whenever a ref is stated,
  **regardless of `language`**, since database membership is a fact, not
  a translation, while the com-object text overlay stays gated on
  `language.is_some()` exactly as before. No product database configured
  overwrites the placeholder with the confirmed `NoDatabase`; a database
  that does not contain the refs yields `NotInDatabase`; a hit yields
  `Resolved` plus a populated `catalog`.
- **Generated TypeScript.** `crates/knx-projection/bindings/` (gitignored,
  materializes via `cargo test -p knx-projection`) gained
  `DeviceProductNode.ts`, `DeviceProductCatalog.ts` and
  `ProductResolution.ts`; `DeviceDetail.ts` now imports `DeviceProductNode`
  and carries `product: DeviceProductNode`. `ProductResolution` is `"Resolved"
  | "NoReference" | "NoDatabase" | "NotInDatabase"`. **Not applied to
  `apps/knx-web`** — a parallel session owns every file under it, and its
  checked-in `apps/knx-web/src/bindings/DeviceDetail.ts` is now stale
  relative to this branch; that copy is deliberately left untouched here,
  for the other session to regenerate on its own schedule. *Update
  (2026-09-13, branch `codex-ui-workbench`):* it did, and built the UI on
  top — see the Codex UI workbench section at the end of this file. The
  checked-in `apps/knx-web/src/bindings/` is no longer stale.
- **Tests.** `knx-projection`: a device with both refs (verbatim refs,
  `NoDatabase` placeholder), a device with neither (`NoReference`), plus
  the three new `ts-rs` export tests. `knx-productdb`: full chain
  resolves; a missing `hardware2program` yields a partial row, not
  `None`; an unknown product id yields `None`; the translated path
  returns overlaid text while the untranslated path and `hardware_name`
  do not, in the same test. `apps/knx-server`
  (`tests/http_device_product.rs`, new file): the overlay replacing the
  placeholder end to end
  (`device_product_resolution_always_overwrites_the_projections_placeholder`),
  no database loaded (`NoDatabase`), a loaded database missing the refs
  (`NotInDatabase`), a device with no stated ref staying `NoReference`
  even with a database loaded, and resolution working with no `language`
  query parameter at all.

`cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D
warnings`, `cargo run -p xtask -- check-layering` and `cargo run -p
xtask -- check-headers` (75 well-formed / 169 without, ceiling 169 / 25
generated skipped) and `cargo deny check` all clean. `cargo test
--workspace --no-fail-fast`: **1106 passed / 0 failed / 3 ignored**
across 78 `test result` lines (one more line than the T34 baseline's 77:
a new test binary, `http_device_product.rs`), up from the freshly
re-measured pre-branch baseline of 1092/0/3 across 77 — 14 new: 2
hand-written `knx-projection` unit tests, 3 `ts-rs` export tests
(`DeviceProductNode`, `DeviceProductCatalog`, `ProductResolution`), 4
`knx-productdb` tests, 5 `knx-server` tests. Web gates
(`npm test -- --run`, `tsc --noEmit` under `apps/knx-web`) are **not
applicable**: this branch's diff touches no path under `apps/knx-web`.

**T17: line-scan (bus-side device discovery) closes, with paperwork
(2026-09-13).** Four prior tasks on this branch shipped the code:
`ScanPlan`/`ScanPlanBuilder` (`crates/knx-core/src/scan.rs`, an exclusion
list honoured by construction, never filtered after the fact); `Tpci`
encode/decode and the device-descriptor Application Layer services
(`crates/knx-net/src/cemi.rs`); `ProbePolicy`/`ProbeOutcome`/
`probe_address`/`scan_line`, implementing `NM_IndividualAddress_Check`
(`crates/knx-net/src/scan.rs`); and the `knx bus scan` CLI
(`apps/knx-cli/src/scan.rs`, `apps/knx-cli/src/main.rs`). This task is
the fifth and last: documentation only, plus one permitted line of code.

Docs updated: `docs/GAP_ANALYSIS_ETS.md` — **T17**'s backlog entry moves
to Done (2026-09-13), recording what shipped against what
[RESEARCH.md §8.5](RESEARCH.md#85-line-scan--bus-side-device-discovery--t17-spike-2026-09-12-shipped-2026-09-13)
specified, plus the corrected timeout-policy citation and the
2026-09-13 live-validation figures; **E2**'s row moves from an open gap
to partially closed — the scan reports where the bus and a project
disagree, nothing reconciles that disagreement into the project file
yet, and that write-back is explicitly out of scope here.
`docs/KNOWN_LIMITATIONS.md` — §72 rewritten from "researched, not
implemented" to the shipped, still-true cost accounting, keeping the
2026-09-12 pre-implementation full-line measurement (`xknx`, 254
addresses, 23.1 minutes) distinct from the 2026-09-13 measurement taken
against this repository's own binary; six new sections (§§73-78) give a
durable home to findings that would otherwise have lived only in a
research doc: what a scan cannot learn (product identity, manufacturer,
serial number — T16's territory), the negative-Layer-2-confirm BUSY case
indistinguishable from absence, why a shorter `--timeout-ms` is
supported but not the default, why the negative-confirm fast path was
deliberately not built and why `Indeterminate` does not retry, the
one-line-at-a-time/no-coupler-crossing scope boundary, and — added by a
later fix round on this branch — other KNXnet/IP tunnelling endpoints
reported as occupied devices.
`docs/RESEARCH.md` §8.5 — Finding 1's mis-attribution of the ~6 s
vacant-probe cost to a client library's own policy constant
(`xknx`'s `MANAGAMENT_CONNECTION_TIMEOUT`) is corrected to the actual
source: `03_03_04 Transport Layer v01.02.03 AS`, clause 4
"Parameters of Transport Layer" (page 16 of 38, `:665-688`) fixes
connection timeout at 6 s system-wide; clause 5's Local Variables table
(page 17 of 38, `:696-705`, specifically `:702-703`) names the two local
timers, `connection_timeout_timer` and `acknowledgment_timeout_timer`,
that implement it — corrected from an imprecise `:760-779` citation this
task's own brief carried, which is actually the clause's Actions table
(corroborating, not naming, the two timers). The "not fully pinned down
from this corpus" sentence about whether the Transport Layer keeps its
own timeout logic is removed; clause 4 answers it. A new paragraph
records why the fast preset is not the default (occupied round trips
measured 13.6-6016.5 ms; a short timeout reports the slowest present
device vacant). A new Finding 4 records the 2026-09-13 live-validation
results against this repository's own binary (dry-run candidate counts,
a nine-address all-occupied run, a five-address all-vacant run, and the
30431 ms vs 30430 ms round-trip arithmetic check) without naming any
address, per this repository's public-facing rule. The section's opening
"nothing described here is implemented" disclaimer is left exactly as
written — not edited into a retroactive lie — with a new paragraph
directly under it noting that it stopped being true on 2026-09-13, and
the stale "documented, not built" TPCI/connection-state-machine
paragraph near the end of the section is replaced with what actually
shipped and what remains out of scope (a general-purpose long-lived
Transport Layer connection manager, cross-coupler scanning, and
concurrent probing).

One line of code outside `docs/`, the only change this task's brief
permitted: `apps/knx-cli/src/main.rs`'s `USAGE` footer stated exit codes
that only ever described `import`/`ga-import` (0/1/2), while `USAGE`
itself lists many `bus` subcommands that never return 2 — confirmed by
`grep -n "ExitCode::"`, every `ExitCode::from(EXIT_IMPORTED_WITH_ERRORS)`
call sits inside `run_import` or `run_ga_import`, nowhere else. The
footer now states the actual rule: 0 = success generally, 1 = failure
for any command, 2 = `import`/`ga-import` only, when a report was
produced but contains errors — no other subcommand, `bus scan` included,
ever returns 2. `USAGE` itself was left untouched, as instructed.

`ROADMAP.md` was not touched by this task: its brief named exactly four
docs (`GAP_ANALYSIS_ETS.md`, `KNOWN_LIMITATIONS.md`, `RESEARCH.md`,
`IMPLEMENTATION_STATUS.md`), and `ROADMAP.md` was not one of them, even
though it linked to §8.5's anchor and that anchor's text changed (the
heading now reads "...T17 spike (2026-09-12), shipped (2026-09-13)").
`81e71db`, a later fix-round commit on this same branch, closed that gap:
both `ROADMAP.md` links now point at the corrected anchor and both read
as shipped.

All six required gates green: `cargo fmt --all --check` clean; `cargo
clippy --workspace --all-targets -- -D warnings` clean; `cargo test
--workspace --no-fail-fast`: **1164 passed / 0 failed / 3 ignored**
across 77 `test result:` lines, unchanged from this branch's pre-task
baseline (a documentation task, plus one comment-only string literal
change, was not expected to move this number, and it did not); `cargo
run -p xtask -- check-layering` clean; `cargo run -p xtask --
check-headers`: 77 well-formed / 169 without a header (ceiling 169), 22
generated files skipped, unchanged; `cargo deny check` clean (only
pre-existing `advisory-not-detected` informational warnings for
advisories that do not match any dependency in this workspace, zero
errors). Web gates: not applicable — this task touched no
`apps/knx-web` path.

**A6, `.knxproj` ZIP-password derivation, verified against the KNX
Standard's own test vectors (2026-09-13), branch
`a6-knxproj-password`.** `docs/KNOWN_LIMITATIONS.md` §13 blamed both
`.knxproj` decryption schemes on "documented from `xknxproject` source
but unverified" as if they were one problem with one fix condition. They
are not: the AES/PBKDF2 key derivation for schema ≥ 21 (ETS6+) is
specified in *The KNX Standard v3.0.0*, *Project Schema23 v01.00.00*,
clause 4.2.4 "Password protection" (p.64/64), complete with its own
published test vectors — testable today, without any encrypted file.
Container decryption is the part that still genuinely needs a real
protected project.

`crates/knx-secure` (previously an empty doc comment) gains
`derive_knxproj_zip_password(project_password: &str) -> ZipPassword`:
UTF-16LE-encodes the password (no BOM), PBKDF2-HMAC-SHA256s it against
the clause's fixed ASCII salt `"21.project.ets.knx.org"` for 65536
iterations into 32 bytes, and Base64-encodes the result via the `pbkdf2`
crate's `pbkdf2_hmac::<Sha256>` (its default `"hmac"` feature is exactly
this generic function — no separate `hmac` crate dependency needed) and
the existing workspace `sha2`. `base64` (already present transitively at
the same 0.22 line, so no new resolved version) does the encoding rather
than a hand-rolled encoder, since the direct dependency was not awkward.
The return type, `ZipPassword`, carries no `Display` impl and no `serde`
impl at all — a compile-time guarantee it cannot enter a report or an
API response by accident — and its `Debug` impl prints a fixed
`"ZipPassword(REDACTED)"` regardless of contents; `expose(&self) -> &str`
is the one deliberate way out, documented as the boundary where leak
responsibility starts. It does not zero its buffer on drop: a hand-rolled
zero-on-drop without a volatile write (the `zeroize` crate, not added
here) can be optimised away as a dead store, and this change chose not
to claim a guarantee it cannot back up — the enforced guarantee is the
type-level one.

Tests assert byte-exact agreement with two of the clause's three
published vectors (`"a"` → `+FAwP4iI7/Pu4WB3HdIHbbFmteLahPAVkjJShKeozAA=`,
`"test"` → `2+IIP7ErCPPKxFjJXc59GFx2+w/1VTLHjJ2duc04CYQ=`) — external
vectors, not the implementation checked against itself. The clause's
third vector, a password containing non-ASCII characters, defeats both
this repository's spec-corpus Markdown extraction and a direct
`pdftotext` run identically: both render it as `Penn¥w1se` followed by
an unmappable glyph, because neither tool's embedded-font ToUnicode CMap
resolves the final character. Rendering the source PDF's page 64 to a
600 DPI raster and reading the glyph directly (bypassing text extraction
altogether) identifies it unambiguously as the "Clown Face" emoji
(U+1F921 🤡) by its distinctive red hair tufts, blue-ringed eyes, red
nose and pink smile. One rendering-visible ambiguity remained — whether
a space sits between `w1se` and the emoji, since typesetting can insert
one before a wide inline glyph purely for layout — and was resolved by
computing the derivation (already verified against the two vectors
above) for both candidates: only `"Penn¥w1se 🤡"` (with the space)
reproduces the clause's published hash
`ZjlYlh+eTtoHvFadU7+EKvF4jOdEm7WkP49uanOMMk0=` exactly. A non-invertible
PBKDF2-HMAC-SHA256 match is not something a wrong reconstruction could
produce by chance, but it is a different evidence path than the other
two vectors' plain text-extraction-and-assert, and the task brief that
scoped this change anticipated this vector would likely stay
unrecoverable — so it is committed as a third test, with the full
recovery account attached in its comment, for a human to accept or
reject on its own merits rather than silently promoted to the same
footing as the two vectors above.

`crates/knx-etsproj/src/container.rs` is untouched:
`ContainerError::PasswordProtected` still refuses before ever opening
the encrypted entry, for both schemes, unconditionally. This change
makes one documented step verifiable; it does not add a feature, and no
document may say otherwise.

Dependencies added, workspace-pinned like their neighbours: `pbkdf2 =
"0.12"` (RustCrypto), `base64 = "0.22"` (already present transitively at
this exact version via the tauri/axum dependency chains — this change's
direct use resolves to the same version, adding no new one).
`cargo deny check` passed clean for both: MIT OR Apache-2.0, both already
on the licence allowlist; `Cargo.lock` gained `hmac 0.12.1` and `pbkdf2
0.12.2` as new entries (pulled in transitively by `pbkdf2`) plus
`subtle 2.6.1`, nothing else.

Docs: `docs/KNOWN_LIMITATIONS.md` §13 rewritten to split "Lifted when"
into the key-derivation half (lifted, as of this change, for schema ≥
21) and the container-decryption half (still needs a real
password-protected project, for *both* schemes — ZipCrypto for schema <
21 remains sourced only from `xknxproject`, untouched by this change).
`docs/RESEARCH.md` §2.3's evidence marker corrected from "[V — read from
`xknxproject` source]" to citing the Standard directly for the ETS6
scheme, with `xknxproject`'s matching implementation now standing as
corroborating evidence rather than the primary source; the ZipCrypto
line is left as `[A]`, unchanged. `docs/GAP_ANALYSIS_ETS.md`'s A6 row
gains a dated note; its core claim ("detected, refused, never
decrypted") was already accurate and did not need correcting.

All eight gates green: `cargo fmt --all --check` clean; `cargo clippy
--workspace --all-targets -- -D warnings` clean; `cargo test --workspace
--no-fail-fast`: **1184 passed / 0 failed / 3 ignored** across 78 `test
result:` lines from one untruncated run (up from this branch's
`e97a91c` baseline of 1180/0/3 across 78 lines — the 4-test increase is
exactly `knx-secure`'s new test module, nothing else moved); `cargo run
-p xtask -- check-layering` clean (`knx-secure` has no internal `knx-*`
dependency, so no edge to check); `cargo run -p xtask -- check-headers`:
ceiling unchanged at 169 files without a header — `knx-secure/src/lib.rs`
already carried one; `cargo deny check` clean (only the same
pre-existing `advisory-not-detected` informational warnings for
advisories matching no dependency in this workspace). Web gates: not
applicable — this task touched no `apps/knx-web` path.

**T36: D10 master-translations, slice 1 — a measured `Master`-scope
reader, locale-prefix matching in one place, and ingest-time translation
counts (2026-09-13), branch `d10-master-translations`.** Backend only,
three requirements of a four-requirement brief (the fourth is this
paragraph); `apps/knx-web` untouched.

`crates/knx-productdb/src/query.rs` gains `query::datapoint_types`/
`query::datapoint_type` (the single-row form), the first `Master`-scope
reader `datapoint_type` has ever had: every row, `main` then `sub`
ascending, with `text` overlaid from a `Master`-scope,
`attribute_name = 'Text'` translation in the requested language when one
resolves, falling back to the package's own untranslated `text`
otherwise — never an error, never an empty string on a miss. Checked
against a real blocker before writing a line of the reader: installing
all five sampled `.knxprod` packages under
`OriginalData/ProductDatabases/` and counting `datapoint_type` gives 383,
354, 234, 234 and 234 rows respectively — never zero, so there was
nothing to report as a blocker.

`best_matching_language` — requested `de` now resolves a stored `de-DE`
— is implemented exactly once, in `query.rs`, and every overlay this
file has, old and new, resolves its language through it: the four
pre-existing overlays plus the new `master_text_overlay` behind
`datapoint_types`. Three dedicated unit tests: an exact match is
preferred over a prefix match; `de` matches `de-DE`; a hypothetical
`deX` does not, which is what proves the match requires the `-`
separator rather than a bare string prefix. No frontend caller sends a
bare primary-language tag yet (`apps/knx-web`'s language pickers
populate their options from the exact tags a package stored), so this
is backend plumbing ahead of any UI exercising it — recorded as such,
not oversold.

The import path now measures, rather than predicts, how many
`translation` rows it wrote. `crates/knx-productdb/src/parse/
translation.rs`'s `insert_translations` returns `bool` (did this
`INSERT OR IGNORE` actually change a row, not merely get parsed and
ignored as a duplicate key) and `ingest_translations` sums that instead
of counting every `Translation` element it walked past — proven by a
new test that ingests the same `knx_master.xml` twice and asserts the
second pass's count is `0`. This threads through
`parse/master.rs`'s new `MasterIngest { unknown, translations }`
(replacing a bare `Vec<UnknownConstruct>` return), `parse/program.rs`'s
`ProgramIngest::translations`, a new `report::TranslationCounts
{ program, catalog, hardware, master }` with `total()`/`add()`, into
`IngestOutcome::Ingested::translations` and
`InstallReport::translations`. Schema bumped to v5
(`CURRENT_PRODUCTDB_VERSION`): `migrate_v4_to_v5` gives `package` four
new `translation_*_count` columns, each guarded by its own
`PRAGMA table_info` check before the `ALTER TABLE` runs — not
decoration; a test fixture that rolls `dynamic_node`/`translation` back
to an earlier shape without touching `package` (see `dynamic_tree.rs`)
replays this migration against a `package` table that already has the
columns, and an unguarded `ALTER TABLE ADD COLUMN` would fail with
SQLite's own "duplicate column name" in exactly that case. A package
installed before this slice reports `0` for all four counters on a
retried install, by design: re-deriving the true count would mean
re-parsing bytes the migration has no access to, so it names the gap
rather than inventing a number. `apps/knx-cli`'s `install` output now
prints the total and the per-scope breakdown. A new integration test
(`standalone_packages.rs`) builds a synthetic package with one
`Translation` in each of the four scopes and asserts
`InstallReport::translations` attributes each to the right scope, that
the total matches `SELECT count(*) FROM translation`, and that a
retried (skipped) install reports the counts recorded at the original
install rather than zero.

**Dispatcher-resolved ambiguity, checked against two corpora rather than
assumed from one.** Every `TranslationElement` under every sampled
package's `knx_master.xml` `<Languages>` block carries
`AttributeName="Text"` — confirmed across all five sampled packages, not
just the three+one this limitation's doc comment previously cited — no
other `AttributeName` value was seen. The `RefId` families a
`Master`-scope translation can carry (`DPST-*`, `DPT-*`, `FP-*_DR-*`,
`FT-*`, `SU-*`) are identical across the two sampled packages whose
`knx_master.xml` uses the newer scheme
(`MDT_KP_AMI_AMS_03_Switch_Actuator_V31a`: 328/47/738/180/342 of 1635;
`Dummy_Applikation_Secure`: 314/45/697/170/323 of 1549) — no surprise
family in either. The other three sampled packages share an older
`knx_master.xml` scheme with only `DPST-*`/`DPT-*`, no `FP-*_DR-*`/
`FT-*`/`SU-*` at all. Neither check widened the implementation: `FT-*`/
`SU-*`/`FP-*_DR-*` still have no table (`parse/master.rs` parses only
`Manufacturer`/`DatapointType`/`DatapointSubtype`), so `datapoint_types`
stays exactly as narrow as its doc comment says, and the gap is reported
rather than quietly worked around.

Docs: `docs/KNOWN_LIMITATIONS.md` §64 gains a dated paragraph covering
all three pieces above and a corrected "Lifted when"; §37 gains a dated
correction narrowing its own "no locale-prefix matching" claim to
"backend only, no caller exploits it yet". `docs/GAP_ANALYSIS_ETS.md`'s
D10 row gains a dated note — the row stays open, both because the
frontend half of locale-prefix matching is untouched and because
`FT-*`/`SU-*`/`FP-*_DR-*` still have no table.

All eight gates green on this branch: `cargo fmt --all --check` clean;
`cargo clippy --workspace --all-targets -- -D warnings` clean (two
pre-existing lint violations surfaced and fixed along the way — a
collapsible `if` in `parse/translation.rs` and an unnamed-complex-type
warning on a pre-existing `query.rs` helper, `catalog_overlay`, neither
introduced by this slice but newly caught by this slice's `-D warnings`
run); `cargo test --workspace --no-fail-fast`: **1241 passed / 0 failed
/ 3 ignored** across 78 `test result:` lines from one untruncated run
(up from this branch's `dd33536` baseline of 1225/0/3 across 78 lines —
the 16-test increase is every test this slice and its immediate
predecessor on this branch added; none removed, none skipped). Fixing
that run surfaced two more latent issues, both fixed here rather than
deferred: `migrate_v4_to_v5`'s `ALTER TABLE` needed the idempotency
guard described above, and two pre-existing integration tests in
`dynamic_tree.rs` asserted a hardcoded `user_version == 4` that the v5
bump would otherwise have broken, alongside three such assertions in
`migration.rs` and two in `standalone_packages.rs` found and fixed
earlier in this same slice — eight hardcoded version literals in total,
all now read `CURRENT_PRODUCTDB_VERSION`. `cargo run -p xtask --
check-layering` clean; `cargo run -p xtask -- check-headers`: ceiling
unchanged at 169 files without a header (no new file added outside two
deleted scratch examples used only to measure the real corpus, never
committed); `cargo deny check` clean (same pre-existing
`advisory-not-detected` informational warnings, nothing new). Web gates:
not applicable — this slice touched no `apps/knx-web` path.

**E6: routing multicast override (2026-09-13), branch
`e6-routing-multicast`.** Closes `docs/GAP_ANALYSIS_ETS.md`'s E6 row —
`route-monitor`/`route-send` can now reach an installation that does not
use the standard `224.0.23.12` group.

R1 first: Core v01.06.02 AS §8.5.2.2 derives the Routing Multicast
Address from the System Setup Multicast Address by an unbounded offset
(default zero); Routing v01.05.02 AS §2.3.1 fixes the *port* at 3671 for
every installation, not the address; §2.3.2's 180-Subnetwork figure is
guidance for when an installation should deviate, not a range the code
can enforce. Neither document narrows the address below "any IPv4
multicast address", and both citations were re-checked against the
original PDF with `pdftotext` (word for word, no discrepancy) before any
validation code was written — see `docs/KNOWN_LIMITATIONS.md` §31 for
the full citations and evidence markers.

`crates/knx-net/src/client.rs`: `BusConnection::connect_routing_to_group`
is new, alongside the unchanged `connect_routing`; both now funnel
through one `RoutingClient::connect_to_group`, so `connect_routing` is
exactly that function called with `ROUTING_MULTICAST`'s own address —
the default and the override cannot silently diverge. `group` is
rejected before any socket call if `!Ipv4Addr::is_multicast()`, via a
new `BusError::NotMulticast(Ipv4Addr)` that names the address, rather
than whatever OS error `join_multicast_v4` would have produced further
in. `RoutingClient` gained a `group: SocketAddrV4` field so `send()`
targets the joined group, not the bare constant. `DISCOVERY_MULTICAST`
is untouched — out of scope, its own design question.

`apps/knx-cli/src/main.rs`: `route-monitor` and `route-send` both gained
`--multicast-group <addr>` — a bare IPv4 address, never `address:port`,
because §2.3.1 makes the port an installation-wide constant, not a
per-connection choice. Omitted, both subcommands call `connect_routing`
exactly as before — verified byte-for-byte by the new
`route_{monitor,send}_args_without_multicast_group_parses_to_none`
tests. `USAGE` documents the flag and its citation; there is no separate
CLI reference doc to update.

Six new tests in `crates/knx-net/src/client.rs`: the validation path
(a unicast address rejected with a named reason, no socket touched, so
it never skips); the two boundary cases of the 224.0.0.0/4 range (one
step below, one step above); a genuine multicast address clearing
validation; the default path asserted against the `ROUTING_MULTICAST`
constant itself, not a repeated literal; and the override path asserted
to join the *given* group, not silently fall back to the default — the
last two skip, not fail, without a multicast route, same policy as the
pre-existing round-trip test. Four new tests in `apps/knx-cli/src/
main.rs` cover `--multicast-group` parsing for both subcommands,
present and omitted.

Stated plainly, in both the code's doc comments and
`docs/KNOWN_LIMITATIONS.md` §31: this override has never been run
against a real installation using a non-default group. A flag that
compiles and a validation that rejects garbage are not proof that a
second KNXnet/IP router on the wire receives anything sent to a custom
group — no hardware exists for this to be tested against, and none was
touched running this task (CLAUDE.md's "only implement protocol
behavior that is technically verified" — the wire behavior here is
verified only for the standard group, which this task did not change).

Docs: `docs/KNOWN_LIMITATIONS.md` §31 rewritten — resolved for routing,
with discovery's still-hardcoded group named explicitly as the
remaining residue, and `[D]`/`[A]` evidence markers on every claim that
needs one. `docs/GAP_ANALYSIS_ETS.md`'s E6 row closed with a dated note.

All eight gates green: `cargo fmt --all --check` clean; `cargo clippy
--workspace --all-targets -- -D warnings` clean; `cargo test --workspace
--no-fail-fast`: **1251 passed / 0 failed / 3 ignored** across **78**
`test result:` lines from one untruncated run (up from this branch's
`f093023` baseline of 1241/0/3 across 78 lines — the **+10** increase is
exactly the ten tests this task added, none removed, none ignored; the
two socket-dependent ones did not take their skip path either, this
sandbox having a multicast route, so both actually asserted); `cargo run -p xtask -- check-layering` clean (`knx-net`'s and
`knx-cli`'s existing dependency edges are unchanged — no new crate
dependency); `cargo run -p xtask -- check-headers`: ceiling unchanged at
169 files without a header (no new file; both touched files already
carried one); `cargo deny check` clean (same pre-existing
`advisory-not-detected` informational warnings, nothing new). Web gates:
`npm test -- --run` and `./node_modules/.bin/tsc --noEmit` both green,
unaffected — this task touched no `apps/knx-web` path.

**PDB-2: a project can now be created from scratch (2026-09-13), branch
`pdb-new-project`.** Closes the backend half of the priority sentence in
`goal.md` — "a user can install a device using its manufacturer-supplied
product database, without requiring that the product data first appeared
in an imported ETS project". It did not work before this slice, and the
reason was structural rather than a bug in the catalog code:
`apps/knx-server/src/domain.rs` had exactly two production sites that
set `*state.project = Some(..)` — the ETS import at line 237 and
`open_native_project` at 338 — so a project could only come into
existence from a `.knxproj` or from a previously saved `.knxdb`.
`create_device_impl` reads no import-supplied field and its product-data
half already worked; it died one line into its second half, at
`project.as_mut().ok_or("no project open")?`. Opening a fresh file was
not a way in either: `knx_store::load_project` answers `NotSaved` for a
database nothing was ever written to.

R1, what a new project actually contains, decided from the domain model
rather than from what made the test pass. The seed is **one
`Installation` and nothing else** — no area, no line, no building part,
no group range. `Command::CreateDevice` needs `installations.first_mut()`
to exist (`crates/knx-core/src/command.rs`, the `CreateDevice` arm) but
takes `line: Option<LineId>`, and a device created with `None` is pushed
onto `topology.unassigned`, which `crates/knx-core/src/topology.rs`
documents in so many words as "valid project state, not an error". So a
device can be placed with no topology at all, and none was invented. The
repository does **not** say what ETS itself puts in a new project — no
document in `docs/` records it and no sample of an ETS-created empty
project exists in `OriginalData/` — so the choice rests on the domain
model alone and is stated that way rather than guessed from memory.

Two fields are seeded beyond the bare `Installation`, each for a named
reason. `info.project_id` gets `P-0001`, shaped like the ids observed in
the reference exports (`P-0512`, `P-03DE`): `knx-etsproj`'s exporter
rejects an empty project id outright (`export/schema11.rs`,
`export/schema21.rs`) and uses it as the ZIP directory name, and no
`Command` in `knx-core` can set it afterwards — an unseeded from-scratch
project could therefore never be exported at all, which is a dead end,
not a minimal seed. The installation's `name` is **not** seeded: it comes
from the request, and an absent one leaves the installation unnamed,
exactly what the ETS mapper produces for an `Installation` with no `Name`
attribute (`crates/knx-etsproj/src/map.rs`, `unwrap_or_default()`).
`Installation::name` is a plain `String`, not a `LocalizedString`, so any
default would be user-visible text; it belongs in the frontend's message
catalogue (`apps/knx-web/src/messages/{de,en}.ts`), not hardcoded in a
layer that must not know about the UI. `Project::new`'s
`default_language` comes from the request too, defaulting to `"en"` —
the same value every other `Project::new` call site in this repository
uses.

R2, the route. `domain::new_project_impl` sits beside
`open_native_project` and reuses its state-reset block — project,
command stack, import counts, opaque passthrough, manufacturer refs —
plus one thing that function *sets* and this one must **clear**:
`store_path`. Leaving the previous file's path behind would let the next
plain `POST /api/project/save` overwrite that file with the new empty
project, which is data loss rather than a cosmetic slip;
`a_new_project_clears_the_path_the_previous_one_was_loaded_from` asserts
the file's size is unchanged after the save is refused. The session log
is reset and gets one `new` entry, matching the two other paths that
replace the whole project, so a new project silently displacing an open
one is not possible.

The unsaved-project ruling: `POST /api/project/new` **refuses** with
`409 Conflict` when a project is open and its command stack has anything
to undo, unless the caller sends `discardChanges: true`. `409`, not
`400`, because this is a state conflict the caller can resolve, not a
malformed request (`errors.rs` documents that split). Nothing in
`AppState` tracks dirtiness and `CommandStack` exposes no save-point, so
`can_undo()` is the only available signal — which means this over-refuses
after a successful save. That direction is deliberate: CLAUDE.md ranks
data integrity above convenience, and the opposite error is
unrecoverable. The refusal itself is logged as a warning rather than
swallowed, and is recorded honestly in
[KNOWN_LIMITATIONS.md §81](KNOWN_LIMITATIONS.md).

R3, the regression test. `apps/knx-server/tests/http_catalog_to_device.rs`
runs `POST /api/project/new` → `POST /api/catalog/install` with a real
corpus `.knxprod` → `GET /api/catalog/items` → `POST /api/devices` →
`GET /api/device/{id}`, and asserts the created device carries **104**
communication objects — measured by running this test against the corpus
for this slice, not carried over from any earlier measurement. No
`.knxproj` path appears anywhere in the file. The catalog item is pinned
by id (the lexicographically first the package yields) so that a
different package silently substituting a different device is a failure
rather than a quiet pass. Without the corpus the test prints a skip
message and returns — the same loud-skip idiom
`crates/knx-productdb/tests/standalone_packages.rs` uses, including the
`KNXBENCH_PRODUCT_CORPUS` override. Three further tests in
`apps/knx-server/tests/http_project_routes.rs` cover the seed's exact
shape, the refuse/`discardChanges` pair (including that the refused edit
survives and is logged), and the cleared `store_path`.

Not closed, and stated as a limitation rather than as done: **no
frontend calls this route**. `App.tsx` renders `ProjectExplorer` only
when a tree already exists, and `ProjectExplorer.tsx` opens
`CatalogBrowser` only against a `catalogTarget` that an empty project
cannot supply. So the capability is real at the HTTP API and covered by
a test, and a *user* still cannot reach it — see
[KNOWN_LIMITATIONS.md §80](KNOWN_LIMITATIONS.md). `docs/GAP_ANALYSIS_ETS.md`'s
B1 row, which claimed closure on 2026-09-08 while its own parenthetical
("could not start a project from scratch") stayed literally true, is
corrected in place rather than quietly rewritten: it now says what T1/T2
actually delivered, what stayed open, and what this slice closes.

One correction to that limitation, found while writing it: the *only*
missing UI piece is the "New project" action itself. Nothing sets
`App.tsx`'s `tree` except `importProject`/`openProject`, and `api.ts` has
no `newProject` at all. The rest already works — the first
installation's "Unassigned" branch always carries an `AddDeviceRow` that
passes a `null` line, so an empty project with no areas and no lines can
already open the catalog browser. One button and one `api.ts` function,
not a screen.

All eight gates green, from the worktree root unless stated.
`cargo fmt --all --check` clean. `cargo clippy --workspace --all-targets
-- -D warnings` clean. `cargo test --workspace --no-fail-fast`: **1256
passed / 0 failed / 3 ignored** across **79** `test result:` lines from
one untruncated run (exit 0), against this branch's `adce9b2` baseline of
**1252 / 0 / 3 across 78** lines. The **+4** is exactly the four tests
this slice adds — one in the new
`apps/knx-server/tests/http_catalog_to_device.rs`, three appended to
`apps/knx-server/tests/http_project_routes.rs` — and the **+1**
`test result:` line is that new test *file*, not an anomaly; none
removed, none newly ignored. The run was made with `OriginalData/` and
`project_dump.json` symlinked into the worktree (both are local-only and
absent from a fresh worktree), so the corpus-dependent tests actually
executed instead of taking their skip path; the symlinks were removed
before committing and `OriginalData/` itself was never written to.
`cargo run -p xtask -- check-layering` clean — no new crate dependency,
the route lives in `apps/knx-server` and reaches only `knx-core`/
`knx-projection`, both of which it already depended on. `cargo run -p
xtask -- check-headers`: 79 files with a well-formed header, **169**
without one, ceiling **169**, unchanged — the one new file carries an
ADR-0018 header, so it joined the "with" column rather than pushing the
ceiling. `cargo deny check` clean (the same pre-existing
`advisory-not-detected` informational warnings, nothing new). Web gates
from `apps/knx-web` after `npm ci`: `npm test -- --run` **340 passed
across 31 files**, `./node_modules/.bin/tsc --noEmit` exit 0 — this
slice changed no file under `apps/knx-web`, and the green pair is the
proof of that rather than a claim about it.

## 2026-09-13 — Codex UI workbench (T21)

The web UI now has a shared three pane workbench with resizable navigation and
properties panes, keyboard accessible tree navigation, a file menu, appearance
preferences (Porcelain, Graphite, System, accent and density), and central
workspaces for buildings, topology and group addresses. Existing commands,
HTTP routes, Inspector editing, parameters, catalogue insertion, log and bus
monitor remain wired through their existing owners. The graphical views use
the generated `ProjectTree` projection and show hierarchy; they do not invent
floor-plan coordinates. Browser evidence is recorded under `/tmp/knx-ui-proof/`.

**T16's product identity, on screen (2026-09-13), branch
`codex-ui-workbench`.** The earlier note here said T16 stayed partially open
because `DeviceDetail` carried no product identity and exposing it needed a
server-side change outside the frontend slice. That change landed meanwhile
(branch `t16-device-product`, merge `036503f`), so this task spent nothing on
plumbing and everything on making the identity readable.

- **Bindings, regenerated, never hand-written.**
  `TS_RS_EXPORT_DIR=../../apps/knx-web/src/bindings cargo test -p
  knx-projection` brought `apps/knx-web/src/bindings/` up to date:
  `DeviceProductNode.ts`, `DeviceProductCatalog.ts` and `ProductResolution.ts`
  are new, `DeviceDetail.ts` gained `product: DeviceProductNode`, and nothing
  else in the directory moved.
- **`Inspector.tsx` gains `DeviceIdentity`**, rendered by `DeviceWorkspace`
  as the content of a third tab, "Produktdaten", beside communication objects
  and parameters — the three-tab strip the approved concept image
  `docs/design/2026-09-13-codex-ui-concept/01-porcelain.png` shows. (The first
  implementation put it in a section above the tab strip, per the controller's
  ruling at the time; the task report flagged the conflict with the mockup and
  the review reversed the ruling in the mockup's favour.) Adding the third tab
  meant replacing `DeviceWorkspace`'s keyboard handler, whose
  `e.key === "End" ? 1 : 1 - tab` encoded "there are exactly two tabs" three
  times over: it hardcoded the last index, it toggled, and it treated
  ArrowLeft and ArrowRight as the same key — which a left-arrow-only test
  could never catch. It is now index arithmetic over the tab array's length,
  with a test that walks both directions, wraps at both ends and checks
  `Home`/`End`. The new panel is a hidden sibling in the same
  `hidden={tab !== n}` shape as the other two, not conditional rendering:
  `ParameterPanel`'s fetch is keyed to its mount and must not restart on every
  tab switch. `product_ref` and `program_ref` print
  verbatim in the monospace face, because an engineer comparing one against a
  manufacturer package needs the exact string. A ref the project never stated
  reads "not stated in the project" in the body face — deliberately not in
  mono, so an absence never looks like a value.
- **Four resolutions, four distinct verdicts.** `RESOLUTION_KEYS` maps
  `ProductResolution` through a `Record`, not a ternary chain, so a fifth
  variant arriving in the generated union is a compile error rather than a
  silent fallthrough. `Resolved` needs no sentence (the catalogue says it);
  `NoDatabase`, `NotInDatabase` and `NoReference` each get their own, and the
  first two are worded so they cannot be mistaken for each other — no product
  database loaded at all is a different problem from a loaded database that
  does not contain this product. There is no bare "unknown" anywhere in the
  section.
- **A partly installed catalogue reads as partly installed.** Of the fourteen
  catalogue fields, three (manufacturer, product text, order number) stay
  above the disclosure; the other eleven live behind a `<details>` grouped as
  product entry / hardware / application program. Fields the database left
  `null` are omitted rather than dashed, a group whose every field is `null`
  says the database holds no values there, and the count of omitted fields is
  stated at the foot of the disclosure — so "the database is silent here" is
  distinguishable from "this view only shows six fields". A `Resolved` verdict
  with no catalogue behind it (which the server never emits, but the generated
  type permits) admits it in words instead of rendering as a resolved device
  with a suspiciously empty field list.
- **Strings and styling.** 36 new `deviceIdentity.*` keys in
  `messages/en.ts`/`messages/de.ts`, including the `omitted.one`/`omitted.other`
  plural pair; German parity is enforced by the existing `Record<MessageKey,
  string>` typing. `styles.css` gains `.device-identity` and friends, built
  from the existing custom properties, so all three themes and five accents
  follow automatically. Density does not: `--knx-control-height` and
  `--knx-cell-padding` are the only density-aware tokens and this block uses
  neither — exactly like `.device-workspace`'s own fixed `padding: 20px`
  around it. Not a regression, but not automatic either. The verdict badge carries its colour in
  border and background tint and its text in `--knx-foreground`. Measured in
  headless Chromium over the background the panel actually renders on — the
  badge's translucent fill composited onto `.device-workspace`'s opaque
  `--knx-surface`, since `.device-identity` contributes no surface of its own
  — the badge word sits at 12.66:1 to 15.32:1 in Porcelain, 9.82:1 to 13.47:1
  in Graphite and 13.94:1 to 18.90:1 in Bitcoin DeFi, across all four
  variants and all five accents (60 combinations; the worst case everywhere is
  `NotInDatabase`, whose fill is the densest). The alternative of tinting the
  word itself was rejected for a reason these numbers state plainly: accent-
  coloured text over the same background spans 4.37:1 to 6.04:1 in Porcelain
  depending on which accent is active, so the verdict's legibility would
  become a side effect of a theme preference. (An earlier revision of this
  entry quoted 10.55:1–14.69:1, measured over a recessed
  `color-mix(--knx-bg 55%, --knx-surface)` panel the block no longer has, and
  3.46:1/3.91:1 for a draft since deleted and not reproducible against this
  tree. Both withdrawn.)
- **Layout, measured rather than assumed.** The first draft put label and
  value side by side and inlined all fourteen fields; screenshotted at 1920px
  the pairs drifted apart and the block grew to roughly 400px, pushing the
  communication-object table off screen on a device that happened to be fully
  resolved. Hence stacked label-over-value pairs in an `auto-fill` grid and
  the disclosure.

Tests: `DeviceWorkspace.test.tsx` grows from 1 case to 13 — one per resolution
variant, the third tab's own panel (the identity is inside it, hidden until
selected, and leaving the tab does not remount `ParameterPanel`), arrow-key
navigation in both directions with wrapping plus `Home`/`End`, the resolved
catalogue's disclosure split and omission count, the partly-installed
catalogue's empty group, a resolution string this build does not recognise,
the omission count under `NoReference` with a catalogue attached, a tab stop
on every panel (so a panel whose content has nothing focusable is still
reachable from the tablist), and the German badge's refusal to call the state
unknown. Every fixture is fictional
(`M-00FA`, "Example Manufacturing", `EX-4210`); no real product or
installation appears. `App.test.tsx`'s device fixture and
`scripts/workbench-browser-proof.mjs`'s mock gained a `product` field — the
proof script's mock would otherwise have served a `DeviceDetail` without one
and crashed the page it was meant to photograph.

Web gates: `npm test -- --run` **363 passed across 36 files** (up from 351,
+12 in `DeviceWorkspace.test.tsx`), `tsc --noEmit` clean.
`scripts/workbench-browser-proof.mjs` now reaches the new tab with two right
arrows, opens the disclosure and photographs it, writing
`01b-porcelain-product-data.png` to the script's local output directory —
**not committed to this repository** (confirmed later, in this
document's design-image audit below); the committed proof images under
`docs/design/2026-09-13-codex-ui-proof/` still predate this filename.
Rust gates re-run
because binding generation touches `crates/knx-projection`: `cargo fmt --all
--check` and `cargo clippy --workspace --all-targets -- -D warnings` both
clean; no Rust source was modified.

What T16 still does not do: the catalogue browser is still insertion-only,
there is no link from a device to its catalogue entry, and a device's serial
number remains unreadable from the bus
([KNOWN_LIMITATIONS.md §73](KNOWN_LIMITATIONS.md#73-a-line-scan-cannot-learn-product-identity-manufacturer-or-serial-number)).

**The group-address table, multi-select moved, and the file menu's keyboard
path (2026-09-13), branch `codex-ui-workbench`.** The workbench's central
group-address view was a two-column address/name list — fewer facts than the
navigation tree beside it already showed, and no test at all. It is now a
table with range context, the resolved DPT and the linked communication
objects with their directions, and the fact that made that possible came from
the projection rather than from the screen.

- **`knx-projection` extends `GroupAddressNode`** with `range:
  Option<u32>` (the id of the containing `GroupRangeNode`), `dpts:
  Vec<String>` and `links: Vec<GroupAddressLinkNode>` (device id, device
  name and address, communication-object id, number and name, and the
  `Direction`). Which objects reference an address is a question about the
  project, not about the view, so it is answered where the project lives.
  One reverse pass over `project.devices.com_objects()` builds the index
  (O(communication objects), not O(addresses × communication objects)), and
  the three-way DPT classification reuses `knx_core`'s own
  `group_address_dpt_from`, newly `pub` with a doc comment explaining why a
  caller that has already gathered the links should not re-derive the rule.
  A group address still has no DPT of its own: `dpts` is empty when nothing
  linked states one, and holds more than one entry when linked objects
  disagree — a conflict the projection reports and never settles. Six new
  tests cover the range/DPT/link projection, the conflict, an object linked
  in both directions (one DPT, two rows), an unlinked address, a link whose
  device is missing from `project.devices`, and links staying attached to
  their own address rather than smeared across all of them.
  `cargo test -p knx-projection`: 36 passed.
- **Bindings regenerated, never hand-edited.**
  `TS_RS_EXPORT_DIR=../../apps/knx-web/src/bindings cargo test -p
  knx-projection` updated `GroupAddressNode.ts` and added
  `GroupAddressLinkNode.ts`; nothing else in the directory moved.
- **`GroupAddressTable.tsx` is the new view**: checkbox, address, name,
  range path, DPT and link counts, with a `type="search"` filter over
  address/name/DPT, a below-table links panel (participant, function,
  direction, Unlink) for the selected address, and distinct empty states
  for "this project has no group addresses" and "this filter matches
  none". Selecting a row drives the Inspector through the existing
  `onSelect` contract. Where it departs from the approved concept image
  `docs/design/2026-09-13-codex-ui-concept/02-graphite.png`, it does so on
  purpose and says so below.
- **The multi-select state machine moved, it was not copied.**
  `multiSelection.ts` now owns `useMultiSelection`, the render-order
  helpers and the ctrl/shift/plain click rules that used to live inside
  `ProjectExplorer.tsx`; the explorer and the new table both receive the
  one handler from `App`, which renders the one `BulkActionToolbar`. There
  is exactly one definition of the rules and exactly one live instance of
  the state. A shift-click spans only the rows currently visible, because
  the handler takes the caller's visible order rather than assuming the
  full tree order. The checkbox synthesises a ctrl-click through a
  structural event type, so no `MouseEvent` cast is needed to add one id.
- **Seven declared departures from `02-graphite.png`,** the approved concept
  image for this view. None is an oversight; each is a ruling, and every
  one that leaves a capability absent names where that capability goes.
  1. **DPTs render `DPST-1-1`, not the dotted `1.001`** the image shows.
     No dotted formatter exists anywhere in this repository, and
     `DptRef`'s `Display` text is the convention every other surface
     already uses. Inventing a second spelling inside a table component is
     how one screen ends up showing two names for one DPT. Changing it
     means one formatter beside `DptRef` and a sweep of every call site,
     which is a change to the domain crate's presentation contract, not to
     a table.
  2. **No "+ Gruppenadresse" primary button** in the address toolbar. The
     create affordance exists as `ProjectExplorer.tsx`'s
     `NewGroupAddressRow`, which renders `<li className="tree-new-row">` —
     tree markup that cannot be lifted into a toolbar without extracting
     the form from the list item first. Creating an address stays
     reachable in the tree; the toolbar button belongs with whichever
     stage does that extraction (it is the same extraction "+ Device"
     needs, so both should move together rather than one at a time).
  3. **The links panel's per-row "…" menu is an explicit Unlink button,
     and there is no "+ Verknüpfen" in its header.** There are no context
     or overflow menus anywhere in `apps/knx-web` (see below), so a "…"
     here would have been the first one, with nothing to be consistent
     with. Linking from the address side additionally needs a
     device/communication-object picker that does not exist; the
     Inspector's `NewGroupLinkRow` remains the one create path, and the
     picker belongs to whatever stage builds that shared component.
  4. **A Range column was added** beyond the image's breadcrumb, because
     the brief asks for range context per row and a breadcrumb only
     describes the current scope. Additive, not a removal.
  5. **The CSV control is two plain buttons, not one "CSV ⌄" dropdown.**
     The region matches the tracked
     `docs/design/2026-09-13-codex-ui-concept/README.md` ("CSV direkt bei
     Gruppenadressen"): `StructureWorkspace`'s `addressActions` slot
     carries `GroupAddressCsvButtons`, whose export and import buttons are
     rendered side by side rather than folded into a disclosure. The row
     differs from the image, which puts "CSV ⌄" in the filter/action row
     beside "+ Gruppenadresse"; here the buttons sit on the title line
     (`StructureWorkspace.tsx:72-74`'s `.workspace-heading`) with the
     filter one row below in `.address-table-toolbar`. The File-menu copies
     are kept, so no entry point is lost. A disclosure of this shape does
     exist — `App.tsx:459-461`'s `<details className="file-menu">` already
     wraps these same buttons behind a label and a `⌄` — but there is no
     reusable menu-button primitive in `apps/knx-web`, and promoting the
     File menu's one-off into the first one, for two buttons, would
     prejudge how every later overflow menu behaves.
  6. **The Verknüpfungen column reads "1 sending · 1 receiving" rather
     than the image's bare count.** The direction is the fact an installer
     needs, the count alone hides which way a link points, and the
     projection now carries both.
  7. **No column sorting and no sort caret.** The image shows a "▲" on the
     Adresse header; `GroupAddressTable.tsx` has no `sort` or `aria-sort`
     at all. Rows render in the order the project stores them
     (`Installation.group_addresses` is a `Vec`, so: insertion order, which
     for an imported project is the file's order and not a guaranteed
     sort). Clicking a header does nothing, nothing announces a sort
     state, and the caret would therefore be a promise the table does not
     keep. **Deferred to the later verification stage** of this UI series —
     the one that checks mouse and keyboard operation, theme and motion
     switching, error states and real on-screen rendering — for three
     reasons: sorting is a table-wide concern that must also cover the
     device and topology tables, it needs `aria-sort` plus an activatable
     header for the keyboard path, and it has to state what it does to a
     shift-click span. Doing it for one table in isolation is how three
     tables end up sorting differently.
- **`groupAddressView.ts`** holds the display helpers both the table and
  the Inspector need — `directionLabel` (moved out of `Inspector.tsx`,
  still one definition), `linkDirectionCounts`, `dptText`,
  `hasDptConflict`, `rangePath` and `rangeWithDescendants`. The
  group-address Inspector gained the same DPT and link-direction facts, so
  the table and the properties pane cannot disagree about one address.
- **`ProjectDiffPanel.tsx` had a real keyboard defect**: the comparison
  report appeared without focus moving into it, and Escape inside it
  closed the surrounding File menu instead of the report. The panel now
  takes focus when it opens, stops Escape from propagating and returns
  focus to the Compare button — so the first Escape closes the report and
  the second closes the menu, innermost first.
- **Two `CatalogBrowser` overlays cannot stack — but both state machines
  are live.** `App.tsx` and `ProjectExplorer.tsx` own separate
  `catalogTarget` state, and **both** have a working UI trigger: the
  workspace's catalogue button for the first, and `AddDeviceRow`'s "+ Add
  device" button in the tree (`ProjectExplorer.tsx`, rendered under the
  first installation's lines and its Unassigned bucket, calling
  `setCatalogTarget`) for the second. Neither is dead code; an earlier
  revision of this entry claimed the second was unreachable, which was
  wrong and is withdrawn.
  What prevents stacking is the shared `Overlay` shell, covering that
  trigger along with every other one. `.search-overlay` is
  `position: fixed; inset: 0; z-index: 10` over the whole viewport, and no
  ancestor of either trigger creates a competing stacking context
  (`.workbench-pane` is `position: relative` with auto z-index,
  `.workbench-toolbar` sits at `z-index: 5`), so the backdrop receives the
  pointer events that would open the second overlay. `Overlay` also moves
  focus into the panel and traps `Tab` — and the panel's search input is
  never unmounted, so the trap always has somewhere to hold focus — which
  closes the keyboard path to the same triggers. The panel is
  `aria-modal="true"`, and no global shortcut and no `CommandContext` entry
  opens the catalogue. Both states were therefore left exactly as they are:
  the duplication is real, it is a tidiness question rather than a defect,
  and unifying it belongs with whoever next has a reason to touch the
  catalogue flow itself.
- **No context menus exist anywhere in `apps/knx-web`** — `onContextMenu`
  and `contextmenu` appear nowhere in the tree — so "context menus
  consistent where they exist" is satisfied vacuously, not by work. The
  CLAUDE.md UX wish list still asks for them; that remains open.

Tests: `GroupAddressTable.test.tsx` is new (10 tests) and covers range path
plus DPT plus "1 sending · 1 receiving", the conflict rendering both DPTs,
selection driving the links panel, a link whose device is missing, Unlink
calling `unlinkComObject`, the checkboxes driving the real
`BulkActionToolbar` through to `batchDeleteGroupAddresses`, filtering by
address/name/DPT, both empty states, and the range scope. The shift-click
test spans a *non-contiguous* visible set — scoped to one range, ids 30, 31
and 33 are on screen with 32 hidden between them — and asserts the batch
delete is called with exactly those three ids. A span over a contiguous
visible set proves nothing, because the full-order fallback produces the
same answer; removing the `visibleOrder` argument from the call site now
makes this test fail with four selected instead of three (verified by
doing it).
`App.test.tsx` gains the keyboard walk through the File menu: the summary is
focusable, activating it lists all eight entries (open, open `.knxdb`, save
as, export `.knxproj`, CSV export, CSV import, documentation export, compare)
with no negative tab index and only the legitimately unavailable
`.knxproj` export disabled, Escape closes the menu and restores focus to the
summary, and the layered Escape on the comparison report is asserted step by
step. `ProjectExplorer.test.tsx` and `StructureWorkspace.test.tsx` wrap the
shared hook in a small harness rather than restating its rules.

Web gates: `npm test -- --run` **377 passed across 37 files** (up from 363
across 36), `tsc --noEmit` clean. Fixtures are fictional throughout — made-up
`1/0/x` group addresses and `1.1.11`/`1.1.13` device addresses, no real
product, device name or occupied address anywhere.

**The workbench coverage matrix, verified against the code (2026-09-13),
branch `codex-ui-workbench`.** The twelve-row matrix that steered this UI
rebuild was written before the shell existed and lives in a gitignored
working log, so it could neither be trusted nor cited. It is re-checked
here against the tree as it stands, one file and line per row, and kept in
`docs/` where a merge can carry it. "Reachable" below means reachable by
some device-independent path, not merely present in the DOM.

| # | Capability | Verdict | Evidence |
| --- | --- | --- | --- |
| 1 | Native/ETS open, save, save as, export | Holds | `apps/knx-web/src/App.tsx:462-468` (File menu), `:493` (Save), `commandRegistry.ts:56-79` (same four as commands) |
| 2 | CSV, documentation export, project diff | Holds | `App.tsx:470` (CSV), `:477` (documentation), `:483` (compare), `:528` (CSV again as the address workspace's actions) |
| 3 | Import errors and warnings | Holds | `Dashboard.tsx:35-46` (counts), `App.tsx:506` (persistent notice), `:524` (log), `:539` (toasts) |
| 4 | Buildings, topology, CRUD | Holds | `App.tsx:512` (navigation), `:525` (`StructureWorkspace`), `:535` (inspector), `StructureWorkspace.tsx:73` (create) |
| 5 | Catalogue, install, device creation | Holds | `App.tsx:513` (navigation entry), `:540` (`CatalogBrowser`), `StructureWorkspace.tsx:73` (contextual create) |
| 6 | Addresses, ranges, DPT, links, flags | Holds | `App.tsx:512` (navigation), `StructureWorkspace.tsx:98` (`GroupAddressTable`), `App.tsx:535` (inspector) |
| 7 | Parameters, diagnostics, module writes | Holds | `App.tsx:531` (`DeviceWorkspace`), `Inspector.tsx:680` (tabs), `:735` (`ParameterPanel`) |
| 8 | Multi-select, bulk, undo, search, palette | Holds | `App.tsx:488-489` (undo/redo), `:491` (search), `:492` (palette), `:499` (`BulkActionToolbar`) |
| 9 | Log, bus monitor, compose | **Was false** — both halves | see below |
| 10 | UI language, product language, packs | Holds | `SettingsPanel.tsx:291` (product data), `:312` (UI), `App.tsx:494`/`:520` (two ways in), `commandRegistry.ts:121` (a third) |
| 11 | Appearance | Holds | `SettingsPanel.tsx:233-256` (theme, accent, density), `theme.ts:5` (`system` is a real entry), `styles.css:1082-1083` (the two density tokens) |
| 12 | Additional diagnostic window | Holds — browser and Tauri desktop, both run 2026-09-13 | `main.tsx:23-26` (one view switch, companion or editor), `DiagnosticsCompanion.tsx` (monitor and log only), `diagnosticsWindow.ts:110` (`openCompanionWindow`), `App.tsx:519` (the button), `busContext.ts:184` (`contextLock`), `capabilities/diagnostics.json` (desktop grants), [KNOWN_LIMITATIONS §82](KNOWN_LIMITATIONS.md#82-the-diagnostics-companions-stale-lock-sees-one-browser-profiles-own-windows-and-nothing-else) (what the lock cannot see) |

Row 9 was true when it was written and false when it was checked, in both
halves.

*Reachability.* Before the shell, Log and Bus monitor were always-visible
toolbar buttons. Afterwards their only entry points were
`App.tsx:517-518`, inside the `diagnostic-navigation` nav at `:516`,
inside the `{navigationOpen && …}` guard at `:510` — so the navigation
toggle at `:497` could remove the only way to reach either panel, and
`commandRegistry.ts` had no entry for them. Settings survived by accident,
via the toolbar gear at `App.tsx:494`. Per the standing rule that new
actions stay reachable through the same validated commands regardless of
input device, `open-log`, `open-bus-monitor` and `open-settings` now exist
(`commandRegistry.ts:109-126`), all three enabled without an open project
because both panels work without one. Three `App.test.tsx` tests collapse
the pane first and then drive the palette. T-UI-06 added a fourth for the
same reason (`commandRegistry.ts:130-135`, `open-diagnostics-window`): the
companion window's only button sits in that same collapsible pane.

*"Diagnostic workspaces".* Neither panel was one: both opened with a bare
`<h2>` while every other centre-pane view uses `.workspace-heading` with
an eyebrow and an `<h1>`. Both now match (`LogPanel.tsx:77`,
`BusMonitorPanel.tsx:473`), with the severity filters and the connect
controls as their action clusters, and the monitor's eyebrow naming its
only transport — `knx-server`'s bus layer is tunnelling-only, no discovery
and no routing (`apps/knx-server/src/bus.rs:5-13`).

Four token escapes inside those three panels were fixed in passing, each a
capability that existed and did not reach the screen:

- `.bus-monitor-table th, td` declared its own cell padding at a
  specificity that beat the shell's `th, td { padding:
  var(--knx-cell-padding); }`, so Compact/Comfortable moved every table in
  the application except the telegram one.
- Eight rules covering ten secondary-text classes dimmed themselves with
  `opacity` instead of `var(--knx-muted)`, which no theme can retune.
  Contrast *falls* with the token (Porcelain 7.98:1 to 5.51:1 against
  `--knx-surface`) and still clears WCAG AA; this is token participation,
  not a contrast improvement.
- `bus-monitor-row-new` — design D34's "arrived in the latest poll"
  emphasis — was set on rows and asserted by nine test expectations, while
  the rule that drew it had been deleted with the rest of the per-telegram
  animation. It is drawn again as a static accent rail, because telegrams
  still must not animate.
- The bus compose form's "Project Undo cannot reverse this action" notice
  had no rule at all and rendered as body text. It is a warning again.

The settings overlay's selects carried `font-size: 1rem` and `padding:
0.4rem` — a 16px control in a 13px shell, the one place the type scale did
not reach. Both are gone; the shared `input, select, textarea` rule, whose
`min-height` the density setting drives, applies instead. Catalogue and
Search selects share that rule and come along.

A new guard, `apps/knx-web/src/diagnosticShell.test.ts`, fails the suite on
either defect shape: a panel class name with no rule in `styles.css` (with
an explicit allowlist for the query hooks that draw nothing on purpose),
and a cell-padding declaration that outranks the density tokens. Its
ADR-0018 header is `/** One sentence. */`: `check-headers`
(`xtask/src/headers.rs:107-116`) recognises no line-comment form for
TypeScript, so a `// …` first line counts as no header at all and trips
the ratchet.

One thing was deliberately left alone. `DeviceWorkspace`'s heading
(`Inspector.tsx:679`) uses `.workspace-heading` with an `<h2>` and no
eyebrow rather than the eyebrow/`<h1>` shape — cosmetic, and it belongs to
the device slice, not this one. Row 12's companion diagnostic window was
left to its own task, which is the section below.

Gates, all eight green from one run each: `cargo fmt --all --check`,
`cargo clippy --workspace --all-targets -- -D warnings`,
`cargo test --workspace --no-fail-fast` (**1187 passed, 0 failed, 3
ignored across 78 `test result:` lines** — unchanged; nothing here touches
`crates/`), `xtask check-layering`, `xtask check-headers` (**92 files with
a well-formed header, 169 without, ceiling 169** — the one new file brings
its own), `cargo deny check`, and in `apps/knx-web` `npm test -- --run`
(**383 passed across 38 files**, up from 377 across 37: three
palette-reachability tests, one registry test and two guard tests in the
new file) plus `tsc --noEmit`.

**T-UI-06 — the diagnostics companion window (2026-09-13, branch
`codex-ui-workbench`).** Row 12 of the matrix above now holds. A second
window hosts the bus monitor and the session log and nothing else; the
project is edited in exactly one window, as before.

*One editing workspace.* `main.tsx:23-26` picks between `<App />` and
`<DiagnosticsCompanion />` from one query parameter (`?view=diagnostics`),
so the companion is the same bundle at a different entry point rather than
a second application. `DiagnosticsCompanion.tsx` imports only
`BusMonitorPanel`, `LogPanel`, `busContext`, `diagnosticsWindow`, `i18n`
and `react` — `DiagnosticsCompanion.test.tsx` asserts that import list
against the module's own source, because an absence that nothing checks
stops being true the first time someone adds "just one small button".

*No project mutation, no project undo.* The Ctrl+Z/Ctrl+Shift+Z handler
lives on `App.tsx`'s `window` listener, which the companion never mounts;
a test presses both combinations against the live companion and asserts
`api.undo`/`api.redo` are never called. The only write it can reach is the
bus compose form's, which was already a bus write and already says project
Undo cannot reverse it.

*One shared bus session.* The companion starts nothing: `BusMonitorPanel`
asks `GET /api/bus/monitor/telegrams` on mount and attaches to whatever
session exists (`apps/knx-server/src/bus_routes.rs:272-311` answers `404`
when there is none), and closing the window runs no teardown, so the
session outlives it. Both are tested. The server's one-session rule is
unchanged: a second `POST /start` still gets a `409` naming the existing
session (`bus_routes.rs:95-109`).

*The stale lock.* `apps/knx-server/src/bus.rs:601-618` freezes a
`GroupAddressContext` — group-address style, names, DPTs — when a session
starts, and never re-resolves it; that snapshot decodes every telegram and
resolves every write's DPT (`bus.rs:1135-1141`). `busContext.ts`
fingerprints exactly those three facts, records the fingerprint when a
session starts, and compares on every poll tick. The verdict is
three-valued: `synced`, `stale` (the project moved — the decoded columns
are struck through, and the compose form is disabled with an explanation)
and `unverified` (this profile did not record this session's start, so
nothing can be confirmed either way — said out loud, sending left
enabled). What it cannot see is [KNOWN_LIMITATIONS §82](KNOWN_LIMITATIONS.md#82-the-diagnostics-companions-stale-lock-sees-one-browser-profiles-own-windows-and-nothing-else).

*Platforms.* Verified in both. Headless Chromium against the Vite dev
server renders the companion shell at `?view=diagnostics` and the editor
at `/`. On the Tauri desktop shell the command opens a real second native
window titled "KNXBench — Diagnostics", a second invocation focuses it
instead of creating a third, and its "Back to main window" button returns
focus to `main`. That needed two capability changes:
`core:webview:allow-create-webview-window` and `core:window:allow-set-focus`
on the main window, plus a new least-privilege
`capabilities/diagnostics.json` scoped to the `diagnostics` window with
`core:window:allow-get-all-windows` and `core:window:allow-set-focus` only.
No KNX bus was touched: no gateway was connected in either run.

Gates, all eight green from one run each: `cargo fmt --all --check`,
`cargo clippy --workspace --all-targets -- -D warnings`,
`cargo test --workspace --no-fail-fast` (**1187 passed, 0 failed, 3
ignored across 78 `test result:` lines** — unchanged; this stage adds no
Rust), `xtask check-layering`, `xtask check-headers` (**98 files with a
well-formed header, 169 without, ceiling 169** — six new files, six new
headers), `cargo deny check`, and in `apps/knx-web` `npm test -- --run`
(**434 passed across 41 files**, up from 383 across 38: three new test
files worth 45 tests, five stale-lock tests in `BusMonitorPanel.test.tsx`
and one in `BusComposeForm.test.tsx`) plus `tsc --noEmit`.

**T-UI-07 — the series closes: three comments that claimed more than they
could prove (2026-09-13, branch `codex-ui-workbench`).** No feature landed
here. This stage answered five findings from the T-UI-06 review, and the
most useful result was that one of them was wrong.

*A depth-1 import list proves nothing about depth 3.*
`DiagnosticsCompanion.tsx`'s header claimed its own import list guaranteed
the companion cannot reach editing code, and that "anything reachable from
this module is reachable from the companion window". The implication runs
backwards, and the premise is false as well: this module imports
`./diagnosticsWindow`, which imports `isTauri` from `./filePicker`, which
imports `./FsPicker`, which `POST`s to `/api/fs/upload`. The comment now
states three properties that are each true and each asserted. (1) This file
names no editing surface directly — the old depth-1 assertion survives,
recommented as the tripwire it always was. (2) Across the whole transitive
value-import graph, every `api` call is a bus or diagnostics call and the
only non-`GET` raw `fetch`es sit in functions this window never calls; the
test walks the graph from source and pins three exact lists — **15 modules,
7 `api` exports called, 2 mutating fetch targets**. Fifteen, not the thirty
the review counted: the difference is exactly the fifteen `bindings/*.ts`
files, reached only by `import type` and erased before anything runs. All
three lists are re-measured by
`./node_modules/.bin/vitest run src/DiagnosticsCompanion.test.tsx`, which
fails with the three lists printed whenever any of them moves. (3) Mounted
and left alone the companion calls exactly one `api` export, the telegram
poll; opening the Log tab adds the session-log read and nothing else. That
last one is asserted as the *set* of exports called, not as three named
absences, so a new mutator fails the test instead of being forgotten. The
review's warning that an honest transitive test must fail today was too
pessimistic: it fails only if it
demands the graph contain no mutating code, which is not the property worth
having. What the window never *calls* is.

*The fingerprint separator that was already there.* The review reported that
`fingerprintProjectContext` concatenates address, name and DPTs without a
separator, so address `1/1/1` name `0Foo` collides with address `1/1/10`
name `Foo`. It does not. `busContext.ts` has used U+0001 between fields and
U+0002 between records since the feature landed, written as literal
non-printing bytes that every display layer — editor, `git diff`, code
review, two successive reviewers — silently swallowed. Reproduce with
`LC_ALL=C grep -n $'[\x01\x02]' apps/knx-web/src/busContext.ts | cat -v`,
which prints `^A` and `^B`. The bytes are now written as `\u0001` and
`\u0002` escapes: byte-identical output, no stored fingerprint invalidated,
and the source finally says what it does. Two tests pin the property rather
than the spelling, one per separator, the record-separator one holding the
address count fixed so the `${count}-` prefix cannot pass it by accident.
The hashing scheme was left alone deliberately — a length-prefixed encoding
would fix a collision that does not exist and would make every stored
fingerprint read `unverified` once, which trades Data Integrity for nothing.

*Two blind spots the lock really has.* Both are now in
[KNOWN_LIMITATIONS §82](KNOWN_LIMITATIONS.md#82-the-diagnostics-companions-stale-lock-sees-one-browser-profiles-own-windows-and-nothing-else),
with the residual that survives the escapes. The digest is 32-bit FNV-1a:
`"synced"` means "almost certainly unchanged", never "provably unchanged",
and a crafted project could collide on purpose. Neither is defended against,
because the lock is a decoding-staleness hint and the blast radius of a miss
is one mislabelled telegram, not a bad write. The residual is a group
address whose *name* contains U+0001 or U+0002 — impossible from a
`.knxproj`, because XML 1.0 §2.2's `Char` production admits no C0 control
character except tab, LF and CR, and no keyboard produces one.

*One edit path really does skip the publish (closed by T3, below).* `App.tsx` claimed no edit path
can forget to publish the project context. `api.setParameterValue` forgets:
`domain.rs` runs `apply(state, cmd)` for `Command::SetParameterValue`, so
the project moves server-side, but the response is a `ParameterPanelDto`, so
`App.tsx` never calls `setTree`, the `useEffect` on `tree` never fires, and
the fingerprint stays where it was. Verified here rather than taken from the
report: `Command::SetParameterValue` writes only `installation.parameters`
through `upsert_parameter_value`, while `resolve_group_address_dpt`
(`crates/knx-core/src/dpt/resolve.rs`) reads only com-object links and
resolved DPT values, and `GroupAddressNode.dpts` — the third fingerprint
input — comes from the same rule over the same com objects. The two sets do
not intersect, so the hole is harmless *today*, which is exactly the kind of
fact that stops being true quietly. The comment now says "no edit path that
lands in `tree`", records that the old one was false when it was written,
and `resolve_group_address_dpt` gained a "Before you widen the inputs"
section: whoever makes a parameter value influence a com object's DPT, links
or activity will read it before they can finish, and will find out that they
have just made the bus monitor report `"synced"` over a decode that changed.

*Nine images, read rather than assumed.* Every `.png` under
`docs/design/2026-09-13-codex-ui-concept/` and
`docs/design/2026-09-13-codex-ui-proof/` was opened and examined. The
concept directory holds three: a Porcelain building/device workspace, a
Graphite group-address workspace and a companion-window bus monitor. The
proof directory holds six produced by `workbench-browser-proof.mjs`, whose
fixtures are invented in the script's own source and labelled
`Beispieldaten · kein reales Gerät` on screen. No real device name, no
manufacturer inventory and no occupied-address list appears in any of them.
The gateway strings in the monitor images are fictitious input placeholders;
the controller has ruled they stay, and new material uses the RFC 5737
placeholder `192.0.2.1`, which is what `BusMonitorPanel.tsx` renders today.
What the audit did find is staleness nobody had written down: the committed
PNGs come from one run at the branch's base commit, the script has gained
two commits since, it now writes `01b-porcelain-product-data.png` which was
never committed, and `01-porcelain.png` shows a two-tab device inspector
where the application now has three. That is recorded in the proof
directory's README, together with the fact that the dev server binds
`[::1]:1420` and *only* that — `vite.config.ts` sets `strictPort: true` and
no `host`, `ss -ltn | grep 1420` reports one IPv6 listener, and a
`127.0.0.1` URL is refused with `curl` exit 7 and an empty body, which one
verification pass mistook for a server answering with nothing. The run block
there previously named `http://127.0.0.1:1427`, a port matching neither the
dev server (1420) nor `vite preview`'s default (4173).

*Numbers carry their commands now.* `GAP_ANALYSIS_ETS.md` D12 proposed that
any number written into prose be written beside the command that measured
it. Adopted here, and applied retroactively to exactly one place: D12's own
row, which now carries `for p in 'title=' 'aria-label=' 'aria-describedby='; do grep -rho "$p" apps/knx-web/src --include='*.tsx' --exclude='*.test.tsx' | wc -l; done`
beside its 8 / 33 / 4, and notes that dropping the `--exclude` doubles
`aria-label` to 64. It was not applied to the historical entries above:
their counts are per-entry records of what was true on the day, not claims
about the tree today, and rewriting them would turn a log into a report. The
convention binds new prose.

Gates, all eight green from one run each: `cargo fmt --all --check`,
`cargo clippy --workspace --all-targets -- -D warnings`,
`cargo test --workspace --no-fail-fast` (**1187 passed, 0 failed, 3 ignored
across 78 `test result:` lines**, summed with `awk '/test result:/{p+=$4;f+=$6;i+=$8;n++} END{print p,f,i,n}'`
over one untruncated log — unchanged; the only Rust change is a doc
comment), `xtask check-layering`, `xtask check-headers` (**99 files with a
well-formed header, 168 without, ceiling 169** — unchanged; no file was
added, and the ceiling stays at 169 because ratcheting it is a decision for
whoever merges this), `cargo deny check`, and in `apps/knx-web`
`npm test -- --run` (**438 passed across 41 files**, up from 434: two
separator tests in `busContext.test.ts` and two in
`DiagnosticsCompanion.test.tsx`, being the import-graph test and the split
of one runtime test into an untouched-lifecycle case and a log-tab case)
plus `./node_modules/.bin/tsc --noEmit`, which needed `existsSync` added to
the hand-written `src/node-builtins.d.ts` shim — the package deliberately
carries no `@types/node`, because `tsc && vite build` type-checks the tests
alongside the application and the full Node surface would let a component
import `node:fs` unnoticed.

## 2026-09-13 — The from-scratch project launcher (branch `launcher-new-project`)

`POST /api/project/new` landed on 2026-09-08 with tests and no caller. The
frontend had two welcome-screen buttons, both meaning "open a file you
already have", so the one capability this cycle exists for — install a device
from a manufacturer product database with no ETS project anywhere — was
reachable only with `curl`. It is now reachable with a mouse.

- **`groupAddressStyle` on the creation route.** `NewProjectBody` gained an
  optional field, threaded through `new_project_impl` into `ProjectInfo`. An
  absent style keeps `Project::new`'s `ThreeLevel`, so every existing caller
  is unchanged; an *unrecognised* one is a `400` naming the three it could
  have been, not a fall back to three-level. Both ETS import
  (`knx-etsproj/src/map.rs`) and the store (`knx-store/src/project.rs`) do
  fall back, correctly — they are reading documents that already exist. This
  route creates one, and the style is effectively permanent once addresses
  exist, so guessing here would be discovered a hundred group addresses
  later. Three tests in `apps/knx-server/tests/http_project_routes.rs`: a
  two-level project accepts `4/612` and returns it formatted that way, a
  default project rejects `4/612` and accepts `4/2/100`, and `"FourLevel"` is
  refused with the value and the alternatives in the message, leaving no
  half-made project behind.
- **`api.ts` gained `newProject` and `isUnsavedChangesConflict`.**
  `discardChanges` is always written to the wire explicitly, `false` when
  nobody asked, so "did this request offer to destroy anything?" is
  answerable from the body alone. The 409 is a named predicate rather than a
  bare status comparison scattered across call sites.
- **`NewProjectDialog.tsx`**, on the shared `Overlay` shell: project name,
  installation name, project language and group address style, every field
  pre-filled with something valid so the whole dialog is one Enter away from
  a project. The two defaults the backend deliberately refuses to invent
  (`new_project_impl`'s own doc comment says the localized label belongs to
  the catalogue) live in `messages/en.ts` and `de.ts`, 23 new keys in each
  (`toolbar.newProject` plus 22 under `newProject.`).
  The language field is validated for BCP-47 well-formedness with
  `languagePack.ts`'s existing `isWellFormedBcp47Tag` — well-formedness only,
  never a registry, since `knx_core::Language` does not validate at all.
- **The 409 is a question, not a toast.** A refusal turns the dialog into a
  prompt naming what is at stake, with the server's own sentence kept
  underneath the translated explanation. `discardChanges: true` leaves the
  frontend from exactly one place: the button a human pressed after reading
  it. "Keep editing" closes the dialog and leaves the open project's edits
  where they were. A second Enter while the prompt is up is swallowed rather
  than re-earning the same 409.
- **Three welcome-screen buttons now**, the new one first and styled as the
  primary action — it is the only one that does not require the user to
  already own a file. Same action in the File menu and as `COMMANDS[0]` in
  the command palette, which shifted every palette index by one and moved
  `CommandPalette.test.tsx`'s ArrowDown walk from four presses to five; the
  walk still starts before the disabled rows and still lands on Search.

Gates, all eight green from one run each: `cargo fmt --all --check`,
`cargo clippy --workspace --all-targets -- -D warnings`,
`cargo test --workspace --no-fail-fast` (**1266 passed, 0 failed, 3 ignored
across 79 `test result:` lines**, summed with `awk '/test result:/{p+=$4;f+=$6;i+=$8;n++} END{print p,f,i,n}'`
over one untruncated log — up 3 from 1263, the three new route tests),
`xtask check-layering`, `xtask check-headers` (**102 files with a well-formed
header, 168 without, ceiling 168** — both new files carry one, so the count
without moved not at all), `cargo deny check`, and in `apps/knx-web`
`npm test -- --run` (**455 passed across 42 files**, up from 438 across 41:
nine dialog tests in the new `NewProjectDialog.test.tsx`, four welcome-screen
wiring tests in `App.test.tsx`, three wire-contract tests in `api.test.ts`,
one palette-enablement test in `commandRegistry.test.ts`) plus
`./node_modules/.bin/tsc --noEmit`.

What is still not true: nobody has clicked any of this in a browser. Every
frontend test here renders against a mocked `./api`. KNOWN_LIMITATIONS.md §83
records that, and §84 records the residue the style field leaves behind — a
project's group address style is now chosen at creation and thereafter
invisible, because `ProjectTree` has no field for it.

## 2026-09-13 — Two parked findings closed: unverified `.signature`, same-file id collisions (productdb-parked)

Two review findings parked against `crates/knx-productdb` were investigated
and closed, neither by implementing what they might at first look like they
ask for.

**`.signature` members.** Confirmed nothing in this crate, `apps/knx-server`
or `apps/knx-web` ever reads a `role = 'Signature'` package member back to
verify it — it is stored verbatim in `source_file`, same as an unrecognised
file. The accessible KNX Standard corpus documents a different "signature"
(a registration change-detection hash, itself also stored unverified) but
nothing about a detached `.signature` file's format or key. No verification
was implemented — there is nothing to verify *against*. Added an
explanatory comment at the role assignment, a pinning test
(`signature_members_are_stored_verbatim_and_never_verified` in
`crates/knx-productdb/tests/standalone_packages.rs`), and
KNOWN_LIMITATIONS.md §85.

**First-writer-wins id collisions.** Two distinct gaps, not one:

1. `datapoint_type` had *zero* conflict tracking — no `source_sha256`
   column to compare against at all. Measured against the real corpus
   (`OriginalData/ProductDatabases/`, copied to a scratch directory and
   deleted afterward, never modified in place): installing the corpus's 4
   distinct packages in sequence drops 0, 234, 354 and 234
   `DatapointType`/`DatapointSubtype` rows respectively — **822 silent
   drops total**, because every package restates the full KNX-standard DPT
   catalogue. Fixed with a small, contained counter:
   `MasterIngest::dropped_datapoint_types` → `InstallReport
   .dropped_datapoint_types` → a new `package.dropped_datapoint_type_count`
   column (schema v6, `migrate_v5_to_v6`) → printed by `knx-cli`'s
   `install` command.
2. The existing `first_winner` helper (duplicated in `hardware.rs` and
   `catalog.rs`) compares a colliding id's `source_sha256`, which is
   constant across one file, so two elements sharing an `@Id` **within the
   same file** never trigger the existing `IdConflict` reporting — the
   second element is dropped with nothing recorded. Not observed in the
   small real corpus (no file there declares a duplicate id), so pinned
   with synthetic fixtures instead:
   `two_hardware_elements_sharing_an_id_in_one_file_conflict_silently`
   (`crates/knx-productdb/src/parse/hardware.rs`, which also demonstrates a
   second consequence — the surviving `Hardware` row's `Product` children
   silently reparent, since `Product`'s own `first_winner` check doesn't
   verify its parent row was freshly inserted) and
   `two_catalog_items_sharing_an_id_in_one_file_conflict_silently`
   (`crates/knx-productdb/src/parse/catalog.rs`). Not fixed: closing this
   needs `first_winner` to hash something finer than "the whole file",
   which is a real behavioural change, not a counter — documented as
   KNOWN_LIMITATIONS.md §86 instead.

Gates run from the worktree root (`.worktrees/productdb-parked`, branch
`productdb-parked`): `cargo fmt --all --check`, `cargo clippy --workspace
--all-targets -- -D warnings`, `cargo test --workspace --no-fail-fast`,
`cargo run -p xtask -- check-layering`, `cargo run -p xtask --
check-headers`, `cargo deny check` — exit codes and test totals recorded in
this session's dispatch report, not reproduced here since they belong to a
single point in time on a branch, not a durable project fact. The two
`apps/knx-web` gates (`tsc`, `vitest`) do not apply: no file under
`apps/knx-web` was touched.

Out of scope, left as-is: the `first_winner` helper's duplication between
`hardware.rs` and `catalog.rs` (copy-pasted, not shared); `master.rs`'s
`manufacturer` table using `ON CONFLICT(id) DO UPDATE` (last-writer-wins,
a different mechanism from `first_winner`, not touched).

## 2026-09-13 — two closing fixes, finished by hand

Both of these were dispatched as subagent tasks and both subagents were
killed by an API rate limit before they could commit. Their work was
complete on disk, so it was gated and committed from the coordinating
session rather than re-dispatched.

**`bool_flag` accepts all four `xs:boolean` spellings**
(`crates/knx-productdb/src/parse/mod.rs`). It accepted `"1"` and `"0"` and
mapped everything else to `None`, which meant `Linkable="false"` — the
spelling schema 20 and 21 use — was read as "attribute absent" and stored
as `NULL`. Measured before the fix: `linkable` was `NULL` for all six
ingested programs. The same helper would have swallowed six of the 30
`LegacyAllowPartialDownloadIfAp2Mismatch` values once T30 wires those up.
`"true"` and `"false"` now join `"1"` and `"0"`, and nothing else does:
`"True"`, `"yes"` and `"-1"` still return `None`, because a spelling
`xs:boolean` does not define is not a boolean this parser is entitled to
guess at. Three new tests, two in `parse/mod.rs` covering the four
accepted and three rejected spellings, one in `parse/program.rs` pinning
the behaviour at ingest level. Existing databases keep their `NULL`s —
KNOWN_LIMITATIONS.md §87 says so and says why.

**`knx-testsupport`, a dev-only crate that owns the fixture paths**
(`crates/knx-testsupport`). The maintainer's corpus export filenames were
hard-coded in 27 places across six crates and apps, so a contributor with a
differently-named corpus had 27 files to edit. The new crate exposes
`reference_ets4_path`, `reference_ets6_path`, `reference_kv_schema21_path`
and `corpus_available`, each path overridable by an environment variable,
and has zero dependencies, so the layering gate has no opinion to form
about it. It appears only under `[dev-dependencies]`, in eight manifests.
Five literal mentions survive on purpose: they assert a parsed project
*name* or a filename a report prints — values under test, not paths being
built. `corpus_available` checks all three projects rather than only the
ETS4 one, so an override of a single variable cannot walk a guarded test
into a panic.

## 2026-09-13 — closes the `setParameterValue` publish hole (T3, goal.md §6 item 6)

`api.setParameterValue` mutated the project server-side — `domain.rs`'s
`set_parameter_value_impl` runs `apply(state, cmd)`, a real undoable
`Command::SetParameterValue`, before it returns — but answered with a
`ParameterPanelDto`, never a `ProjectTree`, so `App.tsx`'s tree-publish
effect never fired and nothing republished. Both places that carried a
comment saying so — `App.tsx` and `ParameterPanel.tsx` — are corrected in
place rather than deleted, since most of what each said (the fingerprint's
own indifference to parameters, and the residual DPT-influence risk) is
still true.

`ParameterPanel` gained a required `onValueApplied` callback, threaded
through `ParameterSectionView`/`ParameterFieldRow` and fired once per
successfully committed field. Required rather than optional: an optional
prop lets a future third mount site forget it and silently reopen the
hole this callback closes; `tsc` now refuses that for its own mount.
`DeviceWorkspace` (`Inspector.tsx`) originally wired it to overlay
`can_undo: true, can_redo: false` onto the `tree` prop it already held
and handed the result to `onApplied` — exact, not invented, because
`CommandStack::do_command` (`crates/knx-core/src/command.rs`) always
pushes onto `undo` and clears `redo` on a successful command. That made
`App.tsx`'s tree-publish effect fire on every parameter edit, the same
as any other command. T3 fix round 1 below replaces this overlay with
the server's own tree; the *fingerprint* limitation described next was
and remains unaffected by that change. The *fingerprint* itself still
does not move, because `fingerprintProjectContext` deliberately excludes
parameters — KNOWN_LIMITATIONS.md §82 item 5 is rewritten to say exactly
that, rather than "never republishes anything", which stopped being true.

New test, `DeviceWorkspace.test.tsx`'s "publishes a committed parameter
edit to onApplied, not just to api.setParameterValue": asserts `onApplied`
is called with the overlaid tree, not merely that `api.setParameterValue`
was called — the latter passed on the unfixed code too, since the call
was never the missing half. Verified failing before the fix (`onApplied`
called 0 times) by stashing the three source changes and rerunning it
alone, then verified passing once they were restored. T3 fix round 1
below changes what this test asserts, without weakening it.

Gates: `npx tsc --noEmit` (`apps/knx-web`) exit 0; `npx vitest run`
(`apps/knx-web`) exit 0, 465 tests across 42 files (up from 464/42 — one
test added, none removed). No Rust file was touched, so the Rust gates do
not apply. The other five parked findings in goal.md §6 and item 7's
screenshot regeneration are untouched — separate tasks, separate owners.

#### T3 fix round 1 (2026-09-14)

A prior agent died mid-fix leaving three files with uncommitted, unreviewed
changes; those were reviewed hunk-by-hunk against the fix-round-1 findings
rather than trusted outright. Six items:

1. This file's own doc-comment counterpart in
   `crates/knx-core/src/dpt/resolve.rs` claimed the publish hole above was
   still open. Rewritten to say it is closed and to rescope the surviving
   warning to the fingerprint's own limited inputs (see above).
2. `command.rs`'s round-trip test gained
   `assert!(stack.can_undo()); assert!(!stack.can_redo());` — but placed
   exactly where the draft/findings text suggested, on a stack whose redo
   list starts empty, they pass whether or not `do_command`'s
   `self.redo.clear()` runs at all. Falsified by commenting out that line
   and rerunning the test: it still passed. Fixed by having the test push
   and undo a scratch command first, so the redo stack is provably
   non-empty before the real `do_command` under test clears it — confirmed
   this version fails without `redo.clear()` and passes with it restored.
3. `ParameterPanel`'s `onValueApplied` — optional at the time the paragraphs
   above were written — is now a required prop (see above); its two mount
   sites (`Inspector.tsx`, `ParameterPanel.test.tsx`) were updated to match.
4. The `T26, first slice, fix round 1` entry earlier in this file gained a
   "(closed by T3, below)" cross-reference to this section.
5. Three occurrences of the false claim
   "`set_parameter_value_impl` ends in `apply(state, cmd)`" — in
   `App.tsx`, `ParameterPanel.tsx`, and this file's own paragraphs above —
   corrected to say it *runs* `apply(state, cmd)` and then returns a
   `ParameterPanelDto`, not that `apply` is the last thing it does.
6. The overlay described above is gone. `ParameterPanelDto`
   (`apps/knx-server/src/routes.rs`) gained a `tree: Option<ProjectTree>`
   field, `None` on every `GET` and on `assemble_parameter_panel`'s other
   construction site, `Some(apply(state, cmd)?)` on a successful
   `setParameterValue` — the same genuine, freshly rebuilt `ProjectTree`
   `apply` already produces for every other command, not a second build.
   `ParameterPanel.tsx`'s `onValueApplied` signature grew a `tree:
   ProjectTree` parameter carrying that value straight through; a missing
   `tree` on an otherwise-successful write throws rather than silently
   falling back to nothing, since a DTO contract violation is a bug, not a
   degraded case to paper over. `Inspector.tsx`'s hand-built
   `{...tree, can_undo: true, can_redo: false}` overlay is deleted; the
   mount site now hands the server's tree straight to `onApplied`.
   `ParameterPanelDto` dropped its `PartialEq` derive (nothing in the
   crate compared whole DTOs for equality; `knx_projection::ProjectTree`
   does not implement it) — the only change outside the DTO/bindings/
   `ParameterPanel`/`Inspector` boundary the findings doc allowed, and it
   stayed inside `knx-server`, never touching `knx-projection` itself.
   `DeviceWorkspace.test.tsx`'s test above now mocks a `setParameterValue`
   response carrying a `tree` that visibly differs from the local `tree`
   prop (`errors`/`warnings` counts) and asserts `onApplied` receives that
   exact server tree, not `{...tree, can_undo: true, can_redo: false}` —
   proving the publish is server-sourced, not caller-reconstructed.

Gates for fix round 1, all judged by exit status and all 0: `cargo fmt --all
--check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test
--workspace --no-fail-fast` (1272 passed, 0 failed, 3 ignored; 1275 declared,
identical to `main`), `xtask check-layering`, `xtask check-headers`, `cargo deny
check`, `npx tsc --noEmit`, and `npx vitest run` (465 tests in 42 files).

## 2026-09-13 — Spatial coordinates decided, not built (ADR-0019, branch `t21-coordinates-adr`)

T21's remaining half was never a UI task. The workbench already renders
areas, lines, devices and nested building parts; what the spatial
canvas/floor-plan editor lacked was a coordinate model underneath, and the
decision is recorded rather than the canvas built:
[ADR-0019](adr/0019-building-model-stays-topological.md), **the building
model stays topological — no spatial coordinates in v1.0.0**. No code
changed, `CURRENT_SCHEMA_VERSION` stays 6, and no table was added.

**The evidence, since a negative is only worth what its search is.** Three
things were checked, in this order:

- *The published schema.* *Project Schema23 v01.00.00* §1.2.6.4
  `complexType Space_t` carries `Id`, `Name`, `Type`, `Usage`, `Number`,
  `Comment`, `CompletionStatus`, `DefaultLine`, `Description`, `Puid` — and
  nothing spatial. §1.2.5.1 `complexType DeviceInstance_t` is the same
  across its ~30 attributes. `coordinate|geometr|floor.?plan` over the
  whole 64-page document: zero hits. The only length quantity in the
  format's vicinity is `Product/@WidthInMillimeter` in `Hardware.xml`, a
  DIN-rail width belonging to the catalogue, not a placement.
- *The three reference projects.* A full element/attribute inventory of
  each project part: schema 11 (`ETS 4.1.8`, `<Buildings><BuildingPart>`),
  schema 21 (KV demo, `6.0.5030.0`, `<Locations><Space>`) and schema 23
  (`6.3.7959.0`, `<Locations><Space>`). No `X`/`Y`/`Z`/position/angle
  attribute on any space, device, area or line in any of them. The only
  non-schema carriers — device `BinaryData` (all three entries named
  `244_Info`) and an `ExtraData/` directory the schema document never
  mentions — hold vendor plugin state, and schema 23 §4.2.1 puts that data
  outside the interoperable content anyway.
- *The Standard itself.* 3/10/3 *KNX IoT Information Model* v2.0.0 is the
  one place that formally models "the actual spatial building structure of
  an Installation", and its location classes carry only relational
  properties plus a postal `vcard:Address`; clause 1.2.2 delegates geometry
  to IFC by reference (`IfcBuilding`, `IfcBuildingStorey`, `IfcSite`,
  `IfcSpace`). Across the 179-document extraction, "floor plan", "site
  plan", "DXF" and "gbXML" appear in zero documents, and the only
  "coordinate" in the Standard is `DPT_Colour_xyY`'s colour coordinate.

**What the ADR pre-commits without building.** A later spatial layer arrives
as separate `FloorPlan` and `Placement` entities in their own tables, integer
millimetres (the unit the format itself uses) and millidegrees, origin at the
imported plan's own top-left rather than a site datum nothing supplies, no
`z` because floors are already a hierarchy level, plans imported rather than
drawn, and an explicit `.knxproj` export loss warning since no schema can
carry any of it. That migration would be store schema 7, additive only, and
every existing project migrates with zero rows — a complete project, not a
deficient one.

**One side finding, recorded rather than swept up.** `BuildingPartType` has
the six variants the reference projects exhibit; schema 23 documents eleven
values, so `Stairway`, `RoomPart`, `Area`, `Ground` and `Segment` are
coarsened to `BuildingPart` on import (with a reported `MapProblem`, not
silently) and re-exported as `BuildingPart`. Now
[KNOWN_LIMITATIONS.md §89](KNOWN_LIMITATIONS.md#89-five-documented-spacetype-values-are-coarsened-to-buildingpart-on-import);
deliberately not fixed inside a coordinate ADR.

Docs updated to match: `DATA_MODEL.md` §5 and §11, `ROADMAP.md` (T21's
motion-constraint and help-gating mentions, plus a new answered row in "Open
questions"), `GAP_ANALYSIS_ETS.md` (D1, D2, the T21 backlog bullet and the
two lists that gated on it), `goal.md` §3, `KNOWN_LIMITATIONS.md` §89.

What this entry does not claim: nothing here rules out a floor-plan feature
inside ETS's own database or a paid ETS App, and schemas 12-14, 20 and 22 were
never sampled — see ADR-0019's "What this evidence does not say".

**T5, DPT main types 20-30 and `6.020` (2026-09-14), branch
`dpt-main-types-20-46`.** The second E4 round on the codec, and the one
that closes the numbering gap: `crates/knx-core/src/dpt/codec.rs` now
decodes and encodes main types **1 through 30 inclusive, with no gaps**,
`6.020` among them. Every layout below was read out of
`03_07_02 Datapoint Types v02.02.01 AS.pdf` with
`pdftotext -layout` before it was implemented, and every clause named here
was re-checked against that output rather than carried over from a draft:
three citations in the draft this branch inherited named clauses that do
not say what the code claimed, and all three are now corrected in
[KNOWN_LIMITATIONS.md §61](KNOWN_LIMITATIONS.md#61-the-dpt-codec-covers-thirty-main-types-infers-rather-than-reads-its-input-and-leaves-several-encoding-questions-to-a-stated-ruling-rather-than-the-standard).

- **`6.020` DPT_Status_Mode3** — §3.7, "8 bit: B5N3", encoding row
  `B B B B B NNN`. New `DptValue::StatusMode3 { status: [bool; 5],
  mode_code: u8 }`. The mode field is one-hot: §3.7's Range row assigns
  only `{001b, 010b, 100b}`, and the other five codes are `InvalidData`,
  not a guessed mode. This was the last subtype-level exclusion inside an
  implemented main type.
- **20** — `N8` enumeration, §3.21. New `DptValue::Enum { code: u8 }`.
- **21** — `Z8`/`B8` bit set, §3.22.1 (General Status) and §3.22.2 (Device
  Control). New `DptValue::BitSet { bits: u32, width: u8 }`, shared by 21,
  22, 27 and 30.
- **22** — `B16`, §4.5.1 (22.100) and §4.5.2 (22.101), plus §8.3 for the
  system subtypes 22.1000 and 22.1010.
- **23** — `N2`, §3.23, with §4.6 for 23.102.
- **24** — `A[n]`, ISO 8859-1, NULL-terminated, §3.24. Extends `Text`.
- **25** — `U4U4`, §8.4. New `DptValue::DoubleNibble { busy, nak }`.
- **26** — `r1B1U6`, §3.25. New `DptValue::SceneInfo { inactive, number }`;
  the field is named `inactive` because §3.25 encodes `1` as "scene is
  inactive", the opposite polarity to main type 18's.
- **27** — `B32`, §3.26/§3.26.1.
- **28** — `A[n]`, UTF-8, NULL-terminated, §3.27/§3.27.1.
- **29** — `V64`, §3.28/§3.28.1. New `DptValue::Signed64(i64)`.
- **30** — `B24`, §8.5.

**Deliberately not implemented, and why.** Main type **31** stays
`UnsupportedDpt`: its single subtype 31.101 `DPT_PB_Action_HVAC_Extended`
(§4.7.1) carries the sentence "This DPT shall not be used for runtime
communication. This DPT shall only be used for encoding Parameter values in
CH_PB_HVAC_Mode_1", so a group-value codec has nothing legitimate to do
with it. It is what
`unimplemented_main_type_is_unsupported_not_a_panic` now names, and what
`apps/knx-cli/tests/cli_bus_dpt.rs`'s unsupported-DPT test now sends —
that test named DPST-20-102, which stopped being unimplemented here. The
eighteen 200-series LTE/system main types stay unsupported too. There is no
**main type 46** to implement at all: 46 is the *count* of main types in one
ETS master-data file, which is
[KNOWN_LIMITATIONS.md §90](KNOWN_LIMITATIONS.md#90-there-is-no-dpt-main-type-46-46-is-a-count-of-main-types-in-one-ets-master-data-file)
in full, including the `unzip | grep` command that re-measures it.

**No `knx_master.xml` catalogue consultation, by ruling.** The codec hands
back a raw enumeration code or raw bits, never a name. The brief did not
settle whether to read enumeration names out of the ETS master data, so
this task ruled it out: an enumeration whose names are invented is not a
feature, the master file's catalogue is a property of that file rather than
of the Standard (`docs/RESEARCH.md` §5), and
[GAP_ANALYSIS_ETS.md](GAP_ANALYSIS_ETS.md) row **E4** already carries
catalogue consultation as its own open item.

**Re-measured, not remembered.**
`cargo test -p knx-core --lib dpt::codec` → **164 passed, 0 failed**
(main's copy of the file holds 123 `#[test]` functions, so 41 are new);
`cargo test -p knx-core --lib dpt::` → **182 passed, 0 failed**;
`cargo test --workspace --no-fail-fast` → **1313 passed, 0 failed, 3
ignored** across 81 test targets. The full Rust gate set — `cargo fmt
--all --check`, `cargo clippy --workspace --all-targets -- -D warnings`,
that workspace test run, `cargo run -p xtask -- check-layering`,
`cargo run -p xtask -- check-headers`, `cargo deny check` — exits `0` six
times.
#### T4: group address style is visible and, cautiously, changeable (2026-09-14)

Closes [KNOWN_LIMITATIONS.md §84](KNOWN_LIMITATIONS.md#84-a-projects-group-address-style-can-be-chosen-and-afterwards-never-seen--resolved-2026-09-14-t4).
Five pieces, in the layer order the brief asked for:

1. `knx-store::project::style_from_str` (`load_project`'s deserializer for
   the `project_info.group_address_style` column) no longer falls back to
   `ThreeLevel` on an unrecognised string. It returns
   `StoreError::UnknownGroupAddressStyle(String)`, matching
   `POST /api/project/new`'s existing `400` for the same input rather than
   contradicting it. `Project::new`'s own in-memory default is untouched —
   only a *persisted, unreadable* value now errors instead of lying.
2. `knx_core::Command::SetGroupAddressStyle { style }` restyles the whole
   project. `apply` walks every installation's every group address first
   and refuses the entire change — no partial mutation, no rollback
   needed — naming the offending address's id and raw value
   (`CommandError::GroupAddressDoesNotFitStyle`) if even one does not fit.
   Self-inverting like every other command, so undo/redo need no special
   case. `knx-store::command_sync` gained a
   `SetGroupAddressStyle` arm calling the existing
   `set_group_address_style` column write directly (it has no device or
   group-address row to key off, unlike every other arm).
   `knx-server::domain::set_group_address_style_impl` and
   `POST /api/project/group-address-style` wire it through, `400` on
   refusal (there is no conflict state to resolve by saving first, unlike
   `POST /api/project/new`'s `409`).
3. `knx_projection::ProjectTree` gained `group_address_style: String`
   (`"Free"` / `"TwoLevel"` / `"ThreeLevel"` — the enum stays in
   `knx-core`, which deliberately has no `serde`/`ts-rs` dependency; same
   pattern as `BuildingPartType` → `building_kind_str`). Regenerated
   `apps/knx-web/src/bindings/ProjectTree.ts` via
   `TS_RS_EXPORT_DIR=../../apps/knx-web/src/bindings cargo test -p
   knx-projection`, mirroring CI's own binding-sync step.
4. `apps/knx-web`: a new `"project"` `Selection` kind, a selectable
   "Project" root node in `ProjectExplorer.tsx`, and a read-only
   `ProjectInspector` panel in `Inspector.tsx` showing
   `tree.group_address_style`. No restyle control anywhere in the UI —
   display only, per the dispatcher's ruling that a change this
   consequential does not qualify as "trivially additive". The `"project"`
   variant carries a structural `id: number` (always `0`, meaningless)
   purely so every existing `Selection`-generic call site
   (`StructureWorkspace.tsx`'s `selected(kind, id)`, `App.tsx`'s React
   `key`) keeps type-checking without being touched.
5. Proved, not asserted: `TwoLevel` (5+11 bits) and `ThreeLevel` (5+3+8
   bits) both partition the full 16 bits of a `u16` with no remainder, so
   `GroupAddress::fits_style` — which renders an address in the target
   style and parses the rendering back, answering `true` only if that
   round trip returns the original address — is `true` for all 65536
   possible raw values under every style, confirmed by exhaustive test,
   not a sample. (Fix round 1, below, replaced an earlier version of
   `fits_style` that compared bounds copied from `parse`/`format` rather
   than calling them, which could drift from the real codec unnoticed;
   the round-trip version cannot.) The "does not fit" branch in
   `Command::SetGroupAddressStyle` and the whole
   `CommandError::GroupAddressDoesNotFitStyle` variant are therefore
   currently unreachable from any real address. Built anyway and
   documented as such: the check is what stops a future change to the bit
   layout from silently making one style narrower than another, and a
   restyle command without a fits-check would be a data-integrity hole
   waiting for that future change to open it.

`CURRENT_SCHEMA_VERSION` stays at 6. No column changed shape or was added;
only `load_project`'s handling of an already-invalid value in an
already-existing column changed, from silent substitution to a typed
error. Nothing that round-tripped correctly before behaves differently now.

New tests: `crates/knx-core/src/address.rs` —
`group_address_largest_possible_value_fits_every_style`,
`group_address_smallest_possible_value_fits_every_style`,
`group_address_fits_style_holds_for_every_possible_raw_value` (all three
styles, boundary and exhaustive; renamed to
`group_address_format_parse_round_trips_for_every_possible_raw_value` by
fix round 1 below, which is the name in the tree today).
`crates/knx-core/src/command.rs` —
`set_group_address_style_do_undo_redo_round_trips_through_the_command_stack`,
`set_group_address_style_checks_every_installation_not_just_the_first`
(likewise renamed below, to
`set_group_address_style_accepts_every_installations_addresses`),
`group_address_does_not_fit_style_error_names_the_offender`.
`crates/knx-store/src/project.rs` —
`a_non_default_group_address_style_round_trips`,
`an_unrecognized_persisted_style_is_refused_not_defaulted`,
`set_group_address_style_overwrites_the_one_column`.
`crates/knx-store/src/command_sync.rs` —
`set_group_address_style_syncs_the_one_column`.
`crates/knx-projection/src/lib.rs` —
`project_tree_carries_the_projects_group_address_style`.
`apps/knx-server/tests/http_edit_routes.rs` —
`restyling_a_project_with_a_group_address_round_trips_and_undoes`,
`restyling_to_an_unknown_style_is_a_400`.
`apps/knx-web/src/Inspector.test.tsx` — `Inspector — project node` (two
cases: default `ThreeLevel`, and a non-default `Free`).

Thirteen pre-existing `apps/knx-web` fixture literals typed as
`ProjectTree`, one each in thirteen `*.test.tsx`/`*.test.ts` files, needed
a `group_address_style: "ThreeLevel"` field added once the type gained
the field — mechanical, no behavioural change to what any of those tests
covered (excluded: `CommandPalette.test.tsx`'s `tree`, cast `as unknown as
ProjectTree` and so exempt from the structural check).

`apps/knx-desktop` needed no changes: it depends on `knx-server` directly
and reuses its HTTP routes rather than duplicating command wiring.
`knx-cli`'s own pre-existing `ThreeLevel`-hardcoding (a different, already
documented limitation) is untouched — out of scope for this task.

Gates, all judged by exit status and all 0: `cargo fmt --all --check`,
`cargo clippy --workspace --all-targets -- -D warnings`, `cargo test
--workspace --no-fail-fast` (1298 passed, 0 failed, 3 ignored — up from
`main`'s 1285/0/3 by exactly the 13 tests listed above, confirmed by diff,
nothing else moved), `xtask check-layering`, `xtask check-headers`, `cargo
deny check`, `npx tsc --noEmit`, and `npx vitest run` (467 tests in 42
files, up from 465 by exactly the 2 new project-node cases). The
`golden_reference_products.rs` corpus tests found their local, gitignored
`OriginalData/` corpus present and ran full assertions (`grep -c 'skip:
OriginalData/ corpus not present' <log>` on the run's own log is 0).

#### T4 fix round 1 (2026-09-14)

Both review verdicts on T4 came back PASS and PASS WITH RESERVATIONS —
nothing here reverts shipped behaviour. The review's own mutation testing
found the ten-line validation loop in `Command::apply`'s
`SetGroupAddressStyle` arm was deletable without failing a single one of
1298 tests, because `fits_style` carried a private copy of `parse`/
`format`'s shifts and maxima rather than calling them, so it could never
observe a disagreement between the two. Eight items:

1. `GroupAddress::fits_style` (`crates/knx-core/src/address.rs`) now
   renders the address in the target style and parses the rendering back,
   answering `true` only if that round trip returns the original address
   — a genuine call through `format`/`parse` rather than a restatement of
   their bounds. Still `true` for every `u16` today (same 5+11/5+3+8
   partition argument as before), but it now fails the moment `format`
   and `parse` disagree about the bit split, which the old version could
   not detect regardless of how badly they disagreed.
2. The exhaustive test moved with it:
   `group_address_fits_style_holds_for_every_possible_raw_value` became
   `group_address_format_parse_round_trips_for_every_possible_raw_value`,
   looping `format`/`parse` directly over all three styles and all 65536
   raw values — the first exhaustive round trip anywhere in `address.rs`
   (the two hand-picked-value tests at the boundaries stay, redundant but
   harmless).
3. `set_group_address_style_checks_every_installation_not_just_the_first`
   — whose body put `u16::MAX` in installation two and asserted `Ok`,
   which passes whether the guard reads every installation, only the
   first, or does not exist — renamed to
   `set_group_address_style_accepts_every_installations_addresses` with a
   doc comment stating plainly what it can and cannot show: it fires only
   if the check wrongly *rejects* a representable address, the opposite
   direction from what the old name claimed; the bit-layout regression it
   cannot observe is item 2's job.
4. The check now also walks `installation.group_ranges`, checking each
   range's `start` and `end` — both `GroupAddress` values, per
   `GroupRange`'s own definition — with a new sibling error variant,
   `CommandError::GroupRangeDoesNotFitStyle { id: GroupRangeId, raw: u16,
   style: GroupAddressStyle }`, rather than stretching the existing
   variant over a different id type. Moot while `fits_style` cannot
   refuse anything, not moot after item 1: a future codec disagreement
   would otherwise catch every group address while letting a range
   boundary through unchecked. Both loops, for every installation, still
   finish before `project.info.group_address_style` is written.
5. `restyling_a_project_with_a_group_address_round_trips_and_undoes`
   (`apps/knx-server/tests/http_edit_routes.rs`) checked status codes and
   the post-undo rendering, never the restyled tree itself — a handler
   ignoring `groupAddressStyle` and hard-coding `ThreeLevel` would have
   passed. It now asserts on the response body before undoing:
   `group_address_style == "Free"` and the raw address `4242` renders as
   plain decimal `"4242"`.
6. `apps/knx-server/tests/save_load_roundtrip.rs` gained
   `restyling_over_http_then_saving_and_reloading_keeps_the_new_style`:
   restyle over HTTP, save-as, reopen, assert the loaded project reports
   the non-default style. The end-to-end claim in KNOWN_LIMITATIONS.md
   §84 was inferred from store-level and route-level coverage, never
   demonstrated directly, until now. No corpus needed — an empty project
   through `POST /api/project/new` is enough.
7. `apps/knx-web/src/ProjectExplorer.tsx`'s Project tree node — the only
   way a user reaches `ProjectInspector` — had no test of its own;
   `Inspector.test.tsx` builds the `{kind: "project"}` selection directly
   and never touches the tree, so deleting the node left 467/467 green
   (the review's own mutation). `ProjectExplorer.test.tsx` gained one
   test: the "Project" label renders and a click on it calls `onSelect`
   with `{kind: "project", id: 0}`.
8. Two loose substring assertions
   (`message.contains("7")`/`message.contains("42")` in
   `group_address_does_not_fit_style_error_names_the_offender`) replaced
   with an exact `assert_eq!` on the full message — `"7"` alone can match
   inside all sorts of unrelated text by accident. The new
   `GroupRangeDoesNotFitStyle` variant's test from item 4 uses the same
   exact-match style from the start.
   `apps/knx-web/src/messages/en.ts`'s comment above the Project node's keys, pointing
   at `ProjectDiffPanel` — a component with nothing to do with this panel
   — was replaced with an accurate description (display only, no restyle
   control here). `docs/IMPLEMENTATION_STATUS.md`'s own **dated** entries were
   normalised from a mix of `##`/`###` to all `##` per the standing ruling
   that the next toucher does it; this round was the next toucher. Per-task
   headings nested under a dated entry, this one included, stay `####` —
   the ruling was about the dated entries, not about every heading in the
   file.

#### T4 pre-merge review follow-ups (coordinator, 2026-09-14)

The mandatory `goal.md` §10 whole-branch review returned **MERGE** with two
Important findings, both about *permanence* rather than about the guard, and
both fixed here by the coordinator rather than in a second fix round — five
one-line edits and one new limitations section.

1. Four places still told the reader a style can never change, which stopped
   being true on this branch: `apps/knx-web/src/messages/en.ts` and `de.ts`'s
   `newProject.styleHint` (**user-facing**, the worst of the four),
   `apps/knx-web/src/NewProjectDialog.tsx`'s header comment, and
   `apps/knx-server/src/routes.rs`'s doc comment on
   `parse_group_address_style` — which now sits directly above the code that
   refutes it, since the restyle route shares that helper. All four rewritten
   to say the style is a rendering choice that can be changed later, while
   keeping the reason it is still asked at creation rather than defaulted.
2. `docs/KNOWN_LIMITATIONS.md` §91, new: a running bus session keeps rendering
   and parsing group addresses in the style its project had at
   `POST /api/bus/start` time, because `GroupAddressContext` is a snapshot and
   nothing refreshes it. Verified by reading both paths that no address is ever
   mis-parsed — the three styles have different field counts, so a cross-style
   string is refused rather than reinterpreted. Left as a documented limitation
   rather than fixed, because refreshing a live session from the mutation path
   inverts the lock order the snapshot exists to avoid.
   `apps/knx-server/src/bus.rs` points at the section from the accessor.
3. `crates/knx-projection/src/lib.rs` said "three crates, one string table" for
   the style's wire spelling; there is a fourth copy at
   `crates/knx-etsproj/src/export/schema11.rs`'s `group_address_style_str`.
   Now says four and names it.
4. This file's own T4 entry listed two test names that fix round 1 renamed
   away, so a reader grepping for them found nothing. Both now carry their
   current names. The heading-normalisation claim was also narrowed: the
   standing ruling covers *dated* entries, which are `##`; per-task headings
   nested under them stay `####`.

Gates for fix round 1, all judged by exit status and all 0: `cargo fmt
--all --check`, `cargo clippy --workspace --all-targets -- -D warnings`,
`cargo test --workspace --no-fail-fast`, `xtask check-layering`, `xtask
check-headers`, `cargo deny check`, `npx tsc --noEmit`, and `npx vitest
run`. Exact totals recorded in
`.superpowers/sdd/2026-09-13-goal-completion/task-4-fixround-1-report.md`,
not reproduced here since they belong to a single point in time on a
branch, not a durable project fact.

## 2026-09-14: Task 6 — a performance baseline, so future slowness has a witness

`docs/PERFORMANCE.md` is new: a reproducible timing of import, open,
projection, search and export over a synthetic, fixed-seed, 5,000-device /
20,000-group-address project (`crates/knx-app/tests/perf_baseline.rs`, an
`#[ignore]`d integration test — `cargo test --workspace` never runs it,
`cargo test -p knx-app --release --test perf_baseline -- --ignored
--nocapture` does). No `OriginalData/` corpus file is touched; the project
is built by pure index arithmetic and pushed through the real
`export_knxproj` / `import_ets_project` / `knx_store::load_project` /
`knx_projection::build_project_tree` functions, so the numbers describe
production code, not a benchmark-only shortcut. The "search" stage has no
backend equivalent to measure — the real search is client-side
(`apps/knx-web/src/searchMatch.ts`) — so it is a self-contained,
documented-as-such substring scan, not a stand-in for the frontend's
algorithm.

Measured once, uncontended, on `Linux 7.2.3-arch1-3 x86_64` /
`cargo 1.98.0 (797e8a9bc 2026-08-05)` / 16 cores: export 78 ms, import
179 ms, open (`load_project`) 945 ms, projection 18 ms, search (41
queries) 19 ms total. `open` is the slowest stage by a wide margin; nothing
under `load_project` has been profiled further, and nothing has been
"optimized" on the strength of one number — see `docs/PERFORMANCE.md` for
the full numbers, the machine/toolchain they came from, and the exact
reproduction command.


## 2026-09-14 — Commissioning phase 2: the download protocol, offline (T21, branch `t30-commissioning-protocol`)

Phase 1 was a design document; this is the code it specified. The
commissioning download protocol of
[docs/superpowers/specs/2026-09-13-commissioning-download-design.md](superpowers/specs/2026-09-13-commissioning-download-design.md)
(§5.4, §5.5, §6.2–§6.5, §7.2, §7.3, §9.1, §10.9, §11.1–§11.3) now exists,
entirely against a simulator written alongside it. **No socket was opened to
any gateway, no device was addressed, and no frame left the machine** —
`KNOWN_LIMITATIONS.md` §92 states what that costs.

**`knx-core`, the domain half (no I/O, no async, ~4 400 lines, 108 tests).**

- `commissioning/load_state.rs` — `LoadState` and `LoadEvent` as separate
  enums because §5.1's read and write encodings differ, plus RES Table 94
  transcribed as `permitted_outcomes(from, stimulus, mask)` with the
  `R:`/`O:` alternatives both admitted. `accepts()` includes the
  intermediate states, `is_settled()` does not, and `narrow_for_mask`
  applies the one narrowing PROF documents (mask `0912h`) and no other.
  `Error` is a trap: only `Unload` leaves it.
- `commissioning/load_control.rs` — the ten-octet event payloads,
  `nr_of_elem = 01h` / `start_index = 01h`, and `allocation_subtype_for`
  mapping mask to subtype (`07B0h`/`17B0h`/`57B0h` ⇒ `0Bh` Data Relative
  Allocation, `0300h` ⇒ `0Ah` Relative Allocation, anything else ⇒
  `MaskNotProfiled`). No fallback between allocation styles: a mask whose
  Table 7 row was not transcribed is a refusal.
- `commissioning/memory.rs` — §6.4's chunk size
  (`PID_MAX_APDU_LENGTH` absent, 255 or router-only ⇒ 12; a value *v* ⇒
  `min(v, 254) − 3` capped at 63) and §6.5's service selection on
  `base + length` rather than on `base`.
- `commissioning/properties.rs`, `error_code.rs`, `authorisation.rs`,
  `procedure.rs`, `programming_mode.rs`, `mutation.rs` — eleven cited PIDs
  and the `PID_DEVICE_CONTROL` bit arithmetic; `DPT_ErrorClass_System`
  decoding shared with the DPT 20.011 codec rather than duplicated;
  `AccessLevel` where lower is more powerful and the ordering says so; the
  cited procedures as declarative step lists that can be rendered and
  dry-run; §4.4's `curr_prog_mode` toggle, which inverts bit 0 and bit 7
  together, never computes a parity, and issues **no write at all** when the
  mode already matches; and §2.3's `WriteAuthorisation`.

**`knx-net`, the session and the sequencer (~5 000 lines, 40 tests).**

- `commissioning.rs` — `ManagementSession`: one connection-oriented door to
  one device. Sequence numbering, the TL clause 4 repeat rules, MP §3.5.1
  authorisation on every connect (and on every reconnect), §6.3's Verify
  Mode as a read-modify-write of `PID_DEVICE_CONTROL` bit 2 that tolerates a
  device leaving it off, and the project's own `[A]` rule that **no write
  path exists without a client-side verification read** — compared against
  `A_Memory_Write.res` when Verify Mode is active, and against an explicit
  `A_Memory_Read` after a programming delay when it is not.
- `commissioning/download.rs` — CP §3.5.2 (complete download), §3.5.3
  (partial download and its escalation), §3.5.4 (unload) and design §9.1
  (recovery) as a sequencer that records every step it took. Nothing is
  retried in a loop.
- `commissioning/simulator.rs` — a device that implements RES Table 94,
  §6's length limits, §7.6's "allocation is ignored outside `Loading`", and
  the §9.2 failure modes including the ones that present as silence. It can
  be told to drop the connection at a chosen step, to fail one allocation
  once, to answer any mask version, and to permit reads while silently
  dropping `PID_LOAD_STATE_CONTROL` writes.

**Five things that are deviations, and are marked as such rather than
quietly chosen.**

1. *A matching CRC does not skip the data write.* CP §3.5.3 offers the
   skip, but step 05 has already unloaded the part and **[D]** RES Table 93
   declares the data undefined, and the "differential download algorithm"
   the clause names for the case is specified nowhere. The comparison is
   performed and reported (`CrcComparison::Matched`/`Differed`/`NoStoredCrc`);
   the payload goes out regardless.
2. *The recorded step order is 01, 03, 02.* `connect()` authorises as it
   connects (§10.3), so the Authorize of CP §3.5.2 step 03 happens before
   the Device Descriptor read of step 02. The trace says so instead of
   pretending otherwise.
3. *CP §3.5.3's Nr. 06 and Nr. 08 are recorded once, under Nr. 06.* Nr. 08
   never appears in a trace.
4. *§9.1 step 5 is read literally:* every part is reloaded during a
   recovery, including parts that read `Loaded`, because an interrupted
   download can leave old and new parts mixed. The cost is §9.3's: a
   `Loaded` part is invalidated on the way through.
5. *The 500 ms programming delay is this project's number.* MP §3.16 says a
   delay is needed and never quantifies it.

**§14's test list, item by item.** Items 1, 2, 3, 4, 5, 8, 9, 10, 11, 12,
13, 14, 15, 16, 17 and 18 are implemented. Item 6's counting rules and item
7's occupied-address refusal belong to §4.2 individual-address programming,
which this task's scope list does not contain; the responder-counting half
of item 6 exists anyway (`programming_mode.rs`), because the toggle needed
it. Item 11 — the exclusion guard — is structural: `1.1.220` cannot be
authorised, cannot be read, cannot be written, and cannot appear in a plan
or in a range that spans it, and each refusal is asserted to be *reported*.

**Fix round 1 (review on `opus`, 2 blocking findings and 13 others, all
addressed).** Two things in the list above were published wrong and are now
right:

- *The `0300h` allocation payload was mis-encoded.* MP §3.31.3.4's *Load Event
  Relative Allocation* table gives subtype `0Ah` a **two**-octet *"number of
  octets"* field followed by six fill octets; `relative_allocation` wrote four
  octets of `usize`. A request for 2048 octets went out as size `0000h` with
  `0800h` leaking into the first two fill octets, which by the clause's own
  next sentence sends the Load State Machine to `Error`. It now writes two
  big-endian octets and refuses anything above `FFFFh`
  (`AllocationSubtypeError::PartTooLarge`) instead of clamping; the `0Bh` path
  refuses above `FFFF FFFFh` for the same reason. Subtype `0Bh`'s **fill**
  octet, previously always `00h` and invisible in the API, is now
  `AllocationMode::Fill(u8)` and lands at its own octet, separate from Mode.
- *Four `[D]` markers cited this project's own prose and have been demoted.*
  `[D]` means the Standard states it and the text is quoted from its PDF. Two
  in `authorisation.rs` (the access-level inference, now `[D, corpus]` PROF
  §4.2; the no-key policy, now named as this project's ruling with only MP
  §3.5.1's `key != FFFFFFFFh` left under `[D]`), one in `load_control.rs` (the
  allocation layout, now MP §3.31.3.4 quoted properly, with Mode and fill as
  the two octets they are), and one paraphrased AL §3.5.3 sentence in
  `commissioning.rs` now quoted verbatim. No marker was promoted. The word
  "spec" no longer stands for both the Standard and the design document in
  `load_control.rs`.

Also in this round: the unverified-write path waits for the T_ACK through
`send_acknowledged()`, so TL's acknowledge time-out and `MAX_REP_COUNT = 3`
now apply to the normal download path instead of being skipped by a bare
`send()`; a chunk is read back through the same service that carried it
(`service_for(address, length)`), so a region straddling `FFFFh` is no longer
verified against the wrong address space; `write_property` takes the required
`WriteScope` as a parameter rather than reading it out of the authorisation it
checks against; `require_subtype` is called from `allocation_payload` instead
of only from its own tests; the partial-download trace uses CP §3.5.3's
*"Partial Download of the 'application program 2'"* numbering (01–14), records
the access-key step, and disconnects at 14 rather than at a number the clause
never uses; and the step counter no longer overflows at 250 loadable parts.

**Re-measured, not remembered.** `cargo test --workspace --no-fail-fast` →
**1508 passed, 0 failed, 3 ignored**; `grep -c 'skip: OriginalData'` over
that log → **0**. The six Rust gates — `cargo fmt --all --check`,
`cargo clippy --workspace --all-targets -- -D warnings`, that workspace test
run, `cargo run -p xtask -- check-layering`,
`cargo run -p xtask -- check-headers` (116 headers, 168 without one, ceiling
168 — unchanged), `cargo deny check` — exit `0` six times. No TypeScript was
touched.

## 2026-09-14: Task 8 — the same-file id collision blind spot, closed for `first_winner`'s real callers

`crates/knx-productdb/src/parse/mod.rs`'s `first_winner` (see the
2026-09-13 entry above, "Two parked findings closed") only detected an
`IdConflict` when the colliding id came from a *different* file, because
every element from one file was handed the same `source_sha256` and two
same-file duplicates always compared equal. `first_winner` now also takes
a `seen_this_call: &mut HashMap<(String, String), u32>`, created fresh
once per `ingest_hardware`/`ingest_catalog` call and threaded through
every call site in that one parse; a conflict is recorded when the old
cross-file check fires *or* an id's occurrence count for this call exceeds
1. `IdConflict` gained `pub occurrence: u32` (`crates/knx-productdb/src/
report.rs`), persisted via the existing `ingest_unknown.occurrences`
column and a new `package_conflict.occurrence` column (schema v6 → v7,
`migrate_v6_to_v7`, `DEFAULT 1` for old rows). `source_sha256`'s meaning as
file provenance is untouched everywhere else it is relied on (idempotent
re-parse, translation backfill). `application_program`'s separate
hand-rolled first-writer-wins copy in `parse/program.rs` was explicitly
left with the same blind spot — out of scope, touched only to keep
compiling against the new mandatory field. `datapoint_type`'s complete
absence of conflict tracking is also untouched. Both are documented
residue in KNOWN_LIMITATIONS.md §86, which is amended (not renumbered) to
record the fix. Pinning tests
`two_hardware_elements_sharing_an_id_in_one_file_conflict_silently` and
`two_catalog_items_sharing_an_id_in_one_file_conflict_silently` are now
`..._record_the_collision`, asserting one conflict with `occurrence == 2`
instead of an empty conflict list; both were confirmed, in their old form,
to fail against the new code before being rewritten.

Gates run from the worktree root
(`.worktrees/productdb-collision-record`, branch
`productdb-collision-record`): `cargo fmt --all -- --check`, `cargo
clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace
--no-fail-fast`, `cargo run -p xtask -- check-layering`, `cargo run -p
xtask -- check-headers`, `cargo deny check` — exit codes and test totals
recorded in this session's dispatch report, not reproduced here since they
belong to a single point in time on a branch, not a durable project fact.

Out of scope, left as documented residue: `parse/program.rs`'s
`application_program` first-writer-wins copy (still same-file blind);
`master.rs`'s `datapoint_type` ingestion (still no conflict tracking of
any kind, cross-file or same-file).

## 2026-09-14 — Migrations may re-derive from stored bytes (ADR-0020, product-database schema v8, branch `productdb-linkable-adr`)

The task was an ADR, and the ADR had to be written before the code so that
the code could be whatever the decision turned out to license. It licensed a
migration, so there is one:
[ADR-0020](adr/0020-migrations-may-rederive-from-stored-bytes.md), **a
product-database migration may re-derive what the stored bytes determine, and
must not invent what only the install knew.**

**The premise the task started from was false, and reading the chain was the
whole finding.** A re-parsing migration was expected to be the first
migration to call the parser. It is the third. `migrate_v2_to_v3` calls
`dynamic::parse::parse_dynamic_trees` over stored blobs and `migrate_v3_to_v4`
calls `classify` plus `parse::translation::ingest_translations`, both with a
per-blob `SAVEPOINT` and a `record_backfill_failure`, and `migrate_v2_to_v3`'s
doc comment already argues the case in prose. So the question was never
whether to take a new architectural risk; it was where the line runs, given
that the chain has been crossing it since schema v3 without writing the rule
down.

**The line, and why it explains the migrations that refused to backfill.**
A value that is a pure function of bytes the database already holds may be
re-derived by a migration. A value that was an artefact of the install *event*
may not be invented, and must keep an honest default. That is why
`migrate_v4_to_v5`'s four `package` counters and `migrate_v5_to_v6`'s
`dropped_datapoint_type_count` default to 0 rather than being reconstructed: what
an `INSERT OR IGNORE` actually changed on a particular afternoon is install
history, and no blob records it. `linkable` is on the other side of the line —
it is one attribute of one element of one file whose bytes are in
`source_file` — so it is re-derivable, and now is.

**Five obligations the ADR imposes on any future backfill.** Call the parse
layer, never a frozen private copy of its rules; write into absent slots only
(ADR-0012's rule, reused as `linkable IS NULL`); scope every write by
`source_sha256`, so a blob repairs only the rows its own bytes produced; wrap
each blob in its own `SAVEPOINT` and record a failure as an `ingest_unknown`
row rather than refusing to open the database; let `user_version` be the
record that the backfill ran. Rules 1, 4 and 5 are the existing backfills'
habits promoted to requirements. Rules 2 and 3 are new, because v2→v3 and
v3→v4 only insert rows, and v7→v8 is the first one that `UPDATE`s a column an
earlier build already wrote.

**What shipped.** `CURRENT_PRODUCTDB_VERSION` is 8. `migrate_v7_to_v8` adds no
DDL at all — it is a backfill, and `user_version` is the entire mechanism by
which it runs once. It selects only the blobs behind a `linkable IS NULL` row,
and `backfill_linkable` (`crates/knx-productdb/src/parse/program.rs`) streams
each one with quick-xml, skipping `Dynamic` subtrees, and on every
`ApplicationProgram` start or empty tag runs the attribute through the same
`bool_flag` an ingest would use. The `UPDATE` carries `AND source_sha256 = ?`
and `AND linkable IS NULL`, so neither another package's row nor a value a
real ingest already determined can be touched, and a successful fill deletes
the "attribute not understood" `ingest_unknown` row it has just made false —
the report had a genuine complaint and no longer does. A blob that fails to
parse rolls back to its own savepoint and records a `LinkableBackfillError`,
joining `DynamicBackfillError` and `TranslationBackfillError` as the third
`kind` of that shape.

**Eight tests, seven of them against a database the previous version built.**
The unit tests run `migrations()[0..6]` — literally the v6 chain, not a
hand-written schema — ingest a program, blank the column the way the old
`bool_flag` left it, plant the stale `ingest_unknown` row the old parser
emitted, stamp `user_version = 6`, and then reopen through `open_and_migrate`.
They cover a false value filled as 0, a true value filled as 1, a file that
never stated the attribute staying `NULL`, a value an ingest already
determined not being overwritten, a second row pointing at the same blob
proving the `source_sha256` guard is load-bearing (the first draft of that test
would have passed vacuously — repointing the row also removed the blob from
the migration's own `SELECT`), a malformed blob recording itself without
stopping the migration, and a database with nothing to fill reading no blob at
all. The eighth is a corpus test in `tests/standalone_packages.rs`: install the
four corpus packages with the current build, snapshot every
`(id, linkable, source_sha256)`, roll the file back to the pre-fix state, and
prove the migration returns every value identical, retires every stale report
row, and records no failure.

**Measured, not assumed.** 34 of 34 corpus programs refilled in 1.14 s, values
identical to a fresh ingest (27 false, 7 true); a v6 database with nothing to
fill opens in 0.038 s. The corpus holds 35 `Linkable` occurrences, all of them
on `<ApplicationProgram>`, with word-spelled values in `project/11`, `20` and
`21` archives and numeric ones in `project/11` and `23` — so the spelling
belongs to the tool that wrote the file, not to the schema version, and the
previous claim that this was a schema-20/21 problem understated it. Four
packages come out of five `.knxprod` files because two Weinzierl archives are
byte-identical, which is exactly the deduplication ADR-0011's content hash is
for. The rejected alternative — rebuild the database from the original files —
costs 17.0 s for 128 MB and needs files the blob store exists so that a user
need not keep.

**Rejected, and why.** Re-derive on read or lazily on first use: there is no
reader to hook, `linkable` has zero consumers today, so the lazy path would be
speculative machinery around a column nobody queries. Force a package
reinstall: it would have to defeat three independent short-circuits and could
still leave holes, because rows are first-writer-wins across packages. Freeze a
private copy of the boolean rules inside the migration: duplicated logic that
drifts silently, which is the failure this whole task is repairing. Deferred
rather than dismissed: a parse-generation marker per `source_file` row,
reported by `knx products verify`, whose by-construction false positive
(a build that changed nothing still bumps the generation) is named in the ADR.
Not built for one column; reach for it if the class turns up a third time.

**[KNOWN_LIMITATIONS.md §87](KNOWN_LIMITATIONS.md#87-a-parse-fix-does-not-reach-rows-that-were-already-ingested-and-only-a-migration-can-go-back-for-them)
keeps its number and changes its subject.** It was "existing databases keep
their `NULL` `linkable`"; it is now the class — a derived row cannot know that
its derivation should run again — with the `linkable` instance marked fixed
and the rebuild workaround, the detection that is missing, and ADR-0020's rule
for the next instance all recorded in it.

**Re-measured, not remembered.** `cargo test --workspace --no-fail-fast -j 2`
→ **1531 passed, 0 failed, 4 ignored** across 82 suites;
`grep -c 'skip: OriginalData'` over that log → **0**, so the corpus tests
actually ran. The six Rust gates — `cargo fmt --all -- --check`,
`cargo clippy --workspace --all-targets -j 2 -- -D warnings`, that test run,
`cargo run -p xtask -- check-layering`,
`cargo run -p xtask -- check-headers` (117 headers, 168 without one, ceiling
168 — unchanged, since no source file was added), `cargo deny check` — exit
`0` six times. No TypeScript was touched.

## 2026-09-14 — T18 slice 5's fix round: product-database schema v9, `parameter_type` bounds re-derived the same way `linkable` was (ADR-0020, branch `t18-format-validation`)

The whole-branch review of T18 slice 5's format-validation fix round found
the same defect ADR-0020 exists to name: `crates/knx-productdb/src/parse/
program.rs` started reading `TypeFloat/@minInclusive`/`@maxInclusive` and
`TypeText/@SizeInBit` with no version bump, so every pre-existing
`products.sqlite` kept `NULL` in `parameter_type.min_inclusive`/
`max_inclusive`/`size_in_bit` forever — the exact shape `linkable` was in
before schema v8. `CURRENT_PRODUCTDB_VERSION` is now **9**.

**What shipped.** `migrate_v8_to_v9` (`crates/knx-productdb/src/
migration.rs`) adds no DDL — same as v7→v8, it is a backfill and
`user_version` is the entire mechanism by which it runs once. It selects
distinct `source_file` blobs behind either a `Float` row with both
`min_inclusive` and `max_inclusive` `NULL`, or a `Text` row with
`size_in_bit` `NULL`, joined through `application_program` (`parameter_type`
carries no `source_sha256` of its own, unlike `application_program`, so the
join stands in for the direct column `backfill_linkable` reads). Each blob
is streamed by `crate::parse::program::backfill_parameter_type_bounds` with
quick-xml, and the `UPDATE` for each kind carries `... IS NULL` on both
target columns plus an `EXISTS` check against `application_program`'s own
`id`/`source_sha256`, so neither another package's row nor a value a real
v9 ingest already wrote can be touched. Each blob runs inside its own
`SAVEPOINT parameter_type_bounds_backfill_blob`; a parse failure rolls back
to it and records a `ParameterTypeBoundsBackfillError` via
`record_backfill_failure`, joining `LinkableBackfillError`,
`DynamicBackfillError`, and `TranslationBackfillError` as the fourth `kind`
of that shape.

**Where it differs from `linkable`, on purpose.** The old (pre-v9) parser
never asked `insert_parameter_type` about these two attributes at all, so
there is no stale `ingest_unknown` row for a successful backfill to retire —
unlike `linkable`, whose backfill turns a genuine old complaint false. A
backfilled row also does not gain the `Encoding`/`Increment`/`DisplayFormat`
`ingest_unknown` rows a fresh v9 ingest now records for `TypeFloat` (T18
slice 5 fix round's `report_unknown_attrs` call) — this migration re-derives
bounds only, matching what it exists to fix, and the gap is named rather
than silently left different (see `migrate_v8_to_v9`'s own doc comment and
`KNOWN_LIMITATIONS.md` §87).

**Six unit tests, one end-to-end test, no corpus roundtrip test of its
own.** The migration tests build a v8-shaped database by ingesting a
program with the current parser and then manually `NULL`-ing the bounds
columns and stamping `user_version = 8`, then reopen through
`open_and_migrate`: a `Float` row's bounds filled from its blob, a `Text`
row's `size_in_bit` filled from its blob, a program whose file never stated
either attribute staying `NULL`, a bound an ingest already determined not
being overwritten, a second blob's row not being touched by a different
file's backfill, a malformed blob recording itself without stopping the
migration, and a database with nothing to fill reading no blob at all.
Unlike `linkable`'s eighth test, there is no separate corpus-roundtrip test
in `tests/standalone_packages.rs` for this backfill — coverage instead comes
from `apps/knx-server/tests/http_parameter_panel.rs`'s
`parameter_type_bounds_flow_from_product_db_through_to_the_write_validator`,
an end-to-end test that ingests a product database, then exercises the HTTP
write endpoint through the product-database layer and `domain.rs`'s
`validate_kind_and_bounds` together for both an accepted and a rejected
value on each kind — it fails if the bounds columns are removed from either
layer.

**[KNOWN_LIMITATIONS.md
§87](KNOWN_LIMITATIONS.md#87-a-parse-fix-does-not-reach-rows-that-were-already-ingested-and-only-a-migration-can-go-back-for-them)
now names this as the class's second instance, not its third** — the
generic per-`source_file` marker mechanism ADR-0020 sketches and defers
stays unbuilt; two occurrences is not the threshold the ADR names for
reaching for it.

**Re-measured, not remembered.** `cargo test --workspace --no-fail-fast -j 2`
→ **1563 passed, 0 failed, 4 ignored** across 82 suites;
`grep -c 'skip: OriginalData'` over that log → **0**. The six gates —
`cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -j 2
-- -D warnings`, that test run, `cargo run -p xtask -- check-layering`,
`cargo run -p xtask -- check-headers` (117 headers, 168 without one, ceiling
168 — unchanged, no source file added or removed), `cargo deny check` —
exit `0` six times.

## 2026-09-14 — Commissioning phase 3: read-only verification against the real installation (T30, branch `t30-commissioning-readonly`)

The repository's second `#[ignore]`d live-hardware test file,
`crates/knx-net/tests/live_commissioning_readonly.rs`, joins
`live_gateway.rs`. Gated the same way: no default gateway address, `KNX_GATEWAY`
must be set as `host:port` by whoever runs it, and it never runs in CI or
unattended (`cargo test -p knx-net --test live_commissioning_readonly --
--ignored --nocapture`).

Its construction is read-only by type, not by discipline. Every session it
opens is a `ManagementSession::read_only` with `AuthorisationPlan::Skip`, so
neither a write path nor `A_Authorize_Request` exists to call — no write of
any kind reached the wire. Its nine targets, `1.1.24`–`1.1.32`, are the
individual addresses design spec §2.2 (R-SAFE-2) approves for active reads,
named one at a time as string literals and never iterated as a range, so
`1.1.220` — the alarm panel R-SAFE-1 excludes structurally — cannot be
constructed from this list by widening it; the test's own first assertion
checks `1.1.220` is still in `EXCLUDED_INDIVIDUAL_ADDRESSES` before a single
frame goes out, redundant with `ManagementSession::build`'s own refusal.

The run itself (RESEARCH §8.8) read Device Descriptor Type 0,
`PID_MANUFACTURER_ID`, `PID_HARDWARE_TYPE`, `PID_PROGRAM_VERSION` and
`PID_LOAD_STATE_CONTROL` on the three loadable Interface Objects against all
nine addresses, then cross-checked with the already-shipped `bus scan`
probe. The finding (§8.8.3) is against the verification method, not the
devices: seven of the nine real, present, answering devices timed out
through `ManagementSession`'s own retry budget and were indistinguishable
from the one genuinely vacant address, while the scan probe saw all seven as
occupied — on its **first** attempt, since it ran at `vacant_confirmations:
1` and therefore never retried at all. Recorded as spec §13 **R20**, a named risk for the write
path this project has not built yet: it must not treat its own read
time-out as proof a target is absent. What discriminated the one address
that did answer `ManagementSession` from the seven that did not is
unconfirmed by this pass and not fixed here.


## 2026-09-14 — Password-protected ETS4/ETS5 projects open now, with the cipher that protects them named for what it is (T15, branch `zipcrypto-projects`)

**What shipped.** `crates/knx-secure/src/zipcrypto.rs` (new, 436 lines, 7
tests) implements PKWARE Traditional Encryption — "ZipCrypto" — directly
against APPNOTE.TXT v6.3.3 §6.1.3–§6.1.7, whose pseudocode is quoted verbatim
in the module's own doc comment `[D]`. Public surface: `HEADER_LEN`,
`CheckBytes`, `ZipCryptoError::{TruncatedHeader, WrongPassword}` and
`decrypt(password, stream, check)`. There is no encryption function and there
will not be one: this repository reads a protected project the caller already
owns and never produces one.

`knx-etsproj`'s `Container` (`crates/knx-etsproj/src/container.rs`, 8 tests)
gains `open_with_password` beside the existing `open`, sharing all outer-archive
parsing through a new private `open_raw`. `Container::open`'s behaviour is
unchanged — it still refuses every protected project outright, now with the
message "no password was supplied" rather than "decryption is not implemented",
because the second sentence stopped being true. The nested `<project part>.zip`
payload's entries are decrypted once at open time, decompressed with `flate2`
(Stored and Deflated only; anything else is a typed error, not a guess), and
then behave exactly like an unprotected project's entries — the opaque nested
blob disappears from `entries()` and is replaced by what it contained.
`knx-etsproj` therefore depends on `knx-secure` now, an edge
[ARCHITECTURE.md](ARCHITECTURE.md)'s crate graph did not have before.

**AES (schema ≥ 21 / ETS6) stays refused**, by a named error rather than a
silent failure: `ContainerError::UnsupportedEncryption { nested_entry, scheme }`,
distinct from `PasswordProtected` so a caller that *did* supply a password can
tell "wrong scheme" from "no password given". The key derivation for it has
lived in `knx-secure` since A6; the container half is unverified against a real
protected export and is not shipped on a guess.

**What this is not.** Both fixtures are synthetic — `crates/knx-secure/
fixtures/zipcrypto-entry.bin` (66 bytes) and `crates/knx-etsproj/fixtures/
zipcrypto-protected.knxproj` (1,037 bytes), both generated with Info-ZIP `zip`,
never by any code path in this repository. **No real ETS4 or ETS5
password-protected export has been opened by this code.** The algorithm is
fully specified and does not vary by writer, so a fixture from an independent
ZIP tool tests the algorithm; it does not test that ETS writes what the
specification says. [COMPATIBILITY.md](COMPATIBILITY.md) §3 and
[KNOWN_LIMITATIONS.md](KNOWN_LIMITATIONS.md) §13 say so in those words.

**One thing the cipher's own specification understates.** The password check is
one byte, and there are two published conventions for what that byte contains
(PKZIP's CRC-32 high byte, Info-ZIP's DOS-time high byte). `decrypt` accepts
either, because nothing in the `zip` crate's public API says which one wrote a
given entry — which doubles the false-accept rate from 1 in 256 to roughly **1
in 128**. ZipCrypto's security is already nil (Biham & Kocher, 1994), so the
number changes no decision, but it is now stated in all three places that state
a number at all rather than only in the one place that is convenient.

**Gates (superseded by the fix round below — see there for the current
numbers):** `cargo fmt` `0`, `cargo clippy --workspace --all-targets -D warnings`
`0`, `cargo test --workspace` **1,542 passed / 0 failed**, `check-layering` `0`,
`check-headers` 118 well-formed / 168 without one (ceiling 168 — the new file
carries its header), `cargo deny check` `0`. No TypeScript was touched.

## 2026-09-14 — T15's fix round: the AES refusal never fired, and a decrypted entry was trusted more than an unencrypted one (branch `zipcrypto-projects`)

Last updated: 2026-09-14.

The whole-branch review reimplemented ZipCrypto independently in Python,
decrypted both fixtures byte for byte against APPNOTE §6.1.5–§6.1.7, and
confirmed the cipher — then returned **MERGE AFTER FIXES** with two blocking
findings and eleven smaller ones. Both blocking findings were about the
container, not the cipher.

**Finding 1: the AES refusal was dead code.** `Container::open_with_password`
tested `entry.compression() == CompressionMethod::AES`. The `zip` crate
overwrites that parsed field with the entry's *real* underlying compression
method the instant it parses a WinZip AES extra field (0x9901), exactly as
APPNOTE §4.5 intends it to — so the comparison could never be true, for any
input. An ETS6 project opened with its **correct** password would fall through
to the ZipCrypto path and be reported as `WrongPassword` in about 127 cases out
of 128, the remaining one being a check-byte false accept on the way to a
decompression error. The fix reads the raw on-disk compression-method field
straight out of the payload bytes (APPNOTE §4.4.5, local-file-header offset 8)
where it still says 99 either way. `scheme` also stopped being a `String` and
became an `EncryptionScheme` enum, so a caller can `match` on it instead of
comparing English prose.

*Ruling: the `zip` crate's `aes-crypto` feature stays off.* The review offered
enabling it as one of three options. Enabling a decryption feature to fix a
*refusal* buys a capability nobody asked for, pulls three crypto crates into
the tree, and would have to be justified to `cargo deny` — all to avoid reading
two bytes at a known offset. Cost if wrong: when AES decryption is eventually
implemented, that feature gets enabled then, and this check becomes redundant
rather than wrong.

**Finding 2: a decrypted entry got no CRC-32 check.** The unprotected path gets
one for free from `zip`'s `Crc32Reader`; the decrypted path, which bypasses
`zip`'s reader entirely, had none — so the *protected* path was less trustworthy
than the unprotected one, which is precisely backwards. ZipCrypto's check byte
(APPNOTE §6.1.6) rules out only 255 of 256 wrong passwords per convention, and
both conventions are tried, so roughly 1 wrong password in 128 walks past it.
Every decrypted entry's decompressed bytes are now checked against the entry's
declared size and its CRC-32 from the central directory, and a mismatch is
reported as `WrongPassword` — after a check byte has already passed, that is
what it almost certainly is.

**The nine non-blocking findings**, all taken: the size guard in the encrypted
branch moved to *before* the allocation it guards (it had sat after both the
allocation and the read, contradicting the module's own doc comment); `decompress`
now bounds inflation by the entry's declared size instead of running until memory
does; a nested entry whose path collides with one already in the inventory is
refused by name (`ContainerError::DuplicateEntry`) instead of being silently
shadowed by whichever copy the lookup happened to prefer; `Container::was_decrypted()`
exists so a later import stage can see the roundtrip gap coming; a test docstring
that described a re-encryption the test never performed lost its false paragraph;
the hard-coded `12-byte encryption header` became `zipcrypto::HEADER_LEN`; and
`decompress`'s doc comment stopped claiming it only ever sees decrypted entries,
which was stale on arrival.

**Four regression tests**, three of which were confirmed to fail with their fix
reverted:

* `an_aes_payload_is_refused_by_name_and_never_blamed_on_the_password` — the
  AES fixture is assembled byte by byte inside the test from APPNOTE §4.3.7,
  §4.3.12, §4.3.16 and §4.5, because nothing in this workspace can *write* AES
  and, more to the point, the defect is about a raw field that any ZIP library
  would overwrite on the way in.
* `a_wrong_password_that_survives_the_check_byte_is_caught_by_the_entrys_crc` —
  needed a second fixture. Against the Deflated fixture, a check-byte false
  accept produces bytes that fail to inflate and the failure is reported long
  before any CRC is compared, so the new check could not be reached at all.
  `fixtures/zipcrypto-stored.knxproj` is the same shape with the nested entry
  **Stored**, generated with the Info-ZIP `zip` CLI exactly as the first was.
  Stored bytes always "decompress", which leaves the CRC as the only gate. The
  test searches for a password that genuinely passes the check byte — about 1
  in 128, so it is arithmetic rather than luck — and asserts the container still
  says `WrongPassword`.
* `a_nested_entry_colliding_with_an_outer_path_is_refused_not_shadowed` —
  `knx_master.xml` in both the outer archive and the payload.
* `the_stored_fixture_decrypts_with_the_right_password` — the new fixture's
  own sanity check, and the only test that exercises `was_decrypted()`.

**Documentation.** `KNOWN_LIMITATIONS.md` §13 now says plainly that **no import
path reaches this decryption**: `import()` calls `Container::open`, the only
callers of `open_with_password` are its own tests, and stage 1 of a six-stage
pipeline opening a protected project is not the pipeline importing one. The same
section gained the roundtrip gap (a decrypted project exported through the opaque
passthrough store comes back out *unprotected*, because the ciphertext is not kept
anywhere) and a corrected account of what happens after a check-byte false accept.
`IMPORT_EXPORT.md` §1's pipeline diagram and prose say the same. `COMPATIBILITY.md`
§2 records the coverage caveat the review found: both container-level fixtures carry
the Info-ZIP DOS-time check-byte convention, so the PKZIP CRC-high-byte convention
is exercised only at the `knx-secure` unit level, never end to end. `ARCHITECTURE.md`
§9 and ADR-0008's Consequences both record the one place a plaintext password now
crosses a crate boundary, and that it is a borrowed parameter — never stored on
`Container`, never in an `EntryInfo`, never interpolated into an error or a
`Display` impl.

*Also closed here:* `IMPORT_EXPORT.md`'s claim that the ZipCrypto password is taken
as UTF-8 bytes carried a `[V]` marker sourced from reading `xknxproject`'s code.
That is evidence about `xknxproject`, not about ETS, and an ASCII password cannot
tell the two readings apart anyway. It is `[A]` now, with the reason written down.

**Gates:** `cargo fmt` `0`, `cargo clippy --workspace --all-targets -D warnings`
`0`, `cargo test --workspace` **1,580 passed / 0 failed / 5 ignored**, 0 corpus
skips, `check-layering` `0`, `check-headers` 119 well-formed / 168 without one
(ceiling 168), `cargo deny check` `0`. No TypeScript was touched.

## 2026-09-16 — Goal Task 17: real-browser verification for “New project” (branch `launcher-browser-verify`)

The from-scratch launcher now has a repeatable browser-level check instead of
only Vitest components backed by a mocked `./api`.
`apps/knx-web/e2e/new-project.e2e.ts` drives the production frontend through
the real `knx-server` in system Chromium. Separate cases create
`ThreeLevel`, `TwoLevel`, and `Free` projects and assert the dialog copy
and defaults, the exact `POST /api/project/new` body, the returned
one-installation tree with no topology or devices, the style shown in project
properties, and the device catalog opened from the empty “Unassigned” branch.
No project file or mocked API response participates.

`apps/knx-web/playwright.config.ts` owns the Linux test boundary: one worker,
the distribution Chromium at `/usr/bin/chromium`, the production build served
by `cargo run -p knx-server`, and output under the already ignored workspace
`target/`. The `*.e2e.ts` suffix and explicit Playwright `testMatch` keep
these tests outside Vitest's `*.test.ts`/`*.spec.ts` discovery. The local
`@playwright/test` dependency and `npm run test:e2e` script make the check
repeatable from a clean npm install.

Measured verification on Chromium 152: `npm test` **468 passed across 42
files**, and `npm run test:e2e` built the production bundle and passed **3/3
Playwright tests**. The browser suite stops after the empty catalog opens;
package installation and device creation remain covered at the server boundary
by `apps/knx-server/tests/http_catalog_to_device.rs`.

## 2026-09-17 — First Linux AppImage: local build, inspection, and bounded launch

[ADR 0021](adr/0021-appimage-is-the-first-linux-package.md) selects AppImage
as KNXBench's first Linux desktop package. The local build on `big-omarchy`
(Arch Linux, Linux `7.2.5-3-omarchy`, x86_64) produced exactly one artifact:
`KNXBench_0.1.0-alpha.1_amd64.AppImage`, `105839096` bytes, mode
`-rwxr-xr-x`, SHA-256
`b0ec49aebcec984ffdce21639306713862fc6c7baf15f7247f060ac314cdaef7`.
The host used Rust `1.98.0`, Cargo `1.98.0`, Node `v26.8.1`, npm `11.19.0`,
Tauri CLI `2.11.4`, and gdk-pixbuf `2.44.7`.

`cargo run -p xtask -- check-appimage` passed. Extraction verified the
executable `usr/bin/knx-desktop`; `KNXBench.desktop` with the expected
`Exec`, `Icon`, and type; the root icon and 128/256-pixel hicolor icons; the
frontend `index.html` and 88 frontend files; and the 34,523-byte bundled
`LICENSE`, byte-identical to the repository's canonical license. The AppImage
started on the active Arch XWayland display, mapped a visible `KNXBench` window
(`Knx-desktop`), and remained alive for 15,003 ms; `timeout` ended it with
status 124. The log contained two GBM allocation warnings, with no missing
library or resource, panic, or server-start failure.

Focused package checks passed: `cargo fmt --all --check`, `cargo test -p xtask`
(46 passed, 0 failed), `cargo clippy -p xtask --all-targets -- -D warnings`,
`check-appimage`, and `git diff --check`. No KNX discovery, gateway contact,
bus read, or bus write occurred. The GitHub Actions workflow is configured to
build/upload artifacts and to publish a pushed tag, but it was not executed;
no tag, release, or push occurred. This is evidence for the local Arch/XWayland
run within the compatible x86_64 glibc, GTK 3, and WebKitGTK 4.1 boundary only,
not Ubuntu CI success or general Linux distribution compatibility.

## 2026-09-17 — Range-less group-address export loss closed

Schema-11 and schema-21 export now reject a group address whose `range` is
`None` with `ExportError::UnrangedGroupAddress`, naming its installation,
internal ID, and raw address. Native `.knxdb` persistence and the optional
range field remain unchanged; no synthetic range is invented. Focused tests
cover both schema writers, replacing the previous successful but lossy export.

## 2026-09-17 — Same-file application-program ID collisions recorded

`parse/program.rs` now routes `ApplicationProgram/@Id` through the shared
`first_winner` helper with a fresh per-file occurrence map. A duplicate ID in
one XML source keeps the first declaration and records `IdConflict` occurrence
2, matching hardware and catalog ingestion. The prior cross-file behavior stays
unchanged. `datapoint_type` collision provenance remains separate because its
table lacks `source_sha256`; the existing dropped-declaration counter remains.

## 2026-09-18 — T30 R20 read-only comparison

A temporary, uncommitted hardware probe narrowed the known management-session
timeout defect without sending any write service. In reverse order on one
shared tunnel, first target `1.1.32` answered mask `0701h` and all later targets
timed out, ruling out a device-specific explanation for the earlier sole
`1.1.24` success. A fresh tunnel per target yielded alternating success and
timeout, so fresh tunnels alone are not a reliable fix and immediate tunnel
lifecycle remains involved. `1.1.220` was structurally excluded before socket
open. The temporary test was deleted; R20 remains open and the independent scan
probe remains the required presence check.
