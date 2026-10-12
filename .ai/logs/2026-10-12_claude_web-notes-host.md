# 2026-10-12 — Claude — project notes workbench host (web integration package 1)

Goal: mount finished leaves in the web UI (Session 1 role). Package 1 of 4; package 5
(MCP rename acceptance with a real client) was deselected by the user.

## What changed
- `apps/knx-web/src/notesHost.ts`: explicit snake_case → camelCase identity mapping,
  refusal of incomplete identity, stable-ID target catalogue (dedupe, ambiguous IDs
  withheld, equal labels get `#id`, iterative building-part walk, 8-bit installation IDs).
- `NotesWorkspace.tsx`: mounts `ProjectNotesEditor`, refreshes via `api.currentProject`
  through App's `handleTreeUpdate`, reports clean on unmount.
- `App.tsx`: `notes` view, nav button, palette command `open-notes`, lazy mount keyed by
  server/project incarnation (not `loadKey`, which deletions also bump), draft guard
  (`NoteDraftGuardDialog`) in front of Open/Import, New project, Quit and window close;
  narrow screens collapse the navigation like Devices/catalog.
- Leaf fixes: refusal alert scrolls into view; opt-in checkbox resets global min-height.
- Help: workbench topic gained a fourth paragraph (EN/DE).

## Evidence
Gate on frozen `114b0097`: 21/22 stages green; full fixture browser stage 349/4 with the
four `device-editor-layout.e2e.ts` DPST-9-1 failures reproduced on untouched base.
Real-server loopback smoke: create, undo/redo, Save As, genuine 409 refusal, Save,
restart, reopen. Five guard mutants caught. Self-review only.

## Lessons
- A mutant can survive when a later remount resets the same flag; test the path where
  nothing remounts (open another project from a different view).
- Lazy mounting matters: an always-mounted notes view would add `GET /api/project/notes`
  to every intercepted e2e spec's "unexpected" list.
- An active parallel session (`ga-members-download-9006b65e`) edited apps/knx-web without
  a lock line; a notice was pushed (`b9e72d54`). Expect textual conflicts in App.tsx and
  the message catalogues for whoever lands second.
