//! Builds the Tauri context, refusing early when the web bundle is missing.
//!
//! `tauri.conf.json` embeds `frontendDist` and ships the same directory as the
//! release `frontend` resource. Tauri's own refusal only names the path; this
//! one names the command that creates it. It never substitutes a placeholder,
//! so no build (debug or release) can succeed without the real bundle.

use std::path::Path;

/// The built web app, relative to this crate (as in `tauri.conf.json`).
const WEB_BUNDLE: &str = "../../knx-web/dist";

fn main() {
    let index = Path::new(WEB_BUNDLE).join("index.html");
    if !index.is_file() {
        panic!(
            "apps/knx-web/dist/index.html is missing: run `npm ci && npm run build` in \
             apps/knx-web before building, linting or testing knx-desktop \
             (docs/VERIFICATION.md, Rust workspace gate)"
        );
    }
    tauri_build::build()
}
