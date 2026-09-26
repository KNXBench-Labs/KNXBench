//! Read-only identification of one individual address, named by the operator.
//!
//! The counterpart to `live_programming_mode.rs` for a device that is *not* in
//! Programming Mode: it identifies whoever currently answers at one address,
//! which is how a readdressing operation is confirmed afterwards.
//!
//! ```text
//! KNX_GATEWAY=<host>:3671 KNX_IDENTIFY_ADDRESS=1.1.67 \
//!     cargo test -p knx-net --test live_identify -- --ignored --nocapture
//! ```
//!
//! Read-only throughout: `ManagementSession::read_only` with
//! `AuthorisationPlan::Skip`, so no write path and no `A_Authorize_Request`
//! exists to use. The address comes from the environment, never a literal, and
//! the project-excluded alarm panel is refused before a socket is opened —
//! `ContactableAddress` would refuse it too, but failing early is clearer.

use std::net::SocketAddrV4;

use knx_core::commissioning::authorisation::AuthorisationPlan;
use knx_core::commissioning::properties::{
    ObjectIndex, PID_HARDWARE_TYPE, PID_MANUFACTURER_ID, PID_PROGRAM_VERSION,
};
use knx_core::{IndividualAddress, EXCLUDED_INDIVIDUAL_ADDRESSES};
use knx_net::{BusConnection, KnxNetIpClient, ManagementSession, SessionTiming};

fn gateway_addr() -> SocketAddrV4 {
    std::env::var("KNX_GATEWAY")
        .expect("set KNX_GATEWAY to your gateway as host:port, e.g. 192.0.2.1:3671")
        .parse()
        .expect("KNX_GATEWAY must be host:port, e.g. 192.0.2.1:3671")
}

fn address() -> IndividualAddress {
    std::env::var("KNX_IDENTIFY_ADDRESS")
        .expect("set KNX_IDENTIFY_ADDRESS to the address to identify, e.g. 1.1.67")
        .parse()
        .expect("KNX_IDENTIFY_ADDRESS must be an individual address, e.g. 1.1.67")
}

#[tokio::test]
#[ignore = "needs a real KNXnet/IP gateway; run with KNX_GATEWAY and KNX_IDENTIFY_ADDRESS"]
async fn identifies_the_device_at_the_given_address() {
    let target = address();
    assert!(
        !EXCLUDED_INDIVIDUAL_ADDRESSES.contains(&target),
        "refusing to contact the project-excluded address {target}"
    );

    let client = KnxNetIpClient::new();
    let tunnel = client
        .connect_tunnel(gateway_addr())
        .await
        .expect("connect_tunnel against the real gateway");

    let mut session = ManagementSession::read_only(
        &tunnel,
        target,
        AuthorisationPlan::Skip,
        SessionTiming::default(),
    )
    .expect("a read-only session must build");

    println!("== {target} ==");
    session
        .connect()
        .await
        .expect("the device must accept a connection");
    match session.read_mask_version().await {
        Ok(mask) => println!("  mask version: {mask}"),
        Err(err) => println!("  mask version: {err}"),
    }
    for (name, pid) in [
        ("PID_MANUFACTURER_ID", PID_MANUFACTURER_ID),
        ("PID_HARDWARE_TYPE", PID_HARDWARE_TYPE),
        ("PID_PROGRAM_VERSION", PID_PROGRAM_VERSION),
    ] {
        match session.read_property(ObjectIndex::DEVICE, pid, 1, 1).await {
            Ok(bytes) => println!("  {name}: {bytes:02x?}"),
            Err(err) => println!("  {name}: {err}"),
        }
    }
    session.disconnect().await;

    tunnel.disconnect().await.expect("clean tunnel disconnect");
}
