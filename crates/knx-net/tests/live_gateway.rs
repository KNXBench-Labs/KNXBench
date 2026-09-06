//! Integration test against a real KNXnet/IP gateway. Requires actual
//! hardware reachable on the LAN — never runs in CI. Run explicitly:
//! `cargo test -p knx-net -- --ignored`.

use std::net::SocketAddrV4;
use std::time::Duration;

use knx_net::BusConnection;

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
