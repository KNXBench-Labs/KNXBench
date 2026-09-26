//! MP §2.3 `NM_IndividualAddress_Write` against real hardware — the first real write.
//!
//! **This test changes a physical device.** It is `#[ignore]`d and additionally
//! gated on an explicit environment variable, so neither `cargo test` nor a CI
//! run nor a careless `--ignored` sweep can perform it by accident:
//!
//! ```text
//! KNX_GATEWAY=<host>:3671 KNX_WRITE_NEW_ADDRESS=1.1.67 \
//!     cargo test -p knx-net --test live_individual_address_write \
//!     -- --ignored --nocapture
//! ```
//!
//! The new address is read from the environment rather than hard-coded, for the
//! same reason the gateway is: the value is one operator's decision about one
//! installation, and a literal in a public repository would be somebody else's
//! device next time.
//!
//! Preconditions this test checks itself, before sending anything:
//!
//! - the new address is not the project-excluded alarm panel (checked against
//!   `EXCLUDED_INDIVIDUAL_ADDRESSES`, which `WriteAuthorisation` also enforces);
//! - exactly one device is in Programming Mode, established by MP §2.2's
//!   broadcast read waited out in full. `individual_address_write` re-verifies
//!   this itself at step 2 and again immediately before the write at step 3 —
//!   this pass is a legible pre-flight, not the load-bearing check.
//!
//! What it then runs is the shipped procedure, unmodified:
//! `individual_address_write` walks MP §2.3's four steps — probe `IA_new` for
//! occupancy, count Programming Mode responders, broadcast
//! `A_IndividualAddress_Write` behind a `ProgrammingModeWitness`, then connect
//! to the new address, verify it answers and `A_Restart` it. Afterwards this
//! test independently confirms the device now answers at the new address.
//!
//! The two authorisations are hardware authorisations carrying the operator's
//! confirmation phrase verbatim, which is what `hardware_write_is_authorised`
//! permits for exactly these two scopes.

use std::net::SocketAddrV4;

use knx_core::commissioning::authorisation::AuthorisationPlan;
use knx_core::commissioning::mutation::{
    required_confirmation_phrase, WriteAuthorisation, WriteScope,
};
use knx_core::commissioning::programming_mode::INDIVIDUAL_ADDRESS_READ_TIMEOUT;
use knx_core::commissioning::properties::{ObjectIndex, PID_MANUFACTURER_ID};
use knx_core::{IndividualAddress, EXCLUDED_INDIVIDUAL_ADDRESSES};
use knx_net::commissioning::individual_address_write::individual_address_write;
use knx_net::{BusConnection, KnxNetIpClient, ManagementSession, SessionTiming};

fn gateway_addr() -> SocketAddrV4 {
    std::env::var("KNX_GATEWAY")
        .expect("set KNX_GATEWAY to your gateway as host:port, e.g. 192.0.2.1:3671")
        .parse()
        .expect("KNX_GATEWAY must be host:port, e.g. 192.0.2.1:3671")
}

/// The second, independent opt-in. `--ignored` alone must not be enough to
/// change a device, so this test refuses to run without the operator having
/// named the address they want assigned.
fn new_address() -> IndividualAddress {
    std::env::var("KNX_WRITE_NEW_ADDRESS")
        .expect(
            "set KNX_WRITE_NEW_ADDRESS to the address to assign, e.g. 1.1.67 — \
             this test writes to real hardware and will not guess it",
        )
        .parse()
        .expect("KNX_WRITE_NEW_ADDRESS must be an individual address, e.g. 1.1.67")
}

#[tokio::test]
#[ignore = "changes a real device; needs KNX_GATEWAY and KNX_WRITE_NEW_ADDRESS"]
async fn assigns_the_new_address_to_the_device_in_programming_mode() {
    let new = new_address();
    assert!(
        !EXCLUDED_INDIVIDUAL_ADDRESSES.contains(&new),
        "refusing to write the project-excluded address {new}"
    );
    println!("target new address: {new}");

    let client = KnxNetIpClient::new();
    let tunnel = client
        .connect_tunnel(gateway_addr())
        .await
        .expect("connect_tunnel against the real gateway");

    // ---- Pre-flight: who is in Programming Mode, and what is their address? ----
    // Read-only, and reported before anything is written, so the log says what
    // the device looked like beforehand.
    let before = {
        let scout = ManagementSession::read_only(
            &tunnel,
            new,
            AuthorisationPlan::Skip,
            SessionTiming::default(),
        )
        .expect("a read-only session must build");
        let responders = scout
            .broadcast_individual_address_read(INDIVIDUAL_ADDRESS_READ_TIMEOUT)
            .await
            .expect("the broadcast programming-mode read must complete");
        println!(
            "pre-flight: {} device(s) in programming mode from {} frame(s)",
            responders.device_count(),
            responders.frame_count()
        );
        for device in responders.devices() {
            println!("  in programming mode: {device}");
        }
        let witness = responders
            .single_responder()
            .expect("exactly one device must be in programming mode before writing");
        witness.current_address()
    };
    println!("pre-flight: the device to be readdressed is currently {before}");

    if before == new {
        println!("the device already holds {new}; the procedure will skip its write step");
    }

    // ---- The write ----
    let programming = WriteAuthorisation::for_hardware(
        new,
        WriteScope::IndividualAddressProgramming,
        &required_confirmation_phrase(new, WriteScope::IndividualAddressProgramming),
    )
    .expect("the confirmation phrase is the required one");
    let restart = WriteAuthorisation::for_hardware(
        new,
        WriteScope::Restart,
        &required_confirmation_phrase(new, WriteScope::Restart),
    )
    .expect("the confirmation phrase is the required one");

    let report = individual_address_write(
        &tunnel,
        AuthorisationPlan::Skip,
        SessionTiming::default(),
        new,
        programming,
        restart,
    )
    .await
    .expect("NM_IndividualAddress_Write must complete");

    println!(
        "occupancy of {new} before the write: {:?}",
        report.occupancy
    );
    println!("step 3 broadcast the address write: {}", report.wrote);
    for step in &report.steps {
        println!("  step {}: {}", step.number, step.title);
    }

    // ---- Independent confirmation, after the procedure's own step 4 ----
    // A fresh session, so this is not the procedure vouching for itself.
    let mut check = ManagementSession::read_only(
        &tunnel,
        new,
        AuthorisationPlan::Skip,
        SessionTiming::default(),
    )
    .expect("a read-only session for the new address must build");
    check
        .connect()
        .await
        .expect("the device must answer at its new address");
    match check.read_mask_version().await {
        Ok(mask) => println!("after the write, {new} reports mask {mask}"),
        Err(err) => panic!("the device did not identify itself at {new}: {err}"),
    }
    match check
        .read_property(ObjectIndex::DEVICE, PID_MANUFACTURER_ID, 1, 1)
        .await
    {
        Ok(bytes) => println!("after the write, {new} reports manufacturer {bytes:02x?}"),
        Err(err) => println!("after the write, manufacturer read: {err}"),
    }
    check.disconnect().await;

    tunnel.disconnect().await.expect("clean tunnel disconnect");
}
