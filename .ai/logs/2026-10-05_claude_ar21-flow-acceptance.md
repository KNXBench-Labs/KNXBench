# 2026-10-05 — Claude (alpha) — AR21 telegram-flow acceptance review

Trigger: cron watch `knxbench-ar21-watch` saw U21 delivered on origin/main
(`fb40a99a`, Web lock released). Worktree `ar21-accept` from `fb40a99a`,
`CARGO_TARGET_DIR` in the profile scratch. No Web file changed, no lock taken.

## Receipt

U19 `51a6004e`, AR20 `85bfab88`, U20 `4525c36e` + `dc298b78`, U21 `9d432d17` +
`deb6813a` + `fb40a99a`. Owner evidence: `telegram-flow.e2e.ts` (7),
`telegram-flow-motion.e2e.ts` (6), load study `flow-load.load.ts`, mutants per
the U20/U21 logs (not rerun here).

## What I ran (exit codes read from saved `.rc` files)

| Gate | Result |
| --- | --- |
| `cargo fmt --all -- --check` | 0 |
| `cargo clippy --workspace --all-targets --exclude knx-desktop -- -D warnings` | 0 |
| `cargo test --workspace --exclude knx-desktop` | 0; 3,184 passed / 0 failed / 177 ignored, 172 result blocks; `http_bus_flow` 9 ran |
| `npm run build` (tsc + vite) | 0 |
| `npm run check:flow-study` | 0 |
| `npx tsc --noEmit -p .` | 0 |
| `npx vitest run` | 0; 116 files, 2,001 tests |
| `npx playwright test` (full) | 1; 130 passed, 1 failed (`group-address-drag.e2e.ts:60`) |
| flow e2e + drag, `--repeat-each=3` | 43 passed, 2 failed (`group-address-drag.e2e.ts:69` twice); flow 39/39 |
| `group-address-drag.e2e.ts --repeat-each=5` alone | 10/10 |

The full suite ran while `cargo test` was compiling; the repeated mixed run
did not, and still failed, so load alone does not explain it.

## Acceptance matrix (§7 → evidence)

- Sources/targets, ambiguous and duplicate addresses, inactive objects, several
  groups on one raw address, truncation: `flowModel.test.ts` (lines 73–178).
- Write/Read/Response, undecodable payload, unknown age, SessionClosed, legacy
  and malformed rows: `flowModel.test.ts`, `flowWire.test.ts`; individual
  destinations are not admitted server-side (`http_bus_flow.rs`).
- Slots, three badges, newest sender wins, no cross-slot overwrite, 7 s
  boundary, no revival of old rows: `flowModel.test.ts` 252–314; browser
  `telegram-flow.e2e.ts` (expiry, Read).
- Generation, stale reply, server restart, reset: `flowModel.test.ts` 128–200,
  `flowFeed.test.tsx`; browser generation and session-change cases.
- Leader, tie, window expiry, no traffic: `flowModel.test.ts` 325–359.
- Freeze, motion off/OS reduce mid-flight, hidden tab, unmount: `flowAnimator.test.ts`,
  `flowMotion.test.tsx`, `telegram-flow-motion.e2e.ts`.
- Bundling and over-capacity counts shown: `flowAnimator.test.ts` 136–145,
  `TelegramFlowView.test.tsx` 187; capture loss stays the monitor's
  `droppedBefore` notice (`BusMonitorPanel.tsx`).
- Theme change, keyboard, Inspector, no announcements: `telegram-flow.e2e.ts`,
  `TelegramFlowView.test.tsx`.
- Productive integration: `BusMonitorPanel.tsx` lines 205, 336, 486, 539
  (`flowFeed.admit/reset`); `TelegramFlowView` has no API call.

## Load probe

Copied `e2e/flow-load.load.ts` to a temporary `e2e/zz-ar21-probe.load.ts`
(deleted afterwards), changed only the output directory (scratch) and the
scenario list to `{devices: 500, groups: 1250, ratePerSecond: 1000, seconds:
15}` with motion on and off; ran `npx playwright test -c
playwright.load.config.ts e2e/zz-ar21-probe.load.ts`. Results in
TELEGRAM_FLOW_VISUALIZATION §13: motion on 0.98 main thread, 111 long tasks,
frames p50 50 ms / p95 133 ms; motion off 0.136. Only 998 edges were drawn:
the study's member formula `(g*7+3) % devices` repeats every 500 groups.
Markers were not detected in either run; I did not chase the cause.

## Findings returned to the UI owner

1. IMPORTANT: global reheat instead of the §9.3 local reheat; deviation not
   recorded; no production measurement at the §7 load.
2. MINOR: badge-height-aware separation (§9.3) not done or recorded.
3. MINOR: `group-address-drag.e2e.ts` flaky.

Full text in TELEGRAM_FLOW_VISUALIZATION §13. `FLOW-01` stays `IN_PROGRESS`;
the cron watch stays active for the correction.
