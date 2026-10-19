//! Integration test against a real KNXnet/IP gateway. Requires actual
//! hardware reachable on the LAN — never runs in CI. Run explicitly:
//! `cargo test -p knx-net -- --ignored`.

use std::net::SocketAddrV4;
use std::time::Duration;

use knx_net::{ApplicationService, BusConnection, Destination, GroupValue};

// No default gateway address. A hard-coded one would be somebody's real
// installation written into a public repository, and a placeholder one
// would only buy a 60-second timeout before failing anyway.
fn gateway_addr() -> SocketAddrV4 {
    std::env::var("KNX_GATEWAY")
        .expect("set KNX_GATEWAY to your gateway as host:port, e.g. 192.0.2.1:3671")
        .parse()
        .expect("KNX_GATEWAY must be host:port, e.g. 192.0.2.1:3671")
}

#[tokio::test]
#[ignore = "needs a real KNXnet/IP gateway on the LAN"]
async fn connects_and_receives_at_least_one_telegram() {
    let client = knx_net::KnxNetIpClient::new();
    let tunnel = client
        .connect_tunnel(gateway_addr())
        .await
        .expect("connect_tunnel against a real gateway");
    let mut telegrams = tunnel.subscribe();
    let received = tokio::time::timeout(Duration::from_secs(60), telegrams.recv()).await;
    tunnel.disconnect().await.expect("clean disconnect");
    let event = received
        .expect("at least one telegram within 60s — trigger a switch on the bus if this times out")
        .expect("broadcast channel still open");
    assert!(
        matches!(event, knx_net::TunnelEvent::Telegram(_)),
        "expected a telegram, not a Closed event, while the gateway was still connected"
    );
}

/// Writes a `GroupValueWrite` to a real group address and expects the
/// gateway to ack it. This physically actuates whatever the address is
/// linked to — deliberately requires `KNX_TEST_GA` to be set explicitly
/// (three-level `main/middle/sub`, e.g. from an unused/spare group
/// address) rather than defaulting to any address in the live project, so
/// it never runs against a real light/blind/etc. by accident. Not part of
/// `cargo test -- --ignored`'s usual run for that reason; run explicitly
/// with both env vars set.
#[tokio::test]
#[ignore = "needs a real KNXnet/IP gateway and an explicit, human-chosen KNX_TEST_GA"]
async fn sends_a_group_value_write_and_gets_acked() {
    let ga_str = std::env::var("KNX_TEST_GA")
        .expect("set KNX_TEST_GA to a spare group address, e.g. 0/0/1, before running this test");
    let group_address =
        knx_core::GroupAddress::parse(&ga_str, knx_core::GroupAddressStyle::ThreeLevel)
            .expect("KNX_TEST_GA must be main/middle/sub");

    let client = knx_net::KnxNetIpClient::new();
    let tunnel = client
        .connect_tunnel(gateway_addr())
        .await
        .expect("connect_tunnel against a real gateway");
    let result = tunnel
        .send(
            Destination::Group(group_address),
            ApplicationService::GroupValueWrite(GroupValue::Short(1)),
        )
        .await;
    tunnel.disconnect().await.expect("clean disconnect");
    result.expect("gateway should ack the TUNNELLING_REQUEST");
}

/// Multicasts a `SEARCH_REQUEST` and expects the known reference gateway
/// to answer with tunnelling support advertised. Needs LAN access to the
/// discovery multicast group (`224.0.23.12:3671`) — not just a route to
/// one known gateway IP, unlike the other tests in this file.
#[tokio::test]
#[ignore = "needs a real KNXnet/IP gateway reachable via multicast on the LAN"]
async fn discovers_the_known_gateway_with_tunnelling_support() {
    let client = knx_net::KnxNetIpClient::new();
    let gateways = client
        .discover()
        .await
        .expect("discover against a real network");
    let expected = gateway_addr();
    let found = gateways
        .iter()
        .find(|g| g.control_endpoint == expected)
        .unwrap_or_else(|| panic!("expected gateway {expected} among {gateways:?}"));
    assert!(
        found.supports_tunnelling,
        "expected the known gateway to advertise tunnelling support"
    );
}
