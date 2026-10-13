# Bridge-network tunnel reachability

[Current summary and ledger status](../KNOWN_LIMITATIONS.md#catalogue-kl-155)

This dossier preserves the technical detail and dated history. Earlier
headings, failures and lifting conditions are historical where later
updates supersede them; they are not new work assignments.

<!-- BEGIN retained catalogue record -->

## §155 Tunnelling from a container on Docker's bridge network gets no answer

**Found 2026-10-05 (user report, measured).** The server in a container on
Docker's default bridge network could not open a tunnel to a gateway on the
LAN, although the host firewall was set up and the CLI on the host worked.

**Cause.** `TunnelClient::connect` (`crates/knx-net/src/client.rs`) puts its
own socket address into the `CONNECT_REQUEST`'s control and data endpoint
HPAIs. In a bridge container that is the private bridge address
(`172.17.0.x`). The gateway answers to the address in the HPAI, not to the
packet's source, and that address does not exist on the LAN. Docker's NAT
rewrites only the IP header. `[V]` A read-only `DESCRIPTION_REQUEST` probe
(no tunnel, no bus frame) against one gateway: no answer from a bridge
container with its own address in the HPAI; answers from the host, from a
host-network container, and from a bridge container with the all-zero
"Route Back" HPAI.

**Standard.** `[D]` Core v01.06.02 AS §8.6.2.2 defines the UDP Route Back HPAI
(address and port all zero) for NAT: the server answers to the address and
port of the received IP packet. §8.4.3.4.3: if the control endpoint is Route
Back, the data endpoint must be too.

**Workaround.** Run the container with `--network host` (Linux Docker Engine),
as the [installation guide](../manual/getting-started/04-installation.md) now
says. The published-port bridge form is for project work only.

**Partly lifted 2026-10-05 (AR14B).** `KNX_TUNNEL_ROUTE_BACK=1` makes the
server's tunnels send the Route Back HPAI in the `CONNECT_REQUEST` (control
and data endpoint), every `CONNECTIONSTATE_REQUEST` and the
`DISCONNECT_REQUEST` (`knx_net::TunnelReturnPath`). Offline loopback tests pin
every HPAI in both modes; the default stays the own address, so hosts and
gateways that work today see no change. The CLI has no switch (it runs on the
host).

**Remaining.** Only one gateway has been measured to answer a Route Back
request, and only with `DESCRIPTION_REQUEST`; a gateway that ignores Route
Back still gets no answer through the bridge. Discovery stays host-network
only. No tunnel from a bridge container to real hardware has been run.
<!-- END retained catalogue record -->
