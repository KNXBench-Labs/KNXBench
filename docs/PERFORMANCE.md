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
- Context for whoever re-measures: other work was happening in sibling git
  worktrees of this repository during the same session, so the `ps aux` check
  above is a statement about the measurement window and not about the whole
  afternoon. A run on an otherwise idle machine may come out faster; a run
  alongside a workspace build will come out slower. Neither invalidates the
  ratios between the five stages, which is what this file is actually for.

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

### `load_project` investigation (2026-09-16)

The open stage was traced on commit `2499127` with the same ignored release
test and synthetic project described above. The measurement machine was the
same AMD Ryzen 7 5800X host (`Linux 7.2.3-arch1-3`, 16 logical CPUs,
`cargo 1.98.0`, `rustc 1.98.0`). No other `cargo` or `rustc` process was
visible around the selected runs. Other project work was active during the
session, so absolute wall-clock results remain sensitive to machine load.

Temporary `Instant` timers split `load_project` at its existing loader calls;
a separate run used SQLite `sqlite3_trace_v2` from immediately before to
immediately after that function. The timers, trace callback, and temporary
dependency were removed after measurement. Reproduce the uninstrumented
top-level number with the command in [Reproduction](#reproduction); the exact
subphase and statement counts below require equivalent temporary
instrumentation.

Three phase-instrumented runs without SQL tracing measured 984.833, 950.831,
and 944.304 ms. Uninstrumented runs measured 1,231.067 ms earlier in the
session and 994.615 ms in a final quiet run after all temporary instrumentation
was removed. This spread is treated as machine variance, not a code regression.
The most stable 944.304 ms phase run had 944.252 ms inside the instrumented
function:

| Load phase | Wall time | Share of instrumented load |
|---|---:|---:|
| Device graph, total | 922.290 ms | 97.7% |
| `load_device` (device row and binary refs) | 122.007 ms | 12.9% |
| Communication-object ID lists | 59.269 ms | 6.3% |
| Communication-object rows and overrides | 511.900 ms | 54.2% |
| Group links | 222.655 ms | 23.6% |
| Installation graph, total | 21.899 ms | 2.3% |
| Topology | 1.159 ms | 0.1% |
| Buildings | 8.640 ms | 0.9% |
| Group ranges | 2.933 ms | 0.3% |
| Group addresses | 8.809 ms | 0.9% |

The three communication-object calls in the table cover four SQL families:
the per-device ID list, the object row, its overrides, and its links. Together
they used 793.824 ms, 84.1% of the complete untraced load.

The SQL trace counted 76,051 statement executions across 24 SQL forms:

| Loader work | Statement executions |
|---|---:|
| Project info, strings, and allocator state | 3 |
| Device-ID scan | 1 |
| Device rows and binary refs (5,000 each) | 10,000 |
| Communication-object graph: 5,000 ID lists plus three queries for each of 20,000 objects | 65,000 |
| Installation row | 1 |
| Topology (base queries plus per-area/per-line membership) | 25 |
| Buildings (base query plus children and devices for each of 364 parts) | 729 |
| Group ranges (base query plus children for each of 288 ranges) | 289 |
| Group addresses, parameters, and modules | 3 |
| **Total** | **76,051** |

The 65,000 communication-object statements are 85.5% of all executions. In
the traced run, SQLite reported 700 ms of profile time over 1,072.657 ms of
instrumented wall time. The top forms were the 20,000 override queries
(238 ms), 20,000 group-link queries (169 ms), 20,000 communication-object row
queries (157 ms), 5,000 device-row queries (44 ms), 5,000 binary-ref queries
(36 ms), and 5,000 communication-object-ID queries (34 ms). SQLite profile
time covers statement execution, not Rust row mapping, allocations, domain-map
insertion, orchestration, or trace overhead; the 372.657 ms difference cannot
be assigned more narrowly by this measurement.

Statement-status counters found no full-table scan in the repeated
communication-object, override, link, device, or binary-ref point queries.
The only full-scan steps were the expected 4,999 steps of the one outer
`SELECT id FROM device ORDER BY id`. Repeated small sorts remain, including
one for each per-device communication-object-ID query, but that SQL form used
34 ms in the trace. The measured cause is therefore the number of indexed
point queries and the work to reconstruct the object graph around them, not a
hidden full scan.

The first optimization candidate is a bulk communication-object graph loader:
read communication objects ordered by device and position, then overrides and
links ordered by communication-object ID and position, and bucket those rows
in Rust. This targets 65,000 calls and the 84.1% wall-time region identified
above. An implementation must preserve device ownership, communication-object
position, override decoding and error behavior, module-instance association,
and group-link order. Lower latency from batching is still a hypothesis until
the same benchmark and round-trip tests measure it; this investigation made no
production optimization.

### Bulk communication-object load result (2026-09-16)

`load_project` now reads the communication-object graph in three ordered scans
instead of issuing one ID-list query per device and three graph queries per
communication object.
The loader still reads each device row and its `binary_data_ref` rows
individually, so that remaining device/binary N+1 work is deliberately outside
this change.

On the machine and command described above, one post-change release run
reported:

```text
PERF device_count=5000 group_address_count=20000 building_depth=5 com_objects=20000
PERF knxproj_bytes=407784
PERF export_ms=83.299
PERF import_ms=207.944
PERF open_ms=245.367
PERF projection_ms=35.878
PERF search_total_ms=29.681 queries=41 avg_us_per_query=723.9
```

The earlier final quiet, uninstrumented run recorded `open_ms=994.615`; this
run recorded `open_ms=245.367`. These are single wall-clock observations on a
shared development machine, so they demonstrate the measured result in this
environment and do not establish a stable speedup.

## Non-claims

This benchmark says nothing about ETS compatibility or KNX certification.
It measures four numbers, once, on one machine, running this project's own
code against data this project's own generator made up.
