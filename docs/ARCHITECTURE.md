# Architecture

The binding architecture for this repository. Decisions recorded here are
argued in [docs/adr/](adr/); the evidence they rest on is in
[RESEARCH.md](RESEARCH.md), cited by section throughout.

## 1. Purpose and scope

A Linux-first, KNX-compatible engineering application, built as an independent
alternative to ETS rather than a reimplementation of it.

**v1 target — a project editor without device parameter configuration.** Import
`.knxproj`; inspect and edit group addresses, links, building structure,
topology, device names and individual addresses; save to the native
`.knxdb` format; and monitor the live bus against the open project.
`.knxproj` export exists as an interop convenience, but per
[ADR-0015](adr/0015-native-output-drops-ets-reimport-goal.md) it is no
longer a goal for the exported file to be re-importable by ETS.

Four things are explicitly out of v1:

| Excluded | Reason |
| --- | --- |
| Device parameter editing | RESEARCH R3 — the Session 4 spike (RESEARCH §4.3) found the `@test` value grammar is Standard-normative, but the `choose`/`when`/`Channel`/`ParameterBlock` structural grammar is still corpus-observed only. T18 slice 1 (2026-09-11) built a headless evaluator over the stored `Dynamic` tree in `knx-productdb`; T18 slice 2 (same day) made it expand a `Module` node into its `ModuleDef`'s own tree too. **T18 slice 3 (2026-09-11)** adds a read/write API (`GET`/`POST /api/device/{id}/parameters`) and an `apps/knx-web` panel, scoped to top-level fields — module-scoped (per-channel) fields are read/displayed but not editable (decisions D20-D26, [design](superpowers/specs/2026-09-11-parameter-editor-design.md)) |
| Commissioning and device download | RESEARCH §8.3/§8.4 — the generic load/unload/reset/memory-write procedures are now documented (R5 spike, §8.4), but a product-specific `Legacy*` compatibility-flag matrix, vendor DLLs and bricking risk on real hardware remain. **Not a permanent exclusion**: the user ruled 2026-09-11 that this is required ([KNOWN_LIMITATIONS.md §7](KNOWN_LIMITATIONS.md#7-commissioning-and-device-download-are-required-but-blocked)) |
| KNX Secure | RESEARCH §9 — no sample material to verify against; the subsystem exists but stays empty |
| Direct `.knxprod` import for master data scheme ≥ 12 | RESEARCH §10 — the encryption layer is unresolved |

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
it makes import, roundtrip and regression tests runnable in CI without a
display.

## 3. Workspace layout

One Cargo workspace:

```text
apps/
  knx-cli/         Headless entry point (bin name: knx)
  knx-server/      axum HTTP API + static frontend serving — the web/Docker
                   deployment target. Same _impl functions and AppState
                   knx-desktop used to own directly, now the only crate
                   that speaks HTTP.
  knx-desktop/     Tauri v2 shell. src-tauri/ is a thin native wrapper that
                   spawns knx-server locally and points a WebView at it —
                   no #[tauri::command] handlers of its own since the
                   web/Docker deployment target. src-tauri/'s only
                   workspace dependency is knx-server.
  knx-web/         React + Vite frontend (npm package, not a Cargo
                   workspace member) — served by knx-server's static-file
                   fallback and, in dev, by knx-desktop's Tauri WebView.
                   Talks to knx-server over fetch(); no Tauri invoke().

crates/
  knx-core/        Domain model, addresses, DPT, override resolution, validation.
                   No IO, no XML, no SQL, no UI.
  knx-app/         Application services: open/save, commands, undo/redo,
                   search, selection, reports
  knx-store/       SQLite project storage, schema migrations, opaque store
  knx-etsproj/     .knxproj read/write: ZIP, schema detection, tolerant XML
                   parser, mapping to/from knx-core, import report
  knx-productdb/   Product database (own SQLite, own migration chain),
                   streaming ingest of manufacturer XML keyed by content
                   hash, and enrichment of ComObjectInstance from it
                   (Session 4)
  knx-projection/  Pure Project -> ProjectTree projection, ts-rs TypeScript
                   bindings for knx-web. No IO; depends on knx-core only.
  knx-net/         KNXnet/IP: discovery, tunnelling, routing, cEMI, telegrams
  knx-secure/      Isolated key material subsystem (empty for now, but present)
  knx-diff/        Pure `Project`-to-`Project` comparison ("KNXBench project
                   diff", never an ETS-comparison claim): `diff_projects`
                   matches entities by `ets_id`/natural key and reports
                   added/removed/changed per entity type. Depends on
                   knx-core only (T14).

xtask/             Repository verification tasks, including the layering gate
```

Two other pure crates, `knx-csv` (T12) and `knx-report` (T13), are missing
from the `crates/` listing above and from both dependency graphs below —
a pre-existing omission from those tasks, not something this edit
retroactively fixes. Flagged here rather than silently adding a third
undocumented crate to the pile.

```text
knx-desktop ─> knx-server ─┬─> knx-app ─> knx-core
                           ├─> knx-store ────> knx-core
                           ├─> knx-etsproj ──> knx-core
                           ├─> knx-projection ─> knx-core
                           ├─> knx-diff ──────> knx-core
                           └─> knx-net ──────> knx-core

knx-cli ────────────────────> knx-app ─> knx-core
                                 ├─> knx-store ────> knx-core
                                 ├─> knx-etsproj ──> knx-core
                                 ├─> knx-productdb ─> knx-core
                                 ├─> knx-net ──────> knx-core
                                 ├─> knx-diff ──────> knx-core
                                 └─> knx-secure
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
3. **The UI communicates only through HTTP, into `knx-server`**, and has no
   path of its own to `knx-app`, `knx-store` or `knx-etsproj`. Originally
   stated as "only through Tauri commands into `knx-app`" when
   `apps/knx-desktop` was the only deployment target (Session 5); the
   web/Docker deployment target replaced Tauri IPC with an HTTP API, so
   the boundary moved from `knx-desktop/src-tauri` to `knx-server`, but the
   shape of the rule — one crate mediates between the UI and everything
   below it — is unchanged. This rule is **still not mechanically
   enforced**: `check-layering`'s four roots (rule 1, above) do not
   include a UI-boundary check, since `apps/knx-web` is an npm package
   outside the Cargo dependency graph `cargo metadata` walks.

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
which layer a value came from, an exporter cannot decide what to write back.
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

Commands are where validation lives — a duplicate individual address, a group
address outside its `GroupRange`, a link to a deleted object. Not in the UI,
and not in the store.

Any command that changes a `Resolved<T>` sets its layer to `UserEdit`, so the
exporter knows what to write with no separate bookkeeping to keep in sync.

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
`--project`; [KNOWN_LIMITATIONS.md §61](KNOWN_LIMITATIONS.md#61-the-dpt-codec-covers-fourteen-main-types-infers-rather-than-reads-its-input-and-leaves-several-encoding-questions-to-a-stated-ruling-rather-than-the-standard) has the full
accounting of what that codec does and does not cover); the connection
itself knows nothing about projects, as this section originally specified.
Since T15 (2026-09-11), `apps/knx-server` runs the same kind of session
server-side, behind `/api/bus/*`, for `apps/knx-web`/`apps/knx-desktop` —
see [ADR-0017](adr/0017-knx-server-depends-on-knx-net.md) for why
`knx-server` now depends on `knx-net` directly rather than shelling out to
`knx-cli`, and [KNOWN_LIMITATIONS.md §62](KNOWN_LIMITATIONS.md#62-the-group-monitor-gui-t15-is-tunnelling-only-single-session-client-filtered-and-has-never-talked-to-a-real-gateway)
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

## 9. Key material

`knx-secure` exists from the first commit of the workspace, with its own
storage, while still empty.

The rules are in force from now on. Key material never enters the `Project`
model, never enters an `ImportReport`, never enters an export, never enters a
log, and is omitted by default from diagnostic dumps (RESEARCH §9). The crate
does not depend on `knx-core`, so there is no type path along which a key can
reach the project model. A test asserts that `knx-secure` types do not
implement `Serialize` toward report or export paths.

Retrofitting isolation is how secrets leak, which is why the boundary exists
before the feature does (ADR-0008).

## 10. Test strategy

Seven levels. Two of them exist today; the rest arrive with the code they test.

| Level | Content | Status |
| --- | --- | --- |
| Unit | Addresses, DPT parsing, override resolution, validation rules | Started — `Layer::is_exported` |
| Golden | Import of the reference project against the entity counts from RESEARCH §3: 36 devices including the unassigned one, 514 group addresses, 907 `ComObjectInstanceRef`, 1390 parameter values, 569 send and 27 receive links. Session 4 adds its own golden ingest of the same project's manufacturer data (4 manufacturers, 24 source files, 12 application programs, 5,630 `com_object_ref` rows, 48,057 translations — `crates/knx-productdb/tests/golden_reference_products.rs`) | Session 3, extended Session 4 |
| Oracle | Comparison against `xknxproject` output where it is not known to be lossy; every deviation must be explained. Session 4 adds a communication-object text/DPT comparison against `project_dump.json`, read as a committed output file per ADR-0002, never a dependency | Session 3, extended Session 4 |
| Roundtrip | The three roundtrip guarantees defined in [IMPORT_EXPORT.md](IMPORT_EXPORT.md), now including `export_is_byte_identical_with_and_without_the_product_database` (`crates/knx-app/tests/product_db.rs`) | Session 3, extended Session 4 |
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
