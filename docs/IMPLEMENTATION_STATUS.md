# IMPLEMENTATION_STATUS.md

Last updated: 2026-09-02 (Session 3)

## Where the project stands

| Session | Scope | Status |
| --- | --- | --- |
| 0 | Technical research | **Done** — see [RESEARCH.md](RESEARCH.md) |
| 1 | Architecture | **Done** — see [ARCHITECTURE.md](ARCHITECTURE.md), [adr/](adr/), [design spec](superpowers/specs/2026-09-02-knx-architecture-design.md) |
| 2 | KNX core | **Done** — see [DATA_MODEL.md](DATA_MODEL.md) |
| 3 | ETS project import | **Done** — see [IMPORT_EXPORT.md](IMPORT_EXPORT.md), [COMPATIBILITY.md](COMPATIBILITY.md) |
| 4 | Manufacturer database | Not started |
| 5 | UI / UX | Not started |
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
(`command.rs`). Schema version bumped to 2 in Session 3 alongside
`knx-store`'s own (lockstep by design), though the Rust shape of `Project`
did not change.

**`knx-store` holds the schema-version migration chain** (`migration.rs`,
now through v2) and the opaque passthrough table (`opaque.rs`,
`insert_opaque`/`load_opaque`), with two frozen fixtures
(`fixtures/v1-empty.sqlite`, `fixtures/v2-empty.sqlite`). Entity tables
(persisting `knx_core::Project` itself into SQLite, beyond the opaque
store) are **not** part of Session 3 and were never listed as such in
[ROADMAP.md](ROADMAP.md)'s own Session 3 deliverables — that promise
belonged only to this document's previous revision, corrected here. Full
entity persistence is Session 4 work, alongside the product database.

**`knx-etsproj` holds the full six-stage import/export pipeline**: the ZIP
container (`container.rs`, with a 64 MB per-entry size guard), schema
detection (`detect.rs`), the tolerant streaming parser for both `0.xml`
and `Project.xml` (`parse/`, `known.rs`'s schema-11 table), attribute value
conversions (`values.rs`), structural validation (`validate.rs`), the
mapper into `knx_core::Project` (`map.rs`), datapoint-type inference
(`infer.rs`), the opaque-entry collector (`opaque.rs`), the import report
(`report.rs`), orchestration (`import_knxproj`/`import_knxproj_bytes` in
`lib.rs`), schema-11 XML writers and container export (`export/`), and the
declared semantic-equality comparison (`compare.rs`). 84 tests in the crate
(63 unit, plus the golden, oracle, roundtrip and malformed-input
integration suites), on top of `knx-core`'s 46 and `knx-store`'s 8.

**`knx-app` holds the import service** (`import.rs`:
`import_ets_project`) — the one crate that sees both `knx-etsproj` and
`knx-store`, enforced by `check-layering`. **`apps/knx-cli` gains an
`import` subcommand**: `knx import <file.knxproj> [--store <path>]
[--report-json <path>]`, human-readable counts on stdout, exit code 0 on
a produced project regardless of warnings, 1 on failure.

`knx-productdb`, `knx-net`, `knx-secure` remain empty crates with their
responsibility stated in a doc comment. There is still no product
database, no manufacturer parameter interpretation, and no UI.

Three architectural rules are enforced mechanically rather than by
discipline, and all three have been observed to fail on a deliberate
violation:

- `cargo run -p xtask -- check-layering` — `knx-core` reaches none of
  `serde_json`, `quick-xml`, `rusqlite`, `tokio`; `knx-etsproj` reaches no
  `knx-store` (Session 3).
- `cargo deny check` — no licence outside the allowlist enters the graph; GPL
  is not on the allowlist.

## What exists

| Path | Purpose |
| --- | --- |
| `Cargo.toml`, `rust-toolchain.toml` | Workspace root; toolchain pinned to Rust 1.98.0. |
| `crates/knx-core/` | Domain model per [DATA_MODEL.md](DATA_MODEL.md), sections 1–9 and 11. No IO. |
| `crates/knx-store/` | SQLite schema-version migration chain through v2 (`migration.rs`), the opaque passthrough table (`opaque.rs`), and two frozen fixtures. |
| `crates/knx-etsproj/` | The full six-stage `.knxproj` import/export pipeline — see the Session 3 paragraph above. No dependency on `knx-store`. |
| `crates/knx-app/` | The import service (`import.rs`) — the one crate that sees both `knx-etsproj` and `knx-store`. |
| `crates/knx-productdb/`, `knx-net/`, `knx-secure/` | Empty crates with their responsibility stated in a doc comment. `knx-secure` deliberately has no dependencies at all. |
| `apps/knx-cli/` | Headless entry point, binary `knx`. `import` subcommand added Session 3; prints its version otherwise. |
| `xtask/` | Repository verification tasks. `check-layering` walks the resolved dependency graph and reports the shortest path to any forbidden package, for two roots (`knx-core`, `knx-etsproj`); `freeze-fixture` (Session 3) regenerates a canonical migration-test fixture. |
| `deny.toml` | Licence, advisory, ban and source policy for `cargo-deny`. |
| `.github/workflows/ci.yml` | CI: formatting, clippy with `-D warnings`, tests, the layering gate, and `cargo deny check`. |
| `docs/ARCHITECTURE.md` | Layering, workspace layout, enforced rules, core approach, UI boundary, KNXnet/IP, key material, test strategy. |
| `docs/DATA_MODEL.md` | The target domain model, per section marked implemented / planned / retained-but-uninterpreted. |
| `docs/IMPORT_EXPORT.md` | The six-stage pipeline, container handling, tolerant parsing, opaque store, import report, export rules, roundtrip guarantees. |
| `docs/COMPATIBILITY.md` | What is verified, what is expected but unverified, what is not supported — every verified row now names the test that verifies it. |
| `docs/KNOWN_LIMITATIONS.md` | Thirteen limitations with cause, impact and the condition that would lift each. |
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

Session 4 (manufacturer databases). Build the shared product database and
its ingest path (IMPORT_EXPORT §10, [ADR-0005](adr/0005-separate-product-database.md)):
manufacturer data currently sits in the opaque store
([KNOWN_LIMITATIONS.md](KNOWN_LIMITATIONS.md) §12), one copy per project,
and needs to move to the shared, content-hashed store the target design
describes. Entry condition met: import produces `ProductRefId`/
`Hardware2ProgramRefId` references worth resolving.

Known gaps carried into Session 4:

- `DptRef` does not yet parse `ComObjectRef/@DatapointType` when it is a
  space-separated list of alternatives (RESEARCH §4.2).
- The `when/@test` expression grammar that would make device parameters
  interpretable is unresearched (RESEARCH R3).
- Schema 23's known-element table does not exist; schema 23 is detected and
  refused by name, not misread (RESEARCH §3.3, [KNOWN_LIMITATIONS.md](KNOWN_LIMITATIONS.md) §1).
- Whether ETS re-imports an unsigned third-party `.knxproj` remains
  untested (risk R9) — see [COMPATIBILITY.md](COMPATIBILITY.md).
