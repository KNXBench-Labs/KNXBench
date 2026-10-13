# Single-line scan scope

[Current summary and ledger status](../KNOWN_LIMITATIONS.md#catalogue-kl-077)

This dossier preserves the technical detail and dated history. Earlier
headings, failures and lifting conditions are historical where later
updates supersede them; they are not new work assignments.

<!-- BEGIN retained catalogue record -->

## 77. A line scan covers one line at a time; it does not cross couplers

**Limitation.** `bus scan` scans one line, reached through one
tunnelling gateway, per invocation. It does not discover or traverse line
or backbone couplers to scan other lines in the same installation
automatically.

**Cause.** Explicitly out of scope for T17 (see the task brief for this
work): scanning across couplers, or scanning more than one line per
invocation, was never attempted, and no concurrency between probes was
built either — probing stays sequential, one outstanding request per
tunnelling connection, for the same evidence-honesty reasons as
§76.

**Impact.** An installation with more than one line needs one `bus scan`
invocation per line, with the operator supplying each line's own
gateway/area/line addressing by hand; there is no "scan the whole
installation" command.

**Lifted when.** A future task adds coupler-aware, multi-line scanning —
not scheduled as part of T17 or its immediate successors.
<!-- END retained catalogue record -->
