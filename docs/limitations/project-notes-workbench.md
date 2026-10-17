# Project notes workbench

[Current summary and ledger status](../KNOWN_LIMITATIONS.md#catalogue-project-notes-workbench)

This dossier preserves the technical detail and dated history. Earlier
headings, failures and lifting conditions are historical where later
updates supersede them; they are not new work assignments.

<!-- BEGIN retained catalogue record -->

## Project notes workbench host (2026-10-12)

The **Notes** view hosts the notes editor with the open tree's real identity
([host](../contracts/project-note-editor-host.md#workbench-host-12-october-2026)).

- Since the follow-ups of 2026-10-12 the target list comes from the service
  (`GET /api/project/notes/targets`, `knx_app::note_targets`) and includes
  communication objects. It offers only targets the domain accepts (an ID that
  occurs twice in a kind is withheld, not guessed). Devices are listed by
  individual address, unaddressed ones last by ID; a device's objects follow its
  own object order. Names are the project's default language, not the UI or
  product-text language. The list is reloaded after every revision change; until
  the answer arrives the previous list stays, and the server re-checks the target
  on preview. Stored targets missing from the list stay unchanged and labelled.
- If the list cannot be loaded (or belongs to another project), the notebook shows
  an alert with **Load targets again** and offers only the project as a new target.
- The draft guard covers Open/Import, New project, Quit and the desktop window's
  close button. In a browser, a reload or closed tab asks through the browser's
  own prompt; its wording cannot be translated or styled. The desktop shell does
  not install that prompt (its close is guarded by the KNXBench dialog and it quits
  with `destroy()`); a reload inside the desktop webview is therefore unguarded.
  Project edits without a note draft still reload without asking.
- The review names actions and the documentation opt-in in EN/DE; the Bavarian and
  Klingon packs fall back to English there, like the rest of the feature-local
  notes copy.
- Evidence is Chromium: an intercepted production build and one loopback-only
  real-server smoke. No native WebKitGTK window, Orca session, hardware, release or
  deployment is claimed. FUTURE-03 stays LATER.
<!-- END retained catalogue record -->

## Update 2026-10-14: the desktop webview asks before a reload too (Goal B2)

The reload/tab-close prompt is now installed in the desktop shell as well. A
reload inside the desktop webview (WebKitGTK's context menu offers **Reload**)
with a note or instruction draft asks through WebKitGTK's own **Stay**/**Leave**
dialog; its wording follows WebKitGTK and the system language, not KNXBench's.
The window's close button and **Quit** still use the KNXBench dialog and ask once:
the window ends with `destroy()`, which runs no `beforeunload`.

Why this should hold natively, from sources rather than a run: WebKitGTK documents
that an unhandled `script-dialog` signal shows a Stay/Leave dialog for
`beforeunload`; wry 0.55.1 handles neither `script-dialog` nor `context-menu`; and
no `webkit_web_view_try_close` call exists in wry, tao or tauri-runtime-wry.
Evidence actually run: Vitest, and a reload with and without a draft (stay keeps
it, leave reloads) in Chromium and in Playwright's Firefox 155. **Not run:** a
native WebKitGTK window (no isolated display on the build host; the owner's
desktop is not used without a go), Playwright's WebKit (host libraries missing)
and Orca. Same-tab navigations to other pages ask the same way; links that open
outside the app use `target="_blank"` and do not unload it.

## Update 2026-10-17: deleting a note's target (ADR-0138)

Deleting an element a note belongs to no longer ends in a generic refusal. The
web app's deletes (properties pane and the bulk toolbar) list the affected
notes and let the user keep each on the project, move it to another element or
delete it; confirming applies the choices and the delete as one undo step
([ADR-0138](../adr/0138-notes-get-an-explicit-decision-before-a-delete.md)).
`FUTURE-03` is closed. Boundaries that remain:

- Other ways of deleting offer no decision dialog: the CLI, a group address CSV
  import with delete rows (CLI and web) and removing missing devices after a bus
  scan refuse with the named list of notes and change nothing. Move or delete
  those notes first in the Notes workspace.
- The dialog's own text is translated (EN/DE; Bavarian and Klingon fall back to
  English); the server's reason inside a refused choice, and the reason the CLI
  and the CSV/scan paths print, is English.
- Selective import carries source notes along their remapped targets and
  requires an explicit choice (skip or attach to the project) for notes outside
  the selection. `.knxproj` sources contain no KNXBench notes, so no shipped
  reader produces such notes today; the choice exists in the application and
  HTTP selection and has no control in the web dialog or the CLI.
- Evidence: unit and HTTP tests plus an intercepted Chromium spec (EN/DE,
  1440/400 px). No real-server browser smoke, no native WebKitGTK window.
