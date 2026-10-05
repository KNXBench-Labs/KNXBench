# AR21 acceptance (Claude, 2026-10-05)

## Scope
Rerun of AR21 findings 6/7 (UI fix `6fa10eb8`) and closure of AR21's remaining
checkboxes: envelope reconciliation, documentation, integrated gates.

## Evidence
- Probes A, B, D, F, G (temporary Vitest cases): counts as specified in
  TELEGRAM_FLOW_VISUALIZATION §22.
- Own mutants: incomplete events ignored, ring overflow ignored, sequence
  baseline not reset, refused-sender event not recorded — each fails flow tests.
- Gates: Web build; Web Vitest 2,044/2,044; flow Vitest 111/111; flow e2e ×3
  42/42; drag e2e ×5 10/10 (network-less, short TMPDIR); `cargo fmt`,
  workspace Clippy `-D warnings`, workspace tests 3,306/0/177 in 187 blocks;
  check-layering/headers/anchors/ledger/corpus-gates.
- Load (production build, Chromium 152): 900 s long session and 720 s edge
  growth to 502 nodes / 2,482 lines (numbers in §22).

## Decision
`FLOW-01` DONE on the recorded envelope: motion for small/medium maps; Motion
Off for several hundred nodes or the §7 load; Chromium only. Heap plateau over
hours not shown. Receipt for AR18: §13–§22 plus this log.

## Pitfall
Playwright/Chromium needs a short `TMPDIR`; the singleton socket path limit
fails every browser test with "Socket path too long" otherwise.
