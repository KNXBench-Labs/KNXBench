# WP-11 — Native Linux acceptance

**Status:** Blocked: consent. Listed in [OPEN_WORK](../OPEN_WORK.md); per-ID status of record is the [ledger](../status/LEDGER.md).

## Task

Record actual native behaviour for named workflows with fictional or
simulator data (no bus contact needed):

- WebKitGTK keyboard, focus, draft and dialog behaviour, including the
  notes, instruction and checked-parameter editors.
- Orca / screen-reader behaviour.
- Wayland save/cancel/overwrite paths and the bus-monitor capture export.
- Application zoom cases on WebKitGTK (KL-130; Chromium-only so far).
- AppImage startup smoke on a newer-Mesa host (issue #6).
- Firefox for the app.

## Prerequisite

Separate permission for desktop control.

## Done when

Native actions and resulting files/states are recorded for each tested cell;
unsupported environments stay explicit. Chromium, Broadway startup and a mock
Tauri runtime are not substitutes. Not a GTK4 port or new window policy.

## References

[KL-020](../limitations/KL-020-overlay-accessibility-evidence.md) ·
[KL-130](../limitations/KL-130-ZOOM-application-zoom.md) ·
[AppImage launcher](../APPIMAGE_LAUNCHER.md) ·
[history](../history/OPEN_WORK_2026-10-25.md#wp-11--native-linux-acceptance)
