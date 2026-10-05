# AR21 finding 4 — reduced-rendering counts (goal-ui owner, 2026-10-05)

Worktree `ui-u21-f4` from `origin/main` `2f6f20b0` (lock-take commit after the
AR21 rerun `6b802934`). Web lock taken in `2f6f20b0`, released by the
delivering commit.

## Finding (TELEGRAM_FLOW_VISUALIZATION §15)

`queuePulses` added `events.length` to `coalescedEvents` for each bundled batch
(including telegrams whose pulses were then refused) and `pulse.count` — one
per recipient line — to `overCapacityEvents`. Alpha measured "21145 … bundled,
39801 without a pulse" after 21,145 telegrams.

## Change

- Each pulse remembers the telegrams it stands for; a refused pulse marks all
  of them incomplete. `overCapacityEvents += incomplete telegrams`;
  `coalescedEvents += telegrams of the bundled batch − incomplete`. Disjoint,
  sum ≤ batch size. Events always have ≥ 1 target (group-box fallback in
  `flowModel.resolve`), so a `to.length > 0` filter was dropped as redundant
  (found while trying to test it: a sender-only group still yields a box line).
- Note wording en/de: "{dropped} not at all or only in part" / "gar nicht oder
  nur teilweise".

## Evidence

- RED: the two new animator tests failed on the old code (100 vs 80, 20 vs
  10); the wording test failed with the old messages.
- Mutants (scratch `f4/mutants.py`, byte-exact restore): revert-old-counting,
  bundle-members-untracked, coalesced-includes-refused, unbundled-not-counted,
  missing-not-counted — 5/5 killed; old wording killed separately.
- Gate (scratch `f4/gate.sh`, leases 7/8/9, inputs hash unchanged): web build
  0, check:flow-study 0, check:theme-fixtures 0, Vitest 2,016 / 116 files,
  Chromium full suite 132 passed, flow specs ×3 39 passed, check-anchors 536 /
  284, check-ledger 186, check-headers 531 / 157 ceiling, `git diff --check`
  clean. No Rust source changed.

No KNX/bus contact; intercepted traffic only.
