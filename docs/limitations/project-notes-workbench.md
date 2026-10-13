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
