← Previous: [Building from source](02-building-from-source.md) · [Manual index](../README.md)

# Architecture tour

This is a map, not the territory. The binding document is
[`docs/ARCHITECTURE.md`](../../ARCHITECTURE.md); the reasoning behind individual decisions
is in [`docs/adr/`](../../adr/README.md). This chapter is for the moment before you read
either of those — when you have a change in mind and want to know which crate it belongs
in.

## The one rule

Everything is arranged in four layers, and dependencies point downward. Only downward.

```text
UI                    apps/knx-web
 ↓
Application           apps/knx-server, crates/knx-app
 ↓
KNX domain core       crates/knx-core
 ↓
Infrastructure        crates/knx-store, knx-etsproj, knx-productdb, knx-net, knx-secure
```

The core does not know the user interface exists. It also does not know any file format
exists: a change in the ETS schema must never propagate into the domain model. That is the
rule that keeps a second schema generation from becoming a rewrite.

This is not an aspiration in a document. `cargo run -p xtask -- check-layering` walks the
resolved dependency graph from `cargo metadata` and fails the build when a crate reaches
something it must not:

- `knx-core` must not reach `serde_json`, `quick-xml`, `rusqlite`, `tokio`, `axum` or
  `tower`. No serialization format, no XML, no SQL, no async runtime, no HTTP.
- `knx-etsproj` must not reach `knx-store` — the import format does not get to know about
  the storage format.
- `knx-productdb` must reach neither of those two, so product data stays separable from
  project files.
- `knx-secure` must reach neither `knx-core` nor `serde`, so key material cannot travel
  into the project model or accidentally gain a `Serialize` implementation.
- `knx-projection`, `knx-csv`, `knx-report` and `knx-diff` each have their own forbidden
  set.

When the gate fails it prints the shortest path to the forbidden package, which usually
names the offending `use` line for you.

> **Note**
>
> One rule is not mechanically enforced: "the UI talks only to `knx-server`". `apps/knx-web`
> is an npm package and therefore invisible to `cargo metadata`, which is what the gate
> walks. `ARCHITECTURE.md` §4 says so plainly rather than pretending otherwise. If you add a
> path from the frontend to anything but `/api/*`, no machine will stop you. Please stop
> yourself.

The full statement of the rules is in
[`ARCHITECTURE.md` §2](../../ARCHITECTURE.md#2-layering) and
[§4](../../ARCHITECTURE.md#4-enforced-rules).

## What lives where

**`knx-core` — the domain.** Individual addresses, group addresses, areas, lines, devices,
communication objects, building parts, datapoint types and the DPT codec, plus validation.
Its distinguishing feature is provenance: a value is not just a value but a
`Resolved<T> { value, layer }` that remembers which layer it came from — the product's
`ComObject`, the reference, the instance, or a user edit. A tool that does not know where
a value came from cannot tell an inherited default from a decision someone made
([ADR-0004](../../adr/0004-provenance-model.md),
[ADR-0010](../../adr/0010-per-attribute-override-representation.md)). Details in
[`DATA_MODEL.md`](../../DATA_MODEL.md).

**`knx-app` — the application services.** Import orchestration (there is no `.knxproj`
export since [ADR-0028](../../adr/0028-no-knxproj-export.md)), and the command
pattern: every mutation is a `Command` that applies to a `Project` and returns its inverse.
Undo and redo are a stack of those inverses. Validation lives here, not in the UI and not
in the store — a duplicate individual address is rejected in one place, whichever client
asked.

**`knx-store` — the native project file.** Everything about `.knxdb`.

**`knx-etsproj` — the ETS project format.** Everything about `.knxproj`.

**`knx-productdb` — the product database.** A separate SQLite file with its own migration
chain, deliberately not part of the project file
([ADR-0005](../../adr/0005-separate-product-database.md),
[ADR-0011](../../adr/0011-product-database-storage.md)). It installs `.knxprod` packages,
ingests manufacturer data found inside a `.knxproj`, and enriches a project's communication
objects from what it knows. Packages are keyed by content hash, so ingesting the same
manufacturer file twice is a no-op.

**`knx-net` — KNXnet/IP.** An own implementation against ISO 22510: discovery, tunneling,
routing, cEMI encoding, telegram decoding, a line scan, and a commissioning management
session. KNX IP Secure is not implemented. Writing to a real device is a guarded path:
the memory-based download for mask `0701h`/`0705h` application programs, verified on one
device, needs a plan, a device-specific confirmation phrase and a backup first; the
property-based downloader is still simulator-only
([`ARCHITECTURE.md` §8](../../ARCHITECTURE.md#8-knxnetip),
[KNOWN_LIMITATIONS §7](../../KNOWN_LIMITATIONS.md#7-commissioning-a-verified-070nh-memory-path-not-general-device-support)).

**`knx-secure` — key material, in quarantine.** It holds the `.knxproj` ZIP password
derivation and nothing else, and the layering gate keeps it from reaching the domain model
([ADR-0008](../../adr/0008-key-material-isolation.md)).

**`knx-projection`, `knx-csv`, `knx-diff`, `knx-report`** are leaf crates that turn a
project into something else: display projections, group-address CSV, a typed diff, and a
self-contained HTML document. Each depends on `knx-core` and little else, which is what
makes them easy to test.

**`knx-server`** is the only crate that speaks HTTP, and **`knx-desktop/src-tauri`** is a
thin native wrapper whose only workspace dependency is `knx-server`. The desktop build does
not have a second implementation of anything; it starts the same server on a loopback port
and points a WebView at it.

## Where an imported `.knxproj` actually goes

An ETS project archive is a ZIP file full of XML. Following one through the code is the
fastest way to understand the layering, because it crosses every layer exactly once.

```text
project.knxproj
   │
   ├─ knx-etsproj::container   open the ZIP, detect the encryption scheme
   ├─ knx-etsproj::detect      work out which ETS schema version this is
   ├─ knx-etsproj::parse       tolerant XML parse into Source* types
   ├─ knx-etsproj::opaque      keep everything not modeled, verbatim
   ├─ knx-etsproj::map         Source* ──> knx_core::Project
   └─ knx-etsproj::validate    collect findings into an ImportReport
   │
   ├─ knx-app::import_ets_project_observed
   │     ├─ hand the manufacturer files to knx-productdb (ingest)
   │     ├─ enrich the project's communication objects from it
   │     └─ write the opaque entries into knx-store, in one transaction
   │
   └─ knx-projection ──> ProjectTree ──> JSON ──> the browser
```

Four things are worth noticing.

**Nothing is discarded.** XML that the model does not understand is not dropped and not
guessed at; it is stored verbatim in the opaque passthrough store, keyed by its source
path, and counted in the import report
([ADR-0006](../../adr/0006-opaque-passthrough-store.md)). Until 2026-09-20 that was what
let an export write back constructs KNXBench never modeled; with the writer gone, it is
evidence of what the source file said. The full treatment is in
[`IMPORT_EXPORT.md`](../../IMPORT_EXPORT.md).

**The parse is tolerant, the report is not.** Unknown constructs, unresolved references and
validation failures all land in the `ImportReport` with a stage, a severity and the XPath
where they occurred. The CLI exits with code `2` when an import produced a project whose
report still contains errors — a project you can open, and a result you were told about.

**The transaction boundary is real.** The import completes entirely before the first row is
inserted, and the insert runs in one SQLite transaction. A failure leaves the store exactly
as it was, never half-written.

**The entry points are the same code.** `knx import` on the command line, `POST
/api/project/import` in the server, and the golden-test harness all go through
`knx_app::import_ets_project_observed`. The observer argument is the only difference: the
server passes one that feeds the load-progress banner
([ADR-0023](../../adr/0023-load-progress-operation.md)), the others pass `&()`.

Which ETS schema versions are actually verified, and which are merely documented, is in
[`COMPATIBILITY.md`](../../COMPATIBILITY.md) and the manual's
[Supported and unsupported](../reference/02-supported-and-unsupported.md) chapter. The
short version: verification means test material, not optimism.

## The native `.knxdb` file and its migrations

`.knxdb` is an ordinary SQLite database
([ADR-0003](../../adr/0003-sqlite-project-format.md)). You can open one with `sqlite3` and
look around, which is occasionally the fastest way to answer a question.

The schema lives in `crates/knx-store/src/`, one module per area — `project.rs`,
`topology.rs`, `devices.rs`, `building.rs`, `group.rs`, `parameter.rs`,
`module_instance.rs`, `opaque.rs`, `strings.rs`, `manifest.rs`, `command_sync.rs`.

Migrations live in `crates/knx-store/src/migration.rs` and the mechanism is deliberately
boring: SQLite's own `user_version` pragma is the entire version marker. There is an
ordered chain of migration functions where index `i` migrates version `i` to `i + 1`,
`CURRENT_SCHEMA_VERSION` is the end of the chain — **10** as of 2026-10-06 — and
opening a file runs every pending step and then sets the pragma, all in one transaction,
so a failed upgrade leaves the file as it was. A file whose
`user_version` is *higher* than the binary supports is refused rather than opened
hopefully, because guessing at a future schema is how you lose somebody's project.

Adding a migration means appending a function to the chain and raising the constant. Never
editing an existing one: somebody's file has already been through it.

The product database has its own, separate chain in
`crates/knx-productdb/src/migration.rs`, with its own `CURRENT_PRODUCTDB_VERSION` — **21**
as of 2026-10-06. A migration there may re-derive anything the stored package bytes
determine, and must not invent what only the original install knew
([ADR-0020](../../adr/0020-migrations-may-rederive-from-stored-bytes.md)).

## How the frontend talks to the server

Over `fetch()`, to `/api/*`, and nothing else. There are no Tauri `invoke()` calls left,
which is why the same frontend bundle works in a browser and inside the desktop shell.

The client is one file, `apps/knx-web/src/api.ts`: a `request<T>()` helper plus one
exported function per route. Errors come back as a plain `Error` with the HTTP status
attached, so a caller can tell "no bus session exists yet" (`404`) from a real failure
without an `instanceof` check.

The types on the wire are **projections**, not mirrored domain types. `knx-projection`
turns a `Project` into display-shaped structures — `ProjectTree`, `DeviceDetail`,
`GroupAddressNode` and friends — and `ts-rs` generates the TypeScript for them into
`apps/knx-web/src/bindings/`. Those files are generated, committed, and checked: CI
regenerates them and fails if `git diff` shows a change, so the Rust and TypeScript sides
cannot drift apart quietly. Do not hand-edit anything in `bindings/`; change the Rust type
and regenerate.

```bash
TS_RS_EXPORT_DIR=../../apps/knx-web/src/bindings cargo test -p knx-projection
```

Large collections are paginated and filtered on the Rust side rather than shipped whole
into the browser. The reasoning, and the rule that a domain problem never gets a UI
workaround, is in [`ARCHITECTURE.md` §7](../../ARCHITECTURE.md#7-ui-boundary) and
[ADR-0009](../../adr/0009-ui-boundary.md).

In development, Vite serves the frontend on port `1420` and proxies `/api` to
`127.0.0.1:4777`. That port is the `DEV_PORT` constant in `apps/knx-server/src/lib.rs`, and
both the Vite config and the Tauri shell refer to it by name.

The server's routes are grouped by file, which is a useful map in itself:
`routes.rs` (projects, devices, group addresses, catalog, undo/redo), `bus_routes.rs`
(monitor start/stop, telegrams, write), `fs_routes.rs` (the in-browser file picker and
upload), `auth_routes.rs` (login, logout, status) and `debug_report_routes.rs`.

## Where the decisions are written down

[`docs/adr/`](../../adr/README.md) holds the architecture decision records. Each one
states a decision that has already been taken, together with the evidence that forced it —
a measurement, a license, a standard — so that a later reader who disagrees can go back and
check whether the evidence still holds, instead of re-arguing from memory.

An ADR is never edited to reverse itself. A new one supersedes it and the old one is marked
`Superseded by ADR-NNNN`. New records follow
[`template.md`](../../adr/template.md) and take the next free number.

If you are about to make a decision that a future contributor would otherwise have to
reverse-engineer from the code — a format choice, a boundary, a deliberate omission — that
is the moment to write one.

## Where to go next

- [`ARCHITECTURE.md`](../../ARCHITECTURE.md) — the binding document, including the
  [test strategy](../../ARCHITECTURE.md#10-test-strategy) and a
  [decision index](../../ARCHITECTURE.md#11-decision-index).
- [`DATA_MODEL.md`](../../DATA_MODEL.md) — the domain model in detail.
- [`IMPORT_EXPORT.md`](../../IMPORT_EXPORT.md) — the format pipeline, schema by schema.
- [`RESEARCH.md`](../../RESEARCH.md) — the measurements the decisions above rest on.
- [Implementation status](../implementation-status.md) and
  [Known issues](../known-issues.md) — what all of this currently does and does not do.

That is the end of the manual. If something in it turned out to be wrong, that is a bug
like any other, and [Contributing](01-contributing.md) explains where to put it.

Next: [Return to the manual index](../README.md) →
