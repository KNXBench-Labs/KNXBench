# Project-diff table presentation

[Current summary and ledger status](../KNOWN_LIMITATIONS.md#catalogue-kl-060)

This dossier preserves the technical detail and dated history. Earlier
headings, failures and lifting conditions are historical where later
updates supersede them; they are not new work assignments.

<!-- BEGIN retained catalogue record -->

## 60. Project diff's web panel lists entities, with filtered, virtualised long tables

**Status.** Largely lifted (CT-1, 2026-09-28). `ProjectDiffPanel.tsx`
keeps the grouped-count summary lines and adds, below them, one
collapsed-by-default disclosure per non-empty table and installation
(`ProjectDiffDetails.tsx`, projection in `projectDiffView.ts`). Expanded,
a table lists each added, removed, changed and ambiguous entity by its
natural key (area `1`, line `1.2`, device address plus ETS id, group
address in the user's notation plus ETS id, group-range bounds, building
path). A changed entity shows how it was matched (ETS id or natural key)
and its before/after table; an ambiguous one shows its candidate counts
on each side; a changed device nests its communication-object and
parameter tables the same way.

**Accessibility.** Disclosures are native buttons with `aria-expanded`/
`aria-controls`, so Enter and Space come from the platform. Every status
is spelled out as a word and prefixed by a symbol (`+`, `−`, `~`, `?`);
colour only reinforces it. English and German catalogues cover every new
string. Escape still closes the report first, as before.

**Long tables (KL-60, lifted 2026-10-04).** Up to 20 entries, a table
stays a plain list. Above that it gets a search field (key and name, case-
insensitive), status toggle buttons (`aria-pressed`, shown when more than
one status occurs), a live match count (`role="status"`) and a bounded
scroll viewport. The viewport renders only the rows near the visible area
(`virtualWindow.ts`); every entry stays reachable by scrolling, and each
rendered row carries `aria-posinset`/`aria-setsize`. Row heights are
measured as rows render (re-measured via `ResizeObserver` when a nested
table opens); unseen rows count with an estimate. Two corrections keep the
view steady: growth of rows above the visible area is added to
`scrollTop`, and a list at its end stays at its end. End/Home on the focused
viewport jump instantly, because Chromium's smooth native jump stopped
short once newly measured rows grew the list. Verified in Chromium on a
synthetic 3,300-entry table with variable row heights
(`e2e/diff-virtual.e2e.ts`): DOM stays under 120 rows, every position
1…3,300 is reached by scrolling, End reaches the last row, and an anchor row
moves by exactly the scrolled distance while rows above it are measured.

**What remains.**

- **Search covers one table's own rows.** Nested tables (a changed
  device's communication objects and parameters) are not searched from
  the device table; they have their own filter once they exceed 20 rows.
  There is no search across tables and no jump from a diff row to the
  entity in the Project Explorer.
- **Focus and Tab follow the rendered window.** A row that scrolls out of
  the window unmounts, so focus inside it is lost; Tab walks only through
  rendered rows, and the others are reached by scrolling the focused list
  (arrows, Page Up/Down, Home/End). Escape clears a typed filter first and
  closes the report only on an empty field.
- **A nested table's filter text is per mount.** Expansion of nested
  tables is remembered while their row scrolls out of the window (UI state
  keyed by table id for the shown report); a typed nested filter is not.
- **Keyboard activation of disclosures is verified structurally in
  Vitest** (happy-dom does not synthesize a button's Enter/Space
  activation, so the tests reproduce the browser's rule). The long-table
  viewport's keyboard scrolling is verified in Chromium. No screen reader
  was used to check the panel.
- The panel never applies or merges a diff (§55) and has no three-way
  mode (§56). Raw `.knxproj` inputs are accepted since CT-6 (§57).

**Alpha decision (user, 2026-10-04, AR11).** Paging is not accepted for
the Alpha: the UI owner replaces it with virtualised tables plus search/
filter inside the diff view before the Alpha. The backend diff API does
not change for this. Delivered 2026-10-04 as described above.
<!-- END retained catalogue record -->
