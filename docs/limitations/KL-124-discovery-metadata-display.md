# Discovery metadata display

[Current summary and ledger status](../KNOWN_LIMITATIONS.md#catalogue-kl-124)

This dossier preserves the technical detail and dated history. Earlier
headings, failures and lifting conditions are historical where later
updates supersede them; they are not new work assignments.

<!-- BEGIN retained catalogue record -->

## 124. The interface search shows four facts about an interface; the protocol carries more

**2026-10-02 UI-owner implementation update.** The narrowing described below
has been removed: `DiscoveredGateway` retains optional decoded Device Info and
the HTTP/UI layers project its medium/status octets, project-installation ID,
serial, routing multicast and MAC without reinterpretation. Missing adapter
metadata is explicitly unavailable, never fabricated zeros. Focused HTTP and
actual unicast-loopback tests, raw-value UI regressions, realistic mutants and
the complete offline gates pass; six additional intercepted-browser cases
verify the disclosure itself. This does not accept native/live Search (§79),
infer device identity/capabilities or grant a write. Publication is pending.
See [UI_ALPHA_READINESS](https://github.com/KNXBench-Labs/KNXBench/blob/bca2d3336b96/docs/archive/alpha-0.1/UI_ALPHA_READINESS.md). The historical title/inventory
identity remains stable; the following cause is historical, not current code.

**Historical description before this follow-up:**

**Limitation.** `POST /api/bus/discover` (`apps/knx-server/src/bus_routes.rs`)
and the panel above it report four things per interface: control endpoint,
individual address, friendly name, and whether tunnelling is among the
service families it advertises. A `SEARCH_RESPONSE`'s Device Info DIB
carries more than that — KNX medium, device status (programming mode), the
project-installation identifier, the KNX serial number, the routing
multicast address, and the MAC address — and `knx-net` decodes every one of
them into `knx_net::core::dib::DeviceInfo`. None of them reach the user.

**Cause.** `KnxNetIpClient::discover()` narrows `DeviceInfo` to the
four-field `DiscoveredGateway` before it returns, so the HTTP route never
sees the rest. This is not a loss the route could avoid by mapping more
carefully: the fields are gone one layer below it. Widening
`DiscoveredGateway` is a change to the protocol crate's public type, which
T25 deliberately did not make — its own boundary was that the search must
not touch `knx-net`'s encoding or decoding.

**Impact.** Small but real, and it grows with the size of the installation.
Two interfaces from the same manufacturer with the same default friendly
name are told apart today only by their addresses; the serial number is the
fact that would distinguish them, and it was decoded and dropped. An
interface sitting in programming mode is likewise invisible here, though
the byte that says so arrived. Nothing is silently wrong — the four
reported fields are accurate — the extra facts are simply not offered.

**Lifted when.** `DiscoveredGateway` carries the `DeviceInfo` it was built
from (or the fields worth keeping), the route maps them, and the panel
shows the ones a user can act on. A day's work in the protocol crate, its
tests, and one DTO — worth doing on the day someone has two identical
interfaces on one network and no way to tell which is which.

**Related.** [§79](../KNOWN_LIMITATIONS.md#79-discovery-needs-ip-multicast-which-dockers-default-bridge-network-does-not-carry)
is the other half of what discovery cannot promise: an empty result is a
result, and on a container without host networking it is the *only*
possible result. The UI says so in its own words rather than presenting an
empty list as a verdict about the installation.
<!-- END retained catalogue record -->
