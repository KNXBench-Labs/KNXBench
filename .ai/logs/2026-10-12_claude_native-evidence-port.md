# 2026-10-12 — Claude — native-evidence port (parked-work adoption, package 1)

## What changed
- Ported the seven commits of the parked `fix/native-evidence-20261011` (`b76ee907`)
  onto fresh main with `cherry-pick -n`, keeping each original message, ADR-0113 as
  0113. The branch's own `.ai/CURRENT_STATE.md` entries were not replayed (stale lock
  and status lines); they stay on the parked branch.
- The 10 content conflicts were all additive: messages and the companion census keep
  both sides; main's sectioned Overview wins on layout, so the branch's flat panel list
  moved into main's cards (Import report: evidence, source reports, payloads;
  Technical details: modeled project information); `knx-app/src/lib.rs` keeps both
  modules in order; docs keep both sides in date order.
- Line-level check: every line the branch added is in the port's diff; the only
  differences are the Dashboard placements (`tree` instead of `props.tree`) and three
  blank lines (restored in the docs commit).
- Integration fixes, each test-first or compile-proven:
  - panels became `div`s with `h4` titles (main's test requires labelled sections;
    main's card `h3` style would have restyled the titles), census sorted;
  - `http_group_members.rs` (newer on main) gained `import_report_historical: false`;
  - runner for the three real-server specs (`tools/verify_native_evidence.py`,
    `playwright.native-evidence.config.ts`) — they had none on the branch;
  - export dialog Cancel made quiet (main's catalogue-update convention) and the DE
    "ETS-.knxproj-Export" wording fixed;
  - two regression assertions for guards the mutants showed were unwatched.
- Main moved during the work (docs-only `e9bf84c8`, `e80f8d61` from a parallel
  session); merged before the gate.

## Lessons
- A ported `*.native.ts` spec without a committed runner is untestable evidence. Port
  the runner, or build one, before counting the cases.
- Chromium aborts with "Socket path too long" when `TMPDIR` is a deep scratch path;
  a runner must inherit a short `TMPDIR`, not set its own.
- Schema-23 devices live in `Line/Segment`. A device directly under `Line` is reported
  as an unknown construct and retained, not placed — the importer did the right thing,
  the fixture was wrong.
- A mutant on a guard that a test covers only by accident (byte 0 breaks the JSON
  before the hash check runs) survives. Corrupt the input in a way that only the
  guarded check can catch.
- Removing `initialFocusRef` was an equivalent mutant (Overlay focuses the first
  button anyway); pointing the ref at the other button is the real one.
