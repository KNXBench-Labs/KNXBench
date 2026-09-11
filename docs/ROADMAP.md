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
- **The `when/@test` expression grammar spike** (risk R3) **ran 2026-09-11**
  (RESEARCH §4.3): the Standard normatively specifies the `@test` value
  grammar; the `Dynamic` tree's structural grammar remains corpus-observed
  only. **T18's first slice shipped the same day**: `knx-productdb` now
  parses and stores the `Dynamic` tree losslessly (schema v3) and
  evaluates it headlessly, with the no-match-branch policy decided as an
  inference (nothing under an unmatched `choose` activates). **T18 slice
  2 also shipped the same day:** the evaluator now expands a `Module`
  node into its `ModuleDef`'s own stored tree. What remains of T18
  (parameter interpretation and editor,
  [GAP_ANALYSIS_ETS.md](GAP_ANALYSIS_ETS.md) Tier 5) is now just the
  editor itself — no fixed session yet.
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

Cycle 11 delivers the fine-grained UI animation candidate — held until
dark/light mode landed so it wouldn't be redone against components that
were still changing structurally — bundled with a second, user-requested
piece: user-customizable theme tokens. `palette.ts` layers four
freely-colorable tokens (`accent`, `bg`, `surface`, `text`) on top of
cycle 7's System/Light/Dark cycle, applied as inline CSS custom
properties on `<html>` so an unoverridden token falls through to the base
theme via ordinary cascade. Motion ships as a three-level
`off`/`subtle`/`standard` setting mapped to a transition-duration token,
gated entirely inside `prefers-reduced-motion: no-preference` — the OS
setting always wins over the user's choice. `ThemePanel.tsx`, opened from
a new gear button beside `ThemeToggle`, is the settings surface for both.
(Both were removed again by cycle 13 along with `ThemePanel.tsx` — the
motion setting has not been replaced; see "Cross-cutting — Motion and
animation" below.)
Renaming the project to KNXBench (repo, README, docs) is done; renaming
the crates themselves (`knx-core`, `knx-store`, `knx-desktop`,
`knx-server`, `knx-cli`, …) is a deliberate non-goal — decided against,
not merely deferred, since every one of them has downstream dependents
and a rename would only ever be cosmetic.

Cycle 13 ([design spec](superpowers/specs/2026-09-08-bitcoin-defi-theme-design.md))
replaces cycle 7's System/Light/Dark cycle and cycle 11's four-token
palette override outright with a named, selectable theme — a complete
visual package (colors, typography, radii, shadows, texture) rather than
a per-user color tweak layered on a light/dark base. "Bitcoin DeFi" is
the first theme and today's default; the registry is built to hold more
later even though it holds one entry now. `theme.ts` is rewritten from a
System/Light/Dark cycling function into a `ThemeDef`/`THEMES` registry
with `loadThemeId`/`saveThemeId`/`useThemeId`; `palette.ts`,
`palette.test.ts`, and `ThemePanel.tsx` are deleted outright rather than
migrated, since a 4-token override doesn't map onto a 12+-token theme
package. `ThemeToggle.tsx`'s sun/moon/monitor cycle button is replaced by
`ThemeSwitcher.tsx`, a `<select>` built against the `THEMES` registry so
a second theme needs no UI change. `index.html`'s inline bootstrap script
now always sets `data-theme` — Bitcoin DeFi is dark-only by design, so
there is no more "system"/unthemed state — and silently falls back any
of cycle 7's old stored values (`"system"`/`"light"`/`"dark"`) to the new
default. Fonts load two ways: self-hosted `@fontsource` packages (Space
Grotesk, Inter, JetBrains Mono) imported in `main.tsx` as the guaranteed
offline fallback, and a Google Fonts `<link>` in `index.html` for a
CDN-first look, per the user's request — no security header blocks the
external request in either deployment target. `styles.css` gains a
24-custom-property design-token layer and component recipes app-wide:
pill-shaped gradient/glow buttons, glass-morphism overlays, mono/gold
technical text, gradient-text on the Dashboard heading, card hover-lift,
and a fading grid-pattern background. Landing-page-only pieces of the
source design system — orbital hero, pricing tiers, blockchain timeline —
have no target in this data-dense project editor and are deliberately
left unbuilt.

Cycle 14+ candidates (from `ideas.md`, not yet scheduled).

## Cross-cutting — Internationalization

**Planned, no cycle scheduled.** Added 2026-09-10 by explicit request.
Not part of the original Session 0-7 breakdown, and deliberately not
folded into Session 5 as "cycle 14", because only half of it is UI work:
the other half reaches into `knx-core`'s string table and
`knx-productdb`'s translation storage, which belong to Sessions 2 and 4.

Two tracks, tracked as **T25** and **T26** in
[GAP_ANALYSIS_ETS.md](GAP_ANALYSIS_ETS.md)'s Tier 6, closing gap **D10**:

1. **UI chrome (T25).** Every user-facing string in `apps/knx-web` is a
   hard-coded English literal today, and the frontend has no i18n
   dependency at all. Extract them into a message catalogue, detect the
   locale, and add an explicit language setting. German first — it is the
   language of the KNX Association's documentation and of the sample
   projects this project is tested against.
2. **KNX data (T26).** `knx_core::string_table` (`Language`,
   `LocalizedString`, `StringTable` with a `default_language` fallback)
   and `knx-productdb`'s `translation` table already exist and are
   already populated on import — one application program alone carries
   5919 translation elements, which is why the indirection went into the
   model on day one. Nothing reads either of them. T26 adds an active
   language and the display-side resolution that turns stored
   translations into rendered text.

The library choice for track 1, the storage of the language setting
(D8's options dialog does not exist yet), and what a project's own
`Language` means once the user can pick a different one all belong in a
design spec, not here. Neither track has one yet.

## Cross-cutting — Motion and animation

**A standing constraint, not a cycle.** Added 2026-09-10 by explicit
request: animations must be toggleable, at the moment they are
implemented — not retrofitted with a switch afterwards.

The rule, stated once so no later cycle has to re-decide it: **any
animation this application ships is switchable off from inside the
application, and `prefers-reduced-motion: reduce` always wins over
whatever the user has chosen in-app.** An OS-level accessibility setting
is not something an application setting may override; the in-app control
exists for the people whose OS says nothing and who still want the UI to
sit still.

Where this stands today, which is worse than it was two cycles ago:
cycle 11 shipped exactly such a control — a three-level
`off`/`subtle`/`standard` motion setting driving
`--knx-transition-duration` — and cycle 13's theme rewrite deleted
`ThemePanel.tsx`, the surface it lived on, without replacing it (its own
[design spec](superpowers/specs/2026-09-08-bitcoin-defi-theme-design.md)
records the loss: "Motion: no user-facing setting (that was `palette.ts`'s
job, now gone)"). What survives is the CSS token and three
`prefers-reduced-motion: no-preference` blocks in `styles.css`. So the
OS preference is currently the only control, and it is all-or-nothing.

Restoring the control, and binding every future animation to it, is
tracked as **T27** in [GAP_ANALYSIS_ETS.md](GAP_ANALYSIS_ETS.md)'s
Tier 7, closing gap **D11**. The constraint applies to work already on
this roadmap that has not been built yet: T15's Group Monitor table,
T17's line-scan UI, T21's graphical topology/building views, and the
"who talks to whom" telegram animation deferred beyond Session 7 below.
That animation is the reason this is written down now rather than at
implementation time — it is the first genuinely motion-heavy feature on
the list, and the cheapest moment to require a switch for it is before
anyone starts writing it.

**Memo (2026-09-10), style direction — not decided, not designed, no
task opened.** Two candidate visual directions were named for whenever
T27's motion work (and any theme it rides alongside) actually gets
designed: (1) an Apple-like direction — sleek, subtle, clean, restrained
motion; (2) a "techy glitch / cyberpunk OS" direction that is still, per
the same request, clean and sleek rather than noisy or gimmicky. Neither
is chosen. Recorded here only so it is not lost before T27 gets a design
spec; whoever writes that spec should treat this as a starting prompt,
not a constraint.

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

## Cross-cutting — LLM / natural-language interaction

**Memo (2026-09-10) — not decided, not designed, no task opened, no
research done yet.** Added by explicit request: a chat window connected to
an LLM, usable for natural-language interaction with the project — and,
per the request, not limited to being MCP-capable from the outside, but
also directly built into the application itself (an in-app chat surface,
not only an MCP server other tools could drive).

This is explicitly a research item before it is a design item: *how* such
a thing should work — which capabilities it gets, how it is authorized
against a live project, how "natural language" maps onto the existing
`Command` layer (the same layer the deferred MCP note above already
identifies as the load-bearing prerequisite), which LLM(s) it talks to and
whether that is local, remote, or configurable, and what it must never be
allowed to do unsupervised to project data — needs to be researched and
written up (`docs/RESEARCH.md`, per this project's own documentation rule)
before any design spec is attempted.

Relationship to the existing deferred MCP note above: that note already
says MCP capabilities need a mature, near-complete `Command` layer as their
foundation, and are premature before Session 7. This item is the same
dependency, plus a second, in-application surface on top of it — so it is
at least as far out, and should not be scheduled ahead of the `Command`
layer's own completion. Recorded here only so the idea is not lost before
someone does that research; whoever picks it up should treat this
paragraph as a starting prompt, not a constraint.

## Session 6 — KNXnet/IP

**Goal.** Talk to the bus.

**Status.** Cycle 1 (2026-09-06) delivered read-only tunnelling per the 2026-09-06
design spec: `crates/knx-net` codec modules, `TunnelClient` state machine,
live-gateway integration test, and `knx bus monitor` CLI subcommand. Cycle 2
(2026-09-06, bounded — no separate design spec) delivered sending:
`cemi::encode_l_data` (the inverse of Cycle 1's decode), `TunnelClient::send`
implementing Tunnelling v01.07.01 AS §2.6's wait-1s/retry-once/
disconnect-on-repeated-failure rule, and a `knx bus write` CLI subcommand.
Cycle 3 (2026-09-06) delivered discovery: `core::dib` DIB decoding,
`KnxNetIpClient::discover`, and a `knx bus discover` CLI subcommand.
Cycle 4 (2026-09-06) delivered routing: `RoutingClient` over the
standard multicast group, and `knx bus route-monitor`/`route-send` CLI
subcommands. KNX IP Secure was scoped next but explicitly shelved for a
later cycle (out of proportion for one cycle: ECDH handshake, a new TCP
transport for unicast sessions, AES-CCM, and a `.knxkeys` keyring format
with no research spike done yet); Cycle 5 (2026-09-06) picked up the
session's other remaining deliverable instead — connection management and
diagnostics — closing three known gaps in the existing `TunnelClient`/
`RoutingClient` flows (heartbeat retry race, no shutdown signal to
subscribers, `ROUTING_BUSY` not honored).

**Decision (2026-09-06).** KNX IP Secure is shelved indefinitely, not
merely deferred to "a later cycle" — no fixed session or cycle owns it.
Rationale: plain tunnelling/routing already covers the common case
(legacy gateways, and current-generation gateways not configured
secure-only); IP Secure only matters for secure-only gateways or
installations with IP Secure explicitly enabled. Revisit when a real
gateway actually needs it, not speculatively — at which point do the
RESEARCH.md §9 spike (ECDH handshake, TCP unicast transport, AES-CCM,
`.knxkeys` keyring; whether keys are even readable from `.knxproj` or
only `.knxkeys`) before implementing. See
[KNOWN_LIMITATIONS.md §26](KNOWN_LIMITATIONS.md).

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

**Status.** Cycle 1 (2026-09-06) audited and froze what "per schema version"
actually means given the fixtures on hand: schema 11 (ETS4 reference
project) already had full roundtrip/oracle/malformed-input coverage —
unchanged. Schema 23 turned out to already have a real-file (not just
synthetic-namespace) refusal test. Schema 21 did not: `KV v2.5 -
demo.knxproj`, a genuinely independent second sample discovered in
`OriginalData/` (gitignored there, committed to the repo root as a fixture
here, same treatment as the other two), got the matching real-file refusal
test. Comparing it against the schema-11 reference (RESEARCH §3.4) found
that every structural delta previously attributed to schema 23 alone
(`Segment`, `GroupObjectTree`, `Puid`, `Locations`) already exists at schema
21, plus one not seen before (`ModuleInstances`, modular application
programs) — flagged as the next major format-support task, not attempted
here (see [KNOWN_LIMITATIONS.md §1](KNOWN_LIMITATIONS.md)). `knx-store`'s
`.knxdb` migration chain (v1→v4, frozen fixtures) was verified, not
touched — already complete, 14/14 tests green. **Cycle 2, T29
(2026-09-11, branch `t29-dpt-codec`):** KNXBench's first DPT codec, in
`knx-core` (fourteen main types — see
[IMPLEMENTATION_STATUS.md](IMPLEMENTATION_STATUS.md) and
[KNOWN_LIMITATIONS.md §61](KNOWN_LIMITATIONS.md#61-the-dpt-codec-covers-fourteen-main-types-infers-rather-than-reads-its-input-and-leaves-several-encoding-questions-to-a-stated-ruling-rather-than-the-standard) for exactly which ones
and why not the rest), with `bus monitor`/`bus write` in `apps/knx-cli`
wired to decode/encode against it. `cargo test --workspace`: 920 passed, 0
failed, 3 ignored. A user can now read `knx bus monitor --project <path>`
output as `On`, `23.5`, or a percentage instead of a raw hex payload, and
write with `knx bus write --dpt DPST-9-1 23.5` instead of computing the
wire encoding themselves — for the main types this cycle covers. Closes
[GAP_ANALYSIS_ETS.md](GAP_ANALYSIS_ETS.md) row **E4** partially; the GUI
(**D5**, **T15**) is still open and now has this codec to build on.

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

## Cross-cutting — In-application help and user documentation

**Deliberately last.** Added 2026-09-10 by explicit request: an in-app
help system — hover explanations, contextual help, an actual manual —
scheduled at the end of everything rather than folded into the UI cycles
that create the things it would describe.

The reason it goes last is the reason it keeps being postponed everywhere
else: help text describes a specific UI, and a UI that is still being
built invalidates its own help every cycle. Writing hover text for a
panel that cycle 13 will delete (as cycle 13 deleted `ThemePanel.tsx`) is
work done twice and wrong once. So this waits until the surfaces have
stopped moving — which, per the sessions above, means after Session 7's
hardening and after the outstanding UI backlog (T17, T18, T21) has
either shipped or been dropped.

What exists today, measured rather than remembered: **one** `title`
attribute in the entire frontend (`Inspector.tsx:218`, showing a
communication-object flag's raw name), four `aria-label`s, no
`aria-describedby` anywhere, no tooltip component, no help panel, no
`F1` handler, and no end-user documentation of any kind — all nine files
in `docs/` are architecture and format documentation written for
developers, and none of them is reachable from inside the application.
The closest thing to user-facing guidance is `commandRegistry.ts`'s
`shortcutHint` field, which surfaces only inside the Command Palette.

The shape this should take is not decided here — that belongs in a design
spec — but the questions it has to answer are: hover/tooltip text versus a
persistent context panel versus both; where help text lives so it can be
translated alongside T25's UI chrome rather than after it; whether KNX
concepts (what a group address *is*, what the five communication-object
flags mean) get explained in-app or linked out to the KNX Association's
own material; and whether any of `docs/` is shipped to the user or
whether user documentation is written separately from the start.

Tracked as **T28** in [GAP_ANALYSIS_ETS.md](GAP_ANALYSIS_ETS.md)'s Tier 8,
closing gap **D12**. Two things it is *not*: it is not the deferred in-app
project documentation/notes feature listed under Session 7 above — that
one stores notes *about a project* and needs a `DATA_MODEL.md` addition
and an ADR — and it is not T13's project documentation export, which
prints a project rather than explaining the application.

## Open questions and where they land

Carried forward from [RESEARCH.md](RESEARCH.md) §12. None of them block the
architecture; each has a defined landing place.

| Question | Lands in |
| --- | --- |
| ETS5 and ETS6 schema deltas (13, 14, 20, 21+) | Schema 21 import+export shipped and round-trip verified (one sample, KV demo project); schema 23 import shipped, module handling inferred not evidenced (no independent module-using schema-23 sample); schema 12, 13, 14, 20, 22 remain undocumented-by-evidence — no fixed session, lands whenever a sample becomes available for each. `Dynamic`/`choose`/`when`'s grammar (mentioned in the row below this one) is now provably avoidable for import (ADR-0014: `GroupObjectTree` already carries ETS's own resolution of it) rather than blocking; the `@test` value grammar itself is documented (RESEARCH §4.3), so this is now purely a parameter-editing (T18) concern, no longer a research one, and not an import one either. |
| `Functions` element semantics | Session 2 — a domain model addition; absent from the reference sample |
| `when/@test` expression grammar | **Answered 2026-09-11** (RESEARCH §4.3): the Standard normatively specifies the `@test` value grammar; `Dynamic`'s structural grammar stays corpus-observed only. Session 4 built the product database around `Dynamic` staying unparsed regardless (`Dynamic`'s raw bytes retained, ADR-0011). **T18 slice 1, same day:** `knx-productdb` now parses, stores (schema v3, `dynamic_node`) and evaluates the tree headlessly. **T18 slice 2, same day:** the evaluator also expands `Module` into its `ModuleDef`'s own stored tree. What remains — the editor — lands in the rest of T18 ([GAP_ANALYSIS_ETS.md](GAP_ANALYSIS_ETS.md) Tier 5), no fixed session yet. |
| Whether ETS re-imports an unsigned third-party `.knxproj` | Session 3 delivered the mechanism (`ExportWarning::Unsigned`, always present); per ADR-0015 (Session 7), ETS reimport is no longer a project goal, so the verification itself (risk R9) is deprioritized — no fixed session, and none needed |
| Whether Data Secure runtime keys are readable from `.knxproj` | Session 7 or later — `knx-secure` |
| `.knxprod` encryption for master data scheme ≥ 12 | Session 4 delivered `.knxproj`-sourced product database ingest; 2026-09-10's standalone package installer (`knx_productdb::install_package`) showed the "encryption" premise was wrong for schemes 11 and 20 specifically — those 5 real-world files parse with no encryption at all, direct `.knxprod` ingest now works for both (see [KNOWN_LIMITATIONS.md §11](KNOWN_LIMITATIONS.md#11-knxprod-files-for-master-data-scheme--12-cannot-be-imported-directly)). Schemes 12-19/21/22 remain untested (no standalone sample acquired yet), no fixed session; `.vd2` is a distinct legacy format, permanently out of scope, not an encryption question at all. |
| The project licence | Session 7 — currently a placeholder, see [KNOWN_LIMITATIONS.md](KNOWN_LIMITATIONS.md) |
