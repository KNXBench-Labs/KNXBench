# ETS Project Export (T10) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Give a user a real way to export the open project back to a `.knxproj` file, through the CLI, the HTTP API, and the web frontend. `knx_app::export_ets_project` already exists and is tested at the crate level (7 tests in `knx-app`) — this plan wires it to the three user-facing entry points that are currently missing, closing [GAP_ANALYSIS_ETS.md](../../GAP_ANALYSIS_ETS.md)'s **C4**: "import works end-to-end for a user; export does not."

**Architecture / key ruling:** `knx_app::export_ets_project(project, conn, products)` needs a `knx_store::Connection` to read back the opaque passthrough table and the manufacturer manifest (ADR-0005/ADR-0011) — data that is captured into a **throwaway in-memory store** during `apps/knx-server`'s ETS import (`domain.rs::import_and_project`, `knx_store::open_and_migrate_in_memory()`) and then discarded; only the plain `knx_core::Project` survives into `AppState`. That in-memory opaque/manifest data is *not* silently lost forever, though: a `.knxdb` save via `save_project`/`save_project_as` already persists the full opaque + manifest tables (schema v4, Session 5 cycle 2 entity persistence) alongside the project. **Ruling: export requires the project to have a `.knxdb` `store_path` set first** (i.e. saved or opened as `.knxdb` at least once) — the export route opens *that* `.knxdb` file read-only purely to source the opaque/manifest connection, while still exporting the live in-memory `state.project` (so unsaved edits since the last save are included, not the on-disk snapshot). This mirrors the existing `save_project` precedent, which already refuses with `"no save location yet — use Save As"` when `store_path` is `None` — export gets the same shape of honest refusal instead of silently exporting an empty/wrong opaque set. A project fresh from ETS import that was *never* saved as `.knxdb` cannot be exported yet; the error message must say so plainly.

**Tech Stack:** Rust (knx-server/axum, knx-cli), TypeScript/React (knx-web), vitest, cargo test.

**Spec:** [docs/GAP_ANALYSIS_ETS.md](../../GAP_ANALYSIS_ETS.md) — Tier 3, **T10**, closes **C4**.

## Global Constraints

- Never silently drop an `ExportWarning`. Every one produced by `export_ets_project` (always at least `Unsigned`) must reach the user: printed on the CLI, returned in the HTTP JSON response, and toasted in the frontend — the same "never silently discard" rule CLAUDE.md/AGENTS.md state for import already applies to export.
- Export writes the **live in-memory `state.project`** (via `apps/knx-server`), not a project reloaded from the `.knxdb` file at `store_path`. The `.knxdb` connection is opened only to source `load_opaque`/`load_manufacturer_refs`.
- If `store_path` is `None` when export is requested, fail with a clear, actionable message (e.g. `"save the project as .knxdb first — export reads passthrough data from the saved store"`) rather than exporting with an empty opaque set. Do not attempt to work around this by opening a fresh in-memory store — that would silently drop every opaque/manufacturer entry the original ETS import carried.
- Follow the existing `CreationDiagnostic`/`CreationDiagnosticDto` conversion pattern in `apps/knx-server/src/routes.rs` (a `Dto` enum plus a `From<domain::X>` impl, `#[serde(rename_all = "camelCase")]`) for `ExportWarning`, since `knx_etsproj::export::ExportWarning` does not derive `Serialize` itself and must not be made to (it lives in `knx-etsproj`, below the HTTP layer).
- No Tauri `invoke()` calls anywhere in `apps/knx-web` — fetch()-based only (`api.ts`'s own header comment).
- Do not touch `/api/project/download` (`apps/knx-server/src/fs_routes.rs`) or its associated `.knxdb` download flow — out of scope, a separate and already-known-unused route not part of this task.
- Keep the three tasks' file sets disjoint (knx-server, knx-cli, knx-web) — no task edits another task's files.

---

### Task 1: `apps/knx-server` — export domain function + `POST /api/project/export` route

**Files:**
- Modify: `apps/knx-server/src/domain.rs`
- Modify: `apps/knx-server/src/routes.rs`
- Test: new `apps/knx-server/tests/http_export_route.rs`

**Interfaces:**
- Consumes: `knx_app::export_ets_project(project: &knx_core::Project, conn: &knx_store::Connection, products: Option<&knx_productdb::Connection>) -> Result<ExportOutcome, AppError>` (re-exported at `knx_app`'s crate root); `ExportOutcome { bytes: Vec<u8>, warnings: Vec<ExportWarning> }` and `ExportWarning` (`crates/knx-etsproj/src/export/mod.rs`, variants `Unsigned { detail }`, `StaleSignature { source_path }`, `ManufacturerDataFromProductDb { entries }`, `MissingManufacturerData { source_path, sha256 }`); `AppState { project, store_path, product_db, .. }` (`apps/knx-server/src/domain.rs`); the existing `PathBody { path: String }` struct and `resolve_new_project_path`/`resolve_project_path` helpers (`apps/knx-server/src/paths.rs`) — export's target path is a *write* target like save-as's, so reuse `resolve_new_project_path`.
- Produces: `domain::export_project(state: &AppState, path: &Path) -> Result<knx_app::export::ExportOutcome, String>` (return the whole `ExportOutcome`, including `bytes`, so the route can both write the file and report the warnings from one call — do not discard `bytes` inside `domain.rs`); `POST /api/project/export` route, consumed by Task 3's frontend.

- [ ] **Step 1: `domain::export_project`**

In `apps/knx-server/src/domain.rs`, add near `save_project_as`/`save_project` (after `save_project`'s closing brace):

```rust
/// Exports the live in-memory project to a `.knxproj` file at `path`.
/// Requires `store_path` already set (i.e. the project has been saved or
/// opened as `.knxdb` at least once): the opaque passthrough table and
/// manufacturer manifest `export_ets_project` needs are read from *that*
/// `.knxdb` connection, never from a fresh empty one, so no opaque or
/// manufacturer data silently goes missing from the export. The project
/// content itself comes from `state.project` (live, possibly edited since
/// the last save), not from re-loading the `.knxdb` file.
pub fn export_project(state: &AppState, path: &Path) -> Result<knx_app::export::ExportOutcome, String> {
    let store_path = state
        .store_path
        .lock()
        .expect("state mutex poisoned")
        .clone()
        .ok_or("save the project as .knxdb first — export reads passthrough data from the saved store")?;
    let conn = knx_store::open_and_migrate(&store_path).map_err(|e| e.to_string())?;
    let project = state.project.lock().expect("state mutex poisoned");
    let project = project.as_ref().ok_or("no project open")?;
    let product_db_guard = state
        .product_db
        .as_ref()
        .map(|m| m.lock().expect("state mutex poisoned"));
    let outcome = knx_app::export_ets_project(project, &conn, product_db_guard.as_deref())
        .map_err(|e| e.to_string())?;
    drop(product_db_guard);
    std::fs::write(path, &outcome.bytes).map_err(|e| e.to_string())?;
    Ok(outcome)
}
```

Add `use std::path::Path;` if not already imported (it is, per the existing `save_project_as_impl(path: &Path, ...)` signature).

- [ ] **Step 2: route + DTOs**

In `apps/knx-server/src/routes.rs`, register the route in the `Router::new()` chain (alongside `/api/project/save-as`):

```rust
.route("/api/project/export", post(export_project))
```

Add the DTOs and handler near `CreateDeviceResponseDto`/`CreationDiagnosticDto` (same conversion shape — `ExportWarning` does not derive `Serialize`, so convert explicitly):

```rust
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
enum ExportWarningDto {
    Unsigned { detail: String },
    StaleSignature { source_path: String },
    ManufacturerDataFromProductDb { entries: usize },
    MissingManufacturerData { source_path: String, sha256: String },
}

impl From<knx_etsproj::export::ExportWarning> for ExportWarningDto {
    fn from(value: knx_etsproj::export::ExportWarning) -> Self {
        use knx_etsproj::export::ExportWarning as W;
        match value {
            W::Unsigned { detail } => Self::Unsigned { detail },
            W::StaleSignature { source_path } => Self::StaleSignature { source_path },
            W::ManufacturerDataFromProductDb { entries } => {
                Self::ManufacturerDataFromProductDb { entries }
            }
            W::MissingManufacturerData { source_path, sha256 } => {
                Self::MissingManufacturerData { source_path, sha256 }
            }
        }
    }
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct ExportReportDto {
    warnings: Vec<ExportWarningDto>,
}

async fn export_project(
    State(state): State<SharedState>,
    Json(body): Json<PathBody>,
) -> Result<Json<ExportReportDto>, ApiError> {
    let path = resolve_new_project_path(&state.data_dir, &body.path)?;
    domain::export_project(&state, &path)
        .map(|outcome| ExportReportDto {
            warnings: outcome.warnings.into_iter().map(Into::into).collect(),
        })
        .map(Json)
        .map_err(ApiError::bad_request)
}
```

Check `knx_etsproj` is already a dependency of `apps/knx-server`'s `Cargo.toml` (it must be, since `knx_projection`/`knx_app` sit above it in the layering and `ExportOutcome`'s type needs naming) — add the dependency if `cargo check` reports it missing; do not add it if the crate root re-exports `ExportWarning` some other way already (`rg "knx_etsproj" apps/knx-server/Cargo.toml` first).

- [ ] **Step 3: tests**

New file `apps/knx-server/tests/http_export_route.rs`, following `http_project_routes.rs`'s exact setup shape (`workspace_root()`, `reference_ets4_path()`, `body_json()`, `AppState::default()` + `knx_server::app(state, None)`, `tempfile::tempdir()`):

1. `exporting_without_a_store_path_is_a_400` — import the reference project (no save), POST `/api/project/export` with a temp path, assert `StatusCode::BAD_REQUEST` and a non-empty `error` mentioning `.knxdb`.
2. `importing_saving_then_exporting_round_trips_and_reports_the_unsigned_warning` — import the reference project, `POST /api/project/save-as` to a temp `.knxdb`, `POST /api/project/export` to a temp `.knxproj` path, assert `StatusCode::OK`, assert the response's `warnings` array contains an entry with `type: "unsigned"` (camelCase-serialized tag — check the actual tag `serde` emits for the unit-like variant and use that literal value, not a guess), assert the file at the export path exists and is non-empty (`std::fs::metadata(&export_path).unwrap().len() > 0`). Do not assert byte-for-byte equality with the original `.knxproj` — this is a round-trip-produces-*a*-file test, not a byte-oracle test (that belongs to `knx-app`'s own crate-level tests, already passing).

Run `cargo test -p knx-server` and confirm both new tests plus the existing suite pass.

---

### Task 2: `apps/knx-cli` — `knx export` subcommand

**Files:**
- Modify: `apps/knx-cli/src/main.rs`
- Test: `apps/knx-cli/tests/cli_import.rs` (add tests) or a new `apps/knx-cli/tests/cli_export.rs` — prefer a new file for the export tests specifically, since `cli_import.rs`'s name should stay accurate to its contents.

**Interfaces:**
- Consumes: `knx_store::open_and_migrate`/`load_project` (`.knxdb` reading, same pattern `knx-server`'s `domain::load_native` uses), `knx_app::export_ets_project`, `knx_productdb::default_path`/`open_and_migrate` (mirroring `run_import`'s existing `--product-db`/`--no-product-db` flag handling — read that function in `apps/knx-cli/src/main.rs` before writing this one, and reuse its flag-parsing helpers, e.g. `take_value`, rather than duplicating them).
- Produces: `knx export <store.knxdb> <out.knxproj> [--product-db <path>] [--no-product-db]` on the CLI, independent of the HTTP path Task 1 adds — this reads a `.knxdb` file directly rather than talking to a running server, since `knx-cli` has no server session to share.

- [ ] **Step 1: usage string + dispatch**

Update `USAGE` in `apps/knx-cli/src/main.rs` to add, after the `import` line:

```
     \x20     knx export <store.knxdb> <out.knxproj> [--product-db <path>] [--no-product-db]\n\
```

Add `Some("export") => run_export(&args[1..]),` to the `match args.first()...` dispatch in `fn main`.

- [ ] **Step 2: `run_export`**

Add a function next to `run_import`, following the same argument-parsing shape (`take_value`, positional-then-flags, `--product-db`/`--no-product-db` mutually exclusive exactly as `ImportArgs` already handles — read that exact handling in `run_import` and copy its rule, don't re-derive a different one):

```rust
fn run_export(args: &[String]) -> ExitCode {
    // parse: args[0] = store path, args[1] = output .knxproj path, then flags
    // open_and_migrate(store_path) -> load_project -> export_ets_project(&project, &conn, product_db.as_ref())
    // on Ok: std::fs::write(out_path, outcome.bytes); print each outcome.warnings entry to stderr; ExitCode::SUCCESS
    // on Err: eprintln!(...); ExitCode::FAILURE
}
```

Print every `ExportWarning` to stderr with `eprintln!("warning: {:?}", warning)` at minimum (a `Debug`-derived message is acceptable here — this crate has no existing `Display` impl for `ExportWarning` to reuse, and adding one is out of scope for this plan) — the requirement is that every warning is visible, not a particular format.

- [ ] **Step 3: tests**

New `apps/knx-cli/tests/cli_export.rs`, mirroring `cli_import.rs`'s `Command::new(env!("CARGO_BIN_EXE_knx"))` setup:

1. Build a `.knxdb` fixture by running `knx import <reference.knxproj> --store <tmp.knxdb> --no-product-db` as a setup step (spawn the binary, same as the existing import tests do), then run `knx export <tmp.knxdb> <tmp.knxproj>` and assert exit code 0, the output file exists and is non-empty, and stderr mentions the unsigned-export warning (substring check, not exact-string).
2. `knx export` against a `.knxdb` path that does not exist → non-zero exit code, a non-empty stderr message.

Run `cargo test -p knx-cli` and confirm all tests pass.

---

### Task 3: `apps/knx-web` — "Export to .knxproj…" button

**Files:**
- Modify: `apps/knx-web/src/api.ts`
- Modify: `apps/knx-web/src/App.tsx`

**Interfaces:**
- Consumes: Task 1's `POST /api/project/export` route (`{ path: string }` body → `{ warnings: ExportWarningDto[] }`), the existing `pickSavePath`/`isTauri` (`filePicker.ts`), `pushError`/`useToasts` (`toast.ts`), the existing `hasStorePath` state `App.tsx` already tracks for `Save`'s own dialog-skip logic.
- Produces: `api.exportProject(path: string): Promise<{ warnings: unknown[] }>` in `api.ts`; an `exportProject()` handler and an "Export to .knxproj…" button in `App.tsx`, gated the same way `saveProjectAs` is (`disabled={!tree}`) plus a second gate on `hasStorePath` (since export requires a saved `.knxdb` per this plan's ruling) — read `App.tsx`'s exact current `disabled` expressions before adding this, and match the existing style rather than inventing a new one.

- [ ] **Step 1: `api.ts`**

Add, next to `saveProjectAs`:

```typescript
export function exportProject(path: string): Promise<{ warnings: unknown[] }> {
  return request("/api/project/export", { method: "POST", body: JSON.stringify({ path }) });
}
```

(`warnings: unknown[]` is deliberate — the frontend only needs to know *how many* warnings there are and, per Step 2, stringify each one for a toast; it does not need a typed binding for `ExportWarningDto`'s shape the way `ProjectTree`/`DeviceDetail` have `ts-rs` bindings, since nothing renders a warning's individual fields yet.)

- [ ] **Step 2: `App.tsx`**

Add an `EXPORT_FILTER` constant next to `KNXDB_FILTER`:

```typescript
const EXPORT_FILTER = [{ name: "ETS project", extensions: ["knxproj"] }];
```

Add a handler next to `saveProjectAs`:

```typescript
async function exportProject() {
  const path = await pickSavePath(EXPORT_FILTER, "project.knxproj");
  if (!path) return;
  clearErrors();
  try {
    const { warnings } = await api.exportProject(path);
    for (const w of warnings) {
      pushError(typeof w === "object" && w !== null && "detail" in w ? String((w as { detail: unknown }).detail) : JSON.stringify(w));
    }
  } catch (e) {
    pushError(api.errorMessage(e));
  }
}
```

Not every `ExportWarningDto` variant has a `detail` field (`ManufacturerDataFromProductDb` has `entries`, not `detail`) — if `"detail" in w` is false, the `JSON.stringify(w)` fallback covers it; do not special-case each variant in the frontend, that belongs server-side if it ever needs a real `detail()` string the way `CreationDiagnostic` has one (out of scope for this plan — file it as a documentation note, not a blocking gap, since `Unsigned`'s `detail` is the one warning every export always carries and the others are rare/edge-case).

Add the button next to `Save As…`:

```tsx
<button onClick={exportProject} disabled={!tree || !hasStorePath}>
  Export to .knxproj…
</button>
```

Confirm `hasStorePath` is in scope at this point in the component (it already is — `saveProjectAs`/`openNativeProject` both set it).

- [ ] **Step 3: build check**

Run `cd apps/knx-web && npx tsc --noEmit && npm run build` — no new vitest is required for this task (the existing suite has no precedent for testing `App.tsx`'s button wiring directly, per this plan's own inspection of `apps/knx-web/src/*.test.*`; do not invent a new test harness for one button). If a reviewer disagrees, a `App.tsx` unit test is a Minor finding to park, not a blocking one.

---

## After all tasks: documentation

Not a separate SDD task (docs-only, no code review needed) — the controller updates these directly after Task 3's review is clean, same as prior cycles' doc-closeout pattern:

- `docs/GAP_ANALYSIS_ETS.md`: mark **T10** done, closing **C4**; update the "Not part of this session's delivery" style note in IMPLEMENTATION_STATUS if one exists for export.
- `docs/IMPLEMENTATION_STATUS.md`: add a dated entry describing what shipped (CLI `knx export`, `POST /api/project/export`, the frontend button) and the store-path precondition ruling.
- `docs/COMPATIBILITY.md`: if it has an explicit "export" capability row, update it; if not, no change needed (do not invent a new section).
- `.ai/CURRENT_STATE.md`: update per the Handover Protocol.
