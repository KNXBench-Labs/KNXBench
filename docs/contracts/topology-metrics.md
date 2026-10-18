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
hover text to decode it. The count is never truncated or wrapped.

## Layout

- `.topology-lines` is `repeat(auto-fill, minmax(min(100%, 230px), 28rem))` with
  `justify-content: start`: a line card never exceeds 28rem, several lines sit side
  by side from the left on a widescreen, and one card still fills a narrow viewport.
- The unassigned card (outside the grid) has the same 28rem ceiling.
- `.diagram-devices` is one `minmax(0, 1fr)` column capped at 32rem (this also bounds
  the Buildings tile rows). The name column shrinks (`min-width: 0`), the name
  ellipsises and carries the full name as `title`; the icon and the count never shrink
  or wrap. `.diagram-devices` is a size container: at 26rem inner width or less the
  count drops onto its own line under the name (still left-aligned with the name),
  above that it sits at the right edge. Only the name may ellipsise.
- Hierarchy, selection (`aria-pressed`), keyboard arrows and the inspector are
  unchanged, except that ArrowUp/ArrowDown/Home/End skip the entries of the closed
  `+ New` menu (they are not rendered, so focusing one dropped focus). The view stays a set of cards, not a data table, so UI-05's column
  widths do not apply and no resize mechanism exists.

## Tests

`StructureWorkspace.test.tsx` (labels, plurals, empty line, large counts, EN/DE) and
`e2e/topology-metrics.e2e.ts` (1280, 2560, 700 and 480 px, Porcelain and Graphite, DE at
2560, 700 and 480: full label text, count neither clipped nor wrapped nor overlapping
the name, side by side from 1280 px, row and card width ceilings, ellipsis, overflow,
selection) and its keyboard cases (arrows, Home/End move focus; Enter and Space select a
line heading and the inspector follows). The unit test drives the same arrow/Home/End
handler with real key events. CRT and LCARS were not part of the browser run; the rules use
no theme token beyond `--knx-muted`.
