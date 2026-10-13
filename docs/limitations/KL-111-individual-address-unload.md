# Individual-address unload

[Current summary and ledger status](../KNOWN_LIMITATIONS.md#catalogue-kl-111)

This dossier preserves the technical detail and dated history. Earlier
headings, failures and lifting conditions are historical where later
updates supersede them; they are not new work assignments.

<!-- BEGIN retained catalogue record -->

## 111. CP §3.5.4 step 07 (unload the individual address) stays unimplemented

**Limitation.** `[C11]` CP §3.5.4's Individual Address unload procedure has
a step that unloads the individual address itself, which — per its own
mechanism, a broadcast write via the device's `PID_LOAD_STATE_CONTROL` — is
the one write that removes the property a Management Client would need to
address that same device again afterward. This project does not implement
that step: nothing in `crates/knx-core/src/commissioning` or
`crates/knx-net/src/commissioning` issues it, and none of the five variants
this task adds walks it either.

**Cause.** Deliberate refusal, not an oversight. Making a device
unaddressable by the tool that is supposed to be commissioning it is not a
failure mode this application accepts as a side effect of a scripted
procedure — task C6 is what makes the *name* of "unload" honest about the
other four parts it does cover, rather than silently implying a fifth.

**Impact.** None on the parts this project unloads (individual address
excluded). A caller asking for CP §3.5.4's full procedure by that name would
get the four parts other than the individual address; there is no code path
that would remove a device's address without an operator taking a
separate, explicit action outside this procedure.

**Lifted when.** A concrete, reviewed use case needs this project to make a
device unaddressable on purpose (a factory-reset-style workflow, say), at
which point step 07 gets its own guarded implementation, cited against CP
§3.5.4, rather than riding in as one more step of a procedure named for
something else.

**Accepted boundary, 2026-09-28 (goal-commission K8).** User decision
2026-09-28, asked for each K8 entry to fix it or accept it: *"das was am
sinnvollsten ist"* (whatever makes the most sense), i.e. the recommendation.
No use case for making a device unaddressable exists; step 07 stays
unimplemented until one does.
<!-- END retained catalogue record -->
