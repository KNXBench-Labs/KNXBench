# WP-45 — Compact dropdown fields with hover marquee on overflow

Origin: owner UI feedback, 17 November 2026 (WebUI session). Not tied to a
GitHub issue.

## Requested outcome

Many `<select>` dropdowns are far too wide — e.g. the manufacturer-fields
("Herstellerfelder") tab, the DPT select and "Verknüpfte Gruppenadressen" in
the inspector. Requirements:

1. Selects cap at roughly **20–25 characters** width (shared value-slot policy
   from WP-41: 10 rem scalar slot stays authoritative where already applied —
   this package is about selects that ignore it).
2. If the selected value does not fit, on **hover** the content scrolls
   through the field: slow soft start from left to right, gentle fade-in,
   2 s pause after the end, then loop — until hover ends.
3. Keyboard/focus and screen-reader access to the full value must not degrade;
   reduced-motion preference disables the scroll (show full value or ellipsis
   statically instead).

## Current basis (source anchors on `origin/main` `9af5c698a`)

- Select styling candidates: `apps/knx-web/src/styles.css:927`
  (`.parameter-field select`), `:982-983` (functions panel), `:1154`
  (`.search-panel select`), `:1253` (`.settings-field select`), `:2371`
  (`.service-control-fields select`); `parameterWorkspace.css:47,50,51`
  (`--parameter-value-width`).
- Named consumers: DPT select in `Inspector.tsx` (`dpt.selectionHint` area,
  `:458/:481`), linked group addresses `inspector.linkedGroupAddresses`
  (`messages/de.ts:2107`), manufacturer-fields tab
  `parameters.restrictedTab` (`ParameterPanel.tsx:630/:757`).
- Motion plumbing: `helpTipAnimates()` in `apps/knx-web/src/help.ts`; global
  and per-category motion controls already exist (LS-05).

## Bounded packages

| Package | Outcome | Status |
| --- | --- | --- |
| 45.1 | Shared bounded select component/CSS class: max-width ~20–25 ch, ellipsis, full value still reachable (title/aria + native dropdown) | Not started |
| 45.2 | Hover marquee behaviour: soft-start left→right scroll, fade-in, 2 s pause, loop until hover ends; disabled under reduced motion | Not started |
| 45.3 | Apply the shared class to the named offenders (DPT, linked group addresses, Herstellerfelder and other oversized selects) without breaking WP-41 slot policy | Not started |

## Boundaries

- Pure web UI/CSS; no data or server changes.
- Must not break keyboard operation, focus visibility, or the native option
  list; tests via existing Vitest/browser lanes.
- Coordinate with WP-41 (shared field-width system) — extend it, do not fork
  it.
