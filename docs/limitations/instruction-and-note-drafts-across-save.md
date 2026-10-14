# Instruction and note drafts across Save

[Current summary and ledger status](../KNOWN_LIMITATIONS.md#catalogue-instruction-and-note-drafts-across-save)

This dossier preserves the technical detail and dated history. Earlier
headings, failures and lifting conditions are historical where later
updates supersede them; they are not new work assignments.

<!-- BEGIN retained catalogue record -->

## Installation instructions tab and Save-proof drafts (2026-10-12)

- The **Installation instructions** tab hosts the existing editor; its source and
  editing limits are unchanged (section *Installation instruction editor integration
  boundary* below).
- Saving the project bumps the snapshot revision. Notes and instruction drafts now
  survive that when their server content is unchanged; any real change (another
  client, Undo/Redo) still turns the draft into a comparison copy that must be
  discarded before a new preview. While the re-read runs, review and apply wait
  briefly; the notes editor also pauses typing for that moment.
- The unapplied-draft guard covers Open/Import, New project, Quit and the desktop
  window close; since the notes follow-ups (2026-10-12, see *Project notes
  workbench host*), a browser reload or closed tab asks through the browser's own
  prompt.
- Evidence is Chromium (intercepted production build and loopback-only real-server
  smokes). No native WebKitGTK window, Orca session, hardware, release or deployment
  is claimed.
<!-- END retained catalogue record -->

## Update 2026-10-14: reloads ask in the desktop webview too (Goal B2)

The browser's reload/tab-close prompt is installed in the desktop shell as well,
where WebKitGTK shows its own Stay/Leave dialog. Evidence and what was not run:
[project notes workbench](project-notes-workbench.md#update-2026-10-14-the-desktop-webview-asks-before-a-reload-too-goal-b2).
