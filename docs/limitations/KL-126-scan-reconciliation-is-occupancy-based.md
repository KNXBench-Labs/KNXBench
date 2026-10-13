# Scan reconciliation is occupancy-based

[Current summary and ledger status](../KNOWN_LIMITATIONS.md#catalogue-kl-126)

This dossier preserves the technical detail and dated history. Earlier
headings, failures and lifting conditions are historical where later
updates supersede them; they are not new work assignments.

<!-- BEGIN retained catalogue record -->

## 126. line-scan reconciliation acts on occupancy evidence, not device identity

**Boundary.** A completed line scan can establish that an address answered,
did not answer within the selected policy, or was not examined. It does not
identify a manufacturer, product, application program, serial number, or the
reason for silence. T09 therefore creates an explicitly product/program-less
device for a selected unexpected response and never enriches it by inference.
Likewise, a missing project device is deleted only after the user explicitly
selects it; the UI states that silence is not proof of absence.

Excluded project addresses and the scanner's own address are shown as
unexamined and have no action. The
server recomputes the comparison against the current project before applying
anything, requires a completed matching scan session, and refuses a missing
address that does not resolve to exactly one project device. It also refuses
removal while building placement, parameter, or module-instance data still
references the device. Unexpected devices use a matching line across all
installations, falling back to the first installation's unassigned list. One
batch and one undo cover all selected findings; an empty selection changes
neither project nor undo history. Project-tree updates invalidate stale UI
selections. No reconciliation action sends KNX traffic **[V]**.

**Lifted when.** Identity may be attached only when a separately verified
protocol procedure or explicit user selection supplies it. A scan response by
itself never becomes product evidence.

**AR14 (2026-10-04).** The four refusals above that had no test —
foreign session, duplicate selection, an address shared by two project
devices and a selection made stale by later project edits — are pinned by
`reconciliation_refuses_foreign_duplicate_ambiguous_and_stale_selections`;
each refusal leaves project, undo history and bus untouched.
<!-- END retained catalogue record -->
