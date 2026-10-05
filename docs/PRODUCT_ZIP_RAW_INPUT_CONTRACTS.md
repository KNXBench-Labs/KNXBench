# Product ZIP raw-input admission contracts

## AR06X scope and source evidence

This slice protects existing raw-input admission, not ZIP member declarations,
successful import of large payloads, a cap raise or a streaming decision.
Source status belongs only in [status/LEDGER](status/LEDGER.md).

On published base `59fb1fc4`, `install_package` in
`crates/knx-productdb/src/package.rs` compares `bytes.len()` with private
`MAX_PACKAGE_SIZE` (256 MiB) before filename dispatch, hashing, transactions or
ZIP parsing. An over-bound input returns `PackageError::SizeLimit` with the
exact caller `source_name`. Do not invent a `<package>` path.

Exactly-at-bound input still undergoes real validation. `preflight_zip` scans
its bounded footer window and rejects an uniform 0xA5 payload with the precise
`InvalidZip` cause `missing complete end-of-directory record`. This is a named
malformed-container refusal, not successful ZIP admission or installation.
No external-format or device-semantic change is introduced.

The fresh existing-six ZIP contract baseline passed 6/0/0. Original log,
producer, all 121 native input hashes and newly compiled executable were
independently checked before test changes. These are old count/declaration
contracts, not new raw-input coverage or additive public totals.

## Proposed native acceptance

- Actual input length exactly 256 MiB: preserve a nonempty seed database and
  original seed archive, reach the precise named missing-footer validation
  error rather than an early size refusal.
- Actual input length 256 MiB + 1 byte: preserve that same complete state and
  original archive, report typed `SizeLimit` with the exact caller filename.

Use the existing deterministic all-table value/BLOB snapshot and retained
archive helpers. Extract only the directly shared failed-import seed check;
rerun the six old cases after this test-helper change. Production code and
constants remain untouched.

## Fixture and execution budget

Each test creates one uniformly nonzero (0xA5) Vec; maximum logical length
268,435,457 bytes. Nonzero initialization avoids confusing lazily mapped
demand-zero storage with a materially resident fixture.
There is no compressed payload, member expansion, archive clone or fixture
file written to disk. Seed/archive and database snapshots remain the small
existing public synthetic fixture. Tests execute with one thread.

Compile and runtime diagnostics are separate: first compile the selected
real test executable, then execute it under a fresh Python `resource.getrusage(RUSAGE_CHILDREN)` harness with an
isolated network and a 120-second runtime timeout. Require actual registered
results and maximum native RSS at/below 512 MiB (524,288 KiB). Do not count
Cargo/rustc RSS as the fixture's runtime metric. The harness has no prior child
usage and runs only the native executable. Retain its original JSON/log.
This budget is a fixture guard, not production hostile-import resource-policy
acceptance. Absence of a resource report is not a pass.

## Semantic controls and remaining boundaries

Compile isolated lower/upper raw-input-bound controls; each snapshot supplies
real workspace target files and passes Cargo metadata admission. Require a
successful compile and the selected specifically named semantic assertion,
not an arbitrary compile error. Preserve canonical source and prior evidence.

Actual successful large-package retention/replay, member/expanded payload
boundaries, real CLI/HTTP callers, cancellation/progress, aggregate hostile
resource policy, owner decisions and Alpha acceptance remain separate.
Native raw-input and semantic-control evidence is now accepted as described
below. Scoped retry stopped at a real header-width violation (101 > 100 columns;
retained); the header was shortened (comment-only). Integrated public16 on merge f867741e (leaf adf7ff29 + owner 036b46a6) independently accepted: 16 exit0, Rust 3240/0/177 in 179 blocks, Web 2001, Chromium 131 plus separate probe 1, 885 frozen inputs, CLI knx 0.1.0-alpha.4+gf867741e.
KL151 and Alpha are not closed by this bounded native contract.

## Runtime measurement provenance

GNU `/usr/bin/time` admission failed with exit 127 before any native test.
No system package is installed. Official Python resource documentation and
the upstream Linux getrusage(2) manual were fetched on 2026-10-05 and
retained in the scoped evidence. Linux reports ru_maxrss in KiB; the new
fresh-process harness reads reaped-child usage, not compiler usage from its
parent runner. Sources: https://docs.python.org/3/library/resource.html and
https://man7.org/linux/man-pages/man2/getrusage.2.html. The configured web
extractor refused URL extraction; direct HTTPS fetched the primary texts.

## Independently checked native evidence

Working-tree fixture on base59fb1fc4: raw selection2/0/0 with6 filtered, then
complete boundary file8/0/0.121 native inputs, original logs, newly compiled
artifact and child-only resource receipts checked. Do not add these totals.
Raw2 measured266.078MiB/5.421s; all8 measured268.734MiB/5.872s. Both run
serially below512MiB/120s; uniformly nonzero buffers materialize resident
fixture pages. These are malformed raw-length cases, not valid large imports.

Two isolated400-real-file workspace snapshots, each with a separate fresh
target, compiled the raw limit -1/+1 controls. Both metadata and compiles
exited0; each exact selected test exited101 at the intended named assertion.
Canonical and snapshot sources were restored and checked. No private data or
KNX hardware was accessed.

First control wrapper exited1 because its verifier read the first of two
Rust --nocapture failure headings. Original refusal/log/runner retained;
lower control independently accepted from that actual compile/runtime evidence,
not rerun. Only the unstarted upper ran in the separate remainder wrapper0.
Runner-authoring syntax correction and missing GNUtime127 are infrastructure
observations, not behavioral kills. Scoped owning-crate and integrated gates
remain pending; this dossier is evidence, not a second status inventory.
