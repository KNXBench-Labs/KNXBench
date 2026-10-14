# ADR 0127: The web workbench resumes the server's open project on every mount

Date: 2026-10-14
Status: Accepted
Session: owner UI feedback P1 (`docs/goals/ui-feedback-p1-p4.md`)

## Context

The server holds the open project in memory (`AppState`); the browser holds a
copy. Before this decision only the diagnostics companion's explicit return
(`?view=editor`) read the server's project back on mount. Every other mount
started on the welcome screen, so after closing a browser tab by accident
(Docker/web deployment) the project the server still had open was invisible.
Opening a file then reported only that the invisible project had unsaved
changes (owner report, point 6).

ADR-0026's sentence that "no endpoint hands a loaded project back to a fresh
mount" predates `GET /api/project` (the authenticated, read-only snapshot the
companion return already uses). Verified facts: the route answers `400 no
project open` when nothing is open (`domain::current_project_tree`), returns
the same projection as every edit reply, including `is_modified` and
`has_store_path`, and mutates nothing.

## Decision

Every editor mount reads `GET /api/project` once and shows the project if one
is open. It is a read, never an open, import or replacement. A `400` leaves the
welcome screen as it is, silently; any other failure is reported. Anything the
user starts before the answer arrives (an open, an import, a new project) wins,
and an answer arriving after unmount is ignored. An automatic resume (reload,
reopened tab) announces itself with a toast that names unsaved changes when the
server reports them; the companion's explicit return stays silent.

No browser "Leave page?" prompt is added for unsaved changes: closing the tab
no longer loses them, and the prompt could not protect against the one case
that does (a server restart). Autosave is the tool for that case.

## Alternatives considered

- **A "Resume open project" button on the welcome screen.** One click more for
  the common case; offered to the owner, who chose the automatic resume.
- **Carry a "project open" flag in `GET /api/auth/status`.** Would avoid one
  request in browser fixtures, but that route answers unauthenticated callers
  and would tell them whether a project is open.
- **A `beforeunload` prompt.** Nags on every reload and still loses work on a
  server restart.

## Consequences

- Two tabs on the same server now show the same project; snapshot ordering
  (ADR-0032) and revision-bound edits already arbitrate between windows, as
  for the companion.
- Browser fixtures must model the real server: `GET /api/project` answers
  `400` until the fixture has a project, otherwise the app resumes the
  fixture's project before the test opens one.
- The desktop shell starts its embedded server empty, so a desktop start shows
  the welcome screen as before; a webview reload resumes.
