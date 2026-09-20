//! The HTTP router behind both `knx-web` in a browser and the Tauri desktop shell.

use std::path::PathBuf;
use std::sync::Arc;

use axum::routing::get;
use axum::{Json, Router};
use serde::Serialize;
use tower_http::services::ServeDir;

mod auth;
pub use auth::{
    bind_address, resolve_auth, short_password_notice, AuthConfig, AuthSetup, DEFAULT_IDLE_TIMEOUT,
    SESSION_COOKIE,
};

mod auth_password;
pub use auth_password::{
    constant_time_eq, hash_password, hash_password_with_iterations, DEFAULT_ITERATIONS,
};

mod auth_routes;
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
mod settings;
/// Exported so `apps/knx-desktop` and integration tests can name the file
/// and the version without re-deriving either. The file's *shape* stays
/// private: everything outside this crate talks to it over `/api/settings`.
pub use settings::{CURRENT_SCHEMA_VERSION, SETTINGS_FILE_NAME};
mod settings_routes;
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
    format_version_line(env!("CARGO_PKG_NAME"), &version_string())
}

/// The version half of [`version_line`] alone — `<version>[+g<sha>]`, no
/// crate name — which is what `GET /api/version` serves. The About dialog
/// renders it under the *application's* name, and "knx-server" is not the
/// application's name: it is the crate that happens to answer the request.
/// Built from the same two inputs as [`version_line`], so the two can
/// never disagree about which build this is.
pub fn version_string() -> String {
    format_version(
        env!("CARGO_PKG_VERSION"),
        option_env!("KNX_BUILD_SHA").filter(|sha| !sha.is_empty()),
    )
}

fn format_version(version: &str, sha: Option<&str>) -> String {
    match sha {
        Some(sha) => format!("{version}+g{sha}"),
        None => version.to_string(),
    }
}

fn format_version_line(name: &str, version: &str) -> String {
    format!("{name} {version}")
}

/// `GET /api/version`'s body. One field, because one fact is all the About
/// dialog needs and a frontend constant would rot the day the manifest
/// version moves.
#[derive(Serialize)]
struct VersionDto {
    version: String,
}

/// Builds the full router with authentication disabled: `/healthz`,
/// `/api/*`, plus — if `static_dir` is given — the built frontend served
/// from `/` as a fallback. `static_dir` is `None` for API-only test builds
/// and the Tauri dev branch, `Some(..)` for the standalone binary and the
/// Tauri release branch.
///
/// The signature is load-bearing: the Tauri shell
/// (`apps/knx-desktop/src-tauri/src/lib.rs`) and every integration test in
/// this crate call it, and ADR-0026 keeps the desktop deliberately
/// login-free — it is one user, talking to a router inside their own
/// process, over a socket nobody else can reach. [`app_with_auth`] is the
/// door the standalone binary uses.
pub fn app(state: SharedState, static_dir: Option<PathBuf>) -> Router {
    app_with_auth(state, static_dir, AuthConfig::disabled())
}

/// [`app`], with a say in whether any of it is guarded.
///
/// Three groups, and the split between them is the whole security model:
///
/// * **Guarded** — everything under `/api/` that is not an auth route.
///   One `route_layer` over the merged group rather than a check per
///   handler, so a route added tomorrow is guarded by construction.
/// * **Open, and correct to be** — `/healthz`, because a liveness probe
///   that needs a password is not a liveness probe; an orchestrator has
///   no session and would restart a perfectly healthy container forever.
/// * **Open, because they are how you stop being unauthenticated** —
///   `/api/auth/login`, `/logout` and `/status`, plus the static frontend
///   assets, which are the login screen.
///
/// With `auth` disabled, the guard is a passthrough and the auth routes
/// still answer: `/api/auth/status` reporting `required: false` is how the
/// frontend knows not to show a login screen at all.
pub fn app_with_auth(state: SharedState, static_dir: Option<PathBuf>, auth: AuthConfig) -> Router {
    let auth = std::sync::Arc::new(auth::AuthState::new(auth));

    let guarded = Router::new()
        .merge(routes::project_routes())
        .merge(fs_routes::fs_routes())
        .merge(bus_routes::bus_routes())
        .merge(debug_report_routes::debug_report_routes())
        .merge(settings_routes::settings_routes())
        // Deliberately not in `routes::project_routes()`: this answers for
        // the build, not for the open project, and it works with no
        // project loaded at all. It is guarded all the same — a build
        // identifier is a small thing to hand a stranger, but it is still
        // a thing, and the exception list is short on purpose.
        .route(
            "/api/version",
            get(|| async {
                Json(VersionDto {
                    version: version_string(),
                })
            }),
        )
        // `route_layer`, not `layer`: an unmatched path must keep falling
        // through to the static fallback with a 404, not collect a 401
        // from a guard it never reached a route behind.
        .route_layer(axum::middleware::from_fn_with_state(
            auth.clone(),
            auth::require_session,
        ))
        .with_state(state);

    let api = guarded
        .route("/healthz", get(|| async { "ok" }))
        .merge(auth_routes::auth_routes().with_state(auth));

    match static_dir {
        Some(dir) => api.fallback_service(ServeDir::new(dir)),
        None => api,
    }
}

#[cfg(test)]
mod tests {
    use super::{format_version, format_version_line};

    #[test]
    fn version_line_carries_the_commit_as_build_metadata_when_known() {
        assert_eq!(
            format_version_line(
                "knx-server",
                &format_version("0.1.0-alpha.1", Some("4cde085"))
            ),
            "knx-server 0.1.0-alpha.1+g4cde085"
        );
    }

    #[test]
    fn version_line_degrades_to_the_manifest_version_without_git() {
        assert_eq!(
            format_version_line("knx-server", &format_version("0.1.0-alpha.1", None)),
            "knx-server 0.1.0-alpha.1"
        );
    }

    // What `GET /api/version` serves, and the one thing that distinguishes
    // it from `version_line`: no crate name in it. The About dialog puts
    // the *application's* name in front of this string, and "knx-server
    // 0.1.0-alpha.1" rendered under "KNXBench" would name two things.
    #[test]
    fn version_string_carries_no_crate_name() {
        assert_eq!(
            format_version("0.1.0-alpha.1", Some("4cde085")),
            "0.1.0-alpha.1+g4cde085"
        );
        assert_eq!(format_version("0.1.0-alpha.1", None), "0.1.0-alpha.1");
    }
}
