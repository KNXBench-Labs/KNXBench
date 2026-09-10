# T11: session log implementation plan

Spec: `docs/superpowers/specs/2026-09-08-session-log-design.md` (approved).
Closes gap analysis **T11** / **D7** (`docs/GAP_ANALYSIS_ETS.md`).

## Global Constraints

- `knx-etsproj::ImportReport` is not changed, flattened, or moved. It stays
  the single owner of import diagnostics.
- The session log lives in `apps/knx-server` only (`AppState`), is
  in-memory, and is never written to `.knxdb`. No `knx-store`/`knx-core`
  change is in scope.
- Entry severities are exactly `error` | `warning` | `info`
  (`#[serde(rename_all = "lowercase")]`, matching
  `knx_etsproj::report::Severity`'s own convention).
- Mapping rules (verbatim from the spec):
  - `ImportError` with `Severity::Error` → **error**.
  - `ImportError` with `Severity::Warning`, every `UnknownConstruct`, every
    `Conflict`, every `UnsupportedFeature` → **warning**.
  - A completed import/native-open/save/export/undo/redo/edit → **info**.
  - A failed import/open/save/export/undo/redo/edit → **error**, using the
    server's existing user-facing error string as `message`.
- **Ruling** (design text lists only "import/open/save/undo/redo/edit" and
  is silent on export): export is included, logged the same way as save.
  Excluding the one other project-level, fallible operation the server
  exposes would be an arbitrary gap the design's own "operational feedback"
  purpose argues against. Cost if wrong: one extra `push` call and one
  extra test to delete later — cheap to revert.
- Reset rule: the log is cleared immediately before a *successful* import or
  native open commits (i.e. right before/at the point `state.project` is
  replaced), then repopulated for that transition. A *failed* import/open
  never clears the existing log — it appends one error entry to whatever
  was already there. Save/export/undo/redo/edit never reset the log,
  success or failure.
- The HTTP projection (`GET /api/log`) returns every entry for the current
  session, oldest-first (append order). Newest-first ordering, severity
  filtering, and the empty state are frontend-only concerns per the spec
  ("filters affect only browser rendering: the endpoint always returns
  every entry").
- Existing behavior must not change: `ProjectTree.errors`/`warnings`
  counting, command validation, undo/redo semantics, `.knxdb` persistence,
  and `ImportReport` content are all out of scope for this plan.
- Run `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D
  warnings`, `cargo test --workspace`, `npm run lint` (if present) or
  `npx tsc --noEmit`, `npm test`, `npm run build` (then restore
  `apps/knx-web/dist/.gitkeep` — `npm run build` transiently deletes it,
  a pre-existing quirk, not something to fix here) before any task is
  considered done.

## Task 1: `SessionLog` core + backend wiring

Add a new module `apps/knx-server/src/session_log.rs`:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity { Error, Warning, Info }

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LogEntry {
    pub timestamp: String,       // RFC3339, chrono::Utc::now().to_rfc3339()
    pub severity: Severity,
    pub source: String,          // e.g. "import", "open", "save", "export",
                                  // "undo", "redo", or the command's own
                                  // name for an edit (e.g. "SetIndividualAddress")
    pub message: String,
    pub location: Option<String>, // xpath, when the origin has one
    pub detail: Option<String>,
}

#[derive(Debug, Default)]
pub struct SessionLog(Vec<LogEntry>);

impl SessionLog {
    pub fn reset(&mut self) { self.0.clear(); }
    pub fn push(&mut self, entry: LogEntry) { self.0.push(entry); }
    pub fn entries(&self) -> &[LogEntry] { &self.0 }
}

/// Converts one `ImportReport` into its warning/error entries (success
/// notice NOT included here — the caller appends that separately, since
/// only the caller knows the operation actually succeeded end to end).
pub fn from_import_report(report: &knx_etsproj::ImportReport) -> Vec<LogEntry> { ... }
```

`from_import_report` maps, in this order: every `report.errors` entry
(`stage`/`severity`/`xpath`/`detail` → `source = format!("import:{stage}")`,
`severity` per the mapping rule, `message = detail.clone()`,
`location = Some(xpath.clone())`), then every `report.unknown` entry
(`UnknownConstruct { source_path, xpath, kind, name, occurrences, sample }`
→ warning, `source = "import:unknown"`, `message` names `kind`/`name`/
`occurrences`, `location = Some(xpath.clone())`), then every
`report.conflicts` entry (`Conflict { group_address, candidates }` →
warning, `source = "import:conflict"`, message names the group address id
and candidate count, no location), then every `report.unsupported` entry
(`UnsupportedFeature { what, consequence }` → warning, `source =
"import:unsupported"`, `message = format!("{what}: {consequence}")`).

Add `pub session_log: Mutex<SessionLog>` to `AppState` (init
`Mutex::new(SessionLog::default())` in `AppState::new`).

Wire every call site below. Each push uses `LogEntry { timestamp: now(), .. }`
via a small `fn now() -> String` in `session_log.rs`
(`chrono::Utc::now().to_rfc3339()` — add the `"clock"` feature to the
workspace `chrono` dependency in the root `Cargo.toml`, it is currently
`features = ["std"]` only).

- `import_and_project` (domain.rs): currently discards `imported.report`
  after calling `apply_report_counts`. Change its return type (or add a
  sibling that also returns it) so `open_project` can build the log entries
  from the same report used for the counts.
- `open_project(state, path)`: on `Ok`, before returning — reset the log,
  extend it with `from_import_report(&report)`, then push one **info**
  entry (`source = "import"`, `message` naming the file and the entity
  counts, e.g. from `tree`'s counts or `report.source.file_name`). On
  `Err`, push one **error** entry (`source = "import"`, `message` = the
  error string) — do **not** reset first.
- `open_native_project(state, path)`: on `Ok`, reset the log, push one
  **info** entry (`source = "open"`, message names `path`). On `Err`, push
  one **error** entry (`source = "open"`), no reset.
- `save_project` / `save_project_as`: on `Ok`, push one **info** entry
  (`source = "save"`). On `Err`, push one **error** entry. Never resets.
- `export_project`: on `Ok`, push one **info** entry (`source = "export"`,
  mention `path`). On `Err`, push one **error** entry. Never resets. (Per
  this plan's ruling above.)
- `apply(state, cmd)` (the shared command dispatcher every `*_impl` edit
  function funnels through): capture a short description of `cmd` (its
  `Debug` form is fine — do this *before* `do_command` consumes it) as
  `source`/`message`. On success, push one **info** entry. On failure, push
  one **error** entry with the returned error string as `message` and the
  same `source`.
- `undo_impl` / `redo_impl`: on success, push one **info** entry
  (`source = "undo"`/`"redo"`). On failure, push one **error** entry.

Add `GET /api/log` to `apps/knx-server/src/routes.rs`
(`project_routes()`'s router): returns `Json(Vec<LogEntry>)` — clone
`state.session_log.lock().unwrap().entries().to_vec()`. No request body, no
error case (an empty/absent log is just `[]`, not a 404 — there need not be
an open project for the route to answer).

**Tests** (`session_log.rs` unit tests + one `apps/knx-server` route test,
following existing route-test conventions in this crate):
- Each `ImportReport` category maps to the documented severity.
- A failed import appends an error entry without touching prior entries
  (reset only happens on success).
- A successful import resets and repopulates the log.
- `GET /api/log` reflects entries after an import, a failed edit, and a
  successful edit, in append order.

**Report contract:** the report must state the exact `LogEntry` JSON field
names and the `GET /api/log` response shape verbatim — Task 2 depends on
matching it exactly, and cannot re-derive it from this brief alone.

## Task 2: frontend Log tab

Depends on Task 1's committed `LogEntry` shape (read it from Task 1's
report, not from this brief).

- `apps/knx-web/src/api.ts`: hand-written `export interface LogEntry {
  timestamp: string; severity: "error" | "warning" | "info"; source:
  string; message: string; location: string | null; detail: string | null;
  }` (mirror `CatalogInstallReport`'s existing pattern in this same file —
  no `ts-rs` binding, this DTO is server-local) plus `export function
  getSessionLog(): Promise<LogEntry[]> { return request("/api/log"); }`.
- New `apps/knx-web/src/LogPanel.tsx`: fetches the log on mount (and
  whenever its `tree` prop changes, so it stays current while left open
  across an edit/undo/import), renders entries **newest-first**, with an
  explicit empty state ("No log entries yet.") and three toggle filters
  (Error/Warning/Info, all on by default) that only affect which already-
  fetched entries render — never re-fetch on filter toggle. Each row shows
  severity, source, message, and (if present) location/detail.
- Wire into `apps/knx-web/src/App.tsx`: add a `logOpen` boolean state and a
  "Log" toolbar button (disabled when `!tree`, alongside the existing
  Undo/Redo/Search buttons). When `logOpen` is true, render `<LogPanel
  tree={tree} />` in place of the `Inspector`/`Dashboard` branch inside
  `.workspace` (same slot, mutually exclusive with both). Selecting an
  entity in `ProjectExplorer` (`selectEntity`) closes the Log panel (sets
  `logOpen` false) so Inspector becomes visible again, matching how
  selection already switches away from `Dashboard`.

**Tests** (Vitest, mirror existing component test conventions — see
`ProjectExplorer.test.tsx`/`CatalogBrowser.test.tsx`): newest-first
rendering, each severity filter hides/shows the right rows, the empty
state, and that toggling a filter does not trigger a re-fetch.

## Task 3: docs closure

- `docs/GAP_ANALYSIS_ETS.md`: close **D7**'s gap-table row and **T11**'s
  backlog entry (`~~**T11. ...**~~` **Closed (2026-09-10)** with the same
  level of detail as T8/T9/T10's closed entries — what shipped, which
  files, exact test counts, "Closes **D7**.").
- `docs/IMPLEMENTATION_STATUS.md`: new dated cycle entry summarizing the
  `SessionLog` module, the ownership/reset rules, the export ruling above,
  the frontend Log tab, test counts, and gate results. Reference
  `docs/superpowers/specs/2026-09-08-session-log-design.md`.
- Full gate suite (Rust workspace tests/clippy/fmt, `tsc`, Vitest, `npm run
  build` with the `.gitkeep` restore) run one more time on the merged
  state of Tasks 1-2 and recorded in the ledger.
