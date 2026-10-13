//! In-app downloads leave the WebView through a native save dialog, never silently.
//!
//! Without a download handler WebKitGTK writes every `<a download>` (blob or
//! URL) straight into the XDG Downloads directory, without a dialog, and
//! replaces a same-named file there (measured on WebKitGTK 2.52.6; see
//! `docs/limitations/KL-182-desktop-silent-downloads.md`). With the handler
//! in `lib.rs` a download first lands in a private staging directory. Once it
//! has finished, the native save dialog decides where it goes, the copy is
//! written atomically, and the staged copy is removed; cancelling discards it.
//!
//! Every window shares one WebKit context (Tauri keys it by data directory,
//! and no window sets one), so the main window's handler also receives
//! downloads started in the Flow and diagnostics windows.
//!
//! Everything here is plain file logic; the dialogs live in `download_handler`.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime};

use crate::native_json_export::write_atomically_from;

/// Staged downloads older than this are left over from a run that ended
/// before its dialog was answered; a live download is minutes old at most.
pub(crate) const ABANDONED_AFTER: Duration = Duration::from_secs(24 * 60 * 60);

/// Fallback when WebKit proposes no usable file name.
const UNNAMED: &str = "download";

pub(crate) struct Staging {
    root: PathBuf,
    next: AtomicU64,
}

/// What to do once WebKit reports a download as finished.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Finish {
    /// Ask where to save `staged`, suggesting `file_name`.
    Deliver { staged: PathBuf, file_name: String },
    /// The download failed; `staged` (if ours) is incomplete and must go.
    Failed { staged: Option<PathBuf> },
    /// Not staged by us (staging was impossible), so WebKit's own
    /// non-overwriting Downloads path was used; tell the user where.
    SavedElsewhere(Option<PathBuf>),
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Delivered {
    Saved(PathBuf),
    Cancelled,
}

impl Staging {
    pub(crate) fn new(root: PathBuf) -> Self {
        Self {
            root,
            next: AtomicU64::new(0),
        }
    }

    /// The private path WebKit should write a download to, given the
    /// Downloads-directory path wry proposed for it. One fresh directory per
    /// download, so equal names never meet.
    pub(crate) fn stage(&self, proposed: &Path) -> std::io::Result<PathBuf> {
        let name = suggested_name(proposed);
        let n = self.next.fetch_add(1, Ordering::Relaxed);
        let directory = self.root.join(format!("{}-{n}", std::process::id()));
        std::fs::create_dir_all(&self.root)?;
        create_private_dir(&directory)?;
        Ok(directory.join(name))
    }

    /// Whether `path` is a staged download (one directory below the root).
    pub(crate) fn owns(&self, path: &Path) -> bool {
        path.parent().and_then(Path::parent) == Some(self.root.as_path())
    }

    pub(crate) fn on_finished(&self, path: Option<PathBuf>, success: bool) -> Finish {
        let ours = path.clone().filter(|p| self.owns(p));
        match (success, ours) {
            (false, staged) => Finish::Failed { staged },
            (true, Some(staged)) => Finish::Deliver {
                file_name: staged
                    .file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or(UNNAMED)
                    .to_owned(),
                staged,
            },
            (true, None) => Finish::SavedElsewhere(path),
        }
    }

    /// Removes staging directories older than `age`; returns how many went.
    pub(crate) fn remove_abandoned(&self, age: Duration) -> usize {
        let Ok(entries) = std::fs::read_dir(&self.root) else {
            return 0;
        };
        let now = SystemTime::now();
        entries
            .flatten()
            .filter(|entry| {
                entry
                    .metadata()
                    .and_then(|meta| meta.modified())
                    .ok()
                    .and_then(|modified| now.duration_since(modified).ok())
                    .is_some_and(|elapsed| elapsed > age)
            })
            .filter(|entry| std::fs::remove_dir_all(entry.path()).is_ok())
            .count()
    }
}

/// Copies a finished staged download to `destination` (`None`: the dialog
/// was cancelled) and removes the staged copy either way. An existing file
/// at `destination` is replaced only once the full copy is on disk.
pub(crate) fn deliver(staged: &Path, destination: Option<&Path>) -> Result<Delivered, String> {
    let result = match destination {
        None => Ok(Delivered::Cancelled),
        Some(destination) => std::fs::File::open(staged)
            .map_err(|error| format!("Cannot read the finished download: {error}"))
            .and_then(|mut file| write_atomically_from(destination, &mut file, "download"))
            .map(|()| Delivered::Saved(destination.to_path_buf())),
    };
    discard(staged);
    result
}

/// Removes a staged download and its now-empty directory.
pub(crate) fn discard(staged: &Path) {
    if let Err(error) = std::fs::remove_file(staged) {
        if error.kind() != std::io::ErrorKind::NotFound {
            eprintln!("knx-desktop: could not remove a staged download: {error}");
        }
    }
    if let Some(directory) = staged.parent() {
        let _ = std::fs::remove_dir(directory);
    }
}

/// The name the page asked for. wry numbers its proposal (`name (1).ext`)
/// when the Downloads directory already holds the name; staging privately
/// makes that counter meaningless, so it is dropped again.
fn suggested_name(proposed: &Path) -> String {
    let Some(name) = proposed
        .file_name()
        .and_then(|n| n.to_str())
        .filter(|n| !n.is_empty())
    else {
        return UNNAMED.to_owned();
    };
    if let (Some(directory), Some(original)) = (proposed.parent(), without_counter(name)) {
        if directory.join(&original).exists() {
            return original;
        }
    }
    name.to_owned()
}

/// `base (n).ext` → `base.ext`, splitting at the first `.` like wry does.
fn without_counter(name: &str) -> Option<String> {
    let (base, ext) = match name.split_once('.') {
        Some((base, ext)) => (base, format!(".{ext}")),
        None => (name, String::new()),
    };
    let open = base.rfind(" (")?;
    let digits = base[open + 2..].strip_suffix(')')?;
    (!digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit()))
        .then(|| format!("{}{ext}", &base[..open]))
}

#[cfg(unix)]
fn create_private_dir(path: &Path) -> std::io::Result<()> {
    use std::os::unix::fs::DirBuilderExt;
    std::fs::DirBuilder::new().mode(0o700).create(path)
}

#[cfg(not(unix))]
fn create_private_dir(path: &Path) -> std::io::Result<()> {
    std::fs::create_dir(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn staging() -> (tempfile::TempDir, Staging) {
        let home = tempfile::tempdir().unwrap();
        let staging = Staging::new(home.path().join("cache/download-staging"));
        (home, staging)
    }

    #[test]
    fn each_download_gets_its_own_private_directory_under_the_page_name() {
        let (home, staging) = staging();
        let downloads = home.path().join("Downloads");
        let a = staging
            .stage(&downloads.join("import-report.json"))
            .unwrap();
        let b = staging
            .stage(&downloads.join("import-report.json"))
            .unwrap();
        assert_ne!(a.parent(), b.parent());
        assert_eq!(a.file_name().unwrap(), "import-report.json");
        assert!(staging.owns(&a) && staging.owns(&b));
        assert!(!a.starts_with(&downloads), "never written to Downloads");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(a.parent().unwrap())
                .unwrap()
                .permissions()
                .mode();
            assert_eq!(mode & 0o777, 0o700);
        }
    }

    #[test]
    fn wrys_collision_counter_is_dropped_only_when_it_is_one() {
        let (home, staging) = staging();
        let downloads = home.path().join("Downloads");
        std::fs::create_dir_all(&downloads).unwrap();
        std::fs::write(downloads.join("import-report.json"), "earlier").unwrap();
        let staged = staging
            .stage(&downloads.join("import-report (1).json"))
            .unwrap();
        assert_eq!(staged.file_name().unwrap(), "import-report.json");
        // A page that really asked for "notes (2).txt" keeps its name.
        let kept = staging.stage(&downloads.join("notes (2).txt")).unwrap();
        assert_eq!(kept.file_name().unwrap(), "notes (2).txt");
        let unnamed = staging.stage(Path::new("/")).unwrap();
        assert_eq!(unnamed.file_name().unwrap(), UNNAMED);
        assert_eq!(without_counter("a (x).b"), None);
        assert_eq!(without_counter("a (12)"), Some("a".into()));
        assert_eq!(without_counter("a (3).tar.gz"), Some("a.tar.gz".into()));
    }

    #[test]
    fn finished_downloads_are_delivered_failed_or_reported_by_origin() {
        let (home, staging) = staging();
        let staged = staging.stage(&home.path().join("x/report.json")).unwrap();
        assert_eq!(
            staging.on_finished(Some(staged.clone()), true),
            Finish::Deliver {
                staged: staged.clone(),
                file_name: "report.json".into()
            }
        );
        assert_eq!(
            staging.on_finished(Some(staged.clone()), false),
            Finish::Failed {
                staged: Some(staged)
            }
        );
        let elsewhere = home.path().join("Downloads/report (1).json");
        assert_eq!(
            staging.on_finished(Some(elsewhere.clone()), false),
            Finish::Failed { staged: None },
            "a failure outside staging deletes nothing"
        );
        assert_eq!(
            staging.on_finished(Some(elsewhere.clone()), true),
            Finish::SavedElsewhere(Some(elsewhere))
        );
        assert_eq!(
            staging.on_finished(None, true),
            Finish::SavedElsewhere(None)
        );
    }

    #[test]
    fn delivery_copies_atomically_and_never_leaves_the_staged_copy() {
        let (home, staging) = staging();
        let staged = staging.stage(&home.path().join("d/payload.bin")).unwrap();
        let bytes: Vec<u8> = (0..=255u8).cycle().take(300_000).collect();
        std::fs::write(&staged, &bytes).unwrap();
        let target = home.path().join("chosen.bin");
        std::fs::write(&target, "previous export").unwrap();
        assert_eq!(
            deliver(&staged, Some(&target)).unwrap(),
            Delivered::Saved(target.clone())
        );
        assert_eq!(std::fs::read(&target).unwrap(), bytes);
        assert!(!staged.exists() && !staged.parent().unwrap().exists());

        let cancelled = staging.stage(&home.path().join("d/x.json")).unwrap();
        std::fs::write(&cancelled, "{}").unwrap();
        assert_eq!(deliver(&cancelled, None).unwrap(), Delivered::Cancelled);
        assert!(!cancelled.exists());
    }

    #[test]
    fn a_failed_delivery_keeps_the_previous_file_and_still_discards_the_stage() {
        let (home, staging) = staging();
        let staged = staging.stage(&home.path().join("d/r.json")).unwrap();
        std::fs::write(&staged, "new").unwrap();
        let missing_dir = home.path().join("gone/r.json");
        assert!(deliver(&staged, Some(&missing_dir)).is_err());
        assert!(!staged.exists());
        // A staged file that vanished is an error, and nothing is written.
        let target = home.path().join("kept.json");
        std::fs::write(&target, "previous").unwrap();
        assert!(deliver(&staged, Some(&target)).is_err());
        assert_eq!(std::fs::read_to_string(&target).unwrap(), "previous");
    }

    #[test]
    fn only_old_staging_directories_are_removed() {
        let (home, staging) = staging();
        let live = staging.stage(&home.path().join("d/a.json")).unwrap();
        std::fs::write(&live, "live").unwrap();
        let old = staging.stage(&home.path().join("d/b.json")).unwrap();
        std::fs::write(&old, "left over").unwrap();
        std::fs::File::open(old.parent().unwrap())
            .unwrap()
            .set_modified(SystemTime::now() - 2 * ABANDONED_AFTER)
            .unwrap();
        assert_eq!(staging.remove_abandoned(ABANDONED_AFTER), 1);
        assert!(live.exists() && !old.exists());
        let missing = Staging::new(home.path().join("never-created"));
        assert_eq!(missing.remove_abandoned(ABANDONED_AFTER), 0);
    }
}
