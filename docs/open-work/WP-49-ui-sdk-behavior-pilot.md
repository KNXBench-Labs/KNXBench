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
| 49.2 | Fallback, comboboxes: apply the ADR-0191 behaviour and testing checklist (focus stays in the input, `aria-activedescendant` follows the highlight, arrows stop at the ends, Enter chooses, first Escape closes and stops, second Escape belongs to the page, Tab closes, pointer press does not blur; `aria-haspopup`/`aria-expanded`/`aria-controls`) to `GroupAddressPicker`, `CommandPalette`, `Search` and `BusComposeForm` without a dependency; fix only gaps the tests expose | Delivered, 23 November 2026, PR #143 (main `bcf205b72`; see below) |
| 49.3 | Fallback, menus: the same checklist for `FileMenu` and the `RenameWorkbench` context menu (dismissal, focus return to the trigger, roving focus, submenus) | Delivered, 23 November 2026, PR #144 (main `790468206`; see below) |
| 49.4a | Fallback, tabs: the checklist for the tablists in `Inspector` and `BusMonitorPanel` | Done, awaiting review |
| 49.4b | Fallback, toolbars: the checklist for `TelegramFlowView` and `features/rtf/InstructionEditor` | After 49.4a |
| 49.4c | Fallback, dialogs: the checklist for the `Overlay` shell and its dialog consumers (`NewProjectDialog`, `LoginScreen`, `RenameWorkbench`, the `App` dialogs) | After 49.4b |

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

### 49.3 result (menus)

One shared checklist helper (`src/test/menuChecklist.ts`, test-only, reusing `press()` from the
combobox helper) runs eight cases per menu, plus five submenu cases for the File menu:
`FileMenu.checklist.test.tsx` (real `FileMenuSummary`/`FileMenuContent`/`FileSubmenu` inside a
harness that copies `App`'s Escape and outside-`pointerdown` wiring; `App.test.tsx` pins that wiring
against the real component, including the unstopped first-level Escape) and `RenameWorkbench.checklist.test.tsx` (the context menu).

- Semantics today: opening focuses the first *enabled* entry and leaves it the only tab stop;
  ArrowDown/ArrowUp **wrap**, Home/End exist (File menu); disabled entries are real `disabled`
  buttons, skipped by the arrows and inert; Enter/Space work through native buttons; ArrowRight opens
  and ArrowLeft/Escape close a submenu with focus back on its trigger (the first Escape is stopped);
  Tab closes the whole menu and focus rests on the File control; an outside `pointerdown` closes.
  The File control announces `aria-haspopup="menu"`/`aria-expanded`; submenu triggers add
  `aria-controls` and the panel `aria-labelledby`. The context menu has one entry, no persistent
  trigger (focus returns to the element it was opened from), Escape and Tab close it and are
  swallowed.
- Fixed (three small gaps, each found by the checklist before the fix): `FileMenuItem` rendered a
  type-less button (a `submit` button inside any form). Fixed class-wide: `type: "button"` is now part of
  the shared item props (`useFileMenuItemProps`), the two CSV buttons that can render outside a menu
  carry it explicitly, and `App.test.tsx` asserts every rendered `[data-file-menu-item]` is
  `type=button` (seven contributed entries were type-less before; no live submit risk was found); the context menu had no accessible name
  (now `aria-label` = the existing "Rename" caption); its arrow keys, Home and End were not
  prevented, so the page behind the menu could scroll.
- Pinned as the current policy (not changed): arrow wrap-around; the first-level Escape of the File
  menu is neither prevented nor stopped (it bubbles to the page; pinned on the real `App` as a
  current-policy test, owner question 8); Tab in the context menu is
  swallowed while the File menu leaves it to the browser; the first-level File menu (`role="menu"`)
  has no accessible name and no `aria-controls` link from the File control (submenus have both).
- Not covered in happy-dom: real focus/blur order on a pointer press and the native Enter/Space to
  click conversion; they are asserted through native `<button type="button">` entries.

### 49.4a result (tabs)

One shared checklist helper (`src/test/tabsChecklist.ts`, test-only, reusing `press()` from the
combobox helper) runs nine cases per tablist against the REAL components:
`DeviceWorkspace.tabs.test.tsx` (the Inspector device tabs, seven tabs) and
`BusMonitorPanel.tabs.test.tsx` (the Table/Flow view tabs, mounted through the panel's own session
reattach). Each mounts inside a parent that counts keydowns.

- Semantics today: both tablists are named (device name; the view caption) and carry `tab` roles
  with exactly one `aria-selected="true"`; roving tabindex (selected 0, others -1); ArrowRight/
  ArrowLeft **wrap** and **activate at once** (automatic activation) and move focus; the arrows are
  prevented (no page scroll); Tab is not handled and leaves the tablist; no tab is ever disabled. The
  Inspector also handles Home/End and puts every panel in the tab order (`tabIndex` 0, hidden ones
  are not focusable); every Inspector tab has `aria-controls` and its panel `aria-labelledby`.
- Fixed (one gap, found by the checklist before the fix): the Inspector's last (configuration) tab
  pointed `aria-controls` at `device-panel-<id>-<last>` while its panel had the id
  `device-panel-<id>-config`, an orphan reference. The panel id now follows the tab index like the
  other six (nothing referenced the old id).
- Pinned as the current policy (not changed): `BusMonitorPanel` ignores Home/End (two tabs); only its
  selected tab carries `aria-controls`, because the Table panel is not mounted while Flow is shown
  (the Flow panel stays mounted, hidden); its panels are not tab stops; handled arrows bubble to
  ancestors and the window (no ARIA-relevant listener acts on them: `multiSelection.ts` only acts on
  Escape, and the achievements Konami matcher, `konami.ts` via `useAchievements.ts`, observes the
  arrows harmlessly; no side effect today).
- Not covered in happy-dom: the browser's real Tab order out of the tablist is checked through the
  roving tabindex and the unprevented Tab, not by moving focus.

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

Raised by 49.3 (owner, not yet decided; behaviour unchanged meanwhile):

6. **Wrapping and Tab:** keep arrow wrap-around in the File menu (the APG allows both), and keep the
   inconsistency that Tab is swallowed by the context menu but left to the browser by the File menu?
7. **Name the first-level File menu?** Today only its submenus are named and linked; naming it needs
   an `id` shared between `FileMenuSummary` and `FileMenuContent` (an `App` call-site change).
8. **First-level Escape in the File menu** is not stopped (submenu Escape is): stop it, so it cannot
   reach other page-level Escape handlers?

Raised by 49.4a (owner, not yet decided; behaviour unchanged meanwhile):

9. **Tabs policy:** keep automatic activation with wrap-around in both tablists (the APG allows
   both), and add Home/End to the two-tab bus monitor view switch (the APG lists them as optional)?
10. **`aria-controls` on unselected bus monitor view tabs:** keep omitting it (the Table panel is
    not mounted then), or always mount both panels?
11. **Modifiers on tab arrows:** both tab keydown handlers (`Inspector.tsx`, `BusMonitorPanel.tsx`)
    ignore modifier keys, so Alt+Left (browser back) and Ctrl+Arrow are `preventDefault`-ed. Skip
    handling when a modifier is held? Behaviour unchanged meanwhile.
