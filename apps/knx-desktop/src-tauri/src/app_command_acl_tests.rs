//! Every app command passes Tauri's ACL from the origins the shell loads (#7).
//!
//! A release window shows the UI from the embedded server at
//! `http://127.0.0.1:<ephemeral port>`. That is neither `devUrl` nor
//! `frontendDist`, so Tauri treats it as a *remote* origin, and a remote
//! origin reaches an app command only through an explicit capability. Without
//! one, "Save log" and "Save capture" failed in every AppImage with
//! `Command … not allowed by ACL`.
//!
//! The requests carry no arguments on purpose. Tauri checks the ACL first,
//! then deserializes arguments, so an allowed command answers with a
//! missing-argument error, a refused one with the ACL error, and neither
//! opens a native dialog. A refusal reads `Command … not allowed by ACL` in a
//! release build and a longer `… not allowed …` diagnosis in a debug build
//! (same branch, `tauri::webview` `on_message`); the tests match both.

use tauri::ipc::{CallbackFn, InvokeBody};
use tauri::test::{get_ipc_response, mock_builder, INVOKE_KEY};
use tauri::webview::InvokeRequest;
use tauri::{App, WebviewUrl, WebviewWindow, WebviewWindowBuilder};

/// Every command `app_command_handler!` registers.
const APP_COMMANDS: [&str; 2] = ["save_session_log", "save_bus_monitor_capture"];

/// The origin a release build loads (any port: it is ephemeral).
const RELEASE_ORIGIN: &str = "http://127.0.0.1:41234/";
/// The Vite dev server a debug build loads (`run`'s `cfg!(debug_assertions)`
/// branch). `tauri.conf.json` sets no `devUrl`, so it is remote to Tauri too.
const DEV_ORIGIN: &str = "http://localhost:1420/";

fn app() -> App<tauri::test::MockRuntime> {
    mock_builder()
        .invoke_handler(app_command_handler!())
        .build(tauri::generate_context!(test = true))
        .expect("mock app with the real capabilities")
}

fn window(
    app: &App<tauri::test::MockRuntime>,
    label: &str,
) -> WebviewWindow<tauri::test::MockRuntime> {
    WebviewWindowBuilder::new(
        app,
        label,
        WebviewUrl::External(RELEASE_ORIGIN.parse().unwrap()),
    )
    .build()
    .expect("mock window")
}

/// The error text of `command`, invoked without arguments from `origin`.
/// Tauri decides on the request's URL, so one window serves every origin.
fn refusal(
    window: &WebviewWindow<tauri::test::MockRuntime>,
    origin: &str,
    command: &str,
) -> String {
    let response = get_ipc_response(
        window,
        InvokeRequest {
            cmd: command.into(),
            callback: CallbackFn(0),
            error: CallbackFn(1),
            url: origin.parse().unwrap(),
            body: InvokeBody::default(),
            headers: Default::default(),
            invoke_key: INVOKE_KEY.to_string(),
        },
    );
    match response {
        Ok(_) => panic!("{command} without arguments must not succeed"),
        Err(error) => error.to_string(),
    }
}

#[test]
fn main_window_reaches_every_app_command_from_the_release_and_dev_origins() {
    let app = app();
    let main = window(&app, "main");
    for origin in [RELEASE_ORIGIN, DEV_ORIGIN] {
        for command in APP_COMMANDS {
            let error = refusal(&main, origin, command);
            assert!(
                !error.contains("not allowed"),
                "{command} from {origin} must pass the ACL, got: {error}"
            );
            assert!(
                error.contains("contents"),
                "{command} from {origin} must reach the command and miss its argument, got: {error}"
            );
        }
    }
}

#[test]
fn read_only_companion_windows_stay_refused() {
    // The grant is for the main window only: the diagnostics companion and
    // Flow windows are read-only and must not open a file-save dialog.
    let app = app();
    for label in ["diagnostics", "flow-1"] {
        let companion = window(&app, label);
        for command in APP_COMMANDS {
            let error = refusal(&companion, RELEASE_ORIGIN, command);
            assert!(
                error.contains("not allowed"),
                "{command} from window {label} must be refused, got: {error}"
            );
        }
    }
}
