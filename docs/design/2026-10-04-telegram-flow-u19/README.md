# Telegram flow — U19 synthetic study (2026-10-04)

Evidence for [TELEGRAM_FLOW_VISUALIZATION §9](../../TELEGRAM_FLOW_VISUALIZATION.md#9-u19-resolution-goal-ui-owner-2026-10-04).
Every picture shows **synthetic** telegrams from `apps/knx-web/e2e/flow-study/`
(addresses `9.x.y`), not a project or live bus data.

| File | Shows |
| --- | --- |
| `slice-graphite.png` | Dark theme, motion on: curved directed edges, active-edge address labels with halo, stacked value badges (`*` = configured target, not confirmed), dashed unresolved group nodes, leader 9.1.1 selected, Inspector |
| `slice-porcelain.png` | The same in a light theme: resting lines, pulses and badges stay legible; badge green is weak on white (U21 contrast check) |
| `slice-graphite-motion-off.png` | Motion Off: no pulses, no layout motion; values keep arriving |
| `slice-graphite-renderer-canvas.png` | Canvas variant of edges and pulses (evaluated, not chosen) |
| `measurements.json` | Per-scenario frame interval, script, draw, layout, value lag, edge writes, model size, refusals, heap and `load1` |

Known visual residue for U21: badges and labels of the activity hub's
neighbours still collide around the leader.

Captured with Chromium 152 (headless, Playwright) on a Ryzen 7 5800X while
other sessions were running; the numbers are upper bounds under contention.
Reproduce with `npx playwright test -c playwright.study.config.ts` in
`apps/knx-web` (`FLOW_STUDY_OUT` selects the output directory).
