use std::path::PathBuf;
use std::sync::Arc;

use axum::routing::get;
use axum::Router;
use tower_http::services::ServeDir;

mod domain;
pub use domain::*;

mod errors;
mod fs_routes;
mod paths;
mod routes;
mod session_log;

pub type SharedState = Arc<AppState>;

/// Fixed port `cargo tauri dev` and `apps/knx-web`'s Vite dev proxy both
/// agree on — see `apps/knx-web/vite.config.ts`'s `server.proxy["/api"]`
/// and this crate's `DEV_PORT` used from `knx-desktop/src-tauri/src/lib.rs`.
/// A constant instead of an env var: both sides are source, not
/// deployment config, so there is nothing to make runtime-configurable.
pub const DEV_PORT: u16 = 4777;

/// Builds the full router: `/healthz` (and, from later tasks, `/api/*`),
/// plus — if `static_dir` is given — the built frontend served from `/`
/// as a fallback. `static_dir` is `None` for API-only test builds and the
/// Tauri dev branch, `Some(..)` for the standalone binary and the Tauri
/// release branch.
pub fn app(state: SharedState, static_dir: Option<PathBuf>) -> Router {
    let api = Router::new()
        .merge(routes::project_routes())
        .merge(fs_routes::fs_routes())
        .route("/healthz", get(|| async { "ok" }))
        .with_state(state);

    match static_dir {
        Some(dir) => api.fallback_service(ServeDir::new(dir)),
        None => api,
    }
}
