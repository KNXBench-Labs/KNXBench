# Design: Web/Docker deployment target (`knx-server`)

Date: 2026-09-05
Session: cross-cutting addition alongside Session 5 (UI/UX); not on the
original Session 0-7 roadmap — added by explicit request, tracked here and
folded into ROADMAP.md once implemented.
Status: Approved

## Context

KNXBench today ships as one deployment target: `apps/knx-desktop`, a Tauri 2
shell (Rust + WebKit2GTK) around a React frontend, talking to the backend
over Tauri's IPC (`invoke()`). That backend logic already lives almost
entirely in Tauri-independent crates (`knx-app`, `knx-core`, `knx-store`,
`knx-etsproj`) — `apps/knx-desktop/src-tauri/src/lib.rs` is a thin layer of
`#[tauri::command]` wrappers around plain `_impl` functions.

The user wants an additional deployment path: a Docker container exposing a
web UI, reachable from any device on the LAN, without requiring the native
Tauri/WebKit2GTK stack on the host. Reasoning explored during brainstorming:

- Docker is more flexible about *where* it runs than a native GTK app, but
  it is not free — KNXnet/IP multicast discovery (a Session 6 concern, not
  started yet, see IMPLEMENTATION_STATUS.md) needs `--network host` inside
  a container. This design does not need to solve that yet: no bus
  communication feature exists in the app today. It is called out here as a
  constraint Session 6 will have to account for, not solved now.
- Maintaining two independent frontends (Tauri-IPC + a separate web one)
  was considered and rejected as ongoing double maintenance for no benefit.

## Goal (this design)

One HTTP API and one frontend, usable both by the existing Tauri desktop
shell (as a thin local wrapper) and by a standalone Docker container. No
change to `knx-core`/`knx-app`/`knx-store`/`knx-etsproj` — this is purely a
new transport layer replacing Tauri IPC as the frontend's only integration
point.

## Decisions made during brainstorming

- **Converge, don't duplicate.** Tauri stops being an IPC layer and becomes
  a thin wrapper that spawns the new HTTP server locally (localhost, fixed
  port) and points its WebView at it. One frontend, one API surface, two
  ways to run it.
- **Single-user.** State stays a single `Mutex`-guarded project in memory,
  same shape as today's `AppState` — no sessions, no multi-tenant model.
  Sufficient for the stated use case (self-hosted, one project at a time).
- **LAN-only, no auth.** The container is not meant to be exposed to the
  internet. No login/auth layer in this design. If that changes later, it's
  a separate design (auth is not free to bolt on afterward for a
  stateful single-project server, but that's future work, not now).
- **Both file-handling strategies, not one.**
  - *Server-mount* (primary): container mounts a host directory; the web UI
    gets a directory-listing endpoint and a path picker, close to desktop
    UX. `open_project_impl`/`save_project_as_impl` already take `&Path`, so
    this is close to free on the backend — the new work is the listing
    endpoint and picker UI, plus constraining paths to the mount root
    (no path traversal out of the volume).
  - *Upload/download* (fallback): for files not on the mount, or ad-hoc
    use without volume setup.
- **Native dialogs stay native.** The Tauri build keeps
  `@tauri-apps/plugin-dialog` (already built, already tested) via a
  `window.__TAURI__` feature check in the frontend. Only the web build (no
  `__TAURI__`) falls back to the mount-picker/upload-download UI. No UX
  regression on desktop for this.

## Architecture

```
crates/
  knx-core, knx-app, knx-store, knx-etsproj,
  knx-productdb, knx-projection, knx-net, knx-secure     unchanged

apps/
  knx-server/            NEW — axum HTTP API + static frontend serving
    src/
      routes.rs            one route per existing Tauri command, thin
                            JSON-in/JSON-out wrapper around the same
                            _impl functions used today
      state.rs              AppState (Mutex<Option<Project>>, CommandStack,
                            import counts) relocated here from src-tauri
      fs.rs                  mount-root-constrained directory listing +
                            upload/download handlers
      static.rs               tower-http::ServeDir for the built frontend
    Cargo.toml              axum, tower-http, tokio, knx-app, knx-core,
                            knx-store, knx-etsproj

  knx-web/                NEW — frontend, moved from apps/knx-desktop/src
    src/api.ts               fetch()-based client, replaces invoke()
    src/fs-picker.tsx         mount-picker + upload/download UI
                            (only rendered when window.__TAURI__ is absent)

  knx-desktop/
    src-tauri/
      src/lib.rs             shrinks to: spawn knx-server locally
                            (localhost, fixed port), open WebView on it.
                            No more #[tauri::command] handlers.
```

`knx-core`/`knx-app`/`knx-store`/`knx-etsproj` keep zero dependency on both
Tauri and any HTTP framework — `knx-server` is the only new crate that
speaks HTTP, same architectural role Tauri's command layer had before.

Framework: **axum** (tokio-based; Tauri already runs a tokio runtime
internally, so no new runtime added for the desktop build).
`tower-http::ServeDir` serves the built frontend's static files.

## API / data flow

Every existing Tauri command becomes a route, same `_impl` functions behind
it:

| Route | Method | Replaces Tauri command |
|---|---|---|
| `/api/project/import` `{path}` | POST | `open_project` (ETS `.knxproj` import) |
| `/api/project/open` `{path}` | POST | `open_native_project` (`.knxdb`) |
| `/api/project/save` | POST | `save_project` |
| `/api/project/save-as` `{path}` | POST | `save_project_as` |
| `/api/device/:id` | GET | `device_detail` |
| `/api/individual-address` `{deviceId, address}` | POST | `set_individual_address` |
| `/api/com-object-dpt` `{...}` | POST | `set_com_object_dpt` |
| `/api/group-addresses` `{name, address}` | POST | `create_group_address` |
| `/api/group-addresses/:id` | DELETE | `delete_group_address` |
| `/api/undo` | POST | `undo` |
| `/api/redo` | POST | `redo` |

New, web-only (file handling):

| Route | Purpose |
|---|---|
| `GET /api/fs/list?path=` | Directory listing, constrained to the mount root |
| `POST /api/fs/upload` | Multipart upload into a staging directory; returns a path usable with `/api/project/import` or `/open` |
| `GET /api/project/download` | Streams the current project as `.knxdb` (browser save fallback) |
| `GET /healthz` | Health/readiness check for container orchestration |

State: same shape as today's `AppState` (`Mutex<Option<Project>>`,
`Mutex<CommandStack>`, `Mutex<(errors, warnings)>`), moved from
`tauri::State` to axum's `State<Arc<AppState>>`. No change to what it holds.

## Error handling

`_impl` functions keep returning `Result<T, AppError>` / `Result<T, String>`
exactly as today. The route layer is the only new translation: maps errors
to an HTTP status (400 for user-facing errors — stale device id, invalid
path — 500 for internal failures) plus a `{"error": "..."}` JSON body. No
new error taxonomy.

## Docker packaging

Multi-stage build, no GTK/WebKit2GTK in the final image (that dependency
was only ever for the native window, not for `knx-server` itself — the
image is smaller and simpler than the native build's toolchain):

```
Stage 1  node:20-alpine    npm ci && npm run build (apps/knx-web) -> dist/
Stage 2  rust:1.98-slim    cargo build --release -p knx-server
Stage 3  debian-slim        server binary + dist/, nothing else
```

Runtime config via env vars: `KNX_DATA_DIR=/data` (mount root for the
server picker and `.knxdb` storage), `KNX_PORT=8080`. `/data` is a Docker
volume.

## Testing

- `knx-server`: integration tests per route via `tower::ServiceExt::oneshot`
  (no real socket needed) — reusing the same fixtures as today's
  `cargo test -p knx-desktop` (`command_dispatch`, `device_detail`,
  `open_reference_project`, `save_load_roundtrip`), now exercised over HTTP
  instead of direct function calls.
- `knx-web`: existing Vitest suites (toast, search, command registry, tree
  utils) are untouched — they test components, not transport. New:
  `api.ts` client tests against a mocked `fetch`.
- `knx-desktop/src-tauri`: shrinks to "spawns the server, WebView loads it"
  — its own tests reduce to spawn/proxy behavior; command-level coverage
  moves to `knx-server`.
- Docker smoke test: build the image, run the container, `curl /healthz`
  plus one import round-trip — scripted, in the spirit of
  `run-knx-desktop`'s `driver.sh`.

## Out of scope (this design)

- Auth / multi-user / sessions.
- KNXnet/IP inside the container (Session 6, not started; host networking
  concern is noted above, not solved here).
- Removing Tauri/the native desktop build — it stays as the primary
  deployment target per CLAUDE.md's Linux-first priority; this design adds
  a second target, it does not replace the first.

## Open risks

- Path-traversal constraint on `/api/fs/list` and the mount picker needs
  careful testing (malformed/relative/symlink paths) — data integrity and
  correctness both depend on this being airtight, not just convenient.
- Moving `apps/knx-desktop/src` to `apps/knx-web` is a file-move touching
  every existing frontend test/import path — mechanical but wide; plan for
  it as its own step so it doesn't get tangled with the new API work.
