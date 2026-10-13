# 2026-10-13 — Claude — ux-feedback port (parked-work adoption, package 2)

Origin: **Codex**, `feature/ux-feedback-20261010` (worktree `ux-feedback-20261010`,
13 local commits up to `88a776c2`, "UX feedback session 1", 10–11 October). The
original worktree was only read (no edit, stash or clean). Port in worktree
`ux-feedback-port-20261013`, branch `feature/ux-feedback-port-20261013`, from fresh
`origin/main` `157a02a8` plus the lock-take entry `71b7fc87`; rebased twice while
main moved (KL-170 base-image warning, tester download reports); the accepted gate
ran on `97219b58` over base `9e1c1574`.

## What landed, per Codex commit

| Codex commit | Package | On main as | Notes |
| --- | --- | --- | --- |
| `219e4c3e` | AP1 notifications | `38ed264d` (hand-port) | Only what main lacked, see below |
| `22a4148d` | AP3 / KL-43 motion categories | `54881f72` | ADR 0109/0112 → **ADR-0122** (main's 0120 and 0121 went to KL-170 and the tester reports during the port); `update` toasts join the Toasts category |
| `92fd73a7`, `8ad22445`, `55df730f`, `fc712028` | AP2 Flow image export, camera | `516defd6` | Specs run without `FLOW_EXPORT_OUTPUT`; config builds its own bundle on port 4835 |
| `86c3167d` | AP4 inventory cards | — | Dropped (owner decision 2026-10-12; main's Project status cards) |
| `4dc9e98d` | AP1 archive z-index | — | The archive itself stayed out |
| `69012c02`, `77b9b66f` | AP5 achievements, narrow layout | `17c95a1a` | Unchanged apart from conflict resolution |
| `59d3ae54`, `6714e82b`, `88a776c2` | Codex handover/acceptance records | — | Superseded by this port's gate; their logs are kept verbatim |
| — | Review fixes | `f45b2179`, `97219b58` | StrictMode clock loss, clipped shadows; stepped-clock GIF spec and `tooSlow` unit tests |

AP1, left out because main already has it (owner rule "leave out anything already on
main"): the 100-entry "Recent errors" archive with its File-menu and toast buttons
(main keeps every UI notification in the session log, 1000 entries, with an
open-in-log action per card), the Dashboard count links (main's Project status
cards), the log row timestamp (main's entry details show it). AP1 ported: visible-slot
timers, hover/focus/hidden-tab pause, bounded queue with counts, errors no longer
replace each other or vanish on selection/import, the child-animation guard, and
Explorer footer counts through main's `openImportFindings`.

Not ported: `docs/contracts/ux-feedback-ownership.md` (file ownership between two
2026-10-10 sessions that no longer exist), `error-archive-presentation.md` and
`project-inventory-cards.md` (their features stayed out), the branch's local
acceptance status page and evidence JSON (they certify the old base `7d39628f`).

## Found on the way

- **Codex AP1 queue overflow** retired cards that were about to be shown: under a
  burst of pushes the `visible` flag lags one effect. Retiree now chosen by queue
  position (red/green test).
- **StrictMode** (review, IMPORTANT): the queue's mount-effect cleanup deleted every
  clock, so App's startup toast never expired and its × did nothing in dev builds and
  the fixture suite. Clocks now survive the cleanup and are re-armed (red/green).
- **Shadows**: the scrolling stack clipped every card's shadow; the corner offset is
  now padding inside the stack.
- **KL-43 evidence gap**: the branch's browser test checked only `<html>` attributes.
  The spec now reads the computed animation duration of real cards (Off 0s,
  Subtle 0.12s); mutant M14 proves it.
- **GIF spec vs. a busy host**: the first full gate's ×3 repeat failed 4/12 while
  another session's cargo build held the load average at 68 on 16 cores. The capture
  correctly refused (`tooSlow`, no skipped frames); that guard had no test. The spec
  now steps a paused page clock one frame at a time (real rasterization/encoding;
  15/15 under 32 extra busy processes), and two unit tests pin the guard. The first
  gate was stopped and its verdict voided. A second, fully green gate (`baa21ea7`,
  corpus 144/144) was superseded when main gained Rust runtime inputs and took
  ADR-0121; the accepted gate is a complete third run on the final candidate.
- Main's Settings panel now asks `GET /api/legacy-password`; the strict fixture
  answers it.

## Lessons

- A queue that decides by a flag refreshed in an effect is wrong during bursts;
  decide by position or by state computed in the same update.
- Effect cleanups run on StrictMode's simulated unmount. Never discard state there
  that an effect of the first pass may already have created; stop timers, keep data,
  re-arm on mount.
- A spec that needs an env var to run cannot live in a suite that runs every
  `*.e2e.ts`; default to `test.info().outputPath()`.
- A guard that refuses late real-time work needs a unit test from both sides and a
  stepped clock in the browser spec; otherwise a busy host turns correct behaviour
  into a flaky failure.

## Verification (gate on `97219b58`, base `9e1c1574`)

Build; `tsc` for all 13 tsconfigs; Vitest 3242/0 in 230 files; full fixture Chromium
433/0; 18 production-preview/intercepted configs 215/0 (ux-feedback 52); the five
ported or touched specs ×3 102/102; fmt, Clippy; Rust 4269/0 (197 ignored);
real-server native evidence 12/12 and naming 4/4; tools 67 (1 skipped);
check-layering/headers/anchors/ledger/corpus-gates; `diff --check`; inputs frozen.
Private corpus: 144/144 in 31 targets (`tools/run_corpus_tests.py`, offline). Mutants: 17, all fail by name (M5 needed its follow-up M5b
against the hover-plus-focus test). Evidence:
[`docs/evidence/ux-feedback-port-2026-10-13.json`](../../docs/evidence/ux-feedback-port-2026-10-13.json).
Self-review only (no subagents, owner rule 2026-09-28); native WebKitGTK not run.
The Codex branch's private screenshots/evidence archive was copied, ignored, to the
root checkout's `.ai/logs/2026-10-11_codex_ux-feedback-evidence/`.

## Addendum: retirement of the parked worktrees (owner go, 2026-10-13)

The port had committed only the Codex logs the branch itself tracked (plus AP1's).
Three more Codex logs (`2026-10-10_codex_ux-feedback.md`, `2026-10-10_codex_flow-images.md`,
`2026-10-11_codex_ux-feedback-ap5.md`) existed only as ignored files in the parked
worktree; they went to main verbatim in `7adfec2a` before the worktree was removed.
Both parked worktrees' ignored files are archived in
`KNXBench.backups/2026-10-13-parked-adoption-retire/` (sha256 manifest); the local
branches stay.
