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
| Submenu **History & comparison** | **Project history**, **Compare with…** (and the comparison report) |
| Submenu **Help & diagnostics** | debug report, support-gap analysis, device support report, diagnostic export, introduction, achievements (while switched on), About |

Native and web differences are unchanged because the conditions are unchanged:
the browser export is absent in the desktop shell, **Quit** exists only there,
**Logout** only with a session. **Save** stays on the toolbar. A submenu row is
always enabled (it is a place, not an action); the entries inside keep their own
disabled states, so with no project open **Project history** and **Compare with…**
are present and disabled.

The submenu markup lives in `apps/knx-web/src/FileMenu.tsx` (`FileMenuContent`,
`FileSubmenu`); `App.tsx` only chooses which entries go where. The menu is still
a native `<details>`, so the three existing closers (Escape, outside click, any
activation) keep working and every child component keeps finding its summary for
focus restoration.

## What a closed submenu keeps

A folded submenu is `hidden`, not unmounted. Several entries own state that must
outlive the panel: the selective-import, documentation, debug-report and support
dialogs live in their button components, and the comparison report renders inside
the History panel. Unmounting would have discarded an in-flight import or a shown
report when the pointer drifted away. A reopened File menu starts with every
submenu folded (the `open` attribute of the `<details>` is observed), but nothing
inside is reset.

## ARIA

- The first-level list is `role="menu"`; every action button in it and in the
  submenus is tagged `role="menuitem"`. Tagging is done by the menu, not by the
  six components that render buttons into it, so a button added later is covered
  too. A button inside a `role="group"` (the comparison report) is content and
  stays a button.
- A submenu row is a `menuitem` with `aria-haspopup="menu"`, `aria-expanded` and
  `aria-controls`; its panel is a `role="menu"` labelled by that row.
- Because the entries are `menuitem`s, a browser test finds them with
  `getByRole("menuitem", …)`, not `getByRole("button", …)`; the closed File menu is not
  in the accessibility tree at all. Entries inside a folded submenu are hidden until
  its row is opened (`openFileSubmenu` in `e2e/file-menu.ts`).
- Items stay real buttons with their natural tab order (no roving `tabindex`, so a
  Tab walk reaches everything, as before). Arrow keys are an addition.
- Disabled items are skipped by arrow navigation, as disabled buttons cannot
  take focus.

## Keyboard

| Key | On | Effect |
| --- | --- | --- |
| ArrowRight, Enter, Space | submenu row | open the submenu and focus its first enabled item |
| ArrowDown, ArrowUp | any item | next / previous enabled item in the same menu, wrapping |
| Home, End | any item | first / last enabled item in the same menu |
| ArrowLeft | submenu item | close the submenu, focus its row |
| Escape | submenu item, or the row of an open submenu | close the submenu, focus its row; the menu stays open |
| Escape | first level | close the menu, focus **File** (unchanged) |
| Tab / Shift+Tab | submenu item | leaves into the next control; a submenu opened by click or key closes when focus moves to another entry of the menu |

Escape is innermost first: an open comparison report closes before its submenu,
the submenu before the menu. Focus that leaves the menu entirely (a file picker, a
dialog) does not close a submenu, because the report renders inside it.

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
- At 650 px and below the panel is not a flyout but an inline expansion under its
  row (static position, indented, left rule), because a flyout would leave the
  viewport. The chevron points down, or up when open. The menu then scrolls as a
  whole (`max-height: 70dvh`); on wide screens it does not scroll, since a flyout
  inside a scrolling box would be clipped.
- Positions are measured when a panel opens and on window resize, and only when the
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
  ArrowLeft, Escape order, Escape on the parent row, Tab), disabled items, hover
  intent and the diagonal path with fake timers, sibling exclusivity, reset on menu
  close, placement flip/shrink/shift/inline, late-added buttons tagged, items kept
  mounted, each original handler called exactly once.
- `src/App.test.tsx`: the real menu: first-level entries, submenu labels (EN, DE),
  per-level item lists, tab order, disabled states without a project, native/web
  export difference, Logout/Quit separators, report-before-submenu-before-menu Escape,
  menu closing after an item runs.
- `e2e/file-menu-submenus.e2e.ts` (Chromium): right placement and viewport fit, EN/DE;
  keyboard flow with real focus; outside click; hover intent and diagonal move with a
  real mouse; flip near the right edge; fit at 700 px; inline expansion at 400 px;
  animation name/duration standard, Off, reduced motion; legibility in Porcelain,
  Graphite, Cupertino, LCARS and CRT.
- Older specs that open an entry now open its submenu first through
  `e2e/file-menu.ts`.

## Not covered

- ArrowDown on the closed **File** summary does not open the menu (Enter/Space and
  click do); Tab walks it as before.
- No type-ahead.
- Native WebKitGTK/Tauri was not run; the behaviour is the DOM's and was verified in
  Chromium and happy-dom only.
