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
| 49.4a | Fallback, tabs: the checklist for the tablists in `Inspector` and `BusMonitorPanel` | Delivered, 23 November 2026, PR #145 (main `b15363a25`; see below) |
| 49.3b | Apply the owner decisions on questions 4-8 (23 November 2026): name the first-level File menu, stop its first-level Escape, turn the other pins into decided rules | Delivered, 23 November 2026, PR #147 (main `f3836adc9`; see below) |
| 49.4b | Fallback, toolbars: the checklist for `TelegramFlowView` and `features/rtf/InstructionEditor` | Delivered, 23 November 2026, PR #148 (main `78b08f298`; see below) |
| 49.4c | Fallback, dialogs: the checklist for the `Overlay` shell and its dialog consumers (`NewProjectDialog`, `LoginScreen`, `RenameWorkbench`, the `App` dialogs) | Delivered, 23 November 2026, PR #150 (main `8ab381831`; see below) |
| 49.5 | Apply the owner decisions on questions 9-17 (23 November 2026) | Delivered, 25 November 2026, PR #152 (main `6092ec52f`; see below) |

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

### 49.4b result (toolbars)

One shared checklist helper (`src/test/toolbarChecklist.ts`, test-only, reusing `press()` from the
combobox helper) runs six cases per toolbar against the REAL components:
`TelegramFlowView.toolbar.test.tsx` (the flow toolbar, plus the empty-state toolbar) and
`features/rtf/InstructionEditor.toolbar.test.tsx` (the formatting toolbar). Each mounts inside a
parent that counts keydowns; toggle state is re-checked after every state change.

- Semantics today: both toolbars are `role="toolbar"` with an accessible name and a horizontal
  orientation (no `aria-orientation`), every control is a named `type="button"`. The flow toolbar's
  Auto zoom, Maximize/Restore and Freeze are toggles with `aria-pressed`; the formatting buttons
  (Bold, Italic, Underline, Remove formatting) act on the selection and are correctly not toggles.
  Disabled controls use the native `disabled` attribute (Rearrange while frozen; all four formatting
  buttons while busy or in a conflict), so they are skipped by Tab and cannot take focus.
- Fixed (one gap, found by the checklist before the fix): the flow view's empty-state toolbar (no
  telegrams observed yet) carried the `flow-toolbar` class but no `role="toolbar"` and no name; it
  now has both, like the populated one. Regression: the empty-state checklist run fails on the
  reverted fix (mutant checked).
- Pinned as the current policy (not changed): neither toolbar uses a roving tabindex; every enabled
  button is a tab stop. Neither handles arrows, Home or End (modified or not), so they are left to the
  browser, bubble to ancestors and the window, and never move focus (question 12; modified arrows:
  question 11 does not apply until a toolbar handles arrows).
- Pinned parent-owned behaviour: while the flow view is maximized, a document capture listener
  restores the view on Escape (prevented and stopped, so neither the toolbar nor the window see it)
  and returns focus to the opener, and it wraps Tab and Shift+Tab between the first and last control
  of the section; outside the maximized state Escape is untouched. The formatting buttons keep the
  textarea selection on a pointer press (`mousedown` prevented, mutant checked) and move focus back to
  the textarea after activation; the editor has no keyboard shortcuts of its own (Ctrl+B/I/U are
  left to the browser and format nothing).
- Not covered in happy-dom: the real focus/blur order on a pointer press and the real caret after a
  format; both are asserted through the recorded selection and the prevented `mousedown`.

Owner question 12 (toolbar tab order) is listed under Open decisions.

### 49.4c result (dialogs)

One shared checklist helper (`src/test/dialogChecklist.ts`, test-only, reusing `press()`) runs against
the REAL components: `Overlay.dialog.test.tsx` (the shared `Overlay` shell through a small consumer, in
two variants: `aria-labelledby` with first-focusable initial focus and a nested dialog, and `aria-label`
with `initialFocusRef` and `restoreFocusRef`), `RenameWorkbench.dialog.test.tsx` and
`LoginScreen.dialog.test.tsx` (initial, expired and signed-out reasons). `TelegramFlowView`'s maximized
section stays pinned in 49.4b. Each mounts inside a parent that counts keydowns.

- Semantics today: every dialog is `role="dialog"` with `aria-modal="true"` and exactly one name source
  (`aria-labelledby` to a visible title, or `aria-label`). `aria-describedby` is wired only on the login
  screen (its notice, for the expired and signed-out reasons); no `Overlay` consumer wires one (recorded,
  not added). The props type allows an `Overlay` with neither name prop, so `overlayNames.test.ts` scans
  the source instead: all 43 production `<Overlay>` tags carry exactly one of `label` (7: Search,
  CommandPalette, FsPicker, LogEntryDetails, RenameWorkbench, two in ThemePackManager) or `labelledBy`
  (36; 31 literal ids, each existing in the same file, 5 dynamic: DocumentationDialog,
  FlowImageExportDialog, NoteDispositionDialog, ProjectDiffPanel, UnsavedChangesDialog).
- Initial focus (pinned per shell): the `initialFocusRef` target if given (22 tags), else the first
  tabbable control, else the panel itself (`tabIndex=-1`, not a tab stop). Rename focuses its name field,
  the login screen its password field (own effect).
- Trap: `Overlay` wraps Tab on the last stop and Shift+Tab on the first (prevented), leaves mid-list Tab
  to the browser and pulls focus back in when it has left the dialog. A nested `Overlay` keeps Tab inside
  the top dialog. The login screen has no trap of its own.
- Escape: `Overlay` stops, prevents and closes it (decided 23 Nov 2026, owner Q14; see 49.5). The
  rename name field handles Escape itself (prevented and stopped). Dialogs that refuse to close while
  busy (`onClose` ignored): RenameWorkbench (`pending`, pinned), ProjectHistoryDialog, LogExportDialog,
  NameRecipeDialog, DeviceComparisonDialog (`adopting`; static grep only, not mounted here). The key is
  still stopped while busy. The login screen handles no Escape: it stays, the key propagates.
- Background: while an `Overlay` is open everything outside its branch is `inert` and `aria-hidden`,
  restored on close (nested dialogs: only the top is active). The login screen isolates nothing itself;
  `AuthGate` makes the app `inert` (pinned in `AuthGate.test.tsx`). No scroll lock exists in either
  (pinned: body and root `overflow` untouched; decided 23 Nov 2026, owner Q16: no scroll lock, see 49.5).
- Stacking: Escape closes only the top dialog and focus returns to the control that opened it inside the
  lower one; the next Escape closes the lower one (nested-in-tree layout).
- Fixed (one gap, found by the RenameWorkbench checklist run): focus did not return to the opener of a
  dialog whose child focuses itself on mount (`autoFocus` in `RenameNameField`, or a mount effect). The
  shell read the opener in its effect, after that child had already taken focus, so it "restored" to the
  vanished child. The shell now reads the opener during its first render. Regression tests: the
  RenameWorkbench checklist case and two `Overlay` cases (autoFocus, mount effect), with the existing
  variants as positive controls; mutant (revert the fix) fails five tests. Behaviour change: after
  closing such a dialog, focus now lands on its opener instead of the page body, or on `restoreFocusRef`
  where the opener is gone.
- Opener gone: `restoreFocusRef` (used by LogEntryDetails and ThemePackManager) takes focus; without it
  focus falls to the page.
- Not covered in happy-dom: real focus order on pointer press and real Tab traversal; Tab wrapping is
  asserted through the prevented event and the focus call.
- Review follow-ups (closed): the focus-pull-in case now really starts outside the dialog for both Tab and
  Shift+Tab; an `Overlay` with no focusable item prevents Tab and keeps focus on the panel; `overlayNames.test.ts`
  pins the exact scan size (40 files, 43 tags; update both when adding or removing a dialog); the login
  password field's `aria-describedby="login-error"` is pinned while an error shows and absent otherwise. Each
  has a mutant that now fails by name. The `RenameWorkbench` context-menu path is pinned as current policy
  (decided 23 Nov 2026, owner Q17; fixed in 49.5).
- Review evidence: the Playwright comparison of candidate against base was identical. Default lane 184/184
  passing on both sides with the same 12 environmental `theme-manager` 403 failures (symlinked `node_modules`
  fonts outside the Vite fs allow-list in archive copies); devices 88/88, catalogue 24/24, dialogs 22/22.

### 49.3b result (owner decisions 4-8)
- Q7 fixed: `FileMenuSummary` and `FileMenuContent` take one required `menuId`; the menu carries that
  `id` and `aria-labelledby` the File control, the control carries `aria-controls` (`App` passes
  `useId()`). The menu checklist now asserts a name and the two-way link for every menu button, and
  `App.test.tsx` asserts both against the real `App`.
- Q8 fixed: the File control's Escape in `App` also calls `preventDefault` and `stopPropagation`.
  Regression tests against the real `App`: a window `keydown` listener does not see the Escape, the
  key is prevented, focus returns to File, and a device multi-selection survives it (positive
  control: the same Escape sent to the window clears it). The checklist harness copy and
  `menuChecklist.ts` no longer have a propagating mode.
- Q4/Q5/Q6 are decided rules, no behaviour change: the combobox and menu checklist tests, adapter
  fields and comments cite the decision (the `escape`/`menuName` adapter fields are gone; `tab`
  stays and documents both decided Tab rules). The tabs pins (questions 9-11) are untouched.
- Open risk: a non-`Overlay` portal child of the File menu (a picker) would now have its Escape stopped
  at the File control too; the control already closed the menu and took focus for it.

### Decisions 23 Nov 2026 (delivered in 49.5)

All nine questions Q9-Q17 are DECIDED ([OWN-01](OWN-01-owner-decisions.md)) and implemented in package 49.5:

- Q9: BusMonitorPanel's tab switcher gains Home/End (wraparound + automatic activation stay); the shared
  `tabsChecklist.ts` now runs Home/End as `present` for both tablists.
- Q10: `aria-controls` stays on the active BusMonitorPanel tab only while only the active panel is mounted;
  the current-policy pin is promoted to the decided contract.
- Q11: both tab key handlers skip Alt/Ctrl/Meta-modified navigation keys (no preventDefault), Shift-only
  handling stays; the checklist gained a named modifier test with plain/Shift positive controls.
- Q12: both toolbars keep every enabled button a tab stop (no roving tabindex); the pins are decided rules
  and ADR-0191 carries a dated amendment recording the conscious APG-toolbar deviation.
- Q13: the InstructionEditor formatting toolbar is named via `deviceInstructions.toolbar` ("Formatting" /
  "Formatierung", plus bar/tlh entries) instead of `copy.rendered`; the preview heading is unchanged.
- Q14: `Overlay`'s Escape handler calls `preventDefault()` before `stopPropagation()`/`onClose` (later
  observers only, not retroactive for capture listeners); the pins asserting `defaultPrevented===false` flipped.
- Q15: `Overlay` props are the exclusive union `OverlayName` (`{label}` XOR `{labelledBy}`); all 43 consumers
  typecheck unchanged, neither/both fails `tsc` (mutant-proven), and `overlayNames.test.ts` keeps only the
  runtime dangling-`labelledBy`-id check.
- Q16: no page/body scroll lock; recorded as decided policy in `Overlay`'s doc comment and the dialog checklist.
- Q17: `RenameWorkbench` passes the renamed tree row as `restoreFocusRef`, so the context-menu path returns
  focus there; pinned in `RenameWorkbench.dialog.test.tsx` (menu, cancel, successful rename, row-gone fallback)
  and verified in a real Chromium spec (`e2e/rename-focus-return.e2e.ts`, dialogs lane: menu path + F2 control).

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

Raised by 49.2 and 49.3, answered by the owner on 23 November 2026 ([OWN-01](OWN-01-owner-decisions.md)); applied in 49.3b:

4. **Escape and Tab in `Search` and `CommandPalette`:** keep "first Escape closes the overlay, Tab stays in
   the dialog" (APG dialog pattern).
5. **`aria-selected` meaning:** keep marking the highlighted option (APG single-select combobox semantics);
   ADR-0191's Fallback section is amended.
6. **Wrapping and Tab:** keep arrow wrap-around in the File menu; the context menu keeps swallowing Tab and
   returning focus to the opener, the File menu leaves Tab to the browser. Intended.
7. **Name the first-level File menu:** done (shared id, `aria-labelledby`, `aria-controls`).
8. **First-level Escape in the File menu:** stopped, so one Escape closes one level.

Raised by 49.4a/49.4b/49.4c, all answered by the owner on 23 November 2026 ([OWN-01](OWN-01-owner-decisions.md)) and delivered in 49.5 — see "Decisions 23 Nov 2026 (delivered in 49.5)" above:

9. **Tabs policy:** keep automatic activation with wrap-around in both tablists, and add Home/End to the bus monitor view switch. DECIDED, delivered.
10. **`aria-controls` on unselected bus monitor view tabs:** keep omitting it while only the active panel is mounted. DECIDED, delivered.
11. **Modifiers on tab arrows:** skip handling Alt/Ctrl/Meta-modified navigation keys in both tab handlers. DECIDED, delivered.
12. **Toolbar tab order:** keep every enabled button a tab stop, no roving tabindex (APG deviation recorded in ADR-0191). DECIDED, delivered.
13. **Instruction editor toolbar name:** its own name via `deviceInstructions.toolbar`. DECIDED, delivered.
14. **Escape default:** `Overlay` now preventDefaults the Escape it handles. DECIDED, delivered.
15. **Required dialog name:** `Overlay` props are the exclusive union `OverlayName`. DECIDED, delivered.
16. **Scroll lock:** keep no page scroll lock; recorded as policy. DECIDED, delivered.
17. **Focus after the rename dialog from the context menu:** pass the tree row as `restoreFocusRef`. DECIDED, delivered.
