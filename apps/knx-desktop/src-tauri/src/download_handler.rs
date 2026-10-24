//! Tauri glue for in-app downloads: WebKit events in, native dialogs out.
//!
//! The file logic (staging, delivery, cleanup) is in `download_delivery`.
//! Messages are English, like the shell's other native prompts: the
//! language preference lives in the frontend.

use std::sync::Arc;

use tauri::webview::DownloadEvent;
use tauri::{AppHandle, Runtime};
use tauri_plugin_dialog::{DialogExt, MessageDialogKind};

use crate::download_delivery::{self, Delivered, Finish, Staging};
use crate::save_directory::with_default_directory;

/// `WebviewWindowBuilder::on_download` for every KNXBench window: stage on
/// request, ask where to save once finished. Always lets WebKit proceed; if
/// staging is impossible, WebKit's own non-overwriting Downloads path (wry
/// numbers a taken name) is used and the user is told where the file went.
pub(crate) fn handle_download<R: Runtime>(
    app: &AppHandle<R>,
    staging: &Arc<Staging>,
    event: DownloadEvent<'_>,
) -> bool {
    match event {
        DownloadEvent::Requested { destination, .. } => {
            match staging.stage(destination) {
                Ok(staged) => *destination = staged,
                Err(error) => eprintln!(
                    "knx-desktop: cannot stage a download ({error}); \
                     WebKit saves it under {}",
                    destination.display()
                ),
            }
            true
        }
        DownloadEvent::Finished { path, success, .. } => {
            match staging.on_finished(path, success) {
                Finish::Deliver { staged, file_name } => {
                    let app = app.clone();
                    tauri::async_runtime::spawn_blocking(move || {
                        let chosen = with_default_directory(app.dialog().file())
                            .set_file_name(&file_name)
                            .blocking_save_file()
                            .map(|file| file.into_path());
                        let result = match chosen {
                            None => download_delivery::deliver(&staged, None),
                            Some(Ok(path)) => download_delivery::deliver(&staged, Some(&path)),
                            Some(Err(error)) => {
                                download_delivery::discard(&staged);
                                Err(format!("Invalid save location: {error}"))
                            }
                        };
                        match result {
                            Ok(Delivered::Saved(_) | Delivered::Cancelled) => {}
                            Err(message) => {
                                tell(
                                    &app,
                                    MessageDialogKind::Error,
                                    &format!(
                                        "{message}. Nothing was saved; please download it again."
                                    ),
                                );
                            }
                        }
                    });
                }
                Finish::Failed { staged } => {
                    if let Some(staged) = staged {
                        download_delivery::discard(&staged);
                    }
                    tell(
                        app,
                        MessageDialogKind::Error,
                        "The download could not be completed. Nothing was saved.",
                    );
                }
                Finish::SavedElsewhere(path) => {
                    let place = path.map_or_else(
                        || "your Downloads folder".to_owned(),
                        |path| path.display().to_string(),
                    );
                    tell(
                        app,
                        MessageDialogKind::Info,
                        &format!("The download was saved to {place}."),
                    );
                }
            }
            true
        }
        _ => true,
    }
}

fn tell<R: Runtime>(app: &AppHandle<R>, kind: MessageDialogKind, message: &str) {
    app.dialog()
        .message(message)
        .title("KNXBench")
        .kind(kind)
        .show(|_| {});
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_requested_download_is_redirected_into_staging_and_allowed() {
        // Requests need no dialog, so the mock runtime suffices; `Finished`
        // branches are covered as plain decisions in `download_delivery`.
        let app = tauri::test::mock_app();
        let home = tempfile::tempdir().unwrap();
        let staging = Arc::new(Staging::new(home.path().join("staging")));
        let proposed = home.path().join("Downloads/import-report.json");
        let mut destination = proposed.clone();
        let allowed = handle_download(
            app.handle(),
            &staging,
            DownloadEvent::Requested {
                url: "blob:http://127.0.0.1:1/x".parse().unwrap(),
                destination: &mut destination,
            },
        );
        assert!(allowed);
        assert_ne!(destination, proposed);
        assert!(staging.owns(&destination));
        assert_eq!(destination.file_name().unwrap(), "import-report.json");
    }

    #[test]
    fn staging_failure_falls_back_to_webkits_own_path_without_refusing() {
        let app = tauri::test::mock_app();
        let home = tempfile::tempdir().unwrap();
        let blocker = home.path().join("not-a-directory");
        std::fs::write(&blocker, "file").unwrap();
        let staging = Arc::new(Staging::new(blocker.join("staging")));
        let proposed = home.path().join("Downloads/report (1).json");
        let mut destination = proposed.clone();
        assert!(handle_download(
            app.handle(),
            &staging,
            DownloadEvent::Requested {
                url: "blob:http://127.0.0.1:1/x".parse().unwrap(),
                destination: &mut destination,
            },
        ));
        assert_eq!(destination, proposed);
    }
}
