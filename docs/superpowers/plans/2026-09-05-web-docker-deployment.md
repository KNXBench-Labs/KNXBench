# Web/Docker deployment target (knx-server) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a Docker-deployable web UI for KNXBench by converging the existing Tauri desktop app and a new web frontend onto one axum HTTP API, with Tauri becoming a thin local wrapper around that same server.

**Architecture:** New crate `apps/knx-server` (axum) hosts all command/domain logic, moved verbatim from `apps/knx-desktop/src-tauri`. `apps/knx-desktop/src-tauri` shrinks to spawning that server locally and pointing its WebView at it. The frontend moves from `apps/knx-desktop/src` to `apps/knx-web/src`, gets a `fetch()`-based API client instead of Tauri's `invoke()`, and keeps native file dialogs only inside the Tauri build (progressive enhancement) with a new server-mount picker + upload/download fallback for the plain web build.

**Tech Stack:** Rust (axum 0.8, tokio, tower-http), React 19 + Vite + TypeScript (unchanged), Tauri 2.

**Spec:** [docs/superpowers/specs/2026-09-05-web-docker-deployment-design.md](../specs/2026-09-05-web-docker-deployment-design.md)

## Global Constraints

- Single-user: one in-memory `Mutex`-guarded project, no sessions.
- LAN-only: no auth layer in this plan.
- `knx-core`/`knx-app`/`knx-store`/`knx-etsproj`/`knx-projection` get **zero** new dependencies (no axum, no HTTP) — `knx-server` is the only crate that speaks HTTP.
- Native file dialogs stay in the Tauri build via a `window.__TAURI__` check; the web build uses a server-mount picker + upload/download.
- Both file-handling strategies ship: server-mount (primary) and upload/download (fallback).
- Repository must build and all existing tests must keep passing after every task (`cargo test --workspace`, `npm run test` in the frontend package, `cargo run -p xtask -- check-layering`).
- `rust-version = "1.98.0"` (workspace floor) — don't pull a dependency that needs newer.

---

## Task 1: Scaffold `knx-server` with a health check

**Files:**
- Create: `apps/knx-server/Cargo.toml`
- Create: `apps/knx-server/src/lib.rs`
- Create: `apps/knx-server/src/main.rs`
- Create: `apps/knx-server/tests/healthz.rs`
- Modify: `Cargo.toml:3-15` (add member, add `axum`/`tokio`/`tower`/`tower-http` workspace deps)

**Interfaces:**
- Produces: `knx_server::app(state: Arc<knx_server::AppState>, static_dir: Option<PathBuf>) -> axum::Router` — every later task adds routes to this function. `knx_server::AppState` — placeholder in this task (empty marker struct), replaced by Task 2's real one.
- Consumes: nothing yet.

- [ ] **Step 1: Add workspace member and shared deps**

In `Cargo.toml`, add `"apps/knx-server",` to `members` (after `"apps/knx-desktop/src-tauri",`) and these lines to `[workspace.dependencies]`:

```toml
axum = "0.8"
tokio = "1"
tower = "0.5"
tower-http = "0.6"
```

- [ ] **Step 2: Write `Cargo.toml`**

```toml
[package]
name = "knx-server"
version = "0.0.0"
edition.workspace = true
license.workspace = true
repository.workspace = true
rust-version.workspace = true
publish = false

[[bin]]
name = "knx-server"
path = "src/main.rs"

[lib]
name = "knx_server"
path = "src/lib.rs"

[dependencies]
knx-app.workspace = true
knx-core.workspace = true
knx-etsproj.workspace = true
knx-projection.workspace = true
knx-store.workspace = true
axum = { workspace = true, features = ["multipart"] }
tokio = { workspace = true, features = ["full"] }
tower = { workspace = true, features = ["util"] }
tower-http = { workspace = true, features = ["fs"] }
serde.workspace = true
serde_json.workspace = true
tempfile.workspace = true
```

- [ ] **Step 3: Write the failing test**

```rust
// apps/knx-server/tests/healthz.rs
use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use tower::ServiceExt;

#[tokio::test]
async fn healthz_returns_ok() {
    let state = Arc::new(knx_server::AppState::default());
    let app = knx_server::app(state, None);
    let response = app
        .oneshot(Request::builder().uri("/healthz").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}
```

- [ ] **Step 4: Run test to verify it fails**

Run: `cargo test -p knx-server`
Expected: FAIL to compile — `knx_server::AppState`/`knx_server::app` don't exist yet.

- [ ] **Step 5: Write minimal `lib.rs`/`main.rs`**

```rust
// apps/knx-server/src/lib.rs
use std::sync::Arc;
use std::path::PathBuf;

use axum::Router;
use axum::routing::get;
use tower_http::services::ServeDir;

/// Placeholder until Task 2 moves the real state in from
/// `apps/knx-desktop/src-tauri`.
#[derive(Default)]
pub struct AppState;

pub type SharedState = Arc<AppState>;

/// Builds the full router: `/healthz` (and, from later tasks, `/api/*`),
/// plus — if `static_dir` is given — the built frontend served from `/`
/// as a fallback. `static_dir` is `None` for API-only test builds and the
/// Tauri dev branch, `Some(..)` for the standalone binary and the Tauri
/// release branch.
pub fn app(state: SharedState, static_dir: Option<PathBuf>) -> Router {
    let api = Router::new()
        .route("/healthz", get(|| async { "ok" }))
        .with_state(state);

    match static_dir {
        Some(dir) => api.fallback_service(ServeDir::new(dir)),
        None => api,
    }
}
```

```rust
// apps/knx-server/src/main.rs
use std::path::PathBuf;
use std::sync::Arc;

#[tokio::main]
async fn main() {
    let port: u16 = std::env::var("KNX_PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(8080);
    let static_dir = std::env::var("KNX_STATIC_DIR").ok().map(PathBuf::from);

    let state = Arc::new(knx_server::AppState::default());
    let app = knx_server::app(state, static_dir);

    let listener = tokio::net::TcpListener::bind(("0.0.0.0", port))
        .await
        .unwrap_or_else(|e| panic!("failed to bind 0.0.0.0:{port}: {e}"));
    println!("knx-server listening on :{port}");
    axum::serve(listener, app).await.expect("server error");
}
```

- [ ] **Step 6: Run test to verify it passes**

Run: `cargo test -p knx-server`
Expected: PASS

- [ ] **Step 7: Commit**

```bash
git add Cargo.toml apps/knx-server
git commit -m "feat(knx-server): scaffold axum crate with a health check"
```

---

## Task 2: Move `AppState` and command logic into `knx-server`

Copies (not yet removes) `AppState` and every `_impl`/state-mutating function from `apps/knx-desktop/src-tauri/src/lib.rs` into `knx-server`, so later HTTP-route tasks have something to call. `apps/knx-desktop` is untouched and still compiles — the duplication is removed in Task 9 once Tauri depends on `knx-server` instead.

While moving, two functions get a small cleanup: `open_project` and `open_native_project` currently exist as two independent copies each (one Tauri-command-only, reimplementing the `_impl` version's logic instead of calling it, because the `_impl` version only returns the display tree and the Tauri command also needs the `Project` to store). Both get a shared private helper so there's one implementation, not two.

**Files:**
- Create: `apps/knx-server/src/domain.rs`
- Create: `apps/knx-server/tests/command_dispatch.rs` (copied from `apps/knx-desktop/src-tauri/tests/command_dispatch.rs`, `knx_desktop_lib` → `knx_server`)
- Create: `apps/knx-server/tests/device_detail.rs` (same copy pattern)
- Create: `apps/knx-server/tests/open_reference_project.rs` (same copy pattern, path depth adjusted — see Step 4)
- Create: `apps/knx-server/tests/save_load_roundtrip.rs` (same copy pattern, path depth adjusted)
- Modify: `apps/knx-server/src/lib.rs` (replace placeholder `AppState`, re-export `domain`)

**Interfaces:**
- Produces: `knx_server::AppState::new(data_dir: PathBuf) -> AppState`, `AppState::default()` (temp-dir fallback for tests). `knx_server::open_project_impl(path: &Path) -> Result<ProjectTree, knx_app::AppError>`, `open_project(state: &AppState, path: &Path) -> Result<ProjectTree, String>`, `open_native_project_impl(path: &Path) -> Result<ProjectTree, String>`, `open_native_project(state: &AppState, path: &Path) -> Result<ProjectTree, String>`, `save_project_as_impl(path: &Path, project: &knx_core::Project) -> Result<(), String>`, `save_project_as(state: &AppState, path: &Path) -> Result<(), String>`, `save_project(state: &AppState) -> Result<(), String>`, `device_detail_impl(project: &knx_core::Project, device_id: u32) -> Result<DeviceDetail, String>`, `device_detail(state: &AppState, device_id: u32) -> Result<DeviceDetail, String>`, `set_individual_address_impl(state: &AppState, device_id: u32, address: Option<String>) -> Result<ProjectTree, String>`, `set_com_object_dpt_impl(state: &AppState, com_object_id: u32, dpt: Option<String>) -> Result<ProjectTree, String>`, `create_group_address_impl(state: &AppState, name: String, address: String) -> Result<ProjectTree, String>`, `delete_group_address_impl(state: &AppState, id: u32) -> Result<ProjectTree, String>`, `undo_impl(state: &AppState) -> Result<ProjectTree, String>`, `redo_impl(state: &AppState) -> Result<ProjectTree, String>`.
- Consumes: Task 1's `AppState` placeholder (replaced here) and `app()` (unaffected).

- [ ] **Step 1: Write `domain.rs`**

```rust
// apps/knx-server/src/domain.rs
//! Moved from `apps/knx-desktop/src-tauri/src/lib.rs` (the web/Docker
//! deployment target, see the design doc linked from the plan this task
//! belongs to) — these functions know nothing about Tauri or HTTP, only
//! `knx-core`/`knx-app`/`knx-store`/`knx-etsproj`/`knx-projection`.
//! `routes.rs`/`fs_routes.rs` are the only places that translate them to
//! JSON.
//!
//! Two independent file formats meet here, same as before the move:
//! `open_project`/`open_project_impl` import an ETS `.knxproj` (always
//! through a throwaway in-memory store — never touches a `.knxdb` file);
//! `save_project`/`save_project_as`/`open_native_project` persist/restore
//! this app's own project state as a `.knxdb` SQLite file. Neither path
//! calls into the other.

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use knx_app::{AppError, ImportOptions};
use knx_projection::ProjectTree;

pub struct AppState {
    pub project: Mutex<Option<knx_core::Project>>,
    /// The `.knxdb` file the in-memory project was last saved to or loaded
    /// from, if any. `None` until `save_project_as`/`open_native_project`
    /// sets it; plain `save_project` requires it already set.
    pub store_path: Mutex<Option<PathBuf>>,
    /// Every applied command's inverse, for undo/redo. Reset to empty on
    /// `open_project`/`open_native_project` — undo history never survives
    /// loading a different project, and is never persisted to `.knxdb`.
    pub command_stack: Mutex<knx_core::CommandStack>,
    /// (errors, warnings) from the initial import's `ImportReport`,
    /// reapplied to every tree rebuilt after a command/undo/redo — edits
    /// don't change what import lost. `(0, 0)` for a `.knxdb` native load.
    pub import_counts: Mutex<(usize, usize)>,
    /// Root directory `fs_routes.rs`'s `/api/fs/*` routes are confined to.
    /// Irrelevant to every function in this file.
    pub data_dir: PathBuf,
}

impl AppState {
    pub fn new(data_dir: PathBuf) -> Self {
        Self {
            project: Mutex::new(None),
            store_path: Mutex::new(None),
            command_stack: Mutex::new(knx_core::CommandStack::new()),
            import_counts: Mutex::new((0, 0)),
            data_dir,
        }
    }
}

impl Default for AppState {
    /// Test-only convenience — production always calls `AppState::new`
    /// with `KNX_DATA_DIR`. Falls back to the OS temp dir so tests that
    /// never touch `/api/fs/*` don't need to care.
    fn default() -> Self {
        Self::new(std::env::temp_dir())
    }
}

/// Fills in `tree.errors`/`tree.warnings` from `report`, keeping a genuine
/// `Severity::Error` (data actually lost or misread) distinct from
/// everything else merely worth a look — the same split
/// `apps/knx-cli/src/main.rs`'s `error_count()` draws, since `ImportReport`
/// keeps both severities in one `errors` Vec.
fn apply_report_counts(tree: &mut ProjectTree, report: &knx_etsproj::ImportReport) {
    let error_count = report
        .errors
        .iter()
        .filter(|e| e.severity == knx_etsproj::report::Severity::Error)
        .count();
    let warning_count = report.errors.len() - error_count;

    tree.errors = error_count;
    tree.warnings =
        warning_count + report.unknown.len() + report.conflicts.len() + report.unsupported.len();
}

/// Shared by `open_project_impl` (display-only) and `open_project`
/// (display + replaces `state`'s project) so there is exactly one import
/// implementation instead of two.
fn import_and_project(path: &Path) -> Result<(ProjectTree, knx_core::Project), AppError> {
    let conn = knx_store::open_and_migrate_in_memory()?;
    let imported = knx_app::import_ets_project_with(path, &conn, ImportOptions::default())?;
    let mut tree = knx_projection::build_project_tree(&imported.project);
    apply_report_counts(&mut tree, &imported.report);
    Ok((tree, imported.project))
}

/// Imports `path` and projects it without touching `state` — what
/// `open_reference_project.rs` exercises directly, no server needed.
pub fn open_project_impl(path: &Path) -> Result<ProjectTree, AppError> {
    import_and_project(path).map(|(tree, _)| tree)
}

/// Imports `path`, replaces `state`'s project, and resets undo history and
/// import counts — what the `/api/project/import` route calls.
pub fn open_project(state: &AppState, path: &Path) -> Result<ProjectTree, String> {
    let (tree, project) = import_and_project(path).map_err(|e| e.to_string())?;
    *state.project.lock().expect("state mutex poisoned") = Some(project);
    *state.command_stack.lock().expect("state mutex poisoned") = knx_core::CommandStack::new();
    *state.import_counts.lock().expect("state mutex poisoned") = (tree.errors, tree.warnings);
    Ok(tree)
}

/// Persists `project` to a fresh or existing `.knxdb` file at `path`,
/// overwriting whatever it held.
pub fn save_project_as_impl(path: &Path, project: &knx_core::Project) -> Result<(), String> {
    let conn = knx_store::open_and_migrate(path).map_err(|e| e.to_string())?;
    knx_store::save_project(&conn, project).map_err(|e| e.to_string())
}

/// Shared by `open_native_project_impl` (display-only) and
/// `open_native_project` (display + replaces `state`'s project) — same
/// reasoning as `import_and_project` above.
fn load_native(path: &Path) -> Result<(ProjectTree, knx_core::Project), String> {
    let conn = knx_store::open_and_migrate(path).map_err(|e| e.to_string())?;
    let project = knx_store::load_project(&conn).map_err(|e| e.to_string())?;
    let tree = knx_projection::build_project_tree(&project);
    Ok((tree, project))
}

/// Loads a `.knxdb` file at `path` and projects it, without touching
/// `state`. No `ImportReport` exists for a native load — nothing was
/// reinterpreted from an external format — so `tree.errors`/`tree.warnings`
/// stay at their default zero.
pub fn open_native_project_impl(path: &Path) -> Result<ProjectTree, String> {
    load_native(path).map(|(tree, _)| tree)
}

/// Loads a `.knxdb` file at `path`, replaces `state`'s project, and points
/// `store_path` at it — what the `/api/project/open` route calls.
pub fn open_native_project(state: &AppState, path: &Path) -> Result<ProjectTree, String> {
    let (tree, project) = load_native(path)?;
    *state.project.lock().expect("state mutex poisoned") = Some(project);
    *state.store_path.lock().expect("state mutex poisoned") = Some(path.to_path_buf());
    *state.command_stack.lock().expect("state mutex poisoned") = knx_core::CommandStack::new();
    *state.import_counts.lock().expect("state mutex poisoned") = (0, 0);
    Ok(tree)
}

pub fn save_project_as(state: &AppState, path: &Path) -> Result<(), String> {
    {
        let project = state.project.lock().expect("state mutex poisoned");
        let project = project.as_ref().ok_or("no project open")?;
        save_project_as_impl(path, project)?;
    }
    *state.store_path.lock().expect("state mutex poisoned") = Some(path.to_path_buf());
    Ok(())
}

pub fn save_project(state: &AppState) -> Result<(), String> {
    let path = state
        .store_path
        .lock()
        .expect("state mutex poisoned")
        .clone()
        .ok_or("no save location yet — use Save As")?;
    let project = state.project.lock().expect("state mutex poisoned");
    let project = project.as_ref().ok_or("no project open")?;
    save_project_as_impl(&path, project)
}

/// Projects one device's detail. `Err` names the device id when it no
/// longer exists in `project` — a stale selection after an edit, for
/// instance.
pub fn device_detail_impl(
    project: &knx_core::Project,
    device_id: u32,
) -> Result<knx_projection::DeviceDetail, String> {
    knx_projection::build_device_detail(project, knx_core::DeviceId(device_id))
        .ok_or_else(|| format!("device {device_id} not found"))
}

pub fn device_detail(state: &AppState, device_id: u32) -> Result<knx_projection::DeviceDetail, String> {
    let project = state.project.lock().expect("state mutex poisoned");
    let project = project.as_ref().ok_or("no project open")?;
    device_detail_impl(project, device_id)
}

/// Rebuilds `tree` from `project` and overlays the counts/undo-redo state
/// that `build_project_tree` alone cannot know about.
fn tree_with_state(
    project: &knx_core::Project,
    stack: &knx_core::CommandStack,
    import_counts: (usize, usize),
) -> knx_projection::ProjectTree {
    let mut tree = knx_projection::build_project_tree(project);
    tree.errors = import_counts.0;
    tree.warnings = import_counts.1;
    tree.can_undo = stack.can_undo();
    tree.can_redo = stack.can_redo();
    tree
}

fn apply(state: &AppState, cmd: knx_core::Command) -> Result<knx_projection::ProjectTree, String> {
    let mut project = state.project.lock().expect("state mutex poisoned");
    let project = project.as_mut().ok_or("no project open")?;
    let mut stack = state.command_stack.lock().expect("state mutex poisoned");
    stack.do_command(project, cmd).map_err(|e| e.to_string())?;
    let import_counts = *state.import_counts.lock().expect("state mutex poisoned");
    Ok(tree_with_state(project, &stack, import_counts))
}

pub fn set_individual_address_impl(
    state: &AppState,
    device_id: u32,
    address: Option<String>,
) -> Result<knx_projection::ProjectTree, String> {
    let address = match address {
        Some(s) => Some(
            s.parse::<knx_core::IndividualAddress>()
                .map_err(|e| e.to_string())?,
        ),
        None => None,
    };
    apply(
        state,
        knx_core::Command::SetIndividualAddress {
            device: knx_core::DeviceId(device_id),
            address,
        },
    )
}

pub fn set_com_object_dpt_impl(
    state: &AppState,
    com_object_id: u32,
    dpt: Option<String>,
) -> Result<knx_projection::ProjectTree, String> {
    let dpt = match dpt {
        Some(s) => Some(knx_core::DptRef::parse(&s).map_err(|e| e.to_string())?),
        None => None,
    };
    apply(
        state,
        knx_core::Command::SetComObjectDpt {
            com_object: knx_core::ComObjectInstanceId(com_object_id),
            dpt,
        },
    )
}

/// Allocates a fresh `GroupAddressId` and creates a new group address in
/// `installations[0]` — the only installation any `Command` targets
/// (`Command::apply`'s own doc comment). `address` is parsed against the
/// project's own `GroupAddressStyle`. `entry.source` is empty: a
/// UI-created object has no ETS origin to preserve.
pub fn create_group_address_impl(
    state: &AppState,
    name: String,
    address: String,
) -> Result<knx_projection::ProjectTree, String> {
    let cmd = {
        let mut project = state.project.lock().expect("state mutex poisoned");
        let project = project.as_mut().ok_or("no project open")?;
        let style = project.info.group_address_style;
        let address = knx_core::GroupAddress::parse(&address, style).map_err(|e| e.to_string())?;
        let id = project.ids.next_group_address_id();
        knx_core::Command::CreateGroupAddress {
            entry: knx_core::GroupAddressEntry {
                id,
                source: knx_core::SourceRef {
                    path: String::new(),
                    ets_id: String::new(),
                },
                name,
                address,
                central: false,
                unfiltered: false,
                range: None,
            },
        }
    };
    apply(state, cmd)
}

pub fn delete_group_address_impl(
    state: &AppState,
    id: u32,
) -> Result<knx_projection::ProjectTree, String> {
    apply(
        state,
        knx_core::Command::DeleteGroupAddress {
            id: knx_core::GroupAddressId(id),
        },
    )
}

pub fn undo_impl(state: &AppState) -> Result<knx_projection::ProjectTree, String> {
    let mut project = state.project.lock().expect("state mutex poisoned");
    let project = project.as_mut().ok_or("no project open")?;
    let mut stack = state.command_stack.lock().expect("state mutex poisoned");
    stack.undo(project).map_err(|e| e.to_string())?;
    let import_counts = *state.import_counts.lock().expect("state mutex poisoned");
    Ok(tree_with_state(project, &stack, import_counts))
}

pub fn redo_impl(state: &AppState) -> Result<knx_projection::ProjectTree, String> {
    let mut project = state.project.lock().expect("state mutex poisoned");
    let project = project.as_mut().ok_or("no project open")?;
    let mut stack = state.command_stack.lock().expect("state mutex poisoned");
    stack.redo(project).map_err(|e| e.to_string())?;
    let import_counts = *state.import_counts.lock().expect("state mutex poisoned");
    Ok(tree_with_state(project, &stack, import_counts))
}
```

- [ ] **Step 2: Wire it into `lib.rs`**

In `apps/knx-server/src/lib.rs`, delete the placeholder `AppState` struct and add:

```rust
mod domain;
pub use domain::*;
```

- [ ] **Step 3: Copy the four test files, renaming the crate they reference**

```bash
for f in command_dispatch device_detail open_reference_project save_load_roundtrip; do
  cp "apps/knx-desktop/src-tauri/tests/$f.rs" "apps/knx-server/tests/$f.rs"
  sed -i 's/knx_desktop_lib/knx_server/g' "apps/knx-server/tests/$f.rs"
done
```

- [ ] **Step 4: Fix the path depth in the two tests that locate the reference project**

`apps/knx-server` is two directories below the workspace root (`apps/knx-server`), not three (`apps/knx-desktop/src-tauri`). In `apps/knx-server/tests/open_reference_project.rs` and `apps/knx-server/tests/save_load_roundtrip.rs`, change:

```rust
fn workspace_root() -> PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(std::path::Path::parent)
        .and_then(std::path::Path::parent)
        .expect("crate lives at <root>/apps/knx-desktop/src-tauri")
        .to_path_buf()
}
```

to:

```rust
fn workspace_root() -> PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(std::path::Path::parent)
        .expect("crate lives at <root>/apps/knx-server")
        .to_path_buf()
}
```

- [ ] **Step 5: Run test to verify it fails, then passes**

Run: `cargo test -p knx-server`
Expected first: FAIL (files reference `knx_server::AppState::default()` etc. — should already resolve after Step 2; if it fails to compile, the most likely cause is a leftover `knx_desktop_lib` the `sed` missed — check with `grep -rn knx_desktop_lib apps/knx-server`).
After Step 2 is in place: PASS — same assertions as `cargo test -p knx-desktop` today, now run against `knx-server`.

- [ ] **Step 6: Commit**

```bash
git add apps/knx-server
git commit -m "feat(knx-server): move AppState and command logic in from knx-desktop

Copied, not moved — apps/knx-desktop/src-tauri keeps its own copy until
it's rewired to depend on knx-server (later task) so the repo stays
buildable at every commit. open_project/open_native_project also lose
their duplicate hand-rolled copies in favor of a shared private helper."
```

---

## Task 3: HTTP routes — project import/open

**Files:**
- Create: `apps/knx-server/src/errors.rs`
- Create: `apps/knx-server/src/routes.rs` (started here, grows through Task 7)
- Create: `apps/knx-server/tests/http_project_routes.rs` (started here, grows through Task 7)
- Modify: `apps/knx-server/src/lib.rs` (wire `routes::project_routes()` into `app()`)

**Interfaces:**
- Consumes: `domain::open_project`, `domain::open_native_project` (Task 2).
- Produces: `routes::project_routes() -> Router<SharedState>`, mounted at `/api/project/import` (POST) and `/api/project/open` (POST). `errors::ApiError`, used by every later route task.

- [ ] **Step 1: Write `errors.rs`**

```rust
// apps/knx-server/src/errors.rs
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::json;

/// Every function in `domain.rs` returns `Result<T, String>` (or
/// `Result<T, knx_app::AppError>` for the `_impl` variants, converted at
/// the route boundary) — there is no error taxonomy below this layer to
/// switch on. The 400/500 split here is drawn by *which* operation
/// failed, not by inspecting the message: `open_project`/
/// `open_native_project`/`save_project`/`save_project_as` touch the
/// filesystem and `knx-store`, so their failures (missing file, corrupt
/// database, disk full) are environment problems -> `internal` (500).
/// Every other route only ever touches already-loaded in-memory state, so
/// its failures (malformed input, stale id, "no project open") are always
/// the caller's to fix -> `bad_request` (400).
pub struct ApiError {
    status: StatusCode,
    message: String,
}

impl ApiError {
    pub fn bad_request(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            message: message.into(),
        }
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            message: message.into(),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.status, Json(json!({ "error": self.message }))).into_response()
    }
}
```

- [ ] **Step 2: Write the failing test**

```rust
// apps/knx-server/tests/http_project_routes.rs
use std::path::{Path, PathBuf};
use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use serde_json::{json, Value};
use tower::ServiceExt;

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("crate lives at <root>/apps/knx-server")
        .to_path_buf()
}

fn reference_ets4_path() -> PathBuf {
    workspace_root().join("Unser Zuhause ets4 - 2025-12-15.knxproj")
}

async fn body_json(response: axum::response::Response) -> Value {
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

#[tokio::test]
async fn importing_the_reference_project_returns_the_golden_counts() {
    let state = Arc::new(knx_server::AppState::default());
    let app = knx_server::app(state, None);

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/project/import")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({ "path": reference_ets4_path().to_string_lossy() }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let tree = body_json(response).await;
    assert_eq!(tree["errors"], 0);
    assert_eq!(tree["warnings"], 2);
    assert_eq!(tree["installations"][0]["topology"][0]["lines"].as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn importing_a_missing_file_is_a_500() {
    let state = Arc::new(knx_server::AppState::default());
    let app = knx_server::app(state, None);

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/project/import")
                .header("content-type", "application/json")
                .body(Body::from(json!({ "path": "/does/not/exist.knxproj" }).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    let body = body_json(response).await;
    assert!(body["error"].as_str().unwrap().len() > 0);
}
```

- [ ] **Step 3: Run test to verify it fails**

Run: `cargo test -p knx-server --test http_project_routes`
Expected: FAIL to compile — `/api/project/import` route doesn't exist.

- [ ] **Step 4: Write `routes.rs` and wire it in**

```rust
// apps/knx-server/src/routes.rs
use std::path::Path;

use axum::extract::State;
use axum::routing::post;
use axum::{Json, Router};
use serde::Deserialize;

use crate::domain;
use crate::errors::ApiError;
use crate::SharedState;

pub fn project_routes() -> Router<SharedState> {
    Router::new()
        .route("/api/project/import", post(import_project))
        .route("/api/project/open", post(open_native_project))
}

#[derive(Deserialize)]
pub(crate) struct PathBody {
    pub(crate) path: String,
}

async fn import_project(
    State(state): State<SharedState>,
    Json(body): Json<PathBody>,
) -> Result<Json<knx_projection::ProjectTree>, ApiError> {
    domain::open_project(&state, Path::new(&body.path))
        .map(Json)
        .map_err(ApiError::internal)
}

async fn open_native_project(
    State(state): State<SharedState>,
    Json(body): Json<PathBody>,
) -> Result<Json<knx_projection::ProjectTree>, ApiError> {
    domain::open_native_project(&state, Path::new(&body.path))
        .map(Json)
        .map_err(ApiError::internal)
}
```

In `apps/knx-server/src/lib.rs`, add `mod errors; mod routes;` and change `app()`:

```rust
pub fn app(state: SharedState, static_dir: Option<PathBuf>) -> Router {
    let api = Router::new()
        .merge(routes::project_routes())
        .route("/healthz", get(|| async { "ok" }))
        .with_state(state);

    match static_dir {
        Some(dir) => api.fallback_service(ServeDir::new(dir)),
        None => api,
    }
}
```

- [ ] **Step 5: Run test to verify it passes**

Run: `cargo test -p knx-server`
Expected: PASS (all of Task 1/2/3's tests)

- [ ] **Step 6: Commit**

```bash
git add apps/knx-server
git commit -m "feat(knx-server): POST /api/project/import and /api/project/open"
```

---

## Task 4: HTTP routes — save/save-as

**Files:**
- Modify: `apps/knx-server/src/routes.rs` (add two routes)
- Modify: `apps/knx-server/tests/http_project_routes.rs` (add tests)

**Interfaces:**
- Consumes: `domain::save_project`, `domain::save_project_as`, `routes::PathBody` (Task 3).
- Produces: `/api/project/save` (POST), `/api/project/save-as` (POST), both `Result<(), ApiError>`.

- [ ] **Step 1: Write the failing test**

Append to `apps/knx-server/tests/http_project_routes.rs`:

```rust
#[tokio::test]
async fn saving_without_an_open_project_is_a_500() {
    let state = Arc::new(knx_server::AppState::default());
    let app = knx_server::app(state, None);

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/project/save")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
}

#[tokio::test]
async fn importing_then_saving_as_then_reopening_round_trips() {
    let state = Arc::new(knx_server::AppState::default());
    let app = knx_server::app(state, None);
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("roundtrip.knxdb");

    let import_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/project/import")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({ "path": reference_ets4_path().to_string_lossy() }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(import_response.status(), StatusCode::OK);

    let save_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/project/save-as")
                .header("content-type", "application/json")
                .body(Body::from(json!({ "path": db_path.to_string_lossy() }).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(save_response.status(), StatusCode::OK);

    let reopen_response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/project/open")
                .header("content-type", "application/json")
                .body(Body::from(json!({ "path": db_path.to_string_lossy() }).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(reopen_response.status(), StatusCode::OK);
    let tree = body_json(reopen_response).await;
    assert_eq!(tree["errors"], 0);
    assert_eq!(tree["warnings"], 0);
}
```

This test needs `Router` to be `Clone` for the three sequential `oneshot` calls sharing one `AppState` — `axum::Router` already implements `Clone` (cloning it clones the `Arc`-backed route table and shares the same `State`), so `app.clone()` is enough; no change needed elsewhere.

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p knx-server --test http_project_routes`
Expected: FAIL to compile — `/api/project/save`/`/api/project/save-as` don't exist.

- [ ] **Step 3: Add the routes**

In `apps/knx-server/src/routes.rs`, add to `project_routes()`:

```rust
        .route("/api/project/save", post(save_project))
        .route("/api/project/save-as", post(save_project_as))
```

and the handlers:

```rust
async fn save_project(State(state): State<SharedState>) -> Result<(), ApiError> {
    domain::save_project(&state).map_err(ApiError::internal)
}

async fn save_project_as(
    State(state): State<SharedState>,
    Json(body): Json<PathBody>,
) -> Result<(), ApiError> {
    domain::save_project_as(&state, Path::new(&body.path)).map_err(ApiError::internal)
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p knx-server`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add apps/knx-server
git commit -m "feat(knx-server): POST /api/project/save and /save-as"
```

---

## Task 5: HTTP route — device detail

**Files:**
- Modify: `apps/knx-server/src/routes.rs`
- Create: `apps/knx-server/tests/http_device_detail.rs`

**Interfaces:**
- Consumes: `domain::device_detail` (Task 2).
- Produces: `/api/device/{id}` (GET) -> `Json<DeviceDetail>`.

- [ ] **Step 1: Write the failing test**

```rust
// apps/knx-server/tests/http_device_detail.rs
use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use knx_core::{DeviceId, DeviceInstance, CommissioningState, Language, Project, SourceRef};
use tower::ServiceExt;

fn state_with_one_device() -> knx_server::AppState {
    let mut project = Project::new(Language("en".into()));
    project.devices.insert(DeviceInstance {
        id: DeviceId(1),
        source: SourceRef { path: "t".into(), ets_id: "t".into() },
        name: "D1".into(),
        description: None,
        address: None,
        product_ref: "P".into(),
        program_ref: "H".into(),
        commissioning: CommissioningState::default(),
        visibility_calculated: true,
        com_objects: vec![],
        binary_data: vec![],
    });
    let state = knx_server::AppState::default();
    *state.project.lock().unwrap() = Some(project);
    state
}

#[tokio::test]
async fn device_detail_returns_the_device() {
    let state = Arc::new(state_with_one_device());
    let app = knx_server::app(state, None);

    let response = app
        .oneshot(Request::builder().uri("/api/device/1").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn device_detail_for_a_missing_device_is_a_400() {
    let state = Arc::new(state_with_one_device());
    let app = knx_server::app(state, None);

    let response = app
        .oneshot(Request::builder().uri("/api/device/999").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p knx-server --test http_device_detail`
Expected: FAIL to compile — `/api/device/{id}` doesn't exist.

- [ ] **Step 3: Add the route**

In `apps/knx-server/src/routes.rs`, add imports `use axum::extract::Path as AxumPath;` and to `project_routes()`:

```rust
        .route("/api/device/{id}", axum::routing::get(device_detail))
```

and the handler:

```rust
async fn device_detail(
    State(state): State<SharedState>,
    AxumPath(id): AxumPath<u32>,
) -> Result<Json<knx_projection::DeviceDetail>, ApiError> {
    domain::device_detail(&state, id)
        .map(Json)
        .map_err(ApiError::bad_request)
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p knx-server`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add apps/knx-server
git commit -m "feat(knx-server): GET /api/device/{id}"
```

---

## Task 6: HTTP routes — edit commands

**Files:**
- Modify: `apps/knx-server/src/routes.rs`
- Create: `apps/knx-server/tests/http_edit_routes.rs`

**Interfaces:**
- Consumes: `domain::set_individual_address_impl`, `domain::set_com_object_dpt_impl`, `domain::create_group_address_impl`, `domain::delete_group_address_impl` (Task 2).
- Produces: `/api/individual-address` (POST), `/api/com-object-dpt` (POST), `/api/group-addresses` (POST), `/api/group-addresses/{id}` (DELETE) — all `Result<Json<ProjectTree>, ApiError>`.

- [ ] **Step 1: Write the failing test**

```rust
// apps/knx-server/tests/http_edit_routes.rs
use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use knx_core::{CompletionStatus, Installation, InstallationId, Language, Project, Topology};
use serde_json::{json, Value};
use tower::ServiceExt;

fn state_with_one_installation() -> knx_server::AppState {
    let mut project = Project::new(Language("en".into()));
    project.installations.push(Installation {
        id: InstallationId(0),
        name: "I".into(),
        default_line: None,
        multicast_address: None,
        completion: CompletionStatus::FinishedDesign,
        topology: Topology { areas: vec![], lines: vec![], unassigned: vec![] },
        buildings: vec![],
        group_ranges: vec![],
        group_addresses: vec![],
        parameters: vec![],
    });
    let state = knx_server::AppState::default();
    *state.project.lock().unwrap() = Some(project);
    state
}

async fn body_json(response: axum::response::Response) -> Value {
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

#[tokio::test]
async fn creating_then_deleting_a_group_address_round_trips() {
    let state = Arc::new(state_with_one_installation());
    let app = knx_server::app(state, None);

    let create = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/group-addresses")
                .header("content-type", "application/json")
                .body(Body::from(json!({ "name": "Living room", "address": "1/1/1" }).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(create.status(), StatusCode::OK);
    let tree = body_json(create).await;
    let ga_id = tree["installations"][0]["group_addresses"][0]["id"].as_u64().unwrap();

    let delete = app
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/group-addresses/{ga_id}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(delete.status(), StatusCode::OK);
    let tree = body_json(delete).await;
    assert!(tree["installations"][0]["group_addresses"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn creating_a_malformed_group_address_is_a_400() {
    let state = Arc::new(state_with_one_installation());
    let app = knx_server::app(state, None);

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/group-addresses")
                .header("content-type", "application/json")
                .body(Body::from(json!({ "name": "GA", "address": "not-an-address" }).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p knx-server --test http_edit_routes`
Expected: FAIL to compile — the routes don't exist.

- [ ] **Step 3: Add the routes**

In `apps/knx-server/src/routes.rs`, add `use axum::routing::delete;` and to `project_routes()`:

```rust
        .route("/api/individual-address", post(set_individual_address))
        .route("/api/com-object-dpt", post(set_com_object_dpt))
        .route("/api/group-addresses", post(create_group_address))
        .route("/api/group-addresses/{id}", delete(delete_group_address))
```

and the handlers:

```rust
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SetIndividualAddressBody {
    device_id: u32,
    address: Option<String>,
}

async fn set_individual_address(
    State(state): State<SharedState>,
    Json(body): Json<SetIndividualAddressBody>,
) -> Result<Json<knx_projection::ProjectTree>, ApiError> {
    domain::set_individual_address_impl(&state, body.device_id, body.address)
        .map(Json)
        .map_err(ApiError::bad_request)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SetComObjectDptBody {
    com_object_id: u32,
    dpt: Option<String>,
}

async fn set_com_object_dpt(
    State(state): State<SharedState>,
    Json(body): Json<SetComObjectDptBody>,
) -> Result<Json<knx_projection::ProjectTree>, ApiError> {
    domain::set_com_object_dpt_impl(&state, body.com_object_id, body.dpt)
        .map(Json)
        .map_err(ApiError::bad_request)
}

#[derive(Deserialize)]
struct CreateGroupAddressBody {
    name: String,
    address: String,
}

async fn create_group_address(
    State(state): State<SharedState>,
    Json(body): Json<CreateGroupAddressBody>,
) -> Result<Json<knx_projection::ProjectTree>, ApiError> {
    domain::create_group_address_impl(&state, body.name, body.address)
        .map(Json)
        .map_err(ApiError::bad_request)
}

async fn delete_group_address(
    State(state): State<SharedState>,
    AxumPath(id): AxumPath<u32>,
) -> Result<Json<knx_projection::ProjectTree>, ApiError> {
    domain::delete_group_address_impl(&state, id)
        .map(Json)
        .map_err(ApiError::bad_request)
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p knx-server`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add apps/knx-server
git commit -m "feat(knx-server): edit-command routes (address, DPT, group addresses)"
```

---

## Task 7: HTTP routes — undo/redo

**Files:**
- Modify: `apps/knx-server/src/routes.rs`
- Modify: `apps/knx-server/tests/http_edit_routes.rs` (add a test)

**Interfaces:**
- Consumes: `domain::undo_impl`, `domain::redo_impl` (Task 2).
- Produces: `/api/undo` (POST), `/api/redo` (POST).

- [ ] **Step 1: Write the failing test**

Append to `apps/knx-server/tests/http_edit_routes.rs`:

```rust
#[tokio::test]
async fn undo_after_create_removes_it_and_redo_brings_it_back() {
    let state = Arc::new(state_with_one_installation());
    let app = knx_server::app(state, None);

    app.clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/group-addresses")
                .header("content-type", "application/json")
                .body(Body::from(json!({ "name": "GA", "address": "1/1/1" }).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    let undo = app
        .clone()
        .oneshot(Request::builder().method("POST").uri("/api/undo").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(undo.status(), StatusCode::OK);
    let tree = body_json(undo).await;
    assert!(tree["installations"][0]["group_addresses"].as_array().unwrap().is_empty());

    let redo = app
        .oneshot(Request::builder().method("POST").uri("/api/redo").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(redo.status(), StatusCode::OK);
    let tree = body_json(redo).await;
    assert_eq!(tree["installations"][0]["group_addresses"].as_array().unwrap().len(), 1);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p knx-server --test http_edit_routes`
Expected: FAIL to compile — `/api/undo`/`/api/redo` don't exist.

- [ ] **Step 3: Add the routes**

In `apps/knx-server/src/routes.rs`, add to `project_routes()`:

```rust
        .route("/api/undo", post(undo))
        .route("/api/redo", post(redo))
```

and the handlers:

```rust
async fn undo(State(state): State<SharedState>) -> Result<Json<knx_projection::ProjectTree>, ApiError> {
    domain::undo_impl(&state).map(Json).map_err(ApiError::bad_request)
}

async fn redo(State(state): State<SharedState>) -> Result<Json<knx_projection::ProjectTree>, ApiError> {
    domain::redo_impl(&state).map(Json).map_err(ApiError::bad_request)
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p knx-server`
Expected: PASS — every project route from the API table in the spec now exists.

- [ ] **Step 5: Commit**

```bash
git add apps/knx-server
git commit -m "feat(knx-server): POST /api/undo and /api/redo"
```

---

## Task 8: HTTP routes — filesystem (list/upload/download)

**Files:**
- Create: `apps/knx-server/src/fs_routes.rs`
- Create: `apps/knx-server/tests/http_fs_routes.rs`
- Modify: `apps/knx-server/src/lib.rs` (merge `fs_routes::fs_routes()`)

**Interfaces:**
- Consumes: `AppState.data_dir` (Task 2), `domain::save_project_as_impl` (Task 2).
- Produces: `GET /api/fs/list?path=`, `POST /api/fs/upload`, `GET /api/project/download`.

- [ ] **Step 1: Write the failing test**

```rust
// apps/knx-server/tests/http_fs_routes.rs
use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use serde_json::Value;
use tower::ServiceExt;

fn state_with_data_dir() -> (knx_server::AppState, tempfile::TempDir) {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("a.knxproj"), b"not a real project, just a listing fixture").unwrap();
    std::fs::create_dir(dir.path().join("sub")).unwrap();
    (knx_server::AppState::new(dir.path().to_path_buf()), dir)
}

async fn body_json(response: axum::response::Response) -> Value {
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

#[tokio::test]
async fn listing_the_data_dir_root_shows_its_entries() {
    let (state, _dir) = state_with_data_dir();
    let app = knx_server::app(Arc::new(state), None);

    let response = app
        .oneshot(Request::builder().uri("/api/fs/list").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let entries = body_json(response).await;
    let names: Vec<&str> = entries.as_array().unwrap().iter().map(|e| e["name"].as_str().unwrap()).collect();
    assert!(names.contains(&"a.knxproj"));
    assert!(names.contains(&"sub"));
}

#[tokio::test]
async fn listing_a_path_that_escapes_the_data_dir_is_a_400() {
    let (state, _dir) = state_with_data_dir();
    let app = knx_server::app(Arc::new(state), None);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/fs/list?path=../../../../etc")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn downloading_with_no_project_open_is_a_400() {
    let (state, _dir) = state_with_data_dir();
    let app = knx_server::app(Arc::new(state), None);

    let response = app
        .oneshot(Request::builder().uri("/api/project/download").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p knx-server --test http_fs_routes`
Expected: FAIL to compile — `/api/fs/list`/`/api/project/download` don't exist, `AppState::new` isn't `pub` from outside `domain` yet (it already is, via `pub use domain::*` from Task 2).

- [ ] **Step 3: Write `fs_routes.rs`**

```rust
// apps/knx-server/src/fs_routes.rs
//! `/api/fs/*` and `/api/project/download` — the two file-handling
//! strategies for the web build (server-mount and upload/download; the
//! Tauri build skips these entirely in favor of native OS dialogs, see
//! `apps/knx-web/src/filePicker.ts`).
use std::path::{Path, PathBuf};

use axum::extract::{Multipart, Query, State};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};

use crate::domain;
use crate::errors::ApiError;
use crate::SharedState;

pub fn fs_routes() -> Router<SharedState> {
    Router::new()
        .route("/api/fs/list", get(list_dir))
        .route("/api/fs/upload", post(upload))
        .route("/api/project/download", get(download))
}

/// Resolves `relative` against `data_dir` and confirms the result still
/// lives under it — the only thing standing between `/api/fs/list` and a
/// `path=../../etc` escape out of the mounted volume. `canonicalize`
/// requires the path to exist, which also rejects a nonexistent `path`
/// with a clear "does not exist" instead of a confusing filesystem error
/// later.
fn resolve_in_data_dir(data_dir: &Path, relative: &str) -> Result<PathBuf, ApiError> {
    let candidate = data_dir.join(relative.trim_start_matches('/'));
    let canonical_root = data_dir
        .canonicalize()
        .map_err(|e| ApiError::internal(format!("data dir unreadable: {e}")))?;
    let canonical = candidate
        .canonicalize()
        .map_err(|_| ApiError::bad_request("path does not exist"))?;
    if !canonical.starts_with(&canonical_root) {
        return Err(ApiError::bad_request("path escapes the data directory"));
    }
    Ok(canonical)
}

#[derive(Deserialize)]
struct ListQuery {
    path: Option<String>,
}

#[derive(Serialize)]
struct DirEntryDto {
    name: String,
    is_dir: bool,
}

async fn list_dir(
    State(state): State<SharedState>,
    Query(q): Query<ListQuery>,
) -> Result<Json<Vec<DirEntryDto>>, ApiError> {
    let dir = resolve_in_data_dir(&state.data_dir, q.path.as_deref().unwrap_or(""))?;
    if !dir.is_dir() {
        return Err(ApiError::bad_request("not a directory"));
    }
    let mut entries = Vec::new();
    for entry in std::fs::read_dir(&dir).map_err(|e| ApiError::internal(e.to_string()))? {
        let entry = entry.map_err(|e| ApiError::internal(e.to_string()))?;
        let is_dir = entry
            .file_type()
            .map_err(|e| ApiError::internal(e.to_string()))?
            .is_dir();
        entries.push(DirEntryDto {
            name: entry.file_name().to_string_lossy().into_owned(),
            is_dir,
        });
    }
    entries.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(Json(entries))
}

#[derive(Serialize)]
struct UploadResponse {
    path: String,
}

async fn upload(
    State(state): State<SharedState>,
    mut multipart: Multipart,
) -> Result<Json<UploadResponse>, ApiError> {
    let uploads_dir = state.data_dir.join("uploads");
    std::fs::create_dir_all(&uploads_dir).map_err(|e| ApiError::internal(e.to_string()))?;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| ApiError::bad_request(e.to_string()))?
    {
        let Some(filename) = field.file_name().map(str::to_owned) else {
            continue;
        };
        // `Path::file_name` drops any directory components the client
        // sent (`../../etc/passwd` -> `passwd`) — the only sanitization
        // an upload's filename needs, since it never becomes a directory.
        let safe_name = Path::new(&filename)
            .file_name()
            .ok_or_else(|| ApiError::bad_request("empty filename"))?;
        let dest = uploads_dir.join(safe_name);
        let bytes = field
            .bytes()
            .await
            .map_err(|e| ApiError::bad_request(e.to_string()))?;
        std::fs::write(&dest, &bytes).map_err(|e| ApiError::internal(e.to_string()))?;
        let relative = dest.strip_prefix(&state.data_dir).unwrap_or(&dest);
        return Ok(Json(UploadResponse {
            path: relative.to_string_lossy().into_owned(),
        }));
    }
    Err(ApiError::bad_request("no file field in upload"))
}

/// Always serializes the current in-memory project fresh into a temp
/// `.knxdb` and streams that — regardless of whether it was ever saved to
/// `store_path` before, so "download" always reflects the latest edits.
async fn download(State(state): State<SharedState>) -> Result<Response, ApiError> {
    let project = state.project.lock().expect("state mutex poisoned");
    let project = project
        .as_ref()
        .ok_or_else(|| ApiError::bad_request("no project open"))?;

    let tmp = tempfile::NamedTempFile::new().map_err(|e| ApiError::internal(e.to_string()))?;
    domain::save_project_as_impl(tmp.path(), project).map_err(ApiError::internal)?;
    let bytes = std::fs::read(tmp.path()).map_err(|e| ApiError::internal(e.to_string()))?;

    Ok((
        [
            (axum::http::header::CONTENT_TYPE, "application/octet-stream"),
            (
                axum::http::header::CONTENT_DISPOSITION,
                "attachment; filename=\"project.knxdb\"",
            ),
        ],
        bytes,
    )
        .into_response())
}
```

Wire it into `apps/knx-server/src/lib.rs`: add `mod fs_routes;` and `.merge(fs_routes::fs_routes())` next to `.merge(routes::project_routes())` in `app()`.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p knx-server`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add apps/knx-server
git commit -m "feat(knx-server): fs list/upload routes and project download, path-traversal-safe"
```

---

## Task 9: Rewire `knx-desktop` to wrap `knx-server`

Replaces `apps/knx-desktop/src-tauri`'s own copies of `AppState`/domain logic (Task 2 left them in place) with a dependency on `knx-server`. This is the task that removes the duplication and makes Tauri "a thin wrapper" per the spec.

**Files:**
- Modify: `apps/knx-desktop/src-tauri/Cargo.toml`
- Modify: `apps/knx-desktop/src-tauri/src/lib.rs` (full rewrite)
- Modify: `apps/knx-desktop/src-tauri/tauri.conf.json`
- Delete: `apps/knx-desktop/src-tauri/tests/command_dispatch.rs`, `device_detail.rs`, `open_reference_project.rs`, `save_load_roundtrip.rs` (equivalents now live in `apps/knx-server/tests/`, moved there in Task 2)

**Interfaces:**
- Consumes: `knx_server::AppState::new`, `knx_server::app`, `knx_server::DEV_PORT` (added in Step 2 of this task).
- Produces: nothing new for later tasks — this is the end of the Rust-side work until Docker (Task 13).

- [ ] **Step 1: Delete the now-duplicated tests**

```bash
git rm apps/knx-desktop/src-tauri/tests/command_dispatch.rs \
       apps/knx-desktop/src-tauri/tests/device_detail.rs \
       apps/knx-desktop/src-tauri/tests/open_reference_project.rs \
       apps/knx-desktop/src-tauri/tests/save_load_roundtrip.rs
```

- [ ] **Step 2: Add `DEV_PORT` to `knx-server`**

In `apps/knx-server/src/lib.rs`, add near the top:

```rust
/// Fixed port `cargo tauri dev` and `apps/knx-web`'s Vite dev proxy both
/// agree on — see `apps/knx-web/vite.config.ts`'s `server.proxy["/api"]`
/// and this crate's `DEV_PORT` used from `knx-desktop/src-tauri/src/lib.rs`.
/// A constant instead of an env var: both sides are source, not
/// deployment config, so there is nothing to make runtime-configurable.
pub const DEV_PORT: u16 = 4777;
```

- [ ] **Step 3: Update `Cargo.toml`**

```toml
[package]
name = "knx-desktop"
version = "0.0.0"
edition.workspace = true
license.workspace = true
repository.workspace = true
rust-version.workspace = true
publish = false

[lib]
name = "knx_desktop_lib"
crate-type = ["staticlib", "cdylib", "rlib"]

[build-dependencies]
tauri-build.workspace = true

[dependencies]
knx-server.workspace = true
axum.workspace = true
tokio.workspace = true
tauri.workspace = true
tauri-plugin-dialog.workspace = true
```

Add `knx-server = { path = "apps/knx-server" }` to the root `Cargo.toml`'s `[workspace.dependencies]`, alongside the other `knx-*` crates, so `knx-server.workspace = true` above resolves — matches how every other `knx-*` crate is wired into a consumer.

- [ ] **Step 4: Rewrite `lib.rs`**

```rust
// apps/knx-desktop/src-tauri/src/lib.rs
//! Thin native wrapper: spawns `knx-server`'s router locally and points
//! the WebView at it. All command/domain logic lives in `knx-server`; this
//! crate owns only window/process wiring plus the native file-dialog
//! plugin (kept for progressive enhancement — see
//! `apps/knx-web/src/filePicker.ts`, which uses it only when
//! `window.__TAURI__` is present).
//!
//! Dev and release differ in where the frontend comes from, not in
//! anything web/API-related:
//! - dev: Vite (`beforeDevCommand` in `tauri.conf.json`) serves the
//!   frontend with HMR; `knx-server` here only answers `/api/*`, on the
//!   fixed `knx_server::DEV_PORT` Vite's dev proxy forwards to.
//! - release: no Vite. `knx-server` serves both `/api/*` and the frontend
//!   bundled next to the binary (`beforeBuildCommand` builds it into
//!   `knx-web/dist`, `tauri.conf.json`'s `bundle.resources` ships it as a
//!   resource named `frontend`), on an OS-assigned ephemeral port.
//!
//! Either way the window is built by hand in `setup()`, not declared in
//! `tauri.conf.json` (`app.windows` is empty there) — the URL to load
//! isn't known until the branch above picks it.

use std::net::TcpListener;
use std::path::PathBuf;
use std::sync::Arc;

use tauri::{Manager, WebviewUrl, WebviewWindowBuilder};

fn spawn_server(listener: TcpListener, state: Arc<knx_server::AppState>, static_dir: Option<PathBuf>) {
    listener
        .set_nonblocking(true)
        .expect("failed to set listener non-blocking");
    tauri::async_runtime::spawn(async move {
        let listener = tokio::net::TcpListener::from_std(listener).expect("listener conversion failed");
        axum::serve(listener, knx_server::app(state, static_dir))
            .await
            .expect("knx-server exited unexpectedly");
    });
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let data_dir = app
                .path()
                .app_data_dir()
                .expect("no app data dir available")
                .join("projects");
            std::fs::create_dir_all(&data_dir)?;
            let state = Arc::new(knx_server::AppState::new(data_dir));

            let window_url = if cfg!(debug_assertions) {
                let listener = TcpListener::bind(("127.0.0.1", knx_server::DEV_PORT)).expect(
                    "dev port already in use — is another `cargo tauri dev` running?",
                );
                spawn_server(listener, state, None);
                "http://localhost:1420".to_string()
            } else {
                let listener =
                    TcpListener::bind(("127.0.0.1", 0)).expect("failed to bind an ephemeral port");
                let port = listener.local_addr()?.port();
                let static_dir = app
                    .path()
                    .resolve("frontend", tauri::path::BaseDirectory::Resource)
                    .expect("bundled frontend missing");
                spawn_server(listener, state, Some(static_dir));
                format!("http://127.0.0.1:{port}")
            };

            WebviewWindowBuilder::new(app, "main", WebviewUrl::External(window_url.parse().unwrap()))
                .title("KNXBench")
                .inner_size(1200.0, 800.0)
                .build()?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running knx-desktop");
}
```

- [ ] **Step 5: Update `tauri.conf.json`**

```json
{
  "$schema": "https://schema.tauri.app/config/2",
  "productName": "KNXBench",
  "version": "0.0.0",
  "identifier": "com.knxbench.knxbench-labs",
  "build": {
    "frontendDist": "../../knx-web/dist",
    "beforeDevCommand": "npm --prefix ../../knx-web run dev",
    "beforeBuildCommand": "npm --prefix ../../knx-web run build"
  },
  "app": {
    "windows": []
  },
  "bundle": {
    "active": false,
    "resources": {
      "../../knx-web/dist": "frontend"
    }
  }
}
```

Note for whoever implements this step: the `bundle.resources` rename-on-copy syntax (`{"source": "destination-name"}`) is the current best understanding of Tauri 2's config schema at spec-writing time, not independently verified against a running `cargo tauri build` — Task 9's Step 7 (manual verification) is where this gets confirmed for real. If a release build fails to find `resolve("frontend", BaseDirectory::Resource)`, this mapping is the first thing to check against `https://tauri.app/reference/config/` for the installed Tauri version.

- [ ] **Step 6: Run the test suite**

Run: `cargo test --workspace`
Expected: PASS. `knx-desktop`'s own `tests/` directory is now empty (deleted in Step 1) — its coverage lives in `knx-server`.

- [ ] **Step 7: Manually verify both dev and release launch**

Use the project's `run-knx-desktop` skill (`apps/knx-desktop/.claude/skills/run-knx-desktop/driver.sh start`, `shot`, `stop`) to confirm the dev build still shows the toolbar and opens the reference project. Then run `cd apps/knx-desktop/src-tauri && cargo tauri build` once and confirm it produces a binary that launches and shows the app (adjust `tauri.conf.json`'s `bundle.resources` mapping per Step 5's note if this fails).

- [ ] **Step 8: Commit**

```bash
git add apps/knx-desktop Cargo.toml
git commit -m "refactor(knx-desktop): wrap knx-server instead of owning command logic

Tauri now only spawns knx-server locally and points the WebView at it.
Dev keeps Vite's HMR loop (knx-server answers /api/* on a fixed dev
port); release serves both API and bundled frontend from knx-server on
an ephemeral port. Removes the AppState/command duplication Task 2
left in place temporarily."
```

---

## Task 10: Move the frontend to `apps/knx-web`

Purely mechanical — no behavior change, no `invoke()`/`api.ts` work yet (that's Task 11). Confirms the move alone doesn't break anything before layering the transport change on top.

**Files:**
- Move: every file under `apps/knx-desktop/src/` → `apps/knx-web/src/` (see Step 1 for the full list)
- Move: `apps/knx-desktop/index.html`, `package.json`, `package-lock.json`, `tsconfig.json`, `vite.config.ts`, `vitest.config.ts` → `apps/knx-web/`
- Delete: `apps/knx-desktop/src/`, and the moved files' old locations

- [ ] **Step 1: Move the files**

```bash
mkdir -p apps/knx-web
git mv apps/knx-desktop/src apps/knx-web/src
git mv apps/knx-desktop/index.html apps/knx-web/index.html
git mv apps/knx-desktop/package.json apps/knx-web/package.json
git mv apps/knx-desktop/package-lock.json apps/knx-web/package-lock.json
git mv apps/knx-desktop/tsconfig.json apps/knx-web/tsconfig.json
git mv apps/knx-desktop/vite.config.ts apps/knx-web/vite.config.ts
git mv apps/knx-desktop/vitest.config.ts apps/knx-web/vitest.config.ts
```

This carries every file already under `src/` along with it (`App.tsx`, `Inspector.tsx`, `ProjectExplorer.tsx`, `CommandPalette.tsx`, `Dashboard.tsx`, `Search.tsx`, `Toast.tsx`, `ThemeToggle.tsx`, `commandRegistry.ts(+.test.ts)`, `dashboardStats.ts(+.test.ts)`, `searchMatch.ts(+.test.ts)`, `selection.ts`, `styles.css`, `theme.ts(+.test.ts)`, `toast.ts(+.test.ts)`, `toastCopy.ts`, `treeUtils.ts(+.test.ts)`, `main.tsx`, `vite-env.d.ts`, and `src/bindings/*.ts`) — no import paths inside these files change, since relative imports (`./App`, `./bindings/ProjectTree`, etc.) are unaffected by moving the whole tree together.

- [ ] **Step 2: Rename the package**

In `apps/knx-web/package.json`, change `"name": "knx-desktop"` to `"name": "knx-web"`.

- [ ] **Step 3: Add the Vite dev proxy**

In `apps/knx-web/vite.config.ts`, add a proxy so `npm run dev` (standalone, no Tauri) reaches a locally-running `knx-server`:

```typescript
import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    proxy: {
      // Matches knx_server::DEV_PORT (apps/knx-server/src/lib.rs) — both
      // sides agree on this fixed dev-only port.
      "/api": "http://127.0.0.1:4777",
    },
  },
});
```

- [ ] **Step 4: Run the existing frontend tests to confirm the move alone didn't break anything**

Run: `cd apps/knx-web && npm install && npm run test`
Expected: PASS — same 6 test files, same 46 tests as before the move (this step only moved files and edited config, it did not touch component logic).

- [ ] **Step 5: Commit**

```bash
git add apps/knx-web apps/knx-desktop
git commit -m "refactor: move frontend from knx-desktop to knx-web

Mechanical move, no behavior change — Tauri IPC (invoke()) is still
what every component calls. The next task swaps that for the HTTP
client."
```

---

## Task 11: `api.ts` client and swap `invoke()` for it

**Files:**
- Create: `apps/knx-web/src/api.ts`
- Modify: `apps/knx-web/src/App.tsx`
- Modify: `apps/knx-web/src/Inspector.tsx`
- Modify: `apps/knx-web/src/ProjectExplorer.tsx`
- Create: `apps/knx-web/src/api.test.ts`

**Interfaces:**
- Produces: `api.importProject(path)`, `api.openProject(path)`, `api.saveProject()`, `api.saveProjectAs(path)`, `api.deviceDetail(deviceId)`, `api.setIndividualAddress(deviceId, address)`, `api.setComObjectDpt(comObjectId, dpt)`, `api.createGroupAddress(name, address)`, `api.deleteGroupAddress(id)`, `api.undo()`, `api.redo()` — all `Promise`-returning, matching the shapes `App.tsx`/`Inspector.tsx`/`ProjectExplorer.tsx` already expect from `invoke()`.
- Consumes: the routes from Tasks 3-7 (same-origin in prod/Docker, proxied by Vite in dev per Task 10).

- [ ] **Step 1: Write the failing test**

```typescript
// apps/knx-web/src/api.test.ts
import { describe, expect, it, vi, beforeEach } from "vitest";
import * as api from "./api";

function mockFetchOnce(body: unknown, ok = true, status = 200) {
  vi.stubGlobal(
    "fetch",
    vi.fn().mockResolvedValue({
      ok,
      status,
      statusText: "",
      headers: new Headers({ "content-length": ok ? "2" : "0" }),
      json: async () => body,
    }),
  );
}

describe("api", () => {
  beforeEach(() => {
    vi.unstubAllGlobals();
  });

  it("importProject posts the path and returns the parsed tree", async () => {
    mockFetchOnce({ installations: [] });
    const tree = await api.importProject("/x.knxproj");
    expect(tree).toEqual({ installations: [] });
    const [url, init] = (fetch as ReturnType<typeof vi.fn>).mock.calls[0];
    expect(url).toBe("/api/project/import");
    expect(init.method).toBe("POST");
    expect(JSON.parse(init.body as string)).toEqual({ path: "/x.knxproj" });
  });

  it("deviceDetail issues a GET to /api/device/:id", async () => {
    mockFetchOnce({ id: 1, name: "D1" });
    await api.deviceDetail(1);
    const [url, init] = (fetch as ReturnType<typeof vi.fn>).mock.calls[0];
    expect(url).toBe("/api/device/1");
    expect(init?.method ?? "GET").toBe("GET");
  });

  it("throws the server's error message on a non-ok response", async () => {
    mockFetchOnce({ error: "no project open" }, false, 400);
    await expect(api.undo()).rejects.toThrow("no project open");
  });

  it("setIndividualAddress sends camelCase field names", async () => {
    mockFetchOnce({ installations: [] });
    await api.setIndividualAddress(7, "1.1.1");
    const [, init] = (fetch as ReturnType<typeof vi.fn>).mock.calls[0];
    expect(JSON.parse(init.body as string)).toEqual({ deviceId: 7, address: "1.1.1" });
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd apps/knx-web && npm run test -- api.test`
Expected: FAIL — `./api` doesn't exist.

- [ ] **Step 3: Write `api.ts`**

```typescript
// apps/knx-web/src/api.ts
//! fetch()-based replacement for @tauri-apps/api/core's invoke() — same
//! function names and argument shapes the components already called, so
//! swapping the import at each call site is the only change there.
import type { ProjectTree } from "./bindings/ProjectTree";
import type { DeviceDetail } from "./bindings/DeviceDetail";

async function request<T>(path: string, init?: RequestInit): Promise<T> {
  const response = await fetch(path, {
    headers: init?.body ? { "Content-Type": "application/json" } : undefined,
    ...init,
  });
  if (!response.ok) {
    const body = await response.json().catch(() => null);
    throw new Error(body?.error ?? `${response.status} ${response.statusText}`);
  }
  if (response.headers.get("content-length") === "0") {
    return undefined as T;
  }
  return response.json() as Promise<T>;
}

export function importProject(path: string): Promise<ProjectTree> {
  return request("/api/project/import", { method: "POST", body: JSON.stringify({ path }) });
}

export function openProject(path: string): Promise<ProjectTree> {
  return request("/api/project/open", { method: "POST", body: JSON.stringify({ path }) });
}

export function saveProject(): Promise<void> {
  return request("/api/project/save", { method: "POST" });
}

export function saveProjectAs(path: string): Promise<void> {
  return request("/api/project/save-as", { method: "POST", body: JSON.stringify({ path }) });
}

export function deviceDetail(deviceId: number): Promise<DeviceDetail> {
  return request(`/api/device/${deviceId}`);
}

export function setIndividualAddress(deviceId: number, address: string | null): Promise<ProjectTree> {
  return request("/api/individual-address", {
    method: "POST",
    body: JSON.stringify({ deviceId, address }),
  });
}

export function setComObjectDpt(comObjectId: number, dpt: string | null): Promise<ProjectTree> {
  return request("/api/com-object-dpt", {
    method: "POST",
    body: JSON.stringify({ comObjectId, dpt }),
  });
}

export function createGroupAddress(name: string, address: string): Promise<ProjectTree> {
  return request("/api/group-addresses", { method: "POST", body: JSON.stringify({ name, address }) });
}

export function deleteGroupAddress(id: number): Promise<ProjectTree> {
  return request(`/api/group-addresses/${id}`, { method: "DELETE" });
}

export function undo(): Promise<ProjectTree> {
  return request("/api/undo", { method: "POST" });
}

export function redo(): Promise<ProjectTree> {
  return request("/api/redo", { method: "POST" });
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cd apps/knx-web && npm run test -- api.test`
Expected: PASS

- [ ] **Step 5: Swap `invoke()` calls in `App.tsx`**

Remove `import { invoke } from "@tauri-apps/api/core";` and add `import * as api from "./api";`. Replace each call:

- `invoke<DeviceDetail>("device_detail", { deviceId: sel.id })` (both occurrences, in `selectEntity` and `handleTreeUpdate`) → `api.deviceDetail(sel.id)`
- `invoke<ProjectTree>("open_project", { path })` (in `pickProject`) → `api.importProject(path)`
- `invoke<ProjectTree>("open_native_project", { path })` (in `openNativeProject`) → `api.openProject(path)`
- `invoke("save_project_as", { path })` (in `saveProjectAs`) → `api.saveProjectAs(path)`
- `invoke("save_project")` (in `saveProject`) → `api.saveProject()`
- `invoke<ProjectTree>("undo")` (in `undo`) → `api.undo()`
- `invoke<ProjectTree>("redo")` (in `redo`) → `api.redo()`

`open`/`save` (from `@tauri-apps/plugin-dialog`) stay untouched in this task — Task 12 replaces them with the progressive-enhancement `filePicker.ts`.

- [ ] **Step 6: Swap `invoke()` calls in `Inspector.tsx`**

Remove `import { invoke } from "@tauri-apps/api/core";` and add `import * as api from "./api";`. Replace:

- `invoke<ProjectTree>("set_individual_address", { deviceId: detail.id, address: value === "" ? null : value })` → `api.setIndividualAddress(detail.id, value === "" ? null : value)`
- `invoke<ProjectTree>("set_com_object_dpt", { comObjectId: com.id, dpt: value === "" ? null : value })` → `api.setComObjectDpt(com.id, value === "" ? null : value)`
- `invoke<ProjectTree>("delete_group_address", { id: ga.id })` → `api.deleteGroupAddress(ga.id)`

- [ ] **Step 7: Swap the `invoke()` call in `ProjectExplorer.tsx`**

Remove `import { invoke } from "@tauri-apps/api/core";` and add `import * as api from "./api";`. Replace:

- `invoke<ProjectTree>("create_group_address", { name, address })` → `api.createGroupAddress(name, address)`

- [ ] **Step 8: Run the full frontend test suite**

Run: `cd apps/knx-web && npm run test`
Expected: PASS — the 6 pre-existing suites are untouched (they test components/logic, not transport), plus `api.test.ts` from this task.

- [ ] **Step 9: Manually verify against `knx-server`**

Run `cargo run -p knx-server` in one terminal (`KNX_DATA_DIR` defaults are fine for a manual check), `npm run dev` in `apps/knx-web` in another, open `http://localhost:1420`, click "Open project…" — this still uses the untouched native-dialog code path, so it will only work if run inside the Tauri shell; for this manual check, confirm instead via `curl` that `POST http://127.0.0.1:1420/api/project/import` (proxied through Vite) returns a tree, proving the proxy and the new client are wired correctly end to end. Full UI verification of the picker happens in Task 12.

- [ ] **Step 10: Commit**

```bash
git add apps/knx-web
git commit -m "feat(knx-web): fetch()-based api.ts, swap out Tauri invoke()"
```

---

## Task 12: File-picker progressive enhancement

**Files:**
- Create: `apps/knx-web/src/filePicker.ts`
- Create: `apps/knx-web/src/FsPicker.tsx`
- Create: `apps/knx-web/src/filePicker.test.ts`
- Modify: `apps/knx-web/src/App.tsx` (swap `@tauri-apps/plugin-dialog` calls for `filePicker.ts`)
- Modify: `apps/knx-web/src/styles.css` (add `.fs-picker-*` rules)

**Interfaces:**
- Produces: `filePicker.isTauri(): boolean`, `filePicker.pickOpenPath(filters): Promise<string | null>`, `filePicker.pickSavePath(filters, defaultName): Promise<string | null>`.
- Consumes: `/api/fs/list`, `/api/fs/upload` (Task 8).

- [ ] **Step 1: Write the failing test**

```typescript
// apps/knx-web/src/filePicker.test.ts
import { describe, expect, it, vi, beforeEach } from "vitest";

describe("isTauri", () => {
  beforeEach(() => {
    vi.resetModules();
    // @ts-expect-error test-only cleanup
    delete window.__TAURI__;
  });

  it("is false when window.__TAURI__ is absent", async () => {
    const { isTauri } = await import("./filePicker");
    expect(isTauri()).toBe(false);
  });

  it("is true when window.__TAURI__ is present", async () => {
    // @ts-expect-error test-only marker, shape doesn't matter
    window.__TAURI__ = {};
    const { isTauri } = await import("./filePicker");
    expect(isTauri()).toBe(true);
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd apps/knx-web && npm run test -- filePicker.test`
Expected: FAIL — `./filePicker` doesn't exist.

- [ ] **Step 3: Write `FsPicker.tsx`**

```tsx
// apps/knx-web/src/FsPicker.tsx
//! Modal file browser over `/api/fs/list` + `/api/fs/upload` for the web
//! build (no native OS picker in a browser). Mounted imperatively by
//! filePicker.ts's openMountPicker/saveMountPicker so callers can
//! `await` it exactly like @tauri-apps/plugin-dialog's open()/save().
import { useEffect, useState } from "react";
import { createRoot } from "react-dom/client";

interface Entry {
  name: string;
  is_dir: boolean;
}

interface Filter {
  name: string;
  extensions: string[];
}

function matchesFilter(name: string, filters: Filter[]): boolean {
  if (filters.length === 0) return true;
  const ext = name.split(".").pop()?.toLowerCase();
  return filters.some((f) => f.extensions.some((e) => e.toLowerCase() === ext));
}

async function listDir(path: string): Promise<Entry[]> {
  const res = await fetch(`/api/fs/list?path=${encodeURIComponent(path)}`);
  if (!res.ok) throw new Error((await res.json().catch(() => null))?.error ?? res.statusText);
  return res.json();
}

async function uploadFile(file: File): Promise<string> {
  const form = new FormData();
  form.append("file", file);
  const res = await fetch("/api/fs/upload", { method: "POST", body: form });
  if (!res.ok) throw new Error((await res.json().catch(() => null))?.error ?? res.statusText);
  return (await res.json()).path as string;
}

function Modal(props: {
  mode: "open" | "save";
  filters: Filter[];
  defaultName?: string;
  onResolve: (path: string | null) => void;
}) {
  const { mode, filters, defaultName, onResolve } = props;
  const [dir, setDir] = useState("");
  const [entries, setEntries] = useState<Entry[]>([]);
  const [name, setName] = useState(defaultName ?? "");
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    listDir(dir)
      .then(setEntries)
      .catch((e) => setError(String(e)));
  }, [dir]);

  return (
    <div className="fs-picker-overlay">
      <div className="fs-picker">
        <h3>
          {mode === "open" ? "Open" : "Save as"} — /{dir}
        </h3>
        {error && <p className="field-error">{error}</p>}
        <ul className="fs-picker-list">
          {dir && <li onClick={() => setDir(dir.split("/").slice(0, -1).join("/"))}>..</li>}
          {entries
            .filter((e) => e.is_dir || mode === "save" || matchesFilter(e.name, filters))
            .map((e) => (
              <li
                key={e.name}
                onClick={() => {
                  if (e.is_dir) setDir(dir ? `${dir}/${e.name}` : e.name);
                  else if (mode === "open") onResolve(dir ? `${dir}/${e.name}` : e.name);
                  else setName(e.name);
                }}
              >
                {e.is_dir ? "\u{1F4C1}" : "\u{1F4C4}"} {e.name}
              </li>
            ))}
        </ul>
        {mode === "save" && (
          <input value={name} onChange={(e) => setName(e.target.value)} placeholder="filename" />
        )}
        <div className="fs-picker-actions">
          {mode === "open" && (
            <label className="fs-picker-upload">
              Upload…
              <input
                type="file"
                style={{ display: "none" }}
                onChange={async (e) => {
                  const file = e.target.files?.[0];
                  if (!file) return;
                  try {
                    onResolve(await uploadFile(file));
                  } catch (err) {
                    setError(String(err));
                  }
                }}
              />
            </label>
          )}
          {mode === "save" && (
            <button
              onClick={() => onResolve(name ? (dir ? `${dir}/${name}` : name) : null)}
              disabled={!name}
            >
              Save
            </button>
          )}
          <button onClick={() => onResolve(null)}>Cancel</button>
        </div>
      </div>
    </div>
  );
}

function openDialog(mode: "open" | "save", filters: Filter[], defaultName?: string): Promise<string | null> {
  return new Promise((resolve) => {
    const host = document.createElement("div");
    document.body.appendChild(host);
    const root = createRoot(host);
    function close(path: string | null) {
      root.unmount();
      host.remove();
      resolve(path);
    }
    root.render(<Modal mode={mode} filters={filters} defaultName={defaultName} onResolve={close} />);
  });
}

export function openMountPicker(filters: Filter[]): Promise<string | null> {
  return openDialog("open", filters);
}

export function saveMountPicker(defaultName: string): Promise<string | null> {
  return openDialog("save", [], defaultName);
}
```

- [ ] **Step 4: Write `filePicker.ts`**

```typescript
// apps/knx-web/src/filePicker.ts
//! Two ways to pick a project file: native OS dialogs
//! (`@tauri-apps/plugin-dialog`, only functional inside the Tauri shell —
//! `window.__TAURI__` marks that) and the server-mount picker
//! (`FsPicker.tsx`) for the plain web build. `App.tsx` calls these
//! instead of choosing between the two itself.
import { open as tauriOpen, save as tauriSave } from "@tauri-apps/plugin-dialog";
import { openMountPicker, saveMountPicker } from "./FsPicker";

interface Filter {
  name: string;
  extensions: string[];
}

export function isTauri(): boolean {
  return typeof window !== "undefined" && "__TAURI__" in window;
}

export async function pickOpenPath(filters: Filter[]): Promise<string | null> {
  if (isTauri()) {
    const path = await tauriOpen({ multiple: false, filters });
    return typeof path === "string" ? path : null;
  }
  return openMountPicker(filters);
}

export async function pickSavePath(filters: Filter[], defaultName: string): Promise<string | null> {
  if (isTauri()) {
    const path = await tauriSave({ filters, defaultPath: defaultName });
    return typeof path === "string" ? path : null;
  }
  return saveMountPicker(defaultName);
}
```

- [ ] **Step 5: Run test to verify it passes**

Run: `cd apps/knx-web && npm run test -- filePicker.test`
Expected: PASS

- [ ] **Step 6: Swap dialog calls in `App.tsx`**

Remove `import { open, save } from "@tauri-apps/plugin-dialog";`, add `import { pickOpenPath, pickSavePath } from "./filePicker";`. Replace each function body:

```typescript
async function pickProject() {
  const path = await pickOpenPath([{ name: "ETS project", extensions: ["knxproj"] }]);
  if (!path) return;
  clearErrors();
  try {
    resetTree(await api.importProject(path));
    setHasStorePath(false); // ETS import has no `.knxdb` location yet
  } catch (e) {
    pushError(String(e));
  }
}

async function openNativeProject() {
  const path = await pickOpenPath(KNXDB_FILTER);
  if (!path) return;
  clearErrors();
  try {
    resetTree(await api.openProject(path));
    setHasStorePath(true);
  } catch (e) {
    pushError(String(e));
  }
}

async function saveProjectAs() {
  const path = await pickSavePath(KNXDB_FILTER, "project.knxdb");
  if (!path) return;
  clearErrors();
  try {
    await api.saveProjectAs(path);
    setHasStorePath(true);
  } catch (e) {
    pushError(String(e));
  }
}
```

- [ ] **Step 7: Add minimal styling**

Append to `apps/knx-web/src/styles.css`:

```css
.fs-picker-overlay {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.4);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 100;
}

.fs-picker {
  background: var(--surface, #fff);
  color: var(--text, #111);
  padding: 1rem;
  min-width: 24rem;
  max-height: 70vh;
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}

.fs-picker-list {
  overflow-y: auto;
  list-style: none;
  margin: 0;
  padding: 0;
}

.fs-picker-list li {
  padding: 0.25rem 0.5rem;
  cursor: pointer;
}

.fs-picker-list li:hover {
  background: var(--hover, #eee);
}

.fs-picker-actions {
  display: flex;
  gap: 0.5rem;
  justify-content: flex-end;
}
```

If `--surface`/`--text`/`--hover` aren't already defined as CSS custom properties in the existing dark/light theme (`apps/knx-web/src/theme.ts` and its CSS), use whatever variable names that theme already exposes instead — check `styles.css`'s existing rules for the pattern other components follow before assuming these three names.

- [ ] **Step 8: Run the full frontend suite**

Run: `cd apps/knx-web && npm run test`
Expected: PASS

- [ ] **Step 9: Manually verify with the `run-knx-desktop` skill**

`apps/knx-desktop/.claude/skills/run-knx-desktop/driver.sh start` then `shot` — confirm native dialogs still open (Tauri build, `window.__TAURI__` present). Then, separately, `cargo run -p knx-server` + `npm run dev` in `apps/knx-web`, open `http://localhost:1420` in a plain browser, click "Open project…" and confirm the `FsPicker` modal appears instead of a native dialog.

- [ ] **Step 10: Commit**

```bash
git add apps/knx-web
git commit -m "feat(knx-web): server-mount + upload file picker for the web build

Progressive enhancement: window.__TAURI__ present -> native OS dialog
(unchanged). Absent -> FsPicker modal against /api/fs/list and
/api/fs/upload."
```

---

## Task 13: Docker packaging

**Files:**
- Create: `apps/knx-server/Dockerfile`
- Create: `.dockerignore`
- Create: `apps/knx-server/scripts/smoke-test.sh`

**Interfaces:**
- Consumes: `apps/knx-server` binary (Task 1-8), `apps/knx-web` build output (Task 10-12).
- Produces: `knxbench-server` Docker image; `KNX_PORT`, `KNX_DATA_DIR`, `KNX_STATIC_DIR` env vars (already read by `main.rs` from Task 1).

- [ ] **Step 1: Write `.dockerignore`**

```
target/
node_modules/
.git/
.worktrees/
```

- [ ] **Step 2: Write `apps/knx-server/Dockerfile`**

```dockerfile
# syntax=docker/dockerfile:1

FROM node:20-alpine AS frontend
WORKDIR /src
COPY apps/knx-web/package.json apps/knx-web/package-lock.json ./
RUN npm ci
COPY apps/knx-web/ ./
RUN npm run build

FROM rust:1.98-slim AS backend
WORKDIR /src
COPY . .
RUN cargo build --release -p knx-server

FROM debian:bookworm-slim AS runtime
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*
COPY --from=backend /src/target/release/knx-server /usr/local/bin/knx-server
COPY --from=frontend /src/dist /app/frontend
ENV KNX_STATIC_DIR=/app/frontend
ENV KNX_DATA_DIR=/data
ENV KNX_PORT=8080
VOLUME /data
EXPOSE 8080
ENTRYPOINT ["/usr/local/bin/knx-server"]
```

- [ ] **Step 3: Write the smoke-test script**

```bash
#!/usr/bin/env bash
# apps/knx-server/scripts/smoke-test.sh
# Builds the image, runs it, and confirms it answers /healthz and can
# import a project mounted into /data. Run from the repository root.
set -euo pipefail

docker build -t knxbench-server -f apps/knx-server/Dockerfile .

mkdir -p .smoke-data
cp "Unser Zuhause ets4 - 2025-12-15.knxproj" .smoke-data/

cid=$(docker run -d -p 18080:8080 -v "$(pwd)/.smoke-data:/data" knxbench-server)
trap 'docker rm -f "$cid" >/dev/null; rm -rf .smoke-data' EXIT

for _ in $(seq 1 30); do
  if curl -sf http://127.0.0.1:18080/healthz >/dev/null; then break; fi
  sleep 1
done
curl -sf http://127.0.0.1:18080/healthz
echo

response=$(curl -sf -X POST http://127.0.0.1:18080/api/project/import \
  -H 'Content-Type: application/json' \
  -d '{"path": "/data/Unser Zuhause ets4 - 2025-12-15.knxproj"}')
echo "$response" | grep -q '"errors":0' || { echo "import did not report 0 errors: $response"; exit 1; }

echo "smoke test passed"
```

```bash
chmod +x apps/knx-server/scripts/smoke-test.sh
```

- [ ] **Step 4: Run it**

Run: `./apps/knx-server/scripts/smoke-test.sh`
Expected: prints `ok`, then `smoke test passed`. Requires Docker; if unavailable in the execution environment, note that in the task's completion report instead of skipping silently — this step's evidence is part of what proves Task 13 done.

- [ ] **Step 5: Commit**

```bash
git add apps/knx-server/Dockerfile apps/knx-server/scripts .dockerignore
git commit -m "feat(knx-server): Docker packaging + smoke test script"
```

---

## Task 14: Docs and final verification

**Files:**
- Modify: `docs/IMPLEMENTATION_STATUS.md`
- Modify: `docs/ROADMAP.md`
- Modify: `docs/KNOWN_LIMITATIONS.md`
- Modify: `docs/ARCHITECTURE.md` (workspace layout section, if it lists crates)

- [ ] **Step 1: Update `docs/IMPLEMENTATION_STATUS.md`**

Add an entry documenting: `apps/knx-server` (axum HTTP API + command/domain logic, moved from `knx-desktop`), `apps/knx-web` (frontend, moved from `apps/knx-desktop/src`), `apps/knx-desktop/src-tauri` now a thin wrapper spawning `knx-server` locally. Reference the spec at `docs/superpowers/specs/2026-09-05-web-docker-deployment-design.md`.

- [ ] **Step 2: Update `docs/ROADMAP.md`**

Add this as a completed cross-cutting item (it wasn't part of the original Session 0-7 breakdown — flag that explicitly, same as the spec's header does), linking the spec and noting the KNXnet/IP-in-Docker constraint (host networking needed once Session 6 exists) stays open.

- [ ] **Step 3: Update `docs/KNOWN_LIMITATIONS.md`**

Add entries for: no auth (LAN-only by design, not yet revisited for internet exposure), `/api/project/download` buffers the whole `.knxdb` file in memory (fine for typical project sizes, would need real streaming for very large ones), `FsPicker` has no drag-and-drop or multi-select (YAGNI'd for this iteration).

- [ ] **Step 4: Check `docs/ARCHITECTURE.md` for a workspace crate list**

If it enumerates crates/apps, add `apps/knx-server` and `apps/knx-web`, and update the description of `apps/knx-desktop` to reflect its new thin-wrapper role.

- [ ] **Step 5: Run full verification**

```bash
cargo test --workspace
cargo run -p xtask -- check-layering
(cd apps/knx-web && npm run test)
./apps/knx-server/scripts/smoke-test.sh
```

Expected: all green. Report any failure here rather than one of the earlier tasks' more narrowly-scoped checks — this is the point that proves the whole plan's result actually holds together.

- [ ] **Step 6: Commit**

```bash
git add docs
git commit -m "docs: record web/Docker deployment target (knx-server) as shipped"
```
