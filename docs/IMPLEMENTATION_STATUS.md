# IMPLEMENTATION_STATUS.md

Last updated: 2026-09-02

## Where the project stands

| Session | Scope | Status |
| --- | --- | --- |
| 0 | Technical research | **Done** — see [RESEARCH.md](RESEARCH.md) |
| 1 | Architecture | **Done** — see [ARCHITECTURE.md](ARCHITECTURE.md), [adr/](adr/), [design spec](superpowers/specs/2026-09-02-knx-architecture-design.md) |
| 2 | KNX core | Not started |
| 3 | ETS project import | Not started |
| 4 | Manufacturer database | Not started |
| 5 | UI / UX | Not started |
| 6 | KNXnet/IP | Not started |
| 7 | Integration & hardening | Not started |

**The repository is a buildable Cargo workspace with eight crates, of which
only `knx-core` has content.** That content is `Layer` and `Resolved<T>` — the
provenance types the rest of the model is shaped around
([ADR-0004](adr/0004-provenance-model.md)) — plus one test. The other seven
crates carry a doc comment stating their responsibility and nothing else. There
is no domain model, no importer, no storage and no UI yet.

Two architectural rules are enforced mechanically rather than by discipline,
and both have been observed to fail on a deliberate violation:

- `cargo run -p xtask -- check-layering` — `knx-core` reaches none of
  `serde_json`, `quick-xml`, `rusqlite`, `tokio`.
- `cargo deny check` — no licence outside the allowlist enters the graph; GPL
  is not on the allowlist.

## What exists

| Path | Purpose |
| --- | --- |
| `Cargo.toml`, `rust-toolchain.toml` | Workspace root; toolchain pinned to Rust 1.98.0. |
| `crates/knx-core/` | Domain model. Currently `Layer` and `Resolved<T>`. No IO. |
| `crates/knx-app/`, `knx-store/`, `knx-etsproj/`, `knx-productdb/`, `knx-net/`, `knx-secure/` | Empty crates with their responsibility stated in a doc comment. `knx-secure` deliberately has no dependencies at all. |
| `apps/knx-cli/` | Headless entry point, binary `knx`. Prints its version; no subcommands yet. |
| `xtask/` | Repository verification tasks. `check-layering` walks the resolved dependency graph and reports the shortest path to any forbidden package. |
| `deny.toml` | Licence, advisory, ban and source policy for `cargo-deny`. |
| `.github/workflows/ci.yml` | CI: formatting, clippy with `-D warnings`, tests, the layering gate, and `cargo deny check`. |
| `docs/ARCHITECTURE.md` | Layering, workspace layout, enforced rules, core approach, UI boundary, KNXnet/IP, key material, test strategy. |
| `docs/DATA_MODEL.md` | The target domain model, per section marked implemented / planned / retained-but-uninterpreted. |
| `docs/IMPORT_EXPORT.md` | The six-stage pipeline, container handling, tolerant parsing, opaque store, import report, export rules, roundtrip guarantees. |
| `docs/COMPATIBILITY.md` | What is verified, what is expected but unverified, what is not supported. |
| `docs/KNOWN_LIMITATIONS.md` | Eleven limitations with cause, impact and the condition that would lift each. |
| `docs/ROADMAP.md` | Sessions 2–7 with deliverables and entry conditions. |
| `docs/adr/` | Nine ADRs, a template and an index. |
| `docs/RESEARCH.md` | Session 0 result: verified findings on the `.knxproj` format, manufacturer data, master data, KNXnet/IP, KNX Secure, licensing, risks. |
| `tools/inspect_knxproj.py` | Stdlib-only inspector that reproduces every container/project number quoted in `RESEARCH.md`. |
| `monitor_bus.py` | Captures live telegrams from the KNXnet/IP gateway into `bus_traffic.jsonl`. |
| `Unser Zuhause ets4 - 2025-12-15.knxproj` | Real ETS 4.1.8 reference project (schema 11), unprotected. |
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

Session 2 (KNX core). Implement the model described in
[DATA_MODEL.md](DATA_MODEL.md), starting from the `Layer` and `Resolved<T>`
types that already exist in `crates/knx-core/src/lib.rs`. Entry condition met:
the workspace builds and both gates pass.

Blocking gap to close before Session 3: no ETS5 (schema 13/14/20) or ETS6
(schema 21+) sample project is available. Everything verified so far is schema
11.
