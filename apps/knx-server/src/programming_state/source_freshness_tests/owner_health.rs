//! An unreadable catalogue owner at settlement costs the memory credit, never the callback.

use std::panic::{catch_unwind, AssertUnwindSafe};

use knx_productdb::managed_catalogue::{LegacyMutationKind, LegacySettlementHint};

use super::*;

fn owner(f: &Fixture) -> &CatalogueOwner {
    f.state.product_db.as_ref().unwrap()
}

fn unreadable(f: &Fixture) -> bool {
    owner(f).with_read(|_| Ok::<_, ()>(())).is_err()
}

/// Settles a finished run started before the owner went bad.
fn settle_with(f: &Fixture, spoil: impl FnOnce(&Fixture)) {
    mark(&f.state, ProgrammingParts::ALL, true);
    let prepared = prepare(&f.state);
    let settler = download_settler(&f.state, super::super::incarnation(&f.state), &prepared);
    spoil(f);
    assert!(unreadable(f), "precondition: the owner refuses reads");
    let status = finished();
    settler(&status);
}

#[test]
fn an_owner_quarantined_by_an_uncertain_mutation_settles_without_memory_credit() {
    let f = fixture(&OLD, &OLD);
    settle_with(&f, |f| {
        let result = owner(f).with_legacy_mutation(
            LegacyMutationKind::PathUpdate,
            |_| Ok::<_, ()>(()),
            |_| LegacySettlementHint::Uncertain,
        );
        assert!(result.is_ok(), "the mutation itself returned");
    });
    // The request cannot be re-read either, so not even the address is
    // confirmed; what the earlier belief held before the run is gone, never
    // a fresh memory credit.
    assert_eq!(
        belief(&f.state),
        NONE,
        "an unreadable request credits no part"
    );
}

#[test]
fn an_owner_poisoned_by_a_panicking_mutation_settles_without_panic_or_credit() {
    let f = fixture(&OLD, &OLD);
    settle_with(&f, |f| {
        let unwound = catch_unwind(AssertUnwindSafe(|| {
            let _ = owner(f).with_legacy_mutation(
                LegacyMutationKind::PathUpdate,
                |_| -> Result<(), ()> { panic!("synthetic mutator failure") },
                |_| LegacySettlementHint::Uncertain,
            );
        }));
        assert!(unwound.is_err(), "the mutator panicked");
    });
    assert_eq!(
        belief(&f.state),
        NONE,
        "an unreadable request credits no part"
    );
}
