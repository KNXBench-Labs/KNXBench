# WP-11 — Native Linux acceptance

**Status:** In progress: first bounded native Alpha 17 slice recorded; remaining cells open. Listed in [OPEN_WORK](../OPEN_WORK.md); per-ID status of record is the [ledger](../status/LEDGER.md).

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

The fictional demo only established checked-editor `unsupportedXml` refusal and
instruction `MissingSource` feedback, not positive editing. Those positive
cells need an admitted-source synthetic fixture. Bus-monitor capture export,
pointer/hover/full-layout and vendor-specific EGL/GPU acceptance remain open.
Orca and Firefox were not installed and remain untested. No ledger row is closed.

## Done when

Native actions and resulting files/states are recorded for each tested cell;
unsupported environments stay explicit. Chromium, Broadway startup and a mock
Tauri runtime are not substitutes. Not a GTK4 port or new window policy.

## References

[KL-020](../limitations/KL-020-overlay-accessibility-evidence.md) ·
[KL-130](../limitations/KL-130-ZOOM-application-zoom.md) ·
[AppImage launcher](../APPIMAGE_LAUNCHER.md) ·
[history](../history/OPEN_WORK_2026-10-25.md#wp-11--native-linux-acceptance)
