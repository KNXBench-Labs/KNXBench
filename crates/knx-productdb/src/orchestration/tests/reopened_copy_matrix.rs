//! Characterization matrix for the implemented admission gates (not new REDs).
use super::*;
use crate::catalogue_copy::{CopyError, DestinationError, DestinationStage};
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

#[derive(Clone, Copy)]
enum Lane {
    PreviewD,
    ApplyD,
    ApplyB,
}
#[derive(Clone, Copy, Debug)]
enum Corruption {
    Row,
    Rowid,
    SourceChunk,
    PackageChunk,
    SchemaRowid,
    Cookie,
    ApplicationId,
    UnsupportedSchema,
    UnsupportedVersion,
}
fn seed(owner: &mut ManagedCatalogue) {
    owner.with_legacy_mutation(LegacyMutationKind::DirectPackage, |conn| {
        conn.execute("INSERT INTO manufacturer(id,name) VALUES('seed','Original')",[])?;
        conn.execute("INSERT INTO source_file(sha256,source_path,len,bytes) VALUES(?1,'synthetic.xml',200000,?2)",rusqlite::params!["a".repeat(64),vec![0x41u8;200000]])?;
        conn.execute("INSERT INTO package(sha256,source_name,scheme,size,bytes,unknown_count) VALUES(?1,'synthetic.knxprod',11,200000,?2,0)",rusqlite::params!["b".repeat(64),vec![0x42u8;200000]])?;
        conn.execute_batch("PRAGMA application_id=12345; PRAGMA schema_version=4321")?;
        Ok::<_,rusqlite::Error>(())
    }, |_| LegacySettlementHint::SuccessfulReturn).unwrap().unwrap();
}
fn corrupt(conn: &crate::Connection, case: Corruption) {
    match case {
        Corruption::Row => { assert_eq!(conn.execute("UPDATE manufacturer SET name='Corrupted' WHERE id='seed'",[]).unwrap(),1); }
        Corruption::Rowid => { assert_eq!(conn.execute("UPDATE manufacturer SET _rowid_=_rowid_+100 WHERE id='seed'",[]).unwrap(),1); }
        Corruption::SourceChunk | Corruption::PackageChunk => {
            let (table, byte) = if matches!(case,Corruption::SourceChunk) { ("source_file",0x41u8) } else { ("package",0x42u8) };
            let mut bytes=vec![byte;200000]; bytes[100000]^=0xff;
            assert_eq!(conn.execute(&format!("UPDATE {table} SET bytes=?1"),[bytes]).unwrap(),1);
        }
        Corruption::SchemaRowid => { conn.execute_batch("PRAGMA writable_schema=ON").unwrap(); assert_eq!(conn.execute("UPDATE sqlite_schema SET rowid=rowid+1000 WHERE name='sqlite_autoindex_manufacturer_1'",[]).unwrap(),1); conn.execute_batch("PRAGMA writable_schema=OFF").unwrap(); }
        Corruption::Cookie => conn.pragma_update(None,"schema_version",77).unwrap(),
        Corruption::ApplicationId => conn.pragma_update(None,"application_id",9876).unwrap(),
        Corruption::UnsupportedSchema => conn.execute_batch("PRAGMA writable_schema=ON; UPDATE sqlite_schema SET sql=replace(sql,'name TEXT','name BLOB') WHERE name='manufacturer'; PRAGMA writable_schema=OFF").unwrap(),
        Corruption::UnsupportedVersion => conn.pragma_update(None,"user_version",27).unwrap(),
    }
}
fn refusal(lane: Lane, case: Corruption) {
    let (_dir, mut owner) = migrated_owner();
    seed(&mut owner);
    let bytes = valid_package();
    let gate = PublicationGate::new();
    let op = OperationContext {
        gate: &gate,
        policy: ControlledPolicy::Standard,
    };
    let shown = if matches!(lane, Lane::PreviewD) {
        None
    } else {
        Some(
            owner
                .preview_current_controlled(
                    admit("matrix.knxprod", &bytes),
                    &op,
                    &mut TestArtifacts::new(),
                )
                .unwrap()
                .binding,
        )
    };
    let before = owner.capture_stamp(&gate).unwrap();
    let role = if matches!(lane, Lane::ApplyB) {
        DestinationRole::Recovery
    } else {
        DestinationRole::Rehearsal
    };
    let observed = Arc::new(AtomicUsize::new(0));
    let shared = observed.clone();
    let mut artifacts = TestArtifacts::new();
    artifacts.inventory.set_hook(
        role,
        Box::new(move |stage, _, conn| {
            if stage == DestinationStage::WriterClose {
                corrupt(conn.unwrap(), case);
                shared.fetch_add(1, Ordering::SeqCst);
            }
        }),
    );
    let error = if let Some(shown) = shown {
        let result = owner.apply_current_controlled(
            admit("matrix.knxprod", &bytes),
            &shown,
            &op,
            &mut artifacts,
        );
        assert_eq!(result.commit, CommitEvidence::NotCommitted);
        assert_eq!(result.rollback, RollbackDisposition::Explicit);
        assert_eq!(result.health, WriterHealth::Healthy);
        assert_eq!(
            result.artifacts,
            if matches!(lane, Lane::ApplyB) {
                ArtifactDisposition::Dispose
            } else {
                ArtifactDisposition::None
            }
        );
        assert!(result.cleanup.is_empty());
        result
            .failure
            .expect("corrupt copy must refuse before live DML")
    } else {
        match owner.preview_current_controlled(admit("matrix.knxprod", &bytes), &op, &mut artifacts)
        {
            Err(e) => e,
            Ok(_) => panic!("corrupt copy reached rehearsal"),
        }
    };
    assert_eq!(
        observed.load(Ordering::SeqCst),
        1,
        "actual destination corruption must execute"
    );
    let failure = match error {
        OrchestrationError::Copy(CopyError::Destination(e)) => e,
        other => panic!("expected disk-copy refusal, got {other:?}"),
    };
    assert!(failure.cleanup.is_empty());
    match case {
        Corruption::ApplicationId | Corruption::UnsupportedVersion => assert!(matches!(
            failure.primary,
            Some(DestinationError::HeaderFacts)
        )),
        Corruption::UnsupportedSchema => assert!(matches!(
            failure.primary,
            Some(DestinationError::Witness(
                crate::catalogue_schema::WitnessError::Admission(
                    crate::catalogue_schema::AdmissionError::UnsupportedSchemaShape
                )
            ))
        )),
        _ => assert!(
            matches!(failure.primary, Some(DestinationError::ContentMismatch)),
            "{case:?}: {:?}",
            failure.primary
        ),
    }
    let after = owner.capture_stamp(&PublicationGate::new()).unwrap();
    assert_eq!(after.witness, before.witness);
    assert_eq!(after.file, before.file);
    assert_eq!(after.total_changes, before.total_changes);
    assert_eq!(
        after.generation,
        before.generation + u64::from(!matches!(lane, Lane::PreviewD))
    );
    assert_eq!(package_count(&owner), 1);
    assert!(
        artifacts.inventory.retained_connection(role).is_none(),
        "writer and validation reader must be explicitly closed"
    );
}
unsafe extern "C" fn deny_read_end(
    _: *mut std::ffi::c_void,
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
        rusqlite::ffi::SQLITE_DENY
    } else {
        rusqlite::ffi::SQLITE_OK
    }
}
#[test]
fn ro_end_and_native_close_failure_retain_handle_and_original_stop() {
    use crate::catalogue_copy::DestinationCleanup;
    use std::sync::Mutex;
    let (_dir, mut owner) = migrated_owner();
    let bytes = valid_package();
    let gate = Arc::new(PublicationGate::new());
    let stop = gate.clone();
    let statement = Arc::new(Mutex::new(0usize));
    let shared = statement.clone();
    let mut artifacts = TestArtifacts::new();
    artifacts.inventory.set_hook(
        DestinationRole::Rehearsal,
        Box::new(move |stage, _, conn| {
            if stage == DestinationStage::ReadEnd {
                let conn = conn.unwrap();
                assert!(conn.is_readonly("main").unwrap());
                assert!(
                    !conn.is_autocommit(),
                    "validation must use a real pinned transaction"
                );
                let mut stmt = std::ptr::null_mut();
                assert_eq!(
                    unsafe {
                        rusqlite::ffi::sqlite3_prepare_v2(
                            conn.handle(),
                            c"SELECT count(*) FROM main.sqlite_schema".as_ptr(),
                            -1,
                            &mut stmt,
                            std::ptr::null_mut(),
                        )
                    },
                    rusqlite::ffi::SQLITE_OK
                );
                *shared.lock().unwrap() = stmt as usize;
                assert_eq!(
                    unsafe {
                        rusqlite::ffi::sqlite3_set_authorizer(
                            conn.handle(),
                            Some(deny_read_end),
                            std::ptr::null_mut(),
                        )
                    },
                    rusqlite::ffi::SQLITE_OK
                );
                assert_eq!(stop.try_cancel(), crate::operation::CancelOutcome::Accepted);
            }
        }),
    );
    let result = owner.preview_current_controlled(
        admit("native.knxprod", &bytes),
        &OperationContext {
            gate: &gate,
            policy: ControlledPolicy::Standard,
        },
        &mut artifacts,
    );
    let stmt = *statement.lock().unwrap() as *mut rusqlite::ffi::sqlite3_stmt;
    assert!(!stmt.is_null());
    assert_eq!(
        unsafe { rusqlite::ffi::sqlite3_finalize(stmt) },
        rusqlite::ffi::SQLITE_OK
    );
    let retained = artifacts
        .inventory
        .retained_connection(DestinationRole::Rehearsal)
        .expect("inventory owns the failed RO close handle");
    assert!(!retained.is_autocommit());
    assert!(retained.is_readonly("main").unwrap());
    assert_eq!(
        unsafe {
            rusqlite::ffi::sqlite3_set_authorizer(retained.handle(), None, std::ptr::null_mut())
        },
        rusqlite::ffi::SQLITE_OK
    );
    retained.execute_batch("ROLLBACK").unwrap();
    let failure = match result {
        Err(OrchestrationError::Copy(CopyError::Destination(e))) => e,
        _ => panic!("expected typed destination settlement"),
    };
    assert!(
        matches!(failure.primary, Some(DestinationError::Stop(_))),
        "Stop must survive RO end/close errors independently"
    );
    assert!(
        matches!(&failure.cleanup[..],[DestinationCleanup::ReadEnd(_),DestinationCleanup::ReadNotAutocommit,DestinationCleanup::ReadClose(rusqlite::Error::SqliteFailure(error,_))] if error.code==rusqlite::ErrorCode::DatabaseBusy)
    );
    assert_eq!(package_count(&owner), 0);
}

macro_rules! lane {
    ($module:ident,$lane:expr) => {
        mod $module {
            use super::*;
            #[test]
            fn row() {
                refusal($lane, Corruption::Row)
            }
            #[test]
            fn hidden_rowid() {
                refusal($lane, Corruption::Rowid)
            }
            #[test]
            fn source_bytes_beyond_chunk() {
                refusal($lane, Corruption::SourceChunk)
            }
            #[test]
            fn package_bytes_beyond_chunk() {
                refusal($lane, Corruption::PackageChunk)
            }
            #[test]
            fn supported_schema_rowid() {
                refusal($lane, Corruption::SchemaRowid)
            }
            #[test]
            fn copied_cookie() {
                refusal($lane, Corruption::Cookie)
            }
            #[test]
            fn application_id() {
                refusal($lane, Corruption::ApplicationId)
            }
            #[test]
            fn unsupported_schema() {
                refusal($lane, Corruption::UnsupportedSchema)
            }
            #[test]
            fn unsupported_version() {
                refusal($lane, Corruption::UnsupportedVersion)
            }
        }
    };
}
lane!(preview_d, Lane::PreviewD);
lane!(apply_d, Lane::ApplyD);
lane!(apply_b, Lane::ApplyB);

#[test]
fn successful_apply_reopens_both_disks_ro_with_full_witness_and_header_agreement() {
    use crate::catalogue_copy::CopyFacts;
    use crate::catalogue_schema::{copy_content_witness, witness_current};
    let (_dir, mut owner) = migrated_owner();
    seed(&mut owner);
    let gate = PublicationGate::new();
    let bytes = valid_package();
    let op = OperationContext {
        gate: &gate,
        policy: ControlledPolicy::Standard,
    };
    let expected = owner
        .with_read(|conn| {
            let mut tx = conn.unchecked_transaction()?;
            tx.set_drop_behavior(rusqlite::DropBehavior::Ignore);
            let copy = copy_content_witness(&tx, &gate).unwrap();
            let live = witness_current(&tx, &gate).unwrap();
            let facts = CopyFacts::read(&tx).unwrap();
            tx.execute_batch("ROLLBACK")?;
            Ok::<_, rusqlite::Error>((copy, live, facts))
        })
        .unwrap()
        .unwrap();
    let shown = owner
        .preview_current_controlled(
            admit("matrix.knxprod", &bytes),
            &op,
            &mut TestArtifacts::new(),
        )
        .unwrap()
        .binding;
    let mut artifacts = TestArtifacts::new();
    let seen = Arc::new(AtomicUsize::new(0));
    for role in [DestinationRole::Rehearsal, DestinationRole::Recovery] {
        let expected = expected.clone();
        let seen = seen.clone();
        artifacts.inventory.set_hook(
            role,
            Box::new(move |stage, _, conn| {
                if stage == DestinationStage::ReadOpened {
                    let conn = conn.unwrap();
                    assert!(conn.is_readonly("main").unwrap());
                    assert_eq!(
                        conn.query_row("PRAGMA schema_version", [], |r| r.get::<_, i64>(0))
                            .unwrap(),
                        1
                    );
                    let mut tx = conn.unchecked_transaction().unwrap();
                    tx.set_drop_behavior(rusqlite::DropBehavior::Ignore);
                    assert_eq!(
                        copy_content_witness(&tx, &PublicationGate::new()).unwrap(),
                        expected.0,
                        "entire witness includes counts and encoded length"
                    );
                    assert_ne!(
                        witness_current(&tx, &PublicationGate::new()).unwrap(),
                        expected.1,
                        "live cookie identity must remain different"
                    );
                    assert_eq!(CopyFacts::read(&tx).unwrap(), expected.2);
                    tx.execute_batch("ROLLBACK").unwrap();
                    seen.fetch_add(1, Ordering::SeqCst);
                }
            }),
        );
    }
    let result = owner.apply_current_controlled(
        admit("matrix.knxprod", &bytes),
        &shown,
        &op,
        &mut artifacts,
    );
    assert!(result.failure.is_none(), "{:?}", result.failure);
    assert!(result.cleanup.is_empty());
    assert_eq!(result.commit, CommitEvidence::Committed);
    assert_eq!(seen.load(Ordering::SeqCst), 2);
    assert_eq!(result.artifacts, ArtifactDisposition::Retain);
    assert_eq!(package_count(&owner), 2);
    for role in [DestinationRole::Rehearsal, DestinationRole::Recovery] {
        assert!(artifacts.inventory.allocated(role));
        assert!(artifacts.inventory.retained_connection(role).is_none());
    }
}
