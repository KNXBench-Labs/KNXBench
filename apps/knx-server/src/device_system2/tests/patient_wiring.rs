//! The System2 worker connects with the patient ACK wait and runs on the patient session timing.

use std::time::Duration;

/// The shared constructor, used by the server and the desktop shell alike, gives System2 the
/// patient profile and leaves every ordinary operation on the standard figures. Literals, so a
/// wrong constant cannot agree with its own test.
#[test]
fn a_new_app_state_gives_system2_the_patient_profile_and_nothing_else() {
    let directory = tempfile::tempdir().unwrap();
    let state = crate::AppState::with_product_db_at(directory.path().into(), None);
    assert_eq!(
        state.system2_timing.response_timeout,
        Duration::from_secs(3)
    );
    assert_eq!(
        state.system2_timing.answer_timeout,
        Some(Duration::from_secs(15))
    );
    assert_eq!(
        state.system2_timing.max_transition,
        Duration::from_secs(120)
    );
    assert_eq!(
        state.system2_timing.connection_timeout,
        Duration::from_secs(6)
    );
    assert_eq!(state.system2_timing.poll_interval, Duration::from_secs(3));
    for ordinary in [
        state.device_download_timing,
        state.address_programming_timing,
    ] {
        assert_eq!(ordinary.response_timeout, Duration::from_secs(3));
        assert_eq!(ordinary.answer_timeout, None);
        assert_eq!(ordinary.max_transition, Duration::from_secs(30));
    }
}

/// Source-bound: the worker's two tunnel connections ask for the patient ACK wait, none uses the
/// plain connector call, and its three session runs use `system2_timing` and not the ordinary
/// download timing. A behavioural run needs the digest-pinned public package of the mounted tests.
#[test]
fn the_worker_connects_patiently_and_runs_on_the_system2_timing() {
    let worker: String = include_str!("../worker.rs").split_whitespace().collect();
    let count = |needle: &str| worker.matches(needle).count();
    assert_eq!(
        count("connector.connect_tunnel_with_ack_wait(gateway,TunnelAckWait::Patient)"),
        2,
        "both connections are patient"
    );
    assert_eq!(count(".connect_tunnel("), 0, "no plain connection");
    assert_eq!(count("state.system2_timing"), 3, "three session runs");
    assert_eq!(
        count("device_download_timing"),
        0,
        "none on the ordinary timing"
    );
}
