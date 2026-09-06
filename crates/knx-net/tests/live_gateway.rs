//! Integration test against a real KNXnet/IP gateway. Requires actual
//! hardware reachable on the LAN — never runs in CI. Run explicitly:
//! `cargo test -p knx-net -- --ignored`.

use std::net::SocketAddrV4;
use std::time::Duration;

use knx_net::{ApplicationService, BusConnection, Destination, GroupValue};

fn gateway_addr() -> SocketAddrV4 {
    std::env::var("KNX_GATEWAY")
        .unwrap_or_else(|_| "192.0.2.1:3671".to_string())
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
    received
        .expect("at least one telegram within 60s — trigger a switch on the bus if this times out")
        .expect("broadcast channel still open");
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
