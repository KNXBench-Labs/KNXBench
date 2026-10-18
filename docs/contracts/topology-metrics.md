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

| Language | Visible text | Hover text |
| --- | --- | --- |
| EN | `0 objects`, `1 object`, `12345 objects` | `1 communication object`, `12345 communication objects` |
| DE | `0 Objekte`, `1 Objekt`, `12345 Objekte` | `1 Kommunikationsobjekt`, `12345 Kommunikationsobjekte` |

Keys `workbench.objectCount` and `workbench.objectCountTitle` are plural pairs
(`.one`/`.other`) resolved by the existing `Intl.PluralRules` helpers. The hover text
is an addition; the visible label carries the meaning on its own.

## Layout

- `.topology-lines` is `repeat(auto-fill, minmax(min(100%, 230px), 28rem))` with
  `justify-content: start`: a line card never exceeds 28rem, several lines sit side
  by side from the left on a widescreen, and one card still fills a narrow viewport.
- The unassigned card (outside the grid) has the same 28rem ceiling.
- `.diagram-devices` is one `minmax(0, 1fr)` column capped at 32rem (this also bounds
  the Buildings tile rows). The name column shrinks (`min-width: 0`), the name
  ellipsises and carries the full name as `title`; the icon and the count never shrink
  or wrap.
- Hierarchy, selection (`aria-pressed`), keyboard arrows and the inspector are
  unchanged. The view stays a set of cards, not a data table, so UI-05's column
  widths do not apply and no resize mechanism exists.

## Tests

`StructureWorkspace.test.tsx` (labels, plurals, empty line, large counts, EN/DE) and
`e2e/topology-metrics.e2e.ts` (1280, 2560 and 700 px, Porcelain and Graphite, one DE
run: label text, row and card width ceilings, name-to-count distance, ellipsis,
overflow, selection). CRT and LCARS were not part of the browser run; the rules use
no theme token beyond `--knx-muted`.
