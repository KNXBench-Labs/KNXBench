# KNXnet/IP protocol scope

[Current summary and ledger status](../KNOWN_LIMITATIONS.md#catalogue-kl-026)

This dossier preserves the technical detail and dated history. Earlier
headings, failures and lifting conditions are historical where later
updates supersede them; they are not new work assignments.

<!-- BEGIN retained catalogue record -->

## 26. `BusConnection` does not yet support KNX IP Secure

**Limitation.** `crates/knx-net`'s `BusConnection` trait implements
tunnelling and discovery: `discover` multicasts a `SEARCH_REQUEST`
(original form only, not Core v2's `SEARCH_REQUEST_EXTENDED`) and
collects `SEARCH_RESPONSE`s; `connect_tunnel` opens a tunnel to a
gateway by known IP; `subscribe` receives telegrams; `TunnelClient::send`
writes one (`GroupValueWrite` or any other `ApplicationService`, no DPT
interpretation — raw bytes only, same scope cut as the receive side).
`connect_routing` (Cycle 4) sends/receives unconfirmed `ROUTING_INDICATION`
frames over the standard multicast group — no custom multicast address
override, and `ROUTING_BUSY` is decoded and logged but never used to
throttle sends (see the two new limitation entries below). Secure-protocol
paths remain unimplemented. A reader should not assume the trait is
feature-complete because it compiles.

**Cause.** Session 6 Cycle 1 delivered read-only tunnelling as the
foundation for bus monitoring; Cycle 2 added sending; Cycle 3 added
discovery. Cycle 4 added routing. Secure protocols are out of v1 scope, handled by the isolated `knx-secure` crate.

**Impact.** A real KNX installation's gateways can be found on the LAN
without a known IP once `discover()` sends a valid discovery HPAI (a
final-review fix: the discovery socket must stay unconnected to receive
unicast `SEARCH_RESPONSE`s from any gateway, so a naive `local_addr()`
read off it reported the invalid `0.0.0.0:<port>` — see the fix commit
for the resolved-IP/real-port workaround), then handed by control
endpoint to `connect_tunnel` for monitoring/actuation by group address
over a tunnel. Live-hardware verification of the full discover-then-connect
flow was left for the user to run, same as Cycle 2's `send` — this sandbox has no real KNXnet/IP gateway to discover. KNX IP Secure
remains unreachable regardless; routing (unencrypted multicast) is
reachable as of Cycle 4. Discovery inside the `knx-server` Docker container
needs `--network host`: the server and web UI now call `discover`, so §79
applies to the shipped image.

**Lifted when.** Shelved indefinitely as of 2026-09-06 — no fixed
session or cycle owns it. Plain tunnelling/routing covers the common
case; IP Secure only matters for secure-only gateways or installations
with it explicitly enabled. Revisit on demand (a real gateway needing
it), doing the RESEARCH.md §9 spike first, not speculatively. See
[ROADMAP.md, Session 6](../ROADMAP.md).

**Update, 2026-09-11.** Folded into **T19**'s scope (KNX Secure = Data
Secure + IP Secure + keyring). The user's 2026-09-11 ruling on T19 —
deferred, documented as a limitation, not rejected — applies here too; this
2026-09-06 shelving decision and T19's ruling stand together, not as two
separate calls.
<!-- END retained catalogue record -->

<a id="8-november-2026--optional-plain-tcp-candidate"></a>

## Optional plain TCP (since Alpha 20)

The bounded [TCP contract](../contracts/tcp-tunnelling.md) covers explicit library,
CLI monitor/write and server/Web monitor/group-traffic selection, with UDP still
the default. The bounded slice ships in Alpha 20 and passed independent review and
all selected package stages ([record](../status/2026-11-08-optional-tcp-tunnelling.md)).
Device programming, scans and other
existing device workflows remain UDP-only. Discovery/routing scope and KNX IP
Secure are not widened, and no real-gateway TCP interoperability is inferred.
The [source review](../research/2026-11-08-knxnet-ip-tcp-tunnelling.md) distinguishes
specified timers from local budgets, no-ACK TCP sending from actual device
delivery, and frame draining from loss accounting. [WP-29](../open-work/WP-29-knxnet-ip-tcp-tunnelling.md)
remains open beyond the bounded monitor/group-traffic slice.

<a id="10-november-2026-serverweb-scan-candidate-pending-acceptance"></a>

## 10 November 2026 — server/Web scan successor (main-only acceptance)

A separately bounded source successor adds recorded TCP to server/Web line scans,
strict session/incarnation effects, actual transport and live/pre-disconnect skip
counts. UDP remains the scan-local default; no monitor preference is reused.
Probe errors and disconnect uncertainty are independent. Terminal publication
waits for consuming cleanup; tab navigation does not cancel a server-owned scan.
No skipped body export, fallback/reconnect, programming/download, discovery,
routing, serial, KNX IP Secure or hardware support is added. Independent source
closure and the complete selected integrated gate passed.
[Acceptance and limitations](../status/2026-11-10-server-web-tcp-scans.md).

## 10 November 2026 — CLI scan source successor

The source now permits explicit UDP/TCP on CLI `bus scan` for a single device
or a line/range; UDP stays the default, with no TCP fallback. This supersedes
only the CLI-scan exclusion above. Descriptor replies and positive connect
confirmations retain their distinct meanings; local socket acceptance alone
never proves occupancy. Target/exclusion safety, deadlines, dry-run no-I/O,
project comparison and diagnostic recording remain unchanged. Loopback tests
cover the actual CLI caller, refusal/closure, skipped-frame disclosure and
observed cleanup. Independent review and all selected integrated package stages
passed on `ac9b23814` ([acceptance](../status/2026-11-10-cli-tcp-scans.md));
no release or real-gateway claim. That accepted CLI baseline left Server/Web
scans on UDP; the separately scoped Server/Web successor is now software-accepted
on main. Programming, other device workflows, serial, discovery/routing
and KNX IP Secure scope is unchanged.

## 10 November 2026 — bounded CLI find-serial exception (main-only acceptance)

Both read-only `device find-serial` queries now accept explicit UDP/TCP with UDP
default and no fallback. This supersedes only the find-serial part of the earlier
other-device-workflow exclusions, not address programming or other writes.
Strict pre-contact unicast IPv4/nonzero-port admission, actual transport and
positive pre-disconnect skip reporting preserve existing result/history/bundle
and best-effort cleanup policies. Independent source review and narrow optional
fixture closure and all selected integrated package stages passed; runtime
publication was read back ([scoped record](../status/2026-11-10-cli-tcp-find-serial.md)).
This does not add serial-port transport, discovery/routing, KNX IP Secure or
real-gateway interoperability and does not close WP-29.
