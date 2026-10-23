use super::*;
use crate::catalogue_jobs::{Incarnation, JobId};

fn workspace() -> (tempfile::TempDir, JobWorkspace) {
    let tmp = tempfile::tempdir().unwrap();
    let root = Arc::new(ArtifactRoot::acquire(tmp.path()).unwrap());
    let incarnation = Incarnation::from_bytes([1; 16]);
    let dir = root.create_incarnation_dir(&incarnation).unwrap();
    let handle = Handle {
        incarnation,
        serial: U64(1),
        job: JobId::from_bytes([2; 16]),
    };
    let workspace = JobWorkspace::create(root, &dir, handle).unwrap();
    (tmp, workspace)
}

#[test]
fn changed_inode_is_not_deleted_or_amnestied() {
    let (_tmp, mut workspace) = workspace();
    let original = workspace.create_file("source.part").unwrap();
    std::fs::rename(
        workspace.directory().join("source.part"),
        workspace.directory().join("not-owned"),
    )
    .unwrap();
    std::fs::write(workspace.directory().join("source.part"), b"replacement").unwrap();
    assert_eq!(workspace.dispose(), Err(ArtifactError::RootInvalid));
    assert_eq!(
        std::fs::read(workspace.directory().join("source.part")).unwrap(),
        b"replacement"
    );
    assert!(workspace.directory().join("inventory.json").is_file());
    drop(original);
}

#[test]
fn restart_reclaims_exact_inventory_but_refuses_unknown_files() {
    let (_tmp, mut workspace) = workspace();
    let directory = workspace.directory().to_path_buf();
    let root = Arc::clone(&workspace._root);
    drop(workspace.create_file("source.part").unwrap());
    drop(workspace);
    JobWorkspace::reclaim_previous(&root).unwrap();
    assert!(!directory.exists());
    assert!(root.root().join("lease").is_file());
    let incarnation = Incarnation::from_bytes([3; 16]);
    let dir = root.create_incarnation_dir(&incarnation).unwrap();
    let mut workspace = JobWorkspace::create(
        Arc::clone(&root),
        &dir,
        Handle {
            incarnation,
            serial: U64(1),
            job: JobId::from_bytes([4; 16]),
        },
    )
    .unwrap();
    drop(workspace.create_file("source.part").unwrap());
    std::fs::write(workspace.directory().join("unrecognised"), b"must survive").unwrap();
    assert_eq!(
        JobWorkspace::reclaim_previous(&root),
        Err(ArtifactError::RootInvalid)
    );
    assert_eq!(
        std::fs::read(workspace.directory().join("unrecognised")).unwrap(),
        b"must survive"
    );
}

#[test]
fn recovery_publication_never_clobbers_and_workspace_disposal_keeps_final_backup() {
    let (tmp, mut workspace) = workspace();
    let catalogue = tmp.path().join("catalogue.sqlite");
    let digest = WireSha256::from_bytes([0; 32]);
    let first = tmp
        .path()
        .join("catalogue.sqlite.backup-0-000000000000.sqlite");
    std::fs::write(&first, b"existing user backup").unwrap();
    let mut staging = workspace.create_file("recovery.part.sqlite").unwrap();
    // Publication-mechanics fixture only; B10 separately validates real SQLite
    // content and explicit native close before invoking this primitive.
    staging.write_all(b"closed validated-copy fixture").unwrap();
    staging.sync_all().unwrap();
    drop(staging);
    workspace
        .refresh("recovery.part.sqlite", "complete")
        .unwrap();
    let publication = workspace.publish_recovery(&catalogue, 0, digest).unwrap();
    assert!(publication.sync_acknowledged);
    assert_eq!(
        publication.name,
        "catalogue.sqlite.backup-0-000000000000-1.sqlite"
    );
    assert_eq!(std::fs::read(&first).unwrap(), b"existing user backup");
    let published = tmp.path().join(&publication.name);
    assert!(published.is_file());
    workspace.dispose().unwrap();
    assert!(published.is_file());
}

#[test]
fn native_sidecars_require_explicit_closed_journal_reconciliation() {
    let (_tmp, mut workspace) = workspace();
    drop(workspace.create_file("rehearsal.sqlite").unwrap());
    workspace
        .admit_database_interval("rehearsal.sqlite")
        .unwrap();
    let mut journal = create_new(&workspace.directory().join("rehearsal.sqlite-journal")).unwrap();
    journal.write_all(b"closed non-hot fixture").unwrap();
    drop(journal);
    assert_eq!(
        workspace.admit_database_interval("rehearsal.sqlite"),
        Err(ArtifactError::RootInvalid)
    );
    assert_eq!(
        workspace.reconcile_closed_database("rehearsal.sqlite", false),
        Err(ArtifactError::RootInvalid)
    );
    workspace
        .reconcile_closed_database("rehearsal.sqlite", true)
        .unwrap();
    workspace.cleanup_transients().unwrap();
    assert!(!workspace
        .directory()
        .join("rehearsal.sqlite-journal")
        .exists());
    assert!(!workspace.directory().join("rehearsal.sqlite").exists());
}
