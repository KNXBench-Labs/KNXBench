//! T30 phase 3: read-only commissioning probes against the real installation, skipping `1.1.220`.
//!
//! Integration test against a real KNXnet/IP gateway, like `live_gateway.rs`
//! — requires actual hardware reachable on the LAN, never runs in CI, and
//! never runs unattended: `cargo test -p knx-net --test
//! live_commissioning_readonly -- --ignored --nocapture`.
//!
//! Scope is exactly commissioning spec §2.2 (R-SAFE-2): individual
//! addresses `1.1.24`–`1.1.32`, nine of them, named literally below and
//! nowhere iterated as a range. Every session this test opens is built with
//! [`ManagementSession::read_only`] and [`AuthorisationPlan::Skip`], so no
//! `A_Authorize_Request` and no write of any kind reaches the wire — the
//! only Application Layer services used are `A_DeviceDescriptor_Read` and
//! `A_PropertyValue_Read` (the latter for `PID_MANUFACTURER_ID`,
//! `PID_HARDWARE_TYPE`, `PID_PROGRAM_VERSION` and `PID_LOAD_STATE_CONTROL`).
//! `ManagementSession::build` refuses a project-excluded target on its own
//! (`knx_core::ContactableAddress::new`); the assertions below are a second,
//! redundant check that fails before any socket opens rather than relying on
//! that alone.
//!
//! This test's own output, captured with `--nocapture`, is the raw material
//! for `docs/RESEARCH.md`'s phase-3 findings — it prints one block per
//! address and never summarises a non-answer away.

use std::net::SocketAddrV4;

use knx_core::commissioning::authorisation::AuthorisationPlan;
use knx_core::commissioning::properties::{
    ObjectIndex, PID_HARDWARE_TYPE, PID_MANUFACTURER_ID, PID_PROGRAM_VERSION,
};
use knx_core::{IndividualAddress, EXCLUDED_INDIVIDUAL_ADDRESSES};
use knx_net::{BusConnection, KnxNetIpClient, ManagementSession, SessionTiming};

// No default gateway address, deliberately, matching `live_gateway.rs`: a
// hard-coded one would be somebody's real installation written into a
// public repository.
fn gateway_addr() -> SocketAddrV4 {
    std::env::var("KNX_GATEWAY")
        .expect("set KNX_GATEWAY to your gateway as host:port, e.g. 192.0.2.1:3671")
        .parse()
        .expect("KNX_GATEWAY must be host:port, e.g. 192.0.2.1:3671")
}

/// The nine addresses commissioning spec §2.2 approves for active reads,
/// named one at a time. Not a range: `1.1.220`, the alarm panel between
/// `32` and the rest of the line, must never be constructible from this
/// list by widening it.
fn approved_targets() -> Vec<IndividualAddress> {
    [
        "1.1.24", "1.1.25", "1.1.26", "1.1.27", "1.1.28", "1.1.29", "1.1.30", "1.1.31", "1.1.32",
    ]
    .iter()
    .map(|s| s.parse().expect("literal address must parse"))
    .collect()
}

#[tokio::test]
#[ignore = "needs the real installation's gateway; run explicitly with KNX_GATEWAY set"]
async fn probes_the_nine_approved_addresses_read_only() {
    let alarm_panel: IndividualAddress = "1.1.220".parse().expect("literal address must parse");
    assert!(
        EXCLUDED_INDIVIDUAL_ADDRESSES.contains(&alarm_panel),
        "the alarm panel must still be in the exclusion set before this test does anything"
    );

    let targets = approved_targets();
    assert_eq!(targets.len(), 9, "spec §2.2 names exactly nine addresses");
    for target in &targets {
        assert_ne!(
            *target, alarm_panel,
            "the alarm panel must never be a probe target"
        );
    }

    let client = KnxNetIpClient::new();
    let tunnel = client
        .connect_tunnel(gateway_addr())
        .await
        .expect("connect_tunnel against the real gateway");

    for target in targets {
        println!("== {target} ==");
        let mut session = ManagementSession::read_only(
            &tunnel,
            target,
            AuthorisationPlan::Skip,
            SessionTiming::default(),
        )
        .expect("read_only session for an approved address must build");

        if let Err(err) = session.connect().await {
            println!("  connect: {err}");
            continue;
        }

        match session.read_mask_version().await {
            Ok(mask) => println!("  mask version: {mask}"),
            Err(err) => println!("  mask version: {err}"),
        }
        match session
            .read_property(ObjectIndex::DEVICE, PID_MANUFACTURER_ID, 1, 1)
            .await
        {
            Ok(bytes) => println!("  PID_MANUFACTURER_ID: {bytes:02x?}"),
            Err(err) => println!("  PID_MANUFACTURER_ID: {err}"),
        }
        match session
            .read_property(ObjectIndex::DEVICE, PID_HARDWARE_TYPE, 1, 1)
            .await
        {
            Ok(bytes) => println!("  PID_HARDWARE_TYPE: {bytes:02x?}"),
            Err(err) => println!("  PID_HARDWARE_TYPE: {err}"),
        }
        match session
            .read_property(ObjectIndex::DEVICE, PID_PROGRAM_VERSION, 1, 1)
            .await
        {
            Ok(bytes) => println!("  PID_PROGRAM_VERSION: {bytes:02x?}"),
            Err(err) => println!("  PID_PROGRAM_VERSION: {err}"),
        }
        for (name, object_index) in [
            ("Address Table Object", ObjectIndex::ADDRESS_TABLE),
            ("Association Table Object", ObjectIndex::ASSOCIATION_TABLE),
            (
                "Application Program Object",
                ObjectIndex::APPLICATION_PROGRAM,
            ),
        ] {
            match session.read_load_state(object_index).await {
                Ok(state) => println!("  load state, {name}: {state}"),
                Err(err) => println!("  load state, {name}: {err}"),
            }
        }

        session.disconnect().await;
    }

    tunnel.disconnect().await.expect("clean tunnel disconnect");
}
