# Native import-diagnostic availability

Local additive API/UI implementation, 2026-10-11. No native-model migration.

## Meaning

`ProjectTree.import_report_available` is optional application metadata:

| State | Field | Counter interpretation |
| --- | --- | --- |
| Successful external import in the current session | `true` | Original import counts remain available across edit/undo/redo/save |
| Newly created project | `true` | Known initial zero, not an external ETS compatibility assessment |
| Native open, including recovery of its working history | `false` | Historical external import findings are unavailable; zero is not a compatibility pass |
| Pure projection / older response | omitted | Availability not asserted by that producer |

The native format does not restore a historical `ImportReport`. Native open is
not a fresh ETS import or a compatibility reassessment. The legacy numeric fields
remain for API compatibility; nonzero current-session findings must remain
visible even when the historical report is unavailable. Save As does not erase
live import evidence; opening the resulting native file in a new session marks
that evidence unavailable. Failed replacement must not publish a new availability
state. Availability is published with the replacement project under its lock.

## Presentation and verification

- Dashboard, Project Explorer and the workbench status control explicitly expose
  unavailable evidence in English and German.
- The status control opens the explanation rather than presenting a guessed
  warning count. It is keyboard-accessible and does not rely on a tooltip alone.
- The notice is informational, not a new measured warning/error. Known current
  counters remain displayed separately.
- Server lifecycle test covers new/measured state, Save As, native reopen,
  detached projection, edit, Undo, Save and GET.
- Dashboard tests cover EN/DE missing history, measured zero, omitted legacy
  metadata and nonzero current-session warnings alongside missing history.
- Full-workbench Chromium tests use typed fictional responses with all API
  requests intercepted, at 400/1440 px and Graphite/Porcelain themes. They check
  notice activation, explanation/Explorer display, measured-zero absence,
  overflow and unexpected requests/page errors. This is not real ETS acceptance.

## Not implemented

Persisting or reconstructing the original full import report, per-finding
historical provenance and an exhaustive native compatibility recomputation are
separate work. This change deliberately does not manufacture missing evidence,
change retained source data, or claim unsupported features became supported.
