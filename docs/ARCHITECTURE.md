# Architecture

## Local import-integrity adapter repair (2026-10-10)

[ADR-0107](adr/0107-import-source-integrity.md) and [the contract](IMPORT_INTEGRITY.md) bind ZIP reads to validated member identity, retain original XML, separate metadata provenance and record parser-owned lexical observations. Application code reuses persisted product diagnostics and baggage classification. Completion vocabulary requires a scalar-only schema12 refusal barrier; no UI/core coupling, new dependency or protocol capability. Local and unpublished; final acceptance remains separately recorded.

## Docker upload and explicit browser return (2026-10-09)

[Separate integration verification](status/2026-10-10-import-expansion-integration.md) supersedes the local-only delivery status, not its historical test results. This is source integration, not a new release or deployment.

The [upload and return contract](contracts/DOCKER_UPLOAD_AND_RETURN.md)
keeps HTTP size admission and streamed temporary-file publication in the server
file adapter. Explicit browser return navigates the current document; native
focus and source-bound Flow selection keep their existing adapter behavior.
A consumed `view=editor` marker requests read-only project resume through the
existing authenticated `/api/project` projection, not open/import or a mutation.
No KNX model, parser budget, storage schema, protocol or dependency changes.

## Selective import shares one application planner (2026-10-10 local source)

[ADR-0106](adr/0106-selective-project-import.md) puts bounded source reading,
explicit installation selection, reference-closure remapping and conflict checks
in `knx-app::selective_import`. HTTP and CLI consume the same admitted plan; UI
only selects, previews and confirms. Native snapshot history provides one undo
step; durable admission precedes publication. CLI compare-and-save checks both
history generation and the reviewed working-snapshot hash under one write lock.
Complete retained sources are scoped by archive identity; selected source
attributes remain evidence, not destination defaults. No KNX Core/UI dependency,
new schema, manufacturer installation or protocol capability is introduced.
The [original local acceptance](status/2026-10-10-import-expansion-verification.md)
is historical; integration acceptance and release remain separate. Future model versions require a new
reference-closure audit rather than automatic acceptance.

## Communication-object table (2026-10-09 source)

[ADR-0102](adr/0102-communication-object-table-view-state.md) and the
[table contract](COMMUNICATION_OBJECT_TABLE.md) keep search/filter/sort in pure UI
derivation and every keyed object editor under one stable table parent. A per-object
mutation boundary guards duplicate starts and late publication; existing application
commands remain authoritative. View state is transient, not a domain/project schema.
No core/store/product/API/protocol/dependency change. Local delivery is not release
or deployment.

## Offline AP1 diagnostics stay outside the execution planner (2026-10-09)

[ADR-0098](adr/0098-offline-ap1-procedure-resolution.md) and the
[offline procedure contract](OFFLINE_PROCEDURE_RESOLUTION.md) place a bounded,
read-only retained-source resolver in the product infrastructure adapter.
The application contribution service selects package ownership and aggregates
coverage/findings; UI and CLI consume the same diagnostics. The KNX domain,
store schemas, production importer admission and download executor are unchanged.
Only one explicit unqualified `MV-07B0` `Load/ap1` template at the observed
`HawkConfigurationData/Procedures/Procedure` path is eligible for reconstruction.

This type is never an executable plan. Source hashes/offsets and unresolved
ordered declarations are local evidence; reduced public exports remove them
and retain value-free fixed issue-code counts. Existing preview, consent and
manual maintainer handoff own disclosure; there is no automatic transmission.
Reuse the existing workspace serde dependency, not a new protocol/UI framework.
Local implementation and publication remain separate.

## Guarded project-local names (2026-10-09)

[ADR-0101](adr/0101-project-local-device-and-ga-names.md) and
[the naming contract](contracts/project-name-editing.md) add name-only domain
commands, exact restoration, authoritative core admission and guarded dedicated
HTTP routes. One UI workflow serves editor/inspector/explorer/tables. The
application owns a transient successful-load generation alongside existing
server/revision identity; no import or native-schema field changes. Native
state history from ADR-0100 is reused, not a new serialized-command format.
No CSV/MCP/protocol/CLI-authoring change or ETS/hardware compatibility claim.


## Native project history (2026-10-09)

[ADR-0100](adr/0100-persistent-native-project-history.md) adds bounded native
working-state/undo/redo persistence and explicit project versions. Core remains
storage/UI independent: active inverse commands and restored normalized snapshot
swaps share the CommandStack, preserving allocator high water. `knx-store` owns
versioned SQLite images, retained-context sharing, strict admission and atomic
baseline/generation binding; `knx-app` coordinates durable save/edit operations;
HTTP/UI add revision-bound consent and lifetime-aware publication. The saved root
is the clean baseline, not necessarily the most recent working state. See
[the contract](PROJECT_HISTORY.md); protocol actions are never replayed.


## Device navigation and canonical catalogue reads (2026-10-08)

[ADR-0096](adr/0096-devices-navigation-and-catalogue-batch.md) and the
[device navigation contract](DEVICE_NAVIGATION.md) keep the new central
Devices list/editor in UI and lightweight, version-1 catalogue reads in the
server adapter. The existing canonical device projection is reused for every
device, including those absent from placement branches; product identities
are resolved once per reference, separately from application compatibility.
Server/revision binding and explicit unavailable states prevent stale or
failed product reads from becoming guessed metadata. Scoped UI links reuse
selection, not protocol actions; the existing monitor remains mounted.
No core/store/product schema, format, protocol or dependency changes.
Local implementation/publication and scoped acceptance are separate.

## Offline community demo authoring (2026-10-08)

[The demo contract](COMMUNITY_DEMO_PROJECTS.md) keeps original fictional
product declarations and building layouts in authoring tooling, not app code.
A Rust example in `knx-app` constructs the normalized typed domain model and
uses the existing native store; a separate synthetic `.knxprod` uses the
existing catalogue adapter. Product data remain independent (ADR-0005), and
no `.knxproj` exporter is reintroduced (ADR-0028). Core/API/UI, persistence
schemas, parser admission and dependencies are unchanged. Browser evidence
uses a real production app in a loopback-only namespace, not bus simulation.
Published on `main` on 2026-10-09.

## Read-only community evidence boundary (2026-10-08)

[ADR-0091](adr/0091-community-evidence-analysis.md) and
[the service/disclosure contract](COMMUNITY_EVIDENCE.md) reuse production
project import and product install/offline planning in disposable storage. A
separate value-free, namespace-expanded shape inventory is structural
observation only, not typed compatibility. API and CLI expose the same
versioned report, exact preview and deterministic consent-gated ZIP. The UI
chooses the disclosure and offers a manual GitHub/mail handoff; the service
has no AppState mutation, bus access or external submission. Native, project
and product schemas and parser admission stay unchanged.

## Static marketing companion (2026-10-08)

[ADR-0095](adr/0095-static-marketing-companion.md) keeps `website/` independent
of the engineering app, domain and server: curated DE/EN HTML, minimal JS, a
stdlib-only deterministic preview build, hash-inventoried local media and the
pinned existing story; loopback serving only. No tracking, no new app
dependency, no live demo and no automatic publication.
[Website contract](WEBSITE.md).

## Device parameter inspection tabs (2026-10-08)

[Parameter workspace](PARAMETER_WORKSPACE.md) keeps one UI-owned parameter read
model shared by Parameters, Diagnostics and Manufacturer fields. Pure
presentation grouping preserves all raw diagnostic occurrences/scope;
manufacturer Access Read/None fields move to inspection-only views without
changing backend authority, DTOs, storage or KNX semantics. No extra fetch on
tab switches. ADR-0080 presentation amendment documents the replaced fold.

## LCARS built-in presentation (2026-10-08)

[ADR-0092](adr/0092-lcars-built-in-presentation.md) separates the complete LCARS
palette from a trusted UI-only presentation marker. The existing Theme dropdown
and settings lifecycle select both without extending declarative v1 packs or
adding a persistent layout preference. Motion and density remain independently
owned. No core/API/project schema/protocol change. See [LCARS guide](DESIGN_LCARS.md).

## Readable Flow and source-bound secondary windows (2026-10-07)

[ADR-0085](adr/0085-readable-flow-and-source-windows.md) records the user-approved
presentation follow-up: measured/growing canvas, readability-first layout,
switchable auto-fit, a read-only window subscribed to the source monitor, and
exact main-editor navigation guarded by project scope/revision. No core/storage
schema or bus protocol changed; no second tunnel or persistent traffic history.
Historical acceptance below remains historical; native live verification has
not been extended by browser tests or capability compilation.

## Post-alpha Linux launcher boundary (2026-10-07)

[ADR-0021's launcher amendment](adr/0021-appimage-is-the-first-linux-package.md#2026-10-07-amendment-owned-display-policy-kl-158)
and [the contract/receipt](APPIMAGE_LAUNCHER.md) keep Wayland/X11 selection
in the AppImage packaging hook. A project-local, checksum-pinned deploy tool
embeds the owned policy before GTK initializes; explicit caller settings win.
No KNX core, application-service, storage/schema or HTTP dependency changes;
the published alpha.4 artifact is not replaced by the local source-built image.

## Approved Telegram-flow extension — session-local, not physical topology

[ADR-0077](adr/0077-session-local-telegram-flow-view.md) and
[the contract](TELEGRAM_FLOW_VISUALIZATION.md) approve the new Alpha capability,
not an implementation. Reuse existing capture/decoding, add the minimal read-only
application/service participant/type/age contract in AR20, and keep disposable
layout/pulses and the value-display reducer in UI U19–U21. No KNX core UI
dependency, project graph entity, position field, project/product migration or
persistent traffic store. Bind inferred participants to the exact session flow
context; flags/links/device edits invalidate that evidence even if DPT decoding
is unchanged. Values update on admitted observations, never animation arrival;
project targets do not establish real device state. AR21 owns integrated Alpha
acceptance, and no commissioning or release authorization follows.
**Status 2026-10-05:** implemented (U20/U21, AR20) and accepted for the Alpha by
AR21 ([TELEGRAM_FLOW_VISUALIZATION §22](TELEGRAM_FLOW_VISUALIZATION.md#22-ar21-rerun-of-findings-6-and-7-and-acceptance-alpha-2026-10-05)).


The binding architecture for this repository. Decisions recorded here are
argued in [docs/adr/](adr/); the evidence they rest on is in
[RESEARCH.md](RESEARCH.md), cited by section throughout.

## 1. Purpose and scope

A Linux-first, KNX-compatible engineering application, built as an independent
alternative to ETS rather than a reimplementation of it.

**v1 target — a project editor with top-level device parameter editing
(module-scoped, per-channel editing still out).** Import
`.knxproj`; inspect and edit group addresses, links, building structure,
topology, device names and individual addresses; save to the native
`.knxdb` format; and monitor the live bus against the open project.
`.knxproj` is read-only: KNXBench imports it and never writes it
([ADR-0028](adr/0028-no-knxproj-export.md), 2026-09-20, superseding
ADR-0015's decision to keep the exporter). Once imported, a project lives
in `.knxdb` and nothing carries it back to ETS.

Four things are explicitly out of v1:

| Excluded | Reason |
| --- | --- |
| Device parameter editing | RESEARCH R3 — the Session 4 spike (RESEARCH §4.3) found the `@test` value grammar is Standard-normative, but the `choose`/`when`/`Channel`/`ParameterBlock` structural grammar is still corpus-observed only. T18 slice 1 (2026-09-11) built a headless evaluator over the stored `Dynamic` tree in `knx-productdb`; T18 slice 2 (same day) made it expand a `Module` node into its `ModuleDef`'s own tree too. **T18 slice 3 (2026-09-11)** adds a read/write API (`GET`/`POST /api/device/{id}/parameters`) and an `apps/knx-web` panel, scoped to top-level fields — module-scoped (per-channel) fields are read/displayed but not editable (decisions D20-D26, [design](https://github.com/KNXBench-Labs/KNXBench/blob/bca2d3336b96/docs/superpowers/specs/2026-09-11-parameter-editor-design.md)) |
| Commissioning and device download | RESEARCH §8.3/§8.4 — the generic load/unload/reset/memory-write procedures are now documented (R5 spike, §8.4), but a product-specific `Legacy*` compatibility-flag matrix, vendor DLLs and bricking risk on real hardware remain. **Not a permanent exclusion**: the user ruled 2026-09-11 that this is required ([KNOWN_LIMITATIONS.md §7](KNOWN_LIMITATIONS.md#7-commissioning-and-device-download-are-required-but-blocked)) |
| KNX Secure | RESEARCH §9 — no sample material to verify against; the subsystem exists but stays empty |
| Writing `.knxproj` | User ruling 2026-09-20, [ADR-0028](adr/0028-no-knxproj-export.md) — import is one-way; the exporter, its CLI subcommand, its HTTP route and its UI control were deleted rather than frozen |

User-facing wording is **"KNX-compatible"**. Never "KNX certified", never "full
ETS compatibility" (RESEARCH §7, §10).

## 2. Layering

```text
UI
 ↓
Application / Services
 ↓
KNX Domain Core
 ↓
Infrastructure  (project storage, import/export, product database, KNXnet/IP)
```

Dependencies point downward only. The KNX core does not depend on the user
interface, and it does not depend on any import or export format: a format
change must never propagate into the domain model.

A headless CLI exists alongside the desktop application, and it is first-class,
not a by-product. It is what keeps the core honest about UI independence, and
it makes import and regression tests runnable in CI without a display.

## 3. Workspace layout

One Cargo workspace:

```text
apps/
  knx-cli/         Headless entry point (bin name: knx)
  knx-mcp/         Read-only MCP server over saved project files, stdio
                   only (ADR-0090). Links no bus, key or HTTP crate.
  knx-server/      axum HTTP API + static frontend serving — the web/Docker
                   deployment target. Same _impl functions and AppState
                   knx-desktop used to own directly, now the only crate
                   that speaks HTTP.
  knx-desktop/     Tauri v2 shell. src-tauri/ is a thin native wrapper that
                   spawns knx-server locally and points a WebView at it;
                   narrowly scoped native save-dialog commands write
                   session-log and bus-monitor JSON snapshots chosen by the
                   user (ADR-0047's local-file boundary). No KNX domain or
                   project mutation lives in the shell.
  knx-web/         React + Vite frontend (npm package, not a Cargo
                   workspace member) — served by knx-server's static-file
                   fallback and, in dev, by knx-desktop's Tauri WebView.
                   Project and bus operations use fetch(); native invokes
                   only deliver the two local JSON snapshots.

crates/
  knx-core/        Domain model, addresses, DPT, override resolution, validation.
                   No IO, no XML, no SQL, no UI.
  knx-app/         Application services: open/save, commands, undo/redo,
                   search, selection, reports; shared payload-free commissioning
                   activity lifecycle (ADR-0075 candidate), with no client/transport dependency
  knx-store/       SQLite project storage, schema migrations, opaque store;
                   separate versioned activity metadata store (ADR-0064),
                   never a project/vendor/recovery database
  knx-etsproj/     .knxproj read/write: ZIP, schema detection, tolerant XML
                   parser, mapping to/from knx-core, import report
  knx-productdb/   Product database (own SQLite, own migration chain),
                   streaming ingest of manufacturer XML keyed by content
                   hash, and enrichment of ComObjectInstance from it
                   (Session 4)
  knx-projection/  Pure Project -> ProjectTree projection, ts-rs TypeScript
                   bindings for knx-web. No IO; depends on knx-core only.
  knx-net/         KNXnet/IP: discovery, tunnelling, routing, cEMI, telegrams
  knx-secure/      Isolated key material subsystem (holds the .knxproj
                   ZIP-password derivation as of A6; no KNX Secure
                   runtime-key handling yet)
  knx-diff/        Pure `Project`-to-`Project` comparison ("KNXBench project
                   diff", never an ETS-comparison claim): `diff_projects`
                   matches entities by `ets_id`/natural key and reports
                   added/removed/changed per entity type. Depends on
                   knx-core only (T14).
  knx-csv/         Reader/writer for "KNXBench group-address CSV v1", a
                   format this project defines and owns (T12)
  knx-report/      Renders a project into one self-contained HTML document —
                   a KNXBench report, never an ETS-compatible one (T13)
  knx-build-stamp/ Build-script helper for both binaries: decides the commit
                   `--version` names and, with KNX_REQUIRE_CLEAN_TREE=1,
                   refuses a release build from a modified tree (ADR-0018
                   amendment). Zero dependencies; a build-dependency only.
  knx-testsupport/ Test-fixture paths and nothing else. Zero dependencies,
                   used only as a `[dev-dependencies]` entry, so it appears
                   in neither graph below: it exists so that no crate has to
                   hard-code the maintainer's corpus export filenames, and
                   each path it hands out is overridable by an environment
                   variable for anyone whose corpus lives elsewhere.

xtask/             Repository verification tasks, including the layering gate
```

```text
knx-desktop ─> knx-server ─┬─> knx-app ─> knx-core
                           ├─> knx-store ────> knx-core
                           ├─> knx-etsproj ──┬─> knx-core
                           │                 └─> knx-secure
                           ├─> knx-projection ─> knx-core
                           ├─> knx-diff ──────> knx-core
                           └─> knx-net ──────> knx-core

knx-mcp ─┬─> knx-store ─────> knx-core     (read-only openers, ADR-0090)
         ├─> knx-productdb ─> knx-core     (device_evaluation, shared with knx-server)
         ├─> knx-projection ─> knx-core
         ├─> knx-diff ──────> knx-core
         └─> knx-csv ───────> knx-core

knx-cli ────────────────────> knx-app ─> knx-core
                                 ├─> knx-store ────> knx-core
                                 ├─> knx-etsproj ──┬─> knx-core
                                 │                 └─> knx-secure
                                 ├─> knx-productdb ─> knx-core
                                 ├─> knx-net ──────> knx-core
                                 ├─> knx-diff ──────> knx-core
                                 └─> knx-secure     (also knx-app's own edge:
                                                     legacy EX-IM files, ADR-0094)
```

(`knx-web` has no place in this graph — it is an npm package, not a Cargo
crate; it reaches `knx-server` over HTTP, not `cargo`'s dependency
resolution.)

`apps/knx-desktop` — the Tauri shell and the React UI — was scaffolded in
Session 5. `apps/knx-server` and `apps/knx-web` (a web/Docker deployment
target, cross-cutting alongside Session 5, see
[ROADMAP.md](ROADMAP.md)) later took over the API surface and the UI
respectively: `knx-desktop/src-tauri` shrank to a thin wrapper spawning
`knx-server` locally, and `knx-web` is the same React application that
used to live at `knx-desktop/src`, now talking to `knx-server` over HTTP
instead of Tauri IPC. Pulling in Tauri and a Node toolchain before there
was a UI to build would have added a large dependency surface with
nothing to run against — the same reasoning that held Tauri out of
Session 1.

`knx-etsproj` and `knx-store` are separate crates because the import format and
the storage format evolve independently. An ETS6 schema delta must not touch
the project file schema, and a model migration must not break the importer.

`knx-secure` is the one crate with no dependency on `knx-core` at all — see
section 9.

`knx-core`/`knx-app`/`knx-store`/`knx-etsproj`/`knx-productdb`/
`knx-projection` keep zero dependency on both Tauri and any HTTP
framework — `knx-server` is the only crate that speaks HTTP, the same
architectural role Tauri's command layer had before it.

`knx-server` also depends directly on `knx-net` since T15 (Group Monitor
GUI) — `knx-server`'s own bus-session/monitor routes need a tunnel, not
just the project-file reading `knx-app` already provides. See
[ADR-0017](adr/0017-knx-server-depends-on-knx-net.md) for why this edge
was added instead of, say, routing bus traffic back through `knx-cli`.

## Theme extension boundary (U14 contract, U15 runtime delivered)

[ADR-0060](adr/0060-versioned-declarative-theme-packs.md) and
[THEME_PACKS](THEME_PACKS.md) define complete, versioned declarative theme packs.
Their parser/contrast validation stays pure in the Web layer; bounded DOM
application extends the existing theme resolver. U15 delivers that admission,
cache revalidation and reversible property ownership; visual fallback never
overwrites an unsupported preference. U16 delivers acknowledged
conditional updates through that same settings client/route: its last server
observation is not a second store, and the optional compare-and-patch capability
is independent of the unchanged opaque settings-file schema. File transport and
inert recovery remain Web utilities; candidate and actual-merged acceptance
passed, source 1f94808d is published with exact remote readback. No theme logic reaches KNX
Core, project storage, product data or KNXnet/IP. U17's verified manager
keeps ephemeral visual intent in App and one root DOM writer; read-only consumers
cannot acquire that lease. Appearance gestures reuse conditional U16 plans,
including the original selector. Intake generations, acknowledged theme/map/
accent fingerprints and explicit content-bound questions prevent stale preview
or consent reuse. Shared Overlay restores a persistent focus target when the
discarded draft's trigger disappears. U17 actual-merged23-command acceptance
passed on f16f1e40 with all697 source/configuration inputs unchanged; final
publication/readback is tracked in the handover. U18 extension-wide acceptance
remains open, including representative-component cross-palette state coverage.

## 4. Enforced rules

These are tests. Each one fails the build.

1. **`knx-core` must not reach `serde_json`, `quick-xml`, `rusqlite` or
   `tokio`** in its dependency graph, **`knx-etsproj` must not reach
   `knx-store`**, and **`knx-productdb` must not reach `knx-etsproj` or
   `knx-store`** — the third rule (Session 4) is what keeps a later
   `.knxprod` ingest from having to travel through the `.knxproj` importer,
   and keeps product data separable from project files (ADR-0005,
   ADR-0011). Enforced by `cargo run -p xtask -- check-layering`, which
   walks the resolved graph from `cargo metadata` and prints the shortest
   path to any forbidden package. All three gates have been observed to
   fail on an injected violation, which is the only way to know a gate
   works.
2. **No runtime crate may depend on a GPL-licensed crate** (RESEARCH R6).
   Enforced by `cargo deny check` against an explicit licence allowlist; any
   licence not on the list is rejected, and GPL is not on the list.
   `xknxproject` stays in `.venv`, invoked only by test scripts, never by the
   Rust build (ADR-0002).
3. **Project and bus operations communicate through HTTP into `knx-server`**;
   the UI has no path of its own to `knx-app`, `knx-store` or `knx-etsproj`.
   The desktop-only exceptions are OS save-dialog commands for local
   session-log and bus-monitor JSON snapshots (the bounded-file boundary
   established in ADR-0047). They write dialog-selected files and cannot
   mutate project or KNX state. Originally this rule was stated as "only through Tauri commands into `knx-app`" when
   `apps/knx-desktop` was the only deployment target (Session 5); the
   web/Docker deployment target moved domain requests to the HTTP API.
   This rule is **still not mechanically enforced**: `check-layering`'s
   four roots (rule 1, above) do not include a UI-boundary check, since
   `apps/knx-web` is an npm package outside the Cargo dependency graph
   `cargo metadata` walks.

All gates run in CI on every push and pull request, and all are runnable
locally with the same command. A check that only exists on CI gets
ignored.

## 5. Core approach

The core is a **normalized domain model carrying provenance per value**, with
one borrowing from a source-faithful design: an opaque store keyed by source
path retains everything not modelled, verbatim, including unknown XML
constructs (ADR-0006).

Provenance is not optional decoration. A communication object's effective
properties resolve through three layers — `ComObject`, `ComObjectRef`,
`ComObjectInstanceRef` — and 758 of 907 instances in the reference project
override the datapoint type at instance level (RESEARCH §3.2). Without knowing
which layer a value came from, nothing can tell the user's own decision apart
from what a product database supplied or what this application guessed.
The type is `Resolved<T> { value, layer }`, and it is in `knx-core` from the
first commit (ADR-0004).

Two alternatives were considered and rejected:

- **Source-faithful document plus computed projection.** The ETS schema would
  become the domain model, which `CLAUDE.md` forbids and which would make a
  second schema generation impossible to attach cleanly.
- **Event-sourced core.** Every query would need materialization, and
  migrations would have to keep replaying historical commands — the most
  expensive form of schema versioning there is. Undo/redo is achieved with a
  command pattern instead, at a fraction of the cost.

## 6. Application layer

Every mutation is a `Command` with `apply(&mut Project) -> Result<Inverse>`.
Undo and redo are a stack of inverses. The UI never holds a mutable reference
to the model.

Today this rule is held by review, not by the type system: `Project`'s fields
are `pub`, and nine live server paths still advance the id allocator or enrich
the project outside a command ([KNOWN_LIMITATIONS §129](KNOWN_LIMITATIONS.md#129-a-stale-id-allocator-snapshot-can-duplicate-ids-and-saving-then-drops-one-entity)).
[ADR-0039](adr/0039-project-mutation-goes-through-commands.md) (Accepted)
records how to enforce it. Its phase 1 backstop is in place: every
id-inserting command refuses an id already in use anywhere in the project
(`CommandError::IdInUse`), so an applied command cannot create a duplicate id
for `save_project` to collapse. Since phase 2 the CSV planner and scan
reconciliation emit the never-rewinding `ReserveIds`. Every applied CSV plan
is bound to its planned revision, so a stale plan is refused and has to be
re-imported. The nine phase-3 sites still allocate ids outside a command, and
a collision with them is caught only by the `IdInUse` backstop.

Catalog multi-creation submits `ReserveIds` and each `CreateDevice` as one
atomic `Batch`; a refused child rolls back both project data and the batch's
ID reservation. `CommandError::BatchItem` retains the child index and typed
cause so a caller can name which requested device failed. A quantity-one
catalog create still uses the original single `CreateDevice` command.

Commands are where validation lives — a duplicate individual address, a group
address outside its `GroupRange`, a link to a deleted object. Not in the UI,
and not in the store.

Any command that changes a `Resolved<T>` sets its layer to `UserEdit`, so a
user's own edit is distinguishable from an imported or inferred value with no
separate bookkeeping to keep in sync.

`knx-store` writes inside a SQLite transaction, incrementally at entity
granularity. A crash leaves either the old state or the new one, never a
half-written project. This is the reason the working file is SQLite rather than
a directory tree (ADR-0003).

## 7. UI boundary

`knx-server`'s HTTP routes form a narrow, explicitly typed API — one route per
former Tauri command, same JSON shapes, same `_impl` functions underneath
(the web/Docker deployment target replaced the transport, not the
contract). The UI requests projections — `ProjectTree`, `DeviceList`,
`GroupAddressTable`, `Inspector<T>` — and sends `Command` values back.

Domain types are not mirrored one-to-one into TypeScript. Projections are
shaped for display and generated from Rust with `ts-rs`, so the two sides
cannot drift apart silently. Large tables are paginated and filtered on the
Rust side rather than shipped whole into the browser: the reference project
alone has 514 group addresses and 907 communication object instances (RESEARCH
§4.1).

UI workarounds for domain problems are not acceptable. The fix belongs in the
layer that owns the problem. See ADR-0009.

T13 snapshot ordering and replacement coherence follow
[ADR-0032](adr/0032-application-snapshot-ordering.md). The application owns
transient ordering metadata, stamped while the project lock is held; the UI
rejects superseded snapshots instead of inferring server order from response
arrival. Save paths, clean baselines, opaque passthrough and manufacturer
manifests share the project-led publication boundary. None of this metadata
enters the KNX domain or native project schema, and it is not a multi-user
conflict-resolution protocol.

## 8. KNXnet/IP

An own implementation against ISO 22510, not a port of an existing stack.

Session 6, Cycle 1 delivered read-only tunnelling: `crates/knx-net` connects
to a KNXnet/IP gateway by known IP, receives KNX telegrams, and decodes
them, exposed as a `BusConnection` trait (`discover`, `connect_tunnel`) and
a `TunnelClient` handle (`send`, `subscribe`) — the same names this
document already fixed, now backed by a real implementation grounded in
the KNX Association specification rather than a port of an existing
stack. `TunnelClient::send` (Cycle 2), `discover` (Cycle 3, multicast
`SEARCH_REQUEST`/`SEARCH_RESPONSE`), and `connect_routing` (Cycle 4,
unconfirmed `ROUTING_INDICATION` over the standard routing multicast
group) are all implemented now; KNX IP Secure remains out of scope, handled
separately by `knx-secure`. The bus monitor is a consumer that
resolves telegrams against the open project (`apps/knx-cli`'s `bus monitor`
subcommand, resolving group address names and — since T29, 2026-09-11,
`crates/knx-core/src/dpt/` — decoding each telegram's value against the
DPT inferred from the project's linked communication objects, given
`--project`; [KNOWN_LIMITATIONS.md §61](KNOWN_LIMITATIONS.md#61-the-dpt-codec-covers-thirty-main-types-infers-rather-than-reads-its-input-and-leaves-several-encoding-questions-to-a-stated-ruling-rather-than-the-standard) has the full
accounting of what that codec does and does not cover); the connection
itself knows nothing about projects, as this section originally specified.
Since T15 (2026-09-11), `apps/knx-server` runs the same kind of session
server-side, behind `/api/bus/*`, for `apps/knx-web`/`apps/knx-desktop` —
see [ADR-0017](adr/0017-knx-server-depends-on-knx-net.md) for why
`knx-server` now depends on `knx-net` directly rather than shelling out to
`knx-cli`, and [KNOWN_LIMITATIONS.md §62](KNOWN_LIMITATIONS.md#62-the-group-monitor-gui-t15-is-tunnelling-only-single-session-client-filtered-and-only-its-passive-receive-path-has-real-gateway-evidence)
for what that GUI does and does not cover.

`BusAccess` from `0.xml` — the ETS commissioning interface connection string —
is preserved verbatim and **not** translated into our own connection model
(RESEARCH §3.1). It is ETS tool configuration, not domain data, and rewriting
it would be inventing meaning we have not verified.

Commissioning and download are not implemented (RESEARCH §8.3) and not
excluded either — the user ruled 2026-09-11 that this capability is
required, blocked until the KNX specification database is finished
([KNOWN_LIMITATIONS.md §7](KNOWN_LIMITATIONS.md#7-commissioning-and-device-download-are-required-but-blocked)).
The architecture does not block that path today: load procedures, memory
layout and mask data all live in the product database.
**Status 2026-10-06 (AR15):** overtaken. Device download is implemented for the
verified `070nh` memory path (ADR-0048/0049): `knx device download` and the
Web download tab, run with read-back on one device. Address programming and
reset fail closed before a tunnel until durable recovery exists (ADR-0057,
ADR-0058); the property-based downloader is simulator-only
([KNOWN_LIMITATIONS §7](KNOWN_LIMITATIONS.md#7-commissioning-and-device-download-are-required-but-blocked)).

## 9. Key material

`knx-secure` exists from the first commit of the workspace, with its own
storage. It was empty until A6 (2026-09-13), which gave it the `.knxproj`
ZIP-password derivation, and T15 (2026-09-14) added `zipcrypto` — a
hand-written implementation of the ZipCrypto stream cipher (APPNOTE.TXT
§6.1), decrypt-only, with no encryption function anywhere in the
workspace. It still holds no KNX Secure runtime-key handling (Data
Secure, IP Secure, keyring) — that part of the crate's purpose remains
unimplemented (`KNOWN_LIMITATIONS.md §8`).

The rules are in force from now on. Key material never enters the `Project`
model, never enters an `ImportReport`, never enters any file this application
writes, never enters a log, and is omitted by default from diagnostic dumps (RESEARCH §9). Both
halves of that are enforced mechanically rather than by convention: `cargo
run -p xtask -- check-layering` fails the build if `knx-secure` ever gains a
dependency path to `knx-core` (so no type path can carry a key into the
project model) or to `serde` (so no `knx-secure` type can gain a
`Serialize`/`Deserialize` impl); and `ZipPassword`, the one key-material type
`knx-secure` currently exports, ships a hand-written `Debug` impl that always
prints a fixed placeholder instead of the value, with no `Display` impl and
no `serde` derive.

One caller now crosses that boundary in the other direction:
`knx-etsproj`'s `Container::open_with_password` takes a plaintext password
as a `&str` and hands its bytes to `knx_secure::zipcrypto::decrypt`. The
password is a borrowed parameter and nothing more — it is not stored on
`Container`, not copied into any `EntryInfo`, and not interpolated into
any error variant or `Display` impl, so no container error can leak it
into a log or a report. `zipcrypto::decrypt` takes `&[u8]` rather than
`ZipPassword` deliberately: `ZipPassword` is the *derived* AES key for
schema ≥ 21, a different thing from the raw ZipCrypto password, and
conflating the two would be exactly the kind of convenience this section
exists to prevent.

A second caller since 2026-10-08 (ADR-0094): `knx_app::legacy` decrypts
legacy ETS3 EX-IM product files (`.vd3`–`.vd5`) with a password the user
types, through the same `zipcrypto::decrypt`. The split is deliberate.
`knx-productdb` reads the legacy container and grammar but never decrypts:
it hands out the raw stream and its check bytes
(`LegacyMember::encrypted_stream`) and finishes from the decrypted bytes
(`LegacyMember::open_decrypted`). `check-layering` forbids
`knx-productdb → knx-secure`, dev edges included. Without that rule
`knx-mcp`, which links `knx-productdb`, would link key material, and
ADR-0090 rules that out. The password lives only in `knx_app::legacy::LegacyPassword`.
It has a redacting `Debug`, no `Display`/`Clone`/serde, and is never stored.

Retrofitting isolation is how secrets leak, which is why the boundary exists
before the feature does (ADR-0008).

## 10. Test strategy

Seven levels. Two of them exist today; the rest arrive with the code they test.

| Level | Content | Status |
| --- | --- | --- |
| Unit | Addresses, DPT parsing, override resolution, validation rules | Started — `Layer::is_exported`, which since ADR-0028 means "this value is the project's own, not inferred", and gates what the CSV and documentation exports write |
| Golden | Import of the reference project against the entity counts from RESEARCH §3: 36 devices including the unassigned one, 514 group addresses, 907 `ComObjectInstanceRef`, 1390 parameter values, 569 send and 27 receive links. Session 4 adds its own golden ingest of the same project's manufacturer data (4 manufacturers, 24 source files, 12 application programs, 5,630 `com_object_ref` rows, 48,190 translations since T32, 2026-09-12 — 48,057 program-scope plus 109 catalog and 24 hardware — `crates/knx-productdb/tests/golden_reference_products.rs`) | Session 3, extended Session 4 |
| Oracle | Comparison against `xknxproject` output where it is not known to be lossy; every deviation must be explained. Session 4 adds a communication-object text/DPT comparison against `project_dump.json`, read as a committed output file per ADR-0002, never a dependency | Session 3, extended Session 4 |
| Roundtrip | Retired 2026-09-20 with the `.knxproj` writer ([ADR-0028](adr/0028-no-knxproj-export.md)) — there is no second half of a trip to compare against. What replaces it is the import-fidelity statement in [IMPORT_EXPORT.md](IMPORT_EXPORT.md) §9, tested by the import suites, and the CSV export/re-plan round trip in `crates/knx-app/tests/csv_roundtrip.rs`, which is a KNXBench format and not an ETS one | Session 3, retired Session 7 |
| Migration | Every schema version has a frozen fixture that must keep loading — `knx-store` through v3, `knx-productdb`'s own v1 through v3 (v2→v3 additionally backfills `dynamic_node` rows into existing databases from their stored blobs, T18 slice 1) | Session 2, extended Session 4 and T18 |
| Malformed input | Broken ZIP, truncated XML, unknown schema, duplicate IDs, invalid addresses, dangling references, password-protected without a password. Session 4 adds `crates/knx-productdb/tests/malformed_input.rs`: a truncated program, an empty file, 10,000 levels of nesting, an id collision across two different content hashes | Session 3, extended Session 4 |
| Licence and layering | The dependency graph reaches no GPL crate; `knx-core` stays IO-free | Done — `cargo deny check`, `cargo run -p xtask -- check-layering` |

The golden numbers are reproducible independently via
[tools/inspect_knxproj.py](../tools/inspect_knxproj.py), which reads the raw XML
without going through our importer or through `xknxproject`. That is what makes
them an oracle rather than an expectation we generated from our own output and
then asserted against itself.

## 11. Decision index

| ADR | Title |
| --- | --- |
| [0001](adr/0001-technology-stack.md) | Technology stack — Rust core, Tauri, React, SQLite |
| [0002](adr/0002-own-knxproj-parser.md) | Own `.knxproj` parser; `xknxproject` as a test oracle only |
| [0003](adr/0003-sqlite-project-format.md) | SQLite as the native project format |
| [0004](adr/0004-provenance-model.md) | Provenance and override-chain model |
| [0005](adr/0005-separate-product-database.md) | Separate, shared product database |
| [0006](adr/0006-opaque-passthrough-store.md) | Opaque passthrough store |
| [0007](adr/0007-roundtrip-fidelity.md) | Roundtrip fidelity definition |
| [0008](adr/0008-key-material-isolation.md) | Key material isolation |
| [0009](adr/0009-ui-boundary.md) | UI boundary via generated projections |
| [0010](adr/0010-per-attribute-override-representation.md) | Overrides are represented per attribute with an explicit empty state |
| [0011](adr/0011-product-database-storage.md) | Product database storage — blobs and parsed tables, content hash as identity |
| [0012](adr/0012-enrichment-into-absent-slots.md) | Enrichment fills only `Override::Absent` slots |
