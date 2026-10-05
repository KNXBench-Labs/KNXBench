# Themes: one dropdown, CRT shipped, storage location, Neon Grid/Bitcoin DeFi removed (goal-ui owner, 2026-10-05)

User request (verbatim intent): ship Modern Retro CRT by default; the preview
cards can go, the dropdown is enough; state the theme storage path in the
Settings pane; Neon Grid and Bitcoin go. Recorded as ADR-0079 (supersedes the
preview/per-theme management parts of ADR-0060).

Worktree `ui-themes` on `origin/main` `4804982b` (after the splitter fix in the
same lock tenure; lock taken in `a33b4073`).

## What "Preview Fehler" meant

Screenshot of the old Appearance section (intercepted fixture): under the Theme
dropdown, a card per theme — "System / Built-in / system Included with the
application / [Preview]" etc. — plus "Use system theme". These are the
"Preview-Felder" the user wanted gone; the dropdown already listed every theme.

## Change

- `ThemePackManager.tsx` rewritten: import (admit → install + select in one
  conditional write; replacement consent overlay kept), Export/Remove for the
  selected pack, diagnostics + recovery, storage hint. No list, no preview, no
  Apply/Cancel, no System button.
- `theme.ts`: `ThemePreview` removed; `findThemePack` / `isBundledThemeId`;
  installed pack wins over a shipped one with the same id; Neon Grid/Bitcoin
  DeFi out of the registry. `SettingsPanel` takes `manageThemes`.
- `bundledThemes.ts` (new): the CRT file through `parseThemePackText`.
- `themePackStorage.planThemeSelection`: a shipped id is selectable without a
  `uiThemePacks` entry (also when the map key is absent).
- `styles.css`: the two palette blocks and the unused list/card rules removed;
  `index.html` bootstrap list follows (guarded by `themeTokens.test.ts`).
- en/de: 11 preview/card keys removed, `themePack.manager.storage` added,
  failure texts no longer mention a preview, heading "Theme files"/"Design-Dateien".

## Evidence

- RED first: `themeSettings.test.tsx` 8 of 9 failing on the old code for the
  intended reasons (shadowing case trivially green without a shipped pack).
- Mutants (scratch, byte-exact restore): 12/12 killed — bundled-wins,
  listed-twice, neon-kept, crt-not-listed, crt-selection-refused,
  crt-removable, replacement-unasked, import-not-applied, storage-hint-missing,
  late-file-applied, second-file-during-write, peer-change-keeps-question.
- Focus finding: Playwright's `setInputFiles` does not focus the file control,
  so the overlay restored focus to `body`; the spec now focuses the control
  first, as a user's click does. `Overlay` unchanged.
- Visual check (scratch probe, removed): German UI with CRT selected renders the
  dropdown, "Design-Dateien", export button and storage hint legibly.
- Gate (scratch `themes/gate.sh`, leases 7/8/9 acquired 14:56 after Alpha's
  ar07-close gate, inputs frozen): build 0, flow-study 0, theme-fixtures 0,
  Vitest 2,002 / 116 files, Chromium full suite 132 passed, theme specs ×3
  81 passed, check-anchors 539 / 285, check-ledger 186, `git diff --check`
  clean; check-headers 1 — three new header lines over 100 columns
  (ADR-0018). Shortened; rerun under the leases: check-headers 533 / 157,
  build 0, the two affected Vitest files 33 / 33, diff-check clean.
- Before publishing, `origin/main` moved five commits (story, AR14B docs,
  handover; no `apps/knx-web` file, Dockerfile still `KNX_DATA_DIR=/data`).
  Rebased without conflicts; docs checks rerun under leases 7/8: anchors 545,
  ledger 187, headers 533 / 157, diff-check clean.

No KNX/bus contact; all settings traffic intercepted/synthetic. No Rust source
changed.
