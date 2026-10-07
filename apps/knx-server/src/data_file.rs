//! Durable writes and never-delete quarantine for small JSON records in the data directory.
//!
//! Two records live directly in `AppState::data_dir` and share these two
//! mechanics: `settings.json` (`crate::settings`) and `achievements.json`
//! (`crate::achievements`). Each owns its own *shape* — version field,
//! migration or merge rules — and comes here only for the parts that must
//! not differ between them: how a file is replaced without a torn write,
//! and how a file this build cannot use is moved out of the way without
//! ever being deleted.

use std::io::Write;
use std::path::{Path, PathBuf};

/// Writes `body` to `data_dir/file_name` atomically: a sibling temp file,
/// flushed to the platter, then a rename, then an fsync of the directory
/// that holds the rename. A crash mid-write therefore leaves the previous
/// file intact rather than half a document, and a crash just after the
/// rename leaves a whole document rather than a durable name pointing at
/// bytes that never landed.
///
/// The temp file has one fixed name (`<file_name>.tmp`), which is safe only
/// because every caller serialises its writers behind its own lock in
/// `AppState`; two unsynchronised writers would race on that path.
pub(crate) fn write_atomically(
    data_dir: &Path,
    file_name: &str,
    body: &str,
) -> std::io::Result<()> {
    std::fs::create_dir_all(data_dir)?;
    let target = data_dir.join(file_name);
    let temporary = data_dir.join(format!("{file_name}.tmp"));
    {
        let mut file = std::fs::File::create(&temporary)?;
        file.write_all(body.as_bytes())?;
        file.sync_all()?;
    }
    std::fs::rename(&temporary, &target)?;
    // A rename is atomic, which is not the same as durable: without this
    // the directory entry can survive a crash the payload did not.
    std::fs::File::open(data_dir)?.sync_all()?;
    Ok(())
}

/// The current UTC time as the compact stamp every moved-aside name
/// carries: `20260921T120000Z`.
pub(crate) fn utc_stamp() -> String {
    chrono::Utc::now().format("%Y%m%dT%H%M%SZ").to_string()
}

/// Renames `path` to `data_dir/<stem>.<label>-<stamp>.json`, adding `-1`,
/// `-2`, … when that name is taken, and returns where it went. Never a
/// delete: the file may be the only copy of something a user cared about,
/// and "this build could not use it" is a long way from "nobody can".
///
/// The stamp is handed in rather than read from the clock so the
/// collision counter can be tested without waiting for two moves to fall
/// inside the same second.
pub(crate) fn move_aside(
    data_dir: &Path,
    path: &Path,
    stem: &str,
    label: &str,
    stamp: &str,
) -> std::io::Result<PathBuf> {
    let mut moved_to = data_dir.join(format!("{stem}.{label}-{stamp}.json"));
    let mut attempt = 1;
    while moved_to.exists() {
        moved_to = data_dir.join(format!("{stem}.{label}-{stamp}-{attempt}.json"));
        attempt += 1;
    }
    std::fs::rename(path, &moved_to)?;
    Ok(moved_to)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_atomic_write_replaces_the_file_and_leaves_no_temp_file_behind() {
        let dir = tempfile::tempdir().unwrap();
        write_atomically(dir.path(), "record.json", "first\n").unwrap();
        write_atomically(dir.path(), "record.json", "second\n").unwrap();

        assert_eq!(
            std::fs::read_to_string(dir.path().join("record.json")).unwrap(),
            "second\n"
        );
        assert!(!dir.path().join("record.json.tmp").exists());
    }

    #[test]
    fn an_atomic_write_creates_a_missing_data_directory() {
        let dir = tempfile::tempdir().unwrap();
        let nested = dir.path().join("not-yet");
        write_atomically(&nested, "record.json", "{}\n").unwrap();
        assert!(nested.join("record.json").exists());
    }

    #[test]
    fn two_moves_in_the_same_second_get_distinct_names_and_keep_their_bytes() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("record.json");
        let stamp = "20260921T120000Z";

        std::fs::write(&path, "first").unwrap();
        let first = move_aside(dir.path(), &path, "record", "damaged", stamp).unwrap();
        std::fs::write(&path, "second").unwrap();
        let second = move_aside(dir.path(), &path, "record", "damaged", stamp).unwrap();

        assert_eq!(
            first.file_name().unwrap(),
            "record.damaged-20260921T120000Z.json"
        );
        assert_eq!(
            second.file_name().unwrap(),
            "record.damaged-20260921T120000Z-1.json"
        );
        assert_eq!(std::fs::read_to_string(first).unwrap(), "first");
        assert_eq!(std::fs::read_to_string(second).unwrap(), "second");
        assert!(!path.exists());
    }
}
