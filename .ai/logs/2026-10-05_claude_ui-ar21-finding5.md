# AR21 finding 5 — telegrams without a line (goal-ui owner, 2026-10-05)

Worktree `ui-flow-f5`, rebased onto `origin/main` `03609b60` (theme package).
Web lock held since `a33b4073` (splitter fix → theme package → this), released
by this commit.

## Finding (TELEGRAM_FLOW_VISUALIZATION §17)

At the model's node limit every target of a telegram can be refused while its
sender is kept; `recordActivity` then records an event with `to: []`. In a
bundled batch `queuePulses` counted it as "drawn as bundled pulses". Alpha's
probe: `maxNodes: 1`, 30 telegrams to a group with two receivers → 0 pulses,
`coalescedEvents` 30, `refusedNodes` 60. §16 had claimed "no third case".

## Change

`queuePulses`: an event without targets is marked incomplete (counted as not
drawn) in bundled and unbundled batches and sets `metrics.reduced`, so the note
appears; the "every event has at least one target" comment is gone. §16's
sentence is annotated, §18 records the correction, the FLOW-01 ledger row and
IMPLEMENTATION_STATUS follow.

## Evidence

- RED: two new `flowAnimator.test.ts` cases at a one-node limit failed on the
  previous code (`coalescedEvents` 30 instead of 0; `overCapacityEvents` 0
  instead of 2).
- Mutants (byte-exact restore): lineless-ignored, lineless-not-reduced,
  lineless-not-counted, lineless-counted-bundled — 4/4 killed.
- Rebase onto `03609b60`: conflicts in IMPLEMENTATION_STATUS (both entries
  kept) and LEDGER (own FLOW-01 row + upstream KL-155 row; scripted check that
  upstream had not changed FLOW-01).
- Gate (leases 7/8/9, inputs frozen): build, flow-study, theme-fixtures 0;
  Vitest 2,004 / 116; Chromium 132; flow specs ×3 42; anchors 546 / 285;
  ledger 187; headers 533 / 157; diff-check clean.
- Before publishing, `origin/main` moved again (AR14B: `1fd1664a` Route Back,
  `365d9056` docs; Rust/docs only, no `apps/knx-web` file). Rebased; the same
  two conflicts resolved by script (own FLOW-01 row + upstream's newer KL-155
  row, checked that upstream had not changed FLOW-01). Docs checks rerun under
  leases 7/8: anchors 546, ledger 187, headers 533 / 157, diff-check clean.

No hardware, no real bus and no KNX socket were used.
