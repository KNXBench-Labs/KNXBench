# 2026-09-28 — claude-cloud — CT-2: documentation export preview and section selection

Branch: `claude/task-ct-2-20lv54`. Scope: `apps/knx-web` plus docs.

## Environment

First start: `webkit2gtk-4.1: MISSING` (apt failed: `dpkg was interrupted,
you must manually run 'dpkg --configure -a'`). Reported and stopped; the
user chose to repair setup. On resume the report showed `apt=ok`,
webkit2gtk-4.1 present, so all gates ran with `knx-desktop` included.
`OriginalData/` absent. Identity KNXBench <github@knxbench.com>.

## Changed

- `src/documentationOptions.ts` (new): the five section names in server
  document order, `documentationOptionsFor(selected, uiLanguage)` building
  the single `{ sections, language }` object, `documentationLanguageFor`
  (`de` → `de`, otherwise `en`).
- `src/api.ts`: `previewDocumentation(options)`; `exportDocumentation(path,
  options?)` forwards options.
- `src/DocumentationDialog.tsx` (new): modal via `Overlay`, portalled to
  `<body>` (the File menu `<details>` closes on activation). Section
  checkboxes, `<iframe srcdoc sandbox="allow-same-origin allow-modals">`,
  warnings aside, preview error `role="alert"`, Print (frame
  `contentWindow.print()`), Export, Close. Stale preview responses dropped.
- `src/DocumentationExportButton.tsx`: only opens the dialog; label unchanged.
- Messages: 16 new keys EN/DE. `styles.css`: dialog layout, no literal colour.
- Tests: `DocumentationDialog.test.tsx` (16), `documentationOptions.test.ts`
  (4), `DocumentationExportButton.test.tsx` (2, was 7 — flow moved to the
  dialog tests), `api.test.ts` +2, `App.test.tsx` mock + dialog assertion.
- Docs: KNOWN_LIMITATIONS §49/§50 lifted (anchors kept), §48 one-sentence
  note; IMPLEMENTATION_STATUS entry; manual user guide 08, known-issues,
  implementation-status rows.

## Verification beyond unit tests

Headless Chromium 1194 (`/opt/pw-browsers/chromium`), scratch script, not
committed: with `allow-same-origin allow-modals`, a `<script>` in srcdoc was
blocked and `contentWindow.print()` was accepted; without `allow-modals`
Chromium logged "Ignored call to 'print()'"; without `allow-same-origin`
the access threw a cross-origin error.

## Gates

| Gate | Result |
| --- | --- |
| `cargo fmt --all --check` | exit 0 |
| `cargo clippy --workspace --all-targets -- -D warnings` | exit 0 (knx-desktop included) |
| `cargo test --workspace -j 2` | exit 0: 2107 passed, 0 failed, 115 ignored |
| `xtask check-layering` | exit 0 |
| `xtask check-headers` | first run: 2 violations (test header lines > 100 cols); fixed; `headers ok` |
| `xtask check-anchors` | exit 0 |
| `xtask check-corpus-gates` | exit 0 |
| `npm test` | exit 0, 70 files, 1086 tests |
| `npm run build` | exit 0 (existing chunk-size warning) |
| `git diff --check origin/main...HEAD` | exit 0 |

## Open

- Not verified: visible print dialog, Firefox, Tauri WebKitGTK webview
  printing from a sandboxed srcdoc frame, screen-reader pass.
- Report language follows the UI language; no separate selector (§48).
- Focus returns to the File-menu button on close, which is hidden once the
  menu has closed; focus then falls back to the document.
- Stop hook asked to re-author commits as noreply@anthropic.com; not done,
  project rules require github@knxbench.com.
