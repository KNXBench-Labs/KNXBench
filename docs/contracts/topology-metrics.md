# Topology metrics and bounded layout

UI-06 of the [UI feedback set](../goals/ui-feedback-2026-10-18.md) (owner request 9,
18 October 2026). Presentation only: no API, schema, stored-data or protocol change.

## What the number is

The figure at the right edge of a device tile in `StructureWorkspace.tsx` is
`DeviceNode.com_object_count`. The projection sets it from `device.com_objects.len()`
(`crates/knx-projection`), so it counts every communication object the device holds,
active or not. It is not a device count, an address part or a line number. The same
tile is used by the Topology view and the Buildings view, so both carry the label.

## Label

| Language | Visible text |
| --- | --- |
| EN | `0 communication objects`, `1 communication object`, `12345 communication objects` |
| DE | `0 Kommunikationsobjekte`, `1 Kommunikationsobjekt`, `12345 Kommunikationsobjekte` |

Key `workbench.objectCount` is a plural pair (`.one`/`.other`) resolved by the existing
`Intl.PluralRules` helpers. The visible text names the metric on its own; there is no
hover text to decode it. The count is never truncated; above the 15rem
container breakpoint it is not wrapped either (see Layout).

## Layout

- `.topology-lines` is `repeat(auto-fill, minmax(min(100%, 230px), 28rem))` with
  `justify-content: start`: a line card never exceeds 28rem, several lines sit side
  by side from the left on a widescreen, and one card still fills a narrow viewport.
- The unassigned card (outside the grid) has the same 28rem ceiling.
- `.diagram-devices` is one `minmax(0, 1fr)` column capped at 32rem (this also bounds
  the Buildings tile rows). The name column shrinks (`min-width: 0`), the name
  ellipsises and carries the full name as `title`; the icon and the count never shrink
  and, above 15rem, never wrap. `.diagram-devices` is a size container: at 26rem inner width or less the
  count drops onto its own line under the name (still left-aligned with the name),
  above that it sits at the right edge. Only the name may ellipsise. At 15rem or less
  (a 200px nested Buildings card, whose ancestors clip overflow) the indent goes and the
  label may wrap between number and words, so the count is never cut off.
- Hierarchy, selection (`aria-pressed`), keyboard arrows and the inspector are
  unchanged, except that ArrowUp/ArrowDown/Home/End skip the entries of the closed
  `+ New` menu (they are not rendered, so focusing one dropped focus). The workspace
  handler ignores keys already handled (`defaultPrevented`), so the open menu keeps its
  own first/last wrapping. The view stays a set of cards, not a data table, so UI-05's column
  widths do not apply and no resize mechanism exists.

## Device-empty ranges (issue #25, source builds after Alpha 14)

The Topology diagram renders only lines that contain at least one device and
areas with at least one such line. A device with zero communication objects
still counts as a device. The separate unassigned-device card remains visible
when populated. An installation without any devices shows a localized empty
state explaining the hidden ranges and the project-tree editing route.

This is a view-only filter over the projection: no area, line, device, address,
identifier or source metadata is removed or changed. Empty areas and lines stay
available in the project tree; the header's creation menu and an explicitly
selected range's property editor remain usable. Adding a device reveals its
line and area on the next project update; removing its last device hides them
again. Each installation is filtered independently. Buildings and group-address
views are unaffected.

## Tests

`StructureWorkspace.test.tsx` (labels, plurals, hidden empty lines/areas, live
project updates, multiple installations, unassigned devices, preserved source,
localized empty states, large counts, EN/DE) and
`e2e/topology-metrics.e2e.ts` (1280, 2560, 700 and 480 px, Porcelain and Graphite, DE at
2560, 700 and 480: full label text, count neither clipped nor wrapped nor overlapping
the name, side by side from 1280 px, row and card width ceilings, ellipsis, overflow,
selection), a German and English Buildings case (200px nested card, five-digit count shown in
full, no clipping ancestor) and its keyboard cases (arrows, Home/End move focus; Enter and Space select a
line heading and the inspector follows). The unit test drives the same arrow/Home/End
handler with real key events, and checks the open `+ New` menu wraps first/last with focus
kept inside it. CRT and LCARS were not part of the browser run; the rules use
no theme token beyond `--knx-muted`.
