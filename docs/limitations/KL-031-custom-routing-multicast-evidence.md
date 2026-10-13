# Custom routing multicast evidence

[Current summary and ledger status](../KNOWN_LIMITATIONS.md#catalogue-kl-031)

This dossier preserves the technical detail and dated history. Earlier
headings, failures and lifting conditions are historical where later
updates supersede them; they are not new work assignments.

<!-- BEGIN retained catalogue record -->

<a id="31-knxnetip-routing-has-no-custom-multicast-address-override--resolved-routing-half"></a>
## 31. Routing multicast override exists; discovery and real custom-group traffic unverified

**Resolved (2026-09-13, E6, branch `e6-routing-multicast`).**
`BusConnection` gained `connect_routing_to_group(own_address, group)`
alongside the unchanged `connect_routing(own_address)` — both funnel
through one `RoutingClient::connect_to_group`, so the default and the
override cannot silently drift apart; `connect_routing` is now exactly
`connect_to_group` called with `ROUTING_MULTICAST`'s own address.
`group` is validated as an IPv4 multicast address
(`Ipv4Addr::is_multicast()`, 224.0.0.0/4) before any socket call; a
non-multicast address fails fast with `BusError::NotMulticast`, naming
the rejected address, instead of a bare OS error several calls into
`join_multicast_v4`. `apps/knx-cli`'s `route-monitor` and `route-send`
both gained `--multicast-group <addr>`: an accepted CLI surface takes a
bare IPv4 address, never `address:port`, because the port is not this
override's to choose (see below); omitted, both join the standard group
exactly as before, byte for byte.

**What the Standard permits — R1, checked against the PDF, not just the
Markdown extraction.** Core v01.06.02 AS §8.5.2.2 **[D]** (p. 48): the
Routing Multicast Address "shall be derived from the... System Setup
Multicast Address by adding an offset", default zero; separate
installations sharing an IP network, or exceeding roughly 180 KNX
Subnetworks, "shall use different" Routing Multicast Addresses (Routing
v01.05.02 AS §2.3.2 **[D]**, p. 9). Routing v01.05.02 AS §2.3.1 **[D]**
(p. 9) fixes the *port*, not the address: "every installation shall use
the same IP multicast address and port... port number 3671 is
registered at [IANA] for this purpose" — which is why the override takes
only an address. Neither document states a maximum offset or any range
narrower than "any IPv4 multicast address"; the 180-Subnetwork figure is
guidance for *when* to deviate, not a constraint the code can enforce on
*what value* is chosen. Both citations were re-checked with `pdftotext`
against the original PDF, word for word, precisely because a numeric
claim is the kind the Markdown extraction has mis-rendered before —
this file's own §61/§62 entries note DPT `10.001`'s Day column
truncating at "7 =" in the extraction, which is how an earlier draft
wrongly called a documented range "undocumented". No such truncation,
table, or bit layout is involved here: §8.5.2.2 and §2.3.1/§2.3.2 are
plain prose in both the Markdown and the PDF, word for word. Per the
permissive-reading rule, the implementation validates the full
224.0.0.0/4 range rather than inventing a narrower one **[A]**.

**What remains open.** `DISCOVERY_MULTICAST` is untouched and still
hardcoded to `224.0.23.12:3671` — `discover()` has its own design
question (a different gap row) and E6's brief explicitly scoped this to
routing only. And, stated plainly because compiling is not the same as
working: **this override has never been run against a real installation
using a non-default group** — every test in `client.rs` either runs on
loopback or rejects an address before any socket call; none of it proves
a second KNXnet/IP router on the wire actually receives anything sent to
a custom group.

**Originally.** `RoutingClient::connect_routing` always joined the
standard KNXnet/IP System Setup Multicast Address, `224.0.23.12:3671`
(Routing v01.05.02 AS §2.3.1); no CLI flag or API parameter selected a
different group. Session 6 Cycle 4's design spec deliberately hardcoded
it, the same call as Cycle 3's discovery multicast address — no
environment at the time needed a non-default group.

**Update, 2026-10-04 (AR14) — a host-side isolation defect, fixed.** The
routing socket binds `0.0.0.0:3671`. On Linux such a socket receives every
multicast group joined by *any* socket on the host unless
`IP_MULTICAST_ALL` is switched off (ip(7); default on). A default-group
client therefore heard telegrams sent on a custom group by another client on
the same machine — measured over loopback by
`a_custom_group_telegram_reaches_its_group_and_not_the_default_one`, which
received the leaked frame before the fix. `RoutingClient` now clears
`IP_MULTICAST_ALL` on Linux; the test passes and a reverting mutant fails it.
Other platforms' delivery policy is unverified. This is loopback evidence
for the host's socket delivery only; a real router on a custom group remains
unverified, as stated above.
<!-- END retained catalogue record -->
