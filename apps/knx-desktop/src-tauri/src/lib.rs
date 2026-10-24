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
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use tauri::{Manager, WebviewUrl, WebviewWindowBuilder};
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons, MessageDialogKind};

mod bus_monitor_export;
mod download_delivery;
mod download_handler;
mod native_json_export;
mod save_directory;
mod session_log_export;
mod web_process;
use bus_monitor_export::write_bus_capture;
use save_directory::{dialog_directory_string, process_default_directory, with_default_directory};
use session_log_export::write_session_log;
use web_process::{CloseDecision, SharedSupervisor, WebProcessSupervisor};

/// Shown only when the page can no longer ask (§133). English, because the
/// language preference lives in the frontend that has just failed.
const DISCARD_PROMPT: &str = "The KNXBench interface stopped working repeatedly and could not be \
restarted. The open project has unsaved changes, and closing now discards them.";

fn close_after_dead_frontend<R: tauri::Runtime>(window: &tauri::Window<R>) {
    let app = window.app_handle();
    let Some(supervisor) = app.try_state::<SharedSupervisor>() else {
        return;
    };
    let unsaved = app
        .try_state::<Arc<knx_server::AppState>>()
        // Without the server state nothing can prove the project is clean.
        .is_none_or(|state| state.has_unsaved_changes());
    let decision = supervisor
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .on_close_requested(unsaved);
    match decision {
        CloseDecision::LeaveToFrontend | CloseDecision::AlreadyAsking => {}
        CloseDecision::Close => destroy(window),
        CloseDecision::ConfirmDiscard => {
            let window = window.clone();
            let supervisor = Arc::clone(&supervisor);
            app.dialog()
                .message(DISCARD_PROMPT)
                .title("KNXBench")
                .kind(MessageDialogKind::Warning)
                .buttons(MessageDialogButtons::OkCancelCustom(
                    "Close and discard changes".into(),
                    "Keep window open".into(),
                ))
                .show(move |discard| {
                    supervisor
                        .lock()
                        .unwrap_or_else(|poisoned| poisoned.into_inner())
                        .discard_prompt_answered();
                    if discard {
                        destroy(&window);
                    }
                });
        }
    }
}

fn destroy<R: tauri::Runtime>(window: &tauri::Window<R>) {
    if let Err(error) = window.destroy() {
        eprintln!("knx-desktop: could not close the window: {error}");
    }
}

/// The folder the frontend's plugin dialogs (`apps/knx-web/src/filePicker.ts`:
/// project Save As/Open, CSV, debug report, …) start in: the same
/// [`process_default_directory`] the Rust-side dialogs use, so none opens in
/// the read-only AppImage mount. `None` keeps the toolkit default. Async so
/// the filesystem checks stay off the main thread.
#[tauri::command]
async fn default_dialog_directory() -> Option<String> {
    dialog_directory_string(process_default_directory())
}

#[tauri::command]
async fn save_session_log<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    contents: String,
) -> Result<bool, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let destination = with_default_directory(app.dialog().file())
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
async fn save_bus_monitor_capture<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    contents: String,
) -> Result<bool, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let destination = with_default_directory(app.dialog().file())
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
) -> knx_server::EmbeddedServerHandle {
    listener
        .set_nonblocking(true)
        .expect("failed to set listener non-blocking");
    tauri::async_runtime::block_on(async move {
        let listener =
            tokio::net::TcpListener::from_std(listener).expect("listener conversion failed");
        knx_server::EmbeddedServerHandle::spawn(listener, state, static_dir)
    })
}

type SharedShutdown = Arc<DesktopShutdown>;

/// Native notifications coalesce; neither their count nor their exit code is
/// authority to force a device/catalogue wait. No desktop force UI is exposed.
struct DesktopShutdown {
    server: knx_server::EmbeddedServerHandle,
    closing: AtomicBool,
    exit_authorized: AtomicBool,
    task: Mutex<Option<tauri::async_runtime::JoinHandle<()>>>,
}

impl DesktopShutdown {
    fn new(server: knx_server::EmbeddedServerHandle) -> Self {
        Self {
            server,
            closing: AtomicBool::new(false),
            exit_authorized: AtomicBool::new(false),
            task: Mutex::new(None),
        }
    }

    fn begin_once(&self) -> bool {
        self.closing
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_ok()
    }

    fn exit_is_authorized(&self) -> bool {
        self.exit_authorized.load(Ordering::SeqCst)
    }
}

fn begin_close<R: tauri::Runtime>(app: &tauri::AppHandle<R>) {
    let Some(shutdown) = app.try_state::<SharedShutdown>() else {
        return;
    };
    if !shutdown.begin_once() {
        return;
    }
    shutdown.server.request_close();
    let lifetime = Arc::clone(&shutdown);
    let app = app.clone();
    let task = tauri::async_runtime::spawn(async move {
        // Ordinary close has no second-stop future. In particular Destroyed,
        // ExitRequested(None) and a queued Some(0) cannot shorten this drain.
        let result = lifetime.server.drain_close(std::future::pending()).await;
        eprintln!("knx-desktop: shutdown {result:?}");
        if result.catalogue.work_outstanding {
            eprintln!("knx-desktop: catalogue work remains; process exit can leave its outcome unavailable/possibly committed; retained backup may need inspection");
        }
        if matches!(
            result.writes.end,
            knx_server::WriteWaitEnd::TimedOut | knx_server::WriteWaitEnd::SecondSignal
        ) {
            eprintln!("knx-desktop: device work still running ends with the process; the device may be partly written, check it before relying on it");
        }
        // Publish before the sole final exit request can reenter the callback.
        lifetime.exit_authorized.store(true, Ordering::SeqCst);
        if result.requires_process_exit() {
            // Returning through runtime Drop could wait forever on blocking
            // work. Explicit exit is an escape, never cleanup evidence.
            std::process::exit(0);
        }
        app.exit(0);
    });
    *shutdown.task.lock().unwrap_or_else(|p| p.into_inner()) = Some(task);
}

/// The app's own IPC commands, shared by `run` and the ACL regression test.
/// A release window loads the UI from `http://127.0.0.1:<port>`, a remote
/// origin to Tauri, so each command also needs its `allow-…` permission in
/// `permissions/app-commands.toml`, granted to the main window in
/// `capabilities/default.json` (issue #7).
macro_rules! app_command_handler {
    () => {
        tauri::generate_handler![
            crate::save_session_log,
            crate::save_bus_monitor_capture,
            crate::default_dialog_directory
        ]
    };
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(app_command_handler!())
        // Keep the frontend/dead-frontend unsaved checks. Accepted window
        // destruction begins owned drain, never an early AppHandle::exit.
        .on_window_event(|window, event| {
            if window.label() != "main" {
                return;
            }
            match event {
                tauri::WindowEvent::Destroyed => begin_close(window.app_handle()),
                tauri::WindowEvent::CloseRequested { .. } => close_after_dead_frontend(window),
                _ => {}
            }
        })
        .setup(|app| {
            let data_dir = app
                .path()
                .app_data_dir()
                .expect("no app data dir available")
                .join("projects");
            std::fs::create_dir_all(&data_dir)?;
            let state = Arc::new(knx_server::AppState::with_user_product_db(data_dir));
            app.manage(Arc::clone(&state));

            let (window_url, server) = if cfg!(debug_assertions) {
                let listener = TcpListener::bind(("127.0.0.1", knx_server::DEV_PORT))
                    .expect("dev port already in use — is another `cargo tauri dev` running?");
                let server = spawn_server(listener, state, None);
                ("http://localhost:1420".to_string(), server)
            } else {
                let listener =
                    TcpListener::bind(("127.0.0.1", 0)).expect("failed to bind an ephemeral port");
                let port = listener.local_addr()?.port();
                let static_dir = app
                    .path()
                    .resolve("frontend", tauri::path::BaseDirectory::Resource)
                    .expect("bundled frontend missing");
                let server = spawn_server(listener, state, Some(static_dir));
                (format!("http://127.0.0.1:{port}"), server)
            };
            app.manage(Arc::new(DesktopShutdown::new(server)));

            // In-app downloads go through a native save dialog instead of
            // landing silently in ~/Downloads (KL-182). All windows share one
            // WebKit context, so this handler also serves Flow/diagnostics.
            let staging = Arc::new(download_delivery::Staging::new(
                app.path().app_cache_dir()?.join("download-staging"),
            ));
            staging.remove_abandoned(download_delivery::ABANDONED_AFTER);
            let window = WebviewWindowBuilder::new(
                app,
                "main",
                WebviewUrl::External(window_url.parse().unwrap()),
            )
            .title("KNXBench")
            .inner_size(1200.0, 800.0)
            .on_download(move |webview, event| {
                download_handler::handle_download(webview.app_handle(), &staging, event)
            })
            .build()?;

            let supervisor: SharedSupervisor = Arc::new(Mutex::new(WebProcessSupervisor::new()));
            app.manage(supervisor.clone());
            #[cfg(target_os = "linux")]
            web_process::watch(&window, supervisor)?;
            #[cfg(not(target_os = "linux"))]
            let _ = (window, supervisor);
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building knx-desktop")
        .run(|app, event| {
            if let tauri::RunEvent::ExitRequested { api, .. } = event {
                let authorized = app
                    .try_state::<SharedShutdown>()
                    .is_some_and(|shutdown| shutdown.exit_is_authorized());
                if !authorized {
                    api.prevent_exit();
                    begin_close(app);
                }
            }
        });
}

#[cfg(test)]
mod shutdown_policy_tests {
    use super::*;

    #[test]
    fn close_request_ipc_is_admitted_only_for_the_existing_main_origins() {
        use tauri::ipc::{CallbackFn, InvokeBody};
        use tauri::test::{get_ipc_response, mock_builder, INVOKE_KEY};
        use tauri::webview::InvokeRequest;

        let app = mock_builder()
            .build(tauri::generate_context!(test = true))
            .unwrap();
        let main = WebviewWindowBuilder::new(
            &app,
            "main",
            WebviewUrl::External("http://127.0.0.1:41234/".parse().unwrap()),
        )
        .build()
        .unwrap();
        for (origin, admitted) in [
            ("http://127.0.0.1:41234/", true),
            ("http://localhost:1420/", true),
            ("https://example.invalid/", false),
        ] {
            let response = get_ipc_response(
                &main,
                InvokeRequest {
                    cmd: "plugin:window|close".into(),
                    callback: CallbackFn(0),
                    error: CallbackFn(1),
                    url: origin.parse().unwrap(),
                    // Invalid argument proves command admission without actually
                    // closing even the mock window or running a native callback.
                    body: InvokeBody::Json(serde_json::json!({"label": 123})),
                    headers: Default::default(),
                    invoke_key: INVOKE_KEY.into(),
                },
            );
            let error = match response {
                Err(error) => error.to_string(),
                Ok(_) => panic!("malformed close request must not execute"),
            };
            assert_eq!(
                !error.contains("not allowed"),
                admitted,
                "{origin}: {error}"
            );
            if admitted {
                assert!(error.contains("label"), "{error}");
            }
        }
    }

    /// Headless notification-policy control, not a native event-loop smoke.
    /// The production callbacks use these same once/authorization methods.
    #[tokio::test]
    async fn ordinary_native_notifications_coalesce_without_force_authority() {
        for notifications in [
            ["destroyed", "implicit", "queued-code-zero"],
            ["implicit", "queued-code-zero", "destroyed"],
            ["queued-code-zero", "destroyed", "implicit"],
        ] {
            let directory = tempfile::tempdir().unwrap();
            let state = Arc::new(knx_server::AppState::new(directory.path().to_path_buf()));
            let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
                .await
                .unwrap();
            let shutdown = DesktopShutdown::new(knx_server::EmbeddedServerHandle::spawn(
                listener, state, None,
            ));
            let mut begins = 0;
            for event in notifications.into_iter().cycle().take(9) {
                assert!(
                    !shutdown.exit_is_authorized(),
                    "{event} must still be prevented"
                );
                if shutdown.begin_once() {
                    begins += 1;
                }
            }
            assert_eq!(begins, 1);
            let result = shutdown.server.drain_close(std::future::pending()).await;
            assert!(!result.requires_process_exit());
            shutdown.exit_authorized.store(true, Ordering::SeqCst);
            assert!(shutdown.exit_is_authorized(), "final reentry may proceed");
            assert!(
                !shutdown.begin_once(),
                "authorized reentry cannot start another drain"
            );
        }
    }
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
        assert!(write_session_log(&path, &"x".repeat(64 * 1024 * 1024 + 1)).is_err());
        // The previous cap's documents are refused, not silently relabelled.
        let old_cap =
            r#"{"format":"knxbench-session-log","version":1,"capacity":1000,"entries":[]}"#;
        assert!(write_session_log(&path, old_cap).is_err());
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "previous");
    }

    #[test]
    fn writes_a_versioned_log_atomically_to_a_dialog_selected_location() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("session-log.json");
        let content =
            r#"{"format":"knxbench-session-log","version":1,"capacity":16000,"entries":[]}"#;
        write_session_log(&path, content).unwrap();
        assert_eq!(std::fs::read_to_string(path).unwrap(), content);
    }

    #[test]
    fn accepts_a_full_16000_entry_log_above_the_old_16_mib_limit() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("session-log.json");
        let entry = serde_json::json!({"timestamp": "2026-10-13T18:00:00Z", "severity": "warning",
            "source": "import:unsupported", "message": "m", "location": null, "detail": "d".repeat(1100)});
        let document = serde_json::json!({"format": "knxbench-session-log", "version": 1,
            "capacity": 16000, "entries": vec![entry; 16000]});
        let content = serde_json::to_string_pretty(&document).unwrap();
        assert!(content.len() > 16 * 1024 * 1024);
        write_session_log(&path, &content).unwrap();
        let mut over = document;
        over["entries"]
            .as_array_mut()
            .unwrap()
            .push(serde_json::json!({}));
        assert!(
            write_session_log(&path, &over.to_string()).is_err(),
            "16001 entries"
        );
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

#[cfg(test)]
mod app_command_acl_tests;

#[cfg(test)]
mod functions_surface_tests;
