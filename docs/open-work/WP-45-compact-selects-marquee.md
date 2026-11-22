# WP-45 — Compact dropdown fields with hover marquee on overflow

**Release:** the delivered parts of this package ship in Alpha 23 ([release notes](../releases/alpha23.md)).

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
| 45.1 | Shared bounded select component/CSS class: max-width ~20–25 ch, ellipsis, full value still reachable (title/aria + native dropdown) | Delivered 18 Nov 2026 on work/wp45-compact-selects-20261118: `CompactSelect.tsx` + `.select-compact` CSS (10 rem scalar slot) |
| 45.2 | Hover marquee behaviour: soft-start left→right scroll, fade-in, 2 s pause, loop until hover ends; disabled under reduced motion | Delivered 18 Nov 2026: `@keyframes knx-select-marquee` (ease-in-out, 40% of the cycle is the end pause at standard motion), scoped to `:hover`/`:focus-within` inside `@media (prefers-reduced-motion: no-preference)` + `:root:not([data-motion-level="off"])`, duration from `--knx-transition-duration` (motionGuard green) |
| 45.2a | Correction (WP-48.6, F10): the delivered animation translated the whole native select, moving its border and arrow without revealing the clipped text | The select now stays stationary. While hovered or keyboard-focus-visible (and motion allowed) an `aria-hidden` overlay over its text area (arrow zone left free; the select's own text turns transparent, options keep their ink) scrolls the complete label; reduced motion and motion "off" leave only the ellipsis, the `title` tooltip and the native list. Overflow is measured at mount, after every commit and on resize (`ResizeObserver`); every forwarded handler is called. Browser geometry proof: `e2e/wp48-compact-select-geometry.e2e.ts` |
| 45.3 | Apply the shared class to the named offenders (DPT, linked group addresses, Herstellerfelder and other oversized selects) without breaking WP-41 slot policy | Delivered 18 Nov 2026: DPT select (Inspector), link-direction select (verknüpfte Gruppenadressen row), Restriction select (Herstellerfelder, ParameterPanel); WP-41 slot CSS extended to `.select-compact` in styles.css and parameterWorkspace.css |

## Boundaries

- Pure web UI/CSS; no data or server changes.
- Must not break keyboard operation, focus visibility, or the native option
  list; tests via existing Vitest/browser lanes.
- Coordinate with WP-41 (shared field-width system) — extend it, do not fork
  it.
