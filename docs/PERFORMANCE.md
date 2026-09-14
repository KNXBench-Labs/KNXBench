# Performance baseline

A reproducible timing of the five stages every large project has to survive:
import, open, projection, search, export. Not a claim that this is fast
enough, or slow — just what one specific piece of hardware measured, on one
specific size of project, on one specific day, so the next measurement has
something to be compared against.

## What "large" means here

Real ETS projects are not checked into this repository (`OriginalData/` is
gitignored — see `crates/knx-testsupport/src/lib.rs`), so "large" cannot mean
"as big as the file nobody can hand you." Instead it means a **synthetic,
deterministic** project, generated from four numbers alone, with no
dependency on any external file:

| Knob | Value |
|---|---|
| Devices | 5,000 |
| Group addresses | 20,000 |
| Communication objects | 20,000 (4 per device) |
| Building hierarchy depth | 5 levels below the root, branching factor 3 (243 leaves) |

The generator (`crates/knx-app/tests/perf_baseline.rs`) builds a real
`knx_core::Project` value by pure index arithmetic — no XML templating, no
randomness beyond a fixed-seed splitmix64 PRNG used only to pick which
names the search stage queries for. The same four constants always produce
byte-identical output. Group addresses use the `ThreeLevel` style's own bit
widths (32 main ranges × 8 middle ranges × up to 256 addresses), so the
project round-trips through the same code a real `ThreeLevel` project would.

## What is actually measured

The generated project is fed through **production code**, not a shortcut
written only for this benchmark:

1. **export** — `knx_etsproj::export::export_knxproj` writes the generated
   `Project` to `.knxproj` bytes (with one placeholder `Signature` opaque
   entry, so the file round-trips through `Container::open` on the way
   back in).
2. **import** — `knx_app::import_ets_project`, the same function the
   desktop/web import path calls, reads the `.knxproj` back into a fresh
   SQLite-backed store.
3. **open** — `knx_store::save_project` (untimed setup) followed by a timed
   `knx_store::load_project` against a second fresh `.knxdb`: "open" means
   reading a database, not writing one.
4. **projection** — `knx_projection::build_project_tree` over the project
   that came back out of the store.
5. **search** — 41 case-insensitive substring queries (20 device names, 20
   group-address names, 1 deliberate miss) against the loaded project.
   **This is not the frontend's search algorithm.** The real search lives
   client-side, in `apps/knx-web/src/searchMatch.ts`; no equivalent exists
   in the Rust backend today. This stage is a self-contained, honestly
   labeled stand-in over the domain model, written for this benchmark, so
   that "search over N group addresses" has *some* measured number rather
   than none. Treat it as a floor, not as the real client's fuzzy-search
   cost.

## Why gated behind `--ignored`, not a feature flag or its own binary

An `#[ignore]`d `cargo test` integration test. `cargo test --workspace`
(the mandatory gate every commit runs) skips it by default — it never adds
a second to that run — while `cargo test -p knx-app --release --test
perf_baseline -- --ignored` runs it on demand. A feature flag would have
meant a second Cargo feature just for one test file; a dedicated binary
would have meant reimplementing the store/import/export wiring `knx-app`'s
existing dev-dependencies already provide. `--ignored` was the smallest
change that satisfied "does not slow the normal test run."

## Reproduction

From a clean checkout of this repository:

```sh
cargo test -p knx-app --release --test perf_baseline -- --ignored --nocapture
```

`--release`: debug-mode XML parsing and SQLite inserts are not what a real
deployment does, and this file's job is to describe that machine, not a
slower one nobody ships.

## Measured numbers

Measured 2026-09-14 on:

- Machine: `Linux 7.2.3-arch1-3 x86_64` (`uname -srm`)
- Toolchain: `cargo 1.98.0 (797e8a9bc 2026-08-05)` (`cargo --version`)
- Cores: 16 (`nproc`)
- No other `cargo`/`rustc` process was running at measurement time (checked
  via `ps aux` immediately before and after) — this run was **not**
  contended by a concurrent build.

Command:

```sh
cargo test -p knx-app --release --test perf_baseline -- --ignored --nocapture
```

Output:

```
PERF device_count=5000 group_address_count=20000 building_depth=5 com_objects=20000
PERF knxproj_bytes=407784
PERF export_ms=78.025
PERF import_ms=179.163
PERF open_ms=945.067
PERF projection_ms=18.445
PERF search_total_ms=19.031 queries=41 avg_us_per_query=464.2
```

| Stage | Time |
|---|---|
| export (`.knxproj`, 407,784 bytes) | 78.0 ms |
| import (`.knxproj` → store) | 179.2 ms |
| open (`load_project` from `.knxdb`) | 945.1 ms |
| projection (`build_project_tree`) | 18.4 ms |
| search (41 substring queries) | 19.0 ms total, 464.2 µs/query |

These are single-run wall-clock numbers from one process, not an average
over repeated trials, and they include this process's own one-time costs
(allocator warm-up, first-touch page faults) rather than a steady-state
loop. Anyone re-running the reproduction command on different hardware, a
different toolchain, or under load from another build should expect
different absolute numbers — the point of recording the machine and
toolchain above is so a future re-run can tell whether a change in the
numbers came from the code or from the environment.

## Reading the numbers, honestly

`open` is the slowest stage by a wide margin — nearly five times `import`,
which itself parses and validates the same amount of data from XML.
`knx_store::load_project` was not profiled beyond this single top-level
number, so nothing below the store's `load_project` function has been
identified as *the* bottleneck yet; no code in this repository has been
changed to speed it up, per this task's own rule of not optimizing
anything the numbers themselves have not pinned down.

## Non-claims

This benchmark says nothing about ETS compatibility or KNX certification.
It measures four numbers, once, on one machine, running this project's own
code against data this project's own generator made up.
