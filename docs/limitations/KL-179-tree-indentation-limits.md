# Tree indentation limits

[Current summary and ledger status](../KNOWN_LIMITATIONS.md#catalogue-kl-179)

This dossier preserves the technical detail and dated history. Earlier
headings, failures and lifting conditions are historical where later
updates supersede them; they are not new work assignments.

<!-- BEGIN retained catalogue record -->

## 179. Tree indentation is capped at twelve levels; flat views stay flat

**Issue #9 (2026-10-13).** In the group-address tree, address rows are now
indented by their level with the range headers' rule (1.2 rem per level), so
a child starts right of its parent. The indent stops growing at level 12;
deeper rows share that indent, so deep trees stay usable on narrow screens.
The flat presentation keeps one column. In the devices view's *By channel*
grouping, the channel header reads from the left (`▾ name · count`) and the
grouped objects start one step right. Only these two tables were changed and
measured; other grouped tables were not re-measured in this package. Browser
geometry tests in Chromium (EN/DE, 1440/400, light/dark); native WebKitGTK
not exercised. Ledger `KL-179`.
<!-- END retained catalogue record -->
