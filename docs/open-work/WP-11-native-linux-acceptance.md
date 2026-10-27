# WP-11 — Native Linux acceptance

**Status:** In progress: first bounded native slice and two positive editor cells recorded; remaining cells open. Listed in [OPEN_WORK](../OPEN_WORK.md); per-ID status of record is the [ledger](../status/LEDGER.md).

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

Next bounded native work can cover capture export with admitted fictional or
simulator data, then remaining keyboard/pointer/layout cells. Other controls,
imported-original instructions and vendor-specific EGL/GPU acceptance stay
open. Orca and Firefox were not installed and remain untested. No ledger row
is closed.

## Done when

Native actions and resulting files/states are recorded for each tested cell;
unsupported environments stay explicit. Chromium, Broadway startup and a mock
Tauri runtime are not substitutes. Not a GTK4 port or new window policy.

## References

[KL-020](../limitations/KL-020-overlay-accessibility-evidence.md) ·
[KL-130](../limitations/KL-130-ZOOM-application-zoom.md) ·
[AppImage launcher](../APPIMAGE_LAUNCHER.md) ·
[history](../history/OPEN_WORK_2026-10-25.md#wp-11--native-linux-acceptance)
