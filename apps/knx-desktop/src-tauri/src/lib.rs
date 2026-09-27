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

fn spawn_server(
    listener: TcpListener,
    state: Arc<knx_server::AppState>,
    static_dir: Option<PathBuf>,
) {
    listener
        .set_nonblocking(true)
        .expect("failed to set listener non-blocking");
    tauri::async_runtime::spawn(async move {
        let listener =
            tokio::net::TcpListener::from_std(listener).expect("listener conversion failed");
        axum::serve(listener, knx_server::app(state, static_dir))
            .await
            .expect("knx-server exited unexpectedly");
    });
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        // The only window this app ever builds is "main" (see `setup` below).
        // Destroying it must end the process, not just hide it: a File > Quit
        // that leaves the binary running is a bug, and a user closing the
        // last window expects the app gone. `exit(0)` needs no plugin;
        // `AppHandle::exit` and `WindowEvent::Destroyed` are both already in
        // the pinned `tauri` crate. `CloseRequested` needs no clause here:
        // the frontend listens for `tauri://close-requested` (`quit.ts`
        // `onWindowCloseRequested`), and while a JS listener exists `tauri`
        // itself calls `prevent_close()` and leaves the decision — the same
        // unsaved-changes check as File > Quit — to it (§132).
        .on_window_event(|window, event| {
            if window.label() == "main" && matches!(event, tauri::WindowEvent::Destroyed) {
                window.app_handle().exit(0);
            }
        })
        .setup(|app| {
            let data_dir = app
                .path()
                .app_data_dir()
                .expect("no app data dir available")
                .join("projects");
            std::fs::create_dir_all(&data_dir)?;
            let state = Arc::new(knx_server::AppState::new(data_dir));

            let window_url = if cfg!(debug_assertions) {
                let listener = TcpListener::bind(("127.0.0.1", knx_server::DEV_PORT))
                    .expect("dev port already in use — is another `cargo tauri dev` running?");
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

            WebviewWindowBuilder::new(
                app,
                "main",
                WebviewUrl::External(window_url.parse().unwrap()),
            )
            .title("KNXBench")
            .inner_size(1200.0, 800.0)
            .build()?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running knx-desktop");
}
