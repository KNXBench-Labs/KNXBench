# File menu and submenus

UI-03 of the [UI feedback set](../goals/ui-feedback-2026-10-18.md) (owner request 1,
18 October 2026). Presentation and keyboard behaviour only: no API, schema,
stored-data or protocol change, and no action was added, removed or re-implemented.
Every entry is the same button with the same handler it had before; the menu
around them changed.

## Shape

The File menu has two kinds of rows.

| Where | Rows |
| --- | --- |
| First level | **New project…**, **Open project…**, **Open (.knxdb)…**, **Save As…** (frequent project actions), then **Logout** (only where a session exists) and **Quit** (desktop shell only) |
| Submenu **Import & export** | web-only **Export project…**, selective import, group-address CSV export/import, documentation export |
| Submenu **History & comparison** | **Project history**, **Compare with…** (its result opens in a dialog, see below) |
| Submenu **Help & diagnostics** | debug report, support-gap analysis, device support report, diagnostic export, introduction, achievements (while switched on), About |

Native and web differences are unchanged because the conditions are unchanged:
the browser export is absent in the desktop shell, **Quit** exists only there,
**Logout** only with a session. **Save** stays on the toolbar. A submenu row is
always enabled (it is a place, not an action); the entries inside keep their own
disabled states, so with no project open **Project history** and **Compare with…**
are present and disabled.

The submenu markup lives in `apps/knx-web/src/FileMenu.tsx` (`FileMenuSummary`,
`FileMenuContent`, `FileMenuItem`, `FileSubmenu`, `useFileMenuItemProps`); `App.tsx`
only chooses which entries go where. The menu is still a native `<details>`, so the
three existing closers (Escape, outside click, any activation) keep working and every
child component keeps finding its summary for focus restoration.

## What a closed submenu keeps

A folded submenu is `hidden`, not unmounted. Several entries own state that must
outlive the panel: the selective-import, documentation, debug-report, support and
comparison dialogs live in their button components. Unmounting would have discarded
an in-flight import or a comparison in progress when the pointer drifted away. A reopened File menu starts with every
submenu folded (the `open` attribute of the `<details>` is observed), but nothing
inside is reset.

## ARIA

The File control and its menu follow the WAI-ARIA menu-button pattern, completely.

- The File control (`FileMenuSummary`, the `<summary>`) carries `aria-haspopup="menu"`
  and `aria-expanded`, which follows the `open` attribute of the `<details>`.
- The first-level list is `role="menu"`. Entries are **declared**, never guessed:
  `FileMenuItem` renders a button the File block owns, and a component that owns its
  own button (selective import, CSV, documentation export, comparison, debug report,
  contribution, support report, diagnostic export) spreads `useFileMenuItemProps()`
  onto it. A declared entry has `role="menuitem"` and `data-file-menu-item`; outside a
  File menu the hook returns nothing, so the reused CSV and support buttons are plain
  buttons elsewhere. Keyboard handling asks for declared entries only.
- **The menu contains declared entries only.** Nothing interactive other than entries
  lives inside the menu or a submenu panel: no report, input or toggle. What an entry
  produces for the user to work with opens in a dialog rendered into the document body
  (a portal), outside the menu. The project comparison is the case in point: **Compare
  with…** is an ordinary entry (the click closes the menu like every other one), and its
  result (`ProjectDiffPanel`, the shared `Overlay`) is a modal dialog labelled
  **Comparison result**, with the grouped counts, the entity tables and **Close**. It
  behaves as before otherwise (file choice, refusal text, import diagnostics, filter,
  expandable tables, achievement event).
- A submenu row is a declared `menuitem` with `aria-haspopup="menu"`, `aria-expanded`
  and `aria-controls`; its panel is a `role="menu"` labelled by that row.
- **Roving tabindex, one tab stop per level.** In each menu level exactly one enabled
  entry has `tabindex="0"` (the one that last had focus, otherwise the first enabled
  one); the others have `-1`. The set is re-derived whenever the menu renders and when
  an entry takes focus, so an entry that appears later (a project opens) joins at `-1`.
  Nothing observes the DOM or tags buttons behind the menu's back.
- Because the entries are `menuitem`s, a browser test finds them with
  `getByRole("menuitem", …)`, not `getByRole("button", …)`; the closed File menu is not
  in the accessibility tree at all. Entries inside a folded submenu are hidden until
  its row is opened (`openFileSubmenu` in `e2e/file-menu.ts`).
- Disabled items are skipped by arrow navigation, as disabled buttons cannot
  take focus.

## Keyboard

| Key | On | Effect |
| --- | --- | --- |
| ArrowDown, Enter, Space | **File** control | open the menu and focus its first enabled entry |
| ArrowUp | **File** control | open the menu and focus its last enabled entry |
| ArrowRight, Enter, Space | submenu row | open the submenu and focus its first enabled item |
| ArrowDown, ArrowUp | any entry | next / previous enabled entry in the same menu, wrapping |
| Home, End | any entry | first / last enabled entry in the same menu |
| ArrowLeft | submenu entry | close the submenu, focus its row |
| Escape | submenu entry, or the row of an open submenu | close the submenu, focus its row; the menu stays open |
| Escape | first level | close the menu, focus **File** (unchanged) |
| Tab / Shift+Tab | any focusable element inside the open menu, at any level | close the whole menu and rest focus on **File**; the browser's own Tab move then goes on from there (forward to what follows File, backward to what precedes it) |

Escape is innermost first: a submenu closes before the menu. The Tab rule is
decided by DOM containment, not by the React tree: a dialog an entry renders into the
document body (comparison, About, import, support) is outside the menu even though
React bubbles its key events through the portal, so Tab there keeps its own meaning
(the dialog traps it). Focus that leaves the menu entirely (a file picker, a dialog)
does not close a submenu. A submenu opened by click or key still closes when focus
moves to another entry of the menu.

## Focus after an action

The activation handler (`handleFileMenuActivation` in `App.tsx`) closes the menu and
moves focus to the visible **File** control in the same click, before the action's
dialog mounts. The entry that was activated is hidden with the menu, so without this
every dialog would capture a hidden element as its restore target and focus would fall
to `<body>` on close. With it, App-owned and child-owned dialogs alike (About, Project
history, Introduction, Achievements, support and diagnostic reports, documentation,
debug report, the comparison result) return focus to **File** on Escape and on their
close button. No entry opts out: only the submenu rows carry `data-menu-stays-open`.

## Pointer

- Hover opens a submenu after 120 ms and a hovered sibling replaces it; the old one
  closes. Only a mouse hovers; touch and pen open by tap.
- A hover-opened submenu closes 320 ms after the pointer rests on another first-level
  row or leaves the menu. Moving diagonally from the row into the panel crosses the
  menu's padding or a sibling row; entering the panel cancels both the pending close
  and a sibling's pending open, so the submenu is not lost on the way. This is a
  grace delay, not a safe triangle.
- A click opens a submenu and pins it (hover no longer closes it); clicking the row
  again closes it. Clicking a hover-opened row pins it instead of closing it.
- An item click runs the original handler once and closes the whole menu. The row
  itself carries `data-menu-stays-open`, the existing opt-out of the activation
  handler, so opening a submenu never closes the menu.
- An outside click closes the menu and with it the submenu.

## Placement

- Default: the panel opens to the right of its row, top-aligned with it.
- No room on the right and the whole panel fits on the left: it flips
  (`data-side="left"`) and slides in from the other side.
- Room on neither side (mid widths just above the 650 px breakpoint; verified at 700 px): it stays on the right and gives up
  width (down to 160 px) instead of leaving the screen. A panel running past the
  bottom edge moves up, and scrolls inside itself when taller than the screen.
- At 650 px width and below, or 600 px height and below (a short window, or high zoom),
  the panel is not a flyout but an inline expansion under its row (static position,
  indented, left rule), because a flyout would leave the viewport or be clipped by the
  scrolling menu around it. The chevron points down, or up when open. The menu then
  scrolls as a whole (`max-height: calc(100dvh - 80px)`), so **Logout** and **Quit**
  stay reachable on a short screen. In flyout mode (taller than 600 px and wider than
  650 px) the menu does not scroll; its own height is far below that limit.
- Positions are measured when a panel opens, on window resize and whenever the open
  panel's own size changes (a `ResizeObserver`, disconnected on close): a panel whose
  items change (a project opens) can push it past an edge. Placement runs only when the
  stylesheet made the panel absolutely positioned.

## Motion

A panel enters with opacity and a 6 px horizontal slide (from the side it opens
toward); inline panels fade only. Duration is 0.7 × `--knx-transition-duration` (140 ms at Standard,
84 ms at Subtle) with the motion style's easing. Motion level **Off** (0 ms) leaves no time to animate and OS
reduced motion removes the animation entirely; either way the panel appears at once. The animation rule sits
inside `prefers-reduced-motion: no-preference`, so reduced motion cannot be
re-enabled by a theme.

Styling reuses the Controls tokens (`controls-and-separators.md`): rows are ordinary
buttons that answer a press with colour only (`--button-press-scale: 1`), the open
row takes an accent tint, the panel is a surface with `--knx-border` and
`--knx-overlay-shadow`, all theme tokens. No new token, no theme-pack
change.

## Tests

- `src/FileMenu.test.tsx` (component): roles, keyboard paths (open, arrows, Home/End,
  ArrowLeft, Escape order, Escape on the parent row, Tab and Shift+Tab closing the
  menu from every kind of focusable descendant and leaving a portalled dialog alone,
  ArrowDown/ArrowUp on the File control), declared entries only, roving tabindex,
  disabled items, hover intent and the diagonal path with fake timers, sibling
  exclusivity, reset on menu close, placement flip/shrink/shift/inline, re-placement
  and disconnect through a `ResizeObserver`, items kept mounted, each original handler
  called exactly once.
- `src/App.test.tsx`: the real menu: first-level entries, submenu labels (EN, DE),
  per-level declared entries, one tab stop per level, disabled states without a project,
  native/web export difference, Logout/Quit separators, the comparison opened from
  History as a dialog (menu closed, Escape, focus on File), menu closing after an item
  runs, focus handed back to File after a dialog.
- `src/ProjectDiffPanel.test.tsx`: the comparison flow, counts, details and refusal, now
  asserted in the dialog; the dialog itself (modal, labelled, outside the menu, Escape
  and Close restore focus, Tab trapped) and the entry button alone in the menu.
- `e2e/file-menu-submenus.e2e.ts` (Chromium): right placement and viewport fit, EN/DE;
  keyboard flow with real focus; outside click; hover intent and diagonal move with a
  real mouse; flip near the right edge; fit at 700 px; inline expansion at 400 px;
  animation name/duration standard, Off, reduced motion; legibility in Porcelain,
  Graphite, Cupertino, LCARS and CRT; the menu-button pattern (aria state, one tab
  stop, ArrowDown/ArrowUp/Enter on File, Tab and Shift+Tab); only declared entries are
  focusable inside the open menu; Tab and Shift+Tab from every entry of the first level
  and of each submenu close File; focus back on File after About, Project history and
  Diagnostic export close by Escape and by their button; the comparison result opened
  from History as a dialog that fits an 800 x 640 viewport with all tables expanded,
  traps Tab and returns focus to File on Escape and on Close; a `.knxproj` comparison
  whose ETS import report (a `<details>` disclosure) is reached by Tab and by Shift+Tab
  and opens on Enter; a 1440 x 500 viewport where **Logout** stays reachable.
- `src/Overlay.test.tsx`: the shared focus trap counts a visible disclosure `summary`,
  `contenteditable` regions and media with `controls` as tab stops, and skips the
  contents of a closed `details`, `display: none` and `visibility: hidden` elements.
- Older specs that open an entry now open its submenu first through
  `e2e/file-menu.ts`.

## Not covered

- No type-ahead.
- After a Tab out of the menu, focus continues from **File**, not from the entry that
  had focus (the entry hides with the menu).
- Native WebKitGTK/Tauri was not run; the behaviour is the DOM's and was verified in
  Chromium and happy-dom only.
