# Test-suite speed: change-scoped gates, tagged lanes, bundled Rust tests (Claude, 2026-10-19)

## Request

The owner: far too many tests run, which slows development down; analyse the
construct and make it faster. Owner decisions (clarify, 19 October): browser
cases parallel by default with a static guard for real-time cases; the
complete gate only for Alpha/Beta/full releases, routine packages
change-scoped; Rust test binaries measured first, bundled if the gain is
clear.

## What was slow (measured)

- Every package ran the complete 16-stage gate (last one 2 h 01 min with the
  lease wait).
- The per-file hash admission of the parallel browser lane had decayed: all
  713 default cases ran on one worker (1,172.9 s).
- 356 Rust test binaries with full debug info: cold `--no-run` 206.5 s,
  rebuild after a `knx-core` change 194.6 s, a strictly serial 467.2 s run,
  71.6 GB target.

## Change

- `tools/gate_scope.py` (planner) and `tools/run_package_gate.py` (runner,
  receipt, `--full`, `--dry-run`, refusals); `run_web_gate.py --profile
  targeted`; `run_corpus_tests.py --package` and module-path names.
- `tools/browser_lanes.py` guard; `@exclusive` tags in the specs;
  `browser-lanes.json` removed; `split_lanes` by tag.
- `tests/it/main.rs` per crate (`large_member_memory.rs` stays alone),
  `[profile.dev]` line tables + optimised dependencies, layout guard.
- Spec fixes found by the parallel lane: explorer pulse phase race, file-menu
  zoom cases pinned to a toast-free date.
- ADR-0144, speed dossier, contract, VERIFICATION, pitfalls, build manual, CI.

## Results

Cold test build 206.5 → 118.0 s; `knx-core` rebuild 194.6 → 10.0 s; test
run 467.2 → 145.0 s; target 71.6 → 6.7 GB. Complete gate on the reviewed
candidate 37.4 min of stages; a one-spec change-scoped gate 5.4 min. Replay of
40 commits on `main`: none needed the complete gate.

## Reviews

Three independent read-only reviews. Round 1: 4 critical (deleted files,
Vitest reading Rust/docs, missing `table-columns`, `xtask` corpus failure), 6
important (unknown web paths, runner inputs, guard gaps, bindings restore,
corpus problems hidden, production workers). Round 2: deleted fixture page
selected no spec (a regression of round 1), same-file methods missed. Round 3:
one minor false positive (function declarations as members), fixed. All test-
first; one already-green test proved by mutation.

## Notes for the next agent

- Use `python3 tools/run_package_gate.py --base origin/main --dry-run` before
  every package; `--full` only for releases.
- A change to `tools/gate_scope.py`, `tools/run_package_gate.py` or
  `tools/web_gate_support.py` selects the complete gate by design.
- Open items are listed in `docs/OPEN_WORK.md` ("Test gates: change-scoped by
  default").
