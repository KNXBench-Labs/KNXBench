# PL110 group-responser table

[Current summary and ledger status](../KNOWN_LIMITATIONS.md#catalogue-kl-110)

This dossier preserves the technical detail and dated history. Earlier
headings, failures and lifting conditions are historical where later
updates supersede them; they are not new work assignments.

<!-- BEGIN retained catalogue record -->

## 110. `PID_GROUP_RESPONSER_TABLE` stays unimplemented on every medium

**Limitation.** `[C11]` CP §3.5.3's Application Program 2, Application
Program 1 and Group Object Table variants each carry a step that writes the
Group Address Table segment and, within it, a group responser table via
`PID_GROUP_RESPONSER_TABLE` — footnoted in each case as PL110-only (CP
§3.5.3 footnotes 8, 9 and 10, pp. 47, 50, 52). RES §4.16.8.2.5, p. 239 is
explicit both ways: *"This Property is mandatory for PL110 devices. For all
other media this Property shall not be implemented."* This project targets
TP1, RF and IP; it does not implement `PID_GROUP_RESPONSER_TABLE`, and the
three affected step lists in `crates/knx-core/src/commissioning/
partial_download_variant.rs` say so inline rather than modelling the
property's write as if it applied everywhere.

**Cause.** Deliberate scope decision, not a gap found by accident. RES's own
text makes implementing this property on TP1/RF/IP a Standard violation, not
merely unnecessary, so "not yet done" would be the wrong description.

**Impact.** None on TP1, RF or IP — the medium this project's simulator and
every product fixture seen so far use. A plan that targets a genuine PL110
device must be *declined* outright rather than run with this half of CP
§3.5.3 AP2 Nr. 11 (and its GOT/AP1 siblings) silently missing: an operator
who thinks a PL110 download finished would otherwise be wrong about a
Standard-mandated property no code here ever touched. No such decline
exists in the sequencer yet, because no PL110 support exists yet for it to
guard; this entry is the refusal recorded ahead of the code, per CLAUDE.md's
"never silently discard information."

**Lifted when.** PL110 support is scoped as its own task, at which point
`PID_GROUP_RESPONSER_TABLE` gets a real implementation and this entry
becomes a completed cross-reference instead of a limitation, or a plan
targeting a PL110 device grows an explicit decline in the sequencer and this
entry's "no decline exists yet" clause is struck.
<!-- END retained catalogue record -->
