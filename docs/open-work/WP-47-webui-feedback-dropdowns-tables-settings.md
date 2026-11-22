# WP-47 — WebUI feedback 19 Nov 2026: dropdowns, table headers, settings layout

**Release:** the delivered parts of this package ship in Alpha 23 ([release notes](../releases/alpha23.md)).

Origin: owner UI feedback, 19 November 2026 (WebUI session, four
screenshots). Intake PR !79 (`64c752286`) was documentation-only; all three
bounded packages were implemented and merged afterwards (see Delivery below).
Not tied to a GitHub issue.

## Delivery status (20 Nov 2026, all on `origin/main`)

| Package | PR | Merge commit | Covers |
| --- | --- | --- | --- |
| WP-47.1 bus monitor destination dropdown (scalar slot, single-line marquee rows) | !81 | `23b27720b` | Points 1, 2, 3, 8 |
| WP-47.2 sticky bold header + free column sorting on the bus monitor table | !83 | `72eaa7eca` | Points 4, 5, 6 |
| WP-47.3 settings layout, redundant header gear removed, fields capped at 25–30 ch | !89 | `f97b75914` | Points 7, 9, 10 |

## Requested outcome (owner points 1–10, verbatim grouping)

1. Bus Monitor: the "Ziel" (destination) dropdown is far too large (screenshot 1).
2. Dropdown content wraps to multiple lines; on overflow it should scroll
   as a marquee instead.
3. Dropdown fields regularly jump back to the start instead of staying at the
   scroll position the user scrolled to.
4. Table headers: the top header row must always stay visible (sticky) and
   be bold.
5. Tables: columns must be freely sortable.
6. Table headers: there must always be some spacing/padding to the start of
   the cells.
7. Configuration tab: no clean layout; align the design with the rest of
   the app (screenshot 2).
8. Screenshot 3: the whole dropdown field scrolls — wrong; only the content
   inside should scroll.
9. The settings button top-right in the app can be removed.
10. In settings, dropdown/text fields are far too wide; cap them at 25–30
    characters (screenshot 4).

## Current basis (source anchors on `origin/main` `40c057e2d`)

- Bus monitor destination/columns: `apps/knx-web/src/BusMonitorPanel.tsx:46`
  (`destination` column, width 160); monitor table scroller
  `.monitor-table-scroll` (`styles.css:4123`).
- Sticky table headers already exist for some tables
  (`.devices-table th`, `.com-table thead th` sticky, `styles.css:3637/3955`;
  catalogue table `catalogTable.css` with sticky headings + sort buttons) —
  the affected monitor/other tables need the same treatment plus bold
  headings and header/cell padding.
- Sortable columns: catalogue table already has sort buttons
  (`CatalogResultsTable.tsx`, WP-24); device/com tables use the resizable
  heading hook (`styles.css:5539`) — free sorting is the new part.
- Settings gear button top-right: `apps/knx-web/src/App.tsx:1734` (header
  `<GearIcon />` button); a second settings entry already exists in the
  left navigation (`App.tsx:1787`, `settings-navigation`), so the header
  button is redundant.
- Settings fields: `SettingsPanel.tsx` (`.settings-field select`,
  `styles.css:1253`); WP-45 delivered `CompactSelect` + `.select-compact`
  (10 rem slot, hover marquee) — the settings fields should reuse that
  component at the requested 25–30 ch cap.
- WP-45 hover marquee and WP-41 shared 10 rem scalar slot are the
  authoritative mechanisms for points 2, 8 and 10; WP-44 sticky details pane
  is the pattern for point 4.

## Bounded packages

| Package | Covers | Outcome |
| --- | --- | --- |
| WP-47.1 Bus monitor destination dropdown | Points 1, 2, 3, 8 | Cap the dropdown width (CompactSelect/10 rem slot), single-line entries with hover marquee on overflow (WP-45 mechanism), scroll position retained while open, only the list content scrolls — never the whole field |
| WP-47.2 Table header and sorting standard | Points 4, 5, 6 | Sticky + bold header row on the affected tables (pattern: `.devices-table th` / `catalogTable.css`), freely sortable columns, visible padding between header and cell content |
| WP-47.3 Settings/configuration tab layout | Points 7, 9, 10 | Align the settings layout with the shared workspace design (WP-41 density/layout system), remove the redundant top-right gear button (left-nav entry stays), cap settings dropdown/text fields at 25–30 ch via the shared compact field |

## Acceptance direction (for a future implementation Go)

- Verify in default and production browser builds at 100% and scaled UI;
  reduced-motion disables marquee (WP-45 guard).
- Keyboard/focus and screen-reader access to full dropdown values must not
  degrade; sticky headers must not cover the first rows (scroll-padding,
  cf. `catalogTable.css`).
- No domain/schema/storage/bus behaviour change; UI-only package.
