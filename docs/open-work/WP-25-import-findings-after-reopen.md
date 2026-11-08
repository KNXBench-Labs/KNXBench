# WP-25 — Import findings after reopening

Owner report and implementation Go: 2 November 2026. Delivered on main as
`3455fdb9c`; ships in Alpha 20.
[Acceptance and boundaries](../status/2026-11-02-wp25-import-findings.md).

## Reproduced cause

Before the fix, native opening read the original report from the loaded working
image to restore `ProjectTree.errors`/`warnings`, then reset the session log
without restoring `import:*` entries. Only a fresh import populated those entries.
The real native-store regression reproduced counters `(1, 4)` beside an empty
finding list. The persisted findings were not lost; the session-log projection was.

## Decision

Re-project stored primary error/warning findings on successful native open.
Reuse the existing severity-filtered log, search/export and explicit full-report
reader rather than add a second findings workspace, API or persistence layer.
Projection comes from the admitted recovered working image before replacement;
entries are appended only after replacement succeeds. A refused open cannot
replace the project or reset its log, and opening twice cannot duplicate findings.

`historical_import_log::project` reads the retained JSON without rewriting it.
It reuses manufacturer/retained-payload grouping and checks per-severity counts
against admitted counters before the log cap applies. Missing display fields say
"not recorded"; missing/non-integer/overflowing occurrence totals are withheld,
not measured zero. Count disagreement is explicitly reported instead of silently
publishing an incomplete projection.

## Reader-facing behaviour and limits

- EN/DE focused and general logs explain that these are historical findings
  restored at opening; timestamps are opening time, not invented import time.
- The focused log exposes the existing explicit full-report action, including
  when its 16,000-entry cap leaves fewer findings than the project counters.
- Native files without stored evidence remain explicitly unavailable; no missing
  legacy findings are reconstructed and zero does not claim a clean assessment.
- Informational opaque/source-observation notes are not replayed; the complete
  original report remains downloadable with unknown fields unchanged.
- Server diagnostic wording remains English; reader-facing scope/provenance and
  report controls are translated. The cap regression uses an error-only report.
- No source reimport, bus operation, release, new schema or compatibility claim.
