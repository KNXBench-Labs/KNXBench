# Session Log design

## Purpose

KNXBench shall provide a Log tab for issues arising while a project is open.
It makes import diagnostics and operational feedback inspectable after their
toast disappeared, without turning transient application activity into project
data.

## Scope

The first slice records import diagnostics and success notices, plus failures
returned by project-level operations and edits. It does not persist entries in
`.knxdb`, provide an export function, or add a general diagnostic model to
`knx-core`.

## Ownership and data flow

`knx-etsproj` remains the owner of `ImportReport`; its structured information
is neither flattened nor moved into the domain model. `knx-server` maps that
report at the HTTP/application boundary into session log entries and owns their
in-memory lifetime. The React frontend obtains the entries only through a
typed `GET /api/log` endpoint.

```text
ImportReport / failed or successful server operation
                    |
                    v
       knx-server session log (in-memory, current project only)
                    |
                    v
             GET /api/log -> React Log tab
```

The server resets the session log before committing a successful ETS import or
native-project open. Thus entries never describe a previously opened project.
If opening/importing fails, the existing project and its log remain intact; the
failed attempt is appended as an error. A successful ETS import replaces the
log with entries derived from that import, followed by its success notice. A
successful native open begins with its own success notice. This is deliberately
session-scoped: saving or reopening a `.knxdb` does not recreate previous log
entries.

## Entry shape and mapping

The HTTP projection contains a server-generated timestamp, severity, source,
message, and optional location/detail. It carries no raw project data or opaque
bytes.

* `ImportError` with `Severity::Error` maps to **error**.
* `ImportError` with `Severity::Warning`, unknown constructs, inference
  conflicts, and unsupported features map to **warning**.
* Completed imports, native opens, saves, undo/redo, and successful edits map
  to **info**.
* Failed import/open/save/undo/redo/edit operations map to **error** and retain
  the server's user-facing error text.

The mapping must preserve source path/XPath and detail where the import report
provides them. It must not treat a documented limitation as data loss: the
existing `ProjectTree.errors`/`warnings` counting semantics stay unchanged.

## User interface

The workspace gains a Log tab alongside the project workspace views. It shows
newest entries first and an explicit empty state. Error, warning, and info
filters affect only browser rendering: the endpoint always returns every entry
for the active session. Existing toast feedback remains unchanged, so users get
immediate feedback as well as a durable-in-session record. Existing import
counters remain as summaries and are not replaced by the tab.

## Error handling and invariants

The frontend never manufactures log records from failed fetches; the server is
the single authority for the session log. A failure to retrieve the log uses
the existing toast path. Log collection must not change command validation,
undo/redo, persistence, importer behavior, or `ImportReport` content.

## Verification

Focused Rust tests cover conversion of every import-report category and the
reset/append rules across successful and failed project transitions. Server
route tests cover the JSON response. Vitest coverage checks severity filters,
newest-first rendering, and the empty state. The normal Rust formatting,
layering, and relevant web test/build gates provide integration proof.
