# Applying or merging project diffs

[Current summary and ledger status](../KNOWN_LIMITATIONS.md#catalogue-kl-055)

This dossier preserves the technical detail and dated history. Earlier
headings, failures and lifting conditions are historical where later
updates supersede them; they are not new work assignments.

<!-- BEGIN retained catalogue record -->

## 55. Project diff cannot merge or apply a diff back onto a project

**Limitation.** `diff_projects` computes and shows what changed; it does
not turn a `ProjectDiff` back into a `Command` sequence that could replay
one project's changes onto another.

**Cause.** Applying a two-way observation is a merge engine, not a renderer
extension. It needs conflict semantics, revision checks, inverse commands
for every applicable field, and a policy for additions/removals and
ambiguities. Some fields (including `product_ref`/`program_ref`) have no
command today. A partial implementation could silently corrupt project
data, so T15 deliberately does not build one.

**Impact.** Reviewing a diff and then manually re-applying the same
edits to another project remains a manual, error-prone step; there is no
"apply this change" control anywhere in the diff panel.

**Lifted when.** Open. Requires a separately designed, atomic merge plan
with complete conflict and undo semantics; no task currently supplies one.
<!-- END retained catalogue record -->
