//! The HTTP router behind both `knx-web` in a browser and the Tauri desktop shell.

use std::path::PathBuf;
use std::sync::Arc;

use axum::routing::get;
use axum::Router;
use tower_http::services::ServeDir;

mod bus;
pub use bus::*;

mod bus_routes;
mod debug_report;
mod debug_report_routes;
mod domain;
pub use domain::*;

mod errors;
mod fs_routes;
mod load_progress;
pub use load_progress::*;
mod paths;
mod routes;
mod session_log;
/// Exported so an integration test can seed a log entry directly. The two
/// types and nothing else: the module also holds `SessionLog`'s internals
/// and the import-report converters, which would drag `knx-etsproj` and
/// `knx-csv` into this crate's public surface for no test's benefit.
pub use session_log::{LogEntry, Severity};

pub type SharedState = Arc<AppState>;

/// Fixed port `cargo tauri dev` and `apps/knx-web`'s Vite dev proxy both
/// agree on — see `apps/knx-web/vite.config.ts`'s `server.proxy["/api"]`
/// and this crate's `DEV_PORT` used from `knx-desktop/src-tauri/src/lib.rs`.
/// A constant instead of an env var: both sides are source, not
/// deployment config, so there is nothing to make runtime-configurable.
pub const DEV_PORT: u16 = 4777;

/// `knx-server <version>[+g<sha>]`: the manifest version, plus SemVer
/// 2.0.0 build metadata naming the commit when the build knew it
/// (`build.rs` asks git; a tarball or Docker build may not have one and
/// can pass `KNX_BUILD_SHA` in instead). Public so the binary's
/// `--version` and anything linking this library say the same thing.
pub fn version_line() -> String {
    format_version_line(
        env!("CARGO_PKG_NAME"),
        env!("CARGO_PKG_VERSION"),
        option_env!("KNX_BUILD_SHA").filter(|sha| !sha.is_empty()),
    )
}

fn format_version_line(name: &str, version: &str, sha: Option<&str>) -> String {
    match sha {
        Some(sha) => format!("{name} {version}+g{sha}"),
        None => format!("{name} {version}"),
    }
}

/// Builds the full router: `/healthz` (and, from later tasks, `/api/*`),
/// plus — if `static_dir` is given — the built frontend served from `/`
/// as a fallback. `static_dir` is `None` for API-only test builds and the
/// Tauri dev branch, `Some(..)` for the standalone binary and the Tauri
/// release branch.
pub fn app(state: SharedState, static_dir: Option<PathBuf>) -> Router {
    let api = Router::new()
        .merge(routes::project_routes())
        .merge(fs_routes::fs_routes())
        .merge(bus_routes::bus_routes())
        .merge(debug_report_routes::debug_report_routes())
        .route("/healthz", get(|| async { "ok" }))
        .with_state(state);

    match static_dir {
        Some(dir) => api.fallback_service(ServeDir::new(dir)),
        None => api,
    }
}

#[cfg(test)]
mod tests {
    use super::format_version_line;

    #[test]
    fn version_line_carries_the_commit_as_build_metadata_when_known() {
        assert_eq!(
            format_version_line("knx-server", "0.1.0-alpha.1", Some("4cde085")),
            "knx-server 0.1.0-alpha.1+g4cde085"
        );
    }

    #[test]
    fn version_line_degrades_to_the_manifest_version_without_git() {
        assert_eq!(
            format_version_line("knx-server", "0.1.0-alpha.1", None),
            "knx-server 0.1.0-alpha.1"
        );
    }
}
