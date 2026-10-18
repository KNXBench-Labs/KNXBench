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

- Hover is for flyouts only. A flyout opens after 120 ms of hover and a hovered sibling
  flyout replaces it; the old one closes. Only a mouse hovers; touch and pen open by tap.
- An inline submenu (the narrow and short-window layouts, and the per-instance
  `data-layout="inline"` below) is a disclosure: hover never opens it, never swaps it for a
  sibling and never closes it. It opens and folds by click, Enter/Space and
  ArrowRight/ArrowLeft; opening a second one by click folds the first. Reason: an inline
  panel moves the rows below it, so a resting mouse suddenly sits on another row, and the
  hover that follows used to replace the submenu the user had just clicked. Whether a
  submenu is inline is read from the rendered layout when the 120 ms elapse (the open panel's
  computed `position`; for a folded panel the same measurement `placePanel` makes, taken by
  unfolding it for one synchronous step), not from a copy of the breakpoint. Hover is
  ignored when the open submenu or the one it would open is inline.
- A hover-opened submenu closes 320 ms after the pointer rests on another first-level
  row or leaves the menu. Moving diagonally from the row into the panel crosses the
  menu's padding or a sibling row; entering the panel cancels both the pending close
  and a sibling's pending open, so the submenu is not lost on the way. This is a
  grace delay, not a safe triangle.
- A click opens a submenu and pins it (hover no longer closes it); clicking the row
  again closes it. Clicking a hover-opened row pins it instead of closing it.
- A press moves focus to the pressed row before the click, in the (compatibility) `mousedown`.
  While a press is in flight that focus move does not fold an open submenu, which would shift
  the rows so that the click lands on another row and is lost. The rule is the same for mouse,
  touch and pen: `pointerdown` and `mousedown` (capture, on the menu) both arm the guard, and
  `pointerup`, `pointercancel`, `mouseup` and `click` (capture, on the window) release it. Arming
  on `mousedown` is what covers a tap: Chromium sends a touch tap's (and a hover-less pen tap's)
  `pointerdown` and `pointerup` first and its `mousedown`, which moves focus, after them, so a
  guard held from `pointerdown` to `pointerup` alone would already be released. A press whose
  release never reaches the page (the button let go over another window) is forgotten when the
  window loses focus (`blur`), and any key in the menu clears it too. A keyboard focus move out of
  an open submenu still folds it.
- An item click runs the original handler once and closes the whole menu. The row
  itself carries `data-menu-stays-open`, the existing opt-out of the activation
  handler, so opening a submenu never closes the menu.
- An outside click closes the menu and with it the submenu.

## Placement

- Default: the panel opens to the right of its row, top-aligned with it.
- No room on the right and the whole panel fits on the left: it flips
  (`data-side="left"`) and slides in from the other side.
- One minimum width for a flyout, `MIN_PANEL_WIDTH_PX` (220 px) in `FileMenu.tsx`. The
  panel states it as `--file-submenu-min-width` and the stylesheet's `min-width` reads that
  property, so the CSS and the placement maths cannot drift apart (the clamp used to be 160 px
  against a 220 px `min-width`, and the CSS won).
- The whole panel fits neither side: it keeps the side with the most room that still offers
  at least the minimum and gives up width (`--file-submenu-max-width`, never below 220 px)
  instead of leaving the screen. A panel running past the bottom edge moves up, and scrolls
  inside itself when taller than the screen. The bottom check uses the panel's height after the
  cap: a capped panel wraps long entries and grows (it used to be checked before, and a capped
  panel at 150% ended up to 26 visual px below the window).
- The room on the right is measured from the row, not from the panel's own box: the panel
  starts at the row's right edge plus the 6 px gap (`SUBMENU_GAP_PX`, the stylesheet's `left`).
  Placement runs as the panel opens, while its entry animation still holds it at
  `translateX(-6px)`; measuring the panel's box there granted those 6 px a second time, so a
  capped panel ate 6 of the 8 px margin (at 100% as well; seen first at 150%).
- Neither side offers the minimum (the strip just above the 650 px breakpoint, where the
  menu leaves under 220 px on its right, and under zoom much wider windows: 960–990 px at 150%): this one submenu, for as long as it is open, uses
  the inline layout below (`data-layout="inline"` on its `.file-submenu`). It is re-decided on
  every placement, so widening the window gives the flyout back and closing the submenu drops
  the mark. While an inline submenu is open the menu scrolls as a whole, as it does in the
  narrow layout. `choosePanelPlacement` is the pure decision and is unit-tested.
- At 650 px width and below, or 600 px height and below (a short window, or high zoom),
  every panel is an inline expansion under its row (static position, indented, left rule),
  by stylesheet alone, because a flyout would leave the viewport or be clipped by the
  scrolling menu around it. The chevron points down, or up when open. The menu then
  scrolls as a whole (`max-height: calc(100dvh - 80px)`), so **Logout** and **Quit**
  stay reachable on a short screen. In flyout mode (taller than 600 px and wider than
  650 px) the menu does not scroll; its own height is far below that limit.
- Positions are measured when a panel opens, on window resize, whenever the open
  panel's own size changes (a `ResizeObserver`, disconnected on close): a panel whose
  items change (a project opens) can push it past an edge, and when the root's `style`
  changes (a `MutationObserver` on `<html>`, disconnected on close): a zoom step resizes
  neither the window nor the panel's layout box, but changes every visual reading.

## Units under zoom

The application scale (`--app-ui-scale`, 80–150%) is a CSS `zoom` on the root. Placement
works in one unit throughout: CSS layout px of the zoomed root, the unit of the stylesheet,
of `MIN_PANEL_WIDTH_PX` (220), the 6 px gap, the 8 px margin and of every length placement
writes back (`--file-submenu-max-width`, `--file-submenu-shift`). Readings that come in visual
px are divided by the scale (`appUiScale()`) before any comparison: the panel's and the row's
`getBoundingClientRect()` and the window's `innerWidth`/`innerHeight` (`layoutGeometry` in
`FileMenu.tsx`, a pure function). `choosePanelPlacement` and `choosePanelShift` take and return
layout px only. Written lengths are rounded to 1/100 px.

Viewport units do not follow the root zoom, so every File menu limit divides them by the
scale first: the flyout's `max-width` fallback `min(520px, calc(100vw / scale - 16px))` and
`max-height: calc(100dvh / scale - 24px)`, and the scrolling menu's
`max-height: calc(100dvh / scale - 80px)` (narrow, short and per-instance inline layouts).
The media queries (650 px wide, 600 px high) are not zoom-aware: they read the window in CSS
px, so at 150% a 990 px window still starts in the flyout layout and placement decides per
submenu (660 layout px wide: no side of the Import & export row keeps 220 layout px, so that
submenu goes inline; at 1040 px it flies out with a cap).

What Chromium reports inside the root-zoomed document was measured, not assumed (system
Chromium 154, 18 October 2026, a 900 x 700 window, a 200 x 100 px box at 100, 50):

| Reading | 100% | 150% | 80% |
| --- | --- | --- | --- |
| `document.documentElement.clientWidth` / `clientHeight` | 900 / 700 | 900 / 700 | 900 / 700 |
| `innerWidth` / `innerHeight`, `visualViewport` | 900 / 700 | 900 / 700 | 900 / 700 |
| box `getBoundingClientRect()` left, top, width | 100, 50, 200 | 150, 75, 300 | 80, 40, 160 |
| box `offsetWidth`, computed `width` | 200, 200px | 200, 200px | 200, 200px |
| `width: 100vw; height: 100dvh`: computed / rendered | 900 x 700 / 900 x 700 | 900 x 700 / 1350 x 1050 | 900 x 700 / 720 x 560 |
| `max-width: calc(100vw - 16px)`: rendered width | 884 | 1326 | 707.2 |
| pointer `clientX` at the box's DOMRect left + 10 | 110 | 160 | 90 |
| `matchMedia("(max-width: 650px)")` | false | false | false |

So the window size is visual (the window's own CSS px, never divided), DOMRect and pointer
coordinates are visual, computed lengths are layout px, and a viewport unit is a layout length
that the zoom then enlarges or shrinks. `elementFromPoint` takes visual coordinates (it hit the
box at its DOMRect centre at all three scales).

Native WebKitGTK/Tauri was not measured; if it reports DOMRect or the window size in another
unit under `zoom`, the conversion would be wrong there
([KL-130](../limitations/KL-130-ZOOM-application-zoom.md)). Placement runs only when the
  stylesheet made the panel absolutely positioned (or the previous placement made it
  inline: every pass starts by undoing the last one).

## Stacking

The open menu sits above the toolbar's controls and below modal dialogs. The toolbar is a
stacking context (`z-index: 5`), so the menu's level is only compared with the controls
inside it. A disabled control (`opacity` below 1) forms a stacking context of its own and
paints in document order among positioned boxes; the menu used to have no level, so the
disabled Search and Save buttons that follow File painted over the **New project…** and
**Open project…** rows, hiding their labels and stealing the pointer. `.file-menu-content`
now has `z-index: 6`. A dialog (`.search-overlay`, `z-index: 10`) belongs to the page's root
stacking context and outranks the whole toolbar, menu included. A flyout panel keeps its
own level inside the menu.

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
  called exactly once; no hover opening or swapping in a stylesheet-inline and in a
  per-instance inline layout, while a flyout still opens on hover.
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
- `e2e/file-menu-submenus.e2e.ts`, UISET-02: at 651, 660, 700, 760 and 900 px wide (800 px
  high) and at 900 x 560 and 760 x 601, in EN and DE, every submenu is opened and its box and
  every entry lie inside the viewport, the pointer at each entry's centre lands on that entry
  and each enabled entry passes Playwright's real-pointer hit check; one activation per
  window, all three submenus in one menu by consecutive real clicks. The panel's stated
  minimum equals its computed `min-width`.
- `e2e/file-menu-submenus.e2e.ts`, UISET-02 round 2, at 400 x 800, 900 x 560 and 660 x 800
  (per-instance inline), in EN and DE: after a real click on one submenu the real mouse rests
  more than 400 ms on the next row; the open submenu stays open and the next one stays
  closed; a real click then opens the next one and folds the first. With nothing open a resting mouse opens nothing, and
  ArrowRight/ArrowLeft/Enter/Space/Escape still work. At 1440 x 900 hover still opens and
  swaps a flyout.
- `e2e/file-menu-submenus.e2e.ts`, UISET-03: at 400 and 1440 px in Porcelain and Graphite,
  with no project, `document.elementFromPoint` at the centre and at the text of every
  first-level entry returns the entry; real clicks on **New project…** and **Open project…**
  reach their handlers; with a modal open, the page's `inert` lifted and the menu forced open,
  toolbar controls and menu rows still lie under the overlay.
- `src/FileMenu.test.tsx`, UISET2-01 (units under zoom): `layoutGeometry` divides every visual
  reading by the scale and takes the panel's left edge from its row; at 150% a side with 309
  visual (206 layout) px goes inline, a cap is written as the 236 layout px of room, not 354
  visual; at 80% 181 visual (226 layout) px keep the flyout with a 226 px cap; at 150% and 80%
  the shift is the 58 layout px of overflow; an open panel is placed again when the root's
  scale changes; a cap is measured beside the row while the slide-in still holds the panel 6 px
  to the side; the shift is measured after the cap wraps the entries. `choosePanelShift` is pure.
- `src/FileMenu.test.tsx`, UISET2-02 (press guard): touch and pen taps in Chromium's order
  (`pointerdown`, `pointerup`, then `mousedown` and the focus move) and in mouse order keep the
  open submenu until the click lands; a press abandoned outside the window (`blur`) and a
  cancelled touch press (`pointercancel`) are forgotten, so a later focus move folds again.
- `e2e/file-menu-submenus.e2e.ts`, UISET2-01: the scale comes from a stored `uiScale` setting
  through App (and once through Ctrl+Plus, the settings path, with a submenu open), never from a
  direct `--app-ui-scale` write. At 150% at 960, 990, 1010, 1040, 1080 and 1440 x 900 and in the
  short windows 1440 x 620 and 1100 x 640, and at 80% at 660, 690, 720 and 760 x 800 and
  900 x 601, in EN and DE: every first-level entry and every entry of every submenu, scrolled into
  view, lies inside the viewport and takes the pointer; a flyout is at least 220 layout px wide,
  inside the viewport and above the bottom margin, its `max-height` is the viewport's in layout
  px, a capped one ends exactly at the margin of its side and its cap times the scale is its
  width, a moved one ends at the bottom (or top) margin; an inline one appears only where
  neither side of its row holds 220 layout px (the test's own measurement); no sideways scroll.
  The panel is read after its entry animation finished.
- `e2e/file-menu-submenus.e2e.ts`, UISET2-02: on a touchscreen (`hasTouch`, `locator.tap()`) at
  400 x 800 and 660 x 800 in EN and DE, a tap on a sibling row opens it while another submenu
  is open, a tap on **Logout** below an open submenu runs it, and a tap on **New project…**
  with a submenu open opens its dialog. With a pen (Chromium's input domain,
  `Input.dispatchMouseEvent` with `pointerType: "pen"`, which arrives in mouse order; the page
  sees `pointerType` `pen`) at the same widths: a sibling row opens and **Logout** runs.
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
  Chromium and happy-dom only. The units under zoom were measured in Chromium only.
- At 80% the stylesheet's narrow rule takes over (650 px window) before any side of a row can
  fall near 220 layout px, so the 80% browser cases exercise the converted CSS limits; the 80%
  JavaScript conversion near the minimum is covered by unit tests only.
- A pen with hover, and a pen tap in the touch-like order (`mousedown` after `pointerup`), were
  not produced in a browser: Playwright emulates no pen, and Chromium's input domain delivers a
  pen press in mouse order. The touch-like pen order is covered by unit tests only.
- The media queries themselves are not zoom-aware (see Units under zoom).
