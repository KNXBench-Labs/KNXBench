# `knx_store::load_project` analysis — 2026-09-16

## Scope

Measured root-cause analysis of the 5,000-device / 20,000-group-address /
20,000-communication-object synthetic performance fixture. No production
optimization was implemented. Temporary timers, SQLite tracing code, and its
temporary direct dependency were removed before the final diff.

Baseline commit: `2499127e271f877ab1d67b517db21cbb577f7f95` on branch
`load-project-analysis`.

## Reproduction and environment

```sh
cargo test -p knx-app --release --test perf_baseline -- --ignored --nocapture
```

- Linux `7.2.3-arch1-3` x86_64
- AMD Ryzen 7 5800X, 8 cores / 16 threads
- `cargo 1.98.0 (797e8a9bc 2026-08-05)`
- `rustc 1.98.0 (88d9e12ae 2026-08-18)`
- No other `cargo`/`rustc` process was visible immediately around the selected
  phase runs. Sibling work existed during the session, so absolute wall clock
  is less reliable than the repeated ratios and exact statement counts.

Observed top-level open times:

- Uninstrumented warm run early in the session: 1,231.067 ms.
- Final quiet uninstrumented run after removing all temporary instrumentation:
  994.615 ms.
- Phase timers, no SQL trace: 984.833, 950.831, 944.304 ms.
- The spread is recorded as machine variance, not evidence of a regression.
- SQL trace plus phase timers: 1,072.705 ms outer / 1,072.657 ms inside
  `load_project`; the trace itself adds overhead.

## Complete load path

`project::load_project` reconstructs one normalized `Project` in this order:

1. Read the singleton `project_info` row. Missing row maps only
   `QueryReturnedNoRows` to `StoreError::NotSaved`.
2. Read all `string_table_entry` rows using the stored default language.
3. Restore all persisted `IdAllocators` counters from the singleton row.
4. Read every device ID in stable ID order.
5. For every device:
   - read its scalar row;
   - read binary refs in position order;
   - read its communication-object IDs in position order;
   - for every communication object, read its scalar row, all override rows,
     and group links in position order;
   - insert communication objects and then the device into `Devices`.
6. Read installation rows. For each installation:
   - rebuild topology: area rows, per-area line memberships, line rows,
     per-line device memberships, and unassigned devices;
   - rebuild buildings from the flat-position row scan plus per-part children
     and device memberships;
   - rebuild group ranges from the flat-position row scan plus per-range
     children;
   - read group addresses, parameters, and module instances (including a
     per-module argument query when modules exist);
   - insert module instances into the global `Devices` owner and assemble the
     installation.
7. Return the current core schema version. The migration layer has already
   upgraded the file, so the pre-migration file version is deliberately not
   copied into the domain object.

## Reconstruction invariants

- `knx-core` remains independent of SQL; all decoding stays in `knx-store`.
- Stored list positions define device-to-line, device-to-building,
  device-to-communication-object, hierarchy child, override, and link order.
- `Devices` is the global owner of devices, communication objects, and module
  instances; topology and buildings retain IDs rather than duplicate objects.
- Device ownership of each communication object and module-instance IDs must
  survive reconstruction.
- Override state/layer/text-kind decoding and its existing unknown-attribute
  error semantics must be preserved.
- Group-link direction and position must survive unchanged.
- Unknown group-address styles remain errors. Stored timestamps and network
  addresses rely on save/import validation and are parsed under that invariant.
- Allocator counters are restored from persistence rather than inferred from
  maximum IDs, so future allocation behavior round-trips.
- Empty binary refs, overrides, links, parameters, modules, and unassigned
  lists are valid and must remain distinguishable from load failure.

## Exact statement counts

`sqlite3_trace_v2` was attached only around `load_project`:

| Work | Calls |
|---|---:|
| Project info + strings + allocator state | 3 |
| Outer device-ID scan | 1 |
| Device scalar row + binary refs, each per 5,000 devices | 10,000 |
| Communication-object IDs, per device | 5,000 |
| Communication-object scalar row, per 20,000 objects | 20,000 |
| Overrides, per 20,000 objects | 20,000 |
| Group links, per 20,000 objects | 20,000 |
| Installation rows | 1 |
| Topology | 25 |
| Buildings | 729 |
| Group ranges | 289 |
| Group addresses + parameters + modules | 3 |
| **Total** | **76,051** |

The communication-object graph accounts for 65,000 calls (85.5%). Device
rows and empty binary-ref lists add 10,000. This is direct N+1 behavior:
query counts scale with devices, objects, building parts, ranges, areas, and
lines rather than with a fixed number of entity-table scans.

SQLite statement-status counters reported zero full-scan steps for the major
repeated point queries. The only full-scan steps were 4,999 from the intended
outer device-ID scan. There were repeated small sorts, most visibly 5,000
sorts for communication-object IDs ordered by position, because the existing
index is on `device_id` alone. That statement family consumed 34 ms in the
trace and is secondary to the complete object-graph query count.

## Timing by phase

The selected stable phase run measured 944.252 ms inside `load_project`:

| Phase | Milliseconds | Share |
|---|---:|---:|
| Device graph total | 922.290 | 97.7% |
| Device rows + binary refs | 122.007 | 12.9% |
| Communication-object ID lists | 59.269 | 6.3% |
| Communication-object rows + overrides | 511.900 | 54.2% |
| Group links | 222.655 | 23.6% |
| Installation graph total | 21.899 | 2.3% |
| Topology | 1.159 | 0.1% |
| Buildings | 8.640 | 0.9% |
| Group ranges | 2.933 | 0.3% |
| Group addresses | 8.809 | 0.9% |
| Parameters | 0.183 | <0.1% |
| Modules | 0.160 | <0.1% |

The communication-object ID, object/override, and link calls together used
793.824 ms (84.1% of the complete load); they cover four SQL families.

In the statement-status trace, SQLite profile time summed to 700 ms. Leading
forms were overrides 238 ms, links 169 ms, communication-object rows 157 ms,
device rows 44 ms, binary refs 36 ms, and communication-object IDs 34 ms.
SQLite profile time excludes Rust row mapping, allocation, domain-map
insertion, orchestration, and callback overhead; the 372.657 ms difference
from traced wall clock is not attributed further.

## Findings and next experiment

**Measured:** the device graph owns 97.7% of load wall time. The
communication-object graph owns 85.5% of statement executions and its timed
calls own 84.1% of load wall time. The repeated queries are indexed; an
accidental full-table scan is not the cause.

**Secondary measured N+1 paths:** buildings issue 729 statements, group
ranges 289, and topology 25. Their combined wall time is small on this
fixture, so they are not the first target.

**Hypothesis to test:** bulk-load communication objects ordered by
`(device_id, position)`, overrides by communication-object ID, and links by
`(communication_object_id, position)`, then bucket them in Rust. This can
replace 65,000 executions with a fixed small number of scans. No speedup is
claimed until the release benchmark measures it.

The implementation must preserve the invariants above, especially ordering,
override decoding and error behavior, module-instance association, device
ownership, and link direction/order. Run focused round-trip tests plus the
same performance fixture before considering device/binary, building, range,
or index changes.

## Verification

- `cargo test -p knx-app --release --test perf_baseline -- --ignored
  --nocapture`: 1 passed; final uninstrumented open 994.615 ms.
- `cargo fmt --all --check`: clean.
- `cargo test -p knx-store`: 63 unit and 2 integration tests passed; doc tests
  passed with no tests.
- `cargo run -p xtask -- check-layering`: passed.
- Local-link check for `docs/PERFORMANCE.md`: passed.
- `git diff --check`: clean.
