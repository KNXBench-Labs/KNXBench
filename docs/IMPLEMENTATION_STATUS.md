# IMPLEMENTATION_STATUS.md

Last updated: 2026-09-03 (Session 5, cycle 2)

## Where the project stands

| Session | Scope | Status |
| --- | --- | --- |
| 0 | Technical research | **Done** — see [RESEARCH.md](RESEARCH.md) |
| 1 | Architecture | **Done** — see [ARCHITECTURE.md](ARCHITECTURE.md), [adr/](adr/), [design spec](superpowers/specs/2026-09-02-knx-architecture-design.md) |
| 2 | KNX core | **Done** — see [DATA_MODEL.md](DATA_MODEL.md) |
| 3 | ETS project import | **Done** — see [IMPORT_EXPORT.md](IMPORT_EXPORT.md), [COMPATIBILITY.md](COMPATIBILITY.md) |
| 4 | Manufacturer database | **Done** — see [IMPORT_EXPORT.md §10](IMPORT_EXPORT.md), [ADR-0011](adr/0011-product-database-storage.md), [ADR-0012](adr/0012-enrichment-into-absent-slots.md) |
| 5 | UI / UX | **In progress** — cycle 1 (shell, projection, Project Explorer) and cycle 2 (`knx-store` entity persistence, [design spec](superpowers/specs/2026-09-03-knx-entity-persistence-design.md)) done, see [ROADMAP.md](ROADMAP.md) |
| 6 | KNXnet/IP | Not started |
| 7 | Integration & hardening | Not started |

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
`StringTable` and `IdAllocators` gain `#[derive(PartialEq)]`, and
`StringTable` gains a public `iter()` — both additive, no behavior change
(see [DATA_MODEL.md §11](DATA_MODEL.md)). 48 tests.

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
IO-free bar as `knx-core` itself. 12 tests.

**`apps/knx-desktop` is the desktop shell**: Tauri v2 with a React + Vite
frontend, scaffolded this session rather than in Session 1. Its one
command so far, `open_project`, imports a `.knxproj`, projects it through
`knx-projection`, and returns the resulting `ProjectTree` to the
frontend's Project Explorer. The store connection it opens for that
import is in-memory (`knx_store::open_and_migrate_in_memory`) and
discarded on drop — this cycle displays a project but does not persist or
reload any of the desktop app's own state. 1 test.

`knx-net`, `knx-secure` remain empty crates with their responsibility
stated in a doc comment. There is still no manufacturer parameter
*interpretation* (the `Dynamic` tree, `when/@test`), and the desktop UI so
far covers only the Project Explorer — no properties inspector, search,
command palette, dark/light mode or persistence yet.

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

292 tests pass across the workspace as of this session.

## What exists

| Path | Purpose |
| --- | --- |
| `Cargo.toml`, `rust-toolchain.toml` | Workspace root; toolchain pinned to Rust 1.98.0. |
| `crates/knx-core/` | Domain model per [DATA_MODEL.md](DATA_MODEL.md), sections 1–9 and 11. No IO. |
| `crates/knx-store/` | SQLite schema-version migration chain through v4 (`migration.rs`), the opaque passthrough table (`opaque.rs`), the manufacturer manifest table (`manifest.rs`, Session 4), full `knx_core::Project` entity persistence (`project.rs`, `strings.rs`, `topology.rs`, `building.rs`, `devices.rs`, `group.rs`, `parameter.rs`, `command_sync.rs` — Session 5 cycle 2), and four frozen fixtures. |
| `crates/knx-etsproj/` | The full six-stage `.knxproj` import/export pipeline — see the Session 3 paragraph above. Hands manufacturer files out separately from opaque entries (Session 4). No dependency on `knx-store`. |
| `crates/knx-productdb/` | The shared product database: own SQLite migration chain, streaming manufacturer-XML ingest, and enrichment of `ComObjectInstance` — see the Session 4 paragraph above. No dependency on `knx-etsproj` or `knx-store`. |
| `crates/knx-projection/` | Pure `Project` → `ProjectTree` projection with `ts-rs` TypeScript bindings — see the Session 5 paragraph above. No dependency beyond `knx-core`; the fourth `check-layering` root. |
| `crates/knx-app/` | The import and export services (`import.rs`, `export.rs`) — the one crate that sees `knx-etsproj`, `knx-store` and `knx-productdb` together. |
| `crates/knx-net/`, `knx-secure/` | Empty crates with their responsibility stated in a doc comment. `knx-secure` deliberately has no dependencies at all. |
| `apps/knx-cli/` | Headless entry point, binary `knx`. `import` subcommand (Session 3, `--product-db`/`--no-product-db` added Session 4) and `products` subcommand (Session 4); prints its version otherwise. |
| `apps/knx-desktop/` | Tauri v2 + React + Vite desktop shell — see the Session 5 paragraph above. `src-tauri/` holds the Rust side (`open_project` command, in-memory store connection); `src/` the React frontend, including the `ts-rs`-generated bindings under `src/bindings/`. |
| `xtask/` | Repository verification tasks. `check-layering` walks the resolved dependency graph and reports the shortest path to any forbidden package, for four roots (`knx-core`, `knx-etsproj`, `knx-productdb` — the third added Session 4 — and `knx-projection`, the fourth, added Session 5); `freeze-fixture` (Session 3) regenerates a canonical migration-test fixture. |
| `deny.toml` | Licence, advisory, ban and source policy for `cargo-deny`. |
| `.github/workflows/ci.yml` | CI: Tauri Linux prerequisites and Node.js setup (Session 5), formatting, clippy with `-D warnings`, tests, the layering gate, `cargo deny check`, and a check that `knx-projection`'s `ts-rs` bindings under `apps/knx-desktop/src/bindings` are not stale (Session 5). |
| `docs/ARCHITECTURE.md` | Layering, workspace layout, enforced rules, core approach, UI boundary, KNXnet/IP, key material, test strategy. |
| `docs/DATA_MODEL.md` | The target domain model, per section marked implemented / planned / retained-but-uninterpreted. |
| `docs/IMPORT_EXPORT.md` | The six-stage pipeline, container handling, tolerant parsing, opaque store, import report, export rules, roundtrip guarantees. |
| `docs/COMPATIBILITY.md` | What is verified, what is expected but unverified, what is not supported — every verified row now names the test that verifies it. |
| `docs/KNOWN_LIMITATIONS.md` | Fifteen limitations with cause, impact and the condition that would lift each. |
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

## Next session

Session 5 (UI / UX). ROADMAP's entry condition — "import produces a model
worth displaying" — is met: the domain model, import/export pipeline and
shared product database all exist, and communication objects now carry
resolved `Program`/`ProgramRef` values in addition to whatever the
project's own `Instance` layer stated.

Known gaps carried forward, none blocking Session 5:

- `knx-store`'s entity persistence (above) has no `knx-app`/desktop wiring
  yet — no Tauri `save_project`/`load_project` command, no save dialog or
  UX. Only `SetIndividualAddress`, `SetComObjectDpt`/`RestoreComObjectDpt`
  and `CreateGroupAddress`/`DeleteGroupAddress` have an incremental
  `sync_after_command` path; every other entity/attribute is written only
  by a full `save_project` until a command exists for it.
- `manifest.rs`/`opaque.rs` still open their own internal SQL transaction
  the same way `strings.rs` did before this cycle's `SAVEPOINT` fix
  (above); not currently reachable from inside `save_project`'s
  transaction, so no live bug, but the same hazard would resurface if a
  future task ever wires them into it. Two ad-hoc `SELECT` queries in
  `command_sync.rs` (a device's current placement; the next
  `group_address` position) should move behind named helpers in
  `devices.rs`/`group.rs` if that module grows — harmless today.
- The `when/@test` expression grammar that would make device parameters
  interpretable is unresearched (RESEARCH R3) — its own spike, prerequisite
  for a parameter editor.
- A program value behind an instance-level `Empty` slot stays invisible in
  the model ([KNOWN_LIMITATIONS.md](KNOWN_LIMITATIONS.md) §12); lifted by
  a layer stack in `Override<T>`, a domain-model change deliberately not
  taken this session ([ADR-0012](adr/0012-enrichment-into-absent-slots.md)).
- An ambiguous, space-separated `DatapointType` list fills nothing
  (RESEARCH §4.2, KNOWN_LIMITATIONS §12) — resolving it needs more context
  (e.g. a linked group address's own DPT) than one communication object
  alone carries.
- Schema 23's known-element table does not exist; schema 23 is detected and
  refused by name, not misread (RESEARCH §3.3, [KNOWN_LIMITATIONS.md](KNOWN_LIMITATIONS.md) §1).
  Schema 23 manufacturer data shares this blocker.
- `.knxprod` direct ingest for master data scheme ≥ 12 remains unsupported
  (KNOWN_LIMITATIONS §11); manufacturer data still reaches the product
  database only via a `.knxproj` that already contains it.
- Whether ETS re-imports an unsigned third-party `.knxproj` remains
  untested (risk R9) — see [COMPATIBILITY.md](COMPATIBILITY.md).
