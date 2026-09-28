# 2026-09-28 — claude-cloud — CT-1: project-diff web panel shows entities and values

Branch: `claude/fervent-mayer-ybc4h1`. Scope: `apps/knx-web` plus docs.

## Environment

Report at start: rustc 1.98.0, node 22, webkit2gtk-4.1 present (setup ran
via `session-start-fallback`), `OriginalData/` absent, identity
KNXBench <github@knxbench.com>. Nothing MISSING. Session runs as uid 0.

## Changed

- `apps/knx-web/src/projectDiffView.ts` (new): pure projection of an
  installation diff into per-table rows (status, natural-key label, name,
  match kind, fieldChanges, ambiguity counts, nested device tables).
- `apps/knx-web/src/ProjectDiffDetails.tsx` (new): collapsed native-button
  disclosures per non-empty table, Field/Before/After tables, status as
  word + symbol (colour only reinforces), 50-row paging with "Show more"
  that moves focus to the first new row, project/installation info tables.
- `ProjectDiffPanel.tsx`: mounts the details below the unchanged summary,
  remounts per comparison (new report starts collapsed).
- `messages/en.ts`, `messages/de.ts`: 16 new keys each. `styles.css`.
- Tests: `ProjectDiffPanel.test.tsx` 9 → 19; new `projectDiffView.test.ts` (3).
- Docs: KNOWN_LIMITATIONS §59 lifted, §60 rewritten to what remains (old
  anchors kept); IMPLEMENTATION_STATUS entry; manual implementation-status
  rows, known-issues entry, user guide 08 "Comparing in the application".

## Gates

| Gate | Result |
| --- | --- |
| `cargo fmt --all --check` | exit 0 |
| `cargo clippy --workspace --all-targets -- -D warnings` | exit 0 (knx-desktop included) |
| `cargo test --workspace -j 2` (`--no-fail-fast` rerun) | exit 101: 2106 passed, 1 failed, 115 ignored |
| `xtask check-layering` | exit 0 |
| `xtask check-headers` | `headers ok` (after shortening one header line) |
| `xtask check-anchors` | exit 0, 389 links |
| `xtask check-corpus-gates` | exit 0 |
| `npm test` (apps/knx-web) | exit 0, 68 files, 1069 tests |
| `npm run build` | exit 0 (existing chunk-size warning) |
| `git diff --check origin/main...HEAD` | exit 0 |

The one Rust failure is
`knx-cli::cli_group_address_csv::ga_import_of_a_real_change_against_a_readonly_store_reports_the_save_error`:
it makes the store read-only and expects the save to fail, but the cloud
session runs as root, which ignores the read-only bit, so the save
succeeds. This branch touches no Rust code; the failure is environmental.
Not changed here (out of scope); the local merge gate (non-root) should
pass it. Corpus tests report as ignored, as expected.

## Open

- No Playwright/browser run of the panel and no screen-reader check.
  happy-dom does not synthesize Enter/Space activation; the keyboard test
  reproduces the browser rule in a helper.
- §60 remainder: paging, not virtualisation; no search/filter; no jump
  from a row to the entity in the explorer; field identifiers untranslated.
- Cloud sessions run as root: consider making that CLI test skip or
  assert differently under uid 0 (separate task).
