//! Actual-allocator growth admission and scoped configuration settlement.
//!
//! Native file-backed controls use finite test-only ceilings, never huge files.

use super::*;
use crate::catalogue_copy::DestinationStage;
use std::cell::{Cell, RefCell};
use std::ffi::{c_char, c_int, c_void, CStr};
use std::rc::Rc;

fn scalar(c: &crate::Connection, sql: &str) -> i64 {
    c.query_row(sql, [], |r| r.get(0)).unwrap()
}
fn shown(owner: &mut ManagedCatalogue, bytes: &[u8]) -> ControlledPreview {
    owner
        .preview_current_controlled(
            admit("growth.knxprod", bytes),
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
    artifacts: &mut dyn CatalogueArtifacts,
) -> ControlledApplySettlement {
    owner.apply_current_controlled(
        admit("growth.knxprod", bytes),
        shown,
        &OperationContext {
            gate: &PublicationGate::new(),
            policy: ControlledPolicy::Standard,
        },
        artifacts,
    )
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Fault {
    None,
    IgnoreSet,
    DenySet,
    DenyRead,
    DenyRestoreSet,
    DenyRestoreRead,
    IgnoreRestoreSet,
    DenyReadRestoreSet,
    DenyPriorRead,
    DenySyncRead,
    IgnoreSyncRead,
}

struct Native {
    db: Cell<*mut rusqlite::ffi::sqlite3>,
    role: &'static str,
    events: Rc<RefCell<Vec<String>>>,
    sets: Cell<usize>,
    dml: Cell<usize>,
    fault: Cell<Fault>,
    deny_package: Cell<bool>,
    cancellation: Cell<Option<crate::operation::CancelOutcome>>,
    cancel: RefCell<Option<std::sync::Arc<PublicationGate>>>,
}
impl Native {
    fn new(role: &'static str, events: Rc<RefCell<Vec<String>>>) -> Rc<Self> {
        Rc::new(Self {
            db: Cell::new(std::ptr::null_mut()),
            role,
            events,
            sets: Cell::new(0),
            dml: Cell::new(0),
            fault: Cell::new(Fault::None),
            deny_package: Cell::new(false),
            cancellation: Cell::new(None),
            cancel: RefCell::new(None),
        })
    }
    fn attach(&self, c: &crate::Connection) {
        let db = unsafe { c.handle() };
        assert!(self.db.replace(db).is_null());
        // SAFETY: Rc pins the context; Probe detaches it before either connection
        // or context is released, including retained incomplete destinations.
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
    fn event(&self, event: String) {
        self.events
            .borrow_mut()
            .push(format!("{}:{event}", self.role));
    }
}
unsafe extern "C" fn authorize(
    context: *mut c_void,
    action: c_int,
    first: *const c_char,
    second: *const c_char,
    _: *const c_char,
    _: *const c_char,
) -> c_int {
    let n = unsafe { &*context.cast::<Native>() };
    let first = if first.is_null() {
        ""
    } else {
        unsafe { CStr::from_ptr(first) }.to_str().unwrap_or("")
    };
    let value = if second.is_null() {
        "read"
    } else {
        unsafe { CStr::from_ptr(second) }.to_str().unwrap_or("")
    };
    if action == rusqlite::ffi::SQLITE_PRAGMA {
        n.event(format!("pragma:{first}:{value}"));
        if first == "synchronous" && second.is_null() {
            match n.fault.get() {
                Fault::DenySyncRead => return rusqlite::ffi::SQLITE_DENY,
                Fault::IgnoreSyncRead => return rusqlite::ffi::SQLITE_IGNORE,
                _ => {}
            }
        }
        if first == "max_page_count" {
            if !second.is_null() {
                let sets = n.sets.get() + 1;
                n.sets.set(sets);
                match (sets, n.fault.get()) {
                    (1, Fault::IgnoreSet) => return rusqlite::ffi::SQLITE_IGNORE,
                    (1, Fault::DenySet) => return rusqlite::ffi::SQLITE_DENY,
                    (2.., Fault::DenyRestoreSet | Fault::DenyReadRestoreSet) => {
                        return rusqlite::ffi::SQLITE_DENY;
                    }
                    (2.., Fault::IgnoreRestoreSet) => return rusqlite::ffi::SQLITE_IGNORE,
                    _ => {}
                }
            } else if (n.sets.get() == 1
                && matches!(n.fault.get(), Fault::DenyRead | Fault::DenyReadRestoreSet))
                || (n.sets.get() == 0 && n.fault.get() == Fault::DenyPriorRead)
                || (n.sets.get() >= 2 && n.fault.get() == Fault::DenyRestoreRead)
            {
                return rusqlite::ffi::SQLITE_DENY;
            }
        }
    }
    if action == rusqlite::ffi::SQLITE_TRANSACTION {
        n.event(format!("tx:{first}"));
    }
    if matches!(
        action,
        rusqlite::ffi::SQLITE_INSERT | rusqlite::ffi::SQLITE_UPDATE | rusqlite::ffi::SQLITE_DELETE
    ) {
        n.dml.set(n.dml.get() + 1);
        n.event(format!("dml:{first}"));
        if first == "package" && n.deny_package.get() {
            return rusqlite::ffi::SQLITE_DENY;
        }
    }
    rusqlite::ffi::SQLITE_OK
}
unsafe extern "C" fn updated(
    context: *mut c_void,
    _: c_int,
    _: *const c_char,
    _: *const c_char,
    _: i64,
) {
    let n = unsafe { &*context.cast::<Native>() };
    if let Some(gate) = n.cancel.borrow_mut().take() {
        n.cancellation.set(Some(gate.try_cancel()));
    }
}
struct Probe {
    inner: TestArtifacts,
    w: Rc<Native>,
    d: Rc<Native>,
    prelive: Vec<(i64, i64, bool, usize)>,
}
impl Probe {
    fn new(owner: &ManagedCatalogue) -> Self {
        let events = Rc::new(RefCell::new(Vec::new()));
        let w = Native::new("W", events.clone());
        owner
            .with_read(|c| {
                w.attach(c);
                Ok::<_, rusqlite::Error>(())
            })
            .unwrap()
            .unwrap();
        let d = Native::new("D", events);
        let dd = d.clone();
        let mut inner = TestArtifacts::new();
        inner.inventory.set_hook(
            DestinationRole::Rehearsal,
            Box::new(move |stage, _, conn| {
                if stage == DestinationStage::WriteOpened {
                    dd.attach(conn.unwrap());
                }
                if stage == DestinationStage::RehearsalClose {
                    dd.event("close".into());
                    dd.detach();
                }
            }),
        );
        Self {
            inner,
            w,
            d,
            prelive: Vec::new(),
        }
    }
}
impl Drop for Probe {
    fn drop(&mut self) {
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
    fn before_live_install(&mut self, c: &crate::Connection) {
        self.prelive.push((
            scalar(c, "PRAGMA main.page_size"),
            scalar(c, "PRAGMA main.max_page_count"),
            c.is_autocommit(),
            self.w.dml.get(),
        ));
        self.w.event("prelive".into());
    }
}

#[test]
fn actual_w_cap_is_scoped_through_commit_and_exactly_restored() {
    for sync in ["FULL", "EXTRA"] {
        let (_dir, mut owner) = migrated_owner();
        owner
            .with_read(|c| {
                c.execute_batch(&format!(
                    "PRAGMA main.synchronous={sync}; PRAGMA main.max_page_count=100000000"
                ))
            })
            .unwrap()
            .unwrap();
        let bytes = valid_package();
        let shown = shown(&mut owner, &bytes);
        let mut probe = Probe::new(&owner);
        let result = apply(&mut owner, &bytes, &shown.binding, &mut probe);
        assert_eq!(result.commit, CommitEvidence::Committed, "{result:?}");
        assert_eq!(probe.prelive.len(), 1);
        let (size, cap, auto, dml) = probe.prelive[0];
        assert_eq!(
            cap,
            68_719_476_736 / size,
            "actual W, not D, must enforce the scope cap"
        );
        assert!(!auto);
        assert_eq!(dml, 0);
        let events = probe.w.events.borrow();
        let pos = |s: &str| events.iter().position(|e| e == s).unwrap();
        assert!(pos("W:pragma:synchronous:read") < pos("W:tx:BEGIN"));
        assert!(pos("D:close") < pos("W:prelive"));
        let commit = pos("W:tx:COMMIT");
        let sets: Vec<_> = events
            .iter()
            .enumerate()
            .filter(|(_, e)| e.starts_with("W:pragma:max_page_count:") && !e.ends_with(":read"))
            .map(|(i, _)| i)
            .collect();
        assert_eq!(sets.len(), 2);
        assert!(sets[0] < pos("W:prelive"));
        assert!(commit < sets[1]);
        assert!(events[sets[0] + 1..commit]
            .iter()
            .any(|s| s == "W:pragma:max_page_count:read"));
        assert!(events[sets[1] + 1..]
            .iter()
            .any(|s| s == "W:pragma:max_page_count:read"));
        drop(events);
        assert_eq!(result.health, WriterHealth::Healthy);
        assert!(result.cleanup.is_empty());
        assert_eq!(
            owner
                .with_read(|c| Ok::<_, rusqlite::Error>(scalar(c, "PRAGMA main.max_page_count")))
                .unwrap()
                .unwrap(),
            100000000
        );
    }
}

#[test]
fn normal_synchronous_refuses_before_begin_without_repair() {
    let (_dir, mut owner) = migrated_owner();
    owner
        .with_read(|c| c.execute_batch("PRAGMA main.synchronous=NORMAL"))
        .unwrap()
        .unwrap();
    let bytes = valid_package();
    let shown = shown(&mut owner, &bytes);
    let mut probe = Probe::new(&owner);
    let result = apply(&mut owner, &bytes, &shown.binding, &mut probe);
    assert_eq!(
        result.commit,
        CommitEvidence::NotCommitted,
        "NORMAL must refuse rather than be repaired"
    );
    assert_eq!(result.rollback, RollbackDisposition::NotApplicable);
    assert!(result.failure.is_some());
    assert!(result.cleanup.is_empty());
    assert_eq!(result.health, WriterHealth::Healthy);
    assert_eq!(result.artifacts, ArtifactDisposition::None);
    assert_eq!(probe.inner.counter, 0);
    assert_eq!(probe.w.dml.get(), 0);
    assert!(!probe.w.events.borrow().iter().any(|s| s == "W:tx:BEGIN"));
    assert_eq!(
        owner
            .with_read(|c| Ok::<_, rusqlite::Error>(scalar(c, "PRAGMA main.synchronous")))
            .unwrap()
            .unwrap(),
        1
    );
}

#[test]
fn unsupported_actual_lanes_refuse_without_conversion_or_installer_dml() {
    for role in ["D", "W"] {
        for sql in [
            "ATTACH ':memory:' AS uncontrolled",
            "CREATE TEMP TABLE application_state(v TEXT)",
            "PRAGMA main.journal_mode=MEMORY",
            "PRAGMA main.locking_mode=EXCLUSIVE",
        ] {
            let (_dir, mut owner) = migrated_owner();
            let bytes = valid_package();
            let shown = shown(&mut owner, &bytes);
            if role == "W" {
                owner.with_read(|c| c.execute_batch(sql)).unwrap().unwrap();
            }
            let mut probe = Probe::new(&owner);
            if role == "D" {
                let dd = probe.d.clone();
                probe.inner.inventory.set_hook(
                    DestinationRole::Rehearsal,
                    Box::new(move |stage, _, conn| {
                        if stage == DestinationStage::WriteOpened {
                            let c = conn.unwrap();
                            c.execute_batch(sql).unwrap();
                            dd.attach(c);
                        }
                        if stage == DestinationStage::RehearsalClose {
                            dd.detach();
                        }
                    }),
                );
            }
            let result = apply(&mut owner, &bytes, &shown.binding, &mut probe);
            assert_eq!(
                result.commit,
                CommitEvidence::NotCommitted,
                "{role}: unsupported actual lane must refuse: {sql}"
            );
            assert!(result.failure.is_some());
            assert_eq!(probe.w.dml.get(), 0);
            assert_eq!(probe.d.dml.get(), 0);
            assert_eq!(result.health, WriterHealth::Healthy, "{result:?}");
            if role == "W" {
                assert_eq!(
                    probe.w.sets.get(),
                    0,
                    "unsupported lane admission must stay read-only"
                );
                owner.with_read(|c| {
                    if sql.contains("journal_mode") { assert_eq!(c.query_row("PRAGMA main.journal_mode", [], |r| r.get::<_, String>(0)).unwrap(), "memory"); }
                    if sql.contains("locking_mode") { assert_eq!(c.query_row("PRAGMA main.locking_mode", [], |r| r.get::<_, String>(0)).unwrap(), "exclusive"); }
                    if sql.contains("ATTACH") { assert_eq!(scalar(c,"SELECT count(*) FROM pragma_database_list WHERE name='uncontrolled'"),1); }
                    if sql.contains("TEMP") { assert_eq!(scalar(c,"SELECT count(*) FROM temp.sqlite_schema WHERE name='application_state'"),1); }
                    Ok::<_, rusqlite::Error>(())
                }).unwrap().unwrap();
            } else {
                assert!(probe
                    .inner
                    .inventory
                    .retained_connection(DestinationRole::Rehearsal)
                    .is_some());
            }
        }
    }
}

#[test]
fn setters_and_independent_readback_are_authority_on_each_actual_allocator() {
    for role in ["D", "W"] {
        for fault in [Fault::IgnoreSet, Fault::DenySet, Fault::DenyRead] {
            let (_dir, mut owner) = migrated_owner();
            owner
                .with_read(|c| c.execute_batch("PRAGMA main.max_page_count=100000000"))
                .unwrap()
                .unwrap();
            let bytes = valid_package();
            let shown = shown(&mut owner, &bytes);
            let mut probe = Probe::new(&owner);
            if role == "D" {
                probe.d.fault.set(fault);
            } else {
                probe.w.fault.set(fault);
            }
            let result = apply(&mut owner, &bytes, &shown.binding, &mut probe);
            assert_eq!(
                result.commit,
                CommitEvidence::NotCommitted,
                "{role}/{fault:?}: admission must refuse"
            );
            let growth = match result.failure.as_ref() {
                Some(OrchestrationError::Growth(e)) if role == "W" => e,
                Some(OrchestrationError::Copy(crate::catalogue_copy::CopyError::Destination(
                    f,
                ))) if role == "D" => {
                    assert!(f.cleanup.is_empty());
                    match f.primary.as_ref() {
                        Some(crate::catalogue_copy::DestinationError::Growth(e)) => e,
                        _ => panic!("typed D growth primary required"),
                    }
                }
                _ => panic!(
                    "{role}/{fault:?}: typed growth primary required: {:?}",
                    result.failure
                ),
            };
            match (fault, growth) {
                (
                    Fault::IgnoreSet,
                    crate::catalogue_growth::GrowthError::Readback { expected, observed },
                ) => {
                    assert_eq!(*expected, 16777216);
                    assert_ne!(*observed, *expected);
                }
                (
                    Fault::DenySet | Fault::DenyRead,
                    crate::catalogue_growth::GrowthError::Sqlite(rusqlite::Error::SqliteFailure(
                        e,
                        _,
                    )),
                ) => assert_eq!(e.code, rusqlite::ErrorCode::AuthorizationForStatementDenied),
                _ => panic!("unexpected authority failure: {growth:?}"),
            }
            assert_eq!(probe.w.dml.get(), 0);
            assert_eq!(probe.d.dml.get(), 0);
            assert!(result.cleanup.is_empty(), "{result:?}");
            assert_eq!(result.health, WriterHealth::Healthy);
            assert_eq!(result.rollback, RollbackDisposition::Explicit);
            if role == "D" {
                assert_eq!(result.artifacts, ArtifactDisposition::Dispose);
                assert!(probe
                    .inner
                    .inventory
                    .retained_connection(DestinationRole::Rehearsal)
                    .is_some());
            } else {
                assert_eq!(result.artifacts, ArtifactDisposition::None);
                assert_eq!(probe.w.sets.get(), 2);
            }
            // Clear the synthetic observer fault, then independently inspect W.
            probe.w.fault.set(Fault::None);
            assert_eq!(
                owner
                    .with_read(|c| Ok::<_, rusqlite::Error>(scalar(
                        c,
                        "PRAGMA main.max_page_count"
                    )))
                    .unwrap()
                    .unwrap(),
                100000000
            );
        }
    }
}

fn copy_witness(c: &crate::Connection) -> crate::catalogue_schema::CopyContentWitness {
    let tx = c.unchecked_transaction().unwrap();
    let witness =
        crate::catalogue_schema::copy_content_witness(&tx, &PublicationGate::new()).unwrap();
    tx.rollback().unwrap();
    witness
}

fn large_package(owner: &ManagedCatalogue) -> (Vec<u8>, i64, i64) {
    let (size, count, free) = owner
        .with_read(|c| {
            Ok::<_, rusqlite::Error>((
                scalar(c, "PRAGMA main.page_size"),
                scalar(c, "PRAGMA main.page_count"),
                scalar(c, "PRAGMA main.freelist_count"),
            ))
        })
        .unwrap()
        .unwrap();
    let opaque = vec![0x5a; usize::try_from(size * count + 65536).unwrap()];
    let mut writer = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    for (path, data) in [
        ("knx_master.xml", MASTER),
        ("M-0001/Hardware.xml", HARDWARE),
        ("M-0001/Baggages/retained.bin", opaque.as_slice()),
    ] {
        writer
            .start_file(
                path,
                zip::write::SimpleFileOptions::default()
                    .compression_method(zip::CompressionMethod::Stored),
            )
            .unwrap();
        writer.write_all(data).unwrap();
    }
    let bytes = writer.finish().unwrap().into_inner();
    assert!(
        bytes.len() as i64 > size * count,
        "raw retained source alone exceeds ALL existing page space, including free pages"
    );
    assert!(bytes.len() < 1_000_000_000);
    assert!(opaque.len() < 1_000_000_000);
    assert!(((opaque.len() + MASTER.len() + HARDWARE.len()) as u64) < 4 * 1024 * 1024 * 1024);
    println!(
        "finite fixture page_size={size} page_count={count} freelist={free} raw={} opaque={}",
        bytes.len(),
        opaque.len()
    );
    (bytes, size, count)
}
fn assert_full(result: &ControlledApplySettlement) {
    assert!(
        matches!(&result.failure, Some(OrchestrationError::Install(crate::PackageError::Database(crate::ProductDbError::Sqlite(rusqlite::Error::SqliteFailure(e,_))))) if e.code == rusqlite::ErrorCode::DiskFull && e.extended_code == rusqlite::ffi::SQLITE_FULL),
        "actual native FULL required: {:?}",
        result.failure
    );
}
#[test]
fn native_full_on_d_is_independent_of_w_and_prevents_live_dml() {
    let (_dir, mut owner) = migrated_owner();
    let (bytes, size, count) = large_package(&owner);
    let shown = shown(&mut owner, &bytes);
    let cap = u64::try_from(size * count).unwrap();
    let _caps = crate::catalogue_growth::TestCeilings::new(Some(cap), None);
    let mut probe = Probe::new(&owner);
    let result = apply(&mut owner, &bytes, &shown.binding, &mut probe);
    assert_eq!(
        result.commit,
        CommitEvidence::NotCommitted,
        "D must refuse a new native page allocation"
    );
    assert_full(&result);
    assert!(probe.d.dml.get() > 0);
    assert_eq!(probe.w.dml.get(), 0);
    assert_eq!(result.rollback, RollbackDisposition::Explicit);
    assert_eq!(result.health, WriterHealth::Healthy);
    assert_eq!(result.artifacts, ArtifactDisposition::Dispose);
    assert!(result.cleanup.is_empty());
    let path = probe.inner.dir.path().join("dest-0.sqlite");
    let d = crate::Connection::open(path).unwrap();
    assert!(scalar(&d, "PRAGMA main.page_count") <= count);
    assert_eq!(package_count(&owner), 0);
}
#[test]
fn native_full_on_w_follows_successful_d_and_restores_complete_prestate() {
    let (_dir, mut owner) = migrated_owner();
    let (bytes, size, count) = large_package(&owner);
    let shown = shown(&mut owner, &bytes);
    let before = owner
        .with_read(|c| Ok::<_, rusqlite::Error>(copy_witness(c)))
        .unwrap()
        .unwrap();
    let prior = owner
        .with_read(|c| Ok::<_, rusqlite::Error>(scalar(c, "PRAGMA main.max_page_count")))
        .unwrap()
        .unwrap();
    let _caps = crate::catalogue_growth::TestCeilings::new(
        None,
        Some(u64::try_from(size * count).unwrap()),
    );
    let mut probe = Probe::new(&owner);
    let result = apply(&mut owner, &bytes, &shown.binding, &mut probe);
    assert_eq!(
        result.commit,
        CommitEvidence::NotCommitted,
        "actual W must refuse independently of successful D"
    );
    assert_full(&result);
    assert!(probe.d.dml.get() > 0);
    assert!(probe.w.dml.get() > 0);
    assert_eq!(probe.prelive[0].1, count);
    assert_eq!(
        result.rollback,
        RollbackDisposition::Automatic,
        "SQLITE_FULL actually ended this transaction"
    );
    assert_eq!(result.health, WriterHealth::Healthy);
    assert_eq!(result.artifacts, ArtifactDisposition::Dispose);
    assert!(result.cleanup.is_empty());
    owner
        .with_read(|c| {
            assert_eq!(scalar(c, "PRAGMA main.page_count"), count);
            assert_eq!(scalar(c, "PRAGMA main.max_page_count"), prior);
            assert_eq!(copy_witness(c), before);
            Ok::<_, rusqlite::Error>(())
        })
        .unwrap()
        .unwrap();
}
#[test]
fn finite_cap_allows_install_below_ceiling_on_both_allocators() {
    let (_dir, mut owner) = migrated_owner();
    let (bytes, size, count) = large_package(&owner);
    let shown = shown(&mut owner, &bytes);
    let pages = count + 4 * (i64::try_from(bytes.len()).unwrap() / size + 1) + 1024;
    let limit = u64::try_from(pages * size).unwrap();
    let _caps = crate::catalogue_growth::TestCeilings::new(Some(limit), Some(limit));
    let mut probe = Probe::new(&owner);
    let result = apply(&mut owner, &bytes, &shown.binding, &mut probe);
    assert_eq!(result.commit, CommitEvidence::Committed, "{result:?}");
    assert_eq!(result.health, WriterHealth::Healthy);
    assert_eq!(probe.prelive[0].1, pages);
    assert_eq!(package_count(&owner), 1);
    owner
        .with_read(|c| {
            let actual = scalar(c, "PRAGMA main.page_count");
            assert!(actual > count && actual <= pages);
            println!("finite successful install page_count={actual} limit={pages}");
            Ok::<_, rusqlite::Error>(())
        })
        .unwrap()
        .unwrap();
}

#[test]
fn refusal_and_accepted_cancel_restore_exact_cap_after_native_rollback() {
    for cancel in [false, true] {
        let (_dir, mut owner) = migrated_owner();
        owner
            .with_read(|c| c.execute_batch("PRAGMA main.max_page_count=100000000"))
            .unwrap()
            .unwrap();
        let bytes = valid_package();
        let shown = shown(&mut owner, &bytes);
        let gate = std::sync::Arc::new(PublicationGate::new());
        let mut probe = Probe::new(&owner);
        if cancel {
            *probe.w.cancel.borrow_mut() = Some(gate.clone());
        } else {
            probe.w.deny_package.set(true);
        }
        let result = owner.apply_current_controlled(
            admit("growth.knxprod", &bytes),
            &shown.binding,
            &OperationContext {
                gate: &gate,
                policy: ControlledPolicy::Standard,
            },
            &mut probe,
        );
        assert_eq!(
            probe.prelive[0].1, 16777216,
            "scope must be active even when install refuses"
        );
        assert_eq!(result.commit, CommitEvidence::NotCommitted);
        assert_eq!(result.rollback, RollbackDisposition::Explicit);
        assert_eq!(result.health, WriterHealth::Healthy);
        assert_eq!(result.artifacts, ArtifactDisposition::Dispose);
        assert!(result.cleanup.is_empty());
        if cancel {
            assert_eq!(
                probe.w.cancellation.get(),
                Some(crate::operation::CancelOutcome::Accepted)
            );
            assert_eq!(
                gate.state(),
                crate::operation::PublicationState::CancelAccepted
            );
            assert!(matches!(
                result.failure,
                Some(OrchestrationError::Install(crate::PackageError::Cancelled(
                    crate::operation::Stop::CancelAccepted
                )))
            ));
        } else {
            assert!(matches!(
                result.failure,
                Some(OrchestrationError::Install(_))
            ));
        }
        let events = probe.w.events.borrow();
        let rollback = events.iter().position(|e| e == "W:tx:ROLLBACK").unwrap();
        let sets: Vec<_> = events
            .iter()
            .enumerate()
            .filter(|(_, e)| e.starts_with("W:pragma:max_page_count:") && !e.ends_with(":read"))
            .map(|(i, _)| i)
            .collect();
        assert_eq!(sets.len(), 2);
        assert!(sets[0] < rollback && rollback < sets[1]);
        assert!(events[sets[1] + 1..]
            .iter()
            .any(|e| e == "W:pragma:max_page_count:read"));
        assert!(!events.iter().any(|e| e == "W:tx:COMMIT"));
        drop(events);
        assert_eq!(
            owner
                .with_read(|c| Ok::<_, rusqlite::Error>(scalar(c, "PRAGMA main.max_page_count")))
                .unwrap()
                .unwrap(),
            100000000
        );
        assert_eq!(package_count(&owner), 0);
    }
}
#[test]
fn restoration_failure_preserves_committed_or_primary_refusal_and_retains_b() {
    for committed in [true, false] {
        for fault in [Fault::DenyRestoreSet, Fault::DenyRestoreRead] {
            let (_dir, mut owner) = migrated_owner();
            owner
                .with_read(|c| c.execute_batch("PRAGMA main.max_page_count=100000000"))
                .unwrap()
                .unwrap();
            let bytes = valid_package();
            let shown = shown(&mut owner, &bytes);
            let mut probe = Probe::new(&owner);
            probe.w.fault.set(fault);
            probe.w.deny_package.set(!committed);
            let result = apply(&mut owner, &bytes, &shown.binding, &mut probe);
            assert_eq!(probe.prelive[0].1, 16777216);
            assert_eq!(
                result.commit,
                if committed {
                    CommitEvidence::Committed
                } else {
                    CommitEvidence::NotCommitted
                },
                "cleanup must never overwrite witnessed commit"
            );
            assert_eq!(
                owner.retained_apply_witness().unwrap().trace,
                if committed {
                    LiveCommitTrace::LiveCommitReturnedOk { autocommit: true }
                } else {
                    LiveCommitTrace::NoLiveCommitInvocation
                },
                "restoration failure cannot rewrite the native trace"
            );
            assert_eq!(
                result.rollback,
                if committed {
                    RollbackDisposition::NotApplicable
                } else {
                    RollbackDisposition::Explicit
                }
            );
            if committed {
                assert!(result.failure.is_none());
            } else {
                assert!(matches!(
                    result.failure,
                    Some(OrchestrationError::Install(_))
                ));
            }
            assert!(
                matches!(&result.cleanup[..],[PreviewCleanupError::GrowthRestore(crate::catalogue_growth::GrowthError::Sqlite(rusqlite::Error::SqliteFailure(e,_)))] if e.code==rusqlite::ErrorCode::AuthorizationForStatementDenied),
                "independent restoration failure required: {result:?}"
            );
            assert!(matches!(result.health, WriterHealth::Quarantined(_)));
            assert_eq!(result.artifacts, ArtifactDisposition::Retain);
            assert!(probe.inner.inventory.allocated(DestinationRole::Recovery));
            assert!(owner.with_read(|_| Ok::<_, rusqlite::Error>(())).is_err());
            let subsequent = apply(
                &mut owner,
                &bytes,
                &shown.binding,
                &mut TestArtifacts::new(),
            );
            assert!(matches!(
                subsequent.failure,
                Some(OrchestrationError::NotHealthy(_))
            ));
            let b = crate::Connection::open_with_flags(
                probe.inner.dir.path().join("dest-1.sqlite"),
                rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
            )
            .unwrap();
            assert_eq!(scalar(&b, "SELECT count(*) FROM package"), 0);
        }
    }
}
#[test]
fn native_clamped_restore_is_not_exact_and_keeps_witnessed_commit() {
    let (dir, mut owner) = migrated_owner();
    let (bytes, size, count) = large_package(&owner);
    owner
        .with_read(|c| c.execute_batch(&format!("PRAGMA main.max_page_count={count}")))
        .unwrap()
        .unwrap();
    let shown = shown(&mut owner, &bytes);
    let mut probe = Probe::new(&owner);
    let result = apply(&mut owner, &bytes, &shown.binding, &mut probe);
    assert_eq!(result.commit, CommitEvidence::Committed);
    assert!(result.failure.is_none());
    assert_eq!(result.rollback, RollbackDisposition::NotApplicable);
    assert!(
        matches!(&result.cleanup[..],[PreviewCleanupError::GrowthRestore(crate::catalogue_growth::GrowthError::Readback{expected,observed})] if *expected==count && *observed>count)
    );
    assert!(matches!(result.health, WriterHealth::Quarantined(_)));
    assert_eq!(result.artifacts, ArtifactDisposition::Retain);
    let c = crate::Connection::open_with_flags(
        dir.path().join("products.sqlite"),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    assert_eq!(scalar(&c, "SELECT count(*) FROM package"), 1);
    assert!(scalar(&c, "PRAGMA main.page_count") > count);
    assert_eq!(scalar(&c, "PRAGMA main.page_size"), size);
}

#[test]
fn unreadable_synchronous_refuses_before_begin_and_missing_prior_cap_stays_read_only() {
    for fault in [
        Fault::DenySyncRead,
        Fault::IgnoreSyncRead,
        Fault::DenyPriorRead,
    ] {
        let (_dir, mut owner) = migrated_owner();
        let bytes = valid_package();
        let shown = shown(&mut owner, &bytes);
        let mut probe = Probe::new(&owner);
        probe.w.fault.set(fault);
        let result = apply(&mut owner, &bytes, &shown.binding, &mut probe);
        assert!(matches!(
            result.failure,
            Some(OrchestrationError::Growth(
                crate::catalogue_growth::GrowthError::Sqlite(_)
            ))
        ));
        assert_eq!(result.commit, CommitEvidence::NotCommitted);
        assert_eq!(result.health, WriterHealth::Healthy);
        assert!(result.cleanup.is_empty());
        assert_eq!(result.artifacts, ArtifactDisposition::None);
        assert_eq!(probe.w.sets.get(), 0);
        assert_eq!(probe.w.dml.get(), 0);
        assert_eq!(probe.inner.counter, 0);
        if fault != Fault::DenyPriorRead {
            assert_eq!(result.rollback, RollbackDisposition::NotApplicable);
            assert!(!probe.w.events.borrow().iter().any(|e| e == "W:tx:BEGIN"));
        } else {
            assert_eq!(result.rollback, RollbackDisposition::Explicit);
        }
    }
}
#[test]
fn existing_extent_above_finite_cap_refuses_each_actual_allocator_before_setting() {
    for role in ["D", "W"] {
        let (_dir, mut owner) = migrated_owner();
        let bytes = valid_package();
        let shown = shown(&mut owner, &bytes);
        let extent = owner
            .with_read(|c| {
                Ok::<_, rusqlite::Error>(
                    scalar(c, "PRAGMA main.page_size") * scalar(c, "PRAGMA main.page_count"),
                )
            })
            .unwrap()
            .unwrap();
        let below = u64::try_from(extent - 1).unwrap();
        let _caps = if role == "D" {
            crate::catalogue_growth::TestCeilings::new(Some(below), None)
        } else {
            crate::catalogue_growth::TestCeilings::new(None, Some(below))
        };
        let mut probe = Probe::new(&owner);
        let result = apply(&mut owner, &bytes, &shown.binding, &mut probe);
        assert_eq!(result.commit, CommitEvidence::NotCommitted);
        assert_eq!(result.health, WriterHealth::Healthy);
        assert!(result.cleanup.is_empty());
        if role == "D" {
            assert!(
                matches!(&result.failure,Some(OrchestrationError::Copy(crate::catalogue_copy::CopyError::Destination(f))) if matches!(f.primary,Some(crate::catalogue_copy::DestinationError::Growth(crate::catalogue_growth::GrowthError::AboveCap))))
            );
            assert_eq!(probe.d.sets.get(), 0);
        } else {
            assert!(matches!(
                result.failure,
                Some(OrchestrationError::Growth(
                    crate::catalogue_growth::GrowthError::AboveCap
                ))
            ));
            assert_eq!(probe.w.sets.get(), 0);
        }
        assert_eq!(probe.w.dml.get(), 0);
        assert_eq!(probe.d.dml.get(), 0);
    }
}
#[test]
fn failed_scope_readback_and_failed_restore_keep_both_independent_facts() {
    let (_dir, mut owner) = migrated_owner();
    let bytes = valid_package();
    let shown = shown(&mut owner, &bytes);
    let mut probe = Probe::new(&owner);
    probe.w.fault.set(Fault::DenyReadRestoreSet);
    let result = apply(&mut owner, &bytes, &shown.binding, &mut probe);
    assert!(matches!(
        result.failure,
        Some(OrchestrationError::Growth(
            crate::catalogue_growth::GrowthError::Sqlite(_)
        ))
    ));
    assert!(matches!(
        &result.cleanup[..],
        [PreviewCleanupError::GrowthRestore(
            crate::catalogue_growth::GrowthError::Sqlite(_)
        )]
    ));
    assert_eq!(probe.w.sets.get(), 2);
    assert_eq!(probe.w.dml.get(), 0);
    assert_eq!(probe.inner.counter, 0);
    assert_eq!(result.commit, CommitEvidence::NotCommitted);
    assert_eq!(result.rollback, RollbackDisposition::Explicit);
    assert!(matches!(result.health, WriterHealth::Quarantined(_)));
    assert_eq!(result.artifacts, ArtifactDisposition::None);
}
#[test]
fn ignored_restore_set_is_caught_by_independent_readback_after_commit() {
    let (_dir, mut owner) = migrated_owner();
    owner
        .with_read(|c| c.execute_batch("PRAGMA main.max_page_count=100000000"))
        .unwrap()
        .unwrap();
    let bytes = valid_package();
    let shown = shown(&mut owner, &bytes);
    let mut probe = Probe::new(&owner);
    probe.w.fault.set(Fault::IgnoreRestoreSet);
    let result = apply(&mut owner, &bytes, &shown.binding, &mut probe);
    assert_eq!(result.commit, CommitEvidence::Committed);
    assert!(result.failure.is_none());
    assert!(matches!(
        &result.cleanup[..],
        [PreviewCleanupError::GrowthRestore(
            crate::catalogue_growth::GrowthError::Readback {
                expected: 100000000,
                observed: 16777216
            }
        )]
    ));
    assert!(matches!(result.health, WriterHealth::Quarantined(_)));
    assert_eq!(result.artifacts, ArtifactDisposition::Retain);
}

#[test]
fn actual_reopened_d_caps_both_preview_and_second_rehearsal() {
    let (_dir, mut owner) = migrated_owner();
    let bytes = valid_package();
    let observed = Rc::new(RefCell::new(Vec::new()));
    let configure = |artifacts: &mut TestArtifacts| {
        let observed = observed.clone();
        artifacts.inventory.set_hook(
            DestinationRole::Rehearsal,
            Box::new(move |stage, _, conn| {
                if stage == DestinationStage::WriterClose {
                    // A cap on the OLD backup handle must not stand in for admission
                    // on the subsequently reopened actual SQL allocator.
                    let c = conn.unwrap();
                    c.execute_batch(&format!(
                        "PRAGMA main.max_page_count={}",
                        scalar(c, "PRAGMA main.page_count") + 32
                    ))
                    .unwrap();
                }
                if stage == DestinationStage::WriteReady {
                    let c = conn.unwrap();
                    observed.borrow_mut().push((
                        scalar(c, "PRAGMA main.page_size"),
                        scalar(c, "PRAGMA main.max_page_count"),
                    ));
                }
            }),
        );
    };
    let mut preview_artifacts = TestArtifacts::new();
    configure(&mut preview_artifacts);
    let preview = owner
        .preview_current_controlled(
            admit("growth.knxprod", &bytes),
            &OperationContext {
                gate: &PublicationGate::new(),
                policy: ControlledPolicy::Standard,
            },
            &mut preview_artifacts,
        )
        .unwrap();
    let mut artifacts = TestArtifacts::new();
    configure(&mut artifacts);
    let result = apply(&mut owner, &bytes, &preview.binding, &mut artifacts);
    assert_eq!(result.commit, CommitEvidence::Committed, "{result:?}");
    assert_eq!(observed.borrow().len(), 2);
    for &(size, cap) in observed.borrow().iter() {
        assert_eq!(
            cap,
            68_719_476_736 / size,
            "actual reopened D must carry the production total-main ceiling"
        );
    }
}
