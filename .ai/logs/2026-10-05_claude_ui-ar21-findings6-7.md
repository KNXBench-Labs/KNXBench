# AR21 findings 6 and 7 (goal-ui owner, 2026-10-05)

Worktree `ui-flow-f67` on `origin/main` `942d2680` (web lock taken there; still
held after this commit for KL-37). Findings from TELEGRAM_FLOW_VISUALIZATION
§19 (finding 6) and §20 (finding 7, plus the stall observation).

## Change

- `flowModel.ts`: `FlowEvent.complete` (false when `apply` refused the sender
  or any recipient); a refused sender's fresh row becomes a lineless event
  without send times (`recordEvent` split out of `recordActivity`);
  `counters.eventsRecorded`.
- `flowAnimator.ts`: incomplete events count once as not (completely) drawn,
  their remaining lines still drawn, never as bundled; events recorded since
  the last sync but no longer in the ring count as not drawn (only while
  motion is on and the page is visible); the event baseline (`lastEventSeq`,
  recorded count) resets when `sync` gets a different model object.
- That last point is a separate defect found while designing finding 7:
  `useFlowFeed` builds a new model on a session change, and the animator kept
  the old session's highest `seq`, so a new session drew no pulses until its
  sequence numbers passed the old mark.
- Stall observation: no change by decision (§21).
- Docs: TELEGRAM_FLOW_VISUALIZATION §21, LEDGER `FLOW-01` owner correction,
  IMPLEMENTATION_STATUS.

## Evidence

- RED: 5 new animator cases failed on the old code (A, B, C, ring overflow,
  session change); 3 absence cases (fully drawn, already-drawn overflow,
  hidden) are pinned by mutants. 2 model cases (`complete`, no send times).
- Mutants 9/9 across the seven flow test files: partial-marked-complete,
  refused-sender-no-event, refused-sender-send-times, recorded-not-counted,
  complete-ignored, ring-overflow-hidden, counts-all-ring-drops,
  baseline-never-advances, no-session-reset; byte-exact restore.
- Gate (leases 7/8/9, inputs frozen, head `942d2680`): build, flow-study,
  theme-fixtures 0; Vitest 2,039 / 116; Chromium 134; flow specs ×3 42;
  check-layering, check-headers (155/155), check-anchors 554, check-ledger 188,
  check-corpus-gates 0; diff-check clean.

No hardware, no bus, no KNX socket.
