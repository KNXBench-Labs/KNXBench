# ADR 0047: Session-log export is a versioned local JSON snapshot

Date: 2026-09-28 (UTC)
Status: Accepted
Session: UI track U4 / ISSUE-13

## Context

`GET /api/log` already supplies the retained in-memory entries to `LogPanel`.
`SessionLog` caps them at 1000, pins a synthetic warning with the number of
older entries dropped, and resets when a replacement project is opened.
The browser's existing server-mount picker returns server paths, not a path
on the user's computer. A log export must not silently imply lifetime
coverage or expose a server filesystem path to the browser.

## Decision

1. Serialize the entries already fetched by `LogPanel` once as a versioned
   `knxbench-session-log` JSON document. It includes `scope` (`all` retained
   or current filtered view), `capacity`, `droppedCount`, original drop
   notices and the raw entries (including location and diagnostic metadata).
   If the warning cannot be parsed safely, report `null` rather than a
   fabricated loss count. JSON string values are not spreadsheet cells;
   user-controlled `=`, `+`, `-`, `@`, tabs and CR remain quoted data.
2. In the browser, deliver the JSON Blob with a local download link. In
   the desktop shell, call one Tauri command which *itself* presents the
   existing native dialog and writes an atomic sibling-temp-file rename.
   JavaScript supplies no destination path, and no HTTP endpoint writes a
   client-supplied path. The desktop command validates format, version,
   capacity and the 16 MiB size bound before touching the destination.
3. The UI explicitly states that the log belongs to this server run, is
   capped at 1000 and may contain addresses/names. A matching-only export
   still carries loss metadata even if its severity/search filter hides
   the synthetic warning. Export does not mutate the log or fetch another
   snapshot behind the user's back.

## Alternatives considered

- Server-side CSV export: rejected because it duplicates the existing
  client-side filter, turns user text into spreadsheet cells (requiring
  formula defanging) and would add a path-writing HTTP route.
- A broad Tauri filesystem capability: rejected; a dialog-owned native
  command limits writes to a path the OS picker returned.
- Labelling the export as a full history: false after cap eviction,
  project replacement or server restart.

## Consequences and limits

- The existing `/api/log` array contract is unchanged. JSON v1 is a
  KNXBench-defined format, not an ETS format or permanent audit archive.
- The current drop count is derived from the server's pinned English
  warning; an unfamiliar warning is carried verbatim and yields
  `droppedCount: null`. A future structured counter should supersede this
  text extraction without changing the JSON meaning.
- Desktop exports larger than 16 MiB are refused without overwriting the
  prior destination. Native-dialog operation is compile/test verified, not
  yet tested interactively on every supported Linux desktop environment.
