# T08 — read-only line diagnostics UI

Date: 2026-09-21

## Outcome

T08 closes UI gap D6 with an incremental, cancellable line-scan API and a diagnostics
surface. The implementation stays read-only and all transport tests use the in-process fake
tunnel; no gateway, LAN endpoint, or KNX device was contacted.

The server exposes estimate, start, incremental-results, and cancel routes. It retains six
distinct `ProbeOutcome` states, returns a monotonic result cursor, rejects concurrent scans,
and disconnects the tunnel after completion or cancellation. Exclusions are supplied by the
request and the fake-transport regression proves an excluded synthetic address is never sent.

The web UI displays the estimate and its timeout/pacing basis before start, progress and
cancellation while running, all six outcomes, and settings-backed protected exclusions whose
removal requires a second click. Polling is single-flight and stale generations cannot append
results after a new scan or cancellation. Outcome text follows the active UI language.

The simultaneous user ruling on group-address notation was applied in the same branch:
structured group addresses always render with slash notation. Dotted forms remain accepted
only as compatible input and search aliases. ADR-0030, the resolved limitation, tests, and the
implementation-status entry were reconciled.

## Verification

- `cargo fmt --all --check`: exit 0.
- `cargo clippy --workspace --all-targets -- -D warnings`: exit 0.
- `cargo test --workspace --no-fail-fast`: 1,937 passed across 92 result blocks, 0 failed.
- `cargo run -p xtask -- check-layering`: exit 0.
- `cargo run -p xtask -- check-headers`: 186 present, 162 absent, ceiling 162.
- `cargo run -p xtask -- check-anchors`: 376 links across 175 Markdown files, none dead.
- `cargo deny check`: exit 0.
- `npx tsc --noEmit`: exit 0.
- `npx vitest run`: 816 passed across 59 files.

The first all-workspace attempt exhausted the `/tmp` quota while linking. Only the dedicated
`/tmp/knxbench-t08-target` build cache was pruned, then the complete Rust sequence was rerun
with one build job, incremental compilation disabled, and debug symbols disabled. The second
run reached every gate; the header gate identified the new Vitest file's environment pragma
above its purpose sentence. Reordering those two lines restored the unchanged ceiling, and
the header, anchor, dependency, focused UI, TypeScript, and full web gates were rerun green.

## Follow-up contract

T09 may consume `LineScanResultsResponse` to build a previewable, undoable project diff. It
must not mutate project state during a scan, and uncertain scan outcomes must not be collapsed
into occupied or vacant.
