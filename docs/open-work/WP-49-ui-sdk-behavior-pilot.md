# WP-49 — Pilot: headless UI behaviours from ui-sdk for menus and comboboxes

Owner request: 21 November 2026 (WebUI session). 23 November 2026: owner Go for
49.1 only ([OWN-01](OWN-01-owner-decisions.md)); 49.1 is delivered as the
[ADR-0191](../adr/0191-ui-sdk-beta-headless-dependency-for-web-ui-behaviours.md)
draft. Later on 23 November 2026 the owner accepted ADR-0191 with the
**fallback**: no ui-sdk dependency; the library's behaviour and testing rules are
applied to the hand-written widgets (49.2 onwards).

## Requested outcome

Find out with a small, reversible pilot whether the headless behaviour
library ui-sdk ([vercel-labs/ui](https://github.com/vercel-labs/ui),
Apache-2.0, npm `@ui-sdk/core` + `@ui-sdk/react`) can replace part of the
web UI's hand-written keyboard, focus, dismissal and ARIA code without a
visual or behavioural regression. If the pilot convinces, record the adoption
as an ADR; if not, remove it again and keep only the behaviour/test lessons.

The library ships behaviours, not styles: KNXBench keeps its own theme tokens,
presentations and CSS.

## Current basis (source anchors on `origin/main` `ed47bcad4`)

The web UI (`apps/knx-web`, React 19.2, Vite, Vitest with happy-dom,
Playwright lanes) builds every interactive widget itself. Production `.tsx`
files (tests excluded) carry 6 `role="dialog"`, 4 `role="menu"`,
4 `role="combobox"`/`role="listbox"` pairs, 2 `role="tablist"` and
2 `role="toolbar"`:

- Comboboxes with `aria-activedescendant` and own arrow/Escape handling:
  `GroupAddressPicker.tsx:49-73`, `CommandPalette.tsx:79-123`,
  `Search.tsx:65-92`, `BusComposeForm.tsx:362-379`.
- Menus: `FileMenu.tsx:39,296-299,358,676` (roving-tabindex menu with flyout
  submenus on `<details>`), `RenameWorkbench.tsx:56` (context menu).
- Tabs: `Inspector.tsx:1081-1084` (device tabs), `BusMonitorPanel.tsx:1222`.
- Toolbars: `TelegramFlowView.tsx:599`, `features/rtf/InstructionEditor.tsx:118`.
- Dialogs are individual components (for example `ContributionButton.tsx:26`,
  `StoredBusAccess.tsx:118`, `DiagnosticExportButton.tsx:28`) layered by
  `ModalForeground.tsx:7`; there is no shared dialog primitive.
- Motion and theme rules that a pilot must keep: motion levels and the
  reduced-motion guard (`motionGuard.test.ts`, `themeTokens.test.ts`), the
  built-in presentations (ADR-0092 LCARS, ADR-0157 rail).

## Bounded packages and actual status

| Package | Outcome | Status |
| --- | --- | --- |
| 49.1 | Evaluation spike in a throwaway branch: install the exact beta version, measure bundle-size delta, check React 19 / TypeScript 7 / Vite 8 / happy-dom compatibility, offline and Tauri operation (no runtime fetch), coexistence of its optional `@layer` CSS with the theme tokens and presentations; write an ADR draft with the measured results | Delivered, 23 November 2026; ADR-0191 accepted with the fallback: +48.6 kB gzip for one combobox (+59 kB with a menu), no peer or transitive packages, no network code, 29/29 `tsc` configs clean; the first Escape propagates and the library CSS fails ADR-0022, so the ADR recommends the fallback (spike evidence outside the repo, never merged) |
| 49.2 | Fallback, comboboxes: apply the ADR-0191 behaviour and testing checklist (focus stays in the input, `aria-activedescendant` follows the highlight, arrows stop at the ends, Enter chooses, first Escape closes and stops, second Escape belongs to the page, Tab closes, pointer press does not blur; `aria-haspopup`/`aria-expanded`/`aria-controls`) to `GroupAddressPicker`, `CommandPalette`, `Search` and `BusComposeForm` without a dependency; fix only gaps the tests expose | Delivered, 23 November 2026 (see below) |
| 49.3 | Fallback, menus: the same checklist for `FileMenu` and the `RenameWorkbench` context menu (dismissal, focus return to the trigger, roving focus, submenus) | After 49.2 |
| 49.4 | Fallback, tabs, toolbars and dialogs: the checklist for the remaining widget families (one package each) | After 49.3 |

### 49.2 result (comboboxes)

One shared checklist helper (`src/test/comboboxChecklist.ts`, test-only, no production
abstraction) runs nine cases per widget in `GroupAddressPicker`, `CommandPalette`,
`Search` and `BusComposeForm` (destination field): focus stays in the input, `aria-activedescendant`
follows the highlight and is absent without one, arrows stop at the ends, Enter chooses,
Escape, Tab, pointer press, and `aria-haspopup`/`aria-expanded`/`aria-controls`.

- Semantics today, all four: `aria-selected` marks the *highlighted* option (no separate
  "chosen" mark); the picker lists are `hidden` and empty when closed; palette and search lists are
  always rendered inside the `Overlay` dialog. Nothing wraps; palette arrows skip disabled rows.
- Fixed (two gaps): a pointer press on a `Search` or `CommandPalette` row moved focus to the dialog
  panel and blurred the input (the pickers already prevented it). Both rows now prevent the
  default of `mousedown`; a press on a disabled palette row no longer blurs either.
- Pinned as the current policy (not changed): in `Search` and `CommandPalette` the first Escape
  closes the whole overlay (the `Overlay` shell stops it; there is no list to close first), and
  Tab does not close the always-visible list (the `Overlay` focus trap keeps focus inside).
- Not covered in happy-dom: real focus/blur order on a pointer press is checked through the
  `mousedown` default being prevented, not through a browser.

## Boundaries

- Web UI only; no domain, server, storage or bus changes.
- No ui-sdk dependency (ADR-0191, fallback accepted 23 November 2026).
- No visual change beyond what the owner accepts; keep theme tokens,
  presentations, motion levels and dark/light behaviour.
- Its agent-tool and WebMCP features are out of scope for this pilot.
- Tests through the existing Vitest and browser lanes; colour contrast is
  checked in a real browser, not in happy-dom.

## Open decisions

Answered by the owner on 23 November 2026 ([OWN-01](OWN-01-owner-decisions.md)):

1. **Is a beta headless dependency acceptable?** Yes in principle, if the spike
   convinces, under an exact version pin (no `^`/`~`), updates only deliberately
   in their own PR with tests, and a documented revert path in the ADR.
2. **Fallback if 49.1 shows a problem?** Yes: apply only the library's behaviour
   and testing rules to the existing hand-written widgets, without the dependency.

3. **ADR-0191 accepted?** Yes, with the fallback (no dependency), 23 November 2026.

Raised by 49.2 (owner, not yet decided; behaviour unchanged meanwhile):

4. **Escape and Tab in `Search` and `CommandPalette`:** keep "first Escape closes the
   overlay, Tab stays in the dialog" (current), or give these always-visible lists the picker
   semantics (first Escape clears/collapses, second closes)?
5. **`aria-selected` meaning:** ADR-0191 suggests `aria-selected` for the chosen option plus a
   separate highlight attribute; all four widgets mark the highlighted option today. Change it?
