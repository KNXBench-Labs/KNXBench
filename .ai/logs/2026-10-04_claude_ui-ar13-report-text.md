# 2026-10-04 — AR13 hand-over: debug-report telegram warning

Agent: Claude, goal-ui.md owner session. Web lock taken for this item
(`a89224e4`).

- Source of truth: `report_markdown` in `apps/knx-server/src/debug_report.rs`.
  `bus-telegrams.json` is not redacted and keeps individual and group
  addresses, group-address names, every telegram's values (text values
  included) and their timestamps, which together can show when the
  installation was in use.
- Change: `debugReport.privacyTelegrams` en/de rewritten to say the same.
- Evidence: `DebugReportButton.test.tsx` gained a content test per language
  (addresses, names, every value, text values, timestamp). It failed on the
  old wording in both languages (RED), then passed. The existing display test
  still checks that the dialog shows the catalogue string.
- Full gate (first attempt): web build, fmt, clippy -D warnings, workspace tests 3,155 passed / 0 failed / 177 ignored in 169 blocks with 0 skip markers, five repository gates (headers 469 ok; anchors 442 ok; ledger 185 rows), tsc, Vitest 1,835/101 files, complete intercepted Chromium suite 108/108, whitespace; source frozen.
- With this, every item of the 2026-10-04 UI owner handoff (MODEL-03, DATA-03,
  MODEL-04, MODEL-01, MODEL-02, UX-01, AR13 text) is delivered.
