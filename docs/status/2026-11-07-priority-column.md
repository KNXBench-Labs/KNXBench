# Optional priority column in the object table — 7 November 2026

Scope: WP-06, owner decision of 7 November 2026: an optional read-only column
with the effective value and a short source tag, sortable and filterable, off
by default and toggled in the table; editing stays in the object editor.
Contract: [communication-object table](../COMMUNICATION_OBJECT_TABLE.md#presentation-and-derived-data).

## Delivered

- Toolbar toggle *Priority column* (`aria-pressed`), off by default, remembered
  through the existing settings store (`comTablePriorityColumn`). The table had
  no column chooser before; this toggle is the whole mechanism.
- Column cell: effective token, short source tag (*yours*/*saved*/*product*,
  de *eigene*/*gespeichert*/*Produkt*), full source or unresolved reason as the
  title (shared wording with the object editor row). Unresolved, unknown or
  missing server states show `—`, never a token.
- Sort Low → High → Alert, unresolved last in either direction; Priority filter
  (all, each token, unresolved) shown with the column and counted as a filter.
  Hiding the column drops its filter and sort. Group headings and editor rows
  span the visible columns.
- No API, storage, projection or programming change.

## Executed checks (author, before review)

- Web: `npm ci`, `tsc -b`, `npm run build`, Vitest 330 files / 5,324 tests
  (four new table tests, one new view test).
- Browser (production build, loopback-only namespace): `communication` 108/108,
  `table-columns` 180/180, including the new spec (en/de at 1440/400 px:
  column off by default, toggle, values and source tags, unresolved title,
  sort, filter, header/cell alignment, no page overflow, exactly two settings
  writes and no project change). Its first run failed 4/4 only because the
  shared fixture did not answer `GET /api/version`, which the app re-reads after
  a settings change; the fixture now answers it.
- Documentation, local-info, headers, anchors, `git diff --check`.

## Independent review and corrections

Review of `3017c51bb` in a separate source copy: **APPROVE**, two Minor and
five Nit findings; 18 reviewer mutants, 16 killed. Browser in the reviewer's
copy: `communication` 108/108, `table-columns` 180/180, focused default specs
156/156, the full default lane 1124/1124, a temporary screenshot spec 11/11
(24 captures, grouped/folder/flat, en/de, 1440/400, light/dark).

Corrections in the follow-up commit:

- The priority filter and sort are dropped by an effect whenever the column is
  hidden, also when another window or a settings refresh hides it (the review
  reproduced a lingering filter); a test hides it through the settings store.
- New folder-view test for the heading span with the column (survivor M07).
- The unresolved dash is `aria-hidden` with a visually hidden `.sr-only` label
  instead of an `aria-label` on a plain span (survivor M16).
- German wording aligned with the object editor (*Nicht aufgelöst*, *von dir*);
  the cell description tolerates a `null` view; table contract wording for the
  folder view column count and the filter list.
- Re-run mutants: M07, M16 and removal of the hide effect are killed.
- The first browser run of these corrections failed 2/2 at 400 px in both
  configurations: the absolutely positioned `.sr-only` label escaped the
  scrolling table and widened the page by 262 px. The priority cell is now
  `position: relative`, which keeps the label inside the table's own scroll.
  Re-run after the fix: Vitest 330 files / 5,326 tests, `communication`
  108/108, `table-columns` 180/180.

Known and accepted: the full explanation is a hover title (the object editor
shows it to keyboard and touch users); in the ~740 px inspector the extra column
needs the table's existing horizontal scroll.

## Acceptance and publication

Feature `3017c51bb` plus review fixes `ffa3a0394`, merged main-first as
`b6b0d29b6` on `6038b9427` (tree identical to the branch head). The gate was
started only after a parallel session's gate had finished, to avoid load-driven
timeouts. The change-scoped package gate selected 11 stages for this web-only
change; all exited 0 with valid proofs, inputs frozen, no user data touched or
unchecked. Web gate: 43 stages with 1,124 browser cases (1,055 functional, 69
exclusive). No Rust, bindings or corpus stage was selected. Pushed after the gate
with `origin/main` unchanged; local, tracking and live `main` read back as
`b6b0d29b6`.

## Limits

- Display of the server's effective priority only; table views outside the
  device editor are unchanged; no report or CSV column.
- The preference is global (all devices), not per device or per project.
