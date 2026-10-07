# 2026-10-04 — U20 part 1: flow wire validation and reducer

Agent: Claude, goal-ui.md owner session; Web lock `5dcd429e` (held for part 2).

- Read: AR20 contract (TELEGRAM_FLOW_VISUALIZATION §10, `flow.rs` DTO widths,
  `bus_routes.rs` snapshot route), `BusMonitorPanel.tsx` poll loop (component-
  local `useEffect`, cursor ref, pause, identity checks), U19 study model.
- Decision for part 2: the flow view lives inside the monitor panel and is fed
  by its existing poll loop (no second loop or tunnel). The reducer runs while a
  session is attached, so switching views loses nothing.
- RED first: wire 24, reducer 26 (one test had a wrong expected id; the GA
  arithmetic was checked before implementing), API 1.
- Mutants: 23. Two survived at first: `old-value-revived` (the test only checked
  visible badges; it now checks `slotCount`/`nextExpiryAt`) and
  `wire-direction-any` (the direction case was refused for its incomplete flags).
  Final sweep: see below.
- Review (in-session, before the gate): IMPORTANT — `Object.assign` in
  `ensureNode` kept stale `candidates`/`ambiguous` across generations; fixed
  RED-first. MINOR — `lastObservedAtMs = -Infinity` for unknown age, documented
  for U21.
- Guard update: `DiagnosticsCompanion.test.tsx` import inventory gains
  `flowWire.ts` with an isolation proof (imports nothing, no API call, no fetch).
- Final mutant sweep on the reviewed code: 23/23 caught, sources restored byte-exact.
- Full gate (first attempt): web build, fmt, clippy -D warnings, workspace tests 3,196 passed / 0 failed / 177 ignored in 175 blocks with 0 skip markers, five repository gates (headers 502 ok; anchors 452 ok; ledger 186 rows), tsc, `check:flow-study`, Vitest 1,936/110 files, complete Chromium suite 118/118, whitespace; source frozen.
