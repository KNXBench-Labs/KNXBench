# Three-way comparison

[Current summary and ledger status](../KNOWN_LIMITATIONS.md#catalogue-kl-056)

This dossier preserves the technical detail and dated history. Earlier
headings, failures and lifting conditions are historical where later
updates supersede them; they are not new work assignments.

<!-- BEGIN retained catalogue record -->

## 56. Project diff does not do a three-way comparison

**Limitation.** `diff_projects` takes exactly two projects. There is no
common-ancestor-aware three-way comparison the way a VCS merge does one.

**Cause.** Nothing in this codebase tracks project ancestry or a common
base. Three-way display could be added independently, but any useful merge
action also depends on §55's unresolved conflict/atomic-apply machinery.

**Impact.** Reconciling two independently edited copies of the same
original project has no tool support beyond running the two-way diff
twice, once against each candidate.

**Lifted when.** Open. Requires an explicit base-project contract first;
merge behavior additionally waits for §55. T15 does not pretend that
running two unrelated two-way comparisons creates a three-way result.
<!-- END retained catalogue record -->
