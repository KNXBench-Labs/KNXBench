# Documented specification contradictions

[Current summary and ledger status](../KNOWN_LIMITATIONS.md#catalogue-kl-095)

This dossier preserves the technical detail and dated history. Earlier
headings, failures and lifting conditions are historical where later
updates supersede them; they are not new work assignments.

<!-- BEGIN retained catalogue record -->

## 95. Six places where the KNX Standard's printed text must not be followed literally

**Limitation.** None — this is a pointer, not a cost. `03_05_03 Configuration
Procedures` ("CP") and `03_05_01 Resources` ("RES") contain six places where
the printed text either contradicts itself or contradicts its own surrounding
rows, and the commissioning code in `crates/knx-core/src/commissioning` and
`crates/knx-net/src/commissioning` deliberately does not follow the literal
text there. A seventh candidate — CP §3.5.2 and §3.5.3 seemingly giving
opposite answers on whether `PID_PROGRAM_VERSION` is written to the three
table objects — turned out on task C18's audit of Volume 6 Profiles to be a
legitimate use of a Property Annex A marks optional there, not errata; it is
recorded as a resolved specification question in
[COMPATIBILITY.md §6](../COMPATIBILITY.md#6-specification-questions-resolved-against-the-standard-not-errata)
instead of here. The full list of the remaining six, each item carrying its
clause and its PDF page — three of the table-variant load procedures reading
Application Program 2's Memory Control Block where every neighbouring row
reads its own; a Group Object Table step filed under the Address Table's
load row (and repeated for AP1 and GOT); a `LoadCompleted` event listed
inside an unload wait, where RES Table 94 marks that transition optional
rather than a step of unloading; a cross-reference to a section that does
not contain what it is sent to find; RES §4.23.2.4.1 naming a Load Control
*value* (`LoadCompleted`, `02h`) as if it were the terminal Load *state*
(`Loaded`, per Table 92); and three mutually incompatible part-unload/load
orderings across CP §3.5.2 and §3.5.4 — lives in
[COMPATIBILITY.md §7](../COMPATIBILITY.md#7-knx-standard-errata--printed-text-this-project-deliberately-does-not-follow),
next to the other compatibility claims and their evidence, rather than here.

**Cause.** A specification this size is not internally consistent by default,
and Configuration Procedures in particular reads like several authors' work
stitched together — the disagreements above are exactly the shape that
produces (misplaced rows, a stale cross-reference, a state/value name
collision), not the shape a single careless reading would produce.

**Impact.** None on a user. Every item above was already the code's actual
behaviour before this entry existed; what changed 2026-09-19 is that the
divergence is now written down next to its clause, so a future reviewer
comparing this code to the printed Standard finds the explanation instead of
mistaking correct behaviour for a bug.

**Lifted when.** It already is; this entry (and its cross-reference) is the
lifting. It would only need revisiting if a later KNX Standard erratum or
edition corrects one of the six clauses, at which point the corresponding
list item names which one no longer applies.
<!-- END retained catalogue record -->
