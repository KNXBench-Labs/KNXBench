# 2026-10-04 — KL-60: virtualised, filterable diff tables

Agent: Claude, goal-ui.md owner session; Web lock taken in `ccdb9038`, released in this delivery.

- Decision (user, AR11): replace 50-row paging in the diff view with
  virtualised tables plus search/filter; backend unchanged.
- Design: up to 20 entries a table stays a plain list. Above that it gets
  text filter (key and name), status toggles, a `role="status"` count and a
  scroll viewport. `virtualWindow(heights, scrollTop, viewport, overscan)`
  is pure, and `before + rendered + after` always equals the total.
- Found only in Chromium (happy-dom has no layout):
  1. End did not reach the end. The smooth native jump stopped short once
     the tall changed rows were measured. Fix: End/Home handled on the
     focused viewport (instant jump plus an end intent), the list stays
     pinned at its end, and growth of rows above the view shifts
     `scrollTop`.
  2. My own filter test expected all 11 tall matches in the DOM, but the
     window correctly held 8. The test now checks the count, `aria-setsize`
     and content.
- Found by the memory test: `useMemo(new Map, [report])` had no effect,
  because the `Disclosure` instances keep their own state. The panel's
  `key={generation}` remount is the real mechanism; the memory is now
  per-mount `useState` and the test checks that contract.
- Controls: e2e 4/4 red against the previous list. Mutants caught: no-clamp,
  no-overscan, filter-key-only, filter-case, status-ignored, never-windowed
  (Vitest); estimates-only, no-end-key, no-end-pin, no-shift,
  no-gapless-rows (Chromium); memory-not-written (Vitest). The
  memory-across-reports mutant survived because the code it mutated was
  ineffective (see above); that code is gone, and `ProjectDiffDetails.test.tsx`
  pins the per-mount contract instead.
- In-session review of the complete diff (before fixes):
  - IMPORTANT: `ProjectDiffPanel.tsx:211-215` closes the report on any
    Escape, so Escape in a typed filter discarded the comparison. Fixed
    innermost-first: a non-empty filter is cleared and the event stops.
    RED first; two mutants caught (never clears, always swallows).
  - MINOR (documented, §60): focus inside a row lost when it unmounts; Tab
    walks rendered rows only.
  - MINOR (accepted): each long table is a `role="region"` landmark, needed
    to name the focusable scroll container.
- Gate attempt 1 was aborted by the owner after web-build/fmt (both 0),
  because the review fix changes source. Log kept as
  `gate-attempt1-aborted-review.log` in scratch.
- Full gate attempt 3 on the tree rebased over AR20 (`85bfab88`) and `1a8acffa`: web build, fmt, clippy -D warnings, workspace tests 3,196 passed / 0 failed / 177 ignored in 175 blocks with 0 skip markers, five repository gates (headers 497 ok; anchors 452 ok; ledger 186 rows), tsc, `check:flow-study`, Vitest 1,885/108 files, complete Chromium suite 118/118, whitespace; source frozen. Attempts 1 and 2 were aborted by the owner (review fix; upstream rebase), logs kept.
