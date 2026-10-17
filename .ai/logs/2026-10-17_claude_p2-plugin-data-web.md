# P2 — plug-in data adoption in the web app (Claude, 17 October 2026)

Package: `docs/goals/open-work-2026-10-17/p2-plugin-data-web.md`. Claim `6a8b681b`.
Worktree `<sources>/KNXBench-plugin-data-web-20261017`, branch
`feat/plugin-data-web-20261017`, base `6a8b681b`.

## Decisions

- ADR-0136 amended instead of a new ADR (0137 is taken by the parameter-grids
  branch): native `.knxdb` required; the way back (named version *Before plug-in
  data adoption*) is written before the adopted working state; undo stack carried
  unchanged; the server keeps the one shown plan (`plugin_data_plan`, like
  `device_download_plan`), apply takes only the token; refusals get a stable code.
- Two transactions (`change_history(Create)` then `persist_working`): if the
  second fails the extra version of the unchanged state remains; nothing is
  published. Accepted and documented rather than a new store API.
- Not done on purpose: modified indicator (follows the model), session-log line
  (the adoption record is the audit trail), Command/Undo integration.

## Findings while building

- Test-side deadlock: a `MutexGuard` temporary inside a struct literal lived
  across an `await` that locked the same mutex (`GET /api/project`). Fixed by one
  statement per lock; the first RED run hung for ten minutes before that.
- Outside-crate tests build `AppState` with `..Default::default()`, so every new
  field must be `pub` with a public type (`ShownAdoption`, re-exported, opaque).
  Four tests use a full literal and needed the field too.
- Browser screenshots caught what geometry checks missed: the shared
  `.search-panel input { width: 100% }` exempts checkboxes only, so a radio took
  the full width and pushed its label out of the list's own scroller; and a flex
  wrapper squeezed a refused pair's name into a narrow column. Both now have
  browser assertions (inner scrollers may not overflow; pair text inside the
  list; refused name above its reason).
- Keyboard: a step's button disables itself while busy, which dropped focus to
  `<body>` and out of the dialog's Escape handling. Each finished step now moves
  focus to its result.
- Review finding: a revision change while the source listing was in flight left
  the dialog busy forever; fixed test-first.

## Evidence (focused, before the package gate)

App `plugin_data_adoption` 19/19; HTTP `http_plugin_data_adoption` 12/12; mutants
4/4 killed (incarnation check, way-back version, durable persist, cancel);
Vitest full 3,680/3,680 (before the last two dialog cases), dialog + module
17/17; browser spec 5/5; `tsc`; `check_documentation` 0 errors. Package gate
results are in the handover entry.
