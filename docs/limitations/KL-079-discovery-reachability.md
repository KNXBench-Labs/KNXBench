# Discovery reachability

[Current summary and ledger status](../KNOWN_LIMITATIONS.md#catalogue-kl-079)

This dossier preserves the technical detail and dated history. Earlier
headings, failures and lifting conditions are historical where later
updates supersede them; they are not new work assignments.

<!-- BEGIN retained catalogue record -->

<a id="79-discovery-needs-ip-multicast-which-dockers-default-bridge-network-does-not-carry"></a>
## 79. Discovery needs reachable multicast and permitted unicast replies

`KnxNetIpClient::discover()` sends `SEARCH_REQUEST` to
`224.0.23.12:3671` and awaits a unicast response. Docker's default bridge
does not provide the required multicast path; the documented Linux
deployment uses host networking when discovery is needed. A manually entered
numeric IPv4 endpoint works over unicast from the host or a host-network
container; from a bridge container the tunnel gets no answer either
([§155](../KNOWN_LIMITATIONS.md#155-tunnelling-from-a-container-on-dockers-bridge-network-gets-no-answer),
corrected 2026-10-05). Host routing, firewall and gateway support still
matter with host networking.

**Host cause resolved (2026-09-30, RESEARCH §20.1).** The host `ufw` had
dropped the gateway's unicast UDP reply from *source* port 3671. The user
allowed that source port from the LAN; `knx bus discover` and
`POST /api/bus/discover` then both found the gateway. A rule for the
multicast destination alone was insufficient. No protocol or AppImage
workaround was needed; an unpackaged server and the AppImage had sent the
same request. The CLI and Web empty-result hints still name host firewall
and container networking as plausible causes on other installations.

**Remaining verification boundary.** A native WebKitGTK click on Search
was not observed. The successful CLI/HTTP test on this host is not proof of
all container networks or multi-homed LANs, and a missing gateway remains
a network diagnostic, not proof that the KNX device is absent.

**Offline coverage, 2026-10-01 (U13).** Two non-skipping UDP loopback tests
exercise the same private discovery exchange as production: advertised HPAI
return address, service/body filtering, duplicate endpoints, multiple reply
sources, gateway projection and no-reply deadline. See RESEARCH §20.1 and
`client::tests::discovery_loopback_roundtrip_uses_advertised_hpai_and_filters_datagrams`.
They bind/send only on `127.0.0.1`; they do not verify multicast routing,
host-firewall policy, AppImage/WebKitGTK Search or physical gateway behavior.
ISSUE-12 can close its offline-test acceptance without lifting this network
verification boundary or claiming a new protocol fix.
<!-- END retained catalogue record -->
