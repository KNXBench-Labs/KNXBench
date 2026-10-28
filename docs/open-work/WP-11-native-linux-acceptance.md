# WP-11 — Native Linux acceptance

**Status:** In progress: first bounded native slice, two positive editor cells, bounded capture export and real-pointer drag/hover cells recorded; one pane-drag defect found and open; remaining cells open. Listed in [OPEN_WORK](../OPEN_WORK.md); per-ID status of record is the [ledger](../status/LEDGER.md).

## Task

Record actual native behaviour for named workflows with fictional or
simulator data (no bus contact needed):

- WebKitGTK keyboard, focus, draft and dialog behaviour, including the
  notes, instruction and checked-parameter editors.
- Orca / screen-reader behaviour.
- Wayland save/cancel/overwrite paths and the bus-monitor capture export.
- Application zoom cases on WebKitGTK (KL-130; bounded native controls now
  recorded, remaining pointer/layout/hover matrix open).
- AppImage startup smoke on a newer-Mesa host (issue #6).
- Firefox for the app.

## Consent and scope — 27 October 2026

The owner explicitly authorises desktop control for these bounded native tests
with isolated fictional/simulator data, including keyboard/focus, dialogs, zoom,
save paths and screen-reader checks where available. No bus access or changes
to real projects are authorised. Consent is granted; the first bounded native Alpha 17 slice below is
separate executed evidence, not consent alone. Missing environments remain
explicitly untested.

## First bounded slice recorded — 27 October 2026

The published Alpha 17 AppImage was exercised in an actual isolated native
Wayland/WebKitGTK window. Save/Cancel/Replace, exact note save/reopen before and
after process restart, settings focus wrap/return, 80/150% shortcuts, bounded
keyboard table/pane controls and File submenus have resulting state/file
evidence. See [acceptance and exclusions](../status/2026-10-27-wp11-native-acceptance.md).

At that first cutoff the fictional demo only established checked-editor
`unsupportedXml` refusal and instruction `MissingSource` feedback, not positive
editing. Those historical outcomes remain refusal evidence.

## Positive native editor cells — 27 October 2026

A separate [positive continuation](../status/2026-10-27-wp11-positive-native-editors.md)
uses independently reviewed original fictional source-20 data. It verifies one
declared integer slider's Cancel/Apply/Save/Undo/Redo and native-created-device
plain instruction preview/cancel/confirm/save. Read-only saved-result oracles
and a real native process restart verify slider `7` and exact instruction text.
This is not all checked controls or imported-original instruction editing.
Virtual-key replacement remains unestablished; physical-keyboard comparison
and product-cause attribution are missing.

## Bounded native capture export — 27 October 2026

The [capture continuation](../status/2026-10-27-wp11-native-capture-export.md)
uses one source-bound fictional UDP peer and the actual Alpha 17 backend,
not mocked Tauri/HTTP. Native Export Cancel creates no capture; Save writes
one real received row with JSON-v1 session/loss provenance; overwrite Cancel
preserves exact bytes/mtime and Replace changes only the export timestamp.
Actual heartbeat/disconnect and ordinary native/peer closure are recorded.
The original/replaced JSON and original-source fixture discrimination remain
separate retained evidence. This is one bounded capture cell, not protocol,
hardware, complete bus-history or whole-WP-11 acceptance.

Remaining work includes other controls, imported-original instructions,
virtual-key replacement attribution, keyboard/pointer/layout cells and
vendor-specific EGL/GPU evidence. Orca and Firefox remain untested; their
previously missing environments were not installed by this continuation.
No ledger row is closed.

## Real-pointer drag and hover cells — 28 October 2026

The [pointer continuation](../status/2026-10-28-wp11-native-pointer-cells.md)
drove the real compositor cursor at 80/100/150% with fictional data. Completed
navigation-pane drags follow the pointer and scale correctly. Table column drags
are stable and reversible: exact at 100/150%, fit-stretched at 80%. Topology
device hover keeps its geometry and shows the highlight at all three scales.

**Open defect:** the pane separators and height splitters do not cancel the
`pointerdown` default. In WebKitGTK a drag can select document text, and
later drags then stop after their first motion without committing. The table
column handle, which cancels the default, is unaffected. Not fixed by this record.

## Done when

Native actions and resulting files/states are recorded for each tested cell;
unsupported environments stay explicit. Chromium, Broadway startup and a mock
Tauri runtime are not substitutes. Not a GTK4 port or new window policy.

## References

[KL-020](../limitations/KL-020-overlay-accessibility-evidence.md) ·
[KL-130](../limitations/KL-130-ZOOM-application-zoom.md) ·
[AppImage launcher](../APPIMAGE_LAUNCHER.md) ·
[history](../history/OPEN_WORK_2026-10-25.md#wp-11--native-linux-acceptance)
