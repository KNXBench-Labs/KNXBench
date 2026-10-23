//! Characterization of lifecycle refusals and retained inventory, on real disks.
use super::*;
use crate::catalogue_copy::{CopyError, DestinationCleanup, DestinationError, DestinationStage};
use std::sync::{Arc, Mutex};

#[test]
fn create_new_never_adopts_or_overwrites_an_existing_file() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("owned.sqlite");
    std::fs::write(&path, b"keep these original bytes").unwrap();
    let mut inventory = ArtifactInventory::default();
    assert!(inventory
        .create_new(DestinationRole::Recovery, path.clone(), 4096)
        .is_err());
    assert!(!inventory.allocated(DestinationRole::Recovery));
    assert_eq!(std::fs::read(path).unwrap(), b"keep these original bytes");
}
#[test]
fn memory_path_cannot_mint_fresh() {
    const CHILD: &str = "KNXBENCH_COPY_MEMORY_CHILD";
    if std::env::var_os(CHILD).is_some() {
        let mut inventory = ArtifactInventory::default();
        assert!(
            inventory
                .create_new(DestinationRole::Recovery, ":memory:".into(), 4096)
                .is_err(),
            "a special SQLite memory path must not mint Fresh"
        );
        assert!(
            inventory
                .retained_connection(DestinationRole::Recovery)
                .is_none(),
            "refusal precedes any SQLite open or backup"
        );
        return;
    }
    // Isolate SQLite's special relative name in a child-owned scratch cwd;
    // changing a process-wide cwd in the parallel parent would be unsafe.
    let dir = tempfile::tempdir().unwrap();
    let result = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "orchestration::tests::reopened_copy_controls::memory_path_cannot_mint_fresh",
            "--nocapture",
        ])
        .env(CHILD, "1")
        .current_dir(dir.path())
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
}
fn fresh_refusal(sql: &'static str, expected: fn(&CopyError) -> bool) {
    struct InvalidFresh {
        inner: TestArtifacts,
        sql: &'static str,
    }
    impl CatalogueArtifacts for InvalidFresh {
        fn inventory(&mut self) -> &mut ArtifactInventory {
            self.inner.inventory()
        }
        fn fresh_rehearsal(&mut self, size: i64) -> Result<FreshDestination, OrchestrationError> {
            let receipt = self.inner.fresh_rehearsal(size)?;
            let sql = self.sql;
            self.inventory().test_writer(&receipt).execute_batch(sql)?;
            Ok(receipt)
        }
        fn fresh_recovery(&mut self, _: i64) -> Result<FreshDestination, OrchestrationError> {
            panic!("no B in preview")
        }
    }
    let (_dir, mut owner) = migrated_owner();
    let bytes = valid_package();
    let gate = PublicationGate::new();
    let before = owner.capture_stamp(&gate).unwrap();
    let mut artifacts = InvalidFresh {
        inner: TestArtifacts::new(),
        sql,
    };
    let result = owner.preview_current_controlled(
        admit("fresh.knxprod", &bytes),
        &OperationContext {
            gate: &gate,
            policy: ControlledPolicy::Standard,
        },
        &mut artifacts,
    );
    let failure = match result {
        Err(OrchestrationError::Copy(CopyError::Destination(e))) => e,
        _ => panic!("expected typed pre-backup refusal"),
    };
    assert!(matches!(failure.primary,Some(DestinationError::Copy(ref e)) if expected(e)));
    assert!(failure.cleanup.is_empty());
    assert!(artifacts
        .inventory()
        .retained_connection(DestinationRole::Rehearsal)
        .is_none());
    assert_eq!(
        owner.capture_stamp(&PublicationGate::new()).unwrap(),
        before
    );
}
#[test]
fn nonfresh_destination_refuses_before_backup() {
    fresh_refusal("CREATE TABLE forbidden(x)", |e| {
        matches!(e, CopyError::DestinationNotFresh(_))
    })
}
#[test]
fn active_destination_refuses_before_backup() {
    fresh_refusal("BEGIN", |e| {
        matches!(e, CopyError::DestinationInTransaction)
    })
}
#[test]
fn unequal_page_size_refuses_before_backup() {
    fresh_refusal("PRAGMA page_size=8192", |e| {
        matches!(e, CopyError::PageSizeMismatch { .. })
    })
}

fn seam_refusal(stage: DestinationStage, change: fn(&std::path::Path)) {
    let (_dir, mut owner) = migrated_owner();
    let bytes = valid_package();
    let gate = PublicationGate::new();
    let before = owner.capture_stamp(&gate).unwrap();
    let mut artifacts = TestArtifacts::new();
    artifacts.inventory.set_hook(
        DestinationRole::Rehearsal,
        Box::new(move |observed, path, _| {
            if observed == stage {
                change(path)
            }
        }),
    );
    let result = owner.preview_current_controlled(
        admit("seam.knxprod", &bytes),
        &OperationContext {
            gate: &gate,
            policy: ControlledPolicy::Standard,
        },
        &mut artifacts,
    );
    assert!(
        matches!(result, Err(OrchestrationError::Copy(_))),
        "seam refusal must precede consumer"
    );
    assert_eq!(
        owner.capture_stamp(&PublicationGate::new()).unwrap(),
        before
    );
    assert!(!artifacts.inventory.allocated(DestinationRole::Recovery));
}
#[test]
fn missing_ro_target_refuses_without_recreation() {
    seam_refusal(DestinationStage::ReadOpen, |p| {
        std::fs::rename(p, p.with_extension("retained")).unwrap()
    })
}
#[test]
fn linked_ro_target_refuses_without_deletion() {
    seam_refusal(DestinationStage::ReadOpen, |p| {
        std::fs::hard_link(p, p.with_extension("retained")).unwrap()
    })
}
#[test]
fn symlink_ro_target_refuses_without_following() {
    seam_refusal(DestinationStage::ReadOpen, |p| {
        let kept = p.with_extension("retained");
        std::fs::rename(p, &kept).unwrap();
        std::os::unix::fs::symlink(kept, p).unwrap()
    })
}
#[test]
fn identical_rw_replacement_refuses() {
    seam_refusal(DestinationStage::WriteOpen, |p| {
        let replacement = p.with_extension("replacement");
        std::fs::copy(p, &replacement).unwrap();
        std::fs::rename(replacement, p).unwrap()
    })
}
#[test]
fn modified_ro_target_refuses_even_after_restoring_equal_bytes() {
    seam_refusal(DestinationStage::ReadOpen, |p| {
        let bytes = std::fs::read(p).unwrap();
        std::fs::write(p, &bytes).unwrap()
    })
}
#[test]
fn modified_already_open_rw_target_refuses_before_consumer() {
    seam_refusal(DestinationStage::WriteReady, |p| {
        let c = crate::Connection::open(p).unwrap();
        c.execute(
            "INSERT INTO manufacturer(id,name) VALUES('late','Not validated')",
            [],
        )
        .unwrap();
        c.close().unwrap()
    })
}

struct SubstitutedSource {
    inner: TestArtifacts,
    replacement: std::path::PathBuf,
}
impl CatalogueArtifacts for SubstitutedSource {
    fn inventory(&mut self) -> &mut ArtifactInventory {
        self.inner.inventory()
    }
    fn before_source_open(&mut self, path: &std::path::Path) {
        std::fs::rename(path, path.with_extension("displaced")).unwrap();
        std::fs::rename(&self.replacement, path).unwrap();
    }
    fn fresh_rehearsal(&mut self, size: i64) -> Result<FreshDestination, OrchestrationError> {
        self.inner.fresh_rehearsal(size)
    }
    fn fresh_recovery(&mut self, size: i64) -> Result<FreshDestination, OrchestrationError> {
        self.inner.fresh_recovery(size)
    }
}
fn source_refusal(old: bool) {
    let (dir, mut owner) = migrated_owner();
    let bytes = valid_package();
    let gate = PublicationGate::new();
    let op = OperationContext {
        gate: &gate,
        policy: ControlledPolicy::Standard,
    };
    let shown = owner
        .preview_current_controlled(
            admit("source.knxprod", &bytes),
            &op,
            &mut TestArtifacts::new(),
        )
        .unwrap()
        .binding;
    let replacement = dir.path().join("other.sqlite");
    if old {
        crate::Connection::open(&replacement)
            .unwrap()
            .close()
            .unwrap();
    } else {
        std::fs::copy(dir.path().join("products.sqlite"), &replacement).unwrap();
        let conn = crate::Connection::open(&replacement).unwrap();
        conn.execute(
            "INSERT INTO manufacturer(id,name) VALUES('other','Mismatched source')",
            [],
        )
        .unwrap();
        conn.close().unwrap();
    }
    let mut artifacts = SubstitutedSource {
        inner: TestArtifacts::new(),
        replacement,
    };
    let result = owner.apply_current_controlled(
        admit("source.knxprod", &bytes),
        &shown,
        &op,
        &mut artifacts,
    );
    assert_eq!(result.commit, CommitEvidence::NotCommitted);
    assert_eq!(result.rollback, RollbackDisposition::Explicit);
    assert_eq!(result.artifacts, ArtifactDisposition::None);
    assert!(matches!(result.health, WriterHealth::Quarantined(_)));
    assert_eq!(artifacts.inner.counter, 0);
    assert!(matches!(
        &result.cleanup[..],
        [PreviewCleanupError::WriterBaseline]
    ));
    if old {
        assert!(matches!(
            result.failure,
            Some(OrchestrationError::Stamp(
                crate::catalogue_stamp::StampError::Witness(
                    crate::catalogue_schema::WitnessError::Admission(
                        crate::catalogue_schema::AdmissionError::UnsupportedVersion(0)
                    )
                )
            ))
        ));
        let actual = crate::Connection::open_with_flags(
            dir.path().join("products.sqlite"),
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
        )
        .unwrap();
        assert_eq!(
            actual
                .query_row("PRAGMA user_version", [], |r| r.get::<_, i64>(0))
                .unwrap(),
            0,
            "strict apply S must not migrate a memory substitute or the file"
        );
        actual.close().unwrap();
    } else {
        assert!(matches!(result.failure, Some(OrchestrationError::Stale)));
    }
}
#[test]
fn apply_strict_source_refuses_actual_old_version_without_migration() {
    source_refusal(true)
}
#[test]
fn apply_full_source_witness_refuses_ordinary_data_mismatch() {
    source_refusal(false)
}

struct SourceEndArtifacts {
    inner: TestArtifacts,
    gate: Arc<PublicationGate>,
    attempts: Box<usize>,
}
unsafe extern "C" fn refuse_source_end(
    context: *mut std::ffi::c_void,
    action: std::ffi::c_int,
    first: *const std::ffi::c_char,
    _: *const std::ffi::c_char,
    _: *const std::ffi::c_char,
    _: *const std::ffi::c_char,
) -> std::ffi::c_int {
    if action == rusqlite::ffi::SQLITE_TRANSACTION
        && !first.is_null()
        && unsafe { std::ffi::CStr::from_ptr(first) }.to_bytes() == b"ROLLBACK"
    {
        unsafe { *context.cast::<usize>() += 1 };
        rusqlite::ffi::SQLITE_DENY
    } else {
        rusqlite::ffi::SQLITE_OK
    }
}
impl CatalogueArtifacts for SourceEndArtifacts {
    fn inventory(&mut self) -> &mut ArtifactInventory {
        self.inner.inventory()
    }
    fn inspect_pinned_source(&mut self, conn: &crate::Connection) {
        assert!(!conn.is_autocommit());
        assert!(conn.is_readonly("main").unwrap());
        assert_eq!(
            unsafe {
                rusqlite::ffi::sqlite3_set_authorizer(
                    conn.handle(),
                    Some(refuse_source_end),
                    (&mut *self.attempts as *mut usize).cast(),
                )
            },
            rusqlite::ffi::SQLITE_OK
        );
        assert_eq!(
            self.gate.try_cancel(),
            crate::operation::CancelOutcome::Accepted
        );
    }
    fn fresh_rehearsal(&mut self, size: i64) -> Result<FreshDestination, OrchestrationError> {
        self.inner.fresh_rehearsal(size)
    }
    fn fresh_recovery(&mut self, size: i64) -> Result<FreshDestination, OrchestrationError> {
        self.inner.fresh_recovery(size)
    }
}
#[test]
fn apply_preserves_source_end_failure_and_primary_stop_then_settles_writer() {
    let (_dir, mut owner) = migrated_owner();
    let bytes = valid_package();
    let gate = Arc::new(PublicationGate::new());
    let op = OperationContext {
        gate: &gate,
        policy: ControlledPolicy::Standard,
    };
    let shown = owner
        .preview_current_controlled(
            admit("source.knxprod", &bytes),
            &op,
            &mut TestArtifacts::new(),
        )
        .unwrap()
        .binding;
    let mut artifacts = SourceEndArtifacts {
        inner: TestArtifacts::new(),
        gate: gate.clone(),
        attempts: Box::new(0),
    };
    let result = owner.apply_current_controlled(
        admit("source.knxprod", &bytes),
        &shown,
        &op,
        &mut artifacts,
    );
    assert_eq!(result.commit, CommitEvidence::NotCommitted);
    assert_eq!(result.rollback, RollbackDisposition::Explicit);
    assert_eq!(result.artifacts, ArtifactDisposition::None);
    assert!(matches!(result.health, WriterHealth::Quarantined(_)));
    assert_eq!(artifacts.inner.counter, 0);
    assert_eq!(*artifacts.attempts, 1);
    let failure = match result.failure {
        Some(OrchestrationError::SourceSettlement(e)) => e,
        _ => panic!("expected independent source settlement errors"),
    };
    assert!(matches!(
        failure.primary,
        Some(OrchestrationError::Stamp(
            crate::catalogue_stamp::StampError::Witness(
                crate::catalogue_schema::WitnessError::Stop(_)
            )
        ))
    ));
    assert!(matches!(
        &failure.cleanup[..],
        [PreviewCleanupError::SourceEnd(_)]
    ));
}

#[test]
fn changed_parent_identity_refuses_even_with_original_main_inode() {
    struct ParentArtifacts {
        dir: tempfile::TempDir,
        inventory: ArtifactInventory,
    }
    impl CatalogueArtifacts for ParentArtifacts {
        fn inventory(&mut self) -> &mut ArtifactInventory {
            &mut self.inventory
        }
        fn fresh_rehearsal(&mut self, size: i64) -> Result<FreshDestination, OrchestrationError> {
            Ok(self.inventory.create_new(
                DestinationRole::Rehearsal,
                self.dir.path().join("sub/D.sqlite"),
                size,
            )?)
        }
        fn fresh_recovery(&mut self, _: i64) -> Result<FreshDestination, OrchestrationError> {
            panic!("no B in preview")
        }
    }
    let (_dir, mut owner) = migrated_owner();
    let bytes = valid_package();
    let gate = PublicationGate::new();
    let mut artifacts = ParentArtifacts {
        dir: tempfile::tempdir().unwrap(),
        inventory: ArtifactInventory::default(),
    };
    std::fs::create_dir(artifacts.dir.path().join("sub")).unwrap();
    artifacts.inventory.set_hook(
        DestinationRole::Rehearsal,
        Box::new(|stage, path, _| {
            if stage == DestinationStage::ReadOpen {
                let parent = path.parent().unwrap();
                let displaced = parent.with_extension("displaced");
                std::fs::rename(parent, &displaced).unwrap();
                std::fs::create_dir(parent).unwrap();
                std::fs::rename(displaced.join(path.file_name().unwrap()), path).unwrap();
            }
        }),
    );
    let result = owner.preview_current_controlled(
        admit("parent.knxprod", &bytes),
        &OperationContext {
            gate: &gate,
            policy: ControlledPolicy::Standard,
        },
        &mut artifacts,
    );
    assert!(
        matches!(result,Err(OrchestrationError::Copy(CopyError::Destination(ref e))) if matches!(e.primary,Some(DestinationError::Identity)))
    );
    assert_eq!(package_count(&owner), 0);
}

#[test]
fn artifact_role_receipt_cannot_be_used_in_the_other_slot() {
    struct SwappedRoles {
        inner: TestArtifacts,
    }
    impl CatalogueArtifacts for SwappedRoles {
        fn inventory(&mut self) -> &mut ArtifactInventory {
            self.inner.inventory()
        }
        fn fresh_rehearsal(&mut self, size: i64) -> Result<FreshDestination, OrchestrationError> {
            self.inner.fresh_recovery(size)
        }
        fn fresh_recovery(&mut self, size: i64) -> Result<FreshDestination, OrchestrationError> {
            self.inner.fresh_rehearsal(size)
        }
    }
    let (_dir, mut owner) = migrated_owner();
    let bytes = valid_package();
    let gate = PublicationGate::new();
    let op = OperationContext {
        gate: &gate,
        policy: ControlledPolicy::Standard,
    };
    let shown = owner
        .preview_current_controlled(
            admit("roles.knxprod", &bytes),
            &op,
            &mut TestArtifacts::new(),
        )
        .unwrap()
        .binding;
    let mut artifacts = SwappedRoles {
        inner: TestArtifacts::new(),
    };
    let result =
        owner.apply_current_controlled(admit("roles.knxprod", &bytes), &shown, &op, &mut artifacts);
    assert_eq!(
        result.commit,
        CommitEvidence::NotCommitted,
        "D/B receipts are role-bound, not interchangeable fresh connections"
    );
    assert!(
        matches!(result.failure,Some(OrchestrationError::Copy(CopyError::Destination(ref e))) if matches!(e.primary,Some(DestinationError::InvalidReceipt)))
    );
    let conn = artifacts
        .inventory()
        .retained_connection(DestinationRole::Recovery)
        .unwrap();
    assert_eq!(
        conn.query_row("PRAGMA schema_version", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        0,
        "wrong-role receipt must refuse before backup"
    );
}

fn validation_reservation(apply: bool) {
    use std::cell::RefCell;
    use std::rc::Rc;
    let (dir, mut owner) = migrated_owner();
    let bytes = valid_package();
    let gate = PublicationGate::new();
    let op = OperationContext {
        gate: &gate,
        policy: ControlledPolicy::Standard,
    };
    let before = owner.capture_stamp(&gate).unwrap();
    let shown = if apply {
        Some(
            owner
                .preview_current_controlled(
                    admit("reservation.knxprod", &bytes),
                    &op,
                    &mut TestArtifacts::new(),
                )
                .unwrap()
                .binding,
        )
    } else {
        None
    };
    let contender = crate::Connection::open(dir.path().join("products.sqlite")).unwrap();
    contender.busy_timeout(std::time::Duration::ZERO).unwrap();
    let contender = Rc::new(RefCell::new(contender));
    let observed = Rc::new(RefCell::new(Vec::new()));
    let mut artifacts = TestArtifacts::new();
    for role in [DestinationRole::Rehearsal, DestinationRole::Recovery] {
        let contender = contender.clone();
        let observed = observed.clone();
        artifacts.inventory.set_hook(role,Box::new(move |stage,_,conn| {
            if stage==DestinationStage::ReadEnd {
                let conn=conn.unwrap();assert!(conn.is_readonly("main").unwrap());assert!(!conn.is_autocommit(),"observe W after the actual full RO validation, before its inspected end");
                let contender=contender.borrow();let result=contender.execute_batch("BEGIN IMMEDIATE");let blocked=matches!(result,Err(rusqlite::Error::SqliteFailure(error,_)) if error.code==rusqlite::ErrorCode::DatabaseBusy);
                if !contender.is_autocommit(){contender.execute_batch("ROLLBACK").unwrap()};observed.borrow_mut().push(blocked);
            }
        }));
    }
    if let Some(shown) = shown {
        let result = owner.apply_current_controlled(
            admit("reservation.knxprod", &bytes),
            &shown,
            &op,
            &mut artifacts,
        );
        assert_eq!(result.commit, CommitEvidence::Committed);
        assert!(result.failure.is_none());
        assert_eq!(&*observed.borrow(), &[true, true]);
    } else {
        let result = owner
            .preview_current_controlled(admit("reservation.knxprod", &bytes), &op, &mut artifacts)
            .unwrap();
        assert_eq!(result.binding.catalogue, before);
        assert_eq!(&*observed.borrow(), &[true]);
        assert_eq!(
            owner.capture_stamp(&PublicationGate::new()).unwrap(),
            before
        );
    }
    contender
        .borrow()
        .execute_batch("BEGIN IMMEDIATE; ROLLBACK")
        .unwrap();
}
#[test]
fn preview_keeps_writer_reserved_through_full_ro_validation() {
    validation_reservation(false)
}
#[test]
fn apply_keeps_writer_reserved_through_both_full_ro_validations() {
    validation_reservation(true)
}

fn recovery_native_close(stage: DestinationStage) {
    let (_dir, mut owner) = migrated_owner();
    let bytes = valid_package();
    let gate = PublicationGate::new();
    let op = OperationContext {
        gate: &gate,
        policy: ControlledPolicy::Standard,
    };
    let shown = owner
        .preview_current_controlled(
            admit("native.knxprod", &bytes),
            &op,
            &mut TestArtifacts::new(),
        )
        .unwrap()
        .binding;
    let statement = Arc::new(Mutex::new(0usize));
    let shared = statement.clone();
    let mut artifacts = TestArtifacts::new();
    artifacts.inventory.set_hook(
        DestinationRole::Recovery,
        Box::new(move |observed, _, conn| {
            if observed == stage {
                let mut stmt = std::ptr::null_mut();
                assert_eq!(
                    unsafe {
                        rusqlite::ffi::sqlite3_prepare_v2(
                            conn.unwrap().handle(),
                            c"SELECT count(*) FROM main.sqlite_schema".as_ptr(),
                            -1,
                            &mut stmt,
                            std::ptr::null_mut(),
                        )
                    },
                    rusqlite::ffi::SQLITE_OK
                );
                *shared.lock().unwrap() = stmt as usize;
            }
        }),
    );
    let result = owner.apply_current_controlled(
        admit("native.knxprod", &bytes),
        &shown,
        &op,
        &mut artifacts,
    );
    let stmt = *statement.lock().unwrap() as *mut rusqlite::ffi::sqlite3_stmt;
    assert!(!stmt.is_null());
    assert_eq!(
        unsafe { rusqlite::ffi::sqlite3_finalize(stmt) },
        rusqlite::ffi::SQLITE_OK
    );
    assert_eq!(result.commit, CommitEvidence::NotCommitted);
    assert_eq!(result.rollback, RollbackDisposition::Explicit);
    assert_eq!(result.health, WriterHealth::Healthy);
    assert_eq!(result.artifacts, ArtifactDisposition::Dispose);
    let failure = match result.failure {
        Some(OrchestrationError::Copy(CopyError::Destination(e))) => e,
        _ => panic!("expected inspected native close refusal"),
    };
    assert!(failure.primary.is_none());
    assert!(
        matches!(&failure.cleanup[..],[DestinationCleanup::WriterClose(rusqlite::Error::SqliteFailure(error,_))]|[DestinationCleanup::ReadClose(rusqlite::Error::SqliteFailure(error,_))] if error.code==rusqlite::ErrorCode::DatabaseBusy)
    );
    assert!(artifacts.inventory.allocated(DestinationRole::Recovery));
    let retained = artifacts
        .inventory
        .retained_connection(DestinationRole::Recovery)
        .expect("B owner retains returned live native connection");
    assert!(retained.is_autocommit());
    assert_eq!(
        retained.is_readonly("main").unwrap(),
        stage == DestinationStage::ReadEnd
    );
    assert_eq!(
        retained
            .query_row("PRAGMA schema_version", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        1
    );
    assert_eq!(package_count(&owner), 0);
}
#[test]
fn recovery_writer_close_failure_is_retained() {
    recovery_native_close(DestinationStage::WriterClose)
}
#[test]
fn recovery_ro_close_failure_is_retained() {
    recovery_native_close(DestinationStage::ReadEnd)
}
