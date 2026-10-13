# CSV read-only and absent columns

[Current summary and ledger status](../KNOWN_LIMITATIONS.md#catalogue-kl-040)

This dossier preserves the technical detail and dated history. Earlier
headings, failures and lifting conditions are historical where later
updates supersede them; they are not new work assignments.

<!-- BEGIN retained catalogue record -->

<a id="40-csv-export-only-columns-are-never-applied-on-import-and-there-are-no-descriptioncomment-columns"></a>
## 40. CSV derived columns are read-only, and there are no `Description`/`Comment` columns

**Limitation.** `DatapointType (read-only)`, `MainGroup (read-only)`, and
`MiddleGroup (read-only)` appear in an exported CSV as derived context, but
are never editable project values. Import validates them against the current
project; a changed value is rejected explicitly and nothing is applied. The
legacy unsuffixed headers remain readable with identical semantics.
Separately, the CSV format has no
`Description` or `Comment` column in either direction, even though the
`.knxproj` schema itself defines `GroupAddress/@Description` and
`@Comment` attributes.

**Cause.** A group address in this domain model (`GroupAddressEntry`,
`crates/knx-core/src/group.rs`) carried no datapoint type at all when this
was written — a DPT belonged to the communication objects linked to the
address, several of which may legitimately disagree, so there was no single
value a CSV row could write back onto the address itself. Since ADR-0078 the
entry keeps a schema-21+ declaration, but the CSV import still never applies
the column; changing that needs its own decision. `MainGroup`/`MiddleGroup` name a
*containing* group range, which is structure, not a field of the address,
so writing one back would mean silently moving the address between ranges
from a rename-focused editor. `Description`/`Comment` are simply not
modelled anywhere in `GroupAddressEntry` yet — the CSV cannot round-trip a
field the domain model does not have.

**Impact.** A user who edits one of these derived cells gets a row-level
read-only error rather than a false success; an unchanged export imports as
a tested no-op. There is no way to bulk-set or bulk-view a description or
comment for a group address via CSV, because there is nowhere in the
project for it to live yet.

**Lifted when.** Making `MainGroup`/`MiddleGroup` editable is tied to the
dedicated range-exchange contract named in §39. `Description`/`Comment`
becoming available is tied to `GroupAddressEntry` gaining those fields in
the domain model — no task currently schedules either.

**Alpha decision (user, 2026-10-04, AR11).** Accepted boundary for the
Alpha: derived columns stay read-only and there are no Description/Comment
columns. No schema change before the release.
<!-- END retained catalogue record -->
