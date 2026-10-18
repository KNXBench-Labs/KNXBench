# XML evidence budgets

[Current summary and ledger status](../KNOWN_LIMITATIONS.md#catalogue-kl-152)

This dossier preserves the technical detail and dated history. Earlier
headings, failures and lifting conditions are historical where later
updates supersede them; they are not new work assignments.

## Storage-ceiling budgets (18 October 2026, ADR-0142)

The 1,048,576-item budget refused a real 281.8 MiB application program once the
member bound no longer did. Since
[ADR-0142](../adr/0142-file-size-limits-follow-the-storage-ceiling.md) the item
budget equals the storage ceiling (an element or raw attribute consumes at least
one input byte) and the charged-byte estimate eight times that. That package
installs in 22 s at a peak RSS of 1.16 GB. Depth, namespaces and all-or-nothing
refusal are unchanged; the boundary tests keep the KL-152 numbers through an
injected budget.

<!-- BEGIN retained catalogue record -->

## §152 The XML evidence item limit refuses two real packages

**2026-10-04 candidate update (verified, not delivered):** exact original853 hashes and a
bounded raw-item census select the same two cases from four size-admitted
scheme14 packages/16 XML documents. Actual same-Release scratch pair proves
baseline2 atomic item refusals/observer2 installs, max802433 items and155281510
estimated evidence bytes, peak134552KiB RSS/max2.985s. Independent byte64MiB
would still refuse an item-only raise. A coupled1048576-item/256MiB candidate
retains depth1024, ZIP caps, namespaces and all-or-nothing reporting. Two
registered REDs, six new inclusive-boundary/late-no-partial tests and focused
GREEN and latest7f57abbb public18 are verified:Rust3028/0/176,Web1739,
Chromium82,source712/logs/Release binary exact. Earlier ed03cb85 bindings17/eight
negative controls and private2 retain their original producer identities.
Six owner Rust changes invalidated earlier440-input equivalence; fresh full853
on7f57abbb is independently reconciled:688 unchanged table-count installs,
163 unchanged diagnostic refusals,2 item-budget admissions→690 installs.
Both binaries/source/logs,original853 hashes/retained archives/atomic refusals
and private temp0 independently checked. Rejected rebuild-verifier attempt
stays rejected; corrected actual public18 retry passes. Scoped correction
published as2b2a267f7873137ccbf3d0a5052a541a76573d59 with exact live/fetched
refs and nine blobs; docs-only owner closure preserved/source712 unchanged.
Four own runtime directories removed; final closing metadata/hygiene separate. Original master-language64MiB/262144 limits
and retained-source classification are explicitly preserved. No limit removal,
data summary/truncation or compatibility
claim. Detailed producer identities/rejected verifier attempt/scope are in
PRODUCT_DATABASE_CORPUS.md. This measured two-package limitation is resolved;
larger hostile inputs still receive explicit atomic bounded refusals.

**Observed 2026-10-03.** `MAX_EVIDENCE_ITEMS` (262,144,
`knx-productdb/src/parse/scheme_evidence.rs`) refuses ABB
`PS5604-KNX AC500.knxprod` and Hager `PS_TXA664D_V105_T5` (both scheme 14;
the Hager one measured after the §149 workaround) with `XML evidence exceeds
item limit 262144`. Reproduced in a fresh database. The refusal is atomic
and explicit, so no data is lost silently. **Lifted when** the evidence
budget is sized against measured real maxima, or evidence collection
degrades to a counted summary instead of refusing the package.
<!-- END retained catalogue record -->
