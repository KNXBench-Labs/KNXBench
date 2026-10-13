# Partial-download escalation scope

[Current summary and ledger status](../KNOWN_LIMITATIONS.md#catalogue-kl-113)

This dossier preserves the technical detail and dated history. Earlier
headings, failures and lifting conditions are historical where later
updates supersede them; they are not new work assignments.

<!-- BEGIN retained catalogue record -->

## 113. An escalation only reloads the segments a shortened plan actually carries

**Limitation.** `[C12]` `Downloader::partial_download`
(`crates/knx-net/src/commissioning/download.rs`) now cites the correct CP
§3.5.3 step number for every escalated reload (`PartialDownloadVariant`,
`[C11]`), including one a shortened plan happens not to carry: a Group
Address Table reload in an Application Program 2 escalation is always
numbered Nr. 11, whether or not Application Program 1 or the Group Object
Table are also in this plan. What it does not do is reload a segment the
plan never listed at all. CP §3.5.3 AP2 Nr. 07, p. 46 — *"unload all the
following segments"* — reads as if it always means all four, because the
clause assumes a device with one Interface Object of each kind and a
download that touches all of them; this project's `DownloadPlan` may
legitimately carry fewer (`[C8]`, entry 109), and the escalation only ever
walks `self.plan.parts[position..]` — the segments *this plan* was told
about, in order, not a fixed set of four names.

**Cause.** A partial download's plan is built by the caller for the one
part being replaced (and whatever it chooses to also carry for a possible
escalation); this project has no independent source of "every segment this
device actually has" to reload one the caller never mentioned, and
reloading data the plan does not carry would mean writing something no
part of this call ever validated. Between under-reloading and inventing a
payload to write, this project reports the smaller, honest set.

**Impact.** A partial-download plan that carries only the target part (the
common case in the test suite and, so far, in every fixture built from
product data) never escalates at all in practice beyond the target's own
retry — there is nothing after `position` to reload. A caller that wants
the full CP §3.5.3 escalation behaviour must build the plan with every
segment after the target already in it, in download order; if it leaves
one out, that segment's reload — and its step number in the trace — simply
does not happen, silently, from this module's point of view (the caller
made the choice; this module cannot tell a deliberate omission from an
oversight).

**Lifted when.** A caller-facing planner exists that always fills a
partial-download plan with every segment CP §3.5.3 says an escalation may
need, sourced from the device's actual Interface Object list rather than
left to each call site to remember.

**Accepted boundary, 2026-09-28 (goal-commission K8).** User decision
2026-09-28, asked for each K8 entry to fix it or accept it: *"das was am
sinnvollsten ist"* (whatever makes the most sense), i.e. the recommendation.
Partial downloads are not offered by any surface (the v1 path, K9, is a full
application download), so the shortened-plan case has no caller; it reopens
with the first partial-download feature.
<!-- END retained catalogue record -->
