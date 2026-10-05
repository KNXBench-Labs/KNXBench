# 2026-10-04 — U19 telegram-flow study and AR20 handoff

Agent: Claude, goal-ui.md owner session. Web lock taken for U19 (`2231d87c`).
The plan itself was published earlier the same evening (`ca5fefa2`, aligned in
`3369dd1a`).

## What was built

- `apps/knx-web/e2e/flow-study/`: `model.ts` (slots, high-water mark, window
  leader, capacity, pulse bundling), `layout.ts` (bounded O(n + e) layout with
  area scale and cooling), `synthetic.ts` (deterministic installation and
  stream), `engine.ts` (one rAF loop, SVG or Canvas edges, measurements).
  `e2e/flow-study-fixture.*` is the React shell (synthetic notice, Freeze
  toggle, keyboard device list, Inspector, counters without `aria-live`).
- `e2e/flow-study.e2e.ts` (normal suite, 4 cases); `e2e/flow-study.study.ts`
  with `playwright.study.config.ts` (explicit measurement run);
  `tsconfig.flow-study.json` and `npm run check:flow-study`.

## Order of work and findings

1. RED first for the model (11), then layout (4), the synthetic stream, the
   slot index and the spread (3 more RED), and cooling (2 more RED).
2. The first browser screenshot showed a packed centre and one-line badges;
   fixed with area-scaled distances, stacked badges and a label halo.
3. The first measurement run used an off-by-one scheduler (events due one
   period late; a constant ~257 ms lag in the small slice). It was fixed
   test-first, and that run is discarded.
4. Measurements show the bottleneck is moving geometry. Canvas was built and
   measured as the alternative and lost. A frozen-geometry scenario confirmed
   the hypothesis (16.7 ms per frame at full target load).
5. Load from other sessions (`load1` 15–41 on 16 threads, the Hermes UI at
   about 860 % CPU) persisted even with all leases held, so it is recorded per
   scenario.
6. Mutation: 11 guards. `read-sets-values` first survived as equivalent (reads
   in the tests carried no value). The contract forbids a read value
   explicitly, so a test with a value-carrying read was added and the mutant
   is now caught.

## Evidence

- Vitest `e2e/flow-study`: 23 passed. Chromium `flow-study.e2e.ts`: 4 passed.
  `check:flow-study` type-checks for real (a planted error was reported).
- Screenshots and `measurements.json` in
  `docs/design/2026-10-04-telegram-flow-u19/`; numbers and decisions in
  `docs/TELEGRAM_FLOW_VISUALIZATION.md` §9; ADR-0077 addendum.
- Full gate (first attempt): web build, fmt, clippy -D warnings, workspace tests 3,178 passed / 0 failed / 177 ignored in 174 blocks with 0 skip markers, five repository gates (headers 485 ok; anchors 450 ok; ledger 186 rows), tsc, `check:flow-study`, Vitest 1,858/104 files, complete Chromium suite 112/112, whitespace; source frozen.
