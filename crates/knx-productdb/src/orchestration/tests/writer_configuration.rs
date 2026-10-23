//! Fixed SQL-safety admission observes the actual managed W without repair.
//!
//! Native hooks are scoped to W, never issue recursive SQL, and detach before
//! test-owned teardown. Original semantic refusals were first exercised using
//! only the pre-existing orchestration types.

use super::*;
use crate::managed_catalogue::OwnerError;
use std::cell::{Cell, RefCell};
use std::ffi::{c_char, c_int, c_void, CStr};

const SETTINGS: [(&str, i64); 5] = [
    ("foreign_keys", 1),
    ("ignore_check_constraints", 0),
    ("writable_schema", 0),
    ("defer_foreign_keys", 0),
    ("query_only", 0),
];

fn health(owner: &ManagedCatalogue) -> WriterHealth {
    match owner.with_read(|_| Ok::<_, rusqlite::Error>(())) {
        Ok(Ok(())) => WriterHealth::Healthy,
        Err(OwnerError::NotHealthy(health)) => health,
        other => panic!("unexpected owner state: {other:?}"),
    }
}
fn scalar(conn: &crate::Connection, name: &str) -> i64 {
    conn.pragma_query_value(None, name, |row| row.get(0))
        .unwrap()
}
fn flags(conn: &crate::Connection) -> Vec<i64> {
    SETTINGS
        .iter()
        .map(|(name, _)| scalar(conn, name))
        .collect()
}
fn shown(owner: &mut ManagedCatalogue, bytes: &[u8]) -> PreviewBinding {
    owner
        .preview_current_controlled(
            admit("configuration.knxprod", bytes),
            &OperationContext {
                gate: &PublicationGate::new(),
                policy: ControlledPolicy::Standard,
            },
            &mut TestArtifacts::new(),
        )
        .unwrap()
        .binding
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Event {
    Pragma(&'static str, bool),
    Transaction(&'static str),
    Dml,
}
#[derive(Default)]
struct Native {
    db: Cell<*mut rusqlite::ffi::sqlite3>,
    events: RefCell<Vec<Event>>,
    hook_conflict: Cell<bool>,
    changes: Cell<usize>,
    target: Cell<Option<&'static str>>,
    reads: Cell<usize>,
    deny_read: Cell<usize>,
    ignore: Cell<bool>,
    deny_rollback: Cell<bool>,
    denied_in_transaction: Cell<Option<bool>>,
}
impl Native {
    fn attach(owner: &ManagedCatalogue) -> Box<Self> {
        let native = Box::new(Self::default());
        owner
            .with_read(|conn| {
                let db = unsafe { conn.handle() };
                native.db.set(db);
                // SAFETY: Box pins callback context; Drop clears both hooks while
                // W is still owned, including quarantined/open transactions.
                unsafe {
                    assert_eq!(
                        rusqlite::ffi::sqlite3_set_authorizer(
                            db,
                            Some(authorize),
                            (&*native as *const Self).cast_mut().cast()
                        ),
                        rusqlite::ffi::SQLITE_OK
                    );
                    rusqlite::ffi::sqlite3_update_hook(
                        db,
                        Some(updated),
                        (&*native as *const Self).cast_mut().cast(),
                    );
                }
                Ok::<_, rusqlite::Error>(())
            })
            .unwrap()
            .unwrap();
        native
    }
    fn event(&self, event: Event) {
        // No RefCell panic is permitted across FFI, even on a broken fixture.
        if let Ok(mut events) = self.events.try_borrow_mut() {
            events.push(event);
        } else {
            self.hook_conflict.set(true);
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
    fn transactions(&self) -> Vec<&'static str> {
        self.events
            .borrow()
            .iter()
            .filter_map(|event| match event {
                Event::Transaction(value) => Some(*value),
                _ => None,
            })
            .collect()
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
    second: *const c_char,
    _: *const c_char,
    _: *const c_char,
) -> c_int {
    let n = unsafe { &*context.cast::<Native>() };
    let name = if first.is_null() {
        &[][..]
    } else {
        unsafe { CStr::from_ptr(first) }.to_bytes()
    };
    if action == rusqlite::ffi::SQLITE_PRAGMA {
        let setting = SETTINGS
            .iter()
            .map(|(name, _)| *name)
            .chain(std::iter::once("max_page_count"))
            .find(|value| value.as_bytes() == name);
        if let Some(setting) = setting {
            n.event(Event::Pragma(setting, !second.is_null()));
            if second.is_null() && n.target.get() == Some(setting) {
                let reads = n.reads.get().saturating_add(1);
                n.reads.set(reads);
                if reads == n.deny_read.get() {
                    // Read-only native state observation, not recursive SQL.
                    n.denied_in_transaction.set(Some(
                        unsafe { rusqlite::ffi::sqlite3_get_autocommit(n.db.get()) } == 0,
                    ));
                    return if n.ignore.get() {
                        rusqlite::ffi::SQLITE_IGNORE
                    } else {
                        rusqlite::ffi::SQLITE_DENY
                    };
                }
            }
        }
    }
    if action == rusqlite::ffi::SQLITE_TRANSACTION {
        let transaction = match name {
            b"BEGIN" => "BEGIN",
            b"COMMIT" => "COMMIT",
            b"ROLLBACK" => "ROLLBACK",
            _ => "OTHER",
        };
        n.event(Event::Transaction(transaction));
        if transaction == "ROLLBACK" && n.deny_rollback.get() {
            return rusqlite::ffi::SQLITE_DENY;
        }
    }
    if matches!(
        action,
        rusqlite::ffi::SQLITE_INSERT | rusqlite::ffi::SQLITE_UPDATE | rusqlite::ffi::SQLITE_DELETE
    ) {
        n.event(Event::Dml);
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
    n.changes.set(n.changes.get().saturating_add(1));
}

#[derive(Clone, Copy, Debug)]
enum Route {
    Preview,
    Apply,
}
fn invoke(
    owner: &mut ManagedCatalogue,
    bytes: &[u8],
    binding: &PreviewBinding,
    artifacts: &mut TestArtifacts,
    route: Route,
) -> Result<(), OrchestrationError> {
    let gate = PublicationGate::new();
    let op = OperationContext {
        gate: &gate,
        policy: ControlledPolicy::Standard,
    };
    let source = admit("configuration.knxprod", bytes);
    match route {
        Route::Preview => owner
            .preview_current_controlled(source, &op, artifacts)
            .map(|_| ()),
        Route::Apply => {
            let settlement = owner.apply_current_controlled(source, binding, &op, artifacts);
            eprintln!("apply: {settlement:?}");
            if settlement.failure.is_some() {
                assert_eq!(settlement.commit, CommitEvidence::NotCommitted);
                assert_eq!(settlement.rollback, RollbackDisposition::NotApplicable);
                assert_eq!(settlement.artifacts, ArtifactDisposition::None);
                assert_eq!(settlement.health, WriterHealth::Healthy);
                assert!(settlement.cleanup.is_empty());
            }
            match settlement.failure {
                Some(error) => Err(error),
                None => Ok(()),
            }
        }
    }
}

fn setting(index: usize) -> WriterSetting {
    match index {
        0 => WriterSetting::ForeignKeys,
        1 => WriterSetting::IgnoreCheckConstraints,
        2 => WriterSetting::WritableSchema,
        3 => WriterSetting::DeferForeignKeys,
        4 => WriterSetting::QueryOnly,
        _ => panic!("unknown fixture index"),
    }
}
fn expected_reads(index: usize) -> Vec<Event> {
    SETTINGS[..=index]
        .iter()
        .map(|(name, _)| Event::Pragma(name, false))
        .collect()
}
fn assert_value_error(error: OrchestrationError, index: usize, expected: i64, observed: i64) {
    match error {
        OrchestrationError::WriterConfiguration(WriterConfigurationError::Value {
            setting: actual,
            expected: actual_expected,
            observed: actual_observed,
        }) => assert_eq!(
            (actual, actual_expected, actual_observed),
            (setting(index), expected, observed)
        ),
        other => panic!("expected exact native setting refusal, got {other:?}"),
    }
}
fn assert_read_error(error: OrchestrationError, index: usize, ignore: bool) {
    match error {
        OrchestrationError::WriterConfiguration(WriterConfigurationError::Read {
            setting: actual,
            source,
        }) => {
            assert_eq!(actual, setting(index));
            if ignore {
                assert!(
                    matches!(source, rusqlite::Error::QueryReturnedNoRows),
                    "{source:?}"
                );
            } else {
                assert!(
                    matches!(
                        source,
                        rusqlite::Error::SqliteFailure(
                            rusqlite::ffi::Error {
                                code: rusqlite::ErrorCode::AuthorizationForStatementDenied,
                                ..
                            },
                            _
                        )
                    ),
                    "{source:?}"
                );
            }
        }
        other => panic!("expected exact native read refusal, got {other:?}"),
    }
}

fn unsupported(route: Route, index: usize) {
    let (_dir, mut owner) = migrated_owner();
    let bytes = valid_package();
    let binding = shown(&mut owner, &bytes);
    let (name, required) = SETTINGS[index];
    // Even reading max_page_count ends a native implicit transaction and can
    // reset defer_foreign_keys. Measure it before injecting unsupported flags.
    let cap = owner
        .with_read(|conn| Ok::<_, rusqlite::Error>(scalar(conn, "max_page_count")))
        .unwrap()
        .unwrap();
    owner
        .with_read(|conn| {
            conn.pragma_update(None, name, 1 - required)?;
            assert_eq!(scalar(conn, name), 1 - required);
            Ok::<_, rusqlite::Error>(())
        })
        .unwrap()
        .unwrap();
    // Do not capture another stamp here: transaction settlement resets defer.
    let before = owner
        .with_read(|conn| Ok::<_, rusqlite::Error>((flags(conn), conn.total_changes())))
        .unwrap()
        .unwrap();
    let native = Native::attach(&owner);
    let mut artifacts = TestArtifacts::new();
    let result = invoke(&mut owner, &bytes, &binding, &mut artifacts, route);
    eprintln!("original-compatible {route:?} {name}: {result:?}; events={:?}; allocations={}; health={:?}", native.events.borrow(), artifacts.counter, health(&owner));
    assert!(result.is_err(), "unsupported actual W {name} must refuse");
    assert_value_error(result.unwrap_err(), index, required, 1 - required);
    assert_eq!(*native.events.borrow(), expected_reads(index));
    assert_eq!(
        native.transactions(),
        Vec::<&str>::new(),
        "refusal must precede BEGIN"
    );
    assert_eq!(artifacts.counter, 0);
    assert_eq!(native.changes.get(), 0);
    assert!(!native.events.borrow().contains(&Event::Dml));
    assert!(!native.hook_conflict.get());
    assert_eq!(health(&owner), WriterHealth::Healthy);
    native.detach();
    owner
        .with_read(|conn| {
            assert!(conn.is_autocommit());
            assert_eq!((flags(conn), conn.total_changes()), before);
            assert_eq!(scalar(conn, "max_page_count"), cap);
            Ok::<_, rusqlite::Error>(())
        })
        .unwrap()
        .unwrap();
    // Only now settle a stamp transaction, after checking every flag unchanged.
    owner
        .with_read(|conn| conn.pragma_update(None, name, required))
        .unwrap()
        .unwrap();
    assert_eq!(
        owner.capture_stamp(&PublicationGate::new()).unwrap(),
        binding.catalogue
    );
}
macro_rules! refusal {
    ($name:ident, $route:ident, $index:expr) => {
        #[test]
        fn $name() {
            unsupported(Route::$route, $index);
        }
    };
}
refusal!(preview_foreign_keys_off, Preview, 0);
refusal!(apply_foreign_keys_off, Apply, 0);
refusal!(preview_ignore_checks_on, Preview, 1);
refusal!(apply_ignore_checks_on, Apply, 1);
refusal!(preview_writable_schema_on, Preview, 2);
refusal!(apply_writable_schema_on, Apply, 2);
refusal!(preview_defer_foreign_keys_on, Preview, 3);
refusal!(apply_defer_foreign_keys_on, Apply, 3);
refusal!(preview_query_only_on, Preview, 4);
refusal!(apply_query_only_on, Apply, 4);

#[test]
fn defer_fixture_measures_flags_before_native_cap_read() {
    let (_dir, owner) = migrated_owner();
    owner
        .with_read(|conn| {
            for (name, _) in SETTINGS
                .iter()
                .chain(std::iter::once(&("max_page_count", 0)))
            {
                conn.pragma_update(None, "defer_foreign_keys", 1)?;
                let first = scalar(conn, "defer_foreign_keys");
                let value = scalar(conn, name);
                let after = scalar(conn, "defer_foreign_keys");
                assert_eq!(first, 1);
                assert_eq!(after, if *name == "max_page_count" { 0 } else { 1 });
                eprintln!("{name}: before={first} value={value} after={after}");
            }
            Ok::<_, rusqlite::Error>(())
        })
        .unwrap()
        .unwrap();
}

fn read_refusal(route: Route, second: bool, ignore: bool, deny_rollback: bool) {
    for (index, (name, _)) in SETTINGS.iter().enumerate() {
        let (_dir, mut owner) = migrated_owner();
        let bytes = valid_package();
        let binding = shown(&mut owner, &bytes);
        let cap = owner
            .with_read(|conn| Ok::<_, rusqlite::Error>(scalar(conn, "max_page_count")))
            .unwrap()
            .unwrap();
        let before = owner
            .with_read(|conn| Ok::<_, rusqlite::Error>((flags(conn), conn.total_changes())))
            .unwrap()
            .unwrap();
        let native = Native::attach(&owner);
        native.target.set(Some(name));
        native.deny_read.set(if second { 2 } else { 1 });
        native.ignore.set(ignore);
        native.deny_rollback.set(deny_rollback);
        let mut artifacts = TestArtifacts::new();
        let gate = PublicationGate::new();
        let op = OperationContext {
            gate: &gate,
            policy: ControlledPolicy::Standard,
        };
        let source = admit("configuration.knxprod", &bytes);
        let error = match route {
            Route::Preview => {
                let error = owner
                    .preview_current_controlled(source, &op, &mut artifacts)
                    .unwrap_err();
                if deny_rollback {
                    match error {
                        OrchestrationError::PreviewSettlement(failure) => {
                            assert_cleanup(&failure.cleanup);
                            failure.primary.expect("independent primary")
                        }
                        other => panic!("expected primary plus cleanup: {other:?}"),
                    }
                } else {
                    error
                }
            }
            Route::Apply => {
                let settlement =
                    owner.apply_current_controlled(source, &binding, &op, &mut artifacts);
                assert_eq!(settlement.commit, CommitEvidence::NotCommitted);
                assert_eq!(settlement.artifacts, ArtifactDisposition::None);
                assert_eq!(
                    settlement.rollback,
                    if deny_rollback {
                        RollbackDisposition::Unresolved
                    } else if second {
                        RollbackDisposition::Explicit
                    } else {
                        RollbackDisposition::NotApplicable
                    }
                );
                assert_eq!(
                    settlement.health,
                    if deny_rollback {
                        WriterHealth::Quarantined(
                            crate::managed_catalogue::QuarantineReason::UncertainSettlement,
                        )
                    } else {
                        WriterHealth::Healthy
                    }
                );
                if deny_rollback {
                    assert_cleanup(&settlement.cleanup);
                } else {
                    assert!(settlement.cleanup.is_empty());
                }
                settlement.failure.expect("independent primary")
            }
        };
        assert_read_error(error, index, ignore);
        assert_eq!(native.reads.get(), if second { 2 } else { 1 });
        assert_eq!(
            native.denied_in_transaction.get(),
            Some(second),
            "second read must see actual W transaction"
        );
        let mut events = if second {
            expected_reads(4)
        } else {
            Vec::new()
        };
        if second {
            events.push(Event::Transaction("BEGIN"));
        }
        events.extend(expected_reads(index));
        if second {
            events.push(Event::Transaction("ROLLBACK"));
        }
        assert_eq!(
            *native.events.borrow(),
            events,
            "ordered W reads and inspected rollback"
        );
        assert!(!native.hook_conflict.get());
        assert_eq!(native.changes.get(), 0);
        assert_eq!(artifacts.counter, 0);
        assert!(!artifacts.inventory.allocated(DestinationRole::Recovery));
        assert!(!artifacts.inventory.allocated(DestinationRole::Rehearsal));
        assert_eq!(
            unsafe { rusqlite::ffi::sqlite3_get_autocommit(native.db.get()) } == 0,
            deny_rollback
        );
        assert_eq!(
            unsafe { rusqlite::ffi::sqlite3_total_changes64(native.db.get()) } as u64,
            before.1
        );
        native.detach();
        if deny_rollback {
            assert_eq!(
                health(&owner),
                WriterHealth::Quarantined(
                    crate::managed_catalogue::QuarantineReason::UncertainSettlement
                )
            );
            // Quarantined ownership intentionally refuses all ordinary access;
            // only test teardown ends the untouched fixture transaction at Drop.
            assert!(owner.with_read(|_| Ok::<_, rusqlite::Error>(())).is_err());
        } else {
            owner
                .with_read(|conn| {
                    assert_eq!((flags(conn), conn.total_changes()), before);
                    assert_eq!(scalar(conn, "max_page_count"), cap);
                    Ok::<_, rusqlite::Error>(())
                })
                .unwrap()
                .unwrap();
            assert_eq!(
                owner.capture_stamp(&PublicationGate::new()).unwrap(),
                binding.catalogue
            );
        }
    }
}
fn assert_cleanup(cleanup: &[PreviewCleanupError]) {
    assert_eq!(
        cleanup.len(),
        2,
        "no discarded or invented cleanup facts: {cleanup:?}"
    );
    assert!(matches!(
        &cleanup[0],
        PreviewCleanupError::WriterRollback(rusqlite::Error::SqliteFailure(
            rusqlite::ffi::Error {
                code: rusqlite::ErrorCode::AuthorizationForStatementDenied,
                ..
            },
            _
        ))
    ));
    assert!(matches!(
        cleanup[1],
        PreviewCleanupError::WriterNotAutocommit
    ));
}
macro_rules! denied {
    ($name:ident, $route:ident, $second:expr, $ignore:expr, $rollback:expr) => {
        #[test]
        fn $name() {
            read_refusal(Route::$route, $second, $ignore, $rollback);
        }
    };
}
denied!(preview_denied_initial_reads, Preview, false, false, false);
denied!(apply_denied_initial_reads, Apply, false, false, false);
denied!(preview_missing_initial_rows, Preview, false, true, false);
denied!(apply_missing_initial_rows, Apply, false, true, false);
denied!(preview_denied_reserved_reads, Preview, true, false, false);
denied!(apply_denied_reserved_reads, Apply, true, false, false);
denied!(preview_missing_reserved_rows, Preview, true, true, false);
denied!(apply_missing_reserved_rows, Apply, true, true, false);
denied!(
    preview_reserved_and_rollback_denied,
    Preview,
    true,
    false,
    true
);
denied!(apply_reserved_and_rollback_denied, Apply, true, false, true);

struct PositiveArtifacts {
    inner: TestArtifacts,
    phases: Vec<&'static str>,
}
impl PositiveArtifacts {
    fn new() -> Self {
        Self {
            inner: TestArtifacts::new(),
            phases: Vec::new(),
        }
    }
}
impl CatalogueArtifacts for PositiveArtifacts {
    fn inventory(&mut self) -> &mut ArtifactInventory {
        &mut self.inner.inventory
    }
    fn fresh_rehearsal(&mut self, page_size: i64) -> Result<FreshDestination, OrchestrationError> {
        self.phases.push("D");
        self.inner.fresh_rehearsal(page_size)
    }
    fn fresh_recovery(&mut self, page_size: i64) -> Result<FreshDestination, OrchestrationError> {
        self.phases.push("B");
        self.inner.fresh_recovery(page_size)
    }
    fn before_rehearsal(&mut self, _: &crate::catalogue_copy::RehearsalDestination) {
        self.phases.push("rehearsal");
    }
    fn before_live_install(&mut self, writer: &crate::Connection) {
        self.phases.push("live");
        assert_eq!(flags(writer), vec![1, 0, 0, 0, 0]);
        assert!(!writer.is_autocommit());
        assert!(self
            .inner
            .inventory
            .retained_connection(DestinationRole::Rehearsal)
            .is_none());
        assert!(self
            .inner
            .inventory
            .retained_connection(DestinationRole::Recovery)
            .is_none());
    }
}
#[test]
fn accepted_actual_preview_and_apply_commit_once() {
    let (_dir, mut owner) = migrated_owner();
    let bytes = valid_package();
    let native = Native::attach(&owner);
    let mut preview_artifacts = PositiveArtifacts::new();
    let binding = owner
        .preview_current_controlled(
            admit("configuration.knxprod", &bytes),
            &OperationContext {
                gate: &PublicationGate::new(),
                policy: ControlledPolicy::Standard,
            },
            &mut preview_artifacts,
        )
        .unwrap()
        .binding;
    assert_eq!(preview_artifacts.phases, ["D", "rehearsal"]);
    assert_eq!(native.transactions(), ["BEGIN", "ROLLBACK"]);
    let mut events = expected_reads(4);
    events.push(Event::Transaction("BEGIN"));
    events.extend(expected_reads(4));
    events.push(Event::Transaction("ROLLBACK"));
    assert_eq!(*native.events.borrow(), events);
    assert_eq!(native.changes.get(), 0);
    native.events.borrow_mut().clear();
    let mut artifacts = PositiveArtifacts::new();
    let settlement = owner.apply_current_controlled(
        admit("configuration.knxprod", &bytes),
        &binding,
        &OperationContext {
            gate: &PublicationGate::new(),
            policy: ControlledPolicy::Standard,
        },
        &mut artifacts,
    );
    assert!(settlement.failure.is_none(), "{settlement:?}");
    assert!(settlement.cleanup.is_empty());
    assert_eq!(settlement.commit, CommitEvidence::Committed);
    assert_eq!(settlement.rollback, RollbackDisposition::NotApplicable);
    assert_eq!(settlement.health, WriterHealth::Healthy);
    assert_eq!(settlement.artifacts, ArtifactDisposition::Retain);
    assert_eq!(native.transactions(), ["BEGIN", "COMMIT"]);
    assert_eq!(artifacts.phases, ["D", "B", "rehearsal", "live"]);
    assert!(native.changes.get() > 0);
    let mut prefix = expected_reads(4);
    prefix.push(Event::Transaction("BEGIN"));
    prefix.extend(expected_reads(4));
    assert!(native.events.borrow().starts_with(&prefix));
    assert!(!native
        .events
        .borrow()
        .iter()
        .any(|event| matches!(event, Event::Pragma(name, true) if *name != "max_page_count")));
    assert!(!native.hook_conflict.get());
    native.detach();
    owner
        .with_read(|conn| {
            assert_eq!(flags(conn), vec![1, 0, 0, 0, 0]);
            assert!(conn.is_autocommit());
            Ok::<_, rusqlite::Error>(())
        })
        .unwrap()
        .unwrap();
    assert_eq!(package_count(&owner), 1);
    assert_eq!(
        owner
            .capture_stamp(&PublicationGate::new())
            .unwrap()
            .generation,
        1
    );
}
#[test]
fn valid_preview_keeps_existing_normal_synchronous_policy() {
    let (_dir, mut owner) = migrated_owner();
    owner
        .with_read(|conn| conn.pragma_update(None, "synchronous", 1))
        .unwrap()
        .unwrap();
    let native = Native::attach(&owner);
    let binding = shown(&mut owner, &valid_package());
    assert_eq!(binding.preview.package_count, 1);
    assert_eq!(native.transactions(), ["BEGIN", "ROLLBACK"]);
    native.detach();
    owner
        .with_read(|conn| {
            assert_eq!(scalar(conn, "synchronous"), 1);
            Ok::<_, rusqlite::Error>(())
        })
        .unwrap()
        .unwrap();
}
