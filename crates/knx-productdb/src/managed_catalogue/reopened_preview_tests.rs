//! Native preview reservation/source settlement controls, without live I/O.
use super::*;
use crate::catalogue_copy::{ArtifactInventory, DestinationRole, FreshDestination};
use crate::controlled::{ControlledPolicy, ControlledSource};
use crate::orchestration::{
    CatalogueArtifacts, OperationContext, OrchestrationError, PreviewCleanupError,
};
use std::ffi::{c_char, c_int, c_void, CStr};

struct SourceFaultArtifacts {
    dir: tempfile::TempDir,
    inventory: ArtifactInventory,
    statement: *mut rusqlite::ffi::sqlite3_stmt,
    deny_end: bool,
    end_attempts: Box<usize>,
    stop: Option<std::sync::Arc<PublicationGate>>,
    inject: bool,
    allow_recovery: bool,
}

impl SourceFaultArtifacts {
    fn new(deny_end: bool) -> Self {
        Self {
            dir: tempfile::tempdir().unwrap(),
            inventory: ArtifactInventory::default(),
            statement: std::ptr::null_mut(),
            deny_end,
            end_attempts: Box::new(0),
            stop: None,
            inject: true,
            allow_recovery: false,
        }
    }
}

unsafe extern "C" fn deny_source_end(
    context: *mut c_void,
    action: c_int,
    first: *const c_char,
    _second: *const c_char,
    _db: *const c_char,
    _trigger: *const c_char,
) -> c_int {
    if action == rusqlite::ffi::SQLITE_TRANSACTION && !first.is_null()
        // SAFETY: SQLite supplies a NUL-terminated authorizer argument.
        && unsafe { CStr::from_ptr(first) }.to_bytes() == b"ROLLBACK"
    {
        // SAFETY: the fixture's boxed counter outlives this source connection.
        unsafe {
            *context.cast::<usize>() += 1;
        }
        rusqlite::ffi::SQLITE_DENY
    } else {
        rusqlite::ffi::SQLITE_OK
    }
}

impl CatalogueArtifacts for SourceFaultArtifacts {
    fn inventory(&mut self) -> &mut ArtifactInventory {
        &mut self.inventory
    }
    fn inspect_pinned_source(&mut self, source: &Connection) {
        if !self.inject {
            return;
        }
        // Deliberately retain an actual native statement across explicit close.
        // The fixture finalizes it before the owner and its retained S drop.
        let handle = unsafe { source.handle() };
        if let Some(gate) = &self.stop {
            assert_eq!(gate.try_cancel(), crate::operation::CancelOutcome::Accepted);
        }
        if self.deny_end {
            let context = (&mut *self.end_attempts as *mut usize).cast();
            assert_eq!(
                unsafe {
                    rusqlite::ffi::sqlite3_set_authorizer(handle, Some(deny_source_end), context)
                },
                rusqlite::ffi::SQLITE_OK
            );
        } else {
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
            assert!(!self.statement.is_null());
        }
    }
    fn fresh_rehearsal(&mut self, size: i64) -> Result<FreshDestination, OrchestrationError> {
        Ok(self.inventory.create_new(
            DestinationRole::Rehearsal,
            self.dir.path().join("D.sqlite"),
            size,
        )?)
    }
    fn fresh_recovery(&mut self, size: i64) -> Result<FreshDestination, OrchestrationError> {
        assert!(self.allow_recovery, "preview must never allocate B");
        Ok(self.inventory.create_new(
            DestinationRole::Recovery,
            self.dir.path().join("B.sqlite"),
            size,
        )?)
    }
}

impl Drop for SourceFaultArtifacts {
    fn drop(&mut self) {
        if !self.statement.is_null() {
            assert_eq!(
                unsafe { rusqlite::ffi::sqlite3_finalize(self.statement) },
                rusqlite::ffi::SQLITE_OK
            );
        }
    }
}

struct ReplacedSourceArtifacts {
    replacement: PathBuf,
    displaced: PathBuf,
    allocations: usize,
    artifacts: SourceFaultArtifacts,
}

impl CatalogueArtifacts for ReplacedSourceArtifacts {
    fn inventory(&mut self) -> &mut ArtifactInventory {
        self.artifacts.inventory()
    }
    fn before_source_open(&mut self, path: &Path) {
        std::fs::rename(path, &self.displaced).unwrap();
        std::fs::rename(&self.replacement, path).unwrap();
    }
    fn fresh_rehearsal(&mut self, size: i64) -> Result<FreshDestination, OrchestrationError> {
        self.allocations += 1;
        self.artifacts.fresh_rehearsal(size)
    }
    fn fresh_recovery(&mut self, _size: i64) -> Result<FreshDestination, OrchestrationError> {
        panic!("preview must never allocate B")
    }
}

#[test]
fn reopened_preview_refuses_mismatched_source_live_witness_before_copy() {
    for cookie_only in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("products.sqlite");
        let conn = crate::open_and_migrate(&path).unwrap();
        let replacement = dir.path().join("replacement.sqlite");
        std::fs::copy(&path, &replacement).unwrap();
        let other = Connection::open(&replacement).unwrap();
        if cookie_only {
            other.pragma_update(None, "schema_version", 12345).unwrap();
        } else {
            other
                .execute(
                    "INSERT INTO manufacturer(id,name) VALUES('different','Other snapshot')",
                    [],
                )
                .unwrap();
        }
        other.close().unwrap();
        let mut owner = ManagedCatalogue::adopt_opened(conn, path).unwrap();
        let mut artifacts = ReplacedSourceArtifacts {
            replacement,
            displaced: dir.path().join("displaced.sqlite"),
            allocations: 0,
            artifacts: SourceFaultArtifacts::new(false),
        };
        let gate = PublicationGate::new();
        let source =
            ControlledSource::admit("invalid.knxprod", b"not a ZIP", ControlledPolicy::Standard)
                .unwrap();
        let result = owner.preview_current_controlled(
            source,
            &OperationContext {
                gate: &gate,
                policy: ControlledPolicy::Standard,
            },
            &mut artifacts,
        );
        let failure = match result {
            Err(OrchestrationError::PreviewSettlement(failure)) => failure,
            _ => panic!("replaced source must refuse and independently report W identity change"),
        };
        assert!(
            matches!(failure.primary, Some(OrchestrationError::Stale)),
            "exact W/S live witness mismatch must remain the primary refusal"
        );
        assert!(matches!(
            &failure.cleanup[..],
            [PreviewCleanupError::WriterBaseline]
        ));
        assert_eq!(
            artifacts.allocations, 0,
            "mismatched W/S must refuse before D allocation"
        );
        assert!(owner.conn.as_ref().unwrap().is_autocommit());
        assert_eq!(owner.generation, 0);
        assert!(matches!(owner.health, WriterHealth::Quarantined(_)));
        assert!(
            artifacts.displaced.exists(),
            "identity uncertainty must not delete either file"
        );
    }
}

#[test]
fn reopened_preview_refuses_old_disk_source_without_memory_migration() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("products.sqlite");
    let conn = crate::open_and_migrate(&path).unwrap();
    let replacement = dir.path().join("old.sqlite");
    Connection::open(&replacement).unwrap().close().unwrap();
    let mut owner = ManagedCatalogue::adopt_opened(conn, path.clone()).unwrap();
    let mut artifacts = ReplacedSourceArtifacts {
        replacement,
        displaced: dir.path().join("displaced.sqlite"),
        allocations: 0,
        artifacts: SourceFaultArtifacts::new(false),
    };
    let gate = PublicationGate::new();
    let source =
        ControlledSource::admit("invalid.knxprod", b"not a ZIP", ControlledPolicy::Standard)
            .unwrap();
    let result = owner.preview_current_controlled(
        source,
        &OperationContext {
            gate: &gate,
            policy: ControlledPolicy::Standard,
        },
        &mut artifacts,
    );
    let failure = match result {
        Err(OrchestrationError::PreviewSettlement(failure)) => failure,
        _ => panic!("old S must refuse and independently report W identity change"),
    };
    use crate::catalogue_schema::{AdmissionError, WitnessError};
    use crate::catalogue_stamp::StampError;
    assert!(
        matches!(
            failure.primary,
            Some(OrchestrationError::Stamp(StampError::Witness(
                WitnessError::Admission(AdmissionError::UnsupportedVersion(0))
            )))
        ),
        "controlled S must refuse its actual old version, not migrate a memory substitute"
    );
    assert_eq!(artifacts.allocations, 0);
    assert!(owner.conn.as_ref().unwrap().is_autocommit());
    let actual =
        Connection::open_with_flags(&path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    assert_eq!(
        actual
            .query_row("PRAGMA user_version", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        0
    );
}

#[test]
fn reopened_preview_source_end_failure_does_not_hide_stop_or_skip_writer_cleanup() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("products.sqlite");
    let conn = crate::open_and_migrate(&path).unwrap();
    let mut owner = ManagedCatalogue::adopt_opened(conn, path.clone()).unwrap();
    let before = owner.capture_stamp(&PublicationGate::new()).unwrap();
    let gate = std::sync::Arc::new(PublicationGate::new());
    let mut artifacts = SourceFaultArtifacts::new(true);
    artifacts.stop = Some(gate.clone());
    let source =
        ControlledSource::admit("invalid.knxprod", b"not a ZIP", ControlledPolicy::Standard)
            .unwrap();
    let result = owner.preview_current_controlled(
        source,
        &OperationContext {
            gate: &gate,
            policy: ControlledPolicy::Standard,
        },
        &mut artifacts,
    );
    let failure = match result {
        Err(OrchestrationError::PreviewSettlement(failure)) => failure,
        _ => panic!("source end failure must survive an independent Stop"),
    };
    assert!(matches!(
        failure.primary,
        Some(OrchestrationError::Stamp(
            crate::catalogue_stamp::StampError::Witness(
                crate::catalogue_schema::WitnessError::Stop(_)
            )
        ))
    ));
    assert!(
        matches!(&failure.cleanup[..], [PreviewCleanupError::SourceEnd(rusqlite::Error::SqliteFailure(error, _))] if error.code == rusqlite::ErrorCode::AuthorizationForStatementDenied)
    );
    assert_eq!(
        *artifacts.end_attempts, 1,
        "one explicit source end attempt, even after Stop"
    );
    assert!(
        owner.unresolved_preview_source.is_none(),
        "native close successfully ended S after denied rollback"
    );
    let writer = owner.conn.as_ref().unwrap();
    assert!(
        writer.is_autocommit(),
        "S cleanup failure cannot skip W rollback"
    );
    assert_eq!(writer.total_changes(), before.total_changes);
    assert_eq!(FileStamp::from_path(&path), before.file);
    assert_eq!(owner.generation, before.generation);
    assert_eq!(
        writer
            .query_row("SELECT count(*) FROM package", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        0
    );
}

#[test]
fn reopened_apply_native_source_close_failure_retains_handle_and_refuses_live_dml() {
    use crate::orchestration::{ArtifactDisposition, CommitEvidence, RollbackDisposition};
    use std::io::Write;
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("products.sqlite");
    let conn = crate::open_and_migrate(&path).unwrap();
    let mut owner = ManagedCatalogue::adopt_opened(conn, path).unwrap();
    let mut zip = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    zip.start_file("knx_master.xml", zip::write::SimpleFileOptions::default())
        .unwrap();
    zip.write_all(br#"<KNX xmlns="http://knx.org/xml/project/11"><MasterData><Manufacturers><Manufacturer Id="M-0001" Name="Example"/></Manufacturers></MasterData></KNX>"#).unwrap();
    zip.start_file(
        "M-0001/Hardware.xml",
        zip::write::SimpleFileOptions::default(),
    )
    .unwrap();
    zip.write_all(br#"<KNX xmlns="http://knx.org/xml/project/11"><ManufacturerData><Manufacturer RefId="M-0001"><Hardware><Hardware Id="H-1" Name="Example"/></Hardware></Manufacturer></ManufacturerData></KNX>"#).unwrap();
    let bytes = zip.finish().unwrap().into_inner();
    let gate = PublicationGate::new();
    let op = OperationContext {
        gate: &gate,
        policy: ControlledPolicy::Standard,
    };
    let mut clean = SourceFaultArtifacts::new(false);
    clean.inject = false;
    let shown = owner
        .preview_current_controlled(
            ControlledSource::admit("source.knxprod", &bytes, op.policy).unwrap(),
            &op,
            &mut clean,
        )
        .unwrap()
        .binding;
    let mut faults = SourceFaultArtifacts::new(false);
    faults.allow_recovery = true;
    let result = owner.apply_current_controlled(
        ControlledSource::admit("source.knxprod", &bytes, op.policy).unwrap(),
        &shown,
        &op,
        &mut faults,
    );
    assert_eq!(
        result.commit,
        CommitEvidence::NotCommitted,
        "S explicit close failure must withhold live COMMIT"
    );
    assert_eq!(result.rollback, RollbackDisposition::Explicit);
    assert_eq!(result.artifacts, ArtifactDisposition::Retain);
    assert!(result.failure.is_some());
    let retained = owner
        .unresolved_preview_source
        .as_ref()
        .expect("apply S close must retain the actual live handle");
    assert_eq!(retained.stamp, shown.catalogue);
    assert!(retained.connection.is_autocommit());
    assert!(retained.connection.is_readonly("main").unwrap());
    assert!(owner.conn.as_ref().unwrap().is_autocommit());
    assert_eq!(
        owner
            .conn
            .as_ref()
            .unwrap()
            .query_row("SELECT count(*) FROM package", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        0
    );
}

#[test]
fn reopened_preview_native_source_close_failure_retains_live_handle() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("products.sqlite");
    let conn = crate::open_and_migrate(&path).unwrap();
    let mut owner = ManagedCatalogue::adopt_opened(conn, path).unwrap();
    let gate = PublicationGate::new();
    let before = owner.capture_stamp(&gate).unwrap();
    let mut artifacts = SourceFaultArtifacts::new(false);
    // Parsing is after the copy/source-settlement gate; it must not be reached.
    let source =
        ControlledSource::admit("invalid.knxprod", b"not a ZIP", ControlledPolicy::Standard)
            .unwrap();
    let result = owner.preview_current_controlled(
        source,
        &OperationContext {
            gate: &gate,
            policy: ControlledPolicy::Standard,
        },
        &mut artifacts,
    );
    let failure = match result {
        Err(OrchestrationError::PreviewSettlement(failure)) => failure,
        _ => panic!("native S close failure must refuse before the rehearsal consumer"),
    };
    assert!(failure.primary.is_none());
    assert!(
        matches!(&failure.cleanup[..], [PreviewCleanupError::SourceClose(rusqlite::Error::SqliteFailure(error, _))] if error.code == rusqlite::ErrorCode::DatabaseBusy)
    );
    let unresolved = owner
        .unresolved_preview_source
        .as_ref()
        .expect("owner must retain the returned live S");
    assert_eq!(
        unresolved.stamp, before,
        "retained handle keeps the original acquisition receipt"
    );
    let retained = &unresolved.connection;
    assert!(
        retained.is_autocommit(),
        "S rollback must have succeeded before failed close"
    );
    assert_eq!(
        retained
            .query_row("PRAGMA user_version", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        crate::migration::CURRENT_PRODUCTDB_VERSION
    );
    assert!(
        owner.conn.as_ref().unwrap().is_autocommit(),
        "W must settle even after S close failure"
    );
    assert_eq!(owner.generation, before.generation);
    assert!(matches!(owner.health, WriterHealth::Quarantined(_)));
}

fn observe_destination_stages(
    artifacts: &mut SourceFaultArtifacts,
) -> std::rc::Rc<std::cell::RefCell<Vec<crate::catalogue_copy::DestinationStage>>> {
    let stages = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
    let observed = stages.clone();
    artifacts.inventory.set_hook(
        DestinationRole::Rehearsal,
        Box::new(move |stage, _, _| observed.borrow_mut().push(stage)),
    );
    stages
}

#[test]
fn orch1_preview_source_close_refusal_never_opens_rehearsal_rw() {
    use crate::catalogue_copy::DestinationStage;
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("products.sqlite");
    let conn = crate::open_and_migrate(&path).unwrap();
    let mut owner = ManagedCatalogue::adopt_opened(conn, path.clone()).unwrap();
    let gate = PublicationGate::new();
    let before = owner.capture_stamp(&gate).unwrap();
    let mut artifacts = SourceFaultArtifacts::new(false);
    let stages = observe_destination_stages(&mut artifacts);
    let result = owner.preview_current_controlled(
        ControlledSource::admit("invalid.knxprod", b"not a ZIP", ControlledPolicy::Standard)
            .unwrap(),
        &OperationContext {
            gate: &gate,
            policy: ControlledPolicy::Standard,
        },
        &mut artifacts,
    );
    // Finalize the native fault before assertions; the owner still retains S.
    assert!(!artifacts.statement.is_null());
    let statement = std::mem::replace(&mut artifacts.statement, std::ptr::null_mut());
    assert_eq!(
        unsafe { rusqlite::ffi::sqlite3_finalize(statement) },
        rusqlite::ffi::SQLITE_OK
    );
    let failure = match result {
        Err(OrchestrationError::PreviewSettlement(failure)) => failure,
        _ => panic!("native S close refusal must prevent rehearsal"),
    };
    assert!(failure.primary.is_none());
    assert!(
        matches!(&failure.cleanup[..], [PreviewCleanupError::SourceClose(rusqlite::Error::SqliteFailure(error, _))] if error.code == rusqlite::ErrorCode::DatabaseBusy)
    );
    let retained = owner.unresolved_preview_source.as_ref().unwrap();
    assert_eq!(retained.stamp, before);
    assert!(retained.connection.is_readonly("main").unwrap());
    assert!(retained.connection.is_autocommit());
    assert!(owner.conn.as_ref().unwrap().is_autocommit());
    assert_eq!(
        owner.conn.as_ref().unwrap().total_changes(),
        before.total_changes
    );
    assert_eq!(FileStamp::from_path(&path), before.file);
    assert_eq!(owner.generation, before.generation);
    assert!(matches!(owner.health, WriterHealth::Quarantined(_)));
    assert!(artifacts.inventory.allocated(DestinationRole::Rehearsal));
    assert!(!artifacts.inventory.allocated(DestinationRole::Recovery));
    assert!(stages.borrow().contains(&DestinationStage::ReadEnd));
    assert!(
        artifacts
            .inventory
            .retained_connection(DestinationRole::Rehearsal)
            .is_none(),
        "S close refusal must leave validated D closed, not strand an RW handle"
    );
    assert!(
        !stages.borrow().contains(&DestinationStage::WriteOpen),
        "S close refusal must precede even the rehearsal RW open attempt"
    );
}

#[test]
fn orch1_preview_writer_rollback_refusal_never_opens_rehearsal_rw() {
    use crate::catalogue_copy::DestinationStage;
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("products.sqlite");
    let conn = crate::open_and_migrate(&path).unwrap();
    let mut owner = ManagedCatalogue::adopt_opened(conn, path.clone()).unwrap();
    let gate = PublicationGate::new();
    let before = owner.capture_stamp(&gate).unwrap();
    let mut artifacts = SourceFaultArtifacts::new(false);
    artifacts.inject = false;
    let stages = observe_destination_stages(&mut artifacts);
    let mut attempts = Box::new(0usize);
    let handle = unsafe { owner.conn.as_ref().unwrap().handle() };
    assert_eq!(
        unsafe {
            rusqlite::ffi::sqlite3_set_authorizer(
                handle,
                Some(deny_source_end),
                (&mut *attempts as *mut usize).cast(),
            )
        },
        rusqlite::ffi::SQLITE_OK
    );
    let result = owner.preview_current_controlled(
        ControlledSource::admit("invalid.knxprod", b"not a ZIP", ControlledPolicy::Standard)
            .unwrap(),
        &OperationContext {
            gate: &gate,
            policy: ControlledPolicy::Standard,
        },
        &mut artifacts,
    );
    let writer_was_active = !owner.conn.as_ref().unwrap().is_autocommit();
    // Restore and explicitly settle W before assertions, even on original RED.
    assert_eq!(
        unsafe { rusqlite::ffi::sqlite3_set_authorizer(handle, None, std::ptr::null_mut()) },
        rusqlite::ffi::SQLITE_OK
    );
    owner
        .conn
        .as_ref()
        .unwrap()
        .execute_batch("ROLLBACK")
        .unwrap();
    let failure = match result {
        Err(OrchestrationError::PreviewSettlement(failure)) => failure,
        _ => panic!("native W rollback refusal must prevent rehearsal"),
    };
    assert!(failure.primary.is_none());
    assert!(
        matches!(&failure.cleanup[..], [PreviewCleanupError::WriterRollback(rusqlite::Error::SqliteFailure(error, _)), PreviewCleanupError::WriterNotAutocommit] if error.code == rusqlite::ErrorCode::AuthorizationForStatementDenied)
    );
    assert_eq!(*attempts, 1, "one inspected native W rollback attempt");
    assert!(
        writer_was_active,
        "failed rollback must not invent autocommit"
    );
    assert!(owner.unresolved_preview_source.is_none());
    assert_eq!(
        owner.conn.as_ref().unwrap().total_changes(),
        before.total_changes
    );
    assert_eq!(FileStamp::from_path(&path), before.file);
    assert_eq!(owner.generation, before.generation);
    assert!(matches!(owner.health, WriterHealth::Quarantined(_)));
    assert!(artifacts.inventory.allocated(DestinationRole::Rehearsal));
    assert!(!artifacts.inventory.allocated(DestinationRole::Recovery));
    assert!(stages.borrow().contains(&DestinationStage::ReadEnd));
    assert!(
        artifacts
            .inventory
            .retained_connection(DestinationRole::Rehearsal)
            .is_none(),
        "W rollback refusal must leave validated D closed, not strand an RW handle"
    );
    assert!(
        !stages.borrow().contains(&DestinationStage::WriteOpen),
        "W rollback refusal must precede even the rehearsal RW open attempt"
    );
}
