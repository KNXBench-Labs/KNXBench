//! Focused tests for the unwired catalogue job admission and replay manager.

use super::{
    AdmissionState, Capability, CatalogueJobManager, CreateBody, CreateOutcome, DisposeOutcome,
    Handle, Incarnation, JobId, Profile, U64,
};

fn incarnation() -> Incarnation {
    Incarnation::from_bytes([0x01_u8; 16])
}

fn body(serial: u64) -> CreateBody {
    CreateBody {
        incarnation: incarnation(),
        serial: U64(serial),
        source_name: "package.knxprod".to_string(),
        declared_bytes: U64(1024),
        profile: Profile::Standard,
        allow_large_package: false,
    }
}

fn manager() -> CatalogueJobManager {
    CatalogueJobManager::new(incarnation())
}

fn expect_fresh(outcome: CreateOutcome) -> JobId {
    match outcome {
        CreateOutcome::Fresh(job) => job,
        other => panic!("expected Fresh, got {other:?}"),
    }
}

fn handle(serial: u64, job: JobId) -> Handle {
    Handle {
        incarnation: incarnation(),
        serial: U64(serial),
        job,
    }
}

#[test]
fn admission_starts_open_at_serial_one() {
    let manager = manager();
    let admission = manager.admission();
    assert_eq!(admission.incarnation, incarnation());
    assert_eq!(admission.next_serial, U64::ONE);
    assert_eq!(admission.state, AdmissionState::Open);
}

#[test]
fn fresh_create_allocates_serial_one_and_advances() {
    let mut manager = manager();
    let job = expect_fresh(manager.create(&Capability::mint(), body(1)));
    assert_eq!(job.to_string().len(), 32);
    assert_eq!(manager.admission().next_serial, U64(2));
    assert!(manager.active_job().is_some());
}

#[test]
fn exact_retained_replay_returns_the_same_job() {
    let mut manager = manager();
    let capability = Capability::mint();
    let job = expect_fresh(manager.create(&capability, body(1)));
    assert_eq!(
        manager.create(&capability, body(1)),
        CreateOutcome::Replay(job)
    );
    assert_eq!(manager.admission().next_serial, U64(2));
}

#[test]
fn conflicting_metadata_for_a_retained_serial_is_a_conflict() {
    let mut manager = manager();
    let capability = Capability::mint();
    expect_fresh(manager.create(&capability, body(1)));
    let mut changed = body(1);
    changed.declared_bytes = U64(2048);
    assert_eq!(
        manager.create(&capability, changed),
        CreateOutcome::CommandConflict
    );
}

#[test]
fn capability_mismatch_on_a_retained_serial_is_a_generic_unavailable() {
    let mut manager = manager();
    let capability = Capability::mint();
    expect_fresh(manager.create(&capability, body(1)));
    assert_eq!(
        manager.create(&Capability::mint(), body(1)),
        CreateOutcome::OutcomeUnavailable
    );
}

#[test]
fn consumed_forgotten_serial_below_next_is_unavailable() {
    let mut manager = manager();
    manager.set_next_serial(U64(10));
    assert_eq!(
        manager.create(&Capability::mint(), body(5)),
        CreateOutcome::OutcomeUnavailable
    );
    assert_eq!(manager.admission().next_serial, U64(10));
}

#[test]
fn future_serial_is_a_stale_offer() {
    let mut manager = manager();
    assert_eq!(
        manager.create(&Capability::mint(), body(2)),
        CreateOutcome::StaleOffer
    );
    assert_eq!(manager.admission().next_serial, U64::ONE);
}

#[test]
fn prior_incarnation_is_refused() {
    let mut manager = manager();
    let mut old = body(1);
    old.incarnation = Incarnation::from_bytes([0x02_u8; 16]);
    assert_eq!(
        manager.create(&Capability::mint(), old),
        CreateOutcome::IncarnationChanged
    );
    assert_eq!(manager.admission().next_serial, U64::ONE);
}

#[test]
fn active_slot_refusal_does_not_consume_the_serial() {
    let mut manager = manager();
    expect_fresh(manager.create(&Capability::mint(), body(1)));
    assert_eq!(
        manager.create(&Capability::mint(), body(2)),
        CreateOutcome::JobBusy
    );
    assert_eq!(manager.admission().next_serial, U64(2));
}

#[test]
fn stopping_refuses_fresh_admission_without_consuming() {
    let mut manager = manager();
    manager.close_admission();
    assert_eq!(
        manager.create(&Capability::mint(), body(1)),
        CreateOutcome::Stopping
    );
    assert_eq!(manager.admission().next_serial, U64::ONE);
}

#[test]
fn exhausted_refuses_fresh_admission() {
    let mut manager = manager();
    manager.exhaust();
    assert_eq!(
        manager.create(&Capability::mint(), body(1)),
        CreateOutcome::Exhausted
    );
}

#[test]
fn busy_and_unavailable_states_refuse_as_busy() {
    for state in [
        AdmissionState::Busy,
        AdmissionState::ArtifactStoreUnavailable,
        AdmissionState::CatalogueUnavailable,
    ] {
        let mut manager = manager();
        manager.set_admission(state);
        assert_eq!(
            manager.create(&Capability::mint(), body(1)),
            CreateOutcome::JobBusy
        );
        assert_eq!(manager.admission().next_serial, U64::ONE);
    }
}

#[test]
fn serial_exhaustion_closes_admission_and_never_wraps() {
    let mut manager = manager();
    manager.set_next_serial(U64::MAX);
    assert_eq!(
        manager.create(&Capability::mint(), body(u64::MAX)),
        CreateOutcome::Exhausted
    );
    assert_eq!(manager.admission().state, AdmissionState::Exhausted);
    // MAX was never allocated: next stays at the exhausted sentinel.
    assert_eq!(manager.admission().next_serial, U64::MAX);
}

#[test]
fn full_terminal_ring_evicts_oldest_into_a_tombstone() {
    let mut manager = manager();
    for serial in 1..=8_u64 {
        expect_fresh(manager.create(&Capability::mint(), body(serial)));
        manager.finish_active(true);
    }
    assert_eq!(manager.terminal_count(), 8);
    let first_terminal = manager.oldest_terminal_serial();
    assert_eq!(first_terminal, Some(U64(1)));
    expect_fresh(manager.create(&Capability::mint(), body(9)));
    assert_eq!(manager.terminal_count(), 7);
    assert_eq!(manager.tombstone_count(), 1);
    assert_eq!(manager.oldest_tombstone_serial(), first_terminal);
    assert_eq!(manager.oldest_tombstone_disposed(), Some(false));
}

#[test]
fn incomplete_cleanup_blocks_eviction_as_busy_without_consuming() {
    let mut manager = manager();
    for serial in 1..=8_u64 {
        expect_fresh(manager.create(&Capability::mint(), body(serial)));
        manager.finish_active(false);
    }
    assert_eq!(manager.terminal_count(), 8);
    assert_eq!(manager.oldest_terminal_cleanup_complete(), Some(false));
    assert_eq!(
        manager.create(&Capability::mint(), body(9)),
        CreateOutcome::JobBusy
    );
    assert_eq!(manager.admission().next_serial, U64(9));
    assert_eq!(manager.terminal_count(), 8);
    assert_eq!(manager.tombstone_count(), 0);
}

#[test]
fn tombstone_ring_evicts_the_oldest_tombstone_first() {
    let mut manager = manager();
    // Fill terminals to 8, then each further create evicts the oldest terminal
    // into the tombstone ring; once that ring is full the oldest tombstone
    // (serial 1) is dropped first.
    for serial in 1..=17_u64 {
        expect_fresh(manager.create(&Capability::mint(), body(serial)));
        manager.finish_active(true);
    }
    assert_eq!(manager.tombstone_count(), 8);
    // Serial 1 (the first eviction) was itself evicted from the tombstone ring.
    assert_eq!(manager.oldest_tombstone_serial(), Some(U64(2)));
}

#[test]
fn dispose_active_is_invalid_phase() {
    let mut manager = manager();
    let capability = Capability::mint();
    let job = expect_fresh(manager.create(&capability, body(1)));
    assert_eq!(
        manager.dispose(&handle(1, job), &capability),
        DisposeOutcome::InvalidPhase
    );
}

#[test]
fn dispose_terminal_with_cleanup_records_a_disposed_tombstone() {
    let mut manager = manager();
    let capability = Capability::mint();
    let job = expect_fresh(manager.create(&capability, body(1)));
    manager.finish_active(true);
    assert_eq!(
        manager.dispose(&handle(1, job), &capability),
        DisposeOutcome::Disposed
    );
    assert_eq!(manager.tombstone_count(), 1);
    assert_eq!(manager.oldest_tombstone_disposed(), Some(true));
}

#[test]
fn dispose_terminal_with_unfinished_cleanup_is_invalid_phase() {
    let mut manager = manager();
    let capability = Capability::mint();
    let job = expect_fresh(manager.create(&capability, body(1)));
    manager.finish_active(false);
    assert_eq!(
        manager.dispose(&handle(1, job), &capability),
        DisposeOutcome::InvalidPhase
    );
    assert_eq!(manager.tombstone_count(), 0);
}

#[test]
fn disposing_a_disposed_tombstone_is_idempotent() {
    let mut manager = manager();
    let capability = Capability::mint();
    let job = expect_fresh(manager.create(&capability, body(1)));
    manager.finish_active(true);
    assert_eq!(
        manager.dispose(&handle(1, job), &capability),
        DisposeOutcome::Disposed
    );
    assert_eq!(
        manager.dispose(&handle(1, job), &capability),
        DisposeOutcome::Disposed
    );
    assert_eq!(manager.tombstone_count(), 1);
}

#[test]
fn an_evicted_tombstone_never_pretends_delete_succeeded() {
    let mut manager = manager();
    let capability = Capability::mint();
    let job = expect_fresh(manager.create(&capability, body(1)));
    manager.finish_active(true);
    // Fill terminals to 8; the next create evicts serial 1 into a tombstone
    // through count eviction (not an explicit dispose).
    for serial in 2..=8_u64 {
        expect_fresh(manager.create(&Capability::mint(), body(serial)));
        manager.finish_active(true);
    }
    expect_fresh(manager.create(&Capability::mint(), body(9)));
    assert_eq!(manager.tombstone_count(), 1);
    assert_eq!(manager.oldest_tombstone_disposed(), Some(false));
    // The evicted tombstone cannot answer a dispose with success.
    assert_eq!(
        manager.dispose(&handle(1, job), &capability),
        DisposeOutcome::OutcomeUnavailable
    );
}

#[test]
fn dispose_forgotten_handle_is_unavailable() {
    let mut manager = manager();
    let handle = Handle {
        incarnation: incarnation(),
        serial: U64(7),
        job: JobId::from_bytes([0x07_u8; 16]),
    };
    assert_eq!(
        manager.dispose(&handle, &Capability::mint()),
        DisposeOutcome::OutcomeUnavailable
    );
}

#[test]
fn dispose_wrong_capability_is_unavailable() {
    let mut manager = manager();
    let capability = Capability::mint();
    let job = expect_fresh(manager.create(&capability, body(1)));
    manager.finish_active(true);
    assert_eq!(
        manager.dispose(&handle(1, job), &Capability::mint()),
        DisposeOutcome::OutcomeUnavailable
    );
    assert_eq!(manager.tombstone_count(), 0);
}

#[test]
fn dispose_under_quarantine_is_refused_without_amnesty() {
    let mut manager = manager();
    let capability = Capability::mint();
    let job = expect_fresh(manager.create(&capability, body(1)));
    manager.finish_active(true);
    manager.mark_artifact_store_unavailable();
    assert_eq!(
        manager.dispose(&handle(1, job), &capability),
        DisposeOutcome::ArtifactStoreUnavailable
    );
    assert_eq!(manager.tombstone_count(), 0);
}
