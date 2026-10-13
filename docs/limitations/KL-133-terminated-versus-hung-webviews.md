# Terminated versus hung webviews

[Current summary and ledger status](../KNOWN_LIMITATIONS.md#catalogue-kl-133)

This dossier preserves the technical detail and dated history. Earlier
headings, failures and lifting conditions are historical where later
updates supersede them; they are not new work assignments.

<!-- BEGIN retained catalogue record -->

## §133 A dead webview cannot be closed with the window manager's close button

**Status.** Partially lifted, 2026-10-04: a **terminated** web process is
recovered; a web process that **hangs without terminating** remains open
(documented 2026-09-27 from the pinned `tauri` 2.11.5 sources; first
reproduced natively 2026-10-04, see below).

**Terminated web process, lifted.** The desktop shell now observes WebKit's
`web-process-terminated` signal (`apps/knx-desktop/src-tauri/src/web_process.rs`).
A crash, an out-of-memory kill or an unknown reason reloads the page. This
loses nothing: the unsaved project lives in the embedded `knx-server`, not in
the web process, and the reloaded frontend reattaches to it and registers a
fresh close handler, so the unsaved-changes guard stays in charge of ×. At most
three reloads are allowed within 60 s. A page that dies on every load then
counts as dead and reloading stops. The shell then answers the user's next
explicit close itself: with nothing unsaved it closes; when the server still
holds edits (`AppState::has_unsaved_changes`, the predicate behind the
published `is_modified`) it asks a native GTK question, English only because
the language preference lives in the failed frontend, and closes only on
"Close and discard changes". Repeated × while it is open stacks no second
question. A deliberate
`terminate_web_process` call (`TerminatedByApi`, never issued by this
application) is neither undone nor treated as dead. Eight shell policy tests
and one server test cover the mapping, budget, window edge, stickiness and the
close decision; nine guard mutants each fail a named test.

**Native evidence, 2026-10-04.** The real debug `knx-desktop` binary ran on a
private, signature-verified Xvfb in a loopback-only network namespace with
private HOME/XDG and D-Bus; no LAN or KNX path existed. The harness killed the
real `WebKitWebProcess` with `SIGKILL` and then sent the ICCCM
`WM_DELETE_WINDOW` client message, which is what a window manager sends for ×.
The unchanged baseline `eafb0322` reproduced this limitation: no new web
process, and the application did not exit within 20 s of the close request.
With the fix, one crash produced `Crashed → Reload`, a new web process and a
clean exit (code 0) on close; four crashes produced three reloads, then
`GiveUp`, and close still exited with code 0. With an unsaved project made
through the real server API, the same four crashes ended in `GiveUp`; close
then left the application running and opened the native question; dismissing
it kept the application running, and the next close asked again. That the
question appeared after three reloads also shows the unsaved project survived
them. Choosing "Close and discard changes" was not driven natively; it shares
the clean path's close call and is covered by the decision tests. This was a
client message on a bare X server, not a click on a full desktop window
manager; Wayland compositors and other window managers were not exercised.

**Alpha scope.** The user placed dead-WebView evidence outside the Alpha scope
on 2026-10-04 (`KL-133` is `ACCEPTED_BOUNDARY` in the Alpha ledger). This
change narrows that boundary; it is not an Alpha requirement.

**Still open: a web process that hangs.** If the web process stays alive but
never answers, × and Alt+F4 still do nothing. Nothing distinguishes a hung page
from a busy one with unsaved edits, so closing it automatically would bring back
the silent loss §132 removed.

**Limitation (original wording, still accurate for a hang).** Since §132, the main window's frontend listens for
`tauri://close-requested`. While such a JS listener is registered, `tauri`
2.11.5 calls `prevent_close()` on every `CloseRequested`
(`manager/window.rs` `on_window_event`), and only the frontend's handler can
then `destroy()` the window. If the WebKit web process has crashed or hangs
for good, × and Alt+F4 do nothing. Rust never drops a JS listener on its own
when a page crashes or reloads, so a stale registration keeps blocking. The
same holds briefly after a reload until `App` mounts again, for example while
a login screen is shown.

**Cost.** The user has to end the process some other way (the compositor's
kill binding, `kill`, a task manager). Unsaved edits are lost exactly as they
would have been before §132; the new cost is that the window will not close
at all.

**Why it is this way.** The obvious mitigation, Rust destroying the window
when the frontend does not answer within N seconds, would close a merely
*busy* frontend with unsaved edits and no question asked. That is the silent
loss §132 removed. Data integrity outranks convenience here.

**Lifted when.** The shell can tell a dead web process from a busy one. For
example, it could observe a WebKit web-process-terminated signal and then
drop the stale listener or destroy the window. Either way this needs a
test, or at least a manual reproduction on a real window manager.
*(2026-10-04: met for termination as described above; the hang case needs a
signal that separates a hung page from a busy one, which WebKitGTK's
`is-web-process-responsive` does not.)*
<!-- END retained catalogue record -->
