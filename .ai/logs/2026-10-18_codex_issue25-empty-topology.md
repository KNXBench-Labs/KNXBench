# Issue #25: hide device-empty topology ranges

## Scope and root cause

The topology renderer mapped every projected area and line directly, including
ranges without devices. The filter belongs only to the diagram, not to the
canonical project, projection, exporter or project explorer.

## Implementation

- Render a line only when its device list is nonempty; render an area only when
  at least one of its lines is populated. Zero-object devices still count.
- Retain the unassigned-device card and an honest EN/DE installation empty state.
- Keep selection-driven editors and header creation entries on the complete tree.
- Update the existing contract and operating chapter, marking the change as
  source-only after Alpha 14.

## Observed verification

- Named RED: the original renderer showed three areas instead of one.
- Corrected focused component suite: 22 passed, zero failed.
- Guarded offline browser run: 23 default-fixture cases and eight production
  structure cases passed; complete/success/input-freeze flags all true.
- Wide and narrow German topology screenshots inspected: only populated Lights
  and Blinds cards remain; counts readable, no clipped metric text.
- Independent review found a stale sibling hover fixture and missing explicit
  hidden-line reachability proof. Fixture now has a fictional zero-object device;
  original hover/icon assertions are intact. New component regression proves
  hidden-line editor and exact catalog target, with unchanged project bytes.
- Independent re-review accepted the complete corrected diff without findings.

## Remaining acceptance

The integrated full gate, publication, issue reply and closure are not yet done.
Two queued browser attempts were canceled before any stage executed, and do not
count as product verdicts. The additional production topology case set will run
on the integrated candidate rather than its superseded branch basis. No hardware,
bus, container, release or deployment action.
