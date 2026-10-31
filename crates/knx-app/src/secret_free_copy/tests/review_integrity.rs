//! Pins saved-root authority, source immutability and scratch cleanup on success and refusal.

use super::*;

#[test]
fn secret_free_native_active_wal_is_refused_without_touching_sidecars() {
    let dir = tempfile::tempdir().unwrap();
    let source = native_project(dir.path(), false);
    let conn = knx_store::open_existing_and_migrate(&source).unwrap();
    conn.pragma_update(None, "journal_mode", "WAL").unwrap();
    conn.execute("UPDATE project_info SET name='Live WAL saved root'", [])
        .unwrap();
    let before = directory_bytes(dir.path());
    assert!(before
        .keys()
        .any(|name| name.to_string_lossy().ends_with("-wal")));
    assert!(before
        .keys()
        .any(|name| name.to_string_lossy().ends_with("-shm")));
    let result = native_project_without_secrets(&source);
    assert!(
        result.is_err(),
        "live WAL source must be unavailable, not opened through a sidecar-writing reader"
    );
    assert_eq!(
        directory_bytes(dir.path()),
        before,
        "source main/WAL/SHM bytes changed"
    );
}

fn directory_bytes(path: &Path) -> BTreeMap<std::ffi::OsString, Vec<u8>> {
    std::fs::read_dir(path)
        .unwrap()
        .map(|entry| {
            let entry = entry.unwrap();
            (entry.file_name(), std::fs::read(entry.path()).unwrap())
        })
        .collect()
}

#[test]
fn secret_free_native_saved_root_not_recovery_workspace_is_authority() {
    use knx_store::project_history::{self, NativeSnapshot};
    let dir = tempfile::tempdir().unwrap();
    let source = native_project(dir.path(), false);
    let conn = knx_store::open_existing_and_migrate(&source).unwrap();
    let saved = NativeSnapshot::read(&conn).unwrap();
    let mut working = saved.clone();
    working.project.info.name = "Unsaved recoverable editor name".into();
    project_history::save_editor(&conn, &working, &knx_core::CommandStack::new(), None, false)
        .unwrap();
    let editor = project_history::load_editor(&conn).unwrap().unwrap();
    assert_eq!(editor.baseline, saved);
    assert_eq!(editor.working, working);
    drop(conn);
    let before = directory_bytes(dir.path());
    let mut scratch = PathBuf::new();
    let copy = native::copy_observed(&source, |path| scratch = path.to_path_buf()).unwrap();
    assert!(
        !scratch.exists(),
        "successful copy stranded private scratch data"
    );
    assert_eq!(
        directory_bytes(dir.path()),
        before,
        "source main/sidecars/directory changed"
    );
    let out = reopened(&copy.bytes, dir.path());
    assert_eq!(knx_store::load_project(&out.conn).unwrap(), saved.project);
    assert!(project_history::load_editor(&out.conn).unwrap().is_none());
    assert!(copy
        .withheld
        .iter()
        .any(|reason| reason.starts_with("project history:")));
}

#[test]
fn secret_free_native_refusal_preserves_source_and_removes_scratch() {
    let dir = tempfile::tempdir().unwrap();
    let source = native_project(dir.path(), false);
    let conn = knx_store::open_existing_and_migrate(&source).unwrap();
    conn.execute("UPDATE project_info SET name='ModelPasswordWitness'", [])
        .unwrap();
    conn.execute("INSERT INTO opaque_entry(source_path,xpath,kind,name,bytes,sha256) VALUES('witness','/a','RetainedAttribute','Password',?1,?2)",
        (b"ModelPasswordWitness".as_slice(),knx_store::project_history::image_hash(b"ModelPasswordWitness"))).unwrap();
    drop(conn);
    let before = directory_bytes(dir.path());
    let mut scratch = PathBuf::new();
    let error = native::copy_observed(&source, |path| scratch = path.to_path_buf()).unwrap_err();
    assert!(matches!(error, SecretFreeCopyError::ResidualSecret { .. }));
    assert!(!scratch.exists(), "refusal stranded private scratch data");
    assert_eq!(
        directory_bytes(dir.path()),
        before,
        "source main/sidecars/directory changed on refusal"
    );
}

#[test]
fn secret_free_native_reopen_error_does_not_echo_storage_values() {
    let dir = tempfile::tempdir().unwrap();
    let source = native_project(dir.path(), false);
    let conn = knx_store::open_existing_and_migrate(&source).unwrap();
    conn.execute(
        "UPDATE project_info SET group_address_style='StoragePasswordWitness'",
        [],
    )
    .unwrap();
    drop(conn);
    let before = directory_bytes(dir.path());
    let error = native_project_without_secrets(&source).unwrap_err();
    assert!(!error.to_string().contains("StoragePasswordWitness"));
    assert!(!format!("{error:?}").contains("StoragePasswordWitness"));
    assert_eq!(directory_bytes(dir.path()), before);
}
