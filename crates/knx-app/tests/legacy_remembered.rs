//! The one remembered legacy password: a 0600 file, and the order in which passwords are tried.

use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;

use knx_app::legacy::{
    open_with_password_policy, LegacyOpenError, LegacyPassword, PasswordUsed, RememberError,
    RememberedPassword,
};
use knx_productdb::legacy::LegacyError;

const PASSWORD: &str = "marvin-synthetic";

fn fixture(name: &str) -> Vec<u8> {
    std::fs::read(
        knx_testsupport::workspace_root()
            .join("crates/knx-productdb/fixtures/legacy")
            .join(name),
    )
    .unwrap()
}

fn store_in(dir: &tempfile::TempDir) -> (RememberedPassword, PathBuf) {
    let path = dir.path().join("config/knx/legacy-vd-password");
    (RememberedPassword::at(path.clone()), path)
}

fn mode(path: &std::path::Path) -> u32 {
    std::fs::metadata(path).unwrap().permissions().mode() & 0o777
}

#[test]
fn a_stored_password_lives_in_a_private_file_and_comes_back() {
    let dir = tempfile::tempdir().unwrap();
    let (store, path) = store_in(&dir);
    assert!(!store.is_set());
    assert!(store.load().unwrap().is_none());
    store.store(&LegacyPassword::new(PASSWORD)).unwrap();
    assert!(store.is_set());
    assert_eq!(mode(&path), 0o600);
    assert_eq!(mode(path.parent().unwrap()), 0o700);
    assert_eq!(
        std::fs::read(&path).unwrap(),
        format!("{PASSWORD}\n").as_bytes()
    );
    // What comes back opens the encrypted fixture.
    let remembered = store.load().unwrap().expect("remembered");
    let (_, used) =
        open_with_password_policy(&fixture("marvin-program.vd4"), Some(&remembered), None).unwrap();
    assert_eq!(used, PasswordUsed::Given);
}

#[test]
fn storing_again_replaces_the_one_password() {
    let dir = tempfile::tempdir().unwrap();
    let (store, path) = store_in(&dir);
    store.store(&LegacyPassword::new("first")).unwrap();
    store.store(&LegacyPassword::new("second")).unwrap();
    assert_eq!(std::fs::read(&path).unwrap(), b"second\n");
    assert_eq!(mode(&path), 0o600);
    let leftovers: Vec<_> = std::fs::read_dir(path.parent().unwrap())
        .unwrap()
        .map(|e| e.unwrap().file_name())
        .collect();
    assert_eq!(leftovers, [std::ffi::OsString::from("legacy-vd-password")]);
}

#[test]
fn forgetting_removes_the_file_and_says_whether_there_was_one() {
    let dir = tempfile::tempdir().unwrap();
    let (store, path) = store_in(&dir);
    assert!(!store.forget().unwrap());
    store.store(&LegacyPassword::new(PASSWORD)).unwrap();
    assert!(store.forget().unwrap());
    assert!(!path.exists());
    assert!(!store.is_set());
}

#[test]
fn an_empty_password_is_never_stored() {
    let dir = tempfile::tempdir().unwrap();
    let (store, path) = store_in(&dir);
    assert!(matches!(
        store.store(&LegacyPassword::new("")),
        Err(RememberError::Empty)
    ));
    assert!(!path.exists());
}

#[test]
fn a_file_others_can_read_is_refused_not_used() {
    let dir = tempfile::tempdir().unwrap();
    let (store, path) = store_in(&dir);
    store.store(&LegacyPassword::new(PASSWORD)).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).unwrap();
    match store.load() {
        Err(error @ RememberError::TooOpen { .. }) => {
            assert!(!format!("{error}").contains(PASSWORD));
            assert!(format!("{error}").contains("0600"), "{error}");
        }
        other => panic!("expected TooOpen, got {other:?}"),
    }
}

#[test]
fn the_password_never_appears_in_errors_or_debug_output() {
    let dir = tempfile::tempdir().unwrap();
    let (store, _) = store_in(&dir);
    store
        .store(&LegacyPassword::new("canary-remembered"))
        .unwrap();
    let loaded = store.load().unwrap().unwrap();
    assert!(!format!("{loaded:?}{store:?}").contains("canary-remembered"));
    let error =
        open_with_password_policy(&fixture("marvin-program.vd4"), None, Some(&store)).unwrap_err();
    assert!(!format!("{error}{error:?}").contains("canary-remembered"));
}

#[test]
fn a_given_password_wins_over_the_remembered_one() {
    let dir = tempfile::tempdir().unwrap();
    let (store, _) = store_in(&dir);
    store.store(&LegacyPassword::new("not-this-one")).unwrap();
    let given = LegacyPassword::new(PASSWORD);
    let (_, used) =
        open_with_password_policy(&fixture("marvin-program.vd4"), Some(&given), Some(&store))
            .unwrap();
    assert_eq!(used, PasswordUsed::Given);
}

#[test]
fn without_a_given_password_the_remembered_one_is_used() {
    let dir = tempfile::tempdir().unwrap();
    let (store, _) = store_in(&dir);
    store.store(&LegacyPassword::new(PASSWORD)).unwrap();
    let (payload, used) =
        open_with_password_policy(&fixture("marvin-program.vd4"), None, Some(&store)).unwrap();
    assert_eq!(used, PasswordUsed::Remembered);
    assert!(payload.container().encrypted);
    // An empty given password counts as none.
    let empty = LegacyPassword::new("");
    let (_, used) =
        open_with_password_policy(&fixture("marvin-program.vd4"), Some(&empty), Some(&store))
            .unwrap();
    assert_eq!(used, PasswordUsed::Remembered);
}

#[test]
fn a_remembered_password_that_does_not_fit_is_named_as_such() {
    let dir = tempfile::tempdir().unwrap();
    let (store, _) = store_in(&dir);
    store
        .store(&LegacyPassword::new("wrong-remembered"))
        .unwrap();
    let error =
        open_with_password_policy(&fixture("marvin-program.vd4"), None, Some(&store)).unwrap_err();
    assert!(
        matches!(error, LegacyOpenError::RememberedDoesNotFit),
        "{error:?}"
    );
    // A wrong *given* password stays the plain legacy error.
    let wrong = LegacyPassword::new("wrong-given");
    let error =
        open_with_password_policy(&fixture("marvin-program.vd4"), Some(&wrong), Some(&store))
            .unwrap_err();
    assert!(
        matches!(
            error,
            LegacyOpenError::Legacy(
                LegacyError::WrongPassword | LegacyError::WrongPasswordOrCorrupt
            )
        ),
        "{error:?}"
    );
}

#[test]
fn nothing_given_and_nothing_remembered_asks_for_a_password() {
    let dir = tempfile::tempdir().unwrap();
    let (store, _) = store_in(&dir);
    for remembered in [None, Some(&store)] {
        let error = open_with_password_policy(&fixture("marvin-program.vd4"), None, remembered)
            .unwrap_err();
        assert!(
            matches!(
                error,
                LegacyOpenError::Legacy(LegacyError::PasswordRequired)
            ),
            "{error:?}"
        );
    }
}

#[test]
fn an_unencrypted_file_needs_no_password_and_reports_none_used() {
    let dir = tempfile::tempdir().unwrap();
    let (store, _) = store_in(&dir);
    store.store(&LegacyPassword::new(PASSWORD)).unwrap();
    let (_, used) =
        open_with_password_policy(&fixture("marvin-program-plain.vd4"), None, Some(&store))
            .unwrap();
    assert_eq!(used, PasswordUsed::None);
}

#[test]
fn an_unusable_remembered_file_is_reported_only_when_it_is_needed() {
    let dir = tempfile::tempdir().unwrap();
    let (store, path) = store_in(&dir);
    store.store(&LegacyPassword::new(PASSWORD)).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o640)).unwrap();
    // A given password does not need the store.
    let given = LegacyPassword::new(PASSWORD);
    assert!(
        open_with_password_policy(&fixture("marvin-program.vd4"), Some(&given), Some(&store))
            .is_ok()
    );
    let error =
        open_with_password_policy(&fixture("marvin-program.vd4"), None, Some(&store)).unwrap_err();
    assert!(
        matches!(
            error,
            LegacyOpenError::Remembered(RememberError::TooOpen { .. })
        ),
        "{error:?}"
    );
}

#[test]
fn the_default_path_follows_xdg_config_home() {
    let path =
        knx_app::legacy::remembered_password_path_from(Some("/xdg".into()), Some("/home/u".into()));
    assert_eq!(path, Some(PathBuf::from("/xdg/knx/legacy-vd-password")));
    let path = knx_app::legacy::remembered_password_path_from(None, Some("/home/u".into()));
    assert_eq!(
        path,
        Some(PathBuf::from("/home/u/.config/knx/legacy-vd-password"))
    );
    let path =
        knx_app::legacy::remembered_password_path_from(Some("".into()), Some("/home/u".into()));
    assert_eq!(
        path,
        Some(PathBuf::from("/home/u/.config/knx/legacy-vd-password"))
    );
    assert_eq!(
        knx_app::legacy::remembered_password_path_from(None, None),
        None
    );
}

#[test]
fn a_remembered_file_with_an_empty_first_line_is_named_not_ignored() {
    let dir = tempfile::tempdir().unwrap();
    let (store, path) = store_in(&dir);
    store.store(&LegacyPassword::new(PASSWORD)).unwrap();
    std::fs::write(&path, b"\nlater line\n").unwrap();
    assert!(store.is_set());
    match store.load() {
        Err(error @ RememberError::EmptyFile { .. }) => {
            assert!(format!("{error}").contains("empty"), "{error}");
        }
        other => panic!("expected EmptyFile, got {other:?}"),
    }
    let error =
        open_with_password_policy(&fixture("marvin-program.vd4"), None, Some(&store)).unwrap_err();
    assert!(
        matches!(
            error,
            LegacyOpenError::Remembered(RememberError::EmptyFile { .. })
        ),
        "{error:?}"
    );
    assert!(store.forget().unwrap());
}

#[test]
fn two_stores_at_once_both_finish_and_one_password_wins() {
    let dir = tempfile::tempdir().unwrap();
    let (store, path) = store_in(&dir);
    std::thread::scope(|scope| {
        let handles: Vec<_> = (0..8)
            .map(|i| {
                let store = store.clone();
                scope.spawn(move || store.store(&LegacyPassword::new(format!("pw-{i}"))))
            })
            .collect();
        for handle in handles {
            handle
                .join()
                .unwrap()
                .expect("every concurrent store succeeds");
        }
    });
    let kept = std::fs::read_to_string(&path).unwrap();
    assert!(kept.starts_with("pw-") && kept.ends_with('\n'), "{kept:?}");
    let leftovers = std::fs::read_dir(path.parent().unwrap()).unwrap().count();
    assert_eq!(leftovers, 1, "no temporary file is left behind");
}
