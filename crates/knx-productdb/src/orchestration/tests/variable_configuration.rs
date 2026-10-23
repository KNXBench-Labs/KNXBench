//! Variable W configuration belongs to immutable consent, never silent repair.
use super::*;

#[test]
fn unreadable_variable_configuration_refuses_initial_and_reserved_getters() {
    use std::cell::Cell;
    use std::ffi::{c_char, c_void, CStr};
    struct Fault {
        target: &'static [u8],
        reads: Cell<usize>,
        deny: usize,
        setters: Cell<usize>,
    }
    unsafe extern "C" fn authorize(
        context: *mut c_void,
        action: i32,
        first: *const c_char,
        second: *const c_char,
        _: *const c_char,
        _: *const c_char,
    ) -> i32 {
        let fault = unsafe { &*context.cast::<Fault>() };
        if action == rusqlite::ffi::SQLITE_PRAGMA
            && !first.is_null()
            && unsafe { CStr::from_ptr(first) }.to_bytes() == fault.target
        {
            if !second.is_null() {
                fault.setters.set(fault.setters.get() + 1);
            } else {
                let reads = fault.reads.get() + 1;
                fault.reads.set(reads);
                if reads == fault.deny {
                    return rusqlite::ffi::SQLITE_DENY;
                }
            }
        }
        rusqlite::ffi::SQLITE_OK
    }
    for target in [
        b"trusted_schema".as_slice(),
        b"recursive_triggers".as_slice(),
    ] {
        for deny in [1, 2] {
            let (_dir, mut owner) = migrated_owner();
            let bytes = valid_package();
            let binding = owner
                .preview_current_controlled(
                    admit("variable.knxprod", &bytes),
                    &OperationContext {
                        gate: &PublicationGate::new(),
                        policy: ControlledPolicy::Standard,
                    },
                    &mut TestArtifacts::new(),
                )
                .unwrap()
                .binding;
            let fault = Fault {
                target,
                reads: Cell::new(0),
                deny,
                setters: Cell::new(0),
            };
            let db = owner
                .with_read(|c| Ok::<_, ()>(unsafe { c.handle() }))
                .unwrap()
                .unwrap();
            unsafe {
                rusqlite::ffi::sqlite3_set_authorizer(
                    db,
                    Some(authorize),
                    (&fault as *const Fault).cast_mut().cast(),
                );
            }
            let mut artifacts = TestArtifacts::new();
            let result = owner.apply_current_controlled(
                admit("variable.knxprod", &bytes),
                &binding,
                &OperationContext {
                    gate: &PublicationGate::new(),
                    policy: ControlledPolicy::Standard,
                },
                &mut artifacts,
            );
            // Detach before assertions/teardown; the native callback never unwinds.
            unsafe {
                rusqlite::ffi::sqlite3_set_authorizer(db, None, std::ptr::null_mut());
            }
            assert!(matches!(
                result.failure,
                Some(OrchestrationError::Sqlite(_))
            ));
            assert_eq!(fault.reads.get(), deny);
            assert_eq!(
                fault.setters.get(),
                0,
                "admission never repairs variable flags"
            );
            assert_eq!(artifacts.counter, 0);
            assert_eq!(
                result.rollback,
                if deny == 1 {
                    RollbackDisposition::NotApplicable
                } else {
                    RollbackDisposition::Explicit
                }
            );
            assert_eq!(result.commit, CommitEvidence::NotCommitted);
            assert_eq!(package_count(&owner), 0);
        }
    }
}

#[test]
fn changed_variable_writer_configuration_refuses_before_copy_or_live_dml() {
    for setting in ["trusted_schema", "recursive_triggers"] {
        let (_dir, mut owner) = migrated_owner();
        let bytes = valid_package();
        let binding = owner
            .preview_current_controlled(
                admit("variable.knxprod", &bytes),
                &OperationContext {
                    gate: &PublicationGate::new(),
                    policy: ControlledPolicy::Standard,
                },
                &mut TestArtifacts::new(),
            )
            .unwrap()
            .binding;
        owner
            .with_read(|c| {
                let before: i64 = c.pragma_query_value(None, setting, |r| r.get(0))?;
                c.pragma_update(None, setting, 1 - before)
            })
            .unwrap()
            .unwrap();
        let mut artifacts = TestArtifacts::new();
        let result = owner.apply_current_controlled(
            admit("variable.knxprod", &bytes),
            &binding,
            &OperationContext {
                gate: &PublicationGate::new(),
                policy: ControlledPolicy::Standard,
            },
            &mut artifacts,
        );
        assert!(
            matches!(result.failure, Some(OrchestrationError::Stale)),
            "variable configuration drift must stale consent"
        );
        assert_eq!(result.commit, CommitEvidence::NotCommitted);
        assert_eq!(artifacts.counter, 0, "refusal must precede D/B allocation");
        assert_eq!(package_count(&owner), 0);
    }
}

#[test]
fn both_observed_variable_values_are_supported_without_repair() {
    for value in [0_i64, 1] {
        let (_dir, mut owner) = migrated_owner();
        owner
            .with_read(|c| {
                c.pragma_update(None, "trusted_schema", value)?;
                c.pragma_update(None, "recursive_triggers", value)
            })
            .unwrap()
            .unwrap();
        let bytes = valid_package();
        let binding = owner
            .preview_current_controlled(
                admit("variable.knxprod", &bytes),
                &OperationContext {
                    gate: &PublicationGate::new(),
                    policy: ControlledPolicy::Standard,
                },
                &mut TestArtifacts::new(),
            )
            .unwrap()
            .binding;
        let result = owner.apply_current_controlled(
            admit("variable.knxprod", &bytes),
            &binding,
            &OperationContext {
                gate: &PublicationGate::new(),
                policy: ControlledPolicy::Standard,
            },
            &mut TestArtifacts::new(),
        );
        assert_eq!(result.commit, CommitEvidence::Committed);
        owner
            .with_read(|c| {
                for setting in ["trusted_schema", "recursive_triggers"] {
                    let actual: i64 = c.pragma_query_value(None, setting, |r| r.get(0))?;
                    assert_eq!(actual, value);
                }
                Ok::<_, rusqlite::Error>(())
            })
            .unwrap()
            .unwrap();
    }
}
