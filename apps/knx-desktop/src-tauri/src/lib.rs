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
use tauri_plugin_dialog::DialogExt;

mod bus_monitor_export;
mod native_json_export;
mod session_log_export;
use bus_monitor_export::write_bus_capture;
use session_log_export::write_session_log;

#[tauri::command]
async fn save_session_log(app: tauri::AppHandle, contents: String) -> Result<bool, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let destination = app
            .dialog()
            .file()
            .add_filter("JSON", &["json"])
            .set_file_name("session-log.json")
            .blocking_save_file();
        let Some(destination) = destination else {
            return Ok(false);
        };
        let path = destination
            .into_path()
            .map_err(|error| format!("Invalid session log destination: {error}"))?;
        write_session_log(&path, &contents)?;
        Ok(true)
    })
    .await
    .map_err(|error| format!("Session log save task failed: {error}"))?
}

#[tauri::command]
async fn save_bus_monitor_capture(app: tauri::AppHandle, contents: String) -> Result<bool, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let destination = app
            .dialog()
            .file()
            .add_filter("JSON", &["json"])
            .set_file_name("bus-monitor-capture.json")
            .blocking_save_file();
        let Some(destination) = destination else {
            return Ok(false);
        };
        let path = destination
            .into_path()
            .map_err(|error| format!("Invalid bus monitor capture destination: {error}"))?;
        write_bus_capture(&path, &contents)?;
        Ok(true)
    })
    .await
    .map_err(|error| format!("Bus monitor capture save task failed: {error}"))?
}

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
        .invoke_handler(tauri::generate_handler![
            save_session_log,
            save_bus_monitor_capture
        ])
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

#[cfg(test)]
mod session_log_export_tests {
    use super::*;

    #[test]
    fn rejects_malformed_or_oversized_exports_without_touching_existing_file() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("session-log.json");
        std::fs::write(&path, "previous").unwrap();
        assert!(write_session_log(&path, "not json").is_err());
        assert!(write_session_log(&path, &"x".repeat(16 * 1024 * 1024 + 1)).is_err());
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "previous");
    }

    #[test]
    fn writes_a_versioned_log_atomically_to_a_dialog_selected_location() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("session-log.json");
        let content =
            r#"{"format":"knxbench-session-log","version":1,"capacity":1000,"entries":[]}"#;
        write_session_log(&path, content).unwrap();
        assert_eq!(std::fs::read_to_string(path).unwrap(), content);
    }
}

#[cfg(test)]
mod bus_capture_export_tests {
    use super::*;
    use serde_json::json;

    fn valid_capture() -> serde_json::Value {
        json!({
            "format": "knxbench-bus-monitor", "version": 1, "capacity": 1000,
            "sessionId": 1, "serverIncarnation": "test-process", "status": "closed",
            "serverDroppedBefore": 2, "clientPrunedCount": 0,
            "exportedAt": "2026-09-29T01:00:00Z", "notice": "retained only",
            "rows": [{"seq": 3, "timestamp": "2026-09-29T00:00:00Z",
                "source": "1.1.5", "destination": "1/2/3", "destinationName": null,
                "service": "GroupValueWrite", "rawPayload": "0x01 (6-bit)",
                "decoded": {"kind": "value", "dpt": "DPST-1-1", "text": "On"}}]
        })
    }

    #[test]
    fn writes_the_bus_capture_and_preserves_the_exact_json() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("bus-monitor-capture.json");
        let document = valid_capture().to_string();
        write_bus_capture(&path, &document).unwrap();
        assert_eq!(std::fs::read_to_string(path).unwrap(), document);
    }

    #[test]
    fn rejects_foreign_malformed_and_oversized_captures_without_replacing_a_file() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("bus-monitor-capture.json");
        std::fs::write(&path, "previous").unwrap();
        assert!(write_bus_capture(&path, "not json").is_err());
        assert!(write_bus_capture(&path, &"x".repeat(16 * 1024 * 1024 + 1)).is_err());
        for (field, wrong) in [
            ("format", json!("knxbench-session-log")),
            ("version", json!(2)),
            ("capacity", json!(5000)),
            ("serverDroppedBefore", json!(-1)),
            ("status", json!("unknown")),
        ] {
            let mut invalid = valid_capture();
            invalid[field] = wrong;
            assert!(
                write_bus_capture(&path, &invalid.to_string()).is_err(),
                "{field}"
            );
        }
        let mut invalid = valid_capture();
        invalid["rows"] = json!([{}]);
        assert!(write_bus_capture(&path, &invalid.to_string()).is_err());
        let mut invalid = valid_capture();
        invalid["rows"] = json!(vec![valid_capture()["rows"][0].clone(); 1001]);
        assert!(write_bus_capture(&path, &invalid.to_string()).is_err());
        assert_eq!(std::fs::read_to_string(path).unwrap(), "previous");
    }
}
