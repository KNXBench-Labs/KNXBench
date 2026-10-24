//! Picks the folder native save dialogs open in, never the AppImage mount.
//!
//! An AppImage's `AppRun` changes the working directory to `$APPDIR/usr`
//! before starting the binary, and a GTK file chooser without a folder opens
//! in the working directory: the read-only image mount. The type-2 AppImage
//! runtime exports the launcher's original working directory as `OWD` (only
//! in mount mode, not with `--appimage-extract-and-run`) and the mount or
//! extraction directory as `APPDIR`; see `docs/APPIMAGE_LAUNCHER.md`.

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use tauri::Runtime;
use tauri_plugin_dialog::FileDialogBuilder;

/// Starts `dialog` in [`process_default_directory`], if any; otherwise leaves
/// the toolkit's own default.
pub(crate) fn with_default_directory<R: Runtime>(
    dialog: FileDialogBuilder<R>,
) -> FileDialogBuilder<R> {
    match process_default_directory() {
        Some(directory) => dialog.set_directory(directory),
        None => dialog,
    }
}

/// [`default_save_directory`] for this process's environment. The Rust-side
/// dialogs and the frontend's plugin dialogs (through the
/// `default_dialog_directory` command) share this one policy.
pub(crate) fn process_default_directory() -> Option<PathBuf> {
    default_save_directory(
        std::env::var_os("OWD"),
        std::env::home_dir(),
        std::env::current_dir().ok(),
        std::env::var_os("APPDIR"),
    )
}

/// `directory` as the string the dialog plugin's `defaultPath` takes. A path
/// that is not valid UTF-8 cannot travel to JavaScript intact, so it yields
/// `None` and the frontend keeps the toolkit default.
pub(crate) fn dialog_directory_string(directory: Option<PathBuf>) -> Option<String> {
    directory?.into_os_string().into_string().ok()
}

/// The first usable folder of: the AppImage's original working directory
/// (`owd`, considered only when `appdir` says this is an AppImage), the
/// current directory, the home directory. A candidate must be an existing,
/// absolute directory outside `appdir`; `None` keeps the toolkit default.
pub(crate) fn default_save_directory(
    owd: Option<OsString>,
    home: Option<PathBuf>,
    cwd: Option<PathBuf>,
    appdir: Option<OsString>,
) -> Option<PathBuf> {
    let appdir = appdir
        .filter(|dir| !dir.is_empty())
        .map(PathBuf::from)
        .map(|dir| canonical(&dir));
    // `OWD` is the AppImage runtime's export; outside an image an inherited
    // value would describe some other program's launch, not ours.
    let owd = owd
        .filter(|_| appdir.is_some())
        .filter(|dir| !dir.is_empty())
        .map(PathBuf::from);
    [owd, cwd, home]
        .into_iter()
        .flatten()
        .find(|dir| usable(dir, appdir.as_deref()))
}

fn usable(dir: &Path, appdir: Option<&Path>) -> bool {
    dir.is_absolute()
        && dir.is_dir()
        && appdir.is_none_or(|appdir| !canonical(dir).starts_with(appdir))
}

/// Resolves symbolic links so `/tmp` aliases cannot hide a path inside the
/// image; an unresolvable path is compared as given.
fn canonical(path: &Path) -> PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Fixture {
        _root: tempfile::TempDir,
        appdir: PathBuf,
        appdir_usr: PathBuf,
        launch: PathBuf,
        home: PathBuf,
        missing: PathBuf,
        file: PathBuf,
    }

    fn fixture() -> Fixture {
        let root = tempfile::tempdir().unwrap();
        let appdir = root.path().join(".mount_test");
        let appdir_usr = appdir.join("usr");
        let launch = root.path().join("projects");
        let home = root.path().join("home");
        for dir in [&appdir_usr, &launch, &home] {
            std::fs::create_dir_all(dir).unwrap();
        }
        let file = root.path().join("not-a-directory");
        std::fs::write(&file, "file").unwrap();
        Fixture {
            missing: root.path().join("gone"),
            _root: root,
            appdir,
            appdir_usr,
            launch,
            home,
            file,
        }
    }

    fn os(path: &Path) -> Option<OsString> {
        Some(path.as_os_str().to_owned())
    }

    #[test]
    fn an_existing_original_working_directory_wins_inside_an_appimage() {
        let f = fixture();
        assert_eq!(
            default_save_directory(
                os(&f.launch),
                Some(f.home.clone()),
                Some(f.appdir_usr.clone()),
                os(&f.appdir),
            ),
            Some(f.launch.clone())
        );
    }

    #[test]
    fn a_missing_or_non_directory_owd_falls_through_and_the_mount_cwd_is_rejected() {
        let f = fixture();
        for owd in [None, os(&f.missing), os(&f.file), Some(OsString::new())] {
            assert_eq!(
                default_save_directory(
                    owd,
                    Some(f.home.clone()),
                    Some(f.appdir_usr.clone()),
                    os(&f.appdir),
                ),
                Some(f.home.clone())
            );
        }
    }

    #[test]
    fn a_cwd_outside_the_image_is_used_when_owd_is_unusable() {
        let f = fixture();
        assert_eq!(
            default_save_directory(
                os(&f.missing),
                Some(f.home.clone()),
                Some(f.launch.clone()),
                os(&f.appdir),
            ),
            Some(f.launch.clone())
        );
    }

    #[test]
    fn nothing_inside_appdir_is_ever_returned() {
        let f = fixture();
        assert_eq!(
            default_save_directory(
                os(&f.appdir_usr),
                Some(f.appdir.clone()),
                Some(f.appdir_usr.clone()),
                os(&f.appdir),
            ),
            None
        );
    }

    #[cfg(unix)]
    #[test]
    fn a_symlinked_alias_of_the_mount_is_still_inside_appdir() {
        let f = fixture();
        let alias = f.launch.join("alias");
        std::os::unix::fs::symlink(&f.appdir, &alias).unwrap();
        assert_eq!(
            default_save_directory(None, Some(f.home.clone()), Some(alias), os(&f.appdir)),
            Some(f.home.clone())
        );
    }

    #[test]
    fn outside_an_appimage_owd_is_ignored_and_cwd_is_used() {
        let f = fixture();
        assert_eq!(
            default_save_directory(
                os(&f.home),
                Some(f.home.clone()),
                Some(f.launch.clone()),
                None
            ),
            Some(f.launch.clone())
        );
    }

    #[test]
    fn home_is_the_fallback_and_nothing_usable_returns_none() {
        let f = fixture();
        assert_eq!(
            default_save_directory(None, Some(f.home.clone()), None, os(&f.appdir)),
            Some(f.home.clone())
        );
        assert_eq!(
            default_save_directory(
                os(&f.missing),
                Some(f.missing.clone()),
                Some(PathBuf::from("relative")),
                os(&f.appdir),
            ),
            None
        );
        assert_eq!(default_save_directory(None, None, None, None), None);
    }

    #[test]
    fn the_dialog_string_is_the_directory_or_none() {
        assert_eq!(
            dialog_directory_string(Some(PathBuf::from("/home/alice/projects"))),
            Some("/home/alice/projects".to_string())
        );
        assert_eq!(dialog_directory_string(None), None);
    }

    #[cfg(unix)]
    #[test]
    fn a_non_utf8_directory_is_not_sent_to_the_frontend() {
        use std::os::unix::ffi::OsStringExt;
        let directory = PathBuf::from(OsString::from_vec(b"/home/alice/\xff".to_vec()));
        assert_eq!(dialog_directory_string(Some(directory)), None);
    }
}
