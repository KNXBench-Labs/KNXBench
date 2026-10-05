# 2026-10-04 — U21 parts A and B: motion for the flow view

Agent: Claude, goal-ui.md owner session; Web lock `de941a9e` (held for part C).

- Part A (reducer): window, leader, edge activity, fresh-event ring. RED 7; 8 mutants
  caught (fan-out, tie, lowest id, window boundary, unknown age, backlog, ring bound,
  edge activity).
- Part B: motion signal (MutationObserver + matchMedia), ported dynamics, and an
  animator with an injected scheduler. Design correction against U19: the 5 s nudge
  now fires only on real change, so a steady map really rests.
- Harness findings, not product bugs:
  - an animator test model had a different session identity than the snapshot
    fixture (the identity guard refused it, correctly);
  - the Freeze e2e skipped the fake server's cursor ahead with seq 10,000;
  - the interval count forgot the nudge timer;
  - a doubly escaped bundle-key separator (cosmetic) was corrected.
- View tests (4) written after the code; 6 view mutants caught. Browser controls: the U20
  view fails 4/5 motion cases; 5 wiring mutants (motion never applied, freeze never
  applied, OS reduce ignored, setting not observed, dispose keeps refresh) caught.
- Shared e2e helpers moved to `e2e/telegram-flow-server.ts` (both flow specs).
- In-session review: MINOR, `currentLeader()` in render mutates pruning state (idempotent; kept). MINOR, the 1 Hz refresh re-renders the whole flow view (measured in part C).
- Full gate (first attempt): web build, fmt, clippy -D warnings, workspace tests 3,196 passed / 0 failed / 177 ignored in 175 blocks with 0 skip markers, five repository gates (headers 518 ok; anchors 453 ok; ledger 186 rows), tsc, `check:flow-study`, Vitest 1,999/116 files, complete Chromium suite 130/130, whitespace; source frozen.
