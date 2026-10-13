# Repeated download part kinds

[Current summary and ledger status](../KNOWN_LIMITATIONS.md#catalogue-kl-109)

This dossier preserves the technical detail and dated history. Earlier
headings, failures and lifting conditions are historical where later
updates supersede them; they are not new work assignments.

<!-- BEGIN retained catalogue record -->

## 109. Two loadable parts of the same `PartKind` have no defined relative order, so `DownloadPlan::new` refuses them both

**Limitation.** `[C8]` `DownloadPlan::new`
(`crates/knx-net/src/commissioning/download.rs`) now enforces the download
order CP §3.5.2 Nr. 06-10 and CP §3.5.3 AP2 Nr. 08-12 both carry by row
position — Application Program 2, Application Program 1, the Group Object
Table, the Group Address Table, then the Association Table — by requiring
each part's `PartKind` to be strictly greater than the part immediately
before it (`PartKind`'s `Ord`, not `>=`). That means a plan with two parts
of the *same* kind — two Association Tables at two different object
indices, say — is refused with `PlanError::OutOfOrder`, reporting the
second as following an equal, not a lesser, predecessor. Neither CP §3.5.2
nor CP §3.5.3 lists more than one row per kind, so there is no row position
to place a second instance against; a plan like that falls outside what
either table describes at all, not merely outside the order they give.

**Cause.** The two normative tables assume exactly one Interface Object of
each of the five kinds per device, which is the ordinary case RES's Object
Index scheme was built around. Whether the Standard permits a device with,
for instance, two Association Table objects — and if so, in what order a
Management Client would download them — was not found stated anywhere in
either knowledge base consulted for this task (the programming-focused and
the full 179-document corpus). Refusing rather than guessing an order is
this project's own inference, made in the direction CLAUDE.md's priority
order requires (Correctness and Data Integrity ahead of Compatibility): a
wrong guess here would feed `Downloader::partial_download`'s escalation
slice (`parts[position..]`) the same silently-wrong target set this task
exists to prevent.

**Impact.** None observed against the simulator or any product data this
project has imported: every fixture and every ETS project seen so far
carries at most one Interface Object per `PartKind`. A device or a product
database entry that genuinely needs two objects of the same kind in one
download plan cannot be represented today; `DownloadPlan::new` refuses it
outright rather than downloading it in an arbitrary or caller-supplied
order.

**Lifted when.** Spec text (a clause, a table row, or a confirmed erratum)
states a relative order for two same-kind objects, or a real product
database entry is found that requires more than one object of the same
`PartKind` in a single download — at which point the order for that case
can be added deliberately, cited, and tested, rather than inferred here.

**Accepted boundary, 2026-09-28 (goal-commission K8).** User decision
2026-09-28, asked for each K8 entry to fix it or accept it: *"das was am
sinnvollsten ist"* (whatever makes the most sense), i.e. the recommendation.
Refusing both parts stays the behaviour; no order is invented. A cited order
or a real product that needs it reopens this.
<!-- END retained catalogue record -->
