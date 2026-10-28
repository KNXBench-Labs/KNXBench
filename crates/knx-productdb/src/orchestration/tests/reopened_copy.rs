//! Real disk destination admission through the actual preview/apply entrypoints.
use super::*;
use crate::catalogue_copy::DestinationStage;

#[test]
fn reopened_copy_refuses_identical_replacement_before_ro_open() {
    let (_dir, mut owner) = migrated_owner();
    let gate = PublicationGate::new();
    let before = owner.capture_stamp(&gate).unwrap();
    let mut artifacts = TestArtifacts::new();
    artifacts.inventory.set_hook(
        DestinationRole::Rehearsal,
        Box::new(|stage, path, _| {
            if stage == DestinationStage::ReadOpen {
                let identical = path.with_extension("identical");
                std::fs::copy(path, &identical).unwrap();
                std::fs::rename(identical, path).unwrap();
            }
        }),
    );
    let bytes = valid_package();
    let result = owner.preview_current_controlled(
        admit("review.knxprod", &bytes),
        &OperationContext {
            gate: &gate,
            policy: ControlledPolicy::Standard,
        },
        &mut artifacts,
    );
    assert!(
        result.is_err(),
        "equal bytes on a replacement inode cannot confer Validated"
    );
    assert_eq!(
        owner.capture_stamp(&PublicationGate::new()).unwrap(),
        before
    );
}

#[test]
fn reopened_copy_refuses_in_place_modification_between_ro_and_rw() {
    let (_dir, mut owner) = migrated_owner();
    let gate = PublicationGate::new();
    let before = owner.capture_stamp(&gate).unwrap();
    let mut artifacts = TestArtifacts::new();
    artifacts.inventory.set_hook(DestinationRole::Rehearsal, Box::new(|stage, path, _| {
        if stage == DestinationStage::WriteOpen {
            let conn = crate::Connection::open(path).unwrap();
            conn.execute("INSERT INTO manufacturer(id,name) VALUES('corrupt','Changed after RO validation')", []).unwrap();
            conn.close().unwrap();
        }
    }));
    let bytes = valid_package();
    let result = owner.preview_current_controlled(
        admit("review.knxprod", &bytes),
        &OperationContext {
            gate: &gate,
            policy: ControlledPolicy::Standard,
        },
        &mut artifacts,
    );
    assert!(
        result.is_err(),
        "in-place modification after validation must invalidate the D capability"
    );
    assert_eq!(
        owner.capture_stamp(&PublicationGate::new()).unwrap(),
        before
    );
}

#[test]
fn reopened_copy_refuses_changed_page_extent_with_equal_copy_content() {
    let (_dir, mut owner) = migrated_owner();
    let gate = PublicationGate::new();
    let mut artifacts = TestArtifacts::new();
    artifacts.inventory.set_hook(
        DestinationRole::Rehearsal,
        Box::new(|stage, _, conn| {
            if stage == DestinationStage::WriterClose {
                let conn = conn.unwrap();
                let pages = conn
                    .query_row("PRAGMA page_count", [], |r| r.get::<_, i64>(0))
                    .unwrap();
                conn.execute(
                    "INSERT INTO manufacturer(id,name) VALUES('extent',?1)",
                    ["x".repeat(300000)],
                )
                .unwrap();
                conn.execute("DELETE FROM manufacturer WHERE id='extent'", [])
                    .unwrap();
                assert!(
                    conn.query_row("PRAGMA page_count", [], |r| r.get::<_, i64>(0))
                        .unwrap()
                        > pages
                );
            }
        }),
    );
    let bytes = valid_package();
    let result = owner.preview_current_controlled(
        admit("review.knxprod", &bytes),
        &OperationContext {
            gate: &gate,
            policy: ControlledPolicy::Standard,
        },
        &mut artifacts,
    );
    assert!(
        result.is_err(),
        "equal content with a different admitted page count is not the copied extent"
    );
    assert_eq!(package_count(&owner), 0);
}

struct MismatchedApplySource {
    inner: TestArtifacts,
    replacement: std::path::PathBuf,
    displaced: std::path::PathBuf,
}
impl CatalogueArtifacts for MismatchedApplySource {
    fn inventory(&mut self) -> &mut ArtifactInventory {
        self.inner.inventory()
    }
    fn before_source_open(&mut self, path: &std::path::Path) {
        std::fs::rename(path, &self.displaced).unwrap();
        std::fs::rename(&self.replacement, path).unwrap();
    }
    fn fresh_rehearsal(&mut self, size: i64) -> Result<FreshDestination, OrchestrationError> {
        self.inner.fresh_rehearsal(size)
    }
    fn fresh_recovery(&mut self, size: i64) -> Result<FreshDestination, OrchestrationError> {
        self.inner.fresh_recovery(size)
    }
}
#[test]
fn reopened_copy_apply_refuses_source_live_witness_mismatch_before_allocation() {
    let (dir, mut owner) = migrated_owner();
    let bytes = valid_package();
    let gate = PublicationGate::new();
    let op = OperationContext {
        gate: &gate,
        policy: ControlledPolicy::Standard,
    };
    let shown = owner
        .preview_current_controlled(
            admit("review.knxprod", &bytes),
            &op,
            &mut TestArtifacts::new(),
        )
        .unwrap()
        .binding;
    let replacement = dir.path().join("other.sqlite");
    std::fs::copy(dir.path().join("products.sqlite"), &replacement).unwrap();
    let other = crate::Connection::open(&replacement).unwrap();
    other.pragma_update(None, "schema_version", 12345).unwrap();
    other.close().unwrap();
    let mut artifacts = MismatchedApplySource {
        inner: TestArtifacts::new(),
        replacement,
        displaced: dir.path().join("original.sqlite"),
    };
    let result = owner.apply_current_controlled(
        admit("review.knxprod", &bytes),
        &shown,
        &op,
        &mut artifacts,
    );
    assert_eq!(
        artifacts.inner.counter, 0,
        "apply must compare full S/W witness before D/B allocation"
    );
    assert!(
        matches!(result.failure, Some(OrchestrationError::Stale)),
        "{:?}",
        result.failure
    );
    assert_eq!(result.commit, CommitEvidence::NotCommitted);
    assert_eq!(result.rollback, RollbackDisposition::Explicit);
}

struct WriterReplacementArtifacts {
    inner: TestArtifacts,
    writer_path: std::path::PathBuf,
}
impl CatalogueArtifacts for WriterReplacementArtifacts {
    fn inventory(&mut self) -> &mut ArtifactInventory {
        self.inner.inventory()
    }
    fn fresh_rehearsal(&mut self, size: i64) -> Result<FreshDestination, OrchestrationError> {
        self.inner.fresh_rehearsal(size)
    }
    fn fresh_recovery(&mut self, size: i64) -> Result<FreshDestination, OrchestrationError> {
        self.inner.fresh_recovery(size)
    }
    fn after_copy(&mut self, recovery: bool, _: &std::path::Path) {
        if recovery {
            let replacement = self.writer_path.with_extension("replacement");
            std::fs::copy(&self.writer_path, &replacement).unwrap();
            std::fs::rename(replacement, &self.writer_path).unwrap();
        }
    }
}
#[test]
fn reopened_copy_apply_rechecks_writer_identity_before_live_dml() {
    let (dir, mut owner) = migrated_owner();
    let bytes = valid_package();
    let gate = PublicationGate::new();
    let op = OperationContext {
        gate: &gate,
        policy: ControlledPolicy::Standard,
    };
    let shown = owner
        .preview_current_controlled(
            admit("review.knxprod", &bytes),
            &op,
            &mut TestArtifacts::new(),
        )
        .unwrap()
        .binding;
    let mut artifacts = WriterReplacementArtifacts {
        inner: TestArtifacts::new(),
        writer_path: dir.path().join("products.sqlite"),
    };
    let result = owner.apply_current_controlled(
        admit("review.knxprod", &bytes),
        &shown,
        &op,
        &mut artifacts,
    );
    assert!(
        matches!(result.failure, Some(OrchestrationError::Stale)),
        "actual W identity failure must precede installer: {:?}",
        result.failure
    );
    assert_eq!(result.commit, CommitEvidence::NotCommitted);
    assert_eq!(result.rollback, RollbackDisposition::Explicit);
    assert_eq!(result.artifacts, ArtifactDisposition::Retain);
    assert!(matches!(result.health, WriterHealth::Quarantined(_)));
}

#[test]
fn reopened_copy_writer_close_retains_native_handle_and_independent_stop() {
    use crate::catalogue_copy::{CopyError, DestinationCleanup, DestinationError};
    use std::sync::{Arc, Mutex};
    let (_dir, mut owner) = migrated_owner();
    let gate = Arc::new(PublicationGate::new());
    let before = owner.capture_stamp(&gate).unwrap();
    let statement = Arc::new(Mutex::new(0usize));
    let mut artifacts = TestArtifacts::new();
    let shared = statement.clone();
    let stop = gate.clone();
    artifacts.inventory.set_hook(
        DestinationRole::Rehearsal,
        Box::new(move |stage, _, conn| {
            if stage == DestinationStage::WriterClose {
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
                assert_eq!(stop.try_cancel(), crate::operation::CancelOutcome::Accepted);
            }
        }),
    );
    let bytes = valid_package();
    let result = owner.preview_current_controlled(
        admit("review.knxprod", &bytes),
        &OperationContext {
            gate: &gate,
            policy: ControlledPolicy::Standard,
        },
        &mut artifacts,
    );
    // Finalize before asserting, so a real RED never abandons a native statement.
    let stmt = *statement.lock().unwrap() as *mut rusqlite::ffi::sqlite3_stmt;
    assert!(!stmt.is_null());
    assert_eq!(
        unsafe { rusqlite::ffi::sqlite3_finalize(stmt) },
        rusqlite::ffi::SQLITE_OK
    );
    let failure = match result {
        Err(OrchestrationError::Copy(CopyError::Destination(failure))) => failure,
        other => panic!("expected independent close failure: {other:?}"),
    };
    assert!(
        matches!(failure.primary, Some(DestinationError::Stop(_))),
        "accepted Stop is independent of native writer close failure: {:?}",
        failure.primary
    );
    assert!(
        matches!(&failure.cleanup[..],[DestinationCleanup::WriterClose(rusqlite::Error::SqliteFailure(error,_))] if error.code == rusqlite::ErrorCode::DatabaseBusy)
    );
    let retained = artifacts
        .inventory
        .retained_connection(DestinationRole::Rehearsal)
        .expect("inventory owns the returned live writer");
    assert!(retained.is_autocommit());
    assert_eq!(
        retained
            .query_row("PRAGMA schema_version", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        1
    );
    assert_eq!(
        owner.capture_stamp(&PublicationGate::new()).unwrap(),
        before
    );
}

#[test]
fn reopened_copy_refuses_trailing_disk_extent_with_equal_page_header_and_content() {
    use std::io::Write;
    let (_dir, mut owner) = migrated_owner();
    let bytes = valid_package();
    let gate = PublicationGate::new();
    let mut artifacts = TestArtifacts::new();
    artifacts.inventory.set_hook(
        DestinationRole::Rehearsal,
        Box::new(|stage, path, conn| {
            if stage == DestinationStage::WriterClose {
                let conn = conn.unwrap();
                let pages: i64 = conn
                    .query_row("PRAGMA page_count", [], |r| r.get(0))
                    .unwrap();
                let size: i64 = conn
                    .query_row("PRAGMA page_size", [], |r| r.get(0))
                    .unwrap();
                assert_eq!(
                    std::fs::metadata(path).unwrap().len(),
                    (pages * size) as u64
                );
                std::fs::OpenOptions::new()
                    .append(true)
                    .open(path)
                    .unwrap()
                    .write_all(&vec![0; size as usize])
                    .unwrap();
                assert_eq!(
                    conn.query_row("PRAGMA page_count", [], |r| r.get::<_, i64>(0))
                        .unwrap(),
                    pages,
                    "native header still agrees despite trailing bytes"
                );
            }
        }),
    );
    let result = owner.preview_current_controlled(
        admit("extent.knxprod", &bytes),
        &OperationContext {
            gate: &gate,
            policy: ControlledPolicy::Standard,
        },
        &mut artifacts,
    );
    assert!(
        result.is_err(),
        "trailing physical extent must not be admitted by equal logical header/content"
    );
    assert_eq!(package_count(&owner), 0);
}

#[test]
fn reopened_copy_preserves_rehearsal_refusal_and_failed_native_rw_close() {
    struct ConsumerCloseArtifacts {
        inner: TestArtifacts,
        statement: *mut rusqlite::ffi::sqlite3_stmt,
    }
    impl CatalogueArtifacts for ConsumerCloseArtifacts {
        fn inventory(&mut self) -> &mut ArtifactInventory {
            self.inner.inventory()
        }
        fn fresh_rehearsal(&mut self, size: i64) -> Result<FreshDestination, OrchestrationError> {
            self.inner.fresh_rehearsal(size)
        }
        fn fresh_recovery(&mut self, _: i64) -> Result<FreshDestination, OrchestrationError> {
            panic!("preview must not allocate B")
        }
        fn before_rehearsal(&mut self, receipt: &crate::catalogue_copy::RehearsalDestination) {
            let handle = unsafe { self.inventory().rehearsal(receipt).unwrap().handle() };
            assert_eq!(
                unsafe {
                    rusqlite::ffi::sqlite3_prepare_v2(
                        handle,
                        c"SELECT count(*) FROM main.sqlite_schema".as_ptr(),
                        -1,
                        &mut self.statement,
                        std::ptr::null_mut(),
                    )
                },
                rusqlite::ffi::SQLITE_OK
            );
        }
    }
    let (_dir, mut owner) = migrated_owner();
    let gate = PublicationGate::new();
    let mut artifacts = ConsumerCloseArtifacts {
        inner: TestArtifacts::new(),
        statement: std::ptr::null_mut(),
    };
    let result = owner.preview_current_controlled(
        admit("invalid.knxprod", b"not a ZIP"),
        &OperationContext {
            gate: &gate,
            policy: ControlledPolicy::Standard,
        },
        &mut artifacts,
    );
    assert!(!artifacts.statement.is_null());
    assert_eq!(
        unsafe { rusqlite::ffi::sqlite3_finalize(artifacts.statement) },
        rusqlite::ffi::SQLITE_OK
    );
    let failure = match result {
        Err(OrchestrationError::PreviewSettlement(e)) => e,
        _ => {
            panic!("original rehearsal refusal must not discard inspected native RW close failure")
        }
    };
    assert!(matches!(
        failure.primary,
        Some(OrchestrationError::Install(_))
    ));
    assert_eq!(failure.cleanup.len(), 1);
    assert!(artifacts
        .inventory()
        .retained_connection(DestinationRole::Rehearsal)
        .is_some());
    assert_eq!(package_count(&owner), 0);
}

#[test]
fn reopened_copy_stop_during_source_end_withholds_consumer_after_writer_cleanup() {
    use std::sync::Arc;
    struct CancelAtSourceEnd {
        inner: TestArtifacts,
        gate: Arc<PublicationGate>,
        consumers: usize,
    }
    unsafe extern "C" fn cancel(
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
            let gate = unsafe { &*context.cast::<PublicationGate>() };
            assert_eq!(gate.try_cancel(), crate::operation::CancelOutcome::Accepted);
        }
        rusqlite::ffi::SQLITE_OK
    }
    impl CatalogueArtifacts for CancelAtSourceEnd {
        fn inventory(&mut self) -> &mut ArtifactInventory {
            self.inner.inventory()
        }
        fn fresh_rehearsal(&mut self, size: i64) -> Result<FreshDestination, OrchestrationError> {
            self.inner.fresh_rehearsal(size)
        }
        fn fresh_recovery(&mut self, _: i64) -> Result<FreshDestination, OrchestrationError> {
            panic!("no B in preview")
        }
        fn inspect_pinned_source(&mut self, conn: &crate::Connection) {
            assert_eq!(
                unsafe {
                    rusqlite::ffi::sqlite3_set_authorizer(
                        conn.handle(),
                        Some(cancel),
                        Arc::as_ptr(&self.gate).cast_mut().cast(),
                    )
                },
                rusqlite::ffi::SQLITE_OK
            );
        }
        fn before_rehearsal(&mut self, _: &crate::catalogue_copy::RehearsalDestination) {
            self.consumers += 1;
        }
    }
    let (_dir, mut owner) = migrated_owner();
    let bytes = valid_package();
    let gate = Arc::new(PublicationGate::new());
    let before = owner.capture_stamp(&gate).unwrap();
    let mut artifacts = CancelAtSourceEnd {
        inner: TestArtifacts::new(),
        gate: gate.clone(),
        consumers: 0,
    };
    let result = owner.preview_current_controlled(
        admit("end.knxprod", &bytes),
        &OperationContext {
            gate: &gate,
            policy: ControlledPolicy::Standard,
        },
        &mut artifacts,
    );
    assert_eq!(
        artifacts.consumers, 0,
        "Stop latched during inspected S end must be rechecked before consumer access"
    );
    assert!(matches!(result, Err(OrchestrationError::Stop(_))));
    assert_eq!(
        owner.capture_stamp(&PublicationGate::new()).unwrap(),
        before
    );
    assert!(
        artifacts
            .inventory()
            .retained_connection(DestinationRole::Rehearsal)
            .is_none(),
        "unconsumed RW handle is explicitly closed after clean W settlement"
    );
}

struct CorruptArtifacts {
    inner: TestArtifacts,
    copied: usize,
}
impl CatalogueArtifacts for CorruptArtifacts {
    fn inventory(&mut self) -> &mut ArtifactInventory {
        self.inner.inventory()
    }
    fn fresh_rehearsal(&mut self, size: i64) -> Result<FreshDestination, OrchestrationError> {
        self.inner.fresh_rehearsal(size)
    }
    fn fresh_recovery(&mut self, size: i64) -> Result<FreshDestination, OrchestrationError> {
        self.inner.fresh_recovery(size)
    }
    fn after_copy(&mut self, recovery: bool, path: &std::path::Path) {
        assert!(!recovery);
        let conn = crate::Connection::open(path).unwrap();
        assert_eq!(
            conn.query_row("PRAGMA schema_version", [], |r| r.get::<_, i64>(0))
                .unwrap(),
            1
        );
        conn.execute(
            "INSERT INTO manufacturer(id,name) VALUES('corrupt','Not the pinned source')",
            [],
        )
        .unwrap();
        conn.close().unwrap();
        self.copied += 1;
    }
}
#[test]
fn reopened_copy_preview_refuses_corrupt_disk_d_before_rehearsal() {
    let (_dir, mut owner) = migrated_owner();
    let gate = PublicationGate::new();
    let before = owner.capture_stamp(&gate).unwrap();
    let bytes = valid_package();
    let mut artifacts = CorruptArtifacts {
        inner: TestArtifacts::new(),
        copied: 0,
    };
    let result = owner.preview_current_controlled(
        admit("review.knxprod", &bytes),
        &OperationContext {
            gate: &gate,
            policy: ControlledPolicy::Standard,
        },
        &mut artifacts,
    );
    assert_eq!(
        artifacts.copied, 1,
        "real copy/corruption boundary must execute"
    );
    assert!(
        result.is_err(),
        "corrupted closed disk D must never reach rehearsal/binding"
    );
    assert_eq!(
        owner.capture_stamp(&PublicationGate::new()).unwrap(),
        before
    );
    assert_eq!(package_count(&owner), 0);
}

#[test]
fn reopened_copy_stop_at_rw_handoff_never_reaches_consumer() {
    use std::sync::Arc;
    let (_dir, mut owner) = migrated_owner();
    owner
        .with_read(|conn| conn.execute_batch("PRAGMA application_id=12345"))
        .unwrap()
        .unwrap();
    let gate = Arc::new(PublicationGate::new());
    let stop = gate.clone();
    let mut artifacts = ConfiguredArtifacts {
        inner: TestArtifacts::new(),
        admitted: 0,
    };
    artifacts.inner.inventory.set_hook(
        DestinationRole::Rehearsal,
        Box::new(move |stage, _, _| {
            if stage == DestinationStage::WriteReady {
                assert_eq!(stop.try_cancel(), crate::operation::CancelOutcome::Accepted);
            }
        }),
    );
    let bytes = valid_package();
    let result = owner.preview_current_controlled(
        admit("review.knxprod", &bytes),
        &OperationContext {
            gate: &gate,
            policy: ControlledPolicy::Standard,
        },
        &mut artifacts,
    );
    assert!(result.is_err());
    assert_eq!(
        artifacts.admitted, 0,
        "latched Stop at validated RW handoff must prevent consumer admission"
    );
    assert_eq!(package_count(&owner), 0);
}

struct ConfiguredArtifacts {
    inner: TestArtifacts,
    admitted: usize,
}
impl CatalogueArtifacts for ConfiguredArtifacts {
    fn inventory(&mut self) -> &mut ArtifactInventory {
        self.inner.inventory()
    }
    fn fresh_rehearsal(&mut self, size: i64) -> Result<FreshDestination, OrchestrationError> {
        let receipt = self.inner.fresh_rehearsal(size)?;
        self.inventory().test_writer(&receipt).execute_batch("PRAGMA trusted_schema=OFF; PRAGMA recursive_triggers=ON; PRAGMA foreign_keys=OFF; PRAGMA ignore_check_constraints=ON")?;
        Ok(receipt)
    }
    fn fresh_recovery(&mut self, size: i64) -> Result<FreshDestination, OrchestrationError> {
        self.inner.fresh_recovery(size)
    }
    fn before_rehearsal(&mut self, receipt: &crate::catalogue_copy::RehearsalDestination) {
        let conn = self.inventory().rehearsal(receipt).unwrap();
        assert!(!conn.is_readonly("main").unwrap());
        for (pragma, expected) in [
            ("foreign_keys", 1),
            ("query_only", 0),
            ("ignore_check_constraints", 0),
            ("writable_schema", 0),
            ("defer_foreign_keys", 0),
            ("trusted_schema", 0),
            ("recursive_triggers", 1),
            ("application_id", 12345),
            ("schema_version", 1),
            ("user_version", crate::migration::CURRENT_PRODUCTDB_VERSION),
        ] {
            assert_eq!(
                conn.query_row(&format!("PRAGMA {pragma}"), [], |r| r.get::<_, i64>(0))
                    .unwrap(),
                expected,
                "reopened D {pragma}"
            );
        }
        self.admitted += 1;
    }
}
#[test]
fn reopened_copy_successful_disk_d_restores_admitted_rw_configuration() {
    let (_dir, mut owner) = migrated_owner();
    owner
        .with_read(|conn| {
            conn.execute_batch("PRAGMA application_id=12345; PRAGMA schema_version=4321")
        })
        .unwrap()
        .unwrap();
    let gate = PublicationGate::new();
    let before = owner.capture_stamp(&gate).unwrap();
    let bytes = valid_package();
    let mut artifacts = ConfiguredArtifacts {
        inner: TestArtifacts::new(),
        admitted: 0,
    };
    let result = owner
        .preview_current_controlled(
            admit("review.knxprod", &bytes),
            &OperationContext {
                gate: &gate,
                policy: ControlledPolicy::Standard,
            },
            &mut artifacts,
        )
        .unwrap();
    assert_eq!(artifacts.admitted, 1);
    assert_eq!(result.binding.catalogue, before);
    assert_eq!(
        owner.capture_stamp(&PublicationGate::new()).unwrap(),
        before
    );
    assert_eq!(package_count(&owner), 0);
}
