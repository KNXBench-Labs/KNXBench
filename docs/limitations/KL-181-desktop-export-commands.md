# Desktop export commands

[Current summary and ledger status](../KNOWN_LIMITATIONS.md#catalogue-kl-181)

This dossier preserves the technical detail and dated history. Earlier
headings, failures and lifting conditions are historical where later
updates supersede them; they are not new work assignments.

<!-- BEGIN retained catalogue record -->

## 181. AppImages up to alpha.11 cannot save the session log or a bus monitor capture

**Reported 2026-10-13 (issue #7, hagbardZ, on `v0.1.0-alpha.10`).** Saving the
session log in the desktop app fails with `Command save_session_log not allowed
by ACL`. The bus monitor's capture export fails the same way
(`save_bus_monitor_capture`). Both exports exist since `v0.1.0-alpha.5`
(`43dbc231`, `ff606f86`); every AppImage from alpha.5 to alpha.11 has the defect.
The web and Docker builds are not affected: there the browser downloads the
file and no desktop command is involved.

- **Cause.** A release window loads the UI from the embedded server at
  `http://127.0.0.1:<ephemeral port>` (`run` in `apps/knx-desktop/src-tauri/src/lib.rs`).
  That is neither a `devUrl` (the configuration sets none) nor `frontendDist`, so
  Tauri 2.11.5 treats it as a remote origin. A remote origin reaches an app
  command only through a permission granted by a capability
  (`tauri::webview` `on_message`). The main window's capability allowed the
  remote URLs but granted only core and dialog permissions, and the app declared
  no permissions for its own commands. A debug build's Vite origin
  `http://localhost:1420` is refused the same way, so a development build would
  not have hidden the defect either; it was simply never exercised in a desktop
  window. The existing tests covered the file writer and the web side.
- **Fix in source (2026-10-13).** `permissions/app-commands.toml` declares
  `allow-save-session-log` and `allow-save-bus-monitor-capture`;
  `capabilities/default.json` grants them to the `main` window only. The
  diagnostics companion and Flow windows stay without them. Because the app now
  has an ACL manifest, every app command is checked against it, so a future
  command without a grant fails its test instead of shipping.
- **Regression test.** `app_command_acl_tests` (mock runtime, the real
  capabilities through `generate_context!`) invokes both commands without
  arguments from the release and the debug origin in the main window: each must
  reach the command and fail only on its missing argument. From the diagnostics
  and a Flow window both must be refused. Red before the fix on the same refusal
  branch as the report (a release build words it `Command … not allowed by ACL`,
  a debug build gives a longer diagnosis); removing the two grants turns it red
  again.
- **Native-window check, 13 October 2026.** Release build of `6ba1590a` in a
  real window (GTK X11 backend, isolated home directory), served from
  `http://127.0.0.1:<port>` like the AppImage: *Export log (JSON)…* opened the
  native save dialog, and *Save* wrote `session-log.json` (mode 0600, `capacity`
  16000, all 62 server entries). No ACL refusal. The import's *Open File*
  dialog worked as well.
- **Boundary.** The automated test runs Tauri's mock runtime. *Export capture* in
  the bus monitor was not driven by hand: it appears only with recorded rows,
  which need a live tunnel to the owner's gateway (one tunnel at a time); it
  relies on the same grant, which the test covers. The Wayland backend was not
  checked by hand. The fix reaches users with the next AppImage. Until then the
  Docker image in a browser exports both files as ordinary downloads.

Ledger `KL-181`.
<!-- END retained catalogue record -->
