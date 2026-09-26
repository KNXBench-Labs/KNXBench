//! Read-only programming-mode search against a real gateway (MP §2.2).
//!
//! Integration test against real hardware, like `live_commissioning_readonly.rs`
//! — requires a gateway on the LAN, never runs in CI, never runs unattended:
//!
//! ```text
//! KNX_GATEWAY=<host>:3671 cargo test -p knx-net \
//!     --test live_programming_mode -- --ignored --nocapture
//! ```
//!
//! What it sends, and nothing else:
//!
//! 1. One connectionless broadcast `A_IndividualAddress_Read`
//!    (`ManagementSession::broadcast_individual_address_read`), then it listens
//!    out the full `INDIVIDUAL_ADDRESS_READ_TIMEOUT`. `[D]` MP §2.2: time-out
//!    3 s, and *"The Management Client shall always wait until the time-out has
//!    elapsed."* The wait is never cut short by an early answer, because the
//!    point of it is to count *all* responders, not to find the first.
//! 2. Optionally, for each address that answered and only then, a
//!    connection-oriented read-only identification pass: Device Descriptor
//!    Type 0, `PID_MANUFACTURER_ID`, `PID_HARDWARE_TYPE`, `PID_PROGRAM_VERSION`.
//!
//! Every session is [`ManagementSession::read_only`] with
//! [`AuthorisationPlan::Skip`], so no `A_Authorize_Request` and no write of any
//! kind can reach the wire: the write path does not exist on a read-only
//! session, and `SessionTiming`'s write-side fields are never consulted.
//!
//! **On the broadcast and `1.1.220`.** Design spec §2.1 calls broadcast the
//! sharp edge, because no address filter can keep a broadcast away from the
//! excluded alarm panel. That clause is about `A_IndividualAddress_Write`.
//! This test issues only `A_IndividualAddress_Read`, which mutates nothing and
//! is answered *solely* by devices whose Programming Mode is on — a device with
//! the mode off stays silent by specification, not by our filtering. The panel
//! is therefore not addressed, not written, and cannot be changed by this
//! frame; if it ever *did* answer, that answer is reported like any other and
//! the test asserts the panel is not then used as an identification target.

use std::net::SocketAddrV4;

use knx_core::commissioning::authorisation::AuthorisationPlan;
use knx_core::commissioning::programming_mode::INDIVIDUAL_ADDRESS_READ_TIMEOUT;
use knx_core::commissioning::properties::{
    ObjectIndex, PID_HARDWARE_TYPE, PID_MANUFACTURER_ID, PID_PROGRAM_VERSION,
};
use knx_core::{IndividualAddress, EXCLUDED_INDIVIDUAL_ADDRESSES};
use knx_net::{BusConnection, KnxNetIpClient, ManagementSession, SessionTiming};

// No default, deliberately, matching the other live tests: a hard-coded
// gateway would be somebody's real installation written into a public
// repository.
fn gateway_addr() -> SocketAddrV4 {
    std::env::var("KNX_GATEWAY")
        .expect("set KNX_GATEWAY to your gateway as host:port, e.g. 192.0.2.1:3671")
        .parse()
        .expect("KNX_GATEWAY must be host:port, e.g. 192.0.2.1:3671")
}

/// The broadcast needs a nominal target to build a session at all, and that
/// target is never addressed by `A_IndividualAddress_Read` — the frame goes to
/// the broadcast destination. It still passes the exclusion guard, so this
/// deliberately does not name `1.1.220` even though nothing would be sent to
/// it.
fn nominal_target() -> IndividualAddress {
    "1.1.67".parse().expect("literal address must parse")
}

#[tokio::test]
#[ignore = "needs a real KNXnet/IP gateway; run explicitly with KNX_GATEWAY set"]
async fn finds_the_device_currently_in_programming_mode() {
    let alarm_panel: IndividualAddress = "1.1.220".parse().expect("literal address must parse");
    assert!(
        EXCLUDED_INDIVIDUAL_ADDRESSES.contains(&alarm_panel),
        "the alarm panel must still be in the exclusion set before this test does anything"
    );
    assert_ne!(
        nominal_target(),
        alarm_panel,
        "the nominal broadcast target must never be the alarm panel"
    );

    let client = KnxNetIpClient::new();
    let tunnel = client
        .connect_tunnel(gateway_addr())
        .await
        .expect("connect_tunnel against the real gateway");

    let session = ManagementSession::read_only(
        &tunnel,
        nominal_target(),
        AuthorisationPlan::Skip,
        SessionTiming::default(),
    )
    .expect("read_only session must build");

    println!(
        "broadcasting A_IndividualAddress_Read, waiting out the full {:?} (MP §2.2)",
        INDIVIDUAL_ADDRESS_READ_TIMEOUT
    );
    let responders = session
        .broadcast_individual_address_read(INDIVIDUAL_ADDRESS_READ_TIMEOUT)
        .await
        .expect("the broadcast read must complete");

    // Reported raw, never summarised away: the frame count includes Layer-2
    // repetitions that the device count deliberately ignores (MP §2.2).
    println!(
        "responders: {} device(s) from {} frame(s)",
        responders.device_count(),
        responders.frame_count()
    );
    let found: Vec<IndividualAddress> = responders.devices().collect();
    for device in &found {
        println!("  in programming mode: {device}");
    }

    match responders.single_responder() {
        Ok(witness) => println!("exactly one responder: {}", witness.current_address()),
        Err(err) => println!("no usable single responder: {err}"),
    }

    // Identification is read-only and connection-oriented, so it happens per
    // answering address and only for addresses that actually answered.
    for device in found {
        assert_ne!(
            device, alarm_panel,
            "the alarm panel must never become an identification target"
        );
        println!("== identifying {device} ==");
        let mut probe = match ManagementSession::read_only(
            &tunnel,
            device,
            AuthorisationPlan::Skip,
            SessionTiming::default(),
        ) {
            Ok(probe) => probe,
            Err(err) => {
                println!("  session refused: {err}");
                continue;
            }
        };
        if let Err(err) = probe.connect().await {
            println!("  connect: {err}");
            continue;
        }
        match probe.read_mask_version().await {
            Ok(mask) => println!("  mask version: {mask}"),
            Err(err) => println!("  mask version: {err}"),
        }
        for (name, pid) in [
            ("PID_MANUFACTURER_ID", PID_MANUFACTURER_ID),
            ("PID_HARDWARE_TYPE", PID_HARDWARE_TYPE),
            ("PID_PROGRAM_VERSION", PID_PROGRAM_VERSION),
        ] {
            match probe.read_property(ObjectIndex::DEVICE, pid, 1, 1).await {
                Ok(bytes) => println!("  {name}: {bytes:02x?}"),
                Err(err) => println!("  {name}: {err}"),
            }
        }
        probe.disconnect().await;
    }

    tunnel.disconnect().await.expect("clean tunnel disconnect");
}
