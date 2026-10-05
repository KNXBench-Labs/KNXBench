# U21 corrections (AR21 findings 1–3) — goal-ui owner, 2026-10-05

Branch/worktree `ui-u21-corrections` from `origin/main` `7a1dfd51` (the lock-take
commit). Web lock taken by claude-goal-ui-owner in `7a1dfd51`, released by the
delivering commit.

## Provenance

- User decision ("übernimm U21"): the alpha session had parked its U21
  corrections (worktree `u21-fix`, 9 uncommitted files, unchanged since 06:17,
  no process). Its state was backed up to owner scratch (`git diff b3c341d5`
  plus untracked copies, sha256 list) and applied here; `u21-fix` itself was
  not touched. Credit: per-node heat, `reheatAround`, moved-only redraw,
  `MIN_DRAWN_MOVE`, badge high-water reheat, the §7 scenarios and the marker
  devices in `flow-load.load.ts` came from that state. Its two intermediate
  measurement files (no marker lag) were superseded by fresh runs and stay in
  the scratch backup only.
- Added here: `nodeFootprint`/`separate` (hub readability as a hard constraint
  with bounded crowd push), drawing-area clamp for circles and names,
  wall-clock cooling, removal of a redundant spring guard (equivalent mutant),
  `e2e/telegram-flow-hub.e2e.ts`, measurements, docs.

## Findings → result

1. Local reheat: implemented (TELEGRAM_FLOW_VISUALIZATION §14). §7 load 60 s
   motion on: long tasks 61.9 s → 22.2 s, frame p50 167 → 33 ms, main thread
   0.999 → 0.924. Still saturated → recorded as envelope (§14, KL §154, guide).
   Steady-state trace: layout at rest (0 solver steps in 5 s at 50 s), Paint
   ~6.2 s of 8 s. A separate pulse layer cut Paint to ~4.0 s but not the
   main-thread total; not adopted. Removing pulses entirely did not bring the
   15 s run below 0.93 either (first layout dominates).
2. Hub readability: implemented; unit tests + Chromium hub e2e; screenshots
   `hub-before.png`/`hub-after.png` (assessed visually: before, all circles
   and texts piled on the hub; after, no circle/name/value overlap, names
   inside the frame; edge labels still cross some text).
3. DnD flake: already fixed on main by `0533230b`; gate repeats the spec.

## Evidence

- RED/negative controls: 12 of the new/changed unit tests fail with the
  `origin/main` flow sources (some by missing API, the rest behavioural); the
  hub e2e fails with them (Device 10/11 circles overlap).
- Mutants (scratch `mutants.py`, byte-exact restore verified): 12/12 killed:
  reheat-global, step-moves-cold, spring-from-cold, spring-to-cold,
  no-separation, footprint-ignores-badges, crowd-unbounded, clamp-centre-only,
  cooling-per-frame, badges-reheat-always, reclass-ignored, subpixel-redrawn.
  The first spring mutant survived as equivalent → guard removed.
- Measurements under the browser-fixture lease; host shared (load avg 16–21).
  The "after" Playwright invocation also collected two scratch probe files
  that were moved away mid-run (exit 1, "Cannot find module"); the six real
  scenarios passed and are the ones recorded.
- Gate (scratch `gate.sh`, leases 7/8/9 held, candidate = this tree on
  `7a1dfd51`, inputs hash unchanged start→end): web build 0, check:flow-study
  0, check:theme-fixtures 0, Vitest 2,013 / 116 files, Chromium full suite
  132 passed twice, repeated spec run (group-address-drag, telegram-flow,
  telegram-flow-motion, telegram-flow-hub, ×5) 80 passed, check-anchors 463 /
  281, check-ledger 186, check-headers 529 / 157 ceiling, `git diff --check`
  clean. No Rust source changed, so no cargo test/clippy run. After the gate
  one doc-only delta (mutant count 13 → 12 in §14) plus this log and the
  handover; anchors and diff-check rerun on that tree.

No KNX/bus contact, no hardware, all traffic intercepted (`page.route`).
