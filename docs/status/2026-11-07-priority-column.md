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

## Limits

- Display of the server's effective priority only; table views outside the
  device editor are unchanged; no report or CSV column.
- The preference is global (all devices), not per device or per project.
