# P5 — OPEN_WORK synchronisation (Claude, 17 October 2026)

Package: `docs/goals/open-work-2026-10-17/p5-open-work-docs-sync.md`, docs only.
Base: `origin/main` `d1c3623d`; claim `f61465af`; docs commit `7580601c`.

## Re-check on fresh main (all four gaps still present)

- KL-185 section said branches move "via `refs/rewritten/*`".
- No KL-186 section; no object-function entry in section 2; no link to the
  package set.
- `docs/goals/` has no index file, and neither `docs/README.md` nor
  `docs/DOCUMENTATION.md` lists goals, so scope item 4 adds the link to
  `OPEN_WORK.md` only (no index created: "where an index exists").

## Facts verified before writing

- `99dd6e65` "Put function choices where the object lives", parent `1ee19383`,
  25 files; equals `refs/kl185-rewritten/heads/fix/object-function-dropdown-20261017`;
  not an ancestor of `origin/main`; no object-function branch on the remote.
- `docs/history/COMMIT_MAP_2026-10-17.txt` exists; 18 refs under
  `refs/kl185-rewritten/`.
- Ledger *Counts*: snapshot 180 rows (DONE=47, BLOCKED_EXTERNAL=2,
  ACCEPTED_BOUNDARY=118, LATER=13), post-snapshot 28 rows (DONE=19,
  ACCEPTED_BOUNDARY=6, LATER=3). OPEN_WORK's "How this was checked" carried
  two older censuses (41/119/20 + 9/2, and the 191-ID recount); they are now
  labelled historical and the current Counts are quoted above them. The
  recount's date is not stated: history says 8 October, its own text says
  "after import expansion" (10 October); no date was guessed.

## Self-review (before the final gate)

- MINOR: an "8 October" date on the old recount was not well supported; removed.
- MINOR: awkward P1 sentence; rewritten.

## Gate (final tree, frozen=1, own CARGO_TARGET_DIR)

check-anchors, check-ledger (208 rows), check-headers, check-layering,
check_documentation (0 errors), CI tool tests limitations_catalogue 7,
documentation_history 15, check_local_info 10, web_gate 14, browser_tmp 21
(all OK), check_local_info tree PASS. 12/12 exit 0.

First run: `test_browser_tmp` red only because the fresh worktree had no
`apps/knx-web/node_modules` (`Cannot find module …@playwright/test/cli.js`);
CI installs them first. `npm ci --prefer-offline` from the lockfile, then the
whole gate re-ran green. Environment, not content.

Not touched: IMPLEMENTATION_STATUS (no implementation changed), ledger rows,
KL dossiers, ADRs.
