//! Read-only memory dump of one mask-`0701h` device, named by the operator.
//!
//! ```text
//! KNX_GATEWAY=<host>:3671 KNX_READ_MEMORY_ADDRESS=1.1.67 \
//!     cargo test -p knx-net --test live_memory_readonly -- --ignored --nocapture
//! ```
//!
//! Its purpose is evidence, not programming. It reads back what a download
//! would have to write, so the offline encoders can be checked against a real
//! device before anything is sent to one, and so the device's configuration
//! is on record before it is replaced:
//!
//! - the Group Address Table segment at `4000h`. `AddressTable` of
//!   `A-0027-15-0BAC` is `AS-4000`, offset 0. Octets 1–2 hold the device's
//!   own individual address, whose value the operator knows. That is what
//!   settled the table-entry byte order (RESEARCH §19);
//! - the Group Object Association Table segment at `4201h` (`AS-4201`);
//! - the application/parameter segment at `4400h` (`AS-4400`);
//! - the four load states at `B6EAh`–`B6EDh` (MP §3.31.2).
//!
//! Read-only throughout: `ManagementSession::read_only` with
//! `AuthorisationPlan::Skip`, so neither a write path nor an
//! `A_Authorize_Request` exists. Every read and every error is printed as it
//! arrives. A failed read is shown, never summarised away.

use std::net::SocketAddrV4;

use knx_core::commissioning::authorisation::AuthorisationPlan;
use knx_core::{IndividualAddress, EXCLUDED_INDIVIDUAL_ADDRESSES};
use knx_net::{BusConnection, KnxNetIpClient, ManagementSession, SessionTiming};

fn gateway_addr() -> SocketAddrV4 {
    std::env::var("KNX_GATEWAY")
        .expect("set KNX_GATEWAY to your gateway as host:port, e.g. 192.0.2.1:3671")
        .parse()
        .expect("KNX_GATEWAY must be host:port, e.g. 192.0.2.1:3671")
}

fn address() -> IndividualAddress {
    std::env::var("KNX_READ_MEMORY_ADDRESS")
        .expect("set KNX_READ_MEMORY_ADDRESS to the device to read, e.g. 1.1.67")
        .parse()
        .expect("KNX_READ_MEMORY_ADDRESS must be an individual address, e.g. 1.1.67")
}

/// Octets per `A_Memory_Read`. Small enough for a standard frame on any
/// mask.
const CHUNK: u8 = 8;

/// `(name, start, length)`. The full `AbsoluteSegment`s of `A-0027-15-0BAC`
/// that a download rewrites (`AS-4000`, `AS-4201`, `AS-4400`), so the dump
/// doubles as a backup of the device's configuration before a write.
const REGIONS: [(&str, u32, u32); 4] = [
    ("group address table segment (AS-4000)", 0x4000, 513),
    ("association table segment (AS-4201)", 0x4201, 511),
    ("application/parameter segment (AS-4400)", 0x4400, 394),
    ("load states B6EA..B6ED", 0xB6EA, 4),
];

#[tokio::test]
#[ignore = "needs a real KNXnet/IP gateway; run with KNX_GATEWAY and KNX_READ_MEMORY_ADDRESS"]
async fn dumps_the_tables_and_load_states_read_only() {
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
    for (name, start, length) in REGIONS {
        println!("  {name}:");
        let mut offset = 0;
        while offset < length {
            let address = start + offset;
            let number = u8::try_from((length - offset).min(u32::from(CHUNK)))
                .expect("at most CHUNK octets");
            match session.read_memory(address, number).await {
                Ok(bytes) => println!("    {address:04X}h: {bytes:02x?}"),
                Err(err) => {
                    println!("    {address:04X}h: {err}");
                    break;
                }
            }
            offset += u32::from(number);
        }
    }
    session.disconnect().await;

    tunnel.disconnect().await.expect("clean tunnel disconnect");
}
