# Roadmap

Seven sessions after the research phase. Each states its goal, its
deliverables, and the entry condition that must hold before it starts. A
session that starts without its entry condition met produces work that has to
be redone.

Current position: **Session 3 complete**. See
[IMPLEMENTATION_STATUS.md](IMPLEMENTATION_STATUS.md).

## Session 0 — Technical research

**Done.** See [RESEARCH.md](RESEARCH.md).

## Session 1 — Architecture

**Done.** The stack, the layering, the domain model design, the import and
export contract, the compatibility statement, nine ADRs, and a Cargo workspace
with two mechanically enforced rules and CI.

## Session 2 — KNX core

**Done.** Implemented the domain model of [DATA_MODEL.md](DATA_MODEL.md) in
`knx-core`: entities (project, installation, area, line, device, group
address, group range, communication object instance, parameter instance,
building part); typed addresses with parsing and formatting; datapoint type
references; the override resolution that turns three source layers into
`Resolved<T>`; validation rules; the string table; the command layer with
inverses for undo and redo; the schema version and the migration chain
skeleton in `knx-store`.

Not part of this session's delivery: the opaque passthrough store
([ADR-0006](adr/0006-opaque-passthrough-store.md)) — nothing exists yet to
pass through, since there is no importer. That lands in Session 3 alongside
the code that populates it.

**Entry condition.** The workspace builds, and both gates pass —
`cargo run -p xtask -- check-layering` and `cargo deny check`. Met.

## Session 3 — ETS project import

**Done.** See [IMPLEMENTATION_STATUS.md](IMPLEMENTATION_STATUS.md) and
[COMPATIBILITY.md](COMPATIBILITY.md) (every verified claim names its test).

**Goal.** Read the reference project into the model, and write it back.

**Deliverables, all shipped.** The six-stage pipeline of
[IMPORT_EXPORT.md](IMPORT_EXPORT.md) for schema 11; the opaque passthrough
store (`knx-store` schema v2); the `ImportReport`; the golden test asserting
the reference project's measured counts (36 devices including the
unassigned one, 514 group addresses, 907 communication object instances,
1390 parameter values — 1174 plain and 216 union, 569 send and 27 receive
links, 261 valued / 497 empty / 149 absent datapoint types); the oracle
comparison against `xknxproject` where it is not known to be lossy; the
three roundtrip guarantees plus a fourth convergence check; the
malformed-input suite (10 tests, including a container entry-size guard and
a 10,000-level nesting-depth check); the `knx import` CLI subcommand.

**Not part of this session's delivery**, carried into Session 4: entity
persistence into SQLite beyond the opaque table; the schema-23
known-element table (schema 23 is detected and refused by name, not
misread); manufacturer data moving from the per-project opaque store into
the shared product database.

**Entry condition.** The core model exists and is testable without IO. Met.

**Blocking risk, still open.** No ETS5 or ETS6 sample project is available
(risk R1) beyond the one project's two exports already diffed (RESEARCH
§2.4/§3.3). Acquiring one is a prerequisite for claiming support beyond
schema 11 — without it, the tolerant parser reports unknown constructs and
that is all we can honestly say.

## Session 4 — Manufacturer databases

**Done.** See [IMPLEMENTATION_STATUS.md](IMPLEMENTATION_STATUS.md),
[IMPORT_EXPORT.md §10](IMPORT_EXPORT.md), [ADR-0011](adr/0011-product-database-storage.md),
[ADR-0012](adr/0012-enrichment-into-absent-slots.md).

**Goal.** The shared product database.

**Deliverables, all shipped.** `knx-productdb`: its own SQLite schema and
migration chain (manufacturer → hardware/product/hardware2program →
application program → parameter types/parameters → communication objects →
translations, plus the `knx_master.xml`-derived manufacturer names and DPT
catalogue); content-hashed blob storage alongside the parsed tables
([ADR-0011](adr/0011-product-database-storage.md)); ingest from
`.knxproj` manufacturer data keyed by content hash, skipping existing
entries, replacing Session 3's per-project opaque-store arrangement
([KNOWN_LIMITATIONS.md](KNOWN_LIMITATIONS.md) §12); the project manifest
(`knx-store` schema v3) naming what a project was imported with,
independent of whether the product database itself is present; graceful
degradation when the database is missing, tested rather than asserted;
enrichment of `ComObjectInstance` from the resolved application program
into `Override::Absent` slots only ([ADR-0012](adr/0012-enrichment-into-absent-slots.md));
the `knx products` CLI subcommand and `--product-db`/`--no-product-db` on
`knx import`.

**Not part of this session's delivery**, carried forward, each its own
future cycle rather than bundled into "Session 4 leftovers":

- **Full entity persistence** of `knx_core::Project` into `knx-store`'s
  SQLite tables, beyond the opaque and manifest tables. Two sessions have
  now deferred this in turn (Session 3's IMPLEMENTATION_STATUS correction,
  this session again) — it belongs at the start of whichever session
  first needs to *save* an edited project, which is Session 5.
- **The `when/@test` expression grammar spike** (risk R3), still
  unresearched and still its own research cycle rather than a feature: the
  `Dynamic` tree is not parsed at all, and the raw bytes are retained
  regardless (ADR-0011), so no data is lost by deferring this again.
- **A layer stack in `Override<T>`** that would make a program value
  behind an instance-level `Empty` slot visible without risking the export
  change ADR-0012 identifies. A domain-model change with a migration; not
  worth taking for visibility alone without a consumer (the parameter
  editor, or a UI) that needs it.
- **Resolving an ambiguous, space-separated `DatapointType` list**
  (RESEARCH §4.2) from context — e.g. a linked group address's own DPT.
  Needs the group-address/communication-object cross-reference a later
  session's entity persistence would make queryable; guessing from one
  communication object alone is not attempted.
- **`.knxprod` direct ingest** for master data scheme ≥ 12
  (KNOWN_LIMITATIONS §11) and **schema 23 manufacturer data** — both share
  a blocker already tracked (the container/encryption layer, and the
  schema-23 known-element table, respectively) and neither is closer to
  resolution after this session.

**Entry condition.** Import produces application program references worth
resolving. Met.

## Session 5 — UI and UX

**Goal.** The desktop application.

**Deliverables.** `apps/knx-desktop` — the Tauri shell and the React
application, scaffolded here rather than in Session 1; the projection layer
(`ProjectTree`, `DeviceList`, `GroupAddressTable`, `Inspector<T>`) with `ts-rs`
bindings and a CI check that they are not stale; Project Explorer, properties
inspector, search, command palette, dark and light mode. Rule 3 of
[ARCHITECTURE.md](ARCHITECTURE.md) section 4 becomes a mechanical gate in this
session, since a UI finally exists to check.

Cycle 1 (this document's own scope split, see
`docs/superpowers/specs/2026-09-03-knx-desktop-shell-design.md`) delivered
the shell, the projection layer, and Project Explorer. Inspector, search,
command palette and dark/light mode are later cycles of this same
session, all now shipped.

Cycle 2 (`docs/superpowers/specs/2026-09-03-knx-entity-persistence-design.md`)
delivered `knx-store` entity persistence for the full `knx_core::Project`
(schema v4): one row-writer module per entity, `save_project`/
`load_project` as a full round trip, and `sync_after_command` incremental
sync for the four `Command` variants that exist today
(`SetIndividualAddress`, `SetComObjectDpt`/`RestoreComObjectDpt`,
`CreateGroupAddress`/`DeleteGroupAddress`). This is storage-layer only —
no Tauri `save_project`/`load_project` command and no desktop save UX yet.
Inspector, search, command palette and dark/light mode have since
shipped.

Cycle 3 adds the knx-app wiring cycle 2 left open: `save_project`/
`save_project_as`/`open_native_project` Tauri commands over a `.knxdb`
file (always a full round trip via `knx_store::{save_project,
load_project}`, no incremental sync), plus Save/Save As/Open buttons in
the frontend. A second, independent file format from ETS `.knxproj`
import — `AppState` now tracks both the in-memory project and, separately,
the `.knxdb` path it was last saved to or loaded from. Inspector, search,
command palette and dark/light mode have since shipped.

Cycle 4 (`docs/superpowers/specs/2026-09-04-selection-inspector-design.md`)
delivers device selection in the Project Explorer and a Properties
Inspector panel: clicking a device row shows its name, description,
editable individual address, and one editable DPT field per communication
object with its resolved-layer badge. Edits run through the app's
pre-existing `CommandStack` (`AppState` gains `command_stack` and
`import_counts`), giving Undo/Redo toolbar buttons and `Ctrl+Z`/
`Ctrl+Shift+Z` shortcuts for free. Five new Tauri commands:
`device_detail`, `set_individual_address`, `set_com_object_dpt`, `undo`,
`redo`. Search, command palette and dark/light mode have since shipped.

Cycle 5 (`docs/superpowers/specs/2026-09-04-search-design.md`) delivers
`Ctrl+K` search across devices, group addresses, and building parts.
`knx-projection` gains group addresses on `InstallationNode` (formatted
per `project.info.group_address_style`, not shown anywhere in the UI
before this cycle). The frontend's `Selection` type generalizes from
device-only to a union over all three kinds, `ProjectExplorer` and
`Inspector` generalize accordingly (group addresses and building parts
render read-only in the Inspector — no `Command` exists for either yet),
and `searchMatch.ts`'s `matchEntries` ranks and caps matches for the new
`Search.tsx` overlay component. CI now runs the frontend's own Vitest
suite. Command palette and dark/light mode have since shipped (cycles 6
and 7, below).

Cycle 6 (`docs/superpowers/specs/2026-09-04-command-palette-design.md`)
delivers the `Ctrl+Shift+P` command palette. `commandRegistry.ts` is a
static list of the app's seven existing actions (the same ones cycles
1-4's toolbar buttons already expose) paired with an `isEnabled`
predicate and a plain case-insensitive substring filter over their
labels — seven static entries need no fuzzy ranking. `CommandPalette.tsx`
reuses `Search.tsx`'s modal-overlay structure (autofocused input,
arrow-key navigation, click-outside-to-close) as one flat list rather
than grouped by kind, with disabled entries rendered greyed-out and
skipped by arrow navigation rather than hidden. Opening the palette
closes an open search overlay and vice versa — `Ctrl+K` and
`Ctrl+Shift+P` are mutually exclusive — and the palette works with no
project loaded, same as the two Open actions it lists.

Cycle 7 (`docs/superpowers/specs/2026-09-04-dark-light-mode-design.md`)
delivers a three-state (System/Light/Dark) theme toggle. `theme.ts` holds
a pure cycling function and `localStorage`-backed load/save functions
with storage injected as a parameter, plus a `useTheme()` hook that
applies `data-theme` to `<html>` and persists on change; `ThemeToggle.tsx`
is a hand-written inline-SVG icon button (sun/moon/monitor), no
icon-library dependency. `styles.css` gains three CSS custom properties
for the only colors that don't already adapt via `currentColor`/system
color keywords — the error color and the search/palette overlay's
backdrop and shadow — with a `prefers-color-scheme` media-query default
and `:root[data-theme]` overrides that outrank it by CSS specificity
regardless of source order. `index.html` gets a small inline script
applying a persisted explicit choice before React mounts, avoiding a
one-frame flash of the wrong theme. CLAUDE.md's UI/UX deliverable list
for this session — Project Explorer, properties inspector, search,
command palette, dark and light mode — is now complete.

Cycle 8 (`docs/superpowers/specs/2026-09-04-dashboard-design.md`) delivers
a read-only project status dashboard, filling the workspace slot that
sat empty when a project was loaded but nothing selected. CLAUDE.md's
Session 5 UI/UX deliverable list was already complete as of cycle 7 —
this is a bonus item pulled from `ideas.md`, not a gap-closer.
`knx-projection` gains one new field, `DeviceNode.com_object_count`
(`build_device_node` populates it from `device.com_objects.len()`), the
one piece of data the frontend didn't already have; every other stat
(installations, areas, lines, devices assigned/unassigned, group
addresses, building parts) is summed from the existing `ProjectTree`
shape by a new pure frontend module, `dashboardStats.ts`, walking
topology and the unassigned bucket for devices/communication objects and
buildings separately for building-part counts — deliberately never
sharing a device between the two walks, since a device is placed in a
line and a building independently (DATA_MODEL.md) and summing both
would double-count it. `Dashboard.tsx` is a presentational component
consuming `computeStats`, rendered in `App.tsx` in place of the
`{selection && <Inspector>}` no-op via `{selection ? <Inspector> :
<Dashboard>}` — mutually exclusive with the Inspector the same way
Inspector and nothing-selected were before this cycle, no new toolbar
button, overlay, or keyboard shortcut. No clickable stat drills into a
filtered view this cycle; `ideas.md` records that as a deliberate future
candidate, since no error-detail view exists yet to drill into.

Cycle 9 (no design spec — a bounded task, brainstormed directly in chat)
delivers group address create and delete: a "Group Addresses" tree branch
per installation (existing `GroupAddressNode` data, previously reachable
only through `Ctrl+K` search, never rendered as a branch) with an inline
create row, and a Delete button on the group-address inspector. Two
`knx-core` validation guards land alongside the UI: `Command::
CreateGroupAddress` now rejects a duplicate group address value
(`ValidationError::DuplicateGroupAddress`), and `Command::
DeleteGroupAddress` now refuses to delete an address still named by any
`ComObjectInstance.links` entry (`CommandError::GroupAddressInUse`) —
closing [KNOWN_LIMITATIONS.md §17](KNOWN_LIMITATIONS.md) by refusing the
dangerous delete rather than cascading it, one of the two resolutions
that limitation's entry had left open. The create row, and (after this
review's fix) the Delete button, both only act on the first installation,
since `Command::apply` only ever targets `installations[0]`.

Cycle 10 ([design spec](superpowers/specs/2026-09-05-toast-easter-eggs-design.md))
delivers three `ideas.md` entries bundled together, since two of them
needed infrastructure that didn't exist yet and the third reuses it: a
toast notification stack (`toast.ts`/`toastCopy.ts`/`Toast.tsx`)
replacing the old single persistent error banner one-for-one; a
lighter, humorous tone on error toasts (the raw backend message is
wrapped in a randomly chosen template, never edited, so the factual
core survives verbatim); and a one-shot startup toast for a listed
holiday or, failing that, a late-night session — never both. Backend
error text is untouched; humor lives entirely in the frontend wrapper.

**Entry condition.** Import produces a model worth displaying.

Cycle 11+ candidates (from `ideas.md`, not yet scheduled), each with no
architectural blocker remaining now that dark/light mode and toasts have
shipped: fine-grained UI animation, deliberately held until dark/light
mode landed so it wouldn't be redone against components that were still
changing structurally — that gate is now clear. Renaming the project to
KNXBench (repo, README, docs) is done; renaming the crates themselves
(`knx-core`, `knx-store`, `knx-desktop`, `knx-server`, `knx-cli`, …) is a
deliberate non-goal — decided against, not merely deferred, since every
one of them has downstream dependents and a rename would only ever be
cosmetic. Cycle 11 is fine-grained UI animation.

## Cross-cutting — Web/Docker deployment target

**Done.** Not part of the original Session 0-7 breakdown above — added by
explicit request alongside Session 5, tracked in
[the design spec](superpowers/specs/2026-09-05-web-docker-deployment-design.md)
and folded in here once implemented, same as that spec's own header flags
it. See [IMPLEMENTATION_STATUS.md](IMPLEMENTATION_STATUS.md) for what
shipped.

**Goal.** A second deployment path — a Docker container exposing a web UI
reachable from any device on the LAN — without duplicating the frontend or
the application logic, and without requiring the native
Tauri/WebKit2GTK stack on the host.

**Deliverables, all shipped.** One HTTP API (`apps/knx-server`, axum) as
the frontend's only integration point, replacing Tauri IPC; the frontend
moved from `apps/knx-desktop/src` to a standalone package, `apps/knx-web`,
talking to it over `fetch()`; `apps/knx-desktop/src-tauri` reduced to a
thin wrapper that spawns `knx-server` locally and points its WebView at
it — one frontend, one API surface, two ways to run it, per the design's
"converge, don't duplicate" decision; a three-stage Docker build with no
GTK/WebKit2GTK in the final image; a scripted smoke test
(`apps/knx-server/scripts/smoke-test.sh`) that builds the image, runs it,
and imports the reference project over HTTP against a mounted volume.
LAN-only, no auth, single in-memory project, by design (see
[KNOWN_LIMITATIONS.md](KNOWN_LIMITATIONS.md)) — matching the stated use
case, not a gap.

**Not part of this delivery, stays open.** KNXnet/IP multicast discovery
does not work unmodified inside a Docker container — it needs
`--network host` — and this delivery does not solve that, since no bus
communication feature exists yet to need it. That constraint is
Session 6's to account for when it starts: discovery, tunnelling and
routing all assume host network access is either already granted or is
itself part of Session 6's container-specific deliverables. Nothing about
this delivery blocks Session 6; it simply doesn't attempt Session 6's
problem early.

**Entry condition.** None — this was schedulable independently of the
Session 0-7 sequence, since it adds a transport layer in front of
already-shipped application logic rather than new domain capability.

## Session 6 — KNXnet/IP

**Goal.** Talk to the bus.

**Status.** Cycle 1 (2026-09-06) delivered read-only tunnelling per the 2026-09-06
design spec: `crates/knx-net` codec modules, `TunnelClient` state machine,
live-gateway integration test, and `knx bus monitor` CLI subcommand. Discovery,
sending, routing, and KNX IP Secure remain for later cycles.

**Deliverables.** The `BusConnection` trait implemented against ISO 22510:
discovery, tunnelling, routing, cEMI and telegram encoding; the bus monitor as
a consumer that resolves telegrams against the open project; connection
management and diagnostics. Device discovery (`ideas.md`) is this session's
`discovery` deliverable, not a separate feature — it cannot start earlier.

**Carried in from the web/Docker deployment target, above.** KNXnet/IP
multicast discovery does not work unmodified inside the `knx-server`
Docker container — it needs `--network host` — a constraint noted, not
solved, when that target was built, since no bus feature existed yet to
need it. This session has to account for it: either document that the
container deployment path requires `--network host` for discovery to
work, or design around it.

**Entry condition.** A project can be opened and its group addresses resolved,
so that captured telegrams have something to resolve against.

## Session 7 — Integration and hardening

**Goal.** Make it trustworthy on real projects.

**Deliverables.** The full roundtrip and migration suites with frozen fixtures
per schema version; performance measurement on large projects, with
optimization driven by those measurements rather than by guesswork; packaging
for Linux; the licence decision.

**Entry condition.** All earlier sessions' deliverables exist and are tested.

Deferred beyond Session 7 (from `ideas.md`, no fixed session): MCP
capabilities and automation of repetitive tasks both need a mature,
near-complete `Command` layer as their foundation — premature before
Session 7. A live "who talks to whom" group-address/device animation is
more valuable once Session 6's bus monitor can feed it real telegrams
rather than only static group links. A mobile app and non-Linux desktop
support are new-platform work, out of scope while the Linux-first desktop
(CLAUDE.md) is still incomplete. An in-app project documentation/notes
feature is a new domain concept absent from
[DATA_MODEL.md](DATA_MODEL.md) — needs its own ADR before implementation,
not bundled into a UI cycle.

## Open questions and where they land

Carried forward from [RESEARCH.md](RESEARCH.md) §12. None of them block the
architecture; each has a defined landing place.

| Question | Lands in |
| --- | --- |
| ETS5 and ETS6 schema deltas (13, 14, 20, 21+) | Session 3 delivered the tolerant parser (reports unknown constructs rather than failing) and schema-23 detection-and-refusal by name; the per-version known-element lists themselves still need an independent sample per version (risk R1) — no fixed session, lands whenever one becomes available |
| `Functions` element semantics | Session 2 — a domain model addition; absent from the reference sample |
| `when/@test` expression grammar | Session 4 built the product database around it staying unparsed (`Dynamic`'s raw bytes retained regardless, ADR-0011); the grammar spike itself remains a research cycle, no fixed session |
| Whether ETS re-imports an unsigned third-party `.knxproj` | Session 3 delivered the mechanism (`ExportWarning::Unsigned`, always present); the verification itself — opening an export in real ETS — is still open (risk R9), no fixed session |
| Whether Data Secure runtime keys are readable from `.knxproj` | Session 7 or later — `knx-secure` |
| `.knxprod` encryption for master data scheme ≥ 12 | Session 4 delivered `.knxproj`-sourced product database ingest; direct `.knxprod` ingest for scheme ≥ 12 remains out of v1 scope, no fixed session |
| The project licence | Session 7 — currently a placeholder, see [KNOWN_LIMITATIONS.md](KNOWN_LIMITATIONS.md) |
