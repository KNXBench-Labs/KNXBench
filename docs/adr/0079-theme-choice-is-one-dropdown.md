# ADR 0079: Theme choice is one dropdown; shipped packs are data; Neon Grid and Bitcoin DeFi retired

Date: 2026-10-05

Status: Accepted — user decision 2026-10-05. Supersedes the preview and
per-theme management parts of [ADR-0060](0060-versioned-declarative-theme-packs.md)
(its "Preview overrides…" paragraph and the U17 preview ownership); the pack
format, admission, conditional persistence and recovery decisions of ADR-0060
stand unchanged.

## Context

Settings › Appearance showed the Theme dropdown *and* a card per theme (System,
each palette, each imported pack) with a Preview button, an Apply/Cancel
preview flow and a "Use system theme" button. The user judged the cards
redundant ("Dropdown reicht"), asked for the Modern Retro Green CRT palette to
ship with the application instead of being a file to import, for the theme
storage location to be stated in the Settings pane, and for Neon Grid and
Bitcoin DeFi to be removed.

The CRT palette already existed as a validated v1 pack file,
`apps/knx-web/themes/modern-retro-green-crt.knx-theme.json`, covered by
`retroGreenTheme.test.ts` (admission, exact design values, contrast, export/DOM
roundtrip).

The server deliberately never sends host paths to the browser
(`apps/knx-server/src/paths.rs`, `SettingsDto::moved_to`): the browser has no
business learning the server's filesystem layout.

## Decision

1. **The Theme dropdown is the only theme chooser.** No per-theme cards, no
   preview state, no "Use system theme" button (System is a dropdown entry).
   Every choice in the application's Appearance goes through the existing
   acknowledged conditional write (unchanged from ADR-0060).
2. **Theme files are managed beside it, without preview.** Importing a file
   admits it with the unchanged parser and then installs *and selects* it in
   one conditional write. Replacing an installed id still needs explicit,
   context-bound consent; a peer change still withdraws an open question.
   Export and Remove are offered for the *selected* pack (Remove only for an
   installed pack, with the existing confirmation and atomic fallback to
   System). Diagnostics and the recovery export stay; the recovery button is
   shown with the diagnostics it exists for.
3. **Shipped packs are data, admitted like an import.** `bundledThemes.ts`
   reads the `.knx-theme.json` files under `apps/knx-web/themes/` at build time
   and passes them through `parseThemePackText`. A shipped pack is never
   written into `uiThemePacks`; selecting it stores only its id. Its id keeps
   the `user-` prefix the v1 format requires, so a user who imported the same
   file before keeps a working selection. **An installed pack with the same id
   takes the shipped one's place** — it is the user's own data — and the
   dropdown lists that id once.
4. **Neon Grid and Bitcoin DeFi are removed** from the registry, the
   stylesheet and the pre-mount bootstrap list. A saved `neon-grid` or
   `bitcoin-defi` choice is not rewritten: it is shown as System and reported
   through the existing `missingSelection` diagnostic, like any unknown id.
5. **The storage location is stated, not served.** The Settings pane names the
   file and keys (`settings.json`, `uiThemePacks`, `theme`) and the data folder
   per deployment: desktop app `~/.local/share/com.knxbench.knxbench-labs`
   (Tauri `app_data_dir`, identifier from `tauri.conf.json`), server
   `KNX_DATA_DIR` (Docker image `/data`). No API returns a host path.

## Alternatives

- Convert CRT into a fourth CSS palette block: rejected; it would duplicate the
  token values held by the pack file and bypass the admission every user pack
  goes through.
- Let the shipped copy win over an installed one with the same id: rejected;
  it would silently override data the user installed.
- Serve the absolute data directory from the server: rejected for now; it
  contradicts the existing no-host-path rule and would need its own decision
  for authenticated/remote deployments.
- Keep preview for imports only: rejected by the user's request; the dropdown
  is an immediate, reversible way back.

## Consequences

- Choosing a theme is a saved change, not a reversible draft. Undo is choosing
  the previous entry in the dropdown.
- Pack-based themes, shipped or installed, are painted after the application
  module loads; the pre-mount bootstrap only knows CSS palettes and shows the
  System palette until then (unchanged behaviour for packs, now also for CRT).
- The stated paths are the documented defaults; a desktop build with a changed
  identifier or a server started with another `KNX_DATA_DIR` must update or
  override them.
- Tests: `themeSettings.test.tsx` (new), the rewritten
  `ThemePackManager.integration.test.tsx` and `e2e/theme-manager.e2e.ts`;
  `themePreview.test.tsx` was removed with the feature it tested.
