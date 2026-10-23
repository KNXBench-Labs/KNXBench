//! Native ordering and settlement controls for apply's second D rehearsal.

use super::*;
use crate::catalogue_copy::DestinationStage;
use crate::operation::PublicationState;
use std::cell::{Cell, RefCell};
use std::ffi::{c_char, c_int, c_void, CStr};
use std::rc::Rc;
use std::sync::Arc;

struct Native {
    db: Cell<*mut rusqlite::ffi::sqlite3>,
    role: &'static str,
    events: Rc<RefCell<Vec<&'static str>>>,
    transactions: RefCell<Vec<String>>,
    dml: Cell<usize>,
    package_inserts: Cell<usize>,
    deny_package: bool,
    denials: Cell<usize>,
    deny_rollback: Cell<bool>,
    cancel_gate: RefCell<Option<Arc<PublicationGate>>>,
    cancellation: Cell<Option<crate::operation::CancelOutcome>>,
}
impl Native {
    fn new(role: &'static str, events: Rc<RefCell<Vec<&'static str>>>, deny: bool) -> Box<Self> {
        Box::new(Self {
            db: Cell::new(std::ptr::null_mut()),
            role,
            events,
            transactions: RefCell::new(Vec::new()),
            dml: Cell::new(0),
            package_inserts: Cell::new(0),
            deny_package: deny,
            denials: Cell::new(0),
            cancel_gate: RefCell::new(None),
            cancellation: Cell::new(None),
            deny_rollback: Cell::new(false),
        })
    }
    fn attach(&self, db: *mut rusqlite::ffi::sqlite3) {
        self.db.set(db);
        // SAFETY: Box keeps the pointee stable; fixture detaches callbacks before
        // closing the native connection or releasing its callback state.
        unsafe {
            assert_eq!(
                rusqlite::ffi::sqlite3_set_authorizer(
                    db,
                    Some(authorize),
                    (self as *const Self).cast_mut().cast()
                ),
                rusqlite::ffi::SQLITE_OK
            );
            rusqlite::ffi::sqlite3_update_hook(
                db,
                Some(updated),
                (self as *const Self).cast_mut().cast(),
            );
        }
    }
    fn detach(&self) {
        let db = self.db.replace(std::ptr::null_mut());
        if !db.is_null() {
            unsafe {
                rusqlite::ffi::sqlite3_set_authorizer(db, None, std::ptr::null_mut());
                rusqlite::ffi::sqlite3_update_hook(db, None, std::ptr::null_mut());
            }
        }
    }
}
impl Drop for Native {
    fn drop(&mut self) {
        self.detach();
    }
}
unsafe extern "C" fn authorize(
    context: *mut c_void,
    action: c_int,
    first: *const c_char,
    _: *const c_char,
    _: *const c_char,
    _: *const c_char,
) -> c_int {
    let state = unsafe { &*context.cast::<Native>() };
    let first = if first.is_null() {
        b"".as_slice()
    } else {
        unsafe { CStr::from_ptr(first) }.to_bytes()
    };
    if action == rusqlite::ffi::SQLITE_TRANSACTION {
        state
            .transactions
            .borrow_mut()
            .push(String::from_utf8_lossy(first).into_owned());
        if state.role == "W" && first == b"ROLLBACK" {
            state.events.borrow_mut().push("W-rollback");
            if state.deny_rollback.get() {
                return rusqlite::ffi::SQLITE_DENY;
            }
        }
    }
    if matches!(
        action,
        rusqlite::ffi::SQLITE_INSERT | rusqlite::ffi::SQLITE_UPDATE | rusqlite::ffi::SQLITE_DELETE
    ) {
        state.dml.set(state.dml.get() + 1);
        if action == rusqlite::ffi::SQLITE_INSERT && first == b"package" && state.deny_package {
            state.denials.set(state.denials.get() + 1);
            return rusqlite::ffi::SQLITE_DENY;
        }
    }
    rusqlite::ffi::SQLITE_OK
}
unsafe extern "C" fn updated(
    context: *mut c_void,
    action: c_int,
    _: *const c_char,
    table: *const c_char,
    _: i64,
) {
    let state = unsafe { &*context.cast::<Native>() };
    state.events.borrow_mut().push(state.role);
    if action == rusqlite::ffi::SQLITE_INSERT
        && unsafe { CStr::from_ptr(table) }.to_bytes() == b"package"
    {
        state.package_inserts.set(state.package_inserts.get() + 1);
        if let Some(gate) = state.cancel_gate.borrow().as_ref() {
            state.cancellation.set(Some(gate.try_cancel()));
        }
    }
}

type PreliveObservation = (u64, bool, PublicationState, bool);

struct Probe {
    inner: TestArtifacts,
    w: Box<Native>,
    d: Box<Native>,
    closes: Rc<Cell<usize>>,
    close_rows: Rc<RefCell<Vec<(i64, i64)>>>,
    prelive: Rc<RefCell<Vec<PreliveObservation>>>,
    before: u64,
    busy_close: Rc<Cell<bool>>,
    statement: Rc<Cell<*mut rusqlite::ffi::sqlite3_stmt>>,
}
impl Probe {
    fn new(
        owner: &ManagedCatalogue,
        path: &std::path::Path,
        gate: Arc<PublicationGate>,
        deny: bool,
    ) -> Self {
        let events = Rc::new(RefCell::new(Vec::new()));
        let w = Native::new("W", events.clone(), false);
        let d = Native::new("D", events, deny);
        let db = owner
            .with_read(|c| Ok::<_, rusqlite::Error>(unsafe { c.handle() }))
            .unwrap()
            .unwrap();
        let before = unsafe { rusqlite::ffi::sqlite3_total_changes64(db) } as u64;
        w.attach(db);
        let contender = crate::Connection::open(path).unwrap();
        contender.busy_timeout(std::time::Duration::ZERO).unwrap();
        let closes = Rc::new(Cell::new(0));
        let close_rows = Rc::new(RefCell::new(Vec::new()));
        let prelive = Rc::new(RefCell::new(Vec::new()));
        let mut inner = TestArtifacts::new();
        let busy_close = Rc::new(Cell::new(false));
        let statement = Rc::new(Cell::new(std::ptr::null_mut()));
        let (busy, stmt) = (busy_close.clone(), statement.clone());
        let d_context = (&*d) as *const Native;
        let (cc, rr, pp) = (closes.clone(), close_rows.clone(), prelive.clone());
        inner.inventory.set_hook(DestinationRole::Rehearsal, Box::new(move |stage, _, conn| {
            // SAFETY: Probe owns the boxed D context until after hook and native
            // callback detachment; moving Probe never moves this allocation.
            let dd = unsafe { &*d_context };
            if stage == DestinationStage::WriteReady { dd.attach(unsafe { conn.unwrap().handle() }); }
            if stage == DestinationStage::RehearsalClose {
                let conn = conn.unwrap();
                cc.set(cc.get()+1);
                rr.borrow_mut().push((
                    conn.query_row("SELECT count(*) FROM package", [], |r| r.get(0)).unwrap(),
                    conn.query_row("SELECT count(*) FROM package_source_name", [], |r| r.get(0)).unwrap(),
                ));
                let blocked = matches!(contender.execute_batch("BEGIN IMMEDIATE"), Err(rusqlite::Error::SqliteFailure(e,_)) if e.code == rusqlite::ErrorCode::DatabaseBusy);
                if !contender.is_autocommit() { contender.execute_batch("ROLLBACK").unwrap(); }
                pp.borrow_mut().push((unsafe { rusqlite::ffi::sqlite3_total_changes64(db) } as u64, unsafe { rusqlite::ffi::sqlite3_get_autocommit(db) } == 0, gate.state(), blocked));
                if busy.get() {
                    let mut raw = std::ptr::null_mut();
                    assert_eq!(unsafe { rusqlite::ffi::sqlite3_prepare_v2(conn.handle(), c"SELECT count(*) FROM package".as_ptr(), -1, &mut raw, std::ptr::null_mut()) }, rusqlite::ffi::SQLITE_OK);
                    stmt.set(raw);
                }
                dd.events.borrow_mut().push("D-close");
                dd.detach();
            }
        }));
        Self {
            inner,
            w,
            d,
            closes,
            close_rows,
            prelive,
            before,
            busy_close,
            statement,
        }
    }
    fn assert_no_w_dml(&self) {
        assert_eq!(
            self.w.dml.get(),
            0,
            "D refusal must precede any W DML authorization"
        );
        assert_eq!(
            unsafe { rusqlite::ffi::sqlite3_total_changes64(self.w.db.get()) } as u64,
            self.before,
            "rollback cannot erase W total_changes"
        );
        assert!(!self.w.events.borrow().contains(&"W"));
    }
    fn assert_closed(&mut self) {
        assert_eq!(
            self.closes.get(),
            1,
            "apply must explicitly close its obtained D receipt"
        );
        assert!(self
            .inner
            .inventory
            .retained_connection(DestinationRole::Rehearsal)
            .is_none());
        let events = self.w.events.borrow();
        if let Some(rollback) = events.iter().position(|e| *e == "W-rollback") {
            assert!(events.iter().position(|e| *e == "D-close").unwrap() < rollback);
        }
    }
}
impl Drop for Probe {
    fn drop(&mut self) {
        let stmt = self.statement.replace(std::ptr::null_mut());
        if !stmt.is_null() {
            // Even assertion unwinds finalize before inventory/connection Drop.
            unsafe {
                rusqlite::ffi::sqlite3_finalize(stmt);
            }
        }
        self.w.detach();
        self.d.detach();
    }
}
impl CatalogueArtifacts for Probe {
    fn inventory(&mut self) -> &mut ArtifactInventory {
        self.inner.inventory()
    }
    fn fresh_rehearsal(&mut self, size: i64) -> Result<FreshDestination, OrchestrationError> {
        self.inner.fresh_rehearsal(size)
    }
    fn fresh_recovery(&mut self, size: i64) -> Result<FreshDestination, OrchestrationError> {
        self.inner.fresh_recovery(size)
    }
}
fn shown(owner: &mut ManagedCatalogue, bytes: &[u8]) -> ControlledPreview {
    owner
        .preview_current_controlled(
            admit("second.knxprod", bytes),
            &OperationContext {
                gate: &PublicationGate::new(),
                policy: ControlledPolicy::Standard,
            },
            &mut TestArtifacts::new(),
        )
        .unwrap()
}
fn apply(
    owner: &mut ManagedCatalogue,
    bytes: &[u8],
    shown: &PreviewBinding,
    gate: &PublicationGate,
    probe: &mut Probe,
) -> ControlledApplySettlement {
    owner.apply_current_controlled(
        admit("second.knxprod", bytes),
        shown,
        &OperationContext {
            gate,
            policy: ControlledPolicy::Standard,
        },
        probe,
    )
}
fn clean_refusal(result: &ControlledApplySettlement) {
    assert_eq!(result.commit, CommitEvidence::NotCommitted);
    assert_eq!(result.rollback, RollbackDisposition::Explicit);
    assert_eq!(result.health, WriterHealth::Healthy);
    assert_eq!(result.artifacts, ArtifactDisposition::Dispose);
    assert!(result.cleanup.is_empty(), "{:?}", result.cleanup);
}

#[test]
fn second_rehearsal_executes_shared_body_before_live_dml() {
    let (dir, mut owner) = migrated_owner();
    let bytes = valid_package();
    let shown = shown(&mut owner, &bytes);
    let expected_copy = owner
        .with_read(|c| {
            let tx = c.unchecked_transaction().unwrap();
            let witness =
                crate::catalogue_schema::copy_content_witness(&tx, &PublicationGate::new())
                    .unwrap();
            tx.rollback().unwrap();
            Ok::<_, rusqlite::Error>(witness)
        })
        .unwrap()
        .unwrap();
    let gate = Arc::new(PublicationGate::new());
    let mut probe = Probe::new(
        &owner,
        &dir.path().join("products.sqlite"),
        gate.clone(),
        false,
    );
    let result = apply(&mut owner, &bytes, &shown.binding, &gate, &mut probe);
    assert_eq!(
        probe.d.package_inserts.get(),
        1,
        "apply must execute actual D installer before W"
    );
    assert_eq!(&*probe.close_rows.borrow(), &[(1, 1)]);
    assert_eq!(
        &*probe.prelive.borrow(),
        &[(probe.before, true, PublicationState::Open, true)]
    );
    let events = probe.w.events.borrow();
    let closed = events.iter().position(|e| *e == "D-close").unwrap();
    let live = events.iter().position(|e| *e == "W").unwrap();
    assert!(events[..closed].contains(&"D"));
    assert!(closed < live);
    drop(events);
    probe.assert_closed();
    assert!(result.failure.is_none(), "{:?}", result.failure);
    assert_eq!(result.commit, CommitEvidence::Committed);
    assert_eq!(result.health, WriterHealth::Healthy);
    assert_eq!(result.artifacts, ArtifactDisposition::Retain);
    assert_eq!(&*probe.w.transactions.borrow(), &["BEGIN", "COMMIT"]);
    assert_eq!(probe.w.package_inserts.get(), 1);
    assert_eq!(package_count(&owner), 1);
    assert_eq!(
        owner
            .capture_stamp(&PublicationGate::new())
            .unwrap()
            .generation,
        shown.binding.catalogue.generation + 1
    );
    let b = crate::Connection::open_with_flags(
        probe.inner.dir.path().join("dest-1.sqlite"),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    let btx = b.unchecked_transaction().unwrap();
    assert_eq!(
        crate::catalogue_schema::copy_content_witness(&btx, &PublicationGate::new()).unwrap(),
        expected_copy
    );
    btx.rollback().unwrap();
    assert_eq!(
        b.query_row("SELECT count(*) FROM package", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        0
    );
    b.close().unwrap();
}

#[test]
fn second_rehearsal_d_installer_refusal_prevents_live_dml() {
    let (dir, mut owner) = migrated_owner();
    let bytes = valid_package();
    let shown = shown(&mut owner, &bytes);
    let gate = Arc::new(PublicationGate::new());
    let mut probe = Probe::new(
        &owner,
        &dir.path().join("products.sqlite"),
        gate.clone(),
        true,
    );
    let result = apply(&mut owner, &bytes, &shown.binding, &gate, &mut probe);
    assert_eq!(
        probe.d.denials.get(),
        1,
        "actual D INSERT INTO package must reach native authorizer"
    );
    assert!(
        matches!(&result.failure, Some(OrchestrationError::Install(crate::PackageError::Database(crate::ProductDbError::Sqlite(rusqlite::Error::SqliteFailure(e,_))))) if e.code == rusqlite::ErrorCode::AuthorizationForStatementDenied),
        "{:?}",
        result.failure
    );
    probe.assert_no_w_dml();
    probe.assert_closed();
    clean_refusal(&result);
    assert_eq!(gate.state(), PublicationState::Open);
    assert_eq!(&*probe.w.transactions.borrow(), &["BEGIN", "ROLLBACK"]);
    assert_eq!(package_count(&owner), 0);
}

#[test]
fn second_rehearsal_complete_preview_mismatch_precedes_live_dml() {
    for field in [
        "version",
        "sha256",
        "package_count",
        "other_member_count",
        "encoded_bytes",
        "raw_source_name",
    ] {
        let (dir, mut owner) = migrated_owner();
        let bytes = valid_package();
        let mut shown = shown(&mut owner, &bytes);
        let original = shown.binding.preview;
        match field {
            "version" => shown.binding.preview.version += 1,
            "sha256" => shown.binding.preview.sha256[0] ^= 1,
            "package_count" => shown.binding.preview.package_count += 1,
            "other_member_count" => shown.binding.preview.other_member_count += 1,
            "encoded_bytes" => shown.binding.preview.encoded_bytes += 1,
            "raw_source_name" => {
                shown.preview.install.packages[0].report.source_names[0] = "change.knxprod".into();
                shown.binding.preview =
                    crate::preview_identity::preview_identity(&shown.preview.install).unwrap();
                assert_eq!(shown.binding.preview.package_count, original.package_count);
                assert_eq!(
                    shown.binding.preview.other_member_count,
                    original.other_member_count
                );
                assert_eq!(shown.binding.preview.encoded_bytes, original.encoded_bytes);
                assert_ne!(shown.binding.preview.sha256, original.sha256);
            }
            _ => unreachable!(),
        }
        let gate = Arc::new(PublicationGate::new());
        let mut probe = Probe::new(
            &owner,
            &dir.path().join("products.sqlite"),
            gate.clone(),
            false,
        );
        let result = apply(&mut owner, &bytes, &shown.binding, &gate, &mut probe);
        probe.assert_no_w_dml();
        assert_eq!(
            probe.d.package_inserts.get(),
            1,
            "{field}: compare the actual D result"
        );
        assert!(
            matches!(result.failure, Some(OrchestrationError::Stale)),
            "{field}: {:?}",
            result.failure
        );
        probe.assert_closed();
        clean_refusal(&result);
        assert_eq!(gate.state(), PublicationState::Open);
        assert_eq!(&*probe.w.transactions.borrow(), &["BEGIN", "ROLLBACK"]);
        assert_eq!(
            owner
                .capture_stamp(&PublicationGate::new())
                .unwrap()
                .generation,
            shown.binding.catalogue.generation + 1
        );
        assert_eq!(package_count(&owner), 0);
    }
}

#[test]
fn second_rehearsal_cancel_during_d_install_closes_before_rollback() {
    let (dir, mut owner) = migrated_owner();
    let bytes = valid_package();
    let shown = shown(&mut owner, &bytes);
    let gate = Arc::new(PublicationGate::new());
    let mut probe = Probe::new(
        &owner,
        &dir.path().join("products.sqlite"),
        gate.clone(),
        false,
    );
    *probe.d.cancel_gate.borrow_mut() = Some(gate.clone());
    let result = apply(&mut owner, &bytes, &shown.binding, &gate, &mut probe);
    assert_eq!(
        probe.d.package_inserts.get(),
        1,
        "cancellation must follow an actual partial D install"
    );
    assert_eq!(
        probe.d.cancellation.get(),
        Some(crate::operation::CancelOutcome::Accepted)
    );
    assert!(
        matches!(
            result.failure,
            Some(OrchestrationError::Install(crate::PackageError::Cancelled(
                crate::operation::Stop::CancelAccepted
            )))
        ),
        "{:?}",
        result.failure
    );
    probe.assert_no_w_dml();
    probe.assert_closed();
    clean_refusal(&result);
    assert_eq!(&*probe.w.transactions.borrow(), &["BEGIN", "ROLLBACK"]);
    assert_eq!(
        &*probe.prelive.borrow(),
        &[(probe.before, true, PublicationState::CancelAccepted, true)]
    );
    assert_eq!(gate.state(), PublicationState::CancelAccepted);
    assert_eq!(package_count(&owner), 0);
}

fn assert_native_busy(error: &crate::catalogue_copy::CopyError) {
    let crate::catalogue_copy::CopyError::Destination(failure) = error else {
        panic!("native destination close required")
    };
    assert!(failure.primary.is_none());
    assert!(
        matches!(&failure.cleanup[..], [crate::catalogue_copy::DestinationCleanup::RehearsalClose(rusqlite::Error::SqliteFailure(e, _))] if e.code == rusqlite::ErrorCode::DatabaseBusy && e.extended_code == rusqlite::ffi::SQLITE_BUSY)
    );
}
#[test]
fn second_rehearsal_native_close_failure_preserves_primary() {
    for primary in ["matching", "mismatch", "cancel"] {
        let (dir, mut owner) = migrated_owner();
        let bytes = valid_package();
        let mut shown = shown(&mut owner, &bytes);
        if primary == "mismatch" {
            shown.binding.preview.encoded_bytes += 1;
        }
        let gate = Arc::new(PublicationGate::new());
        let mut probe = Probe::new(
            &owner,
            &dir.path().join("products.sqlite"),
            gate.clone(),
            false,
        );
        probe.busy_close.set(true);
        if primary == "cancel" {
            *probe.d.cancel_gate.borrow_mut() = Some(gate.clone());
        }
        let result = apply(&mut owner, &bytes, &shown.binding, &gate, &mut probe);
        assert_eq!(
            probe.closes.get(),
            1,
            "{primary}: apply must attempt native D close"
        );
        assert!(
            !probe.statement.get().is_null(),
            "real native statement must prevent close"
        );
        assert_eq!(probe.d.package_inserts.get(), 1);
        probe.assert_no_w_dml();
        clean_refusal(&result);
        match (&result.failure, primary) {
            (Some(OrchestrationError::Copy(e)), "matching") => assert_native_busy(e),
            (Some(OrchestrationError::PreviewSettlement(f)), _) => {
                if primary == "mismatch" {
                    assert!(matches!(f.primary, Some(OrchestrationError::Stale)));
                } else {
                    assert!(matches!(
                        f.primary,
                        Some(OrchestrationError::Install(crate::PackageError::Cancelled(
                            crate::operation::Stop::CancelAccepted
                        )))
                    ));
                }
                match &f.cleanup[..] {
                    [PreviewCleanupError::RehearsalClose(e)] => assert_native_busy(e),
                    _ => panic!("retain independent native D close error"),
                }
            }
            _ => panic!(
                "{primary}: missing primary/native close combination: {:?}",
                result.failure
            ),
        }
        let retained = probe
            .inner
            .inventory
            .retained_connection(DestinationRole::Rehearsal)
            .expect("inventory must retain native handle after SQLITE_BUSY");
        assert!(!retained.is_readonly("main").unwrap());
        assert!(retained.is_autocommit());
        assert_eq!(
            retained
                .query_row("SELECT count(*) FROM package", [], |r| r.get::<_, i64>(0))
                .unwrap(),
            1
        );
        assert_eq!(&*probe.w.transactions.borrow(), &["BEGIN", "ROLLBACK"]);
        assert_eq!(
            gate.state(),
            if primary == "cancel" {
                PublicationState::CancelAccepted
            } else {
                PublicationState::Open
            }
        );
        assert_eq!(
            unsafe {
                rusqlite::ffi::sqlite3_finalize(probe.statement.replace(std::ptr::null_mut()))
            },
            rusqlite::ffi::SQLITE_OK
        );
        assert_eq!(package_count(&owner), 0);
    }
}

#[test]
fn second_rehearsal_refusal_failed_w_rollback_retains_b() {
    let (dir, mut owner) = migrated_owner();
    let bytes = valid_package();
    let mut shown = shown(&mut owner, &bytes);
    shown.binding.preview.encoded_bytes += 1;
    let gate = Arc::new(PublicationGate::new());
    let mut probe = Probe::new(
        &owner,
        &dir.path().join("products.sqlite"),
        gate.clone(),
        false,
    );
    probe.w.deny_rollback.set(true);
    let result = apply(&mut owner, &bytes, &shown.binding, &gate, &mut probe);
    probe.assert_no_w_dml();
    assert_eq!(probe.d.package_inserts.get(), 1);
    probe.assert_closed();
    assert!(matches!(result.failure, Some(OrchestrationError::Stale)));
    assert_eq!(result.commit, CommitEvidence::NotCommitted);
    assert_eq!(result.rollback, RollbackDisposition::Unresolved);
    assert!(matches!(result.health, WriterHealth::Quarantined(_)));
    assert_eq!(result.artifacts, ArtifactDisposition::Retain);
    assert!(probe.inner.inventory.allocated(DestinationRole::Recovery));
    assert!(
        matches!(&result.cleanup[..], [PreviewCleanupError::WriterRollback(rusqlite::Error::SqliteFailure(e,_)), PreviewCleanupError::WriterNotAutocommit] if e.code == rusqlite::ErrorCode::AuthorizationForStatementDenied)
    );
    assert_eq!(&*probe.w.transactions.borrow(), &["BEGIN", "ROLLBACK"]);
    assert_eq!(
        unsafe { rusqlite::ffi::sqlite3_get_autocommit(probe.w.db.get()) },
        0
    );
    assert_eq!(gate.state(), PublicationState::Open);
    assert!(owner.with_read(|_| Ok::<_, rusqlite::Error>(())).is_err());
}
