# 2026-10-18 — Claude — UP258 stage 0: read-only declared-segment memory evidence

Branch `feat/up258-stage0-20261018` (own worktree), based on `64c5cedf`. Not pushed.

## Delivered

- `knx-core` `commissioning::declared_memory`: validated `DeclaredRanges`
  (≤ 8 ranges, ≤ 2048 octets, no overlap, end below `FFFFh` so every chunk is
  `A_Memory_Read`), 12-octet standard-frame chunk plan.
- `knx-productdb` `declared_segments::derive_declared_segments`: precondition
  `system2::project_tool_metadata`, then every `AbsSegment` in source order plus
  the one Device Object PID 78 `CompareProp`; typed refusal, never partial.
- `knx-net` `diagnostic_export::collect_declared_memory`: shares connect,
  identity reads and teardown with `collect`; memory only after descriptor,
  PID12 and PID78 (`property_matches`) all match; `MEMORY_DEADLINE` 180 s.
- `knx-app` archive format 2 (`memory.json`, manifest `notice`), `read_export`,
  `bundle_export`, `write_new_export`, `prepare_memory`, `acquire_memory`;
  `read_bundle` refuses format 2. Format 1 unchanged.
- `knx-cli` `--profile system2-declared-segments-v1` (preview, `--simulate`,
  `--confirm-read`), help text.
- ADR-0143, contract section, manual CLI section, goal state, IMPLEMENTATION_STATUS,
  OPEN_WORK, draft reporter request (not sent).

## Decisions taken differently from the brief, with reason

- Chunk size: no `PID_MAX_APDU_LENGTH` read. `write_limit` is never below
  12 and the standard-frame cap is 12, so `min` is always 12 (unit test over
  every `ApduLengthSource` shape). One bus exchange fewer, same result.
- Range bound is "end below `FFFFh`" (CP §3.5.2 service switch), slightly
  stricter than "inside 16-bit space", so no chunk can become `A_UserMemory_Read`.
- Shared range validation lives in `knx-core` so ProductDB and the collector
  use one type instead of duplicating checks.
- No new KL id: the boundaries are recorded in ADR-0143 and the contract.
- `tokio` `test-util` added to `knx-net` dev-dependencies (feature of the
  existing workspace crate) for the paused-time deadline test.

## Evidence (scratch logs, not tracked)

RED: core, productdb, net, app (compile errors for missing API), CLI (3
assertion failures). Mutation: disabling the PID78 check turns
`each_identity_mismatch_sends_no_memory_read` red. The exact public-package
test was written after the derivation and passed on its first run.
Gates: fmt; Clippy `-D warnings` for core/productdb/net/app/cli/server all
targets; `cargo test -p knx-core -p knx-productdb -p knx-net -p knx-app -p knx-cli
--no-fail-fast` 3050 passed, 0 failed, 100 ignored; `knx-server`
`http_device_diagnostics` 5/5; public UP258 package 3/3 (ignored, env);
private corpus 155/155 in 34 targets (`device_reports_corpus` 2/2); xtask
layering/headers/anchors/ledger/corpus-gates; documentation 0 errors; diff-check.

## Open

Reporter's consented run on the UP 258D12; offline comparison of the result;
web entry (UI owner). No bus contact, push or merge in this package.

## Correction round 1 (review findings F1–F4)

Each finding got a test that failed on the previous HEAD first (scratch logs
`red-cr1-F1..F4.log`), then the fix; one local commit per finding.

- F1 `ccc9ff8a`: `DeclaredSegments` and `MemoryPrepared` fields private with
  read-only accessors; only `derive_declared_segments` / `prepare_memory`
  construct them. Five `compile_fail` doc-tests (forge or edit a profile or
  plan) compiled on the old HEAD and fail now. Stable rustdoc does not check
  the error code, so the RED run is the proof that each snippet is otherwise
  valid. The largest-profile test now derives 8 × 256 octets from a
  synthetic program and reads them from a consistent simulator. The net
  collector documents the app service as its one admitted entry.
- F2 `0984c548`: the format-2 reader re-checks identity admission when any
  chunk shows an outcome, time or bytes (exact reads, element counts,
  descriptor = mask, PID12 = manufacturer, PID78 `property_matches`
  inlineData) and attempt ≤ completion times. 13 rehashed cases.
- F3 `568b6f73`: the reader replays the collector's chunk order and stop
  reason (refusal/partial end the range, error ends the run and names the
  stop, unread gap = `cancelled`, nothing ended = no stop or the shared
  `TUNNEL_DISCONNECT_UNCONFIRMED`; empty partial refused because the session
  turns an empty answer into a refusal). 9 rehashed cases; real refused,
  partial, timeout, cancelled and identity-mismatch simulator runs write and
  reread identically.
- F4 `9ee2005b`: confirmed on the old HEAD — two `memory.json` (format 2) or
  two `report.json` (format 1) entries were accepted. `read_export` now walks
  the raw EOCD and central directory first (≤ 64 entries, no extraction,
  ZIP64 refused), refuses repeated names and requires ZipArchive to agree
  name by name.

Gates after the round: fmt; Clippy `-D warnings` productdb/net/app/cli all
targets; `cargo test -p knx-core -p knx-productdb -p knx-net -p knx-app -p
knx-cli --no-fail-fast` 3062 passed, 0 failed, 100 ignored; public UP258
package 3/3; private corpus 155/155 in 34 targets (`device_reports_corpus`
2/2, format 1 byte-identical); xtask layering/headers/anchors/ledger/
corpus-gates; documentation 0 errors; diff-check. Contract, ADR-0143 and
IMPLEMENTATION_STATUS updated. No bus contact, no push.

## Correction round 2 (re-review: F4 remainder, F5)

RED first again (scratch logs `red-cr2-F4.log`, `red-cr2-F5.log`).

- F4 `b56714b2`: confirmed on the old HEAD — the competing-end-record
  construction (outer EOCD whose comment ends at EOF and holds an inner EOCD
  plus padding pointing at a directory with a duplicate) was accepted for
  `memory.json` (format 2) and `report.json` (format 1); a plain ZIP comment
  was accepted too. The fixed format has no comment: the EOCD must be the
  last 22 octets. The raw walk returns name, central and local header offset
  per entry, and `read_export` requires `ZipArchive` to report the same
  directory start and offsets entry by entry. A mutation run (old EOCD search
  restored, binding kept) still refuses both competing constructions, so
  either rule alone closes the bypass.
- F5 `ad72c0b1`: the reader refused the collector's own in-flight snapshot —
  for every memory chunk (`diagnostics-invalid-memory`) and, found by the
  "bundle every snapshot" test, for every identity item in both formats
  (`diagnostics-invalid-pending`; 7 of 17 format-2 snapshots, 3 of 7
  format-1). Incomplete reports now admit one in-flight item/chunk
  (attempted, not completed, nothing received, no error, nothing later
  attempted, not inside an ended range or run). Complete reports keep the
  strict rule. Extending the rule to identity items goes beyond the memory
  scope of the finding; it was needed for "every snapshot exportable" and
  matches the documented "observer snapshots remain exportable".

Gates after round 2: fmt; Clippy `-D warnings` productdb/net/app/cli all
targets; five-crate `cargo test --no-fail-fast` 3071 passed, 0 failed, 100
ignored; public UP258 package 3/3; private corpus 155/155 in 34 targets
(`device_reports_corpus` 2/2, all format-1 archives still read and rewrite
byte-identically); xtask layering/headers/anchors/ledger/corpus-gates;
documentation 0 errors; diff-check. No bus contact, no push.
