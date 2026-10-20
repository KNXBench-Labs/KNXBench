# Desktop silent downloads

[Current summary and ledger status](../KNOWN_LIMITATIONS.md#catalogue-kl-182)

## First fixed AppImage: alpha.12 (20 October 2026 reconciliation)

The retained record's "next AppImage" is now historical.
[Alpha 12 release notes](../releases/alpha12.md) include the staged-download
repair and native save dialog; the retained
[publication receipt](../evidence/release-alpha12-2026-10-15.json) identifies
the published `v0.1.0-alpha.12` AppImage. Alpha.11 and older assets are not
retroactively repaired, and publication does not upgrade an installed copy.

The fallback when private staging fails remains reported and non-overwriting.
The recorded X11 check is not new Wayland, failure-dialog or companion-window
acceptance. Evidence: [download handler](../../apps/knx-desktop/src-tauri/src/download_handler.rs).

This dossier preserves the technical detail and dated history. Earlier
headings, failures and lifting conditions are historical where later
updates supersede them; they are not new work assignments.

<!-- BEGIN retained catalogue record -->

## 182. AppImages up to alpha.11 save in-app downloads silently into Downloads and replace same-named files

**Found 2026-10-13 while checking the import-report download for issue #7.**
Every download the UI starts the browser way (an `<a download>` on a blob or a
server URL) behaves differently in the AppImage than in a browser. That covers the
full import report, source import reports, retained payloads, the download
verification report, settings export, the contribution ZIP, Flow images, the
confirmed native project export (`project.knxdb`) and the browser fallbacks.
There is no dialog and no visible feedback, the file goes straight into the XDG
Downloads folder, and a file of the same name already there is **replaced**. Web
and Docker builds are not affected: the browser handles the download.

- **Cause.** Tauri 2.11.5 registers a WebKit download handler only when a window
  sets `on_download`, and wry 0.55.1 passes none on otherwise. KNXBench set none.
  Without a handler WebKitGTK's default decides the destination. Measured
  2026-10-13 with a probe that reproduces `downloadLocalBlob` exactly (WebKitGTK
  2.52.6 through `gi`, page base `http://127.0.0.1`, isolated `HOME`/XDG
  directories): the blob was saved as `import-report.json` in the configured
  Downloads folder, and a second run replaced an existing file of that name
  without asking. Wayland and X11 behaved the same. The probe did not run inside
  the AppImage, whose bundled WebKitGTK may differ in version.
- **Fix in source (2026-10-13).** The main window sets `on_download`
  (`download_handler`, `download_delivery` in `apps/knx-desktop/src-tauri`).
  A requested download is redirected into a private staging directory
  (`<app cache>/download-staging/<pid>-<n>/`, mode 0700, one per download). Once
  WebKit reports it finished, the native save dialog asks where it goes, with the
  page's file name suggested (wry's `name (1)` counter is removed again). The copy
  is written atomically through the same primitive as the session-log export, and
  the staged copy is removed whether the user saves or cancels. A failed download
  is discarded and reported in a native message. If staging itself is impossible,
  WebKit keeps wry's own non-overwriting Downloads path (`name (1).ext`) and the
  user is told where the file went. Staging directories older than a day, left
  over from a run that ended mid-dialog, are removed at start-up. Tauri keys the
  WebKit context by data directory and no window sets one, so the main window's
  handler also serves downloads from the Flow and diagnostics windows.
- **Evidence.** A second probe confirmed the mechanism the fix relies on: a plain
  absolute path set in `decide-destination` (what wry does) is honoured, including
  spaces and non-ASCII characters, and `destination()` reports that plain path, so
  the handler recognises its own staged files. Tests: `download_delivery` (6)
  covers per-download private directories, the counter rule, finished-download
  decisions, atomic delivery with the staged copy always removed, a failed
  delivery that keeps the previous file, and cleanup of old staging directories
  only. `download_handler` (2, mock runtime) checks that a request is redirected
  into staging and allowed, and that a staging failure falls back without
  refusing. Two mutants (request not redirected, staged copy not discarded) turn
  them red.
- **Native-window check, 13 October 2026.** Release build of `6ba1590a`, driven
  by hand in a real window (GTK X11 backend, isolated home directory), with the
  demo project *KV v2.5* imported: **Download full report** opened the native
  save dialog with `import-report.json` suggested. *Cancel* wrote nothing and
  emptied the private staging directory (mode 0700, removed with the staged
  copy). *Save* wrote the chosen path (mode 0600, the same 84 683 bytes as the
  staged copy) and emptied the staging directory again. The XDG Downloads folder
  stayed empty throughout.
- **Boundary.** No automated test drives a native WebKitGTK window: WebKitWebDriver
  and a headless compositor are not available on the development host. Not
  checked by hand either: the Wayland backend (the AppImage's default is
  `wayland,x11`), the message dialogs for a failed copy, and the Flow/diagnostics
  window routing. The fix reaches users with the next AppImage.

Ledger `KL-182`.
<!-- END retained catalogue record -->
